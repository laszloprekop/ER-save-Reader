#!/usr/bin/env python3
"""
Extract world pickups from ItemLotParam_map.param.xml with proper item name resolution.
Uses all equip param files to resolve item names.
"""

import xml.etree.ElementTree as ET
import re
from pathlib import Path

# Path to game files
GAME_FILES = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/regulation-bin")

# Item category mapping from ItemLotParam_map
# lotItemCategory values:
# 0 = Weapon
# 1 = Protector (Armor)
# 2 = Accessory
# 3 = Goods
# 4 = Gem (Ashes of War)

def load_item_names(param_file: Path) -> dict[int, str]:
    """Load item ID -> name mapping from a param XML file."""
    items = {}
    if not param_file.exists():
        print(f"Warning: {param_file} not found")
        return items

    tree = ET.parse(param_file)
    root = tree.getroot()

    for row in root.findall('.//row'):
        item_id = int(row.get('id', 0))
        name = row.get('paramdexName', '')
        if name and not name.startswith('Type '):  # Skip template entries
            items[item_id] = name

    print(f"Loaded {len(items)} items from {param_file.name}")
    return items

def load_all_item_databases():
    """Load all item name databases."""
    databases = {
        0: load_item_names(GAME_FILES / "EquipParamWeapon.param.xml"),      # Weapon
        1: load_item_names(GAME_FILES / "EquipParamProtector.param.xml"),   # Armor
        2: load_item_names(GAME_FILES / "EquipParamAccessory.param.xml"),   # Accessory
        3: load_item_names(GAME_FILES / "EquipParamGoods.param.xml"),       # Goods
        4: load_item_names(GAME_FILES / "EquipParamGem.param.xml"),         # Ash of War (Gem)
    }
    return databases

def resolve_item_name(databases: dict, item_id: int, category: int) -> tuple[str, str]:
    """
    Resolve item name by trying multiple databases based on category.
    Returns (item_name, item_type).

    Category mapping (based on game data analysis):
    0 = Weapon
    1 = Protector (Armor)
    2 = Protector or Weapon (depends on ID range)
    3 = Goods
    4 = Accessory (Talismans)
    5 = Gem (Ashes of War)
    6 = Goods (Spells)
    """
    # Define search order for each category
    category_search = {
        0: [(0, "Weapon"), (3, "Good")],  # Weapon, fallback to Goods
        1: [(1, "Armor"), (3, "Good")],   # Protector, fallback to Goods
        2: [(1, "Armor"), (0, "Weapon"), (2, "Accessory")],  # Protector > Weapon > Accessory
        3: [(3, "Good"), (4, "AshOfWar"), (1, "Armor")],  # Goods > Gem > Protector
        4: [(2, "Accessory"), (3, "Good")],  # Accessory > Goods
        5: [(4, "AshOfWar"), (3, "Good")],  # Gem (Ash of War) > Goods
        6: [(3, "Good"), (4, "AshOfWar")],  # Goods (Spells) > Gem
    }

    search_order = category_search.get(category, [(3, "Good")])

    for db_key, item_type in search_order:
        db = databases.get(db_key, {})
        if item_id in db:
            return (db[item_id], item_type)

    # If not found in primary search, try all databases
    for db_key, item_type in [(0, "Weapon"), (1, "Armor"), (2, "Accessory"), (3, "Good"), (4, "AshOfWar")]:
        db = databases.get(db_key, {})
        if item_id in db:
            return (db[item_id], item_type)

    return (None, "Unknown")

def get_region_from_lot_id(lot_id: int) -> tuple[str, int, int]:
    """
    Determine region from lot ID pattern.
    Format: AABBCCDDDD where:
    - AA = map area (10, 11, 12, 30, 31, etc.)
    - BB = tile X
    - CC = tile Y
    - DDDD = item index

    Returns (region_name, tile_x, tile_y)
    """
    lot_str = str(lot_id)

    # Different patterns based on ID length
    if len(lot_str) >= 8:
        # 10-digit format: AABBCCDDDD
        area = lot_str[:2]
        tile_x = int(lot_str[2:4]) if len(lot_str) > 4 else 0
        tile_y = int(lot_str[4:6]) if len(lot_str) > 6 else 0
    elif len(lot_str) >= 6:
        # Shorter format
        area = lot_str[:2]
        tile_x = int(lot_str[2:4]) if len(lot_str) > 3 else 0
        tile_y = int(lot_str[4:6]) if len(lot_str) > 5 else 0
    else:
        return ("Unknown", 0, 0)

    # Map area codes to region names
    region_map = {
        "10": "Limgrave",
        "11": "Liurnia",
        "12": "Altus Plateau",
        "13": "Mt. Gelmir",
        "14": "Caelid",
        "15": "Mountaintops",
        "16": "Siofra River",
        "17": "Ainsel River",
        "18": "Deeproot Depths",
        "19": "Lake of Rot",
        "20": "Shadow Realm",  # DLC
        "21": "Shadow Realm",  # DLC
        "30": "Stormveil Castle",
        "31": "Raya Lucaria",
        "32": "Redmane Castle",
        "33": "Volcano Manor",
        "34": "Leyndell",
        "35": "Shunning-Grounds",
        "36": "Academy Crystal Cave",
        "37": "Ainsel Main",
        "38": "Nokron",
        "39": "Mohgwyn Palace",
        "40": "Elphael",
        "41": "Farum Azula",
        "50": "Tutorial",
        "60": "Overworld",  # Legacy dungeon items
    }

    region = region_map.get(area, f"Area {area}")
    return (region, tile_x, tile_y)

