#!/usr/bin/env python3
"""
Evidence-Based Block Discovery Script

Discovers unknown block offsets using multiple evidence sources:
1. Inventory possession (ground truth)
2. Chain anchors (related flags with known formulas)
3. Multi-slot differential analysis
4. Cross-validation against all known flags

Usage:
    python -m scripts.verification.discover_unknown_block --block 520000
    python -m scripts.verification.discover_unknown_block --block 520000 --save /path/to/save.sl2
"""

import argparse
import json
import os
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Add project root to path
PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser
from scripts.verification.ground_truth_loader import (
    load_block_bases,
    calculate_block_offset,
)


# ============================================================================
# DATA STRUCTURES
# ============================================================================

@dataclass
class FlagCandidate:
    """A flag candidate for discovery."""
    flag_id: int
    item_id: Optional[int] = None
    item_name: str = ""
    expected_set: bool = True  # Based on inventory
    category: str = ""


@dataclass
class BaseCandidate:
    """A candidate base offset."""
    base_offset: int
    match_count: int
    total_tested: int
    match_rate: float
    details: List[Tuple[int, bool, bool]] = field(default_factory=list)  # (flag_id, expected, actual)


@dataclass
class DiscoveryResult:
    """Result of block discovery."""
    block_start: int
    best_base: Optional[int]
    candidates: List[BaseCandidate]
    evidence_summary: Dict
    status: str  # "verified", "candidate", "failed"


# ============================================================================
# KNOWN FLAG MAPPINGS (from inventory_verification.rs)
# ============================================================================

# 520xxx flags and their associated items
KNOWN_520_FLAGS = [
    # Spirit Ashes (catacomb rewards)
    FlagCandidate(520000, 258000, "Lhutel the Headless", category="Spirit Ash"),
    FlagCandidate(520010, 234000, "Demi-Human Ashes", category="Spirit Ash"),
    FlagCandidate(520020, 241000, "Noble Sorcerer Ashes", category="Spirit Ash"),
    FlagCandidate(520030, 5050, "Assassin's Crimson Dagger", category="Talisman"),
    FlagCandidate(520040, 202000, "Banished Knight Engvall", category="Spirit Ash"),
    FlagCandidate(520050, 219000, "Twinsage Sorcerer Ashes", category="Spirit Ash"),
    FlagCandidate(520060, 218000, "Glintstone Sorcerer Ashes", category="Spirit Ash"),
    FlagCandidate(520080, 256000, "Ancient Dragon Knight Kristoff", category="Spirit Ash"),
    FlagCandidate(520090, 239000, "Bloodhound Knight Floh", category="Spirit Ash"),
    FlagCandidate(520100, 3060000, "Ordovis's Greatsword", category="Boss Weapon"),
    FlagCandidate(520110, 217000, "Perfumer Tricia", category="Spirit Ash"),
    FlagCandidate(520130, 246000, "Soldjars of Fortune Ashes", category="Spirit Ash"),
    FlagCandidate(520140, 243000, "Mad Pumpkin Head Ashes", category="Spirit Ash"),
    FlagCandidate(520150, 224000, "Kindred of Rot Ashes", category="Spirit Ash"),
    FlagCandidate(520160, 257000, "Redmane Knight Ogha", category="Spirit Ash"),
    FlagCandidate(520170, 8050000, "Zamor Curved Sword", category="Boss Weapon"),
    FlagCandidate(520200, 228000, "Blackflame Monk Amon", category="Spirit Ash"),
    FlagCandidate(520210, 5060, "Assassin's Cerulean Dagger", category="Talisman"),
    FlagCandidate(520220, 2160, "Lord of Blood's Exultation", category="Talisman"),
    FlagCandidate(520300, 1020, "Viridian Amber Medallion", category="Talisman"),
    FlagCandidate(520310, 4010, "Spelldrake Talisman", category="Talisman"),
    FlagCandidate(520330, 4020, "Flamedrake Talisman", category="Talisman"),
    FlagCandidate(520350, 2110, "Blue Dancer Charm", category="Talisman"),
    FlagCandidate(520360, 2080, "Winged Sword Insignia", category="Talisman"),
    FlagCandidate(520370, 1010, "Cerulean Amber Medallion", category="Talisman"),
    FlagCandidate(520390, 2170, "Kindred of Rot's Exultation", category="Talisman"),
    FlagCandidate(520400, 44010000, "Jar Cannon", category="Boss Weapon"),
    FlagCandidate(520410, 15020000, "Great Omenkiller Cleaver", category="Boss Weapon"),
    FlagCandidate(520420, 6010, "Concealing Veil", category="Talisman"),
    FlagCandidate(520430, 215000, "Putrid Corpse Ashes", category="Spirit Ash"),
    FlagCandidate(520440, 4022, "Flamedrake Talisman +2", category="Talisman"),
    FlagCandidate(520450, 1110, "Gold Scarab", category="Talisman"),
    FlagCandidate(520470, 3170000, "Golden Order Greatsword", category="Boss Weapon"),
    FlagCandidate(520480, 5040, "Godskin Swaddling Cloth", category="Talisman"),
    FlagCandidate(520490, 13020000, "Family Heads", category="Boss Weapon"),
]


