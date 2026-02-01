#!/usr/bin/env python3
"""
Inventory-Driven Flag Discovery

KEY INSIGHT: The inventory IS ground truth evidence.
If an item is in inventory, its acquisition flag MUST be set somewhere.

Approach:
1. Parse inventory to find items present
2. For items with unknown flag formulas (520xxx), search entire EF section
3. Use multiple save files/slots as corroborating evidence
4. Record source references with each piece of evidence
"""

import os
import sys
import json
import struct
from pathlib import Path
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Set, Tuple
from collections import defaultdict

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser


# ============================================================================
# EVIDENCE TRACKING
# ============================================================================

@dataclass
class EvidenceSource:
    """Track where evidence came from."""
    save_file: str
    slot_index: int
    evidence_type: str  # "inventory", "flag_set", "manual_log", "differential"
    timestamp: str = ""

    def __str__(self):
        return f"{Path(self.save_file).name}:slot{self.slot_index}:{self.evidence_type}"


@dataclass
class ItemEvidence:
    """Evidence about an item's presence."""
    item_id: int
    item_name: str
    expected_flag: int
    present_in: List[EvidenceSource] = field(default_factory=list)
    absent_in: List[EvidenceSource] = field(default_factory=list)
    flag_candidates: List[Tuple[int, int, EvidenceSource]] = field(default_factory=list)  # (offset, bit, source)


@dataclass
class FlagCandidate:
    """A candidate location for a flag."""
    byte_offset: int
    bit: int
    set_in: List[EvidenceSource] = field(default_factory=list)
    unset_in: List[EvidenceSource] = field(default_factory=list)
    confidence: float = 0.0


# ============================================================================
# ITEM DATABASE
# ============================================================================

# Items that use 520xxx flags (from inventory_verification.rs)
ITEMS_520 = {
    # Spirit Ashes
    258000: ("Lhutel the Headless", 520000),
    234000: ("Demi-Human Ashes", 520010),
    241000: ("Noble Sorcerer Ashes", 520020),
    5050: ("Assassin's Crimson Dagger", 520030),
    202000: ("Banished Knight Engvall", 520040),
    219000: ("Twinsage Sorcerer Ashes", 520050),
    218000: ("Glintstone Sorcerer Ashes", 520060),
    256000: ("Ancient Dragon Knight Kristoff", 520080),
    239000: ("Bloodhound Knight Floh", 520090),
    3060000: ("Ordovis's Greatsword", 520100),
    217000: ("Perfumer Tricia", 520110),
    246000: ("Soldjars of Fortune Ashes", 520130),
    243000: ("Mad Pumpkin Head Ashes", 520140),
    224000: ("Kindred of Rot Ashes", 520150),
    257000: ("Redmane Knight Ogha", 520160),
    8050000: ("Zamor Curved Sword", 520170),
    228000: ("Blackflame Monk Amon", 520200),
    # Talismans
    5060: ("Assassin's Cerulean Dagger", 520210),
    2160: ("Lord of Blood's Exultation", 520220),
    1020: ("Viridian Amber Medallion", 520300),
    4010: ("Spelldrake Talisman", 520310),
    4020: ("Flamedrake Talisman", 520330),
    2110: ("Blue Dancer Charm", 520350),
    2080: ("Winged Sword Insignia", 520360),
    1010: ("Cerulean Amber Medallion", 520370),
    2170: ("Kindred of Rot's Exultation", 520390),
    44010000: ("Jar Cannon", 520400),
    15020000: ("Great Omenkiller Cleaver", 520410),
    6010: ("Concealing Veil", 520420),
    215000: ("Putrid Corpse Ashes", 520430),
    4022: ("Flamedrake Talisman +2", 520440),
    1110: ("Gold Scarab", 520450),
    3170000: ("Golden Order Greatsword", 520470),
    5040: ("Godskin Swaddling Cloth", 520480),
    13020000: ("Family Heads", 520490),
}


# ============================================================================
# INVENTORY PARSING
# ============================================================================

def extract_inventory_items(slot_data: bytes, ef_offset: int) -> Set[int]:
    """
    Extract item IDs from inventory section.

    Inventory is stored before the event flags section.
    GaItems structure contains item handles.
    """
    items = set()

    # The inventory/GaItems section is before EF offset
    # Search for item handle patterns
    # Item handles are 4-byte values: type_prefix | item_id
    # Types: 0x00 = weapon, 0x10 = armor, 0x20 = accessory, 0x40 = goods

    # Scan the region before EF for recognizable item handles
    search_start = max(0, ef_offset - 500000)  # Inventory is large
    search_end = ef_offset

    # Look for specific known item IDs
    for item_id in ITEMS_520.keys():
        # Item handles are stored with type prefix
        # For Spirit Ashes (goods): 0x40000000 | item_id
        # For Talismans (accessory): 0x20000000 | item_id
        # For Weapons: 0x00000000 | item_id

        if item_id >= 1000000:  # Weapons have high IDs
            handle = item_id  # Weapons use direct ID
        elif item_id >= 200000:  # Spirit ashes
            handle = 0x40000000 | item_id
        elif item_id < 10000:  # Talismans
            handle = 0x20000000 | item_id
        else:
            handle = 0x40000000 | item_id  # Default to goods

        # Search for this handle in the inventory region
        handle_bytes = struct.pack('<I', handle)
        pos = slot_data.find(handle_bytes, search_start, search_end)
        if pos != -1:
            items.add(item_id)

    return items


