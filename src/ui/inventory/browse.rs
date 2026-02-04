use eframe::egui::{Ui, Color32, RichText};
use serde::Serialize;
use crate::{
    ui::components::{
        filter::{FilterBar, FilterOption, fuzzy_match_default},
        table::{UnifiedTable, Column, RowData, SortDirection},
        export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown},
    },
    ui::tokens::{colors, spacing},
    vm::{
        inventory::{InventoryTypeRoute, InventoryItemViewModel, InventoryGaitemType, StorageLocation},
        vm::vm::ViewModel,
    },
};

/// Export item structure for inventory items
#[derive(Serialize)]
struct InventoryExportItem {
    item_id: u32,
    item_name: String,
    quantity: u32,
    item_type: String,
    storage: String,
    acquisition_sort_id: u32,
}

pub fn browse_inventory(ui: &mut Ui, vm: &mut ViewModel) {
    let inventory_vm = &mut vm.slots[vm.index].inventory_vm;
    let state = &mut inventory_vm.browse_view_state;

    // Build storage location options
    let storage_options: Vec<FilterOption> = vec![
        FilterOption { label: "All".to_string(), value: "All".to_string() },
        FilterOption { label: "Equipped".to_string(), value: "Equipped".to_string() },
        FilterOption { label: "Storage Box".to_string(), value: "StorageBox".to_string() },
    ];

    // Sync filter state
    state.search = state.filter_state.search.clone();
    state.storage_location = match state.filter_state.category.as_str() {
        "Equipped" => StorageLocation::Equipped,
        "StorageBox" => StorageLocation::StorageBox,
        _ => StorageLocation::All,
    };

    // Filter bar with storage location dropdown and search
    FilterBar::new("inventory_browse_filter", &mut state.filter_state)
        .category("Storage", storage_options)
        .search("Search items...")
        .show(ui);

    spacing::space_sm(ui);

    // Type filter chips
    ui.horizontal(|ui| {
        ui.label(RichText::new("Type:").color(Color32::LIGHT_GRAY));
        for variant in InventoryTypeRoute::all_variants() {
            if ui.selectable_label(state.type_filter == *variant, variant.label()).clicked() {
                state.type_filter = *variant;
            }
        }
    });

    spacing::space_sm(ui);

    // Export toolbar
    let has_filters = state.filter_state.has_active_filters() || state.storage_location != StorageLocation::All;
    let export_response = ExportToolbar::new("inventory_browse_export", &mut state.export_format, &mut state.export_filtered_only)
        .has_filters(has_filters)
        .show(ui);

    spacing::space_sm(ui);

    // Get filter values
    let storage_location = state.storage_location;
    let type_filter = state.type_filter;
    let search = state.search.clone();
    let export_format = state.export_format;

    // Build combined item list based on storage location
    let mut items: Vec<(InventoryItemViewModel, &'static str)> = Vec::new();

    // Helper to get items from a storage by type
    let get_items_by_type = |storage: &crate::vm::inventory::InventoryStorage, type_filter: InventoryTypeRoute| -> Vec<InventoryItemViewModel> {
        match type_filter {
            InventoryTypeRoute::CommonItems => storage.common_items.iter()
                .filter(|i| i.r#type == InventoryGaitemType::ITEM)
                .cloned()
                .collect(),
            InventoryTypeRoute::KeyItems => storage.key_items.iter()
                .filter(|i| i.quantity > 0)
                .cloned()
                .collect(),
            InventoryTypeRoute::Weapons => storage.common_items.iter()
                .filter(|i| i.r#type == InventoryGaitemType::WEAPON)
                .cloned()
                .collect(),
            InventoryTypeRoute::Armors => storage.common_items.iter()
                .filter(|i| i.r#type == InventoryGaitemType::ARMOR)
                .cloned()
                .collect(),
            InventoryTypeRoute::AshOfWar => storage.common_items.iter()
                .filter(|i| i.r#type == InventoryGaitemType::AOW)
                .cloned()
                .collect(),
            InventoryTypeRoute::Talismans => storage.common_items.iter()
                .filter(|i| i.r#type == InventoryGaitemType::ACCESSORY)
                .cloned()
                .collect(),
        }
    };

    // Get items based on storage location filter
    match storage_location {
        StorageLocation::All => {
            // Add from equipped (storage[0])
            for item in get_items_by_type(&inventory_vm.storage[0], type_filter) {
                items.push((item, "Equipped"));
            }
            // Add from storage box (storage[1])
            for item in get_items_by_type(&inventory_vm.storage[1], type_filter) {
                items.push((item, "Storage Box"));
            }
        }
        StorageLocation::Equipped => {
            for item in get_items_by_type(&inventory_vm.storage[0], type_filter) {
                items.push((item, "Equipped"));
            }
        }
        StorageLocation::StorageBox => {
            for item in get_items_by_type(&inventory_vm.storage[1], type_filter) {
                items.push((item, "Storage Box"));
            }
        }
    }

    // Apply search filter
    if !search.is_empty() {
        items.retain(|(item, _)| fuzzy_match_default(&item.item_name, &search));
    }

    // Apply sorting
    if let Some(sort_col) = &state.table_state.sort_column {
        let asc = state.table_state.sort_direction == SortDirection::Ascending;
        match sort_col.as_str() {
            "item_id" => items.sort_by(|a, b| if asc { a.0.item_id.cmp(&b.0.item_id) } else { b.0.item_id.cmp(&a.0.item_id) }),
            "name" => items.sort_by(|a, b| if asc { a.0.item_name.cmp(&b.0.item_name) } else { b.0.item_name.cmp(&a.0.item_name) }),
            "qty" => items.sort_by(|a, b| if asc { a.0.quantity.cmp(&b.0.quantity) } else { b.0.quantity.cmp(&a.0.quantity) }),
            "storage" => items.sort_by(|a, b| if asc { a.1.cmp(b.1) } else { b.1.cmp(a.1) }),
            "sort_id" => items.sort_by(|a, b| if asc { a.0.inventory_index.cmp(&b.0.inventory_index) } else { b.0.inventory_index.cmp(&a.0.inventory_index) }),
            _ => {}
        }
    }

    // Count totals
    let total_equipped: usize = get_items_by_type(&inventory_vm.storage[0], type_filter).len();
    let total_storage: usize = get_items_by_type(&inventory_vm.storage[1], type_filter).len();
    let total_count = total_equipped + total_storage;
    let filtered_count = items.len();

    // Summary
    let summary = if filtered_count < total_count {
        format!("{}: {} items (showing {})", type_filter.label(), total_count, filtered_count)
    } else {
        format!("{}: {} items ({} equipped, {} in storage)", type_filter.label(), total_count, total_equipped, total_storage)
    };
    ui.label(RichText::new(&summary).strong());

    spacing::space_sm(ui);

    // Build row data
    let rows: Vec<RowData> = items.iter().map(|(item, storage)| {
        // Determine color based on storage location
        let row_color = if *storage == "Equipped" {
            colors::STATUS_COLLECTED
        } else {
            Color32::LIGHT_GRAY
        };

        RowData::new(vec![
            item.item_id.to_string(),
            item.item_name.clone(),
            item.quantity.to_string(),
            storage.to_string(),
            item.inventory_index.to_string(),
        ]).with_color(row_color)
    }).collect();

    // Define columns
    let columns = vec![
        Column::new("item_id", "Item ID").width(100.0).sortable(true).monospace(true),
        Column::new("name", "Item Name").width_fraction(0.35).sortable(true),
        Column::new("qty", "Qty").width(60.0).sortable(true).right(),
        Column::new("storage", "Storage").width(100.0).sortable(true),
        Column::new("sort_id", "Sort ID").width(80.0).sortable(true).monospace(true),
    ];

    // Show table
    let table_response = UnifiedTable::new("inventory_browse_table", &mut state.table_state)
        .columns(columns)
        .rows(rows)
        .zebra_stripe(true)
        .selectable(true)
        .show(ui);

    // Handle copy
    if let Some(text) = table_response.clipboard_text {
        ui.output_mut(|o| o.copied_text = text);
    }

    // Handle double-click to copy row
    if let Some(row_idx) = table_response.double_clicked_row {
        if let Some((item, storage)) = items.get(row_idx) {
            let row_text = format!("{}\t{}\t{}\t{}\t{}", item.item_id, item.item_name, item.quantity, storage, item.inventory_index);
            ui.output_mut(|o| o.copied_text = row_text);
        }
    }

    // Handle export
    if export_response.export_clicked || export_response.copy_clicked {
        let export_data: Vec<InventoryExportItem> = items.iter()
            .map(|(item, storage)| InventoryExportItem {
                item_id: item.item_id,
                item_name: item.item_name.clone(),
                quantity: item.quantity,
                item_type: type_filter.label().to_string(),
                storage: storage.to_string(),
                acquisition_sort_id: item.inventory_index,
            })
            .collect();

        let content = match export_format {
            ExportFormat::Json => {
                let export = PageExport::new(
                    PageExportMetadata::new(&format!("Inventory - {}", type_filter.label()))
                        .with_counts(total_count, filtered_count),
                    &export_data,
                );
                to_json(&export).unwrap_or_default()
            }
            ExportFormat::Csv => {
                let headers = &["Item ID", "Item Name", "Quantity", "Type", "Storage", "Sort ID"];
                let rows: Vec<Vec<String>> = export_data.iter()
                    .map(|item| vec![
                        item.item_id.to_string(),
                        item.item_name.clone(),
                        item.quantity.to_string(),
                        item.item_type.clone(),
                        item.storage.clone(),
                        item.acquisition_sort_id.to_string(),
                    ])
                    .collect();
                to_csv(headers, &rows)
            }
            ExportFormat::Markdown => {
                let headers = &["Item ID", "Item Name", "Quantity", "Type", "Storage", "Sort ID"];
                let rows: Vec<Vec<String>> = export_data.iter()
                    .map(|item| vec![
                        item.item_id.to_string(),
                        item.item_name.clone(),
                        item.quantity.to_string(),
                        item.item_type.clone(),
                        item.storage.clone(),
                        item.acquisition_sort_id.to_string(),
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
