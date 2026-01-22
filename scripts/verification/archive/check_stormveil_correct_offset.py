#!/usr/bin/env python3
"""
Check Stormveil graces using the CORRECT EventFlags offset from Rust code.

The Rust code searches from 0x12000 within the slot, not 0x1901D0!
FALLBACK_OFFSET = 0x12B00
"""

import struct
from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010

# Correct search range from Rust code
SEARCH_START = 0x12000
FALLBACK_OFFSET = 0x12B00

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

NEGATIVE_VALIDATION_FLAGS = [
    (76223, 3277, 0, "Fortified Manor, First Floor"),
    (76224, 3278, 7, "East Capital Rampart"),
    (76225, 3278, 6, "Divine Bridge"),
    (76300, 3287, 3, "Zamor Ruins"),
    (76301, 3287, 2, "Ancient Snow Valley Ruins"),
    (76350, 3293, 5, "Haligtree Town"),
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
    """Detect EventFlags using the same algorithm as Rust code."""
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - 0x1bf99f)
    min_offset = 500

    actual_start = max(search_start, min_offset)

    # Phase 1: Find offsets where ALL positive flags match
    candidates = []

    for test_offset in range(actual_start, search_end):
        positive_score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    positive_score += 1

        if positive_score == len(VALIDATION_FLAGS):
            # Count negative flags that are NOT set
            negative_score = 0
            for flag_id, byte_offset, bit_pos, name in NEGATIVE_VALIDATION_FLAGS:
                abs_pos = test_offset + byte_offset
                if abs_pos < len(slot_data):
                    byte_val = slot_data[abs_pos]
                    if not (byte_val & (1 << bit_pos)):
                        negative_score += 1

            candidates.append((test_offset, negative_score))

            if negative_score == len(NEGATIVE_VALIDATION_FLAGS):
                return test_offset, positive_score, True

    if candidates:
        candidates.sort(key=lambda x: (-x[1], x[0]))
        best_offset, best_neg_score = candidates[0]
        confident = best_neg_score >= len(NEGATIVE_VALIDATION_FLAGS) // 2
        return best_offset, len(VALIDATION_FLAGS), confident

    # Phase 2: Fallback to best partial match
    best_offset = actual_start
    best_score = 0

    for test_offset in range(actual_start, search_end):
        positive_score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    positive_score += 1

        if positive_score > best_score:
            best_score = positive_score
            best_offset = test_offset

    if best_score >= 2:
        return best_offset, best_score, False
    else:
        return fallback_offset, 0, False

def read_slot_data(slot_index):
    with open(SAVE_FILE, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def main():
    print("=" * 70)
    print("STORMVEIL GRACES CHECK - CORRECT EVENTFLAGS OFFSET")
    print("=" * 70)

    print(f"\nUsing Rust code's search range:")
    print(f"  SEARCH_START = 0x{SEARCH_START:X}")
    print(f"  FALLBACK_OFFSET = 0x{FALLBACK_OFFSET:X}")

    for slot_idx in range(2):  # Check slots 0 and 1
        slot_data = read_slot_data(slot_idx)

        print(f"\n{'='*70}")
        print(f"SLOT {slot_idx}")
        print(f"{'='*70}")

        # Detect EF start
        ef_start, score, confident = detect_event_flags_start(slot_data, SEARCH_START, FALLBACK_OFFSET)

        print(f"  Detected EF start: 0x{ef_start:X} ({ef_start})")
        print(f"  Validation score: {score}/{len(VALIDATION_FLAGS)}")
        print(f"  Confident: {confident}")

        # Verify validation flags
        print(f"\n  Validation flags at detected offset:")
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = ef_start + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                is_set = bool(byte_val & (1 << bit_pos))
                status = "SET" if is_set else "unset"
                print(f"    {flag_id} ({name:25s}): {status}")

        # Check Stormveil graces
        print(f"\n  Stormveil graces (block 71000, base 2625):")
        base_71000 = 2625  # Derived from: 2725 - (71800-71000)/8 = 2725 - 100 = 2625

        set_count = 0
        for flag_id, name in STORMVEIL_GRACES:
            local = flag_id - 71000
            byte_offset = base_71000 + local // 8
            bit_pos = 7 - (local % 8)
            abs_pos = ef_start + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                is_set = bool(byte_val & (1 << bit_pos))
                if is_set:
                    set_count += 1
                    status = "SET"
                else:
                    status = "unset"
                print(f"    {flag_id} ({name:25s}): {status}")

        print(f"\n  Summary: {set_count}/{len(STORMVEIL_GRACES)} Stormveil graces SET")

        # Show raw bytes
        abs_base = ef_start + base_71000
        print(f"\n  Raw bytes at base 2625 (absolute 0x{abs_base:X}):")
        for i in range(2):
            pos = abs_base + i
            if pos < len(slot_data):
                byte_val = slot_data[pos]
                print(f"    Byte {base_71000 + i}: 0x{byte_val:02X} ({byte_val:08b})")

    # Also show what the webapp would see
    print("\n" + "=" * 70)
    print("WEBAPP DATA COMPARISON")
    print("=" * 70)

    print("\nThe webapp reported flag 71007 (Secluded Cell) as NOT SET.")
    print("User manually marked it as COMPLETE (mismatch).")
    print("\nIf Stormveil graces are now showing SET, the webapp may have been")
    print("using the wrong EventFlags offset (like we were in Python).")

if __name__ == "__main__":
    main()
