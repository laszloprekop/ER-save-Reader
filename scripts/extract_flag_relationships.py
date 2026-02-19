#!/usr/bin/env python3
"""
Extract flag relationships from decompiled Elden Ring game files.

Maps connections between flags for the same item/event across:
- ItemLotParam_map (world pickups)
- ShopLineupParam (shop purchases)
- BonfireWarpParam (grace discovery)
- common.emevd.js (event script relationships)

Output: JSON graph of flag relationships for multi-point verification.
"""

import json
import re
import xml.etree.ElementTree as ET
from pathlib import Path
from collections import defaultdict
from dataclasses import dataclass, asdict
from typing import List, Dict, Optional, Set

# Path to decompiled game files
GAME_FILES_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files")
REG_BIN = GAME_FILES_DIR / "regulation-bin"
EVENT_DIR = GAME_FILES_DIR / "event"

@dataclass
class FlagRelationship:
    """A relationship between two flags"""
    source_flag: int
    target_flag: int
    relationship_type: str  # "enables", "triggered_by", "same_item", "progression"
    source_file: str
    item_name: Optional[str] = None
    notes: Optional[str] = None

# Pickup edges where the tile-side flag (source) is the row_id position which
# the game never writes to. The actual tracking flag is a pre-assigned block flag
# (target). Corroboration should not compare these two positions.
SKIP_CORROBORATION_PAIRS: Set[tuple] = {
    (1034480200, 62022),   # Map: Liurnia, West
    (1034500100, 60430),   # Memory Stone
    (1035470100, 60420),   # Memory Stone
    (1036540500, 62032),   # Map: Mt. Gelmir
    (1037440210, 62021),   # Map: Liurnia, North
    (1037460001, 60150),   # Golden Tailoring Tools
    (1038410200, 62020),   # Map: Liurnia, East
    (1040520500, 62030),   # Map: Altus Plateau
    (1042370200, 62010),   # Map: Limgrave, West
    (1042371010, 60130),   # Whetstone Knife
    (1042510500, 62031),   # Map: Leyndell, Royal Capital
    (1044320000, 62011),   # Map: Weeping Peninsula
    (1045370020, 62012),   # Map: Limgrave, East
    (1046380300, 60020),   # Flask of Wondrous Physick
    (1049370500, 62040),   # Map: Caelid
    (1049400500, 62041),   # Map: Dragonbarrow
}

@dataclass
class FlagNode:
    """A flag with its relationships and metadata"""
    flag_id: int
    name: str
    category: str
    source_file: str
    related_flags: List[int]
    item_id: Optional[int] = None
    relationships: List[dict] = None

def parse_xml_param(filepath: Path) -> List[dict]:
    """Parse a param XML file and return rows as dicts"""
    try:
        tree = ET.parse(filepath)
        root = tree.getroot()
        rows = []
        for row in root.findall('.//row'):
            rows.append(row.attrib)
        return rows
    except Exception as e:
        print(f"Error parsing {filepath}: {e}")
        return []

def extract_itemlot_relationships() -> List[FlagRelationship]:
    """Extract flag relationships from ItemLotParam_map"""
    relationships = []
    filepath = REG_BIN / "ItemLotParam_map.param.xml"

    rows = parse_xml_param(filepath)
    for row in rows:
        row_id = int(row.get('id', 0))
        main_flag = int(row.get('getItemFlagId', 0))

        if main_flag == 0:
            continue

        # Check for additional flags
        for i in range(1, 9):
            additional_flag = int(row.get(f'getItemFlagId0{i}', 0) or 0)
            if additional_flag != 0 and additional_flag != main_flag:
                relationships.append(FlagRelationship(
                    source_flag=main_flag,
                    target_flag=additional_flag,
                    relationship_type="same_pickup",
                    source_file="ItemLotParam_map",
                    notes=f"Row {row_id}: Both flags set on same pickup"
                ))

        # Row ID is the world location - might be related to tile flags
        if row_id >= 1000000000:
            # 10-digit world pickup ID
            relationships.append(FlagRelationship(
                source_flag=row_id,  # World location flag
                target_flag=main_flag,  # Item obtained flag
                relationship_type="pickup_sets_flag",
                source_file="ItemLotParam_map",
                notes=f"Picking up at {row_id} sets flag {main_flag}"
            ))

    return relationships

def extract_shop_relationships() -> List[FlagRelationship]:
    """Extract flag relationships from ShopLineupParam"""
    relationships = []
    filepath = REG_BIN / "ShopLineupParam.param.xml"

    rows = parse_xml_param(filepath)
    for row in rows:
        stock_flag = int(row.get('eventFlag_forStock', 0))
        release_flag = int(row.get('eventFlag_forRelease', 0))
        item_name = row.get('paramdexName', '')

        if stock_flag != 0 and release_flag != 0:
            relationships.append(FlagRelationship(
                source_flag=release_flag,
                target_flag=stock_flag,
                relationship_type="enables_purchase",
                source_file="ShopLineupParam",
                item_name=item_name,
                notes=f"Flag {release_flag} enables item, {stock_flag} tracks purchase"
            ))

    return relationships

