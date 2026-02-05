//! Breadcrumb navigation display component.

use eframe::egui::{self, Ui, Color32, RichText};
use super::NavigationStack;
use crate::ui::menu::menu::Route;

/// Navigation action returned from breadcrumb interaction.
#[derive(Debug, Clone, PartialEq)]
pub enum NavAction {
    /// Navigate to a specific route.
    GoTo(Route),
    /// Go back in history.
    Back,
    /// Go forward in history.
    Forward,
    /// No action.
    None,
}

/// Render navigation breadcrumb with back/forward buttons.
///
/// Returns a `NavAction` indicating what navigation action was triggered.
pub fn navigation_breadcrumb(ui: &mut Ui, stack: &NavigationStack) -> NavAction {
    let mut action = NavAction::None;

    ui.horizontal(|ui| {
        // Back button
        let back_enabled = stack.can_go_back();
        if ui.add_enabled(back_enabled, egui::Button::new("◀").small()).clicked() {
            action = NavAction::Back;
        }

        // Forward button
        let forward_enabled = stack.can_go_forward();
        if ui.add_enabled(forward_enabled, egui::Button::new("▶").small()).clicked() {
            action = NavAction::Forward;
        }

        ui.separator();

        // Breadcrumb trail
        let entries = stack.breadcrumb_entries(5);
        for (i, entry) in entries.iter().enumerate() {
            if i > 0 {
                ui.label(RichText::new("›").color(Color32::GRAY));
            }

            let is_current = i == entries.len() - 1;
            let text = if is_current {
                RichText::new(&entry.label).strong()
            } else {
                RichText::new(&entry.label).color(Color32::from_rgb(100, 149, 237))
            };

            if !is_current {
                if ui.link(text).clicked() {
                    action = NavAction::GoTo(entry.route);
                }
            } else {
                ui.label(text);
            }
        }
    });

    action
}

/// Simpler breadcrumb that just displays the current location.
pub fn simple_breadcrumb(ui: &mut Ui, entries: &[&str]) {
    ui.horizontal(|ui| {
        for (i, entry) in entries.iter().enumerate() {
            if i > 0 {
                ui.label(RichText::new("›").color(Color32::GRAY));
            }

            let is_last = i == entries.len() - 1;
            if is_last {
                ui.label(RichText::new(*entry).strong());
            } else {
                ui.label(RichText::new(*entry).color(Color32::GRAY));
            }
        }
    });
}
