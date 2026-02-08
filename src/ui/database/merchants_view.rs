//! Merchants database view.
//!
//! Shows shop inventories from ShopLineupParam with filtering by merchant.

use eframe::egui::{Ui, Color32, RichText};
use crate::db::merchants_data::{MERCHANT_ITEMS, MERCHANT_NAMES};
use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData, SortDirection};
use crate::ui::components::filter::{FilterBar, FilterBarState, fuzzy_match_default};
use crate::ui::components::export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
use crate::ui::components::detail_panel::{DetailPanelState, SelectedEntity, DetailPanelAction, RelationshipSection, RelationshipItem};
use crate::ui::tokens::spacing;
use serde::Serialize;

pub struct MerchantsViewState {
    pub merchant_filter: String,
    pub item_type_filter: String,
    pub search: String,
    pub selected_shop_id: Option<u32>,
    /// Track which shop item we last opened the detail panel for
    pub last_detail_shop_id: Option<u32>,
    pub table_state: TableState,
    pub filter_state: FilterBarState,
    pub export_format: ExportFormat,
    pub export_filtered_only: bool,
}

impl Default for MerchantsViewState {
    fn default() -> Self {
        Self {
            merchant_filter: "All".to_string(),
            item_type_filter: "All".to_string(),
            search: String::new(),
            selected_shop_id: None,
            last_detail_shop_id: None,
            table_state: TableState::new().with_sort("shop_id", SortDirection::Ascending),
            filter_state: FilterBarState::new(),
            export_format: ExportFormat::Json,
            export_filtered_only: false,
        }
    }
}

#[derive(Serialize)]
struct MerchantExportItem {
    shop_id: u32,
    merchant: String,
    item_name: String,
    item_id: u32,
    price: u32,
    quantity: i16,
    item_type: String,
    stock_flag: u32,
    release_flag: u32,
}

/// Build detail panel sections for a merchant shop entry.
fn build_merchant_sections(item: &crate::db::merchants_data::MerchantItem) -> Vec<RelationshipSection> {
    vec![
        RelationshipSection::new("Item Sold").with_items(vec![
            RelationshipItem::new(
                item.item_name.to_string(),
                DetailPanelAction::NavigateToItem {
                    category: item.equip_type.as_str().to_string(),
                    id: item.item_id,
                    name: item.item_name.to_string(),
                },
            ).with_secondary(format!("{} runes", item.price))
        ])
    ]
}

