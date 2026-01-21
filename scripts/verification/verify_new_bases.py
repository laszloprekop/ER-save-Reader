#!/usr/bin/env python3
"""
Verify newly discovered block bases with multiple methods.

New discoveries:
- Block 62000 (Map fragments): Base 34499
- Block 65000 (Crystal Tears): Base 37412
- Block 67000 (Cookbooks): Base 37411
"""

import json
from pathlib import Path
from typing import Optional

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

# Newly discovered bases
NEW_BASES = {
    62000: 34499,  # Map fragments
    65000: 37412,  # Crystal Tears
    67000: 37411,  # Cookbooks
}

# Test flags per block with expected states
TEST_FLAGS = {
    62000: [
        (62010, "Map: Limgrave, West", True),
        (62011, "Map: Weeping Peninsula", True),
        (62012, "Map: Limgrave, East", True),
        (62020, "Map: Liurnia, East", True),
        (62021, "Map: Liurnia, North", True),
        (62022, "Map: Liurnia, West", True),
    ],
    65000: [
        (65010, "Greenspill Crystal Tear", True),
        (65080, "Opaline Bubbletear", True),
        (65090, "Crimsonburst Crystal Tear", True),
        (65100, "Greenburst Crystal Tear", True),
    ],
    67000: [
        (67020, "Nomadic Warrior's Cookbook [6]", True),
        (67050, "Nomadic Warrior's Cookbook [7]", True),
        (67060, "Nomadic Warrior's Cookbook [12]", True),
        (67800, "Nomadic Warrior's Cookbook [4]", True),
    ],
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
    """Check if flag is set using given base offset."""
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset >= len(event_flags) or byte_offset < 0:
        return None

    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


def main():
    print("Loading save file...")
    with open(SAVE_PATH, 'rb') as f:
        f.seek(HEADER_SIZE + 0 * SLOT_SIZE)  # Slot 0
        slot_data = f.read(SLOT_SIZE)

    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    print(f"Event flags offset: 0x{event_flags_offset:X}")
    event_flags = slot_data[event_flags_offset:]

    print("\n" + "="*80)
    print("VERIFICATION OF NEW BLOCK BASES")
    print("="*80)

    for block_start, base in NEW_BASES.items():
        test_flags = TEST_FLAGS.get(block_start, [])

        print(f"\n{'='*60}")
        print(f"Block {block_start}: Base {base}")
        print("-"*60)

        matches = 0
        total = 0

        for flag_id, name, expected in test_flags:
            relative = flag_id - block_start
            byte_offset = base + relative // 8
            bit_pos = 7 - (flag_id % 8)

            byte_val = event_flags[byte_offset]
            actual = (byte_val >> bit_pos) & 1 == 1

            total += 1
            if actual == expected:
                matches += 1
                status = "✓"
            else:
                status = "✗"

            state = "SET" if actual else "---"
            print(f"  {status} {flag_id} {name[:35]:35} byte {byte_offset}, bit {bit_pos}: {state}")
            print(f"      Raw byte: 0x{byte_val:02X} ({byte_val:08b})")

        print(f"\nResult: {matches}/{total} matches")

    # Cross-check: Show raw bytes around each base
    print("\n" + "="*80)
    print("RAW BYTE ANALYSIS")
    print("="*80)

    for block_start, base in NEW_BASES.items():
        print(f"\nBlock {block_start} (base {base}):")
        print(f"  Bytes {base}-{base+15}:")
        for i in range(16):
            val = event_flags[base + i]
            print(f"    {base+i}: 0x{val:02X} ({val:08b})", end="")
            if val != 0:
                print(" <-- non-zero", end="")
            print()

    # Check adjacent flags (some should be unset)
    print("\n" + "="*80)
    print("ADJACENT FLAG CHECK (expect some NOT set)")
    print("="*80)

    # Map fragments user likely doesn't have
    print("\nMap fragments the user likely does NOT have:")
    unlikely_maps = [
        (62050, "Map: Mountaintops, West"),
        (62051, "Map: Mountaintops, East"),
        (62070, "Map: Lake of Rot"),
        (62080, "Map: Mohgwyn Palace"),
    ]

    base = NEW_BASES[62000]
    for flag_id, name in unlikely_maps:
        actual = check_flag(event_flags, flag_id, 62000, base)
        state = "SET" if actual else "---"
        print(f"  {flag_id} {name:40} {state}")


if __name__ == "__main__":
    main()
