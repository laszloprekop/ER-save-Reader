//! Unified filter bar component with standard filter dimensions.
//!
//! # Usage
//!
//! ```rust
//! use crate::ui::components::filter::{FilterBar, FilterBarState, FilterOption};
//!
//! let mut state = FilterBarState::new();
//!
//! FilterBar::new("my_filter", &mut state)
//!     .completion_filter()
//!     .category_strings("Type", &["Weapons", "Armor", "Items"])
//!     .region_strings("Region", &["Limgrave", "Liurnia", "Caelid"])
//!     .search("Search items...")
//!     .show(ui);
//!
//! // Apply filters to data
//! let filtered: Vec<_> = data.iter()
//!     .filter(|item| {
//!         // Check completion status
//!         match state.completion {
//!             CompletionStatus::Collected => item.collected,
//!             CompletionStatus::NotCollected => !item.collected,
//!             _ => true,
//!         }
//!     })
//!     .filter(|item| state.category == "All" || item.category == state.category)
//!     .filter(|item| state.region == "All" || item.region == state.region)
//!     .filter(|item| fuzzy_match(&item.name, &state.search, 0.7))
//!     .collect();
//! ```

pub mod dimension;
pub mod search;
pub mod state;
pub mod bar;

pub use dimension::{CompletionStatus, FilterOption};
pub use search::{fuzzy_match, fuzzy_match_default, fuzzy_match_any, match_score, DEFAULT_THRESHOLD};
pub use state::FilterBarState;
pub use bar::{FilterBar, FilterBarResponse};
