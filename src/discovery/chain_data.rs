
/// A boss defeat chain with all related flags
#[derive(Debug, Clone)]
pub struct BossDefeatChain {
    pub name: &'static str,
    pub defeat_flag: u32,           // Boss defeat marker (171, 172, etc.)
    pub remembrance_flag: u32,      // Remembrance possession (91xx)
    pub great_rune_flag: Option<u32>,    // Great Rune possession (16x)
    pub activation_flag: Option<u32>,    // Great Rune activation (18x)
    pub remembrance_item: Option<u32>,   // Remembrance duplication flag (69xxx)
}

/// All major boss defeat chains
/// Source: common.emevd.js Events 720, 730, 1100, 1720
pub static BOSS_DEFEAT_CHAINS: &[BossDefeatChain] = &[
    BossDefeatChain {
        name: "Godrick the Grafted",
        defeat_flag: 171,
        remembrance_flag: 9101,
        great_rune_flag: Some(160),
        activation_flag: Some(180),
        remembrance_item: Some(69010),
    },
    BossDefeatChain {
        name: "Rennala, Queen of the Full Moon",
        defeat_flag: 172,
        remembrance_flag: 9102,
        great_rune_flag: Some(161),  // Unborn Great Rune (special)
        activation_flag: None,        // No activation needed
        remembrance_item: Some(69020),
    },
    BossDefeatChain {
        name: "Starscourge Radahn",
        defeat_flag: 173,
        remembrance_flag: 9103,
        great_rune_flag: Some(162),
        activation_flag: Some(182),
        remembrance_item: Some(69030),
    },
    BossDefeatChain {
        name: "Rykard, Lord of Blasphemy",
        defeat_flag: 174,
        remembrance_flag: 9104,
        great_rune_flag: Some(163),
        activation_flag: Some(183),
        remembrance_item: Some(69040),
    },
    BossDefeatChain {
        name: "Morgott, the Omen King",
        defeat_flag: 175,
        remembrance_flag: 9105,
        great_rune_flag: Some(164),
        activation_flag: Some(184),
        remembrance_item: Some(69050),
    },
    BossDefeatChain {
        name: "Mohg, Lord of Blood",
        defeat_flag: 176,
        remembrance_flag: 9106,
        great_rune_flag: Some(165),
        activation_flag: Some(185),
        remembrance_item: Some(69060),
    },
    BossDefeatChain {
        name: "Malenia, Blade of Miquella",
        defeat_flag: 177,
        remembrance_flag: 9107,
        great_rune_flag: Some(166),
        activation_flag: Some(186),
        remembrance_item: Some(69070),
    },
    BossDefeatChain {
        name: "Maliketh, the Black Blade",
        defeat_flag: 178,
        remembrance_flag: 9108,
        great_rune_flag: None,  // Destined Death, not equippable
        activation_flag: None,
        remembrance_item: Some(69080),
    },
    BossDefeatChain {
        name: "Hoarah Loux, Warrior",
        defeat_flag: 179,
        remembrance_flag: 9109,
        great_rune_flag: None,
        activation_flag: None,
        remembrance_item: Some(69090),
    },
    BossDefeatChain {
        name: "Radagon / Elden Beast",
        defeat_flag: 180,  // Note: This is final boss
        remembrance_flag: 9110,
        great_rune_flag: None,
        activation_flag: None,
        remembrance_item: Some(69100),
    },
];

/// Area prerequisite: items/flags needed to access an area
#[derive(Debug, Clone)]
pub struct AreaPrerequisite {
    pub area_name: &'static str,
    pub required_flags: &'static [u32],  // All must be set
    pub required_any: &'static [u32],    // At least one must be set (OR condition)
    pub area_flags_start: u32,           // Start of area-specific flags
    pub landmark_range: Option<(u32, u32)>, // Landmark flag range for area
}

