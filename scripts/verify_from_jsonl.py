#!/usr/bin/env python3
"""
Verify flags from elden-map flag-correlation-candidates.jsonl against actual save files.

This script reads the JSONL file, parses the save files, and checks if the
computed offsets match the actual flag states.
"""

import json
import sys
from pathlib import Path
from collections import defaultdict

# Add the scripts directory to path
sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser
from verification.archive.flag_formulas import FlagFormulas


def load_verification_records(jsonl_path: Path) -> list:
    """Load verification records from JSONL file."""
    records = []
    with open(jsonl_path) as f:
        for line in f:
            if line.strip():
                records.append(json.loads(line))
    return records


def group_by_slot(records: list) -> dict:
    """Group records by slotIndex."""
    by_slot = defaultdict(list)
    for rec in records:
        by_slot[rec["slotIndex"]].append(rec)
    return by_slot


def verify_record(record: dict, event_flags: bytes, formulas: FlagFormulas) -> dict:
    """Verify a single record against actual save data."""
    flag_id = record["flagId"]
    manual_status = record["userMarkedComplete"]
    flag_type = record["flagType"]

    # Calculate offset using our formulas
    results = formulas.calculate_offset(flag_id)

    # Get the appropriate formula result
    formula_result = None
    formula_used = None

    if flag_type == "block" and "block" in results:
        formula_result = results["block"]
        formula_used = "block"
    elif flag_type == "tile" and "tile" in results:
        formula_result = results["tile"]
        formula_used = "tile"
    elif flag_type == "dungeon" and "dungeon" in results:
        formula_result = results["dungeon"]
        formula_used = "dungeon"
    elif results:
        # Use first available formula
        formula_used = list(results.keys())[0]
        formula_result = results[formula_used]

    if not formula_result or not formula_result.is_valid:
        return {
            "flag_id": flag_id,
            "flag_name": record["flagName"],
            "flag_type": flag_type,
            "manual_status": manual_status,
            "formula_used": formula_used,
            "formula_valid": False,
            "error": formula_result.error_message if formula_result else "No formula",
            "actual_status": None,
            "matches": False,
            "computed_offset": None,
            "computed_bit": None,
        }

    byte_offset = formula_result.byte_offset
    bit_position = formula_result.bit_position

    # Check bounds
    if byte_offset >= len(event_flags):
        return {
            "flag_id": flag_id,
            "flag_name": record["flagName"],
            "flag_type": flag_type,
            "manual_status": manual_status,
            "formula_used": formula_used,
            "formula_valid": True,
            "error": f"Offset {byte_offset} exceeds EventFlags size {len(event_flags)}",
            "actual_status": None,
            "matches": False,
            "computed_offset": byte_offset,
            "computed_bit": bit_position,
        }

    # Read actual flag state
    # bit_position is the logical bit position (0-7 from right in standard notation)
    # To extract: (byte >> bit_position) & 1
    byte_val = event_flags[byte_offset]
    actual_status = bool((byte_val >> bit_position) & 1)
    matches = (actual_status == manual_status)

    return {
        "flag_id": flag_id,
        "flag_name": record["flagName"],
        "flag_type": flag_type,
        "manual_status": manual_status,
        "formula_used": formula_used,
        "formula_valid": True,
        "error": None,
        "actual_status": actual_status,
        "matches": matches,
        "computed_offset": byte_offset,
        "computed_bit": bit_position,
        "byte_value": f"0x{byte_val:02X}",
    }


