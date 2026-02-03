//! Builder API for unified tables.

use eframe::egui::{self, Color32, RichText, Sense, Ui};
use egui_extras::{Column as EguiColumn, TableBuilder};
use super::column::{Column, ColumnWidth};
use super::state::TableState;
use crate::ui::tokens::{colors, typography, dimensions};

/// Icon size multiplier (150% of base size)
const ICON_SIZE_MULTIPLIER: f32 = 1.5;

/// Row data for rendering
pub struct RowData {
    /// Cell values for each column
    pub cells: Vec<String>,
    /// Optional row color override
    pub color: Option<Color32>,
}

impl RowData {
    pub fn new(cells: Vec<String>) -> Self {
        Self { cells, color: None }
    }

    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

/// Builder for creating unified tables
pub struct UnifiedTable<'a> {
    id: String,
    state: &'a mut TableState,
    columns: Vec<Column>,
    rows: Vec<RowData>,
    zebra_stripe: bool,
    selectable: bool,
    resizable: bool,
    striped: bool,
    row_height: f32,
}

impl<'a> UnifiedTable<'a> {
    /// Create a new unified table
    pub fn new(id: impl Into<String>, state: &'a mut TableState) -> Self {
        Self {
            id: id.into(),
            state,
            columns: Vec::new(),
            rows: Vec::new(),
            zebra_stripe: true,
            selectable: true,
            resizable: true,
            striped: true,
            row_height: dimensions::TABLE_ROW_HEIGHT,
        }
    }

    /// Set column definitions
    pub fn columns(mut self, columns: Vec<Column>) -> Self {
        self.columns = columns;
        self
    }

    /// Add a single column
    pub fn column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    /// Set row data
    pub fn rows(mut self, rows: Vec<RowData>) -> Self {
        self.rows = rows;
        self
    }

    /// Enable/disable zebra striping
    pub fn zebra_stripe(mut self, enabled: bool) -> Self {
        self.zebra_stripe = enabled;
        self.striped = enabled;
        self
    }

    /// Enable/disable row selection
    pub fn selectable(mut self, enabled: bool) -> Self {
        self.selectable = enabled;
        self
    }

    /// Enable/disable column resizing
    pub fn resizable(mut self, enabled: bool) -> Self {
        self.resizable = enabled;
        self
    }

