#!/usr/bin/env python3
"""
Search for the actual locations of high 520xxx flags (520200+).

These flags don't appear to be at base 1341 like 520000-520110.
They might be in a separate block or use a different formula.
"""

import sys
import struct
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser


# Items with high 520xxx flags that we know are in S0 but NOT in S1
HIGH_520_ITEMS = {
    # These are confirmed present in S0, absent in S1
    1020: ("Viridian Amber Medallion", 520300),
    4010: ("Spelldrake Talisman", 520310),
    2110: ("Blue Dancer Charm", 520350),
    1010: ("Cerulean Amber Medallion", 520370),
    2170: ("Kindred of Rot's Exultation", 520390),
    5040: ("Godskin Swaddling Cloth", 520480),
    # These showed BOTH SET at base 1341 (need re-search)
    5060: ("Assassin's Cerulean Dagger", 520210),
    4020: ("Flamedrake Talisman", 520330),
    1110: ("Gold Scarab", 520450),
}


def search_flag_location(ef_s0: bytes, ef_s1: bytes, flag_id: int, item_name: str):
    """Search entire EF section for where this flag might be."""
    expected_bit = 7 - (flag_id % 8)
    candidates = []

    for offset in range(min(len(ef_s0), len(ef_s1))):
        s0_byte = ef_s0[offset]
        s1_byte = ef_s1[offset]

        # Skip if same in both slots
        if s0_byte == s1_byte:
            continue

        s0_bit = (s0_byte >> expected_bit) & 1
        s1_bit = (s1_byte >> expected_bit) & 1

        # We want: SET in S0, UNSET in S1
        if s0_bit == 1 and s1_bit == 0:
            # Calculate what base this would imply
            if flag_id >= 520000:
                implied_base = offset - (flag_id - 520000) // 8
                candidates.append({
                    'offset': offset,
                    'bit': expected_bit,
                    'implied_base': implied_base,
                    's0_byte': s0_byte,
                    's1_byte': s1_byte,
                })

    return candidates


def main():
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    parser = SaveParser()
    parsed = parser.parse(save_path)

    # Also load raw save to verify inventory
    with open(save_path, 'rb') as f:
        raw_save = f.read()

    ef_s0 = parsed.slots[0].event_flags
    ef_s1 = parsed.slots[1].event_flags

    s0_raw = raw_save[parsed.slots[0].slot_offset:parsed.slots[0].slot_offset + 2000000]
    s1_raw = raw_save[parsed.slots[1].slot_offset:parsed.slots[1].slot_offset + 2000000]

    print("=" * 80)
    print("SEARCHING FOR HIGH 520xxx FLAG LOCATIONS")
    print("=" * 80)

    # First, confirm inventory status
    print("\nInventory verification:")
    for item_id, (name, flag_id) in HIGH_520_ITEMS.items():
        # Check for item in both inventories
        patterns = [
            struct.pack('<I', item_id),
            struct.pack('<I', 0x20000000 | (item_id & 0x0FFFFFFF)),
        ]

        in_s0 = any(p in s0_raw for p in patterns)
        in_s1 = any(p in s1_raw for p in patterns)

        status = f"S0:{'YES' if in_s0 else 'no'} S1:{'YES' if in_s1 else 'no'}"
        diff = "DIFF" if in_s0 and not in_s1 else ("BOTH" if in_s0 and in_s1 else "")
        print(f"  {name} ({flag_id}): {status}  {diff}")

    # Search for each high flag
    print("\n" + "-" * 80)
    print("SEARCHING FOR FLAG LOCATIONS")
    print("-" * 80)

    all_implied_bases = {}

    for item_id, (name, flag_id) in HIGH_520_ITEMS.items():
        print(f"\n{name} (flag {flag_id}):")

        candidates = search_flag_location(ef_s0, ef_s1, flag_id, name)

        if candidates:
            print(f"  Found {len(candidates)} candidate locations")
            for c in candidates[:5]:
                print(f"    offset={c['offset']}, implied_base={c['implied_base']}, "
                      f"S0=0x{c['s0_byte']:02X}, S1=0x{c['s1_byte']:02X}")

                # Track implied bases
                ib = c['implied_base']
                if ib not in all_implied_bases:
                    all_implied_bases[ib] = []
                all_implied_bases[ib].append((flag_id, name, c['offset']))
        else:
            print(f"  No candidates found!")

    # Analyze implied bases
    print("\n" + "=" * 80)
    print("IMPLIED BASE ANALYSIS")
    print("=" * 80)

    sorted_bases = sorted(all_implied_bases.items(), key=lambda x: len(x[1]), reverse=True)

    print("\nTop implied bases by flag count:")
    for implied_base, flags in sorted_bases[:15]:
        print(f"\n  Base {implied_base}: {len(flags)} flags")
        for flag_id, name, offset in flags:
            print(f"    {flag_id} ({name}) @ offset {offset}")

    # Test the top candidate
    if sorted_bases:
        best_base, best_flags = sorted_bases[0]
        print("\n" + "=" * 80)
        print(f"TESTING TOP CANDIDATE: BASE {best_base}")
        print("=" * 80)

        # Test all HIGH_520_ITEMS at this base
        matches = 0
        for item_id, (name, flag_id) in HIGH_520_ITEMS.items():
            byte_offset = best_base + (flag_id - 520000) // 8
            bit = 7 - (flag_id % 8)

            if byte_offset >= len(ef_s0) or byte_offset < 0:
                print(f"  OUT OF RANGE: {flag_id} ({name}) @ offset {byte_offset}")
                continue

            s0_byte = ef_s0[byte_offset]
            s1_byte = ef_s1[byte_offset]
            s0_bit = (s0_byte >> bit) & 1
            s1_bit = (s1_byte >> bit) & 1

            if s0_bit == 1 and s1_bit == 0:
                matches += 1
                print(f"  OK: {flag_id} ({name})")
            elif s0_bit == 1 and s1_bit == 1:
                print(f"  BOTH SET: {flag_id} ({name})")
            elif s0_bit == 0:
                print(f"  NOT SET in S0: {flag_id} ({name})")
            else:
                print(f"  INVERTED: {flag_id} ({name}) S0={s0_bit}, S1={s1_bit}")

        print(f"\nMatch rate: {matches}/{len(HIGH_520_ITEMS)}")


if __name__ == "__main__":
    main()
