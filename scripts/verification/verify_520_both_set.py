#!/usr/bin/env python3
"""
Verify the 3 items that showed "BOTH SET" in S0 and S1.

Hypothesis: These items are present in BOTH slot inventories,
which would explain why their flags are set in both slots.
If confirmed, base 1341 is 100% verified for block 520000.
"""

import sys
import struct
from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.verification.save_parser import SaveParser

# Items that showed "BOTH SET" - need to verify if they're in S1 inventory
BOTH_SET_ITEMS = {
    5060: ("Assassin's Cerulean Dagger", 520210),  # Talisman
    4020: ("Flamedrake Talisman", 520330),          # Talisman
    1110: ("Gold Scarab", 520450),                  # Talisman
}

# All 520xxx items for comprehensive check
ALL_520_ITEMS = {
    # Spirit Ashes
    258000: ("Lhutel the Headless", 520000),
    234000: ("Demi-Human Ashes", 520010),
    241000: ("Noble Sorcerer Ashes", 520020),
    5050: ("Assassin's Crimson Dagger", 520030),
    202000: ("Banished Knight Engvall", 520040),
    219000: ("Twinsage Sorcerer Ashes", 520050),
    218000: ("Glintstone Sorcerer Ashes", 520060),
    256000: ("Ancient Dragon Knight Kristoff", 520080),
    239000: ("Bloodhound Knight Floh", 520090),
    3060000: ("Ordovis's Greatsword", 520100),
    217000: ("Perfumer Tricia", 520110),
    246000: ("Soldjars of Fortune Ashes", 520130),
    243000: ("Mad Pumpkin Head Ashes", 520140),
    224000: ("Kindred of Rot Ashes", 520150),
    257000: ("Redmane Knight Ogha", 520160),
    8050000: ("Zamor Curved Sword", 520170),
    228000: ("Blackflame Monk Amon", 520200),
    # Talismans
    5060: ("Assassin's Cerulean Dagger", 520210),
    2160: ("Lord of Blood's Exultation", 520220),
    1020: ("Viridian Amber Medallion", 520300),
    4010: ("Spelldrake Talisman", 520310),
    4020: ("Flamedrake Talisman", 520330),
    2110: ("Blue Dancer Charm", 520350),
    2080: ("Winged Sword Insignia", 520360),
    1010: ("Cerulean Amber Medallion", 520370),
    2170: ("Kindred of Rot's Exultation", 520390),
    44010000: ("Jar Cannon", 520400),
    15020000: ("Great Omenkiller Cleaver", 520410),
    6010: ("Concealing Veil", 520420),
    215000: ("Putrid Corpse Ashes", 520430),
    4022: ("Flamedrake Talisman +2", 520440),
    1110: ("Gold Scarab", 520450),
    3170000: ("Golden Order Greatsword", 520470),
    5040: ("Godskin Swaddling Cloth", 520480),
    13020000: ("Family Heads", 520490),
}


def search_inventory_for_item(slot_raw: bytes, item_id: int) -> bool:
    """Search for an item ID in slot's raw data."""
    patterns = [
        struct.pack('<I', item_id),  # Direct
        struct.pack('<I', 0x40000000 | (item_id & 0x0FFFFFFF)),  # Goods type
        struct.pack('<I', 0x20000000 | (item_id & 0x0FFFFFFF)),  # Accessory type
    ]

    for pattern in patterns:
        if pattern in slot_raw:
            return True
    return False


