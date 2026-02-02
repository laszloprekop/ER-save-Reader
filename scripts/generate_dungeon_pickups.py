#!/usr/bin/env python3
"""
Generate complete dungeon_pickups.rs from extracted_event_flags.json and ItemLotParam_map.

This script:
1. Uses extracted_event_flags.json as primary source (has positions, names, MSB backing)
2. Falls back to ItemLotParam_map for any missing entries (items without MSB position)
3. Cross-references with item name databases for readable names
4. Generates complete Rust code for src/db/dungeon_pickups.rs

Usage:
    python scripts/generate_dungeon_pickups.py > src/db/dungeon_pickups.rs
    # Or to just analyze:
    python scripts/generate_dungeon_pickups.py --analyze
"""

import json
import xml.etree.ElementTree as ET
from pathlib import Path
from collections import defaultdict
from dataclasses import dataclass
from typing import Optional
import argparse
import sys

# Paths
EXTRACTED_FLAGS = Path("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/public/data/extracted_event_flags.json")
ITEMLOT_PARAM = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/regulation-bin/ItemLotParam_map.param.xml")
EQUIP_PARAM_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/regulation-bin")

# Item category mapping (game's lotItemCategory values)
# 0 = None, 1 = Goods, 2 = Weapons, 3 = Protector (Armor), 4 = Accessory (Talisman), 5 = Gem (Ash of War)
ITEM_CATEGORY_MAP = {
    0: "Other",
    1: "Consumables",  # Goods - includes golden runes, smithing stones, etc.
    2: "Weapons",
    3: "Armor",
    4: "Talismans",
    5: "AshesOfWar",
}

# Dungeon area names
DUNGEON_AREA_NAMES = {
    10: "Stormveil Castle",
    11: "Leyndell",
    12: "Underground",
    13: "Crumbling Farum Azula",
    14: "Academy of Raya Lucaria",
    15: "Haligtree",
    16: "Volcano Manor",
    18: "Roundtable Hold",
    20: "Stranded Graveyard",
    21: "Haligtree (Elphael)",
    22: "Castle Sol",
    28: "DLC Dungeon",
    30: "Catacombs",
    31: "Caves",
    32: "Tunnels",
    34: "Divine Towers",
    35: "Mohgwyn Palace",
    39: "Elden Throne",
    40: "Hero's Graves",
    41: "Minor Dungeons",
    42: "Crystal Caves",
    43: "Evergaols",
    59: "Unknown 59",
    99: "Special",
}

# Item ID ranges for category refinement
GOLDEN_RUNE_RANGE = (2900, 2930)  # Golden Rune [1] through Golden Rune [13], Lord's Rune, etc.
SMITHING_STONE_RANGE = (10100, 10199)  # Smithing Stone [1] through [8]
SOMBER_STONE_RANGE = (10800, 10899)  # Somber Smithing Stone [1] through [9]
GLOVEWORT_RANGE = (10200, 10299)  # Grave Glovewort [1] through [9]
GHOST_GLOVEWORT_RANGE = (10700, 10799)  # Ghost Glovewort [1] through [9]


@dataclass
class DungeonPickup:
    item_lot_id: int
    event_flag: int
    item_id: int
    name: str
    quantity: int
    category: str
    region: str
    dungeon_area: int
    section: int
    has_position: bool = True  # Whether we have MSB position data


def refine_category(item_id: int, base_category: str) -> str:
    """Refine category based on specific item ID ranges."""
    if base_category == "Consumables":
        # Check for specific consumable types
        if GOLDEN_RUNE_RANGE[0] <= item_id <= GOLDEN_RUNE_RANGE[1]:
            return "GoldenRunes"
        elif SMITHING_STONE_RANGE[0] <= item_id <= SMITHING_STONE_RANGE[1]:
            return "SmithingStones"
        elif SOMBER_STONE_RANGE[0] <= item_id <= SOMBER_STONE_RANGE[1]:
            return "SomberStones"
        elif GLOVEWORT_RANGE[0] <= item_id <= GLOVEWORT_RANGE[1]:
            return "Glovewort"
        elif GHOST_GLOVEWORT_RANGE[0] <= item_id <= GHOST_GLOVEWORT_RANGE[1]:
            return "Glovewort"
    return base_category


