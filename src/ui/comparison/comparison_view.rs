//! Character comparison view for analyzing differences between character slots.

use eframe::egui::{self, ComboBox, RichText, ScrollArea, Ui};
use serde::Serialize;

use crate::vm::slot::slot_view_model::SlotViewModel;
use crate::db::pickup_flags::is_flag_set;
use crate::ui::components::filter::fuzzy_match_default;
use crate::ui::components::export::{ExportToolbar, ExportFormat, to_json};
use crate::ui::tokens::{colors, spacing};
use crate::ui::components::legend::{entity_icons, nav_icons};

use super::{ComparisonState, ComparisonTab};

/// Stat difference for display.
struct StatDiff {
    name: &'static str,
    value_a: u32,
    value_b: u32,
}

impl StatDiff {
    fn is_different(&self) -> bool {
        self.value_a != self.value_b
    }

    fn diff(&self) -> i64 {
        self.value_b as i64 - self.value_a as i64
    }
}

/// Event flag difference for display.
struct FlagDiff {
    flag_id: u32,
    name: String,
    category: String,
    in_a: bool,
    in_b: bool,
}

impl FlagDiff {
    fn is_different(&self) -> bool {
        self.in_a != self.in_b
    }
}

/// Inventory item difference for display.
struct ItemDiff {
    name: String,
    category: String,
    qty_a: u32,
    qty_b: u32,
}

impl ItemDiff {
    fn is_different(&self) -> bool {
        self.qty_a != self.qty_b
    }

    fn diff(&self) -> i64 {
        self.qty_b as i64 - self.qty_a as i64
    }
}

// Export structures
#[derive(Serialize)]
struct ComparisonExport {
    slot_a: SlotInfo,
    slot_b: SlotInfo,
    stats: Vec<StatExport>,
    event_flags: FlagDiffExport,
    inventory: Vec<InventoryExport>,
}

#[derive(Serialize)]
struct SlotInfo {
    index: usize,
    name: String,
    level: u32,
}

#[derive(Serialize)]
struct StatExport {
    name: String,
    slot_a_value: u32,
    slot_b_value: u32,
    difference: i64,
}

#[derive(Serialize)]
struct FlagDiffExport {
    only_in_a: Vec<FlagExport>,
    only_in_b: Vec<FlagExport>,
    common_count: usize,
}

#[derive(Serialize)]
struct FlagExport {
    flag_id: u32,
    name: String,
    category: String,
}

#[derive(Serialize)]
struct InventoryExport {
    name: String,
    category: String,
    slot_a_qty: u32,
    slot_b_qty: u32,
    difference: i64,
}

