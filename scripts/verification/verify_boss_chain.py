#!/usr/bin/env python3
"""
Verify boss defeat -> remembrance pickup flag chains.

This script validates that event flags form consistent chains:
1. Dungeon defeat flags (e.g., 16000800 for Rykard) indicate boss killed
2. Pickup flags (510xxx) indicate remembrance was collected
3. Valid states:
   - Both unset: Boss not defeated
   - Dungeon set, pickup unset: Boss killed, remembrance not yet collected
   - Both set: Boss killed and remembrance collected
   - Dungeon unset, pickup set: CONTRADICTION (impossible without cheating)

Key discoveries (2026-01-21):
- Event 1100 (91xx flags) awards progression items (Talisman Pouch), NOT remembrances
- Remembrances are world drops with 510xxx pickup flags
- The 91xx flags set on boss death (e.g., 9122 for Rykard) trigger progression rewards
- Remembrance items: 2950-2964 in EquipParamGoods.param.xml

Chain data source: ItemLotParam_map.param.xml getItemFlagId field

NOTE: Full inventory verification would require parsing ga_items -> equip_inventory_data
relationship via gaitem_handle lookups. Currently only checks event flags.
"""

import struct
import sys
from pathlib import Path
from typing import Optional, Tuple, List, Dict, NamedTuple

# Add project root to path for imports
PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.ground_truth_loader import (
    load_block_bases,
    get_block_base,
    get_validation_flags,
)

# ============================================================================
# CONSTANTS
# ============================================================================

SLOT_SIZE = 0x280020  # Size of each character slot
HEADER_SIZE = 0x310   # Save file header size

# Validation flags loaded from ground_truth
_GT_VALIDATION_FLAGS = get_validation_flags()
VALIDATION_FLAGS = [
    (flag_id, offset, bit, name)
    for flag_id, (offset, bit, name) in _GT_VALIDATION_FLAGS.items()
]

# ============================================================================
# BOSS CHAIN DATA
# From common.emevd.js Event 1100 and Event 9300
# ============================================================================

class BossChain(NamedTuple):
    name: str
    remembrance_item_id: int   # 2950-2964 - item ID in EquipParamGoods
    dungeon_defeat_flag: int   # 8-digit dungeon flag (0 if field boss)
    pickup_flag: int           # 510xxx - remembrance pickup flag
    item_lot: int              # ItemLot ID that drops remembrance
    dungeon_base: int          # Base offset for dungeon formula (0 if unknown)

# Corrected boss chain data from ItemLotParam_map.param.xml
# Chain: Boss defeat (dungeon flag) -> Remembrance drops -> Pickup flag set
BOSS_CHAINS = [
    # Main game bosses with dungeon defeat flags
    # Format: name, remembrance_item_id, dungeon_flag, pickup_flag, item_lot, dungeon_base
    BossChain("Godrick the Grafted", 2950, 10000800, 510010, 10011, 4112),
    BossChain("Rennala, Queen of the Full Moon", 2959, 14000800, 197, 10180, 29987),  # pickup 197 is special
    BossChain("Morgott, the Omen King", 2952, 11000800, 510040, 10040, 8612),
    BossChain("Rykard, Lord of Blasphemy", 2953, 16000800, 510220, 10220, 36737),
    BossChain("Mohg, Lord of Blood", 2955, 12050800, 510120, 10120, 15362),
    BossChain("Malenia, Blade of Miquella", 2954, 15000800, 510200, 10200, 33362),
    BossChain("Maliketh, the Black Blade", 2956, 13000800, 510160, 10160, 26612),
    BossChain("Hoarah Loux, Warrior", 2957, 11000850, 510070, 10070, 8612),
    BossChain("Radagon / Elden Beast", 2963, 19000800, 510230, 10230, 46862),

    # Field bosses (no dungeon defeat flag, but may have area flags)
    BossChain("Starscourge Radahn", 2951, 0, 510300, 10300, 0),  # Caelid field boss
    BossChain("Fire Giant", 2961, 0, 510310, 10310, 0),  # Mountaintops field boss
    BossChain("Lichdragon Fortissax", 2960, 0, 510110, 10110, 0),  # Deeproot Depths
    BossChain("Dragonlord Placidusax", 2958, 0, 510150, 10150, 0),  # Crumbling Farum Azula
    BossChain("Astel, Naturalborn of the Void", 2964, 0, 510080, 10080, 0),  # Lake of Rot
    BossChain("Regal Ancestor Spirit", 2962, 0, 510330, 10330, 0),  # Siofra River
]


