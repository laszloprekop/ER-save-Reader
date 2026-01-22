/// Event Graph Module
///
/// Loads and indexes the EMEVD event graph from event_graph.json,
/// providing queryable access to flag triggers, dependencies, entity mappings,
/// and progression chains extracted from all 589 EMEVD files.
///
/// Key features:
/// - O(1) flag trigger lookup: what action sets each flag
/// - Dependency graph traversal: prerequisite chains
/// - Entity-to-flag mapping: boss/grace entities to their flags
/// - Progression chain lookup: remembrances, map fragments, etc.
///
/// Primary use case: Validate flag existence via SetEventFlagID evidence

use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;

use serde::{Deserialize, Serialize};

/// What action sets a specific flag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagTrigger {
    pub event_id: u32,
    pub source_file: String,
    pub action: String,  // "ON" or "OFF"
    pub trigger_context: String,  // "boss_defeat", "grace_discovery", etc.
    pub entity_id: Option<u64>,
}

/// Entry containing all triggers for a flag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagTriggerEntry {
    pub flag_id: u32,
    pub triggers: Vec<FlagTrigger>,
}

/// A dependency relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub required_flag: u32,
    pub condition_type: String,
    pub source_event: Option<u32>,
    pub source_file: Option<String>,
}

/// Enabled flag relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnablesInfo {
    pub enabled_flag: u32,
    pub relationship: String,
}

/// Entry for flag dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagDependencyEntry {
    pub flag_id: Option<u32>,
    pub depends_on: Vec<DependencyInfo>,
    pub enables: Vec<EnablesInfo>,
}

/// Associated flag info for entity mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociatedFlag {
    pub flag_id: u32,
    pub relationship: String,
}

/// Entity to flag mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityFlagMapping {
    pub entity_type: String,
    pub map_tile: String,
    pub associated_flags: Vec<AssociatedFlag>,
}

/// Progression chain (remembrance, map fragment, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressionChain {
    pub chain_type: String,
    pub boss_defeat: Option<u32>,
    pub item_lot: Option<u32>,
    pub possession_flag: Option<u32>,
    pub event_id: u32,
    pub params: Vec<i64>,
}

/// Metadata about the extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetadata {
    pub extraction_date: String,
    pub emevd_files_parsed: u32,
    pub total_unique_flags: u32,
    pub total_triggers: u32,
    pub total_dependencies: u32,
    pub entity_mappings: u32,
    pub progression_chains: u32,
}

