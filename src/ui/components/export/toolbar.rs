//! Export toolbar component.

use eframe::egui::{self, Ui};
use super::formats::ExportFormat;
use crate::ui::tokens::spacing;

/// Builder for export toolbar
pub struct ExportToolbar<'a> {
    id: String,
    format: &'a mut ExportFormat,
    filtered_only: &'a mut bool,
    show_format_dropdown: bool,
    show_filtered_checkbox: bool,
    has_filters: bool,
}

impl<'a> ExportToolbar<'a> {
    /// Create a new export toolbar
    pub fn new(
        id: impl Into<String>,
        format: &'a mut ExportFormat,
        filtered_only: &'a mut bool,
    ) -> Self {
        Self {
            id: id.into(),
            format,
            filtered_only,
            show_format_dropdown: true,
            show_filtered_checkbox: true,
            has_filters: false,
        }
    }

    /// Set whether there are active filters
    pub fn has_filters(mut self, has_filters: bool) -> Self {
        self.has_filters = has_filters;
        self
    }

    /// Hide the format dropdown (JSON only)
    pub fn json_only(mut self) -> Self {
        self.show_format_dropdown = false;
        *self.format = ExportFormat::Json;
        self
    }

    /// Hide the filtered checkbox
    pub fn no_filter_option(mut self) -> Self {
        self.show_filtered_checkbox = false;
        self
    }

    /// Show the toolbar
    pub fn show(self, ui: &mut Ui) -> ExportToolbarResponse {
        let mut response = ExportToolbarResponse::default();

        ui.horizontal(|ui| {
            // Export button
            if ui.button(format!(
                "{} Export {}",
                egui_phosphor::regular::EXPORT,
                self.format.label()
            )).clicked() {
                response.export_clicked = true;
            }

            // Format dropdown
            if self.show_format_dropdown {
                egui::ComboBox::from_id_salt(format!("{}_format", self.id))
                    .selected_text(self.format.label())
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(self.format, ExportFormat::Json, "JSON");
                        ui.selectable_value(self.format, ExportFormat::Csv, "CSV");
                        ui.selectable_value(self.format, ExportFormat::Markdown, "Markdown");
                    });
            }

            // Copy to clipboard button
            if ui.button(format!("{} Copy", egui_phosphor::regular::CLIPBOARD)).clicked() {
                response.copy_clicked = true;
            }

            // Filtered only checkbox
            if self.show_filtered_checkbox && self.has_filters {
                ui.add_space(spacing::SM);
                ui.checkbox(self.filtered_only, "Filtered only");
            }
        });

        response
    }
}

/// Response from export toolbar
#[derive(Default)]
pub struct ExportToolbarResponse {
    /// Export button was clicked
    pub export_clicked: bool,
    /// Copy button was clicked
    pub copy_clicked: bool,
}
