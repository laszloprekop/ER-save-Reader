#!/usr/bin/env python3
"""
Extended probe for Volcano Manor graces.

Hypothesis: Grace flags might use different bases for different sub-ranges:
- 71800-71999: base 2625 (verified via Cave of Knowledge)
- 71600-71799: different base (Volcano Manor area)

Testing if base 2750 works for 71600-71699.
"""

import sys
from typing import Optional

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

# Validation flags
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


def check_flag(event_flags: bytes, flag_id: int, base: int, block_start: int = 71000) -> Optional[bool]:
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)
    if byte_offset >= len(event_flags):
        return None
    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


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

    # VM graces
    vm_graces = [
        (71600, "Audience Pathway (post-Rykard)"),
        (71601, "Volcano Manor"),
        (71602, "Prison Town Church"),
        (71603, "Temple of Eiglay"),
        (71604, "Guest Hall"),
        (71605, "Abductor Virgin"),
        (71606, "Rykard, Lord of Blasphemy"),
        (71607, "Subterranean Inquisition Chamber"),
    ]

    # Test hypothesis: VM graces at different base
    print("\n" + "="*70)
    print("HYPOTHESIS TEST: Different bases for grace sub-ranges")
    print("="*70)

    # Test various bases specifically for 71600-71699 range
    test_bases = [2625, 2650, 2675, 2700, 2725, 2750, 2775, 2800]

    print("\nVolcano Manor graces (71600-71607) at various bases:")
    print("-"*70)

    for base in test_bases:
        set_count = 0
        results = []
        for flag_id, name in vm_graces:
            is_set = check_flag(event_flags, flag_id, base)
            if is_set:
                set_count += 1
            status = "SET" if is_set else "---"
            results.append((flag_id, name, status))

        # Show results for this base
        print(f"\nBase {base} ({set_count} SET):")
        for flag_id, name, status in results:
            print(f"  {flag_id} {name[:35]:35} {status}")

    # Also check what flags 71800-71807 look like with base 2625
    print("\n" + "="*70)
    print("VERIFICATION: Tutorial graces (71800-71807) with base 2625")
    print("="*70)

    tutorial_graces = [
        (71800, "Cave of Knowledge"),
        (71801, "Stranded Graveyard"),
        (71802, "Fringefolk Hero's Grave"),
        (71803, "Unknown 71803"),
        (71804, "Unknown 71804"),
        (71805, "Unknown 71805"),
        (71806, "Unknown 71806"),
        (71807, "Unknown 71807"),
    ]

    for flag_id, name in tutorial_graces:
        is_set = check_flag(event_flags, flag_id, 2625)
        status = "SET" if is_set else "---"
        relative = flag_id - 71000
        byte_off = 2625 + relative // 8
        bit = 7 - (flag_id % 8)
        print(f"  {flag_id} {name[:30]:30} byte {byte_off}, bit {bit}: {status}")

    # Check if the confirmed graces from the user align with any base
    print("\n" + "="*70)
    print("CROSS-VALIDATION: Which bases give plausible VM exploration pattern?")
    print("="*70)
    print("\nExpected for mid-game VM explorer:")
    print("  - 71601 (VM main entrance): SHOULD BE SET")
    print("  - 71607 (Subterranean Inquisition Chamber): CONFIRMED SET by user")
    print("  - 71600 (Audience Pathway): Probably NOT SET (post-Rykard)")
    print("  - 71606 (Rykard grace): Probably NOT SET")

    for base in test_bases:
        main_entrance = check_flag(event_flags, 71601, base)
        inquisition = check_flag(event_flags, 71607, base)
        post_rykard = check_flag(event_flags, 71600, base)
        rykard_grace = check_flag(event_flags, 71606, base)

        # Score: 71601 SET (+1), 71607 SET (+2 - confirmed), 71600 NOT SET (+1), 71606 NOT SET (+1)
        score = 0
        if main_entrance: score += 1
        if inquisition: score += 2  # Confirmed by user
        if not post_rykard: score += 1
        if not rykard_grace: score += 1

        print(f"\nBase {base} (score {score}/5):")
        print(f"  71601 VM entrance:  {'SET' if main_entrance else '---'}")
        print(f"  71607 Inquisition:  {'SET' if inquisition else '---'} (MUST be SET)")
        print(f"  71600 post-Rykard:  {'SET' if post_rykard else '---'} (expect NOT SET)")
        print(f"  71606 Rykard:       {'SET' if rykard_grace else '---'} (expect NOT SET)")


if __name__ == "__main__":
    main()
