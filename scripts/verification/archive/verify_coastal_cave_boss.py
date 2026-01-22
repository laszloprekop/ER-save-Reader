#!/usr/bin/env python3
"""
Verify Coastal Cave boss defeat (31010800) in Slot 3.
The progression analysis showed this flag as SET in Slot 3 but UNSET elsewhere.
This could be a true positive if V2 character defeated the Beastman of Farum Azula.

Area 31 = Caves
Section 01 = Coastal Cave
Flag 31010800 = Beastman of Farum Azula boss defeat
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

# Area 31 (Caves) base from ground_truth.rs: 28634
CAVES_BASE = 28634
SECTION_SIZE = 1125

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

def calc_dungeon_flag_offset(flag_id, base_offset, section_size=1125):
    """Calculate offset for dungeon flag given a base."""
    section = (flag_id // 10_000) % 100
    local_id = flag_id % 10_000
    byte_offset = base_offset + section * section_size + local_id // 8
    bit_pos = 7 - (local_id % 8)
    return byte_offset, bit_pos

def main():
    print("=" * 80)
    print("VERIFY COASTAL CAVE BOSS DEFEAT (31010800)")
    print("=" * 80)
    print(f"\nArea 31 (Caves) base: {CAVES_BASE}")
    print("Flag 31010800 = Beastman of Farum Azula boss defeat (Coastal Cave)")

    # Calculate offset
    byte_off, bit_pos = calc_dungeon_flag_offset(31010800, CAVES_BASE)
    print(f"\nFlag 31010800 calculation:")
    print(f"  Section: 01")
    print(f"  Local ID: 0800")
    print(f"  Byte offset: {CAVES_BASE} + 1 * {SECTION_SIZE} + 800 / 8 = {byte_off}")
    print(f"  Bit: {bit_pos}")

    print("\n" + "=" * 80)
    print("CHECK FLAG ACROSS ALL SLOTS")
    print("=" * 80)

    slot_names = ["Confessor (mid)", "Wretch (early)", "V1", "V2 (different path)", "V3"]

    for slot_idx in range(5):
        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = detect_event_flags_start(slot_data, SEARCH_START)
        ef_data = slot_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        byte_val = ef_data[byte_off] if byte_off < len(ef_data) else 0
        is_set = bool(byte_val & (1 << bit_pos))

        print(f"\nSlot {slot_idx} ({slot_names[slot_idx]}):")
        print(f"  EF start: 0x{ef_start:X}")
        print(f"  Byte at {byte_off}: 0x{byte_val:02X} ({byte_val:08b})")
        print(f"  Flag 31010800: {'SET - BOSS DEFEATED' if is_set else 'UNSET'}")

    # Show surrounding bytes for Slot 3 to verify this isn't padding
    print("\n" + "=" * 80)
    print("ANALYZE RAW BYTES AROUND OFFSET (SLOT 3)")
    print("=" * 80)

    slot3_data = read_slot_data(BACKUP_FILE, 3)
    ef_start = detect_event_flags_start(slot3_data, SEARCH_START)
    ef_data = slot3_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

    print(f"\nSlot 3 (V2), EF start 0x{ef_start:X}:")
    print(f"Bytes around offset {byte_off}:")
    start = max(0, byte_off - 5)
    end = min(len(ef_data), byte_off + 15)
    for i in range(start, end):
        marker = " <-- FLAG" if i == byte_off else ""
        print(f"  Byte {i}: 0x{ef_data[i]:02X} ({ef_data[i]:08b}){marker}")

    # Check other cave boss flags for corroboration
    print("\n" + "=" * 80)
    print("CHECK OTHER CAVE BOSS FLAGS FOR CORROBORATION")
    print("=" * 80)

    cave_bosses = [
        (31000800, "Murkwater Cave - Patches"),
        (31010800, "Coastal Cave - Beastman"),
        (31020800, "Groveside Cave - Beastman"),
        (31030800, "Stillwater Cave - Cleanrot Knight"),
        (31040800, "Lakeside Crystal Cave - Bloodhound Knight"),
        (31050800, "Academy Crystal Cave - Crystalians"),
        (31060800, "Seethewater Cave - Kindred of Rot"),
        (31070800, "Volcano Cave - Demi-Human Queen"),
    ]

    print("\nSlot 3 (V2) Cave Boss Flags:")
    for flag_id, name in cave_bosses:
        byte_off, bit_pos = calc_dungeon_flag_offset(flag_id, CAVES_BASE)
        byte_val = ef_data[byte_off] if byte_off < len(ef_data) else 0
        is_set = bool(byte_val & (1 << bit_pos))
        status = "SET" if is_set else "unset"
        print(f"  {flag_id} ({name}): {status}")

    print("\n" + "=" * 80)
    print("CONCLUSION")
    print("=" * 80)
    print("""
If ONLY flag 31010800 (Coastal Cave) is SET in Slot 3:
  -> Likely a true positive (V2 character defeated Beastman)
  -> Validates Area 31 base offset (28634)

If multiple cave bosses are SET:
  -> Check if progression matches what V2 character would have done
  -> Could indicate formula issues if too many are set
""")

if __name__ == "__main__":
    main()
