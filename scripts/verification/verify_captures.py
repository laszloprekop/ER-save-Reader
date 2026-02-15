#!/usr/bin/env python3
"""
Capture Pair Temporal Verification — uses utils.py for reliable EF detection.

Verifies before/after capture pairs from the capture catalog. For each pair,
checks that the expected event flag bit transitions 0→1 between before and after.

Two verification modes:
  1. Formula-based: Uses ground truth formulas to calculate expected offset
  2. Transition-based: Brute-force compares EF sections to find actual bit changes

Usage:
    python scripts/verification/verify_captures.py
    python scripts/verification/verify_captures.py --filter tile
    python scripts/verification/verify_captures.py --verbose --json /tmp/capture_results.json
    python scripts/verification/verify_captures.py --transitions  # Show actual EF transitions
"""

import argparse
import json
import sys
from pathlib import Path
from dataclasses import dataclass, asdict, field
from typing import Dict, List, Optional, Tuple, Any

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.utils import (
    read_slot_data,
    detect_event_flags_start,
    extract_event_flags,
)
from scripts.verification.ground_truth_loader import (
    load_block_bases,
    load_dungeon_bases,
    get_tile_config,
    calculate_block_offset,
    calculate_tile_offset,
    calculate_dungeon_offset,
    calculate_tile_offset_calibrated,
)
from scripts.verification.calibration import CalibrationService
from scripts.verification.constants import EVENT_FLAGS_SIZE

# ============================================================================
# PATHS
# ============================================================================

SNAPSHOT_DIR = Path(
    "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/"
    "Granular snapshots for debugging"
)
CATALOG_PATH = SNAPSHOT_DIR / "capture_catalog.json"

# Subdirectory search paths for captures not found in root
SUBDIRS = [
    "slot 0 Confessor",
    "slot 1 Wretch",
    "slot 6 Sam",
]


# ============================================================================
# DATA CLASSES
# ============================================================================

@dataclass
class CaptureResult:
    pair_id: str
    flag_id: int
    name: str
    formula_type: str
    slot_index: int
    byte_offset: int
    bit_position: int

    before_file: str
    before_byte: int
    before_bit: int

    after_file: str
    after_byte: int
    after_bit: int

    status: str       # "verified", "failed", "inconclusive"
    notes: str
    calibrated: bool  # Whether tile base was calibrated

    # Transition data (when --transitions mode)
    ef_transitions_set: int = 0
    ef_transitions_cleared: int = 0


# ============================================================================
# PATH RESOLUTION
# ============================================================================

def resolve_capture_path(filename: str) -> Optional[Path]:
    """
    Find the actual file path for a capture filename.

    The catalog may reference files that are:
    1. Directly in SNAPSHOT_DIR
    2. Inside a subdirectory (e.g., "slot 0 Confessor/")
    """
    # Try direct path first
    direct = SNAPSHOT_DIR / filename
    if direct.exists():
        return direct

    # Try subdirectories
    for subdir in SUBDIRS:
        sub_path = SNAPSHOT_DIR / subdir / filename
        if sub_path.exists():
            return sub_path

    return None


# ============================================================================
# FLAG OFFSET CALCULATION (with calibration support)
# ============================================================================

def calculate_flag_offset(
    flag_id: int,
    save_path: Optional[str] = None,
    slot_index: int = 0,
) -> Optional[Tuple[int, int, str, bool]]:
    """
    Calculate byte_offset, bit_position, formula_type, calibrated.

    Uses calibrated tile base when save_path is provided.
    Uses per-section dungeon bases from ground truth.
    """
    bit = 7 - (flag_id % 8)

    # Tile flags (10-digit)
    if flag_id >= 1_000_000_000:
        if save_path:
            result = calculate_tile_offset_calibrated(flag_id, save_path, slot_index)
            if result:
                return (result[0], result[1], "tile", True)

        result = calculate_tile_offset(flag_id)
        if result:
            return (result[0], result[1], "tile", False)
        return None

    # Dungeon flags (8-digit)
    if 10_000_000 <= flag_id < 100_000_000:
        result = calculate_dungeon_offset(flag_id)
        if result:
            return (result[0], result[1], "dungeon", False)
        return None

    # Block flags (5-6 digit)
    if 60_000 <= flag_id < 100_000:
        try:
            result = calculate_block_offset(flag_id)
            if result and result[0] is not None:
                return (result[0], result[1], "block", False)
        except (TypeError, ValueError):
            pass
        return None

    # Simple flags (< 60_000)
    if flag_id < 60_000:
        byte_offset = flag_id // 8
        return (byte_offset, bit, "simple", False)

    # Midrange (100_000-999_999)
    if 100_000 <= flag_id < 1_000_000:
        try:
            result = calculate_block_offset(flag_id)
            if result and result[0] is not None:
                return (result[0], result[1], "midrange", False)
        except (TypeError, ValueError):
            pass
        return None

    return None


