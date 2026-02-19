/// Relationship Graph Module
///
/// Loads and indexes flag relationships from flag_relationships.json
/// providing multi-point corroboration capabilities.
///
/// Key features:
/// - Load 2,796 relationships across 5,079 flags
/// - Identify 122 dual-formula corroboration pairs (tile↔block)
/// - Support relationship-based validation

use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;

use serde::{Deserialize, Serialize};

/// Type of relationship between two flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// World pickup sets a possession flag
    PickupSetsFlag,
    /// Release flag enables stock flag
    EnablesPurchase,
    /// Entity ID triggers grace discovery flag
    GraceDiscovery,
    /// Boss remembrance links (91xx → 510xxx)
    BossRemembrance,
    /// Flags set together in event scripts
    EventSequence,
    /// Map fragment discovery → possession
    MapFragment,
    /// Boss defeat chain: defeat → remembrance → great rune possession → activation
    BossDefeatChain,
    /// Area prerequisite: items/flags required to access an area
    AreaPrerequisite,
    /// Geographic proximity: flags in the same region correlate
    GeographicProximity,
    /// Scroll/item given to NPC unlocks spells/items
    ScrollUnlock,
}

impl RelationshipType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pickup_sets_flag" => Some(Self::PickupSetsFlag),
            "enables_purchase" => Some(Self::EnablesPurchase),
            "grace_discovery" => Some(Self::GraceDiscovery),
            "boss_remembrance" => Some(Self::BossRemembrance),
            "event_sequence" => Some(Self::EventSequence),
            "map_fragment" => Some(Self::MapFragment),
            "boss_defeat_chain" => Some(Self::BossDefeatChain),
            "area_prerequisite" => Some(Self::AreaPrerequisite),
            "geographic_proximity" => Some(Self::GeographicProximity),
            "scroll_unlock" => Some(Self::ScrollUnlock),
            _ => None,
        }
    }
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PickupSetsFlag => write!(f, "pickup_sets_flag"),
            Self::EnablesPurchase => write!(f, "enables_purchase"),
            Self::GraceDiscovery => write!(f, "grace_discovery"),
            Self::BossRemembrance => write!(f, "boss_remembrance"),
            Self::EventSequence => write!(f, "event_sequence"),
            Self::MapFragment => write!(f, "map_fragment"),
            Self::BossDefeatChain => write!(f, "boss_defeat_chain"),
            Self::AreaPrerequisite => write!(f, "area_prerequisite"),
            Self::GeographicProximity => write!(f, "geographic_proximity"),
            Self::ScrollUnlock => write!(f, "scroll_unlock"),
        }
    }
}

/// A relationship between two flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagRelationship {
    pub source_flag: u32,
    pub target_flag: u32,
    pub relationship_type: RelationshipType,
    pub source_file: Option<String>,
    pub item_name: Option<String>,
    pub notes: Option<String>,
    /// When true, this edge is excluded from corroboration checks.
    /// Used for pickup_sets_flag edges where the tile-side flag is the row_id
    /// position (never written by the game), not the actual getItemFlagId.
    #[serde(default)]
    pub skip_corroboration: bool,
}

/// A dual-formula corroboration pair
/// These pairs can be validated using two different formulas:
/// - tile_flag uses the tile formula (10-digit world pickup)
/// - block_flag uses the block formula (5-digit possession flag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorroborationPair {
    pub tile_flag: u32,      // 10-digit flag, tile formula
    pub block_flag: u32,     // 5-digit flag, block formula
    pub item_name: Option<String>,
    pub notes: Option<String>,
}

/// Statistics about the relationship graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatistics {
    pub total_flags: usize,
    pub total_relationships: usize,
    pub relationship_types: HashMap<String, usize>,
}

/// Raw JSON structure for flag_relationships.json
#[derive(Debug, Deserialize)]
struct RawRelationshipGraph {
    nodes: HashMap<String, RawNode>,
    edges: Vec<RawEdge>,
    by_type: HashMap<String, Vec<RawEdge>>,
    statistics: GraphStatistics,
}

