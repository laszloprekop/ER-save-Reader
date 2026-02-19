//! WebAssembly module for Elden Ring EventFlags detection and pickup flag calculations
//!
//! This is the **SINGLE SOURCE OF TRUTH** for:
//! - EventFlags offset detection
//! - Pickup flag offset calculations (dungeon, tile, block)
//!
//! Used by both ER-save-Editor (native Rust) and elden-map (via WASM).
//!
//! ## Documentation
//!
//! - ER-save-Editor: `docs/WASM-EVENT-FLAGS.md`
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
use std::collections::HashMap;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Event flags section size (constant across all saves)
pub const EVENT_FLAGS_SIZE: usize = 0x1BF99F;  // 1,833,375 bytes

/// Search parameters for EventFlags detection
pub const SEARCH_START: usize = 0x30000;  // 196608 - skip inventory region where false positives occur
pub const MAX_SEARCH_RANGE: usize = 200_000;

/// Tile flag constants (10-digit flags like 1035537020)
pub const TILE_BASE_OFFSET: u32 = 337375;
pub const TILE_ROW_BASE: u32 = 33;
pub const TILE_COL_BASE: u32 = 30;
pub const TILE_BYTES_PER_SLOT: u32 = 875;
pub const TILE_SLOTS_PER_ROW: u32 = 40;
pub const TILE_MAX_LOCAL_ID: u32 = 6999;

/// World pickup row_id base (DEPRECATED — kept for backward compatibility)
/// CORRECTED (2026-02-16): The row_id formula is NOT how the game stores tile
/// pickup flags. Pickups with getItemFlagId local_id >= 7000 are stored in the
/// TILE region at converted local_id (flagId - 7000). See
/// calculate_tile_flag_offset_unified() for the correct routing.
pub const WORLD_PICKUP_ROW_ID_BASE: u32 = 1037373320;

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

