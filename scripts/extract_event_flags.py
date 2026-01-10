#!/usr/bin/env python3
"""
Extract Event Flags from Elden Ring Decompiled Game Files

Extracts event flags from:
- ItemLotParam_map.param.xml (world pickups)
- BonfireWarpParam.param.xml (graces)
- ShopLineupParam.param.xml (shop items)
- common.emevd.js (Great Runes, Remembrances, etc.)

Output: Markdown table format with full data preservation.
"""

import xml.etree.ElementTree as ET
import json
import re
from pathlib import Path
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional, Any

# Base paths
GAME_FILES = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files")
REGULATION_BIN = GAME_FILES / "regulation-bin"
MSG_ENGUS = GAME_FILES / "msg" / "engus" / "item-msgbnd-dcx"
MSG_DLC01 = GAME_FILES / "msg" / "engus" / "item_dlc01-msgbnd-dcx"
MSG_DLC02 = GAME_FILES / "msg" / "engus" / "item_dlc02-msgbnd-dcx"
EVENT_DIR = GAME_FILES / "event"
MSB_DIR = GAME_FILES / "map" / "mapstudio"

@dataclass
class EventFlag:
    """Event flag data structure preserving all game file information."""
    flag_id: int
    name: str
    category: str
    region: str
    source_file: str
    source_row_id: Optional[int] = None
    item_id: Optional[int] = None
    item_category: Optional[int] = None
    # Spatial data (preserved from game files)
    area_no: Optional[int] = None        # areaNo from game (10=Stormveil, 60=overworld, etc.)
    grid_x: Optional[int] = None         # gridXNo from game (tile index for overworld, dungeon ID for dungeons)
    grid_z: Optional[int] = None         # gridZNo from game (tile index for overworld, section for dungeons)
    pos_x: Optional[float] = None        # posX LOCAL coordinate within tile/dungeon
    pos_y: Optional[float] = None        # posY height coordinate
    pos_z: Optional[float] = None        # posZ LOCAL coordinate within tile/dungeon
    map_tile: Optional[str] = None       # Derived: "m60_42_37" format
    region_id: Optional[int] = None      # bonfireSubCategoryId or derived
    # Computed world coordinates (only valid for overworld areaNo 60/61)
    is_overworld: bool = False           # True if areaNo is 60 (base) or 61 (DLC)
    world_x: Optional[float] = None      # Computed: grid_x * 256 + pos_x (overworld only)
    world_z: Optional[float] = None      # Computed: grid_z * 256 + pos_z (overworld only)
    # Area classification
    area_type: str = "unknown"           # overworld_surface, underworld, subterranean, legacy_dungeon, minor_dungeon, divine_tower, tutorial
    is_dlc: bool = False                 # True if from Shadow of the Erdtree DLC
    raw_data: Dict[str, Any] = field(default_factory=dict)

def load_name_lookup(fmg_path: Path) -> Dict[int, str]:
    """Load name lookup from FMG XML file."""
    lookup = {}
    if not fmg_path.exists():
        return lookup

    try:
        tree = ET.parse(fmg_path)
        root = tree.getroot()
        for text in root.findall(".//text"):
            text_id = int(text.get("id", 0))
            name = text.text or ""
            if name and name != "%null%" and name != "[ERROR]":
                lookup[text_id] = name
    except Exception as e:
        print(f"  Warning: Error parsing {fmg_path.name}: {e}")

    return lookup

def merge_lookups(base: Dict[int, str], *others: Dict[int, str]) -> Dict[int, str]:
    """Merge multiple lookups, later ones override earlier."""
    result = dict(base)
    for other in others:
        result.update(other)
    return result


def load_world_map_pieces() -> Dict[int, Dict]:
    """Load region names and bounds from WorldMapPieceParam."""
    pieces = {}
    xml_path = REGULATION_BIN / "WorldMapPieceParam.param.xml"

    if not xml_path.exists():
        print(f"  Warning: {xml_path.name} not found")
        return pieces

    try:
        tree = ET.parse(xml_path)
        root = tree.getroot()
        for row in root.findall(".//row"):
            piece_id = int(row.get("id", 0))
            pieces[piece_id] = {
                "name": row.get("paramdexName", f"Region_{piece_id}"),
                "open_flag": int(row.get("openEventFlagId", 0)),
                "acquisition_flag": int(row.get("acquisitionEventFlagId", 0)),
            }
    except Exception as e:
        print(f"  Warning: Error parsing {xml_path.name}: {e}")

    return pieces


def load_world_map_points() -> Dict[int, Dict]:
    """Load POI coordinates indexed by event flag ID."""
    points = {}
    xml_path = REGULATION_BIN / "WorldMapPointParam.param.xml"

    if not xml_path.exists():
        print(f"  Warning: {xml_path.name} not found")
        return points

    try:
        tree = ET.parse(xml_path)
        root = tree.getroot()
        for row in root.findall(".//row"):
            flag_id = int(row.get("eventFlagId", 0))
            if flag_id == 0:
                continue

            points[flag_id] = {
                "row_id": int(row.get("id", 0)),
                "name": row.get("paramdexName", ""),
                "pos_x": float(row.get("posX", 0)),
                "pos_y": float(row.get("posY", 0)),
                "pos_z": float(row.get("posZ", 0)),
                "area_no": int(row.get("areaNo", 0)),
                "grid_x": int(row.get("gridXNo", 0)),
                "grid_z": int(row.get("gridZNo", 0)),
                "icon_id": int(row.get("iconId", 0)),
                "text_id": int(row.get("textId1", 0)),
            }
    except Exception as e:
        print(f"  Warning: Error parsing {xml_path.name}: {e}")

    return points


