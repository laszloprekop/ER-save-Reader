//! WebAssembly module for Elden Ring EventFlags detection and pickup flag calculations
//!
//! This is the **SINGLE SOURCE OF TRUTH** for:
//! - EventFlags offset detection
//! - Pickup flag offset calculations (dungeon, tile, block)
//!
//! Used by both ER-save-Reader (native Rust) and elden-map (via WASM).
//!
//! ## Documentation
//!
//! - ER-save-Reader: `docs/WASM-EVENT-FLAGS.md`
//! - elden-map: `docs/WASM-EVENT-FLAGS.md`
//!
//! ## Rebuilding WASM
//!
//! ```bash
//! cd crates/wasm-event-flags
//! wasm-pack build --target web --out-dir ../../../elden-map/wasm-event-flags
//! ```

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Event flags section size (constant across all saves)
pub const EVENT_FLAGS_SIZE: usize = 0x1BF99F;  // 1,833,375 bytes

/// Search parameters for EventFlags detection
// 2026-07-05: was 0x30000 (196,608), which SKIPPED the real flag region
// (grace-family base ≈ 76k-82k) and made the fallback land on the ~222k
// lookalike. The "skip inventory region" rationale was backwards — the b24/b25
// kill-transition pair proves the flags live in the low region.
pub const SEARCH_START: usize = 0x12000;  // 73,728
pub const MAX_SEARCH_RANGE: usize = 200_000;

/// Tile flag constants (10-digit flags like 1035537020)
pub const TILE_ROW_BASE: u32 = 33;
pub const TILE_COL_BASE: u32 = 30;
pub const TILE_BYTES_PER_SLOT: u32 = 875;
pub const TILE_SLOTS_PER_ROW: u32 = 40;
pub const TILE_MAX_LOCAL_ID: u32 = 6999;

// TILE_BASE_OFFSET (337375) and WORLD_PICKUP_ROW_ID_BASE (1037373320) removed 2026-07-20
// (ADR-0008). 337375 is real but it is the distance BETWEEN two flag families, never a
// base — see CLAUDE.md and tombstone `tile-base-337375-grace-anchored`. The row_id base
// belonged to a storage model disproven 2026-02-16. Neither had a caller left that was
// not itself being removed.

/// Dungeon section size
pub const DUNGEON_SECTION_SIZE: u32 = 1125;

/// Maximum local ID for tile flags (7000+ use row_id formula instead)
pub const MAX_TILE_LOCAL_ID: u32 = 6999;

/// Player coordinate extraction constants
/// (from ground_truth_offsets.json player_coords_extraction)
pub const PLAYER_COORDS_SEARCH_START: usize = 0x1D0000;  // 1,900,544
pub const PLAYER_COORDS_SEARCH_END: usize = 0x280000;    // 2,621,440
pub const PLAYER_COORDS_STRUCT_SIZE: usize = 61;
pub const MID_SECTION_SIZE: usize = 17;
pub const MID_SECTION_MIN_ZEROS: usize = 10;
pub const FACING_ANGLE_OFFSET: usize = 4;
pub const PAD2_SIZE: usize = 16;
pub const PAD2_MIN_ZEROS: usize = 8;
pub const COORD_RANGE_MAX: f32 = 10000.0;
pub const MAGNITUDE_THRESHOLD: f32 = 10.0;
pub const FACING_ANGLE_MAX: f32 = std::f32::consts::TAU; // 2π ≈ 6.283

/// Check if an f32 is denormalized (exponent bits all zero, but value non-zero).
/// Denormalized floats indicate garbage data, not real game coordinates.
fn is_denormalized(v: f32) -> bool {
    v != 0.0 && (v.to_bits() & 0x7f800000) == 0
}

// =============================================================================
// EVENTFLAGS DETECTION
// =============================================================================

/// Validation flag for detecting EventFlags offset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationFlag {
    pub flag_id: u32,
    pub byte_offset: u32,
    pub bit_position: u8,
    pub name: String,
    #[serde(default)]
    pub tier: u8,
}

/// Known grace flags used to validate EventFlags offset detection (POSITIVE).
pub const POSITIVE_VALIDATION_FLAGS: &[(u32, u32, u8, &str, u8)] = &[
    // Tier 1: Tutorial and first graces (MUST be set)
    (71800, 2725, 7, "Cave of Knowledge", 1),
    (71801, 2725, 6, "Stranded Graveyard", 1),
    (76100, 3262, 3, "The First Step", 1),
    (76101, 3262, 2, "Church of Elleh", 1),
    // Tier 2: Early game graces (likely set for most characters)
    (76102, 3262, 1, "Stormhill Shack", 2),
    (76104, 3263, 7, "Agheel Lake South", 2),
    (76106, 3263, 5, "Church of Dragon Communion", 2),
];

/// Late-game grace flags used for NEGATIVE validation.
pub const NEGATIVE_VALIDATION_FLAGS: &[(u32, u32, u8, &str)] = &[
    (76223, 3277, 0, "Fortified Manor, First Floor"),
    (76224, 3278, 7, "East Capital Rampart"),
    (76225, 3278, 6, "Divine Bridge"),
    (76300, 3287, 3, "Zamor Ruins"),
    (76301, 3287, 2, "Ancient Snow Valley Ruins"),
    (76350, 3293, 5, "Haligtree Town"),
];

/// Result of EventFlags offset detection
#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub offset: usize,
    pub positive_score: usize,
    pub negative_score: usize,
    pub confident: bool,
}

/// Detect the EventFlags offset within slot data.
#[wasm_bindgen]
pub fn detect_event_flags_offset(slot_data: &[u8]) -> DetectionResult {
    detect_event_flags_offset_impl(slot_data)
}

/// Minimum tier1 score to accept a content-based candidate
const MIN_TIER1_SCORE: usize = 2;

/// Window for the gaEnd-anchored scan: grace-family base minus GaItems end,
/// measured across saves (Bee timeline Feb-2026 era: 35,111..35,207;
/// 2026-01-11 backup slots: 35,437..37,021). Generous margins on both sides.
const EF_WINDOW_AFTER_GA_END: core::ops::Range<usize> = 30_000..45_000;

/// Internal implementation (also usable from native Rust without WASM)
///
/// Detection strategy (2026-07-05 rework, see docs/adr/0003 and BACKLOG Priority 0b):
/// 1. **gaEnd-windowed content scan** (primary): parse GaItems end (byte-exact,
///    verified via PlayerGameData name position), then scan
///    [gaEnd+30k, gaEnd+45k] scoring the grace validation flags. The tight window
///    makes the known lookalike regions (~106k content echo, ~222k struct-walk
///    position) unreachable.
/// 2. **Full-range content search** (fallback): only if GaItems parsing fails or
///    the windowed scan finds no acceptable candidate.
///
/// The former "structural computation" is intentionally NOT used for detection:
/// its section model overshoots the real flag region by ~146k bytes (empirically
/// disproven by the b24/b25 kill-transition pair: flag 30020800 flips at the
/// windowed position, and the struct-walk position stays zero). It poisoned all
/// consumers from ~Mar 2026 ("real EF at ~222K" was a lookalike).
///
/// CAVEAT (per-family float): the returned offset is the GRACE-FAMILY base.
/// Other flag families (catacombs, tiles, ...) float independently per save by
/// up to a few hundred bytes and need their own calibration; do not treat this
/// offset as a universal anchor for all families.
pub fn detect_event_flags_offset_impl(slot_data: &[u8]) -> DetectionResult {
    // === PRIMARY: gaEnd-windowed content scan ===
    if let Some(ga_end) = find_ga_items_end(slot_data) {
        if let Some(result) = detect_in_window(slot_data, ga_end) {
            return result;
        }
    }

    // === FALLBACK: Content-based search over the legacy full range ===
    detect_event_flags_content_based(slot_data)
}

/// Scan the gaEnd-anchored window for the best grace-validation candidate.
/// Returns None if no candidate reaches MIN_TIER1_SCORE (e.g. a character that
/// has not touched the tutorial graces yet).
fn detect_in_window(slot_data: &[u8], ga_end: usize) -> Option<DetectionResult> {
    let lo = ga_end + EF_WINDOW_AFTER_GA_END.start;
    let hi = (ga_end + EF_WINDOW_AFTER_GA_END.end).min(slot_data.len().saturating_sub(4096));
    if lo >= hi {
        return None;
    }

    let mut best: Option<(usize, usize, usize, usize)> = None; // (tier1, pos, neg, offset)
    for offset in lo..hi {
        let (tier1, pos, neg) = validate_at_offset(slot_data, offset);
        if tier1 < MIN_TIER1_SCORE {
            continue;
        }
        let better = match best {
            None => true,
            // prefer higher tier1, then higher pos, then higher neg;
            // on full ties keep the FIRST (lowest offset): scoring plateaus are
            // small shifted echoes, and the low edge matched the byte-exact
            // c=0 verification on the Bee timeline (sd_000259).
            Some((bt, bp, bn, _)) => (tier1, pos, neg) > (bt, bp, bn),
        };
        if better {
            best = Some((tier1, pos, neg, offset));
        }
    }

    let (tier1, pos, neg, offset) = best?;
    // Confidence: all tier-1 anchors present and at most one late-game negative
    // violation (mid/late-game characters legitimately set some).
    let confident = tier1 >= 3 && neg >= NEGATIVE_VALIDATION_FLAGS.len() - 1;
    Some(DetectionResult {
        offset,
        positive_score: pos,
        negative_score: neg,
        confident,
    })
}

/// Validate grace flags at a candidate EventFlags offset.
/// Returns (tier1_score, positive_score, negative_score).
fn validate_at_offset(slot_data: &[u8], offset: usize) -> (usize, usize, usize) {
    let mut tier1_score = 0;
    let mut positive_score = 0;
    let mut negative_score = 0;

    for &(_, byte_offset, bit_pos, _, tier) in POSITIVE_VALIDATION_FLAGS {
        let abs_pos = offset + byte_offset as usize;
        if abs_pos < slot_data.len()
            && (slot_data[abs_pos] & (1 << bit_pos)) != 0
        {
            positive_score += 1;
            if tier == 1 {
                tier1_score += 1;
            }
        }
    }

    for &(_, byte_offset, bit_pos, _) in NEGATIVE_VALIDATION_FLAGS {
        let abs_pos = offset + byte_offset as usize;
        if abs_pos < slot_data.len()
            && (slot_data[abs_pos] & (1 << bit_pos)) == 0
        {
            negative_score += 1;
        }
    }

    (tier1_score, positive_score, negative_score)
}

/// Content-based EventFlags detection (legacy fallback).
///
/// Searches for grace flag patterns across a 200K byte range.
/// Susceptible to false positives for characters with few graces.
fn detect_event_flags_content_based(slot_data: &[u8]) -> DetectionResult {
    let search_end = (SEARCH_START + MAX_SEARCH_RANGE).min(slot_data.len().saturating_sub(10000));

    let tier1_count = POSITIVE_VALIDATION_FLAGS.iter()
        .filter(|(_, _, _, _, tier)| *tier == 1)
        .count();

    struct Candidate {
        offset: usize,
        tier1_score: usize,
        positive_score: usize,
        negative_score: usize,
    }

    let mut candidates: Vec<Candidate> = Vec::new();

    for test_offset in SEARCH_START..search_end {
        let mut tier1_score = 0;
        let mut positive_score = 0;

        for &(_, byte_offset, bit_pos, _, tier) in POSITIVE_VALIDATION_FLAGS {
            let abs_pos = test_offset + byte_offset as usize;
            if abs_pos < slot_data.len() {
                let byte = slot_data[abs_pos];
                if (byte & (1 << bit_pos)) != 0 {
                    positive_score += 1;
                    if tier == 1 {
                        tier1_score += 1;
                    }
                }
            }
        }

        if tier1_score >= MIN_TIER1_SCORE {
            let mut negative_score = 0;
            for &(_, byte_offset, bit_pos, _) in NEGATIVE_VALIDATION_FLAGS {
                let abs_pos = test_offset + byte_offset as usize;
                if abs_pos < slot_data.len() {
                    let byte = slot_data[abs_pos];
                    if (byte & (1 << bit_pos)) == 0 {
                        negative_score += 1;
                    }
                }
            }

            candidates.push(Candidate {
                offset: test_offset,
                tier1_score,
                positive_score,
                negative_score,
            });
        }
    }

    if !candidates.is_empty() {
        candidates.sort_by(|a, b| {
            b.tier1_score.cmp(&a.tier1_score)
                .then_with(|| b.negative_score.cmp(&a.negative_score))
                .then_with(|| b.positive_score.cmp(&a.positive_score))
                .then_with(|| b.offset.cmp(&a.offset))
        });

        let best = &candidates[0];
        return DetectionResult {
            offset: best.offset,
            positive_score: best.positive_score,
            negative_score: best.negative_score,
            confident: best.tier1_score >= tier1_count
                && best.negative_score >= NEGATIVE_VALIDATION_FLAGS.len() / 2,
        };
    }

    // Ultimate fallback: no detection possible
    DetectionResult {
        offset: SEARCH_START,
        positive_score: 0,
        negative_score: 0,
        confident: false,
    }
}

