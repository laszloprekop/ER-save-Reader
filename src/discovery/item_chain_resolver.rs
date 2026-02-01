//! Item Chain Resolver Module
//!
//! Resolves the chain of event flags associated with obtaining an inventory item.
//! Instead of mapping item → single flag, we spider through the network of connected
//! flags to find which ones are actually detectable in the save file.
//!
//! ## Approach
//!
//! 1. Start with inventory item (item_id)
//! 2. Find associated flags through multiple sources:
//!    - world_pickups: pickup event flags
//!    - BOSS_DEFEAT_CHAINS: remembrances, great runes
//!    - event_graph: EMEVD triggers and dependencies
//! 3. Traverse the chain to collect all related flags
//! 4. Check which flags are SET in the save file
//! 5. Identify the "blocking flag" that prevents re-obtaining the item
//!
//! ## Use Cases
//!
//! - Inventory verification: Find a detectable flag for any item
//! - Completion tracking: Identify what gates item acquisition
//! - Debug: Understand why an item's flag isn't detected

use std::collections::{HashMap, HashSet};

use crate::db::pickup_flags::{get_flag_offset, is_flag_set};
use super::chain_data::{BOSS_DEFEAT_CHAINS, BossDefeatChain};
use super::event_graph::EventGraph;

// ============================================================================
// TYPES
// ============================================================================

/// A resolved chain of flags for an inventory item
#[derive(Debug, Clone)]
pub struct ItemFlagChain {
    /// The item ID this chain is for
    pub item_id: u32,
    /// Item name for display
    pub item_name: String,
    /// All flags in the chain, with their roles
    pub chain_flags: Vec<ChainFlag>,
    /// The best flag to use for verification (has formula + is set when item obtained)
    pub verification_flag: Option<u32>,
    /// The blocking flag (prevents re-obtaining the item)
    pub blocking_flag: Option<u32>,
    /// Chain type for categorization
    pub chain_type: ChainType,
}

/// A flag in the chain with its role
#[derive(Debug, Clone)]
pub struct ChainFlag {
    pub flag_id: u32,
    pub role: FlagRole,
    pub source: FlagSource,
    pub has_formula: bool,
    pub is_set: Option<bool>,
    pub notes: String,
}

/// The role a flag plays in the acquisition chain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagRole {
    /// Boss must be defeated to get item
    BossDefeat,
    /// Item pickup/acquisition event
    ItemPickup,
    /// Item possession tracking
    ItemPossession,
    /// Item consumption (used at shop, etc.)
    ItemConsumption,
    /// Enables access to item location
    AreaAccess,
    /// Recipe/skill unlock
    Unlock,
    /// Unknown role
    Unknown,
}

impl FlagRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlagRole::BossDefeat => "Boss Defeat",
            FlagRole::ItemPickup => "Item Pickup",
            FlagRole::ItemPossession => "Possession",
            FlagRole::ItemConsumption => "Consumption",
            FlagRole::AreaAccess => "Area Access",
            FlagRole::Unlock => "Unlock",
            FlagRole::Unknown => "Unknown",
        }
    }
}

/// Where we found the flag association
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagSource {
    BossDefeatChain,
    WorldPickups,
    EventGraph,
    ManualMapping,
}

impl FlagSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlagSource::BossDefeatChain => "Boss Chain",
            FlagSource::WorldPickups => "World Pickups",
            FlagSource::EventGraph => "EMEVD Graph",
            FlagSource::ManualMapping => "Manual",
        }
    }
}

/// The type of chain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainType {
    Remembrance,
    GreatRune,
    WorldPickup,
    CatacombReward,
    BossWeapon,
    KeyItem,
    Unknown,
}

// ============================================================================
// ITEM TO FLAG MAPPINGS (known associations)
// ============================================================================

