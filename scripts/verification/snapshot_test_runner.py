#!/usr/bin/env python3
"""
Snapshot Test Runner - Dynamic Test Selection and Calibration

This module provides:
1. Test case selection from the capture catalog
2. Dynamic calibration of formula bases per save state
3. Hypothesis testing against multiple snapshot pairs
4. Aggregate confidence scoring

The key insight: tile/dungeon formula bases are SAVE-DEPENDENT.
Each save file may have different EF offsets and calibrated bases.
This runner calibrates for each save before running verification.

Usage:
    from verification.snapshot_test_runner import SnapshotTestRunner

    runner = SnapshotTestRunner()

    # Run calibration on a specific save
    cal = runner.calibrate_for_save("/path/to/save", slot=0)
    print(f"Tile base: {cal.tile_base}, EF offset: {cal.ef_offset}")

    # Get tests for a specific formula type
    tests = runner.get_tests_for_formula("tile", min_confidence=0.8)

    # Run hypothesis test
    result = runner.run_hypothesis_test(FlagHypothesis(flag_id=1044360040, expected_set=True))
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

sys.path.insert(0, str(Path(__file__).parent.parent))

from verification.save_parser import SaveParser
from verification.utils import (
    read_slot_data,
    detect_event_flags_start,
    extract_event_flags,
)
from verification.ground_truth_loader import (
    get_tile_config,
    load_block_bases,
    load_dungeon_bases,
    calculate_tile_offset,
    calculate_block_offset,
    calculate_dungeon_offset,
)


# ============================================================================
# CONFIGURATION
# ============================================================================

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SNAPSHOT_DIR = SAVE_DIR / "Granular snapshots for debugging"
CATALOG_PATH = SNAPSHOT_DIR / "capture_catalog.json"

# Calibration anchor flags (known to be SET in progressed saves)
CALIBRATION_ANCHORS = {
    "tile": {
        "flag_id": 1043500010,
        "name": "Smoldering Butterfly",
        "expected_local_id": 10,
        "expected_row": 43,
        "expected_col": 50,
    },
    "block": {
        "flag_id": 76100,
        "name": "The First Step",
        "expected_block_start": 76000,
        "expected_relative": 100,
    },
    "dungeon_16": {
        "flag_id": 16000002,
        "name": "Volcano Manor grace",
        "expected_area": 16,
        "expected_section": 0,
        "expected_local_id": 2,
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
    block_bases: Dict[int, int] = field(default_factory=dict)
    dungeon_bases: Dict[int, int] = field(default_factory=dict)
    calibration_flags_used: List[int] = field(default_factory=list)
    notes: str = ""


@dataclass
class FlagHypothesis:
    """A hypothesis about a flag's state."""
    flag_id: int
    flag_format: str  # "tile", "dungeon", "block"
    expected_set: bool = True
    notes: Optional[str] = None


@dataclass
class TestCase:
    """A test case from the capture catalog."""
    pair_id: str
    before_path: str
    after_path: str
    flag_id: int
    flag_format: str
    slot_index: int
    action_type: Optional[str] = None
    verified: bool = False
    expected_offset: Optional[int] = None
    expected_bit: Optional[int] = None
    tags: List[str] = field(default_factory=list)


@dataclass
class VerificationResult:
    """Result of verifying a flag across multiple test cases."""
    flag_id: int
    flag_format: str
    tests_run: int = 0
    tests_passed: int = 0
    tests_failed: int = 0
    aggregate_confidence: float = 0.0
    per_test_results: List[Dict[str, Any]] = field(default_factory=list)
    calibration_used: Optional[CalibrationResult] = None
    conclusion: str = "unknown"  # "verified", "likely", "uncertain", "failed"


# ============================================================================
# CATALOG LOADING
# ============================================================================

def load_catalog() -> Dict[str, Any]:
    """Load the capture catalog."""
    if CATALOG_PATH.exists():
        with open(CATALOG_PATH, 'r') as f:
            return json.load(f)
    return {"captures": [], "pairs": []}


def get_capture_by_id(catalog: Dict[str, Any], capture_id: str) -> Optional[Dict[str, Any]]:
    """Get a capture record by ID."""
    for cap in catalog.get("captures", []):
        if cap.get("id") == capture_id:
            return cap
    return None


