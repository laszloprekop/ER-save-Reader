#!/usr/bin/env python3
"""
Database Generator for ER-save-Editor
======================================

Parses decompiled game param XML files and generates Rust database files
with static HashMaps.

Data Sources:
- PRIMARY: Decompiled game files (regulation-bin/*.param.xml, msg/engus/*.fmg.xml)
- ENRICHMENT: MapGenie data from elden-map (enriched-comprehensive-pois.json, event-flag-to-mapgenie.json)

Usage:
    python scripts/generate_db.py

Generated files:
    src/db/graces_data.rs      - Sites of grace with event flags
    src/db/unified_items.rs    - Combined items from all EquipParam files
    src/db/merchants_data.rs   - Shop inventory from ShopLineupParam
    src/db/bosses_data.rs      - Boss data with defeat flags
"""

import os
import json
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Dict, List, Optional, Tuple, Any
from dataclasses import dataclass, field


# ============================================================================
# Configuration
# ============================================================================

# Paths
GAME_FILES_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files")
ELDEN_MAP_DATA_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data")
OUTPUT_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/src/db")

# Game param files
BONFIRE_WARP_PARAM = GAME_FILES_DIR / "regulation-bin" / "BonfireWarpParam.param.xml"
EQUIP_PARAM_GOODS = GAME_FILES_DIR / "regulation-bin" / "EquipParamGoods.param.xml"
EQUIP_PARAM_WEAPON = GAME_FILES_DIR / "regulation-bin" / "EquipParamWeapon.param.xml"
EQUIP_PARAM_PROTECTOR = GAME_FILES_DIR / "regulation-bin" / "EquipParamProtector.param.xml"
EQUIP_PARAM_ACCESSORY = GAME_FILES_DIR / "regulation-bin" / "EquipParamAccessory.param.xml"
SHOP_LINEUP_PARAM = GAME_FILES_DIR / "regulation-bin" / "ShopLineupParam.param.xml"
NPC_PARAM = GAME_FILES_DIR / "regulation-bin" / "NpcParam.param.xml"

# Message files for names
PLACE_NAME_FMG = GAME_FILES_DIR / "msg/engus/item-msgbnd-dcx/PlaceName.fmg.xml"
GOODS_NAME_FMG = GAME_FILES_DIR / "msg/engus/item-msgbnd-dcx/GoodsName.fmg.xml"
WEAPON_NAME_FMG = GAME_FILES_DIR / "msg/engus/item-msgbnd-dcx/WeaponName.fmg.xml"
PROTECTOR_NAME_FMG = GAME_FILES_DIR / "msg/engus/item-msgbnd-dcx/ProtectorName.fmg.xml"
ACCESSORY_NAME_FMG = GAME_FILES_DIR / "msg/engus/item-msgbnd-dcx/AccessoryName.fmg.xml"
NPC_NAME_FMG = GAME_FILES_DIR / "msg/engus/item-msgbnd-dcx/NpcName.fmg.xml"

# MapGenie enrichment files
ENRICHED_POIS = ELDEN_MAP_DATA_DIR / "game-pois/enriched-comprehensive-pois.json"
EVENT_FLAG_TO_MAPGENIE = ELDEN_MAP_DATA_DIR / "event-flag-to-mapgenie.json"


# ============================================================================
# Data Classes
# ============================================================================

@dataclass
class GraceData:
    """Site of Grace from BonfireWarpParam"""
    id: int
    event_flag_id: int
    name: str
    paramdex_name: str
    area_no: int
    grid_x: int
    grid_z: int
    pos_x: float
    pos_y: float
    pos_z: float
    text_id: int
    # Enrichment
    mapgenie_id: Optional[str] = None
    mapgenie_title: Optional[str] = None
    region: str = "Unknown"


@dataclass
class UnifiedItem:
    """Combined item from any EquipParam file"""
    id: int
    name: str
    category: str  # Weapon, Armor, Accessory, Good
    icon_id: int
    weight: float
    sell_value: int
    max_hold: int
    # Enrichment
    mapgenie_category: Optional[str] = None