/// Render the comparison view.
pub fn comparison_view(
    ui: &mut Ui,
    state: &mut ComparisonState,
    slots: &[SlotViewModel],
    event_flags_a: Option<&[u8]>,
    event_flags_b: Option<&[u8]>,
) {
    // Title
    ui.horizontal(|ui| {
        ui.label(RichText::new(entity_icons::REGION).size(18.0));
        ui.add_space(spacing::XS);
        ui.label(RichText::new("Character Comparison").heading());
    });

    ui.add_space(spacing::SM);

    // Slot selection row
    ui.horizontal(|ui| {
        // Slot A selector
        ui.label("Slot A:");
        ComboBox::from_id_salt("slot_a_select")
            .selected_text(slot_label(state.slot_a, slots))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.slot_a, None, "Select...");
                for (i, slot) in slots.iter().enumerate() {
                    if slot.active {
                        let name = slot.general_vm.character_name.trim_matches('\0');
                        ui.selectable_value(&mut state.slot_a, Some(i), format!("{}: {}", i, name));
                    }
                }
            });

        ui.add_space(spacing::LG);

        // Slot B selector
        ui.label("Slot B:");
        ComboBox::from_id_salt("slot_b_select")
            .selected_text(slot_label(state.slot_b, slots))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.slot_b, None, "Select...");
                for (i, slot) in slots.iter().enumerate() {
                    if slot.active {
                        let name = slot.general_vm.character_name.trim_matches('\0');
                        ui.selectable_value(&mut state.slot_b, Some(i), format!("{}: {}", i, name));
                    }
                }
            });

        ui.add_space(spacing::LG);

        // Reset button
        if ui.button("Reset").clicked() {
            state.reset();
        }
    });

    ui.add_space(spacing::SM);

    // Show warning if same slot selected
    if let (Some(a), Some(b)) = (state.slot_a, state.slot_b) {
        if a == b {
            ui.colored_label(colors::STATUS_WARNING, "Please select two different slots to compare.");
            return;
        }
    }

    // Show prompt if slots not selected
    if !state.can_compare() {
        ui.colored_label(colors::TEXT_SECONDARY, "Select two different character slots to compare.");
        return;
    }

    let slot_a_idx = state.slot_a.unwrap();
    let slot_b_idx = state.slot_b.unwrap();

    // Tab selection
    ui.horizontal(|ui| {
        for tab in [ComparisonTab::Stats, ComparisonTab::EventFlags, ComparisonTab::Inventory] {
            let selected = state.active_tab == tab;
            if ui.selectable_label(selected, tab.display_name()).clicked() {
                state.active_tab = tab;
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Differences only toggle
            ui.checkbox(&mut state.show_differences_only, "Show differences only");
        });
    });

    ui.separator();

    // Search bar for event flags and inventory tabs
    if matches!(state.active_tab, ComparisonTab::EventFlags | ComparisonTab::Inventory) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(nav_icons::SEARCH).color(colors::TEXT_SECONDARY));
            ui.text_edit_singleline(&mut state.search_query);
            if !state.search_query.is_empty() && ui.small_button(nav_icons::CLOSE).clicked() {
                state.search_query.clear();
            }
        });
        ui.add_space(spacing::XS);
    }

    // Export toolbar
    let export_clicked = ExportToolbar::new("comparison_export", &mut state.export_format, &mut false)
        .show(ui);

    ui.add_space(spacing::SM);

    // Tab content
    ScrollArea::vertical().show(ui, |ui| {
        match state.active_tab {
            ComparisonTab::Stats => {
                show_stats_comparison(ui, &slots[slot_a_idx], &slots[slot_b_idx], state.show_differences_only);
            }
            ComparisonTab::EventFlags => {
                show_event_flags_comparison(
                    ui,
                    &slots[slot_a_idx],
                    &slots[slot_b_idx],
                    event_flags_a,
                    event_flags_b,
                    state.show_differences_only,
                    &state.search_query,
                );
            }
            ComparisonTab::Inventory => {
                show_inventory_comparison(
                    ui,
                    &slots[slot_a_idx],
                    &slots[slot_b_idx],
                    state.show_differences_only,
                    &state.search_query,
                );
            }
        }
    });

    // Handle export
    if export_clicked.export_clicked || export_clicked.copy_clicked {
        let export_data = build_comparison_export(
            slot_a_idx,
            slot_b_idx,
            &slots[slot_a_idx],
            &slots[slot_b_idx],
            event_flags_a,
            event_flags_b,
        );

        let content = match state.export_format {
            ExportFormat::Json => to_json(&export_data).unwrap_or_else(|e| format!("Error: {}", e)),
            ExportFormat::Csv => comparison_to_csv(&export_data),
            ExportFormat::Markdown => comparison_to_markdown(&export_data),
        };

        // Copy to clipboard
        ui.output_mut(|o| o.copied_text = content);
    }
}

fn slot_label(slot: Option<usize>, slots: &[SlotViewModel]) -> String {
    match slot {
        Some(i) => {
            let name = &slots[i].general_vm.character_name;
            format!("{}: {}", i, name.trim_matches('\0'))
        }
        None => "Select...".to_string(),
    }
}