pub fn merchants_view(ui: &mut Ui, state: &mut MerchantsViewState, detail_panel: &mut DetailPanelState) {
    // Sync filter_state with state
    state.merchant_filter = state.filter_state.category.clone();
    state.search = state.filter_state.search.clone();

    // Build merchant options
    let merchant_options: Vec<_> = MERCHANT_NAMES.iter().map(|s| *s).collect();

    // Filter bar
    FilterBar::new("merchants_filter", &mut state.filter_state)
        .category_strings("Merchant", &merchant_options)
        .search("Search items...")
        .show(ui);

    spacing::space_sm(ui);

    // Export toolbar
    let export_response = ExportToolbar::new("merchants_export", &mut state.export_format, &mut state.export_filtered_only)
        .has_filters(state.filter_state.has_active_filters())
        .show(ui);

    spacing::space_sm(ui);

    // Build merchant data with filtering and sorting
    let mut items: Vec<(u32, &crate::db::merchants_data::MerchantItem)> = MERCHANT_ITEMS.iter()
        .filter_map(|(shop_id, item)| {
            // Merchant filter
            if state.merchant_filter != "All" && item.merchant_name != state.merchant_filter {
                return None;
            }

            // Search filter
            if !state.search.is_empty() {
                if !fuzzy_match_default(item.item_name, &state.search)
                    && !fuzzy_match_default(item.merchant_name, &state.search) {
                    return None;
                }
            }

            Some((*shop_id, item))
        })
        .collect();

    // Apply sorting
    if let Some(sort_col) = &state.table_state.sort_column {
        let asc = state.table_state.sort_direction == SortDirection::Ascending;
        match sort_col.as_str() {
            "shop_id" => items.sort_by(|a, b| if asc { a.0.cmp(&b.0) } else { b.0.cmp(&a.0) }),
            "merchant" => items.sort_by(|a, b| if asc { a.1.merchant_name.cmp(b.1.merchant_name) } else { b.1.merchant_name.cmp(a.1.merchant_name) }),
            "item" => items.sort_by(|a, b| if asc { a.1.item_name.cmp(b.1.item_name) } else { b.1.item_name.cmp(a.1.item_name) }),
            "price" => items.sort_by(|a, b| if asc { a.1.price.cmp(&b.1.price) } else { b.1.price.cmp(&a.1.price) }),
            "qty" => items.sort_by(|a, b| if asc { a.1.quantity.cmp(&b.1.quantity) } else { b.1.quantity.cmp(&a.1.quantity) }),
            "type" => items.sort_by(|a, b| {
                let ta = a.1.equip_type.as_str();
                let tb = b.1.equip_type.as_str();
                if asc { ta.cmp(tb) } else { tb.cmp(ta) }
            }),
            _ => {}
        }
    }

    // Auto-open detail panel if selection was set programmatically (from navigation)
    if let Some(shop_id) = state.selected_shop_id {
        if state.last_detail_shop_id != Some(shop_id) {
            if let Some((_, item)) = items.iter().find(|(id, _)| *id == shop_id) {
                detail_panel.select_with_relationships(
                    SelectedEntity::Merchant {
                        shop_id,
                        merchant_name: item.merchant_name.to_string(),
                        item_name: item.item_name.to_string(),
                    },
                    build_merchant_sections(item),
                );
                state.last_detail_shop_id = Some(shop_id);
            }
        }
    }

    // Summary
    let total_count = MERCHANT_ITEMS.len();
    let filtered_count = items.len();
    if filtered_count < total_count {
        ui.label(RichText::new(format!("Shop Items: {} (showing {}/{})", total_count, filtered_count, total_count)).strong());
    } else {
        ui.label(RichText::new(format!("Shop Items: {}", total_count)).strong());
    }

    spacing::space_sm(ui);

    // Build row data
    let rows: Vec<RowData> = items.iter().map(|(shop_id, item)| {
        let is_selected = state.selected_shop_id == Some(*shop_id);

        let qty_str = if item.quantity < 0 {
            "∞".to_string()
        } else {
            item.quantity.to_string()
        };

        let mut row = RowData::new(vec![
            item.merchant_name.to_string(),
            item.item_name.to_string(),
            item.price.to_string(),
            qty_str,
            item.equip_type.as_str().to_string(),
            shop_id.to_string(),
        ]);

        if is_selected {
            row = row.with_color(Color32::YELLOW);
        }

        row
    }).collect();

    // Show table with auto-width columns
    let table_response = UnifiedTable::new("merchants_table", &mut state.table_state)
        .columns(vec![
            Column::new("merchant", "Merchant").sortable(true),
            Column::new("item", "Item").sortable(true),
            Column::new("price", "Price").sortable(true).right(),
            Column::new("qty", "Stock").sortable(true).right(),
            Column::new("type", "Type").sortable(true),
            Column::new("shop_id", "Shop ID").sortable(true).monospace(true),
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
        if let Some((shop_id, item)) = items.get(row_idx) {
            detail_panel.select_with_relationships(
                SelectedEntity::Merchant {
                    shop_id: *shop_id,
                    merchant_name: item.merchant_name.to_string(),
                    item_name: item.item_name.to_string(),
                },
                build_merchant_sections(item),
            );
            state.last_detail_shop_id = Some(*shop_id);
        }
    }

    // Update selected based on table selection
    if state.table_state.selection_count() == 1 {
        if let Some(&idx) = state.table_state.selected_rows.iter().next() {
            if let Some((shop_id, _)) = items.get(idx) {
                state.selected_shop_id = Some(*shop_id);
            }
        }
    }

    // Handle export
    if export_response.export_clicked || export_response.copy_clicked {
        let data_to_export: Vec<_> = items.iter()
            .map(|(shop_id, item)| MerchantExportItem {
                shop_id: *shop_id,
                merchant: item.merchant_name.to_string(),
                item_name: item.item_name.to_string(),
                item_id: item.item_id,
                price: item.price,
                quantity: item.quantity,
                item_type: item.equip_type.as_str().to_string(),
                stock_flag: item.event_flag_stock,
                release_flag: item.event_flag_release,
            })
            .collect();

        let content = match state.export_format {
            ExportFormat::Json => {
                let export = PageExport::new(
                    PageExportMetadata::new("Merchant Items")
                        .with_counts(total_count, filtered_count),
                    &data_to_export,
                );
                to_json(&export).unwrap_or_else(|_| String::new())
            }
            ExportFormat::Csv | ExportFormat::Markdown => {
                let headers = &["Shop ID", "Merchant", "Item", "Item ID", "Price", "Stock", "Type", "Stock Flag", "Release Flag"];
                let rows: Vec<Vec<String>> = data_to_export.iter()
                    .map(|m| vec![
                        m.shop_id.to_string(),
                        m.merchant.clone(),
                        m.item_name.clone(),
                        m.item_id.to_string(),
                        m.price.to_string(),
                        m.quantity.to_string(),
                        m.item_type.clone(),
                        m.stock_flag.to_string(),
                        m.release_flag.to_string(),
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
