#!/usr/bin/env python3
"""
Test 71008 (Stormveil Main Gate) against the Confessor level 93 snapshot.

User confirms they discovered Main Gate BEFORE January 11.
Let's check this more recent snapshot file.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")
CONFESSOR_SNAPSHOT = SAVE_DIR / "ER0000.sl2 Confessor - level 93 snapshot"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
SEARCH_START = 0x12000

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
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

def detect_event_flags_start(slot_data, search_start):
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

    return 0x12B00

def read_slot_data(save_file, slot_index):
    with open(save_file, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def check_flag(ef_data, byte_offset, bit_pos):
    if byte_offset < len(ef_data):
        return bool(ef_data[byte_offset] & (1 << bit_pos))
    return None

def main():
    print("=" * 70)
    print("TEST CONFESSOR LEVEL 93 SNAPSHOT")
    print("=" * 70)

    print(f"\nSave file: {CONFESSOR_SNAPSHOT}")

    if not CONFESSOR_SNAPSHOT.exists():
        print("ERROR: File not found!")
        return

    slot0_data = read_slot_data(CONFESSOR_SNAPSHOT, 0)
    ef_start = detect_event_flags_start(slot0_data, SEARCH_START)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_data = slot0_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

    print(f"\nDetected EF start: 0x{ef_start:X}")

    # Test base 2673
    print("\n" + "=" * 70)
    print("STORMVEIL GRACES AT BASE 2673")
    print("=" * 70)

    base = 2673

    # Show raw bytes
    print(f"\nRaw bytes at base {base}:")
    for i in range(3):
        byte_val = ef_data[base + i] if base + i < len(ef_data) else 0
        print(f"  Byte {base + i}: 0x{byte_val:02X} ({byte_val:08b})")

    print(f"\nStormveil graces:")
    set_count = 0
    for flag_id, name in STORMVEIL_GRACES:
        local = flag_id - 71000
        byte_offset = base + local // 8
        bit_pos = 7 - (local % 8)

        val = check_flag(ef_data, byte_offset, bit_pos)
        if val:
            set_count += 1
            status = "SET"
        else:
            status = "unset"
        print(f"  {flag_id} ({name:25s}): {status}")

    print(f"\nSummary: {set_count}/9 graces SET at base 2673")

    # Also test some alternate bases
    print("\n" + "=" * 70)
    print("TESTING ALTERNATE BASES")
    print("=" * 70)

    for test_base in [2673, 2674, 2672, 2680, 2670]:
        set_count = 0
        flags_set = []
        for flag_id, name in STORMVEIL_GRACES:
            local = flag_id - 71000
            byte_offset = test_base + local // 8
            bit_pos = 7 - (local % 8)

            if check_flag(ef_data, byte_offset, bit_pos):
                set_count += 1
                flags_set.append(flag_id)

        if set_count > 0:
            byte0 = ef_data[test_base] if test_base < len(ef_data) else 0
            byte1 = ef_data[test_base + 1] if test_base + 1 < len(ef_data) else 0
            print(f"\n  Base {test_base}: {set_count}/9 SET")
            print(f"    Bytes: 0x{byte0:02X} 0x{byte1:02X}")
            print(f"    Flags SET: {flags_set}")

    # Search for a base where 71008 IS SET
    print("\n" + "=" * 70)
    print("SEARCHING FOR BASE WHERE 71008 IS SET")
    print("=" * 70)

    found_bases = []
    for test_base in range(2600, 2800):
        local_71008 = 8
        byte_offset = test_base + local_71008 // 8
        bit_pos = 7 - (local_71008 % 8)

        if byte_offset < len(ef_data) and ef_data[byte_offset] & (1 << bit_pos):
            # 71008 is SET at this base
            # Count total SET flags
            set_count = 0
            for flag_id, name in STORMVEIL_GRACES:
                local = flag_id - 71000
                bo = test_base + local // 8
                bp = 7 - (local % 8)
                if bo < len(ef_data) and ef_data[bo] & (1 << bp):
                    set_count += 1

            if set_count >= 5:  # Reasonable threshold
                found_bases.append((test_base, set_count))

    if found_bases:
        print(f"\nBases where 71008 is SET (with 5+ total graces):")
        for base, count in sorted(found_bases, key=lambda x: -x[1])[:10]:
            byte0 = ef_data[base] if base < len(ef_data) else 0
            byte1 = ef_data[base + 1] if base + 1 < len(ef_data) else 0
            print(f"  Base {base}: {count}/9 SET | bytes: 0x{byte0:02X} 0x{byte1:02X}")

            # Show which flags
            flags_set = []
            for flag_id, name in STORMVEIL_GRACES:
                local = flag_id - 71000
                bo = base + local // 8
                bp = 7 - (local % 8)
                if bo < len(ef_data) and ef_data[bo] & (1 << bp):
                    flags_set.append(flag_id)
            print(f"    Flags: {flags_set}")
    else:
        print("\nNo bases found where 71008 is SET with 5+ total graces")

if __name__ == "__main__":
    main()
