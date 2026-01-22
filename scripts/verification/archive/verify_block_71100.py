#!/usr/bin/env python3
"""
Verify Block 71100 (Leyndell graces) at candidate base 2705.
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

# All Leyndell graces (71100-71199)
LEYNDELL_GRACES = [
    (71100, "Elden Throne"),
    (71101, "Erdtree Sanctuary"),
    (71102, "East Capital Rampart"),
    (71103, "Lower Capital Church"),
    (71104, "Avenue Balcony"),
    (71105, "West Capital Rampart"),
    # 71106 missing
    (71107, "Queen's Bedchamber"),
    (71108, "Fortified Manor, First Floor"),
    (71109, "Divine Bridge"),  # <-- Teleport trap destination
    (71110, "Morgott, the Omen King"),  # Post-boss grace
    # Ashen Capital variants (71120-71125)
    (71120, "Elden Throne (Ashen)"),
    (71121, "Erdtree Sanctuary (Ashen)"),
    (71122, "East Capital Rampart (Ashen)"),
    (71123, "Leyndell, Capital of Ash"),
    (71124, "Queen's Bedchamber (Ashen)"),
    (71125, "Divine Bridge (Ashen)"),
    # Roundtable Hold
    (71190, "Table of Lost Grace"),
]

CANDIDATE_BASE = 2705


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


def check_flag(ef_data, base, flag_id, block_start=71100):
    """Check flag using block formula."""
    local = flag_id - block_start
    byte_offset = base + local // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset < len(ef_data):
        byte_val = ef_data[byte_offset]
        is_set = bool(byte_val & (1 << bit_pos))
        return is_set, byte_offset, bit_pos, byte_val
    return None, byte_offset, bit_pos, 0


def main():
    print("=" * 80)
    print("VERIFY BLOCK 71100 (LEYNDELL GRACES) AT CANDIDATE BASE 2705")
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

    # Show raw bytes at candidate base
    print(f"\n{'='*80}")
    print(f"RAW BYTES AT BASE {CANDIDATE_BASE}")
    print("=" * 80)

    print("\nSlot 0:")
    for i in range(15):
        byte_val = ef_data_s0[CANDIDATE_BASE + i]
        print(f"  Byte {CANDIDATE_BASE + i}: 0x{byte_val:02X} ({byte_val:08b})")

    print("\nSlot 1:")
    for i in range(15):
        byte_val = ef_data_s1[CANDIDATE_BASE + i]
        print(f"  Byte {CANDIDATE_BASE + i}: 0x{byte_val:02X} ({byte_val:08b})")

    # Check each Leyndell grace
    print(f"\n{'='*80}")
    print(f"LEYNDELL GRACES AT BASE {CANDIDATE_BASE}")
    print("=" * 80)

    print(f"\n{'Flag ID':<8} {'Name':<35} {'S0':>6} {'S1':>6} {'Byte':>6} {'Bit':>4}")
    print("-" * 70)

    s0_count = 0
    s1_count = 0
    s0_set_flags = []

    for flag_id, name in LEYNDELL_GRACES:
        is_set_s0, byte_off, bit_pos, byte_val_s0 = check_flag(ef_data_s0, CANDIDATE_BASE, flag_id)
        is_set_s1, _, _, byte_val_s1 = check_flag(ef_data_s1, CANDIDATE_BASE, flag_id)

        s0_status = "SET" if is_set_s0 else "unset"
        s1_status = "SET" if is_set_s1 else "unset"

        if is_set_s0:
            s0_count += 1
            s0_set_flags.append((flag_id, name))
        if is_set_s1:
            s1_count += 1

        # Highlight important graces
        highlight = ""
        if flag_id == 71109:
            highlight = " <-- Divine Bridge (teleport trap)"
        elif flag_id == 71190:
            highlight = " <-- Roundtable Hold"
        elif flag_id == 71110:
            highlight = " <-- Post-Morgott"

        print(f"{flag_id:<8} {name:<35} {s0_status:>6} {s1_status:>6} {byte_off:>6} {bit_pos:>4}{highlight}")

    print("-" * 70)
    print(f"{'TOTAL':<8} {'':<35} {s0_count}/{len(LEYNDELL_GRACES):>4} {s1_count}/{len(LEYNDELL_GRACES):>4}")

    # Search for better base
    print(f"\n{'='*80}")
    print("SEARCHING FOR BEST BASE (0-5000)")
    print("=" * 80)

    best_bases = []
    for test_base in range(0, 5000):
        set_count = 0
        flags_set = []
        for flag_id, name in LEYNDELL_GRACES:
            is_set, _, _, _ = check_flag(ef_data_s0, test_base, flag_id)
            if is_set:
                set_count += 1
                flags_set.append(flag_id)

        # Check Slot 1 (should have minimal graces for early-game)
        s1_count = 0
        for flag_id, _ in LEYNDELL_GRACES:
            is_set, _, _, _ = check_flag(ef_data_s1, test_base, flag_id)
            if is_set:
                s1_count += 1

        # Good candidate: some flags in S0, few/none in S1
        if set_count >= 1 and s1_count <= 1:
            byte0 = ef_data_s0[test_base]
            # Exclude 0xFF false positives
            if byte0 != 0xFF:
                best_bases.append((test_base, set_count, s1_count, flags_set))

    if best_bases:
        print(f"\nBases with 1+ graces in S0 and <=1 in S1:")
        for base, s0_cnt, s1_cnt, flags in sorted(best_bases, key=lambda x: -x[1])[:15]:
            print(f"  Base {base}: S0={s0_cnt}, S1={s1_cnt}")
            print(f"    Flags: {flags}")

    # Analysis
    print(f"\n{'='*80}")
    print("ANALYSIS")
    print("=" * 80)

    if s0_set_flags:
        print(f"\nFlags SET in Slot 0 at base {CANDIDATE_BASE}:")
        for flag_id, name in s0_set_flags:
            print(f"  {flag_id}: {name}")

        # Check for expected patterns
        has_divine_bridge = any(f == 71109 for f, _ in s0_set_flags)
        has_roundtable = any(f == 71190 for f, _ in s0_set_flags)

        if has_divine_bridge:
            print("\n✓ Divine Bridge (71109) is SET - matches teleport trap access")
        if has_roundtable:
            print("✓ Table of Lost Grace (71190) is SET - player has Roundtable Hold")
    else:
        print(f"\nNo flags SET at base {CANDIDATE_BASE}")


if __name__ == "__main__":
    main()