def main():
    # Paths
    jsonl_path = Path("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/flag-correlation-candidates.jsonl")
    # Use the current save file from CrossOver
    save_path = Path("/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2")

    if not jsonl_path.exists():
        print(f"Error: {jsonl_path} not found")
        return

    if not save_path.exists():
        print(f"Error: {save_path} not found")
        return

    # Load verification records
    print(f"Loading verification records from {jsonl_path}")
    records = load_verification_records(jsonl_path)
    print(f"Loaded {len(records)} records")

    # Group by slot
    by_slot = group_by_slot(records)
    print(f"Slots in data: {sorted(by_slot.keys())}")

    # Parse save file
    print(f"\nParsing save file: {save_path}")
    parser = SaveParser()

    # Get unique slots needed
    slots_needed = sorted(by_slot.keys())
    save_data = parser.parse(save_path, slots_needed)

    # Initialize formulas
    formulas = FlagFormulas()

    # Process each slot
    all_results = []
    for slot_idx in slots_needed:
        slot_records = by_slot[slot_idx]

        # Find parsed slot
        slot_data = None
        for s in save_data.slots:
            if s.slot_index == slot_idx:
                slot_data = s
                break

        if not slot_data:
            print(f"\nSlot {slot_idx}: NOT FOUND IN SAVE")
            continue

        print(f"\nSlot {slot_idx} ({slot_data.character_name or 'Unknown'}):")
        print(f"  EventFlags offset: {slot_data.event_flags_offset}")
        print(f"  EventFlags size: {len(slot_data.event_flags)}")
        print(f"  Validation score: {slot_data.validation_score}/4")
        print(f"  Records to verify: {len(slot_records)}")

        slot_results = []
        for rec in slot_records:
            result = verify_record(rec, slot_data.event_flags, formulas)
            result["slot_index"] = slot_idx
            slot_results.append(result)

        all_results.extend(slot_results)

    # Analyze results
    print("\n" + "=" * 70)
    print("VERIFICATION RESULTS")
    print("=" * 70)

    # Overall stats
    total = len(all_results)
    matching = sum(1 for r in all_results if r["matches"])
    valid_formula = sum(1 for r in all_results if r["formula_valid"])

    print(f"\nOverall: {matching}/{total} matching ({100*matching/total:.1f}%)")
    print(f"Valid formulas: {valid_formula}/{total}")

    # By flag type
    print("\n--- By Flag Type ---")
    by_type = defaultdict(list)
    for r in all_results:
        by_type[r["flag_type"]].append(r)

    for flag_type in sorted(by_type.keys()):
        results = by_type[flag_type]
        type_matching = sum(1 for r in results if r["matches"])
        type_valid = sum(1 for r in results if r["formula_valid"])
        print(f"  {flag_type}: {type_matching}/{len(results)} matching, {type_valid} valid formulas")

    # Show mismatches for block formulas (should be working)
    print("\n--- Block Formula Mismatches (should be working) ---")
    block_mismatches = [r for r in all_results if r["flag_type"] == "block" and not r["matches"] and r["formula_valid"]]
    if block_mismatches:
        for r in block_mismatches[:20]:
            print(f"  {r['flag_id']} ({r['flag_name']}): manual={r['manual_status']}, actual={r['actual_status']}, offset={r['computed_offset']}, bit={r['computed_bit']}, byte={r.get('byte_value', 'N/A')}")
    else:
        print("  All block formulas matching!")

    # Show tile formula details
    print("\n--- Tile Formula Results ---")
    tile_results = [r for r in all_results if r["flag_type"] == "tile"]
    tile_valid = [r for r in tile_results if r["formula_valid"]]
    tile_matching = [r for r in tile_valid if r["matches"]]
    tile_untrackable = [r for r in tile_results if not r["formula_valid"] and r.get("error") and "UNTRACKABLE" in r.get("error", "")]

    print(f"  Total: {len(tile_results)}")
    print(f"  Valid formulas: {len(tile_valid)}")
    print(f"  Matching: {len(tile_matching)}")
    print(f"  Untrackable (localId >= 7000): {len(tile_untrackable)}")

    if tile_valid and not tile_matching:
        print("\n  Valid tile formula mismatches (first 10):")
        for r in [r for r in tile_valid if not r["matches"]][:10]:
            print(f"    {r['flag_id']} ({r['flag_name']}): manual={r['manual_status']}, actual={r['actual_status']}, offset={r['computed_offset']}, bit={r['computed_bit']}")

    # Show dungeon formula details
    print("\n--- Dungeon Formula Results ---")
    dungeon_results = [r for r in all_results if r["flag_type"] == "dungeon"]
    dungeon_valid = [r for r in dungeon_results if r["formula_valid"]]
    dungeon_matching = [r for r in dungeon_valid if r["matches"]]

    print(f"  Total: {len(dungeon_results)}")
    print(f"  Valid formulas: {len(dungeon_valid)}")
    print(f"  Matching: {len(dungeon_matching)}")

    for r in dungeon_results:
        status = "MATCH" if r["matches"] else ("INVALID" if not r["formula_valid"] else "MISMATCH")
        err = r.get("error", "")
        print(f"    {r['flag_id']} ({r['flag_name']}): {status} - {err}")

    # Save detailed results
    output_path = Path(__file__).parent / "verification_results.json"
    with open(output_path, "w") as f:
        json.dump({
            "summary": {
                "total": total,
                "matching": matching,
                "valid_formulas": valid_formula,
            },
            "by_type": {
                k: {
                    "total": len(v),
                    "matching": sum(1 for r in v if r["matches"]),
                    "valid": sum(1 for r in v if r["formula_valid"]),
                }
                for k, v in by_type.items()
            },
            "results": all_results,
        }, f, indent=2)
    print(f"\nDetailed results saved to {output_path}")


if __name__ == "__main__":
    main()
