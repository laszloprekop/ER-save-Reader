//! `knowledge family-distances` — is the distance BETWEEN flag families constant?
//! `knowledge origin-probe`      — is the remaining drift explained by a record count?
//!
//! Context (docs/BACKLOG.md step 4b). Every family sits at a base that moves
//! between saves, so an application holding a single save cannot position a
//! family the way the pipeline does (isolated-flip analysis needs a before/after
//! PAIR). The 2026-07-19 investigation showed the movement is *quantized*: every
//! observed step, in every family, is a multiple of 4 bytes — the signature of a
//! variable-length u32 record list sitting ahead of the flag data.
//!
//! `family-distances` tested the first consequence of that model and confirmed it:
//! the families are rigidly locked to each other (three distances, zero spread
//! across 37 files, mutually consistent). So locating ONE family locates all of
//! them, and what remains of 4b is pinning a single origin.
//!
//! `origin-probe` attacks that remainder. If the drift really is a growing u32
//! list, some count field in the file must satisfy
//!     family_base - ga_end = FIXED + multiplier * count
//! with ONE fixed value across every file. The probe searches for such a field.
//!
//! METHOD (and its limits). A base is accepted only if it is the UNIQUE position
//! in a bounded window at which every flag whose state is known for that file
//! reads as expected. Expected states come from set-monotonicity: a flag verified
//! to flip in an earlier capture of the same character is SET in every later one.
//! The window is centred on the family's already-measured range — these commands
//! do NOT discover where a family lives (the claims store already did that), they
//! re-measure a known family in files where no flag of that family flipped, so
//! that two families can be observed in ONE file. A window that has to be centred
//! on prior measurements cannot prove a base from nothing, and this code does not
//! claim to. Files with too few constraining flags are reported rather than
//! resolved, because a weakly-constrained window is exactly how the timeline
//! re-annotation produced 32,893 impossible events (docs/BACKLOG.md step 3).
//!
//! Emits knowledge/claims/family-distances.json and origin-probe.json. Like every
//! claims artifact these are generated, never hand-edited (ADR-0004).

use serde_json::{json, Map, Value};
use wasm_event_flags::{FlagState, ResolvedFlags};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::claims::{Claims, Status};
use super::evidence::{slot_slice, Evidence};
use super::pipeline::{
    bit_at, bit_of, family_rel, load_save, SaveFile, INPUT_ALLOCLISTS, INPUT_TRANSITIONS,
};

const CLAIMS: &str = "knowledge/claims/event-flags.json";
const OUT_DISTANCES: &str = "knowledge/claims/family-distances.json";
const OUT_ORIGIN: &str = "knowledge/claims/origin-probe.json";
const OUT_LIST_HUNT: &str = "knowledge/claims/list-hunt.json";
const OUT_VALIDATE: &str = "knowledge/claims/origin-validation.json";
const OUT_CONSTANTS: &str = "knowledge/claims/family-constants.json";

/// Search margin either side of a family's already-measured delta range.
/// Wide enough to cover record-list growth beyond what has been observed,
/// narrow enough that the anchor set stays decisive.
const WINDOW_MARGIN: i64 = 512;

/// Minimum distinct expected-SET flags required before a resolved base is
/// trusted. Below this a window of this size admits coincidental matches.
const MIN_ANCHORS: usize = 3;

/// How far past ga_end the origin probe looks for a record count. Must reach
/// the origin proxy itself (world-state-b sits at ~ga_end+183k), since any
/// variable structure ahead of a family can be what moves it.
const PROBE_SPAN: usize = 190_000;

/// One evidence file after measurement: enough to test structural models
/// without holding the whole 2.6MB slot.
struct Measured {
    rel_path: String,
    corpus: String,
    ga_end: i64,
    grace: i64,
    /// end of the append-only u32 list, offset from ga_end, as resolved by the
    /// reference implementation (ADR-0005) — never a second local copy
    list_end: Option<usize>,
    /// family -> absolute base within the slot
    bases: BTreeMap<String, i64>,
    /// slot bytes from ga_end, PROBE_SPAN long (short if the slot ends first)
    window: Vec<u8>,
    /// slot bytes from the detected grace base, PROBE_SPAN long. The variable
    /// section between ga_end and the EF region (~1.3k of spread) shifts any
    /// count field stored after it, so a count for an in-EF list is only at a
    /// stable offset when measured from inside the EF region.
    grace_window: Vec<u8>,
}

struct FileRef {
    corpus: String,
    save_slot: usize,
    rel_path: String,
}

/// A flag whose state in a given file is known ahead of the search.
struct Expectation {
    flag: u64,
    rel: u64,
    bit: u8,
    expect_set: bool,
}

struct Measurement {
    files: Vec<Measured>,
    notes: Vec<Value>,
    windows: BTreeMap<String, (i64, i64)>,
}

// ---------------------------------------------------------------------------
// Shared measurement
// ---------------------------------------------------------------------------