@dataclass
class MerchantItem:
    """Shop item from ShopLineupParam"""
    shop_id: int
    merchant_name: str
    item_id: int
    item_name: str
    price: int
    quantity: int  # -1 = infinite
    equip_type: int  # 0=Goods, 1=Weapon, 2=Protector, 3=Accessory
    event_flag_stock: int
    event_flag_release: int


@dataclass
class BossData:
    """Boss data from NpcParam + event scripts"""
    id: int
    name: str
    hp: int
    souls: int  # getSoul
    defeat_flag: int
    region: str
    # Enrichment
    mapgenie_id: Optional[str] = None
    boss_type: Optional[str] = None


# ============================================================================
# XML Parsing Utilities
# ============================================================================

def parse_fmg_xml(path: Path) -> Dict[int, str]:
    """Parse FMG XML file to get id -> text mapping"""
    names = {}
    if not path.exists():
        print(f"Warning: FMG file not found: {path}")
        return names

    tree = ET.parse(path)
    root = tree.getroot()

    for entry in root.findall(".//text"):
        id_str = entry.get("id")
        if id_str:
            text = entry.text or ""
            if text != "%null%" and text != "[ERROR]":
                names[int(id_str)] = text

    return names


def parse_param_xml(path: Path) -> List[Dict[str, Any]]:
    """Parse param XML file to get list of row dictionaries"""
    rows = []
    if not path.exists():
        print(f"Warning: Param file not found: {path}")
        return rows

    tree = ET.parse(path)
    root = tree.getroot()

    for row in root.findall(".//row"):
        row_dict = dict(row.attrib)
        rows.append(row_dict)

    return rows


def get_row_attr(row: Dict, key: str, default=None, cast_type=str):
    """Get attribute from row with type casting and default"""
    val = row.get(key, default)
    if val is None:
        return default
    try:
        return cast_type(val)
    except (ValueError, TypeError):
        return default


# ============================================================================
# Enrichment Loading
# ============================================================================

def load_enrichment_data() -> Tuple[Dict[int, Dict], Dict[int, Dict]]:
    """Load MapGenie enrichment data"""
    pois_by_flag = {}
    mapgenie_by_flag = {}

    # Load enriched POIs
    if ENRICHED_POIS.exists():
        with open(ENRICHED_POIS, 'r') as f:
            data = json.load(f)
            for poi in data.get("pois", []):
                event_flag = poi.get("eventFlag")
                if event_flag:
                    pois_by_flag[event_flag] = poi

    # Load event flag to MapGenie mappings
    if EVENT_FLAG_TO_MAPGENIE.exists():
        with open(EVENT_FLAG_TO_MAPGENIE, 'r') as f:
            data = json.load(f)
            for flag_str, mapping in data.get("mappings", {}).items():
                try:
                    mapgenie_by_flag[int(flag_str)] = mapping
                except ValueError:
                    pass

    return pois_by_flag, mapgenie_by_flag


# ============================================================================
# Region Mapping
# ============================================================================

AREA_TO_REGION = {
    10: "Stormveil Castle",
    11: "Leyndell",
    12: "Underground",
    13: "Crumbling Farum Azula",
    14: "Academy of Raya Lucaria",
    15: "Haligtree",
    16: "Volcano Manor",
    18: "Stranded Graveyard",
    19: "Stone Platform",
    20: "Belurat",
    21: "Enir-Ilim",
    22: "Specimen Storehouse",
    25: "Scadutree",
    60: "Limgrave",
    61: "Liurnia",
    62: "Altus Plateau",
    63: "Mountaintops",
    64: "Caelid",
}

def get_region_from_area(area_no: int) -> str:
    """Get region name from area number"""
    return AREA_TO_REGION.get(area_no, "Unknown")


def extract_region_from_name(name: str) -> str:
    """Extract region from paramdex name like '[Stormveil Castle] Godrick'"""
    if name.startswith("[") and "]" in name:
        return name[1:name.index("]")]
    return "Unknown"


# ============================================================================
# Generator: Graces
# ============================================================================