fn show_stats_comparison(ui: &mut Ui, slot_a: &SlotViewModel, slot_b: &SlotViewModel, differences_only: bool) {
    let stats_a = &slot_a.stats_vm;
    let stats_b = &slot_b.stats_vm;

    let stat_diffs: Vec<StatDiff> = vec![
        StatDiff { name: "Level", value_a: stats_a.level, value_b: stats_b.level },
        StatDiff { name: "Vigor", value_a: stats_a.vigor, value_b: stats_b.vigor },
        StatDiff { name: "Mind", value_a: stats_a.mind, value_b: stats_b.mind },
        StatDiff { name: "Endurance", value_a: stats_a.endurance, value_b: stats_b.endurance },
        StatDiff { name: "Strength", value_a: stats_a.strength, value_b: stats_b.strength },
        StatDiff { name: "Dexterity", value_a: stats_a.dexterity, value_b: stats_b.dexterity },
        StatDiff { name: "Intelligence", value_a: stats_a.intelligence, value_b: stats_b.intelligence },
        StatDiff { name: "Faith", value_a: stats_a.faith, value_b: stats_b.faith },
        StatDiff { name: "Arcane", value_a: stats_a.arcane, value_b: stats_b.arcane },
        StatDiff { name: "Souls", value_a: stats_a.souls, value_b: stats_b.souls },
        StatDiff { name: "Souls Memory", value_a: stats_a.soulsmemory, value_b: stats_b.soulsmemory },
        StatDiff { name: "Scadutree Level", value_a: stats_a.scadutree, value_b: stats_b.scadutree },
        StatDiff { name: "Spirit Ash Level", value_a: stats_a.spirit_ash, value_b: stats_b.spirit_ash },
        StatDiff { name: "Max HP", value_a: stats_a.max_hp, value_b: stats_b.max_hp },
        StatDiff { name: "Max FP", value_a: stats_a.max_fp, value_b: stats_b.max_fp },
        StatDiff { name: "Max Stamina", value_a: stats_a.max_stamina, value_b: stats_b.max_stamina },
    ];

    // Header row
    ui.horizontal(|ui| {
        ui.add_space(150.0);
        ui.label(RichText::new("Slot A").strong().color(colors::ACCENT_PRIMARY));
        ui.add_space(50.0);
        ui.label(RichText::new("Slot B").strong().color(colors::ACCENT_PRIMARY));
        ui.add_space(50.0);
        ui.label(RichText::new("Diff").strong());
    });

    ui.separator();

    for diff in stat_diffs {
        if differences_only && !diff.is_different() {
            continue;
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new(diff.name).strong().monospace());
            ui.add_space(150.0 - ui.min_rect().width());

            ui.label(RichText::new(format!("{}", diff.value_a)).monospace());
            ui.add_space(60.0);

            ui.label(RichText::new(format!("{}", diff.value_b)).monospace());
            ui.add_space(60.0);

            if diff.is_different() {
                let diff_val = diff.diff();
                let color = if diff_val > 0 { colors::STATUS_COLLECTED } else { colors::CAT_RED };
                let sign = if diff_val > 0 { "+" } else { "" };
                ui.label(RichText::new(format!("{}{}", sign, diff_val)).color(color).monospace());
            } else {
                ui.label(RichText::new("=").color(colors::TEXT_DISABLED).monospace());
            }
        });
    }
}

