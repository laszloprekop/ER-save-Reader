#!/usr/bin/env python3
"""
Extract world pickup data from ItemLotParam_map.param.xml and generate world_pickups.rs
"""

import xml.etree.ElementTree as ET
from pathlib import Path
from collections import defaultdict

BASE_PATH = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/regulation-bin")
OUTPUT_FILE = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/src/db/world_pickups.rs")

def load_item_names() -> dict[int, str]:
    """Load item names from EquipParamGoods."""
    names = {}
    try:
        tree = ET.parse(BASE_PATH / "EquipParamGoods.param.xml")
        root = tree.getroot()
        for row in root.findall('.//row'):
            item_id = int(row.get('id', '0'))
            name = row.get('paramdexName', '')
            if name:
                names[item_id] = name
    except Exception as e:
        print(f"Warning: Could not load goods names: {e}")
    return names

def load_weapon_names() -> dict[int, str]:
    """Load weapon names from EquipParamWeapon."""
    names = {}
    try:
        tree = ET.parse(BASE_PATH / "EquipParamWeapon.param.xml")
        root = tree.getroot()
        for row in root.findall('.//row'):
            item_id = int(row.get('id', '0'))
            name = row.get('paramdexName', '')
            if name:
                names[item_id] = name
    except Exception as e:
        print(f"Warning: Could not load weapon names: {e}")
    return names

def load_armor_names() -> dict[int, str]:
    """Load armor names from EquipParamProtector."""
    names = {}
    try:
        tree = ET.parse(BASE_PATH / "EquipParamProtector.param.xml")
        root = tree.getroot()
        for row in root.findall('.//row'):
            item_id = int(row.get('id', '0'))
            name = row.get('paramdexName', '')
            if name:
                names[item_id] = name
    except Exception as e:
        print(f"Warning: Could not load armor names: {e}")
    return names

def load_accessory_names() -> dict[int, str]:
    """Load talisman names from EquipParamAccessory."""
    names = {}
    try:
        tree = ET.parse(BASE_PATH / "EquipParamAccessory.param.xml")
        root = tree.getroot()
        for row in root.findall('.//row'):
            item_id = int(row.get('id', '0'))
            name = row.get('paramdexName', '')
            if name:
                names[item_id] = name
    except Exception as e:
        print(f"Warning: Could not load accessory names: {e}")
    return names

def category_to_type(category: int) -> tuple[str, dict]:
    """Map category ID to item type and name lookup."""
    # Category values from ItemLotParam
    # 0 = Weapon, 1 = Protector (Armor), 2 = Accessory, 3 = Goods, 4 = Gem (AoW)
    if category == 0:
        return "Weapon", "weapons"
    elif category == 1:
        return "Armor", "armor"
    elif category == 2:
        return "Accessory", "accessories"
    elif category == 3:
        return "Good", "goods"
    elif category == 4:
        return "AshOfWar", "goods"  # AoW uses goods IDs
    else:
        return "Unknown", "goods"

def decode_flag_location(flag_id: int) -> tuple[str, int, int]:
    """Decode a 10-digit flag ID to region and tile coordinates."""
    if flag_id < 1000000000:
        return "Unknown", 0, 0

    # Format: 1XXYYZZZZ where XX=tileX, YY=tileY
    flag_str = str(flag_id)
    if len(flag_str) == 10 and flag_str[0] in ('1', '2'):
        try:
            prefix = flag_str[0]  # 1=base game, 2=DLC
            tile_x = int(flag_str[1:3])
            tile_y = int(flag_str[3:5])

            # Map tile coordinates to region names
            if prefix == '1':
                return get_region_name(tile_x, tile_y), tile_x, tile_y
            else:
                return "Shadow of the Erdtree", tile_x, tile_y
        except:
            pass

    return "Unknown", 0, 0

def get_region_name(tile_x: int, tile_y: int) -> str:
    """Get region name from tile coordinates."""
    # Approximate region mapping based on tile coordinates
    regions = {
        # Limgrave (30-44, 36-46)
        ((30, 44), (36, 46)): "Limgrave",
        # Weeping Peninsula (30-44, 30-36)
        ((30, 44), (30, 36)): "Weeping Peninsula",
        # Liurnia (36-50, 46-58)
        ((36, 50), (46, 58)): "Liurnia of the Lakes",
        # Caelid (44-52, 36-52)
        ((44, 52), (36, 52)): "Caelid",
        # Altus Plateau (44-52, 52-60)
        ((44, 52), (52, 60)): "Altus Plateau",
        # Mt. Gelmir (38-46, 54-62)
        ((38, 46), (54, 62)): "Mt. Gelmir",
        # Mountaintops (46-56, 52-60)
        ((46, 56), (52, 60)): "Mountaintops of the Giants",
    }

    for ((x_min, x_max), (y_min, y_max)), region in regions.items():
        if x_min <= tile_x <= x_max and y_min <= tile_y <= y_max:
            return region

    return "Lands Between"

