pub mod events_view_model {
    use std::collections::BTreeMap;
    use wasm_event_flags::{FlagState, ResolvedFlags};

    use crate::{db::{bosses::bosses::{Boss, BOSSES}, colosseums::colosseums::{Colosseum, COLOSSEUMS}, cookbooks::books::{Cookbook, COOKBOKS}, graces::maps::{Grace, GRACES}, landmarks::landmarks::{Landmark, LANDMARKS}, map_name::map_name::{MapName, MAP_NAME}, maps::maps::{Map, MAPS}, summoning_pools::summoning_pools::{SummoningPool, SUMMONING_POOLS}, whetblades::whetblades::{Whetblade, WHETBLADES}, pickup_flags::world_flag_state}, save::common::save_slot::SaveSlot, ui::components::{table::{TableState, SortDirection}, filter::FilterBarState, export::ExportFormat}};

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

    /// The nine reconstructed data maps for one slot's event flags. Widget state
    /// (navigation, filters, sorts, the verification view) lives in `ScreenState`
    /// (`vm/screen_state.rs`), split out in D1 (2026-07-23) so the reconstruction
    /// can be read without the egui state that used to surround it.
    #[derive(Clone)]
    pub struct EventsViewModel  {
        pub grace_groups: BTreeMap<MapName, Vec<Grace>>,
        pub graces: BTreeMap<Grace, FlagState>,
        pub whetblades: BTreeMap<Whetblade, bool>,
        pub cookbooks: BTreeMap<Cookbook, bool>,
        pub maps: BTreeMap<Map, bool>,
        pub bosses: BTreeMap<Boss, bool>,
        pub summoning_pools: BTreeMap<SummoningPool, bool>,
        pub colosseums: BTreeMap<Colosseum, bool>,
        pub landmarks: BTreeMap<Landmark, bool>,
    }

    impl Default for EventsViewModel {
        fn default() -> Self {
            Self {
                grace_groups: MAP_NAME.lock().unwrap().iter().map(|m| (*m.0, Vec::new())).collect::<BTreeMap<_,_>>(),
                graces: Default::default(),
                whetblades: Default::default(),
                cookbooks: Default::default(),
                maps: Default::default(),
                bosses: Default::default(),
                summoning_pools: Default::default(),
                colosseums: Default::default(),
                landmarks: Default::default(),
             }
        }
    }

    impl EventsViewModel {
        pub fn from_save(slot: &SaveSlot) -> Self {
            Self::from_event_flags(&slot.event_flags.flags)
        }

        /// Build the events view model from a slot's raw event-flag region.
        ///
        /// Split out from `from_save` (which only ever reads `event_flags.flags`)
        /// so the flag-reading logic is testable against a synthetic region without
        /// constructing a whole `SaveSlot` — the seam the whetblade regression test
        /// (frozen offset vs resolved Origin) needs.
        pub fn from_event_flags(ef: &[u8]) -> Self {
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
            let resolved = ResolvedFlags::from_event_flags(ef);
            for (key, value) in GRACES.lock().unwrap().iter() {
                let flag_id = value.1;

                let status = resolved
                    .as_ref()
                    .map_or(FlagState::Unknown, |r| r.world_state(flag_id));

                events_vm.graces.insert(*key, status);
                events_vm.grace_groups.get_mut(&value.0).expect("").push(*key);
                events_vm.grace_groups.get_mut(&value.0).expect("").sort();
            }

            // Whetblades — world-state block flags (65610-65720), the same family
            // as graces. CUT OVER 2026-07-23 from the frozen `get_flag_offset` block
            // base: that base is a fixed offset valid only for the save it was
            // measured on, but the family floats per save, so on any other save the
            // fixed offset reads the wrong bytes. Diagnosed against ER0000.sl2 slot 5,
            // where the frozen path reported 2 of the 3 owned whetblades (both bits
            // correct only by coincidence) and missed Iron and Glintstone entirely;
            // `world_state` resolves the base per save and reads all 7 flags right.
            //
            // Unknown (region unresolvable) collapses to not-discovered here because
            // the shared `simple_event_flag_view` is bool-typed and cannot carry the
            // third state. That is a known limitation, recorded in the post-mortem —
            // not a reintroduction of the Unknown-is-Clear bug at the reader layer.
            for (key, value) in WHETBLADES.lock().unwrap().iter() {
                let on = resolved
                    .as_ref()
                    .map_or(FlagState::Unknown, |r| r.world_state(value.0))
                    == FlagState::Set;
                events_vm.whetblades.insert(*key, on);
            }

            // The rest of the cluster followed whetblades off the frozen block
            // base and onto the per-save resolver on 2026-07-23. `world_flag_state`
            // routes each id to its Flag Family (world-state / tile-world / dungeon)
            // and resolves the base per save; Set → discovered, Clear/Unknown → not.
            // Cookbooks and colosseums are pure world-state block flags; maps,
            // bosses, summoning pools and landmarks mix families, which the router
            // handles by id range. (Whetblades above still call `world_state`
            // directly — same result for the 65k block; left as the diagnosed case.)
            let read = |flag_id: u32| {
                resolved
                    .as_ref()
                    .map_or(FlagState::Unknown, |r| world_flag_state(r, flag_id))
                    == FlagState::Set
            };

            for (key, value) in COOKBOKS.lock().unwrap().iter() {
                events_vm.cookbooks.insert(*key, read(value.0));
            }
            for (key, value) in MAPS.lock().unwrap().iter() {
                events_vm.maps.insert(*key, read(value.0));
            }
            for (key, value) in BOSSES.lock().unwrap().iter() {
                events_vm.bosses.insert(*key, read(value.0));
            }
            // Summoning pools are deliberately NOT routed. Their flags (120 are
            // 8-digit like 10000040, 42 are 10-digit) verify against neither reader:
            // on slot 5 the frozen path found 7 set, the resolver's dungeon/tile
            // routing found 0, and ids like 10000040 do not parse as valid
            // map-encoded dungeon flags — the resolver *places* them at a bogus slot
            // and reads a false Clear rather than refusing. Until the family is
            // identified (BACKLOG: "Summoning pool flag family is unidentified"),
            // read them as not-discovered rather than guess a family (ADR-0008).
            for (key, _value) in SUMMONING_POOLS.lock().unwrap().iter() {
                events_vm.summoning_pools.insert(*key, false);
            }
            for (key, value) in COLOSSEUMS.lock().unwrap().iter() {
                events_vm.colosseums.insert(*key, read(value.0));
            }
            for (key, value) in LANDMARKS.lock().unwrap().iter() {
                events_vm.landmarks.insert(*key, read(value.0));
            }

            events_vm
        }

