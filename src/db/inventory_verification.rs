//! Inventory Verification Module
//!
//! Provides a third verification point for event flags by checking if items
//! associated with flags are present in the character's inventory.
//!
//! ## Verification Triangle
//!
//! 1. **Auto-detection**: Event flag status from save file
//! 2. **User logged**: Manual completion in UI
//! 3. **Inventory possession**: Character has the item (this module)
//!
//! ## Phase 1: High-Confidence Unique Items
//!
//! - Remembrances (2950-2964): Boss defeat rewards, one per boss
//! - Great Runes (8148-8153): Shardbearer rewards, one per boss
//! - Cookbooks (8000-8099): Crafting unlocks, one per location
//! - Whetblades (8200-8299): AoW affinity unlocks, one per location

use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy;

use crate::save::common::save_slot::EquipInventoryData;

// ============================================================================
// TYPES
// ============================================================================

/// Confidence level for inventory verification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationConfidence {
    /// Item is unique, one-time only (Remembrances, Great Runes)
    VeryHigh,
    /// Item has one known source (Cookbooks, Whetblades, unique weapons)
    High,
    /// Item could come from multiple sources
    Medium,
    /// Item is stackable/common or no mapping exists
    Low,
    /// No inventory data available
    Unknown,
}

impl VerificationConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationConfidence::VeryHigh => "very_high",
            VerificationConfidence::High => "high",
            VerificationConfidence::Medium => "medium",
            VerificationConfidence::Low => "low",
            VerificationConfidence::Unknown => "unknown",
        }
    }
}

/// Result of verifying an event flag against inventory
#[derive(Debug, Clone)]
pub struct InventoryVerificationResult {
    /// The event flag ID being verified
    pub flag_id: u32,
    /// Expected item ID(s) for this flag
    pub expected_items: Vec<UniqueItemMapping>,
    /// Whether any expected item is in inventory
    pub has_any_item: bool,
    /// Items found in inventory
    pub items_found: Vec<(u32, String, u32)>, // (item_id, name, quantity)
    /// Confidence level of this verification
    pub confidence: VerificationConfidence,
    /// Whether flag status matches inventory possession
    pub flag_matches_inventory: Option<bool>,
}

/// Mapping between a unique item and its associated event flag
#[derive(Debug, Clone)]
pub struct UniqueItemMapping {
    /// Item ID in inventory
    pub item_id: u32,
    /// Item name for display
    pub name: &'static str,
    /// Associated event flag
    pub event_flag: u32,
    /// Category for grouping
    pub category: UniqueItemCategory,
    /// Confidence level
    pub confidence: VerificationConfidence,
}

/// Categories of unique items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UniqueItemCategory {
    Remembrance,
    GreatRune,
    Cookbook,
    Whetblade,
    BossWeapon,
    KeyItem,
    AshOfWar,
    SpiritAsh,
    Talisman,
}

impl UniqueItemCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            UniqueItemCategory::Remembrance => "Remembrance",
            UniqueItemCategory::GreatRune => "Great Rune",
            UniqueItemCategory::Cookbook => "Cookbook",
            UniqueItemCategory::Whetblade => "Whetblade",
            UniqueItemCategory::BossWeapon => "Boss Weapon",
            UniqueItemCategory::KeyItem => "Key Item",
            UniqueItemCategory::AshOfWar => "Ash of War",
            UniqueItemCategory::SpiritAsh => "Spirit Ash",
            UniqueItemCategory::Talisman => "Talisman",
        }
    }
}

// ============================================================================
// UNIQUE ITEM DATABASE
// ============================================================================

/// All unique items that can be used for inventory verification
/// Maps event_flag -> UniqueItemMapping
pub static UNIQUE_ITEMS_BY_FLAG: Lazy<HashMap<u32, Vec<UniqueItemMapping>>> = Lazy::new(|| {
    let mut m: HashMap<u32, Vec<UniqueItemMapping>> = HashMap::new();

    for item in UNIQUE_ITEMS.iter() {
        m.entry(item.event_flag)
            .or_default()
            .push(item.clone());
    }

    m
});