#[derive(Debug, Deserialize)]
struct RawNode {
    id: u64,
    connections: usize,
}

#[derive(Debug, Deserialize)]
struct RawEdge {
    source: u64,
    target: u64,
    #[serde(rename = "type")]
    edge_type: String,
    file: Option<String>,
    item: Option<String>,
    notes: Option<String>,
    #[serde(default)]
    skip_corroboration: bool,
}

/// The relationship graph with indexed lookups
#[derive(Debug)]
pub struct RelationshipGraph {
    /// All relationships
    relationships: Vec<FlagRelationship>,

    /// Relationships indexed by source flag
    by_source: HashMap<u32, Vec<usize>>,

    /// Relationships indexed by target flag
    by_target: HashMap<u32, Vec<usize>>,

    /// Relationships grouped by type
    by_type: HashMap<RelationshipType, Vec<usize>>,

    /// Dual-formula corroboration pairs (tile → block)
    corroboration_pairs: Vec<CorroborationPair>,

    /// All unique flags in the graph
    all_flags: Vec<u32>,

    /// Connection count per flag
    connections: HashMap<u32, usize>,

    /// Original statistics
    pub statistics: GraphStatistics,
}

impl RelationshipGraph {
    /// Load relationship graph from JSON file
    pub fn load_from_json(path: &Path) -> Result<Self, GraphError> {
        let file = File::open(path)
            .map_err(|e| GraphError::IoError(format!("Failed to open graph: {}", e)))?;

        let reader = BufReader::new(file);
        let raw: RawRelationshipGraph = serde_json::from_reader(reader)
            .map_err(|e| GraphError::ParseError(format!("Failed to parse graph: {}", e)))?;

        Self::from_raw(raw)
    }

    /// Load from default location (scripts/flag_relationships.json)
    pub fn load_default() -> Result<Self, GraphError> {
        let default_path = Path::new("scripts/flag_relationships.json");
        if default_path.exists() {
            return Self::load_from_json(default_path);
        }

        let abs_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/scripts/flag_relationships.json");
        if abs_path.exists() {
            return Self::load_from_json(abs_path);
        }

        Err(GraphError::NotFound("Could not find flag_relationships.json".to_string()))
    }

    /// Build from raw JSON data
    fn from_raw(raw: RawRelationshipGraph) -> Result<Self, GraphError> {
        let mut relationships = Vec::with_capacity(raw.edges.len());
        let mut by_source: HashMap<u32, Vec<usize>> = HashMap::new();
        let mut by_target: HashMap<u32, Vec<usize>> = HashMap::new();
        let mut by_type: HashMap<RelationshipType, Vec<usize>> = HashMap::new();
        let mut corroboration_pairs = Vec::new();
        let mut all_flags_set = std::collections::HashSet::new();
        let mut connections: HashMap<u32, usize> = HashMap::new();

        for edge in raw.edges {
            let source = edge.source as u32;
            let target = edge.target as u32;

            let rel_type = match RelationshipType::from_str(&edge.edge_type) {
                Some(t) => t,
                None => continue, // Skip unknown types
            };

            let rel = FlagRelationship {
                source_flag: source,
                target_flag: target,
                relationship_type: rel_type,
                source_file: edge.file,
                item_name: edge.item.clone(),
                notes: edge.notes.clone(),
                skip_corroboration: edge.skip_corroboration,
            };

            let idx = relationships.len();
            relationships.push(rel);

            by_source.entry(source).or_default().push(idx);
            by_target.entry(target).or_default().push(idx);
            by_type.entry(rel_type).or_default().push(idx);

            all_flags_set.insert(source);
            all_flags_set.insert(target);

            *connections.entry(source).or_insert(0) += 1;
            *connections.entry(target).or_insert(0) += 1;

            // Identify dual-formula corroboration pairs
            // These are pickup_sets_flag where source is 10-digit and target is 5-digit
            // Skip edges marked as non-corroborable (tile flag is row_id, not getItemFlagId)
            if rel_type == RelationshipType::PickupSetsFlag
                && source >= 1_000_000_000
                && target >= 60000
                && target < 100_000
                && !edge.skip_corroboration
            {
                corroboration_pairs.push(CorroborationPair {
                    tile_flag: source,
                    block_flag: target,
                    item_name: edge.item,
                    notes: edge.notes,
                });
            }
        }

        let mut all_flags: Vec<u32> = all_flags_set.into_iter().collect();
        all_flags.sort();

        Ok(Self {
            relationships,
            by_source,
            by_target,
            by_type,
            corroboration_pairs,
            all_flags,
            connections,
            statistics: raw.statistics,
        })
    }