def generate_graces() -> List[GraceData]:
    """Generate grace data from BonfireWarpParam + enrichment"""
    print("Generating graces...")

    # Load names
    place_names = parse_fmg_xml(PLACE_NAME_FMG)

    # Load enrichment
    pois_by_flag, mapgenie_by_flag = load_enrichment_data()

    # Parse param
    rows = parse_param_xml(BONFIRE_WARP_PARAM)

    graces = []
    for row in rows:
        row_id = get_row_attr(row, "id", 0, int)
        if row_id == 0:
            continue  # Skip invalid entries

        event_flag_id = get_row_attr(row, "eventflagId", 0, int)
        paramdex_name = get_row_attr(row, "paramdexName", "", str)
        text_id = get_row_attr(row, "textId1", 0, int)
        area_no = get_row_attr(row, "areaNo", 0, int)

        # Get name from text ID or paramdex
        name = place_names.get(text_id, "")
        if not name and paramdex_name:
            # Extract name from paramdex (remove [Region] prefix)
            if "]" in paramdex_name:
                name = paramdex_name[paramdex_name.index("]")+1:].strip()
            else:
                name = paramdex_name

        # Get region
        region = extract_region_from_name(paramdex_name) if paramdex_name else get_region_from_area(area_no)

        grace = GraceData(
            id=row_id,
            event_flag_id=event_flag_id,
            name=name,
            paramdex_name=paramdex_name,
            area_no=area_no,
            grid_x=get_row_attr(row, "gridXNo", 0, int),
            grid_z=get_row_attr(row, "gridZNo", 0, int),
            pos_x=get_row_attr(row, "posX", 0.0, float),
            pos_y=get_row_attr(row, "posY", 0.0, float),
            pos_z=get_row_attr(row, "posZ", 0.0, float),
            text_id=text_id,
            region=region,
        )

        # Apply enrichment
        if event_flag_id in pois_by_flag:
            poi = pois_by_flag[event_flag_id]
            grace.mapgenie_id = poi.get("mapgenieId")
            grace.mapgenie_title = poi.get("mapgenieTitle")
        elif event_flag_id in mapgenie_by_flag:
            mapping = mapgenie_by_flag[event_flag_id]
            grace.mapgenie_id = mapping.get("location_id")
            grace.mapgenie_title = mapping.get("name")

        graces.append(grace)

    print(f"  Generated {len(graces)} graces")
    return graces


# ============================================================================
# Generator: Unified Items
# ============================================================================

def generate_unified_items() -> List[UnifiedItem]:
    """Generate unified items from all EquipParam files"""
    print("Generating unified items...")

    # Load names
    goods_names = parse_fmg_xml(GOODS_NAME_FMG)
    weapon_names = parse_fmg_xml(WEAPON_NAME_FMG)
    protector_names = parse_fmg_xml(PROTECTOR_NAME_FMG)
    accessory_names = parse_fmg_xml(ACCESSORY_NAME_FMG)

    items = []

    # Goods
    for row in parse_param_xml(EQUIP_PARAM_GOODS):
        row_id = get_row_attr(row, "id", 0, int)
        name = goods_names.get(row_id, f"Unknown Good {row_id}")

        items.append(UnifiedItem(
            id=row_id,
            name=name,
            category="Good",
            icon_id=get_row_attr(row, "iconId", 0, int),
            weight=get_row_attr(row, "weight", 0.0, float),
            sell_value=get_row_attr(row, "sellValue", 0, int),
            max_hold=get_row_attr(row, "maxNum", 1, int),
        ))

    # Weapons
    for row in parse_param_xml(EQUIP_PARAM_WEAPON):
        row_id = get_row_attr(row, "id", 0, int)
        name = weapon_names.get(row_id, f"Unknown Weapon {row_id}")

        items.append(UnifiedItem(
            id=row_id,
            name=name,
            category="Weapon",
            icon_id=get_row_attr(row, "iconId", 0, int),
            weight=get_row_attr(row, "weight", 0.0, float),
            sell_value=get_row_attr(row, "sellValue", 0, int),
            max_hold=get_row_attr(row, "maxNum", 1, int),
        ))

    # Armor (Protector)
    for row in parse_param_xml(EQUIP_PARAM_PROTECTOR):
        row_id = get_row_attr(row, "id", 0, int)
        name = protector_names.get(row_id, f"Unknown Armor {row_id}")

        items.append(UnifiedItem(
            id=row_id,
            name=name,
            category="Armor",
            icon_id=get_row_attr(row, "iconId", 0, int),
            weight=get_row_attr(row, "weight", 0.0, float),
            sell_value=get_row_attr(row, "sellValue", 0, int),
            max_hold=1,
        ))

    # Accessories (Talismans)
    for row in parse_param_xml(EQUIP_PARAM_ACCESSORY):
        row_id = get_row_attr(row, "id", 0, int)
        name = accessory_names.get(row_id, f"Unknown Accessory {row_id}")

        items.append(UnifiedItem(
            id=row_id,
            name=name,
            category="Accessory",
            icon_id=get_row_attr(row, "iconId", 0, int),
            weight=get_row_attr(row, "weight", 0.0, float),
            sell_value=get_row_attr(row, "sellValue", 0, int),
            max_hold=1,
        ))

    print(f"  Generated {len(items)} items")
    return items