/// Raw JSON structure matching event_graph.json
#[derive(Debug, Deserialize)]
struct RawEventGraph {
    metadata: GraphMetadata,
    flag_triggers: HashMap<String, FlagTriggerEntry>,
    flag_dependencies: HashMap<String, FlagDependencyEntry>,
    entity_flag_map: HashMap<String, EntityFlagMapping>,
    progression_chains: HashMap<String, ProgressionChain>,
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphSummary {
    pub total_flags: usize,
    pub total_triggers: usize,
    pub total_dependencies: usize,
    pub entity_mappings: usize,
    pub progression_chains: usize,
    pub files_parsed: u32,
}

/// Error type for event graph operations
#[derive(Debug)]
pub enum GraphError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    NotFound(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::IoError(e) => write!(f, "IO error: {}", e),
            GraphError::JsonError(e) => write!(f, "JSON parse error: {}", e),
            GraphError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for GraphError {}

impl From<std::io::Error> for GraphError {
    fn from(err: std::io::Error) -> Self {
        GraphError::IoError(err)
    }
}

impl From<serde_json::Error> for GraphError {
    fn from(err: serde_json::Error) -> Self {
        GraphError::JsonError(err)
    }
}

/// The main event graph with indexed lookups
#[derive(Debug)]
pub struct EventGraph {
    /// Metadata from extraction
    pub metadata: GraphMetadata,

    /// Flag triggers indexed by flag ID
    flag_triggers: HashMap<u32, FlagTriggerEntry>,

    /// Flag dependencies indexed by flag ID
    flag_dependencies: HashMap<u32, FlagDependencyEntry>,

    /// Entity flag mappings indexed by entity ID
    entity_flag_map: HashMap<u64, EntityFlagMapping>,

    /// Progression chains indexed by key
    progression_chains: HashMap<String, ProgressionChain>,

    /// Reverse index: flags that enable other flags
    reverse_enables: HashMap<u32, Vec<u32>>,

    /// Context index: flags by trigger context
    by_context: HashMap<String, Vec<u32>>,
}

impl EventGraph {
    /// Default path to event_graph.json
    const DEFAULT_PATH: &'static str = "scripts/event_graph.json";

    /// Load event graph from JSON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, GraphError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let raw: RawEventGraph = serde_json::from_reader(reader)?;

        // Convert flag triggers (string keys -> u32)
        let mut flag_triggers = HashMap::new();
        for (key, entry) in raw.flag_triggers {
            if let Ok(flag_id) = key.parse::<u32>() {
                flag_triggers.insert(flag_id, entry);
            }
        }

        // Convert flag dependencies
        let mut flag_dependencies = HashMap::new();
        for (key, entry) in raw.flag_dependencies {
            if let Ok(flag_id) = key.parse::<u32>() {
                flag_dependencies.insert(flag_id, entry);
            }
        }

        // Convert entity mappings
        let mut entity_flag_map = HashMap::new();
        for (key, mapping) in raw.entity_flag_map {
            if let Ok(entity_id) = key.parse::<u64>() {
                entity_flag_map.insert(entity_id, mapping);
            }
        }

        // Build reverse enables index
        let mut reverse_enables: HashMap<u32, Vec<u32>> = HashMap::new();
        for (flag_id, deps) in &flag_dependencies {
            for enables in &deps.enables {
                reverse_enables
                    .entry(enables.enabled_flag)
                    .or_default()
                    .push(*flag_id);
            }
        }

        // Build context index
        let mut by_context: HashMap<String, Vec<u32>> = HashMap::new();
        for (flag_id, entry) in &flag_triggers {
            for trigger in &entry.triggers {
                by_context
                    .entry(trigger.trigger_context.clone())
                    .or_default()
                    .push(*flag_id);
            }
        }

        Ok(Self {
            metadata: raw.metadata,
            flag_triggers,
            flag_dependencies,
            entity_flag_map,
            progression_chains: raw.progression_chains,
            reverse_enables,
            by_context,
        })
    }

    /// Load from the default location (scripts/event_graph.json)
    pub fn load_default() -> Result<Self, GraphError> {
        Self::load(Self::DEFAULT_PATH)
    }

    /// Get summary statistics
    pub fn summary(&self) -> GraphSummary {
        GraphSummary {
            total_flags: self.flag_triggers.len(),
            total_triggers: self.flag_triggers.values().map(|e| e.triggers.len()).sum(),
            total_dependencies: self.flag_dependencies.len(),
            entity_mappings: self.entity_flag_map.len(),
            progression_chains: self.progression_chains.len(),
            files_parsed: self.metadata.emevd_files_parsed,
        }
    }

    // === Primary Query Methods ===

    /// Check if a flag has any triggers (validates flag existence)
    pub fn has_trigger(&self, flag_id: u32) -> bool {
        self.flag_triggers.contains_key(&flag_id)
    }

    /// Get all triggers for a flag
    pub fn get_triggers(&self, flag_id: u32) -> Option<&Vec<FlagTrigger>> {
        self.flag_triggers.get(&flag_id).map(|e| &e.triggers)
    }

    /// Get the primary trigger context for a flag
    pub fn get_trigger_context(&self, flag_id: u32) -> Option<&str> {
        self.flag_triggers
            .get(&flag_id)
            .and_then(|e| e.triggers.first())
            .map(|t| t.trigger_context.as_str())
    }

    /// Get dependencies for a flag (what flags must be set first)
    pub fn get_dependencies(&self, flag_id: u32) -> Option<&Vec<DependencyInfo>> {
        self.flag_dependencies.get(&flag_id).map(|e| &e.depends_on)
    }

    /// Get flags that this flag enables
    pub fn get_enables(&self, flag_id: u32) -> Option<&Vec<EnablesInfo>> {
        self.flag_dependencies.get(&flag_id).map(|e| &e.enables)
    }

    /// Get flags that enable/depend on this flag (reverse lookup)
    pub fn get_dependent_flags(&self, flag_id: u32) -> Option<&Vec<u32>> {
        self.reverse_enables.get(&flag_id)
    }

    /// Get all flags with a specific trigger context
    pub fn get_flags_by_context(&self, context: &str) -> Option<&Vec<u32>> {
        self.by_context.get(context)
    }

    // === Entity Queries ===

    /// Get entity flag mapping by entity ID
    pub fn get_entity_flags(&self, entity_id: u64) -> Option<&EntityFlagMapping> {
        self.entity_flag_map.get(&entity_id)
    }

    /// Find entity ID for a given flag
    pub fn find_entity_for_flag(&self, flag_id: u32) -> Option<u64> {
        for (entity_id, mapping) in &self.entity_flag_map {
            for assoc in &mapping.associated_flags {
                if assoc.flag_id == flag_id {
                    return Some(*entity_id);
                }
            }
        }
        None
    }

    // === Progression Chain Queries ===

    /// Get a progression chain by key
    pub fn get_chain(&self, key: &str) -> Option<&ProgressionChain> {
        self.progression_chains.get(key)
    }

    /// Find remembrance chain by boss defeat flag
    pub fn find_remembrance_chain(&self, boss_defeat: u32) -> Option<&ProgressionChain> {
        let key = format!("remembrance_{}", boss_defeat);
        self.progression_chains.get(&key)
    }

    /// Find map fragment chain by discovery flag
    pub fn find_map_fragment_chain(&self, discovery_flag: u32) -> Option<&ProgressionChain> {
        let key = format!("map_fragment_{}", discovery_flag);
        self.progression_chains.get(&key)
    }

    /// Get all chains of a specific type
    pub fn get_chains_by_type(&self, chain_type: &str) -> Vec<&ProgressionChain> {
        self.progression_chains
            .values()
            .filter(|c| c.chain_type == chain_type)
            .collect()
    }

    // === Traversal Methods ===

    /// Traverse dependency chain up to a maximum depth
    /// Returns all prerequisite flags in dependency order
    pub fn traverse_dependencies(&self, flag_id: u32, max_depth: usize) -> Vec<u32> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.traverse_deps_recursive(flag_id, 0, max_depth, &mut visited, &mut result);
        result
    }

