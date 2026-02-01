#!/usr/bin/env python3
"""
Analyze the 520xxx block structure to understand padding gaps.

Verified: Base 1341 gives 12/15 matches
Issue: 3 flags (520210, 520330, 520450) land in 0xFF padding bytes

This script maps out the actual data vs padding layout.
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
    print("520xxx BLOCK STRUCTURE ANALYSIS")
    print("=" * 80)
    print(f"\nBase: {base}")
    print(f"Block: 520000-520999 would span offsets {base} to {base + 124}")

    # Map out each byte in the potential block
    print("\n" + "-" * 80)
    print("BYTE MAP (offset: S0_byte / S1_byte => flag range)")
    print("-" * 80)

    # Key flags we care about
    key_flags = {
        520000: "Lhutel", 520030: "AssassinCrimson", 520040: "Engvall",
        520050: "Twinsage", 520090: "Floh", 520110: "Tricia",
        520210: "AssassinCerulean", 520300: "ViridianAmber", 520310: "Spelldrake",
        520330: "Flamedrake", 520350: "BlueDancer", 520370: "CeruleanAmber",
        520390: "KindredRot", 520450: "GoldScarab", 520480: "Swaddling",
    }

    # Create a visual map
    padding_bytes = []
    data_bytes = []

    for rel_byte in range(70):  # Cover flags up to 520559
        offset = base + rel_byte
        flag_start = 520000 + rel_byte * 8
        flag_end = flag_start + 7

        s0_byte = ef_s0[offset]
        s1_byte = ef_s1[offset]

        # Check if this is padding
        is_padding = (s0_byte == 0xFF and s1_byte == 0xFF)

        # Check for key flags in this byte
        flags_in_byte = [f for f in key_flags if flag_start <= f <= flag_end]
        flag_names = [key_flags[f] for f in flags_in_byte]

        if is_padding:
            padding_bytes.append(rel_byte)
            marker = "PADDING"
        else:
            data_bytes.append(rel_byte)
            marker = "data"

        # Only print interesting bytes
        if is_padding or flags_in_byte or (s0_byte != s1_byte):
            flag_info = f"  [{', '.join(flag_names)}]" if flag_names else ""
            diff = "DIFF" if s0_byte != s1_byte else ""
            print(f"  +{rel_byte:3d} ({offset:4d}): S0=0x{s0_byte:02X} S1=0x{s1_byte:02X}  "
                  f"{marker:8s} flags {flag_start}-{flag_end}{flag_info} {diff}")

    print("\n" + "-" * 80)
    print("SUMMARY")
    print("-" * 80)
    print(f"\nData bytes: {len(data_bytes)} (positions: {min(data_bytes) if data_bytes else 'N/A'} - {max(data_bytes) if data_bytes else 'N/A'})")
    print(f"Padding bytes: {len(padding_bytes)} (positions: {padding_bytes[:10]}...)" if padding_bytes else "")

    # Check which key flags hit padding
    print("\n" + "-" * 80)
    print("KEY FLAG ANALYSIS")
    print("-" * 80)

    for flag_id, name in sorted(key_flags.items()):
        rel_byte = (flag_id - 520000) // 8
        offset = base + rel_byte
        bit = 7 - (flag_id % 8)

        s0_byte = ef_s0[offset]
        s1_byte = ef_s1[offset]
        s0_bit = (s0_byte >> bit) & 1
        s1_bit = (s1_byte >> bit) & 1

        is_padding = (s0_byte == 0xFF and s1_byte == 0xFF)

        if is_padding:
            status = "PADDING (0xFF/0xFF)"
        elif s0_bit == 1 and s1_bit == 0:
            status = "OK (S0=1, S1=0)"
        elif s0_bit == 1 and s1_bit == 1:
            status = "BOTH SET"
        else:
            status = f"S0={s0_bit}, S1={s1_bit}"

        print(f"  {flag_id} ({name:15s}): +{rel_byte:2d} offset={offset} bit={bit} => {status}")

    # Conclusion
    print("\n" + "=" * 80)
    print("CONCLUSION")
    print("=" * 80)

    # Count how many key flags land in padding
    padding_flags = []
    ok_flags = []
    for flag_id in key_flags:
        rel_byte = (flag_id - 520000) // 8
        offset = base + rel_byte
        s0_byte = ef_s0[offset]
        s1_byte = ef_s1[offset]
        if s0_byte == 0xFF and s1_byte == 0xFF:
            padding_flags.append(flag_id)
        else:
            bit = 7 - (flag_id % 8)
            s0_bit = (s0_byte >> bit) & 1
            s1_bit = (s1_byte >> bit) & 1
            if s0_bit == 1 and s1_bit == 0:
                ok_flags.append(flag_id)

    print(f"\nFlags verified OK: {len(ok_flags)}")
    print(f"Flags in padding: {len(padding_flags)}: {padding_flags}")

    if padding_flags:
        print(f"\nThe 520xxx block at base 1341 has gaps at certain offsets.")
        print("Flags landing in these gaps show false 'BOTH SET' because 0xFF has all bits set.")
        print("\nFor partial verification, we can record:")
        print(f'''
  "520000": {{
    "block_start": 520000,
    "base_offset": {base},
    "block_size": 500,
    "status": "partial",
    "verified_flags": {ok_flags},
    "notes": "Block has 0xFF gaps. {len(ok_flags)}/{len(key_flags)} key flags verified. Flags {padding_flags} land in padding gaps."
  }}''')


if __name__ == "__main__":
    main()
