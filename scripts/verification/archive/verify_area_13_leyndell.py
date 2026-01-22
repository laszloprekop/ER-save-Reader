#!/usr/bin/env python3
"""
Verify Area 13 (Leyndell Royal Capital dungeon events) at calculated base 26612.

Note: This is different from Block 71100 (Leyndell graces at base 2593).
Area 13 contains dungeon events like boss fights, item pickups within the dungeon.
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

CANDIDATE_BASE = 26612
SECTION_SIZE = 1125

# Leyndell dungeon flags (13SSSCCC format)
# Section 00 = main Leyndell
LEYNDELL_FLAGS = [
    # Boss flags
    (13000800, "Morgott, the Omen King (boss)", "boss"),
    (13000850, "Godfrey, First Elden Lord (Golden Shade)", "boss"),

    # Common dungeon event flags
    (13000900, "Main gate opened", "event"),
    (13000001, "First entry event", "event"),

    # Item pickups are typically in lower ranges
    (13000100, "Item pickup 100", "item"),
    (13000200, "Item pickup 200", "item"),
    (13000300, "Item pickup 300", "item"),
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
    print(f"VERIFY AREA 13 (LEYNDELL DUNGEON) AT BASE {CANDIDATE_BASE}")
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

    # Show raw bytes at base
    print(f"\n{'='*80}")
    print(f"RAW BYTES AT BASE {CANDIDATE_BASE}")
    print("=" * 80)

    print("\nSlot 0 (first 30 bytes):")
    for i in range(30):
        byte_val = ef_data_s0[CANDIDATE_BASE + i]
        print(f"  Byte {CANDIDATE_BASE + i}: 0x{byte_val:02X} ({byte_val:08b})")

    print("\nSlot 1 (first 30 bytes):")
    for i in range(30):
        byte_val = ef_data_s1[CANDIDATE_BASE + i]
        print(f"  Byte {CANDIDATE_BASE + i}: 0x{byte_val:02X} ({byte_val:08b})")

    # Check known flags
    print(f"\n{'='*80}")
    print(f"LEYNDELL FLAGS AT BASE {CANDIDATE_BASE}")
    print("=" * 80)

    print(f"\n{'Flag ID':<12} {'Name':<45} {'S0':>6} {'S1':>6} {'Sec':>4} {'Loc':>4}")
    print("-" * 90)

    for flag_id, name, flag_type in LEYNDELL_FLAGS:
        result_s0 = check_dungeon_flag(ef_data_s0, CANDIDATE_BASE, flag_id)
        result_s1 = check_dungeon_flag(ef_data_s1, CANDIDATE_BASE, flag_id)

        is_set_s0, byte_off, bit_pos, byte_val, section, local = result_s0
        is_set_s1 = result_s1[0]

        s0_status = "SET" if is_set_s0 else "unset"
        s1_status = "SET" if is_set_s1 else "unset"

        print(f"{flag_id:<12} {name:<45} {s0_status:>6} {s1_status:>6} {section:>4} {local:>4}")

    # Scan section 0 for activity
    print(f"\n{'='*80}")
    print(f"SCAN SECTION 0 FOR SET FLAGS")
    print("=" * 80)

    s0_set_count = 0
    s1_set_count = 0
    s0_set_flags = []

    for local in range(1000):
        flag_id = 13000000 + local
        result_s0 = check_dungeon_flag(ef_data_s0, CANDIDATE_BASE, flag_id)
        result_s1 = check_dungeon_flag(ef_data_s1, CANDIDATE_BASE, flag_id)

        if result_s0[0]:
            s0_set_count += 1
            s0_set_flags.append((flag_id, local, result_s0[1], result_s0[2]))
        if result_s1[0]:
            s1_set_count += 1

    print(f"\nSection 0: S0={s0_set_count} flags, S1={s1_set_count} flags")

    if s0_set_flags:
        print("\nFirst 30 SET flags in S0:")
        for flag_id, local, byte_off, bit_pos in s0_set_flags[:30]:
            # Identify special flags
            marker = ""
            if local == 800:
                marker = " (Morgott boss)"
            elif local == 850:
                marker = " (Godfrey shade)"
            elif 900 <= local < 920:
                marker = " (grace/event)"

            print(f"  {flag_id} (local {local}): byte {byte_off}, bit {bit_pos}{marker}")

    # Sparsity analysis
    print(f"\n{'='*80}")
    print("SPARSITY ANALYSIS")
    print("=" * 80)

    total_s0 = sum(bin(ef_data_s0[CANDIDATE_BASE + i]).count('1') for i in range(SECTION_SIZE) if CANDIDATE_BASE + i < len(ef_data_s0))
    total_s1 = sum(bin(ef_data_s1[CANDIDATE_BASE + i]).count('1') for i in range(SECTION_SIZE) if CANDIDATE_BASE + i < len(ef_data_s1))

    print(f"\nTotal SET bits in section 0: S0={total_s0}, S1={total_s1}")
    print(f"Density: S0={total_s0/(SECTION_SIZE*8)*100:.2f}%, S1={total_s1/(SECTION_SIZE*8)*100:.2f}%")

    # Analysis
    print(f"\n{'='*80}")
    print("ANALYSIS")
    print("=" * 80)

    if total_s0 > total_s1 * 2:
        print(f"\n✓ Good differential: S0 has significantly more flags than S1")
        print("  Base likely correct for Leyndell dungeon events")
    elif total_s0 > total_s1:
        print(f"\n~ Partial differential: S0={total_s0}, S1={total_s1}")
    else:
        print(f"\n? Poor or inverted differential: S0={total_s0}, S1={total_s1}")


if __name__ == "__main__":
    main()