# ============================================================================
# Generator: Merchants
# ============================================================================

def generate_merchants() -> List[MerchantItem]:
    """Generate merchant data from ShopLineupParam"""
    print("Generating merchants...")

    # Load item names
    goods_names = parse_fmg_xml(GOODS_NAME_FMG)
    weapon_names = parse_fmg_xml(WEAPON_NAME_FMG)
    protector_names = parse_fmg_xml(PROTECTOR_NAME_FMG)
    accessory_names = parse_fmg_xml(ACCESSORY_NAME_FMG)

    merchants = []

    for row in parse_param_xml(SHOP_LINEUP_PARAM):
        shop_id = get_row_attr(row, "id", 0, int)
        equip_type = get_row_attr(row, "equipType", 0, int)
        equip_id = get_row_attr(row, "equipId", 0, int)

        # Get item name based on type
        if equip_type == 0:  # Goods
            item_name = goods_names.get(equip_id, f"Unknown Good {equip_id}")
        elif equip_type == 1:  # Weapon
            item_name = weapon_names.get(equip_id, f"Unknown Weapon {equip_id}")
        elif equip_type == 2:  # Protector
            item_name = protector_names.get(equip_id, f"Unknown Armor {equip_id}")
        elif equip_type == 3:  # Accessory
            item_name = accessory_names.get(equip_id, f"Unknown Accessory {equip_id}")
        else:
            item_name = f"Unknown Item {equip_id}"

        # Extract merchant name from paramdexName
        paramdex_name = get_row_attr(row, "paramdexName", "", str)
        merchant_name = "Unknown Merchant"
        if paramdex_name:
            # Format is usually "[Merchant Name] Item Name"
            if paramdex_name.startswith("[") and "]" in paramdex_name:
                merchant_name = paramdex_name[1:paramdex_name.index("]")]

        price = get_row_attr(row, "value", 0, int)
        if price < 0:
            price = 0  # Treat negative prices as 0 (free)

        merchants.append(MerchantItem(
            shop_id=shop_id,
            merchant_name=merchant_name,
            item_id=equip_id,
            item_name=item_name,
            price=price,
            quantity=get_row_attr(row, "sellQuantity", -1, int),
            equip_type=equip_type,
            event_flag_stock=get_row_attr(row, "eventFlag_forStock", 0, int),
            event_flag_release=get_row_attr(row, "eventFlag_forRelease", 0, int),
        ))

    print(f"  Generated {len(merchants)} merchant items")
    return merchants


# ============================================================================
# Generator: Bosses
# ============================================================================

