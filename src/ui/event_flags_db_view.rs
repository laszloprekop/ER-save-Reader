pub mod event_flags_db_view {
    use eframe::egui::{self, Ui, Color32, RichText};
    use rfd::FileDialog;
    use std::fs::File;
    use std::io::Write;
    use crate::db::event_flags_db::event_flags_db::{
        EVENT_FLAGS_DB, EventFlagCategory, EventFlagEntryOwned, get_unique_regions,
        export_to_json, export_filtered_to_json
    };
    use crate::ui::style::{TABLE_MONO_SIZE, spacer};

    #[derive(Clone, Copy, PartialEq)]
    pub enum EventFlagCategoryFilter {
        All,
        GreatRune,
        BossDefeat,
        Remembrance,
        MapFragment,
        Landmark,
        Grace,
        Cookbook,
        Whetblade,
        PotUpgrade,
        TalismanPouch,
        WorldPickup,
        DungeonPickup,
        DLCPickup,
        ShopStock,
        ShopUnlock,
        NpcState,
        SummoningPool,
        Colosseum,
        Progression,
        System,
        Unknown,
    }

    impl EventFlagCategoryFilter {
        fn matches(&self, category: EventFlagCategory) -> bool {
            match self {
                EventFlagCategoryFilter::All => true,
                EventFlagCategoryFilter::GreatRune => category == EventFlagCategory::GreatRune,
                EventFlagCategoryFilter::BossDefeat => category == EventFlagCategory::BossDefeat,
                EventFlagCategoryFilter::Remembrance => category == EventFlagCategory::Remembrance,
                EventFlagCategoryFilter::MapFragment => category == EventFlagCategory::MapFragment,
                EventFlagCategoryFilter::Landmark => category == EventFlagCategory::Landmark,
                EventFlagCategoryFilter::Grace => category == EventFlagCategory::Grace,
                EventFlagCategoryFilter::Cookbook => category == EventFlagCategory::Cookbook,
                EventFlagCategoryFilter::Whetblade => category == EventFlagCategory::Whetblade,
                EventFlagCategoryFilter::PotUpgrade => category == EventFlagCategory::PotUpgrade,
                EventFlagCategoryFilter::TalismanPouch => category == EventFlagCategory::TalismanPouch,
                EventFlagCategoryFilter::WorldPickup => category == EventFlagCategory::WorldPickup,
                EventFlagCategoryFilter::DungeonPickup => category == EventFlagCategory::DungeonPickup,
                EventFlagCategoryFilter::DLCPickup => category == EventFlagCategory::DLCPickup,
                EventFlagCategoryFilter::ShopStock => category == EventFlagCategory::ShopStock,
                EventFlagCategoryFilter::ShopUnlock => category == EventFlagCategory::ShopUnlock,
                EventFlagCategoryFilter::NpcState => category == EventFlagCategory::NpcState,
                EventFlagCategoryFilter::SummoningPool => category == EventFlagCategory::SummoningPool,
                EventFlagCategoryFilter::Colosseum => category == EventFlagCategory::Colosseum,
                EventFlagCategoryFilter::Progression => category == EventFlagCategory::Progression,
                EventFlagCategoryFilter::System => category == EventFlagCategory::System,
                EventFlagCategoryFilter::Unknown => category == EventFlagCategory::Unknown,
            }
        }
    }

    pub struct EventFlagsDbViewState {
        pub category_filter: EventFlagCategoryFilter,
        pub region_filter: String,
        pub search: String,
        pub selected_id: Option<u32>,
        pub regions_cache: Vec<String>,
        pub export_status: Option<String>,
    }

    impl Default for EventFlagsDbViewState {
        fn default() -> Self {
            Self {
                category_filter: EventFlagCategoryFilter::All,
                region_filter: "All".to_string(),
                search: String::new(),
                selected_id: None,
                regions_cache: Vec::new(),
                export_status: None,
            }
        }
    }

    pub fn event_flags_db_view(ui: &mut Ui, state: &mut EventFlagsDbViewState) {
        // Initialize regions cache if empty
        if state.regions_cache.is_empty() {
            state.regions_cache = get_unique_regions();
        }

        // Header row 1: Category filters
        ui.horizontal(|ui| {
            ui.label(RichText::new("Category:").color(Color32::LIGHT_GRAY));

            // Main category filters
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::All, "All");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::GreatRune, "Great Rune");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::BossDefeat, "Boss");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Remembrance, "Remembrance");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Grace, "Grace");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::MapFragment, "Map");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Landmark, "Landmark");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Cookbook, "Cookbook");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Whetblade, "Whetblade");
        });

        // Header row 2: More category filters
        ui.horizontal(|ui| {
            ui.add_space(60.0); // Indent to align with row above
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::WorldPickup, "World");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::DungeonPickup, "Dungeon");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::DLCPickup, "DLC");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::ShopStock, "Shop");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::NpcState, "NPC");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Progression, "Progress");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::System, "System");
        });

        spacer(ui);

        // Header row 3: Region filter and search
        ui.horizontal(|ui| {
            ui.label(RichText::new("Region:").color(Color32::LIGHT_GRAY));

            egui::ComboBox::from_id_salt("region_filter")
                .selected_text(&state.region_filter)
                .width(200.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.region_filter, "All".to_string(), "All");
                    for region in &state.regions_cache {
                        ui.selectable_value(&mut state.region_filter, region.clone(), region.as_str());
                    }
                });

            spacer(ui);
            ui.label(RichText::new("Search:").color(Color32::LIGHT_GRAY));
            ui.add(egui::TextEdit::singleline(&mut state.search).desired_width(200.0));

            if ui.button("Clear").clicked() {
                state.search.clear();
                state.region_filter = "All".to_string();
                state.category_filter = EventFlagCategoryFilter::All;
            }
        });

        spacer(ui);

        // Export buttons row
        ui.horizontal(|ui| {
            if ui.button("Export All to JSON").clicked() {
                if let Some(path) = FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name("event_flags_db.json")
                    .save_file()
                {
                    match export_to_json() {
                        Ok(json) => {
                            match File::create(&path) {
                                Ok(mut file) => {
                                    match file.write_all(json.as_bytes()) {
                                        Ok(_) => {
                                            state.export_status = Some(format!("Exported {} flags to {}", EVENT_FLAGS_DB.len(), path.display()));
                                        }
                                        Err(e) => {
                                            state.export_status = Some(format!("Write error: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    state.export_status = Some(format!("File error: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            state.export_status = Some(format!("JSON error: {}", e));
                        }
                    }
                }
            }

            if ui.button("Export Filtered to JSON").clicked() {
                let search_lower = state.search.to_lowercase();
                let filtered: Vec<&EventFlagEntryOwned> = EVENT_FLAGS_DB
                    .iter()
                    .filter(|e| filter_entry(e, state, &search_lower))
                    .collect();

                if let Some(path) = FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name("event_flags_filtered.json")
                    .save_file()
                {
                    match export_filtered_to_json(&filtered) {
                        Ok(json) => {
                            match File::create(&path) {
                                Ok(mut file) => {
                                    match file.write_all(json.as_bytes()) {
                                        Ok(_) => {
                                            state.export_status = Some(format!("Exported {} flags to {}", filtered.len(), path.display()));
                                        }
                                        Err(e) => {
                                            state.export_status = Some(format!("Write error: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    state.export_status = Some(format!("File error: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            state.export_status = Some(format!("JSON error: {}", e));
                        }
                    }
                }
            }

            // Show export status
            if let Some(status) = &state.export_status {
                ui.label(RichText::new(status).color(Color32::YELLOW).small());
            }
        });

        spacer(ui);

        // Column headers
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!(
                "{:<12} | {:<50} | {:<15} | {}",
                "Flag ID", "Name", "Category", "Region"
            )).color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        spacer(ui);

        // Count filtered entries
        let search_lower = state.search.to_lowercase();
        let filtered_count = EVENT_FLAGS_DB.iter()
            .filter(|entry| filter_entry(entry, state, &search_lower))
            .count();

        ui.label(RichText::new(format!("Showing {} of {} flags", filtered_count, EVENT_FLAGS_DB.len()))
            .color(Color32::GRAY).small());

        spacer(ui);

        // Scrollable list
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for entry in EVENT_FLAGS_DB.iter() {
                    // Apply filters
                    if !filter_entry(entry, state, &search_lower) {
                        continue;
                    }

                    let coords_str = if let Some(coords) = &entry.coords {
                        format!(" [{:.1}, {:.1}, {:.1}]", coords.x, coords.y, coords.z)
                    } else {
                        String::new()
                    };

                    let row_text = format!(
                        "{:<12} | {:<50} | {:<15} | {}{}",
                        entry.flag_id,
                        truncate_str(&entry.name, 50),
                        entry.category.name(),
                        &entry.region,
                        coords_str
                    );

                    let is_selected = state.selected_id == Some(entry.flag_id);
                    let text_color = if is_selected {
                        Color32::YELLOW
                    } else {
                        get_category_color(entry.category)
                    };

                    let response = ui.add(
                        egui::Label::new(RichText::new(&row_text).color(text_color).monospace().size(TABLE_MONO_SIZE))
                            .sense(egui::Sense::click())
                    );

                    if response.clicked() {
                        state.selected_id = Some(entry.flag_id);
                    }

                    // Copy on double-click
                    if response.double_clicked() {
                        ui.output_mut(|o| o.copied_text = row_text.clone());
                    }

                    // Context menu
                    response.context_menu(|ui| {
                        if ui.button("Copy row").clicked() {
                            ui.output_mut(|o| o.copied_text = row_text.clone());
                            ui.close_menu();
                        }
                        if ui.button("Copy Flag ID").clicked() {
                            ui.output_mut(|o| o.copied_text = entry.flag_id.to_string());
                            ui.close_menu();
                        }
                        if ui.button("Copy Name").clicked() {
                            ui.output_mut(|o| o.copied_text = entry.name.clone());
                            ui.close_menu();
                        }
                        if ui.button("Copy Flag ID (Hex)").clicked() {
                            ui.output_mut(|o| o.copied_text = format!("0x{:X}", entry.flag_id));
                            ui.close_menu();
                        }
                        if entry.coords.is_some() {
                            if ui.button("Copy Coordinates").clicked() {
                                if let Some(coords) = &entry.coords {
                                    ui.output_mut(|o| o.copied_text = format!("{}, {}, {}", coords.x, coords.y, coords.z));
                                }
                                ui.close_menu();
                            }
                        }
                    });
                }
            });
    }

    fn filter_entry(entry: &EventFlagEntryOwned, state: &EventFlagsDbViewState, search_lower: &str) -> bool {
        // Category filter
        if !state.category_filter.matches(entry.category) {
            return false;
        }

        // Region filter
        if state.region_filter != "All" && entry.region != state.region_filter {
            return false;
        }

        // Search filter (by name or flag ID)
        if !search_lower.is_empty() {
            let name_match = entry.name.to_lowercase().contains(search_lower);
            let id_match = entry.flag_id.to_string().contains(search_lower);
            if !name_match && !id_match {
                return false;
            }
        }

        true
    }

    fn truncate_str(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }

    fn get_category_color(category: EventFlagCategory) -> Color32 {
        match category {
            EventFlagCategory::GreatRune => Color32::from_rgb(255, 215, 0),      // Gold
            EventFlagCategory::BossDefeat => Color32::from_rgb(255, 100, 100),   // Red
            EventFlagCategory::Remembrance => Color32::from_rgb(255, 180, 100),  // Orange
            EventFlagCategory::MapFragment => Color32::from_rgb(150, 200, 255),  // Light blue
            EventFlagCategory::Landmark => Color32::from_rgb(180, 220, 255),     // Slightly different blue
            EventFlagCategory::Grace => Color32::from_rgb(255, 255, 150),        // Light yellow
            EventFlagCategory::Cookbook => Color32::from_rgb(200, 255, 200),     // Light green
            EventFlagCategory::Whetblade => Color32::from_rgb(200, 200, 255),    // Light purple
            EventFlagCategory::PotUpgrade => Color32::from_rgb(255, 200, 150),   // Peach
            EventFlagCategory::TalismanPouch => Color32::from_rgb(200, 150, 255),// Purple
            EventFlagCategory::WorldPickup => Color32::LIGHT_GRAY,
            EventFlagCategory::DungeonPickup => Color32::from_rgb(180, 180, 180),
            EventFlagCategory::DLCPickup => Color32::from_rgb(100, 200, 150),    // Teal
            EventFlagCategory::ShopStock => Color32::from_rgb(150, 150, 200),
            EventFlagCategory::ShopUnlock => Color32::from_rgb(150, 150, 200),
            EventFlagCategory::NpcState => Color32::from_rgb(200, 150, 200),     // Pink
            EventFlagCategory::SummoningPool => Color32::from_rgb(100, 200, 200),
            EventFlagCategory::Colosseum => Color32::from_rgb(200, 100, 100),
            EventFlagCategory::Progression => Color32::from_rgb(150, 200, 150),
            EventFlagCategory::System => Color32::GRAY,
            EventFlagCategory::Unknown => Color32::DARK_GRAY,
        }
    }
}
