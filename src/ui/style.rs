//! Shared UI styling constants for consistent appearance across views.

use eframe::egui::Color32;

// =========================================================================
// Monospace Table Styling
// =========================================================================

/// Standard monospace font size for table content (75% of original 12.0)
pub const TABLE_MONO_SIZE: f32 = 9.0;

// =========================================================================
// Catppuccin Frappé Color Palette (shared across views)
// =========================================================================

pub const CAT_RED: Color32 = Color32::from_rgb(231, 130, 132);      // #e78284
pub const CAT_GREEN: Color32 = Color32::from_rgb(166, 209, 137);    // #a6d189
pub const CAT_YELLOW: Color32 = Color32::from_rgb(229, 200, 144);   // #e5c890
pub const CAT_PEACH: Color32 = Color32::from_rgb(239, 159, 118);    // #ef9f76
pub const CAT_TEAL: Color32 = Color32::from_rgb(129, 200, 190);     // #81c8be
pub const CAT_SUBTEXT: Color32 = Color32::from_rgb(165, 173, 206);  // #a5adce
pub const CAT_OVERLAY: Color32 = Color32::from_rgb(131, 139, 167);  // #838ba7
