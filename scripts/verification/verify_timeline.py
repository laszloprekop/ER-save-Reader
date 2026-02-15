#!/usr/bin/env python3
"""
Timeline-Based Temporal Verification — uses s5-Bee slot_diffs for flag transition detection.

Reads the timeline slot_changes.jsonl and corresponding binary slot_diffs to
detect event flag transitions correlated with inventory changes.

The .bin files are SPARSE DIFFS: each 6-byte record encodes one changed byte:
    [u32_LE offset][u8 old_value][u8 new_value]

The offset is relative to the full 2.6MB slot. Using the eventFlagsOffset from
the JSONL metadata, we filter records falling within the EF section and extract
bit-level transitions.

Usage:
    python scripts/verification/verify_timeline.py
    python scripts/verification/verify_timeline.py --limit 50
    python scripts/verification/verify_timeline.py --json /tmp/timeline_results.json
    python scripts/verification/verify_timeline.py --verbose
"""

import argparse
import json
import struct
import sys
from pathlib import Path
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional, Tuple, Any

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.constants import EVENT_FLAGS_SIZE

# ============================================================================
# PATHS
# ============================================================================

TIMELINE_DIR = Path(
    "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/"
    "Granular snapshots for debugging/timeline"
)
SLOT_CHANGES = TIMELINE_DIR / "slot_changes.jsonl"
SLOT_DIFFS = TIMELINE_DIR / "slot_diffs"

# Each sparse diff record: 4-byte offset (u32 LE) + 1-byte old + 1-byte new
RECORD_SIZE = 6


# ============================================================================
# DATA CLASSES
# ============================================================================

@dataclass
class TimelineEntry:
    id: str
    timestamp: str
    slot_index: int
    character_name: str
    ef_offset: Optional[int]
    ga_items_end: Optional[int]
    ef_confident: Optional[bool]
    items_added: List[Dict[str, Any]]
    items_removed: List[Dict[str, Any]]
    graces_discovered: List[Dict[str, Any]]
    bosses_defeated: List[Dict[str, Any]]
    diff_file: str
    bytes_changed: int


@dataclass
class FlagTransition:
    """A single bit that changed in the EF section."""
    ef_byte_offset: int   # Offset within event flags section
    bit_position: int
    old_value: int        # 0 or 1
    new_value: int        # 0 or 1
    direction: str        # "set" (0→1) or "cleared" (1→0)


@dataclass
class TimelineVerificationResult:
    entry_id: str
    timestamp: str
    items_added: List[Dict[str, Any]]
    graces_discovered: List[Dict[str, Any]]
    bosses_defeated: List[Dict[str, Any]]
    ef_offset: int
    ef_confident: bool
    ef_transitions_set: int       # Number of 0→1 bit transitions in EF section
    ef_transitions_cleared: int   # Number of 1→0 bit transitions
    total_bytes_changed: int
    ef_bytes_changed: int         # Bytes changed within EF section
    flag_transitions: List[Dict[str, Any]]  # Details of EF bit changes
    skipped: bool = False
    skip_reason: str = ""


# ============================================================================
# SPARSE DIFF PARSING
# ============================================================================

def parse_sparse_diff(diff_path: Path) -> List[Tuple[int, int, int]]:
    """
    Parse a sparse diff .bin file into (offset, old_value, new_value) tuples.

    Each record is 6 bytes: [u32_LE offset][u8 old][u8 new]
    """
    data = diff_path.read_bytes()
    records = []

    for pos in range(0, len(data) - RECORD_SIZE + 1, RECORD_SIZE):
        offset = struct.unpack_from('<I', data, pos)[0]
        old_val = data[pos + 4]
        new_val = data[pos + 5]
        records.append((offset, old_val, new_val))

    return records


