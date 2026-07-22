//! Game icon loading and caching system.
//!
//! Loads item icons from extracted game files and caches them as egui textures.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Default path to extracted game icons (can be overridden)
const DEFAULT_ICONS_PATH: &str = "/Users/laszloprekop/dev/Elden Ring stuff ARCHIVE/Elden-map references/reference-images/Elden Ring v1.14 Item Images and Maps/hi_01_common-tpf-dcx_split";

/// Icon size for display (smaller than original 160x160)
pub const ICON_DISPLAY_SIZE: f32 = 64.0;
/// Small icon size for compact views
pub const ICON_SMALL_SIZE: f32 = 32.0;

/// Global icon cache
static ICON_CACHE: Lazy<Mutex<IconCache>> = Lazy::new(|| Mutex::new(IconCache::new()));

/// Cache for loaded icon textures
pub struct IconCache {
    textures: HashMap<u16, TextureHandle>,
    icons_path: PathBuf,
    placeholder_loaded: bool,
}

impl IconCache {
    fn new() -> Self {
        Self {
            textures: HashMap::new(),
            icons_path: PathBuf::from(DEFAULT_ICONS_PATH),
            placeholder_loaded: false,
        }
    }

    /// Set the path to icon files
    pub fn set_icons_path(&mut self, path: PathBuf) {
        self.icons_path = path;
        // Clear cache when path changes
        self.textures.clear();
    }

    /// Check if icons directory exists
    pub fn icons_available(&self) -> bool {
        self.icons_path.exists() && self.icons_path.is_dir()
    }
}

/// Get the path to an icon file by icon_id
fn icon_path(icons_dir: &Path, icon_id: u16) -> PathBuf {
    icons_dir.join(format!("MENU_ItemIcon_{:05}.png", icon_id))
}

/// Load an icon texture from disk
fn load_icon_texture(ctx: &egui::Context, path: &PathBuf, icon_id: u16) -> Option<TextureHandle> {
    if !path.exists() {
        return None;
    }

    match image::open(path) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let pixels = rgba.into_raw();

            let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);

            Some(ctx.load_texture(
                format!("icon_{}", icon_id),
                color_image,
                TextureOptions::LINEAR,
            ))
        }
        Err(_) => None,
    }
}

/// Get an icon texture by icon_id, loading it if necessary
pub fn get_icon(ctx: &egui::Context, icon_id: u16) -> Option<TextureHandle> {
    if icon_id == 0 {
        return None;
    }

    let mut cache = ICON_CACHE.lock().ok()?;

    // Return cached texture if available
    if let Some(texture) = cache.textures.get(&icon_id) {
        return Some(texture.clone());
    }

    // Check if icons are available
    if !cache.icons_available() {
        return None;
    }

    // Load the icon
    let path = icon_path(&cache.icons_path, icon_id);
    if let Some(texture) = load_icon_texture(ctx, &path, icon_id) {
        cache.textures.insert(icon_id, texture.clone());
        return Some(texture);
    }

    None
}

/// Check if icons are available on the system
pub fn icons_available() -> bool {
    ICON_CACHE
        .lock()
        .map(|c| c.icons_available())
        .unwrap_or(false)
}

/// Set the icons directory path
pub fn set_icons_path(path: PathBuf) {
    if let Ok(mut cache) = ICON_CACHE.lock() {
        cache.set_icons_path(path);
    }
}

/// Display an icon with fallback to placeholder
pub fn icon_image(ui: &mut egui::Ui, icon_id: u16, size: f32) -> egui::Response {
    let ctx = ui.ctx().clone();

    if let Some(texture) = get_icon(&ctx, icon_id) {
        ui.add(egui::Image::new(&texture).fit_to_exact_size(egui::vec2(size, size)))
    } else {
        // Fallback: show a dark placeholder square
        let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            ui.painter()
                .rect_filled(rect, 4.0, egui::Color32::from_rgb(40, 40, 45));
            // Draw a subtle border
            /*             ui.painter().rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 65)),
            ); */
        }

        response
    }
}

/// Display an icon with name below (compact equipment slot style)
pub fn icon_with_name(ui: &mut egui::Ui, icon_id: u16, name: &str, size: f32) -> egui::Response {
    const NAME_MAX_WIDTH: f32 = 100.0;
    let response = ui.vertical(|ui| {
        // Use the larger of icon size or name width for container
        let container_width = NAME_MAX_WIDTH.max(size);
        ui.set_width(container_width);
        ui.set_height(size + 20.0); // Icon + space for wrapped text

        // Center the icon
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            icon_image(ui, icon_id, size);

            // Name below in condensed style, wrapped to two lines
            let display_name = if name == "Empty" || name.is_empty() {
                "—"
            } else {
                name
            };

            let color = if name == "Empty" || name.is_empty() {
                egui::Color32::from_rgb(100, 100, 105)
            } else {
                egui::Color32::from_rgb(220, 220, 220)
            };

            // Use a fixed-width label that wraps to two lines
            ui.allocate_ui(egui::vec2(NAME_MAX_WIDTH, 32.0), |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(egui::RichText::new(display_name).size(9.0).color(color))
                            .wrap(),
                    );
                });
            });
        });
    });

    response.response
}
