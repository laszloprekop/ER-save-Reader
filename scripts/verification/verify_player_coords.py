#!/usr/bin/env python3
"""
Verify player coordinate extraction from save file snapshots.

Reads PlayerCoords from granular snapshot save files and compares
against known grace/boss world positions to validate correctness.

Extraction method: signature-based search for the map_id pattern
(from the slot header) followed by characteristic padding bytes,
with coordinate validation. This avoids needing to parse through
all intermediate structures (EventFlags, UknownLists) to reach
the PlayerCoords struct.

PlayerCoords struct (from save_slot.rs:82-128):
    player_coords: (f32, f32, f32) = 12 bytes
    map_id: [u8; 4] = 4 bytes        <-- matches slot header map_id
    _0x11: 17 bytes padding
    player_coords2: (f32, f32, f32) = 12 bytes
    _0x10: 16 bytes padding           <-- mostly zeros, strong signature
"""

import struct
import json
import math
import sys
from pathlib import Path

# ============================================================================
# CONSTANTS (loaded from ground_truth_offsets.json via ground_truth_loader)
# ============================================================================

# Add parent path for imports when running standalone
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from scripts.verification.ground_truth_loader import get_player_coords_config

_coords_config = get_player_coords_config()

BND4_HEADER_SIZE = 0x40
BND4_ENTRY_SIZE = 0x20
BND4_ENTRY_OFFSET_POS = 0x10
SLOT_CHECKSUM_SIZE = 16

PLAYER_COORDS_SEARCH_START = _coords_config.get("search_start", 0x1D0000)
PLAYER_COORDS_SEARCH_END = _coords_config.get("search_end", 0x280000)
PLAYER_COORDS_SIZE = _coords_config.get("struct_size", 61)
MID_SECTION_SIZE = _coords_config.get("mid_section_size", 17)
MID_SECTION_MIN_ZEROS = _coords_config.get("mid_section_min_zeros", 10)
FACING_ANGLE_OFFSET = _coords_config.get("facing_angle_offset", 4)
PAD2_SIZE = _coords_config.get("padding2_size", 16)
PAD2_MIN_ZEROS = _coords_config.get("padding2_min_zeros", 8)
COORD_RANGE_MAX = _coords_config.get("coordinate_range_max", 10000.0)
MAGNITUDE_THRESHOLD = _coords_config.get("magnitude_threshold", 10.0)

# ============================================================================
# REFERENCE POSITIONS (loaded from generated reference_positions.json)
# ============================================================================

REFERENCE_POSITIONS_PATH = Path(__file__).parent / "reference_positions.json"

# Hardcoded fallback for when reference_positions.json hasn't been generated yet
_FALLBACK_REFERENCE_POSITIONS = {
    "Minor Erdtree Church grace": (-116.50, 910.02, -54.99),
    "The First Step grace": (-12.83, 90.70, -54.50),
    "Church of Elleh grace": (-40.73, 90.97, 79.34),
    "Gatefront grace": (13.61, 110.65, 110.21),
    "Agheel Lake North grace": (-80.05, 88.81, 56.10),
    "Volcano Manor grace": (40.88, 4.95, -60.27),
    "Prison Town Church grace": (-62.98, -9.21, -113.62),
    "Crucible Knight (Stormhill)": (-83.61, 160.59, 65.00),
    "Erdtree Burial Watchdog (Stormfoot)": (103.67, 94.93, 74.18),
    "Cave of Knowledge grace": (-88.40, -10.17, 43.86),
}


def load_reference_positions():
    """Load reference positions from generated JSON, with hardcoded fallback."""
    if REFERENCE_POSITIONS_PATH.exists():
        with open(REFERENCE_POSITIONS_PATH, 'r') as f:
            data = json.load(f)
        positions = {}
        for entry in data.get("positions", []):
            name = entry["name"]
            suffix = " grace" if entry["type"] == "grace" else ""
            key = f"{name}{suffix}"
            positions[key] = (entry["x"], entry["y"], entry["z"])
        return positions
    return _FALLBACK_REFERENCE_POSITIONS


REFERENCE_POSITIONS = load_reference_positions()


