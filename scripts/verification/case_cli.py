#!/usr/bin/env python3
"""
Case-Based Verification CLI

Command-line interface for managing verification cases.

Usage:
    python case_cli.py create --flag 520000 --name "Lhutel" --category spirit_ash
    python case_cli.py verify --case-id 520000_20260131 --save /path/to/save.sl2
    python case_cli.py list
    python case_cli.py report --case-id 520000_20260131
    python case_cli.py batch --block 520000 --save /path/to/save.sl2
"""

import argparse
import sys
from pathlib import Path
from typing import List, Tuple

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.case_manager import (
    CaseManager,
    FlagHypothesis,
    CaseStatus,
    VerificationCase,
)
from scripts.verification.ground_truth_loader import (
    load_block_bases,
    get_block_base,
)
from scripts.verification.flag_schema import BlockSchema, AllocationBitmap


# Default save path
DEFAULT_SAVE = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

# Known items by block for batch verification
# Format: (flag_id, item_id, name, category)
# NOTE: Base offsets are loaded dynamically from ground_truth_offsets.json via ground_truth_loader
BLOCK_ITEMS = {
    520000: [
        (520000, 258000, "Lhutel the Headless", "spirit_ash"),
        (520030, 5050, "Assassin's Crimson Dagger", "talisman"),
        (520040, 202000, "Banished Knight Engvall", "spirit_ash"),
        (520050, 219000, "Twinsage Sorcerer Ashes", "spirit_ash"),
        (520090, 239000, "Bloodhound Knight Floh", "spirit_ash"),
        (520110, 217000, "Perfumer Tricia", "spirit_ash"),
        (520210, 5060, "Assassin's Cerulean Dagger", "talisman"),
        (520300, 1020, "Viridian Amber Medallion", "talisman"),
        (520310, 4010, "Spelldrake Talisman", "talisman"),
        (520330, 4020, "Flamedrake Talisman", "talisman"),
        (520350, 2110, "Blue Dancer Charm", "talisman"),
        (520370, 1010, "Cerulean Amber Medallion", "talisman"),
        (520390, 2170, "Kindred of Rot's Exultation", "talisman"),
        (520450, 1110, "Gold Scarab", "talisman"),
        (520480, 5040, "Godskin Swaddling Cloth", "talisman"),
    ],
    # NOTE: Block 62000 DISABLED - flag IDs 62010-62080 don't exist in game data.
    # Actual map fragment pickup flags are 10-digit tile-based (e.g., 1042370200).
    # Block 62000 contains WorldMapPointParam flags for location discovery.
    # See ground_truth_offsets.json notes for details.
    # 62000: [
    #     # INVALID - these flag IDs don't exist
    #     (62010, 8600, "Map: Limgrave, West", "landmark"),
    #     ...
    # ],
    # NOTE: Blocks 67000/68000 need re-verification - base offsets may be incorrect.
    # Flag IDs exist but verification shows flags unset even when items present.
    # See ground_truth_offsets.json notes for details.
    67000: [
        # Cookbooks - NEEDS RE-VERIFICATION
        (67000, 9300, "Nomadic Warrior's Cookbook [1]", "cookbook"),
        (67010, 9301, "Nomadic Warrior's Cookbook [3]", "cookbook"),
        (67030, 9303, "Nomadic Warrior's Cookbook [10]", "cookbook"),
        (67080, 9304, "Nomadic Warrior's Cookbook [11]", "cookbook"),
        (67100, 9305, "Nomadic Warrior's Cookbook [12]", "cookbook"),
        (67110, 9306, "Nomadic Warrior's Cookbook [13]", "cookbook"),
        (67120, 9310, "Missionary's Cookbook [1]", "cookbook"),
        (67130, 9311, "Missionary's Cookbook [2]", "cookbook"),
        (67210, 9312, "Missionary's Cookbook [3]", "cookbook"),
        (67220, 9313, "Missionary's Cookbook [4]", "cookbook"),
        (67230, 9314, "Missionary's Cookbook [5]", "cookbook"),
        (67270, 9315, "Missionary's Cookbook [6]", "cookbook"),
        (67280, 9316, "Missionary's Cookbook [7]", "cookbook"),
    ],
    68000: [
        # Cookbooks continued - NEEDS RE-VERIFICATION
        (68000, 9350, "Ancient Dragon Apostle's Cookbook [1]", "cookbook"),
        (68010, 9351, "Ancient Dragon Apostle's Cookbook [2]", "cookbook"),
        (68020, 9352, "Ancient Dragon Apostle's Cookbook [3]", "cookbook"),
        (68030, 9353, "Ancient Dragon Apostle's Cookbook [4]", "cookbook"),
        (68100, 9360, "Frenzied's Cookbook [1]", "cookbook"),
        (68110, 9361, "Frenzied's Cookbook [2]", "cookbook"),
        (68200, 9330, "Perfumer's Cookbook [1]", "cookbook"),
        (68210, 9331, "Perfumer's Cookbook [2]", "cookbook"),
        (68220, 9332, "Perfumer's Cookbook [3]", "cookbook"),
        (68230, 9333, "Perfumer's Cookbook [4]", "cookbook"),
    ],
}


