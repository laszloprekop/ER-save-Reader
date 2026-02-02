#!/usr/bin/env python3
"""
Build complete per-section pickup base mapping.

This script:
1. Loads all save files from the save directory
2. For each dungeon section, brute-force discovers the actual pickup base
3. Outputs a Rust HashMap for DUNGEON_PICKUP_SECTION_BASES

The key insight: Dungeon pickups do NOT follow the linear formula.
Each (area, section) combination has its own empirically-determined base.

Usage:
    python scripts/build_pickup_section_map.py
"""

import struct
import json
from pathlib import Path
from collections import defaultdict

# Paths
SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
EXTRACTED_FLAGS = Path("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/public/data/extracted_event_flags.json")

# Constants
EVENT_FLAGS_SIZE = 0x1BF99F

# BND4 parsing
BND4_HEADER_SIZE = 0x40
BND4_ENTRY_SIZE = 0x20
BND4_ENTRY_OFFSET_POS = 0x10

# Validation flags
VALIDATION_FLAGS = [
    (71800, 2725, 7), (71801, 2725, 6),
    (76100, 3262, 3), (76101, 3262, 2),
]


def find_event_flags_offset(slot_data: bytes) -> int:
    best_offset = 0x12B00
    best_score = 0
    for test_offset in range(0x10000, min(0x30000, len(slot_data) - EVENT_FLAGS_SIZE), 4):
        score = sum(1 for _, byte_off, bit_pos in VALIDATION_FLAGS
                    if test_offset + byte_off < len(slot_data) and
                    (slot_data[test_offset + byte_off] & (1 << bit_pos)) != 0)
        if score > best_score:
            best_score = score
            best_offset = test_offset
    return best_offset


def parse_slot(save_data: bytes, slot_idx: int) -> dict:
    entry_offset = BND4_HEADER_SIZE + (slot_idx * BND4_ENTRY_SIZE) + BND4_ENTRY_OFFSET_POS
    if entry_offset + 4 > len(save_data):
        return None
    slot_data_offset = struct.unpack('<I', save_data[entry_offset:entry_offset+4])[0]
    slot_data = save_data[slot_data_offset:slot_data_offset + 0x280000]
    ef_offset = find_event_flags_offset(slot_data)
    return {'event_flags': slot_data[ef_offset:ef_offset + EVENT_FLAGS_SIZE]}


def search_section_base(event_flags: bytes, pickups: list) -> dict:
    if not pickups:
        return None
    best_base = 0
    best_matches = 0
    for test_base in range(0, min(60000, len(event_flags) - 2000)):
        matches = sum(1 for p in pickups
                      if test_base + p['local_id'] // 8 < len(event_flags) and
                      (event_flags[test_base + p['local_id'] // 8] & (1 << (7 - p['local_id'] % 8))) != 0)
        if matches > best_matches:
            best_matches = matches
            best_base = test_base
    return {'base': best_base, 'matches': best_matches, 'total': len(pickups),
            'confidence': best_matches / len(pickups) if pickups else 0}


def main():
    # Load all save files
    save_files = list(SAVE_DIR.glob("*.sl2"))
    if not save_files:
        print(f"No .sl2 files found in {SAVE_DIR}")
        return

    # Load extracted flags
    with open(EXTRACTED_FLAGS) as f:
        extracted = json.load(f)

    # Group pickups by (area, section)
    pickups_by_section = defaultdict(list)
    for flag in extracted.get('flags', []):
        if flag.get('category') != 'Dungeon Pickup':
            continue
        flag_id = flag['flag_id']
        if 10_000_000 <= flag_id < 50_000_000:
            area = flag_id // 1_000_000
            section = (flag_id // 10000) % 100
            local_id = flag_id % 10000
            if local_id >= 7000:
                pickups_by_section[(area, section)].append({
                    'flag_id': flag_id, 'local_id': local_id,
                    'name': flag.get('name', 'Unknown')
                })

    # Collect all event_flags from all slots
    all_event_flags = []
    for save_path in save_files:
        print(f"Loading: {save_path.name}", file=__import__('sys').stderr)
        with open(save_path, 'rb') as f:
            save_data = f.read()
        for slot_idx in range(10):
            slot = parse_slot(save_data, slot_idx)
            if slot:
                all_event_flags.append(slot['event_flags'])

    print(f"Loaded {len(all_event_flags)} slots", file=__import__('sys').stderr)

    # Discover bases for each section
    section_bases = {}
    for (area, section), pickups in sorted(pickups_by_section.items()):
        # Try each slot's event_flags
        best_result = None
        for ef in all_event_flags:
            result = search_section_base(ef, pickups)
            if result and (best_result is None or result['matches'] > best_result['matches']):
                best_result = result

        if best_result and best_result['matches'] > 0:
            section_bases[(area, section)] = best_result
            status = "OK" if best_result['confidence'] >= 0.5 else "LOW"
            print(f"({area}, {section:2d}): base={best_result['base']:5d}, "
                  f"matches={best_result['matches']:2d}/{best_result['total']:2d} [{status}]",
                  file=__import__('sys').stderr)

    # Output Rust code
    print("\n// Auto-generated per-section pickup bases")
    print("// Generated by scripts/build_pickup_section_map.py")
    print("pub static DUNGEON_PICKUP_SECTION_BASES: Lazy<HashMap<(u32, u32), u32>> = Lazy::new(|| {")
    print("    HashMap::from([")

    for (area, section), result in sorted(section_bases.items()):
        if result['confidence'] >= 0.3:  # Only include reasonably confident results
            print(f"        (({area}, {section:2d}), {result['base']:5d}),  // {result['matches']}/{result['total']} matches")

    print("    ])")
    print("});")


if __name__ == '__main__':
    main()
