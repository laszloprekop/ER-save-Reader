pub mod shop_items_view {
    use eframe::egui::{Ui, Color32, RichText};
    use crate::db::shop_items::{SHOP_ITEMS, MERCHANTS, ItemCategory};
    use crate::ui::components::table::{UnifiedTable, Column, TableState, RowData, SortDirection};
    use crate::ui::components::filter::{FilterBar, FilterBarState, FilterOption, fuzzy_match_default};
    use crate::ui::components::export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown};
    use crate::ui::tokens::spacing;
    use serde::Serialize;

    pub struct ShopItemsViewState {
        pub merchant_filter: String,
        pub search: String,
        pub selected_id: Option<u32>,
        pub table_state: TableState,
        pub filter_state: FilterBarState,
        pub export_format: ExportFormat,
        pub export_filtered_only: bool,
    }

    impl Default for ShopItemsViewState {
        fn default() -> Self {
            Self {
                merchant_filter: "All".to_string(),
                search: String::new(),
                selected_id: None,
                table_state: TableState::new().with_sort("id", SortDirection::Ascending),
                filter_state: FilterBarState::new(),
                export_format: ExportFormat::Json,
                export_filtered_only: false,
            }
        }
    }

    #[derive(Serialize)]
    struct ShopItemExportItem {
        id: u32,
        merchant: String,
        item_name: String,
        category: String,
        price: u32,
        quantity: String,
        stock_flag: u32,
    }

    pub fn shop_items_view(ui: &mut Ui, state: &mut ShopItemsViewState) {
        // Build merchant options
        let mut merchants: Vec<&str> = MERCHANTS.keys().copied().collect();
        merchants.sort();
        let merchant_options: Vec<FilterOption> = std::iter::once(FilterOption::all())
            .chain(merchants.iter().map(|m| FilterOption::from_str(*m)))
            .collect();

        // Sync filter state
        state.merchant_filter = state.filter_state.category.clone();
        state.search = state.filter_state.search.clone();

        // Filter bar
        FilterBar::new("shop_items_filter", &mut state.filter_state)
            .category("Merchant", merchant_options)
            .search("Search items...")
            .show(ui);

        spacing::space_sm(ui);

        // Export toolbar
        let export_response = ExportToolbar::new("shop_items_export", &mut state.export_format, &mut state.export_filtered_only)
            .has_filters(state.filter_state.has_active_filters())
            .show(ui);

        spacing::space_sm(ui);

        // Build data with filtering and sorting
        let mut items: Vec<(u32, &crate::db::shop_items::ShopItem)> = SHOP_ITEMS.iter()
            .filter(|(_, item)| {
                // Merchant filter
                if state.merchant_filter != "All" && item.merchant != state.merchant_filter {
                    return false;
                }

                // Search filter
                if !state.search.is_empty()
                    && !fuzzy_match_default(item.item_name, &state.search) &&
                       !fuzzy_match_default(item.merchant, &state.search) {
                        return false;
                    }

                true
            })
            .map(|(id, item)| (*id, item))
            .collect();

        // Apply sorting
        if let Some(sort_col) = &state.table_state.sort_column {
            let asc = state.table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "id" => items.sort_by(|a, b| if asc { a.0.cmp(&b.0) } else { b.0.cmp(&a.0) }),
                "merchant" => items.sort_by(|a, b| if asc { a.1.merchant.cmp(b.1.merchant) } else { b.1.merchant.cmp(a.1.merchant) }),
                "item" => items.sort_by(|a, b| if asc { a.1.item_name.cmp(b.1.item_name) } else { b.1.item_name.cmp(a.1.item_name) }),
                "category" => items.sort_by(|a, b| {
                    let ca = format!("{:?}", a.1.category);
                    let cb = format!("{:?}", b.1.category);
                    if asc { ca.cmp(&cb) } else { cb.cmp(&ca) }
                }),
                "price" => items.sort_by(|a, b| if asc { a.1.price.cmp(&b.1.price) } else { b.1.price.cmp(&a.1.price) }),
                "quantity" => items.sort_by(|a, b| if asc { a.1.quantity.cmp(&b.1.quantity) } else { b.1.quantity.cmp(&a.1.quantity) }),
                "stock_flag" => items.sort_by(|a, b| if asc { a.1.stock_flag.cmp(&b.1.stock_flag) } else { b.1.stock_flag.cmp(&a.1.stock_flag) }),
                _ => {}
            }
        }

        // Summary
        let total_count = SHOP_ITEMS.len();
        let filtered_count = items.len();
        if filtered_count < total_count {
            ui.label(RichText::new(format!("Shop Items: {} (showing {}/{})", total_count, filtered_count, total_count)).strong());
        } else {
            ui.label(RichText::new(format!("Shop Items: {}", total_count)).strong());
        }

        spacing::space_sm(ui);

        // Build row data
        let rows: Vec<RowData> = items.iter().map(|(id, item)| {
            let category_str = match item.category {
                ItemCategory::Weapon => "Weapon",
                ItemCategory::Armor => "Armor",
                ItemCategory::Accessory => "Accessory",
                ItemCategory::Good => "Good",
                ItemCategory::AshOfWar => "Ash of War",
                ItemCategory::Unknown => "Unknown",
            };

            let qty_str = if item.quantity < 0 { "\u{221e}".to_string() } else { item.quantity.to_string() };

            let is_selected = state.selected_id == Some(*id);

            let mut row = RowData::new(vec![
                id.to_string(),
                item.merchant.to_string(),
                item.item_name.to_string(),
                category_str.to_string(),
                item.price.to_string(),
                qty_str,
                item.stock_flag.to_string(),
            ]);

            if is_selected {
                row = row.with_color(Color32::YELLOW);
            }

            row
        }).collect();

        // Show table
        let table_response = UnifiedTable::new("shop_items_table", &mut state.table_state)
            .columns(vec![
                Column::new("id", "ID").width(60.0).sortable(true).monospace(true),
                Column::new("merchant", "Merchant").width(140.0).sortable(true),
                Column::new("item", "Item").width_fraction(0.3).sortable(true),
                Column::new("category", "Category").width(100.0).sortable(true),
                Column::new("price", "Price").width(80.0).sortable(true).right(),
                Column::new("quantity", "Qty").width(50.0).sortable(true).right(),
                Column::new("stock_flag", "Stock Flag").width(80.0).sortable(true).monospace(true),
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
            if let Some((id, _)) = items.get(row_idx) {
                let item = &items[row_idx].1;
                let category_str = match item.category {
                    ItemCategory::Weapon => "Weapon",
                    ItemCategory::Armor => "Armor",
                    ItemCategory::Accessory => "Accessory",
                    ItemCategory::Good => "Good",
                    ItemCategory::AshOfWar => "Ash of War",
                    ItemCategory::Unknown => "Unknown",
                };
                let qty_str = if item.quantity < 0 { "\u{221e}".to_string() } else { item.quantity.to_string() };
                let row_text = format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    id, item.merchant, item.item_name, category_str, item.price, qty_str, item.stock_flag
                );
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Update selected_id
        if state.table_state.selection_count() == 1 {
            if let Some(&idx) = state.table_state.selected_rows.iter().next() {
                if let Some((id, _)) = items.get(idx) {
                    state.selected_id = Some(*id);
                }
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let data_to_export: Vec<_> = items.iter()
                .map(|(id, item)| ShopItemExportItem {
                    id: *id,
                    merchant: item.merchant.to_string(),
                    item_name: item.item_name.to_string(),
                    category: match item.category {
                        ItemCategory::Weapon => "Weapon".to_string(),
                        ItemCategory::Armor => "Armor".to_string(),
                        ItemCategory::Accessory => "Accessory".to_string(),
                        ItemCategory::Good => "Good".to_string(),
                        ItemCategory::AshOfWar => "Ash of War".to_string(),
                        ItemCategory::Unknown => "Unknown".to_string(),
                    },
                    price: item.price,
                    quantity: if item.quantity < 0 { "\u{221e}".to_string() } else { item.quantity.to_string() },
                    stock_flag: item.stock_flag,
                })
                .collect();

            let content = match state.export_format {
                ExportFormat::Json => {
                    let export = PageExport::new(
                        PageExportMetadata::new("Shop Items")
                            .with_counts(total_count, filtered_count),
                        &data_to_export,
                    );
                    to_json(&export).unwrap_or_else(|_| String::new())
                }
                ExportFormat::Csv => {
                    let headers = &["ID", "Merchant", "Item", "Category", "Price", "Qty", "Stock Flag"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|s| vec![
                            s.id.to_string(),
                            s.merchant.clone(),
                            s.item_name.clone(),
                            s.category.clone(),
                            s.price.to_string(),
                            s.quantity.clone(),
                            s.stock_flag.to_string(),
                        ])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["ID", "Merchant", "Item", "Category", "Price", "Qty", "Stock Flag"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|s| vec![
                            s.id.to_string(),
                            s.merchant.clone(),
                            s.item_name.clone(),
                            s.category.clone(),
                            s.price.to_string(),
                            s.quantity.clone(),
                            s.stock_flag.to_string(),
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