def extract_grace_relationships() -> List[FlagRelationship]:
    """Extract flag relationships from BonfireWarpParam"""
    relationships = []
    filepath = REG_BIN / "BonfireWarpParam.param.xml"

    rows = parse_xml_param(filepath)
    for row in rows:
        flag_id = int(row.get('eventflagId', 0))
        entity_id = int(row.get('bonfireEntityId', 0))
        name = row.get('paramdexName', '')

        if flag_id != 0:
            # Grace discovery flag relates to entity
            relationships.append(FlagRelationship(
                source_flag=entity_id,
                target_flag=flag_id,
                relationship_type="grace_discovery",
                source_file="BonfireWarpParam",
                item_name=name,
                notes=f"Entity {entity_id} -> Discovery flag {flag_id}"
            ))

    return relationships

def extract_event_script_relationships() -> List[FlagRelationship]:
    """Extract flag relationships from common.emevd.js"""
    relationships = []
    filepath = EVENT_DIR / "common.emevd.js"

    if not filepath.exists():
        print(f"Event script not found: {filepath}")
        return relationships

    content = filepath.read_text(encoding='utf-8', errors='ignore')

    # Pattern: $InitializeEvent(index, event_id, flag1, flag2, flag3, flag4)
    init_pattern = r'\$InitializeEvent\(\d+,\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+)\)'

    for match in re.finditer(init_pattern, content):
        event_id = int(match.group(1))
        params = [int(match.group(i)) for i in range(2, 6)]

        # Event 1100: Boss remembrance
        if event_id == 1100:
            event_flag, item_lot1, item_lot2, event_flag2 = params
            if event_flag != 0 and event_flag2 != 0:
                relationships.append(FlagRelationship(
                    source_flag=event_flag,
                    target_flag=event_flag2,
                    relationship_type="boss_remembrance",
                    source_file="common.emevd.js",
                    notes=f"Event 1100: Remembrance flag {event_flag} linked to {event_flag2}"
                ))

        # Event 1600: Map fragments
        elif event_id == 1600:
            # Map fragment events link discovery to possession
            if params[0] != 0 and params[1] != 0:
                relationships.append(FlagRelationship(
                    source_flag=params[0],
                    target_flag=params[1],
                    relationship_type="map_fragment",
                    source_file="common.emevd.js",
                    notes=f"Event 1600: Map fragment relationship"
                ))

    # Pattern: SetEventFlagID sequences
    set_flag_pattern = r'SetEventFlagID\((\d+),\s*(ON|OFF)\)'

    # Find events that set multiple flags
    event_blocks = re.findall(r'\$Event\(\d+[^}]+\}', content, re.DOTALL)
    for block in event_blocks[:100]:  # Limit for performance
        flags_set = re.findall(set_flag_pattern, block)
        if len(flags_set) > 1:
            flag_ids = [int(f[0]) for f in flags_set if f[1] == 'ON']
            # Link consecutive flags as related
            for i in range(len(flag_ids) - 1):
                if flag_ids[i] != flag_ids[i+1]:
                    relationships.append(FlagRelationship(
                        source_flag=flag_ids[i],
                        target_flag=flag_ids[i+1],
                        relationship_type="event_sequence",
                        source_file="common.emevd.js",
                        notes="Set in same event block"
                    ))

    return relationships

def extract_emevd_enemy_item_relationships() -> List[FlagRelationship]:
    """Extract enemy defeat → item acquisition relationships from map EMEVD files."""
    relationships = []

    # Load ItemLotParam row_id → getItemFlagId mapping
    itemlot_flags = {}
    rows = parse_xml_param(REG_BIN / "ItemLotParam_map.param.xml")
    for row in rows:
        row_id = int(row.get('id', 0))
        flag_id = int(row.get('getItemFlagId', 0))
        if flag_id > 0:
            itemlot_flags[row_id] = flag_id

    # Templates that link enemy entities to item lots
    TEMPLATES_WITH_ITEMS = {
        90005300: {"flag_idx": 0, "item_lot_idx": 2},
        90005301: {"flag_idx": 0, "item_lot_idx": 2},
        90005390: {"flag_idx": 0, "item_lot_idx": 5},
        90005861: {"flag_idx": 0, "item_lot_idx": 4},
        90005880: {"flag_idx": 0, "item_lot_idx": 4},
        90005555: {"flag_idx": 0, "item_lot_idx": 1},
    }

    pattern = re.compile(
        r'\$InitializeCommonEvent\(\s*\d+\s*,\s*(\d+)\s*,\s*([^)]+)\)'
    )

    for js_file in sorted(EVENT_DIR.glob("m*.emevd.js")):
        try:
            content = js_file.read_text(encoding='utf-8', errors='ignore')
        except Exception:
            continue

        for match in pattern.finditer(content):
            template_id = int(match.group(1))
            if template_id not in TEMPLATES_WITH_ITEMS:
                continue

            tmpl = TEMPLATES_WITH_ITEMS[template_id]
            params = []
            for p in match.group(2).split(','):
                p = p.strip()
                try:
                    params.append(int(p))
                except ValueError:
                    params.append(0)

            flag_idx = tmpl["flag_idx"]
            ilt_idx = tmpl["item_lot_idx"]
            if flag_idx >= len(params) or ilt_idx >= len(params):
                continue

            defeat_flag = params[flag_idx]
            item_lot_id = params[ilt_idx]
            if defeat_flag <= 0 or item_lot_id <= 0:
                continue

            item_flag = itemlot_flags.get(item_lot_id)
            if item_flag and item_flag != defeat_flag:
                relationships.append(FlagRelationship(
                    source_flag=defeat_flag,
                    target_flag=item_flag,
                    relationship_type="enemy_drops_item",
                    source_file=js_file.name,
                    notes=f"Enemy defeat {defeat_flag} drops item lot {item_lot_id} -> flag {item_flag}"
                ))

    return relationships


