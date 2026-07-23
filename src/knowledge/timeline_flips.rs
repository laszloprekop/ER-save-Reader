//! `knowledge timeline-flips` — does confining flip extraction to a segment
//! fix the monotonicity violation that killed the 2026-07-06 attempt?
//!
//! THE QUESTION, not the feature. BACKLOG step 3's rejected timeline
//! re-annotation failed on a specific, decisive symptom: it reported thousands
//! of event flags transitioning 0->1 *more than once* — one of them 69 times.
//! An event flag bit is set-monotonic, so a bit that "first becomes set" 69
//! times is proof the analysis was reading a moving target, not that the
//! player did anything 69 times. v0.36.1 then found the likely cause: the
//! timeline is not one chain, so consecutive captures can straddle unobserved
//! play, and comparing across that gap compares two unrelated states.
//!
//! `timeline-segments` turned that into an exact map (27 boundaries, 28
//! segments). This command runs the cheapest experiment that can falsify the
//! proposed fix: extract grace-aligned isolated flips exactly as `pipeline.rs`
//! does, but never across a boundary, and count how often each (position, bit)
//! goes 0->1. It runs the SAME count with boundaries ignored, so the two
//! numbers sit side by side and the improvement (or its absence) is measured
//! rather than assumed.
//!
//! WHAT A PASS AND A FAIL LOOK LIKE. If segment-confinement is the missing
//! constraint, within-segment repeat-transitions should collapse toward zero.
//! If they merely shrink, the boundary crossing was one cause among several
//! and the clustering design still lacks something — which is a finding worth
//! having BEFORE building the clustering, not after.
//!
//! THIS COMMAND ASSERTS NO FLAGS. It resolves no family base and names no
//! game event. It reports an internal-consistency statistic about a method.
//! Per ADR-0004 the output is generated, never hand-edited.

use serde_json::json;
use std::collections::HashMap;
use std::fs;

use super::claims::Claims;
use super::timeline::{load_target, read_diff_verified, SLOT_SIZE};

const OUTPUT: &str = "knowledge/claims/timeline-flip-monotonicity.json";
const SEGMENTS_IN: &str = "knowledge/claims/timeline-segments.json";
/// Same neighborhood half-width as `pipeline.rs`'s proven isolated-flip test.
/// Identical ±16 context on both sides rejects the shift illusions that a
/// growing record list produces.
const ISOLATION_W: usize = 16;

/// Grace-aligned isolated byte flips between two replayed states, aligned at
/// each state's own detected offset. Deliberately a copy of `pipeline.rs`'s
/// rule rather than a new one: the point is to test the SEGMENT constraint, so
/// every other part of the method must stay identical or the comparison is
/// confounded.
fn isolated_flips(
    before: &[u8],
    gb: usize,
    after: &[u8],
    ga: usize,
    ef_size: usize,
    out: &mut Vec<(usize, u8, u8)>,
) {
    out.clear();
    let max_i = ef_size
        .min(SLOT_SIZE.saturating_sub(ga + ISOLATION_W + 1))
        .min(SLOT_SIZE.saturating_sub(gb + ISOLATION_W + 1));
    if gb < ISOLATION_W || ga < ISOLATION_W {
        return;
    }
    for i in 0..max_i {
        let (pb, pa) = (gb + i, ga + i);
        if before[pb] == after[pa] {
            continue;
        }
        if before[pb - ISOLATION_W..pb] == after[pa - ISOLATION_W..pa]
            && before[pb + 1..pb + 1 + ISOLATION_W] == after[pa + 1..pa + 1 + ISOLATION_W]
        {
            out.push((i, before[pb], after[pa]));
        }
    }
}

/// Repeat-transition profile for one accounting of the chain.
struct Profile {
    /// (grace_rel, bit) -> how many times that bit was observed going 0->1.
    counts: HashMap<(usize, u8), u32>,
    pairs_considered: u64,
    total_transitions: u64,
}

impl Profile {
    fn new() -> Self {
        Profile {
            counts: HashMap::new(),
            pairs_considered: 0,
            total_transitions: 0,
        }
    }
    fn record(&mut self, flips: &[(usize, u8, u8)]) {
        for &(rel, b, a) in flips {
            let rose = a & !b; // bits that went 0 -> 1
            for bit in 0..8u8 {
                if rose >> bit & 1 == 1 {
                    *self.counts.entry((rel, bit)).or_insert(0) += 1;
                    self.total_transitions += 1;
                }
            }
        }
    }
    /// The decisive statistic: a set-monotonic bit may go 0->1 at most once.
    fn violations(&self) -> (usize, usize, u32) {
        let distinct = self.counts.len();
        let repeats = self.counts.values().filter(|&&c| c > 1).count();
        let worst = self.counts.values().copied().max().unwrap_or(0);
        (distinct, repeats, worst)
    }
}