/// Internal implementation (also usable from native Rust without WASM)
///
/// Detection strategy (ordered by reliability):
/// 1. **Structural computation** (primary): Sequential section parsing from slot start
///    through all intermediate sections to EventFlags. Deterministic, zero false positives.
///    Works even for brand-new characters with zero graces.
/// 2. **Content-based search** (fallback): Scan for grace flag patterns in the data.
///    Only used if structural computation fails (data corruption, unknown format).
pub fn detect_event_flags_offset_impl(slot_data: &[u8]) -> DetectionResult {
    // === PRIMARY: Structural computation ===
    if let Some(structural_offset) = compute_structural_ef_offset(slot_data) {
        // Validate the structural offset against grace flags (sanity check only)
        let (_tier1_score, positive_score, negative_score) = validate_at_offset(slot_data, structural_offset);

        return DetectionResult {
            offset: structural_offset,
            positive_score,
            negative_score,
            // Confident if we have structural + at least some grace validation,
            // OR if we have structural alone (new characters have no graces but offset is still correct)
            confident: true,
        };
    }

    // === FALLBACK: Content-based search ===
    // Only reached if structural computation fails (e.g., data too short, corrupted GaItems)
    detect_event_flags_content_based(slot_data)
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

/// Per-section pickup bases for dungeon flags (local_id >= 7000)
/// DISCOVERY (2026-02-02): Each (area, section) has its own empirically-discovered base.
/// The linear formula `base + section * 1125` is WRONG.
fn get_dungeon_pickup_section_bases() -> HashMap<(u32, u32), u32> {
    HashMap::from([
        // Area 10: Stormveil Castle
        ((10,  0), 31904),
        ((10,  1),  1787),
        // Area 11: Leyndell Royal Capital
        ((11,  0), 31903),
        ((11,  5),  1835),
        ((11, 10),  1812),
        // Area 12: Underground (Siofra, Ainsel, etc.)
        ((12,  1), 31900),
        ((12,  2), 31903),
        ((12,  3), 31902),
        ((12,  5), 31902),
        ((12,  7), 31903),
        // Area 13: Crumbling Farum Azula
        ((13,  0), 31903),
        // Area 14: Academy of Raya Lucaria
        ((14,  0), 31903),
        // Area 15: Miquella's Haligtree
        ((15,  0), 31903),
        // Area 16: Volcano Manor
        ((16,  0), 31903),
        // Area 18: Roundtable Hold
        ((18,  0),  3847),
        // Area 20: Stranded Graveyard
        ((20,  0), 31903),
        ((20,  1), 31903),
        // Area 21: Haligtree (Elphael)
        ((21,  0), 31903),
        ((21,  1), 31903),
        ((21,  2), 31903),
        // Area 22: Castle Sol
        ((22,  0), 28962),
        // Area 28: DLC
        ((28,  0), 28974),
        // Area 30: Catacombs (21 sections)
        ((30,  0),  1790),
        ((30,  1),  1786),
        ((30,  2),  1787),
        ((30,  3),  1835),
        ((30,  4),  1787),
        ((30,  5),  1835),
        ((30,  6),  3827),
        ((30,  7),  1812),
        ((30,  8),  1834),
        ((30,  9),  3764),
        ((30, 10),  3826),
        ((30, 11),  1787),
        ((30, 12),  1787),
        ((30, 13),  1785),
        ((30, 14),  1835),
        ((30, 15),  1787),
        ((30, 16),  1835),
        ((30, 17),  1835),
        ((30, 18),  1787),
        ((30, 19),  1835),
        ((30, 20),  3723),
        // Area 31: Caves (19 sections)
        ((31,  0),  1787),
        ((31,  1),  1835),
        ((31,  2),  1797),
        ((31,  3),  1787),
        ((31,  4),  1835),
        ((31,  5),  3828),
        ((31,  6),  1787),
        ((31,  7),  3764),
        ((31,  9),  1835),
        ((31, 10),  1790),
        ((31, 11), 28975),
        ((31, 12), 28974),
        ((31, 15),  1786),
        ((31, 17),  3719),
        ((31, 18),  3718),
        ((31, 19), 28974),
        ((31, 20),  1787),
        ((31, 21), 31903),
        ((31, 22),  3827),
        // Area 32: Tunnels (8 sections)
        ((32,  0),  3847),
        ((32,  1),  1835),
        ((32,  2),  3847),
        ((32,  4),  1835),
        ((32,  5),  3723),
        ((32,  7),  1788),
        ((32,  8), 28979),
        ((32, 11),  3725),
        // Area 34: Divine Towers
        ((34, 10),  1787),
        ((34, 11), 31902),
        ((34, 12), 28974),
        ((34, 13),  1787),
        ((34, 14),  1789),
        // Area 35: Mohgwyn Palace
        ((35,  0), 31903),
        // Area 39: Elden Throne
        ((39, 20), 28974),
        // Area 40: Hero's Graves
        ((40,  0), 28986),
        ((40,  1), 28974),
        ((40,  2), 28974),
        // Area 41: Minor Dungeons
        ((41,  0), 31903),
        ((41,  1), 31902),
        ((41,  2), 31903),
        // Area 42: Crystal Caves
        ((42,  0),  3708),
        ((42,  2),  3827),
        ((42,  3),  3708),
        // Area 43: Evergaols
        ((43,  0),  1835),
        ((43,  1),  1835),
    ])
}

// =============================================================================
// BLOCK, MIDRANGE, AND GENERAL DUNGEON BASES
// =============================================================================

/// Block bases for flags 60000-99999 (special system flags)
/// Bases derived from common.emevd.js event definitions (see EVENT-FLAG-GEOGRAPHY.md)
/// Key: block start (e.g., 60000, 71800, 76000)
/// Value: base offset in event flags array (relative to EF section start)
///
/// CORRECTED 2026-02-15: Non-grace block bases were wrong — they had been calibrated
/// against a false-positive EF offset (0x1A570 in GaItemData section) rather than the
/// correct structural EF offset. Correct values from game event scripts (common.emevd.js)
/// and verified via timeline diffs for map fragments (6/6 exact bit matches at base 1500).
/// Sub-block bases (100-granularity) — checked FIRST in calculate_simple_flag_offset.
/// These override the main-block when a flag falls in a sub-block range with a
/// different allocation (e.g., base game cookbooks in DLC blocks, Stormveil graces).
fn get_sub_block_bases() -> HashMap<u32, u32> {
    HashMap::from([
        // Stormveil dungeon graces (71000-71099) — separate allocation region
        // Verified: 55 flags in ground_truth_offsets.json, base 9315 confirmed
        (71000, 9315),
        // Tutorial graces (71800-71899) — VALIDATED via 71800, 71801
        (71800, 2725),
        // Base game cookbook sub-block overrides
        // The DLC introduced SEPARATE flag allocations for blocks 67000/68000.
        // Base game cookbook flags (getItemFlagId from ItemLotParam_map) are stored
        // at DIFFERENT byte offsets than DLC cookbook flags in the same nominal block.
        // Verified empirically: Confessor (4/4 non-ADA), Bee (2/2) match at these bases.
        (68200, 1500),    // Base game Fevor's/Missionary's[7] cookbooks (base 1475 + 200/8)
        (68400, 1525),    // Base game Frenzied's cookbooks (base 1475 + 400/8)
        (67600, 2145),    // Base game Missionary's cookbooks (base 2070 + 600/8)
    ])
}

/// Main-block bases (1000-granularity) — fallback when no sub-block matches.
fn get_main_block_bases() -> HashMap<u32, u32> {
    HashMap::from([
        // System flags — emevd hex + 1 for non-map/non-progression blocks
        // Blocks 60000/62000: emevd hex value IS the correct byte offset
        // Blocks 65000-69000, 91000-92000: emevd hex + 1 (verified via mod-10 flag alignment)
        (60000, 1260),    // 0x4ec - Progression flags — VERIFIED (60020,60130,60220 SET)
        (62000, 1500),    // 0x5dc - Map/Landmarks — VERIFIED via 6 timeline diffs
        (65000, 1685),    // 0x694+1 - Whetblades & Crystal Tears
        (66000, 1725),    // 0x6bc+1 - Pot/Perfume Upgrades
        (67000, 1765),    // 0x6e4+1 - Cookbooks (DLC) — VERIFIED (6/6 flags mod10=0)
        (68000, 1805),    // 0x70c+1 - Cookbooks continued (DLC) — VERIFIED (16/16 mod10=0)
        (69000, 1845),    // 0x734+1 - Remembrance/Notes — VERIFIED (20/20 mod10=0)
        (91000, 2385),    // 0x950+1 - Boss Remembrance — VERIFIED (41/41 mod10=0)
        (92000, 2425),    // 0x978+1 - Container Upgrades — VERIFIED (16/16 mod10=0)
        // Dungeon graces (71100-71799) — standard block formula with base 2625
        // Confirmed by computing all 55 verified flags in ground_truth_offsets.json
        (71000, 2625),
        // Grace flags — verified via multi-slot validation
        (72000, 2750),    // DLC graces (Enir-Ilim) - verified (10+ consistent proven)
        (73000, 2662),    // Dungeon graces - verified via temporal diff
        (74000, 3000),    // DLC dungeon graces - verified (8+ consistent proven)
        (76000, 3250),    // 0xcb2 - World graces - VALIDATED via 76100, 76101
        (78000, 3500),    // Grace guidance flags - verified (8+ proven flags)
    ])
}

/// Midrange block bases for 6-digit flags (100000-999999)
/// SYNCED with ground_truth_offsets.json verified values from eventFlagService.ts
fn get_midrange_bases() -> HashMap<u32, u32> {
    HashMap::from([
        (510000, 63750),  // Remembrance consumption flags - verified
        (540000, 67500),  // Sorcery/Incantation/Ash unlock flags - verified (129 flags)
        (710000, 13875),  // Roundtable Hold NPC progression - verified (41 EMEVD flags)
    ])
}

/// General dungeon base offsets (for local_id < 7000)
/// From eventflagalloclist - formula: base + section * 1125 + local_id / 8
/// Key format: "XX_YY" where XX is map area, YY is section
fn get_dungeon_general_bases() -> HashMap<(u32, u32), u32> {
    HashMap::from([
        // Stormveil Castle (m10)
        ((10,  0), 4112), ((10,  1), 5237),
        // Leyndell (m11) - section formula: 8612 + section * 1125
        ((11,  0), 8612), ((11,  5), 14237), ((11, 10), 19862), ((11, 71), 88487),
        // Underground areas (m12)
        ((12,  1), 16487), ((12,  2), 17612), ((12,  3), 18737), ((12,  4), 19862),
        ((12,  5), 20987), ((12,  6), 22112), ((12,  7), 23237), ((12,  8), 24362), ((12,  9), 25487),
        // Crumbling Farum Azula (m13)
        ((13,  0), 26612),
        // Academy of Raya Lucaria (m14)
        ((14,  0), 29987),
        // Miquella's Haligtree (m15)
        ((15,  0), 33362),
        // Volcano Manor (m16) - verified (was 36737 - WRONG, corrected to 40517)
        ((16,  0), 40517),
        // Roundtable Hold (m18)
        ((18,  0), 43487),
        // Chapel of Anticipation (m19)
        ((19,  0), 46862),
        // Stranded Graveyard (m20)
        ((20,  0), 50237),
        // Miquella's Haligtree sections (m21)
        ((21,  0), 53612), ((21,  1), 54737), ((21,  2), 55862),
        // Castle Sol (m22)
        ((22,  0), 59237),
        // Catacombs (m30) - VERIFIED base 27411
        ((30,  0), 27411), ((30,  1), 28536), ((30,  2), 29661), ((30,  3), 30786),
        ((30,  4), 31911), ((30,  5), 33036), ((30,  6), 34161), ((30,  7), 35286),
        ((30,  8), 36411), ((30,  9), 37536), ((30, 10), 38661), ((30, 11), 39786),
        ((30, 12), 40911), ((30, 13), 42036), ((30, 14), 43161), ((30, 15), 44286),
        ((30, 16), 45411), ((30, 17), 46536), ((30, 18), 47661), ((30, 19), 48786), ((30, 20), 49911),
        // Caves (m31) - VERIFIED base 28634
        ((31,  0), 28634), ((31,  1), 29759), ((31,  2), 30884), ((31,  3), 32009),
        ((31,  4), 33134), ((31,  5), 34259), ((31,  6), 35384), ((31,  7), 36509),
        ((31,  9), 38759), ((31, 10), 39884), ((31, 11), 41009), ((31, 12), 42134),
        ((31, 15), 45509), ((31, 17), 47759), ((31, 18), 48884), ((31, 19), 50009),
        ((31, 20), 51134), ((31, 21), 52259), ((31, 22), 53384),
        // Tunnels (m32) - VERIFIED base 31577
        ((32,  0), 31577), ((32,  1), 32702), ((32,  2), 33827), ((32,  4), 36077),
        ((32,  5), 37202), ((32,  7), 39452), ((32,  8), 40577), ((32, 11), 43952),
        // Divine Towers (m34)
        ((34, 10), 71612), ((34, 11), 72737), ((34, 12), 73862), ((34, 13), 74987),
        ((34, 14), 76112), ((34, 15), 77237), ((34, 16), 78362),
        // Mohgwyn Palace (m35)
        ((35,  0), 50237),
        // Elden Throne (m39)
        ((39, 20), 53612),
    ])
}

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

/// Calculate offset for dungeon pickup flags (8-digit, local_id >= 7000)
///
/// Uses per-section lookup instead of linear formula.
/// Returns None if section base is unknown.
#[wasm_bindgen]
pub fn calculate_dungeon_pickup_offset(flag_id: u32) -> FlagOffset {
    calculate_dungeon_pickup_offset_impl(flag_id)
}

pub fn calculate_dungeon_pickup_offset_impl(flag_id: u32) -> FlagOffset {
    // Must be 8-digit dungeon flag
    if flag_id < 10_000_000 || flag_id >= 44_000_000 {
        return FlagOffset::invalid();
    }

    let area = (flag_id / 1_000_000) % 100;
    let section = (flag_id / 10_000) % 100;
    let local_id = flag_id % 10_000;

    // Only handles pickup flags (local_id >= 7000)
    if local_id < 7000 {
        return FlagOffset::invalid();
    }

    let section_bases = get_dungeon_pickup_section_bases();

    if let Some(&section_base) = section_bases.get(&(area, section)) {
        let byte_offset = section_base + local_id / 8;
        let bit_position = (7 - (flag_id % 8)) as u8;

        if byte_offset < EVENT_FLAGS_SIZE as u32 {
            return FlagOffset::new(byte_offset, bit_position);
        }
    }

    FlagOffset::invalid()
}

/// Calculate offset for tile-based world pickup flags (10-digit, 1XXYYZZZZ)
#[wasm_bindgen]
pub fn calculate_tile_pickup_offset(flag_id: u32) -> FlagOffset {
    calculate_tile_pickup_offset_with_base(flag_id, TILE_BASE_OFFSET)
}

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

/// Calculate offset for world pickup tracking using ItemLotParam row_id
///
/// DISCOVERY (2026-02-02): World pickups with getItemFlagId (local_id >= 7000)
/// are NOT stored in the tile region. Instead, they're tracked by row_id
/// in a separate bitfield region at a different offset.
///
/// Formula:
/// - byte_offset = (row_id - WORLD_PICKUP_ROW_ID_BASE) / 8
/// - bit_position = 7 - ((row_id - WORLD_PICKUP_ROW_ID_BASE) % 8)
///
/// NOTE (2026-02-09): Row_ids with local_id < 7000 (like 1044360310) are
/// actually stored via the TILE formula, not this row_id formula. This formula
/// applies only to row_ids whose corresponding getItemFlagId would have
/// local_id >= 7000 AND no valid tile slot (i.e., the tile formula returns
/// invalid). The unified get_flag_offset() handles routing correctly.
#[wasm_bindgen]
pub fn calculate_world_pickup_offset_by_row_id(row_id: u32) -> FlagOffset {
    calculate_world_pickup_offset_by_row_id_impl(row_id)
}

pub fn calculate_world_pickup_offset_by_row_id_impl(row_id: u32) -> FlagOffset {
    // Only valid for 10-digit row_ids in the 1B range
    if row_id < 1_000_000_000 || row_id >= 2_000_000_000 {
        return FlagOffset::invalid();
    }

    // row_id should have local_id < 7000 (it's the raw ItemLotParam row_id)
    let local_id = row_id % 10000;
    if local_id >= 7000 {
        return FlagOffset::invalid();
    }

    // Must be >= base to have valid storage
    if row_id < WORLD_PICKUP_ROW_ID_BASE {
        return FlagOffset::invalid();
    }

    let bit_offset = row_id - WORLD_PICKUP_ROW_ID_BASE;
    let byte_offset = bit_offset / 8;
    let bit_position = (7 - (bit_offset % 8)) as u8;

    if byte_offset < EVENT_FLAGS_SIZE as u32 {
        FlagOffset::new(byte_offset, bit_position)
    } else {
        FlagOffset::invalid()
    }
}

/// Convert getItemFlagId to storable row_id for tile-based world pickups.
///
/// DISCOVERY (2026-01-23): For tile-based world pickups:
/// - ItemLotParam has getItemFlagId = row_id + 7000
/// - The game stores row_id (localId 0-999), NOT getItemFlagId (localId 7000+)
#[wasm_bindgen]
pub fn convert_to_row_id(flag_id: u32) -> i64 {
    // Only applies to 10-digit tile flags (1B range)
    if flag_id < 1_000_000_000 || flag_id >= 2_000_000_000 {
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
    if flag_id < 10_000_000 || flag_id >= 44_000_000 {
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

/// Get all known dungeon pickup section keys as JSON
/// Returns array of [area, section] pairs
#[wasm_bindgen]
pub fn get_dungeon_pickup_sections() -> String {
    let bases = get_dungeon_pickup_section_bases();
    let keys: Vec<(u32, u32)> = bases.keys().cloned().collect();
    serde_json::to_string(&keys).unwrap_or_else(|_| "[]".to_string())
}

// =============================================================================
// UNIFIED FLAG OFFSET CALCULATION
// =============================================================================

/// Calculate byte offset and bit position for ANY event flag.
///
/// This is the unified entry point that handles all flag types:
/// - Simple flags (< 60,000): Direct flag_id / 8 calculation
/// - Block flags (60,000-99,999): Uses BLOCK_BASES lookup
/// - Midrange flags (100,000-999,999): Uses MIDRANGE_BASES lookup
/// - Dungeon flags (10,000,000-43,999,999): General events and pickup flags
/// - Tile flags (>= 1,000,000,000): Formula-based tile calculation
///
/// NOTE: Uses static TILE_BASE_OFFSET. For calibrated tile results, use
/// get_flag_offset_calibrated() with a per-save calibrated base.
#[wasm_bindgen]
pub fn get_flag_offset(flag_id: u32) -> FlagOffset {
    get_flag_offset_with_tile_base(flag_id, TILE_BASE_OFFSET)
}

/// Calculate flag offset using a calibrated tile base.
/// Same as get_flag_offset() but uses a per-save calibrated tile base
/// for accurate tile flag results.
#[wasm_bindgen]
pub fn get_flag_offset_calibrated(flag_id: u32, tile_base: u32) -> FlagOffset {
    get_flag_offset_with_tile_base(flag_id, tile_base)
}

fn get_flag_offset_with_tile_base(flag_id: u32, tile_base: u32) -> FlagOffset {
    // 10-digit open world tile flags (1,000,000,000+)
    if flag_id >= 1_000_000_000 {
        return calculate_tile_flag_offset_unified(flag_id, tile_base);
    }

    // 8-digit dungeon flags (10,000,000-43,999,999)
    if flag_id >= 10_000_000 && flag_id < 44_000_000 {
        return calculate_dungeon_flag_offset_unified(flag_id);
    }

    // 6-digit midrange flags (100,000-999,999)
    if flag_id >= 100_000 && flag_id < 1_000_000 {
        return calculate_midrange_flag_offset(flag_id);
    }

    // Simple and block flags (< 100,000)
    if flag_id < 100_000 {
        return calculate_simple_flag_offset(flag_id);
    }

    // Flags 1,000,000-9,999,999 are not commonly used
    FlagOffset::invalid()
}

/// Calculate tile flag offset with getItemFlagId conversion for local_id >= 7000
///
/// CORRECTED (2026-02-16): getItemFlagId (local_id >= 7000) is stored in the TILE
/// region at a converted local_id, NOT in the row_id bitfield region.
///
/// Empirically verified with Axe Talisman (getItemFlagId 1045377100, local_id 7100):
///   - Subtracting 7000 gives 1045370100 (local_id 100)
///   - Flag IS SET at tile (45,37) local_id=100 in the save file
///   - Flag is NOT SET at row_id formula offset 999597
fn calculate_tile_flag_offset_unified(flag_id: u32, tile_base: u32) -> FlagOffset {
    let local_id = flag_id % 10000;

    // getItemFlagId (local_id >= 7000): convert to tile-storable local_id
    // by subtracting 7000, then use standard tile formula
    if local_id > MAX_TILE_LOCAL_ID {
        let converted = flag_id - 7000;
        return calculate_tile_pickup_offset_with_base(converted, tile_base);
    }

    // Standard tile formula
    calculate_tile_pickup_offset_with_base(flag_id, tile_base)
}

/// Calculate dungeon flag offset for both general events (local_id < 7000)
/// and pickup flags (local_id >= 7000)
fn calculate_dungeon_flag_offset_unified(flag_id: u32) -> FlagOffset {
    let area = (flag_id / 1_000_000) % 100;
    let section = (flag_id / 10_000) % 100;
    let local_id = flag_id % 10_000;
    let bit_position = (7 - (flag_id % 8)) as u8;

    // Pickup flags (local_id >= 7000) use per-section bases
    if local_id >= 7000 {
        return calculate_dungeon_pickup_offset_impl(flag_id);
    }

    // General dungeon events (local_id < 7000) use dungeon general bases
    let general_bases = get_dungeon_general_bases();

    // Try exact (area, section) lookup first
    if let Some(&base) = general_bases.get(&(area, section)) {
        let byte_offset = base + local_id / 8;
        if byte_offset < EVENT_FLAGS_SIZE as u32 {
            return FlagOffset::new(byte_offset, bit_position);
        }
    }

    // Fall back: calculate from section 0 base using DUNGEON_SECTION_SIZE
    if let Some(&base_00) = general_bases.get(&(area, 0)) {
        let byte_offset = base_00 + section * DUNGEON_SECTION_SIZE + local_id / 8;
        if byte_offset < EVENT_FLAGS_SIZE as u32 {
            return FlagOffset::new(byte_offset, bit_position);
        }
    }

    FlagOffset::invalid()
}

/// Calculate byte offset for simple flags (< 60,000) and block flags (60,000-99,999)
fn calculate_simple_flag_offset(flag_id: u32) -> FlagOffset {
    let bit = (7 - (flag_id % 8)) as u8;

    // Simple flags use direct calculation
    if flag_id < 60000 {
        let byte_offset = flag_id / 8;
        if byte_offset < EVENT_FLAGS_SIZE as u32 {
            return FlagOffset::new(byte_offset, bit);
        }
        return FlagOffset::invalid();
    }

    // Block flags (60,000-99,999) - check sub-block (100-rounded) then main block (1000-rounded)
    let sub_block = (flag_id / 100) * 100;
    let main_block = (flag_id / 1000) * 1000;

    // Sub-block (100-granularity) — checked first for overrides
    let sub_bases = get_sub_block_bases();
    if let Some(&base) = sub_bases.get(&sub_block) {
        let relative = flag_id - sub_block;
        let byte_offset = base + relative / 8;
        if byte_offset < EVENT_FLAGS_SIZE as u32 {
            return FlagOffset::new(byte_offset, bit);
        }
    }

    // Main-block (1000-granularity) — fallback
    let main_bases = get_main_block_bases();
    if let Some(&base) = main_bases.get(&main_block) {
        let relative = flag_id - main_block;
        let byte_offset = base + relative / 8;
        if byte_offset < EVENT_FLAGS_SIZE as u32 {
            return FlagOffset::new(byte_offset, bit);
        }
    }

    FlagOffset::invalid()
}

/// Calculate byte offset for midrange flags (100,000-999,999)
fn calculate_midrange_flag_offset(flag_id: u32) -> FlagOffset {
    let bit = (7 - (flag_id % 8)) as u8;
    let midrange_bases = get_midrange_bases();

    // Try 10,000-flag block granularity first
    let block_10k = (flag_id / 10000) * 10000;
    if let Some(&base) = midrange_bases.get(&block_10k) {
        let relative = flag_id - block_10k;
        let byte_offset = base + relative / 8;
        if byte_offset < EVENT_FLAGS_SIZE as u32 {
            return FlagOffset::new(byte_offset, bit);
        }
    }

    // Fall back to 1,000-flag granularity
    let block_1k = (flag_id / 1000) * 1000;
    if let Some(&base) = midrange_bases.get(&block_1k) {
        let relative = flag_id - block_1k;
        let byte_offset = base + relative / 8;
        if byte_offset < EVENT_FLAGS_SIZE as u32 {
            return FlagOffset::new(byte_offset, bit);
        }
    }

    FlagOffset::invalid()
}

/// Check if an event flag is set in the event flags data.
/// Combines offset calculation + bit checking in one call.
#[wasm_bindgen]
pub fn is_flag_set(event_flags: &[u8], flag_id: u32) -> bool {
    let result = get_flag_offset(flag_id);
    if !result.valid {
        return false;
    }
    let byte_off = result.byte_offset as usize;
    if byte_off >= event_flags.len() {
        return false;
    }
    (event_flags[byte_off] & (1 << result.bit_position)) != 0
}

/// Check if an event flag is set using a calibrated tile base.
#[wasm_bindgen]
pub fn is_flag_set_calibrated(event_flags: &[u8], flag_id: u32, tile_base: u32) -> bool {
    let result = get_flag_offset_calibrated(flag_id, tile_base);
    if !result.valid {
        return false;
    }
    let byte_off = result.byte_offset as usize;
    if byte_off >= event_flags.len() {
        return false;
    }
    (event_flags[byte_off] & (1 << result.bit_position)) != 0
}

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
pub fn get_tile_base_offset() -> u32 {
    TILE_BASE_OFFSET
}

#[wasm_bindgen]
pub fn get_tile_max_local_id() -> u32 {
    TILE_MAX_LOCAL_ID
}

#[wasm_bindgen]
pub fn get_world_pickup_row_id_base() -> u32 {
    WORLD_PICKUP_ROW_ID_BASE
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

// Section sizes from ER-save-Editor save_slot.rs (authoritative)
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
/// sections (matching ER-save-Editor save_slot.rs read order). Returns JSON
/// with computed offsets and parsed equipment data.
///
/// This is the SINGLE SOURCE OF TRUTH for equipment section offsets,
/// shared between ER-save-Editor and elden-map.
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
    for i in 0..4 {
        talismans[i] = read_u32_le(data, p); p += 4;
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
    for i in 0..4 {
        talismans[i] = read_u32_le(data, p); p += 4;
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
        assert_eq!(SEARCH_START, 0x30000);
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

    #[test]
    fn test_dungeon_pickup_offset_stormveil() {
        // Stormveil Castle section 0, local_id 7000
        let result = calculate_dungeon_pickup_offset_impl(10007000);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 31904 + 7000 / 8); // 32779
        assert_eq!(result.bit_position, 7);
    }

    #[test]
    fn test_dungeon_pickup_offset_catacombs() {
        // Catacombs section 6 (Cliffbottom), local_id 7000
        let result = calculate_dungeon_pickup_offset_impl(30067000);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 3827 + 7000 / 8); // 4702
    }

    #[test]
    fn test_dungeon_pickup_offset_unknown_section() {
        // Area 30, section 99 (doesn't exist)
        let result = calculate_dungeon_pickup_offset_impl(30997000);
        assert!(!result.valid);
    }

    #[test]
    fn test_tile_offset() {
        // Limgrave tile (42, 36), local_id 10
        let result = calculate_tile_pickup_offset_with_base(1042360010, TILE_BASE_OFFSET);
        assert!(result.valid);
    }

    #[test]
    fn test_tile_offset_high_local_id() {
        // local_id >= 7000 should be invalid (no storage)
        let result = calculate_tile_pickup_offset_with_base(1042367000, TILE_BASE_OFFSET);
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

    #[test]
    fn test_section_bases_count() {
        let bases = get_dungeon_pickup_section_bases();
        assert!(bases.len() >= 88); // 88+ verified sections
    }

    #[test]
    fn test_world_pickup_offset_by_row_id() {
        // NOTE: These row_ids (1044360310, 1044360340) have local_id < 7000,
        // so the game actually stores them via the TILE formula, NOT the row_id
        // formula. The row_id formula produces valid-looking results but at the
        // WRONG offsets. The unified get_flag_offset() correctly routes these
        // through the tile formula instead.
        //
        // This test verifies the row_id formula's math is correct, but see
        // test_tile_world_pickup_m60_4_43 for the correct storage locations.
        let result = calculate_world_pickup_offset_by_row_id_impl(1044360310);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 873373);
        assert_eq!(result.bit_position, 1);

        let result = calculate_world_pickup_offset_by_row_id_impl(1044360340);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 873377);
        assert_eq!(result.bit_position, 3);
    }

    #[test]
    fn test_world_pickup_offset_invalid_inputs() {
        // row_id below base should be invalid
        let result = calculate_world_pickup_offset_by_row_id_impl(1037373319);
        assert!(!result.valid);

        // getItemFlagId (local_id >= 7000) should be invalid - must convert first
        let result = calculate_world_pickup_offset_by_row_id_impl(1044367310);
        assert!(!result.valid);

        // 8-digit flag should be invalid
        let result = calculate_world_pickup_offset_by_row_id_impl(10007000);
        assert!(!result.valid);
    }

    #[test]
    fn test_world_pickup_row_id_base() {
        assert_eq!(WORLD_PICKUP_ROW_ID_BASE, 1037373320);
    }

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
    // UNIFIED get_flag_offset TESTS
    // =========================================================================

    #[test]
    fn test_get_flag_offset_simple() {
        // Flag 300: byte = 300/8 = 37, bit = 7 - (300%8) = 7-4 = 3
        let result = get_flag_offset(300);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 37);
        assert_eq!(result.bit_position, 3);
    }

    #[test]
    fn test_get_flag_offset_block_grace() {
        // Flag 76100 (The First Step): block 76000 base=3250, relative=100
        // byte = 3250 + 100/8 = 3262, bit = 7 - (76100%8) = 7-4 = 3
        let result = get_flag_offset(76100);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 3262);
        assert_eq!(result.bit_position, 3);
    }

    #[test]
    fn test_get_flag_offset_block_cookbook() {
        // Flag 67000 (Nomadic Warrior's Cookbook [1]): block 67000 base=1765
        // byte = 1765 + 0/8 = 1765, bit = 7 - (67000%8) = 7-0 = 7
        let result = get_flag_offset(67000);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 1765);
        assert_eq!(result.bit_position, 7);
    }

    #[test]
    fn test_get_flag_offset_midrange() {
        // Flag 540000 (sorcery unlock): block 540000 base=67500
        // byte = 67500 + 0/8 = 67500, bit = 7
        let result = get_flag_offset(540000);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 67500);
        assert_eq!(result.bit_position, 7);
    }

    #[test]
    fn test_get_flag_offset_dungeon_general() {
        // Flag 10000800 (Godrick): area=10, section=0, local=800
        // base = 4112, byte = 4112 + 800/8 = 4212, bit = 7-(800%8) = 7
        let result = get_flag_offset(10000800);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 4212);
        assert_eq!(result.bit_position, 7);
    }

    #[test]
    fn test_get_flag_offset_dungeon_general_catacombs() {
        // Flag 30020800 (Erdtree Burial Watchdog): area=30, section=2, local=800
        // base for 30_02 = 29661, byte = 29661 + 800/8 = 29761, bit = 7
        let result = get_flag_offset(30020800);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 29761);
        assert_eq!(result.bit_position, 7);
    }

    #[test]
    fn test_get_flag_offset_dungeon_pickup() {
        // Flag 10007000 (Stormveil pickup): local_id=7000, pickup base=31904
        // byte = 31904 + 7000/8 = 32779, bit = 7
        let result = get_flag_offset(10007000);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 32779);
        assert_eq!(result.bit_position, 7);
    }

    #[test]
    fn test_get_flag_offset_tile() {
        // Flag 1043500010 (Smoldering Butterfly): empirical byte=704876, bit=5
        // Verified via before/after save captures (09/10) and slot 7 captures (135-149).
        // tile_base(337375) + slot(420)*875 + 10/8 = 337375 + 367500 + 1 = 704876
        let result = get_flag_offset(1043500010);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 704876);
        assert_eq!(result.bit_position, 5);
    }

    #[test]
    fn test_get_flag_offset_tile_row_id() {
        // CORRECTED (2026-02-16): getItemFlagId with local_id >= 7000 routes
        // through tile formula with converted local_id, NOT row_id formula.
        // 1042377300 → converted = 1042370300, local_id=300
        // row=42, col=37, slot=(42-33)*40+(37-30)=367
        // byte = 337375 + 367*875 + 300/8 = 658500 + 37 = 658537, bit = 3
        let result = get_flag_offset(1042377300);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 658537);
        assert_eq!(result.bit_position, 3);
    }

    #[test]
    fn test_get_flag_offset_calibrated_tile() {
        // Using calibrated base 490000 instead of default 337375
        // Flag 1042360010: row=42,col=36,local=10
        // slot = (42-33)*40 + (36-30) = 366
        // byte = 490000 + 366*875 + 10/8 = 490000 + 320250 + 1 = 810251
        let result = get_flag_offset_calibrated(1042360010, 490000);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 810251);
    }

    /// Verify tile formula for 5 world pickups at tile (44,36) in m60_4_43.
    ///
    /// EMPIRICALLY VERIFIED (2026-02-09) via granular before/after save captures
    /// (files 119-127, slot 2 / V1 character). All 5 pickups use the TILE formula
    /// because their local_ids (300-340) are < 7000.
    ///
    /// Timeline order of pickups:
    ///   1. 1044360310 (file 119)
    ///   2. 1044360340 (file 121)
    ///   3. 1044360320 (file 123)
    ///   4. 1044360330 (file 125)
    ///   5. 1044360300 (file 127 - all 5 accumulated)
    ///
    /// Tile slot: (44-33)*40 + (36-30) = 446
    /// All flags cluster in a 6-byte span at tile_base + 446*875 + 37..42.
    /// Tile base: 337375 (verified across 3 characters, 6+ tiles, 10+ snapshot diffs).
    #[test]
    fn test_tile_world_pickup_m60_4_43() {
        // These row_ids route through tile formula via get_flag_offset()
        // because local_id < 7000. Verify with DEFAULT tile base.
        // slot = (44-33)*40 + (36-30) = 446
        // slot_offset = 337375 + 446*875 = 727625

        // 1044360300: local_id=300, byte = 727625 + 300/8 = 727662, bit = 7-(300%8) = 3
        let result = get_flag_offset(1044360300);
        assert!(result.valid, "1044360300 should use tile formula (local_id=300 < 7000)");
        assert_eq!(result.byte_offset, 727662);
        assert_eq!(result.bit_position, 3);

        // 1044360310: local_id=310, byte = 727625 + 310/8 = 727663, bit = 7-(310%8) = 1
        let result = get_flag_offset(1044360310);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 727663);
        assert_eq!(result.bit_position, 1);

        // 1044360320: local_id=320, byte = 727625 + 320/8 = 727665, bit = 7-(320%8) = 7
        let result = get_flag_offset(1044360320);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 727665);
        assert_eq!(result.bit_position, 7);

        // 1044360330: local_id=330, byte = 727625 + 330/8 = 727666, bit = 7-(330%8) = 5
        let result = get_flag_offset(1044360330);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 727666);
        assert_eq!(result.bit_position, 5);

        // 1044360340: local_id=340, byte = 727625 + 340/8 = 727667, bit = 7-(340%8) = 3
        let result = get_flag_offset(1044360340);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 727667);
        assert_eq!(result.bit_position, 3);
    }

    /// Same 5 world pickups but with calibrated tile base.
    /// Empirical tile_base = 337375, verified via before/after save captures
    /// across Confessor (captures 05-10), V1 (119-127), and Slot 7 (135-149).
    /// The correct tile_base matches the default TILE_BASE_OFFSET.
    #[test]
    fn test_tile_world_pickup_m60_4_43_calibrated() {
        let tile_base = 337375;
        // slot_offset = 337375 + 446*875 = 727625

        let result = get_flag_offset_calibrated(1044360300, tile_base);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 727662); // 727625 + 37
        assert_eq!(result.bit_position, 3);

        let result = get_flag_offset_calibrated(1044360310, tile_base);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 727663); // 727625 + 38
        assert_eq!(result.bit_position, 1);

        let result = get_flag_offset_calibrated(1044360320, tile_base);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 727665); // 727625 + 40
        assert_eq!(result.bit_position, 7);

        let result = get_flag_offset_calibrated(1044360330, tile_base);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 727666); // 727625 + 41
        assert_eq!(result.bit_position, 5);

        let result = get_flag_offset_calibrated(1044360340, tile_base);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 727667); // 727625 + 42
        assert_eq!(result.bit_position, 3);
    }

    #[test]
    fn test_get_item_flag_id_tile_routing() {
        // CORRECTED (2026-02-16): getItemFlagId with local_id >= 7000 should route
        // through tile formula with converted local_id, NOT row_id formula.
        //
        // Axe Talisman: getItemFlagId=1045377100, local_id=7100
        // Converted: 1045377100 - 7000 = 1045370100, local_id=100
        // Tile: row=45, col=37, slot=(45-33)*40+(37-30)=487
        // slot_offset = 337375 + 487*875 = 763500
        // byte = 763500 + 100/8 = 763512, bit = 7-(100%8) = 3
        let tile_base = 337375;
        let result = get_flag_offset_calibrated(1045377100, tile_base);
        assert!(result.valid, "getItemFlagId 1045377100 should be valid via tile formula");
        assert_eq!(result.byte_offset, 763512);
        assert_eq!(result.bit_position, 3);

        // Verify the converted flag gives the same result through standard path
        let result_direct = get_flag_offset_calibrated(1045370100, tile_base);
        assert!(result_direct.valid);
        assert_eq!(result_direct.byte_offset, result.byte_offset);
        assert_eq!(result_direct.bit_position, result.bit_position);
    }

    #[test]
    fn test_get_flag_offset_calibrated_non_tile() {
        // Non-tile flags ignore calibrated base
        let result_default = get_flag_offset(76100);
        let result_calibrated = get_flag_offset_calibrated(76100, 999999);
        assert_eq!(result_default.byte_offset, result_calibrated.byte_offset);
        assert_eq!(result_default.bit_position, result_calibrated.bit_position);
    }

    #[test]
    fn test_is_flag_set_basic() {
        // Create synthetic event flags data
        let mut event_flags = vec![0u8; EVENT_FLAGS_SIZE];

        // Set flag 300: byte 37, bit 3
        event_flags[37] = 0b00001000; // bit 3 set

        assert!(is_flag_set(&event_flags, 300));
        assert!(!is_flag_set(&event_flags, 301)); // different bit in same byte
    }

    #[test]
    fn test_is_flag_set_block_flag() {
        let mut event_flags = vec![0u8; EVENT_FLAGS_SIZE];

        // Flag 76100 (The First Step): byte 3262, bit 3
        event_flags[3262] = 0b00001000;

        assert!(is_flag_set(&event_flags, 76100));
        assert!(!is_flag_set(&event_flags, 76101)); // bit 2, not set
    }

    #[test]
    fn test_is_flag_set_unknown_flag() {
        let event_flags = vec![0xFF; EVENT_FLAGS_SIZE]; // all bits set
        // Flag in unknown range (1000000-9999999) should return false
        assert!(!is_flag_set(&event_flags, 5000000));
    }

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
    fn test_detect_ef_structural_primary() {
        // Verify that detect_event_flags_offset_impl uses structural path
        // when structural computation succeeds.
        let header_size = 4 + 4 + 0x18;
        let ga_items_size = GA_ITEMS_MAX * 8;
        let ga_end = header_size + ga_items_size;

        let expected_ef = ga_end + FIXED_BEFORE_PROJECTILE
            + 4 + FIXED_BETWEEN_PROJ_AND_REGIONS
            + 4 + FIXED_AFTER_REGIONS + PRE_EVENT_FLAGS_GAP;

        let total_needed = expected_ef + EVENT_FLAGS_SIZE;
        let slot_data = vec![0u8; total_needed];

        let result = detect_event_flags_offset_impl(&slot_data);
        // Should use structural path and be confident
        assert_eq!(result.offset, expected_ef);
        assert!(result.confident, "Structural detection should be confident");
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
    // DUNGEON GRACE RESOLUTION TESTS (sub-block / main-block split)
    // =========================================================================

    #[test]
    fn test_stormveil_grace_sub_block() {
        // Flag 71000: sub_block=71000 → hits sub_bases(9315), relative=0
        // byte = 9315 + 0/8 = 9315, bit = 7 - (71000%8) = 7
        let result = get_flag_offset(71000);
        assert!(result.valid, "Flag 71000 should resolve via Stormveil sub-block");
        assert_eq!(result.byte_offset, 9315);
        assert_eq!(result.bit_position, 7);
    }

    #[test]
    fn test_dungeon_grace_main_block_fallback() {
        // Flag 71120: sub_block=71100 → miss in sub_bases
        //             main_block=71000 → hits main_bases(2625), relative=120
        // byte = 2625 + 120/8 = 2640, bit = 7 - (71120%8) = 7
        let result = get_flag_offset(71120);
        assert!(result.valid, "Flag 71120 should resolve via main-block fallback");
        assert_eq!(result.byte_offset, 2640);
        assert_eq!(result.bit_position, 7);
    }

    #[test]
    fn test_tutorial_grace_sub_block() {
        // Flag 71800: sub_block=71800 → hits sub_bases(2725), relative=0
        // byte = 2725, bit = 7 - (71800%8) = 7
        let result = get_flag_offset(71800);
        assert!(result.valid, "Flag 71800 should resolve via tutorial sub-block");
        assert_eq!(result.byte_offset, 2725);
        assert_eq!(result.bit_position, 7);
    }

    #[test]
    fn test_world_grace_unchanged() {
        // Flag 76100: sub_block=76100 → miss, main_block=76000 → hits(3250)
        // byte = 3250 + 100/8 = 3262, bit = 3
        // This must remain unchanged from before the split.
        let result = get_flag_offset(76100);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 3262);
        assert_eq!(result.bit_position, 3);
    }

    #[test]
    fn test_dungeon_grace_leyndell() {
        // Flag 71100 (Leyndell range): sub=71100 → miss, main=71000 → 2625
        // relative = 100, byte = 2625 + 100/8 = 2637, bit = 7-(71100%8) = 3
        let result = get_flag_offset(71100);
        assert!(result.valid, "Flag 71100 should resolve via main-block");
        assert_eq!(result.byte_offset, 2637);
        assert_eq!(result.bit_position, 3);
    }

    #[test]
    fn test_sub_block_bases_no_conflict() {
        // Verify sub-block and main-block don't share keys
        let sub = get_sub_block_bases();
        let main = get_main_block_bases();
        // Key 71000 is intentionally in BOTH — sub for 100-granularity, main for 1000-granularity
        // But they resolve at different levels (sub_block=71000 vs main_block=71000)
        // All other sub-block keys should NOT appear in main-block
        for &key in sub.keys() {
            if key == 71000 {
                continue; // Expected dual presence
            }
            assert!(!main.contains_key(&key),
                "Sub-block key {} should not appear in main-block bases", key);
        }
    }

}