def parse_msb_dir_name(msb_dir_name: str) -> Optional[Dict]:
    """
    Parse MSB directory name to extract area and grid info.

    Format: m{area}_{gridX}_{gridZ}_{section}-msb-dcx
    Examples:
    - m60_42_37_00-msb-dcx → area=60, grid_x=42, grid_z=37
    - m12_01_00_00-msb-dcx → area=12, grid_x=1, grid_z=0
    - m10_00_00_00-msb-dcx → area=10, grid_x=0, grid_z=0
    """
    # Pattern: m{area}_{gridX}_{gridZ}_{section}-msb-dcx
    match = re.match(r'm(\d+)_(\d+)_(\d+)_(\d+)-msb-dcx', msb_dir_name)
    if match:
        return {
            "area_no": int(match.group(1)),
            "grid_x": int(match.group(2)),
            "grid_z": int(match.group(3)),
            "section": int(match.group(4))
        }
    return None


def load_msb_treasure_positions() -> Dict[int, Dict]:
    """
    Load treasure positions from MSB (Map Studio Binary) files.

    Returns dict keyed by ItemLotID (row_id from ItemLotParam):
    {item_lot_id: {"pos_x": float, "pos_y": float, "pos_z": float,
                   "area_no": int, "grid_x": int, "grid_z": int,
                   "asset_name": str, "msb_dir": str}}
    """
    positions = {}

    if not MSB_DIR.exists():
        print(f"  Warning: MSB directory not found: {MSB_DIR}")
        return positions

    # Find all MSB directories (m60_XX_YY_00-msb-dcx pattern for base game)
    msb_dirs = sorted(MSB_DIR.glob("m*-msb-dcx"))
    print(f"  Found {len(msb_dirs)} MSB directories")

    processed = 0
    for msb_dir in msb_dirs:
        # Parse area/grid from directory name
        msb_location = parse_msb_dir_name(msb_dir.name)
        treasure_dir = msb_dir / "Event" / "Treasure"
        asset_dir = msb_dir / "Part" / "Asset"

        if not treasure_dir.exists() or not asset_dir.exists():
            continue

        # Step 1: Parse Treasure events to get ItemLotID → TreasurePartName
        treasure_mapping = {}  # {item_lot_id: treasure_part_name}
        for treasure_file in treasure_dir.glob("*.xml"):
            try:
                tree = ET.parse(treasure_file)
                root = tree.getroot()

                # Look for <ItemLotID> and <TreasurePartName>
                item_lot_elem = root.find(".//ItemLotID")
                part_name_elem = root.find(".//TreasurePartName")

                if item_lot_elem is not None and part_name_elem is not None:
                    item_lot_id = int(item_lot_elem.text or 0)
                    part_name = part_name_elem.text or ""
                    if item_lot_id > 0 and part_name:
                        treasure_mapping[item_lot_id] = part_name
            except Exception:
                continue

        # Step 2: Parse Asset files to get positions
        asset_positions = {}  # {asset_name: (x, y, z)}
        for asset_file in asset_dir.glob("*.xml"):
            try:
                tree = ET.parse(asset_file)
                root = tree.getroot()

                name_elem = root.find(".//Name")
                pos_elem = root.find(".//Position")

                if name_elem is not None and pos_elem is not None:
                    asset_name = name_elem.text or ""
                    x_elem = pos_elem.find("X")
                    y_elem = pos_elem.find("Y")
                    z_elem = pos_elem.find("Z")

                    if x_elem is not None and y_elem is not None and z_elem is not None:
                        asset_positions[asset_name] = (
                            float(x_elem.text or 0),
                            float(y_elem.text or 0),
                            float(z_elem.text or 0)
                        )
            except Exception:
                continue

        # Step 3: Link ItemLotID → Position (with area/grid from MSB dir name)
        for item_lot_id, part_name in treasure_mapping.items():
            if part_name in asset_positions:
                x, y, z = asset_positions[part_name]
                entry = {
                    "pos_x": x,
                    "pos_y": y,
                    "pos_z": z,
                    "asset_name": part_name,
                    "msb_dir": msb_dir.name
                }
                # Add area/grid from MSB directory name
                if msb_location:
                    entry["area_no"] = msb_location["area_no"]
                    entry["grid_x"] = msb_location["grid_x"]
                    entry["grid_z"] = msb_location["grid_z"]
                positions[item_lot_id] = entry

        processed += 1
        if processed % 100 == 0:
            print(f"    Processed {processed}/{len(msb_dirs)} MSB directories...")

    print(f"  Loaded {len(positions)} treasure positions from MSB files")
    return positions


def format_map_tile(area_no: int, grid_x: int, grid_z: int) -> str:
    """Format map tile string from area and grid coordinates."""
    if area_no == 0 and grid_x == 0 and grid_z == 0:
        return None
    return f"m{area_no}_{grid_x:02d}_{grid_z:02d}"


def is_overworld_area(area_no: int) -> bool:
    """Check if area_no represents overworld (where grid coordinates form a world map)."""
    # 60 = Base game overworld
    # 61 = DLC (Shadow of the Erdtree) overworld
    return area_no in (60, 61)


def get_area_type(area_no: int) -> str:
    """
    Classify area_no into location types.

    Returns one of:
    - "overworld_surface": Open world surface (base game or DLC)
    - "underworld": Underground open areas (Siofra, Ainsel, Nokron, etc.)
    - "subterranean": Deep underground areas (Shunning-Grounds, Mohgwyn)
    - "legacy_dungeon": Major story dungeons
    - "minor_dungeon": Caves, catacombs, tunnels, etc.
    - "divine_tower": Divine Tower locations
    - "tutorial": Tutorial/starting area
    - "unknown": Unclassified
    """
    if area_no is None:
        return "unknown"

    # Base game overworld surface
    if area_no == 60:
        return "overworld_surface"
    # DLC overworld surface
    if area_no == 61:
        return "overworld_surface"

    # Underworld - large underground open areas (base game)
    # Siofra River, Ainsel River, Nokron, Nokstella, Lake of Rot, Deeproot Depths
    if area_no == 12:
        return "underworld"

    # Subterranean - deep underground (Shunning-Grounds, Mohgwyn Palace)
    if area_no == 35:
        return "subterranean"

    # Tutorial area
    if area_no == 18:
        return "tutorial"

    # Legacy dungeons (base game)
    # 10=Stormveil, 11=Leyndell, 13=Farum Azula, 14=Raya Lucaria,
    # 15=Haligtree, 16=Volcano Manor, 19=Elden Throne
    if area_no in (10, 11, 13, 14, 15, 16, 19):
        return "legacy_dungeon"

    # Legacy dungeons (DLC)
    # 20=Belurat, 21=Shadow Keep, 22=Stone Coffin Fissure,
    # 25=Finger Birthing Grounds, 28=Manus Metyr
    if area_no in (20, 21, 22, 25, 28):
        return "legacy_dungeon"

    # Divine Towers
    if area_no == 34:
        return "divine_tower"

    # Minor dungeons (base game)
    # 30=Catacombs, 31=Caves, 32=Tunnels, 39=Ruin-Strewn Precipice, 40=Hero's Graves
    if area_no in (30, 31, 32, 39, 40):
        return "minor_dungeon"

    # Minor dungeons (DLC)
    # 40=Catacombs, 41=Gaols, 42=Forges, 43=Caves
    if area_no in (41, 42, 43):
        return "minor_dungeon"

    return "unknown"


