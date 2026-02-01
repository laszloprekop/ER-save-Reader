pub mod verification_view {
    use eframe::egui::{self, Ui, RichText, ScrollArea, Color32};
    use crate::vm::verification_vm::{VerificationViewModel, VerificationFilterStatus, DetectionCategory};
    use crate::ui::style::{
        TABLE_MONO_SIZE,
        CAT_RED, CAT_GREEN, CAT_YELLOW, CAT_PEACH, CAT_TEAL, CAT_SUBTEXT, CAT_OVERLAY,
    };
    use crate::save::common::save_slot::EquipInventoryData;
    use crate::discovery::inventory_verification::{
        InventoryVerificationService, VerificationStats, InventoryMismatchReport,
        UniqueItemCategory, VerificationConfidence, UNIQUE_ITEMS,
    };
    use std::collections::HashSet;

    /// Main verification comparison view
    pub fn verification_view(ui: &mut Ui, vm: &mut VerificationViewModel) {
        if !vm.has_records() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("No verification records loaded.\n\nVerification records file not found or no records for current slot.")
                    .color(CAT_OVERLAY));
            });
            return;
        }

        // Summary header
        let summary = vm.get_summary();
        ui.horizontal(|ui| {
            let color = if summary.agreement_rate >= 80.0 {
                CAT_GREEN
            } else if summary.agreement_rate >= 50.0 {
                CAT_YELLOW
            } else {
                CAT_RED
            };
            ui.label(RichText::new(format!(
                "Agreement: {:.1}% ({}/{} matching)",
                summary.agreement_rate, summary.matches, summary.total
            )).color(color).size(16.0).strong());
        });
        ui.separator();

        // Category breakdown
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("By Category:").color(CAT_SUBTEXT));
            for (cat, stats) in &summary.by_category {
                let color = if stats.rate >= 80.0 {
                    CAT_GREEN
                } else if stats.rate >= 50.0 {
                    CAT_YELLOW
                } else {
                    CAT_RED
                };
                ui.label(RichText::new(format!(
                    "{}: {:.0}% ({}/{})",
                    cat, stats.rate, stats.matches, stats.total
                )).color(color));
                ui.separator();
            }
        });
        ui.separator();

        // Flagged detections section
        let flagged_count = vm.suspicious_count();
        let formula_error_count = vm.formula_error_count();
        let informational_count = vm.informational_count();

        if flagged_count > 0 {
            let flagged_by_category = vm.suspicious_by_reason();

            // Show formula errors prominently if any exist
            if formula_error_count > 0 {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("⚠ Formula Errors: {}", formula_error_count))
                        .color(CAT_RED)
                        .strong());
                    ui.label(RichText::new(" (manual=true but auto=false - needs investigation)")
                        .color(CAT_RED)
                        .small());
                });
            }

            // Show informational counts
            if informational_count > 0 {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("ℹ Informational: {}", informational_count))
                        .color(CAT_PEACH)
                        .small());
                    for (category, count) in &flagged_by_category {
                        if category != "Formula Error" {
                            ui.label(RichText::new(format!(" | {}: {}", category, count))
                                .color(CAT_PEACH)
                                .small());
                        }
                    }
                });
            }

            // Collapsible flagged detections list
            let header_text = if formula_error_count > 0 {
                format!("Show flagged detections ({} errors, {} info)", formula_error_count, informational_count)
            } else {
                format!("Show flagged detections ({})", flagged_count)
            };
            egui::CollapsingHeader::new(RichText::new(header_text).small())
                .default_open(formula_error_count > 0)  // Auto-open if there are errors
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(
                            "Flag ID      | Name                                                                              | Category         | Region           | Status"
                        ).color(CAT_YELLOW).monospace().size(TABLE_MONO_SIZE));
                    });

                    for det in &vm.suspicious_detections {
                        // Color based on detection category
                        let row_color = match det.detection_category {
                            DetectionCategory::FormulaError => CAT_RED,
                            DetectionCategory::PendingVerification => CAT_PEACH,
                            DetectionCategory::UndiscoveredRegion => CAT_YELLOW,
                        };

                        let row = format!(
                            "{:<12} | {:<80} | {:<16} | {:<16} | {}",
                            det.flag_id,
                            &det.flag_name,
                            &det.flag_category,
                            &det.region,
                            det.detection_category.as_str()
                        );
                        let response = ui.add(egui::Label::new(RichText::new(&row)
                            .color(row_color)
                            .monospace()
                            .size(TABLE_MONO_SIZE))
                            .sense(egui::Sense::click()))
                            .on_hover_text(&det.description);

                        // Context menu with copy options
                        response.context_menu(|ui| {
                            if ui.button("Copy Description").clicked() {
                                ui.output_mut(|o| o.copied_text = det.description.clone());
                                ui.close_menu();
                            }
                            if ui.button("Copy Flag ID").clicked() {
                                ui.output_mut(|o| o.copied_text = det.flag_id.to_string());
                                ui.close_menu();
                            }
                            if ui.button("Copy Flag Name").clicked() {
                                ui.output_mut(|o| o.copied_text = det.flag_name.clone());
                                ui.close_menu();
                            }
                            ui.separator();
                            ui.label(RichText::new("Details:").small().strong());
                            ui.label(RichText::new(&det.description).small());
                        });
                    }
                });
            ui.separator();
        }

        // Discovered regions info
        if !vm.discovered_regions.is_empty() {
            let mut regions: Vec<_> = vm.discovered_regions.iter().collect();
            regions.sort();
            egui::CollapsingHeader::new(RichText::new(format!("Discovered Regions ({})", regions.len())).small())
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for region in regions {
                            ui.label(RichText::new(region).color(CAT_TEAL).small());
                            ui.separator();
                        }
                    });
                });
            ui.separator();
        }

        // Filter controls - Status
        ui.horizontal(|ui| {
            ui.label(RichText::new("Filter:").color(CAT_SUBTEXT));
            ui.selectable_value(&mut vm.filter_status, VerificationFilterStatus::All, "All");
            ui.selectable_value(&mut vm.filter_status, VerificationFilterStatus::Matching, "Matching");
            ui.selectable_value(&mut vm.filter_status, VerificationFilterStatus::Mismatched, "Mismatched");
        });

        // Filter controls - Category (wrapped to handle many categories)
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Category:").color(CAT_SUBTEXT));
            if ui.selectable_label(vm.filter_category.is_none(), "All").clicked() {
                vm.filter_category = None;
            }

            // Show category buttons
            let categories = vm.get_categories();
            for cat in categories {
                let is_selected = vm.filter_category.as_ref() == Some(&cat);
                if ui.selectable_label(is_selected, &cat).clicked() {
                    vm.filter_category = Some(cat);
                }
            }
        });
        ui.separator();

        // Results count
        let filtered_count = vm.filtered_count();
        ui.label(RichText::new(format!("Showing {} records", filtered_count))
            .color(CAT_OVERLAY)
            .small());
        ui.separator();

        // Column header
        ui.horizontal(|ui| {
            ui.label(RichText::new(
                "Flag ID      | Name                                                                              | Category         | Manual | Auto   | Match"
            ).color(CAT_YELLOW).monospace().size(TABLE_MONO_SIZE));
        });
        ui.separator();

        // Records table (horizontal scroll for wide content)
        ScrollArea::both()
            .auto_shrink(false)
            .show(ui, |ui| {
                for record in vm.get_filtered_records() {
                    let match_color = if record.statuses_align {
                        CAT_GREEN
                    } else {
                        CAT_RED
                    };

                    let row = format!(
                        "{:<12} | {:<80} | {:<16} | {:<6} | {:<6} | {}",
                        record.flag_id,
                        &record.flag_name,
                        &record.flag_category,
                        if record.user_marked_complete { "TRUE" } else { "false" },
                        if record.webapp_parsed_status { "TRUE" } else { "false" },
                        if record.statuses_align { "OK" } else { "DIFF" }
                    );

                    // Make row clickable for context menu
                    let response = ui.add(egui::Label::new(
                        RichText::new(&row).color(match_color).monospace().size(TABLE_MONO_SIZE)
                    ).sense(egui::Sense::click()));

                    // Context menu
                    response.context_menu(|ui| {
                        if ui.button("Copy Flag ID").clicked() {
                            ui.output_mut(|o| o.copied_text = record.flag_id.to_string());
                            ui.close_menu();
                        }
                        if ui.button("Copy Flag Name").clicked() {
                            ui.output_mut(|o| o.copied_text = record.flag_name.clone());
                            ui.close_menu();
                        }
                        if ui.button("Copy Offset Info").clicked() {
                            ui.output_mut(|o| o.copied_text = format!(
                                "offset: {}, bit: {}",
                                record.computed_byte_offset,
                                record.computed_bit_position
                            ));
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label(format!("Type: {}", record.flag_type));
                        ui.label(format!("Region: {}", record.flag_region));
                        ui.label(format!("Offset: {}", record.computed_byte_offset));
                        ui.label(format!("Bit: {}", record.computed_bit_position));
                    });
                }
            });
    }

    /// Inventory verification summary view
    /// Shows the verification triangle: Flag status vs Inventory possession
    pub fn inventory_verification_summary(
        ui: &mut Ui,
        set_flags: &HashSet<u32>,
        inventory: Option<&EquipInventoryData>,
    ) {
        let inventory = match inventory {
            Some(inv) => inv,
            None => {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("No inventory data available.\n\nLoad a save file to view inventory verification.")
                        .color(CAT_OVERLAY));
                });
                return;
            }
        };

        // Get verification stats
        let stats = InventoryVerificationService::get_verification_stats(set_flags, inventory);
        let report = InventoryVerificationService::find_mismatches(set_flags, inventory);

        // Summary header with verification triangle icon
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔺 Inventory Verification Triangle").size(16.0).strong().color(CAT_TEAL));
        });
        ui.separator();

        // Overall stats
        ui.horizontal_wrapped(|ui| {
            let match_rate_color = if stats.match_rate >= 0.9 {
                CAT_GREEN
            } else if stats.match_rate >= 0.7 {
                CAT_YELLOW
            } else {
                CAT_RED
            };

            ui.label(RichText::new(format!(
                "Match Rate: {:.1}% ({}/{} verified)",
                stats.match_rate * 100.0,
                stats.matches,
                stats.total_verifiable
            )).color(match_rate_color).strong());

            ui.separator();

            ui.label(RichText::new(format!(
                "Database: {} unique items",
                stats.total_unique_items
            )).color(CAT_SUBTEXT));
        });
        ui.separator();

        // Count high vs low confidence items
        let high_confidence_items: Vec<_> = UNIQUE_ITEMS.iter()
            .filter(|i| matches!(i.confidence, VerificationConfidence::VeryHigh | VerificationConfidence::High))
            .collect();
        let low_confidence_items: Vec<_> = UNIQUE_ITEMS.iter()
            .filter(|i| matches!(i.confidence, VerificationConfidence::Low | VerificationConfidence::Medium))
            .collect();

        // Show confidence breakdown
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!(
                "Verifiable: {} items (high/very high confidence)",
                high_confidence_items.len()
            )).color(CAT_GREEN).small());
            ui.separator();
            ui.label(RichText::new(format!(
                "Pending: {} items (no formula/low confidence)",
                low_confidence_items.len()
            )).color(CAT_OVERLAY).small());
        });
        ui.separator();

        // Category breakdown
        let categories = [
            (UniqueItemCategory::Remembrance, "Remembrances", CAT_PEACH),
            (UniqueItemCategory::GreatRune, "Great Runes", CAT_YELLOW),
            (UniqueItemCategory::Cookbook, "Cookbooks", CAT_GREEN),
            (UniqueItemCategory::Whetblade, "Whetblades", Color32::from_rgb(150, 200, 255)),
            (UniqueItemCategory::AshOfWar, "Ashes of War", Color32::from_rgb(200, 150, 255)),
            (UniqueItemCategory::SpiritAsh, "Spirit Ashes", Color32::from_rgb(150, 255, 200)),
            (UniqueItemCategory::Talisman, "Talismans", Color32::from_rgb(255, 200, 150)),
            (UniqueItemCategory::BossWeapon, "Boss Weapons", CAT_RED),
            (UniqueItemCategory::KeyItem, "Key Items", CAT_TEAL),
        ];

        ui.horizontal_wrapped(|ui| {
            for (cat, name, color) in &categories {
                let cat_items: Vec<_> = UNIQUE_ITEMS.iter()
                    .filter(|i| i.category == *cat)
                    .collect();

                if cat_items.is_empty() {
                    continue;
                }

                // Count high confidence items in this category
                let high_conf_count = cat_items.iter()
                    .filter(|i| matches!(i.confidence, VerificationConfidence::VeryHigh | VerificationConfidence::High))
                    .count();

                let owned: usize = cat_items.iter()
                    .filter(|item| set_flags.contains(&item.event_flag))
                    .count();

                // Dim categories with no verifiable items
                let display_color = if high_conf_count == 0 {
                    CAT_OVERLAY
                } else {
                    *color
                };

                ui.label(RichText::new(format!("{}: {}/{}", name, owned, cat_items.len()))
                    .color(display_color)
                    .small());
                ui.separator();
            }
        });
        ui.separator();

        // Mismatch sections
        if !report.flag_set_no_item.is_empty() {
            egui::CollapsingHeader::new(RichText::new(format!(
                "⚠ Flag Set but No Item ({})",
                report.flag_set_no_item.len()
            )).color(CAT_YELLOW))
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(RichText::new("These flags are set but the corresponding item is not in inventory. Could indicate: item was used/sold, or flag detection error.")
                        .color(CAT_SUBTEXT)
                        .small());
                    ui.separator();

                    for item in &report.flag_set_no_item {
                        let row = format!(
                            "{:<12} | {:<40} | {}",
                            item.event_flag,
                            item.name,
                            item.category.as_str()
                        );
                        let response = ui.add(egui::Label::new(
                            RichText::new(&row).color(CAT_YELLOW).monospace().size(TABLE_MONO_SIZE)
                        ).sense(egui::Sense::click()));

                        response.context_menu(|ui| {
                            if ui.button("Copy Flag ID").clicked() {
                                ui.output_mut(|o| o.copied_text = item.event_flag.to_string());
                                ui.close_menu();
                            }
                            if ui.button("Copy Item Name").clicked() {
                                ui.output_mut(|o| o.copied_text = item.name.to_string());
                                ui.close_menu();
                            }
                        });
                    }
                });
        }

        if !report.item_present_no_flag.is_empty() {
            // Separate by confidence level
            let high_conf_mismatches: Vec<_> = report.item_present_no_flag.iter()
                .filter(|i| matches!(i.confidence, VerificationConfidence::VeryHigh | VerificationConfidence::High))
                .collect();
            let low_conf_items: Vec<_> = report.item_present_no_flag.iter()
                .filter(|i| matches!(i.confidence, VerificationConfidence::Low | VerificationConfidence::Medium))
                .collect();

            // Show high-confidence mismatches as errors (actual formula bugs)
            if !high_conf_mismatches.is_empty() {
                egui::CollapsingHeader::new(RichText::new(format!(
                    "❌ Item Present but Flag Not Set ({})",
                    high_conf_mismatches.len()
                )).color(CAT_RED))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("These items are in inventory but the flag is not detected. Indicates: flag formula bug, or item obtained through non-standard means.")
                            .color(CAT_SUBTEXT)
                            .small());
                        ui.separator();

                        for item in high_conf_mismatches {
                            let row = format!(
                                "{:<12} | {:<40} | {}",
                                item.event_flag,
                                item.name,
                                item.category.as_str()
                            );
                            let response = ui.add(egui::Label::new(
                                RichText::new(&row).color(CAT_RED).monospace().size(TABLE_MONO_SIZE)
                            ).sense(egui::Sense::click()));

                            response.context_menu(|ui| {
                                if ui.button("Copy Flag ID").clicked() {
                                    ui.output_mut(|o| o.copied_text = item.event_flag.to_string());
                                    ui.close_menu();
                                }
                                if ui.button("Copy Item Name").clicked() {
                                    ui.output_mut(|o| o.copied_text = item.name.to_string());
                                    ui.close_menu();
                                }
                            });
                        }
                    });
            }

            // Show low-confidence items as informational (no formula yet)
            if !low_conf_items.is_empty() {
                egui::CollapsingHeader::new(RichText::new(format!(
                    "ℹ No Formula Yet ({})",
                    low_conf_items.len()
                )).color(CAT_OVERLAY))
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(RichText::new("These items use flag ranges (520xxx) where no formula has been discovered yet. They cannot be verified until the 520000 block offset is discovered.")
                            .color(CAT_SUBTEXT)
                            .small());
                        ui.separator();

                        for item in low_conf_items {
                            let row = format!(
                                "{:<12} | {:<40} | {}",
                                item.event_flag,
                                item.name,
                                item.category.as_str()
                            );
                            ui.label(RichText::new(&row).color(CAT_OVERLAY).monospace().size(TABLE_MONO_SIZE));
                        }
                    });
            }
        }

        // Matches section (collapsed by default)
        if !report.matches.is_empty() {
            egui::CollapsingHeader::new(RichText::new(format!(
                "✓ Verified Matches ({})",
                report.matches.len()
            )).color(CAT_GREEN))
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(RichText::new("Flag status and inventory possession match. High confidence in detection accuracy.")
                        .color(CAT_SUBTEXT)
                        .small());
                    ui.separator();

                    ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for item in &report.matches {
                                let row = format!(
                                    "{:<12} | {:<40} | {}",
                                    item.event_flag,
                                    item.name,
                                    item.category.as_str()
                                );
                                ui.label(RichText::new(&row).color(CAT_GREEN).monospace().size(TABLE_MONO_SIZE));
                            }
                        });
                });
        }
    }
}