// =============================================================================
// PICKUP FLAG CALCULATIONS
// =============================================================================

// REMOVED 2026-07-20 (ADR-0008): `get_dungeon_pickup_section_bases()` — 88 per-section
// bases. See the removal note at `calculate_dungeon_pickup_offset` below.

// =============================================================================
// BLOCK, MIDRANGE, AND GENERAL DUNGEON BASES
// =============================================================================

// REMOVED 2026-07-20 (ADR-0008): `get_sub_block_bases()`, `get_main_block_bases()` and
// `get_midrange_bases()` — the block and midrange base tables for flags 60000-999999.
// They became unreachable when the static-offset exports were removed above, and the
// compiler said so. Each entry was a byte offset measured against one save's layout, so
// none of them survive a save whose flag list has grown. World-state flags in these
// ranges now resolve through `ResolvedFlags::world_state`, which locates the family in the
// flag region it is handed.

// REMOVED 2026-07-20 (ADR-0008): `get_dungeon_general_bases()` — the "+3375 per area"
// stride table. Its own audit had already disproven (18,0) and (19,0) against every save
// on this machine and marked most of the m10-m22 entries UNVERIFIED and all-zero, yet the
// exported readers kept serving offsets from it. Legacy-dungeon general flags now resolve
// through `ResolvedFlags::dungeon` / `legacy_alloc_slot`, which take the flag region and
// return Unknown when they cannot place a family. The dead entries are not preserved here:
// they were fabricated by a stride assumption, so there is nothing to recover.

/// Result of flag offset calculation
#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagOffset {
    pub byte_offset: u32,
    pub bit_position: u8,
    pub valid: bool,
}

impl FlagOffset {
    fn invalid() -> Self {
        FlagOffset { byte_offset: 0, bit_position: 0, valid: false }
    }

    fn new(byte_offset: u32, bit_position: u8) -> Self {
        FlagOffset { byte_offset, bit_position, valid: true }
    }
}

// REMOVED 2026-07-20 (ADR-0008): `calculate_dungeon_pickup_offset{,_impl}` and the
// `get_dungeon_pickup_section_bases()` table they read. Same defect as the general
// dungeon table: 88 per-section byte offsets, each measured against one save's layout,
// handed out for any flag id. Dungeon pickups now resolve through `ResolvedFlags::dungeon_pickup`
// (family FAMILY_LEGACY_DUNGEON_PICKUP), whose geometry comes from the game's own
// alloclists via `legacy_alloc_slot` rather than from calibration against a save.

// REMOVED 2026-07-20 (ADR-0008): `calculate_tile_pickup_offset` — the wrapper that
// supplied the static TILE_BASE_OFFSET. CLAUDE.md tombstones 337375: it is real, but it
// is the DISTANCE BETWEEN TWO FAMILIES, not a base. The geometry it wrapped survives
// below and is what `tile_read` uses, called with base 0 and offset by a resolved family.

/// Calculate offset for tile-based flags with a calibrated base
#[wasm_bindgen]
pub fn calculate_tile_pickup_offset_calibrated(flag_id: u32, tile_base: u32) -> FlagOffset {
    calculate_tile_pickup_offset_with_base(flag_id, tile_base)
}

pub fn calculate_tile_pickup_offset_with_base(flag_id: u32, tile_base: u32) -> FlagOffset {
    // Must be 10-digit tile flag
    if flag_id < 1_000_000_000 {
        return FlagOffset::invalid();
    }

    let tile_index = (flag_id - 1_000_000_000) / 10000;
    let local_id = flag_id % 10000;

    // LocalId > 6999 has no storage
    if local_id > TILE_MAX_LOCAL_ID {
        return FlagOffset::invalid();
    }

    let row = tile_index / 100;
    let col = tile_index % 100;

    let slot = (row as i32 - TILE_ROW_BASE as i32) * TILE_SLOTS_PER_ROW as i32
             + (col as i32 - TILE_COL_BASE as i32);

    if slot < 0 {
        return FlagOffset::invalid();
    }

    let slot_offset = tile_base + (slot as u32) * TILE_BYTES_PER_SLOT;
    let byte_offset = slot_offset + local_id / 8;
    let bit_position = (7 - (flag_id % 8)) as u8;

    if byte_offset < EVENT_FLAGS_SIZE as u32 {
        FlagOffset::new(byte_offset, bit_position)
    } else {
        FlagOffset::invalid()
    }
}

// REMOVED 2026-07-20 (ADR-0008): `calculate_world_pickup_offset_by_row_id{,_impl}` and
// the WORLD_PICKUP_ROW_ID_BASE constant. Its own doc comment already recorded that the
// row_id bitfield model was superseded in 2026-02-16 — world pickups with local_id >= 7000
// read in the TILE region at a converted local_id, not in a separate row_id bitfield. It
// survived only because the deleted `get_flag_offset` router was documented as "handling
// the routing correctly". That router is gone; use `ResolvedFlags::tile_pickup`.

/// Convert getItemFlagId to storable row_id for tile-based world pickups.
///
/// DISCOVERY (2026-01-23): For tile-based world pickups:
/// - ItemLotParam has getItemFlagId = row_id + 7000
/// - The game stores row_id (localId 0-999), NOT getItemFlagId (localId 7000+)
#[wasm_bindgen]
pub fn convert_to_row_id(flag_id: u32) -> i64 {
    // Only applies to 10-digit tile flags (1B range)
    if !(1_000_000_000..2_000_000_000).contains(&flag_id) {
        return -1;
    }

    let local_id = flag_id % 10000;
    if local_id >= 7000 {
        (flag_id - 7000) as i64
    } else {
        -1  // Already a valid flag (localId < 7000)
    }
}

/// Check if a flag is a dungeon pickup flag (8-digit, local_id >= 7000)
#[wasm_bindgen]
pub fn is_dungeon_pickup_flag(flag_id: u32) -> bool {
    if !(10_000_000..44_000_000).contains(&flag_id) {
        return false;
    }
    let local_id = flag_id % 10000;
    local_id >= 7000
}

/// Check if a flag is a tile-based world pickup flag (10-digit)
#[wasm_bindgen]
pub fn is_tile_pickup_flag(flag_id: u32) -> bool {
    flag_id >= 1_000_000_000
}

// REMOVED 2026-07-20 (ADR-0008): `get_dungeon_pickup_sections()` — it served the removed
// table's keys as JSON, letting a caller enumerate and trust bases that no longer exist.

// =============================================================================
// REMOVED: UNIFIED FLAG OFFSET CALCULATION (2026-07-20, ADR-0008)
// =============================================================================
//
// `get_flag_offset`, `get_flag_offset_calibrated`, `is_flag_set`,
// `is_flag_set_calibrated` and their shared router `get_flag_offset_with_tile_base`
// are GONE, along with `calculate_tile_flag_offset_unified` and
// `calculate_dungeon_flag_offset_unified`.
//
// They are not deprecated, re-pointed, or left returning invalid: they are removed.
// Their signature — flag_id in, static byte offset out — encodes a model this project
// has abandoned. Every flag family sits at a base that floats per save (it follows an
// append-only list that grows as the character plays), so there is no correct static
// offset for these functions to return. An honest replacement must take the flag region
// so it can resolve the family for THAT save, which makes it a different function.
//
// Callers wanting a flag's state build a `ResolvedFlags` (which resolves the origin once)
// and call the family method, or use the matching `*_state` export; both answer the
// tri-state `FlagState`/i32 and report Unknown rather than guessing:
//   world state (6-digit and block flags) .. ResolvedFlags::world_state   / world_state_flag_state
//   open-world tiles (10-digit, local<7000) ResolvedFlags::tile_world     / tile_world_flag_state
//   world pickups  (10-digit, local>=7000)  ResolvedFlags::tile_pickup    / tile_pickup_state
//   legacy dungeons (8-digit, local<7000) . ResolvedFlags::dungeon        / dungeon_flag_state
//   dungeon pickups (8-digit, local>=7000)  ResolvedFlags::dungeon_pickup / dungeon_pickup_state
//
// A bare flag id does not identify a family (see CLAUDE.md): the caller must pick the
// reader. That is the point — routing on the value silently reads the wrong bit.

// `is_flag_set` / `is_flag_set_calibrated` removed 2026-07-20 (ADR-0008) — see the
// removal note above. Both collapsed "could not place this flag" into `false`, which is
// indistinguishable from "flag not set"; the region-taking readers return Unknown.

// =============================================================================
// PLAYER POSITION EXTRACTION
// =============================================================================

/// Result of player position extraction from slot data
#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPositionResult {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub x2: f32,
    pub y2: f32,
    pub z2: f32,
    pub facing_angle: f32,
    pub map_id_0: u8,
    pub map_id_1: u8,
    pub map_id_2: u8,
    pub map_id_3: u8,
    pub valid: bool,
    pub offset: usize,
}

impl PlayerPositionResult {
    fn invalid() -> Self {
        PlayerPositionResult {
            x: 0.0, y: 0.0, z: 0.0,
            x2: 0.0, y2: 0.0, z2: 0.0,
            facing_angle: 0.0,
            map_id_0: 0, map_id_1: 0, map_id_2: 0, map_id_3: 0,
            valid: false, offset: 0,
        }
    }
}

/// Extract player position from slot data using signature-based search.
///
/// Algorithm: Search for the map_id from the slot header (bytes 4-7) within the
/// PlayerCoords struct region. Validate via characteristic padding patterns and
/// coordinate ranges. The proven signature from verify_player_coords.py.
#[wasm_bindgen]
pub fn extract_player_position(slot_data: &[u8]) -> PlayerPositionResult {
    extract_player_position_impl(slot_data)
}

