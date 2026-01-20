#!/usr/bin/env python3
"""
Extract test cases from verification-records.jsonl for use in the Rust test suite.

This script:
1. Loads verification records from the JSONL file
2. Recalculates offsets with the CORRECTED formulas
3. Outputs test case definitions for different flag types
"""

import json
import sys
from pathlib import Path
from collections import defaultdict
from typing import List, Dict, Tuple, Optional

# CORRECTED formula constants
TILE_BASE_OFFSET = 489981  # CORRECTED 2026-01-20 (was 485330)
TILE_BYTES_PER_SLOT = 875
TILE_SLOTS_PER_ROW = 40
TILE_ROW_BASE = 33
TILE_COL_BASE = 30
TILE_MAX_LOCAL_ID = 6999

# Block bases (from ground_truth_offsets.json)
BLOCK_BASES = {
    60000: 2548,  # Progression flags
    62000: 1500,  # Map fragments
    65000: 1875,  # Whetblades (unverified)
    67000: 3546,  # Cookbooks
    68000: 3671,  # Cookbooks continued
    71000: 2625,  # Tutorial graces
    72000: 2750,  # Dungeon graces (unverified)
    73000: 2664,  # Dungeon graces (verified)
    74000: 3000,  # Extended dungeon graces (unverified)
    75000: 3125,  # Extended graces (unverified)
    76000: 3250,  # World graces
    77000: 3375,  # Extended world graces
    78000: 3500,  # POI flags (unverified)
}

# Dungeon bases (from ground_truth_offsets.json)
DUNGEON_BASES = {
    10: 4112,   # Stormveil Castle
    11: 4112,   # Leyndell (unverified)
    12: 0,      # Underground (unknown)
    14: 0,      # Academy (unknown)
    18: 0,      # Roundtable Hold (unknown)
    30: 27411,  # Catacombs
    31: 28634,  # Caves
    32: 31577,  # Tunnels
    34: 0,      # Divine Towers (unknown)
}


