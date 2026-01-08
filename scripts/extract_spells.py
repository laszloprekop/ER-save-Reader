#!/usr/bin/env python3
"""
Extract spell data from Magic.param.xml and generate spells.rs
"""

import xml.etree.ElementTree as ET
import re
from pathlib import Path

PARAM_FILE = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/regulation-bin/Magic.param.xml")
OUTPUT_FILE = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/src/db/spells.rs")

def parse_spell_name(paramdex_name: str) -> tuple[str, str]:
    """Parse the paramdexName to extract spell type and clean name."""
    # Format: "[Sorcery] Spell Name" or "[Incantation] Spell Name"
    match = re.match(r'\[(Sorcery|Incantation)\]\s*(.+)', paramdex_name)
    if match:
        spell_type = match.group(1)
        name = match.group(2).strip()
        return spell_type, name
    return "Unknown", paramdex_name

def main():
    tree = ET.parse(PARAM_FILE)
    root = tree.getroot()

    spells = []

    for row in root.findall('.//row'):
        spell_id = int(row.get('id'))
        paramdex_name = row.get('paramdexName', '')

        # Skip disabled/unused spells
        disable_param = row.get('disableParam_NT', '1')
        if disable_param == '1' and 'Unused' in paramdex_name:
            continue

        spell_type, name = parse_spell_name(paramdex_name)
        fp_cost = int(row.get('mp', '0'))
        slots = int(row.get('slotLength', '1'))
        int_req = int(row.get('requirementIntellect', '0'))
        fai_req = int(row.get('requirementFaith', '0'))
        ez_type = int(row.get('ezStateBehaviorType', '0'))  # 0=Sorcery, 1=Incantation

        # Determine spell type from ezStateBehaviorType (more reliable)
        if ez_type == 0:
            spell_type_enum = "SpellType::Sorcery"
        else:
            spell_type_enum = "SpellType::Incantation"

        spells.append({
            'id': spell_id,
            'name': name,
            'spell_type': spell_type_enum,
            'fp_cost': fp_cost,
            'slots': slots,
            'int_req': int_req,
            'fai_req': fai_req,
        })

    # Sort by ID
    spells.sort(key=lambda s: s['id'])

    # Generate Rust code
    rust_code = '''// Auto-generated from Magic.param.xml
// DO NOT EDIT MANUALLY

use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellType {
    Sorcery,
    Incantation,
}

#[derive(Debug, Clone)]
pub struct SpellInfo {
    pub name: &'static str,
    pub spell_type: SpellType,
    pub fp_cost: u16,
    pub slots: u8,
    pub int_req: u8,
    pub fai_req: u8,
}

pub static SPELLS: Lazy<HashMap<u32, SpellInfo>> = Lazy::new(|| {
    let mut map = HashMap::new();
'''

    for spell in spells:
        # Escape any quotes in the name
        name = spell['name'].replace('"', '\\"')
        rust_code += f'''    map.insert({spell['id']}, SpellInfo {{
        name: "{name}",
        spell_type: {spell['spell_type']},
        fp_cost: {spell['fp_cost']},
        slots: {spell['slots']},
        int_req: {spell['int_req']},
        fai_req: {spell['fai_req']},
    }});
'''

    rust_code += '''    map
});

/// Get spell info by ID
pub fn get_spell(id: u32) -> Option<&'static SpellInfo> {
    SPELLS.get(&id)
}

/// Get spell name by ID, returns "Unknown Spell" if not found
pub fn get_spell_name(id: u32) -> &'static str {
    SPELLS.get(&id).map(|s| s.name).unwrap_or("Unknown Spell")
}
'''

    OUTPUT_FILE.write_text(rust_code)
    print(f"Generated {OUTPUT_FILE} with {len(spells)} spells")

if __name__ == "__main__":
    main()
