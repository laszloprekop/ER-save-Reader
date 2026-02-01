#!/usr/bin/env python3
"""
Compare the current algorithm's detection vs the correct offset.
Shows if gaItemsEnd-based search finds the wrong offset.
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

ITEM_TYPE_WEAPON = 0x00000000
ITEM_TYPE_ARMOR = 0x10000000
GA_ITEMS_COUNT = 5120


def get_slot_data(filepath: Path, slot_index: int = 0) -> bytes:
    with open(filepath, 'rb') as f:
        data = f.read()
    entry_offset = BND4_HEADER_SIZE + (slot_index * BND4_ENTRY_SIZE) + BND4_ENTRY_OFFSET_POS
    bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
    slot_offset = bnd4_offset + SLOT_CHECKSUM_SIZE
    return data[slot_offset:slot_offset + 0x280000]


def calculate_ga_items_end(slot_data: bytes) -> int:
    """Calculate gaItemsEnd by parsing GaItems section."""
    view = memoryview(slot_data)
    version = struct.unpack_from('<I', slot_data, 0)[0]
    header_padding = 0x8 if version == 81 else 0x18
    pos = 4 + 4 + header_padding  # version + map_id + padding

    for _ in range(GA_ITEMS_COUNT):
        if pos + 8 > len(slot_data):
            break
        item_id = struct.unpack_from('<I', slot_data, pos + 4)[0]
        pos += 8  # base entry

        if item_id != 0 and item_id != 0xffffffff:
            item_type = item_id & 0xf0000000
            if item_type == ITEM_TYPE_WEAPON:
                pos += 13
            elif item_type == ITEM_TYPE_ARMOR:
                pos += 8

    return pos


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


def find_best_offset_from_gaItems(slot_data: bytes, ga_items_end: int, max_search: int = 200000) -> tuple:
    """Current algorithm: search from gaItemsEnd."""
    best_offset = ga_items_end
    best_tier1 = 0
    best_total = 0

    for test_offset in range(ga_items_end, min(ga_items_end + max_search, len(slot_data) - EVENT_FLAGS_SIZE)):
        tier1, total = score_offset(slot_data, test_offset)
        if tier1 > best_tier1 or (tier1 == best_tier1 and total > best_total):
            best_tier1 = tier1
            best_total = total
            best_offset = test_offset
            if total == len(VALIDATION_FLAGS):
                break  # Perfect match

    return best_offset, best_tier1, best_total


def find_best_offset_fixed_range(slot_data: bytes) -> tuple:
    """Better algorithm: search fixed range."""
    best_offset = 0
    best_tier1 = 0
    best_total = 0

    for test_offset in range(0x10000, min(0x30000, len(slot_data) - EVENT_FLAGS_SIZE), 4):
        tier1, total = score_offset(slot_data, test_offset)
        if tier1 > best_tier1 or (tier1 == best_tier1 and total > best_total):
            best_tier1 = tier1
            best_total = total
            best_offset = test_offset
            # Don't break early - search entire range

    return best_offset, best_tier1, best_total


def main():
    snapshots = sorted(SNAPSHOTS_DIR.glob("ER0000.sl2*"),
                       key=lambda p: p.stat().st_mtime)

    print("="*130)
    print("EF DETECTION COMPARISON: gaItemsEnd-based vs Fixed-range")
    print("="*130)
    print(f"{'Snapshot':<48} {'GaItemsEnd':>10} │ {'Current':>10} {'T1':>4} {'Tot':>4} │ {'FixedRange':>10} {'T1':>4} {'Tot':>4} │ {'Match':>6}")
    print("-"*130)

    mismatches = 0
    for snapshot in snapshots:
        try:
            slot_data = get_slot_data(snapshot)
            ga_items_end = calculate_ga_items_end(slot_data)

            # Current algorithm
            curr_off, curr_t1, curr_tot = find_best_offset_from_gaItems(slot_data, ga_items_end)

            # Fixed range algorithm
            fix_off, fix_t1, fix_tot = find_best_offset_fixed_range(slot_data)

            match = "✓" if curr_off == fix_off else "✗ DIFF"
            if curr_off != fix_off:
                mismatches += 1

            name = snapshot.name[:46]
            print(f"{name:<48} 0x{ga_items_end:05X}   │ 0x{curr_off:05X}   {curr_t1:>2}/{4:>1} {curr_tot:>2}/{7:>1} │ 0x{fix_off:05X}   {fix_t1:>2}/{4:>1} {fix_tot:>2}/{7:>1} │ {match}")

        except Exception as e:
            print(f"{snapshot.name[:46]:<48} ERROR: {e}")

    print("="*130)
    print(f"\nMismatches: {mismatches}/{len(snapshots)}")

    if mismatches > 0:
        print("\nConclusion: The gaItemsEnd-based algorithm finds different (potentially wrong) offsets!")
        print("The fixed-range algorithm consistently finds the best offset.")


if __name__ == "__main__":
    main()