def cmd_create(args):
    """Create a new verification case."""
    manager = CaseManager()

    hypothesis = None
    if args.offset is not None and args.bit is not None:
        hypothesis = FlagHypothesis(
            byte_offset=args.offset,
            bit_position=args.bit,
            implied_base=args.base,
        )

    case = manager.create_case(
        flag_id=args.flag,
        item_name=args.name,
        category=args.category,
        item_id=args.item_id,
        hypothesis=hypothesis,
    )

    print(f"Created case: {case.case_id}")
    print(f"  Flag: {case.flag_id}")
    print(f"  Name: {case.item_name}")
    print(f"  Category: {case.category}")
    if case.hypothesis:
        print(f"  Hypothesis: {case.hypothesis}")

    # Save case
    filepath = manager.save_case(case)
    print(f"\nSaved to: {filepath}")


def cmd_verify(args):
    """Run verification on a case."""
    manager = CaseManager()

    # Load case
    case_dir = PROJECT_ROOT / "scripts" / "verification" / "cases"
    case_files = list(case_dir.glob(f"{args.case_id}*.json"))

    if not case_files:
        print(f"Error: No case found matching '{args.case_id}'")
        return 1

    case = manager.load_case(case_files[0])
    print(f"Loaded case: {case.case_id}")

    # Run verification
    save_path = args.save or DEFAULT_SAVE

    # Determine slots
    slots_with = [int(s) for s in args.slots_with.split(",")] if args.slots_with else [0]
    slots_without = [int(s) for s in args.slots_without.split(",")] if args.slots_without else [1, 2, 3, 4]

    print(f"\nRunning verification...")
    print(f"  Save: {Path(save_path).name}")
    print(f"  Slots with item: {slots_with}")
    print(f"  Slots without item: {slots_without}")

    manager.run_full_verification(
        case,
        save_path,
        slots_with_item=slots_with,
        slots_without_item=slots_without,
        min_iterations=args.iterations,
    )

    # Print result
    print(f"\n{manager.get_case_report(case)}")

    # Save updated case
    filepath = manager.save_case(case)
    print(f"\nSaved to: {filepath}")


def cmd_list(args):
    """List all cases."""
    manager = CaseManager()
    cases = manager.load_all_cases()

    if not cases:
        print("No cases found.")
        return

    print(f"Found {len(cases)} cases:\n")
    print(f"{'Case ID':<35} {'Flag':<10} {'Status':<12} {'Confidence':<10} {'Name'}")
    print("-" * 90)

    for case in sorted(cases, key=lambda c: c.flag_id):
        print(f"{case.case_id:<35} {case.flag_id:<10} {case.status.value:<12} "
              f"{case.confidence:<10.2f} {case.item_name}")

    # Summary
    summary = manager.get_verification_summary()
    print(f"\nSummary:")
    for status, count in summary["by_status"].items():
        print(f"  {status}: {count}")


