//! State management for character comparison view.

use crate::ui::components::export::ExportFormat;

/// Comparison tabs for different data categories.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ComparisonTab {
    #[default]
    Stats,
    EventFlags,
    Inventory,
}

impl ComparisonTab {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Stats => "Stats",
            Self::EventFlags => "Event Flags",
            Self::Inventory => "Inventory",
        }
    }
}

/// State for the comparison view.
#[derive(Debug, Default)]
pub struct ComparisonState {
    /// First slot index for comparison (None = not selected).
    pub slot_a: Option<usize>,
    /// Second slot index for comparison (None = not selected).
    pub slot_b: Option<usize>,
    /// Currently active comparison tab.
    pub active_tab: ComparisonTab,
    /// Show only differences (hide items that are the same).
    pub show_differences_only: bool,
    /// Search filter for event flags/inventory.
    pub search_query: String,
    /// Export format selection.
    pub export_format: ExportFormat,
}

impl ComparisonState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if both slots are selected and different.
    pub fn can_compare(&self) -> bool {
        match (self.slot_a, self.slot_b) {
            (Some(a), Some(b)) => a != b,
            _ => false,
        }
    }

    /// Reset comparison (clear slot selections).
    pub fn reset(&mut self) {
        self.slot_a = None;
        self.slot_b = None;
        self.search_query.clear();
    }
}
