#!/usr/bin/env python3
"""
Export Anchors Script

Merges multiple data sources into anchor_database.json for chain-based verification.

Data Sources:
- event_graph.json: flag dependencies, enables, progression chains
- ground_truth_offsets.json: calibration anchors
- chain_data.rs: boss defeat chains, area prerequisites (parsed to JSON)
- flag_relationships.json: relationship edges

Usage:
    python export_anchors.py [--output anchor_database.json]
"""

import argparse
import json
import re
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


PROJECT_ROOT = Path(__file__).parent.parent.parent

# Source file paths
EVENT_GRAPH_PATH = PROJECT_ROOT / "scripts" / "event_graph.json"
GROUND_TRUTH_PATH = PROJECT_ROOT / "ground_truth_offsets.json"
FLAG_RELATIONSHIPS_PATH = PROJECT_ROOT / "scripts" / "flag_relationships.json"
CHAIN_DATA_PATH = PROJECT_ROOT / "src" / "discovery" / "chain_data.rs"
OUTPUT_PATH = PROJECT_ROOT / "scripts" / "verification" / "anchor_database.json"


def load_event_graph() -> Dict[str, Any]:
    """Load event_graph.json and extract relevant data."""
    print(f"Loading event_graph.json...")
    with open(EVENT_GRAPH_PATH) as f:
        data = json.load(f)

    result = {
        "metadata": data.get("metadata", {}),
        "progression_chains": data.get("progression_chains", {}),
        "flag_dependencies": {},
        "flag_triggers_by_context": {},
    }

    # Extract flag dependencies (limited to avoid huge memory)
    deps = data.get("flag_dependencies", {})
    for flag_id, dep_data in list(deps.items())[:1000]:  # Limit for manageability
        if dep_data.get("depends_on") or dep_data.get("enables"):
            result["flag_dependencies"][flag_id] = dep_data

    # Index triggers by context
    triggers = data.get("flag_triggers", {})
    context_index = {}
    for flag_id, trigger_data in triggers.items():
        for trigger in trigger_data.get("triggers", []):
            ctx = trigger.get("trigger_context", "unknown")
            if ctx not in context_index:
                context_index[ctx] = []
            context_index[ctx].append(int(flag_id))

    result["flag_triggers_by_context"] = {
        ctx: sorted(set(flags))[:50]  # Keep top 50 per context
        for ctx, flags in context_index.items()
    }

    print(f"  Loaded {len(result['progression_chains'])} progression chains")
    print(f"  Loaded {len(result['flag_dependencies'])} flag dependencies")
    print(f"  Indexed {len(context_index)} trigger contexts")

    return result


def load_ground_truth() -> Dict[str, Any]:
    """Load ground_truth_offsets.json calibration anchors."""
    print(f"Loading ground_truth_offsets.json...")
    with open(GROUND_TRUTH_PATH) as f:
        data = json.load(f)

    calibration = data.get("calibration_anchors", {})
    print(f"  Loaded calibration anchors: tile={len(calibration.get('tile', {}))}, "
          f"block={len(calibration.get('block', {}))}, "
          f"dungeon={len(calibration.get('dungeon', {}))}")

    return {
        "calibration_anchors": calibration,
        "verified_flags_sample": {
            k: v for k, v in list(data.get("verified_flags", {}).items())[:20]
        },
    }


def load_flag_relationships() -> Dict[str, Any]:
    """Load flag_relationships.json edges."""
    print(f"Loading flag_relationships.json...")
    with open(FLAG_RELATIONSHIPS_PATH) as f:
        data = json.load(f)

    edges = data.get("edges", [])

    # Group edges by relationship type
    edges_by_type = {}
    for edge in edges:
        rel_type = edge.get("type", "unknown")
        if rel_type not in edges_by_type:
            edges_by_type[rel_type] = []
        edges_by_type[rel_type].append(edge)

    # Create lookup index by source flag
    source_index = {}
    for edge in edges:
        source = str(edge.get("source"))
        if source not in source_index:
            source_index[source] = []
        source_index[source].append({
            "target": edge.get("target"),
            "type": edge.get("type"),
            "item": edge.get("item"),
        })

    print(f"  Loaded {len(edges)} edges")
    print(f"  Relationship types: {list(edges_by_type.keys())}")

    return {
        "edge_count": len(edges),
        "edges_by_type": {k: len(v) for k, v in edges_by_type.items()},
        "source_index": source_index,
        "sample_edges": edges[:100],  # Keep sample for reference
    }


