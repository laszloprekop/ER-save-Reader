use std::{cmp::Ordering, collections::HashMap};

use crate::{
    db::{
        accessory_name::accessory_name::ACCESSORY_NAME, aow_name::aow_name::AOW_NAME,
        armor_name::armor_name::ARMOR_NAME, item_name::item_name::ITEM_NAME,
        weapon_name::weapon_name::WEAPON_NAME,
    },
    save::common::save_slot::{
        EquipInventoryData, EquipInventoryItem, EquipProjectileData, GaItem, GaItemData, SaveSlot,
    },
    ui::components::{
        table::{TableState, SortDirection},
        filter::FilterBarState,
        export::ExportFormat,
    },
};

/// `None` and `Add` are never constructed: `Add` routed to the item-add screen,
/// which ADR-0009 made dormant ("Can't reach this state anymore",
/// `ui/inventory/inventory.rs:35`). They are still *matched* there, so deleting
/// them would delete the add screen with them — that is a decision about the
/// dormant write path, not a dead-code cleanup, and it is not taken here.
#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Copy)]
pub enum InventoryRoute {
    None,
    Add,
    #[default]
    Browse,
}

/// Storage location filter for inventory browse
#[derive(Default, Clone, Copy, PartialEq)]
pub enum StorageLocation {
    #[default]
    All,
    Equipped,
    StorageBox,
}

/// View state for inventory browse page
#[derive(Clone)]
pub struct BrowseViewState {
    pub storage_location: StorageLocation,
    pub type_filter: InventoryTypeRoute,
    pub search: String,
    pub table_state: TableState,
    pub filter_state: FilterBarState,
    pub export_format: ExportFormat,
    pub export_filtered_only: bool,
}

impl Default for BrowseViewState {
    fn default() -> Self {
        Self {
            storage_location: StorageLocation::All,
            type_filter: InventoryTypeRoute::CommonItems,
            search: String::new(),
            table_state: TableState::new().with_sort("name", SortDirection::Ascending),
            filter_state: FilterBarState::new(),
            export_format: ExportFormat::Json,
            export_filtered_only: false,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum InventoryTypeRoute {
    #[default]
    CommonItems,
    KeyItems,
    Weapons,
    Armors,
    AshOfWar,
    Talismans,
}

impl InventoryTypeRoute {
    pub fn label(&self) -> &'static str {
        match self {
            InventoryTypeRoute::CommonItems => "Common Items",
            InventoryTypeRoute::KeyItems => "Key Items",
            InventoryTypeRoute::Weapons => "Weapons",
            InventoryTypeRoute::Armors => "Armors",
            InventoryTypeRoute::AshOfWar => "Ashes of War",
            InventoryTypeRoute::Talismans => "Talismans",
        }
    }

    pub fn all_variants() -> &'static [InventoryTypeRoute] {
        &[
            InventoryTypeRoute::CommonItems,
            InventoryTypeRoute::KeyItems,
            InventoryTypeRoute::Weapons,
            InventoryTypeRoute::Armors,
            InventoryTypeRoute::AshOfWar,
            InventoryTypeRoute::Talismans,
        ]
    }
}

#[derive(PartialEq, Clone, Default, Copy)]
#[repr(i64)] // discriminants exceed i32; every read site casts `as u32`
pub enum InventoryItemType {
    #[default]
    None = -1,
    Weapon = 0x0,
    Armor = 0x10000000,
    Accessory = 0x20000000,
    Item = 0x40000000,
    Aow = 0x80000000,
}
impl From<u8> for InventoryItemType {
    fn from(value: u8) -> Self {
        match value {
            0x0 => InventoryItemType::Weapon,
            0x10 => InventoryItemType::Armor,
            0x20 => InventoryItemType::Accessory,
            0x40 => InventoryItemType::Item,
            0x80 => InventoryItemType::Aow,
            _ => InventoryItemType::None,
        }
    }
}
impl std::fmt::Display for InventoryItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            InventoryItemType::None => "None",
            InventoryItemType::Weapon => "WEAPON",
            InventoryItemType::Armor => "ARMOR",
            InventoryItemType::Accessory => "ACCESSORY",
            InventoryItemType::Item => "ITEM",
            InventoryItemType::Aow => "AOW",
        })
    }
}

