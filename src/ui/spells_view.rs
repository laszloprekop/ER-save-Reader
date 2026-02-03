pub mod spells_view {
    use eframe::egui::{self, Ui, Color32, RichText};
    use crate::db::spells::{SPELLS, SpellType};
    use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData, SortDirection};
    use crate::ui::components::filter::{FilterBar, FilterBarState, FilterOption, fuzzy_match_default};
    use crate::ui::components::export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
    use crate::ui::tokens::{spacing, colors};
    use serde::Serialize;

    #[derive(Clone, Copy, PartialEq)]
    pub enum SpellFilter {
        All,
        Sorceries,
        Incantations,
    }

    impl SpellFilter {
        fn to_filter_value(&self) -> &'static str {
            match self {
                SpellFilter::All => "All",
                SpellFilter::Sorceries => "Sorceries",
                SpellFilter::Incantations => "Incantations",
            }
        }

        fn from_filter_value(s: &str) -> Self {
            match s {
                "Sorceries" => SpellFilter::Sorceries,
                "Incantations" => SpellFilter::Incantations,
                _ => SpellFilter::All,
            }
        }
    }

    pub struct SpellsViewState {
        pub filter: SpellFilter,
        pub search: String,
        pub selected_id: Option<u32>,
        pub table_state: TableState,
        pub filter_state: FilterBarState,
        pub export_format: ExportFormat,
        pub export_filtered_only: bool,
    }

    impl Default for SpellsViewState {
        fn default() -> Self {
            Self {
                filter: SpellFilter::All,
                search: String::new(),
                selected_id: None,
                table_state: TableState::new().with_sort("id", SortDirection::Ascending),
                filter_state: FilterBarState::new(),
                export_format: ExportFormat::Json,
                export_filtered_only: false,
            }
        }
    }

    #[derive(Serialize)]
    struct SpellExportItem {
        id: u32,
        name: String,
        spell_type: String,
        fp_cost: u16,
        slots: u8,
        int_req: u8,
        fai_req: u8,
    }

    pub fn spells_view(ui: &mut Ui, state: &mut SpellsViewState) {
        // Sync filter_state.category with state.filter
        state.filter = SpellFilter::from_filter_value(&state.filter_state.category);
        state.search = state.filter_state.search.clone();

        // Filter bar
        FilterBar::new("spells_filter", &mut state.filter_state)
            .category_strings("Type", &["Sorceries", "Incantations"])
            .search("Search spells...")
            .show(ui);

        spacing::space_sm(ui);

        // Export toolbar
        let export_response = ExportToolbar::new("spells_export", &mut state.export_format, &mut state.export_filtered_only)
            .has_filters(state.filter_state.has_active_filters())
            .show(ui);

        spacing::space_sm(ui);

        // Build spell data with filtering and sorting
        let search_lower = state.search.to_lowercase();
        let mut spells: Vec<(u32, &crate::db::spells::SpellInfo)> = SPELLS.iter()
            .filter(|(_, spell)| {
                // Type filter
                let type_match = match state.filter {
                    SpellFilter::All => true,
                    SpellFilter::Sorceries => spell.spell_type == SpellType::Sorcery,
                    SpellFilter::Incantations => spell.spell_type == SpellType::Incantation,
                };
                if !type_match {
                    return false;
                }

                // Search filter
                if !state.search.is_empty() {
                    if !fuzzy_match_default(&spell.name, &state.search) {
                        return false;
                    }
                }

                true
            })
            .map(|(id, spell)| (*id, spell))
            .collect();

        // Apply sorting
        if let Some(sort_col) = &state.table_state.sort_column {
            let asc = state.table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "id" => spells.sort_by(|a, b| if asc { a.0.cmp(&b.0) } else { b.0.cmp(&a.0) }),
                "name" => spells.sort_by(|a, b| if asc { a.1.name.cmp(&b.1.name) } else { b.1.name.cmp(&a.1.name) }),
                "type" => spells.sort_by(|a, b| {
                    let ta = format!("{:?}", a.1.spell_type);
                    let tb = format!("{:?}", b.1.spell_type);
                    if asc { ta.cmp(&tb) } else { tb.cmp(&ta) }
                }),
                "fp" => spells.sort_by(|a, b| if asc { a.1.fp_cost.cmp(&b.1.fp_cost) } else { b.1.fp_cost.cmp(&a.1.fp_cost) }),
                "slots" => spells.sort_by(|a, b| if asc { a.1.slots.cmp(&b.1.slots) } else { b.1.slots.cmp(&a.1.slots) }),
                "int" => spells.sort_by(|a, b| if asc { a.1.int_req.cmp(&b.1.int_req) } else { b.1.int_req.cmp(&a.1.int_req) }),
                "fth" => spells.sort_by(|a, b| if asc { a.1.fai_req.cmp(&b.1.fai_req) } else { b.1.fai_req.cmp(&a.1.fai_req) }),
                _ => {}
            }
        }

        // Summary
        let total_count = SPELLS.len();
        let filtered_count = spells.len();
        if filtered_count < total_count {
            ui.label(RichText::new(format!("Spells: {} (showing {}/{})", total_count, filtered_count, total_count)).strong());
        } else {
            ui.label(RichText::new(format!("Spells: {}", total_count)).strong());
        }

        spacing::space_sm(ui);

        // Build row data
        let rows: Vec<RowData> = spells.iter().map(|(id, spell)| {
            let type_str = match spell.spell_type {
                SpellType::Sorcery => "Sorcery",
                SpellType::Incantation => "Incantation",
            };

            let is_selected = state.selected_id == Some(*id);

            let mut row = RowData::new(vec![
                id.to_string(),
                spell.name.to_string(),
                type_str.to_string(),
                spell.fp_cost.to_string(),
                spell.slots.to_string(),
                spell.int_req.to_string(),
                spell.fai_req.to_string(),
            ]);

            if is_selected {
                row = row.with_color(Color32::YELLOW);
            }

            row
        }).collect();

        // Show table
        let table_response = UnifiedTable::new("spells_table", &mut state.table_state)
            .columns(vec![
                Column::new("id", "ID").width(60.0).sortable(true).monospace(true),
                Column::new("name", "Name").width_fraction(0.3).sortable(true),
                Column::new("type", "Type").width(100.0).sortable(true),
                Column::new("fp", "FP").width(50.0).sortable(true).right(),
                Column::new("slots", "Slots").width(50.0).sortable(true).right(),
                Column::new("int", "INT").width(50.0).sortable(true).right(),
                Column::new("fth", "FTH").width(50.0).sortable(true).right(),
            ])
            .rows(rows)
            .zebra_stripe(true)
            .selectable(true)
            .show(ui);

        // Handle copy
        if let Some(text) = table_response.clipboard_text {
            ui.output_mut(|o| o.copied_text = text);
        }

        // Handle double-click
        if let Some(row_idx) = table_response.double_clicked_row {
            if let Some((id, _)) = spells.get(row_idx) {
                // Copy row on double-click
                let spell = &spells[row_idx].1;
                let type_str = match spell.spell_type {
                    SpellType::Sorcery => "Sorcery",
                    SpellType::Incantation => "Incantation",
                };
                let row_text = format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    id, spell.name, type_str, spell.fp_cost, spell.slots, spell.int_req, spell.fai_req
                );
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Update selected_id based on table selection
        if state.table_state.selection_count() == 1 {
            if let Some(&idx) = state.table_state.selected_rows.iter().next() {
                if let Some((id, _)) = spells.get(idx) {
                    state.selected_id = Some(*id);
                }
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let data_to_export: Vec<_> = spells.iter()
                .map(|(id, spell)| SpellExportItem {
                    id: *id,
                    name: spell.name.to_string(),
                    spell_type: match spell.spell_type {
                        SpellType::Sorcery => "Sorcery".to_string(),
                        SpellType::Incantation => "Incantation".to_string(),
                    },
                    fp_cost: spell.fp_cost,
                    slots: spell.slots,
                    int_req: spell.int_req,
                    fai_req: spell.fai_req,
                })
                .collect();

            let content = match state.export_format {
                ExportFormat::Json => {
                    let export = PageExport::new(
                        PageExportMetadata::new("Spells")
                            .with_counts(total_count, filtered_count),
                        &data_to_export,
                    );
                    to_json(&export).unwrap_or_else(|_| String::new())
                }
                ExportFormat::Csv => {
                    let headers = &["ID", "Name", "Type", "FP", "Slots", "INT", "FTH"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|s| vec![
                            s.id.to_string(),
                            s.name.clone(),
                            s.spell_type.clone(),
                            s.fp_cost.to_string(),
                            s.slots.to_string(),
                            s.int_req.to_string(),
                            s.fai_req.to_string(),
                        ])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["ID", "Name", "Type", "FP", "Slots", "INT", "FTH"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|s| vec![
                            s.id.to_string(),
                            s.name.clone(),
                            s.spell_type.clone(),
                            s.fp_cost.to_string(),
                            s.slots.to_string(),
                            s.int_req.to_string(),
                            s.fai_req.to_string(),
                        ])
                        .collect();
                    to_markdown(headers, &rows)
                }
            };

            if export_response.copy_clicked {
                ui.output_mut(|o| o.copied_text = content);
            }
            // TODO: File save dialog for export_clicked
        }
    }
}
