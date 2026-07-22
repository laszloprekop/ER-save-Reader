//! Bosses database view.
//!
//! Shows bosses with defeat flags and status tracking.

use eframe::egui::{Ui, Color32, RichText};
use crate::db::bosses_data::{BOSSES_DATA, BOSS_REGIONS, BossType};
use crate::db::entity_relationships::{get_boss_relationships, RelationType};
use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData, SortDirection};
use crate::ui::components::filter::{FilterBar, FilterBarState, fuzzy_match_default};
use crate::ui::components::export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
use crate::ui::components::detail_panel::{DetailPanelState, SelectedEntity, RelationshipSection, RelationshipItem, DetailPanelAction, mapgenie_section, section_from_relationships};
use crate::ui::tokens::spacing;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum BossTypeFilter {
    #[default]
    All,
    Demigod,
    GreatBoss,
    Boss,
    Miniboss,
}

impl BossTypeFilter {
    fn matches(&self, boss_type: BossType) -> bool {
        match self {
            Self::All => true,
            Self::Demigod => boss_type == BossType::Demigod,
            Self::GreatBoss => boss_type == BossType::GreatBoss,
            Self::Boss => boss_type == BossType::Boss,
            Self::Miniboss => boss_type == BossType::Miniboss,
        }
    }
}

pub struct BossesViewState {
    pub region_filter: String,
    pub type_filter: BossTypeFilter,
    pub search: String,
    pub selected_flag: Option<u32>,
    /// Track which boss we last opened the detail panel for
    pub last_detail_flag: Option<u32>,
    pub table_state: TableState,
    pub filter_state: FilterBarState,
    pub export_format: ExportFormat,
    pub export_filtered_only: bool,
}