def is_base_game_area(area_no: int) -> bool:
    """Check if area_no is from the base game (not DLC)."""
    if area_no is None:
        return True  # Default assumption
    # DLC areas: 20-28 (legacy), 40-43 (minor), 61 (overworld)
    dlc_areas = {20, 21, 22, 25, 28, 40, 41, 42, 43, 61}
    # Note: area 40 is shared (Hero's Graves base + DLC Catacombs)
    # For grid_x 0-10 it's base game, higher is DLC
    return area_no not in dlc_areas


def compute_world_coords(area_no: int, grid_x: int, grid_z: int,
                         pos_x: float, pos_z: float) -> tuple:
    """
    Compute world coordinates from grid and local position.

    Only valid for overworld areas (60, 61) where:
    - grid_x/grid_z represent tile indices on the world map
    - Each tile is 256 units
    - pos_x/pos_z are offsets within the tile

    For dungeons (area_no != 60/61):
    - grid_x identifies the dungeon type/index
    - pos_x/pos_z are local dungeon coordinates
    - World coordinates are NOT meaningful

    Returns (world_x, world_z) or (None, None) for non-overworld areas.
    """
    if not is_overworld_area(area_no):
        return None, None

    if grid_x is None or grid_z is None:
        return None, None
    if pos_x is None or pos_z is None:
        return None, None

    world_x = grid_x * 256.0 + pos_x
    world_z = grid_z * 256.0 + pos_z
    return world_x, world_z


