#!/usr/bin/env python3
"""
EF Section Verifier

Verify that we're actually finding the real EF section by checking:
1. Multiple independent flag anchors
2. Consistency of known flag relationships
3. Byte patterns that should only exist in EF data
"""

import struct
from pathlib import Path
from typing import Dict, List, Tuple, Optional

SNAPSHOTS_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/slot 0 Confessor")

BND4_HEADER_SIZE = 0x40
BND4_ENTRY_SIZE = 0x20
BND4_ENTRY_OFFSET_POS = 0x10
SLOT_CHECKSUM_SIZE = 16
EVENT_FLAGS_SIZE = 0x1BF99F

# STRICT validation anchors - these are flags we KNOW are set for this character
# Format: (flag_id, byte_offset, bit_position, block_base, expected_state, tier)
# Tier 1 = critical flags, Tier 2 = early game flags
STRICT_ANCHORS = [
    # Tier 1: Tutorial graces - MUST be set
    (71800, 2725, 7, 2725, True, 1),   # Cave of Knowledge
    (71801, 2725, 6, 2725, True, 1),   # Stranded Graveyard

    # Tier 1: Early world graces - MUST be set
    (76100, 3262, 3, 3250, True, 1),   # The First Step
    (76101, 3262, 2, 3250, True, 1),   # Church of Elleh

    # Tier 2: Early game graces - help distinguish real EF from false positives
    (76102, 3262, 1, 3250, True, 2),   # Gatefront Ruins
    (76104, 3263, 7, 3250, True, 2),   # Agheel Lake South
    (76106, 3263, 5, 3250, True, 2),   # Church of Dragon Communion
]


def get_slot_data(filepath: Path, slot_index: int = 0) -> bytes:
    """Extract raw slot data."""
    with open(filepath, 'rb') as f:
        data = f.read()

    entry_offset = BND4_HEADER_SIZE + (slot_index * BND4_ENTRY_SIZE) + BND4_ENTRY_OFFSET_POS
    bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
    slot_offset = bnd4_offset + SLOT_CHECKSUM_SIZE
    return data[slot_offset:slot_offset + 0x280000]


def check_flag_at_position(data: bytes, offset: int, bit: int) -> bool:
    """Check if a specific bit is set."""
    if offset < len(data):
        return (data[offset] & (1 << bit)) != 0
    return False


def find_ef_candidates(slot_data: bytes, search_start: int = 0x10000,
                       search_end: int = 0x30000) -> List[Tuple[int, int]]:
    """
    Find ALL potential EF offsets that pass 2-anchor validation.
    Returns list of (offset, score) tuples.
    """
    # Minimal 2-anchor check (what current algorithm does)
    minimal_anchors = [(2725, 7), (3262, 3)]  # 71800 and 76100

    candidates = []
    for test_offset in range(search_start, min(search_end, len(slot_data) - EVENT_FLAGS_SIZE), 4):
        score = sum(1 for byte_off, bit_pos in minimal_anchors
                    if test_offset + byte_off < len(slot_data)
                    and check_flag_at_position(slot_data, test_offset + byte_off, bit_pos))
        if score >= 2:
            candidates.append((test_offset, score))

    return candidates


def strict_validate_ef(slot_data: bytes, ef_offset: int) -> Tuple[int, int, List[str]]:
    """
    Strictly validate a potential EF offset using multiple anchors.
    Returns (tier1_score, total_score, list of failures).
    """
    failures = []
    tier1_score = 0
    total_score = 0

    for flag_id, byte_off, bit_pos, base, expected, tier in STRICT_ANCHORS:
        actual = check_flag_at_position(slot_data, ef_offset + byte_off, bit_pos)
        if actual == expected:
            total_score += 1
            if tier == 1:
                tier1_score += 1
        else:
            failures.append(f"Flag {flag_id} at {byte_off}:{bit_pos} expected {expected}, got {actual}")

    return tier1_score, total_score, failures


def analyze_ef_consistency(snapshots: List[Path]) -> None:
    """Analyze EF consistency across snapshots."""
    print("="*80)
    print("EF SECTION VERIFICATION ANALYSIS")
    print("="*80)

    for snapshot in snapshots[:20]:  # First 20 snapshots
        print(f"\n{'-'*60}")
        print(f"Snapshot: {snapshot.name[:55]}")

        try:
            slot_data = get_slot_data(snapshot)
        except Exception as e:
            print(f"  ERROR: {e}")
            continue

        # Find all candidates using minimal validation
        candidates = find_ef_candidates(slot_data)

        if not candidates:
            print(f"  No candidates found!")
            continue

        tier1_count = sum(1 for a in STRICT_ANCHORS if a[5] == 1)
        print(f"  Found {len(candidates)} candidate(s) with 2+ anchor matches:")

        for offset, min_score in candidates[:5]:  # Show first 5
            # Now strictly validate each candidate
            tier1_score, total_score, failures = strict_validate_ef(slot_data, offset)

            status = "✓" if total_score == len(STRICT_ANCHORS) else "✗"
            print(f"    Offset 0x{offset:05X}: minimal={min_score}/2, tier1={tier1_score}/{tier1_count}, total={total_score}/{len(STRICT_ANCHORS)} {status}")

            if failures and len(failures) <= 3:
                for f in failures:
                    print(f"      - {f}")


