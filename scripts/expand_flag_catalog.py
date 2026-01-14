#!/usr/bin/env python3
"""
Expand Flag Catalog

Extracts additional event flags from:
1. ItemLotParam_map.param.xml - Item pickup flags
2. Event scripts (*.emevd.js) - All flag references
3. ShopLineupParam.param.xml - Shop stock/release flags
4. Map-specific event scripts - Dungeon flags

Merges with existing extracted_event_flags.json
"""

import json
import os
import re
import xml.etree.ElementTree as ET
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Optional, Set, Tuple

# Paths
DECOMPILED_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files")
SCRIPTS_DIR = Path(__file__).parent
CATALOG_PATH = SCRIPTS_DIR / "extracted_event_flags.json"
OUTPUT_PATH = SCRIPTS_DIR / "extracted_event_flags_expanded.json"

# Map IDs to region names
MAP_REGIONS = {
    "m10": "Stormveil Castle",
    "m11": "Raya Lucaria",
    "m12": "Underground",
    "m13": "Leyndell",
    "m14": "Shunning Grounds",
    "m15": "Haligtree",
    "m16": "Farum Azula",
    "m18": "Roundtable Hold",
    "m19": "Ainsel River",
    "m30": "Catacombs",
    "m31": "Caves",
    "m32": "Tunnels",
    "m34": "Divine Tower",
    "m35": "Mohgwyn Palace",
    "m39": "Deeproot Depths",
    "m60": "Overworld",
}

# Flag ID prefixes to dungeon names
DUNGEON_PREFIXES = {
    10: "Stormveil Castle",
    11: "Raya Lucaria",
    12: "Underground Dungeon",
    13: "Leyndell",
    14: "Shunning Grounds",
    15: "Haligtree",
    16: "Farum Azula",
    18: "Roundtable Hold",
    19: "Ainsel River",
    30: "Catacombs",
    31: "Cave",
    32: "Tunnel",
    34: "Divine Tower",
    35: "Mohgwyn Palace",
    39: "Deeproot Depths",
}

# Item category mapping
ITEM_CATEGORIES = {
    1: "Goods",
    2: "Weapon",
    3: "Armor",
    4: "Accessory",
    5: "Ash of War",
}


def load_existing_catalog() -> Dict:
    """Load existing flag catalog"""
    if CATALOG_PATH.exists():
        with open(CATALOG_PATH, 'r', encoding='utf-8') as f:
            return json.load(f)
    return {"metadata": {}, "flags": []}


def extract_flags_from_itemlot() -> List[Dict]:
    """Extract flags from ItemLotParam_map.param.xml"""
    xml_path = DECOMPILED_DIR / "regulation-bin" / "ItemLotParam_map.param.xml"
    if not xml_path.exists():
        print(f"Warning: {xml_path} not found")
        return []

    flags = []
    tree = ET.parse(xml_path)
    root = tree.getroot()

    for row in root.findall('.//row'):
        row_id = row.get('id')
        flag_id = row.get('getItemFlagId')

        if flag_id and int(flag_id) > 0:
            flag_id = int(flag_id)
            item_id = row.get('lotItemId01')
            item_cat = row.get('lotItemCategory01')

            # Determine category and region from flag ID
            category, region = categorize_flag(flag_id)

            flags.append({
                "flag_id": flag_id,
                "name": f"ItemLot {row_id}",
                "category": category,
                "region": region,
                "source_file": "ItemLotParam_map.param.xml",
                "source_row_id": int(row_id) if row_id else None,
                "item_id": int(item_id) if item_id else None,
                "item_category": int(item_cat) if item_cat else None,
            })

    return flags


