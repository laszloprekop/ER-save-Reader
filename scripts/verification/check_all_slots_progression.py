#!/usr/bin/env python3
"""
Check progression across all 5 slots to find which has content
that could help discover unknown flag offsets.
"""

import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser
from scripts.verification.ground_truth_loader import (
    load_block_bases,
    calculate_block_offset,
    calculate_dungeon_offset,
)


def check_flag(ef_data: bytes, flag_id: int, formula_type: str = 'block') -> bool:
    """Check if a flag is set."""
    if formula_type == 'block':
        result = calculate_block_offset(flag_id)
    elif formula_type == 'dungeon':
        result = calculate_dungeon_offset(flag_id)
    else:
        return False

    if result is None:
        return False

    byte_offset, bit = result
    if byte_offset >= len(ef_data):
        return False

    return bool((ef_data[byte_offset] >> bit) & 1)


def analyze_slot_progression(ef_data: bytes, slot_name: str):
    """Analyze progression markers for a slot."""
    print(f"\n{'='*60}")
    print(f"Slot: {slot_name}")
    print(f"EF size: {len(ef_data)} bytes")
    print(f"{'='*60}")

    # Boss defeat flags (block: 1-200)
    boss_flags = [
        (171, "Godrick"),
        (172, "Rennala"),
        (173, "Radahn"),
        (174, "Rykard"),
        (175, "Morgott"),
        (176, "Mohg"),
        (177, "Malenia"),
        (178, "Maliketh"),
        (179, "Hoarah Loux"),
        (180, "Elden Beast"),
    ]

    print("\nBoss Defeats:")
    boss_count = 0
    for flag_id, name in boss_flags:
        is_set = check_flag(ef_data, flag_id)
        if is_set:
            boss_count += 1
            print(f"  [{flag_id}] {name}: SET")

    if boss_count == 0:
        print("  (none)")

    # Grace discoveries (block: 71xxx, 76xxx)
    grace_flags = [
        (71800, "Cave of Knowledge"),
        (71801, "Stranded Graveyard"),
        (76100, "The First Step"),
        (76101, "Church of Elleh"),
        (76120, "Gatefront Ruins"),
        (71010, "Godrick the Grafted (grace)"),
        (76200, "Agheel Lake North"),
        (76201, "Agheel Lake South"),
        (76300, "Fort Haight West"),
        (76400, "Third Church of Marika"),
    ]

    print("\nGrace Discoveries (sample):")
    grace_count = 0
    for flag_id, name in grace_flags:
        is_set = check_flag(ef_data, flag_id)
        if is_set:
            grace_count += 1
            print(f"  [{flag_id}] {name}: SET")

    print(f"  Total from sample: {grace_count}/{len(grace_flags)}")

    # Catacomb boss defeats (dungeon: 30xx0800)
    catacomb_flags = [
        (30000800, "Stormfoot Catacombs boss"),
        (30010800, "Murkwater Catacombs boss"),
        (30020800, "Tombsward Catacombs boss"),
        (30030800, "Impaler's Catacombs boss"),
        (30040800, "Deathtouched Catacombs boss"),
        (31010800, "Cliffbottom Catacombs boss"),
        (31020800, "Road's End Catacombs boss"),
        (31030800, "Black Knife Catacombs boss"),
    ]

    print("\nCatacomb Boss Defeats:")
    catacomb_count = 0
    for flag_id, name in catacomb_flags:
        is_set = check_flag(ef_data, flag_id, 'dungeon')
        if is_set:
            catacomb_count += 1
            print(f"  [{flag_id}] {name}: SET")

    if catacomb_count == 0:
        print("  (none)")

    # Key item flags (block: 60xxx)
    key_flags = [
        (60100, "Crafting Kit"),
        (60130, "Whetstone Knife"),
        (60420, "Rold Medallion"),
        (60430, "Haligtree Medallion Right"),
        (60431, "Haligtree Medallion Left"),
    ]

    print("\nKey Items:")
    key_count = 0
    for flag_id, name in key_flags:
        is_set = check_flag(ef_data, flag_id)
        if is_set:
            key_count += 1
            print(f"  [{flag_id}] {name}: SET")

    if key_count == 0:
        print("  (none)")

    # Cookbook flags (block: 67xxx)
    cookbook_flags = [
        (67000, "Armorer's Cookbook [1]"),
        (67010, "Armorer's Cookbook [2]"),
        (67100, "Glintstone Craftsman's [1]"),
        (67200, "Missionary's Cookbook [1]"),
        (67300, "Nomadic Warrior's [1]"),
    ]

    print("\nCookbooks (sample):")
    cookbook_count = 0
    for flag_id, name in cookbook_flags:
        is_set = check_flag(ef_data, flag_id)
        if is_set:
            cookbook_count += 1
            print(f"  [{flag_id}] {name}: SET")

    if cookbook_count == 0:
        print("  (none)")

    return {
        'bosses': boss_count,
        'graces': grace_count,
        'catacombs': catacomb_count,
        'keys': key_count,
        'cookbooks': cookbook_count,
    }


def main():
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    parser = SaveParser()
    parsed = parser.parse(save_path)

    slot_names = [
        "Slot 0 (Confessor, mid-game)",
        "Slot 1 (Wretch, early-game)",
        "Slot 2 (V1, item pickup debug)",
        "Slot 3 (V2, different path)",
        "Slot 4 (V3, true negative)",
    ]

    summaries = []
    for i, slot in enumerate(parsed.slots):
        if slot.event_flags:
            name = slot_names[i] if i < len(slot_names) else f"Slot {i}"
            summary = analyze_slot_progression(slot.event_flags, name)
            summaries.append((name, summary))

    # Summary comparison
    print("\n" + "="*60)
    print("SUMMARY COMPARISON")
    print("="*60)
    print(f"\n{'Slot':<35} {'Bosses':<8} {'Graces':<8} {'Catacombs':<10} {'Keys':<6} {'Cookbooks':<10}")
    print("-" * 85)

    for name, s in summaries:
        short_name = name[:33] if len(name) > 33 else name
        print(f"{short_name:<35} {s['bosses']:<8} {s['graces']:<8} {s['catacombs']:<10} {s['keys']:<6} {s['cookbooks']:<10}")

    # Recommendations
    print("\n" + "="*60)
    print("RECOMMENDATIONS FOR 520xxx DISCOVERY")
    print("="*60)

    has_catacombs = any(s['catacombs'] > 0 for _, s in summaries)
    if has_catacombs:
        for name, s in summaries:
            if s['catacombs'] > 0:
                print(f"\n{name} has {s['catacombs']} catacomb(s) completed!")
                print("  -> Use this slot for 520xxx differential discovery")
    else:
        print("\nNo slots have catacomb completions.")
        print("To discover 520xxx flags:")
        print("  1. Complete a catacomb in any slot")
        print("  2. Capture before/after saves")
        print("  3. Run temporal differential analysis")


if __name__ == "__main__":
    main()
