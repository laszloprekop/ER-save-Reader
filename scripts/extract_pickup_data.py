#!/usr/bin/env python3
"""
Extract world pickup data from ItemLotParam_map.param.xml and generate pickup_data.rs

Fixes applied:
1. Proper region derivation for different lot ID formats (8-digit vs 10-digit)
2. Correct category mapping based on item type databases
3. Filter out boss drops (Great Runes) and non-world pickups
4. Proper item name resolution using all equip param files
"""

import xml.etree.ElementTree as ET
from pathlib import Path
from collections import defaultdict

# Path to game files
GAME_FILES = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/regulation-bin")
OUTPUT_FILE = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/src/db/pickup_data.rs")

# Item ID ranges for categorization
GOLDEN_RUNE_IDS = set(range(2900, 2920))  # Golden Rune [1] through [13], Hero's Runes, Lord's Rune
SMITHING_STONE_IDS = set(range(10100, 10141))  # Smithing Stone [1-8], Ancient Dragon
SOMBER_STONE_IDS = set(range(10160, 10201))  # Somber [1-9], Somber Ancient Dragon
GLOVEWORT_IDS = set(range(10900, 10920))  # Grave/Ghost Glovewort
GREAT_RUNE_IDS = set(range(190, 198))  # Great Runes (these are KEY items, not world pickups)

def load_item_database(filename: str) -> dict[int, str]:
    """Load item ID -> name mapping from a param XML file."""
    items = {}
    filepath = GAME_FILES / filename
    if not filepath.exists():
        print(f"Warning: {filepath} not found")
        return items

    tree = ET.parse(filepath)
    root = tree.getroot()

    for row in root.findall('.//row'):
        item_id = int(row.get('id', 0))
        name = row.get('paramdexName', '')
        if name and not name.startswith('['):  # Skip template entries
            # Clean up name
            name = name.strip()
            if name:
                items[item_id] = name

    print(f"Loaded {len(items)} items from {filename}")
    return items

def load_all_databases():
    """Load all item name databases."""
    return {
        'weapons': load_item_database("EquipParamWeapon.param.xml"),
        'armor': load_item_database("EquipParamProtector.param.xml"),
        'accessories': load_item_database("EquipParamAccessory.param.xml"),
        'goods': load_item_database("EquipParamGoods.param.xml"),
        'gems': load_item_database("EquipParamGem.param.xml"),
    }

