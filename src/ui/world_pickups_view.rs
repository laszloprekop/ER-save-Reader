pub mod world_pickups_view {
    use eframe::egui::{self, Ui, Color32, RichText};
    use crate::db::world_pickups::{WORLD_PICKUPS, PickupItemType};
    use crate::db::pickup_flags::get_flag_offset_calibrated;
    use crate::calibration::CalibrationService;
    use crate::util::bit::bit::get_bit;
    use crate::ui::style::{TABLE_MONO_SIZE, spacer};
    use crate::save::common::save_slot::EquipInventoryData;
    use crate::discovery::inventory_verification::{
        InventoryVerificationService, UNIQUE_ITEMS_BY_FLAG, VerificationConfidence,
    };

    #[derive(Clone, Copy, PartialEq)]
    pub enum PickupFilter {
        All,
        Weapon,
        Armor,
        Accessory,
        Good,
        AshOfWar,
    }

    #[derive(Clone, Copy, PartialEq)]
    pub enum CollectedFilter {
        All,
        Collected,
        NotCollected,
        Unverified,
    }

    pub struct WorldPickupsViewState {
        pub filter: PickupFilter,
        pub collected_filter: CollectedFilter,
        pub region_filter: String,
        pub search: String,
        pub selected_id: Option<u32>,
    }

    impl Default for WorldPickupsViewState {
        fn default() -> Self {
            Self {
                filter: PickupFilter::All,
                collected_filter: CollectedFilter::All,
                region_filter: "All".to_string(),
                search: String::new(),
                selected_id: None,
            }
        }
    }

    /// Check if a pickup's event flag is set (collected)
    /// Uses calibrated formula-based offset calculation from pickup_flags.rs
    ///
    /// # Arguments
    /// * `flag_id` - The event flag ID to check
    /// * `event_flags` - The event flags byte slice from the save
    /// * `calibrated_tile_base` - The calibrated tile base for this save
    fn is_pickup_collected(flag_id: u32, event_flags: Option<&[u8]>, calibrated_tile_base: u32) -> Option<bool> {
        let event_flags = event_flags?;

        if let Some((byte_offset, bit_position)) = get_flag_offset_calibrated(flag_id, calibrated_tile_base) {
            if (byte_offset as usize) < event_flags.len() {
                return Some(get_bit(event_flags[byte_offset as usize], bit_position));
            }
        }

        None
    }

    /// Check if an item is in the character's inventory based on flag ID
    /// Returns (has_item, confidence) or None if no mapping exists
    fn is_item_in_inventory(flag_id: u32, inventory: Option<&EquipInventoryData>) -> Option<(bool, VerificationConfidence)> {
        let inventory = inventory?;

        // Check if this flag has associated unique items
        if let Some(mappings) = UNIQUE_ITEMS_BY_FLAG.get(&flag_id) {
            // Extract item IDs from inventory
            let inventory_items: std::collections::HashSet<u32> = inventory
                .common_items
                .iter()
                .chain(inventory.key_items.iter())
                .filter(|item| item.ga_item_handle != 0)
                .map(|item| item.ga_item_handle & 0x0FFFFFFF)
                .collect();

            // Check if any expected item is present
            for mapping in mappings {
                if inventory_items.contains(&mapping.item_id) {
                    return Some((true, mapping.confidence));
                }
            }

            // Item not found, return best confidence from mappings
            let best_confidence = mappings
                .iter()
                .map(|m| m.confidence)
                .min_by_key(|c| match c {
                    VerificationConfidence::VeryHigh => 0,
                    VerificationConfidence::High => 1,
                    VerificationConfidence::Medium => 2,
                    VerificationConfidence::Low => 3,
                    VerificationConfidence::Unknown => 4,
                })
                .unwrap_or(VerificationConfidence::Unknown);

            return Some((false, best_confidence));
        }

        None // No unique item mapping for this flag
    }

    pub fn world_pickups_view(
        ui: &mut Ui,
        state: &mut WorldPickupsViewState,
        event_flags: Option<&[u8]>,
        inventory: Option<&EquipInventoryData>,
    ) {
        // Calibrate tile base once per frame (if event flags available)
        let calibrated_tile_base = event_flags
            .map(|ef| CalibrationService::calibrate(ef).tile_base)
            .unwrap_or(crate::generated::ground_truth::VERIFIED_TILE_BASE_OFFSET);

        // Header with filters
        ui.horizontal(|ui| {
            ui.label(RichText::new("Type:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut state.filter, PickupFilter::All, "All");
            ui.selectable_value(&mut state.filter, PickupFilter::Weapon, "Weapon");
            ui.selectable_value(&mut state.filter, PickupFilter::Armor, "Armor");
            ui.selectable_value(&mut state.filter, PickupFilter::Accessory, "Accessory");
            ui.selectable_value(&mut state.filter, PickupFilter::Good, "Good");
            ui.selectable_value(&mut state.filter, PickupFilter::AshOfWar, "Ash of War");
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Region:").color(Color32::LIGHT_GRAY));

            // Get unique regions
            let mut regions: Vec<&str> = WORLD_PICKUPS.values()
                .map(|p| p.region)
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            regions.sort();
            regions.insert(0, "All");

            egui::ComboBox::from_id_salt("region_filter")
                .selected_text(&state.region_filter)
                .show_ui(ui, |ui| {
                    for region in &regions {
                        ui.selectable_value(&mut state.region_filter, region.to_string(), *region);
                    }
                });

            spacer(ui);
            ui.label(RichText::new("Search:").color(Color32::LIGHT_GRAY));
            ui.text_edit_singleline(&mut state.search);
        });

        // Collected filter (only show if event flags are available)
        if event_flags.is_some() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Status:").color(Color32::LIGHT_GRAY));
                ui.selectable_value(&mut state.collected_filter, CollectedFilter::All, "All");
                ui.selectable_value(&mut state.collected_filter, CollectedFilter::Collected, "Collected");
                ui.selectable_value(&mut state.collected_filter, CollectedFilter::NotCollected, "Not Collected");
                ui.selectable_value(&mut state.collected_filter, CollectedFilter::Unverified, "Unverified");
            });
        }
        spacer(ui);

        // Column headers
        let header = match (event_flags.is_some(), inventory.is_some()) {
            (true, true) => "Flag | Inv | Lot ID | Flag ID | Item | Type | Qty | Region | Tile",
            (true, false) => "Flag | Lot ID | Flag ID | Item | Type | Qty | Region | Tile",
            (false, true) => "Inv | Lot ID | Flag ID | Item | Type | Qty | Region | Tile",
            (false, false) => "Lot ID | Flag ID | Item | Type | Qty | Region | Tile",
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new(header).color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        spacer(ui);

        // Scrollable list
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let search_lower = state.search.to_lowercase();

                let mut pickups: Vec<_> = WORLD_PICKUPS.iter().collect();
                pickups.sort_by_key(|(id, _)| *id);

                for (id, pickup) in pickups {
                    // Check collected status using calibrated tile base
                    let is_collected = is_pickup_collected(pickup.flag_id, event_flags, calibrated_tile_base);

                    // Apply collected filter
                    match state.collected_filter {
                        CollectedFilter::All => {},
                        CollectedFilter::Collected => {
                            if is_collected != Some(true) {
                                continue;
                            }
                        },
                        CollectedFilter::NotCollected => {
                            if is_collected == Some(true) {
                                continue;
                            }
                        },
                        CollectedFilter::Unverified => {
                            // Show only items where verification status is unknown
                            if is_collected.is_some() {
                                continue;
                            }
                        },
                    }

                    // Apply type filter
                    let type_match = match state.filter {
                        PickupFilter::All => true,
                        PickupFilter::Weapon => pickup.item_type == PickupItemType::Weapon,
                        PickupFilter::Armor => pickup.item_type == PickupItemType::Armor,
                        PickupFilter::Accessory => pickup.item_type == PickupItemType::Accessory,
                        PickupFilter::Good => pickup.item_type == PickupItemType::Good,
                        PickupFilter::AshOfWar => pickup.item_type == PickupItemType::AshOfWar,
                    };

                    if !type_match {
                        continue;
                    }

                    // Apply region filter
                    if state.region_filter != "All" && pickup.region != state.region_filter {
                        continue;
                    }

                    // Apply search
                    if !state.search.is_empty() {
                        let matches = pickup.item_name.to_lowercase().contains(&search_lower)
                            || pickup.region.to_lowercase().contains(&search_lower);
                        if !matches {
                            continue;
                        }
                    }

                    let type_str = match pickup.item_type {
                        PickupItemType::Weapon => "Weapon",
                        PickupItemType::Armor => "Armor",
                        PickupItemType::Accessory => "Accessory",
                        PickupItemType::Good => "Good",
                        PickupItemType::AshOfWar => "Ash of War",
                        PickupItemType::Unknown => "Unknown",
                    };

                    let tile_str = format!("({}, {})", pickup.tile_x, pickup.tile_y);

                    // Check inventory status
                    let inv_status = is_item_in_inventory(pickup.flag_id, inventory);

                    // Build flag status string
                    let flag_str = match is_collected {
                        Some(true) => "[X]",
                        Some(false) => "[ ]",
                        None => "[?]",
                    };

                    // Build inventory status string with confidence indicator
                    let inv_str = match inv_status {
                        Some((true, VerificationConfidence::VeryHigh)) => "[X]",  // Has item, very high confidence
                        Some((true, VerificationConfidence::High)) => "[x]",     // Has item, high confidence
                        Some((true, _)) => "[~]",                                 // Has item, lower confidence
                        Some((false, VerificationConfidence::VeryHigh)) => "[ ]", // No item, very high confidence
                        Some((false, VerificationConfidence::High)) => "[-]",    // No item, high confidence
                        Some((false, _)) => "[.]",                                // No item, lower confidence
                        None => "   ",                                            // No mapping
                    };

                    // Build row text based on available data
                    let row_text = match (event_flags.is_some(), inventory.is_some()) {
                        (true, true) => format!(
                            "{} | {} | {} | {} | {} | {} | {} | {} | {}",
                            flag_str, inv_str, id, pickup.flag_id, pickup.item_name, type_str, pickup.quantity, pickup.region, tile_str
                        ),
                        (true, false) => format!(
                            "{} | {} | {} | {} | {} | {} | {} | {}",
                            flag_str, id, pickup.flag_id, pickup.item_name, type_str, pickup.quantity, pickup.region, tile_str
                        ),
                        (false, true) => format!(
                            "{} | {} | {} | {} | {} | {} | {} | {}",
                            inv_str, id, pickup.flag_id, pickup.item_name, type_str, pickup.quantity, pickup.region, tile_str
                        ),
                        (false, false) => format!(
                            "{} | {} | {} | {} | {} | {} | {}",
                            id, pickup.flag_id, pickup.item_name, type_str, pickup.quantity, pickup.region, tile_str
                        ),
                    };

                    let status_str = flag_str.to_string();

                    let is_selected = state.selected_id == Some(*id);

                    // Determine if there's a mismatch between flag and inventory
                    let has_mismatch = match (is_collected, inv_status) {
                        (Some(flag_set), Some((has_item, _))) if flag_set != has_item => true,
                        _ => false,
                    };

                    // Color based on collected status and mismatches
                    let text_color = if is_selected {
                        Color32::YELLOW
                    } else if has_mismatch {
                        Color32::from_rgb(255, 165, 0) // Orange for mismatch
                    } else {
                        match is_collected {
                            Some(true) => Color32::from_rgb(100, 200, 100), // Green for collected
                            Some(false) => Color32::LIGHT_GRAY,
                            None => Color32::from_rgb(180, 180, 180), // Dim for unknown
                        }
                    };

                    let response = ui.add(
                        egui::Label::new(RichText::new(&row_text).color(text_color).monospace().size(TABLE_MONO_SIZE))
                            .sense(egui::Sense::click())
                    );

                    if response.clicked() {
                        state.selected_id = Some(*id);
                    }

                    if response.double_clicked() {
                        ui.output_mut(|o| o.copied_text = row_text.clone());
                    }

                    response.context_menu(|ui| {
                        if ui.button("Copy row").clicked() {
                            ui.output_mut(|o| o.copied_text = row_text.clone());
                            ui.close_menu();
                        }
                        if ui.button("Copy Item Name").clicked() {
                            ui.output_mut(|o| o.copied_text = pickup.item_name.to_string());
                            ui.close_menu();
                        }
                        if ui.button("Copy Flag ID").clicked() {
                            ui.output_mut(|o| o.copied_text = pickup.flag_id.to_string());
                            ui.close_menu();
                        }
                        if ui.button("Copy Tile").clicked() {
                            ui.output_mut(|o| o.copied_text = tile_str.clone());
                            ui.close_menu();
                        }
                        if !status_str.is_empty() {
                            if ui.button("Copy Status").clicked() {
                                let status_text = match is_collected {
                                    Some(true) => "Collected",
                                    Some(false) => "Not Collected",
                                    None => "Unknown",
                                };
                                ui.output_mut(|o| o.copied_text = status_text.to_string());
                                ui.close_menu();
                            }
                        }
                    });
                }
            });
    }
}
