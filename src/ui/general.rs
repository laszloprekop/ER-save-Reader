pub mod general {
    use eframe::egui::{self, Ui, Color32, RichText};
    use crate::vm::{general::general_view_model::Gender, vm::vm::ViewModel};
    use crate::ui::style::TABLE_MONO_SIZE;

    pub fn general(ui: &mut Ui, vm: &mut ViewModel) {
        let general_vm = &vm.slots[vm.index].general_vm;

        // Column headers
        ui.horizontal(|ui| {
            ui.label(RichText::new("Property | Value").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                // Character Name
                let name_row = format!("Character Name | {}", general_vm.character_name.trim_matches('\0'));
                display_row(ui, &name_row, "Character Name", general_vm.character_name.trim_matches('\0'));

                // Gender
                let gender_str = match general_vm.gender {
                    Gender::Male => "Male",
                    Gender::Female => "Female",
                    Gender::Uknown => "Unknown",
                };
                let gender_row = format!("Gender | {}", gender_str);
                display_row(ui, &gender_row, "Gender", gender_str);

                // Level (calculated)
                let stats_vm = &vm.slots[vm.index].stats_vm;
                let level = stats_vm.vigor + stats_vm.mind + stats_vm.endurance +
                    stats_vm.strength + stats_vm.dexterity + stats_vm.intelligence +
                    stats_vm.faith + stats_vm.arcane - 79;
                let level_row = format!("Level | {}", level);
                display_row(ui, &level_row, "Level", &level.to_string());

                // Class
                let class_str = stats_vm.arche_type.to_string();
                let class_row = format!("Starting Class | {}", class_str);
                display_row(ui, &class_row, "Starting Class", &class_str);

                // Souls
                let souls_row = format!("Runes | {}", stats_vm.souls);
                display_row(ui, &souls_row, "Runes", &stats_vm.souls.to_string());
            });
    }

    fn display_row(ui: &mut Ui, row_text: &str, _label: &str, value: &str) {
        let response = ui.add(
            egui::Label::new(RichText::new(row_text).color(Color32::LIGHT_GRAY).monospace().size(TABLE_MONO_SIZE))
                .sense(egui::Sense::click())
        );

        if response.double_clicked() {
            ui.output_mut(|o| o.copied_text = row_text.to_string());
        }

        response.context_menu(|ui| {
            if ui.button("Copy row").clicked() {
                ui.output_mut(|o| o.copied_text = row_text.to_string());
                ui.close_menu();
            }
            if ui.button("Copy value").clicked() {
                ui.output_mut(|o| o.copied_text = value.to_string());
                ui.close_menu();
            }
        });
    }
}