def resolve_item(databases: dict, item_id: int, lot_category: int) -> tuple[str, str]:
    """
    Resolve item name and determine category.

    lot_category from ItemLotParam:
    0 = Weapon
    1 = Protector (Armor)
    2 = Accessory (Talisman)
    3 = Goods
    4 = Gem (Ash of War)

    IMPORTANT: Item IDs are NOT globally unique - the same numeric ID can exist
    in multiple equipment databases (e.g., 1210 = "Exalted Flesh" in Goods,
    but 1210 = "Bull-Goat's Talisman" in Accessories). We MUST use lot_category
    to pick the correct database.
    """
    # Map lot_category to database and category - this is the PRIMARY lookup
    # VERIFIED from actual ItemLotParam_map.param.xml entries:
    # - Category 1: Goods (item 10100 = "Smithing Stone [1]")
    # - Category 2: Weapons (item 2060000 = "Ornamental Straight Sword")
    # - Category 3: Protector/Armor (item 590000 = "All-Knowing Helm")
    # - Category 4: Accessory/Talisman (item 5050 = "Assassin's Crimson Dagger")
    # - Category 5: Gem/Ash of War (item 22100 = "Ash of War: Black Flame Tornado")
    category_map = {
        0: ('goods', "Other"),       # Empty/special - fall back to goods
        1: ('goods', "Other"),       # Goods (consumables, materials, etc.)
        2: ('weapons', "Weapons"),   # Weapons
        3: ('armor', "Armor"),       # Protector (Armor)
        4: ('accessories', "Talismans"),  # Accessory (Talisman)
        5: ('gems', "AshesOfWar"),   # Gem (Ash of War)
    }

    db_key, category = category_map.get(lot_category, ('goods', "Other"))
    db = databases.get(db_key, {})

    # Try to find item in the CORRECT database based on lot_category
    if item_id in db:
        name = db[item_id]

        # First check special item ID ranges for categorization refinement
        if item_id in GOLDEN_RUNE_IDS:
            return name, "GoldenRunes"
        if item_id in SMITHING_STONE_IDS:
            return name, "SmithingStones"
        if item_id in SOMBER_STONE_IDS:
            return name, "SomberStones"
        if item_id in GLOVEWORT_IDS:
            return name, "Glovewort"
        if item_id in GREAT_RUNE_IDS:
            return name, "KeyItems"

        # Refine category for goods based on item name
        if category == "Other" and db_key == 'goods':
            name_lower = name.lower()
            if 'cookbook' in name_lower:
                category = "KeyItems"
            elif 'key' in name_lower or 'bell bearing' in name_lower:
                category = "KeyItems"
            elif any(x in name_lower for x in ['meat', 'liver', 'foot', 'grease', 'remedy', 'bolus']):
                category = "Consumables"
            elif any(x in name_lower for x in ['bone', 'root', 'flower', 'mushroom', 'crystal', 'string']):
                category = "CraftingMaterials"

        return name, category

    # Fallback: Try other databases in a STRICT priority order
    # This handles cases where lot_category might be incorrect in the source data
    # Priority: gems (Ashes of War) > goods > weapons > armor > accessories
    # This order prevents accessories from shadowing goods items
    fallback_order = [
        ('gems', "AshesOfWar"),
        ('goods', "Other"),
        ('weapons', "Weapons"),
        ('armor', "Armor"),
        ('accessories', "Talismans"),
    ]

    for fb_db_key, fb_category in fallback_order:
        if fb_db_key == db_key:
            continue  # Already tried this one
        fb_db = databases.get(fb_db_key, {})
        if item_id in fb_db:
            name = fb_db[item_id]
            # Check special ranges
            if item_id in GOLDEN_RUNE_IDS:
                return name, "GoldenRunes"
            if item_id in SMITHING_STONE_IDS:
                return name, "SmithingStones"
            if item_id in SOMBER_STONE_IDS:
                return name, "SomberStones"
            if item_id in GLOVEWORT_IDS:
                return name, "Glovewort"
            return name, fb_category

    return f"Unknown Item {item_id}", "Other"

def normalize_lot_id(lot_id: int) -> int:
    """
    Normalize malformed lot IDs.

    Some lot IDs in ItemLotParam appear to be truncated 10-digit overworld IDs.
    For example, 942370060 should be 1042370060 (Limgrave tile 42_37).

    Pattern: 9-digit IDs where the leading digit appears to be a corrupted '10'
    (e.g., '9' instead of '10') should have the first digit replaced with '10'.
    Format: 9XXYYZZZZ -> 10XXYYZZZZ where XX=33-60, YY=30-60
    """
    lot_str = str(lot_id)

    # Check for 9-digit IDs that look like truncated 10-digit overworld IDs
    if len(lot_str) == 9:
        first_digit = lot_str[0]
        # Pattern: 9XXYYZZZZ where the '9' should be '10'
        # XX is at positions [1:3], YY is at positions [3:5]
        if first_digit in ('9', '1', '2'):  # Could be corrupted 10, 10, or 20 prefix
            try:
                potential_tile_x = int(lot_str[1:3])
                potential_tile_y = int(lot_str[3:5])
                # Valid overworld tile ranges
                if 33 <= potential_tile_x <= 60 and 30 <= potential_tile_y <= 60:
                    # Determine correct prefix based on first digit
                    if first_digit == '9' or first_digit == '1':
                        # Likely missing '0' after the leading '1', or '9' should be '10'
                        return int('10' + lot_str[1:])
                    elif first_digit == '2':
                        # DLC area - '2' should be '20'
                        return int('20' + lot_str[1:])
            except ValueError:
                pass

    return lot_id