def deep_ef_analysis(snapshot_path: Path) -> None:
    """
    Deep analysis of a single snapshot to understand EF structure.
    """
    print("\n" + "="*80)
    print("DEEP EF STRUCTURE ANALYSIS")
    print("="*80)
    print(f"Analyzing: {snapshot_path.name}")

    slot_data = get_slot_data(snapshot_path)

    # Find the candidate that passes most strict validation
    candidates = find_ef_candidates(slot_data, 0x10000, 0x30000)

    best_offset = None
    best_tier1 = 0
    best_total = 0

    for offset, _ in candidates:
        tier1, total, _ = strict_validate_ef(slot_data, offset)
        # Prioritize tier1 score, then total score
        if tier1 > best_tier1 or (tier1 == best_tier1 and total > best_total):
            best_tier1 = tier1
            best_total = total
            best_offset = offset

    if not best_offset:
        print("No valid EF offset found!")
        return

    tier1_count = sum(1 for a in STRICT_ANCHORS if a[5] == 1)
    print(f"\nBest EF offset: 0x{best_offset:05X} (tier1={best_tier1}/{tier1_count}, total={best_total}/{len(STRICT_ANCHORS)})")

    # Dump the first 100 bytes at key offsets
    print(f"\nKey offset dumps (relative to EF start at 0x{best_offset:05X}):")

    key_offsets = [
        (2725, "71800/71801 area"),
        (3262, "76100/76101 area"),
        (3250, "block 76000 start"),
        (2662, "block 73000 start"),
        (3198, "potential 71600 area"),
    ]

    for rel_offset, desc in key_offsets:
        abs_offset = best_offset + rel_offset
        if abs_offset + 8 <= len(slot_data):
            bytes_at = slot_data[abs_offset:abs_offset + 8]
            print(f"  Offset {rel_offset:5d} ({desc:20s}): {bytes_at.hex()}")

            # Show individual bits for first byte
            first_byte = bytes_at[0]
            bits = ''.join(str((first_byte >> (7-i)) & 1) for i in range(8))
            print(f"    First byte bits (7→0): {bits}")

    # Now let's look for a potential header or pointer table in first 1000 bytes
    print(f"\nSearching for structure in first 1000 bytes of EF section...")
    ef_data = slot_data[best_offset:best_offset + 1000]

    # Look for 4-byte values that could be offsets
    print("  4-byte values that could be offsets (range 1000-50000):")
    for i in range(0, min(100, len(ef_data) - 4), 4):
        val = struct.unpack_from('<I', ef_data, i)[0]
        if 1000 < val < 50000:
            print(f"    At EF+{i:4d}: {val:8d} (0x{val:06X})")


def compare_ef_across_snapshots(snapshots: List[Path], rel_offset: int = 2725) -> None:
    """
    Compare the exact bytes at a relative offset across snapshots.
    """
    print("\n" + "="*80)
    print(f"COMPARING BYTES AT RELATIVE OFFSET {rel_offset} ACROSS SNAPSHOTS")
    print("="*80)

    prev_bytes = None

    for snapshot in snapshots:
        try:
            slot_data = get_slot_data(snapshot)
            candidates = find_ef_candidates(slot_data)

            if not candidates:
                continue

            # Take first candidate (what current algorithm does)
            ef_offset = candidates[0][0]
            abs_offset = ef_offset + rel_offset

            if abs_offset + 4 <= len(slot_data):
                bytes_at = slot_data[abs_offset:abs_offset + 4]

                if prev_bytes is None or bytes_at != prev_bytes:
                    print(f"  EF=0x{ef_offset:05X} @ rel {rel_offset}: {bytes_at.hex()} | {snapshot.name[:40]}")
                    prev_bytes = bytes_at
        except:
            pass


def main():
    import re

    snapshots = sorted(SNAPSHOTS_DIR.glob("ER0000.sl2*"),
                       key=lambda p: p.stat().st_mtime)

    print(f"Found {len(snapshots)} snapshots\n")

    # 1. Consistency analysis
    analyze_ef_consistency(snapshots)

    # 2. Deep analysis of latest snapshot
    if snapshots:
        deep_ef_analysis(snapshots[-1])

    # 3. Compare byte 2725 (71800/71801) across snapshots
    compare_ef_across_snapshots(snapshots, 2725)

    # 4. Compare byte 3262 (76100/76101) across snapshots
    compare_ef_across_snapshots(snapshots, 3262)


if __name__ == "__main__":
    main()
