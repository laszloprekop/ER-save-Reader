pub mod stats_view_model {
    use crate::{
        db::classes::classes::ArcheType,
        save::common::save_slot::SaveSlot,
        ui::components::{
            table::TableState,
            export::ExportFormat,
        },
    };

    #[derive(Clone)]
    pub struct StatsViewModel {
        pub table_state: TableState,
        pub export_format: ExportFormat,
        pub arche_type: ArcheType,
        pub vigor: u32,
        pub mind: u32,
        pub endurance: u32,
        pub strength: u32,
        pub dexterity: u32,
        pub intelligence: u32,
        pub faith: u32,
        pub arcane: u32,
        pub level: u32,
        pub souls: u32,
        pub soulsmemory: u32,
        pub scadutree: u32,
        pub spirit_ash: u32,
        // HP, FP, SP (Stamina)
        pub hp: u32,
        pub max_hp: u32,
        pub fp: u32,
        pub max_fp: u32,
        /// Unread: the UI shows `max_stamina` only. Kept because this block mirrors
        /// the save's HP/FP/SP triple and the current value is part of that layout —
        /// deleting it would erase the fact that the save carries one.
        #[allow(dead_code)]
        pub stamina: u32,
        pub max_stamina: u32,
    }

    impl Default for StatsViewModel {
        fn default() -> Self {
            Self {
                table_state: TableState::default(),
                export_format: ExportFormat::default(),
                arche_type: ArcheType::Unknown,
                vigor: Default::default(),
                mind: Default::default(),
                endurance: Default::default(),
                strength: Default::default(),
                dexterity: Default::default(),
                intelligence: Default::default(),
                faith: Default::default(),
                arcane: Default::default(),
                level: Default::default(),
                souls: Default::default(),
                soulsmemory: Default::default(),
                scadutree: Default::default(),
                spirit_ash: Default::default(),
                hp: Default::default(),
                max_hp: Default::default(),
                fp: Default::default(),
                max_fp: Default::default(),
                stamina: Default::default(),
                max_stamina: Default::default(),
            }
        }
    }

    impl StatsViewModel {
        pub fn from_save(slot: &SaveSlot) -> Self {
            let arche_type = ArcheType::try_from(slot.player_game_data.arche_type).expect("");
            let vigor = slot.player_game_data.vigor;
            let mind = slot.player_game_data.mind;
            let endurance = slot.player_game_data.endurance;
            let strength = slot.player_game_data.strength;
            let dexterity = slot.player_game_data.dexterity;
            let intelligence = slot.player_game_data.intelligence;
            let faith = slot.player_game_data.faith;
            let arcane = slot.player_game_data.arcane;
            let level = slot.player_game_data.level;
            let souls = slot.player_game_data.souls;
            let soulsmemory = slot.player_game_data.soulsmemory;

            // DLC Stats
            let scadutree = slot.player_game_data.scadutree_lvl.into();
            let spirit_ash = slot.player_game_data.spirit_ash_lvl.into();

            // HP, FP, Stamina
            let hp = slot.player_game_data.health;
            let max_hp = slot.player_game_data.max_health;
            let fp = slot.player_game_data.fp;
            let max_fp = slot.player_game_data.max_fp;
            let stamina = slot.player_game_data.sp;
            let max_stamina = slot.player_game_data.max_sp;

            Self {
                arche_type,
                vigor,
                mind,
                endurance,
                strength,
                dexterity,
                intelligence,
                faith,
                arcane,
                level,
                souls,
                soulsmemory,
                scadutree,
                spirit_ash,
                hp,
                max_hp,
                fp,
                max_fp,
                stamina,
                max_stamina,
                ..Default::default()
            }
        }
    }
}
