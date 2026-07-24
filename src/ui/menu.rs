pub mod menu {
    use eframe::egui::{self, Ui};
    use crate::App;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Route {
        // Landing page (home view)
        Landing,

        // Character selection (file loaded, no character selected yet)
        CharacterSelect,

        // Character views (file loaded, character selected)
        CharacterGeneral,
        CharacterStats,
        CharacterEquipment,
        CharacterInventory,
        CharacterEventFlags,
        CharacterRegions,
        CharacterComparison,
        CharacterValidation,

        // Database selection (no specific database selected yet)
        DatabaseSelect,

        // Database views (reference data from decompiled game files)
        DatabaseSpells,
        DatabaseNpcs,
        DatabaseShopItems,
        DatabaseWorldPickups,
        DatabaseDungeonPickups,
        DatabaseEventFlags,

        // Database Explorer views (new comprehensive database views)
        DatabaseItems,       // Unified items from all EquipParam files
        DatabaseGraces,      // Sites of grace from BonfireWarpParam
        DatabaseMerchants,   // Merchant inventories from ShopLineupParam
        DatabaseBosses,      // Boss encounters with defeat flags
        DatabaseEventChains, // Quest progression visualization

        // Utilities selection (no specific utility selected yet)
        UtilitiesSelect,

        // Utilities views
        UtilitiesIcons,      // Icomoon font glyph reference grid
    }

    impl Route {
        pub fn is_database_view(&self) -> bool {
            matches!(
                self,
                Route::DatabaseSpells
                    | Route::DatabaseNpcs
                    | Route::DatabaseShopItems
                    | Route::DatabaseWorldPickups
                    | Route::DatabaseDungeonPickups
                    | Route::DatabaseEventFlags
                    | Route::DatabaseItems
                    | Route::DatabaseGraces
                    | Route::DatabaseMerchants
                    | Route::DatabaseBosses
                    | Route::DatabaseEventChains
            )
        }

        pub fn is_utilities_view(&self) -> bool {
            matches!(self, Route::UtilitiesIcons)
        }

        pub fn is_character_view(&self) -> bool {
            matches!(
                self,
                Route::CharacterGeneral
                    | Route::CharacterStats
                    | Route::CharacterEquipment
                    | Route::CharacterInventory
                    | Route::CharacterEventFlags
                    | Route::CharacterRegions
                    | Route::CharacterComparison
                    | Route::CharacterValidation
            )
        }

        pub fn display_name(&self) -> &'static str {
            match self {
                Route::Landing => "",
                Route::CharacterSelect => "",
                Route::CharacterGeneral => "General",
                Route::CharacterStats => "Stats",
                Route::CharacterEquipment => "Equipment",
                Route::CharacterInventory => "Inventory",
                Route::CharacterEventFlags => "Event Flags",
                Route::CharacterRegions => "Regions",
                Route::CharacterComparison => "Comparison",
                Route::CharacterValidation => "Validation",
                Route::DatabaseSelect => "",
                Route::DatabaseSpells => "Spells",
                Route::DatabaseNpcs => "NPCs",
                Route::DatabaseShopItems => "Shop Items",
                Route::DatabaseWorldPickups => "World Pickups",
                Route::DatabaseDungeonPickups => "Dungeon Pickups",
                Route::DatabaseEventFlags => "Event Flags DB",
                Route::DatabaseItems => "Items",
                Route::DatabaseGraces => "Graces",
                Route::DatabaseMerchants => "Merchants",
                Route::DatabaseBosses => "Bosses",
                Route::DatabaseEventChains => "Quest Progress",
                Route::UtilitiesSelect => "",
                Route::UtilitiesIcons => "Icons",
            }
        }
    }

    /// Character-specific menu (shown in slot sections panel)
    pub fn character_menu(ui: &mut Ui, app: &mut App) {
        // Create the buttons
        let general = ui.add_sized([120., 40.], egui::Button::new("General"));
        let stats = ui.add_sized([120., 40.], egui::Button::new("Stats"));
        let equipment = ui.add_sized([120., 40.], egui::Button::new("Equipment"));
        let inventory = ui.add_sized([120., 40.], egui::Button::new("Inventory"));
        let event_flags = ui.add_sized([120., 40.], egui::Button::new("Event Flags"));
        let regions = ui.add_sized([120., 40.], egui::Button::new("Regions"));

        // Listen for clicks
        if general.clicked() { app.current_route = Route::CharacterGeneral; }
        if stats.clicked() { app.current_route = Route::CharacterStats; }
        if equipment.clicked() { app.current_route = Route::CharacterEquipment; }
        if inventory.clicked() { app.current_route = Route::CharacterInventory; }
        if event_flags.clicked() { app.current_route = Route::CharacterEventFlags; }
        if regions.clicked() { app.current_route = Route::CharacterRegions; }

        // Highlight active
        match app.current_route {
            Route::CharacterGeneral => { general.highlight(); },
            Route::CharacterStats => { stats.highlight(); },
            Route::CharacterEquipment => { equipment.highlight(); },
            Route::CharacterInventory => { inventory.highlight(); },
            Route::CharacterEventFlags => { event_flags.highlight(); },
            Route::CharacterRegions => { regions.highlight(); },
            _ => {},
        }
    }

    /// Database views menu (shown under character list)
    pub fn database_menu(ui: &mut Ui, app: &mut App) {
        ui.label(egui::RichText::new("Database Views").small());

        let spells = ui.add_sized([120., 40.], egui::Button::new("Spells"));
        let npcs = ui.add_sized([120., 40.], egui::Button::new("NPCs"));
        let shop_items = ui.add_sized([120., 40.], egui::Button::new("Shop Items"));
        let world_pickups = ui.add_sized([120., 40.], egui::Button::new("World Pickups"));
        let dungeon_pickups = ui.add_sized([120., 40.], egui::Button::new("Dungeon Pickups"));
        let event_flags_db = ui.add_sized([120., 40.], egui::Button::new("Event Flags DB"));

        // Listen for clicks
        if spells.clicked() { app.current_route = Route::DatabaseSpells; }
        if npcs.clicked() { app.current_route = Route::DatabaseNpcs; }
        if shop_items.clicked() { app.current_route = Route::DatabaseShopItems; }
        if world_pickups.clicked() { app.current_route = Route::DatabaseWorldPickups; }
        if dungeon_pickups.clicked() { app.current_route = Route::DatabaseDungeonPickups; }
        if event_flags_db.clicked() { app.current_route = Route::DatabaseEventFlags; }

        // Highlight active
        match app.current_route {
            Route::DatabaseSpells => { spells.highlight(); },
            Route::DatabaseNpcs => { npcs.highlight(); },
            Route::DatabaseShopItems => { shop_items.highlight(); },
            Route::DatabaseWorldPickups => { world_pickups.highlight(); },
            Route::DatabaseDungeonPickups => { dungeon_pickups.highlight(); },
            Route::DatabaseEventFlags => { event_flags_db.highlight(); },
            _ => {},
        }
    }

    /// Legacy menu function (kept for compatibility, now calls character_menu)
    pub fn menu(ui: &mut Ui, app: &mut App) {
        character_menu(ui, app);
    }

    /// Breadcrumb navigation bar (Row 2)
    /// Path A: Home → PC|SteamID → CharName → Area → Subroute
    /// Path B: Home → Database → DatabaseName
    pub fn breadcrumb_bar(ui: &mut Ui, app: &mut App) {
        use crate::vm::events::events_view_model::EventsRoute;
        use crate::save::save::save::SaveType;

        let caret = egui_phosphor::regular::CARET_RIGHT;
        let home_icon = egui_phosphor::regular::HOUSE;

        // Home icon (always visible) → goes to landing page
        if ui.selectable_label(false, home_icon).clicked() {
            app.current_route = Route::Landing;
        }

        // ===== PATH A: File/Character hierarchy =====
        // Level 2: CharacterSelect (file loaded, showing character list)
        if matches!(app.current_route, Route::CharacterSelect) && app.picked_path.exists() {
            ui.label(caret);
            // PC|SteamID - current level (not clickable, shown as strong)
            let platform_label = match &app.save.save_type {
                SaveType::PC(_) => format!("PC | {}", &app.vm.steam_id),
                SaveType::PlayStation(_) => "PlayStation".to_string(),
                SaveType::Unknown => "Unknown".to_string(),
            };
            let response = ui.label(egui::RichText::new(&platform_label).strong());
            if response.hovered() {
                egui::show_tooltip(ui.ctx(), ui.layer_id(), response.id, |ui| {
                    ui.label(app.picked_path.to_string_lossy().to_string());
                });
            }
        }
        // Level 3+: Character view (character selected)
        else if app.current_route.is_character_view() && app.picked_path.exists() {
            ui.label(caret);

            // PC|SteamID - clickable → goes to CharacterSelect
            let platform_label = match &app.save.save_type {
                SaveType::PC(_) => format!("PC | {}", &app.vm.steam_id),
                SaveType::PlayStation(_) => "PlayStation".to_string(),
                SaveType::Unknown => "Unknown".to_string(),
            };
            let response = ui.selectable_label(false, &platform_label);
            if response.hovered() {
                egui::show_tooltip(ui.ctx(), ui.layer_id(), response.id, |ui| {
                    ui.label(app.picked_path.to_string_lossy().to_string());
                });
            }
            if response.clicked() {
                app.current_route = Route::CharacterSelect;
            }

            ui.label(caret);

            // Character name segment
            let char_name = app.vm.slots[app.vm.index].general_vm.character_name.trim_matches('\0');

            // Level 3: CharacterGeneral - char name is current (strong)
            if matches!(app.current_route, Route::CharacterGeneral) {
                ui.label(egui::RichText::new(char_name).strong());
            }
            // Level 4+: Deeper views - char name is clickable
            else {
                if ui.selectable_label(false, char_name).clicked() {
                    app.current_route = Route::CharacterGeneral;
                    app.vm.slots[app.vm.index].screen_state.current_route = EventsRoute::None;
                }

                ui.label(caret);

                let area_name = app.current_route.display_name();

                // Level 4: Area view (not EventFlags or EventFlags without subroute)
                if matches!(app.current_route, Route::CharacterEventFlags) {
                    let events_route = &app.vm.slots[app.vm.index].screen_state.current_route;
                    let subroute_name = events_route.display_name();

                    if subroute_name.is_empty() {
                        // No subroute - area name is current
                        ui.label(egui::RichText::new(area_name).strong());
                    } else {
                        // Level 5: Has subroute - area name is clickable
                        if ui.selectable_label(false, area_name).clicked() {
                            app.vm.slots[app.vm.index].screen_state.current_route = EventsRoute::None;
                        }
                        ui.label(caret);
                        ui.label(egui::RichText::new(subroute_name).strong());
                    }
                } else {
                    // Other areas - area name is current
                    ui.label(egui::RichText::new(area_name).strong());
                }
            }
        }

        // ===== PATH B: Database hierarchy =====
        // Level 2: DatabaseSelect (showing database list)
        else if matches!(app.current_route, Route::DatabaseSelect) {
            ui.label(caret);
            ui.label(egui::RichText::new("Database").strong());
        }
        // Level 3: Specific database view
        else if app.current_route.is_database_view() {
            ui.label(caret);
            // Database - clickable → goes to DatabaseSelect
            if ui.selectable_label(false, "Database").clicked() {
                app.current_route = Route::DatabaseSelect;
            }
            ui.label(caret);
            ui.label(egui::RichText::new(app.current_route.display_name()).strong());
        }

        // ===== PATH C: Utilities hierarchy =====
        // Level 2: UtilitiesSelect (showing utilities list)
        else if matches!(app.current_route, Route::UtilitiesSelect) {
            ui.label(caret);
            ui.label(egui::RichText::new("Utilities").strong());
        }
        // Level 3: Specific utility view
        else if app.current_route.is_utilities_view() {
            ui.label(caret);
            if ui.selectable_label(false, "Utilities").clicked() {
                app.current_route = Route::UtilitiesSelect;
            }
            ui.label(caret);
            ui.label(egui::RichText::new(app.current_route.display_name()).strong());
        }
    }

    /// Navigation buttons (Row 3 - Sub Menu)
    /// Shows context-dependent navigation items based on current route
    pub fn navigation_buttons(ui: &mut Ui, app: &mut App) {
        match app.current_route {
            // Landing: no submenu
            Route::Landing => {},

            // Path A Level 2: CharacterSelect → show character slots
            Route::CharacterSelect => {
                character_select_navigation(ui, app);
            },

            // Path A Level 3: Character view → show area buttons
            Route::CharacterGeneral | Route::CharacterStats | Route::CharacterEquipment |
            Route::CharacterInventory | Route::CharacterRegions | Route::CharacterComparison |
            Route::CharacterValidation => {
                area_navigation(ui, app);
            },

            // Path A Level 4: EventFlags → show subroute buttons
            Route::CharacterEventFlags => {
                event_flags_navigation(ui, app);
            },

            // Path B Level 2: DatabaseSelect → show database list
            Route::DatabaseSelect => {
                database_select_navigation(ui, app);
            },

            // Path B Level 3: Specific database → no submenu (or sub-items if any)
            Route::DatabaseSpells | Route::DatabaseNpcs | Route::DatabaseShopItems |
            Route::DatabaseWorldPickups | Route::DatabaseDungeonPickups | Route::DatabaseEventFlags |
            Route::DatabaseItems | Route::DatabaseGraces | Route::DatabaseMerchants | Route::DatabaseBosses |
            Route::DatabaseEventChains => {
                // No submenu for specific database views currently
            },

            // Path C Level 2: UtilitiesSelect → show utilities list
            Route::UtilitiesSelect => {
                utilities_select_navigation(ui, app);
            },

            // Path C Level 3: Specific utility → no submenu
            Route::UtilitiesIcons => {},
        }
    }

    /// Path A Level 2: Character slot buttons
    fn character_select_navigation(ui: &mut Ui, app: &mut App) {
        for i in 0..0xA {
            if app.vm.profile_summary[i].active {
                let char_name = app.vm.slots[i].general_vm.character_name.trim_matches('\0');
                let is_selected = app.vm.index == i;
                if ui.selectable_label(is_selected, char_name).clicked() {
                    app.vm.index = i;
                    app.current_route = Route::CharacterGeneral;
                }
            }
        }
    }

    /// Path A Level 3: Area buttons (General, Stats, Equipment, etc.)
    fn area_navigation(ui: &mut Ui, app: &mut App) {
        let area_buttons = [
            ("General", Route::CharacterGeneral),
            ("Stats", Route::CharacterStats),
            ("Equipment", Route::CharacterEquipment),
            ("Inventory", Route::CharacterInventory),
            ("Event Flags", Route::CharacterEventFlags),
            ("Regions", Route::CharacterRegions),
            ("Comparison", Route::CharacterComparison),
            ("Validation", Route::CharacterValidation),
        ];

        for (label, route) in area_buttons {
            let is_selected = std::mem::discriminant(&app.current_route) == std::mem::discriminant(&route);
            if ui.selectable_label(is_selected, label).clicked() {
                app.current_route = route;
            }
        }
    }

    /// Path A Level 4: EventFlags subroute buttons
    fn event_flags_navigation(ui: &mut Ui, app: &mut App) {
        use crate::vm::events::events_view_model::EventsRoute;

        let subroute_buttons = [
            ("Sites Of Grace", EventsRoute::SitesOfGrace),
            ("Whetblades", EventsRoute::Whetblades),
            ("Cookbooks", EventsRoute::Cookboks),
            ("Maps", EventsRoute::Maps),
            ("Bosses", EventsRoute::Bosses),
            // Summoning Pools hidden 2026-07-24: their flag family is unidentified, so the
            // page could only ever show "none" (the flag ids read 0 on every slot, incl. one
            // with a pool known-activated in-game). Low-value info; re-enable once the family
            // is found via a targeted differential. Route/VM/view kept, just not surfaced.
            ("Colosseums", EventsRoute::Colosseums),
            ("Landmarks", EventsRoute::Landmarks),
            ("World Pickups", EventsRoute::WorldPickups),
            ("Dungeon Pickups", EventsRoute::DungeonPickups),
            ("Verification", EventsRoute::Verification),
        ];

        let current_subroute = app.vm.slots[app.vm.index].screen_state.current_route.clone();

        for (label, route) in subroute_buttons {
            let is_selected = std::mem::discriminant(&current_subroute) == std::mem::discriminant(&route);
            if ui.selectable_label(is_selected, label).clicked() {
                app.vm.slots[app.vm.index].screen_state.current_route = route;
            }
        }
    }

    /// Path B Level 2: Database list buttons
    fn database_select_navigation(ui: &mut Ui, app: &mut App) {
        // Database Explorer section
        ui.label(egui::RichText::new("Explorer").small().color(egui::Color32::GRAY));
        let explorer_buttons = [
            ("Items", Route::DatabaseItems),
            ("Graces", Route::DatabaseGraces),
            ("Merchants", Route::DatabaseMerchants),
            ("Bosses", Route::DatabaseBosses),
            ("Quest Progress", Route::DatabaseEventChains),
        ];

        for (label, route) in explorer_buttons {
            if ui.selectable_label(false, label).clicked() {
                app.current_route = route;
            }
        }

        ui.separator();

        // Legacy database views
        ui.label(egui::RichText::new("Reference").small().color(egui::Color32::GRAY));
        let database_buttons = [
            ("World Pickups", Route::DatabaseWorldPickups),
            ("Dungeon Pickups", Route::DatabaseDungeonPickups),
            ("Event Flags DB", Route::DatabaseEventFlags),
            ("Spells", Route::DatabaseSpells),
            ("NPCs", Route::DatabaseNpcs),
            ("Shop Items", Route::DatabaseShopItems),
        ];

        for (label, route) in database_buttons {
            if ui.selectable_label(false, label).clicked() {
                app.current_route = route;
            }
        }
    }

    /// Path C Level 2: Utilities list buttons
    fn utilities_select_navigation(ui: &mut Ui, app: &mut App) {
        let utilities_buttons = [
            ("Icons", Route::UtilitiesIcons),
        ];

        for (label, route) in utilities_buttons {
            if ui.selectable_label(false, label).clicked() {
                app.current_route = route;
            }
        }
    }
}
