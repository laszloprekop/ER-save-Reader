//! Sites of Grace database view.
//!
//! Shows all sites of grace with filtering by region and discovered status.

use eframe::egui::{Ui, Color32, RichText};
use crate::db::graces_data::{GRACES_DATA, GRACE_REGIONS};
use crate::db::pickup_flags::is_flag_set;
use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData, SortDirection};
use crate::ui::components::filter::{FilterBar, FilterBarState, fuzzy_match_default};
use crate::ui::components::export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
use crate::ui::components::detail_panel::{DetailPanelState, SelectedEntity, RelationshipSection, RelationshipItem, DetailPanelAction};
use crate::ui::tokens::spacing;
use serde::Serialize;

pub struct GracesViewState {
    pub region_filter: String,
    pub search: String,
    pub selected_flag: Option<u32>,
    /// Track which flag we last opened the detail panel for
    pub last_detail_flag: Option<u32>,
    pub table_state: TableState,
    pub filter_state: FilterBarState,
    pub export_format: ExportFormat,
    pub export_filtered_only: bool,
}

impl Default for GracesViewState {
    fn default() -> Self {
        Self {
            region_filter: "All".to_string(),
            search: String::new(),
            selected_flag: None,
            last_detail_flag: None,
            table_state: TableState::new().with_sort("flag", SortDirection::Ascending),
            filter_state: FilterBarState::new(),
            export_format: ExportFormat::Json,
            export_filtered_only: false,
        }
    }
}

#[derive(Serialize)]
struct GraceExportItem {
    event_flag: u32,
    name: String,
    region: String,
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    mapgenie_id: Option<String>,
}

/// Check if a grace is discovered using event flags.
///
/// Grace discovery flags are in the 71XXX-76XXX range (block flags).
/// Uses the verified block base offsets from ground_truth_offsets.json.
fn is_grace_discovered(event_flag: u32, event_flags: Option<&[u8]>) -> Option<bool> {
    event_flags.map(|flags| is_flag_set(flags, event_flag))
}