def extract_flags_from_shop() -> List[Dict]:
    """Extract flags from ShopLineupParam.param.xml"""
    xml_path = DECOMPILED_DIR / "regulation-bin" / "ShopLineupParam.param.xml"
    if not xml_path.exists():
        print(f"Warning: {xml_path} not found")
        return []

    flags = []
    tree = ET.parse(xml_path)
    root = tree.getroot()

    for row in root.findall('.//row'):
        row_id = row.get('id')
        stock_flag = row.get('eventFlag_forStock')
        release_flag = row.get('eventFlag_forRelease')
        equip_id = row.get('equipId')

        if stock_flag and int(stock_flag) > 0:
            flags.append({
                "flag_id": int(stock_flag),
                "name": f"Shop Stock {row_id}",
                "category": "Shop Stock",
                "region": "Unknown",
                "source_file": "ShopLineupParam.param.xml",
                "source_row_id": int(row_id) if row_id else None,
                "item_id": int(equip_id) if equip_id else None,
            })

        if release_flag and int(release_flag) > 0:
            flags.append({
                "flag_id": int(release_flag),
                "name": f"Shop Release {row_id}",
                "category": "Shop Unlock",
                "region": "Unknown",
                "source_file": "ShopLineupParam.param.xml",
                "source_row_id": int(row_id) if row_id else None,
            })

    return flags


def extract_flags_from_event_scripts() -> List[Dict]:
    """Extract all flag references from event scripts"""
    event_dir = DECOMPILED_DIR / "event"
    if not event_dir.exists():
        print(f"Warning: {event_dir} not found")
        return []

    flags = []
    seen_flags: Set[int] = set()

    # Pattern to match flag IDs in various contexts
    # EventFlag(12345), SetEventFlag(12345, ...), flag IDs as parameters
    flag_patterns = [
        r'EventFlag\((\d+)\)',
        r'SetEventFlag\((\d+)',
        r'GetEventFlagValue\((\d+)',
        r'ClearEventFlag\((\d+)',
        r', (\d{7,10})[,\)]',  # 7-10 digit numbers as parameters
    ]

    for js_file in event_dir.glob("*.emevd.js"):
        map_id = js_file.stem.replace(".emevd", "")
        region = get_region_from_map(map_id)

        with open(js_file, 'r', encoding='utf-8') as f:
            content = f.read()

        for pattern in flag_patterns:
            for match in re.finditer(pattern, content):
                try:
                    flag_id = int(match.group(1))

                    # Only interested in significant flag IDs
                    if flag_id < 100 or flag_id in seen_flags:
                        continue

                    # Skip obviously non-flag numbers (entity IDs, etc)
                    if flag_id > 3000000000:
                        continue

                    seen_flags.add(flag_id)
                    category, flag_region = categorize_flag(flag_id)

                    # Use map region if we couldn't determine from flag ID
                    if flag_region == "Unknown" and region != "Unknown":
                        flag_region = region

                    flags.append({
                        "flag_id": flag_id,
                        "name": f"Event Flag {flag_id}",
                        "category": category,
                        "region": flag_region,
                        "source_file": js_file.name,
                    })
                except (ValueError, IndexError):
                    continue

    return flags


def categorize_flag(flag_id: int) -> Tuple[str, str]:
    """Determine category and region from flag ID pattern"""

    # Simple flags (0-59999)
    if flag_id < 60000:
        return "Simple Flag", "Unknown"

    # Block flags (60000-99999)
    if 60000 <= flag_id < 100000:
        if 60000 <= flag_id < 61000:
            return "Progression", "Unknown"
        if 62000 <= flag_id < 63000:
            return "Map Fragment", "Unknown"
        if 65000 <= flag_id < 66000:
            return "Whetblade", "Unknown"
        if 66000 <= flag_id < 67000:
            return "Pot Upgrade", "Unknown"
        if 67000 <= flag_id < 69000:
            return "Cookbook", "Unknown"
        if 69000 <= flag_id < 70000:
            return "Mausoleum Duplication", "Unknown"
        if 76000 <= flag_id < 77000:
            return "Grace", "Unknown"
        return "Block Flag", "Unknown"

    # Dungeon/Legacy flags (10000000-44999999)
    if 10000000 <= flag_id < 45000000:
        prefix = flag_id // 1000000
        region = DUNGEON_PREFIXES.get(prefix, "Legacy Dungeon")

        # Determine category from sub-range
        sub_id = flag_id % 1000000
        if sub_id < 1000:
            return "Boss Defeat", region
        if 2000 <= sub_id < 3000:
            return "NPC Event", region
        if 7000 <= sub_id < 8000:
            return "Dungeon Pickup", region
        if 50000 <= sub_id < 60000:
            return "Dungeon State", region

        return "Dungeon Event", region

    # Tile/World flags (1000000000+)
    if flag_id >= 1000000000:
        # Extract map coordinates
        if flag_id < 2000000000:
            return "World Pickup", "Overworld"
        else:
            return "DLC Pickup", "Shadow Realm"

    return "Unknown", "Unknown"


