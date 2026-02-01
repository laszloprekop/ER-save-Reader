#!/usr/bin/env python3
"""
Blindspot Analysis Tool

Analyzes the event flags section to identify:
1. Coverage of known blocks (data vs padding)
2. Unknown regions with data (potential undiscovered blocks)
3. Lookup table patterns
4. Recommendations for further investigation

Usage:
    python blindspot_analysis.py --save /path/to/save.sl2
    python blindspot_analysis.py --block 520000 --base 1341
    python blindspot_analysis.py --full-scan
"""

import argparse
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Tuple

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser
from scripts.verification.ground_truth_loader import load_block_bases
from scripts.verification.case_analysis import (
    CoverageAnalyzer,
    UnknownBaseTracker,
    LookupTableDiscovery,
    ComprehensiveAnalyzer,
)


DEFAULT_SAVE = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"


@dataclass
class BlindspotReport:
    """Comprehensive blindspot analysis report."""
    # Coverage stats
    total_known_blocks: int
    total_data_bytes: int
    total_padding_bytes: int
    average_coverage: float

    # Block details
    block_coverage: List[Dict[str, Any]]

    # Unknown regions
    unknown_regions: List[Dict[str, Any]]
    unknown_data_bytes: int

    # Potential discoveries
    potential_bases: List[Dict[str, Any]]
    lookup_table_candidates: List[Dict[str, Any]]

    # Recommendations
    recommendations: List[str]


def analyze_all_blocks(ef_data: bytes) -> List[Dict[str, Any]]:
    """Analyze coverage for all known blocks."""
    block_bases = load_block_bases()
    analyzer = CoverageAnalyzer()

    results = []
    for block_start, info in sorted(block_bases.items()):
        base = info["base_offset"]
        size = info.get("block_size", 1000)
        status = info.get("status", "unknown")

        coverage = analyzer.analyze_block_coverage(ef_data, block_start, base, size)

        results.append({
            "block": block_start,
            "base": base,
            "status": status,
            "total_bytes": coverage.total_bytes,
            "data_bytes": coverage.data_bytes,
            "padding_bytes": coverage.padding_bytes,
            "data_coverage": coverage.get_data_coverage() * 100,
            "data_regions": len(coverage.data_regions),
            "padding_regions": len(coverage.padding_regions),
        })

    return results


def find_unknown_data_regions(
    ef_data: bytes,
    known_blocks: List[Dict[str, Any]],
    min_size: int = 8,
) -> List[Dict[str, Any]]:
    """Find regions with data that aren't part of known blocks."""
    # Build set of known byte offsets
    known_offsets = set()
    for block in known_blocks:
        base = block["base"]
        size = block["total_bytes"]
        for offset in range(base, base + size):
            known_offsets.add(offset)

    # Scan for unknown data
    unknown_regions = []
    in_region = False
    region_start = 0
    region_bytes = []

    for offset in range(len(ef_data)):
        byte_val = ef_data[offset]
        is_data = (byte_val != 0xFF and byte_val != 0x00)
        is_known = offset in known_offsets

        if is_data and not is_known:
            if not in_region:
                in_region = True
                region_start = offset
                region_bytes = []
            region_bytes.append(byte_val)
        else:
            if in_region:
                if len(region_bytes) >= min_size:
                    # Calculate potential flag IDs this region could represent
                    potential_flags = analyze_region_pattern(region_start, region_bytes)

                    unknown_regions.append({
                        "start": region_start,
                        "end": region_start + len(region_bytes) - 1,
                        "size": len(region_bytes),
                        "first_bytes": [f"0x{b:02X}" for b in region_bytes[:8]],
                        "potential_flags": potential_flags,
                    })
                in_region = False

    return unknown_regions


def analyze_region_pattern(start_offset: int, bytes_data: List[int]) -> List[str]:
    """Analyze a region to guess what flags it might contain."""
    patterns = []

    # Check if it could be a block with various starting points
    for test_block in range(0, 1000000, 10000):
        # If this region is at start_offset, what block would have this as base?
        # base = start_offset, block_start = test_block
        # Expected flags: test_block to test_block + len(bytes_data) * 8

        potential_start = test_block
        potential_end = test_block + len(bytes_data) * 8

        # Check if this makes sense (reasonable flag range)
        if 1000 <= potential_start <= 999999:
            patterns.append(f"Could be block {potential_start}-{potential_end}")
            if len(patterns) >= 3:
                break

    return patterns


