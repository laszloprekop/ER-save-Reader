#!/usr/bin/env python3
"""
Pinpoint 520xxx flag locations using inventory differential.

Evidence:
- Slot 0: Has 18 items with 520xxx flags
- Slots 1-4: Missing most of these items

Strategy:
- For each item present in S0 but absent in S1:
  - Find bytes where expected bit is SET in S0 but UNSET in S1
  - This gives us the exact flag location
"""

import sys
from pathlib import Path
from collections import defaultdict
from dataclasses import dataclass
from typing import Dict, List, Tuple

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser


# Items present in S0 but not in S1 (from inventory discovery)
DIFFERENTIAL_ITEMS = {
    520000: "Lhutel the Headless",
    520030: "Assassin's Crimson Dagger",
    520040: "Banished Knight Engvall",
    520050: "Twinsage Sorcerer Ashes",
    520090: "Bloodhound Knight Floh",
    520110: "Perfumer Tricia",
    520210: "Assassin's Cerulean Dagger",
    520300: "Viridian Amber Medallion",
    520310: "Spelldrake Talisman",
    520330: "Flamedrake Talisman",
    520350: "Blue Dancer Charm",
    520370: "Cerulean Amber Medallion",
    520390: "Kindred of Rot's Exultation",
    520450: "Gold Scarab",
    520480: "Godskin Swaddling Cloth",
}


@dataclass
class FlagLocation:
    """Discovered flag location."""
    flag_id: int
    item_name: str
    byte_offset: int
    bit: int
    implied_base: int
    s0_byte: int
    s1_byte: int


def find_flag_location(
    ef_s0: bytes,
    ef_s1: bytes,
    flag_id: int,
    item_name: str,
) -> List[FlagLocation]:
    """
    Find where a flag is stored by looking for differential.

    If item is in S0 but not S1, the flag should be SET in S0 but UNSET in S1.
    """
    candidates = []
    block_start = 520000
    expected_bit = 7 - (flag_id % 8)
    relative = flag_id - block_start

    for offset in range(len(ef_s0)):
        s0_byte = ef_s0[offset]
        s1_byte = ef_s1[offset]

        # Skip if both are same or if it's padding
        if s0_byte == s1_byte:
            continue
        if s0_byte == 0xFF or s1_byte == 0xFF:
            continue

        # Check if expected bit is SET in S0, UNSET in S1
        s0_bit = (s0_byte >> expected_bit) & 1
        s1_bit = (s1_byte >> expected_bit) & 1

        if s0_bit == 1 and s1_bit == 0:
            implied_base = offset - (relative // 8)
            candidates.append(FlagLocation(
                flag_id=flag_id,
                item_name=item_name,
                byte_offset=offset,
                bit=expected_bit,
                implied_base=implied_base,
                s0_byte=s0_byte,
                s1_byte=s1_byte,
            ))

    return candidates


def main():
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    parser = SaveParser()
    parsed = parser.parse(save_path)

    ef_s0 = parsed.slots[0].event_flags
    ef_s1 = parsed.slots[1].event_flags

    print(f"Pinpointing 520xxx flag locations")
    print(f"S0 EF: {len(ef_s0)} bytes, S1 EF: {len(ef_s1)} bytes")
    print(f"Items to locate: {len(DIFFERENTIAL_ITEMS)}")

    # Find locations for each flag
    all_locations = []
    base_votes = defaultdict(list)

    for flag_id, item_name in DIFFERENTIAL_ITEMS.items():
        locations = find_flag_location(ef_s0, ef_s1, flag_id, item_name)

        print(f"\n{item_name} (flag {flag_id}):")
        print(f"  Expected bit: {7 - (flag_id % 8)}")
        print(f"  Candidates: {len(locations)}")

        if locations:
            for loc in locations[:5]:  # Show first 5
                print(f"    offset={loc.byte_offset}, bit={loc.bit}, "
                      f"S0=0x{loc.s0_byte:02X}, S1=0x{loc.s1_byte:02X}, "
                      f"implied_base={loc.implied_base}")
                base_votes[loc.implied_base].append(loc)
            all_locations.extend(locations[:5])

    # Analyze base candidates
    print(f"\n{'='*60}")
    print("BASE OFFSET ANALYSIS")
    print(f"{'='*60}")

    sorted_bases = sorted(base_votes.items(), key=lambda x: len(x[1]), reverse=True)

    print("\nTop base candidates by flag count:")
    for base, flags in sorted_bases[:10]:
        flag_ids = [f.flag_id for f in flags]
        print(f"\n  Base {base}: {len(flags)} flags")
        print(f"    Flags: {flag_ids}")

        # Check if these flags form a consistent pattern
        if len(flags) >= 3:
            print(f"    Checking consistency...")
            consistent = True
            for f in flags:
                expected_offset = base + (f.flag_id - 520000) // 8
                if f.byte_offset != expected_offset:
                    print(f"      INCONSISTENT: flag {f.flag_id} at {f.byte_offset}, expected {expected_offset}")
                    consistent = False
            if consistent:
                print(f"    CONSISTENT! All flags match formula.")

    # Try to find THE base
    print(f"\n{'='*60}")
    print("VALIDATION")
    print(f"{'='*60}")

    # Test top candidate
    if sorted_bases:
        best_base, best_flags = sorted_bases[0]
        print(f"\nTesting best candidate: base {best_base}")

        # Validate all known flags with this base
        matches = 0
        for flag_id, item_name in DIFFERENTIAL_ITEMS.items():
            byte_offset = best_base + (flag_id - 520000) // 8
            bit = 7 - (flag_id % 8)

            if byte_offset < len(ef_s0):
                s0_bit = (ef_s0[byte_offset] >> bit) & 1
                s1_bit = (ef_s1[byte_offset] >> bit) & 1

                if s0_bit == 1 and s1_bit == 0:
                    matches += 1
                    print(f"  OK: {flag_id} ({item_name})")
                elif s0_bit == 1:
                    print(f"  BOTH SET: {flag_id} ({item_name})")
                else:
                    print(f"  NOT SET: {flag_id} ({item_name})")

        print(f"\nMatch rate: {matches}/{len(DIFFERENTIAL_ITEMS)}")

        if matches == len(DIFFERENTIAL_ITEMS):
            print(f"\n*** VERIFIED: Block 520000 base = {best_base} ***")
            print(f"\nground_truth_offsets.json entry:")
            print(f'''
  "520000": {{
    "block_start": 520000,
    "base_offset": {best_base},
    "block_size": 1000,
    "status": "verified",
    "notes": "Spirit Ash/Talisman catacomb rewards. Discovered via inventory-driven differential. {matches}/{len(DIFFERENTIAL_ITEMS)} items verified."
  }}''')


if __name__ == "__main__":
    main()
