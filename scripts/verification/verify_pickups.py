#!/usr/bin/env python3
"""
Pickup Flag Verification — comprehensive check of tile, dungeon, and block pickup formulas.

Loads a save file, detects EF offset, calibrates tile base, and verifies all known
pickup flags against actual event flags data. Reports coverage and accuracy.

Usage:
    python scripts/verification/verify_pickups.py --save /path/to/ER0000.sl2 --slot 5
    python scripts/verification/verify_pickups.py --slot 0  # uses default backup save
    python scripts/verification/verify_pickups.py --all-slots  # verify all 10 slots
    python scripts/verification/verify_pickups.py --slot 0 --json /tmp/results.json
"""

import argparse
import json
import sys
from pathlib import Path
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional, Tuple, Any

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.utils import (
    read_slot_data,
    detect_event_flags_start,
    extract_event_flags,
    check_flag,
    is_likely_false_positive,
)
from scripts.verification.ground_truth_loader import (
    load_block_bases,
    load_dungeon_bases,
    get_tile_config,
    calculate_block_offset,
    calculate_tile_offset,
    calculate_dungeon_offset,
    calculate_tile_offset_calibrated,
)
from scripts.verification.calibration import CalibrationService
from scripts.verification.constants import (
    DEFAULT_SAVE_DIR,
    SLOT_COUNT,
    EVENT_FLAGS_SIZE,
)

# ============================================================================
# PATHS
# ============================================================================

LIVE_SAVE = Path(
    "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/"
    "Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/"
    "76561197969778805/ER0000.sl2"
)
BACKUP_SAVE = DEFAULT_SAVE_DIR / "ER0000-backup-2026-01-11.sl2"
DECOMPILED_DIR = Path(
    "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files"
)


# ============================================================================
# DATA CLASSES
# ============================================================================

@dataclass
class FlagResult:
    flag_id: int
    name: str
    formula_type: str  # "tile", "dungeon", "block"
    byte_offset: int
    bit_position: int
    is_set: bool
    is_false_positive: bool
    error: Optional[str] = None


@dataclass
class SlotReport:
    slot_index: int
    save_path: str
    ef_offset: Optional[int]
    ef_confident: bool
    tile_base: Optional[int]
    tile_base_confidence: float
    tile_base_source: str

    tile_flags_checked: int = 0
    tile_flags_set: int = 0
    tile_false_positives: int = 0

    dungeon_flags_checked: int = 0
    dungeon_flags_set: int = 0
    dungeon_false_positives: int = 0

    block_flags_checked: int = 0
    block_flags_set: int = 0
    block_false_positives: int = 0

    errors: List[str] = field(default_factory=list)
    details: List[Dict[str, Any]] = field(default_factory=list)


# ============================================================================
# FLAG COLLECTION — build lists of flags to verify
# ============================================================================

def collect_tile_flags_from_scan(
    event_flags: bytes, tile_base: int
) -> List[Tuple[int, str]]:
    """Scan tile section for non-zero bytes and report which tile flags are SET."""
    config = get_tile_config()
    bytes_per_slot = config.get("bytes_per_slot", 875)
    slots_per_row = config.get("slots_per_row", 40)
    row_base = config.get("row_base", 33)
    col_base = config.get("col_base", 30)
    max_local_id = config.get("max_local_id", 6999)

    results = []
    # Scan reasonable tile range (rows 30-55, cols 25-55)
    for row in range(30, 56):
        for col in range(25, 56):
            slot = (row - row_base) * slots_per_row + (col - col_base)
            if slot < 0:
                continue
            slot_start = tile_base + slot * bytes_per_slot
            slot_end = slot_start + bytes_per_slot

            if slot_end > len(event_flags):
                continue

            for byte_idx in range(slot_start, slot_end):
                byte_val = event_flags[byte_idx]
                if byte_val == 0:
                    continue
                local_byte = byte_idx - slot_start
                for bit in range(8):
                    if (byte_val >> bit) & 1:
                        local_id = local_byte * 8 + (7 - bit)
                        if local_id <= max_local_id:
                            flag_id = 1_000_000_000 + row * 1_000_000 + col * 10_000 + local_id
                            results.append((flag_id, f"tile({row},{col}):{local_id}"))

    return results