/// Area prerequisites based on game progression
/// Source: docs/EVENT-FLAG-GEOGRAPHY.md
pub static AREA_PREREQUISITES: &[AreaPrerequisite] = &[
    // Consecrated Snowfield requires both medallion halves
    AreaPrerequisite {
        area_name: "Consecrated Snowfield",
        required_flags: &[60430, 60431],  // Left and Right medallion halves
        required_any: &[],
        area_flags_start: 1033550000,     // Tile flags for area
        landmark_range: Some((62550, 62574)),
    },
    // Miquella's Haligtree requires Consecrated Snowfield access
    AreaPrerequisite {
        area_name: "Miquella's Haligtree",
        required_flags: &[60430, 60431],  // Medallion halves (transitive)
        required_any: &[],
        area_flags_start: 15000000,       // Legacy dungeon flags
        landmark_range: None,
    },
    // Leyndell requires 2 Great Runes (at least one shardbearer defeated)
    AreaPrerequisite {
        area_name: "Leyndell, Royal Capital",
        required_flags: &[],
        required_any: &[171, 172, 173, 174],  // Godrick, Rennala, Radahn, or Rykard
        area_flags_start: 13000000,
        landmark_range: Some((62900, 62943)),
    },
    // Mohgwyn Palace requires either Varre questline or Consecrated Snowfield
    AreaPrerequisite {
        area_name: "Mohgwyn Palace",
        required_flags: &[],
        required_any: &[60430],  // Simplified: Medallion half or Varre quest
        area_flags_start: 35000000,
        landmark_range: Some((62800, 62831)),
    },
    // Crumbling Farum Azula requires beating Morgott and burning Erdtree
    AreaPrerequisite {
        area_name: "Crumbling Farum Azula",
        required_flags: &[175],  // Morgott defeated
        required_any: &[],
        area_flags_start: 16000000,
        landmark_range: Some((62950, 62981)),
    },
    // Mountaintops of the Giants requires Rold Medallion
    AreaPrerequisite {
        area_name: "Mountaintops of the Giants",
        required_flags: &[60420],  // Rold Medallion
        required_any: &[],
        area_flags_start: 1037530000,
        landmark_range: Some((62510, 62531)),
    },
];

/// Geographic region definition for proximity validation
#[derive(Debug, Clone)]
pub struct GeographicRegion {
    pub name: &'static str,
    pub landmark_range: (u32, u32),       // Landmark flag range (62xxx)
    pub tile_x_range: (u32, u32),         // Tile X coordinate range
    pub tile_y_range: (u32, u32),         // Tile Y coordinate range
    pub grace_range: Option<(u32, u32)>,  // Grace flag range (76xxx-79xxx)
    pub map_fragment: Option<u32>,        // Map fragment flag (62010-62084)
}

