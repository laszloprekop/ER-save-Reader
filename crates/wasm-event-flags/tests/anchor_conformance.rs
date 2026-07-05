//! Anchor conformance fixtures (ADR-0003).
//!
//! These fixtures DEFINE the grace-family anchor convention: real slot-data
//! prefixes (first 128 KiB) carved from known saves. If detection code and
//! these assertions disagree, the code is wrong.
//!
//! Golden values recorded 2026-07-05 with the gaEnd-windowed detection that
//! replaced the disproven structural walk (~146k overshoot; see BACKLOG
//! Priority 0b). `confident=false` cases are deliberate: they document honest
//! uncertainty (negative-validation violations), not failures.
//!
//! CAVEAT: the detected offset is the GRACE-FAMILY base. Flag families float
//! independently per save (grace vs catacombs family differ by 0..~500 bytes
//! across the fixture saves), and even within one save-pair regions shift by
//! different amounts (b24->b25: GaItems +16, flag region +4). Byte-exact
//! per-family bases are the re-verification pipeline's job, not detection's.

use wasm_event_flags::{detect_event_flags_offset_impl, parse_ga_items_end};

struct Fixture {
    file: &'static str,
    ga_end: i64,
    offset: usize,
    confident: bool,
}

const FIXTURES: &[Fixture] = &[
    Fixture { file: "backup_2026-01-11_slot0_prefix128k.bin", ga_end: 44546, offset: 81077, confident: true },
    Fixture { file: "backup_2026-01-11_slot1_prefix128k.bin", ga_end: 41448, offset: 76758, confident: false },
    Fixture { file: "backup_2026-01-11_slot2_prefix128k.bin", ga_end: 41448, offset: 76787, confident: true },
    Fixture { file: "backup_2026-01-11_slot3_prefix128k.bin", ga_end: 41448, offset: 76787, confident: true },
    Fixture { file: "backup_2026-01-11_slot4_prefix128k.bin", ga_end: 41448, offset: 76779, confident: true },
    Fixture { file: "confessor_lvl93_slot0_prefix128k.bin", ga_end: 44664, offset: 81251, confident: false },
    Fixture { file: "b24_watchdog_before_slot0_prefix128k.bin", ga_end: 45105, offset: 81644, confident: false },
    Fixture { file: "b25_watchdog_after_slot0_prefix128k.bin", ga_end: 45121, offset: 81660, confident: false },
];

fn load(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {path}: {e}"))
}

#[test]
fn golden_detection_results() {
    for f in FIXTURES {
        let data = load(f.file);
        assert_eq!(parse_ga_items_end(&data), f.ga_end, "{}: gaEnd", f.file);
        let det = detect_event_flags_offset_impl(&data);
        assert_eq!(det.offset, f.offset, "{}: offset", f.file);
        assert_eq!(det.confident, f.confident, "{}: confident", f.file);
    }
}

#[test]
fn detection_stays_in_ga_end_window() {
    // The lookalike regions (~106k content echo, ~222k struct-walk position)
    // must be unreachable: every detection lands in [gaEnd+30k, gaEnd+45k].
    for f in FIXTURES {
        let data = load(f.file);
        let det = detect_event_flags_offset_impl(&data);
        let delta = det.offset as i64 - f.ga_end;
        assert!(
            (30_000..45_000).contains(&delta),
            "{}: delta {} outside window",
            f.file,
            delta
        );
    }
}

#[test]
fn tier1_anchor_bits_present_at_detected_offset() {
    // All fixture characters touched the tutorial graces, so the tier-1
    // anchor bits must be set at the detected grace-family base.
    for f in FIXTURES {
        let data = load(f.file);
        let det = detect_event_flags_offset_impl(&data);
        let b2725 = data[det.offset + 2725];
        let b3262 = data[det.offset + 3262];
        assert_eq!(b2725 & 0xC0, 0xC0, "{}: 71800/71801 bits", f.file);
        assert_eq!(b3262 & 0x0C, 0x0C, "{}: 76100/76101 bits", f.file);
    }
}

#[test]
fn ga_items_growth_tracked_across_kill_pair() {
    // b24 -> b25 (Erdtree Burial Watchdog kill + loot): GaItems grew by 16
    // bytes and the detected grace base moved with it. Detection must track
    // per-save layout churn instead of assuming stable absolute offsets.
    let b24 = load("b24_watchdog_before_slot0_prefix128k.bin");
    let b25 = load("b25_watchdog_after_slot0_prefix128k.bin");
    let d24 = detect_event_flags_offset_impl(&b24);
    let d25 = detect_event_flags_offset_impl(&b25);
    assert_eq!(parse_ga_items_end(&b25) - parse_ga_items_end(&b24), 16);
    assert_eq!(d25.offset as i64 - d24.offset as i64, 16);
}
