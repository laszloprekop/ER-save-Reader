//! Quest Chain Database
//!
//! Contains quest chains for main story progression, NPC questlines, and faction quests.
//! Flags are sourced from EMEVD scripts where available, otherwise from community data.

/// A quest step with its event flag
#[derive(Debug, Clone)]
pub struct QuestStep {
    pub name: &'static str,
    pub flag_id: u32,
    pub description: &'static str,
    /// true = verified from EMEVD scripts, false = community-sourced
    pub verified: bool,
}

/// A quest chain with multiple steps
#[derive(Debug, Clone)]
pub struct QuestChain {
    pub id: u32,
    pub name: &'static str,
    pub category: QuestCategory,
    /// NPC name for NPC questlines, None for main story/factions
    pub npc_name: Option<&'static str>,
    pub steps: &'static [QuestStep],
}

/// Quest chain categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuestCategory {
    MainStory,
    NpcQuestline,
    Faction,
    Optional,
}

impl QuestCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            QuestCategory::MainStory => "Main Story",
            QuestCategory::NpcQuestline => "NPC Questline",
            QuestCategory::Faction => "Faction",
            QuestCategory::Optional => "Optional",
        }
    }

    pub fn all_categories() -> &'static [QuestCategory] {
        &[
            QuestCategory::MainStory,
            QuestCategory::NpcQuestline,
            QuestCategory::Faction,
            QuestCategory::Optional,
        ]
    }
}