def get_region_from_lot_id(lot_id: int, event_flag: int = 0) -> str:
    """
    Determine region from lot ID and optionally event flag.

    Formats:
    - 10-digit (10XXYYZZZZ): Overworld pickups, XX=tileX, YY=tileY
    - 10-digit (20XXYYZZZZ): DLC overworld pickups
    - 8-digit base game (AABBxxxx): Legacy dungeons, AA=area code, BB=section
    - 8-digit DLC (2Axxxxxx): DLC legacy dungeons
    - 6-digit (10xxxx, 11xxxx): NPC drops and quest rewards (NOT dungeon pickups)
    - 5-digit (AAxxx): Mini-dungeons and caves

    For short lot IDs, the event_flag can help determine if it's a quest item.
    """
    # First normalize the lot ID to handle truncated IDs
    lot_id = normalize_lot_id(lot_id)
    lot_str = str(lot_id)

    # Check event_flag for NPC/quest rewards
    # - 400xxx flags are NPC quest rewards
    # - 60xxx flags are specific progression items (like Spectral Steed Whistle)
    if event_flag > 0:
        flag_str = str(event_flag)
        # 400xxx flags = NPC drops and quest rewards
        if len(flag_str) == 6 and flag_str.startswith("400"):
            return "NPCReward"
        # Only specific 6xxxx ranges are quest items, not all of them
        # 60xxx flags for major quest progression items
        if event_flag >= 60000 and event_flag <= 60999:
            return "QuestReward"

    # 10-digit format: 10XXYYZZZZ or 20XXYYZZZZ
    if len(lot_str) == 10:
        try:
            prefix = lot_str[0:2]  # '10' or '20'
            tile_x = int(lot_str[2:4])  # XX
            tile_y = int(lot_str[4:6])  # YY

            if prefix == '20':
                return "ShadowOfTheErdtree"

            if prefix == '10':
                # Map overworld tile coordinates to regions
                return get_overworld_region(tile_x, tile_y)
        except:
            pass

    # 8-digit format: Legacy dungeons (AABBCCCC)
    if len(lot_str) == 8:
        area_code = lot_str[:2]

        # DLC legacy dungeons (2Axxxxxx, 28xxxxxx)
        if area_code in ("20", "21", "22", "23", "24", "25", "26", "27", "28", "29"):
            return "ShadowOfTheErdtree"

        # DLC gaols and dungeons (4Axxxxxx)
        if area_code in ("40", "41", "42", "43", "44", "45", "46", "47", "48", "49"):
            return "ShadowOfTheErdtree"

        legacy_regions = {
            "10": "StormveilCastle",
            "11": "RayaLucaria",
            "12": "Catacombs",          # Various catacombs (120xxxxx)
            "13": "Leyndell",
            "14": "ShunningGrounds",
            "15": "Haligtree",
            "16": "FarumAzula",
            "18": "CariaManor",
            "19": "RedmaneCastle",
            "30": "Caves",               # Various caves (30xxxxxx)
            "31": "Caves",
            "32": "Caves",
            "33": "VolcanoManor",
            "34": "Leyndell",            # Royal Capital
            "35": "MohgwynPalace",
            "36": "Underground",         # Siofra/Ainsel
            "37": "Underground",
            "38": "Underground",         # Nokron/Nokstella
            "39": "DeepRoot",
            "99": "BossDrops",           # Special/boss rewards
        }

        return legacy_regions.get(area_code, "Unknown")

    # 9-digit format - check prefixes
    if len(lot_str) == 9:
        area_code = lot_str[:2]
        if area_code in ("12", "30", "31", "32"):
            return "Caves"
        # 9-digit starting with 90 are often boss-related
        if area_code == "90":
            return "BossDrops"
        return "Unknown"

    # 7-digit format (mostly 99xxxxx boss/special drops)
    if len(lot_str) == 7:
        if lot_str.startswith("99"):
            return "BossDrops"
        if lot_str.startswith("46"):
            return "ShadowOfTheErdtree"  # DLC boss drops
        return "Unknown"

    # 6-digit format (AAxxxx)
    # IMPORTANT: 6-digit lot IDs starting with 10xxxx, 11xxxx are NPC drops and
    # quest rewards, NOT legacy dungeon world pickups. Legacy dungeon pickups
    # use 8-digit format (AABBxxxx where AA=area, BB=section).
    if len(lot_str) == 6:
        area_code = lot_str[:2]

        six_digit_regions = {
            # 10xxxx and 11xxxx are NPC/quest rewards (NOT dungeons)
            "10": "NPCReward",           # 10xxxx - NPC drops (Melina, Varre, etc.)
            "11": "NPCReward",           # 11xxxx - NPC drops
            "12": "NPCReward",           # 12xxxx - NPC drops
            "13": "NPCReward",           # 13xxxx - NPC drops
            "14": "NPCReward",           # 14xxxx - NPC drops
            "15": "NPCReward",           # 15xxxx - NPC drops
            "16": "NPCReward",           # 16xxxx - NPC drops
            "18": "NPCReward",           # 18xxxx - NPC drops
            "19": "NPCReward",           # 19xxxx - NPC drops
            # DLC NPC drops
            "20": "ShadowOfTheErdtree",  # 20xxxx - DLC NPC drops
            "21": "ShadowOfTheErdtree",  # 21xxxx - DLC
            "22": "ShadowOfTheErdtree",  # 22xxxx - DLC
            "23": "ShadowOfTheErdtree",  # 23xxxx - DLC
            "24": "ShadowOfTheErdtree",  # 24xxxx - DLC
            "25": "ShadowOfTheErdtree",  # 25xxxx - DLC
            # Cave-related drops (often mini-dungeon rewards)
            "30": "Caves",               # 30xxxx - Cave drops
            "31": "Caves",               # 31xxxx - Cave drops
            "32": "Caves",               # 32xxxx - Cave drops
            "33": "NPCReward",           # 33xxxx - NPC drops (often at Volcano Manor)
            "34": "NPCReward",           # 34xxxx - NPC drops
            "35": "NPCReward",           # 35xxxx - NPC drops
            "36": "NPCReward",           # 36xxxx - NPC drops
            "37": "NPCReward",           # 37xxxx - NPC drops
            "38": "NPCReward",           # 38xxxx - NPC drops
            "39": "NPCReward",           # 39xxxx - NPC drops
            "40": "ShadowOfTheErdtree",  # 40xxxx - DLC dungeons
            "41": "ShadowOfTheErdtree",  # 41xxxx - DLC gaols
            "42": "ShadowOfTheErdtree",  # 42xxxx - DLC
            "43": "ShadowOfTheErdtree",  # 43xxxx - DLC
            "60": "OverworldQuest",      # 60xxxx - Quest/progression items
            "61": "OverworldQuest",      # 61xxxx - Quest/progression items
            "62": "OverworldQuest",      # 62xxxx - Maps
            "63": "OverworldQuest",      # 63xxxx - Quest items
            "64": "OverworldQuest",      # 64xxxx - Quest items
            "65": "OverworldQuest",      # 65xxxx - Whetblades etc.
            "66": "OverworldQuest",      # 66xxxx - Pots
            "67": "OverworldQuest",      # 67xxxx - Cookbooks
            "68": "OverworldQuest",      # 68xxxx - Cookbooks
            "99": "BossDrops",           # 99xxxx - Boss rewards
        }

        return six_digit_regions.get(area_code, "NPCReward")

    # 5-digit format (mostly NPC drops and quest rewards, similar to 6-digit)
    if len(lot_str) == 5:
        area_code = lot_str[:2]

        five_digit_regions = {
            # 5-digit 10xxx, 11xxx etc are NPC/quest rewards like their 6-digit counterparts
            "10": "NPCReward",           # 10xxx - NPC drops
            "11": "NPCReward",           # 11xxx - NPC drops
            "12": "NPCReward",           # 12xxx - NPC drops
            "20": "ShadowOfTheErdtree",  # 20xxx - DLC
            "30": "Caves",               # 30xxx - Caves
            "40": "ShadowOfTheErdtree",  # 40xxx - DLC mini-dungeons
            "41": "ShadowOfTheErdtree",  # 41xxx - DLC gaols
            "42": "ShadowOfTheErdtree",  # 42xxx - DLC
            "43": "ShadowOfTheErdtree",  # 43xxx - DLC
            "50": "NPCReward",           # 50xxx - NPC drops
            "60": "OverworldQuest",      # 60xxx - Progression
            "99": "BossDrops",           # 99xxx - Boss items
        }

        return five_digit_regions.get(area_code, "NPCReward")

    # Very short IDs are usually special rewards or boss items
    if len(lot_str) <= 4:
        return "BossDrops"

    return "Unknown"