impl Default for BossesViewState {
    fn default() -> Self {
        Self {
            region_filter: "All".to_string(),
            type_filter: BossTypeFilter::All,
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
struct BossExportItem {
    defeat_flag: u32,
    name: String,
    region: String,
    boss_type: String,
    defeated: Option<bool>,
    mapgenie_id: Option<String>,
}

/// Check if a boss is defeated using event flags.
///
/// CUT OVER 2026-07-20 (ADR-0006, migration step 4). Previously this called
/// `pickup_flags::is_flag_set`, which read absolute offsets from the frozen
/// legacy store and returned a bare `bool` — so a save whose flag families had
/// drifted away from those offsets reported every boss as NOT defeated, which
/// is exactly how `batch-validate` came to print 0/110 on a mid-game character.
///
/// Both families are resolved per save from the flag region's own origin. The
/// two ranges are distinct families, not two encodings of one:
///
///   10-digit  open-world tile bosses  -> tile-open-world
///    8-digit  legacy-map bosses       -> legacy-dungeon (alloc-slot layout)
///
/// Routing on the digit count is safe HERE — unlike the pickup case, where a
/// 10-digit id was ambiguous between two families 500 bytes apart — because a
/// boss defeat is always an event flag, never a pickup, in whichever map hosts
/// it. Both reads reject ids outside their own family rather than guessing.
///
/// `None` is UNKNOWN (unresolved origin, or a map with no allocation) and is
/// rendered as "-", never as "not defeated".
fn is_boss_defeated(defeat_flag: u32, event_flags: Option<&[u8]>) -> Option<bool> {
    let flags = event_flags?;
    if defeat_flag >= 1_000_000_000 {
        wasm_event_flags::is_tile_world_flag_set(flags, defeat_flag)
    } else {
        wasm_event_flags::is_dungeon_flag_set(flags, defeat_flag)
    }
}

/// Build detail panel sections for a boss, including relationships.
fn build_boss_sections(boss: &crate::db::bosses_data::BossData, flag: u32) -> Vec<RelationshipSection> {
    let mut sections = Vec::new();

    if let Some(id) = boss.mapgenie_id {
        sections.push(mapgenie_section(id));
    }

    // Boss info
    let mut info_items = vec![
        RelationshipItem::new(
            format!("Type: {}", boss.boss_type.as_str()),
            DetailPanelAction::None,
        ),
        RelationshipItem::new(
            format!("Region: {}", boss.region),
            DetailPanelAction::None,
        ),
    ];
    if boss.runes > 0 {
        info_items.push(RelationshipItem::new(
            format!("Runes: {}", boss.runes),
            DetailPanelAction::None,
        ));
    }
    sections.push(RelationshipSection::new("Info").with_items(info_items));

    // Relationship-based sections (drops + nearby graces)
    let rels = get_boss_relationships(flag);

    if let Some(s) = section_from_relationships("Drops", &rels, RelationType::Drops, |r| {
        DetailPanelAction::NavigateToItem {
            category: r.secondary.clone().unwrap_or_default(),
            id: r.target_id,
            name: r.label.to_string(),
        }
    }) {
        sections.push(s);
    }

    if let Some(s) = section_from_relationships("Nearby Graces", &rels, RelationType::NearbyGrace, |r| {
        DetailPanelAction::NavigateToGrace {
            event_flag: r.target_id,
            name: r.label.to_string(),
        }
    }) {
        sections.push(s);
    }

    sections
}

pub fn bosses_view(ui: &mut Ui, state: &mut BossesViewState, event_flags: Option<&[u8]>, detail_panel: &mut DetailPanelState) {
    // Sync filter_state with state
    state.region_filter = state.filter_state.category.clone();
    state.search = state.filter_state.search.clone();

    // Build region options
    let region_options: Vec<_> = BOSS_REGIONS.to_vec();

    // Filter bar
    FilterBar::new("bosses_filter", &mut state.filter_state)
        .category_strings("Region", &region_options)
        .search("Search bosses...")
        .show(ui);

    spacing::space_sm(ui);

    // Type filter buttons
    ui.horizontal(|ui| {
        ui.label("Type:");
        ui.selectable_value(&mut state.type_filter, BossTypeFilter::All, "All");
        ui.selectable_value(&mut state.type_filter, BossTypeFilter::Demigod, "Demigod");
        ui.selectable_value(&mut state.type_filter, BossTypeFilter::GreatBoss, "Great Boss");
        ui.selectable_value(&mut state.type_filter, BossTypeFilter::Boss, "Boss");
        ui.selectable_value(&mut state.type_filter, BossTypeFilter::Miniboss, "Miniboss");
    });

    spacing::space_sm(ui);

    // Export toolbar
    let export_response = ExportToolbar::new("bosses_export", &mut state.export_format, &mut state.export_filtered_only)
        .has_filters(state.filter_state.has_active_filters() || state.type_filter != BossTypeFilter::All)
        .show(ui);

    spacing::space_sm(ui);

    // Build boss data with filtering and sorting
    let mut bosses: Vec<(u32, &crate::db::bosses_data::BossData, Option<bool>)> = BOSSES_DATA.iter()
        .filter_map(|(flag, boss)| {
            // Region filter
            if state.region_filter != "All" && boss.region != state.region_filter {
                return None;
            }

            // Type filter
            if !state.type_filter.matches(boss.boss_type) {
                return None;
            }

            // Search filter
            if !state.search.is_empty()
                && !fuzzy_match_default(boss.name, &state.search)
                    && !fuzzy_match_default(boss.region, &state.search) {
                    return None;
                }

            let defeated = is_boss_defeated(*flag, event_flags);
            Some((*flag, boss, defeated))
        })
        .collect();

    // Apply sorting
    if let Some(sort_col) = &state.table_state.sort_column {
        let asc = state.table_state.sort_direction == SortDirection::Ascending;
        match sort_col.as_str() {
            "flag" => bosses.sort_by(|a, b| if asc { a.0.cmp(&b.0) } else { b.0.cmp(&a.0) }),
            "name" => bosses.sort_by(|a, b| if asc { a.1.name.cmp(b.1.name) } else { b.1.name.cmp(a.1.name) }),
            "region" => bosses.sort_by(|a, b| if asc { a.1.region.cmp(b.1.region) } else { b.1.region.cmp(a.1.region) }),
            "type" => bosses.sort_by(|a, b| {
                let ta = a.1.boss_type.as_str();
                let tb = b.1.boss_type.as_str();
                if asc { ta.cmp(tb) } else { tb.cmp(ta) }
            }),
            "status" => bosses.sort_by(|a, b| {
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
            if let Some((_, boss, _)) = bosses.iter().find(|(f, _, _)| *f == flag) {
                detail_panel.select_with_relationships(
                    SelectedEntity::Boss {
                        defeat_flag: flag,
                        name: boss.name.to_string(),
                    },
                    build_boss_sections(boss, flag),
                );
                state.last_detail_flag = Some(flag);
            }
        }
    }

    // Summary
    let total_count = BOSSES_DATA.len();
    let filtered_count = bosses.len();
    if filtered_count < total_count {
        ui.label(RichText::new(format!("Bosses: {} (showing {}/{})", total_count, filtered_count, total_count)).strong());
    } else {
        ui.label(RichText::new(format!("Bosses: {}", total_count)).strong());
    }

    spacing::space_sm(ui);

    // Build row data
    let rows: Vec<RowData> = bosses.iter().map(|(flag, boss, defeated)| {
        let is_selected = state.selected_flag == Some(*flag);

        let status_str = match defeated {
            Some(true) => "✓",
            Some(false) => "○",
            None => "-",
        };

        let mut row = RowData::new(vec![
            boss.name.to_string(),
            flag.to_string(),
            boss.region.to_string(),
            boss.boss_type.as_str().to_string(),
            status_str.to_string(),
        ]);

        if is_selected {
            row = row.with_color(Color32::YELLOW);
        } else if *defeated == Some(true) {
            row = row.with_color(Color32::from_rgb(144, 238, 144)); // Light green
        } else if boss.boss_type == BossType::Demigod {
            row = row.with_color(Color32::from_rgb(255, 215, 0)); // Gold
        }

        row
    }).collect();

    // Show table with auto-width columns
    let table_response = UnifiedTable::new("bosses_table", &mut state.table_state)
        .columns(vec![
            Column::new("name", "Name").sortable(true),
            Column::new("flag", "Defeat Flag").sortable(true).monospace(true),
            Column::new("region", "Region").sortable(true),
            Column::new("type", "Type").sortable(true),
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

    // Handle single click - open detail panel
    if let Some(row_idx) = table_response.clicked_row {
        if let Some((flag, boss, _)) = bosses.get(row_idx) {
            detail_panel.select_with_relationships(
                SelectedEntity::Boss {
                    defeat_flag: *flag,
                    name: boss.name.to_string(),
                },
                build_boss_sections(boss, *flag),
            );
            state.last_detail_flag = Some(*flag);
        }
    }

    // Update selected based on table selection
    if state.table_state.selection_count() == 1 {
        if let Some(&idx) = state.table_state.selected_rows.iter().next() {
            if let Some((flag, _, _)) = bosses.get(idx) {
                state.selected_flag = Some(*flag);
            }
        }
    }

    // Handle export
    if export_response.export_clicked || export_response.copy_clicked {
        let data_to_export: Vec<_> = bosses.iter()
            .map(|(flag, boss, defeated)| BossExportItem {
                defeat_flag: *flag,
                name: boss.name.to_string(),
                region: boss.region.to_string(),
                boss_type: boss.boss_type.as_str().to_string(),
                defeated: *defeated,
                mapgenie_id: boss.mapgenie_id.map(|s| s.to_string()),
            })
            .collect();

        let content = match state.export_format {
            ExportFormat::Json => {
                let export = PageExport::new(
                    PageExportMetadata::new("Bosses")
                        .with_counts(total_count, filtered_count),
                    &data_to_export,
                );
                to_json(&export).unwrap_or_else(|_| String::new())
            }
            ExportFormat::Csv | ExportFormat::Markdown => {
                let headers = &["Defeat Flag", "Name", "Region", "Type", "Defeated", "MapGenie ID"];
                let rows: Vec<Vec<String>> = data_to_export.iter()
                    .map(|b| vec![
                        b.defeat_flag.to_string(),
                        b.name.clone(),
                        b.region.clone(),
                        b.boss_type.clone(),
                        b.defeated.map(|d| if d { "Yes" } else { "No" }).unwrap_or("-").to_string(),
                        b.mapgenie_id.clone().unwrap_or_default(),
                    ])
                    .collect();
                if state.export_format == ExportFormat::Csv { to_csv(headers, &rows) } else { to_markdown(headers, &rows) }
            }
        };

        if export_response.copy_clicked {
            ui.output_mut(|o| o.copied_text = content);
        }
    }
}