#[derive(Default, Clone, PartialEq)]
#[repr(i64)] // discriminants exceed i32; every read site casts `as u32`
pub enum InventoryGaitemType {
    #[default]
    Empty = -1,
    Weapon = 0x80000000,
    Armor = 0x90000000,
    Accessory = 0xa0000000,
    Item = 0xb0000000,
    Aow = 0xc0000000,
}
impl From<u32> for InventoryGaitemType {
    fn from(value: u32) -> Self {
        match value {
            x if x == InventoryGaitemType::Weapon as u32 => InventoryGaitemType::Weapon,
            x if x == InventoryGaitemType::Armor as u32 => InventoryGaitemType::Armor,
            x if x == InventoryGaitemType::Accessory as u32 => InventoryGaitemType::Accessory,
            x if x == InventoryGaitemType::Item as u32 => InventoryGaitemType::Item,
            x if x == InventoryGaitemType::Aow as u32 => InventoryGaitemType::Aow,
            _ => InventoryGaitemType::Empty,
        }
    }
}

#[derive(Default, Clone)]
pub struct InventoryItemViewModel {
    pub ga_item_handle: u32,
    pub item_id: u32,
    pub item_name: String,
    pub quantity: u32,
    pub inventory_index: u32,
    /// Unread by the reader, but part of the save's per-item record.
    #[allow(dead_code)]
    pub equip_index: u32,
    pub r#type: InventoryGaitemType,
}

/// A table name for `id`, or the `[UNKOWN_{id}]` fallback for a missing or empty
/// entry (spelling kept deliberately for output stability).
fn name_or_unknown(found: Option<&&str>, id: u32) -> String {
    match found {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => format!("[UNKOWN_{}]", id),
    }
}

/// The reader's display name (Enrichment, ADR-0010) for an inventory item's gaitem
/// type and its already-decoded id. This is the **single source** shared by
/// [`InventoryItemViewModel::from_save`] and the facts-sourced inventory browse view,
/// so the two cannot drift about an item's name. Weapons key off the base id and show
/// the reinforcement level (`id % 100`, from the full reinforced id); the other
/// families look the id up directly. The id-decode (the reconstruction) stays with the
/// caller; only the name (Enrichment) lives here.
pub fn resolve_item_name(gaitem_type: &InventoryGaitemType, id: u32) -> String {
    match gaitem_type {
        InventoryGaitemType::Weapon => {
            let base = (id / 100) * 100;
            let upgrade = id % 100;
            match WEAPON_NAME.lock().unwrap().get(&base) {
                Some(name) if !name.is_empty() => {
                    if upgrade > 0 {
                        format!("{} +{}", name, upgrade)
                    } else {
                        name.to_string()
                    }
                }
                _ => format!("[UNKOWN_{}]", base),
            }
        }
        InventoryGaitemType::Armor => name_or_unknown(ARMOR_NAME.lock().unwrap().get(&id), id),
        InventoryGaitemType::Accessory => {
            name_or_unknown(ACCESSORY_NAME.lock().unwrap().get(&id), id)
        }
        InventoryGaitemType::Item => name_or_unknown(ITEM_NAME.lock().unwrap().get(&id), id),
        InventoryGaitemType::Aow => name_or_unknown(AOW_NAME.lock().unwrap().get(&id), id),
        InventoryGaitemType::Empty => String::new(),
    }
}