fn show_event_flags_comparison(
    ui: &mut Ui,
    slot_a: &SlotViewModel,
    slot_b: &SlotViewModel,
    event_flags_a: Option<&[u8]>,
    event_flags_b: Option<&[u8]>,
    differences_only: bool,
    search_query: &str,
) {
    // Compare graces
    ui.label(RichText::new("Sites of Grace").heading().size(14.0));
    ui.add_space(spacing::XS);

    let mut grace_diffs: Vec<FlagDiff> = Vec::new();
    let graces_lookup = crate::db::graces::maps::GRACES.lock().unwrap();

    for (grace, status_a) in &slot_a.events_vm.graces {
        let status_b = slot_b.events_vm.graces.get(grace);
        let in_a = matches!(status_a, crate::vm::events::events_view_model::GraceStatus::Discovered);
        let in_b = status_b.map(|s| matches!(s, crate::vm::events::events_view_model::GraceStatus::Discovered)).unwrap_or(false);

        let name = graces_lookup.get(grace)
            .map(|g| g.2.to_string())
            .unwrap_or_else(|| "Unknown Grace".to_string());

        grace_diffs.push(FlagDiff {
            flag_id: 0, // Grace enum doesn't have direct flag_id access
            name,
            category: "Grace".to_string(),
            in_a,
            in_b,
        });
    }
    drop(graces_lookup);

    show_flag_diff_table(ui, &grace_diffs, differences_only, search_query);

    ui.add_space(spacing::MD);

    // Compare bosses
    ui.label(RichText::new("Bosses Defeated").heading().size(14.0));
    ui.add_space(spacing::XS);

    let mut boss_diffs: Vec<FlagDiff> = Vec::new();
    let bosses_lookup = crate::db::bosses::bosses::BOSSES.lock().unwrap();

    for (boss, defeated_a) in &slot_a.events_vm.bosses {
        let defeated_b = slot_b.events_vm.bosses.get(boss).copied().unwrap_or(false);

        let name = bosses_lookup.get(boss)
            .map(|b| b.1.to_string())
            .unwrap_or_else(|| "Unknown Boss".to_string());

        boss_diffs.push(FlagDiff {
            flag_id: 0, // Boss enum doesn't have direct flag_id access
            name,
            category: "Boss".to_string(),
            in_a: *defeated_a,
            in_b: defeated_b,
        });
    }
    drop(bosses_lookup);

    show_flag_diff_table(ui, &boss_diffs, differences_only, search_query);

    ui.add_space(spacing::MD);

    // Compare world pickups if event flags available
    if let (Some(flags_a), Some(flags_b)) = (event_flags_a, event_flags_b) {
        ui.label(RichText::new("World Pickups (Sample)").heading().size(14.0));
        ui.add_space(spacing::XS);

        let mut pickup_diffs: Vec<FlagDiff> = Vec::new();

        // Sample first 100 pickups to avoid performance issues
        for pickup in crate::db::pickup_data::WORLD_PICKUPS.iter().take(100) {
            let in_a = is_flag_set(flags_a, pickup.event_flag);
            let in_b = is_flag_set(flags_b, pickup.event_flag);

            if in_a != in_b || !differences_only {
                pickup_diffs.push(FlagDiff {
                    flag_id: pickup.event_flag,
                    name: pickup.name.to_string(),
                    category: "Pickup".to_string(),
                    in_a,
                    in_b,
                });
            }
        }

        show_flag_diff_table(ui, &pickup_diffs, differences_only, search_query);
    }
}

