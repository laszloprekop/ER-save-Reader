/// Unified Flag Database
///
/// Merges data from three sources into a single queryable database:
/// 1. Flag Catalog - names, positions, regions, item associations
/// 2. Param Database - source traceability (param file, row, field)
/// 3. Event Graph - EMEVD triggers, dependencies, progression chains
///
/// This provides a complete picture of each flag:
/// - WHAT it represents (catalog)
/// - WHERE it's defined in game data (params)
/// - HOW it gets set (EMEVD)

use std::collections::HashMap;
use std::path::Path;
use std::fs;

use serde::{Deserialize, Serialize};

use super::flag_catalog::{FlagCatalog, CatalogFlag};
use super::param_flags::{ParamFlagDb, ParamFlag, ParamSource, FlagCategory};
use super::event_graph::{EventGraph, FlagTrigger};

/// Unified flag entry combining all sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFlag {
    pub flag_id: u32,

    // === Catalog Data (display/spatial) ===
    pub name: Option<String>,
    pub category: Option<String>,
    pub region: Option<String>,
    pub map_tile: Option<String>,
    pub position: Option<Position>,
    pub item_info: Option<ItemInfo>,

    // === Param Data (source traceability) ===
    pub param_sources: Vec<ParamSourceInfo>,
    pub boss_name: Option<String>,
    pub flag_category: FlagCategory,

    // === Event Graph Data (EMEVD) ===
    pub emevd_triggers: Vec<TriggerInfo>,
    pub trigger_context: Option<String>,
    pub has_dependencies: bool,
    pub in_progression_chain: Option<String>,

    // === Derived/Computed ===
    pub source_count: usize,  // How many sources have this flag
    pub confidence: SourceConfidence,
}

/// Position data from catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub world_x: Option<f64>,
    pub world_z: Option<f64>,
}

/// Item association from catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemInfo {
    pub item_id: u32,
    pub item_category: Option<u32>,
    pub treasure_type: Option<String>,
}

/// Param source info (simplified for storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSourceInfo {
    pub param_name: String,
    pub row_id: u32,
    pub field_name: String,
}

impl From<&ParamSource> for ParamSourceInfo {
    fn from(source: &ParamSource) -> Self {
        Self {
            param_name: source.param_name().to_string(),
            row_id: source.row_id(),
            field_name: source.field_name().to_string(),
        }
    }
}

/// Trigger info from event graph (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInfo {
    pub event_id: u32,
    pub source_file: String,
    pub action: String,
    pub context: String,
}

impl From<&FlagTrigger> for TriggerInfo {
    fn from(trigger: &FlagTrigger) -> Self {
        Self {
            event_id: trigger.event_id,
            source_file: trigger.source_file.clone(),
            action: trigger.action.clone(),
            context: trigger.trigger_context.clone(),
        }
    }
}

/// Confidence level based on source coverage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceConfidence {
    /// All three sources agree
    High,
    /// Two sources have the flag
    Medium,
    /// Only one source
    Low,
    /// Computed/inferred only
    Inferred,
}

impl SourceConfidence {
    fn from_count(count: usize) -> Self {
        match count {
            3 => SourceConfidence::High,
            2 => SourceConfidence::Medium,
            1 => SourceConfidence::Low,
            _ => SourceConfidence::Inferred,
        }
    }
}

/// Statistics about the unified database
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnifiedStats {
    pub total_flags: usize,
    pub in_catalog: usize,
    pub in_params: usize,
    pub in_emevd: usize,
    pub in_all_three: usize,
    pub in_two: usize,
    pub in_one: usize,
    pub with_names: usize,
    pub with_positions: usize,
    pub with_triggers: usize,
    pub by_category: HashMap<String, usize>,
    pub by_param: HashMap<String, usize>,
    pub by_trigger_context: HashMap<String, usize>,
}

/// Metadata about the unified database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMetadata {
    pub build_date: String,
    pub catalog_flags: usize,
    pub param_flags: usize,
    pub emevd_flags: usize,
    pub stats: UnifiedStats,
}

/// Error type for unified database
#[derive(Debug)]
pub enum UnifiedDbError {
    IoError(std::io::Error),
    JsonError(String),
    SourceError(String),
}

