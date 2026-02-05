//! Event Chains View for quest progression reference.
//!
//! Shows major quest lines and their steps as reference data (no character data required).

use eframe::egui::{Ui, Color32, RichText};
use serde::Serialize;
use crate::db::quest_chains::{QUEST_CHAINS, QuestCategory, get_all_npcs};
use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData, SortDirection};
use crate::ui::components::filter::{FilterBar, FilterBarState, FilterOption, fuzzy_match_default};
use crate::ui::components::export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
use crate::ui::components::detail_panel::{DetailPanelState, SelectedEntity, RelationshipSection, RelationshipItem, DetailPanelAction};
use crate::ui::tokens::spacing;

pub struct EventChainsViewState {
    pub category_filter: String,
    pub npc_filter: String,
    pub search: String,
    pub selected_chain: Option<u32>,
    /// Track which chain we last opened the detail panel for
    pub last_detail_chain: Option<u32>,
    pub table_state: TableState,
    pub filter_state: FilterBarState,
    pub export_format: ExportFormat,
    pub export_filtered_only: bool,
}

impl Default for EventChainsViewState {
    fn default() -> Self {
        Self {
            category_filter: "All".to_string(),
            npc_filter: "All".to_string(),
            search: String::new(),
            selected_chain: None,
            last_detail_chain: None,
            table_state: TableState::new().with_sort("name", SortDirection::Ascending),
            filter_state: FilterBarState::new(),
            export_format: ExportFormat::Json,
            export_filtered_only: false,
        }
    }
}

#[derive(Serialize)]
struct QuestChainExport {
    id: u32,
    name: String,
    category: String,
    npc: Option<String>,
    steps_count: usize,
    steps: Vec<QuestStepExport>,
}

#[derive(Serialize)]
struct QuestStepExport {
    step: usize,
    name: String,
    flag_id: u32,
    description: String,
    verified: bool,
}

