pub mod shop_items_view {
    use eframe::egui::{self, Ui, Color32, RichText};
    use crate::db::shop_items::{SHOP_ITEMS, MERCHANTS, ItemCategory};

    pub struct ShopItemsViewState {
        pub merchant_filter: String,
        pub search: String,
        pub selected_id: Option<u32>,
    }

    impl Default for ShopItemsViewState {
        fn default() -> Self {
            Self {
                merchant_filter: "All".to_string(),
                search: String::new(),
                selected_id: None,
            }
        }
    }

    pub fn shop_items_view(ui: &mut Ui, state: &mut ShopItemsViewState) {
        // Header with filters
        ui.horizontal(|ui| {
            ui.label(RichText::new("Merchant:").color(Color32::LIGHT_GRAY));

            // Get merchant names
            let mut merchants: Vec<&str> = MERCHANTS.keys().copied().collect();
            merchants.sort();
            merchants.insert(0, "All");

            egui::ComboBox::from_id_salt("merchant_filter")
                .selected_text(&state.merchant_filter)
                .show_ui(ui, |ui| {
                    for merchant in &merchants {
                        ui.selectable_value(&mut state.merchant_filter, merchant.to_string(), *merchant);
                    }
                });

            ui.separator();
            ui.label(RichText::new("Search:").color(Color32::LIGHT_GRAY));
            ui.text_edit_singleline(&mut state.search);
        });
        ui.separator();

        // Column headers
        ui.horizontal(|ui| {
            ui.label(RichText::new("ID | Merchant | Item | Category | Price | Qty | Stock Flag").color(Color32::YELLOW).monospace());
        });
        ui.separator();

        // Scrollable list
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let search_lower = state.search.to_lowercase();

                let mut items: Vec<_> = SHOP_ITEMS.iter().collect();
                items.sort_by_key(|(id, _)| *id);

                for (id, item) in items {
                    // Apply merchant filter
                    if state.merchant_filter != "All" && item.merchant != state.merchant_filter {
                        continue;
                    }

                    // Apply search
                    if !state.search.is_empty() {
                        let matches = item.item_name.to_lowercase().contains(&search_lower)
                            || item.merchant.to_lowercase().contains(&search_lower);
                        if !matches {
                            continue;
                        }
                    }

                    let category_str = match item.category {
                        ItemCategory::Weapon => "Weapon",
                        ItemCategory::Armor => "Armor",
                        ItemCategory::Accessory => "Accessory",
                        ItemCategory::Good => "Good",
                        ItemCategory::AshOfWar => "Ash of War",
                        ItemCategory::Unknown => "Unknown",
                    };

                    let qty_str = if item.quantity < 0 { "∞".to_string() } else { item.quantity.to_string() };

                    let row_text = format!(
                        "{} | {} | {} | {} | {} | {} | {}",
                        id, item.merchant, item.item_name, category_str, item.price, qty_str, item.stock_flag
                    );

                    let is_selected = state.selected_id == Some(*id);
                    let text_color = if is_selected { Color32::YELLOW } else { Color32::LIGHT_GRAY };

                    let response = ui.add(
                        egui::Label::new(RichText::new(&row_text).color(text_color).monospace())
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
                            ui.output_mut(|o| o.copied_text = item.item_name.to_string());
                            ui.close_menu();
                        }
                        if ui.button("Copy Stock Flag").clicked() {
                            ui.output_mut(|o| o.copied_text = item.stock_flag.to_string());
                            ui.close_menu();
                        }
                    });
                }
            });
    }
}
