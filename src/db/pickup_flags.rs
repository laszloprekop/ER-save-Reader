/// Module for calculating event flag offsets and tracking item pickups
///
/// The EventFlags array (0x1BF99F bytes) uses hierarchical allocation:
/// - Small flags (0-59999): Direct mapping with base offset
/// - Block flags (60000-99999): Block base + relative offset
/// - Dungeon flags (10000000-43999999): Map base + local offset
/// - Open world (1000000000+): Formula-based tile calculation
///
/// V2: Uses formula-based tile offset calculation for ~90% coverage
/// Based on elden-map project's test-event-flag-calculations.ts

use std::collections::HashMap;
use once_cell::sync::Lazy;

// ============================================================================
// CONSTANTS (derived from elden-map analysis)
// ============================================================================

/// Base offset for small flags (subtract from byte calculation)
pub const FLAG_BASE_OFFSET: u32 = 6250;

/// Base offset where tile flags start in event flags array
pub const TILE_BASE_OFFSET: u32 = 347000;

/// First row (X coordinate) of world tiles
pub const TILE_ROW_BASE: u32 = 33;

/// First column (Y coordinate) of world tiles
pub const TILE_COL_BASE: u32 = 42;

/// Bytes allocated per tile slot (1000 flags / 8 bits * ~7 categories)
pub const TILE_BYTES_PER_SLOT: u32 = 875;

/// Number of tile slots per row
pub const TILE_SLOTS_PER_ROW: u32 = 40;

/// Total size of event flags section
pub const EVENT_FLAGS_SIZE: u32 = 0x1BF99F; // 1,833,375 bytes

/// Bytes per dungeon section
pub const DUNGEON_SECTION_SIZE: u32 = 1125;

/// Bytes per category in open world tiles
pub const CATEGORY_SIZE_BYTES: u32 = 125;

/// Category 7 is used for item pickups (treasures)
pub const TREASURE_CATEGORY: u32 = 7;

// ============================================================================
// DUNGEON BASE OFFSETS (complete mapping from elden-map)
// ============================================================================

/// Complete dungeon base offsets for save file event flags
///
/// CORRECTED (2026-01-09): Empirical verification against actual save files showed
/// the previous offsets (1,383,375+) were from runtime memory, not save file format.
/// Correct base for Stormveil (10_00) is 4112, verified against:
/// - Godskin Prayerbook (flag 10007990): byte 5110, bit 1 ✓
/// - Fire Grease (flag 10007040): byte 4992 ✓
/// - Arbalest (flag 10007550): byte 5055 ✓
///
/// Key format: "XX_YY" where XX is map area, YY is section
/// Formula: offset = 4112 + slot_index * 1125 (where slot comes from legacymap.eventflagalloclist)
pub static DUNGEON_BASE_OFFSETS: Lazy<HashMap<&'static str, u32>> = Lazy::new(|| {
    HashMap::from([
        // Stormveil Castle (m10) - Slot 0, 1
        ("10_00", 4112), ("10_01", 5237),
        // Leyndell (m11) - Slots 4, 5, 6, 7
        ("11_00", 8612), ("11_05", 9737), ("11_10", 10862), ("11_71", 11987),
        // Underground areas (m12) - Slots 11-19
        ("12_01", 16487), ("12_02", 17612), ("12_03", 18737), ("12_04", 19862),
        ("12_05", 20987), ("12_06", 22112), ("12_07", 23237), ("12_08", 24362),
        ("12_09", 25487),
        // Crumbling Farum Azula (m13) - Slot 20
        ("13_00", 26612),
        // Academy of Raya Lucaria (m14) - Slot 23
        ("14_00", 29987),
        // Caria Manor (m15) - Slot 26
        ("15_00", 33362),
        // Volcano Manor (m16) - Slot 29
        ("16_00", 36737),
        // Roundtable Hold (m18) - Slot 35
        ("18_00", 43487),
        // Chapel of Anticipation (m19) - Slot 38
        ("19_00", 46862),
        // Stranded Graveyard / Cave of Knowledge (m20) - Slot 41
        ("20_00", 50237),
        // Miquella's Haligtree (m21) - Slots 44, 45, 46
        ("21_00", 53612), ("21_01", 54737), ("21_02", 55862),
        // Castle Sol (m22) - Slot 47
        ("22_00", 59237),
        // Catacombs (m30) - Slots 78-98
        ("30_00", 94112), ("30_01", 95237), ("30_02", 96362), ("30_03", 97487),
        ("30_04", 98612), ("30_05", 99737), ("30_06", 100862), ("30_07", 101987),
        ("30_08", 103112), ("30_09", 104237), ("30_10", 105362), ("30_11", 106487),
        ("30_12", 107612), ("30_13", 108737), ("30_14", 109862), ("30_15", 110987),
        ("30_16", 112112), ("30_17", 113237), ("30_18", 114362), ("30_19", 115487),
        ("30_20", 116612),
        // Caves (m31) - Slots 108-129
        ("31_00", 127862), ("31_01", 128987), ("31_02", 130112), ("31_03", 131237),
        ("31_04", 132362), ("31_05", 133487), ("31_06", 134612), ("31_07", 135737),
        ("31_09", 137987), ("31_10", 139112), ("31_11", 140237), ("31_12", 141362),
        ("31_15", 144737), ("31_17", 145862), ("31_18", 146987), ("31_19", 148112),
        ("31_20", 149237), ("31_21", 150362), ("31_22", 151487),
        // Tunnels (m32) - Slots 138-149
        ("32_00", 161612), ("32_01", 162737), ("32_02", 163862), ("32_04", 164987),
        ("32_05", 167237), ("32_07", 168362), ("32_08", 169487), ("32_11", 170612),
        // Divine Towers (m34) - Slots 58-64
        ("34_10", 71612), ("34_11", 72737), ("34_12", 73862), ("34_13", 74987),
        ("34_14", 76112), ("34_15", 77237), ("34_16", 78362),
        // Mohgwyn Palace (m35) - Slot 41 (same as m20)
        ("35_00", 50237),
        // Elden Throne (m39) - Slot 44 (same as m21_00)
        ("39_20", 53612),
        // Hero's Graves (m40) - Slot 150+
        ("40_00", 171737),
        // Minor dungeons (m41) - Slot 158+
        ("41_00", 180737),
        // Crystal tunnels (m42) - Slot 167+
        ("42_00", 190737), ("42_02", 191862),
        // Misc dungeons (m43) - Slot 176+
        ("43_00", 200737),
    ])
});

