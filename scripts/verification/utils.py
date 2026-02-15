"""
Verification Utilities - Shared functions for all verification scripts.

This module provides the common functions needed by verification scripts:
- Save file reading
- Event flags extraction and detection
- Flag checking with automatic formula selection
- Multi-slot differential analysis

All functions use ground_truth_loader for offset calculations and
constants.py for save file structure.

Usage:
    from scripts.verification.utils import (
        read_slot_data,
        detect_event_flags_start,
        extract_event_flags,
        check_flag,
    )

    slot_data = read_slot_data("/path/to/save", slot_index=0)
    ef_start = detect_event_flags_start(slot_data)
    event_flags = extract_event_flags(slot_data)
    is_set, offset, bit = check_flag(event_flags, 71800)
"""

from __future__ import annotations

from pathlib import Path
from typing import Optional, Tuple, List, Dict, Any, Union

from .constants import (
    SLOT_0_OFFSET,
    SLOT_SIZE,
    SLOT_COUNT,
    EVENT_FLAGS_SIZE,
    EVENT_FLAGS_SEARCH_MIN,
    EVENT_FLAGS_SEARCH_MAX,
    DEFAULT_SAVE_FILE,
)
from .ground_truth_loader import (
    get_validation_flags,
    calculate_block_offset,
    calculate_tile_offset,
    calculate_dungeon_offset,
)


# ============================================================================
# SAVE FILE READING
# ============================================================================

def read_slot_data(save_path: str | Path, slot_index: int) -> bytes:
    """
    Read raw slot data from a save file.

    Args:
        save_path: Path to save file (ER0000.sl2)
        slot_index: Character slot index (0-9)

    Returns:
        Raw bytes of the slot data

    Raises:
        ValueError: If slot_index is out of range
        FileNotFoundError: If save file doesn't exist
    """
    if not 0 <= slot_index < SLOT_COUNT:
        raise ValueError(f"Slot index must be 0-{SLOT_COUNT - 1}, got {slot_index}")

    save_path = Path(save_path)
    slot_offset = SLOT_0_OFFSET + slot_index * SLOT_SIZE

    with open(save_path, 'rb') as f:
        f.seek(slot_offset)
        return f.read(SLOT_SIZE)


def detect_event_flags_start(slot_data: bytes) -> Optional[int]:
    """
    Find event flags section offset using validation flag patterns.

    Uses get_validation_flags() from ground_truth_loader - NOT hardcoded values.
    Searches for the offset where ALL validation flags match, rejecting 0xFF
    false positives.

    Args:
        slot_data: Raw slot data bytes

    Returns:
        Offset within slot_data where event flags section starts,
        or None if not found
    """
    validation_flags = get_validation_flags()

    best_offset = None
    best_score = 0
    max_search = min(EVENT_FLAGS_SEARCH_MAX, len(slot_data) - EVENT_FLAGS_SIZE)

    for test_offset in range(EVENT_FLAGS_SEARCH_MIN, max_search):
        score = 0
        has_0xff = False
        for flag_id, (rel_offset, bit, name) in validation_flags.items():
            abs_pos = test_offset + rel_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val == 0xFF:
                    has_0xff = True
                    break
                if (byte_val >> bit) & 1:
                    score += 1

        # Skip candidates where any validation byte is 0xFF (padding)
        if has_0xff:
            continue

        if score > best_score:
            best_score = score
            best_offset = test_offset

            if best_score == len(validation_flags):
                # Perfect match - all validation flags found
                return test_offset

    # Return best match if we got at least 2 flags
    if best_score >= 2:
        return best_offset

    return None