def cmd_report(args):
    """Show detailed report for a case."""
    manager = CaseManager()

    case_dir = PROJECT_ROOT / "scripts" / "verification" / "cases"
    case_files = list(case_dir.glob(f"{args.case_id}*.json"))

    if not case_files:
        print(f"Error: No case found matching '{args.case_id}'")
        return 1

    case = manager.load_case(case_files[0])
    print(manager.get_case_report(case))


def cmd_batch(args):
    """Run batch verification for a block."""
    manager = CaseManager()
    save_path = args.save or DEFAULT_SAVE

    block = args.block
    if block not in BLOCK_ITEMS:
        print(f"Error: No items defined for block {block}")
        print(f"Available blocks: {list(BLOCK_ITEMS.keys())}")
        return 1

    items = BLOCK_ITEMS[block]

    # Use explicit base if provided (non-default), otherwise look up from ground_truth
    gt_base = get_block_base(block)
    if args.base != 1341:
        base = args.base
        base_source = "(from --base argument)"
    elif gt_base is not None:
        base = gt_base
        base_source = "(from ground_truth_offsets.json)"
    else:
        base = args.base  # Fall back to default
        base_source = "(default - not in ground_truth)"

    print("=" * 70)
    print(f"BATCH VERIFICATION: Block {block}")
    print("=" * 70)
    print(f"Save: {Path(save_path).name}")
    print(f"Base offset: {base} {base_source}")
    print(f"Items to verify: {len(items)}")

    # Show catalog/anchor/cross-save status
    use_catalog = getattr(args, 'use_catalog', False)
    use_anchors = getattr(args, 'use_anchors', False)
    use_all_saves = getattr(args, 'all_saves', False)
    diff_set = getattr(args, 'differential_set', None)

    if use_catalog:
        coverage = manager.get_catalog_coverage()
        print(f"Catalog: {coverage['total_pairs']} pairs, {coverage['tagged_pairs']} tagged")
    if use_anchors:
        print(f"Anchors: enabled (will query anchor_database.json)")
    if use_all_saves:
        save_summary = manager.get_save_config_summary()
        print(f"Cross-save: {save_summary.get('saves', 0)} saves, {save_summary.get('total_slots', 0)} slots")

    # Collect related flags for alternative base search
    related_flags = [(flag_id, item_id) for flag_id, item_id, _, _ in items]
    known_flags = [(flag_id, name) for flag_id, _, name, _ in items]

    # Determine slots
    slots_with = [int(s) for s in args.slots_with.split(",")] if args.slots_with else [0]
    slots_without = [int(s) for s in args.slots_without.split(",")] if args.slots_without else [1, 2, 3, 4]

    results = {
        "verified": [],
        "partial": [],
        "rejected": [],
        "inconclusive": [],
    }

    # Track catalog/anchor usage
    temporal_hits = 0
    anchor_hits = 0

    # Track evidence gaps for feedback
    gaps = {
        "no_formula": [],      # Flags with no formula in ground_truth
        "no_inventory": [],    # Flags without item_id for inventory check
        "padding_detected": [],  # Flags landing in padding region
        "formula_update_proposals": [],  # Rejected cases with better alternatives
        "schema_filtered": [],  # Flags skipped due to schema-based allocation detection
    }

    # Schema-based pre-filtering
    use_schema_filter = getattr(args, 'schema_filter', False)
    untrackable_flags: set = set()

    if use_schema_filter:
        print(f"\nSchema filtering: enabled")
        schema = BlockSchema(block, base)
        extracted_path = PROJECT_ROOT / "scripts" / "extracted_event_flags.json"
        count = schema.load_flags_from_extracted(extracted_path)
        print(f"  Loaded {count} flags into schema")

        if count > 0:
            bitmap = schema.probe_allocation(save_path)
            untrackable_flags = set(bitmap.get_untrackable_flags())
            trackable_count = len(bitmap.get_trackable_flags())
            print(f"  Trackable: {trackable_count}, Untrackable (sparse gaps): {len(untrackable_flags)}")

            if untrackable_flags:
                # Pre-populate gaps with schema-detected untrackable flags
                for entry in bitmap.unallocated:
                    if entry.flag_id in [flag_id for flag_id, _, _, _ in items]:
                        gaps["schema_filtered"].append((entry.flag_id, entry.item_name))

    for flag_id, item_id, name, category in items:
        print(f"\n{'─' * 50}")
        print(f"Processing: {name} (flag {flag_id})")

        # Skip untrackable flags if schema filtering is enabled
        if use_schema_filter and flag_id in untrackable_flags:
            print(f"  SKIPPED: Flag is in sparse allocation gap (untrackable)")
            gaps["padding_detected"].append((flag_id, name))
            results["rejected"].append((flag_id, name, 0.0))
            continue

        # Create hypothesis
        byte_offset = base + (flag_id - block) // 8
        bit_position = 7 - (flag_id % 8)

        hypothesis = FlagHypothesis(
            byte_offset=byte_offset,
            bit_position=bit_position,
            implied_base=base,
            block_start=block,
        )

        case = manager.create_case(
            flag_id=flag_id,
            item_name=name,
            category=category,
            item_id=item_id,
            hypothesis=hypothesis,
        )

        # Run basic verification
        manager.run_full_verification(
            case,
            save_path,
            slots_with_item=slots_with,
            slots_without_item=slots_without,
            related_flags=related_flags,
            known_flags=known_flags,
            min_iterations=1,
        )

        # Additional defense: Auto-temporal from catalog
        if use_catalog:
            temporal_evidence = manager.defend_with_temporal_auto(case, slot_index=0)
            if temporal_evidence:
                temporal_hits += 1
                print(f"  Temporal: {len(temporal_evidence)} pair(s) found")

        # Additional defense: Chain anchors from anchor database
        if use_anchors:
            anchor_evidence = manager.defend_with_chain_anchor(case, save_path, slot_index=0)
            if anchor_evidence and anchor_evidence.supports_hypothesis:
                anchor_hits += 1
                print(f"  Anchors: {anchor_evidence.notes}")

        # Additional defense: Cross-save validation from all configured saves
        if use_all_saves:
            cross_save_evidence = manager.defend_with_cross_save_auto(case, differential_set=diff_set)
            if cross_save_evidence:
                supporting = sum(1 for e in cross_save_evidence if e.supports_hypothesis)
                print(f"  Cross-save: {supporting}/{len(cross_save_evidence)} supporting")

        # Categorize
        status_key = case.status.value
        if status_key in results:
            results[status_key].append((flag_id, name, case.confidence))
        else:
            results["inconclusive"].append((flag_id, name, case.confidence))

        print(f"  Status: {case.status.value.upper()}")
        print(f"  Confidence: {case.confidence:.2f}")

        # Track gaps for feedback
        if not item_id:
            gaps["no_inventory"].append((flag_id, name))
        if gt_base is None:
            gaps["no_formula"].append((flag_id, name))

        # Check for padding in challenges
        for challenge in case.challenges:
            if challenge.challenge_type == "padding_check" and challenge.disproves_hypothesis:
                gaps["padding_detected"].append((flag_id, name))
                break

        # Get formula update proposals for rejected cases
        if case.status == CaseStatus.REJECTED:
            proposal = manager.propose_formula_update(case)
            if proposal:
                gaps["formula_update_proposals"].append((flag_id, name, proposal))

        # Save case
        if args.save_cases:
            manager.save_case(case)

    # Print summary
    print("\n" + "=" * 70)
    print("BATCH SUMMARY")
    print("=" * 70)

    for status, batch_items in results.items():
        if batch_items:
            print(f"\n{status.upper()} ({len(batch_items)}):")
            for flag_id, name, confidence in batch_items:
                marker = "✓" if status == "verified" else "◐" if status == "partial" else "✗"
                print(f"  {marker} {flag_id}: {name} (confidence: {confidence:.2f})")

    # Statistics
    total = sum(len(v) for v in results.values())
    verified = len(results["verified"])
    partial = len(results["partial"])

    print(f"\n{'─' * 40}")
    print(f"Total: {total}")
    print(f"Verified: {verified} ({verified/total*100:.1f}%)")
    print(f"Partial: {partial} ({partial/total*100:.1f}%)")
    print(f"Verification rate: {(verified + partial)/total*100:.1f}%")

    # Catalog/anchor coverage report
    if use_catalog or use_anchors:
        print(f"\nEvidence coverage:")
        if use_catalog:
            print(f"  Temporal pairs found: {temporal_hits}/{total} ({temporal_hits/total*100:.1f}%)")
        if use_anchors:
            print(f"  Anchor matches found: {anchor_hits}/{total} ({anchor_hits/total*100:.1f}%)")

    # Evidence gap report
    has_gaps = any(len(v) > 0 for v in gaps.values())
    if has_gaps:
        print("\n" + "=" * 70)
        print("EVIDENCE GAPS")
        print("=" * 70)

        if gaps["no_formula"]:
            print(f"\nNo formula in ground_truth ({len(gaps['no_formula'])} items):")
            for flag_id, name in gaps["no_formula"][:5]:
                print(f"  - {flag_id}: {name}")
            if len(gaps["no_formula"]) > 5:
                print(f"  ... and {len(gaps['no_formula']) - 5} more")

        if gaps["no_inventory"]:
            print(f"\nNo item_id for inventory check ({len(gaps['no_inventory'])} items):")
            for flag_id, name in gaps["no_inventory"][:5]:
                print(f"  - {flag_id}: {name}")
            if len(gaps["no_inventory"]) > 5:
                print(f"  ... and {len(gaps['no_inventory']) - 5} more")

        if gaps["padding_detected"]:
            print(f"\nPadding detected ({len(gaps['padding_detected'])} items):")
            for flag_id, name in gaps["padding_detected"][:5]:
                print(f"  - {flag_id}: {name}")
            if len(gaps["padding_detected"]) > 5:
                print(f"  ... and {len(gaps['padding_detected']) - 5} more")

        if gaps["schema_filtered"]:
            print(f"\nSchema-filtered (sparse gaps) ({len(gaps['schema_filtered'])} items):")
            for flag_id, name in gaps["schema_filtered"][:5]:
                print(f"  - {flag_id}: {name}")
            if len(gaps["schema_filtered"]) > 5:
                print(f"  ... and {len(gaps['schema_filtered']) - 5} more")

        if gaps["formula_update_proposals"]:
            print(f"\nFormula update proposals ({len(gaps['formula_update_proposals'])} items):")
            for flag_id, name, proposal in gaps["formula_update_proposals"]:
                action = proposal.get("action", "unknown")
                if action == "update_block_base":
                    print(f"  - {flag_id}: {name}")
                    print(f"    Suggest: block {proposal.get('block')} base {proposal.get('current')} -> {proposal.get('proposed')}")
                    print(f"    Reason: {proposal.get('reason', 'N/A')}")
                elif action == "investigate_block":
                    print(f"  - {flag_id}: {name}")
                    print(f"    Action: Investigate block {proposal.get('block')}")
                    print(f"    Reason: {proposal.get('reason', 'N/A')}")


