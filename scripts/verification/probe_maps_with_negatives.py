#!/usr/bin/env python3
"""
Find correct map fragment base using both positive and negative evidence.

The user is mid-game (explored Limgrave through Altus/Mt. Gelmir).
They have NOT explored: Mountaintops, Lake of Rot, Mohgwyn Palace, Deeproot Depths.

A correct base must:
1. Show confirmed maps as SET
2. Show unexplored area maps as UNSET
"""

import json
from pathlib import Path
from typing import Optional, List, Tuple

SAVE_PATH = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
]

# Flags we KNOW should be SET (user confirmed)
EXPECTED_SET = [
    (62010, "Map: Limgrave, West"),
    (62011, "Map: Weeping Peninsula"),
    (62012, "Map: Limgrave, East"),
    (62020, "Map: Liurnia, East"),
    (62021, "Map: Liurnia, North"),
    (62022, "Map: Liurnia, West"),
    (62030, "Map: Altus Plateau"),
    (62032, "Map: Mt. Gelmir"),
    (62040, "Map: Caelid"),
    (62041, "Map: Dragonbarrow"),
    (62060, "Map: Ainsel River"),
    (62063, "Map: Siofra River"),
]

# Flags we KNOW should be UNSET (unexplored late-game areas)
EXPECTED_UNSET = [
    (62050, "Map: Mountaintops, West"),
    (62051, "Map: Mountaintops, East"),
    (62061, "Map: Lake of Rot"),  # Different from 62070
    (62062, "Map: Deeproot Depths"),
    (62070, "Map: Lake of Rot"),  # Duplicate ID?
    (62080, "Map: Mohgwyn Palace"),
]


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


def check_flag(event_flags: bytes, flag_id: int, block_start: int, base: int) -> Optional[bool]:
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset >= len(event_flags) or byte_offset < 0:
        return None

    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


def score_base(event_flags: bytes, base: int) -> Tuple[int, int, int, int]:
    """Score a base by checking expected SET and UNSET flags.
    Returns (set_matches, set_total, unset_matches, unset_total)
    """
    set_matches = 0
    set_total = len(EXPECTED_SET)

    for flag_id, name in EXPECTED_SET:
        actual = check_flag(event_flags, flag_id, 62000, base)
        if actual is True:
            set_matches += 1

    unset_matches = 0
    unset_total = len(EXPECTED_UNSET)

    for flag_id, name in EXPECTED_UNSET:
        actual = check_flag(event_flags, flag_id, 62000, base)
        if actual is False:
            unset_matches += 1

    return set_matches, set_total, unset_matches, unset_total


def main():
    print("Loading save file...")
    with open(SAVE_PATH, 'rb') as f:
        f.seek(HEADER_SIZE + 0 * SLOT_SIZE)
        slot_data = f.read(SLOT_SIZE)

    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    print(f"Event flags offset: 0x{event_flags_offset:X}")
    event_flags = slot_data[event_flags_offset:]

    print(f"\nSearching for base where:")
    print(f"  - {len(EXPECTED_SET)} confirmed maps are SET")
    print(f"  - {len(EXPECTED_UNSET)} unexplored maps are UNSET")

    print("\n" + "="*80)
    print("WIDE SEARCH (0-100000)")
    print("="*80)

    results = []

    for base in range(0, 100000):
        set_m, set_t, unset_m, unset_t = score_base(event_flags, base)

        # Combined score: both positive and negative evidence
        total_matches = set_m + unset_m
        total_tests = set_t + unset_t

        if total_matches == total_tests:  # Perfect match
            results.append((base, set_m, set_t, unset_m, unset_t, "PERFECT"))
        elif total_matches >= total_tests - 1:  # Off by 1
            results.append((base, set_m, set_t, unset_m, unset_t, "CLOSE"))

    # Sort by total matches
    results.sort(key=lambda x: -(x[1] + x[3]))

    print("\nPerfect matches (all SET correct AND all UNSET correct):")
    perfect = [r for r in results if r[5] == "PERFECT"]
    if perfect:
        for base, set_m, set_t, unset_m, unset_t, status in perfect[:20]:
            print(f"  Base {base}: SET {set_m}/{set_t}, UNSET {unset_m}/{unset_t}")
    else:
        print("  None found!")

    print("\nClose matches (off by 1):")
    close = [r for r in results if r[5] == "CLOSE"]
    for base, set_m, set_t, unset_m, unset_t, status in close[:20]:
        print(f"  Base {base}: SET {set_m}/{set_t}, UNSET {unset_m}/{unset_t}")

    # Show details for best candidates
    if perfect:
        best_base = perfect[0][0]
    elif close:
        best_base = close[0][0]
    else:
        print("\nNo good candidates found!")
        # Show top results anyway
        results.sort(key=lambda x: -(x[1] + x[3]))
        print("\nTop results regardless:")
        for base, set_m, set_t, unset_m, unset_t, _ in results[:10]:
            print(f"  Base {base}: SET {set_m}/{set_t}, UNSET {unset_m}/{unset_t}")
        return

    print(f"\n{'='*60}")
    print(f"DETAILS FOR BEST CANDIDATE: Base {best_base}")
    print("="*60)

    print("\nExpected SET flags:")
    for flag_id, name in EXPECTED_SET:
        actual = check_flag(event_flags, flag_id, 62000, best_base)
        status = "SET" if actual else "---"
        match = "✓" if actual else "✗"
        relative = flag_id - 62000
        byte_off = best_base + relative // 8
        bit_pos = 7 - (flag_id % 8)
        print(f"  {match} {flag_id} {name:30} {status} (byte {byte_off}, bit {bit_pos})")

    print("\nExpected UNSET flags:")
    for flag_id, name in EXPECTED_UNSET:
        actual = check_flag(event_flags, flag_id, 62000, best_base)
        status = "SET" if actual else "---"
        match = "✓" if not actual else "✗"
        relative = flag_id - 62000
        byte_off = best_base + relative // 8
        bit_pos = 7 - (flag_id % 8)
        print(f"  {match} {flag_id} {name:30} {status} (byte {byte_off}, bit {bit_pos})")

    # Show raw bytes
    print(f"\nRaw bytes at base {best_base}:")
    for i in range(15):
        val = event_flags[best_base + i]
        print(f"  {best_base + i}: 0x{val:02X} ({val:08b})")


if __name__ == "__main__":
    main()
