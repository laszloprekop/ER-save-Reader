//! `ResolvedFlags` must agree, bit for bit, with the readers it replaces.
//!
//! The free `is_*_set` functions resolved the origin on every call; `ResolvedFlags`
//! resolves it once and answers from cached bases. That is only a refactor if the
//! answers are identical — including the refusals, which is the half that matters,
//! because a resolution difference shows up as `Clear` rather than as an error.
//!
//! These tests deliberately call the deprecated functions: comparing old against
//! new is the whole point, and the deprecation is a migration signal for callers,
//! not a claim that the old answers were wrong.
#![allow(deprecated)]

use wasm_event_flags::{
    dungeon_flag_state, dungeon_pickup_state, is_dungeon_flag_set, is_dungeon_pickup_set,
    is_tile_pickup_set, is_tile_world_flag_set, is_world_state_flag_set, tile_pickup_state,
    tile_world_flag_state, world_state_flag_state, FlagState, ResolvedFlags,
    FAMILY_LEGACY_DUNGEON, FAMILY_LEGACY_DUNGEON_PICKUP, FAMILY_TILE_OPEN_WORLD,
    FAMILY_TILE_PICKUP_ROW_ID, FAMILY_WORLD_STATE_B,
};

/// Same construction as `origin_conformance.rs`: a single non-zero marker at
/// 20,000 puts the list end in the detectable range, far from any family base.
fn synthetic_ef(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    buf[20_000] = 0x01;
    buf
}

/// One id per family, plus ids that belong to none.
const IDS: &[u32] = &[
    76_100,        // world-state-b (The First Step)
    71_800,        // world-state-b (Cave of Knowledge)
    1_042_370_800, // tile-open-world (Crucible Knight)
    1_033_450_800, // tile-open-world (Bols)
    1_044_360_310, // tile-pickup-row-id
    30_020_800,    // legacy-dungeon
    30_027_000,    // legacy-dungeon-pickup
    2_045_450_800, // DLC tile: no verified layout
    123_456,       // six-digit: no family
    0,
    u32::MAX,
];

/// Every method must return exactly what the function it replaces returned, on a
/// region where the origin resolves.
#[test]
fn methods_agree_with_the_functions_they_replace() {
    let buf = synthetic_ef(2_100_000);
    let flags = ResolvedFlags::from_event_flags(&buf).expect("origin should resolve");

    for &id in IDS {
        assert_eq!(
            Option::<bool>::from(flags.world_state(id)),
            is_world_state_flag_set(&buf, id),
            "world_state disagrees on {id}"
        );
        assert_eq!(
            Option::<bool>::from(flags.tile_world(id)),
            is_tile_world_flag_set(&buf, id),
            "tile_world disagrees on {id}"
        );
        assert_eq!(
            Option::<bool>::from(flags.tile_pickup(id)),
            is_tile_pickup_set(&buf, id),
            "tile_pickup disagrees on {id}"
        );
        assert_eq!(
            Option::<bool>::from(flags.dungeon(id)),
            is_dungeon_flag_set(&buf, id),
            "dungeon disagrees on {id}"
        );
        assert_eq!(
            Option::<bool>::from(flags.dungeon_pickup(id)),
            is_dungeon_pickup_set(&buf, id),
            "dungeon_pickup disagrees on {id}"
        );
    }
}

/// Agreement must hold with bits actually SET, not only on an all-clear region —
/// an all-zero buffer would let a wrong base pass by reading zero either way.
#[test]
fn methods_agree_when_the_bits_are_set() {
    let mut buf = synthetic_ef(2_100_000);

    // Set every byte of each family's region so any plausible base reads SET.
    for constant in [
        FAMILY_WORLD_STATE_B,
        FAMILY_TILE_OPEN_WORLD,
        FAMILY_TILE_PICKUP_ROW_ID,
        FAMILY_LEGACY_DUNGEON,
        FAMILY_LEGACY_DUNGEON_PICKUP,
    ] {
        let probe = ResolvedFlags::from_event_flags(&buf).unwrap();
        if let Some(base) = probe.family_base(constant) {
            let end = (base + 40_000).min(buf.len());
            buf[base..end].fill(0xff);
        }
    }

    let flags = ResolvedFlags::from_event_flags(&buf).expect("origin should still resolve");
    let mut any_set = false;
    for &id in IDS {
        for (got, want) in [
            (flags.world_state(id), is_world_state_flag_set(&buf, id)),
            (flags.tile_world(id), is_tile_world_flag_set(&buf, id)),
            (flags.tile_pickup(id), is_tile_pickup_set(&buf, id)),
            (flags.dungeon(id), is_dungeon_flag_set(&buf, id)),
            (flags.dungeon_pickup(id), is_dungeon_pickup_set(&buf, id)),
        ] {
            assert_eq!(Option::<bool>::from(got), want, "disagreement on {id}");
            any_set |= got == FlagState::Set;
        }
    }
    assert!(any_set, "test is vacuous unless at least one read came back Set");
}

/// The refusals must match too. An unresolvable region yields no `ResolvedFlags`
/// at all — the refusal happens once, at construction, instead of per flag.
#[test]
fn construction_refuses_exactly_where_the_functions_refused() {
    for buf in [vec![0u8; 0], vec![0u8; 40_000], vec![0xffu8; 100_000]] {
        assert!(
            ResolvedFlags::from_event_flags(&buf).is_none(),
            "origin must not resolve on a {}-byte region of this shape",
            buf.len()
        );
        // And the functions agree that nothing is readable there.
        assert_eq!(is_world_state_flag_set(&buf, 76_100), None);
        assert_eq!(is_dungeon_flag_set(&buf, 30_020_800), None);
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
    for constant in [
        FAMILY_WORLD_STATE_B,
        FAMILY_TILE_OPEN_WORLD,
        FAMILY_TILE_PICKUP_ROW_ID,
        FAMILY_LEGACY_DUNGEON,
        FAMILY_LEGACY_DUNGEON_PICKUP,
    ] {
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
    for &id in IDS {
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
