pub mod npcs_view {
    use eframe::egui::{self, Ui, Color32, RichText};
    use crate::db::npcs::{NPCS, NpcType};
    use crate::ui::style::{TABLE_MONO_SIZE, spacer};

    #[derive(Clone, Copy, PartialEq)]
    pub enum NpcFilter {
        All,
        Merchants,
        QuestNpcs,
        RoundtableNpcs,
        Invaders,
    }

    pub struct NpcsViewState {
        pub filter: NpcFilter,
        pub search: String,
        pub selected_id: Option<u32>,
    }

    impl Default for NpcsViewState {
        fn default() -> Self {
            Self {
                filter: NpcFilter::All,
                search: String::new(),
                selected_id: None,
            }
        }
    }

    pub fn npcs_view(ui: &mut Ui, state: &mut NpcsViewState) {
        // Header with filters
        ui.horizontal(|ui| {
            ui.label(RichText::new("Filter:").color(Color32::LIGHT_GRAY));
            ui.selectable_value(&mut state.filter, NpcFilter::All, "All");
            ui.selectable_value(&mut state.filter, NpcFilter::Merchants, "Merchants");
            ui.selectable_value(&mut state.filter, NpcFilter::QuestNpcs, "Quest NPCs");
            ui.selectable_value(&mut state.filter, NpcFilter::RoundtableNpcs, "Roundtable");
            ui.selectable_value(&mut state.filter, NpcFilter::Invaders, "Invaders");
            spacer(ui);
            ui.label(RichText::new("Search:").color(Color32::LIGHT_GRAY));
            ui.text_edit_singleline(&mut state.search);
        });
        spacer(ui);

        // Column headers
        ui.horizontal(|ui| {
            ui.label(RichText::new("ID | Name | Type | Location | Discovery Flag | Death Flag").color(Color32::YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        spacer(ui);

        // Scrollable list
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let search_lower = state.search.to_lowercase();

                let mut npcs: Vec<_> = NPCS.iter().collect();
                npcs.sort_by_key(|(id, _)| *id);

                for (id, npc) in npcs {
                    // Apply filter
                    let type_match = match state.filter {
                        NpcFilter::All => true,
                        NpcFilter::Merchants => npc.npc_type == NpcType::Merchant,
                        NpcFilter::QuestNpcs => npc.npc_type == NpcType::QuestNpc,
                        NpcFilter::RoundtableNpcs => npc.npc_type == NpcType::RoundtableNpc,
                        NpcFilter::Invaders => npc.npc_type == NpcType::Invader,
                    };

                    if !type_match {
                        continue;
                    }

                    // Apply search
                    if !state.search.is_empty() {
                        let matches = npc.name.to_lowercase().contains(&search_lower)
                            || npc.location.to_lowercase().contains(&search_lower);
                        if !matches {
                            continue;
                        }
                    }

                    let type_str = match npc.npc_type {
                        NpcType::Merchant => "Merchant",
                        NpcType::QuestNpc => "Quest NPC",
                        NpcType::RoundtableNpc => "Roundtable",
                        NpcType::Invader => "Invader",
                        NpcType::Boss => "Boss",
                        NpcType::Spirit => "Spirit",
                    };

                    let discovery_str = npc.discovery_flag.map(|f| f.to_string()).unwrap_or("-".to_string());
                    let death_str = npc.death_flag.map(|f| f.to_string()).unwrap_or("-".to_string());

                    let row_text = format!(
                        "{} | {} | {} | {} | {} | {}",
                        id, npc.name, type_str, npc.location, discovery_str, death_str
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

                    if response.double_clicked() {
                        ui.output_mut(|o| o.copied_text = row_text.clone());
                    }

                    response.context_menu(|ui| {
                        if ui.button("Copy row").clicked() {
                            ui.output_mut(|o| o.copied_text = row_text.clone());
                            ui.close_menu();
                        }
                        if ui.button("Copy Name").clicked() {
                            ui.output_mut(|o| o.copied_text = npc.name.to_string());
                            ui.close_menu();
                        }
                        if let Some(flag) = npc.discovery_flag {
                            if ui.button("Copy Discovery Flag").clicked() {
                                ui.output_mut(|o| o.copied_text = flag.to_string());
                                ui.close_menu();
                            }
                        }
                    });
                }
            });
    }
}
