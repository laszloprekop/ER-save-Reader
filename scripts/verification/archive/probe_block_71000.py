#!/usr/bin/env python3
"""
Probe Block 71000 (Stormveil graces) to understand the actual byte layout.

Known Stormveil graces:
- 71000: Godrick the Grafted (post-boss)
- 71001: Secluded Cell
- 71002: Godrick the Grafted (pre-boss entry)
- 71003: Liftside Chamber
- 71004: Stormveil Cliffside
- 71005: Rampart Tower
- 71006: Gateside Chamber
- 71007: Stormveil Main Gate
- 71008: Margit, The Fell Omen

We need to determine the correct base offset.
"""

import struct
from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
EVENT_FLAGS_SIZE = 0x1bf99f

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Stormveil graces
STORMVEIL_GRACES = [
    (71000, "Godrick the Grafted (post-boss)"),
    (71001, "Secluded Cell"),
    (71002, "Godrick the Grafted (pre-boss)"),
    (71003, "Liftside Chamber"),
    (71004, "Stormveil Cliffside"),
    (71005, "Rampart Tower"),
    (71006, "Gateside Chamber"),
    (71007, "Stormveil Main Gate"),
    (71008, "Margit, The Fell Omen"),
]

def detect_event_flags_start(slot_data, search_start=0):
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - 10000)

    for test_offset in range(search_start, search_end):
        score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    score += 1

        if score == len(VALIDATION_FLAGS):
            return test_offset, score

    return None, 0

def read_slot_event_flags(slot_index):
    with open(SAVE_FILE, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        slot_data = f.read(SLOT_SIZE)

    ef_start, score = detect_event_flags_start(slot_data, search_start=0x1901D0 - 1000)
    if ef_start is None:
        ef_start = 0x1901D0

    ef_end = min(ef_start + EVENT_FLAGS_SIZE, len(slot_data))
    return slot_data[ef_start:ef_end]

def check_flag(event_flags, base_offset, flag_id, block_start=71000):
    """Check flag using block formula."""
    local = flag_id - block_start
    byte_offset = base_offset + local // 8
    bit_pos = 7 - (local % 8)

    if byte_offset >= len(event_flags):
        return None

    return bool(event_flags[byte_offset] & (1 << bit_pos))

def main():
    print("=" * 70)
    print("BLOCK 71000 (STORMVEIL GRACES) INVESTIGATION")
    print("=" * 70)

    print("\nLoading event flags...")
    slot0_flags = read_slot_event_flags(0)
    slot1_flags = read_slot_event_flags(1)

    # Test different base offsets
    test_bases = [2625, 2725, 2821, 2800, 2826, 2827]

    for base in test_bases:
        print(f"\n{'='*70}")
        print(f"Testing base {base} (0x{base:X}):")
        print(f"{'='*70}")

        match_count_s0 = 0
        match_count_s1 = 0

        for flag_id, name in STORMVEIL_GRACES:
            val_s0 = check_flag(slot0_flags, base, flag_id)
            val_s1 = check_flag(slot1_flags, base, flag_id)

            # For mid-game character (slot 0), most Stormveil graces should be discovered
            # For early-game character (slot 1), fewer should be discovered
            status_s0 = "SET" if val_s0 else "UNSET"
            status_s1 = "SET" if val_s1 else "UNSET"

            print(f"  {flag_id} ({name:35s}): S0={status_s0:5s} S1={status_s1:5s}")

            if val_s0:
                match_count_s0 += 1
            if val_s1:
                match_count_s1 += 1

        print(f"\n  Summary: Slot 0 has {match_count_s0}/9 SET, Slot 1 has {match_count_s1}/9 SET")

    # Also check raw bytes around offset 2821
    print("\n" + "=" * 70)
    print("RAW BYTE ANALYSIS around offset 2821")
    print("=" * 70)

    for offset in range(2820, 2830):
        byte_s0 = slot0_flags[offset] if offset < len(slot0_flags) else 0
        byte_s1 = slot1_flags[offset] if offset < len(slot1_flags) else 0
        print(f"  Byte {offset} (0x{offset:X}): S0=0x{byte_s0:02X} ({byte_s0:08b}) S1=0x{byte_s1:02X} ({byte_s1:08b})")

    # Check verified_flags entry: 71000 at offset 2625, bit 7
    print("\n" + "=" * 70)
    print("CHECKING verified_flags ENTRY: 71000 at offset 2625, bit 7")
    print("=" * 70)

    byte_2625_s0 = slot0_flags[2625] if 2625 < len(slot0_flags) else 0
    byte_2625_s1 = slot1_flags[2625] if 2625 < len(slot1_flags) else 0
    bit7_s0 = bool(byte_2625_s0 & (1 << 7))
    bit7_s1 = bool(byte_2625_s1 & (1 << 7))

    print(f"  Byte 2625: S0=0x{byte_2625_s0:02X} ({byte_2625_s0:08b}) S1=0x{byte_2625_s1:02X} ({byte_2625_s1:08b})")
    print(f"  Bit 7 at byte 2625: S0={bit7_s0}, S1={bit7_s1}")

if __name__ == "__main__":
    main()