/// Internal implementation (also usable from native Rust without WASM)
pub fn extract_player_position_impl(slot_data: &[u8]) -> PlayerPositionResult {
    if slot_data.len() < 12 {
        return PlayerPositionResult::invalid();
    }

    // Read header map_id from slot bytes 4-7
    let header_map_id = &slot_data[4..8];

    let search_end = PLAYER_COORDS_SEARCH_END.min(slot_data.len());

    struct Candidate {
        offset: usize,
        x: f32, y: f32, z: f32,
        x2: f32, y2: f32, z2: f32,
        facing_angle: f32,
        map_id: [u8; 4],
        pad1_zeros: usize,
        pad2_zeros: usize,
        has_position: bool,
    }

    let mut candidates: Vec<Candidate> = Vec::new();

    // Search for map_id match within the expected range
    let actual_start = PLAYER_COORDS_SEARCH_START;
    if actual_start + PLAYER_COORDS_STRUCT_SIZE > search_end {
        return PlayerPositionResult::invalid();
    }

    for i in actual_start..search_end.saturating_sub(PLAYER_COORDS_STRUCT_SIZE) {
        // Check if 4 bytes at this position match header map_id
        if &slot_data[i..i + 4] != header_map_id {
            continue;
        }

        // Check padding2 (16 bytes after coords2): should be mostly zeros
        let pad2_start = i + 4 + MID_SECTION_SIZE + 12; // map_id + mid_section + coords2
        if pad2_start + PAD2_SIZE > slot_data.len() {
            continue;
        }
        let pad2_zeros = slot_data[pad2_start..pad2_start + PAD2_SIZE]
            .iter()
            .filter(|&&b| b == 0)
            .count();
        if pad2_zeros < PAD2_MIN_ZEROS {
            continue;
        }

        // Check mid_section (17 bytes after map_id): should be mostly zeros
        let mid_section = &slot_data[i + 4..i + 4 + MID_SECTION_SIZE];
        let pad1_zeros = mid_section.iter().filter(|&&b| b == 0).count();
        if pad1_zeros < MID_SECTION_MIN_ZEROS {
            continue;
        }

        // Read coords before map_id (12 bytes = 3 x f32)
        if i < 12 {
            continue;
        }
        let coords_offset = i - 12;
        let x = f32::from_le_bytes([
            slot_data[coords_offset], slot_data[coords_offset + 1],
            slot_data[coords_offset + 2], slot_data[coords_offset + 3],
        ]);
        let y = f32::from_le_bytes([
            slot_data[coords_offset + 4], slot_data[coords_offset + 5],
            slot_data[coords_offset + 6], slot_data[coords_offset + 7],
        ]);
        let z = f32::from_le_bytes([
            slot_data[coords_offset + 8], slot_data[coords_offset + 9],
            slot_data[coords_offset + 10], slot_data[coords_offset + 11],
        ]);

        // Skip NaN/Inf/out-of-range
        if x.is_nan() || x.is_infinite() || x.abs() > COORD_RANGE_MAX
            || y.is_nan() || y.is_infinite() || y.abs() > COORD_RANGE_MAX
            || z.is_nan() || z.is_infinite() || z.abs() > COORD_RANGE_MAX
        {
            continue;
        }

        // Reject denormalized floats (garbage data, e.g. z=7.006e-44)
        if is_denormalized(x) || is_denormalized(y) || is_denormalized(z) {
            continue;
        }

        // Read coords2 (12 bytes after mid_section)
        let coords2_offset = i + 4 + MID_SECTION_SIZE;
        let x2 = f32::from_le_bytes([
            slot_data[coords2_offset], slot_data[coords2_offset + 1],
            slot_data[coords2_offset + 2], slot_data[coords2_offset + 3],
        ]);
        let y2 = f32::from_le_bytes([
            slot_data[coords2_offset + 4], slot_data[coords2_offset + 5],
            slot_data[coords2_offset + 6], slot_data[coords2_offset + 7],
        ]);
        let z2 = f32::from_le_bytes([
            slot_data[coords2_offset + 8], slot_data[coords2_offset + 9],
            slot_data[coords2_offset + 10], slot_data[coords2_offset + 11],
        ]);

        if x2.is_nan() || x2.is_infinite() || x2.abs() > COORD_RANGE_MAX
            || y2.is_nan() || y2.is_infinite() || y2.abs() > COORD_RANGE_MAX
            || z2.is_nan() || z2.is_infinite() || z2.abs() > COORD_RANGE_MAX
        {
            continue;
        }

        // Reject denormalized floats in coords2
        if is_denormalized(x2) || is_denormalized(y2) || is_denormalized(z2) {
            continue;
        }

        // Read facing angle from mid_section bytes [4:8] as f32 (little-endian)
        let facing_offset = i + 4 + FACING_ANGLE_OFFSET;
        let facing_angle = f32::from_le_bytes([
            slot_data[facing_offset], slot_data[facing_offset + 1],
            slot_data[facing_offset + 2], slot_data[facing_offset + 3],
        ]);
        let facing_angle = if facing_angle.is_finite() { facing_angle } else { 0.0 };

        // Reject candidates with extreme angles (valid game angles are within ±π)
        if facing_angle.abs() > FACING_ANGLE_MAX {
            continue;
        }

        // Magnitude threshold to distinguish real positions from near-zero
        let magnitude = x.abs() + y.abs() + z.abs();
        let has_position = magnitude > MAGNITUDE_THRESHOLD;

        let map_id = [slot_data[i], slot_data[i + 1], slot_data[i + 2], slot_data[i + 3]];

        candidates.push(Candidate {
            offset: coords_offset,
            x, y, z, x2, y2, z2,
            facing_angle,
            map_id,
            pad1_zeros,
            pad2_zeros,
            has_position,
        });
    }

    if candidates.is_empty() {
        return PlayerPositionResult::invalid();
    }

    // Select best: prefer non-zero coords, then highest padding zeros
    candidates.sort_by(|a, b| {
        b.has_position.cmp(&a.has_position)
            .then_with(|| b.pad2_zeros.cmp(&a.pad2_zeros))
            .then_with(|| b.pad1_zeros.cmp(&a.pad1_zeros))
    });

    let best = &candidates[0];
    if !best.has_position {
        return PlayerPositionResult::invalid();
    }

    PlayerPositionResult {
        x: best.x, y: best.y, z: best.z,
        x2: best.x2, y2: best.y2, z2: best.z2,
        facing_angle: best.facing_angle,
        map_id_0: best.map_id[0], map_id_1: best.map_id[1],
        map_id_2: best.map_id[2], map_id_3: best.map_id[3],
        valid: true,
        offset: best.offset,
    }
}

// =============================================================================
// UTILITY EXPORTS
// =============================================================================

#[wasm_bindgen]
pub fn get_event_flags_size() -> usize {
    EVENT_FLAGS_SIZE
}

#[wasm_bindgen]
pub fn get_search_start() -> usize {
    SEARCH_START
}

#[wasm_bindgen]
pub fn get_tile_max_local_id() -> u32 {
    TILE_MAX_LOCAL_ID
}

#[wasm_bindgen]
pub fn get_player_coords_search_start() -> usize {
    PLAYER_COORDS_SEARCH_START
}

#[wasm_bindgen]
pub fn get_player_coords_search_end() -> usize {
    PLAYER_COORDS_SEARCH_END
}

// =============================================================================
// EQUIPMENT DATA EXTRACTION
// =============================================================================

// Section sizes from ER-save-Reader save_slot.rs (authoritative)
const PLAYER_GAME_DATA_SIZE: usize = 0x1B0;
const PRE_EQUIP_PADDING: usize = 0xD0;
const EQUIP_DATA_STRUCT_SIZE: usize = 0x58; // 22 u32
const CHR_ASM_STRUCT_SIZE: usize = 0x74;    // 29 u32
const CHR_ASM2_STRUCT_SIZE: usize = 0x58;   // 22 u32
const EQUIP_INV_COMMON_SLOTS: usize = 0xA80; // 2688
const EQUIP_INV_KEY_SLOTS: usize = 0x180;    // 384
const EQUIP_INV_ITEM_BYTES: usize = 12;      // 3 × u32
const EQUIP_MAGIC_SPELL_SLOTS: usize = 12;
const EQUIP_MAGIC_DATA_PADDING_BYTES: usize = 0x10;
const EQUIP_ITEM_QUICK_COUNT: usize = 10;
const EQUIP_ITEM_POUCH_COUNT: usize = 6;
const EQUIP_ITEM_TRAILING_BYTES: usize = 8;
const GESTURES_SLOT_COUNT: usize = 6;
const EQUIPPED_ITEMS_U32_COUNT: usize = 39; // 0x9C / 4
const GA_ITEMS_MAX: usize = 0x1400; // 5120

// Computed sizes
const EQUIP_INV_DATA_SIZE: usize =
    4 + EQUIP_INV_COMMON_SLOTS * EQUIP_INV_ITEM_BYTES +
    4 + EQUIP_INV_KEY_SLOTS * EQUIP_INV_ITEM_BYTES +
    4 + 4; // = 0x9010

const EQUIP_MAGIC_DATA_STRUCT_SIZE: usize =
    EQUIP_MAGIC_SPELL_SLOTS * 8 + EQUIP_MAGIC_DATA_PADDING_BYTES + 4; // = 0x74

const EQUIP_ITEM_DATA_STRUCT_SIZE: usize =
    EQUIP_ITEM_QUICK_COUNT * 8 + 4 + EQUIP_ITEM_POUCH_COUNT * 8 + EQUIP_ITEM_TRAILING_BYTES; // = 0x8C

const GESTURES_STRUCT_SIZE: usize = GESTURES_SLOT_COUNT * 4; // = 0x18
const EQUIPPED_ITEMS_STRUCT_SIZE: usize = EQUIPPED_ITEMS_U32_COUNT * 4; // = 0x9C

// =============================================================================
// STRUCTURAL EVENTFLAGS DETECTION
// =============================================================================
//
// Section sizes for the complete chain from GaItems end to EventFlags.
// Verified empirically across 898 slot measurements (scripts/verification/measure_pre_ef_gap.py).
//
// The section chain from save_slot.rs read() order:
//   GaItems → PlayerGameData → Padding → EquipData → ChrAsm → ChrAsm2 →
//   EquipInventoryData → EquipMagicData → EquipItemData → EquipGestureData →
//   EquipProjectileData(VARIABLE) → EquippedItems → EquipPhysicsData →
//   Padding → FaceData → StorageInventoryData → GestureGameData →
//   Regions(VARIABLE) → RideGameData → Misc fields → MenuProfileSaveLoad →
//   TrophyEquipData → GaItemData → TutorialData → PRE_EF_GAP → EventFlags

/// EquipPhysicsData: 2 × u32
const EQUIP_PHYSICS_DATA_SIZE: usize = 8;

/// FaceData section
const FACE_DATA_SIZE: usize = 0x12F; // 303 bytes

/// StorageInventoryData: EquipInventoryData(0x780, 0x80)
/// = 4 + 0x780*12 + 4 + 0x80*12 + 4 + 4 = 24,592
const STORAGE_INV_COMMON_SLOTS: usize = 0x780; // 1920
const STORAGE_INV_KEY_SLOTS: usize = 0x80;     // 128
const STORAGE_INV_DATA_SIZE: usize =
    4 + STORAGE_INV_COMMON_SLOTS * EQUIP_INV_ITEM_BYTES +
    4 + STORAGE_INV_KEY_SLOTS * EQUIP_INV_ITEM_BYTES +
    4 + 4; // = 0x6010

/// GestureGameData: 0x40 × i32
const GESTURE_GAME_DATA_SIZE: usize = 0x40 * 4; // = 0x100

/// RideGameData: 3×f32 + i32 + [u8;0x10] + u32 + u32
const RIDE_GAME_DATA_SIZE: usize = 0x28; // 40 bytes

/// Misc fields between Regions and MenuProfileSaveLoad
/// _0x1(1) + _0x40(64) + _0x4_1(4) + _0x4_2(4) + _0x4_3(4) = 77
const MISC_FIELDS_SIZE: usize = 1 + 0x40 + 4 + 4 + 4; // 77 bytes

/// MenuProfileSaveLoad
const MENU_PROFILE_SAVE_LOAD_SIZE: usize = 0x1008; // 4104 bytes

/// TrophyEquipData
const TROPHY_EQUIP_DATA_SIZE: usize = 0x34; // 52 bytes

/// GaItemData: i32 + i32 + 0x1B58 × GaItem2(4×u32=16B) = 8 + 112000 = 112008
const GA_ITEM2_SIZE: usize = 16; // 4 × u32
const GA_ITEM2_COUNT: usize = 0x1B58; // 7000
const GA_ITEM_DATA_SIZE: usize = 4 + 4 + GA_ITEM2_COUNT * GA_ITEM2_SIZE; // 112008

/// TutorialData
const TUTORIAL_DATA_SIZE: usize = 0x408; // 1032 bytes

/// Fixed gap between TutorialData end and EventFlags start.
/// Empirically verified constant = 29 bytes (0x1D) across ALL save versions.
/// Measured across 898 slot measurements from 2 backup saves + 100+ snapshots.
const PRE_EVENT_FLAGS_GAP: usize = 0x1D; // 29 bytes

/// Sum of all fixed-size sections from GaItems end to EquipProjectileData start.
/// PlayerGameData(0x1B0) + Padding(0xD0) + EquipData(0x58) + ChrAsm(0x74) +
/// ChrAsm2(0x58) + EquipInventoryData(0x9010) + EquipMagicData(0x74) +
/// EquipItemData(0x8C) + EquipGestureData(0x18) = 0x94CC
const FIXED_BEFORE_PROJECTILE: usize =
    PLAYER_GAME_DATA_SIZE + PRE_EQUIP_PADDING +
    EQUIP_DATA_STRUCT_SIZE + CHR_ASM_STRUCT_SIZE + CHR_ASM2_STRUCT_SIZE +
    EQUIP_INV_DATA_SIZE + EQUIP_MAGIC_DATA_STRUCT_SIZE +
    EQUIP_ITEM_DATA_STRUCT_SIZE + GESTURES_STRUCT_SIZE;

/// Sum of fixed-size sections between EquipProjectileData and Regions.
/// EquippedItems(0x9C) + EquipPhysicsData(0x08) + Padding(0x04) +
/// FaceData(0x12F) + StorageInventoryData(0x6010) + GestureGameData(0x100)
const FIXED_BETWEEN_PROJ_AND_REGIONS: usize =
    EQUIPPED_ITEMS_STRUCT_SIZE + EQUIP_PHYSICS_DATA_SIZE + 4 +
    FACE_DATA_SIZE + STORAGE_INV_DATA_SIZE + GESTURE_GAME_DATA_SIZE;

/// Sum of fixed-size sections from Regions end through TutorialData end.
/// RideGameData(0x28) + Misc(77) + MenuProfileSaveLoad(0x1008) +
/// TrophyEquipData(0x34) + GaItemData(112008) + TutorialData(0x408)
const FIXED_AFTER_REGIONS: usize =
    RIDE_GAME_DATA_SIZE + MISC_FIELDS_SIZE + MENU_PROFILE_SAVE_LOAD_SIZE +
    TROPHY_EQUIP_DATA_SIZE + GA_ITEM_DATA_SIZE + TUTORIAL_DATA_SIZE;

