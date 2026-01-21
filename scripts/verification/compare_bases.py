#!/usr/bin/env python3
"""
Compare old vs new bases for blocks 62000, 65000, 67000.
"""

import json
from typing import Optional

RECORDS_PATH = "/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/flag-correlation-candidates.jsonl"
SAVE_PATH = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
]

# Old bases from ground_truth vs New bases from probing
BASES_TO_TEST = {
    62000: {
        "old": 1500,   # From ground_truth
        "new": 9359,   # From probing with negative evidence
    },
    65000: {
        "old": 1875,   # From ground_truth
        "new": 37412,  # From probing
    },
    67000: {
        "old": 2280,   # From ground_truth
        "new": 37411,  # From probing
    },
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


def main():
    print("Loading records and save files...")
    records = load_records()
    slot0_records = [r for r in records if r['slotIndex'] == 0 and r['userMarkedComplete']]

    with open(SAVE_PATH, 'rb') as f:
        f.seek(HEADER_SIZE + 0 * SLOT_SIZE)
        slot0_data = f.read(SLOT_SIZE)
        f.seek(HEADER_SIZE + 1 * SLOT_SIZE)
        slot1_data = f.read(SLOT_SIZE)

    slot0_offset = detect_event_flags_offset(slot0_data)
    slot1_offset = detect_event_flags_offset(slot1_data)

    ef0 = slot0_data[slot0_offset:]
    ef1 = slot1_data[slot1_offset:]

    print(f"\nSlot 0 offset: 0x{slot0_offset:X}")
    print(f"Slot 1 offset: 0x{slot1_offset:X}")

    for block_start, bases in BASES_TO_TEST.items():
        print(f"\n{'='*70}")
        print(f"BLOCK {block_start}")
        print("="*70)

        # Get test flags for this block
        block_records = [r for r in slot0_records if block_start <= r['flagId'] < block_start + 1000]
        print(f"Test flags: {len(block_records)}")

        for base_name, base in bases.items():
            print(f"\n--- {base_name.upper()} BASE: {base} ---")

            # Test against Slot 0 (should be SET)
            slot0_matches = 0
            for r in block_records:
                actual = check_flag(ef0, r['flagId'], block_start, base)
                if actual is True:
                    slot0_matches += 1

            # Test against Slot 1 (should be UNSET for early game)
            slot1_unset = 0
            for r in block_records:
                actual = check_flag(ef1, r['flagId'], block_start, base)
                if actual is False:
                    slot1_unset += 1

            print(f"  Slot 0 (mid-game): {slot0_matches}/{len(block_records)} SET")
            print(f"  Slot 1 (early-game): {slot1_unset}/{len(block_records)} UNSET")

            # Show some details
            print(f"\n  Sample flags:")
            for r in block_records[:5]:
                actual0 = check_flag(ef0, r['flagId'], block_start, base)
                actual1 = check_flag(ef1, r['flagId'], block_start, base)
                s0 = "SET" if actual0 else "---"
                s1 = "SET" if actual1 else "---"
                print(f"    {r['flagId']} {r['flagName'][:35]:35} Slot0:{s0:3} Slot1:{s1:3}")


if __name__ == "__main__":
    main()