def extract_inventory_simple(slot_data: bytes) -> Set[int]:
    """
    Simple inventory extraction - look for item ID patterns.
    """
    items = set()

    # Search entire slot for known item IDs
    for item_id in ITEMS_520.keys():
        # Try different representations
        patterns = [
            struct.pack('<I', item_id),  # Direct
            struct.pack('<I', 0x40000000 | (item_id & 0x0FFFFFFF)),  # Goods type
            struct.pack('<I', 0x20000000 | (item_id & 0x0FFFFFFF)),  # Accessory type
        ]

        for pattern in patterns:
            if pattern in slot_data:
                items.add(item_id)
                break

    return items


# ============================================================================
# FLAG SEARCH
# ============================================================================

def search_for_flag_candidates(
    ef_data: bytes,
    flag_id: int,
    source: EvidenceSource,
    item_present: bool,
) -> List[FlagCandidate]:
    """
    Search for candidate locations where this flag might be stored.

    If item is present, the flag MUST be set somewhere.
    Search for bytes where the expected bit is set.
    """
    candidates = []
    block_start = 520000
    expected_bit = 7 - (flag_id % 8)

    for offset in range(len(ef_data)):
        byte_val = ef_data[offset]

        # Skip 0xFF regions (padding)
        if byte_val == 0xFF:
            continue

        bit_set = (byte_val >> expected_bit) & 1

        if item_present and bit_set:
            # Item is present AND bit is set - this is a candidate
            candidate = FlagCandidate(
                byte_offset=offset,
                bit=expected_bit,
            )
            candidate.set_in.append(source)
            candidates.append(candidate)
        elif not item_present and not bit_set:
            # Item is absent AND bit is unset - also interesting
            pass  # We'll track these separately

    return candidates