def category_to_type(category: int) -> str:
    """Convert lotItemCategory to PickupItemType."""
    type_map = {
        0: "Weapon",
        1: "Armor",
        2: "Accessory",
        3: "Good",
        4: "AshOfWar",
    }
    return type_map.get(category, "Unknown")

def escape_rust_string(s: str) -> str:
    """Escape a string for Rust."""
    return s.replace('\\', '\\\\').replace('"', '\\"')

def main():
    print("Loading item databases...")
    databases = load_all_item_databases()

    # Load ItemLotParam_map
    item_lot_file = GAME_FILES / "ItemLotParam_map.param.xml"
    print(f"\nParsing {item_lot_file}...")

    tree = ET.parse(item_lot_file)
    root = tree.getroot()

    pickups = []
    unknown_count = 0
    resolved_count = 0

    for row in root.findall('.//row'):
        lot_id = int(row.get('id', 0))
        flag_id = int(row.get('getItemFlagId', 0))

        # Process up to 8 item slots per lot
        for i in range(1, 9):
            item_id_attr = f'lotItemId{i:02d}'
            category_attr = f'lotItemCategory{i:02d}'
            num_attr = f'lotItemNum{i:02d}'

            item_id = row.get(item_id_attr)
            if item_id is None or item_id == '0':
                continue

            item_id = int(item_id)
            if item_id <= 0:  # Skip invalid or empty item IDs
                continue

            category = int(row.get(category_attr, 0))
            quantity = int(row.get(num_attr, 1))

            # Look up item name using smart resolution
            item_name, item_type = resolve_item_name(databases, item_id, category)

            if item_name:
                resolved_count += 1
            else:
                unknown_count += 1
                item_name = f"Unknown Item {item_id}"
                item_type = category_to_type(category)

            # Get region info
            region, tile_x, tile_y = get_region_from_lot_id(lot_id)

            pickups.append({
                'lot_id': lot_id,
                'flag_id': flag_id,
                'item_id': item_id,
                'item_name': item_name,
                'item_type': item_type,
                'quantity': quantity,
                'region': region,
                'tile_x': tile_x,
                'tile_y': tile_y,
            })

    print(f"\nProcessed {len(pickups)} pickup entries")
    print(f"Resolved: {resolved_count}, Unknown: {unknown_count}")
    print(f"Resolution rate: {resolved_count / (resolved_count + unknown_count) * 100:.1f}%")

    # Generate Rust code
    output_file = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/src/db/world_pickups.rs")

    with open(output_file, 'w') as f:
        f.write('//! World pickup database generated from ItemLotParam_map.param.xml\n')
        f.write('//! This file is auto-generated - do not edit manually\n\n')
        f.write('use once_cell::sync::Lazy;\n')
        f.write('use std::collections::HashMap;\n\n')

        f.write('#[derive(Debug, Clone, Copy, PartialEq)]\n')
        f.write('pub enum PickupItemType {\n')
        f.write('    Weapon,\n')
        f.write('    Armor,\n')
        f.write('    Accessory,\n')
        f.write('    Good,\n')
        f.write('    AshOfWar,\n')
        f.write('    Unknown,\n')
        f.write('}\n\n')

        f.write('#[derive(Debug, Clone)]\n')
        f.write('pub struct WorldPickup {\n')
        f.write('    pub flag_id: u32,\n')
        f.write('    pub item_id: u32,\n')
        f.write('    pub item_name: &\'static str,\n')
        f.write('    pub item_type: PickupItemType,\n')
        f.write('    pub quantity: u32,\n')
        f.write('    pub region: &\'static str,\n')
        f.write('    pub tile_x: u32,\n')
        f.write('    pub tile_y: u32,\n')
        f.write('}\n\n')

        f.write(f'/// World pickups database ({len(pickups)} entries)\n')
        f.write('pub static WORLD_PICKUPS: Lazy<HashMap<u32, WorldPickup>> = Lazy::new(|| {\n')
        f.write('    let mut m = HashMap::new();\n')

        for p in pickups:
            name_escaped = escape_rust_string(p['item_name'])
            region_escaped = escape_rust_string(p['region'])
            f.write(f'    m.insert({p["lot_id"]}, WorldPickup {{ ')
            f.write(f'flag_id: {p["flag_id"]}, ')
            f.write(f'item_id: {p["item_id"]}, ')
            f.write(f'item_name: "{name_escaped}", ')
            f.write(f'item_type: PickupItemType::{p["item_type"]}, ')
            f.write(f'quantity: {p["quantity"]}, ')
            f.write(f'region: "{region_escaped}", ')
            f.write(f'tile_x: {p["tile_x"]}, ')
            f.write(f'tile_y: {p["tile_y"]} ')
            f.write('});\n')

        f.write('    m\n')
        f.write('});\n')

    print(f"\nGenerated {output_file}")

if __name__ == '__main__':
    main()