def detect_event_flags_start_robust(
    slot_data: bytes,
    hint_offset: Optional[int] = None,
) -> Optional[int]:
    """
    Robust EF detection with structural validation and optional hint.

    Improvements over detect_event_flags_start():
    1. Searches entire range (doesn't stop at first 4/4 match)
    2. Rejects 0xFF padding candidates
    3. Validates structurally using GaItems count
    4. If hint_offset provided, prefers candidates near it (for timeline stability)

    Args:
        slot_data: Raw slot data bytes
        hint_offset: Optional previous EF offset for stability

    Returns:
        Offset within slot_data where event flags section starts,
        or None if not found
    """
    import struct

    validation_flags = get_validation_flags()
    max_search = min(EVENT_FLAGS_SEARCH_MAX, len(slot_data) - EVENT_FLAGS_SIZE)

    # Collect ALL candidates with perfect or near-perfect scores
    candidates = []

    for test_offset in range(EVENT_FLAGS_SEARCH_MIN, max_search):
        score = 0
        has_0xff = False

        for flag_id, (rel_offset, bit, name) in validation_flags.items():
            abs_pos = test_offset + rel_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val == 0xFF:
                    has_0xff = True
                    break
                if (byte_val >> bit) & 1:
                    score += 1

        if has_0xff:
            continue

        if score >= len(validation_flags):
            candidates.append((test_offset, score))

    if not candidates:
        # Fall back to basic detection
        return detect_event_flags_start(slot_data)

    # If only one perfect candidate, use it
    if len(candidates) == 1:
        return candidates[0][0]

    # Multiple candidates — use structural validation
    # Read GaItems count to estimate expected EF position
    ga_items_offset = 0x20  # FIXED_HEADER_SIZE
    if ga_items_offset + 4 <= len(slot_data):
        ga_count = struct.unpack_from('<I', slot_data, ga_items_offset)[0]
        if ga_count > 0x1400:
            ga_count = 0  # Sanity check
        ga_items_end = ga_items_offset + 8 + ga_count * 48

        # EF should be ~170K-200K after GaItems end (intermediate sections)
        expected_ef_min = ga_items_end + 170_000
        expected_ef_max = ga_items_end + 200_000

        # Filter candidates within structural range
        structural_candidates = [
            (off, score) for off, score in candidates
            if expected_ef_min <= off <= expected_ef_max
        ]

        if structural_candidates:
            # If hint provided, prefer closest to hint
            if hint_offset is not None:
                structural_candidates.sort(key=lambda x: abs(x[0] - hint_offset))
            return structural_candidates[0][0]

    # No structural match — use hint proximity if available
    if hint_offset is not None:
        candidates.sort(key=lambda x: abs(x[0] - hint_offset))
        return candidates[0][0]

    # Fall back to highest offset (heuristic: real EF tends to be higher)
    candidates.sort(key=lambda x: -x[0])
    return candidates[0][0]


def extract_event_flags(slot_data: bytes, ef_start: Optional[int] = None) -> bytes:
    """
    Extract event flags section from slot data.

    Args:
        slot_data: Raw slot data bytes
        ef_start: Event flags start offset (auto-detected if None)

    Returns:
        Event flags section bytes

    Raises:
        ValueError: If event flags section cannot be found
    """
    if ef_start is None:
        ef_start = detect_event_flags_start(slot_data)

    if ef_start is None:
        raise ValueError("Could not detect event flags offset")

    ef_end = ef_start + EVENT_FLAGS_SIZE
    if ef_end <= len(slot_data):
        return slot_data[ef_start:ef_end]
    else:
        return slot_data[ef_start:]


# ============================================================================
# FLAG CHECKING
# ============================================================================

def check_flag(event_flags: bytes, flag_id: int) -> Tuple[bool, int, int]:
    """
    Check if a flag is set using ground_truth formulas.

    Automatically selects the appropriate formula based on flag_id:
    - 10-digit flags (1XXXXXXXXX): Tile formula
    - 8-digit flags (10XXXXXX-39XXXXXX): Dungeon formula
    - 5-6 digit flags: Block formula

    Args:
        event_flags: Event flags section bytes
        flag_id: The event flag ID to check

    Returns:
        Tuple of (is_set, byte_offset, bit_position)

    Raises:
        ValueError: If no formula applies to this flag
    """
    result = None

    # Determine formula type and calculate offset
    if flag_id >= 1_000_000_000:
        # 10-digit tile flag
        result = calculate_tile_offset(flag_id)
    elif 10_000_000 <= flag_id < 100_000_000:
        # 8-digit dungeon flag
        result = calculate_dungeon_offset(flag_id)
    else:
        # 5-6 digit block flag
        result = calculate_block_offset(flag_id)

    if result is None:
        raise ValueError(f"No formula found for flag {flag_id}")

    byte_offset, bit_position = result

    if byte_offset >= len(event_flags):
        return (False, byte_offset, bit_position)

    byte_val = event_flags[byte_offset]
    is_set = (byte_val >> bit_position) & 1 == 1

    return (is_set, byte_offset, bit_position)


