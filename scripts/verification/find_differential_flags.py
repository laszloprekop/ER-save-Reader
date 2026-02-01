#!/usr/bin/env python3
"""
Find all differentially set flags between two slots.
This helps identify what content has been completed and provides
candidate flags for formula discovery.
"""

import sys
from pathlib import Path
from collections import defaultdict

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser
from scripts.verification.ground_truth_loader import (
    load_block_bases,
    load_dungeon_bases,
    get_tile_config,
)


def find_differentials(ef_s0: bytes, ef_s1: bytes, max_results: int = 100):
    """
    Find byte positions where S0 has bits set that S1 doesn't.

    Returns list of (byte_offset, bit, s0_val, s1_val)
    """
    differentials = []

    for offset in range(min(len(ef_s0), len(ef_s1))):
        s0_byte = ef_s0[offset]
        s1_byte = ef_s1[offset]

        # Skip if both are same or if S0 is 0xFF (padding)
        if s0_byte == s1_byte or s0_byte == 0xFF:
            continue

        # Find bits that are SET in S0 but NOT in S1
        for bit in range(8):
            s0_bit = (s0_byte >> bit) & 1
            s1_bit = (s1_byte >> bit) & 1

            if s0_bit == 1 and s1_bit == 0:
                differentials.append((offset, bit, s0_byte, s1_byte))

        if len(differentials) >= max_results:
            break

    return differentials


def reverse_lookup_flag(offset: int, bit: int) -> list:
    """
    Try to reverse-calculate possible flag IDs from offset and bit.
    """
    candidates = []

    # Block flags
    block_bases = load_block_bases()
    for block_start, info in block_bases.items():
        base_offset = info['base_offset']
        if offset >= base_offset:
            relative_byte = offset - base_offset
            # flag_id = block_start + relative_byte * 8 + (7 - bit)
            flag_id = block_start + relative_byte * 8 + (7 - bit)
            if flag_id < block_start + 10000:  # Reasonable range
                candidates.append(('block', block_start, flag_id))

    # Dungeon flags
    dungeon_bases = load_dungeon_bases()
    for map_area, info in dungeon_bases.items():
        base_offset = info['base_offset']
        section_size = 1125

        if offset >= base_offset:
            relative = offset - base_offset
            section = relative // section_size
            local_byte = relative % section_size
            local_id = local_byte * 8 + (7 - bit)

            if section < 100 and local_id < 10000:
                flag_id = int(f"{map_area:02d}{section:02d}{local_id:04d}")
                candidates.append(('dungeon', map_area, flag_id))

    # Tile flags (more complex)
    tile_config = get_tile_config()
    if tile_config:
        tile_base = tile_config.get('base_offset', 485330)
        bytes_per_slot = tile_config.get('bytes_per_slot', 875)
        slots_per_row = tile_config.get('slots_per_row', 40)
        row_base = tile_config.get('row_base', 33)
        col_base = tile_config.get('col_base', 30)

        if offset >= tile_base:
            relative = offset - tile_base
            tile_slot = relative // bytes_per_slot
            local_byte = relative % bytes_per_slot
            local_id = local_byte * 8 + (7 - bit)

            row = row_base + tile_slot // slots_per_row
            col = col_base + tile_slot % slots_per_row

            if row < 99 and col < 99 and local_id < 10000:
                flag_id = int(f"10{row:02d}{col:02d}{local_id:04d}")
                candidates.append(('tile', (row, col), flag_id))

    return candidates


def main():
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    parser = SaveParser()
    parsed = parser.parse(save_path)

    ef_s0 = parsed.slots[0].event_flags
    ef_s1 = parsed.slots[1].event_flags

    print(f"Finding differential flags (S0 SET, S1 UNSET)...")
    print(f"S0: {len(ef_s0)} bytes, S1: {len(ef_s1)} bytes\n")

    differentials = find_differentials(ef_s0, ef_s1, max_results=200)

    print(f"Found {len(differentials)} differential positions\n")

    # Group by offset range
    by_range = defaultdict(list)
    for offset, bit, s0_val, s1_val in differentials:
        range_start = (offset // 1000) * 1000
        by_range[range_start].append((offset, bit, s0_val, s1_val))

    print("By offset range:")
    for range_start in sorted(by_range.keys()):
        items = by_range[range_start]
        print(f"  {range_start:>6} - {range_start + 999}: {len(items)} differentials")

    print("\n" + "="*60)
    print("DETAILED ANALYSIS (first 50)")
    print("="*60)

    for i, (offset, bit, s0_val, s1_val) in enumerate(differentials[:50]):
        candidates = reverse_lookup_flag(offset, bit)

        print(f"\n[{i+1}] Offset {offset}, bit {bit}")
        print(f"    S0: 0x{s0_val:02X} ({bin(s0_val)})")
        print(f"    S1: 0x{s1_val:02X} ({bin(s1_val)})")

        if candidates:
            for flag_type, context, flag_id in candidates[:3]:
                print(f"    -> {flag_type}: {flag_id} (context: {context})")
        else:
            print(f"    -> No known formula matches")

    # Look specifically for midrange flags (100000-999999)
    print("\n" + "="*60)
    print("SEARCHING FOR MIDRANGE FLAGS (510xxx, 520xxx, 540xxx)")
    print("="*60)

    # For midrange, we know some bases:
    # 510000 -> base 63750 (verified)
    # 540000 -> base 67500 (verified)
    midrange_bases = {
        510000: 63750,
        540000: 67500,
    }

    for block_start, base in midrange_bases.items():
        print(f"\nBlock {block_start} (base {base}):")

        # Check first 100 flags in this block
        for flag_offset in range(0, 100):
            flag_id = block_start + flag_offset
            byte_offset = base + flag_offset // 8
            bit = 7 - (flag_id % 8)

            if byte_offset < len(ef_s0):
                s0_set = (ef_s0[byte_offset] >> bit) & 1
                s1_set = (ef_s1[byte_offset] >> bit) & 1

                if s0_set and not s1_set:
                    print(f"  {flag_id}: SET in S0, unset in S1 (offset {byte_offset}, bit {bit})")

    # Search for potential 520000 base
    print("\n" + "="*60)
    print("SEARCHING FOR 520xxx BLOCK")
    print("="*60)

    # Try to find 520000 flag (bit 7) anywhere in the first 100k bytes
    print("\nSearching for flag 520000 (need bit 7 set in S0, unset in S1)...")

    candidates_520 = []
    for offset in range(0, min(100000, len(ef_s0))):
        s0_byte = ef_s0[offset]
        s1_byte = ef_s1[offset]

        # Bit 7 for 520000
        s0_bit7 = (s0_byte >> 7) & 1
        s1_bit7 = (s1_byte >> 7) & 1

        if s0_bit7 == 1 and s1_bit7 == 0 and s0_byte != 0xFF:
            # This could be 520000 flag
            # implied base = offset - 0 = offset
            candidates_520.append((offset, s0_byte))

    print(f"Found {len(candidates_520)} candidate locations for 520000")
    for offset, byte_val in candidates_520[:20]:
        print(f"  offset={offset}, byte=0x{byte_val:02X}")


if __name__ == "__main__":
    main()