/// Reverse mapping: item_id -> event_flag(s)
pub static FLAGS_BY_ITEM: Lazy<HashMap<u32, Vec<u32>>> = Lazy::new(|| {
    let mut m: HashMap<u32, Vec<u32>> = HashMap::new();

    for item in UNIQUE_ITEMS.iter() {
        m.entry(item.item_id)
            .or_default()
            .push(item.event_flag);
    }

    m
});

/// Static list of unique items for Phase 1
///
/// IMPORTANT: remembrances/great runes are verified against the SOURCE BOSS'S DEFEAT FLAG
/// (from `bosses_data`), not the low 171-180 "world drop" ids that earlier versions used.
/// Those 171-180 ids are < 50,000, so no resolver family covers them and they always read
/// Unknown (the false-negatives fixed in v0.37.20/21). The defeat flags are 8-digit dungeon
/// or 10-digit tile ids, read via `world_flag_state` in `collect_set_flags` (`src/ui/events.rs`).
/// Defeat-flag semantics are also what let the triangle flag a *consumed* remembrance as
/// flag-set-but-absent. (510xxx consumption flags — set when the remembrance is spent at Enia —
/// are deliberately NOT used.)
pub static UNIQUE_ITEMS: &[UniqueItemMapping] = &[
    // ========================================================================
    // REMEMBRANCES (Boss defeat rewards - VERY HIGH confidence)
    // event_flag = the source boss's defeat flag from bosses_data (world_flag_state)
    // ========================================================================
    UniqueItemMapping {
        item_id: 2950,
        name: "Remembrance of the Grafted",
        event_flag: 10000800, // Boss defeat = item obtained
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 2951,
        name: "Remembrance of the Starscourge",
        event_flag: 1052380800, // Radahn defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 2952,
        name: "Remembrance of the Omen King",
        event_flag: 11000800, // Morgott defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 2953,
        name: "Remembrance of the Blasphemous",
        event_flag: 16000800, // Rykard defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 2954,
        name: "Remembrance of the Rot Goddess",
        event_flag: 15000800, // Malenia defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 2955,
        name: "Remembrance of the Blood Lord",
        event_flag: 12050800, // Mohg defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 2956,
        name: "Remembrance of the Black Blade",
        event_flag: 13000800, // Maliketh defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 2957,
        name: "Remembrance of Hoarah Loux",
        event_flag: 11050800, // Hoarah Loux defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 2958,
        name: "Remembrance of the Dragonlord",
        event_flag: 13000830, // Placidusax defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::Medium, // Lower confidence until verified
    },
    UniqueItemMapping {
        item_id: 2959,
        name: "Remembrance of the Full Moon Queen",
        event_flag: 14000800, // Rennala defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 2960,
        name: "Remembrance of the Lichdragon",
        event_flag: 12030850, // Fortissax defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::Medium,
    },
    UniqueItemMapping {
        item_id: 2961,
        name: "Remembrance of the Fire Giant",
        event_flag: 1052520800, // Fire Giant defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::Medium,
    },
    UniqueItemMapping {
        item_id: 2962,
        name: "Remembrance of the Regal Ancestor",
        event_flag: 12090800, // Regal Ancestor defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::Medium,
    },
    UniqueItemMapping {
        item_id: 2963,
        name: "Elden Remembrance",
        event_flag: 19000800, // Elden Beast defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 2964,
        name: "Remembrance of the Naturalborn",
        event_flag: 12040800, // Astel Naturalborn defeat
        category: UniqueItemCategory::Remembrance,
        confidence: VerificationConfidence::Medium,
    },

    // ========================================================================
    // GREAT RUNES (Shardbearer rewards - VERY HIGH confidence)
    // Uses BOSS DEFEAT FLAGS - same flags as remembrances for shardbearers
    // ========================================================================
    UniqueItemMapping {
        item_id: 8148,
        name: "Godrick's Great Rune",
        event_flag: 10000800, // Boss defeat flag
        category: UniqueItemCategory::GreatRune,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 8149,
        name: "Radahn's Great Rune",
        event_flag: 1052380800, // Radahn defeat
        category: UniqueItemCategory::GreatRune,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 8150,
        name: "Morgott's Great Rune",
        event_flag: 11000800, // Morgott defeat
        category: UniqueItemCategory::GreatRune,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 8151,
        name: "Rykard's Great Rune",
        event_flag: 16000800, // Rykard defeat
        category: UniqueItemCategory::GreatRune,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 8152,
        name: "Mohg's Great Rune",
        event_flag: 12050800, // Mohg defeat
        category: UniqueItemCategory::GreatRune,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 8153,
        name: "Malenia's Great Rune",
        event_flag: 15000800, // Malenia defeat
        category: UniqueItemCategory::GreatRune,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 10080,
        name: "Great Rune of the Unborn",
        event_flag: 14000800, // Rennala defeat
        category: UniqueItemCategory::GreatRune,
        confidence: VerificationConfidence::VeryHigh,
    },

    // ========================================================================
    // WHETBLADES (Affinity unlock items - HIGH confidence)
    // Flags in 65xxx range
    // ========================================================================
    UniqueItemMapping {
        item_id: 8900,
        name: "Whetstone Knife",
        event_flag: 60130, // Early game item
        category: UniqueItemCategory::Whetblade,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8901,
        name: "Iron Whetblade",
        event_flag: 65000,
        category: UniqueItemCategory::Whetblade,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8902,
        name: "Glintstone Whetblade",
        event_flag: 65010,
        category: UniqueItemCategory::Whetblade,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8903,
        name: "Red-Hot Whetblade",
        event_flag: 65020,
        category: UniqueItemCategory::Whetblade,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8904,
        name: "Sanctified Whetblade",
        event_flag: 65030,
        category: UniqueItemCategory::Whetblade,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8905,
        name: "Black Whetblade",
        event_flag: 65040,
        category: UniqueItemCategory::Whetblade,
        confidence: VerificationConfidence::High,
    },

    // ========================================================================
    // COOKBOOKS (Crafting unlocks - HIGH confidence)
    // Flags in 67xxx-68xxx range, item IDs 8000-8099
    // ========================================================================
    UniqueItemMapping {
        item_id: 8000,
        name: "Armorer's Cookbook [1]",
        event_flag: 67000,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8001,
        name: "Armorer's Cookbook [2]",
        event_flag: 67010,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8002,
        name: "Armorer's Cookbook [3]",
        event_flag: 67020,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8003,
        name: "Armorer's Cookbook [4]",
        event_flag: 67030,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8004,
        name: "Armorer's Cookbook [5]",
        event_flag: 67040,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8005,
        name: "Armorer's Cookbook [6]",
        event_flag: 67050,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8006,
        name: "Armorer's Cookbook [7]",
        event_flag: 67060,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8010,
        name: "Glintstone Craftsman's Cookbook [1]",
        event_flag: 67100,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8011,
        name: "Glintstone Craftsman's Cookbook [2]",
        event_flag: 67110,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8012,
        name: "Glintstone Craftsman's Cookbook [3]",
        event_flag: 67120,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8013,
        name: "Glintstone Craftsman's Cookbook [4]",
        event_flag: 67130,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8014,
        name: "Glintstone Craftsman's Cookbook [5]",
        event_flag: 67140,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8015,
        name: "Glintstone Craftsman's Cookbook [6]",
        event_flag: 67150,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8016,
        name: "Glintstone Craftsman's Cookbook [7]",
        event_flag: 67160,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8017,
        name: "Glintstone Craftsman's Cookbook [8]",
        event_flag: 67170,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8020,
        name: "Missionary's Cookbook [1]",
        event_flag: 67200,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8021,
        name: "Missionary's Cookbook [2]",
        event_flag: 67210,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8022,
        name: "Missionary's Cookbook [3]",
        event_flag: 67220,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8023,
        name: "Missionary's Cookbook [4]",
        event_flag: 67230,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8024,
        name: "Missionary's Cookbook [5]",
        event_flag: 67240,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8025,
        name: "Missionary's Cookbook [6]",
        event_flag: 67250,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8026,
        name: "Missionary's Cookbook [7]",
        event_flag: 67260,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8030,
        name: "Nomadic Warrior's Cookbook [1]",
        event_flag: 67300,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8031,
        name: "Nomadic Warrior's Cookbook [2]",
        event_flag: 67310,
        category: UniqueItemCategory::Cookbook,
        confidence: VerificationConfidence::High,
    },
    // ... more cookbooks can be added

    // ========================================================================
    // KEY ITEMS (Progression items - HIGH confidence)
    // ========================================================================
    UniqueItemMapping {
        item_id: 8100,
        name: "Crafting Kit",
        event_flag: 60100,
        category: UniqueItemCategory::KeyItem,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8135,
        name: "Rold Medallion",
        event_flag: 60420,
        category: UniqueItemCategory::KeyItem,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 8175,
        name: "Haligtree Secret Medallion (Left)",
        event_flag: 60431,
        category: UniqueItemCategory::KeyItem,
        confidence: VerificationConfidence::VeryHigh,
    },
    UniqueItemMapping {
        item_id: 8176,
        name: "Haligtree Secret Medallion (Right)",
        event_flag: 60430,
        category: UniqueItemCategory::KeyItem,
        confidence: VerificationConfidence::VeryHigh,
    },

    // ========================================================================
    // ASHES OF WAR (Unique skill unlocks - HIGH confidence)
    // Flags in 540xxx range have verified formula (base 67500)
    // NOTE: 510xxx are CONSUMPTION flags (set when used at Enia), not pickup
    // ========================================================================
    // These use 510xxx (consumption flags) - set when USED, not when obtained
    // Marking as Low confidence because detection semantics differ
    UniqueItemMapping {
        item_id: 22100,
        name: "Ash of War: Black Flame Tornado",
        event_flag: 510140, // 510xxx = consumption flag, set when used
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::Low, // Consumption flag - detection != possession
    },
    UniqueItemMapping {
        item_id: 11800,
        name: "Ash of War: Loretta's Slash",
        event_flag: 510810, // 510xxx = consumption flag
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::Low, // Consumption flag - detection != possession
    },
    UniqueItemMapping {
        item_id: 30600,
        name: "Ash of War: Storm Wall",
        event_flag: 540100,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 11000,
        name: "Ash of War: Wild Strikes",
        event_flag: 540104,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 60000,
        name: "Ash of War: Determination",
        event_flag: 540108,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 11400,
        name: "Ash of War: Unsheathe",
        event_flag: 540112,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 50600,
        name: "Ash of War: Ground Slam",
        event_flag: 540116,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 20100,
        name: "Ash of War: Sacred Blade",
        event_flag: 540118,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 10700,
        name: "Ash of War: Stamp (Sweep)",
        event_flag: 540120,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 40200,
        name: "Ash of War: Mighty Shot",
        event_flag: 540140,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 12200,
        name: "Ash of War: Storm Assault",
        event_flag: 540170,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 12300,
        name: "Ash of War: Stormcaller",
        event_flag: 540172,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 22700,
        name: "Ash of War: Chilling Mist",
        event_flag: 540200,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 10500,
        name: "Ash of War: Charge Forth",
        event_flag: 540202,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 50100,
        name: "Ash of War: Hoarfrost Stomp",
        event_flag: 540204,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 31000,
        name: "Ash of War: Thops's Barrier",
        event_flag: 540206,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 70100,
        name: "Ash of War: Vow of the Indomitable",
        event_flag: 540208,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 30000,
        name: "Ash of War: Shield Bash",
        event_flag: 540210,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 65000,
        name: "Ash of War: Barbaric Roar",
        event_flag: 540224,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 12400,
        name: "Ash of War: Sword Dance",
        event_flag: 540238,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 22600,
        name: "Ash of War: Spectral Lance",
        event_flag: 540272,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 60400,
        name: "Ash of War: Sacred Order",
        event_flag: 540300,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 30800,
        name: "Ash of War: Shield Crash",
        event_flag: 540302,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 21200,
        name: "Ash of War: Earthshaker",
        event_flag: 540304,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 22400,
        name: "Ash of War: Blood Blade",
        event_flag: 540306,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 50700,
        name: "Ash of War: Golden Slam",
        event_flag: 540308,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 50400,
        name: "Ash of War: Lightning Ram",
        event_flag: 540310,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 20800,
        name: "Ash of War: Prayerful Strike",
        event_flag: 540314,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 30700,
        name: "Ash of War: Golden Parry",
        event_flag: 540316,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 21700,
        name: "Ash of War: Lightning Slash",
        event_flag: 540318,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 40100,
        name: "Ash of War: Barrage",
        event_flag: 540332,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 40000,
        name: "Ash of War: Through and Through",
        event_flag: 540334,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 21600,
        name: "Ash of War: Thunderbolt",
        event_flag: 540372,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 20500,
        name: "Ash of War: Lifesteal Fist",
        event_flag: 540402,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 22200,
        name: "Ash of War: Sacred Ring of Light",
        event_flag: 540404,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 22800,
        name: "Ash of War: Poisonous Mist",
        event_flag: 540406,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 21400,
        name: "Ash of War: Flaming Strike",
        event_flag: 540408,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 50500,
        name: "Ash of War: Flame of the Redmanes",
        event_flag: 540410,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 40500,
        name: "Ash of War: Sky Shot",
        event_flag: 540412,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 60700,
        name: "Ash of War: Cragblade",
        event_flag: 540414,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 11200,
        name: "Ash of War: Double Slash",
        event_flag: 540418,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 60600,
        name: "Ash of War: Seppuku",
        event_flag: 540510,
        category: UniqueItemCategory::AshOfWar,
        confidence: VerificationConfidence::High,
    },

    // ========================================================================
    // SPIRIT ASHES (Summon unlocks - LOW confidence)
    // FLAGS IN 520xxx RANGE - NO FORMULA EXISTS IN ground_truth_offsets.json
    // These items CANNOT be verified until 520000 block is discovered
    // Keeping for future when formula is discovered
    // ========================================================================
    UniqueItemMapping {
        item_id: 258000,
        name: "Lhutel the Headless",
        event_flag: 520000,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 234000,
        name: "Demi-Human Ashes",
        event_flag: 520010,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 241000,
        name: "Noble Sorcerer Ashes",
        event_flag: 520020,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 202000,
        name: "Banished Knight Engvall",
        event_flag: 520040,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 219000,
        name: "Twinsage Sorcerer Ashes",
        event_flag: 520050,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 218000,
        name: "Glintstone Sorcerer Ashes",
        event_flag: 520060,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 256000,
        name: "Ancient Dragon Knight Kristoff",
        event_flag: 520080,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 239000,
        name: "Bloodhound Knight Floh",
        event_flag: 520090,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 217000,
        name: "Perfumer Tricia",
        event_flag: 520110,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 246000,
        name: "Soldjars of Fortune Ashes",
        event_flag: 520130,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 243000,
        name: "Mad Pumpkin Head Ashes",
        event_flag: 520140,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 224000,
        name: "Kindred of Rot Ashes",
        event_flag: 520150,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 257000,
        name: "Redmane Knight Ogha",
        event_flag: 520160,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 228000,
        name: "Blackflame Monk Amon",
        event_flag: 520200,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 215000,
        name: "Putrid Corpse Ashes",
        event_flag: 520430,
        category: UniqueItemCategory::SpiritAsh,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },

    // ========================================================================
    // TALISMANS (Unique accessory unlocks - LOW confidence)
    // FLAGS IN 520xxx RANGE - NO FORMULA EXISTS IN ground_truth_offsets.json
    // These items CANNOT be verified until 520000 block is discovered
    // ========================================================================
    UniqueItemMapping {
        item_id: 5050,
        name: "Assassin's Crimson Dagger",
        event_flag: 520030,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 5060,
        name: "Assassin's Cerulean Dagger",
        event_flag: 520210,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 2160,
        name: "Lord of Blood's Exultation",
        event_flag: 520220,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 1020,
        name: "Viridian Amber Medallion",
        event_flag: 520300,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 4010,
        name: "Spelldrake Talisman",
        event_flag: 520310,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 4020,
        name: "Flamedrake Talisman",
        event_flag: 520330,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 2110,
        name: "Blue Dancer Charm",
        event_flag: 520350,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 2080,
        name: "Winged Sword Insignia",
        event_flag: 520360,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 1010,
        name: "Cerulean Amber Medallion",
        event_flag: 520370,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 2170,
        name: "Kindred of Rot's Exultation",
        event_flag: 520390,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 6010,
        name: "Concealing Veil",
        event_flag: 520420,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 4022,
        name: "Flamedrake Talisman +2",
        event_flag: 520440,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 1110,
        name: "Gold Scarab",
        event_flag: 520450,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 5040,
        name: "Godskin Swaddling Cloth",
        event_flag: 520480,
        category: UniqueItemCategory::Talisman,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },

    // ========================================================================
    // BOSS WEAPONS (Unique weapon drops - mixed confidence)
    // 510xxx = consumption/acquisition flags (verified formula: base 63750)
    // 520xxx = NO FORMULA EXISTS in ground_truth_offsets.json
    // ========================================================================
    // These use 510xxx (verified formula exists)
    UniqueItemMapping {
        item_id: 4100000,
        name: "Grafted Blade Greatsword",
        event_flag: 510800, // 510xxx: verified formula (base 63750)
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 3150000,
        name: "Marais Executioner's Sword",
        event_flag: 510820, // 510xxx: verified formula
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 4080000,
        name: "Ruins Greatsword",
        event_flag: 510830, // 510xxx: verified formula
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 21120000,
        name: "Veteran's Prosthesis",
        event_flag: 510840, // 510xxx: verified formula
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 8040000,
        name: "Magma Wyrm's Scalesword",
        event_flag: 510260, // 510xxx: verified formula
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 18100000,
        name: "Loretta's War Sickle",
        event_flag: 510190, // 510xxx: verified formula
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 6010000,
        name: "Godskin Stitcher",
        event_flag: 510210, // 510xxx: verified formula
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::High,
    },
    UniqueItemMapping {
        item_id: 16130000,
        name: "Inquisitor's Girandole",
        event_flag: 510290, // 510xxx: verified formula
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::High,
    },
    // These use 520xxx - NO FORMULA EXISTS
    UniqueItemMapping {
        item_id: 3060000,
        name: "Ordovis's Greatsword",
        event_flag: 520100,
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 8050000,
        name: "Zamor Curved Sword",
        event_flag: 520170,
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 44010000,
        name: "Jar Cannon",
        event_flag: 520400,
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 15020000,
        name: "Great Omenkiller Cleaver",
        event_flag: 520410,
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 3170000,
        name: "Golden Order Greatsword",
        event_flag: 520470,
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
    UniqueItemMapping {
        item_id: 13020000,
        name: "Family Heads",
        event_flag: 520490,
        category: UniqueItemCategory::BossWeapon,
        confidence: VerificationConfidence::Low, // 520xxx: No formula exists
    },
];

// ============================================================================
// INVENTORY VERIFICATION SERVICE
// ============================================================================

/// Service for verifying event flags against inventory possession
pub struct InventoryVerificationService;

impl InventoryVerificationService {
    /// Verify a single event flag against inventory
    pub fn verify_flag(
        flag_id: u32,
        flag_is_set: bool,
        inventory: &EquipInventoryData,
    ) -> InventoryVerificationResult {
        // Get expected items for this flag
        let expected_items = UNIQUE_ITEMS_BY_FLAG
            .get(&flag_id)
            .cloned()
            .unwrap_or_default();

        if expected_items.is_empty() {
            return InventoryVerificationResult {
                flag_id,
                expected_items: vec![],
                has_any_item: false,
                items_found: vec![],
                confidence: VerificationConfidence::Unknown,
                flag_matches_inventory: None,
            };
        }

        // Extract item IDs from inventory
        let inventory_items = Self::extract_inventory_item_ids(inventory);

        // Check which expected items are in inventory
        let mut items_found = Vec::new();
        let mut has_any = false;

        for mapping in &expected_items {
            if inventory_items.contains(&mapping.item_id) {
                has_any = true;
                items_found.push((mapping.item_id, mapping.name.to_string(), 1));
            }
        }

        // Determine best confidence from expected items
        let confidence = expected_items
            .iter()
            .map(|m| m.confidence)
            .min_by_key(|c| match c {
                VerificationConfidence::VeryHigh => 0,
                VerificationConfidence::High => 1,
                VerificationConfidence::Medium => 2,
                VerificationConfidence::Low => 3,
                VerificationConfidence::Unknown => 4,
            })
            .unwrap_or(VerificationConfidence::Unknown);

        // Check if flag matches inventory
        let flag_matches_inventory = Some(flag_is_set == has_any);

        InventoryVerificationResult {
            flag_id,
            expected_items,
            has_any_item: has_any,
            items_found,
            confidence,
            flag_matches_inventory,
        }
    }

    /// Verify multiple flags at once
    pub fn verify_flags(
        flags: &[(u32, bool)], // (flag_id, is_set)
        inventory: &EquipInventoryData,
    ) -> Vec<InventoryVerificationResult> {
        flags
            .iter()
            .map(|(flag_id, is_set)| Self::verify_flag(*flag_id, *is_set, inventory))
            .collect()
    }

    /// Get all unique items that should be in inventory based on set flags
    pub fn get_expected_inventory_from_flags(
        set_flags: &HashSet<u32>,
    ) -> Vec<&'static UniqueItemMapping> {
        let mut expected = Vec::new();

        for flag_id in set_flags {
            if let Some(mappings) = UNIQUE_ITEMS_BY_FLAG.get(flag_id) {
                for mapping in mappings {
                    expected.push(mapping);
                }
            }
        }

        expected
    }

    /// Get all flags that should be set based on inventory contents
    pub fn get_expected_flags_from_inventory(
        inventory: &EquipInventoryData,
    ) -> Vec<(u32, &'static UniqueItemMapping)> {
        let inventory_items = Self::extract_inventory_item_ids(inventory);
        let mut expected_flags = Vec::new();

        for item_id in inventory_items {
            if let Some(flag_ids) = FLAGS_BY_ITEM.get(&item_id) {
                for flag_id in flag_ids {
                    if let Some(mappings) = UNIQUE_ITEMS_BY_FLAG.get(flag_id) {
                        for mapping in mappings {
                            if mapping.item_id == item_id {
                                expected_flags.push((*flag_id, mapping));
                            }
                        }
                    }
                }
            }
        }

        expected_flags
    }

    /// Find mismatches between flags and inventory
    pub fn find_mismatches(
        set_flags: &HashSet<u32>,
        inventory: &EquipInventoryData,
    ) -> InventoryMismatchReport {
        let inventory_items = Self::extract_inventory_item_ids(inventory);

        let mut flag_set_no_item = Vec::new();
        let mut item_present_no_flag = Vec::new();
        let mut matches = Vec::new();

        // Check all unique items
        for mapping in UNIQUE_ITEMS.iter() {
            let flag_is_set = set_flags.contains(&mapping.event_flag);
            let has_item = inventory_items.contains(&mapping.item_id);

            match (flag_is_set, has_item) {
                (true, true) => matches.push(mapping),
                (true, false) => flag_set_no_item.push(mapping),
                (false, true) => item_present_no_flag.push(mapping),
                (false, false) => {} // Both absent, expected
            }
        }

        InventoryMismatchReport {
            flag_set_no_item,
            item_present_no_flag,
            matches,
            total_checked: UNIQUE_ITEMS.len(),
        }
    }

    /// Extract all item IDs from inventory (common + key items)
    fn extract_inventory_item_ids(inventory: &EquipInventoryData) -> HashSet<u32> {
        let mut items = HashSet::new();

        // Extract from common items
        for item in &inventory.common_items {
            if item.ga_item_handle != 0 {
                // ga_item_handle format: type_prefix | item_id
                // Type prefixes: 0x00000000 (weapon), 0x10000000 (armor),
                //                0x20000000 (accessory), 0x40000000 (goods)
                let item_id = item.ga_item_handle & 0x0FFFFFFF;
                items.insert(item_id);
            }
        }

        // Extract from key items
        for item in &inventory.key_items {
            if item.ga_item_handle != 0 {
                let item_id = item.ga_item_handle & 0x0FFFFFFF;
                items.insert(item_id);
            }
        }

        items
    }

    /// Get summary statistics
    pub fn get_verification_stats(
        set_flags: &HashSet<u32>,
        inventory: &EquipInventoryData,
    ) -> VerificationStats {
        let report = Self::find_mismatches(set_flags, inventory);

        let total_verifiable = report.matches.len()
            + report.flag_set_no_item.len()
            + report.item_present_no_flag.len();

        VerificationStats {
            total_unique_items: UNIQUE_ITEMS.len(),
            total_verifiable,
            matches: report.matches.len(),
            flag_set_no_item: report.flag_set_no_item.len(),
            item_present_no_flag: report.item_present_no_flag.len(),
            match_rate: if total_verifiable > 0 {
                report.matches.len() as f32 / total_verifiable as f32
            } else {
                0.0
            },
        }
    }
}