    /// Get all relationships where this flag is the source
    pub fn get_by_source(&self, flag_id: u32) -> Vec<&FlagRelationship> {
        self.by_source
            .get(&flag_id)
            .map(|indices| indices.iter().map(|&i| &self.relationships[i]).collect())
            .unwrap_or_default()
    }

    /// Get all relationships where this flag is the target
    pub fn get_by_target(&self, flag_id: u32) -> Vec<&FlagRelationship> {
        self.by_target
            .get(&flag_id)
            .map(|indices| indices.iter().map(|&i| &self.relationships[i]).collect())
            .unwrap_or_default()
    }

    /// Get all relationships involving this flag (as source or target)
    pub fn get_related(&self, flag_id: u32) -> Vec<&FlagRelationship> {
        let mut results = self.get_by_source(flag_id);
        results.extend(self.get_by_target(flag_id));
        results
    }

    /// Get all related flag IDs for a given flag
    pub fn get_related_flag_ids(&self, flag_id: u32) -> Vec<u32> {
        let mut related = Vec::new();

        for rel in self.get_by_source(flag_id) {
            related.push(rel.target_flag);
        }
        for rel in self.get_by_target(flag_id) {
            related.push(rel.source_flag);
        }

        related.sort();
        related.dedup();
        related
    }

    /// Get relationships by type
    pub fn get_by_type(&self, rel_type: RelationshipType) -> Vec<&FlagRelationship> {
        self.by_type
            .get(&rel_type)
            .map(|indices| indices.iter().map(|&i| &self.relationships[i]).collect())
            .unwrap_or_default()
    }

    /// Get all dual-formula corroboration pairs
    pub fn get_corroboration_pairs(&self) -> &[CorroborationPair] {
        &self.corroboration_pairs
    }

    /// Find corroboration pair for a block flag (5-digit)
    pub fn find_corroboration_for_block(&self, block_flag: u32) -> Option<&CorroborationPair> {
        self.corroboration_pairs.iter().find(|p| p.block_flag == block_flag)
    }

    /// Find corroboration pair for a tile flag (10-digit)
    pub fn find_corroboration_for_tile(&self, tile_flag: u32) -> Option<&CorroborationPair> {
        self.corroboration_pairs.iter().find(|p| p.tile_flag == tile_flag)
    }

    /// Get connection count for a flag
    pub fn connection_count(&self, flag_id: u32) -> usize {
        self.connections.get(&flag_id).copied().unwrap_or(0)
    }

    /// Get all flags with N+ connections
    pub fn flags_with_min_connections(&self, min_connections: usize) -> Vec<(u32, usize)> {
        self.connections
            .iter()
            .filter(|(_, &count)| count >= min_connections)
            .map(|(&flag, &count)| (flag, count))
            .collect()
    }

    /// Get total number of relationships
    pub fn len(&self) -> usize {
        self.relationships.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.relationships.is_empty()
    }

    /// Get total number of unique flags
    pub fn flag_count(&self) -> usize {
        self.all_flags.len()
    }

    /// Get number of corroboration pairs
    pub fn corroboration_pair_count(&self) -> usize {
        self.corroboration_pairs.len()
    }

    /// Iterate over all relationships
    pub fn iter(&self) -> impl Iterator<Item = &FlagRelationship> {
        self.relationships.iter()
    }

    /// Iterate over all flags
    pub fn flags(&self) -> impl Iterator<Item = &u32> {
        self.all_flags.iter()
    }