def euclidean_distance(p1, p2):
    """3D Euclidean distance."""
    return math.sqrt(sum((a - b) ** 2 for a, b in zip(p1, p2)))


def find_slot_offset(data, slot_index):
    """Read BND4 entry to find absolute offset for a slot."""
    entry_offset = BND4_HEADER_SIZE + (slot_index * BND4_ENTRY_SIZE)
    if entry_offset + BND4_ENTRY_SIZE > len(data):
        return None
    slot_data_offset = struct.unpack_from('<I', data, entry_offset + BND4_ENTRY_OFFSET_POS)[0]
    return slot_data_offset + SLOT_CHECKSUM_SIZE


def find_player_coords_by_signature(slot_data, header_map_id_bytes):
    """
    Find PlayerCoords by searching for the map_id + padding signature.

    The map_id in PlayerCoords matches the map_id from the slot header (bytes 4-7).
    The struct has a distinctive 16-byte padding block after coords2 that's mostly zeros.

    Selection criteria for the real PlayerCoords among candidates:
    1. Non-zero coordinates (the real position, not a cleared/default struct)
    2. High zero count in padding2 (16/16 or 13+/16)
    3. High zero count in padding1 (12+/17)
    """
    candidates = []

    search_end = min(len(slot_data), PLAYER_COORDS_SEARCH_END)

    for i in range(PLAYER_COORDS_SEARCH_START, search_end - PLAYER_COORDS_SIZE):
        # Check if 4 bytes at this position match header map_id
        if slot_data[i:i+4] != header_map_id_bytes:
            continue

        # Check padding2 (PAD2_SIZE bytes after coords2): should be mostly zeros
        padding2_start = i + 4 + MID_SECTION_SIZE + 12  # map_id + mid_section + coords2
        if padding2_start + PAD2_SIZE > len(slot_data):
            continue
        padding2 = slot_data[padding2_start:padding2_start+PAD2_SIZE]
        padding2_zeros = sum(1 for b in padding2 if b == 0)
        if padding2_zeros < PAD2_MIN_ZEROS:
            continue

        # Check mid_section (MID_SECTION_SIZE bytes after map_id): should be mostly zeros
        padding1 = slot_data[i+4:i+4+MID_SECTION_SIZE]
        padding1_zeros = sum(1 for b in padding1 if b == 0)
        if padding1_zeros < MID_SECTION_MIN_ZEROS:
            continue

        # Read coords before map_id (12 bytes = 3 x f32)
        if i < 12:
            continue
        coords_offset = i - 12
        x, y, z = struct.unpack_from('<fff', slot_data, coords_offset)

        # Skip NaN/Inf/out-of-range
        if any(math.isnan(c) or math.isinf(c) or abs(c) > COORD_RANGE_MAX for c in (x, y, z)):
            continue

        # Read coords2
        x2, y2, z2 = struct.unpack_from('<fff', slot_data, i + 4 + MID_SECTION_SIZE)
        if any(math.isnan(c) or math.isinf(c) or abs(c) > COORD_RANGE_MAX for c in (x2, y2, z2)):
            continue

        # Read facing angle from mid_section bytes [FACING_ANGLE_OFFSET:FACING_ANGLE_OFFSET+4]
        facing_angle = struct.unpack_from('<f', slot_data, i + 4 + FACING_ANGLE_OFFSET)[0]
        if not math.isfinite(facing_angle):
            facing_angle = 0.0

        # Magnitude threshold to distinguish real positions from near-zero
        magnitude = abs(x) + abs(y) + abs(z)
        has_position = magnitude > MAGNITUDE_THRESHOLD

        candidates.append({
            'offset': coords_offset,
            'coords': (x, y, z),
            'coords2': (x2, y2, z2),
            'facing_angle': facing_angle,
            'map_id': struct.unpack_from('<4B', slot_data, i),
            'padding1_zeros': padding1_zeros,
            'padding2_zeros': padding2_zeros,
            'has_position': has_position,
        })

    if not candidates:
        return None

    # Select best candidate: prefer non-zero coords, then highest padding zeros
    candidates.sort(key=lambda c: (
        c['has_position'],      # Non-zero coords first
        c['padding2_zeros'],    # More padding2 zeros = better
        c['padding1_zeros'],    # More padding1 zeros = better
    ), reverse=True)

    best = candidates[0]
    if not best['has_position']:
        return None  # No candidate with real coordinates found

    return best


