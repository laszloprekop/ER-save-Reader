#!/usr/bin/env python3
"""
Analyze verification-records.jsonl to find leads for formula verification.

Strategy:
1. Identify mismatches (manualStatus != autoStatus)
2. Group by category, region, and formula type
3. Look for patterns that suggest formula errors vs user errors
4. Apply inseparable evidence methodology where possible
"""

import json
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Any

RECORDS_PATH = "/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/verification-records.jsonl"


def load_records() -> List[Dict]:
    records = []
    with open(RECORDS_PATH, 'r') as f:
        for line in f:
            if line.strip():
                records.append(json.loads(line))
    return records


def analyze_records(records: List[Dict]):
    # Group by various dimensions
    by_slot = defaultdict(list)
    by_category = defaultdict(list)
    by_region = defaultdict(list)
    by_flag_type = defaultdict(list)

    mismatches = []
    agreements = []

    for r in records:
        by_slot[r['slotIndex']].append(r)
        by_category[r['flagCategory']].append(r)
        by_region[r.get('flagRegion', 'Unknown')].append(r)
        by_flag_type[r['flagType']].append(r)

        if r['matches']:
            agreements.append(r)
        else:
            mismatches.append(r)

    return {
        'by_slot': dict(by_slot),
        'by_category': dict(by_category),
        'by_region': dict(by_region),
        'by_flag_type': dict(by_flag_type),
        'mismatches': mismatches,
        'agreements': agreements,
    }


def print_summary(analysis: Dict):
    print("="*80)
    print("VERIFICATION RECORDS ANALYSIS")
    print("="*80)

    total = len(analysis['mismatches']) + len(analysis['agreements'])
    print(f"\nTotal records: {total}")
    print(f"Agreements (matches=true): {len(analysis['agreements'])}")
    print(f"Mismatches (matches=false): {len(analysis['mismatches'])}")

    # By slot
    print("\n" + "-"*60)
    print("BY CHARACTER SLOT:")
    print("-"*60)
    for slot, records in sorted(analysis['by_slot'].items()):
        char_name = records[0]['characterName'] if records else 'Unknown'
        matches = sum(1 for r in records if r['matches'])
        mismatches = len(records) - matches
        print(f"  Slot {slot} ({char_name}): {len(records)} records, {matches} match, {mismatches} mismatch")

    # By category
    print("\n" + "-"*60)
    print("BY CATEGORY:")
    print("-"*60)
    for cat, records in sorted(analysis['by_category'].items(), key=lambda x: -len(x[1])):
        matches = sum(1 for r in records if r['matches'])
        mismatches = len(records) - matches
        print(f"  {cat:30} {len(records):4} records, {matches:4} match, {mismatches:4} mismatch")

    # By flag type
    print("\n" + "-"*60)
    print("BY FLAG TYPE:")
    print("-"*60)
    for ftype, records in sorted(analysis['by_flag_type'].items()):
        matches = sum(1 for r in records if r['matches'])
        mismatches = len(records) - matches
        print(f"  {ftype:10} {len(records):4} records, {matches:4} match, {mismatches:4} mismatch")


