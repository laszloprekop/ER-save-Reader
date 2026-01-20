/// Comprehensive Event Flags Database with Region Resolution and JSON Export
///
/// This module provides a reference database of all known event flags
/// with their categories, region/location information, and optional coordinates.
/// Data is consolidated from all existing database modules.

pub mod event_flags_db {
    use once_cell::sync::Lazy;
    use std::collections::HashSet;
    use serde::{Serialize, Deserialize};

    use crate::db::pickup_data::{WORLD_PICKUPS, PickupCategory};
    use crate::db::graces::maps::GRACES;
    use crate::db::bosses::bosses::BOSSES;
    use crate::db::cookbooks::books::COOKBOKS;
    use crate::db::whetblades::whetblades::WHETBLADES;
    use crate::db::landmarks::landmarks::LANDMARKS;
    use crate::db::map_name::map_name::MapName;

    /// Event flag categories based on flag ID ranges and purposes
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum EventFlagCategory {
        GreatRune,       // 100-199: Great Rune possession/activation
        BossDefeat,      // Boss defeat markers
        Remembrance,     // 9100-9199: Boss remembrance possession
        MapFragment,     // 62010-62084: Map fragment discovery
        Landmark,        // 62100-62999: Map landmarks (POIs)
        Grace,           // Grace site discovery
        Cookbook,        // 67000-68950: Cookbook pickups
        Whetblade,       // 65000-65720: Whetstone blade unlocks
        PotUpgrade,      // 66000-66990: Pot capacity upgrades
        TalismanPouch,   // 9200-9281: Talisman pouch upgrades
        WorldPickup,     // 10XXYYZZZZ: Open world item pickups
        DungeonPickup,   // 8-digit: Legacy dungeon pickups
        DLCPickup,       // 20XXYYZZZZ: DLC area pickups
        ShopStock,       // Shop purchase flags
        ShopUnlock,      // Shop unlock conditions
        NpcState,        // NPC progression flags
        SummoningPool,   // Summoning pool activation
        Colosseum,       // Arena unlock flags
        Progression,     // 60000-60520: General progression
        System,          // Internal game system flags
        Unknown,         // Unclassified flags
    }

    impl EventFlagCategory {
        pub fn name(&self) -> &'static str {
            match self {
                EventFlagCategory::GreatRune => "Great Rune",
                EventFlagCategory::BossDefeat => "Boss Defeat",
                EventFlagCategory::Remembrance => "Remembrance",
                EventFlagCategory::MapFragment => "Map Fragment",
                EventFlagCategory::Landmark => "Landmark",
                EventFlagCategory::Grace => "Grace",
                EventFlagCategory::Cookbook => "Cookbook",
                EventFlagCategory::Whetblade => "Whetblade",
                EventFlagCategory::PotUpgrade => "Pot Upgrade",
                EventFlagCategory::TalismanPouch => "Talisman Pouch",
                EventFlagCategory::WorldPickup => "World Pickup",
                EventFlagCategory::DungeonPickup => "Dungeon Pickup",
                EventFlagCategory::DLCPickup => "DLC Pickup",
                EventFlagCategory::ShopStock => "Shop Stock",
                EventFlagCategory::ShopUnlock => "Shop Unlock",
                EventFlagCategory::NpcState => "NPC State",
                EventFlagCategory::SummoningPool => "Summoning Pool",
                EventFlagCategory::Colosseum => "Colosseum",
                EventFlagCategory::Progression => "Progression",
                EventFlagCategory::System => "System",
                EventFlagCategory::Unknown => "Unknown",
            }
        }

