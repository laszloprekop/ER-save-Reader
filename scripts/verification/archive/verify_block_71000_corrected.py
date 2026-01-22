#!/usr/bin/env python3
"""
Verify the corrected Block 71000 base offset.
Check absolute positions and validate against multiple save files.
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

def main():
    print("=" * 70)
    print("VERIFY CORRECTED BLOCK 71000 BASE")
    print("=" * 70)

    results = []

    for save_path, name in SAVE_FILES:
        if not save_path.exists():
            continue

        slot0_data = read_slot_data(save_path, 0)
        ef_start = detect_event_flags_start(slot0_data, SEARCH_START)

        EVENT_FLAGS_SIZE = 0x1bf99f
        ef_data = slot0_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        # Find best base for Stormveil graces
        best_base = None
        best_count = 0
        best_flags = []

        for test_base in range(0, 15000):
            if test_base + 2 >= len(ef_data):
                continue

            set_count = 0
            flags_set = []
            for flag_id, _ in STORMVEIL_GRACES:
                local = flag_id - 71000
                byte_offset = test_base + local // 8
                bit_pos = 7 - (local % 8)
                if byte_offset < len(ef_data) and ef_data[byte_offset] & (1 << bit_pos):
                    set_count += 1
                    flags_set.append(flag_id)

            byte0 = ef_data[test_base]
            byte1 = ef_data[test_base + 1] if test_base + 1 < len(ef_data) else 0

            # Require non-0xFF and include 71008
            if set_count > best_count and byte0 != 0xFF and 71008 in flags_set:
                best_count = set_count
                best_base = test_base
                best_flags = flags_set

        abs_pos = ef_start + best_base if best_base else 0
        results.append({
            'name': name,
            'ef_start': ef_start,
            'base': best_base,
            'abs_pos': abs_pos,
            'count': best_count,
            'flags': best_flags,
        })

    print("\n" + "=" * 70)
    print("RESULTS SUMMARY")
    print("=" * 70)

    for r in results:
        print(f"\n{r['name']}:")
        print(f"  EF start: 0x{r['ef_start']:X}")
        print(f"  Best base: {r['base']}")
        print(f"  Absolute pos in slot: 0x{r['abs_pos']:X} ({r['abs_pos']})")
        print(f"  Graces SET: {r['count']}/9")
        print(f"  Flags: {r['flags']}")

    # Calculate relationship
    if len(results) == 2:
        print("\n" + "=" * 70)
        print("RELATIONSHIP ANALYSIS")
        print("=" * 70)

        r1, r2 = results
        ef_diff = r2['ef_start'] - r1['ef_start']
        base_diff = r2['base'] - r1['base']
        abs_diff = r2['abs_pos'] - r1['abs_pos']

        print(f"\nEF start difference: {ef_diff} bytes (0x{ef_diff:X})")
        print(f"Base difference: {base_diff} bytes")
        print(f"Absolute position difference: {abs_diff} bytes (0x{abs_diff:X})")

        # The relationship between EF start and base
        print(f"\nBase appears to shift {base_diff - ef_diff} bytes relative to EF start change")

        # Calculate a normalized offset
        # If absolute position is consistent, we can use that as reference
        avg_abs = (r1['abs_pos'] + r2['abs_pos']) // 2
        print(f"\nAverage absolute position: 0x{avg_abs:X}")

        # For ground truth, we should use a formula or store both
        print(f"\nRECOMMENDED: Store as 'absolute_offset_from_slot_start: 0x{avg_abs:X}'")
        print(f"Or use dynamic calculation: base = detected_ef_start + relative_offset")

    print("\n" + "=" * 70)
    print("GROUND TRUTH UPDATE RECOMMENDATION")
    print("=" * 70)

    # Use the first result as reference
    r = results[0]
    print(f"""
Block 71000 (Stormveil Castle graces):
  - Previous base (WRONG): 2673
  - Correct base for EF 0x{r['ef_start']:X}: {r['base']}
  - Absolute offset in slot data: 0x{r['abs_pos']:X}

The base varies with EF start. For accurate detection:
  1. Detect EF start dynamically
  2. Use the absolute offset 0x{r['abs_pos']:X} or
  3. Calculate: base = abs_offset - ef_start = 0x{r['abs_pos']:X} - 0x{r['ef_start']:X} = {r['base']}
""")

if __name__ == "__main__":
    main()