/// Block bases for flags 60000-99999 (special system flags)
/// Verified against EVENT_FLAGS static mappings
pub static BLOCK_BASES: Lazy<HashMap<u32, u32>> = Lazy::new(|| {
    HashMap::from([
        (60000, 1250),   // Map flags
        (62000, 1500),   // Grace flags
        (65000, 1875),   // Whetblade flags (verified: 65610 → 0x79f = 1951)
        (66000, 2000),   // Pot flags
        (67000, 2125),   // Cookbook flags (verified: 67000 → 0x84d = 2125)
        (68000, 2250),   // Cookbook flags continued
        (69000, 2375),   // Remembrance flags
        (71000, 2625),   // Boss/dungeon flags
        (73000, 2875),   // System flags
        (76000, 3250),   // Other system flags
    ])
});

// ============================================================================
// FLAG OFFSET CALCULATIONS
// ============================================================================

/// Calculate byte offset and bit position for a tile (open world) flag
///
/// Flag format: 10XXYYZZZN where:
/// - XX: tile row (33-60+)
/// - YY: tile column (30-58+)
/// - Z: category (0-9)
/// - NNN: local offset (0-999)
fn calculate_tile_flag_offset(flag_id: u32) -> Option<(u32, u8)> {
    let bit = (7 - (flag_id % 8)) as u8;

    let tile_index = (flag_id - 1_000_000_000) / 10000;
    let local_id = flag_id % 10000;

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
/// Flag format: XXYYZZZZ where:
/// - XX: map area (10-43)
/// - YY: section (00-22)
/// - ZZZZ: local offset (0-9999)
fn calculate_dungeon_flag_offset(flag_id: u32) -> Option<(u32, u8)> {
    let bit = (7 - (flag_id % 8)) as u8;

    let flag_str = format!("{:08}", flag_id);
    let dungeon_area = &flag_str[0..2];
    let section = &flag_str[2..4];
    let local_id: u32 = flag_str[4..8].parse().ok()?;

    let key = format!("{}_{}", dungeon_area, section);

    // Try to get exact base offset
    let base_offset = if let Some(&base) = DUNGEON_BASE_OFFSETS.get(key.as_str()) {
        base
    } else {
        // Fall back to calculating from base section
        let base_key = format!("{}_00", dungeon_area);
        let section_num: u32 = section.parse().ok()?;
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

    // Block flags (60000-99999)
    let block_start = (flag_id / 1000) * 1000;
    if let Some(&base) = BLOCK_BASES.get(&block_start) {
        let relative = flag_id - block_start;
        let byte_offset = base + relative / 8;
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
pub fn get_flag_offset(flag_id: u32) -> Option<(u32, u8)> {
    // 10-digit open world tile flags (1000000000+)
    if flag_id >= 1_000_000_000 {
        return calculate_tile_flag_offset(flag_id);
    }

    // 8-digit dungeon flags (10000000-43999999)
    if flag_id >= 10_000_000 && flag_id < 44_000_000 {
        return calculate_dungeon_flag_offset(flag_id);
    }

    // Simple flags (< 100000)
    if flag_id < 100_000 {
        return calculate_simple_flag_offset(flag_id);
    }

    // Flags 100000-9999999 are not commonly used for pickups
    None
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
        // Limgrave
        (41..=44, 36..=39) => "Limgrave",
        (43..=44, 30..=35) => "Weeping Peninsula",
        (44..=45, 32..=35) => "Stormhill",

        // Liurnia
        (33..=40, 40..=50) => "Liurnia of the Lakes",

        // Caelid
        (45..=52, 36..=43) => "Caelid",

        // Altus Plateau
        (38..=44, 49..=55) => "Altus Plateau",

        // Mt. Gelmir
        (33..=38, 49..=55) => "Mt. Gelmir",

        // Mountaintops of the Giants
        (47..=54, 54..=58) => "Mountaintops of the Giants",

        // Consecrated Snowfield
        (47..=54, 55..=58) => "Consecrated Snowfield",

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

    #[test]
    fn test_small_flag_offset() {
        // Flag 300 should be at byte 37, bit 3
        assert_eq!(get_flag_offset(300), Some((37, 3)));

        // Flag 6080 should be at byte 760, bit 7
        assert_eq!(get_flag_offset(6080), Some((760, 7)));
    }

    #[test]
    fn test_tile_flag_formula() {
        // Test Limgrave tile 42_37
        // Slot = (42-33)*40 + (37-42) = 360 - 5 = 355
        // Base = 347000 + 355*875 = 347000 + 310625 = 657625
        let result = get_flag_offset(1042370000);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 657625);
        assert_eq!(bit, 7);
    }

    #[test]
    fn test_dungeon_flag() {
        // Stormveil Castle flag (m10_00)
        // Base for 10_00 is 4112 (empirically verified 2026-01-09)
        // Flag 10007030 -> local 7030, byte offset = 4112 + 7030/8 = 4112 + 878 = 4990
        let result = get_flag_offset(10007030);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 4112 + 7030 / 8);  // 4990
        assert_eq!(bit, (7 - (7030 % 8)) as u8);
    }

    #[test]
    fn test_stormveil_godskin_prayerbook() {
        // Godskin Prayerbook (flag 10007990) - verified against actual save file
        // Local ID: 7990, byte within section: 7990/8 = 998
        // Byte offset: 4112 + 998 = 5110
        // Bit position: 7 - (7990 % 8) = 7 - 6 = 1
        let result = get_flag_offset(10007990);
        assert!(result.is_some());
        let (byte, bit) = result.unwrap();
        assert_eq!(byte, 5110);
        assert_eq!(bit, 1);
    }

    #[test]
    fn test_bit_calculation() {
        // Verify bit position is always 7 - (flag % 8)
        for flag in [1042377000u32, 1042377001, 1042377007, 1042377008] {
            if let Some((_, bit)) = get_flag_offset(flag) {
                assert_eq!(bit, (7 - (flag % 8)) as u8);
            }
        }
    }

    #[test]
    fn test_coverage_improvement() {
        // Test some flags that were previously not calculable
        // These should now work with the formula approach

        // Tile 33_40 (Liurnia)
        assert!(get_flag_offset(1033400000).is_some());

        // Tile 45_37 (Caelid)
        assert!(get_flag_offset(1045370000).is_some());

        // Tile 50_55 (Mountaintops)
        assert!(get_flag_offset(1050550000).is_some());
    }
}
