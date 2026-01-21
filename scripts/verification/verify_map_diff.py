#!/usr/bin/env python3
"""
Verify map fragment base using before/after save snapshots.

Wretch save snapshots:
- Snapshot 33: Before map pickup (rested at Agheel Lake North)
- Snapshot 34: After map pickup (moved south of Wayward cellar)

If base 9359 is correct, flag 62010 (Map: Limgrave, West) should be:
- UNSET in snapshot 33
- SET in snapshot 34
"""

from typing import Optional

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
]

# Paths to before/after snapshots
BEFORE_PATH = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/ER0000.sl2 Wretch - 33 Limgrave, rested at Agheel Lake North grace, continue game"
AFTER_PATH = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/ER0000.sl2 Wretch - 34 Limgrave Map picked, moved to south of Wayward cellar sarchophagi"

# Map fragment to test
MAP_FLAG = 62010
MAP_NAME = "Map: Limgrave, West"
BLOCK_START = 62000
BASE = 9359  # Our discovered base


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


def load_slot_data(path: str, slot: int = 1) -> bytes:
    """Load event flags from a save file."""
    with open(path, 'rb') as f:
        f.seek(HEADER_SIZE + slot * SLOT_SIZE)
        return f.read(SLOT_SIZE)


def main():
    print("="*70)
    print("MAP FRAGMENT TEMPORAL VALIDATION")
    print("="*70)
    print(f"\nTesting flag {MAP_FLAG} ({MAP_NAME})")
    print(f"Using base {BASE} for block {BLOCK_START}")

    # Load before snapshot (Slot 1 = Wretch)
    print(f"\n--- BEFORE (Snapshot 33) ---")
    print(f"Path: {BEFORE_PATH[:60]}...")

    try:
        before_data = load_slot_data(BEFORE_PATH, slot=1)
        before_offset = detect_event_flags_offset(before_data)

        if before_offset is None:
            print("ERROR: Could not detect event_flags offset in BEFORE snapshot")
            return

        print(f"Event flags offset: 0x{before_offset:X}")
        before_ef = before_data[before_offset:]

        # Check the map flag
        before_state = check_flag(before_ef, MAP_FLAG, BLOCK_START, BASE)
        print(f"Flag {MAP_FLAG} state: {'SET' if before_state else 'UNSET'}")

        # Show raw byte
        relative = MAP_FLAG - BLOCK_START
        byte_offset = BASE + relative // 8
        bit_pos = 7 - (MAP_FLAG % 8)
        byte_val = before_ef[byte_offset]
        print(f"Raw byte {byte_offset}: 0x{byte_val:02X} ({byte_val:08b}), bit {bit_pos}")

    except FileNotFoundError:
        print("ERROR: BEFORE snapshot file not found")
        return

    # Load after snapshot
    print(f"\n--- AFTER (Snapshot 34) ---")
    print(f"Path: {AFTER_PATH[:60]}...")

    try:
        after_data = load_slot_data(AFTER_PATH, slot=1)
        after_offset = detect_event_flags_offset(after_data)

        if after_offset is None:
            print("ERROR: Could not detect event_flags offset in AFTER snapshot")
            return

        print(f"Event flags offset: 0x{after_offset:X}")
        after_ef = after_data[after_offset:]

        # Check the map flag
        after_state = check_flag(after_ef, MAP_FLAG, BLOCK_START, BASE)
        print(f"Flag {MAP_FLAG} state: {'SET' if after_state else 'UNSET'}")

        # Show raw byte
        byte_val = after_ef[byte_offset]
        print(f"Raw byte {byte_offset}: 0x{byte_val:02X} ({byte_val:08b}), bit {bit_pos}")

    except FileNotFoundError:
        print("ERROR: AFTER snapshot file not found")
        return

    # Verify the transition
    print("\n" + "="*70)
    print("VALIDATION RESULT")
    print("="*70)

    expected_before = False  # Should be UNSET before pickup
    expected_after = True    # Should be SET after pickup

    if before_state == expected_before and after_state == expected_after:
        print("✓ VERIFIED: Base 9359 is CORRECT for map fragments")
        print(f"  - Before pickup: {MAP_FLAG} was UNSET")
        print(f"  - After pickup:  {MAP_FLAG} is SET")
        print("  - Temporal diff confirms the base!")
    elif before_state == after_state:
        print("✗ INCONCLUSIVE: No change detected")
        print(f"  - Before: {'SET' if before_state else 'UNSET'}")
        print(f"  - After:  {'SET' if after_state else 'UNSET'}")
        print("  - The base may be wrong, or the flag wasn't affected")
    else:
        print("? UNEXPECTED: State changed but in wrong direction")
        print(f"  - Before: {'SET' if before_state else 'UNSET'} (expected UNSET)")
        print(f"  - After:  {'SET' if after_state else 'UNSET'} (expected SET)")

    # Also show byte diff
    print("\n--- BYTE DIFF ---")
    print(f"Byte {byte_offset}:")
    print(f"  Before: 0x{before_ef[byte_offset]:02X} ({before_ef[byte_offset]:08b})")
    print(f"  After:  0x{after_ef[byte_offset]:02X} ({after_ef[byte_offset]:08b})")

    # Show surrounding bytes for context
    print("\nSurrounding bytes (±5):")
    for i in range(-5, 6):
        b_off = byte_offset + i
        b_val = before_ef[b_off]
        a_val = after_ef[b_off]
        diff = " <-- CHANGED" if b_val != a_val else ""
        print(f"  {b_off}: 0x{b_val:02X} -> 0x{a_val:02X}{diff}")


if __name__ == "__main__":
    main()
