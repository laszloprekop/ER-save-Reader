#!/usr/bin/env python3
"""
Cross-examine Confessor inventory for Godrick defeat evidence.
Check Great Rune possession, World Drop flag, and Remembrance flags.

Small flags (< 60000) formula: byte_offset = flag_id / 8, bit = 7 - (flag_id % 8)
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
BACKUP_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
SEARCH_START = 0x12000
EVENT_FLAGS_SIZE = 0x1bf99f

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

def detect_event_flags_start(slot_data, search_start):
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - EVENT_FLAGS_SIZE)

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

def calc_small_flag_offset(flag_id):
    """Calculate offset for small flags (< 60000)."""
    byte_offset = flag_id // 8
    bit_pos = 7 - (flag_id % 8)
    return byte_offset, bit_pos

def main():
    print("=" * 80)
    print("GODRICK DEFEAT - INVENTORY CROSS-EXAMINATION")
    print("=" * 80)

    # Godrick-related flags to check
    godrick_flags = [
        # Great Rune possession/activation
        (160, "Godrick's Great Rune - Possession"),
        (180, "Godrick's Great Rune - Activated"),
        # Boss defeat marker (world drop)
        (171, "Godrick the Grafted - World Drop"),
        # Remembrance possession
        (9101, "Remembrance of the Grafted"),
        # Comparison: Margit (no Great Rune, but common)
        (10000800, "Boss defeat flag (dungeon formula)"),
    ]

    for slot_idx in [0, 1]:
        slot_name = "Confessor (mid-game)" if slot_idx == 0 else "Wretch (early game)"
        print(f"\n{'='*80}")
        print(f"SLOT {slot_idx}: {slot_name}")
        print("=" * 80)

        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = detect_event_flags_start(slot_data, SEARCH_START)
        ef_data = slot_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        print(f"EF start: 0x{ef_start:X}")
        print("\n--- Godrick-related Flags ---\n")

        for flag_id, name in godrick_flags:
            if flag_id < 60000:
                # Small flag formula
                byte_offset, bit_pos = calc_small_flag_offset(flag_id)
                formula = "small"
            elif flag_id >= 10_000_000:
                # Dungeon flag - 10000800 uses dungeon formula
                # Area 10 (Stormveil), section 00, local 0800
                # base = 4112, local = 800, byte = 4112 + 800/8 = 4112 + 100 = 4212
                byte_offset = 4112 + 800 // 8
                bit_pos = 7 - (800 % 8)
                formula = "dungeon"
            else:
                continue

            if byte_offset < len(ef_data):
                byte_val = ef_data[byte_offset]
                is_set = bool(byte_val & (1 << bit_pos))
                status = "SET" if is_set else "UNSET"
                print(f"  Flag {flag_id:>8}: {name}")
                print(f"    Offset: {byte_offset} (0x{byte_offset:X}), bit {bit_pos}")
                print(f"    Byte value: 0x{byte_val:02X} ({byte_val:08b})")
                print(f"    Status: {status}")
                print()

    # Show bytes around the Great Rune area
    print("\n" + "=" * 80)
    print("RAW BYTES: GREAT RUNE/BOSS DEFEAT REGION (bytes 0-30)")
    print("=" * 80)

    for slot_idx in [0]:
        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = detect_event_flags_start(slot_data, SEARCH_START)
        ef_data = slot_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        print(f"\nSlot {slot_idx}:")
        for i in range(0, 30, 10):
            bytes_row = [ef_data[i + j] for j in range(10)]
            hex_row = " ".join(f"{b:02X}" for b in bytes_row)
            print(f"  Byte {i:3}-{i+9:3}: {hex_row}")

    # Check for Remembrance region
    print("\n" + "=" * 80)
    print("RAW BYTES: REMEMBRANCE REGION (bytes 1135-1145 for flag 9101)")
    print("=" * 80)

    for slot_idx in [0, 1]:
        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = detect_event_flags_start(slot_data, SEARCH_START)
        ef_data = slot_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        print(f"\nSlot {slot_idx}:")
        for i in range(1135, 1145):
            byte_val = ef_data[i]
            print(f"  Byte {i}: 0x{byte_val:02X} ({byte_val:08b})")

    print("\n" + "=" * 80)
    print("CONCLUSION")
    print("=" * 80)
    print("""
If Flag 160 (Great Rune Possession) is SET: Player has Godrick's Great Rune -> HAS DEFEATED GODRICK
If Flag 171 (World Drop) is SET: Player triggered boss drop -> HAS DEFEATED GODRICK
If Flag 9101 (Remembrance) is SET: Player picked up Remembrance -> HAS DEFEATED GODRICK

If all three are UNSET: Player has NOT defeated Godrick (matches 71000 grace being unset)
""")

if __name__ == "__main__":
    main()
