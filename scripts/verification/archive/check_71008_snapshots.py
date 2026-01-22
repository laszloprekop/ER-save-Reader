#!/usr/bin/env python3
"""
Check flag 71008 (Stormveil Main Gate) across multiple save files.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SNAPSHOT_DIR = SAVE_DIR / "Granular snapshots for debugging"

SAVE_FILES = [
    (SAVE_DIR / "ER0000-backup-2026-01-11.sl2", "Backup Jan 11"),
    (SNAPSHOT_DIR / "ER0000.sl2 Confessor - level 93 snapshot", "Confessor L93 (Dec 25)"),
    (SNAPSHOT_DIR / "ER0000.sl2 Confessor - 01 before Missionary Cookbok [4] pickup", "Confessor Dec 29"),
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
    print("CHECK FLAG 71008 (STORMVEIL MAIN GATE) ACROSS SAVE FILES")
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

        # Check Stormveil graces at base 2673
        print(f"\nStormveil graces at base 2673:")
        set_count = 0
        for flag_id, grace_name in STORMVEIL_GRACES:
            local = flag_id - 71000
            byte_offset = 2673 + local // 8
            bit_pos = 7 - (local % 8)
            val = check_flag(ef_data, byte_offset, bit_pos)
            if val:
                set_count += 1
            status = "SET" if val else "unset"
            highlight = " <-- TARGET" if flag_id == 71008 else ""
            print(f"  {flag_id} ({grace_name:25s}): {status}{highlight}")
        print(f"  Total: {set_count}/9")

        # Show raw bytes at base 2673
        print(f"\nRaw bytes at base 2673-2676:")
        for i in range(4):
            byte_val = ef_data[2673 + i]
            print(f"  Byte {2673 + i}: 0x{byte_val:02X} ({byte_val:08b})")

        # Search wider for any base where 71008 is SET
        print(f"\nSearching bases 2600-2800 for 71008 SET...")
        found_71008 = []
        for test_base in range(2600, 2800):
            local_71008 = 8
            byte_offset = test_base + local_71008 // 8
            bit_pos = 7 - (local_71008 % 8)

            if byte_offset >= len(ef_data):
                continue

            byte_val = ef_data[byte_offset]
            if byte_val & (1 << bit_pos):
                # 71008 is SET here
                # Count other graces
                grace_count = 0
                for fid, _ in STORMVEIL_GRACES:
                    local = fid - 71000
                    bo = test_base + local // 8
                    bp = 7 - (local % 8)
                    if bo < len(ef_data) and ef_data[bo] & (1 << bp):
                        grace_count += 1

                # Exclude 0xFF false positives
                base_byte = ef_data[test_base] if test_base < len(ef_data) else 0
                if base_byte != 0xFF:
                    found_71008.append((test_base, grace_count, base_byte))

        if found_71008:
            print(f"  Found {len(found_71008)} bases where 71008 is SET:")
            for base, count, b0 in sorted(found_71008, key=lambda x: -x[1])[:5]:
                print(f"    Base {base}: {count}/9 graces, byte0=0x{b0:02X}")
        else:
            print(f"  71008 NOT SET at any base in range 2600-2800")

if __name__ == "__main__":
    main()