pub fn cmd_timeline_flips(args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let target_id = args.first().cloned().unwrap_or_else(|| "bee".to_string());

    // The segment map is an INPUT, produced by `timeline-segments`. Requiring
    // it rather than recomputing keeps one definition of a boundary.
    let seg_text = fs::read_to_string(repo_root.join(SEGMENTS_IN)).map_err(|e| {
        format!(
            "{}: {} — run `knowledge timeline-segments {}` first",
            SEGMENTS_IN, e, target_id
        )
    })?;
    let seg: serde_json::Value = serde_json::from_str(&seg_text).map_err(|e| e.to_string())?;
    if seg["target"] != serde_json::Value::String(target_id.clone()) {
        return Err(format!(
            "{} was generated for target '{}', not '{}'",
            SEGMENTS_IN, seg["target"], target_id
        ));
    }
    let boundary_at: std::collections::HashSet<usize> = seg["boundaries"]
        .as_array()
        .ok_or("segments file has no boundaries array")?
        .iter()
        .filter_map(|b| b["entry_index"].as_u64().map(|v| v as usize))
        .collect();
    println!(
        "segment map: {} boundaries / {} segments (from {})",
        boundary_at.len(),
        seg["segment_count"],
        SEGMENTS_IN
    );

    let t = load_target(&repo_root, &target_id)?;
    println!(
        "  {} entries for {} (slot {})",
        t.entries.len(),
        t.character,
        t.slot_index
    );

    let ef_size = wasm_event_flags::EVENT_FLAGS_SIZE;
    let mut state = vec![0u8; SLOT_SIZE];
    let mut prev_state: Vec<u8> = Vec::new();
    let mut prev_off: Option<usize> = None;

    let mut within = Profile::new(); // boundaries respected
    let mut ignoring = Profile::new(); // boundaries ignored (the old method)
    let mut flips: Vec<(usize, u8, u8)> = Vec::new();
    let mut skipped_unconfident = 0u64;
    let mut skipped_boundary = 0u64;

    println!("replaying + extracting isolated flips (verify-on-read)…");
    for (i, e) in t.entries.iter().enumerate() {
        let data = read_diff_verified(&t, e)?;
        for r in 0..data.len() / 6 {
            let rec = &data[r * 6..r * 6 + 6];
            let off = u32::from_le_bytes([rec[0], rec[1], rec[2], rec[3]]) as usize;
            if off < SLOT_SIZE {
                state[off] = rec[5];
            }
        }

        let det = wasm_event_flags::detect_event_flags_offset_impl(&state);
        let cur_off = det.confident.then_some(det.offset);

        if let (Some(gb), Some(ga), false) = (prev_off, cur_off, prev_state.is_empty()) {
            isolated_flips(&prev_state, gb, &state, ga, ef_size, &mut flips);
            // The old method: every consecutive pair, boundaries invisible.
            ignoring.pairs_considered += 1;
            ignoring.record(&flips);
            // The proposed fix: never compare across a boundary.
            if boundary_at.contains(&i) {
                skipped_boundary += 1;
            } else {
                within.pairs_considered += 1;
                within.record(&flips);
            }
        } else if i > 0 {
            skipped_unconfident += 1;
        }

        prev_state.clear();
        prev_state.extend_from_slice(&state);
        prev_off = cur_off;

        if i % 500 == 0 {
            println!("  … {}/{}", i, t.entries.len());
        }
    }

    let (w_distinct, w_repeats, w_worst) = within.violations();
    let (o_distinct, o_repeats, o_worst) = ignoring.violations();

    println!();
    println!("=== monotonicity check (a set-monotonic bit may go 0->1 at most ONCE) ===");
    println!(
        "  boundaries IGNORED  : {} pairs, {} transitions, {} distinct (rel,bit), {} repeat-violations, worst {}x",
        ignoring.pairs_considered, ignoring.total_transitions, o_distinct, o_repeats, o_worst
    );
    println!(
        "  boundaries RESPECTED: {} pairs, {} transitions, {} distinct (rel,bit), {} repeat-violations, worst {}x",
        within.pairs_considered, within.total_transitions, w_distinct, w_repeats, w_worst
    );
    println!(
        "  pairs skipped: {} at a boundary, {} for unconfident detection",
        skipped_boundary, skipped_unconfident
    );
    // The decisive comparison is not "did violations drop" — dropping SOME
    // pairs must drop some violations. It is whether boundary pairs are
    // ENRICHED for violations. If excluding x% of pairs removes x% of
    // violations, boundary pairs are statistically ordinary and the boundary
    // is not the mechanism.
    let pct_pairs_excluded = if ignoring.pairs_considered > 0 {
        100.0 * skipped_boundary as f64 / ignoring.pairs_considered as f64
    } else {
        0.0
    };
    let pct_violations_removed = if o_repeats > 0 {
        100.0 * (o_repeats - w_repeats) as f64 / o_repeats as f64
    } else {
        0.0
    };
    let enrichment = if pct_pairs_excluded > 0.0 {
        pct_violations_removed / pct_pairs_excluded
    } else {
        f64::NAN
    };
    println!();
    println!("  boundary pairs excluded : {:.3}% of pairs", pct_pairs_excluded);
    println!("  repeat-violations removed: {:.3}%", pct_violations_removed);
    println!(
        "  ENRICHMENT (removed/excluded): {:.2}x  — 1.0 means boundary pairs are statistically ORDINARY",
        enrichment
    );

    let verdict = if w_repeats == 0 {
        "PASS — confining to segments eliminates the violation entirely"
    } else if o_repeats > 0 && (w_repeats as f64) < 0.1 * (o_repeats as f64) {
        "PARTIAL — segment confinement removes most but not all violations; another cause remains"
    } else {
        "FAIL — segment confinement does not explain the violation; the clustering design needs more than a segment map"
    };
    println!();
    println!("  VERDICT: {}", verdict);

    let out = json!({
        "note": "Internal-consistency experiment, NOT a flag re-annotation. It asserts no flags, resolves no family base, and names no game event. It asks one question: does confining grace-aligned isolated-flip extraction to a single timeline segment remove the set-monotonicity violation that caused the 2026-07-06 re-annotation attempt to be rejected? The same extraction is run with boundaries respected and ignored so the difference is measured, not assumed.",
        "target": t.target_id,
        "character": t.character,
        "slot_index": t.slot_index,
        "segment_source": SEGMENTS_IN,
        "boundaries_used": boundary_at.len(),
        "isolation_half_width": ISOLATION_W,
        "method_note": "The isolated-flip rule is copied verbatim from pipeline.rs (identical +/-16 neighborhood, grace-aligned at each state's own detected offset). Only the segment constraint differs between the two arms, so the comparison is not confounded.",
        "boundaries_ignored": {
            "pairs": ignoring.pairs_considered,
            "transitions_0_to_1": ignoring.total_transitions,
            "distinct_rel_bit": o_distinct,
            "repeat_violations": o_repeats,
            "worst_repeat_count": o_worst,
        },
        "boundaries_respected": {
            "pairs": within.pairs_considered,
            "transitions_0_to_1": within.total_transitions,
            "distinct_rel_bit": w_distinct,
            "repeat_violations": w_repeats,
            "worst_repeat_count": w_worst,
        },
        "pairs_skipped_at_boundary": skipped_boundary,
        "pairs_skipped_unconfident_detection": skipped_unconfident,
        "enrichment_test": {
            "what_it_measures": "Excluding any pairs must remove some violations, so a raw drop proves nothing. This asks whether boundary pairs are ENRICHED for violations: the ratio of (% violations removed) to (% pairs excluded). ~1.0 means boundary pairs violate at exactly the rate of ordinary pairs, i.e. the boundary is not the mechanism.",
            "pct_pairs_excluded": pct_pairs_excluded,
            "pct_violations_removed": pct_violations_removed,
            "enrichment": enrichment,
        },
        "verdict": verdict,
    });
    Claims::new(
        "timeline-flip-monotonicity/1",
        "er-save-reader knowledge timeline-flips",
    )
    .input("segment_census", &repo_root, SEGMENTS_IN)?
    .body(out)
    .write(&repo_root, OUTPUT)?;
    println!("monotonicity experiment written ({})", OUTPUT);
    Ok(())
}
