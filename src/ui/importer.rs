pub mod import {
    use eframe::egui::{self, Color32, RichText, Ui};
    use crate::{save::save::save::Save, ui::style::spacer, vm::{importer::general_view_model::ImporterViewModel, vm::vm::ViewModel}};

    pub fn character_importer(ui: &mut Ui, open: &mut bool, importer_vm: &mut ImporterViewModel, _to_save:&mut Save, _vm: &mut ViewModel) {
        egui::Window::new("Importer")
        .open(open)
        .resizable(false)
        .show(ui.ctx(), |ui| {
            if importer_vm.valid {
                ui.columns(2, |uis|{
                    uis[0].vertical_centered_justified(|ui|{
                        ui.heading("From");
                        spacer(ui);
                        for (i, from_character) in importer_vm.from_list.iter().filter(|c|c.active).enumerate() {
                            if ui.selectable_label(importer_vm.selected_from_index == i, &from_character.name).clicked() {
                                importer_vm.selected_from_index = i
                            }
                        }
                    });
                    uis[1].vertical_centered_justified(|ui|{
                        ui.heading("To");
                        spacer(ui);
                        for (i, to_character) in importer_vm.to_list.iter().filter(|c|c.active).enumerate() {
                            if ui.selectable_label(importer_vm.selected_to_index == i, &to_character.name).clicked() {
                                importer_vm.selected_to_index = i
                            }
                        }
                    });
                });
                ui.add_space(5.);
                ui.vertical_centered_justified(|ui|{
                    // Import button is disabled (destructive feature)
                    let import_btn = ui.add_enabled(
                        false,
                        egui::Button::new(RichText::new("Import (Disabled)").strikethrough())
                            .min_size(egui::vec2(ui.available_width(), 40.))
                    );
                    if import_btn.hovered() {
                        egui::popup::show_tooltip(ui.ctx(), ui.layer_id(), import_btn.id, |ui: &mut egui::Ui|{
                            ui.label(RichText::new("Editing features are disabled in display-only mode.").size(10.0).color(Color32::GRAY));
                        });
                    }
                });
            }
            else {
                ui.label(RichText::new("Save file has irregular data!").color(Color32::DARK_RED));
            }
        });
    }
}