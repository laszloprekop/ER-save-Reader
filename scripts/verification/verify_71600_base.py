#!/usr/bin/env python3
"""
Verify correct base for sub-block 71600 (VM graces).

Two candidates found:
- Base 2726: 71606/71607 at byte 2726 (0x1F)
- Base 2750: 71607 at byte 2825 (0x01) - original discovery

Need to determine which is actually correct.
"""

import sys
from typing import Optional

SAVE_PATH = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"
SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
]

VM_GRACES = [
    (71600, "Audience Pathway", False),  # Post-Rykard - user confirmed NOT defeated
    (71601, "Volcano Manor", None),       # Unknown
    (71602, "Prison Town Church", None),
    (71603, "Temple of Eiglay", None),
    (71604, "Guest Hall", None),
    (71605, "Abductor Virgin", True),     # User may have this
    (71606, "Rykard grace", False),       # Post-Rykard - should be NOT SET
    (71607, "Subterranean Inquisition Chamber", True),  # User CONFIRMED
]


def detect_event_flags_offset(slot_data: bytes) -> Optional[int]:
    for test_offset in range(0x12000, 0x15000):
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
    with open(SAVE_PATH, 'rb') as f:
        f.seek(HEADER_SIZE + 0 * SLOT_SIZE)
        slot_data = f.read(SLOT_SIZE)

    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    event_flags = slot_data[event_flags_offset:]

    print("="*70)
    print("VM GRACE BASE VERIFICATION")
    print("="*70)

    # Check both candidate bases
    for base in [2726, 2750]:
        print(f"\n--- Base {base} for sub-block 71600 ---")
        score = 0
        total = 0

        for flag_id, name, expected in VM_GRACES:
            relative = flag_id - 71600
            byte_off = base + relative // 8
            bit_pos = 7 - (flag_id % 8)

            byte_val = event_flags[byte_off]
            is_set = (byte_val >> bit_pos) & 1 == 1

            status = "SET" if is_set else "---"

            if expected is not None:
                total += 1
                if is_set == expected:
                    score += 1
                    match = "✓"
                else:
                    match = "✗"
            else:
                match = "?"

            print(f"  {flag_id} {name[:35]:35} byte {byte_off}, bit {bit_pos}: {status} {match}")

        if total > 0:
            print(f"\n  Match score: {score}/{total} ({score/total*100:.1f}%)")

    # Show raw bytes
    print("\n" + "="*70)
    print("RAW BYTE VALUES")
    print("="*70)
    for byte_off in [2726, 2825]:
        val = event_flags[byte_off]
        print(f"  byte {byte_off}: 0x{val:02X} = {val:08b}")


if __name__ == "__main__":
    main()
