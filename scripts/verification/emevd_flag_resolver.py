"""
EMEVD Flag Resolver

Resolves the actual event flag used for NPC/boss drops by cross-referencing EMEVD scripts.

Key Insight:
- ItemLotParam.param.xml contains item lots with getItemFlagId (e.g., 16007940)
- However, NPC drops don't set a separate "collected" flag
- Instead, they use an EMEVD event (e.g., 90005792) that:
  1. Waits for the "defeated" flag (e.g., 16000180)
  2. Awards the item via AwardItemsIncludingClients(itemLotId)

This means for NPC drops:
- The flag to check is the DEFEAT flag, not the ItemLotParam flag
- The ItemLotParam row_id/getItemFlagId is just for item lot lookup
- The actual flag stored in save data is from EMEVD

Usage:
    from verification.emevd_flag_resolver import EMEVDFlagResolver

    resolver = EMEVDFlagResolver()

    # Get the actual flag for Ghiza's Wheel (NPC drop)
    result = resolver.resolve_npc_drop_flag(item_lot_id=16000940)
    print(f"Actual flag: {result['defeat_flag']}")  # 16000180
"""

from __future__ import annotations

import re
from pathlib import Path
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple, Any


# Path to decompiled EMEVD files
EMEVD_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/event")


@dataclass
class NPCDropMapping:
    """Mapping from item lot to defeat flag for an NPC drop."""
    item_lot_id: int
    defeat_flag: int
    event_id: int  # e.g., 90005792
    source_file: str
    # Additional context
    intermediate_flags: List[int] = field(default_factory=list)
    npc_name: Optional[str] = None


@dataclass
class ResolverResult:
    """Result of resolving a flag via EMEVD."""
    original_flag_id: int
    resolved_flag_id: int
    resolution_type: str  # "npc_drop", "boss_drop", "direct", "unknown"
    mapping: Optional[NPCDropMapping] = None
    notes: str = ""


class EMEVDFlagResolver:
    """
    Resolves event flags for NPC/boss drops via EMEVD cross-reference.

    NPC invader drops use common event 90005792 pattern:
        $InitializeCommonEvent(0, 90005792, defeatFlag, flag1, flag2, checkFlag, itemLotId, extra)

    The defeat flag (first param after event ID) is what gets stored in save data.
    """

    # Known EMEVD event patterns for NPC drops
    NPC_DROP_EVENTS = {
        90005792: {
            "name": "NPC Invader Drop",
            "param_order": ["defeat_flag", "flag1", "flag2", "check_flag", "item_lot_id", "extra"],
        },
        90005790: {
            "name": "NPC Spawn Control",
            "param_order": ["extra", "defeat_flag", "flag1", "flag2", "check_flag", "count", "flag3", "flag4", "a", "b", "bool", "c"],
        },
        90005791: {
            "name": "NPC State Tracking",
            "param_order": ["defeat_flag", "flag1", "flag2", "check_flag"],
        },
    }

    def __init__(self, emevd_dir: Path = EMEVD_DIR):
        self.emevd_dir = emevd_dir
        self._npc_drop_cache: Dict[int, NPCDropMapping] = {}
        self._loaded = False

    def _load_npc_drops(self) -> None:
        """Parse EMEVD files to build NPC drop mappings."""
        if self._loaded:
            return

        # Pattern to match InitializeCommonEvent calls for event 90005792
        pattern = re.compile(
            r'\$InitializeCommonEvent\s*\(\s*\d+\s*,\s*90005792\s*,\s*'
            r'(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)'
        )

        for emevd_file in self.emevd_dir.glob("*.emevd.js"):
            try:
                content = emevd_file.read_text()
                for match in pattern.finditer(content):
                    defeat_flag = int(match.group(1))
                    flag1 = int(match.group(2))
                    flag2 = int(match.group(3))
                    check_flag = int(match.group(4))
                    item_lot_id = int(match.group(5))

                    if item_lot_id > 0:
                        mapping = NPCDropMapping(
                            item_lot_id=item_lot_id,
                            defeat_flag=defeat_flag,
                            event_id=90005792,
                            source_file=emevd_file.name,
                            intermediate_flags=[flag1, flag2, check_flag],
                        )
                        self._npc_drop_cache[item_lot_id] = mapping

            except Exception as e:
                print(f"Warning: Could not parse {emevd_file}: {e}")

        self._loaded = True

    def resolve_npc_drop_flag(self, item_lot_id: int) -> Optional[NPCDropMapping]:
        """
        Resolve the actual defeat flag for an NPC drop item lot.

        Args:
            item_lot_id: The item lot ID from ItemLotParam

        Returns:
            NPCDropMapping with defeat flag, or None if not found
        """
        self._load_npc_drops()
        return self._npc_drop_cache.get(item_lot_id)

    def resolve_flag(
        self,
        flag_id: int,
        category: Optional[str] = None
    ) -> ResolverResult:
        """
        Resolve a flag ID, checking if it needs EMEVD cross-reference.

        Args:
            flag_id: The flag ID to resolve
            category: Optional category hint (e.g., "Boss World Drop")

        Returns:
            ResolverResult with resolved flag and resolution type
        """
        result = ResolverResult(
            original_flag_id=flag_id,
            resolved_flag_id=flag_id,
            resolution_type="direct",
        )

        # Check if this is a known NPC drop item lot
        npc_mapping = self.resolve_npc_drop_flag(flag_id)
        if npc_mapping:
            result.resolved_flag_id = npc_mapping.defeat_flag
            result.resolution_type = "npc_drop"
            result.mapping = npc_mapping
            result.notes = f"NPC drop: use defeat flag {npc_mapping.defeat_flag} instead"
            return result

        # Check category hints
        if category == "Boss World Drop":
            result.resolution_type = "boss_drop"
            result.notes = "Boss drop may use defeat flag - check EMEVD"

        return result

    def get_all_npc_drops(self) -> Dict[int, NPCDropMapping]:
        """Get all discovered NPC drop mappings."""
        self._load_npc_drops()
        return self._npc_drop_cache.copy()

    def find_flag_for_item_lot(self, item_lot_id: int) -> Optional[int]:
        """
        Find the defeat flag that gates a specific item lot.

        This is the inverse lookup - given an item lot, find which flag
        must be set before the item is awarded.
        """
        self._load_npc_drops()
        mapping = self._npc_drop_cache.get(item_lot_id)
        return mapping.defeat_flag if mapping else None

    def get_mappings_for_area(self, area_id: int) -> List[NPCDropMapping]:
        """
        Get all NPC drop mappings for a specific area.

        Args:
            area_id: The dungeon area ID (e.g., 16 for Volcano Manor)

        Returns:
            List of NPCDropMapping for that area
        """
        self._load_npc_drops()

        area_mappings = []
        for mapping in self._npc_drop_cache.values():
            # Extract area from defeat flag (format: AASSSSII where AA=area)
            if mapping.defeat_flag >= 10_000_000:
                flag_area = mapping.defeat_flag // 1_000_000
                if flag_area == area_id:
                    area_mappings.append(mapping)

        return area_mappings