def get_overworld_region(tile_x: int, tile_y: int) -> str:
    """
    Map overworld tile coordinates to region names.

    Based on actual tile data analysis from ItemLotParam_map.param.xml:
    - Tile X ranges from 33 to 54
    - Tile Y ranges from 30 to 58

    Elden Ring map layout (verified from known item locations):
    - Weeping Peninsula: SW (X=33-36, Y=40-47)
    - Limgrave: South-central (X=37-43, Y=37-49) - includes Gatefront, First Step
    - Stormhill: North Limgrave transition (X=38-43, Y=47-51)
    - Caelid: East (X=44-52, Y=31-44)
    - Dragonbarrow: NE Caelid (X=49-52, Y=36-44)
    - Liurnia: NW (X=34-43, Y=50-55)
    - Altus Plateau: Central-North (X=38-46, Y=48-54)
    - Mt. Gelmir: West (X=35-39, Y=51-54)
    - Capital Outskirts: Around Leyndell (X=40-46, Y=48-54)
    - Mountaintops: NE (X=47-54, Y=51-58)
    - Consecrated Snowfield: Far North (X=47-50, Y=55-58)
    """

    # Underground areas use different coordinates - detected by low X values
    if tile_x < 20:
        return "Underground"

    # Weeping Peninsula - southernmost area, south of Limgrave
    # Roughly X=33-44, Y=30-36 (the peninsula south of the starting area)
    if tile_y >= 30 and tile_y <= 36:
        if tile_x >= 33 and tile_x <= 44:
            return "WeepingPeninsula"

    # Limgrave - starting area including Gatefront Ruins, First Step, Stormhill
    # Main area: X=37-44, Y=36-44 (stops before Liurnia which starts ~Y=44-46)
    if tile_x >= 37 and tile_x <= 44 and tile_y >= 36 and tile_y <= 44:
        return "Limgrave"

    # Extended Limgrave - tiles at X=41-44 with lower Y values are still Limgrave
    if tile_x >= 41 and tile_x <= 44 and tile_y >= 32 and tile_y <= 40:
        return "Limgrave"

    # Caelid - eastern region (high X values, lower Y values)
    # Note: tile_x=44 is still Limgrave, Caelid starts at tile_x=45
    if tile_x >= 45 and tile_x <= 52 and tile_y >= 31 and tile_y <= 44:
        return "Caelid"

    # Dragonbarrow - northeast extension of Caelid
    if tile_x >= 49 and tile_x <= 52 and tile_y >= 36 and tile_y <= 44:
        return "Caelid"

    # Extended Limgrave East - tile_x=44 is still Limgrave
    if tile_x == 44 and tile_y >= 31 and tile_y <= 44:
        return "Limgrave"

    # Consecrated Snowfield - far north (X=47-54, Y=55-58)
    # Accessed after Grand Lift of Rold
    if tile_y >= 55 and tile_x >= 47:
        return "ConsecratedSnowfield"

    # Mountaintops of the Giants - northeast (Y=51-58, X=47-54)
    if tile_x >= 47 and tile_y >= 51:
        if tile_y >= 55:
            return "ConsecratedSnowfield"
        return "Mountaintops"

    # Forbidden Lands - corridor between Altus and Mountaintops (X=46, Y=51-58)
    # This includes the approach to the Mountaintops
    if tile_x == 46 and tile_y >= 51:
        return "ForbiddenLands"

    # Mt. Gelmir - west side, volcanic area (X=35-39, Y=50-56)
    if tile_x >= 35 and tile_x <= 39 and tile_y >= 50 and tile_y <= 56:
        return "MtGelmir"

    # Altus Plateau - central-north golden plains (X=40-46, Y=47-56)
    # Includes Windmill Village (Dominula) area at Y=55
    if tile_x >= 40 and tile_x <= 46 and tile_y >= 47 and tile_y <= 56:
        return "AltusPlateau"

    # Liurnia of the Lakes - northwest lake area
    # Main lake area: X=34-43, Y=42-52
    # This includes Liurnia West, East, and South
    if tile_x >= 34 and tile_x <= 43 and tile_y >= 42 and tile_y <= 52:
        # Exclude tiles that should be Mt. Gelmir (X=35-39, Y=50+)
        if tile_x >= 35 and tile_x <= 39 and tile_y >= 50:
            return "MtGelmir"
        # Exclude tiles that should be Altus (X=40+, Y=47+)
        if tile_x >= 40 and tile_y >= 47:
            return "AltusPlateau"
        return "Liurnia"

    # Fallback - try to categorize by general position
    if tile_y <= 44:
        if tile_x >= 44:
            return "Caelid"
        return "Limgrave"
    elif tile_y <= 52:
        if tile_x <= 39:
            return "Liurnia"
        elif tile_x <= 46:
            return "AltusPlateau"
        else:
            return "Mountaintops"
    else:
        # tile_y > 52 - northern areas
        if tile_x >= 47:
            if tile_y >= 55:
                return "ConsecratedSnowfield"
            return "Mountaintops"
        elif tile_x >= 40:
            return "AltusPlateau"  # Windmill Village and northern Altus
        elif tile_x >= 35:
            return "MtGelmir"
        else:
            return "Unknown"