/// Geographic regions with flag ranges
/// Source: docs/EVENT-FLAG-GEOGRAPHY.md lines 47-59, 127-146
pub static GEOGRAPHIC_REGIONS: &[GeographicRegion] = &[
    GeographicRegion {
        name: "Limgrave",
        landmark_range: (62100, 62138),
        tile_x_range: (42, 44),
        tile_y_range: (36, 40),
        grace_range: Some((76100, 76199)),
        map_fragment: Some(62010),  // Limgrave, West
    },
    GeographicRegion {
        name: "Weeping Peninsula",
        landmark_range: (62150, 62184),
        tile_x_range: (40, 43),
        tile_y_range: (33, 35),
        grace_range: Some((76200, 76299)),
        map_fragment: Some(62011),  // Weeping Peninsula
    },
    GeographicRegion {
        name: "Liurnia of the Lakes",
        landmark_range: (62200, 62284),
        tile_x_range: (37, 44),
        tile_y_range: (41, 47),
        grace_range: Some((76300, 76499)),
        map_fragment: Some(62020),  // Liurnia, East
    },
    GeographicRegion {
        name: "Altus Plateau",
        landmark_range: (62300, 62348),
        tile_x_range: (37, 44),
        tile_y_range: (48, 52),
        grace_range: Some((76500, 76599)),
        map_fragment: Some(62040),  // Altus Plateau
    },
    GeographicRegion {
        name: "Mt. Gelmir",
        landmark_range: (62350, 62389),
        tile_x_range: (33, 38),
        tile_y_range: (48, 52),
        grace_range: Some((76600, 76699)),
        map_fragment: Some(62050),  // Mt. Gelmir
    },
    GeographicRegion {
        name: "Caelid",
        landmark_range: (62400, 62438),
        tile_x_range: (46, 54),
        tile_y_range: (36, 44),
        grace_range: Some((76700, 76799)),
        map_fragment: Some(62030),  // Caelid
    },
    GeographicRegion {
        name: "Greyoll's Dragonbarrow",
        landmark_range: (62460, 62475),
        tile_x_range: (48, 54),
        tile_y_range: (45, 50),
        grace_range: Some((76800, 76899)),
        map_fragment: Some(62031),  // Dragonbarrow
    },
    GeographicRegion {
        name: "Mountaintops of the Giants",
        landmark_range: (62510, 62531),
        tile_x_range: (37, 44),
        tile_y_range: (53, 58),
        grace_range: Some((77000, 77099)),
        map_fragment: Some(62060),  // Mountaintops, West
    },
    GeographicRegion {
        name: "Consecrated Snowfield",
        landmark_range: (62550, 62574),
        tile_x_range: (33, 38),
        tile_y_range: (55, 58),
        grace_range: Some((77100, 77199)),
        map_fragment: Some(62070),  // Consecrated Snowfield
    },
    GeographicRegion {
        name: "Siofra River",
        landmark_range: (62610, 62634),
        tile_x_range: (0, 0),  // Underground, no tiles
        tile_y_range: (0, 0),
        grace_range: Some((77200, 77299)),
        map_fragment: Some(62080),
    },
    GeographicRegion {
        name: "Ainsel River",
        landmark_range: (62640, 62640),
        tile_x_range: (0, 0),
        tile_y_range: (0, 0),
        grace_range: Some((77300, 77399)),
        map_fragment: Some(62081),
    },
    GeographicRegion {
        name: "Deeproot Depths",
        landmark_range: (62700, 62740),
        tile_x_range: (0, 0),
        tile_y_range: (0, 0),
        grace_range: Some((77400, 77499)),
        map_fragment: Some(62082),
    },
    GeographicRegion {
        name: "Mohgwyn Palace",
        landmark_range: (62800, 62831),
        tile_x_range: (0, 0),
        tile_y_range: (0, 0),
        grace_range: Some((77500, 77599)),
        map_fragment: None,
    },
    GeographicRegion {
        name: "Lake of Rot",
        landmark_range: (62840, 62844),
        tile_x_range: (0, 0),
        tile_y_range: (0, 0),
        grace_range: Some((77600, 77699)),
        map_fragment: Some(62083),
    },
    GeographicRegion {
        name: "Nokron / Nokstella",
        landmark_range: (62850, 62891),
        tile_x_range: (0, 0),
        tile_y_range: (0, 0),
        grace_range: Some((77700, 77799)),
        map_fragment: Some(62084),
    },
    GeographicRegion {
        name: "Leyndell",
        landmark_range: (62900, 62943),
        tile_x_range: (0, 0),  // Legacy dungeon
        tile_y_range: (0, 0),
        grace_range: Some((77800, 77899)),
        map_fragment: None,
    },
    GeographicRegion {
        name: "Crumbling Farum Azula",
        landmark_range: (62950, 62981),
        tile_x_range: (0, 0),
        tile_y_range: (0, 0),
        grace_range: Some((77900, 77999)),
        map_fragment: None,
    },
];

/// Scroll/item unlock: giving item to NPC unlocks spells/recipes
#[derive(Debug, Clone)]
pub struct ScrollUnlock {
    pub item_name: &'static str,
    pub pickup_flag: u32,           // World pickup flag (10-digit)
    pub unlocked_items: &'static [&'static str],
    pub npc: &'static str,
}

