#!/usr/bin/env python3
"""
Verify Area 12 (Underground - Siofra River, Ainsel River) at calculated base 15362.

Underground areas include:
- Siofra River (m12_01_00)
- Ainsel River (m12_02_00)
- Deeproot Depths (m12_03_00)
- Lake of Rot (m12_04_00)
- Nokron, Eternal City (m12_05_00)
- Nokstella, Eternal City (m12_07_00)
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

CANDIDATE_BASE = 15362
SECTION_SIZE = 1125

# Underground dungeon flags (12SSSCCC format)
# Section 01 = Siofra River
# Section 02 = Ainsel River
# etc.

UNDERGROUND_FLAGS = [
    # Siofra River (section 01)
    (12010800, "Ancestor Spirit (boss)", "boss"),
    (12010900, "Siofra River Bank (grace)", "grace"),
    (12010901, "Worshippers' Woods (grace)", "grace"),
    (12010902, "Below the Well (grace)", "grace"),

    # Ainsel River (section 02)
    (12020800, "Dragonkin Soldier of Nokstella (boss)", "boss"),
    (12020900, "Ainsel River Well Depths (grace)", "grace"),
    (12020901, "Ainsel River Sluice Gate (grace)", "grace"),
    (12020902, "Ainsel River Downstream (grace)", "grace"),

    # Deeproot Depths (section 03)
    (12030800, "Lichdragon Fortissax (boss)", "boss"),
    (12030900, "Deeproot Depths (grace)", "grace"),
    (12030901, "The Nameless Eternal City (grace)", "grace"),
    (12030902, "Across the Roots (grace)", "grace"),
    (12030903, "Prince of Death's Throne (grace)", "grace"),

    # Lake of Rot (section 04)
    (12040800, "Astel, Naturalborn of the Void (boss)", "boss"),
    (12040900, "Lake of Rot Shoreside (grace)", "grace"),
    (12040901, "Grand Cloister (grace)", "grace"),

    # Nokron (section 05)
    (12050800, "Mimic Tear (boss)", "boss"),
    (12050900, "Nokron, Eternal City (grace)", "grace"),
    (12050901, "Ancestral Woods (grace)", "grace"),
    (12050902, "Aqueduct-Facing Cliffs (grace)", "grace"),
    (12050903, "Night's Sacred Ground (grace)", "grace"),

    # Nokstella (section 07)
    (12070900, "Nokstella, Eternal City (grace)", "grace"),
    (12070901, "Nokstella Waterfall Basin (grace)", "grace"),
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
    print(f"VERIFY AREA 12 (UNDERGROUND) AT BASE {CANDIDATE_BASE}")
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

    # Check known underground flags
    print(f"\n{'='*80}")
    print(f"UNDERGROUND FLAGS AT BASE {CANDIDATE_BASE}")
    print("=" * 80)

    print(f"\n{'Flag ID':<12} {'Name':<40} {'Type':<6} {'S0':>6} {'S1':>6} {'Sec':>4} {'Loc':>4}")
    print("-" * 90)

    s0_set = []
    current_section = None

    for flag_id, name, flag_type in UNDERGROUND_FLAGS:
        result_s0 = check_dungeon_flag(ef_data_s0, CANDIDATE_BASE, flag_id)
        result_s1 = check_dungeon_flag(ef_data_s1, CANDIDATE_BASE, flag_id)

        is_set_s0, byte_off, bit_pos, byte_val, section, local = result_s0
        is_set_s1 = result_s1[0]

        # Print section header
        if section != current_section:
            current_section = section
            section_names = {
                1: "Siofra River",
                2: "Ainsel River",
                3: "Deeproot Depths",
                4: "Lake of Rot",
                5: "Nokron",
                7: "Nokstella"
            }
            print(f"\n--- Section {section:02d}: {section_names.get(section, 'Unknown')} ---")

        s0_status = "SET" if is_set_s0 else "unset"
        s1_status = "SET" if is_set_s1 else "unset"

        if is_set_s0:
            s0_set.append((flag_id, name, flag_type))

        print(f"{flag_id:<12} {name:<40} {flag_type:<6} {s0_status:>6} {s1_status:>6} {section:>4} {local:>4}")

    # Scan all sections for activity
    print(f"\n{'='*80}")
    print("SECTION ACTIVITY ANALYSIS")
    print("=" * 80)

    for section in range(0, 10):
        section_start = CANDIDATE_BASE + section * SECTION_SIZE

        if section_start + SECTION_SIZE > len(ef_data_s0):
            continue

        total_s0 = sum(bin(ef_data_s0[section_start + i]).count('1') for i in range(SECTION_SIZE))
        total_s1 = sum(bin(ef_data_s1[section_start + i]).count('1') for i in range(SECTION_SIZE))

        section_names = {
            0: "Section 00 (base)",
            1: "Siofra River",
            2: "Ainsel River",
            3: "Deeproot Depths",
            4: "Lake of Rot",
            5: "Nokron",
            6: "Section 06",
            7: "Nokstella",
            8: "Section 08",
            9: "Section 09"
        }

        if total_s0 > 0 or total_s1 > 0:
            print(f"\n  Section {section:02d} ({section_names.get(section, 'Unknown')}):")
            print(f"    S0: {total_s0} bits SET, S1: {total_s1} bits SET")
            print(f"    Density: S0={total_s0/(SECTION_SIZE*8)*100:.2f}%, S1={total_s1/(SECTION_SIZE*8)*100:.2f}%")

    # Analysis
    print(f"\n{'='*80}")
    print("ANALYSIS")
    print("=" * 80)

    if s0_set:
        print(f"\nFlags SET in Slot 0:")
        for flag_id, name, flag_type in s0_set:
            print(f"  {flag_id}: {name} ({flag_type})")
    else:
        print("\nNo expected underground flags SET in Slot 0")
        print("Character may not have explored underground areas")


if __name__ == "__main__":
    main()
