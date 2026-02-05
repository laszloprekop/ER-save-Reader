//! Character comparison module for analyzing differences between character slots.
//!
//! This module provides read-only comparison between two character slots,
//! allowing users to see differences in stats, event flags, and inventory.

mod comparison_state;
mod comparison_view;

pub use comparison_state::{ComparisonState, ComparisonTab};
pub use comparison_view::comparison_view;
