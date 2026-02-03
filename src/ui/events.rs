pub mod events {

    use eframe::egui::{self, Ui, Color32, RichText};
    use crate::{db::{bosses::bosses::BOSSES, colosseums::colosseums::COLOSSEUMS, cookbooks::books::COOKBOKS, graces::maps::GRACES, landmarks::landmarks::LANDMARKS, map_name::map_name::MAP_NAME, maps::maps::MAPS, summoning_pools::summoning_pools::SUMMONING_POOLS, whetblades::whetblades::WHETBLADES, pickup_data::{WORLD_PICKUPS, PickupCategory}, pickup_flags::{is_flag_set_with_status, get_flag_verification_status, DUNGEON_PICKUP_BASES}, dungeon_pickups::{DUNGEON_PICKUPS, get_dungeon_area_name}, item_name::item_name::ITEM_NAME, weapon_name::weapon_name::WEAPON_NAME, armor_name::armor_name::ARMOR_NAME, accessory_name::accessory_name::ACCESSORY_NAME, aow_name::aow_name::AOW_NAME}, ui::{verification_view::verification_view::{verification_view, inventory_verification_summary}, style::TABLE_MONO_SIZE}, vm::{events::events_view_model::{EventsRoute, PickupTypeFilter, CollectedFilter, GraceStatus}, vm::vm::ViewModel}};
    use crate::save::common::save_slot::EquipInventoryData;

    pub fn events(ui: &mut Ui, vm: &mut ViewModel, event_flags: Option<&[u8]>, inventory: Option<&EquipInventoryData>, storage: Option<&EquipInventoryData>, save_path: &str) {
        egui::SidePanel::left("inventory_menu").show(ui.ctx(), |ui|{
            egui::ScrollArea::vertical()
            .id_salt("left")
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    let sites_of_grace = ui.add_sized([100., 40.], egui::Button::new("Sites Of Grace"));
                    let whetblades = ui.add_sized([100., 40.], egui::Button::new("Whetblades"));
                    let cookboks = ui.add_sized([100., 40.], egui::Button::new("Cookbooks"));
                    let maps = ui.add_sized([100., 40.], egui::Button::new("Maps"));
                    let bosses = ui.add_sized([100., 40.], egui::Button::new("Bosses"));
                    let summoning_pools = ui.add_sized([100., 60.], egui::Button::new("Summoning\nPools"));
                    let colosseums = ui.add_sized([100., 40.], egui::Button::new("Colosseums"));
                    let landmarks = ui.add_sized([100., 40.], egui::Button::new("Landmarks"));
                    let world_pickups = ui.add_sized([100., 40.], egui::Button::new("World Pickups"));
                    let dungeon_pickups = ui.add_sized([100., 60.], egui::Button::new("Dungeon\nPickups"));
                    ui.separator();
                    let verification = ui.add_sized([100., 40.], egui::Button::new("Verification"));

                    if sites_of_grace.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::SitesOfGrace}
                    if whetblades.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::Whetblades}
                    if cookboks.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::Cookboks}
                    if maps.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::Maps}
                    if bosses.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::Bosses}
                    if summoning_pools.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::SummoningPools}
                    if colosseums.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::Colosseums}
                    if landmarks.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::Landmarks}
                    if world_pickups.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::WorldPickups}
                    if dungeon_pickups.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::DungeonPickups}
                    if verification.clicked() {vm.slots[vm.index].events_vm.current_route = EventsRoute::Verification}

                    // Highlight active
                    match vm.slots[vm.index].events_vm.current_route {
                        EventsRoute::None => {},
                        EventsRoute::SitesOfGrace => {sites_of_grace.highlight();},
                        EventsRoute::Whetblades => {whetblades.highlight();},
                        EventsRoute::Cookboks => {cookboks.highlight();},
                        EventsRoute::Maps => {maps.highlight();},
                        EventsRoute::Bosses => {bosses.highlight();},
                        EventsRoute::SummoningPools => {summoning_pools.highlight();},
                        EventsRoute::Colosseums => {colosseums.highlight();},
                        EventsRoute::Landmarks => {landmarks.highlight();},
                        EventsRoute::WorldPickups => {world_pickups.highlight();},
                        EventsRoute::DungeonPickups => {dungeon_pickups.highlight();},
                        EventsRoute::Verification => {verification.highlight();},
                    }
                })
            });
        });

        // Right sidebar for flag details (only show when a flag is selected in world/dungeon pickups)
        let selected_flag = match vm.slots[vm.index].events_vm.current_route {
            EventsRoute::WorldPickups => vm.slots[vm.index].events_vm.world_pickups_filter.selected_flag_id,
            EventsRoute::DungeonPickups => vm.slots[vm.index].events_vm.dungeon_pickups_filter.selected_flag_id,
            _ => None,
        };

        if selected_flag.is_some() {
            egui::SidePanel::right("flag_details_panel")
                .default_width(280.0)
                .min_width(200.0)
                .show(ui.ctx(), |ui| {
                    flag_details_sidebar(ui, vm, event_flags, inventory, storage, save_path);
                });
        }

        egui::CentralPanel::default().show(ui.ctx(), |ui|{
            egui::ScrollArea::vertical()
            .id_salt("left")
            .auto_shrink(false)
            .show(ui, |ui| {
                match vm.slots[vm.index].events_vm.current_route {
                    EventsRoute::None => {},
                    EventsRoute::SitesOfGrace => {graces(ui, vm);},
                    EventsRoute::Whetblades => {whetblades(ui, vm);},
                    EventsRoute::Cookboks => {cookbooks(ui, vm);},
                    EventsRoute::Maps => {maps(ui, vm);},
                    EventsRoute::Bosses => {bosses(ui, vm);},
                    EventsRoute::SummoningPools => {summoning_pools(ui, vm);},
                    EventsRoute::Colosseums => {colosseums(ui, vm);},
                    EventsRoute::Landmarks => {landmarks_view(ui, vm);},
                    EventsRoute::WorldPickups => {world_pickups(ui, vm, event_flags);},
                    EventsRoute::DungeonPickups => {dungeon_pickups(ui, vm, event_flags);},
                    EventsRoute::Verification => {
                        // Inventory Verification Triangle section first
                        if inventory.is_some() || event_flags.is_some() {
                            let set_flags = collect_set_flags(event_flags);
                            inventory_verification_summary(ui, &set_flags, inventory);
                            ui.separator();
                            ui.add_space(10.0);
                        }

                        // Existing flag verification view
                        verification_view(ui, &mut vm.slots[vm.index].events_vm.verification_vm);
                    },
                }
            });
        });

    }

    fn graces(ui: &mut Ui, vm: &mut ViewModel) {
        let graces_data = &vm.slots[vm.index].events_vm.graces;
        let grace_groups = &vm.slots[vm.index].events_vm.grace_groups;

        // Count discovered (only from reliable blocks)
        let discovered_count = graces_data.values().filter(|v| v.is_discovered()).count();
        let unreliable_count = graces_data.values().filter(|v| v.is_unreliable()).count();
        let total_count = graces_data.len();

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Name | Region | Flag ID").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        if unreliable_count > 0 {
            ui.label(RichText::new(format!("? = unreliable block ({} graces) - result may be inaccurate", unreliable_count))
                .color(Color32::from_rgb(255, 200, 100)).small());
        }
        ui.separator();

        // Summary - show reliable discovered vs total reliable
        let reliable_total = total_count - unreliable_count;
        let summary = if unreliable_count > 0 {
            format!("Sites of Grace: {}/{} discovered ({} unreliable)", discovered_count, reliable_total, unreliable_count)
        } else {
            format!("Sites of Grace: {}/{} discovered", discovered_count, total_count)
        };
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        let graces_lookup = GRACES.lock().unwrap();

        // Group by region
        for (map_id, grace_ids) in grace_groups {
            let region_name = MAP_NAME.lock().unwrap().get(map_id).cloned().unwrap_or("Unknown");
            let region_discovered = grace_ids.iter()
                .filter(|g| graces_data.get(g).map(|s| s.is_discovered()).unwrap_or(false))
                .count();
            let region_unreliable = grace_ids.iter()
                .filter(|g| graces_data.get(g).map(|s| s.is_unreliable()).unwrap_or(false))
                .count();

            let region_header = if region_unreliable > 0 {
                format!("{} ({}/{}, {} unreliable)", region_name, region_discovered, grace_ids.len() - region_unreliable, region_unreliable)
            } else {
                format!("{} ({}/{})", region_name, region_discovered, grace_ids.len())
            };
            ui.label(RichText::new(region_header).strong());

            for grace_id in grace_ids {
                if let Some(grace_info) = graces_lookup.get(grace_id) {
                    let grace_status = graces_data.get(grace_id).copied().unwrap_or(GraceStatus::NotDiscovered);
                    let flag_id = grace_info.1;
                    let name = grace_info.2;

                    let (status_text, text_color) = match grace_status {
                        GraceStatus::Discovered => ("[X]", Color32::from_rgb(100, 200, 100)),
                        GraceStatus::NotDiscovered => ("[ ]", Color32::LIGHT_GRAY),
                        GraceStatus::Unreliable => ("[?]", Color32::from_rgb(255, 200, 100)),
                    };

                    let row_text = format!("{} | {} | {} | {}", status_text, name, region_name, flag_id);

                    let response = ui.add(
                        egui::Label::new(RichText::new(&row_text).color(text_color).monospace().size(TABLE_MONO_SIZE))
                            .sense(egui::Sense::click())
                    );

                    if response.double_clicked() {
                        ui.output_mut(|o| o.copied_text = row_text.clone());
                    }

                    response.context_menu(|ui| {
                        if ui.button("Copy row").clicked() {
                            ui.output_mut(|o| o.copied_text = row_text.clone());
                            ui.close_menu();
                        }
                        if ui.button("Copy name").clicked() {
                            ui.output_mut(|o| o.copied_text = name.to_string());
                            ui.close_menu();
                        }
                        if ui.button("Copy flag ID").clicked() {
                            ui.output_mut(|o| o.copied_text = flag_id.to_string());
                            ui.close_menu();
                        }
                    });
                }
            }
            ui.separator();
        }
    }

    fn whetblades(ui: &mut Ui, vm: &mut ViewModel) {
        let whetblades_data = &vm.slots[vm.index].events_vm.whetblades;

        let discovered_count = whetblades_data.values().filter(|v| **v).count();
        let total_count = whetblades_data.len();

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        let summary = format!("Whetblades: {}/{} discovered", discovered_count, total_count);
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        let whetblades_lookup = WHETBLADES.lock().unwrap();

        for (whetblade, discovered) in whetblades_data {
            if let Some(info) = whetblades_lookup.get(whetblade) {
                let status = if *discovered { "[X]" } else { "[ ]" };
                let row_text = format!("{} | {} | {}", status, info.1, info.0);
                display_event_row(ui, &row_text, info.1, info.0, *discovered);
            }
        }
    }

    fn cookbooks(ui: &mut Ui, vm: &mut ViewModel) {
        let cookbooks_data = &vm.slots[vm.index].events_vm.cookbooks;

        let discovered_count = cookbooks_data.values().filter(|v| **v).count();
        let total_count = cookbooks_data.len();

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        let summary = format!("Cookbooks: {}/{} discovered", discovered_count, total_count);
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        let cookbooks_lookup = COOKBOKS.lock().unwrap();

        for (cookbook, discovered) in cookbooks_data {
            if let Some(info) = cookbooks_lookup.get(cookbook) {
                let status = if *discovered { "[X]" } else { "[ ]" };
                let row_text = format!("{} | {} | {}", status, info.1, info.0);
                display_event_row(ui, &row_text, info.1, info.0, *discovered);
            }
        }
    }

    fn maps(ui: &mut Ui, vm: &mut ViewModel) {
        let maps_data = &vm.slots[vm.index].events_vm.maps;

        let discovered_count = maps_data.values().filter(|v| **v).count();
        let total_count = maps_data.len();

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        let summary = format!("Maps: {}/{} discovered", discovered_count, total_count);
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        let maps_lookup = MAPS.lock().unwrap();

        for (map, discovered) in maps_data {
            if let Some(info) = maps_lookup.get(map) {
                let status = if *discovered { "[X]" } else { "[ ]" };
                let row_text = format!("{} | {} | {}", status, info.1, info.0);
                display_event_row(ui, &row_text, info.1, info.0, *discovered);
            }
        }
    }

    fn bosses(ui: &mut Ui, vm: &mut ViewModel) {
        let bosses_data = &vm.slots[vm.index].events_vm.bosses;

        let defeated_count = bosses_data.values().filter(|v| **v).count();
        let total_count = bosses_data.len();

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        let summary = format!("Bosses: {}/{} defeated", defeated_count, total_count);
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        let bosses_lookup = BOSSES.lock().unwrap();

        for (boss, defeated) in bosses_data {
            if let Some(info) = bosses_lookup.get(boss) {
                let status = if *defeated { "[X]" } else { "[ ]" };
                let row_text = format!("{} | {} | {}", status, info.1, info.0);
                display_event_row(ui, &row_text, info.1, info.0, *defeated);
            }
        }
    }

    fn summoning_pools(ui: &mut Ui, vm: &mut ViewModel) {
        let pools_data = &vm.slots[vm.index].events_vm.summoning_pools;

        let discovered_count = pools_data.values().filter(|v| **v).count();
        let total_count = pools_data.len();

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        let summary = format!("Summoning Pools: {}/{} discovered", discovered_count, total_count);
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        let pools_lookup = SUMMONING_POOLS.lock().unwrap();

        for (pool, discovered) in pools_data {
            if let Some(info) = pools_lookup.get(pool) {
                let status = if *discovered { "[X]" } else { "[ ]" };
                let row_text = format!("{} | {} | {}", status, info.1, info.0);
                display_event_row(ui, &row_text, info.1, info.0, *discovered);
            }
        }
    }

    fn colosseums(ui: &mut Ui, vm: &mut ViewModel) {
        let colosseums_data = &vm.slots[vm.index].events_vm.colosseums;

        let discovered_count = colosseums_data.values().filter(|v| **v).count();
        let total_count = colosseums_data.len();

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        let summary = format!("Colosseums: {}/{} discovered", discovered_count, total_count);
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        let colosseums_lookup = COLOSSEUMS.lock().unwrap();

        for (colosseum, discovered) in colosseums_data {
            if let Some(info) = colosseums_lookup.get(colosseum) {
                let status = if *discovered { "[X]" } else { "[ ]" };
                let row_text = format!("{} | {} | {}", status, info.1, info.0);
                display_event_row(ui, &row_text, info.1, info.0, *discovered);
            }
        }
    }

    fn landmarks_view(ui: &mut Ui, vm: &mut ViewModel) {
        let landmarks_data = &vm.slots[vm.index].events_vm.landmarks;

        let discovered_count = landmarks_data.values().filter(|v| **v).count();
        let total_count = landmarks_data.len();

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        let summary = format!("Landmarks: {}/{} discovered", discovered_count, total_count);
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        let landmarks_lookup = LANDMARKS.lock().unwrap();

        for (landmark, discovered) in landmarks_data {
            if let Some(info) = landmarks_lookup.get(landmark) {
                let status = if *discovered { "[X]" } else { "[ ]" };
                let row_text = format!("{} | {} | {}", status, info.1, info.0);
                display_event_row(ui, &row_text, info.1, info.0, *discovered);
            }
        }
    }

    fn display_event_row(ui: &mut Ui, row_text: &str, name: &str, flag_id: u32, discovered: bool) {
        let verification_status = get_flag_verification_status(flag_id);
        let is_unverified = verification_status.is_uncertain();

        // Add "!" indicator for unverified flags - insert after the status brackets [X] or [ ]
        let display_text = if is_unverified {
            // Insert "!" after the first 3 characters (the [X] or [ ] part)
            let (status_part, rest) = row_text.split_at(3.min(row_text.len()));
            format!("{}!{}", status_part, rest)
        } else {
            row_text.to_string()
        };

        let text_color = if is_unverified {
            Color32::from_rgb(255, 200, 100) // Orange/yellow for unverified
        } else if discovered {
            Color32::from_rgb(100, 200, 100)
        } else {
            Color32::LIGHT_GRAY
        };

        let response = ui.add(
            egui::Label::new(RichText::new(&display_text).color(text_color).monospace().size(TABLE_MONO_SIZE))
                .sense(egui::Sense::click())
        );

        if response.double_clicked() {
            ui.output_mut(|o| o.copied_text = display_text.clone());
        }

        response.context_menu(|ui| {
            if ui.button("Copy row").clicked() {
                ui.output_mut(|o| o.copied_text = display_text.clone());
                ui.close_menu();
            }
            if ui.button("Copy name").clicked() {
                ui.output_mut(|o| o.copied_text = name.to_string());
                ui.close_menu();
            }
            if ui.button("Copy flag ID").clicked() {
                ui.output_mut(|o| o.copied_text = flag_id.to_string());
                ui.close_menu();
            }
        });
    }

    fn world_pickups(ui: &mut Ui, vm: &mut ViewModel, event_flags: Option<&[u8]>) {
        let filter = &mut vm.slots[vm.index].events_vm.world_pickups_filter;

        // Type filter row - using scroll area for many options
        ui.horizontal(|ui| {
            ui.label(RichText::new("Type:").color(Color32::LIGHT_GRAY));
            egui::ComboBox::from_id_salt("char_pickup_type_filter")
                .selected_text(match filter.type_filter {
                    PickupTypeFilter::All => "All",
                    PickupTypeFilter::GoldenRunes => "Golden Runes",
                    PickupTypeFilter::SmithingStones => "Smithing Stones",
                    PickupTypeFilter::SomberStones => "Somber Stones",
                    PickupTypeFilter::Glovewort => "Glovewort",
                    PickupTypeFilter::Weapons => "Weapons",
                    PickupTypeFilter::Armor => "Armor",
                    PickupTypeFilter::Talismans => "Talismans",
                    PickupTypeFilter::AshesOfWar => "Ashes of War",
                    PickupTypeFilter::KeyItems => "Key Items",
                    PickupTypeFilter::CraftingMaterials => "Crafting",
                    PickupTypeFilter::Consumables => "Consumables",
                    PickupTypeFilter::Other => "Other",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::All, "All");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::GoldenRunes, "Golden Runes");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::SmithingStones, "Smithing Stones");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::SomberStones, "Somber Stones");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Glovewort, "Glovewort");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Weapons, "Weapons");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Armor, "Armor");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Talismans, "Talismans");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::AshesOfWar, "Ashes of War");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::KeyItems, "Key Items");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::CraftingMaterials, "Crafting Materials");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Consumables, "Consumables");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Other, "Other");
                });
        });

        // Region filter and search row
        ui.horizontal(|ui| {
            ui.label(RichText::new("Region:").color(Color32::LIGHT_GRAY));

            // Get unique regions from new data format
            let mut regions: Vec<&str> = WORLD_PICKUPS.iter()
                .map(|p| p.region)
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            regions.sort();
            regions.insert(0, "All");

            egui::ComboBox::from_id_salt("char_world_pickups_region_filter")
                .selected_text(&filter.region_filter)
                .show_ui(ui, |ui| {
                    for region in &regions {
                        ui.selectable_value(&mut filter.region_filter, region.to_string(), *region);
                    }
                });

            ui.separator();
            ui.label(RichText::new("Search:").color(Color32::LIGHT_GRAY));
            ui.text_edit_singleline(&mut filter.search);
        });

        // Collected filter row
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::All, "All");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::Collected, "Collected");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::NotCollected, "Not Collected");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::Unverified, "Unverified");
        });
        ui.separator();

        // Get current filter values (to avoid borrow issues)
        let type_filter = filter.type_filter;
        let collected_filter = filter.collected_filter;
        let region_filter = filter.region_filter.clone();
        let search = filter.search.clone();
        let search_lower = search.to_lowercase();

        // Count collected/total using new formula-based flag checking
        let mut collected = 0;
        let mut total = 0;
        let mut filtered_total = 0;
        let mut filtered_collected = 0;
        let mut unverified_count = 0;

        let ef = event_flags.unwrap_or(&[]);

        for pickup in WORLD_PICKUPS.iter() {
            let (is_collected, status) = is_flag_set_with_status(ef, pickup.event_flag);
            if is_collected {
                collected += 1;
            }
            if status.is_uncertain() {
                unverified_count += 1;
            }
            total += 1;

            // Check if passes filters
            if passes_pickup_filters(pickup, is_collected, status, type_filter, collected_filter, &region_filter, &search_lower) {
                filtered_total += 1;
                if is_collected {
                    filtered_collected += 1;
                }
            }
        }

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Lot ID | Flag ID | Item | Category | Qty | Region").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        if unverified_count > 0 {
            ui.label(RichText::new("! = unverified formula (result may be inaccurate)").color(Color32::from_rgb(255, 200, 100)).small());
        }
        ui.separator();

        // Summary
        let summary = if filtered_total == total {
            if unverified_count > 0 {
                format!("World Pickups: {}/{} collected ({} unverified)", collected, total, unverified_count)
            } else {
                format!("World Pickups: {}/{} collected", collected, total)
            }
        } else {
            format!("World Pickups: {}/{} collected (showing {}/{})", collected, total, filtered_collected, filtered_total)
        };
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        // Group by region
        let mut regions_map: std::collections::BTreeMap<&str, Vec<_>> = std::collections::BTreeMap::new();
        for pickup in WORLD_PICKUPS.iter() {
            regions_map.entry(pickup.region).or_default().push(pickup);
        }

        for (region, pickups) in regions_map {
            // Count collected and filtered in this region
            let mut region_collected = 0;
            let mut region_filtered = 0;

            for pickup in &pickups {
                let (is_collected, status) = is_flag_set_with_status(ef, pickup.event_flag);
                if is_collected {
                    region_collected += 1;
                }
                if passes_pickup_filters(pickup, is_collected, status, type_filter, collected_filter, &region_filter, &search_lower) {
                    region_filtered += 1;
                }
            }

            // Skip region if no items pass filter
            if region_filtered == 0 {
                continue;
            }

            let region_header = if region_filtered == pickups.len() {
                format!("{} ({}/{})", region, region_collected, pickups.len())
            } else {
                format!("{} ({}/{} collected, showing {})", region, region_collected, pickups.len(), region_filtered)
            };
            ui.label(RichText::new(region_header).strong());

            for pickup in &pickups {
                let (is_collected, verification_status) = is_flag_set_with_status(ef, pickup.event_flag);

                // Apply filters
                if !passes_pickup_filters(pickup, is_collected, verification_status, type_filter, collected_filter, &region_filter, &search_lower) {
                    continue;
                }

                // Add "!" indicator for unverified flags
                let unverified_marker = if verification_status.is_uncertain() { "!" } else { "" };
                let status = if is_collected { "[X]" } else { "[ ]" };

                let row_text = format!(
                    "{}{} | {} | {} | {} | {} | {} | {}",
                    status, unverified_marker, pickup.item_lot_id, pickup.event_flag, pickup.name,
                    pickup.category.display_name(), pickup.quantity, region
                );

                // Check if this row is selected
                let is_selected = vm.slots[vm.index].events_vm.world_pickups_filter.selected_flag_id == Some(pickup.event_flag);

                let text_color = if is_selected {
                    Color32::YELLOW // Highlight selected row
                } else if verification_status.is_uncertain() {
                    Color32::from_rgb(255, 200, 100) // Orange/yellow for unverified
                } else if is_collected {
                    Color32::from_rgb(100, 200, 100)
                } else {
                    Color32::LIGHT_GRAY
                };

                let response = ui.add(
                    egui::Label::new(RichText::new(&row_text).color(text_color).monospace().size(TABLE_MONO_SIZE))
                        .sense(egui::Sense::click())
                );

                // Single click selects for details panel
                if response.clicked() {
                    vm.slots[vm.index].events_vm.world_pickups_filter.selected_flag_id = Some(pickup.event_flag);
                }

                if response.double_clicked() {
                    ui.output_mut(|o| o.copied_text = row_text.clone());
                }

                response.context_menu(|ui| {
                    if ui.button("Copy row").clicked() {
                        ui.output_mut(|o| o.copied_text = row_text.clone());
                        ui.close_menu();
                    }
                    if ui.button("Copy item name").clicked() {
                        ui.output_mut(|o| o.copied_text = pickup.name.to_string());
                        ui.close_menu();
                    }
                    if ui.button("Copy flag ID").clicked() {
                        ui.output_mut(|o| o.copied_text = pickup.event_flag.to_string());
                        ui.close_menu();
                    }
                });
            }

            ui.separator();
        }
    }

    fn dungeon_pickups(ui: &mut Ui, vm: &mut ViewModel, event_flags: Option<&[u8]>) {
        use crate::db::dungeon_pickups::DungeonPickup;
        use crate::db::pickup_flags::DUNGEON_SECTION_SIZE;
        use crate::util::bit::bit::get_bit;

        let filter = &mut vm.slots[vm.index].events_vm.dungeon_pickups_filter;

        // Type filter row
        ui.horizontal(|ui| {
            ui.label(RichText::new("Type:").color(Color32::LIGHT_GRAY));
            egui::ComboBox::from_id_salt("dungeon_pickup_type_filter")
                .selected_text(match filter.type_filter {
                    PickupTypeFilter::All => "All",
                    PickupTypeFilter::GoldenRunes => "Golden Runes",
                    PickupTypeFilter::SmithingStones => "Smithing Stones",
                    PickupTypeFilter::SomberStones => "Somber Stones",
                    PickupTypeFilter::Glovewort => "Glovewort",
                    PickupTypeFilter::Weapons => "Weapons",
                    PickupTypeFilter::Armor => "Armor",
                    PickupTypeFilter::Talismans => "Talismans",
                    PickupTypeFilter::AshesOfWar => "Ashes of War",
                    PickupTypeFilter::KeyItems => "Key Items",
                    PickupTypeFilter::CraftingMaterials => "Crafting",
                    PickupTypeFilter::Consumables => "Consumables",
                    PickupTypeFilter::Other => "Other",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::All, "All");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::GoldenRunes, "Golden Runes");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::SmithingStones, "Smithing Stones");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::SomberStones, "Somber Stones");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Glovewort, "Glovewort");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Weapons, "Weapons");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Armor, "Armor");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Talismans, "Talismans");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::AshesOfWar, "Ashes of War");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::KeyItems, "Key Items");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::CraftingMaterials, "Crafting Materials");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Consumables, "Consumables");
                    ui.selectable_value(&mut filter.type_filter, PickupTypeFilter::Other, "Other");
                });
        });

        // Dungeon filter and search row
        ui.horizontal(|ui| {
            ui.label(RichText::new("Dungeon:").color(Color32::LIGHT_GRAY));

            // Get unique dungeons from data
            let mut dungeons: Vec<&str> = DUNGEON_PICKUPS.iter()
                .map(|p| get_dungeon_area_name(p.dungeon_area))
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            dungeons.sort();
            dungeons.insert(0, "All");

            egui::ComboBox::from_id_salt("dungeon_pickups_dungeon_filter")
                .selected_text(&filter.dungeon_filter)
                .show_ui(ui, |ui| {
                    for dungeon in &dungeons {
                        ui.selectable_value(&mut filter.dungeon_filter, dungeon.to_string(), *dungeon);
                    }
                });

            ui.separator();
            ui.label(RichText::new("Search:").color(Color32::LIGHT_GRAY));
            ui.text_edit_singleline(&mut filter.search);
        });

        // Collected filter row
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::All, "All");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::Collected, "Collected");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::NotCollected, "Not Collected");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::Unverified, "Unverified");
        });
        ui.separator();

        // Get current filter values (to avoid borrow issues)
        let type_filter = filter.type_filter;
        let collected_filter = filter.collected_filter;
        let dungeon_filter = filter.dungeon_filter.clone();
        let search = filter.search.clone();
        let search_lower = search.to_lowercase();

        // Count collected/total
        let mut collected = 0;
        let mut total = 0;
        let mut filtered_total = 0;
        let mut filtered_collected = 0;
        let mut unverified_count = 0;

        let ef = event_flags.unwrap_or(&[]);

        // Helper to check if dungeon pickup flag is set
        fn is_dungeon_pickup_collected(ef: &[u8], pickup: &DungeonPickup) -> (bool, bool) {
            // Check if base is verified
            let base = DUNGEON_PICKUP_BASES.get(&pickup.dungeon_area);
            if base.is_none() {
                return (false, false); // Unverified
            }
            let base = *base.unwrap();

            let byte_offset = base + pickup.section * DUNGEON_SECTION_SIZE + pickup.event_flag % 10000 / 8;
            let bit_pos = 7 - (pickup.event_flag % 8);

            if byte_offset as usize >= ef.len() {
                return (false, false);
            }

            let is_set = get_bit(ef[byte_offset as usize], bit_pos as u8);
            (is_set, true) // true = verified base
        }

        for pickup in DUNGEON_PICKUPS.iter() {
            let (is_collected, is_verified) = is_dungeon_pickup_collected(ef, pickup);
            if is_collected {
                collected += 1;
            }
            if !is_verified {
                unverified_count += 1;
            }
            total += 1;

            // Check if passes filters
            if passes_dungeon_pickup_filters(pickup, is_collected, is_verified, type_filter, collected_filter, &dungeon_filter, &search_lower) {
                filtered_total += 1;
                if is_collected {
                    filtered_collected += 1;
                }
            }
        }

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Flag ID | Item | Category | Qty | Dungeon").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        if unverified_count > 0 {
            ui.label(RichText::new("! = unverified base (result may be inaccurate)").color(Color32::from_rgb(255, 200, 100)).small());
        }
        ui.separator();

        // Summary
        let summary = if filtered_total == total {
            if unverified_count > 0 {
                format!("Dungeon Pickups: {}/{} collected ({} unverified)", collected, total, unverified_count)
            } else {
                format!("Dungeon Pickups: {}/{} collected", collected, total)
            }
        } else {
            format!("Dungeon Pickups: {}/{} collected (showing {}/{})", collected, total, filtered_collected, filtered_total)
        };
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        // Group by dungeon area
        let mut dungeons_map: std::collections::BTreeMap<u32, Vec<_>> = std::collections::BTreeMap::new();
        for pickup in DUNGEON_PICKUPS.iter() {
            dungeons_map.entry(pickup.dungeon_area).or_default().push(pickup);
        }

        for (area, pickups) in dungeons_map {
            let area_name = get_dungeon_area_name(area);

            // Count collected and filtered in this area
            let mut area_collected = 0;
            let mut area_filtered = 0;

            for pickup in &pickups {
                let (is_collected, is_verified) = is_dungeon_pickup_collected(ef, pickup);
                if is_collected {
                    area_collected += 1;
                }
                if passes_dungeon_pickup_filters(pickup, is_collected, is_verified, type_filter, collected_filter, &dungeon_filter, &search_lower) {
                    area_filtered += 1;
                }
            }

            // Skip area if no items pass filter
            if area_filtered == 0 {
                continue;
            }

            let area_header = if area_filtered == pickups.len() {
                format!("{} ({}/{})", area_name, area_collected, pickups.len())
            } else {
                format!("{} ({}/{} collected, showing {})", area_name, area_collected, pickups.len(), area_filtered)
            };
            ui.label(RichText::new(area_header).strong());

            for pickup in &pickups {
                let (is_collected, is_verified) = is_dungeon_pickup_collected(ef, pickup);

                // Apply filters
                if !passes_dungeon_pickup_filters(pickup, is_collected, is_verified, type_filter, collected_filter, &dungeon_filter, &search_lower) {
                    continue;
                }

                // Add "!" indicator for unverified bases
                let unverified_marker = if !is_verified { "!" } else { "" };
                let status = if is_collected { "[X]" } else { "[ ]" };

                let row_text = format!(
                    "{}{} | {} | {} | {} | {} | {}",
                    status, unverified_marker, pickup.event_flag, pickup.name,
                    pickup.category.display_name(), pickup.quantity, area_name
                );

                // Check if this row is selected
                let is_selected = vm.slots[vm.index].events_vm.dungeon_pickups_filter.selected_flag_id == Some(pickup.event_flag);

                let text_color = if is_selected {
                    Color32::YELLOW // Highlight selected row
                } else if !is_verified {
                    Color32::from_rgb(255, 200, 100) // Orange/yellow for unverified
                } else if is_collected {
                    Color32::from_rgb(100, 200, 100)
                } else {
                    Color32::LIGHT_GRAY
                };

                let response = ui.add(
                    egui::Label::new(RichText::new(&row_text).color(text_color).monospace().size(TABLE_MONO_SIZE))
                        .sense(egui::Sense::click())
                );

                // Single click selects for details panel
                if response.clicked() {
                    vm.slots[vm.index].events_vm.dungeon_pickups_filter.selected_flag_id = Some(pickup.event_flag);
                }

                if response.double_clicked() {
                    ui.output_mut(|o| o.copied_text = row_text.clone());
                }

                response.context_menu(|ui| {
                    if ui.button("Copy row").clicked() {
                        ui.output_mut(|o| o.copied_text = row_text.clone());
                        ui.close_menu();
                    }
                    if ui.button(format!("Copy flag ID: {}", pickup.event_flag)).clicked() {
                        ui.output_mut(|o| o.copied_text = pickup.event_flag.to_string());
                        ui.close_menu();
                    }
                });
            }

            ui.separator();
        }
    }

    fn passes_dungeon_pickup_filters(
        pickup: &crate::db::dungeon_pickups::DungeonPickup,
        is_collected: bool,
        is_verified: bool,
        type_filter: PickupTypeFilter,
        collected_filter: CollectedFilter,
        dungeon_filter: &str,
        search_lower: &str,
    ) -> bool {
        // Apply collected filter
        match collected_filter {
            CollectedFilter::All => {},
            CollectedFilter::Collected => {
                if !is_collected {
                    return false;
                }
            },
            CollectedFilter::NotCollected => {
                if is_collected {
                    return false;
                }
            },
            CollectedFilter::Unverified => {
                // Show only items with unverified bases
                if is_verified {
                    return false;
                }
            },
        }

        // Apply type filter
        let type_match = match type_filter {
            PickupTypeFilter::All => true,
            PickupTypeFilter::GoldenRunes => pickup.category == PickupCategory::GoldenRunes,
            PickupTypeFilter::SmithingStones => pickup.category == PickupCategory::SmithingStones,
            PickupTypeFilter::SomberStones => pickup.category == PickupCategory::SomberStones,
            PickupTypeFilter::Glovewort => pickup.category == PickupCategory::Glovewort,
            PickupTypeFilter::Weapons => pickup.category == PickupCategory::Weapons,
            PickupTypeFilter::Armor => pickup.category == PickupCategory::Armor,
            PickupTypeFilter::Talismans => pickup.category == PickupCategory::Talismans,
            PickupTypeFilter::AshesOfWar => pickup.category == PickupCategory::AshesOfWar,
            PickupTypeFilter::KeyItems => pickup.category == PickupCategory::KeyItems,
            PickupTypeFilter::CraftingMaterials => pickup.category == PickupCategory::CraftingMaterials,
            PickupTypeFilter::Consumables => pickup.category == PickupCategory::Consumables,
            PickupTypeFilter::Other => pickup.category == PickupCategory::Other,
        };

        if !type_match {
            return false;
        }

        // Apply dungeon filter
        let dungeon_name = get_dungeon_area_name(pickup.dungeon_area);
        if dungeon_filter != "All" && dungeon_name != dungeon_filter {
            return false;
        }

        // Apply search
        if !search_lower.is_empty() {
            let matches = pickup.name.to_lowercase().contains(search_lower)
                || dungeon_name.to_lowercase().contains(search_lower);
            if !matches {
                return false;
            }
        }

        true
    }

    fn passes_pickup_filters(
        pickup: &crate::db::pickup_data::WorldPickup,
        is_collected: bool,
        verification_status: crate::db::pickup_flags::VerificationStatus,
        type_filter: PickupTypeFilter,
        collected_filter: CollectedFilter,
        region_filter: &str,
        search_lower: &str,
    ) -> bool {
        // Apply collected filter
        match collected_filter {
            CollectedFilter::All => {},
            CollectedFilter::Collected => {
                if !is_collected {
                    return false;
                }
            },
            CollectedFilter::NotCollected => {
                if is_collected {
                    return false;
                }
            },
            CollectedFilter::Unverified => {
                // Show only items with uncertain verification status
                if !verification_status.is_uncertain() {
                    return false;
                }
            },
        }

        // Apply type filter
        let type_match = match type_filter {
            PickupTypeFilter::All => true,
            PickupTypeFilter::GoldenRunes => pickup.category == PickupCategory::GoldenRunes,
            PickupTypeFilter::SmithingStones => pickup.category == PickupCategory::SmithingStones,
            PickupTypeFilter::SomberStones => pickup.category == PickupCategory::SomberStones,
            PickupTypeFilter::Glovewort => pickup.category == PickupCategory::Glovewort,
            PickupTypeFilter::Weapons => pickup.category == PickupCategory::Weapons,
            PickupTypeFilter::Armor => pickup.category == PickupCategory::Armor,
            PickupTypeFilter::Talismans => pickup.category == PickupCategory::Talismans,
            PickupTypeFilter::AshesOfWar => pickup.category == PickupCategory::AshesOfWar,
            PickupTypeFilter::KeyItems => pickup.category == PickupCategory::KeyItems,
            PickupTypeFilter::CraftingMaterials => pickup.category == PickupCategory::CraftingMaterials,
            PickupTypeFilter::Consumables => pickup.category == PickupCategory::Consumables,
            PickupTypeFilter::Other => pickup.category == PickupCategory::Other,
        };

        if !type_match {
            return false;
        }

        // Apply region filter
        if region_filter != "All" && pickup.region != region_filter {
            return false;
        }

        // Apply search
        if !search_lower.is_empty() {
            let matches = pickup.name.to_lowercase().contains(search_lower)
                || pickup.region.to_lowercase().contains(search_lower);
            if !matches {
                return false;
            }
        }

        true
    }

    /// Collect all set flags from the unique items database for verification
    fn collect_set_flags(event_flags: Option<&[u8]>) -> std::collections::HashSet<u32> {
        use crate::discovery::inventory_verification::UNIQUE_ITEMS;
        use crate::db::pickup_flags::is_flag_set_with_status;

        let mut set_flags = std::collections::HashSet::new();

        if let Some(ef) = event_flags {
            for item in UNIQUE_ITEMS.iter() {
                let (is_set, _status) = is_flag_set_with_status(ef, item.event_flag);
                if is_set {
                    set_flags.insert(item.event_flag);
                }
            }
        }

        set_flags
    }

    /// Resolve an inventory item's name from its ga_item_handle
    /// Returns (display_name, item_type_str, raw_item_id)
    fn resolve_inventory_item_name_with_id(ga_item_handle: u32) -> (String, &'static str, u32) {
        let item_type = ga_item_handle & 0xF0000000;
        let item_id = ga_item_handle & 0x0FFFFFFF;

        match item_type {
            0x80000000 => {
                // Weapon
                let base_id = (item_id / 100) * 100;
                let upgrade = item_id % 100;
                if let Some(name) = WEAPON_NAME.lock().unwrap().get(&base_id) {
                    let display_name = if upgrade > 0 {
                        format!("{} +{}", name, upgrade)
                    } else {
                        name.to_string()
                    };
                    (display_name, "Weapon", item_id)
                } else {
                    (format!("[Unknown Weapon {}]", item_id), "Weapon", item_id)
                }
            }
            0x90000000 => {
                // Armor
                if let Some(name) = ARMOR_NAME.lock().unwrap().get(&item_id) {
                    (name.to_string(), "Armor", item_id)
                } else {
                    (format!("[Unknown Armor {}]", item_id), "Armor", item_id)
                }
            }
            0xA0000000 => {
                // Accessory (Talisman)
                if let Some(name) = ACCESSORY_NAME.lock().unwrap().get(&item_id) {
                    (name.to_string(), "Accessory", item_id)
                } else {
                    (format!("[Unknown Accessory {}]", item_id), "Accessory", item_id)
                }
            }
            0xB0000000 => {
                // Item/Good
                if let Some(name) = ITEM_NAME.lock().unwrap().get(&item_id) {
                    (name.to_string(), "Item", item_id)
                } else {
                    (format!("[Unknown Item {}]", item_id), "Item", item_id)
                }
            }
            0xC0000000 => {
                // Ash of War
                if let Some(name) = AOW_NAME.lock().unwrap().get(&item_id) {
                    (name.to_string(), "Ash of War", item_id)
                } else {
                    (format!("[Unknown AoW {}]", item_id), "Ash of War", item_id)
                }
            }
            _ => (format!("[Unknown Type 0x{:X}]", ga_item_handle), "Unknown", item_id),
        }
    }

    /// Fuzzy match: check if two item names are likely the same item
    /// Returns (is_match, match_score) where score is 0-100
    fn fuzzy_match_item_names(flag_name: &str, inv_name: &str) -> (bool, u32) {
        let flag_lower = flag_name.to_lowercase();
        let inv_lower = inv_name.to_lowercase();

        // Exact match
        if flag_lower == inv_lower {
            return (true, 100);
        }

        // One contains the other (handles upgrade levels like "Uchigatana +5")
        if inv_lower.contains(&flag_lower) || flag_lower.contains(&inv_lower) {
            return (true, 90);
        }

        // Strip common suffixes and prefixes for comparison
        let flag_clean = flag_lower
            .trim_end_matches(|c: char| c.is_numeric() || c == '+' || c == ' ')
            .trim();
        let inv_clean = inv_lower
            .trim_end_matches(|c: char| c.is_numeric() || c == '+' || c == ' ')
            .trim();

        if flag_clean == inv_clean {
            return (true, 85);
        }

        // Word-based matching: check if significant words overlap
        let flag_words: std::collections::HashSet<&str> = flag_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();
        let inv_words: std::collections::HashSet<&str> = inv_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        if !flag_words.is_empty() && !inv_words.is_empty() {
            let common: usize = flag_words.intersection(&inv_words).count();
            let total = flag_words.len().max(inv_words.len());
            let overlap_ratio = (common * 100) / total;

            if overlap_ratio >= 60 {
                return (true, overlap_ratio as u32);
            }
        }

        (false, 0)
    }

    /// Inventory match result for display and debugging
    struct InventoryMatch {
        // Display fields
        item_name: String,
        item_type: &'static str,
        quantity: u32,
        match_score: u32,
        is_supporting: bool, // true = supports flag status, false = challenges it
        // Debug fields
        ga_item_handle: u32,
        inventory_index: u32,
        storage_location: &'static str, // "equip_common", "equip_key", "storage_common", "storage_key"
        raw_item_id: u32, // item_id extracted from ga_item_handle
    }

    /// Flag details right sidebar
    fn flag_details_sidebar(
        ui: &mut Ui,
        vm: &mut ViewModel,
        event_flags: Option<&[u8]>,
        inventory: Option<&EquipInventoryData>,
        storage: Option<&EquipInventoryData>,
        save_path: &str,
    ) {
        let (selected_flag_id, flag_name, is_collected, is_world_pickup) = match vm.slots[vm.index].events_vm.current_route {
            EventsRoute::WorldPickups => {
                if let Some(flag_id) = vm.slots[vm.index].events_vm.world_pickups_filter.selected_flag_id {
                    // Find the pickup data for this flag
                    let pickup = WORLD_PICKUPS.iter().find(|p| p.event_flag == flag_id);
                    if let Some(p) = pickup {
                        let ef = event_flags.unwrap_or(&[]);
                        let (is_set, _) = is_flag_set_with_status(ef, flag_id);
                        (Some(flag_id), p.name.to_string(), is_set, true)
                    } else {
                        (None, String::new(), false, true)
                    }
                } else {
                    (None, String::new(), false, true)
                }
            }
            EventsRoute::DungeonPickups => {
                if let Some(flag_id) = vm.slots[vm.index].events_vm.dungeon_pickups_filter.selected_flag_id {
                    // Find the dungeon pickup data for this flag
                    let pickup = DUNGEON_PICKUPS.iter().find(|p| p.event_flag == flag_id);
                    if let Some(p) = pickup {
                        // For dungeon pickups, we need to check the flag differently
                        // using the dungeon base offsets
                        let ef = event_flags.unwrap_or(&[]);
                        let is_set = if let Some(&base) = DUNGEON_PICKUP_BASES.get(&p.dungeon_area) {
                            use crate::db::pickup_flags::DUNGEON_SECTION_SIZE;
                            use crate::util::bit::bit::get_bit;
                            let byte_offset = base + p.section * DUNGEON_SECTION_SIZE + p.event_flag % 10000 / 8;
                            let bit_pos = 7 - (p.event_flag % 8);
                            if (byte_offset as usize) < ef.len() {
                                get_bit(ef[byte_offset as usize], bit_pos as u8)
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        (Some(flag_id), p.name.to_string(), is_set, false)
                    } else {
                        (None, String::new(), false, false)
                    }
                } else {
                    (None, String::new(), false, false)
                }
            }
            _ => (None, String::new(), false, true),
        };

        let selected_flag_id = match selected_flag_id {
            Some(id) => id,
            None => {
                ui.label("No flag selected");
                return;
            }
        };

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Flag Details").strong().size(14.0));
            if ui.small_button("✕").clicked() {
                // Clear selection
                if is_world_pickup {
                    vm.slots[vm.index].events_vm.world_pickups_filter.selected_flag_id = None;
                } else {
                    vm.slots[vm.index].events_vm.dungeon_pickups_filter.selected_flag_id = None;
                }
            }
        });
        ui.separator();

        // Flag Info section
        ui.label(RichText::new("Raw Data").color(Color32::YELLOW).small());
        ui.horizontal(|ui| {
            ui.label(RichText::new("Flag ID:").color(Color32::LIGHT_GRAY));
            ui.label(RichText::new(format!("{}", selected_flag_id)).monospace());
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Hex:").color(Color32::LIGHT_GRAY));
            ui.label(RichText::new(format!("0x{:X}", selected_flag_id)).monospace());
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Item:").color(Color32::LIGHT_GRAY));
            ui.label(&flag_name);
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status:").color(Color32::LIGHT_GRAY));
            let (status_text, status_color) = if is_collected {
                ("COLLECTED", Color32::from_rgb(100, 200, 100))
            } else {
                ("NOT COLLECTED", Color32::LIGHT_GRAY)
            };
            ui.label(RichText::new(status_text).color(status_color));
        });
        ui.separator();

        // Inventory Matching section
        ui.label(RichText::new("Inventory Evidence").color(Color32::YELLOW).small());

        if let Some(inv) = inventory {
            // Find fuzzy matches in inventory
            let mut matches: Vec<InventoryMatch> = Vec::new();

            // Helper to create match entry
            let mut add_match = |item: &crate::save::common::save_slot::EquipInventoryItem,
                                 storage_location: &'static str| {
                if item.ga_item_handle == 0 || item.quantity == 0 {
                    return;
                }
                let (item_name, item_type, raw_item_id) = resolve_inventory_item_name_with_id(item.ga_item_handle);
                let (is_match, score) = fuzzy_match_item_names(&flag_name, &item_name);
                if is_match {
                    let is_supporting = is_collected;
                    matches.push(InventoryMatch {
                        item_name,
                        item_type,
                        quantity: item.quantity,
                        match_score: score,
                        is_supporting,
                        ga_item_handle: item.ga_item_handle,
                        inventory_index: item.inventory_index,
                        storage_location,
                        raw_item_id,
                    });
                }
            };

            // Check equipped common items
            for item in &inv.common_items {
                add_match(item, "equip_common");
            }

            // Check equipped key items
            for item in &inv.key_items {
                add_match(item, "equip_key");
            }

            // Check storage box (if available)
            if let Some(stor) = storage {
                for item in &stor.common_items {
                    add_match(item, "storage_common");
                }
                for item in &stor.key_items {
                    add_match(item, "storage_key");
                }
            }

            // Sort by match score descending
            matches.sort_by(|a, b| b.match_score.cmp(&a.match_score));

            if matches.is_empty() {
                if is_collected {
                    // Flag says collected but no matching item found
                    ui.label(RichText::new("⚠ No matching item in inventory")
                        .color(Color32::from_rgb(255, 200, 100))
                        .small());
                    ui.label(RichText::new("This CHALLENGES the flag status")
                        .color(Color32::from_rgb(255, 165, 0))
                        .small());
                    ui.label(RichText::new("(item may have been sold/used)")
                        .color(Color32::GRAY)
                        .small());
                } else {
                    // Flag says not collected and no item found - consistent
                    ui.label(RichText::new("No matching item found")
                        .color(Color32::GRAY)
                        .small());
                    ui.label(RichText::new("This SUPPORTS the flag status")
                        .color(Color32::from_rgb(100, 200, 100))
                        .small());
                }
            } else {
                // Show matches
                ui.label(RichText::new(format!("Found {} match(es):", matches.len()))
                    .color(Color32::LIGHT_GRAY)
                    .small());
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .id_salt("flag_details_matches")
                    .max_height(250.0)
                    .show(ui, |ui| {
                        for (idx, m) in matches.iter().enumerate() {
                            let evidence_type = if m.is_supporting {
                                ("SUPPORTS", Color32::from_rgb(100, 200, 100))
                            } else {
                                ("CHALLENGES", Color32::from_rgb(255, 165, 0))
                            };

                            // Use push_id to ensure unique widget IDs for each match
                            ui.push_id(idx, |ui| {
                                egui::Frame::none()
                                    .inner_margin(egui::Margin::same(4.0))
                                    .fill(Color32::from_rgb(40, 40, 50))
                                    .rounding(2.0)
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(&m.item_name).strong());
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(m.item_type).color(Color32::GRAY).small());
                                            ui.label(RichText::new(format!("x{}", m.quantity)).small());
                                            ui.label(RichText::new(format!("{}%", m.match_score)).color(Color32::GRAY).small());
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(evidence_type.0).color(evidence_type.1).small());
                                            ui.label(RichText::new("flag status").color(Color32::GRAY).small());
                                        });
                                        // Debug details (collapsible)
                                        ui.collapsing(RichText::new("Raw Data").color(Color32::GRAY).small(), |ui| {
                                            ui.label(RichText::new(format!("ga_item_handle: 0x{:08X}", m.ga_item_handle)).monospace().small());
                                            ui.label(RichText::new(format!("raw_item_id: {}", m.raw_item_id)).monospace().small());
                                            ui.label(RichText::new(format!("inventory_index: {}", m.inventory_index)).monospace().small());
                                            ui.label(RichText::new(format!("storage: {}", m.storage_location)).monospace().small());
                                        });
                                    });
                                ui.add_space(2.0);
                            });
                        }
                    });

                // Summary
                if !is_collected && !matches.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("⚠ Item found but flag NOT set")
                        .color(Color32::from_rgb(255, 165, 0))
                        .small());
                    ui.label(RichText::new("This CHALLENGES the flag status")
                        .color(Color32::from_rgb(255, 165, 0))
                        .small());
                    ui.label(RichText::new("(possible detection error)")
                        .color(Color32::GRAY)
                        .small());
                }
            }
        } else {
            ui.label(RichText::new("No inventory data available").color(Color32::GRAY));
        }

        ui.separator();

        // Copy Details button - generates comprehensive debug output
        if ui.button("Copy Details").clicked() {
            let mut details = String::new();

            // Context metadata for precise understanding
            let slot_index = vm.index;
            let character_name = vm.slots[slot_index].general_vm.character_name.trim_matches('\0');
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

            details.push_str("=== CONTEXT ===\n");
            details.push_str(&format!("timestamp: {}\n", timestamp));
            details.push_str(&format!("save_file: {}\n", save_path));
            details.push_str(&format!("slot_index: {}\n", slot_index));
            details.push_str(&format!("character_name: {}\n", character_name));
            details.push_str(&format!("event_flags_size: {}\n", event_flags.map(|ef| ef.len()).unwrap_or(0)));

            details.push_str("\n=== FLAG DETAILS ===\n");
            details.push_str(&format!("flag_id: {}\n", selected_flag_id));
            details.push_str(&format!("flag_id_hex: 0x{:08X}\n", selected_flag_id));
            details.push_str(&format!("item_name: {}\n", flag_name));
            details.push_str(&format!("is_collected: {}\n", is_collected));
            details.push_str(&format!("pickup_type: {}\n", if is_world_pickup { "world" } else { "dungeon" }));

            // Add flag offset info if available
            if let Some(ef) = event_flags {
                use crate::db::pickup_flags::get_flag_offset;
                if let Some((byte_off, bit_pos)) = get_flag_offset(selected_flag_id) {
                    details.push_str(&format!("byte_offset: {} (0x{:X})\n", byte_off, byte_off));
                    details.push_str(&format!("bit_position: {}\n", bit_pos));
                    if (byte_off as usize) < ef.len() {
                        let byte_value = ef[byte_off as usize];
                        details.push_str(&format!("byte_value: 0x{:02X} (binary: {:08b})\n", byte_value, byte_value));
                        let bit_set = (byte_value & (1 << bit_pos)) != 0;
                        details.push_str(&format!("bit_is_set: {}\n", bit_set));
                    }
                } else {
                    details.push_str("offset: UNKNOWN (no formula)\n");
                }
            }

            details.push_str("\n=== INVENTORY EVIDENCE ===\n");

            // Helper closure for scanning items
            let scan_items = |items: &[crate::save::common::save_slot::EquipInventoryItem],
                             location: &str,
                             flag_name: &str,
                             is_collected: bool,
                             match_count: &mut u32,
                             details: &mut String| {
                for item in items {
                    if item.ga_item_handle == 0 || item.quantity == 0 {
                        continue;
                    }
                    let (item_name_resolved, item_type, raw_id) = resolve_inventory_item_name_with_id(item.ga_item_handle);
                    let (is_match, score) = fuzzy_match_item_names(flag_name, &item_name_resolved);
                    if is_match {
                        *match_count += 1;
                        details.push_str(&format!("\n--- Match {} ---\n", *match_count));
                        details.push_str(&format!("storage: {}\n", location));
                        details.push_str(&format!("item_name: {}\n", item_name_resolved));
                        details.push_str(&format!("item_type: {}\n", item_type));
                        details.push_str(&format!("quantity: {}\n", item.quantity));
                        details.push_str(&format!("match_score: {}%\n", score));
                        details.push_str(&format!("ga_item_handle: 0x{:08X}\n", item.ga_item_handle));
                        details.push_str(&format!("raw_item_id: {}\n", raw_id));
                        details.push_str(&format!("inventory_index: {}\n", item.inventory_index));
                        let verdict = if is_collected { "SUPPORTS" } else { "CHALLENGES" };
                        details.push_str(&format!("verdict: {} flag status\n", verdict));
                    }
                }
            };

            let mut match_count = 0u32;

            if let Some(inv) = inventory {
                details.push_str(&format!("equip_common_count: {}\n", inv.common_inventory_items_distinct_count));
                details.push_str(&format!("equip_key_count: {}\n", inv.key_inventory_items_distinct_count));
                scan_items(&inv.common_items, "equip_common", &flag_name, is_collected, &mut match_count, &mut details);
                scan_items(&inv.key_items, "equip_key", &flag_name, is_collected, &mut match_count, &mut details);
            } else {
                details.push_str("equip_inventory: NOT AVAILABLE\n");
            }

            if let Some(stor) = storage {
                details.push_str(&format!("storage_common_count: {}\n", stor.common_inventory_items_distinct_count));
                details.push_str(&format!("storage_key_count: {}\n", stor.key_inventory_items_distinct_count));
                scan_items(&stor.common_items, "storage_common", &flag_name, is_collected, &mut match_count, &mut details);
                scan_items(&stor.key_items, "storage_key", &flag_name, is_collected, &mut match_count, &mut details);
            } else {
                details.push_str("storage_inventory: NOT AVAILABLE\n");
            }

            if match_count == 0 {
                details.push_str("\nNo matching items found in any inventory\n");
            } else {
                details.push_str(&format!("\nTotal matches: {}\n", match_count));
            }

            ui.output_mut(|o| o.copied_text = details);
        }
    }
}
