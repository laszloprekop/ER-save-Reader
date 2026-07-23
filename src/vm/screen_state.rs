pub mod screen_state {
    //! Per-screen mutable widget state for one slot — navigation, filters, sorts,
    //! selection, and the verification comparison view. Nothing reconstructed lives
    //! here; that is `Character`'s job (Workstream D, `ARCHITECTURE-DEEPENING.md`).
    //!
    //! Held per slot on `SlotViewModel` (`[ScreenState; 0xA]` via `ViewModel::slots`),
    //! which preserves the pre-D1 behaviour where each slot kept its own filter/sort/
    //! selection across slot switches (decision D.2, 2026-07-23). The lazy per-slot
    //! loading of `verification_vm` (guarded by `App::verification_loaded_slots`) keeps
    //! working unchanged because the field stays per slot.

    use crate::vm::{
        events::events_view_model::{
            DungeonPickupsFilter, EventsRoute, GracesViewState, SimpleEventFlagViewState,
            WorldPickupsFilter,
        },
        verification_vm::VerificationViewModel,
    };

    #[derive(Clone)]
    pub struct ScreenState {
        /// Which events sub-page is showing.
        pub current_route: EventsRoute,
        pub world_pickups_filter: WorldPickupsFilter,
        pub dungeon_pickups_filter: DungeonPickupsFilter,
        /// Verification comparison view model (per-slot, lazily loaded).
        pub verification_vm: VerificationViewModel,
        pub graces_view_state: GracesViewState,
        pub whetblades_view_state: SimpleEventFlagViewState,
        pub cookbooks_view_state: SimpleEventFlagViewState,
        pub maps_view_state: SimpleEventFlagViewState,
        pub bosses_view_state: SimpleEventFlagViewState,
        pub summoning_pools_view_state: SimpleEventFlagViewState,
        pub colosseums_view_state: SimpleEventFlagViewState,
        pub landmarks_view_state: SimpleEventFlagViewState,
    }

    impl Default for ScreenState {
        fn default() -> Self {
            Self {
                current_route: EventsRoute::None,
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
}
