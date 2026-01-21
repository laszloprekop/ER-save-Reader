#!/usr/bin/env python3
"""
Deep probe for Stormveil graces (71000-71099 range).

From verification records, Confessor has these Stormveil graces confirmed:
- 71000 Godrick the Grafted
- 71001 Margit, the Fell Omen
- 71002 Castleward Tunnel
- 71003 Gateside Chamber
- 71004 Stormveil Cliffside
- 71005 Rampart Tower
- 71006 Liftside Chamber
- 71007 Secluded Cell
- 71008 Stormveil Main Gate

Probe found base 2821 gives 77.8% match. Let's find exact base.
"""

import sys
from typing import Optional, List, Tuple

SAVE_PATH = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"
SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Stormveil graces from verification records (Confessor confirmed as discovered)
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

# Also check nearby: 71109 (Divine Bridge) was confirmed
OTHER_71XXX = [
    (71109, "Divine Bridge"),
    (71402, "Church of the Cuckoo"),  # Raya Lucaria
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


def check_flag(event_flags: bytes, flag_id: int, block_start: int, base: int) -> Tuple[Optional[bool], int, int]:
    """Returns (is_set, byte_offset, bit_pos)"""
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)
    if byte_offset >= len(event_flags) or byte_offset < 0:
        return (None, byte_offset, bit_pos)
    byte_val = event_flags[byte_offset]
    is_set = (byte_val >> bit_pos) & 1 == 1
    return (is_set, byte_offset, bit_pos)


def main():
    with open(SAVE_PATH, 'rb') as f:
        f.seek(HEADER_SIZE + 0 * SLOT_SIZE)
        slot_data = f.read(SLOT_SIZE)

    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    print(f"Event flags offset: 0x{event_flags_offset:X}")
    event_flags = slot_data[event_flags_offset:]

    print("\n" + "="*70)
    print("STORMVEIL GRACE PROBE (71000-71008)")
    print("="*70)

    # Test range of bases
    best_bases = []
    for base in range(2700, 2900):
        matches = 0
        total = len(STORMVEIL_GRACES)
        for flag_id, name in STORMVEIL_GRACES:
            is_set, _, _ = check_flag(event_flags, flag_id, 71000, base)
            if is_set:
                matches += 1
        if matches > 0:
            best_bases.append((base, matches, total))

    best_bases.sort(key=lambda x: -x[1])

    print("\nTop 10 bases for Stormveil graces:")
    for base, matches, total in best_bases[:10]:
        pct = matches / total * 100
        print(f"  Base {base}: {matches}/{total} ({pct:.1f}%)")

    # Show detailed breakdown for top bases
    print("\n" + "-"*70)
    print("DETAILED FLAG CHECK FOR TOP BASES")
    print("-"*70)

    for base in [best_bases[0][0], 2821, 2725]:
        print(f"\n=== Base {base} ===")
        for flag_id, name in STORMVEIL_GRACES:
            is_set, byte_off, bit = check_flag(event_flags, flag_id, 71000, base)
            status = "SET" if is_set else "---"
            byte_val = event_flags[byte_off] if byte_off < len(event_flags) else 0
            print(f"  {flag_id} {name[:30]:30} byte {byte_off}, bit {bit}: {status} (0x{byte_val:02X})")

    # Check which bytes have the expected pattern (71000-71008 should all be SET)
    print("\n" + "="*70)
    print("BYTE PATTERN ANALYSIS")
    print("="*70)

    # For 71000-71008 to all be SET, we need a byte pattern like:
    # 71000-71007: byte X, bits 7-0 (all 8 bits)
    # 71008: byte X+1, bit 7
    # So we need: byte X = 0xFF (or high bits set), byte X+1 bit 7 = 1

    print("\nSearching for bytes where 71000-71007 would all be SET...")
    for byte_off in range(2700, 2900):
        byte_val = event_flags[byte_off]
        next_val = event_flags[byte_off + 1] if byte_off + 1 < len(event_flags) else 0

        # Check if all 8 bits of this byte are set (71000-71007)
        # AND bit 7 of next byte is set (71008)
        bits_set = bin(byte_val).count('1')
        next_bit7 = (next_val >> 7) & 1

        if bits_set >= 7:  # At least 7 of 8 bits set
            print(f"  byte {byte_off}: 0x{byte_val:02X} ({bits_set} bits), next byte bit 7: {next_bit7}")
            # Implied base would be: base = byte_off - 0 = byte_off
            # For flag 71000 at this byte: base + (71000-71000)/8 = base
            print(f"    -> Implied base: {byte_off}")

    # Check 71109 (Divine Bridge) which was also confirmed
    print("\n" + "="*70)
    print("71109 (Divine Bridge) CHECK")
    print("="*70)

    for base in [2821, 2725, 2750]:
        is_set, byte_off, bit = check_flag(event_flags, 71109, 71000, base)
        byte_val = event_flags[byte_off] if byte_off < len(event_flags) else 0
        status = "SET" if is_set else "---"
        print(f"  Base {base}: byte {byte_off}, bit {bit} = {status} (0x{byte_val:02X})")

    # What if 71100+ uses a different sub-block?
    print("\n  Checking with sub-block 71100:")
    for base in range(2700, 2900, 25):
        is_set, byte_off, bit = check_flag(event_flags, 71109, 71100, base)
        if is_set:
            print(f"    Base {base}: byte {byte_off}, bit {bit} = SET")


if __name__ == "__main__":
    main()
