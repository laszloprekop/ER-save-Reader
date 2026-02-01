#!/usr/bin/env python3
"""
Extract test cases from flag-correlation-candidates.jsonl for use in the Rust test suite.

This script:
1. Loads verification records from the JSONL file
2. Recalculates offsets with the CORRECTED formulas
3. Outputs test case definitions for different flag types

IMPORTANT: All formula constants are loaded from ground_truth_loader.py
which reads from ground_truth_offsets.json (the single source of truth).
"""

import json
import sys
from pathlib import Path
from collections import defaultdict
from typing import List, Dict, Tuple, Optional

# Add project root to path
PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.ground_truth_loader import (
    load_block_bases,
    load_dungeon_bases,
    get_tile_config,
    calculate_block_offset as gt_calculate_block_offset,
    calculate_tile_offset as gt_calculate_tile_offset,
    calculate_dungeon_offset as gt_calculate_dungeon_offset,
)


def calculate_block_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """Calculate offset for block-based (5-6 digit) flags.

    Delegates to ground_truth_loader.calculate_block_offset for centralized formula.
    """
    return gt_calculate_block_offset(flag_id)


def calculate_tile_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """Calculate offset for tile-based (10-digit) flags.

    Delegates to ground_truth_loader.calculate_tile_offset for centralized formula.
    """
    return gt_calculate_tile_offset(flag_id)


def calculate_dungeon_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """Calculate offset for dungeon (8-digit) flags.

    Delegates to ground_truth_loader.calculate_dungeon_offset for centralized formula.
    """
    return gt_calculate_dungeon_offset(flag_id)


def get_flag_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """Calculate offset using appropriate formula.

    Uses ground_truth_loader functions which read from ground_truth_offsets.json.
    """
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
        if not record.get('userMarkedComplete'):
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
    print(f"// Test cases extracted from flag-correlation-candidates.jsonl for {slot_name}")
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
    jsonl_path = "/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/flag-correlation-candidates.jsonl"

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
