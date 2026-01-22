#!/usr/bin/env python3
"""
Investigate tile vs block contradictions.

Tile 1043500030 vs Block 67640 (Missionary's Cookbook [4])
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
BACKUP_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
SEARCH_START = 0x12000

# Tile formula constants (from ground_truth)
TILE_BASE = 489981
BYTES_PER_SLOT = 875
SLOTS_PER_ROW = 40
ROW_BASE = 33
COL_BASE = 30

# Block 67000 base
BLOCK_67000_BASE = 37411

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Contradictions to investigate
CONTRADICTIONS = [
    (1041530050, 67070, "Nomadic Warrior's Cookbook [19]"),
    (1043500030, 67640, "Missionary's Cookbook [4]"),
    (1044530210, 65120, "Unknown (65120)"),
    (1046400030, 67650, "Missionary's Cookbook [3]"),
    (1048360000, 68030, "Ancient Dragon Apostle's Cookbook [3]"),
    (1051360010, 67260, "Armorer's Cookbook [4]"),
]


def detect_event_flags_start(slot_data, search_start):
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - 0x1bf99f)

    for test_offset in range(search_start, search_end):
        positive_score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    positive_score += 1

        if positive_score == len(VALIDATION_FLAGS):
            return test_offset

    return 0x12B00


def read_slot_data(save_file, slot_index):
    with open(save_file, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)


def check_tile_flag(ef_data, tile_flag):
    """Check tile flag using tile formula."""
    tile_index = (tile_flag - 1_000_000_000) // 10000
    local_id = tile_flag % 10000

    row = tile_index // 100
    col = tile_index % 100

    slot = (row - ROW_BASE) * SLOTS_PER_ROW + (col - COL_BASE)
    byte_offset = TILE_BASE + slot * BYTES_PER_SLOT + local_id // 8
    bit_pos = 7 - (local_id % 8)

    if byte_offset < len(ef_data):
        byte_val = ef_data[byte_offset]
        is_set = bool(byte_val & (1 << bit_pos))
        return is_set, byte_offset, bit_pos, byte_val, row, col, slot, local_id
    return None, byte_offset, bit_pos, 0, row, col, slot, local_id


def check_block_flag(ef_data, block_flag, base=BLOCK_67000_BASE, block_start=67000):
    """Check block flag using block formula."""
    local = block_flag - block_start
    byte_offset = base + local // 8
    bit_pos = 7 - (block_flag % 8)

    if byte_offset < len(ef_data):
        byte_val = ef_data[byte_offset]
        is_set = bool(byte_val & (1 << bit_pos))
        return is_set, byte_offset, bit_pos, byte_val
    return None, byte_offset, bit_pos, 0


def main():
    print("=" * 80)
    print("INVESTIGATE TILE VS BLOCK CONTRADICTIONS")
    print("=" * 80)

    slot0_data = read_slot_data(BACKUP_FILE, 0)
    ef_start = detect_event_flags_start(slot0_data, SEARCH_START)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_data = slot0_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

    print(f"\nEventFlags start: 0x{ef_start:X}")

    for tile_flag, block_flag, name in CONTRADICTIONS:
        print(f"\n{'='*80}")
        print(f"ITEM: {name}")
        print(f"Tile flag: {tile_flag}, Block flag: {block_flag}")
        print("=" * 80)

        # Check tile flag
        tile_result = check_tile_flag(ef_data, tile_flag)
        tile_set, t_byte, t_bit, t_val, row, col, slot, local_id = tile_result

        print(f"\nTile flag {tile_flag}:")
        print(f"  Tile: ({row}, {col}), Slot: {slot}, LocalID: {local_id}")
        print(f"  Byte offset: {t_byte} (0x{t_byte:X})")
        print(f"  Bit position: {t_bit}")
        print(f"  Byte value: 0x{t_val:02X} ({t_val:08b})")
        print(f"  Flag SET: {tile_set}")

        # Check block flag
        if 67000 <= block_flag < 68000:
            base = BLOCK_67000_BASE
            block_start = 67000
        elif 68000 <= block_flag < 69000:
            base = 37536  # Block 68000 base
            block_start = 68000
        elif 65000 <= block_flag < 66000:
            base = None  # Unknown
            block_start = 65000
            print(f"\n⚠ Block 65000 base unknown - cannot verify")
            continue
        else:
            base = None
            print(f"\n⚠ Unknown block for flag {block_flag}")
            continue

        block_result = check_block_flag(ef_data, block_flag, base, block_start)
        block_set, b_byte, b_bit, b_val = block_result

        print(f"\nBlock flag {block_flag}:")
        print(f"  Base: {base}, Block start: {block_start}")
        print(f"  Byte offset: {b_byte} (0x{b_byte:X})")
        print(f"  Bit position: {b_bit}")
        print(f"  Byte value: 0x{b_val:02X} ({b_val:08b})")
        print(f"  Flag SET: {block_set}")

        # Show surrounding bytes for tile
        print(f"\nTile area bytes (offset {t_byte-2} to {t_byte+3}):")
        for i in range(-2, 4):
            off = t_byte + i
            if 0 <= off < len(ef_data):
                val = ef_data[off]
                mark = " <-- flag byte" if i == 0 else ""
                print(f"  Byte {off}: 0x{val:02X} ({val:08b}){mark}")

        # Analysis
        print(f"\nANALYSIS:")
        if block_set and not tile_set:
            print("  Block SET, Tile UNSET")
            print("  Possible causes:")
            print("    1. Item purchased from shop (no world pickup)")
            print("    2. Item from quest reward")
            print("    3. Tile formula error")
            print("    4. Tile bytes contain different data")
        elif not block_set and tile_set:
            print("  Block UNSET, Tile SET - UNEXPECTED")
            print("  Block formula likely wrong")
        else:
            print(f"  Block={block_set}, Tile={tile_set} - AGREEMENT")


if __name__ == "__main__":
    main()
