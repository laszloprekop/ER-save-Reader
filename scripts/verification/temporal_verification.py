#!/usr/bin/env python3
"""
Temporal Verification using Before/After Capture Pairs

This is the gold standard for verification - actual before/after temporal evidence.
Uses captures with known flag IDs to verify that the flag bit changes from 0 to 1.
"""

import json
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser
from scripts.verification.ground_truth_loader import (
    load_block_bases,
    load_dungeon_bases,
    get_tile_config,
)


def load_ground_truth() -> dict:
    """Load ground truth as a unified dict for backward compatibility."""
    return {
        "formulas": {
            "tile_formula": get_tile_config(),
            "dungeon_formula": {str(k): v for k, v in load_dungeon_bases().items()},
            "block_bases": {str(k): v for k, v in load_block_bases().items()},
        }
    }

# Paths
CATALOG_PATH = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/capture_catalog.json")
SNAPSHOT_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")


@dataclass
class TemporalResult:
    """Result of temporal verification for a single flag."""
    flag_id: int
    pair_id: str
    name: str
    category: str

    # Calculated location
    byte_offset: int
    bit_position: int
    formula_type: str  # "tile", "dungeon", "block"

    # Before state
    before_file: str
    before_byte: int
    before_bit: int

    # After state
    after_file: str
    after_byte: int
    after_bit: int

    # Verdict
    status: str  # "verified", "failed", "inconclusive"
    notes: str


def load_catalog() -> dict:
    """Load the capture catalog."""
    with open(CATALOG_PATH) as f:
        return json.load(f)