def collect_grace_flags() -> List[Tuple[int, str]]:
    """Collect known grace flags from block bases."""
    bases = load_block_bases()
    flags = []

    grace_blocks = {
        71800: ("Tutorial graces", 200),
        72000: ("DLC graces", 1000),
        73000: ("Dungeon graces", 1000),
        74000: ("DLC dungeon graces", 1000),
        76000: ("World graces", 2000),
        78000: ("Grace guidance", 500),
    }

    for block_start, (label, scan_range) in grace_blocks.items():
        if block_start in bases and bases[block_start].get("status") in ("verified", "partial"):
            for i in range(scan_range):
                flag_id = block_start + i
                flags.append((flag_id, f"{label}:{flag_id}"))

    return flags


def collect_progression_flags() -> List[Tuple[int, str]]:
    """Collect progression flags (block 60000)."""
    bases = load_block_bases()
    flags = []

    prog_blocks = {
        60000: ("Progression", 1000),
        61000: ("Map area visits", 1000),
        62000: ("Map fragments", 1000),
        65000: ("Crystal Tears", 1000),
        67000: ("Cookbooks", 1000),
        68000: ("Cookbooks2", 1000),
    }

    for block_start, (label, scan_range) in prog_blocks.items():
        if block_start in bases:
            for i in range(scan_range):
                flag_id = block_start + i
                flags.append((flag_id, f"{label}:{flag_id}"))

    return flags


def collect_dungeon_boss_flags() -> List[Tuple[int, str]]:
    """Collect known dungeon boss defeat flags."""
    dungeon_bases = load_dungeon_bases()
    flags = []

    # Boss defeat flags are at local_id 800, 850 within each area+section
    bosses = {
        (10, 0): [("Godrick the Grafted", 800), ("Margit, the Fell Omen", 850)],
        (11, 0): [("Morgott, the Omen King", 800), ("Godfrey Golden Shade", 850)],
        (12, 1): [("Dragonkin Soldier", 800)],
        (12, 2): [("Ancestor Spirit", 800)],
        (12, 5): [("Mohg, Lord of Blood", 800)],
        (13, 0): [("Maliketh", 800), ("Godskin Duo", 850)],
        (14, 0): [("Rennala", 800), ("Red Wolf of Radagon", 850)],
        (15, 0): [("Malenia", 800), ("Loretta", 850)],
        (16, 0): [("Rykard", 800), ("Godskin Noble", 850)],
        (18, 0): [("Soldier of Godrick", 850)],
        (30, 2): [("Erdtree Burial Watchdog (Stormfoot)", 800)],
        (30, 4): [("Cemetery Shade (Tombsward)", 800)],
        (30, 5): [("Black Knife Assassin (Deathtouched)", 800)],
        (31, 2): [("Miranda the Blighted Bloom", 800)],
        (31, 4): [("Cleanrot Knight", 800)],
    }

    for (area, section), boss_list in bosses.items():
        if area not in dungeon_bases:
            continue
        for boss_name, local_id in boss_list:
            flag_id = area * 1_000_000 + section * 10_000 + local_id
            flags.append((flag_id, boss_name))

    return flags


# ============================================================================
# VERIFICATION ENGINE
# ============================================================================