/// Manual mapping of item IDs to known flag associations
/// Format: (item_id, [(flag_id, role, notes)])
static KNOWN_ITEM_FLAGS: &[(u32, &[(u32, FlagRole, &str)])] = &[
    // Remembrances - item_id maps to boss defeat which grants the item
    (2950, &[(171, FlagRole::BossDefeat, "Godrick defeat grants Remembrance of the Grafted")]),
    (2951, &[(173, FlagRole::BossDefeat, "Radahn defeat grants Remembrance of the Starscourge")]),
    (2952, &[(175, FlagRole::BossDefeat, "Morgott defeat grants Remembrance of the Omen King")]),
    (2953, &[(174, FlagRole::BossDefeat, "Rykard defeat grants Remembrance of the Blasphemous")]),
    (2954, &[(177, FlagRole::BossDefeat, "Malenia defeat grants Remembrance of the Rot Goddess")]),
    (2955, &[(176, FlagRole::BossDefeat, "Mohg defeat grants Remembrance of the Blood Lord")]),
    (2956, &[(178, FlagRole::BossDefeat, "Maliketh defeat grants Remembrance of the Black Blade")]),
    (2957, &[(179, FlagRole::BossDefeat, "Hoarah Loux defeat grants Remembrance")]),
    (2958, &[(9108, FlagRole::BossDefeat, "Dragonlord defeat - flag needs verification")]),
    (2959, &[(172, FlagRole::BossDefeat, "Rennala defeat grants Remembrance of Full Moon Queen")]),
    (2960, &[(9110, FlagRole::BossDefeat, "Lichdragon defeat - flag needs verification")]),
    (2961, &[(9111, FlagRole::BossDefeat, "Fire Giant defeat - flag needs verification")]),
    (2962, &[(9112, FlagRole::BossDefeat, "Regal Ancestor defeat - flag needs verification")]),
    (2963, &[(180, FlagRole::BossDefeat, "Elden Beast defeat grants Elden Remembrance")]),
    (2964, &[(9114, FlagRole::BossDefeat, "Naturalborn defeat - flag needs verification")]),

    // Great Runes - same boss defeat flags
    (8148, &[(171, FlagRole::BossDefeat, "Godrick's Great Rune from defeating Godrick")]),
    (8149, &[(173, FlagRole::BossDefeat, "Radahn's Great Rune from defeating Radahn")]),
    (8150, &[(175, FlagRole::BossDefeat, "Morgott's Great Rune from defeating Morgott")]),
    (8151, &[(174, FlagRole::BossDefeat, "Rykard's Great Rune from defeating Rykard")]),
    (8152, &[(176, FlagRole::BossDefeat, "Mohg's Great Rune from defeating Mohg")]),
    (8153, &[(177, FlagRole::BossDefeat, "Malenia's Great Rune from defeating Malenia")]),
    (10080, &[(172, FlagRole::BossDefeat, "Great Rune of the Unborn from defeating Rennala")]),

    // Key progression items
    (8135, &[(60420, FlagRole::ItemPickup, "Rold Medallion pickup")]),
    (8175, &[(60431, FlagRole::ItemPickup, "Haligtree Secret Medallion (Left)")]),
    (8176, &[(60430, FlagRole::ItemPickup, "Haligtree Secret Medallion (Right)")]),
    (8100, &[(60100, FlagRole::ItemPickup, "Crafting Kit from Kale")]),
    (8900, &[(60130, FlagRole::ItemPickup, "Whetstone Knife pickup")]),
];

// ============================================================================
// CHAIN RESOLVER
// ============================================================================

/// Service for resolving item-to-flag chains
pub struct ItemChainResolver {
    /// Known item-to-flag mappings (loaded from statics)
    known_mappings: HashMap<u32, Vec<(u32, FlagRole, String)>>,
    /// Boss defeat chains (from chain_data)
    boss_chains: HashMap<u32, &'static BossDefeatChain>,
    /// Remembrance item_id to boss chain mapping
    remembrance_to_boss: HashMap<u32, u32>,
}

impl ItemChainResolver {
    /// Create a new resolver
    pub fn new() -> Self {
        let mut known_mappings = HashMap::new();
        for (item_id, flags) in KNOWN_ITEM_FLAGS {
            let entries: Vec<_> = flags
                .iter()
                .map(|(f, r, n)| (*f, *r, n.to_string()))
                .collect();
            known_mappings.insert(*item_id, entries);
        }

        // Index boss chains by defeat flag
        let mut boss_chains = HashMap::new();
        for chain in BOSS_DEFEAT_CHAINS {
            boss_chains.insert(chain.defeat_flag, chain);
        }

        // Map remembrance item_ids to their boss defeat flags
        // Remembrance item IDs: 2950-2964 correspond to bosses
        let remembrance_to_boss = HashMap::from([
            (2950, 171u32), // Godrick
            (2951, 173),    // Radahn
            (2952, 175),    // Morgott
            (2953, 174),    // Rykard
            (2954, 177),    // Malenia
            (2955, 176),    // Mohg
            (2956, 178),    // Maliketh
            (2957, 179),    // Hoarah Loux
            (2958, 178),    // Dragonlord (placeholder)
            (2959, 172),    // Rennala
            (2963, 180),    // Elden Beast
        ]);

        Self {
            known_mappings,
            boss_chains,
            remembrance_to_boss,
        }
    }

