//! Filter bar builder and rendering.

use eframe::egui::{self, Color32, RichText, Ui};
use super::dimension::{CompletionStatus, FilterOption};
use super::state::FilterBarState;
use crate::ui::tokens::{colors, typography, dimensions, spacing};

/// Builder for creating filter bars
pub struct FilterBar<'a> {
    id: String,
    state: &'a mut FilterBarState,
    show_completion: bool,
    completion_variants: &'static [CompletionStatus],
    category_options: Vec<FilterOption>,
    category_label: String,
    region_options: Vec<FilterOption>,
    region_label: String,
    search_placeholder: String,
    show_search: bool,
    show_clear: bool,
}

impl<'a> FilterBar<'a> {
    /// Create a new filter bar builder
    pub fn new(id: impl Into<String>, state: &'a mut FilterBarState) -> Self {
        Self {
            id: id.into(),
            state,
            show_completion: false,
            completion_variants: CompletionStatus::all_variants(),
            category_options: Vec::new(),
            category_label: "Category".to_string(),
            region_options: Vec::new(),
            region_label: "Region".to_string(),
            search_placeholder: "Search...".to_string(),
            show_search: true,
            show_clear: true,
        }
    }

    /// Add completion status filter (button group)
    pub fn completion_filter(mut self) -> Self {
        self.show_completion = true;
        self
    }

    /// Use possession-style completion variants
    pub fn possession_filter(mut self) -> Self {
        self.show_completion = true;
        self.completion_variants = CompletionStatus::possession_variants();
        self
    }

    /// Add category dropdown filter
    pub fn category(mut self, label: impl Into<String>, options: Vec<FilterOption>) -> Self {
        self.category_label = label.into();
        self.category_options = options;
        self
    }

    /// Add category dropdown from string options
    pub fn category_strings(mut self, label: impl Into<String>, options: &[&str]) -> Self {
        self.category_label = label.into();
        self.category_options = std::iter::once(FilterOption::all())
            .chain(options.iter().map(|s| FilterOption::from_str(*s)))
            .collect();
        self
    }

    /// Add region/area dropdown filter
    pub fn region(mut self, label: impl Into<String>, options: Vec<FilterOption>) -> Self {
        self.region_label = label.into();
        self.region_options = options;
        self
    }

    /// Add region dropdown from string options
    pub fn region_strings(mut self, label: impl Into<String>, options: &[&str]) -> Self {
        self.region_label = label.into();
        self.region_options = std::iter::once(FilterOption::all())
            .chain(options.iter().map(|s| FilterOption::from_str(*s)))
            .collect();
        self
    }

    /// Set search placeholder text
    pub fn search(mut self, placeholder: impl Into<String>) -> Self {
        self.search_placeholder = placeholder.into();
        self.show_search = true;
        self
    }

    /// Disable search field
    pub fn no_search(mut self) -> Self {
        self.show_search = false;
        self
    }

    /// Disable clear button
    pub fn no_clear(mut self) -> Self {
        self.show_clear = false;
        self
    }

    /// Show the filter bar
    pub fn show(self, ui: &mut Ui) -> FilterBarResponse {
        let mut response = FilterBarResponse::default();

        // First row: completion status (if enabled)
        if self.show_completion {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Status:").color(colors::TEXT_LABEL));

                for variant in self.completion_variants {
                    let is_selected = self.state.completion == *variant;
                    let text = variant.label();

                    if ui.selectable_label(is_selected, text).clicked() {
                        self.state.completion = *variant;
                        response.changed = true;
                    }
                }
            });
            spacing::space_xs(ui);
        }

        // Second row: category, region, search
        ui.horizontal(|ui| {
            // Category dropdown
            if !self.category_options.is_empty() {
                ui.label(RichText::new(format!("{}:", self.category_label)).color(colors::TEXT_LABEL));

                let current_label = self.category_options.iter()
                    .find(|o| o.value == self.state.category)
                    .map(|o| o.label.as_str())
                    .unwrap_or("All");

                egui::ComboBox::from_id_salt(format!("{}_category", self.id))
                    .selected_text(current_label)
                    .width(dimensions::FILTER_DROPDOWN_WIDTH)
                    .show_ui(ui, |ui| {
                        for option in &self.category_options {
                            if ui.selectable_value(&mut self.state.category, option.value.clone(), &option.label).changed() {
                                response.changed = true;
                            }
                        }
                    });

                ui.add_space(spacing::SM);
            }

            // Region dropdown
            if !self.region_options.is_empty() {
                ui.label(RichText::new(format!("{}:", self.region_label)).color(colors::TEXT_LABEL));

                let current_label = self.region_options.iter()
                    .find(|o| o.value == self.state.region)
                    .map(|o| o.label.as_str())
                    .unwrap_or("All");

                egui::ComboBox::from_id_salt(format!("{}_region", self.id))
                    .selected_text(current_label)
                    .width(dimensions::FILTER_DROPDOWN_WIDTH)
                    .show_ui(ui, |ui| {
                        for option in &self.region_options {
                            if ui.selectable_value(&mut self.state.region, option.value.clone(), &option.label).changed() {
                                response.changed = true;
                            }
                        }
                    });

                ui.add_space(spacing::SM);
            }

            // Search field
            if self.show_search {
                ui.label(RichText::new("Search:").color(colors::TEXT_LABEL));

                let search_edit = egui::TextEdit::singleline(&mut self.state.search)
                    .hint_text(&self.search_placeholder)
                    .desired_width(dimensions::FILTER_SEARCH_WIDTH);

                if ui.add(search_edit).changed() {
                    response.changed = true;
                }
            }

            // Clear button
            if self.show_clear && self.state.has_active_filters() {
                ui.add_space(spacing::SM);
                if ui.small_button("Clear filters").clicked() {
                    self.state.reset();
                    response.changed = true;
                    response.cleared = true;
                }
            }
        });

        response
    }
}

/// Response from showing a filter bar
#[derive(Default)]
pub struct FilterBarResponse {
    /// Whether any filter value changed
    pub changed: bool,
    /// Whether the clear button was clicked
    pub cleared: bool,
}
