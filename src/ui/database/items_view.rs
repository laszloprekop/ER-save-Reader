//! Unified Items database view.
//!
//! Shows items from all EquipParam files with filtering by category.

use eframe::egui::{Ui, Color32, RichText};
use crate::db::unified_items::{UNIFIED_ITEMS, UnifiedItemCategory};
use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData, SortDirection};
use crate::ui::components::filter::{FilterBar, FilterBarState, fuzzy_match_default};
use crate::ui::components::export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
use crate::ui::components::detail_panel::{DetailPanelState, SelectedEntity, RelationshipSection, RelationshipItem, DetailPanelAction};
use crate::ui::tokens::spacing;
use crate::db::entity_relationships::get_item_relationships;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ItemCategoryFilter {
    #[default]
    All,
    Weapon,
    Armor,
    Accessory,
    Good,
}

impl ItemCategoryFilter {
    fn to_filter_value(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Weapon => "Weapon",
            Self::Armor => "Armor",
            Self::Accessory => "Accessory",
            Self::Good => "Good",
        }
    }

    fn from_filter_value(s: &str) -> Self {
        match s {
            "Weapon" => Self::Weapon,
            "Armor" => Self::Armor,
            "Accessory" => Self::Accessory,
            "Good" => Self::Good,
            _ => Self::All,
        }
    }

    fn matches(&self, category: UnifiedItemCategory) -> bool {
        match self {
            Self::All => true,
            Self::Weapon => category == UnifiedItemCategory::Weapon,
            Self::Armor => category == UnifiedItemCategory::Armor,
            Self::Accessory => category == UnifiedItemCategory::Accessory,
            Self::Good => category == UnifiedItemCategory::Good,
        }
    }
}

pub struct ItemsViewState {
    pub filter: ItemCategoryFilter,
    pub search: String,
    pub selected_key: Option<(UnifiedItemCategory, u32)>,
    /// Track which item we last opened the detail panel for
    pub last_detail_key: Option<(UnifiedItemCategory, u32)>,
    pub table_state: TableState,
    pub filter_state: FilterBarState,
    pub export_format: ExportFormat,
    pub export_filtered_only: bool,
}

impl Default for ItemsViewState {
    fn default() -> Self {
        Self {
            filter: ItemCategoryFilter::All,
            search: String::new(),
            selected_key: None,
            last_detail_key: None,
            table_state: TableState::new().with_sort("id", SortDirection::Ascending),
            filter_state: FilterBarState::new(),
            export_format: ExportFormat::Json,
            export_filtered_only: false,
        }
    }
}

#[derive(Serialize)]
struct ItemExportItem {
    id: u32,
    name: String,
    category: String,
    icon_id: u16,
    weight: f32,
    sell_value: i32,
    max_hold: u16,
}