fn measure_all(repo_root: &Path, keep_window: bool) -> Result<Measurement, String> {
    let input: Value = read_json(repo_root, INPUT_TRANSITIONS)?;
    let evidence = Evidence::open(repo_root)?;
    let claims: Value = read_json(repo_root, CLAIMS)?;
    let alloc_json: Value = read_json(repo_root, INPUT_ALLOCLISTS)?;

    let mut alloc = BTreeMap::new();
    for list in ["legacymap", "legacymap_dlc02"] {
        for e in alloc_json["lists"][list]["entries"]
            .as_array()
            .unwrap_or(&vec![])
        {
            alloc.insert(
                e["map"].as_str().unwrap_or_default().to_string(),
                e["slot"].as_u64().unwrap_or(0),
            );
        }
    }

    let default_corpus = input["corpus"].as_str().ok_or("input missing corpus")?;
    let default_slot = input["save_slot"].as_u64().unwrap_or(0) as usize;

    // ---- family windows, from the claims store's own measurements -----------
    // delta = family_base_abs - ga_end, per file, recovered exactly:
    // family_base_grace_rel was computed against the same grace_base we add back.
    let mut per_file_meas: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    if let Some(pm) = claims["per_file_measurements"].as_object() {
        for (key, v) in pm {
            let name = key.rsplit('#').next().unwrap_or(key).to_string();
            if let (Some(gb), Some(ga)) = (v["grace_base"].as_i64(), v["ga_items_end"].as_i64()) {
                per_file_meas.entry(name).or_insert((gb, ga));
            }
        }
    }
    let mut family_deltas: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    for f in claims["flags"].as_array().unwrap_or(&vec![]) {
        let fam = match f["family"].as_str() {
            Some(s) => s.to_string(),
            None => continue,
        };
        for side in ["before", "after"] {
            let rel = match f["measured"][format!("family_base_grace_rel_{}", side)].as_i64() {
                Some(v) => v,
                None => continue,
            };
            let name = match f["evidence"][side]["file"].as_str() {
                Some(s) => s,
                None => continue,
            };
            if let Some((gb, ga)) = per_file_meas.get(name) {
                family_deltas
                    .entry(fam.clone())
                    .or_default()
                    .push(gb + rel - ga);
            }
        }
    }
    let windows: BTreeMap<String, (i64, i64)> = family_deltas
        .iter()
        .map(|(fam, ds)| {
            let lo = ds.iter().min().copied().unwrap_or(0) - WINDOW_MARGIN;
            let hi = ds.iter().max().copied().unwrap_or(0) + WINDOW_MARGIN;
            (fam.clone(), (lo, hi))
        })
        .collect();

    // ---- expectations: flag -> the order from which it reads SET ------------
    struct FlipRec {
        order: u64,
        family: String,
        flag: u64,
    }
    let mut flips: BTreeMap<(String, usize), Vec<FlipRec>> = BTreeMap::new();
    let mut order_of: BTreeMap<String, u64> = BTreeMap::new();
    let mut files: Vec<FileRef> = Vec::new();
    let mut seen: BTreeSet<(String, usize, String)> = BTreeSet::new();

    let verified: BTreeSet<(String, u64)> = claims["flags"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|f| Some((f["family"].as_str()?.to_string(), f["flag"].as_u64()?)))
        .collect();

    for p in input["pairs"].as_array().ok_or("input missing pairs")? {
        let corpus = p["corpus"].as_str().unwrap_or(default_corpus).to_string();
        let slot = p["save_slot"]
            .as_u64()
            .map(|v| v as usize)
            .unwrap_or(default_slot);
        let order = p["order"].as_u64().ok_or("pair missing order")?;
        let family = p["family"].as_str().unwrap_or_default().to_string();
        let flag = p["flag"].as_u64().ok_or("pair missing flag")?;

        for side in ["before", "after"] {
            if let Some(name) = p[side].as_str() {
                let o = if side == "before" { order - 1 } else { order };
                order_of.entry(name.to_string()).or_insert(o);
                if seen.insert((corpus.clone(), slot, name.to_string())) {
                    files.push(FileRef {
                        corpus: corpus.clone(),
                        save_slot: slot,
                        rel_path: name.to_string(),
                    });
                }
            }
        }
        if verified.contains(&(family.clone(), flag)) {
            flips.entry((corpus, slot)).or_default().push(FlipRec {
                order,
                family,
                flag,
            });
        }
    }

    let mut known_set: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    if let Some(ks) = input["known_set_before_all_pairs"].as_object() {
        for (fam, v) in ks {
            known_set.insert(
                fam.clone(),
                v.as_array()
                    .map(|a| a.iter().filter_map(|x| x.as_u64()).collect())
                    .unwrap_or_default(),
            );
        }
    }

    // ---- measure every family we can, in every file -------------------------
    let mut out_files: Vec<Measured> = Vec::new();
    let mut notes: Vec<Value> = Vec::new();

    for fr in &files {
        let sf: SaveFile = load_save(&evidence, &fr.corpus, &fr.rel_path, fr.save_slot)?;
        let here = *order_of.get(&fr.rel_path).unwrap_or(&0);

        let mut exps: BTreeMap<String, Vec<Expectation>> = BTreeMap::new();
        for (fam, flags) in &known_set {
            for &flag in flags {
                if let Ok(rel) = family_rel(fam, flag, &alloc) {
                    exps.entry(fam.clone()).or_default().push(Expectation {
                        flag,
                        rel,
                        bit: bit_of(flag),
                        expect_set: true,
                    });
                }
            }
        }
        if let Some(recs) = flips.get(&(fr.corpus.clone(), fr.save_slot)) {
            for r in recs {
                // strictly earlier flips are SET here; a flag flipping AT this
                // order is ambiguous between the pair's two sides.
                if r.order >= here {
                    continue;
                }
                if let Ok(rel) = family_rel(&r.family, r.flag, &alloc) {
                    exps.entry(r.family.clone()).or_default().push(Expectation {
                        flag: r.flag,
                        rel,
                        bit: bit_of(r.flag),
                        expect_set: true,
                    });
                }
            }
        }

        let mut bases = BTreeMap::new();
        for (fam, mut es) in exps {
            es.sort_by_key(|e| (e.rel, e.bit));
            es.dedup_by_key(|e| (e.rel, e.bit));
            let window = match windows.get(&fam) {
                Some(w) => *w,
                None => continue,
            };
            if es.len() < MIN_ANCHORS {
                notes.push(json!({
                    "file": fr.rel_path, "family": fam,
                    "skipped": "too few anchors",
                    "anchors": es.len(), "min_required": MIN_ANCHORS
                }));
                continue;
            }
            let hits = search_base(&sf, &es, window);
            match hits.len() {
                1 => {
                    bases.insert(fam.clone(), hits[0]);
                }
                n => notes.push(json!({
                    "file": fr.rel_path, "family": fam,
                    "skipped": if n == 0 { "no candidate" } else { "ambiguous" },
                    "candidates": n, "anchors": es.len(),
                    "flags": es.iter().map(|e| e.flag).collect::<Vec<_>>(),
                })),
            }
        }

        let grab = |from: usize| -> Vec<u8> {
            let e = (from + PROBE_SPAN).min(sf.slot.len());
            if from < e {
                sf.slot[from..e].to_vec()
            } else {
                Vec::new()
            }
        };
        let (window, grace_window) = if keep_window {
            (grab(sf.ga_end.max(0) as usize), grab(sf.grace))
        } else {
            (Vec::new(), Vec::new())
        };

        out_files.push(Measured {
            rel_path: fr.rel_path.clone(),
            corpus: fr.corpus.clone(),
            ga_end: sf.ga_end,
            grace: sf.grace as i64,
            list_end: wasm_event_flags::find_flag_list_end_from(
                &sf.slot,
                sf.ga_end.max(0) as usize,
            ),
            bases,
            window,
            grace_window,
        });
    }

    Ok(Measurement {
        files: out_files,
        notes,
        windows,
    })
}

fn read_json(repo_root: &Path, rel: &str) -> Result<Value, String> {
    serde_json::from_str(
        &fs::read_to_string(repo_root.join(rel)).map_err(|e| format!("{}: {}", rel, e))?,
    )
    .map_err(|e| format!("{}: {}", rel, e))
}

/// Every position in the window at which all expectations hold.
fn search_base(sf: &SaveFile, exps: &[Expectation], window: (i64, i64)) -> Vec<i64> {
    let (lo, hi) = window;
    let mut hits = Vec::new();
    for cand in (sf.ga_end + lo)..=(sf.ga_end + hi) {
        if cand < 0 {
            continue;
        }
        let base_rel = cand - sf.grace as i64;
        let ok = exps.iter().all(|e| {
            match bit_at(sf, (base_rel + e.rel as i64).max(0) as u64, e.bit) {
                Some(v) => v == e.expect_set,
                None => false,
            }
        });
        if ok {
            hits.push(cand);
        }
    }
    hits
}

// ---------------------------------------------------------------------------
// knowledge family-distances
// ---------------------------------------------------------------------------

