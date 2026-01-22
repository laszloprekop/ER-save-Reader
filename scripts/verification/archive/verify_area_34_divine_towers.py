#!/usr/bin/env python3
"""
Verify Area 34 (Divine Towers) at calculated base 60362.

Divine Towers in the game:
- Divine Tower of Limgrave (Godrick's Rune)
- Divine Tower of Liurnia (Rennala - no rune)
- Divine Tower of Caelid (Radahn's Rune)
- Divine Tower of West Altus (Rykard's Rune)
- Divine Tower of East Altus (Morgott/Mohg's Rune)
- Isolated Divine Tower (Malenia's Rune)
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

CANDIDATE_BASE = 60362
SECTION_SIZE = 1125

# Divine Tower flags (34SSSCCC format)
# Sections 10-15 are allocated for Divine Towers
DIVINE_TOWER_FLAGS = [
    # Section 10 - Divine Tower of Limgrave
    (34100800, "Divine Tower of Limgrave (completion?)", "boss"),
    (34100900, "Divine Tower of Limgrave (grace)", "grace"),

    # Section 11 - Divine Tower of Liurnia
    (34110800, "Divine Tower of Liurnia (completion?)", "boss"),
    (34110900, "Divine Tower of Liurnia (grace)", "grace"),

    # Section 12 - Divine Tower of Caelid
    (34120800, "Divine Tower of Caelid (completion?)", "boss"),
    (34120900, "Divine Tower of Caelid (grace)", "grace"),

    # Section 13 - Divine Tower of West Altus
    (34130800, "Divine Tower of West Altus (completion?)", "boss"),
    (34130900, "Divine Tower of West Altus (grace)", "grace"),

    # Section 14 - Divine Tower of East Altus
    (34140800, "Divine Tower of East Altus (completion?)", "boss"),
    (34140900, "Divine Tower of East Altus (grace)", "grace"),

    # Section 15 - Isolated Divine Tower
    (34150800, "Isolated Divine Tower (completion?)", "boss"),
    (34150900, "Isolated Divine Tower (grace)", "grace"),
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


def check_dungeon_flag(ef_data, base, flag_id):
    """Check dungeon flag using section formula."""
    area = flag_id // 1000000
    section = (flag_id % 1000000) // 1000
    local = flag_id % 1000

    byte_offset = base + section * SECTION_SIZE + local // 8
    bit_pos = 7 - (local % 8)

    if byte_offset < len(ef_data):
        byte_val = ef_data[byte_offset]
        is_set = bool(byte_val & (1 << bit_pos))
        return is_set, byte_offset, bit_pos, byte_val, section, local
    return None, byte_offset, bit_pos, 0, section, local


def main():
    print("=" * 80)
    print(f"VERIFY AREA 34 (DIVINE TOWERS) AT BASE {CANDIDATE_BASE}")
    print("=" * 80)

    slot0_data = read_slot_data(BACKUP_FILE, 0)
    slot1_data = read_slot_data(BACKUP_FILE, 1)

    ef_start_s0 = detect_event_flags_start(slot0_data, SEARCH_START)
    ef_start_s1 = detect_event_flags_start(slot1_data, SEARCH_START)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_data_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_data_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]

    print(f"\nSlot 0 EF start: 0x{ef_start_s0:X}")
    print(f"Slot 1 EF start: 0x{ef_start_s1:X}")

    # Check known flags
    print(f"\n{'='*80}")
    print(f"DIVINE TOWER FLAGS AT BASE {CANDIDATE_BASE}")
    print("=" * 80)

    print(f"\n{'Flag ID':<12} {'Name':<45} {'S0':>6} {'S1':>6} {'Sec':>4} {'Byte':>8}")
    print("-" * 90)

    for flag_id, name, flag_type in DIVINE_TOWER_FLAGS:
        result_s0 = check_dungeon_flag(ef_data_s0, CANDIDATE_BASE, flag_id)
        result_s1 = check_dungeon_flag(ef_data_s1, CANDIDATE_BASE, flag_id)

        is_set_s0, byte_off, bit_pos, byte_val, section, local = result_s0
        is_set_s1 = result_s1[0]

        s0_status = "SET" if is_set_s0 else "unset"
        s1_status = "SET" if is_set_s1 else "unset"

        print(f"{flag_id:<12} {name:<45} {s0_status:>6} {s1_status:>6} {section:>4} {byte_off:>8}")

    # Scan sections 10-15 for activity
    print(f"\n{'='*80}")
    print("SECTION ACTIVITY ANALYSIS (Sections 10-15)")
    print("=" * 80)

    for section in range(10, 16):
        section_start = CANDIDATE_BASE + section * SECTION_SIZE

        if section_start + SECTION_SIZE > len(ef_data_s0):
            print(f"\nSection {section}: OUT OF RANGE (offset {section_start})")
            continue

        total_s0 = sum(bin(ef_data_s0[section_start + i]).count('1') for i in range(SECTION_SIZE))
        total_s1 = sum(bin(ef_data_s1[section_start + i]).count('1') for i in range(SECTION_SIZE))

        tower_names = {
            10: "Divine Tower of Limgrave",
            11: "Divine Tower of Liurnia",
            12: "Divine Tower of Caelid",
            13: "Divine Tower of West Altus",
            14: "Divine Tower of East Altus",
            15: "Isolated Divine Tower"
        }

        print(f"\nSection {section} ({tower_names.get(section, 'Unknown')}):")
        print(f"  Offset: {section_start}")
        print(f"  S0: {total_s0} bits SET, S1: {total_s1} bits SET")

        if total_s0 > 0:
            # Show first few set flags in this section
            set_flags = []
            for local in range(1000):
                byte_off = section_start + local // 8
                bit_pos = 7 - (local % 8)
                if ef_data_s0[byte_off] & (1 << bit_pos):
                    set_flags.append(local)
            if set_flags:
                print(f"  Set locals (first 10): {set_flags[:10]}")

    # Analysis
    print(f"\n{'='*80}")
    print("ANALYSIS")
    print("=" * 80)


if __name__ == "__main__":
    main()
