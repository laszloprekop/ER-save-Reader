//! Spacing tokens using a 4px base scale.
//!
//! Provides consistent spacing throughout the application.

use eframe::egui::Ui;

// =========================================================================
// Spacing Scale (4px base)
// =========================================================================

/// Extra small spacing - tight, within elements (4px)
pub const XS: f32 = 4.0;

/// Small spacing - between elements (8px)
pub const SM: f32 = 8.0;

/// Medium spacing - between groups (12px)
pub const MD: f32 = 12.0;

/// Large spacing - between sections (16px)
pub const LG: f32 = 16.0;

/// Extra large spacing - page-level (24px)
pub const XL: f32 = 24.0;

/// Double extra large spacing - major sections (32px)
pub const XXL: f32 = 32.0;

// =========================================================================
// Convenience Functions
// =========================================================================

/// Add extra small space
#[inline]
pub fn space_xs(ui: &mut Ui) {
    ui.add_space(XS);
}

/// Add small space
#[inline]
pub fn space_sm(ui: &mut Ui) {
    ui.add_space(SM);
}

/// Add medium space
#[inline]
pub fn space_md(ui: &mut Ui) {
    ui.add_space(MD);
}

/// Add large space
#[inline]
pub fn space_lg(ui: &mut Ui) {
    ui.add_space(LG);
}

/// Add extra large space
#[inline]
pub fn space_xl(ui: &mut Ui) {
    ui.add_space(XL);
}

/// Standard section spacing (backwards compatible with style::spacer)
#[inline]
pub fn section_spacer(ui: &mut Ui) {
    ui.add_space(SM);
}
