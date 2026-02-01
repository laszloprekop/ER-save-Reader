#!/usr/bin/env python3
"""
Case Analysis Extensions

Addresses:
1. Confidence normalization (diminishing returns for repeated evidence types)
2. Blindspot/coverage analysis
3. Unknown region tracking
4. Lookup table discovery

These extensions improve the reliability of the case-based verification system.
"""

import struct
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser
from scripts.verification.ground_truth_loader import load_block_bases


# =============================================================================
# NORMALIZED CONFIDENCE SCORING
# =============================================================================

@dataclass
class NormalizedConfidence:
    """
    Confidence scoring with diminishing returns.

    Prevents score inflation from repeated evidence of the same type.
    """

    # Base weights (first piece of evidence of each type)
    BASE_WEIGHTS = {
        "inventory_present": 0.30,
        "flag_detected": 0.25,
        "manual_completion": 0.20,
        "cross_slot_differential": 0.15,
        "chain_anchor": 0.10,
        "temporal_consistency": 0.10,
        "cross_save": 0.10,
        "formula_consistency": 0.05,
    }

    # Diminishing factor for repeated evidence (each subsequent = factor * previous)
    DIMINISHING_FACTOR = 0.5

    # Maximum contribution per evidence type (cap)
    MAX_PER_TYPE = {
        "inventory_present": 0.35,      # Can't exceed 0.35 total from inventory checks
        "cross_slot_differential": 0.25, # Can't exceed 0.25 from differentials
        "cross_save": 0.20,              # Can't exceed 0.20 from cross-save
        "chain_anchor": 0.15,            # Can't exceed 0.15 from anchors
        "temporal_consistency": 0.15,
    }

    evidence_counts: Dict[str, int] = field(default_factory=lambda: defaultdict(int))
    evidence_contributions: Dict[str, float] = field(default_factory=lambda: defaultdict(float))

    def add_evidence(self, evidence_type: str, supports: bool) -> float:
        """
        Add evidence and return the contribution with diminishing returns.

        Args:
            evidence_type: Type of evidence
            supports: Whether it supports the hypothesis

        Returns:
            The actual confidence contribution (may be less than base weight)
        """
        base_weight = self.BASE_WEIGHTS.get(evidence_type, 0.05)
        count = self.evidence_counts[evidence_type]

        # Apply diminishing returns
        contribution = base_weight * (self.DIMINISHING_FACTOR ** count)

        # Check cap
        max_cap = self.MAX_PER_TYPE.get(evidence_type, 0.50)
        current_total = self.evidence_contributions[evidence_type]

        if current_total + contribution > max_cap:
            contribution = max(0, max_cap - current_total)

        # Update tracking
        self.evidence_counts[evidence_type] += 1
        if supports:
            self.evidence_contributions[evidence_type] += contribution
            return contribution
        else:
            # Negative evidence has less diminishing (stays impactful)
            neg_contribution = contribution * 0.7
            self.evidence_contributions[evidence_type] -= neg_contribution
            return -neg_contribution

    def get_total_confidence(self) -> float:
        """Get total confidence score."""
        total = sum(self.evidence_contributions.values())
        return max(0.0, min(1.0, total))

    def get_breakdown(self) -> Dict[str, Any]:
        """Get detailed breakdown of confidence contributions."""
        return {
            "total": self.get_total_confidence(),
            "by_type": dict(self.evidence_contributions),
            "counts": dict(self.evidence_counts),
        }


# =============================================================================
# BLINDSPOT / COVERAGE ANALYSIS
# =============================================================================

@dataclass
class RegionInfo:
    """Information about a data region."""
    start_offset: int
    end_offset: int
    size: int
    is_padding: bool
    byte_pattern: str  # "data", "0xFF", "0x00", "mixed"
    flags_discovered: int = 0
    flags_verified: int = 0