pub fn cmd_family_distances(_args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    println!("measuring families per file (verify-on-read)…");
    let m = measure_all(&repo_root, false)?;

    println!("\nfamily search windows (delta from ga_end):");
    for (fam, (lo, hi)) in &m.windows {
        println!("  {:24} [{}, {}]", fam, lo, hi);
    }

    let mut pairwise: BTreeMap<(String, String), Vec<(i64, String)>> = BTreeMap::new();
    for f in &m.files {
        let names: Vec<&String> = f.bases.keys().collect();
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                let (a, b) = (names[i].clone(), names[j].clone());
                let d = f.bases[&b] - f.bases[&a];
                pairwise
                    .entry((a, b))
                    .or_default()
                    .push((d, f.rel_path.clone()));
            }
        }
    }

    let measured = m.files.iter().filter(|f| !f.bases.is_empty()).count();
    let multi = m.files.iter().filter(|f| f.bases.len() >= 2).count();
    println!(
        "\nfiles measured: {}   files carrying >=2 family bases: {}",
        measured, multi
    );

    let mut results = Vec::new();
    if pairwise.is_empty() {
        println!("\nNO file yielded two family bases — inter-family distance is untestable.");
    } else {
        println!("\ninter-family distances:");
        for ((a, b), obs) in &pairwise {
            let ds: BTreeSet<i64> = obs.iter().map(|(d, _)| *d).collect();
            let spread = ds.iter().next_back().unwrap() - ds.iter().next().unwrap();
            println!(
                "  {} -> {}   n={}  distinct={:?}  spread={}  [{}]",
                a,
                b,
                obs.len(),
                ds,
                spread,
                if ds.len() == 1 { "CONSTANT" } else { "VARIES" }
            );
            results.push(json!({
                "from": a, "to": b, "n": obs.len(),
                "distances": ds.iter().collect::<Vec<_>>(),
                "spread": spread,
                "constant": ds.len() == 1,
                "observations": obs.iter().map(|(d, f)| json!({"distance": d, "file": f}))
                    .collect::<Vec<_>>(),
            }));
        }
    }

    let mut per_file_out = Map::new();
    for f in &m.files {
        if f.bases.is_empty() {
            continue;
        }
        per_file_out.insert(
            f.rel_path.clone(),
            json!(f
                .bases
                .iter()
                .map(|(k, v)| (k.clone(), json!(v - f.ga_end)))
                .collect::<Map<_, _>>()),
        );
    }

    let out = json!({
        "question": "Is the distance between flag families constant across saves? \
                     If so, locating one family locates all of them (BACKLOG step 4b).",
        "method": method_block(),
        "family_windows": m.windows.iter()
            .map(|(k, (lo, hi))| (k.clone(), json!({"lo": lo, "hi": hi})))
            .collect::<Map<_, _>>(),
        "delta_from_ga_end_per_file": per_file_out,
        "inter_family": results,
        "unresolved": m.notes,
    });
    write_claims(
        &repo_root,
        OUT_DISTANCES,
        "family-distances/1",
        "er-save-reader knowledge family-distances",
        out,
    )
}

// ---------------------------------------------------------------------------
// knowledge origin-probe
// ---------------------------------------------------------------------------

pub fn cmd_origin_probe(_args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    println!("measuring families per file (verify-on-read)…");
    let m = measure_all(&repo_root, true)?;

    // Use whichever family is measured in the most files as the origin proxy;
    // the families are rigidly locked, so any of them tracks the same drift.
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &m.files {
        for fam in f.bases.keys() {
            *counts.entry(fam.as_str()).or_default() += 1;
        }
    }
    let target = counts
        .iter()
        .max_by_key(|(_, n)| **n)
        .map(|(f, _)| f.to_string())
        .ok_or("no family resolved in any file")?;

    let obs: Vec<(&Measured, i64)> = m
        .files
        .iter()
        .filter_map(|f| f.bases.get(&target).map(|b| (f, b - f.ga_end)))
        .filter(|(f, _)| !f.window.is_empty())
        .collect();

    let deltas: BTreeSet<i64> = obs.iter().map(|(_, d)| *d).collect();
    println!(
        "\norigin proxy: {} ({} files)\ndelta from ga_end: {} distinct values, range {}..{}",
        target,
        obs.len(),
        deltas.len(),
        deltas.iter().next().copied().unwrap_or(0),
        deltas.iter().next_back().copied().unwrap_or(0),
    );
    if deltas.len() < 2 {
        println!("delta does not vary across these files — nothing to explain.");
    }

    // Search for a count field: delta - mult*u32(anchor + p) constant over all
    // files. Two anchors, because where the count field sits determines what it
    // is measured from: a count stored before the ga_end->EF variable section is
    // stable from ga_end, one stored after it is only stable from inside EF.
    println!(
        "\nsearching [0,{}) from ga_end AND from grace_base for a count field…",
        PROBE_SPAN
    );
    let mut hits: Vec<Value> = Vec::new();
    for anchor in ["ga_end", "grace_base"] {
        for mult in [4i64, 8, 12, 1, 2] {
            for p in (0..PROBE_SPAN.saturating_sub(4)).step_by(4) {
                let mut fixed: Option<i64> = None;
                let mut ok = true;
                let mut values = Vec::new();
                for (f, delta) in &obs {
                    let w = if anchor == "ga_end" {
                        &f.window
                    } else {
                        &f.grace_window
                    };
                    if p + 4 > w.len() {
                        ok = false;
                        break;
                    }
                    let n = u32::from_le_bytes([w[p], w[p + 1], w[p + 2], w[p + 3]]) as i64;
                    if n > 1_000_000 {
                        ok = false;
                        break;
                    }
                    let candidate = delta - mult * n;
                    match fixed {
                        None => fixed = Some(candidate),
                        Some(v) if v == candidate => {}
                        Some(_) => {
                            ok = false;
                            break;
                        }
                    }
                    values.push(n);
                }
                if !ok {
                    continue;
                }
                // a field that never varies explains nothing
                if values.iter().collect::<BTreeSet<_>>().len() < 2 {
                    continue;
                }
                hits.push(json!({
                    "anchor": anchor,
                    "offset_from_anchor": p,
                    "multiplier": mult,
                    "fixed_amount": fixed,
                    "distinct_counts": values.iter().collect::<BTreeSet<_>>().len(),
                }));
            }
        }
    }

    if hits.is_empty() {
        println!("  NO single count field explains the drift, from either anchor.");
        println!("  The record-list model is not refuted (there may be several lists,");
        println!("  or the growth may not be counted by any u32 in range), but it is");
        println!("  not confirmed in this single-count form.");
    } else {
        println!("  {} candidate field(s):", hits.len());
        for h in hits.iter().take(20) {
            println!(
                "    {}+{:<6} x{:<3} fixed={:<10} distinct_counts={}",
                h["anchor"].as_str().unwrap_or("?"),
                h["offset_from_anchor"],
                h["multiplier"],
                h["fixed_amount"],
                h["distinct_counts"]
            );
        }
    }

    let out = json!({
        "question": "Is the family drift explained by a single u32 record count in the \
                     save, i.e. family_base - ga_end = FIXED + multiplier * count?",
        "method": method_block(),
        "origin_proxy_family": target,
        "files_used": obs.len(),
        "probe_span_from_ga_end": PROBE_SPAN,
        "delta_distinct_values": deltas.len(),
        "candidates": hits,
        "observations": obs.iter().map(|(f, d)| json!({
            "file": f.rel_path, "corpus": f.corpus, "ga_end": f.ga_end, "delta": d,
        })).collect::<Vec<_>>(),
        "unresolved": m.notes,
    });
    write_claims(
        &repo_root,
        OUT_ORIGIN,
        "origin-probe/1",
        "er-save-reader knowledge origin-probe",
        out,
    )
}

