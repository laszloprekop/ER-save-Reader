pub mod menu {
    use eframe::egui::{self, Ui};
    use crate::App;

    pub enum Route {
        None,
        General,
        Stats,
        Equipment,
        Inventory,
        EventFlags,
        Regions,
        // Database views
        Spells,
        Npcs,
        ShopItems,
        WorldPickups,
        EventFlagsDb,
    }

    impl Route {
        pub fn is_database_view(&self) -> bool {
            matches!(self, Route::Spells | Route::Npcs | Route::ShopItems | Route::WorldPickups | Route::EventFlagsDb)
        }

        pub fn is_character_view(&self) -> bool {
            matches!(self, Route::General | Route::Stats | Route::Equipment | Route::Inventory | Route::EventFlags | Route::Regions)
        }

        pub fn display_name(&self) -> &'static str {
            match self {
                Route::None => "",
                Route::General => "General",
                Route::Stats => "Stats",
                Route::Equipment => "Equipment",
                Route::Inventory => "Inventory",
                Route::EventFlags => "Event Flags",
                Route::Regions => "Regions",
                Route::Spells => "Spells",
                Route::Npcs => "NPCs",
                Route::ShopItems => "Shop Items",
                Route::WorldPickups => "World Pickups",
                Route::EventFlagsDb => "Event Flags DB",
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
        if general.clicked() { app.current_route = Route::General; }
        if stats.clicked() { app.current_route = Route::Stats; }
        if equipment.clicked() { app.current_route = Route::Equipment; }
        if inventory.clicked() { app.current_route = Route::Inventory; }
        if event_flags.clicked() { app.current_route = Route::EventFlags; }
        if regions.clicked() { app.current_route = Route::Regions; }

        // Highlight active
        match app.current_route {
            Route::General => { general.highlight(); },
            Route::Stats => { stats.highlight(); },
            Route::Equipment => { equipment.highlight(); },
            Route::Inventory => { inventory.highlight(); },
            Route::EventFlags => { event_flags.highlight(); },
            Route::Regions => { regions.highlight(); },
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
        let event_flags_db = ui.add_sized([120., 40.], egui::Button::new("Event Flags DB"));

        // Listen for clicks
        if spells.clicked() { app.current_route = Route::Spells; }
        if npcs.clicked() { app.current_route = Route::Npcs; }
        if shop_items.clicked() { app.current_route = Route::ShopItems; }
        if world_pickups.clicked() { app.current_route = Route::WorldPickups; }
        if event_flags_db.clicked() { app.current_route = Route::EventFlagsDb; }

        // Highlight active
        match app.current_route {
            Route::Spells => { spells.highlight(); },
            Route::Npcs => { npcs.highlight(); },
            Route::ShopItems => { shop_items.highlight(); },
            Route::WorldPickups => { world_pickups.highlight(); },
            Route::EventFlagsDb => { event_flags_db.highlight(); },
            _ => {},
        }
    }

    /// Legacy menu function (kept for compatibility, now calls character_menu)
    pub fn menu(ui: &mut Ui, app: &mut App) {
        character_menu(ui, app);
    }

    /// Breadcrumb navigation bar (Row 2)
    /// Shows clickable navigation path: Characters > CharacterName > AreaName > SubrouteName
    pub fn breadcrumb_bar(ui: &mut Ui, app: &mut App) {
        use crate::vm::events::events_view_model::EventsRoute;

        // "Characters" is always the root
        if ui.selectable_label(false, egui::RichText::new("Characters").strong()).clicked() {
            app.current_route = Route::None;
        }

        // If a character is selected (either character view or we have a valid index)
        if app.vm.active.is_some_and(|v| v) && (app.current_route.is_character_view() || app.current_route.is_database_view()) {
            if app.current_route.is_character_view() {
                ui.label(">");
                let char_name = app.vm.slots[app.vm.index].general_vm.character_name.trim_matches('\0');
                if ui.selectable_label(false, char_name).clicked() {
                    app.current_route = Route::General;
                    // Reset EventFlags subroute when navigating back to character
                    app.vm.slots[app.vm.index].events_vm.current_route = EventsRoute::None;
                }

                // If we're in a specific area
                let area_name = app.current_route.display_name();
                if !area_name.is_empty() {
                    ui.label(">");

                    // If EventFlags, check for subroute
                    if matches!(app.current_route, Route::EventFlags) {
                        let events_route = &app.vm.slots[app.vm.index].events_vm.current_route;
                        let subroute_name = events_route.display_name();

                        if subroute_name.is_empty() {
                            // No subroute - area name is current (not clickable)
                            ui.label(egui::RichText::new(area_name).strong());
                        } else {
                            // Has subroute - area name is clickable
                            if ui.selectable_label(false, area_name).clicked() {
                                app.vm.slots[app.vm.index].events_vm.current_route = EventsRoute::None;
                            }
                            ui.label(">");
                            // Subroute name is current (not clickable)
                            ui.label(egui::RichText::new(subroute_name).strong());
                        }
                    } else {
                        // Not EventFlags - area name is current (not clickable)
                        ui.label(egui::RichText::new(area_name).strong());
                    }
                }
            } else if app.current_route.is_database_view() {
                // Database view
                ui.label(">");
                ui.label(egui::RichText::new(app.current_route.display_name()).strong());
            }
        }
    }

    /// Navigation buttons (Row 3)
    /// Shows appropriate buttons based on current navigation level
    pub fn navigation_buttons(ui: &mut Ui, app: &mut App) {
        // Determine current level
        // Level 1: Route::None or database view -> show characters + databases
        // Level 2: Character view but NOT EventFlags -> show area buttons
        // Level 3: EventFlags -> show EventFlags subroute buttons

        if matches!(app.current_route, Route::None) || app.current_route.is_database_view() {
            level1_navigation(ui, app);
        } else if app.current_route.is_character_view() {
            if matches!(app.current_route, Route::EventFlags) {
                level3_event_flags_navigation(ui, app);
            } else {
                level2_area_navigation(ui, app);
            }
        }
    }

    /// Level 1 navigation: Characters + Databases horizontal buttons
    fn level1_navigation(ui: &mut Ui, app: &mut App) {
        // Character buttons
        for i in 0..0xA {
            if app.vm.profile_summary[i].active {
                let char_name = app.vm.slots[i].general_vm.character_name.trim_matches('\0');
                let is_selected = app.vm.index == i && app.current_route.is_character_view();
                let button = ui.selectable_label(is_selected, char_name);
                if button.clicked() {
                    app.vm.index = i;
                    app.current_route = Route::General;
                }
            }
        }

        ui.add_space(20.0);

        // Database buttons
        let db_buttons = [
            ("Spells", Route::Spells),
            ("NPCs", Route::Npcs),
            ("Shop Items", Route::ShopItems),
            ("World Pickups", Route::WorldPickups),
            ("Event Flags DB", Route::EventFlagsDb),
        ];

        for (label, route) in db_buttons {
            let is_selected = std::mem::discriminant(&app.current_route) == std::mem::discriminant(&route);
            if ui.selectable_label(is_selected, label).clicked() {
                app.current_route = route;
            }
        }
    }

    /// Level 2 navigation: Area buttons (General, Stats, Equipment, etc.)
    fn level2_area_navigation(ui: &mut Ui, app: &mut App) {
        let area_buttons = [
            ("General", Route::General),
            ("Stats", Route::Stats),
            ("Equipment", Route::Equipment),
            ("Inventory", Route::Inventory),
            ("Event Flags", Route::EventFlags),
            ("Regions", Route::Regions),
        ];

        for (label, route) in area_buttons {
            let is_selected = std::mem::discriminant(&app.current_route) == std::mem::discriminant(&route);
            if ui.selectable_label(is_selected, label).clicked() {
                app.current_route = route;
            }
        }
    }

    /// Level 3 navigation: EventFlags subroute buttons
    fn level3_event_flags_navigation(ui: &mut Ui, app: &mut App) {
        use crate::vm::events::events_view_model::EventsRoute;

        let subroute_buttons = [
            ("Sites Of Grace", EventsRoute::SitesOfGrace),
            ("Whetblades", EventsRoute::Whetblades),
            ("Cookbooks", EventsRoute::Cookboks),
            ("Maps", EventsRoute::Maps),
            ("Bosses", EventsRoute::Bosses),
            ("Summoning Pools", EventsRoute::SummoningPools),
            ("Colosseums", EventsRoute::Colosseums),
            ("Landmarks", EventsRoute::Landmarks),
            ("World Pickups", EventsRoute::WorldPickups),
            ("Dungeon Pickups", EventsRoute::DungeonPickups),
            ("Verification", EventsRoute::Verification),
        ];

        // Clone the current route to avoid borrow issues
        let current_subroute = app.vm.slots[app.vm.index].events_vm.current_route.clone();

        for (label, route) in subroute_buttons {
            let is_selected = std::mem::discriminant(&current_subroute) == std::mem::discriminant(&route);
            if ui.selectable_label(is_selected, label).clicked() {
                app.vm.slots[app.vm.index].events_vm.current_route = route;
            }
        }
    }
}