# Known boss defeat flags from event scripts
BOSS_DEFEAT_FLAGS = {
    # Shardbearers
    10000800: ("Godrick the Grafted", "Stormveil Castle", "demigod"),
    14000800: ("Rennala, Queen of the Full Moon", "Academy of Raya Lucaria", "demigod"),
    16000800: ("Rykard, Lord of Blasphemy", "Volcano Manor", "demigod"),
    12010800: ("Starscourge Radahn", "Caelid", "demigod"),
    15000800: ("Malenia, Blade of Miquella", "Haligtree", "demigod"),
    12050800: ("Mohg, Lord of Blood", "Mohgwyn Palace", "demigod"),
    11000800: ("Morgott, the Omen King", "Leyndell", "demigod"),
    13000800: ("Maliketh, the Black Blade", "Crumbling Farum Azula", "great_boss"),

    # Major bosses
    19000800: ("Radagon / Elden Beast", "Stone Platform", "great_boss"),
    16000850: ("God-Devouring Serpent", "Volcano Manor", "great_boss"),
    11050800: ("Hoarah Loux, Warrior", "Leyndell", "great_boss"),

    # Field bosses and minibosses
    1035420800: ("Margit, the Fell Omen", "Stormhill", "boss"),
    1042380800: ("Tree Sentinel", "Limgrave", "boss"),
    1044350800: ("Flying Dragon Agheel", "Limgrave", "boss"),
    1034500800: ("Tibia Mariner", "Limgrave", "boss"),
    1035500800: ("Bell Bearing Hunter", "Limgrave", "boss"),
    1036540800: ("Night's Cavalry", "Limgrave", "boss"),
}


def generate_bosses() -> List[BossData]:
    """Generate boss data from known defeat flags + NpcParam"""
    print("Generating bosses...")

    # Load enrichment
    pois_by_flag, mapgenie_by_flag = load_enrichment_data()

    bosses = []

    for defeat_flag, (name, region, boss_type) in BOSS_DEFEAT_FLAGS.items():
        boss = BossData(
            id=defeat_flag,
            name=name,
            hp=0,  # Would need NpcParam correlation
            souls=0,
            defeat_flag=defeat_flag,
            region=region,
            boss_type=boss_type,
        )

        # Apply enrichment
        if defeat_flag in mapgenie_by_flag:
            mapping = mapgenie_by_flag[defeat_flag]
            boss.mapgenie_id = mapping.get("location_id")

        bosses.append(boss)

    print(f"  Generated {len(bosses)} bosses")
    return bosses


# ============================================================================
# Rust Code Generation
# ============================================================================

def escape_rust_string(s: str) -> str:
    """Escape string for Rust string literal"""
    return s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")


def option_str(val: Optional[str]) -> str:
    """Convert optional string to Rust Option literal"""
    if val is None:
        return "None"
    return f'Some("{escape_rust_string(val)}")'


def write_graces_rust(graces: List[GraceData]):
    """Write graces_data.rs"""
    path = OUTPUT_DIR / "graces_data.rs"

    with open(path, 'w') as f:
        f.write("""//! Sites of Grace database generated from BonfireWarpParam.param.xml
//! This file is auto-generated by scripts/generate_db.py - do not edit manually

use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GraceData {
    pub id: u32,
    pub event_flag_id: u32,
    pub name: &'static str,
    pub region: &'static str,
    pub area_no: u8,
    pub grid_x: u8,
    pub grid_z: u8,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    // Enrichment (from MapGenie)
    pub mapgenie_id: Option<&'static str>,
}

""")

        f.write(f"/// Sites of Grace database ({len(graces)} entries)\n")
        f.write("pub static GRACES_DATA: Lazy<HashMap<u32, GraceData>> = Lazy::new(|| {\n")
        f.write("    let mut m = HashMap::new();\n")

        for grace in graces:
            f.write(f'    m.insert({grace.event_flag_id}, GraceData {{ ')
            f.write(f'id: {grace.id}, ')
            f.write(f'event_flag_id: {grace.event_flag_id}, ')
            f.write(f'name: "{escape_rust_string(grace.name)}", ')
            f.write(f'region: "{escape_rust_string(grace.region)}", ')
            f.write(f'area_no: {grace.area_no}, ')
            f.write(f'grid_x: {grace.grid_x}, ')
            f.write(f'grid_z: {grace.grid_z}, ')
            f.write(f'pos_x: {grace.pos_x:.2f}, ')
            f.write(f'pos_y: {grace.pos_y:.2f}, ')
            f.write(f'pos_z: {grace.pos_z:.2f}, ')
            f.write(f'mapgenie_id: {option_str(grace.mapgenie_id)}, ')
            f.write("});\n")

        f.write("    m\n")
        f.write("});\n\n")

        # Add helper to get all regions
        regions = sorted(set(g.region for g in graces if g.region != "Unknown"))
        f.write("/// All unique regions with graces\n")
        f.write("pub static GRACE_REGIONS: &[&str] = &[\n")
        for region in regions:
            f.write(f'    "{escape_rust_string(region)}",\n')
        f.write("];\n")

    print(f"  Wrote {path}")


