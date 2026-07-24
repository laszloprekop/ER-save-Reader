pub mod events {

    use eframe::egui::{self, Ui, Color32, RichText};
    use serde::Serialize;
    use crate::{db::{bosses::bosses::BOSSES, colosseums::colosseums::COLOSSEUMS, cookbooks::books::COOKBOKS, graces::maps::GRACES, landmarks::landmarks::LANDMARKS, map_name::map_name::MAP_NAME, maps::maps::MAPS, summoning_pools::summoning_pools::SUMMONING_POOLS, whetblades::whetblades::WHETBLADES, pickup_data::{WORLD_PICKUPS, PickupCategory}, pickup_flags::get_flag_verification_status, dungeon_pickups::{DUNGEON_PICKUPS, get_dungeon_area_name}, item_name::item_name::ITEM_NAME, weapon_name::weapon_name::WEAPON_NAME, armor_name::armor_name::ARMOR_NAME, accessory_name::accessory_name::ACCESSORY_NAME, aow_name::aow_name::AOW_NAME}, db::inventory_verification::{UNIQUE_ITEMS_BY_FLAG, VerificationConfidence}, ui::{verification_view::verification_view::{verification_view, inventory_verification_summary}, style::{TABLE_MONO_SIZE, spacer}, components::{legend::icons, table::{UnifiedTable, Column, RowData, SortDirection}, filter::{FilterBar, FilterOption, fuzzy_match_default}, export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown}}, tokens::{colors, spacing}}, vm::{events::events_view_model::{EventsRoute, PickupTypeFilter, CollectedFilter, SimpleEventFlagViewState}, vm::vm::ViewModel, character::character::Character, screen_state::screen_state::ScreenState}};
    use wasm_event_flags::{FlagState, ResolvedFlags};
    use crate::save::common::save_slot::EquipInventoryData;

    type PickupRow<'a> = (
        &'a crate::db::pickup_data::WorldPickup,
        FlagState,
        Option<(bool, VerificationConfidence)>,
    );

    /// Icon size multiplier for table icons (150%)
    const ICON_SIZE_MULTIPLIER: f32 = 1.5;

    pub fn events(ui: &mut Ui, vm: &mut ViewModel, event_flags: Option<&[u8]>, inventory: Option<&EquipInventoryData>, storage: Option<&EquipInventoryData>, save_path: &str) {
        // Split the active slot into its immutable reconstruction (`ch`, holding one
        // ResolvedFlags for the whole render) and its mutable widget state (`ss`).
        // Every view below reads flags through `ch`, so the origin resolves once here
        // instead of once per view (Workstream D2).
        let idx = vm.index;
        let (ch, ss) = vm.slots[idx].split(idx, event_flags);

        // Right sidebar for flag details (only show when a flag is selected in world/dungeon pickups)
        let selected_flag = match ss.current_route {
            EventsRoute::WorldPickups => ss.world_pickups_filter.selected_flag_id,
            EventsRoute::DungeonPickups => ss.dungeon_pickups_filter.selected_flag_id,
            _ => None,
        };

        if selected_flag.is_some() {
            egui::SidePanel::right("flag_details_panel")
                .default_width(280.0)
                .min_width(200.0)
                .show(ui.ctx(), |ui| {
                    flag_details_sidebar(ui, &ch, ss, inventory, storage, save_path);
                });
        }

        // Content renders directly into the provided ui (wrapped in scroll area)
        egui::ScrollArea::vertical()
            .id_salt("events_content")
            .auto_shrink(false)
            .show(ui, |ui| {
                match ss.current_route {
                    EventsRoute::None => {
                        ui.centered_and_justified(|ui| {
                            ui.label("Select an Event Flags category from the navigation bar above");
                        });
                    },
                    EventsRoute::SitesOfGrace => {graces(ui, &ch, ss);},
                    EventsRoute::Whetblades => {whetblades(ui, &ch, ss);},
                    EventsRoute::Cookboks => {cookbooks(ui, &ch, ss);},
                    EventsRoute::Maps => {maps(ui, &ch, ss);},
                    EventsRoute::Bosses => {bosses(ui, &ch, ss);},
                    EventsRoute::SummoningPools => {summoning_pools(ui, &ch, ss);},
                    EventsRoute::Colosseums => {colosseums(ui, &ch, ss);},
                    EventsRoute::Landmarks => {landmarks_view(ui, &ch, ss);},
                    EventsRoute::WorldPickups => {world_pickups(ui, &ch, ss, inventory);},
                    EventsRoute::DungeonPickups => {dungeon_pickups(ui, &ch, ss);},
                    EventsRoute::Verification => {
                        // Inventory Verification Triangle section first
                        if inventory.is_some() || ch.flag_bytes().is_some() {
                            let set_flags = collect_set_flags(ch.flag_bytes());
                            inventory_verification_summary(ui, &set_flags, inventory);
                            spacer(ui);
                            ui.add_space(10.0);
                        }

                        // Existing flag verification view
                        verification_view(ui, &mut ss.verification_vm);
                    },
                }
            });
    }

    /// Export item structure for graces
    #[derive(Serialize)]
    struct GraceExportItem {
        name: String,
        region: String,
        flag_id: u32,
        status: String,
    }

    fn graces(ui: &mut Ui, ch: &Character, ss: &mut ScreenState) {
        let graces_data = &ch.events().graces;
        let state = &mut ss.graces_view_state;

        // Build region filter options
        let map_name_lock = MAP_NAME.lock().unwrap();
        let mut regions: Vec<&str> = map_name_lock.values().cloned().collect();
        regions.sort();

        let region_options: Vec<FilterOption> = std::iter::once(FilterOption::all())
            .chain(regions.iter().map(|r| FilterOption::from_str(*r)))
            .collect();

        // Sync filter state
        state.region_filter = state.filter_state.category.clone();
        state.search = state.filter_state.search.clone();

        // Filter bar with region dropdown and search
        FilterBar::new("graces_filter", &mut state.filter_state)
            .category("Region", region_options)
            .search("Search graces...")
            .show(ui);

        spacing::space_sm(ui);

        // Status filter chips (Discovered / Not Discovered / Unreliable)
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut state.collected_filter, CollectedFilter::All, "All");
            ui.selectable_value(&mut state.collected_filter, CollectedFilter::Collected, "Discovered");
            ui.selectable_value(&mut state.collected_filter, CollectedFilter::NotCollected, "Not Discovered");
            ui.selectable_value(&mut state.collected_filter, CollectedFilter::Unverified, "Unreliable");
        });

        spacing::space_sm(ui);

        // Export toolbar
        let has_filters = state.filter_state.has_active_filters() || state.collected_filter != CollectedFilter::All;
        let export_response = ExportToolbar::new("graces_export", &mut state.export_format, &mut state.export_filtered_only)
            .has_filters(has_filters)
            .show(ui);

        spacing::space_sm(ui);

        // Get filter values
        let region_filter = state.region_filter.clone();
        let search = state.search.clone();
        let collected_filter = state.collected_filter;
        let export_format = state.export_format;

        let graces_lookup = GRACES.lock().unwrap();

        // Build filtered data - flat list with region column
        let mut items: Vec<(&crate::db::graces::maps::Grace, &str, &str, u32, FlagState)> = graces_data.iter()
            .filter_map(|(grace, &status)| {
                let info = graces_lookup.get(grace)?;
                let region_name = map_name_lock.get(&info.0).cloned().unwrap_or("Unknown");
                let name = info.2;
                let flag_id = info.1;

                // Apply collected filter. Set = discovered, Unknown = the origin
                // could not be resolved (shown as its own "unverified" state).
                match collected_filter {
                    CollectedFilter::All => {},
                    CollectedFilter::Collected => if status != FlagState::Set { return None; },
                    CollectedFilter::NotCollected => if status != FlagState::Clear { return None; },
                    CollectedFilter::Unverified => if status != FlagState::Unknown { return None; },
                }

                // Apply region filter
                if region_filter != "All" && region_name != region_filter {
                    return None;
                }

                // Apply search
                if !search.is_empty() {
                    let matches = fuzzy_match_default(name, &search) || fuzzy_match_default(region_name, &search);
                    if !matches {
                        return None;
                    }
                }

                Some((grace, name, region_name, flag_id, status))
            })
            .collect();

        // Apply sorting
        if let Some(sort_col) = &state.table_state.sort_column {
            let asc = state.table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "name" => items.sort_by(|a, b| if asc { a.1.cmp(b.1) } else { b.1.cmp(a.1) }),
                "region" => items.sort_by(|a, b| if asc { a.2.cmp(b.2) } else { b.2.cmp(a.2) }),
                "flag_id" => items.sort_by(|a, b| if asc { a.3.cmp(&b.3) } else { b.3.cmp(&a.3) }),
                "status" => items.sort_by(|a, b| {
                    let rank = |s: FlagState| match s { FlagState::Set => 2, FlagState::Unknown => 1, FlagState::Clear => 0 };
                    let (sa, sb) = (rank(a.4), rank(b.4));
                    if asc { sa.cmp(&sb) } else { sb.cmp(&sa) }
                }),
                _ => {}
            }
        }

        // Summary
        let discovered_count = graces_data.values().filter(|&&v| v == FlagState::Set).count();
        let unreliable_count = graces_data.values().filter(|&&v| v == FlagState::Unknown).count();
        let total_count = graces_data.len();
        let filtered_count = items.len();
        let reliable_total = total_count - unreliable_count;

        let summary = if filtered_count < total_count {
            if unreliable_count > 0 {
                format!("Sites of Grace: {}/{} discovered ({} unreliable) - showing {}", discovered_count, reliable_total, unreliable_count, filtered_count)
            } else {
                format!("Sites of Grace: {}/{} discovered - showing {}", discovered_count, total_count, filtered_count)
            }
        } else if unreliable_count > 0 {
            format!("Sites of Grace: {}/{} discovered ({} unreliable)", discovered_count, reliable_total, unreliable_count)
        } else {
            format!("Sites of Grace: {}/{} discovered", discovered_count, total_count)
        };
        ui.label(RichText::new(&summary).strong());

        spacing::space_sm(ui);

        // Build row data
        let rows: Vec<RowData> = items.iter().map(|(_, name, region, flag_id, status)| {
            let (status_icon, row_color) = match status {
                FlagState::Set => (icons::COLLECTED, colors::STATUS_COLLECTED),
                FlagState::Clear => (icons::NOT_COLLECTED, Color32::LIGHT_GRAY),
                FlagState::Unknown => (icons::UNKNOWN, colors::STATUS_WARNING),
            };

            RowData::new(vec![
                status_icon.to_string(),
                name.to_string(),
                region.to_string(),
                flag_id.to_string(),
            ]).with_color(row_color)
        }).collect();

        // Define columns
        let columns = vec![
            Column::new("status", "Status").width(50.0).sortable(true).center().icon(),
            Column::new("name", "Name").width_fraction(0.35).sortable(true),
            Column::new("region", "Region").width_fraction(0.25).sortable(true),
            Column::new("flag_id", "Flag ID").width(100.0).sortable(true).monospace(true),
        ];

        // Show table
        let table_response = UnifiedTable::new("graces_table", &mut state.table_state)
            .columns(columns)
            .rows(rows)
            .zebra_stripe(true)
            .selectable(true)
            .show(ui);

        // Handle copy
        if let Some(text) = table_response.clipboard_text {
            ui.output_mut(|o| o.copied_text = text);
        }

        // Handle double-click to copy row
        if let Some(row_idx) = table_response.double_clicked_row {
            if let Some((_, name, region, flag_id, status)) = items.get(row_idx) {
                let status_text = match status {
                    FlagState::Set => "Discovered",
                    FlagState::Clear => "Not discovered",
                    FlagState::Unknown => "Unreliable",
                };
                let row_text = format!("{}\t{}\t{}\t{}", status_text, name, region, flag_id);
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let export_data: Vec<GraceExportItem> = items.iter()
                .map(|(_, name, region, flag_id, status)| GraceExportItem {
                    name: name.to_string(),
                    region: region.to_string(),
                    flag_id: *flag_id,
                    status: match status {
                        FlagState::Set => "Discovered",
                        FlagState::Clear => "Not Discovered",
                        FlagState::Unknown => "Unreliable",
                    }.to_string(),
                })
                .collect();

            let content = match export_format {
                ExportFormat::Json => {
                    let export = PageExport::new(
                        PageExportMetadata::new("Sites of Grace")
                            .with_counts(total_count, filtered_count),
                        &export_data,
                    );
                    to_json(&export).unwrap_or_default()
                }
                ExportFormat::Csv => {
                    let headers = &["Name", "Region", "Flag ID", "Status"];
                    let rows: Vec<Vec<String>> = export_data.iter()
                        .map(|item| vec![
                            item.name.clone(),
                            item.region.clone(),
                            item.flag_id.to_string(),
                            item.status.clone(),
                        ])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["Name", "Region", "Flag ID", "Status"];
                    let rows: Vec<Vec<String>> = export_data.iter()
                        .map(|item| vec![
                            item.name.clone(),
                            item.region.clone(),
                            item.flag_id.to_string(),
                            item.status.clone(),
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

    fn whetblades(ui: &mut Ui, ch: &Character, ss: &mut ScreenState) {
        let whetblades_data = &ch.events().whetblades;
        let state = &mut ss.whetblades_view_state;

        let whetblades_lookup = WHETBLADES.lock().unwrap();

        simple_event_flag_view(
            ui,
            "whetblades",
            "Whetblades",
            "discovered",
            whetblades_data,
            state,
            |key| whetblades_lookup.get(key).map(|info| (info.1, info.0)),
        );
    }

    fn cookbooks(ui: &mut Ui, ch: &Character, ss: &mut ScreenState) {
        let cookbooks_data = &ch.events().cookbooks;
        let state = &mut ss.cookbooks_view_state;

        let cookbooks_lookup = COOKBOKS.lock().unwrap();

        simple_event_flag_view(
            ui,
            "cookbooks",
            "Cookbooks",
            "discovered",
            cookbooks_data,
            state,
            |key| cookbooks_lookup.get(key).map(|info| (info.1, info.0)),
        );
    }

    fn maps(ui: &mut Ui, ch: &Character, ss: &mut ScreenState) {
        let maps_data = &ch.events().maps;
        let state = &mut ss.maps_view_state;

        let maps_lookup = MAPS.lock().unwrap();

        simple_event_flag_view(
            ui,
            "maps",
            "Maps",
            "discovered",
            maps_data,
            state,
            |key| maps_lookup.get(key).map(|info| (info.1, info.0)),
        );
    }

    fn bosses(ui: &mut Ui, ch: &Character, ss: &mut ScreenState) {
        let bosses_data = &ch.events().bosses;
        let state = &mut ss.bosses_view_state;

        let bosses_lookup = BOSSES.lock().unwrap();

        simple_event_flag_view(
            ui,
            "bosses",
            "Bosses",
            "defeated",
            bosses_data,
            state,
            |key| bosses_lookup.get(key).map(|info| (info.1, info.0)),
        );
    }

    fn summoning_pools(ui: &mut Ui, ch: &Character, ss: &mut ScreenState) {
        let pools_data = &ch.events().summoning_pools;
        let state = &mut ss.summoning_pools_view_state;

        let pools_lookup = SUMMONING_POOLS.lock().unwrap();

        simple_event_flag_view(
            ui,
            "summoning_pools",
            "Summoning Pools",
            "discovered",
            pools_data,
            state,
            |key| pools_lookup.get(key).map(|info| (info.1, info.0)),
        );
    }

    fn colosseums(ui: &mut Ui, ch: &Character, ss: &mut ScreenState) {
        let colosseums_data = &ch.events().colosseums;
        let state = &mut ss.colosseums_view_state;

        let colosseums_lookup = COLOSSEUMS.lock().unwrap();

        simple_event_flag_view(
            ui,
            "colosseums",
            "Colosseums",
            "discovered",
            colosseums_data,
            state,
            |key| colosseums_lookup.get(key).map(|info| (info.1, info.0)),
        );
    }

    fn landmarks_view(ui: &mut Ui, ch: &Character, ss: &mut ScreenState) {
        let landmarks_data = &ch.events().landmarks;
        let state = &mut ss.landmarks_view_state;

        let landmarks_lookup = LANDMARKS.lock().unwrap();

        simple_event_flag_view(
            ui,
            "landmarks",
            "Landmarks",
            "discovered",
            landmarks_data,
            state,
            |key| landmarks_lookup.get(key).map(|info| (info.1, info.0)),
        );
    }

    /// Display an event row with icon-based status
    fn display_event_row_with_icon(ui: &mut Ui, name: &str, flag_id: u32, discovered: bool) {
        let verification_status = get_flag_verification_status(flag_id);
        let is_unverified = verification_status.is_uncertain();

        // Determine status icon and color
        let (status_icon, status_color, status_text) = if is_unverified {
            (icons::MISMATCH, colors::STATUS_WARNING, "Unverified")
        } else if discovered {
            (icons::COLLECTED, colors::STATUS_COLLECTED, "Discovered")
        } else {
            (icons::NOT_COLLECTED, Color32::LIGHT_GRAY, "Not discovered")
        };

        let row_text = format!("{} | {}", name, flag_id);
        let full_row_text = format!("{} | {} | {}", status_text, name, flag_id);

        let response = ui.horizontal(|ui| {
            // Status icon at 150% size
            ui.label(RichText::new(status_icon)
                .color(status_color)
                .size(TABLE_MONO_SIZE * ICON_SIZE_MULTIPLIER));
            ui.add(
                egui::Label::new(RichText::new(&row_text).color(status_color).monospace().size(TABLE_MONO_SIZE))
                    .sense(egui::Sense::click())
            )
        }).inner;

        if response.double_clicked() {
            ui.output_mut(|o| o.copied_text = full_row_text.clone());
        }

        response.context_menu(|ui| {
            if ui.button("Copy row").clicked() {
                ui.output_mut(|o| o.copied_text = full_row_text.clone());
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

    /// Export item structure for simple event flag pages
    #[derive(Serialize)]
    struct SimpleEventFlagExportItem {
        name: String,
        flag_id: u32,
        discovered: bool,
    }

    /// Generic view function for simple event flag pages
    /// Renders FilterBar + UnifiedTable + ExportToolbar pattern
    fn simple_event_flag_view<K, F>(
        ui: &mut Ui,
        page_id: &str,
        page_name: &str,
        status_verb: &str,  // "discovered" or "defeated"
        data: &std::collections::BTreeMap<K, bool>,
        state: &mut SimpleEventFlagViewState,
        lookup_fn: F,
    ) where
        K: Copy + Ord,
        F: Fn(&K) -> Option<(&'static str, u32)>,  // Returns (name, flag_id)
    {
        // Sync filter state
        state.search = state.filter_state.search.clone();

        // Filter bar with search only (simple pages don't have categories)
        FilterBar::new(format!("{}_filter", page_id), &mut state.filter_state)
            .search("Search...")
            .show(ui);

        spacing::space_sm(ui);

        // Status filter chips
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut state.collected_filter, CollectedFilter::All, "All");
            ui.selectable_value(&mut state.collected_filter, CollectedFilter::Collected,
                if status_verb == "defeated" { "Defeated" } else { "Discovered" });
            ui.selectable_value(&mut state.collected_filter, CollectedFilter::NotCollected,
                if status_verb == "defeated" { "Not Defeated" } else { "Not Discovered" });
        });

        spacing::space_sm(ui);

        // Export toolbar
        let has_filters = state.filter_state.has_active_filters() || state.collected_filter != CollectedFilter::All;
        let export_response = ExportToolbar::new(format!("{}_export", page_id), &mut state.export_format, &mut state.export_filtered_only)
            .has_filters(has_filters)
            .show(ui);

        spacing::space_sm(ui);

        // Build filtered data
        let search = state.search.clone();
        let collected_filter = state.collected_filter;
        let export_format = state.export_format;

        let mut items: Vec<(&K, &str, u32, bool)> = data.iter()
            .filter_map(|(key, &discovered)| {
                let (name, flag_id) = lookup_fn(key)?;

                // Apply collected filter
                match collected_filter {
                    CollectedFilter::All => {},
                    CollectedFilter::Collected => if !discovered { return None; },
                    CollectedFilter::NotCollected => if discovered { return None; },
                    CollectedFilter::Unverified => {
                        // For simple flags, use the verification status
                        let status = get_flag_verification_status(flag_id);
                        if !status.is_uncertain() { return None; }
                    },
                }

                // Apply search
                if !search.is_empty() && !fuzzy_match_default(name, &search) {
                    return None;
                }

                Some((key, name, flag_id, discovered))
            })
            .collect();

        // Apply sorting
        if let Some(sort_col) = &state.table_state.sort_column {
            let asc = state.table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "name" => items.sort_by(|a, b| if asc { a.1.cmp(b.1) } else { b.1.cmp(a.1) }),
                "flag_id" => items.sort_by(|a, b| if asc { a.2.cmp(&b.2) } else { b.2.cmp(&a.2) }),
                "status" => items.sort_by(|a, b| {
                    let sa = if a.3 { 1 } else { 0 };
                    let sb = if b.3 { 1 } else { 0 };
                    if asc { sa.cmp(&sb) } else { sb.cmp(&sa) }
                }),
                _ => {}
            }
        }

        // Summary
        let discovered_count = data.values().filter(|v| **v).count();
        let total_count = data.len();
        let filtered_count = items.len();

        if filtered_count < total_count {
            ui.label(RichText::new(format!("{}: {}/{} {} (showing {})", page_name, discovered_count, total_count, status_verb, filtered_count)).strong());
        } else {
            ui.label(RichText::new(format!("{}: {}/{} {}", page_name, discovered_count, total_count, status_verb)).strong());
        }

        spacing::space_sm(ui);

        // Build row data
        let rows: Vec<RowData> = items.iter().map(|(_, name, flag_id, discovered)| {
            let status = get_flag_verification_status(*flag_id);
            let is_unverified = status.is_uncertain();

            let status_icon = if is_unverified {
                icons::MISMATCH
            } else if *discovered {
                icons::COLLECTED
            } else {
                icons::NOT_COLLECTED
            };

            let row_color = if is_unverified {
                colors::STATUS_WARNING
            } else if *discovered {
                colors::STATUS_COLLECTED
            } else {
                Color32::LIGHT_GRAY
            };

            RowData::new(vec![
                status_icon.to_string(),
                name.to_string(),
                flag_id.to_string(),
            ]).with_color(row_color)
        }).collect();

        // Define columns
        let columns = vec![
            Column::new("status", "Status").width(50.0).sortable(true).center().icon(),
            Column::new("name", "Name").width_fraction(0.5).sortable(true),
            Column::new("flag_id", "Flag ID").width(100.0).sortable(true).monospace(true),
        ];

        // Show table
        let table_response = UnifiedTable::new(format!("{}_table", page_id), &mut state.table_state)
            .columns(columns)
            .rows(rows)
            .zebra_stripe(true)
            .selectable(true)
            .show(ui);

        // Handle copy
        if let Some(text) = table_response.clipboard_text {
            ui.output_mut(|o| o.copied_text = text);
        }

        // Handle double-click to copy row
        if let Some(row_idx) = table_response.double_clicked_row {
            if let Some((_, name, flag_id, discovered)) = items.get(row_idx) {
                let status_text = if *discovered {
                    if status_verb == "defeated" { "Defeated" } else { "Discovered" }
                } else if status_verb == "defeated" { "Not defeated" } else { "Not discovered" };
                let row_text = format!("{}\t{}\t{}", status_text, name, flag_id);
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let export_data: Vec<SimpleEventFlagExportItem> = items.iter()
                .map(|(_, name, flag_id, discovered)| SimpleEventFlagExportItem {
                    name: name.to_string(),
                    flag_id: *flag_id,
                    discovered: *discovered,
                })
                .collect();

            let content = match export_format {
                ExportFormat::Json => {
                    let export = PageExport::new(
                        PageExportMetadata::new(page_name)
                            .with_counts(total_count, filtered_count),
                        &export_data,
                    );
                    to_json(&export).unwrap_or_default()
                }
                ExportFormat::Csv => {
                    let headers = &["Name", "Flag ID", if status_verb == "defeated" { "Defeated" } else { "Discovered" }];
                    let rows: Vec<Vec<String>> = export_data.iter()
                        .map(|item| vec![
                            item.name.clone(),
                            item.flag_id.to_string(),
                            if item.discovered { "Yes" } else { "No" }.to_string(),
                        ])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["Name", "Flag ID", if status_verb == "defeated" { "Defeated" } else { "Discovered" }];
                    let rows: Vec<Vec<String>> = export_data.iter()
                        .map(|item| vec![
                            item.name.clone(),
                            item.flag_id.to_string(),
                            if item.discovered { "Yes" } else { "No" }.to_string(),
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

    /// Pickup state in the (collected, status) shape this view already renders.
    ///
    /// Read one pickup's state from an already-resolved region, routing by family
    /// (`WORLD_PICKUPS` mixes open-world tiles, legacy-map pickups and
    /// world-state-b flags). `resolved` is `None` when the save's origin would not
    /// resolve, so every read is `Unknown` — which the table shows as its own
    /// state, never collapsed into "not collected".
    ///
    /// Returns `FlagState`, not a `(bool, _)` pair, deliberately: the detail panel
    /// used to take `.0` of the pair and render `Unknown` as "not collected". A
    /// tri-state that cannot be indexed into a bool cannot be misread that way.
    fn pickup_state(resolved: Option<&ResolvedFlags>, flag_id: u32) -> FlagState {
        resolved.map_or(FlagState::Unknown, |r| {
            crate::db::pickup_flags::pickup_state(r, flag_id)
        })
    }

    fn world_pickups(ui: &mut Ui, ch: &Character, ss: &mut ScreenState, inventory: Option<&EquipInventoryData>) {
        let filter = &mut ss.world_pickups_filter;

        // Build region filter options
        let mut regions: Vec<&str> = WORLD_PICKUPS.iter()
            .map(|p| p.region)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        regions.sort();

        let region_options: Vec<FilterOption> = std::iter::once(FilterOption::all())
            .chain(regions.iter().map(|r| FilterOption::from_str(*r)))
            .collect();

        // Sync filter state
        filter.region_filter = filter.filter_state.category.clone();
        filter.search = filter.filter_state.search.clone();

        // Filter bar with region dropdown and search
        FilterBar::new("char_world_pickups_filter", &mut filter.filter_state)
            .category("Region", region_options)
            .search("Search items...")
            .show(ui);

        spacing::space_sm(ui);

        // Type filter chips
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

        // Collected filter row
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::All, "All");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::Collected, "Collected");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::NotCollected, "Not Collected");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::Unverified, "Unverified");
        });

        spacing::space_sm(ui);

        // Export toolbar
        let export_response = ExportToolbar::new("char_world_pickups_export", &mut filter.export_format, &mut filter.export_filtered_only)
            .has_filters(filter.filter_state.has_active_filters() || filter.type_filter != PickupTypeFilter::All || filter.collected_filter != CollectedFilter::All)
            .show(ui);

        spacing::space_sm(ui);

        // Get current filter values (to avoid borrow issues)
        let type_filter = filter.type_filter;
        let collected_filter = filter.collected_filter;
        let region_filter = filter.region_filter.clone();
        let search = filter.search.clone();

        // The origin is resolved ONCE on the Character (~13,400-byte scan), not per
        // row and not per view. `ch.flags()` is `None` when the origin did not
        // resolve, so every read is Unknown.
        // Build filtered data with inventory status
        let mut pickups: Vec<PickupRow<'_>> = WORLD_PICKUPS.iter()
            .filter_map(|pickup| {
                // CUT OVER 2026-07-20 (ADR-0006). Resolved per save and routed by
                // family; `Unknown` is its own state, which the table renders
                // distinctly rather than as "not collected".
                let state = pickup_state(ch.flags(), pickup.event_flag);

                // Apply collected filter
                match collected_filter {
                    CollectedFilter::All => {},
                    CollectedFilter::Collected => if state != FlagState::Set { return None; },
                    CollectedFilter::NotCollected => if state != FlagState::Clear { return None; },
                    CollectedFilter::Unverified => if state != FlagState::Unknown { return None; },
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
                    return None;
                }

                // Apply region filter
                if region_filter != "All" && pickup.region != region_filter {
                    return None;
                }

                // Apply search using fuzzy match
                if !search.is_empty() {
                    let matches = fuzzy_match_default(pickup.name, &search)
                        || fuzzy_match_default(pickup.region, &search);
                    if !matches {
                        return None;
                    }
                }

                // Check inventory status
                let inv_status = is_item_in_inventory(pickup.event_flag, inventory);

                Some((pickup, state, inv_status))
            })
            .collect();

        // Apply sorting
        let table_state = &ss.world_pickups_filter.table_state;
        if let Some(sort_col) = &table_state.sort_column {
            let asc = table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "lot_id" => pickups.sort_by(|a, b| if asc { a.0.item_lot_id.cmp(&b.0.item_lot_id) } else { b.0.item_lot_id.cmp(&a.0.item_lot_id) }),
                "flag_id" => pickups.sort_by(|a, b| if asc { a.0.event_flag.cmp(&b.0.event_flag) } else { b.0.event_flag.cmp(&a.0.event_flag) }),
                "item" => pickups.sort_by(|a, b| if asc { a.0.name.cmp(b.0.name) } else { b.0.name.cmp(a.0.name) }),
                "category" => pickups.sort_by(|a, b| {
                    let ca = a.0.category.display_name();
                    let cb = b.0.category.display_name();
                    if asc { ca.cmp(cb) } else { cb.cmp(ca) }
                }),
                "qty" => pickups.sort_by(|a, b| if asc { a.0.quantity.cmp(&b.0.quantity) } else { b.0.quantity.cmp(&a.0.quantity) }),
                "region" => pickups.sort_by(|a, b| if asc { a.0.region.cmp(b.0.region) } else { b.0.region.cmp(a.0.region) }),
                "status" => pickups.sort_by(|a, b| {
                    let rank = |s: FlagState| match s { FlagState::Set => 2, FlagState::Unknown => 1, FlagState::Clear => 0 };
                    let (sa, sb) = (rank(a.1), rank(b.1));
                    if asc { sa.cmp(&sb) } else { sb.cmp(&sa) }
                }),
                _ => {}
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
        let selected_flag_id = ss.world_pickups_filter.selected_flag_id;
        let rows: Vec<RowData> = pickups.iter().map(|(pickup, state, inv_status)| {
            let flag_str = match state {
                FlagState::Unknown => icons::MISMATCH,
                FlagState::Set => icons::COLLECTED,
                FlagState::Clear => icons::NOT_COLLECTED,
            };

            // Inventory status icon (same logic as world_pickups_view.rs)
            let inv_str = match inv_status {
                Some((true, VerificationConfidence::VeryHigh)) => icons::HIGH_CONFIDENCE,
                Some((true, VerificationConfidence::High)) => icons::COLLECTED,
                Some((true, _)) => icons::PARTIAL,
                Some((false, VerificationConfidence::VeryHigh)) => icons::NOT_COLLECTED,
                Some((false, VerificationConfidence::High)) => icons::NO_DATA,
                Some((false, _)) => icons::LOW_CONFIDENCE,
                None => icons::NO_DATA,
            };

            // Check for mismatch between flag and inventory. Only a resolved
            // flag can mismatch — Unknown asserts nothing to disagree with.
            let has_mismatch = match (state, inv_status) {
                (FlagState::Set, Some((false, _))) => true,  // Flag says collected but not in inventory
                (FlagState::Clear, Some((true, _))) => true, // Flag says not collected but in inventory
                _ => false,
            };

            let mut cells = vec![
                flag_str.to_string(),
            ];

            // Add Inv column if inventory data is available
            if inventory.is_some() {
                cells.push(inv_str.to_string());
            }

            cells.extend(vec![
                pickup.item_lot_id.to_string(),
                pickup.event_flag.to_string(),
                pickup.name.to_string(),
                pickup.category.display_name().to_string(),
                pickup.quantity.to_string(),
                pickup.region.to_string(),
            ]);

            let is_selected = selected_flag_id == Some(pickup.event_flag);

            let mut row = RowData::new(cells);

            // Color based on status and mismatch
            if is_selected {
                row = row.with_color(Color32::YELLOW);
            } else if has_mismatch {
                row = row.with_color(Color32::from_rgb(255, 165, 0)); // Orange for mismatch
            } else {
                row = match state {
                    FlagState::Unknown => row.with_color(colors::STATUS_WARNING),
                    FlagState::Set => row.with_color(colors::STATUS_COLLECTED),
                    FlagState::Clear => row.with_color(Color32::LIGHT_GRAY),
                };
            }

            row
        }).collect();

        // Build columns dynamically based on available data
        let mut columns = vec![
            Column::new("status", "Flag").width(40.0).sortable(true).center().icon(),
        ];
        if inventory.is_some() {
            columns.push(Column::new("inv", "Inv").width(40.0).center().icon());
        }
        columns.extend(vec![
            Column::new("lot_id", "Lot ID").width(80.0).sortable(true).monospace(true),
            Column::new("flag_id", "Flag ID").width(80.0).sortable(true).monospace(true),
            Column::new("item", "Item").width_fraction(0.25).sortable(true),
            Column::new("category", "Category").width(100.0).sortable(true),
            Column::new("qty", "Qty").width(40.0).sortable(true).right(),
            Column::new("region", "Region").width_fraction(0.15).sortable(true),
        ]);

        // Show table
        let table_state = &mut ss.world_pickups_filter.table_state;
        let table_response = UnifiedTable::new("char_world_pickups_table", table_state)
            .columns(columns)
            .rows(rows)
            .zebra_stripe(true)
            .selectable(true)
            .show(ui);

        // Handle copy
        if let Some(text) = table_response.clipboard_text {
            ui.output_mut(|o| o.copied_text = text);
        }

        // Handle row selection for details panel
        if table_response.sort_changed || ss.world_pickups_filter.table_state.selection_count() == 1 {
            if let Some(&idx) = ss.world_pickups_filter.table_state.selected_rows.iter().next() {
                if let Some((pickup, _, _)) = pickups.get(idx) {
                    ss.world_pickups_filter.selected_flag_id = Some(pickup.event_flag);
                }
            }
        }

        // Handle double-click
        if let Some(row_idx) = table_response.double_clicked_row {
            if let Some((pickup, state, inv_status)) = pickups.get(row_idx) {
                let status_text = match state {
                    FlagState::Unknown => "Unverified",
                    FlagState::Set => "Collected",
                    FlagState::Clear => "Not collected",
                };
                let inv_text = match inv_status {
                    Some((true, _)) => "In Inv",
                    Some((false, _)) => "Not in Inv",
                    None => "N/A",
                };
                let row_text = format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    status_text, inv_text, pickup.item_lot_id, pickup.event_flag, pickup.name,
                    pickup.category.display_name(), pickup.quantity, pickup.region
                );
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let content = build_world_pickups_export(&pickups, &ss.world_pickups_filter.export_format);

            if export_response.copy_clicked {
                ui.output_mut(|o| o.copied_text = content);
            }
        }
    }

    /// Build export content for world pickups
    fn build_world_pickups_export(
        pickups: &[PickupRow<'_>],
        format: &crate::ui::components::export::ExportFormat,
    ) -> String {
        use crate::ui::components::export::{ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
        use serde::Serialize;

        #[derive(Serialize)]
        struct ExportItem {
            lot_id: u32,
            flag_id: u32,
            item_name: String,
            category: String,
            quantity: u32,
            region: String,
            collected: bool,
            verified: bool,
            in_inventory: Option<bool>,
        }

        let data: Vec<ExportItem> = pickups.iter()
            .map(|(p, state, inv_status)| ExportItem {
                lot_id: p.item_lot_id,
                flag_id: p.event_flag,
                item_name: p.name.to_string(),
                category: p.category.display_name().to_string(),
                quantity: p.quantity,
                region: p.region.to_string(),
                collected: *state == FlagState::Set,
                verified: *state != FlagState::Unknown,
                in_inventory: inv_status.map(|(has, _)| has),
            })
            .collect();

        match format {
            ExportFormat::Json => {
                let export = PageExport::new(
                    PageExportMetadata::new("Character World Pickups")
                        .with_counts(crate::db::pickup_data::WORLD_PICKUPS.len(), pickups.len()),
                    &data,
                );
                to_json(&export).unwrap_or_else(|_| String::new())
            }
            ExportFormat::Csv => {
                let headers = &["Lot ID", "Flag ID", "Item", "Category", "Qty", "Region", "Collected", "Verified", "In Inventory"];
                let rows: Vec<Vec<String>> = data.iter()
                    .map(|p| vec![
                        p.lot_id.to_string(),
                        p.flag_id.to_string(),
                        p.item_name.clone(),
                        p.category.clone(),
                        p.quantity.to_string(),
                        p.region.clone(),
                        if p.collected { "Yes" } else { "No" }.to_string(),
                        if p.verified { "Yes" } else { "No" }.to_string(),
                        p.in_inventory.map(|has| if has { "Yes" } else { "No" }.to_string()).unwrap_or_default(),
                    ])
                    .collect();
                to_csv(headers, &rows)
            }
            ExportFormat::Markdown => {
                let headers = &["Lot ID", "Flag ID", "Item", "Category", "Qty", "Region", "Collected", "Verified", "In Inventory"];
                let rows: Vec<Vec<String>> = data.iter()
                    .map(|p| vec![
                        p.lot_id.to_string(),
                        p.flag_id.to_string(),
                        p.item_name.clone(),
                        p.category.clone(),
                        p.quantity.to_string(),
                        p.region.clone(),
                        if p.collected { "Yes" } else { "No" }.to_string(),
                        if p.verified { "Yes" } else { "No" }.to_string(),
                        p.in_inventory.map(|has| if has { "Yes" } else { "No" }.to_string()).unwrap_or_default(),
                    ])
                    .collect();
                to_markdown(headers, &rows)
            }
        }
    }

    /// Export item structure for dungeon pickups
    #[derive(Serialize)]
    struct DungeonPickupExportItem {
        flag_id: u32,
        item_name: String,
        category: String,
        quantity: u32,
        dungeon: String,
        collected: bool,
        verified: bool,
    }

    fn dungeon_pickups(ui: &mut Ui, ch: &Character, ss: &mut ScreenState) {
        use crate::db::dungeon_pickups::DungeonPickup;

        let filter = &mut ss.dungeon_pickups_filter;

        // Build dungeon filter options
        let mut dungeons: Vec<&str> = DUNGEON_PICKUPS.iter()
            .map(|p| get_dungeon_area_name(p.dungeon_area))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        dungeons.sort();

        let dungeon_options: Vec<FilterOption> = std::iter::once(FilterOption::all())
            .chain(dungeons.iter().map(|d| FilterOption::from_str(*d)))
            .collect();

        // Sync filter state
        filter.dungeon_filter = filter.filter_state.category.clone();
        filter.search = filter.filter_state.search.clone();

        // Filter bar with dungeon dropdown and search
        FilterBar::new("dungeon_pickups_filter", &mut filter.filter_state)
            .category("Dungeon", dungeon_options)
            .search("Search items...")
            .show(ui);

        spacing::space_sm(ui);

        // Type filter chips
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

        // Status filter chips
        ui.horizontal(|ui| {
            ui.label(RichText::new("Status:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::All, "All");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::Collected, "Collected");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::NotCollected, "Not Collected");
            ui.selectable_value(&mut filter.collected_filter, CollectedFilter::Unverified, "Unverified");
        });

        spacing::space_sm(ui);

        // Export toolbar
        let has_filters = filter.filter_state.has_active_filters()
            || filter.type_filter != PickupTypeFilter::All
            || filter.collected_filter != CollectedFilter::All;
        let export_response = ExportToolbar::new("dungeon_pickups_export", &mut filter.export_format, &mut filter.export_filtered_only)
            .has_filters(has_filters)
            .show(ui);

        spacing::space_sm(ui);

        // Get current filter values (to avoid borrow issues)
        let type_filter = filter.type_filter;
        let collected_filter = filter.collected_filter;
        let dungeon_filter = filter.dungeon_filter.clone();
        let search = filter.search.clone();
        let export_format = filter.export_format;

        // CUT OVER 2026-07-20 (ADR-0006, migration step 4). This used
        // DUNGEON_PICKUP_BASES — absolute per-area offsets from the frozen
        // legacy store — plus the pickup's own `dungeon_area`/`section` fields.
        // The base is now resolved per save from the flag region's origin, and
        // the position comes from the flag itself: the legacy-dungeon-pickup
        // family lays out as alloc_slot(map) * 1125 + local/8, and the map is
        // encoded in the flag id, so `dungeon_area`/`section` are display data
        // only and can no longer disagree with the flag they label.
        //
        // `FlagState::Unknown` — an unresolved origin, a map the game allocates
        // twice, or an id outside the family — is the table's own state, rendered
        // distinctly rather than as "not collected".
        fn is_dungeon_pickup_collected(resolved: Option<&ResolvedFlags>, pickup: &DungeonPickup) -> FlagState {
            resolved.map_or(FlagState::Unknown, |r| r.dungeon_pickup(pickup.event_flag))
        }

        // The origin is resolved once on the Character, shared by the whole table.
        // Build filtered data - flat list
        let mut items: Vec<(&DungeonPickup, &str, FlagState)> = DUNGEON_PICKUPS.iter()
            .filter_map(|pickup| {
                let state = is_dungeon_pickup_collected(ch.flags(), pickup);
                let dungeon_name = get_dungeon_area_name(pickup.dungeon_area);

                // Apply collected filter
                match collected_filter {
                    CollectedFilter::All => {},
                    CollectedFilter::Collected => if state != FlagState::Set { return None; },
                    CollectedFilter::NotCollected => if state != FlagState::Clear { return None; },
                    CollectedFilter::Unverified => if state != FlagState::Unknown { return None; },
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
                if !type_match { return None; }

                // Apply dungeon filter
                if dungeon_filter != "All" && dungeon_name != dungeon_filter {
                    return None;
                }

                // Apply search using fuzzy match
                if !search.is_empty() {
                    let matches = fuzzy_match_default(pickup.name, &search)
                        || fuzzy_match_default(dungeon_name, &search);
                    if !matches { return None; }
                }

                Some((pickup, dungeon_name, state))
            })
            .collect();

        // Apply sorting
        let table_state = &ss.dungeon_pickups_filter.table_state;
        if let Some(sort_col) = &table_state.sort_column {
            let asc = table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "flag_id" => items.sort_by(|a, b| if asc { a.0.event_flag.cmp(&b.0.event_flag) } else { b.0.event_flag.cmp(&a.0.event_flag) }),
                "item" => items.sort_by(|a, b| if asc { a.0.name.cmp(b.0.name) } else { b.0.name.cmp(a.0.name) }),
                "category" => items.sort_by(|a, b| {
                    let ca = a.0.category.display_name();
                    let cb = b.0.category.display_name();
                    if asc { ca.cmp(cb) } else { cb.cmp(ca) }
                }),
                "qty" => items.sort_by(|a, b| if asc { a.0.quantity.cmp(&b.0.quantity) } else { b.0.quantity.cmp(&a.0.quantity) }),
                "dungeon" => items.sort_by(|a, b| if asc { a.1.cmp(b.1) } else { b.1.cmp(a.1) }),
                "status" => items.sort_by(|a, b| {
                    let rank = |s: FlagState| match s { FlagState::Set => 2, FlagState::Unknown => 1, FlagState::Clear => 0 };
                    let (sa, sb) = (rank(a.2), rank(b.2));
                    if asc { sa.cmp(&sb) } else { sb.cmp(&sa) }
                }),
                _ => {}
            }
        }

        // Count totals (reusing the origin resolved once above, not per pickup).
        let total_count = DUNGEON_PICKUPS.len();
        let filtered_count = items.len();
        let collected_count: usize = DUNGEON_PICKUPS.iter()
            .filter(|p| is_dungeon_pickup_collected(ch.flags(), p) == FlagState::Set)
            .count();
        let unknown_count: usize = DUNGEON_PICKUPS.iter()
            .filter(|p| is_dungeon_pickup_collected(ch.flags(), p) == FlagState::Unknown)
            .count();

        // Summary
        let summary = if filtered_count < total_count {
            if unknown_count > 0 {
                format!("Dungeon Pickups: {}/{} collected ({} unknown) - showing {}", collected_count, total_count, unknown_count, filtered_count)
            } else {
                format!("Dungeon Pickups: {}/{} collected - showing {}", collected_count, total_count, filtered_count)
            }
        } else if unknown_count > 0 {
            format!("Dungeon Pickups: {}/{} collected ({} unknown)", collected_count, total_count, unknown_count)
        } else {
            format!("Dungeon Pickups: {}/{} collected", collected_count, total_count)
        };
        ui.label(RichText::new(&summary).strong());

        spacing::space_sm(ui);

        // Build row data
        let selected_flag_id = ss.dungeon_pickups_filter.selected_flag_id;
        let rows: Vec<RowData> = items.iter().map(|(pickup, dungeon_name, state)| {
            let status_icon = match state {
                FlagState::Unknown => icons::MISMATCH,
                FlagState::Set => icons::COLLECTED,
                FlagState::Clear => icons::NOT_COLLECTED,
            };

            let is_selected = selected_flag_id == Some(pickup.event_flag);

            let row_color = if is_selected {
                Color32::YELLOW
            } else {
                match state {
                    FlagState::Unknown => colors::STATUS_WARNING,
                    FlagState::Set => colors::STATUS_COLLECTED,
                    FlagState::Clear => Color32::LIGHT_GRAY,
                }
            };

            RowData::new(vec![
                status_icon.to_string(),
                pickup.event_flag.to_string(),
                pickup.name.to_string(),
                pickup.category.display_name().to_string(),
                pickup.quantity.to_string(),
                dungeon_name.to_string(),
            ]).with_color(row_color)
        }).collect();

        // Define columns
        let columns = vec![
            Column::new("status", "Status").width(50.0).sortable(true).center().icon(),
            Column::new("flag_id", "Flag ID").width(80.0).sortable(true).monospace(true),
            Column::new("item", "Item").width_fraction(0.25).sortable(true),
            Column::new("category", "Category").width(100.0).sortable(true),
            Column::new("qty", "Qty").width(50.0).sortable(true).right(),
            Column::new("dungeon", "Dungeon").width_fraction(0.2).sortable(true),
        ];

        // Show table
        let table_state = &mut ss.dungeon_pickups_filter.table_state;
        let table_response = UnifiedTable::new("dungeon_pickups_table", table_state)
            .columns(columns)
            .rows(rows)
            .zebra_stripe(true)
            .selectable(true)
            .show(ui);

        // Handle copy
        if let Some(text) = table_response.clipboard_text {
            ui.output_mut(|o| o.copied_text = text);
        }

        // Handle row selection for details panel
        if table_response.sort_changed || ss.dungeon_pickups_filter.table_state.selection_count() == 1 {
            if let Some(&idx) = ss.dungeon_pickups_filter.table_state.selected_rows.iter().next() {
                if let Some((pickup, _, _)) = items.get(idx) {
                    ss.dungeon_pickups_filter.selected_flag_id = Some(pickup.event_flag);
                }
            }
        }

        // Handle double-click
        if let Some(row_idx) = table_response.double_clicked_row {
            if let Some((pickup, dungeon_name, state)) = items.get(row_idx) {
                let status_text = match state {
                    FlagState::Unknown => "Unverified",
                    FlagState::Set => "Collected",
                    FlagState::Clear => "Not collected",
                };
                let row_text = format!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    status_text, pickup.event_flag, pickup.name,
                    pickup.category.display_name(), pickup.quantity, dungeon_name
                );
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let export_data: Vec<DungeonPickupExportItem> = items.iter()
                .map(|(pickup, dungeon_name, state)| DungeonPickupExportItem {
                    flag_id: pickup.event_flag,
                    item_name: pickup.name.to_string(),
                    category: pickup.category.display_name().to_string(),
                    quantity: pickup.quantity,
                    dungeon: dungeon_name.to_string(),
                    collected: *state == FlagState::Set,
                    verified: *state != FlagState::Unknown,
                })
                .collect();

            let content = match export_format {
                ExportFormat::Json => {
                    let export = PageExport::new(
                        PageExportMetadata::new("Dungeon Pickups")
                            .with_counts(total_count, filtered_count),
                        &export_data,
                    );
                    to_json(&export).unwrap_or_default()
                }
                ExportFormat::Csv => {
                    let headers = &["Flag ID", "Item", "Category", "Qty", "Dungeon", "Collected", "Verified"];
                    let rows: Vec<Vec<String>> = export_data.iter()
                        .map(|item| vec![
                            item.flag_id.to_string(),
                            item.item_name.clone(),
                            item.category.clone(),
                            item.quantity.to_string(),
                            item.dungeon.clone(),
                            if item.collected { "Yes" } else { "No" }.to_string(),
                            if item.verified { "Yes" } else { "No" }.to_string(),
                        ])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["Flag ID", "Item", "Category", "Qty", "Dungeon", "Collected", "Verified"];
                    let rows: Vec<Vec<String>> = export_data.iter()
                        .map(|item| vec![
                            item.flag_id.to_string(),
                            item.item_name.clone(),
                            item.category.clone(),
                            item.quantity.to_string(),
                            item.dungeon.clone(),
                            if item.collected { "Yes" } else { "No" }.to_string(),
                            if item.verified { "Yes" } else { "No" }.to_string(),
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

    /// Collect all set flags from the unique items database for verification.
    ///
    /// CUT OVER 2026-07-20 (ADR-0006). Only flags whose family is known are
    /// resolvable: of UNIQUE_ITEMS' 141 entries, 34 are world-state-b and the
    /// remaining 107 are 3-to-6-digit ids belonging to no verified family. The
    /// unresolvable ones are simply absent from the returned set, which is what
    /// they were before — a set of flags observed SET — so an unknown flag has
    /// never been representable here as anything but absent.
    fn collect_set_flags(event_flags: Option<&[u8]>) -> std::collections::HashSet<u32> {
        use crate::db::inventory_verification::{UNIQUE_ITEMS, UniqueItemCategory};
        use crate::db::pickup_flags::{pickup_state, world_flag_state};

        let mut set_flags = std::collections::HashSet::new();

        // Membership means "known Set". Unknown is correctly absent — a flag we
        // could not resolve is not asserted to be set.
        //
        // Remembrances and Great Runes are verified against their source boss's
        // DEFEAT flag (a world/dungeon flag), not a pickup: "you beat the boss, so
        // you were granted this", which also lets the triangle flag a consumed or
        // traded remembrance as flag-set-but-absent. They route through
        // `world_flag_state` (tile_world/dungeon). Every other category is an item
        // acquisition addressed by its own pickup flag and routes through
        // `pickup_state`. Routing on the value would hit the tile/pickup ambiguity
        // (CLAUDE.md); routing on the item's own category does not.
        if let Some(ef) = event_flags {
            let resolved = ResolvedFlags::from_event_flags(ef);
            for item in UNIQUE_ITEMS.iter() {
                let state = resolved.as_ref().map_or(FlagState::Unknown, |r| match item.category {
                    UniqueItemCategory::Remembrance | UniqueItemCategory::GreatRune => {
                        world_flag_state(r, item.event_flag)
                    }
                    _ => pickup_state(r, item.event_flag),
                });
                if state == FlagState::Set {
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
        ch: &Character,
        ss: &mut ScreenState,
        inventory: Option<&EquipInventoryData>,
        storage: Option<&EquipInventoryData>,
        save_path: &str,
    ) {
        // `state` is FlagState, not bool. The detail panel previously took `.0` of
        // a `(bool, _)` pair here, which rendered Unknown as "NOT COLLECTED" — the
        // exact collapse this migration removes. `Unknown` is the default when no
        // flag is selected, which never reaches the status display.
        let (selected_flag_id, flag_name, state, is_world_pickup) = match ss.current_route {
            EventsRoute::WorldPickups => {
                if let Some(flag_id) = ss.world_pickups_filter.selected_flag_id {
                    // Find the pickup data for this flag
                    let pickup = WORLD_PICKUPS.iter().find(|p| p.event_flag == flag_id);
                    if let Some(p) = pickup {
                        let state = pickup_state(ch.flags(), flag_id);
                        (Some(flag_id), p.name.to_string(), state, true)
                    } else {
                        (None, String::new(), FlagState::Unknown, true)
                    }
                } else {
                    (None, String::new(), FlagState::Unknown, true)
                }
            }
            EventsRoute::DungeonPickups => {
                if let Some(flag_id) = ss.dungeon_pickups_filter.selected_flag_id {
                    // Find the dungeon pickup data for this flag
                    let pickup = DUNGEON_PICKUPS.iter().find(|p| p.event_flag == flag_id);
                    if let Some(p) = pickup {
                        // CUT OVER 2026-07-20 (ADR-0006). This detail panel had its
                        // OWN copy of the legacy DUNGEON_PICKUP_BASES arithmetic,
                        // separate from the table's — so the panel and the row it
                        // was opened from could disagree about the same pickup.
                        // Both now go through the same resolver.
                        let state = pickup_state(ch.flags(), p.event_flag);
                        (Some(flag_id), p.name.to_string(), state, false)
                    } else {
                        (None, String::new(), FlagState::Unknown, false)
                    }
                } else {
                    (None, String::new(), FlagState::Unknown, false)
                }
            }
            _ => (None, String::new(), FlagState::Unknown, true),
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
                    ss.world_pickups_filter.selected_flag_id = None;
                } else {
                    ss.dungeon_pickups_filter.selected_flag_id = None;
                }
            }
        });
        spacer(ui);

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
            let (status_text, status_color) = match state {
                FlagState::Set => ("COLLECTED", Color32::from_rgb(100, 200, 100)),
                FlagState::Clear => ("NOT COLLECTED", Color32::LIGHT_GRAY),
                // The position could not be resolved: we do not know, and must not
                // claim "not collected" for a save we failed to read.
                FlagState::Unknown => ("UNKNOWN", colors::STATUS_WARNING),
            };
            ui.label(RichText::new(status_text).color(status_color));
        });
        spacer(ui);

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
                    let is_supporting = state == FlagState::Set;
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
                if state == FlagState::Set {
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

                // Summary. Only a resolved Clear contradicts a found item;
                // Unknown makes no claim to contradict.
                if state == FlagState::Clear && !matches.is_empty() {
                    spacer(ui);
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

        spacer(ui);

        // Copy Details button - generates comprehensive debug output
        if ui.button("Copy Details").clicked() {
            let mut details = String::new();

            // Context metadata for precise understanding
            let slot_index = ch.index();
            let character_name = ch.general().character_name.trim_matches('\0');
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

            details.push_str("=== CONTEXT ===\n");
            details.push_str(&format!("timestamp: {}\n", timestamp));
            details.push_str(&format!("save_file: {}\n", save_path));
            details.push_str(&format!("slot_index: {}\n", slot_index));
            details.push_str(&format!("character_name: {}\n", character_name));
            details.push_str(&format!("event_flags_size: {}\n", ch.flag_bytes().map(|ef| ef.len()).unwrap_or(0)));

            details.push_str("\n=== FLAG DETAILS ===\n");
            details.push_str(&format!("flag_id: {}\n", selected_flag_id));
            details.push_str(&format!("flag_id_hex: 0x{:08X}\n", selected_flag_id));
            details.push_str(&format!("item_name: {}\n", flag_name));
            details.push_str(&format!("flag_state: {}\n", match state {
                FlagState::Set => "set",
                FlagState::Clear => "clear",
                FlagState::Unknown => "unknown",
            }));
            details.push_str(&format!("pickup_type: {}\n", if is_world_pickup { "world" } else { "dungeon" }));

            // Add flag offset info if available
            if let Some(ef) = ch.flag_bytes() {
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
                scan_items(&inv.common_items, "equip_common", &flag_name, state == FlagState::Set, &mut match_count, &mut details);
                scan_items(&inv.key_items, "equip_key", &flag_name, state == FlagState::Set, &mut match_count, &mut details);
            } else {
                details.push_str("equip_inventory: NOT AVAILABLE\n");
            }

            if let Some(stor) = storage {
                details.push_str(&format!("storage_common_count: {}\n", stor.common_inventory_items_distinct_count));
                details.push_str(&format!("storage_key_count: {}\n", stor.key_inventory_items_distinct_count));
                scan_items(&stor.common_items, "storage_common", &flag_name, state == FlagState::Set, &mut match_count, &mut details);
                scan_items(&stor.key_items, "storage_key", &flag_name, state == FlagState::Set, &mut match_count, &mut details);
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
