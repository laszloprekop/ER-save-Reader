#!/usr/bin/env python3
"""
Investigate the 0xFF padding pattern that appears at different offsets in different slots.
This pattern is causing false positives for dungeon boss flags.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
BACKUP_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
SEARCH_START = 0x12000
EVENT_FLAGS_SIZE = 0x1bf99f

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

def detect_event_flags_start(slot_data, search_start):
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - EVENT_FLAGS_SIZE)

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

def find_0xff_runs(ef_data, min_length=4):
    """Find runs of 0xFF bytes of minimum length."""
    runs = []
    start = None
    length = 0

    for i, byte_val in enumerate(ef_data):
        if byte_val == 0xFF:
            if start is None:
                start = i
            length += 1
        else:
            if start is not None and length >= min_length:
                runs.append((start, length))
            start = None
            length = 0

    if start is not None and length >= min_length:
        runs.append((start, length))

    return runs

def main():
    print("=" * 80)
    print("INVESTIGATE 0xFF PADDING PATTERN")
    print("=" * 80)

    slot_names = ["Confessor (mid)", "Wretch (early)", "V1", "V2", "V3"]

    print("\n--- EF START OFFSETS ---")
    ef_starts = []
    for slot_idx in range(5):
        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = detect_event_flags_start(slot_data, SEARCH_START)
        ef_starts.append(ef_start)
        print(f"Slot {slot_idx} ({slot_names[slot_idx]}): EF start = 0x{ef_start:X} ({ef_start})")

    print("\n--- 0xFF RUNS IN EACH SLOT (relative to EF start) ---")
    print("Looking for runs of 4+ consecutive 0xFF bytes in first 50KB of EF data")

    all_runs = []
    for slot_idx in range(5):
        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = ef_starts[slot_idx]
        ef_data = slot_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        runs = find_0xff_runs(ef_data[:50000], min_length=4)
        all_runs.append(runs)

        print(f"\nSlot {slot_idx} ({slot_names[slot_idx]}):")
        if runs:
            for start, length in runs[:10]:  # Show first 10 runs
                abs_pos = ef_start + start
                print(f"  Offset {start} (abs 0x{abs_pos:X}), length {length} bytes")
        else:
            print("  No significant 0xFF runs found")

    # Calculate absolute positions
    print("\n--- 0xFF RUNS ABSOLUTE POSITIONS (from slot start) ---")
    for slot_idx in range(5):
        ef_start = ef_starts[slot_idx]
        runs = all_runs[slot_idx]

        print(f"\nSlot {slot_idx} ({slot_names[slot_idx]}), EF start 0x{ef_start:X}:")
        if runs:
            for start, length in runs[:10]:
                abs_pos = ef_start + start
                print(f"  Relative: {start}, Absolute: 0x{abs_pos:X} ({abs_pos}), Length: {length}")

    # Check if there's a pattern - are the 0xFF blocks at consistent ABSOLUTE positions?
    print("\n--- ANALYSIS: ARE 0xFF BLOCKS AT CONSISTENT POSITIONS? ---")

    # Find the first significant 0xFF run in each slot
    first_runs = []
    for slot_idx in range(5):
        runs = all_runs[slot_idx]
        if runs:
            start, length = runs[0]
            abs_pos = ef_starts[slot_idx] + start
            first_runs.append((slot_idx, start, abs_pos, length))

    if first_runs:
        print("\nFirst 0xFF run in each slot:")
        for slot_idx, rel_pos, abs_pos, length in first_runs:
            print(f"  Slot {slot_idx}: relative {rel_pos}, absolute 0x{abs_pos:X}, length {length}")

        # Check if relative positions are consistent
        rel_positions = [r[1] for r in first_runs]
        abs_positions = [r[2] for r in first_runs]

        print(f"\n  Relative positions: {rel_positions}")
        print(f"  Absolute positions: {[hex(p) for p in abs_positions]}")

        if len(set(rel_positions)) == 1:
            print("\n  -> 0xFF blocks are at CONSISTENT RELATIVE position (offset from EF start)")
            print("  -> This suggests they're part of the EF data structure (padding/alignment)")
        elif len(set(abs_positions)) == 1:
            print("\n  -> 0xFF blocks are at CONSISTENT ABSOLUTE position (offset from slot start)")
            print("  -> This suggests they're OUTSIDE the EF data (header/other data)")
        else:
            print("\n  -> 0xFF blocks are at VARYING positions")
            print("  -> Need more analysis to determine cause")

    # The problematic offsets
    print("\n--- CHECK PROBLEMATIC OFFSETS ---")
    print("Area 14 (Sewers) offset 30087 and Area 31 (Caves) offset 29859")

    for slot_idx in range(5):
        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = ef_starts[slot_idx]
        ef_data = slot_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        val_30087 = ef_data[30087] if 30087 < len(ef_data) else 0
        val_29859 = ef_data[29859] if 29859 < len(ef_data) else 0

        print(f"Slot {slot_idx}: EF[30087]=0x{val_30087:02X}, EF[29859]=0x{val_29859:02X}")

    # Check if these offsets fall within 0xFF runs
    print("\n--- DO PROBLEMATIC OFFSETS FALL IN 0xFF RUNS? ---")
    for slot_idx in range(5):
        runs = all_runs[slot_idx]
        in_run_30087 = any(start <= 30087 < start + length for start, length in runs)
        in_run_29859 = any(start <= 29859 < start + length for start, length in runs)
        print(f"Slot {slot_idx}: 30087 in 0xFF run: {in_run_30087}, 29859 in 0xFF run: {in_run_29859}")

if __name__ == "__main__":
    main()
