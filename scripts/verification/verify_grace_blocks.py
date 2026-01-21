#!/usr/bin/env python3
"""
Verify grace block bases using known graces.

Discovery: Different grace sub-ranges may use different base offsets.
- 71800-71899 (Tutorial): base 2625 (verified)
- 71600-71699 (Volcano Manor): base 2750 (discovered 2026-01-21)

Testing if other grace ranges also have different bases.
"""

import sys
from typing import Optional, Dict, List, Tuple

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


def check_flag(event_flags: bytes, flag_id: int, base: int, block_start: int) -> Tuple[Optional[bool], int, int]:
    """Returns (is_set, byte_offset, bit_pos)"""
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)
    if byte_offset >= len(event_flags):
        return (None, byte_offset, bit_pos)
    byte_val = event_flags[byte_offset]
    is_set = (byte_val >> bit_pos) & 1 == 1
    return (is_set, byte_offset, bit_pos)


# Grace ranges and their expected bases (from ground_truth + new discovery)
GRACE_RANGES = {
    # (block_start, suspected_base, range_name)
    71000: (2625, "Block 71000 (Tutorial, etc.)"),
    71600: (2750, "Sub-block 71600 (Volcano Manor) - TESTING"),
    72000: (2750, "Block 72000 (DLC Enir-Ilim)"),
    73000: (2662, "Block 73000 (Dungeon graces)"),
    74000: (3000, "Block 74000 (DLC dungeon graces)"),
    76000: (3250, "Block 76000 (Limgrave world graces)"),
    78000: (3500, "Block 78000 (Grace guidance)"),
}

# Known graces for testing
KNOWN_GRACES = [
    # Tutorial/Early graces
    (71800, "Cave of Knowledge", True),  # Almost everyone has this
    (71801, "Stranded Graveyard", True),

    # Volcano Manor (testing new base)
    (71607, "Subterranean Inquisition Chamber", True),  # User confirmed
    (71601, "Volcano Manor", None),  # Unknown
    (71600, "Audience Pathway", False),  # Post-Rykard, likely not

    # Limgrave (should be set for mid-game)
    (76100, "The First Step", True),
    (76101, "Church of Elleh", True),
    (76102, "Gatefront", None),
    (76117, "Stormhill Shack", None),

    # DLC (probably not set)
    (72000, "Divine Gate Front Staircase", None),
]


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

    print("\n" + "="*80)
    print("GRACE BLOCK VERIFICATION")
    print("="*80)

    # Test each known grace with its suspected block base
    for flag_id, name, expected in KNOWN_GRACES:
        # Determine which block this grace belongs to
        if 71600 <= flag_id < 71700:
            block_start = 71600
            base = 2750  # New discovery
        elif 71000 <= flag_id < 72000:
            block_start = 71000
            base = 2625
        elif 72000 <= flag_id < 73000:
            block_start = 72000
            base = 2750
        elif 76000 <= flag_id < 77000:
            block_start = 76000
            base = 3250
        else:
            continue

        is_set, byte_off, bit = check_flag(event_flags, flag_id, base, block_start)

        # Status display
        status = "SET" if is_set else "---"
        if expected is True:
            match = "✓" if is_set else "✗ MISMATCH"
        elif expected is False:
            match = "✓" if not is_set else "✗ MISMATCH"
        else:
            match = "?"

        print(f"  {flag_id} {name[:40]:40} [{block_start}+{base}] byte {byte_off}, bit {bit}: {status} {match}")

    # Verify the hypothesis: check if 716xx range really needs base 2750
    print("\n" + "="*80)
    print("HYPOTHESIS VERIFICATION: 716xx range base offset")
    print("="*80)

    print("\nChecking 71607 (Subterranean Inquisition Chamber, CONFIRMED SET by user):")

    # Test with original block 71000 base
    is_set_old, byte_old, bit_old = check_flag(event_flags, 71607, 2625, 71000)
    print(f"  Base 2625 (block 71000): byte {byte_old}, bit {bit_old} = {'SET' if is_set_old else 'NOT SET'}")

    # Test with new sub-block 71600 base
    is_set_new, byte_new, bit_new = check_flag(event_flags, 71607, 2750, 71600)
    print(f"  Base 2750 (sub-block 71600): byte {byte_new}, bit {bit_new} = {'SET' if is_set_new else 'NOT SET'}")

    # Verify 71800/71801 still work with old base
    print("\nVerifying 71800-71801 still use base 2625:")
    for fid in [71800, 71801]:
        is_set, byte_off, bit = check_flag(event_flags, fid, 2625, 71000)
        print(f"  {fid}: byte {byte_off}, bit {bit} = {'SET' if is_set else 'NOT SET'}")

    # Check if there's a pattern - maybe 71000-71599 and 71600-71999 have different bases?
    print("\n" + "="*80)
    print("PATTERN ANALYSIS: Do different 71xxx sub-ranges use different bases?")
    print("="*80)

    # Check the byte contents around the expected locations
    print("\nBytes at key offsets:")
    key_bytes = [2700, 2725, 2750, 2775, 2800, 2825]
    for b in key_bytes:
        val = event_flags[b]
        print(f"  byte {b}: 0x{val:02X} = {val:08b}")

    # Final conclusion
    print("\n" + "="*80)
    print("CONCLUSION")
    print("="*80)

    if is_set_new and not is_set_old:
        print("\n✓ CONFIRMED: Flag 71607 is SET at base 2750, NOT SET at base 2625")
        print("  The 71600-71699 range uses a different base offset (2750) than")
        print("  the 71800-71899 range (2625). Difference: +125 bytes.")
        print("\n  This suggests grace blocks may have sub-ranges with different bases:")
        print("    71000-71599: base 2625 (flags 0-599, bytes 2625-2699)")
        print("    71600-71999: base 2750 (flags 600-999, bytes 2750-2874)")
        print("    OR")
        print("    71600-71699 has its own allocation separate from 71000 block")


if __name__ == "__main__":
    main()
