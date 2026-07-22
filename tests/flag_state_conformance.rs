//! Unknown is not Clear (`CONTEXT.md` → Unknown).
//!
//! A flag read has three outcomes, not two: set, clear, and *we could not tell*.
//! The third is what `Option<bool>::None` means everywhere in this crate, and
//! collapsing it to `false` is the failure that made `batch-validate` report 0/110
//! boss defeats on a finished character — every one of them Unknown, every one of
//! them rendered as "not defeated".
//!
//! These tests cross the library seam deliberately. `src/db/pickup_flags.rs` has 21
//! inline tests that reach its internals; this file can only see what `src/lib.rs`
//! re-exports, which is the same view a caller has. That is the point — the
//! interface is the test surface, and until this crate had a `[lib]` target there
//! was no way to stand outside it and look in.
//!
//! They need no fixture, so they run on a fresh clone.

use er_save_reader::pickup_flag_state;

/// An empty flag region resolves no Origin, so every family base is unknown and
/// every read must be Unknown — never "not collected".
#[test]
fn unresolvable_region_reads_unknown_not_clear() {
    // One id from each family `pickup_flag_state` routes to, so a regression in
    // any single branch fails here rather than in one screen at runtime.
    let ids = [
        1_043_500_010, // tile-pickup-row-id (10-digit, 1xxxxxxxxx)
        30_000_000,    // legacy-dungeon-pickup (8-digit, localId >= 7000)
        76_100,        // world-state-b (5-digit, 50000..80000)
    ];

    for id in ids {
        assert_eq!(
            pickup_flag_state(&[], id),
            None,
            "flag {id} on an empty region must read Unknown, not Some(false). \
             `Some(false)` here would render as a confident 'not collected'."
        );
    }
}

/// A region of the right shape but all zeroes still has no Origin: the resolver
/// scans for the end of the append-only record list, and there is no list here.
/// "Plausible bytes" must not be mistaken for "resolvable".
#[test]
fn zeroed_region_of_realistic_size_still_reads_unknown() {
    let region = vec![0u8; 1 << 20];
    assert_eq!(pickup_flag_state(&region, 1_043_500_010), None);
    assert_eq!(pickup_flag_state(&region, 76_100), None);
}

/// Ids belonging to no known family are Unknown regardless of the region — the
/// DLC tiles (`2xxxxxxxxx`) and the ~935 six-digit ids in `WORLD_PICKUPS` that
/// have no verified layout. Answering `false` for these would report every DLC
/// pickup as uncollected on a completed character.
#[test]
fn out_of_family_ids_read_unknown() {
    let unrouted = [
        2_045_450_800, // DLC tile: no verified layout
        123_456,       // six-digit: belongs to no known family
        0,             // not an id at all
        u32::MAX,
    ];

    for id in unrouted {
        assert_eq!(
            pickup_flag_state(&[], id),
            None,
            "unrouted id {id} must read Unknown"
        );
    }
}
