#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
mod vm;
mod save;
mod util;
mod read;
mod write;
mod ui;
mod db;
mod generated;

use std::{fs::File, io::Write, path::PathBuf};

use eframe::{egui::{self, text::LayoutJob, Align, FontSelection, Id, LayerId, Layout, Order, RichText, Rounding, Style}, epaint::Color32};
use rfd::FileDialog;
use save::save::save::{Save, SaveType};
use ui::{equipment::equipment::equipment, events::events::events, general::general::general, inventory::inventory::inventory::inventory, menu::menu::{menu, database_menu, Route}, none::none::none, regions::regions::regions, stats::stats::stats, spells_view::spells_view::{spells_view, SpellsViewState}, npcs_view::npcs_view::{npcs_view, NpcsViewState}, shop_items_view::shop_items_view::{shop_items_view, ShopItemsViewState}, world_pickups_view::world_pickups_view::{world_pickups_view, WorldPickupsViewState}, event_flags_db_view::event_flags_db_view::{event_flags_db_view, EventFlagsDbViewState}};
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
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Fill);
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
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            save: Save::default(),
            picked_path: Default::default(),
            current_route: Route::None,
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
        }
    }

    /// Load verification records for the current slot into slot's events_vm
    fn load_verification_records_for_slot(&mut self) {
        let slot_index = self.vm.index;

        // Default path - relative to typical development location
        let default_path = std::path::Path::new("../elden-map/server/data/verification-records-manually-set-to-complete.jsonl");

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
        self.picked_path = path.clone();
        // Reset verification state - will be loaded on demand per slot
        self.verification_loaded_slots = [false; 10];
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

    fn open_file_dialog() -> Option<PathBuf> {
        FileDialog::new()
        .add_filter("SL2", &["sl2", "Regular Save File"])
        .add_filter("TXT", &["txt", "Save Wizard Exported TXT File"])
        .add_filter("*", &["*", "All files"])
        .set_directory("/")
        .pick_file()
    }

    fn save_file_dialog() -> Option<PathBuf> {
        FileDialog::new()
        .add_filter("SL2", &["sl2", "Regular Save File"])
        .add_filter("TXT", &["txt", "Save Wizard Exported TXT File"])
        .add_filter("*", &["*", "Any format"])
        .set_directory("/")
        .save_file()
    }

    fn export_file_dialog(character_name: &str) -> Option<PathBuf> {
        FileDialog::new()
        .add_filter("JSON", &["json"])
        .set_file_name(&format!("{}.json", character_name))
        .set_directory("/")
        .save_file()
    }

    fn export_character(&mut self) {
        let slot_index = self.vm.index;
        let character_name = self.vm.slots[slot_index].general_vm.character_name.trim_matches('\0');
        let path = Self::export_file_dialog(character_name);

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
        // TOP PANEL
        egui::TopBottomPanel::top("toolbar").default_height(35.).show(ctx, |ui| {
            ui.columns(2, |uis|{
                uis[0].with_layout(Layout::left_to_right(Align::Center),| ui| {
                    if ui.button(egui::RichText::new(format!("{} open", egui_phosphor::regular::FOLDER_OPEN))).clicked() {
                        let files = Self::open_file_dialog();
                        match files {
                            Some(path) => self.open(path),
                            None => {},
                        }
                    }
                    // Save disabled - display only mode
                    ui.add_enabled(false, egui::Button::new(egui::RichText::new(format!("{} save", egui_phosphor::regular::FLOPPY_DISK)).strikethrough()));
                });

                uis[1].with_layout(Layout::right_to_left(egui::Align::Center),|ui| {
                    // Import disabled - display only mode
                    ui.add_enabled(false, egui::Button::new(egui::RichText::new(format!("{} Import Character", egui_phosphor::regular::DOWNLOAD_SIMPLE)).strikethrough()));

                    // Export Character button (JSON export still enabled)
                    let export_button = egui::widgets::Button::new(egui::RichText::new(format!("{} Export JSON", egui_phosphor::regular::UPLOAD_SIMPLE)));
                    if ui.add_enabled(!self.vm.steam_id.is_empty(), export_button).clicked() {
                        self.export_character();
                    }
                });
            });

        });

        // TOP PANEL
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            if self.picked_path.exists() {
                let save_type = match self.save.save_type {
                    SaveType::Unknown => {
                        "Platform: Unknown"
                    }
                    SaveType::PC(_) => {
                        "Platform: PC"
                    }
                    SaveType::PlayStation(_) => {
                        "Platform: Playstation"
                    },
                };

                ui.columns(2,| uis| {
                    if self.vm.active.is_some_and(|valid| valid) {
                        egui::Frame::none().show(&mut uis[1], |ui| {
                            let steam_id_text_edit = egui::widgets::TextEdit::singleline(&mut self.vm.steam_id)
                            .char_limit(17)
                            .desired_width(125.);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("Character: {}", self.vm.slots[self.vm.index].general_vm.character_name));

                                match self.save.save_type {
                                    SaveType::Unknown => {},
                                    SaveType::PC(_) => {
                                        let steam_id_text_edit = ui.add(steam_id_text_edit).labelled_by(ui.label("Steam Id:").id);
                                        if steam_id_text_edit.hovered() {
                                            egui::popup::show_tooltip(ui.ctx(), ui.layer_id(), steam_id_text_edit.id, |ui: &mut egui::Ui|{
                                                ui.label(egui::RichText::new("Important: This needs to match the id of the steam account that will use this save!").size(8.0).color(Color32::PLACEHOLDER));
                                            });
                                        }
                                    },
                                    SaveType::PlayStation(_) => {},
                                };
                            });
                        });
                    }
                    egui::Frame::none().show(&mut uis[0], |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(format!("{}",save_type));
                        });
                    });
                });
            }
        });

        // Character List Panel
        if self.vm.active.is_some_and(|valid| valid) {
            egui::SidePanel::left("characters").show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("left")
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            // Character list
                            for i in 0..0xA {
                                if self.vm.profile_summary[i].active {
                                    let button = ui.add_sized([120., 40.], egui::Button::new(&self.vm.slots[i].general_vm.character_name));
                                    if button.clicked() {
                                        self.vm.index = i;
                                        // Switch to General view when selecting a character (if not already in a character view)
                                        if !self.current_route.is_character_view() {
                                            self.current_route = Route::General;
                                        }
                                    }
                                    if self.vm.index == i && self.current_route.is_character_view() {
                                        button.highlight();
                                    }
                                }
                            }

                            // Database Views section (under character list)
                            ui.add_space(20.);
                            ui.separator();
                            database_menu(ui, self);
                        })
                    });
            });

            // Slot Section Panel (only show for character views)
            if self.current_route.is_character_view() {
                egui::SidePanel::left("slot_sections_menu").show(ctx, |ui| {
                    egui::ScrollArea::vertical().id_salt("left").show(ui, |ui| {
                        ui.vertical(|ui| {
                            menu(ui, self);
                        })
                    });
                });
            }

            // Main Content
            egui::CentralPanel::default().show(ctx, |ui| {
                match self.current_route {
                    Route::None => none(ui),
                    Route::General => general(ui, &mut self.vm),
                    Route::Stats => stats(ui, &mut self.vm),
                    Route::Equipment => equipment(ui, &mut self.vm),
                    Route::Inventory => inventory(ui, &mut self.vm),
                    Route::EventFlags => {
                        // Load verification records on demand for current slot
                        if !self.verification_loaded_slots[self.vm.index] {
                            self.load_verification_records_for_slot();
                        }
                        let event_flags = self.save.save_type.get_event_flags(self.vm.index);
                        events(ui, &mut self.vm, event_flags);
                    },
                    Route::Regions => regions(ui, &mut self.vm),
                    Route::Spells => spells_view(ui, &mut self.spells_view_state),
                    Route::Npcs => npcs_view(ui, &mut self.npcs_view_state),
                    Route::ShopItems => shop_items_view(ui, &mut self.shop_items_view_state),
                    Route::WorldPickups => {
                        let event_flags = self.save.save_type.get_event_flags(self.vm.index);
                        world_pickups_view(ui, &mut self.world_pickups_view_state, event_flags);
                    },
                    Route::EventFlagsDb => event_flags_db_view(ui, &mut self.event_flags_db_view_state),
                }
            });
        }
        // No file loaded View
        else {
            // Listen for dragged files and update path
            egui::CentralPanel::default().show(ctx, |ui| {
                // Check if hovering a file
                let path = ctx.input(|i| {
                    if !i.raw.hovered_files.is_empty() {
                        let file = i.raw.hovered_files[0].clone();
                        let path: std::path::PathBuf = file.path.expect("Error!");
                        return path.into_os_string().into_string().expect("");
                    }
                    "".to_string()
                });

                // Display indicator of hovering file
                ui.centered_and_justified(|ui| {
                    if !path.is_empty() {
                        let painter =
                            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

                        let screen_rect = ctx.screen_rect();
                        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(96));
                        ui.label(egui::RichText::new(path));
                    }
                    else {
                        let style = Style::default();
                        let mut layout_job = LayoutJob::default();
                        if self.vm.active.is_some_and(|valid| !valid) {
                            RichText::new("Save file has irregular data!\n\n")
                            .color(Color32::DARK_RED)
                            .append_to(
                                &mut layout_job,
                                &style,
                                FontSelection::Default,
                                Align::Center,
                            );
                        }
                        RichText::new("Drop a save file here or click 'Open' to browse")
                        .append_to(
                            &mut layout_job,
                            &style,
                            FontSelection::Default,
                            Align::Center,
                        );
                        ui.label(layout_job);
                    }
                });

                // Check a file that has been dropped in the window
                ctx.input(|i| {
                    if !i.raw.dropped_files.is_empty() {
                        let file = i.raw.dropped_files[0].clone();
                        let path: std::path::PathBuf = file.path.expect("Error!");
                        self.open(path);
                    }
                });
            });
        }
    }
}