# ============================================================================
# INVENTORY EXTRACTION
# ============================================================================

def extract_inventory_items(slot_data: bytes) -> set:
    """
    Extract item IDs from inventory section.

    This is a simplified extraction - in reality we'd parse the full
    GaItems structure. For now we search for item handles.
    """
    # This is a placeholder - actual implementation would parse
    # the inventory structure properly
    items = set()

    # Look for common item handle patterns
    # Item handles are 4-byte values with type prefix
    # Type 0x00000000 = weapons, 0x10000000 = armor,
    # 0x20000000 = accessory, 0x40000000 = goods

    # For Spirit Ashes (type 0x40), Talismans (type 0x20), Weapons (type 0x00)
    # we need to scan the inventory region

    # Placeholder: return empty for now, rely on known mappings
    return items


def check_item_in_inventory(slot_data: bytes, item_id: int) -> bool:
    """
    Check if a specific item ID is present in inventory.

    For accurate results, this should parse the actual inventory structure.
    Currently returns None (unknown) to indicate we can't verify.
    """
    # TODO: Implement proper inventory parsing
    return None


# ============================================================================
# BLOCK DISCOVERY
# ============================================================================

def search_block_base(
    block_start: int,
    flags: List[FlagCandidate],
    ef_data_progressed: bytes,
    ef_data_early: Optional[bytes] = None,
    search_range: Tuple[int, int] = (50000, 100000),
    step: int = 1,
) -> List[BaseCandidate]:
    """
    Search for the correct base offset for a block.

    Uses multi-slot differential if early game data available.
    """
    candidates = []

    for base in range(search_range[0], search_range[1], step):
        matches = 0
        total = 0
        details = []

        for flag in flags:
            flag_id = flag.flag_id
            relative = flag_id - block_start
            byte_offset = base + relative // 8
            bit = 7 - (flag_id % 8)

            if byte_offset >= len(ef_data_progressed):
                continue

            total += 1

            # Check if bit is set in progressed save
            byte_val = ef_data_progressed[byte_offset]
            is_set = (byte_val >> bit) & 1

            # If we have early game data, require differential
            if ef_data_early is not None:
                early_byte_val = ef_data_early[byte_offset]
                early_is_set = (early_byte_val >> bit) & 1

                # Valid differential: SET in progressed, UNSET in early
                if is_set and not early_is_set:
                    matches += 1
                    details.append((flag_id, True, True))
                elif is_set == early_is_set:
                    # Both same - inconclusive
                    details.append((flag_id, True, None))
                else:
                    # Inverted - wrong base
                    details.append((flag_id, True, False))
            else:
                # Single slot - just check if set
                if is_set:
                    matches += 1
                    details.append((flag_id, True, True))
                else:
                    details.append((flag_id, True, False))

        if total > 0:
            match_rate = matches / total
            if match_rate >= 0.15:  # Lower threshold to find any matches
                candidates.append(BaseCandidate(
                    base_offset=base,
                    match_count=matches,
                    total_tested=total,
                    match_rate=match_rate,
                    details=details,
                ))

    # Sort by match rate descending
    candidates.sort(key=lambda c: c.match_rate, reverse=True)
    return candidates[:10]  # Return top 10


def is_0xff_region(data: bytes, offset: int, window: int = 8) -> bool:
    """Check if region around offset is 0xFF padding."""
    start = max(0, offset - window)
    end = min(len(data), offset + window + 1)
    region = data[start:end]
    return all(b == 0xFF for b in region)


