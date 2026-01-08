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
    }

    impl Route {
        pub fn is_database_view(&self) -> bool {
            matches!(self, Route::Spells | Route::Npcs | Route::ShopItems | Route::WorldPickups)
        }

        pub fn is_character_view(&self) -> bool {
            matches!(self, Route::General | Route::Stats | Route::Equipment | Route::Inventory | Route::EventFlags | Route::Regions)
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

        // Listen for clicks
        if spells.clicked() { app.current_route = Route::Spells; }
        if npcs.clicked() { app.current_route = Route::Npcs; }
        if shop_items.clicked() { app.current_route = Route::ShopItems; }
        if world_pickups.clicked() { app.current_route = Route::WorldPickups; }

        // Highlight active
        match app.current_route {
            Route::Spells => { spells.highlight(); },
            Route::Npcs => { npcs.highlight(); },
            Route::ShopItems => { shop_items.highlight(); },
            Route::WorldPickups => { world_pickups.highlight(); },
            _ => {},
        }
    }

    /// Legacy menu function (kept for compatibility, now calls character_menu)
    pub fn menu(ui: &mut Ui, app: &mut App) {
        character_menu(ui, app);
    }
}
