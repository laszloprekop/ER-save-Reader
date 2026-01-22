#!/usr/bin/env python3
"""
Verify Whetblade flags.
According to CLAUDE.md:
- 65000-65300: Whetblade pickups (ItemLotParam_map)
- 65610-65720: Whetblade shop unlocks (common.emevd.js Event 1450)
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

# Known Whetblades (from game knowledge)
WHETBLADES = [
    (65100, "Whetstone Knife", "Gatefront Ruins cellar"),
    (65110, "Iron Whetblade", "Stormveil Castle"),
    (65120, "Glintstone Whetblade", "Raya Lucaria"),
    (65130, "Red-Hot Whetblade", "Redmane Castle"),
    (65140, "Sanctified Whetblade", "Fortified Manor, Leyndell"),
    (65150, "Black Whetblade", "Night's Sacred Ground"),
]

# Crystal Tears (already verified in block 65000 at base 37412)
CRYSTAL_TEARS = [
    (65000, "Crimson Crystal Tear"),
    (65010, "Cerulean Crystal Tear"),
    (65020, "Greenspill Crystal Tear"),
    (65030, "Crimsonspill Crystal Tear"),
    (65040, "Opaline Hardtear"),
]

BLOCK_65000_BASE = 37412


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


def check_flag(ef_data, base, flag_id, block_start=65000):
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
    print("VERIFY WHETBLADE FLAGS")
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

    # Check Crystal Tears at base 37412
    print(f"\n{'='*80}")
    print(f"CRYSTAL TEARS AT BASE {BLOCK_65000_BASE}")
    print("=" * 80)

    print(f"\n{'Flag ID':<8} {'Name':<30} {'S0':>6} {'S1':>6} {'Byte':>8} {'Bit':>4}")
    print("-" * 70)

    for flag_id, name in CRYSTAL_TEARS:
        is_set_s0, byte_off, bit_pos, _ = check_flag(ef_data_s0, BLOCK_65000_BASE, flag_id)
        is_set_s1, _, _, _ = check_flag(ef_data_s1, BLOCK_65000_BASE, flag_id)

        s0_status = "SET" if is_set_s0 else "unset"
        s1_status = "SET" if is_set_s1 else "unset"

        print(f"{flag_id:<8} {name:<30} {s0_status:>6} {s1_status:>6} {byte_off:>8} {bit_pos:>4}")

    # Check Whetblades at base 37412
    print(f"\n{'='*80}")
    print(f"WHETBLADES AT BASE {BLOCK_65000_BASE}")
    print("=" * 80)

    print(f"\n{'Flag ID':<8} {'Name':<25} {'Location':<20} {'S0':>6} {'S1':>6} {'Byte':>8} {'Bit':>4}")
    print("-" * 90)

    s0_count = 0
    for flag_id, name, location in WHETBLADES:
        is_set_s0, byte_off, bit_pos, _ = check_flag(ef_data_s0, BLOCK_65000_BASE, flag_id)
        is_set_s1, _, _, _ = check_flag(ef_data_s1, BLOCK_65000_BASE, flag_id)

        s0_status = "SET" if is_set_s0 else "unset"
        s1_status = "SET" if is_set_s1 else "unset"

        if is_set_s0:
            s0_count += 1

        print(f"{flag_id:<8} {name:<25} {location:<20} {s0_status:>6} {s1_status:>6} {byte_off:>8} {bit_pos:>4}")

    print("-" * 90)
    print(f"Total SET in S0: {s0_count}/{len(WHETBLADES)}")

    # Search for better base for whetblades
    print(f"\n{'='*80}")
    print("SEARCHING FOR WHETBLADE BASE (0-50000)")
    print("=" * 80)

    best_bases = []
    for test_base in range(0, 50000):
        s0_count = 0
        s1_count = 0
        flags_set = []

        for flag_id, name, _ in WHETBLADES:
            is_set_s0, _, _, _ = check_flag(ef_data_s0, test_base, flag_id)
            is_set_s1, _, _, _ = check_flag(ef_data_s1, test_base, flag_id)

            if is_set_s0:
                s0_count += 1
                flags_set.append(flag_id)
            if is_set_s1:
                s1_count += 1

        # Good candidate: some flags in S0, none in S1
        if s0_count >= 2 and s1_count == 0:
            # Check byte value at base
            byte_val = ef_data_s0[test_base] if test_base < len(ef_data_s0) else 0
            if byte_val != 0xFF:  # Exclude 0xFF false positives
                best_bases.append((test_base, s0_count, s1_count, flags_set))

    if best_bases:
        print(f"\nBases with 2+ whetblades in S0 and 0 in S1:")
        for base, s0_cnt, s1_cnt, flags in sorted(best_bases, key=lambda x: -x[1])[:20]:
            print(f"  Base {base}: S0={s0_cnt}, S1={s1_cnt}")
            print(f"    Flags: {flags}")
    else:
        print("\nNo good candidates found - whetblades may not use standard block formula")

    # Check raw bytes at whetblade range
    print(f"\n{'='*80}")
    print("RAW BYTES AT BLOCK 65000 WHETBLADE RANGE")
    print("=" * 80)

    # Whetblades are at 65100-65150, which is local offset 100-150 from block start
    # At base 37412: byte offset = 37412 + 100//8 = 37412 + 12 = 37424
    whetblade_start = BLOCK_65000_BASE + 100 // 8
    print(f"\nWhetblade range starts at byte {whetblade_start}")
    print(f"\nSlot 0 bytes:")
    for i in range(10):
        byte_val = ef_data_s0[whetblade_start + i]
        print(f"  Byte {whetblade_start + i}: 0x{byte_val:02X} ({byte_val:08b})")

    print(f"\nSlot 1 bytes:")
    for i in range(10):
        byte_val = ef_data_s1[whetblade_start + i]
        print(f"  Byte {whetblade_start + i}: 0x{byte_val:02X} ({byte_val:08b})")


if __name__ == "__main__":
    main()