def load_item_names() -> dict[tuple[int, int], str]:
    """Load item names from various param files.

    Returns a dict keyed by (item_category, item_id) to handle overlapping IDs.
    Game categories: 0=None, 1=Goods, 2=Weapons, 3=Protector, 4=Accessory, 5=Gem
    """
    names = {}

    # Load goods (consumables, key items, etc.) - category 1
    goods_path = EQUIP_PARAM_DIR / "EquipParamGoods.param.xml"
    if goods_path.exists():
        tree = ET.parse(goods_path)
        for row in tree.findall('.//row'):
            row_id = int(row.get('id', 0))
            name = row.get('paramdexName') or row.get('name') or f"Goods {row_id}"
            names[(1, row_id)] = name

    # Load weapons - category 2
    weapon_path = EQUIP_PARAM_DIR / "EquipParamWeapon.param.xml"
    if weapon_path.exists():
        tree = ET.parse(weapon_path)
        for row in tree.findall('.//row'):
            row_id = int(row.get('id', 0))
            name = row.get('paramdexName') or row.get('name') or f"Weapon {row_id}"
            names[(2, row_id)] = name

    # Load armor (protector) - category 3
    armor_path = EQUIP_PARAM_DIR / "EquipParamProtector.param.xml"
    if armor_path.exists():
        tree = ET.parse(armor_path)
        for row in tree.findall('.//row'):
            row_id = int(row.get('id', 0))
            name = row.get('paramdexName') or row.get('name') or f"Armor {row_id}"
            names[(3, row_id)] = name

    # Load accessories (talismans) - category 4
    accessory_path = EQUIP_PARAM_DIR / "EquipParamAccessory.param.xml"
    if accessory_path.exists():
        tree = ET.parse(accessory_path)
        for row in tree.findall('.//row'):
            row_id = int(row.get('id', 0))
            name = row.get('paramdexName') or row.get('name') or f"Talisman {row_id}"
            names[(4, row_id)] = name

    # Load ashes of war (gems) - category 5
    gem_path = EQUIP_PARAM_DIR / "EquipParamGem.param.xml"
    if gem_path.exists():
        tree = ET.parse(gem_path)
        for row in tree.findall('.//row'):
            row_id = int(row.get('id', 0))
            name = row.get('paramdexName') or row.get('name') or f"Ash of War {row_id}"
            names[(5, row_id)] = name

    return names


def load_extracted_flags() -> dict[int, dict]:
    """Load dungeon pickups from extracted_event_flags.json."""
    with open(EXTRACTED_FLAGS, 'r') as f:
        data = json.load(f)

    pickups = {}
    for flag in data['flags']:
        if flag.get('category') == 'Dungeon Pickup':
            row_id = flag.get('source_row_id')
            if row_id:
                pickups[row_id] = flag

    return pickups


def load_itemlot_param() -> dict[int, dict]:
    """Load all dungeon entries from ItemLotParam_map."""
    tree = ET.parse(ITEMLOT_PARAM)
    root = tree.getroot()

    entries = {}
    for row in root.findall('.//row'):
        row_id = int(row.get('id', 0))
        # Dungeon range: 10000000 - 99999999
        if 10000000 <= row_id < 100000000:
            item_id = row.get('lotItemId01')
            if item_id and int(item_id) > 0:
                entries[row_id] = {
                    'row_id': row_id,
                    'item_id': int(item_id),
                    'item_category': int(row.get('lotItemCategory01', 0)),
                    'quantity': int(row.get('lotItemNum01', 1)),
                    'rarity': int(row.get('lotItem_Rarity', -1)),
                }

    return entries


