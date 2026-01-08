#!/usr/bin/env python3
"""
Extract shop item data from ShopLineupParam.param.xml and generate shop_items.rs
"""

import xml.etree.ElementTree as ET
import re
from pathlib import Path
from collections import defaultdict

PARAM_FILE = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/regulation-bin/ShopLineupParam.param.xml")
OUTPUT_FILE = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/src/db/shop_items.rs")

def parse_merchant_name(paramdex_name: str) -> tuple[str, str]:
    """Extract merchant name and item name from paramdexName."""
    # Format: "[Merchant Name] Item Name" or "[Merchant - Condition] Item Name"
    match = re.match(r'\[([^\]]+)\]\s*(.+)', paramdex_name)
    if match:
        merchant = match.group(1).strip()
        item = match.group(2).strip()
        # Normalize merchant name (remove conditions like "- Quest")
        merchant = re.sub(r'\s*-\s*(Quest|Scroll|.+Scroll).*$', '', merchant)
        return merchant, item
    return "Unknown", paramdex_name

def equip_type_to_category(equip_type: int) -> str:
    """Convert equipType to category name."""
    categories = {
        0: "Weapon",
        1: "Armor",
        2: "Accessory",
        3: "Good",
        4: "AshOfWar",
    }
    return categories.get(equip_type, "Unknown")

def main():
    tree = ET.parse(PARAM_FILE)
    root = tree.getroot()

    shops = []
    merchants = defaultdict(list)

    for row in root.findall('.//row'):
        item_id = int(row.get('id'))
        equip_id = int(row.get('equipId', '0'))
        equip_type = int(row.get('equipType', '3'))
        value = int(row.get('value', '0'))
        stock_flag = int(row.get('eventFlag_forStock', '0'))
        release_flag = int(row.get('eventFlag_forRelease', '0'))
        sell_quantity = int(row.get('sellQuantity', '-1'))

        paramdex_name = row.get('paramdexName', '')

        # Skip entries without a stock flag (not purchasable items)
        if stock_flag == 0 and not paramdex_name:
            continue

        if paramdex_name:
            merchant_name, item_name = parse_merchant_name(paramdex_name)
        else:
            merchant_name = "Unknown"
            item_name = f"Item {equip_id}"

        category = equip_type_to_category(equip_type)

        shop_item = {
            'id': item_id,
            'equip_id': equip_id,
            'category': category,
            'merchant': merchant_name,
            'item_name': item_name,
            'price': value,
            'stock_flag': stock_flag,
            'release_flag': release_flag,
            'quantity': sell_quantity,
        }

        shops.append(shop_item)
        merchants[merchant_name].append(item_id)

    # Generate Rust code
    rust_code = '''// Auto-generated shop items database from ShopLineupParam.param.xml
// Contains {} shop items across {} merchants

use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemCategory {{
    Weapon,
    Armor,
    Accessory,
    Good,
    AshOfWar,
    Unknown,
}}

#[derive(Debug, Clone)]
pub struct ShopItem {{
    pub id: u32,
    pub equip_id: u32,
    pub category: ItemCategory,
    pub merchant: &'static str,
    pub item_name: &'static str,
    pub price: u32,
    pub stock_flag: u32,
    pub release_flag: u32,
    pub quantity: i32,
}}

/// All shop items indexed by shop lineup ID
pub static SHOP_ITEMS: Lazy<HashMap<u32, ShopItem>> = Lazy::new(|| {{
    let mut map = HashMap::new();
'''.format(len(shops), len(merchants))

    for item in shops:
        merchant = item['merchant'].replace('"', '\\"')
        item_name = item['item_name'].replace('"', '\\"')
        rust_code += f'''    map.insert({item['id']}, ShopItem {{
        id: {item['id']},
        equip_id: {item['equip_id']},
        category: ItemCategory::{item['category']},
        merchant: "{merchant}",
        item_name: "{item_name}",
        price: {item['price']},
        stock_flag: {item['stock_flag']},
        release_flag: {item['release_flag']},
        quantity: {item['quantity']},
    }});
'''

    rust_code += '''    map
});

/// Merchants and their shop item IDs
pub static MERCHANTS: Lazy<HashMap<&'static str, Vec<u32>>> = Lazy::new(|| {
    let mut map = HashMap::new();
'''

    for merchant, item_ids in sorted(merchants.items()):
        merchant_escaped = merchant.replace('"', '\\"')
        ids_str = ', '.join(str(i) for i in item_ids)
        rust_code += f'    map.insert("{merchant_escaped}", vec![{ids_str}]);\n'

    rust_code += '''    map
});

/// Get shop item by ID
pub fn get_shop_item(id: u32) -> Option<&'static ShopItem> {
    SHOP_ITEMS.get(&id)
}

/// Get all items for a specific merchant
pub fn get_merchant_items(merchant: &str) -> Vec<&'static ShopItem> {
    MERCHANTS.get(merchant)
        .map(|ids| ids.iter().filter_map(|id| SHOP_ITEMS.get(id)).collect())
        .unwrap_or_default()
}

/// Get all shop items with a specific stock flag
pub fn get_items_by_stock_flag(flag: u32) -> Vec<&'static ShopItem> {
    SHOP_ITEMS.values()
        .filter(|item| item.stock_flag == flag)
        .collect()
}

/// Get all merchants
pub fn get_merchant_names() -> Vec<&'static str> {
    MERCHANTS.keys().copied().collect()
}
'''

    OUTPUT_FILE.write_text(rust_code)
    print(f"Generated {OUTPUT_FILE}")
    print(f"Total shop items: {len(shops)}")
    print(f"Total merchants: {len(merchants)}")

if __name__ == "__main__":
    main()
