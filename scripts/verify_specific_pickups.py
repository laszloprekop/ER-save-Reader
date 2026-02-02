#!/usr/bin/env python3
"""
Verify specific dungeon pickups against actual save data.

This script helps debug false negatives by:
1. Loading a save file with known collected items
2. Checking if those items show as collected using our formula
3. Identifying which sections/bases might be wrong

Usage:
    python scripts/verify_specific_pickups.py [slot_index]
"""

import struct
import json
from pathlib import Path
from collections import defaultdict

# Paths
SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
EXTRACTED_FLAGS = Path("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/public/data/extracted_event_flags.json")

# Constants
DUNGEON_SECTION_SIZE = 1125
EVENT_FLAGS_SIZE = 0x1BF99F

# BND4 parsing
BND4_HEADER_SIZE = 0x40
BND4_ENTRY_SIZE = 0x20
BND4_ENTRY_OFFSET_POS = 0x10

# Per-section pickup bases (empirically discovered 2026-02-02)
# The linear formula was WRONG - each (area, section) has its own base
DUNGEON_PICKUP_SECTION_BASES = {
    (10,  0): 31904, (10,  1):  1787,
    (11,  0): 31903, (11,  5):  1835, (11, 10):  1812,
    (12,  1): 31900, (12,  2): 31903, (12,  3): 31902, (12,  5): 31902, (12,  7): 31903,
    (13,  0): 31903,
    (14,  0): 31903,
    (15,  0): 31903,
    (16,  0): 31903,
    (18,  0):  3847,
    (20,  0): 31903, (20,  1): 31903,
    (21,  0): 31903, (21,  1): 31903, (21,  2): 31903,
    (22,  0): 28962,
    (28,  0): 28974,
    (30,  0):  1790, (30,  1):  1786, (30,  2):  1787, (30,  3):  1835, (30,  4):  1787,
    (30,  5):  1835, (30,  6):  3827, (30,  7):  1812, (30,  8):  1834, (30,  9):  3764,
    (30, 10):  3826, (30, 11):  1787, (30, 12):  1787, (30, 13):  1785, (30, 14):  1835,
    (30, 15):  1787, (30, 16):  1835, (30, 17):  1835, (30, 18):  1787, (30, 19):  1835,
    (30, 20):  3723,
    (31,  0):  1787, (31,  1):  1835, (31,  2):  1797, (31,  3):  1787, (31,  4):  1835,
    (31,  5):  3828, (31,  6):  1787, (31,  7):  3764, (31,  9):  1835, (31, 10):  1790,
    (31, 11): 28975, (31, 12): 28974, (31, 15):  1786, (31, 17):  3719, (31, 18):  3718,
    (31, 19): 28974, (31, 20):  1787, (31, 21): 31903, (31, 22):  3827,
    (32,  0):  3847, (32,  1):  1835, (32,  2):  3847, (32,  4):  1835, (32,  5):  3723,
    (32,  7):  1788, (32,  8): 28979, (32, 11):  3725,
    (34, 10):  1787, (34, 11): 31902, (34, 12): 28974, (34, 13):  1787, (34, 14):  1789,
    (35,  0): 31903,
    (39, 20): 28974,
}

# Legacy area-based bases (deprecated, kept for comparison)
DUNGEON_PICKUP_BASES = {
    10: 31904, 11: 31903, 12: 31900, 13: 31903, 14: 31903, 15: 31903,
    16: 31903, 18: 3847, 20: 31903, 21: 31903, 22: 28962, 28: 28974, 35: 31903,
}

# Validation flags for event_flags offset detection
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]


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

    slot_data_offset = struct.unpack('<I', save_data[entry_offset:entry_offset+4])[0]
    slot_data = save_data[slot_data_offset:slot_data_offset + 0x280000]

    ef_offset = find_event_flags_offset(slot_data)
    event_flags = slot_data[ef_offset:ef_offset + EVENT_FLAGS_SIZE]

    return {
        'slot_data': slot_data,
        'event_flags': event_flags,
        'ef_offset': ef_offset,
    }


def check_pickup_flag(event_flags: bytes, area: int, section: int, local_id: int) -> tuple[bool, int, int]:
    """Check if a pickup flag is set using per-section lookup.

    Returns: (is_set, byte_offset, bit_position)
    """
    section_base = DUNGEON_PICKUP_SECTION_BASES.get((area, section))
    if section_base is None:
        return (False, 0, 0)

    # Formula: offset = section_base + local_id/8
    byte_offset = section_base + local_id // 8
    bit_pos = 7 - (local_id % 8)

    if byte_offset >= len(event_flags):
        return (False, byte_offset, bit_pos)

    is_set = (event_flags[byte_offset] & (1 << bit_pos)) != 0
    return (is_set, byte_offset, bit_pos)


