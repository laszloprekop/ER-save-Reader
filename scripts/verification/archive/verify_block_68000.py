#!/usr/bin/env python3
"""
Verify Block 68000 (Cookbooks continued) empirically.

Current calculated base: 37536 (67000 base 37411 + 125)
Status: Needs empirical verification
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

# Block 68000 cookbooks (base game only - DLC likely not collected)
COOKBOOKS_68000 = [
    (68000, "Ancient Dragon Apostle's Cookbook [1]"),
    (68010, "Ancient Dragon Apostle's Cookbook [2]"),
    (68020, "Ancient Dragon Apostle's Cookbook [4]"),
    (68030, "Ancient Dragon Apostle's Cookbook [3]"),
    (68200, "Fevor's Cookbook [1]"),
    (68210, "Fevor's Cookbook [3]"),
    (68220, "Fevor's Cookbook [2]"),
    (68230, "Missionary's Cookbook [7]"),
    (68400, "Frenzied's Cookbook [1]"),
    (68410, "Frenzied's Cookbook [2]"),
]

# Also check block 67000 for comparison (verified base 37411)
COOKBOOKS_67000_SAMPLE = [
    (67000, "Nomadic warrior's Cookbook [1]"),
    (67010, "Nomadic warrior's Cookbook [3]"),
    (67640, "Missionary's Cookbook [4]"),  # Known collected
]

CALCULATED_BASE_68000 = 37536  # Current calculated value
VERIFIED_BASE_67000 = 37411    # Verified working base


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


def check_flag(ef_data, base, flag_id, block_start):
    """Check flag using block formula."""
    local = flag_id - block_start
    byte_offset = base + local // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset < len(ef_data):
        byte_val = ef_data[byte_offset]
        return bool(byte_val & (1 << bit_pos)), byte_val, byte_offset, bit_pos
    return None, 0, byte_offset, bit_pos


def main():
    print("=" * 70)
    print("VERIFY BLOCK 68000 (COOKBOOKS CONTINUED)")
    print("=" * 70)

    slot0_data = read_slot_data(BACKUP_FILE, 0)
    slot1_data = read_slot_data(BACKUP_FILE, 1)

    ef_start_s0 = detect_event_flags_start(slot0_data, SEARCH_START)
    ef_start_s1 = detect_event_flags_start(slot1_data, SEARCH_START)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_data_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_data_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]

    print(f"\nSlot 0 EF start: 0x{ef_start_s0:X}")
    print(f"Slot 1 EF start: 0x{ef_start_s1:X}")

    # First verify Block 67000 works as expected
    print(f"\n{'='*70}")
    print(f"CONTROL CHECK: BLOCK 67000 AT BASE {VERIFIED_BASE_67000}")
    print("=" * 70)

    print(f"\n{'Flag ID':<8} {'Name':<40} {'S0':>6} {'S1':>6}")
    print("-" * 65)

    for flag_id, name in COOKBOOKS_67000_SAMPLE:
        is_set_s0, byte_val_s0, byte_off, bit_pos = check_flag(ef_data_s0, VERIFIED_BASE_67000, flag_id, 67000)
        is_set_s1, byte_val_s1, _, _ = check_flag(ef_data_s1, VERIFIED_BASE_67000, flag_id, 67000)

        s0_status = "SET" if is_set_s0 else "unset"
        s1_status = "SET" if is_set_s1 else "unset"
        print(f"{flag_id:<8} {name:<40} {s0_status:>6} {s1_status:>6}")

    # Now test Block 68000 at calculated base
    print(f"\n{'='*70}")
    print(f"TEST: BLOCK 68000 AT CALCULATED BASE {CALCULATED_BASE_68000}")
    print("=" * 70)

    print(f"\n{'Flag ID':<8} {'Name':<40} {'S0':>6} {'S1':>6} {'Byte':>8} {'Bit':>4}")
    print("-" * 75)

    s0_count = 0
    s1_count = 0

    for flag_id, name in COOKBOOKS_68000:
        is_set_s0, byte_val_s0, byte_off, bit_pos = check_flag(ef_data_s0, CALCULATED_BASE_68000, flag_id, 68000)
        is_set_s1, byte_val_s1, _, _ = check_flag(ef_data_s1, CALCULATED_BASE_68000, flag_id, 68000)

        s0_status = "SET" if is_set_s0 else "unset"
        s1_status = "SET" if is_set_s1 else "unset"

        if is_set_s0:
            s0_count += 1
        if is_set_s1:
            s1_count += 1

        print(f"{flag_id:<8} {name:<40} {s0_status:>6} {s1_status:>6} {byte_off:>8} {bit_pos:>4}")

    print("-" * 75)
    print(f"{'TOTAL':<8} {'':<40} {s0_count}/{len(COOKBOOKS_68000):>4} {s1_count}/{len(COOKBOOKS_68000):>4}")

    # Show raw bytes at base 37536
    print(f"\n{'='*70}")
    print(f"RAW BYTES AT BASE {CALCULATED_BASE_68000}")
    print("=" * 70)

    print("\nSlot 0:")
    for i in range(10):
        byte_val = ef_data_s0[CALCULATED_BASE_68000 + i]
        print(f"  Byte {CALCULATED_BASE_68000 + i}: 0x{byte_val:02X} ({byte_val:08b})")

    print("\nSlot 1:")
    for i in range(10):
        byte_val = ef_data_s1[CALCULATED_BASE_68000 + i]
        print(f"  Byte {CALCULATED_BASE_68000 + i}: 0x{byte_val:02X} ({byte_val:08b})")

    # Search for better base if current shows poor results
    if s0_count < 3:
        print(f"\n{'='*70}")
        print("SEARCHING FOR BETTER BASE")
        print("=" * 70)

        best_bases = []
        for test_base in range(35000, 40000):
            set_count = 0
            flags_set = []
            for flag_id, name in COOKBOOKS_68000:
                is_set, _, _, _ = check_flag(ef_data_s0, test_base, flag_id, 68000)
                if is_set:
                    set_count += 1
                    flags_set.append(flag_id)

            if set_count >= 3:
                # Check Slot 1 differential
                s1_set = 0
                for flag_id, _ in COOKBOOKS_68000:
                    is_set, _, _, _ = check_flag(ef_data_s1, test_base, flag_id, 68000)
                    if is_set:
                        s1_set += 1

                if s1_set == 0:  # Good differential
                    byte0 = ef_data_s0[test_base]
                    if byte0 != 0xFF:  # Exclude false positives
                        best_bases.append((test_base, set_count, s1_set, flags_set))

        if best_bases:
            print(f"\nBases with 3+ flags SET in S0 and 0 in S1:")
            for base, count, s1, flags in sorted(best_bases, key=lambda x: -x[1])[:10]:
                print(f"  Base {base}: S0={count}/{len(COOKBOOKS_68000)}, S1={s1}")
                print(f"    Flags: {flags}")
        else:
            print("\nNo better base found with good differential")

    # Verification summary
    print(f"\n{'='*70}")
    print("VERIFICATION SUMMARY")
    print("=" * 70)

    if s0_count >= 3 and s1_count == 0:
        print(f"\n✓ Block 68000 base {CALCULATED_BASE_68000} shows good differential pattern")
        print(f"  Slot 0: {s0_count}/{len(COOKBOOKS_68000)} SET")
        print(f"  Slot 1: {s1_count}/{len(COOKBOOKS_68000)} SET (expected for early-game)")
    elif s0_count == 0 and s1_count == 0:
        print(f"\n? Both slots show 0 flags SET - may need different base or no 68xxx cookbooks collected")
    else:
        print(f"\n⚠ Unexpected pattern: S0={s0_count}, S1={s1_count}")


if __name__ == "__main__":
    main()
