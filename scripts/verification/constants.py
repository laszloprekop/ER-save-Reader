"""
Verification Constants - Save file structure magic numbers.

This file contains ONLY save file structure constants that are NOT stored
in ground_truth_offsets.json.

For validation flags, block bases, and formula offsets, use ground_truth_loader.py
which reads from the single source of truth: ground_truth_offsets.json

DO NOT ADD:
- VALIDATION_FLAGS → use ground_truth_loader.get_validation_flags()
- Block bases → use ground_truth_loader.load_block_bases()
- Tile config → use ground_truth_loader.get_tile_config()
- Dungeon bases → use ground_truth_loader.load_dungeon_bases()
"""

from pathlib import Path

# ============================================================================
# SAVE FILE STRUCTURE
# These values describe the binary format of Elden Ring save files (.sl2)
# and are NOT stored in ground_truth_offsets.json
# ============================================================================

# Header and slot layout
SLOT_0_OFFSET = 0x310        # Offset to first character slot (after BND4 header)
SLOT_SIZE = 0x280010         # Size of each character slot in bytes (2,621,456)
SLOT_COUNT = 10              # Maximum number of character slots

# Event flags section
EVENT_FLAGS_SIZE = 0x1BF99F  # Size of event flags section (1,833,375 bytes)
EVENT_FLAGS_SEARCH_MIN = 0x10000  # Minimum offset to search for EF start
EVENT_FLAGS_SEARCH_MAX = 0x20000  # Maximum offset to search for EF start

# GaItems structure (inventory)
GAITEM_SIZE = 48             # Each GaItem entry is 48 bytes
GAITEM_MAX_COUNT = 0x1400    # Maximum 5,120 entries
GAITEM_HEADER_SIZE = 8       # 4-byte count + 4-byte padding

# BND4 container format
BND4_HEADER_SIZE = 0x40      # BND4 header before file entries
BND4_ENTRY_SIZE = 0x20       # Each BND4 file entry is 32 bytes
BND4_ENTRY_OFFSET_POS = 0x10 # Offset within entry for data offset
SLOT_CHECKSUM_SIZE = 16      # Checksum header before each slot

# ============================================================================
# FILE PATHS
# Default paths for save files and resources
# ============================================================================

DEFAULT_SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
DEFAULT_SAVE_FILE = DEFAULT_SAVE_DIR / "ER0000.sl2"

DECOMPILED_FILES_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files")


# ============================================================================
# HELPER FUNCTIONS
# ============================================================================

def get_slot_offset(slot_index: int) -> int:
    """
    Calculate the absolute offset for a character slot.

    Note: This is a simplified calculation. For accurate offsets,
    use save_parser.py which reads from BND4 entries.

    Args:
        slot_index: Slot index (0-9)

    Returns:
        Absolute byte offset in save file
    """
    if not 0 <= slot_index < SLOT_COUNT:
        raise ValueError(f"Slot index must be 0-{SLOT_COUNT - 1}, got {slot_index}")
    return SLOT_0_OFFSET + slot_index * SLOT_SIZE