    /// Resolve the flag chain for an item
    pub fn resolve_chain(
        &self,
        item_id: u32,
        item_name: &str,
        event_flags: Option<&[u8]>,
        event_graph: Option<&EventGraph>,
    ) -> ItemFlagChain {
        let mut chain_flags = Vec::new();
        let mut chain_type = ChainType::Unknown;

        // 1. Check known manual mappings
        if let Some(mappings) = self.known_mappings.get(&item_id) {
            for (flag_id, role, notes) in mappings {
                let has_formula = get_flag_offset(*flag_id).is_some();
                let is_set = event_flags.map(|ef| is_flag_set(ef, *flag_id));

                chain_flags.push(ChainFlag {
                    flag_id: *flag_id,
                    role: *role,
                    source: FlagSource::ManualMapping,
                    has_formula,
                    is_set,
                    notes: notes.clone(),
                });

                // Determine chain type from role
                if *role == FlagRole::BossDefeat {
                    // Check if it's a remembrance or great rune
                    if item_id >= 2950 && item_id <= 2964 {
                        chain_type = ChainType::Remembrance;
                    } else if item_id >= 8148 && item_id <= 8153 || item_id == 10080 {
                        chain_type = ChainType::GreatRune;
                    }
                }
            }
        }

        // 2. If it's a remembrance, also add related flags from boss chain
        if let Some(&defeat_flag) = self.remembrance_to_boss.get(&item_id) {
            if let Some(chain) = self.boss_chains.get(&defeat_flag) {
                // Add remembrance possession flag if we have it
                let remem_flag = chain.remembrance_flag;
                let has_formula = get_flag_offset(remem_flag).is_some();
                let is_set = event_flags.map(|ef| is_flag_set(ef, remem_flag));

                chain_flags.push(ChainFlag {
                    flag_id: remem_flag,
                    role: FlagRole::ItemPossession,
                    source: FlagSource::BossDefeatChain,
                    has_formula,
                    is_set,
                    notes: format!("Remembrance possession flag from {}", chain.name),
                });

                // Add remembrance duplication flag if exists
                if let Some(dup_flag) = chain.remembrance_item {
                    let has_formula = get_flag_offset(dup_flag).is_some();
                    let is_set = event_flags.map(|ef| is_flag_set(ef, dup_flag));

                    chain_flags.push(ChainFlag {
                        flag_id: dup_flag,
                        role: FlagRole::ItemConsumption,
                        source: FlagSource::BossDefeatChain,
                        has_formula,
                        is_set,
                        notes: "Remembrance duplication (Walking Mausoleum)".to_string(),
                    });
                }

                chain_type = ChainType::Remembrance;
            }
        }

        // 3. If we have the event graph, explore dependencies and enables
        if let Some(graph) = event_graph {
            for cf in chain_flags.clone() {
                // Get flags that this flag enables
                if let Some(enables) = graph.get_enables(cf.flag_id) {
                    for en in enables {
                        if !chain_flags.iter().any(|c| c.flag_id == en.enabled_flag) {
                            let has_formula = get_flag_offset(en.enabled_flag).is_some();
                            let is_set = event_flags.map(|ef| is_flag_set(ef, en.enabled_flag));

                            chain_flags.push(ChainFlag {
                                flag_id: en.enabled_flag,
                                role: FlagRole::Unknown,
                                source: FlagSource::EventGraph,
                                has_formula,
                                is_set,
                                notes: format!("Enabled by {} ({})", cf.flag_id, en.relationship),
                            });
                        }
                    }
                }

                // Get dependencies (prerequisites)
                if let Some(deps) = graph.get_dependencies(cf.flag_id) {
                    for dep in deps {
                        if !chain_flags.iter().any(|c| c.flag_id == dep.required_flag) {
                            let has_formula = get_flag_offset(dep.required_flag).is_some();
                            let is_set = event_flags.map(|ef| is_flag_set(ef, dep.required_flag));

                            chain_flags.push(ChainFlag {
                                flag_id: dep.required_flag,
                                role: FlagRole::AreaAccess,
                                source: FlagSource::EventGraph,
                                has_formula,
                                is_set,
                                notes: format!("Required by {} ({})", cf.flag_id, dep.condition_type),
                            });
                        }
                    }
                }
            }
        }

        // 4. Find the best verification flag (has formula AND is set)
        let verification_flag = chain_flags
            .iter()
            .filter(|cf| cf.has_formula && cf.is_set == Some(true))
            .min_by_key(|cf| match cf.role {
                // Prefer boss defeat > pickup > possession > others
                FlagRole::BossDefeat => 0,
                FlagRole::ItemPickup => 1,
                FlagRole::ItemPossession => 2,
                _ => 3,
            })
            .map(|cf| cf.flag_id);

        // 5. Find the blocking flag (the one that prevents re-obtaining)
        // Usually this is the boss defeat flag or pickup flag
        let blocking_flag = chain_flags
            .iter()
            .filter(|cf| cf.role == FlagRole::BossDefeat || cf.role == FlagRole::ItemPickup)
            .map(|cf| cf.flag_id)
            .next();

        ItemFlagChain {
            item_id,
            item_name: item_name.to_string(),
            chain_flags,
            verification_flag,
            blocking_flag,
            chain_type,
        }
    }