/// Compute EventFlags offset structurally by sequential section parsing.
///
/// This mirrors the save_slot.rs read order: after GaItems, parse all
/// intermediate sections (fixed + 2 variable), then add the constant
/// pre-EventFlags gap (0x1D bytes).
///
/// Returns Some(ef_offset) or None if parsing fails (data too short).
fn compute_structural_ef_offset(slot_data: &[u8]) -> Option<usize> {
    let ga_end = find_ga_items_end(slot_data)?;

    // Fixed sections before EquipProjectileData
    let mut pos = ga_end + FIXED_BEFORE_PROJECTILE;

    // EquipProjectileData: count(i32) + count × 8 bytes
    if pos + 4 > slot_data.len() {
        return None;
    }
    let proj_count = read_i32_le(slot_data, pos).max(0) as usize;
    pos += 4 + proj_count * 8;

    // Fixed sections between EquipProjectileData and Regions
    pos += FIXED_BETWEEN_PROJ_AND_REGIONS;

    // Regions: count(u32) + count × 4 bytes
    if pos + 4 > slot_data.len() {
        return None;
    }
    let regions_count = read_u32_le(slot_data, pos) as usize;
    pos += 4 + regions_count * 4;

    // Fixed sections after Regions through TutorialData
    pos += FIXED_AFTER_REGIONS;

    // Constant gap before EventFlags
    pos += PRE_EVENT_FLAGS_GAP;

    // Bounds check: EventFlags must fit in remaining data
    if pos + EVENT_FLAGS_SIZE > slot_data.len() {
        return None;
    }

    Some(pos)
}

/// WASM export: Compute EventFlags offset structurally.
/// Returns -1 if structural computation fails.
#[wasm_bindgen]
pub fn compute_structural_event_flags_offset(slot_data: &[u8]) -> i64 {
    match compute_structural_ef_offset(slot_data) {
        Some(pos) => pos as i64,
        None => -1,
    }
}

// Helper functions for reading little-endian values
fn read_u32_le(data: &[u8], pos: usize) -> u32 {
    u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]])
}

fn read_i32_le(data: &[u8], pos: usize) -> i32 {
    i32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]])
}

/// Find the end of the GaItems section (variable-length).
/// Returns the absolute offset within slot_data, or None if parsing fails.
fn find_ga_items_end(slot_data: &[u8]) -> Option<usize> {
    if slot_data.len() < 12 {
        return None;
    }

    let version = read_u32_le(slot_data, 0);
    let header_padding: usize = if version == 81 { 0x8 } else { 0x18 };
    let mut pos = 4 + 4 + header_padding;

    for _ in 0..GA_ITEMS_MAX {
        if pos + 8 > slot_data.len() {
            return None;
        }
        pos += 4; // gaitem_handle
        let item_id = read_u32_le(slot_data, pos);
        pos += 4; // item_id

        if item_id == 0 || item_id == 0xFFFFFFFF {
            continue;
        }

        match item_id & 0xF0000000 {
            0x00000000 => pos += 13, // Weapon
            0x10000000 => pos += 8,  // Armor
            _ => {}
        }

        if pos > slot_data.len() {
            return None;
        }
    }

    Some(pos)
}

/// WASM export: Find the end of GaItems section.
/// Returns -1 if parsing fails.
#[wasm_bindgen]
pub fn parse_ga_items_end(slot_data: &[u8]) -> i64 {
    match find_ga_items_end(slot_data) {
        Some(pos) => pos as i64,
        None => -1,
    }
}

// JSON serialization types for equipment extraction

#[derive(Serialize)]
struct EquipmentOffsets {
    player_game_data: usize,
    equip_data: usize,
    chr_asm: usize,
    chr_asm2: usize,
    equip_inventory_data: usize,
    equip_magic_data: usize,
    equip_item_data: usize,
    gestures: usize,
    equip_projectile_data: usize,
    equipped_items: usize,
    equip_physics_data: usize,
}

#[derive(Serialize)]
struct ChrAsmJson {
    arm_style: u32,
    left_hand_active_slot: u32,
    right_hand_active_slot: u32,
    left_arrow_active_slot: u32,
    right_arrow_active_slot: u32,
    left_bolt_active_slot: u32,
    right_bolt_active_slot: u32,
    left_hand_armaments: [u32; 3],
    right_hand_armaments: [u32; 3],
    arrows: [u32; 2],
    bolts: [u32; 2],
    head: u32,
    chest: u32,
    arms: u32,
    legs: u32,
    talismans: [u32; 4],
}

#[derive(Serialize)]
struct EquipItemJson {
    item_id: u32,
    equip_index: u32,
}

#[derive(Serialize)]
struct EquipItemDataJson {
    quick_items: Vec<EquipItemJson>,
    active_slot: i32,
    pouch_items: Vec<EquipItemJson>,
}

#[derive(Serialize)]
struct EquipMagicSpellJson {
    spell_id: i32,
}

#[derive(Serialize)]
struct EquipMagicDataJson {
    spells: Vec<EquipMagicSpellJson>,
    active_slot: i32,
}

#[derive(Serialize)]
struct EquippedItemsJson {
    left_hand_armaments: [u32; 3],
    right_hand_armaments: [u32; 3],
    arrows: [u32; 2],
    bolts: [u32; 2],
    head: u32,
    chest: u32,
    arms: u32,
    legs: u32,
    talismans: [u32; 4],
    quickitems: Vec<u32>,
    pouch: Vec<u32>,
}

#[derive(Serialize)]
struct EquipmentExtraction {
    valid: bool,
    ga_items_end: usize,
    offsets: EquipmentOffsets,
    chr_asm: ChrAsmJson,
    equip_item_data: EquipItemDataJson,
    equip_magic_data: EquipMagicDataJson,
    equipped_items: EquippedItemsJson,
}

