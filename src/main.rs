#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
mod vm;
mod save;
mod util;
mod read;
mod write;
mod ui;
mod db;
mod generated;
mod discovery;
mod calibration;

use std::{env, fs::File, io::Write, path::PathBuf};

use eframe::{egui::{self, text::LayoutJob, Align, FontSelection, Id, LayerId, Layout, Order, RichText, Rounding, Style}, epaint::Color32};
use rfd::FileDialog;
use save::save::save::{Save, SaveType};
use ui::{equipment::equipment::equipment, events::events::events, general::general::general, inventory::inventory::inventory::inventory, menu::menu::{Route, breadcrumb_bar, navigation_buttons}, none::none::none, regions::regions::regions, stats::stats::stats, spells_view::spells_view::{spells_view, SpellsViewState}, npcs_view::npcs_view::{npcs_view, NpcsViewState}, shop_items_view::shop_items_view::{shop_items_view, ShopItemsViewState}, world_pickups_view::world_pickups_view::{world_pickups_view, WorldPickupsViewState}, event_flags_db_view::event_flags_db_view::{event_flags_db_view, EventFlagsDbViewState}, components::status_bar::show_status_bar, landing::landing::landing_page, state::RecentFilesManager};
use vm::verification_vm::VerificationViewModel;
use util::verification_records::{load_verification_records, get_records_for_slot, recompute_auto_status};
use vm::{importer::general_view_model::ImporterViewModel, vm::vm::ViewModel};
use crate::write::write::Write as w;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "icon/"]
struct Asset;

const WINDOW_WIDTH: f32 = 1920.;
const WINDOW_HEIGHT: f32 = 1200.;

fn main() -> Result<(), eframe::Error> {
    // Check for CLI commands
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "discovery" {
        let cli_args: Vec<String> = args.into_iter().skip(2).collect();
        match discovery::cli::run_cli(&cli_args) {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    // App Icon
    let mut app_icon = egui::IconData::default();

    let image = Asset::get("icon.png").expect("Failed to get image data").data;
    let icon = image::load_from_memory(&image).expect("Failed to open icon path").to_rgba8();
    let (icon_width, icon_height) = icon.dimensions();
    app_icon.rgba = icon.into_raw();
    app_icon.width = icon_width;
    app_icon.height = icon_height;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("ER Save Editor {}", env!("CARGO_PKG_VERSION")))
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_icon(app_icon),
        ..Default::default()
    };

    eframe::run_native("ER Save Editor", options, Box::new(|creation_context| {
        let mut fonts = egui::FontDefinitions::default();

        // IBM Plex font family
        fonts.font_data.insert(
            "IBMPlexSans".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/IBM_Plex_Sans/static/IBMPlexSans-Regular.ttf")),
        );
        fonts.font_data.insert(
            "IBMPlexSansCondensed".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/IBM_Plex_Sans_Condensed/IBMPlexSansCondensed-Regular.ttf")),
        );
        fonts.font_data.insert(
            "IBMPlexMono".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/IBM_Plex_Mono/IBMPlexMono-Regular.ttf")),
        );
        fonts.font_data.insert(
            "IBMPlexSerif".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/IBM_Plex_Serif/IBMPlexSerif-Regular.ttf")),
        );

        // Set IBM Plex Sans as default proportional font
        fonts.families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "IBMPlexSans".to_owned());

        // Set IBM Plex Mono as default monospace font
        fonts.families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "IBMPlexMono".to_owned());

        // Named families for specific uses
        fonts.families.insert(
            egui::FontFamily::Name("Condensed".into()),
            vec!["IBMPlexSansCondensed".to_owned()],
        );
        fonts.families.insert(
            egui::FontFamily::Name("Serif".into()),
            vec!["IBMPlexSerif".to_owned()],
        );

        // Add phosphor icons (Regular variant only - Fill would overwrite Regular due to same font key)
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        creation_context.egui_ctx.set_fonts(fonts);
        let mut visuals = creation_context.egui_ctx.style().visuals.clone();
        let rounding = 3.;
        visuals.window_rounding = Rounding::default().at_least(rounding);
        visuals.window_highlight_topmost = false;
        creation_context.egui_ctx.set_visuals(visuals);
        Ok(Box::new(App::new(creation_context)))
    }))
}