def is_world_pickup(lot_id: int, flag_id: int, item_id: int) -> bool:
    """
    Filter out non-world pickups.

    Returns False for:
    - Boss rewards (Great Runes, Remembrances)
    - Quest rewards
    - Shop items
    - Items with invalid or zero flags
    """
    # Skip items with no flag (can't track collection)
    if flag_id == 0:
        return False

    # Skip Great Runes - they're boss rewards with special flags (171-197)
    if item_id in GREAT_RUNE_IDS:
        return False

    # Normalize lot_id for length checks (handles truncated 9-digit IDs)
    normalized_lot_id = normalize_lot_id(lot_id)

    # Skip items with very low flag IDs (usually boss/special rewards)
    # World pickup flags are typically 5+ digits or 10-digit format
    if flag_id < 1000 and len(str(normalized_lot_id)) < 8:
        return False

    # Skip if lot_id is suspiciously small (usually NPC drops or special items)
    if normalized_lot_id < 10000:
        return False

    return True

def escape_rust_string(s: str) -> str:
    """Escape a string for Rust source code."""
    return s.replace('\\', '\\\\').replace('"', '\\"')

def main():
    print("Loading item databases...")
    databases = load_all_databases()

    print(f"\nParsing {GAME_FILES / 'ItemLotParam_map.param.xml'}...")
    tree = ET.parse(GAME_FILES / "ItemLotParam_map.param.xml")
    root = tree.getroot()

    pickups = []
    skipped_count = 0
    category_counts = defaultdict(int)
    region_counts = defaultdict(int)

    for row in root.findall('.//row'):
        raw_lot_id = int(row.get('id', 0))
        get_item_flag_id = int(row.get('getItemFlagId', 0))

        # CRITICAL FIX (2026-01-23): For tile-based world pickups, the game stores
        # the ROW ID (lot_id) as the actual event flag, NOT getItemFlagId.
        # getItemFlagId = lot_id + 7000, placing local_id in 7000+ range (unstorable).
        # The tile slot only has 875 bytes = 7000 flags, so local_id >= 7000 has NO storage.
        is_tile_based = 1_000_000_000 <= raw_lot_id < 3_000_000_000
        flag_id = raw_lot_id if is_tile_based else get_item_flag_id

        # Process primary item in the lot (slot 01)
        item_id = int(row.get('lotItemId01', 0))
        lot_category = int(row.get('lotItemCategory01', 0))
        quantity = int(row.get('lotItemNum01', 1))

        if item_id == 0:
            continue

        # Filter out non-world pickups
        if not is_world_pickup(raw_lot_id, flag_id, item_id):
            skipped_count += 1
            continue

        # Normalize the lot_id for consistent storage
        # This fixes truncated 9-digit IDs like 942370060 -> 1042370060
        lot_id = normalize_lot_id(raw_lot_id)

        # Resolve item name and category
        item_name, category = resolve_item(databases, item_id, lot_category)

        # Get region from lot ID and event flag (already uses normalized ID internally)
        region = get_region_from_lot_id(lot_id, flag_id)

        pickups.append({
            'lot_id': lot_id,
            'flag_id': flag_id,
            'item_id': item_id,
            'name': item_name,
            'quantity': quantity,
            'category': category,
            'region': region,
        })

        category_counts[category] += 1
        region_counts[region] += 1

    print(f"\nProcessed {len(pickups)} world pickups (skipped {skipped_count})")
    print(f"\nBy category:")
    for cat, count in sorted(category_counts.items(), key=lambda x: -x[1]):
        print(f"  {cat}: {count}")
    print(f"\nBy region:")
    for reg, count in sorted(region_counts.items(), key=lambda x: -x[1]):
        print(f"  {reg}: {count}")

    # Sort pickups by region then name for better organization
    pickups.sort(key=lambda p: (p['region'], p['name']))

    # Generate Rust code
    print(f"\nGenerating {OUTPUT_FILE}...")

    with open(OUTPUT_FILE, 'w') as f:
        f.write('//! World Pickup Database\n')
        f.write('//!\n')
        f.write(f'//! Generated from ItemLotParam_map.param.xml with {len(pickups)} pickups.\n')
        f.write('//! Each pickup maps an event flag to an item that can be collected in the world.\n')
        f.write('//!\n')
        f.write('//! Auto-generated - do not edit manually.\n\n')

        # Struct definition
        f.write('/// A world pickup entry\n')
        f.write('#[derive(Debug, Clone)]\n')
        f.write('pub struct WorldPickup {\n')
        f.write('    pub item_lot_id: u32,\n')
        f.write('    pub event_flag: u32,\n')
        f.write('    pub item_id: u32,\n')
        f.write('    pub name: &\'static str,\n')
        f.write('    pub quantity: u32,\n')
        f.write('    pub category: PickupCategory,\n')
        f.write('    pub region: &\'static str,\n')
        f.write('}\n\n')

        # Category enum
        f.write('/// Pickup categories\n')
        f.write('#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]\n')
        f.write('pub enum PickupCategory {\n')
        f.write('    GoldenRunes,\n')
        f.write('    SmithingStones,\n')
        f.write('    SomberStones,\n')
        f.write('    Glovewort,\n')
        f.write('    Weapons,\n')
        f.write('    Armor,\n')
        f.write('    Talismans,\n')
        f.write('    AshesOfWar,\n')
        f.write('    KeyItems,\n')
        f.write('    CraftingMaterials,\n')
        f.write('    Consumables,\n')
        f.write('    Other,\n')
        f.write('}\n\n')

        f.write('impl PickupCategory {\n')
        f.write('    pub fn display_name(&self) -> &\'static str {\n')
        f.write('        match self {\n')
        f.write('            PickupCategory::GoldenRunes => "Golden Runes",\n')
        f.write('            PickupCategory::SmithingStones => "Smithing Stones",\n')
        f.write('            PickupCategory::SomberStones => "Somber Smithing Stones",\n')
        f.write('            PickupCategory::Glovewort => "Glovewort",\n')
        f.write('            PickupCategory::Weapons => "Weapons",\n')
        f.write('            PickupCategory::Armor => "Armor",\n')
        f.write('            PickupCategory::Talismans => "Talismans",\n')
        f.write('            PickupCategory::AshesOfWar => "Ashes of War",\n')
        f.write('            PickupCategory::KeyItems => "Key Items",\n')
        f.write('            PickupCategory::CraftingMaterials => "Crafting Materials",\n')
        f.write('            PickupCategory::Consumables => "Consumables",\n')
        f.write('            PickupCategory::Other => "Other",\n')
        f.write('        }\n')
        f.write('    }\n')
        f.write('}\n\n')

        # Static array of pickups
        f.write(f'/// All world pickups ({len(pickups)} entries)\n')
        f.write('pub static WORLD_PICKUPS: &[WorldPickup] = &[\n')

        for p in pickups:
            name_escaped = escape_rust_string(p['name'])
            region_escaped = escape_rust_string(p['region'])
            f.write(f'    WorldPickup {{\n')
            f.write(f'        item_lot_id: {p["lot_id"]},\n')
            f.write(f'        event_flag: {p["flag_id"]},\n')
            f.write(f'        item_id: {p["item_id"]},\n')
            f.write(f'        name: "{name_escaped}",\n')
            f.write(f'        quantity: {p["quantity"]},\n')
            f.write(f'        category: PickupCategory::{p["category"]},\n')
            f.write(f'        region: "{region_escaped}",\n')
            f.write(f'    }},\n')

        f.write('];\n\n')

        # Helper functions
        f.write('/// Get pickups by category\n')
        f.write('pub fn get_pickups_by_category(category: PickupCategory) -> Vec<&\'static WorldPickup> {\n')
        f.write('    WORLD_PICKUPS.iter().filter(|p| p.category == category).collect()\n')
        f.write('}\n\n')

        f.write('/// Get pickups by region\n')
        f.write('pub fn get_pickups_by_region(region: &str) -> Vec<&\'static WorldPickup> {\n')
        f.write('    WORLD_PICKUPS.iter().filter(|p| p.region == region).collect()\n')
        f.write('}\n\n')

        f.write('/// Get pickup by event flag\n')
        f.write('pub fn get_pickup_by_flag(event_flag: u32) -> Option<&\'static WorldPickup> {\n')
        f.write('    WORLD_PICKUPS.iter().find(|p| p.event_flag == event_flag)\n')
        f.write('}\n\n')

        f.write('/// Get all unique regions\n')
        f.write('pub fn get_all_regions() -> Vec<&\'static str> {\n')
        f.write('    let mut regions: Vec<_> = WORLD_PICKUPS.iter()\n')
        f.write('        .map(|p| p.region)\n')
        f.write('        .collect::<std::collections::HashSet<_>>()\n')
        f.write('        .into_iter()\n')
        f.write('        .collect();\n')
        f.write('    regions.sort();\n')
        f.write('    regions\n')
        f.write('}\n\n')

        f.write('/// Get category counts\n')
        f.write('pub fn get_category_counts() -> Vec<(PickupCategory, usize)> {\n')
        f.write('    use std::collections::HashMap;\n')
        f.write('    let mut counts: HashMap<PickupCategory, usize> = HashMap::new();\n')
        f.write('    for pickup in WORLD_PICKUPS {\n')
        f.write('        *counts.entry(pickup.category).or_insert(0) += 1;\n')
        f.write('    }\n')
        f.write('    let mut result: Vec<_> = counts.into_iter().collect();\n')
        f.write('    result.sort_by(|a, b| b.1.cmp(&a.1));\n')
        f.write('    result\n')
        f.write('}\n')

    print(f"Done! Generated {OUTPUT_FILE}")

if __name__ == '__main__':
    main()