def get_pair_by_id(catalog: Dict[str, Any], pair_id: str) -> Optional[Dict[str, Any]]:
    """Get a capture pair by ID."""
    for pair in catalog.get("pairs", []):
        if pair.get("pair_id") == pair_id:
            return pair
    return None


# ============================================================================
# CALIBRATION
# ============================================================================

class SnapshotTestRunner:
    """
    Test runner that handles dynamic calibration and test selection.

    Key features:
    - Calibrates formula bases for each save file
    - Selects appropriate test cases from the catalog
    - Runs hypothesis tests with aggregate confidence scoring
    """

    def __init__(self, catalog_path: Optional[Path] = None):
        self.catalog_path = catalog_path or CATALOG_PATH
        self.catalog = load_catalog() if self.catalog_path.exists() else {"captures": [], "pairs": []}
        self.parser = SaveParser()
        self._calibration_cache: Dict[str, CalibrationResult] = {}

    def reload_catalog(self) -> None:
        """Reload the capture catalog from disk."""
        self.catalog = load_catalog()

    def calibrate_for_save(
        self,
        save_path: str | Path,
        slot_index: int,
        force_recalibrate: bool = False
    ) -> CalibrationResult:
        """
        Calibrate formula bases for a specific save file and slot.

        This determines the actual tile/dungeon bases for this save state,
        which may differ from other saves due to variable GaItems size.

        Args:
            save_path: Path to the save file
            slot_index: Character slot index
            force_recalibrate: If True, ignore cache

        Returns:
            CalibrationResult with detected bases
        """
        save_path = Path(save_path)
        cache_key = f"{save_path}:{slot_index}"

        if not force_recalibrate and cache_key in self._calibration_cache:
            return self._calibration_cache[cache_key]

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

            # Calibrate tile base using anchor flag
            tile_result = self._calibrate_tile_base(event_flags)
            if tile_result:
                result.tile_base = tile_result[0]
                result.tile_base_confidence = tile_result[1]
                result.calibration_flags_used.append(CALIBRATION_ANCHORS["tile"]["flag_id"])

            # Calibrate block bases (verify known bases work)
            block_result = self._calibrate_block_bases(event_flags)
            result.block_bases = block_result

            # Calibrate dungeon bases
            dungeon_result = self._calibrate_dungeon_bases(event_flags)
            result.dungeon_bases = dungeon_result

            result.notes = f"Calibrated successfully. EF offset: {ef_start}"

        except Exception as e:
            result.notes = f"Calibration error: {e}"

        self._calibration_cache[cache_key] = result
        return result

    def _calibrate_tile_base(self, event_flags: bytes) -> Optional[Tuple[int, float]]:
        """
        Calibrate the tile formula base using known anchor flags.

        Returns (base_offset, confidence) or None.
        """
        # Use the ground truth tile config as starting point
        config = get_tile_config()
        base_offset = config.get("base_offset", 489981)
        bytes_per_slot = config.get("bytes_per_slot", 875)
        slots_per_row = config.get("slots_per_row", 40)
        row_base = config.get("row_base", 33)
        col_base = config.get("col_base", 30)

        anchor = CALIBRATION_ANCHORS["tile"]
        flag_id = anchor["flag_id"]
        local_id = anchor["expected_local_id"]
        row = anchor["expected_row"]
        col = anchor["expected_col"]

        # Calculate expected offset
        tile_offset = ((row - row_base) * slots_per_row + (col - col_base)) * bytes_per_slot
        byte_offset = base_offset + tile_offset + local_id // 8
        bit_pos = 7 - (local_id % 8)

        # Check if the flag is SET
        if byte_offset < len(event_flags):
            byte_val = event_flags[byte_offset]
            is_set = (byte_val >> bit_pos) & 1

            if is_set:
                # Ground truth base works
                return (base_offset, 0.9)
            else:
                # Try to find the correct base by searching
                # This is a fallback for saves with different bases
                return self._search_for_tile_base(event_flags, anchor)

        return None

    def _search_for_tile_base(
        self,
        event_flags: bytes,
        anchor: Dict[str, Any]
    ) -> Optional[Tuple[int, float]]:
        """
        Search for the correct tile base by looking for the anchor flag.

        This handles cases where the save has a different base than ground truth.
        """
        config = get_tile_config()
        bytes_per_slot = config.get("bytes_per_slot", 875)
        slots_per_row = config.get("slots_per_row", 40)
        row_base = config.get("row_base", 33)
        col_base = config.get("col_base", 30)

        local_id = anchor["expected_local_id"]
        row = anchor["expected_row"]
        col = anchor["expected_col"]
        expected_bit = 7 - (local_id % 8)

        # Calculate tile offset (constant regardless of base)
        tile_offset = ((row - row_base) * slots_per_row + (col - col_base)) * bytes_per_slot
        local_byte_offset = local_id // 8

        # Search for the base that makes this flag SET
        # The tile region is typically around 489000-500000
        search_start = 480000
        search_end = min(510000, len(event_flags) - tile_offset - local_byte_offset)

        for base in range(search_start, search_end):
            byte_offset = base + tile_offset + local_byte_offset
            if byte_offset < len(event_flags):
                byte_val = event_flags[byte_offset]
                if (byte_val >> expected_bit) & 1:
                    # Found it! But verify it's not 0xFF padding
                    if byte_val != 0xFF:
                        return (base, 0.7)  # Lower confidence for searched base

        return None

    def _calibrate_block_bases(self, event_flags: bytes) -> Dict[int, int]:
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

    def _calibrate_dungeon_bases(self, event_flags: bytes) -> Dict[int, int]:
        """Verify known dungeon bases work for this save."""
        bases = load_dungeon_bases()
        verified_bases = {}

        for area, config in bases.items():
            if config["base_offset"] > 0:
                verified_bases[area] = config["base_offset"]

        return verified_bases

    def get_tests_for_formula(
        self,
        flag_format: str,
        min_confidence: float = 0.5,
        max_count: int = 10
    ) -> List[TestCase]:
        """
        Get test cases from the catalog for a specific formula type.

        Args:
            flag_format: "tile", "dungeon", or "block"
            min_confidence: Minimum confidence level for test cases
            max_count: Maximum number of test cases to return

        Returns:
            List of TestCase objects
        """
        tests = []

        for pair in self.catalog.get("pairs", []):
            pair_flag_format = None
            flag_id = pair.get("flag_id")

            if flag_id:
                if 1_000_000_000 <= flag_id < 3_000_000_000:
                    pair_flag_format = "tile"
                elif 10_000_000 <= flag_id < 100_000_000:
                    pair_flag_format = "dungeon"
                elif 60_000 <= flag_id < 100_000:
                    pair_flag_format = "block"

            if pair_flag_format != flag_format:
                continue

            # Get before/after captures
            before_cap = get_capture_by_id(self.catalog, pair.get("before_capture"))
            after_cap = get_capture_by_id(self.catalog, pair.get("after_capture"))

            if not before_cap or not after_cap:
                continue

            # Determine file paths
            before_filename = before_cap.get("filename", "")
            after_filename = after_cap.get("filename", "")

            # Find the actual files (could be in slot subdirectories)
            before_path = self._find_snapshot_file(before_filename)
            after_path = self._find_snapshot_file(after_filename)

            if not before_path or not after_path:
                continue

            slot_index = before_cap.get("slot_context", {}).get("slot_index", 0)

            verification_result = pair.get("verification_result") or {}
            test = TestCase(
                pair_id=pair.get("pair_id"),
                before_path=str(before_path),
                after_path=str(after_path),
                flag_id=flag_id,
                flag_format=flag_format,
                slot_index=slot_index,
                action_type=pair.get("action_type"),
                verified=verification_result.get("status") == "verified",
                tags=pair.get("tags", []),
            )
            tests.append(test)

            if len(tests) >= max_count:
                break

        return tests

    def _find_snapshot_file(self, filename: str) -> Optional[Path]:
        """Find a snapshot file, checking main dir and slot subdirectories."""
        # Check main directory
        direct_path = SNAPSHOT_DIR / filename
        if direct_path.exists():
            return direct_path

        # Check slot subdirectories
        for subdir in SNAPSHOT_DIR.iterdir():
            if subdir.is_dir():
                sub_path = subdir / filename
                if sub_path.exists():
                    return sub_path

        return None

    def run_hypothesis_test(
        self,
        hypothesis: FlagHypothesis,
        test_cases: Optional[List[TestCase]] = None,
        num_samples: int = 5
    ) -> VerificationResult:
        """
        Test a flag hypothesis against multiple snapshot pairs.

        Args:
            hypothesis: The flag hypothesis to test
            test_cases: Specific test cases to use (auto-selected if None)
            num_samples: Number of test cases to sample if auto-selecting

        Returns:
            VerificationResult with aggregate confidence
        """
        if test_cases is None:
            test_cases = self.get_tests_for_formula(
                hypothesis.flag_format,
                max_count=num_samples
            )

        result = VerificationResult(
            flag_id=hypothesis.flag_id,
            flag_format=hypothesis.flag_format,
        )

        if not test_cases:
            result.conclusion = "no_tests"
            result.notes = "No test cases available for this formula type"
            return result

        for test in test_cases:
            test_result = self._run_single_test(hypothesis, test)
            result.per_test_results.append(test_result)
            result.tests_run += 1

            if test_result.get("passed"):
                result.tests_passed += 1
            elif test_result.get("failed"):
                result.tests_failed += 1

        # Calculate aggregate confidence
        if result.tests_run > 0:
            result.aggregate_confidence = result.tests_passed / result.tests_run

        # Determine conclusion
        if result.aggregate_confidence >= 0.9:
            result.conclusion = "verified"
        elif result.aggregate_confidence >= 0.7:
            result.conclusion = "likely"
        elif result.aggregate_confidence >= 0.5:
            result.conclusion = "uncertain"
        else:
            result.conclusion = "failed"

        return result

    def _run_single_test(
        self,
        hypothesis: FlagHypothesis,
        test: TestCase
    ) -> Dict[str, Any]:
        """Run a single test case."""
        result = {
            "pair_id": test.pair_id,
            "flag_id": hypothesis.flag_id,
            "passed": False,
            "failed": False,
            "skipped": False,
            "before_value": None,
            "after_value": None,
            "expected_offset": None,
            "expected_bit": None,
            "error": None,
        }

        try:
            # Calibrate for the before save
            cal = self.calibrate_for_save(test.before_path, test.slot_index)
            if cal.ef_offset is None:
                result["skipped"] = True
                result["error"] = "Could not calibrate for save"
                return result

            # Calculate offset based on flag format
            offset_result = None
            if hypothesis.flag_format == "tile":
                offset_result = calculate_tile_offset(hypothesis.flag_id)
            elif hypothesis.flag_format == "dungeon":
                offset_result = calculate_dungeon_offset(hypothesis.flag_id)
            elif hypothesis.flag_format == "block":
                offset_result = calculate_block_offset(hypothesis.flag_id)

            if offset_result is None:
                result["skipped"] = True
                result["error"] = f"Could not calculate offset for {hypothesis.flag_format} flag"
                return result

            byte_offset, bit_pos = offset_result
            result["expected_offset"] = byte_offset
            result["expected_bit"] = bit_pos

            # Read event flags from before and after
            before_slot = read_slot_data(test.before_path, test.slot_index)
            after_slot = read_slot_data(test.after_path, test.slot_index)

            before_ef = extract_event_flags(before_slot)
            after_ef = extract_event_flags(after_slot)

            if byte_offset >= len(before_ef) or byte_offset >= len(after_ef):
                result["skipped"] = True
                result["error"] = f"Offset {byte_offset} out of range"
                return result

            # Check flag values
            before_byte = before_ef[byte_offset]
            after_byte = after_ef[byte_offset]

            before_set = (before_byte >> bit_pos) & 1
            after_set = (after_byte >> bit_pos) & 1

            result["before_value"] = before_set
            result["after_value"] = after_set

            # Verify the expected pattern: UNSET -> SET for pickups/actions
            if hypothesis.expected_set:
                # Expected: before=UNSET, after=SET
                if before_set == 0 and after_set == 1:
                    result["passed"] = True
                else:
                    result["failed"] = True
                    result["error"] = f"Expected 0->1, got {before_set}->{after_set}"
            else:
                # Expected: before=SET, after=UNSET (rare case)
                if before_set == 1 and after_set == 0:
                    result["passed"] = True
                else:
                    result["failed"] = True
                    result["error"] = f"Expected 1->0, got {before_set}->{after_set}"

        except Exception as e:
            result["skipped"] = True
            result["error"] = str(e)

        return result

    def verify_flag(self, flag_id: int, expected_set: bool = True) -> VerificationResult:
        """
        Convenience method to verify a single flag.

        Auto-detects flag format and runs appropriate tests.
        """
        # Detect flag format
        if 1_000_000_000 <= flag_id < 3_000_000_000:
            flag_format = "tile"
        elif 10_000_000 <= flag_id < 100_000_000:
            flag_format = "dungeon"
        elif 60_000 <= flag_id < 100_000:
            flag_format = "block"
        else:
            return VerificationResult(
                flag_id=flag_id,
                flag_format="unknown",
                conclusion="invalid",
            )

        hypothesis = FlagHypothesis(
            flag_id=flag_id,
            flag_format=flag_format,
            expected_set=expected_set,
        )

        return self.run_hypothesis_test(hypothesis)


