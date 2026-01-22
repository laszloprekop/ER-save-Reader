#!/usr/bin/env python3
"""
Search for base where 71008 (Stormveil Main Gate) IS SET.

User confirms they've discovered Main Gate grace, but both our analysis
and the webapp show it as UNSET at base 2673.

Either:
1. The save file is from before they discovered it
2. Base 2673 is wrong
3. The flag ID is different

Let's search for bases where ALL 9 Stormveil graces are SET.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

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
    print("SEARCH FOR BASE WHERE 71008 (MAIN GATE) IS SET")
    print("=" * 70)

    slot0_data = read_slot_data(0)
    slot1_data = read_slot_data(1)

    ef_start_s0 = detect_event_flags_start(slot0_data, SEARCH_START)
    ef_start_s1 = detect_event_flags_start(slot1_data, SEARCH_START)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]

    print(f"\nDetected EF starts: S0=0x{ef_start_s0:X}, S1=0x{ef_start_s1:X}")

    # First, verify current base 2673
    print("\n" + "=" * 70)
    print("CURRENT BASE 2673 CHECK")
    print("=" * 70)

    base = 2673
    byte_71008 = base + 8 // 8  # 2674
    bit_71008 = 7 - (8 % 8)  # 7

    print(f"\n71008 at base {base}:")
    print(f"  byte = {byte_71008}, bit = {bit_71008}")
    byte_val_s0 = ef_s0[byte_71008] if byte_71008 < len(ef_s0) else 0
    print(f"  Byte {byte_71008} value: 0x{byte_val_s0:02X} ({byte_val_s0:08b})")
    print(f"  Bit {bit_71008}: {bool(byte_val_s0 & (1 << bit_71008))}")

    # Search for bases where 71008 IS SET in S0
    print("\n" + "=" * 70)
    print("SEARCHING FOR BASES WHERE 71008 IS SET IN S0")
    print("=" * 70)

    candidates_71008_set = []

    for test_base in range(0, 10000):
        if test_base + 2 >= len(ef_s0):
            continue

        # Check if 71008 is SET
        local_71008 = 8  # 71008 - 71000
        byte_offset = test_base + local_71008 // 8
        bit_pos = 7 - (local_71008 % 8)

        if byte_offset < len(ef_s0):
            if ef_s0[byte_offset] & (1 << bit_pos):
                # 71008 is SET at this base
                # Now check how many other Stormveil graces are SET
                set_count = 0
                for flag_id, name in STORMVEIL_GRACES:
                    local = flag_id - 71000
                    bo = test_base + local // 8
                    bp = 7 - (local % 8)
                    if bo < len(ef_s0) and ef_s0[bo] & (1 << bp):
                        set_count += 1

                # Check S1 pattern (should have fewer)
                set_count_s1 = 0
                for flag_id, name in STORMVEIL_GRACES:
                    local = flag_id - 71000
                    bo = test_base + local // 8
                    bp = 7 - (local % 8)
                    if bo < len(ef_s1) and ef_s1[bo] & (1 << bp):
                        set_count_s1 += 1

                if set_count >= 7 and set_count_s1 <= 2:  # Good differential
                    candidates_71008_set.append({
                        'base': test_base,
                        'set_s0': set_count,
                        'set_s1': set_count_s1,
                    })

    print(f"\nFound {len(candidates_71008_set)} bases where 71008 is SET with good pattern")

    if candidates_71008_set:
        # Sort by number of flags SET in S0
        candidates_71008_set.sort(key=lambda x: -x['set_s0'])
        print("\nTop candidates:")
        for c in candidates_71008_set[:10]:
            base = c['base']
            byte0 = ef_s0[base] if base < len(ef_s0) else 0
            byte1 = ef_s0[base + 1] if base + 1 < len(ef_s0) else 0
            print(f"\n  Base {base}: S0={c['set_s0']}/9 SET, S1={c['set_s1']}/9 SET")
            print(f"    Bytes: 0x{byte0:02X} 0x{byte1:02X}")

            # Show which flags are SET
            flags_set = []
            for flag_id, name in STORMVEIL_GRACES:
                local = flag_id - 71000
                bo = base + local // 8
                bp = 7 - (local % 8)
                if bo < len(ef_s0) and ef_s0[bo] & (1 << bp):
                    flags_set.append(f"{flag_id}({name[:10]})")
            print(f"    SET: {', '.join(flags_set)}")

    # Also check: maybe 71008 uses a different formula?
    print("\n" + "=" * 70)
    print("CHECKING IF 71008 USES DIFFERENT STORAGE")
    print("=" * 70)

    print("\nPossibility: 71008 might be stored separately from 71000-71007")
    print("Let's check byte 2674 (where 71008 would be at base 2673):")

    byte_2674_s0 = ef_s0[2674]
    byte_2674_s1 = ef_s1[2674]
    print(f"  Byte 2674: S0=0x{byte_2674_s0:02X} ({byte_2674_s0:08b}), S1=0x{byte_2674_s1:02X}")

    print("\nIf base 2673 is correct, 71008 is at byte 2674, bit 7")
    print(f"  Bit 7 of byte 2674 in S0: {bool(byte_2674_s0 & 0x80)}")

    # The save backup date
    print("\n" + "=" * 70)
    print("IMPORTANT NOTE")
    print("=" * 70)
    print("""
The save file is: ER0000-backup-2026-01-11.sl2 (backup from Jan 11)

If you discovered Stormveil Main Gate AFTER January 11, it wouldn't be
in this backup file. The flag would genuinely be UNSET in this save.

To verify:
1. Check if you discovered Main Gate before or after Jan 11
2. Or test with the current save file (not the backup)
""")

if __name__ == "__main__":
    main()
