//! ER Save Reader — reconstructs a character's state from a save file the way the
//! game loads one, and stops there (ADR-0009). It never writes a save back.
//!
//! # Visibility
//!
//! Every module below is `pub(crate)` by default. This is deliberate and is not
//! bookkeeping: in a library, a `pub` module is reachable API, so dead-code
//! analysis stops applying to everything inside it. This crate depends on that
//! analysis — it is what surfaced `src/calibration.rs`, 997 lines that were
//! compiled and linted while reachable from nothing.
//!
//! Consequently the public surface is opened one item at a time, by `pub use`
//! below, rather than by publishing a module wholesale. Add to it when a test or
//! the binary genuinely needs an item, and prefer re-exporting the item over
//! promoting its module.
//!
//! There is no external consumer to break: elden-map depends on
//! `crates/wasm-event-flags`, which is its own crate with its own `[lib]`.

pub(crate) mod db;
pub(crate) mod generated;
pub(crate) mod knowledge;
pub(crate) mod read;
pub(crate) mod save;
pub(crate) mod ui;
pub(crate) mod util;
pub(crate) mod vm;
#[cfg(feature = "save-writeback")]
pub(crate) mod write;

mod app;

// `ui/landing.rs` and `ui/menu.rs` refer to `crate::App`; re-exporting it at the
// root keeps those paths correct now that the type lives in `app`.
pub(crate) use app::App;

/// Run the reader. `src/main.rs` is a thin wrapper around this.
pub use app::run;

/// Read one pickup flag, routed to its Flag Family, for one save's flag region.
///
/// Exposed for `tests/flag_state_conformance.rs`. `None` is Unknown — the family
/// could not be resolved, or the id belongs to no known family — and is never
/// "not collected" (`CONTEXT.md` → Unknown).
pub use db::pickup_flags::pickup_flag_state;

/// Run the knowledge pipeline CLI (`er-save-reader knowledge …`).
pub use knowledge::run_cli;
