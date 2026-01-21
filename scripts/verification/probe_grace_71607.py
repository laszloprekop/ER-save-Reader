#!/usr/bin/env python3
"""
Probe for the correct offset of grace flag 71607 (Subterranean Inquisition Chamber).

User confirmed: Slot 0 Confessor HAS this grace (userMarkedComplete=true)
Current formula gives: byte 2700, bit 0 = NOT SET (webappParsedStatus=false)

This script searches for the correct offset where bit 0 is SET.
"""

import sys
from typing import Optional, List, Tuple

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

# Validation flags to detect event_flags section
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]


def detect_event_flags_offset(slot_data: bytes, search_start: int = 0x12000) -> Optional[int]:
    """Detect the event_flags section offset within slot data."""
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


def find_set_bit_positions(event_flags: bytes, bit_pos: int, search_range: Tuple[int, int]) -> List[int]:
    """Find all byte offsets where the specified bit is SET."""
    start, end = search_range
    results = []
    for byte_offset in range(start, min(end, len(event_flags))):
        byte_val = event_flags[byte_offset]
        if (byte_val >> bit_pos) & 1 == 1:
            results.append(byte_offset)
    return results


def main():
    save_path = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"
    slot_index = 0  # Confessor

    if len(sys.argv) > 1:
        save_path = sys.argv[1]
    if len(sys.argv) > 2:
        slot_index = int(sys.argv[2])

    print(f"Probing for grace 71607 (Subterranean Inquisition Chamber)")
    print(f"Save: {save_path}")
    print(f"Slot: {slot_index}")
    print()

    # Read slot data
    with open(save_path, 'rb') as f:
        f.seek(HEADER_SIZE + slot_index * SLOT_SIZE)
        slot_data = f.read(SLOT_SIZE)

    # Detect event_flags offset
    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    print(f"Detected event_flags offset: 0x{event_flags_offset:X}")
    event_flags = slot_data[event_flags_offset:]

    # Current formula: byte 2700, bit 0
    current_byte = 2700
    current_bit = 0
    current_val = event_flags[current_byte]
    current_is_set = (current_val >> current_bit) & 1 == 1
    print(f"\nCurrent formula: byte {current_byte}, bit {current_bit}")
    print(f"  Value at byte {current_byte}: 0x{current_val:02X} = {current_val:08b}")
    print(f"  Bit {current_bit} is {'SET' if current_is_set else 'NOT SET'}")

    # For flag 71607, bit position is 7 - (71607 % 8) = 7 - 7 = 0
    # So we need to find byte offsets where bit 0 is SET
    target_bit = 0

    # Search around the expected area (2500-3000)
    print(f"\nSearching for bytes where bit {target_bit} is SET (range 2500-3000)...")
    matches = find_set_bit_positions(event_flags, target_bit, (2500, 3000))
    print(f"Found {len(matches)} candidates")

    if matches:
        print("\nCandidates with bit 0 SET:")
        for byte_off in matches[:30]:  # Show first 30
            # Calculate what base this would imply for block 71000
            # byte_offset = base + (flag_id - 71000) / 8
            # byte_offset = base + 607 / 8 = base + 75
            # So: base = byte_offset - 75
            implied_base = byte_off - 75
            byte_val = event_flags[byte_off]
            print(f"  byte {byte_off}: 0x{byte_val:02X} -> implies base {implied_base} for block 71000")

    # Also check what other VM graces might look like
    # 71600 = Audience Pathway (post-Rykard) - bit 7 - (71600 % 8) = 7 - 0 = 7
    # 71601-71606 are other VM graces
    print("\n" + "="*60)
    print("Checking known Volcano Manor grace bits at various bases:")
    print("="*60)

    # VM graces: 71600-71607
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

    # Try a few candidate bases
    candidate_bases = [2625, 2550, 2650, 2700, 2750]

    for base in candidate_bases:
        print(f"\nBase {base}:")
        for flag_id, name in vm_graces:
            relative = flag_id - 71000
            byte_off = base + relative // 8
            bit_pos = 7 - (flag_id % 8)
            if byte_off < len(event_flags):
                byte_val = event_flags[byte_off]
                is_set = (byte_val >> bit_pos) & 1 == 1
                status = "SET" if is_set else "---"
                print(f"  {flag_id} ({name[:30]:30}): byte {byte_off}, bit {bit_pos} = {status}")


if __name__ == "__main__":
    main()