fn show_flag_diff_table(ui: &mut Ui, diffs: &[FlagDiff], differences_only: bool, search_query: &str) {
    let filtered: Vec<&FlagDiff> = diffs.iter()
        .filter(|d| {
            if differences_only && !d.is_different() {
                return false;
            }
            if !search_query.is_empty() && !fuzzy_match_default(&d.name, search_query) {
                return false;
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        ui.colored_label(colors::TEXT_SECONDARY, "No matching entries.");
        return;
    }

    // Summary
    let only_a = filtered.iter().filter(|d| d.in_a && !d.in_b).count();
    let only_b = filtered.iter().filter(|d| !d.in_a && d.in_b).count();
    let both = filtered.iter().filter(|d| d.in_a && d.in_b).count();

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Only A: {}", only_a)).color(colors::ACCENT_PRIMARY).small());
        ui.label("|");
        ui.label(RichText::new(format!("Only B: {}", only_b)).color(colors::STATUS_WARNING).small());
        ui.label("|");
        ui.label(RichText::new(format!("Both: {}", both)).color(colors::STATUS_COLLECTED).small());
    });

    ui.add_space(spacing::XS);

    // Table header
    ui.horizontal(|ui| {
        ui.label(RichText::new("Name").strong());
        ui.add_space(200.0);
        ui.label(RichText::new("A").strong());
        ui.add_space(20.0);
        ui.label(RichText::new("B").strong());
    });

    ui.separator();

    // Show first 50 entries to avoid performance issues
    for diff in filtered.iter().take(50) {
        ui.horizontal(|ui| {
            let name_color = if diff.is_different() { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY };
            ui.label(RichText::new(&diff.name).color(name_color));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Slot B status
                let b_icon = if diff.in_b { egui_phosphor::regular::CHECK } else { egui_phosphor::regular::X };
                let b_color = if diff.in_b { colors::STATUS_COLLECTED } else { colors::TEXT_DISABLED };
                ui.label(RichText::new(b_icon).color(b_color));

                ui.add_space(20.0);

                // Slot A status
                let a_icon = if diff.in_a { egui_phosphor::regular::CHECK } else { egui_phosphor::regular::X };
                let a_color = if diff.in_a { colors::STATUS_COLLECTED } else { colors::TEXT_DISABLED };
                ui.label(RichText::new(a_icon).color(a_color));
            });
        });
    }

    if filtered.len() > 50 {
        ui.colored_label(colors::TEXT_SECONDARY, format!("... and {} more entries", filtered.len() - 50));
    }
}

fn show_inventory_comparison(
    ui: &mut Ui,
    slot_a: &SlotViewModel,
    slot_b: &SlotViewModel,
    differences_only: bool,
    search_query: &str,
) {
    let inv_a = &slot_a.inventory_vm;
    let inv_b = &slot_b.inventory_vm;

    // Build combined inventory map
    let mut item_diffs: Vec<ItemDiff> = Vec::new();

    // Compare weapons
    let storage_a = &inv_a.storage[0];
    let storage_b = &inv_b.storage[0];

    for item in &storage_a.filtered_weapons {
        if item.ga_item_handle == 0 {
            continue;
        }
        let qty_b = storage_b.filtered_weapons.iter()
            .find(|i| i.item_id == item.item_id)
            .map(|i| i.quantity)
            .unwrap_or(0);
        item_diffs.push(ItemDiff {
            name: item.item_name.clone(),
            category: "Weapon".to_string(),
            qty_a: item.quantity,
            qty_b,
        });
    }

    // Add weapons only in B
    for item in &storage_b.filtered_weapons {
        if item.ga_item_handle == 0 {
            continue;
        }
        if !storage_a.filtered_weapons.iter().any(|i| i.item_id == item.item_id) {
            item_diffs.push(ItemDiff {
                name: item.item_name.clone(),
                category: "Weapon".to_string(),
                qty_a: 0,
                qty_b: item.quantity,
            });
        }
    }

    // Compare armors (simplified - just check presence)
    for item in &storage_a.filtered_armors {
        if item.ga_item_handle == 0 {
            continue;
        }
        let qty_b = storage_b.filtered_armors.iter()
            .find(|i| i.item_id == item.item_id)
            .map(|i| i.quantity)
            .unwrap_or(0);
        item_diffs.push(ItemDiff {
            name: item.item_name.clone(),
            category: "Armor".to_string(),
            qty_a: item.quantity,
            qty_b,
        });
    }

    // Filter and display
    let filtered: Vec<&ItemDiff> = item_diffs.iter()
        .filter(|d| {
            if differences_only && !d.is_different() {
                return false;
            }
            if !search_query.is_empty() && !fuzzy_match_default(&d.name, search_query) {
                return false;
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        ui.colored_label(colors::TEXT_SECONDARY, "No matching inventory differences.");
        return;
    }

    // Summary
    let only_a = filtered.iter().filter(|d| d.qty_a > 0 && d.qty_b == 0).count();
    let only_b = filtered.iter().filter(|d| d.qty_a == 0 && d.qty_b > 0).count();
    let both_diff = filtered.iter().filter(|d| d.qty_a > 0 && d.qty_b > 0 && d.is_different()).count();

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Only A: {}", only_a)).color(colors::ACCENT_PRIMARY).small());
        ui.label("|");
        ui.label(RichText::new(format!("Only B: {}", only_b)).color(colors::STATUS_WARNING).small());
        ui.label("|");
        ui.label(RichText::new(format!("Qty Diff: {}", both_diff)).color(colors::STATUS_INFO).small());
    });

    ui.add_space(spacing::XS);

    // Table header
    ui.horizontal(|ui| {
        ui.label(RichText::new("Item").strong());
        ui.add_space(200.0);
        ui.label(RichText::new("A").strong());
        ui.add_space(20.0);
        ui.label(RichText::new("B").strong());
        ui.add_space(20.0);
        ui.label(RichText::new("Diff").strong());
    });

    ui.separator();

    for diff in filtered.iter().take(50) {
        ui.horizontal(|ui| {
            let name_color = if diff.is_different() { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY };
            ui.label(RichText::new(&diff.name).color(name_color));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Diff
                if diff.is_different() {
                    let diff_val = diff.diff();
                    let color = if diff_val > 0 { colors::STATUS_COLLECTED } else { colors::CAT_RED };
                    let sign = if diff_val > 0 { "+" } else { "" };
                    ui.label(RichText::new(format!("{}{}", sign, diff_val)).color(color).monospace());
                } else {
                    ui.label(RichText::new("=").color(colors::TEXT_DISABLED).monospace());
                }

                ui.add_space(20.0);

                // Slot B qty
                ui.label(RichText::new(format!("{}", diff.qty_b)).monospace());

                ui.add_space(20.0);

                // Slot A qty
                ui.label(RichText::new(format!("{}", diff.qty_a)).monospace());
            });
        });
    }

    if filtered.len() > 50 {
        ui.colored_label(colors::TEXT_SECONDARY, format!("... and {} more items", filtered.len() - 50));
    }
}