def generate_pickups(item_names: dict[tuple[int, int], str]) -> list[DungeonPickup]:
    """Generate complete list of dungeon pickups."""
    extracted = load_extracted_flags()
    itemlot = load_itemlot_param()

    pickups = []

    # Process all ItemLotParam entries
    for row_id, lot_data in sorted(itemlot.items()):
        area = row_id // 1000000
        section = (row_id // 10000) % 100
        event_flag = row_id + 7000  # Standard derivation

        item_id = lot_data['item_id']
        quantity = lot_data['quantity']
        game_category = lot_data['item_category']

        # Get category
        base_category = ITEM_CATEGORY_MAP.get(game_category, "Other")
        category = refine_category(item_id, base_category)

        # Get name and region from extracted if available
        if row_id in extracted:
            ext = extracted[row_id]
            name = ext.get('name', f"Unknown Item {item_id}")
            region = ext.get('region', DUNGEON_AREA_NAMES.get(area, f"Area {area}"))
            has_position = True
            # Use quantity from raw_data if available
            raw = ext.get('raw_data', {})
            if raw.get('lotItemNum01'):
                quantity = raw['lotItemNum01']
        else:
            # Fallback: use item_names database keyed by (category, item_id)
            name = item_names.get((game_category, item_id), f"Unknown Item {item_id}")
            region = DUNGEON_AREA_NAMES.get(area, f"Area {area}")
            has_position = False

        pickups.append(DungeonPickup(
            item_lot_id=row_id,
            event_flag=event_flag,
            item_id=item_id,
            name=name,
            quantity=quantity,
            category=category,
            region=region,
            dungeon_area=area,
            section=section,
            has_position=has_position,
        ))

    return pickups


def analyze_coverage(pickups: list[DungeonPickup]):
    """Print analysis of pickup coverage."""
    print("=" * 70, file=sys.stderr)
    print("DUNGEON PICKUP ANALYSIS", file=sys.stderr)
    print("=" * 70, file=sys.stderr)

    # Count by area
    area_counts = defaultdict(lambda: {'total': 0, 'with_pos': 0})
    category_counts = defaultdict(int)

    for p in pickups:
        area_counts[p.dungeon_area]['total'] += 1
        if p.has_position:
            area_counts[p.dungeon_area]['with_pos'] += 1
        category_counts[p.category] += 1

    print("\nBy Dungeon Area:", file=sys.stderr)
    print(f"{'Area':<5} {'Name':<30} {'Total':>6} {'w/Pos':>6} {'Missing':>8}", file=sys.stderr)
    print("-" * 60, file=sys.stderr)
    for area in sorted(area_counts.keys()):
        counts = area_counts[area]
        name = DUNGEON_AREA_NAMES.get(area, f"Area {area}")
        missing = counts['total'] - counts['with_pos']
        print(f"{area:<5} {name:<30} {counts['total']:>6} {counts['with_pos']:>6} {missing:>8}", file=sys.stderr)

    print("\nBy Category:", file=sys.stderr)
    for cat, count in sorted(category_counts.items(), key=lambda x: -x[1]):
        print(f"  {cat}: {count}", file=sys.stderr)

    total = len(pickups)
    with_pos = sum(1 for p in pickups if p.has_position)
    print(f"\nTotal: {total} pickups ({with_pos} with position, {total - with_pos} without)", file=sys.stderr)
    print("=" * 70, file=sys.stderr)


def escape_rust_string(s: str) -> str:
    """Escape a string for Rust."""
    return s.replace('\\', '\\\\').replace('"', '\\"')


def generate_rust_code(pickups: list[DungeonPickup]) -> str:
    """Generate Rust code for dungeon_pickups.rs."""
    lines = []

    # Header
    lines.append('//! Dungeon Pickup Database')
    lines.append('//!')
    lines.append(f'//! Generated from extracted_event_flags.json and ItemLotParam_map with {len(pickups)} dungeon pickups.')
    lines.append('//! Each pickup maps an event flag (local_id >= 7000) to an item in a dungeon.')
    lines.append('//!')
    lines.append('//! Auto-generated by scripts/generate_dungeon_pickups.py - do not edit manually.')
    lines.append('')
    lines.append('use crate::db::pickup_data::PickupCategory;')
    lines.append('')

    # Struct definition
    lines.append('/// A dungeon pickup entry')
    lines.append('#[derive(Debug, Clone)]')
    lines.append('pub struct DungeonPickup {')
    lines.append('    pub item_lot_id: u32,')
    lines.append('    pub event_flag: u32,')
    lines.append('    pub item_id: u32,')
    lines.append('    pub name: &\'static str,')
    lines.append('    pub quantity: u32,')
    lines.append('    pub category: PickupCategory,')
    lines.append('    pub region: &\'static str,')
    lines.append('    pub dungeon_area: u32,')
    lines.append('    pub section: u32,')
    lines.append('}')
    lines.append('')

    # Area name function
    lines.append('/// Dungeon area names for display')
    lines.append('pub fn get_dungeon_area_name(area: u32) -> &\'static str {')
    lines.append('    match area {')
    for area, name in sorted(DUNGEON_AREA_NAMES.items()):
        lines.append(f'        {area} => "{escape_rust_string(name)}",')
    lines.append('        _ => "Unknown",')
    lines.append('    }')
    lines.append('}')
    lines.append('')

    # Static array
    lines.append(f'/// All dungeon pickups ({len(pickups)} entries)')
    lines.append('pub static DUNGEON_PICKUPS: &[DungeonPickup] = &[')

    for p in pickups:
        lines.append('    DungeonPickup {')
        lines.append(f'        item_lot_id: {p.item_lot_id},')
        lines.append(f'        event_flag: {p.event_flag},')
        lines.append(f'        item_id: {p.item_id},')
        lines.append(f'        name: "{escape_rust_string(p.name)}",')
        lines.append(f'        quantity: {p.quantity},')
        lines.append(f'        category: PickupCategory::{p.category},')
        lines.append(f'        region: "{escape_rust_string(p.region)}",')
        lines.append(f'        dungeon_area: {p.dungeon_area},')
        lines.append(f'        section: {p.section},')
        lines.append('    },')

    lines.append('];')

    return '\n'.join(lines)


def main():
    parser = argparse.ArgumentParser(description='Generate dungeon_pickups.rs')
    parser.add_argument('--analyze', action='store_true', help='Only analyze, do not generate code')
    parser.add_argument('--output', '-o', type=str, help='Output file (default: stdout)')
    args = parser.parse_args()

    print("Loading item names...", file=sys.stderr)
    item_names = load_item_names()
    print(f"  Loaded {len(item_names)} item names", file=sys.stderr)

    print("Generating pickups...", file=sys.stderr)
    pickups = generate_pickups(item_names)
    print(f"  Generated {len(pickups)} pickups", file=sys.stderr)

    analyze_coverage(pickups)

    if not args.analyze:
        code = generate_rust_code(pickups)
        if args.output:
            with open(args.output, 'w') as f:
                f.write(code)
            print(f"\nWrote {len(pickups)} entries to {args.output}", file=sys.stderr)
        else:
            print(code)


if __name__ == '__main__':
    main()
