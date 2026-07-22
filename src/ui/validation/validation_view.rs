//! Validation view for displaying save file analysis results.

use eframe::egui::{RichText, ScrollArea, Ui};
use serde::Serialize;

use crate::save::save::save::Save;
use crate::vm::slot::slot_view_model::SlotViewModel;
use crate::ui::components::filter::fuzzy_match_default;
use crate::ui::components::export::{ExportToolbar, ExportFormat, to_json};
use crate::ui::tokens::{colors, spacing};
use crate::ui::components::legend::nav_icons;

use super::{ValidationState, ValidationReport, ValidationIssue, Severity};

/// Run validation on the save file and generate a report.
pub fn run_validation(save: &Save, slots: &[SlotViewModel]) -> ValidationReport {
    let mut report = ValidationReport::new();

    for (slot_idx, active) in save.save_type.active_slots().iter().enumerate() {
        if !*active {
            continue;
        }

        let slot = &slots[slot_idx];
        let char_name = slot.general_vm.character_name.trim_matches('\0');

        // Basic info about the slot
        report.add_info(ValidationIssue::info(
            "Character",
            slot_idx,
            &format!("Slot {} - {}", slot_idx, char_name),
            &format!("Level {}, {} runes", slot.stats_vm.level, slot.stats_vm.souls),
        ));

        // Check for weapons validation
        if !crate::util::validator::validator::Validator::is_weapons_valid_detailed(save, slot_idx, &mut report) {
            // Issues already added to report
        }

        // Check for items validation
        if !crate::util::validator::validator::Validator::is_items_valid_detailed(save, slot_idx, &mut report) {
            // Issues already added to report
        }

        // Check for armor validation
        if !crate::util::validator::validator::Validator::is_armor_valid_detailed(save, slot_idx, &mut report) {
            // Issues already added to report
        }

        // Check for physics validation
        if !crate::util::validator::validator::Validator::is_physics_valid_detailed(save, slot_idx, &mut report) {
            // Issues already added to report
        }

        // Check for equipped items validation
        if !crate::util::validator::validator::Validator::is_equipped_items_valid_detailed(save, slot_idx, &mut report) {
            // Issues already added to report
        }

        // Additional checks

        // Check stat ranges
        check_stat_ranges(slot, slot_idx, &mut report);

        // Check inventory consistency
        check_inventory_counts(slot, slot_idx, &mut report);
    }

    report
}

fn check_stat_ranges(slot: &SlotViewModel, slot_idx: usize, report: &mut ValidationReport) {
    let stats = &slot.stats_vm;

    // Level should be between 1 and 713 (max possible)
    if stats.level == 0 || stats.level > 713 {
        report.add_warning(ValidationIssue::warning(
            "Stats",
            slot_idx,
            "Unusual level",
            &format!("Level {} is outside expected range (1-713)", stats.level),
        ));
    }

    // Individual stats should be between 1 and 99
    let stat_names = ["Vigor", "Mind", "Endurance", "Strength", "Dexterity", "Intelligence", "Faith", "Arcane"];
    let stat_values = [stats.vigor, stats.mind, stats.endurance, stats.strength, stats.dexterity, stats.intelligence, stats.faith, stats.arcane];

    for (name, value) in stat_names.iter().zip(stat_values.iter()) {
        if *value == 0 || *value > 99 {
            report.add_warning(ValidationIssue::warning(
                "Stats",
                slot_idx,
                &format!("Unusual {} value", name),
                &format!("{} is {} (expected 1-99)", name, value),
            ));
        }
    }

    // DLC stats have limits
    if stats.scadutree > 20 {
        report.add_warning(ValidationIssue::warning(
            "Stats",
            slot_idx,
            "Unusual Scadutree level",
            &format!("Scadutree level {} exceeds maximum (20)", stats.scadutree),
        ));
    }

    if stats.spirit_ash > 10 {
        report.add_warning(ValidationIssue::warning(
            "Stats",
            slot_idx,
            "Unusual Spirit Ash level",
            &format!("Spirit Ash level {} exceeds maximum (10)", stats.spirit_ash),
        ));
    }
}

fn check_inventory_counts(slot: &SlotViewModel, slot_idx: usize, report: &mut ValidationReport) {
    let inv = &slot.inventory_vm;
    let storage = &inv.storage[0];

    // Report inventory counts as info
    let _total_items = storage.common_item_count + storage.key_item_count;
    report.add_info(ValidationIssue::info(
        "Inventory",
        slot_idx,
        "Inventory summary",
        &format!("{} common items, {} key items", storage.common_item_count, storage.key_item_count),
    ));

    // Check for suspiciously high item counts
    if storage.common_item_count > 2000 {
        report.add_warning(ValidationIssue::warning(
            "Inventory",
            slot_idx,
            "Very large inventory",
            &format!("{} common items is unusually high", storage.common_item_count),
        ));
    }
}

/// Export structures
#[derive(Serialize)]
struct ValidationExport {
    is_valid: bool,
    error_count: usize,
    warning_count: usize,
    info_count: usize,
    issues: Vec<IssueExport>,
}

#[derive(Serialize)]
struct IssueExport {
    severity: String,
    category: String,
    slot: usize,
    message: String,
    details: String,
}

