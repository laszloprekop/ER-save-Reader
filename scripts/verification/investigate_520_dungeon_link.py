#!/usr/bin/env python3
"""
Investigate the relationship between 520xxx flags and dungeon completion flags.

Hypothesis: 520xxx flags (catacomb rewards) may be stored with or near
their associated dungeon flags (30xxx format: AASSZZZZ).

Approach:
1. Map known 520xxx items to their dungeon locations
2. Check dungeon completion flags for those dungeons
3. Look for patterns in the dungeon flag region
4. Search for 520xxx flags near dungeon allocations
"""

import sys
from pathlib import Path
from dataclasses import dataclass
from typing import List, Optional, Tuple

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser
from scripts.verification.ground_truth_loader import load_dungeon_bases, calculate_dungeon_offset


# ============================================================================
# CATACOMB REWARD MAPPINGS
# ============================================================================

@dataclass
class CatacombReward:
    """A catacomb and its reward."""
    name: str
    map_area: int  # First 2 digits of dungeon flag
    section: int   # Digits 3-4 of dungeon flag
    reward_flag: int  # 520xxx flag
    reward_item: str
    reward_item_id: int
    boss_name: str
    boss_flag_local: int  # Local ID within dungeon (usually 800 for boss defeat)


# Known catacomb rewards mapped to their dungeons
# Format: AASSZZZZ where AA=map_area, SS=section, ZZZZ=local_id
CATACOMB_REWARDS = [
    # Limgrave Catacombs
    CatacombReward(
        name="Stormfoot Catacombs",
        map_area=30,
        section=0,
        reward_flag=520000,
        reward_item="Lhutel the Headless",
        reward_item_id=258000,
        boss_name="Erdtree Burial Watchdog",
        boss_flag_local=800,
    ),
    CatacombReward(
        name="Murkwater Catacombs",
        map_area=30,
        section=1,
        reward_flag=520010,
        reward_item="Demi-Human Ashes",
        reward_item_id=234000,
        boss_name="Grave Warden Duelist",
        boss_flag_local=800,
    ),
    CatacombReward(
        name="Tombsward Catacombs",
        map_area=30,
        section=2,
        reward_flag=520030,  # Assassin's Crimson Dagger
        reward_item="Assassin's Crimson Dagger",
        reward_item_id=5050,
        boss_name="Cemetery Shade",
        boss_flag_local=800,
    ),
    CatacombReward(
        name="Impaler's Catacombs",
        map_area=30,
        section=3,
        reward_flag=520040,
        reward_item="Banished Knight Engvall",
        reward_item_id=202000,
        boss_name="Erdtree Burial Watchdog",
        boss_flag_local=800,
    ),
    CatacombReward(
        name="Deathtouched Catacombs",
        map_area=30,
        section=4,
        reward_flag=520020,  # Noble Sorcerer Ashes - check this
        reward_item="Noble Sorcerer Ashes",
        reward_item_id=241000,
        boss_name="Black Knife Assassin",
        boss_flag_local=800,
    ),
    # Liurnia Catacombs
    CatacombReward(
        name="Cliffbottom Catacombs",
        map_area=31,
        section=1,
        reward_flag=520050,
        reward_item="Twinsage Sorcerer Ashes",
        reward_item_id=219000,
        boss_name="Erdtree Burial Watchdog",
        boss_flag_local=800,
    ),
    CatacombReward(
        name="Road's End Catacombs",
        map_area=31,
        section=2,
        reward_flag=520060,
        reward_item="Glintstone Sorcerer Ashes",
        reward_item_id=218000,
        boss_name="Spirit-caller Snail",
        boss_flag_local=800,
    ),
    CatacombReward(
        name="Black Knife Catacombs",
        map_area=31,
        section=3,
        reward_flag=520210,  # Assassin's Cerulean Dagger
        reward_item="Assassin's Cerulean Dagger",
        reward_item_id=5060,
        boss_name="Cemetery Shade",
        boss_flag_local=800,
    ),
    # Altus Catacombs
    CatacombReward(
        name="Sainted Hero's Grave",
        map_area=30,
        section=10,
        reward_flag=520080,
        reward_item="Ancient Dragon Knight Kristoff",
        reward_item_id=256000,
        boss_name="Ancient Hero of Zamor",
        boss_flag_local=800,
    ),
    CatacombReward(
        name="Gelmir Hero's Grave",
        map_area=30,
        section=16,
        reward_flag=520090,
        reward_item="Bloodhound Knight Floh",
        reward_item_id=239000,
        boss_name="Red Wolf of the Champion",
        boss_flag_local=800,
    ),
    # Mountaintops
    CatacombReward(
        name="Giant-Conquering Hero's Grave",
        map_area=30,
        section=19,
        reward_flag=520160,
        reward_item="Redmane Knight Ogha",
        reward_item_id=257000,
        boss_name="Ancient Hero of Zamor",
        boss_flag_local=800,
    ),
]


def get_dungeon_boss_flag(map_area: int, section: int, local_id: int = 800) -> int:
    """Construct dungeon boss defeat flag."""
    return int(f"{map_area:02d}{section:02d}{local_id:04d}")


