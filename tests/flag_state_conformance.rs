//! Unknown is not Clear (`CONTEXT.md` → Unknown).
//!
//! A flag read has three outcomes, not two: set, clear, and *we could not tell*.
//! Since v0.37.8 the third is `FlagState::Unknown`, and the router refuses in two
//! stages: an unresolvable region yields no `ResolvedFlags` at all, and even a
//! resolved region answers `Unknown` for an id whose family has no verified
//! layout. Collapsing either to "not collected" is the failure that made
//! `batch-validate` report 0/110 boss defeats on a finished character.
//!
//! These tests cross the library seam deliberately. `src/db/pickup_flags.rs` has
//! inline tests that reach its internals; this file can only see what `src/lib.rs`
//! re-exports, which is the same view a caller has. That is the point — the
//! interface is the test surface.
//!
//! They need no fixture, so they run on a fresh clone.

use er_save_reader::{pickup_state, FlagState, ResolvedFlags};

/// Stage one of the refusal: a region whose origin cannot be located yields no
/// `ResolvedFlags`, so there is nothing to read a flag from — every read is
/// Unknown by construction, not by a per-flag `false`.
#[test]
fn unresolvable_region_yields_no_resolved_flags() {
    assert!(
        ResolvedFlags::from_event_flags(&[]).is_none(),
        "an empty region has no origin; construction must refuse"
    );
    // The right shape but all zeroes still has no append-only list to find its
    // end of — "plausible bytes" is not "resolvable".
    assert!(
        ResolvedFlags::from_event_flags(&vec![0u8; 1 << 20]).is_none(),
        "a zeroed 1MB region has no list end; construction must refuse"
    );
}

/// The synthetic region the wasm origin-conformance tests use: a single non-zero
/// marker at 20,000 puts the list end in the detectable range, so construction
/// succeeds and family reads can be exercised.
fn resolvable_region() -> Vec<u8> {
    let mut buf = vec![0u8; 2_100_000];
    buf[20_000] = 0x01;
    buf
}

/// Stage two: on a region that DID resolve, an id belonging to no known family
/// still reads Unknown. Holding a `ResolvedFlags` promises the origin was found,
/// not that any given id can be placed.
#[test]
fn out_of_family_ids_read_unknown_even_when_resolved() {
    let region = resolvable_region();
    let flags = ResolvedFlags::from_event_flags(&region)
        .expect("the synthetic region should resolve");

    let unrouted = [
        2_045_450_800, // DLC tile: no verified layout
        123_456,       // six-digit: belongs to no known family
        0,             // not an id at all
        u32::MAX,
    ];
    for id in unrouted {
        assert_eq!(
            pickup_state(&flags, id),
            FlagState::Unknown,
            "unrouted id {id} must read Unknown, never Clear"
        );
    }
}

/// On a resolved region a placeable id reads a definite `Clear`, not a collapsed
/// `Unknown` — so the router distinguishes "we read it, the bit is 0" from "we
/// could not read it". The world-state grace 76100 is unconditionally placeable
/// (fixed layout `(flag-50000)/8`), unlike tile/dungeon ids whose placement
/// depends on map allocation and is covered by the wasm crate's conformance.
#[test]
fn a_placeable_id_on_a_resolved_region_reads_clear_not_unknown() {
    let region = resolvable_region();
    let flags = ResolvedFlags::from_event_flags(&region).unwrap();

    assert_eq!(
        pickup_state(&flags, 76_100),
        FlagState::Clear,
        "a zero bit at a resolved position is Clear, not Unknown"
    );
}
