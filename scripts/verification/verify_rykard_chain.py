#!/usr/bin/env python3
"""
Verify complete Rykard event chain.

Complete chain discovered from m16_00_00_00.emevd.js and common.emevd.js:

1. Boss Fight Entry:
   - 16002805: Boss fight initiated flag

2. Boss Defeat (Event 16002800):
   - 16000800: Dungeon defeat flag (Rykard dead)
   - 9122: Progression flag (triggers Event 1100)
   - 61122: Single-player completion flag (if in own world)

3. Remembrance Award (Event 1100 with 9122):
   - Waits for 9122
   - Awards ItemLot 10220 (Rykard's Remembrance, item 2953)
   - Sets 510220 when picked up

4. Post-Boss:
   - 16000000: Grace flag (Audience Pathway grace)
   - 16001950: Grace asset enabled
   - 16007950: ItemLot 16000950 pickup (item 7520)
"""

import struct
import sys
from typing import Optional

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

# Validation flags to detect event_flags section
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Complete Rykard chain flags
RYKARD_FLAGS = {
    # Boss defeat flags
    "16000800": {"desc": "Rykard dungeon defeat", "type": "dungeon", "area": 16, "base": 36737},
    "9122": {"desc": "Rykard progression flag", "type": "block", "base": None},  # Need to verify base
    "61122": {"desc": "Rykard SP completion", "type": "block", "base": None},

    # Remembrance
    "510220": {"desc": "Remembrance pickup flag", "type": "block", "base": 63750},

    # Grace/Post-boss
    "16000000": {"desc": "Audience Pathway grace", "type": "dungeon", "area": 16, "base": 36737},
    "16007950": {"desc": "Post-boss ItemLot pickup", "type": "dungeon", "area": 16, "base": 36737},

    # Boss fight state
    "16002805": {"desc": "Boss fight initiated", "type": "dungeon", "area": 16, "base": 36737},
}


def detect_event_flags_offset(slot_data: bytes, search_start: int = 0x12000) -> Optional[int]:
    """Detect the event_flags section offset within slot data."""
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


def check_dungeon_flag(event_flags: bytes, flag_id: int, base: int) -> Optional[bool]:
    """Check an 8-digit dungeon flag."""
    section = (flag_id // 10_000) % 100
    local_id = flag_id % 10_000
    section_size = 1125

    byte_offset = base + section * section_size + local_id // 8
    bit_pos = 7 - (local_id % 8)

    if byte_offset >= len(event_flags):
        return None

    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


def check_block_flag(event_flags: bytes, flag_id: int, base: int) -> Optional[bool]:
    """Check a block flag with known base."""
    if base is None:
        return None

    block_start = (flag_id // 1000) * 1000
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset >= len(event_flags):
        return None

    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


def check_flag(event_flags: bytes, flag_str: str, info: dict) -> Optional[bool]:
    """Check a flag based on its type."""
    flag_id = int(flag_str)

    if info["type"] == "dungeon":
        return check_dungeon_flag(event_flags, flag_id, info["base"])
    elif info["type"] == "block":
        return check_block_flag(event_flags, flag_id, info.get("base"))
    return None


def main():
    save_path = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"
    slot_index = 0

    if len(sys.argv) > 1:
        save_path = sys.argv[1]
    if len(sys.argv) > 2:
        slot_index = int(sys.argv[2])

    print(f"Loading save file: {save_path}")
    print(f"Slot index: {slot_index}")
    print()

    # Read slot data
    with open(save_path, 'rb') as f:
        f.seek(HEADER_SIZE + slot_index * SLOT_SIZE)
        slot_data = f.read(SLOT_SIZE)

    # Detect event_flags offset
    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    print(f"Detected event_flags offset: 0x{event_flags_offset:X}")
    event_flags = slot_data[event_flags_offset:]

    # Check all Rykard chain flags
    print()
    print("=" * 70)
    print("COMPLETE RYKARD EVENT CHAIN VERIFICATION")
    print("=" * 70)

    results = {}
    for flag_str, info in RYKARD_FLAGS.items():
        value = check_flag(event_flags, flag_str, info)
        results[flag_str] = value

        status = "SET" if value is True else "NOT SET" if value is False else "UNKNOWN"
        symbol = "✓" if value is True else "○" if value is False else "?"
        print(f"  {symbol} {flag_str:12} = {status:8}  ({info['desc']})")

    # Analysis
    print()
    print("=" * 70)
    print("CHAIN ANALYSIS")
    print("=" * 70)

    # Check consistency
    defeat_flag = results.get("16000800")
    progression_flag = results.get("9122")
    pickup_flag = results.get("510220")

    print()
    print("Expected chain states:")
    print("  1. Boss not encountered: All flags UNSET")
    print("  2. Boss defeated, remembrance not picked up:")
    print("     - 16000800 = SET, 9122 = SET (maybe), 510220 = NOT SET")
    print("  3. Boss defeated, remembrance collected:")
    print("     - 16000800 = SET, 9122 = SET, 510220 = SET")
    print()

    if defeat_flag is True:
        print("Status: RYKARD DEFEATED")
        if pickup_flag is True:
            print("  └── Remembrance COLLECTED")
        elif pickup_flag is False:
            print("  └── Remembrance NOT YET COLLECTED (still on ground)")
        else:
            print("  └── Remembrance status UNKNOWN (510220 not readable)")
    elif defeat_flag is False:
        print("Status: RYKARD NOT DEFEATED")
    else:
        print("Status: UNKNOWN (dungeon flag not readable)")


if __name__ == "__main__":
    main()
