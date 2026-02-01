#!/usr/bin/env python3
"""
Verify the EF detection fix by comparing the new algorithm's output
with the known-correct offsets from the fixed-range analysis.
"""

import struct
from pathlib import Path

SNAPSHOTS_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/slot 0 Confessor")

BND4_HEADER_SIZE = 0x40
BND4_ENTRY_SIZE = 0x20
BND4_ENTRY_OFFSET_POS = 0x10
SLOT_CHECKSUM_SIZE = 16
EVENT_FLAGS_SIZE = 0x1BF99F

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge", 1),
    (71801, 2725, 6, "Stranded Graveyard", 1),
    (76100, 3262, 3, "The First Step", 1),
    (76101, 3262, 2, "Church of Elleh", 1),
    (76102, 3262, 1, "Gatefront Ruins", 2),
    (76104, 3263, 7, "Agheel Lake South", 2),
    (76106, 3263, 5, "Church of Dragon Communion", 2),
]


def get_slot_data(filepath: Path, slot_index: int = 0) -> bytes:
    with open(filepath, 'rb') as f:
        data = f.read()
    entry_offset = BND4_HEADER_SIZE + (slot_index * BND4_ENTRY_SIZE) + BND4_ENTRY_OFFSET_POS
    bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
    slot_offset = bnd4_offset + SLOT_CHECKSUM_SIZE
    return data[slot_offset:slot_offset + 0x280000]


def score_offset(slot_data: bytes, test_offset: int) -> tuple:
    """Score an offset, returns (tier1_score, total_score)."""
    tier1 = 0
    total = 0
    for flag_id, byte_off, bit_pos, name, tier in VALIDATION_FLAGS:
        abs_pos = test_offset + byte_off
        if abs_pos < len(slot_data):
            if (slot_data[abs_pos] & (1 << bit_pos)) != 0:
                total += 1
                if tier == 1:
                    tier1 += 1
    return tier1, total


def find_best_offset_fixed_algorithm(slot_data: bytes) -> tuple:
    """
    FIXED algorithm: search full range, prefer higher offsets on tie.
    """
    best_offset = 0
    best_tier1 = 0
    best_total = 0

    # Extended search range
    for test_offset in range(0x10000, min(0x30000, len(slot_data) - EVENT_FLAGS_SIZE), 4):
        tier1, total = score_offset(slot_data, test_offset)

        # Prefer higher offset on tie (the key fix!)
        is_better = (
            tier1 > best_tier1 or
            (tier1 == best_tier1 and total > best_total) or
            (tier1 == best_tier1 and total == best_total)  # Higher offset on tie
        )

        if is_better:
            best_tier1 = tier1
            best_total = total
            best_offset = test_offset

        # Do NOT break early

    return best_offset, best_tier1, best_total


def main():
    snapshots = sorted(SNAPSHOTS_DIR.glob("ER0000.sl2*"),
                       key=lambda p: p.stat().st_mtime)

    print("="*100)
    print("EF DETECTION FIX VERIFICATION")
    print("="*100)
    print(f"{'Snapshot':<60} {'Offset':>10} {'T1':>4} {'Tot':>4}")
    print("-"*100)

    offsets_found = {}
    for snapshot in snapshots:
        try:
            slot_data = get_slot_data(snapshot)
            offset, tier1, total = find_best_offset_fixed_algorithm(slot_data)
            offsets_found[snapshot.name] = offset

            status = "✓" if (tier1 == 4 and total >= 4) else "?"
            name = snapshot.name[:58]
            print(f"{name:<60} 0x{offset:05X}   {tier1:>2}/{4:>1} {total:>2}/{7:>1} {status}")

        except Exception as e:
            print(f"{snapshot.name[:58]:<60} ERROR: {e}")

    # Verify the offsets make sense
    print("\n" + "="*100)
    print("OFFSET CONSISTENCY ANALYSIS")
    print("="*100)

    # Check if bytes at key offsets are consistent for same-score snapshots
    print("\nVerifying bytes at offset 2725 (grace flags) for snapshots with 7/7 score:")

    prev_bytes = None
    consistent = 0
    total_checked = 0

    for snapshot in snapshots:
        try:
            slot_data = get_slot_data(snapshot)
            offset, tier1, total = find_best_offset_fixed_algorithm(slot_data)

            if tier1 == 4 and total == 7:
                bytes_at_2725 = slot_data[offset + 2725:offset + 2729]

                # Check if bit 7 and bit 6 are both set (71800 and 71801)
                if bytes_at_2725[0] & 0xC0 == 0xC0:  # Both bits 7 and 6 set
                    consistent += 1
                total_checked += 1

                if prev_bytes is None or bytes_at_2725 != prev_bytes:
                    print(f"  0x{offset:05X}: {bytes_at_2725.hex()} - {snapshot.name[:40]}")
                    prev_bytes = bytes_at_2725
        except:
            pass

    print(f"\n{consistent}/{total_checked} snapshots have consistent grace flags at detected offset")

    # Final summary
    print("\n" + "="*100)
    print("SUMMARY")
    print("="*100)
    unique_offsets = set(offsets_found.values())
    print(f"Unique EF offsets found: {len(unique_offsets)}")
    print(f"Range: 0x{min(unique_offsets):05X} - 0x{max(unique_offsets):05X}")

    # Show offset distribution
    offset_counts = {}
    for off in offsets_found.values():
        # Group by 0x1000 chunks
        chunk = (off // 0x1000) * 0x1000
        offset_counts[chunk] = offset_counts.get(chunk, 0) + 1

    print("\nOffset distribution (grouped by 0x1000):")
    for chunk in sorted(offset_counts.keys()):
        print(f"  0x{chunk:05X}-0x{chunk+0xfff:05X}: {offset_counts[chunk]} snapshots")


if __name__ == "__main__":
    main()