def parse_chain_data_rs() -> Dict[str, Any]:
    """Parse chain_data.rs for boss chains and area prerequisites."""
    print(f"Parsing chain_data.rs...")

    with open(CHAIN_DATA_PATH) as f:
        content = f.read()

    result = {
        "boss_defeat_chains": [],
        "area_prerequisites": [],
        "geographic_regions": [],
        "verified_block_bases": [],
    }

    # Parse BOSS_DEFEAT_CHAINS
    boss_pattern = r'BossDefeatChain\s*\{\s*name:\s*"([^"]+)"[^}]*defeat_flag:\s*(\d+)[^}]*remembrance_flag:\s*(\d+)[^}]*great_rune_flag:\s*(Some\((\d+)\)|None)[^}]*activation_flag:\s*(Some\((\d+)\)|None)[^}]*remembrance_item:\s*(Some\((\d+)\)|None)'

    for match in re.finditer(boss_pattern, content, re.DOTALL):
        chain = {
            "name": match.group(1),
            "defeat_flag": int(match.group(2)),
            "remembrance_flag": int(match.group(3)),
            "great_rune_flag": int(match.group(5)) if match.group(5) else None,
            "activation_flag": int(match.group(7)) if match.group(7) else None,
            "remembrance_item": int(match.group(9)) if match.group(9) else None,
        }
        result["boss_defeat_chains"].append(chain)

    print(f"  Parsed {len(result['boss_defeat_chains'])} boss defeat chains")

    # Parse AREA_PREREQUISITES
    area_pattern = r'AreaPrerequisite\s*\{\s*area_name:\s*"([^"]+)"[^}]*required_flags:\s*&\[([^\]]*)\][^}]*required_any:\s*&\[([^\]]*)\][^}]*area_flags_start:\s*(\d+)[^}]*landmark_range:\s*(Some\(\((\d+),\s*(\d+)\)\)|None)'

    for match in re.finditer(area_pattern, content, re.DOTALL):
        req_flags = [int(f.strip()) for f in match.group(2).split(',') if f.strip().isdigit()]
        req_any = [int(f.strip()) for f in match.group(3).split(',') if f.strip().isdigit()]

        area = {
            "area_name": match.group(1),
            "required_flags": req_flags,
            "required_any": req_any,
            "area_flags_start": int(match.group(4)),
            "landmark_range": [int(match.group(6)), int(match.group(7))] if match.group(6) else None,
        }
        result["area_prerequisites"].append(area)

    print(f"  Parsed {len(result['area_prerequisites'])} area prerequisites")

    # Parse GEOGRAPHIC_REGIONS
    region_pattern = r'GeographicRegion\s*\{\s*name:\s*"([^"]+)"[^}]*landmark_range:\s*\((\d+),\s*(\d+)\)[^}]*grace_range:\s*(Some\(\((\d+),\s*(\d+)\)\)|None)[^}]*map_fragment:\s*(Some\((\d+)\)|None)'

    for match in re.finditer(region_pattern, content, re.DOTALL):
        region = {
            "name": match.group(1),
            "landmark_range": [int(match.group(2)), int(match.group(3))],
            "grace_range": [int(match.group(5)), int(match.group(6))] if match.group(5) else None,
            "map_fragment": int(match.group(8)) if match.group(8) else None,
        }
        result["geographic_regions"].append(region)

    print(f"  Parsed {len(result['geographic_regions'])} geographic regions")

    # Parse VERIFIED_BLOCK_BASES
    block_pattern = r'BlockBaseOffset\s*\{\s*block_start:\s*(\d+)[^}]*base_offset:\s*0x([0-9a-fA-F]+)[^}]*category:\s*"([^"]+)"'

    for match in re.finditer(block_pattern, content):
        block = {
            "block_start": int(match.group(1)),
            "base_offset": int(match.group(2), 16),
            "category": match.group(3),
        }
        result["verified_block_bases"].append(block)

    print(f"  Parsed {len(result['verified_block_bases'])} verified block bases")

    return result


