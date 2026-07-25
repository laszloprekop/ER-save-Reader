pub mod general {
    use crate::ui::icons::{icon_with_name, ICON_DISPLAY_SIZE};
    use crate::ui::tokens::{colors, typography};
    use crate::vm::equipment::equipment_view_model::EquipmentItemViewModel;
    use crate::vm::{general::general_view_model::Gender, character::character::Character};
    use crate::db::classes::classes::class_display;
    use er_reconstruct::ReconstructedCharacter;
    use eframe::egui::{self, Color32, Frame, RichText, Rounding, Ui};

    /// Deep gray card background (darker than app background)
    const CARD_BG: Color32 = Color32::from_rgb(30, 30, 35);

    /// Section header styling
    fn section_header(ui: &mut Ui, text: &str) {
        ui.label(
            RichText::new(text)
                .size(typography::TEXT_SM)
                .color(colors::CAT_SUBTEXT0)
                .strong(),
        );
        ui.add_space(4.0);
    }

    /// Stat row with label on left, value on right (game style)
    fn stat_row(ui: &mut Ui, label: &str, value: &str) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(label)
                    .size(typography::TEXT_SM)
                    .color(colors::CAT_SUBTEXT1),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(value)
                        .size(typography::TEXT_SM)
                        .color(colors::CAT_TEXT),
                );
            });
        });
    }

    /// Stat row with two values (current / max)
    fn stat_row_dual(ui: &mut Ui, label: &str, current: u32, max: u32) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(label)
                    .size(typography::TEXT_SM)
                    .color(colors::CAT_SUBTEXT1),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} / {}", current, max))
                        .size(typography::TEXT_SM)
                        .color(colors::CAT_TEXT)
                        .monospace(),
                );
            });
        });
    }

    /// Equipment slot display
    fn equipment_slot(ui: &mut Ui, label: &str, name: &str) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(label)
                    .size(typography::TEXT_XS)
                    .color(colors::CAT_OVERLAY1),
            );

            let (display_name, color) = if name == "Empty" || name.is_empty() {
                ("—", colors::CAT_OVERLAY0)
            } else {
                (name, colors::CAT_TEXT)
            };

            let response = ui.add(
                egui::Label::new(
                    RichText::new(display_name)
                        .size(typography::TEXT_SM)
                        .color(color),
                )
                .sense(egui::Sense::click()),
            );

            if response.double_clicked() && !name.is_empty() && name != "Empty" {
                ui.output_mut(|o| o.copied_text = name.to_string());
            }

            response.context_menu(|ui| {
                if ui.button("Copy name").clicked() {
                    ui.output_mut(|o| o.copied_text = name.to_string());
                    ui.close_menu();
                }
            });
        });
    }

    /// Compact equipment slot (just numbered)
    fn equipment_slot_numbered(ui: &mut Ui, index: usize, name: &str, available: bool) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{}.", index + 1))
                    .size(typography::TEXT_SM)
                    .color(if available {
                        colors::CAT_OVERLAY0
                    } else {
                        colors::CAT_SURFACE2
                    })
                    .monospace(),
            );

            if !available {
                ui.label(
                    RichText::new("🔒")
                        .size(typography::TEXT_SM)
                        .color(colors::CAT_SURFACE2),
                );
                return;
            }

            let (display_name, color) = if name == "Empty" || name.is_empty() {
                ("—", colors::CAT_OVERLAY0)
            } else {
                (name, colors::CAT_TEXT)
            };

            let response = ui.add(
                egui::Label::new(
                    RichText::new(display_name)
                        .size(typography::TEXT_SM)
                        .color(color),
                )
                .sense(egui::Sense::click()),
            );

            if response.double_clicked() && !name.is_empty() && name != "Empty" {
                ui.output_mut(|o| o.copied_text = name.to_string());
            }
        });
    }

    /// Gear slot for grid layout (compact, no label)
    fn gear_slot(ui: &mut Ui, name: &str) {
        let (display_name, color) = if name == "Empty" || name.is_empty() {
            ("—", colors::CAT_OVERLAY0)
        } else {
            (name, colors::CAT_TEXT)
        };

        let response = ui.add(
            egui::Label::new(
                RichText::new(display_name)
                    .size(typography::TEXT_XS)
                    .color(color),
            )
            .sense(egui::Sense::click()),
        );

        if response.double_clicked() && !name.is_empty() && name != "Empty" {
            ui.output_mut(|o| o.copied_text = name.to_string());
        }

        response.context_menu(|ui| {
            if ui.button("Copy name").clicked() {
                ui.output_mut(|o| o.copied_text = name.to_string());
                ui.close_menu();
            }
        });
    }

    /// Gear slot centered (for armor in middle column)
    fn gear_slot_centered(ui: &mut Ui, name: &str) {
        let (display_name, color) = if name == "Empty" || name.is_empty() {
            ("—", colors::CAT_OVERLAY0)
        } else {
            (name, colors::CAT_TEXT)
        };

        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            let response = ui.add(
                egui::Label::new(
                    RichText::new(display_name)
                        .size(typography::TEXT_XS)
                        .color(color),
                )
                .sense(egui::Sense::click()),
            );

            if response.double_clicked() && !name.is_empty() && name != "Empty" {
                ui.output_mut(|o| o.copied_text = name.to_string());
            }

            response.context_menu(|ui| {
                if ui.button("Copy name").clicked() {
                    ui.output_mut(|o| o.copied_text = name.to_string());
                    ui.close_menu();
                }
            });
        });
    }

    /// Equipment slot with game icon (icon on top, name below)
    fn equipment_icon_slot(ui: &mut Ui, item: &EquipmentItemViewModel, size: f32) {
        let response = icon_with_name(ui, item.icon_id, &item.name, size);

        if response.double_clicked() && !item.name.is_empty() && item.name != "Empty" {
            ui.output_mut(|o| o.copied_text = item.name.clone());
        }

        response.context_menu(|ui| {
            if ui.button("Copy name").clicked() {
                ui.output_mut(|o| o.copied_text = item.name.clone());
                ui.close_menu();
            }
        });
    }

    /// Locked equipment slot placeholder
    fn locked_slot(ui: &mut Ui, size: f32) {
        let (rect, _response) =
            ui.allocate_exact_size(egui::vec2(size + 8.0, size + 20.0), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            // Draw locked placeholder
            let icon_rect =
                egui::Rect::from_min_size(rect.min + egui::vec2(4.0, 0.0), egui::vec2(size, size));
            ui.painter()
                .rect_filled(icon_rect, 4.0, Color32::from_rgb(30, 30, 35));
            ui.painter()
                .rect_stroke(icon_rect, 4.0, egui::Stroke::new(1.0, colors::CAT_CRUST));
            // Draw lock icon
            let lock_pos = icon_rect.center();
            ui.painter().text(
                lock_pos,
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::LOCK,
                egui::FontId::proportional(size * 0.4),
                colors::CAT_CRUST,
            );
        }
    }

    /// Quick item slot (compact, shows index)
    fn quick_slot(ui: &mut Ui, index: usize, name: &str) {
        ui.horizontal(|ui| {
            // Show slot number with different styling for main slots vs extended
            let slot_color = if index < 4 {
                colors::CAT_SUBTEXT0
            } else {
                colors::CAT_OVERLAY0
            };

            ui.label(
                RichText::new(format!("{:>2}.", index + 1))
                    .size(typography::TEXT_XS)
                    .color(slot_color)
                    .monospace(),
            );

            let (display_name, color) = if name == "Empty" || name.is_empty() {
                ("—", colors::CAT_OVERLAY0)
            } else {
                (name, colors::CAT_TEXT)
            };

            ui.label(
                RichText::new(display_name)
                    .size(typography::TEXT_XS)
                    .color(color),
            );
        });
    }

    /// Format number with thousand separators
    fn format_number(n: u32) -> String {
        let s = n.to_string();
        let mut result = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.insert(0, ',');
            }
            result.insert(0, c);
        }
        result
    }

    pub fn general(ui: &mut Ui, ch: &Character, facts: Option<&ReconstructedCharacter>) {
        let general_vm = &ch.general();
        let stats_vm = &ch.stats();
        let equipment_vm = &ch.equipment();

        let gender_str = match general_vm.gender {
            Gender::Male => "♂",
            Gender::Female => "♀",
            Gender::Uknown => "?",
        };

        // Identity is rendered from the reconstruction core's facts (ADR-0010) when
        // present: `name`/`level` verbatim, and `class_id` mapped to its Canonical
        // Name here in the reader's Enrichment (the core keeps the raw id, no
        // display string). Falls back to the ViewModel only for the empty/default
        // state, where no save has been reconstructed yet.
        let (name_str, level_val, class_str): (String, u32, String) = match facts {
            Some(f) => (
                f.name.clone(),
                f.level,
                class_display(f.class_id),
            ),
            None => (
                general_vm.character_name.trim_matches('\0').to_string(),
                stats_vm.level,
                stats_vm.arche_type.to_string(),
            ),
        };

        // Attributes, derived vitals, runes, and DLC blessings likewise render from
        // the core's `stats` facts (ADR-0010) when present — the values are identical
        // to the ViewModel's, so this simply makes the shared core the single source
        // and retires the reader's parallel compute for this panel. The ViewModel is
        // the fallback for the empty/default state only.
        struct RenderStats {
            vigor: u32, mind: u32, endurance: u32, strength: u32,
            dexterity: u32, intelligence: u32, faith: u32, arcane: u32,
            hp: u32, max_hp: u32, fp: u32, max_fp: u32, max_stamina: u32,
            runes: u32, runes_memory: u32, scadutree: u32, spirit_ash: u32,
        }
        let s = match facts {
            Some(f) => {
                let st = &f.stats;
                RenderStats {
                    vigor: st.vigor, mind: st.mind, endurance: st.endurance, strength: st.strength,
                    dexterity: st.dexterity, intelligence: st.intelligence, faith: st.faith, arcane: st.arcane,
                    hp: st.hp, max_hp: st.max_hp, fp: st.fp, max_fp: st.max_fp, max_stamina: st.max_stamina,
                    runes: st.runes, runes_memory: st.runes_memory,
                    scadutree: u32::from(st.scadutree_level), spirit_ash: u32::from(st.spirit_ash_level),
                }
            }
            None => RenderStats {
                vigor: stats_vm.vigor, mind: stats_vm.mind, endurance: stats_vm.endurance, strength: stats_vm.strength,
                dexterity: stats_vm.dexterity, intelligence: stats_vm.intelligence, faith: stats_vm.faith, arcane: stats_vm.arcane,
                hp: stats_vm.hp, max_hp: stats_vm.max_hp, fp: stats_vm.fp, max_fp: stats_vm.max_fp, max_stamina: stats_vm.max_stamina,
                runes: stats_vm.souls, runes_memory: stats_vm.soulsmemory,
                scadutree: stats_vm.scadutree, spirit_ash: stats_vm.spirit_ash,
            },
        };

        // Main content: three columns with auto-width
        egui::Grid::new("general_main_layout")
            .num_columns(3)
            .spacing([12.0, 0.0])
            .show(ui, |ui| {
                // ═══════════════════════════════════════════════════════════════
                // COLUMN 1: Character Status (matching game layout)
                // ═══════════════════════════════════════════════════════════════
                ui.vertical(|ui| {
                    ui.set_min_width(250.0);

                    Frame::none()
                        // .fill(CARD_BG)
                        .rounding(Rounding::same(6.0))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            // Character Header
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&name_str)
                                        .size(typography::HEADING_MD)
                                        .color(colors::CAT_TEXT)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(gender_str)
                                        .size(typography::HEADING_MD)
                                        .color(colors::CAT_SUBTEXT0),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("Lv. {}", level_val))
                                        .size(typography::HEADING_SM)
                                        .color(colors::CAT_YELLOW)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(&class_str)
                                        .size(typography::TEXT_SM)
                                        .color(colors::CAT_SUBTEXT0),
                                );
                            });

                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            section_header(ui, "ATTRIBUTES");

                            stat_row(ui, "Vigor", &s.vigor.to_string());
                            stat_row(ui, "Mind", &s.mind.to_string());
                            stat_row(ui, "Endurance", &s.endurance.to_string());
                            stat_row(ui, "Strength", &s.strength.to_string());
                            stat_row(ui, "Dexterity", &s.dexterity.to_string());
                            stat_row(ui, "Intelligence", &s.intelligence.to_string());
                            stat_row(ui, "Faith", &s.faith.to_string());
                            stat_row(ui, "Arcane", &s.arcane.to_string());

                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // Derived stats
                            stat_row_dual(ui, "HP", s.hp, s.max_hp);
                            stat_row_dual(ui, "FP", s.fp, s.max_fp);
                            stat_row(ui, "Stamina", &s.max_stamina.to_string());

                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // Additional info
                            stat_row(ui, "Runes Held", &format_number(s.runes));
                            stat_row(ui, "Total Runes", &format_number(s.runes_memory));
                            stat_row(ui, "Weapon Level", &general_vm.weapon_level.to_string());

                            // DLC Blessings (only show if > 0)
                            if s.scadutree > 0 || s.spirit_ash > 0 {
                                ui.add_space(4.0);
                                ui.separator();
                                ui.add_space(4.0);

                                if s.scadutree > 0 {
                                    stat_row(
                                        ui,
                                        "Scadutree",
                                        &format!("{}/20", s.scadutree),
                                    );
                                }
                                if s.spirit_ash > 0 {
                                    stat_row(
                                        ui,
                                        "Spirit Ash",
                                        &format!("{}/10", s.spirit_ash),
                                    );
                                }
                            }

                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // Location
                            stat_row(ui, "Location", general_vm.map_id.display_name());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("").size(typography::TEXT_XS));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(general_vm.map_id.to_string())
                                                .size(typography::TEXT_XS)
                                                .color(colors::CAT_OVERLAY0)
                                                .monospace(),
                                        );
                                    },
                                );
                            });
                        });
                });

                // ═══════════════════════════════════════════════════════════════
                // COLUMN 2: Equipment (Grid layout like game UI with icons)
                // ═══════════════════════════════════════════════════════════════
                ui.vertical(|ui| {
                    // Equipped Gear - Grid: Right Hand | Armor | Left Hand
                    Frame::none().show(ui, |ui| {
                        section_header(ui, "EQUIPPED GEAR");

                        // Grid: 3 columns (Right Hand | Armor | Left Hand) with icons
                        egui::Grid::new("equipped_gear_grid")
                            .num_columns(3)
                            .spacing([4.0, 4.0])
                            .show(ui, |ui| {
                                // Row 0: Empty | Head | Empty
                                ui.allocate_space(egui::vec2(ICON_DISPLAY_SIZE + 8.0, 1.0));
                                equipment_icon_slot(ui, &equipment_vm.head, ICON_DISPLAY_SIZE);
                                ui.allocate_space(egui::vec2(ICON_DISPLAY_SIZE + 8.0, 1.0));
                                ui.end_row();

                                // Row 1: R-Hand 1 | Chest | L-Hand 1
                                equipment_icon_slot(
                                    ui,
                                    &equipment_vm.right_hand_armaments[0],
                                    ICON_DISPLAY_SIZE,
                                );
                                equipment_icon_slot(ui, &equipment_vm.chest, ICON_DISPLAY_SIZE);
                                equipment_icon_slot(
                                    ui,
                                    &equipment_vm.left_hand_armaments[0],
                                    ICON_DISPLAY_SIZE,
                                );
                                ui.end_row();

                                // Row 2: R-Hand 2 | Arms | L-Hand 2
                                equipment_icon_slot(
                                    ui,
                                    &equipment_vm.right_hand_armaments[1],
                                    ICON_DISPLAY_SIZE,
                                );
                                equipment_icon_slot(ui, &equipment_vm.arms, ICON_DISPLAY_SIZE);
                                equipment_icon_slot(
                                    ui,
                                    &equipment_vm.left_hand_armaments[1],
                                    ICON_DISPLAY_SIZE,
                                );
                                ui.end_row();

                                // Row 3: R-Hand 3 | Legs | L-Hand 3
                                equipment_icon_slot(
                                    ui,
                                    &equipment_vm.right_hand_armaments[2],
                                    ICON_DISPLAY_SIZE,
                                );
                                equipment_icon_slot(ui, &equipment_vm.legs, ICON_DISPLAY_SIZE);
                                equipment_icon_slot(
                                    ui,
                                    &equipment_vm.left_hand_armaments[2],
                                    ICON_DISPLAY_SIZE,
                                );
                                ui.end_row();
                            });
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Armaments (Ammo) - 4 columns: Arrow1 | Arrow2 | Bolt1 | Bolt2
                    Frame::none().show(ui, |ui| {
                        section_header(ui, "ARMAMENTS");

                        egui::Grid::new("ammo_grid")
                            .num_columns(4)
                            .spacing([4.0, 4.0])
                            .show(ui, |ui| {
                                equipment_icon_slot(ui, &equipment_vm.arrows[0], ICON_DISPLAY_SIZE);
                                equipment_icon_slot(ui, &equipment_vm.arrows[1], ICON_DISPLAY_SIZE);
                                equipment_icon_slot(ui, &equipment_vm.bolts[0], ICON_DISPLAY_SIZE);
                                equipment_icon_slot(ui, &equipment_vm.bolts[1], ICON_DISPLAY_SIZE);
                                ui.end_row();
                            });
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Talismans - 4 columns with icons
                    Frame::none().show(ui, |ui| {
                        section_header(ui, "TALISMANS");

                        egui::Grid::new("talisman_grid")
                            .num_columns(4)
                            .spacing([4.0, 4.0])
                            .show(ui, |ui| {
                                for (i, talisman) in equipment_vm.talismans.iter().enumerate() {
                                    let available = (i as u32) < equipment_vm.talisman_count;
                                    if available {
                                        equipment_icon_slot(ui, talisman, ICON_DISPLAY_SIZE);
                                    } else {
                                        locked_slot(ui, ICON_DISPLAY_SIZE);
                                    }
                                }
                                ui.end_row();
                            });
                    });
                });

                // ═══════════════════════════════════════════════════════════════
                // COLUMN 3: Quick Items & Pouch (with icons in grid layout)
                // ═══════════════════════════════════════════════════════════════
                ui.vertical(|ui| {
                    // Quick Items - 5x2 grid
                    Frame::none().show(ui, |ui| {
                        section_header(ui, "QUICK ITEMS");

                        egui::Grid::new("quickitems_grid")
                            .num_columns(5)
                            .spacing([2.0, 4.0])
                            .show(ui, |ui| {
                                for (i, item) in equipment_vm.quickitems.iter().enumerate() {
                                    equipment_icon_slot(ui, item, ICON_DISPLAY_SIZE);
                                    if (i + 1) % 5 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Pouch - 3x2 grid
                    Frame::none().show(ui, |ui| {
                        section_header(ui, "POUCH");

                        egui::Grid::new("pouch_grid")
                            .num_columns(3)
                            .spacing([2.0, 4.0])
                            .show(ui, |ui| {
                                for (i, item) in equipment_vm.pouch.iter().enumerate() {
                                    equipment_icon_slot(ui, item, ICON_DISPLAY_SIZE);
                                    if (i + 1) % 3 == 0 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                });

                ui.end_row();
            });
    }
}
