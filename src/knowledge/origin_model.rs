//! The origin model: the algorithmic core the `knowledge` origin/family commands
//! share, extracted from `family_distances.rs` so the commands become thin argv
//! adapters over it and the algorithms get a test surface of their own.
//!
//! Three layers live here:
//!   - **Measurement** (`measure_all`, `Measured`, `Measurement`, `search_base`):
//!     locate every family base the claims store can constrain, per evidence file.
//!   - **List alignment** (`scan_list_end`, `aligned_at`, `shift_at`, `narrow`):
//!     the differential byte-alignment used by `list-hunt` to find where the save
//!     grows between ga_end and the flag families.
//!   - **The origin constants** (`ORIGIN_CONSTANTS`, `origin_constant`,
//!     `predict_base`): each family's distance past the list end, and a
//!     history-free base prediction that delegates to the reference resolver
//!     (ADR-0005) so the pipeline and the app cannot disagree about where a
//!     family sits.
//!
//! Nothing here decides its own flag positions — `predict_base` takes the save
//! bytes and asks the resolver; the constants are measured, not derived
//! (ADR-0008). `method_block` and `write_claims` are the shared emission
//! helpers; every command writes through the one `Claims` emitter (ADR-0004).

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::claims::Claims;
use super::evidence::Evidence;
use super::pipeline::{
    bit_at, bit_of, family_rel, load_save, SaveFile, INPUT_ALLOCLISTS, INPUT_TRANSITIONS,
};

pub(super) const CLAIMS: &str = "knowledge/claims/event-flags.json";

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
pub(super) const PROBE_SPAN: usize = 190_000;

