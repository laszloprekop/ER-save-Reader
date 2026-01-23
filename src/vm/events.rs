pub mod events_view_model {
    use std::collections::BTreeMap;

    use crate::{db::{bosses::bosses::{Boss, BOSSES}, colosseums::colosseums::{Colosseum, COLOSSEUMS}, cookbooks::books::{Cookbook, COOKBOKS}, graces::maps::{Grace, GRACES}, landmarks::landmarks::{Landmark, LANDMARKS}, map_name::map_name::{MapName, MAP_NAME}, maps::maps::{Map, MAPS}, summoning_pools::summoning_pools::{SummoningPool, SUMMONING_POOLS}, whetblades::whetblades::{Whetblade, WHETBLADES}, pickup_flags::get_flag_offset}, save::common::save_slot::SaveSlot, util::bit::bit::get_bit, vm::verification_vm::VerificationViewModel};

    #[derive(Clone)]
    pub enum EventsRoute {
        None,
        SitesOfGrace,
        Whetblades,
        Cookboks,
        Maps,
        Bosses,
        SummoningPools,
        Colosseums,
        Landmarks,
        WorldPickups,
        Verification,
    }

    #[derive(Clone, Copy, PartialEq)]
    pub enum PickupTypeFilter {
        All,
        GoldenRunes,
        SmithingStones,
        SomberStones,
        Glovewort,
        Weapons,
        Armor,
        Talismans,
        AshesOfWar,
        KeyItems,
        CraftingMaterials,
        Consumables,
        Other,
    }

    #[derive(Clone, Copy, PartialEq)]
    pub enum CollectedFilter {
        All,
        Collected,
        NotCollected,
        Unverified,
    }

    #[derive(Clone)]
    pub struct WorldPickupsFilter {
        pub type_filter: PickupTypeFilter,
        pub collected_filter: CollectedFilter,
        pub region_filter: String,
        pub search: String,
    }

    impl Default for WorldPickupsFilter {
        fn default() -> Self {
            Self {
                type_filter: PickupTypeFilter::All,
                collected_filter: CollectedFilter::All,
                region_filter: "All".to_string(),
                search: String::new(),
            }
        }
    }

    #[derive(Clone)]
    pub struct EventsViewModel  {
        pub current_route: EventsRoute,
        pub grace_groups: BTreeMap<MapName, Vec<Grace>>,
        pub graces: BTreeMap<Grace, bool>,
        pub whetblades: BTreeMap<Whetblade, bool>,
        pub cookbooks: BTreeMap<Cookbook, bool>,
        pub maps: BTreeMap<Map, bool>,
        pub bosses: BTreeMap<Boss, bool>,
        pub summoning_pools: BTreeMap<SummoningPool, bool>,
        pub colosseums: BTreeMap<Colosseum, bool>,
        pub landmarks: BTreeMap<Landmark, bool>,
        pub world_pickups_filter: WorldPickupsFilter,
        /// Verification comparison view model (per-slot)
        pub verification_vm: VerificationViewModel,
    }

    impl Default for EventsViewModel {
        fn default() -> Self {
            Self {
                current_route: EventsRoute::None,
                grace_groups: MAP_NAME.lock().unwrap().iter().map(|m| (*m.0, Vec::new())).collect::<BTreeMap<_,_>>(),
                graces: Default::default(),
                whetblades: Default::default(),
                cookbooks: Default::default(),
                maps: Default::default(),
                bosses: Default::default(),
                summoning_pools: Default::default(),
                colosseums: Default::default(),
                landmarks: Default::default(),
                world_pickups_filter: Default::default(),
                verification_vm: Default::default(),
             }
        }
    }

    impl EventsViewModel {
        pub fn from_save(slot:& SaveSlot) -> Self {
            let mut events_vm = EventsViewModel::default();

            // Graces - use formula-based offset calculation
            for (key, value) in GRACES.lock().unwrap().iter() {
                let flag_id = value.1;
                if let Some((byte_offset, bit_position)) = get_flag_offset(flag_id) {
                    if (byte_offset as usize) < slot.event_flags.flags.len() {
                        let on = get_bit(slot.event_flags.flags[byte_offset as usize], bit_position);
                        events_vm.graces.insert(*key, on);
                        events_vm.grace_groups.get_mut(&value.0).expect("").push(*key);
                        events_vm.grace_groups.get_mut(&value.0).expect("").sort();
                    }
                }
            }

            // Whetblades
            for (key, value) in WHETBLADES.lock().unwrap().iter() {
                let flag_id = value.0;
                if let Some((byte_offset, bit_position)) = get_flag_offset(flag_id) {
                    if (byte_offset as usize) < slot.event_flags.flags.len() {
                        let on = get_bit(slot.event_flags.flags[byte_offset as usize], bit_position);
                        events_vm.whetblades.insert(*key, on);
                    }
                }
            }

            // Cookbooks
            for (key, value) in COOKBOKS.lock().unwrap().iter() {
                let flag_id = value.0;
                if let Some((byte_offset, bit_position)) = get_flag_offset(flag_id) {
                    if (byte_offset as usize) < slot.event_flags.flags.len() {
                        let on = get_bit(slot.event_flags.flags[byte_offset as usize], bit_position);
                        events_vm.cookbooks.insert(*key, on);
                    }
                }
            }

            // Maps
            for (key, value) in MAPS.lock().unwrap().iter() {
                let flag_id = value.0;
                if let Some((byte_offset, bit_position)) = get_flag_offset(flag_id) {
                    if (byte_offset as usize) < slot.event_flags.flags.len() {
                        let on = get_bit(slot.event_flags.flags[byte_offset as usize], bit_position);
                        events_vm.maps.insert(*key, on);
                    }
                }
            }

            // Bosses
            for (key, value) in BOSSES.lock().unwrap().iter() {
                let flag_id = value.0;
                if let Some((byte_offset, bit_position)) = get_flag_offset(flag_id) {
                    if (byte_offset as usize) < slot.event_flags.flags.len() {
                        let on = get_bit(slot.event_flags.flags[byte_offset as usize], bit_position);
                        events_vm.bosses.insert(*key, on);
                    }
                }
            }

            // Summoning Pools
            for (key, value) in SUMMONING_POOLS.lock().unwrap().iter() {
                let flag_id = value.0;
                if let Some((byte_offset, bit_position)) = get_flag_offset(flag_id) {
                    if (byte_offset as usize) < slot.event_flags.flags.len() {
                        let on = get_bit(slot.event_flags.flags[byte_offset as usize], bit_position);
                        events_vm.summoning_pools.insert(*key, on);
                    }
                }
            }

            // Colosseums
            for (key, value) in COLOSSEUMS.lock().unwrap().iter() {
                let flag_id = value.0;
                if let Some((byte_offset, bit_position)) = get_flag_offset(flag_id) {
                    if (byte_offset as usize) < slot.event_flags.flags.len() {
                        let on = get_bit(slot.event_flags.flags[byte_offset as usize], bit_position);
                        events_vm.colosseums.insert(*key, on);
                    }
                }
            }

            // Landmarks
            for (key, value) in LANDMARKS.lock().unwrap().iter() {
                let flag_id = value.0;
                if let Some((byte_offset, bit_position)) = get_flag_offset(flag_id) {
                    if (byte_offset as usize) < slot.event_flags.flags.len() {
                        let on = get_bit(slot.event_flags.flags[byte_offset as usize], bit_position);
                        events_vm.landmarks.insert(*key, on);
                    } else {
                        events_vm.landmarks.insert(*key, false);
                    }
                } else {
                    // Flag not in formula ranges, default to false (not discovered)
                    events_vm.landmarks.insert(*key, false);
                }
            }

            events_vm
        }
    }
}