# Convenience functions

def resolve_npc_drop(item_lot_id: int) -> Optional[int]:
    """
    Quick lookup: get the defeat flag for an NPC drop.

    Returns the defeat flag ID, or None if not an NPC drop.
    """
    resolver = EMEVDFlagResolver()
    mapping = resolver.resolve_npc_drop_flag(item_lot_id)
    return mapping.defeat_flag if mapping else None


def is_npc_drop(item_lot_id: int) -> bool:
    """Check if an item lot is an NPC drop (needs EMEVD resolution)."""
    resolver = EMEVDFlagResolver()
    return resolver.resolve_npc_drop_flag(item_lot_id) is not None


if __name__ == "__main__":
    print("EMEVD Flag Resolver - NPC Drop Analysis")
    print("=" * 60)

    resolver = EMEVDFlagResolver()
    all_drops = resolver.get_all_npc_drops()

    print(f"\nFound {len(all_drops)} NPC drop mappings\n")

    # Group by area
    by_area: Dict[int, List[NPCDropMapping]] = {}
    for mapping in all_drops.values():
        if mapping.defeat_flag >= 10_000_000:
            area = mapping.defeat_flag // 1_000_000
            if area not in by_area:
                by_area[area] = []
            by_area[area].append(mapping)

    for area in sorted(by_area.keys()):
        mappings = by_area[area]
        print(f"\nArea {area} ({len(mappings)} NPC drops):")
        for m in mappings[:5]:
            print(f"  ItemLot {m.item_lot_id} -> DefeatFlag {m.defeat_flag}")
        if len(mappings) > 5:
            print(f"  ... and {len(mappings) - 5} more")

    # Specific test: Ghiza's Wheel
    print("\n" + "=" * 60)
    print("Ghiza's Wheel Resolution Test:")
    print("=" * 60)

    ghiza_lot = 16000940
    mapping = resolver.resolve_npc_drop_flag(ghiza_lot)
    if mapping:
        print(f"  Item Lot: {ghiza_lot}")
        print(f"  Defeat Flag: {mapping.defeat_flag}")
        print(f"  Source: {mapping.source_file}")
        print(f"  \nConclusion: To verify Ghiza's Wheel pickup, check flag {mapping.defeat_flag}")
    else:
        print(f"  Item Lot {ghiza_lot} not found in NPC drop mappings")
