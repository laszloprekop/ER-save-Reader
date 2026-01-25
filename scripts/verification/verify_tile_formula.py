#!/usr/bin/env python3
"""
Verify the tile formula by checking for ANY set tile flags.

Uses shared modules from the verification framework:
- constants.py for save file structure
- utils.py for slot data reading and EF detection
- ground_truth_loader.py for tile formula configuration
"""

from pathlib import Path
import sys

# Add parent to path for imports when run directly
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from scripts.verification.constants import (
    DEFAULT_SAVE_DIR,
    EVENT_FLAGS_SIZE,
)
from scripts.verification.utils import (
    read_slot_data,
    detect_event_flags_start,
    extract_event_flags,
)
from scripts.verification.ground_truth_loader import get_tile_config


def main():
    print("=" * 80)
    print("VERIFY TILE FORMULA - SCAN FOR SET TILE FLAGS")
    print("=" * 80)

    # Get tile formula config from ground truth
    tile_config = get_tile_config()

    TILE_BASE = tile_config.get("base_offset", 485330)
    BYTES_PER_SLOT = tile_config.get("bytes_per_slot", 875)
    SLOTS_PER_ROW = tile_config.get("slots_per_row", 40)
    ROW_BASE = tile_config.get("row_base", 33)
    COL_BASE = tile_config.get("col_base", 30)

    print(f"\nTile formula config (from ground_truth):")
    print(f"  base_offset: {TILE_BASE}")
    print(f"  bytes_per_slot: {BYTES_PER_SLOT}")
    print(f"  slots_per_row: {SLOTS_PER_ROW}")
    print(f"  row_base: {ROW_BASE}")
    print(f"  col_base: {COL_BASE}")
    print(f"  status: {tile_config.get('status', 'unknown')}")

    # Load save file
    save_file = DEFAULT_SAVE_DIR / "ER0000-backup-2026-01-11.sl2"
    if not save_file.exists():
        save_file = DEFAULT_SAVE_DIR / "ER0000.sl2"

    print(f"\nLoading: {save_file}")

    slot0_data = read_slot_data(save_file, 0)
    ef_start = detect_event_flags_start(slot0_data)

    if ef_start is None:
        print("ERROR: Could not detect event flags offset!")
        return

    ef_data = extract_event_flags(slot0_data, ef_start)

    print(f"\nEventFlags start: 0x{ef_start:X}")
    print(f"EventFlags size: {len(ef_data):,} bytes")

    # Tile section starts at TILE_BASE
    print(f"\nTile section starts at byte: {TILE_BASE}")

    # Check bytes at the tile section
    print(f"\nFirst 100 bytes of tile section:")
    non_zero = 0
    for i in range(100):
        if TILE_BASE + i < len(ef_data):
            val = ef_data[TILE_BASE + i]
            if val != 0:
                non_zero += 1
                print(f"  Byte {TILE_BASE + i}: 0x{val:02X} ({val:08b})")

    print(f"\nNon-zero bytes in first 100: {non_zero}")

    # Scan entire tile section for non-zero bytes
    print(f"\nScanning tile section for non-zero bytes...")

    # Max slot = (54 - ROW_BASE) * SLOTS_PER_ROW + (58 - COL_BASE)
    max_slot = (54 - ROW_BASE) * SLOTS_PER_ROW + (58 - COL_BASE)
    tile_section_end = TILE_BASE + max_slot * BYTES_PER_SLOT + BYTES_PER_SLOT

    print(f"Tile section range: {TILE_BASE} to {tile_section_end}")

    non_zero_bytes = []
    for offset in range(TILE_BASE, min(tile_section_end, len(ef_data))):
        val = ef_data[offset]
        if val != 0:
            rel_offset = offset - TILE_BASE
            slot_num = rel_offset // BYTES_PER_SLOT
            local_byte = rel_offset % BYTES_PER_SLOT

            row = slot_num // SLOTS_PER_ROW + ROW_BASE
            col = slot_num % SLOTS_PER_ROW + COL_BASE

            non_zero_bytes.append((offset, val, row, col, local_byte))

    print(f"\nTotal non-zero bytes in tile section: {len(non_zero_bytes)}")

    if non_zero_bytes:
        print(f"\nFirst 20 non-zero bytes:")
        for offset, val, row, col, local_byte in non_zero_bytes[:20]:
            print(f"  Byte {offset}: 0x{val:02X} - Tile ({row},{col}), local byte {local_byte}")

        tiles_with_data = {}
        for offset, val, row, col, local_byte in non_zero_bytes:
            tile_key = (row, col)
            if tile_key not in tiles_with_data:
                tiles_with_data[tile_key] = 0
            tiles_with_data[tile_key] += bin(val).count('1')

        print(f"\nTiles with flags SET (showing top 20):")
        sorted_tiles = sorted(tiles_with_data.items(), key=lambda x: -x[1])
        for (row, col), count in sorted_tiles[:20]:
            print(f"  Tile ({row},{col}): {count} flags SET")
    else:
        print("\n⚠ NO non-zero bytes found in tile section!")
        print("This suggests the tile formula base offset may be wrong")


if __name__ == "__main__":
    main()
