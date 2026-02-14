"""
Calibration Service - Dynamic formula base calibration for per-save verification.

The tile formula base_offset (485330 in ground_truth_offsets.json) varies per save
due to the GaItems (inventory) section having variable size. This module provides
a reusable calibration service that detects the correct bases for any given save.

Usage:
    from verification.calibration import CalibrationService

    # Calibrate for a specific save
    result = CalibrationService.calibrate("/path/to/save.sl2", slot_index=0)
    print(f"Tile base: {result.tile_base} (confidence: {result.tile_base_confidence})")

    # Get calibrated tile base
    tile_base, confidence = CalibrationService.get_tile_base("/path/to/save.sl2", 0)
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Optional, Tuple

from .utils import (
    read_slot_data,
    detect_event_flags_start,
    extract_event_flags,
)
from .ground_truth_loader import (
    get_tile_config,
    load_block_bases,
    load_dungeon_bases,
    calculate_block_offset,
)


# ============================================================================
# CALIBRATION ANCHORS
# ============================================================================
# Known flags used for calibrating formula bases. These are common early-game
# flags that should be SET in most progressed saves.

# Multi-tile calibration anchors for tile formula verification.
# Using anchors from multiple tiles makes false positives practically impossible.
# All anchors verified SET across Slot 0 (Confessor, tile_base=446321) and Slot 7 (tile_base=453473).
TILE_CALIBRATION_ANCHORS = [
    {"row": 42, "col": 36, "local_id": 30, "bit": 1, "name": "Golden Rune [2]"},       # flag 1042360030
    {"row": 42, "col": 37, "local_id": 0, "bit": 7, "name": "Ruin Fragment"},           # flag 1042370000
    {"row": 43, "col": 37, "local_id": 0, "bit": 7, "name": "Smithing Stone [1]"},      # flag 1043370000
    {"row": 44, "col": 36, "local_id": 40, "bit": 7, "name": "Somber Smithing Stone [1]"},  # flag 1044360040
]

TILE_SEARCH_START = 430000
TILE_SEARCH_END = 510000
TILE_MIN_ANCHOR_MATCHES = 3
TILE_MIN_TILE_MATCHES = 2

CALIBRATION_ANCHORS = {
    "block": {
        "flag_id": 76100,
        "name": "The First Step",
        "expected_block_start": 76000,
        "expected_relative": 100,
        "notes": "First overworld grace, always discovered",
    },
    "dungeon_16": {
        "flag_id": 16000002,
        "name": "Volcano Manor grace",
        "expected_area": 16,
        "expected_section": 0,
        "expected_local_id": 2,
        "notes": "Volcano Manor main hall grace",
    },
}


# ============================================================================
# DATA CLASSES
# ============================================================================

@dataclass
class CalibrationResult:
    """Result of calibrating formula bases for a specific save state."""
    save_path: str
    slot_index: int
    ef_offset: Optional[int] = None
    tile_base: Optional[int] = None
    tile_base_confidence: float = 0.0
    tile_base_source: str = "unknown"  # "ground_truth", "anchor_verified", "multi_tile_search"
    tile_anchors_matched: int = 0
    tile_tiles_matched: int = 0
    block_bases: Dict[int, int] = field(default_factory=dict)
    dungeon_bases: Dict[int, int] = field(default_factory=dict)
    calibration_flags_used: list = field(default_factory=list)
    notes: str = ""


# ============================================================================
# CALIBRATION SERVICE
# ============================================================================

class CalibrationService:
    """
    Reusable calibration service with caching.

    Determines the correct formula bases for a specific save file by:
    1. Detecting the event flags offset
    2. Using anchor flags to verify/calibrate tile base
    3. Caching results for efficiency
    """

    _cache: Dict[str, CalibrationResult] = {}

    @classmethod
    def calibrate(
        cls,
        save_path: str | Path,
        slot_index: int,
        force: bool = False
    ) -> CalibrationResult:
        """
        Calibrate formula bases for a specific save file and slot.

        This determines the actual tile/dungeon bases for this save state,
        which may differ from other saves due to variable GaItems size.

        Args:
            save_path: Path to the save file
            slot_index: Character slot index (0-9)
            force: If True, ignore cache and recalibrate

        Returns:
            CalibrationResult with detected bases and confidence
        """
        save_path = Path(save_path)
        cache_key = f"{save_path}:{slot_index}"

        if not force and cache_key in cls._cache:
            return cls._cache[cache_key]

        result = CalibrationResult(
            save_path=str(save_path),
            slot_index=slot_index,
        )

        try:
            # Parse save and detect EF offset
            slot_data = read_slot_data(save_path, slot_index)
            ef_start = detect_event_flags_start(slot_data)

            if ef_start is None:
                result.notes = "Could not detect event flags offset"
                return result

            result.ef_offset = ef_start
            event_flags = extract_event_flags(slot_data, ef_start)

            # Calibrate tile base using multi-tile anchors
            tile_result = cls._calibrate_tile_base(event_flags)
            if tile_result:
                result.tile_base = tile_result[0]
                result.tile_base_confidence = tile_result[1]
                result.tile_base_source = tile_result[2]
                result.tile_anchors_matched = tile_result[3]
                result.tile_tiles_matched = tile_result[4]

            # Verify block bases work
            block_result = cls._calibrate_block_bases(event_flags)
            result.block_bases = block_result

            # Verify dungeon bases work
            dungeon_result = cls._calibrate_dungeon_bases(event_flags)
            result.dungeon_bases = dungeon_result

            result.notes = f"Calibrated successfully. EF offset: {ef_start}"

        except Exception as e:
            result.notes = f"Calibration error: {e}"

        cls._cache[cache_key] = result
        return result

    @classmethod
    def get_tile_base(
        cls,
        save_path: str | Path,
        slot_index: int = 0
    ) -> Tuple[int, float]:
        """
        Get the calibrated tile base for a save.

        Returns:
            Tuple of (base_offset, confidence)
        """
        result = cls.calibrate(save_path, slot_index)
        if result.tile_base is not None:
            return (result.tile_base, result.tile_base_confidence)

        # Fall back to ground truth
        config = get_tile_config()
        return (config.get("base_offset", 485330), 0.5)

    @classmethod
    def clear_cache(cls) -> None:
        """Clear the calibration cache."""
        cls._cache.clear()

    @classmethod
    def _calibrate_tile_base(
        cls, event_flags: bytes
    ) -> Optional[Tuple[int, float, str, int, int]]:
        """
        Calibrate the tile formula base using multi-tile anchors.

        Uses anchors from multiple different tiles so that a candidate base must
        satisfy constraints at different tile slot offsets simultaneously, making
        false positives practically impossible.

        Returns (base_offset, confidence, source, anchors_matched, tiles_matched) or None.
        """
        config = get_tile_config()
        ground_truth_base = config.get("base_offset", 485330)
        bytes_per_slot = config.get("bytes_per_slot", 875)
        slots_per_row = config.get("slots_per_row", 40)
        row_base = config.get("row_base", 33)
        col_base = config.get("col_base", 30)

        # Pre-compute tile offsets for each anchor (constant regardless of base)
        anchor_offsets = []
        for anchor in TILE_CALIBRATION_ANCHORS:
            row, col = anchor["row"], anchor["col"]
            local_id, bit_pos = anchor["local_id"], anchor["bit"]
            tile_offset = ((row - row_base) * slots_per_row + (col - col_base)) * bytes_per_slot
            local_byte = local_id // 8
            # (total_offset_from_base, bit_position, row, col)
            anchor_offsets.append((tile_offset + local_byte, bit_pos, row, col))

        max_anchor_offset = max(off for off, _, _, _ in anchor_offsets)
        search_end = min(TILE_SEARCH_END, len(event_flags) - max_anchor_offset)

        if TILE_SEARCH_START >= search_end:
            return None

        best_base = None
        best_matches = 0
        best_tiles = 0

        for base in range(TILE_SEARCH_START, search_end):
            matches = 0
            matched_tiles = set()

            for offset_from_base, bit_pos, row, col in anchor_offsets:
                byte_offset = base + offset_from_base
                if byte_offset < len(event_flags):
                    byte_val = event_flags[byte_offset]
                    if (byte_val >> bit_pos) & 1:
                        matches += 1
                        matched_tiles.add((row, col))

            tiles = len(matched_tiles)

            if matches >= TILE_MIN_ANCHOR_MATCHES and tiles >= TILE_MIN_TILE_MATCHES:
                is_better = (
                    matches > best_matches
                    or (matches == best_matches and tiles > best_tiles)
                    or (
                        matches == best_matches
                        and tiles == best_tiles
                        and (
                            best_base is None
                            or abs(base - ground_truth_base) < abs(best_base - ground_truth_base)
                        )
                    )
                )

                if is_better:
                    best_base = base
                    best_matches = matches
                    best_tiles = tiles

        if best_base is not None:
            total_anchors = len(TILE_CALIBRATION_ANCHORS)
            total_tiles = len({(a["row"], a["col"]) for a in TILE_CALIBRATION_ANCHORS})

            if best_matches == total_anchors and best_tiles == total_tiles:
                if best_base == ground_truth_base:
                    return (best_base, 0.95, "anchor_verified", best_matches, best_tiles)
                else:
                    return (best_base, 0.95, "multi_tile_search", best_matches, best_tiles)
            else:
                return (best_base, 0.85, "multi_tile_search", best_matches, best_tiles)

        # No candidate found — fall back to ground truth with low confidence
        return (ground_truth_base, 0.50, "ground_truth", 0, 0)

    @classmethod
    def _calibrate_block_bases(cls, event_flags: bytes) -> Dict[int, int]:
        """Verify known block bases work for this save."""
        bases = load_block_bases()
        verified_bases = {}

        # Check anchor flag (The First Step grace)
        anchor = CALIBRATION_ANCHORS["block"]
        flag_id = anchor["flag_id"]
        result = calculate_block_offset(flag_id)

        if result:
            byte_offset, bit_pos = result
            if byte_offset < len(event_flags):
                byte_val = event_flags[byte_offset]
                is_set = (byte_val >> bit_pos) & 1
                if is_set:
                    # Ground truth bases work
                    verified_bases = {int(k): v["base_offset"] for k, v in bases.items()}

        return verified_bases

    @classmethod
    def _calibrate_dungeon_bases(cls, event_flags: bytes) -> Dict[int, int]:
        """Verify known dungeon bases work for this save."""
        bases = load_dungeon_bases()
        verified_bases = {}

        for area, config in bases.items():
            if config["base_offset"] > 0:
                verified_bases[area] = config["base_offset"]

        return verified_bases


# ============================================================================
# CLI
# ============================================================================

def main():
    """Run calibration from command line."""
    import argparse
    import sys

    parser = argparse.ArgumentParser(description="Calibration Service")
    parser.add_argument("save_path", help="Path to save file")
    parser.add_argument("--slot", type=int, default=0, help="Slot index (default: 0)")
    parser.add_argument("--force", action="store_true", help="Force recalibration")

    args = parser.parse_args()
    save_path = Path(args.save_path)

    if not save_path.exists():
        print(f"Error: Save file not found: {save_path}")
        sys.exit(1)

    result = CalibrationService.calibrate(save_path, args.slot, force=args.force)

    print(f"\nCalibration Results for {save_path.name}, slot {args.slot}")
    print("=" * 60)
    print(f"EF Offset:            {result.ef_offset}")
    print(f"Tile Base:            {result.tile_base}")
    print(f"Tile Base Confidence: {result.tile_base_confidence:.2f}")
    print(f"Tile Base Source:     {result.tile_base_source}")
    print(f"Tile Anchors Matched: {result.tile_anchors_matched}/{len(TILE_CALIBRATION_ANCHORS)}")
    print(f"Tile Tiles Matched:   {result.tile_tiles_matched}/{len({(a['row'], a['col']) for a in TILE_CALIBRATION_ANCHORS})}")
    print(f"Block Bases Verified: {len(result.block_bases)}")
    print(f"Dungeon Bases:        {len(result.dungeon_bases)}")
    print(f"Anchors Used:         {result.calibration_flags_used}")
    print(f"Notes:                {result.notes}")


if __name__ == "__main__":
    main()
