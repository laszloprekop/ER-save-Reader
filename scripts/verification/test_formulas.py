#!/usr/bin/env python3
"""
Formula Validation Test Suite

Ensures all verified event flag formulas produce correct offsets.
Run this after any changes to flag_formulas.py to catch regressions.

Usage:
    python -m verification.test_formulas
    python scripts/verification/test_formulas.py
    python scripts/verification/test_formulas.py --save /path/to/ER0000.sl2
"""
from __future__ import annotations

import sys
from pathlib import Path
from dataclasses import dataclass
from typing import List, Tuple, Optional, Dict

# Add scripts directory to path
script_dir = Path(__file__).parent.parent
sys.path.insert(0, str(script_dir))

from verification.archive.flag_formulas import FlagFormulas
from verification.save_parser import SaveParser, SlotData


# Default save file location
DEFAULT_SAVE_PATH = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2")


@dataclass
class SlotInfo:
    """Character slot identification info."""
    slot_index: int
    character_name: str
    flag_set: bool = False


@dataclass
class TestCase:
    """A single test case for formula validation."""
    flag_id: int
    expected_byte_offset: int
    expected_bit_position: int
    name: str
    formula_type: str  # 'block', 'tile', 'dungeon'
    notes: str = ""


# ============================================================================
# ANCHOR FLAGS - 100% reliable validation points
# ============================================================================
ANCHOR_FLAGS: List[TestCase] = [
    # Tutorial Graces (71xxx) - verified from validation flag detection
    TestCase(71800, 2725, 7, "Cave of Knowledge", "block", "Tutorial grace"),
    TestCase(71801, 2725, 6, "Stranded Graveyard", "block", "Tutorial grace"),

    # World Graces (76xxx) - verified from validation flag detection
    TestCase(76100, 3262, 3, "The First Step", "block", "First world grace"),
    TestCase(76101, 3262, 2, "Church of Elleh", "block", "Early world grace"),
]


# ============================================================================
# VERIFIED BLOCK FORMULA TESTS
# ============================================================================
BLOCK_FORMULA_TESTS: List[TestCase] = [
    # 60xxx Progression - base=1260 (0x4ec from common.emevd.js)
    TestCase(60100, 1272, 3, "Crafting Kit", "block", "60xxx base=1260"),
    TestCase(60130, 1276, 5, "Whetstone Knife", "block", "60xxx base=1260"),
    TestCase(60220, 1287, 3, "Furled Finger", "block", "60xxx base=1260"),

    # 62xxx Map Fragments - base=1500 (0x5dc), verified via 6 timeline diffs
    TestCase(62174, 1521, 1, "Ailing Village Map", "block", "62xxx base=1500"),

    # 67xxx Cookbooks - base=1764 (0x6e4 from common.emevd.js, shop stock flags)
    TestCase(67020, 1766, 3, "Missionary's Cookbook [4]", "block", "67xxx base=1764"),

    # 71xxx Tutorial Graces - base=2625, same as anchors
    TestCase(71800, 2725, 7, "Cave of Knowledge", "block", "71xxx base=2625"),
    TestCase(71801, 2725, 6, "Stranded Graveyard", "block", "71xxx base=2625"),

    # 73xxx Dungeon Graces - base=2664, verified via 13/13 slot comparison
    TestCase(73100, 2676, 3, "Impaler's Catacombs", "block", "73xxx base=2664"),
    TestCase(73110, 2677, 1, "Stormfoot Catacombs", "block", "73xxx base=2664"),

    # 76xxx World Graces - base=3250, same as anchors
    TestCase(76100, 3262, 3, "The First Step", "block", "76xxx base=3250"),
    TestCase(76101, 3262, 2, "Church of Elleh", "block", "76xxx base=3250"),
]


# ============================================================================
# VERIFIED TILE FORMULA TESTS
# ============================================================================
TILE_FORMULA_TESTS: List[TestCase] = [
    # Verified from granular snapshot diff (Smoldering Butterfly pickup)
    TestCase(1043500010, 852831, 5, "Smoldering Butterfly", "tile",
             "row=43, col=50, local=10, verified 2026-01-12"),
]


# ============================================================================
# VERIFIED DUNGEON FORMULA TESTS
# ============================================================================
DUNGEON_FORMULA_TESTS: List[TestCase] = [
    # Area 30 (Catacombs) - base=27411, section_size=1125
    # Formula: byte = 27411 + section * 1125 + local_id // 8
    TestCase(30000800, 27511, 7, "Erdtree Burial Watchdog (Impaler's)", "dungeon",
             "area=30, section=0, local=800"),
    TestCase(30010800, 28636, 7, "Erdtree Burial Watchdog (Stormfoot)", "dungeon",
             "area=30, section=1, local=800"),

    # Area 31 (Caves) - base=28634, section_size=1125
    TestCase(31000800, 28734, 7, "Beastman of Farum Azula (Groveside)", "dungeon",
             "area=31, section=0, local=800"),
    TestCase(31050800, 34359, 7, "Cleanrot Knight (Stillwater)", "dungeon",
             "area=31, section=5, local=800"),

    # Area 32 (Tunnels) - base=31577, section_size=1125
    TestCase(32000800, 31677, 7, "Stonedigger Troll (Limgrave)", "dungeon",
             "area=32, section=0, local=800"),
    TestCase(32010800, 32802, 7, "Crystalian (Raya Lucaria)", "dungeon",
             "area=32, section=1, local=800"),
]


def load_save_slots(save_path: Optional[Path]) -> Optional[List[SlotData]]:
    """Load save file and return slot data."""
    if save_path is None:
        save_path = DEFAULT_SAVE_PATH

    if not save_path.exists():
        return None

    try:
        parser = SaveParser()
        save = parser.parse(save_path)
        return save.slots
    except Exception as e:
        print(f"Warning: Could not load save file: {e}")
        return None