pub struct App {
    save: Save,
    vm: ViewModel,
    picked_path: PathBuf,
    current_route: Route,
    importer_vm: ImporterViewModel,
    importer_open: bool,
    // Database view states
    spells_view_state: SpellsViewState,
    npcs_view_state: NpcsViewState,
    shop_items_view_state: ShopItemsViewState,
    world_pickups_view_state: WorldPickupsViewState,
    event_flags_db_view_state: EventFlagsDbViewState,
    // Track which slots have had verification records loaded
    verification_loaded_slots: [bool; 10],
    // Remember the last directory used for file dialogs
    last_directory: Option<PathBuf>,
    // Recent files manager for landing page
    recent_files: RecentFilesManager,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Load recent files from disk
        let mut recent_files = RecentFilesManager::load();
        recent_files.prune_missing();

        Self {
            save: Save::default(),
            picked_path: Default::default(),
            current_route: Route::Landing,
            vm: ViewModel::default(),
            importer_vm: Default::default(),
            importer_open: Default::default(),
            // Database view states
            spells_view_state: SpellsViewState::default(),
            npcs_view_state: NpcsViewState::default(),
            shop_items_view_state: ShopItemsViewState::default(),
            world_pickups_view_state: WorldPickupsViewState::default(),
            event_flags_db_view_state: EventFlagsDbViewState::default(),
            // Track which slots have had verification records loaded
            verification_loaded_slots: [false; 10],
            // Initialize last directory to None (will use system default)
            last_directory: None,
            // Recent files for landing page
            recent_files,
        }
    }

    /// Load verification records for the current slot into slot's events_vm
    fn load_verification_records_for_slot(&mut self) {
        let slot_index = self.vm.index;

        // Default path - relative to typical development location
        let default_path = std::path::Path::new("../elden-map/server/data/flag-correlation-candidates.jsonl");

        // Try environment variable first
        let path = std::env::var("ER_VERIFICATION_RECORDS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_path.to_path_buf());

        if let Ok(all_records) = load_verification_records(&path) {
            let mut slot_records = get_records_for_slot(&all_records, slot_index as u32);

            // Re-compute auto_status based on actual save data
            if let Some(event_flags) = self.save.save_type.get_event_flags(slot_index) {
                recompute_auto_status(&mut slot_records, event_flags);
            }

            let mut verification_vm = VerificationViewModel::from_records(slot_records);
            verification_vm.records_path = Some(path.to_string_lossy().to_string());

            // Compute discovered regions from grace flags
            let discovered_regions = self.get_discovered_regions();
            verification_vm.set_discovered_regions(discovered_regions);

            self.vm.slots[slot_index].events_vm.verification_vm = verification_vm;
        } else {
            self.vm.slots[slot_index].events_vm.verification_vm = VerificationViewModel::default();
        }
        self.verification_loaded_slots[slot_index] = true;
    }

    /// Get regions that have at least one discovered grace
    fn get_discovered_regions(&self) -> std::collections::HashSet<String> {
        use crate::db::graces::maps::GRACES;
        use crate::db::map_name::map_name::MAP_NAME;
        use crate::db::pickup_flags::is_flag_set;

        let mut regions = std::collections::HashSet::new();

        let event_flags = match self.save.save_type.get_event_flags(self.vm.index) {
            Some(flags) => flags,
            None => return regions,
        };

        // Check each grace and see if it's discovered
        let graces = match GRACES.lock() {
            Ok(g) => g,
            Err(_) => return regions,
        };

        let map_names = match MAP_NAME.lock() {
            Ok(m) => m,
            Err(_) => return regions,
        };

        for (_grace, (map_name, flag_id, _name)) in graces.iter() {
            // Check if this grace is discovered
            if is_flag_set(event_flags, *flag_id) {
                // Get the region name string
                if let Some(region_str) = map_names.get(map_name) {
                    regions.insert(region_str.to_string());
                }
            }
        }

        regions
    }

    fn open(&mut self, path: PathBuf) {
        self.save = Save::from_path(&path).expect("Failed to read save");
        self.vm = ViewModel::from_save(&self.save);
        // Remember the parent directory for next file dialog
        if let Some(parent) = path.parent() {
            self.last_directory = Some(parent.to_path_buf());
        }
        self.picked_path = path.clone();
        // Reset verification state - will be loaded on demand per slot
        self.verification_loaded_slots = [false; 10];

        // Collect character names for recent files
        let character_names: Vec<String> = self.vm.profile_summary
            .iter()
            .enumerate()
            .filter(|(_, ps)| ps.active)
            .map(|(i, _)| self.vm.slots[i].general_vm.character_name.trim_matches('\0').to_string())
            .filter(|n| !n.is_empty())
            .collect();

        // Add to recent files
        self.recent_files.add(&path, &character_names);

        // Navigate to character selection
        self.current_route = Route::CharacterSelect;
    }

    fn save(&mut self, path: PathBuf) {
        self.vm.update_save(&mut self.save.save_type);
        let mut f = File::create(path).expect("");
        let bytes = self.save.write().expect("");
        let res = f.write_all(&bytes);

        match res {
            Ok(_) => {},
            Err(_) => todo!(),
        }
    }

    fn open_file_dialog(last_dir: Option<&PathBuf>) -> Option<PathBuf> {
        let mut dialog = FileDialog::new()
            .add_filter("SL2", &["sl2", "Regular Save File"])
            .add_filter("TXT", &["txt", "Save Wizard Exported TXT File"])
            .add_filter("*", &["*", "All files"]);
        if let Some(dir) = last_dir {
            dialog = dialog.set_directory(dir);
        }
        dialog.pick_file()
    }

    fn save_file_dialog(last_dir: Option<&PathBuf>) -> Option<PathBuf> {
        let mut dialog = FileDialog::new()
            .add_filter("SL2", &["sl2", "Regular Save File"])
            .add_filter("TXT", &["txt", "Save Wizard Exported TXT File"])
            .add_filter("*", &["*", "Any format"]);
        if let Some(dir) = last_dir {
            dialog = dialog.set_directory(dir);
        }
        dialog.save_file()
    }

    fn export_file_dialog(character_name: &str, last_dir: Option<&PathBuf>) -> Option<PathBuf> {
        let mut dialog = FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name(&format!("{}.json", character_name));
        if let Some(dir) = last_dir {
            dialog = dialog.set_directory(dir);
        }
        dialog.save_file()
    }

    fn export_character(&mut self) {
        let slot_index = self.vm.index;
        let character_name = self.vm.slots[slot_index].general_vm.character_name.trim_matches('\0');
        let path = Self::export_file_dialog(character_name, self.last_directory.as_ref());

        match path {
            Some(path) => {
                let steam_id: u64 = self.vm.steam_id.parse().unwrap_or(0);
                let event_flags = self.save.save_type.get_event_flags(slot_index);
                let mut export_data = self.vm.slots[slot_index].to_export_data(slot_index, steam_id, event_flags);

                // Load verification records if not already loaded for this slot
                if !self.verification_loaded_slots[slot_index] {
                    self.load_verification_records_for_slot();
                }

                // Add verification data to export from slot's events_vm
                export_data.verification = vm::slot::slot_view_model::SlotViewModel::build_verification_export(
                    &self.vm.slots[slot_index].events_vm.verification_vm
                );

                match serde_json::to_string_pretty(&export_data) {
                    Ok(json) => {
                        if let Err(_) = std::fs::write(&path, json) {
                            // Handle write error silently for now
                        }
                    },
                    Err(_) => {
                        // Handle serialization error silently for now
                    }
                }
            },
            None => {},
        }
    }
}


impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_zoom_factor(1.5);

        // Row 1: Top Menu Bar
        egui::TopBottomPanel::top("toolbar").default_height(35.).show(ctx, |ui| {
            ui.horizontal(|ui| {
                // LEFT SIDE: Open | Database

                // Open button with dropdown for recent files
                ui.menu_button(
                    egui::RichText::new(format!("{} Open", egui_phosphor::regular::FOLDER_OPEN)),
                    |ui| {
                        if ui.button("Browse...").clicked() {
                            ui.close_menu();
                            let files = Self::open_file_dialog(self.last_directory.as_ref());
                            if let Some(path) = files {
                                self.open(path);
                            }
                        }
                        if !self.recent_files.is_empty() {
                            ui.separator();
                            ui.label(RichText::new("Recent:").small().color(Color32::GRAY));
                            // Collect paths first to avoid borrow issues
                            let recent_items: Vec<(PathBuf, String)> = self.recent_files
                                .get_recent()
                                .iter()
                                .take(5)
                                .map(|r| (r.path.clone(), r.display_name()))
                                .collect();
                            for (path, label) in recent_items {
                                if ui.button(&label).clicked() {
                                    ui.close_menu();
                                    self.open(path);
                                }
                            }
                        }
                    },
                );

                // Database button → navigates to DatabaseSelect
                if ui.button("Database").clicked() {
                    self.current_route = Route::DatabaseSelect;
                }

                // RIGHT SIDE: ~~Save~~ | Export
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Export button
                    let export_button = egui::widgets::Button::new(
                        egui::RichText::new(format!("{} Export", egui_phosphor::regular::UPLOAD_SIMPLE))
                    );
                    if ui.add_enabled(self.picked_path.exists() && !self.vm.steam_id.is_empty(), export_button).clicked() {
                        self.export_character();
                    }

                    // Save (disabled with strikethrough)
                    ui.add_enabled(
                        false,
                        egui::Button::new(
                            egui::RichText::new(format!("{} Save", egui_phosphor::regular::FLOPPY_DISK)).strikethrough()
                        ),
                    );
                });
            });
        });

        // Determine if we should show the breadcrumb and navigation bars
        // Show for all routes except Landing
        let show_panels = !matches!(self.current_route, Route::Landing);

        if show_panels {
            // Row 2: Breadcrumb
            egui::TopBottomPanel::top("breadcrumb").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    breadcrumb_bar(ui, self);
                });
            });

            // Row 3: Navigation Buttons (context-dependent)
            // Show for CharacterSelect, character views, DatabaseSelect
            let show_nav = matches!(self.current_route,
                Route::CharacterSelect |
                Route::CharacterGeneral | Route::CharacterStats | Route::CharacterEquipment |
                Route::CharacterInventory | Route::CharacterEventFlags | Route::CharacterRegions |
                Route::DatabaseSelect
            );
            if show_nav {
                egui::TopBottomPanel::top("navigation").show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        navigation_buttons(ui, self);
                    });
                });
            }

            // Status Bar (bottom) - shows icon legend
            egui::TopBottomPanel::bottom("status_bar")
                .exact_height(24.0)
                .show(ctx, |ui| {
                    show_status_bar(ui);
                });
        }

        // Main Content
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_route {
                Route::Landing => {
                    landing_page(ui, self, ctx);
                },
                Route::CharacterSelect => {
                    // Show a message prompting to select a character
                    ui.centered_and_justified(|ui| {
                        ui.label("Select a character from the menu above");
                    });
                },
                Route::CharacterGeneral => general(ui, &mut self.vm),
                Route::CharacterStats => stats(ui, &mut self.vm),
                Route::CharacterEquipment => equipment(ui, &mut self.vm),
                Route::CharacterInventory => inventory(ui, &mut self.vm),
                Route::CharacterEventFlags => {
                    // Load verification records on demand for current slot
                    if !self.verification_loaded_slots[self.vm.index] {
                        self.load_verification_records_for_slot();
                    }
                    let event_flags = self.save.save_type.get_event_flags(self.vm.index);
                    let inventory = self.save.save_type.get_inventory(self.vm.index);
                    let storage = self.save.save_type.get_storage_inventory(self.vm.index);
                    let save_path = self.picked_path.to_string_lossy().to_string();
                    events(ui, &mut self.vm, event_flags, inventory, storage, &save_path);
                },
                Route::CharacterRegions => regions(ui, &mut self.vm),
                Route::DatabaseSelect => {
                    // Show a message prompting to select a database
                    ui.centered_and_justified(|ui| {
                        ui.label("Select a database from the menu above");
                    });
                },
                Route::DatabaseSpells => spells_view(ui, &mut self.spells_view_state),
                Route::DatabaseNpcs => npcs_view(ui, &mut self.npcs_view_state),
                Route::DatabaseShopItems => shop_items_view(ui, &mut self.shop_items_view_state),
                Route::DatabaseWorldPickups => {
                    let event_flags = self.save.save_type.get_event_flags(self.vm.index);
                    let inventory = self.save.save_type.get_inventory(self.vm.index);
                    world_pickups_view(ui, &mut self.world_pickups_view_state, event_flags, inventory);
                },
                Route::DatabaseDungeonPickups => {
                    // TODO: Implement dungeon pickups view (similar to world pickups)
                    ui.centered_and_justified(|ui| {
                        ui.label("Dungeon Pickups view - Coming soon");
                    });
                },
                Route::DatabaseEventFlags => event_flags_db_view(ui, &mut self.event_flags_db_view_state),
            }
        });
    }
}