    /// Resolve chains for multiple items and return summary
    pub fn resolve_many(
        &self,
        items: &[(u32, &str)],
        event_flags: Option<&[u8]>,
        event_graph: Option<&EventGraph>,
    ) -> Vec<ItemFlagChain> {
        items
            .iter()
            .map(|(id, name)| self.resolve_chain(*id, name, event_flags, event_graph))
            .collect()
    }

    /// Find all detectable flags for items in inventory
    pub fn find_detectable_flags(
        &self,
        item_ids: &[u32],
        event_flags: &[u8],
    ) -> HashMap<u32, Vec<ChainFlag>> {
        let mut result = HashMap::new();

        for &item_id in item_ids {
            let chain = self.resolve_chain(item_id, "", Some(event_flags), None);
            let detectable: Vec<_> = chain
                .chain_flags
                .into_iter()
                .filter(|cf| cf.has_formula && cf.is_set == Some(true))
                .collect();

            if !detectable.is_empty() {
                result.insert(item_id, detectable);
            }
        }

        result
    }
}

impl Default for ItemChainResolver {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// DISPLAY HELPERS
// ============================================================================

impl std::fmt::Display for ItemFlagChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Chain for {} (item_id: {})", self.item_name, self.item_id)?;
        writeln!(f, "  Type: {:?}", self.chain_type)?;
        writeln!(f, "  Verification flag: {:?}", self.verification_flag)?;
        writeln!(f, "  Blocking flag: {:?}", self.blocking_flag)?;
        writeln!(f, "  Flags in chain:")?;

        for cf in &self.chain_flags {
            let status = match cf.is_set {
                Some(true) => "[X]",
                Some(false) => "[ ]",
                None => "[?]",
            };
            let formula = if cf.has_formula { "✓" } else { "✗" };
            writeln!(
                f,
                "    {} {} {:>8} | {:>12} | {:>12} | {}",
                status,
                formula,
                cf.flag_id,
                cf.role.as_str(),
                cf.source.as_str(),
                cf.notes
            )?;
        }

        Ok(())
    }
}

// ============================================================================
// CHAIN ANALYSIS HELPERS
// ============================================================================