# ============================================================================
# EF TRANSITION DETECTION
# ============================================================================

def find_ef_transitions(
    before_ef: bytes,
    after_ef: bytes,
    max_transitions: int = 200,
) -> Tuple[List[Tuple[int, int]], List[Tuple[int, int]]]:
    """
    Find all bit transitions between two EF sections.

    Returns:
        (set_transitions, cleared_transitions) — lists of (byte_offset, bit_position)
    """
    set_transitions = []
    cleared_transitions = []

    compare_len = min(len(before_ef), len(after_ef))
    total = 0

    for i in range(compare_len):
        if before_ef[i] == after_ef[i]:
            continue

        diff = before_ef[i] ^ after_ef[i]
        for bit in range(8):
            if (diff >> bit) & 1:
                b_bit = (before_ef[i] >> bit) & 1
                a_bit = (after_ef[i] >> bit) & 1
                if b_bit == 0 and a_bit == 1:
                    set_transitions.append((i, bit))
                else:
                    cleared_transitions.append((i, bit))
                total += 1
                if total >= max_transitions:
                    return set_transitions, cleared_transitions

    return set_transitions, cleared_transitions


# ============================================================================
# VERIFICATION
# ============================================================================

def _make_inconclusive(pair_id, flag_id, name, formula_type, slot_index,
                       byte_offset, bit_pos, before_filename, after_filename,
                       notes, calibrated=False):
    """Helper to build an inconclusive CaptureResult."""
    return CaptureResult(
        pair_id=pair_id, flag_id=flag_id, name=name,
        formula_type=formula_type, slot_index=slot_index,
        byte_offset=byte_offset, bit_position=bit_pos,
        before_file=before_filename, before_byte=0, before_bit=0,
        after_file=after_filename, after_byte=0, after_bit=0,
        status="inconclusive", notes=notes, calibrated=calibrated,
    )