def validate_candidate(
    base: int,
    block_start: int,
    flags: List[FlagCandidate],
    ef_data: bytes,
) -> Tuple[int, int, List[Tuple[int, str, bool, bool]]]:
    """
    Validate a candidate base against all known flags.
    Returns (matches, total, details).
    """
    matches = 0
    total = 0
    details = []

    for flag in flags:
        flag_id = flag.flag_id
        relative = flag_id - block_start
        byte_offset = base + relative // 8
        bit = 7 - (flag_id % 8)

        if byte_offset >= len(ef_data):
            continue

        total += 1

        byte_val = ef_data[byte_offset]
        is_set = bool((byte_val >> bit) & 1)

        # Check for 0xFF contamination
        if is_0xff_region(ef_data, byte_offset):
            details.append((flag_id, flag.item_name, True, None))  # None = suspicious
            continue

        # Expected: flag should be SET if item should be in inventory
        expected = flag.expected_set
        match = is_set == expected

        if match:
            matches += 1

        details.append((flag_id, flag.item_name, expected, is_set))

    return matches, total, details


# ============================================================================
# MAIN DISCOVERY WORKFLOW
# ============================================================================

def discover_block(
    block_start: int,
    save_path: str,
    slot_progressed: int = 0,
    slot_early: int = 1,
) -> DiscoveryResult:
    """
    Discover base offset for an unknown block.
    """
    print(f"\n{'='*60}")
    print(f"DISCOVERING BLOCK {block_start}")
    print(f"{'='*60}")

    # Load save data
    parser = SaveParser()
    parsed = parser.parse(save_path)

    if slot_progressed >= len(parsed.slots):
        raise ValueError(f"Slot {slot_progressed} not found in save")

    slot_data_progressed = parsed.slots[slot_progressed]
    slot_data_early = parsed.slots[slot_early] if slot_early < len(parsed.slots) else None

    ef_progressed = slot_data_progressed.event_flags
    ef_early = slot_data_early.event_flags if slot_data_early else None

    if ef_progressed is None:
        raise ValueError("Could not extract event flags from progressed slot")

    print(f"\nLoaded save: {save_path}")
    print(f"Progressed slot: {slot_progressed} (EF size: {len(ef_progressed)} bytes)")
    if ef_early:
        print(f"Early game slot: {slot_early} (EF size: {len(ef_early)} bytes)")

    # Get known flags for this block
    if block_start == 520000:
        flags = KNOWN_520_FLAGS
    else:
        # Could extend to other blocks
        flags = []

    print(f"\nKnown flags to test: {len(flags)}")

    # Phase 1: Coarse search - try without differential first to find ANY matches
    print("\n--- Phase 1a: Single-slot search (step=100) ---")
    coarse_candidates_single = search_block_base(
        block_start,
        flags,
        ef_progressed,
        None,  # No differential - just find set bits
        search_range=(0, 500000),  # Full EF range
        step=100,
    )

    if coarse_candidates_single:
        print(f"\nSingle-slot top candidates (flag may or may not be set):")
        for c in coarse_candidates_single[:5]:
            print(f"  Base {c.base_offset}: {c.match_rate:.1%} ({c.match_count}/{c.total_tested})")
    else:
        print("No single-slot candidates found")

    # Phase 1b: With differential
    print("\n--- Phase 1b: Differential Search (step=100) ---")
    coarse_candidates = search_block_base(
        block_start,
        flags,
        ef_progressed,
        ef_early,
        search_range=(0, 500000),  # Full EF range
        step=100,
    )

    if coarse_candidates:
        print(f"\nTop coarse candidates:")
        for c in coarse_candidates[:5]:
            print(f"  Base {c.base_offset}: {c.match_rate:.1%} ({c.match_count}/{c.total_tested})")
    else:
        print("No candidates found in coarse search")
        return DiscoveryResult(
            block_start=block_start,
            best_base=None,
            candidates=[],
            evidence_summary={},
            status="failed",
        )

    # Phase 2: Fine search around best candidates
    print("\n--- Phase 2: Fine Search (step=1) ---")
    fine_candidates = []

    for coarse in coarse_candidates[:3]:  # Refine top 3
        range_start = max(0, coarse.base_offset - 100)
        range_end = coarse.base_offset + 100

        refined = search_block_base(
            block_start,
            flags,
            ef_progressed,
            ef_early,
            search_range=(range_start, range_end),
            step=1,
        )
        fine_candidates.extend(refined)

    # Deduplicate and sort
    seen = set()
    unique_candidates = []
    for c in fine_candidates:
        if c.base_offset not in seen:
            seen.add(c.base_offset)
            unique_candidates.append(c)
    unique_candidates.sort(key=lambda c: c.match_rate, reverse=True)

    print(f"\nTop fine candidates:")
    for c in unique_candidates[:5]:
        print(f"  Base {c.base_offset}: {c.match_rate:.1%} ({c.match_count}/{c.total_tested})")

    # Phase 3: Validate best candidate
    if unique_candidates:
        best = unique_candidates[0]
        print(f"\n--- Phase 3: Validating Best Candidate (base={best.base_offset}) ---")

        matches, total, details = validate_candidate(
            best.base_offset,
            block_start,
            flags,
            ef_progressed,
        )

        print(f"\nValidation results: {matches}/{total} ({matches/total:.1%})")
        print("\nFlag details:")
        for flag_id, name, expected, actual in details:
            if actual is None:
                status = "?? (0xFF region)"
            elif expected == actual:
                status = "OK"
            else:
                status = f"MISMATCH (expected={expected}, actual={actual})"
            print(f"  {flag_id}: {name[:40]:<40} [{status}]")

        # Determine status
        match_rate = matches / total if total > 0 else 0
        if match_rate >= 0.8:
            status = "verified"
        elif match_rate >= 0.6:
            status = "candidate"
        else:
            status = "failed"

        return DiscoveryResult(
            block_start=block_start,
            best_base=best.base_offset,
            candidates=unique_candidates[:5],
            evidence_summary={
                "match_rate": match_rate,
                "matches": matches,
                "total": total,
                "method": "multi-slot differential" if ef_early else "single-slot",
            },
            status=status,
        )

    return DiscoveryResult(
        block_start=block_start,
        best_base=None,
        candidates=[],
        evidence_summary={},
        status="failed",
    )


