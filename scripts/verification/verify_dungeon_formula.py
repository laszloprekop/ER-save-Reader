#!/usr/bin/env python3
"""
Verify dungeon flag formulas against actual save file data.

This script:
1. Loads a save file and extracts event_flags for a given slot
2. Tests specific dungeon flags using our formula
3. Reports which are set/unset to verify base offsets
"""

import struct
import sys
from pathlib import Path
from typing import Optional, Tuple, List, Dict

# ============================================================================
# CONSTANTS
# ============================================================================

SLOT_SIZE = 0x280020  # Size of each character slot
HEADER_SIZE = 0x310   # Save file header size
EVENT_FLAGS_SIZE = 0x1BF99F  # 1,833,375 bytes

# Validation flags to detect event_flags section (pre-calculated offsets)
# Format: (flag_id, byte_offset, bit_position, name)
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Cave of Knowledge discovered"),
    (76100, 3262, 3, "The First Step grace"),
    (76101, 3262, 2, "The First Step discovered"),
]

# VERIFIED dungeon bases (from ground_truth_offsets.json)
# Only include bases we're confident about
VERIFIED_DUNGEON_BASES = {
    10: {"base": 4112, "section_size": 1125, "name": "Stormveil Castle", "status": "verified"},
    14: {"base": 29987, "section_size": 1125, "name": "Tutorial Areas", "status": "verified"},
    18: {"base": 43487, "section_size": 1125, "name": "Roundtable Hold", "status": "verified"},
    30: {"base": 27411, "section_size": 1125, "name": "Catacombs", "status": "verified"},
    31: {"base": 28634, "section_size": 1125, "name": "Caves", "status": "verified"},
    32: {"base": 31577, "section_size": 1125, "name": "Tunnels", "status": "verified"},
}

# CALCULATED dungeon bases (from slot formula, NEEDS VERIFICATION)
CALCULATED_DUNGEON_BASES = {
    11: {"base": 8612, "section_size": 1125, "name": "Leyndell", "status": "calculated", "slot": 4},
    12: {"base": 15362, "section_size": 1125, "name": "Underground", "status": "calculated", "slot": 10},
    13: {"base": 26612, "section_size": 1125, "name": "Leyndell Royal Capital", "status": "calculated", "slot": 20},
    15: {"base": 33362, "section_size": 1125, "name": "Miquella's Haligtree", "status": "calculated", "slot": 26},
    16: {"base": 36737, "section_size": 1125, "name": "Volcano Manor", "status": "calculated", "slot": 29},
    19: {"base": 46862, "section_size": 1125, "name": "Chapel of Anticipation", "status": "calculated", "slot": 38},
    34: {"base": 60362, "section_size": 1125, "name": "Divine Towers", "status": "calculated", "slot": 60},
    35: {"base": 50237, "section_size": 1125, "name": "Mohgwyn Palace", "status": "calculated", "slot": 41},
    39: {"base": 31112, "section_size": 1125, "name": "Elden Throne", "status": "calculated", "slot": 44},
}

# Known dungeon flags for testing (from the game)
# Format: (flag_id, name, expected_state_for_confessor)
TEST_FLAGS = {
    10: [  # Stormveil Castle
        (10000800, "Godrick the Grafted (defeated)", True),  # Mid-game Confessor has defeated
        (10000850, "Margit, the Fell Omen (defeated)", True),
        (10007990, "Godskin Prayerbook", None),  # Unknown
    ],
    11: [  # Leyndell
        (11000800, "Morgott, the Omen King (defeated)", None),
        (11000850, "Godfrey Golden Shade (defeated)", None),
    ],
    12: [  # Underground
        (12010800, "Dragonkin Soldier (defeated)", None),
        (12020800, "Ancestor Spirit (defeated)", None),
        (12030850, "Fia's Champions (defeated)", None),
        (12050800, "Mohg, Lord of Blood (defeated)", None),
    ],
    13: [  # Leyndell Royal Capital (actually Crumbling Farum Azula per slot table)
        (13000800, "Maliketh (defeated)", None),
        (13000850, "Godskin Duo (defeated)", None),
    ],
    15: [  # Haligtree
        (15000800, "Malenia (defeated)", None),
        (15000850, "Loretta (defeated)", None),
    ],
    16: [  # Volcano Manor
        (16000800, "Rykard (defeated)", None),
        (16000850, "Godskin Noble (defeated)", None),
    ],
    18: [  # Roundtable Hold
        (18000850, "Soldier of Godrick (defeated)", True),  # Tutorial boss
    ],
    30: [  # Catacombs
        (30020800, "Erdtree Burial Watchdog (Stormfoot)", None),
        (30110800, "Black Knife Assassin (Deathtouched)", None),
    ],
}