    fn traverse_deps_recursive(
        &self,
        flag_id: u32,
        depth: usize,
        max_depth: usize,
        visited: &mut std::collections::HashSet<u32>,
        result: &mut Vec<u32>,
    ) {
        if depth > max_depth || visited.contains(&flag_id) {
            return;
        }
        visited.insert(flag_id);

        if let Some(deps) = self.get_dependencies(flag_id) {
            for dep in deps {
                self.traverse_deps_recursive(
                    dep.required_flag,
                    depth + 1,
                    max_depth,
                    visited,
                    result,
                );
                result.push(dep.required_flag);
            }
        }
    }

    /// Traverse enablement chain (what does this flag enable, recursively)
    pub fn traverse_enables(&self, flag_id: u32, max_depth: usize) -> Vec<u32> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.traverse_enables_recursive(flag_id, 0, max_depth, &mut visited, &mut result);
        result
    }

    fn traverse_enables_recursive(
        &self,
        flag_id: u32,
        depth: usize,
        max_depth: usize,
        visited: &mut std::collections::HashSet<u32>,
        result: &mut Vec<u32>,
    ) {
        if depth > max_depth || visited.contains(&flag_id) {
            return;
        }
        visited.insert(flag_id);

        if let Some(enables) = self.get_enables(flag_id) {
            for en in enables {
                result.push(en.enabled_flag);
                self.traverse_enables_recursive(
                    en.enabled_flag,
                    depth + 1,
                    max_depth,
                    visited,
                    result,
                );
            }
        }
    }

    // === Validation Helpers ===

    /// Validate that a flag exists in EMEVD (has a SetEventFlagID call)
    /// This is the primary use case for formula validation
    pub fn validate_flag_existence(&self, flag_id: u32) -> bool {
        self.has_trigger(flag_id)
    }

    /// Get validation evidence for a flag
    pub fn get_validation_evidence(&self, flag_id: u32) -> Option<ValidationEvidence> {
        self.flag_triggers.get(&flag_id).map(|entry| {
            ValidationEvidence {
                flag_id,
                trigger_count: entry.triggers.len(),
                primary_context: entry.triggers.first().map(|t| t.trigger_context.clone()),
                source_files: entry
                    .triggers
                    .iter()
                    .map(|t| t.source_file.clone())
                    .collect(),
                has_entity: entry.triggers.iter().any(|t| t.entity_id.is_some()),
            }
        })
    }

    /// List all unique trigger contexts
    pub fn list_contexts(&self) -> Vec<&str> {
        self.by_context.keys().map(|s| s.as_str()).collect()
    }

    /// Get all flag IDs that have triggers
    pub fn get_all_flag_ids(&self) -> Vec<u32> {
        self.flag_triggers.keys().copied().collect()
    }
}

