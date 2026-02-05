//! Save file validation module for read-only analysis.
//!
//! This module provides a UI for viewing validation results of save files.
//! All checks are read-only and never modify save data.

mod validation_state;
mod validation_view;

pub use validation_state::{ValidationState, ValidationReport, ValidationIssue, Severity};
pub use validation_view::validation_view;
