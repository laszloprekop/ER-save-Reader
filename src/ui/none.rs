pub mod none {
    use eframe::egui::{self, Ui};

    pub fn none(ui: &mut Ui) {
        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
            ui.label("Select a character or database view from the navigation bar above");
        });
    }
}