    /// Set row height
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }

    /// Show the table
    pub fn show(self, ui: &mut Ui) -> TableResponse {
        let mut response = TableResponse::default();

        // Handle keyboard shortcuts
        if self.selectable {
            let ctx = ui.ctx();
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A)) {
                self.state.select_all(self.rows.len());
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C)) && self.state.has_selection() {
                response.copy_requested = true;
            }
        }

        // Build the table
        let available_width = ui.available_width();
        let mut builder = TableBuilder::new(ui)
            .striped(self.striped)
            .resizable(self.resizable)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .min_scrolled_height(0.0);

        // Add columns
        for col in &self.columns {
            if !col.visible {
                continue;
            }

            let egui_col = match col.width {
                ColumnWidth::Fixed(w) => {
                    EguiColumn::initial(w).at_least(col.min_width).resizable(self.resizable)
                }
                ColumnWidth::Fraction(f) => {
                    let w = available_width * f;
                    EguiColumn::initial(w).at_least(col.min_width).resizable(self.resizable)
                }
                ColumnWidth::Auto => {
                    EguiColumn::auto().at_least(col.min_width).resizable(self.resizable)
                }
            };
            builder = builder.column(egui_col);
        }

        // Render table
        builder
            .header(dimensions::TABLE_HEADER_HEIGHT, |mut header| {
                for col in &self.columns {
                    if !col.visible {
                        continue;
                    }

                    header.col(|ui| {
                        let header_text = if col.sortable && self.state.is_sorted_by(&col.id) {
                            let icon = self.state.sort_direction.icon();
                            format!("{} {}", col.header, icon)
                        } else {
                            col.header.clone()
                        };

                        let label = RichText::new(&header_text)
                            .color(colors::TABLE_HEADER_TEXT)
                            .size(typography::TABLE_HEADER);

                        if col.sortable {
                            let resp = ui.add(
                                egui::Label::new(label)
                                    .sense(Sense::click())
                            );
                            if resp.clicked() {
                                self.state.toggle_sort(&col.id);
                                response.sort_changed = true;
                            }
                            if resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                        } else {
                            ui.label(label);
                        }
                    });
                }
            })
            .body(|body| {
                body.rows(self.row_height, self.rows.len(), |mut row| {
                    let row_index = row.index();
                    let is_selected = self.state.is_selected(row_index);

                    let row_data = &self.rows[row_index];

                    for (col_idx, col) in self.columns.iter().filter(|c| c.visible).enumerate() {
                        row.col(|ui| {
                            // Only paint background for selected rows
                            // Zebra striping is handled by egui's built-in .striped(true)
                            if is_selected {
                                let rect = ui.available_rect_before_wrap();
                                ui.painter().rect_filled(rect, 0.0, colors::TABLE_ROW_SELECTED);
                            }

                            // Get cell text
                            let cell_text = row_data.cells.get(col_idx)
                                .cloned()
                                .unwrap_or_default();

                            // Determine text color
                            let text_color = if is_selected {
                                Color32::WHITE
                            } else {
                                row_data.color.unwrap_or(colors::TEXT_PRIMARY)
                            };

                            // Create text with appropriate font and size
                            let text = if col.icon {
                                // Icon columns use 150% size
                                RichText::new(&cell_text)
                                    .color(text_color)
                                    .size(typography::TABLE_CONTENT * ICON_SIZE_MULTIPLIER)
                            } else if col.monospace {
                                RichText::new(&cell_text)
                                    .color(text_color)
                                    .size(typography::MONO_CONTENT)
                                    .monospace()
                            } else {
                                RichText::new(&cell_text)
                                    .color(text_color)
                                    .size(typography::TABLE_CONTENT)
                            };

                            let label_resp = ui.add(
                                egui::Label::new(text)
                                    .sense(if self.selectable { Sense::click() } else { Sense::hover() })
                            );

                            // Handle selection
                            if self.selectable && label_resp.clicked() {
                                let modifiers = ui.input(|i| i.modifiers);
                                if modifiers.shift {
                                    self.state.extend_selection(row_index);
                                } else if modifiers.command {
                                    self.state.toggle_row(row_index);
                                } else {
                                    self.state.select_row(row_index);
                                }
                            }

                            // Handle double-click to copy
                            if label_resp.double_clicked() {
                                response.double_clicked_row = Some(row_index);
                            }
                        });
                    }
                });
            });

        // Build clipboard content if copy requested
        if response.copy_requested && self.state.has_selection() {
            response.clipboard_text = Some(self.build_clipboard_text());
        }

        response
    }

    /// Build clipboard text for selected rows
    fn build_clipboard_text(&self) -> String {
        let mut lines = Vec::new();

        // Header line
        let headers: Vec<&str> = self.columns.iter()
            .filter(|c| c.visible)
            .map(|c| c.header.as_str())
            .collect();
        lines.push(headers.join("\t"));

        // Selected rows
        let mut selected: Vec<usize> = self.state.selected_rows.iter().copied().collect();
        selected.sort();

        for idx in selected {
            if let Some(row) = self.rows.get(idx) {
                lines.push(row.cells.join("\t"));
            }
        }

        lines.join("\n")
    }
}

/// Response from showing a unified table
#[derive(Default)]
pub struct TableResponse {
    /// Whether the sort column or direction changed
    pub sort_changed: bool,
    /// Whether a copy was requested (Cmd+C)
    pub copy_requested: bool,
    /// Text to copy to clipboard (if copy_requested)
    pub clipboard_text: Option<String>,
    /// Row index that was double-clicked
    pub double_clicked_row: Option<usize>,
}