def build_anchor_database(
    event_graph: Dict[str, Any],
    ground_truth: Dict[str, Any],
    flag_rels: Dict[str, Any],
    chain_data: Dict[str, Any],
) -> Dict[str, Any]:
    """Build the merged anchor database."""
    print("\nBuilding merged anchor database...")

    db = {
        "version": "1.0",
        "description": "Merged chain anchor relationships for corroboration-based verification",
        "sources": [
            "scripts/event_graph.json",
            "ground_truth_offsets.json",
            "src/discovery/chain_data.rs",
            "scripts/flag_relationships.json",
        ],
        "metadata": {
            "event_graph": event_graph.get("metadata", {}),
            "edge_counts": flag_rels.get("edges_by_type", {}),
        },
    }

    # Build category anchors
    db["category_anchors"] = {
        "spirit_ash": {
            "description": "Spirit Ashes obtained from catacomb/dungeon bosses",
            "related_types": ["catacomb_boss", "dungeon_grace", "boss_defeat"],
            "verification_pattern": "Spirit ash pickup correlates with dungeon boss defeat",
        },
        "remembrance": {
            "description": "Boss remembrance possession flags",
            "related_types": ["boss_defeat", "great_rune", "rune_activation"],
            "verification_pattern": "Remembrance possession requires boss defeat",
        },
        "grace": {
            "description": "Site of Grace discovery flags",
            "related_types": ["map_fragment", "landmark", "geographic_region"],
            "verification_pattern": "Grace discovery correlates with nearby landmarks",
        },
        "cookbook": {
            "description": "Crafting cookbook collection flags",
            "related_types": ["world_pickup", "merchant_purchase"],
            "verification_pattern": "Cookbook flag SET when item in inventory",
        },
        "landmark": {
            "description": "Map landmark discovery flags",
            "related_types": ["grace_discovery", "map_fragment"],
            "verification_pattern": "Landmarks correlate with nearby graces",
        },
        "talisman": {
            "description": "Talisman collection flags from dungeon rewards",
            "related_types": ["catacomb_boss", "evergaol_boss"],
            "verification_pattern": "Talisman pickup correlates with boss defeat",
        },
    }

    # Convert boss chains to dict format
    db["boss_defeat_chains"] = {
        str(chain["defeat_flag"]): chain
        for chain in chain_data.get("boss_defeat_chains", [])
    }

    # Convert area prerequisites
    db["area_prerequisites"] = {
        area["area_name"]: {
            "required_flags": area["required_flags"],
            "required_any": area["required_any"],
            "landmark_range": area["landmark_range"],
        }
        for area in chain_data.get("area_prerequisites", [])
    }

    # Convert geographic regions
    db["geographic_regions"] = {
        region["name"]: {
            "landmark_range": region["landmark_range"],
            "grace_range": region["grace_range"],
            "map_fragment": region["map_fragment"],
        }
        for region in chain_data.get("geographic_regions", [])
    }

    # Copy calibration anchors
    db["calibration_anchors"] = ground_truth.get("calibration_anchors", {})

    # Build progression chains from event_graph
    db["progression_chains"] = event_graph.get("progression_chains", {})

    # Build flag relationship index from flag_relationships.json
    db["flag_relationship_index"] = flag_rels.get("source_index", {})

    # Build event graph index by context
    db["event_graph_index"] = {
        "by_context": event_graph.get("flag_triggers_by_context", {}),
    }

    # Flag dependencies (limited)
    db["flag_dependencies"] = event_graph.get("flag_dependencies", {})

    # Verified block bases
    db["verified_block_bases"] = {
        str(block["block_start"]): {
            "base_offset": block["base_offset"],
            "category": block["category"],
        }
        for block in chain_data.get("verified_block_bases", [])
    }

    return db


def main():
    parser = argparse.ArgumentParser(description="Export merged anchor database")
    parser.add_argument("--output", "-o", type=Path, default=OUTPUT_PATH,
                        help="Output file path")
    args = parser.parse_args()

    # Load all sources
    event_graph = load_event_graph()
    ground_truth = load_ground_truth()
    flag_rels = load_flag_relationships()
    chain_data = parse_chain_data_rs()

    # Build merged database
    db = build_anchor_database(event_graph, ground_truth, flag_rels, chain_data)

    # Write output
    print(f"\nWriting to {args.output}...")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.output, 'w') as f:
        json.dump(db, f, indent=2)

    print(f"Done! Output size: {args.output.stat().st_size / 1024:.1f} KB")

    # Print summary
    print("\nSummary:")
    print(f"  Boss defeat chains: {len(db['boss_defeat_chains'])}")
    print(f"  Area prerequisites: {len(db['area_prerequisites'])}")
    print(f"  Geographic regions: {len(db['geographic_regions'])}")
    print(f"  Progression chains: {len(db['progression_chains'])}")
    print(f"  Flag dependencies: {len(db['flag_dependencies'])}")
    print(f"  Verified block bases: {len(db['verified_block_bases'])}")


if __name__ == "__main__":
    main()
