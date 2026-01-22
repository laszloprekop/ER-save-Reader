#!/usr/bin/env python3
"""
Debug the EventFlags detection to understand why validation flags are showing as UNSET.
"""

import struct
from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
EVENT_FLAGS_SIZE = 0x1bf99f

# Validation flags - these MUST be SET for a valid detection
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

def detect_event_flags_start(slot_data, search_start, search_end):
    """Detect EventFlags start with detailed logging."""
    best_offset = None
    best_score = 0

    for test_offset in range(search_start, min(search_end, len(slot_data) - 10000)):
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

def main():
    print("=" * 70)
    print("DEBUG: EVENT FLAGS DETECTION")
    print("=" * 70)

    with open(SAVE_FILE, 'rb') as f:
        file_size = f.seek(0, 2)
        print(f"\nSave file size: {file_size:,} bytes")

        # Read slot 0
        slot_0_start = SLOT_0_OFFSET
        f.seek(slot_0_start)
        slot_0_data = f.read(SLOT_SIZE)
        print(f"Slot 0 start: 0x{slot_0_start:X}, read {len(slot_0_data):,} bytes")

    # Try different search ranges
    print("\n" + "=" * 70)
    print("SEARCHING FOR EVENT FLAGS START")
    print("=" * 70)

    # Expected offset is around 0x1901D0
    expected_start = 0x1901D0
    print(f"\nExpected start: 0x{expected_start:X} ({expected_start:,})")

    # Search in different ranges
    ranges = [
        (expected_start - 1000, expected_start + 1000),
        (0, 50000),
        (100000, 200000),
        (0x190000, 0x191000),
    ]

    for search_start, search_end in ranges:
        offset, score = detect_event_flags_start(slot_0_data, search_start, search_end)
        if offset is not None:
            print(f"\n  Range 0x{search_start:X}-0x{search_end:X}: offset=0x{offset:X} ({offset}), score={score}/{len(VALIDATION_FLAGS)}")
        else:
            print(f"\n  Range 0x{search_start:X}-0x{search_end:X}: NO MATCH FOUND, score={score}")

        if score >= 1 and offset is not None:
            # Show what we found at this offset
            print(f"    Checking validation flags at detected offset:")
            for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
                abs_pos = offset + byte_offset
                if abs_pos < len(slot_0_data):
                    byte_val = slot_0_data[abs_pos]
                    is_set = bool(byte_val & (1 << bit_pos))
                    print(f"      {flag_id} ({name}): byte=0x{byte_val:02X}, bit{bit_pos}={is_set}")

    # Check raw bytes at expected locations
    print("\n" + "=" * 70)
    print("RAW BYTES AT EXPECTED LOCATIONS (relative to slot start)")
    print("=" * 70)

    # EventFlags might start at various offsets
    possible_starts = [0x1901D0, 0x190000, 0x1901C8, 0x1901D8]

    for ef_start in possible_starts:
        print(f"\n  Testing EF start at 0x{ef_start:X}:")

        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = ef_start + byte_offset
            if abs_pos < len(slot_0_data):
                byte_val = slot_0_data[abs_pos]
                is_set = bool(byte_val & (1 << bit_pos))
                print(f"    {flag_id} ({name}): abs=0x{abs_pos:X}, byte=0x{byte_val:02X}, bit{bit_pos}={is_set}")
            else:
                print(f"    {flag_id} ({name}): OUT OF RANGE at 0x{abs_pos:X}")

    # Let's also check if the data is all zeros in the event flags area
    print("\n" + "=" * 70)
    print("CHECKING FOR NON-ZERO DATA IN EVENT FLAGS REGION")
    print("=" * 70)

    # Count non-zero bytes in different regions
    regions = [
        (0x190000, 0x191000, "0x190000-0x191000"),
        (0x1901D0, 0x1911D0, "0x1901D0+4096"),
        (0x1901D0 + 2700, 0x1901D0 + 2800, "Around byte 2725"),
        (0x1901D0 + 3200, 0x1901D0 + 3300, "Around byte 3262"),
    ]

    for start, end, desc in regions:
        if end <= len(slot_0_data):
            region = slot_0_data[start:end]
            nonzero = sum(1 for b in region if b != 0)
            print(f"  {desc}: {nonzero}/{end-start} non-zero bytes")
        else:
            print(f"  {desc}: OUT OF RANGE")

    # Check the actual expected byte positions
    print("\n" + "=" * 70)
    print("CHECKING EXACT VALIDATION BYTE POSITIONS")
    print("=" * 70)

    ef_start = 0x1901D0

    print(f"\nWith EF start = 0x{ef_start:X}:")
    print(f"  Byte 2725 (for 71800/71801) at absolute 0x{ef_start + 2725:X}")
    print(f"  Byte 3262 (for 76100/76101) at absolute 0x{ef_start + 3262:X}")

    # Show actual values
    for byte_offset, flags in [(2725, "71800/71801"), (3262, "76100/76101")]:
        abs_pos = ef_start + byte_offset
        if abs_pos < len(slot_0_data):
            byte_val = slot_0_data[abs_pos]
            print(f"  Byte {byte_offset} (for {flags}): 0x{byte_val:02X} ({byte_val:08b})")

            # Show surrounding bytes
            print(f"    Surrounding: ", end="")
            for i in range(-5, 6):
                pos = abs_pos + i
                if 0 <= pos < len(slot_0_data):
                    v = slot_0_data[pos]
                    mark = "**" if i == 0 else ""
                    print(f"{mark}0x{v:02X}{mark} ", end="")
            print()

    # Search for the specific pattern we expect
    print("\n" + "=" * 70)
    print("SEARCHING FOR BYTE PATTERN 0xC0 (bits 7 and 6 set)")
    print("(Expected at byte 2725 for Cave of Knowledge + Stranded Graveyard)")
    print("=" * 70)

    # 71800 bit 7 + 71801 bit 6 = 0xC0
    pattern_found = []
    for i in range(len(slot_0_data) - 1):
        if slot_0_data[i] == 0xC0 or (slot_0_data[i] & 0xC0) == 0xC0:
            pattern_found.append(i)

    print(f"\nFound {len(pattern_found)} positions with bits 7 and 6 set")
    if pattern_found:
        print("First 20 positions:")
        for pos in pattern_found[:20]:
            byte_val = slot_0_data[pos]
            # Calculate what EF start this would imply
            implied_ef_start = pos - 2725
            print(f"  Position 0x{pos:X} ({pos}): 0x{byte_val:02X} → implies EF start at 0x{implied_ef_start:X}")

if __name__ == "__main__":
    main()