def check_flag_in_slots(
    slots: List[SlotData],
    flag_id: int,
    parser: SaveParser
) -> List[SlotInfo]:
    """Check which slots have a flag set and return slot info."""
    results = []
    for slot in slots:
        is_set, _ = parser.check_flag(slot.event_flags, flag_id)
        results.append(SlotInfo(
            slot_index=slot.slot_index,
            character_name=slot.character_name or f"Slot {slot.slot_index}",
            flag_set=is_set
        ))
    return results


def format_slot_status(slot_infos: List[SlotInfo], max_name_len: int = 12) -> str:
    """Format slot status as a compact string."""
    parts = []
    for info in slot_infos:
        name = info.character_name[:max_name_len]
        marker = "✓" if info.flag_set else "·"
        parts.append(f"{marker}")
    return " ".join(parts)


def run_tests(
    verbose: bool = True,
    save_path: Optional[Path] = None
) -> Tuple[int, int, List[str]]:
    """
    Run all formula validation tests.

    Args:
        verbose: Print detailed output
        save_path: Optional path to save file for slot verification

    Returns:
        Tuple of (passed_count, failed_count, failure_messages)
    """
    formulas = FlagFormulas()
    parser = SaveParser()
    slots = load_save_slots(save_path)

    all_tests = (
        ANCHOR_FLAGS +
        BLOCK_FORMULA_TESTS +
        TILE_FORMULA_TESTS +
        DUNGEON_FORMULA_TESTS
    )

    # Remove duplicates (anchors appear in both lists)
    seen = set()
    unique_tests = []
    for test in all_tests:
        key = (test.flag_id, test.expected_byte_offset, test.expected_bit_position)
        if key not in seen:
            seen.add(key)
            unique_tests.append(test)

    passed = 0
    failed = 0
    failures = []

    if verbose:
        print("=" * 90)
        print("EVENT FLAG FORMULA VALIDATION TEST SUITE")
        print("=" * 90)

        # Print slot legend if save file loaded
        if slots:
            print()
            print("Character Slots:")
            for slot in slots:
                name = slot.character_name or f"Slot {slot.slot_index}"
                print(f"  S{slot.slot_index}: {name}")
            print()
            print("Flag Status: ✓ = set, · = not set")
        print()

    for test in unique_tests:
        results = formulas.calculate_offset(test.flag_id)

        # Find the matching result
        result = None
        if test.formula_type == "block" and "block" in results:
            result = results["block"]
        elif test.formula_type == "tile" and "tile" in results:
            result = results["tile"]
        elif test.formula_type == "dungeon" and "dungeon" in results:
            result = results["dungeon"]

        if result is None or not result.is_valid:
            failed += 1
            msg = f"FAIL: {test.flag_id} ({test.name}) - No valid {test.formula_type} result"
            failures.append(msg)
            if verbose:
                print(f"  ✗ {msg}")
            continue

        byte_ok = result.byte_offset == test.expected_byte_offset
        bit_ok = result.bit_position == test.expected_bit_position

        # Check slots if available
        slot_status = ""
        if slots:
            slot_infos = check_flag_in_slots(slots, test.flag_id, parser)
            slot_status = f" [{format_slot_status(slot_infos)}]"

        if byte_ok and bit_ok:
            passed += 1
            if verbose:
                print(f"  ✓ {test.flag_id} ({test.name}): byte={result.byte_offset}, bit={result.bit_position}{slot_status}")
        else:
            failed += 1
            msg = (f"FAIL: {test.flag_id} ({test.name}) - "
                   f"Expected byte={test.expected_byte_offset}, bit={test.expected_bit_position}, "
                   f"Got byte={result.byte_offset}, bit={result.bit_position}")
            failures.append(msg)
            if verbose:
                print(f"  ✗ {msg}{slot_status}")

    if verbose:
        print()
        print("-" * 90)
        print(f"Results: {passed} passed, {failed} failed, {len(unique_tests)} total")
        if slots:
            print(f"Save file: {save_path or DEFAULT_SAVE_PATH}")
        if failures:
            print()
            print("FAILURES:")
            for f in failures:
                print(f"  {f}")

    return passed, failed, failures


def validate_anchor_flags() -> bool:
    """
    Quick validation using only anchor flags.
    Use this for fast sanity checks.
    """
    formulas = FlagFormulas()

    for test in ANCHOR_FLAGS:
        results = formulas.calculate_offset(test.flag_id)
        if "block" not in results or not results["block"].is_valid:
            return False
        result = results["block"]
        if result.byte_offset != test.expected_byte_offset:
            return False
        if result.bit_position != test.expected_bit_position:
            return False

    return True


def main():
    """Run the test suite and exit with appropriate code."""
    import argparse

    parser = argparse.ArgumentParser(description="Validate event flag formulas")
    parser.add_argument("--quiet", "-q", action="store_true",
                        help="Only show failures")
    parser.add_argument("--anchors-only", action="store_true",
                        help="Only test anchor flags (fast)")
    parser.add_argument("--save", type=Path,
                        help="Path to save file for slot verification (default: uses standard location)")
    parser.add_argument("--no-save", action="store_true",
                        help="Skip save file loading entirely")
    args = parser.parse_args()

    if args.anchors_only:
        if validate_anchor_flags():
            print("✓ All anchor flags validated")
            sys.exit(0)
        else:
            print("✗ Anchor flag validation failed!")
            sys.exit(1)

    save_path = None if args.no_save else args.save
    passed, failed, failures = run_tests(verbose=not args.quiet, save_path=save_path)

    if failed > 0:
        sys.exit(1)
    else:
        sys.exit(0)


if __name__ == "__main__":
    main()