/// WASM export: Extract all equipment data from slot data.
///
/// Parses GaItems to find their end, then sequentially reads all equipment
/// sections (matching ER-save-Reader save_slot.rs read order). Returns JSON
/// with computed offsets and parsed equipment data.
///
/// This is the SINGLE SOURCE OF TRUTH for equipment section offsets,
/// shared between ER-save-Reader and elden-map.
#[wasm_bindgen]
pub fn extract_equipment_data(slot_data: &[u8]) -> String {
    match extract_equipment_impl(slot_data) {
        Some(data) => serde_json::to_string(&data)
            .unwrap_or_else(|_| r#"{"valid":false}"#.to_string()),
        None => r#"{"valid":false}"#.to_string(),
    }
}

fn extract_equipment_impl(slot_data: &[u8]) -> Option<EquipmentExtraction> {
    let ga_items_end = find_ga_items_end(slot_data)?;

    // Sequential offsets from gaItemsEnd (matching save_slot.rs read order):
    // PlayerGameData(0x1B0) + _0xD0(0xD0) + EquipData(0x58) + ChrAsm(0x74) + ChrAsm2(0x58)
    // = 0x3A4 to EquipInventoryData
    let pgd_off = ga_items_end;
    let equip_data_off = pgd_off + PLAYER_GAME_DATA_SIZE + PRE_EQUIP_PADDING;
    let chr_asm_off = equip_data_off + EQUIP_DATA_STRUCT_SIZE;
    let chr_asm2_off = chr_asm_off + CHR_ASM_STRUCT_SIZE;
    let equip_inv_off = chr_asm2_off + CHR_ASM2_STRUCT_SIZE;
    let equip_magic_off = equip_inv_off + EQUIP_INV_DATA_SIZE;
    let equip_item_off = equip_magic_off + EQUIP_MAGIC_DATA_STRUCT_SIZE;
    let gestures_off = equip_item_off + EQUIP_ITEM_DATA_STRUCT_SIZE;
    let equip_proj_off = gestures_off + GESTURES_STRUCT_SIZE;

    // EquipProjectileData is variable: count(i32) + count * 8 bytes
    if equip_proj_off + 4 > slot_data.len() {
        return None;
    }
    let proj_count = read_i32_le(slot_data, equip_proj_off).max(0) as usize;
    let equip_proj_size = 4 + proj_count * 8;

    let equipped_items_off = equip_proj_off + equip_proj_size;
    let equip_physics_off = equipped_items_off + EQUIPPED_ITEMS_STRUCT_SIZE;

    // Bounds check for final section
    if equip_physics_off + 8 > slot_data.len() {
        return None;
    }

    // Parse sections
    let chr_asm = parse_chr_asm_data(slot_data, chr_asm_off)?;
    let equip_magic = parse_equip_magic_data(slot_data, equip_magic_off)?;
    let equip_item = parse_equip_item_data(slot_data, equip_item_off)?;
    let equipped_items = parse_equipped_items_data(slot_data, equipped_items_off)?;

    Some(EquipmentExtraction {
        valid: true,
        ga_items_end,
        offsets: EquipmentOffsets {
            player_game_data: pgd_off,
            equip_data: equip_data_off,
            chr_asm: chr_asm_off,
            chr_asm2: chr_asm2_off,
            equip_inventory_data: equip_inv_off,
            equip_magic_data: equip_magic_off,
            equip_item_data: equip_item_off,
            gestures: gestures_off,
            equip_projectile_data: equip_proj_off,
            equipped_items: equipped_items_off,
            equip_physics_data: equip_physics_off,
        },
        chr_asm,
        equip_item_data: equip_item,
        equip_magic_data: equip_magic,
        equipped_items,
    })
}

fn parse_chr_asm_data(data: &[u8], off: usize) -> Option<ChrAsmJson> {
    if off + CHR_ASM_STRUCT_SIZE > data.len() {
        return None;
    }

    let mut p = off;
    let arm_style = read_u32_le(data, p); p += 4;
    let left_hand_active_slot = read_u32_le(data, p); p += 4;
    let right_hand_active_slot = read_u32_le(data, p); p += 4;
    let left_arrow_active_slot = read_u32_le(data, p); p += 4;
    let right_arrow_active_slot = read_u32_le(data, p); p += 4;
    let left_bolt_active_slot = read_u32_le(data, p); p += 4;
    let right_bolt_active_slot = read_u32_le(data, p); p += 4;

    // Interleaved: L[0], R[0], L[1], R[1], L[2], R[2]
    let mut left_hand = [0u32; 3];
    let mut right_hand = [0u32; 3];
    for i in 0..3 {
        left_hand[i] = read_u32_le(data, p); p += 4;
        right_hand[i] = read_u32_le(data, p); p += 4;
    }

    let mut arrows = [0u32; 2];
    let mut bolts = [0u32; 2];
    arrows[0] = read_u32_le(data, p); p += 4;
    bolts[0] = read_u32_le(data, p); p += 4;
    arrows[1] = read_u32_le(data, p); p += 4;
    bolts[1] = read_u32_le(data, p); p += 4;

    p += 4; // _0x4
    p += 4; // _0x4_1

    let head = read_u32_le(data, p); p += 4;
    let chest = read_u32_le(data, p); p += 4;
    let arms = read_u32_le(data, p); p += 4;
    let legs = read_u32_le(data, p); p += 4;

    p += 4; // _0x4_2

    let mut talismans = [0u32; 4];
    for t in talismans.iter_mut() {
        *t = read_u32_le(data, p); p += 4;
    }
    // unk field: p += 4 (not needed)

    Some(ChrAsmJson {
        arm_style,
        left_hand_active_slot,
        right_hand_active_slot,
        left_arrow_active_slot,
        right_arrow_active_slot,
        left_bolt_active_slot,
        right_bolt_active_slot,
        left_hand_armaments: left_hand,
        right_hand_armaments: right_hand,
        arrows,
        bolts,
        head, chest, arms, legs,
        talismans,
    })
}

fn parse_equip_magic_data(data: &[u8], off: usize) -> Option<EquipMagicDataJson> {
    if off + EQUIP_MAGIC_DATA_STRUCT_SIZE > data.len() {
        return None;
    }

    let mut p = off;
    let mut spells = Vec::with_capacity(EQUIP_MAGIC_SPELL_SLOTS);
    for _ in 0..EQUIP_MAGIC_SPELL_SLOTS {
        let spell_id = read_i32_le(data, p); p += 4;
        p += 4; // unk
        spells.push(EquipMagicSpellJson { spell_id });
    }
    p += EQUIP_MAGIC_DATA_PADDING_BYTES; // _0x10
    let active_slot = read_i32_le(data, p);

    Some(EquipMagicDataJson { spells, active_slot })
}

fn parse_equip_item_data(data: &[u8], off: usize) -> Option<EquipItemDataJson> {
    if off + EQUIP_ITEM_DATA_STRUCT_SIZE > data.len() {
        return None;
    }

    let mut p = off;
    let mut quick_items = Vec::with_capacity(EQUIP_ITEM_QUICK_COUNT);
    for _ in 0..EQUIP_ITEM_QUICK_COUNT {
        let item_id = read_u32_le(data, p); p += 4;
        let equip_index = read_u32_le(data, p); p += 4;
        quick_items.push(EquipItemJson { item_id, equip_index });
    }

    let active_slot = read_i32_le(data, p); p += 4;

    let mut pouch_items = Vec::with_capacity(EQUIP_ITEM_POUCH_COUNT);
    for _ in 0..EQUIP_ITEM_POUCH_COUNT {
        let item_id = read_u32_le(data, p); p += 4;
        let equip_index = read_u32_le(data, p); p += 4;
        pouch_items.push(EquipItemJson { item_id, equip_index });
    }

    Some(EquipItemDataJson { quick_items, active_slot, pouch_items })
}

fn parse_equipped_items_data(data: &[u8], off: usize) -> Option<EquippedItemsJson> {
    if off + EQUIPPED_ITEMS_STRUCT_SIZE > data.len() {
        return None;
    }

    let mut p = off;

    // Interleaved: L[0], R[0], L[1], R[1], L[2], R[2]
    let mut left_hand = [0u32; 3];
    let mut right_hand = [0u32; 3];
    for i in 0..3 {
        left_hand[i] = read_u32_le(data, p); p += 4;
        right_hand[i] = read_u32_le(data, p); p += 4;
    }

    let mut arrows = [0u32; 2];
    let mut bolts = [0u32; 2];
    arrows[0] = read_u32_le(data, p); p += 4;
    bolts[0] = read_u32_le(data, p); p += 4;
    arrows[1] = read_u32_le(data, p); p += 4;
    bolts[1] = read_u32_le(data, p); p += 4;

    p += 4; // _unk1
    p += 4; // _unk2

    let head = read_u32_le(data, p); p += 4;
    let chest = read_u32_le(data, p); p += 4;
    let arms = read_u32_le(data, p); p += 4;
    let legs = read_u32_le(data, p); p += 4;

    p += 4; // _unk3

    let mut talismans = [0u32; 4];
    for t in talismans.iter_mut() {
        *t = read_u32_le(data, p); p += 4;
    }

    p += 4; // _unk4 (covenant)

    let mut quickitems = Vec::with_capacity(10);
    for _ in 0..10 {
        quickitems.push(read_u32_le(data, p)); p += 4;
    }

    let mut pouch = Vec::with_capacity(6);
    for _ in 0..6 {
        pouch.push(read_u32_le(data, p)); p += 4;
    }
    // _padding17: p += 4 (not needed)

    Some(EquippedItemsJson {
        left_hand_armaments: left_hand,
        right_hand_armaments: right_hand,
        arrows,
        bolts,
        head, chest, arms, legs,
        talismans,
        quickitems,
        pouch,
    })
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(SEARCH_START, 0x12000);
        assert_eq!(EVENT_FLAGS_SIZE, 0x1BF99F);
        assert_eq!(POSITIVE_VALIDATION_FLAGS.len(), 7);
        assert_eq!(NEGATIVE_VALIDATION_FLAGS.len(), 6);
    }

    #[test]
    fn test_tier1_count() {
        let tier1_count = POSITIVE_VALIDATION_FLAGS.iter()
            .filter(|(_, _, _, _, tier)| *tier == 1)
            .count();
        assert_eq!(tier1_count, 4);
    }

    // test_dungeon_pickup_offset_{stormveil,catacombs,unknown_section} removed
    // 2026-07-20 (ADR-0008) with the per-section base table they asserted against.
    // Dungeon-pickup placement is now covered by tests/origin_conformance.rs
    // (`dungeon_reads_split_by_family_and_refuse_foreign_ids`), which checks the
    // family split and the refusal path instead of a literal byte offset.

    // These now pass base 0, which is how `tile_read` calls the same function: the
    // result is a FAMILY-RELATIVE offset, and the caller adds a base resolved from the
    // save. Passing a literal base here would re-assert the tombstoned 337375.

    #[test]
    fn test_tile_offset() {
        // Limgrave tile (42, 36), local_id 10 — slot (42-33)*40 + (36-30) = 366
        let result = calculate_tile_pickup_offset_with_base(1042360010, 0);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 366 * TILE_BYTES_PER_SLOT + 10 / 8);
        assert_eq!(result.bit_position, 7 - (1042360010 % 8) as u8);
    }

    #[test]
    fn test_tile_offset_high_local_id() {
        // local_id >= 7000 should be invalid (no storage)
        let result = calculate_tile_pickup_offset_with_base(1042367000, 0);
        assert!(!result.valid);
    }

    #[test]
    fn test_convert_to_row_id() {
        // getItemFlagId with local_id 7300 -> row_id with local_id 300
        assert_eq!(convert_to_row_id(1042367300), 1042360300);
        // Already valid (local_id < 7000)
        assert_eq!(convert_to_row_id(1042360300), -1);
        // Not a tile flag
        assert_eq!(convert_to_row_id(10007000), -1);
    }

    // test_section_bases_count, test_world_pickup_offset_by_row_id,
    // test_world_pickup_offset_invalid_inputs and test_world_pickup_row_id_base removed
    // 2026-07-20 (ADR-0008). The first counted rows in a deleted table. The others pinned
    // the row_id bitfield formula, and their own comments conceded it "produces
    // valid-looking results but at the WRONG offsets" — a test asserting the arithmetic
    // of a storage model known not to be the game's.

    #[test]
    fn test_player_coords_constants() {
        assert_eq!(PLAYER_COORDS_SEARCH_START, 0x1D0000);
        assert_eq!(PLAYER_COORDS_SEARCH_END, 0x280000);
        assert_eq!(PLAYER_COORDS_STRUCT_SIZE, 61);
        assert_eq!(MID_SECTION_SIZE, 17);
        assert_eq!(FACING_ANGLE_OFFSET, 4);
        assert_eq!(PAD2_SIZE, 16);
    }

    #[test]
    fn test_player_position_empty_data() {
        let result = extract_player_position_impl(&[]);
        assert!(!result.valid);
    }

    #[test]
    fn test_player_position_too_small() {
        let result = extract_player_position_impl(&[0u8; 100]);
        assert!(!result.valid);
    }

    #[test]
    fn test_player_position_synthetic() {
        // Build a synthetic slot data with known coordinates
        // Need: header with map_id at bytes 4-7, then at search_start:
        // coords(12B) + map_id(4B) + mid_section(17B) + coords2(12B) + pad2(16B)
        let map_id: [u8; 4] = [0, 36, 44, 60]; // m60_44_36_00

        let mut slot_data = vec![0u8; PLAYER_COORDS_SEARCH_START + PLAYER_COORDS_STRUCT_SIZE + 100];

        // Set header map_id at bytes 4-7
        slot_data[4] = map_id[0];
        slot_data[5] = map_id[1];
        slot_data[6] = map_id[2];
        slot_data[7] = map_id[3];

        // Position the struct at search_start + 12 (coords before map_id)
        let map_id_pos = PLAYER_COORDS_SEARCH_START + 12;

        // Write coords1 (x=-12.83, y=90.70, z=-54.50) just before map_id
        let x: f32 = -12.83;
        let y: f32 = 90.70;
        let z: f32 = -54.50;
        let coords_pos = map_id_pos - 12;
        slot_data[coords_pos..coords_pos + 4].copy_from_slice(&x.to_le_bytes());
        slot_data[coords_pos + 4..coords_pos + 8].copy_from_slice(&y.to_le_bytes());
        slot_data[coords_pos + 8..coords_pos + 12].copy_from_slice(&z.to_le_bytes());

        // Write map_id
        slot_data[map_id_pos..map_id_pos + 4].copy_from_slice(&map_id);

        // Write mid_section: 4 zeros + facing angle + 8 zeros + 0x01
        let mid_start = map_id_pos + 4;
        // First 4 bytes are already zero
        let facing: f32 = 1.5; // ~86 degrees
        slot_data[mid_start + 4..mid_start + 8].copy_from_slice(&facing.to_le_bytes());
        // Bytes 8-15 already zero
        slot_data[mid_start + 16] = 0x01;

        // Write coords2 (same as coords1 for simplicity)
        let coords2_pos = mid_start + MID_SECTION_SIZE;
        slot_data[coords2_pos..coords2_pos + 4].copy_from_slice(&x.to_le_bytes());
        slot_data[coords2_pos + 4..coords2_pos + 8].copy_from_slice(&y.to_le_bytes());
        slot_data[coords2_pos + 8..coords2_pos + 12].copy_from_slice(&z.to_le_bytes());

        // pad2 is already zeros (16 bytes)

        let result = extract_player_position_impl(&slot_data);
        assert!(result.valid, "Expected valid result");
        assert!((result.x - x).abs() < 0.01, "x mismatch: {} vs {}", result.x, x);
        assert!((result.y - y).abs() < 0.01, "y mismatch: {} vs {}", result.y, y);
        assert!((result.z - z).abs() < 0.01, "z mismatch: {} vs {}", result.z, z);
        assert!((result.facing_angle - facing).abs() < 0.01, "facing mismatch");
        assert_eq!(result.map_id_0, map_id[0]);
        assert_eq!(result.map_id_1, map_id[1]);
        assert_eq!(result.map_id_2, map_id[2]);
        assert_eq!(result.map_id_3, map_id[3]);
    }

    // =========================================================================
    // REMOVED: UNIFIED get_flag_offset TESTS (2026-07-20, ADR-0008)
    // =========================================================================
    //
    // These pinned the static-offset exports that no longer exist. Every one asserted a
    // literal byte offset for a flag id — exactly the promise the exports could not keep,
    // since each family's base floats per save. Re-pointing them at the region-taking
    // readers was not possible: those need a flag region, and these tests had none.
    //
    // Two carried real evidence worth not losing, both already recorded outside the code:
    //   - test_tile_world_pickup_m60_4_43{,_calibrated}: 5 world pickups at tile (44,36),
    //     captures 119-127 (V1). The tile GEOMETRY they verified (slot = (row-33)*40 +
    //     (col-30), 875 bytes/slot) survives in `calculate_tile_pickup_offset_with_base`
    //     and is still covered by `test_tile_offset`; only the static 337375 base is gone.
    //   - test_get_item_flag_id_tile_routing: getItemFlagId (local>=7000) reads at
    //     converted local_id, not the row_id bitfield. That routing now lives in the
    //     split between `ResolvedFlags::tile_pickup` and `ResolvedFlags::tile_world`,
    //     covered by tests/origin_conformance.rs.

    // =========================================================================
    // EQUIPMENT EXTRACTION TESTS
    // =========================================================================

    #[test]
    fn test_equipment_section_size_constants() {
        // Verify computed sizes match expected values
        assert_eq!(EQUIP_INV_DATA_SIZE, 0x9010);
        assert_eq!(EQUIP_MAGIC_DATA_STRUCT_SIZE, 0x74);
        assert_eq!(EQUIP_ITEM_DATA_STRUCT_SIZE, 0x8C);
        assert_eq!(GESTURES_STRUCT_SIZE, 0x18);
        assert_eq!(EQUIPPED_ITEMS_STRUCT_SIZE, 0x9C);
    }

    #[test]
    fn test_equipment_offset_chain() {
        // Verify the offset chain from gaItemsEnd to EquipInventoryData
        let gap = PLAYER_GAME_DATA_SIZE + PRE_EQUIP_PADDING +
                  EQUIP_DATA_STRUCT_SIZE + CHR_ASM_STRUCT_SIZE + CHR_ASM2_STRUCT_SIZE;
        assert_eq!(gap, 0x3A4, "gaItemsEnd to EquipInventoryData should be 0x3A4");

        // EquipInventoryData to EquipMagicData
        assert_eq!(EQUIP_INV_DATA_SIZE, 0x9010);

        // EquipMagicData to EquipItemData
        assert_eq!(EQUIP_MAGIC_DATA_STRUCT_SIZE, 0x74);

        // Total gaItemsEnd to EquipItemData
        let total = gap + EQUIP_INV_DATA_SIZE + EQUIP_MAGIC_DATA_STRUCT_SIZE;
        assert_eq!(total, 0x9428, "gaItemsEnd to EquipItemData should be 0x9428");
    }

    #[test]
    fn test_find_ga_items_end_empty() {
        assert!(find_ga_items_end(&[]).is_none());
        assert!(find_ga_items_end(&[0; 10]).is_none());
    }

    #[test]
    fn test_find_ga_items_end_minimal() {
        // Build minimal slot data: version(4) + map_id(4) + padding(0x18) + 5120 empty items
        let header_size = 4 + 4 + 0x18; // version=0 (not 81), so 0x18 padding
        let ga_items_size = GA_ITEMS_MAX * 8; // all empty = 8 bytes each
        let total = header_size + ga_items_size;
        let slot_data = vec![0u8; total];
        let result = find_ga_items_end(&slot_data);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), header_size + ga_items_size);
    }

    #[test]
    fn test_extract_equipment_too_small() {
        let result = extract_equipment_data(&[0; 100]);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["valid"], false);
    }

    // =========================================================================
    // STRUCTURAL DETECTION TESTS
    // =========================================================================

    #[test]
    fn test_structural_section_size_constants() {
        // Verify key computed sizes
        assert_eq!(EQUIP_PHYSICS_DATA_SIZE, 8);
        assert_eq!(FACE_DATA_SIZE, 0x12F);
        assert_eq!(GESTURE_GAME_DATA_SIZE, 0x100);
        assert_eq!(RIDE_GAME_DATA_SIZE, 0x28);
        assert_eq!(MISC_FIELDS_SIZE, 77);
        assert_eq!(MENU_PROFILE_SAVE_LOAD_SIZE, 0x1008);
        assert_eq!(TROPHY_EQUIP_DATA_SIZE, 0x34);
        assert_eq!(TUTORIAL_DATA_SIZE, 0x408);
        assert_eq!(PRE_EVENT_FLAGS_GAP, 0x1D);

        // GaItemData: 8 + 7000 * 16 = 112008
        assert_eq!(GA_ITEM2_SIZE, 16);
        assert_eq!(GA_ITEM2_COUNT, 7000);
        assert_eq!(GA_ITEM_DATA_SIZE, 112008);

        // StorageInventoryData: 4 + 1920*12 + 4 + 128*12 + 4 + 4 = 24592
        assert_eq!(STORAGE_INV_DATA_SIZE, 24592);
    }

    #[test]
    fn test_structural_fixed_section_sums() {
        // FIXED_BEFORE_PROJECTILE = 0x94CC (matches equipment extraction offset)
        assert_eq!(FIXED_BEFORE_PROJECTILE, 0x94CC);

        // FIXED_BETWEEN_PROJ_AND_REGIONS
        let expected_between = EQUIPPED_ITEMS_STRUCT_SIZE + EQUIP_PHYSICS_DATA_SIZE + 4
            + FACE_DATA_SIZE + STORAGE_INV_DATA_SIZE + GESTURE_GAME_DATA_SIZE;
        assert_eq!(FIXED_BETWEEN_PROJ_AND_REGIONS, expected_between);

        // FIXED_AFTER_REGIONS
        let expected_after = RIDE_GAME_DATA_SIZE + MISC_FIELDS_SIZE
            + MENU_PROFILE_SAVE_LOAD_SIZE + TROPHY_EQUIP_DATA_SIZE
            + GA_ITEM_DATA_SIZE + TUTORIAL_DATA_SIZE;
        assert_eq!(FIXED_AFTER_REGIONS, expected_after);
    }

    #[test]
    fn test_structural_ef_offset_too_small() {
        // Data too small for structural parsing
        assert!(compute_structural_ef_offset(&[]).is_none());
        assert!(compute_structural_ef_offset(&[0; 100]).is_none());
    }

    #[test]
    fn test_structural_ef_offset_synthetic() {
        // Build a synthetic slot with known structure:
        // - Version 0 (not 81) → 0x18 header padding
        // - All GaItems empty (5120 × 8 bytes = 40960)
        // - EquipProjectileData count = 0
        // - Regions count = 0
        // This gives a deterministic offset.

        let header_size = 4 + 4 + 0x18; // 0x20
        let ga_items_size = GA_ITEMS_MAX * 8; // 5120 * 8 = 40960
        let ga_end = header_size + ga_items_size; // 0x20 + 0xA000 = 0xA020

        // Expected: ga_end + FIXED_BEFORE_PROJ + proj_header(4) +
        //           FIXED_BETWEEN + regions_header(4) +
        //           FIXED_AFTER + PRE_EF_GAP
        let expected_ef = ga_end + FIXED_BEFORE_PROJECTILE
            + 4 // proj count = 0, just 4 bytes header
            + FIXED_BETWEEN_PROJ_AND_REGIONS
            + 4 // regions count = 0, just 4 bytes header
            + FIXED_AFTER_REGIONS
            + PRE_EVENT_FLAGS_GAP;

        // Create slot data large enough
        let total_needed = expected_ef + EVENT_FLAGS_SIZE;
        let slot_data = vec![0u8; total_needed];

        let result = compute_structural_ef_offset(&slot_data);
        assert!(result.is_some(), "Structural computation should succeed");
        assert_eq!(result.unwrap(), expected_ef);
    }

    #[test]
    fn test_structural_ef_offset_with_projectiles_and_regions() {
        // Same as above but with proj_count=5 and regions_count=10
        let header_size = 4 + 4 + 0x18;
        let ga_items_size = GA_ITEMS_MAX * 8;
        let ga_end = header_size + ga_items_size;

        let proj_count: i32 = 5;
        let regions_count: u32 = 10;

        let expected_ef = ga_end + FIXED_BEFORE_PROJECTILE
            + 4 + (proj_count as usize) * 8
            + FIXED_BETWEEN_PROJ_AND_REGIONS
            + 4 + (regions_count as usize) * 4
            + FIXED_AFTER_REGIONS
            + PRE_EVENT_FLAGS_GAP;

        let total_needed = expected_ef + EVENT_FLAGS_SIZE;
        let mut slot_data = vec![0u8; total_needed];

        // Write proj_count at the EquipProjectileData position
        let proj_pos = ga_end + FIXED_BEFORE_PROJECTILE;
        slot_data[proj_pos..proj_pos + 4].copy_from_slice(&proj_count.to_le_bytes());

        // Write regions_count at the Regions position
        let regions_pos = proj_pos + 4 + (proj_count as usize) * 8
            + FIXED_BETWEEN_PROJ_AND_REGIONS;
        slot_data[regions_pos..regions_pos + 4].copy_from_slice(&regions_count.to_le_bytes());

        let result = compute_structural_ef_offset(&slot_data);
        assert!(result.is_some(), "Structural computation should succeed");
        assert_eq!(result.unwrap(), expected_ef);
    }

    #[test]
    fn test_detect_ef_no_anchors_not_confident() {
        // 2026-07-05: an all-zero slot has no grace anchors, so detection must
        // NOT claim confidence. (The old structural-primary path returned
        // confident=true here, which is how the ~146k overshoot went unnoticed.)
        let slot_data = vec![0u8; 0x280000];
        let result = detect_event_flags_offset_impl(&slot_data);
        assert!(
            !result.confident,
            "no grace anchors present, detection must not be confident"
        );
    }

    #[test]
    fn test_detect_ef_windowed_scan_finds_planted_anchors() {
        // Plant the tier-1 validation flags at a known offset inside the
        // gaEnd window and verify the windowed scan returns exactly it.
        let header_size = 4 + 4 + 0x18;
        let ga_items_size = GA_ITEMS_MAX * 8;
        let ga_end = header_size + ga_items_size; // all-zero GaItems parse

        let planted = ga_end + 36_000; // inside EF_WINDOW_AFTER_GA_END
        let mut slot_data = vec![0u8; 0x280000];
        slot_data[planted + 2725] = 0x80 | 0x40; // 71800 bit7, 71801 bit6
        slot_data[planted + 3262] = 0x08 | 0x04; // 76100 bit3, 76101 bit2

        let result = detect_event_flags_offset_impl(&slot_data);
        assert_eq!(result.offset, planted);
        assert!(result.confident, "all tier-1 anchors present in-window");

        // The struct-walk position (~146k past the real base) must lose:
        // planting the same pattern there too must not displace the
        // in-window candidate.
        let lookalike = planted + 146_104;
        slot_data[lookalike + 2725] = 0x80 | 0x40;
        slot_data[lookalike + 3262] = 0x08 | 0x04;
        let result = detect_event_flags_offset_impl(&slot_data);
        assert_eq!(
            result.offset, planted,
            "out-of-window lookalike must be unreachable"
        );
    }

    #[test]
    fn test_validate_at_offset() {
        let mut data = vec![0u8; 10000];
        // Set Cave of Knowledge flag at offset 100: byte 100+2725=2825, bit 7
        data[2825] = 0x80; // bit 7
        // Set Stranded Graveyard flag: byte 100+2725=2825, bit 6
        data[2825] |= 0x40; // bit 6

        let (tier1, pos, neg) = validate_at_offset(&data, 100);
        assert_eq!(tier1, 2); // Cave of Knowledge + Stranded Graveyard
        assert_eq!(pos, 2);
        // Negative flags should all be "unset" since data is zeros
        assert_eq!(neg, NEGATIVE_VALIDATION_FLAGS.len());
    }

    // =========================================================================
    // REMOVED: DUNGEON GRACE RESOLUTION TESTS (2026-07-20, ADR-0008)
    // =========================================================================
    //
    // The sub-block/main-block split they exercised is gone with the block base tables.
    // They asserted fixed byte offsets for grace flags 71000-76100, which held only for
    // the save the bases were measured against. Note the tutorial graces here (71800) —
    // CLAUDE.md already records that these are NOT universal anchors; they read clear on
    // minimal characters, so a test asserting their position proved nothing about a real
    // save. Grace flags now go through `ResolvedFlags::world_state`.


}

