pub mod stats {
    use eframe::egui::{self, Ui, Color32, RichText};
    use crate::vm::vm::vm::ViewModel;

    pub fn stats(ui: &mut Ui, vm: &mut ViewModel) {
        let stats_vm = &vm.slots[vm.index].stats_vm;

        // Calculate level from stats
        let level = stats_vm.vigor + stats_vm.mind + stats_vm.endurance +
            stats_vm.strength + stats_vm.dexterity + stats_vm.intelligence +
            stats_vm.faith + stats_vm.arcane - 79;

        // Column headers
        ui.horizontal(|ui| {
            ui.label(RichText::new("Stat | Value").color(Color32::YELLOW).monospace());
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                // Starting Class
                display_stat_row(ui, "Starting Class", &stats_vm.arche_type.to_string());

                ui.separator();

                // Level
                display_stat_row(ui, "Level", &level.to_string());

                ui.separator();

                // Main Stats
                display_stat_row(ui, "Vigor", &stats_vm.vigor.to_string());
                display_stat_row(ui, "Mind", &stats_vm.mind.to_string());
                display_stat_row(ui, "Endurance", &stats_vm.endurance.to_string());
                display_stat_row(ui, "Strength", &stats_vm.strength.to_string());
                display_stat_row(ui, "Dexterity", &stats_vm.dexterity.to_string());
                display_stat_row(ui, "Intelligence", &stats_vm.intelligence.to_string());
                display_stat_row(ui, "Faith", &stats_vm.faith.to_string());
                display_stat_row(ui, "Arcane", &stats_vm.arcane.to_string());

                ui.separator();

                // DLC Stats
                display_stat_row(ui, "Scadutree Blessing", &stats_vm.scadutree.to_string());
                display_stat_row(ui, "Shadow Realm Blessing", &stats_vm.spirit_ash.to_string());

                ui.separator();

                // Runes
                display_stat_row(ui, "Current Runes", &stats_vm.souls.to_string());
            });
    }

    fn display_stat_row(ui: &mut Ui, stat_name: &str, value: &str) {
        let row_text = format!("{} | {}", stat_name, value);

        let response = ui.add(
            egui::Label::new(RichText::new(&row_text).color(Color32::LIGHT_GRAY).monospace())
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
            if ui.button("Copy value").clicked() {
                ui.output_mut(|o| o.copied_text = value.to_string());
                ui.close_menu();
            }
        });
    }
}
