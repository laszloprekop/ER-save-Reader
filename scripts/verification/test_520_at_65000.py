#!/usr/bin/env python3
"""
Test 520xxx flags at base 65000 (interpolated from 510000/540000 pattern).

Pattern:
  510000 → 63750
  520000 → 65000 (predicted)
  530000 → 66250 (predicted)
  540000 → 67500

Also compare against base 1341 to see which is more accurate.
"""

import sys
import struct
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser


# All verified items with 520xxx flags (from inventory_verification.rs)
ALL_520_ITEMS = {
    # Spirit Ashes
    258000: ("Lhutel the Headless", 520000),
    234000: ("Demi-Human Ashes", 520010),
    241000: ("Noble Sorcerer Ashes", 520020),
    5050: ("Assassin's Crimson Dagger", 520030),
    202000: ("Banished Knight Engvall", 520040),
    219000: ("Twinsage Sorcerer Ashes", 520050),
    218000: ("Glintstone Sorcerer Ashes", 520060),
    256000: ("Ancient Dragon Knight Kristoff", 520080),
    239000: ("Bloodhound Knight Floh", 520090),
    3060000: ("Ordovis's Greatsword", 520100),
    217000: ("Perfumer Tricia", 520110),
    246000: ("Soldjars of Fortune Ashes", 520130),
    243000: ("Mad Pumpkin Head Ashes", 520140),
    224000: ("Kindred of Rot Ashes", 520150),
    257000: ("Redmane Knight Ogha", 520160),
    8050000: ("Zamor Curved Sword", 520170),
    228000: ("Blackflame Monk Amon", 520200),
    # Talismans
    5060: ("Assassin's Cerulean Dagger", 520210),
    2160: ("Lord of Blood's Exultation", 520220),
    1020: ("Viridian Amber Medallion", 520300),
    4010: ("Spelldrake Talisman", 520310),
    4020: ("Flamedrake Talisman", 520330),
    2110: ("Blue Dancer Charm", 520350),
    2080: ("Winged Sword Insignia", 520360),
    1010: ("Cerulean Amber Medallion", 520370),
    2170: ("Kindred of Rot's Exultation", 520390),
    44010000: ("Jar Cannon", 520400),
    15020000: ("Great Omenkiller Cleaver", 520410),
    6010: ("Concealing Veil", 520420),
    215000: ("Putrid Corpse Ashes", 520430),
    4022: ("Flamedrake Talisman +2", 520440),
    1110: ("Gold Scarab", 520450),
    3170000: ("Golden Order Greatsword", 520470),
    5040: ("Godskin Swaddling Cloth", 520480),
    13020000: ("Family Heads", 520490),
}


def check_inventory(slot_raw: bytes, item_id: int) -> bool:
    """Check if item is in inventory."""
    patterns = [
        struct.pack('<I', item_id),
        struct.pack('<I', 0x40000000 | (item_id & 0x0FFFFFFF)),
        struct.pack('<I', 0x20000000 | (item_id & 0x0FFFFFFF)),
    ]
    return any(p in slot_raw for p in patterns)