def verify_pair(
    pair_id: str,
    before_cap: dict,
    after_cap: dict,
    show_transitions: bool = False,
) -> Optional[CaptureResult]:
    """Verify a before/after capture pair using utils.py for EF detection."""

    # Get flag ID (from pair or from captures)
    flag_id = (
        before_cap.get("poi", {}).get("flag_id")
        or after_cap.get("poi", {}).get("flag_id")
    )
    if not flag_id:
        return None

    name = before_cap.get("poi", {}).get("name", "Unknown")
    slot_index = before_cap.get("slot_context", {}).get("slot_index", 0)

    # Resolve file paths
    before_filename = before_cap.get("filename", "")
    after_filename = after_cap.get("filename", "")

    before_path = resolve_capture_path(before_filename)
    after_path = resolve_capture_path(after_filename)

    if not before_path or not after_path:
        location = calculate_flag_offset(flag_id)
        ftype = location[2] if location else "unknown"
        boff = location[0] if location else 0
        bpos = location[1] if location else 0
        missing = []
        if not before_path:
            missing.append("before")
        if not after_path:
            missing.append("after")
        return _make_inconclusive(
            pair_id, flag_id, name, ftype, slot_index, boff, bpos,
            before_filename, after_filename,
            f"File not found: {', '.join(missing)} MISSING",
        )

    # Calculate flag offset WITH calibration
    location = calculate_flag_offset(flag_id, str(before_path), slot_index)
    if not location:
        return _make_inconclusive(
            pair_id, flag_id, name, "unknown", slot_index, 0, 0,
            before_filename, after_filename,
            f"No formula for flag {flag_id}",
        )

    byte_offset, bit_pos, formula_type, calibrated = location

    # Read slot data and detect EF using utils.py (not SaveParser)
    try:
        before_slot = read_slot_data(str(before_path), slot_index)
        after_slot = read_slot_data(str(after_path), slot_index)

        before_ef_start = detect_event_flags_start(before_slot)
        after_ef_start = detect_event_flags_start(after_slot)

        if before_ef_start is None or after_ef_start is None:
            return _make_inconclusive(
                pair_id, flag_id, name, formula_type, slot_index,
                byte_offset, bit_pos, before_filename, after_filename,
                "Could not detect EF offset",
                calibrated=calibrated,
            )

        before_ef = extract_event_flags(before_slot, before_ef_start)
        after_ef = extract_event_flags(after_slot, after_ef_start)

    except Exception as e:
        return _make_inconclusive(
            pair_id, flag_id, name, formula_type, slot_index,
            byte_offset, bit_pos, before_filename, after_filename,
            f"Parse error: {e}",
            calibrated=calibrated,
        )

    # Bounds check
    if byte_offset >= len(before_ef) or byte_offset >= len(after_ef):
        return _make_inconclusive(
            pair_id, flag_id, name, formula_type, slot_index,
            byte_offset, bit_pos, before_filename, after_filename,
            f"Offset {byte_offset} out of bounds (EF len={min(len(before_ef), len(after_ef))})",
            calibrated=calibrated,
        )

    # Read actual values
    before_byte = before_ef[byte_offset]
    after_byte = after_ef[byte_offset]
    before_bit = (before_byte >> bit_pos) & 1
    after_bit = (after_byte >> bit_pos) & 1

    # Determine verdict
    if before_bit == 0 and after_bit == 1:
        status = "verified"
        notes = "0→1 transition confirmed"
    elif before_bit == 1 and after_bit == 1:
        status = "inconclusive"
        notes = "Already set in before"
    elif before_bit == 0 and after_bit == 0:
        status = "failed"
        notes = "Flag NOT set in after — formula may be wrong"
    else:
        status = "failed"
        notes = "INVERTED: 1→0"

    # Check for padding
    if before_byte == 0xFF or after_byte == 0xFF:
        status = "inconclusive"
        notes = f"Padding byte (before=0x{before_byte:02X}, after=0x{after_byte:02X})"

    # Count transitions if requested
    ef_set = 0
    ef_clr = 0
    if show_transitions:
        set_trans, clr_trans = find_ef_transitions(before_ef, after_ef)
        ef_set = len(set_trans)
        ef_clr = len(clr_trans)

    return CaptureResult(
        pair_id=pair_id,
        flag_id=flag_id,
        name=name,
        formula_type=formula_type,
        slot_index=slot_index,
        byte_offset=byte_offset,
        bit_position=bit_pos,
        before_file=before_filename,
        before_byte=before_byte,
        before_bit=before_bit,
        after_file=after_filename,
        after_byte=after_byte,
        after_bit=after_bit,
        status=status,
        notes=notes,
        calibrated=calibrated,
        ef_transitions_set=ef_set,
        ef_transitions_cleared=ef_clr,
    )


def load_and_verify(
    filter_type: Optional[str] = None,
    show_transitions: bool = False,
) -> List[CaptureResult]:
    """Load catalog and verify all pairs."""

    with open(CATALOG_PATH) as f:
        catalog = json.load(f)

    captures = catalog.get("captures", [])
    pairs = catalog.get("pairs", [])

    results = []

    for pair_info in pairs:
        pair_id = pair_info.get("pair_id", "unknown")
        before_id = pair_info.get("before_capture") or pair_info.get("before")
        after_id = pair_info.get("after_capture") or pair_info.get("after")

        # Also check for flag_id on the pair itself
        pair_flag_id = pair_info.get("flag_id")

        before_cap = next((c for c in captures if c["id"] == before_id), None)
        after_cap = next((c for c in captures if c["id"] == after_id), None)

        if not before_cap or not after_cap:
            continue

        # If pair has flag_id but captures don't, inject it
        if pair_flag_id:
            if not before_cap.get("poi", {}).get("flag_id"):
                before_cap.setdefault("poi", {})["flag_id"] = pair_flag_id
            if not after_cap.get("poi", {}).get("flag_id"):
                after_cap.setdefault("poi", {})["flag_id"] = pair_flag_id

        result = verify_pair(pair_id, before_cap, after_cap, show_transitions)
        if result is None:
            continue

        # Apply type filter
        if filter_type and result.formula_type != filter_type:
            continue

        results.append(result)

    return results


# ============================================================================
# REPORTING
# ============================================================================