@dataclass
class BlockCoverage:
    """Coverage analysis for a single block."""
    block_start: int
    base_offset: int
    expected_size: int  # Expected bytes for the block

    # Region breakdown
    data_regions: List[RegionInfo] = field(default_factory=list)
    padding_regions: List[RegionInfo] = field(default_factory=list)

    # Coverage stats
    total_bytes: int = 0
    data_bytes: int = 0
    padding_bytes: int = 0

    # Flag coverage
    expected_flags: int = 0
    discovered_flags: int = 0
    verified_flags: int = 0
    padding_flags: int = 0  # Flags that land in padding

    def get_data_coverage(self) -> float:
        """Percentage of block that is actual data (not padding)."""
        if self.total_bytes == 0:
            return 0.0
        return self.data_bytes / self.total_bytes

    def get_flag_coverage(self) -> float:
        """Percentage of expected flags that are verified."""
        if self.expected_flags == 0:
            return 0.0
        return self.verified_flags / self.expected_flags


class CoverageAnalyzer:
    """
    Analyzes coverage and blindspots in the event flags section.
    """

    def __init__(self):
        self.parser = SaveParser()
        self.block_bases = load_block_bases()

    def analyze_block_coverage(
        self,
        ef_data: bytes,
        block_start: int,
        base_offset: int,
        block_size: int = 1000,
    ) -> BlockCoverage:
        """
        Analyze coverage for a single block.

        Args:
            ef_data: Event flags section bytes
            block_start: Block's flag ID start (e.g., 520000)
            base_offset: Block's base byte offset
            block_size: Number of flags in block

        Returns:
            BlockCoverage with detailed analysis
        """
        bytes_needed = (block_size + 7) // 8
        coverage = BlockCoverage(
            block_start=block_start,
            base_offset=base_offset,
            expected_size=bytes_needed,
            expected_flags=block_size,
        )

        # Analyze each byte in the block
        current_region_start = base_offset
        current_is_padding = None
        data_bytes = 0
        padding_bytes = 0

        for i in range(bytes_needed):
            offset = base_offset + i
            if offset >= len(ef_data):
                break

            byte_val = ef_data[offset]
            is_padding = (byte_val == 0xFF)

            # Detect region transitions
            if current_is_padding is None:
                current_is_padding = is_padding
            elif is_padding != current_is_padding:
                # Region ended
                region = RegionInfo(
                    start_offset=current_region_start,
                    end_offset=offset - 1,
                    size=offset - current_region_start,
                    is_padding=current_is_padding,
                    byte_pattern="0xFF" if current_is_padding else "data",
                )
                if current_is_padding:
                    coverage.padding_regions.append(region)
                    padding_bytes += region.size
                else:
                    coverage.data_regions.append(region)
                    data_bytes += region.size

                current_region_start = offset
                current_is_padding = is_padding

            if is_padding:
                padding_bytes += 1
            else:
                data_bytes += 1

        # Close final region
        if current_is_padding is not None:
            final_offset = min(base_offset + bytes_needed, len(ef_data))
            region = RegionInfo(
                start_offset=current_region_start,
                end_offset=final_offset - 1,
                size=final_offset - current_region_start,
                is_padding=current_is_padding,
                byte_pattern="0xFF" if current_is_padding else "data",
            )
            if current_is_padding:
                coverage.padding_regions.append(region)
            else:
                coverage.data_regions.append(region)

        coverage.total_bytes = bytes_needed
        coverage.data_bytes = data_bytes
        coverage.padding_bytes = padding_bytes

        return coverage

    def find_unknown_regions(
        self,
        ef_data: bytes,
        scan_range: Tuple[int, int] = None,
    ) -> List[RegionInfo]:
        """
        Find regions with data that don't belong to known blocks.

        These are potential new blocks or unknown data structures.
        """
        if scan_range is None:
            scan_range = (0, min(100000, len(ef_data)))

        start, end = scan_range

        # Build map of known regions
        known_ranges = set()
        for block_start, info in self.block_bases.items():
            base = info["base_offset"]
            size = info.get("block_size", 1000)
            bytes_needed = (size + 7) // 8
            for offset in range(base, base + bytes_needed):
                known_ranges.add(offset)

        # Find unknown data regions
        unknown_regions = []
        in_unknown = False
        unknown_start = 0

        for offset in range(start, end):
            if offset >= len(ef_data):
                break

            byte_val = ef_data[offset]
            is_data = (byte_val != 0xFF and byte_val != 0x00)
            is_known = offset in known_ranges

            if is_data and not is_known:
                if not in_unknown:
                    in_unknown = True
                    unknown_start = offset
            else:
                if in_unknown:
                    # Unknown region ended
                    region = RegionInfo(
                        start_offset=unknown_start,
                        end_offset=offset - 1,
                        size=offset - unknown_start,
                        is_padding=False,
                        byte_pattern="unknown_data",
                    )
                    if region.size >= 4:  # Only track significant regions
                        unknown_regions.append(region)
                    in_unknown = False

        return unknown_regions

    def correlate_with_inventory(
        self,
        ef_data: bytes,
        inventory_items: List[Tuple[int, int, str]],  # [(flag_id, item_id, name), ...]
        block_start: int,
        base_offset: int,
    ) -> Dict[str, Any]:
        """
        Correlate block regions with inventory items.

        Returns analysis of how well the block structure matches inventory.
        """
        analysis = {
            "total_items": len(inventory_items),
            "items_in_data_regions": 0,
            "items_in_padding_regions": 0,
            "items_by_region": defaultdict(list),
        }

        for flag_id, item_id, name in inventory_items:
            byte_offset = base_offset + (flag_id - block_start) // 8

            if byte_offset >= len(ef_data):
                continue

            byte_val = ef_data[byte_offset]
            is_padding = (byte_val == 0xFF)

            if is_padding:
                analysis["items_in_padding_regions"] += 1
                analysis["items_by_region"]["padding"].append((flag_id, name))
            else:
                analysis["items_in_data_regions"] += 1
                analysis["items_by_region"]["data"].append((flag_id, name))

        return analysis


