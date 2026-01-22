#!/usr/bin/env python3
"""
Verify EventFlags at detected offset 0x315 and check Stormveil graces there.

The debug script found that validation flags match at offset 0x315 within slot data,
while the expected location (0x1901D0) is all zeros.
"""

import struct
from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
EVENT_FLAGS_SIZE = 0x1bf99f

# New detected EF start
DETECTED_EF_START = 0x315

# Validation flags
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Stormveil graces (from BonfireWarpParam)
STORMVEIL_GRACES = [
    (71000, "Godrick the Grafted"),
    (71001, "Secluded Cell"),  # Was mislabeled as 71007 in probe
    (71002, "Godrick the Grafted (pre-boss)"),
    (71003, "Liftside Chamber"),
    (71004, "Stormveil Cliffside"),
    (71005, "Rampart Tower"),
    (71006, "Gateside Chamber"),
    (71007, "Stormveil Main Gate"),  # User's webapp data showed this as "Secluded Cell"
    (71008, "Margit, The Fell Omen"),
]

def read_slot_data(slot_index):
    with open(SAVE_FILE, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def check_flag(slot_data, ef_start, flag_id, block_start=71000):
    """Check flag using block formula."""
    local = flag_id - block_start
    byte_offset = ef_start + (2725 - 100) + local // 8  # Base derived from validation flags
    # Actually, let's derive the base properly
    # 71800 at byte 2725 with block_start 71000: local = 800, byte = base + 100, so base = 2625
    # With EF_start = 0x315, the absolute byte position for flag 71800 is 0x315 + 2725 = 0xCDA

    # For block 71000:
    # base = 2725 - (71800 - 71000)/8 = 2725 - 100 = 2625
    base_71000 = 2625

    local = flag_id - 71000
    byte_offset = ef_start + base_71000 + local // 8
    bit_pos = 7 - (local % 8)

    if byte_offset >= len(slot_data):
        return None

    return bool(slot_data[byte_offset] & (1 << bit_pos))

def main():
    print("=" * 70)
    print("VERIFY EVENT FLAGS AT DETECTED OFFSET 0x315")
    print("=" * 70)

    slot0_data = read_slot_data(0)
    slot1_data = read_slot_data(1)

    print(f"\nSlot 0 data: {len(slot0_data):,} bytes")
    print(f"Slot 1 data: {len(slot1_data):,} bytes")
    print(f"\nDetected EF start: 0x{DETECTED_EF_START:X}")

    # First verify validation flags work at this offset
    print("\n" + "=" * 70)
    print("VERIFYING VALIDATION FLAGS AT EF_START = 0x315")
    print("=" * 70)

    for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
        abs_pos_s0 = DETECTED_EF_START + byte_offset
        abs_pos_s1 = DETECTED_EF_START + byte_offset

        if abs_pos_s0 < len(slot0_data):
            byte_s0 = slot0_data[abs_pos_s0]
            byte_s1 = slot1_data[abs_pos_s1]
            val_s0 = bool(byte_s0 & (1 << bit_pos))
            val_s1 = bool(byte_s1 & (1 << bit_pos))
            print(f"  {flag_id} ({name}):")
            print(f"    S0: byte=0x{byte_s0:02X}, bit{bit_pos}={val_s0}")
            print(f"    S1: byte=0x{byte_s1:02X}, bit{bit_pos}={val_s1}")

    # Now check Stormveil graces
    print("\n" + "=" * 70)
    print("CHECKING STORMVEIL GRACES (BLOCK 71000)")
    print("=" * 70)

    # Base for block 71000 = 2625 (derived: 2725 - 800/8 = 2725 - 100 = 2625)
    base_71000 = 2625

    print(f"\nUsing base {base_71000} for block 71000:")

    for flag_id, name in STORMVEIL_GRACES:
        local = flag_id - 71000
        byte_offset = base_71000 + local // 8
        bit_pos = 7 - (local % 8)

        abs_pos_s0 = DETECTED_EF_START + byte_offset
        abs_pos_s1 = DETECTED_EF_START + byte_offset

        if abs_pos_s0 < len(slot0_data):
            byte_s0 = slot0_data[abs_pos_s0]
            byte_s1 = slot1_data[abs_pos_s1]
            val_s0 = bool(byte_s0 & (1 << bit_pos))
            val_s1 = bool(byte_s1 & (1 << bit_pos))
            status_s0 = "SET" if val_s0 else "unset"
            status_s1 = "SET" if val_s1 else "unset"
            print(f"  {flag_id} ({name:30s}): S0={status_s0:5s}, S1={status_s1}")

    # Show raw bytes at block 71000 location
    print("\n" + "=" * 70)
    print("RAW BYTES AT BLOCK 71000 LOCATION")
    print("=" * 70)

    abs_base = DETECTED_EF_START + base_71000
    print(f"\nAbsolute byte position: 0x{abs_base:X}")

    for i in range(3):  # First 3 bytes cover flags 71000-71023
        pos_s0 = abs_base + i
        pos_s1 = abs_base + i
        if pos_s0 < len(slot0_data):
            byte_s0 = slot0_data[pos_s0]
            byte_s1 = slot1_data[pos_s1]
            print(f"  Byte {base_71000 + i} (0x{pos_s0:X}): S0=0x{byte_s0:02X} ({byte_s0:08b}), S1=0x{byte_s1:02X} ({byte_s1:08b})")

            # Show which flags this byte covers
            start_flag = 71000 + i * 8
            end_flag = start_flag + 7
            print(f"    (covers flags {start_flag}-{end_flag})")

    # Also verify some other known graces
    print("\n" + "=" * 70)
    print("VERIFYING OTHER KNOWN GRACES")
    print("=" * 70)

    other_graces = [
        (71600, 2825, "Volcano Manor area graces"),  # Block 71600, base calculated: 2725 + (71600-71800)/8 = 2725 - 25 = 2700... hmm
        (76000, 3250, "Limgrave overworld graces"),
    ]

    # Actually let's calculate bases properly:
    # 71800 at byte 2725 means:
    #   If block boundaries are at 71000, 71100, ..., 71800, 71900, ...
    #   Then 71800 is at start of its block, base = 2725

    # For 71600:
    #   71600 is 200 flags before 71800
    #   If same block (71000), local = 600, byte = base + 75
    #   71600 byte = 2625 + 75 = 2700

    # For 71000:
    #   local = 0, byte = 2625

    # For 76100:
    #   byte 3262, bit 3
    #   local = 76100 - 76000 = 100
    #   byte = base + 12
    #   3262 = base + 12 → base = 3250

    for flag_id, expected_base, desc in other_graces:
        print(f"\n  {desc} (flag {flag_id}):")

        # Calculate from block structure
        if flag_id >= 76000:
            block_start = 76000
            base = 3250
        else:
            block_start = 71000
            base = 2625

        local = flag_id - block_start
        byte_offset = base + local // 8
        bit_pos = 7 - (local % 8)

        abs_pos = DETECTED_EF_START + byte_offset
        if abs_pos < len(slot0_data):
            byte_s0 = slot0_data[abs_pos]
            byte_s1 = slot1_data[abs_pos]
            val_s0 = bool(byte_s0 & (1 << bit_pos))
            val_s1 = bool(byte_s1 & (1 << bit_pos))
            print(f"    Calculated: base={base}, local={local}, byte={byte_offset}, bit={bit_pos}")
            print(f"    Result: S0={val_s0}, S1={val_s1}")

if __name__ == "__main__":
    main()
