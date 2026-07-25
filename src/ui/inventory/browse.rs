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
        inventory::{InventoryTypeRoute, InventoryItemViewModel, InventoryGaitemType, InventoryStorage, StorageLocation, resolve_item_name},
        vm::vm::ViewModel,
    },
};
use er_reconstruct::{ReconstructedCharacter, InventoryFact, ItemCategory};

/// Export item structure for inventory items
#[derive(Serialize)]
struct InventoryExportItem {
    item_id: u32,
    item_name: String,
    quantity: u32,
    item_type: String,
    storage: String,
}

/// The reader's `(gaitem type, display name)` for a core inventory fact's
/// `(category, item_id)`. The category maps 1:1 to the reader's gaitem type; the name
/// comes from the shared [`resolve_item_name`] Enrichment resolver — the same one
/// `InventoryItemViewModel::from_save` uses — so a facts-sourced name is identical to
/// the ViewModel's. The core carries only the id; the name is Enrichment (ADR-0010).
fn resolve_fact_name(category: ItemCategory, item_id: u32) -> (InventoryGaitemType, String) {
    let gaitem_type = match category {
        ItemCategory::Weapon => InventoryGaitemType::Weapon,
        ItemCategory::Armor => InventoryGaitemType::Armor,
        ItemCategory::Accessory => InventoryGaitemType::Accessory,
        ItemCategory::Item => InventoryGaitemType::Item,
        ItemCategory::Aow => InventoryGaitemType::Aow,
    };
    let name = resolve_item_name(&gaitem_type, item_id);
    (gaitem_type, name)
}

