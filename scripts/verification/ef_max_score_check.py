#!/usr/bin/env python3
"""
Check the MAXIMUM achievable validation scores across all candidates for each snapshot.
This helps determine if EF detection is possible or if the anchors are inadequate.
"""

import struct
from pathlib import Path

SNAPSHOTS_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/slot 0 Confessor")

BND4_HEADER_SIZE = 0x40
BND4_ENTRY_SIZE = 0x20
BND4_ENTRY_OFFSET_POS = 0x10
SLOT_CHECKSUM_SIZE = 16
EVENT_FLAGS_SIZE = 0x1BF99F

# Validation anchors
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


def find_max_score(slot_data: bytes) -> dict:
    """Find the maximum achievable tier1 and total scores across all candidates."""
    max_tier1 = 0
    max_total = 0
    best_offset = 0
    tier1_count = sum(1 for f in VALIDATION_FLAGS if f[4] == 1)
    candidates_with_perfect_tier1 = 0
    candidates_total = 0

    for test_offset in range(0x10000, min(0x30000, len(slot_data) - EVENT_FLAGS_SIZE), 4):
        tier1_score = 0
        total_score = 0

        for flag_id, byte_off, bit_pos, name, tier in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_off
            if abs_pos < len(slot_data):
                if (slot_data[abs_pos] & (1 << bit_pos)) != 0:
                    total_score += 1
                    if tier == 1:
                        tier1_score += 1

        if tier1_score > 0 or total_score > 0:
            candidates_total += 1

        if tier1_score == tier1_count:
            candidates_with_perfect_tier1 += 1

        # Track best
        if tier1_score > max_tier1 or (tier1_score == max_tier1 and total_score > max_total):
            max_tier1 = tier1_score
            max_total = total_score
            best_offset = test_offset

    return {
        "max_tier1": max_tier1,
        "max_total": max_total,
        "best_offset": best_offset,
        "tier1_max": tier1_count,
        "total_max": len(VALIDATION_FLAGS),
        "candidates_total": candidates_total,
        "candidates_with_perfect_tier1": candidates_with_perfect_tier1,
    }


def main():
    snapshots = sorted(SNAPSHOTS_DIR.glob("ER0000.sl2*"),
                       key=lambda p: p.stat().st_mtime)

    print("="*100)
    print("MAXIMUM ACHIEVABLE VALIDATION SCORES PER SNAPSHOT")
    print("="*100)
    print(f"{'Snapshot':<60} {'MaxT1':>6} {'MaxTot':>7} {'BestOff':>10} {'Perfect T1':>12}")
    print("-"*100)

    for snapshot in snapshots:
        try:
            slot_data = get_slot_data(snapshot)
            result = find_max_score(slot_data)

            tier1_status = "✓" if result["max_tier1"] == result["tier1_max"] else "✗"
            name = snapshot.name[:58]

            print(f"{name:<60} {result['max_tier1']:>3}/{result['tier1_max']:>1} {tier1_status} "
                  f"{result['max_total']:>3}/{result['total_max']:>1}  "
                  f"0x{result['best_offset']:05X}  "
                  f"{result['candidates_with_perfect_tier1']:>10}")
        except Exception as e:
            print(f"{snapshot.name[:58]:<60} ERROR: {e}")

    print("="*100)


if __name__ == "__main__":
    main()
