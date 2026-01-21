#!/usr/bin/env python3
"""
Find correct bases for blocks 65000 (Crystal Tears) and 67000 (Cookbooks)
using both positive and negative evidence across multiple slots.

Strategy:
- Use Slot 0 (mid-game) for positive evidence (items collected)
- Use Slot 1 (early game) for negative evidence (items NOT collected)
"""

import json
from pathlib import Path
from typing import Optional, List, Tuple

RECORDS_PATH = "/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/flag-correlation-candidates.jsonl"
SAVE_PATH = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
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


def load_slot_data(slot: int):
    with open(SAVE_PATH, 'rb') as f:
        f.seek(HEADER_SIZE + slot * SLOT_SIZE)
        return f.read(SLOT_SIZE)


def check_flag(event_flags: bytes, flag_id: int, block_start: int, base: int) -> Optional[bool]:
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset >= len(event_flags) or byte_offset < 0:
        return None

    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


def load_records():
    records = []
    with open(RECORDS_PATH, 'r') as f:
        for line in f:
            if line.strip():
                records.append(json.loads(line))
    return records


def find_best_base(
    event_flags_slot0: bytes,
    event_flags_slot1: bytes,
    expected_set: List[int],
    expected_unset: List[int],
    block_start: int
) -> List[Tuple[int, int, int, int, int]]:
    """
    Find base that:
    - Makes expected_set flags SET in slot0
    - Makes expected_unset flags UNSET in slot1 (or slot0)
    """
    results = []

    for base in range(0, 100000):
        # Check slot0 expected SET
        set_matches = 0
        for flag_id in expected_set:
            actual = check_flag(event_flags_slot0, flag_id, block_start, base)
            if actual is True:
                set_matches += 1

        # Check slot1 - should have NOTHING (early game)
        slot1_unset = 0
        slot1_total = 0
        for flag_id in expected_set:
            actual = check_flag(event_flags_slot1, flag_id, block_start, base)
            if actual is not None:
                slot1_total += 1
                if actual is False:
                    slot1_unset += 1

        # Good base: all slot0 SET, all slot1 UNSET
        total_score = set_matches + slot1_unset
        max_score = len(expected_set) * 2

        if total_score >= max_score - 2:  # Within 2 of perfect
            results.append((base, set_matches, len(expected_set), slot1_unset, slot1_total))

    results.sort(key=lambda x: -(x[1] + x[3]))
    return results


def main():
    print("Loading verification records...")
    records = load_records()

    # Get confirmed items from Slot 0
    slot0_records = [r for r in records if r['slotIndex'] == 0 and r['userMarkedComplete']]

    # Group by block
    block_65 = [r for r in slot0_records if 65000 <= r['flagId'] < 66000]
    block_67 = [r for r in slot0_records if 67000 <= r['flagId'] < 68000]

    print(f"Block 65000 records: {len(block_65)}")
    print(f"Block 67000 records: {len(block_67)}")

    # Load save data
    print("\nLoading save files...")
    slot0_data = load_slot_data(0)
    slot1_data = load_slot_data(1)

    slot0_offset = detect_event_flags_offset(slot0_data)
    slot1_offset = detect_event_flags_offset(slot1_data)

    if not slot0_offset or not slot1_offset:
        print("ERROR: Could not detect offsets")
        return

    print(f"Slot 0 offset: 0x{slot0_offset:X}")
    print(f"Slot 1 offset: 0x{slot1_offset:X}")

    event_flags_slot0 = slot0_data[slot0_offset:]
    event_flags_slot1 = slot1_data[slot1_offset:]

    # Search for Block 65000 (Crystal Tears)
    print("\n" + "="*80)
    print("BLOCK 65000: Crystal Tears")
    print("="*80)

    expected_set_65 = [r['flagId'] for r in block_65]
    print(f"Expected SET (from user confirmations): {len(expected_set_65)}")

    results_65 = find_best_base(event_flags_slot0, event_flags_slot1, expected_set_65, [], 65000)

    print("\nTop candidates (slot0 SET + slot1 UNSET):")
    for base, set_m, set_t, unset_m, unset_t in results_65[:10]:
        print(f"  Base {base}: Slot0 SET {set_m}/{set_t}, Slot1 UNSET {unset_m}/{unset_t}")

    # Verify best candidate
    if results_65:
        best_base = results_65[0][0]
        print(f"\nDetails for best base {best_base}:")

        print("\nSlot 0 (Confessor - should have these):")
        for r in block_65[:10]:
            actual = check_flag(event_flags_slot0, r['flagId'], 65000, best_base)
            status = "SET" if actual else "---"
            match = "✓" if actual else "✗"
            print(f"  {match} {r['flagId']} {r['flagName'][:40]:40} {status}")

        print("\nSlot 1 (Wretch - should NOT have these):")
        for r in block_65[:10]:
            actual = check_flag(event_flags_slot1, r['flagId'], 65000, best_base)
            status = "SET" if actual else "---"
            match = "✓" if not actual else "✗"
            print(f"  {match} {r['flagId']} {r['flagName'][:40]:40} {status}")

    # Search for Block 67000 (Cookbooks)
    print("\n" + "="*80)
    print("BLOCK 67000: Cookbooks")
    print("="*80)

    expected_set_67 = [r['flagId'] for r in block_67]
    print(f"Expected SET (from user confirmations): {len(expected_set_67)}")

    results_67 = find_best_base(event_flags_slot0, event_flags_slot1, expected_set_67, [], 67000)

    print("\nTop candidates (slot0 SET + slot1 UNSET):")
    for base, set_m, set_t, unset_m, unset_t in results_67[:10]:
        print(f"  Base {base}: Slot0 SET {set_m}/{set_t}, Slot1 UNSET {unset_m}/{unset_t}")

    # Verify best candidate
    if results_67:
        best_base = results_67[0][0]
        print(f"\nDetails for best base {best_base}:")

        print("\nSlot 0 (Confessor - should have these):")
        for r in block_67[:10]:
            actual = check_flag(event_flags_slot0, r['flagId'], 67000, best_base)
            status = "SET" if actual else "---"
            match = "✓" if actual else "✗"
            print(f"  {match} {r['flagId']} {r['flagName'][:40]:40} {status}")

        print("\nSlot 1 (Wretch - should NOT have these):")
        for r in block_67[:10]:
            actual = check_flag(event_flags_slot1, r['flagId'], 67000, best_base)
            status = "SET" if actual else "---"
            match = "✓" if not actual else "✗"
            print(f"  {match} {r['flagId']} {r['flagName'][:40]:40} {status}")


if __name__ == "__main__":
    main()
