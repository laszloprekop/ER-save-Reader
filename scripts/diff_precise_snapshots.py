#!/usr/bin/env python3
"""
Diff precise before/after snapshots to find exact flag byte locations.

Uses the granular snapshots that were taken immediately before/after specific actions.
"""

import sys
from pathlib import Path
from collections import defaultdict

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser


def diff_event_flags(before_flags: bytes, after_flags: bytes) -> list:
    """Find all bit differences between two event flag sections."""
    changes = []
    min_len = min(len(before_flags), len(after_flags))

    for byte_off in range(min_len):
        if before_flags[byte_off] != after_flags[byte_off]:
            before_byte = before_flags[byte_off]
            after_byte = after_flags[byte_off]
            diff = before_byte ^ after_byte

            for bit in range(8):
                if (diff >> bit) & 1:
                    before_bit = (before_byte >> bit) & 1
                    after_bit = (after_byte >> bit) & 1
                    changes.append({
                        'byte_offset': byte_off,
                        'bit_position': bit,  # Physical bit (0-7 from right)
                        'logical_bit': 7 - bit,  # Logical bit used in flag formula
                        'direction': 'SET' if after_bit else 'CLEARED',
                        'before_byte': f'0x{before_byte:02X}',
                        'after_byte': f'0x{after_byte:02X}',
                    })

    return changes


def reverse_calc_flag_id(byte_offset: int, logical_bit: int, block_start: int) -> int:
    """
    Calculate flag ID from byte offset and bit position.

    Formula: byte_offset = base_offset + (flag_id - block_start) // 8
             logical_bit = 7 - (flag_id % 8)

    Therefore:
      flag_id % 8 = 7 - logical_bit
      flag_id = block_start + (byte_offset - base_offset) * 8 + (7 - logical_bit)
    """
    # We need to find base_offset, which is what we're trying to discover
    # For reverse calculation with known flag: flag_id = block_start + relative
    # where relative // 8 = byte_offset - base_offset
    #   and relative % 8 = 7 - logical_bit

    remainder = 7 - logical_bit
    # relative = (byte_offset - base_offset) * 8 + remainder
    # But we don't know base_offset yet

    # What we CAN do: for a KNOWN flag, calculate what base_offset MUST be
    return remainder  # Return just the remainder for combining with byte info


