pub mod events_view_model {
    use std::collections::BTreeMap;

    use crate::{calibration::{CalibrationService, GraceBlockCalibration}, db::{bosses::bosses::{Boss, BOSSES}, colosseums::colosseums::{Colosseum, COLOSSEUMS}, cookbooks::books::{Cookbook, COOKBOKS}, graces::maps::{Grace, GRACES}, landmarks::landmarks::{Landmark, LANDMARKS}, map_name::map_name::{MapName, MAP_NAME}, maps::maps::{Map, MAPS}, summoning_pools::summoning_pools::{SummoningPool, SUMMONING_POOLS}, whetblades::whetblades::{Whetblade, WHETBLADES}, pickup_flags::{get_flag_offset, is_block_reliable}}, save::common::save_slot::SaveSlot, util::bit::bit::get_bit, vm::verification_vm::VerificationViewModel};

    /// Progression gates for late-game graces (76400+).
    /// Only show graces if prerequisite bosses are defeated.
    /// Format: (flag_range_start, flag_range_end, required_boss_flags)
    const PROGRESSION_GATES: [(u32, u32, &[u32]); 4] = [
        (76400, 76500, &[]),                        // Caelid late - no gate
        (76500, 76600, &[11000800]),                // Forbidden Lands - Morgott
        (76600, 76700, &[11000800]),                // Mountaintops - Morgott
        (76700, 77000, &[11000800, 1052520800]),    // Consecrated Snowfield - Fire Giant
    ];

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
        DungeonPickups,
        Verification,
    }

    impl EventsRoute {
        pub fn display_name(&self) -> &'static str {
            match self {
                EventsRoute::None => "",
                EventsRoute::SitesOfGrace => "Sites of Grace",
                EventsRoute::Whetblades => "Whetblades",
                EventsRoute::Cookboks => "Cookbooks",
                EventsRoute::Maps => "Maps",
                EventsRoute::Bosses => "Bosses",
                EventsRoute::SummoningPools => "Summoning Pools",
                EventsRoute::Colosseums => "Colosseums",
                EventsRoute::Landmarks => "Landmarks",
                EventsRoute::WorldPickups => "World Pickups",
                EventsRoute::DungeonPickups => "Dungeon Pickups",
                EventsRoute::Verification => "Verification",
            }
        }
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

    /// Status of a grace discovery check
    #[derive(Clone, Copy, PartialEq)]
    pub enum GraceStatus {
        /// Grace is discovered (verified formula)
        Discovered,
        /// Grace is not discovered (verified formula)
        NotDiscovered,
        /// Grace is from an unreliable block - cannot determine status
        Unreliable,
    }

    impl GraceStatus {
        /// Returns true if the grace appears to be discovered
        /// Note: For Unreliable status, this returns false to avoid false positives
        pub fn is_discovered(&self) -> bool {
            matches!(self, GraceStatus::Discovered)
        }

        /// Returns true if this status is from an unreliable block
        pub fn is_unreliable(&self) -> bool {
            matches!(self, GraceStatus::Unreliable)
        }
    }

    #[derive(Clone)]
    pub struct WorldPickupsFilter {
        pub type_filter: PickupTypeFilter,
        pub collected_filter: CollectedFilter,
        pub region_filter: String,
        pub search: String,
        /// Currently selected flag ID for details panel
        pub selected_flag_id: Option<u32>,
    }

    impl Default for WorldPickupsFilter {
        fn default() -> Self {
            Self {
                type_filter: PickupTypeFilter::All,
                collected_filter: CollectedFilter::All,
                region_filter: "All".to_string(),
                search: String::new(),
                selected_flag_id: None,
            }
        }
    }

    #[derive(Clone)]
    pub struct DungeonPickupsFilter {
        pub type_filter: PickupTypeFilter,
        pub collected_filter: CollectedFilter,
        pub dungeon_filter: String,  // "All" or specific dungeon area name
        pub search: String,
        /// Currently selected flag ID for details panel
        pub selected_flag_id: Option<u32>,
    }

    impl Default for DungeonPickupsFilter {
        fn default() -> Self {
            Self {
                type_filter: PickupTypeFilter::All,
                collected_filter: CollectedFilter::All,
                dungeon_filter: "All".to_string(),
                search: String::new(),
                selected_flag_id: None,
            }
        }
    }

    #[derive(Clone)]
    pub struct EventsViewModel  {
        pub current_route: EventsRoute,
        pub grace_groups: BTreeMap<MapName, Vec<Grace>>,
        pub graces: BTreeMap<Grace, GraceStatus>,
        pub whetblades: BTreeMap<Whetblade, bool>,
        pub cookbooks: BTreeMap<Cookbook, bool>,
        pub maps: BTreeMap<Map, bool>,
        pub bosses: BTreeMap<Boss, bool>,
        pub summoning_pools: BTreeMap<SummoningPool, bool>,
        pub colosseums: BTreeMap<Colosseum, bool>,
        pub landmarks: BTreeMap<Landmark, bool>,
        pub world_pickups_filter: WorldPickupsFilter,
        pub dungeon_pickups_filter: DungeonPickupsFilter,
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
                dungeon_pickups_filter: Default::default(),
                verification_vm: Default::default(),
             }
        }
    }

    impl EventsViewModel {
        pub fn from_save(slot:& SaveSlot) -> Self {
            let mut events_vm = EventsViewModel::default();

            // Run calibration for grace blocks
            let calibration = CalibrationService::calibrate(&slot.event_flags.flags);
            let grace_calibration = &calibration.grace_block_calibration;

            // Graces - use formula-based offset calculation with reliability check
            // For unreliable blocks, try calibration first
            for (key, value) in GRACES.lock().unwrap().iter() {
                let flag_id = value.1;

                // Check progression gates for late-game graces (76400+)
                // Don't show graces if prerequisite bosses are not defeated
                let progression_blocked = if flag_id >= 76400 && flag_id < 77000 {
                    Self::check_progression_gate(flag_id, &slot.event_flags.flags)
                } else {
                    false
                };

                let status = if progression_blocked {
                    // Progression gate not met - mark as not discovered to avoid false positives
                    GraceStatus::NotDiscovered
                } else if !is_block_reliable(flag_id) {
                    // Block is unreliable - try calibration
                    Self::get_calibrated_grace_status(
                        flag_id,
                        &slot.event_flags.flags,
                        grace_calibration,
                    )
                } else {
                    // Block is reliable - use the standard offset
                    if let Some((byte_offset, bit_position)) = get_flag_offset(flag_id) {
                        if (byte_offset as usize) < slot.event_flags.flags.len() {
                            let on = get_bit(slot.event_flags.flags[byte_offset as usize], bit_position);
                            if on { GraceStatus::Discovered } else { GraceStatus::NotDiscovered }
                        } else {
                            GraceStatus::Unreliable
                        }
                    } else {
                        GraceStatus::Unreliable
                    }
                };

                events_vm.graces.insert(*key, status);
                events_vm.grace_groups.get_mut(&value.0).expect("").push(*key);
                events_vm.grace_groups.get_mut(&value.0).expect("").sort();
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

        /// Check if a late-game grace is blocked by progression gates.
        ///
        /// Returns true if the flag is in a gated range AND the prerequisite bosses
        /// are NOT defeated, meaning we should NOT show this grace as discovered.
        fn check_progression_gate(flag_id: u32, event_flags: &[u8]) -> bool {
            for &(range_start, range_end, required_bosses) in &PROGRESSION_GATES {
                if flag_id >= range_start && flag_id < range_end {
                    // Found the gate for this flag
                    if required_bosses.is_empty() {
                        return false; // No gate required
                    }

                    // Check if ALL prerequisite bosses are defeated
                    for &boss_flag in required_bosses {
                        if let Some((byte_offset, bit_position)) = get_flag_offset(boss_flag) {
                            if (byte_offset as usize) < event_flags.len() {
                                let byte_val = event_flags[byte_offset as usize];
                                let boss_defeated = (byte_val >> bit_position) & 1 == 1;
                                if !boss_defeated {
                                    return true; // Boss not defeated - block this grace
                                }
                            } else {
                                return true; // Can't check boss - be safe and block
                            }
                        } else {
                            return true; // Can't calculate offset - be safe and block
                        }
                    }
                    return false; // All bosses defeated - allow this grace
                }
            }
            false // Not in any gated range
        }

        /// Get grace status using calibrated offset for unreliable blocks.
        ///
        /// If calibration succeeded, uses the calibrated offset to read the flag.
        /// Otherwise, returns GraceStatus::Unreliable.
        fn get_calibrated_grace_status(
            flag_id: u32,
            event_flags: &[u8],
            calibration: &GraceBlockCalibration,
        ) -> GraceStatus {
            // If calibration failed, mark as unreliable
            if !calibration.success {
                return GraceStatus::Unreliable;
            }

            // Get calibrated offset
            if let Some((byte_offset, bit_position)) = CalibrationService::get_grace_offset_calibrated(
                flag_id,
                calibration,
            ) {
                if (byte_offset as usize) < event_flags.len() {
                    let byte_val = event_flags[byte_offset as usize];

                    // Note: We previously skipped 0xFF bytes as "padding/uninitialized", but this
                    // is wrong for grace flags. When all 8 graces in a byte are discovered, the
                    // byte is legitimately 0xFF. For example, Stormveil Castle graces (71000-71007)
                    // all discovered = 0xFF at the calibrated offset.
                    // The calibration already validated the block location, so we trust the data.

                    let on = get_bit(byte_val, bit_position);
                    return if on { GraceStatus::Discovered } else { GraceStatus::NotDiscovered };
                }
            }

            GraceStatus::Unreliable
        }
    }
}