def check_flag_at_offset(event_flags: bytes, byte_offset: int, bit_position: int) -> bool:
    """
    Check if a flag is set at a specific offset (bypassing formula lookup).

    Args:
        event_flags: Event flags section bytes
        byte_offset: Byte offset within event_flags
        bit_position: Bit position (0-7)

    Returns:
        True if flag is set, False otherwise
    """
    if byte_offset >= len(event_flags):
        return False
    return (event_flags[byte_offset] >> bit_position) & 1 == 1


# ============================================================================
# FALSE POSITIVE DETECTION
# ============================================================================

def is_0xff_padding(event_flags: bytes, offset: int, window: int = 4) -> bool:
    """
    Check if a region is 0xFF padding (false positive indicator).

    0xFF padding causes false positives because all bits read as SET.
    This function checks if the target byte and surrounding region
    are all 0xFF.

    Args:
        event_flags: Event flags section bytes
        offset: Byte offset to check
        window: Number of bytes on each side to check

    Returns:
        True if the region is likely 0xFF padding
    """
    start = max(0, offset - window)
    end = min(len(event_flags), offset + window + 1)
    region = event_flags[start:end]

    return all(b == 0xFF for b in region)


def is_likely_false_positive(event_flags: bytes, offset: int, bit: int) -> bool:
    """
    Check if a SET flag is likely a false positive.

    A SET flag is suspicious if:
    - The byte is 0xFF (all bits set)
    - Surrounding region is also 0xFF (padding)

    Args:
        event_flags: Event flags section bytes
        offset: Byte offset
        bit: Bit position

    Returns:
        True if likely false positive
    """
    if offset >= len(event_flags):
        return True

    byte_val = event_flags[offset]

    # If byte is not 0xFF, flag is probably real
    if byte_val != 0xFF:
        return False

    # Byte is 0xFF - check if it's padding
    return is_0xff_padding(event_flags, offset)


# ============================================================================
# MULTI-SLOT DIFFERENTIAL ANALYSIS
# ============================================================================

def multi_slot_differential(
    ef_progressed: bytes,
    ef_early: bytes,
    flags_to_check: List[Tuple[int, str]],
) -> List[Dict[str, Any]]:
    """
    Gold standard verification: compare progressed vs early-game slots.

    For each flag, checks:
    - Is it SET in the progressed slot (S0)?
    - Is it UNSET in the early-game slot (S1)?
    - A valid differential means SET in S0 and UNSET in S1

    Args:
        ef_progressed: Event flags from progressed character (e.g., Slot 0)
        ef_early: Event flags from early-game character (e.g., Slot 1)
        flags_to_check: List of (flag_id, name) tuples

    Returns:
        List of result dicts with keys:
        - flag_id: The flag ID
        - name: Flag name
        - s0_set: Whether flag is SET in progressed slot
        - s1_set: Whether flag is SET in early-game slot
        - status: "valid_differential", "both_set", "both_unset", "inverted", "error"
        - offset: Byte offset
        - bit: Bit position
    """
    results = []

    for flag_id, name in flags_to_check:
        try:
            s0_set, offset, bit = check_flag(ef_progressed, flag_id)
            s1_set, _, _ = check_flag(ef_early, flag_id)

            # Determine status
            if s0_set and not s1_set:
                status = "valid_differential"
            elif s0_set and s1_set:
                status = "both_set"
            elif not s0_set and not s1_set:
                status = "both_unset"
            else:  # not s0_set and s1_set
                status = "inverted"

            # Check for false positives
            if s0_set and is_likely_false_positive(ef_progressed, offset, bit):
                status = "likely_false_positive"

            results.append({
                "flag_id": flag_id,
                "name": name,
                "s0_set": s0_set,
                "s1_set": s1_set,
                "status": status,
                "offset": offset,
                "bit": bit,
            })

        except ValueError as e:
            results.append({
                "flag_id": flag_id,
                "name": name,
                "s0_set": None,
                "s1_set": None,
                "status": "error",
                "error": str(e),
                "offset": None,
                "bit": None,
            })

    return results


