"""
Event Flag Verification Framework

A systematic tool to verify event flag calculations against actual save file data,
establish ground truth for Elden Ring save file parsing, and generate reliable
offset tables for autocompletion tools.

Architecture:
    ground_truth_offsets.json  ← Single source of truth for offsets
         ↓
    ground_truth_loader.py     ← Python API to read ground_truth
         ↓
    constants.py               ← Save file structure constants only
         ↓
    utils.py                   ← Unified API combining both
         ↓
    verification scripts       ← Import from utils.py

Usage:
    # Quick flag check
    from scripts.verification.utils import load_and_check_flag
    is_set, offset, bit = load_and_check_flag("/path/to/save.sl2", slot=0, flag_id=71800)

    # Multi-slot differential (gold standard)
    from scripts.verification.utils import quick_slot_comparison
    results = quick_slot_comparison("/path/to/save.sl2", slot_progressed=0, slot_early=1, flags)

    # Direct ground truth access
    from scripts.verification.ground_truth_loader import load_block_bases, get_tile_config
    bases = load_block_bases()
    tile_config = get_tile_config()

Legacy modules (still available but use new imports when possible):
    - save_parser: Full BND4 save file parsing
    - flag_formulas: DEPRECATED - use ground_truth_loader instead
    - diff_analyzer: Before/after save comparison
    - verification_data: Data structures for verification results
"""

__version__ = "2.0.0"
__author__ = "ER-Save-Editor Project"

# Constants (save file structure only)
from .constants import (
    SLOT_0_OFFSET,
    SLOT_SIZE,
    SLOT_COUNT,
    EVENT_FLAGS_SIZE,
    EVENT_FLAGS_SEARCH_MIN,
    EVENT_FLAGS_SEARCH_MAX,
    DEFAULT_SAVE_DIR,
    DEFAULT_SAVE_FILE,
    get_slot_offset,
)

# Ground truth loader (single source of truth)
from .ground_truth_loader import (
    load_block_bases,
    load_dungeon_bases,
    get_tile_config,
    get_validation_flags,
    get_block_base,
    get_dungeon_base,
    calculate_block_offset,
    calculate_tile_offset,
    calculate_dungeon_offset,
)

# Utilities (unified API)
from .utils import (
    read_slot_data,
    detect_event_flags_start,
    extract_event_flags,
    check_flag,
    check_flag_at_offset,
    is_0xff_padding,
    is_likely_false_positive,
    multi_slot_differential,
    print_differential_results,
    load_and_check_flag,
    quick_slot_comparison,
)

# Legacy modules (backward compatibility)
from .verification_data import (
    FlagVerification,
    VerificationStatus,
    FlagCategory,
    VerificationReport,
)
from .save_parser import SaveParser, SlotData
from .diff_analyzer import DiffAnalyzer

# DEPRECATED: flag_formulas - use ground_truth_loader instead
# from .flag_formulas import FlagFormulas  # Not imported by default

__all__ = [
    # Constants
    "SLOT_0_OFFSET",
    "SLOT_SIZE",
    "SLOT_COUNT",
    "EVENT_FLAGS_SIZE",
    "EVENT_FLAGS_SEARCH_MIN",
    "EVENT_FLAGS_SEARCH_MAX",
    "DEFAULT_SAVE_DIR",
    "DEFAULT_SAVE_FILE",
    "get_slot_offset",
    # Ground truth
    "load_block_bases",
    "load_dungeon_bases",
    "get_tile_config",
    "get_validation_flags",
    "get_block_base",
    "get_dungeon_base",
    "calculate_block_offset",
    "calculate_tile_offset",
    "calculate_dungeon_offset",
    # Utilities
    "read_slot_data",
    "detect_event_flags_start",
    "extract_event_flags",
    "check_flag",
    "check_flag_at_offset",
    "is_0xff_padding",
    "is_likely_false_positive",
    "multi_slot_differential",
    "print_differential_results",
    "load_and_check_flag",
    "quick_slot_comparison",
    # Legacy
    "FlagVerification",
    "VerificationStatus",
    "FlagCategory",
    "VerificationReport",
    "SaveParser",
    "SlotData",
    "DiffAnalyzer",
]
