#!/usr/bin/env python3
"""
Extract event flags with coordinates from game params.
Generates enhanced event_flags.rs with WorldCoords support.
"""

import xml.etree.ElementTree as ET
import re
from pathlib import Path
from dataclasses import dataclass
from typing import Optional

BASE_PATH = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/regulation-bin")
OUTPUT_FILE = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/src/db/event_flags.rs")

# Current event_flags.rs mappings (we'll preserve these)
EXISTING_FLAGS = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/src/db/event_flags.rs")

@dataclass
class EventFlag:
    flag_id: int
    byte_offset: int
    bit_position: int
    name: str
    category: str
    x: Optional[float] = None
    y: Optional[float] = None
    z: Optional[float] = None
    map_area: Optional[int] = None
    map_x: Optional[int] = None
    map_z: Optional[int] = None

def flag_to_offset(flag_id: int) -> tuple[int, int]:
    """Convert flag ID to (byte_offset, bit_position)."""
    byte_offset = flag_id // 8
    bit_position = 7 - (flag_id % 8)
    return (byte_offset, bit_position)

def parse_existing_flags() -> dict[int, tuple[int, int]]:
    """Parse existing event_flags.rs to get known flag offsets."""
    flags = {}
    content = EXISTING_FLAGS.read_text()

    # Match patterns like (6080,(0x2f8,7)) or (300,(0x25,3))
    pattern = r'\((\d+),\s*\(0x([0-9a-fA-F]+),\s*(\d+)\)\)'
    for match in re.finditer(pattern, content):
        flag_id = int(match.group(1))
        byte_offset = int(match.group(2), 16)
        bit_position = int(match.group(3))
        flags[flag_id] = (byte_offset, bit_position)

    return flags

def parse_world_map_points() -> list[EventFlag]:
    """Parse WorldMapPointParam.param.xml for POI coordinates."""
    flags = []
    tree = ET.parse(BASE_PATH / "WorldMapPointParam.param.xml")
    root = tree.getroot()

    for row in root.findall('.//row'):
        flag_id = int(row.get('eventFlagId', '0'))
        if flag_id == 0:
            continue

        name = row.get('paramdexName', '')

        # Determine category from name/icon
        icon_id = int(row.get('iconId', '0'))
        if 'Grace' in name or icon_id == 83:
            category = "Grace"
        elif 'Guidance' in name:
            category = "Grace"
        else:
            category = "Landmark"

        # Clean up name
        if ':' in name:
            name = name.split(':')[-1].strip()
        name = name.split(',')[0].strip()  # Take first part if comma separated

        x = float(row.get('posX', '0'))
        y = float(row.get('posY', '0'))
        z = float(row.get('posZ', '0'))
        area_no = int(row.get('areaNo', '60'))
        grid_x = int(row.get('gridXNo', '0'))
        grid_z = int(row.get('gridZNo', '0'))

        byte_offset, bit_pos = flag_to_offset(flag_id)

        flags.append(EventFlag(
            flag_id=flag_id,
            byte_offset=byte_offset,
            bit_position=bit_pos,
            name=name,
            category=category,
            x=x, y=y, z=z,
            map_area=area_no, map_x=grid_x, map_z=grid_z
        ))

    return flags

def parse_item_lot_params() -> list[EventFlag]:
    """Parse ItemLotParam_map.param.xml for world pickup flags."""
    flags = []
    tree = ET.parse(BASE_PATH / "ItemLotParam_map.param.xml")
    root = tree.getroot()

    # Load item names for better descriptions
    item_names = load_item_names()

    for row in root.findall('.//row'):
        flag_id = int(row.get('getItemFlagId', '0'))
        if flag_id == 0:
            continue

        # Get item ID and category for naming
        item_id = int(row.get('lotItemId01', '0'))
        item_cat = int(row.get('lotItemCategory01', '0'))

        # Determine name from item
        if item_id in item_names:
            name = item_names[item_id]
        else:
            name = f"Item {item_id}"

        # Category based on item type
        if item_cat == 0:  # Weapon
            category = "WorldPickup"
        elif item_cat == 1:  # Good/Consumable
            category = "WorldPickup"
        elif item_cat == 2:  # Armor
            category = "WorldPickup"
        elif item_cat == 4:  # Accessory/Talisman
            category = "WorldPickup"
        else:
            category = "WorldPickup"

        byte_offset, bit_pos = flag_to_offset(flag_id)

        flags.append(EventFlag(
            flag_id=flag_id,
            byte_offset=byte_offset,
            bit_position=bit_pos,
            name=name,
            category=category,
        ))

    return flags

def load_item_names() -> dict[int, str]:
    """Load item names from EquipParamGoods."""
    names = {}
    try:
        tree = ET.parse(BASE_PATH / "EquipParamGoods.param.xml")
        root = tree.getroot()
        for row in root.findall('.//row'):
            item_id = int(row.get('id', '0'))
            name = row.get('paramdexName', '')
            if name:
                names[item_id] = name
    except:
        pass
    return names

