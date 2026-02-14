#!/usr/bin/env python3
"""
Timeline Analysis Script for Elden Ring Save File Granular Snapshots

Parses binary diff files captured between autosaves and analyzes:
- Event flag regions (single-bit changes in clustered offset ranges)
- Noisy vs. meaningful offset regions
- Potential event flag ID mapping using known formulas
"""

import json
import struct
import sys
from collections import defaultdict, Counter
from pathlib import Path

# ─── Paths ───────────────────────────────────────────────────────────────────

TIMELINE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files"
                     "/Granular snapshots for debugging/timeline")
JSONL_PATH = TIMELINE_DIR / "slot_changes.jsonl"
DIFFS_DIR = TIMELINE_DIR / "slot_diffs"
GROUND_TRUTH_PATH = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor"
                          "/ground_truth_offsets.json")

# ─── Known block bases from ground truth ─────────────────────────────────────
# Format: block_start -> (base_offset, block_size, name, status)

KNOWN_BLOCK_BASES = {
    60000: (2548, 1000, "Progression flags", "verified"),
    61000: (2671, 1000, "Map area visit flags", "verified"),
    65000: (37412, 1000, "Crystal Tears", "verified"),
    71800: (2725, 200, "Tutorial graces", "verified"),
    72000: (2750, 1000, "DLC graces (Enir-Ilim)", "verified"),
    74000: (3000, 1000, "DLC dungeon graces", "verified"),
    76000: (3250, 1000, "Limgrave world graces", "verified"),
    77000: (3373, 1000, "Graces block 77k", "verified"),
    510000: (63750, 1000, "Remembrance consumption", "verified"),
    520000: (1341, 500, "Spirit Ashes/Talismans", "partial"),
    540000: (67500, 1000, "Sorcery/Incantation/AoW unlock", "verified"),
    710000: (13875, 1000, "Roundtable Hold NPC progression", "verified"),
}

# Tile formula parameters
TILE_BASE_OFFSET = 485330
TILE_BYTES_PER_SLOT = 875
TILE_SLOTS_PER_ROW = 40
TILE_ROW_BASE = 33
TILE_COL_BASE = 30

# World pickup row ID formula
WORLD_PICKUP_ROW_ID_BASE = 1037373320

# Dungeon formula
DUNGEON_BASES = {
    10: (4112, 1125, "Stormveil Castle"),
    11: (8612, 1125, "Leyndell Royal Capital"),
    12: (15362, 1125, "Underground"),
    13: (26612, 1125, "Crumbling Farum Azula"),
    14: (29987, 1125, "Academy of Raya Lucaria / Tutorial"),
    15: (33362, 1125, "Miquella's Haligtree"),
    16: (40517, 1125, "Volcano Manor"),
    18: (43487, 1125, "Roundtable Hold"),
    30: (27411, 1125, "Catacombs"),
    31: (28634, 1125, "Caves"),
    32: (31577, 1125, "Tunnels"),
}


def popcount(x):
    """Count bits set in a byte."""
    c = 0
    while x:
        c += x & 1
        x >>= 1
    return c


def is_single_bit_change(old_val, new_val):
    """True if exactly one bit differs between old and new."""
    return popcount(old_val ^ new_val) == 1


def bit_positions_changed(old_val, new_val):
    """Return list of (bit_position, direction) for each changed bit."""
    diff = old_val ^ new_val
    results = []
    for bit in range(8):
        if diff & (1 << (7 - bit)):
            direction = "SET" if (new_val & (1 << (7 - bit))) else "CLEARED"
            results.append((bit, direction))
    return results


def parse_diff_file(path):
    """Parse a binary diff file into list of (offset, old_val, new_val) records."""
    data = path.read_bytes()
    records = []
    for i in range(0, len(data), 6):
        if i + 6 > len(data):
            break
        offset = struct.unpack_from('<I', data, i)[0]
        old_val = data[i + 4]
        new_val = data[i + 5]
        records.append((offset, old_val, new_val))
    return records


