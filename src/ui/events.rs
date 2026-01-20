pub mod events {

    use eframe::egui::{self, Ui, Color32, RichText, ScrollArea};
    use crate::{db::{bosses::bosses::BOSSES, colosseums::colosseums::COLOSSEUMS, cookbooks::books::COOKBOKS, graces::maps::GRACES, landmarks::landmarks::LANDMARKS, map_name::map_name::MAP_NAME, maps::maps::MAPS, summoning_pools::summoning_pools::SUMMONING_POOLS, whetblades::whetblades::WHETBLADES, pickup_data::{WORLD_PICKUPS, PickupCategory}, pickup_flags::is_flag_set}, ui::verification_view::verification_view::verification_view, vm::{events::events_view_model::{EventsRoute, PickupTypeFilter, CollectedFilter}, vm::vm::ViewModel}};

    pub fn events(ui: &mut Ui, vm: &mut ViewModel, event_flags: Option<&[u8]>) {
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
                        EventsRoute::Verification => {verification.highlight();},
                    }
                })
            });
        });

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
                    EventsRoute::Verification => {
                        verification_view(ui, &mut vm.slots[vm.index].events_vm.verification_vm);
                    },
                }
            });
        });

    }

    fn graces(ui: &mut Ui, vm: &mut ViewModel) {
        let graces_data = &vm.slots[vm.index].events_vm.graces;
        let grace_groups = &vm.slots[vm.index].events_vm.grace_groups;

        // Count discovered
        let discovered_count = graces_data.values().filter(|v| **v).count();
        let total_count = graces_data.len();

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Name | Region | Flag ID").color(Color32::YELLOW).monospace());
        });
        ui.separator();

        // Summary
        let summary = format!("Sites of Grace: {}/{} discovered", discovered_count, total_count);
        ui.label(RichText::new(&summary).strong());
        ui.separator();

        let graces_lookup = GRACES.lock().unwrap();

        // Group by region
        for (map_id, grace_ids) in grace_groups {
            let region_name = MAP_NAME.lock().unwrap().get(map_id).cloned().unwrap_or("Unknown");
            let region_discovered = grace_ids.iter().filter(|g| graces_data.get(g) == Some(&true)).count();

            ui.label(RichText::new(format!("{} ({}/{})", region_name, region_discovered, grace_ids.len())).strong());

            for grace_id in grace_ids {
                if let Some(grace_info) = graces_lookup.get(grace_id) {
                    let discovered = graces_data.get(grace_id) == Some(&true);
                    let status = if discovered { "[X]" } else { "[ ]" };
                    let flag_id = grace_info.1;
                    let name = grace_info.2;

                    let row_text = format!("{} | {} | {} | {}", status, name, region_name, flag_id);
                    display_event_row(ui, &row_text, name, flag_id, discovered);
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
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace());
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
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace());
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
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace());
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
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace());
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
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace());
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
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace());
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
            ui.label(RichText::new("Status | Name | Flag ID").color(Color32::YELLOW).monospace());
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
        let text_color = if discovered {
            Color32::from_rgb(100, 200, 100)
        } else {
            Color32::LIGHT_GRAY
        };

        let response = ui.add(
            egui::Label::new(RichText::new(row_text).color(text_color).monospace())
                .sense(egui::Sense::click())
        );

        if response.double_clicked() {
            ui.output_mut(|o| o.copied_text = row_text.to_string());
        }

        response.context_menu(|ui| {
            if ui.button("Copy row").clicked() {
                ui.output_mut(|o| o.copied_text = row_text.to_string());
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

        let ef = event_flags.unwrap_or(&[]);

        for pickup in WORLD_PICKUPS.iter() {
            let is_collected = is_flag_set(ef, pickup.event_flag);
            if is_collected {
                collected += 1;
            }
            total += 1;

            // Check if passes filters
            if passes_pickup_filters(pickup, is_collected, type_filter, collected_filter, &region_filter, &search_lower) {
                filtered_total += 1;
                if is_collected {
                    filtered_collected += 1;
                }
            }
        }

        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status | Lot ID | Flag ID | Item | Category | Qty | Region").color(Color32::YELLOW).monospace());
        });
        ui.separator();

        // Summary
        let summary = if filtered_total == total {
            format!("World Pickups: {}/{} collected", collected, total)
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
                let is_collected = is_flag_set(ef, pickup.event_flag);
                if is_collected {
                    region_collected += 1;
                }
                if passes_pickup_filters(pickup, is_collected, type_filter, collected_filter, &region_filter, &search_lower) {
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
                let is_collected = is_flag_set(ef, pickup.event_flag);

                // Apply filters
                if !passes_pickup_filters(pickup, is_collected, type_filter, collected_filter, &region_filter, &search_lower) {
                    continue;
                }

                let status = if is_collected { "[X]" } else { "[ ]" };

                let row_text = format!(
                    "{} | {} | {} | {} | {} | {} | {}",
                    status, pickup.item_lot_id, pickup.event_flag, pickup.name,
                    pickup.category.display_name(), pickup.quantity, region
                );

                let text_color = if is_collected {
                    Color32::from_rgb(100, 200, 100)
                } else {
                    Color32::LIGHT_GRAY
                };

                let response = ui.add(
                    egui::Label::new(RichText::new(&row_text).color(text_color).monospace())
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

    fn passes_pickup_filters(
        pickup: &crate::db::pickup_data::WorldPickup,
        is_collected: bool,
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
}
