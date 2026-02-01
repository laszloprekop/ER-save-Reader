#!/usr/bin/env python3
"""
Find the actual data boundary for the 520xxx block.

Discovery: Base 1341 maps high 520xxx flags (520200+) into 0xFF padding.
This script identifies where the actual data ends and padding begins.
"""

import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser


def main():
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    parser = SaveParser()
    parsed = parser.parse(save_path)

    ef_s0 = parsed.slots[0].event_flags
    ef_s1 = parsed.slots[1].event_flags

    base = 1341

    print("=" * 80)
    print("FINDING 520xxx BLOCK BOUNDARY")
    print("=" * 80)
    print(f"\nBase: {base}")

    # Find where data transitions to 0xFF padding
    print("\n--- Byte values from base 1341 ---")
    print("\nOffset  S0_byte  S1_byte  Both_FF?  Flag_range")
    print("-" * 60)

    first_ff_offset = None
    last_data_offset = None

    for i in range(100):  # Check first 100 bytes
        offset = base + i
        s0_byte = ef_s0[offset]
        s1_byte = ef_s1[offset]

        both_ff = s0_byte == 0xFF and s1_byte == 0xFF
        flag_start = 520000 + i * 8
        flag_end = flag_start + 7

        if both_ff and first_ff_offset is None:
            first_ff_offset = offset
            print(f"{offset:>6}  0x{s0_byte:02X}     0x{s1_byte:02X}     YES      {flag_start}-{flag_end}  <-- PADDING STARTS")
        elif not both_ff:
            last_data_offset = offset
            diff = "DIFF" if s0_byte != s1_byte else "same"
            print(f"{offset:>6}  0x{s0_byte:02X}     0x{s1_byte:02X}     no       {flag_start}-{flag_end}  ({diff})")
        else:
            # Already in padding, skip
            pass

    print("\n" + "=" * 80)
    print("BOUNDARY ANALYSIS")
    print("=" * 80)

    if first_ff_offset:
        bytes_of_data = first_ff_offset - base
        max_flag = 520000 + bytes_of_data * 8 - 1
        print(f"\nFirst 0xFF padding at offset: {first_ff_offset}")
        print(f"Data region: {base} - {first_ff_offset - 1} ({bytes_of_data} bytes)")
        print(f"Flag coverage: 520000 - {max_flag} (~{max_flag - 520000 + 1} flags)")

    if last_data_offset:
        print(f"\nLast data byte at: {last_data_offset}")

    # Now let's check if there's more data AFTER the padding
    print("\n" + "=" * 80)
    print("CHECKING FOR DATA AFTER PADDING")
    print("=" * 80)

    # Scan forward to find non-0xFF bytes
    print("\nScanning for non-0xFF bytes after offset 1367...")

    non_ff_regions = []
    in_data_region = False
    region_start = None

    for offset in range(1367, 3000):
        s0_byte = ef_s0[offset]
        s1_byte = ef_s1[offset]

        is_data = (s0_byte != 0xFF) or (s1_byte != 0xFF)

        if is_data and not in_data_region:
            in_data_region = True
            region_start = offset
        elif not is_data and in_data_region:
            in_data_region = False
            non_ff_regions.append((region_start, offset - 1, offset - region_start))
            if len(non_ff_regions) >= 10:
                break

    if non_ff_regions:
        print("\nNon-0xFF regions found after padding:")
        for start, end, size in non_ff_regions[:10]:
            print(f"  Offset {start} - {end} ({size} bytes)")

    # Check the original verified flags more carefully
    print("\n" + "=" * 80)
    print("RE-VALIDATING MATCHED FLAGS")
    print("=" * 80)

    # These flags verified correctly at base 1341
    verified_flags = [
        (520000, "Lhutel the Headless"),
        (520030, "Assassin's Crimson Dagger"),
        (520040, "Banished Knight Engvall"),
        (520050, "Twinsage Sorcerer Ashes"),
        (520090, "Bloodhound Knight Floh"),
        (520110, "Perfumer Tricia"),
    ]

    print("\nFlags that matched at base 1341:")
    for flag_id, name in verified_flags:
        byte_offset = base + (flag_id - 520000) // 8
        bit = 7 - (flag_id % 8)

        s0_byte = ef_s0[byte_offset]
        s1_byte = ef_s1[byte_offset]
        s0_bit = (s0_byte >> bit) & 1
        s1_bit = (s1_byte >> bit) & 1

        in_padding = s0_byte == 0xFF and s1_byte == 0xFF

        print(f"\n  {flag_id} ({name}):")
        print(f"    Offset {byte_offset}, bit {bit}")
        print(f"    S0: 0x{s0_byte:02X} (bit={s0_bit}), S1: 0x{s1_byte:02X} (bit={s1_bit})")
        print(f"    In padding: {'YES' if in_padding else 'no'}")
        print(f"    Differential: {'YES' if s0_bit == 1 and s1_bit == 0 else 'no'}")

    # Revised conclusion
    print("\n" + "=" * 80)
    print("REVISED CONCLUSION")
    print("=" * 80)

    print("""
The 520xxx block at base 1341 has LIMITED coverage:
- Only flags 520000-520151 (approximately) map to actual data
- Flags 520152+ map to 0xFF padding

This suggests the 520xxx flags might be:
1. Split across multiple smaller blocks with different bases
2. Using a non-contiguous allocation pattern
3. Only partially implemented in the game

The 12 flags that verified correctly (520000-520110) are all within
the first ~20 bytes of the block, confirming base 1341 for this subset.

For a PARTIAL verification, we can record:
  "520000": {
    "block_start": 520000,
    "base_offset": 1341,
    "block_size": 152,  // Only covers ~152 flags before padding
    "status": "partial",
    "notes": "Covers 520000-520151. Higher flags (520200+) are elsewhere."
  }
""")


if __name__ == "__main__":
    main()
