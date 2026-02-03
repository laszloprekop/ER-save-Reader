//! Typography tokens for consistent font sizing across the application.
//!
//! Uses OS-normalized font sizes and defines specific fonts for different contexts.

use eframe::egui::FontFamily;
use std::sync::Arc;

// =========================================================================
// Font Sizes (in points)
// =========================================================================

/// Micro labels - smallest readable text
pub const TEXT_XS: f32 = 9.0;

/// Secondary text, captions, hints
pub const TEXT_SM: f32 = 10.5;

/// Body text - primary reading size
pub const TEXT_BASE: f32 = 12.0;

/// Table content - optimized for data density
pub const TABLE_CONTENT: f32 = 10.0;

/// Table headers - slightly larger than content
pub const TABLE_HEADER: f32 = 11.0;

/// Monospace content - hex values, Flag IDs, offsets
pub const MONO_CONTENT: f32 = 10.0;

/// Section headers
pub const HEADING_SM: f32 = 14.0;

/// Page headers
pub const HEADING_MD: f32 = 16.0;

/// Large headers
pub const HEADING_LG: f32 = 20.0;

// =========================================================================
// Font Families
// =========================================================================

/// Default proportional font (IBM Plex Sans)
#[inline]
pub fn font_proportional() -> FontFamily {
    FontFamily::Proportional
}

/// Monospace font for code-like content (IBM Plex Mono)
#[inline]
pub fn font_monospace() -> FontFamily {
    FontFamily::Monospace
}

/// Condensed font for headers (IBM Plex Sans Condensed)
#[inline]
pub fn font_condensed() -> FontFamily {
    FontFamily::Name(Arc::from("Condensed"))
}

/// Serif font for descriptive text (IBM Plex Serif)
#[inline]
pub fn font_serif() -> FontFamily {
    FontFamily::Name(Arc::from("Serif"))
}
