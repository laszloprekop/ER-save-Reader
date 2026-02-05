//! Boss Drops Database
//!
//! Comprehensive database of items dropped by bosses, including:
//! - Remembrances
//! - Weapons
//! - Ashes of War
//! - Talismans
//! - Spirit Ashes
//! - Key Items

/// Drop category for boss rewards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropCategory {
    Remembrance,
    GreatRune,
    Weapon,
    AshOfWar,
    Talisman,
    SpiritAsh,
    KeyItem,
    Incantation,
    Sorcery,
    Other,
}

impl DropCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            DropCategory::Remembrance => "Remembrance",
            DropCategory::GreatRune => "Great Rune",
            DropCategory::Weapon => "Weapon",
            DropCategory::AshOfWar => "Ash of War",
            DropCategory::Talisman => "Talisman",
            DropCategory::SpiritAsh => "Spirit Ash",
            DropCategory::KeyItem => "Key Item",
            DropCategory::Incantation => "Incantation",
            DropCategory::Sorcery => "Sorcery",
            DropCategory::Other => "Other",
        }
    }
}

/// A boss drop entry
#[derive(Debug, Clone)]
pub struct BossDrop {
    pub boss_flag: u32,
    pub boss_name: &'static str,
    pub item_id: u32,
    pub item_name: &'static str,
    pub category: DropCategory,
}