def generate_rust_code(flags: list[EventFlag], existing: dict[int, tuple[int, int]]) -> str:
    """Generate the Rust source code."""

    # Merge existing flags (keeping their offsets as authoritative)
    flag_map = {}

    # First add existing flags
    for flag_id, (byte_off, bit_pos) in existing.items():
        flag_map[flag_id] = EventFlag(
            flag_id=flag_id,
            byte_offset=byte_off,
            bit_position=bit_pos,
            name="",
            category="Unknown"
        )

    # Then add/update with new flags (coordinate data, names)
    for f in flags:
        if f.flag_id in flag_map:
            # Update with coordinate data if available
            existing_flag = flag_map[f.flag_id]
            if f.x is not None:
                existing_flag.x = f.x
                existing_flag.y = f.y
                existing_flag.z = f.z
                existing_flag.map_area = f.map_area
                existing_flag.map_x = f.map_x
                existing_flag.map_z = f.map_z
            if f.name:
                existing_flag.name = f.name
            if f.category != "Unknown":
                existing_flag.category = f.category
        else:
            flag_map[f.flag_id] = f

    # Sort by flag ID
    sorted_flags = sorted(flag_map.values(), key=lambda f: f.flag_id)

    rust_code = '''// Auto-generated event flags database with coordinate support
// Contains {count} event flags

pub mod event_flags {{
    use std::{{collections::HashMap, sync::Mutex}};
    use once_cell::sync::Lazy;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct WorldCoords {{
        pub x: f32,
        pub y: f32,
        pub z: f32,
        pub map_area: u8,
        pub map_x: u8,
        pub map_z: u8,
    }}

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EventFlagCategory {{
        Grace,
        Boss,
        WorldPickup,
        LegacyPickup,
        ShopStock,
        NpcState,
        GreatRune,
        Cookbook,
        Whetblade,
        SummoningPool,
        Colosseum,
        Mausoleum,
        Map,
        Landmark,
        Unknown,
    }}

    #[derive(Debug, Clone)]
    pub struct EventFlagInfo {{
        pub byte_offset: u32,
        pub bit_position: u8,
        pub name: &'static str,
        pub category: EventFlagCategory,
        pub coords: Option<WorldCoords>,
    }}

    /// Legacy lookup: flag_id -> (byte_offset, bit_position)
    /// Kept for backwards compatibility
    pub static EVENT_FLAGS: Lazy<Mutex<HashMap<u32,(u32,u8)>>> = Lazy::new(|| {{
        Mutex::new(HashMap::from([
'''.format(count=len(sorted_flags))

    # Add legacy format entries
    for f in sorted_flags:
        rust_code += f"            ({f.flag_id},(0x{f.byte_offset:x},{f.bit_position})),\n"

    rust_code += '''        ]))
    });

    /// Enhanced lookup: flag_id -> EventFlagInfo (with coordinates)
    pub static EVENT_FLAGS_INFO: Lazy<HashMap<u32, EventFlagInfo>> = Lazy::new(|| {
        let mut map = HashMap::new();
'''

    # Add enhanced entries
    for f in sorted_flags:
        name = f.name.replace('"', '\\"') if f.name else ""

        if f.x is not None:
            coords = f"Some(WorldCoords {{ x: {f.x:.2f}, y: {f.y:.2f}, z: {f.z:.2f}, map_area: {f.map_area or 60}, map_x: {f.map_x or 0}, map_z: {f.map_z or 0} }})"
        else:
            coords = "None"

        rust_code += f'''        map.insert({f.flag_id}, EventFlagInfo {{
            byte_offset: 0x{f.byte_offset:x},
            bit_position: {f.bit_position},
            name: "{name}",
            category: EventFlagCategory::{f.category},
            coords: {coords},
        }});
'''

    rust_code += '''        map
    });

    /// Get event flag info by ID
    pub fn get_flag_info(flag_id: u32) -> Option<&'static EventFlagInfo> {
        EVENT_FLAGS_INFO.get(&flag_id)
    }

    /// Get byte offset and bit position for a flag
    pub fn get_flag_offset(flag_id: u32) -> Option<(u32, u8)> {
        EVENT_FLAGS.lock().ok()?.get(&flag_id).copied()
    }
}
'''

    return rust_code

def main():
    print("Parsing existing event_flags.rs...")
    existing = parse_existing_flags()
    print(f"  Found {len(existing)} existing flag mappings")

    print("Parsing WorldMapPointParam.param.xml...")
    world_points = parse_world_map_points()
    print(f"  Found {len(world_points)} world map points")

    print("Parsing ItemLotParam_map.param.xml...")
    item_lots = parse_item_lot_params()
    print(f"  Found {len(item_lots)} item lot flags")

    all_flags = world_points + item_lots

    print("Generating Rust code...")
    rust_code = generate_rust_code(all_flags, existing)

    OUTPUT_FILE.write_text(rust_code)
    print(f"Generated {OUTPUT_FILE}")
    print(f"Total flags: {len(existing) + len([f for f in all_flags if f.flag_id not in existing])}")

if __name__ == "__main__":
    main()