def calculate_block_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """Calculate offset for block-based (5-6 digit) flags."""
    block_start = (flag_id // 1000) * 1000
    if block_start not in BLOCK_BASES:
        return None

    base = BLOCK_BASES[block_start]
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_position = 7 - (flag_id % 8)
    return (byte_offset, bit_position)


def calculate_tile_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """Calculate offset for tile-based (10-digit) flags."""
    flag_str = str(flag_id)
    if len(flag_str) != 10:
        return None

    row = int(flag_str[2:4])
    col = int(flag_str[4:6])
    local_id = int(flag_str[6:])

    if local_id > TILE_MAX_LOCAL_ID:
        return None  # Untrackable

    slot = (row - TILE_ROW_BASE) * TILE_SLOTS_PER_ROW + (col - TILE_COL_BASE)
    if slot < 0:
        return None

    byte_offset = TILE_BASE_OFFSET + slot * TILE_BYTES_PER_SLOT + (local_id // 8)
    bit_position = 7 - (local_id % 8)
    return (byte_offset, bit_position)


def calculate_dungeon_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """Calculate offset for dungeon (8-digit) flags."""
    flag_str = f"{flag_id:08d}"
    area = int(flag_str[0:2])
    section = int(flag_str[2:4])
    local_id = int(flag_str[4:8])

    if area not in DUNGEON_BASES or DUNGEON_BASES[area] == 0:
        return None

    base = DUNGEON_BASES[area]
    byte_offset = base + section * 1125 + (local_id // 8)
    bit_position = 7 - (flag_id % 8)
    return (byte_offset, bit_position)


def get_flag_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """Calculate offset using appropriate formula."""
    if flag_id >= 1_000_000_000:
        return calculate_tile_offset(flag_id)
    elif flag_id >= 10_000_000:
        return calculate_dungeon_offset(flag_id)
    else:
        return calculate_block_offset(flag_id)


def load_records(jsonl_path: str) -> List[Dict]:
    """Load verification records from JSONL file."""
    records = []
    with open(jsonl_path, 'r') as f:
        for line in f:
            if line.strip():
                records.append(json.loads(line))
    return records


def analyze_records(records: List[Dict], slot_index: int = 0):
    """Analyze records and find candidates for test cases."""
    slot_records = [r for r in records if r.get('slotIndex') == slot_index]

    results = {
        'confirmed_matches': [],
        'formula_mismatch': [],
        'untrackable': [],
        'unknown_formula': [],
    }

    for record in slot_records:
        if not record.get('manualStatus'):
            continue  # Skip flags where user didn't confirm state

        flag_id = record.get('flagId')
        offset = get_flag_offset(flag_id)

        if offset is None:
            flag_str = str(flag_id)
            if len(flag_str) == 10 and int(flag_str[6:]) >= 7000:
                results['untrackable'].append(record)
            else:
                results['unknown_formula'].append(record)
            continue

        byte_offset, bit_position = offset

        # Check if our formula matches the stored computed offset
        stored_offset = record.get('computedByteOffset', -1)
        stored_bit = record.get('computedBitPosition', -1)

        # Create enhanced record
        enhanced = {
            **record,
            'new_byte_offset': byte_offset,
            'new_bit_position': bit_position,
        }

        if record.get('matches'):
            results['confirmed_matches'].append(enhanced)
        else:
            results['formula_mismatch'].append(enhanced)

    return results


def print_test_cases(results: Dict, slot_name: str = "Confessor"):
    """Print test cases in a format suitable for Rust."""
    print(f"// Test cases extracted from verification-records.jsonl for {slot_name}")
    print(f"// Total: {len(results['confirmed_matches'])} confirmed matches")
    print()

    # Group by flag type
    by_type = defaultdict(list)
    for r in results['confirmed_matches']:
        flag_type = r.get('flagType', 'unknown')
        by_type[flag_type].append(r)

    for flag_type, flags in sorted(by_type.items()):
        print(f"// {flag_type.upper()} flags: {len(flags)}")
        for f in flags:
            flag_id = f.get('flagId')
            name = f.get('flagName', 'Unknown')
            byte_offset = f.get('new_byte_offset')
            bit_position = f.get('new_bit_position')
            region = f.get('flagRegion', 'Unknown')

            # Rust-friendly format
            print(f'    // {name} ({region})')
            print(f'    ({flag_id}, {byte_offset}, {bit_position}, true, "{name}"),')
        print()


def print_discovery_opportunities(results: Dict):
    """Print flags that might help discover missing bases."""
    print("\n// FLAGS WITH FORMULA MISMATCHES (potential base discovery)")
    print(f"// Total: {len(results['formula_mismatch'])}")

    # Group by block/area
    by_area = defaultdict(list)
    for r in results['formula_mismatch']:
        flag_id = r.get('flagId')
        if flag_id >= 1_000_000_000:
            flag_str = str(flag_id)
            area = f"tile_{flag_str[2:6]}"
        elif flag_id >= 10_000_000:
            area = f"dungeon_{flag_id // 1_000_000}"
        else:
            area = f"block_{(flag_id // 1000) * 1000}"
        by_area[area].append(r)

    for area, flags in sorted(by_area.items()):
        print(f"\n// {area}: {len(flags)} flags")
        for f in flags[:3]:
            flag_id = f.get('flagId')
            name = f.get('flagName', 'Unknown')[:40]
            print(f'//   {flag_id}: {name}')


def main():
    jsonl_path = "/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/verification-records.jsonl"

    if len(sys.argv) > 1:
        jsonl_path = sys.argv[1]

    print(f"Loading records from: {jsonl_path}")
    records = load_records(jsonl_path)
    print(f"Total records: {len(records)}")

    results = analyze_records(records, slot_index=0)

    print(f"\nSlot 0 Analysis:")
    print(f"  Confirmed matches: {len(results['confirmed_matches'])}")
    print(f"  Formula mismatches: {len(results['formula_mismatch'])}")
    print(f"  Untrackable (local_id >= 7000): {len(results['untrackable'])}")
    print(f"  Unknown formula: {len(results['unknown_formula'])}")

    print("\n" + "="*60)
    print_test_cases(results, "Confessor")
    print_discovery_opportunities(results)


if __name__ == "__main__":
    main()