fn build_comparison_export(
    slot_a_idx: usize,
    slot_b_idx: usize,
    slot_a: &SlotViewModel,
    slot_b: &SlotViewModel,
    _event_flags_a: Option<&[u8]>,
    _event_flags_b: Option<&[u8]>,
) -> ComparisonExport {
    let stats_a = &slot_a.stats_vm;
    let stats_b = &slot_b.stats_vm;

    // Slot info
    let slot_info_a = SlotInfo {
        index: slot_a_idx,
        name: slot_a.general_vm.character_name.trim_matches('\0').to_string(),
        level: stats_a.level,
    };
    let slot_info_b = SlotInfo {
        index: slot_b_idx,
        name: slot_b.general_vm.character_name.trim_matches('\0').to_string(),
        level: stats_b.level,
    };

    // Stats
    let stats = vec![
        StatExport { name: "Level".to_string(), slot_a_value: stats_a.level, slot_b_value: stats_b.level, difference: stats_b.level as i64 - stats_a.level as i64 },
        StatExport { name: "Vigor".to_string(), slot_a_value: stats_a.vigor, slot_b_value: stats_b.vigor, difference: stats_b.vigor as i64 - stats_a.vigor as i64 },
        StatExport { name: "Mind".to_string(), slot_a_value: stats_a.mind, slot_b_value: stats_b.mind, difference: stats_b.mind as i64 - stats_a.mind as i64 },
        StatExport { name: "Endurance".to_string(), slot_a_value: stats_a.endurance, slot_b_value: stats_b.endurance, difference: stats_b.endurance as i64 - stats_a.endurance as i64 },
        StatExport { name: "Strength".to_string(), slot_a_value: stats_a.strength, slot_b_value: stats_b.strength, difference: stats_b.strength as i64 - stats_a.strength as i64 },
        StatExport { name: "Dexterity".to_string(), slot_a_value: stats_a.dexterity, slot_b_value: stats_b.dexterity, difference: stats_b.dexterity as i64 - stats_a.dexterity as i64 },
        StatExport { name: "Intelligence".to_string(), slot_a_value: stats_a.intelligence, slot_b_value: stats_b.intelligence, difference: stats_b.intelligence as i64 - stats_a.intelligence as i64 },
        StatExport { name: "Faith".to_string(), slot_a_value: stats_a.faith, slot_b_value: stats_b.faith, difference: stats_b.faith as i64 - stats_a.faith as i64 },
        StatExport { name: "Arcane".to_string(), slot_a_value: stats_a.arcane, slot_b_value: stats_b.arcane, difference: stats_b.arcane as i64 - stats_a.arcane as i64 },
    ];

    // Event flags - simplified export
    let mut only_a = Vec::new();
    let mut only_b = Vec::new();
    let mut common_count = 0;

    let graces_lookup = crate::db::graces::maps::GRACES.lock().unwrap();
    for (grace, status_a) in &slot_a.events_vm.graces {
        let status_b = slot_b.events_vm.graces.get(grace);
        let in_a = matches!(status_a, crate::vm::events::events_view_model::GraceStatus::Discovered);
        let in_b = status_b.map(|s| matches!(s, crate::vm::events::events_view_model::GraceStatus::Discovered)).unwrap_or(false);

        let name = graces_lookup.get(grace)
            .map(|g| g.2.to_string())
            .unwrap_or_else(|| "Unknown Grace".to_string());

        if in_a && !in_b {
            only_a.push(FlagExport { flag_id: 0, name, category: "Grace".to_string() });
        } else if !in_a && in_b {
            only_b.push(FlagExport { flag_id: 0, name, category: "Grace".to_string() });
        } else if in_a && in_b {
            common_count += 1;
        }
    }
    drop(graces_lookup);

    let event_flags = FlagDiffExport { only_in_a: only_a, only_in_b: only_b, common_count };

    // Inventory - simplified
    let inventory = Vec::new(); // Full inventory export would be verbose

    ComparisonExport {
        slot_a: slot_info_a,
        slot_b: slot_info_b,
        stats,
        event_flags,
        inventory,
    }
}

