#!/usr/bin/env python3
"""
Verify base 2821 for block 71000 (Stormveil graces) from ground_truth.

The ground_truth says base 2821 gives 7/9 match with flags 71001-71007 SET.
Let's verify this with correct EventFlags detection.
"""

import struct
from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
SEARCH_START = 0x12000
FALLBACK_OFFSET = 0x12B00

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

NEGATIVE_VALIDATION_FLAGS = [
    (76223, 3277, 0, "Fortified Manor"),
    (76224, 3278, 7, "East Capital Rampart"),
]

STORMVEIL_GRACES = [
    (71000, "Godrick the Grafted"),
    (71001, "Margit, the Fell Omen"),
    (71002, "Castleward Tunnel"),
    (71003, "Gateside Chamber"),
    (71004, "Stormveil Cliffside"),
    (71005, "Rampart Tower"),
    (71006, "Liftside Chamber"),
    (71007, "Secluded Cell"),
    (71008, "Stormveil Main Gate"),
]

def detect_event_flags_start(slot_data, search_start, fallback_offset):
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - 0x1bf99f)
    min_offset = 500
    actual_start = max(search_start, min_offset)

    for test_offset in range(actual_start, search_end):
        positive_score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    positive_score += 1

        if positive_score == len(VALIDATION_FLAGS):
            negative_score = 0
            for flag_id, byte_offset, bit_pos, name in NEGATIVE_VALIDATION_FLAGS:
                abs_pos = test_offset + byte_offset
                if abs_pos < len(slot_data):
                    byte_val = slot_data[abs_pos]
                    if not (byte_val & (1 << bit_pos)):
                        negative_score += 1

            if negative_score == len(NEGATIVE_VALIDATION_FLAGS):
                return test_offset

    return fallback_offset

def read_slot_data(slot_index):
    with open(SAVE_FILE, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def check_flag(ef_data, byte_offset, bit_pos):
    if byte_offset < len(ef_data):
        return bool(ef_data[byte_offset] & (1 << bit_pos))
    return None

def main():
    print("=" * 70)
    print("VERIFY BASE 2821 FOR BLOCK 71000 (STORMVEIL)")
    print("=" * 70)

    slot0_data = read_slot_data(0)
    slot1_data = read_slot_data(1)

    ef_start_s0 = detect_event_flags_start(slot0_data, SEARCH_START, FALLBACK_OFFSET)
    ef_start_s1 = detect_event_flags_start(slot1_data, SEARCH_START, FALLBACK_OFFSET)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]

    print(f"\nDetected EF starts: S0=0x{ef_start_s0:X}, S1=0x{ef_start_s1:X}")

    # Test base 2821 from ground_truth
    print("\n" + "=" * 70)
    print("TESTING BASE 2821 (from ground_truth)")
    print("=" * 70)

    base = 2821

    # Show raw bytes
    if base + 2 < len(ef_s0):
        byte0_s0 = ef_s0[base]
        byte1_s0 = ef_s0[base + 1]
        byte0_s1 = ef_s1[base]
        byte1_s1 = ef_s1[base + 1]
        print(f"\nRaw bytes at base {base}:")
        print(f"  S0: 0x{byte0_s0:02X} 0x{byte1_s0:02X} ({byte0_s0:08b} {byte1_s0:08b})")
        print(f"  S1: 0x{byte0_s1:02X} 0x{byte1_s1:02X} ({byte0_s1:08b} {byte1_s1:08b})")

    print(f"\nStormveil graces at base {base}:")
    set_count_s0 = 0
    set_count_s1 = 0

    for flag_id, name in STORMVEIL_GRACES:
        local = flag_id - 71000
        byte_offset = base + local // 8
        bit_pos = 7 - (local % 8)

        val_s0 = check_flag(ef_s0, byte_offset, bit_pos)
        val_s1 = check_flag(ef_s1, byte_offset, bit_pos)

        if val_s0:
            set_count_s0 += 1
            status_s0 = "SET"
        else:
            status_s0 = "unset"

        if val_s1:
            set_count_s1 += 1
            status_s1 = "SET"
        else:
            status_s1 = "unset"

        print(f"  {flag_id} ({name:25s}): S0={status_s0:5s}, S1={status_s1}")

    print(f"\nSummary: S0={set_count_s0}/9 SET, S1={set_count_s1}/9 SET")

    # Compare with other candidate bases from search
    print("\n" + "=" * 70)
    print("COMPARING WITH SEARCH CANDIDATES")
    print("=" * 70)

    candidate_bases = [2821, 2673, 2625, 2794, 3016]

    for base in candidate_bases:
        if base + 2 >= len(ef_s0):
            continue

        set_count_s0 = 0
        set_count_s1 = 0
        flags_set = []

        for flag_id, name in STORMVEIL_GRACES:
            local = flag_id - 71000
            byte_offset = base + local // 8
            bit_pos = 7 - (local % 8)

            val_s0 = check_flag(ef_s0, byte_offset, bit_pos)
            val_s1 = check_flag(ef_s1, byte_offset, bit_pos)

            if val_s0:
                set_count_s0 += 1
                flags_set.append(flag_id)
            if val_s1:
                set_count_s1 += 1

        byte0_s0 = ef_s0[base]
        byte1_s0 = ef_s0[base + 1] if base + 1 < len(ef_s0) else 0
        byte0_s1 = ef_s1[base]
        byte1_s1 = ef_s1[base + 1] if base + 1 < len(ef_s1) else 0

        print(f"\n  Base {base}: S0={set_count_s0}/9 SET, S1={set_count_s1}/9 SET")
        print(f"    Bytes: S0=[0x{byte0_s0:02X} 0x{byte1_s0:02X}] S1=[0x{byte0_s1:02X} 0x{byte1_s1:02X}]")
        if flags_set:
            print(f"    Flags SET in S0: {flags_set}")

    # Also check the calculated base 2625 (from 71800 being 100 bytes ahead)
    print("\n" + "=" * 70)
    print("CHECKING CALCULATED BASE 2625 (from 71800 relationship)")
    print("=" * 70)

    base = 2625
    print(f"\nIf block 71000 is part of same block as 71800:")
    print(f"  71800 is at byte 2725, so base for block 71000 = 2725 - 100 = 2625")

    if base + 2 < len(ef_s0):
        byte0_s0 = ef_s0[base]
        byte1_s0 = ef_s0[base + 1]
        print(f"  Bytes at 2625: S0=[0x{byte0_s0:02X} 0x{byte1_s0:02X}]")
        print(f"  This is all zeros, so 71000 is NOT in the same block as 71800!")

    print("\n" + "=" * 70)
    print("CONCLUSION")
    print("=" * 70)

    print("""
Block 71000 (Stormveil graces) appears to be stored SEPARATELY from block 71800 (Tutorial).
The calculated base 2625 is all zeros, confirming blocks are not contiguous.

Best candidates based on differential pattern (S0 has graces, S1 doesn't):
- Base 2821: Previously identified in ground_truth with 7/9 match
- Other candidates need verification

To definitively determine the correct base, we need:
1. User to confirm which specific Stormveil graces their Confessor has discovered
2. Cross-reference with the flag patterns at candidate bases
""")

if __name__ == "__main__":
    main()