def test_base(ef_s0: bytes, ef_s1: bytes, s0_raw: bytes, s1_raw: bytes, base: int, label: str):
    """Test a base offset against all items."""
    print(f"\n{'='*70}")
    print(f"TESTING BASE {base} ({label})")
    print(f"{'='*70}")

    matches = 0
    both_set = 0
    not_set = 0
    inverted = 0
    not_in_inv = 0

    for item_id, (name, flag_id) in ALL_520_ITEMS.items():
        byte_offset = base + (flag_id - 520000) // 8
        bit = 7 - (flag_id % 8)

        in_s0 = check_inventory(s0_raw, item_id)
        in_s1 = check_inventory(s1_raw, item_id)

        if byte_offset >= len(ef_s0) or byte_offset < 0:
            print(f"  OUT OF RANGE: {flag_id} ({name}) @ offset {byte_offset}")
            continue

        s0_byte = ef_s0[byte_offset]
        s1_byte = ef_s1[byte_offset]
        s0_bit = (s0_byte >> bit) & 1
        s1_bit = (s1_byte >> bit) & 1

        # Expected: if in S0 and not in S1, then s0_bit=1, s1_bit=0
        if in_s0 and not in_s1:
            if s0_bit == 1 and s1_bit == 0:
                matches += 1
                print(f"  OK: {flag_id} {name}")
            elif s0_bit == 1 and s1_bit == 1:
                both_set += 1
                print(f"  BOTH SET: {flag_id} {name}")
            elif s0_bit == 0:
                not_set += 1
                print(f"  NOT SET: {flag_id} {name} (S0={s0_bit}, S1={s1_bit})")
            else:
                inverted += 1
                print(f"  INVERTED: {flag_id} {name} (S0={s0_bit}, S1={s1_bit})")
        else:
            not_in_inv += 1
            # print(f"  SKIP: {flag_id} {name} (not differential: S0={in_s0}, S1={in_s1})")

    total_testable = matches + both_set + not_set + inverted
    print(f"\nResults for base {base}:")
    print(f"  Matches: {matches}/{total_testable}")
    print(f"  Both set: {both_set}")
    print(f"  Not set: {not_set}")
    print(f"  Inverted: {inverted}")
    print(f"  Skipped (not differential): {not_in_inv}")

    return matches


def main():
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    parser = SaveParser()
    parsed = parser.parse(save_path)

    with open(save_path, 'rb') as f:
        raw_save = f.read()

    ef_s0 = parsed.slots[0].event_flags
    ef_s1 = parsed.slots[1].event_flags

    s0_raw = raw_save[parsed.slots[0].slot_offset:parsed.slots[0].slot_offset + 2000000]
    s1_raw = raw_save[parsed.slots[1].slot_offset:parsed.slots[1].slot_offset + 2000000]

    # Print inventory summary
    print("=" * 70)
    print("INVENTORY SUMMARY")
    print("=" * 70)

    s0_count = sum(1 for item_id in ALL_520_ITEMS if check_inventory(s0_raw, item_id))
    s1_count = sum(1 for item_id in ALL_520_ITEMS if check_inventory(s1_raw, item_id))
    diff_count = sum(1 for item_id in ALL_520_ITEMS
                     if check_inventory(s0_raw, item_id) and not check_inventory(s1_raw, item_id))

    print(f"S0 has {s0_count} items with 520xxx flags")
    print(f"S1 has {s1_count} items with 520xxx flags")
    print(f"Differential (S0 - S1): {diff_count} items")

    # Test both bases
    results = {}
    results[1341] = test_base(ef_s0, ef_s1, s0_raw, s1_raw, 1341, "discovered via differential")
    results[65000] = test_base(ef_s0, ef_s1, s0_raw, s1_raw, 65000, "interpolated from 510k/540k")

    # Also test some nearby values
    print("\n" + "=" * 70)
    print("SCANNING RANGE AROUND 65000")
    print("=" * 70)

    best_base = None
    best_matches = 0

    for scan_base in range(64900, 65100):
        matches = 0
        for item_id, (name, flag_id) in ALL_520_ITEMS.items():
            byte_offset = scan_base + (flag_id - 520000) // 8
            bit = 7 - (flag_id % 8)

            in_s0 = check_inventory(s0_raw, item_id)
            in_s1 = check_inventory(s1_raw, item_id)

            if not (in_s0 and not in_s1):
                continue

            if byte_offset >= len(ef_s0) or byte_offset < 0:
                continue

            s0_bit = (ef_s0[byte_offset] >> bit) & 1
            s1_bit = (ef_s1[byte_offset] >> bit) & 1

            if s0_bit == 1 and s1_bit == 0:
                matches += 1

        if matches > best_matches:
            best_matches = matches
            best_base = scan_base

    print(f"\nBest base in 64900-65100 range: {best_base} with {best_matches} matches")

    if best_base and best_matches > 0:
        test_base(ef_s0, ef_s1, s0_raw, s1_raw, best_base, "best in 65k range")

    # Summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"\nBase 1341: {results[1341]} matches (from early differential)")
    print(f"Base 65000: {results[65000]} matches (from 510k/540k pattern)")
    if best_base:
        print(f"Best 65k range: {best_base} with {best_matches} matches")


if __name__ == "__main__":
    main()
