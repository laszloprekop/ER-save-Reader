#!/usr/bin/env python3
"""
Discover dungeon pickup bases for areas 30 (Catacombs), 31 (Caves), 32 (Tunnels).

Item pickup flags (local_id >= 7000) use DIFFERENT base offsets than general dungeon events.
For comparison:
- Area 10 (Stormveil) general events: base 4112, pickups: base 6459 (+2347)
- Area 11 (Leyndell) general events: base 8612, pickups: base 33725 (+25113)

Discovery strategy:
1. Use temporal verification: compare saves with different dungeon item collections
2. Find pickup flags from extracted_event_flags.json for areas 30, 31, 32
3. Search for base offset that makes collected items show as SET
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

# Known general event bases (NOT pickup bases)
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
    # Read slot offset from BND4 entry
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

    # Find event flags offset
    ef_offset = find_event_flags_offset(slot_data)
    event_flags = slot_data[ef_offset:ef_offset + EVENT_FLAGS_SIZE]

    return {
        'slot_idx': slot_idx,
        'event_flags': event_flags,
        'ef_offset': ef_offset,
    }


def check_flag_at_base(event_flags: bytes, flag_id: int, base: int, section_size: int) -> bool:
    """Check if a dungeon pickup flag is set using a specific base."""
    area = flag_id // 1_000_000
    section = (flag_id // 10000) % 100
    local_id = flag_id % 10000

    byte_offset = base + section * section_size + local_id // 8
    bit_position = 7 - (local_id % 8)

    if byte_offset < 0 or byte_offset >= len(event_flags):
        return False

    return (event_flags[byte_offset] & (1 << bit_position)) != 0


def probe_base_candidates(event_flags: bytes, pickup_flags: dict, area: int):
    """
    Probe different base candidates for a given area to find patterns.
    """
    flags = pickup_flags[area]
    if not flags:
        print(f"  No pickup flags found for area {area}")
        return None

    print(f"\n  Area {area}: {len(flags)} pickup flags")
    print(f"  Sample flags: {[f['flag_id'] for f in flags[:5]]}")

    # Group by section
    by_section = defaultdict(list)
    for f in flags:
        by_section[f['section']].append(f)

    print(f"  Sections with pickups: {sorted(by_section.keys())}")

    # Check what's currently set using general event base (expecting WRONG results)
    general_base = GENERAL_EVENT_BASES[area]
    general_set_count = sum(1 for f in flags
                           if check_flag_at_base(event_flags, f['flag_id'], general_base, DUNGEON_SECTION_SIZE))
    print(f"  Using general event base {general_base}: {general_set_count} flags appear set (likely wrong)")

    print(f"\n  Searching for pickup base...")
    best_base = None
    best_count = 0
    best_flags = []

    # Search a wide range
    for base in range(0, 70000):
        set_flags = []
        for f in flags:
            if check_flag_at_base(event_flags, f['flag_id'], base, DUNGEON_SECTION_SIZE):
                set_flags.append(f)

        if len(set_flags) > best_count:
            best_count = len(set_flags)
            best_base = base
            best_flags = set_flags

    print(f"  Best base found: {best_base} with {best_count} flags set")
    if best_flags:
        print(f"  Sample matched flags:")
        for f in best_flags[:5]:
            print(f"    {f['flag_id']}: {f['name']} (section {f['section']}, local {f['local_id']})")

    return best_base, best_count, best_flags


def main():
    print("=" * 70)
    print("DUNGEON PICKUP BASE DISCOVERY")
    print("=" * 70)

    # Load pickup flags
    pickup_flags = load_pickup_flags()
    print(f"\nLoaded pickup flags:")
    for area, flags in pickup_flags.items():
        print(f"  Area {area}: {len(flags)} flags")

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

    # Try slot 0 (Confessor - mid game)
    print("\n" + "=" * 70)
    print("Analyzing Slot 0 (Confessor - mid game)")
    print("=" * 70)

    slot_data = parse_slot(save_data, 0)
    if not slot_data:
        print("Could not parse slot 0")
        return

    print(f"EF offset: 0x{slot_data['ef_offset']:X}")

    event_flags = slot_data['event_flags']

    # Probe each area
    results = {}
    for area in [30, 31, 32]:
        result = probe_base_candidates(event_flags, pickup_flags, area)
        if result:
            results[area] = result

    # Summary
    print("\n" + "=" * 70)
    print("DISCOVERY SUMMARY")
    print("=" * 70)

    for area, (base, count, flags) in results.items():
        area_name = {30: "Catacombs", 31: "Caves", 32: "Tunnels"}[area]
        general_base = GENERAL_EVENT_BASES[area]
        delta = base - general_base if base else 0

        print(f"\nArea {area} ({area_name}):")
        print(f"  General event base: {general_base}")
        print(f"  PICKUP base: {base} (delta: {delta:+d})")
        print(f"  Flags matched: {count}")


if __name__ == "__main__":
    main()