/// One browse row built from a shared-core inventory fact (ADR-0010): identity
/// (id / quantity / type) straight from the fact, name via the reader's Enrichment
/// tables. The per-save acquisition index ("Sort ID") is not a fact and is not
/// surfaced — the browse view no longer shows it (churn, not identity).
fn item_from_fact(fact: &InventoryFact) -> InventoryItemViewModel {
    let (r#type, item_name) = resolve_fact_name(fact.category, fact.item_id);
    InventoryItemViewModel {
        ga_item_handle: 0,
        item_id: fact.item_id,
        item_name,
        quantity: fact.quantity,
        inventory_index: 0,
        equip_index: 0,
        r#type,
    }
}

/// Build one storage's `common` and `key` item lists from the core's facts.
fn storage_from_facts(common: &[InventoryFact], key: &[InventoryFact]) -> InventoryStorage {
    InventoryStorage {
        common_items: common.iter().map(item_from_fact).collect(),
        key_items: key.iter().map(item_from_fact).collect(),
        ..Default::default()
    }
}

pub fn browse_inventory(ui: &mut Ui, vm: &mut ViewModel, facts: Option<&ReconstructedCharacter>) {
    let inventory_vm = &mut vm.slots[vm.index].inventory_vm;
    let state = &mut inventory_vm.browse_view_state;

    // Source the browse list from the shared core's inventory facts (ADR-0010) when a
    // save is loaded — identity from the facts, names via the reader's Enrichment
    // tables (`item_from_fact`). The ViewModel's own storage is the fallback for the
    // empty/default state. `storage[0]` is the held inventory, `storage[1]` the box.
    let facts_storages: Option<Vec<InventoryStorage>> = facts.map(|f| vec![
        storage_from_facts(&f.held_inventory, &f.held_key_items),
        storage_from_facts(&f.storage_inventory, &f.storage_key_items),
    ]);
    let storages: &[InventoryStorage] = facts_storages
        .as_deref()
        .unwrap_or(inventory_vm.storage.as_slice());

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
                .filter(|i| i.r#type == InventoryGaitemType::Item)
                .cloned()
                .collect(),
            InventoryTypeRoute::KeyItems => storage.key_items.iter()
                .filter(|i| i.quantity > 0)
                .cloned()
                .collect(),
            InventoryTypeRoute::Weapons => storage.common_items.iter()
                .filter(|i| i.r#type == InventoryGaitemType::Weapon)
                .cloned()
                .collect(),
            InventoryTypeRoute::Armors => storage.common_items.iter()
                .filter(|i| i.r#type == InventoryGaitemType::Armor)
                .cloned()
                .collect(),
            InventoryTypeRoute::AshOfWar => storage.common_items.iter()
                .filter(|i| i.r#type == InventoryGaitemType::Aow)
                .cloned()
                .collect(),
            InventoryTypeRoute::Talismans => storage.common_items.iter()
                .filter(|i| i.r#type == InventoryGaitemType::Accessory)
                .cloned()
                .collect(),
        }
    };

    // Get items based on storage location filter
    match storage_location {
        StorageLocation::All => {
            // Add from equipped (storage[0])
            for item in get_items_by_type(&storages[0], type_filter) {
                items.push((item, "Equipped"));
            }
            // Add from storage box (storage[1])
            for item in get_items_by_type(&storages[1], type_filter) {
                items.push((item, "Storage Box"));
            }
        }
        StorageLocation::Equipped => {
            for item in get_items_by_type(&storages[0], type_filter) {
                items.push((item, "Equipped"));
            }
        }
        StorageLocation::StorageBox => {
            for item in get_items_by_type(&storages[1], type_filter) {
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
            _ => {}
        }
    }

    // Count totals
    let total_equipped: usize = get_items_by_type(&storages[0], type_filter).len();
    let total_storage: usize = get_items_by_type(&storages[1], type_filter).len();
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
        ]).with_color(row_color)
    }).collect();

    // Define columns
    let columns = vec![
        Column::new("item_id", "Item ID").width(100.0).sortable(true).monospace(true),
        Column::new("name", "Item Name").width_fraction(0.35).sortable(true),
        Column::new("qty", "Qty").width(60.0).sortable(true).right(),
        Column::new("storage", "Storage").width(100.0).sortable(true),
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
            let row_text = format!("{}\t{}\t{}\t{}", item.item_id, item.item_name, item.quantity, storage);
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
            })
            .collect();

        let content = match export_format {
            ExportFormat::Json => {
                let export = PageExport::new(
                    PageExportMetadata::new(format!("Inventory - {}", type_filter.label()))
                        .with_counts(total_count, filtered_count),
                    &export_data,
                );
                to_json(&export).unwrap_or_default()
            }
            ExportFormat::Csv => {
                let headers = &["Item ID", "Item Name", "Quantity", "Type", "Storage"];
                let rows: Vec<Vec<String>> = export_data.iter()
                    .map(|item| vec![
                        item.item_id.to_string(),
                        item.item_name.clone(),
                        item.quantity.to_string(),
                        item.item_type.clone(),
                        item.storage.clone(),
                    ])
                    .collect();
                to_csv(headers, &rows)
            }
            ExportFormat::Markdown => {
                let headers = &["Item ID", "Item Name", "Quantity", "Type", "Storage"];
                let rows: Vec<Vec<String>> = export_data.iter()
                    .map(|item| vec![
                        item.item_id.to_string(),
                        item.item_name.clone(),
                        item.quantity.to_string(),
                        item.item_type.clone(),
                        item.storage.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::weapon_name::weapon_name::WEAPON_NAME;

    /// `item_from_fact` carries the fact's identity verbatim, drops the per-save index
    /// to 0 (not a fact), and maps the category to the reader's gaitem type.
    #[test]
    fn item_from_fact_carries_identity_and_drops_index() {
        let fact = InventoryFact { category: ItemCategory::Item, item_id: 999_999_999, quantity: 42 };
        let row = item_from_fact(&fact);
        assert_eq!(row.item_id, 999_999_999);
        assert_eq!(row.quantity, 42);
        assert_eq!(row.inventory_index, 0);
        assert!(matches!(row.r#type, InventoryGaitemType::Item));
        // An id absent from the tables renders the same `[UNKOWN_…]` form as the VM.
        assert!(row.item_name.starts_with("[UNKOWN_"));
    }

    /// Each fact category maps to the matching gaitem type (drives the type filter).
    #[test]
    fn resolve_fact_name_maps_every_category() {
        assert!(matches!(resolve_fact_name(ItemCategory::Weapon, 1).0, InventoryGaitemType::Weapon));
        assert!(matches!(resolve_fact_name(ItemCategory::Armor, 1).0, InventoryGaitemType::Armor));
        assert!(matches!(resolve_fact_name(ItemCategory::Accessory, 1).0, InventoryGaitemType::Accessory));
        assert!(matches!(resolve_fact_name(ItemCategory::Item, 1).0, InventoryGaitemType::Item));
        assert!(matches!(resolve_fact_name(ItemCategory::Aow, 1).0, InventoryGaitemType::Aow));
    }

    /// A weapon fact's full reinforced id keys the name off its base and appends the
    /// reinforcement level, exactly as `InventoryItemViewModel::from_save`. The base is
    /// pulled from the live table so the test doesn't hard-code weapon data.
    #[test]
    fn resolve_fact_name_weapon_shows_upgrade() {
        let (base, expected) = {
            let lock = WEAPON_NAME.lock().unwrap();
            lock.iter()
                .find(|(k, v)| !v.is_empty() && (**k / 100) * 100 == **k)
                .map(|(k, v)| (*k, v.to_string()))
                .expect("WEAPON_NAME has a non-empty base entry")
        };
        let (ty, name) = resolve_fact_name(ItemCategory::Weapon, base + 7);
        assert!(matches!(ty, InventoryGaitemType::Weapon));
        assert_eq!(name, format!("{} +7", expected));
    }
}