        // REMOVED 2026-07-20 (grace family cutover, ADR-0006):
        // check_progression_gate() and get_calibrated_grace_status() existed to
        // compensate for legacy absolute offsets — the first by overriding the
        // byte with an inference about prerequisite bosses, the second by
        // re-deriving a base for "unreliable" blocks. Grace positions now resolve
        // per save (wasm_event_flags::ResolvedFlags::world_state), so both would only
        // layer guesses on top of a verified read. PROGRESSION_GATES is kept: it
        // still documents real prerequisite relationships, just not as a flag mask.
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use wasm_event_flags::{resolve_family_base_in_ef, FAMILY_WORLD_STATE_B};

        /// Set a world-state block flag at the position a resolved Origin puts it,
        /// mirroring `ResolvedFlags::world_state` geometry.
        fn set_world_state_flag(ef: &mut [u8], base: usize, flag_id: u32) {
            let byte = base + ((flag_id - 50_000) / 8) as usize;
            let bit = 7 - (flag_id % 8) as u8;
            ef[byte] |= 1 << bit;
        }

        /// Regression (diagnosed on ER0000.sl2 slot 5, 2026-07-23): whetblades must
        /// read from the per-save resolved Origin, not the frozen block base.
        ///
        /// The region below resolves to a world-state base that does NOT coincide
        /// with the frozen `get_flag_offset` block offset, so only the resolver
        /// lands on the bits we set. The old path read fixed bytes that are clear
        /// here and reported the Iron Whetblade as not-owned — exactly the failure
        /// on the real save, where it saw 2 of 3 owned whetblades (by coincidence)
        /// and missed Iron and Glintstone entirely.
        #[test]
        fn whetblades_read_from_resolved_origin_not_frozen_base() {
            // A region that resolves — marker puts the list end in the detectable
            // range, as tests/flag_state_conformance.rs constructs one.
            let mut ef = vec![0u8; 2_100_000];
            ef[20_000] = 0x01;
            let base = resolve_family_base_in_ef(&ef, FAMILY_WORLD_STATE_B)
                .expect("synthetic region should resolve a world-state base");

            // The three Iron Whetblade affinity flags, set at their RESOLVED
            // positions; every other whetblade flag left clear.
            for flag in [65_610, 65_620, 65_630] {
                set_world_state_flag(&mut ef, base, flag);
            }

            let vm = EventsViewModel::from_event_flags(&ef);

            // Owned affinities read set...
            assert_eq!(vm.whetblades.get(&Whetblade::IronWhetbladeHeavy), Some(&true));
            assert_eq!(vm.whetblades.get(&Whetblade::IronWhetbladeKeen), Some(&true));
            assert_eq!(vm.whetblades.get(&Whetblade::IronWhetbladeQuality), Some(&true));
            // ...and an unset one stays clear (guards against "reads everything set").
            assert_eq!(vm.whetblades.get(&Whetblade::BlackWhetbladeBlood), Some(&false));
        }
    }
}