/// Scroll and prayerbook unlocks
/// Source: ShopLineupParam.param.xml eventFlag_forRelease patterns
pub static SCROLL_UNLOCKS: &[ScrollUnlock] = &[
    // Sorcery Scrolls for Sellen
    ScrollUnlock {
        item_name: "Royal House Scroll",
        pickup_flag: 1044369244,
        unlocked_items: &["Glintstone Stars", "Glintstone Arc"],
        npc: "Sorceress Sellen",
    },
    ScrollUnlock {
        item_name: "Academy Scroll",
        pickup_flag: 1036489244,  // Liurnia
        unlocked_items: &["Great Glintstone Shard", "Swift Glintstone Shard"],
        npc: "Sorceress Sellen",
    },
    ScrollUnlock {
        item_name: "Conspectus Scroll",
        pickup_flag: 1035459244,  // Raya Lucaria
        unlocked_items: &["Glintstone Cometshard", "Star Shower"],
        npc: "Sorceress Sellen",
    },
    // Incantation Prayerbooks for Brother Corhyn / Miriel
    ScrollUnlock {
        item_name: "Assassin's Prayerbook",
        pickup_flag: 1039429244,
        unlocked_items: &["Assassin's Approach", "Darkness"],
        npc: "Brother Corhyn / Miriel",
    },
    ScrollUnlock {
        item_name: "Two Fingers' Prayerbook",
        pickup_flag: 1034509244,
        unlocked_items: &["Lord's Heal", "Lord's Aid"],
        npc: "Brother Corhyn / Miriel",
    },
    ScrollUnlock {
        item_name: "Fire Monks' Prayerbook",
        pickup_flag: 11109874,  // Legacy dungeon flag
        unlocked_items: &["O, Flame!", "Surge, O Flame!"],
        npc: "Brother Corhyn / Miriel",
    },
    ScrollUnlock {
        item_name: "Giant's Prayerbook",
        pickup_flag: 1052569244,
        unlocked_items: &["Giantsflame Take Thee", "Flame, Fall Upon Them"],
        npc: "Brother Corhyn / Miriel",
    },
    ScrollUnlock {
        item_name: "Dragon Cult Prayerbook",
        pickup_flag: 1036449244,
        unlocked_items: &["Lightning Spear", "Honed Bolt"],
        npc: "Brother Corhyn / Miriel",
    },
    ScrollUnlock {
        item_name: "Ancient Dragon Prayerbook",
        pickup_flag: 1051369244,
        unlocked_items: &["Ancient Dragons' Lightning Spear", "Ancient Dragons' Lightning Strike"],
        npc: "Brother Corhyn / Miriel",
    },
    ScrollUnlock {
        item_name: "Golden Order Principia",
        pickup_flag: 1035469244,
        unlocked_items: &["Radagon's Rings of Light", "Law of Regression"],
        npc: "Brother Corhyn / Miriel",
    },
];

/// Verified block base offsets for cross-validation
/// Source: docs/EVENT-FLAG-GEOGRAPHY.md lines 73-84
#[derive(Debug, Clone, Copy)]
pub struct BlockBaseOffset {
    pub block_start: u32,
    pub base_offset: u32,  // Hex value as decimal
    pub category: &'static str,
}

pub static VERIFIED_BLOCK_BASES: &[BlockBaseOffset] = &[
    BlockBaseOffset { block_start: 60000, base_offset: 0x4ec, category: "Progression" },
    BlockBaseOffset { block_start: 62000, base_offset: 0x5dc, category: "Map/Landmarks" },
    BlockBaseOffset { block_start: 65000, base_offset: 0x694, category: "Whetblades" },
    BlockBaseOffset { block_start: 66000, base_offset: 0x6bc, category: "Pot Upgrades" },
    BlockBaseOffset { block_start: 67000, base_offset: 0x6e4, category: "Cookbooks" },
    BlockBaseOffset { block_start: 68000, base_offset: 0x70c, category: "Cookbooks" },
    BlockBaseOffset { block_start: 69000, base_offset: 0x734, category: "Remembrance" },
    BlockBaseOffset { block_start: 76000, base_offset: 0xcb2, category: "Graces" },
    BlockBaseOffset { block_start: 91000, base_offset: 0x950, category: "Boss Remembrance" },
    BlockBaseOffset { block_start: 92000, base_offset: 0x978, category: "Container Upgrades" },
];

/// Find which region a flag belongs to
pub fn find_region_for_flag(flag_id: u32) -> Option<&'static GeographicRegion> {
    // Check landmark ranges
    if flag_id >= 62100 && flag_id < 63000 {
        return GEOGRAPHIC_REGIONS.iter().find(|r| {
            flag_id >= r.landmark_range.0 && flag_id <= r.landmark_range.1
        });
    }

    // Check grace ranges
    if flag_id >= 76000 && flag_id < 80000 {
        return GEOGRAPHIC_REGIONS.iter().find(|r| {
            if let Some((start, end)) = r.grace_range {
                flag_id >= start && flag_id <= end
            } else {
                false
            }
        });
    }

    // Check tile flags (10-digit)
    if flag_id >= 1_000_000_000 {
        let tile_index = (flag_id - 1_000_000_000) / 10000;
        let tile_x = (tile_index / 100) as u32;
        let tile_y = (tile_index % 100) as u32;

        return GEOGRAPHIC_REGIONS.iter().find(|r| {
            tile_x >= r.tile_x_range.0 && tile_x <= r.tile_x_range.1 &&
            tile_y >= r.tile_y_range.0 && tile_y <= r.tile_y_range.1
        });
    }

    None
}

