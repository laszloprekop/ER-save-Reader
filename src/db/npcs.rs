// NPC tracking database
// Contains key NPCs with discovery/death flags and locations

use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcType {
    Merchant,
    QuestNpc,
    RoundtableNpc,
    Invader,
    Boss,
    Spirit,
}

#[derive(Debug, Clone)]
pub struct NpcInfo {
    pub id: u32,
    pub name: &'static str,
    pub npc_type: NpcType,
    pub discovery_flag: Option<u32>,
    pub death_flag: Option<u32>,
    pub location: &'static str,
    pub quest_flags: &'static [u32],
}

/// Key NPCs in Elden Ring
pub static NPCS: Lazy<HashMap<u32, NpcInfo>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Merchants
    map.insert(1000, NpcInfo {
        id: 1000,
        name: "Kalé",
        npc_type: NpcType::Merchant,
        discovery_flag: Some(1034500800),
        death_flag: Some(1034500801),
        location: "Church of Elleh",
        quest_flags: &[],
    });
    map.insert(1001, NpcInfo {
        id: 1001,
        name: "Merchant (Limgrave)",
        npc_type: NpcType::Merchant,
        discovery_flag: Some(1037450800),
        death_flag: Some(1037450801),
        location: "Coastal Cave",
        quest_flags: &[],
    });
    map.insert(1002, NpcInfo {
        id: 1002,
        name: "Isolated Merchant (Weeping Peninsula)",
        npc_type: NpcType::Merchant,
        discovery_flag: Some(1042360800),
        death_flag: Some(1042360801),
        location: "Weeping Peninsula",
        quest_flags: &[],
    });

    // Roundtable Hold NPCs
    map.insert(2000, NpcInfo {
        id: 2000,
        name: "Hewg",
        npc_type: NpcType::RoundtableNpc,
        discovery_flag: Some(11109600),
        death_flag: None,
        location: "Roundtable Hold",
        quest_flags: &[],
    });
    map.insert(2001, NpcInfo {
        id: 2001,
        name: "Roderika",
        npc_type: NpcType::RoundtableNpc,
        discovery_flag: Some(1034500850),
        death_flag: None,
        location: "Stormhill Shack → Roundtable Hold",
        quest_flags: &[3690, 3691, 3692, 3693],
    });
    map.insert(2002, NpcInfo {
        id: 2002,
        name: "Gideon Ofnir",
        npc_type: NpcType::RoundtableNpc,
        discovery_flag: Some(11109650),
        death_flag: Some(9150),
        location: "Roundtable Hold",
        quest_flags: &[],
    });
    map.insert(2003, NpcInfo {
        id: 2003,
        name: "Fia",
        npc_type: NpcType::RoundtableNpc,
        discovery_flag: Some(11109700),
        death_flag: Some(11109701),
        location: "Roundtable Hold",
        quest_flags: &[3800, 3801, 3802, 3803, 3804],
    });
    map.insert(2004, NpcInfo {
        id: 2004,
        name: "D, Hunter of the Dead",
        npc_type: NpcType::RoundtableNpc,
        discovery_flag: Some(1036440800),
        death_flag: Some(11109751),
        location: "Limgrave → Roundtable Hold",
        quest_flags: &[3820, 3821, 3822],
    });
    map.insert(2005, NpcInfo {
        id: 2005,
        name: "Corhyn",
        npc_type: NpcType::RoundtableNpc,
        discovery_flag: Some(11109800),
        death_flag: Some(11109801),
        location: "Roundtable Hold",
        quest_flags: &[3860, 3861, 3862],
    });

    // Major Quest NPCs
    map.insert(3000, NpcInfo {
        id: 3000,
        name: "Ranni the Witch",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(1034500900),
        death_flag: None,
        location: "Church of Elleh → Ranni's Rise",
        quest_flags: &[1034509200, 1034509201, 1034509202, 1034509203],
    });
    map.insert(3001, NpcInfo {
        id: 3001,
        name: "Blaidd",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(1044380800),
        death_flag: Some(1035500801),
        location: "Mistwood → Various",
        quest_flags: &[],
    });
    map.insert(3002, NpcInfo {
        id: 3002,
        name: "Alexander, Iron Fist",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(1039440800),
        death_flag: Some(13000851),
        location: "Stormhill → Crumbling Farum Azula",
        quest_flags: &[3640, 3641, 3642, 3643, 3644, 3645],
    });
    map.insert(3003, NpcInfo {
        id: 3003,
        name: "Millicent",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(1049370800),
        death_flag: Some(1051360801),
        location: "Church of the Plague → Haligtree",
        quest_flags: &[3700, 3701, 3702, 3703, 3704, 3705, 3706],
    });
    map.insert(3004, NpcInfo {
        id: 3004,
        name: "Melina",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(76400),
        death_flag: None,
        location: "Various Sites of Grace",
        quest_flags: &[],
    });
    map.insert(3005, NpcInfo {
        id: 3005,
        name: "White Mask Varré",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(1034450800),
        death_flag: Some(12010801),
        location: "First Step → Rose Church",
        quest_flags: &[3380, 3381, 3382, 3383, 3384],
    });
    map.insert(3006, NpcInfo {
        id: 3006,
        name: "Patches",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(31000800),
        death_flag: Some(31000801),
        location: "Murkwater Cave",
        quest_flags: &[3500, 3501, 3502],
    });
    map.insert(3007, NpcInfo {
        id: 3007,
        name: "Sellen",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(31170800),
        death_flag: Some(1035450801),
        location: "Waypoint Ruins",
        quest_flags: &[3580, 3581, 3582, 3583, 3584, 3585],
    });
    map.insert(3008, NpcInfo {
        id: 3008,
        name: "Hyetta",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(1036450800),
        death_flag: Some(35000801),
        location: "Lake-Facing Cliffs → Frenzied Flame Proscription",
        quest_flags: &[3720, 3721, 3722, 3723, 3724],
    });
    map.insert(3009, NpcInfo {
        id: 3009,
        name: "Nepheli Loux",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(10000800),
        death_flag: Some(10000801),
        location: "Stormveil Castle → Roundtable Hold",
        quest_flags: &[3460, 3461, 3462, 3463, 3464],
    });
    map.insert(3010, NpcInfo {
        id: 3010,
        name: "Kenneth Haight",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(1041380800),
        death_flag: Some(1041380801),
        location: "Limgrave",
        quest_flags: &[3440, 3441, 3442, 3443],
    });
    map.insert(3011, NpcInfo {
        id: 3011,
        name: "Diallos",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(11109850),
        death_flag: Some(1044390801),
        location: "Roundtable Hold → Volcano Manor",
        quest_flags: &[3540, 3541, 3542, 3543, 3544],
    });
    map.insert(3012, NpcInfo {
        id: 3012,
        name: "Boc the Seamster",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(1042380800),
        death_flag: Some(11109811),
        location: "Limgrave → Roundtable Hold",
        quest_flags: &[3520, 3521, 3522, 3523],
    });
    map.insert(3013, NpcInfo {
        id: 3013,
        name: "Goldmask",
        npc_type: NpcType::QuestNpc,
        discovery_flag: Some(1037510800),
        death_flag: Some(1051570801),
        location: "Various locations",
        quest_flags: &[3860, 3861, 3862, 3863, 3864],
    });

    // Invaders
    map.insert(4000, NpcInfo {
        id: 4000,
        name: "Recusant Henricus",
        npc_type: NpcType::Invader,
        discovery_flag: Some(1039390800),
        death_flag: Some(1039390801),
        location: "Limgrave",
        quest_flags: &[],
    });
    map.insert(4001, NpcInfo {
        id: 4001,
        name: "Bloody Finger Nerijus",
        npc_type: NpcType::Invader,
        discovery_flag: Some(1044380850),
        death_flag: Some(1044380851),
        location: "Murkwater Catacombs",
        quest_flags: &[],
    });
    map.insert(4002, NpcInfo {
        id: 4002,
        name: "Anastasia, Tarnished-Eater",
        npc_type: NpcType::Invader,
        discovery_flag: Some(1043330800),
        death_flag: Some(1043330801),
        location: "Various (3 encounters)",
        quest_flags: &[],
    });
    map.insert(4003, NpcInfo {
        id: 4003,
        name: "Edgar the Revenger",
        npc_type: NpcType::Invader,
        discovery_flag: Some(1043340800),
        death_flag: Some(1043340801),
        location: "Revenger's Shack",
        quest_flags: &[],
    });
    map.insert(4004, NpcInfo {
        id: 4004,
        name: "Juno Hoslow",
        npc_type: NpcType::Invader,
        discovery_flag: Some(1051570800),
        death_flag: Some(1051570801),
        location: "Mountaintops of the Giants",
        quest_flags: &[],
    });

    // Spirit Summons (for Ranni's quest)
    map.insert(5000, NpcInfo {
        id: 5000,
        name: "Seluvis",
        npc_type: NpcType::Spirit,
        discovery_flag: Some(1034509250),
        death_flag: Some(1034509251),
        location: "Seluvis's Rise",
        quest_flags: &[],
    });
    map.insert(5001, NpcInfo {
        id: 5001,
        name: "Iji",
        npc_type: NpcType::Spirit,
        discovery_flag: Some(1034509260),
        death_flag: Some(1034509261),
        location: "Road to the Manor",
        quest_flags: &[],
    });

    map
});

/// Get NPC info by ID
pub fn get_npc(id: u32) -> Option<&'static NpcInfo> {
    NPCS.get(&id)
}

/// Get all NPCs of a specific type
pub fn get_npcs_by_type(npc_type: NpcType) -> Vec<&'static NpcInfo> {
    NPCS.values()
        .filter(|n| n.npc_type == npc_type)
        .collect()
}

/// Get all merchants
pub fn get_merchants() -> Vec<&'static NpcInfo> {
    get_npcs_by_type(NpcType::Merchant)
}

/// Get all quest NPCs
pub fn get_quest_npcs() -> Vec<&'static NpcInfo> {
    get_npcs_by_type(NpcType::QuestNpc)
}

/// Get all Roundtable Hold NPCs
pub fn get_roundtable_npcs() -> Vec<&'static NpcInfo> {
    get_npcs_by_type(NpcType::RoundtableNpc)
}