impl InventoryItemViewModel {
    pub fn from_save(
        item_info: &EquipInventoryItem,
        equip_index: u32,
        gaitem: &GaItem,
        gaitem_type: InventoryGaitemType,
    ) -> Self {
        let gaitem_handle = item_info.ga_item_handle;

        // Decode the item id per family (this is the reconstruction). A weapon keeps
        // its full reinforced id; the others clear their type tag. The display name
        // then comes from the shared Enrichment resolver, so this ViewModel and the
        // browse view (rendering from the core's facts) resolve names identically.
        let item_id = match gaitem_type {
            InventoryGaitemType::Weapon => gaitem.item_id,
            InventoryGaitemType::Armor => gaitem.item_id ^ InventoryItemType::Armor as u32,
            InventoryGaitemType::Accessory => gaitem_handle ^ InventoryGaitemType::Accessory as u32,
            InventoryGaitemType::Item => gaitem_handle ^ InventoryGaitemType::Item as u32,
            InventoryGaitemType::Aow => gaitem.item_id ^ InventoryItemType::Aow as u32,
            InventoryGaitemType::Empty => panic!("We shouldn't reach this!"),
        };

        Self {
            ga_item_handle: item_info.ga_item_handle,
            item_id,
            item_name: resolve_item_name(&gaitem_type, item_id),
            quantity: item_info.quantity,
            inventory_index: item_info.inventory_index,
            equip_index,
            r#type: gaitem_type,
        }
    }
}

#[derive(Default, Clone)]
pub struct InventoryStorage {
    pub common_items: Vec<InventoryItemViewModel>,
    pub key_items: Vec<InventoryItemViewModel>,

    pub filtered_items: Vec<InventoryItemViewModel>,
    pub filtered_key_items: Vec<InventoryItemViewModel>,
    pub filtered_weapons: Vec<InventoryItemViewModel>,
    pub filtered_armors: Vec<InventoryItemViewModel>,
    pub filtered_aows: Vec<InventoryItemViewModel>,
    pub filtered_accessories: Vec<InventoryItemViewModel>,

    pub common_item_count: u32,
    pub key_item_count: u32,
    pub next_acquisition_sort_order_index: u32,
    pub next_equip_index: u32,
}

#[derive(Default, Clone)]
pub struct InventoryViewModel {
    // Navigation
    pub at_single_items: bool,
    pub current_route: InventoryRoute,
    pub current_type_route: InventoryTypeRoute,
    pub current_bulk_type_route: InventoryTypeRoute,

    // Data
    pub filter_text: String,
    pub storage: Vec<InventoryStorage>,
    pub gaitem_map: Vec<GaItem>,
    pub projectile_list: EquipProjectileData,
    pub gaitem_data: GaItemData,
    pub bulk_items_selected: Vec<HashMap<u32, bool>>,
    pub bulk_items_max_quantity: bool,
    pub bulk_items_arrow_quantity: u32,
    pub bulk_items_weapon_level: u32,

    // Used for unqeuipping weapon or armor
    pub unarmed: InventoryItemViewModel,
    pub naked_head: InventoryItemViewModel,
    pub naked_body: InventoryItemViewModel,
    pub naked_arms: InventoryItemViewModel,
    pub naked_legs: InventoryItemViewModel,

    // Changed indicator
    pub changed: bool,

    // Log
    pub log: Vec<String>,

    // Indexes for when adding items
    next_gaitem_handle: u32,
    part_gaitem_handle: u8,
    next_aow_index: usize,
    next_armament_or_armor_index: usize,

    // Browse view state
    pub browse_view_state: BrowseViewState,
}