def calculate_flag_offset(flag_id: int, ground_truth: dict) -> Optional[Tuple[int, int, str]]:
    """
    Calculate byte offset and bit position for a flag using ground truth formulas.

    Returns: (byte_offset, bit_position, formula_type) or None
    """
    bit = 7 - (flag_id % 8)

    # Tile flags (10-digit, >= 1_000_000_000)
    if flag_id >= 1_000_000_000:
        tile_formula = ground_truth.get("formulas", {}).get("tile_formula", {})
        base = tile_formula.get("base_offset", 485330)
        bytes_per_slot = tile_formula.get("bytes_per_slot", 875)
        slots_per_row = tile_formula.get("slots_per_row", 40)
        row_base = tile_formula.get("row_base", 33)
        col_base = tile_formula.get("col_base", 30)
        max_local_id = tile_formula.get("max_local_id", 6999)

        tile_index = (flag_id - 1_000_000_000) // 10000
        local_id = flag_id % 10000

        if local_id > max_local_id:
            return None  # Untrackable

        row = tile_index // 100
        col = tile_index % 100
        slot = (row - row_base) * slots_per_row + (col - col_base)

        if slot < 0:
            return None

        byte_offset = base + slot * bytes_per_slot + local_id // 8
        return (byte_offset, bit, "tile")

    # Dungeon flags (8-digit, 10_000_000 - 43_999_999)
    if 10_000_000 <= flag_id < 44_000_000:
        area = flag_id // 1_000_000
        section = (flag_id // 10_000) % 100
        local_id = flag_id % 10_000

        dungeon_bases = ground_truth.get("formulas", {}).get("dungeon_formula", {})
        section_size = 1125

        area_info = dungeon_bases.get(str(area), {})
        if area_info and area_info.get("base_offset"):
            base = area_info["base_offset"]
            byte_offset = base + section * section_size + local_id // 8
            return (byte_offset, bit, "dungeon")

        return None

    # Block flags (60_000 - 99_999)
    if 60_000 <= flag_id < 100_000:
        block_bases = ground_truth.get("formulas", {}).get("block_bases", {})

        # Try sub-block first (100-granularity)
        sub_block = (flag_id // 100) * 100
        if str(sub_block) in block_bases:
            base_info = block_bases[str(sub_block)]
            if base_info.get("status") in ("verified", "partial"):
                base = base_info["base_offset"]
                relative = flag_id - sub_block
                byte_offset = base + relative // 8
                return (byte_offset, bit, "block")

        # Try main block (1000-granularity)
        main_block = (flag_id // 1000) * 1000
        if str(main_block) in block_bases:
            base_info = block_bases[str(main_block)]
            if base_info.get("status") in ("verified", "partial"):
                base = base_info["base_offset"]
                relative = flag_id - main_block
                byte_offset = base + relative // 8
                return (byte_offset, bit, "block")

        return None

    # Simple flags (< 60_000)
    if flag_id < 60_000:
        byte_offset = flag_id // 8
        return (byte_offset, bit, "simple")

    return None


def verify_temporal_pair(
    parser: SaveParser,
    pair_id: str,
    before_cap: dict,
    after_cap: dict,
    ground_truth: dict,
) -> Optional[TemporalResult]:
    """
    Verify a before/after capture pair.

    Checks that the flag bit is 0 in 'before' and 1 in 'after'.
    """
    # Get flag ID from POI
    flag_id = before_cap.get("poi", {}).get("flag_id") or after_cap.get("poi", {}).get("flag_id")
    if not flag_id:
        return None

    name = before_cap.get("poi", {}).get("name", "Unknown")

    # Calculate expected offset
    location = calculate_flag_offset(flag_id, ground_truth)
    if not location:
        return TemporalResult(
            flag_id=flag_id,
            pair_id=pair_id,
            name=name,
            category="unknown",
            byte_offset=0,
            bit_position=0,
            formula_type="unknown",
            before_file=before_cap.get("filename", ""),
            before_byte=0,
            before_bit=0,
            after_file=after_cap.get("filename", ""),
            after_byte=0,
            after_bit=0,
            status="inconclusive",
            notes=f"No formula for flag {flag_id}"
        )

    byte_offset, bit_pos, formula_type = location

    # Load before and after saves
    slot_index = before_cap.get("slot_context", {}).get("slot_index", 0)

    before_path = SNAPSHOT_DIR / before_cap["filename"]
    after_path = SNAPSHOT_DIR / after_cap["filename"]

    if not before_path.exists() or not after_path.exists():
        return TemporalResult(
            flag_id=flag_id,
            pair_id=pair_id,
            name=name,
            category=formula_type,
            byte_offset=byte_offset,
            bit_position=bit_pos,
            formula_type=formula_type,
            before_file=before_cap.get("filename", ""),
            before_byte=0,
            before_bit=0,
            after_file=after_cap.get("filename", ""),
            after_byte=0,
            after_bit=0,
            status="inconclusive",
            notes="Save files not found"
        )

    try:
        before_parsed = parser.parse(str(before_path), slots_to_parse=[slot_index])
        after_parsed = parser.parse(str(after_path), slots_to_parse=[slot_index])
    except Exception as e:
        return TemporalResult(
            flag_id=flag_id,
            pair_id=pair_id,
            name=name,
            category=formula_type,
            byte_offset=byte_offset,
            bit_position=bit_pos,
            formula_type=formula_type,
            before_file=before_cap.get("filename", ""),
            before_byte=0,
            before_bit=0,
            after_file=after_cap.get("filename", ""),
            after_byte=0,
            after_bit=0,
            status="inconclusive",
            notes=f"Parse error: {e}"
        )

    if not before_parsed.slots or not after_parsed.slots:
        return None

    before_ef = before_parsed.slots[0].event_flags
    after_ef = after_parsed.slots[0].event_flags

    # Check bounds
    if byte_offset >= len(before_ef) or byte_offset >= len(after_ef):
        return TemporalResult(
            flag_id=flag_id,
            pair_id=pair_id,
            name=name,
            category=formula_type,
            byte_offset=byte_offset,
            bit_position=bit_pos,
            formula_type=formula_type,
            before_file=before_cap.get("filename", ""),
            before_byte=0,
            before_bit=0,
            after_file=after_cap.get("filename", ""),
            after_byte=0,
            after_bit=0,
            status="inconclusive",
            notes=f"Offset {byte_offset} out of bounds"
        )

    # Read actual values
    before_byte = before_ef[byte_offset]
    after_byte = after_ef[byte_offset]
    before_bit = (before_byte >> bit_pos) & 1
    after_bit = (after_byte >> bit_pos) & 1

    # Determine verdict
    if before_bit == 0 and after_bit == 1:
        status = "verified"
        notes = "PERFECT: 0 → 1 transition confirmed"
    elif before_bit == 1 and after_bit == 1:
        status = "inconclusive"
        notes = "Already set in before (may have been set earlier)"
    elif before_bit == 0 and after_bit == 0:
        status = "failed"
        notes = "Flag NOT set in after - formula may be wrong"
    else:  # before=1, after=0
        status = "failed"
        notes = "INVERTED: 1 → 0 (very wrong)"

    # Check for padding bytes
    if before_byte == 0xFF or after_byte == 0xFF:
        status = "inconclusive"
        notes = f"Padding byte detected (before={before_byte:02x}, after={after_byte:02x})"

    return TemporalResult(
        flag_id=flag_id,
        pair_id=pair_id,
        name=name,
        category=formula_type,
        byte_offset=byte_offset,
        bit_position=bit_pos,
        formula_type=formula_type,
        before_file=before_cap.get("filename", ""),
        before_byte=before_byte,
        before_bit=before_bit,
        after_file=after_cap.get("filename", ""),
        after_byte=after_byte,
        after_bit=after_bit,
        status=status,
        notes=notes
    )


def main():
    import argparse

    arg_parser = argparse.ArgumentParser(description="Temporal verification using before/after pairs")
    arg_parser.add_argument("--output", "-o", help="Output JSON file")
    arg_parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    arg_parser.add_argument("--filter", help="Filter by formula type (tile, dungeon, block)")
    args = arg_parser.parse_args()

    print("=" * 70)
    print("TEMPORAL VERIFICATION - Before/After Capture Pairs")
    print("=" * 70)

    # Load data
    catalog = load_catalog()
    ground_truth = load_ground_truth()
    parser = SaveParser()

    # Group captures by pair_id
    captures_by_pair = {}
    for cap in catalog.get("captures", []):
        pair_id = cap.get("pair_id")
        if pair_id:
            if pair_id not in captures_by_pair:
                captures_by_pair[pair_id] = {"before": None, "after": None}
            if cap.get("phase") == "before":
                captures_by_pair[pair_id]["before"] = cap
            elif cap.get("phase") == "after":
                captures_by_pair[pair_id]["after"] = cap

    # Filter to complete pairs with flag IDs
    valid_pairs = []
    for pair_id, caps in captures_by_pair.items():
        if caps["before"] and caps["after"]:
            flag_id = (caps["before"].get("poi", {}).get("flag_id") or
                      caps["after"].get("poi", {}).get("flag_id"))
            if flag_id:
                valid_pairs.append((pair_id, caps["before"], caps["after"]))

    print(f"\nFound {len(valid_pairs)} complete pairs with flag IDs")

    # Run verification
    results = []
    summary = {"verified": 0, "failed": 0, "inconclusive": 0, "by_type": {}}

    for pair_id, before, after in sorted(valid_pairs):
        result = verify_temporal_pair(parser, pair_id, before, after, ground_truth)
        if result:
            # Apply filter if specified
            if args.filter and result.formula_type != args.filter:
                continue

            results.append(result)
            summary[result.status] += 1

            if result.formula_type not in summary["by_type"]:
                summary["by_type"][result.formula_type] = {"verified": 0, "failed": 0, "inconclusive": 0}
            summary["by_type"][result.formula_type][result.status] += 1

            if args.verbose or result.status != "verified":
                status_icon = "✓" if result.status == "verified" else "✗" if result.status == "failed" else "?"
                print(f"\n{status_icon} {pair_id}: flag {result.flag_id} ({result.name})")
                print(f"   Type: {result.formula_type}, Offset: {result.byte_offset}, Bit: {result.bit_position}")
                print(f"   Before: byte={result.before_byte:02x}, bit={result.before_bit}")
                print(f"   After:  byte={result.after_byte:02x}, bit={result.after_bit}")
                print(f"   Status: {result.status} - {result.notes}")

    # Print summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)

    total = len(results)
    if total > 0:
        print(f"\nTotal pairs verified: {total}")
        print(f"  Verified: {summary['verified']} ({summary['verified']/total*100:.1f}%)")
        print(f"  Failed: {summary['failed']} ({summary['failed']/total*100:.1f}%)")
        print(f"  Inconclusive: {summary['inconclusive']} ({summary['inconclusive']/total*100:.1f}%)")

        print("\nBy formula type:")
        for ftype, counts in summary["by_type"].items():
            ftotal = sum(counts.values())
            print(f"  {ftype}: {counts['verified']}/{ftotal} verified ({counts['verified']/ftotal*100:.0f}%)")

    # Save output
    if args.output:
        output_data = {
            "summary": summary,
            "results": [vars(r) for r in results]
        }
        with open(args.output, 'w') as f:
            json.dump(output_data, f, indent=2)
        print(f"\nResults saved to: {args.output}")

    return 0 if summary["failed"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
