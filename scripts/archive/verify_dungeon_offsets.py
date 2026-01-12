#!/usr/bin/env python3
"""
Empirical verification of dungeon event flag offsets.

This script searches for the correct base offset for legacy dungeon flags
by scanning actual save file data for known collected items.

Usage:
    python3 scripts/verify_dungeon_offsets.py <save_file.sl2> <slot_index>

The script will:
1. Parse the save file structure
2. Extract the event flags array
3. Search for the correct dungeon base offset
4. Verify against multiple known flags
"""

import sys
import struct
from pathlib import Path

# Known dungeon flags that should be set for characters with Stormveil progress
# Format: (flag_id, item_name, expected_for_confessor)
STORMVEIL_TEST_FLAGS = [
    (10007990, "Godskin Prayerbook", True),  # User confirmed collected
    (10007005, "Highland Axe", False),  # Early Stormveil
    (10007030, "Furlcalling Finger Remedy", False),
    (10007040, "Fire Grease", False),
    (10007110, "Golden Rune [1]", False),
    (10007200, "Throwing Dagger", False),
    (10007430, "Arrow x10", False),
    (10007550, "Arbalest", False),
]

# Short flags that use BLOCK_BASES (verified working)
SHORT_FLAGS = [
    (65610, "Iron Whetblade"),  # Stormveil
    (67030, "Nomadic Warrior's Cookbook [10]"),  # Stormveil
    (71800, "Cave of Knowledge Grace"),  # Tutorial
    (76100, "The First Step Grace"),
    (76101, "Church of Elleh Grace"),
]

# Current dungeon base offset (to verify)
CURRENT_DUNGEON_BASE = 1383375
SECTION_SIZE = 1125
EVENT_FLAGS_SIZE = 0x1BF99F  # 1,833,375 bytes


def parse_flag_id(flag_id: int) -> tuple:
    """Parse an 8-digit dungeon flag into components."""
    flag_str = f"{flag_id:08d}"
    area = flag_str[0:2]
    section = flag_str[2:4]
    local = int(flag_str[4:8])
    return area, section, local


def calc_offset_with_base(flag_id: int, base_offset: int) -> tuple:
    """Calculate byte offset and bit position for a dungeon flag."""
    _, _, local_id = parse_flag_id(flag_id)
    byte_offset = base_offset + local_id // 8
    bit_pos = 7 - (flag_id % 8)
    return byte_offset, bit_pos


def check_flag(event_flags: bytes, byte_offset: int, bit_pos: int) -> bool:
    """Check if a flag is set at the given offset and bit position."""
    if byte_offset >= len(event_flags):
        return False
    return (event_flags[byte_offset] & (1 << bit_pos)) != 0


def check_block_flag(event_flags: bytes, flag_id: int) -> bool:
    """Check a short flag using BLOCK_BASES formula."""
    BLOCK_BASES = {
        65000: 1875,
        67000: 2125,
        71000: 2625,
        76000: 3250,
    }
    block_start = (flag_id // 1000) * 1000
    base = BLOCK_BASES.get(block_start)
    if base is None:
        return False
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)
    return check_flag(event_flags, byte_offset, bit_pos)


def find_dungeon_base_offset(event_flags: bytes, known_flags: list) -> list:
    """
    Search for the correct dungeon base offset by scanning the event flags.

    For flag 10007990 (Godskin Prayerbook), the local offset is 7990.
    If the flag is set, we can calculate: base = byte_where_set - (7990 // 8)
    """
    results = []

    # For Godskin Prayerbook (flag 10007990):
    # local_id = 7990
    # byte_within_section = 7990 // 8 = 998
    # bit_pos = 7 - (7990 % 8) = 7 - 6 = 1

    target_flag = 10007990
    _, _, local_id = parse_flag_id(target_flag)
    byte_within_section = local_id // 8  # 998
    bit_pos = 7 - (target_flag % 8)  # 1

    print(f"\nSearching for base offset where flag {target_flag} is set...")
    print(f"  Local ID: {local_id}")
    print(f"  Byte within section: {byte_within_section}")
    print(f"  Bit position: {bit_pos}")

    # Search through possible base offsets
    candidates = []
    for base in range(0, EVENT_FLAGS_SIZE - 2000, 1):  # Step by 1 for thorough search
        byte_offset = base + byte_within_section
        if byte_offset >= len(event_flags):
            continue

        if check_flag(event_flags, byte_offset, bit_pos):
            candidates.append(base)

    print(f"\nFound {len(candidates)} potential base offsets where flag appears set")

    if len(candidates) > 100:
        print("  (Too many candidates - filtering by alignment)")
        # Filter by likely alignment (multiples of SECTION_SIZE or common values)
        candidates = [c for c in candidates if c % 125 == 0 or c % 1125 == 0]
        print(f"  Filtered to {len(candidates)} aligned candidates")

    # Show candidates near the current assumed base
    nearby = [c for c in candidates if abs(c - CURRENT_DUNGEON_BASE) < 50000]
    if nearby:
        print(f"\n  Candidates near current base ({CURRENT_DUNGEON_BASE}):")
        for c in nearby[:10]:
            diff = c - CURRENT_DUNGEON_BASE
            print(f"    {c} (diff: {diff:+d})")

    return candidates