def detect_event_flags_offset(slot_data: bytes, search_start: int = 0x12000) -> Optional[int]:
    """Detect the event_flags section offset within slot data."""
    for test_offset in range(search_start, min(0x15000, len(slot_data) - 10000)):
        score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if (byte_val & (1 << bit_pos)) != 0:
                    score += 1
        if score == len(VALIDATION_FLAGS):
            return test_offset
    return None


def check_block_flag(event_flags: bytes, flag_id: int) -> Optional[bool]:
    """Check a block flag (5-6 digit flags).

    Uses ground_truth_loader for base offsets instead of hardcoded values.
    """
    # For 510xxx flags - use ground_truth_loader
    if 510000 <= flag_id <= 510999:
        base = get_block_base(510000)
        if base is None:
            # Fallback for 510000 range if not in ground_truth
            base = 63750  # Legacy estimate
        relative = flag_id - 510000
        byte_offset = base + relative // 8
        bit_pos = 7 - (flag_id % 8)

        if byte_offset >= len(event_flags):
            return None

        byte_val = event_flags[byte_offset]
        return (byte_val >> bit_pos) & 1 == 1

    # For small flags (< 1000) like flag 197 for Rennala
    if flag_id < 1000:
        # These are likely in the early part of event_flags
        # Flags 0-999 typically at the very start
        byte_offset = flag_id // 8
        bit_pos = 7 - (flag_id % 8)

        if byte_offset >= len(event_flags):
            return None

        byte_val = event_flags[byte_offset]
        return (byte_val >> bit_pos) & 1 == 1

    # For standard 5-6 digit block flags, use ground_truth_loader
    base = get_block_base(flag_id)
    if base is not None:
        block_start = (flag_id // 1000) * 1000
        relative = flag_id - block_start
        byte_offset = base + relative // 8
        bit_pos = 7 - (flag_id % 8)

        if byte_offset >= len(event_flags):
            return None

        byte_val = event_flags[byte_offset]
        return (byte_val >> bit_pos) & 1 == 1

    return None


def check_dungeon_flag(event_flags: bytes, flag_id: int, dungeon_base: int) -> Optional[bool]:
    """Check an 8-digit dungeon defeat flag."""
    if flag_id == 0 or dungeon_base == 0:
        return None

    # Extract components: AASSLLLL
    area = flag_id // 1_000_000
    section = (flag_id // 10_000) % 100
    local_id = flag_id % 10_000

    section_size = 1125
    byte_offset = dungeon_base + section * section_size + local_id // 8
    bit_pos = 7 - (local_id % 8)

    if byte_offset >= len(event_flags):
        return None

    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


def extract_slot_data(save_path: str, slot_index: int) -> bytes:
    """Extract slot data from save file."""
    with open(save_path, 'rb') as f:
        f.seek(HEADER_SIZE + slot_index * SLOT_SIZE)
        return f.read(SLOT_SIZE)


def verify_boss_chains(event_flags: bytes) -> List[Dict]:
    """Verify all boss defeat -> remembrance pickup chains.

    Chain verification logic:
    1. Check if remembrance pickup flag (510xxx) is set
    2. If pickup flag is set, check if dungeon defeat flag matches
    3. If dungeon defeat flag is set but pickup not set -> valid (player hasn't picked up yet)
    """
    results = []

    for chain in BOSS_CHAINS:
        result = {
            "boss": chain.name,
            "remembrance_item_id": chain.remembrance_item_id,
            "dungeon_flag": chain.dungeon_defeat_flag,
            "pickup_flag": chain.pickup_flag,
            "pickup_set": None,
            "dungeon_defeated": None,
            "status": "unknown",
            "contradiction": None,
        }

        # Check remembrance pickup flag (510xxx or special flag)
        result["pickup_set"] = check_block_flag(event_flags, chain.pickup_flag)

        # Check dungeon defeat flag (if applicable)
        if chain.dungeon_defeat_flag != 0:
            result["dungeon_defeated"] = check_dungeon_flag(
                event_flags, chain.dungeon_defeat_flag, chain.dungeon_base
            )

        # Determine status and check for contradictions
        if result["pickup_set"] is True:
            # Remembrance was picked up - boss should be defeated
            if result["dungeon_defeated"] is True:
                result["status"] = "consistent"
            elif result["dungeon_defeated"] is False:
                result["status"] = "contradiction"
                result["contradiction"] = f"Remembrance picked up but {chain.name} not defeated"
            elif chain.dungeon_defeat_flag == 0:
                # Field boss - can't verify via dungeon flag
                result["status"] = "picked_up_field_boss"
            else:
                result["status"] = "partial"
        elif result["pickup_set"] is False:
            # Remembrance not picked up
            if result["dungeon_defeated"] is True:
                result["status"] = "defeat_no_pickup"
                # This is valid - player killed boss but hasn't picked up remembrance yet
            elif result["dungeon_defeated"] is False:
                result["status"] = "not_defeated"
            elif chain.dungeon_defeat_flag == 0:
                result["status"] = "field_boss_not_checked"
            else:
                result["status"] = "unknown_defeat"
        else:
            # Pickup flag couldn't be read
            result["status"] = "unknown_pickup"

        results.append(result)

    return results


def main():
    # Default save file path
    save_path = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"
    slot_index = 0

    if len(sys.argv) > 1:
        save_path = sys.argv[1]
    if len(sys.argv) > 2:
        slot_index = int(sys.argv[2])

    print(f"Loading save file: {save_path}")
    print(f"Slot index: {slot_index}")
    print()

    # Extract slot data
    slot_data = extract_slot_data(save_path, slot_index)
    print(f"Slot data size: {len(slot_data):,} bytes")

    # Detect event_flags offset
    event_flags_offset = detect_event_flags_offset(slot_data)
    if event_flags_offset is None:
        print("ERROR: Could not detect event_flags offset!")
        return

    print(f"Detected event_flags offset: 0x{event_flags_offset:X} ({event_flags_offset})")
    print()

    # Extract event_flags section
    event_flags = slot_data[event_flags_offset:]

    # Verify boss chains
    results = verify_boss_chains(event_flags)

    # Print results
    print("=" * 80)
    print("BOSS DEFEAT -> REMEMBRANCE PICKUP CHAIN VERIFICATION")
    print("=" * 80)

    # Group by status
    consistent = [r for r in results if r["status"] == "consistent"]
    contradictions = [r for r in results if r["status"] == "contradiction"]
    defeat_no_pickup = [r for r in results if r["status"] == "defeat_no_pickup"]
    picked_up_field = [r for r in results if r["status"] == "picked_up_field_boss"]
    not_defeated = [r for r in results if r["status"] == "not_defeated"]
    field_not_checked = [r for r in results if r["status"] == "field_boss_not_checked"]
    unknown = [r for r in results if r["status"] in ["unknown", "unknown_pickup", "unknown_defeat", "partial"]]

    if contradictions:
        print("\n⚠️  CONTRADICTIONS DETECTED:")
        print("-" * 60)
        for r in contradictions:
            print(f"  {r['boss']}:")
            print(f"    Pickup flag {r['pickup_flag']}: {r['pickup_set']}")
            print(f"    Dungeon flag {r['dungeon_flag']}: {r['dungeon_defeated']}")
            print(f"    Issue: {r['contradiction']}")

    if consistent:
        print(f"\n✓ CONSISTENT ({len(consistent)} bosses):")
        print("-" * 60)
        for r in consistent:
            print(f"  {r['boss']}: pickup flag SET, dungeon flag SET ✓")

    if defeat_no_pickup:
        print(f"\n⏳ DEFEATED BUT NOT PICKED UP ({len(defeat_no_pickup)} bosses):")
        print("-" * 60)
        for r in defeat_no_pickup:
            print(f"  {r['boss']}: dungeon flag SET, pickup flag NOT SET")
            print(f"    (Player killed boss but hasn't picked up remembrance)")

    if picked_up_field:
        print(f"\n✓ FIELD BOSS PICKUPS ({len(picked_up_field)} bosses):")
        print("-" * 60)
        for r in picked_up_field:
            print(f"  {r['boss']}: pickup flag SET (field boss, no dungeon flag to verify)")

    if not_defeated:
        print(f"\n○ NOT DEFEATED ({len(not_defeated)} bosses):")
        print("-" * 60)
        for r in not_defeated:
            print(f"  {r['boss']}")

    if field_not_checked:
        print(f"\n○ FIELD BOSSES NOT CHECKED ({len(field_not_checked)} bosses):")
        print("-" * 60)
        for r in field_not_checked:
            print(f"  {r['boss']} (no dungeon flag to verify)")

    if unknown:
        print(f"\n? UNKNOWN ({len(unknown)} bosses):")
        print("-" * 60)
        for r in unknown:
            print(f"  {r['boss']} ({r['status']})")
            print(f"    Pickup: {r['pickup_set']}, Dungeon: {r['dungeon_defeated']}")

    # Summary
    print()
    print("=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print(f"  Consistent (pickup + defeat): {len(consistent)}")
    print(f"  Contradictions: {len(contradictions)}")
    print(f"  Defeated, not picked up: {len(defeat_no_pickup)}")
    print(f"  Field boss pickups: {len(picked_up_field)}")
    print(f"  Not defeated: {len(not_defeated)}")
    print(f"  Field bosses not checked: {len(field_not_checked)}")
    print(f"  Unknown: {len(unknown)}")

    if not contradictions:
        print("\n✓ No contradictions found - flag chains are consistent!")


if __name__ == "__main__":
    main()
