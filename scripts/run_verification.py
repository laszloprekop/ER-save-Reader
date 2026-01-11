#!/usr/bin/env python3
"""
Event Flag Verification Runner

Main script to run the verification framework against save files.
Tests all flag formulas and generates ground truth data.

Usage:
    python scripts/run_verification.py [options]

Options:
    --save PATH         Path to save file (default: searches common locations)
    --extracted PATH    Path to extracted_event_flags.json
    --manual PATH       Path to user-manually-set completions.txt
    --output PATH       Output path for ground_truth_offsets.json
    --categories CAT    Comma-separated list of categories to verify (default: priority)
    --verbose          Print detailed output
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path
from datetime import datetime
from typing import List, Optional

# Add scripts directory to path for imports
script_dir = Path(__file__).parent
sys.path.insert(0, str(script_dir))

from verification import (
    FlagVerification,
    VerificationStatus,
    FlagCategory,
    VerificationReport,
    SaveParser,
    SlotData,
    FlagFormulas,
    DiffAnalyzer,
)
from verification.data_loader import DataLoader
from verification.verification_data import FormulaResult, EmpiricalEvidence


def find_default_paths():
    """Find default paths for data files."""
    base = Path(__file__).parent.parent

    # Try to find save file
    save_locations = [
        Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2"),
        base.parent / "Elden Ring save files" / "ER0000.sl2",
        Path.home() / "AppData/Roaming/EldenRing" / "ER0000.sl2",  # Windows
    ]
    save_path = None
    for loc in save_locations:
        if loc.exists():
            save_path = loc
            break

    # Extracted flags
    extracted_path = base / "scripts" / "extracted_event_flags.json"
    if not extracted_path.exists():
        extracted_path = None

    # Manual completions
    manual_locations = [
        Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/user-manually-set completions.txt"),
        base.parent / "Elden Ring save files" / "user-manually-set completions.txt",
    ]
    manual_path = None
    for loc in manual_locations:
        if loc.exists():
            manual_path = loc
            break

    return save_path, extracted_path, manual_path


def run_verification(
    save_path: Path,
    extracted_path: Path,
    manual_path: Path,
    output_path: Path,
    categories: Optional[List[str]] = None,
    verbose: bool = False
) -> VerificationReport:
    """
    Run the full verification pipeline.

    1. Load extracted flags and manual completions
    2. Parse save file slots
    3. Test each flag against save data
    4. Generate verification report
    5. Export ground truth JSON
    """
    print("=" * 70)
    print("ELDEN RING EVENT FLAG VERIFICATION")
    print("=" * 70)
    print(f"Started: {datetime.now().isoformat()}")
    print()

    # Load data
    print("Loading data...")
    loader = DataLoader()

    if extracted_path and extracted_path.exists():
        loader.load_extracted_flags(extracted_path)
    else:
        print(f"WARNING: Extracted flags not found at {extracted_path}")

    if manual_path and manual_path.exists():
        loader.load_manual_completions(manual_path)
    else:
        print(f"WARNING: Manual completions not found at {manual_path}")

    # Get flags to verify
    if categories:
        flags_to_verify = loader.create_verification_entries(categories)
    else:
        # Default: priority categories
        flags_to_verify = loader.get_priority_flags()

    print(f"\nFlags to verify: {len(flags_to_verify)}")

    # Parse save file
    print(f"\nParsing save file: {save_path}")
    parser = SaveParser()
    formulas = FlagFormulas()

    try:
        parsed_save = parser.parse(save_path)
    except Exception as e:
        print(f"ERROR: Failed to parse save file: {e}")
        return None

    print(f"Found {len(parsed_save.slots)} slots, {len(parsed_save.active_slots)} active")

    # Create report
    report = VerificationReport()
    report.generated_date = datetime.now().isoformat()
    report.verification_method = "empirical_multi_save"

    # Add formula configurations
    config = formulas.export_config()
    report.block_bases = config["block_bases"]
    report.tile_formula_config = config["tile_formula"]
    report.dungeon_formula_config = config["dungeon_configs"]

    # Match manual completions to extracted flags
    matches_data = loader.match_manual_to_extracted()
    manual_flag_ids = {m["flag_id"]: m["manual"] for m in matches_data["matched"]}

    # Verify each flag against all slots
    print(f"\nVerifying {len(flags_to_verify)} flags against {len(parsed_save.slots)} slots...")

    for i, flag_entry in enumerate(flags_to_verify):
        if verbose and i % 100 == 0:
            print(f"  Progress: {i}/{len(flags_to_verify)}")

        flag_id = flag_entry.flag_id

        # Calculate offsets using formulas
        formula_results = formulas.calculate_offset(flag_id)
        for name, result in formula_results.items():
            flag_entry.add_formula_result(result)

        # Check against each slot
        for slot in parsed_save.slots:
            is_set, error = parser.check_flag(slot.event_flags, flag_id)

            if is_set:
                # Flag is set - add as evidence
                flag_entry.add_empirical_evidence(EmpiricalEvidence(
                    source=f"slot_{slot.slot_index}",
                    byte_offset=None,  # We don't know the actual offset from just checking
                    bit_position=None,
                    save_file=str(save_path.name),
                    slot_index=slot.slot_index,
                    confidence=0.8 if slot.validation_score >= 3 else 0.5,
                    notes=f"Flag set in slot {slot.slot_index}"
                ))

                # If we have a valid formula result, record the offset
                for name, result in formula_results.items():
                    if result.is_valid:
                        flag_entry.add_empirical_evidence(EmpiricalEvidence(
                            source=f"{name}_formula_validated",
                            byte_offset=result.byte_offset,
                            bit_position=result.bit_position,
                            save_file=str(save_path.name),
                            slot_index=slot.slot_index,
                            confidence=0.9,
                            notes=f"Formula {name} validated by slot {slot.slot_index}"
                        ))
                        break

        # Check manual completion
        if flag_id in manual_flag_ids:
            flag_entry.manual_completion = True
            flag_entry.auto_completion = len(flag_entry.empirical_evidence) > 0
            flag_entry.matches = flag_entry.manual_completion == flag_entry.auto_completion

        # Determine final status
        flag_entry.determine_status()

        # Add to report
        report.add_flag(flag_entry)

    # Compute statistics
    report.compute_statistics()

    # Print summary
    report.print_summary()

    # Export ground truth
    print(f"\nExporting ground truth to: {output_path}")
    report.export_ground_truth(output_path)

    print(f"\nVerification complete!")
    return report


def main():
    parser = argparse.ArgumentParser(
        description="Verify Elden Ring event flag calculations against save data"
    )
    parser.add_argument(
        "--save",
        type=Path,
        help="Path to ER0000.sl2 save file"
    )
    parser.add_argument(
        "--extracted",
        type=Path,
        help="Path to extracted_event_flags.json"
    )
    parser.add_argument(
        "--manual",
        type=Path,
        help="Path to user-manually-set completions.txt"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("ground_truth_offsets.json"),
        help="Output path for ground truth JSON"
    )
    parser.add_argument(
        "--categories",
        type=str,
        help="Comma-separated list of categories to verify"
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Print detailed output"
    )

    args = parser.parse_args()

    # Find default paths
    default_save, default_extracted, default_manual = find_default_paths()

    save_path = args.save or default_save
    extracted_path = args.extracted or default_extracted
    manual_path = args.manual or default_manual

    # Validate paths
    if not save_path or not save_path.exists():
        print(f"ERROR: Save file not found: {save_path}")
        print("Use --save to specify the path to ER0000.sl2")
        sys.exit(1)

    if not extracted_path or not extracted_path.exists():
        print(f"WARNING: Extracted flags not found: {extracted_path}")
        print("Run extract_event_flags.py first, or use --extracted to specify path")

    # Parse categories
    categories = None
    if args.categories:
        categories = [c.strip() for c in args.categories.split(",")]

    # Run verification
    report = run_verification(
        save_path=save_path,
        extracted_path=extracted_path,
        manual_path=manual_path,
        output_path=args.output,
        categories=categories,
        verbose=args.verbose
    )

    if report:
        print(f"\nGround truth saved to: {args.output}")


if __name__ == "__main__":
    main()
