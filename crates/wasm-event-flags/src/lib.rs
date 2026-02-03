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
pub const SEARCH_START: usize = 0x12000;  // 73728
pub const MAX_SEARCH_RANGE: usize = 200_000;

/// Tile flag constants (10-digit flags like 1035537020)
pub const TILE_BASE_OFFSET: u32 = 485330;
pub const TILE_ROW_BASE: u32 = 33;
pub const TILE_COL_BASE: u32 = 30;
pub const TILE_BYTES_PER_SLOT: u32 = 875;
pub const TILE_SLOTS_PER_ROW: u32 = 40;
pub const TILE_MAX_LOCAL_ID: u32 = 6999;

/// World pickup row_id tracking (for getItemFlagId with local_id >= 7000)
/// DISCOVERY (2026-02-02): Pickups with getItemFlagId local_id >= 7000 are NOT
/// stored in tile region. They're tracked by ItemLotParam row_id in a separate
/// bitfield region. Verified via before/after save capture of Golden Rune pickup.
pub const WORLD_PICKUP_ROW_ID_BASE: u32 = 1037373320;

/// Dungeon section size
pub const DUNGEON_SECTION_SIZE: u32 = 1125;

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
    (76102, 3262, 1, "Gatefront Ruins", 2),
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

/// Internal implementation (also usable from native Rust without WASM)
pub fn detect_event_flags_offset_impl(slot_data: &[u8]) -> DetectionResult {
    let search_end = (SEARCH_START + MAX_SEARCH_RANGE).min(slot_data.len().saturating_sub(10000));

    let tier1_flags: Vec<_> = POSITIVE_VALIDATION_FLAGS.iter()
        .filter(|(_, _, _, _, tier)| *tier == 1)
        .collect();
    let tier1_count = tier1_flags.len();

    struct Candidate {
        offset: usize,
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

        if tier1_score >= tier1_count {
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

            if negative_score == NEGATIVE_VALIDATION_FLAGS.len() {
                return DetectionResult {
                    offset: test_offset,
                    positive_score,
                    negative_score,
                    confident: true,
                };
            }

            candidates.push(Candidate {
                offset: test_offset,
                positive_score,
                negative_score,
            });
        }
    }

    if !candidates.is_empty() {
        candidates.sort_by(|a, b| {
            b.negative_score.cmp(&a.negative_score)
                .then_with(|| b.positive_score.cmp(&a.positive_score))
                .then_with(|| a.offset.cmp(&b.offset))
        });

        let best = &candidates[0];
        return DetectionResult {
            offset: best.offset,
            positive_score: best.positive_score,
            negative_score: best.negative_score,
            confident: best.negative_score >= NEGATIVE_VALIDATION_FLAGS.len() / 2,
        };
    }

    // Fallback
    let mut best_offset = SEARCH_START;
    let mut best_tier1_score = 0;

    for test_offset in SEARCH_START..search_end {
        let mut tier1_score = 0;

        for &(_, byte_offset, bit_pos, _, tier) in POSITIVE_VALIDATION_FLAGS {
            if tier != 1 { continue; }
            let abs_pos = test_offset + byte_offset as usize;
            if abs_pos < slot_data.len() {
                let byte = slot_data[abs_pos];
                if (byte & (1 << bit_pos)) != 0 {
                    tier1_score += 1;
                }
            }
        }

        if tier1_score > best_tier1_score {
            best_tier1_score = tier1_score;
            best_offset = test_offset;
        }
    }

    DetectionResult {
        offset: best_offset,
        positive_score: best_tier1_score,
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
/// This was verified via before/after save captures of Golden Rune [1] and [3]
/// pickups at row_id 1044360310 and 1044360340.
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
        // Golden Rune [1] pickup at tile (44,36): row_id 1044360310
        // Discovered: EF+873373 bit 1
        let result = calculate_world_pickup_offset_by_row_id_impl(1044360310);
        assert!(result.valid);
        assert_eq!(result.byte_offset, 873373);
        assert_eq!(result.bit_position, 1);

        // Golden Rune [3] pickup at tile (44,36): row_id 1044360340
        // Discovered: EF+873377 bit 3
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
}
