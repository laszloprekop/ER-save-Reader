#!/usr/bin/env python3
"""
Check Godrick boss defeat flag (10000800) to explain why flag 71000 is unset.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
BACKUP_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
SEARCH_START = 0x12000

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Boss defeat flag from dungeon formula
# Flag 10000800 uses dungeon formula: map 10 (Stormveil), area 00, flag 0800
# Dungeon formula: (map_id, inner_area, local_flag) -> offset in EF data
# For flag 10000800: local = 0800 (2048 decimal)
# From event_flags.rs: (10000800,(0x151c33,7)) - absolute offset 0x151c33 = 1383475

GODRICK_DEFEAT_FLAG = 10000800
GODRICK_OFFSET_FROM_RS = 0x151c33  # From event_flags.rs

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

def main():
    print("=" * 70)
    print("GODRICK BOSS DEFEAT FLAG CHECK")
    print("=" * 70)

    for slot_idx in [0, 1]:
        print(f"\n--- SLOT {slot_idx} ---")
        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = detect_event_flags_start(slot_data, SEARCH_START)

        EVENT_FLAGS_SIZE = 0x1bf99f
        ef_data = slot_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        print(f"EF start: 0x{ef_start:X}")

        # Check Godrick defeat flag at the RS-defined offset
        # The offset 0x151c33 is relative to EF start
        godrick_byte_offset = GODRICK_OFFSET_FROM_RS
        godrick_bit = 7

        if godrick_byte_offset < len(ef_data):
            byte_val = ef_data[godrick_byte_offset]
            is_set = bool(byte_val & (1 << godrick_bit))
            print(f"\nGodrick defeat flag (10000800):")
            print(f"  Offset: 0x{godrick_byte_offset:X} ({godrick_byte_offset}), bit {godrick_bit}")
            print(f"  Byte value: 0x{byte_val:02X} ({byte_val:08b})")
            print(f"  Flag status: {'SET - GODRICK DEFEATED' if is_set else 'UNSET - GODRICK NOT DEFEATED'}")
        else:
            print(f"  Offset {godrick_byte_offset} out of range (EF size: {len(ef_data)})")

        # Also check Stormveil graces for context
        print(f"\nStormveil graces (Block 71000, base 9315):")
        STORMVEIL_GRACES = [
            (71000, "Godrick the Grafted"),
            (71001, "Margit, the Fell Omen"),
            (71008, "Stormveil Main Gate"),
        ]

        for flag_id, name in STORMVEIL_GRACES:
            local = flag_id - 71000
            byte_offset = 9315 + local // 8
            bit_pos = 7 - (local % 8)

            if byte_offset < len(ef_data):
                byte_val = ef_data[byte_offset]
                is_set = bool(byte_val & (1 << bit_pos))
                print(f"  {flag_id} {name}: {'SET' if is_set else 'unset'}")

    print("\n" + "=" * 70)
    print("CONCLUSION")
    print("=" * 70)
    print("""
If Godrick defeat flag (10000800) is UNSET:
  -> Flag 71000 (Godrick grace) being UNSET is EXPECTED
  -> The grace doesn't spawn until Godrick is defeated

If Godrick defeat flag is SET but 71000 is UNSET:
  -> Player defeated Godrick but never rested at the grace
  -> This is possible but unusual
""")

if __name__ == "__main__":
    main()