// ===========================================================================
// Event-flag origin resolution
// ===========================================================================
//
// Flag families do not sit at fixed offsets: an append-only u32 list ahead of
// them grows as the character plays, pushing every family along by 4 bytes per
// appended record. Measuring from that list's END removes the drift entirely,
// so a single save with no history can position every family:
//
//     family_base = ga_end + flag_list_end + FAMILY_CONSTANT
//
// Evidence: docs/BACKLOG.md step 4b, knowledge/claims/{list-hunt,
// origin-validation}.json. The constants were measured across 47 Confessor
// captures and validated out-of-sample on V1/V2/V3 (exact expected bit
// patterns, including CLEAR bits) and on the Wretch slot.
//
// This is a bounded structural scan, NOT a full parse of the enclosing section:
// the list carries no length prefix (the bytes before it are zeros), so its end
// is found by scanning. Every assumption below is therefore checked, and the
// resolver returns None rather than a plausible-looking wrong answer — a wrong
// base reads garbage flags silently, which is the failure mode this whole
// investigation existed to eliminate.

/// Where to start looking for the list, as an offset from ga_end. Must land in
/// the zero gap BEFORE the list; observed list starts are 63,187-64,767.
pub const ORIGIN_PROBE_START: usize = 50_000;

/// A zero run this long terminates the list.
pub const ORIGIN_ZERO_RUN: usize = 64;

/// Zeros we must actually observe at the probe point before trusting that we
/// started in the gap rather than in the middle of the list itself.
pub const ORIGIN_MIN_LEAD_ZEROS: usize = 256;

/// Plausible range for (list_end - ga_end). Observed: 63,629-65,949 across
/// five characters. A result outside this is a parse failure, not a save with
/// unusual content.
pub const ORIGIN_MIN_GAP: usize = 55_000;
pub const ORIGIN_MAX_GAP: usize = 80_000;

/// Distance from the list's end to each family's base.
pub const FAMILY_WORLD_STATE_B: i64 = 117_192;
pub const FAMILY_TILE_PICKUP_ROW_ID: i64 = 454_567;
pub const FAMILY_LEGACY_DUNGEON_PICKUP: i64 = 1_500_442;

/// End of the append-only u32 list, as an offset from ga_end.
/// None when the structure cannot be identified with confidence.
pub fn find_flag_list_end_from(slot_data: &[u8], ga_end: usize) -> Option<usize> {
    let probe = ga_end.checked_add(ORIGIN_PROBE_START)?;
    if probe >= slot_data.len() {
        return None;
    }

    // The probe must land in the zero gap. If it lands on data we may be inside
    // the list already, and its "start" would be wherever we happened to enter.
    let lead = slot_data[probe..]
        .iter()
        .take(ORIGIN_MIN_LEAD_ZEROS)
        .take_while(|&&b| b == 0)
        .count();
    if lead < ORIGIN_MIN_LEAD_ZEROS {
        return None;
    }

    // Skip the gap to where records resume.
    let mut i = probe;
    while i < slot_data.len() && slot_data[i] == 0 {
        i += 1;
    }
    if i >= slot_data.len() {
        return None;
    }

    // Then the first run of zeros long enough to end the list.
    let mut run = 0usize;
    let mut end = None;
    while i < slot_data.len() {
        if slot_data[i] == 0 {
            run += 1;
            if run >= ORIGIN_ZERO_RUN {
                end = Some(i + 1 - run);
                break;
            }
        } else {
            run = 0;
        }
        i += 1;
    }
    let end = end?;

    let gap = end.checked_sub(ga_end)?;
    if !(ORIGIN_MIN_GAP..=ORIGIN_MAX_GAP).contains(&gap) {
        return None;
    }
    Some(gap)
}

/// As above, parsing ga_end from the slot.
pub fn find_flag_list_end(slot_data: &[u8]) -> Option<usize> {
    let ga_end = find_ga_items_end(slot_data)?;
    find_flag_list_end_from(slot_data, ga_end)
}

/// Absolute base of a flag family within the slot, from the slot alone.
/// None when the origin cannot be resolved.
pub fn resolve_family_base(slot_data: &[u8], family_constant: i64) -> Option<i64> {
    let ga_end = find_ga_items_end(slot_data)?;
    let list_end = find_flag_list_end_from(slot_data, ga_end)?;
    let base = ga_end as i64 + list_end as i64 + family_constant;
    if base < 0 || base as usize >= slot_data.len() {
        return None;
    }
    Some(base)
}

/// WASM export: end of the append-only list (offset from ga_end), -1 on failure.
#[wasm_bindgen]
pub fn flag_list_end(slot_data: &[u8]) -> i64 {
    find_flag_list_end(slot_data).map(|v| v as i64).unwrap_or(-1)
}

/// WASM export: absolute family base, -1 on failure.
#[wasm_bindgen]
pub fn family_base(slot_data: &[u8], family_constant: i64) -> i64 {
    resolve_family_base(slot_data, family_constant).unwrap_or(-1)
}