def extract_ef_transitions(
    records: List[Tuple[int, int, int]],
    ef_offset: int,
) -> List[FlagTransition]:
    """
    Filter diff records for the EF section and extract bit-level transitions.

    Args:
        records: Parsed sparse diff records (offset, old, new)
        ef_offset: Absolute offset of EF section within slot data

    Returns:
        List of FlagTransition for bits that changed in the EF section
    """
    transitions = []
    ef_end = ef_offset + EVENT_FLAGS_SIZE

    for abs_offset, old_byte, new_byte in records:
        if abs_offset < ef_offset or abs_offset >= ef_end:
            continue

        if old_byte == new_byte:
            continue

        ef_byte = abs_offset - ef_offset
        diff = old_byte ^ new_byte

        for bit in range(8):
            if (diff >> bit) & 1:
                old_bit = (old_byte >> bit) & 1
                new_bit = (new_byte >> bit) & 1
                transitions.append(FlagTransition(
                    ef_byte_offset=ef_byte,
                    bit_position=bit,
                    old_value=old_bit,
                    new_value=new_bit,
                    direction="set" if new_bit == 1 else "cleared",
                ))

    return transitions


# ============================================================================
# TIMELINE PARSING
# ============================================================================

def load_timeline_entries(limit: Optional[int] = None) -> List[TimelineEntry]:
    """Load timeline entries from slot_changes.jsonl."""
    entries = []

    with open(SLOT_CHANGES) as f:
        for i, line in enumerate(f):
            if limit and i >= limit:
                break

            raw = json.loads(line.strip())
            delta = raw.get("inventoryDelta") or {}
            offsets = raw.get("structuralOffsets") or {}

            entries.append(TimelineEntry(
                id=raw.get("id", ""),
                timestamp=raw.get("timestamp", ""),
                slot_index=raw.get("slotIndex", 5),
                character_name=raw.get("characterName", ""),
                ef_offset=offsets.get("eventFlagsOffset"),
                ga_items_end=offsets.get("gaItemsEnd"),
                ef_confident=offsets.get("efConfident"),
                items_added=delta.get("added", []),
                items_removed=delta.get("removed", []),
                graces_discovered=raw.get("gracesDiscovered", []),
                bosses_defeated=raw.get("bossesDefeated", []),
                diff_file=raw.get("diffFile", ""),
                bytes_changed=raw.get("bytesChanged", 0),
            ))

    return entries


