//! Unified table component with sorting, selection, and virtual scrolling.
//!
//! # Usage
//!
//! ```text
//! use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData};
//!
//! let mut state = TableState::new();
//!
//! let response = UnifiedTable::new("my_table", &mut state)
//!     .columns(vec![
//!         Column::new("id", "ID").width(60.0).sortable(true).monospace(true),
//!         Column::new("name", "Name").width_fraction(0.4).sortable(true),
//!         Column::new("value", "Value").width(100.0),
//!     ])
//!     .rows(data.iter().map(|item| {
//!         RowData::new(vec![
//!             item.id.to_string(),
//!             item.name.clone(),
//!             item.value.to_string(),
//!         ])
//!     }).collect())
//!     .zebra_stripe(true)
//!     .selectable(true)
//!     .show(ui);
//!
//! // Handle clipboard copy
//! if let Some(text) = response.clipboard_text {
//!     ui.output_mut(|o| o.copied_text = text);
//! }
//! ```

pub mod column;
pub mod state;
pub mod builder;

pub use column::{Column, SortDirection};
pub use state::TableState;
pub use builder::{UnifiedTable, RowData};
