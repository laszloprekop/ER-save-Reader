//! `ResolvedFlags` resolves the origin once and answers every family from cached
//! bases. This file pins the properties that makes it correct rather than merely
//! fast: a filled region reads Set (not vacuously Clear), the two tile families
//! stay separate, every base is the origin plus its constant, the wasm exports
//! agree with the methods, and an unresolvable region refuses at construction —
//! yielding Unknown, never Clear.
//!
//! It once compared these methods bit-for-bit against the deprecated free
//! `is_*_set` readers they replaced; those readers were deleted in v0.37.9, so
//! the assertions now name the expected `FlagState` directly.

use wasm_event_flags::{
    dungeon_flag_state, dungeon_pickup_state, tile_pickup_state, tile_world_flag_state,
    world_state_flag_state, FlagState, ResolvedFlags, FAMILY_LEGACY_DUNGEON,
    FAMILY_LEGACY_DUNGEON_PICKUP, FAMILY_TILE_OPEN_WORLD, FAMILY_TILE_PICKUP_ROW_ID,
    FAMILY_WORLD_STATE_B,
};

/// Same construction as `origin_conformance.rs`: a single non-zero marker at
/// 20,000 puts the list end in the detectable range, far from any family base.
fn synthetic_ef(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    buf[20_000] = 0x01;
    buf
}

/// The five family constants, in the order the model lists them.
const FAMILIES: &[i64] = &[
    FAMILY_WORLD_STATE_B,
    FAMILY_TILE_OPEN_WORLD,
    FAMILY_TILE_PICKUP_ROW_ID,
    FAMILY_LEGACY_DUNGEON,
    FAMILY_LEGACY_DUNGEON_PICKUP,
];

/// One placeable id per family, paired with the method that reads it. Every one
/// has a verified layout, so on a resolved region each returns a definite state.
fn placeable_reads(flags: &ResolvedFlags) -> Vec<(FlagState, &'static str)> {
    vec![
        (flags.world_state(76_100), "world_state (The First Step)"),
        (flags.world_state(71_800), "world_state (Cave of Knowledge)"),
        (flags.tile_world(1_042_370_800), "tile_world (Crucible Knight)"),
        (flags.tile_world(1_033_450_800), "tile_world (Bols)"),
        (flags.tile_pickup(1_044_360_310), "tile_pickup"),
        (flags.dungeon(30_020_800), "dungeon"),
        (flags.dungeon_pickup(30_027_000), "dungeon_pickup"),
    ]
}

/// On an all-clear resolved region every placeable id reads Clear — not Unknown.
/// Holding a `ResolvedFlags` means the origin was found, so a placeable id has a
/// definite answer; the region just happens to have the bit clear.
#[test]
fn placeable_ids_read_clear_on_an_all_clear_region() {
    let buf = synthetic_ef(2_100_000);
    let flags = ResolvedFlags::from_event_flags(&buf).expect("origin should resolve");
    for (got, label) in placeable_reads(&flags) {
        assert_eq!(got, FlagState::Clear, "{label} should read Clear on an empty region");
    }
}

/// The same ids read Set once their exact bit is set. An all-clear region alone
/// would let a wrong base pass by reading zero either way, so here each id's true
/// byte+bit is resolved via `flag_offset_in_ef` and only that one bit is set —
/// then the method for that family must read it back as Set. This exercises the
/// full round trip (resolve position → set bit → resolve state) per family,
/// including the tile families whose geometry lands hundreds of KB past the base.
#[test]
fn placeable_ids_read_set_when_their_exact_bit_is_set() {
    use wasm_event_flags::{
        flag_offset_in_ef, FAMILY_CODE_DUNGEON, FAMILY_CODE_DUNGEON_PICKUP, FAMILY_CODE_TILE_PICKUP,
        FAMILY_CODE_TILE_WORLD, FAMILY_CODE_WORLD_STATE,
    };

    // (family code, one placeable id in that family, label). The reader is chosen
    // by the same family code below — a value never routes itself.
    let cases = [
        (FAMILY_CODE_WORLD_STATE, 76_100u32, "world_state (The First Step)"),
        (FAMILY_CODE_TILE_WORLD, 1_042_370_800, "tile_world (Crucible Knight)"),
        (FAMILY_CODE_TILE_PICKUP, 1_044_360_310, "tile_pickup"),
        (FAMILY_CODE_DUNGEON, 30_020_800, "dungeon"),
        (FAMILY_CODE_DUNGEON_PICKUP, 30_027_000, "dungeon_pickup"),
    ];

    for (family, id, label) in cases {
        let mut buf = synthetic_ef(4_200_000);
        let off = flag_offset_in_ef(&buf, id, family);
        assert!(off.valid, "{label}: position must resolve in a region this size");
        buf[off.byte_offset as usize] |= 1 << off.bit_position;

        let flags = ResolvedFlags::from_event_flags(&buf).expect("origin should resolve");
        let got = match family {
            FAMILY_CODE_WORLD_STATE => flags.world_state(id),
            FAMILY_CODE_TILE_WORLD => flags.tile_world(id),
            FAMILY_CODE_TILE_PICKUP => flags.tile_pickup(id),
            FAMILY_CODE_DUNGEON => flags.dungeon(id),
            FAMILY_CODE_DUNGEON_PICKUP => flags.dungeon_pickup(id),
            _ => unreachable!(),
        };
        assert_eq!(got, FlagState::Set, "{label}: the exact bit set must read Set");
    }
}

