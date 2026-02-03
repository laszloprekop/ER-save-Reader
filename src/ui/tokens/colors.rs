//! Color tokens based on Catppuccin Frappe palette with semantic aliases.
//!
//! Provides both raw palette colors and semantic aliases for consistent styling.

use eframe::egui::Color32;

// =========================================================================
// Catppuccin Frappe Base Palette
// =========================================================================

// Accent colors
pub const CAT_ROSEWATER: Color32 = Color32::from_rgb(242, 213, 207); // #f2d5cf
pub const CAT_FLAMINGO: Color32 = Color32::from_rgb(238, 190, 190);  // #eebebe
pub const CAT_PINK: Color32 = Color32::from_rgb(244, 184, 228);      // #f4b8e4
pub const CAT_MAUVE: Color32 = Color32::from_rgb(202, 158, 230);     // #ca9ee6
pub const CAT_RED: Color32 = Color32::from_rgb(231, 130, 132);       // #e78284
pub const CAT_MAROON: Color32 = Color32::from_rgb(234, 153, 156);    // #ea999c
pub const CAT_PEACH: Color32 = Color32::from_rgb(239, 159, 118);     // #ef9f76
pub const CAT_YELLOW: Color32 = Color32::from_rgb(229, 200, 144);    // #e5c890
pub const CAT_GREEN: Color32 = Color32::from_rgb(166, 209, 137);     // #a6d189
pub const CAT_TEAL: Color32 = Color32::from_rgb(129, 200, 190);      // #81c8be
pub const CAT_SKY: Color32 = Color32::from_rgb(153, 209, 219);       // #99d1db
pub const CAT_SAPPHIRE: Color32 = Color32::from_rgb(133, 193, 220);  // #85c1dc
pub const CAT_BLUE: Color32 = Color32::from_rgb(140, 170, 238);      // #8caaee
pub const CAT_LAVENDER: Color32 = Color32::from_rgb(186, 187, 241);  // #babbf1

// Surface colors (dark to light)
pub const CAT_CRUST: Color32 = Color32::from_rgb(35, 38, 52);        // #232634
pub const CAT_MANTLE: Color32 = Color32::from_rgb(41, 44, 60);       // #292c3c
pub const CAT_BASE: Color32 = Color32::from_rgb(48, 52, 70);         // #303446
pub const CAT_SURFACE0: Color32 = Color32::from_rgb(65, 69, 89);     // #414559
pub const CAT_SURFACE1: Color32 = Color32::from_rgb(81, 87, 109);    // #51576d
pub const CAT_SURFACE2: Color32 = Color32::from_rgb(98, 104, 128);   // #626880

// Text colors (dark to light)
pub const CAT_OVERLAY0: Color32 = Color32::from_rgb(115, 121, 148);  // #737994
pub const CAT_OVERLAY1: Color32 = Color32::from_rgb(131, 139, 167);  // #838ba7
pub const CAT_OVERLAY2: Color32 = Color32::from_rgb(148, 156, 187);  // #949cbb
pub const CAT_SUBTEXT0: Color32 = Color32::from_rgb(165, 173, 206);  // #a5adce
pub const CAT_SUBTEXT1: Color32 = Color32::from_rgb(181, 191, 226);  // #b5bfe2
pub const CAT_TEXT: Color32 = Color32::from_rgb(198, 208, 245);      // #c6d0f5

// =========================================================================
// Semantic Aliases - Status
// =========================================================================

/// Collected/discovered/success status
pub const STATUS_COLLECTED: Color32 = Color32::from_rgb(100, 200, 100);

/// Not collected/not discovered - neutral gray
pub const STATUS_NOT_COLLECTED: Color32 = Color32::LIGHT_GRAY;

/// Unverified/uncertain status - orange/yellow warning
pub const STATUS_UNVERIFIED: Color32 = Color32::from_rgb(255, 200, 100);

/// Error/danger status
pub const STATUS_ERROR: Color32 = CAT_RED;

/// Warning status
pub const STATUS_WARNING: Color32 = Color32::from_rgb(255, 165, 0);

/// Info status
pub const STATUS_INFO: Color32 = CAT_BLUE;

// =========================================================================
// Semantic Aliases - Table
// =========================================================================

/// Even row background for zebra striping (very subtle - just 4 points lighter than base)
pub const TABLE_ROW_EVEN: Color32 = Color32::from_rgb(52, 56, 74);

/// Odd row background (transparent/base)
pub const TABLE_ROW_ODD: Color32 = Color32::TRANSPARENT;

/// Selected row background
pub const TABLE_ROW_SELECTED: Color32 = Color32::from_rgb(60, 80, 120);

/// Hovered row background
pub const TABLE_ROW_HOVER: Color32 = Color32::from_rgb(55, 65, 95);

/// Table header text color
pub const TABLE_HEADER_TEXT: Color32 = CAT_YELLOW;

/// Table header background
pub const TABLE_HEADER_BG: Color32 = CAT_SURFACE1;

// =========================================================================
// Semantic Aliases - UI Elements
// =========================================================================

/// Primary text color
pub const TEXT_PRIMARY: Color32 = CAT_TEXT;

/// Secondary/muted text color
pub const TEXT_SECONDARY: Color32 = CAT_SUBTEXT0;

/// Disabled text color
pub const TEXT_DISABLED: Color32 = CAT_OVERLAY0;

/// Label text color
pub const TEXT_LABEL: Color32 = Color32::LIGHT_GRAY;

/// Highlight/accent color
pub const ACCENT: Color32 = CAT_BLUE;

/// Focus indicator color
pub const FOCUS: Color32 = CAT_LAVENDER;

/// Border color
pub const BORDER: Color32 = CAT_SURFACE1;

/// Panel background
pub const PANEL_BG: Color32 = Color32::from_rgb(40, 40, 50);

// =========================================================================
// Legacy Aliases (backwards compatibility with style.rs)
// =========================================================================

/// Alias for CAT_SUBTEXT (was CAT_SUBTEXT in style.rs)
pub const CAT_SUBTEXT: Color32 = CAT_SUBTEXT0;

/// Alias for CAT_OVERLAY (was CAT_OVERLAY in style.rs)
pub const CAT_OVERLAY: Color32 = CAT_OVERLAY1;
