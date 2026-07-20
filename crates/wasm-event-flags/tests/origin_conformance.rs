//! Conformance for the event-flag origin resolver (docs/BACKLOG.md step 4b).
//!
//! The resolver positions flag families in a save with no history by measuring
//! from the end of the append-only u32 list that pushes them along. These golden
//! values were measured 2026-07-20 and validated out-of-sample against V1/V2/V3
//! (exact expected bit patterns) and the Wretch slot — see
//! knowledge/claims/origin-validation.json.
//!
//! A change here means the origin moved. That silently corrupts every flag read
//! downstream, so treat a diff as a regression until proven otherwise.

use std::fs;
use std::path::Path;
use wasm_event_flags::{
    find_flag_list_end, find_ga_items_end_pub as ga_items_end, resolve_family_base,
    FAMILY_LEGACY_DUNGEON_PICKUP, FAMILY_TILE_PICKUP_ROW_ID, FAMILY_WORLD_STATE_B,
    ORIGIN_MAX_GAP, ORIGIN_MIN_GAP,
};

/// (fixture, ga_end, list_end offset from ga_end)
const GOLDEN: &[(&str, usize, usize)] = &[
    ("backup_2026-01-11_slot0_prefix128k.bin", 44546, 65933),
    ("backup_2026-01-11_slot1_prefix128k.bin", 41448, 63661),
    ("backup_2026-01-11_slot2_prefix128k.bin", 41448, 63641),
    ("backup_2026-01-11_slot3_prefix128k.bin", 41448, 63637),
    ("backup_2026-01-11_slot4_prefix128k.bin", 41448, 63629),
    ("confessor_lvl93_slot0_prefix128k.bin", 44664, 65909),
    ("b24_watchdog_before_slot0_prefix128k.bin", 45105, 65945),
    ("b25_watchdog_after_slot0_prefix128k.bin", 45121, 65949),
];

fn load(name: &str) -> Vec<u8> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read(&p).unwrap_or_else(|e| panic!("{}: {}", p.display(), e))
}

#[test]
fn list_end_matches_golden() {
    for (name, ga, end) in GOLDEN {
        let data = load(name);
        assert_eq!(ga_items_end(&data), Some(*ga), "{}: ga_end", name);
        assert_eq!(find_flag_list_end(&data), Some(*end), "{}: list end", name);
    }
}

#[test]
fn list_end_stays_in_declared_range() {
    // The resolver's own sanity bounds must actually contain real saves, with
    // margin — bounds that only just fit are bounds about to reject a valid one.
    for (name, _, end) in GOLDEN {
        assert!(
            (ORIGIN_MIN_GAP..=ORIGIN_MAX_GAP).contains(end),
            "{}: golden list end {} outside declared range [{}, {}]",
            name,
            end,
            ORIGIN_MIN_GAP,
            ORIGIN_MAX_GAP
        );
    }
    let lo = GOLDEN.iter().map(|(_, _, e)| *e).min().unwrap();
    let hi = GOLDEN.iter().map(|(_, _, e)| *e).max().unwrap();
    assert!(
        lo - ORIGIN_MIN_GAP >= 2_000 && ORIGIN_MAX_GAP - hi >= 2_000,
        "observed list ends {}..{} sit too close to the bounds [{}, {}]",
        lo,
        hi,
        ORIGIN_MIN_GAP,
        ORIGIN_MAX_GAP
    );
}

#[test]
fn family_constants_reproduce_measured_inter_family_distances() {
    // The families are rigidly locked; these gaps were measured independently
    // of the origin constants (knowledge/claims/family-distances.json) and both
    // chains agreed to the byte. Editing one constant alone breaks that
    // agreement, which is what this guards.
    assert_eq!(FAMILY_TILE_PICKUP_ROW_ID - FAMILY_WORLD_STATE_B, 337_375);
    assert_eq!(FAMILY_LEGACY_DUNGEON_PICKUP - FAMILY_WORLD_STATE_B, 1_383_250);
    assert_eq!(
        FAMILY_LEGACY_DUNGEON_PICKUP - FAMILY_TILE_PICKUP_ROW_ID,
        1_045_875
    );
}

#[test]
fn refuses_bases_that_fall_outside_the_data() {
    // The fixtures are 128k prefixes; every family base lands past that. A
    // resolver that returned an offset here would be inviting an out-of-bounds
    // or wrapped read downstream.
    for (name, _, _) in GOLDEN {
        let data = load(name);
        assert!(
            find_flag_list_end(&data).is_some(),
            "{}: list end should still resolve in a prefix",
            name
        );
        assert_eq!(
            resolve_family_base(&data, FAMILY_WORLD_STATE_B),
            None,
            "{}: base lies beyond a 128k prefix and must be refused",
            name
        );
    }
}

#[test]
fn refuses_rather_than_guesses_on_unusable_input() {
    // The whole point of the hardening: no plausible-looking wrong answers.
    assert_eq!(find_flag_list_end(&[]), None, "empty");
    assert_eq!(find_flag_list_end(&vec![0u8; 4096]), None, "too short");
    assert_eq!(
        find_flag_list_end(&vec![0u8; 300_000]),
        None,
        "all zeros: no list exists, must not report one"
    );
    let mut truncated = load(GOLDEN[0].0);
    truncated.truncate(GOLDEN[0].1 + 60_000);
    assert_eq!(
        find_flag_list_end(&truncated),
        None,
        "truncated before the list end must fail, not return the truncation point"
    );
}

#[test]
fn resolution_is_deterministic() {
    for (name, _, _) in GOLDEN {
        let data = load(name);
        assert_eq!(find_flag_list_end(&data), find_flag_list_end(&data), "{}", name);
    }
}

#[test]
fn ef_relative_path_agrees_with_slot_absolute_path() {
    // The application holds only the flag region, so it uses the EF-anchored
    // resolver. That must locate the SAME list, or the two code paths would
    // disagree about where every family lives — the exact class of bug this
    // work exists to remove.
    use wasm_event_flags::{detect_event_flags_offset_impl, find_flag_list_end_in_ef};
    for (name, ga_end, list_end) in GOLDEN {
        let data = load(name);
        let det = detect_event_flags_offset_impl(&data);
        assert!(det.offset > 0, "{}: no EF offset", name);

        let ef = &data[det.offset..];
        let via_ef = find_flag_list_end_in_ef(ef).unwrap_or_else(|| panic!("{}", name));

        // Both describe the same absolute byte.
        let abs_from_slot = ga_end + list_end;
        let abs_from_ef = det.offset + via_ef;
        assert_eq!(
            abs_from_ef, abs_from_slot,
            "{}: EF path resolved {} but slot path resolved {}",
            name, abs_from_ef, abs_from_slot
        );
    }
}

#[test]
fn world_state_reads_are_unknown_not_false_when_unresolvable() {
    // A truncated region cannot place the base. The read must report UNKNOWN.
    // Collapsing that to "not set" is how a fully progressed character came to
    // display 0/110 boss defeats.
    use wasm_event_flags::is_world_state_flag_set;
    assert_eq!(is_world_state_flag_set(&[], 76100), None, "empty");
    assert_eq!(
        is_world_state_flag_set(&vec![0u8; 40_000], 76100),
        None,
        "region too short to contain the base: must be None, never Some(false)"
    );
    // Out-of-family ids are not this family's business.
    assert_eq!(is_world_state_flag_set(&vec![0u8; 300_000], 1_042_370_800), None);
    assert_eq!(is_world_state_flag_set(&vec![0u8; 300_000], 30_020_800), None);
}
