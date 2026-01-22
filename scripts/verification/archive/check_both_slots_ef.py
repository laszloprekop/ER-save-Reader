#!/usr/bin/env python3
"""
Check EventFlags start for both slots independently.

The previous check showed inverted patterns - need to verify each slot's EF start.
"""

import struct
from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

def detect_event_flags_start(slot_data, search_start=0, search_end=None):
    """Detect EventFlags start within slot data."""
    if search_end is None:
        search_end = min(200000, len(slot_data) - 10000)

    best_offset = None
    best_score = 0

    for test_offset in range(search_start, search_end):
        score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    score += 1

        if score > best_score:
            best_score = score
            best_offset = test_offset

        if score == len(VALIDATION_FLAGS):
            return test_offset, score

    return best_offset, best_score

def read_slot_data(slot_index):
    with open(SAVE_FILE, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def main():
    print("=" * 70)
    print("CHECK EVENT FLAGS START FOR BOTH SLOTS")
    print("=" * 70)

    for slot_idx in range(5):  # Check all 5 slots
        slot_data = read_slot_data(slot_idx)

        print(f"\n{'='*70}")
        print(f"SLOT {slot_idx}")
        print(f"{'='*70}")

        # Check if slot has any non-zero data (empty slot check)
        first_100_bytes = slot_data[:100]
        nonzero = sum(1 for b in first_100_bytes if b != 0)
        if nonzero < 10:
            print(f"  (Likely empty slot - only {nonzero} non-zero bytes in first 100)")
            continue

        # Detect EF start
        ef_start, score = detect_event_flags_start(slot_data)

        if ef_start is not None:
            print(f"  Detected EF start: 0x{ef_start:X} ({ef_start}), score={score}/4")

            # Verify validation flags
            print(f"\n  Validation flags at EF_START = 0x{ef_start:X}:")
            for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
                abs_pos = ef_start + byte_offset
                if abs_pos < len(slot_data):
                    byte_val = slot_data[abs_pos]
                    is_set = bool(byte_val & (1 << bit_pos))
                    status = "SET" if is_set else "unset"
                    print(f"    {flag_id} ({name:20s}): {status}")

            # Check Stormveil graces
            print(f"\n  Stormveil graces (base 2625):")
            base_71000 = 2625
            stormveil_graces = [
                (71000, "Godrick the Grafted"),
                (71001, "Secluded Cell"),
                (71003, "Liftside Chamber"),
                (71004, "Stormveil Cliffside"),
                (71005, "Rampart Tower"),
                (71006, "Gateside Chamber"),
                (71007, "Stormveil Main Gate"),
                (71008, "Margit, The Fell Omen"),
            ]

            set_count = 0
            for flag_id, name in stormveil_graces:
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

            print(f"\n  Summary: {set_count}/{len(stormveil_graces)} Stormveil graces SET")

            # Show raw bytes
            abs_base = ef_start + base_71000
            print(f"\n  Raw bytes at base 2625 (absolute 0x{abs_base:X}):")
            for i in range(2):
                pos = abs_base + i
                if pos < len(slot_data):
                    byte_val = slot_data[pos]
                    print(f"    Byte {i}: 0x{byte_val:02X} ({byte_val:08b})")
        else:
            print(f"  No EventFlags start detected (best score = {score})")

        # Also check the expected EF location (0x1901D0)
        print(f"\n  Check at expected location 0x1901D0:")
        expected_ef = 0x1901D0
        expected_base = expected_ef + 2625
        if expected_base + 2 < len(slot_data):
            byte0 = slot_data[expected_base]
            byte1 = slot_data[expected_base + 1]
            print(f"    Bytes at 2625: 0x{byte0:02X} 0x{byte1:02X}")

            # Check validation flags there
            score = 0
            for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
                abs_pos = expected_ef + byte_offset
                if abs_pos < len(slot_data):
                    byte_val = slot_data[abs_pos]
                    if byte_val & (1 << bit_pos):
                        score += 1
            print(f"    Validation score at 0x1901D0: {score}/4")

if __name__ == "__main__":
    main()