def cmd_discover(args):
    """Discover base offset for an unknown block."""
    from scripts.verification.save_parser import SaveParser
    from datetime import datetime
    import struct

    save_path = args.save or DEFAULT_SAVE
    block = args.block
    items = args.items  # Comma-separated: flag_id:item_id:name,...

    if not items:
        print("Error: --items required for discovery")
        print("Format: flag_id:item_id:name,flag_id:item_id:name,...")
        return 1

    # Parse items
    parsed_items = []
    for item_str in items.split(","):
        parts = item_str.strip().split(":")
        if len(parts) >= 2:
            flag_id = int(parts[0])
            item_id = int(parts[1])
            name = parts[2] if len(parts) > 2 else f"Item_{item_id}"
            parsed_items.append((flag_id, item_id, name))

    print("=" * 70)
    print(f"DISCOVER BASE OFFSET: Block {block}")
    print("=" * 70)
    print(f"Items for discovery: {len(parsed_items)}")

    # Load save
    parser = SaveParser()
    parsed = parser.parse(save_path)
    with open(save_path, 'rb') as f:
        raw_save = f.read()

    ef_s0 = parsed.slots[0].event_flags
    ef_s1 = parsed.slots[1].event_flags

    # Get slot raw data for inventory check
    s0_raw = raw_save[parsed.slots[0].slot_offset:parsed.slots[0].slot_offset + 2000000]
    s1_raw = raw_save[parsed.slots[1].slot_offset:parsed.slots[1].slot_offset + 2000000]

    def check_inventory(raw_slot, item_id):
        patterns = [
            struct.pack('<I', item_id),
            struct.pack('<I', 0x40000000 | (item_id & 0x0FFFFFFF)),
            struct.pack('<I', 0x20000000 | (item_id & 0x0FFFFFFF)),
        ]
        return any(p in raw_slot for p in patterns)

    # Find differential items
    differential = []
    for flag_id, item_id, name in parsed_items:
        in_s0 = check_inventory(s0_raw, item_id)
        in_s1 = check_inventory(s1_raw, item_id)
        if in_s0 and not in_s1:
            differential.append((flag_id, item_id, name))
            print(f"  Differential: {name} ({flag_id}) - in S0, not in S1")

    if not differential:
        print("\nNo differential items found. Need items present in S0 but absent in S1.")
        return 1

    print(f"\nSearching for base offset using {len(differential)} differential items...")

    # Search for best base
    search_start = args.search_start or 0
    search_end = args.search_end or min(100000, len(ef_s0))

    best_base = None
    best_matches = 0

    for test_base in range(search_start, search_end):
        matches = 0
        for flag_id, item_id, name in differential:
            byte_offset = test_base + (flag_id - block) // 8
            bit = 7 - (flag_id % 8)

            if byte_offset < 0 or byte_offset >= len(ef_s0):
                continue

            s0_byte = ef_s0[byte_offset]
            s1_byte = ef_s1[byte_offset]

            # Skip padding
            if s0_byte == 0xFF and s1_byte == 0xFF:
                continue

            s0_bit = (s0_byte >> bit) & 1
            s1_bit = (s1_byte >> bit) & 1

            if s0_bit == 1 and s1_bit == 0:
                matches += 1

        if matches > best_matches:
            best_matches = matches
            best_base = test_base

    if best_base is not None:
        print(f"\n{'=' * 50}")
        print(f"DISCOVERY RESULT")
        print(f"{'=' * 50}")
        print(f"Best base: {best_base}")
        print(f"Matches: {best_matches}/{len(differential)}")
        print(f"Match rate: {best_matches/len(differential)*100:.1f}%")

        # Verify each item at this base
        print(f"\nVerification at base {best_base}:")
        for flag_id, item_id, name in differential:
            byte_offset = best_base + (flag_id - block) // 8
            bit = 7 - (flag_id % 8)

            s0_byte = ef_s0[byte_offset]
            s1_byte = ef_s1[byte_offset]
            s0_bit = (s0_byte >> bit) & 1
            s1_bit = (s1_byte >> bit) & 1

            is_padding = (s0_byte == 0xFF and s1_byte == 0xFF)
            if is_padding:
                status = "PADDING"
            elif s0_bit == 1 and s1_bit == 0:
                status = "OK"
            else:
                status = f"MISMATCH (S0={s0_bit}, S1={s1_bit})"

            print(f"  {flag_id} ({name}): {status}")

        # Persist discovery result
        match_rate = best_matches / len(differential)
        discovery_result = {
            "block": block,
            "base_offset": best_base,
            "match_rate": match_rate,
            "matches": best_matches,
            "total_items": len(differential),
            "timestamp": datetime.now().isoformat(),
            "save_file": Path(save_path).name,
            "items_tested": [
                {"flag_id": f, "item_id": i, "name": n}
                for f, i, n in differential
            ],
        }

        cases_dir = PROJECT_ROOT / "scripts" / "verification" / "cases"
        cases_dir.mkdir(parents=True, exist_ok=True)
        output_file = cases_dir / f"discovery_{block}.json"

        with open(output_file, 'w') as f:
            json.dump(discovery_result, f, indent=2)

        print(f"\n{'─' * 50}")
        print(f"Discovery saved to: {output_file}")
        print(f"Next step: Verify with 'batch --block {block} --base {best_base}'")
    else:
        print("\nNo suitable base found.")


