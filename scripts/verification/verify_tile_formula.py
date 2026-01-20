#!/usr/bin/env python3
"""
Verify tile formula by correctly calculating absolute file offsets.

The key insight: The tile formula gives an offset WITHIN the event_flags section.
To get the absolute file offset, we need:
    absolute_offset = slot_start + event_flags_offset_in_slot + formula_offset

For PC saves:
- Header: 0x300 bytes
- Slot 0 starts at: 0x310
- Each slot: 0x280020 bytes (includes checksum)
- Event flags offset in slot: ~0x12B00 (dynamic, detected via validation flags)
- Event flags size: 0x1bf99f bytes
"""

import sys
import struct

# Constants
HEADER_SIZE = 0x310  # After header, first slot data starts
SLOT_SIZE = 0x280020  # Total slot size including checksum
SLOT_DATA_SIZE = 0x280000  # Slot data without checksum
EVENT_FLAGS_OFFSET_DEFAULT = 0x12B00  # Default offset within slot

# Tile formula constants
# CORRECTED: Old value 485330 was wrong by 4651 bytes
# Derived from Smoldering Butterfly pickup: actual_offset=857482, tile_offset=367501
TILE_BASE_OFFSET = 489981
TILE_BYTES_PER_SLOT = 875
TILE_SLOTS_PER_ROW = 40
TILE_ROW_BASE = 33
TILE_COL_BASE = 30

# Validation flags (known offsets within event_flags)
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]


def detect_event_flags_offset(slot_data: bytes, search_start: int = 0x12000) -> int:
    """Detect event flags offset using validation flags."""
    best_offset = EVENT_FLAGS_OFFSET_DEFAULT
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

    return best_offset


def parse_10digit_flag(flag_id: int) -> tuple:
    """Parse a 10-digit flag into (row, col, local_id)."""
    flag_str = str(flag_id)
    if len(flag_str) != 10:
        return None
    row = int(flag_str[2:4])
    col = int(flag_str[4:6])
    local_id = int(flag_str[6:])
    return (row, col, local_id)