fn method_block() -> Value {
    json!({
        "base_acceptance": "unique position in a bounded window where every expected \
                            flag state matches",
        "expectations": "set-monotonicity from pipeline-verified flips, plus \
                         known_set_before_all_pairs anchors",
        "window": "family's measured delta range from ga_end, +/- margin",
        "window_margin": WINDOW_MARGIN,
        "min_anchors": MIN_ANCHORS,
        "limitation": "windows are centred on prior measurements, so this re-measures \
                       known families; it does not locate a family from nothing and \
                       must not be cited as doing so",
    })
}

/// Emit one analysis output through the shared `Claims` emitter — the single
/// owner of the claims format and provenance envelope (ADR-0004). `body` is the
/// command-specific payload; `schema` and `generated_by` are the envelope.
fn write_claims(
    repo_root: &Path,
    rel: &str,
    schema: &str,
    generated_by: &str,
    body: Value,
) -> Result<(), String> {
    Claims::new(schema, generated_by).body(body).write(repo_root, rel)?;
    println!("\nwrote {}", rel);
    Ok(())
}

// ---------------------------------------------------------------------------
// knowledge list-hunt
// ---------------------------------------------------------------------------

/// Bytes that must agree before two files are considered aligned at a shift.
const SYNC_WINDOW: usize = 64;

/// Largest insertion the shift search will follow.
const MAX_SHIFT: usize = 4096;

/// The reference scan, re-anchored: skip leading zeros from `probe`, then take
/// the first ORIGIN_ZERO_RUN zero run. Used only to test whether the EF slice is
/// a usable anchor; the shipped resolver lives in the reference implementation.
fn scan_list_end(win: &[u8], probe: usize) -> Option<usize> {
    let mut i = probe;
    while i < win.len() && win[i] == 0 {
        i += 1;
    }
    if i >= win.len() {
        return None;
    }
    let mut run = 0usize;
    while i < win.len() {
        if win[i] == 0 {
            run += 1;
            if run >= wasm_event_flags::ORIGIN_ZERO_RUN {
                return Some(i + 1 - run);
            }
        } else {
            run = 0;
        }
        i += 1;
    }
    None
}

/// Non-zero bytes a sync window must contain before an alignment counts.
/// The flag region is mostly zeros, and a zero run matches at EVERY shift —
/// without this the search reports shift 0 straight through the bitmap and
/// insertions inside it become invisible.
const MIN_INFORMATIVE: usize = 8;

/// Coarse step for the first pass over the region; transitions are then
/// narrowed to the byte by binary search.
const SCAN_STRIDE: usize = 256;

/// Does `a[i..]` match `b[i+s..]` over SYNC_WINDOW bytes? `s` may be negative:
/// a region can shrink as well as grow between captures.
fn aligned_at(a: &[u8], b: &[u8], i: usize, s: i64) -> bool {
    let j = i as i64 + s;
    if j < 0 || i + SYNC_WINDOW > a.len() || j as usize + SYNC_WINDOW > b.len() {
        return false;
    }
    let j = j as usize;
    let wa = &a[i..i + SYNC_WINDOW];
    if wa.iter().filter(|&&x| x != 0).count() < MIN_INFORMATIVE {
        return false;
    }
    wa == &b[j..j + SYNC_WINDOW]
}

/// Shift of smallest magnitude (multiple of 4) that aligns the two files at `i`.
fn shift_at(a: &[u8], b: &[u8], i: usize) -> Option<i64> {
    let mut cands: Vec<i64> = (0..=MAX_SHIFT as i64)
        .step_by(4)
        .flat_map(|s| if s == 0 { vec![0] } else { vec![s, -s] })
        .collect();
    cands.sort_by_key(|s| (s.abs(), *s));
    cands.into_iter().find(|&s| aligned_at(a, b, i, s))
}

