pub mod event_flags_db_view {
    use eframe::egui::{Ui, Color32, RichText};
    use rfd::FileDialog;
    use std::fs::File;
    use std::io::Write;
    use crate::db::event_flags_db::event_flags_db::{
        EVENT_FLAGS_DB, EventFlagCategory, EventFlagEntryOwned, get_unique_regions
    };
    use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData, SortDirection};
    use crate::ui::components::filter::{FilterBar, FilterBarState, FilterOption, fuzzy_match_default};
    use crate::ui::components::export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
    use crate::ui::tokens::spacing;
    use serde::Serialize;

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

    #[derive(Serialize)]
    struct EventFlagExportItem {
        flag_id: u32,
        name: String,
        category: String,
        region: String,
        coords: Option<(f32, f32, f32)>,
    }

    pub struct EventFlagsDbViewState {
        pub category_filter: EventFlagCategoryFilter,
        pub region_filter: String,
        pub search: String,
        pub selected_id: Option<u32>,
        pub regions_cache: Vec<String>,
        pub export_status: Option<String>,
        pub table_state: TableState,
        pub filter_state: FilterBarState,
        pub export_format: ExportFormat,
        pub export_filtered_only: bool,
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
                table_state: TableState::new().with_sort("flag_id", SortDirection::Ascending),
                filter_state: FilterBarState::new(),
                export_format: ExportFormat::Json,
                export_filtered_only: false,
            }
        }
    }

    pub fn event_flags_db_view(ui: &mut Ui, state: &mut EventFlagsDbViewState) {
        // Initialize regions cache if empty
        if state.regions_cache.is_empty() {
            state.regions_cache = get_unique_regions();
        }

        // Build region filter options
        let region_options: Vec<FilterOption> = std::iter::once(FilterOption::all())
            .chain(state.regions_cache.iter().map(FilterOption::from_str))
            .collect();

        // Sync filter state
        state.region_filter = state.filter_state.category.clone();
        state.search = state.filter_state.search.clone();

        // Filter bar with region dropdown and search
        FilterBar::new("event_flags_filter", &mut state.filter_state)
            .category("Region", region_options)
            .search("Search flags...")
            .show(ui);

        spacing::space_sm(ui);

        // Category filter chips (keeping these separate since there are many)
        ui.horizontal(|ui| {
            ui.label(RichText::new("Category:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::All, "All");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::GreatRune, "Rune");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::BossDefeat, "Boss");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Remembrance, "Remembrance");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Grace, "Grace");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::MapFragment, "Map");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Landmark, "Landmark");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Cookbook, "Cookbook");
        });

        ui.horizontal(|ui| {
            ui.add_space(60.0);
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::WorldPickup, "World");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::DungeonPickup, "Dungeon");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::DLCPickup, "DLC");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::ShopStock, "Shop");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::NpcState, "NPC");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::Progression, "Progress");
            ui.selectable_value(&mut state.category_filter, EventFlagCategoryFilter::System, "System");
        });

        spacing::space_sm(ui);

        // Export toolbar
        let export_response = ExportToolbar::new("event_flags_export", &mut state.export_format, &mut state.export_filtered_only)
            .has_filters(state.filter_state.has_active_filters() || state.category_filter != EventFlagCategoryFilter::All)
            .show(ui);

        spacing::space_sm(ui);

        // Build filtered data
        let search_lower = state.search.to_lowercase();
        let mut entries: Vec<&EventFlagEntryOwned> = EVENT_FLAGS_DB.iter()
            .filter(|entry| filter_entry(entry, state, &search_lower))
            .collect();

        // Apply sorting
        if let Some(sort_col) = &state.table_state.sort_column {
            let asc = state.table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "flag_id" => entries.sort_by(|a, b| if asc { a.flag_id.cmp(&b.flag_id) } else { b.flag_id.cmp(&a.flag_id) }),
                "name" => entries.sort_by(|a, b| if asc { a.name.cmp(&b.name) } else { b.name.cmp(&a.name) }),
                "category" => entries.sort_by(|a, b| {
                    let ca = a.category.name();
                    let cb = b.category.name();
                    if asc { ca.cmp(cb) } else { cb.cmp(ca) }
                }),
                "region" => entries.sort_by(|a, b| if asc { a.region.cmp(&b.region) } else { b.region.cmp(&a.region) }),
                _ => {}
            }
        }

        // Summary
        let total_count = EVENT_FLAGS_DB.len();
        let filtered_count = entries.len();
        if filtered_count < total_count {
            ui.label(RichText::new(format!("Event Flags: {} (showing {}/{})", total_count, filtered_count, total_count)).strong());
        } else {
            ui.label(RichText::new(format!("Event Flags: {}", total_count)).strong());
        }

        spacing::space_sm(ui);

        // Build row data with category colors
        let rows: Vec<RowData> = entries.iter().map(|entry| {
            let coords_str = if let Some(coords) = &entry.coords {
                format!("({:.0}, {:.0}, {:.0})", coords.x, coords.y, coords.z)
            } else {
                "-".to_string()
            };

            let is_selected = state.selected_id == Some(entry.flag_id);

            let mut row = RowData::new(vec![
                entry.flag_id.to_string(),
                entry.name.clone(),
                entry.category.name().to_string(),
                entry.region.clone(),
                coords_str,
            ]);

            if is_selected {
                row = row.with_color(Color32::YELLOW);
            } else {
                row = row.with_color(get_category_color(entry.category));
            }

            row
        }).collect();

        // Show table
        let table_response = UnifiedTable::new("event_flags_table", &mut state.table_state)
            .columns(vec![
                Column::new("flag_id", "Flag ID").width(100.0).sortable(true).monospace(true),
                Column::new("name", "Name").width_fraction(0.35).sortable(true),
                Column::new("category", "Category").width(120.0).sortable(true),
                Column::new("region", "Region").width_fraction(0.2).sortable(true),
                Column::new("coords", "Coordinates").width(140.0).monospace(true),
            ])
            .rows(rows)
            .zebra_stripe(true)
            .selectable(true)
            .show(ui);

        // Handle copy
        if let Some(text) = table_response.clipboard_text {
            ui.output_mut(|o| o.copied_text = text);
        }

        // Handle double-click
        if let Some(row_idx) = table_response.double_clicked_row {
            if let Some(entry) = entries.get(row_idx) {
                let row_text = format!(
                    "{}\t{}\t{}\t{}",
                    entry.flag_id, entry.name, entry.category.name(), entry.region
                );
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Update selected_id
        if state.table_state.selection_count() == 1 {
            if let Some(&idx) = state.table_state.selected_rows.iter().next() {
                if let Some(entry) = entries.get(idx) {
                    state.selected_id = Some(entry.flag_id);
                }
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let data_to_export: Vec<_> = entries.iter()
                .map(|entry| EventFlagExportItem {
                    flag_id: entry.flag_id,
                    name: entry.name.clone(),
                    category: entry.category.name().to_string(),
                    region: entry.region.clone(),
                    coords: entry.coords.as_ref().map(|c| (c.x, c.y, c.z)),
                })
                .collect();

            let content = match state.export_format {
                ExportFormat::Json => {
                    let export = PageExport::new(
                        PageExportMetadata::new("Event Flags")
                            .with_counts(total_count, filtered_count),
                        &data_to_export,
                    );
                    to_json(&export).unwrap_or_else(|_| String::new())
                }
                ExportFormat::Csv => {
                    let headers = &["Flag ID", "Name", "Category", "Region", "Coordinates"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|e| vec![
                            e.flag_id.to_string(),
                            e.name.clone(),
                            e.category.clone(),
                            e.region.clone(),
                            e.coords.map(|(x, y, z)| format!("{}, {}, {}", x, y, z)).unwrap_or_default(),
                        ])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["Flag ID", "Name", "Category", "Region", "Coordinates"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|e| vec![
                            e.flag_id.to_string(),
                            e.name.clone(),
                            e.category.clone(),
                            e.region.clone(),
                            e.coords.map(|(x, y, z)| format!("{}, {}, {}", x, y, z)).unwrap_or_default(),
                        ])
                        .collect();
                    to_markdown(headers, &rows)
                }
            };

            if export_response.copy_clicked {
                ui.output_mut(|o| o.copied_text = content.clone());
            }

            // Handle file export
            if export_response.export_clicked {
                let extension = match state.export_format {
                    ExportFormat::Json => "json",
                    ExportFormat::Csv => "csv",
                    ExportFormat::Markdown => "md",
                };
                if let Some(path) = FileDialog::new()
                    .add_filter(extension.to_uppercase().as_str(), &[extension])
                    .set_file_name(format!("event_flags.{}", extension))
                    .save_file()
                {
                    match File::create(&path) {
                        Ok(mut file) => {
                            if let Err(e) = file.write_all(content.as_bytes()) {
                                state.export_status = Some(format!("Write error: {}", e));
                            } else {
                                state.export_status = Some(format!("Exported {} flags to {}", filtered_count, path.display()));
                            }
                        }
                        Err(e) => {
                            state.export_status = Some(format!("File error: {}", e));
                        }
                    }
                }
            }
        }

        // Show export status
        if let Some(status) = &state.export_status {
            ui.label(RichText::new(status).color(Color32::YELLOW).small());
        }
    }

    fn filter_entry(entry: &EventFlagEntryOwned, state: &EventFlagsDbViewState, _search_lower: &str) -> bool {
        // Category filter
        if !state.category_filter.matches(entry.category) {
            return false;
        }

        // Region filter
        if state.region_filter != "All" && entry.region != state.region_filter {
            return false;
        }

        // Search filter (by name or flag ID) using fuzzy match
        if !state.search.is_empty() {
            let name_match = fuzzy_match_default(&entry.name, &state.search);
            let id_match = entry.flag_id.to_string().contains(&state.search);
            if !name_match && !id_match {
                return false;
            }
        }

        true
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