def extract_coords_from_save(file_path, slot_index=0):
    """Extract player coordinates from a save file for a given slot."""
    data = Path(file_path).read_bytes()

    slot_offset = find_slot_offset(data, slot_index)
    if slot_offset is None:
        return None

    slot_data = data[slot_offset:]

    # Read header map_id (bytes 4-7 of slot)
    header_map_id = slot_data[4:8]

    result = find_player_coords_by_signature(slot_data, header_map_id)
    if result is None:
        return None

    result['slot_offset'] = slot_offset
    return result


def format_map_id(map_id):
    """Format map_id tuple into human-readable form."""
    return f"m{map_id[3]}_{map_id[2]:02d}_{map_id[1]:02d}_{map_id[0]:02d}"


def find_nearest_reference(coords, references):
    """Find the nearest reference position to the given coords."""
    nearest = None
    nearest_dist = float('inf')
    for name, ref_pos in references.items():
        if ref_pos is None:
            continue
        dist = euclidean_distance(coords, ref_pos)
        if dist < nearest_dist:
            nearest_dist = dist
            nearest = name
    return nearest, nearest_dist


# ============================================================================
# Test cases
# ============================================================================

SNAPSHOT_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")

TEST_CASES = [
    # (file_path_relative, slot_index, expected_near, description, threshold)
    # Confessor slot 0 - Minor Erdtree Church area
    ("slot 0 Confessor/ER0000.sl2 Confessor - 03 before touching Minor Erdtree Church grace EF-76310 mapTile-m60_43_50", 0,
     "Minor Erdtree Church grace", "Before touching Minor Erdtree Church grace", 100),
    ("slot 0 Confessor/ER0000.sl2 Confessor - 04 after touched Minor Erdtree Church grace EF-76310 mapTile-m60_43_50", 0,
     "Minor Erdtree Church grace", "After touched Minor Erdtree Church grace", 100),

    # Confessor slot 0 - Crucible Knight fight (Evergaol)
    ("slot 0 Confessor/ER0000.sl2 S0 - b11 before defeating Crucible Knight EF-1042370800 rowId-1042370800 mapTile-m60_42_37", 0,
     "Crucible Knight (Stormhill)", "Before defeating Crucible Knight", 200),
    ("slot 0 Confessor/ER0000.sl2 S0 - b12 after Crucible Knight felled EF-1042370800 rowId-1042370800 mapTile-m60_42_37", 0,
     "Crucible Knight (Stormhill)", "After Crucible Knight felled", 200),

    # Confessor slot 0 - Erdtree Burial Watchdog (Stormfoot Catacombs)
    ("slot 0 Confessor/ER0000.sl2 S0 - b24 Before first traversing the yellow mist to defeat EF-30020800 npc_param_id-42600110 lotItemId01-2200 mapTile-m30_02_00", 0,
     "Erdtree Burial Watchdog (Stormfoot)", "Before fighting Erdtree Burial Watchdog", 200),
    ("slot 0 Confessor/ER0000.sl2 S0 - b25 After Erdtree Burial Watchdog felled EF-30020800 npc_param_id-42600110 lotItemId01-2200 mapTile-m30_02_00", 0,
     "Erdtree Burial Watchdog (Stormfoot)", "After Erdtree Burial Watchdog felled", 200),

    # Confessor slot 0 - Volcano Manor grace
    ("slot 0 Confessor/ER0000.sl2 S0 - b38 before discovering Volcano Manor grace EF-71602 rowId-160002 m16_00_00", 0,
     "Volcano Manor grace", "Before discovering Volcano Manor grace", 100),
    ("slot 0 Confessor/ER0000.sl2 S0 - b39 after discovering Volcano Manor grace EF-71602 rowId-160002 m16_00_00", 0,
     "Volcano Manor grace", "After discovering Volcano Manor grace", 100),

    # Wretch slot 1 - The First Step grace
    # Before: player is still on m18_00_00 (Chapel of Anticipation area), not yet near grace
    ("slot 1 Wretch/ER0000.sl2 Wretch - 14 Limgrave, before The First Step grace", 1,
     "The First Step grace", "Before touching The First Step grace (in transit)", 200),
    ("slot 1 Wretch/ER0000.sl2 Wretch - 15 Limgrave, touched The First Step grace", 1,
     "The First Step grace", "After touching The First Step grace", 100),

    # Wretch slot 1 - Church of Elleh grace
    ("slot 1 Wretch/ER0000.sl2 Wretch - 19 Limgrave, touched Church of Elleh grace", 1,
     "Church of Elleh grace", "After touching Church of Elleh grace", 100),

    # Wretch slot 1 - Gatefront grace
    ("slot 1 Wretch/ER0000.sl2 Wretch - 22 Limgrave, touched Gatefront grace", 1,
     "Gatefront grace", "After touching Gatefront grace", 100),

    # Wretch slot 1 - Agheel Lake North grace
    # Player dismounted Torrent and moved away from grace after touching it
    ("slot 1 Wretch/ER0000.sl2 Wretch - 28 Limgrave, touched Agheel Lake North grace, dismounted", 1,
     "Agheel Lake North grace", "After touching Agheel Lake North grace (dismounted, moved)", 300),

    # Root captures - Prison Town Church grace
    ("ER0000.sl2_capture_115_before_71603", 0,
     "Prison Town Church grace", "Before touching Prison Town Church grace", 100),
    ("ER0000.sl2_capture_116_after_71603", 0,
     "Prison Town Church grace", "After touching Prison Town Church grace", 100),
]


