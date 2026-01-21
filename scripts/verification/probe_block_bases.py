#!/usr/bin/env python3
"""
Systematically probe block bases using known flag states from correlation candidates.

Strategy:
1. Load flag-correlation-candidates with userMarkedComplete=true (user confirmed flags)
2. For each problematic block, search for the correct base offset
3. A correct base should make MOST user-confirmed flags show as SET

Note: userMarkedComplete indicates user manually marked this flag as complete.
"""

import json
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Optional, Tuple

RECORDS_PATH = "/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/flag-correlation-candidates.jsonl"
SAVE_PATH = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

# Validation flags to detect event_flags section
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]


def detect_event_flags_offset(slot_data: bytes, search_start: int = 0x12000) -> Optional[int]:
    for test_offset in range(search_start, min(0x15000, len(slot_data) - 10000)):
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


def check_flag_at_base(event_flags: bytes, flag_id: int, block_start: int, base: int) -> Optional[bool]:
    """Check if flag is set using given base offset."""
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset >= len(event_flags) or byte_offset < 0:
        return None

    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


def find_best_base_for_flags(
    event_flags: bytes,
    flags: List[Tuple[int, bool]],  # (flag_id, expected_state)
    block_start: int,
    search_range: Tuple[int, int]
) -> List[Tuple[int, int, int]]:
    """
    Find base offsets that maximize agreement with expected flag states.

    Returns list of (base, matches, total) sorted by matches descending.
    """
    results = []

    for base in range(search_range[0], search_range[1]):
        matches = 0
        total = 0

        for flag_id, expected in flags:
            actual = check_flag_at_base(event_flags, flag_id, block_start, base)
            if actual is not None:
                total += 1
                if actual == expected:
                    matches += 1

        if total > 0:
            results.append((base, matches, total))

    results.sort(key=lambda x: (-x[1], x[0]))  # Sort by matches desc, then base asc
    return results


def main():
    print("Loading verification records...")
    records = load_records()

    # Filter to slot 0 (Confessor) which has most data
    slot_records = [r for r in records if r['slotIndex'] == 0]
    print(f"Slot 0 records: {len(slot_records)}")

    # Load save data
    print("\nLoading save file...")
    with open(SAVE_PATH, 'rb') as f:
        f.seek(HEADER_SIZE + 0 * SLOT_SIZE)
        slot_data = f.read(SLOT_SIZE)

    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    print(f"Event flags offset: 0x{event_flags_offset:X}")
    event_flags = slot_data[event_flags_offset:]

    # Group records by block (for block-type flags)
    block_records = defaultdict(list)
    for r in slot_records:
        if r['flagType'] == 'block' and r['userMarkedComplete']:  # User confirmed as SET
            block = (r['flagId'] // 1000) * 1000
            block_records[block].append(r)

    # Problematic blocks to investigate
    BLOCKS_TO_PROBE = [
        (76000, (3000, 3500), "World graces (Limgrave, etc.)"),
        (71000, (2500, 2900), "Legacy graces (Stormveil, etc.)"),
        (73000, (2500, 2900), "Dungeon graces"),
        (62000, (1300, 1700), "Map fragments"),
        (67000, (2100, 2500), "Cookbooks"),
        (65000, (1700, 2100), "Crystal Tears"),
        (60000, (2300, 2700), "Progression flags"),
        (78000, (3300, 3700), "Grace guidance"),
    ]

    print("\n" + "="*80)
    print("BLOCK BASE PROBING RESULTS")
    print("="*80)

    for block_start, search_range, description in BLOCKS_TO_PROBE:
        recs = block_records.get(block_start, [])
        if not recs:
            print(f"\nBlock {block_start} ({description}): No user-confirmed flags")
            continue

        # Create flag list with expected states (all True since user confirmed)
        flags = [(r['flagId'], True) for r in recs]

        print(f"\n{'='*60}")
        print(f"Block {block_start}: {description}")
        print(f"User-confirmed flags: {len(flags)}")
        print(f"Search range: {search_range[0]} - {search_range[1]}")
        print("-"*60)

        # Find best bases
        results = find_best_base_for_flags(event_flags, flags, block_start, search_range)

        # Show top 5 results
        print("Top candidates (base, matches/total):")
        for base, matches, total in results[:5]:
            pct = (matches / total * 100) if total > 0 else 0
            marker = " <-- BEST" if matches == results[0][1] else ""
            print(f"  Base {base}: {matches}/{total} ({pct:.1f}%){marker}")

        # Show which flags match at best base
        if results:
            best_base = results[0][0]
            print(f"\nFlags at best base {best_base}:")
            for r in recs[:10]:
                actual = check_flag_at_base(event_flags, r['flagId'], block_start, best_base)
                status = "SET" if actual else "---"
                match = "✓" if actual else "✗"
                print(f"  {match} {r['flagId']} {r['flagName'][:40]:40} {status}")
            if len(recs) > 10:
                print(f"  ... and {len(recs) - 10} more")

    # Also check sub-blocks for 71000
    print("\n" + "="*80)
    print("SUB-BLOCK ANALYSIS: Block 71000")
    print("="*80)

    # Group 71000 flags by 100s
    sub_blocks = defaultdict(list)
    for r in block_records.get(71000, []):
        sub = (r['flagId'] // 100) * 100
        sub_blocks[sub].append(r)

    for sub_start, recs in sorted(sub_blocks.items()):
        if len(recs) < 2:
            continue

        flags = [(r['flagId'], True) for r in recs]
        results = find_best_base_for_flags(event_flags, flags, sub_start, (2500, 2900))

        print(f"\nSub-block {sub_start}: {len(recs)} flags")
        if results:
            for base, matches, total in results[:3]:
                pct = (matches / total * 100) if total > 0 else 0
                print(f"  Base {base}: {matches}/{total} ({pct:.1f}%)")


if __name__ == "__main__":
    main()