def build_flag_graph(relationships: List[FlagRelationship]) -> Dict:
    """Build a graph representation of flag relationships"""
    graph = {
        "nodes": {},
        "edges": [],
        "by_type": defaultdict(list),
        "statistics": {}
    }

    all_flags = set()
    for rel in relationships:
        all_flags.add(rel.source_flag)
        all_flags.add(rel.target_flag)

        edge = {
            "source": rel.source_flag,
            "target": rel.target_flag,
            "type": rel.relationship_type,
            "file": rel.source_file,
            "item": rel.item_name,
            "notes": rel.notes
        }
        if (rel.source_flag, rel.target_flag) in SKIP_CORROBORATION_PAIRS:
            edge["skip_corroboration"] = True
        graph["edges"].append(edge)
        graph["by_type"][rel.relationship_type].append(edge)

    # Create nodes
    for flag_id in all_flags:
        graph["nodes"][flag_id] = {
            "id": flag_id,
            "connections": len([e for e in graph["edges"]
                              if e["source"] == flag_id or e["target"] == flag_id])
        }

    # Statistics
    graph["statistics"] = {
        "total_flags": len(all_flags),
        "total_relationships": len(relationships),
        "relationship_types": {k: len(v) for k, v in graph["by_type"].items()}
    }

    return graph

def find_related_flags(graph: Dict, flag_id: int, depth: int = 2) -> Set[int]:
    """Find all flags related to a given flag within N hops"""
    related = {flag_id}
    frontier = {flag_id}

    for _ in range(depth):
        new_frontier = set()
        for edge in graph["edges"]:
            if edge["source"] in frontier:
                new_frontier.add(edge["target"])
            if edge["target"] in frontier:
                new_frontier.add(edge["source"])
        frontier = new_frontier - related
        related.update(frontier)

    return related

def main():
    print("Extracting flag relationships from decompiled game files...")

    all_relationships = []

    print("\n1. Processing ItemLotParam_map...")
    itemlot_rels = extract_itemlot_relationships()
    all_relationships.extend(itemlot_rels)
    print(f"   Found {len(itemlot_rels)} relationships")

    print("\n2. Processing ShopLineupParam...")
    shop_rels = extract_shop_relationships()
    all_relationships.extend(shop_rels)
    print(f"   Found {len(shop_rels)} relationships")

    print("\n3. Processing BonfireWarpParam...")
    grace_rels = extract_grace_relationships()
    all_relationships.extend(grace_rels)
    print(f"   Found {len(grace_rels)} relationships")

    print("\n4. Processing common.emevd.js...")
    event_rels = extract_event_script_relationships()
    all_relationships.extend(event_rels)
    print(f"   Found {len(event_rels)} relationships")

    print("\n5. Processing map EMEVD files (enemy→item relationships)...")
    emevd_item_rels = extract_emevd_enemy_item_relationships()
    all_relationships.extend(emevd_item_rels)
    print(f"   Found {len(emevd_item_rels)} relationships")

    print("\n6. Building flag relationship graph...")
    graph = build_flag_graph(all_relationships)

    print(f"\n=== Statistics ===")
    print(f"Total unique flags: {graph['statistics']['total_flags']}")
    print(f"Total relationships: {graph['statistics']['total_relationships']}")
    print(f"\nRelationship types:")
    for rtype, count in graph['statistics']['relationship_types'].items():
        print(f"  - {rtype}: {count}")

    # Save to JSON
    output_path = Path(__file__).parent / "flag_relationships.json"
    with open(output_path, 'w') as f:
        # Convert defaultdict to dict for JSON serialization
        graph["by_type"] = dict(graph["by_type"])
        json.dump(graph, f, indent=2)
    print(f"\nSaved to: {output_path}")

    # Example: Find all flags related to a cookbook flag
    print("\n=== Example: Flags related to 67650 (Missionary's Cookbook [3]) ===")
    related = find_related_flags(graph, 67650)
    print(f"Related flags: {sorted(related)}")

    # Show specific relationships for this flag
    for edge in graph["edges"]:
        if edge["source"] == 67650 or edge["target"] == 67650:
            print(f"  {edge['source']} --[{edge['type']}]--> {edge['target']}")
            if edge['notes']:
                print(f"    Notes: {edge['notes']}")

if __name__ == "__main__":
    main()