def detect_event_flags_offset(slot_data: bytes, search_start: int = 0x12000) -> Optional[int]:
    """
    Detect the event_flags section offset within slot data.
    Uses validation flags to find the correct offset.
    """
    best_offset = None
    best_score = 0
    max_search = min(0x15000, len(slot_data) - 10000)

    for test_offset in range(search_start, max_search):
        score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if (byte_val & (1 << bit_pos)) != 0:
                    score += 1

        if score > best_score:
            best_score = score
            best_offset = test_offset

        if score == len(VALIDATION_FLAGS):
            return test_offset  # Perfect match

    # Return best match if we got at least 2 flags
    if best_score >= 2:
        return best_offset

    return None


def calculate_dungeon_offset(flag_id: int, dungeon_bases: Dict) -> Optional[Tuple[int, int, str]]:
    """
    Calculate offset for a dungeon flag.
    Returns (byte_offset, bit_position, status) or None.
    """
    flag_str = f"{flag_id:08d}"
    area = int(flag_str[0:2])
    section = int(flag_str[2:4])
    local_id = int(flag_str[4:8])

    if area not in dungeon_bases:
        return None

    info = dungeon_bases[area]
    base = info["base"]
    section_size = info["section_size"]
    status = info["status"]

    byte_offset = base + section * section_size + local_id // 8
    bit_position = 7 - (flag_id % 8)

    return (byte_offset, bit_position, status)


def check_flag(event_flags: bytes, byte_offset: int, bit_position: int) -> bool:
    """Check if a flag is set at the given offset."""
    if byte_offset >= len(event_flags):
        return False
    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_position) & 1 == 1


def extract_slot_data(save_path: str, slot_index: int) -> bytes:
    """Extract slot data from save file."""
    with open(save_path, 'rb') as f:
        f.seek(HEADER_SIZE + slot_index * SLOT_SIZE)
        return f.read(SLOT_SIZE)


def main():
    # Default save file path
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2"
    slot_index = 0  # Confessor

    if len(sys.argv) > 1:
        save_path = sys.argv[1]
    if len(sys.argv) > 2:
        slot_index = int(sys.argv[2])

    print(f"Loading save file: {save_path}")
    print(f"Slot index: {slot_index}")
    print()

    # Extract slot data
    slot_data = extract_slot_data(save_path, slot_index)
    print(f"Slot data size: {len(slot_data):,} bytes")

    # Detect event_flags offset
    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    print(f"Detected event_flags offset: 0x{event_flags_offset:X} ({event_flags_offset})")
    print()

    # Extract event_flags section
    event_flags = slot_data[event_flags_offset:event_flags_offset + EVENT_FLAGS_SIZE]
    print(f"Event flags size: {len(event_flags):,} bytes")
    print()

    # Combine verified and calculated bases for testing
    all_bases = {**VERIFIED_DUNGEON_BASES, **CALCULATED_DUNGEON_BASES}

    # Test flags by area
    print("=" * 80)
    print("DUNGEON FLAG VERIFICATION RESULTS")
    print("=" * 80)

    for area in sorted(TEST_FLAGS.keys()):
        flags = TEST_FLAGS[area]
        if area not in all_bases:
            print(f"\nArea {area}: NO BASE DEFINED")
            continue

        info = all_bases[area]
        print(f"\nArea {area} - {info['name']} (base={info['base']}, status={info['status']})")
        print("-" * 60)

        for flag_id, name, expected in flags:
            result = calculate_dungeon_offset(flag_id, all_bases)
            if result is None:
                print(f"  {flag_id}: {name} - NO FORMULA")
                continue

            byte_off, bit_pos, status = result
            is_set = check_flag(event_flags, byte_off, bit_pos)

            # Check if result matches expectation
            match_str = ""
            if expected is not None:
                if is_set == expected:
                    match_str = " [MATCH]"
                else:
                    match_str = " [MISMATCH!]"

            state = "ON" if is_set else "OFF"
            print(f"  {flag_id}: {name}")
            print(f"    -> byte={byte_off}, bit={bit_pos}, state={state}{match_str}")

    # Additional probing: scan for potential base offsets
    print()
    print("=" * 80)
    print("BASE OFFSET PROBING")
    print("=" * 80)

    # For unverified areas, try to find flags that are set
    for area in [11, 12, 13, 15, 16]:
        if area not in all_bases:
            continue

        info = all_bases[area]
        base = info["base"]

        # Check bytes around the base for any non-zero values
        set_bytes = []
        for offset in range(base, min(base + 200, len(event_flags))):
            if event_flags[offset] != 0:
                set_bytes.append((offset, event_flags[offset]))

        if set_bytes:
            print(f"\nArea {area} ({info['name']}): Found {len(set_bytes)} non-zero bytes near base {base}")
            for off, val in set_bytes[:5]:  # Show first 5
                print(f"  Byte {off}: 0x{val:02X} ({bin(val)})")
        else:
            print(f"\nArea {area} ({info['name']}): All zeros near base {base} - likely WRONG BASE")


if __name__ == "__main__":
    main()
