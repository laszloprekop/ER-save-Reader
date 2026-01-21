#!/usr/bin/env python3
"""
Validate map fragment base discovery using inseparable evidence.

Key insight: Map fragments require physical exploration. If a map shows as "collected"
but the user hasn't explored that area (no graces discovered there), the formula is wrong.

Test cases:
- Maps the user HAS (marked complete): Should be SET
- Maps from unexplored areas: Should be UNSET (no graces = no maps)
"""

import json
from pathlib import Path
from collections import defaultdict
from typing import Optional, Dict, List

RECORDS_PATH = "/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/flag-correlation-candidates.jsonl"
SAVE_PATH = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Map fragment to grace region mapping
# If a region has no graces discovered, the map shouldn't be collected
MAP_TO_REGION = {
    62010: ("Limgrave", "West Limgrave"),
    62011: ("Weeping Peninsula",),
    62012: ("Limgrave", "East Limgrave"),
    62020: ("Liurnia", "Liurnia East"),
    62021: ("Liurnia", "Liurnia North"),
    62022: ("Liurnia", "Liurnia West"),
    62030: ("Altus Plateau",),
    62031: ("Leyndell",),  # Capital Outskirts
    62032: ("Mt. Gelmir",),
    62040: ("Caelid",),
    62041: ("Dragonbarrow",),
    62050: ("Mountaintops", "Mountaintops West"),
    62051: ("Mountaintops", "Mountaintops East", "Consecrated Snowfield"),
    62060: ("Ainsel River",),
    62061: ("Lake of Rot",),
    62062: ("Deeproot Depths",),
    62063: ("Siofra River",),
    62070: ("Lake of Rot",),  # Duplicate?
    62080: ("Mohgwyn Palace",),
}


def detect_event_flags_offset(slot_data: bytes) -> Optional[int]:
    for test_offset in range(0x12000, 0x15000):
        score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if (byte_val & (1 << bit_pos)) != 0:
                    score += 1
        if score == len(VALIDATION_FLAGS):
            return test_offset
    return None


def load_records() -> List[Dict]:
    records = []
    with open(RECORDS_PATH, 'r') as f:
        for line in f:
            if line.strip():
                records.append(json.loads(line))
    return records


def check_flag(event_flags: bytes, flag_id: int, block_start: int, base: int) -> Optional[bool]:
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset >= len(event_flags) or byte_offset < 0:
        return None

    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


def main():
    print("Loading records...")
    records = load_records()
    slot0_records = [r for r in records if r['slotIndex'] == 0]

    # Build grace region set from user completions
    grace_records = [r for r in slot0_records if r['flagCategory'] == 'Grace' and r['userMarkedComplete']]
    explored_regions = set()
    for r in grace_records:
        region = r.get('flagRegion', 'Unknown')
        if region:
            explored_regions.add(region)

    print(f"\nExplored regions (from {len(grace_records)} graces):")
    for region in sorted(explored_regions):
        print(f"  - {region}")

    # Get user-marked map fragments
    map_records = [r for r in slot0_records if r['flagCategory'] == 'Map Fragment']
    user_maps = {r['flagId']: r['userMarkedComplete'] for r in map_records}

    print(f"\nUser-marked map fragments ({len(map_records)} records):")
    for r in map_records:
        status = "HAS" if r['userMarkedComplete'] else "---"
        print(f"  {status} {r['flagId']} {r['flagName']}")

    print("\n" + "="*80)
    print("TESTING CANDIDATE BASE 34499")
    print("="*80)

    # Load save
    with open(SAVE_PATH, 'rb') as f:
        f.seek(HEADER_SIZE + 0 * SLOT_SIZE)
        slot_data = f.read(SLOT_SIZE)

    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    event_flags = slot_data[event_flags_offset:]

    # Test base 34499
    base = 34499
    print(f"\nAll map fragments at base {base}:")

    contradictions = []
    agreements = []

    for flag_id, regions in sorted(MAP_TO_REGION.items()):
        actual = check_flag(event_flags, flag_id, 62000, base)
        actual_str = "SET" if actual else "---"

        # Check if user has explored any of these regions
        has_region = any(r in explored_regions for r in regions)
        user_marked = user_maps.get(flag_id)

        # Determine expected state based on exploration
        if has_region:
            expected = "likely SET"
        else:
            expected = "likely UNSET"

        # Check for contradictions
        if actual and not has_region and user_marked is not True:
            # Formula says SET but no exploration evidence
            contradictions.append((flag_id, regions, "Formula=SET, No exploration"))
            marker = " ⚠️ CONTRADICTION"
        elif not actual and has_region and user_marked is True:
            # Formula says UNSET but user has it
            contradictions.append((flag_id, regions, "Formula=UNSET, User has it"))
            marker = " ⚠️ CONTRADICTION"
        else:
            agreements.append(flag_id)
            marker = " ✓" if actual == (user_marked or has_region) else ""

        region_str = ", ".join(regions)
        print(f"  {flag_id} {actual_str:3} (expected: {expected:12}) regions: {region_str}{marker}")

    print(f"\nSummary:")
    print(f"  Agreements: {len(agreements)}")
    print(f"  Contradictions: {len(contradictions)}")

    if contradictions:
        print(f"\nContradiction details:")
        for flag_id, regions, reason in contradictions:
            print(f"  {flag_id}: {reason} - regions: {', '.join(regions)}")

    # Also test alternative - maybe base is completely wrong
    print("\n" + "="*80)
    print("SEARCHING FOR CORRECT BASE (wide search)")
    print("="*80)

    # Get all user-confirmed maps
    confirmed_maps = [(fid, True) for fid, marked in user_maps.items() if marked]
    unconfirmed_maps = [(fid, False) for fid, marked in user_maps.items() if not marked]

    if confirmed_maps:
        print(f"\nSearching for base that makes {len(confirmed_maps)} confirmed maps SET...")
        print(f"And {len(unconfirmed_maps)} unconfirmed maps UNSET...")

        best_results = []
        for test_base in range(0, 100000, 100):  # Coarse search
            matches = 0
            total = 0

            for flag_id, expected in confirmed_maps:
                actual = check_flag(event_flags, flag_id, 62000, test_base)
                if actual is not None:
                    total += 1
                    if actual == expected:
                        matches += 1

            for flag_id, expected in unconfirmed_maps:
                actual = check_flag(event_flags, flag_id, 62000, test_base)
                if actual is not None:
                    total += 1
                    if actual == expected:
                        matches += 1

            if matches > 0:
                best_results.append((test_base, matches, total))

        best_results.sort(key=lambda x: -x[1])
        print("\nTop candidates considering BOTH set and unset flags:")
        for base, matches, total in best_results[:10]:
            print(f"  Base {base}: {matches}/{total} ({matches/total*100:.1f}%)")


if __name__ == "__main__":
    main()