/// Report of mismatches between flags and inventory
#[derive(Debug)]
pub struct InventoryMismatchReport {
    /// Flags that are set but corresponding item not in inventory
    /// (Could mean: item was used/sold, or flag is false positive)
    pub flag_set_no_item: Vec<&'static UniqueItemMapping>,

    /// Items in inventory but corresponding flag not set
    /// (Could mean: flag formula bug, or item obtained differently)
    pub item_present_no_flag: Vec<&'static UniqueItemMapping>,

    /// Flags and items that match (both present or both absent)
    pub matches: Vec<&'static UniqueItemMapping>,

    /// Total items checked
    pub total_checked: usize,
}

/// Summary statistics for verification
#[derive(Debug, Clone)]
pub struct VerificationStats {
    pub total_unique_items: usize,
    pub total_verifiable: usize,
    pub matches: usize,
    pub flag_set_no_item: usize,
    pub item_present_no_flag: usize,
    pub match_rate: f32,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_items_database() {
        // Verify database is populated (now with 120+ items)
        assert!(UNIQUE_ITEMS.len() > 100, "Should have at least 100 unique items, got {}", UNIQUE_ITEMS.len());

        // Check remembrances
        let remembrances: Vec<_> = UNIQUE_ITEMS
            .iter()
            .filter(|i| i.category == UniqueItemCategory::Remembrance)
            .collect();
        assert!(remembrances.len() >= 10, "Should have at least 10 remembrances");

        // Check great runes
        let great_runes: Vec<_> = UNIQUE_ITEMS
            .iter()
            .filter(|i| i.category == UniqueItemCategory::GreatRune)
            .collect();
        assert!(great_runes.len() >= 6, "Should have at least 6 great runes");

        // Check ashes of war
        let ashes_of_war: Vec<_> = UNIQUE_ITEMS
            .iter()
            .filter(|i| i.category == UniqueItemCategory::AshOfWar)
            .collect();
        assert!(ashes_of_war.len() >= 30, "Should have at least 30 ashes of war, got {}", ashes_of_war.len());

        // Check spirit ashes
        let spirit_ashes: Vec<_> = UNIQUE_ITEMS
            .iter()
            .filter(|i| i.category == UniqueItemCategory::SpiritAsh)
            .collect();
        assert!(spirit_ashes.len() >= 10, "Should have at least 10 spirit ashes, got {}", spirit_ashes.len());

        // Check talismans
        let talismans: Vec<_> = UNIQUE_ITEMS
            .iter()
            .filter(|i| i.category == UniqueItemCategory::Talisman)
            .collect();
        assert!(talismans.len() >= 10, "Should have at least 10 talismans, got {}", talismans.len());
    }

