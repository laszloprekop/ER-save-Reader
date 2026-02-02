#!/usr/bin/env python3
"""
Discover dungeon pickup bases for ALL dungeon areas.

Item pickup flags (local_id >= 7000) use DIFFERENT base offsets than general dungeon events.
For verified areas:
- Area 10 (Stormveil) general events: base 4112, pickups: base 6459 (+2347)
- Area 11 (Leyndell) general events: base 8612, pickups: base 33725 (+25113)
- Area 30 (Catacombs) general events: base 27411, pickups: base 17731 (-9680)
- Area 31 (Caves) general events: base 28634, pickups: base 8346 (-20288)
- Area 32 (Tunnels) general events: base 31577, pickups: base 29658 (-1919)

Discovery strategy:
1. Use temporal verification: compare saves with different dungeon item collections
   - Slot 0 (mid-game Confessor): More items collected
   - Slot 1 (early-game Wretch): Fewer items collected
2. Find pickup flags from extracted_event_flags.json for all areas
3. Search for base offset where temporal differential is maximized
4. Output ground_truth_offsets.json compatible entries
"""

import struct
import json
from pathlib import Path
from collections import defaultdict
from datetime import datetime

# Paths
SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
EXTRACTED_FLAGS = Path("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/public/data/extracted_event_flags.json")
GROUND_TRUTH = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/ground_truth_offsets.json")

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

# Known VERIFIED dungeon pickup bases (do not re-discover these!)
# These were verified via temporal differential
VERIFIED_PICKUP_BASES = {
    10: 31906,  # Stormveil Castle - CORRECTED 2026-02-02 (was 6459, which showed 0 matches)
    11: 33725,  # Leyndell Royal Capital - VERIFIED 2026-01-23
    30: 17731,  # Catacombs - VERIFIED 2026-02-01
    31: 8346,   # Caves - VERIFIED 2026-02-01
    32: 29658,  # Tunnels - VERIFIED 2026-02-01
}

# Areas to skip in discovery (already verified)
SKIP_AREAS = set(VERIFIED_PICKUP_BASES.keys())

# Known general event bases (for reference)
GENERAL_EVENT_BASES = {
    10: 4112,
    11: 8612,
    12: 15362,
    13: 26612,
    14: 29987,
    15: 33362,
    16: 40517,
    18: 43487,
    19: 46862,
    20: 50237,
    21: 53612,
    22: 59237,
    30: 27411,
    31: 28634,
    32: 31577,
    34: 60362,
    35: 50237,
    39: 31112,
    40: 171737,
    41: 180737,
    42: 190737,
    43: 200737,
}

# Areas with dungeon pickups to discover (excluding already verified)
# Note: Areas 10, 11, 30, 31, 32 are already verified and will be skipped
AREAS_TO_DISCOVER = [12, 13, 14, 15, 16, 18, 20, 21, 22, 28, 34, 35, 39, 40, 41, 42, 43]