def find_potential_lookup_tables(
    raw_data: bytes,
    ef_start: int,
    known_bases: List[int],
) -> List[Dict[str, Any]]:
    """Search for potential offset lookup tables."""
    discovery = LookupTableDiscovery()

    # Search in the region before EF section (might contain metadata)
    search_start = max(0, ef_start - 10000)
    search_end = ef_start

    entries = discovery.search_for_offset_table(raw_data, (search_start, search_end))
    clusters = discovery.find_offset_clusters(entries)

    results = []
    for cluster in clusters:
        analysis = discovery.analyze_cluster(cluster)
        if analysis["known_matches"] >= 2:
            results.append(analysis)

    return results


def generate_recommendations(
    block_coverage: List[Dict],
    unknown_regions: List[Dict],
    potential_bases: List[Dict],
) -> List[str]:
    """Generate actionable recommendations."""
    recommendations = []

    # Low coverage blocks
    low_coverage_blocks = [b for b in block_coverage if b["data_coverage"] < 50]
    if low_coverage_blocks:
        recommendations.append(
            f"INVESTIGATE: {len(low_coverage_blocks)} blocks have <50% data coverage. "
            f"These may have incorrect base offsets or use different formulas."
        )

    # Blocks with many padding gaps
    fragmented_blocks = [b for b in block_coverage if b["padding_regions"] > 5]
    if fragmented_blocks:
        blocks = [str(b["block"]) for b in fragmented_blocks[:3]]
        recommendations.append(
            f"FRAGMENTED: Blocks {', '.join(blocks)} have many padding gaps. "
            "Some flags within these blocks may be unreachable."
        )

    # Unknown data regions
    if unknown_regions:
        total_unknown = sum(r["size"] for r in unknown_regions)
        recommendations.append(
            f"UNKNOWN DATA: Found {len(unknown_regions)} regions with {total_unknown} bytes "
            "of unidentified data. These may be undiscovered blocks."
        )

        # Highlight largest unknown regions
        largest = sorted(unknown_regions, key=lambda x: x["size"], reverse=True)[:3]
        for region in largest:
            recommendations.append(
                f"  → Region at offset {region['start']}-{region['end']} ({region['size']} bytes) "
                "needs investigation."
            )

    # Partial status blocks
    partial_blocks = [b for b in block_coverage if b["status"] == "partial"]
    if partial_blocks:
        recommendations.append(
            f"PARTIAL: {len(partial_blocks)} blocks have partial verification. "
            "Additional evidence needed for full verification."
        )

    return recommendations


def run_full_analysis(save_path: str, slot_index: int = 0) -> BlindspotReport:
    """Run comprehensive blindspot analysis."""
    parser = SaveParser()
    parsed = parser.parse(save_path)

    with open(save_path, 'rb') as f:
        raw_save = f.read()

    ef_data = parsed.slots[slot_index].event_flags
    ef_offset = parsed.slots[slot_index].event_flags_offset_absolute

    print(f"Analyzing slot {slot_index}...")
    print(f"  EF section: {len(ef_data)} bytes at offset {ef_offset}")

    # 1. Analyze all known blocks
    print("\nAnalyzing known blocks...")
    block_coverage = analyze_all_blocks(ef_data)

    total_data = sum(b["data_bytes"] for b in block_coverage)
    total_padding = sum(b["padding_bytes"] for b in block_coverage)
    avg_coverage = sum(b["data_coverage"] for b in block_coverage) / len(block_coverage) if block_coverage else 0

    # 2. Find unknown data regions
    print("Scanning for unknown regions...")
    unknown_regions = find_unknown_data_regions(ef_data, block_coverage)
    unknown_data = sum(r["size"] for r in unknown_regions)

    # 3. Search for potential bases
    print("Searching for potential base patterns...")
    base_tracker = UnknownBaseTracker()
    potential_bases = []  # Would be populated from rejected cases

    # 4. Look for lookup tables
    print("Searching for lookup tables...")
    known_bases = [b["base"] for b in block_coverage]
    lookup_candidates = find_potential_lookup_tables(raw_save, ef_offset, known_bases)

    # 5. Generate recommendations
    recommendations = generate_recommendations(block_coverage, unknown_regions, potential_bases)

    return BlindspotReport(
        total_known_blocks=len(block_coverage),
        total_data_bytes=total_data,
        total_padding_bytes=total_padding,
        average_coverage=avg_coverage,
        block_coverage=block_coverage,
        unknown_regions=unknown_regions,
        unknown_data_bytes=unknown_data,
        potential_bases=potential_bases,
        lookup_table_candidates=lookup_candidates,
        recommendations=recommendations,
    )


