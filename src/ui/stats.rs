pub mod stats {
    use eframe::egui::{Ui, RichText};
    use serde::Serialize;
    use er_reconstruct::ReconstructedCharacter;
    use crate::{
        db::classes::classes::class_display,
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

    pub fn stats(ui: &mut Ui, vm: &mut ViewModel, facts: Option<&ReconstructedCharacter>) {
        let stats_vm = &vm.slots[vm.index].stats_vm;

        // The stat values render from the reconstruction core's facts (ADR-0010)
        // when a save is loaded — attributes, blessings and runes from `stats`,
        // level and starting class from identity — mirroring the character-overview
        // panel (src/ui/general.rs). Attributes/blessings/runes are identical to the
        // ViewModel's (cross-checked in er-reconstruct's conformance corpus and
        // elden-map's native==WASM parity). Level now reads the stored level
        // (`f.level`), the same value the overview panel already shows, rather than
        // the table's former derived `sum(attrs) − 79`; the two agree for any legit
        // save (the level invariant) and the panels no longer disagree on tampered
        // ones. The ViewModel is the fallback for the empty/default state only; the
        // table's sort/export state stays in the ViewModel (UI state, not a fact).
        struct RenderStats {
            class: String,
            level: u32,
            vigor: u32, mind: u32, endurance: u32, strength: u32,
            dexterity: u32, intelligence: u32, faith: u32, arcane: u32,
            scadutree: u32, spirit_ash: u32, runes: u32,
        }
        let s = match facts {
            Some(f) => {
                let st = &f.stats;
                RenderStats {
                    class: class_display(f.class_id),
                    level: f.level,
                    vigor: st.vigor, mind: st.mind, endurance: st.endurance, strength: st.strength,
                    dexterity: st.dexterity, intelligence: st.intelligence, faith: st.faith, arcane: st.arcane,
                    scadutree: u32::from(st.scadutree_level), spirit_ash: u32::from(st.spirit_ash_level),
                    runes: st.runes,
                }
            }
            None => RenderStats {
                class: stats_vm.arche_type.to_string(),
                level: stats_vm.level,
                vigor: stats_vm.vigor, mind: stats_vm.mind, endurance: stats_vm.endurance, strength: stats_vm.strength,
                dexterity: stats_vm.dexterity, intelligence: stats_vm.intelligence, faith: stats_vm.faith, arcane: stats_vm.arcane,
                scadutree: stats_vm.scadutree, spirit_ash: stats_vm.spirit_ash, runes: stats_vm.souls,
            },
        };

        let mut stats_data: Vec<(&str, String)> = vec![
            ("Starting Class", s.class),
            ("Level", s.level.to_string()),
            ("Vigor", s.vigor.to_string()),
            ("Mind", s.mind.to_string()),
            ("Endurance", s.endurance.to_string()),
            ("Strength", s.strength.to_string()),
            ("Dexterity", s.dexterity.to_string()),
            ("Intelligence", s.intelligence.to_string()),
            ("Faith", s.faith.to_string()),
            ("Arcane", s.arcane.to_string()),
            ("Scadutree Blessing", s.scadutree.to_string()),
            ("Shadow Realm Blessing", s.spirit_ash.to_string()),
            ("Current Runes", s.runes.to_string()),
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
