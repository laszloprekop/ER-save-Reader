"""
Ground Truth Loader - Single source of truth for verification scripts.

Reads from ground_truth_offsets.json (synced with src/generated/ground_truth.rs).

Usage:
    from ground_truth_loader import load_block_bases, load_dungeon_bases, get_tile_config

    # Get all block bases
    bases = load_block_bases()
    print(bases[71000])  # {'base_offset': 9315, 'block_size': 100, 'status': 'verified', ...}

    # Get a specific block base offset
    base = get_block_base(71600)  # Returns 2825

    # Calculate flag offset
    offset, bit = calculate_block_offset(71607)  # Returns (2825, 0)
"""

import json
from pathlib import Path
from typing import Dict, Optional, Tuple, Any


# Path to ground truth JSON (relative to this file)
GROUND_TRUTH_PATH = Path(__file__).parent.parent.parent / "ground_truth_offsets.json"


def _load_json() -> Dict[str, Any]:
    """Load and cache the ground truth JSON."""
    if not hasattr(_load_json, '_cache'):
        with open(GROUND_TRUTH_PATH, 'r') as f:
            _load_json._cache = json.load(f)
    return _load_json._cache


def load_block_bases() -> Dict[int, Dict[str, Any]]:
    """
    Load block base offsets from ground truth.

    Returns:
        Dict mapping block_start (int) to config dict with:
        - base_offset: int
        - block_size: int
        - status: str ("verified", "candidate", "calculated", "disproven", "unverified")
        - notes: str
    """
    data = _load_json()
    block_bases = data.get("formulas", {}).get("block_bases", {})

    return {
        int(k): {
            "base_offset": v["base_offset"],
            "block_size": v.get("block_size", 1000),
            "status": v.get("status", "unverified"),
            "notes": v.get("notes", ""),
        }
        for k, v in block_bases.items()
    }


def load_dungeon_bases() -> Dict[int, Dict[str, Any]]:
    """
    Load dungeon base offsets from ground truth.

    Returns:
        Dict mapping map_area (int) to config dict with:
        - base_offset: int
        - section_size: int (typically 1125)
        - status: str
        - notes: str
    """
    data = _load_json()
    dungeon_formula = data.get("formulas", {}).get("dungeon_formula", {})

    return {
        int(k): {
            "base_offset": v["base_offset"],
            "section_size": v.get("section_size", 1125),
            "status": v.get("status", "unverified"),
            "notes": v.get("notes", ""),
        }
        for k, v in dungeon_formula.items()
    }


def get_tile_config() -> Dict[str, Any]:
    """
    Get verified tile formula configuration.

    Returns:
        Dict with:
        - base_offset: int (489981 as of 2026-01-20)
        - bytes_per_slot: int (875)
        - slots_per_row: int (40)
        - row_base: int (33)
        - col_base: int (30)
        - max_local_id: int (6999)
        - status: str
        - notes: str
    """
    data = _load_json()
    return data.get("formulas", {}).get("tile_formula", {})


