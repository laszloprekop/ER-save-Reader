#!/usr/bin/env python3
"""
Investigate the anomalous 520xxx flags that are SET in S1 despite items being absent.

Flags: 520210, 520330, 520450
Items: Assassin's Cerulean Dagger, Flamedrake Talisman, Gold Scarab

These are NOT in S1 inventory, but their calculated flag locations have the bit set.
This indicates either:
1. Wrong base offset for these specific flags
2. These flags use a different formula
3. The calculated location maps to a different flag that happens to be set
"""

import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser

ANOMALOUS_FLAGS = [
    (520210, "Assassin's Cerulean Dagger"),
    (520330, "Flamedrake Talisman"),
    (520450, "Gold Scarab"),
]

# Include neighboring flags for comparison
NEIGHBORING_FLAGS = [
    (520200, "Blackflame Monk Amon"),
    (520210, "Assassin's Cerulean Dagger"),  # ANOMALY
    (520220, "Lord of Blood's Exultation"),
    (520300, "Viridian Amber Medallion"),
    (520310, "Spelldrake Talisman"),
    (520320, "Unknown 520320"),
    (520330, "Flamedrake Talisman"),  # ANOMALY
    (520340, "Unknown 520340"),
    (520350, "Blue Dancer Charm"),
    (520440, "Flamedrake Talisman +2"),
    (520450, "Gold Scarab"),  # ANOMALY
    (520460, "Unknown 520460"),
    (520470, "Golden Order Greatsword"),
    (520480, "Godskin Swaddling Cloth"),
]


def main():
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    parser = SaveParser()
    parsed = parser.parse(save_path)

    ef_s0 = parsed.slots[0].event_flags
    ef_s1 = parsed.slots[1].event_flags

    base = 1341

    print("=" * 80)
    print("INVESTIGATING 520xxx ANOMALIES")
    print("=" * 80)
    print(f"\nBase offset: {base}")
    print("\nFlag layout at base 1341:")
    print("  flag_id = block_start + byte_relative * 8 + (7 - bit)")
    print("  byte_offset = base + (flag_id - 520000) // 8")
    print("  bit = 7 - (flag_id % 8)")

    print("\n" + "-" * 80)
    print("DETAILED BYTE ANALYSIS FOR ANOMALOUS FLAGS")
    print("-" * 80)

    for flag_id, name in ANOMALOUS_FLAGS:
        relative = flag_id - 520000
        byte_offset = base + relative // 8
        bit = 7 - (flag_id % 8)

        s0_byte = ef_s0[byte_offset]
        s1_byte = ef_s1[byte_offset]

        s0_bit_val = (s0_byte >> bit) & 1
        s1_bit_val = (s1_byte >> bit) & 1

        print(f"\n{flag_id} ({name}):")
        print(f"  Calculation: base({base}) + relative({relative}) // 8 = offset {byte_offset}")
        print(f"  Bit position: 7 - ({flag_id} % 8) = 7 - {flag_id % 8} = {bit}")
        print(f"  S0 byte @ {byte_offset}: 0x{s0_byte:02X} = {bin(s0_byte)}")
        print(f"  S1 byte @ {byte_offset}: 0x{s1_byte:02X} = {bin(s1_byte)}")
        print(f"  S0 bit {bit}: {s0_bit_val}  |  S1 bit {bit}: {s1_bit_val}")

        # Show all 8 flags in this byte
        print(f"\n  All flags in this byte (offset {byte_offset}):")
        byte_start_flag = 520000 + ((byte_offset - base) * 8)
        for b in range(8):
            f = byte_start_flag + (7 - b)
            s0_b = (s0_byte >> b) & 1
            s1_b = (s1_byte >> b) & 1
            marker = " <-- ANOMALY" if f == flag_id else ""
            print(f"    Flag {f}: S0={s0_b}, S1={s1_b}{marker}")

    print("\n" + "-" * 80)
    print("TESTING ALTERNATIVE BASE OFFSETS")
    print("-" * 80)

    # Search for a base where all 3 anomalous flags show S0=1, S1=0
    print("\nSearching for alternative base where anomalous flags match expectation...")

    for test_base in range(1300, 1400):
        all_match = True
        for flag_id, _ in ANOMALOUS_FLAGS:
            byte_offset = test_base + (flag_id - 520000) // 8
            bit = 7 - (flag_id % 8)

            if byte_offset >= len(ef_s0) or byte_offset >= len(ef_s1):
                all_match = False
                break

            s0_bit = (ef_s0[byte_offset] >> bit) & 1
            s1_bit = (ef_s1[byte_offset] >> bit) & 1

            if s0_bit != 1 or s1_bit != 0:
                all_match = False
                break

        if all_match:
            print(f"  Candidate base {test_base}: All 3 anomalous flags match S0=1, S1=0")

    print("\n" + "-" * 80)
    print("HYPOTHESIS: FLAGS MAY BE IN DIFFERENT SUB-BLOCKS")
    print("-" * 80)

    print("\nThe 520xxx range appears to have multiple sub-blocks with different bases.")
    print("Let's group flags by their byte offset and look for patterns.\n")

    # Test if certain flag ranges have different bases
    test_ranges = [
        (520000, 520099, "520000-520099 (Spirit Ashes A)"),
        (520100, 520199, "520100-520199 (Weapons/Spirit Ashes B)"),
        (520200, 520299, "520200-520299 (Talismans A)"),
        (520300, 520399, "520300-520399 (Talismans B)"),
        (520400, 520499, "520400-520499 (Mixed)"),
    ]

    for range_start, range_end, label in test_ranges:
        print(f"\n{label}:")

        # Find best base for this sub-range
        best_base = None
        best_score = 0

        for test_base in range(1300, 1400):
            score = 0
            for flag_id in range(range_start, range_end + 1, 10):
                byte_offset = test_base + (flag_id - 520000) // 8
                bit = 7 - (flag_id % 8)

                if byte_offset < len(ef_s0):
                    s0_bit = (ef_s0[byte_offset] >> bit) & 1
                    s1_bit = (ef_s1[byte_offset] >> bit) & 1

                    # We want to see differentiation (not all same)
                    if s0_bit != s1_bit:
                        score += 1

            if score > best_score:
                best_score = score
                best_base = test_base

        if best_base:
            print(f"  Best base: {best_base} (score: {best_score})")

    print("\n" + "-" * 80)
    print("CHECKING IF ANOMALOUS FLAGS ARE FROM DIFFERENT GAME ACTIVITY")
    print("-" * 80)

    print("\nThese items might have been obtained through different means in S1:")
    print("  - Assassin's Cerulean Dagger: Catacomb reward (Black Knife Catacombs)")
    print("  - Flamedrake Talisman: Catacomb reward (Groveside Cave)")
    print("  - Gold Scarab: Abandoned Cave boss drop")
    print("\nIf S1 visited these locations but didn't get the specific item,")
    print("a different flag at the same byte could be set.")


if __name__ == "__main__":
    main()
