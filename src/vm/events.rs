pub mod events_view_model {
    use std::collections::BTreeMap;
    use wasm_event_flags::{FlagState, ResolvedFlags};

    use crate::{db::{bosses::bosses::{Boss, BOSSES}, colosseums::colosseums::{Colosseum, COLOSSEUMS}, cookbooks::books::{Cookbook, COOKBOKS}, graces::maps::{Grace, GRACES}, landmarks::landmarks::{Landmark, LANDMARKS}, map_name::map_name::{MapName, MAP_NAME}, maps::maps::{Map, MAPS}, summoning_pools::summoning_pools::{SummoningPool, SUMMONING_POOLS}, whetblades::whetblades::{Whetblade, WHETBLADES}, pickup_flags::get_flag_offset}, save::common::save_slot::SaveSlot, util::bit::bit::get_bit, vm::verification_vm::VerificationViewModel, ui::components::{table::{TableState, SortDirection}, filter::FilterBarState, export::ExportFormat}};

    /// Progression gates for late-game graces (76400+).
    /// Only show graces if prerequisite bosses are defeated.
    /// Format: (flag_range_start, flag_range_end, required_boss_flags)
    ///
    /// Unused on purpose. Its consumer `check_progression_gate()` was removed in the
    /// 2026-07-20 grace cutover (see the note at the end of this module) because it
    /// overrode a resolved byte with an inference. The table itself is kept as a
    /// record of real prerequisite relationships — it is documentation, not a mask.
    #[allow(dead_code)]
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

    // A grace's discovery status is a `FlagState` (crate::wasm_event_flags):
    // Set = discovered, Clear = not discovered, Unknown = origin unresolved. The
    // old `GraceStatus` enum was a third copy of that tri-state, and its
    // `is_discovered()` — which returned false for the unreliable case — was one
    // of the collapse sites this migration removed. The grace-specific wording
    // ("Discovered" / "Unreliable") lives at the render sites in `ui/events.rs`,
    // which is where wording belongs.

    /// Generic view state for simple event flag pages (whetblades, cookbooks, maps, bosses, etc.)
    #[derive(Clone)]
    pub struct SimpleEventFlagViewState {
        pub collected_filter: CollectedFilter,
        pub search: String,
        pub table_state: TableState,
        pub filter_state: FilterBarState,
        pub export_format: ExportFormat,
        pub export_filtered_only: bool,
    }

    impl Default for SimpleEventFlagViewState {
        fn default() -> Self {
            Self {
                collected_filter: CollectedFilter::All,
                search: String::new(),
                table_state: TableState::new().with_sort("name", SortDirection::Ascending),
                filter_state: FilterBarState::new(),
                export_format: ExportFormat::Json,
                export_filtered_only: false,
            }
        }
    }

    /// View state for Sites of Grace (has region filter and GraceStatus)
    #[derive(Clone)]
    pub struct GracesViewState {
        pub collected_filter: CollectedFilter,
        pub region_filter: String,
        pub search: String,
        pub table_state: TableState,
        pub filter_state: FilterBarState,
        pub export_format: ExportFormat,
        pub export_filtered_only: bool,
    }

    impl Default for GracesViewState {
        fn default() -> Self {
            Self {
                collected_filter: CollectedFilter::All,
                region_filter: "All".to_string(),
                search: String::new(),
                table_state: TableState::new().with_sort("name", SortDirection::Ascending),
                filter_state: FilterBarState::new(),
                export_format: ExportFormat::Json,
                export_filtered_only: false,
            }
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
        /// Table state for unified table
        pub table_state: TableState,
        /// Filter bar state
        pub filter_state: FilterBarState,
        /// Export format
        pub export_format: ExportFormat,
        /// Export filtered only
        pub export_filtered_only: bool,
    }

    impl Default for WorldPickupsFilter {
        fn default() -> Self {
            Self {
                type_filter: PickupTypeFilter::All,
                collected_filter: CollectedFilter::All,
                region_filter: "All".to_string(),
                search: String::new(),
                selected_flag_id: None,
                table_state: TableState::new().with_sort("lot_id", SortDirection::Ascending),
                filter_state: FilterBarState::new(),
                export_format: ExportFormat::Json,
                export_filtered_only: false,
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
        /// Table state for unified table
        pub table_state: TableState,
        /// Filter bar state
        pub filter_state: FilterBarState,
        /// Export format
        pub export_format: ExportFormat,
        /// Export filtered only
        pub export_filtered_only: bool,
    }

    impl Default for DungeonPickupsFilter {
        fn default() -> Self {
            Self {
                type_filter: PickupTypeFilter::All,
                collected_filter: CollectedFilter::All,
                dungeon_filter: "All".to_string(),
                search: String::new(),
                selected_flag_id: None,
                table_state: TableState::new().with_sort("flag_id", SortDirection::Ascending),
                filter_state: FilterBarState::new(),
                export_format: ExportFormat::Json,
                export_filtered_only: false,
            }
        }
    }

    #[derive(Clone)]
    pub struct EventsViewModel  {
        pub current_route: EventsRoute,
        pub grace_groups: BTreeMap<MapName, Vec<Grace>>,
        pub graces: BTreeMap<Grace, FlagState>,
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
        /// View state for Sites of Grace
        pub graces_view_state: GracesViewState,
        /// View state for Whetblades
        pub whetblades_view_state: SimpleEventFlagViewState,
        /// View state for Cookbooks
        pub cookbooks_view_state: SimpleEventFlagViewState,
        /// View state for Maps
        pub maps_view_state: SimpleEventFlagViewState,
        /// View state for Bosses
        pub bosses_view_state: SimpleEventFlagViewState,
        /// View state for Summoning Pools
        pub summoning_pools_view_state: SimpleEventFlagViewState,
        /// View state for Colosseums
        pub colosseums_view_state: SimpleEventFlagViewState,
        /// View state for Landmarks
        pub landmarks_view_state: SimpleEventFlagViewState,
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
                graces_view_state: Default::default(),
                whetblades_view_state: Default::default(),
                cookbooks_view_state: Default::default(),
                maps_view_state: Default::default(),
                bosses_view_state: Default::default(),
                summoning_pools_view_state: Default::default(),
                colosseums_view_state: Default::default(),
                landmarks_view_state: Default::default(),
             }
        }
    }

    impl EventsViewModel {
        pub fn from_save(slot:& SaveSlot) -> Self {
            let mut events_vm = EventsViewModel::default();

            // Grace family CUT OVER 2026-07-20 (ADR-0006, migration step 4).
            // Positions resolve per save from the flag region (world-state-b),
            // replacing the reliable/unreliable block split and its calibration
            // fallback — both of which existed because the legacy offsets were
            // absolute positions from a single save's layout.
            //
            // The 76400-77000 "progression gate" is deliberately gone with them.
            // It overrode the actual byte with an inference ("prerequisite boss
            // not defeated, so report not discovered"), which suppressed false
            // positives produced by wrong offsets. Against a correctly resolved
            // position that inference can only ever manufacture a false NEGATIVE,
            // hiding a grace the player really has. Read the byte; if it cannot
            // be located, say so.
            //
            // Resolve the origin ONCE for the whole grace table: every grace reads
            // from the same world-state-b base, so re-scanning per grace (~13,400
            // bytes each) would repeat the same work ~340 times. If the origin will
            // not resolve, `resolved` is None and every grace reads Unknown.
            let resolved = ResolvedFlags::from_event_flags(&slot.event_flags.flags);
            for (key, value) in GRACES.lock().unwrap().iter() {
                let flag_id = value.1;

                let status = resolved
                    .as_ref()
                    .map_or(FlagState::Unknown, |r| r.world_state(flag_id));

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

        // REMOVED 2026-07-20 (grace family cutover, ADR-0006):
        // check_progression_gate() and get_calibrated_grace_status() existed to
        // compensate for legacy absolute offsets — the first by overriding the
        // byte with an inference about prerequisite bosses, the second by
        // re-deriving a base for "unreliable" blocks. Grace positions now resolve
        // per save (wasm_event_flags::is_world_state_flag_set), so both would only
        // layer guesses on top of a verified read. PROGRESSION_GATES is kept: it
        // still documents real prerequisite relationships, just not as a flag mask.
    }
}