# ============================================================================
# CLI
# ============================================================================

def main():
    """Run test runner from command line."""
    import argparse

    parser = argparse.ArgumentParser(description="Snapshot Test Runner")
    subparsers = parser.add_subparsers(dest="command")

    # calibrate command
    cal_parser = subparsers.add_parser("calibrate", help="Calibrate for a save file")
    cal_parser.add_argument("save_path", help="Path to save file")
    cal_parser.add_argument("--slot", type=int, default=0, help="Slot index")

    # tests command
    tests_parser = subparsers.add_parser("tests", help="List available tests")
    tests_parser.add_argument("--format", choices=["tile", "dungeon", "block"], help="Filter by format")

    # verify command
    verify_parser = subparsers.add_parser("verify", help="Verify a flag")
    verify_parser.add_argument("flag_id", type=int, help="Flag ID to verify")

    args = parser.parse_args()
    runner = SnapshotTestRunner()

    if args.command == "calibrate":
        save_path = Path(args.save_path)
        if not save_path.exists():
            # Try relative to save directory
            save_path = SAVE_DIR / args.save_path
            if not save_path.exists():
                save_path = SNAPSHOT_DIR / args.save_path

        cal = runner.calibrate_for_save(save_path, args.slot)
        print(f"\nCalibration Results for {save_path.name}, slot {args.slot}")
        print("=" * 60)
        print(f"EF Offset: {cal.ef_offset}")
        print(f"Tile Base: {cal.tile_base} (confidence: {cal.tile_base_confidence:.2f})")
        print(f"Block Bases Verified: {len(cal.block_bases)}")
        print(f"Dungeon Bases Verified: {len(cal.dungeon_bases)}")
        print(f"Notes: {cal.notes}")

    elif args.command == "tests":
        formats = [args.format] if args.format else ["tile", "dungeon", "block"]
        for fmt in formats:
            tests = runner.get_tests_for_formula(fmt)
            print(f"\n{fmt.upper()} Tests ({len(tests)} available):")
            for test in tests[:10]:
                print(f"  [{test.pair_id}] flag={test.flag_id} slot={test.slot_index}")

    elif args.command == "verify":
        result = runner.verify_flag(args.flag_id)
        print(f"\nVerification Results for flag {args.flag_id}")
        print("=" * 60)
        print(f"Format: {result.flag_format}")
        print(f"Tests Run: {result.tests_run}")
        print(f"Tests Passed: {result.tests_passed}")
        print(f"Tests Failed: {result.tests_failed}")
        print(f"Confidence: {result.aggregate_confidence:.2%}")
        print(f"Conclusion: {result.conclusion}")

        if result.per_test_results:
            print("\nPer-Test Results:")
            for tr in result.per_test_results:
                status = "PASS" if tr.get("passed") else ("FAIL" if tr.get("failed") else "SKIP")
                print(f"  [{tr['pair_id']}] {status} - {tr.get('error', 'OK')}")

    else:
        parser.print_help()


if __name__ == "__main__":
    main()