/// All quest chains in Elden Ring
pub static QUEST_CHAINS: &[QuestChain] = &[
    // ===== MAIN STORY =====
    QuestChain {
        id: 1,
        name: "Main Story - Limgrave",
        category: QuestCategory::MainStory,
        npc_name: None,
        steps: &[
            QuestStep { name: "Defeat Margit", flag_id: 1035420800, description: "Defeat Margit, the Fell Omen at Stormhill", verified: true },
            QuestStep { name: "Defeat Godrick", flag_id: 10000800, description: "Defeat Godrick the Grafted in Stormveil Castle", verified: true },
        ],
    },
    QuestChain {
        id: 2,
        name: "Main Story - Liurnia",
        category: QuestCategory::MainStory,
        npc_name: None,
        steps: &[
            QuestStep { name: "Defeat Rennala", flag_id: 14000800, description: "Defeat Rennala, Queen of the Full Moon", verified: true },
        ],
    },
    QuestChain {
        id: 3,
        name: "Main Story - Caelid",
        category: QuestCategory::MainStory,
        npc_name: None,
        steps: &[
            QuestStep { name: "Defeat Radahn", flag_id: 12010800, description: "Defeat Starscourge Radahn at Redmane Castle", verified: true },
        ],
    },
    QuestChain {
        id: 4,
        name: "Main Story - Altus Plateau",
        category: QuestCategory::MainStory,
        npc_name: None,
        steps: &[
            QuestStep { name: "Reach Altus Plateau", flag_id: 118, description: "Use the Grand Lift of Dectus or cave route", verified: false },
            QuestStep { name: "Defeat Morgott", flag_id: 11000800, description: "Defeat Morgott, the Omen King in Leyndell", verified: true },
        ],
    },
    QuestChain {
        id: 5,
        name: "Main Story - Mountaintops",
        category: QuestCategory::MainStory,
        npc_name: None,
        steps: &[
            QuestStep { name: "Defeat Fire Giant", flag_id: 1052520800, description: "Defeat the Fire Giant to unlock Forge", verified: true },
            QuestStep { name: "Burn the Erdtree", flag_id: 9401, description: "Commit the cardinal sin at the Forge", verified: false },
        ],
    },
    QuestChain {
        id: 6,
        name: "Main Story - Farum Azula",
        category: QuestCategory::MainStory,
        npc_name: None,
        steps: &[
            QuestStep { name: "Defeat Godskin Duo", flag_id: 13000850, description: "Defeat Godskin Duo", verified: true },
            QuestStep { name: "Defeat Maliketh", flag_id: 13000800, description: "Defeat Maliketh, the Black Blade", verified: true },
        ],
    },
    QuestChain {
        id: 7,
        name: "Main Story - Endgame",
        category: QuestCategory::MainStory,
        npc_name: None,
        steps: &[
            QuestStep { name: "Defeat Sir Gideon", flag_id: 11050850, description: "Defeat Sir Gideon Ofnir, the All-Knowing", verified: true },
            QuestStep { name: "Defeat Hoarah Loux", flag_id: 11050800, description: "Defeat Hoarah Loux, Warrior", verified: true },
            QuestStep { name: "Defeat Elden Beast", flag_id: 19000800, description: "Defeat Radagon and the Elden Beast", verified: true },
        ],
    },

    // ===== NPC QUESTLINES =====
    QuestChain {
        id: 100,
        name: "Ranni's Questline",
        category: QuestCategory::NpcQuestline,
        npc_name: Some("Ranni"),
        steps: &[
            QuestStep { name: "Meet Ranni at Church", flag_id: 1044389255, description: "Meet Ranni at Church of Elleh (night)", verified: false },
            QuestStep { name: "Visit Ranni's Rise", flag_id: 1034509400, description: "Speak with Ranni at Ranni's Rise", verified: false },
            QuestStep { name: "Obtain Fingerslayer Blade", flag_id: 12070800, description: "Get Fingerslayer Blade from Nokron", verified: false },
            QuestStep { name: "Give Blade to Ranni", flag_id: 1034509406, description: "Give the blade to Ranni", verified: false },
            QuestStep { name: "Get Carian Inverted Statue", flag_id: 1034509407, description: "Receive inverted statue from Ranni", verified: false },
            QuestStep { name: "Reach Lake of Rot", flag_id: 12010800, description: "Navigate through Lake of Rot", verified: false },
            QuestStep { name: "Defeat Astel", flag_id: 12040800, description: "Defeat Astel, Naturalborn of the Void", verified: true },
            QuestStep { name: "Reach Moonlight Altar", flag_id: 1034519400, description: "Reach Moonlight Altar via coffin", verified: false },
            QuestStep { name: "Obtain Dark Moon Ring", flag_id: 1034519401, description: "Get Dark Moon Ring", verified: false },
            QuestStep { name: "Complete Questline", flag_id: 1034509409, description: "Wear ring and marry Ranni", verified: false },
        ],
    },
    QuestChain {
        id: 101,
        name: "Millicent's Questline",
        category: QuestCategory::NpcQuestline,
        npc_name: Some("Millicent"),
        steps: &[
            QuestStep { name: "Meet Millicent", flag_id: 1049389200, description: "Find Millicent in Church of the Plague", verified: false },
            QuestStep { name: "Obtain Unalloyed Gold Needle", flag_id: 1049389203, description: "Get needle from Commander O'Neil", verified: false },
            QuestStep { name: "Give Needle to Gowry", flag_id: 1049389206, description: "Give needle to Gowry for repair", verified: false },
            QuestStep { name: "Cure Millicent", flag_id: 1049389207, description: "Give repaired needle to Millicent", verified: false },
            QuestStep { name: "Meet at Erdtree-Gazing Hill", flag_id: 1039449200, description: "Find Millicent at Erdtree-Gazing Hill", verified: false },
            QuestStep { name: "Give Prosthesis", flag_id: 1039449201, description: "Give Valkyrie's Prosthesis to Millicent", verified: false },
            QuestStep { name: "Meet at Windmill Village", flag_id: 1042559200, description: "Find Millicent at Dominula, Windmill Village", verified: false },
            QuestStep { name: "Help at Prayer Room", flag_id: 15000910, description: "Help Millicent in Haligtree", verified: false },
        ],
    },
    QuestChain {
        id: 102,
        name: "Alexander's Questline",
        category: QuestCategory::NpcQuestline,
        npc_name: Some("Alexander"),
        steps: &[
            QuestStep { name: "Free Alexander (Limgrave)", flag_id: 1042369250, description: "Help Alexander out of the ground in Limgrave", verified: false },
            QuestStep { name: "Meet at Radahn Festival", flag_id: 1051369200, description: "Speak with Alexander at Redmane Castle", verified: false },
            QuestStep { name: "Summon for Radahn", flag_id: 1051369201, description: "Summon Alexander during Radahn fight", verified: false },
            QuestStep { name: "Free Alexander (Liurnia)", flag_id: 1038469200, description: "Help Alexander out at Artists' Shack", verified: false },
            QuestStep { name: "Meet at Mt. Gelmir", flag_id: 1036509200, description: "Find Alexander in lava at Mt. Gelmir", verified: false },
            QuestStep { name: "Duel Alexander", flag_id: 13000890, description: "Defeat Alexander in Crumbling Farum Azula", verified: true },
        ],
    },
    QuestChain {
        id: 103,
        name: "Nepheli Loux Questline",
        category: QuestCategory::NpcQuestline,
        npc_name: Some("Nepheli Loux"),
        steps: &[
            QuestStep { name: "Meet Nepheli", flag_id: 10009200, description: "Meet Nepheli in Stormveil Castle", verified: false },
            QuestStep { name: "Summon for Godrick", flag_id: 10009201, description: "Can summon Nepheli for Godrick fight", verified: false },
            QuestStep { name: "Speak at Roundtable", flag_id: 11109200, description: "Speak with Nepheli at Roundtable Hold", verified: false },
            QuestStep { name: "Give Stormhawk King", flag_id: 11109204, description: "Give Stormhawk King ashes to Nepheli", verified: false },
            QuestStep { name: "Complete Questline", flag_id: 11109205, description: "Nepheli becomes ruler of Stormveil", verified: false },
        ],
    },
    QuestChain {
        id: 104,
        name: "Fia's Questline",
        category: QuestCategory::NpcQuestline,
        npc_name: Some("Fia"),
        steps: &[
            QuestStep { name: "Meet Fia", flag_id: 11109100, description: "Meet Fia at Roundtable Hold", verified: false },
            QuestStep { name: "Receive Baldachin's Blessing", flag_id: 11109101, description: "Receive blessing from Fia's embrace", verified: false },
            QuestStep { name: "Speak about D", flag_id: 11109103, description: "Learn about Fia and D's conflict", verified: false },
            QuestStep { name: "Find Fia in Deeproot", flag_id: 12030900, description: "Find Fia at Prince of Death's Throne", verified: false },
            QuestStep { name: "Defeat Lichdragon", flag_id: 12030850, description: "Defeat Lichdragon Fortissax", verified: true },
            QuestStep { name: "Obtain Mending Rune", flag_id: 12030901, description: "Get Mending Rune of the Death-Prince", verified: false },
        ],
    },
    QuestChain {
        id: 105,
        name: "Goldmask & Corhyn Questline",
        category: QuestCategory::NpcQuestline,
        npc_name: Some("Goldmask"),
        steps: &[
            QuestStep { name: "Meet Corhyn", flag_id: 11109050, description: "Meet Brother Corhyn at Roundtable Hold", verified: false },
            QuestStep { name: "Find Goldmask", flag_id: 1039449050, description: "Find Goldmask on broken bridge in Altus", verified: false },
            QuestStep { name: "Tell Corhyn location", flag_id: 11109051, description: "Tell Corhyn about Goldmask's location", verified: false },
            QuestStep { name: "Show Law of Regression", flag_id: 11009050, description: "Cast Law of Regression at statue in Leyndell", verified: false },
            QuestStep { name: "Report to Goldmask", flag_id: 11009051, description: "Tell Goldmask about Radagon's secret", verified: false },
            QuestStep { name: "Obtain Mending Rune", flag_id: 11059050, description: "Get Mending Rune of Perfect Order after Maliketh", verified: false },
        ],
    },
    QuestChain {
        id: 106,
        name: "Hyetta Questline",
        category: QuestCategory::NpcQuestline,
        npc_name: Some("Hyetta"),
        steps: &[
            QuestStep { name: "Meet at Lake-Facing Cliffs", flag_id: 1038419200, description: "Meet Hyetta near Lake-Facing Cliffs", verified: false },
            QuestStep { name: "Give First Grape", flag_id: 1038419201, description: "Give Shabriri Grape to Hyetta", verified: false },
            QuestStep { name: "Meet at Purified Ruins", flag_id: 1037459200, description: "Find Hyetta at Purified Ruins", verified: false },
            QuestStep { name: "Give Second Grape", flag_id: 1037459201, description: "Give second Shabriri Grape", verified: false },
            QuestStep { name: "Meet at Gate Town Bridge", flag_id: 1039409200, description: "Find Hyetta near Gate Town Bridge", verified: false },
            QuestStep { name: "Give Fingerprint Grape", flag_id: 1039409201, description: "Give Fingerprint Grape to Hyetta", verified: false },
            QuestStep { name: "Meet at Frenzied Flame", flag_id: 35009200, description: "Find Hyetta at Frenzied Flame Proscription", verified: false },
        ],
    },
    QuestChain {
        id: 107,
        name: "Varre's Questline",
        category: QuestCategory::NpcQuestline,
        npc_name: Some("White Mask Varre"),
        steps: &[
            QuestStep { name: "Meet at First Step", flag_id: 1042369200, description: "Meet Varre at The First Step", verified: false },
            QuestStep { name: "Speak after Godrick", flag_id: 11109155, description: "Speak to Varre at Rose Church after defeating a shardbearer", verified: false },
            QuestStep { name: "Receive Festering Fingers", flag_id: 11109156, description: "Receive Festering Bloody Fingers from Varre", verified: false },
            QuestStep { name: "Invade 3 Times", flag_id: 11109159, description: "Use Festering Fingers to invade 3 times", verified: false },
            QuestStep { name: "Receive Cloth", flag_id: 11109160, description: "Receive Lord of Blood's Favor", verified: false },
            QuestStep { name: "Soak Cloth", flag_id: 11109161, description: "Soak cloth in maiden's blood", verified: false },
            QuestStep { name: "Get Pureblood Medal", flag_id: 11109163, description: "Receive Pureblood Knight's Medal", verified: false },
        ],
    },
    QuestChain {
        id: 108,
        name: "Sellen's Questline",
        category: QuestCategory::NpcQuestline,
        npc_name: Some("Sorceress Sellen"),
        steps: &[
            QuestStep { name: "Meet at Waypoint Ruins", flag_id: 1044369200, description: "Meet Sellen at Waypoint Ruins cellar", verified: false },
            QuestStep { name: "Become Apprentice", flag_id: 1044369201, description: "Accept Sellen as your teacher", verified: false },
            QuestStep { name: "Find Azur", flag_id: 1036549200, description: "Find Primeval Sorcerer Azur at Mt. Gelmir", verified: false },
            QuestStep { name: "Return to Sellen", flag_id: 1044369203, description: "Tell Sellen about Azur", verified: false },
            QuestStep { name: "Find Sellen's Body", flag_id: 1035449200, description: "Find Sellen's real body in Weeping Peninsula", verified: false },
            QuestStep { name: "Give Primal Glintstone", flag_id: 1044369205, description: "Transplant Sellen's soul", verified: false },
            QuestStep { name: "Help in Academy", flag_id: 14009209, description: "Help Sellen in the Academy", verified: false },
        ],
    },
    QuestChain {
        id: 109,
        name: "Blaidd's Questline",
        category: QuestCategory::NpcQuestline,
        npc_name: Some("Blaidd"),
        steps: &[
            QuestStep { name: "Meet at Mistwood", flag_id: 1044369300, description: "Hear howling in Mistwood Ruins", verified: false },
            QuestStep { name: "Learn Gesture", flag_id: 11109300, description: "Learn finger snap from Merchant Kalé", verified: false },
            QuestStep { name: "Meet Blaidd", flag_id: 1044369301, description: "Snap fingers at ruins to meet Blaidd", verified: false },
            QuestStep { name: "Hunt Darriwil", flag_id: 1044369302, description: "Defeat Bloodhound Knight Darriwil together", verified: false },
            QuestStep { name: "Meet at Siofra", flag_id: 12080300, description: "Meet Blaidd in Siofra River", verified: false },
            QuestStep { name: "Attend Radahn Festival", flag_id: 1051369300, description: "Find Blaidd at Redmane Castle", verified: false },
            QuestStep { name: "Final Encounter", flag_id: 1034509300, description: "Face Blaidd at Ranni's Rise after her quest", verified: false },
        ],
    },

    // ===== FACTIONS =====
    QuestChain {
        id: 200,
        name: "Volcano Manor Contracts",
        category: QuestCategory::Faction,
        npc_name: None,
        steps: &[
            QuestStep { name: "Join Volcano Manor", flag_id: 16009200, description: "Accept invitation from Tanith", verified: false },
            QuestStep { name: "First Contract: Old Knight Istvan", flag_id: 16009201, description: "Kill Old Knight Istvan at Stormhill", verified: false },
            QuestStep { name: "Second Contract: Rileigh", flag_id: 16009202, description: "Kill Rileigh the Idle at Altus Plateau", verified: false },
            QuestStep { name: "Third Contract: Juno Hoslow", flag_id: 16009203, description: "Kill Juno Hoslow at Mountaintops", verified: false },
            QuestStep { name: "Speak to Tanith", flag_id: 16009204, description: "Report to Tanith after contracts", verified: false },
            QuestStep { name: "Defeat Rykard", flag_id: 16000800, description: "Face Rykard, Lord of Blasphemy", verified: true },
        ],
    },
    QuestChain {
        id: 201,
        name: "Roundtable Hold Invasion",
        category: QuestCategory::Faction,
        npc_name: None,
        steps: &[
            QuestStep { name: "Ensha Attacks", flag_id: 11109500, description: "Get attacked by Ensha after obtaining half of medallion", verified: false },
            QuestStep { name: "Defeat Ensha", flag_id: 11109501, description: "Kill Ensha in Roundtable Hold", verified: false },
        ],
    },

    // ===== OPTIONAL =====
    QuestChain {
        id: 300,
        name: "Haligtree Secret",
        category: QuestCategory::Optional,
        npc_name: None,
        steps: &[
            QuestStep { name: "Obtain Left Medallion", flag_id: 1039539200, description: "Get Secret Haligtree Medallion (Left) from Village of the Albinaurics", verified: false },
            QuestStep { name: "Obtain Right Medallion", flag_id: 15009200, description: "Get Secret Haligtree Medallion (Right) from Castle Sol", verified: false },
            QuestStep { name: "Use Grand Lift", flag_id: 15009201, description: "Use Grand Lift of Rold with secret medallion", verified: false },
            QuestStep { name: "Navigate Liturgical Town", flag_id: 15009202, description: "Navigate through Ordina, Liturgical Town puzzle", verified: false },
            QuestStep { name: "Reach Haligtree", flag_id: 15009203, description: "Enter Miquella's Haligtree", verified: false },
            QuestStep { name: "Defeat Malenia", flag_id: 15000800, description: "Defeat Malenia, Blade of Miquella", verified: true },
        ],
    },
    QuestChain {
        id: 301,
        name: "Mohgwyn Palace",
        category: QuestCategory::Optional,
        npc_name: None,
        steps: &[
            QuestStep { name: "Access via Varre", flag_id: 11109163, description: "Use Pureblood Knight's Medal from Varre", verified: false },
            QuestStep { name: "Access via Waygate", flag_id: 12059200, description: "Use Waygate in Consecrated Snowfield", verified: false },
            QuestStep { name: "Defeat Mohg", flag_id: 12050800, description: "Defeat Mohg, Lord of Blood", verified: true },
        ],
    },
    QuestChain {
        id: 302,
        name: "Dragonlord Placidusax",
        category: QuestCategory::Optional,
        npc_name: None,
        steps: &[
            QuestStep { name: "Find Hidden Path", flag_id: 13009200, description: "Find lie-down spot in Crumbling Farum Azula", verified: false },
            QuestStep { name: "Defeat Placidusax", flag_id: 13000830, description: "Defeat Dragonlord Placidusax", verified: true },
        ],
    },
    QuestChain {
        id: 303,
        name: "Frenzied Flame",
        category: QuestCategory::Optional,
        npc_name: None,
        steps: &[
            QuestStep { name: "Reach Cathedral", flag_id: 35009100, description: "Find Cathedral of the Forsaken beneath Leyndell", verified: false },
            QuestStep { name: "Navigate Platforming", flag_id: 35009101, description: "Descend through the secret pit", verified: false },
            QuestStep { name: "Embrace Flame", flag_id: 35009102, description: "Open door and inherit Frenzied Flame", verified: false },
        ],
    },
];

/// Get quest chains by category
pub fn get_chains_by_category(category: QuestCategory) -> Vec<&'static QuestChain> {
    QUEST_CHAINS.iter().filter(|c| c.category == category).collect()
}

/// Get quest chains by NPC name
pub fn get_chains_by_npc(npc_name: &str) -> Vec<&'static QuestChain> {
    QUEST_CHAINS.iter().filter(|c| c.npc_name == Some(npc_name)).collect()
}

/// Get all unique NPC names
pub fn get_all_npcs() -> Vec<&'static str> {
    let mut npcs: Vec<_> = QUEST_CHAINS.iter()
        .filter_map(|c| c.npc_name)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    npcs.sort();
    npcs
}

/// Get quest chain by ID
pub fn get_chain_by_id(id: u32) -> Option<&'static QuestChain> {
    QUEST_CHAINS.iter().find(|c| c.id == id)
}
