#!/usr/bin/env python3
"""
Find where the map flag bit changed between before/after snapshots.

Strategy: Compare the entire event_flags sections and find which bits changed.
The map pickup should cause exactly 1 bit to flip from 0 to 1.
"""

from typing import Optional, List, Tuple

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
]

BEFORE_PATH = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/ER0000.sl2 Wretch - 33 Limgrave, rested at Agheel Lake North grace, continue game"
AFTER_PATH = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/ER0000.sl2 Wretch - 34 Limgrave Map picked, moved to south of Wayward cellar sarchophagi"


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


def find_bit_changes(before_ef: bytes, after_ef: bytes, max_bytes: int = 100000) -> List[Tuple[int, int, int, int]]:
    """
    Find all bit changes between two event_flags sections.
    Returns list of (byte_offset, bit_position, before_bit, after_bit)
    """
    changes = []

    for i in range(min(len(before_ef), len(after_ef), max_bytes)):
        if before_ef[i] != after_ef[i]:
            before_byte = before_ef[i]
            after_byte = after_ef[i]

            for bit in range(8):
                before_bit = (before_byte >> bit) & 1
                after_bit = (after_byte >> bit) & 1
                if before_bit != after_bit:
                    changes.append((i, bit, before_bit, after_bit))

    return changes


def load_slot_data(path: str, slot: int = 1) -> bytes:
    with open(path, 'rb') as f:
        f.seek(HEADER_SIZE + slot * SLOT_SIZE)
        return f.read(SLOT_SIZE)


def main():
    print("="*70)
    print("FIND MAP BIT CHANGE BETWEEN SNAPSHOTS")
    print("="*70)

    # Load before snapshot
    print("\nLoading BEFORE snapshot...")
    before_data = load_slot_data(BEFORE_PATH, slot=1)
    before_offset = detect_event_flags_offset(before_data)
    print(f"Event flags offset: 0x{before_offset:X}")
    before_ef = before_data[before_offset:]

    # Load after snapshot
    print("\nLoading AFTER snapshot...")
    after_data = load_slot_data(AFTER_PATH, slot=1)
    after_offset = detect_event_flags_offset(after_data)
    print(f"Event flags offset: 0x{after_offset:X}")
    after_ef = after_data[after_offset:]

    # Note offset difference
    offset_diff = after_offset - before_offset
    print(f"\nOffset difference: {offset_diff} bytes")

    if offset_diff != 0:
        print("WARNING: Event flags offsets differ between snapshots!")
        print("This means the base offset calculation needs adjustment.")

    # Find all bit changes
    print("\n" + "="*70)
    print("BIT CHANGES (0 -> 1 = newly set flags)")
    print("="*70)

    changes = find_bit_changes(before_ef, after_ef)

    print(f"\nTotal bit changes: {len(changes)}")

    # Filter to bits that went 0->1 (newly set)
    new_sets = [(off, bit, bv, av) for off, bit, bv, av in changes if bv == 0 and av == 1]
    print(f"Bits set (0->1): {len(new_sets)}")

    # Show newly set bits
    print("\nNewly SET bits:")
    for byte_off, bit_pos, _, _ in new_sets:
        # Calculate what flag ID this could be for block 62000
        # If this is the map flag, we can derive the base
        # flag_id = block_start + (byte_off - base) * 8 + (7 - bit_pos)

        # Try different block starts
        for block_start in [62000, 63000, 65000, 67000]:
            # We expect the map flag to be 62010
            # So: 62010 = 62000 + (byte_off - base) * 8 + (7 - bit_pos)
            # 10 = (byte_off - base) * 8 + (7 - bit_pos)
            # For bit_pos=5: 10 = (byte_off - base) * 8 + 2, so byte_off - base = 1
            # So base = byte_off - 1

            potential_base = byte_off - (62010 - 62000) // 8
            expected_bit = 7 - ((62010 - 62000) % 8)

            if bit_pos == expected_bit:
                print(f"\n  Byte {byte_off}, bit {bit_pos}:")
                print(f"    If block=62000, base would be: {potential_base}")
                print(f"    This would make flag ID: 62010 (Map: Limgrave, West)")
            else:
                # Generic output
                print(f"\n  Byte {byte_off}, bit {bit_pos}:")

                # Calculate potential flag IDs for different bases
                for test_base in [9359, 1500, 2725, 3250]:
                    rel = (byte_off - test_base) * 8 + (7 - bit_pos)
                    if 0 <= rel < 1000:
                        flag_id = 62000 + rel
                        print(f"    If base={test_base}: flag {flag_id}")

    # Also check bits that went 1->0 (cleared)
    cleared = [(off, bit, bv, av) for off, bit, bv, av in changes if bv == 1 and av == 0]
    if cleared:
        print(f"\nCleared bits (1->0): {len(cleared)}")
        for byte_off, bit_pos, _, _ in cleared[:10]:
            print(f"  Byte {byte_off}, bit {bit_pos}")


if __name__ == "__main__":
    main()
