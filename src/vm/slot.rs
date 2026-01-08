pub mod slot_view_model {
    use std::time::{SystemTime, UNIX_EPOCH};
    use crate::{
        db::{
            bosses::bosses::BOSSES,
            colosseums::colosseums::COLOSSEUMS,
            cookbooks::books::COOKBOKS,
            graces::maps::GRACES,
            maps::maps::MAPS,
            regions::regions::REGIONS,
            summoning_pools::summoning_pools::SUMMONING_POOLS,
            whetblades::whetblades::WHETBLADES,
            pickup_data::{WORLD_PICKUPS, PickupCategory},
            pickup_flags::is_flag_set,
        },
        save::common::save_slot::SaveSlot,
        vm::{
            equipment::equipment_view_model::EquipmentViewModel,
            events::events_view_model::EventsViewModel,
            export::{
                ExportData, ExportEquipment, ExportEquipmentItem, ExportEventItem,
                ExportEvents, ExportGeneral, ExportInventory, ExportInventoryItem,
                ExportMetadata, ExportRegionItem, ExportRegions, ExportStats,
                ExportWorldPickupItem,
            },
            general::general_view_model::{Gender, GeneralViewModel},
            inventory::InventoryViewModel,
            regions::regions_view_model::RegionsViewModel,
            stats::stats_view_model::StatsViewModel,
        },
    };

    #[derive(Default, Clone)]
    pub struct SlotViewModel {
        pub active: bool,
        pub general_vm: GeneralViewModel,
        pub stats_vm: StatsViewModel,
        pub equipment_vm: EquipmentViewModel,
        pub inventory_vm: InventoryViewModel,
        pub events_vm: EventsViewModel,
        pub regions_vm: RegionsViewModel,
    }

    impl SlotViewModel {
        pub fn from_save(slot: &SaveSlot) -> Self {
            let active = true;

            let general_vm = GeneralViewModel::from_save(slot);
            let stats_vm = StatsViewModel::from_save(slot);
            let equipment_vm = EquipmentViewModel::from_save(slot);
            let inventory_vm = InventoryViewModel::from_save(slot);
            let events_vm = EventsViewModel::from_save(slot);
            let regions_vm = RegionsViewModel::from_save(slot);

            Self {
                active,
                general_vm,
                stats_vm,
                equipment_vm,
                inventory_vm,
                events_vm,
                regions_vm,
            }
        }

        pub fn to_export_data(&self, slot_index: usize, steam_id: u64, event_flags: Option<&[u8]>) -> ExportData {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // Metadata
            let metadata = ExportMetadata {
                export_version: "1.0.0".to_string(),
                export_date: format!("unix_timestamp_{}", timestamp),
                slot_index,
                steam_id,
            };

            // General
            let gender_str = match self.general_vm.gender {
                Gender::Male => "Male",
                Gender::Female => "Female",
                Gender::Uknown => "Unknown",
            };
            let general = ExportGeneral {
                character_name: self.general_vm.character_name.trim_matches('\0').to_string(),
                gender: gender_str.to_string(),
                level: self.stats_vm.level,
                souls: self.stats_vm.souls,
                souls_memory: self.stats_vm.soulsmemory,
                arche_type: self.stats_vm.arche_type as u8,
                arche_type_name: self.stats_vm.arche_type.to_string(),
                gift: 0, // Gift is not stored in VMs, default to 0
                match_making_weapon_level: self.general_vm.weapon_level,
            };

            // Stats
            let stats = ExportStats {
                vigor: self.stats_vm.vigor,
                mind: self.stats_vm.mind,
                endurance: self.stats_vm.endurance,
                strength: self.stats_vm.strength,
                dexterity: self.stats_vm.dexterity,
                intelligence: self.stats_vm.intelligence,
                faith: self.stats_vm.faith,
                arcane: self.stats_vm.arcane,
                scadutree_level: self.stats_vm.scadutree,
                spirit_ash_level: self.stats_vm.spirit_ash,
            };

            // Equipment
            let equipment = self.build_equipment_export();

            // Inventory
            let inventory = self.build_inventory_export();

            // Events
            let events = self.build_events_export(event_flags);

            // Regions
            let regions = self.build_regions_export();

            ExportData {
                metadata,
                general,
                stats,
                equipment,
                inventory,
                events,
                regions,
            }
        }

        fn build_equipment_export(&self) -> ExportEquipment {
            let eq = &self.equipment_vm;

            let left_hand_armaments: Vec<ExportEquipmentItem> = eq
                .left_hand_armaments
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    if item.gaitem_handle == 0 || item.name == "Empty" {
                        ExportEquipmentItem::empty(&format!("left_hand_{}", i + 1))
                    } else {
                        ExportEquipmentItem::new(
                            &format!("left_hand_{}", i + 1),
                            item.gaitem_handle,
                            item.id,
                            &item.name,
                        )
                    }
                })
                .collect();

            let right_hand_armaments: Vec<ExportEquipmentItem> = eq
                .right_hand_armaments
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    if item.gaitem_handle == 0 || item.name == "Empty" {
                        ExportEquipmentItem::empty(&format!("right_hand_{}", i + 1))
                    } else {
                        ExportEquipmentItem::new(
                            &format!("right_hand_{}", i + 1),
                            item.gaitem_handle,
                            item.id,
                            &item.name,
                        )
                    }
                })
                .collect();

            let arrows: Vec<ExportEquipmentItem> = eq
                .arrows
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    if item.gaitem_handle == 0 || item.name == "Empty" {
                        ExportEquipmentItem::empty(&format!("arrow_{}", i + 1))
                    } else {
                        ExportEquipmentItem::new(
                            &format!("arrow_{}", i + 1),
                            item.gaitem_handle,
                            item.id,
                            &item.name,
                        )
                    }
                })
                .collect();

            let bolts: Vec<ExportEquipmentItem> = eq
                .bolts
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    if item.gaitem_handle == 0 || item.name == "Empty" {
                        ExportEquipmentItem::empty(&format!("bolt_{}", i + 1))
                    } else {
                        ExportEquipmentItem::new(
                            &format!("bolt_{}", i + 1),
                            item.gaitem_handle,
                            item.id,
                            &item.name,
                        )
                    }
                })
                .collect();

            let head = if eq.head.gaitem_handle == 0 || eq.head.name == "Empty" {
                ExportEquipmentItem::empty("head")
            } else {
                ExportEquipmentItem::new("head", eq.head.gaitem_handle, eq.head.id, &eq.head.name)
            };

            let chest = if eq.chest.gaitem_handle == 0 || eq.chest.name == "Empty" {
                ExportEquipmentItem::empty("chest")
            } else {
                ExportEquipmentItem::new("chest", eq.chest.gaitem_handle, eq.chest.id, &eq.chest.name)
            };

            let arms = if eq.arms.gaitem_handle == 0 || eq.arms.name == "Empty" {
                ExportEquipmentItem::empty("arms")
            } else {
                ExportEquipmentItem::new("arms", eq.arms.gaitem_handle, eq.arms.id, &eq.arms.name)
            };

            let legs = if eq.legs.gaitem_handle == 0 || eq.legs.name == "Empty" {
                ExportEquipmentItem::empty("legs")
            } else {
                ExportEquipmentItem::new("legs", eq.legs.gaitem_handle, eq.legs.id, &eq.legs.name)
            };

            let talismans: Vec<ExportEquipmentItem> = eq
                .talismans
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    if item.gaitem_handle == 0 || item.name == "Empty" {
                        ExportEquipmentItem::empty(&format!("talisman_{}", i + 1))
                    } else {
                        ExportEquipmentItem::new(
                            &format!("talisman_{}", i + 1),
                            item.gaitem_handle,
                            item.id,
                            &item.name,
                        )
                    }
                })
                .collect();

            let quick_slots: Vec<ExportEquipmentItem> = eq
                .quickitems
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    if item.gaitem_handle == 0 || item.name == "Empty" {
                        ExportEquipmentItem::empty(&format!("quick_slot_{}", i + 1))
                    } else {
                        ExportEquipmentItem::new(
                            &format!("quick_slot_{}", i + 1),
                            item.gaitem_handle,
                            item.id,
                            &item.name,
                        )
                    }
                })
                .collect();

            let pouch: Vec<ExportEquipmentItem> = eq
                .pouch
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    if item.gaitem_handle == 0 || item.name == "Empty" {
                        ExportEquipmentItem::empty(&format!("pouch_{}", i + 1))
                    } else {
                        ExportEquipmentItem::new(
                            &format!("pouch_{}", i + 1),
                            item.gaitem_handle,
                            item.id,
                            &item.name,
                        )
                    }
                })
                .collect();

            ExportEquipment {
                left_hand_armaments,
                right_hand_armaments,
                arrows,
                bolts,
                head,
                chest,
                arms,
                legs,
                talismans,
                quick_slots,
                pouch,
            }
        }

        fn build_inventory_export(&self) -> ExportInventory {
            let inv = &self.inventory_vm;
            let storage = &inv.storage[0]; // Main inventory (not storage box)

            let weapons: Vec<ExportInventoryItem> = storage
                .filtered_weapons
                .iter()
                .filter(|item| item.ga_item_handle != 0)
                .map(|item| {
                    ExportInventoryItem::new(
                        item.ga_item_handle,
                        item.item_id,
                        &item.item_name,
                        item.quantity,
                        item.inventory_index,
                        "WEAPON",
                    )
                })
                .collect();

            let armors: Vec<ExportInventoryItem> = storage
                .filtered_armors
                .iter()
                .filter(|item| item.ga_item_handle != 0)
                .map(|item| {
                    ExportInventoryItem::new(
                        item.ga_item_handle,
                        item.item_id,
                        &item.item_name,
                        item.quantity,
                        item.inventory_index,
                        "ARMOR",
                    )
                })
                .collect();

            let accessories: Vec<ExportInventoryItem> = storage
                .filtered_accessories
                .iter()
                .filter(|item| item.ga_item_handle != 0)
                .map(|item| {
                    ExportInventoryItem::new(
                        item.ga_item_handle,
                        item.item_id,
                        &item.item_name,
                        item.quantity,
                        item.inventory_index,
                        "ACCESSORY",
                    )
                })
                .collect();

            let items: Vec<ExportInventoryItem> = storage
                .filtered_items
                .iter()
                .filter(|item| item.ga_item_handle != 0)
                .map(|item| {
                    ExportInventoryItem::new(
                        item.ga_item_handle,
                        item.item_id,
                        &item.item_name,
                        item.quantity,
                        item.inventory_index,
                        "ITEM",
                    )
                })
                .collect();

            let ashes_of_war: Vec<ExportInventoryItem> = storage
                .filtered_aows
                .iter()
                .filter(|item| item.ga_item_handle != 0)
                .map(|item| {
                    ExportInventoryItem::new(
                        item.ga_item_handle,
                        item.item_id,
                        &item.item_name,
                        item.quantity,
                        item.inventory_index,
                        "AOW",
                    )
                })
                .collect();

            let key_items: Vec<ExportInventoryItem> = storage
                .key_items
                .iter()
                .filter(|item| item.ga_item_handle != 0 && item.quantity > 0)
                .map(|item| {
                    ExportInventoryItem::new(
                        item.ga_item_handle,
                        item.item_id,
                        &item.item_name,
                        item.quantity,
                        item.inventory_index,
                        "KEY_ITEM",
                    )
                })
                .collect();

            ExportInventory {
                common_item_count: storage.common_item_count,
                key_item_count: storage.key_item_count,
                weapons,
                armors,
                accessories,
                items,
                ashes_of_war,
                key_items,
            }
        }

        fn build_events_export(&self, event_flags: Option<&[u8]>) -> ExportEvents {
            let ev = &self.events_vm;

            let graces_lookup = GRACES.lock().unwrap();
            let graces: Vec<ExportEventItem> = ev
                .graces
                .iter()
                .map(|(grace, discovered)| {
                    let grace_info = graces_lookup.get(grace);
                    let name = grace_info.map(|g| g.2).unwrap_or("Unknown");
                    ExportEventItem::new(name, *discovered)
                })
                .collect();
            drop(graces_lookup);

            let bosses_lookup = BOSSES.lock().unwrap();
            let bosses: Vec<ExportEventItem> = ev
                .bosses
                .iter()
                .map(|(boss, discovered)| {
                    let boss_info = bosses_lookup.get(boss);
                    let name = boss_info.map(|b| b.1).unwrap_or("Unknown");
                    ExportEventItem::new(name, *discovered)
                })
                .collect();
            drop(bosses_lookup);

            let pools_lookup = SUMMONING_POOLS.lock().unwrap();
            let summoning_pools: Vec<ExportEventItem> = ev
                .summoning_pools
                .iter()
                .map(|(pool, discovered)| {
                    let pool_info = pools_lookup.get(pool);
                    let name = pool_info.map(|p| p.1).unwrap_or("Unknown");
                    ExportEventItem::new(name, *discovered)
                })
                .collect();
            drop(pools_lookup);

            let colosseums_lookup = COLOSSEUMS.lock().unwrap();
            let colosseums: Vec<ExportEventItem> = ev
                .colosseums
                .iter()
                .map(|(col, discovered)| {
                    let col_info = colosseums_lookup.get(col);
                    let name = col_info.map(|c| c.1).unwrap_or("Unknown");
                    ExportEventItem::new(name, *discovered)
                })
                .collect();
            drop(colosseums_lookup);

            let whetblades_lookup = WHETBLADES.lock().unwrap();
            let whetblades: Vec<ExportEventItem> = ev
                .whetblades
                .iter()
                .map(|(blade, discovered)| {
                    let blade_info = whetblades_lookup.get(blade);
                    let name = blade_info.map(|w| w.1).unwrap_or("Unknown");
                    ExportEventItem::new(name, *discovered)
                })
                .collect();
            drop(whetblades_lookup);

            let cookbooks_lookup = COOKBOKS.lock().unwrap();
            let cookbooks: Vec<ExportEventItem> = ev
                .cookbooks
                .iter()
                .map(|(book, discovered)| {
                    let book_info = cookbooks_lookup.get(book);
                    let name = book_info.map(|c| c.1).unwrap_or("Unknown");
                    ExportEventItem::new(name, *discovered)
                })
                .collect();
            drop(cookbooks_lookup);

            let maps_lookup = MAPS.lock().unwrap();
            let maps: Vec<ExportEventItem> = ev
                .maps
                .iter()
                .map(|(map, discovered)| {
                    let map_info = maps_lookup.get(map);
                    let name = map_info.map(|m| m.1).unwrap_or("Unknown");
                    ExportEventItem::new(name, *discovered)
                })
                .collect();
            drop(maps_lookup);

            // World Pickups
            let world_pickups: Vec<ExportWorldPickupItem> = if let Some(flags) = event_flags {
                WORLD_PICKUPS
                    .iter()
                    .map(|pickup| {
                        let collected = is_flag_set(flags, pickup.event_flag);

                        let type_str = match pickup.category {
                            PickupCategory::GoldenRunes => "GoldenRunes",
                            PickupCategory::SmithingStones => "SmithingStones",
                            PickupCategory::SomberStones => "SomberStones",
                            PickupCategory::Glovewort => "Glovewort",
                            PickupCategory::Weapons => "Weapons",
                            PickupCategory::Armor => "Armor",
                            PickupCategory::Talismans => "Talismans",
                            PickupCategory::AshesOfWar => "AshesOfWar",
                            PickupCategory::KeyItems => "KeyItems",
                            PickupCategory::CraftingMaterials => "CraftingMaterials",
                            PickupCategory::Consumables => "Consumables",
                            PickupCategory::Other => "Other",
                        };

                        ExportWorldPickupItem::new(
                            pickup.item_lot_id,
                            pickup.event_flag,
                            pickup.name,
                            type_str,
                            pickup.quantity,
                            pickup.region,
                            collected,
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            };

            ExportEvents {
                graces,
                bosses,
                summoning_pools,
                colosseums,
                whetblades,
                cookbooks,
                maps,
                world_pickups,
            }
        }

        fn build_regions_export(&self) -> ExportRegions {
            let reg = &self.regions_vm;

            let regions_lookup = REGIONS.lock().unwrap();
            let regions: Vec<ExportRegionItem> = reg
                .regions
                .iter()
                .map(|(region, (unlocked, is_open_world, is_dungeon, is_boss))| {
                    let region_info = regions_lookup.get(region);
                    let name = region_info.map(|r| r.1).unwrap_or("Unknown");
                    ExportRegionItem::new(
                        name,
                        *unlocked,
                        *is_open_world,
                        *is_dungeon,
                        *is_boss,
                    )
                })
                .collect();
            drop(regions_lookup);

            ExportRegions { regions }
        }
    }
}