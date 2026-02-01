#!/usr/bin/env python3
"""
Analyze the 520000 region in more detail.

Findings from discover_unknown_block.py:
- Base 1254 works for flags 520000-520060
- Flags 520080+ fall into 0xFF padding region

This script investigates:
1. Where exactly does the 0xFF padding start?
2. Are higher flags (520080+) stored elsewhere?
3. What's the actual structure of this region?
"""

import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser


def analyze_region(ef_data: bytes, start: int, end: int, label: str):
    """Analyze a region of event flags."""
    print(f"\n{'='*60}")
    print(f"Region: {label} (bytes {start}-{end})")
    print(f"{'='*60}")

    region = ef_data[start:end]

    # Count 0xFF bytes
    ff_count = sum(1 for b in region if b == 0xFF)
    zero_count = sum(1 for b in region if b == 0x00)

    print(f"Total bytes: {len(region)}")
    print(f"0xFF bytes: {ff_count} ({ff_count/len(region)*100:.1f}%)")
    print(f"0x00 bytes: {zero_count} ({zero_count/len(region)*100:.1f}%)")
    print(f"Other bytes: {len(region) - ff_count - zero_count}")

    # Show byte values
    print(f"\nByte values (hex):")
    for i in range(0, min(len(region), 64), 8):
        bytes_str = ' '.join(f'{b:02X}' for b in region[i:i+8])
        offset = start + i
        print(f"  {offset:>6}: {bytes_str}")

    if len(region) > 64:
        print(f"  ... ({len(region) - 64} more bytes)")


def check_specific_flags(ef_data: bytes, base: int, flags: list):
    """Check specific flag states."""
    print(f"\n{'='*60}")
    print(f"Flag checks at base {base}")
    print(f"{'='*60}")

    for flag_id, name in flags:
        block_start = (flag_id // 1000) * 1000
        relative = flag_id - block_start
        byte_offset = base + relative // 8
        bit = 7 - (flag_id % 8)

        if byte_offset < len(ef_data):
            byte_val = ef_data[byte_offset]
            is_set = (byte_val >> bit) & 1
            is_ff = byte_val == 0xFF

            status = "SET" if is_set else "unset"
            if is_ff:
                status += " (0xFF region)"

            print(f"  {flag_id}: {name:<40} offset={byte_offset}, bit={bit}, byte=0x{byte_val:02X} -> {status}")
        else:
            print(f"  {flag_id}: {name:<40} OUT OF RANGE")


def find_non_ff_regions(ef_data: bytes, search_range: tuple = (0, 10000)):
    """Find regions that are NOT all 0xFF."""
    print(f"\n{'='*60}")
    print(f"Non-0xFF regions in bytes {search_range[0]}-{search_range[1]}")
    print(f"{'='*60}")

    start, end = search_range
    regions = []
    region_start = None

    for i in range(start, min(end, len(ef_data))):
        if ef_data[i] != 0xFF:
            if region_start is None:
                region_start = i
        else:
            if region_start is not None:
                regions.append((region_start, i - 1))
                region_start = None

    if region_start is not None:
        regions.append((region_start, min(end, len(ef_data)) - 1))

    for r_start, r_end in regions:
        length = r_end - r_start + 1
        sample = ef_data[r_start:r_start+min(8, length)]
        sample_hex = ' '.join(f'{b:02X}' for b in sample)
        print(f"  {r_start:>6} - {r_end:>6} ({length:>4} bytes): {sample_hex}...")

    print(f"\nTotal non-0xFF regions: {len(regions)}")
    return regions


def main():
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    parser = SaveParser()
    parsed = parser.parse(save_path)

    # Analyze slot 0 (progressed)
    ef_data = parsed.slots[0].event_flags
    print(f"Analyzing Slot 0 event flags ({len(ef_data)} bytes)")

    # Candidate base for 520000 block
    base_520 = 1254

    # Analyze the region around base 1254
    analyze_region(ef_data, 1250, 1300, "Around base 1254")

    # Check the specific flags
    flags_to_check = [
        (520000, "Lhutel the Headless"),
        (520010, "Demi-Human Ashes"),
        (520020, "Noble Sorcerer Ashes"),
        (520030, "Assassin's Crimson Dagger"),
        (520040, "Banished Knight Engvall"),
        (520050, "Twinsage Sorcerer Ashes"),
        (520060, "Glintstone Sorcerer Ashes"),
        (520070, "Unknown"),
        (520080, "Ancient Dragon Knight Kristoff"),
        (520090, "Bloodhound Knight Floh"),
        (520100, "Ordovis's Greatsword"),
    ]
    check_specific_flags(ef_data, base_520, flags_to_check)

    # Find all non-0xFF regions in the first 10k bytes
    find_non_ff_regions(ef_data, (0, 10000))

    # Check if there's another allocation for 520xxx elsewhere
    print(f"\n{'='*60}")
    print("Searching for 520080+ flags elsewhere...")
    print(f"{'='*60}")

    # For flag 520080, try different bases
    flag_id = 520080
    block_start = 520000
    relative = flag_id - block_start  # 80
    bit = 7 - (flag_id % 8)  # 7 - 0 = 7

    print(f"\nSearching for flag {flag_id} (relative offset 80//8=10, bit {bit})")

    # Look for byte offsets where bit 7 is set in slot 0 but not in slot 1
    ef_s0 = ef_data
    ef_s1 = parsed.slots[1].event_flags

    candidates = []
    for offset in range(len(ef_s0)):
        byte_s0 = ef_s0[offset]
        byte_s1 = ef_s1[offset]

        bit_s0 = (byte_s0 >> bit) & 1
        bit_s1 = (byte_s1 >> bit) & 1

        # SET in s0, UNSET in s1
        if bit_s0 == 1 and bit_s1 == 0:
            # Calculate what base this would imply
            implied_base = offset - (relative // 8)
            if implied_base >= 0 and byte_s0 != 0xFF:
                candidates.append((offset, implied_base, byte_s0))

    print(f"Found {len(candidates)} candidate locations")
    for offset, implied_base, byte_val in candidates[:20]:
        print(f"  offset={offset}, implied_base={implied_base}, byte=0x{byte_val:02X}")


if __name__ == "__main__":
    main()
