#!/usr/bin/env python3
"""
Generate Block Cases

Batch generator for creating and verifying cases for items in a block.
Uses block_items.json for item definitions and runs the full defense/challenge cycle.

Usage:
    python generate_block_cases.py --block 520000
    python generate_block_cases.py --block 67000 --save-cases
    python generate_block_cases.py --all-blocks --use-catalog --use-anchors
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.case_manager import (
    CaseManager,
    CaseStatus,
    FlagHypothesis,
    VerificationCase,
)
from scripts.verification.ground_truth_loader import calculate_block_offset


# Default paths
DEFAULT_SAVE = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"
BLOCK_ITEMS_PATH = PROJECT_ROOT / "scripts" / "verification" / "block_items.json"


def load_block_items() -> Dict[str, Any]:
    """Load block_items.json."""
    if not BLOCK_ITEMS_PATH.exists():
        print(f"Error: block_items.json not found at {BLOCK_ITEMS_PATH}")
        return {}

    with open(BLOCK_ITEMS_PATH) as f:
        return json.load(f)


def generate_cases_for_block(
    manager: CaseManager,
    block_id: int,
    block_data: Dict[str, Any],
    save_path: str,
    slots_with: List[int] = None,
    slots_without: List[int] = None,
    use_catalog: bool = False,
    use_anchors: bool = False,
    use_all_saves: bool = False,
    save_cases: bool = False,
) -> Dict[str, Any]:
    """Generate and verify cases for all items in a block.

    Args:
        manager: CaseManager instance
        block_id: Block ID (e.g., 520000)
        block_data: Block configuration from block_items.json
        save_path: Save file path
        slots_with: Slot indices with items (default: [0])
        slots_without: Slot indices without items (default: [1,2,3,4])
        use_catalog: Use capture catalog for temporal defense
        use_anchors: Use anchor database for chain anchor defense
        use_all_saves: Use all configured saves for cross-save defense
        save_cases: Save individual cases to JSON

    Returns:
        Results dictionary with counts and case details
    """
    slots_with = slots_with or [0]
    slots_without = slots_without or [1, 2, 3, 4]

    items = block_data.get('items', [])
    base_offset = block_data.get('base_offset')
    formula_type = block_data.get('formula_type', 'block')
    category = block_data.get('category', 'unknown')

    results = {
        "block_id": block_id,
        "category": category,
        "formula_type": formula_type,
        "base_offset": base_offset,
        "total": len(items),
        "verified": [],
        "partial": [],
        "rejected": [],
        "inconclusive": [],
        "errors": [],
    }

    # Collect related flags for alternative base search
    related_flags = [(item['flag_id'], item.get('item_id')) for item in items if item.get('item_id')]
    known_flags = [(item['flag_id'], item['name']) for item in items]

    print(f"\n{'=' * 70}")
    print(f"GENERATING CASES: Block {block_id} ({category})")
    print(f"{'=' * 70}")
    print(f"Items: {len(items)}")
    print(f"Base offset: {base_offset}")
    print(f"Formula type: {formula_type}")

    for item in items:
        flag_id = item['flag_id']
        item_id = item.get('item_id')
        name = item['name']

        print(f"\n{'─' * 50}")
        print(f"Processing: {name} (flag {flag_id})")

        try:
            # Calculate hypothesis
            if formula_type == 'block' and base_offset is not None:
                # Use block formula
                byte_offset = base_offset + (flag_id - block_id) // 8
                bit_position = 7 - (flag_id % 8)
                hypothesis = FlagHypothesis(
                    byte_offset=byte_offset,
                    bit_position=bit_position,
                    implied_base=base_offset,
                    block_start=block_id,
                )
            else:
                # Use ground_truth_loader for dungeon or other formulas
                result = calculate_block_offset(flag_id)
                if result:
                    byte_offset, bit_position = result
                    hypothesis = FlagHypothesis(
                        byte_offset=byte_offset,
                        bit_position=bit_position,
                    )
                else:
                    print(f"  Warning: Could not calculate offset for {flag_id}")
                    results['errors'].append((flag_id, name, "No offset calculation"))
                    continue

            # Create case
            case = manager.create_case(
                flag_id=flag_id,
                item_name=name,
                category=item.get('category', category),
                item_id=item_id,
                hypothesis=hypothesis,
            )

            # Run defense phase
            manager.run_defense_phase(
                case,
                save_path,
                slots_with_item=slots_with if item_id else None,
                slots_without_item=slots_without if item_id else None,
            )

            # Run challenge phase
            manager.run_challenge_phase(
                case,
                save_path,
                related_flags=related_flags if item_id else None,
                known_flags=known_flags,
            )

            # Additional defense: Auto-temporal from catalog
            if use_catalog:
                temporal_evidence = manager.defend_with_temporal_auto(case, slot_index=0)
                if temporal_evidence:
                    print(f"  Temporal: {len(temporal_evidence)} pair(s)")

            # Additional defense: Chain anchors
            if use_anchors:
                anchor_evidence = manager.defend_with_chain_anchor(case, save_path, slot_index=0)
                if anchor_evidence and anchor_evidence.supports_hypothesis:
                    print(f"  Anchors: matched")

            # Additional defense: Cross-save from config
            if use_all_saves:
                cross_evidence = manager.defend_with_cross_save_auto(case)
                if cross_evidence:
                    supporting = sum(1 for e in cross_evidence if e.supports_hypothesis)
                    print(f"  Cross-save: {supporting}/{len(cross_evidence)}")

            # Categorize result
            status_key = case.status.value
            case_info = {
                "flag_id": flag_id,
                "name": name,
                "confidence": case.confidence,
                "case_id": case.case_id,
                "hypothesis": {
                    "byte_offset": hypothesis.byte_offset,
                    "bit_position": hypothesis.bit_position,
                },
            }

            if status_key == "verified":
                results['verified'].append(case_info)
            elif status_key == "partial":
                results['partial'].append(case_info)
            elif status_key == "rejected":
                results['rejected'].append(case_info)
            else:
                results['inconclusive'].append(case_info)

            print(f"  Status: {case.status.value.upper()}")
            print(f"  Confidence: {case.confidence:.2f}")

            # Save case
            if save_cases:
                filepath = manager.save_case(case)
                print(f"  Saved: {filepath.name}")

        except Exception as e:
            print(f"  Error: {e}")
            results['errors'].append((flag_id, name, str(e)))

    return results


def print_summary(results: Dict[str, Any]):
    """Print summary of results."""
    print(f"\n{'=' * 70}")
    print("SUMMARY")
    print(f"{'=' * 70}")

    print(f"\nBlock: {results['block_id']} ({results['category']})")
    print(f"Total items: {results['total']}")

    for status in ['verified', 'partial', 'rejected', 'inconclusive']:
        items = results.get(status, [])
        if items:
            print(f"\n{status.upper()} ({len(items)}):")
            for item in items:
                marker = "✓" if status == "verified" else "◐" if status == "partial" else "✗" if status == "rejected" else "?"
                print(f"  {marker} {item['flag_id']}: {item['name']} (conf: {item['confidence']:.2f})")

    if results.get('errors'):
        print(f"\nERRORS ({len(results['errors'])}):")
        for flag_id, name, error in results['errors']:
            print(f"  ! {flag_id}: {name} - {error}")

    # Statistics
    total = results['total']
    verified = len(results.get('verified', []))
    partial = len(results.get('partial', []))

    print(f"\n{'─' * 40}")
    print(f"Verified: {verified}/{total} ({verified/total*100:.1f}%)" if total > 0 else "Verified: 0")
    print(f"Partial: {partial}/{total} ({partial/total*100:.1f}%)" if total > 0 else "Partial: 0")
    print(f"Verification rate: {(verified + partial)/total*100:.1f}%" if total > 0 else "N/A")


def main():
    parser = argparse.ArgumentParser(
        description="Generate verification cases for block items",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    parser.add_argument("--block", type=int, help="Block ID to generate cases for")
    parser.add_argument("--all-blocks", action="store_true", help="Generate for all blocks")
    parser.add_argument("--save", default=DEFAULT_SAVE, help="Save file path")
    parser.add_argument("--slots-with", help="Comma-separated slot indices with item")
    parser.add_argument("--slots-without", help="Comma-separated slot indices without item")
    parser.add_argument("--use-catalog", action="store_true", help="Use capture catalog")
    parser.add_argument("--use-anchors", action="store_true", help="Use anchor database")
    parser.add_argument("--all-saves", action="store_true", help="Use all configured saves")
    parser.add_argument("--save-cases", action="store_true", help="Save individual cases")
    parser.add_argument("--output", "-o", help="Output JSON file for results")
    parser.add_argument("--list-blocks", action="store_true", help="List available blocks")

    args = parser.parse_args()

    # Load block items
    block_items = load_block_items()
    if not block_items:
        return 1

    blocks = block_items.get('blocks', {})

    # List blocks
    if args.list_blocks:
        print("Available blocks:")
        for block_id, data in sorted(blocks.items()):
            print(f"  {block_id}: {data.get('category', 'unknown')} - {len(data.get('items', []))} items")
        return 0

    # Validate block selection
    if not args.block and not args.all_blocks:
        print("Error: Specify --block or --all-blocks")
        parser.print_help()
        return 1

    # Parse slots
    slots_with = [int(s) for s in args.slots_with.split(",")] if args.slots_with else [0]
    slots_without = [int(s) for s in args.slots_without.split(",")] if args.slots_without else [1, 2, 3, 4]

    # Create manager
    manager = CaseManager()

    # Generate cases
    all_results = []

    if args.all_blocks:
        blocks_to_process = [(int(k), v) for k, v in blocks.items()]
    else:
        block_str = str(args.block)
        if block_str not in blocks:
            print(f"Error: Block {args.block} not found in block_items.json")
            print(f"Available: {list(blocks.keys())}")
            return 1
        blocks_to_process = [(args.block, blocks[block_str])]

    for block_id, block_data in blocks_to_process:
        results = generate_cases_for_block(
            manager,
            block_id,
            block_data,
            args.save,
            slots_with=slots_with,
            slots_without=slots_without,
            use_catalog=args.use_catalog,
            use_anchors=args.use_anchors,
            use_all_saves=args.all_saves,
            save_cases=args.save_cases,
        )
        all_results.append(results)
        print_summary(results)

    # Output JSON results
    if args.output:
        output_path = Path(args.output)
        with open(output_path, 'w') as f:
            json.dump(all_results, f, indent=2)
        print(f"\nResults saved to: {output_path}")

    # Overall summary for all blocks
    if len(all_results) > 1:
        print(f"\n{'=' * 70}")
        print("OVERALL SUMMARY")
        print(f"{'=' * 70}")

        total_items = sum(r['total'] for r in all_results)
        total_verified = sum(len(r.get('verified', [])) for r in all_results)
        total_partial = sum(len(r.get('partial', [])) for r in all_results)

        print(f"Blocks processed: {len(all_results)}")
        print(f"Total items: {total_items}")
        print(f"Verified: {total_verified} ({total_verified/total_items*100:.1f}%)" if total_items > 0 else "")
        print(f"Partial: {total_partial} ({total_partial/total_items*100:.1f}%)" if total_items > 0 else "")
        print(f"Overall rate: {(total_verified + total_partial)/total_items*100:.1f}%" if total_items > 0 else "")

    return 0


if __name__ == "__main__":
    sys.exit(main())