/// One evidence file after measurement: enough to test structural models
/// without holding the whole 2.6MB slot.
pub(super) struct Measured {
    pub(super) rel_path: String,
    pub(super) corpus: String,
    pub(super) ga_end: i64,
    pub(super) grace: i64,
    /// end of the append-only u32 list, offset from ga_end, as resolved by the
    /// reference implementation (ADR-0005) — never a second local copy
    pub(super) list_end: Option<usize>,
    /// family -> absolute base within the slot
    pub(super) bases: BTreeMap<String, i64>,
    /// slot bytes from ga_end, PROBE_SPAN long (short if the slot ends first)
    pub(super) window: Vec<u8>,
    /// slot bytes from the detected grace base, PROBE_SPAN long. The variable
    /// section between ga_end and the EF region (~1.3k of spread) shifts any
    /// count field stored after it, so a count for an in-EF list is only at a
    /// stable offset when measured from inside the EF region.
    pub(super) grace_window: Vec<u8>,
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

pub(super) struct Measurement {
    pub(super) files: Vec<Measured>,
    pub(super) notes: Vec<Value>,
    pub(super) windows: BTreeMap<String, (i64, i64)>,
}

// ---------------------------------------------------------------------------
// Shared measurement
// ---------------------------------------------------------------------------

pub(super) fn measure_all(repo_root: &Path, keep_window: bool) -> Result<Measurement, String> {
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

pub(super) fn read_json(repo_root: &Path, rel: &str) -> Result<Value, String> {
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

pub(super) fn method_block() -> Value {
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
pub(super) fn write_claims(
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
// list-hunt alignment
// ---------------------------------------------------------------------------

/// Bytes that must agree before two files are considered aligned at a shift.
pub(super) const SYNC_WINDOW: usize = 64;

/// Largest insertion the shift search will follow.
pub(super) const MAX_SHIFT: usize = 4096;

/// The reference scan, re-anchored: skip leading zeros from `probe`, then take
/// the first ORIGIN_ZERO_RUN zero run. Used only to test whether the EF slice is
/// a usable anchor; the shipped resolver lives in the reference implementation.
pub(super) fn scan_list_end(win: &[u8], probe: usize) -> Option<usize> {
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
pub(super) const SCAN_STRIDE: usize = 256;

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
pub(super) fn shift_at(a: &[u8], b: &[u8], i: usize) -> Option<i64> {
    let mut cands: Vec<i64> = (0..=MAX_SHIFT as i64)
        .step_by(4)
        .flat_map(|s| if s == 0 { vec![0] } else { vec![s, -s] })
        .collect();
    cands.sort_by_key(|s| (s.abs(), *s));
    cands.into_iter().find(|&s| aligned_at(a, b, i, s))
}

/// Narrow a shift change to the exact byte: the last position holding `lo_shift`.
pub(super) fn narrow(a: &[u8], b: &[u8], mut lo: usize, mut hi: usize, lo_shift: i64) -> usize {
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

// ---------------------------------------------------------------------------
// origin constants
// ---------------------------------------------------------------------------

/// Family base = ga_end + list_end + constant. The constants live in the
/// reference implementation (ADR-0005); this table only names them.
pub(super) const ORIGIN_CONSTANTS: [(&str, i64); 5] = [
    ("world-state-b", wasm_event_flags::FAMILY_WORLD_STATE_B),
    ("tile-open-world", wasm_event_flags::FAMILY_TILE_OPEN_WORLD),
    ("tile-pickup-row-id", wasm_event_flags::FAMILY_TILE_PICKUP_ROW_ID),
    ("legacy-dungeon", wasm_event_flags::FAMILY_LEGACY_DUNGEON),
    ("legacy-dungeon-pickup", wasm_event_flags::FAMILY_LEGACY_DUNGEON_PICKUP),
];

pub(super) fn origin_constant(family: &str) -> Option<i64> {
    ORIGIN_CONSTANTS
        .iter()
        .find(|(f, _)| *f == family)
        .map(|(_, c)| *c)
}

/// Predict a family base in a slot with NO history. Delegates to the reference
/// implementation so the pipeline and the app cannot disagree about where a
/// family is — the failure this whole investigation existed to eliminate.
pub(super) fn predict_base(slot: &[u8], family: &str) -> Option<i64> {
    wasm_event_flags::resolve_family_base(slot, origin_constant(family)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_constant_names_the_five_families_and_nothing_else() {
        assert_eq!(origin_constant("world-state-b"), Some(wasm_event_flags::FAMILY_WORLD_STATE_B));
        assert_eq!(
            origin_constant("legacy-dungeon-pickup"),
            Some(wasm_event_flags::FAMILY_LEGACY_DUNGEON_PICKUP)
        );
        assert_eq!(origin_constant("no-such-family"), None);
    }

    #[test]
    fn scan_list_end_finds_the_first_zero_run_after_data() {
        // leading zeros are skipped; then the first ORIGIN_ZERO_RUN-long zero run
        // terminates the list. Build: [nonzero data] then a long zero run.
        let run = wasm_event_flags::ORIGIN_ZERO_RUN;
        let mut win = vec![0u8; 4]; // leading zeros, skipped
        win.extend_from_slice(&[1, 2, 3, 4]); // data at index 4..8
        let data_end = win.len();
        win.extend(std::iter::repeat(0).take(run + 2)); // terminating run
        assert_eq!(scan_list_end(&win, 0), Some(data_end));
        // an all-zero window past the probe has no terminating data -> None
        assert_eq!(scan_list_end(&vec![0u8; 8], 0), None);
    }

    #[test]
    fn aligned_at_requires_informative_bytes_not_just_a_zero_run() {
        // Two identical windows of pure zeros: byte-equal but not informative,
        // so an alignment there is rejected (that is the whole point of the guard).
        let zeros = vec![0u8; SYNC_WINDOW * 2];
        assert!(!aligned_at(&zeros, &zeros, 0, 0));
        // Identical windows carrying >= MIN_INFORMATIVE non-zero bytes DO align.
        let mut a = vec![0u8; SYNC_WINDOW * 2];
        for (k, b) in a.iter_mut().take(MIN_INFORMATIVE).enumerate() {
            *b = (k as u8) + 1;
        }
        let b = a.clone();
        assert!(aligned_at(&a, &b, 0, 0));
    }

    #[test]
    fn shift_at_prefers_the_smallest_magnitude_shift() {
        // b is a copy of a shifted right by 4 bytes: the informative window at
        // a[i..] matches b[i+4..]. shift_at should report +4.
        let mut a = vec![0u8; 256];
        for (k, byte) in a.iter_mut().enumerate().take(64).skip(16) {
            *byte = (k as u8) | 1;
        }
        let mut b = vec![0u8; 256];
        b[4..].copy_from_slice(&a[..252]);
        assert_eq!(shift_at(&a, &b, 16), Some(4));
    }
}