def print_report(report: BlindspotReport):
    """Print a formatted report."""
    print("\n" + "=" * 80)
    print("BLINDSPOT ANALYSIS REPORT")
    print("=" * 80)

    print("\n--- SUMMARY ---")
    print(f"Known blocks analyzed: {report.total_known_blocks}")
    print(f"Total data bytes: {report.total_data_bytes:,}")
    print(f"Total padding bytes: {report.total_padding_bytes:,}")
    print(f"Average coverage: {report.average_coverage:.1f}%")
    print(f"Unknown data regions: {len(report.unknown_regions)} ({report.unknown_data_bytes:,} bytes)")

    print("\n--- BLOCK COVERAGE ---")
    print(f"{'Block':<10} {'Base':<8} {'Status':<10} {'Coverage':<10} {'Data':<8} {'Gaps':<6}")
    print("-" * 60)

    for b in sorted(report.block_coverage, key=lambda x: x["data_coverage"]):
        coverage_bar = "█" * int(b["data_coverage"] / 10) + "░" * (10 - int(b["data_coverage"] / 10))
        print(f"{b['block']:<10} {b['base']:<8} {b['status']:<10} "
              f"{coverage_bar} {b['data_bytes']:<8} {b['padding_regions']:<6}")

    if report.unknown_regions:
        print("\n--- UNKNOWN DATA REGIONS ---")
        print(f"{'Offset':<15} {'Size':<8} {'First Bytes':<30}")
        print("-" * 60)

        for region in sorted(report.unknown_regions, key=lambda x: x["size"], reverse=True)[:15]:
            offset_str = f"{region['start']}-{region['end']}"
            bytes_str = " ".join(region["first_bytes"][:4])
            print(f"{offset_str:<15} {region['size']:<8} {bytes_str:<30}")

    if report.lookup_table_candidates:
        print("\n--- POTENTIAL LOOKUP TABLES ---")
        for table in report.lookup_table_candidates:
            print(f"\nTable at offset {table['start_offset']}-{table['end_offset']}:")
            print(f"  Entries: {table['entry_count']}")
            print(f"  Known matches: {table['known_matches']}")
            print(f"  Matched blocks: {table['matched_blocks']}")

    if report.recommendations:
        print("\n--- RECOMMENDATIONS ---")
        for rec in report.recommendations:
            print(f"\n• {rec}")

    print("\n" + "=" * 80)


def main():
    parser = argparse.ArgumentParser(description="Blindspot Analysis Tool")
    parser.add_argument("--save", default=DEFAULT_SAVE, help="Save file path")
    parser.add_argument("--slot", type=int, default=0, help="Slot index")
    parser.add_argument("--block", type=int, help="Analyze specific block")
    parser.add_argument("--base", type=int, help="Base offset for block analysis")
    parser.add_argument("--full-scan", action="store_true", help="Full scan of unknown regions")

    args = parser.parse_args()

    if args.block and args.base:
        # Analyze specific block
        analyzer = ComprehensiveAnalyzer(args.save)
        result = analyzer.analyze_block(args.block, args.base, args.slot)

        print(f"\n--- Block {args.block} Analysis ---")
        print(f"Base: {args.base}")
        print(f"Coverage: {result['coverage']['data_percentage']:.1f}%")
        print(f"Data regions: {result['coverage']['data_regions']}")
        print(f"Padding regions: {result['coverage']['padding_regions']}")

        if result["recommendations"]:
            print("\nRecommendations:")
            for rec in result["recommendations"]:
                print(f"  • {rec}")
    else:
        # Full analysis
        report = run_full_analysis(args.save, args.slot)
        print_report(report)


if __name__ == "__main__":
    main()