def main():
    parser = argparse.ArgumentParser(
        description="Case-Based Verification CLI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Create a case
  python case_cli.py create --flag 520000 --name "Lhutel" --category spirit_ash --item-id 258000

  # Verify a case
  python case_cli.py verify --case-id 520000 --save /path/to/save.sl2

  # Batch verify a block
  python case_cli.py batch --block 520000 --base 1341

  # Discover base for unknown block
  python case_cli.py discover --block 520000 --items "520000:258000:Lhutel,520030:5050:AssassinDagger"
        """
    )

    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # Create command
    create_parser = subparsers.add_parser("create", help="Create a new case")
    create_parser.add_argument("--flag", type=int, required=True, help="Flag ID")
    create_parser.add_argument("--name", required=True, help="Item/event name")
    create_parser.add_argument("--category", required=True, help="Category (spirit_ash, talisman, etc)")
    create_parser.add_argument("--item-id", type=int, help="Game item ID")
    create_parser.add_argument("--offset", type=int, help="Byte offset (optional)")
    create_parser.add_argument("--bit", type=int, help="Bit position (optional)")
    create_parser.add_argument("--base", type=int, help="Block base offset (optional)")

    # Verify command
    verify_parser = subparsers.add_parser("verify", help="Verify a case")
    verify_parser.add_argument("--case-id", required=True, help="Case ID (or prefix)")
    verify_parser.add_argument("--save", help="Save file path")
    verify_parser.add_argument("--slots-with", help="Comma-separated slot indices with item")
    verify_parser.add_argument("--slots-without", help="Comma-separated slot indices without item")
    verify_parser.add_argument("--iterations", type=int, default=2, help="Verification iterations")

    # List command
    list_parser = subparsers.add_parser("list", help="List all cases")

    # Report command
    report_parser = subparsers.add_parser("report", help="Show case report")
    report_parser.add_argument("--case-id", required=True, help="Case ID (or prefix)")

    # Batch command
    batch_parser = subparsers.add_parser("batch", help="Batch verify a block")
    batch_parser.add_argument("--block", type=int, required=True, help="Block start (e.g., 520000)")
    batch_parser.add_argument("--base", type=int, default=1341, help="Base offset")
    batch_parser.add_argument("--save", help="Save file path")
    batch_parser.add_argument("--slots-with", help="Comma-separated slot indices with item")
    batch_parser.add_argument("--slots-without", help="Comma-separated slot indices without item")
    batch_parser.add_argument("--save-cases", action="store_true", help="Save individual cases")
    batch_parser.add_argument("--use-catalog", action="store_true", help="Use capture catalog for auto-temporal defense")
    batch_parser.add_argument("--use-anchors", action="store_true", help="Use anchor database for chain anchor defense")
    batch_parser.add_argument("--all-saves", action="store_true", help="Run verification across all configured saves")
    batch_parser.add_argument("--differential-set", help="Differential set name from save_config.json")
    batch_parser.add_argument("--schema-filter", action="store_true", help="Pre-filter untrackable flags using schema-based allocation detection")

    # Discover command
    discover_parser = subparsers.add_parser("discover", help="Discover base offset for unknown block")
    discover_parser.add_argument("--block", type=int, required=True, help="Block start")
    discover_parser.add_argument("--items", help="Items to use: flag_id:item_id:name,...")
    discover_parser.add_argument("--save", help="Save file path")
    discover_parser.add_argument("--search-start", type=int, help="Search range start")
    discover_parser.add_argument("--search-end", type=int, help="Search range end")

    args = parser.parse_args()

    if args.command == "create":
        return cmd_create(args)
    elif args.command == "verify":
        return cmd_verify(args)
    elif args.command == "list":
        return cmd_list(args)
    elif args.command == "report":
        return cmd_report(args)
    elif args.command == "batch":
        return cmd_batch(args)
    elif args.command == "discover":
        return cmd_discover(args)
    else:
        parser.print_help()
        return 0


if __name__ == "__main__":
    sys.exit(main() or 0)
