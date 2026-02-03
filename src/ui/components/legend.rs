//! Legend component for displaying icon/symbol meanings.

use eframe::egui::{Color32, RichText, Ui};
use crate::ui::tokens::colors;

/// Icon size multiplier (150% of base size)
const ICON_SIZE_MULTIPLIER: f32 = 1.5;
/// Base icon size
const BASE_ICON_SIZE: f32 = 12.0;
/// Legend icon size (150% of base)
const LEGEND_ICON_SIZE: f32 = BASE_ICON_SIZE * ICON_SIZE_MULTIPLIER;

/// A single legend entry with icon and description
pub struct LegendEntry {
    pub icon: &'static str,
    pub label: &'static str,
    pub color: Option<Color32>,
}

impl LegendEntry {
    pub fn new(icon: &'static str, label: &'static str) -> Self {
        Self {
            icon,
            label,
            color: None,
        }
    }

    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

/// Display a compact horizontal legend
pub fn show_legend(ui: &mut Ui, entries: &[LegendEntry]) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Legend:").color(colors::TEXT_SECONDARY).small());
        ui.add_space(4.0);

        for (i, entry) in entries.iter().enumerate() {
            if i > 0 {
                ui.label(RichText::new("|").color(colors::TEXT_DISABLED).small());
            }

            let icon_text = if let Some(color) = entry.color {
                RichText::new(entry.icon).color(color).size(LEGEND_ICON_SIZE)
            } else {
                RichText::new(entry.icon).size(LEGEND_ICON_SIZE)
            };

            ui.label(icon_text);
            ui.label(RichText::new(entry.label).color(colors::TEXT_SECONDARY).small());
        }
    });
}

/// Display a vertical legend (for sidebars or detail panels)
pub fn show_legend_vertical(ui: &mut Ui, title: &str, entries: &[LegendEntry]) {
    ui.label(RichText::new(title).strong().small());

    for entry in entries {
        ui.horizontal(|ui| {
            let icon_text = if let Some(color) = entry.color {
                RichText::new(entry.icon).color(color).size(LEGEND_ICON_SIZE)
            } else {
                RichText::new(entry.icon).size(LEGEND_ICON_SIZE)
            };

            ui.label(icon_text);
            ui.label(RichText::new(entry.label).color(colors::TEXT_SECONDARY).small());
        });
    }
}

// =========================================================================
// Common icon constants using egui_phosphor (non-circle variants)
// =========================================================================

/// Phosphor icons for status indicators
pub mod icons {
    /// Collected/verified/success
    pub const COLLECTED: &str = egui_phosphor::regular::CHECK;
    /// Not collected/failed
    pub const NOT_COLLECTED: &str = egui_phosphor::regular::X;
    /// Unknown/uncertain status
    pub const UNKNOWN: &str = egui_phosphor::regular::QUESTION;
    /// Mismatch/warning
    pub const MISMATCH: &str = egui_phosphor::regular::WARNING;
    /// No data/not applicable
    pub const NO_DATA: &str = egui_phosphor::regular::MINUS;
    /// Partial/approximate
    pub const PARTIAL: &str = egui_phosphor::regular::ASTERISK;
    /// High confidence
    pub const HIGH_CONFIDENCE: &str = egui_phosphor::regular::CHECK;
    /// Low confidence
    pub const LOW_CONFIDENCE: &str = egui_phosphor::regular::DOTS_THREE;
}
