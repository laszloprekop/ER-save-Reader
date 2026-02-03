pub mod inventory {
    use eframe::egui::{self, Color32, RichText, Ui};
    use crate::ui::inventory::{add::add, browse::browse_inventory};
    use crate::vm::{inventory::InventoryRoute, vm::vm::ViewModel};

    pub fn inventory(ui: &mut Ui, vm:&mut ViewModel) {
        egui::SidePanel::left("inventory_menu").show(ui.ctx(), |ui|{
            egui::ScrollArea::vertical()
            .id_salt("inventory_item_type_menu")
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    // Add button is disabled (destructive feature)
                    let add_items = ui.add_enabled(
                        false,
                        egui::Button::new(RichText::new("Add\n(Disabled)").strikethrough())
                            .min_size(egui::vec2(120., 60.))
                    );
                    let browse_items = ui.add_sized([120., 40.], egui::Button::new("Browse"));

                    // Add button tooltip explaining why it's disabled
                    if add_items.hovered() {
                        egui::popup::show_tooltip(ui.ctx(), ui.layer_id(), add_items.id, |ui: &mut egui::Ui|{
                            ui.label(RichText::new("Editing features are disabled in display-only mode.").size(10.0).color(Color32::GRAY));
                        });
                    }
                    if browse_items.clicked() {
                        vm.slots[vm.index].inventory_vm.filter();
                        vm.regulation.filter(&vm.slots[vm.index].inventory_vm.current_type_route, &vm.slots[vm.index].inventory_vm.filter_text);
                        vm.slots[vm.index].inventory_vm.current_route = InventoryRoute::Browse
                    }

                    // Highlight active (only Browse can be active now)
                    match vm.slots[vm.index].inventory_vm.current_route {
                        InventoryRoute::None => {},
                        InventoryRoute::Add => {}, // Can't reach this state anymore
                        InventoryRoute::Browse => {browse_items.highlight();},
                    }
                })
            });
        });

        egui::CentralPanel::default().show(ui.ctx(), |ui|{
            match vm.slots[vm.index].inventory_vm.current_route {
                InventoryRoute::None => {ui.label("Empty");},
                InventoryRoute::Add => {add(ui, vm);},
                InventoryRoute::Browse => {browse_inventory(ui, vm);},
            }
        });
    }
}