def find_pickup_transitions(
    entries: List[TimelineEntry],
    verbose: bool = False,
) -> List[TimelineVerificationResult]:
    """
    For timeline entries where inventory changed, parse the sparse diff
    to find EF bit transitions.

    The sparse diff records absolute byte positions. Because the EF section
    moves as GaItems grows, the diff is only meaningful for EF analysis when
    the current AND previous entries share the same ef_offset. Otherwise,
    bytes at the same absolute position represent different EF data and
    the comparison is nonsensical.
    """
    results = []
    no_ef_offset = 0
    ef_shifted = 0

    for i, entry in enumerate(entries):
        # Only interested in entries with events
        has_pickup = len(entry.items_added) > 0
        has_grace = len(entry.graces_discovered) > 0
        has_boss = len(entry.bosses_defeated) > 0

        if not (has_pickup or has_grace or has_boss):
            continue

        # Need EF offset from JSONL metadata
        if entry.ef_offset is None:
            no_ef_offset += 1
            results.append(TimelineVerificationResult(
                entry_id=entry.id,
                timestamp=entry.timestamp,
                items_added=entry.items_added,
                graces_discovered=entry.graces_discovered,
                bosses_defeated=entry.bosses_defeated,
                ef_offset=0,
                ef_confident=False,
                ef_transitions_set=0,
                ef_transitions_cleared=0,
                total_bytes_changed=entry.bytes_changed,
                ef_bytes_changed=0,
                flag_transitions=[],
                skipped=True,
                skip_reason="No eventFlagsOffset in metadata",
            ))
            continue

        # Check previous entry's EF offset — diff is only valid if EF didn't shift
        prev_ef_offset = None
        if i > 0:
            prev_ef_offset = entries[i - 1].ef_offset

        if prev_ef_offset is None or prev_ef_offset != entry.ef_offset:
            ef_shifted += 1
            results.append(TimelineVerificationResult(
                entry_id=entry.id,
                timestamp=entry.timestamp,
                items_added=entry.items_added,
                graces_discovered=entry.graces_discovered,
                bosses_defeated=entry.bosses_defeated,
                ef_offset=entry.ef_offset,
                ef_confident=entry.ef_confident or False,
                ef_transitions_set=0,
                ef_transitions_cleared=0,
                total_bytes_changed=entry.bytes_changed,
                ef_bytes_changed=0,
                flag_transitions=[],
                skipped=True,
                skip_reason=f"EF shifted: prev={prev_ef_offset}, curr={entry.ef_offset}",
            ))
            continue

        # Parse the sparse diff
        diff_path = SLOT_DIFFS / entry.diff_file
        if not diff_path.exists():
            results.append(TimelineVerificationResult(
                entry_id=entry.id,
                timestamp=entry.timestamp,
                items_added=entry.items_added,
                graces_discovered=entry.graces_discovered,
                bosses_defeated=entry.bosses_defeated,
                ef_offset=entry.ef_offset,
                ef_confident=entry.ef_confident or False,
                ef_transitions_set=0,
                ef_transitions_cleared=0,
                total_bytes_changed=entry.bytes_changed,
                ef_bytes_changed=0,
                flag_transitions=[],
                skipped=True,
                skip_reason=f"Diff file not found: {entry.diff_file}",
            ))
            continue

        records = parse_sparse_diff(diff_path)
        transitions = extract_ef_transitions(records, entry.ef_offset)

        # Count EF bytes changed
        ef_end = entry.ef_offset + EVENT_FLAGS_SIZE
        ef_bytes = sum(
            1 for off, old, new in records
            if entry.ef_offset <= off < ef_end and old != new
        )

        set_count = sum(1 for t in transitions if t.direction == "set")
        cleared_count = sum(1 for t in transitions if t.direction == "cleared")

        # Convert to dicts for reporting
        flag_details = []
        for t in transitions[:100]:
            flag_details.append({
                "ef_byte": t.ef_byte_offset,
                "bit": t.bit_position,
                "old": t.old_value,
                "new": t.new_value,
                "direction": t.direction,
            })

        results.append(TimelineVerificationResult(
            entry_id=entry.id,
            timestamp=entry.timestamp,
            items_added=entry.items_added,
            graces_discovered=entry.graces_discovered,
            bosses_defeated=entry.bosses_defeated,
            ef_offset=entry.ef_offset,
            ef_confident=entry.ef_confident or False,
            ef_transitions_set=set_count,
            ef_transitions_cleared=cleared_count,
            total_bytes_changed=entry.bytes_changed,
            ef_bytes_changed=ef_bytes,
            flag_transitions=flag_details,
        ))

    if verbose:
        print(f"  Entries without EF offset: {no_ef_offset}")
        print(f"  Entries with EF shift (skipped): {ef_shifted}")

    return results


# ============================================================================
# REPORTING
# ============================================================================