impl std::fmt::Display for UnifiedDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnifiedDbError::IoError(e) => write!(f, "IO error: {}", e),
            UnifiedDbError::JsonError(msg) => write!(f, "JSON error: {}", msg),
            UnifiedDbError::SourceError(msg) => write!(f, "Source error: {}", msg),
        }
    }
}

impl std::error::Error for UnifiedDbError {}

impl From<std::io::Error> for UnifiedDbError {
    fn from(err: std::io::Error) -> Self {
        UnifiedDbError::IoError(err)
    }
}

/// The unified flag database
#[derive(Debug)]
pub struct UnifiedFlagDb {
    /// All flags indexed by ID
    flags: HashMap<u32, UnifiedFlag>,

    /// Flags by catalog category
    by_category: HashMap<String, Vec<u32>>,

    /// Flags by param source
    by_param: HashMap<String, Vec<u32>>,

    /// Flags by trigger context
    by_trigger_context: HashMap<String, Vec<u32>>,

    /// Flags by region
    by_region: HashMap<String, Vec<u32>>,

    /// Flags by confidence level
    by_confidence: HashMap<SourceConfidence, Vec<u32>>,

    /// Metadata
    pub metadata: UnifiedMetadata,
}

impl UnifiedFlagDb {
    /// Build unified database from all sources
    pub fn build(
        catalog: Option<&FlagCatalog>,
        param_db: Option<&ParamFlagDb>,
        event_graph: Option<&EventGraph>,
    ) -> Result<Self, UnifiedDbError> {
        let mut flags: HashMap<u32, UnifiedFlag> = HashMap::new();

        let mut catalog_count = 0;
        let mut param_count = 0;
        let mut emevd_count = 0;

        // Step 1: Add catalog flags
        if let Some(catalog) = catalog {
            for flag in catalog.iter() {
                catalog_count += 1;
                let entry = flags.entry(flag.flag_id).or_insert_with(|| UnifiedFlag::new(flag.flag_id));
                entry.merge_catalog(flag);
            }
        }

        // Step 2: Add param flags
        if let Some(param_db) = param_db {
            for flag_id in param_db.all_flag_ids() {
                param_count += 1;
                if let Some(param_flag) = param_db.get(flag_id) {
                    let entry = flags.entry(flag_id).or_insert_with(|| UnifiedFlag::new(flag_id));
                    entry.merge_param(param_flag, param_db.get_boss_name(flag_id));
                }
            }
        }

        // Step 3: Add event graph flags
        if let Some(graph) = event_graph {
            for flag_id in graph.get_all_flag_ids() {
                emevd_count += 1;
                let entry = flags.entry(flag_id).or_insert_with(|| UnifiedFlag::new(flag_id));
                entry.merge_event_graph(flag_id, graph);
            }
        }

        // Step 4: Compute derived fields
        for flag in flags.values_mut() {
            flag.compute_derived();
        }

        // Step 5: Build indexes
        let mut by_category: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_param: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_trigger_context: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_region: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_confidence: HashMap<SourceConfidence, Vec<u32>> = HashMap::new();

        let mut stats = UnifiedStats::default();
        stats.total_flags = flags.len();

        for (flag_id, flag) in &flags {
            // Index by category
            if let Some(ref cat) = flag.category {
                by_category.entry(cat.clone()).or_default().push(*flag_id);
                *stats.by_category.entry(cat.clone()).or_default() += 1;
            }

            // Index by param source
            for source in &flag.param_sources {
                by_param.entry(source.param_name.clone()).or_default().push(*flag_id);
                *stats.by_param.entry(source.param_name.clone()).or_default() += 1;
            }

            // Index by trigger context
            if let Some(ref ctx) = flag.trigger_context {
                by_trigger_context.entry(ctx.clone()).or_default().push(*flag_id);
                *stats.by_trigger_context.entry(ctx.clone()).or_default() += 1;
            }

            // Index by region
            if let Some(ref region) = flag.region {
                by_region.entry(region.clone()).or_default().push(*flag_id);
            }

            // Index by confidence
            by_confidence.entry(flag.confidence).or_default().push(*flag_id);

            // Count sources
            let in_catalog = flag.name.is_some() || flag.category.is_some();
            let in_params = !flag.param_sources.is_empty();
            let in_emevd = !flag.emevd_triggers.is_empty();

            if in_catalog { stats.in_catalog += 1; }
            if in_params { stats.in_params += 1; }
            if in_emevd { stats.in_emevd += 1; }
            if in_catalog && in_params && in_emevd { stats.in_all_three += 1; }

            let count = (in_catalog as usize) + (in_params as usize) + (in_emevd as usize);
            match count {
                2 => stats.in_two += 1,
                1 => stats.in_one += 1,
                _ => {}
            }

            if flag.name.is_some() { stats.with_names += 1; }
            if flag.position.is_some() { stats.with_positions += 1; }
            if !flag.emevd_triggers.is_empty() { stats.with_triggers += 1; }
        }

        let metadata = UnifiedMetadata {
            build_date: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            catalog_flags: catalog_count,
            param_flags: param_count,
            emevd_flags: emevd_count,
            stats,
        };

        Ok(Self {
            flags,
            by_category,
            by_param,
            by_trigger_context,
            by_region,
            by_confidence,
            metadata,
        })
    }