/// Evidence that a flag exists in EMEVD
#[derive(Debug, Clone)]
pub struct ValidationEvidence {
    pub flag_id: u32,
    pub trigger_count: usize,
    pub primary_context: Option<String>,
    pub source_files: Vec<String>,
    pub has_entity: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_event_graph() {
        let graph = EventGraph::load_default();
        assert!(graph.is_ok(), "Failed to load event graph: {:?}", graph.err());

        let graph = graph.unwrap();
        let summary = graph.summary();

        println!("Event Graph Summary:");
        println!("  Total flags: {}", summary.total_flags);
        println!("  Total triggers: {}", summary.total_triggers);
        println!("  Total dependencies: {}", summary.total_dependencies);
        println!("  Entity mappings: {}", summary.entity_mappings);
        println!("  Progression chains: {}", summary.progression_chains);

        assert!(summary.total_flags > 0, "No flags loaded");
    }

    #[test]
    fn test_known_flag_triggers() {
        let graph = EventGraph::load_default();
        if graph.is_err() {
            println!("Skipping test - event_graph.json not found");
            return;
        }
        let graph = graph.unwrap();

        // Test First Step grace (76100)
        assert!(graph.has_trigger(76100), "First Step grace (76100) should have trigger");

        // Test Godrick remembrance (9100)
        assert!(graph.has_trigger(9100), "Godrick remembrance (9100) should have trigger");

        // Test Limgrave West map fragment (62010)
        assert!(graph.has_trigger(62010), "Map fragment (62010) should have trigger");
    }

    #[test]
    fn test_trigger_context() {
        let graph = EventGraph::load_default();
        if graph.is_err() {
            return;
        }
        let graph = graph.unwrap();

        // Grace should have grace context
        if let Some(context) = graph.get_trigger_context(76100) {
            println!("76100 context: {}", context);
        }

        // Remembrance should have remembrance context
        if let Some(triggers) = graph.get_triggers(9100) {
            println!("9100 has {} triggers", triggers.len());
            for t in triggers {
                println!("  - context: {}, source: {}", t.trigger_context, t.source_file);
            }
        }
    }

    #[test]
    fn test_progression_chains() {
        let graph = EventGraph::load_default();
        if graph.is_err() {
            return;
        }
        let graph = graph.unwrap();

        // Find Godrick remembrance chain
        if let Some(chain) = graph.find_remembrance_chain(9100) {
            println!("Godrick remembrance chain:");
            println!("  Type: {}", chain.chain_type);
            println!("  Boss defeat: {:?}", chain.boss_defeat);
            println!("  Possession flag: {:?}", chain.possession_flag);
        }

        // List all remembrance chains
        let remembrances = graph.get_chains_by_type("remembrance");
        println!("Found {} remembrance chains", remembrances.len());
    }

    #[test]
    fn test_validation_evidence() {
        let graph = EventGraph::load_default();
        if graph.is_err() {
            return;
        }
        let graph = graph.unwrap();

        // Get evidence for First Step grace
        if let Some(evidence) = graph.get_validation_evidence(76100) {
            println!("Validation evidence for 76100:");
            println!("  Trigger count: {}", evidence.trigger_count);
            println!("  Primary context: {:?}", evidence.primary_context);
            println!("  Source files: {:?}", evidence.source_files);
            println!("  Has entity: {}", evidence.has_entity);
        }
    }
}
