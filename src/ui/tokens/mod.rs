#![allow(dead_code)]
//! Design tokens for consistent UI styling.
//!
//! This module provides a centralized set of design tokens for typography,
//! spacing, colors, and component dimensions. Using these tokens ensures
//! visual consistency across the application.
//!
//! # Usage
//!
//! ```text
//! use crate::ui::tokens::{colors, spacing, typography, dimensions};
//!
//! // Typography
//! let text = RichText::new("Hello").size(typography::TEXT_BASE);
//!
//! // Spacing
//! spacing::space_md(ui);
//!
//! // Colors
//! let color = colors::STATUS_COLLECTED;
//!
//! // Dimensions
//! let row_height = dimensions::TABLE_ROW_HEIGHT;
//! ```

pub mod typography;
pub mod spacing;
pub mod colors;
pub mod dimensions;

// Re-export commonly used items at the module level



