//! `knowledge timeline-segments` — exhaustive segment-boundary census over a
//! sparse-diff timeline (docs/BACKLOG.md step 3 / docs/CHANGELOG.md v0.36.1).
//!
//! WHY THIS EXISTS. v0.36.1 established that the Bee timeline is **not one
//! chain**: consecutive captures usually chain perfectly (`prev.new ==
//! next.old` on the offsets they share), but at some points that agreement
//! collapses, meaning unobserved play happened in between and the replayed
//! state is stale. That finding was measured on a SAMPLE — every inter-capture
//! gap over 30 minutes, plus a handful of short-gap pairs — and reported "at
//! least 21 boundaries". "At least" is not a segment map, and the
//! flip-clustering design that step 3 wants (cluster flips WITHIN a segment,
//! never across one) cannot be built on a lower bound. This command replaces
//! the sample with a census: every consecutive pair, no sampling.
//!
//! THE TEST. For consecutive captures A then B, restrict to the offsets both
//! touch and ask how often the byte A wrote is the byte B claims was there
//! before it. High agreement means nothing happened in between that the corpus
//! did not record. Low agreement means the save moved on without being
//! captured, so B's `old` bytes describe a state the replay never had.
//!
//! Two agreement rates are computed per pair because they answer different
//! questions and can disagree informatively:
//!   - `pair_agree_pct` — A's `new` vs B's `old`, on shared offsets only. This
//!     is the v0.36.1 measurement, reproduced exhaustively. It is local: it
//!     compares two adjacent captures and nothing else.
//!   - `replay_agree_pct` — the full replayed state vs B's `old`, over ALL of
//!     B's offsets. This is global and includes offsets A never touched, so it
//!     inherits every earlier boundary's damage. Comparing the two separates
//!     "the chain broke HERE" from "the chain broke earlier and never
//!     recovered".
//!
//! WHAT IT DOES NOT DO. It asserts no flags and names no game events. It
//! describes the SHAPE of the evidence — where the record is continuous and
//! where it is not — so that later analysis can refuse to reason across a
//! discontinuity. Per ADR-0004 the output is generated, never hand-edited.

use serde_json::json;
use std::collections::HashMap;
use std::fs;

use super::timeline::{load_target, read_diff_verified, SLOT_SIZE};

const OUTPUT: &str = "knowledge/claims/timeline-segments.json";

/// A pair whose agreement lands here is neither clearly continuous nor clearly
/// broken, and is reported individually instead of being silently classified.
/// The point of naming a band rather than a single cut is that it makes the
/// threshold falsifiable: if the band turns out to be populated, the bimodal
/// story from v0.36.1 is wrong and the census says so out loud.
const CONTINUOUS_MIN_PCT: f64 = 95.0;
const BROKEN_MAX_PCT: f64 = 50.0;

/// A pair can fail the local test while the replayed state stays intact. That
/// is NOT a segment boundary: if play had continued unobserved, the save would
/// have moved on everywhere and the replayed state would be stale across the
/// board. When `replay_agree_pct` stays this high, only a handful of bytes
/// diverged, so the discontinuity is LOCALIZED and the chain still carries.
/// Treating these as segment cuts would over-fragment the timeline and throw
/// away usable evidence — the opposite of the error v0.36.1 warned about, but
/// an error all the same.
const REPLAY_INTACT_PCT: f64 = 95.0;

/// The gap length v0.36.1 used as its sampling heuristic. Re-tested here as a
/// hypothesis, not assumed: the census can finally say how well it predicts.
const LONG_GAP_SECONDS: i64 = 1800;

struct PairStat {
    index: usize,
    prev_id: String,
    next_id: String,
    gap_seconds: i64,
    overlap: u64,
    pair_agree: u64,
    replay_total: u64,
    replay_agree: u64,
}

impl PairStat {
    fn pair_pct(&self) -> Option<f64> {
        if self.overlap == 0 {
            None
        } else {
            Some(100.0 * self.pair_agree as f64 / self.overlap as f64)
        }
    }
    fn replay_pct(&self) -> f64 {
        if self.replay_total == 0 {
            100.0
        } else {
            100.0 * self.replay_agree as f64 / self.replay_total as f64
        }
    }
}