impl InventoryViewModel {
    pub fn from_save(slot: &SaveSlot) -> Self {
        let mut inventory_vm = InventoryViewModel {
            at_single_items: true,
            current_route: InventoryRoute::Browse, // Default to Browse
            storage: vec![InventoryStorage::default(); 2],
            ..Default::default()
        };
        inventory_vm.replace_bulk_items_selected_map(InventoryTypeRoute::CommonItems);

        // Gaitem_map
        inventory_vm.gaitem_map = slot.ga_items.clone();

        // Gaitem_data
        inventory_vm.gaitem_data = slot.ga_item_data.clone();

        // Projectile list
        inventory_vm.projectile_list = slot.equip_projectile_data.clone();

        // Find the next gaitem_handle used when adding new weapon, armors or ashes of war
        inventory_vm
            .gaitem_map
            .iter()
            .enumerate()
            .for_each(|(index, gaitem)| {
                if (gaitem.gaitem_handle & 0xF0000000) == InventoryGaitemType::Aow as u32 {
                    inventory_vm.next_aow_index = index;
                }
                if (gaitem.gaitem_handle & 0xFFFF) > (inventory_vm.next_gaitem_handle) {
                    inventory_vm.next_gaitem_handle = gaitem.gaitem_handle & 0xFFFF;
                    inventory_vm.next_armament_or_armor_index = index;
                }
            });
        inventory_vm.part_gaitem_handle =
            ((inventory_vm.gaitem_map[0].gaitem_handle >> 16) & 0xFF) as u8;

        inventory_vm.next_gaitem_handle += 1;
        inventory_vm.next_aow_index += 1;
        inventory_vm.next_armament_or_armor_index += 1;

        inventory_vm.fill_stroage_type(
            &slot.equip_inventory_data,
            slot.equip_inventory_data.next_acquisition_sort_id,
            slot.equip_inventory_data.next_equip_index,
            0,
        );
        inventory_vm.fill_stroage_type(
            &slot.storage_inventory_data,
            slot.storage_inventory_data.next_acquisition_sort_id,
            slot.storage_inventory_data.next_equip_index,
            1,
        );

        inventory_vm
    }

