#!/usr/bin/env python3
"""
Deep EventFlags analysis - check consistency across detection runs
and verify exact byte positions for 71008.
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

def detect_event_flags_start_detailed(slot_data, search_start):
    """Detect EF start with detailed logging."""
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - 0x1bf99f)
    min_offset = 500
    actual_start = max(search_start, min_offset)

    all_matches = []

    for test_offset in range(actual_start, search_end):
        positive_score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    positive_score += 1

        if positive_score == len(VALIDATION_FLAGS):
            all_matches.append(test_offset)

    return all_matches

def read_slot_data(save_file, slot_index):
    with open(save_file, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def main():
    print("=" * 70)
    print("DEEP EVENTFLAGS ANALYSIS")
    print("=" * 70)

    slot0_data = read_slot_data(BACKUP_FILE, 0)
    print(f"\nSlot 0 data size: {len(slot0_data):,} bytes")

    # Find ALL matching EF start offsets
    print("\n" + "=" * 70)
    print("FINDING ALL VALID EF START OFFSETS")
    print("=" * 70)

    matches = detect_event_flags_start_detailed(slot0_data, SEARCH_START)
    print(f"\nFound {len(matches)} offsets where all 4 validation flags are SET")

    if matches:
        print("\nFirst 10 matches:")
        for offset in matches[:10]:
            print(f"  0x{offset:X} ({offset})")

        # Show bytes at validation positions for each match
        print("\nDetailed check of first 3 matches:")
        for offset in matches[:3]:
            print(f"\n  Offset 0x{offset:X}:")
            for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
                abs_pos = offset + byte_offset
                byte_val = slot0_data[abs_pos]
                print(f"    {flag_id} ({name}): byte 0x{byte_val:02X} at abs_pos 0x{abs_pos:X}")

    # Use the first match as our EF start
    ef_start = matches[0] if matches else 0x12B00
    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_data = slot0_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

    print(f"\nUsing EF start: 0x{ef_start:X}")

    # Check block 71000 at base 2673
    print("\n" + "=" * 70)
    print("BLOCK 71000 ANALYSIS AT BASE 2673")
    print("=" * 70)

    base = 2673

    # Flag 71008 position
    local_71008 = 8  # 71008 - 71000
    byte_71008 = base + local_71008 // 8  # 2673 + 1 = 2674
    bit_71008 = 7 - (local_71008 % 8)  # 7 - 0 = 7

    print(f"\n71008 calculation:")
    print(f"  local = 71008 - 71000 = 8")
    print(f"  byte = 2673 + 8//8 = 2673 + 1 = 2674")
    print(f"  bit = 7 - 8%8 = 7 - 0 = 7")

    # Show bytes 2673-2680
    print(f"\nRaw bytes at base 2673:")
    for i in range(8):
        byte_val = ef_data[base + i]
        print(f"  Byte {base + i}: 0x{byte_val:02X} ({byte_val:08b})")

    # Check each Stormveil grace
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

    print(f"\nStormveil graces at base {base}:")
    for flag_id, name in STORMVEIL_GRACES:
        local = flag_id - 71000
        byte_offset = base + local // 8
        bit_pos = 7 - (local % 8)

        byte_val = ef_data[byte_offset]
        is_set = bool(byte_val & (1 << bit_pos))
        status = "SET" if is_set else "unset"

        print(f"  {flag_id} ({name:25s}): byte {byte_offset}, bit {bit_pos} = {status}")
        print(f"    byte value: 0x{byte_val:02X}, mask: 0x{1 << bit_pos:02X}, result: {byte_val & (1 << bit_pos)}")

    # Check if maybe the flags are in a different location
    print("\n" + "=" * 70)
    print("SEARCHING FOR 71008 IN NEARBY BASES")
    print("=" * 70)

    print("\nLooking for bases where 71008 is SET and matches known graces...")

    for test_base in range(2650, 2700):
        # Check 71008
        local_71008 = 8
        byte_offset = test_base + local_71008 // 8
        bit_pos = 7 - (local_71008 % 8)

        byte_val = ef_data[byte_offset]
        is_71008_set = bool(byte_val & (1 << bit_pos))

        if is_71008_set:
            # Count other graces
            set_count = 0
            for flag_id, name in STORMVEIL_GRACES:
                local = flag_id - 71000
                bo = test_base + local // 8
                bp = 7 - (local % 8)
                if ef_data[bo] & (1 << bp):
                    set_count += 1

            print(f"  Base {test_base}: 71008 SET, total {set_count}/9")

if __name__ == "__main__":
    main()