        pub fn all() -> &'static [EventFlagCategory] {
            &[
                EventFlagCategory::GreatRune,
                EventFlagCategory::BossDefeat,
                EventFlagCategory::Remembrance,
                EventFlagCategory::MapFragment,
                EventFlagCategory::Landmark,
                EventFlagCategory::Grace,
                EventFlagCategory::Cookbook,
                EventFlagCategory::Whetblade,
                EventFlagCategory::PotUpgrade,
                EventFlagCategory::TalismanPouch,
                EventFlagCategory::WorldPickup,
                EventFlagCategory::DungeonPickup,
                EventFlagCategory::DLCPickup,
                EventFlagCategory::ShopStock,
                EventFlagCategory::ShopUnlock,
                EventFlagCategory::NpcState,
                EventFlagCategory::SummoningPool,
                EventFlagCategory::Colosseum,
                EventFlagCategory::Progression,
                EventFlagCategory::System,
                EventFlagCategory::Unknown,
            ]
        }
    }

    /// World coordinates for a flag location
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub struct Coordinates {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    /// A single event flag entry (owned version for building)
    #[derive(Debug, Clone)]
    pub struct EventFlagEntryOwned {
        pub flag_id: u32,
        pub name: String,
        pub category: EventFlagCategory,
        pub region: String,
        pub coords: Option<Coordinates>,
    }

    /// A single event flag entry for JSON export
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct EventFlagExport {
        pub flag_id: u32,
        pub name: String,
        pub category: String,
        pub region: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub x: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub y: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub z: Option<f32>,
    }

    impl From<&EventFlagEntryOwned> for EventFlagExport {
        fn from(entry: &EventFlagEntryOwned) -> Self {
            EventFlagExport {
                flag_id: entry.flag_id,
                name: entry.name.clone(),
                category: entry.category.name().to_string(),
                region: entry.region.clone(),
                x: entry.coords.map(|c| c.x),
                y: entry.coords.map(|c| c.y),
                z: entry.coords.map(|c| c.z),
            }
        }
    }

    /// Convert MapName to region string
    fn map_name_to_region(map: MapName) -> &'static str {
        match map {
            MapName::Limgrave => "Limgrave",
            MapName::WeepingPeninsula => "Weeping Peninsula",
            MapName::Stormhill => "Stormhill",
            MapName::LiurniaOfTheLakes => "Liurnia of the Lakes",
            MapName::Caelid => "Caelid",
            MapName::GreyollsDragonbarrow => "Dragonbarrow",
            MapName::AltusPlateau => "Altus Plateau",
            MapName::MtGelmir => "Mt. Gelmir",
            MapName::MountaintopsOfTheGiants => "Mountaintops of the Giants",
            MapName::ConsecratedSnowfield => "Consecrated Snowfield",
            MapName::StormveilCastle => "Stormveil Castle",
            MapName::AcademyOfRayaLucaria => "Academy of Raya Lucaria",
            MapName::VolcanoManor => "Volcano Manor",
            MapName::LeyndellRoyalCapital => "Leyndell, Royal Capital",
            MapName::LeyndellAshenCapital => "Leyndell, Ashen Capital",
            MapName::CrumblingFarumAzula => "Crumbling Farum Azula",
            MapName::MiquellasHaligtree => "Miquella's Haligtree",
            MapName::ElphaelBraceOfTheHaligtree => "Elphael, Brace of the Haligtree",
            MapName::SiofraRiver => "Siofra River",
            MapName::AinselRiver => "Ainsel River",
            MapName::AinselRiverMain => "Ainsel River Main",
            MapName::DeeprootDepths => "Deeproot Depths",
            MapName::LakeOfRot => "Lake of Rot",
            MapName::NokronEternalCity => "Nokron, Eternal City",
            MapName::MohgwynPalace => "Mohgwyn Palace",
            MapName::RoundtableHold => "Roundtable Hold",
            MapName::StrandedGraveyard => "Stranded Graveyard",
            MapName::BellumHighway => "Bellum Highway",
            MapName::RuinStrewnPrecipice => "Ruin-Strewn Precipice",
            MapName::MoonlightAltar => "Moonlight Altar",
            MapName::SubterraneanShunningGrounds => "Subterranean Shunning-Grounds",
            MapName::CapitalOutskirts => "Capital Outskirts",
            MapName::SwampOfAeonia => "Swamp of Aeonia",
            MapName::ForbiddenLands => "Forbidden Lands",
            MapName::FlamePeak => "Flame Peak",
            MapName::StonePlatform => "Stone Platform",
            // DLC - Realm of Shadow
            MapName::RealmOfShadowGravesitePlain => "Shadow of the Erdtree - Gravesite Plain",
            MapName::RealmOfShadowScaduAltus => "Shadow of the Erdtree - Scadu Altus",
            MapName::RealmOfShadowCeruleanCoast => "Shadow of the Erdtree - Cerulean Coast",
            MapName::RealmOfShadowJaggedPeak => "Shadow of the Erdtree - Jagged Peak",
            MapName::RealmOfShadowRauh => "Shadow of the Erdtree - Ancient Ruins of Rauh",
            MapName::RealmOfShadowAbyssalWoods => "Shadow of the Erdtree - Abyssal Woods",
            MapName::RealmOfShadowScaduview => "Shadow of the Erdtree - Scaduview",
            MapName::RealmOfShadowBelurat => "Shadow of the Erdtree - Belurat",
            MapName::RealmOfShadowShadowKeep => "Shadow of the Erdtree - Shadow Keep",
            MapName::RealmOfShadowStorehouse => "Shadow of the Erdtree - Storehouse",
            MapName::RealmOfShadowEnirIlim => "Shadow of the Erdtree - Enir-Ilim",
            MapName::RealmOfShadowCastleEnsis => "Shadow of the Erdtree - Castle Ensis",
            MapName::RealmOfShadowMidrasManse => "Shadow of the Erdtree - Midra's Manse",
            MapName::RealmOfShadowStoneCoffinFissure => "Shadow of the Erdtree - Stone Coffin Fissure",
            MapName::RealmOfShadowCharosGrave => "Shadow of the Erdtree - Charo's Hidden Grave",
        }
    }

    /// Resolve region name for a flag ID based on its format
    pub fn resolve_region(flag_id: u32) -> &'static str {
        // 10-digit base game world flags (1000000000+)
        if flag_id >= 1_000_000_000 && flag_id < 2_000_000_000 {
            let tile_index = (flag_id - 1_000_000_000) / 10000;
            let tile_x = (tile_index / 100) as u32;
            let tile_y = (tile_index % 100) as u32;
            return get_region_name(tile_x, tile_y);
        }

        // 10-digit DLC flags (2000000000+)
        if flag_id >= 2_000_000_000 {
            return "Shadow of the Erdtree";
        }

        // 8-digit dungeon flags (10000000-43999999)
        if flag_id >= 10_000_000 && flag_id < 44_000_000 {
            let map_area = flag_id / 1_000_000;
            let section = (flag_id / 10_000) % 100;
            return get_dungeon_name(map_area, section);
        }

        // Landmark flags (62100-62999)
        if flag_id >= 62100 && flag_id < 63000 {
            return get_landmark_region(flag_id);
        }

        // System/progression flags - no specific region
        ""
    }

    /// Get region name for landmark flags based on flag ID ranges
    fn get_landmark_region(flag_id: u32) -> &'static str {
        match flag_id {
            // Limgrave & Stormveil
            62100..=62138 => "Limgrave",
            // Weeping Peninsula
            62150..=62184 => "Weeping Peninsula",
            // Liurnia of the Lakes
            62200..=62284 => "Liurnia of the Lakes",
            // Altus Plateau
            62300..=62348 => "Altus Plateau",
            // Mt. Gelmir
            62350..=62389 => "Mt. Gelmir",
            // Caelid
            62400..=62438 => "Caelid",
            // Greyoll's Dragonbarrow
            62460..=62475 => "Greyoll's Dragonbarrow",
            // Mountaintops of the Giants
            62510..=62531 => "Mountaintops of the Giants",
            // Consecrated Snowfield
            62550..=62574 => "Consecrated Snowfield",
            // Siofra River
            62610..=62634 => "Siofra River",
            // Ainsel River
            62640..=62640 => "Ainsel River",
            // Deeproot Depths
            62700..=62740 => "Deeproot Depths",
            // Mohgwyn Palace
            62800..=62831 => "Mohgwyn Palace",
            // Lake of Rot
            62840..=62844 => "Lake of Rot",
            // Nokron / Nokstella
            62850..=62891 => "Nokron / Nokstella",
            // Leyndell
            62900..=62943 => "Leyndell",
            // Crumbling Farum Azula
            62950..=62981 => "Crumbling Farum Azula",
            // DLC areas (if any in this range)
            _ => ""
        }
    }

    /// Get region name from tile coordinates
    fn get_region_name(tile_x: u32, tile_y: u32) -> &'static str {
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

    /// Get dungeon name from map area and section
    fn get_dungeon_name(map_area: u32, _section: u32) -> &'static str {
        match map_area {
            10 => "Stormveil Castle",
            11 => "Leyndell, Royal Capital",
            12 => "Underground",
            13 => "Crumbling Farum Azula",
            14 => "Academy of Raya Lucaria",
            15 => "Caria Manor",
            16 => "Volcano Manor",
            18 => "Roundtable Hold",
            19 => "Chapel of Anticipation",
            20 => "Stranded Graveyard",
            21 => "Miquella's Haligtree",
            22 => "Castle Sol",
            30 => "Catacombs",
            31 => "Cave",
            32 => "Tunnel",
            34 => "Divine Tower",
            35 => "Mohgwyn Palace",
            39 => "Elden Throne",
            40 => "Hero's Grave",
            41 => "Minor Dungeon",
            42 => "Crystal Cave",
            43 => "Evergaol",
            _ => "Unknown Dungeon",
        }
    }

    /// Normalize region names for consistency
    fn normalize_region(region: &str) -> String {
        match region {
            "AltusPlateau" => "Altus Plateau".to_string(),
            "MtGelmir" => "Mt. Gelmir".to_string(),
            "MountaintopsOfTheGiants" => "Mountaintops of the Giants".to_string(),
            "ConsecratedSnowfield" => "Consecrated Snowfield".to_string(),
            "WeepingPeninsula" => "Weeping Peninsula".to_string(),
            "LiurniaOfTheLakes" => "Liurnia of the Lakes".to_string(),
            "SiofraRiver" => "Siofra River".to_string(),
            "AinselRiver" => "Ainsel River".to_string(),
            "DeeprootDepths" => "Deeproot Depths".to_string(),
            "LakeOfRot" => "Lake of Rot".to_string(),
            "NokronEternalCity" => "Nokron, Eternal City".to_string(),
            "NokstellaEternalCity" => "Nokstella, Eternal City".to_string(),
            "MohgwynPalace" => "Mohgwyn Palace".to_string(),
            "RoundtableHold" => "Roundtable Hold".to_string(),
            "StrandedGraveyard" => "Stranded Graveyard".to_string(),
            "StormveilCastle" => "Stormveil Castle".to_string(),
            "RayaLucaria" => "Academy of Raya Lucaria".to_string(),
            "VolcanoManor" => "Volcano Manor".to_string(),
            "CrumblingFarumAzula" => "Crumbling Farum Azula".to_string(),
            "MiquellaHaligtree" => "Miquella's Haligtree".to_string(),
            _ => region.to_string(),
        }
    }

    /// Get unique regions from the database
    pub fn get_unique_regions() -> Vec<String> {
        let mut regions: HashSet<String> = HashSet::new();
        for entry in EVENT_FLAGS_DB.iter() {
            regions.insert(entry.region.clone());
        }
        let mut regions_vec: Vec<String> = regions.into_iter().collect();
        regions_vec.sort();
        regions_vec
    }

    /// Export the database to JSON
    pub fn export_to_json() -> Result<String, serde_json::Error> {
        let exports: Vec<EventFlagExport> = EVENT_FLAGS_DB
            .iter()
            .map(|e| EventFlagExport::from(e))
            .collect();
        serde_json::to_string_pretty(&exports)
    }

    /// Export filtered entries to JSON
    pub fn export_filtered_to_json(entries: &[&EventFlagEntryOwned]) -> Result<String, serde_json::Error> {
        let exports: Vec<EventFlagExport> = entries
            .iter()
            .map(|e| EventFlagExport::from(*e))
            .collect();
        serde_json::to_string_pretty(&exports)
    }

    /// The comprehensive event flags database
    pub static EVENT_FLAGS_DB: Lazy<Vec<EventFlagEntryOwned>> = Lazy::new(|| {
        let mut entries: Vec<EventFlagEntryOwned> = Vec::with_capacity(15000);
        let mut seen_flags: HashSet<u32> = HashSet::new();

        // ========================================================================
        // IMPORT FROM WORLD PICKUPS (pickup_data.rs) - ~4809 entries
        // ========================================================================
        for pickup in WORLD_PICKUPS.iter() {
            if pickup.event_flag == 0 || seen_flags.contains(&pickup.event_flag) {
                continue;
            }
            seen_flags.insert(pickup.event_flag);

            let category = match pickup.event_flag {
                // Special flag ranges with known categories
                f if (62010..=62099).contains(&f) => EventFlagCategory::MapFragment,
                f if (67000..=68999).contains(&f) => EventFlagCategory::Cookbook,
                f if (65000..=65799).contains(&f) => EventFlagCategory::Whetblade,
                f if (66000..=66999).contains(&f) => EventFlagCategory::PotUpgrade,
                f if (160..=199).contains(&f) => EventFlagCategory::GreatRune,
                f if (9100..=9199).contains(&f) => EventFlagCategory::Remembrance,
                f if (9200..=9299).contains(&f) => EventFlagCategory::TalismanPouch,
                // General flag ranges
                f if f >= 2_000_000_000 => EventFlagCategory::DLCPickup,
                f if f >= 1_000_000_000 => EventFlagCategory::WorldPickup,
                f if f >= 10_000_000 && f < 44_000_000 => EventFlagCategory::DungeonPickup,
                _ => match pickup.category {
                    PickupCategory::GoldenRunes => EventFlagCategory::WorldPickup,
                    PickupCategory::SmithingStones => EventFlagCategory::WorldPickup,
                    PickupCategory::SomberStones => EventFlagCategory::WorldPickup,
                    PickupCategory::Glovewort => EventFlagCategory::WorldPickup,
                    PickupCategory::Weapons => EventFlagCategory::WorldPickup,
                    PickupCategory::Armor => EventFlagCategory::WorldPickup,
                    PickupCategory::Talismans => EventFlagCategory::WorldPickup,
                    PickupCategory::AshesOfWar => EventFlagCategory::WorldPickup,
                    PickupCategory::KeyItems => EventFlagCategory::Progression,
                    PickupCategory::CraftingMaterials => EventFlagCategory::WorldPickup,
                    PickupCategory::Consumables => EventFlagCategory::WorldPickup,
                    PickupCategory::Other => EventFlagCategory::Unknown,
                },
            };

            let region = if pickup.region != "Unknown" {
                normalize_region(pickup.region)
            } else {
                resolve_region(pickup.event_flag).to_string()
            };

            entries.push(EventFlagEntryOwned {
                flag_id: pickup.event_flag,
                name: pickup.name.to_string(),
                category,
                region,
                coords: None,
            });
        }

        // ========================================================================
        // IMPORT FROM GRACES (graces.rs) - ~300 entries
        // ========================================================================
        if let Ok(graces_guard) = GRACES.lock() {
            for (grace, (map, flag_id, name)) in graces_guard.iter() {
                if *flag_id == 0 || seen_flags.contains(flag_id) {
                    continue;
                }
                seen_flags.insert(*flag_id);

                entries.push(EventFlagEntryOwned {
                    flag_id: *flag_id,
                    name: format!("{} - Grace", name),
                    category: EventFlagCategory::Grace,
                    region: map_name_to_region(*map).to_string(),
                    coords: None,
                });
            }
        }

        // ========================================================================
        // IMPORT FROM BOSSES (bosses.rs) - ~200 entries
        // ========================================================================
        if let Ok(bosses_guard) = BOSSES.lock() {
            for (_boss, (flag_id, name)) in bosses_guard.iter() {
                if *flag_id == 0 || seen_flags.contains(flag_id) {
                    continue;
                }
                seen_flags.insert(*flag_id);

                let region = resolve_region(*flag_id).to_string();

                entries.push(EventFlagEntryOwned {
                    flag_id: *flag_id,
                    name: format!("{} - Defeated", name),
                    category: EventFlagCategory::BossDefeat,
                    region,
                    coords: None,
                });
            }
        }

        // ========================================================================
        // IMPORT FROM COOKBOOKS (cookbooks.rs)
        // ========================================================================
        if let Ok(cookbooks_guard) = COOKBOKS.lock() {
            for (_cookbook, (flag_id, name)) in cookbooks_guard.iter() {
                if *flag_id == 0 || seen_flags.contains(flag_id) {
                    continue;
                }
                seen_flags.insert(*flag_id);

                entries.push(EventFlagEntryOwned {
                    flag_id: *flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::Cookbook,
                    region: String::new(), // No specific region data
                    coords: None,
                });
            }
        }

        // ========================================================================
        // IMPORT FROM WHETBLADES (whetblades.rs)
        // ========================================================================
        if let Ok(whetblades_guard) = WHETBLADES.lock() {
            for (_whetblade, (flag_id, name)) in whetblades_guard.iter() {
                if *flag_id == 0 || seen_flags.contains(flag_id) {
                    continue;
                }
                seen_flags.insert(*flag_id);

                entries.push(EventFlagEntryOwned {
                    flag_id: *flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::Whetblade,
                    region: String::new(), // No specific region data
                    coords: None,
                });
            }
        }

        // ========================================================================
        // IMPORT FROM LANDMARKS (landmarks.rs) - ~308 entries
        // ========================================================================
        if let Ok(landmarks_guard) = LANDMARKS.lock() {
            for (_landmark, (flag_id, name)) in landmarks_guard.iter() {
                if *flag_id == 0 || seen_flags.contains(flag_id) {
                    continue;
                }
                seen_flags.insert(*flag_id);

                entries.push(EventFlagEntryOwned {
                    flag_id: *flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::Landmark,
                    region: resolve_region(*flag_id).to_string(),
                    coords: None,
                });
            }
        }

        // ========================================================================
        // MANUAL ENTRIES: GREAT RUNES, REMEMBRANCES, SYSTEM FLAGS
        // ========================================================================

        // Great Runes (100-199)
        let great_runes = [
            (160, "Godrick's Great Rune - Possession", "Stormveil Castle"),
            (161, "Rennala's Great Rune - Possession", "Academy of Raya Lucaria"),
            (162, "Radahn's Great Rune - Possession", "Caelid"),
            (163, "Morgott's Great Rune - Possession", "Leyndell, Royal Capital"),
            (164, "Rykard's Great Rune - Possession", "Volcano Manor"),
            (165, "Mohg's Great Rune - Possession", "Mohgwyn Palace"),
            (166, "Malenia's Great Rune - Possession", "Miquella's Haligtree"),
            (180, "Godrick's Great Rune - Activated", "Divine Tower"),
            (181, "Rennala's Great Rune - Activated", "Divine Tower"),
            (182, "Radahn's Great Rune - Activated", "Divine Tower"),
            (183, "Morgott's Great Rune - Activated", "Divine Tower"),
            (184, "Rykard's Great Rune - Activated", "Divine Tower"),
            (185, "Mohg's Great Rune - Activated", "Divine Tower"),
            (186, "Malenia's Great Rune - Activated", "Divine Tower"),
        ];
        for (flag_id, name, region) in great_runes {
            if !seen_flags.contains(&flag_id) {
                seen_flags.insert(flag_id);
                entries.push(EventFlagEntryOwned {
                    flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::GreatRune,
                    region: region.to_string(),
                    coords: None,
                });
            }
        }

        // Boss Defeat Markers (171-197)
        let boss_defeats = [
            (171, "Godrick the Grafted - World Drop", "Stormveil Castle"),
            (172, "Rennala, Queen of the Full Moon - World Drop", "Academy of Raya Lucaria"),
            (173, "Starscourge Radahn - World Drop", "Caelid"),
            (174, "Morgott, the Omen King - World Drop", "Leyndell, Royal Capital"),
            (175, "Rykard, Lord of Blasphemy - World Drop", "Volcano Manor"),
            (176, "Mohg, Lord of Blood - World Drop", "Mohgwyn Palace"),
            (177, "Malenia, Blade of Miquella - World Drop", "Miquella's Haligtree"),
            (191, "Radagon / Elden Beast - World Drop", "Elden Throne"),
            (192, "Fire Giant - World Drop", "Mountaintops of the Giants"),
            (193, "Godfrey, First Elden Lord - World Drop", "Leyndell, Ashen Capital"),
            (194, "Maliketh, the Black Blade - World Drop", "Crumbling Farum Azula"),
            (195, "Hoarah Loux, Warrior - World Drop", "Leyndell, Ashen Capital"),
            (196, "Lichdragon Fortissax - World Drop", "Deeproot Depths"),
            (197, "Astel, Naturalborn of the Void - World Drop", "Lake of Rot"),
        ];
        for (flag_id, name, region) in boss_defeats {
            if !seen_flags.contains(&flag_id) {
                seen_flags.insert(flag_id);
                entries.push(EventFlagEntryOwned {
                    flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::BossDefeat,
                    region: region.to_string(),
                    coords: None,
                });
            }
        }

        // Remembrances (9100-9199)
        let remembrances = [
            (9101, "Remembrance of the Grafted", "Stormveil Castle"),
            (9102, "Remembrance of the Full Moon Queen", "Academy of Raya Lucaria"),
            (9103, "Remembrance of the Starscourge", "Caelid"),
            (9104, "Remembrance of the Omen King", "Leyndell, Royal Capital"),
            (9105, "Remembrance of the Blasphemous", "Volcano Manor"),
            (9106, "Remembrance of the Blood Lord", "Mohgwyn Palace"),
            (9107, "Remembrance of the Rot Goddess", "Miquella's Haligtree"),
            (9108, "Elden Remembrance", "Elden Throne"),
            (9109, "Remembrance of the Fire Giant", "Mountaintops of the Giants"),
            (9110, "Remembrance of the Lichdragon", "Deeproot Depths"),
            (9111, "Remembrance of the Naturalborn", "Lake of Rot"),
            (9112, "Remembrance of the Black Blade", "Crumbling Farum Azula"),
            (9113, "Remembrance of the Dragonlord", "Crumbling Farum Azula"),
            (9114, "Remembrance of Hoarah Loux", "Leyndell, Ashen Capital"),
        ];
        for (flag_id, name, region) in remembrances {
            if !seen_flags.contains(&flag_id) {
                seen_flags.insert(flag_id);
                entries.push(EventFlagEntryOwned {
                    flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::Remembrance,
                    region: region.to_string(),
                    coords: None,
                });
            }
        }

        // Map Fragments (62010-62084)
        let map_fragments = [
            (62010, "Map: Limgrave, West", "Limgrave"),
            (62011, "Map: Limgrave, East", "Limgrave"),
            (62012, "Map: Weeping Peninsula", "Weeping Peninsula"),
            (62020, "Map: Liurnia, East", "Liurnia of the Lakes"),
            (62021, "Map: Liurnia, North", "Liurnia of the Lakes"),
            (62022, "Map: Liurnia, West", "Liurnia of the Lakes"),
            (62030, "Map: Caelid", "Caelid"),
            (62031, "Map: Dragonbarrow", "Dragonbarrow"),
            (62040, "Map: Altus Plateau", "Altus Plateau"),
            (62041, "Map: Leyndell, Royal Capital", "Leyndell, Royal Capital"),
            (62050, "Map: Mt. Gelmir", "Mt. Gelmir"),
            (62060, "Map: Mountaintops of the Giants, West", "Mountaintops of the Giants"),
            (62061, "Map: Mountaintops of the Giants, East", "Mountaintops of the Giants"),
            (62070, "Map: Consecrated Snowfield", "Consecrated Snowfield"),
            (62080, "Map: Siofra River", "Siofra River"),
            (62081, "Map: Ainsel River", "Ainsel River"),
            (62082, "Map: Lake of Rot", "Lake of Rot"),
            (62083, "Map: Deeproot Depths", "Deeproot Depths"),
            (62084, "Map: Mohgwyn Palace", "Mohgwyn Palace"),
        ];
        for (flag_id, name, region) in map_fragments {
            if !seen_flags.contains(&flag_id) {
                seen_flags.insert(flag_id);
                entries.push(EventFlagEntryOwned {
                    flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::MapFragment,
                    region: region.to_string(),
                    coords: None,
                });
            }
        }

        // Progression flags (60000-60520)
        let progression_flags = [
            (60000, "Flask of Crimson Tears - Obtained", "Stranded Graveyard"),
            (60010, "Spirit Calling Bell - Obtained", "Various"),
            (60020, "Crafting Kit - Obtained", "Church of Elleh"),
            (60100, "Memory Stone - First", "Various"),
            (60110, "Memory Stone - Second", "Various"),
            (60120, "Memory Stone - Third", "Various"),
            (60130, "Memory Stone - Fourth", "Various"),
            (60140, "Memory Stone - Fifth", "Various"),
            (60150, "Memory Stone - Sixth", "Various"),
            (60160, "Memory Stone - Seventh", "Various"),
            (60170, "Memory Stone - Eighth", "Various"),
        ];
        for (flag_id, name, region) in progression_flags {
            if !seen_flags.contains(&flag_id) {
                seen_flags.insert(flag_id);
                entries.push(EventFlagEntryOwned {
                    flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::Progression,
                    region: region.to_string(),
                    coords: None,
                });
            }
        }

        // Talisman Pouches (9200-9281)
        let talisman_pouches = [
            (9200, "Talisman Pouch - First", "Various"),
            (9201, "Talisman Pouch - Second", "Various"),
            (9202, "Talisman Pouch - Third", "Various"),
        ];
        for (flag_id, name, region) in talisman_pouches {
            if !seen_flags.contains(&flag_id) {
                seen_flags.insert(flag_id);
                entries.push(EventFlagEntryOwned {
                    flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::TalismanPouch,
                    region: region.to_string(),
                    coords: None,
                });
            }
        }

        // System flags
        let system_flags = [
            (9000, "Game Started", "Various"),
            (9001, "Tutorial Completed", "Stranded Graveyard"),
            (9020, "Reached Roundtable Hold", "Roundtable Hold"),
            (71800, "Validation Flag 1", "Various"),
            (71801, "Validation Flag 2", "Various"),
        ];
        for (flag_id, name, region) in system_flags {
            if !seen_flags.contains(&flag_id) {
                seen_flags.insert(flag_id);
                entries.push(EventFlagEntryOwned {
                    flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::System,
                    region: region.to_string(),
                    coords: None,
                });
            }
        }

        // NPC State flags
        let npc_flags = [
            (300, "Melina - Met", "Limgrave"),
            (301, "Torrent - Obtained", "Limgrave"),
            (1050, "Ranni - Quest Started", "Liurnia of the Lakes"),
            (1051, "Ranni - Quest Stage 1", "Liurnia of the Lakes"),
            (1100, "Sellen - Quest Started", "Limgrave"),
            (2000, "Varre - Spoke to at First Step", "Limgrave"),
            (2002, "Varre - Invaded", "Mohgwyn Palace"),
            (6000, "D, Hunter of the Dead - Met", "Limgrave"),
            (6001, "Fia - Embraced", "Roundtable Hold"),
        ];
        for (flag_id, name, region) in npc_flags {
            if !seen_flags.contains(&flag_id) {
                seen_flags.insert(flag_id);
                entries.push(EventFlagEntryOwned {
                    flag_id,
                    name: name.to_string(),
                    category: EventFlagCategory::NpcState,
                    region: region.to_string(),
                    coords: None,
                });
            }
        }

        // Sort by flag_id
        entries.sort_by_key(|e| e.flag_id);

        entries
    });
}
