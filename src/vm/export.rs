use serde::Serialize;

#[derive(Serialize)]
pub struct ExportData {
    pub metadata: ExportMetadata,
    pub general: ExportGeneral,
    pub stats: ExportStats,
    pub equipment: ExportEquipment,
    pub inventory: ExportInventory,
    pub events: ExportEvents,
    pub regions: ExportRegions,
}

#[derive(Serialize)]
pub struct ExportMetadata {
    pub export_version: String,
    pub export_date: String,
    pub slot_index: usize,
    pub steam_id: u64,
}

#[derive(Serialize)]
pub struct ExportGeneral {
    pub character_name: String,
    pub gender: String,
    pub level: u32,
    pub souls: u32,
    pub souls_memory: u32,
    pub arche_type: u8,
    pub arche_type_name: String,
    pub gift: u8,
    pub match_making_weapon_level: u8,
}

#[derive(Serialize)]
pub struct ExportStats {
    pub vigor: u32,
    pub mind: u32,
    pub endurance: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub faith: u32,
    pub arcane: u32,
    pub scadutree_level: u32,
    pub spirit_ash_level: u32,
}

#[derive(Serialize)]
pub struct ExportEquipmentItem {
    pub slot_name: String,
    pub gaitem_handle: String,
    pub gaitem_handle_raw: u32,
    pub item_id: u32,
    pub item_id_hex: String,
    pub item_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrade_level: Option<u32>,
}

impl ExportEquipmentItem {
    pub fn new(slot_name: &str, gaitem_handle: u32, item_id: u32, item_name: &str) -> Self {
        let upgrade_level = if item_id > 0 && item_id % 100 != 0 {
            Some(item_id % 100)
        } else {
            None
        };

        Self {
            slot_name: slot_name.to_string(),
            gaitem_handle: format!("0x{:08X}", gaitem_handle),
            gaitem_handle_raw: gaitem_handle,
            item_id,
            item_id_hex: format!("0x{:08X}", item_id),
            item_name: item_name.to_string(),
            upgrade_level,
        }
    }

    pub fn empty(slot_name: &str) -> Self {
        Self {
            slot_name: slot_name.to_string(),
            gaitem_handle: "0x00000000".to_string(),
            gaitem_handle_raw: 0,
            item_id: 0,
            item_id_hex: "0x00000000".to_string(),
            item_name: "Empty".to_string(),
            upgrade_level: None,
        }
    }
}

#[derive(Serialize)]
pub struct ExportEquipment {
    pub left_hand_armaments: Vec<ExportEquipmentItem>,
    pub right_hand_armaments: Vec<ExportEquipmentItem>,
    pub arrows: Vec<ExportEquipmentItem>,
    pub bolts: Vec<ExportEquipmentItem>,
    pub head: ExportEquipmentItem,
    pub chest: ExportEquipmentItem,
    pub arms: ExportEquipmentItem,
    pub legs: ExportEquipmentItem,
    pub talismans: Vec<ExportEquipmentItem>,
    pub quick_slots: Vec<ExportEquipmentItem>,
    pub pouch: Vec<ExportEquipmentItem>,
}

#[derive(Serialize)]
pub struct ExportInventoryItem {
    pub gaitem_handle: String,
    pub gaitem_handle_raw: u32,
    pub item_id: u32,
    pub item_id_hex: String,
    pub item_name: String,
    pub quantity: u32,
    pub inventory_index: u32,
    pub item_type: String,
}

impl ExportInventoryItem {
    pub fn new(
        gaitem_handle: u32,
        item_id: u32,
        item_name: &str,
        quantity: u32,
        inventory_index: u32,
        item_type: &str,
    ) -> Self {
        Self {
            gaitem_handle: format!("0x{:08X}", gaitem_handle),
            gaitem_handle_raw: gaitem_handle,
            item_id,
            item_id_hex: format!("0x{:08X}", item_id),
            item_name: item_name.to_string(),
            quantity,
            inventory_index,
            item_type: item_type.to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct ExportInventory {
    pub common_item_count: u32,
    pub key_item_count: u32,
    pub weapons: Vec<ExportInventoryItem>,
    pub armors: Vec<ExportInventoryItem>,
    pub accessories: Vec<ExportInventoryItem>,
    pub items: Vec<ExportInventoryItem>,
    pub ashes_of_war: Vec<ExportInventoryItem>,
    pub key_items: Vec<ExportInventoryItem>,
}

#[derive(Serialize)]
pub struct ExportEventItem {
    pub name: String,
    pub discovered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

impl ExportEventItem {
    pub fn new(name: &str, discovered: bool) -> Self {
        Self {
            name: name.to_string(),
            discovered,
            region: None,
        }
    }

    pub fn with_region(name: &str, discovered: bool, region: &str) -> Self {
        Self {
            name: name.to_string(),
            discovered,
            region: Some(region.to_string()),
        }
    }
}

#[derive(Serialize)]
pub struct ExportEvents {
    pub graces: Vec<ExportEventItem>,
    pub bosses: Vec<ExportEventItem>,
    pub summoning_pools: Vec<ExportEventItem>,
    pub colosseums: Vec<ExportEventItem>,
    pub whetblades: Vec<ExportEventItem>,
    pub cookbooks: Vec<ExportEventItem>,
    pub maps: Vec<ExportEventItem>,
}

#[derive(Serialize)]
pub struct ExportRegionItem {
    pub name: String,
    pub unlocked: bool,
    pub is_open_world: bool,
    pub is_dungeon: bool,
    pub is_boss: bool,
}

impl ExportRegionItem {
    pub fn new(name: &str, unlocked: bool, is_open_world: bool, is_dungeon: bool, is_boss: bool) -> Self {
        Self {
            name: name.to_string(),
            unlocked,
            is_open_world,
            is_dungeon,
            is_boss,
        }
    }
}

#[derive(Serialize)]
pub struct ExportRegions {
    pub regions: Vec<ExportRegionItem>,
}
