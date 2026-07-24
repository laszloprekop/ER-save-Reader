//! Module for calculating event flag offsets and tracking item pickups
//!
//! The EventFlags array (0x1BF99F bytes) uses hierarchical allocation:
//! - Small flags (0-59999): Direct mapping with base offset
//! - Midrange flags (100000-999999): Sorcery/incantation unlock flags
//! - Block flags (60000-99999): Block base + relative offset
//! - Dungeon flags (10000000-43999999): Map base + local offset
//! - Open world (1000000000+): Formula-based tile calculation
//!
//! V4: Added midrange flag support (540xxx sorceries/incantations)
//! Uses verified ground truth offsets from ground_truth_offsets.json
//! Generated at build time via build.rs

use std::collections::HashMap;
use once_cell::sync::Lazy;
use wasm_event_flags::{FlagState, ResolvedFlags};

// Import verified constants from generated module
use crate::generated::ground_truth::{
    VERIFIED_TILE_BASE_OFFSET,
    TILE_BYTES_PER_SLOT as GT_TILE_BYTES_PER_SLOT,
    TILE_SLOTS_PER_ROW as GT_TILE_SLOTS_PER_ROW,
    TILE_ROW_BASE as GT_TILE_ROW_BASE,
    TILE_COL_BASE as GT_TILE_COL_BASE,
    TILE_MAX_LOCAL_ID,
    VERIFIED_BLOCK_BASES,
    VERIFIED_DUNGEON_BASES,
    VERIFIED_MIDRANGE_BASES,
};

// ============================================================================
// CONSTANTS (from verified ground_truth_offsets.json)
// ============================================================================

/// Base offset where tile flags start in event flags array (VERIFIED)
pub const TILE_BASE_OFFSET: u32 = VERIFIED_TILE_BASE_OFFSET;

/// First row (X coordinate) of world tiles
pub const TILE_ROW_BASE: u32 = GT_TILE_ROW_BASE;

/// First column (Y coordinate) of world tiles
pub const TILE_COL_BASE: u32 = GT_TILE_COL_BASE;

/// Bytes allocated per tile slot (875 bytes = 7000 flags max)
pub const TILE_BYTES_PER_SLOT: u32 = GT_TILE_BYTES_PER_SLOT;

/// Number of tile slots per row
pub const TILE_SLOTS_PER_ROW: u32 = GT_TILE_SLOTS_PER_ROW;

/// Total size of event flags section
pub const EVENT_FLAGS_SIZE: u32 = 0x1BF99F; // 1,833,375 bytes

/// Bytes per dungeon section
pub const DUNGEON_SECTION_SIZE: u32 = 1125;

/// Maximum local ID for tile flags (7000+ use row_id formula instead)
pub const MAX_TILE_LOCAL_ID: u32 = TILE_MAX_LOCAL_ID;

/// Base offset for world pickup row_id-based tracking
/// DISCOVERY (2026-02-02): World pickups with getItemFlagId local_id >= 7000
/// are tracked by row_id in a separate bitfield region starting at this offset.
pub const WORLD_PICKUP_ROW_ID_BASE: u32 = 1037373320;

/// Convert getItemFlagId to storable row_id for tile-based world pickups.
///
/// DISCOVERY (2026-01-23): For tile-based world pickups:
/// - ItemLotParam has getItemFlagId = row_id + 7000
/// - The game stores row_id (storable, localId 0-999), NOT getItemFlagId (unstorable, localId 7000+)
/// - MapGenie mappings use getItemFlagId, but we need row_id to check the save
///
/// # Arguments
///
/// * `flag_id` - The event flag ID (potentially getItemFlagId with localId >= 7000)
///
/// # Returns
///
/// The row_id (with localId 0-999) if conversion applies, None otherwise
pub fn convert_to_row_id(flag_id: u32) -> Option<u32> {
    // Only applies to 10-digit tile flags
    if !(1_000_000_000..2_000_000_000).contains(&flag_id) {
        return None;
    }

    let local_id = flag_id % 10000;
    // If localId >= 7000, this is getItemFlagId - convert to row_id
    if local_id >= 7000 {
        Some(flag_id - 7000) // row_id has localId in 0-999 range
    } else {
        None // Already a valid flag (localId < 7000)
    }
}

// ============================================================================
// DUNGEON BASE OFFSETS (complete mapping from elden-map)
// ============================================================================

/// Dungeon base offsets for save file event flags (VERIFIED ONLY)
///
/// IMPORTANT: Only include areas that have been EMPIRICALLY VERIFIED against actual save files.
/// Unverified areas cause false positives in the UI. Use VERIFIED_DUNGEON_BASES (from
/// ground_truth_offsets.json) for areas with verified formulas.
///
/// CORRECTED (2026-01-09): Empirical verification against actual save files showed
/// the previous offsets (1,383,375+) were from runtime memory, not save file format.
///
/// Key format: "XX_YY" where XX is map area, YY is section
/// Formula: offset = 4112 + slot_index * 1125 (from legacymap.eventflagalloclist)
///
/// VERIFIED areas (matched empirical save file data):
/// - Area 10: Stormveil Castle - Godskin Prayerbook, Fire Grease, Arbalest all verified
/// - Area 14: Tutorial Areas - 1968/1968 flags match (also verified slot formula)
/// - Area 18: Roundtable Hold - 176/176 flags match (also verified slot formula)
/// - Areas 30, 31, 32: Use VERIFIED_DUNGEON_BASES (different values than slot formula!)
pub static DUNGEON_BASE_OFFSETS: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    HashMap::from([
        // Stormveil Castle (m10) - Slot 0, 1 - VERIFIED
        ("10_00", 4112), ("10_01", 5237),
        // Tutorial Areas (m14) - Slot 23 - VERIFIED (1968/1968 flags match)
        ("14_00", 29987),
        // Roundtable Hold (m18) - Slot 35 - VERIFIED (176/176 flags match)
        ("18_00", 43487),
        // NOTE: Areas 30, 31, 32 (catacombs, caves, tunnels) are handled by
        // VERIFIED_DUNGEON_BASES with empirically-discovered bases (27411, 28634, 31577)
        // which differ from the slot formula. Do NOT add them here.
    ])
});