def print_differential_results(results: List[Dict[str, Any]], verbose: bool = False):
    """
    Print multi-slot differential results in a readable format.

    Args:
        results: Results from multi_slot_differential()
        verbose: If True, print all flags; if False, only print mismatches
    """
    valid = [r for r in results if r["status"] == "valid_differential"]
    both_set = [r for r in results if r["status"] == "both_set"]
    both_unset = [r for r in results if r["status"] == "both_unset"]
    inverted = [r for r in results if r["status"] == "inverted"]
    false_pos = [r for r in results if r["status"] == "likely_false_positive"]
    errors = [r for r in results if r["status"] == "error"]

    print(f"Multi-Slot Differential Results")
    print(f"=" * 60)
    print(f"Valid differentials (SET in S0, UNSET in S1): {len(valid)}")
    print(f"Both SET (S0 and S1): {len(both_set)}")
    print(f"Both UNSET (S0 and S1): {len(both_unset)}")
    print(f"Inverted (SET in S1, UNSET in S0): {len(inverted)}")
    print(f"Likely false positives: {len(false_pos)}")
    print(f"Errors: {len(errors)}")
    print()

    if verbose or inverted:
        if inverted:
            print("INVERTED (potential formula errors):")
            print("-" * 40)
            for r in inverted:
                print(f"  {r['flag_id']}: {r['name']}")
                print(f"    S0={r['s0_set']}, S1={r['s1_set']}, offset={r['offset']}")
            print()

    if verbose or false_pos:
        if false_pos:
            print("LIKELY FALSE POSITIVES (0xFF padding):")
            print("-" * 40)
            for r in false_pos:
                print(f"  {r['flag_id']}: {r['name']} at offset {r['offset']}")
            print()

    if verbose:
        print("VALID DIFFERENTIALS:")
        print("-" * 40)
        for r in valid:
            print(f"  {r['flag_id']}: {r['name']} [OK]")


# ============================================================================
# CONVENIENCE FUNCTIONS
# ============================================================================

def load_and_check_flag(
    save_path: str | Path,
    slot_index: int,
    flag_id: int
) -> Tuple[bool, int, int]:
    """
    Convenience function to load a save and check a single flag.

    Args:
        save_path: Path to save file
        slot_index: Slot index (0-9)
        flag_id: Event flag ID

    Returns:
        Tuple of (is_set, byte_offset, bit_position)
    """
    slot_data = read_slot_data(save_path, slot_index)
    event_flags = extract_event_flags(slot_data)
    return check_flag(event_flags, flag_id)


def quick_slot_comparison(
    save_path: str | Path,
    slot_progressed: int,
    slot_early: int,
    flags: List[Tuple[int, str]]
) -> List[Dict[str, Any]]:
    """
    Convenience function for quick slot comparison.

    Args:
        save_path: Path to save file
        slot_progressed: Index of progressed character slot
        slot_early: Index of early-game character slot
        flags: List of (flag_id, name) tuples

    Returns:
        Results from multi_slot_differential()
    """
    s0_data = read_slot_data(save_path, slot_progressed)
    s1_data = read_slot_data(save_path, slot_early)

    ef0 = extract_event_flags(s0_data)
    ef1 = extract_event_flags(s1_data)

    return multi_slot_differential(ef0, ef1, flags)
