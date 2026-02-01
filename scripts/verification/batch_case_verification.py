#!/usr/bin/env python3
"""
Batch Case Verification - Run case-based verification on multiple flags.

Demonstrates the full case lifecycle with defense and challenge phases.
"""

import sys
from pathlib import Path
from typing import List, Tuple

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.case_manager import (
    CaseManager,
    FlagHypothesis,
    CaseStatus,
)


# Items to verify with their flag IDs and item IDs
ITEMS_TO_VERIFY = [
    # Spirit Ashes (520xxx block)
    (520000, 258000, "Lhutel the Headless", "spirit_ash"),
    (520030, 5050, "Assassin's Crimson Dagger", "talisman"),
    (520040, 202000, "Banished Knight Engvall", "spirit_ash"),
    (520050, 219000, "Twinsage Sorcerer Ashes", "spirit_ash"),
    (520090, 239000, "Bloodhound Knight Floh", "spirit_ash"),
    (520110, 217000, "Perfumer Tricia", "spirit_ash"),
    (520210, 5060, "Assassin's Cerulean Dagger", "talisman"),  # Known padding issue
    (520300, 1020, "Viridian Amber Medallion", "talisman"),
    (520310, 4010, "Spelldrake Talisman", "talisman"),
    (520330, 4020, "Flamedrake Talisman", "talisman"),  # Known padding issue
    (520350, 2110, "Blue Dancer Charm", "talisman"),
    (520370, 1010, "Cerulean Amber Medallion", "talisman"),
    (520390, 2170, "Kindred of Rot's Exultation", "talisman"),
    (520450, 1110, "Gold Scarab", "talisman"),  # Known padding issue
    (520480, 5040, "Godskin Swaddling Cloth", "talisman"),
]


def run_batch_verification():
    """Run case-based verification on all items."""
    print("=" * 80)
    print("BATCH CASE VERIFICATION")
    print("=" * 80)

    manager = CaseManager()
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    # Base for 520xxx block
    base = 1341

    # Collect related flags for alternative base challenge
    related_flags = [(flag_id, item_id) for flag_id, item_id, _, _ in ITEMS_TO_VERIFY]

    results = {
        "verified": [],
        "partial": [],
        "rejected": [],
        "inconclusive": [],
    }

    for flag_id, item_id, name, category in ITEMS_TO_VERIFY:
        print(f"\n{'─' * 70}")
        print(f"Processing: {name} (flag {flag_id})")
        print(f"{'─' * 70}")

        # Create hypothesis
        byte_offset = base + (flag_id - 520000) // 8
        bit_position = 7 - (flag_id % 8)

        hypothesis = FlagHypothesis(
            byte_offset=byte_offset,
            bit_position=bit_position,
            implied_base=base,
            block_start=520000,
        )

        # Create case
        case = manager.create_case(
            flag_id=flag_id,
            item_name=name,
            category=category,
            item_id=item_id,
            hypothesis=hypothesis,
        )

        # Run defense phase
        manager.run_defense_phase(
            case,
            save_path,
            slots_with_item=[0],  # Mid-game character
            slots_without_item=[1, 2, 3, 4],  # Early-game characters
        )

        # Run challenge phase
        manager.run_challenge_phase(case, save_path, related_flags)

        # Categorize result
        if case.status == CaseStatus.VERIFIED:
            results["verified"].append((flag_id, name, case.confidence))
            status_str = "VERIFIED"
        elif case.status == CaseStatus.PARTIAL:
            results["partial"].append((flag_id, name, case.confidence))
            status_str = "PARTIAL"
        elif case.status == CaseStatus.REJECTED:
            results["rejected"].append((flag_id, name, case.confidence))
            status_str = "REJECTED"
        else:
            results["inconclusive"].append((flag_id, name, case.confidence))
            status_str = "INCONCLUSIVE"

        # Print summary
        evidence_support = len([e for e in case.evidence if e.supports_hypothesis])
        evidence_oppose = len([e for e in case.evidence if not e.supports_hypothesis])
        challenges_survived = case.surviving_challenges
        challenges_total = len(case.challenges)

        print(f"  Status: {status_str}")
        print(f"  Confidence: {case.confidence:.2f}")
        print(f"  Evidence: {evidence_support} supporting, {evidence_oppose} opposing")
        print(f"  Challenges: {challenges_survived}/{challenges_total} survived")

        # Show challenge details if failed
        failed_challenges = [c for c in case.challenges if c.disproves_hypothesis]
        if failed_challenges:
            print("  Failed challenges:")
            for ch in failed_challenges:
                print(f"    - {ch.challenge_type}: {ch.notes}")

    # Print summary
    print("\n" + "=" * 80)
    print("VERIFICATION SUMMARY")
    print("=" * 80)

    print(f"\nVERIFIED ({len(results['verified'])}):")
    for flag_id, name, confidence in results["verified"]:
        print(f"  ✓ {flag_id}: {name} (confidence: {confidence:.2f})")

    print(f"\nPARTIAL ({len(results['partial'])}):")
    for flag_id, name, confidence in results["partial"]:
        print(f"  ◐ {flag_id}: {name} (confidence: {confidence:.2f})")

    print(f"\nREJECTED ({len(results['rejected'])}):")
    for flag_id, name, confidence in results["rejected"]:
        print(f"  ✗ {flag_id}: {name} (confidence: {confidence:.2f})")

    print(f"\nINCONCLUSIVE ({len(results['inconclusive'])}):")
    for flag_id, name, confidence in results["inconclusive"]:
        print(f"  ? {flag_id}: {name} (confidence: {confidence:.2f})")

    # Calculate overall statistics
    total = len(ITEMS_TO_VERIFY)
    verified = len(results["verified"])
    partial = len(results["partial"])
    rejected = len(results["rejected"])

    print("\n" + "-" * 40)
    print("STATISTICS:")
    print(f"  Total items: {total}")
    print(f"  Verified: {verified} ({verified/total*100:.1f}%)")
    print(f"  Partial: {partial} ({partial/total*100:.1f}%)")
    print(f"  Rejected: {rejected} ({rejected/total*100:.1f}%)")
    print(f"  Verification rate: {(verified + partial)/total*100:.1f}%")

    # Identify padding gap patterns
    print("\n" + "-" * 40)
    print("PADDING GAP ANALYSIS:")

    rejected_flags = [flag_id for flag_id, _, _ in results["rejected"]]
    if rejected_flags:
        print(f"  Flags in padding gaps: {rejected_flags}")

        # Analyze pattern
        gap_offsets = []
        for flag_id in rejected_flags:
            rel_byte = (flag_id - 520000) // 8
            gap_offsets.append(rel_byte)

        print(f"  Relative byte offsets: {gap_offsets}")
        print(f"  These offsets map to 0xFF padding regions in the block structure.")

    return results


if __name__ == "__main__":
    run_batch_verification()
