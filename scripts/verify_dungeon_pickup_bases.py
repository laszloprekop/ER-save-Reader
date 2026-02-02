#!/usr/bin/env python3
"""
Verify discovered dungeon pickup bases by:
1. Cross-checking against early-game save (should have fewer pickups)
2. Checking specific known items the Confessor should have collected
3. Verifying no false positives with early-game character
"""

import struct
import json
from pathlib import Path
from collections import defaultdict

# Paths
SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
EXTRACTED_FLAGS = Path("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/public/data/extracted_event_flags.json")
GRANULAR_DIR = SAVE_DIR / "Granular snapshots for debugging"

# Known dungeon section size
DUNGEON_SECTION_SIZE = 1125
EVENT_FLAGS_SIZE = 0x1BF99F  # 1,833,375 bytes

# BND4 parsing constants
BND4_HEADER_SIZE = 0x40
BND4_ENTRY_SIZE = 0x20
BND4_ENTRY_OFFSET_POS = 0x10
SLOT_CHECKSUM_SIZE = 16
SLOT_SIZE = 0x280000
FIXED_HEADER_SIZE = 0x20

# Validation flags for event flags offset detection
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Discovered pickup bases
DISCOVERED_PICKUP_BASES = {
    30: 17731,  # Catacombs
    31: 8346,   # Caves
    32: 29658,  # Tunnels
}

# General event bases for comparison
GENERAL_EVENT_BASES = {
    30: 27411,  # Catacombs
    31: 28634,  # Caves
    32: 31577,  # Tunnels
}