/// Find boss chain by defeat flag
pub fn find_boss_chain_by_defeat(defeat_flag: u32) -> Option<&'static BossDefeatChain> {
    BOSS_DEFEAT_CHAINS.iter().find(|c| c.defeat_flag == defeat_flag)
}

/// Find boss chain by remembrance flag
pub fn find_boss_chain_by_remembrance(remembrance_flag: u32) -> Option<&'static BossDefeatChain> {
    BOSS_DEFEAT_CHAINS.iter().find(|c| c.remembrance_flag == remembrance_flag)
}

/// Find area prerequisite by name
pub fn find_area_prerequisite(area_name: &str) -> Option<&'static AreaPrerequisite> {
    AREA_PREREQUISITES.iter().find(|a| a.area_name == area_name)
}

/// Check if a flag is in a late-game area (requires prerequisites)
pub fn is_late_game_flag(flag_id: u32) -> bool {
    for area in AREA_PREREQUISITES {
        // Check landmark range
        if let Some((start, end)) = area.landmark_range {
            if flag_id >= start && flag_id <= end {
                return true;
            }
        }
        // Check area flags
        if flag_id >= area.area_flags_start && flag_id < area.area_flags_start + 10_000_000 {
            return true;
        }
    }
    false
}

/// Get all flags that should be correlated with a given flag based on geography
pub fn get_geographic_correlations(flag_id: u32) -> Vec<(u32, &'static str)> {
    let mut correlations = Vec::new();

    if let Some(region) = find_region_for_flag(flag_id) {
        // Add map fragment if available
        if let Some(map_flag) = region.map_fragment {
            correlations.push((map_flag, "map_fragment"));
        }

        // Add sample landmarks from region
        let (start, end) = region.landmark_range;
        if flag_id < start || flag_id > end {
            // Flag is not a landmark, add some landmarks as correlations
            correlations.push((start, "landmark_start"));
            if end > start {
                correlations.push(((start + end) / 2, "landmark_mid"));
            }
        }

        // Add sample graces from region
        if let Some((grace_start, _)) = region.grace_range {
            if flag_id < 76000 || flag_id >= 80000 {
                correlations.push((grace_start, "grace_start"));
            }
        }
    }

    correlations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boss_chains_complete() {
        assert!(BOSS_DEFEAT_CHAINS.len() >= 8, "Should have at least 8 major bosses");

        // Check Godrick chain
        let godrick = find_boss_chain_by_defeat(171).unwrap();
        assert_eq!(godrick.name, "Godrick the Grafted");
        assert_eq!(godrick.remembrance_flag, 9101);
        assert_eq!(godrick.great_rune_flag, Some(160));
        assert_eq!(godrick.activation_flag, Some(180));
    }

    #[test]
    fn test_region_lookup() {
        // Limgrave landmark
        let region = find_region_for_flag(62110).unwrap();
        assert_eq!(region.name, "Limgrave");

        // Caelid landmark
        let region = find_region_for_flag(62410).unwrap();
        assert_eq!(region.name, "Caelid");

        // Tile-based lookup
        let region = find_region_for_flag(1043380100).unwrap();
        assert_eq!(region.name, "Limgrave");
    }

    #[test]
    fn test_late_game_flags() {
        // Consecrated Snowfield is late game
        assert!(is_late_game_flag(62560));

        // Limgrave is not late game
        assert!(!is_late_game_flag(62110));
    }

    #[test]
    fn test_block_bases() {
        assert!(VERIFIED_BLOCK_BASES.len() >= 10);

        let grace_block = VERIFIED_BLOCK_BASES.iter()
            .find(|b| b.block_start == 76000)
            .unwrap();
        assert_eq!(grace_block.base_offset, 0xcb2);
    }
}