# =============================================================================
# UNKNOWN BASE TRACKER
# =============================================================================

@dataclass
class UnknownBase:
    """Tracks a potential unknown base offset."""
    implied_base: int
    supporting_flags: List[int] = field(default_factory=list)
    evidence_sources: List[str] = field(default_factory=list)
    match_count: int = 0
    confidence: float = 0.0
    notes: str = ""


class UnknownBaseTracker:
    """
    Tracks potential unknown base offsets discovered during verification.

    When cases are rejected or inconclusive, this tracker looks for patterns
    that might indicate undiscovered blocks.
    """

    def __init__(self):
        self.unknown_bases: Dict[int, UnknownBase] = {}
        self.rejected_flags: List[Tuple[int, str, int]] = []  # (flag_id, name, attempted_base)

    def record_rejected_flag(
        self,
        flag_id: int,
        name: str,
        attempted_base: int,
        rejection_reason: str,
    ):
        """Record a flag that was rejected during verification."""
        self.rejected_flags.append((flag_id, name, attempted_base))

    def search_for_patterns(
        self,
        ef_data: bytes,
        rejected_flags: List[Tuple[int, int, str]],  # [(flag_id, item_id, name), ...]
        search_range: Tuple[int, int] = (0, 10000),
    ) -> List[UnknownBase]:
        """
        Search for patterns among rejected flags that might indicate new bases.
        """
        candidates = defaultdict(list)

        for flag_id, item_id, name in rejected_flags:
            expected_bit = 7 - (flag_id % 8)

            # Search for where this flag COULD be
            for offset in range(search_range[0], min(search_range[1], len(ef_data))):
                byte_val = ef_data[offset]

                if byte_val == 0xFF:
                    continue  # Skip padding

                bit_set = (byte_val >> expected_bit) & 1

                if bit_set:
                    # This offset has the expected bit set
                    # Calculate what base this would imply
                    block_start = (flag_id // 1000) * 1000
                    implied_base = offset - (flag_id - block_start) // 8

                    if implied_base >= 0:
                        candidates[implied_base].append((flag_id, name))

        # Find bases with multiple supporting flags
        result = []
        for implied_base, flags in candidates.items():
            if len(flags) >= 2:  # At least 2 flags support this base
                unknown = UnknownBase(
                    implied_base=implied_base,
                    supporting_flags=[f[0] for f in flags],
                    match_count=len(flags),
                    confidence=min(0.5, len(flags) * 0.1),
                    notes=f"Supported by {len(flags)} rejected flags",
                )
                result.append(unknown)
                self.unknown_bases[implied_base] = unknown

        return sorted(result, key=lambda x: x.match_count, reverse=True)


# =============================================================================
# LOOKUP TABLE DISCOVERY
# =============================================================================

@dataclass
class PotentialLookupEntry:
    """A potential entry in a lookup table."""
    table_offset: int
    stored_value: int
    interpreted_as: str  # "offset", "flag_id", "unknown"
    matches_known: bool
    matched_block: Optional[int] = None


class LookupTableDiscovery:
    """
    Discovers potential offset lookup tables in the save file.

    Some games store offset addresses directly rather than using formulas.
    This class searches for such patterns.
    """

    def __init__(self):
        self.block_bases = load_block_bases()
        self.known_offsets = set()

        # Build set of known offsets
        for block_start, info in self.block_bases.items():
            self.known_offsets.add(info["base_offset"])

    def search_for_offset_table(
        self,
        raw_data: bytes,
        search_range: Tuple[int, int],
        pointer_size: int = 4,  # 4 bytes for 32-bit offsets
    ) -> List[PotentialLookupEntry]:
        """
        Search for regions that might contain stored offsets.

        Looks for sequences of values that match known block offsets.
        """
        entries = []
        start, end = search_range

        for offset in range(start, end - pointer_size):
            # Read potential offset value
            if pointer_size == 4:
                value = struct.unpack('<I', raw_data[offset:offset + 4])[0]
            elif pointer_size == 2:
                value = struct.unpack('<H', raw_data[offset:offset + 2])[0]
            else:
                continue

            # Check if this matches a known offset
            if value in self.known_offsets:
                matched_block = None
                for block_start, info in self.block_bases.items():
                    if info["base_offset"] == value:
                        matched_block = block_start
                        break

                entries.append(PotentialLookupEntry(
                    table_offset=offset,
                    stored_value=value,
                    interpreted_as="offset",
                    matches_known=True,
                    matched_block=matched_block,
                ))

            # Check if it could be a reasonable offset (within EF section range)
            elif 0 < value < 1000000:
                entries.append(PotentialLookupEntry(
                    table_offset=offset,
                    stored_value=value,
                    interpreted_as="potential_offset",
                    matches_known=False,
                ))

        return entries

    def find_offset_clusters(
        self,
        entries: List[PotentialLookupEntry],
        max_gap: int = 16,
    ) -> List[List[PotentialLookupEntry]]:
        """
        Find clusters of potential lookup entries.

        Lookup tables typically have entries close together.
        """
        if not entries:
            return []

        # Sort by table offset
        sorted_entries = sorted(entries, key=lambda x: x.table_offset)

        clusters = []
        current_cluster = [sorted_entries[0]]

        for entry in sorted_entries[1:]:
            if entry.table_offset - current_cluster[-1].table_offset <= max_gap:
                current_cluster.append(entry)
            else:
                if len(current_cluster) >= 3:  # Minimum 3 entries for a cluster
                    clusters.append(current_cluster)
                current_cluster = [entry]

        if len(current_cluster) >= 3:
            clusters.append(current_cluster)

        return clusters

    def analyze_cluster(
        self,
        cluster: List[PotentialLookupEntry],
    ) -> Dict[str, Any]:
        """Analyze a cluster of potential lookup entries."""
        known_matches = [e for e in cluster if e.matches_known]

        return {
            "start_offset": cluster[0].table_offset,
            "end_offset": cluster[-1].table_offset,
            "entry_count": len(cluster),
            "known_matches": len(known_matches),
            "matched_blocks": [e.matched_block for e in known_matches if e.matched_block],
            "confidence": len(known_matches) / len(cluster) if cluster else 0,
            "entries": [
                {
                    "offset": e.table_offset,
                    "value": e.stored_value,
                    "matches_known": e.matches_known,
                    "matched_block": e.matched_block,
                }
                for e in cluster
            ],
        }


# =============================================================================
# INTEGRATED ANALYSIS
# =============================================================================

class ComprehensiveAnalyzer:
    """
    Combines all analysis techniques for comprehensive case analysis.
    """

    def __init__(self, save_path: str):
        self.save_path = save_path
        self.parser = SaveParser()
        self.parsed = self.parser.parse(save_path)

        with open(save_path, 'rb') as f:
            self.raw_save = f.read()

        self.coverage_analyzer = CoverageAnalyzer()
        self.base_tracker = UnknownBaseTracker()
        self.lookup_discovery = LookupTableDiscovery()

    def analyze_block(
        self,
        block_start: int,
        base_offset: int,
        slot_index: int = 0,
        known_items: List[Tuple[int, int, str]] = None,
    ) -> Dict[str, Any]:
        """
        Comprehensive analysis of a block.

        Returns:
            Dict with coverage, blindspots, and recommendations
        """
        ef_data = self.parsed.slots[slot_index].event_flags

        # Coverage analysis
        coverage = self.coverage_analyzer.analyze_block_coverage(
            ef_data, block_start, base_offset
        )

        # Inventory correlation
        inventory_correlation = None
        if known_items:
            inventory_correlation = self.coverage_analyzer.correlate_with_inventory(
                ef_data, known_items, block_start, base_offset
            )

        # Find unknown regions nearby
        search_start = max(0, base_offset - 1000)
        search_end = base_offset + 2000
        unknown_regions = self.coverage_analyzer.find_unknown_regions(
            ef_data, (search_start, search_end)
        )

        return {
            "block_start": block_start,
            "base_offset": base_offset,
            "coverage": {
                "data_percentage": coverage.get_data_coverage() * 100,
                "data_bytes": coverage.data_bytes,
                "padding_bytes": coverage.padding_bytes,
                "data_regions": len(coverage.data_regions),
                "padding_regions": len(coverage.padding_regions),
            },
            "regions": {
                "data": [
                    {"start": r.start_offset, "end": r.end_offset, "size": r.size}
                    for r in coverage.data_regions
                ],
                "padding": [
                    {"start": r.start_offset, "end": r.end_offset, "size": r.size}
                    for r in coverage.padding_regions
                ],
            },
            "inventory_correlation": inventory_correlation,
            "unknown_regions_nearby": [
                {"start": r.start_offset, "end": r.end_offset, "size": r.size}
                for r in unknown_regions
            ],
            "recommendations": self._generate_recommendations(
                coverage, inventory_correlation, unknown_regions
            ),
        }

    def _generate_recommendations(
        self,
        coverage: BlockCoverage,
        inventory_correlation: Optional[Dict],
        unknown_regions: List[RegionInfo],
    ) -> List[str]:
        """Generate actionable recommendations."""
        recommendations = []

        # Coverage issues
        if coverage.get_data_coverage() < 0.5:
            recommendations.append(
                f"Block has {coverage.get_data_coverage()*100:.0f}% data coverage. "
                "Consider if this block uses a different formula or is sparsely populated."
            )

        # Padding gap issues
        if len(coverage.padding_regions) > 2:
            recommendations.append(
                f"Block has {len(coverage.padding_regions)} padding gaps. "
                "Some flags may be unreachable at this base offset."
            )

        # Inventory mismatch
        if inventory_correlation:
            padding_items = inventory_correlation["items_in_padding_regions"]
            if padding_items > 0:
                recommendations.append(
                    f"{padding_items} inventory items land in padding regions. "
                    "These flags may use a different formula."
                )

        # Unknown regions
        if unknown_regions:
            total_unknown = sum(r.size for r in unknown_regions)
            recommendations.append(
                f"Found {len(unknown_regions)} unknown data regions ({total_unknown} bytes) "
                "near this block. Consider investigating for additional blocks."
            )

        return recommendations

    def full_blindspot_analysis(
        self,
        slot_index: int = 0,
    ) -> Dict[str, Any]:
        """
        Full blindspot analysis across all known blocks.
        """
        ef_data = self.parsed.slots[slot_index].event_flags
        block_bases = load_block_bases()

        results = {
            "analyzed_blocks": [],
            "total_coverage": 0.0,
            "unknown_regions": [],
            "undiscovered_bases": [],
        }

        total_data_bytes = 0
        total_expected_bytes = 0

        for block_start, info in block_bases.items():
            base = info["base_offset"]
            size = info.get("block_size", 1000)

            coverage = self.coverage_analyzer.analyze_block_coverage(
                ef_data, block_start, base, size
            )

            total_data_bytes += coverage.data_bytes
            total_expected_bytes += coverage.total_bytes

            results["analyzed_blocks"].append({
                "block": block_start,
                "base": base,
                "data_coverage": coverage.get_data_coverage() * 100,
                "padding_gaps": len(coverage.padding_regions),
            })

        # Overall coverage
        if total_expected_bytes > 0:
            results["total_coverage"] = (total_data_bytes / total_expected_bytes) * 100

        # Find all unknown regions
        results["unknown_regions"] = [
            {"start": r.start_offset, "end": r.end_offset, "size": r.size}
            for r in self.coverage_analyzer.find_unknown_regions(ef_data)
        ]

        return results


# =============================================================================
# CLI / DEMO
# =============================================================================

def demo():
    """Demonstrate the analysis capabilities."""
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    print("=" * 70)
    print("COMPREHENSIVE CASE ANALYSIS DEMO")
    print("=" * 70)

    analyzer = ComprehensiveAnalyzer(save_path)

    # Analyze block 520000
    items_520 = [
        (520000, 258000, "Lhutel the Headless"),
        (520030, 5050, "Assassin's Crimson Dagger"),
        (520210, 5060, "Assassin's Cerulean Dagger"),
        (520330, 4020, "Flamedrake Talisman"),
        (520450, 1110, "Gold Scarab"),
    ]

    print("\n--- Block 520000 Analysis ---")
    result = analyzer.analyze_block(520000, 1341, known_items=items_520)

    print(f"\nCoverage:")
    print(f"  Data: {result['coverage']['data_percentage']:.1f}%")
    print(f"  Data bytes: {result['coverage']['data_bytes']}")
    print(f"  Padding bytes: {result['coverage']['padding_bytes']}")
    print(f"  Data regions: {result['coverage']['data_regions']}")
    print(f"  Padding regions: {result['coverage']['padding_regions']}")

    if result["inventory_correlation"]:
        ic = result["inventory_correlation"]
        print(f"\nInventory Correlation:")
        print(f"  Items in data regions: {ic['items_in_data_regions']}")
        print(f"  Items in padding regions: {ic['items_in_padding_regions']}")

    if result["recommendations"]:
        print(f"\nRecommendations:")
        for rec in result["recommendations"]:
            print(f"  - {rec}")

    # Normalized confidence demo
    print("\n--- Normalized Confidence Demo ---")
    nc = NormalizedConfidence()

    # Add multiple evidence pieces of same type
    print("\nAdding inventory presence evidence (with diminishing returns):")
    for i in range(5):
        contrib = nc.add_evidence("inventory_present", True)
        print(f"  Evidence {i+1}: +{contrib:.3f} (total: {nc.get_total_confidence():.3f})")

    print(f"\nBreakdown: {nc.get_breakdown()}")


if __name__ == "__main__":
    demo()