pub fn graces_view(ui: &mut Ui, state: &mut GracesViewState, event_flags: Option<&[u8]>, detail_panel: &mut DetailPanelState) {
    // Sync filter_state with state
    state.region_filter = state.filter_state.category.clone();
    state.search = state.filter_state.search.clone();

    // Build region options
    let region_options: Vec<_> = GRACE_REGIONS.iter().map(|s| *s).collect();

    // Filter bar
    FilterBar::new("graces_filter", &mut state.filter_state)
        .category_strings("Region", &region_options)
        .search("Search graces...")
        .show(ui);

    spacing::space_sm(ui);

    // Export toolbar
    let export_response = ExportToolbar::new("graces_export", &mut state.export_format, &mut state.export_filtered_only)
        .has_filters(state.filter_state.has_active_filters())
        .show(ui);

    spacing::space_sm(ui);

    // Build grace data with filtering and sorting
    let mut graces: Vec<(u32, &crate::db::graces_data::GraceData, Option<bool>)> = GRACES_DATA.iter()
        .filter_map(|(flag, grace)| {
            // Region filter
            if state.region_filter != "All" && grace.region != state.region_filter {
                return None;
            }

            // Search filter
            if !state.search.is_empty() {
                if !fuzzy_match_default(grace.name, &state.search)
                    && !fuzzy_match_default(grace.region, &state.search) {
                    return None;
                }
            }

            let discovered = is_grace_discovered(*flag, event_flags);
            Some((*flag, grace, discovered))
        })
        .collect();

    // Apply sorting
    if let Some(sort_col) = &state.table_state.sort_column {
        let asc = state.table_state.sort_direction == SortDirection::Ascending;
        match sort_col.as_str() {
            "flag" => graces.sort_by(|a, b| if asc { a.0.cmp(&b.0) } else { b.0.cmp(&a.0) }),
            "name" => graces.sort_by(|a, b| if asc { a.1.name.cmp(b.1.name) } else { b.1.name.cmp(a.1.name) }),
            "region" => graces.sort_by(|a, b| if asc { a.1.region.cmp(b.1.region) } else { b.1.region.cmp(a.1.region) }),
            "status" => graces.sort_by(|a, b| {
                let sa = a.2.map(|d| if d { 1 } else { 0 }).unwrap_or(2);
                let sb = b.2.map(|d| if d { 1 } else { 0 }).unwrap_or(2);
                if asc { sa.cmp(&sb) } else { sb.cmp(&sa) }
            }),
            _ => {}
        }
    }

    // Auto-open detail panel if selection was set programmatically (from navigation)
    if let Some(flag) = state.selected_flag {
        if state.last_detail_flag != Some(flag) {
            // Find the grace in the filtered list and open its detail panel
            if let Some((_, grace, _)) = graces.iter().find(|(f, _, _)| *f == flag) {
                let mut sections = Vec::new();

                // Add MapGenie link if available
                if let Some(mapgenie_id) = grace.mapgenie_id {
                    let mapgenie_url = format!("https://mapgenie.io/elden-ring/maps/the-lands-between?locationIds={}", mapgenie_id);
                    sections.push(
                        RelationshipSection::new("External Links").with_items(vec![
                            RelationshipItem::new(
                                format!("View on MapGenie ({})", mapgenie_id),
                                DetailPanelAction::OpenExternalUrl { url: mapgenie_url.clone() },
                            ).with_secondary(mapgenie_url)
                        ])
                    );
                }

                // Add location info
                sections.push(
                    RelationshipSection::new("Location").with_items(vec![
                        RelationshipItem::new(
                            grace.region.to_string(),
                            DetailPanelAction::None,
                        ).with_secondary(format!("{:.0}, {:.0}, {:.0}", grace.pos_x, grace.pos_y, grace.pos_z))
                    ])
                );

                detail_panel.select_with_relationships(
                    SelectedEntity::Grace {
                        event_flag: flag,
                        name: grace.name.to_string(),
                    },
                    sections,
                );
                state.last_detail_flag = Some(flag);
            }
        }
    }

    // Summary
    let total_count = GRACES_DATA.len();
    let filtered_count = graces.len();
    if filtered_count < total_count {
        ui.label(RichText::new(format!("Graces: {} (showing {}/{})", total_count, filtered_count, total_count)).strong());
    } else {
        ui.label(RichText::new(format!("Graces: {}", total_count)).strong());
    }

    spacing::space_sm(ui);

    // Build row data
    let rows: Vec<RowData> = graces.iter().map(|(flag, grace, discovered)| {
        let is_selected = state.selected_flag == Some(*flag);

        let status_str = match discovered {
            Some(true) => "✓",
            Some(false) => "○",
            None => "-",
        };

        let position = format!("{:.0}, {:.0}, {:.0}", grace.pos_x, grace.pos_y, grace.pos_z);

        let mut row = RowData::new(vec![
            flag.to_string(),
            grace.name.to_string(),
            grace.region.to_string(),
            position,
            status_str.to_string(),
        ]);

        if is_selected {
            row = row.with_color(Color32::YELLOW);
        } else if *discovered == Some(true) {
            row = row.with_color(Color32::from_rgb(144, 238, 144)); // Light green
        }

        row
    }).collect();

    // Show table with auto-width columns
    let table_response = UnifiedTable::new("graces_table", &mut state.table_state)
        .columns(vec![
            Column::new("flag", "Flag ID").sortable(true).monospace(true),
            Column::new("name", "Name").sortable(true),
            Column::new("region", "Region").sortable(true),
            Column::new("position", "Position"),
            Column::new("status", "Status").sortable(true).center(),
        ])
        .rows(rows)
        .zebra_stripe(true)
        .selectable(true)
        .show(ui);

    // Handle copy
    if let Some(text) = table_response.clipboard_text {
        ui.output_mut(|o| o.copied_text = text);
    }

    // Handle single click - open detail panel with MapGenie info
    if let Some(row_idx) = table_response.clicked_row {
        if let Some((flag, grace, _)) = graces.get(row_idx) {
            let mut sections = Vec::new();

            // Add MapGenie link if available
            if let Some(mapgenie_id) = grace.mapgenie_id {
                let mapgenie_url = format!("https://mapgenie.io/elden-ring/maps/the-lands-between?locationIds={}", mapgenie_id);
                sections.push(
                    RelationshipSection::new("External Links").with_items(vec![
                        RelationshipItem::new(
                            format!("View on MapGenie ({})", mapgenie_id),
                            DetailPanelAction::OpenExternalUrl { url: mapgenie_url.clone() },
                        ).with_secondary(mapgenie_url)
                    ])
                );
            }

            // Add location info
            sections.push(
                RelationshipSection::new("Location").with_items(vec![
                    RelationshipItem::new(
                        grace.region.to_string(),
                        DetailPanelAction::None,
                    ).with_secondary(format!("{:.0}, {:.0}, {:.0}", grace.pos_x, grace.pos_y, grace.pos_z))
                ])
            );

            detail_panel.select_with_relationships(
                SelectedEntity::Grace {
                    event_flag: *flag,
                    name: grace.name.to_string(),
                },
                sections,
            );
            state.last_detail_flag = Some(*flag);
        }
    }

    // Update selected based on table selection
    if state.table_state.selection_count() == 1 {
        if let Some(&idx) = state.table_state.selected_rows.iter().next() {
            if let Some((flag, _, _)) = graces.get(idx) {
                state.selected_flag = Some(*flag);
            }
        }
    }

    // Handle export
    if export_response.export_clicked || export_response.copy_clicked {
        let data_to_export: Vec<_> = graces.iter()
            .map(|(flag, grace, _)| GraceExportItem {
                event_flag: *flag,
                name: grace.name.to_string(),
                region: grace.region.to_string(),
                pos_x: grace.pos_x,
                pos_y: grace.pos_y,
                pos_z: grace.pos_z,
                mapgenie_id: grace.mapgenie_id.map(|s| s.to_string()),
            })
            .collect();

        let content = match state.export_format {
            ExportFormat::Json => {
                let export = PageExport::new(
                    PageExportMetadata::new("Sites of Grace")
                        .with_counts(total_count, filtered_count),
                    &data_to_export,
                );
                to_json(&export).unwrap_or_else(|_| String::new())
            }
            ExportFormat::Csv => {
                let headers = &["Flag ID", "Name", "Region", "Pos X", "Pos Y", "Pos Z", "MapGenie ID"];
                let rows: Vec<Vec<String>> = data_to_export.iter()
                    .map(|g| vec![
                        g.event_flag.to_string(),
                        g.name.clone(),
                        g.region.clone(),
                        format!("{:.1}", g.pos_x),
                        format!("{:.1}", g.pos_y),
                        format!("{:.1}", g.pos_z),
                        g.mapgenie_id.clone().unwrap_or_default(),
                    ])
                    .collect();
                to_csv(headers, &rows)
            }
            ExportFormat::Markdown => {
                let headers = &["Flag ID", "Name", "Region", "Pos X", "Pos Y", "Pos Z", "MapGenie ID"];
                let rows: Vec<Vec<String>> = data_to_export.iter()
                    .map(|g| vec![
                        g.event_flag.to_string(),
                        g.name.clone(),
                        g.region.clone(),
                        format!("{:.1}", g.pos_x),
                        format!("{:.1}", g.pos_y),
                        format!("{:.1}", g.pos_z),
                        g.mapgenie_id.clone().unwrap_or_default(),
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