    /// Build from default locations
    pub fn build_default() -> Result<Self, UnifiedDbError> {
        // Load catalog
        let catalog = FlagCatalog::load_default().ok();

        // Load param database
        let param_db = if Path::new("param_flags.json").exists() {
            ParamFlagDb::load_from_json("param_flags.json").ok()
        } else {
            None
        };

        // Load event graph
        let event_graph = EventGraph::load_default().ok();

        Self::build(catalog.as_ref(), param_db.as_ref(), event_graph.as_ref())
    }

    /// Save to JSON
    pub fn save_to_json<P: AsRef<Path>>(&self, path: P) -> Result<(), UnifiedDbError> {
        let saved = SavedUnifiedDb {
            metadata: self.metadata.clone(),
            flags: self.flags.clone(),
        };
        let json = serde_json::to_string_pretty(&saved)
            .map_err(|e| UnifiedDbError::JsonError(e.to_string()))?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load from JSON
    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Self, UnifiedDbError> {
        let content = fs::read_to_string(path)?;
        let saved: SavedUnifiedDb = serde_json::from_str(&content)
            .map_err(|e| UnifiedDbError::JsonError(e.to_string()))?;

        // Rebuild indexes
        let mut by_category: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_param: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_trigger_context: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_region: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_confidence: HashMap<SourceConfidence, Vec<u32>> = HashMap::new();

        for (flag_id, flag) in &saved.flags {
            if let Some(ref cat) = flag.category {
                by_category.entry(cat.clone()).or_default().push(*flag_id);
            }
            for source in &flag.param_sources {
                by_param.entry(source.param_name.clone()).or_default().push(*flag_id);
            }
            if let Some(ref ctx) = flag.trigger_context {
                by_trigger_context.entry(ctx.clone()).or_default().push(*flag_id);
            }
            if let Some(ref region) = flag.region {
                by_region.entry(region.clone()).or_default().push(*flag_id);
            }
            by_confidence.entry(flag.confidence).or_default().push(*flag_id);
        }

        Ok(Self {
            flags: saved.flags,
            by_category,
            by_param,
            by_trigger_context,
            by_region,
            by_confidence,
            metadata: saved.metadata,
        })
    }

    // === Query Methods ===

    /// Get flag by ID
    pub fn get(&self, flag_id: u32) -> Option<&UnifiedFlag> {
        self.flags.get(&flag_id)
    }

    /// Check if flag exists
    pub fn has_flag(&self, flag_id: u32) -> bool {
        self.flags.contains_key(&flag_id)
    }

    /// Get all flags in a category
    pub fn flags_by_category(&self, category: &str) -> Vec<&UnifiedFlag> {
        self.by_category
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all flags from a param source
    pub fn flags_by_param(&self, param_name: &str) -> Vec<&UnifiedFlag> {
        self.by_param
            .get(param_name)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all flags with a trigger context
    pub fn flags_by_trigger_context(&self, context: &str) -> Vec<&UnifiedFlag> {
        self.by_trigger_context
            .get(context)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all flags in a region
    pub fn flags_by_region(&self, region: &str) -> Vec<&UnifiedFlag> {
        self.by_region
            .get(region)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all flags with a confidence level
    pub fn flags_by_confidence(&self, confidence: SourceConfidence) -> Vec<&UnifiedFlag> {
        self.by_confidence
            .get(&confidence)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get flags that are in params but NOT in EMEVD (potential formula candidates)
    pub fn flags_needing_formulas(&self) -> Vec<&UnifiedFlag> {
        self.flags.values()
            .filter(|f| !f.param_sources.is_empty() && f.emevd_triggers.is_empty())
            .collect()
    }

    /// Get flags that are in EMEVD but NOT in params
    pub fn flags_emevd_only(&self) -> Vec<&UnifiedFlag> {
        self.flags.values()
            .filter(|f| f.param_sources.is_empty() && !f.emevd_triggers.is_empty())
            .collect()
    }

    /// Search by name (case-insensitive)
    pub fn search_by_name(&self, query: &str) -> Vec<&UnifiedFlag> {
        let query_lower = query.to_lowercase();
        self.flags.values()
            .filter(|f| {
                f.name.as_ref().map(|n| n.to_lowercase().contains(&query_lower)).unwrap_or(false)
                || f.boss_name.as_ref().map(|n| n.to_lowercase().contains(&query_lower)).unwrap_or(false)
            })
            .collect()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<&str> {
        self.by_category.keys().map(|s| s.as_str()).collect()
    }

    /// Get all param sources
    pub fn param_sources(&self) -> Vec<&str> {
        self.by_param.keys().map(|s| s.as_str()).collect()
    }

    /// Get all trigger contexts
    pub fn trigger_contexts(&self) -> Vec<&str> {
        self.by_trigger_context.keys().map(|s| s.as_str()).collect()
    }

    /// Get all regions
    pub fn regions(&self) -> Vec<&str> {
        self.by_region.keys().map(|s| s.as_str()).collect()
    }

    /// Total flag count
    pub fn len(&self) -> usize {
        self.flags.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }

    /// Get stats
    pub fn stats(&self) -> &UnifiedStats {
        &self.metadata.stats
    }

    /// Print summary
    pub fn print_summary(&self) {
        let stats = &self.metadata.stats;
        println!("=== Unified Flag Database ===");
        println!();
        println!("Build date: {}", self.metadata.build_date);
        println!();
        println!("Source Coverage:");
        println!("  Catalog flags:    {:>6}", self.metadata.catalog_flags);
        println!("  Param flags:      {:>6}", self.metadata.param_flags);
        println!("  EMEVD flags:      {:>6}", self.metadata.emevd_flags);
        println!();
        println!("Total unique flags: {:>6}", stats.total_flags);
        println!();
        println!("Source Overlap:");
        println!("  In all three:     {:>6} ({:.1}%)",
            stats.in_all_three,
            100.0 * stats.in_all_three as f64 / stats.total_flags as f64);
        println!("  In two sources:   {:>6} ({:.1}%)",
            stats.in_two,
            100.0 * stats.in_two as f64 / stats.total_flags as f64);
        println!("  In one source:    {:>6} ({:.1}%)",
            stats.in_one,
            100.0 * stats.in_one as f64 / stats.total_flags as f64);
        println!();
        println!("Data Quality:");
        println!("  With names:       {:>6} ({:.1}%)",
            stats.with_names,
            100.0 * stats.with_names as f64 / stats.total_flags as f64);
        println!("  With positions:   {:>6} ({:.1}%)",
            stats.with_positions,
            100.0 * stats.with_positions as f64 / stats.total_flags as f64);
        println!("  With EMEVD:       {:>6} ({:.1}%)",
            stats.with_triggers,
            100.0 * stats.with_triggers as f64 / stats.total_flags as f64);
        println!();
        println!("Top Categories:");
        let mut cats: Vec<_> = stats.by_category.iter().collect();
        cats.sort_by(|a, b| b.1.cmp(a.1));
        for (cat, count) in cats.iter().take(10) {
            println!("  {:25} {:>6}", cat, count);
        }
        println!();
        println!("Top Trigger Contexts:");
        let mut ctxs: Vec<_> = stats.by_trigger_context.iter().collect();
        ctxs.sort_by(|a, b| b.1.cmp(a.1));
        for (ctx, count) in ctxs.iter().take(10) {
            println!("  {:25} {:>6}", ctx, count);
        }
    }
}

impl UnifiedFlag {
    fn new(flag_id: u32) -> Self {
        Self {
            flag_id,
            name: None,
            category: None,
            region: None,
            map_tile: None,
            position: None,
            item_info: None,
            param_sources: Vec::new(),
            boss_name: None,
            flag_category: FlagCategory::from_flag_id(flag_id),
            emevd_triggers: Vec::new(),
            trigger_context: None,
            has_dependencies: false,
            in_progression_chain: None,
            source_count: 0,
            confidence: SourceConfidence::Inferred,
        }
    }

    fn merge_catalog(&mut self, catalog: &CatalogFlag) {
        self.name = Some(catalog.name.clone());
        self.category = Some(catalog.category.clone());
        self.region = Some(catalog.region.clone());
        self.map_tile = catalog.map_tile.clone();

        // Position
        if catalog.pos_x.is_some() || catalog.pos_y.is_some() || catalog.pos_z.is_some() {
            self.position = Some(Position {
                pos_x: catalog.pos_x.unwrap_or(0.0),
                pos_y: catalog.pos_y.unwrap_or(0.0),
                pos_z: catalog.pos_z.unwrap_or(0.0),
                world_x: catalog.world_x,
                world_z: catalog.world_z,
            });
        }

        // Item info
        if let Some(item_id) = catalog.item_id {
            self.item_info = Some(ItemInfo {
                item_id,
                item_category: catalog.item_category,
                treasure_type: catalog.treasure_type.clone(),
            });
        }
    }

    fn merge_param(&mut self, param: &ParamFlag, boss_name: Option<&str>) {
        self.param_sources = param.sources.iter().map(|s| s.into()).collect();
        self.boss_name = boss_name.map(|s| s.to_string());
        self.flag_category = param.category;
    }

    fn merge_event_graph(&mut self, flag_id: u32, graph: &EventGraph) {
        if let Some(triggers) = graph.get_triggers(flag_id) {
            self.emevd_triggers = triggers.iter().map(|t| t.into()).collect();
            if let Some(first) = triggers.first() {
                self.trigger_context = Some(first.trigger_context.clone());
            }
        }

        // Check dependencies
        if let Some(deps) = graph.get_dependencies(flag_id) {
            self.has_dependencies = !deps.is_empty();
        }

        // Check progression chains
        if graph.find_remembrance_chain(flag_id).is_some() {
            self.in_progression_chain = Some("remembrance".to_string());
        } else if graph.find_map_fragment_chain(flag_id).is_some() {
            self.in_progression_chain = Some("map_fragment".to_string());
        }
    }

    fn compute_derived(&mut self) {
        // Count sources
        let in_catalog = self.name.is_some() || self.category.is_some();
        let in_params = !self.param_sources.is_empty();
        let in_emevd = !self.emevd_triggers.is_empty();

        self.source_count = (in_catalog as usize) + (in_params as usize) + (in_emevd as usize);
        self.confidence = SourceConfidence::from_count(self.source_count);
    }

    /// Get a display name (prefer catalog name, fall back to boss name or generated)
    pub fn display_name(&self) -> String {
        self.name.clone()
            .or_else(|| self.boss_name.clone())
            .unwrap_or_else(|| format!("Flag {}", self.flag_id))
    }

    /// Check if flag has catalog data
    pub fn has_catalog_data(&self) -> bool {
        self.name.is_some() || self.category.is_some()
    }

    /// Check if flag has param data
    pub fn has_param_data(&self) -> bool {
        !self.param_sources.is_empty()
    }

    /// Check if flag has EMEVD data
    pub fn has_emevd_data(&self) -> bool {
        !self.emevd_triggers.is_empty()
    }
}

/// Serializable version
#[derive(Debug, Serialize, Deserialize)]
struct SavedUnifiedDb {
    metadata: UnifiedMetadata,
    flags: HashMap<u32, UnifiedFlag>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_unified_db() {
        let db = UnifiedFlagDb::build_default();
        match db {
            Ok(db) => {
                db.print_summary();
                assert!(db.len() > 0);
            }
            Err(e) => {
                eprintln!("Failed to build: {}", e);
            }
        }
    }

    #[test]
    fn test_query_flag() {
        if let Ok(db) = UnifiedFlagDb::build_default() {
            // Query First Step grace
            if let Some(flag) = db.get(76100) {
                println!("Flag 76100:");
                println!("  Name: {:?}", flag.name);
                println!("  Category: {:?}", flag.category);
                println!("  Param sources: {:?}", flag.param_sources.len());
                println!("  EMEVD triggers: {:?}", flag.emevd_triggers.len());
                println!("  Confidence: {:?}", flag.confidence);
            }
        }
    }
}