def main():
    print("Loading item name databases...")
    goods_names = load_item_names()
    weapon_names = load_weapon_names()
    armor_names = load_armor_names()
    accessory_names = load_accessory_names()

    name_lookups = {
        "goods": goods_names,
        "weapons": weapon_names,
        "armor": armor_names,
        "accessories": accessory_names,
    }

    print("Parsing ItemLotParam_map.param.xml...")
    tree = ET.parse(BASE_PATH / "ItemLotParam_map.param.xml")
    root = tree.getroot()

    pickups = []
    regions = defaultdict(list)

    for row in root.findall('.//row'):
        lot_id = int(row.get('id', '0'))
        flag_id = int(row.get('getItemFlagId', '0'))

        if flag_id == 0:
            continue

        # Get first item in the lot
        item_id = int(row.get('lotItemId01', '0'))
        category = int(row.get('lotItemCategory01', '0'))
        quantity = int(row.get('lotItemNum01', '1'))

        if item_id == 0:
            continue

        item_type, lookup_key = category_to_type(category)
        lookup = name_lookups.get(lookup_key, {})
        item_name = lookup.get(item_id, f"Unknown Item {item_id}")

        region, tile_x, tile_y = decode_flag_location(flag_id)

        pickup = {
            'lot_id': lot_id,
            'flag_id': flag_id,
            'item_id': item_id,
            'item_type': item_type,
            'item_name': item_name,
            'quantity': quantity,
            'region': region,
            'tile_x': tile_x,
            'tile_y': tile_y,
        }

        pickups.append(pickup)
        regions[region].append(lot_id)

    print(f"Found {len(pickups)} world pickups across {len(regions)} regions")

    # Generate Rust code
    rust_code = '''// Auto-generated world pickups database from ItemLotParam_map.param.xml
// Contains {} world pickups

use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickupItemType {{
    Weapon,
    Armor,
    Accessory,
    Good,
    AshOfWar,
    Unknown,
}}

#[derive(Debug, Clone)]
pub struct WorldPickup {{
    pub lot_id: u32,
    pub flag_id: u32,
    pub item_id: u32,
    pub item_type: PickupItemType,
    pub item_name: &'static str,
    pub quantity: u8,
    pub region: &'static str,
    pub tile_x: u8,
    pub tile_y: u8,
}}

/// All world pickups indexed by item lot ID
pub static WORLD_PICKUPS: Lazy<HashMap<u32, WorldPickup>> = Lazy::new(|| {{
    let mut map = HashMap::new();
'''.format(len(pickups))

    for p in pickups:
        item_name = p['item_name'].replace('"', '\\"')
        region = p['region'].replace('"', '\\"')
        rust_code += f'''    map.insert({p['lot_id']}, WorldPickup {{
        lot_id: {p['lot_id']},
        flag_id: {p['flag_id']},
        item_id: {p['item_id']},
        item_type: PickupItemType::{p['item_type']},
        item_name: "{item_name}",
        quantity: {p['quantity']},
        region: "{region}",
        tile_x: {p['tile_x']},
        tile_y: {p['tile_y']},
    }});
'''

    rust_code += '''    map
});

/// Index of world pickups by flag ID
pub static PICKUPS_BY_FLAG: Lazy<HashMap<u32, u32>> = Lazy::new(|| {
    WORLD_PICKUPS.iter()
        .map(|(lot_id, pickup)| (pickup.flag_id, *lot_id))
        .collect()
});

/// Get world pickup by lot ID
pub fn get_pickup(lot_id: u32) -> Option<&'static WorldPickup> {
    WORLD_PICKUPS.get(&lot_id)
}

/// Get world pickup by flag ID
pub fn get_pickup_by_flag(flag_id: u32) -> Option<&'static WorldPickup> {
    PICKUPS_BY_FLAG.get(&flag_id)
        .and_then(|lot_id| WORLD_PICKUPS.get(lot_id))
}

/// Get all pickups in a region
pub fn get_pickups_in_region(region: &str) -> Vec<&'static WorldPickup> {
    WORLD_PICKUPS.values()
        .filter(|p| p.region == region)
        .collect()
}

/// Get all unique regions
pub fn get_regions() -> Vec<&'static str> {
    let mut regions: Vec<_> = WORLD_PICKUPS.values()
        .map(|p| p.region)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    regions.sort();
    regions
}
'''

    OUTPUT_FILE.write_text(rust_code)
    print(f"Generated {OUTPUT_FILE}")
    print(f"Regions: {list(regions.keys())}")

if __name__ == "__main__":
    main()
