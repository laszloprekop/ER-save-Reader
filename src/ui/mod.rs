// TODO(architecture-deepening, workstream A): this subtree-wide allow is still in
// place, deliberately. Lifting it yields ~130 warnings, overwhelmingly unused
// accessors on generated tables and view-state helpers — a lower-signal job than
// the read path (vm/, save/common/, util/), which was swept on 2026-07-22. See
// docs/ARCHITECTURE-DEEPENING.md.
#![allow(dead_code)]
mod custom;
pub mod tokens;
pub mod style;
pub mod components;
pub mod state;
pub mod menu;
pub mod icons;
pub mod none;
pub mod general;
pub mod stats;
pub mod inventory;
pub mod events;
pub mod regions;
#[cfg(feature = "save-writeback")]
pub mod importer;
pub mod equipment;
pub mod spells_view;
pub mod npcs_view;
pub mod shop_items_view;
pub mod world_pickups_view;
pub mod event_flags_db_view;
pub mod verification_view;
pub mod landing;
pub mod database;
pub mod comparison;
pub mod validation;
pub mod utilities;
