#!/usr/bin/env python3
"""
Measure the _pre_event_flags_gap across all available save data.

For each save slot, performs:
1. GaItems parsing to find variable-length section end (matching WASM find_ga_items_end)
2. Sequential parsing of all intermediate fixed/variable sections (matching save_slot.rs)
3. Content-based EventFlags detection (matching WASM detect_event_flags_offset_impl)
4. Gap measurement: detected_EF_offset - position_after_TutorialData

Output: gap size distribution across all slots/snapshots, revealing whether
the gap is constant (always 0x1d) or variable.
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path
from collections import Counter
from typing import Optional, List, Dict, Tuple

# =============================================================================
# BND4 CONTAINER CONSTANTS
# =============================================================================
BND4_HEADER_SIZE = 0x40
BND4_ENTRY_SIZE = 0x20
BND4_ENTRY_OFFSET_POS = 0x10
SLOT_CHECKSUM_SIZE = 16
SLOT_SIZE = 0x280000
SLOT_COUNT = 10

# =============================================================================
# GAITEM CONSTANTS
# =============================================================================
GAITEM_MAX_COUNT = 0x1400  # 5120

# =============================================================================
# EVENTFLAGS CONSTANTS
# =============================================================================
EVENT_FLAGS_SIZE = 0x1BF99F

# Validation flags (matching WASM POSITIVE_VALIDATION_FLAGS)
POSITIVE_VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge", 1),
    (71801, 2725, 6, "Stranded Graveyard", 1),
    (76100, 3262, 3, "The First Step", 1),
    (76101, 3262, 2, "Church of Elleh", 1),
    (76102, 3262, 1, "Stormhill Shack", 2),
    (76104, 3263, 7, "Agheel Lake South", 2),
    (76106, 3263, 5, "Church of Dragon Communion", 2),
]

NEGATIVE_VALIDATION_FLAGS = [
    (76223, 3277, 0, "Fortified Manor, First Floor"),
    (76224, 3278, 7, "East Capital Rampart"),
    (76225, 3278, 6, "Divine Bridge"),
    (76300, 3287, 3, "Zamor Ruins"),
    (76301, 3287, 2, "Ancient Snow Valley Ruins"),
    (76350, 3293, 5, "Haligtree Town"),
]


# =============================================================================
# SECTION SIZES (from save_slot.rs, verified against source)
# =============================================================================
# EquipInventoryItem = 3 × u32 = 12 bytes
# EquipInventoryData(common_count, key_count) = 4 + common*12 + 4 + key*12 + 4 + 4

SECTION_SIZES = {
    # Fixed sections before EquipProjectileData (first variable section)
    'PlayerGameData': 0x1B0,        # 432
    'Padding_0xD0': 0xD0,           # 208
    'EquipData': 0x58,              # 88
    'ChrAsm': 0x74,                 # 116
    'ChrAsm2': 0x58,                # 88
    'EquipInventoryData': 0x9010,   # 4 + 0xA80*12 + 4 + 0x180*12 + 4 + 4 = 36,880
    'EquipMagicData': 0x74,         # 116
    'EquipItemData': 0x8C,          # 140
    'EquipGestureData': 0x18,       # 6 * 4 = 24

    # --- VARIABLE: EquipProjectileData = 4 + count * 8 ---

    # Fixed sections between EquipProjectileData and Regions
    'EquippedItems': 0x9C,          # 156
    'EquipPhysicsData': 0x08,       # 8
    'Padding_0x4': 0x04,            # 4
    'FaceData': 0x12F,              # 303
    'StorageInventoryData': 4 + 0x780 * 12 + 4 + 0x80 * 12 + 4 + 4,  # 24,592
    'GestureGameData': 0x100,       # 0x40 * 4 = 256

    # --- VARIABLE: Regions = 4 + count * 4 ---

    # Fixed sections after Regions through TutorialData
    'RideGameData': 0x28,           # 40
    'Misc_0x1': 0x01,               # 1
    'Misc_0x40': 0x40,              # 64
    'Misc_3xi32': 0x0C,             # 12
    'MenuProfileSaveLoad': 0x1008,  # 4,104
    'TrophyEquipData': 0x34,        # 52
    'GaItemData': 4 + 4 + 0x1B58 * 16,  # 8 + 7000*16 = 112,008
    'TutorialData': 0x408,          # 1,032
}

# Pre-compute section groups
FIXED_BEFORE_PROJ = sum([
    SECTION_SIZES['PlayerGameData'],
    SECTION_SIZES['Padding_0xD0'],
    SECTION_SIZES['EquipData'],
    SECTION_SIZES['ChrAsm'],
    SECTION_SIZES['ChrAsm2'],
    SECTION_SIZES['EquipInventoryData'],
    SECTION_SIZES['EquipMagicData'],
    SECTION_SIZES['EquipItemData'],
    SECTION_SIZES['EquipGestureData'],
])

FIXED_BETWEEN_PROJ_AND_REGIONS = sum([
    SECTION_SIZES['EquippedItems'],
    SECTION_SIZES['EquipPhysicsData'],
    SECTION_SIZES['Padding_0x4'],
    SECTION_SIZES['FaceData'],
    SECTION_SIZES['StorageInventoryData'],
    SECTION_SIZES['GestureGameData'],
])

FIXED_AFTER_REGIONS = sum([
    SECTION_SIZES['RideGameData'],
    SECTION_SIZES['Misc_0x1'],
    SECTION_SIZES['Misc_0x40'],
    SECTION_SIZES['Misc_3xi32'],
    SECTION_SIZES['MenuProfileSaveLoad'],
    SECTION_SIZES['TrophyEquipData'],
    SECTION_SIZES['GaItemData'],
    SECTION_SIZES['TutorialData'],
])


def read_bnd4_slot_offsets(data: bytes) -> list:
    """Read slot data offsets from BND4 file entries."""
    offsets = []
    for i in range(SLOT_COUNT):
        entry_offset = BND4_HEADER_SIZE + (i * BND4_ENTRY_SIZE) + BND4_ENTRY_OFFSET_POS
        if entry_offset + 4 <= len(data):
            bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
            offsets.append(bnd4_offset + SLOT_CHECKSUM_SIZE)
        else:
            offsets.append(0)
    return offsets


def find_ga_items_end(slot_data: bytes) -> int | None:
    """Mirror of WASM find_ga_items_end."""
    if len(slot_data) < 12:
        return None

    version = struct.unpack_from('<I', slot_data, 0)[0]
    header_padding = 0x8 if version == 81 else 0x18
    pos = 4 + 4 + header_padding

    for _ in range(GAITEM_MAX_COUNT):
        if pos + 8 > len(slot_data):
            return None
        pos += 4  # gaitem_handle
        item_id = struct.unpack_from('<I', slot_data, pos)[0]
        pos += 4  # item_id

        if item_id == 0 or item_id == 0xFFFFFFFF:
            continue

        category = item_id & 0xF0000000
        if category == 0x00000000:  # Weapon
            pos += 13
        elif category == 0x10000000:  # Armor
            pos += 8

        if pos > len(slot_data):
            return None

    return pos


def detect_ef_offset(slot_data: bytes) -> tuple:
    """Content-based EF detection (matching WASM algorithm)."""
    search_start = 0x30000
    search_end = min(search_start + 200_000, len(slot_data) - 10000)

    tier1_flags = [f for f in POSITIVE_VALIDATION_FLAGS if f[4] == 1]
    tier1_count = len(tier1_flags)

    best_offset = search_start
    best_tier1 = 0
    best_pos_score = 0
    best_neg_score = 0

    for test_offset in range(search_start, search_end):
        tier1 = 0
        pos_score = 0

        for _, byte_off, bit_pos, _, tier in POSITIVE_VALIDATION_FLAGS:
            abs_pos = test_offset + byte_off
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if (byte_val & (1 << bit_pos)) != 0:
                    pos_score += 1
                    if tier == 1:
                        tier1 += 1

        if tier1 >= 2:
            neg_score = 0
            for _, byte_off, bit_pos, _ in NEGATIVE_VALIDATION_FLAGS:
                abs_pos = test_offset + byte_off
                if abs_pos < len(slot_data):
                    byte_val = slot_data[abs_pos]
                    if (byte_val & (1 << bit_pos)) == 0:
                        neg_score += 1

            is_better = (
                tier1 > best_tier1 or
                (tier1 == best_tier1 and neg_score > best_neg_score) or
                (tier1 == best_tier1 and neg_score == best_neg_score and pos_score > best_pos_score)
            )
            if is_better:
                best_tier1 = tier1
                best_pos_score = pos_score
                best_neg_score = neg_score
                best_offset = test_offset

    return best_offset, best_tier1, best_pos_score


def compute_structural_position(slot_data: bytes, ga_items_end: int) -> tuple:
    """
    Compute position after TutorialData by sequential parsing.
    Returns (post_tutorial_pos, details_dict) or (None, {}).
    """
    pos = ga_items_end

    # Fixed sections before EquipProjectileData
    pos += FIXED_BEFORE_PROJ

    # EquipProjectileData (VARIABLE: 4 + count * 8)
    if pos + 4 > len(slot_data):
        return None, {}
    proj_count = struct.unpack_from('<i', slot_data, pos)[0]
    proj_count = max(0, proj_count)
    proj_size = 4 + proj_count * 8
    pos += proj_size

    # Fixed sections between projectile and regions
    pos += FIXED_BETWEEN_PROJ_AND_REGIONS

    # Regions (VARIABLE: 4 + count * 4)
    if pos + 4 > len(slot_data):
        return None, {}
    regions_count = struct.unpack_from('<I', slot_data, pos)[0]
    regions_size = 4 + regions_count * 4
    pos += regions_size

    # Fixed sections after regions through TutorialData
    pos += FIXED_AFTER_REGIONS

    details = {
        'proj_count': proj_count,
        'proj_size': proj_size,
        'regions_count': regions_count,
        'regions_size': regions_size,
    }

    return pos, details


PRE_EF_GAP = 0x1D  # 29 bytes - hypothesized constant gap


def validate_at_offset(slot_data: bytes, ef_offset: int) -> dict:
    """Validate grace flags at a candidate EF offset."""
    tier1 = 0
    pos_score = 0
    neg_score = 0
    matched_graces = []

    for _, byte_off, bit_pos, name, tier in POSITIVE_VALIDATION_FLAGS:
        abs_pos = ef_offset + byte_off
        if abs_pos < len(slot_data):
            byte_val = slot_data[abs_pos]
            if (byte_val & (1 << bit_pos)) != 0:
                pos_score += 1
                if tier == 1:
                    tier1 += 1
                matched_graces.append(name)

    for _, byte_off, bit_pos, _ in NEGATIVE_VALIDATION_FLAGS:
        abs_pos = ef_offset + byte_off
        if abs_pos < len(slot_data):
            byte_val = slot_data[abs_pos]
            if (byte_val & (1 << bit_pos)) == 0:
                neg_score += 1

    return {
        'tier1': tier1,
        'pos_score': pos_score,
        'neg_score': neg_score,
        'graces': matched_graces,
    }


def analyze_slot(slot_data: bytes, slot_index: int, source_name: str) -> Optional[dict]:
    """Analyze a single slot and return gap measurement."""
    ga_end = find_ga_items_end(slot_data)
    if ga_end is None:
        return None

    # Content-based detection
    detected_ef, tier1, pos_score = detect_ef_offset(slot_data)

    # Structural computation
    post_tutorial, details = compute_structural_position(slot_data, ga_end)
    if post_tutorial is None:
        return None

    # Structural EF offset (post_tutorial + 29-byte gap)
    structural_ef = post_tutorial + PRE_EF_GAP

    # Validate both offsets using grace flags
    content_validation = validate_at_offset(slot_data, detected_ef)
    structural_validation = validate_at_offset(slot_data, structural_ef)

    gap = detected_ef - post_tutorial
    match = detected_ef == structural_ef

    return {
        'source': source_name,
        'slot': slot_index,
        'ga_items_end': ga_end,
        'detected_ef': detected_ef,
        'structural_ef': structural_ef,
        'post_tutorial': post_tutorial,
        'gap': gap,
        'gap_hex': f'0x{gap:X}' if gap >= 0 else f'-0x{-gap:X}',
        'match': match,
        # Content-based validation
        'content_tier1': content_validation['tier1'],
        'content_pos': content_validation['pos_score'],
        'content_neg': content_validation['neg_score'],
        # Structural validation
        'struct_tier1': structural_validation['tier1'],
        'struct_pos': structural_validation['pos_score'],
        'struct_neg': structural_validation['neg_score'],
        'struct_graces': structural_validation['graces'],
        **details,
    }


def analyze_save_file(filepath: Path, label: str = None) -> list:
    """Analyze all slots in a save file."""
    if label is None:
        label = filepath.name

    data = filepath.read_bytes()
    if data[:4] != b'BND4':
        print(f"  WARNING: {filepath.name} is not a valid BND4 file, skipping")
        return []

    slot_offsets = read_bnd4_slot_offsets(data)
    results = []

    for i in range(SLOT_COUNT):
        offset = slot_offsets[i]
        if offset == 0:
            continue
        if offset + SLOT_SIZE > len(data):
            slot_data = data[offset:]
        else:
            slot_data = data[offset:offset + SLOT_SIZE]

        if len(slot_data) < 0x100:
            continue

        # Skip empty slots (version 0)
        version = struct.unpack_from('<I', slot_data, 0)[0]
        if version == 0:
            continue

        result = analyze_slot(slot_data, i, f"{label}:slot{i}")
        if result:
            results.append(result)

    return results


def hex_dump_gap(slot_data: bytes, post_tutorial: int, gap_size: int, limit: int = 64):
    """Hex dump the gap bytes for structural analysis."""
    start = post_tutorial
    end = start + min(gap_size, limit)
    if end > len(slot_data):
        end = len(slot_data)
    gap_bytes = slot_data[start:end]
    hex_str = ' '.join(f'{b:02X}' for b in gap_bytes)
    return hex_str


def main():
    save_dir = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
    snapshot_dir = save_dir / "Granular snapshots for debugging"

    all_results = []

    # 1. Analyze backup save files
    print("=" * 80)
    print("PHASE 1: Backup save files")
    print("=" * 80)
    for sl2_file in sorted(save_dir.glob("*.sl2")):
        print(f"\nAnalyzing: {sl2_file.name}")
        results = analyze_save_file(sl2_file)
        for r in results:
            status = "MATCH" if r['match'] else "MISMATCH"
            print(f"  Slot {r['slot']}: [{status}] gap={r['gap']} ({r['gap_hex']})")
            print(f"    Content:    EF=0x{r['detected_ef']:X} tier1={r['content_tier1']} pos={r['content_pos']} neg={r['content_neg']}")
            print(f"    Structural: EF=0x{r['structural_ef']:X} tier1={r['struct_tier1']} pos={r['struct_pos']} neg={r['struct_neg']}")
            print(f"    Graces at structural: {', '.join(r['struct_graces']) or 'None'}")
            print(f"    proj={r['proj_count']}, regions={r['regions_count']}")
        all_results.extend(results)

    # 2. Analyze granular snapshots
    print("\n" + "=" * 80)
    print("PHASE 2: Granular snapshots")
    print("=" * 80)
    snapshot_files = []
    if snapshot_dir.exists():
        # Files are named like "ER0000.sl2 - 119 s2-V1 ..." (no standard extension)
        for f in sorted(snapshot_dir.iterdir()):
            if f.is_file() and f.name.startswith("ER0000") and f.stat().st_size > 100_000:
                snapshot_files.append(f)
        # Also check slot-specific directories
        for subdir in sorted(snapshot_dir.iterdir()):
            if subdir.is_dir():
                for f in sorted(subdir.iterdir()):
                    if f.is_file() and f.name.startswith("ER0000") and f.stat().st_size > 100_000:
                        snapshot_files.append(f)

    seen = set()
    snapshot_struct_ok = 0
    snapshot_struct_fail = 0
    for sl2_file in snapshot_files:
        canon = str(sl2_file.resolve())
        if canon in seen:
            continue
        seen.add(canon)

        results = analyze_save_file(sl2_file, sl2_file.name[:60])
        for r in results:
            if r['struct_tier1'] >= 2:
                snapshot_struct_ok += 1
            else:
                snapshot_struct_fail += 1
            # Only print first 10 and mismatches
            if len(all_results) < 20 or not r['match']:
                status = "MATCH" if r['match'] else "MISMATCH"
                print(f"  [{r['source'][:40]}] Slot {r['slot']}: [{status}] "
                      f"struct_t1={r['struct_tier1']} struct_pos={r['struct_pos']}")
        all_results.extend(results)
    print(f"  Structural validation: {snapshot_struct_ok} OK, {snapshot_struct_fail} insufficient tier1")

    # 3. Analyze timeline snapshots
    print("\n" + "=" * 80)
    print("PHASE 3: Timeline snapshots")
    print("=" * 80)
    timeline_dir = snapshot_dir / "timeline"
    if timeline_dir.exists():
        timeline_files = sorted(timeline_dir.glob("*.sl2"))
        sample_count = 0
        for sl2_file in timeline_files:
            if sl2_file.stat().st_size < 100_000:
                continue
            results = analyze_save_file(sl2_file, sl2_file.name[:40])
            if results:
                sample_count += 1
                # Only print every 10th for brevity
                if sample_count <= 5 or sample_count % 10 == 0:
                    for r in results:
                        print(f"  [{r['source'][:30]}] Slot {r['slot']}: gap={r['gap']} ({r['gap_hex']})")
            all_results.extend(results)
        print(f"  ... {sample_count} timeline files analyzed")

    # ==========================================================================
    # SUMMARY
    # ==========================================================================
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)

    if not all_results:
        print("No results! No save data could be analyzed.")
        return

    print(f"\nTotal measurements: {len(all_results)}")

    # Structural validation analysis
    struct_validated = [r for r in all_results if r['struct_tier1'] >= 2]
    struct_perfect = [r for r in all_results if r['struct_tier1'] >= 4]

    print(f"\nStructural offset validation:")
    print(f"  tier1 >= 2 (minimum usable): {len(struct_validated)} / {len(all_results)}")
    print(f"  tier1 == 4 (perfect):        {len(struct_perfect)} / {len(all_results)}")

    # Check if structural is ALWAYS correct (tier1 >= 2 at structural offset)
    struct_fail = [r for r in all_results if r['struct_tier1'] < 2]
    if struct_fail:
        print(f"\n  WARNING: {len(struct_fail)} slots had struct_tier1 < 2:")
        for r in struct_fail:
            print(f"    {r['source']}: struct_tier1={r['struct_tier1']}, graces={r['struct_graces']}")
    else:
        print(f"\n  *** ALL slots validate at structural offset (tier1 >= 2) ***")

    # Content vs structural comparison
    content_better = [r for r in all_results if r['content_tier1'] > r['struct_tier1']]
    struct_better = [r for r in all_results if r['struct_tier1'] > r['content_tier1']]
    equal = [r for r in all_results if r['content_tier1'] == r['struct_tier1']]
    matches = [r for r in all_results if r['match']]

    print(f"\nContent vs Structural comparison:")
    print(f"  Offsets match:           {len(matches)} / {len(all_results)}")
    print(f"  Content better tier1:    {len(content_better)}")
    print(f"  Structural better tier1: {len(struct_better)}")
    print(f"  Equal tier1:             {len(equal)}")

    if struct_better:
        print(f"\n  Structural detection OUTPERFORMS content-based for:")
        for r in struct_better:
            print(f"    {r['source']}: content_t1={r['content_tier1']} vs struct_t1={r['struct_tier1']}")


if __name__ == '__main__':
    main()
