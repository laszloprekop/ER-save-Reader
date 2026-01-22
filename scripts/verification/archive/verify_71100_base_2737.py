#!/usr/bin/env python3
"""
Verify base 2737 for Block 71100 (Leyndell graces).

Search results showed base 2737 gives S0=2/10 SET, S1=0/10 SET.
Let's see which specific graces are SET.
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

# All Leyndell graces from BonfireWarpParam
LEYNDELL_GRACES = [
    (71100, "Elden Throne"),
    (71101, "Erdtree Sanctuary"),
    (71102, "East Capital Rampart"),
    (71103, "Lower Capital Church"),
    (71104, "Avenue Balcony"),
    (71105, "Fortified Manor, First Floor"),
    (71106, "Queen's Bedchamber"),
    (71107, "Divine Bridge"),
    (71108, "West Capital Rampart"),
    (71109, "Divine Bridge (2)"),  # May be different name
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
    print("VERIFY BASE 2737 FOR BLOCK 71100 (LEYNDELL)")
    print("=" * 70)

    slot0_data = read_slot_data(0)
    slot1_data = read_slot_data(1)

    ef_start_s0 = detect_event_flags_start(slot0_data, SEARCH_START, FALLBACK_OFFSET)
    ef_start_s1 = detect_event_flags_start(slot1_data, SEARCH_START, FALLBACK_OFFSET)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]

    print(f"\nDetected EF starts: S0=0x{ef_start_s0:X}, S1=0x{ef_start_s1:X}")

    # Test base 2737
    base = 2737
    print(f"\n" + "=" * 70)
    print(f"LEYNDELL GRACES AT BASE {base}")
    print("=" * 70)

    # Show raw bytes
    print(f"\nRaw bytes at base {base}:")
    for i in range(3):
        byte_s0 = ef_s0[base + i] if base + i < len(ef_s0) else 0
        byte_s1 = ef_s1[base + i] if base + i < len(ef_s1) else 0
        print(f"  Byte {base + i}: S0=0x{byte_s0:02X} ({byte_s0:08b}), S1=0x{byte_s1:02X} ({byte_s1:08b})")

    print(f"\nLeyndell grace flags:")
    set_count_s0 = 0
    set_count_s1 = 0
    flags_set_s0 = []

    for flag_id, name in LEYNDELL_GRACES:
        local = flag_id - 71100
        byte_offset = base + local // 8
        bit_pos = 7 - (local % 8)

        val_s0 = check_flag(ef_s0, byte_offset, bit_pos)
        val_s1 = check_flag(ef_s1, byte_offset, bit_pos)

        if val_s0:
            set_count_s0 += 1
            flags_set_s0.append(name)
            status_s0 = "SET"
        else:
            status_s0 = "unset"

        if val_s1:
            set_count_s1 += 1
            status_s1 = "SET"
        else:
            status_s1 = "unset"

        print(f"  {flag_id} ({name:30s}): S0={status_s0:5s}, S1={status_s1}")

    print(f"\nSummary: S0={set_count_s0}/10 SET, S1={set_count_s1}/10 SET")
    if flags_set_s0:
        print(f"Graces SET in S0: {flags_set_s0}")

    # Divine Bridge is accessible via teleport trap in Tower of Return
    # Check if that specific grace is SET
    print("\n" + "=" * 70)
    print("DIVINE BRIDGE CHECK")
    print("=" * 70)

    print("""
Divine Bridge (71107) is accessible early via teleport trap in Tower of Return.
If only Divine Bridge-related graces are SET, this confirms:
1. The Confessor used the teleport trap
2. Base 2737 is correct for Block 71100
""")

    # Also search for alternate bases that show similar pattern
    print("\n" + "=" * 70)
    print("SEARCHING FOR ALTERNATE BASES")
    print("=" * 70)

    print("\nLooking for bases where exactly 1-3 flags are SET in S0, 0 in S1:")

    for test_base in range(2700, 2800):
        if test_base + 2 >= len(ef_s0):
            continue

        set_s0 = 0
        set_s1 = 0
        flags_set = []

        for flag_id, name in LEYNDELL_GRACES:
            local = flag_id - 71100
            byte_offset = test_base + local // 8
            bit_pos = 7 - (local % 8)

            if check_flag(ef_s0, byte_offset, bit_pos):
                set_s0 += 1
                flags_set.append(flag_id)
            if check_flag(ef_s1, byte_offset, bit_pos):
                set_s1 += 1

        if 1 <= set_s0 <= 3 and set_s1 == 0:
            byte0 = ef_s0[test_base]
            byte1 = ef_s0[test_base + 1] if test_base + 1 < len(ef_s0) else 0
            print(f"  Base {test_base}: S0={set_s0}/10 SET, S1={set_s1}/10 SET")
            print(f"    Bytes: 0x{byte0:02X} 0x{byte1:02X}")
            print(f"    Flags SET: {flags_set}")

if __name__ == "__main__":
    main()
