pub mod stats {
    use eframe::egui::{Ui, RichText};
    use serde::Serialize;
    use crate::{
        ui::components::{
            table::{UnifiedTable, Column, RowData, SortDirection},
            export::{ExportToolbar, ExportFormat, PageExport, PageExportMetadata, to_json, to_csv, to_markdown},
        },
        ui::tokens::spacing,
        vm::vm::vm::ViewModel,
    };

    #[derive(Serialize)]
    struct StatExportItem {
        stat: String,
        value: String,
    }

    pub fn stats(ui: &mut Ui, vm: &mut ViewModel) {
        let stats_vm = &vm.slots[vm.index].stats_vm;

        // Build stats data
        let level = stats_vm.vigor + stats_vm.mind + stats_vm.endurance +
            stats_vm.strength + stats_vm.dexterity + stats_vm.intelligence +
            stats_vm.faith + stats_vm.arcane - 79;

        let mut stats_data: Vec<(&str, String)> = vec![
            ("Starting Class", stats_vm.arche_type.to_string()),
            ("Level", level.to_string()),
            ("Vigor", stats_vm.vigor.to_string()),
            ("Mind", stats_vm.mind.to_string()),
            ("Endurance", stats_vm.endurance.to_string()),
            ("Strength", stats_vm.strength.to_string()),
            ("Dexterity", stats_vm.dexterity.to_string()),
            ("Intelligence", stats_vm.intelligence.to_string()),
            ("Faith", stats_vm.faith.to_string()),
            ("Arcane", stats_vm.arcane.to_string()),
            ("Scadutree Blessing", stats_vm.scadutree.to_string()),
            ("Shadow Realm Blessing", stats_vm.spirit_ash.to_string()),
            ("Current Runes", stats_vm.souls.to_string()),
        ];

        // Export toolbar
        let mut dummy_filtered = false;
        let stats_vm = &mut vm.slots[vm.index].stats_vm;
        let export_response = ExportToolbar::new("stats_export", &mut stats_vm.export_format, &mut dummy_filtered)
            .no_filter_option()
            .show(ui);

        let export_format = stats_vm.export_format;

        spacing::space_sm(ui);

        // Apply sorting
        let stats_vm = &mut vm.slots[vm.index].stats_vm;
        if let Some(sort_col) = &stats_vm.table_state.sort_column {
            let asc = stats_vm.table_state.sort_direction == SortDirection::Ascending;
            match sort_col.as_str() {
                "stat" => stats_data.sort_by(|a, b| if asc { a.0.cmp(b.0) } else { b.0.cmp(a.0) }),
                "value" => stats_data.sort_by(|a, b| {
                    let va = a.1.parse::<u32>().ok();
                    let vb = b.1.parse::<u32>().ok();
                    match (va, vb) {
                        (Some(a_num), Some(b_num)) => if asc { a_num.cmp(&b_num) } else { b_num.cmp(&a_num) },
                        _ => if asc { a.1.cmp(&b.1) } else { b.1.cmp(&a.1) },
                    }
                }),
                _ => {}
            }
        }

        // Summary
        let count = stats_data.len();
        ui.label(RichText::new(format!("Character Stats: {} stats", count)).strong());

        spacing::space_sm(ui);

        // Build row data
        let rows: Vec<RowData> = stats_data.iter().map(|(stat, value)| {
            RowData::new(vec![
                stat.to_string(),
                value.clone(),
            ])
        }).collect();

        // Show table
        let table_response = UnifiedTable::new("stats_table", &mut vm.slots[vm.index].stats_vm.table_state)
            .columns(vec![
                Column::new("stat", "Stat").width_fraction(0.4).sortable(true),
                Column::new("value", "Value").width_fraction(0.4).sortable(true),
            ])
            .rows(rows)
            .zebra_stripe(true)
            .selectable(true)
            .show(ui);

        // Handle clipboard copy
        if let Some(text) = table_response.clipboard_text {
            ui.output_mut(|o| o.copied_text = text);
        }

        // Handle double-click copy
        if let Some(row_idx) = table_response.double_clicked_row {
            if let Some((stat, value)) = stats_data.get(row_idx) {
                let row_text = format!("{}\t{}", stat, value);
                ui.output_mut(|o| o.copied_text = row_text);
            }
        }

        // Handle export
        if export_response.export_clicked || export_response.copy_clicked {
            let data_to_export: Vec<_> = stats_data.iter()
                .map(|(stat, value)| StatExportItem {
                    stat: stat.to_string(),
                    value: value.clone(),
                })
                .collect();

            let content = match export_format {
                ExportFormat::Json => {
                    let export = PageExport::new(
                        PageExportMetadata::new("Character Stats")
                            .with_counts(count, count),
                        &data_to_export,
                    );
                    to_json(&export).unwrap_or_else(|_| String::new())
                }
                ExportFormat::Csv => {
                    let headers = &["Stat", "Value"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|s| vec![s.stat.clone(), s.value.clone()])
                        .collect();
                    to_csv(headers, &rows)
                }
                ExportFormat::Markdown => {
                    let headers = &["Stat", "Value"];
                    let rows: Vec<Vec<String>> = data_to_export.iter()
                        .map(|s| vec![s.stat.clone(), s.value.clone()])
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