def calculate_tile_offset(flag_id: int) -> tuple:
    """Calculate offset within event_flags using tile formula."""
    parsed = parse_10digit_flag(flag_id)
    if parsed is None:
        return None

    row, col, local_id = parsed

    # Calculate tile slot
    slot = (row - TILE_ROW_BASE) * TILE_SLOTS_PER_ROW + (col - TILE_COL_BASE)
    tile_offset = slot * TILE_BYTES_PER_SLOT

    # Final offset within event_flags
    byte_offset = TILE_BASE_OFFSET + tile_offset + (local_id // 8)
    bit_position = 7 - (local_id % 8)

    return (byte_offset, bit_position, row, col, local_id, slot, tile_offset)


def get_slot_data(save_data: bytes, slot_index: int) -> bytes:
    """Extract slot data for a given slot index."""
    slot_start = HEADER_SIZE + (slot_index * SLOT_SIZE)
    slot_end = slot_start + SLOT_DATA_SIZE
    return save_data[slot_start:slot_end]


def check_flag_in_slot(save_data: bytes, slot_index: int, flag_id: int):
    """Check if a specific flag is set in a slot."""
    slot_data = get_slot_data(save_data, slot_index)
    ef_offset = detect_event_flags_offset(slot_data)
    ef_size = 0x1bf99f
    event_flags = slot_data[ef_offset:ef_offset + ef_size]

    result = calculate_tile_offset(flag_id)
    if result:
        byte_offset, bit_pos, row, col, local_id, slot, tile_offset = result
        if byte_offset < len(event_flags):
            byte_val = event_flags[byte_offset]
            is_set = (byte_val & (1 << bit_pos)) != 0
            return is_set, byte_val
    return None, None


def main():
    if len(sys.argv) < 4:
        print("Usage: python verify_tile_formula.py <before.sl2> <after.sl2> <slot_index> [flag_id]")
        print("       python verify_tile_formula.py --check-all <save.sl2> <flag_id>")
        print("\nExample: python verify_tile_formula.py before.sl2 after.sl2 4 1044360310")
        sys.exit(1)

    # Check-all mode: verify flag across all slots in a single save
    if sys.argv[1] == "--check-all":
        save_path = sys.argv[2]
        flag_id = int(sys.argv[3])

        with open(save_path, 'rb') as f:
            save_data = f.read()

        result = calculate_tile_offset(flag_id)
        if result:
            byte_offset, bit_pos, row, col, local_id, slot, tile_offset = result
            print(f"Flag {flag_id}: row={row}, col={col}, local_id={local_id}")
            print(f"  Expected: offset={byte_offset}, bit={bit_pos}\n")

        # First, check validation flags work in all slots
        print("Validation flags check:")
        for slot_idx in range(5):
            slot_data = get_slot_data(save_data, slot_idx)
            if len(slot_data) < 0x100:
                continue
            ef_offset = detect_event_flags_offset(slot_data)
            ef_size = 0x1bf99f
            event_flags = slot_data[ef_offset:ef_offset + ef_size]

            # Check The First Step (76100)
            val_byte = event_flags[3262] if 3262 < len(event_flags) else 0
            first_step = (val_byte & (1 << 3)) != 0
            print(f"  Slot {slot_idx}: The First Step={first_step}, ef_offset=0x{ef_offset:X}, ef_size={len(event_flags)}")

        print(f"\nTarget flag {flag_id}:")
        for slot_idx in range(10):
            slot_data = get_slot_data(save_data, slot_idx)
            if len(slot_data) < 0x100:
                continue
            ef_offset = detect_event_flags_offset(slot_data)
            is_set, byte_val = check_flag_in_slot(save_data, slot_idx, flag_id)
            if is_set is not None:
                print(f"  Slot {slot_idx}: is_set={is_set}, byte=0x{byte_val:02X}, ef_offset=0x{ef_offset:X} ({ef_offset})")
        return

    before_path = sys.argv[1]
    after_path = sys.argv[2]
    slot_index = int(sys.argv[3])

    # Load save files
    print(f"Loading save files...")
    with open(before_path, 'rb') as f:
        before_data = f.read()
    with open(after_path, 'rb') as f:
        after_data = f.read()

    # Extract slot data
    before_slot = get_slot_data(before_data, slot_index)
    after_slot = get_slot_data(after_data, slot_index)

    # Detect event flags offset
    before_ef_offset = detect_event_flags_offset(before_slot)
    after_ef_offset = detect_event_flags_offset(after_slot)

    print(f"\nSlot {slot_index} Analysis:")
    print(f"  Before event_flags offset: 0x{before_ef_offset:X} ({before_ef_offset})")
    print(f"  After event_flags offset:  0x{after_ef_offset:X} ({after_ef_offset})")

    if before_ef_offset != after_ef_offset:
        print(f"  WARNING: Event flags offsets differ!")

    # Extract event flags
    ef_size = 0x1bf99f  # 1,833,375 bytes
    before_ef = before_slot[before_ef_offset:before_ef_offset + ef_size]
    after_ef = after_slot[after_ef_offset:after_ef_offset + ef_size]

    print(f"\n  Event flags size: {len(before_ef)} bytes")

    # Verify validation flags are set
    print(f"\nValidation Flags Check:")
    for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
        before_byte = before_ef[byte_offset] if byte_offset < len(before_ef) else 0
        after_byte = after_ef[byte_offset] if byte_offset < len(after_ef) else 0
        before_bit = (before_byte & (1 << bit_pos)) != 0
        after_bit = (after_byte & (1 << bit_pos)) != 0
        print(f"  {name} ({flag_id}): before={before_bit}, after={after_bit}")

    # Test specific flag if provided
    if len(sys.argv) >= 5:
        flag_id = int(sys.argv[4])
        result = calculate_tile_offset(flag_id)

        if result:
            byte_offset, bit_pos, row, col, local_id, slot, tile_offset = result

            print(f"\nFlag {flag_id} Analysis:")
            print(f"  Parsed: row={row}, col={col}, local_id={local_id}")
            print(f"  Tile slot: {slot} (row-33={row-33}, col-30={col-30})")
            print(f"  Tile offset: {tile_offset}")
            print(f"  Final offset in event_flags: {byte_offset} (0x{byte_offset:X})")
            print(f"  Bit position: {bit_pos}")

            if byte_offset < len(before_ef):
                before_byte = before_ef[byte_offset]
                after_byte = after_ef[byte_offset]
                before_bit = (before_byte & (1 << bit_pos)) != 0
                after_bit = (after_byte & (1 << bit_pos)) != 0

                print(f"\n  Before: byte=0x{before_byte:02X} ({before_byte:08b}), bit {bit_pos}={before_bit}")
                print(f"  After:  byte=0x{after_byte:02X} ({after_byte:08b}), bit {bit_pos}={after_bit}")

                if not before_bit and after_bit:
                    print(f"\n  ✓ SUCCESS: Flag changed from 0 to 1!")
                elif before_bit and after_bit:
                    print(f"\n  - Flag was already set in both")
                elif not before_bit and not after_bit:
                    print(f"\n  ✗ FAIL: Flag is 0 in both files!")

                    # Search for any byte change in a region around the expected offset
                    print(f"\n  Searching for changes around expected offset...")
                    search_range = 100
                    found_changes = []
                    for i in range(max(0, byte_offset - search_range), min(len(before_ef), byte_offset + search_range)):
                        if before_ef[i] != after_ef[i]:
                            found_changes.append((i, before_ef[i], after_ef[i]))

                    if found_changes:
                        print(f"  Found {len(found_changes)} changed bytes nearby:")
                        for offset, before, after in found_changes[:10]:
                            delta = offset - byte_offset
                            print(f"    Offset {offset} (delta={delta:+d}): 0x{before:02X} -> 0x{after:02X}")
                    else:
                        print(f"  No changes found within {search_range} bytes of expected offset")
            else:
                print(f"\n  ERROR: Offset {byte_offset} exceeds event_flags size {len(before_ef)}")
        else:
            print(f"\nError: Could not parse flag {flag_id}")

    # Find all changes in event flags
    print(f"\n--- Scanning for ALL changes in event_flags ---")
    changes = []
    for i in range(min(len(before_ef), len(after_ef))):
        if before_ef[i] != after_ef[i]:
            changes.append((i, before_ef[i], after_ef[i]))

    print(f"Total bytes changed in event_flags: {len(changes)}")

    # Find bits that went from 0->1 (flag was SET)
    set_bits = []
    for offset, before, after in changes:
        changed = before ^ after
        for bit in range(8):
            if (changed & (1 << bit)) != 0:
                was_set = (before & (1 << bit)) != 0
                now_set = (after & (1 << bit)) != 0
                if not was_set and now_set:
                    set_bits.append((offset, bit))

    print(f"  Bits that went from 0->1 (flags SET): {len(set_bits)}")

    # Filter to high offsets (tile region starts around 489981)
    tile_region_bits = [(o, b) for o, b in set_bits if o > 400000]
    if tile_region_bits:
        print(f"  In tile region (>400000): {len(tile_region_bits)}")
        for offset, bit in tile_region_bits[:30]:
            print(f"    Offset {offset} (0x{offset:X}), bit {bit}")
    else:
        print(f"  No SET bits found in tile region (>400000)")
        print(f"  First 10 SET bits overall:")
        for offset, bit in set_bits[:10]:
            print(f"    Offset {offset} (0x{offset:X}), bit {bit}")


if __name__ == "__main__":
    main()
