pub mod verification_view {
    use eframe::egui::{self, Ui, Color32, RichText, ScrollArea};
    use crate::vm::verification_vm::{VerificationViewModel, VerificationFilterStatus, DetectionCategory};

    // =========================================================================
    // Catppuccin Frappé Color Palette
    // =========================================================================
    const CAT_RED: Color32 = Color32::from_rgb(231, 130, 132);      // #e78284
    const CAT_GREEN: Color32 = Color32::from_rgb(166, 209, 137);    // #a6d189
    const CAT_YELLOW: Color32 = Color32::from_rgb(229, 200, 144);   // #e5c890
    const CAT_PEACH: Color32 = Color32::from_rgb(239, 159, 118);    // #ef9f76
    const CAT_TEAL: Color32 = Color32::from_rgb(129, 200, 190);     // #81c8be
    const CAT_SUBTEXT: Color32 = Color32::from_rgb(165, 173, 206);  // #a5adce
    const CAT_OVERLAY: Color32 = Color32::from_rgb(131, 139, 167);  // #838ba7

    /// Monospace font size (85% of default ~14px)
    const MONO_SIZE: f32 = 12.0;

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
                        ).color(CAT_YELLOW).monospace().size(MONO_SIZE));
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
                            .size(MONO_SIZE))
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

        // Filter controls - Category
        ui.horizontal(|ui| {
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
            ).color(CAT_YELLOW).monospace().size(MONO_SIZE));
        });
        ui.separator();

        // Records table (horizontal scroll for wide content)
        ScrollArea::both()
            .auto_shrink(false)
            .show(ui, |ui| {
                for record in vm.get_filtered_records() {
                    let match_color = if record.matches {
                        CAT_GREEN
                    } else {
                        CAT_RED
                    };

                    let row = format!(
                        "{:<12} | {:<80} | {:<16} | {:<6} | {:<6} | {}",
                        record.flag_id,
                        &record.flag_name,
                        &record.flag_category,
                        if record.manual_status { "TRUE" } else { "false" },
                        if record.auto_status { "TRUE" } else { "false" },
                        if record.matches { "OK" } else { "DIFF" }
                    );

                    // Make row clickable for context menu
                    let response = ui.add(egui::Label::new(
                        RichText::new(&row).color(match_color).monospace().size(MONO_SIZE)
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
}
