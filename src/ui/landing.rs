//! Landing page shown when no file is loaded or when navigating to home.

pub mod landing {
    use eframe::egui::{self, Color32, Id, LayerId, Order, RichText, Ui};
    use crate::App;
    

    /// Render the landing page
    pub fn landing_page(ui: &mut Ui, app: &mut App, ctx: &egui::Context) {
        // Check if hovering a file (drag-drop)
        let hovered_path = ctx.input(|i| {
            if !i.raw.hovered_files.is_empty() {
                let file = i.raw.hovered_files[0].clone();
                file.path.map(|p| p.to_string_lossy().to_string())
            } else {
                None
            }
        });

        // Display drag overlay if hovering
        if let Some(path) = &hovered_path {
            let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));
            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(96));
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(path));
            });
            return;
        }

        // Check for dropped files
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let file = i.raw.dropped_files[0].clone();
                if let Some(path) = file.path {
                    app.open(path);
                }
            }
        });

        // Main landing page content
        ui.vertical_centered(|ui| {
            ui.add_space(32.0);
            ui.heading("ER Save Editor");
            ui.add_space(24.0);

            // Recent Saves section
            if !app.recent_files.is_empty() {
                ui.label(RichText::new("Recent Saves").strong());
                ui.add_space(12.0);

                // Collect recent files info first to avoid borrow issues
                let recent_items: Vec<(std::path::PathBuf, String)> = app.recent_files
                    .get_recent()
                    .iter()
                    .map(|recent| {
                        let display = if recent.character_names.is_empty() {
                            recent.display_name()
                        } else {
                            format!(
                                "{} - {}",
                                recent.display_name(),
                                recent.character_names.join(", ")
                            )
                        };
                        (recent.path.clone(), display)
                    })
                    .collect();

                let mut path_to_open: Option<std::path::PathBuf> = None;

                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(16.0, 8.0))
                    .show(ui, |ui| {
                        for (path, display) in &recent_items {
                            ui.horizontal(|ui| {
                                if ui.selectable_label(false, display).clicked() {
                                    path_to_open = Some(path.clone());
                                }
                            });
                        }
                    });

                // Open the file after the UI loop
                if let Some(path) = path_to_open {
                    app.open(path);
                }

                ui.add_space(24.0);
                ui.separator();
                ui.add_space(16.0);
            }

            // Fallback message
            ui.label(RichText::new("Drop a save file here or click 'Open' to browse").color(Color32::GRAY));
        });
    }
}
