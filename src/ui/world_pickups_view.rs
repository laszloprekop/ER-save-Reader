pub mod world_pickups_view {
    use eframe::egui::{Ui, Color32, RichText};
    use crate::db::world_pickups::{WORLD_PICKUPS, PickupItemType};
    use crate::save::common::save_slot::EquipInventoryData;
    use crate::discovery::inventory_verification::{UNIQUE_ITEMS_BY_FLAG, VerificationConfidence};
    use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData, SortDirection};
    use crate::ui::components::filter::{FilterBar, FilterBarState, FilterOption, fuzzy_match_default};
    use crate::ui::components::export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
    use crate::ui::components::legend::icons;
    use crate::ui::components::detail_panel::{DetailPanelState, SelectedEntity, RelationshipSection, RelationshipItem, DetailPanelAction};
    use crate::ui::tokens::spacing;
    use serde::Serialize;

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

    #[derive(Serialize)]
    struct WorldPickupExportItem {
        lot_id: u32,
        flag_id: u32,
        item_name: String,
        item_type: String,
        quantity: i32,
        region: String,
        tile: String,
        collected: Option<bool>,
        in_inventory: Option<bool>,
    }

    pub struct WorldPickupsViewState {
        pub filter: PickupFilter,
        pub collected_filter: CollectedFilter,
        pub region_filter: String,
        pub search: String,
        pub selected_id: Option<u32>,
        /// Track which pickup we last opened the detail panel for
        pub last_detail_id: Option<u32>,
        pub table_state: TableState,
        pub filter_state: FilterBarState,
        pub export_format: ExportFormat,
        pub export_filtered_only: bool,
    }

    impl Default for WorldPickupsViewState {
        fn default() -> Self {
            Self {
                filter: PickupFilter::All,
                collected_filter: CollectedFilter::All,
                region_filter: "All".to_string(),
                search: String::new(),
                selected_id: None,
                last_detail_id: None,
                table_state: TableState::new().with_sort("lot_id", SortDirection::Ascending),
                filter_state: FilterBarState::new(),
                export_format: ExportFormat::Json,
                export_filtered_only: false,
            }
        }
    }

    /// Check if a pickup's event flag is set (collected)
    /// Uses calibrated formula-based offset calculation from pickup_flags.rs
    ///
    /// # Arguments
    /// * `flag_id` - The event flag ID to check
    /// * `event_flags` - The event flags byte slice from the save
    /// CUT OVER 2026-07-20 (ADR-0006, migration step 4): world pickups no longer
    /// use a calibrated tile base. Position resolves per save from the flag
    /// region, and the two tile families are separated by local id — pickups
    /// (localId >= 7000) live in their own region, addressed by row_id, 500 bytes
    /// from the open-world family. Sending one to the other's base reads a
    /// plausible-looking wrong bit rather than failing.
    ///
    /// `None` means the position could not be resolved: UNKNOWN, not "not
    /// collected". The table renders that distinctly.
    fn is_pickup_collected(flag_id: u32, event_flags: Option<&[u8]>) -> Option<bool> {
        wasm_event_flags::is_tile_pickup_set(event_flags?, flag_id)
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
        detail_panel: &mut DetailPanelState,
    ) {
        // Build region filter options
        let mut regions: Vec<&str> = WORLD_PICKUPS.values()
            .map(|p| p.region)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        regions.sort();

        let region_options: Vec<FilterOption> = std::iter::once(FilterOption::all())
            .chain(regions.iter().map(|r| FilterOption::from_str(*r)))
            .collect();

        // Sync filter state
        state.region_filter = state.filter_state.category.clone();
        state.search = state.filter_state.search.clone();

        // Filter bar with region dropdown and search
        FilterBar::new("world_pickups_filter", &mut state.filter_state)
            .category("Region", region_options)
            .search("Search items...")
            .show(ui);

        spacing::space_sm(ui);

        // Type filter chips
        ui.horizontal(|ui| {
            ui.label(RichText::new("Type:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut state.filter, PickupFilter::All, "All");
            ui.selectable_value(&mut state.filter, PickupFilter::Weapon, "Weapon");
            ui.selectable_value(&mut state.filter, PickupFilter::Armor, "Armor");
            ui.selectable_value(&mut state.filter, PickupFilter::Accessory, "Accessory");
            ui.selectable_value(&mut state.filter, PickupFilter::Good, "Good");
            ui.selectable_value(&mut state.filter, PickupFilter::AshOfWar, "Ash of War");
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

        spacing::space_sm(ui);

        // Export toolbar
        let export_response = ExportToolbar::new("world_pickups_export", &mut state.export_format, &mut state.export_filtered_only)
            .has_filters(state.filter_state.has_active_filters() || state.filter != PickupFilter::All || state.collected_filter != CollectedFilter::All)
            .show(ui);

        spacing::space_sm(ui);

        // Legend is now shown in the app-wide status bar

        // Build filtered data
        let mut pickups: Vec<(u32, &crate::db::world_pickups::WorldPickup, Option<bool>, Option<(bool, VerificationConfidence)>)> = WORLD_PICKUPS.iter()
            .filter_map(|(id, pickup)| {
                // Check collected status using calibrated tile base
                let is_collected = is_pickup_collected(pickup.flag_id, event_flags);

                // Apply collected filter
                match state.collected_filter {
                    CollectedFilter::All => {},
                    CollectedFilter::Collected => {
                        if is_collected != Some(true) {
                            return None;
                        }
                    },
                    CollectedFilter::NotCollected => {
                        if is_collected == Some(true) {
                            return None;
                        }
                    },
                    CollectedFilter::Unverified => {
                        if is_collected.is_some() {
                            return None;
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
                    return None;
                }

                // Apply region filter
                if state.region_filter != "All" && pickup.region != state.region_filter {
                    return None;
                }

                // Apply search using fuzzy match
                if !state.search.is_empty() {
                    let matches = fuzzy_match_default(&pickup.item_name, &state.search)
                        || fuzzy_match_default(&pickup.region, &state.search);
                    if !matches {
                        return None;
                    }
                }

                let inv_status = is_item_in_inventory(pickup.flag_id, inventory);
                Some((*id, pickup, is_collected, inv_status))
            })
            .collect();

        // Apply sorting
        if let Some(sort_col) = &state.table_state.sort_column {
            let asc = state.table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "lot_id" => pickups.sort_by(|a, b| if asc { a.0.cmp(&b.0) } else { b.0.cmp(&a.0) }),
                "flag_id" => pickups.sort_by(|a, b| if asc { a.1.flag_id.cmp(&b.1.flag_id) } else { b.1.flag_id.cmp(&a.1.flag_id) }),
                "item" => pickups.sort_by(|a, b| if asc { a.1.item_name.cmp(&b.1.item_name) } else { b.1.item_name.cmp(&a.1.item_name) }),
                "type" => pickups.sort_by(|a, b| {
                    let ta = format!("{:?}", a.1.item_type);
                    let tb = format!("{:?}", b.1.item_type);
                    if asc { ta.cmp(&tb) } else { tb.cmp(&ta) }
                }),
                "qty" => pickups.sort_by(|a, b| if asc { a.1.quantity.cmp(&b.1.quantity) } else { b.1.quantity.cmp(&a.1.quantity) }),
                "region" => pickups.sort_by(|a, b| if asc { a.1.region.cmp(&b.1.region) } else { b.1.region.cmp(&a.1.region) }),
                "status" => pickups.sort_by(|a, b| {
                    let sa = a.2.map(|c| if c { 1 } else { 0 }).unwrap_or(2);
                    let sb = b.2.map(|c| if c { 1 } else { 0 }).unwrap_or(2);
                    if asc { sa.cmp(&sb) } else { sb.cmp(&sa) }
                }),
                _ => {}
            }
        }

        // Auto-open detail panel if selection was set programmatically (from navigation)
        if let Some(id) = state.selected_id {
            if state.last_detail_id != Some(id) {
                // Find the pickup in the filtered list and open its detail panel
                if let Some((_, pickup, _, _)) = pickups.iter().find(|(lot_id, _, _, _)| *lot_id == id) {
                    let type_str = match pickup.item_type {
                        PickupItemType::Weapon => "Weapon",
                        PickupItemType::Armor => "Armor",
                        PickupItemType::Accessory => "Accessory",
                        PickupItemType::Good => "Good",
                        PickupItemType::AshOfWar => "Ash of War",
                        PickupItemType::Unknown => "Unknown",
                    };

                    let mut sections = Vec::new();

                    // Add location info
                    sections.push(
                        RelationshipSection::new("Location").with_items(vec![
                            RelationshipItem::new(
                                pickup.region.to_string(),
                                DetailPanelAction::None,
                            ).with_secondary(format!("Tile ({}, {})", pickup.tile_x, pickup.tile_y)),
                        ])
                    );

                    // Add item type info
                    sections.push(
                        RelationshipSection::new("Details").with_items(vec![
                            RelationshipItem::new(
                                format!("Type: {}", type_str),
                                DetailPanelAction::None,
                            ),
                            RelationshipItem::new(
                                format!("Quantity: {}", pickup.quantity),
                                DetailPanelAction::None,
                            ),
                            RelationshipItem::new(
                                format!("Flag ID: {}", pickup.flag_id),
                                DetailPanelAction::None,
                            ),
                        ])
                    );

                    detail_panel.select_with_relationships(
                        SelectedEntity::Pickup {
                            flag_id: pickup.flag_id,
                            item_name: pickup.item_name.to_string(),
                        },
                        sections,
                    );
                    state.last_detail_id = Some(id);
                }
            }
        }

        // Summary
        let total_count = WORLD_PICKUPS.len();
        let filtered_count = pickups.len();
        if filtered_count < total_count {
            ui.label(RichText::new(format!("World Pickups: {} (showing {}/{})", total_count, filtered_count, total_count)).strong());
        } else {
            ui.label(RichText::new(format!("World Pickups: {}", total_count)).strong());
        }

        spacing::space_sm(ui);

        // Build row data with status colors
        let rows: Vec<RowData> = pickups.iter().map(|(id, pickup, is_collected, inv_status)| {
            let type_str = match pickup.item_type {
                PickupItemType::Weapon => "Weapon",
                PickupItemType::Armor => "Armor",
                PickupItemType::Accessory => "Accessory",
                PickupItemType::Good => "Good",
                PickupItemType::AshOfWar => "Ash of War",
                PickupItemType::Unknown => "Unknown",
            };

            let tile_str = format!("({}, {})", pickup.tile_x, pickup.tile_y);

            let flag_str = match is_collected {
                Some(true) => icons::COLLECTED,
                Some(false) => icons::NOT_COLLECTED,
                None => icons::UNKNOWN,
            };

            let inv_str = match inv_status {
                Some((true, VerificationConfidence::VeryHigh)) => icons::HIGH_CONFIDENCE,
                Some((true, VerificationConfidence::High)) => icons::COLLECTED,
                Some((true, _)) => icons::PARTIAL,
                Some((false, VerificationConfidence::VeryHigh)) => icons::NOT_COLLECTED,
                Some((false, VerificationConfidence::High)) => icons::NO_DATA,
                Some((false, _)) => icons::LOW_CONFIDENCE,
                None => icons::NO_DATA,
            };

            let is_selected = state.selected_id == Some(*id);

            // Determine if there's a mismatch between flag and inventory
            let has_mismatch = match (is_collected, inv_status) {
                (Some(flag_set), Some((has_item, _))) if *flag_set != *has_item => true,
                _ => false,
            };

            // Build row cells - include all columns
            let mut cells = vec![];

            if event_flags.is_some() {
                cells.push(flag_str.to_string());
            }
            if inventory.is_some() {
                cells.push(inv_str.to_string());
            }

            cells.extend(vec![
                id.to_string(),
                pickup.flag_id.to_string(),
                pickup.item_name.to_string(),
                type_str.to_string(),
                pickup.quantity.to_string(),
                pickup.region.to_string(),
                tile_str,
            ]);

            let mut row = RowData::new(cells);

            // Color based on collected status and mismatches
            if is_selected {
                row = row.with_color(Color32::YELLOW);
            } else if has_mismatch {
                row = row.with_color(Color32::from_rgb(255, 165, 0)); // Orange for mismatch
            } else {
                let color = match is_collected {
                    Some(true) => Color32::from_rgb(100, 200, 100), // Green for collected
                    Some(false) => Color32::LIGHT_GRAY,
                    None => Color32::from_rgb(180, 180, 180), // Dim for unknown
                };
                row = row.with_color(color);
            }

            row
        }).collect();

        // Build columns dynamically based on available data
        let mut columns = vec![];
        // Build columns with auto-width (icon columns keep small width)
        if event_flags.is_some() {
            columns.push(Column::new("status", "Flag").sortable(true).center().icon());
        }
        if inventory.is_some() {
            columns.push(Column::new("inv", "Inv").center().icon());
        }
        columns.extend(vec![
            Column::new("lot_id", "Lot ID").sortable(true).monospace(true),
            Column::new("flag_id", "Flag ID").sortable(true).monospace(true),
            Column::new("item", "Item").sortable(true),
            Column::new("type", "Type").sortable(true),
            Column::new("qty", "Qty").sortable(true).right(),
            Column::new("region", "Region").sortable(true),
            Column::new("tile", "Tile").monospace(true),
        ]);

        // Show table
        let table_response = UnifiedTable::new("world_pickups_table", &mut state.table_state)
            .columns(columns)
            .rows(rows)
            .zebra_stripe(true)
            .selectable(true)
            .show(ui);

        // Handle copy
        if let Some(text) = table_response.clipboard_text {
            ui.output_mut(|o| o.copied_text = text);
        }

        // Handle single click - open detail panel
        if let Some(row_idx) = table_response.clicked_row {
            if let Some((id, pickup, _, _)) = pickups.get(row_idx) {
                let type_str = match pickup.item_type {
                    PickupItemType::Weapon => "Weapon",
                    PickupItemType::Armor => "Armor",
                    PickupItemType::Accessory => "Accessory",
                    PickupItemType::Good => "Good",
                    PickupItemType::AshOfWar => "Ash of War",
                    PickupItemType::Unknown => "Unknown",
                };

                let mut sections = Vec::new();

                // Add location info
                sections.push(
                    RelationshipSection::new("Location").with_items(vec![
                        RelationshipItem::new(
                            pickup.region.to_string(),
                            DetailPanelAction::None,
                        ).with_secondary(format!("Tile ({}, {})", pickup.tile_x, pickup.tile_y)),
                    ])
                );

                // Add item type info
                sections.push(
                    RelationshipSection::new("Details").with_items(vec![
                        RelationshipItem::new(
                            format!("Type: {}", type_str),
                            DetailPanelAction::None,
                        ),
                        RelationshipItem::new(
                            format!("Quantity: {}", pickup.quantity),
                            DetailPanelAction::None,
                        ),
                        RelationshipItem::new(
                            format!("Flag ID: {}", pickup.flag_id),
                            DetailPanelAction::None,
                        ),
                    ])
                );

                detail_panel.select_with_relationships(
                    SelectedEntity::Pickup {
                        flag_id: pickup.flag_id,
                        item_name: pickup.item_name.to_string(),
                    },
                    sections,
                );
                state.last_detail_id = Some(*id);
            }
        }

        // Handle double-click - copy row data
        if let Some(row_idx) = table_response.double_clicked_row {
            if let Some((id, pickup, _, _)) = pickups.get(row_idx) {
                let row_text = format!(
                    "{}\t{}\t{}\t{}\t{}",
                    id, pickup.flag_id, pickup.item_name, pickup.quantity, pickup.region
                );
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Update selected_id
        if state.table_state.selection_count() == 1 {
            if let Some(&idx) = state.table_state.selected_rows.iter().next() {
                if let Some((id, _, _, _)) = pickups.get(idx) {
                    state.selected_id = Some(*id);
                }
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let data_to_export: Vec<_> = pickups.iter()
                .map(|(id, pickup, is_collected, inv_status)| WorldPickupExportItem {
                    lot_id: *id,
                    flag_id: pickup.flag_id,
                    item_name: pickup.item_name.to_string(),
                    item_type: match pickup.item_type {
                        PickupItemType::Weapon => "Weapon".to_string(),
                        PickupItemType::Armor => "Armor".to_string(),
                        PickupItemType::Accessory => "Accessory".to_string(),
                        PickupItemType::Good => "Good".to_string(),
                        PickupItemType::AshOfWar => "Ash of War".to_string(),
                        PickupItemType::Unknown => "Unknown".to_string(),
                    },
                    quantity: pickup.quantity as i32,
                    region: pickup.region.to_string(),
                    tile: format!("({}, {})", pickup.tile_x, pickup.tile_y),
                    collected: *is_collected,
                    in_inventory: inv_status.map(|(has, _)| has),
                })
                .collect();

            let content = match state.export_format {
                ExportFormat::Json => {
                    let export = PageExport::new(
                        PageExportMetadata::new("World Pickups")
                            .with_counts(total_count, filtered_count),
                        &data_to_export,
                    );
                    to_json(&export).unwrap_or_else(|_| String::new())
                }
                ExportFormat::Csv => {
                    let headers = &["Lot ID", "Flag ID", "Item", "Type", "Qty", "Region", "Tile", "Collected", "In Inventory"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|p| vec![
                            p.lot_id.to_string(),
                            p.flag_id.to_string(),
                            p.item_name.clone(),
                            p.item_type.clone(),
                            p.quantity.to_string(),
                            p.region.clone(),
                            p.tile.clone(),
                            p.collected.map(|c| if c { "Yes" } else { "No" }.to_string()).unwrap_or_default(),
                            p.in_inventory.map(|c| if c { "Yes" } else { "No" }.to_string()).unwrap_or_default(),
                        ])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["Lot ID", "Flag ID", "Item", "Type", "Qty", "Region", "Tile", "Collected", "In Inventory"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|p| vec![
                            p.lot_id.to_string(),
                            p.flag_id.to_string(),
                            p.item_name.clone(),
                            p.item_type.clone(),
                            p.quantity.to_string(),
                            p.region.clone(),
                            p.tile.clone(),
                            p.collected.map(|c| if c { "Yes" } else { "No" }.to_string()).unwrap_or_default(),
                            p.in_inventory.map(|c| if c { "Yes" } else { "No" }.to_string()).unwrap_or_default(),
                        ])
                        .collect();
                    to_markdown(headers, &rows)
                }
            };

            if export_response.copy_clicked {
                ui.output_mut(|o| o.copied_text = content);
            }
        }
    }
}