/// Public wrapper over the GaItems walk, for conformance tests that need to
/// pin ga_end and the list end together.
pub fn find_ga_items_end_pub(slot_data: &[u8]) -> Option<usize> {
    find_ga_items_end(slot_data)
}

// ---------------------------------------------------------------------------
// EF-relative origin resolution (for callers holding only the flag region)
// ---------------------------------------------------------------------------
//
// The application parses saves into structs and keeps `event_flags.flags` — the
// flag region — not raw slot bytes. The append-only list lives INSIDE that
// region (~29.3k in), so the same scan works anchored on the region start, and
// the app needs no access to the slot.
//
// This is self-correcting with respect to where the region actually begins:
// every value is relative to the slice, so if the caller's region start is off
// by N, the list end is found N earlier and the resolved base shifts back by the
// same N — indexing into that same slice still lands on the right byte.
//
// Verified 62/62 against the pipeline's own measurements across probe points
// EF+8,000 through EF+24,000 (docs/BACKLOG.md step 4b).

/// Where to start scanning within the flag region. Must sit in the zero gap
/// before the list; observed list ends are ~29,322-29,426 into the region.
pub const EF_ORIGIN_PROBE_START: usize = 16_000;

/// Plausible range for the list end measured from the flag region start.
pub const EF_ORIGIN_MIN: usize = 20_000;
pub const EF_ORIGIN_MAX: usize = 45_000;

/// End of the append-only u32 list, relative to the flag region start.
pub fn find_flag_list_end_in_ef(event_flags: &[u8]) -> Option<usize> {
    let probe = EF_ORIGIN_PROBE_START;
    if probe >= event_flags.len() {
        return None;
    }
    let lead = event_flags[probe..]
        .iter()
        .take(ORIGIN_MIN_LEAD_ZEROS)
        .take_while(|&&b| b == 0)
        .count();
    if lead < ORIGIN_MIN_LEAD_ZEROS {
        return None;
    }
    let mut i = probe;
    while i < event_flags.len() && event_flags[i] == 0 {
        i += 1;
    }
    if i >= event_flags.len() {
        return None;
    }
    let mut run = 0usize;
    while i < event_flags.len() {
        if event_flags[i] == 0 {
            run += 1;
            if run >= ORIGIN_ZERO_RUN {
                let end = i + 1 - run;
                return (EF_ORIGIN_MIN..=EF_ORIGIN_MAX).contains(&end).then_some(end);
            }
        } else {
            run = 0;
        }
        i += 1;
    }
    None
}

/// Base of a flag family relative to the flag region start.
pub fn resolve_family_base_in_ef(event_flags: &[u8], family_constant: i64) -> Option<usize> {
    let list_end = find_flag_list_end_in_ef(event_flags)? as i64;
    let base = list_end + family_constant;
    if base < 0 || base as usize >= event_flags.len() {
        return None;
    }
    Some(base as usize)
}

/// WASM export: -1 unresolved, 0 clear, 1 set.
#[wasm_bindgen]
pub fn world_state_flag_state(event_flags: &[u8], flag_id: u32) -> i32 {
    ResolvedFlags::from_event_flags(event_flags)
        .map_or(FlagState::Unknown, |r| r.world_state(flag_id))
        .as_i32()
}

// ---------------------------------------------------------------------------
// Tile (open-world) flag reads
// ---------------------------------------------------------------------------
//
// Open-world tiles carry TWO families in separate regions, 500 bytes apart:
//
//   tile-open-world     localId <  7000  — boss kills, world state
//   tile-pickup-row-id  localId >= 7000  — item pickups, addressed by
//                                          ItemLotParam row_id = flag - 7000
//
// Same tile layout (slot * 875), different bases. Sending a pickup flag to the
// open-world base reads a plausible-looking wrong bit 500 bytes away, so the
// split is enforced here rather than left to callers.

/// Distance from the list end to the open-world tile family's base.
/// Measured on two attributed boss-kill pairs (Crucible Knight 1042370800,
/// Bols 1033450800), both giving exactly this value, and corroborated by the
/// claims store's independently measured bases sitting 500 bytes apart
/// (~483,469 vs ~483,969 grace-relative). Thinner evidence than the other
/// constants: two files rather than dozens.
pub const FAMILY_TILE_OPEN_WORLD: i64 = 454_067;

/// WASM export: open-world tile flag. -1 unresolved, 0 clear, 1 set.
#[wasm_bindgen]
pub fn tile_world_flag_state(event_flags: &[u8], flag_id: u32) -> i32 {
    ResolvedFlags::from_event_flags(event_flags)
        .map_or(FlagState::Unknown, |r| r.tile_world(flag_id))
        .as_i32()
}

/// WASM export: world pickup by row_id or getItemFlagId. -1 unresolved.
#[wasm_bindgen]
pub fn tile_pickup_state(event_flags: &[u8], id: u32) -> i32 {
    ResolvedFlags::from_event_flags(event_flags)
        .map_or(FlagState::Unknown, |r| r.tile_pickup(id))
        .as_i32()
}

// ---------------------------------------------------------------------------
// Legacy-dungeon flag reads
// ---------------------------------------------------------------------------
//
// Legacy maps (castles, catacombs, caves, tunnels — anything not an open-world
// tile) address flags by ALLOCATION SLOT, not by map id:
//
//     byte = alloc_slot(map) * 1125 + localId / 8
//
// and, as with tiles, the flags split into two families in separate regions:
//
//   legacy-dungeon         localId <  7000  — boss kills, world state, NPC state
//   legacy-dungeon-pickup  localId >= 7000  — item pickups
//
// The slots come from the GAME's own eventflagalloclists, not from the
// "+3375 per area" stride assumption that produced `get_dungeon_general_bases()`
// above — that table has entries disproven by every save on this machine
// (see its audit comment). Nothing here derives from it.

/// Bytes allocated to one legacy map's flags.
const LEGACY_SLOT_STRIDE: u64 = 1125;

/// Distance from the list end to the legacy-dungeon (event) family's base.
///
/// Measured by `knowledge family-constants` from the two attributed boss-kill
/// pairs that pinned this family: Erdtree Burial Watchdog (30020800, b24-b25)
/// and the m30_03 catacombs boss (30030800, b32-b33). Both give exactly this
/// value, and they sit at different list lengths — the second capture's list has
/// grown — so the agreement survives a drift step rather than restating one
/// measurement twice.
///
/// The derivation is thin — two files, both catacombs (alloc slots 82 and 83) —
/// but it holds away from those slots and on another character. The Wretch
/// (backup slot 1, never used to derive anything here) reads exactly ONE legacy
/// boss defeated across all 102: 18000850, Soldier of Godrick, the tutorial
/// enemy, at alloc slot 35. That is what the evidence catalog says about that
/// character, and slot 35 is 47 allocations away from where the constant was
/// measured, so it exercises the 1125 stride rather than just the base.
///
/// It also retires an old negative result: m18's flags were called DISPROVEN
/// because "all five slots read zero" at base 43,487. They read zero because
/// 43,487 was the wrong convention, not because m18 is unallocated.
pub const FAMILY_LEGACY_DUNGEON: i64 = 1_500_567;

/// Byte offset of a legacy-map flag from its family's base.
pub fn legacy_dungeon_rel_byte(flag_id: u32) -> Option<u64> {
    let slot = legacy_alloc_slot(flag_id / 10_000)?;
    Some(slot as u64 * LEGACY_SLOT_STRIDE + (flag_id % 10_000) as u64 / 8)
}

/// Allocation slot for a legacy map, keyed by the flag prefix AABB
/// (m30_02_00_00 -> 3002). `None` for anything with no allocation — including
/// every 10-digit open-world tile id, whose prefix is six digits and belongs to
/// the tile families instead.
pub fn legacy_alloc_slot(map_prefix: u32) -> Option<u16> {
    let key = u16::try_from(map_prefix).ok()?;
    LEGACY_ALLOC_SLOTS
        .binary_search_by_key(&key, |&(m, _)| m)
        .ok()
        .map(|i| LEGACY_ALLOC_SLOTS[i].1)
}

/// Maps the game allocates TWICE. Which allocation the flag bits actually live
/// in is not established by anything in the evidence, and picking one would read
/// a plausible wrong bit ~92KB away, so both resolve to `None` (Unknown).
/// Kept as data rather than a comment so the conformance test can assert it.
pub const LEGACY_ALLOC_AMBIGUOUS: [(u16, [u16; 2]); 2] = [(3412, [62, 144]), (4000, [70, 170])];

/// map prefix AABB -> allocation slot, from the game's own eventflagalloclists
/// (regulation/exe ProductVersion 2.6.2 = 1.16.x, evidence corpus
/// `game-raw-1162`, decompressed to knowledge/game/eventflag-alloclists.json).
/// Sorted by prefix for binary search; the two ambiguous maps above are absent
/// by design. `tests/origin_conformance.rs` asserts this table still equals its
/// source file.
#[rustfmt::skip]
const LEGACY_ALLOC_SLOTS: [(u16, u16); 99] = [
    (1000, 0), (1001, 1), (1100, 4), (1105, 5), (1110, 6), (1200, 10), (1201, 11), (1202, 12),
    (1203, 13), (1204, 14), (1205, 15), (1206, 16), (1207, 17), (1208, 18), (1209, 19),
    (1300, 20), (1400, 23), (1500, 26), (1600, 29), (1700, 32), (1800, 35), (1900, 38),
    (2000, 150), (2001, 151), (2100, 154), (2101, 155), (2102, 156), (2200, 158), (2500, 160),
    (2800, 166), (3000, 80), (3001, 81), (3002, 82), (3003, 83), (3004, 84), (3005, 85),
    (3006, 86), (3007, 87), (3008, 88), (3009, 89), (3010, 90), (3011, 91), (3012, 92),
    (3013, 93), (3014, 94), (3015, 95), (3016, 96), (3017, 97), (3018, 98), (3019, 99),
    (3020, 100), (3100, 110), (3101, 111), (3102, 112), (3103, 113), (3104, 114), (3105, 115),
    (3106, 116), (3107, 117), (3108, 118), (3109, 119), (3110, 120), (3111, 121), (3112, 122),
    (3113, 123), (3114, 124), (3115, 125), (3117, 126), (3118, 127), (3119, 128), (3120, 129),
    (3121, 130), (3122, 131), (3200, 140), (3201, 141), (3202, 142), (3204, 143), (3205, 145),
    (3207, 146), (3208, 147), (3211, 148), (3410, 60), (3411, 61), (3413, 63), (3414, 64),
    (3415, 65), (3500, 41), (3920, 44), (4001, 171), (4002, 172), (4100, 175), (4101, 176),
    (4102, 177), (4200, 180), (4201, 181), (4202, 182), (4203, 183), (4300, 185), (4301, 186),
];

/// WASM export: legacy-map event flag. -1 unresolved, 0 clear, 1 set.
#[wasm_bindgen]
pub fn dungeon_flag_state(event_flags: &[u8], flag_id: u32) -> i32 {
    ResolvedFlags::from_event_flags(event_flags)
        .map_or(FlagState::Unknown, |r| r.dungeon(flag_id))
        .as_i32()
}

/// WASM export: legacy-map pickup flag. -1 unresolved, 0 clear, 1 set.
#[wasm_bindgen]
pub fn dungeon_pickup_state(event_flags: &[u8], flag_id: u32) -> i32 {
    ResolvedFlags::from_event_flags(event_flags)
        .map_or(FlagState::Unknown, |r| r.dungeon_pickup(flag_id))
        .as_i32()
}

// ---------------------------------------------------------------------------
// Per-save flag position (for callers that need a byte offset, not just a bit)
// ---------------------------------------------------------------------------
//
// ADR-0008 removed every export that turned a bare flag id into a byte offset:
// each read a base baked into this crate, and every family floats per save. This
// is the honest replacement. It takes the flag REGION, resolves the chosen
// family's base for THAT save from the append-only list, and returns Unknown
// (`valid = false`) when it cannot — the same contract as the tri-state readers,
// but yielding the position rather than the bit. Its consumer is elden-map's
// character-explorer hex view, which overlays flag names on save bytes and so
// needs the address, not only the state.
//
// The caller still chooses the family (FAMILY_CODE_*): a bare 10-digit id is
// ambiguous between the two tile families, which live in regions 500 bytes
// apart, so routing on the value would read a plausible wrong byte. Geometry and
// family selection mirror the tri-state readers above exactly.

/// Family selector for `flag_offset_in_ef`. Stable ABI — do not renumber.
pub const FAMILY_CODE_WORLD_STATE: u32 = 0;
pub const FAMILY_CODE_TILE_WORLD: u32 = 1;
pub const FAMILY_CODE_TILE_PICKUP: u32 = 2;
pub const FAMILY_CODE_DUNGEON: u32 = 3;
pub const FAMILY_CODE_DUNGEON_PICKUP: u32 = 4;

