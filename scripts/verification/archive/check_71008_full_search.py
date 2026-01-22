#!/usr/bin/env python3
"""
Full search for Stormveil graces across all save files.
Check validation flags and search for the correct base.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SNAPSHOT_DIR = SAVE_DIR / "Granular snapshots for debugging"

SAVE_FILES = [
    (SAVE_DIR / "ER0000-backup-2026-01-11.sl2", "Backup Jan 11"),
    (SNAPSHOT_DIR / "ER0000.sl2 Confessor - level 93 snapshot", "Confessor L93 (Dec 25)"),
]

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

def check_flag(ef_data, byte_offset, bit_pos):
    if byte_offset < len(ef_data):
        return bool(ef_data[byte_offset] & (1 << bit_pos))
    return None

def main():
    print("=" * 70)
    print("FULL SEARCH FOR STORMVEIL GRACES")
    print("=" * 70)

    for save_path, name in SAVE_FILES:
        if not save_path.exists():
            print(f"\n{name}: FILE NOT FOUND")
            continue

        print(f"\n{'='*70}")
        print(f"FILE: {name}")
        print(f"{'='*70}")

        slot0_data = read_slot_data(save_path, 0)
        ef_start = detect_event_flags_start(slot0_data, SEARCH_START)

        EVENT_FLAGS_SIZE = 0x1bf99f
        ef_data = slot0_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        print(f"\nEventFlags start: 0x{ef_start:X}")

        # Verify validation flags
        print(f"\nValidation flags:")
        for flag_id, byte_offset, bit_pos, flag_name in VALIDATION_FLAGS:
            val = check_flag(ef_data, byte_offset, bit_pos)
            status = "SET" if val else "unset"
            byte_val = ef_data[byte_offset] if byte_offset < len(ef_data) else 0
            print(f"  {flag_id} ({flag_name}): {status} (byte=0x{byte_val:02X})")

        # Wide search for Stormveil graces - find bases with MOST graces SET
        print(f"\nSearching ALL bases 0-10000 for Stormveil graces...")
        best_bases = []
        for test_base in range(0, 10000):
            if test_base + 2 >= len(ef_data):
                continue

            set_count = 0
            flags_set = []
            for flag_id, grace_name in STORMVEIL_GRACES:
                local = flag_id - 71000
                byte_offset = test_base + local // 8
                bit_pos = 7 - (local % 8)
                if check_flag(ef_data, byte_offset, bit_pos):
                    set_count += 1
                    flags_set.append(flag_id)

            if set_count >= 3:  # At least 3 graces
                byte0 = ef_data[test_base]
                byte1 = ef_data[test_base + 1] if test_base + 1 < len(ef_data) else 0
                # Exclude obvious false positives (all 0xFF)
                if byte0 != 0xFF and byte1 != 0xFF:
                    best_bases.append((test_base, set_count, flags_set, byte0, byte1))

        if best_bases:
            print(f"\nBases with 3+ graces SET (top 10):")
            for base, count, flags, b0, b1 in sorted(best_bases, key=lambda x: -x[1])[:10]:
                has_71008 = 71008 in flags
                mark = " ** HAS 71008 **" if has_71008 else ""
                print(f"  Base {base}: {count}/9 SET | bytes: 0x{b0:02X} 0x{b1:02X}{mark}")
                print(f"    Flags: {flags}")
        else:
            print("  No bases found with 3+ graces SET")

        # Specifically search for 71008 SET anywhere
        print(f"\nSearching for ANY base where 71008 is SET...")
        found_71008 = []
        for test_base in range(0, 10000):
            local_71008 = 8
            byte_offset = test_base + local_71008 // 8  # test_base + 1
            bit_pos = 7 - (local_71008 % 8)  # bit 7

            if byte_offset >= len(ef_data):
                continue

            byte_val = ef_data[byte_offset]
            if byte_val & (1 << bit_pos):  # bit 7 is SET
                base_byte = ef_data[test_base] if test_base < len(ef_data) else 0
                if base_byte != 0xFF:  # Exclude false positives
                    grace_count = 0
                    for fid, _ in STORMVEIL_GRACES:
                        local = fid - 71000
                        bo = test_base + local // 8
                        bp = 7 - (local % 8)
                        if bo < len(ef_data) and ef_data[bo] & (1 << bp):
                            grace_count += 1
                    found_71008.append((test_base, grace_count, byte_val, base_byte))

        if found_71008:
            print(f"  Found {len(found_71008)} bases where 71008 is SET:")
            for base, count, bval, b0 in sorted(found_71008, key=lambda x: -x[1])[:10]:
                print(f"    Base {base}: {count}/9 graces, byte at 71008=0x{bval:02X}, byte0=0x{b0:02X}")
        else:
            print("  71008 is NOT SET at ANY base (0-10000)")

if __name__ == "__main__":
    main()