def main():
    save_path = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-backup-2026-01-11.sl2"

    parser = SaveParser()
    parsed = parser.parse(save_path)

    # Read raw save for inventory search
    with open(save_path, 'rb') as f:
        raw_save = f.read()

    print("=" * 70)
    print("VERIFYING 'BOTH SET' ITEMS ACROSS SLOTS")
    print("=" * 70)
    print("\nHypothesis: If items 520210, 520330, 520450 are in BOTH S0 and S1")
    print("inventory, then base 1341 is 100% correct for block 520000.\n")

    # Check each slot for the "BOTH SET" items
    for slot_idx in range(5):
        slot = parsed.slots[slot_idx]
        if not slot.event_flags:
            continue

        slot_start = slot.slot_offset
        slot_end = slot_start + 2000000
        slot_raw = raw_save[slot_start:min(slot_end, len(raw_save))]

        print(f"\n--- Slot {slot_idx} ---")

        # Check "BOTH SET" items specifically
        print("\n'BOTH SET' items:")
        for item_id, (name, flag_id) in BOTH_SET_ITEMS.items():
            present = search_inventory_for_item(slot_raw, item_id)
            status = "PRESENT" if present else "absent"
            print(f"  {name} (item {item_id}, flag {flag_id}): {status}")

        # Count all 520xxx items present
        present_count = 0
        present_items = []
        for item_id, (name, flag_id) in ALL_520_ITEMS.items():
            if search_inventory_for_item(slot_raw, item_id):
                present_count += 1
                present_items.append((name, flag_id))

        print(f"\nTotal 520xxx items present: {present_count}")
        if present_items and slot_idx <= 1:
            print("  Items:")
            for name, flag_id in sorted(present_items, key=lambda x: x[1]):
                print(f"    {flag_id}: {name}")

    # Now verify base 1341 with the corrected differential
    print("\n" + "=" * 70)
    print("VERIFICATION WITH BASE 1341")
    print("=" * 70)

    base = 1341
    ef_s0 = parsed.slots[0].event_flags
    ef_s1 = parsed.slots[1].event_flags

    # Get S0 and S1 inventory
    s0_raw = raw_save[parsed.slots[0].slot_offset:parsed.slots[0].slot_offset + 2000000]
    s1_raw = raw_save[parsed.slots[1].slot_offset:parsed.slots[1].slot_offset + 2000000]

    s0_items = {item_id for item_id in ALL_520_ITEMS if search_inventory_for_item(s0_raw, item_id)}
    s1_items = {item_id for item_id in ALL_520_ITEMS if search_inventory_for_item(s1_raw, item_id)}

    # True differential: items in S0 but NOT in S1
    true_differential = s0_items - s1_items

    print(f"\nS0 inventory has {len(s0_items)} items with 520xxx flags")
    print(f"S1 inventory has {len(s1_items)} items with 520xxx flags")
    print(f"True differential (S0 - S1): {len(true_differential)} items")

    # Verify each true differential item
    print("\n--- Verifying TRUE differential items ---")
    matches = 0
    for item_id in true_differential:
        name, flag_id = ALL_520_ITEMS[item_id]
        byte_offset = base + (flag_id - 520000) // 8
        bit = 7 - (flag_id % 8)

        if byte_offset < len(ef_s0):
            s0_bit = (ef_s0[byte_offset] >> bit) & 1
            s1_bit = (ef_s1[byte_offset] >> bit) & 1

            if s0_bit == 1 and s1_bit == 0:
                matches += 1
                print(f"  OK: {flag_id} ({name})")
            elif s0_bit == 1 and s1_bit == 1:
                print(f"  BOTH SET: {flag_id} ({name}) - item in both inventories?")
            else:
                print(f"  FAIL: {flag_id} ({name}) - S0={s0_bit}, S1={s1_bit}")

    print(f"\nMatch rate: {matches}/{len(true_differential)}")

    if matches == len(true_differential):
        print("\n" + "=" * 70)
        print("*** VERIFIED: Block 520000 base = 1341 ***")
        print("=" * 70)
        print("\nground_truth_offsets.json entry:")
        print(f'''
  "520000": {{
    "block_start": 520000,
    "base_offset": {base},
    "block_size": 1000,
    "status": "verified",
    "notes": "Spirit Ash/Talisman catacomb rewards. Verified via inventory-driven differential with {matches} items."
  }}''')


if __name__ == "__main__":
    main()