def verify_slot(
    save_path: str,
    slot_index: int,
    scan_tiles: bool = True,
    check_graces: bool = True,
    check_progression: bool = True,
    check_dungeon_bosses: bool = True,
    verbose: bool = False,
) -> SlotReport:
    """Run full pickup verification for a single slot."""

    report = SlotReport(
        slot_index=slot_index,
        save_path=str(save_path),
        ef_offset=None,
        ef_confident=False,
        tile_base=None,
        tile_base_confidence=0.0,
        tile_base_source="none",
    )

    # Load slot data and detect EF offset
    try:
        slot_data = read_slot_data(save_path, slot_index)
    except Exception as e:
        report.errors.append(f"Failed to read slot {slot_index}: {e}")
        return report

    ef_start = detect_event_flags_start(slot_data)
    if ef_start is None:
        report.errors.append(f"Could not detect EF offset for slot {slot_index}")
        return report

    report.ef_offset = ef_start

    try:
        event_flags = extract_event_flags(slot_data, ef_start)
    except Exception as e:
        report.errors.append(f"Failed to extract event flags: {e}")
        return report

    # Calibrate tile base
    try:
        cal = CalibrationService.calibrate(str(save_path), slot_index)
        report.tile_base = cal.tile_base
        report.tile_base_confidence = cal.tile_base_confidence
        report.tile_base_source = cal.tile_base_source
        report.ef_confident = True  # If calibration succeeds, EF is reliable
    except Exception as e:
        report.tile_base = get_tile_config().get("base_offset", 485330)
        report.tile_base_confidence = 0.0
        report.tile_base_source = "ground_truth_fallback"
        report.errors.append(f"Calibration failed, using ground truth: {e}")

    # 1. Tile pickup scan
    if scan_tiles and report.tile_base:
        tile_flags = collect_tile_flags_from_scan(event_flags, report.tile_base)
        report.tile_flags_checked = len(tile_flags)

        for flag_id, name in tile_flags:
            byte_off = 0
            bit_pos = 0
            try:
                # Use calibrated tile offset
                result = calculate_tile_offset(flag_id)
                if result:
                    # Adjust for calibrated base
                    config = get_tile_config()
                    gt_base = config.get("base_offset", 485330)
                    adjusted_off = result[0] - gt_base + report.tile_base
                    byte_off = adjusted_off
                    bit_pos = result[1]

                    if byte_off < len(event_flags):
                        byte_val = event_flags[byte_off]
                        is_set = (byte_val >> bit_pos) & 1 == 1
                        fp = is_likely_false_positive(event_flags, byte_off, bit_pos)

                        if is_set:
                            report.tile_flags_set += 1
                        if fp:
                            report.tile_false_positives += 1

                        if verbose:
                            report.details.append({
                                "flag_id": flag_id,
                                "name": name,
                                "type": "tile",
                                "offset": byte_off,
                                "bit": bit_pos,
                                "set": is_set,
                                "false_positive": fp,
                            })
            except Exception as e:
                report.errors.append(f"Tile flag {flag_id}: {e}")

    # 2. Grace flags (block-based)
    if check_graces:
        grace_flags = collect_grace_flags()
        for flag_id, name in grace_flags:
            try:
                result = calculate_block_offset(flag_id)
                if result is None:
                    continue

                byte_off, bit_pos = result
                report.block_flags_checked += 1

                if byte_off < len(event_flags):
                    byte_val = event_flags[byte_off]
                    is_set = (byte_val >> bit_pos) & 1 == 1
                    fp = is_likely_false_positive(event_flags, byte_off, bit_pos)

                    if is_set and not fp:
                        report.block_flags_set += 1
                    if fp and is_set:
                        report.block_false_positives += 1

                    if verbose and is_set:
                        report.details.append({
                            "flag_id": flag_id,
                            "name": name,
                            "type": "block",
                            "offset": byte_off,
                            "bit": bit_pos,
                            "set": is_set,
                            "false_positive": fp,
                        })
            except Exception:
                pass

    # 3. Progression flags
    if check_progression:
        prog_flags = collect_progression_flags()
        for flag_id, name in prog_flags:
            try:
                result = calculate_block_offset(flag_id)
                if result is None:
                    continue

                byte_off, bit_pos = result
                report.block_flags_checked += 1

                if byte_off < len(event_flags):
                    byte_val = event_flags[byte_off]
                    is_set = (byte_val >> bit_pos) & 1 == 1
                    fp = is_likely_false_positive(event_flags, byte_off, bit_pos)

                    if is_set and not fp:
                        report.block_flags_set += 1
                    if fp and is_set:
                        report.block_false_positives += 1

                    if verbose and is_set:
                        report.details.append({
                            "flag_id": flag_id,
                            "name": name,
                            "type": "block_progression",
                            "offset": byte_off,
                            "bit": bit_pos,
                            "set": is_set,
                            "false_positive": fp,
                        })
            except Exception:
                pass

    # 4. Dungeon boss flags
    if check_dungeon_bosses:
        boss_flags = collect_dungeon_boss_flags()
        for flag_id, name in boss_flags:
            try:
                result = calculate_dungeon_offset(flag_id)
                if result is None:
                    continue

                byte_off, bit_pos = result
                report.dungeon_flags_checked += 1

                if byte_off < len(event_flags):
                    byte_val = event_flags[byte_off]
                    is_set = (byte_val >> bit_pos) & 1 == 1
                    fp = is_likely_false_positive(event_flags, byte_off, bit_pos)

                    if is_set and not fp:
                        report.dungeon_flags_set += 1
                    if fp and is_set:
                        report.dungeon_false_positives += 1

                    if verbose:
                        report.details.append({
                            "flag_id": flag_id,
                            "name": name,
                            "type": "dungeon",
                            "offset": byte_off,
                            "bit": bit_pos,
                            "set": is_set,
                            "false_positive": fp,
                        })
            except Exception:
                pass

    return report


