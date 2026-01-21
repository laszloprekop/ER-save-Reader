#!/usr/bin/env python3
"""
Wide-range search for block bases with 0% match in normal ranges.

These blocks likely use bases far outside expected ranges:
- 62000 (Map fragments): 0% in 1300-1700
- 65000 (Crystal Tears): 0% in 1700-2100
- 67000 (Cookbooks): 0% in 2100-2500

Strategy: Search the entire event_flags section (0 to ~500000 bytes).
"""

import json
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Optional, Tuple

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


def find_best_base_wide(
    event_flags: bytes,
    flags: List[Tuple[int, bool]],
    block_start: int,
    max_offset: int = 100000,
    step: int = 1
) -> List[Tuple[int, int, int]]:
    """
    Search wide range for best base offset.
    """
    results = []

    for base in range(0, max_offset, step):
        matches = 0
        total = 0

        for flag_id, expected in flags:
            actual = check_flag_at_base(event_flags, flag_id, block_start, base)
            if actual is not None:
                total += 1
                if actual == expected:
                    matches += 1

        if total > 0 and matches > 0:
            results.append((base, matches, total))

    results.sort(key=lambda x: (-x[1], x[0]))
    return results


def main():
    print("Loading verification records...")
    records = load_records()

    slot_records = [r for r in records if r['slotIndex'] == 0]
    print(f"Slot 0 records: {len(slot_records)}")

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

    # Group records by block
    block_records = defaultdict(list)
    for r in slot_records:
        if r['flagType'] == 'block' and r['userMarkedComplete']:
            block = (r['flagId'] // 1000) * 1000
            block_records[block].append(r)

    # Blocks that showed 0% match in normal ranges
    BLOCKS_TO_SEARCH = [
        (62000, "Map fragments"),
        (65000, "Crystal Tears"),
        (67000, "Cookbooks"),
    ]

    print("\n" + "="*80)
    print("WIDE-RANGE SEARCH FOR 0% BLOCKS")
    print("="*80)

    for block_start, description in BLOCKS_TO_SEARCH:
        recs = block_records.get(block_start, [])
        if not recs:
            print(f"\nBlock {block_start} ({description}): No user-confirmed flags")
            continue

        flags = [(r['flagId'], True) for r in recs]

        print(f"\n{'='*60}")
        print(f"Block {block_start}: {description}")
        print(f"User-confirmed flags: {len(flags)}")
        print(f"Flags: {[f[0] for f in flags[:10]]}{'...' if len(flags) > 10 else ''}")
        print("-"*60)

        # First pass: coarse search (step=100)
        print("Phase 1: Coarse search (step=100, range 0-100000)...")
        coarse_results = find_best_base_wide(event_flags, flags, block_start, max_offset=100000, step=100)

        if coarse_results:
            best_coarse = coarse_results[0][0]
            print(f"Coarse best: Base ~{best_coarse}, {coarse_results[0][1]}/{coarse_results[0][2]}")

            # Fine search around best coarse result
            fine_start = max(0, best_coarse - 200)
            fine_end = best_coarse + 200
            print(f"\nPhase 2: Fine search ({fine_start}-{fine_end})...")

            fine_results = []
            for base in range(fine_start, fine_end):
                matches = 0
                total = 0
                for flag_id, expected in flags:
                    actual = check_flag_at_base(event_flags, flag_id, block_start, base)
                    if actual is not None:
                        total += 1
                        if actual == expected:
                            matches += 1
                if total > 0 and matches > 0:
                    fine_results.append((base, matches, total))

            fine_results.sort(key=lambda x: (-x[1], x[0]))

            print("Top fine candidates:")
            for base, matches, total in fine_results[:5]:
                pct = (matches / total * 100) if total > 0 else 0
                print(f"  Base {base}: {matches}/{total} ({pct:.1f}%)")

            # Show flags at best base
            if fine_results:
                best_base = fine_results[0][0]
                print(f"\nFlags at best base {best_base}:")
                for r in recs[:15]:
                    actual = check_flag_at_base(event_flags, r['flagId'], block_start, best_base)
                    status = "SET" if actual else "---"
                    match = "✓" if actual else "✗"
                    print(f"  {match} {r['flagId']} {r['flagName'][:40]:40} {status}")
                if len(recs) > 15:
                    print(f"  ... and {len(recs) - 15} more")
        else:
            print("No matches found in coarse search!")


if __name__ == "__main__":
    main()
