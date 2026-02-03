//! App-wide status bar with compact icon legend using hover tooltips.

use eframe::egui::{self, AboveOrBelow, Color32, Id, RichText, Ui};
use crate::ui::components::legend::icons;
use crate::ui::tokens::colors;
use crate::ui::style::TABLE_MONO_SIZE;

/// Icon size for compact status bar
const ICON_SIZE: f32 = TABLE_MONO_SIZE * 1.3;

/// Show the app-wide status bar with icon + label legend (detailed explanation on hover)
pub fn show_status_bar(ui: &mut Ui) {
//    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Flag:").color(Color32::GRAY).small());

        // Flag column icons - event flag detection status
        legend_item(
            ui,
            icons::COLLECTED,
            "Collected",
            "Event flag is SET in save data",
            colors::STATUS_COLLECTED,
        );
        legend_item(
            ui,
            icons::NOT_COLLECTED,
            "Not collected",
            "Event flag is NOT set",
            Color32::LIGHT_GRAY,
        );
        legend_item(
            ui,
            icons::UNKNOWN,
            "Unknown",
            "No flag ID mapped for this item",
            Color32::LIGHT_GRAY,
        );
        legend_item(
            ui,
            icons::MISMATCH,
            "Mismatch",
            "Flag and inventory status disagree",
            colors::STATUS_WARNING,
        );

        ui.separator();

        ui.label(RichText::new("Inv:").color(Color32::GRAY).small());

        // Inv column icons - inventory verification status
        legend_item(
            ui,
            icons::HIGH_CONFIDENCE,
            "In inventory",
            "Item found in character inventory/storage",
            colors::STATUS_COLLECTED,
        );
        legend_item(
            ui,
            icons::PARTIAL,
            "Uncertain",
            "Partial match (similar item ID found)",
            colors::CAT_YELLOW,
        );
        legend_item(
            ui,
            icons::NO_DATA,
            "No data",
            "Item ID not mapped for inventory lookup",
            Color32::LIGHT_GRAY,
        );
        legend_item(
            ui,
            icons::LOW_CONFIDENCE,
            "Not tracked",
            "Item type not tracked in inventory",
            Color32::LIGHT_GRAY,
        );
    });
    ui.add_space(2.0);
}

/// Render a legend item with icon + visible label, detailed explanation on hover (tooltip above)
fn legend_item(ui: &mut Ui, icon: &str, label: &str, detail: &str, color: Color32) {
    let response = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.label(RichText::new(icon).color(color).size(ICON_SIZE));
        ui.label(RichText::new(label).color(color).small());
    }).response;

    // Show tooltip above the legend item (since it's at the bottom of the screen)
    if response.hovered() {
        egui::popup::popup_above_or_below_widget(
            ui,
            Id::new(label).with("tooltip"),
            &response,
            AboveOrBelow::Above,
            egui::PopupCloseBehavior::CloseOnClick,
            |ui| {
                ui.label(detail);
            },
        );
    }

    ui.add_space(8.0);
}