def print_results(results: List[TimelineVerificationResult], verbose: bool = False):
    """Print verification results."""
    print(f"\n{'='*70}")
    print(f"TIMELINE VERIFICATION — Sparse Diff Flag Transitions")
    print(f"{'='*70}")
    print(f"Total entries with events: {len(results)}")

    total_sets = 0
    total_clears = 0
    entries_with_transitions = 0
    entries_with_pickups = 0
    entries_with_graces = 0
    entries_with_bosses = 0
    skipped_entries = 0
    confident_entries = 0

    for r in results:
        total_sets += r.ef_transitions_set
        total_clears += r.ef_transitions_cleared
        if r.ef_transitions_set > 0 or r.ef_transitions_cleared > 0:
            entries_with_transitions += 1
        if r.items_added:
            entries_with_pickups += 1
        if r.graces_discovered:
            entries_with_graces += 1
        if r.bosses_defeated:
            entries_with_bosses += 1
        if r.skipped:
            skipped_entries += 1
        if r.ef_confident:
            confident_entries += 1

    print(f"\nSummary:")
    print(f"  Entries with item pickups: {entries_with_pickups}")
    print(f"  Entries with grace discoveries: {entries_with_graces}")
    print(f"  Entries with boss defeats: {entries_with_bosses}")
    print(f"  Entries with EF transitions: {entries_with_transitions}")
    print(f"  Entries skipped (no EF offset): {skipped_entries}")
    print(f"  Entries with confident EF: {confident_entries}")
    print(f"  Total flags SET (0→1): {total_sets}")
    print(f"  Total flags CLEARED (1→0): {total_clears}")

    # Distribution of transition counts
    non_skipped = [r for r in results if not r.skipped]
    if non_skipped:
        transition_counts = [r.ef_transitions_set + r.ef_transitions_cleared for r in non_skipped]
        avg = sum(transition_counts) / len(transition_counts) if transition_counts else 0
        max_tc = max(transition_counts) if transition_counts else 0
        zero_tc = sum(1 for t in transition_counts if t == 0)
        print(f"\n  Transition distribution (non-skipped):")
        print(f"    Avg per entry: {avg:.1f}")
        print(f"    Max: {max_tc}")
        print(f"    Zero transitions: {zero_tc}/{len(non_skipped)}")

    if verbose:
        print(f"\n{'─'*70}")
        print(f"Detailed Transitions:")
        print(f"{'─'*70}")

        for r in results:
            if r.skipped:
                if verbose:
                    print(f"\n  {r.entry_id} ({r.timestamp}) — SKIPPED: {r.skip_reason}")
                continue

            if r.ef_transitions_set == 0 and r.ef_transitions_cleared == 0:
                continue

            conf_tag = " [confident]" if r.ef_confident else ""
            print(f"\n  {r.entry_id} ({r.timestamp}){conf_tag}")
            print(f"    EF offset: {r.ef_offset}, EF bytes changed: {r.ef_bytes_changed}/{r.total_bytes_changed}")

            if r.items_added:
                items_str = ", ".join(
                    f"{it.get('itemId')}({it.get('category','?')})"
                    for it in r.items_added[:5]
                )
                if len(r.items_added) > 5:
                    items_str += f" +{len(r.items_added)-5} more"
                print(f"    Items added: {items_str}")

            if r.graces_discovered:
                graces_str = ", ".join(
                    f"{g.get('flagId')}:{g.get('name','?')}"
                    for g in r.graces_discovered[:3]
                )
                if len(r.graces_discovered) > 3:
                    graces_str += f" +{len(r.graces_discovered)-3} more"
                print(f"    Graces: {graces_str}")

            if r.bosses_defeated:
                bosses_str = ", ".join(
                    f"{b.get('flagId')}:{b.get('name','?')}"
                    for b in r.bosses_defeated
                )
                print(f"    Bosses: {bosses_str}")

            print(f"    EF transitions: {r.ef_transitions_set} SET, {r.ef_transitions_cleared} CLEARED")

            if r.flag_transitions:
                for ft in r.flag_transitions[:10]:
                    print(f"      EF[{ft['ef_byte']}] bit {ft['bit']}: {ft['old']}→{ft['new']} ({ft['direction']})")
                if len(r.flag_transitions) > 10:
                    print(f"      ... +{len(r.flag_transitions)-10} more transitions")


# ============================================================================
# MAIN
# ============================================================================

def main():
    parser = argparse.ArgumentParser(description="Timeline-based temporal verification")
    parser.add_argument("--limit", type=int, help="Limit number of entries to process")
    parser.add_argument("--verbose", "-v", action="store_true", help="Show per-entry details")
    parser.add_argument("--json", type=str, help="Write JSON results to file")
    args = parser.parse_args()

    if not SLOT_CHANGES.exists():
        print(f"Timeline data not found: {SLOT_CHANGES}", file=sys.stderr)
        sys.exit(1)

    print("Loading timeline entries...")
    entries = load_timeline_entries(limit=args.limit)
    print(f"Loaded {len(entries)} entries")

    print("Analyzing flag transitions from sparse diffs...")
    results = find_pickup_transitions(entries, verbose=args.verbose)

    print_results(results, verbose=args.verbose)

    if args.json:
        json_data = [asdict(r) for r in results]
        with open(args.json, 'w') as f:
            json.dump(json_data, f, indent=2)
        print(f"\nJSON results written to {args.json}")


if __name__ == "__main__":
    main()