/// WASM export: byte+bit position of a flag within the flag region, resolved for
/// THIS save. `family` is a FAMILY_CODE_*. `valid = false` means the id is out of
/// the chosen family, or the family's origin cannot be resolved in these bytes.
/// Takes `event_flags` and invents no base (ADR-0008).
#[wasm_bindgen]
pub fn flag_offset_in_ef(event_flags: &[u8], flag_id: u32, family: u32) -> FlagOffset {
    // (rel byte from the family base, bit position, family constant) — guards and
    // geometry identical to the ResolvedFlags family methods (world_state,
    // tile_world, tile_pickup, dungeon, dungeon_pickup).
    let (rel, bit, fam_const): (u64, u8, i64) = match family {
        FAMILY_CODE_WORLD_STATE => {
            if !(50_000..80_000).contains(&flag_id) {
                return FlagOffset::invalid();
            }
            (((flag_id - 50_000) / 8) as u64, 7 - (flag_id % 8) as u8, FAMILY_WORLD_STATE_B)
        }
        FAMILY_CODE_TILE_WORLD => {
            if !(1_000_000_000..2_000_000_000).contains(&flag_id) || flag_id % 10_000 >= 7_000 {
                return FlagOffset::invalid();
            }
            let off = calculate_tile_pickup_offset_with_base(flag_id, 0);
            if !off.valid {
                return FlagOffset::invalid();
            }
            (off.byte_offset as u64, off.bit_position, FAMILY_TILE_OPEN_WORLD)
        }
        FAMILY_CODE_TILE_PICKUP => {
            if !(1_000_000_000..2_000_000_000).contains(&flag_id) {
                return FlagOffset::invalid();
            }
            let row_id = if flag_id % 10_000 >= 7_000 { flag_id - 7_000 } else { flag_id };
            let off = calculate_tile_pickup_offset_with_base(row_id, 0);
            if !off.valid {
                return FlagOffset::invalid();
            }
            (off.byte_offset as u64, off.bit_position, FAMILY_TILE_PICKUP_ROW_ID)
        }
        FAMILY_CODE_DUNGEON => {
            if flag_id % 10_000 >= 7_000 {
                return FlagOffset::invalid();
            }
            match legacy_dungeon_rel_byte(flag_id) {
                Some(rel) => (rel, 7 - (flag_id % 8) as u8, FAMILY_LEGACY_DUNGEON),
                None => return FlagOffset::invalid(),
            }
        }
        FAMILY_CODE_DUNGEON_PICKUP => {
            if flag_id % 10_000 < 7_000 {
                return FlagOffset::invalid();
            }
            match legacy_dungeon_rel_byte(flag_id) {
                Some(rel) => (rel, 7 - (flag_id % 8) as u8, FAMILY_LEGACY_DUNGEON_PICKUP),
                None => return FlagOffset::invalid(),
            }
        }
        _ => return FlagOffset::invalid(),
    };

    let base = match resolve_family_base_in_ef(event_flags, fam_const) {
        Some(b) => b as u64,
        None => return FlagOffset::invalid(),
    };
    let byte = base + rel;
    if byte >= event_flags.len() as u64 {
        return FlagOffset::invalid();
    }
    FlagOffset::new(byte as u32, bit)
}

// ---------------------------------------------------------------------------
// Tri-state reads, resolved once per save
// ---------------------------------------------------------------------------
//
// Every family's base is `origin + FAMILY_CONSTANT`, and the origin is found by
// scanning from EF+16,000 for the end of the append-only record list — roughly
// 13,400 bytes of scan. Resolving per FLAG would repeat that scan once per read,
// so a screen listing 4,809 pickups would pay it 4,809 times for an answer that
// cannot change between rows. `ResolvedFlags` pays it once and answers each
// family from a cached base. (Free `is_*_set` readers that resolved per call
// existed until v0.37.9; they were deleted once every caller held a
// `ResolvedFlags`.)
//
// The second reason is the more important one. `Option<bool>` is a correct
// tri-state and a poor one: `unwrap_or(false)`, `unwrap_or_default()` and
// `is_some_and()` all turn "we could not tell" into "no" in a way that compiles,
// reads naturally, and is wrong. `FlagState` names the third state and offers
// exactly one way back to a bool, spelled so the call site admits what it is
// discarding.

/// The three outcomes of reading a flag.
///
/// `Unknown` is NOT `Clear` (`CONTEXT.md` → Unknown). It means the position
/// could not be resolved — an unresolvable origin, an id belonging to no known
/// family, a DLC tile with no verified layout — so nothing at all is known about
/// the flag. Rendering it as "not collected" is the failure that made
/// `batch-validate` report 0/110 boss defeats on a finished character.
///
/// There is deliberately no `is_set()`. That method is how the distinction gets
/// lost: `GraceStatus::is_discovered()` was exactly it, and returned `false` for
/// the unreliable case.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[must_use]
pub enum FlagState {
    Set,
    Clear,
    Unknown,
}

impl FlagState {
    /// Collapse to a bool, treating `Unknown` as clear.
    ///
    /// Every call is a place a real distinction is deliberately discarded, which
    /// is why it is named rather than spelled `is_set()`: `grep -rn
    /// 'unknown_as_clear'` is the complete audit list of those places. Legitimate
    /// when an absent answer and a negative answer genuinely mean the same thing
    /// to the caller — building a region filter, say, where an unresolved save
    /// should offer no regions. Not legitimate when a user is reading the value.
    pub fn unknown_as_clear(self) -> bool {
        matches!(self, FlagState::Set)
    }

    /// WASM/FFI encoding: -1 unresolved, 0 clear, 1 set.
    pub fn as_i32(self) -> i32 {
        match self {
            FlagState::Unknown => -1,
            FlagState::Clear => 0,
            FlagState::Set => 1,
        }
    }
}

impl From<Option<bool>> for FlagState {
    fn from(v: Option<bool>) -> Self {
        match v {
            None => FlagState::Unknown,
            Some(false) => FlagState::Clear,
            Some(true) => FlagState::Set,
        }
    }
}

impl From<FlagState> for Option<bool> {
    fn from(v: FlagState) -> Self {
        match v {
            FlagState::Unknown => None,
            FlagState::Clear => Some(false),
            FlagState::Set => Some(true),
        }
    }
}

/// One save's flag region with every family's base already resolved.
///
/// Construction is where refusal happens: no origin, no `ResolvedFlags`. Holding
/// one is a promise that the origin was found — not that any given flag can be
/// read, which is why the methods still return `FlagState` and can still answer
/// `Unknown` for an id whose family has no verified layout.
///
/// It borrows the flag region rather than copying it, so the resolved bases and
/// the bytes they were measured from cannot be separated and then recombined
/// with a different save's.
///
/// Deliberately NOT `#[wasm_bindgen]`. Exporting it would put the primary reader
/// behind `impl` methods, which `tests/export_shape_conformance.rs` does not
/// scan — the ADR-0008 guard would silently stop covering it. The exported
/// surface stays the flat `*_state` functions.
pub struct ResolvedFlags<'a> {
    ef: &'a [u8],
    origin: usize,
    world_state: Option<usize>,
    tile_world: Option<usize>,
    tile_pickup: Option<usize>,
    dungeon: Option<usize>,
    dungeon_pickup: Option<usize>,
}

impl<'a> ResolvedFlags<'a> {
    /// Resolve every family base for this flag region, or refuse.
    ///
    /// `None` means the origin could not be pinned, so nothing in this region can
    /// be read and no caller should pretend otherwise.
    pub fn from_event_flags(ef: &'a [u8]) -> Option<Self> {
        let origin = find_flag_list_end_in_ef(ef)?;
        let base = |c: i64| {
            let b = origin as i64 + c;
            (b >= 0 && (b as usize) < ef.len()).then_some(b as usize)
        };
        Some(Self {
            ef,
            origin,
            world_state: base(FAMILY_WORLD_STATE_B),
            tile_world: base(FAMILY_TILE_OPEN_WORLD),
            tile_pickup: base(FAMILY_TILE_PICKUP_ROW_ID),
            dungeon: base(FAMILY_LEGACY_DUNGEON),
            dungeon_pickup: base(FAMILY_LEGACY_DUNGEON_PICKUP),
        })
    }

    /// End of the append-only record list, relative to the flag region start.
    pub fn origin(&self) -> usize {
        self.origin
    }

    /// Base of one family, or `None` if it falls outside this region.
    pub fn family_base(&self, family_constant: i64) -> Option<usize> {
        match family_constant {
            FAMILY_WORLD_STATE_B => self.world_state,
            FAMILY_TILE_OPEN_WORLD => self.tile_world,
            FAMILY_TILE_PICKUP_ROW_ID => self.tile_pickup,
            FAMILY_LEGACY_DUNGEON => self.dungeon,
            FAMILY_LEGACY_DUNGEON_PICKUP => self.dungeon_pickup,
            _ => None,
        }
    }

    fn read(&self, base: Option<usize>, rel: u64, bit: u8) -> FlagState {
        let Some(base) = base else {
            return FlagState::Unknown;
        };
        let Some(byte) = base.checked_add(rel as usize) else {
            return FlagState::Unknown;
        };
        match self.ef.get(byte) {
            None => FlagState::Unknown,
            Some(b) => {
                if (b >> bit) & 1 == 1 {
                    FlagState::Set
                } else {
                    FlagState::Clear
                }
            }
        }
    }

    /// Graces and world state, `[50000, 80000)`.
    pub fn world_state(&self, flag_id: u32) -> FlagState {
        if !(50_000..80_000).contains(&flag_id) {
            return FlagState::Unknown;
        }
        self.read(
            self.world_state,
            ((flag_id - 50_000) / 8) as u64,
            7 - (flag_id % 8) as u8,
        )
    }

    /// Open-world tile flags: boss kills, world state. NOT pickups.
    ///
    /// A bare 10-digit id with localId < 7000 is ambiguous between this family
    /// and `tile_pickup`, whose region sits 500 bytes away. The caller chooses;
    /// this method never guesses.
    pub fn tile_world(&self, flag_id: u32) -> FlagState {
        if !(1_000_000_000..2_000_000_000).contains(&flag_id) || flag_id % 10_000 >= 7_000 {
            return FlagState::Unknown;
        }
        self.tile_read(self.tile_world, flag_id)
    }

    /// World pickups, addressed by ItemLotParam row_id. Accepts either the
    /// row_id or the getItemFlagId (row_id + 7000) and normalises.
    pub fn tile_pickup(&self, id: u32) -> FlagState {
        if !(1_000_000_000..2_000_000_000).contains(&id) {
            return FlagState::Unknown;
        }
        let row_id = if id % 10_000 >= 7_000 { id - 7_000 } else { id };
        self.tile_read(self.tile_pickup, row_id)
    }

    fn tile_read(&self, base: Option<usize>, addr_id: u32) -> FlagState {
        let off = calculate_tile_pickup_offset_with_base(addr_id, 0);
        if !off.valid {
            return FlagState::Unknown;
        }
        self.read(base, off.byte_offset as u64, 7 - (addr_id % 8) as u8)
    }

    /// Legacy-map event flags: boss kills, world state, NPC state. NOT pickups.
    pub fn dungeon(&self, flag_id: u32) -> FlagState {
        if flag_id % 10_000 >= 7_000 {
            return FlagState::Unknown;
        }
        self.dungeon_read(self.dungeon, flag_id)
    }

    /// Legacy-map pickups (localId >= 7000), in their own region.
    ///
    /// SETTLED 2026-07-20 (docs/BACKLOG.md, step 4b). This family's base sits
    /// 125 bytes below the event family's while both index by the raw
    /// `localId / 8`, so the two computed address ranges OVERLAP: event localId
    /// L and pickup localId L + 1000 land on the same bit. That is a real
    /// property of the layout, not a modelling error — the single-base
    /// alternative was refuted directly, in b33, where a known-set event flag
    /// reads clear at the pickup base and the byte that actually flipped for
    /// pickup 30027000 is at the pickup base.
    ///
    /// It is harmless because the overlap band is empty on the event side.
    /// Legacy event flags cluster in localId 0-2999 and pickups in 7000-7999;
    /// 6000-6999 is used by neither. Checked across 4,540 distinct legacy flags
    /// from three independent sources and against the primary `ItemLotParam_map`
    /// (regulation 1.16.1), which has 2,143 legacy getItemFlagIds in 7000-7999
    /// and none in 6000-6999. If a legacy event flag with localId in 6000-6999
    /// is ever found, it collides with a real pickup and this layout needs
    /// revisiting. Nothing else does.
    pub fn dungeon_pickup(&self, flag_id: u32) -> FlagState {
        if flag_id % 10_000 < 7_000 {
            return FlagState::Unknown;
        }
        self.dungeon_read(self.dungeon_pickup, flag_id)
    }

    fn dungeon_read(&self, base: Option<usize>, flag_id: u32) -> FlagState {
        match legacy_dungeon_rel_byte(flag_id) {
            None => FlagState::Unknown,
            Some(rel) => self.read(base, rel, 7 - (flag_id % 8) as u8),
        }
    }
}
