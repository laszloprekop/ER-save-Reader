#!/usr/bin/env python3
"""
Probe all possible locations for VM grace 71607.

We know:
- User confirmed grace 71607 (Subterranean Inquisition Chamber) is SET
- BonfireWarpParam confirms eventflagId = 71607
- Byte 2700 (base 2625) = 0x00 - NOT SET
- Byte 2825 (base 2750 relative to block 71000) = 0x01 - bit 0 SET

The question is: what formula actually maps to byte 2825?
"""

import sys
from typing import Optional

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]


def detect_event_flags_offset(slot_data: bytes, search_start: int = 0x12000) -> Optional[int]:
    for test_offset in range(search_start, min(0x15000, len(slot_data) - 10000)):
        score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if (byte_val & (1 << bit_pos)) != 0:
                    score += 1
        if score == len(VALIDATION_FLAGS):
            return test_offset
    return None


def main():
    save_path = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"
    slot_index = 0

    with open(save_path, 'rb') as f:
        f.seek(HEADER_SIZE + slot_index * SLOT_SIZE)
        slot_data = f.read(SLOT_SIZE)

    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    print(f"Event flags offset: 0x{event_flags_offset:X}")
    event_flags = slot_data[event_flags_offset:]

    print("\n" + "="*70)
    print("KNOWN FACTS ABOUT FLAG 71607")
    print("="*70)
    print("  eventflagId: 71607 (from BonfireWarpParam)")
    print("  Grace name: Subterranean Inquisition Chamber")
    print("  User status: CONFIRMED SET")
    print()

    # Check the expected location (base 2625)
    print("Expected location (base 2625 for block 71000):")
    byte_2700 = event_flags[2700]
    print(f"  byte 2700 = 0x{byte_2700:02X} = {byte_2700:08b}")
    print(f"  bit 0: {'SET' if (byte_2700 & 1) else 'NOT SET'}")

    # Check where bit 0 IS set
    print("\n" + "="*70)
    print("SEARCHING: Where is bit 0 SET in the range 2600-3000?")
    print("="*70)

    bit0_set_locations = []
    for byte_off in range(2600, 3000):
        if event_flags[byte_off] & 1:
            bit0_set_locations.append(byte_off)

    print(f"Found {len(bit0_set_locations)} bytes with bit 0 SET:")
    for loc in bit0_set_locations:
        val = event_flags[loc]
        # What flag ID would this be for various bases?
        print(f"\n  byte {loc}: 0x{val:02X} = {val:08b}")

        # For block 71000, what flag would be at this byte with bit 0?
        # byte = base + (flag - 71000) / 8
        # flag = 71000 + (byte - base) * 8 + (7 - bit)
        # For bit 0: flag = 71000 + (byte - base) * 8 + 7

        for base in [2625, 2650, 2700, 2750]:
            flag_id = 71000 + (loc - base) * 8 + 7
            if 71000 <= flag_id < 72000:
                print(f"    base {base}: would be flag {flag_id}")

    # Check the specific byte 2825 which was flagged earlier
    print("\n" + "="*70)
    print("DETAILED CHECK: Byte 2825")
    print("="*70)
    byte_2825 = event_flags[2825]
    print(f"byte 2825 = 0x{byte_2825:02X} = {byte_2825:08b}")

    # What flags could be at byte 2825?
    for base in [2625, 2700, 2750]:
        # For each bit position
        for bit in range(8):
            flag_id = 71000 + (2825 - base) * 8 + (7 - bit)
            if 71000 <= flag_id < 72000:
                is_set = (byte_2825 >> bit) & 1
                if is_set:
                    print(f"  base {base}, bit {bit}: flag {flag_id} = SET")

    # Also check if this could be a different block
    print("\n" + "="*70)
    print("ALTERNATIVE: Could 71607 be stored as dungeon flag 16xxxxxx?")
    print("="*70)

    # Check if there's a pattern like 16000xxx that could map to this grace
    # Dungeon formula: byte = base + section * 1125 + local_id / 8

    # For Area 16, we don't know the correct base, but let's see what's there
    # Using the legacymap formula: base = 4112 + slot * 1125
    # Slot 29 for Area 16 would give 36737 (but we disproved this)

    print("  Row ID from BonfireWarpParam: 160007")
    print("  If this were a dungeon flag (16 00 0007):")
    print("    Area: 16, Section: 00, Local: 0007")
    print("    byte = base + 0 * 1125 + 0007 / 8 = base + 0")
    print("    bit = 7 - (7 % 8) = 0")

    # For various potential Area 16 bases, check byte, bit 0
    potential_bases = [36737, 40000, 43000, 45000, 50000]
    print("\n  Checking potential Area 16 bases for flag 16000007:")
    for base in potential_bases:
        if base < len(event_flags):
            val = event_flags[base]
            is_set = (val >> 0) & 1
            print(f"    base {base}: 0x{val:02X}, bit 0 = {'SET' if is_set else '---'}")


if __name__ == "__main__":
    main()