    #[test]
    fn test_flag_lookup() {
        // Godrick's Great Rune maps to Godrick's DEFEAT flag (10000800), not the
        // old <50k world-drop flag 171 — see the 2026-07-24 remembrance/great-rune
        // cutover to boss-defeat flags read via world_flag_state.
        let mappings = UNIQUE_ITEMS_BY_FLAG.get(&10000800).unwrap();
        assert!(mappings.iter().any(|m| m.item_id == 8148));
    }

    #[test]
    fn test_item_lookup() {
        // Item 8148 (Godrick's Great Rune) maps to Godrick's defeat flag 10000800.
        let flags = FLAGS_BY_ITEM.get(&8148).unwrap();
        assert!(flags.contains(&10000800));
    }

    #[test]
    fn test_confidence_levels() {
        // Remembrances should be VeryHigh
        let remembrance = UNIQUE_ITEMS
            .iter()
            .find(|i| i.category == UniqueItemCategory::Remembrance)
            .unwrap();
        assert_eq!(remembrance.confidence, VerificationConfidence::VeryHigh);

        // Cookbooks should be High
        let cookbook = UNIQUE_ITEMS
            .iter()
            .find(|i| i.category == UniqueItemCategory::Cookbook)
            .unwrap();
        assert_eq!(cookbook.confidence, VerificationConfidence::High);
    }
}