def get_region_from_map(map_id: str) -> str:
    """Get region name from map ID"""
    if map_id == "common" or map_id == "common_func":
        return "Global"

    # Extract map prefix (e.g., m10 from m10_00_00_00)
    match = re.match(r'm(\d+)', map_id)
    if match:
        prefix = f"m{match.group(1)}"
        return MAP_REGIONS.get(prefix, "Unknown")

    return "Unknown"


def merge_flags(existing: List[Dict], new_flags: List[Dict]) -> List[Dict]:
    """Merge new flags with existing, avoiding duplicates"""
    existing_ids = {f["flag_id"] for f in existing}

    merged = list(existing)
    added = 0

    for flag in new_flags:
        if flag["flag_id"] not in existing_ids:
            # Fill in missing fields with defaults
            full_flag = {
                "flag_id": flag["flag_id"],
                "name": flag.get("name", f"Flag {flag['flag_id']}"),
                "category": flag.get("category", "Unknown"),
                "region": flag.get("region", "Unknown"),
                "source_file": flag.get("source_file"),
                "source_row_id": flag.get("source_row_id"),
                "item_id": flag.get("item_id"),
                "item_category": flag.get("item_category"),
                "area_no": None,
                "grid_x": None,
                "grid_z": None,
                "pos_x": None,
                "pos_y": None,
                "pos_z": None,
                "map_tile": None,
                "region_id": None,
                "is_overworld": None,
                "world_x": None,
                "world_z": None,
                "area_type": None,
                "is_dlc": flag.get("is_dlc", False),
                "treasure_type": None,
                "item_rarity": None,
                "position_confidence": None,
                "is_underground": None,
                "raw_data": None,
            }
            merged.append(full_flag)
            existing_ids.add(flag["flag_id"])
            added += 1

    print(f"Added {added} new flags")
    return merged


def update_category_counts(flags: List[Dict]) -> Dict[str, int]:
    """Count flags by category"""
    counts = defaultdict(int)
    for flag in flags:
        counts[flag.get("category", "Unknown")] += 1
    return dict(sorted(counts.items(), key=lambda x: -x[1]))


def main():
    print("Loading existing catalog...")
    catalog = load_existing_catalog()
    existing_flags = catalog.get("flags", [])
    print(f"Existing flags: {len(existing_flags)}")

    print("\nExtracting from ItemLotParam_map.param.xml...")
    itemlot_flags = extract_flags_from_itemlot()
    print(f"Found {len(itemlot_flags)} ItemLot flags")

    print("\nExtracting from ShopLineupParam.param.xml...")
    shop_flags = extract_flags_from_shop()
    print(f"Found {len(shop_flags)} Shop flags")

    print("\nExtracting from event scripts...")
    event_flags = extract_flags_from_event_scripts()
    print(f"Found {len(event_flags)} event flags")

    print("\nMerging flags...")
    all_new = itemlot_flags + shop_flags + event_flags
    merged_flags = merge_flags(existing_flags, all_new)

    # Sort by flag_id
    merged_flags.sort(key=lambda x: x["flag_id"])

    # Update metadata
    metadata = catalog.get("metadata", {})
    metadata["total_flags"] = len(merged_flags)
    metadata["category_counts"] = update_category_counts(merged_flags)
    metadata["sources"] = list(set(metadata.get("sources", []) + [
        "ItemLotParam_map.param.xml",
        "ShopLineupParam.param.xml",
        "Event scripts (*.emevd.js)",
    ]))

    # Save expanded catalog
    output = {
        "metadata": metadata,
        "flags": merged_flags,
    }

    with open(OUTPUT_PATH, 'w', encoding='utf-8') as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\nSaved expanded catalog to {OUTPUT_PATH}")
    print(f"Total flags: {len(merged_flags)}")
    print("\nTop categories:")
    for cat, count in list(metadata["category_counts"].items())[:15]:
        print(f"  {cat}: {count}")


if __name__ == "__main__":
    main()