def check_dungeon_flags(ef_data: bytes, ef_data_early: bytes, catacomb: CatacombReward):
    """Check dungeon-related flags for a catacomb."""
    print(f"\n{'='*60}")
    print(f"Catacomb: {catacomb.name}")
    print(f"Reward: {catacomb.reward_item} (flag {catacomb.reward_flag})")
    print(f"{'='*60}")

    # Construct dungeon flags
    boss_flag = get_dungeon_boss_flag(catacomb.map_area, catacomb.section, 800)

    print(f"\nDungeon: map_area={catacomb.map_area}, section={catacomb.section}")
    print(f"Boss defeat flag: {boss_flag}")

    # Calculate dungeon flag offset
    result = calculate_dungeon_offset(boss_flag)
    if result:
        byte_offset, bit = result
        print(f"Dungeon flag offset: byte={byte_offset}, bit={bit}")

        # Check if set in both slots
        if byte_offset < len(ef_data):
            s0_set = (ef_data[byte_offset] >> bit) & 1
            s1_set = (ef_data_early[byte_offset] >> bit) & 1
            print(f"  S0 (progressed): {'SET' if s0_set else 'unset'}")
            print(f"  S1 (early): {'SET' if s1_set else 'unset'}")

            if s0_set and not s1_set:
                print(f"  => Valid differential! Dungeon completed in S0.")
            elif s0_set and s1_set:
                print(f"  => Both set - dungeon completed in both slots")
            elif not s0_set:
                print(f"  => NOT completed in S0")
    else:
        print(f"  No formula for dungeon {catacomb.map_area}")

    # Now search for the 520xxx flag near the dungeon region
    print(f"\nSearching for reward flag {catacomb.reward_flag}...")

    # The reward flag should be SET if boss is defeated
    # Search around the dungeon region
    if result:
        dungeon_offset = result[0]
        search_start = max(0, dungeon_offset - 50)
        search_end = min(len(ef_data), dungeon_offset + 200)

        # Calculate expected bit for the 520xxx flag
        reward_bit = 7 - (catacomb.reward_flag % 8)

        candidates = []
        for offset in range(search_start, search_end):
            s0_byte = ef_data[offset]
            s1_byte = ef_data_early[offset]

            s0_bit_set = (s0_byte >> reward_bit) & 1
            s1_bit_set = (s1_byte >> reward_bit) & 1

            # Valid differential for reward flag
            if s0_bit_set and not s1_bit_set and s0_byte != 0xFF:
                candidates.append((offset, s0_byte))

        if candidates:
            print(f"  Found {len(candidates)} candidate locations:")
            for offset, byte_val in candidates[:5]:
                # Calculate implied base for 520000 block
                relative = catacomb.reward_flag - 520000
                implied_base = offset - (relative // 8)
                print(f"    offset={offset}, byte=0x{byte_val:02X}, implied_base={implied_base}")
        else:
            print(f"  No candidates found near dungeon region")


def analyze_dungeon_region(ef_data: bytes, map_area: int):
    """Analyze the dungeon region for a map area."""
    bases = load_dungeon_bases()

    if map_area not in bases:
        print(f"No base defined for map area {map_area}")
        return

    base_info = bases[map_area]
    base_offset = base_info['base_offset']

    print(f"\n{'='*60}")
    print(f"Dungeon Region: Map Area {map_area}")
    print(f"Base offset: {base_offset}")
    print(f"{'='*60}")

    # Each section has 1125 bytes (9000 flags / 8 bits)
    section_size = 1125

    # Show first few sections
    for section in range(5):
        section_start = base_offset + section * section_size
        section_end = section_start + 20  # Just first 20 bytes

        if section_start < len(ef_data):
            region = ef_data[section_start:section_end]
            non_ff = sum(1 for b in region if b != 0xFF)
            hex_str = ' '.join(f'{b:02X}' for b in region[:8])
            print(f"  Section {section:02d} (offset {section_start}): {hex_str}... ({non_ff}/20 non-FF)")


def main():
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    parser = SaveParser()
    parsed = parser.parse(save_path)

    ef_s0 = parsed.slots[0].event_flags
    ef_s1 = parsed.slots[1].event_flags

    print(f"Loaded save: {save_path}")
    print(f"S0 EF size: {len(ef_s0)} bytes")
    print(f"S1 EF size: {len(ef_s1)} bytes")

    # Analyze dungeon regions
    for map_area in [30, 31, 32, 34, 35]:
        analyze_dungeon_region(ef_s0, map_area)

    # Check each catacomb
    print("\n" + "="*60)
    print("CATACOMB REWARD FLAG INVESTIGATION")
    print("="*60)

    for catacomb in CATACOMB_REWARDS[:5]:  # First 5 for now
        check_dungeon_flags(ef_s0, ef_s1, catacomb)

    # Summary: Look for patterns
    print("\n" + "="*60)
    print("PATTERN ANALYSIS")
    print("="*60)

    print("\nChecking if reward flags are stored with offset from boss flag...")

    for catacomb in CATACOMB_REWARDS[:5]:
        boss_flag = get_dungeon_boss_flag(catacomb.map_area, catacomb.section, 800)
        boss_result = calculate_dungeon_offset(boss_flag)

        if boss_result:
            boss_offset, boss_bit = boss_result

            # Try different offsets from boss flag
            for delta in range(-10, 20):
                test_offset = boss_offset + delta
                if test_offset < 0 or test_offset >= len(ef_s0):
                    continue

                reward_bit = 7 - (catacomb.reward_flag % 8)
                s0_byte = ef_s0[test_offset]
                s1_byte = ef_s1[test_offset]

                s0_set = (s0_byte >> reward_bit) & 1
                s1_set = (s1_byte >> reward_bit) & 1

                if s0_set and not s1_set and s0_byte != 0xFF:
                    print(f"  {catacomb.name}: reward flag might be at boss_offset{delta:+d} = {test_offset}")
                    print(f"    Boss flag {boss_flag} at offset {boss_offset}")
                    print(f"    Reward flag {catacomb.reward_flag} candidate at offset {test_offset}")
                    break


if __name__ == "__main__":
    main()