def write_unified_items_rust(items: List[UnifiedItem]):
    """Write unified_items.rs"""
    path = OUTPUT_DIR / "unified_items.rs"

    with open(path, 'w') as f:
        f.write("""//! Unified Items database generated from EquipParam files
//! This file is auto-generated by scripts/generate_db.py - do not edit manually

use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnifiedItemCategory {
    Weapon,
    Armor,
    Accessory,
    Good,
}

impl UnifiedItemCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Weapon => "Weapon",
            Self::Armor => "Armor",
            Self::Accessory => "Accessory",
            Self::Good => "Good",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnifiedItem {
    pub id: u32,
    pub name: &'static str,
    pub category: UnifiedItemCategory,
    pub icon_id: u16,
    pub weight: f32,
    pub sell_value: i32,
    pub max_hold: u16,
    // Enrichment
    pub mapgenie_category: Option<&'static str>,
}

""")

        f.write(f"/// Unified items database ({len(items)} entries)\n")
        f.write("pub static UNIFIED_ITEMS: Lazy<HashMap<(UnifiedItemCategory, u32), UnifiedItem>> = Lazy::new(|| {\n")
        f.write("    let mut m = HashMap::new();\n")

        for item in items:
            cat = f"UnifiedItemCategory::{item.category}"
            f.write(f'    m.insert(({cat}, {item.id}), UnifiedItem {{ ')
            f.write(f'id: {item.id}, ')
            f.write(f'name: "{escape_rust_string(item.name)}", ')
            f.write(f'category: {cat}, ')
            f.write(f'icon_id: {item.icon_id}, ')
            f.write(f'weight: {item.weight:.1f}, ')
            f.write(f'sell_value: {item.sell_value}, ')
            f.write(f'max_hold: {item.max_hold}, ')
            f.write(f'mapgenie_category: {option_str(item.mapgenie_category)}, ')
            f.write("});\n")

        f.write("    m\n")
        f.write("});\n")

    print(f"  Wrote {path}")


def write_merchants_rust(merchants: List[MerchantItem]):
    """Write merchants_data.rs"""
    path = OUTPUT_DIR / "merchants_data.rs"

    with open(path, 'w') as f:
        f.write("""//! Merchants database generated from ShopLineupParam.param.xml
//! This file is auto-generated by scripts/generate_db.py - do not edit manually

use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShopEquipType {
    Good,
    Weapon,
    Armor,
    Accessory,
    Unknown,
}

impl ShopEquipType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Good,
            1 => Self::Weapon,
            2 => Self::Armor,
            3 => Self::Accessory,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Good => "Good",
            Self::Weapon => "Weapon",
            Self::Armor => "Armor",
            Self::Accessory => "Accessory",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MerchantItem {
    pub shop_id: u32,
    pub merchant_name: &'static str,
    pub item_id: u32,
    pub item_name: &'static str,
    pub price: u32,
    pub quantity: i16,  // -1 = infinite
    pub equip_type: ShopEquipType,
    pub event_flag_stock: u32,
    pub event_flag_release: u32,
}

""")

        f.write(f"/// Merchant items database ({len(merchants)} entries)\n")
        f.write("pub static MERCHANT_ITEMS: Lazy<HashMap<u32, MerchantItem>> = Lazy::new(|| {\n")
        f.write("    let mut m = HashMap::new();\n")

        for item in merchants:
            equip_type = ["Good", "Weapon", "Armor", "Accessory", "Unknown"][min(item.equip_type, 4)]
            f.write(f'    m.insert({item.shop_id}, MerchantItem {{ ')
            f.write(f'shop_id: {item.shop_id}, ')
            f.write(f'merchant_name: "{escape_rust_string(item.merchant_name)}", ')
            f.write(f'item_id: {item.item_id}, ')
            f.write(f'item_name: "{escape_rust_string(item.item_name)}", ')
            f.write(f'price: {item.price}, ')
            f.write(f'quantity: {item.quantity}, ')
            f.write(f'equip_type: ShopEquipType::{equip_type}, ')
            f.write(f'event_flag_stock: {item.event_flag_stock}, ')
            f.write(f'event_flag_release: {item.event_flag_release}, ')
            f.write("});\n")

        f.write("    m\n")
        f.write("});\n\n")

        # Add unique merchants list
        merchants_set = sorted(set(m.merchant_name for m in merchants if m.merchant_name != "Unknown Merchant"))
        f.write("/// All unique merchant names\n")
        f.write("pub static MERCHANT_NAMES: &[&str] = &[\n")
        for name in merchants_set:
            f.write(f'    "{escape_rust_string(name)}",\n')
        f.write("];\n")

    print(f"  Wrote {path}")