def load_all_pickup_flags() -> dict:
    """Load dungeon pickup flags (localId >= 7000) for all areas."""
    with open(EXTRACTED_FLAGS, 'r') as f:
        data = json.load(f)

    pickup_flags = defaultdict(list)

    for flag in data['flags']:
        if flag.get('category') != 'Dungeon Pickup':
            continue

        flag_id = flag['flag_id']
        if flag_id >= 10_000_000 and flag_id < 50_000_000:
            area = flag_id // 1_000_000
            local_id = flag_id % 10000
            section = (flag_id // 10000) % 100

            if local_id >= 7000:
                pickup_flags[area].append({
                    'flag_id': flag_id,
                    'name': flag.get('name', 'Unknown'),
                    'area': area,
                    'section': section,
                    'local_id': local_id,
                })

    return dict(pickup_flags)


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


def temporal_differential(slot0_ef: bytes, slot1_ef: bytes, flags: list, base: int) -> tuple:
    """
    Count flags SET in slot0 but UNSET in slot1 - these are true pickups.

    Returns (differential, slot0_count, slot1_count, matched_flags)
    """
    differential = 0
    slot0_count = 0
    slot1_count = 0
    matched_flags = []

    for f in flags:
        set_slot0 = check_flag_at_base(slot0_ef, f['flag_id'], base, DUNGEON_SECTION_SIZE)
        set_slot1 = check_flag_at_base(slot1_ef, f['flag_id'], base, DUNGEON_SECTION_SIZE)

        if set_slot0:
            slot0_count += 1
        if set_slot1:
            slot1_count += 1

        # Key metric: flags that changed between saves (collected in slot0 but not slot1)
        if set_slot0 and not set_slot1:
            differential += 1
            matched_flags.append(f)

    return differential, slot0_count, slot1_count, matched_flags


def discover_pickup_base(slot0_ef: bytes, slot1_ef: bytes, area: int, flags: list) -> dict:
    """
    Discover the pickup base for a given area using temporal differential.

    Returns dict with:
    - best_base: The discovered base offset
    - differential: Number of flags that differ between saves
    - slot0_count: Total flags set in slot 0
    - slot1_count: Total flags set in slot 1
    - matched_flags: List of flags that matched
    - confidence: Confidence score (0-1)
    """
    if not flags:
        return None

    best_base = None
    best_differential = 0
    best_slot0 = 0
    best_slot1 = 0
    best_matched = []

    # Search a wide range (0-100000 should cover all dungeon bases)
    for base in range(0, 100000):
        diff, s0, s1, matched = temporal_differential(slot0_ef, slot1_ef, flags, base)

        # We want the base that maximizes the temporal differential
        # This indicates flags that are SET in mid-game but UNSET in early-game
        if diff > best_differential:
            best_differential = diff
            best_base = base
            best_slot0 = s0
            best_slot1 = s1
            best_matched = matched

    # Calculate confidence
    # Higher differential relative to total flags = more confident
    confidence = best_differential / len(flags) if flags else 0

    return {
        'best_base': best_base,
        'differential': best_differential,
        'slot0_count': best_slot0,
        'slot1_count': best_slot1,
        'matched_flags': best_matched,
        'total_flags': len(flags),
        'confidence': confidence,
    }


def verify_known_bases(slot0_ef: bytes, slot1_ef: bytes, pickup_flags: dict):
    """Verify known pickup bases still work correctly."""
    print("\n" + "=" * 70)
    print("VERIFYING KNOWN PICKUP BASES")
    print("=" * 70)

    for area, known_base in VERIFIED_PICKUP_BASES.items():
        if area not in pickup_flags:
            continue

        flags = pickup_flags[area]
        diff, s0, s1, matched = temporal_differential(slot0_ef, slot1_ef, flags, known_base)

        status = "VERIFIED" if diff >= 5 else "NEEDS REVIEW"
        print(f"\n  Area {area}: base {known_base}")
        print(f"    Temporal diff: {diff} flags (slot0={s0}, slot1={s1})")
        print(f"    Status: {status}")
        if matched[:3]:
            print(f"    Sample matches: {[f['name'] for f in matched[:3]]}")


def discover_all_areas(slot0_ef: bytes, slot1_ef: bytes, pickup_flags: dict) -> dict:
    """Discover pickup bases for areas that are not yet verified."""
    print("\n" + "=" * 70)
    print("DISCOVERING PICKUP BASES FOR UNVERIFIED AREAS")
    print("=" * 70)

    results = {}

    for area in sorted(pickup_flags.keys()):
        # Skip already verified areas - use the known bases for them
        if area in SKIP_AREAS:
            print(f"\n  Area {area}: SKIPPED (already verified, base={VERIFIED_PICKUP_BASES[area]})")
            # Include verified bases in results for completeness
            flags = pickup_flags[area]
            known_base = VERIFIED_PICKUP_BASES[area]
            diff, s0, s1, matched = temporal_differential(slot0_ef, slot1_ef, flags, known_base)
            results[area] = {
                'best_base': known_base,
                'differential': diff,
                'slot0_count': s0,
                'slot1_count': s1,
                'matched_flags': matched,
                'total_flags': len(flags),
                'confidence': diff / len(flags) if flags else 0,
                'previously_verified': True,
            }
            continue

        flags = pickup_flags[area]
        print(f"\n  Area {area}: {len(flags)} pickup flags")

        # Skip areas with very few flags
        if len(flags) < 3:
            print(f"    Skipped: too few flags")
            continue

        result = discover_pickup_base(slot0_ef, slot1_ef, area, flags)

        if result and result['best_base'] is not None:
            results[area] = result
            print(f"    Best base: {result['best_base']}")
            print(f"    Temporal diff: {result['differential']} (slot0={result['slot0_count']}, slot1={result['slot1_count']})")
            print(f"    Confidence: {result['confidence']:.2%}")
            if result['matched_flags'][:3]:
                print(f"    Sample matches:")
                for f in result['matched_flags'][:3]:
                    print(f"      {f['flag_id']}: {f['name']}")
        else:
            print(f"    No base found")

    return results


def generate_ground_truth_entries(results: dict) -> dict:
    """Generate ground_truth_offsets.json compatible entries."""
    entries = {}

    for area, result in results.items():
        # Skip already verified areas - they're already in ground_truth
        if result.get('previously_verified'):
            continue

        # Only include areas with good temporal differential
        if result['differential'] < 3:
            continue

        # Determine status based on confidence and differential
        if result['differential'] >= 10 or result['confidence'] >= 0.2:
            status = "verified"
        elif result['differential'] >= 5:
            status = "likely_correct"
        else:
            status = "needs_verification"

        # Generate notes
        sample_items = [f['name'] for f in result['matched_flags'][:3]]
        notes = (f"DISCOVERED {datetime.now().strftime('%Y-%m-%d')}: "
                f"Temporal diff shows slot0 (mid-game) has {result['slot0_count']} pickups, "
                f"slot1 (early-game) has {result['slot1_count']} pickups. "
                f"Items include: {', '.join(sample_items)}.")

        entries[str(area)] = {
            "map_area": area,
            "base_offset": result['best_base'],
            "section_size": DUNGEON_SECTION_SIZE,
            "status": status,
            "notes": notes
        }

    return entries


def main():
    print("=" * 70)
    print("ALL DUNGEON PICKUP BASE DISCOVERY")
    print("=" * 70)

    # Load pickup flags
    pickup_flags = load_all_pickup_flags()
    print(f"\nLoaded pickup flags:")
    for area in sorted(pickup_flags.keys()):
        print(f"  Area {area}: {len(pickup_flags[area])} flags")

    # Parse save file
    save_path = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"
    if not save_path.exists():
        save_path = SAVE_DIR / "ER0000-backup-2026-01-01.sl2"

    if not save_path.exists():
        # Try to find any backup file
        backups = list(SAVE_DIR.glob("ER0000-backup-*.sl2"))
        if backups:
            save_path = max(backups, key=lambda p: p.stat().st_mtime)
        else:
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

    print(f"\nSlot 0 (mid-game) EF offset: 0x{slot0['ef_offset']:X}")
    print(f"Slot 1 (early-game) EF offset: 0x{slot1['ef_offset']:X}")

    # Verify known bases first
    verify_known_bases(slot0['event_flags'], slot1['event_flags'], pickup_flags)

    # Discover all areas
    results = discover_all_areas(slot0['event_flags'], slot1['event_flags'], pickup_flags)

    # Generate ground truth entries
    print("\n" + "=" * 70)
    print("GENERATED GROUND TRUTH ENTRIES")
    print("=" * 70)

    entries = generate_ground_truth_entries(results)
    print(json.dumps(entries, indent=2))

    # Summary
    print("\n" + "=" * 70)
    print("DISCOVERY SUMMARY")
    print("=" * 70)

    verified_count = sum(1 for e in entries.values() if e['status'] == 'verified')
    likely_count = sum(1 for e in entries.values() if e['status'] == 'likely_correct')
    needs_review = sum(1 for e in entries.values() if e['status'] == 'needs_verification')

    print(f"\n  Total areas processed: {len(results)}")
    print(f"  Verified (diff >= 10 or conf >= 20%): {verified_count}")
    print(f"  Likely correct (diff >= 5): {likely_count}")
    print(f"  Needs verification: {needs_review}")

    # Compare with known bases
    print("\n  Comparison with known bases:")
    for area in sorted(VERIFIED_PICKUP_BASES.keys()):
        known = VERIFIED_PICKUP_BASES[area]
        if area in results:
            discovered = results[area]['best_base']
            match = "" if known == discovered else f" MISMATCH (known={known})"
            print(f"    Area {area}: discovered {discovered}{match}")

    # Output for copy-paste into ground_truth_offsets.json
    print("\n" + "=" * 70)
    print("JSON FOR GROUND TRUTH (copy to dungeon_pickup_bases section):")
    print("=" * 70)
    print(json.dumps(entries, indent=6))


if __name__ == "__main__":
    main()
