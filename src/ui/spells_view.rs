pub mod spells_view {
    use eframe::egui::{self, Ui, Color32, RichText};
    use crate::db::spells::{SPELLS, SpellType};
    use crate::ui::style::TABLE_MONO_SIZE;

    #[derive(Clone, Copy, PartialEq)]
    pub enum SpellFilter {
        All,
        Sorceries,
        Incantations,
    }

    pub struct SpellsViewState {
        pub filter: SpellFilter,
        pub search: String,
        pub selected_id: Option<u32>,
    }

    impl Default for SpellsViewState {
        fn default() -> Self {
            Self {
                filter: SpellFilter::All,
                search: String::new(),
                selected_id: None,
            }
        }
    }

    pub fn spells_view(ui: &mut Ui, state: &mut SpellsViewState) {
        // Header with filters
        ui.horizontal(|ui| {
            ui.label(RichText::new("Filter:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut state.filter, SpellFilter::All, "All");
            ui.selectable_value(&mut state.filter, SpellFilter::Sorceries, "Sorceries");
            ui.selectable_value(&mut state.filter, SpellFilter::Incantations, "Incantations");
            ui.separator();
            ui.label(RichText::new("Search:").color(Color32::LIGHT_GRAY));
            ui.text_edit_singleline(&mut state.search);
        });
        ui.separator();

        // Column headers
        ui.horizontal(|ui| {
            ui.label(RichText::new("ID | Name | Type | FP | Slots | INT | FTH").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        // Scrollable list
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let search_lower = state.search.to_lowercase();

                let mut spells: Vec<_> = SPELLS.iter().collect();
                spells.sort_by_key(|(id, _)| *id);

                for (id, spell) in spells {
                    // Apply filter
                    let type_match = match state.filter {
                        SpellFilter::All => true,
                        SpellFilter::Sorceries => spell.spell_type == SpellType::Sorcery,
                        SpellFilter::Incantations => spell.spell_type == SpellType::Incantation,
                    };

                    if !type_match {
                        continue;
                    }

                    // Apply search
                    if !state.search.is_empty() && !spell.name.to_lowercase().contains(&search_lower) {
                        continue;
                    }

                    let type_str = match spell.spell_type {
                        SpellType::Sorcery => "Sorcery",
                        SpellType::Incantation => "Incantation",
                    };

                    let row_text = format!(
                        "{} | {} | {} | {} | {} | {} | {}",
                        id, spell.name, type_str, spell.fp_cost, spell.slots, spell.int_req, spell.fai_req
                    );

                    let is_selected = state.selected_id == Some(*id);
                    let text_color = if is_selected { Color32::YELLOW } else { Color32::LIGHT_GRAY };

                    let response = ui.add(
                        egui::Label::new(RichText::new(&row_text).color(text_color).monospace().size(TABLE_MONO_SIZE))
                            .sense(egui::Sense::click())
                    );

                    if response.clicked() {
                        state.selected_id = Some(*id);
                    }

                    // Copy on double-click or right-click
                    if response.double_clicked() {
                        ui.output_mut(|o| o.copied_text = row_text.clone());
                    }

                    response.context_menu(|ui| {
                        if ui.button("Copy row").clicked() {
                            ui.output_mut(|o| o.copied_text = row_text.clone());
                            ui.close_menu();
                        }
                        if ui.button("Copy ID").clicked() {
                            ui.output_mut(|o| o.copied_text = id.to_string());
                            ui.close_menu();
                        }
                        if ui.button("Copy Name").clicked() {
                            ui.output_mut(|o| o.copied_text = spell.name.to_string());
                            ui.close_menu();
                        }
                    });
                }
            });
    }
}