/// Analyze chains for common inventory items and print results
pub fn analyze_inventory_chains(
    event_flags: &[u8],
    inventory_item_ids: &[u32],
) -> ChainAnalysisReport {
    let resolver = ItemChainResolver::new();

    let mut chains = Vec::new();
    let mut detectable_count = 0;
    let mut missing_formula_count = 0;
    let mut flag_not_set_count = 0;

    // Common items to test
    let test_items: &[(u32, &str)] = &[
        // Remembrances
        (2950, "Remembrance of the Grafted"),
        (2951, "Remembrance of the Starscourge"),
        (2952, "Remembrance of the Omen King"),
        (2953, "Remembrance of the Blasphemous"),
        (2954, "Remembrance of the Rot Goddess"),
        (2955, "Remembrance of the Blood Lord"),
        (2959, "Remembrance of the Full Moon Queen"),
        // Great Runes
        (8148, "Godrick's Great Rune"),
        (8149, "Radahn's Great Rune"),
        (8150, "Morgott's Great Rune"),
        (8151, "Rykard's Great Rune"),
        (8152, "Mohg's Great Rune"),
        (8153, "Malenia's Great Rune"),
        (10080, "Great Rune of the Unborn"),
        // Key Items
        (8135, "Rold Medallion"),
        (8175, "Haligtree Secret Medallion (Left)"),
        (8176, "Haligtree Secret Medallion (Right)"),
        (8100, "Crafting Kit"),
        (8900, "Whetstone Knife"),
    ];

    for (item_id, name) in test_items {
        // Only analyze items that are in the inventory
        if !inventory_item_ids.contains(item_id) {
            continue;
        }

        let chain = resolver.resolve_chain(*item_id, name, Some(event_flags), None);

        if let Some(_) = chain.verification_flag {
            detectable_count += 1;
        } else {
            // Check why not detectable
            let has_any_formula = chain.chain_flags.iter().any(|cf| cf.has_formula);
            let any_set = chain.chain_flags.iter().any(|cf| cf.is_set == Some(true));

            if !has_any_formula {
                missing_formula_count += 1;
            } else if !any_set {
                flag_not_set_count += 1;
            }
        }

        chains.push(chain);
    }

    ChainAnalysisReport {
        total_items: chains.len(),
        detectable: detectable_count,
        missing_formula: missing_formula_count,
        flag_not_set: flag_not_set_count,
        chains,
    }
}

/// Report from chain analysis
#[derive(Debug)]
pub struct ChainAnalysisReport {
    pub total_items: usize,
    pub detectable: usize,
    pub missing_formula: usize,
    pub flag_not_set: usize,
    pub chains: Vec<ItemFlagChain>,
}

impl std::fmt::Display for ChainAnalysisReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Item Chain Analysis Report ===")?;
        writeln!(f, "Total items in inventory: {}", self.total_items)?;
        writeln!(f, "Detectable via chain: {} ({:.1}%)",
            self.detectable,
            if self.total_items > 0 { self.detectable as f64 / self.total_items as f64 * 100.0 } else { 0.0 }
        )?;
        writeln!(f, "Missing formula: {}", self.missing_formula)?;
        writeln!(f, "Flag not set: {}", self.flag_not_set)?;
        writeln!(f)?;

        for chain in &self.chains {
            let status = if chain.verification_flag.is_some() { "✓" } else { "✗" };
            writeln!(f, "{} {} (item {})", status, chain.item_name, chain.item_id)?;

            for cf in &chain.chain_flags {
                let set_status = match cf.is_set {
                    Some(true) => "[X]",
                    Some(false) => "[ ]",
                    None => "[?]",
                };
                let formula_status = if cf.has_formula { "F" } else { "-" };
                writeln!(f, "  {} {} {:>8} {:>12} | {}",
                    set_status, formula_status, cf.flag_id, cf.role.as_str(), cf.notes
                )?;
            }
        }

        Ok(())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_creation() {
        let resolver = ItemChainResolver::new();
        assert!(!resolver.known_mappings.is_empty());
        assert!(!resolver.boss_chains.is_empty());
    }

    #[test]
    fn test_remembrance_chain() {
        let resolver = ItemChainResolver::new();
        let chain = resolver.resolve_chain(2950, "Remembrance of the Grafted", None, None);

        assert_eq!(chain.item_id, 2950);
        assert_eq!(chain.chain_type, ChainType::Remembrance);
        assert!(!chain.chain_flags.is_empty());

        // Should have boss defeat flag 171
        assert!(chain.chain_flags.iter().any(|cf| cf.flag_id == 171));
    }

    #[test]
    fn test_great_rune_chain() {
        let resolver = ItemChainResolver::new();
        let chain = resolver.resolve_chain(8148, "Godrick's Great Rune", None, None);

        assert_eq!(chain.chain_type, ChainType::GreatRune);
        assert!(chain.chain_flags.iter().any(|cf| cf.flag_id == 171));
    }

    #[test]
    fn test_blocking_flag_identified() {
        let resolver = ItemChainResolver::new();
        let chain = resolver.resolve_chain(2950, "Remembrance of the Grafted", None, None);

        // Blocking flag should be the boss defeat flag
        assert_eq!(chain.blocking_flag, Some(171));
    }
}