pub fn items_view(ui: &mut Ui, state: &mut ItemsViewState, detail_panel: &mut DetailPanelState) {
    // Sync filter_state with state
    state.filter = ItemCategoryFilter::from_filter_value(&state.filter_state.category);
    state.search = state.filter_state.search.clone();

    // Filter bar
    FilterBar::new("items_filter", &mut state.filter_state)
        .category_strings("Category", &["Weapon", "Armor", "Accessory", "Good"])
        .search("Search items...")
        .show(ui);

    spacing::space_sm(ui);

    // Export toolbar
    let export_response = ExportToolbar::new("items_export", &mut state.export_format, &mut state.export_filtered_only)
        .has_filters(state.filter_state.has_active_filters())
        .show(ui);

    spacing::space_sm(ui);

    // Build item data with filtering and sorting
    let mut items: Vec<((UnifiedItemCategory, u32), &crate::db::unified_items::UnifiedItem)> = UNIFIED_ITEMS.iter()
        .filter(|((cat, _), item)| {
            // Category filter
            if !state.filter.matches(*cat) {
                return false;
            }

            // Search filter
            if !state.search.is_empty() {
                if !fuzzy_match_default(item.name, &state.search) {
                    return false;
                }
            }

            true
        })
        .map(|(key, item)| (*key, item))
        .collect();

    // Apply sorting
    if let Some(sort_col) = &state.table_state.sort_column {
        let asc = state.table_state.sort_direction == SortDirection::Ascending;
        match sort_col.as_str() {
            "id" => items.sort_by(|a, b| if asc { a.0.1.cmp(&b.0.1) } else { b.0.1.cmp(&a.0.1) }),
            "name" => items.sort_by(|a, b| if asc { a.1.name.cmp(b.1.name) } else { b.1.name.cmp(a.1.name) }),
            "category" => items.sort_by(|a, b| {
                let ca = a.1.category.as_str();
                let cb = b.1.category.as_str();
                if asc { ca.cmp(cb) } else { cb.cmp(ca) }
            }),
            "weight" => items.sort_by(|a, b| {
                let wa = a.1.weight;
                let wb = b.1.weight;
                if asc { wa.partial_cmp(&wb).unwrap_or(std::cmp::Ordering::Equal) }
                else { wb.partial_cmp(&wa).unwrap_or(std::cmp::Ordering::Equal) }
            }),
            "value" => items.sort_by(|a, b| if asc { a.1.sell_value.cmp(&b.1.sell_value) } else { b.1.sell_value.cmp(&a.1.sell_value) }),
            "max" => items.sort_by(|a, b| if asc { a.1.max_hold.cmp(&b.1.max_hold) } else { b.1.max_hold.cmp(&a.1.max_hold) }),
            _ => {}
        }
    }

    // Auto-open detail panel if selection was set programmatically (from navigation)
    if let Some(key) = state.selected_key {
        if state.last_detail_key != Some(key) {
            // Find the item in the filtered list and open its detail panel
            if let Some((_, item)) = items.iter().find(|(k, _)| *k == key) {
                // Build relationship sections
                let relationships = get_item_relationships(item.id);
                let mut sections = Vec::new();

                // Group by relationship type - Sold By
                let sold_by: Vec<_> = relationships.iter()
                    .filter(|r| matches!(r.rel_type, crate::db::entity_relationships::RelationType::SoldBy))
                    .collect();

                if !sold_by.is_empty() {
                    let rel_items: Vec<RelationshipItem> = sold_by.iter()
                        .map(|r| {
                            let mut rel_item = RelationshipItem::new(
                                r.label.to_string(),
                                DetailPanelAction::None,
                            );
                            if let Some(secondary) = &r.secondary {
                                rel_item = rel_item.with_secondary(secondary.clone());
                            }
                            rel_item
                        })
                        .collect();
                    sections.push(RelationshipSection::new("Sold By").with_items(rel_items));
                }

                // Group by relationship type - Dropped By
                let dropped_by: Vec<_> = relationships.iter()
                    .filter(|r| matches!(r.rel_type, crate::db::entity_relationships::RelationType::DroppedBy))
                    .collect();

                if !dropped_by.is_empty() {
                    let rel_items: Vec<RelationshipItem> = dropped_by.iter()
                        .map(|r| {
                            RelationshipItem::new(
                                r.label.to_string(),
                                DetailPanelAction::NavigateToBoss { defeat_flag: r.target_id, name: r.label.to_string() },
                            )
                        })
                        .collect();
                    sections.push(RelationshipSection::new("Dropped By").with_items(rel_items));
                }

                // Group by relationship type - Found At
                let found_at: Vec<_> = relationships.iter()
                    .filter(|r| matches!(r.rel_type, crate::db::entity_relationships::RelationType::FoundAt))
                    .collect();

                if !found_at.is_empty() {
                    let rel_items: Vec<RelationshipItem> = found_at.iter()
                        .take(10)
                        .map(|r| {
                            let mut rel_item = RelationshipItem::new(
                                r.label.to_string(),
                                DetailPanelAction::NavigateToPickup { flag_id: r.target_id, name: r.label.to_string() },
                            );
                            if let Some(secondary) = &r.secondary {
                                rel_item = rel_item.with_secondary(secondary.clone());
                            }
                            rel_item
                        })
                        .collect();
                    let title = if found_at.len() > 10 {
                        format!("Found At ({} locations, showing 10)", found_at.len())
                    } else {
                        "Found At".to_string()
                    };
                    sections.push(RelationshipSection::new(title).with_items(rel_items));
                }

                detail_panel.select_with_relationships(
                    SelectedEntity::Item {
                        category: key.0.as_str().to_string(),
                        id: key.1,
                        name: item.name.to_string(),
                    },
                    sections,
                );
                state.last_detail_key = Some(key);
            }
        }
    }

    // Summary
    let total_count = UNIFIED_ITEMS.len();
    let filtered_count = items.len();
    if filtered_count < total_count {
        ui.label(RichText::new(format!("Items: {} (showing {}/{})", total_count, filtered_count, total_count)).strong());
    } else {
        ui.label(RichText::new(format!("Items: {}", total_count)).strong());
    }

    spacing::space_sm(ui);

    // Build row data
    let rows: Vec<RowData> = items.iter().map(|(key, item)| {
        let is_selected = state.selected_key == Some(*key);

        let mut row = RowData::new(vec![
            item.id.to_string(),
            item.name.to_string(),
            item.category.as_str().to_string(),
            format!("{:.1}", item.weight),
            item.sell_value.to_string(),
            item.max_hold.to_string(),
        ]);

        if is_selected {
            row = row.with_color(Color32::YELLOW);
        }

        row
    }).collect();

    // Show table with auto-width columns
    let table_response = UnifiedTable::new("items_table", &mut state.table_state)
        .columns(vec![
            Column::new("id", "ID").sortable(true).monospace(true),
            Column::new("name", "Name").sortable(true),
            Column::new("category", "Category").sortable(true),
            Column::new("weight", "Weight").sortable(true).right(),
            Column::new("value", "Value").sortable(true).right(),
            Column::new("max", "Max").sortable(true).right(),
        ])
        .rows(rows)
        .zebra_stripe(true)
        .selectable(true)
        .show(ui);

    // Handle copy
    if let Some(text) = table_response.clipboard_text {
        ui.output_mut(|o| o.copied_text = text);
    }

    // Handle single click - open detail panel with relationships
    if let Some(row_idx) = table_response.clicked_row {
        if let Some(((category, id), item)) = items.get(row_idx) {
            // Build relationship sections
            let relationships = get_item_relationships(item.id);
            let mut sections = Vec::new();

            // Group by relationship type - Sold By
            let sold_by: Vec<_> = relationships.iter()
                .filter(|r| matches!(r.rel_type, crate::db::entity_relationships::RelationType::SoldBy))
                .collect();

            if !sold_by.is_empty() {
                let rel_items: Vec<RelationshipItem> = sold_by.iter()
                    .map(|r| {
                        let mut rel_item = RelationshipItem::new(
                            r.label.to_string(),
                            DetailPanelAction::None, // TODO: Navigate to merchant
                        );
                        if let Some(secondary) = &r.secondary {
                            rel_item = rel_item.with_secondary(secondary.clone());
                        }
                        rel_item
                    })
                    .collect();
                sections.push(RelationshipSection::new("Sold By").with_items(rel_items));
            }

            // Group by relationship type - Dropped By (show first, most important)
            let dropped_by: Vec<_> = relationships.iter()
                .filter(|r| matches!(r.rel_type, crate::db::entity_relationships::RelationType::DroppedBy))
                .collect();

            if !dropped_by.is_empty() {
                let rel_items: Vec<RelationshipItem> = dropped_by.iter()
                    .map(|r| {
                        RelationshipItem::new(
                            r.label.to_string(),
                            DetailPanelAction::NavigateToBoss { defeat_flag: r.target_id, name: r.label.to_string() },
                        )
                    })
                    .collect();
                sections.push(RelationshipSection::new("Dropped By").with_items(rel_items));
            }

            // Group by relationship type - Found At
            let found_at: Vec<_> = relationships.iter()
                .filter(|r| matches!(r.rel_type, crate::db::entity_relationships::RelationType::FoundAt))
                .collect();

            if !found_at.is_empty() {
                let rel_items: Vec<RelationshipItem> = found_at.iter()
                    .take(10) // Limit to first 10 to avoid overwhelming the panel
                    .map(|r| {
                        let mut rel_item = RelationshipItem::new(
                            r.label.to_string(),
                            DetailPanelAction::NavigateToPickup { flag_id: r.target_id, name: r.label.to_string() },
                        );
                        if let Some(secondary) = &r.secondary {
                            rel_item = rel_item.with_secondary(secondary.clone());
                        }
                        rel_item
                    })
                    .collect();
                let title = if found_at.len() > 10 {
                    format!("Found At ({} locations, showing 10)", found_at.len())
                } else {
                    "Found At".to_string()
                };
                sections.push(RelationshipSection::new(title).with_items(rel_items));
            }

            detail_panel.select_with_relationships(
                SelectedEntity::Item {
                    category: category.as_str().to_string(),
                    id: *id,
                    name: item.name.to_string(),
                },
                sections,
            );
            state.last_detail_key = Some((*category, *id));
        }
    }

    // Update selected based on table selection
    if state.table_state.selection_count() == 1 {
        if let Some(&idx) = state.table_state.selected_rows.iter().next() {
            if let Some((key, _)) = items.get(idx) {
                state.selected_key = Some(*key);
            }
        }
    }

    // Handle export
    if export_response.export_clicked || export_response.copy_clicked {
        let data_to_export: Vec<_> = items.iter()
            .map(|(_, item)| ItemExportItem {
                id: item.id,
                name: item.name.to_string(),
                category: item.category.as_str().to_string(),
                icon_id: item.icon_id,
                weight: item.weight,
                sell_value: item.sell_value,
                max_hold: item.max_hold,
            })
            .collect();

        let content = match state.export_format {
            ExportFormat::Json => {
                let export = PageExport::new(
                    PageExportMetadata::new("Unified Items")
                        .with_counts(total_count, filtered_count),
                    &data_to_export,
                );
                to_json(&export).unwrap_or_else(|_| String::new())
            }
            ExportFormat::Csv => {
                let headers = &["ID", "Name", "Category", "Icon", "Weight", "Value", "Max"];
                let rows: Vec<Vec<String>> = data_to_export.iter()
                    .map(|i| vec![
                        i.id.to_string(),
                        i.name.clone(),
                        i.category.clone(),
                        i.icon_id.to_string(),
                        format!("{:.1}", i.weight),
                        i.sell_value.to_string(),
                        i.max_hold.to_string(),
                    ])
                    .collect();
                to_csv(headers, &rows)
            }
            ExportFormat::Markdown => {
                let headers = &["ID", "Name", "Category", "Icon", "Weight", "Value", "Max"];
                let rows: Vec<Vec<String>> = data_to_export.iter()
                    .map(|i| vec![
                        i.id.to_string(),
                        i.name.clone(),
                        i.category.clone(),
                        i.icon_id.to_string(),
                        format!("{:.1}", i.weight),
                        i.sell_value.to_string(),
                        i.max_hold.to_string(),
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
