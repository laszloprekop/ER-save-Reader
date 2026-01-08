// Auto-generated from Magic.param.xml
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
    map.insert(4000, SpellInfo {
        name: "Glintstone Pebble",
        spell_type: SpellType::Sorcery,
        fp_cost: 7,
        slots: 1,
        int_req: 10,
        fai_req: 0,
    });
    map.insert(4001, SpellInfo {
        name: "Great Glintstone Shard",
        spell_type: SpellType::Sorcery,
        fp_cost: 12,
        slots: 1,
        int_req: 16,
        fai_req: 0,
    });
    map.insert(4010, SpellInfo {
        name: "Swift Glintstone Shard",
        spell_type: SpellType::Sorcery,
        fp_cost: 5,
        slots: 1,
        int_req: 12,
        fai_req: 0,
    });
    map.insert(4020, SpellInfo {
        name: "Glintstone Cometshard",
        spell_type: SpellType::Sorcery,
        fp_cost: 17,
        slots: 1,
        int_req: 36,
        fai_req: 0,
    });
    map.insert(4021, SpellInfo {
        name: "Comet",
        spell_type: SpellType::Sorcery,
        fp_cost: 24,
        slots: 1,
        int_req: 52,
        fai_req: 0,
    });
    map.insert(4030, SpellInfo {
        name: "Shard Spiral",
        spell_type: SpellType::Sorcery,
        fp_cost: 14,
        slots: 1,
        int_req: 27,
        fai_req: 0,
    });
    map.insert(4040, SpellInfo {
        name: "Glintstone Stars",
        spell_type: SpellType::Sorcery,
        fp_cost: 12,
        slots: 1,
        int_req: 12,
        fai_req: 0,
    });
    map.insert(4050, SpellInfo {
        name: "Star Shower",
        spell_type: SpellType::Sorcery,
        fp_cost: 23,
        slots: 1,
        int_req: 24,
        fai_req: 0,
    });
    map.insert(4060, SpellInfo {
        name: "Crystal Barrage",
        spell_type: SpellType::Sorcery,
        fp_cost: 14,
        slots: 1,
        int_req: 23,
        fai_req: 0,
    });
    map.insert(4070, SpellInfo {
        name: "Glintstone Arc",
        spell_type: SpellType::Sorcery,
        fp_cost: 9,
        slots: 1,
        int_req: 13,
        fai_req: 0,
    });
    map.insert(4080, SpellInfo {
        name: "Cannon of Haima",
        spell_type: SpellType::Sorcery,
        fp_cost: 38,
        slots: 1,
        int_req: 25,
        fai_req: 0,
    });
    map.insert(4090, SpellInfo {
        name: "Crystal Burst",
        spell_type: SpellType::Sorcery,
        fp_cost: 14,
        slots: 1,
        int_req: 18,
        fai_req: 0,
    });
    map.insert(4100, SpellInfo {
        name: "Shatter Earth",
        spell_type: SpellType::Sorcery,
        fp_cost: 10,
        slots: 1,
        int_req: 15,
        fai_req: 0,
    });
    map.insert(4110, SpellInfo {
        name: "Rock Blaster",
        spell_type: SpellType::Sorcery,
        fp_cost: 22,
        slots: 1,
        int_req: 21,
        fai_req: 0,
    });
    map.insert(4120, SpellInfo {
        name: "Gavel of Haima",
        spell_type: SpellType::Sorcery,
        fp_cost: 22,
        slots: 1,
        int_req: 25,
        fai_req: 0,
    });
    map.insert(4130, SpellInfo {
        name: "Terra Magicus",
        spell_type: SpellType::Sorcery,
        fp_cost: 20,
        slots: 1,
        int_req: 20,
        fai_req: 0,
    });
    map.insert(4140, SpellInfo {
        name: "Starlight",
        spell_type: SpellType::Sorcery,
        fp_cost: 9,
        slots: 1,
        int_req: 15,
        fai_req: 0,
    });
    map.insert(4200, SpellInfo {
        name: "Comet Azur",
        spell_type: SpellType::Sorcery,
        fp_cost: 40,
        slots: 3,
        int_req: 60,
        fai_req: 0,
    });
    map.insert(4210, SpellInfo {
        name: "Founding Rain of Stars",
        spell_type: SpellType::Sorcery,
        fp_cost: 27,
        slots: 2,
        int_req: 52,
        fai_req: 0,
    });
    map.insert(4220, SpellInfo {
        name: "Stars of Ruin",
        spell_type: SpellType::Sorcery,
        fp_cost: 32,
        slots: 1,
        int_req: 43,
        fai_req: 0,
    });
    map.insert(4300, SpellInfo {
        name: "Glintblade Phalanx",
        spell_type: SpellType::Sorcery,
        fp_cost: 18,
        slots: 1,
        int_req: 22,
        fai_req: 0,
    });
    map.insert(4301, SpellInfo {
        name: "Carian Phalanx",
        spell_type: SpellType::Sorcery,
        fp_cost: 24,
        slots: 1,
        int_req: 34,
        fai_req: 0,
    });
    map.insert(4302, SpellInfo {
        name: "Greatblade Phalanx",
        spell_type: SpellType::Sorcery,
        fp_cost: 30,
        slots: 1,
        int_req: 29,
        fai_req: 0,
    });
    map.insert(4360, SpellInfo {
        name: "Rennala's Full Moon",
        spell_type: SpellType::Sorcery,
        fp_cost: 47,
        slots: 2,
        int_req: 70,
        fai_req: 0,
    });
    map.insert(4361, SpellInfo {
        name: "Ranni's Dark Moon",
        spell_type: SpellType::Sorcery,
        fp_cost: 57,
        slots: 2,
        int_req: 68,
        fai_req: 0,
    });
    map.insert(4370, SpellInfo {
        name: "Magic Downpour",
        spell_type: SpellType::Sorcery,
        fp_cost: 18,
        slots: 1,
        int_req: 15,
        fai_req: 0,
    });
    map.insert(4380, SpellInfo {
        name: "Loretta's Greatbow",
        spell_type: SpellType::Sorcery,
        fp_cost: 24,
        slots: 1,
        int_req: 26,
        fai_req: 0,
    });
    map.insert(4381, SpellInfo {
        name: "Loretta's Mastery",
        spell_type: SpellType::Sorcery,
        fp_cost: 39,
        slots: 1,
        int_req: 46,
        fai_req: 0,
    });
    map.insert(4390, SpellInfo {
        name: "Magic Glintblade",
        spell_type: SpellType::Sorcery,
        fp_cost: 12,
        slots: 1,
        int_req: 14,
        fai_req: 0,
    });
    map.insert(4400, SpellInfo {
        name: "Glintstone Icecrag",
        spell_type: SpellType::Sorcery,
        fp_cost: 12,
        slots: 1,
        int_req: 15,
        fai_req: 0,
    });
    map.insert(4410, SpellInfo {
        name: "Zamor Ice Storm",
        spell_type: SpellType::Sorcery,
        fp_cost: 17,
        slots: 1,
        int_req: 36,
        fai_req: 0,
    });
    map.insert(4420, SpellInfo {
        name: "Freezing Mist",
        spell_type: SpellType::Sorcery,
        fp_cost: 20,
        slots: 1,
        int_req: 21,
        fai_req: 0,
    });
    map.insert(4430, SpellInfo {
        name: "Carian Greatsword",
        spell_type: SpellType::Sorcery,
        fp_cost: 12,
        slots: 1,
        int_req: 24,
        fai_req: 0,
    });
    map.insert(4431, SpellInfo {
        name: "Adula's Moonblade",
        spell_type: SpellType::Sorcery,
        fp_cost: 22,
        slots: 1,
        int_req: 32,
        fai_req: 0,
    });
    map.insert(4440, SpellInfo {
        name: "Carian Slicer",
        spell_type: SpellType::Sorcery,
        fp_cost: 4,
        slots: 1,
        int_req: 14,
        fai_req: 0,
    });
    map.insert(4450, SpellInfo {
        name: "Carian Piercer",
        spell_type: SpellType::Sorcery,
        fp_cost: 17,
        slots: 1,
        int_req: 27,
        fai_req: 0,
    });
    map.insert(4460, SpellInfo {
        name: "Scholar's Armament",
        spell_type: SpellType::Sorcery,
        fp_cost: 25,
        slots: 1,
        int_req: 12,
        fai_req: 0,
    });
    map.insert(4470, SpellInfo {
        name: "Scholar's Shield",
        spell_type: SpellType::Sorcery,
        fp_cost: 30,
        slots: 1,
        int_req: 12,
        fai_req: 0,
    });
    map.insert(4480, SpellInfo {
        name: "Lucidity",
        spell_type: SpellType::Sorcery,
        fp_cost: 10,
        slots: 1,
        int_req: 17,
        fai_req: 0,
    });
    map.insert(4490, SpellInfo {
        name: "Frozen Armament",
        spell_type: SpellType::Sorcery,
        fp_cost: 20,
        slots: 1,
        int_req: 15,
        fai_req: 0,
    });
    map.insert(4500, SpellInfo {
        name: "Shattering Crystal",
        spell_type: SpellType::Sorcery,
        fp_cost: 21,
        slots: 1,
        int_req: 38,
        fai_req: 0,
    });
    map.insert(4510, SpellInfo {
        name: "Crystal Release",
        spell_type: SpellType::Sorcery,
        fp_cost: 34,
        slots: 1,
        int_req: 41,
        fai_req: 0,
    });
    map.insert(4520, SpellInfo {
        name: "Crystal Torrent",
        spell_type: SpellType::Sorcery,
        fp_cost: 20,
        slots: 1,
        int_req: 47,
        fai_req: 0,
    });
    map.insert(4600, SpellInfo {
        name: "Ambush Shard",
        spell_type: SpellType::Sorcery,
        fp_cost: 13,
        slots: 1,
        int_req: 23,
        fai_req: 0,
    });
    map.insert(4610, SpellInfo {
        name: "Night Shard",
        spell_type: SpellType::Sorcery,
        fp_cost: 7,
        slots: 1,
        int_req: 18,
        fai_req: 0,
    });
    map.insert(4620, SpellInfo {
        name: "Night Comet",
        spell_type: SpellType::Sorcery,
        fp_cost: 21,
        slots: 1,
        int_req: 38,
        fai_req: 0,
    });
    map.insert(4630, SpellInfo {
        name: "Thops's Barrier",
        spell_type: SpellType::Sorcery,
        fp_cost: 7,
        slots: 1,
        int_req: 18,
        fai_req: 0,
    });
    map.insert(4640, SpellInfo {
        name: "Carian Retaliation",
        spell_type: SpellType::Sorcery,
        fp_cost: 8,
        slots: 1,
        int_req: 17,
        fai_req: 0,
    });
    map.insert(4650, SpellInfo {
        name: "Eternal Darkness",
        spell_type: SpellType::Sorcery,
        fp_cost: 25,
        slots: 1,
        int_req: 35,
        fai_req: 0,
    });
    map.insert(4660, SpellInfo {
        name: "Unseen Blade",
        spell_type: SpellType::Sorcery,
        fp_cost: 13,
        slots: 1,
        int_req: 12,
        fai_req: 0,
    });
    map.insert(4670, SpellInfo {
        name: "Unseen Form",
        spell_type: SpellType::Sorcery,
        fp_cost: 20,
        slots: 1,
        int_req: 16,
        fai_req: 0,
    });
    map.insert(4700, SpellInfo {
        name: "Meteorite",
        spell_type: SpellType::Sorcery,
        fp_cost: 30,
        slots: 1,
        int_req: 30,
        fai_req: 0,
    });
    map.insert(4701, SpellInfo {
        name: "Meteorite of Astel",
        spell_type: SpellType::Sorcery,
        fp_cost: 60,
        slots: 2,
        int_req: 55,
        fai_req: 0,
    });
    map.insert(4710, SpellInfo {
        name: "Rock Sling",
        spell_type: SpellType::Sorcery,
        fp_cost: 18,
        slots: 1,
        int_req: 18,
        fai_req: 0,
    });
    map.insert(4720, SpellInfo {
        name: "Gravity Well",
        spell_type: SpellType::Sorcery,
        fp_cost: 12,
        slots: 1,
        int_req: 17,
        fai_req: 0,
    });
    map.insert(4721, SpellInfo {
        name: "Collapsing Stars",
        spell_type: SpellType::Sorcery,
        fp_cost: 18,
        slots: 1,
        int_req: 36,
        fai_req: 0,
    });
    map.insert(4800, SpellInfo {
        name: "Magma Shot",
        spell_type: SpellType::Sorcery,
        fp_cost: 16,
        slots: 1,
        int_req: 19,
        fai_req: 10,
    });
    map.insert(4810, SpellInfo {
        name: "Gelmir's Fury",
        spell_type: SpellType::Sorcery,
        fp_cost: 16,
        slots: 1,
        int_req: 28,
        fai_req: 15,
    });
    map.insert(4820, SpellInfo {
        name: "Roiling Magma",
        spell_type: SpellType::Sorcery,
        fp_cost: 28,
        slots: 1,
        int_req: 21,
        fai_req: 12,
    });
    map.insert(4830, SpellInfo {
        name: "Rykard's Rancor",
        spell_type: SpellType::Sorcery,
        fp_cost: 23,
        slots: 2,
        int_req: 40,
        fai_req: 18,
    });
    map.insert(4900, SpellInfo {
        name: "Briars of Sin",
        spell_type: SpellType::Sorcery,
        fp_cost: 6,
        slots: 1,
        int_req: 0,
        fai_req: 24,
    });
    map.insert(4910, SpellInfo {
        name: "Briars of Punishment",
        spell_type: SpellType::Sorcery,
        fp_cost: 9,
        slots: 1,
        int_req: 0,
        fai_req: 21,
    });
    map.insert(5000, SpellInfo {
        name: "Rancorcall",
        spell_type: SpellType::Sorcery,
        fp_cost: 12,
        slots: 1,
        int_req: 16,
        fai_req: 14,
    });
    map.insert(5001, SpellInfo {
        name: "Ancient Death Rancor",
        spell_type: SpellType::Sorcery,
        fp_cost: 21,
        slots: 1,
        int_req: 34,
        fai_req: 24,
    });
    map.insert(5010, SpellInfo {
        name: "Explosive Ghostflame",
        spell_type: SpellType::Sorcery,
        fp_cost: 29,
        slots: 1,
        int_req: 42,
        fai_req: 30,
    });
    map.insert(5020, SpellInfo {
        name: "Fia's Mist",
        spell_type: SpellType::Sorcery,
        fp_cost: 25,
        slots: 1,
        int_req: 23,
        fai_req: 18,
    });
    map.insert(5030, SpellInfo {
        name: "Tibia's Summons",
        spell_type: SpellType::Sorcery,
        fp_cost: 17,
        slots: 1,
        int_req: 28,
        fai_req: 20,
    });
    map.insert(5040, SpellInfo {
        name: "Death Lightning",
        spell_type: SpellType::Incantation,
        fp_cost: 28,
        slots: 2,
        int_req: 0,
        fai_req: 47,
    });
    map.insert(5100, SpellInfo {
        name: "Oracle Bubbles",
        spell_type: SpellType::Sorcery,
        fp_cost: 12,
        slots: 1,
        int_req: 19,
        fai_req: 0,
    });
    map.insert(5110, SpellInfo {
        name: "Great Oracular Bubble",
        spell_type: SpellType::Sorcery,
        fp_cost: 16,
        slots: 1,
        int_req: 25,
        fai_req: 0,
    });
    map.insert(6000, SpellInfo {
        name: "Catch Flame",
        spell_type: SpellType::Incantation,
        fp_cost: 10,
        slots: 1,
        int_req: 0,
        fai_req: 8,
    });
    map.insert(6001, SpellInfo {
        name: "O, Flame!",
        spell_type: SpellType::Incantation,
        fp_cost: 16,
        slots: 1,
        int_req: 0,
        fai_req: 16,
    });
    map.insert(6010, SpellInfo {
        name: "Flame Sling",
        spell_type: SpellType::Incantation,
        fp_cost: 11,
        slots: 1,
        int_req: 0,
        fai_req: 10,
    });
    map.insert(6020, SpellInfo {
        name: "Flame Fall Upon Them",
        spell_type: SpellType::Incantation,
        fp_cost: 16,
        slots: 1,
        int_req: 0,
        fai_req: 28,
    });
    map.insert(6030, SpellInfo {
        name: "Whirl, O Flame",
        spell_type: SpellType::Incantation,
        fp_cost: 19,
        slots: 1,
        int_req: 0,
        fai_req: 13,
    });
    map.insert(6040, SpellInfo {
        name: "Flame Cleanse Me",
        spell_type: SpellType::Incantation,
        fp_cost: 14,
        slots: 1,
        int_req: 0,
        fai_req: 12,
    });
    map.insert(6050, SpellInfo {
        name: "Flame Grant Me Strength",
        spell_type: SpellType::Incantation,
        fp_cost: 28,
        slots: 1,
        int_req: 0,
        fai_req: 15,
    });
    map.insert(6060, SpellInfo {
        name: "Flame Protect Me",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 1,
        int_req: 0,
        fai_req: 24,
    });
    map.insert(6100, SpellInfo {
        name: "Giantsflame Take Thee",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 2,
        int_req: 0,
        fai_req: 30,
    });
    map.insert(6110, SpellInfo {
        name: "Flame of The Fell God",
        spell_type: SpellType::Incantation,
        fp_cost: 34,
        slots: 2,
        int_req: 0,
        fai_req: 41,
    });
    map.insert(6120, SpellInfo {
        name: "Burn, O Flame!",
        spell_type: SpellType::Incantation,
        fp_cost: 26,
        slots: 1,
        int_req: 0,
        fai_req: 27,
    });
    map.insert(6210, SpellInfo {
        name: "Black Flame",
        spell_type: SpellType::Incantation,
        fp_cost: 18,
        slots: 1,
        int_req: 0,
        fai_req: 20,
    });
    map.insert(6220, SpellInfo {
        name: "Surge, O Flame",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 9,
    });
    map.insert(6230, SpellInfo {
        name: "Scouring Black Flame",
        spell_type: SpellType::Incantation,
        fp_cost: 21,
        slots: 1,
        int_req: 0,
        fai_req: 28,
    });
    map.insert(6240, SpellInfo {
        name: "Black Flame Ritual",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 1,
        int_req: 0,
        fai_req: 42,
    });
    map.insert(6250, SpellInfo {
        name: "Black Flame Blade",
        spell_type: SpellType::Incantation,
        fp_cost: 15,
        slots: 1,
        int_req: 0,
        fai_req: 17,
    });
    map.insert(6260, SpellInfo {
        name: "Black Flame's Protection",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 1,
        int_req: 0,
        fai_req: 30,
    });
    map.insert(6270, SpellInfo {
        name: "Noble Presence",
        spell_type: SpellType::Incantation,
        fp_cost: 20,
        slots: 1,
        int_req: 0,
        fai_req: 26,
    });
    map.insert(6300, SpellInfo {
        name: "Bloodflame Talons",
        spell_type: SpellType::Incantation,
        fp_cost: 12,
        slots: 1,
        int_req: 0,
        fai_req: 13,
    });
    map.insert(6310, SpellInfo {
        name: "Bloodboon",
        spell_type: SpellType::Incantation,
        fp_cost: 13,
        slots: 1,
        int_req: 0,
        fai_req: 14,
    });
    map.insert(6320, SpellInfo {
        name: "Bloodflame Blade",
        spell_type: SpellType::Incantation,
        fp_cost: 20,
        slots: 1,
        int_req: 0,
        fai_req: 12,
    });
    map.insert(6330, SpellInfo {
        name: "Barrier of Gold",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 1,
        int_req: 0,
        fai_req: 24,
    });
    map.insert(6340, SpellInfo {
        name: "Protection of The Erdtree",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 1,
        int_req: 0,
        fai_req: 35,
    });
    map.insert(6400, SpellInfo {
        name: "Rejection",
        spell_type: SpellType::Incantation,
        fp_cost: 9,
        slots: 1,
        int_req: 0,
        fai_req: 12,
    });
    map.insert(6410, SpellInfo {
        name: "Wrath of Gold",
        spell_type: SpellType::Incantation,
        fp_cost: 40,
        slots: 1,
        int_req: 0,
        fai_req: 32,
    });
    map.insert(6420, SpellInfo {
        name: "Urgent Heal",
        spell_type: SpellType::Incantation,
        fp_cost: 16,
        slots: 1,
        int_req: 0,
        fai_req: 8,
    });
    map.insert(6421, SpellInfo {
        name: "Heal",
        spell_type: SpellType::Incantation,
        fp_cost: 32,
        slots: 1,
        int_req: 0,
        fai_req: 12,
    });
    map.insert(6422, SpellInfo {
        name: "Great Heal",
        spell_type: SpellType::Incantation,
        fp_cost: 45,
        slots: 1,
        int_req: 0,
        fai_req: 15,
    });
    map.insert(6423, SpellInfo {
        name: "Lord's Heal",
        spell_type: SpellType::Incantation,
        fp_cost: 55,
        slots: 1,
        int_req: 0,
        fai_req: 20,
    });
    map.insert(6424, SpellInfo {
        name: "Erdtree Heal",
        spell_type: SpellType::Incantation,
        fp_cost: 65,
        slots: 1,
        int_req: 0,
        fai_req: 42,
    });
    map.insert(6430, SpellInfo {
        name: "Blessing's Boon",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 1,
        int_req: 0,
        fai_req: 24,
    });
    map.insert(6431, SpellInfo {
        name: "Blessing of The Erdtree",
        spell_type: SpellType::Incantation,
        fp_cost: 60,
        slots: 1,
        int_req: 0,
        fai_req: 38,
    });
    map.insert(6440, SpellInfo {
        name: "Cure Poison",
        spell_type: SpellType::Incantation,
        fp_cost: 7,
        slots: 1,
        int_req: 0,
        fai_req: 10,
    });
    map.insert(6441, SpellInfo {
        name: "Lord's Aid",
        spell_type: SpellType::Incantation,
        fp_cost: 12,
        slots: 1,
        int_req: 0,
        fai_req: 12,
    });
    map.insert(6450, SpellInfo {
        name: "Flame Fortification",
        spell_type: SpellType::Incantation,
        fp_cost: 20,
        slots: 1,
        int_req: 0,
        fai_req: 10,
    });
    map.insert(6460, SpellInfo {
        name: "Magic Fortification",
        spell_type: SpellType::Incantation,
        fp_cost: 20,
        slots: 1,
        int_req: 0,
        fai_req: 10,
    });
    map.insert(6470, SpellInfo {
        name: "Lightning Fortification",
        spell_type: SpellType::Incantation,
        fp_cost: 20,
        slots: 1,
        int_req: 0,
        fai_req: 10,
    });
    map.insert(6480, SpellInfo {
        name: "Divine Fortification",
        spell_type: SpellType::Incantation,
        fp_cost: 20,
        slots: 1,
        int_req: 0,
        fai_req: 10,
    });
    map.insert(6490, SpellInfo {
        name: "Lord's Divine Fortification",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 1,
        int_req: 0,
        fai_req: 27,
    });
    map.insert(6500, SpellInfo {
        name: "Night Maiden's Mist",
        spell_type: SpellType::Sorcery,
        fp_cost: 20,
        slots: 1,
        int_req: 14,
        fai_req: 0,
    });
    map.insert(6510, SpellInfo {
        name: "Assassin's Approach",
        spell_type: SpellType::Incantation,
        fp_cost: 15,
        slots: 1,
        int_req: 0,
        fai_req: 10,
    });
    map.insert(6520, SpellInfo {
        name: "Shadow Bait",
        spell_type: SpellType::Incantation,
        fp_cost: 15,
        slots: 1,
        int_req: 0,
        fai_req: 13,
    });
    map.insert(6530, SpellInfo {
        name: "Darkness",
        spell_type: SpellType::Incantation,
        fp_cost: 24,
        slots: 1,
        int_req: 0,
        fai_req: 18,
    });
    map.insert(6600, SpellInfo {
        name: "Golden Vow",
        spell_type: SpellType::Incantation,
        fp_cost: 47,
        slots: 1,
        int_req: 0,
        fai_req: 25,
    });
    map.insert(6700, SpellInfo {
        name: "Discus of Light",
        spell_type: SpellType::Incantation,
        fp_cost: 3,
        slots: 1,
        int_req: 13,
        fai_req: 13,
    });
    map.insert(6701, SpellInfo {
        name: "Triple Rings of Light",
        spell_type: SpellType::Incantation,
        fp_cost: 23,
        slots: 1,
        int_req: 23,
        fai_req: 23,
    });
    map.insert(6710, SpellInfo {
        name: "Radagon's Rings of Light",
        spell_type: SpellType::Incantation,
        fp_cost: 21,
        slots: 1,
        int_req: 31,
        fai_req: 31,
    });
    map.insert(6720, SpellInfo {
        name: "Elden Stars",
        spell_type: SpellType::Incantation,
        fp_cost: 41,
        slots: 2,
        int_req: 0,
        fai_req: 50,
    });
    map.insert(6730, SpellInfo {
        name: "Law of Regression",
        spell_type: SpellType::Incantation,
        fp_cost: 55,
        slots: 1,
        int_req: 37,
        fai_req: 0,
    });
    map.insert(6740, SpellInfo {
        name: "Immutable Shield",
        spell_type: SpellType::Incantation,
        fp_cost: 15,
        slots: 1,
        int_req: 19,
        fai_req: 19,
    });
    map.insert(6750, SpellInfo {
        name: "Litany of Proper Death",
        spell_type: SpellType::Incantation,
        fp_cost: 17,
        slots: 1,
        int_req: 17,
        fai_req: 17,
    });
    map.insert(6760, SpellInfo {
        name: "Law of Causality",
        spell_type: SpellType::Incantation,
        fp_cost: 22,
        slots: 1,
        int_req: 29,
        fai_req: 0,
    });
    map.insert(6770, SpellInfo {
        name: "Order's Blade",
        spell_type: SpellType::Incantation,
        fp_cost: 22,
        slots: 1,
        int_req: 13,
        fai_req: 13,
    });
    map.insert(6780, SpellInfo {
        name: "Order Healing",
        spell_type: SpellType::Incantation,
        fp_cost: 15,
        slots: 1,
        int_req: 11,
        fai_req: 11,
    });
    map.insert(6800, SpellInfo {
        name: "Bestial Sling",
        spell_type: SpellType::Incantation,
        fp_cost: 7,
        slots: 1,
        int_req: 0,
        fai_req: 10,
    });
    map.insert(6810, SpellInfo {
        name: "Stone of Gurranq",
        spell_type: SpellType::Incantation,
        fp_cost: 15,
        slots: 1,
        int_req: 0,
        fai_req: 13,
    });
    map.insert(6820, SpellInfo {
        name: "Beast Claw",
        spell_type: SpellType::Incantation,
        fp_cost: 10,
        slots: 1,
        int_req: 0,
        fai_req: 8,
    });
    map.insert(6830, SpellInfo {
        name: "Gurranq's Beast Claw",
        spell_type: SpellType::Incantation,
        fp_cost: 21,
        slots: 1,
        int_req: 0,
        fai_req: 15,
    });
    map.insert(6840, SpellInfo {
        name: "Bestial Vitality",
        spell_type: SpellType::Incantation,
        fp_cost: 18,
        slots: 1,
        int_req: 0,
        fai_req: 12,
    });
    map.insert(6850, SpellInfo {
        name: "Bestial Constitution",
        spell_type: SpellType::Incantation,
        fp_cost: 10,
        slots: 1,
        int_req: 0,
        fai_req: 9,
    });
    map.insert(6900, SpellInfo {
        name: "Lightning Spear",
        spell_type: SpellType::Incantation,
        fp_cost: 18,
        slots: 1,
        int_req: 0,
        fai_req: 17,
    });
    map.insert(6910, SpellInfo {
        name: "Ancient Dragons' Light Strike",
        spell_type: SpellType::Incantation,
        fp_cost: 36,
        slots: 1,
        int_req: 0,
        fai_req: 26,
    });
    map.insert(6920, SpellInfo {
        name: "Lightning Strike",
        spell_type: SpellType::Incantation,
        fp_cost: 19,
        slots: 1,
        int_req: 0,
        fai_req: 28,
    });
    map.insert(6921, SpellInfo {
        name: "Frozen Lightning Spear",
        spell_type: SpellType::Incantation,
        fp_cost: 29,
        slots: 1,
        int_req: 0,
        fai_req: 34,
    });
    map.insert(6930, SpellInfo {
        name: "Honed Bolt",
        spell_type: SpellType::Incantation,
        fp_cost: 12,
        slots: 1,
        int_req: 0,
        fai_req: 24,
    });
    map.insert(6940, SpellInfo {
        name: "Ancient Dragons' Light Spear",
        spell_type: SpellType::Incantation,
        fp_cost: 25,
        slots: 1,
        int_req: 0,
        fai_req: 32,
    });
    map.insert(6941, SpellInfo {
        name: "Fortissax's Light Spear",
        spell_type: SpellType::Incantation,
        fp_cost: 35,
        slots: 1,
        int_req: 0,
        fai_req: 46,
    });
    map.insert(6950, SpellInfo {
        name: "Lansseax's Glaive",
        spell_type: SpellType::Incantation,
        fp_cost: 22,
        slots: 1,
        int_req: 0,
        fai_req: 40,
    });
    map.insert(6960, SpellInfo {
        name: "Electrify Armament",
        spell_type: SpellType::Incantation,
        fp_cost: 27,
        slots: 1,
        int_req: 0,
        fai_req: 15,
    });
    map.insert(6970, SpellInfo {
        name: "Vyke's Dragonbolt",
        spell_type: SpellType::Incantation,
        fp_cost: 35,
        slots: 1,
        int_req: 0,
        fai_req: 23,
    });
    map.insert(6971, SpellInfo {
        name: "Dragonbolt Blessing",
        spell_type: SpellType::Incantation,
        fp_cost: 20,
        slots: 1,
        int_req: 0,
        fai_req: 21,
    });
    map.insert(7000, SpellInfo {
        name: "Dragonfire",
        spell_type: SpellType::Incantation,
        fp_cost: 28,
        slots: 1,
        int_req: 0,
        fai_req: 15,
    });
    map.insert(7001, SpellInfo {
        name: "Agheel's Flame",
        spell_type: SpellType::Incantation,
        fp_cost: 36,
        slots: 1,
        int_req: 0,
        fai_req: 23,
    });
    map.insert(7010, SpellInfo {
        name: "Magma Breath",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 1,
        int_req: 0,
        fai_req: 14,
    });
    map.insert(7011, SpellInfo {
        name: "Theodorix's Magma",
        spell_type: SpellType::Incantation,
        fp_cost: 45,
        slots: 1,
        int_req: 0,
        fai_req: 21,
    });
    map.insert(7020, SpellInfo {
        name: "Dragonice",
        spell_type: SpellType::Incantation,
        fp_cost: 36,
        slots: 1,
        int_req: 0,
        fai_req: 15,
    });
    map.insert(7021, SpellInfo {
        name: "Borealis's Mist",
        spell_type: SpellType::Incantation,
        fp_cost: 48,
        slots: 1,
        int_req: 0,
        fai_req: 23,
    });
    map.insert(7030, SpellInfo {
        name: "Rotten Breath",
        spell_type: SpellType::Incantation,
        fp_cost: 36,
        slots: 1,
        int_req: 0,
        fai_req: 15,
    });
    map.insert(7031, SpellInfo {
        name: "Ekzykes's Decay",
        spell_type: SpellType::Incantation,
        fp_cost: 48,
        slots: 1,
        int_req: 0,
        fai_req: 23,
    });
    map.insert(7040, SpellInfo {
        name: "Glintstone Breath",
        spell_type: SpellType::Incantation,
        fp_cost: 28,
        slots: 1,
        int_req: 0,
        fai_req: 15,
    });
    map.insert(7041, SpellInfo {
        name: "Smarag's Glint Breath",
        spell_type: SpellType::Incantation,
        fp_cost: 36,
        slots: 1,
        int_req: 0,
        fai_req: 23,
    });
    map.insert(7050, SpellInfo {
        name: "Placidusax's Ruin",
        spell_type: SpellType::Incantation,
        fp_cost: 62,
        slots: 3,
        int_req: 0,
        fai_req: 36,
    });
    map.insert(7060, SpellInfo {
        name: "Dragonclaw",
        spell_type: SpellType::Incantation,
        fp_cost: 24,
        slots: 1,
        int_req: 0,
        fai_req: 17,
    });
    map.insert(7080, SpellInfo {
        name: "Dragonmaw",
        spell_type: SpellType::Incantation,
        fp_cost: 34,
        slots: 1,
        int_req: 0,
        fai_req: 24,
    });
    map.insert(7090, SpellInfo {
        name: "Greyoll's Roar",
        spell_type: SpellType::Incantation,
        fp_cost: 50,
        slots: 2,
        int_req: 0,
        fai_req: 28,
    });
    map.insert(7200, SpellInfo {
        name: "Pest Threads",
        spell_type: SpellType::Incantation,
        fp_cost: 19,
        slots: 1,
        int_req: 0,
        fai_req: 11,
    });
    map.insert(7210, SpellInfo {
        name: "Swarm of Flies",
        spell_type: SpellType::Incantation,
        fp_cost: 14,
        slots: 1,
        int_req: 0,
        fai_req: 11,
    });
    map.insert(7220, SpellInfo {
        name: "Poison Mist",
        spell_type: SpellType::Incantation,
        fp_cost: 18,
        slots: 1,
        int_req: 0,
        fai_req: 12,
    });
    map.insert(7230, SpellInfo {
        name: "Poison Armament",
        spell_type: SpellType::Incantation,
        fp_cost: 15,
        slots: 1,
        int_req: 0,
        fai_req: 10,
    });
    map.insert(7240, SpellInfo {
        name: "Scarlet Aeonia",
        spell_type: SpellType::Incantation,
        fp_cost: 48,
        slots: 3,
        int_req: 0,
        fai_req: 35,
    });
    map.insert(7300, SpellInfo {
        name: "Inescapable Frenzy",
        spell_type: SpellType::Incantation,
        fp_cost: 22,
        slots: 1,
        int_req: 0,
        fai_req: 21,
    });
    map.insert(7310, SpellInfo {
        name: "The Flame of Frenzy",
        spell_type: SpellType::Incantation,
        fp_cost: 16,
        slots: 1,
        int_req: 0,
        fai_req: 16,
    });
    map.insert(7311, SpellInfo {
        name: "Unendurable Frenzy",
        spell_type: SpellType::Incantation,
        fp_cost: 22,
        slots: 1,
        int_req: 0,
        fai_req: 31,
    });
    map.insert(7320, SpellInfo {
        name: "Frenzied Burst",
        spell_type: SpellType::Incantation,
        fp_cost: 24,
        slots: 1,
        int_req: 0,
        fai_req: 22,
    });
    map.insert(7330, SpellInfo {
        name: "Howl of Shabriri",
        spell_type: SpellType::Incantation,
        fp_cost: 21,
        slots: 1,
        int_req: 0,
        fai_req: 33,
    });
    map.insert(7500, SpellInfo {
        name: "Aspects of the Crucible: Tail",
        spell_type: SpellType::Incantation,
        fp_cost: 20,
        slots: 1,
        int_req: 0,
        fai_req: 27,
    });
    map.insert(7510, SpellInfo {
        name: "Aspects of the Crucible: Horns",
        spell_type: SpellType::Incantation,
        fp_cost: 18,
        slots: 1,
        int_req: 0,
        fai_req: 27,
    });
    map.insert(7520, SpellInfo {
        name: "Aspects of the Crucible: Breath",
        spell_type: SpellType::Incantation,
        fp_cost: 28,
        slots: 1,
        int_req: 0,
        fai_req: 27,
    });
    map.insert(7530, SpellInfo {
        name: "Black Blade",
        spell_type: SpellType::Incantation,
        fp_cost: 26,
        slots: 2,
        int_req: 0,
        fai_req: 46,
    });
    map.insert(7900, SpellInfo {
        name: "Fire's Deadly Sin",
        spell_type: SpellType::Incantation,
        fp_cost: 26,
        slots: 1,
        int_req: 0,
        fai_req: 19,
    });
    map.insert(7903, SpellInfo {
        name: "Golden Light Fortification",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 1,
        int_req: 0,
        fai_req: 24,
    });
    map.insert(8000, SpellInfo {
        name: "Briars of Sin",
        spell_type: SpellType::Sorcery,
        fp_cost: 9,
        slots: 1,
        int_req: 24,
        fai_req: 0,
    });
    map.insert(8001, SpellInfo {
        name: "Briars of Punishment",
        spell_type: SpellType::Sorcery,
        fp_cost: 9,
        slots: 1,
        int_req: 21,
        fai_req: 0,
    });
    map.insert(53010, SpellInfo {
        name: "[NPC: Incantation] Catch Flame",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53011, SpellInfo {
        name: "[NPC: Incantation] Bloodflame Blade",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53012, SpellInfo {
        name: "[NPC: Incantation] Swarm of Flies 1",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53040, SpellInfo {
        name: "[NPC: Incantation] Lightning Strike",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53041, SpellInfo {
        name: "[NPC: Incantation] Fortissax's Lightning Spear",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53042, SpellInfo {
        name: "[NPC: Incantation] Vyke's Dragonbolt",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53043, SpellInfo {
        name: "[NPC: Incantation] The Flame of Frenzy",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53044, SpellInfo {
        name: "[NPC: Incantation] Frenzied Burst",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53045, SpellInfo {
        name: "[NPC: Incantation] Howl of Shabriri (Variant)",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53110, SpellInfo {
        name: "[NPC: Incantation] Wrath of Gold 1",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53111, SpellInfo {
        name: "[NPC: Incantation] Golden Vow 1",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53120, SpellInfo {
        name: "[NPC: Sorcery] Magic Glintblade 1",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53160, SpellInfo {
        name: "[NPC: Sorcery] Glintstone Cometshard",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53161, SpellInfo {
        name: "[NPC: Sorcery] Founding Rain of Stars",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53162, SpellInfo {
        name: "[NPC: Sorcery] Shattering Crystal",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53163, SpellInfo {
        name: "[NPC: Sorcery] Crystal Release",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53180, SpellInfo {
        name: "[NPC: Incantation] Inescapable Frenzy (Variant)",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53181, SpellInfo {
        name: "[NPC: Incantation] Unendurable Frenzy (Variant)",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53182, SpellInfo {
        name: "[NPC: Incantation] Unendurable Frenzy (Variant)",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53183, SpellInfo {
        name: "[NPC: Incantation] Howl of Shabriri (Variant)",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53190, SpellInfo {
        name: "[NPC: Incantation] Litany of Proper Death",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53191, SpellInfo {
        name: "[NPC: Incantation] Beast Claw 1",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53192, SpellInfo {
        name: "[NPC: Incantation] Discus of Light",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53210, SpellInfo {
        name: "[NPC: Incantation] Wrath of Gold 2",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53240, SpellInfo {
        name: "[NPC: Sorcery] Comet",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53241, SpellInfo {
        name: "[NPC: Sorcery] Comet Azur",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53242, SpellInfo {
        name: "[NPC: Sorcery] Carian Phalanx",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53243, SpellInfo {
        name: "[NPC: Incantation] Black Flame Ritual",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53244, SpellInfo {
        name: "[NPC: Incantation] Triple Rings of Light 1",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53245, SpellInfo {
        name: "[NPC: Incantation] Law of Causality",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53246, SpellInfo {
        name: "[NPC: Sorcery] Rykard's Rancor",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53247, SpellInfo {
        name: "[NPC: Incantation] Bloodboon",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53248, SpellInfo {
        name: "[NPC: Incantation] Scarlet Aeonia",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53250, SpellInfo {
        name: "[NPC: Sorcery] Glintstone Pebble",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53251, SpellInfo {
        name: "[NPC: Sorcery] Magic Downpour 1",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53252, SpellInfo {
        name: "[NPC: Sorcery] Scholar's Armament",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53260, SpellInfo {
        name: "[NPC: Incantation] Protection of The Erdtree",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53261, SpellInfo {
        name: "[NPC: Incantation] Golden Vow 2",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53270, SpellInfo {
        name: "[NPC: Incantation] Agheel's Flame",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53271, SpellInfo {
        name: "[NPC: Incantation] Greyoll's Roar",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53290, SpellInfo {
        name: "[NPC: Sorcery] Rancorcall",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53291, SpellInfo {
        name: "[NPC: Sorcery] Fia's Mist",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53330, SpellInfo {
        name: "[NPC: Sorcery] Great Glintstone Shard",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53350, SpellInfo {
        name: "[NPC: Incantation] Triple Rings of Light 2",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53390, SpellInfo {
        name: "[NPC: Sorcery] Collapsing Stars",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53510, SpellInfo {
        name: "[NPC: Incantation] Heal",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53520, SpellInfo {
        name: "[NPC: Sorcery] Loretta's Greatbow 1",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53521, SpellInfo {
        name: "[NPC: Sorcery] Loretta's Greatbow 2",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53522, SpellInfo {
        name: "[NPC: Sorcery] Greatblade Phalanx",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53523, SpellInfo {
        name: "[NPC: Sorcery] Magic Downpour 2",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53524, SpellInfo {
        name: "[NPC: Sorcery] Carian Slicer 1",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53560, SpellInfo {
        name: "[NPC: Incantation] O, Flame!",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53561, SpellInfo {
        name: "[NPC: Incantation] Flame of The Fell God",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53590, SpellInfo {
        name: "[NPC: Sorcery] Carian Piercer",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53610, SpellInfo {
        name: "[NPC: Sorcery] Carian Slicer 2",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53630, SpellInfo {
        name: "[NPC: Incantation] Great Heal",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53720, SpellInfo {
        name: "[NPC: Incantation] Poison Mist",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53750, SpellInfo {
        name: "[NPC: Sorcery] Carian Phalanx",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53751, SpellInfo {
        name: "[NPC: Sorcery] Magic Glintblade 2",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53760, SpellInfo {
        name: "[NPC: Incantation] Tibia's Summons",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53770, SpellInfo {
        name: "[NPC: Sorcery] Great Glintstone Shard",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53800, SpellInfo {
        name: "[NPC: Sorcery] Crystal Burst",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53801, SpellInfo {
        name: "[NPC: Sorcery] Stars of Ruin",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53802, SpellInfo {
        name: "[NPC: Sorcery] Ambush Shard 1",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53803, SpellInfo {
        name: "[NPC: Sorcery] Night Comet 1",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53804, SpellInfo {
        name: "[NPC: Sorcery] Ambush Shard 2",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53805, SpellInfo {
        name: "[NPC: Sorcery] Night Comet 2",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53820, SpellInfo {
        name: "[NPC: Incantation] Catch Flame",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53821, SpellInfo {
        name: "[NPC: Incantation] Burn, O Flame!",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53822, SpellInfo {
        name: "[NPC: Incantation] Surge, O Flame!",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53830, SpellInfo {
        name: "[NPC: Incantation] Whirl, O Flame!",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53850, SpellInfo {
        name: "[NPC: Sorcery] Glintstone Icecrag",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53851, SpellInfo {
        name: "[NPC: Sorcery] Freezing Mist",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53852, SpellInfo {
        name: "[NPC: Sorcery] Briars of Sin",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53860, SpellInfo {
        name: "[NPC: Sorcery] Bloodflame Talons",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53861, SpellInfo {
        name: "[NPC: Sorcery] Swarm of Flies 2",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53900, SpellInfo {
        name: "[NPC: Sorcery] Pest Threads",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53910, SpellInfo {
        name: "[NPC: Incantation] Bestial Sling",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53911, SpellInfo {
        name: "[NPC: Incantation] Beast Claw 2",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(53912, SpellInfo {
        name: "[NPC: Incantation] Gurranq's Beast Claw",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 0,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2004300, SpellInfo {
        name: "Miriam's Vanishing",
        spell_type: SpellType::Sorcery,
        fp_cost: 9,
        slots: 1,
        int_req: 26,
        fai_req: 0,
    });
    map.insert(2004310, SpellInfo {
        name: "Glintblade Trio",
        spell_type: SpellType::Sorcery,
        fp_cost: 19,
        slots: 1,
        int_req: 28,
        fai_req: 0,
    });
    map.insert(2004320, SpellInfo {
        name: "Rellana's Twin Moons",
        spell_type: SpellType::Sorcery,
        fp_cost: 47,
        slots: 2,
        int_req: 72,
        fai_req: 0,
    });
    map.insert(2004500, SpellInfo {
        name: "Glintstone Nail",
        spell_type: SpellType::Sorcery,
        fp_cost: 10,
        slots: 1,
        int_req: 18,
        fai_req: 0,
    });
    map.insert(2004510, SpellInfo {
        name: "Glintstone Nails",
        spell_type: SpellType::Sorcery,
        fp_cost: 23,
        slots: 1,
        int_req: 32,
        fai_req: 0,
    });
    map.insert(2004700, SpellInfo {
        name: "Blades of Stone",
        spell_type: SpellType::Sorcery,
        fp_cost: 18,
        slots: 2,
        int_req: 48,
        fai_req: 0,
    });
    map.insert(2004710, SpellInfo {
        name: "Gravitational Missile",
        spell_type: SpellType::Sorcery,
        fp_cost: 18,
        slots: 1,
        int_req: 36,
        fai_req: 0,
    });
    map.insert(2004900, SpellInfo {
        name: "Mantle of Thorns",
        spell_type: SpellType::Sorcery,
        fp_cost: 9,
        slots: 1,
        int_req: 0,
        fai_req: 20,
    });
    map.insert(2004910, SpellInfo {
        name: "Impenetrable Thorns",
        spell_type: SpellType::Sorcery,
        fp_cost: 15,
        slots: 1,
        int_req: 0,
        fai_req: 24,
    });
    map.insert(2005000, SpellInfo {
        name: "Rings of Spectral Light",
        spell_type: SpellType::Sorcery,
        fp_cost: 14,
        slots: 1,
        int_req: 24,
        fai_req: 18,
    });
    map.insert(2006200, SpellInfo {
        name: "Vortex of Putrescence",
        spell_type: SpellType::Sorcery,
        fp_cost: 29,
        slots: 2,
        int_req: 32,
        fai_req: 26,
    });
    map.insert(2006210, SpellInfo {
        name: "Mass of Putrescence",
        spell_type: SpellType::Sorcery,
        fp_cost: 41,
        slots: 1,
        int_req: 28,
        fai_req: 22,
    });
    map.insert(2006300, SpellInfo {
        name: "Furious Blade of Ansbach",
        spell_type: SpellType::Incantation,
        fp_cost: 18,
        slots: 1,
        int_req: 0,
        fai_req: 19,
    });
    map.insert(2006400, SpellInfo {
        name: "Heal from Afar",
        spell_type: SpellType::Incantation,
        fp_cost: 45,
        slots: 1,
        int_req: 0,
        fai_req: 18,
    });
    map.insert(2006650, SpellInfo {
        name: "Aspects of the Crucible: Thorns",
        spell_type: SpellType::Incantation,
        fp_cost: 14,
        slots: 1,
        int_req: 0,
        fai_req: 27,
    });
    map.insert(2006660, SpellInfo {
        name: "Aspects of the Crucible: Bloom",
        spell_type: SpellType::Incantation,
        fp_cost: 23,
        slots: 1,
        int_req: 0,
        fai_req: 27,
    });
    map.insert(2006670, SpellInfo {
        name: "Minor Erdtree",
        spell_type: SpellType::Incantation,
        fp_cost: 30,
        slots: 2,
        int_req: 0,
        fai_req: 70,
    });
    map.insert(2006680, SpellInfo {
        name: "Land of Shadow",
        spell_type: SpellType::Incantation,
        fp_cost: 40,
        slots: 1,
        int_req: 0,
        fai_req: 58,
    });
    map.insert(2006690, SpellInfo {
        name: "Wrath from Afar",
        spell_type: SpellType::Incantation,
        fp_cost: 18,
        slots: 1,
        int_req: 0,
        fai_req: 34,
    });
    map.insert(2006700, SpellInfo {
        name: "Light of Miquella",
        spell_type: SpellType::Incantation,
        fp_cost: 48,
        slots: 2,
        int_req: 0,
        fai_req: 72,
    });
    map.insert(2006710, SpellInfo {
        name: "Multilayered Ring of Light",
        spell_type: SpellType::Incantation,
        fp_cost: 23,
        slots: 1,
        int_req: 0,
        fai_req: 36,
    });
    map.insert(2006800, SpellInfo {
        name: "Roar of Rugalea",
        spell_type: SpellType::Incantation,
        fp_cost: 17,
        slots: 1,
        int_req: 0,
        fai_req: 14,
    });
    map.insert(2006900, SpellInfo {
        name: "Knight's Lightning Spear",
        spell_type: SpellType::Incantation,
        fp_cost: 29,
        slots: 1,
        int_req: 0,
        fai_req: 36,
    });
    map.insert(2006910, SpellInfo {
        name: "Dragonbolt of Florissax",
        spell_type: SpellType::Incantation,
        fp_cost: 35,
        slots: 1,
        int_req: 0,
        fai_req: 52,
    });
    map.insert(2006920, SpellInfo {
        name: "Electrocharge",
        spell_type: SpellType::Incantation,
        fp_cost: 26,
        slots: 1,
        int_req: 0,
        fai_req: 30,
    });
    map.insert(2007000, SpellInfo {
        name: "Bayle's Tyranny",
        spell_type: SpellType::Incantation,
        fp_cost: 46,
        slots: 2,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2007010, SpellInfo {
        name: "Bayle's Flame Lightning",
        spell_type: SpellType::Incantation,
        fp_cost: 43,
        slots: 2,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2007020, SpellInfo {
        name: "Ghostflame Breath",
        spell_type: SpellType::Incantation,
        fp_cost: 36,
        slots: 1,
        int_req: 0,
        fai_req: 23,
    });
    map.insert(2007200, SpellInfo {
        name: "Rotten Butterflies",
        spell_type: SpellType::Incantation,
        fp_cost: 48,
        slots: 1,
        int_req: 0,
        fai_req: 33,
    });
    map.insert(2007210, SpellInfo {
        name: "Pest-Thread Spears",
        spell_type: SpellType::Incantation,
        fp_cost: 28,
        slots: 1,
        int_req: 0,
        fai_req: 26,
    });
    map.insert(2007300, SpellInfo {
        name: "Midra's Flame of Frenzy",
        spell_type: SpellType::Incantation,
        fp_cost: 22,
        slots: 2,
        int_req: 0,
        fai_req: 41,
    });
    map.insert(2007410, SpellInfo {
        name: "Fleeting Microcosm",
        spell_type: SpellType::Sorcery,
        fp_cost: 26,
        slots: 1,
        int_req: 42,
        fai_req: 0,
    });
    map.insert(2007420, SpellInfo {
        name: "Cherishing Fingers",
        spell_type: SpellType::Sorcery,
        fp_cost: 20,
        slots: 1,
        int_req: 36,
        fai_req: 0,
    });
    map.insert(2007600, SpellInfo {
        name: "Watchful Spirit",
        spell_type: SpellType::Incantation,
        fp_cost: 12,
        slots: 1,
        int_req: 0,
        fai_req: 26,
    });
    map.insert(2007700, SpellInfo {
        name: "Golden Arcs",
        spell_type: SpellType::Incantation,
        fp_cost: 12,
        slots: 1,
        int_req: 0,
        fai_req: 22,
    });
    map.insert(2007710, SpellInfo {
        name: "Giant Golden Arc",
        spell_type: SpellType::Incantation,
        fp_cost: 24,
        slots: 1,
        int_req: 0,
        fai_req: 34,
    });
    map.insert(2007720, SpellInfo {
        name: "Spira",
        spell_type: SpellType::Incantation,
        fp_cost: 10,
        slots: 2,
        int_req: 0,
        fai_req: 48,
    });
    map.insert(2007730, SpellInfo {
        name: "Divine Beast Tornado",
        spell_type: SpellType::Incantation,
        fp_cost: 24,
        slots: 1,
        int_req: 0,
        fai_req: 28,
    });
    map.insert(2007740, SpellInfo {
        name: "Divine Bird Feathers",
        spell_type: SpellType::Incantation,
        fp_cost: 3,
        slots: 1,
        int_req: 0,
        fai_req: 24,
    });
    map.insert(2007800, SpellInfo {
        name: "Fire Serpent",
        spell_type: SpellType::Incantation,
        fp_cost: 11,
        slots: 1,
        int_req: 0,
        fai_req: 16,
    });
    map.insert(2007810, SpellInfo {
        name: "Rain of Fire",
        spell_type: SpellType::Incantation,
        fp_cost: 27,
        slots: 1,
        int_req: 0,
        fai_req: 52,
    });
    map.insert(2007820, SpellInfo {
        name: "Messmer's Orb",
        spell_type: SpellType::Incantation,
        fp_cost: 31,
        slots: 2,
        int_req: 0,
        fai_req: 60,
    });
    map.insert(2050000, SpellInfo {
        name: "[NPC: Incantation] Dragonclaw",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050001, SpellInfo {
        name: "[NPC: Incantation] Greyoll's Roar",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 2,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050010, SpellInfo {
        name: "[NPC: Incantation]",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050011, SpellInfo {
        name: "[NPC: Incantation] Dragonbolt of Florissax",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050020, SpellInfo {
        name: "[NPC: Incantation] Magma Breath",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050021, SpellInfo {
        name: "[NPC: Incantation] Dragonmaw",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050030, SpellInfo {
        name: "[NPC: Incantation] Fire Serpent",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050040, SpellInfo {
        name: "[NPC: Incantation] Multilayered Ring of Light",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050050, SpellInfo {
        name: "[NPC: Incantation] Furious Blade of Ansbach",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050060, SpellInfo {
        name: "[NPC: Sorcery] Miriam's Vanishing",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050061, SpellInfo {
        name: "[NPC: Sorcery] Glintstone Nails",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050062, SpellInfo {
        name: "[NPC: Sorcery] Cherishing Fingers",
        spell_type: SpellType::Sorcery,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050063, SpellInfo {
        name: "[NPC: Sorcery] Fleeting Microcosm",
        spell_type: SpellType::Sorcery,
        fp_cost: 0,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050064, SpellInfo {
        name: "[NPC: Sorcery] Fleeting Microcosm",
        spell_type: SpellType::Sorcery,
        fp_cost: 0,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050070, SpellInfo {
        name: "[NPC: Incantation] Roar of Rugalea",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050080, SpellInfo {
        name: "[NPC: Incantation] Lord's Heal",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050081, SpellInfo {
        name: "[NPC: Incantation] Golden Vow",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050082, SpellInfo {
        name: "[NPC: Incantation] Discus of Light",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(2050090, SpellInfo {
        name: "[NPC: Incantation] The Flame of Frenzy",
        spell_type: SpellType::Incantation,
        fp_cost: 1,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map.insert(999999999, SpellInfo {
        name: "[NPC: Incantation] Golden Lightning Fortification",
        spell_type: SpellType::Incantation,
        fp_cost: 7,
        slots: 1,
        int_req: 0,
        fai_req: 0,
    });
    map
});

/// Get spell info by ID
pub fn get_spell(id: u32) -> Option<&'static SpellInfo> {
    SPELLS.get(&id)
}

/// Get spell name by ID, returns "Unknown Spell" if not found
pub fn get_spell_name(id: u32) -> &'static str {
    SPELLS.get(&id).map(|s| s.name).unwrap_or("Unknown Spell")
}
