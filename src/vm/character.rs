pub mod character {
    //! `Character` — one slot's reconstruction, read the way the game loads one:
    //! immutable, no egui, no filters (ARCHITECTURE-DEEPENING.md, Workstream D).
    //!
    //! It borrows the slot's already-built read models and holds ONE `ResolvedFlags`
    //! over the slot's flag region, so "which bytes" stops being a question each view
    //! answers for itself — the two sources of flag bytes (the vm's build-time
    //! `slot.event_flags.flags` and the views' render-time `get_event_flags`) collapse
    //! into this single resolved value.
    //!
    //! Paired with `&mut ScreenState` via `SlotViewModel::split`, which hands out
    //! disjoint borrows of the same slot: the reconstruction immutably (here) and the
    //! widget state mutably. That split is why a view can take `(&Character, &mut
    //! ScreenState)` at all — the lifetime question B settled (borrowing is free) in
    //! its most demanding form.
    //!
    //! D2a exposes the surface the events-view cluster consumes; it grows to carry
    //! stats / equipment / inventory / regions as D2b migrates those views.

    use wasm_event_flags::ResolvedFlags;

    use crate::vm::{
        events::events_view_model::EventsViewModel,
        general::general_view_model::GeneralViewModel,
    };

    pub struct Character<'a> {
        index: usize,
        events: &'a EventsViewModel,
        general: &'a GeneralViewModel,
        flag_bytes: Option<&'a [u8]>,
        flags: Option<ResolvedFlags<'a>>,
    }

    impl<'a> Character<'a> {
        pub fn new(
            index: usize,
            events: &'a EventsViewModel,
            general: &'a GeneralViewModel,
            event_flags: Option<&'a [u8]>,
        ) -> Self {
            // Resolve the origin ONCE for this save's flag region. `None` means the
            // origin would not resolve, so every flag read is Unknown — refusal at
            // construction, not re-decided per flag (CONTEXT.md → ResolvedFlags).
            let flags = event_flags.and_then(ResolvedFlags::from_event_flags);
            Self { index, events, general, flag_bytes: event_flags, flags }
        }

        pub fn index(&self) -> usize {
            self.index
        }

        pub fn events(&self) -> &'a EventsViewModel {
            self.events
        }

        pub fn general(&self) -> &'a GeneralViewModel {
            self.general
        }

        /// The resolved flag families for this slot, or `None` if the origin did not
        /// resolve. `None` is NOT "all clear": callers map it to `FlagState::Unknown`.
        pub fn flags(&self) -> Option<&ResolvedFlags<'a>> {
            self.flags.as_ref()
        }

        /// The raw event-flag region, for the few sites that need bytes rather than
        /// resolved families (the detail panel's byte/offset dump).
        pub fn flag_bytes(&self) -> Option<&'a [u8]> {
            self.flag_bytes
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // The real event-flag region length (0x1bf99f). A single marker past
        // EF+16,000 puts the list end in the detectable range, matching the
        // fixture shape the wasm crate's conformance tests use.
        fn synthetic_ef() -> Vec<u8> {
            let mut buf = vec![0u8; 0x1bf99f];
            buf[20_000] = 0x01;
            buf
        }

        /// No flag bytes means `flags()` is `None` — which callers read as
        /// `FlagState::Unknown`, never `Clear`. This is the seam that keeps an
        /// unresolved read from silently becoming "not collected".
        #[test]
        fn no_flag_bytes_yields_none_not_clear() {
            let events = EventsViewModel::default();
            let general = GeneralViewModel::default();
            let ch = Character::new(3, &events, &general, None);

            assert!(ch.flags().is_none(), "no bytes -> no resolved flags");
            assert!(ch.flag_bytes().is_none());
            assert_eq!(ch.index(), 3);
        }

        /// The whole point of `Character`: it resolves the origin ONCE, and that
        /// resolution is byte-identical to a direct `ResolvedFlags::from_event_flags`
        /// over the same bytes. Phrased as `Option`-to-`Option` so it pins the
        /// agreement whether or not this synthetic buffer happens to resolve — the
        /// contract is "Character adds no divergence", not a fixture assertion.
        #[test]
        fn flags_agree_with_a_direct_resolve() {
            let events = EventsViewModel::default();
            let general = GeneralViewModel::default();
            let ef = synthetic_ef();
            let ch = Character::new(0, &events, &general, Some(&ef));

            let direct = ResolvedFlags::from_event_flags(&ef);
            for id in [76_100_u32, 71_800, 1_042_370_800] {
                let via_character = ch.flags().map(|r| r.world_state(id));
                let via_direct = direct.as_ref().map(|r| r.world_state(id));
                assert_eq!(
                    via_character, via_direct,
                    "Character must resolve flag {id} identically to a direct resolve",
                );
            }
            // Refusal is a whole-save decision: either both resolve or neither does.
            assert_eq!(ch.flags().is_some(), direct.is_some());
            assert_eq!(ch.flag_bytes(), Some(ef.as_slice()));
        }

        /// The reconstruction accessors hand back exactly the borrowed models —
        /// `Character` is a lens over the slot, not a copy of it.
        #[test]
        fn accessors_borrow_the_slot_they_were_built_from() {
            let events = EventsViewModel::default();
            let general = GeneralViewModel::default();
            let ch = Character::new(1, &events, &general, None);

            assert!(std::ptr::eq(ch.events(), &events));
            assert!(std::ptr::eq(ch.general(), &general));
        }
    }
}