# ============================================================================
# REPORTING
# ============================================================================

def print_report(report: SlotReport):
    """Print a human-readable verification report."""
    print(f"\n{'='*70}")
    print(f"PICKUP VERIFICATION — Slot {report.slot_index}")
    print(f"{'='*70}")
    print(f"Save: {report.save_path}")
    print(f"EF Offset: {report.ef_offset} (0x{report.ef_offset:X})" if report.ef_offset else "EF Offset: NOT DETECTED")
    print(f"Tile Base: {report.tile_base} (confidence={report.tile_base_confidence:.2f}, source={report.tile_base_source})")
    print()

    total_checked = report.tile_flags_checked + report.block_flags_checked + report.dungeon_flags_checked
    total_set = report.tile_flags_set + report.block_flags_set + report.dungeon_flags_set
    total_fp = report.tile_false_positives + report.block_false_positives + report.dungeon_false_positives

    print(f"{'Category':<25} {'Checked':>8} {'SET':>8} {'FP':>4} {'Rate':>8}")
    print(f"{'-'*25} {'-'*8} {'-'*8} {'-'*4} {'-'*8}")
    print(f"{'Tile pickups':<25} {report.tile_flags_checked:>8} {report.tile_flags_set:>8} {report.tile_false_positives:>4} {_pct(report.tile_flags_set, report.tile_flags_checked):>8}")
    print(f"{'Block (grace/prog)':<25} {report.block_flags_checked:>8} {report.block_flags_set:>8} {report.block_false_positives:>4} {_pct(report.block_flags_set, report.block_flags_checked):>8}")
    print(f"{'Dungeon bosses':<25} {report.dungeon_flags_checked:>8} {report.dungeon_flags_set:>8} {report.dungeon_false_positives:>4} {_pct(report.dungeon_flags_set, report.dungeon_flags_checked):>8}")
    print(f"{'-'*25} {'-'*8} {'-'*8} {'-'*4} {'-'*8}")
    print(f"{'TOTAL':<25} {total_checked:>8} {total_set:>8} {total_fp:>4} {_pct(total_set, total_checked):>8}")

    if report.errors:
        print(f"\nErrors ({len(report.errors)}):")
        for e in report.errors[:10]:
            print(f"  - {e}")
        if len(report.errors) > 10:
            print(f"  ... and {len(report.errors) - 10} more")


def _pct(n: int, total: int) -> str:
    if total == 0:
        return "—"
    return f"{100 * n / total:.1f}%"


# ============================================================================
# MAIN
# ============================================================================

def main():
    parser = argparse.ArgumentParser(description="Pickup flag verification")
    parser.add_argument("--save", type=str, help="Path to save file (.sl2)")
    parser.add_argument("--slot", type=int, default=None, help="Slot index (0-9)")
    parser.add_argument("--all-slots", action="store_true", help="Verify all 10 slots")
    parser.add_argument("--live", action="store_true", help="Use live game save file")
    parser.add_argument("--verbose", "-v", action="store_true", help="Show per-flag details")
    parser.add_argument("--json", type=str, help="Write JSON results to file")
    args = parser.parse_args()

    # Determine save file
    if args.save:
        save_path = args.save
    elif args.live:
        save_path = str(LIVE_SAVE)
    else:
        save_path = str(BACKUP_SAVE)

    if not Path(save_path).exists():
        print(f"Save file not found: {save_path}", file=sys.stderr)
        sys.exit(1)

    # Determine slots
    if args.all_slots:
        slots = list(range(SLOT_COUNT))
    elif args.slot is not None:
        slots = [args.slot]
    else:
        slots = [0]

    all_reports = []
    for slot in slots:
        report = verify_slot(save_path, slot, verbose=args.verbose)
        print_report(report)
        all_reports.append(report)

    # Write JSON if requested
    if args.json:
        json_data = []
        for r in all_reports:
            d = asdict(r)
            json_data.append(d)
        with open(args.json, 'w') as f:
            json.dump(json_data, f, indent=2)
        print(f"\nJSON results written to {args.json}")


if __name__ == "__main__":
    main()