/// Narrow a shift change to the exact byte: the last position holding `lo_shift`.
fn narrow(a: &[u8], b: &[u8], mut lo: usize, mut hi: usize, lo_shift: i64) -> usize {
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if aligned_at(a, b, mid, lo_shift) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

pub fn cmd_list_hunt(_args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    println!("measuring families per file (verify-on-read)…");
    let m = measure_all(&repo_root, true)?;

    let target = "world-state-b";
    let mut obs: Vec<(&Measured, i64)> = m
        .files
        .iter()
        .filter_map(|f| f.bases.get(target).map(|b| (f, b - f.ga_end)))
        .filter(|(f, _)| !f.window.is_empty())
        .collect();
    obs.sort_by_key(|(_, d)| *d);

    // one representative file per distinct delta
    let mut reps: Vec<(&Measured, i64)> = Vec::new();
    for (f, d) in &obs {
        if reps.last().map(|(_, ld)| ld != d).unwrap_or(true) {
            reps.push((f, *d));
        }
    }
    println!(
        "\n{} distinct deltas; diffing {} consecutive representative pairs",
        reps.len(),
        reps.len().saturating_sub(1)
    );

    let mut findings = Vec::new();
    for w in reps.windows(2) {
        let ((fa, da), (fb, db)) = (&w[0], &w[1]);
        let growth = db - da;
        println!(
            "\n=== delta {} -> {}  (+{} bytes)\n    A: {}\n    B: {}",
            da,
            db,
            growth,
            &fa.rel_path[..fa.rel_path.len().min(72)],
            &fb.rel_path[..fb.rel_path.len().min(72)]
        );

        // walk the region from ga_end up to just past the family base.
        // Captures far apart in the timeline differ in content as well as
        // length, so a position that aligns at no shift is skipped rather than
        // treated as fatal — only shift CHANGES are of interest.
        let limit = (*db as usize + 512).min(fa.window.len().min(fb.window.len()));
        let mut transitions: Vec<(usize, i64, i64)> = Vec::new(); // (pos, from, to)
        let mut profile: Vec<(usize, i64)> = Vec::new();
        let mut i = 0usize;
        while i + SCAN_STRIDE < limit {
            if let Some(s) = shift_at(&fa.window, &fb.window, i) {
                profile.push((i, s));
            }
            i += SCAN_STRIDE;
        }
        if profile.is_empty() {
            println!("    files never align in this region — skipped");
            continue;
        }
        let start_shift = profile[0].1;
        let mut cur = start_shift;
        for w2 in profile.windows(2) {
            let ((pi, ps), (ni, ns)) = (w2[0], w2[1]);
            if ns != ps {
                let at = narrow(&fa.window, &fb.window, pi, ni, ps);
                transitions.push((at, ps, ns));
            }
            cur = ns;
        }

        println!(
            "    shift at ga_end: +{}   shift near family base: +{}   transitions: {}",
            start_shift,
            cur,
            transitions.len()
        );
        for (pos, from, to) in &transitions {
            println!(
                "      change at ga_end+{:<7} shift {} -> {}  ({:+} bytes)",
                pos,
                from,
                to,
                to - from
            );
            let ctx = &fa.window[pos.saturating_sub(16)..(*pos + 40).min(fa.window.len())];
            let hex: Vec<String> = ctx.iter().map(|b| format!("{:02x}", b)).collect();
            println!("        A around insertion: {}", hex.join(" "));
            let s = ((*pos as i64 + from - 16).max(0)) as usize;
            let e = ((*pos as i64 + to + 40).max(0) as usize)
                .min(fb.window.len())
                .max(s);
            let ctxb = &fb.window[s..e];
            let hexb: Vec<String> = ctxb.iter().map(|b| format!("{:02x}", b)).collect();
            println!("        B around insertion: {}", hexb.join(" "));
        }

        findings.push(json!({
            "delta_from": da, "delta_to": db, "growth": growth,
            "file_a": fa.rel_path, "file_b": fb.rel_path,
            "shift_at_ga_end": start_shift,
            "shift_at_family_base": cur,
            "insertions": transitions.iter().map(|(p, f, t)| json!({
                "offset_from_ga_end": p, "shift_from": f, "shift_to": t, "bytes": t - f
            })).collect::<Vec<_>>(),
        }));
    }

    // ---- the payoff test -------------------------------------------------
    // If that append-only list is what moves the families, then measuring from
    // its END must remove the drift entirely: family_base - list_end constant.
    println!("\n=== list-end test: is family_base - list_end constant?");
    println!("    (per family AND per corpus — one character proves nothing)");
    let mut ends: Vec<Value> = Vec::new();
    let mut groups: BTreeMap<(String, String), BTreeSet<i64>> = BTreeMap::new();
    let mut by_family: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();
    for f in &m.files {
        let end = match f.list_end {
            Some(e) => e,
            None => continue,
        };
        for (fam, base) in &f.bases {
            let v = base - (f.ga_end + end as i64);
            groups
                .entry((fam.clone(), f.corpus.clone()))
                .or_default()
                .insert(v);
            by_family.entry(fam.clone()).or_default().insert(v);
            ends.push(json!({
                "file": f.rel_path, "corpus": f.corpus, "family": fam,
                "list_end_from_ga_end": end,
                "base_minus_list_end": v,
            }));
        }
    }
    for ((fam, corpus), vals) in &groups {
        let n = ends
            .iter()
            .filter(|e| e["family"] == fam.as_str() && e["corpus"] == corpus.as_str())
            .count();
        let spread = vals.iter().next_back().unwrap() - vals.iter().next().unwrap();
        println!(
            "    {:24} {:20} n={:<3} values={:?} spread={} [{}]",
            fam,
            corpus,
            n,
            vals,
            spread,
            if vals.len() == 1 { "CONSTANT" } else { "VARIES" }
        );
    }
    println!("    --- across all corpora, per family:");
    for (fam, vals) in &by_family {
        println!(
            "    {:24} values={:?} [{}]",
            fam,
            vals,
            if vals.len() == 1 { "CONSTANT" } else { "VARIES" }
        );
    }
    let distinct: BTreeSet<i64> = by_family
        .get(target)
        .cloned()
        .unwrap_or_default();

    // ---- can the list be found from the EF slice alone? --------------------
    // The app holds event_flags.flags (a struct-parsed EF region), not raw slot
    // bytes. If the same scan works anchored on the EF start, the grace cutover
    // needs no replumbing of every call site.
    println!("\n=== EF-slice test: can the list end be found without raw slot bytes?");
    for probe in [8_000usize, 12_000, 16_000, 20_000, 24_000] {
        let (mut agree, mut disagree, mut none) = (0usize, 0usize, 0usize);
        for f in &m.files {
            let (Some(le), false) = (f.list_end, f.grace_window.is_empty()) else {
                continue;
            };
            let want = f.ga_end + le as i64 - f.grace; // list end, EF-relative
            match scan_list_end(&f.grace_window, probe) {
                Some(got) if got as i64 == want => agree += 1,
                Some(_) => disagree += 1,
                None => none += 1,
            }
        }
        println!(
            "    probe EF+{:<6} agree={:<3} disagree={:<3} unresolved={}",
            probe, agree, disagree, none
        );
    }

    let out = json!({
        "question": "WHERE does the save grow between ga_end and the flag families? \
                     Each insertion point is a variable-length structure; the one \
                     inside the EF region is what moves the family bases.",
        "method": {
            "approach": "differential alignment: for two captures whose measured \
                         family delta differs, find every position where the byte \
                         alignment shifts",
            "sync_window": SYNC_WINDOW,
            "max_shift": MAX_SHIFT,
            "scan_stride": SCAN_STRIDE,
            "note": "an insertion point is narrowed to the exact byte by binary \
                     search on the pre-insertion shift",
        },
        "origin_proxy_family": target,
        "pairs": findings,
        "list_end_test": {
            "question": "does measuring from the append-only list's end remove the drift?",
            "probe_start_from_ga_end": wasm_event_flags::ORIGIN_PROBE_START,
            "zero_run_terminator": wasm_event_flags::ORIGIN_ZERO_RUN,
            "distinct_values_origin_proxy": distinct.iter().collect::<Vec<_>>(),
            "constant_origin_proxy": distinct.len() == 1,
            "per_family": by_family.iter()
                .map(|(k, v)| (k.clone(), json!({
                    "values": v.iter().collect::<Vec<_>>(),
                    "constant": v.len() == 1
                })))
                .collect::<Map<_, _>>(),
            "per_family_per_corpus": groups.iter()
                .map(|((f, c), v)| (format!("{} @ {}", f, c), json!({
                    "values": v.iter().collect::<Vec<_>>(),
                    "constant": v.len() == 1
                })))
                .collect::<Map<_, _>>(),
            "observations": ends,
        },
    });
    write_claims(
        &repo_root,
        OUT_LIST_HUNT,
        "list-hunt/1",
        "er-save-reader knowledge list-hunt",
        out,
    )
}

// ---------------------------------------------------------------------------
// knowledge validate-origin
// ---------------------------------------------------------------------------

/// Family base = ga_end + list_end + constant. The constants live in the
/// reference implementation (ADR-0005); this table only names them.
const ORIGIN_CONSTANTS: [(&str, i64); 5] = [
    ("world-state-b", wasm_event_flags::FAMILY_WORLD_STATE_B),
    ("tile-open-world", wasm_event_flags::FAMILY_TILE_OPEN_WORLD),
    ("tile-pickup-row-id", wasm_event_flags::FAMILY_TILE_PICKUP_ROW_ID),
    ("legacy-dungeon", wasm_event_flags::FAMILY_LEGACY_DUNGEON),
    ("legacy-dungeon-pickup", wasm_event_flags::FAMILY_LEGACY_DUNGEON_PICKUP),
];

fn origin_constant(family: &str) -> Option<i64> {
    ORIGIN_CONSTANTS
        .iter()
        .find(|(f, _)| *f == family)
        .map(|(_, c)| *c)
}

/// Predict a family base in a slot with NO history. Delegates to the reference
/// implementation so the pipeline and the app cannot disagree about where a
/// family is — the failure this whole investigation existed to eliminate.
fn predict_base(slot: &[u8], family: &str) -> Option<i64> {
    wasm_event_flags::resolve_family_base(slot, origin_constant(family)?)
}

pub fn cmd_validate_origin(_args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let input: Value = read_json(&repo_root, INPUT_TRANSITIONS)?;
    let evidence = Evidence::open(&repo_root)?;
    let alloc_json: Value = read_json(&repo_root, INPUT_ALLOCLISTS)?;

    let mut alloc = BTreeMap::new();
    for list in ["legacymap", "legacymap_dlc02"] {
        for e in alloc_json["lists"][list]["entries"]
            .as_array()
            .unwrap_or(&vec![])
        {
            alloc.insert(
                e["map"].as_str().unwrap_or_default().to_string(),
                e["slot"].as_u64().unwrap_or(0),
            );
        }
    }

    println!("OUT-OF-SAMPLE ORIGIN VALIDATION");
    println!("The constants were measured almost entirely on the Confessor. Every");
    println!("character below is predicted from its own bytes and checked against");
    println!("states established independently of this model.\n");

    let mut results = Vec::new();
    let (mut pass, mut fail) = (0usize, 0usize);

    // ---- A: multi-slot differentials (V1 / V2 / V3, exact known bit patterns)
    println!("=== A. multi-slot differentials — exact expected bits, foreign characters");
    for msd in input["multi_slot_differentials"]
        .as_array()
        .unwrap_or(&vec![])
    {
        let family = msd["family"].as_str().unwrap_or_default();
        let corpus_id = msd["corpus"].as_str().unwrap_or_default();

        for slot_entry in msd["slots"].as_array().unwrap_or(&vec![]) {
            let save_slot = slot_entry["save_slot"].as_u64().unwrap_or(0) as usize;
            let character = slot_entry["character"].as_str().unwrap_or("?");
            for (fname, expects) in slot_entry["files"].as_object().into_iter().flatten() {
                let sf = match load_save(&evidence, corpus_id, fname, save_slot) {
                    Ok(v) => v,
                    Err(e) => {
                        println!("    {} slot {}: {}", character, save_slot, e);
                        continue;
                    }
                };
                let base = match predict_base(&sf.slot, family) {
                    Some(b) => b,
                    None => {
                        println!("    {} slot {}: prediction failed", character, save_slot);
                        continue;
                    }
                };
                let (mut ok, mut bad) = (0usize, Vec::new());
                for (flag_s, expect) in expects.as_object().into_iter().flatten() {
                    let flag: u64 = match flag_s.parse() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let rel = match family_rel(family, flag, &alloc) {
                        Ok(r) => r,
                        Err(_) => continue,
                    };
                    let want = expect.as_bool().unwrap_or(false);
                    let pos = base + rel as i64;
                    let got = sf
                        .slot
                        .get(pos as usize)
                        .map(|b| (b >> bit_of(flag)) & 1 == 1)
                        .unwrap_or(false);
                    if got == want {
                        ok += 1;
                    } else {
                        bad.push(format!("{} want={} got={}", flag, want, got));
                    }
                }
                let total = ok + bad.len();
                if bad.is_empty() {
                    pass += 1;
                } else {
                    fail += 1;
                }
                println!(
                    "    {:3} slot {}  predicted base {:>9}  {}/{} bits correct  {}",
                    character,
                    save_slot,
                    base,
                    ok,
                    total,
                    if bad.is_empty() {
                        "PASS".to_string()
                    } else {
                        format!("FAIL {:?}", bad)
                    }
                );
                results.push(json!({
                    "test": "multi_slot_differential", "character": character,
                    "save_slot": save_slot, "family": family, "file": fname,
                    "predicted_base": base, "bits_correct": ok, "bits_total": total,
                    "pass": bad.is_empty(), "mismatches": bad,
                }));
            }
        }
    }

    // ---- B: backup save, five different characters, tutorial anchors --------
    println!("\n=== B. backup save slots 0-4 — five characters, tutorial grace anchors");
    let anchors: Vec<u64> = input["known_set_before_all_pairs"]["world-state-b"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();
    for backup_id in ["backup-2026-01-11", "backup-2026-01-01"] {
        // The second backup is optional: skip it if the catalog does not list it.
        // A cataloged backup that is absent or drifted is still a hard error.
        let data = match evidence.bytes(backup_id, "") {
            Ok(d) => d,
            Err(e) if e.contains("not in evidence catalog") => continue,
            Err(e) => return Err(e),
        };
        println!("  {}", backup_id);
        for save_slot in 0..5usize {
            let slot = match slot_slice(&data, save_slot) {
                Some(s) => s,
                None => continue,
            };
            let base = match predict_base(slot, "world-state-b") {
                Some(b) => b,
                None => {
                    println!("    slot {}: prediction failed", save_slot);
                    continue;
                }
            };
            let (mut ok, mut bad) = (0usize, Vec::new());
            for &flag in &anchors {
                let rel = match family_rel("world-state-b", flag, &alloc) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let pos = base + rel as i64;
                let got = slot
                    .get(pos as usize)
                    .map(|b| (b >> bit_of(flag)) & 1 == 1)
                    .unwrap_or(false);
                if got {
                    ok += 1;
                } else {
                    bad.push(flag);
                }
            }
            // A wrong base and a genuinely-unvisited grace look the same from a
            // pass/fail count. Discriminate: is there ANY nearby base at which
            // all anchors read SET? If not, the model is not mislocated — the
            // expectation was, and these anchors are legitimately clear for a
            // character that never touched them.
            let mut rescue: Option<i64> = None;
            if !bad.is_empty() {
                for cand in (base - 4096)..=(base + 4096) {
                    if cand < 0 {
                        continue;
                    }
                    let all = anchors.iter().all(|&flag| {
                        family_rel("world-state-b", flag, &alloc)
                            .ok()
                            .and_then(|rel| slot.get((cand + rel as i64) as usize))
                            .map(|b| (b >> bit_of(flag)) & 1 == 1)
                            .unwrap_or(false)
                    });
                    if all {
                        rescue = Some(cand);
                        break;
                    }
                }
            }
            if bad.is_empty() {
                pass += 1;
            } else {
                fail += 1;
            }
            if let Some(r) = rescue {
                println!(
                    "        NOTE: all anchors DO read SET at {} ({:+} from prediction) \
                     — the model may be mislocated for this slot",
                    r,
                    r - base
                );
            } else if !bad.is_empty() {
                println!(
                    "        NOTE: no base within +/-4096 sets all anchors — consistent \
                     with these graces being genuinely untouched, not a bad base"
                );
            }
            // End-to-end: the SHIPPED read path (EF-relative, what graces_view
            // calls) must agree flag-for-flag with the validated slot-absolute
            // path. Disagreement here means the app and the evidence disagree.
            let det = wasm_event_flags::detect_event_flags_offset_impl(slot);
            let mut shipped = Vec::new();
            let mut shipped_agrees = true;
            if det.offset > 0 {
                let ef = &slot[det.offset..];
                let resolved = ResolvedFlags::from_event_flags(ef);
                for &flag in &anchors {
                    let via_shipped: Option<bool> = resolved.as_ref()
                        .map_or(FlagState::Unknown, |r| r.world_state(flag as u32)).into();
                    let via_direct = family_rel("world-state-b", flag, &alloc)
                        .ok()
                        .and_then(|rel| slot.get((base + rel as i64) as usize))
                        .map(|b| (b >> bit_of(flag)) & 1 == 1);
                    if via_shipped != via_direct {
                        shipped_agrees = false;
                    }
                    shipped.push(json!({
                        "flag": flag, "shipped": via_shipped, "direct": via_direct
                    }));
                }
            } else {
                shipped_agrees = false;
            }
            // Aggregate smoke test: a systematic error can still satisfy a
            // handful of anchors, but it cannot make a whole grace database look
            // plausible. Mid-game characters should show many; minimal ones few.
            if det.offset > 0 {
                let ef = &slot[det.offset..];
                let resolved = ResolvedFlags::from_event_flags(ef);
                let (mut yes, mut no, mut unknown) = (0usize, 0usize, 0usize);
                for (flag, _) in crate::db::graces_data::GRACES_DATA.iter() {
                    match Option::<bool>::from(resolved.as_ref().map_or(FlagState::Unknown, |r| r.world_state(*flag))) {
                        Some(true) => yes += 1,
                        Some(false) => no += 1,
                        None => unknown += 1,
                    }
                }
                println!(
                    "        graces: {} discovered / {} not / {} unknown (of {})",
                    yes,
                    no,
                    unknown,
                    crate::db::graces_data::GRACES_DATA.len()
                );
            }
            println!(
                "        shipped EF-relative read agrees with validated path: {}",
                if shipped_agrees { "YES" } else { "NO" }
            );
            if !shipped_agrees {
                fail += 1;
            }
            println!(
                "    slot {}  predicted base {:>9}  {}/{} tutorial anchors SET  {}",
                save_slot,
                base,
                ok,
                anchors.len(),
                if bad.is_empty() {
                    "PASS".to_string()
                } else {
                    format!("FAIL (clear: {:?})", bad)
                }
            );
            results.push(json!({
                "test": "backup_tutorial_anchors", "corpus": backup_id,
                "save_slot": save_slot, "family": "world-state-b",
                "predicted_base": base, "anchors_set": ok,
                "anchors_total": anchors.len(), "pass": bad.is_empty(),
                "clear_anchors": bad,
                "alternative_base_setting_all_anchors": rescue,
                "shipped_ef_path_agrees": shipped_agrees,
                "shipped_ef_path_reads": shipped,
            }));
        }
    }

    // ---- C: transition test on every verified flag -------------------------
    // The strongest available check on a read function: the pipeline attributed
    // these flips to real actions, so the shipped read must see CLEAR before and
    // SET after. Covers whichever families have a read function; families without
    // one are reported as skipped rather than silently passing.
    println!("\n=== C. attributed transitions — shipped reads must go clear -> set");
    let claims: Value = read_json(&repo_root, CLAIMS)?;
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    let (mut tpass, mut tfail) = (0usize, 0usize);

    let mut hypotheses = 0usize;
    for f in claims["flags"].as_array().unwrap_or(&vec![]) {
        // Respect the status ladder (ADR-0004): applications consume
        // corroborated+verified only. The hypotheses are recorded precisely
        // BECAUSE their labelled flags do not flip where the label suggests —
        // asserting on them would be asserting on a known-open question.
        if !Status::from_json(&f["status"]).is_some_and(Status::is_consumable) {
            hypotheses += 1;
            continue;
        }
        let family = f["family"].as_str().unwrap_or_default().to_string();
        let flag = match f["flag"].as_u64() {
            Some(v) => v,
            None => continue,
        };
        // The claims store addresses the pickup family by row_id; the read
        // function takes the raw getItemFlagId, so convert back.
        let read_id: u32 = match family.as_str() {
            "world-state-b" | "tile-open-world" => flag as u32,
            "legacy-dungeon" | "legacy-dungeon-pickup" => flag as u32,
            "tile-pickup-row-id" => (flag + 7_000) as u32,
            other => {
                *skipped.entry(other.to_string()).or_default() += 1;
                continue;
            }
        };
        let prov = &f["evidence"];
        let corpus_id = prov["corpus"].as_str().unwrap_or_default();
        let save_slot = prov["save_slot"].as_u64().unwrap_or(0) as usize;

        let mut states = Vec::new();
        for side in ["before", "after"] {
            let name = match prov[side]["file"].as_str() { Some(n) => n, None => break };
            // A flag shipped as verified/corroborated must resolve, verify and
            // load: a corpus missing from the catalog is a hard error, not a
            // silently dropped flag (which is how an absent corpus used to read
            // as an empty one — the divergence this seam removes).
            let sf = load_save(&evidence, corpus_id, name, save_slot)?;
            let efs = &sf.slot[sf.grace..];
            // The family is known here, so the read method is chosen explicitly —
            // a bare tile id cannot tell you which family it is.
            let rf = ResolvedFlags::from_event_flags(efs);
            let fs = rf.as_ref().map_or(FlagState::Unknown, |r| match family.as_str() {
                "world-state-b" => r.world_state(read_id),
                "tile-open-world" => r.tile_world(read_id),
                "legacy-dungeon" => r.dungeon(read_id),
                "legacy-dungeon-pickup" => r.dungeon_pickup(read_id),
                _ => r.tile_pickup(read_id),
            });
            states.push(Option::<bool>::from(fs));
        }
        if states.len() != 2 { continue; }
        let ok = states[0] == Some(false) && states[1] == Some(true);
        if ok { tpass += 1; } else {
            tfail += 1;
            println!("    FAIL {:22} flag {:>10}  before={:?} after={:?}",
                     family, flag, states[0], states[1]);
        }
    }
    println!("    {} verified flags read clear->set, {} did not", tpass, tfail);
    println!("    {} hypothesis flag(s) not asserted on (status ladder)", hypotheses);
    for (fam, n) in &skipped {
        println!("    skipped {} flag(s) in {} (no read function yet)", n, fam);
    }
    pass += tpass;
    fail += tfail;

    println!("\n=== RESULT: {} pass, {} fail", pass, fail);
    let out = json!({
        "question": "Do the list-end origin constants hold on characters that were \
                     NOT used to derive them?",
        "model": "family_base = ga_end + find_list_end(slot) + constant(family)",
        "constants": ORIGIN_CONSTANTS.iter()
            .map(|(f, c)| (f.to_string(), json!(c))).collect::<Map<_, _>>(),
        "caveat": "the origin is a bounded structural scan, not a full parse: \
                   the list carries no length prefix. The resolver checks its \
                   assumptions and returns nothing rather than a plausible wrong \
                   answer; constants are measured, not derived.",
        "pass": pass, "fail": fail,
        "results": results,
    });
    write_claims(
        &repo_root,
        OUT_VALIDATE,
        "origin-validation/1",
        "er-save-reader knowledge validate-origin",
        out,
    )
}

// ---------------------------------------------------------------------------
// knowledge family-constants
// ---------------------------------------------------------------------------
//
// `list-hunt` measures a family constant only where `family-distances` could
// re-measure that family's base, which needs MIN_ANCHORS known flag states in a
// file where nothing of that family flipped. Two families never clear that bar:
// `tile-open-world` and `legacy-dungeon` have only two verified flags each, so
// no file ever carries three anchors for them.
//
// The pipeline already knows where those families sit in specific files: an
// isolated-flip analysis pins the base in the very pair that established the
// flag, and it records it (`measured.family_base_grace_rel_after`). That is a
// STRONGER positioning than the windowed re-measurement — it comes from an
// observed transition rather than from a pattern match — it just exists in
// fewer files. This command reads those measurements back and expresses each
// one against the same origin the resolver uses:
//
//     constant = (grace_base + family_base_grace_rel) - (ga_end + list_end)
//
// Verified flags only. A hypothesis-status flag is recorded precisely because
// its labelled flag did NOT flip where the label said, so it carries no base.

/// One family constant measured from one flip.
struct ConstantObs {
    family: String,
    flag: u64,
    file: String,
    side: &'static str,
    constant: i64,
}

pub fn cmd_family_constants(_args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let claims: Value = read_json(&repo_root, CLAIMS)?;
    let evidence = Evidence::open(&repo_root)?;

    println!("FAMILY CONSTANTS FROM ATTRIBUTED FLIPS");
    println!("Each observation is one pipeline-verified flag whose flip pinned its");
    println!("family base in a named file. The base is re-expressed against the");
    println!("origin (ga_end + list end) that the shipped resolver uses.\n");

    // (corpus, slot, file) -> (grace_base, ga_end, list_end); None = unreadable
    type Origin = Option<(i64, i64, usize)>;
    let mut seen: BTreeMap<(String, usize, String), Origin> = BTreeMap::new();
    let mut obs: Vec<ConstantObs> = Vec::new();
    let mut skipped: Vec<Value> = Vec::new();

    for f in claims["flags"].as_array().unwrap_or(&vec![]) {
        if Status::from_json(&f["status"]) != Some(Status::Verified) {
            continue;
        }
        let (Some(family), Some(flag)) = (f["family"].as_str(), f["flag"].as_u64()) else {
            continue;
        };
        let ev = &f["evidence"];
        let corpus_id = ev["corpus"].as_str().unwrap_or_default().to_string();
        let save_slot = ev["save_slot"].as_u64().unwrap_or(0) as usize;

        for side in ["before", "after"] {
            let Some(rel) = f["measured"][format!("family_base_grace_rel_{}", side)].as_i64()
            else {
                continue;
            };
            let Some(name) = ev[side]["file"].as_str() else {
                continue;
            };

            let key = (corpus_id.clone(), save_slot, name.to_string());
            if !seen.contains_key(&key) {
                // verify-on-read; an unreadable or drifted file is reported as
                // None, never guessed — but a corpus missing from the catalog is
                // a malformed claim and stays a hard error.
                let v = match load_save(&evidence, &corpus_id, name, save_slot) {
                    Ok(sf) => wasm_event_flags::find_flag_list_end_from(
                        &sf.slot,
                        sf.ga_end.max(0) as usize,
                    )
                    .map(|end| (sf.grace as i64, sf.ga_end, end)),
                    Err(e) if e.contains("not in evidence catalog") => return Err(e),
                    Err(_) => None,
                };
                seen.insert(key.clone(), v);
            }

            match seen[&key] {
                Some((grace, ga_end, end)) => obs.push(ConstantObs {
                    family: family.to_string(),
                    flag,
                    file: name.to_string(),
                    side,
                    constant: (grace + rel) - (ga_end + end as i64),
                }),
                None => skipped.push(json!({
                    "family": family, "flag": flag, "file": name, "side": side,
                    "reason": "file unreadable or origin unresolved",
                })),
            }
        }
    }

    let mut by_family: BTreeMap<String, Vec<&ConstantObs>> = BTreeMap::new();
    for o in &obs {
        by_family.entry(o.family.clone()).or_default().push(o);
    }

    let shipped = origin_constant;

    let mut families = Map::new();
    let mut agree = 0usize;
    let mut disagree = 0usize;
    for (fam, os) in &by_family {
        let values: BTreeSet<i64> = os.iter().map(|o| o.constant).collect();
        let spread = values.iter().next_back().unwrap() - values.iter().next().unwrap();
        let verdict = match (values.len(), shipped(fam)) {
            (1, Some(c)) if *values.iter().next().unwrap() == c => {
                agree += 1;
                "AGREES WITH SHIPPED".to_string()
            }
            (1, Some(c)) => {
                disagree += 1;
                format!("DISAGREES WITH SHIPPED {}", c)
            }
            (1, None) => "constant; not shipped yet".to_string(),
            _ => {
                disagree += 1;
                "NOT CONSTANT".to_string()
            }
        };
        println!(
            "  {:24} n={:<3} files={:<3} values={:?} spread={}  [{}]",
            fam,
            os.len(),
            os.iter().map(|o| &o.file).collect::<BTreeSet<_>>().len(),
            values,
            spread,
            verdict
        );
        for o in os {
            println!(
                "      {:>10} {:<6} {} = {}",
                o.flag,
                o.side,
                &o.file[..o.file.len().min(64)],
                o.constant
            );
        }
        families.insert(
            fam.clone(),
            json!({
                "observations": os.len(),
                "distinct_files": os.iter().map(|o| &o.file).collect::<BTreeSet<_>>().len(),
                "values": values.iter().collect::<Vec<_>>(),
                "spread": spread,
                "shipped_constant": shipped(fam),
                "verdict": verdict,
                "flags": os.iter().map(|o| json!({
                    "flag": o.flag, "side": o.side, "file": o.file, "constant": o.constant
                })).collect::<Vec<_>>(),
            }),
        );
    }

    println!(
        "\n{} family constant(s) reproduced from flips; {} problem(s); {} observation(s) skipped",
        agree,
        disagree,
        skipped.len()
    );

    let out = json!({
        "question": "What is each family's distance from the origin, measured from \
                     the attributed flips that pinned its base?",
        "method": {
            "formula": "constant = (grace_base + family_base_grace_rel) - (ga_end + list_end)",
            "source": "knowledge/claims/event-flags.json, verified flags only — a \
                       hypothesis-status flag has no measured base by construction",
            "why_not_list_hunt": "list-hunt derives constants from family-distances, \
                                  which re-measures a base by pattern-matching >=3 known \
                                  flag states in a window. tile-open-world and \
                                  legacy-dungeon have only two verified flags each, so no \
                                  file ever carries enough anchors and both are absent \
                                  from its table. The flips themselves pin those bases.",
            "limits": "coverage is per-family as thin as the flip evidence: a family \
                       measured on two files is exactly two independent measurements, \
                       and agreement across them is weaker than the 16-47 file \
                       agreement behind the other constants.",
        },
        "families": families,
        "skipped": skipped,
    });
    write_claims(
        &repo_root,
        OUT_CONSTANTS,
        "family-constants/1",
        "er-save-reader knowledge family-constants",
        out,
    )
}