    /// Check if a flag exists in the graph
    pub fn contains_flag(&self, flag_id: u32) -> bool {
        self.connections.contains_key(&flag_id)
    }

    /// Get summary statistics
    pub fn summary(&self) -> GraphSummary {
        let mut by_type_counts: HashMap<RelationshipType, usize> = HashMap::new();
        for rel in &self.relationships {
            *by_type_counts.entry(rel.relationship_type).or_insert(0) += 1;
        }

        GraphSummary {
            total_relationships: self.relationships.len(),
            total_flags: self.all_flags.len(),
            corroboration_pairs: self.corroboration_pairs.len(),
            by_type: by_type_counts,
        }
    }
}

/// Summary of the graph
#[derive(Debug)]
pub struct GraphSummary {
    pub total_relationships: usize,
    pub total_flags: usize,
    pub corroboration_pairs: usize,
    pub by_type: HashMap<RelationshipType, usize>,
}

impl std::fmt::Display for GraphSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Relationship Graph Summary:")?;
        writeln!(f, "  Total relationships: {}", self.total_relationships)?;
        writeln!(f, "  Total flags: {}", self.total_flags)?;
        writeln!(f, "  Corroboration pairs: {}", self.corroboration_pairs)?;
        writeln!(f, "  By type:")?;
        for (rel_type, count) in &self.by_type {
            writeln!(f, "    {}: {}", rel_type, count)?;
        }
        Ok(())
    }
}

/// Errors from graph operations
#[derive(Debug)]
pub enum GraphError {
    IoError(String),
    ParseError(String),
    NotFound(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::IoError(msg) => write!(f, "IO error: {}", msg),
            GraphError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            GraphError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for GraphError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_graph() {
        match RelationshipGraph::load_default() {
            Ok(graph) => {
                println!("{}", graph.summary());
                assert!(graph.len() > 2000, "Expected 2000+ relationships");
                assert!(graph.corroboration_pair_count() >= 100, "Expected 100+ corroboration pairs");
            }
            Err(e) => {
                println!("Could not load graph (expected in CI): {}", e);
            }
        }
    }

    #[test]
    fn test_corroboration_pairs() {
        if let Ok(graph) = RelationshipGraph::load_default() {
            let pairs = graph.get_corroboration_pairs();
            println!("Found {} corroboration pairs", pairs.len());

            // Check a known pair (Missionary's Cookbook [3])
            if let Some(pair) = graph.find_corroboration_for_block(67650) {
                println!("Found pair for 67650: tile={}, item={:?}",
                    pair.tile_flag, pair.item_name);
                assert!(pair.tile_flag >= 1_000_000_000);
            }
        }
    }

    #[test]
    fn test_relationship_lookup() {
        if let Ok(graph) = RelationshipGraph::load_default() {
            // Test a grace flag (should have grace_discovery relationship)
            let grace_rels = graph.get_by_target(71000);
            println!("Relationships targeting flag 71000: {}", grace_rels.len());

            // Test getting related flags
            let related = graph.get_related_flag_ids(71000);
            println!("Related flags for 71000: {:?}", related);
        }
    }

    #[test]
    fn test_relationship_type_parsing() {
        assert_eq!(RelationshipType::from_str("pickup_sets_flag"), Some(RelationshipType::PickupSetsFlag));
        assert_eq!(RelationshipType::from_str("enables_purchase"), Some(RelationshipType::EnablesPurchase));
        assert_eq!(RelationshipType::from_str("grace_discovery"), Some(RelationshipType::GraceDiscovery));
        assert_eq!(RelationshipType::from_str("unknown"), None);
    }

    #[test]
    fn test_connection_counts() {
        if let Ok(graph) = RelationshipGraph::load_default() {
            // Find highly connected flags
            let high_conn = graph.flags_with_min_connections(5);
            println!("Flags with 5+ connections: {}", high_conn.len());

            for (flag, count) in high_conn.iter().take(10) {
                println!("  Flag {}: {} connections", flag, count);
            }
        }
    }
}