def parse_flag_id_location(flag_id: int) -> Optional[Dict]:
    """Extract location data encoded in flag ID format."""
    if 1_000_000_000 <= flag_id < 2_000_000_000:
        # Base game 10-digit: 1XXYYZZZZ
        tile_index = (flag_id - 1_000_000_000) // 10000
        grid_x = tile_index // 100
        grid_z = tile_index % 100
        return {
            "area_no": 60,  # Overworld
            "grid_x": grid_x,
            "grid_z": grid_z,
            "map_tile": f"m60_{grid_x:02d}_{grid_z:02d}",
            "is_dlc": False
        }
    elif flag_id >= 2_000_000_000:
        # DLC 10-digit: 2XXYYZZZZ
        tile_index = (flag_id - 2_000_000_000) // 10000
        grid_x = tile_index // 100
        grid_z = tile_index % 100
        return {
            "area_no": 61,  # DLC overworld
            "grid_x": grid_x,
            "grid_z": grid_z,
            "map_tile": f"m61_{grid_x:02d}_{grid_z:02d}",
            "is_dlc": True
        }
    elif 10_000_000 <= flag_id < 100_000_000:
        # Dungeon 8-digit: XXYYZZZZ
        map_area = flag_id // 1_000_000
        section = (flag_id // 10_000) % 100
        return {
            "area_no": map_area,
            "grid_x": section,
            "grid_z": 0,
            "map_tile": f"m{map_area}_{section:02d}_00",
            "is_dlc": False
        }
    return None


def load_all_name_lookups() -> Dict[str, Dict[int, str]]:
    """Load all name lookup tables including DLC."""
    lookups = {}

    print("Loading base game names...")
    base_goods = load_name_lookup(MSG_ENGUS / "GoodsName.fmg.xml")
    base_weapons = load_name_lookup(MSG_ENGUS / "WeaponName.fmg.xml")
    base_armor = load_name_lookup(MSG_ENGUS / "ProtectorName.fmg.xml")
    base_accessories = load_name_lookup(MSG_ENGUS / "AccessoryName.fmg.xml")
    base_gems = load_name_lookup(MSG_ENGUS / "GemName.fmg.xml")
    base_magic = load_name_lookup(MSG_ENGUS / "MagicName.fmg.xml")
    base_places = load_name_lookup(MSG_ENGUS / "PlaceName.fmg.xml")
    base_npcs = load_name_lookup(MSG_ENGUS / "NpcName.fmg.xml")

    print("Loading DLC01 names...")
    dlc01_goods = load_name_lookup(MSG_DLC01 / "GoodsName_dlc01.fmg.xml")
    dlc01_weapons = load_name_lookup(MSG_DLC01 / "WeaponName_dlc01.fmg.xml")
    dlc01_armor = load_name_lookup(MSG_DLC01 / "ProtectorName_dlc01.fmg.xml")
    dlc01_accessories = load_name_lookup(MSG_DLC01 / "AccessoryName_dlc01.fmg.xml")
    dlc01_gems = load_name_lookup(MSG_DLC01 / "GemName_dlc01.fmg.xml")
    dlc01_magic = load_name_lookup(MSG_DLC01 / "MagicName_dlc01.fmg.xml")
    dlc01_places = load_name_lookup(MSG_DLC01 / "PlaceName_dlc01.fmg.xml")
    dlc01_npcs = load_name_lookup(MSG_DLC01 / "NpcName_dlc01.fmg.xml")

    # Merge base + DLC
    lookups["goods"] = merge_lookups(base_goods, dlc01_goods)
    lookups["weapons"] = merge_lookups(base_weapons, dlc01_weapons)
    lookups["armor"] = merge_lookups(base_armor, dlc01_armor)
    lookups["accessories"] = merge_lookups(base_accessories, dlc01_accessories)
    lookups["gems"] = merge_lookups(base_gems, dlc01_gems)
    lookups["magic"] = merge_lookups(base_magic, dlc01_magic)
    lookups["places"] = merge_lookups(base_places, dlc01_places)
    lookups["npcs"] = merge_lookups(base_npcs, dlc01_npcs)

    print(f"\nLoaded name lookups:")
    for name, lookup in lookups.items():
        print(f"  {name}: {len(lookup)} entries")

    return lookups

def get_item_name(item_id: int, item_category: int, lookups: Dict) -> str:
    """Get item name from ID and category."""
    if item_category == 1:
        return lookups["goods"].get(item_id, f"Good_{item_id}")
    elif item_category == 2:
        base_id = (item_id // 10000) * 10000
        return lookups["weapons"].get(base_id, lookups["weapons"].get(item_id, f"Weapon_{item_id}"))
    elif item_category == 3:
        base_id = (item_id // 10000) * 10000
        return lookups["armor"].get(base_id, lookups["armor"].get(item_id, f"Armor_{item_id}"))
    elif item_category == 4:
        return lookups["accessories"].get(item_id, f"Accessory_{item_id}")
    elif item_category == 5:
        return lookups["gems"].get(item_id, f"AshOfWar_{item_id}")
    else:
        return f"Item_{item_id}"

def get_region_from_flag(flag_id: int) -> str:
    """Derive region from flag ID format."""
    if flag_id >= 2_000_000_000:
        return "Shadow of the Erdtree"
    elif flag_id >= 1_000_000_000:
        tile_index = (flag_id - 1_000_000_000) // 10000
        tile_x = tile_index // 100
        tile_y = tile_index % 100
        return get_tile_region(tile_x, tile_y)
    elif flag_id >= 10_000_000:
        map_area = flag_id // 1_000_000
        return get_dungeon_region(map_area)
    else:
        return "Various"

def get_tile_region(tile_x: int, tile_y: int) -> str:
    """Get region name from tile coordinates."""
    if 41 <= tile_x <= 44 and 36 <= tile_y <= 39:
        return "Limgrave"
    elif 43 <= tile_x <= 44 and 30 <= tile_y <= 35:
        return "Weeping Peninsula"
    elif 33 <= tile_x <= 40 and 40 <= tile_y <= 50:
        return "Liurnia of the Lakes"
    elif 45 <= tile_x <= 52 and 36 <= tile_y <= 43:
        return "Caelid"
    elif 38 <= tile_x <= 44 and 49 <= tile_y <= 55:
        return "Altus Plateau"
    elif 33 <= tile_x <= 38 and 49 <= tile_y <= 55:
        return "Mt. Gelmir"
    elif 47 <= tile_x <= 54 and 54 <= tile_y <= 58:
        return "Mountaintops of the Giants"
    else:
        return f"World ({tile_x},{tile_y})"

def get_dungeon_region(map_area: int) -> str:
    """Get region from dungeon map area."""
    regions = {
        10: "Stormveil Castle",
        11: "Leyndell",
        12: "Underground",
        13: "Crumbling Farum Azula",
        14: "Academy of Raya Lucaria",
        15: "Caria Manor",
        16: "Volcano Manor",
        18: "Roundtable Hold",
        19: "Chapel of Anticipation",
        20: "Stranded Graveyard",
        21: "Miquella's Haligtree",
        22: "Castle Sol",
        30: "Catacombs",
        31: "Cave",
        32: "Tunnel",
        34: "Divine Tower",
        35: "Mohgwyn Palace",
        39: "Elden Throne",
        40: "Hero's Grave",
        41: "Minor Dungeon",
    }
    return regions.get(map_area, f"Dungeon_{map_area}")

def categorize_flag(flag_id: int, source: str, item_name: str = "") -> str:
    """Categorize flag based on ID range, source, and item name."""
    # Great Runes (possession: 160-167, activation: 180-187)
    if 160 <= flag_id <= 167:
        return "Great Rune Possession"
    elif 180 <= flag_id <= 187:
        return "Great Rune Activation"

    # Boss world drops (171-199)
    elif 171 <= flag_id <= 199:
        return "Boss World Drop"

    # Map Fragments (62010-62099)
    elif 62010 <= flag_id <= 62099:
        return "Map Fragment"

    # Crystal Tears (65000-65399) - NOT Whetblades!
    elif 65000 <= flag_id <= 65399:
        return "Crystal Tear"

    # DLC Crystal Tears (65400-65599)
    elif 65400 <= flag_id <= 65599:
        return "Crystal Tear (DLC)"

    # Actual Whetblades (65610-65720)
    elif 65610 <= flag_id <= 65720:
        return "Whetblade"

    # Ash of War unlocks (65810-65999)
    elif 65810 <= flag_id <= 65999:
        return "Ash of War Unlock"

    # Pot Upgrades (66000-66999)
    elif 66000 <= flag_id <= 66999:
        return "Pot Upgrade"

    # Cookbooks (67000-68999)
    elif 67000 <= flag_id <= 68999:
        return "Cookbook"

    # Remembrances (9100-9199)
    elif 9100 <= flag_id <= 9199:
        return "Remembrance"

    # Talisman Pouches (9200-9299)
    elif 9200 <= flag_id <= 9299:
        return "Talisman Pouch"

    # Mending Runes (9500-9599)
    elif 9500 <= flag_id <= 9599:
        return "Mending Rune"

    # Mausoleum duplication (69000-69999)
    elif 69000 <= flag_id <= 69999:
        return "Mausoleum Duplication"

    # Progression items (60000-60999)
    elif 60000 <= flag_id <= 60999:
        return "Progression"

    # Source-based categories
    if source == "BonfireWarpParam":
        return "Grace"
    elif source == "ShopLineupParam.stock":
        return "Shop Stock"
    elif source == "ShopLineupParam.release":
        return "Shop Unlock"
    elif source == "common.emevd.js":
        return "Event Script"

    # Flag format based (large numbers)
    if flag_id >= 2_000_000_000:
        return "DLC Pickup"
    elif flag_id >= 1_000_000_000:
        return "World Pickup"
    elif flag_id >= 10_000_000:
        return "Dungeon Pickup"

    return "Unknown"

def extract_item_lot_param(lookups: Dict, world_map_points: Dict[int, Dict], msb_positions: Dict[int, Dict]) -> List[EventFlag]:
    """Extract event flags from ItemLotParam_map with spatial data from multiple sources."""
    flags = []
    xml_path = REGULATION_BIN / "ItemLotParam_map.param.xml"

    if not xml_path.exists():
        print(f"Error: {xml_path} not found")
        return flags

    tree = ET.parse(xml_path)
    root = tree.getroot()

    msb_hits = 0  # Track how many positions came from MSB

    for row in root.findall(".//row"):
        row_id = int(row.get("id", 0))
        flag_id = int(row.get("getItemFlagId", 0))
        if flag_id == 0:
            continue

        item_id = int(row.get("lotItemId01", 0))
        item_category = int(row.get("lotItemCategory01", 0))

        if item_id == 0:
            continue

        name = get_item_name(item_id, item_category, lookups)
        category = categorize_flag(flag_id, "ItemLotParam_map", name)
        region = get_region_from_flag(flag_id)

        # Derive location from flag ID format
        location = parse_flag_id_location(flag_id)
        area_no = location["area_no"] if location else None
        grid_x = location["grid_x"] if location else None
        grid_z = location["grid_z"] if location else None
        map_tile = location["map_tile"] if location else None

        # Cross-reference with WorldMapPointParam for exact coordinates (highest priority)
        pos_x, pos_y, pos_z = None, None, None
        position_source = None
        poi_data = world_map_points.get(flag_id)
        if poi_data:
            pos_x = poi_data["pos_x"]
            pos_y = poi_data["pos_y"]
            pos_z = poi_data["pos_z"]
            position_source = "WorldMapPointParam"
            # Use POI's grid data if available (more accurate)
            if poi_data["area_no"] != 0:
                area_no = poi_data["area_no"]
                grid_x = poi_data["grid_x"]
                grid_z = poi_data["grid_z"]
                map_tile = format_map_tile(area_no, grid_x, grid_z)

        # Fallback to MSB treasure positions if no POI data
        if pos_x is None and row_id in msb_positions:
            msb_data = msb_positions[row_id]
            pos_x = msb_data["pos_x"]
            pos_y = msb_data["pos_y"]
            pos_z = msb_data["pos_z"]
            position_source = "MSB"
            msb_hits += 1
            # Use MSB area/grid if not already set from flag ID
            if area_no is None and "area_no" in msb_data:
                area_no = msb_data["area_no"]
                grid_x = msb_data["grid_x"]
                grid_z = msb_data["grid_z"]
                map_tile = format_map_tile(area_no, grid_x, grid_z)

        # Preserve raw data from XML
        raw_data = {
            "lotItemId01": item_id,
            "lotItemCategory01": item_category,
            "lotItemNum01": int(row.get("lotItemNum01", 1)),
        }
        # Add additional item slots if present
        for i in range(2, 9):
            lot_id = int(row.get(f"lotItemId0{i}", 0))
            if lot_id != 0:
                raw_data[f"lotItemId0{i}"] = lot_id
                raw_data[f"lotItemCategory0{i}"] = int(row.get(f"lotItemCategory0{i}", 0))
                raw_data[f"lotItemNum0{i}"] = int(row.get(f"lotItemNum0{i}", 1))

        # Add derived location to raw_data
        if location:
            raw_data["derived_location"] = location

        # Track position source in raw_data
        if position_source:
            raw_data["position_source"] = position_source
            if position_source == "MSB" and row_id in msb_positions:
                raw_data["msb_asset"] = msb_positions[row_id].get("asset_name", "")
                raw_data["msb_dir"] = msb_positions[row_id].get("msb_dir", "")

        # Compute world coordinates (only for overworld areas)
        overworld = is_overworld_area(area_no) if area_no else False
        world_x, world_z = compute_world_coords(area_no, grid_x, grid_z, pos_x, pos_z)

        # Classify area type and DLC status
        area_type = get_area_type(area_no)
        is_dlc = not is_base_game_area(area_no) if area_no else False

        flags.append(EventFlag(
            flag_id=flag_id,
            name=name,
            category=category,
            region=region,
            source_file="ItemLotParam_map.param.xml",
            source_row_id=row_id,
            item_id=item_id,
            item_category=item_category,
            area_no=area_no,
            grid_x=grid_x,
            grid_z=grid_z,
            pos_x=pos_x,
            pos_y=pos_y,
            pos_z=pos_z,
            map_tile=map_tile,
            is_overworld=overworld,
            world_x=world_x,
            world_z=world_z,
            area_type=area_type,
            is_dlc=is_dlc,
            raw_data=raw_data
        ))

    if msb_hits > 0:
        print(f"    MSB positions used: {msb_hits}")

    return flags

def extract_bonfire_warp_param(lookups: Dict) -> List[EventFlag]:
    """Extract event flags from BonfireWarpParam (graces) with full spatial data."""
    flags = []
    xml_path = REGULATION_BIN / "BonfireWarpParam.param.xml"

    if not xml_path.exists():
        print(f"Error: {xml_path} not found")
        return flags

    tree = ET.parse(xml_path)
    root = tree.getroot()

    for row in root.findall(".//row"):
        row_id = int(row.get("id", 0))
        flag_id = int(row.get("eventflagId", 0))
        if flag_id == 0:
            continue

        text_id = int(row.get("textId1", -1))
        name = lookups["places"].get(text_id, f"Grace_{text_id}")

        sub_cat = int(row.get("bonfireSubCategoryId", 0))
        region = lookups["places"].get(sub_cat, get_region_from_flag(flag_id))

        # Extract spatial data directly from XML
        area_no = int(row.get("areaNo", 0))
        grid_x = int(row.get("gridXNo", 0))
        grid_z = int(row.get("gridZNo", 0))
        pos_x = float(row.get("posX", 0))
        pos_y = float(row.get("posY", 0))
        pos_z = float(row.get("posZ", 0))

        # Preserve raw data from XML
        raw_data = {
            "textId1": text_id,
            "bonfireSubCategoryId": sub_cat,
            "areaNo": area_no,
            "gridXNo": grid_x,
            "gridZNo": grid_z,
            "posX": pos_x,
            "posY": pos_y,
            "posZ": pos_z,
        }

        # Compute world coordinates (only for overworld areas)
        overworld = is_overworld_area(area_no)
        world_x, world_z = compute_world_coords(area_no, grid_x, grid_z, pos_x, pos_z)

        # Classify area type and DLC status
        area_type = get_area_type(area_no)
        is_dlc = not is_base_game_area(area_no)

        flags.append(EventFlag(
            flag_id=flag_id,
            name=name,
            category="Grace",
            region=region,
            source_file="BonfireWarpParam.param.xml",
            source_row_id=row_id,
            area_no=area_no,
            grid_x=grid_x,
            grid_z=grid_z,
            pos_x=pos_x,
            pos_y=pos_y,
            pos_z=pos_z,
            map_tile=format_map_tile(area_no, grid_x, grid_z),
            region_id=sub_cat,
            is_overworld=overworld,
            world_x=world_x,
            world_z=world_z,
            area_type=area_type,
            is_dlc=is_dlc,
            raw_data=raw_data
        ))

    return flags

def extract_shop_lineup_param(lookups: Dict) -> List[EventFlag]:
    """Extract event flags from ShopLineupParam."""
    flags = []
    xml_path = REGULATION_BIN / "ShopLineupParam.param.xml"

    if not xml_path.exists():
        print(f"Error: {xml_path} not found")
        return flags

    tree = ET.parse(xml_path)
    root = tree.getroot()

    for row in root.findall(".//row"):
        row_id = int(row.get("id", 0))
        stock_flag = int(row.get("eventFlag_forStock", 0))
        release_flag = int(row.get("eventFlag_forRelease", 0))
        equip_id = int(row.get("equipId", 0))
        equip_type = int(row.get("equipType", 0))
        shop_type = int(row.get("shopType", 0))
        price = int(row.get("value", 0))
        quantity = int(row.get("sellQuantity", -1))

        paramdex_name = row.get("paramdexName", "")
        type_map = {0: 2, 1: 3, 3: 1, 4: 5}  # equipType to item_category

        if paramdex_name:
            name = paramdex_name
        else:
            name = get_item_name(equip_id, type_map.get(equip_type, 1), lookups)

        # Raw data for both stock and release flags
        raw_data = {
            "equipId": equip_id,
            "equipType": equip_type,
            "shopType": shop_type,
            "value": price,
            "sellQuantity": quantity,
            "eventFlag_forStock": stock_flag,
            "eventFlag_forRelease": release_flag,
        }

        if stock_flag != 0:
            flags.append(EventFlag(
                flag_id=stock_flag,
                name=f"{name} - Purchased",
                category=categorize_flag(stock_flag, "ShopLineupParam.stock", name),
                region="Various",
                source_file="ShopLineupParam.param.xml",
                source_row_id=row_id,
                item_id=equip_id,
                item_category=type_map.get(equip_type, 1),
                raw_data=raw_data
            ))

        if release_flag != 0:
            flags.append(EventFlag(
                flag_id=release_flag,
                name=f"{name} - Unlocked",
                category=categorize_flag(release_flag, "ShopLineupParam.release", name),
                region=get_region_from_flag(release_flag),
                source_file="ShopLineupParam.param.xml",
                source_row_id=row_id,
                item_id=equip_id,
                item_category=type_map.get(equip_type, 1),
                raw_data=raw_data
            ))

    return flags

def extract_common_emevd(lookups: Dict) -> List[EventFlag]:
    """Extract event flags from common.emevd.js event scripts."""
    flags = []
    js_path = EVENT_DIR / "common.emevd.js"

    if not js_path.exists():
        print(f"Error: {js_path} not found")
        return flags

    with open(js_path, "r", encoding="utf-8") as f:
        content = f.read()

    # Great Rune possession flags (Event 720)
    # Pattern: Event(720, Default, function (X0_4, X4_4) { ... SetEventFlag(X0_4 + 160, ON); ...
    great_runes = [
        (160, "Godrick's Great Rune"),
        (161, "Radahn's Great Rune"),
        (162, "Morgott's Great Rune"),
        (163, "Rykard's Great Rune"),
        (164, "Mohg's Great Rune"),
        (165, "Malenia's Great Rune"),
        (166, "Miquella's Great Rune"),
        (167, "Placidusax's Old Lord Talisman"),  # DLC related
    ]
    for flag_id, name in great_runes:
        flags.append(EventFlag(
            flag_id=flag_id,
            name=f"{name} - Possessed",
            category="Great Rune Possession",
            region="Various",
            source_file="common.emevd.js",
            raw_data={"event_id": 720, "description": "Set when Great Rune is obtained"}
        ))

    # Great Rune activation flags (Event 730)
    great_rune_activation = [
        (180, "Godrick's Great Rune"),
        (181, "Radahn's Great Rune"),
        (182, "Morgott's Great Rune"),
        (183, "Rykard's Great Rune"),
        (184, "Mohg's Great Rune"),
        (185, "Malenia's Great Rune"),
        (186, "Miquella's Great Rune"),
        (187, "Unknown Great Rune"),
    ]
    for flag_id, name in great_rune_activation:
        flags.append(EventFlag(
            flag_id=flag_id,
            name=f"{name} - Activated",
            category="Great Rune Activation",
            region="Divine Tower",
            source_file="common.emevd.js",
            raw_data={"event_id": 730, "description": "Set when Great Rune is activated at Divine Tower"}
        ))

    # Boss Remembrances (Event 1100 pattern)
    # Pattern includes 91xx flags for remembrance possession
    remembrances = [
        (9100, "Remembrance of the Grafted"),
        (9101, "Remembrance of the Starscourge"),
        (9102, "Omen King's Remembrance"),
        (9103, "Remembrance of the Blasphemous"),
        (9104, "Remembrance of the Blood Lord"),
        (9105, "Remembrance of the Rot Goddess"),
        (9106, "Elden Remembrance"),
        (9107, "Remembrance of the Lichdragon"),
        (9108, "Remembrance of the Naturalborn"),
        (9109, "Remembrance of the Regal Ancestor"),
        (9110, "Remembrance of the Full Moon Queen"),
        (9111, "Remembrance of the Dragonlord"),
        (9112, "Remembrance of the Fire Giant"),
        (9113, "Remembrance of Hoarah Loux"),
        (9114, "Remembrance of the Black Blade"),
    ]
    for flag_id, name in remembrances:
        flags.append(EventFlag(
            flag_id=flag_id,
            name=name,
            category="Remembrance",
            region="Various",
            source_file="common.emevd.js",
            raw_data={"event_id": 1100, "description": "Set when boss remembrance is obtained"}
        ))

    # Talisman Pouch upgrades (Event 1200 - 92xx flags)
    talisman_upgrades = [
        (9200, "First Talisman Pouch"),
        (9201, "Second Talisman Pouch"),
        (9202, "Third Talisman Pouch"),
    ]
    for flag_id, name in talisman_upgrades:
        flags.append(EventFlag(
            flag_id=flag_id,
            name=name,
            category="Talisman Pouch",
            region="Various",
            source_file="common.emevd.js",
            raw_data={"event_id": 1200, "description": "Set when talisman pouch obtained"}
        ))

    # Mending Runes (endings)
    mending_runes = [
        (9500, "Mending Rune of the Fell Curse"),
        (9501, "Mending Rune of Perfect Order"),
        (9502, "Mending Rune of the Death-Prince"),
    ]
    for flag_id, name in mending_runes:
        flags.append(EventFlag(
            flag_id=flag_id,
            name=name,
            category="Mending Rune",
            region="Various",
            source_file="common.emevd.js",
            raw_data={"description": "Quest item for alternate ending"}
        ))

    return flags


def extract_world_map_points(lookups: Dict, world_map_points: Dict[int, Dict]) -> List[EventFlag]:
    """Extract event flags from WorldMapPointParam (POI discovery flags)."""
    flags = []

    # Icon ID to category mapping
    icon_categories = {
        83: "Grace",  # Sites of Grace icon
        # Other icon types can be added as discovered
    }

    for flag_id, poi in world_map_points.items():
        icon_id = poi["icon_id"]

        # Get name from places lookup or POI data
        name = lookups["places"].get(poi["text_id"], poi["name"]) or f"POI_{flag_id}"

        # Categorize based on icon
        if icon_id == 83:
            category = "Grace"
        else:
            category = "Map POI"

        # Derive region from grid
        region = get_region_from_flag(flag_id) if flag_id >= 10_000_000 else "Various"

        raw_data = {
            "row_id": poi["row_id"],
            "iconId": icon_id,
            "textId1": poi["text_id"],
            "areaNo": poi["area_no"],
            "gridXNo": poi["grid_x"],
            "gridZNo": poi["grid_z"],
            "posX": poi["pos_x"],
            "posY": poi["pos_y"],
            "posZ": poi["pos_z"],
        }

        # Compute world coordinates (only for overworld areas)
        area_no = poi["area_no"]
        overworld = is_overworld_area(area_no)
        world_x, world_z = compute_world_coords(
            area_no, poi["grid_x"], poi["grid_z"], poi["pos_x"], poi["pos_z"]
        )

        # Classify area type and DLC status
        area_type = get_area_type(area_no)
        is_dlc = not is_base_game_area(area_no)

        flags.append(EventFlag(
            flag_id=flag_id,
            name=name,
            category=category,
            region=region,
            source_file="WorldMapPointParam.param.xml",
            source_row_id=poi["row_id"],
            area_no=area_no,
            grid_x=poi["grid_x"],
            grid_z=poi["grid_z"],
            pos_x=poi["pos_x"],
            pos_y=poi["pos_y"],
            pos_z=poi["pos_z"],
            map_tile=format_map_tile(area_no, poi["grid_x"], poi["grid_z"]),
            is_overworld=overworld,
            world_x=world_x,
            world_z=world_z,
            area_type=area_type,
            is_dlc=is_dlc,
            raw_data=raw_data
        ))

    return flags


def format_output_markdown(flags: List[EventFlag]) -> str:
    """Format flags as proper markdown table with spatial data."""
    flags.sort(key=lambda f: f.flag_id)

    seen = set()
    unique_flags = []
    for f in flags:
        if f.flag_id not in seen:
            seen.add(f.flag_id)
            unique_flags.append(f)

    lines = []
    lines.append("# Extracted Event Flags")
    lines.append("")
    lines.append(f"Total unique flags: {len(unique_flags)}")
    lines.append("")
    lines.append("| Flag ID | Name | Category | Region | Map Tile | Local Pos (X,Y,Z) | World (X,Z) | Source |")
    lines.append("|---------|------|----------|--------|----------|-------------------|-------------|--------|")

    for f in unique_flags:
        # No truncation - full names preserved
        # Escape pipe characters in names
        name = f.name.replace("|", "\\|")
        region = f.region.replace("|", "\\|")
        source = f.source_file.replace(".param.xml", "").replace(".emevd.js", "")

        # Format map tile
        map_tile = f.map_tile or "-"

        # Format local coordinates
        if f.pos_x is not None and f.pos_y is not None and f.pos_z is not None:
            local_coords = f"{f.pos_x:.1f}, {f.pos_y:.1f}, {f.pos_z:.1f}"
        else:
            local_coords = "-"

        # Format world coordinates (only for overworld)
        if f.world_x is not None and f.world_z is not None:
            world_coords = f"{f.world_x:.1f}, {f.world_z:.1f}"
        else:
            world_coords = "-"

        lines.append(f"| {f.flag_id} | {name} | {f.category} | {region} | {map_tile} | {local_coords} | {world_coords} | {source} |")

    return "\n".join(lines)


def get_category_summary(flags: List[EventFlag]) -> Dict[str, int]:
    """Get count of flags per category."""
    seen = set()
    counts = {}
    for f in flags:
        if f.flag_id not in seen:
            seen.add(f.flag_id)
            counts[f.category] = counts.get(f.category, 0) + 1
    return dict(sorted(counts.items(), key=lambda x: -x[1]))

def main():
    print("=" * 80)
    print("Elden Ring Event Flag Extractor")
    print("=" * 80)

    print("\nLoading name lookups...")
    lookups = load_all_name_lookups()

    print("\nLoading spatial reference data...")
    world_map_points = load_world_map_points()
    print(f"  WorldMapPointParam: {len(world_map_points)} POIs with coordinates")

    world_map_pieces = load_world_map_pieces()
    print(f"  WorldMapPieceParam: {len(world_map_pieces)} map regions")

    print("\nLoading MSB treasure positions...")
    msb_positions = load_msb_treasure_positions()

    print("\n" + "-" * 40)
    print("Extracting from game param files...")
    print("-" * 40)

    print("\nExtracting from ItemLotParam_map...")
    item_lot_flags = extract_item_lot_param(lookups, world_map_points, msb_positions)
    print(f"  Found {len(item_lot_flags)} flags")

    print("\nExtracting from BonfireWarpParam...")
    bonfire_flags = extract_bonfire_warp_param(lookups)
    print(f"  Found {len(bonfire_flags)} flags")

    print("\nExtracting from ShopLineupParam...")
    shop_flags = extract_shop_lineup_param(lookups)
    print(f"  Found {len(shop_flags)} flags")

    print("\nExtracting from common.emevd.js...")
    emevd_flags = extract_common_emevd(lookups)
    print(f"  Found {len(emevd_flags)} flags")

    print("\nExtracting from WorldMapPointParam...")
    poi_flags = extract_world_map_points(lookups, world_map_points)
    print(f"  Found {len(poi_flags)} flags")

    all_flags = item_lot_flags + bonfire_flags + shop_flags + emevd_flags + poi_flags

    print(f"\n{'=' * 40}")
    print(f"Total flags extracted: {len(all_flags)}")

    # Deduplicate and count
    seen = set()
    unique_flags = []
    for f in all_flags:
        if f.flag_id not in seen:
            seen.add(f.flag_id)
            unique_flags.append(f)

    print(f"Unique flags: {len(unique_flags)}")

    # Category summary
    print(f"\n{'=' * 40}")
    print("Category Summary:")
    print("-" * 40)
    category_counts = get_category_summary(all_flags)
    for cat, count in category_counts.items():
        print(f"  {cat}: {count}")

    # Spatial coverage statistics
    print(f"\n{'=' * 40}")
    print("Spatial Data Coverage:")
    print("-" * 40)
    with_local_coords = sum(1 for f in unique_flags if f.pos_x is not None)
    with_world_coords = sum(1 for f in unique_flags if f.world_x is not None)
    with_map_tile = sum(1 for f in unique_flags if f.map_tile is not None)
    from_poi = sum(1 for f in unique_flags if f.raw_data.get("position_source") == "WorldMapPointParam")
    from_msb = sum(1 for f in unique_flags if f.raw_data.get("position_source") == "MSB")
    from_grace = sum(1 for f in unique_flags if f.source_file == "BonfireWarpParam.param.xml" and f.pos_x is not None)

    print(f"  Flags with local coords: {with_local_coords}/{len(unique_flags)} ({100*with_local_coords//len(unique_flags)}%)")
    print(f"  Flags with world coords: {with_world_coords}/{len(unique_flags)} ({100*with_world_coords//len(unique_flags)}%)")
    print(f"  Flags with map tile: {with_map_tile}/{len(unique_flags)} ({100*with_map_tile//len(unique_flags)}%)")
    print(f"  Coordinates from BonfireWarpParam: {from_grace}")
    print(f"  Coordinates from WorldMapPointParam: {from_poi}")
    print(f"  Coordinates from MSB files: {from_msb}")

    # Area type breakdown
    print(f"\n{'=' * 40}")
    print("Area Type Breakdown:")
    print("-" * 40)
    area_type_counts = {}
    dlc_count = 0
    base_count = 0
    for f in unique_flags:
        area_type_counts[f.area_type] = area_type_counts.get(f.area_type, 0) + 1
        if f.is_dlc:
            dlc_count += 1
        else:
            base_count += 1

    for area_type, count in sorted(area_type_counts.items(), key=lambda x: -x[1]):
        print(f"  {area_type}: {count}")
    print(f"  ---")
    print(f"  Base game: {base_count}")
    print(f"  DLC: {dlc_count}")

    # Output directory
    output_dir = Path(__file__).parent

    # Write markdown output (no truncation)
    md_output = format_output_markdown(all_flags)
    md_path = output_dir / "extracted_event_flags.md"
    with open(md_path, "w", encoding="utf-8") as f:
        f.write(md_output)
    print(f"\nMarkdown output: {md_path}")

    # Write JSON output with full data preservation
    json_data = {
        "metadata": {
            "extraction_date": str(Path(__file__).stat().st_mtime),
            "total_flags": len(unique_flags),
            "sources": [
                "ItemLotParam_map.param.xml",
                "BonfireWarpParam.param.xml",
                "ShopLineupParam.param.xml",
                "common.emevd.js",
                "WorldMapPointParam.param.xml",
                "MSB files (map/mapstudio/m*-msb-dcx/)"
            ],
            "category_counts": category_counts
        },
        "flags": [asdict(f) for f in sorted(unique_flags, key=lambda x: x.flag_id)]
    }
    json_path = output_dir / "extracted_event_flags.json"
    with open(json_path, "w", encoding="utf-8") as f:
        json.dump(json_data, f, indent=2, ensure_ascii=False)
    print(f"JSON output: {json_path}")

    # Print sample
    print(f"\n{'=' * 80}")
    print("SAMPLE OUTPUT (first 30 entries):")
    print("=" * 80)
    sample_lines = md_output.split("\n")[:36]  # Header + 30 rows
    for line in sample_lines:
        print(line)

    print(f"\n{'=' * 80}")
    print(f"Done! Check {output_dir} for output files.")


if __name__ == "__main__":
    main()
