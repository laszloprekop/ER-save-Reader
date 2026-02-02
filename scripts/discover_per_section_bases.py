#!/usr/bin/env python3
"""
Discover per-section bases for dungeon pickups.

The current formula assumes: offset = area_base + section * 1125 + local_id/8
But verification shows this only works for SOME sections.

This script:
1. For each section with known collected items, brute-force search for the actual base
2. Check if there's a pattern (e.g., non-contiguous section allocation)
3. Output corrected per-section bases

Usage:
    python scripts/discover_per_section_bases.py [slot_index]
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


def search_section_base(event_flags: bytes, pickups: list, search_range: tuple[int, int] = (0, 60000)) -> dict:
    """
    Brute-force search for the base offset that maximizes matches for a section's pickups.

    Args:
        event_flags: The event flags byte array
        pickups: List of pickup dicts with 'local_id' field
        search_range: (min_base, max_base) to search

    Returns:
        {'base': int, 'matches': int, 'total': int, 'confidence': float}
    """
    if not pickups:
        return None

    best_base = 0
    best_matches = 0

    for test_base in range(search_range[0], min(search_range[1], len(event_flags) - 2000)):
        matches = 0
        for pickup in pickups:
            local_id = pickup['local_id']
            byte_offset = test_base + local_id // 8
            bit_pos = 7 - (local_id % 8)

            if byte_offset < len(event_flags):
                if (event_flags[byte_offset] & (1 << bit_pos)) != 0:
                    matches += 1

        if matches > best_matches:
            best_matches = matches
            best_base = test_base

    total = len(pickups)
    return {
        'base': best_base,
        'matches': best_matches,
        'total': total,
        'confidence': best_matches / total if total > 0 else 0,
    }


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

    # Group pickups by area and section
    pickups_by_area_section = defaultdict(lambda: defaultdict(list))

    for flag in extracted.get('flags', []):
        if flag.get('category') != 'Dungeon Pickup':
            continue

        flag_id = flag['flag_id']
        if not (10_000_000 <= flag_id < 50_000_000):
            continue

        area = flag_id // 1_000_000
        section = (flag_id // 10000) % 100
        local_id = flag_id % 10000

        if local_id >= 7000:
            pickups_by_area_section[area][section].append({
                'flag_id': flag_id,
                'name': flag.get('name', 'Unknown'),
                'local_id': local_id,
            })

    # Analyze problem areas
    problem_areas = [30, 31, 32]
    area_names = {30: 'Catacombs', 31: 'Caves', 32: 'Tunnels'}

    for area in problem_areas:
        print(f"\n{'='*70}")
        print(f"Area {area} ({area_names.get(area, 'Unknown')})")
        print('='*70)

        sections = pickups_by_area_section[area]
        section_bases = {}

        for section in sorted(sections.keys()):
            pickups = sections[section]
            result = search_section_base(slot['event_flags'], pickups)

            if result and result['matches'] > 0:
                section_bases[section] = result

                # Calculate what the linear formula would predict
                # If we knew the area base, linear would be: area_base + section * 1125
                status = "FOUND" if result['confidence'] >= 0.5 else "PARTIAL"
                print(f"Section {section:02d}: base=0x{result['base']:05X} ({result['base']:6d}), "
                      f"matches={result['matches']:2d}/{result['total']:2d} ({result['confidence']*100:5.1f}%) [{status}]")
            else:
                print(f"Section {section:02d}: NO MATCHES (items not collected?)")

        # Analyze pattern
        if len(section_bases) >= 2:
            print(f"\nPattern analysis:")
            sorted_sections = sorted(section_bases.keys())

            # Check if bases follow linear pattern
            deltas = []
            for i in range(1, len(sorted_sections)):
                prev_sec = sorted_sections[i-1]
                curr_sec = sorted_sections[i]
                prev_base = section_bases[prev_sec]['base']
                curr_base = section_bases[curr_sec]['base']

                sec_diff = curr_sec - prev_sec
                base_diff = curr_base - prev_base
                expected_diff = sec_diff * DUNGEON_SECTION_SIZE

                deltas.append((prev_sec, curr_sec, base_diff, expected_diff))

                if base_diff == expected_diff:
                    print(f"  Section {prev_sec:02d} -> {curr_sec:02d}: delta={base_diff:+6d} (expected {expected_diff:+6d}) ✓")
                else:
                    print(f"  Section {prev_sec:02d} -> {curr_sec:02d}: delta={base_diff:+6d} (expected {expected_diff:+6d}) ✗ OFF BY {base_diff - expected_diff:+d}")

            # Try to compute area_base from found section bases
            print(f"\nInferred area bases:")
            for section, result in sorted(section_bases.items()):
                inferred_area_base = result['base'] - section * DUNGEON_SECTION_SIZE
                print(f"  From section {section:02d}: area_base = {result['base']} - {section}*{DUNGEON_SECTION_SIZE} = {inferred_area_base}")


if __name__ == '__main__':
    main()