    fn fill_stroage_type(
        &mut self,
        equip_inventory_data: &EquipInventoryData,
        next_acquisition_sort_id: u32,
        next_equip_index: u32,
        inventory_storage_index: usize,
    ) {
        let inventory_storage = &mut self.storage[inventory_storage_index];

        for (index, item) in equip_inventory_data.common_items.iter().enumerate() {
            // Determine item type from gaitem_handle
            let inventory_gaitem_type = InventoryGaitemType::from(item.ga_item_handle & 0xf0000000);

            // Equip_index
            let equip_index = (index as u32) + 0x180;

            match inventory_gaitem_type {
                InventoryGaitemType::Weapon => {
                    let gaitem = self
                        .gaitem_map
                        .iter()
                        .find(|gaitem| gaitem.gaitem_handle == item.ga_item_handle)
                        .unwrap();
                    let inventory_item_vm = InventoryItemViewModel::from_save(
                        item,
                        equip_index,
                        gaitem,
                        InventoryGaitemType::Weapon,
                    );
                    if inventory_item_vm.item_id == 110000 && self.unarmed.item_id != 110000 {
                        self.unarmed = inventory_item_vm.clone();
                    }
                    inventory_storage
                        .common_items
                        .push(inventory_item_vm.clone());
                    inventory_storage.filtered_weapons.push(inventory_item_vm);
                }
                InventoryGaitemType::Armor => {
                    let gaitem = self
                        .gaitem_map
                        .iter()
                        .find(|gaitem| gaitem.gaitem_handle == item.ga_item_handle)
                        .unwrap();
                    let inventory_item_vm = InventoryItemViewModel::from_save(
                        item,
                        equip_index,
                        gaitem,
                        InventoryGaitemType::Armor,
                    );
                    if inventory_item_vm.item_id == 10000 {
                        self.naked_head = inventory_item_vm.clone();
                    } else if inventory_item_vm.item_id == 10100 {
                        self.naked_body = inventory_item_vm.clone();
                    } else if inventory_item_vm.item_id == 10200 {
                        self.naked_arms = inventory_item_vm.clone();
                    } else if inventory_item_vm.item_id == 10300 {
                        self.naked_legs = inventory_item_vm.clone();
                    }
                    inventory_storage
                        .common_items
                        .push(inventory_item_vm.clone());
                    inventory_storage.filtered_armors.push(inventory_item_vm);
                }
                InventoryGaitemType::Accessory => {
                    let inventory_item_vm = InventoryItemViewModel::from_save(
                        item,
                        equip_index,
                        &GaItem::default(),
                        InventoryGaitemType::Accessory,
                    );
                    inventory_storage
                        .common_items
                        .push(inventory_item_vm.clone());
                    inventory_storage
                        .filtered_accessories
                        .push(inventory_item_vm);
                }
                InventoryGaitemType::Item => {
                    let inventory_item_vm = InventoryItemViewModel::from_save(
                        item,
                        equip_index,
                        &GaItem::default(),
                        InventoryGaitemType::Item,
                    );
                    inventory_storage
                        .common_items
                        .push(inventory_item_vm.clone());
                    inventory_storage.filtered_items.push(inventory_item_vm);
                }
                InventoryGaitemType::Aow => {
                    let gaitem = self
                        .gaitem_map
                        .iter()
                        .find(|gaitem| gaitem.gaitem_handle == item.ga_item_handle)
                        .unwrap();
                    let inventory_item_vm = InventoryItemViewModel::from_save(
                        item,
                        equip_index,
                        gaitem,
                        InventoryGaitemType::Aow,
                    );
                    inventory_storage
                        .common_items
                        .push(inventory_item_vm.clone());
                    inventory_storage.filtered_aows.push(inventory_item_vm);
                }
                InventoryGaitemType::Empty => {
                    inventory_storage
                        .common_items
                        .push(InventoryItemViewModel::default());
                }
            }
        }

        for key_item in equip_inventory_data.key_items.iter() {
            let inventory_item_vm = InventoryItemViewModel::from_save(
                key_item,
                0,
                &GaItem::default(),
                InventoryGaitemType::Item,
            );
            inventory_storage.key_items.push(inventory_item_vm);
        }

        inventory_storage
            .filtered_weapons
            .sort_by(|a, b| a.item_name.cmp(&b.item_name));
        inventory_storage
            .filtered_armors
            .sort_by(|a, b| a.item_name.cmp(&b.item_name));
        inventory_storage
            .filtered_accessories
            .sort_by(|a, b| a.item_name.cmp(&b.item_name));
        inventory_storage
            .filtered_items
            .sort_by(|a, b| a.item_name.cmp(&b.item_name));
        inventory_storage
            .filtered_key_items
            .sort_by(|a, b| a.item_name.cmp(&b.item_name));
        inventory_storage
            .filtered_aows
            .sort_by(|a, b| a.item_name.cmp(&b.item_name));

        inventory_storage.common_item_count =
            equip_inventory_data.common_inventory_items_distinct_count;
        inventory_storage.key_item_count = equip_inventory_data.key_inventory_items_distinct_count;
        inventory_storage.next_acquisition_sort_order_index = next_acquisition_sort_id;
        inventory_storage.next_equip_index = next_equip_index;
    }

    pub fn filter(&mut self) {
        for inventory_storage in &mut self.storage {
            inventory_storage.filtered_weapons = inventory_storage
                .common_items
                .iter()
                .filter(|i| {
                    if i.r#type != InventoryGaitemType::Weapon {
                        return false;
                    }
                    if self.filter_text.is_empty() {
                        return true;
                    }
                    i.item_name
                        .to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                })
                .cloned()
                .collect();

            inventory_storage.filtered_armors = inventory_storage
                .common_items
                .iter()
                .filter(|i| {
                    if i.r#type != InventoryGaitemType::Armor {
                        return false;
                    }
                    if self.filter_text.is_empty() {
                        return true;
                    }
                    i.item_name
                        .to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                })
                .cloned()
                .collect();