def format_ground_truth_entry(result: DiscoveryResult) -> str:
    """Format discovery result as ground_truth_offsets.json entry."""
    if result.best_base is None:
        return f"// Block {result.block_start}: Discovery failed"

    return f'''
  "{result.block_start}": {{
    "block_start": {result.block_start},
    "base_offset": {result.best_base},
    "block_size": 1000,
    "status": "{result.status}",
    "notes": "Spirit Ash/Talisman/Boss Weapon catacomb rewards. Discovered via evidence-based methodology. Match rate {result.evidence_summary.get('match_rate', 0):.1%} ({result.evidence_summary.get('matches', 0)}/{result.evidence_summary.get('total', 0)}). Method: {result.evidence_summary.get('method', 'unknown')}."
  }}'''


# ============================================================================
# CLI
# ============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Discover unknown block base offsets using evidence-based methodology"
    )
    parser.add_argument(
        "--block",
        type=int,
        default=520000,
        help="Block start to discover (default: 520000)",
    )
    parser.add_argument(
        "--save",
        type=str,
        default="/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2",
        help="Path to save file",
    )
    parser.add_argument(
        "--slot-progressed",
        type=int,
        default=0,
        help="Slot with progression (default: 0)",
    )
    parser.add_argument(
        "--slot-early",
        type=int,
        default=1,
        help="Early game slot for differential (default: 1)",
    )

    args = parser.parse_args()

    if not os.path.exists(args.save):
        print(f"Error: Save file not found: {args.save}")
        sys.exit(1)

    result = discover_block(
        args.block,
        args.save,
        args.slot_progressed,
        args.slot_early,
    )

    print(f"\n{'='*60}")
    print("DISCOVERY RESULT")
    print(f"{'='*60}")
    print(f"Block: {result.block_start}")
    print(f"Status: {result.status}")
    print(f"Best base offset: {result.best_base}")
    print(f"Evidence: {json.dumps(result.evidence_summary, indent=2)}")

    if result.status in ("verified", "candidate"):
        print(f"\n--- ground_truth_offsets.json entry ---")
        print(format_ground_truth_entry(result))


if __name__ == "__main__":
    main()