def write_bosses_rust(bosses: List[BossData]):
    """Write bosses_data.rs"""
    path = OUTPUT_DIR / "bosses_data.rs"

    with open(path, 'w') as f:
        f.write("""//! Bosses database with defeat flags
//! This file is auto-generated by scripts/generate_db.py - do not edit manually

use once_cell::sync::Lazy;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BossType {
    Demigod,
    GreatBoss,
    Boss,
    Miniboss,
    Unknown,
}

impl BossType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Demigod => "Demigod",
            Self::GreatBoss => "Great Boss",
            Self::Boss => "Boss",
            Self::Miniboss => "Miniboss",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BossData {
    pub id: u32,
    pub name: &'static str,
    pub defeat_flag: u32,
    pub region: &'static str,
    pub boss_type: BossType,
    // Enrichment
    pub mapgenie_id: Option<&'static str>,
}

""")

        f.write(f"/// Bosses database ({len(bosses)} entries)\n")
        f.write("pub static BOSSES_DATA: Lazy<HashMap<u32, BossData>> = Lazy::new(|| {\n")
        f.write("    let mut m = HashMap::new();\n")

        for boss in bosses:
            boss_type_rust = {
                "demigod": "Demigod",
                "great_boss": "GreatBoss",
                "boss": "Boss",
                "miniboss": "Miniboss",
            }.get(boss.boss_type, "Unknown")

            f.write(f'    m.insert({boss.defeat_flag}, BossData {{ ')
            f.write(f'id: {boss.id}, ')
            f.write(f'name: "{escape_rust_string(boss.name)}", ')
            f.write(f'defeat_flag: {boss.defeat_flag}, ')
            f.write(f'region: "{escape_rust_string(boss.region)}", ')
            f.write(f'boss_type: BossType::{boss_type_rust}, ')
            f.write(f'mapgenie_id: {option_str(boss.mapgenie_id)}, ')
            f.write("});\n")

        f.write("    m\n")
        f.write("});\n\n")

        # Add regions
        regions = sorted(set(b.region for b in bosses))
        f.write("/// All unique regions with bosses\n")
        f.write("pub static BOSS_REGIONS: &[&str] = &[\n")
        for region in regions:
            f.write(f'    "{escape_rust_string(region)}",\n')
        f.write("];\n")

    print(f"  Wrote {path}")


# ============================================================================
# Main
# ============================================================================

def main():
    print("=" * 60)
    print("Database Generator for ER-save-Editor")
    print("=" * 60)
    print()

    # Verify paths exist
    if not GAME_FILES_DIR.exists():
        print(f"ERROR: Game files directory not found: {GAME_FILES_DIR}")
        return 1

    if not OUTPUT_DIR.exists():
        print(f"ERROR: Output directory not found: {OUTPUT_DIR}")
        return 1

    # Generate all databases
    graces = generate_graces()
    items = generate_unified_items()
    merchants = generate_merchants()
    bosses = generate_bosses()

    print()
    print("Writing Rust files...")

    write_graces_rust(graces)
    write_unified_items_rust(items)
    write_merchants_rust(merchants)
    write_bosses_rust(bosses)

    print()
    print("Done!")
    print("Don't forget to add new modules to src/db/mod.rs:")
    print("  pub mod graces_data;")
    print("  pub mod unified_items;")
    print("  pub mod merchants_data;")
    print("  pub mod bosses_data;")

    return 0


if __name__ == "__main__":
    exit(main())