/// An unresolvable region yields no `ResolvedFlags` at all — the refusal happens
/// once, at construction. The wasm exports, which resolve internally, then report
/// Unknown (-1) for every id, never Clear (0).
#[test]
fn construction_refuses_on_unresolvable_regions() {
    for buf in [vec![0u8; 0], vec![0u8; 40_000], vec![0xffu8; 100_000]] {
        assert!(
            ResolvedFlags::from_event_flags(&buf).is_none(),
            "origin must not resolve on a {}-byte region of this shape",
            buf.len()
        );
        // Nothing is readable there: Unknown, not Clear.
        assert_eq!(world_state_flag_state(&buf, 76_100), -1);
        assert_eq!(dungeon_flag_state(&buf, 30_020_800), -1);
    }
}

/// A resolved region still answers Unknown for ids with no verified layout.
/// Holding a `ResolvedFlags` promises the origin was found, not that every flag
/// can be read.
#[test]
fn resolved_region_still_answers_unknown_for_unplaceable_ids() {
    let buf = synthetic_ef(2_100_000);
    let flags = ResolvedFlags::from_event_flags(&buf).unwrap();

    assert_eq!(flags.tile_world(2_045_450_800), FlagState::Unknown, "DLC tile");
    assert_eq!(flags.world_state(123_456), FlagState::Unknown, "no family");
    assert_eq!(flags.world_state(1_042_370_800), FlagState::Unknown, "wrong family");
    assert_eq!(flags.dungeon(30_027_000), FlagState::Unknown, "pickup id via event reader");
    assert_eq!(flags.dungeon_pickup(30_020_800), FlagState::Unknown, "event id via pickup reader");
}

/// The two tile families sit 500 bytes apart and a bare 10-digit id cannot say
/// which it belongs to. `ResolvedFlags` must keep them separate rather than
/// picking one — the caller chooses by choosing a method.
#[test]
fn the_two_tile_families_stay_distinct() {
    let buf = synthetic_ef(2_100_000);
    let flags = ResolvedFlags::from_event_flags(&buf).unwrap();
    let world = flags.family_base(FAMILY_TILE_OPEN_WORLD).unwrap();
    let pickup = flags.family_base(FAMILY_TILE_PICKUP_ROW_ID).unwrap();
    assert_eq!(pickup - world, 500, "the families' separation is what makes them distinct");
}

/// Every family base is the origin plus that family's constant. This is the
/// whole model in one assertion.
#[test]
fn every_base_is_the_origin_plus_its_constant() {
    let buf = synthetic_ef(2_100_000);
    let flags = ResolvedFlags::from_event_flags(&buf).unwrap();
    let origin = flags.origin() as i64;
    for &constant in FAMILIES {
        assert_eq!(
            flags.family_base(constant).map(|b| b as i64),
            Some(origin + constant),
            "family {constant} is not at origin + constant"
        );
    }
    assert_eq!(flags.family_base(999_999), None, "unknown constant names no family");
}

/// The wasm exports encode the same three states, and must agree with the
/// methods now that they route through them.
#[test]
fn wasm_exports_agree_with_the_methods() {
    let buf = synthetic_ef(2_100_000);
    let flags = ResolvedFlags::from_event_flags(&buf).unwrap();
    let ids = [76_100, 1_042_370_800, 1_044_360_310, 30_020_800, 30_027_000, 123_456, u32::MAX];
    for id in ids {
        assert_eq!(world_state_flag_state(&buf, id), flags.world_state(id).as_i32());
        assert_eq!(tile_world_flag_state(&buf, id), flags.tile_world(id).as_i32());
        assert_eq!(tile_pickup_state(&buf, id), flags.tile_pickup(id).as_i32());
        assert_eq!(dungeon_flag_state(&buf, id), flags.dungeon(id).as_i32());
        assert_eq!(dungeon_pickup_state(&buf, id), flags.dungeon_pickup(id).as_i32());
    }
    // -1 is Unknown, and an unresolvable region gives it for everything.
    assert_eq!(world_state_flag_state(&[], 76_100), -1);
    assert_eq!(FlagState::Unknown.as_i32(), -1);
}

/// `unknown_as_clear` is the only way back to a bool, and it must treat Unknown
/// as clear rather than as set — the safe direction for a filter.
#[test]
fn unknown_as_clear_is_the_only_narrowing_and_it_is_conservative() {
    assert!(FlagState::Set.unknown_as_clear());
    assert!(!FlagState::Clear.unknown_as_clear());
    assert!(!FlagState::Unknown.unknown_as_clear());
}