def search_for_flag_in_ef(event_flags: bytes, local_id: int, search_range: tuple[int, int] = (0, 50000)) -> list[int]:
    """Brute-force search for where a local_id might be set in event_flags.

    Returns list of byte offsets where the bit is set.
    """
    bit_pos = 7 - (local_id % 8)
    found = []

    # We don't know the base or section, so search the likely range
    for byte_offset in range(search_range[0], min(search_range[1], len(event_flags))):
        if (event_flags[byte_offset] & (1 << bit_pos)) != 0:
            found.append(byte_offset)

    return found


def analyze_area(event_flags: bytes, area: int, extracted_flags: dict) -> dict:
    """Analyze all pickups for an area, checking which are detected vs actual."""
    results = {
        'area': area,
        'total': 0,
        'detected_set': 0,
        'detected_unset': 0,
        'by_section': defaultdict(lambda: {'total': 0, 'set': 0}),
        'samples': [],
    }

    for flag in extracted_flags.get('flags', []):
        if flag.get('category') != 'Dungeon Pickup':
            continue

        flag_id = flag['flag_id']
        flag_area = flag_id // 1_000_000
        if flag_area != area:
            continue

        section = (flag_id // 10000) % 100
        local_id = flag_id % 10000

        if local_id < 7000:
            continue

        results['total'] += 1
        results['by_section'][section]['total'] += 1

        is_set, byte_offset, bit_pos = check_pickup_flag(event_flags, area, section, local_id)

        if is_set:
            results['detected_set'] += 1
            results['by_section'][section]['set'] += 1
        else:
            results['detected_unset'] += 1

        # Sample first few items per section
        if len([s for s in results['samples'] if s['section'] == section]) < 3:
            results['samples'].append({
                'flag_id': flag_id,
                'name': flag.get('name', 'Unknown'),
                'section': section,
                'local_id': local_id,
                'is_set': is_set,
                'byte_offset': byte_offset,
                'bit_pos': bit_pos,
            })

    return results


def main():
    import sys

    slot_idx = int(sys.argv[1]) if len(sys.argv) > 1 else 0

    # Find save file
    save_files = list(SAVE_DIR.glob("*.sl2"))
    if not save_files:
        print(f"No .sl2 files found in {SAVE_DIR}")
        return

    save_path = save_files[0]
    print(f"Loading save: {save_path.name}")
    print(f"Analyzing slot {slot_idx}")

    with open(save_path, 'rb') as f:
        save_data = f.read()

    slot = parse_slot(save_data, slot_idx)
    if not slot:
        print(f"Failed to parse slot {slot_idx}")
        return

    print(f"Event flags offset: 0x{slot['ef_offset']:X}")

    # Load extracted flags
    with open(EXTRACTED_FLAGS, 'r') as f:
        extracted = json.load(f)

    # Analyze problem areas (catacombs, caves, tunnels)
    problem_areas = [30, 31, 32]  # Add more as needed

    for area in problem_areas:
        print(f"\n{'='*60}")
        area_names = {30: 'Catacombs', 31: 'Caves', 32: 'Tunnels'}
        print(f"Area {area} ({area_names.get(area, 'Unknown')})")
        # Count how many sections have known bases
        known_sections = sum(1 for (a, s) in DUNGEON_PICKUP_SECTION_BASES if a == area)
        print(f"Known section bases: {known_sections}")
        print('='*60)

        results = analyze_area(slot['event_flags'], area, extracted)

        print(f"Total pickups: {results['total']}")
        print(f"Detected as SET: {results['detected_set']}")
        print(f"Detected as UNSET: {results['detected_unset']}")

        print(f"\nBy section:")
        for section in sorted(results['by_section'].keys()):
            sec_data = results['by_section'][section]
            pct = sec_data['set'] / sec_data['total'] * 100 if sec_data['total'] > 0 else 0
            status = "OK" if pct > 50 else "PROBLEM?" if pct > 0 else "ALL UNSET"
            print(f"  Section {section:02d}: {sec_data['set']:3d}/{sec_data['total']:3d} set ({pct:5.1f}%) {status}")

        print(f"\nSample items:")
        for sample in results['samples'][:10]:
            status = "SET" if sample['is_set'] else "UNSET"
            print(f"  [{status:5s}] {sample['name'][:30]:30s} (sec={sample['section']:02d}, local={sample['local_id']}, off=0x{sample['byte_offset']:X}, bit={sample['bit_pos']})")


if __name__ == '__main__':
    main()