def load_pickup_flags() -> dict:
    """Load dungeon pickup flags (localId >= 7000) for areas 30, 31, 32."""
    with open(EXTRACTED_FLAGS, 'r') as f:
        data = json.load(f)

    pickup_flags = {30: [], 31: [], 32: []}

    for flag in data['flags']:
        if flag.get('category') != 'Dungeon Pickup':
            continue

        flag_id = flag['flag_id']
        if flag_id >= 30_000_000 and flag_id < 33_000_000:
            area = flag_id // 1_000_000
            local_id = flag_id % 10000
            section = (flag_id // 10000) % 100

            if local_id >= 7000 and area in pickup_flags:
                pickup_flags[area].append({
                    'flag_id': flag_id,
                    'name': flag['name'],
                    'area': area,
                    'section': section,
                    'local_id': local_id,
                })

    return pickup_flags


def find_event_flags_offset(slot_data: bytes) -> int:
    """Find event flags offset by searching for validation pattern."""
    best_offset = 0x12B00
    best_score = 0

    for test_offset in range(0x10000, min(0x30000, len(slot_data) - EVENT_FLAGS_SIZE), 4):
        score = 0
        for flag_id, byte_off, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_off
            if abs_pos < len(slot_data):
                if (slot_data[abs_pos] & (1 << bit_pos)) != 0:
                    score += 1

        if score > best_score:
            best_score = score
            best_offset = test_offset

    return best_offset


def parse_slot(save_data: bytes, slot_idx: int) -> dict:
    """Parse a single slot from save data."""
    entry_offset = BND4_HEADER_SIZE + (slot_idx * BND4_ENTRY_SIZE) + BND4_ENTRY_OFFSET_POS
    if entry_offset + 4 > len(save_data):
        return None

    bnd4_offset = struct.unpack_from('<I', save_data, entry_offset)[0]
    slot_offset = bnd4_offset + SLOT_CHECKSUM_SIZE

    if slot_offset + SLOT_SIZE > len(save_data):
        slot_data = save_data[slot_offset:]
    else:
        slot_data = save_data[slot_offset:slot_offset + SLOT_SIZE]

    if len(slot_data) < FIXED_HEADER_SIZE:
        return None

    version = struct.unpack_from('<I', slot_data, 0)[0]
    if version == 0:
        return None

    ef_offset = find_event_flags_offset(slot_data)
    event_flags = slot_data[ef_offset:ef_offset + EVENT_FLAGS_SIZE]

    return {
        'slot_idx': slot_idx,
        'event_flags': event_flags,
        'ef_offset': ef_offset,
    }


def check_flag_at_base(event_flags: bytes, flag_id: int, base: int, section_size: int) -> bool:
    """Check if a dungeon pickup flag is set using a specific base."""
    section = (flag_id // 10000) % 100
    local_id = flag_id % 10000

    byte_offset = base + section * section_size + local_id // 8
    bit_position = 7 - (local_id % 8)

    if byte_offset < 0 or byte_offset >= len(event_flags):
        return False

    return (event_flags[byte_offset] & (1 << bit_position)) != 0


def verify_bases(slot0_ef: bytes, slot1_ef: bytes, pickup_flags: dict):
    """
    Verify discovered bases by comparing mid-game (slot 0) vs early-game (slot 1).

    Criteria for validation:
    1. Mid-game should have MORE pickups set than early-game
    2. Early-game should have very few or zero pickups (depending on progression)
    3. The items that ARE set should make sense for the character's progression
    """
    print("\n" + "=" * 70)
    print("TEMPORAL VERIFICATION: Slot 0 (mid-game) vs Slot 1 (early-game)")
    print("=" * 70)

    for area in [30, 31, 32]:
        area_name = {30: "Catacombs", 31: "Caves", 32: "Tunnels"}[area]
        flags = pickup_flags[area]
        discovered_base = DISCOVERED_PICKUP_BASES[area]
        general_base = GENERAL_EVENT_BASES[area]

        print(f"\n  Area {area} ({area_name}):")
        print(f"    Testing base: {discovered_base}")

        # Count set flags in each slot
        slot0_set = []
        slot1_set = []
        for f in flags:
            if check_flag_at_base(slot0_ef, f['flag_id'], discovered_base, DUNGEON_SECTION_SIZE):
                slot0_set.append(f)
            if check_flag_at_base(slot1_ef, f['flag_id'], discovered_base, DUNGEON_SECTION_SIZE):
                slot1_set.append(f)

        print(f"    Slot 0 (mid-game): {len(slot0_set)}/{len(flags)} flags set")
        print(f"    Slot 1 (early-game): {len(slot1_set)}/{len(flags)} flags set")

        # Show differences
        slot0_only = [f for f in slot0_set if f not in slot1_set]
        slot1_only = [f for f in slot1_set if f not in slot0_set]

        if slot0_only:
            print(f"\n    Items ONLY in Slot 0 (expected - mid-game progression):")
            for f in slot0_only[:5]:
                print(f"      {f['flag_id']}: {f['name']}")
            if len(slot0_only) > 5:
                print(f"      ... and {len(slot0_only) - 5} more")

        if slot1_only:
            print(f"\n    Items ONLY in Slot 1 (unexpected - should verify):")
            for f in slot1_only[:5]:
                print(f"      {f['flag_id']}: {f['name']}")

        # Validation verdict
        if len(slot0_set) > len(slot1_set):
            print(f"\n    ✓ VALID: Mid-game has more pickups than early-game")
        elif len(slot0_set) == len(slot1_set) == 0:
            print(f"\n    ⚠ INCONCLUSIVE: No pickups in either slot")
        else:
            print(f"\n    ✗ SUSPICIOUS: Early-game has same or more pickups")

        # Also verify using general event base gives fewer matches
        slot0_general = sum(1 for f in flags
                          if check_flag_at_base(slot0_ef, f['flag_id'], general_base, DUNGEON_SECTION_SIZE))
        print(f"\n    Using GENERAL event base ({general_base}): {slot0_general} flags")
        print(f"    Using PICKUP base ({discovered_base}): {len(slot0_set)} flags")

        if len(slot0_set) > slot0_general:
            print(f"    ✓ Pickup base finds more correct matches")
        else:
            print(f"    ⚠ General base finds same or more matches")


def main():
    print("=" * 70)
    print("VERIFYING DUNGEON PICKUP BASES")
    print("=" * 70)
    print(f"\nDiscovered bases to verify:")
    for area, base in DISCOVERED_PICKUP_BASES.items():
        area_name = {30: "Catacombs", 31: "Caves", 32: "Tunnels"}[area]
        print(f"  Area {area} ({area_name}): {base}")

    # Load pickup flags
    pickup_flags = load_pickup_flags()

    # Parse save file
    save_path = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"
    if not save_path.exists():
        save_path = SAVE_DIR / "ER0000-backup-2026-01-01.sl2"

    if not save_path.exists():
        print(f"No save file found")
        return

    print(f"\nLoading save: {save_path.name}")
    with open(save_path, 'rb') as f:
        save_data = f.read()

    # Parse both slots
    slot0 = parse_slot(save_data, 0)
    slot1 = parse_slot(save_data, 1)

    if not slot0:
        print("Could not parse slot 0")
        return
    if not slot1:
        print("Could not parse slot 1")
        return

    print(f"\nSlot 0 EF offset: 0x{slot0['ef_offset']:X}")
    print(f"Slot 1 EF offset: 0x{slot1['ef_offset']:X}")

    # Verify bases
    verify_bases(slot0['event_flags'], slot1['event_flags'], pickup_flags)

    # Final summary
    print("\n" + "=" * 70)
    print("FINAL VERIFICATION SUMMARY")
    print("=" * 70)

    for area, base in DISCOVERED_PICKUP_BASES.items():
        area_name = {30: "Catacombs", 31: "Caves", 32: "Tunnels"}[area]
        flags = pickup_flags[area]

        slot0_count = sum(1 for f in flags
                        if check_flag_at_base(slot0['event_flags'], f['flag_id'], base, DUNGEON_SECTION_SIZE))
        slot1_count = sum(1 for f in flags
                        if check_flag_at_base(slot1['event_flags'], f['flag_id'], base, DUNGEON_SECTION_SIZE))

        status = "✓ VERIFIED" if slot0_count > slot1_count else "⚠ NEEDS REVIEW"
        print(f"\n  {area_name} (area {area}): base = {base}")
        print(f"    Slot 0: {slot0_count} pickups, Slot 1: {slot1_count} pickups")
        print(f"    Status: {status}")


if __name__ == "__main__":
    main()
