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

    For 10-digit tile-based IDs (1XXYYZZZZ):
    - 1 = base game, 2 = DLC
    - XX = tile X coordinate (33-54 base game)
    - YY = tile Y coordinate (31-58 base game)
    - ZZZZ = local flag index

    For 8-digit dungeon IDs (AASSZZZZ):
    - AA = area code (10-43)
    - SS = section
    - ZZZZ = local flag index

    Returns (region_name, tile_x, tile_y)
    """
    # 10-digit tile-based world pickups (1000000000+)
    if lot_id >= 1_000_000_000 and lot_id < 3_000_000_000:
        prefix = lot_id // 1_000_000_000  # 1 = base, 2 = DLC
        tile_index = (lot_id // 10000) % 10000
        tile_x = tile_index // 100
        tile_y = tile_index % 100

        # Map tile coordinates to regions (approximate based on EVENT-FLAG-GEOGRAPHY.md)
        if prefix == 2:
            return ("Shadow Realm (DLC)", tile_x, tile_y)

        # Base game tile regions
        if 42 <= tile_x <= 44 and 36 <= tile_y <= 40:
            return ("Limgrave", tile_x, tile_y)
        elif 40 <= tile_x <= 43 and 33 <= tile_y <= 35:
            return ("Weeping Peninsula", tile_x, tile_y)
        elif 37 <= tile_x <= 44 and 41 <= tile_y <= 47:
            return ("Liurnia of the Lakes", tile_x, tile_y)
        elif 37 <= tile_x <= 44 and 48 <= tile_y <= 52:
            return ("Altus Plateau", tile_x, tile_y)
        elif 33 <= tile_x <= 38 and 48 <= tile_y <= 52:
            return ("Mt. Gelmir", tile_x, tile_y)
        elif 46 <= tile_x <= 54 and 36 <= tile_y <= 44:
            return ("Caelid", tile_x, tile_y)
        elif 48 <= tile_x <= 54 and 45 <= tile_y <= 50:
            return ("Greyoll's Dragonbarrow", tile_x, tile_y)
        elif 37 <= tile_x <= 44 and 53 <= tile_y <= 58:
            return ("Mountaintops of the Giants", tile_x, tile_y)
        elif 33 <= tile_x <= 38 and 55 <= tile_y <= 58:
            return ("Consecrated Snowfield", tile_x, tile_y)
        else:
            return ("Open World", tile_x, tile_y)

    # 8-digit dungeon flags
    lot_str = str(lot_id)
    if len(lot_str) >= 8:
        area = lot_str[:2]
        section = int(lot_str[2:4]) if len(lot_str) > 4 else 0

        dungeon_map = {
            "10": "Stormveil Castle",
            "11": "Leyndell Royal Capital",
            "12": "Underground",
            "13": "Crumbling Farum Azula",
            "14": "Academy of Raya Lucaria",
            "15": "Miquella's Haligtree",
            "16": "Volcano Manor",
            "18": "Roundtable Hold",
            "20": "Elden Throne",
            "21": "Elden Throne",
            "22": "Mohgwyn Palace",
            "28": "Divine Tower",
            "30": "Catacombs",
            "31": "Caves",
            "32": "Tunnels",
            "34": "Divine Towers",
            "35": "Mohgwyn Palace",
            "39": "Deeproot Depths",
            "40": "Elphael",
            "41": "Haligtree",
            "42": "Gaols",
            "43": "Evergaols",
        }

        region = dungeon_map.get(area, f"Dungeon {area}")
        return (region, 0, section)

    return ("Unknown", 0, 0)

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
        get_item_flag_id = int(row.get('getItemFlagId', 0))

        # CRITICAL DISCOVERY (2026-01-23): For tile-based world pickups, the game stores
        # the ROW ID (lot_id) as the actual event flag, NOT getItemFlagId.
        # getItemFlagId = lot_id + 7000, placing local_id in 7000+ range (unstorable).
        # Save file diff analysis confirmed the game uses lot_id for persistence.
        is_tile_based = 1_000_000_000 <= lot_id < 3_000_000_000
        flag_id = lot_id if is_tile_based else get_item_flag_id

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
