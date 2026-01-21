pub mod equipment {
    use eframe::egui::{self, Ui, Color32, RichText};
    use crate::vm::vm::vm::ViewModel;
    use crate::ui::style::TABLE_MONO_SIZE;

    pub fn equipment(ui: &mut Ui, vm: &mut ViewModel) {
        let equipment_vm = &vm.slots[vm.index].equipment_vm;

        // Column headers
        ui.horizontal(|ui| {
            ui.label(RichText::new("Slot | Item Name | Item ID | GA Handle").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                // Right Hand Weapons
                ui.label(RichText::new("Right Hand Weapons").strong());
                for (i, weapon) in equipment_vm.right_hand_armaments.iter().enumerate() {
                    let slot = format!("Right Hand {}", i + 1);
                    display_equipment_row(ui, &slot, &weapon.name, weapon.id, weapon.gaitem_handle);
                }

                ui.separator();

                // Left Hand Weapons
                ui.label(RichText::new("Left Hand Weapons").strong());
                for (i, weapon) in equipment_vm.left_hand_armaments.iter().enumerate() {
                    let slot = format!("Left Hand {}", i + 1);
                    display_equipment_row(ui, &slot, &weapon.name, weapon.id, weapon.gaitem_handle);
                }

                ui.separator();

                // Arrows
                ui.label(RichText::new("Arrows").strong());
                for (i, arrow) in equipment_vm.arrows.iter().enumerate() {
                    let slot = format!("Arrow {}", i + 1);
                    display_equipment_row(ui, &slot, &arrow.name, arrow.id, arrow.gaitem_handle);
                }

                ui.separator();

                // Bolts
                ui.label(RichText::new("Bolts").strong());
                for (i, bolt) in equipment_vm.bolts.iter().enumerate() {
                    let slot = format!("Bolt {}", i + 1);
                    display_equipment_row(ui, &slot, &bolt.name, bolt.id, bolt.gaitem_handle);
                }

                ui.separator();

                // Armor
                ui.label(RichText::new("Armor").strong());
                display_equipment_row(ui, "Head", &equipment_vm.head.name, equipment_vm.head.id, equipment_vm.head.gaitem_handle);
                display_equipment_row(ui, "Chest", &equipment_vm.chest.name, equipment_vm.chest.id, equipment_vm.chest.gaitem_handle);
                display_equipment_row(ui, "Arms", &equipment_vm.arms.name, equipment_vm.arms.id, equipment_vm.arms.gaitem_handle);
                display_equipment_row(ui, "Legs", &equipment_vm.legs.name, equipment_vm.legs.id, equipment_vm.legs.gaitem_handle);

                ui.separator();

                // Talismans
                ui.label(RichText::new("Talismans").strong());
                for (i, talisman) in equipment_vm.talismans.iter().enumerate() {
                    let slot = format!("Talisman {}", i + 1);
                    display_equipment_row(ui, &slot, &talisman.name, talisman.id, talisman.gaitem_handle);
                }

                ui.separator();

                // Quick Items
                ui.label(RichText::new("Quick Items").strong());
                for (i, item) in equipment_vm.quickitems.iter().enumerate() {
                    let slot = format!("Quick Item {}", i + 1);
                    display_equipment_row(ui, &slot, &item.name, item.id, item.gaitem_handle);
                }

                ui.separator();

                // Pouch
                ui.label(RichText::new("Pouch").strong());
                for (i, item) in equipment_vm.pouch.iter().enumerate() {
                    let slot = format!("Pouch {}", i + 1);
                    display_equipment_row(ui, &slot, &item.name, item.id, item.gaitem_handle);
                }
            });
    }

    fn display_equipment_row(ui: &mut Ui, slot: &str, name: &str, item_id: u32, ga_handle: u32) {
        let item_id_hex = format!("0x{:08X}", item_id);
        let ga_handle_hex = format!("0x{:08X}", ga_handle);
        let row_text = format!("{} | {} | {} | {}", slot, name, item_id_hex, ga_handle_hex);

        let text_color = if item_id == 0 || name == "Empty" || name == "Unarmed" {
            Color32::DARK_GRAY
        } else {
            Color32::LIGHT_GRAY
        };

        let response = ui.add(
            egui::Label::new(RichText::new(&row_text).color(text_color).monospace().size(TABLE_MONO_SIZE))
                .sense(egui::Sense::click())
        );

        if response.double_clicked() {
            ui.output_mut(|o| o.copied_text = row_text.clone());
        }

        response.context_menu(|ui| {
            if ui.button("Copy row").clicked() {
                ui.output_mut(|o| o.copied_text = row_text.clone());
                ui.close_menu();
            }
            if ui.button("Copy item name").clicked() {
                ui.output_mut(|o| o.copied_text = name.to_string());
                ui.close_menu();
            }
            if ui.button("Copy item ID").clicked() {
                ui.output_mut(|o| o.copied_text = item_id_hex);
                ui.close_menu();
            }
            if ui.button("Copy GA handle").clicked() {
                ui.output_mut(|o| o.copied_text = ga_handle_hex);
                ui.close_menu();
            }
        });
    }
}