/// Per-section pickup bases (for local_id >= 7000)
///
/// DISCOVERY (2026-02-02): The linear formula `base + section * 1125` is WRONG.
/// Each (area, section) combination has its own empirically-discovered base offset.
/// This was discovered by brute-force searching save files with known collected items.
///
/// Formula: offset = section_base + local_id/8, bit = 7 - (local_id % 8)
/// Note: local_id is the full flag % 10000, so for pickup flags it's 7000-9999
///
/// All 77 entries verified with 100% match rates across multiple save slots.
/// Originally generated by the removed Python lab (scripts/build_pickup_section_map.py; see docs/archive/PYTHON-LAB.md).
pub static DUNGEON_PICKUP_SECTION_BASES: Lazy<HashMap<(u32, u32), u32>> = Lazy::new(|| {
    HashMap::from([
        // Area 10: Stormveil Castle
        ((10,  0), 31904),
        ((10,  1),  1787),
        // Area 11: Leyndell
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
        // Area 31: Caves (19 sections, note: section 8 missing from save data)
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
});

/// Legacy area-based pickup bases (DEPRECATED - use DUNGEON_PICKUP_SECTION_BASES)
/// Kept for reference only - the per-section lookup is more accurate
pub static DUNGEON_PICKUP_BASES: Lazy<HashMap<u32, u32>> = Lazy::new(|| {
    HashMap::from([
        (10, 31904),  // Stormveil Castle (section 0)
        (11, 31903),  // Leyndell (section 0)
        (12, 31900),  // Underground (varies by section)
        (13, 31903),  // Crumbling Farum Azula
        (14, 31903),  // Academy of Raya Lucaria
        (15, 31903),  // Miquella's Haligtree
        (16, 31903),  // Volcano Manor
        (18,  3847),  // Roundtable Hold
        (20, 31903),  // Stranded Graveyard
        (21, 31903),  // Haligtree (Elphael)
        (22, 28962),  // Castle Sol
        (28, 28974),  // DLC
        (35, 31903),  // Mohgwyn Palace
    ])
});

// Block bases for flags 60000-99999 (special system flags)
// Now uses VERIFIED_BLOCK_BASES from ground_truth_offsets.json
// The old hardcoded values were incorrect (e.g., 67000 was 2125, verified is 3546)

// ============================================================================
// FLAG OFFSET CALCULATIONS
// ============================================================================

/// Calculate byte offset and bit position for a tile (open world) flag
///
/// Flag format: 1XXYYZZZZ where:
/// - XX: tile row (33-60+)
/// - YY: tile column (30-58+)
/// - ZZZZ: local ID (0-6999 use tile formula, 7000+ use row_id formula)
///
/// Uses VERIFIED_TILE_BASE_OFFSET (337375, corrected 2026-02-15)
/// For local_id >= 7000, delegates to row_id-based formula.
fn calculate_tile_flag_offset(flag_id: u32) -> Option<(u32, u8)> {
    let local_id = flag_id % 10000;

    // LocalId >= 7000 uses row_id-based formula (not tile storage)
    // DISCOVERY (2026-02-02): These are tracked by row_id in a separate bitfield
    if local_id > MAX_TILE_LOCAL_ID {
        let row_id = flag_id - 7000;
        return calculate_world_pickup_offset_by_row_id(row_id);
    }

    let bit = (7 - (flag_id % 8)) as u8;
    let tile_index = (flag_id - 1_000_000_000) / 10000;

    let row = tile_index / 100;
    let col = tile_index % 100;

    // Calculate slot position in the tile array
    let slot = (row as i32 - TILE_ROW_BASE as i32) * TILE_SLOTS_PER_ROW as i32
             + (col as i32 - TILE_COL_BASE as i32);

    if slot < 0 {
        return None;
    }

    let base_offset = TILE_BASE_OFFSET + (slot as u32) * TILE_BYTES_PER_SLOT;
    let byte_offset = base_offset + local_id / 8;

    if byte_offset >= EVENT_FLAGS_SIZE {
        return None;
    }

    Some((byte_offset, bit))
}

/// Calculate byte offset and bit position for a dungeon flag
///
/// Flag format: AASSZZZZ where:
/// - AA: map area (10-43)
/// - SS: section (00-22)
/// - ZZZZ: local offset (0-9999)
///
/// Uses VERIFIED_DUNGEON_BASES for areas 30, 31, 32 (catacombs, caves, tunnels)
/// Falls back to DUNGEON_BASE_OFFSETS for other areas
fn calculate_dungeon_flag_offset(flag_id: u32) -> Option<(u32, u8)> {
    let bit = (7 - (flag_id % 8)) as u8;

    let area = flag_id / 1_000_000;
    let section = (flag_id / 10_000) % 100;
    let local_id = flag_id % 10_000;

    // For item pickup flags (local_id >= 7000), use per-section lookup
    // DISCOVERY (2026-02-02): Each (area, section) has its own base - no linear formula!
    // The old formula `base + section * 1125` was WRONG and caused false negatives
    if local_id >= 7000 {
        if let Some(&section_base) = DUNGEON_PICKUP_SECTION_BASES.get(&(area, section)) {
            // Formula: offset = section_base + local_id/8
            // Note: local_id is already 7000+, so we add 875+ bytes to the section base
            let byte_offset = section_base + local_id / 8;
            if byte_offset < EVENT_FLAGS_SIZE {
                return Some((byte_offset, bit));
            }
        }
        // Section base not available - return None to avoid false positives
        // Unverified sections will show as "unknown" in UI rather than incorrect
        return None;
    }

    // For general dungeon events (local_id < 7000), check verified dungeon bases
    // Areas 30, 31, 32 (catacombs, caves, tunnels) are verified
    if let Some(dungeon_base) = VERIFIED_DUNGEON_BASES.get(&area) {
        if dungeon_base.status == "verified" && dungeon_base.base_offset > 0 {
            let byte_offset = dungeon_base.base_offset + section * dungeon_base.section_size + local_id / 8;
            if byte_offset < EVENT_FLAGS_SIZE {
                return Some((byte_offset, bit));
            }
        }
    }

    // Fall back to detailed DUNGEON_BASE_OFFSETS mapping
    let flag_str = format!("{:08}", flag_id);
    let dungeon_area = &flag_str[0..2];
    let section_str = &flag_str[2..4];

    let key = format!("{}_{}", dungeon_area, section_str);

    // Try to get exact base offset
    let base_offset = if let Some(&base) = DUNGEON_BASE_OFFSETS.get(key.as_str()) {
        base
    } else {
        // Fall back to calculating from base section
        let base_key = format!("{}_00", dungeon_area);
        let section_num: u32 = section_str.parse().ok()?;
        let section_00_base = DUNGEON_BASE_OFFSETS.get(base_key.as_str())?;
        section_00_base + section_num * DUNGEON_SECTION_SIZE
    };

    let byte_offset = base_offset + local_id / 8;

    if byte_offset >= EVENT_FLAGS_SIZE {
        return None;
    }

    Some((byte_offset, bit))
}

/// Calculate byte offset and bit position for a simple flag (< 100000)
/// Uses VERIFIED_BLOCK_BASES from ground_truth_offsets.json for block flags
fn calculate_simple_flag_offset(flag_id: u32) -> Option<(u32, u8)> {
    let bit = (7 - (flag_id % 8)) as u8;

    // Small flags use direct calculation with base offset adjustment
    if flag_id < 60000 {
        let byte_offset = flag_id / 8;
        if byte_offset >= EVENT_FLAGS_SIZE {
            return None;
        }
        return Some((byte_offset, bit));
    }

    // Block flags (60000-99999) - use verified block bases
    // First try sub-block at 100-flag granularity (e.g., 71600, 71800)
    let sub_block_start = (flag_id / 100) * 100;
    if let Some(block_base) = VERIFIED_BLOCK_BASES.get(&sub_block_start) {
        let relative = flag_id - sub_block_start;
        let byte_offset = block_base.base_offset + relative / 8;
        if byte_offset < EVENT_FLAGS_SIZE {
            return Some((byte_offset, bit));
        }
    }

    // Fall back to main block at 1000-flag granularity (e.g., 71000)
    let block_start = (flag_id / 1000) * 1000;
    if let Some(block_base) = VERIFIED_BLOCK_BASES.get(&block_start) {
        let relative = flag_id - block_start;
        let byte_offset = block_base.base_offset + relative / 8;
        if byte_offset < EVENT_FLAGS_SIZE {
            return Some((byte_offset, bit));
        }
    }

    None
}

/// Calculate byte offset and bit position for a midrange flag (100000-999999)
///
/// Flag format: 6 digits for sorceries, incantations, ashes of war unlock
/// Uses VERIFIED_MIDRANGE_BASES from ground_truth_offsets.json
fn calculate_midrange_flag_offset(flag_id: u32) -> Option<(u32, u8)> {
    let bit = (7 - (flag_id % 8)) as u8;

    // Try 10000-flag block granularity first (e.g., 540000 for all 54xxxx flags)
    let block_10k = (flag_id / 10000) * 10000;
    if let Some(midrange_base) = VERIFIED_MIDRANGE_BASES.get(&block_10k) {
        if midrange_base.status == "verified" {
            let relative = flag_id - block_10k;
            let byte_offset = midrange_base.base_offset + relative / 8;
            if byte_offset < EVENT_FLAGS_SIZE {
                return Some((byte_offset, bit));
            }
        }
    }

    // Fall back to 1000-flag granularity
    let block_1k = (flag_id / 1000) * 1000;
    if let Some(midrange_base) = VERIFIED_MIDRANGE_BASES.get(&block_1k) {
        let relative = flag_id - block_1k;
        let byte_offset = midrange_base.base_offset + relative / 8;
        if byte_offset < EVENT_FLAGS_SIZE {
            return Some((byte_offset, bit));
        }
    }

    None
}

/// Calculate byte offset and bit position for an event flag
///
/// Returns Some((byte_offset, bit_position)) if the flag can be calculated
/// Returns None if the flag type is unknown
///
/// NOTE: For tile flags, this uses the static TILE_BASE_OFFSET which may not be
/// accurate for all saves. Use `get_flag_offset_calibrated` with a calibrated
/// tile base for more accurate results.
pub fn get_flag_offset(flag_id: u32) -> Option<(u32, u8)> {
    // 10-digit open world tile flags (1000000000+)
    if flag_id >= 1_000_000_000 {
        return calculate_tile_flag_offset(flag_id);
    }

    // 8-digit dungeon flags (10000000-43999999)
    if (10_000_000..44_000_000).contains(&flag_id) {
        return calculate_dungeon_flag_offset(flag_id);
    }

    // 6-digit midrange flags (100000-999999) - sorceries, incantations, etc.
    if (100_000..1_000_000).contains(&flag_id) {
        return calculate_midrange_flag_offset(flag_id);
    }

    // Simple flags (< 100000)
    if flag_id < 100_000 {
        return calculate_simple_flag_offset(flag_id);
    }

    // Flags 1000000-9999999 are not commonly used
    None
}

/// Calculate byte offset and bit position for an event flag using a calibrated tile base.
///
/// This is the preferred method for tile flags when you have calibrated the tile base
/// for the specific save file. The tile base varies per-save due to variable GaItems
/// (inventory) section sizes.
///
/// # Arguments
///
/// * `flag_id` - The event flag ID
/// * `calibrated_tile_base` - The calibrated tile base offset from CalibrationService
///
/// Returns Some((byte_offset, bit_position)) if the flag can be calculated
/// Returns None if the flag type is unknown
pub fn get_flag_offset_calibrated(flag_id: u32, calibrated_tile_base: u32) -> Option<(u32, u8)> {
    // 10-digit open world tile flags (1000000000+) - use calibrated base
    if flag_id >= 1_000_000_000 {
        return calculate_tile_flag_offset_with_base(flag_id, calibrated_tile_base);
    }

    // Other flag types don't need calibration - delegate to standard function
    get_flag_offset(flag_id)
}

/// Calculate tile flag offset using a specific base offset.
///
/// This allows using a calibrated base instead of the static TILE_BASE_OFFSET.
/// For flags with local_id >= 7000, uses the row_id-based formula instead.
fn calculate_tile_flag_offset_with_base(flag_id: u32, base_offset: u32) -> Option<(u32, u8)> {
    let local_id = flag_id % 10000;

    // LocalId >= 7000 uses row_id-based formula (not tile storage)
    // DISCOVERY (2026-02-02): These are tracked by row_id in a separate bitfield
    if local_id > MAX_TILE_LOCAL_ID {
        // Convert getItemFlagId to row_id and use row_id formula
        let row_id = flag_id - 7000;
        return calculate_world_pickup_offset_by_row_id(row_id);
    }

    let bit = (7 - (flag_id % 8)) as u8;
    let tile_index = (flag_id - 1_000_000_000) / 10000;

    let row = tile_index / 100;
    let col = tile_index % 100;

    // Calculate slot position in the tile array
    let slot = (row as i32 - TILE_ROW_BASE as i32) * TILE_SLOTS_PER_ROW as i32
             + (col as i32 - TILE_COL_BASE as i32);

    if slot < 0 {
        return None;
    }

    let slot_offset = base_offset + (slot as u32) * TILE_BYTES_PER_SLOT;
    let byte_offset = slot_offset + local_id / 8;

    if byte_offset >= EVENT_FLAGS_SIZE {
        return None;
    }

    Some((byte_offset, bit))
}

/// Calculate offset for world pickup tracking using ItemLotParam row_id.
///
/// DISCOVERY (2026-02-02): World pickups with getItemFlagId (local_id >= 7000)
/// are NOT stored in the tile region. Instead, they're tracked by row_id
/// in a separate bitfield region.
///
/// Formula:
/// - byte_offset = (row_id - WORLD_PICKUP_ROW_ID_BASE) / 8
/// - bit_position = 7 - ((row_id - WORLD_PICKUP_ROW_ID_BASE) % 8)
///
/// Verified via before/after save captures of Golden Rune [1] and [3] pickups.
pub fn calculate_world_pickup_offset_by_row_id(row_id: u32) -> Option<(u32, u8)> {
    // Only valid for 10-digit row_ids in the 1B range
    if !(1_000_000_000..2_000_000_000).contains(&row_id) {
        return None;
    }

    // row_id should have local_id < 7000 (it's the raw ItemLotParam row_id)
    let local_id = row_id % 10000;
    if local_id >= 7000 {
        return None;
    }

    // Must be >= base to have valid storage
    if row_id < WORLD_PICKUP_ROW_ID_BASE {
        return None;
    }

    let bit_offset = row_id - WORLD_PICKUP_ROW_ID_BASE;
    let byte_offset = bit_offset / 8;
    let bit_position = (7 - (bit_offset % 8)) as u8;

    if byte_offset >= EVENT_FLAGS_SIZE {
        return None;
    }

    Some((byte_offset, bit_position))
}

/// Verification status for flag offset calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    /// Formula has been empirically verified against save files
    Verified,
    /// Formula is calculated/extrapolated but not directly verified
    Calculated,
    /// Formula base is unverified - results may be inaccurate
    Unverified,
    /// Flag type is unknown or unsupported
    Unknown,
}

/// Check if a block-based flag (5-digit) is from a reliable/verified block
///
/// Returns true if the block has status "verified", false for "unreliable",
/// "needs_investigation", "disproven", or unknown blocks.
///
/// Use this to filter out flags that may produce false positives on some saves.
pub fn is_block_reliable(flag_id: u32) -> bool {
    // Only applies to 5-digit block flags (60000-99999)
    if !(60000..100000).contains(&flag_id) {
        return true; // Non-block flags use different formulas
    }

    // Check sub-block first (100-flag granularity, e.g., 71600)
    let sub_block = (flag_id / 100) * 100;
    if let Some(base) = VERIFIED_BLOCK_BASES.get(&sub_block) {
        return base.status == "verified";
    }

    // Check main block (1000-flag granularity, e.g., 71000)
    let block = (flag_id / 1000) * 1000;
    if let Some(base) = VERIFIED_BLOCK_BASES.get(&block) {
        return base.status == "verified";
    }

    // Unknown block - treat as unreliable
    false
}

impl VerificationStatus {
    /// Returns true if this status indicates potential inaccuracy
    pub fn is_uncertain(&self) -> bool {
        matches!(self, VerificationStatus::Unverified | VerificationStatus::Unknown)
    }
}

/// Get the verification status for a flag's offset calculation
pub fn get_flag_verification_status(flag_id: u32) -> VerificationStatus {
    // 10-digit tile flags - tile formula is verified
    if flag_id >= 1_000_000_000 {
        return VerificationStatus::Verified;
    }

    // 8-digit dungeon flags
    if (10_000_000..44_000_000).contains(&flag_id) {
        let area = flag_id / 1_000_000;

        // Check VERIFIED_DUNGEON_BASES first
        if let Some(dungeon_base) = VERIFIED_DUNGEON_BASES.get(&area) {
            return match dungeon_base.status {
                "verified" => VerificationStatus::Verified,
                "calculated" => VerificationStatus::Calculated,
                "needs_review" => VerificationStatus::Unverified,
                _ => VerificationStatus::Unverified,
            };
        }

        // Fall back to DUNGEON_BASE_OFFSETS - these are from eventflagalloclist
        let section = (flag_id / 10_000) % 100;
        let key = format!("{}_{:02}", area, section);
        if DUNGEON_BASE_OFFSETS.contains_key(key.as_str()) {
            return VerificationStatus::Calculated; // From game files but not empirically verified
        }

        return VerificationStatus::Unknown;
    }

    // Simple flags (< 60000) - direct calculation is verified
    if flag_id < 60000 {
        return VerificationStatus::Verified;
    }

    // Block flags (60000-99999) - check VERIFIED_BLOCK_BASES
    if flag_id < 100_000 {
        let block_start = (flag_id / 1000) * 1000;
        if let Some(block_base) = VERIFIED_BLOCK_BASES.get(&block_start) {
            return match block_base.status {
                "verified" => VerificationStatus::Verified,
                "calculated" => VerificationStatus::Calculated,
                _ => VerificationStatus::Unverified,
            };
        }
        return VerificationStatus::Unknown;
    }

    // Midrange flags (100000-999999) - sorceries, incantations, etc.
    if (100_000..1_000_000).contains(&flag_id) {
        // Check 10000-flag granularity
        let block_10k = (flag_id / 10000) * 10000;
        if let Some(midrange_base) = VERIFIED_MIDRANGE_BASES.get(&block_10k) {
            return match midrange_base.status {
                "verified" => VerificationStatus::Verified,
                "calculated" => VerificationStatus::Calculated,
                _ => VerificationStatus::Unverified,
            };
        }
        // Check 1000-flag granularity
        let block_1k = (flag_id / 1000) * 1000;
        if let Some(midrange_base) = VERIFIED_MIDRANGE_BASES.get(&block_1k) {
            return match midrange_base.status {
                "verified" => VerificationStatus::Verified,
                "calculated" => VerificationStatus::Calculated,
                _ => VerificationStatus::Unverified,
            };
        }
        return VerificationStatus::Unknown;
    }

    VerificationStatus::Unknown
}

/// Check if an event flag is set in the save file's EventFlags section
pub fn is_flag_set(event_flags: &[u8], flag_id: u32) -> bool {
    if let Some((byte_off, bit)) = get_flag_offset(flag_id) {
        if (byte_off as usize) < event_flags.len() {
            return (event_flags[byte_off as usize] & (1 << bit)) != 0;
        }
    }
    false
}

/// Check if a tile pickup flag is set, using row_id conversion if needed.
///
/// DISCOVERY (2026-01-23): For tile-based world pickups with localId >= 7000 (getItemFlagId),
/// the game stores row_id (localId 0-999), NOT getItemFlagId (localId 7000+).
/// This function handles the conversion automatically.
///
/// # Arguments
///
/// * `event_flags` - The event flags byte slice from the save
/// * `flag_id` - The event flag ID (may be getItemFlagId with localId >= 7000)
/// * `calibrated_tile_base` - The calibrated tile base offset (use CalibrationService)
///
/// # Returns
///
/// true if the flag is set, false otherwise
pub fn is_tile_pickup_flag_set(event_flags: &[u8], flag_id: u32, calibrated_tile_base: u32) -> bool {
    // Check if this is a getItemFlagId that needs conversion
    if let Some(row_id) = convert_to_row_id(flag_id) {
        // Use row_id - this is what the game actually stores
        if let Some((byte_off, bit)) = get_flag_offset_calibrated(row_id, calibrated_tile_base) {
            if (byte_off as usize) < event_flags.len() {
                return (event_flags[byte_off as usize] & (1 << bit)) != 0;
            }
        }
        return false;
    }

    // Not a getItemFlagId - use normal check with calibrated base
    if let Some((byte_off, bit)) = get_flag_offset_calibrated(flag_id, calibrated_tile_base) {
        if (byte_off as usize) < event_flags.len() {
            return (event_flags[byte_off as usize] & (1 << bit)) != 0;
        }
    }
    false
}

/// State of a flag from a PICKUP table, resolved per save.
///
/// CUT OVER 2026-07-20 (ADR-0006, migration step 4). This is what
/// `is_flag_set_with_status` could not be: that function takes a bare id and so
/// cannot know which family it belongs to. The missing information was never in
/// the id — it is in the CALLER. An entry in `WORLD_PICKUPS` or
/// `DUNGEON_PICKUPS` is known to be a pickup, which resolves the one ambiguity
/// that blocked the cutover (a 10-digit id being either an open-world event flag
/// or a pickup row_id). So the family follows from the id's SHAPE once
/// "this is a pickup" is given:
///
///   10-digit, 1xxxxxxxxx   open-world tile pickup, by row_id / getItemFlagId
///    8-digit, local >= 7000  legacy-map pickup (alloc-slot layout)
///    5-digit, 50000-79999    world-state-b
///
/// `FlagState::Unknown` is never "not collected". It covers DLC tiles
/// (`2xxxxxxxxx`, no verified layout), the ~935 six-digit ids in `WORLD_PICKUPS`
/// that belong to no known family, doubly-allocated maps, and any save whose
/// origin will not resolve (in which case the caller holds no `ResolvedFlags` at
/// all and reads `Unknown` for everything).
///
/// Takes an already-resolved `ResolvedFlags` rather than raw bytes on purpose:
/// the origin is a ~13,400-byte scan, and a table view calling this per row must
/// pay it once, not once per pickup. Build one `ResolvedFlags` above the loop.
///
/// The id→family routing stays HERE rather than in the wasm crate: `WORLD_PICKUPS`
/// is NOT a single-family table despite its name (of 4,809 entries: 1,232
/// open-world tiles, 2,010 legacy-map, 100 world-state-b, 935 unclassified, 532
/// DLC), so which families it mixes is knowledge about THIS table, not about the
/// save format. Routing the whole table through the tile reader — which is what
/// shipped in v0.28.0 — left 3,577 of them reading Unknown.
pub fn pickup_state(flags: &ResolvedFlags, flag_id: u32) -> FlagState {
    match flag_id {
        1_000_000_000..=1_999_999_999 => flags.tile_pickup(flag_id),
        10_000_000..=999_999_999 => flags.dungeon_pickup(flag_id),
        50_000..=79_999 => flags.world_state(flag_id),
        _ => FlagState::Unknown,
    }
}

/// Route a WORLD-state flag (not a pickup) to its Flag Family and read it from an
/// already-resolved region. The sibling of `pickup_state`: same id→family ranges,
/// but the world-semantics readers — `tile_world`/`dungeon`, not the `_pickup`
/// variants — because the callers (event-flag tables: bosses, maps, summoning
/// pools, colosseums, landmarks, cookbooks, whetblades) mean "boss defeated /
/// grace lit / area discovered", never "item picked up".
///
/// The tile/pickup ambiguity CLAUDE.md warns about (a bare 10-digit id fits both
/// families, 500 bytes apart) is resolved HERE by the caller's semantics: these
/// tables are world flags, so a 10-digit id is `tile_world`. Like `pickup_state`
/// this holds no base table — it delegates to the per-save `ResolvedFlags`, so it
/// does not reintroduce the frozen `flag_id → offset` model (ADR-0008).
pub fn world_flag_state(flags: &ResolvedFlags, flag_id: u32) -> FlagState {
    match flag_id {
        1_000_000_000..=1_999_999_999 => flags.tile_world(flag_id),
        10_000_000..=999_999_999 => flags.dungeon(flag_id),
        50_000..=79_999 => flags.world_state(flag_id),
        _ => FlagState::Unknown,
    }
}

/// Check if an event flag is set and return verification status
/// Returns (is_set, verification_status)
///
/// DEPRECATED for pickup tables — use `pickup_flag_state`, which resolves
/// positions per save. This remains only for callers that have neither a family
/// nor a resolved origin, and it reads absolute offsets from the frozen legacy
/// store, so a save whose families have drifted reads every flag as clear.
pub fn is_flag_set_with_status(event_flags: &[u8], flag_id: u32) -> (bool, VerificationStatus) {
    // NOT cut over to the resolver, deliberately. A bare 10-digit id is
    // ambiguous between the open-world and pickup tile families (both use
    // localId < 7000, in regions 500 bytes apart), so this function cannot pick
    // the right one from the value. Callers that know the semantics use
    // wasm_event_flags::ResolvedFlags::tile_pickup / tile_world directly.
    let status = get_flag_verification_status(flag_id);
    let is_set = if let Some((byte_off, bit)) = get_flag_offset(flag_id) {
        if (byte_off as usize) < event_flags.len() {
            (event_flags[byte_off as usize] & (1 << bit)) != 0
        } else {
            false
        }
    } else {
        false
    };
    (is_set, status)
}

/// Set an event flag in the save file's EventFlags section
pub fn set_flag(event_flags: &mut [u8], flag_id: u32, value: bool) -> bool {
    if let Some((byte_off, bit)) = get_flag_offset(flag_id) {
        if (byte_off as usize) < event_flags.len() {
            if value {
                event_flags[byte_off as usize] |= 1 << bit;
            } else {
                event_flags[byte_off as usize] &= !(1 << bit);
            }
            return true;
        }
    }
    false
}

// ============================================================================
// ITEM CATEGORIES AND CONSTANTS
// ============================================================================

/// Item categories from ItemLotParam
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ItemCategory {
    None = 0,
    Goods = 1,      // Consumables, materials, runes
    Weapon = 2,
    Armor = 3,
    Accessory = 4,  // Talismans
    AshOfWar = 5,
    Other = 6,
}

impl From<u32> for ItemCategory {
    fn from(v: u32) -> Self {
        match v {
            1 => ItemCategory::Goods,
            2 => ItemCategory::Weapon,
            3 => ItemCategory::Armor,
            4 => ItemCategory::Accessory,
            5 => ItemCategory::AshOfWar,
            6 => ItemCategory::Other,
            _ => ItemCategory::None,
        }
    }
}

/// Important item IDs for common collectibles
pub mod item_ids {
    // Golden Runes
    pub const GOLDEN_RUNE_1: u32 = 2900;
    pub const GOLDEN_RUNE_13: u32 = 2912;
    pub const HEROS_RUNE_1: u32 = 2914;
    pub const HEROS_RUNE_5: u32 = 2918;
    pub const LORDS_RUNE: u32 = 2919;

    // Smithing Stones
    pub const SMITHING_STONE_1: u32 = 10100;
    pub const SMITHING_STONE_8: u32 = 10107;
    pub const ANCIENT_DRAGON_SMITHING_STONE: u32 = 10140;

    // Somber Smithing Stones
    pub const SOMBER_SMITHING_STONE_1: u32 = 10160;
    pub const SOMBER_SMITHING_STONE_9: u32 = 10168;
    pub const SOMBER_ANCIENT_DRAGON: u32 = 10200;

    // Glovewort
    pub const GRAVE_GLOVEWORT_1: u32 = 10900;
    pub const GRAVE_GLOVEWORT_9: u32 = 10908;
    pub const GREAT_GRAVE_GLOVEWORT: u32 = 10909;
    pub const GHOST_GLOVEWORT_1: u32 = 10910;
    pub const GHOST_GLOVEWORT_9: u32 = 10918;
    pub const GREAT_GHOST_GLOVEWORT: u32 = 10919;

    // Special items
    pub const RUNE_ARC: u32 = 190;
    pub const STONESWORD_KEY: u32 = 10030;
    pub const MEMORY_OF_GRACE: u32 = 10040;
    pub const LARVAL_TEAR: u32 = 10060;
    pub const CELESTIAL_DEW: u32 = 10070;
    pub const STARLIGHT_SHARD: u32 = 10080;
}

/// Region names for open world tiles
pub fn get_region_name(tile_x: u32, tile_y: u32) -> &'static str {
    match (tile_x, tile_y) {
        // Limgrave - Stormhill checked first (more specific), then Weeping Peninsula, then general Limgrave
        (44..=45, 32..=35) => "Stormhill",
        (43, 30..=35) => "Weeping Peninsula",
        (41..=44, 36..=39) => "Limgrave",

        // Liurnia
        (33..=40, 40..=50) => "Liurnia of the Lakes",

        // Caelid
        (45..=52, 36..=43) => "Caelid",

        // Mt. Gelmir - checked before Altus to avoid overlap at 38
        (33..=37, 49..=55) => "Mt. Gelmir",

        // Altus Plateau
        (38..=44, 49..=55) => "Altus Plateau",

        // Consecrated Snowfield - checked before Mountaintops (more northern)
        (47..=54, 56..=58) => "Consecrated Snowfield",

        // Mountaintops of the Giants
        (47..=54, 54..=55) => "Mountaintops of the Giants",

        // DLC (Shadow of the Erdtree - m61)
        (60, 33..=44) => "Shadow of the Erdtree",

        _ => "Unknown",
    }
}