/// Render the validation view.
pub fn validation_view(
    ui: &mut Ui,
    state: &mut ValidationState,
    save: &Save,
    slots: &[SlotViewModel],
) {
    // Title
    ui.horizontal(|ui| {
        ui.label(RichText::new(egui_phosphor::regular::SHIELD_CHECK).size(18.0));
        ui.add_space(spacing::XS);
        ui.label(RichText::new("Save File Validation").heading());
    });

    ui.add_space(spacing::SM);

    // Run validation button
    ui.horizontal(|ui| {
        if ui.button("Run Validation").clicked() {
            state.running = true;
            let report = run_validation(save, slots);
            state.set_report(report);
        }

        if state.report.is_some()
            && ui.button("Clear Results").clicked() {
                state.reset();
            }
    });

    ui.add_space(spacing::SM);

    // Show results
    if state.report.is_none() {
        ui.colored_label(colors::TEXT_SECONDARY, "Click 'Run Validation' to analyze the save file.");
        return;
    }

    // Extract data we need from report before mutable access
    let report = state.report.as_ref().unwrap();
    let is_valid = report.is_valid;
    let error_count = report.error_count();
    let warning_count = report.warning_count();
    let info_count = report.info_count();

    // Clone issues for iteration
    let all_issues: Vec<ValidationIssue> = report.all_issues().into_iter().cloned().collect();

    // For export
    let export_data = build_export(report);

    // Summary
    ui.horizontal(|ui| {
        let status_text = if is_valid { "Valid" } else { "Issues Found" };
        let status_color = if is_valid { colors::STATUS_COLLECTED } else { colors::STATUS_WARNING };
        let status_icon = if is_valid { egui_phosphor::regular::CHECK_CIRCLE } else { egui_phosphor::regular::WARNING };

        ui.label(RichText::new(status_icon).size(16.0).color(status_color));
        ui.label(RichText::new(status_text).strong().color(status_color));
    });

    ui.add_space(spacing::XS);

    // Counts
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} errors", error_count)).color(colors::CAT_RED));
        ui.label("|");
        ui.label(RichText::new(format!("{} warnings", warning_count)).color(colors::STATUS_WARNING));
        ui.label("|");
        ui.label(RichText::new(format!("{} info", info_count)).color(colors::STATUS_INFO));
    });

    ui.add_space(spacing::SM);

    // Filters
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.show_errors, "Errors");
        ui.checkbox(&mut state.show_warnings, "Warnings");
        ui.checkbox(&mut state.show_info, "Info");

        ui.add_space(spacing::LG);

        ui.label(RichText::new(nav_icons::SEARCH).color(colors::TEXT_SECONDARY));
        ui.text_edit_singleline(&mut state.search_query);
        if !state.search_query.is_empty() && ui.small_button(nav_icons::CLOSE).clicked() {
            state.search_query.clear();
        }
    });

    ui.add_space(spacing::SM);

    // Export
    let mut export_format = ExportFormat::Json;
    let mut export_filtered = false;
    let export_response = ExportToolbar::new("validation_export", &mut export_format, &mut export_filtered)
        .show(ui);

    if export_response.export_clicked || export_response.copy_clicked {
        let content = to_json(&export_data).unwrap_or_else(|e| format!("Error: {}", e));
        ui.output_mut(|o| o.copied_text = content);
    }

    ui.add_space(spacing::SM);
    ui.separator();
    ui.add_space(spacing::SM);

    // Issue list
    let show_errors = state.show_errors;
    let show_warnings = state.show_warnings;
    let show_info = state.show_info;
    let search_query = state.search_query.clone();

    ScrollArea::vertical().show(ui, |ui| {
        let mut idx = 0;

        for issue in &all_issues {
            // Filter by severity
            let show = match issue.severity {
                Severity::Error => show_errors,
                Severity::Warning => show_warnings,
                Severity::Info => show_info,
            };

            if !show {
                idx += 1;
                continue;
            }

            // Filter by search
            if !search_query.is_empty()
                && !fuzzy_match_default(&issue.message, &search_query)
                    && !fuzzy_match_default(&issue.category, &search_query)
                    && !fuzzy_match_default(&issue.details, &search_query)
                {
                    idx += 1;
                    continue;
                }

            show_issue(ui, issue, idx, state);
            idx += 1;
        }
    });
}

fn show_issue(ui: &mut Ui, issue: &ValidationIssue, idx: usize, state: &mut ValidationState) {
    let is_expanded = state.expanded_issues.contains(&idx);

    let severity_color = match issue.severity {
        Severity::Error => colors::CAT_RED,
        Severity::Warning => colors::STATUS_WARNING,
        Severity::Info => colors::STATUS_INFO,
    };

    ui.horizontal(|ui| {
        // Expand/collapse button
        let expand_icon = if is_expanded { nav_icons::EXPAND } else { nav_icons::COLLAPSE };
        if ui.small_button(expand_icon).clicked() {
            if is_expanded {
                state.expanded_issues.remove(&idx);
            } else {
                state.expanded_issues.insert(idx);
            }
        }

        // Severity icon
        ui.label(RichText::new(issue.severity.icon()).color(severity_color));

        // Category and slot
        ui.label(RichText::new(&issue.category).small().color(colors::TEXT_SECONDARY));
        ui.label(RichText::new(format!("[{}]", issue.slot)).small().color(colors::TEXT_DISABLED));

        // Message
        ui.label(RichText::new(&issue.message).strong());
    });

    if is_expanded {
        ui.horizontal(|ui| {
            ui.add_space(spacing::XL);
            ui.label(RichText::new(&issue.details).color(colors::TEXT_SECONDARY).small());
        });
    }

    ui.add_space(spacing::XS);
}

fn build_export(report: &ValidationReport) -> ValidationExport {
    let issues: Vec<IssueExport> = report.all_issues()
        .iter()
        .map(|issue| IssueExport {
            severity: issue.severity.display_name().to_string(),
            category: issue.category.clone(),
            slot: issue.slot,
            message: issue.message.clone(),
            details: issue.details.clone(),
        })
        .collect();

    ValidationExport {
        is_valid: report.is_valid,
        error_count: report.error_count(),
        warning_count: report.warning_count(),
        info_count: report.info_count(),
        issues,
    }
}
