pub mod general {
    use eframe::egui::{self, Ui, Color32, RichText, Frame, Rounding};
    use crate::vm::{general::general_view_model::Gender, vm::vm::ViewModel};
    use crate::ui::tokens::{colors, spacing, typography};

    /// Deep gray card background (darker than app background)
    const CARD_BG: Color32 = Color32::from_rgb(30, 30, 35);

    /// Section header styling
    fn section_header(ui: &mut Ui, text: &str) {
        ui.label(
            RichText::new(text)
                .size(typography::TEXT_SM)
                .color(colors::CAT_SUBTEXT0)
                .strong()
        );
        ui.add_space(4.0);
    }

    /// Stat row with label on left, value on right (game style)
    fn stat_row(ui: &mut Ui, label: &str, value: &str) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(label)
                    .size(typography::TEXT_SM)
                    .color(colors::CAT_SUBTEXT1)
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(value)
                        .size(typography::TEXT_SM)
                        .color(colors::CAT_TEXT)
                        .monospace()
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
                    .color(colors::CAT_SUBTEXT1)
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} / {}", current, max))
                        .size(typography::TEXT_SM)
                        .color(colors::CAT_TEXT)
                        .monospace()
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
                    .color(colors::CAT_OVERLAY1)
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
                        .color(color)
                )
                .sense(egui::Sense::click())
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
                    .color(if available { colors::CAT_OVERLAY0 } else { colors::CAT_SURFACE2 })
                    .monospace()
            );

            if !available {
                ui.label(
                    RichText::new("🔒")
                        .size(typography::TEXT_SM)
                        .color(colors::CAT_SURFACE2)
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
                        .color(color)
                )
                .sense(egui::Sense::click())
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
                    .color(color)
            )
            .sense(egui::Sense::click())
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
                        .color(color)
                )
                .sense(egui::Sense::click())
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
                    .monospace()
            );

            let (display_name, color) = if name == "Empty" || name.is_empty() {
                ("—", colors::CAT_OVERLAY0)
            } else {
                (name, colors::CAT_TEXT)
            };

            ui.label(
                RichText::new(display_name)
                    .size(typography::TEXT_XS)
                    .color(color)
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

    pub fn general(ui: &mut Ui, vm: &mut ViewModel) {
        let general_vm = &vm.slots[vm.index].general_vm;
        let stats_vm = &vm.slots[vm.index].stats_vm;
        let equipment_vm = &vm.slots[vm.index].equipment_vm;

        let gender_str = match general_vm.gender {
            Gender::Male => "♂",
            Gender::Female => "♀",
            Gender::Uknown => "?",
        };

        // Character Header
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(general_vm.character_name.trim_matches('\0'))
                    .size(typography::HEADING_MD)
                    .color(colors::CAT_TEXT)
                    .strong()
            );
            ui.label(
                RichText::new(gender_str)
                    .size(typography::HEADING_MD)
                    .color(colors::CAT_SUBTEXT0)
            );
            ui.add_space(12.0);
            ui.label(
                RichText::new(format!("Lv. {}", stats_vm.level))
                    .size(typography::HEADING_SM)
                    .color(colors::CAT_YELLOW)
                    .strong()
            );
            ui.label(
                RichText::new(stats_vm.arche_type.to_string())
                    .size(typography::TEXT_SM)
                    .color(colors::CAT_SUBTEXT0)
            );
        });

        spacing::space_sm(ui);

        // Main content: three columns using egui columns
        ui.columns(3, |columns| {
            // ═══════════════════════════════════════════════════════════════
            // COLUMN 1: Character Status (matching game layout)
            // ═══════════════════════════════════════════════════════════════
            Frame::none()
                .fill(CARD_BG)
                .rounding(Rounding::same(6.0))
                .inner_margin(12.0)
                .show(&mut columns[0], |ui| {
                    section_header(ui, "CHARACTER STATUS");

                    // Level & Runes
                    stat_row(ui, "Level", &stats_vm.level.to_string());
                    stat_row(ui, "Runes Held", &format_number(stats_vm.souls));

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Attributes
                    stat_row(ui, "Vigor", &stats_vm.vigor.to_string());
                    stat_row(ui, "Mind", &stats_vm.mind.to_string());
                    stat_row(ui, "Endurance", &stats_vm.endurance.to_string());
                    stat_row(ui, "Strength", &stats_vm.strength.to_string());
                    stat_row(ui, "Dexterity", &stats_vm.dexterity.to_string());
                    stat_row(ui, "Intelligence", &stats_vm.intelligence.to_string());
                    stat_row(ui, "Faith", &stats_vm.faith.to_string());
                    stat_row(ui, "Arcane", &stats_vm.arcane.to_string());

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Derived stats
                    stat_row_dual(ui, "HP", stats_vm.hp, stats_vm.max_hp);
                    stat_row_dual(ui, "FP", stats_vm.fp, stats_vm.max_fp);
                    stat_row(ui, "Stamina", &stats_vm.max_stamina.to_string());

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Additional info
                    stat_row(ui, "Weapon Level", &general_vm.weapon_level.to_string());
                    stat_row(ui, "Total Runes", &format_number(stats_vm.soulsmemory));

                    // DLC Blessings (only show if > 0)
                    if stats_vm.scadutree > 0 || stats_vm.spirit_ash > 0 {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);

                        if stats_vm.scadutree > 0 {
                            stat_row(ui, "Scadutree", &format!("{}/20", stats_vm.scadutree));
                        }
                        if stats_vm.spirit_ash > 0 {
                            stat_row(ui, "Spirit Ash", &format!("{}/10", stats_vm.spirit_ash));
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Location
                    stat_row(ui, "Location", general_vm.map_id.display_name());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("").size(typography::TEXT_XS));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(general_vm.map_id.to_string())
                                    .size(typography::TEXT_XS)
                                    .color(colors::CAT_OVERLAY0)
                                    .monospace()
                            );
                        });
                    });
                });

            // ═══════════════════════════════════════════════════════════════
            // COLUMN 2: Equipment (Grid layout like game UI)
            // ═══════════════════════════════════════════════════════════════
            // Equipped Gear - Grid: Right Hand | Armor | Left Hand
            Frame::none()
                .fill(CARD_BG)
                .rounding(Rounding::same(6.0))
                .inner_margin(12.0)
                .show(&mut columns[1], |ui| {
                    section_header(ui, "EQUIPPED GEAR");

                    // Grid: 3 columns (Right Hand | Armor | Left Hand)
                    let col_width = (ui.available_width() - 16.0) / 3.0;
                    egui::Grid::new("equipped_gear_grid")
                        .num_columns(3)
                        .spacing([8.0, 6.0])
                        .min_col_width(col_width)
                        .show(ui, |ui| {
                            // Row 0: Empty | Head | Empty
                            ui.label(RichText::new("").size(typography::TEXT_SM)); // empty
                            gear_slot_centered(ui, &equipment_vm.head.name);
                            ui.label(RichText::new("").size(typography::TEXT_SM)); // empty
                            ui.end_row();

                            // Row 1: R-Hand 1 | Chest | L-Hand 1
                            gear_slot(ui, &equipment_vm.right_hand_armaments[0].name);
                            gear_slot_centered(ui, &equipment_vm.chest.name);
                            gear_slot(ui, &equipment_vm.left_hand_armaments[0].name);
                            ui.end_row();

                            // Row 2: R-Hand 2 | Arms | L-Hand 2
                            gear_slot(ui, &equipment_vm.right_hand_armaments[1].name);
                            gear_slot_centered(ui, &equipment_vm.arms.name);
                            gear_slot(ui, &equipment_vm.left_hand_armaments[1].name);
                            ui.end_row();

                            // Row 3: R-Hand 3 | Legs | L-Hand 3
                            gear_slot(ui, &equipment_vm.right_hand_armaments[2].name);
                            gear_slot_centered(ui, &equipment_vm.legs.name);
                            gear_slot(ui, &equipment_vm.left_hand_armaments[2].name);
                            ui.end_row();
                        });
                });

            spacing::space_sm(&mut columns[1]);

            // Armaments (Ammo) - 4 columns: Arrow1 | Arrow2 | Bolt1 | Bolt2
            Frame::none()
                .fill(CARD_BG)
                .rounding(Rounding::same(6.0))
                .inner_margin(12.0)
                .show(&mut columns[1], |ui| {
                    section_header(ui, "ARMAMENTS");

                    let col_width = (ui.available_width() - 18.0) / 4.0;
                    egui::Grid::new("ammo_grid")
                        .num_columns(4)
                        .spacing([6.0, 4.0])
                        .min_col_width(col_width)
                        .show(ui, |ui| {
                            gear_slot(ui, &equipment_vm.arrows[0].name);
                            gear_slot(ui, &equipment_vm.arrows[1].name);
                            gear_slot(ui, &equipment_vm.bolts[0].name);
                            gear_slot(ui, &equipment_vm.bolts[1].name);
                            ui.end_row();
                        });
                });

            spacing::space_sm(&mut columns[1]);

            // Talismans - 4 columns
            Frame::none()
                .fill(CARD_BG)
                .rounding(Rounding::same(6.0))
                .inner_margin(12.0)
                .show(&mut columns[1], |ui| {
                    section_header(ui, "TALISMANS");

                    let col_width = (ui.available_width() - 18.0) / 4.0;
                    egui::Grid::new("talisman_grid")
                        .num_columns(4)
                        .spacing([6.0, 4.0])
                        .min_col_width(col_width)
                        .show(ui, |ui| {
                            for (i, talisman) in equipment_vm.talismans.iter().enumerate() {
                                let available = (i as u32) < equipment_vm.talisman_count;
                                if available {
                                    gear_slot(ui, &talisman.name);
                                } else {
                                    ui.label(
                                        RichText::new("🔒")
                                            .size(typography::TEXT_SM)
                                            .color(colors::CAT_SURFACE2)
                                    );
                                }
                            }
                            ui.end_row();
                        });
                });

            // ═══════════════════════════════════════════════════════════════
            // COLUMN 3: Quick Items & Pouch
            // ═══════════════════════════════════════════════════════════════
            // Quick Items
            Frame::none()
                .fill(CARD_BG)
                .rounding(Rounding::same(6.0))
                .inner_margin(12.0)
                .show(&mut columns[2], |ui| {
                    section_header(ui, "QUICK ITEMS");

                    for (i, item) in equipment_vm.quickitems.iter().enumerate() {
                        quick_slot(ui, i, &item.name);
                    }
                });

            spacing::space_sm(&mut columns[2]);

            // Pouch
            Frame::none()
                .fill(CARD_BG)
                .rounding(Rounding::same(6.0))
                .inner_margin(12.0)
                .show(&mut columns[2], |ui| {
                    section_header(ui, "POUCH");

                    for (i, item) in equipment_vm.pouch.iter().enumerate() {
                        quick_slot(ui, i, &item.name);
                    }
                });
        });
    }
}
