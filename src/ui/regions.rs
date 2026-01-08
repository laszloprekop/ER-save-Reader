pub mod regions {
    use std::collections::BTreeMap;

    use eframe::egui::{self, Ui};

    use crate::{db::{map_name::map_name::MAP_NAME, regions::regions::REGIONS}, ui::custom::checkbox::checkbox::{three_states_checkbox, State}, vm::vm::vm::ViewModel};

    pub fn regions(ui: &mut Ui, vm:&mut ViewModel) { 
        egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            let maps = &vm.slots[vm.index].regions_vm.region_groups;
            let regions = &mut vm.slots[vm.index].regions_vm.regions;
            ui.horizontal(|ui| {
                display_all_checkbox(ui, regions, "All Regions");
                ui.separator();
                display_open_world_checkbox(ui, regions, "Open World");
                ui.separator();
                display_dungeon_checkbox(ui, regions, "Dungeons");
                ui.separator();
                display_bosses_checkbox(ui, regions, "Bosses");
            });
            ui.separator();
            
            for map in maps {
                ui.push_id(map.0, |ui| {
                    let collapsing = egui::containers::collapsing_header::CollapsingHeader::new(MAP_NAME.lock().unwrap()[&map.0]);
                    ui.horizontal(|ui|{
                        let mut state = State::Off;
                        if map.1.iter().all(|g| regions[&g].0) {
                            state = State::On;
                        }
                        else if map.1.iter().any(|g| regions[&g].0) {
                            state = State::InBetween;
                        }

                        three_states_checkbox(ui, &state);

                        collapsing.show(ui, |ui| {
                            for region in map.1 {
                                let region_info = REGIONS.lock().unwrap()[&region];
                                let on = &mut regions.get_mut(region).expect("").0;
                                ui.add_enabled(false, egui::Checkbox::new(on, region_info.1.to_string()));
                            }
                        });
                    })
                });
            }
        });
    }


    fn display_all_checkbox<T>(ui: &mut Ui, map: &mut BTreeMap<T, (bool, bool, bool, bool)>, label: &str) {
        let mut state = State::Off;
        if map.values().all(|(on,_,_,_)| *on) {
            state = State::On;
        }
        else if map.values().any(|(on,_,_,_)| *on) {
            state = State::InBetween;
        }

        ui.horizontal(|ui| {
            three_states_checkbox(ui, &state);
            ui.label(label);
        });
    }


    fn display_open_world_checkbox<T>(ui: &mut Ui, map: &mut BTreeMap<T, (bool, bool, bool, bool)>, label: &str) {
        let mut state = State::Off;
        if map.values().filter(|(_, is_open_world,_,_)|*is_open_world).all(|(on,_,_,_)| *on) {
            state = State::On;
        }
        else if map.values().filter(|(_, is_open_world,_,_)|*is_open_world).any(|(on,_,_,_)| *on) {
            state = State::InBetween;
        }

        ui.horizontal(|ui| {
            three_states_checkbox(ui, &state);
            ui.label(label);
        });
    }

    fn display_dungeon_checkbox<T>(ui: &mut Ui, map: &mut BTreeMap<T, (bool, bool, bool, bool)>, label: &str) {
        let mut state = State::Off;
        if map.values().filter(|(_,_, is_dungeon,_)| *is_dungeon).all(|(on,_,_,_)| *on) {
            state = State::On;
        }
        else if map.values().filter(|(_,_, is_dungeon,_)| *is_dungeon).any(|(on,_,_,_)| *on) {
            state = State::InBetween;
        }

        ui.horizontal(|ui| {
            three_states_checkbox(ui, &state);
            ui.label(label);
        });
    }

    fn display_bosses_checkbox<T>(ui: &mut Ui, map: &mut BTreeMap<T, (bool, bool, bool, bool)>, label: &str) {
        let mut state = State::Off;
        if map.values().filter(|(_,_,_, is_boss)| *is_boss).all(|(on,_,_,_)| *on) {
            state = State::On;
        }
        else if map.values().filter(|(_,_,_, is_boss)| *is_boss).any(|(on,_,_,_)| *on) {
            state = State::InBetween;
        }

        ui.horizontal(|ui| {
            three_states_checkbox(ui, &state);
            ui.label(label);
        });
    }
}