/// All boss drops (excluding remembrance exchange rewards)
pub static BOSS_DROPS: &[BossDrop] = &[
    // ===== DEMIGODS =====
    // Godrick the Grafted
    BossDrop { boss_flag: 10000800, boss_name: "Godrick the Grafted", item_id: 8150, item_name: "Remembrance of the Grafted", category: DropCategory::Remembrance },
    BossDrop { boss_flag: 10000800, boss_name: "Godrick the Grafted", item_id: 8101, item_name: "Godrick's Great Rune", category: DropCategory::GreatRune },

    // Rennala, Queen of the Full Moon
    BossDrop { boss_flag: 14000800, boss_name: "Rennala, Queen of the Full Moon", item_id: 8151, item_name: "Remembrance of the Full Moon Queen", category: DropCategory::Remembrance },
    BossDrop { boss_flag: 14000800, boss_name: "Rennala, Queen of the Full Moon", item_id: 8103, item_name: "Great Rune of the Unborn", category: DropCategory::GreatRune },

    // Starscourge Radahn
    BossDrop { boss_flag: 12010800, boss_name: "Starscourge Radahn", item_id: 8153, item_name: "Remembrance of the Starscourge", category: DropCategory::Remembrance },
    BossDrop { boss_flag: 12010800, boss_name: "Starscourge Radahn", item_id: 8105, item_name: "Radahn's Great Rune", category: DropCategory::GreatRune },

    // Rykard, Lord of Blasphemy
    BossDrop { boss_flag: 16000800, boss_name: "Rykard, Lord of Blasphemy", item_id: 8152, item_name: "Remembrance of the Blasphemous", category: DropCategory::Remembrance },
    BossDrop { boss_flag: 16000800, boss_name: "Rykard, Lord of Blasphemy", item_id: 8104, item_name: "Rykard's Great Rune", category: DropCategory::GreatRune },

    // Morgott, the Omen King
    BossDrop { boss_flag: 11000800, boss_name: "Morgott, the Omen King", item_id: 8156, item_name: "Remembrance of the Omen King", category: DropCategory::Remembrance },
    BossDrop { boss_flag: 11000800, boss_name: "Morgott, the Omen King", item_id: 8102, item_name: "Morgott's Great Rune", category: DropCategory::GreatRune },

    // Mohg, Lord of Blood
    BossDrop { boss_flag: 12050800, boss_name: "Mohg, Lord of Blood", item_id: 8155, item_name: "Remembrance of the Blood Lord", category: DropCategory::Remembrance },
    BossDrop { boss_flag: 12050800, boss_name: "Mohg, Lord of Blood", item_id: 8107, item_name: "Mohg's Great Rune", category: DropCategory::GreatRune },

    // Malenia, Blade of Miquella
    BossDrop { boss_flag: 15000800, boss_name: "Malenia, Blade of Miquella", item_id: 8154, item_name: "Remembrance of the Rot Goddess", category: DropCategory::Remembrance },
    BossDrop { boss_flag: 15000800, boss_name: "Malenia, Blade of Miquella", item_id: 8106, item_name: "Malenia's Great Rune", category: DropCategory::GreatRune },

    // ===== GREAT BOSSES =====
    // Maliketh, the Black Blade
    BossDrop { boss_flag: 13000800, boss_name: "Maliketh, the Black Blade", item_id: 8157, item_name: "Remembrance of the Black Blade", category: DropCategory::Remembrance },

    // Hoarah Loux, Warrior
    BossDrop { boss_flag: 11050800, boss_name: "Hoarah Loux, Warrior", item_id: 8159, item_name: "Remembrance of Hoarah Loux", category: DropCategory::Remembrance },

    // Radagon / Elden Beast
    BossDrop { boss_flag: 19000800, boss_name: "Radagon / Elden Beast", item_id: 8158, item_name: "Elden Remembrance", category: DropCategory::Remembrance },

    // Fire Giant
    BossDrop { boss_flag: 1052520800, boss_name: "Fire Giant", item_id: 8164, item_name: "Remembrance of the Fire Giant", category: DropCategory::Remembrance },

    // Dragonlord Placidusax
    BossDrop { boss_flag: 13000830, boss_name: "Dragonlord Placidusax", item_id: 8161, item_name: "Remembrance of the Dragonlord", category: DropCategory::Remembrance },

    // Lichdragon Fortissax
    BossDrop { boss_flag: 12030850, boss_name: "Lichdragon Fortissax", item_id: 8162, item_name: "Remembrance of the Lichdragon", category: DropCategory::Remembrance },

    // Astel, Naturalborn of the Void
    BossDrop { boss_flag: 12040800, boss_name: "Astel, Naturalborn of the Void", item_id: 8165, item_name: "Remembrance of the Naturalborn", category: DropCategory::Remembrance },

    // Regal Ancestor Spirit
    BossDrop { boss_flag: 12020800, boss_name: "Regal Ancestor Spirit", item_id: 8163, item_name: "Remembrance of the Regal Ancestor", category: DropCategory::Remembrance },

    // ===== FIELD BOSSES - WEAPONS =====
    // Leonine Misbegotten
    BossDrop { boss_flag: 1043300800, boss_name: "Leonine Misbegotten", item_id: 21100000, item_name: "Grafted Blade Greatsword", category: DropCategory::Weapon },

    // Bloodhound Knight Darriwil
    BossDrop { boss_flag: 1044360800, boss_name: "Bloodhound Knight Darriwil", item_id: 17020000, item_name: "Bloodhound's Fang", category: DropCategory::Weapon },

    // Tree Sentinel
    BossDrop { boss_flag: 1042380850, boss_name: "Tree Sentinel", item_id: 15110000, item_name: "Golden Halberd", category: DropCategory::Weapon },

    // Grave Warden Duelist (Murkwater)
    BossDrop { boss_flag: 30000800, boss_name: "Grave Warden Duelist", item_id: 9000000, item_name: "Battle Hammer", category: DropCategory::Weapon },

    // Scaly Misbegotten (Morne Tunnel)
    BossDrop { boss_flag: 32010800, boss_name: "Scaly Misbegotten", item_id: 17170000, item_name: "Rusted Anchor", category: DropCategory::Weapon },

    // Full-Grown Fallingstar Beast
    BossDrop { boss_flag: 1037530800, boss_name: "Full-Grown Fallingstar Beast", item_id: 21150000, item_name: "Fallingstar Beast Jaw", category: DropCategory::Weapon },

    // Commander O'Neil
    BossDrop { boss_flag: 1049380800, boss_name: "Commander O'Neil", item_id: 15140000, item_name: "Commander's Standard", category: DropCategory::Weapon },

    // Valiant Gargoyles
    BossDrop { boss_flag: 12020800, boss_name: "Valiant Gargoyles", item_id: 12040000, item_name: "Gargoyle's Greatsword", category: DropCategory::Weapon },
    BossDrop { boss_flag: 12020800, boss_name: "Valiant Gargoyles", item_id: 12060000, item_name: "Gargoyle's Twinblade", category: DropCategory::Weapon },

    // Magma Wyrm Makar
    BossDrop { boss_flag: 39200800, boss_name: "Magma Wyrm Makar", item_id: 21040000, item_name: "Magma Wyrm's Scalesword", category: DropCategory::Weapon },

    // ===== FIELD BOSSES - KEY ITEMS =====
    // Red Wolf of Radagon
    BossDrop { boss_flag: 14000850, boss_name: "Red Wolf of Radagon", item_id: 8010, item_name: "Memory Stone", category: DropCategory::KeyItem },

    // Flying Dragon Agheel
    BossDrop { boss_flag: 1044350800, boss_name: "Flying Dragon Agheel", item_id: 8000, item_name: "Dragon Heart", category: DropCategory::KeyItem },

    // Glintstone Dragon Smarag
    BossDrop { boss_flag: 1034450800, boss_name: "Glintstone Dragon Smarag", item_id: 8000, item_name: "Dragon Heart", category: DropCategory::KeyItem },

    // Decaying Ekzykes
    BossDrop { boss_flag: 1048370800, boss_name: "Decaying Ekzykes", item_id: 8000, item_name: "Dragon Heart", category: DropCategory::KeyItem },

    // Godfrey, First Elden Lord (Shade)
    BossDrop { boss_flag: 11000850, boss_name: "Godfrey, First Elden Lord (Shade)", item_id: 8011, item_name: "Talisman Pouch", category: DropCategory::KeyItem },

    // Commander Niall
    BossDrop { boss_flag: 1051560800, boss_name: "Commander Niall", item_id: 1350, item_name: "Veteran's Prosthesis", category: DropCategory::Weapon },

    // Mimic Tear
    BossDrop { boss_flag: 12070800, boss_name: "Mimic Tear", item_id: 1980, item_name: "Larval Tear", category: DropCategory::KeyItem },

    // ===== TALISMANS =====
    // Ancestor Spirit
    BossDrop { boss_flag: 12080800, boss_name: "Ancestor Spirit", item_id: 1080, item_name: "Ancestral Spirit's Horn", category: DropCategory::Talisman },

    // Ancient Hero of Zamor (Weeping)
    BossDrop { boss_flag: 1042330800, boss_name: "Ancient Hero of Zamor (Weeping)", item_id: 1020, item_name: "Radagon's Scarseal", category: DropCategory::Talisman },

    // Spirit-Caller Snail (Spiritcaller Cave)
    BossDrop { boss_flag: 31190800, boss_name: "Spirit-Caller Snail", item_id: 1200, item_name: "Godskin Swaddling Cloth", category: DropCategory::Talisman },

    // ===== INCANTATIONS & SORCERIES =====
    // Crucible Knight (Stormhill Evergaol)
    BossDrop { boss_flag: 1042380800, boss_name: "Crucible Knight (Stormhill)", item_id: 4040, item_name: "Aspects of the Crucible: Tail", category: DropCategory::Incantation },

    // Crucible Knight Ordovis
    BossDrop { boss_flag: 30070800, boss_name: "Crucible Knight Ordovis", item_id: 310200, item_name: "Ordovis's Vortex", category: DropCategory::Incantation },

    // Royal Knight Loretta (Caria Manor)
    BossDrop { boss_flag: 1035500800, boss_name: "Royal Knight Loretta (Caria)", item_id: 4003, item_name: "Loretta's Greatbow", category: DropCategory::Sorcery },

    // Mohg, the Omen
    BossDrop { boss_flag: 35000800, boss_name: "Mohg, the Omen", item_id: 4370, item_name: "Bloodflame Talons", category: DropCategory::Incantation },

    // ===== SPIRIT ASHES =====
    // Cemetery Shade (Tombsward)
    BossDrop { boss_flag: 30030800, boss_name: "Cemetery Shade (Tombsward)", item_id: 410000, item_name: "Lhutel the Headless", category: DropCategory::SpiritAsh },

    // Ancestor Spirit
    BossDrop { boss_flag: 12080800, boss_name: "Ancestor Spirit", item_id: 419000, item_name: "Ancestral Follower Ashes", category: DropCategory::SpiritAsh },

    // Alecto, Black Knife Ringleader
    BossDrop { boss_flag: 1050570800, boss_name: "Alecto, Black Knife Ringleader", item_id: 424000, item_name: "Black Knife Tiche", category: DropCategory::SpiritAsh },

    // Ancient Hero of Zamor (Sainted Hero's Grave)
    BossDrop { boss_flag: 30190800, boss_name: "Ancient Hero of Zamor (Sainted)", item_id: 417000, item_name: "Ancient Dragon Knight Kristoff", category: DropCategory::SpiritAsh },

    // ===== ASHES OF WAR =====
    // Night's Cavalry - various locations
    BossDrop { boss_flag: 1042370800, boss_name: "Night's Cavalry (Agheel Lake)", item_id: 22000200, item_name: "Ash of War: Repeating Thrust", category: DropCategory::AshOfWar },
    BossDrop { boss_flag: 1048380800, boss_name: "Night's Cavalry (Caelid)", item_id: 22000400, item_name: "Ash of War: Poison Moth Flight", category: DropCategory::AshOfWar },
    BossDrop { boss_flag: 1037500800, boss_name: "Night's Cavalry (Liurnia)", item_id: 22000800, item_name: "Ash of War: Ice Spear", category: DropCategory::AshOfWar },
    BossDrop { boss_flag: 1040510800, boss_name: "Night's Cavalry (Altus)", item_id: 22001400, item_name: "Ash of War: Shared Order", category: DropCategory::AshOfWar },
];

/// Get all drops for a specific boss
pub fn get_drops_for_boss(boss_flag: u32) -> Vec<&'static BossDrop> {
    BOSS_DROPS.iter().filter(|d| d.boss_flag == boss_flag).collect()
}

/// Get all bosses that drop a specific item
pub fn get_bosses_for_item(item_id: u32) -> Vec<&'static BossDrop> {
    BOSS_DROPS.iter().filter(|d| d.item_id == item_id).collect()
}

/// Get drops by category
pub fn get_drops_by_category(category: DropCategory) -> Vec<&'static BossDrop> {
    BOSS_DROPS.iter().filter(|d| d.category == category).collect()
}

/// Get unique boss names
pub fn get_unique_bosses() -> Vec<&'static str> {
    let mut bosses: Vec<_> = BOSS_DROPS.iter()
        .map(|d| d.boss_name)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    bosses.sort();
    bosses
}
