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
    find_flag_list_end, find_ga_items_end_pub as ga_items_end, legacy_alloc_slot,
    resolve_family_base, FAMILY_LEGACY_DUNGEON, FAMILY_LEGACY_DUNGEON_PICKUP,
    FAMILY_TILE_PICKUP_ROW_ID, FAMILY_WORLD_STATE_B, LEGACY_ALLOC_AMBIGUOUS, ORIGIN_MAX_GAP,
    ORIGIN_MIN_GAP,
};
use wasm_event_flags::{
    dungeon_flag_state, dungeon_pickup_state, flag_offset_in_ef, tile_pickup_state,
    tile_world_flag_state, world_state_flag_state, FAMILY_CODE_DUNGEON,
    FAMILY_CODE_DUNGEON_PICKUP, FAMILY_CODE_TILE_PICKUP, FAMILY_CODE_TILE_WORLD,
    FAMILY_CODE_WORLD_STATE,
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

/// Build a synthetic flag region whose append-only list ends in the declared
/// range, so `resolve_family_base_in_ef` succeeds. A single non-zero marker at
/// 20_000 sits far from any family base; everything else is zero and can be
/// poked without disturbing list-end detection.
fn synthetic_ef(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    buf[20_000] = 0x01;
    buf
}

/// `flag_offset_in_ef` must land on the EXACT byte and bit the tri-state readers
/// use — it shares their base resolution and geometry, and a drift between the
/// two would show the hex view a different bit than the readout claims. Checked
/// in both directions across all five families: set the bit the offset names,
/// confirm the reader now reports SET; clear it, confirm CLEAR.
#[test]
fn offset_export_lands_on_the_same_bit_as_the_readers() {
    // (flag id, family code, tri-state reader)
    let cases: &[(u32, u32, fn(&[u8], u32) -> i32)] = &[
        (76100, FAMILY_CODE_WORLD_STATE, world_state_flag_state),
        (1_042_370_800, FAMILY_CODE_TILE_WORLD, tile_world_flag_state),
        (1_044_360_310, FAMILY_CODE_TILE_PICKUP, tile_pickup_state),
        (30_020_800, FAMILY_CODE_DUNGEON, dungeon_flag_state),
        (30_027_000, FAMILY_CODE_DUNGEON_PICKUP, dungeon_pickup_state),
    ];
    let mut buf = synthetic_ef(2_100_000);
    for &(id, code, reader) in cases {
        let off = flag_offset_in_ef(&buf, id, code);
        assert!(off.valid, "id {id} family {code}: offset should resolve on a valid buffer");
        let (byte, bit) = (off.byte_offset as usize, off.bit_position);

        buf[byte] |= 1 << bit;
        assert_eq!(reader(&buf, id), 1, "id {id}: reader must see SET at the offset's bit");

        buf[byte] &= !(1 << bit);
        assert_eq!(reader(&buf, id), 0, "id {id}: reader must see CLEAR once the bit is cleared");
    }
}

/// The offset export refuses on the same inputs the readers refuse on: an id out
/// of the chosen family, and a buffer whose origin cannot be resolved.
#[test]
fn offset_export_refuses_out_of_family_and_unresolvable() {
    let buf = synthetic_ef(2_100_000);
    // A world-state id addressed as a tile family, and vice versa.
    assert!(!flag_offset_in_ef(&buf, 76100, FAMILY_CODE_TILE_PICKUP).valid);
    assert!(!flag_offset_in_ef(&buf, 1_044_360_310, FAMILY_CODE_WORLD_STATE).valid);
    // A legacy pickup id (localId >= 7000) addressed as the event family.
    assert!(!flag_offset_in_ef(&buf, 30_027_000, FAMILY_CODE_DUNGEON).valid);
    // An unknown family code.
    assert!(!flag_offset_in_ef(&buf, 76100, 99).valid);
    // No list end to find -> unresolvable -> invalid (not offset 0 masquerading as valid).
    assert!(!flag_offset_in_ef(&[], 76100, FAMILY_CODE_WORLD_STATE).valid);
    assert!(!flag_offset_in_ef(&vec![0u8; 40_000], 76100, FAMILY_CODE_WORLD_STATE).valid);
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

#[test]
fn legacy_dungeon_constant_matches_its_measured_distance_from_the_pickups() {
    // Both legacy families were pinned by attributed flips in the SAME two
    // catacombs (knowledge/claims/family-constants.json). Their separation is
    // the only cross-check the constant has, so it is pinned here: an edit to
    // either constant alone moves one family into the other's region.
    assert_eq!(FAMILY_LEGACY_DUNGEON - FAMILY_LEGACY_DUNGEON_PICKUP, 125);
}

#[test]
fn legacy_alloc_table_matches_the_game_alloclists() {
    // The table is a copy of the game's own eventflagalloclists. A copy that
    // drifts from its source is worse than no copy: it reads a wrong bit
    // 1125 bytes per slot away, silently. So the source is re-read here.
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../knowledge/game/eventflag-alloclists.json");
    let raw = fs::read_to_string(&src).unwrap_or_else(|e| panic!("{}: {}", src.display(), e));
    let json: serde_json::Value = serde_json::from_str(&raw).expect("alloclists json");

    // prefix -> every slot the game allocates for it
    let mut want: std::collections::BTreeMap<u32, std::collections::BTreeSet<u16>> =
        Default::default();
    for list in ["legacymap", "legacymap_dlc02"] {
        for e in json["lists"][list]["entries"].as_array().expect("entries") {
            let map = e["map"].as_str().expect("map");
            let prefix: u32 = map[1..3].parse::<u32>().unwrap() * 100 + map[4..6].parse::<u32>().unwrap();
            want.entry(prefix)
                .or_default()
                .insert(e["slot"].as_u64().unwrap() as u16);
        }
    }

    for (prefix, slots) in &want {
        match slots.len() {
            1 => assert_eq!(
                legacy_alloc_slot(*prefix),
                slots.iter().next().copied(),
                "prefix {} disagrees with the alloclists",
                prefix
            ),
            _ => assert_eq!(
                legacy_alloc_slot(*prefix),
                None,
                "prefix {} is allocated {:?}; an ambiguous map must resolve to Unknown",
                prefix,
                slots
            ),
        }
    }
    // and nothing invented: the table has no prefix the game does not allocate
    for p in 0u32..10_000 {
        if legacy_alloc_slot(p).is_some() {
            assert!(want.contains_key(&p), "prefix {} is not in the alloclists", p);
        }
    }
    // the ambiguous set is declared, not merely absent
    for (prefix, slots) in LEGACY_ALLOC_AMBIGUOUS {
        assert_eq!(
            want.get(&(prefix as u32)).map(|s| s.iter().copied().collect::<Vec<_>>()),
            Some(slots.to_vec()),
            "declared ambiguity for {} does not match the alloclists",
            prefix
        );
    }
}

#[test]
fn dungeon_reads_split_by_family_and_refuse_foreign_ids() {
    use wasm_event_flags::{is_dungeon_flag_set, is_dungeon_pickup_set, legacy_dungeon_rel_byte};

    // The layout, on the two flags that established the families.
    // m30_02 is alloc slot 82: 82*1125 = 92,250.
    assert_eq!(legacy_dungeon_rel_byte(30_020_800), Some(92_350)); // + 800/8
    assert_eq!(legacy_dungeon_rel_byte(30_027_000), Some(93_125)); // + 7000/8

    // Each function refuses the other family's ids rather than reading 125
    // bytes into the wrong region.
    let big = vec![0u8; 300_000];
    assert_eq!(is_dungeon_flag_set(&big, 30_027_000), None, "pickup id to event reader");
    assert_eq!(is_dungeon_pickup_set(&big, 30_020_800), None, "event id to pickup reader");

    // Open-world tile ids belong to the tile families; their six-digit prefix
    // must not be truncated into a legacy slot lookup.
    assert_eq!(legacy_dungeon_rel_byte(1_042_370_800), None);
    assert_eq!(is_dungeon_flag_set(&big, 1_042_370_800), None);

    // Unknown and ambiguous maps are Unknown, never "not set".
    assert_eq!(legacy_dungeon_rel_byte(34_120_800), None, "m34_12 is allocated twice");
    assert_eq!(is_dungeon_flag_set(&[], 30_020_800), None, "no origin in an empty region");
}
