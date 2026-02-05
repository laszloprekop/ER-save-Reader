//! Navigation infrastructure for Database Explorer.
//!
//! Provides cross-table navigation with history stack and breadcrumb display.

mod stack;
mod breadcrumb;

pub use stack::{NavigationStack, NavigationEntry, EntityReference};
pub use breadcrumb::{navigation_breadcrumb, NavAction};
