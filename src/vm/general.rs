pub mod general_view_model {
    use crate::save::common::save_slot::SaveSlot;

    #[derive(Default, Clone)]
    pub struct MapID {
        pub area_id: u8,
        pub block_id: u8,
        pub region_id: u8,
        pub index_id: u8,
    }

    impl std::fmt::Display for MapID {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "m{:02}_{:02}_{:02}_{:02}", self.area_id, self.block_id, self.region_id, self.index_id)
        }
    }

    impl MapID {
        pub fn from_bytes(bytes: &[u8; 4]) -> Self {
            Self {
                area_id: bytes[0],
                block_id: bytes[1],
                region_id: bytes[2],
                index_id: bytes[3],
            }
        }

        pub fn display_name(&self) -> &'static str {
            // Map area_id to region names
            match self.area_id {
                10 => "Limgrave",
                11 => "Liurnia",
                12 => "Altus Plateau",
                13 => "Mt. Gelmir",
                14 => "Caelid",
                15 => "Mountaintops",
                16 => "Siofra River",
                17 => "Ainsel River",
                18 => "Deeproot Depths",
                19 => "Mohgwyn Palace",
                // DLC areas (must come before base game region 20)
                20 if self.block_id >= 40 => "Shadow Realm",
                20 => "Leyndell",
                21 => "Shadow Realm",
                22 => "Shadow Realm",
                30 => "Stormveil Castle",
                31 => "Raya Lucaria",
                32 => "Redmane Castle",
                33 => "Volcano Manor",
                34 => "Leyndell, Royal Capital",
                35 => "Crumbling Farum Azula",
                36 => "Haligtree",
                37 => "Elphael",
                39 => "Elden Throne",
                40 => "Roundtable Hold",
                60 => "Chapel of Anticipation",
                61 => "Stranded Graveyard",
                _ => "Unknown Region",
            }
        }
    }

    #[derive(Default, Clone, PartialEq, Eq, Copy)]
    pub enum Gender {
        Female,
        Male,
        #[default]Uknown,
    }

    impl TryFrom<u8> for Gender {
        type Error = ();
        fn try_from(v: u8) -> Result<Self, Self::Error> {
            match v {
                x if x == Gender::Male as u8 => Ok(Gender::Male),
                x if x == Gender::Female as u8 => Ok(Gender::Female),
                _ => Err(()),
            }
        }
    }

    #[derive(Default, Clone)]
    pub struct GeneralViewModel  {
        /// Unread here — the UI reads `ViewModel::steam_id`. Kept because the save
        /// carries a steam id per slot, and this mirrors that.
        #[allow(dead_code)]
        pub steam_id: String,
        pub character_name: String,
        pub gender: Gender,
        pub weapon_level: u8,
        pub map_id: MapID,
    }

    impl GeneralViewModel {
        pub fn from_save(slot:& SaveSlot) -> Self {

            // Steam Id
            let steam_id = slot.steam_id.to_string();

            // Character Name
            let character_name = slot.player_game_data.character_name;
            let mut character_name_trimmed: [u16; 0x10] = [0;0x10];
            for (i, char) in character_name.iter().enumerate() {
                if *char == 0 { break; }
                character_name_trimmed[i] = *char;
            }
            let character_name = String::from_utf16(&character_name_trimmed).expect("");

            // Gender
            let gender = Gender::try_from(slot.player_game_data.gender).expect("");

            // Weapon Level
            let weapon_level = slot.player_game_data.match_making_wpn_lvl;

            // Map ID (location)
            let map_id = MapID::from_bytes(&slot.map_id);

            Self {
                steam_id,
                character_name,
                gender,
                weapon_level,
                map_id,
            }
        }
    }
}