def main():
    print("=" * 80)
    print("Player Coordinate Extraction Verification")
    print("=" * 80)
    print("Method: signature-based search (map_id + padding pattern)")

    passed = 0
    failed = 0
    errors = 0

    for rel_path, slot_idx, expected_near, description, threshold in TEST_CASES:
        file_path = SNAPSHOT_DIR / rel_path
        if not file_path.exists():
            print(f"\n  SKIP: {description}")
            print(f"    File not found: {rel_path}")
            errors += 1
            continue

        print(f"\n  TEST: {description}")
        result = extract_coords_from_save(file_path, slot_idx)

        if result is None:
            print(f"    ERROR: Could not extract coordinates")
            errors += 1
            continue

        coords = result['coords']
        coords2 = result['coords2']
        map_id = result['map_id']

        print(f"    Offset: 0x{result['offset']:X}")
        print(f"    Map ID: {format_map_id(map_id)} (raw: {list(map_id)})")
        print(f"    Position (current): ({coords[0]:.2f}, {coords[1]:.2f}, {coords[2]:.2f})")
        print(f"    Position (respawn): ({coords2[0]:.2f}, {coords2[1]:.2f}, {coords2[2]:.2f})")
        facing = result.get('facing_angle', 0.0)
        print(f"    Facing angle: {math.degrees(facing):.1f}° ({facing:.4f} rad)")
        print(f"    Padding quality: pad1={result['padding1_zeros']}/{MID_SECTION_SIZE} pad2={result['padding2_zeros']}/{PAD2_SIZE}")

        if not result['has_position']:
            print(f"    WARNING: All-zero coordinates")
            errors += 1
            continue

        # Check proximity to expected reference
        ref_pos = REFERENCE_POSITIONS.get(expected_near)
        if ref_pos is not None:
            dist = euclidean_distance(coords, ref_pos)

            if dist <= threshold:
                print(f"    PASS: {dist:.1f} units from {expected_near} (threshold: {threshold})")
                passed += 1
            else:
                print(f"    FAIL: {dist:.1f} units from {expected_near} (threshold: {threshold})")
                nearest, nearest_dist = find_nearest_reference(coords, REFERENCE_POSITIONS)
                print(f"    Nearest reference: {nearest} at {nearest_dist:.1f} units")
                failed += 1
        else:
            nearest, nearest_dist = find_nearest_reference(coords, REFERENCE_POSITIONS)
            print(f"    INFO: Nearest reference: {nearest} at {nearest_dist:.1f} units")
            passed += 1

    print("\n" + "=" * 80)
    print(f"Results: {passed} passed, {failed} failed, {errors} errors out of {len(TEST_CASES)} tests")
    print("=" * 80)

    return 0 if failed == 0 and errors == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