fn comparison_to_csv(data: &ComparisonExport) -> String {
    let mut csv = String::new();
    csv.push_str("Stat,Slot A,Slot B,Difference\n");
    for stat in &data.stats {
        csv.push_str(&format!("{},{},{},{}\n", stat.name, stat.slot_a_value, stat.slot_b_value, stat.difference));
    }
    csv
}

fn comparison_to_markdown(data: &ComparisonExport) -> String {
    let mut md = String::new();
    md.push_str(&format!("# Character Comparison\n\n"));
    md.push_str(&format!("**Slot A:** {} (Level {})\n", data.slot_a.name, data.slot_a.level));
    md.push_str(&format!("**Slot B:** {} (Level {})\n\n", data.slot_b.name, data.slot_b.level));

    md.push_str("## Stats\n\n");
    md.push_str("| Stat | Slot A | Slot B | Diff |\n");
    md.push_str("|------|--------|--------|------|\n");
    for stat in &data.stats {
        let diff_str = if stat.difference > 0 {
            format!("+{}", stat.difference)
        } else if stat.difference < 0 {
            format!("{}", stat.difference)
        } else {
            "=".to_string()
        };
        md.push_str(&format!("| {} | {} | {} | {} |\n", stat.name, stat.slot_a_value, stat.slot_b_value, diff_str));
    }

    md.push_str("\n## Event Flags\n\n");
    md.push_str(&format!("- Only in A: {}\n", data.event_flags.only_in_a.len()));
    md.push_str(&format!("- Only in B: {}\n", data.event_flags.only_in_b.len()));
    md.push_str(&format!("- Common: {}\n", data.event_flags.common_count));

    md
}