/// Get dungeon name from map ID
pub fn get_dungeon_name(map_area: u32, section: u32) -> &'static str {
    match (map_area, section) {
        (10, _) => "Stormveil Castle",
        (11, 0..=4) => "Leyndell, Royal Capital",
        (11, 5..=9) => "Leyndell, Ashen Capital",
        (11, 71) => "Fortified Manor",
        (12, 1) => "Ainsel River",
        (12, 2) => "Siofra River",
        (12, 3) => "Deeproot Depths",
        (12, 4) => "Lake of Rot",
        (12, 5) => "Nokstella",
        (12, 7) => "Nokron, Eternal City",
        (13, _) => "Crumbling Farum Azula",
        (14, _) => "Academy of Raya Lucaria",
        (15, _) => "Caria Manor",
        (16, _) => "Volcano Manor",
        (18, _) => "Roundtable Hold",
        (19, _) => "Chapel of Anticipation",
        (20, _) => "Stranded Graveyard",
        (21, _) => "Miquella's Haligtree",
        (22, _) => "Castle Sol",
        (30, _) => "Catacombs",
        (31, _) => "Cave",
        (32, _) => "Tunnel",
        (34, _) => "Divine Tower",
        (35, _) => "Mohgwyn Palace",
        (39, _) => "Elden Throne",
        (40, _) => "Hero's Grave",
        (41, _) => "Minor Dungeon",
        (42, _) => "Crystal Cave",
        (43, _) => "Evergaol",
        _ => "Unknown Dungeon",
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// `world_flag_state` routes by id range to the world-semantics families and
    /// refuses (Unknown) outside them — never collapsing an unrouted id to Clear.
    /// Guards the router the event-flag reader cluster and the remembrance/great-rune
    /// verification both depend on (v0.37.19-20).
    #[test]
    fn world_flag_state_routes_and_refuses() {
        use wasm_event_flags::{resolve_family_base_in_ef, FAMILY_WORLD_STATE_B};

        // Out-of-family ids refuse, never Clear.
        let mut ef = vec![0u8; 2_100_000];
        ef[20_000] = 0x01; // marker → list end in detectable range → region resolves
        let flags = ResolvedFlags::from_event_flags(&ef).unwrap();
        assert_eq!(world_flag_state(&flags, 0), FlagState::Unknown);
        assert_eq!(world_flag_state(&flags, 49_999), FlagState::Unknown);
        assert_eq!(world_flag_state(&flags, 2_000_000_001), FlagState::Unknown);

        // A world-state block flag set at its resolved position reads Set through the
        // router; a sibling left clear reads a definite Clear, not Unknown.
        let base = resolve_family_base_in_ef(&ef, FAMILY_WORLD_STATE_B).unwrap();
        let flag = 65_610u32;
        ef[base + ((flag - 50_000) / 8) as usize] |= 1 << (7 - (flag % 8) as u8);
        let flags = ResolvedFlags::from_event_flags(&ef).unwrap();
        assert_eq!(world_flag_state(&flags, flag), FlagState::Set);
        assert_eq!(world_flag_state(&flags, 65_700), FlagState::Clear);
    }

    #[test]
    fn test_small_flag_offset() {
        // Flag 300 should be at byte 37, bit 3
        assert_eq!(get_flag_offset(300), Some((37, 3)));

        // Flag 6080 should be at byte 760, bit 7
        assert_eq!(get_flag_offset(6080), Some((760, 7)));
    }

    #[test]
    fn test_tile_flag_formula_verified() {
        // Test Limgrave tile 42_37
        // tile_index = (1042370000 - 1_000_000_000) / 10000 = 4237
        // row = 4237 / 100 = 42, col = 4237 % 100 = 37
        // Slot = (42-33)*40 + (37-30) = 9*40 + 7 = 367
        // Base = 337375 + 367*875 = 337375 + 321125 = 658500
        let result = get_flag_offset(1042370000);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 658500);
        assert_eq!(bit, 7);
    }

    #[test]
    fn test_tile_confirmed_empirical() {
        // Test Smoldering Butterfly pickup
        // Empirically verified via before/after save captures (09/10, 135-149)
        // Flag 1043500010: row=43, col=50, local_id=10
        // slot = (43-33)*40 + (50-30) = 420
        // byte_offset = 337375 + 420*875 + 10/8 = 337375 + 367500 + 1 = 704876
        let result = get_flag_offset(1043500010);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 704876, "Empirically confirmed via temporal diff");
        assert_eq!(bit, 5); // 7 - (10 % 8) = 7 - 2 = 5
    }

    #[test]
    fn test_tile_row_id_formula() {
        // UPDATED (2026-02-02): LocalId >= 7000 now uses row_id formula (not "untrackable")
        // Flag 1042377300 has localId 7300, which converts to row_id 1042370300
        // Formula: byte_offset = (row_id - 1037373320) / 8
        //          bit_position = 7 - ((row_id - 1037373320) % 8)
        let result = get_flag_offset(1042377300);
        assert!(result.is_some(), "LocalId 7300 should be trackable via row_id formula");
        let (byte, bit) = result.unwrap();
        // row_id = 1042370300, bit_offset = 1042370300 - 1037373320 = 4996980
        // byte = 4996980 / 8 = 624622, bit = 7 - (4996980 % 8) = 7 - 4 = 3
        assert_eq!(byte, 624622);
        assert_eq!(bit, 3);

        // LocalId 6999 uses tile formula
        let result = get_flag_offset(1042376999);
        assert!(result.is_some(), "LocalId 6999 should be trackable via tile formula");
    }

    #[test]
    fn test_dungeon_flag() {
        // Stormveil Castle flag (m10_00) - PICKUP flag with local_id >= 7000
        // UPDATED (2026-02-02): Pickup flags use DUNGEON_PICKUP_SECTION_BASES
        // Section base for (10, 0) is 31904 (empirically verified)
        // Flag 10007030 -> local 7030, byte offset = 31904 + 7030/8 = 31904 + 878 = 32782
        let result = get_flag_offset(10007030);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 31904 + 7030 / 8);  // 32782
        assert_eq!(bit, (7 - (7030 % 8)) as u8);
    }

    #[test]
    fn test_verified_stormveil_bosses() {
        // Stormveil Castle (area 10) verified via verification-records.jsonl
        // Base for 10_00 is 4112, verified by:
        //   10000800 (Godrick): byte=4212, matches=true
        //   10000850 (Margit): byte=4218, matches=true

        // Flag 10000800 (Godrick): byte=4112+800/8=4212, bit=7-(800%8)=7
        let result = get_flag_offset(10000800);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 4212);
        assert_eq!(bit, 7);

        // Flag 10000850 (Margit): byte=4112+850/8=4218, bit=7-(850%8)=5
        let result = get_flag_offset(10000850);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 4218);
        assert_eq!(bit, 5);
    }

    #[test]
    fn test_verified_dungeon_catacombs() {
        // Test catacombs (area 30) using VERIFIED_DUNGEON_BASES
        // Area 30 has status="verified", base_offset=27411, section_size=1125
        // Verified against 7 boss flags: 30020800=29761, 30030800=30886, etc.

        // Flag 30020800 (Erdtree Burial Watchdog): byte=27411+2*1125+100=29761, bit=7
        let result = get_flag_offset(30020800);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 29761);
        assert_eq!(bit, 7);

        // Flag 30050800 (Cemetery Shade): byte=27411+5*1125+100=33136, bit=7
        let result = get_flag_offset(30050800);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 33136);
        assert_eq!(bit, 7);

        // Flag 30110800 (Black Knife Assassin): byte=27411+11*1125+100=39886, bit=7
        let result = get_flag_offset(30110800);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 39886);
        assert_eq!(bit, 7);
    }

    #[test]
    fn test_verified_dungeon_tunnels() {
        // Test tunnels (area 32) pickup flag using DUNGEON_PICKUP_SECTION_BASES
        // UPDATED (2026-02-02): Pickup flags use per-section bases
        // Section base for (32, 1) is 1835 (empirically verified)
        // Flag 32017000 -> area=32, section=1, local_id=7000
        // byte = 1835 + 7000/8 = 1835 + 875 = 2710
        // bit = 7 - (7000 % 8) = 7 - 0 = 7
        let result = get_flag_offset(32017000);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 2710);  // Verified via per-section discovery
        assert_eq!(bit, 7);
    }

    #[test]
    fn test_verified_dungeon_caves() {
        // Test caves (area 31) using VERIFIED_DUNGEON_BASES
        // Area 31 has status="verified", base_offset=28634, section_size=1125
        // Flag 31112840 -> area=31, section=11, local_id=2840
        // byte = 28634 + 11*1125 + 2840/8 = 28634 + 12375 + 355 = 41364
        // bit = 7 - (2840 % 8) = 7 - 0 = 7
        let result = get_flag_offset(31112840);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 41364);  // Verified via boss matching
        assert_eq!(bit, 7);
    }

    #[test]
    fn test_verified_dungeon_shunning_grounds() {
        // Test Shunning Grounds (area 14) - verified: 1968/1968 flags match
        // Area 14 has base_offset=29987, section_size=1125

        // Flag 14000080: byte=29987+0*1125+80/8=29997, bit=7-(80%8)=7
        let result = get_flag_offset(14000080);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 29997);
        assert_eq!(bit, 7);

        // Flag 14000082: byte=29987+0*1125+82/8=29997, bit=7-(82%8)=5
        let result = get_flag_offset(14000082);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 29997);
        assert_eq!(bit, 5);
    }

    #[test]
    fn test_verified_dungeon_roundtable() {
        // Test Roundtable Hold (area 18) - verified: 176/176 flags match
        // Area 18 has base_offset=43487, section_size=1125

        // Sample flag calculation verification
        // Flag 18000XXX format: byte=43487+section*1125+local/8
        let result = get_flag_offset(18000000);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 43487);  // base + 0/8
        assert_eq!(bit, 7);
    }

    #[test]
    fn test_block_78000_grace_guidance_verified() {
        // Block 78000 (Grace Guidance) verified via 8+ proven flags
        // Block base_offset=3500, verified by matching proven flags

        // Flag 78210 (Bellum Highway guidance): byte=3500+210/8=3526, bit=7-(210%8)=5
        let result = get_flag_offset(78210);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 3526);
        assert_eq!(bit, 5);

        // Flag 78304 (Capital Outskirts guidance): byte=3500+304/8=3538, bit=7-(304%8)=7
        let result = get_flag_offset(78304);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 3538);
        assert_eq!(bit, 7);

        // Flag 78352 (Mt. Gelmir guidance): byte=3500+352/8=3544, bit=7-(352%8)=7
        let result = get_flag_offset(78352);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 3544);
        assert_eq!(bit, 7);
    }

    #[test]
    fn test_block_72000_dlc_graces_verified() {
        // Block 72000 (DLC Enir-Ilim graces) verified via multiple proven flags
        // Block base_offset=2750, verified by matching 10+ consistent proven flags

        // Flag 72000 (Theatre of the Divine Beast): byte=2750, bit=7
        let result = get_flag_offset(72000);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 2750);
        assert_eq!(bit, 7);

        // Flag 72010 (Gate of Divinity): byte=2750+10/8=2751, bit=7-(10%8)=5
        let result = get_flag_offset(72010);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 2751);
        assert_eq!(bit, 5);

        // Flag 72016 (Divine Gate Front Staircase): byte=2750+16/8=2752, bit=7-(16%8)=7
        let result = get_flag_offset(72016);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 2752);
        assert_eq!(bit, 7);
    }

    #[test]
    fn test_block_74000_dlc_dungeon_graces_verified() {
        // Block 74000 (DLC dungeon graces) verified via multiple proven flags
        // Block base_offset=3000, verified by matching 8+ consistent proven flags

        // Flag 74000 (Fog Rift Catacombs): byte=3000, bit=7
        let result = get_flag_offset(74000);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 3000);
        assert_eq!(bit, 7);

        // Flag 74100 (Belurat Gaol): byte=3000+100/8=3012, bit=7-(100%8)=3
        let result = get_flag_offset(74100);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 3012);
        assert_eq!(bit, 3);

        // Flag 74200 (Ruined Forge Lava Intake): byte=3000+200/8=3025, bit=7-(200%8)=7
        let result = get_flag_offset(74200);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 3025);
        assert_eq!(bit, 7);
    }

    #[test]
    fn test_block_65000_crystal_tears_verified() {
        // Block 65000 (Whetblades/Crystal Tears) - CORRECTED 2026-02-15
        // Block base_offset=1685 (0x694+1) from common.emevd.js with +1 correction
        //   65610 -> 1685 + 610/8 = 1685 + 76 = 1761
        //   65700 -> 1685 + 700/8 = 1685 + 87 = 1772
        //   65720 -> 1685 + 720/8 = 1685 + 90 = 1775

        // Flag 65610: byte=1761, bit=7-(610%8)=7-2=5
        let result = get_flag_offset(65610);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 1761);
        assert_eq!(bit, 5);

        // Flag 65700: byte=1772, bit=7-(700%8)=7-4=3
        let result = get_flag_offset(65700);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 1772);
        assert_eq!(bit, 3);

        // Flag 65720: byte=1775, bit=7-(720%8)=7-0=7
        let result = get_flag_offset(65720);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 1775);
        assert_eq!(bit, 7);
    }

    #[test]
    fn test_bit_calculation() {
        // Verify bit position is always 7 - (flag % 8)
        for flag in [1042370000u32, 1042370001, 1042370007, 1042370008] {
            if let Some((_, bit)) = get_flag_offset(flag) {
                assert_eq!(bit, (7 - (flag % 8)) as u8);
            }
        }
    }

    #[test]
    fn test_block_flag_verified() {
        // Test block flag 76100 (The First Step grace) using VERIFIED_BLOCK_BASES
        // Block 76000 base = 3250, relative = 100
        // byte = 3250 + 100/8 = 3250 + 12 = 3262
        // bit = 7 - (76100 % 8) = 7 - 4 = 3
        let result = get_flag_offset(76100);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 3262);
        assert_eq!(bit, 3);
    }

    #[test]
    fn test_coverage_improvement() {
        // Test some flags that were previously not calculable
        // These should now work with the formula approach

        // Tile 33_42 (first valid tile)
        assert!(get_flag_offset(1033420000).is_some());

        // Tile 45_37 (Caelid)
        assert!(get_flag_offset(1045370000).is_some());

        // Tile 50_55 (Mountaintops)
        assert!(get_flag_offset(1050550000).is_some());
    }

    #[test]
    fn test_is_block_reliable() {
        // Verified blocks should be reliable
        assert!(is_block_reliable(76100)); // The First Step - block 76000 is verified
        assert!(is_block_reliable(76101)); // Another Limgrave grace
        assert!(is_block_reliable(71800)); // Cave of Knowledge - block 71800 is verified
        assert!(is_block_reliable(72000)); // DLC grace - block 72000 is verified

        // Unreliable blocks should NOT be reliable
        assert!(!is_block_reliable(71000)); // Godrick the Grafted - block 71000 is unreliable
        assert!(!is_block_reliable(71007)); // Stormveil grace - block 71000 is unreliable
        assert!(!is_block_reliable(71100)); // Leyndell grace - block 71100 is unreliable
        assert!(!is_block_reliable(71105)); // Another Leyndell grace
        assert!(!is_block_reliable(71600)); // Rykard - block 71600 is unreliable
        assert!(!is_block_reliable(71607)); // Volcano Manor grace

        // Non-block flags should be reliable (they use different formulas)
        assert!(is_block_reliable(300));           // Small flag
        assert!(is_block_reliable(1042370000));    // Tile flag
        assert!(is_block_reliable(10007030));      // Dungeon flag
    }

    #[test]
    fn test_convert_to_row_id() {
        // getItemFlagId with localId >= 7000 should be converted
        // Example: 1033407100 (localId 7100) -> 1033400100 (localId 100)
        assert_eq!(convert_to_row_id(1033407100), Some(1033400100));

        // Another example: 1044367310 (localId 7310) -> 1044360310 (localId 310)
        assert_eq!(convert_to_row_id(1044367310), Some(1044360310));

        // LocalId < 7000 should return None (already storable)
        assert_eq!(convert_to_row_id(1033400100), None);
        assert_eq!(convert_to_row_id(1043500010), None); // Smoldering Butterfly

        // Non-tile flags should return None
        assert_eq!(convert_to_row_id(76100), None);     // Block flag
        assert_eq!(convert_to_row_id(10007030), None);  // Dungeon flag
        assert_eq!(convert_to_row_id(300), None);       // Simple flag
    }
}