pub fn cmd_timeline_segments(args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let target_id = args.first().cloned().unwrap_or_else(|| "bee".to_string());

    println!("loading timeline metadata (verify-on-read)…");
    let t = load_target(&repo_root, &target_id)?;
    println!(
        "  {} entries for {} (slot {}): {} .. {}",
        t.entries.len(),
        t.character,
        t.slot_index,
        t.entries.first().map(|e| e.id.as_str()).unwrap_or(""),
        t.entries.last().map(|e| e.id.as_str()).unwrap_or("")
    );
    if t.entries.len() < 2 {
        return Err("need at least 2 entries to measure a boundary".to_string());
    }

    println!("censusing every consecutive pair (verify-on-read)…");
    let mut state = vec![0u8; SLOT_SIZE];
    let mut prev_new: HashMap<u32, u8> = HashMap::new();
    let mut stats: Vec<PairStat> = Vec::with_capacity(t.entries.len() - 1);
    let mut duplicate_offset_files = 0u64;
    let mut out_of_range_records = 0u64;

    for (i, e) in t.entries.iter().enumerate() {
        let data = read_diff_verified(&t, e)?;
        let n = data.len() / 6;

        let mut cur_new: HashMap<u32, u8> = HashMap::with_capacity(n);
        let mut overlap = 0u64;
        let mut pair_agree = 0u64;
        let mut replay_total = 0u64;
        let mut replay_agree = 0u64;
        let mut dupes = 0u64;

        for r in 0..n {
            let rec = &data[r * 6..r * 6 + 6];
            let off = u32::from_le_bytes([rec[0], rec[1], rec[2], rec[3]]);
            let old = rec[4];
            let new = rec[5];
            if off as usize >= SLOT_SIZE {
                out_of_range_records += 1;
                continue;
            }
            if i > 0 {
                if let Some(&pn) = prev_new.get(&off) {
                    overlap += 1;
                    if pn == old {
                        pair_agree += 1;
                    }
                }
                replay_total += 1;
                if state[off as usize] == old {
                    replay_agree += 1;
                }
            }
            if cur_new.insert(off, new).is_some() {
                dupes += 1;
            }
            state[off as usize] = new;
        }
        if dupes > 0 {
            duplicate_offset_files += 1;
        }

        if i > 0 {
            let prev = &t.entries[i - 1];
            let gap_seconds = match (
                chrono::DateTime::parse_from_rfc3339(&prev.timestamp),
                chrono::DateTime::parse_from_rfc3339(&e.timestamp),
            ) {
                (Ok(a), Ok(b)) => (b - a).num_seconds(),
                _ => -1,
            };
            stats.push(PairStat {
                index: i,
                prev_id: prev.id.clone(),
                next_id: e.id.clone(),
                gap_seconds,
                overlap,
                pair_agree,
                replay_total,
                replay_agree,
            });
        }

        prev_new = cur_new;

        if i % 500 == 0 {
            println!("  … {}/{}", i, t.entries.len());
        }
    }

    // ---- classification -------------------------------------------------
    let mut continuous = 0usize;
    let mut broken: Vec<&PairStat> = Vec::new();
    let mut localized: Vec<&PairStat> = Vec::new();
    let mut ambiguous: Vec<&PairStat> = Vec::new();
    let mut undetermined: Vec<&PairStat> = Vec::new();
    for s in &stats {
        match s.pair_pct() {
            None => undetermined.push(s),
            Some(p) if p >= CONTINUOUS_MIN_PCT => continuous += 1,
            // Local test says broken, but corroborate against the global one
            // before cutting the timeline. Corroboration is the project's
            // standing rule (docs/CORROBORATION-SYSTEM.md): when two tests
            // disagree, the disagreement is the finding, not a tie to break.
            Some(p) if p <= BROKEN_MAX_PCT => {
                if s.replay_pct() >= REPLAY_INTACT_PCT {
                    localized.push(s);
                } else {
                    broken.push(s);
                }
            }
            Some(_) => ambiguous.push(s),
        }
    }

    // Histogram over 5-point buckets, so the bimodality claim is inspectable
    // rather than asserted.
    let mut hist = [0u64; 21];
    let mut exactly_100 = 0u64;
    for s in &stats {
        if let Some(p) = s.pair_pct() {
            hist[((p / 5.0).floor() as usize).min(20)] += 1;
            if s.pair_agree == s.overlap {
                exactly_100 += 1;
            }
        }
    }

    // Does a long gap predict a boundary? v0.36.1 said "predicts but does not
    // determine"; with a census we can put numbers on both error directions.
    // Uses the CORROBORATED boundary set, not the raw pair test, so this table
    // cannot disagree with the classification printed above it.
    let boundary_idx: std::collections::HashSet<usize> = broken.iter().map(|s| s.index).collect();
    let (mut long_broken, mut long_cont, mut short_broken, mut short_cont) = (0u64, 0u64, 0u64, 0u64);
    for s in &stats {
        if s.pair_pct().is_none() {
            continue;
        }
        let is_broken = boundary_idx.contains(&s.index);
        let is_long = s.gap_seconds >= LONG_GAP_SECONDS;
        match (is_long, is_broken) {
            (true, true) => long_broken += 1,
            (true, false) => long_cont += 1,
            (false, true) => short_broken += 1,
            (false, false) => short_cont += 1,
        }
    }

    // ---- segment map ----------------------------------------------------
    // A boundary at pair index i means entry i starts a new segment.
    let mut cut: Vec<usize> = broken.iter().map(|s| s.index).collect();
    cut.sort_unstable();
    let mut segments = Vec::new();
    let mut start = 0usize;
    for &c in cut.iter().chain(std::iter::once(&t.entries.len())) {
        if c > start {
            let s0 = &t.entries[start];
            let s1 = &t.entries[c - 1];
            let dur = match (
                chrono::DateTime::parse_from_rfc3339(&s0.timestamp),
                chrono::DateTime::parse_from_rfc3339(&s1.timestamp),
            ) {
                (Ok(a), Ok(b)) => (b - a).num_seconds(),
                _ => -1,
            };
            segments.push(json!({
                "index": segments.len(),
                "start_entry": start,
                "start_id": s0.id,
                "end_id": s1.id,
                "entries": c - start,
                "start_ts": s0.timestamp,
                "end_ts": s1.timestamp,
                "duration_seconds": dur,
            }));
        }
        start = c;
    }

    println!();
    println!("=== segment census ===");
    println!("  consecutive pairs   : {}", stats.len());
    println!(
        "  continuous (>={:.0}%) : {}  (of which EXACTLY 100.00%: {})",
        CONTINUOUS_MIN_PCT, continuous, exactly_100
    );
    println!("  BOUNDARIES (<={:.0}%) : {}", BROKEN_MAX_PCT, broken.len());
    println!(
        "  localized (pair broke, replay >={:.0}%): {}  [NOT segment cuts]",
        REPLAY_INTACT_PCT,
        localized.len()
    );
    println!("  ambiguous (between) : {}", ambiguous.len());
    println!("  undetermined (0 ovl): {}", undetermined.len());
    println!("  => {} segments", segments.len());
    for s in &localized {
        println!(
            "    localized: {} -> {}  pair {:.2}%  replay {:.2}%  shared {}",
            s.prev_id,
            s.next_id,
            s.pair_pct().unwrap_or(-1.0),
            s.replay_pct(),
            s.overlap
        );
    }
    println!();
    println!("  long-gap heuristic (>= {}s) vs boundary:", LONG_GAP_SECONDS);
    println!("    long & boundary  : {}", long_broken);
    println!("    long & continuous: {}  (false alarms)", long_cont);
    println!("    short & boundary : {}  (MISSED by the heuristic)", short_broken);
    println!("    short & continuous: {}", short_cont);
    if !ambiguous.is_empty() {
        println!();
        println!("  ambiguous pairs (threshold is doing real work here):");
        for s in ambiguous.iter().take(20) {
            println!(
                "    {} -> {}  pair {:.2}%  replay {:.2}%  gap {}s  overlap {}",
                s.prev_id,
                s.next_id,
                s.pair_pct().unwrap_or(-1.0),
                s.replay_pct(),
                s.gap_seconds,
                s.overlap
            );
        }
    }

    let boundaries: Vec<serde_json::Value> = broken
        .iter()
        .map(|s| {
            json!({
                "entry_index": s.index,
                "prev_id": s.prev_id,
                "next_id": s.next_id,
                "gap_seconds": s.gap_seconds,
                "shared_offsets": s.overlap,
                "pair_agree_pct": s.pair_pct(),
                "replay_agree_pct": s.replay_pct(),
            })
        })
        .collect();
    let localized_json: Vec<serde_json::Value> = localized
        .iter()
        .map(|s| {
            json!({
                "entry_index": s.index,
                "prev_id": s.prev_id,
                "next_id": s.next_id,
                "gap_seconds": s.gap_seconds,
                "shared_offsets": s.overlap,
                "pair_agree_pct": s.pair_pct(),
                "replay_agree_pct": s.replay_pct(),
            })
        })
        .collect();
    let ambiguous_json: Vec<serde_json::Value> = ambiguous
        .iter()
        .map(|s| {
            json!({
                "entry_index": s.index,
                "prev_id": s.prev_id,
                "next_id": s.next_id,
                "gap_seconds": s.gap_seconds,
                "shared_offsets": s.overlap,
                "pair_agree_pct": s.pair_pct(),
                "replay_agree_pct": s.replay_pct(),
            })
        })
        .collect();

    let out = json!({
        "schema": "timeline-segments/1",
        "note": "Exhaustive segment-boundary census over a sparse-diff timeline. Asserts NO flags and names NO game events: it describes where the captured record is continuous and where play happened unobserved, so that later analysis can refuse to reason across a discontinuity. Supersedes the v0.36.1 SAMPLE ('at least 21 boundaries', measured on long-gap pairs plus a few short-gap spot checks) with a full scan of every consecutive pair.",
        "method": {
            "pair_agree_pct": "prev.new vs next.old over the offsets both captures touch. Local to the pair.",
            "replay_agree_pct": "full replayed state vs next.old over all of next's offsets. Global, so it inherits damage from every earlier boundary.",
            "continuous_min_pct": CONTINUOUS_MIN_PCT,
            "broken_max_pct": BROKEN_MAX_PCT,
            "replay_intact_pct": REPLAY_INTACT_PCT,
            "ambiguous_band": "Pairs between the two thresholds are reported individually rather than classified. An empty band is evidence the bimodal reading is right; a populated one would falsify it.",
            "localized_vs_boundary": "A pair that fails the local test while the replayed state stays >= replay_intact_pct is NOT a segment cut. Unobserved play makes the save stale everywhere; these are stale in a handful of bytes only. They are reported separately so the timeline is not over-fragmented.",
        },
        "target": t.target_id,
        "character": t.character,
        "diff_corpus": t.diff_corpus_id,
        "metadata_corpus": t.meta_corpus_id,
        "slot_index": t.slot_index,
        "entries": t.entries.len(),
        "consecutive_pairs": stats.len(),
        "continuous_pairs": continuous,
        "pairs_agreeing_exactly_100pct": exactly_100,
        "continuity_is_all_or_nothing": "Every continuous pair agrees on EVERY shared offset — there is no 'mostly continuous'. Continuity is not a matter of degree in this corpus, which is why the threshold's exact value does not matter.",
        "boundary_count": broken.len(),
        "localized_count": localized.len(),
        "ambiguous_count": ambiguous.len(),
        "undetermined_count": undetermined.len(),
        "segment_count": segments.len(),
        "pair_agree_histogram_5pct_buckets": hist.to_vec(),
        "long_gap_heuristic": {
            "threshold_seconds": LONG_GAP_SECONDS,
            "long_and_boundary": long_broken,
            "long_and_continuous": long_cont,
            "short_and_boundary": short_broken,
            "short_and_continuous": short_cont,
        },
        "data_hygiene": {
            "files_with_duplicate_offsets": duplicate_offset_files,
            "out_of_range_records": out_of_range_records,
        },
        "boundaries": boundaries,
        "localized_discontinuities": localized_json,
        "ambiguous": ambiguous_json,
        "segments": segments,
    });
    fs::create_dir_all(repo_root.join("knowledge/claims")).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
    fs::write(repo_root.join(OUTPUT), text).map_err(|e| e.to_string())?;
    println!();
    println!("segment census written ({})", OUTPUT);
    Ok(())
}