def calculate_implied_base(offset: int, flag_id: int, block_start: int = 520000) -> int:
    """Calculate what the block base would be if flag is at this offset."""
    relative = flag_id - block_start
    return offset - (relative // 8)


# ============================================================================
# MULTI-SOURCE DISCOVERY
# ============================================================================

class InventoryDrivenDiscovery:
    """Discovery engine that uses inventory as ground truth."""

    def __init__(self):
        self.evidence: Dict[int, ItemEvidence] = {}  # flag_id -> evidence
        self.base_candidates: Dict[int, int] = defaultdict(int)  # base -> vote count
        self.sources: List[EvidenceSource] = []

    def add_save_file(self, save_path: str, slots: List[int] = None):
        """Add a save file as evidence source."""
        parser = SaveParser()
        parsed = parser.parse(save_path)

        # Also read raw file for inventory extraction
        with open(save_path, 'rb') as f:
            raw_save = f.read()

        if slots is None:
            slots = range(len(parsed.slots))

        for slot_idx in slots:
            if slot_idx >= len(parsed.slots):
                continue

            slot = parsed.slots[slot_idx]
            if not slot.event_flags:
                continue

            source = EvidenceSource(
                save_file=save_path,
                slot_index=slot_idx,
                evidence_type="inventory+flags",
            )
            self.sources.append(source)

            # Extract inventory items from raw save data
            # Slot data starts at slot.slot_offset
            slot_start = slot.slot_offset
            slot_end = slot_start + 2000000  # Generous slot size
            slot_raw = raw_save[slot_start:min(slot_end, len(raw_save))]
            inventory_items = extract_inventory_simple(slot_raw)

            print(f"\n{'='*60}")
            print(f"Source: {source}")
            print(f"EF size: {len(slot.event_flags)} bytes")
            print(f"Found {len(inventory_items)} items from 520xxx database in slot")
            print(f"{'='*60}")

            # For each item with 520xxx flag
            for item_id, (item_name, flag_id) in ITEMS_520.items():
                if flag_id not in self.evidence:
                    self.evidence[flag_id] = ItemEvidence(
                        item_id=item_id,
                        item_name=item_name,
                        expected_flag=flag_id,
                    )

                evidence = self.evidence[flag_id]
                item_present = item_id in inventory_items

                if item_present:
                    evidence.present_in.append(source)
                    print(f"  PRESENT: {item_name} (item {item_id}) -> flag {flag_id}")

                    # Search for where this flag might be
                    candidates = search_for_flag_candidates(
                        slot.event_flags,
                        flag_id,
                        source,
                        item_present=True,
                    )

                    if candidates:
                        print(f"    Found {len(candidates)} candidate locations")
                        for c in candidates[:5]:
                            implied_base = calculate_implied_base(c.byte_offset, flag_id)
                            self.base_candidates[implied_base] += 1
                            evidence.flag_candidates.append((c.byte_offset, c.bit, source))
                else:
                    evidence.absent_in.append(source)

    def analyze_results(self):
        """Analyze collected evidence to find the block base."""
        print(f"\n{'='*60}")
        print("EVIDENCE ANALYSIS")
        print(f"{'='*60}")

        # Items with presence evidence
        items_present = [e for e in self.evidence.values() if e.present_in]
        items_absent = [e for e in self.evidence.values() if not e.present_in and e.absent_in]

        print(f"\nItems with presence evidence: {len(items_present)}")
        print(f"Items confirmed absent: {len(items_absent)}")

        if items_present:
            print("\nPresent items:")
            for e in items_present:
                sources = [str(s) for s in e.present_in]
                print(f"  {e.item_name} (flag {e.expected_flag})")
                print(f"    Sources: {', '.join(sources)}")
                print(f"    Candidate locations: {len(e.flag_candidates)}")

        # Analyze base candidates
        print(f"\n{'='*60}")
        print("BASE OFFSET CANDIDATES")
        print(f"{'='*60}")

        if self.base_candidates:
            sorted_bases = sorted(
                self.base_candidates.items(),
                key=lambda x: x[1],
                reverse=True
            )

            print("\nTop candidates by vote count:")
            for base, votes in sorted_bases[:20]:
                print(f"  Base {base}: {votes} votes")
        else:
            print("\nNo base candidates found.")
            print("This means either:")
            print("  1. No 520xxx items are present in any inventory")
            print("  2. The flag storage uses a non-standard formula")

        # Cross-reference: if item is present in one slot but absent in another,
        # we can use differential
        print(f"\n{'='*60}")
        print("DIFFERENTIAL OPPORTUNITIES")
        print(f"{'='*60}")

        for flag_id, e in self.evidence.items():
            if e.present_in and e.absent_in:
                print(f"\n{e.item_name} (flag {flag_id}):")
                print(f"  Present in: {[str(s) for s in e.present_in]}")
                print(f"  Absent in: {[str(s) for s in e.absent_in]}")
                print(f"  => Can use differential to pinpoint location!")

    def find_contradictions(self):
        """Find contradicting evidence that helps narrow down the formula."""
        print(f"\n{'='*60}")
        print("CONTRADICTION ANALYSIS")
        print(f"{'='*60}")

        # For each item present in inventory, the flag MUST be set
        # If we can't find it at the expected location, that's a contradiction

        contradictions = []

        for flag_id, e in self.evidence.items():
            if e.present_in:
                # Item is in inventory - flag MUST be set somewhere
                if not e.flag_candidates:
                    contradictions.append({
                        'type': 'no_flag_found',
                        'item': e.item_name,
                        'flag_id': flag_id,
                        'sources': e.present_in,
                        'note': 'Item in inventory but no flag candidate found',
                    })
                else:
                    # Check if candidates are consistent across sources
                    offsets = set(c[0] for c in e.flag_candidates)
                    if len(offsets) > 5:  # Too many candidates
                        contradictions.append({
                            'type': 'ambiguous',
                            'item': e.item_name,
                            'flag_id': flag_id,
                            'candidate_count': len(offsets),
                            'note': 'Too many candidate locations - need more evidence',
                        })

        if contradictions:
            print("\nContradictions found:")
            for c in contradictions:
                print(f"  [{c['type']}] {c['item']} (flag {c['flag_id']})")
                print(f"    {c['note']}")
        else:
            print("\nNo contradictions found - evidence is consistent.")


# ============================================================================
# MAIN
# ============================================================================

def main():
    # Find all save files
    save_dir = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
    snapshot_dir = save_dir / "Granular snapshots for debugging"

    discovery = InventoryDrivenDiscovery()

    # Add main save files
    for save_file in save_dir.glob("*.sl2"):
        print(f"\nProcessing: {save_file.name}")
        discovery.add_save_file(str(save_file), slots=[0, 1, 2, 3, 4])

    # Add snapshot files if they exist
    if snapshot_dir.exists():
        snapshots = list(snapshot_dir.glob("*.sl2"))[:5]  # First 5 for now
        for save_file in snapshots:
            print(f"\nProcessing snapshot: {save_file.name}")
            discovery.add_save_file(str(save_file), slots=[0])

    # Analyze results
    discovery.analyze_results()
    discovery.find_contradictions()


if __name__ == "__main__":
    main()