def main():
    snapshot_dir = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")
    parser = SaveParser()

    # Define before/after pairs and what flag they should affect
    # (before_file, after_file, slot_index, expected_flag_name, expected_flag_range)
    pairs = [
        # Gatefront grace (should be 76xxx)
        ("ER0000.sl2 Wretch - 21 Limgrave, before Gatefront grace",
         "ER0000.sl2 Wretch - 22 Limgrave, touched Gatefront grace",
         1, "Gatefront Grace", 76000),

        # The First Step grace (76100 - we know this already)
        ("ER0000.sl2 Wretch - 14 Limgrave, before The First Step grace",
         "ER0000.sl2 Wretch - 15 Limgrave, touched The First Step grace",
         1, "The First Step", 76000),

        # Church of Elleh grace (76101 - we know this already)
        ("ER0000.sl2 Wretch - 17 Limgrave, before Church of Elleh grace",
         "ER0000.sl2 Wretch - 19 Limgrave, touched Church of Elleh grace",
         1, "Church of Elleh", 76000),

        # Cave of Knowledge grace (71800 - we know this already)
        ("ER0000.sl2 Wretch - 03 Cave of knowledge, before Site of grace",
         "ER0000.sl2 Wretch - 04 Cave of knowledge, touched Site of grace",
         1, "Cave of Knowledge", 71000),

        # Stranded Graveyard grace (71801 - we know this already)
        ("ER0000.sl2 Wretch - 11 Stranded Graveyard, before touching grace",
         "ER0000.sl2 Wretch - 12 Stranded Graveyard, after touching grace",
         1, "Stranded Graveyard", 71000),

        # Missionary Cookbook [4] (67xxx)
        ("ER0000.sl2 Confessor - 01 before Missionary Cookbok [4] pickup",
         "ER0000.sl2 Confessor - 02 after Missionary Cookbok [4] picked up",
         0, "Missionary Cookbook [4]", 67000),

        # Minor Eldtree Church grace (should be 76xxx)
        ("ER0000.sl2 Confessor - 03 before touching  Minor Eldtree Church grace",
         "ER0000.sl2 Confessor - 04 after touched Minor Eldtree Church grace",
         0, "Minor Erdtree Church Grace", 76000),

        # Agheel Lake North grace (should be 76xxx)
        ("ER0000.sl2 Wretch - 27 Limgrave, approaching Agheel Lake North grace, on mount",
         "ER0000.sl2 Wretch - 28 Limgrave, touched Agheel Lake North grace, dismounted",
         1, "Agheel Lake North Grace", 76000),

        # Smoldering Butterfly pickup (tile flag 1043500010)
        ("ER0000.sl2 Confessor - 09 before picking up Smoldering Butterfly treasure_m60_43_50_00_1043500010",
         "ER0000.sl2 Confessor - 10 after picked up Smoldering Butterfly treasure_m60_43_50_00_1043500010",
         0, "Smoldering Butterfly", 1043500000),  # Tile flag

        # Limgrave Map pickup
        ("ER0000.sl2 Wretch - 33 Limgrave, rested at Agheel Lake North grace, continue game",
         "ER0000.sl2 Wretch - 34 Limgrave Map picked, moved to south of Wayward cellar sarchophagi",
         1, "Limgrave West Map", 62000),
    ]

    print("=" * 80)
    print("PRECISE SNAPSHOT DIFF ANALYSIS")
    print("=" * 80)

    for before_file, after_file, slot_idx, expected_name, expected_range in pairs:
        before_path = snapshot_dir / before_file
        after_path = snapshot_dir / after_file

        if not before_path.exists() or not after_path.exists():
            print(f"\nSkipping {expected_name}: Files not found")
            continue

        print(f"\n{'=' * 80}")
        print(f"{expected_name}")
        print(f"{'=' * 80}")
        print(f"Before: {before_file}")
        print(f"After:  {after_file}")
        print(f"Slot:   {slot_idx}")

        # Parse both saves
        before_save = parser.parse(before_path, [slot_idx])
        after_save = parser.parse(after_path, [slot_idx])

        if not before_save.slots or not after_save.slots:
            print("  Error: Could not parse slots")
            continue

        before_flags = before_save.slots[0].event_flags
        after_flags = after_save.slots[0].event_flags

        # Find differences
        changes = diff_event_flags(before_flags, after_flags)

        print(f"\nTotal bit changes: {len(changes)}")

        if len(changes) == 0:
            print("  No changes detected!")
            continue

        # Separate SET vs CLEARED
        set_changes = [c for c in changes if c['direction'] == 'SET']
        cleared_changes = [c for c in changes if c['direction'] == 'CLEARED']

        print(f"  SET: {len(set_changes)}, CLEARED: {len(cleared_changes)}")

        # Show SET changes (most interesting for flag discovery)
        print("\nSET changes:")
        for c in set_changes[:20]:  # Show up to 20
            byte_off = c['byte_offset']
            logical_bit = c['logical_bit']
            remainder = 7 - logical_bit  # flag_id % 8

            # For block-based flags, we can compute potential flag IDs
            # If this is a 76xxx flag, flag_id = 76000 + offset*8 + remainder
            # but we need to know base_offset to compute offset

            # Instead, show the byte info and let us derive base manually
            print(f"  byte={byte_off}, bit={logical_bit} (physical={c['bit_position']})")
            print(f"    {c['before_byte']} -> {c['after_byte']}")

            # If we know expected_range, calculate what base_offset would be
            # for various flag IDs in that range
            if 60000 <= expected_range < 80000:
                # Block-based flag
                block_start = (expected_range // 1000) * 1000
                # Possible flag_ids given this byte and bit:
                # flag_id = block_start + X where X % 8 == remainder and X // 8 == byte_off - base_offset
                # So for small X (0-999), base_offset = byte_off - X // 8
                print(f"    If block {block_start}xxx, possible bases:")
                for x in range(0, 1000):
                    if x % 8 == remainder:
                        potential_base = byte_off - (x // 8)
                        flag_id = block_start + x
                        if 1000 <= potential_base <= 5000:  # Reasonable range
                            print(f"      flag={flag_id} → base={potential_base}")
                            if x > 200:  # Stop after first few
                                break

        # For known validation flags, verify the calculation
        if expected_range == 71000:
            # Tutorial graces: known base = 2625
            # Check if any change matches expected pattern
            print("\n  Verification (known base=2625):")
            for c in set_changes:
                byte_off = c['byte_offset']
                logical_bit = c['logical_bit']
                # Calculate flag ID: flag_id = 71000 + (byte_off - 2625) * 8 + (7 - logical_bit)
                if byte_off >= 2625:
                    calc_flag_id = 71000 + (byte_off - 2625) * 8 + (7 - logical_bit)
                    if 71000 <= calc_flag_id < 72000:
                        print(f"    byte={byte_off}, bit={logical_bit} → flag {calc_flag_id}")

        if expected_range == 76000:
            # World graces: known base = 3250
            print("\n  Verification (known base=3250):")
            for c in set_changes:
                byte_off = c['byte_offset']
                logical_bit = c['logical_bit']
                if byte_off >= 3250:
                    calc_flag_id = 76000 + (byte_off - 3250) * 8 + (7 - logical_bit)
                    if 76000 <= calc_flag_id < 77000:
                        print(f"    byte={byte_off}, bit={logical_bit} → flag {calc_flag_id}")

        if expected_range == 67000:
            # Cookbooks: known base = 3987 (verified)
            print("\n  Verification (known base=3987):")
            for c in set_changes:
                byte_off = c['byte_offset']
                logical_bit = c['logical_bit']
                if byte_off >= 3987:
                    calc_flag_id = 67000 + (byte_off - 3987) * 8 + (7 - logical_bit)
                    if 67000 <= calc_flag_id < 68000:
                        print(f"    byte={byte_off}, bit={logical_bit} → flag {calc_flag_id}")


if __name__ == "__main__":
    main()
