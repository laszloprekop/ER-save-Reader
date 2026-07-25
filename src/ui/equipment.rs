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
        vm::equipment::equipment_view_model::{EquipmentItemViewModel, EquipItemKind, resolve_equip_name},
    };
    use er_reconstruct::{ReconstructedCharacter, EquipSlot};

    /// One browse row: (category, slot label, item name, item id, is_empty). The GA
    /// handle the view used to carry is per-save churn (not a fact) and is gone.
    type EquipRow = (&'static str, String, String, u32, bool);

    /// The equipment facts' fixed slots, in display order (matching `EquipSlot`'s own
    /// order): each slot's name-resolution kind, view category, and slot label. This is
    /// the single table driving the render, so a slot's kind and category cannot drift
    /// apart. Quick-slots and pouch are not equipment facts (ADR-0010 §08) and are not
    /// shown here.
    const EQUIP_SLOTS: [(EquipSlot, EquipItemKind, &str, &str); 18] = [
        (EquipSlot::RightHand1, EquipItemKind::Weapon, "Right Hand", "Slot 1"),
        (EquipSlot::RightHand2, EquipItemKind::Weapon, "Right Hand", "Slot 2"),
        (EquipSlot::RightHand3, EquipItemKind::Weapon, "Right Hand", "Slot 3"),
        (EquipSlot::LeftHand1, EquipItemKind::Weapon, "Left Hand", "Slot 1"),
        (EquipSlot::LeftHand2, EquipItemKind::Weapon, "Left Hand", "Slot 2"),
        (EquipSlot::LeftHand3, EquipItemKind::Weapon, "Left Hand", "Slot 3"),
        (EquipSlot::Arrow1, EquipItemKind::Projectile, "Arrows", "Slot 1"),
        (EquipSlot::Arrow2, EquipItemKind::Projectile, "Arrows", "Slot 2"),
        (EquipSlot::Bolt1, EquipItemKind::Projectile, "Bolts", "Slot 1"),
        (EquipSlot::Bolt2, EquipItemKind::Projectile, "Bolts", "Slot 2"),
        (EquipSlot::Head, EquipItemKind::Armor, "Armor", "Head"),
        (EquipSlot::Chest, EquipItemKind::Armor, "Armor", "Chest"),
        (EquipSlot::Arms, EquipItemKind::Armor, "Armor", "Arms"),
        (EquipSlot::Legs, EquipItemKind::Armor, "Armor", "Legs"),
        (EquipSlot::Talisman1, EquipItemKind::Talisman, "Talismans", "Slot 1"),
        (EquipSlot::Talisman2, EquipItemKind::Talisman, "Talismans", "Slot 2"),
        (EquipSlot::Talisman3, EquipItemKind::Talisman, "Talismans", "Slot 3"),
        (EquipSlot::Talisman4, EquipItemKind::Talisman, "Talismans", "Slot 4"),
    ];

    /// A slot's name/id is "empty" when there is nothing equipped — id 0, an absent
    /// slot, or the Unarmed hand fallback (110000). Empty rows render greyed.
    fn is_empty(item_id: u32, name: &str) -> bool {
        item_id == 0 || name == "Empty" || name == "Unarmed"
    }

    /// The equipment rows sourced from the shared core's facts (ADR-0010): every fixed
    /// slot in order, its item id straight from the fact and its name via the shared
    /// [`resolve_equip_name`] Enrichment resolver (the same one the ViewModel uses). A
    /// slot the facts don't carry is truly empty and renders "Empty". Only occupied
    /// slots are facts, so a missing hand slot never happens (hands fall back to Unarmed).
    fn equip_data_from_facts(facts: &ReconstructedCharacter) -> Vec<EquipRow> {
        EQUIP_SLOTS
            .iter()
            .map(|(slot, kind, category, slot_name)| match facts.equipment.iter().find(|e| e.slot == *slot) {
                Some(fact) => {
                    let name = resolve_equip_name(*kind, fact.item_id);
                    let empty = is_empty(fact.item_id, &name);
                    (*category, slot_name.to_string(), name, fact.item_id, empty)
                }
                None => (*category, slot_name.to_string(), "Empty".to_string(), 0, true),
            })
            .collect()
    }

    /// The equipment rows from the ViewModel — the fallback for the empty/default state,
    /// before any save is reconstructed. Mirrors the facts layout (same 18 slots, no
    /// quick-slots/pouch, no GA handle) so the two paths produce the same table shape.
    fn equip_data_from_vm(vm: &crate::vm::equipment::equipment_view_model::EquipmentViewModel) -> Vec<EquipRow> {
        fn entry(category: &'static str, slot: String, item: &EquipmentItemViewModel) -> EquipRow {
            (category, slot, item.name.clone(), item.id, is_empty(item.id, &item.name))
        }
        let mut data = Vec::with_capacity(18);
        for (i, w) in vm.right_hand_armaments.iter().enumerate() {
            data.push(entry("Right Hand", format!("Slot {}", i + 1), w));
        }
        for (i, w) in vm.left_hand_armaments.iter().enumerate() {
            data.push(entry("Left Hand", format!("Slot {}", i + 1), w));
        }
        for (i, a) in vm.arrows.iter().enumerate() {
            data.push(entry("Arrows", format!("Slot {}", i + 1), a));
        }
        for (i, b) in vm.bolts.iter().enumerate() {
            data.push(entry("Bolts", format!("Slot {}", i + 1), b));
        }
        data.push(entry("Armor", "Head".to_string(), &vm.head));
        data.push(entry("Armor", "Chest".to_string(), &vm.chest));
        data.push(entry("Armor", "Arms".to_string(), &vm.arms));
        data.push(entry("Armor", "Legs".to_string(), &vm.legs));
        for (i, t) in vm.talismans.iter().enumerate() {
            data.push(entry("Talismans", format!("Slot {}", i + 1), t));
        }
        data
    }

    #[derive(Serialize)]
    struct EquipmentExportItem {
        category: String,
        slot: String,
        item_name: String,
        item_id: String,
        is_empty: bool,
    }

    pub fn equipment(ui: &mut Ui, vm: &mut ViewModel, facts: Option<&ReconstructedCharacter>) {
        // Equipment rows come from the shared core's facts (ADR-0010) when a save is
        // loaded; the ViewModel is the fallback for the empty/default state.
        let mut equip_data: Vec<EquipRow> = match facts {
            Some(f) => equip_data_from_facts(f),
            None => equip_data_from_vm(&vm.slots[vm.index].equipment_vm),
        };

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
            equip_data.retain(|(_, _, name, _, _)| fuzzy_match_default(name, &search));
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
                _ => {}
            }
        }

        // Summary
        let equipped_count = equip_data.iter().filter(|(_, _, _, _, empty)| !empty).count();
        let filtered_count = equip_data.len();
        if filtered_count < total_slots {
            ui.label(RichText::new(format!("Equipment: {} slots (showing {})", total_slots, filtered_count)).strong());
        } else {
            ui.label(RichText::new(format!("Equipment: {} slots ({} equipped)", total_slots, equipped_count)).strong());
        }

        spacing::space_sm(ui);

        // Build row data
        let rows: Vec<RowData> = equip_data.iter().map(|(category, slot, name, item_id, empty)| {
            let item_id_hex = format!("0x{:08X}", item_id);

            let mut row = RowData::new(vec![
                category.to_string(),
                slot.clone(),
                name.clone(),
                item_id_hex,
            ]);

            if *empty {
                row = row.with_color(Color32::DARK_GRAY);
            }

            row
        }).collect();

        // Show table
        let table_response = UnifiedTable::new("equipment_table", &mut vm.slots[vm.index].equipment_vm.table_state)
            .columns(vec![
                Column::new("category", "Category").width(140.0).sortable(true),
                Column::new("slot", "Slot").width(120.0).sortable(true),
                Column::new("name", "Item Name").width_fraction(0.45).sortable(true),
                Column::new("item_id", "Item ID").width(100.0).sortable(true).monospace(true),
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
            if let Some((category, slot, name, item_id, _)) = equip_data.get(row_idx) {
                let row_text = format!(
                    "{}\t{}\t{}\t0x{:08X}",
                    category, slot, name, item_id
                );
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let data_to_export: Vec<_> = equip_data.iter()
                .map(|(category, slot, name, item_id, empty)| EquipmentExportItem {
                    category: category.to_string(),
                    slot: slot.clone(),
                    item_name: name.clone(),
                    item_id: format!("0x{:08X}", item_id),
                    is_empty: *empty,
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
                    let headers = &["Category", "Slot", "Item Name", "Item ID"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|e| vec![
                            e.category.clone(),
                            e.slot.clone(),
                            e.item_name.clone(),
                            e.item_id.clone(),
                        ])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["Category", "Slot", "Item Name", "Item ID"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|e| vec![
                            e.category.clone(),
                            e.slot.clone(),
                            e.item_name.clone(),
                            e.item_id.clone(),
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

        /// A slot reads empty for nothing-equipped: id 0, the "Empty" fallback, or the
        /// Unarmed hand fallback — regardless of the other field.
        #[test]
        fn is_empty_flags_nothing_equipped() {
            assert!(is_empty(0, "Longsword"));
            assert!(is_empty(110000, "Unarmed"));
            assert!(is_empty(5, "Empty"));
            assert!(!is_empty(1000000, "Longsword"));
        }

        /// The slot table is the full 18 fact slots, in order, and every entry's
        /// name-resolution kind agrees with its category.
        #[test]
        fn equip_slots_table_is_complete_and_consistent() {
            assert_eq!(EQUIP_SLOTS.len(), 18);
            assert_eq!(EQUIP_SLOTS[0].2, "Right Hand");
            assert_eq!(EQUIP_SLOTS[17].2, "Talismans");
            for (_slot, kind, category, _) in EQUIP_SLOTS.iter() {
                let kind_ok = match *category {
                    "Right Hand" | "Left Hand" => matches!(*kind, EquipItemKind::Weapon),
                    "Arrows" | "Bolts" => matches!(*kind, EquipItemKind::Projectile),
                    "Armor" => matches!(*kind, EquipItemKind::Armor),
                    "Talismans" => matches!(*kind, EquipItemKind::Talisman),
                    other => panic!("unexpected category {other}"),
                };
                assert!(kind_ok, "category {category} disagrees with kind");
            }
        }
    }
}
