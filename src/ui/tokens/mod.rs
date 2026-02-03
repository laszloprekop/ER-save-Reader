//! Design tokens for consistent UI styling.
//!
//! This module provides a centralized set of design tokens for typography,
//! spacing, colors, and component dimensions. Using these tokens ensures
//! visual consistency across the application.
//!
//! # Usage
//!
//! ```rust
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
pub use typography::{
    TEXT_XS, TEXT_SM, TEXT_BASE, TABLE_CONTENT, TABLE_HEADER, MONO_CONTENT,
    HEADING_SM, HEADING_MD, HEADING_LG,
    font_proportional, font_monospace, font_condensed, font_serif,
};

pub use spacing::{
    XS, SM, MD, LG, XL, XXL,
    space_xs, space_sm, space_md, space_lg, space_xl, section_spacer,
};

pub use colors::{
    // Catppuccin palette
    CAT_ROSEWATER, CAT_FLAMINGO, CAT_PINK, CAT_MAUVE, CAT_RED, CAT_MAROON,
    CAT_PEACH, CAT_YELLOW, CAT_GREEN, CAT_TEAL, CAT_SKY, CAT_SAPPHIRE,
    CAT_BLUE, CAT_LAVENDER,
    CAT_CRUST, CAT_MANTLE, CAT_BASE, CAT_SURFACE0, CAT_SURFACE1, CAT_SURFACE2,
    CAT_OVERLAY0, CAT_OVERLAY1, CAT_OVERLAY2, CAT_SUBTEXT0, CAT_SUBTEXT1, CAT_TEXT,
    // Semantic status colors
    STATUS_COLLECTED, STATUS_NOT_COLLECTED, STATUS_UNVERIFIED,
    STATUS_ERROR, STATUS_WARNING, STATUS_INFO,
    // Table colors
    TABLE_ROW_EVEN, TABLE_ROW_ODD, TABLE_ROW_SELECTED, TABLE_ROW_HOVER,
    TABLE_HEADER_TEXT, TABLE_HEADER_BG,
    // UI element colors
    TEXT_PRIMARY, TEXT_SECONDARY, TEXT_DISABLED, TEXT_LABEL,
    ACCENT, FOCUS, BORDER, PANEL_BG,
    // Legacy aliases
    CAT_SUBTEXT, CAT_OVERLAY,
};

pub use dimensions::{
    TABLE_ROW_HEIGHT, TABLE_ROW_HEIGHT_COMPACT, TABLE_HEADER_HEIGHT,
    TABLE_COLUMN_MIN_WIDTH, TABLE_COLUMN_ID, TABLE_COLUMN_FLAG,
    TABLE_COLUMN_STATUS, TABLE_COLUMN_NAME,
    FILTER_SEARCH_WIDTH, FILTER_DROPDOWN_WIDTH, FILTER_BUTTON_MIN_WIDTH,
    SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH,
    BUTTON_HEIGHT, BUTTON_HEIGHT_SM, BUTTON_HEIGHT_LG, BUTTON_ICON_SIZE,
    BORDER_RADIUS, BORDER_RADIUS_SM, BORDER_RADIUS_LG, SCROLL_THUMB_WIDTH,
};
