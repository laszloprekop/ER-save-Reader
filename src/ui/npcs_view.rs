pub mod npcs_view {
    use eframe::egui::{self, Ui, Color32, RichText};
    use crate::db::npcs::{NPCS, NpcType};
    use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData, SortDirection};
    use crate::ui::components::filter::{FilterBar, FilterBarState, fuzzy_match_default};
    use crate::ui::components::export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
    use crate::ui::tokens::spacing;
    use serde::Serialize;

    #[derive(Clone, Copy, PartialEq)]
    pub enum NpcFilter {
        All,
        Merchants,
        QuestNpcs,
        RoundtableNpcs,
        Invaders,
    }

    impl NpcFilter {
        fn from_filter_value(s: &str) -> Self {
            match s {
                "Merchants" => NpcFilter::Merchants,
                "Quest NPCs" => NpcFilter::QuestNpcs,
                "Roundtable" => NpcFilter::RoundtableNpcs,
                "Invaders" => NpcFilter::Invaders,
                _ => NpcFilter::All,
            }
        }
    }

    pub struct NpcsViewState {
        pub filter: NpcFilter,
        pub search: String,
        pub selected_id: Option<u32>,
        pub table_state: TableState,
        pub filter_state: FilterBarState,
        pub export_format: ExportFormat,
        pub export_filtered_only: bool,
    }

    impl Default for NpcsViewState {
        fn default() -> Self {
            Self {
                filter: NpcFilter::All,
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
    struct NpcExportItem {
        id: u32,
        name: String,
        npc_type: String,
        location: String,
        discovery_flag: Option<u32>,
        death_flag: Option<u32>,
    }

    pub fn npcs_view(ui: &mut Ui, state: &mut NpcsViewState) {
        // Sync filter state
        state.filter = NpcFilter::from_filter_value(&state.filter_state.category);
        state.search = state.filter_state.search.clone();

        // Filter bar
        FilterBar::new("npcs_filter", &mut state.filter_state)
            .category_strings("Type", &["Merchants", "Quest NPCs", "Roundtable", "Invaders"])
            .search("Search NPCs...")
            .show(ui);

        spacing::space_sm(ui);

        // Export toolbar
        let export_response = ExportToolbar::new("npcs_export", &mut state.export_format, &mut state.export_filtered_only)
            .has_filters(state.filter_state.has_active_filters())
            .show(ui);

        spacing::space_sm(ui);

        // Build NPC data with filtering and sorting
        let mut npcs: Vec<(u32, &crate::db::npcs::NpcInfo)> = NPCS.iter()
            .filter(|(_, npc)| {
                // Type filter
                let type_match = match state.filter {
                    NpcFilter::All => true,
                    NpcFilter::Merchants => npc.npc_type == NpcType::Merchant,
                    NpcFilter::QuestNpcs => npc.npc_type == NpcType::QuestNpc,
                    NpcFilter::RoundtableNpcs => npc.npc_type == NpcType::RoundtableNpc,
                    NpcFilter::Invaders => npc.npc_type == NpcType::Invader,
                };
                if !type_match {
                    return false;
                }

                // Search filter
                if !state.search.is_empty() {
                    if !fuzzy_match_default(&npc.name, &state.search) &&
                       !fuzzy_match_default(&npc.location, &state.search) {
                        return false;
                    }
                }

                true
            })
            .map(|(id, npc)| (*id, npc))
            .collect();

        // Apply sorting
        if let Some(sort_col) = &state.table_state.sort_column {
            let asc = state.table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "id" => npcs.sort_by(|a, b| if asc { a.0.cmp(&b.0) } else { b.0.cmp(&a.0) }),
                "name" => npcs.sort_by(|a, b| if asc { a.1.name.cmp(&b.1.name) } else { b.1.name.cmp(&a.1.name) }),
                "type" => npcs.sort_by(|a, b| {
                    let ta = format!("{:?}", a.1.npc_type);
                    let tb = format!("{:?}", b.1.npc_type);
                    if asc { ta.cmp(&tb) } else { tb.cmp(&ta) }
                }),
                "location" => npcs.sort_by(|a, b| if asc { a.1.location.cmp(&b.1.location) } else { b.1.location.cmp(&a.1.location) }),
                "discovery" => npcs.sort_by(|a, b| if asc { a.1.discovery_flag.cmp(&b.1.discovery_flag) } else { b.1.discovery_flag.cmp(&a.1.discovery_flag) }),
                "death" => npcs.sort_by(|a, b| if asc { a.1.death_flag.cmp(&b.1.death_flag) } else { b.1.death_flag.cmp(&a.1.death_flag) }),
                _ => {}
            }
        }

        // Summary
        let total_count = NPCS.len();
        let filtered_count = npcs.len();
        if filtered_count < total_count {
            ui.label(RichText::new(format!("NPCs: {} (showing {}/{})", total_count, filtered_count, total_count)).strong());
        } else {
            ui.label(RichText::new(format!("NPCs: {}", total_count)).strong());
        }

        spacing::space_sm(ui);

        // Build row data
        let rows: Vec<RowData> = npcs.iter().map(|(id, npc)| {
            let type_str = match npc.npc_type {
                NpcType::Merchant => "Merchant",
                NpcType::QuestNpc => "Quest NPC",
                NpcType::RoundtableNpc => "Roundtable",
                NpcType::Invader => "Invader",
                NpcType::Boss => "Boss",
                NpcType::Spirit => "Spirit",
            };

            let discovery_str = npc.discovery_flag.map(|f| f.to_string()).unwrap_or("-".to_string());
            let death_str = npc.death_flag.map(|f| f.to_string()).unwrap_or("-".to_string());

            let is_selected = state.selected_id == Some(*id);

            let mut row = RowData::new(vec![
                id.to_string(),
                npc.name.to_string(),
                type_str.to_string(),
                npc.location.to_string(),
                discovery_str,
                death_str,
            ]);

            if is_selected {
                row = row.with_color(Color32::YELLOW);
            }

            row
        }).collect();

        // Show table
        let table_response = UnifiedTable::new("npcs_table", &mut state.table_state)
            .columns(vec![
                Column::new("id", "ID").width(60.0).sortable(true).monospace(true),
                Column::new("name", "Name").width_fraction(0.25).sortable(true),
                Column::new("type", "Type").width(100.0).sortable(true),
                Column::new("location", "Location").width_fraction(0.25).sortable(true),
                Column::new("discovery", "Discovery Flag").width(100.0).sortable(true).monospace(true),
                Column::new("death", "Death Flag").width(100.0).sortable(true).monospace(true),
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
            if let Some((id, _)) = npcs.get(row_idx) {
                let npc = &npcs[row_idx].1;
                let type_str = match npc.npc_type {
                    NpcType::Merchant => "Merchant",
                    NpcType::QuestNpc => "Quest NPC",
                    NpcType::RoundtableNpc => "Roundtable",
                    NpcType::Invader => "Invader",
                    NpcType::Boss => "Boss",
                    NpcType::Spirit => "Spirit",
                };
                let row_text = format!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    id, npc.name, type_str, npc.location,
                    npc.discovery_flag.map(|f| f.to_string()).unwrap_or("-".to_string()),
                    npc.death_flag.map(|f| f.to_string()).unwrap_or("-".to_string())
                );
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Update selected_id
        if state.table_state.selection_count() == 1 {
            if let Some(&idx) = state.table_state.selected_rows.iter().next() {
                if let Some((id, _)) = npcs.get(idx) {
                    state.selected_id = Some(*id);
                }
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let data_to_export: Vec<_> = npcs.iter()
                .map(|(id, npc)| NpcExportItem {
                    id: *id,
                    name: npc.name.to_string(),
                    npc_type: match npc.npc_type {
                        NpcType::Merchant => "Merchant".to_string(),
                        NpcType::QuestNpc => "Quest NPC".to_string(),
                        NpcType::RoundtableNpc => "Roundtable".to_string(),
                        NpcType::Invader => "Invader".to_string(),
                        NpcType::Boss => "Boss".to_string(),
                        NpcType::Spirit => "Spirit".to_string(),
                    },
                    location: npc.location.to_string(),
                    discovery_flag: npc.discovery_flag,
                    death_flag: npc.death_flag,
                })
                .collect();

            let content = match state.export_format {
                ExportFormat::Json => {
                    let export = PageExport::new(
                        PageExportMetadata::new("NPCs")
                            .with_counts(total_count, filtered_count),
                        &data_to_export,
                    );
                    to_json(&export).unwrap_or_else(|_| String::new())
                }
                ExportFormat::Csv => {
                    let headers = &["ID", "Name", "Type", "Location", "Discovery Flag", "Death Flag"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|n| vec![
                            n.id.to_string(),
                            n.name.to_string(),
                            n.npc_type.clone(),
                            n.location.to_string(),
                            n.discovery_flag.map(|f| f.to_string()).unwrap_or_default(),
                            n.death_flag.map(|f| f.to_string()).unwrap_or_default(),
                        ])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["ID", "Name", "Type", "Location", "Discovery Flag", "Death Flag"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|n| vec![
                            n.id.to_string(),
                            n.name.to_string(),
                            n.npc_type.clone(),
                            n.location.to_string(),
                            n.discovery_flag.map(|f| f.to_string()).unwrap_or_default(),
                            n.death_flag.map(|f| f.to_string()).unwrap_or_default(),
                        ])
                        .collect();
                    to_markdown(headers, &rows)
                }
            };

            if export_response.copy_clicked {
                ui.output_mut(|o| o.copied_text = content);
            }
        }
    }
}