            inventory_storage.filtered_accessories = inventory_storage
                .common_items
                .iter()
                .filter(|i| {
                    if i.r#type != InventoryGaitemType::Accessory {
                        return false;
                    }
                    if self.filter_text.is_empty() {
                        return true;
                    }
                    i.item_name
                        .to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                })
                .cloned()
                .collect();

            inventory_storage.filtered_items = inventory_storage
                .common_items
                .iter()
                .filter(|i| {
                    if i.r#type != InventoryGaitemType::Item {
                        return false;
                    }
                    if self.filter_text.is_empty() {
                        return true;
                    }
                    i.item_name
                        .to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                })
                .cloned()
                .collect();

            inventory_storage.filtered_key_items = inventory_storage
                .key_items
                .iter()
                .filter(|i| {
                    if i.quantity == 0 {
                        return false;
                    }
                    if self.filter_text.is_empty() {
                        return true;
                    }
                    i.item_name
                        .to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                })
                .cloned()
                .collect();

            inventory_storage.filtered_aows = inventory_storage
                .common_items
                .iter()
                .filter(|i| {
                    if i.r#type != InventoryGaitemType::Aow {
                        return false;
                    }
                    if self.filter_text.is_empty() {
                        return true;
                    }
                    i.item_name
                        .to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
                })
                .cloned()
                .collect();

            inventory_storage.filtered_weapons.sort_by(|a, b| {
                if self.filter_text.is_empty() {
                    return a.item_name.cmp(&b.item_name);
                }
                let a_contains = a
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                let b_contains = b
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                if a_contains && !b_contains {
                    Ordering::Less
                } else if !a_contains && b_contains {
                    Ordering::Greater
                } else {
                    a.item_name.cmp(&b.item_name)
                }
            });

            inventory_storage.filtered_armors.sort_by(|a, b| {
                if self.filter_text.is_empty() {
                    return a.item_name.cmp(&b.item_name);
                }
                let a_contains = a
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                let b_contains = b
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                if a_contains && !b_contains {
                    Ordering::Less
                } else if !a_contains && b_contains {
                    Ordering::Greater
                } else {
                    a.item_name.cmp(&b.item_name)
                }
            });

            inventory_storage.filtered_accessories.sort_by(|a, b| {
                if self.filter_text.is_empty() {
                    return a.item_name.cmp(&b.item_name);
                }
                let a_contains = a
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                let b_contains = b
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                if a_contains && !b_contains {
                    Ordering::Less
                } else if !a_contains && b_contains {
                    Ordering::Greater
                } else {
                    a.item_name.cmp(&b.item_name)
                }
            });

            inventory_storage.filtered_items.sort_by(|a, b| {
                if self.filter_text.is_empty() {
                    return a.item_name.cmp(&b.item_name);
                }
                let a_contains = a
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                let b_contains = b
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                if a_contains && !b_contains {
                    Ordering::Less
                } else if !a_contains && b_contains {
                    Ordering::Greater
                } else {
                    a.item_name.cmp(&b.item_name)
                }
            });

            inventory_storage.filtered_key_items.sort_by(|a, b| {
                if self.filter_text.is_empty() {
                    return a.item_name.cmp(&b.item_name);
                }
                let a_contains = a
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                let b_contains = b
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                if a_contains && !b_contains {
                    Ordering::Less
                } else if !a_contains && b_contains {
                    Ordering::Greater
                } else {
                    a.item_name.cmp(&b.item_name)
                }
            });

            inventory_storage.filtered_aows.sort_by(|a, b| {
                if self.filter_text.is_empty() {
                    return a.item_name.cmp(&b.item_name);
                }
                let a_contains = a
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                let b_contains = b
                    .item_name
                    .to_lowercase()
                    .contains(&self.filter_text.to_lowercase());
                if a_contains && !b_contains {
                    Ordering::Less
                } else if !a_contains && b_contains {
                    Ordering::Greater
                } else {
                    a.item_name.cmp(&b.item_name)
                }
            });
        }
    }

}

// Splitting up inventory into multiple files
mod add_bulk;
mod add_single;