def offset_to_block_flag_id(byte_offset):
    """Try to map a byte offset within EF section to a block-based flag ID.
    Returns list of (flag_id, block_name) or empty if no match."""
    results = []
    for block_start, (base_offset, block_size, name, status) in KNOWN_BLOCK_BASES.items():
        if base_offset <= byte_offset < base_offset + (block_size // 8) + 1:
            for bit in range(8):
                flag_id = block_start + (byte_offset - base_offset) * 8 + bit
                if block_start <= flag_id < block_start + block_size:
                    results.append((flag_id, name, status))
            break
    return results


def load_jsonl():
    """Load all entries from the JSONL timeline file."""
    entries = []
    with open(JSONL_PATH) as f:
        for line in f:
            line = line.strip()
            if line:
                entries.append(json.loads(line))
    return entries


def load_ground_truth_flags():
    """Load specific known flags from ground truth for matching."""
    try:
        with open(GROUND_TRUTH_PATH) as f:
            gt = json.load(f)
        known_flags = {}
        # Positive validation flags
        for vf in gt.get("event_flags_detection", {}).get("positive_validation_flags", []):
            known_flags[vf["flag_id"]] = {
                "name": vf["name"],
                "byte_offset": vf["byte_offset"],
                "bit_position": vf["bit_position"],
            }
        # Negative validation flags
        for vf in gt.get("event_flags_detection", {}).get("negative_validation_flags", []):
            known_flags[vf["flag_id"]] = {
                "name": vf["name"],
                "byte_offset": vf["byte_offset"],
                "bit_position": vf["bit_position"],
            }
        return known_flags
    except Exception as e:
        print(f"  Warning: could not load ground truth: {e}")
        return {}


def main():
    print("=" * 80)
    print("ELDEN RING SAVE FILE TIMELINE ANALYSIS")
    print("=" * 80)

    # ─── 1. Load JSONL metadata ──────────────────────────────────────────────
    entries = load_jsonl()
    print(f"\nTotal snapshots: {len(entries)}")
    print(f"Character: {entries[0]['characterName']} (Slot {entries[0]['slotIndex']})")
    ts_first = entries[0]["timestamp"]
    ts_last = entries[-1]["timestamp"]
    print(f"Time range: {ts_first} -> {ts_last}")

    bytes_changed = [(e["id"], e["bytesChanged"]) for e in entries]
    bytes_changed_sorted = sorted(bytes_changed, key=lambda x: x[1])

    print(f"\nSmallest diffs (most likely pure flag changes):")
    for sid, bc in bytes_changed_sorted[:15]:
        print(f"  {sid}: {bc} changes")

    print(f"\nLargest diffs:")
    for sid, bc in bytes_changed_sorted[-5:]:
        print(f"  {sid}: {bc} changes")

    # ─── 2. Parse all diff files ─────────────────────────────────────────────
    print("\n" + "=" * 80)
    print("PARSING ALL DIFF FILES")
    print("=" * 80)

    all_records_by_snapshot = {}
    all_offsets_global = set()
    offset_appearance_count = Counter()
    offset_single_bit_only = defaultdict(lambda: True)  # offset -> always single-bit?
    offset_single_bit_count = Counter()
    offset_multi_bit_count = Counter()

    for entry in entries:
        sid = entry["id"]
        diff_path = DIFFS_DIR / entry["diffFile"]
        if not diff_path.exists():
            print(f"  WARNING: {diff_path} not found")
            continue

        records = parse_diff_file(diff_path)
        all_records_by_snapshot[sid] = records

        for offset, old_val, new_val in records:
            all_offsets_global.add(offset)
            offset_appearance_count[offset] += 1
            if is_single_bit_change(old_val, new_val):
                offset_single_bit_count[offset] += 1
            else:
                offset_multi_bit_count[offset] += 1
                offset_single_bit_only[offset] = False

    total_offsets_seen = len(all_offsets_global)
    min_offset = min(all_offsets_global) if all_offsets_global else 0
    max_offset = max(all_offsets_global) if all_offsets_global else 0

    print(f"\nTotal unique offsets changed across all diffs: {total_offsets_seen}")
    print(f"Offset range: {min_offset} (0x{min_offset:X}) to {max_offset} (0x{max_offset:X})")

    # ─── 3. Identify noisy vs. rare offsets ──────────────────────────────────
    print("\n" + "=" * 80)
    print("OFFSET FREQUENCY ANALYSIS")
    print("=" * 80)

    total_snapshots = len(entries)

    # Offsets appearing in > 80% of snapshots = noisy
    noisy_threshold = int(total_snapshots * 0.80)
    noisy_offsets = {o for o, c in offset_appearance_count.items() if c >= noisy_threshold}
    rare_offsets = {o for o, c in offset_appearance_count.items() if c <= 3}
    moderate_offsets = all_offsets_global - noisy_offsets - rare_offsets

    print(f"\nNoisy offsets (appear in >{noisy_threshold}/{total_snapshots} snapshots): "
          f"{len(noisy_offsets)}")
    print(f"Moderate offsets (4-{noisy_threshold-1} appearances): {len(moderate_offsets)}")
    print(f"Rare offsets (1-3 appearances): {len(rare_offsets)}")

    # Show noisy offset ranges (contiguous runs)
    if noisy_offsets:
        sorted_noisy = sorted(noisy_offsets)
        ranges = []
        start = sorted_noisy[0]
        end = sorted_noisy[0]
        for o in sorted_noisy[1:]:
            if o <= end + 16:  # Allow small gaps
                end = o
            else:
                ranges.append((start, end))
                start = o
                end = o
        ranges.append((start, end))

        print(f"\nNoisy offset ranges (play time, animation state, etc.):")
        for s, e in ranges[:20]:
            count_in_range = sum(1 for o in sorted_noisy if s <= o <= e)
            print(f"  {s}-{e} (0x{s:X}-0x{e:X}) [{count_in_range} offsets, span {e-s+1} bytes]")
        if len(ranges) > 20:
            print(f"  ... and {len(ranges) - 20} more ranges")

    # ─── 4. Identify single-bit-only offsets ─────────────────────────────────
    print("\n" + "=" * 80)
    print("SINGLE-BIT CHANGE ANALYSIS (Event Flag Detection)")
    print("=" * 80)

    pure_single_bit_offsets = {o for o in all_offsets_global
                               if offset_single_bit_only.get(o, True)
                               and offset_single_bit_count[o] > 0
                               and offset_multi_bit_count[o] == 0}

    print(f"\nOffsets where ONLY single-bit changes occur: {len(pure_single_bit_offsets)}")

    if pure_single_bit_offsets:
        sorted_sb = sorted(pure_single_bit_offsets)
        # Group into contiguous ranges
        ranges = []
        start = sorted_sb[0]
        end = sorted_sb[0]
        for o in sorted_sb[1:]:
            if o <= end + 32:  # Allow gaps up to 32 bytes
                end = o
            else:
                ranges.append((start, end))
                start = o
                end = o
        ranges.append((start, end))

        print(f"\nSingle-bit-only offset ranges (likely event flag regions):")
        for s, e in sorted(ranges, key=lambda r: r[1] - r[0], reverse=True)[:25]:
            count_in_range = sum(1 for o in sorted_sb if s <= o <= e)
            avg_appearances = sum(offset_appearance_count[o] for o in sorted_sb if s <= o <= e) / max(count_in_range, 1)
            print(f"  {s}-{e} (0x{s:X}-0x{e:X}) "
                  f"[{count_in_range} offsets, span {e-s+1} bytes, avg appearances: {avg_appearances:.1f}]")
        if len(ranges) > 25:
            print(f"  ... and {len(ranges) - 25} more ranges")

    # ─── 5. Deep analysis of small diffs ─────────────────────────────────────
    print("\n" + "=" * 80)
    print("SMALL DIFF DETAILED ANALYSIS")
    print("=" * 80)

    # Analyze snapshots with < 5000 changes
    small_diffs = [(e["id"], e["bytesChanged"], e) for e in entries if e["bytesChanged"] < 5000]
    small_diffs.sort(key=lambda x: x[1])

    known_flags = load_ground_truth_flags()

    for sid, bc, entry in small_diffs:
        records = all_records_by_snapshot.get(sid, [])
        if not records:
            continue

        offsets = [r[0] for r in records]
        single_bit_records = [(o, old, new) for o, old, new in records
                              if is_single_bit_change(old, new)]
        multi_bit_records = [(o, old, new) for o, old, new in records
                             if not is_single_bit_change(old, new)]

        print(f"\n--- {sid} ({bc} changes) ---")
        ts = entry.get("timestamp", "?")
        pos = entry.get("playerPosition")
        if pos and pos.get("mapId"):
            mid = pos["mapId"]
            print(f"  Time: {ts}")
            print(f"  Map: {mid}  Position: ({pos.get('x',0):.1f}, {pos.get('y',0):.1f}, {pos.get('z',0):.1f})")
        else:
            print(f"  Time: {ts}")

        inv = entry.get("inventoryDelta")
        if inv:
            added = inv.get("added", [])
            removed = inv.get("removed", [])
            if added:
                print(f"  Inventory added: {len(added)} items")
            if removed:
                print(f"  Inventory removed: {len(removed)} items")

        print(f"  Total records: {len(records)}")
        print(f"  Single-bit changes: {len(single_bit_records)}")
        print(f"  Multi-bit changes: {len(multi_bit_records)}")

        if single_bit_records:
            sb_offsets = sorted(set(r[0] for r in single_bit_records))
            print(f"  Single-bit offset range: {sb_offsets[0]}-{sb_offsets[-1]} "
                  f"(0x{sb_offsets[0]:X}-0x{sb_offsets[-1]:X})")

            print(f"\n  Single-bit changes detail (flag candidates):")
            for offset, old_val, new_val in sorted(single_bit_records):
                bits = bit_positions_changed(old_val, new_val)
                noisy_marker = " [NOISY]" if offset in noisy_offsets else ""
                for bit_pos, direction in bits:
                    # Try to match against known block bases
                    matches = offset_to_block_flag_id(offset)
                    match_str = ""
                    if matches:
                        for fid, bname, bstatus in matches:
                            if fid % 8 == bit_pos or (7 - (fid % 8)) == bit_pos:
                                known = known_flags.get(fid, {})
                                name = known.get("name", "")
                                if name:
                                    match_str = f" -> flag {fid} ({name}) [{bstatus}]"
                                else:
                                    match_str = f" -> flag {fid} ({bname}) [{bstatus}]"

                    print(f"    offset {offset} (0x{offset:X}): "
                          f"0x{old_val:02X}->0x{new_val:02X} bit {bit_pos} {direction}"
                          f"{match_str}{noisy_marker}")

        if multi_bit_records and len(multi_bit_records) <= 30:
            print(f"\n  Multi-bit changes detail:")
            for offset, old_val, new_val in sorted(multi_bit_records)[:30]:
                bits_changed = popcount(old_val ^ new_val)
                noisy_marker = " [NOISY]" if offset in noisy_offsets else ""
                print(f"    offset {offset} (0x{offset:X}): "
                      f"0x{old_val:02X}->0x{new_val:02X} ({bits_changed} bits){noisy_marker}")

    # ─── 6. Event flag region discovery ──────────────────────────────────────
    print("\n" + "=" * 80)
    print("EVENT FLAG REGION DISCOVERY")
    print("=" * 80)

    # Collect all single-bit changes across all snapshots
    all_single_bit_by_offset = defaultdict(list)  # offset -> [(snapshot, old, new, bit, dir)]

    for sid, records in all_records_by_snapshot.items():
        for offset, old_val, new_val in records:
            if is_single_bit_change(old_val, new_val):
                bits = bit_positions_changed(old_val, new_val)
                for bit_pos, direction in bits:
                    all_single_bit_by_offset[offset].append(
                        (sid, old_val, new_val, bit_pos, direction))

    # Find offsets with ONLY SET transitions (never CLEARED) -> likely new discoveries
    set_only_offsets = {}
    for offset, changes in all_single_bit_by_offset.items():
        if offset in noisy_offsets:
            continue
        set_changes = [c for c in changes if c[4] == "SET"]
        clear_changes = [c for c in changes if c[4] == "CLEARED"]
        if set_changes and not clear_changes:
            set_only_offsets[offset] = set_changes

    print(f"\nOffsets with ONLY SET (never CLEARED) single-bit changes "
          f"(excluding noisy): {len(set_only_offsets)}")
    print("These strongly suggest permanent event flags being set during gameplay.\n")

    # Group by contiguous regions
    if set_only_offsets:
        sorted_offsets = sorted(set_only_offsets.keys())
        regions = []
        region_start = sorted_offsets[0]
        region_end = sorted_offsets[0]
        region_offsets = [sorted_offsets[0]]

        for o in sorted_offsets[1:]:
            if o <= region_end + 64:  # Allow gaps up to 64 bytes
                region_end = o
                region_offsets.append(o)
            else:
                regions.append((region_start, region_end, list(region_offsets)))
                region_start = o
                region_end = o
                region_offsets = [o]
        regions.append((region_start, region_end, list(region_offsets)))

        regions.sort(key=lambda r: len(r[2]), reverse=True)
        print("SET-only event flag regions (sorted by density):")
        for start, end, offsets in regions[:15]:
            print(f"\n  Region: {start}-{end} (0x{start:X}-0x{end:X}) "
                  f"[{len(offsets)} offsets, span {end-start+1} bytes]")
            for o in sorted(offsets)[:20]:
                changes = set_only_offsets[o]
                for sid, old_val, new_val, bit_pos, direction in changes:
                    print(f"    offset {o} (0x{o:X}) bit {bit_pos}: "
                          f"0x{old_val:02X}->0x{new_val:02X} in {sid}")
            if len(offsets) > 20:
                print(f"    ... and {len(offsets) - 20} more offsets")

    # ─── 7. Try to match against known formulas ─────────────────────────────
    print("\n" + "=" * 80)
    print("FLAG ID MAPPING ATTEMPTS")
    print("=" * 80)

    # Collect ALL single-bit changes from small diffs for flag ID mapping
    # Try different base offsets to see if any produce recognizable flag IDs

    all_flag_like_changes = []
    for sid, records in all_records_by_snapshot.items():
        entry_map = {e["id"]: e for e in entries}
        if entry_map[sid]["bytesChanged"] > 10000:
            continue
        for offset, old_val, new_val in records:
            if is_single_bit_change(old_val, new_val) and offset not in noisy_offsets:
                bits = bit_positions_changed(old_val, new_val)
                for bit_pos, direction in bits:
                    all_flag_like_changes.append((offset, bit_pos, direction, sid))

    print(f"\nTotal flag-like changes in small diffs (non-noisy): {len(all_flag_like_changes)}")

    # Try each known block base to see if any offsets match
    print("\n--- Block Formula Matching ---")
    for block_start, (base_offset, block_size, name, status) in sorted(KNOWN_BLOCK_BASES.items()):
        matches = []
        for offset, bit_pos, direction, sid in all_flag_like_changes:
            if base_offset <= offset < base_offset + (block_size // 8) + 1:
                flag_id = block_start + (offset - base_offset) * 8 + bit_pos
                if block_start <= flag_id < block_start + block_size:
                    matches.append((flag_id, offset, bit_pos, direction, sid))

        if matches:
            print(f"\n  Block {block_start} ({name}) [base={base_offset}, {status}]:")
            for fid, offset, bit_pos, direction, sid in sorted(matches):
                known = known_flags.get(fid, {})
                known_name = known.get("name", "")
                name_str = f" = {known_name}" if known_name else ""
                print(f"    Flag {fid}{name_str}: offset {offset} bit {bit_pos} "
                      f"{direction} in {sid}")

    # Try dungeon formula matching
    print("\n--- Dungeon Formula Matching ---")
    for area_id, (base_offset, section_size, area_name) in sorted(DUNGEON_BASES.items()):
        matches = []
        for offset, bit_pos, direction, sid in all_flag_like_changes:
            # Check if offset falls within any section of this area
            for section in range(23):  # max sections
                section_start = base_offset + section * section_size
                section_end = section_start + section_size
                if section_start <= offset < section_end:
                    local_id = (offset - section_start) * 8 + bit_pos
                    flag_id = area_id * 1000000 + section * 10000 + local_id
                    matches.append((flag_id, offset, bit_pos, direction, sid, section, local_id))
                    break

        if matches:
            print(f"\n  Area {area_id} ({area_name}) [base={base_offset}]:")
            for fid, offset, bit_pos, direction, sid, section, local_id in sorted(matches)[:20]:
                print(f"    Flag {fid} (section {section}, local {local_id}): "
                      f"offset {offset} bit {bit_pos} {direction} in {sid}")
            if len(matches) > 20:
                print(f"    ... and {len(matches) - 20} more")

    # ─── 8. Summary statistics ───────────────────────────────────────────────
    print("\n" + "=" * 80)
    print("OVERALL SUMMARY")
    print("=" * 80)

    total_records = sum(len(r) for r in all_records_by_snapshot.values())
    total_single_bit = sum(
        sum(1 for _, old, new in records if is_single_bit_change(old, new))
        for records in all_records_by_snapshot.values()
    )

    print(f"\nTotal snapshots: {len(entries)}")
    print(f"Total byte-change records: {total_records}")
    print(f"Total single-bit changes: {total_single_bit} ({100*total_single_bit/max(total_records,1):.1f}%)")
    print(f"Total unique offsets touched: {len(all_offsets_global)}")
    print(f"Offsets always single-bit: {len(pure_single_bit_offsets)}")
    print(f"SET-only event flag candidates: {len(set_only_offsets)}")
    print(f"Noisy offsets (counters etc.): {len(noisy_offsets)}")

    # Histogram of bytesChanged
    print(f"\nbytesChanged distribution:")
    brackets = [(0, 100), (100, 1000), (1000, 5000), (5000, 50000),
                (50000, 200000), (200000, 500000)]
    for lo, hi in brackets:
        count = sum(1 for e in entries if lo <= e["bytesChanged"] < hi)
        if count:
            print(f"  {lo:>7}-{hi:<7}: {count} snapshots")

    # ─── 9. Offset region heatmap ────────────────────────────────────────────
    print("\n" + "=" * 80)
    print("OFFSET REGION HEATMAP (64-byte buckets)")
    print("=" * 80)

    bucket_size = 64
    bucket_counts = Counter()
    bucket_single_bit_ratio = defaultdict(lambda: [0, 0])  # [single, total]

    for records in all_records_by_snapshot.values():
        for offset, old_val, new_val in records:
            bucket = (offset // bucket_size) * bucket_size
            bucket_counts[bucket] += 1
            bucket_single_bit_ratio[bucket][1] += 1
            if is_single_bit_change(old_val, new_val):
                bucket_single_bit_ratio[bucket][0] += 1

    # Show buckets with high single-bit ratio and moderate frequency
    print(f"\nBuckets with >80% single-bit changes AND >5 total changes "
          f"(strong flag region indicators):")
    flag_region_buckets = []
    for bucket in sorted(bucket_single_bit_ratio.keys()):
        single, total = bucket_single_bit_ratio[bucket]
        if total >= 5 and single / total > 0.80:
            flag_region_buckets.append((bucket, single, total))
            print(f"  0x{bucket:06X}-0x{bucket+bucket_size-1:06X} "
                  f"({bucket}-{bucket+bucket_size-1}): "
                  f"{single}/{total} single-bit ({100*single/total:.0f}%)")

    # Group these into contiguous flag regions
    if flag_region_buckets:
        print(f"\nContiguous high-single-bit regions:")
        sorted_fb = [b[0] for b in flag_region_buckets]
        regions = []
        region_start = sorted_fb[0]
        region_end = sorted_fb[0] + bucket_size
        for b in sorted_fb[1:]:
            if b <= region_end + bucket_size:
                region_end = b + bucket_size
            else:
                regions.append((region_start, region_end))
                region_start = b
                region_end = b + bucket_size
        regions.append((region_start, region_end))

        for s, e in regions:
            size = e - s
            total_in_region = sum(t for b, sb, t in flag_region_buckets if s <= b < e)
            single_in_region = sum(sb for b, sb, t in flag_region_buckets if s <= b < e)
            print(f"  {s}-{e} (0x{s:X}-0x{e:X}) "
                  f"[{size} bytes, {single_in_region}/{total_in_region} single-bit]")

    print("\n" + "=" * 80)
    print("ANALYSIS COMPLETE")
    print("=" * 80)


if __name__ == "__main__":
    main()
