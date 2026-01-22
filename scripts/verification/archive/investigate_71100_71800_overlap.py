#!/usr/bin/env python3
"""
Investigate Block 71100 and 71800 overlap.

Ground truth shows both blocks at base 2725, but they cover different flag ranges.
Let's verify the actual structure.

Block 71100: Leyndell graces (71100-71199)
Block 71800: Tutorial graces (71800-71999)
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
]

# Tutorial graces (71800-71899)
TUTORIAL_GRACES = [
    (71800, "Cave of Knowledge"),
    (71801, "Stranded Graveyard"),
]

# Leyndell graces (71100-71199)
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
    (71109, "Divine Bridge (accessible)"),
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
    print("INVESTIGATE BLOCK 71100 AND 71800 OVERLAP")
    print("=" * 70)

    slot0_data = read_slot_data(0)
    slot1_data = read_slot_data(1)

    ef_start_s0 = detect_event_flags_start(slot0_data, SEARCH_START, FALLBACK_OFFSET)
    ef_start_s1 = detect_event_flags_start(slot1_data, SEARCH_START, FALLBACK_OFFSET)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]

    print(f"\nDetected EF starts: S0=0x{ef_start_s0:X}, S1=0x{ef_start_s1:X}")

    # Check Block 71800 (Tutorial) - verified at base 2725
    print("\n" + "=" * 70)
    print("BLOCK 71800 (Tutorial) - Base 2725")
    print("=" * 70)

    base_71800 = 2725
    for flag_id, name in TUTORIAL_GRACES:
        local = flag_id - 71800
        byte_offset = base_71800 + local // 8
        bit_pos = 7 - (local % 8)

        val_s0 = check_flag(ef_s0, byte_offset, bit_pos)
        val_s1 = check_flag(ef_s1, byte_offset, bit_pos)
        status_s0 = "SET" if val_s0 else "unset"
        status_s1 = "SET" if val_s1 else "unset"
        print(f"  {flag_id} ({name:25s}): S0={status_s0}, S1={status_s1}")
        print(f"    (byte {byte_offset}, bit {bit_pos})")

    # Check Block 71100 (Leyndell) - ground_truth says base 2725
    print("\n" + "=" * 70)
    print("BLOCK 71100 (Leyndell) - Testing Base 2725")
    print("=" * 70)

    base_71100 = 2725
    print(f"\nUsing same base as 71800 (2725):")
    print("If 71100 and 71800 share the same byte range, their flags would OVERLAP!")
    print()

    for flag_id, name in LEYNDELL_GRACES[:5]:
        local = flag_id - 71100
        byte_offset = base_71100 + local // 8
        bit_pos = 7 - (local % 8)

        val_s0 = check_flag(ef_s0, byte_offset, bit_pos)
        val_s1 = check_flag(ef_s1, byte_offset, bit_pos)
        status_s0 = "SET" if val_s0 else "unset"
        status_s1 = "SET" if val_s1 else "unset"
        print(f"  {flag_id} ({name:25s}): S0={status_s0}, S1={status_s1}")
        print(f"    (byte {byte_offset}, bit {bit_pos})")

    # Show the byte breakdown
    print("\n" + "=" * 70)
    print("BYTE LAYOUT ANALYSIS")
    print("=" * 70)

    print("\nBytes 2725-2735 (covers 80 flags worth of bits):")
    for byte_off in range(2725, 2736):
        if byte_off < len(ef_s0):
            byte_s0 = ef_s0[byte_off]
            byte_s1 = ef_s1[byte_off]
            print(f"  Byte {byte_off}: S0=0x{byte_s0:02X} ({byte_s0:08b}), S1=0x{byte_s1:02X} ({byte_s1:08b})")

            # Which flags this covers depends on block assignment
            # If base_71800 = 2725: byte covers 71800-71807
            # If base_71100 = 2725: byte covers 71100-71107
            offset_from_base = byte_off - 2725
            flags_71800 = f"71{800 + offset_from_base * 8}-71{800 + offset_from_base * 8 + 7}"
            flags_71100 = f"71{100 + offset_from_base * 8}-71{100 + offset_from_base * 8 + 7}"
            print(f"    If base=2725 for 71800: covers {flags_71800}")
            print(f"    If base=2725 for 71100: covers {flags_71100}")

    # The key insight: if both blocks use base 2725, they would share bytes!
    # Block 71100 flags 71100-71107 would be at byte 2725 (same as 71800-71807)
    # This would mean flag 71100 and flag 71800 share the same bit!

    print("\n" + "=" * 70)
    print("KEY INSIGHT: FLAG COLLISION ANALYSIS")
    print("=" * 70)

    print("""
If both 71100 and 71800 use base 2725:
- Flag 71100: local 0, byte 2725, bit 7
- Flag 71800: local 0, byte 2725, bit 7
COLLISION! Same bit would represent BOTH flags!

This proves one of the following:
1. Block 71100 uses a DIFFERENT base than 71800
2. The flag ID format for 71100 is different
3. One of the ground_truth entries is wrong
""")

    # Search for correct base for Block 71100
    print("\n" + "=" * 70)
    print("SEARCHING FOR CORRECT BASE FOR BLOCK 71100")
    print("=" * 70)

    # Mid-game Confessor likely hasn't reached Leyndell (requires 2 Great Runes)
    # So most 71100 flags should be UNSET in S0 and S1

    print("\nExpectation: Confessor (S0) probably hasn't reached Leyndell")
    print("So 71100-71199 flags should mostly be UNSET")
    print()

    # Try different bases
    test_bases = [2725, 2737, 2750, 2612, 2700, 2800]

    for base in test_bases:
        set_count_s0 = 0
        set_count_s1 = 0

        for flag_id, name in LEYNDELL_GRACES:
            local = flag_id - 71100
            byte_offset = base + local // 8
            bit_pos = 7 - (local % 8)

            if check_flag(ef_s0, byte_offset, bit_pos):
                set_count_s0 += 1
            if check_flag(ef_s1, byte_offset, bit_pos):
                set_count_s1 += 1

        print(f"  Base {base}: S0={set_count_s0}/10 SET, S1={set_count_s1}/10 SET")

    # The correct base should show 0 or very few flags SET (Leyndell not reached)
    print("\n" + "=" * 70)
    print("CONCLUSION")
    print("=" * 70)

    print("""
The correct base for Block 71100 should show:
- Few or no flags SET in S0 (Confessor hasn't reached Leyndell)
- Few or no flags SET in S1 (Wretch definitely hasn't)

If base 2725 shows many flags SET, it's likely collision with Tutorial graces.

Block 71100 and 71800 are probably stored at DIFFERENT bases despite both
being in the 71xxx range. Each legacy dungeon likely has its own base.
""")

if __name__ == "__main__":
    main()