def get_block_base(flag_id: int) -> Optional[int]:
    """
    Get the base offset for a block-based flag (5-6 digits).

    Tries sub-block granularity (100) first, then falls back to main block (1000).

    Args:
        flag_id: The event flag ID (e.g., 71607)

    Returns:
        The base offset (int) or None if not found
    """
    bases = load_block_bases()

    # Try 100-flag granularity first (sub-block)
    sub_block = (flag_id // 100) * 100
    if sub_block in bases:
        return bases[sub_block]["base_offset"]

    # Fall back to 1000-flag granularity (main block)
    main_block = (flag_id // 1000) * 1000
    if main_block in bases:
        return bases[main_block]["base_offset"]

    return None


def get_dungeon_base(map_area: int) -> Optional[int]:
    """
    Get the base offset for a dungeon map area.

    Args:
        map_area: The dungeon map area code (e.g., 10 for Stormveil, 30 for Catacombs)

    Returns:
        The base offset (int) or None if not found
    """
    bases = load_dungeon_bases()
    if map_area in bases:
        return bases[map_area]["base_offset"]
    return None


def calculate_block_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """
    Calculate byte offset and bit position for a block-based flag.

    Args:
        flag_id: The event flag ID (5-6 digits)

    Returns:
        Tuple of (byte_offset, bit_position) or None if block not found
    """
    # Get the appropriate block start
    sub_block = (flag_id // 100) * 100
    main_block = (flag_id // 1000) * 1000
    bases = load_block_bases()

    if sub_block in bases:
        block_start = sub_block
        base_offset = bases[sub_block]["base_offset"]
    elif main_block in bases:
        block_start = main_block
        base_offset = bases[main_block]["base_offset"]
    else:
        return None

    relative = flag_id - block_start
    byte_offset = base_offset + relative // 8
    bit_position = 7 - (flag_id % 8)

    return (byte_offset, bit_position)


def calculate_tile_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """
    Calculate byte offset and bit position for a tile-based flag (10-digit).

    Format: 10XXYYZZZZ where XX=row, YY=col, ZZZZ=local_id

    Args:
        flag_id: The 10-digit event flag ID (e.g., 1043500010)

    Returns:
        Tuple of (byte_offset, bit_position) or None if invalid
    """
    # Validate format
    if not (1_000_000_000 <= flag_id < 2_000_000_000):
        return None

    flag_str = str(flag_id)
    if len(flag_str) != 10:
        return None

    config = get_tile_config()
    if not config or config.get("base_offset", 0) == 0:
        return None

    # Parse components
    row = int(flag_str[2:4])
    col = int(flag_str[4:6])
    local_id = int(flag_str[6:])

    # Check localId limit
    max_local = config.get("max_local_id", 6999)
    if local_id > max_local:
        return None  # Untrackable

    # Calculate tile slot offset
    row_base = config.get("row_base", 33)
    col_base = config.get("col_base", 30)
    bytes_per_slot = config.get("bytes_per_slot", 875)
    slots_per_row = config.get("slots_per_row", 40)

    tile_offset = ((row - row_base) * slots_per_row + (col - col_base)) * bytes_per_slot
    byte_offset = config["base_offset"] + tile_offset + local_id // 8
    bit_position = 7 - (local_id % 8)  # Use local_id, NOT flag_id!

    return (byte_offset, bit_position)


def calculate_dungeon_offset(flag_id: int) -> Optional[Tuple[int, int]]:
    """
    Calculate byte offset and bit position for a dungeon flag (8-digit).

    Format: AASSZZZZ where AA=map_area, SS=section, ZZZZ=local_id

    Args:
        flag_id: The 8-digit event flag ID (e.g., 30020800)

    Returns:
        Tuple of (byte_offset, bit_position) or None if invalid
    """
    # Validate format
    if not (10_000_000 <= flag_id < 100_000_000):
        return None

    flag_str = f"{flag_id:08d}"

    map_area = int(flag_str[0:2])
    section = int(flag_str[2:4])
    local_id = int(flag_str[4:8])

    base = get_dungeon_base(map_area)
    if base is None or base == 0:
        return None

    bases = load_dungeon_bases()
    section_size = bases[map_area].get("section_size", 1125)

    byte_offset = base + section * section_size + local_id // 8
    bit_position = 7 - (local_id % 8)  # Use local_id, NOT flag_id!

    return (byte_offset, bit_position)


def get_validation_flags() -> Dict[int, Tuple[int, int, str]]:
    """
    Get the anchor validation flags used for EF start detection.

    Returns:
        Dict mapping flag_id to (relative_offset, bit, name)
    """
    return {
        71800: (2725, 7, "Cave of Knowledge"),
        71801: (2725, 6, "Stranded Graveyard"),
        76100: (3262, 3, "The First Step"),
        76101: (3262, 2, "Church of Elleh"),
    }


# Quick test when run directly
if __name__ == "__main__":
    print("Ground Truth Loader Test")
    print("=" * 50)

    print("\nBlock Bases:")
    bases = load_block_bases()
    for block, config in sorted(bases.items()):
        print(f"  {block}: offset={config['base_offset']}, status={config['status']}")

    print("\nDungeon Bases:")
    dungeons = load_dungeon_bases()
    for area, config in sorted(dungeons.items()):
        if config['base_offset'] > 0:
            print(f"  Area {area}: offset={config['base_offset']}, status={config['status']}")

    print("\nTile Config:")
    tile = get_tile_config()
    print(f"  base_offset={tile.get('base_offset')}, status={tile.get('status')}")

    print("\nTest Calculations:")
    test_flags = [
        (71800, "Cave of Knowledge (block)"),
        (71607, "Volcano Manor grace (sub-block)"),
        (1043500010, "Smoldering Butterfly (tile)"),
        (30020800, "Catacombs boss (dungeon)"),
    ]
    for flag_id, name in test_flags:
        if flag_id < 10_000_000:
            result = calculate_block_offset(flag_id)
        elif flag_id < 100_000_000:
            result = calculate_dungeon_offset(flag_id)
        else:
            result = calculate_tile_offset(flag_id)

        if result:
            print(f"  {flag_id} ({name}): offset={result[0]}, bit={result[1]}")
        else:
            print(f"  {flag_id} ({name}): NOT FOUND")