def print_results(results: List[CaptureResult], verbose: bool = False,
                  show_transitions: bool = False):
    """Print verification results."""
    print(f"\n{'='*70}")
    print(f"CAPTURE PAIR VERIFICATION")
    print(f"{'='*70}")

    verified = [r for r in results if r.status == "verified"]
    failed = [r for r in results if r.status == "failed"]
    inconclusive = [r for r in results if r.status == "inconclusive"]

    print(f"\nTotal pairs: {len(results)}")
    print(f"  Verified:     {len(verified)} ({_pct(len(verified), len(results))})")
    print(f"  Failed:       {len(failed)} ({_pct(len(failed), len(results))})")
    print(f"  Inconclusive: {len(inconclusive)} ({_pct(len(inconclusive), len(results))})")

    # By formula type
    types = set(r.formula_type for r in results)
    if len(types) > 1:
        print(f"\nBy formula type:")
        for ftype in sorted(types):
            type_results = [r for r in results if r.formula_type == ftype]
            type_verified = [r for r in type_results if r.status == "verified"]
            print(f"  {ftype}: {len(type_verified)}/{len(type_results)} verified")

    # Show verified
    if verbose and verified:
        print(f"\n{'─'*70}")
        print(f"VERIFIED ({len(verified)}):")
        for r in verified:
            cal_tag = " [calibrated]" if r.calibrated else ""
            print(f"  {r.pair_id}: {r.flag_id} ({r.name}) [{r.formula_type}]{cal_tag}")
            print(f"    offset={r.byte_offset}, bit={r.bit_position}, {r.before_bit}→{r.after_bit}")
            if show_transitions and r.ef_transitions_set > 0:
                print(f"    EF transitions: {r.ef_transitions_set} SET, {r.ef_transitions_cleared} CLEARED")

    # Always show failures
    if failed:
        print(f"\n{'─'*70}")
        print(f"FAILED ({len(failed)}):")
        for r in failed:
            cal_tag = " [calibrated]" if r.calibrated else ""
            print(f"  {r.pair_id}: {r.flag_id} ({r.name}) [{r.formula_type}]{cal_tag}")
            print(f"    offset={r.byte_offset}, bit={r.bit_position}")
            print(f"    before=0x{r.before_byte:02X} bit={r.before_bit}, after=0x{r.after_byte:02X} bit={r.after_bit}")
            if show_transitions and r.ef_transitions_set > 0:
                print(f"    EF transitions: {r.ef_transitions_set} SET, {r.ef_transitions_cleared} CLEARED")
            print(f"    {r.notes}")

    # Show inconclusive reasons
    if verbose and inconclusive:
        print(f"\n{'─'*70}")
        print(f"INCONCLUSIVE ({len(inconclusive)}):")
        reasons = {}
        for r in inconclusive:
            reason = r.notes.split(" - ")[0] if " - " in r.notes else r.notes
            reasons.setdefault(reason, []).append(r)

        for reason, group in sorted(reasons.items(), key=lambda x: -len(x[1])):
            print(f"  {reason}: {len(group)} pairs")
            if len(group) <= 5:
                for r in group:
                    print(f"    {r.pair_id}: {r.flag_id} ({r.name})")


def _pct(n: int, total: int) -> str:
    if total == 0:
        return "—"
    return f"{100 * n / total:.1f}%"


# ============================================================================
# MAIN
# ============================================================================

def main():
    parser = argparse.ArgumentParser(description="Capture pair temporal verification")
    parser.add_argument("--filter", type=str, choices=["tile", "dungeon", "block", "simple", "midrange"],
                        help="Filter by formula type")
    parser.add_argument("--verbose", "-v", action="store_true", help="Show all details")
    parser.add_argument("--transitions", "-t", action="store_true",
                        help="Count EF transitions for each pair")
    parser.add_argument("--json", type=str, help="Write JSON results to file")
    args = parser.parse_args()

    if not CATALOG_PATH.exists():
        print(f"Catalog not found: {CATALOG_PATH}", file=sys.stderr)
        sys.exit(1)

    print("Loading and verifying capture pairs...")
    results = load_and_verify(
        filter_type=args.filter,
        show_transitions=args.transitions,
    )

    print_results(results, verbose=args.verbose, show_transitions=args.transitions)

    if args.json:
        json_data = [asdict(r) for r in results]
        with open(args.json, 'w') as f:
            json.dump(json_data, f, indent=2)
        print(f"\nJSON results written to {args.json}")


if __name__ == "__main__":
    main()