def find_mismatch_patterns(analysis: Dict):
    """Look for patterns in mismatches that might indicate formula errors."""

    print("\n" + "="*80)
    print("MISMATCH ANALYSIS")
    print("="*80)

    mismatches = analysis['mismatches']

    # Categorize mismatches
    user_set_formula_not = [r for r in mismatches if r['manualStatus'] and not r['autoStatus']]
    user_not_formula_set = [r for r in mismatches if not r['manualStatus'] and r['autoStatus']]

    print(f"\nUser SET, Formula NOT SET: {len(user_set_formula_not)}")
    print(f"User NOT SET, Formula SET: {len(user_not_formula_set)}")

    # User SET, Formula NOT SET - could be formula error
    print("\n" + "-"*60)
    print("USER SET, FORMULA NOT SET (possible formula errors):")
    print("-"*60)

    # Group by flag type
    by_type = defaultdict(list)
    for r in user_set_formula_not:
        by_type[r['flagType']].append(r)

    for ftype, records in sorted(by_type.items()):
        print(f"\n  {ftype.upper()} FLAGS ({len(records)}):")

        # Further group by region or block
        if ftype == 'block':
            by_block = defaultdict(list)
            for r in records:
                block = (r['flagId'] // 1000) * 1000
                by_block[block].append(r)

            for block, block_records in sorted(by_block.items()):
                print(f"    Block {block}: {len(block_records)} flags")
                for r in block_records[:5]:  # Show first 5
                    print(f"      {r['flagId']} {r['flagName'][:40]:40} (slot {r['slotIndex']})")
                if len(block_records) > 5:
                    print(f"      ... and {len(block_records) - 5} more")

        elif ftype == 'dungeon':
            by_area = defaultdict(list)
            for r in records:
                area = r['flagId'] // 1_000_000
                by_area[area].append(r)

            for area, area_records in sorted(by_area.items()):
                print(f"    Area {area}: {len(area_records)} flags")
                for r in area_records[:5]:
                    print(f"      {r['flagId']} {r['flagName'][:40]:40} (slot {r['slotIndex']})")
                if len(area_records) > 5:
                    print(f"      ... and {len(area_records) - 5} more")

        elif ftype == 'tile':
            # Group by tile
            by_tile = defaultdict(list)
            for r in records:
                tile = (r['flagId'] // 10000) % 10000  # Extract tile index
                by_tile[tile].append(r)

            print(f"    Tiles with mismatches: {len(by_tile)}")
            for tile, tile_records in sorted(by_tile.items())[:10]:
                print(f"      Tile {tile}: {len(tile_records)} flags")

    # User NOT SET, Formula SET - likely false positives in formula
    if user_not_formula_set:
        print("\n" + "-"*60)
        print("USER NOT SET, FORMULA SET (likely false positives):")
        print("-"*60)

        for r in user_not_formula_set[:20]:
            print(f"  {r['flagId']:12} {r['flagName'][:40]:40} [{r['flagType']}] (slot {r['slotIndex']})")


def find_corroboration_opportunities(analysis: Dict):
    """Find opportunities to verify flags using corroborating evidence."""

    print("\n" + "="*80)
    print("CORROBORATION OPPORTUNITIES")
    print("="*80)

    # Look for grace clusters - if user has grace A, check nearby graces
    grace_records = [r for r in analysis['by_category'].get('Grace', []) if r['manualStatus']]

    print(f"\nGraces marked as discovered: {len(grace_records)}")

    # Group by slot and region
    by_slot_region = defaultdict(list)
    for r in grace_records:
        key = (r['slotIndex'], r.get('flagRegion', 'Unknown'))
        by_slot_region[key].append(r)

    print("\nGrace clusters by slot and region:")
    for (slot, region), records in sorted(by_slot_region.items()):
        char = records[0]['characterName']
        matches = sum(1 for r in records if r['matches'])
        mismatches = len(records) - matches
        if mismatches > 0:
            print(f"  Slot {slot} ({char}) - {region}: {len(records)} graces, {mismatches} MISMATCHES")
            for r in records:
                if not r['matches']:
                    print(f"    ! {r['flagId']} {r['flagName'][:40]} (formula says NOT SET)")


def identify_high_confidence_leads(analysis: Dict):
    """Identify high-confidence verification leads."""

    print("\n" + "="*80)
    print("HIGH-CONFIDENCE VERIFICATION LEADS")
    print("="*80)

    mismatches = analysis['mismatches']

    # Find blocks/areas with MANY mismatches - suggests formula error
    user_set_not_auto = [r for r in mismatches if r['manualStatus'] and not r['autoStatus']]

    # Count by block for block-type flags
    block_counts = defaultdict(int)
    block_examples = defaultdict(list)
    for r in user_set_not_auto:
        if r['flagType'] == 'block':
            block = (r['flagId'] // 1000) * 1000
            block_counts[block] += 1
            block_examples[block].append(r)

    print("\nBlocks with multiple mismatches (user SET, formula NOT SET):")
    for block, count in sorted(block_counts.items(), key=lambda x: -x[1]):
        if count >= 2:
            print(f"\n  Block {block}: {count} mismatches")
            for r in block_examples[block][:5]:
                print(f"    {r['flagId']} {r['flagName'][:40]} (slot {r['slotIndex']} - {r['characterName']})")

    # Count by area for dungeon-type flags
    area_counts = defaultdict(int)
    area_examples = defaultdict(list)
    for r in user_set_not_auto:
        if r['flagType'] == 'dungeon':
            area = r['flagId'] // 1_000_000
            area_counts[area] += 1
            area_examples[area].append(r)

    print("\n\nDungeon areas with multiple mismatches (user SET, formula NOT SET):")
    for area, count in sorted(area_counts.items(), key=lambda x: -x[1]):
        if count >= 2:
            print(f"\n  Area {area}: {count} mismatches")
            for r in area_examples[area][:5]:
                print(f"    {r['flagId']} {r['flagName'][:40]} (slot {r['slotIndex']} - {r['characterName']})")


def main():
    print("Loading verification records...")
    records = load_records()
    print(f"Loaded {len(records)} records")

    analysis = analyze_records(records)

    print_summary(analysis)
    find_mismatch_patterns(analysis)
    find_corroboration_opportunities(analysis)
    identify_high_confidence_leads(analysis)


if __name__ == "__main__":
    main()