pub fn event_chains_view(ui: &mut Ui, state: &mut EventChainsViewState, detail_panel: &mut DetailPanelState) {
    // Sync filter state
    state.search = state.filter_state.search.clone();

    // Build NPC filter options
    let npcs = get_all_npcs();
    let npc_options: Vec<FilterOption> = std::iter::once(FilterOption::all())
        .chain(npcs.iter().map(|n| FilterOption::from_str(*n)))
        .collect();

    // Filter bar with NPC dropdown and search
    FilterBar::new("event_chains_filter", &mut state.filter_state)
        .category("NPC", npc_options)
        .search("Search quests...")
        .show(ui);

    // Sync NPC filter from filter state
    state.npc_filter = state.filter_state.category.clone();

    spacing::space_sm(ui);

    // Category filter chips
    ui.horizontal(|ui| {
        ui.label(RichText::new("Category:").color(Color32::LIGHT_GRAY));
        if ui.selectable_label(state.category_filter == "All", "All").clicked() {
            state.category_filter = "All".to_string();
        }
        for cat in QuestCategory::all_categories() {
            let cat_name = cat.display_name();
            if ui.selectable_label(state.category_filter == cat_name, cat_name).clicked() {
                state.category_filter = cat_name.to_string();
            }
        }
    });

    spacing::space_sm(ui);

    // Export toolbar
    let has_filters = state.filter_state.has_active_filters()
        || state.category_filter != "All";
    let export_response = ExportToolbar::new("event_chains_export", &mut state.export_format, &mut state.export_filtered_only)
        .has_filters(has_filters)
        .show(ui);

    spacing::space_sm(ui);

    // Build quest chain data with filtering
    let chains: Vec<(u32, &crate::db::quest_chains::QuestChain)> = QUEST_CHAINS.iter()
        .filter_map(|chain| {
            // Category filter
            if state.category_filter != "All" && chain.category.display_name() != state.category_filter {
                return None;
            }

            // NPC filter
            if state.npc_filter != "All" {
                if chain.npc_name != Some(state.npc_filter.as_str()) {
                    return None;
                }
            }

            // Search filter
            if !state.search.is_empty() {
                let matches = fuzzy_match_default(chain.name, &state.search)
                    || chain.npc_name.map(|n| fuzzy_match_default(n, &state.search)).unwrap_or(false)
                    || chain.steps.iter().any(|s| fuzzy_match_default(s.name, &state.search));
                if !matches {
                    return None;
                }
            }

            Some((chain.id, chain))
        })
        .collect();

    // Auto-open detail panel if selection was set programmatically (from navigation)
    if let Some(chain_id) = state.selected_chain {
        if state.last_detail_chain != Some(chain_id) {
            // Find the chain and open its detail panel
            if let Some(chain) = QUEST_CHAINS.iter().find(|c| c.id == chain_id) {
                let sections = build_detail_sections(chain);

                detail_panel.select_with_relationships(
                    SelectedEntity::QuestChain {
                        id: chain_id,
                        name: chain.name.to_string(),
                        category: chain.category.display_name().to_string(),
                    },
                    sections,
                );
                state.last_detail_chain = Some(chain_id);
            }
        }
    }

    // Summary
    let total_chains = QUEST_CHAINS.len();
    let filtered_count = chains.len();

    if filtered_count < total_chains {
        ui.label(RichText::new(format!(
            "Quest Chains: {} (showing {}/{})",
            total_chains, filtered_count, total_chains
        )).strong());
    } else {
        ui.label(RichText::new(format!(
            "Quest Chains: {}",
            total_chains
        )).strong());
    }

    spacing::space_sm(ui);

    // Build row data
    let rows: Vec<RowData> = chains.iter().map(|(id, chain)| {
        let is_selected = state.selected_chain == Some(*id);

        // Show verification status: all verified, some verified, none verified
        let verified_count = chain.steps.iter().filter(|s| s.verified).count();
        let verified_indicator = if verified_count == chain.steps.len() {
            "✓" // All verified
        } else if verified_count > 0 {
            "◐" // Partially verified
        } else {
            "○" // None verified
        };

        let mut row = RowData::new(vec![
            chain.name.to_string(),
            chain.category.display_name().to_string(),
            chain.npc_name.unwrap_or("-").to_string(),
            chain.steps.len().to_string(),
            verified_indicator.to_string(),
        ]);

        if is_selected {
            row = row.with_color(Color32::YELLOW);
        }

        row
    }).collect();

    // Show table
    let table_response = UnifiedTable::new("event_chains_table", &mut state.table_state)
        .columns(vec![
            Column::new("name", "Quest Chain").sortable(true),
            Column::new("category", "Category").sortable(true),
            Column::new("npc", "NPC").sortable(true),
            Column::new("steps", "Steps").center().sortable(true),
            Column::new("verified", "V").center(),
        ])
        .rows(rows)
        .zebra_stripe(true)
        .selectable(true)
        .show(ui);

    // Handle single click - open detail panel with quest steps
    if let Some(row_idx) = table_response.clicked_row {
        if let Some((chain_id, chain)) = chains.get(row_idx) {
            let sections = build_detail_sections(chain);

            detail_panel.select_with_relationships(
                SelectedEntity::QuestChain {
                    id: *chain_id,
                    name: chain.name.to_string(),
                    category: chain.category.display_name().to_string(),
                },
                sections,
            );
            state.last_detail_chain = Some(*chain_id);
        }
    }

    // Handle selection (update state based on table selection)
    if state.table_state.selection_count() == 1 {
        if let Some(&idx) = state.table_state.selected_rows.iter().next() {
            if let Some((id, _)) = chains.get(idx) {
                state.selected_chain = Some(*id);
            }
        }
    }

    // Handle copy
    if let Some(text) = table_response.clipboard_text {
        ui.output_mut(|o| o.copied_text = text);
    }

    // Handle export
    if export_response.export_clicked || export_response.copy_clicked {
        let data_to_export: Vec<QuestChainExport> = chains.iter()
            .map(|(_, chain)| {
                let steps: Vec<QuestStepExport> = chain.steps.iter().enumerate()
                    .map(|(i, step)| {
                        QuestStepExport {
                            step: i + 1,
                            name: step.name.to_string(),
                            flag_id: step.flag_id,
                            description: step.description.to_string(),
                            verified: step.verified,
                        }
                    })
                    .collect();

                QuestChainExport {
                    id: chain.id,
                    name: chain.name.to_string(),
                    category: chain.category.display_name().to_string(),
                    npc: chain.npc_name.map(|s| s.to_string()),
                    steps_count: chain.steps.len(),
                    steps,
                }
            })
            .collect();

        let content = match state.export_format {
            ExportFormat::Json => {
                let export = PageExport::new(
                    PageExportMetadata::new("Quest Chains")
                        .with_counts(total_chains, filtered_count),
                    &data_to_export,
                );
                to_json(&export).unwrap_or_else(|_| String::new())
            }
            ExportFormat::Csv => {
                let headers = &["ID", "Name", "Category", "NPC", "Steps"];
                let rows: Vec<Vec<String>> = data_to_export.iter()
                    .map(|c| vec![
                        c.id.to_string(),
                        c.name.clone(),
                        c.category.clone(),
                        c.npc.clone().unwrap_or_default(),
                        c.steps_count.to_string(),
                    ])
                    .collect();
                to_csv(headers, &rows)
            }
            ExportFormat::Markdown => {
                let headers = &["ID", "Name", "Category", "NPC", "Steps"];
                let rows: Vec<Vec<String>> = data_to_export.iter()
                    .map(|c| vec![
                        c.id.to_string(),
                        c.name.clone(),
                        c.category.clone(),
                        c.npc.clone().unwrap_or_default(),
                        c.steps_count.to_string(),
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

/// Build detail panel sections for a quest chain
fn build_detail_sections(chain: &crate::db::quest_chains::QuestChain) -> Vec<RelationshipSection> {
    let mut sections = Vec::new();

    // Build steps section
    let step_items: Vec<RelationshipItem> = chain.steps.iter().enumerate()
        .map(|(i, step)| {
            let verified_badge = if step.verified { " [V]" } else { "" };

            RelationshipItem::new(
                format!("{}. {}{}", i + 1, step.name, verified_badge),
                DetailPanelAction::None,
            ).with_secondary(format!("Flag: {} - {}", step.flag_id, step.description))
        })
        .collect();

    sections.push(RelationshipSection::new("Quest Steps").with_items(step_items));

    // Add NPC info if available
    if let Some(npc) = chain.npc_name {
        sections.push(
            RelationshipSection::new("NPC").with_items(vec![
                RelationshipItem::new(npc.to_string(), DetailPanelAction::None)
            ])
        );
    }

    sections
}