def extract_event_flags_from_sl2(filepath: Path, slot_index: int) -> bytes:
    """
    Extract event flags from an SL2 save file.

    This is a simplified parser - may need adjustment based on actual format.
    """
    with open(filepath, 'rb') as f:
        data = f.read()

    print(f"Save file size: {len(data)} bytes")

    # Check for BND4 header
    if data[:4] == b'BND4':
        print("Detected BND4 container format")

        # BND4 structure:
        # 0x00: "BND4"
        # 0x04: Unknown (usually 0)
        # 0x08: File count (1)
        # 0x0C: Header size (0x40)
        # ... header continues
        # After header: file entries, then data

        # For Elden Ring SL2, the structure is complex
        # Let me try to find the event flags by searching for patterns

        # The event flags section is ~1.8MB and contains many set bits
        # We can try to identify it by its size and content

        # For now, try a heuristic: look for a large section that matches expected patterns
        # The validation flags should be detectable

        # Search for the validation pattern (grace flags)
        # Grace 76100 should be at offset 3262 from section start, bit 3
        # If we find multiple hits at consistent offsets, we found the section

        print("\nSearching for event flags section using grace flag validation...")

        for search_start in range(0, len(data) - EVENT_FLAGS_SIZE, 0x1000):
            # Check if validation flags match at this offset
            matches = 0
            for flag_id, byte_off, bit_pos, name in [
                (71800, 2725, 7, "Cave of Knowledge"),
                (76100, 3262, 3, "The First Step"),
                (76101, 3262, 2, "Church of Elleh"),
            ]:
                abs_pos = search_start + byte_off
                if abs_pos < len(data):
                    if (data[abs_pos] & (1 << bit_pos)) != 0:
                        matches += 1

            if matches >= 2:
                print(f"  Found potential event flags at offset 0x{search_start:X} ({matches}/3 validation flags)")

                # Extract this section
                if search_start + EVENT_FLAGS_SIZE <= len(data):
                    return data[search_start:search_start + EVENT_FLAGS_SIZE]

        print("  Could not locate event flags section!")
        return b''

    print("Unknown save format")
    return b''


def main():
    if len(sys.argv) < 2:
        print("Usage: python3 verify_dungeon_offsets.py <save_file.sl2> [slot_index]")
        print("\nThis script verifies dungeon event flag offsets against actual save data.")
        sys.exit(1)

    save_path = Path(sys.argv[1])
    slot_index = int(sys.argv[2]) if len(sys.argv) > 2 else 0

    if not save_path.exists():
        print(f"Error: Save file not found: {save_path}")
        sys.exit(1)

    print(f"Analyzing save file: {save_path}")
    print(f"Slot index: {slot_index}")
    print()

    # Extract event flags
    event_flags = extract_event_flags_from_sl2(save_path, slot_index)

    if not event_flags:
        print("\nFailed to extract event flags. Try with a different save file.")
        sys.exit(1)

    print(f"\nExtracted {len(event_flags)} bytes of event flags")

    # First verify that short flags work (these use BLOCK_BASES)
    print("\n=== Verifying SHORT flags (BLOCK_BASES) ===")
    for flag_id, name in SHORT_FLAGS:
        is_set = check_block_flag(event_flags, flag_id)
        status = "[X]" if is_set else "[ ]"
        print(f"  {status} {flag_id}: {name}")

    # Check current dungeon flag calculation
    print(f"\n=== Testing DUNGEON flags with current base ({CURRENT_DUNGEON_BASE}) ===")
    for flag_id, name, _ in STORMVEIL_TEST_FLAGS:
        byte_off, bit_pos = calc_offset_with_base(flag_id, CURRENT_DUNGEON_BASE)
        is_set = check_flag(event_flags, byte_off, bit_pos)
        status = "[X]" if is_set else "[ ]"
        print(f"  {status} {flag_id}: {name} (byte={byte_off}, bit={bit_pos})")

    # Search for correct base offset
    print("\n=== Searching for correct dungeon base offset ===")
    candidates = find_dungeon_base_offset(event_flags, STORMVEIL_TEST_FLAGS)

    # If we found candidates, test them
    if candidates:
        print("\n=== Testing top candidate base offsets ===")
        for candidate_base in candidates[:5]:
            print(f"\n  Testing base offset {candidate_base}:")
            matches = 0
            for flag_id, name, expected in STORMVEIL_TEST_FLAGS:
                byte_off, bit_pos = calc_offset_with_base(flag_id, candidate_base)
                is_set = check_flag(event_flags, byte_off, bit_pos)
                status = "[X]" if is_set else "[ ]"
                print(f"    {status} {flag_id}: {name}")
                if is_set:
                    matches += 1
            print(f"  Matched {matches}/{len(STORMVEIL_TEST_FLAGS)} flags")


if __name__ == '__main__':
    main()
