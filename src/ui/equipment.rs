pub mod equipment {
    use eframe::egui::{Ui, Color32, RichText};
    use serde::Serialize;
    use crate::{
        ui::components::{
            table::{UnifiedTable, Column, RowData, SortDirection},
            filter::{FilterBar, fuzzy_match_default},
            export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown},
        },
        ui::tokens::spacing,
        vm::vm::vm::ViewModel,
    };

    #[derive(Serialize)]
    struct EquipmentExportItem {
        category: String,
        slot: String,
        item_name: String,
        item_id: String,
        ga_handle: String,
        is_empty: bool,
    }

    pub fn equipment(ui: &mut Ui, vm: &mut ViewModel) {
        let equipment_vm = &vm.slots[vm.index].equipment_vm;

        // Build flat equipment list: (category, slot_name, item_name, item_id, ga_handle, is_empty)
        let mut equip_data: Vec<(&str, String, String, u32, u32, bool)> = Vec::with_capacity(30);

        for (i, weapon) in equipment_vm.right_hand_armaments.iter().enumerate() {
            let is_empty = weapon.id == 0 || weapon.name == "Empty" || weapon.name == "Unarmed";
            equip_data.push(("Right Hand", format!("Slot {}", i + 1), weapon.name.clone(), weapon.id, weapon.gaitem_handle, is_empty));
        }
        for (i, weapon) in equipment_vm.left_hand_armaments.iter().enumerate() {
            let is_empty = weapon.id == 0 || weapon.name == "Empty" || weapon.name == "Unarmed";
            equip_data.push(("Left Hand", format!("Slot {}", i + 1), weapon.name.clone(), weapon.id, weapon.gaitem_handle, is_empty));
        }
        for (i, arrow) in equipment_vm.arrows.iter().enumerate() {
            let is_empty = arrow.id == 0 || arrow.name == "Empty";
            equip_data.push(("Arrows", format!("Slot {}", i + 1), arrow.name.clone(), arrow.id, arrow.gaitem_handle, is_empty));
        }
        for (i, bolt) in equipment_vm.bolts.iter().enumerate() {
            let is_empty = bolt.id == 0 || bolt.name == "Empty";
            equip_data.push(("Bolts", format!("Slot {}", i + 1), bolt.name.clone(), bolt.id, bolt.gaitem_handle, is_empty));
        }
        {
            let is_empty = equipment_vm.head.id == 0 || equipment_vm.head.name == "Empty";
            equip_data.push(("Armor", "Head".to_string(), equipment_vm.head.name.clone(), equipment_vm.head.id, equipment_vm.head.gaitem_handle, is_empty));
        }
        {
            let is_empty = equipment_vm.chest.id == 0 || equipment_vm.chest.name == "Empty";
            equip_data.push(("Armor", "Chest".to_string(), equipment_vm.chest.name.clone(), equipment_vm.chest.id, equipment_vm.chest.gaitem_handle, is_empty));
        }
        {
            let is_empty = equipment_vm.arms.id == 0 || equipment_vm.arms.name == "Empty";
            equip_data.push(("Armor", "Arms".to_string(), equipment_vm.arms.name.clone(), equipment_vm.arms.id, equipment_vm.arms.gaitem_handle, is_empty));
        }
        {
            let is_empty = equipment_vm.legs.id == 0 || equipment_vm.legs.name == "Empty";
            equip_data.push(("Armor", "Legs".to_string(), equipment_vm.legs.name.clone(), equipment_vm.legs.id, equipment_vm.legs.gaitem_handle, is_empty));
        }
        for (i, talisman) in equipment_vm.talismans.iter().enumerate() {
            let is_empty = talisman.id == 0 || talisman.name == "Empty";
            equip_data.push(("Talismans", format!("Slot {}", i + 1), talisman.name.clone(), talisman.id, talisman.gaitem_handle, is_empty));
        }
        for (i, item) in equipment_vm.quickitems.iter().enumerate() {
            let is_empty = item.id == 0 || item.name == "Empty";
            equip_data.push(("Quick Items", format!("Slot {}", i + 1), item.name.clone(), item.id, item.gaitem_handle, is_empty));
        }
        for (i, item) in equipment_vm.pouch.iter().enumerate() {
            let is_empty = item.id == 0 || item.name == "Empty";
            equip_data.push(("Pouch", format!("Slot {}", i + 1), item.name.clone(), item.id, item.gaitem_handle, is_empty));
        }

        let total_slots = equip_data.len();

        // Filter bar
        let equipment_vm = &mut vm.slots[vm.index].equipment_vm;
        FilterBar::new("equipment_filter", &mut equipment_vm.filter_state)
            .search("Search equipment...")
            .no_clear()
            .show(ui);

        // Apply search filter
        let search = equipment_vm.filter_state.search.clone();
        if !search.is_empty() {
            equip_data.retain(|(_, _, name, _, _, _)| fuzzy_match_default(name, &search));
        }

        spacing::space_sm(ui);

        // Export toolbar
        let mut dummy_filtered = false;
        let has_filters = equipment_vm.filter_state.has_active_filters();
        let export_response = ExportToolbar::new("equipment_export", &mut equipment_vm.export_format, &mut dummy_filtered)
            .has_filters(has_filters)
            .no_filter_option()
            .show(ui);

        let export_format = equipment_vm.export_format;

        spacing::space_sm(ui);

        // Apply sorting
        if let Some(sort_col) = &equipment_vm.table_state.sort_column {
            let asc = equipment_vm.table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "category" => equip_data.sort_by(|a, b| if asc { a.0.cmp(b.0) } else { b.0.cmp(a.0) }),
                "slot" => equip_data.sort_by(|a, b| if asc { a.1.cmp(&b.1) } else { b.1.cmp(&a.1) }),
                "name" => equip_data.sort_by(|a, b| if asc { a.2.cmp(&b.2) } else { b.2.cmp(&a.2) }),
                "item_id" => equip_data.sort_by(|a, b| if asc { a.3.cmp(&b.3) } else { b.3.cmp(&a.3) }),
                "ga_handle" => equip_data.sort_by(|a, b| if asc { a.4.cmp(&b.4) } else { b.4.cmp(&a.4) }),
                _ => {}
            }
        }

        // Summary
        let equipped_count = equip_data.iter().filter(|(_, _, _, _, _, is_empty)| !is_empty).count();
        let filtered_count = equip_data.len();
        if filtered_count < total_slots {
            ui.label(RichText::new(format!("Equipment: {} slots (showing {})", total_slots, filtered_count)).strong());
        } else {
            ui.label(RichText::new(format!("Equipment: {} slots ({} equipped)", total_slots, equipped_count)).strong());
        }

        spacing::space_sm(ui);

        // Build row data
        let rows: Vec<RowData> = equip_data.iter().map(|(category, slot, name, item_id, ga_handle, is_empty)| {
            let item_id_hex = format!("0x{:08X}", item_id);
            let ga_handle_hex = format!("0x{:08X}", ga_handle);

            let mut row = RowData::new(vec![
                category.to_string(),
                slot.clone(),
                name.clone(),
                item_id_hex,
                ga_handle_hex,
            ]);

            if *is_empty {
                row = row.with_color(Color32::DARK_GRAY);
            }

            row
        }).collect();

        // Show table
        let table_response = UnifiedTable::new("equipment_table", &mut vm.slots[vm.index].equipment_vm.table_state)
            .columns(vec![
                Column::new("category", "Category").width(140.0).sortable(true),
                Column::new("slot", "Slot").width(120.0).sortable(true),
                Column::new("name", "Item Name").width_fraction(0.35).sortable(true),
                Column::new("item_id", "Item ID").width(100.0).sortable(true).monospace(true),
                Column::new("ga_handle", "GA Handle").width(100.0).sortable(true).monospace(true),
            ])
            .rows(rows)
            .zebra_stripe(true)
            .selectable(true)
            .show(ui);

        // Handle clipboard copy
        if let Some(text) = table_response.clipboard_text {
            ui.output_mut(|o| o.copied_text = text);
        }

        // Handle double-click copy
        if let Some(row_idx) = table_response.double_clicked_row {
            if let Some((category, slot, name, item_id, ga_handle, _)) = equip_data.get(row_idx) {
                let row_text = format!(
                    "{}\t{}\t{}\t0x{:08X}\t0x{:08X}",
                    category, slot, name, item_id, ga_handle
                );
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let data_to_export: Vec<_> = equip_data.iter()
                .map(|(category, slot, name, item_id, ga_handle, is_empty)| EquipmentExportItem {
                    category: category.to_string(),
                    slot: slot.clone(),
                    item_name: name.clone(),
                    item_id: format!("0x{:08X}", item_id),
                    ga_handle: format!("0x{:08X}", ga_handle),
                    is_empty: *is_empty,
                })
                .collect();

            let content = match export_format {
                ExportFormat::Json => {
                    let export = PageExport::new(
                        PageExportMetadata::new("Equipment")
                            .with_counts(total_slots, filtered_count),
                        &data_to_export,
                    );
                    to_json(&export).unwrap_or_else(|_| String::new())
                }
                ExportFormat::Csv => {
                    let headers = &["Category", "Slot", "Item Name", "Item ID", "GA Handle"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|e| vec![
                            e.category.clone(),
                            e.slot.clone(),
                            e.item_name.clone(),
                            e.item_id.clone(),
                            e.ga_handle.clone(),
                        ])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["Category", "Slot", "Item Name", "Item ID", "GA Handle"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|e| vec![
                            e.category.clone(),
                            e.slot.clone(),
                            e.item_name.clone(),
                            e.item_id.clone(),
                            e.ga_handle.clone(),
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
