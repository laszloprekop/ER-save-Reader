/// Param Flags Module
///
/// Extracts event flag IDs from decompiled regulation-bin XML param files.
/// These params provide ground truth for what flags exist and their semantic meaning.
///
/// Supported param files and their flag fields:
/// - ItemLotParam_map: getItemFlagId (world pickup flags)
/// - BonfireWarpParam: eventflagId, clearedEventFlagId (grace discovery)
/// - WorldMapPointParam: eventFlagId (map marker discovery)
/// - ShopLineupParam: eventFlag_forRelease, eventFlag_forStock (shop unlocks)
/// - GameAreaParam: defeatBossFlagId, foundBossFlagId, bossChallengeFlagId (boss defeat)
/// - NpcParam: Flag_Alive, Flag_Dead (NPC state)

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Source of a flag (which param file and field)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParamSource {
    ItemLotMap { row_id: u32, field: String },
    BonfireWarp { row_id: u32, field: String },
    WorldMapPoint { row_id: u32, field: String },
    ShopLineup { row_id: u32, field: String },
    GameArea { row_id: u32, field: String, boss_name: Option<String> },
    NpcParam { row_id: u32, field: String },
}

impl ParamSource {
    pub fn param_name(&self) -> &'static str {
        match self {
            ParamSource::ItemLotMap { .. } => "ItemLotParam_map",
            ParamSource::BonfireWarp { .. } => "BonfireWarpParam",
            ParamSource::WorldMapPoint { .. } => "WorldMapPointParam",
            ParamSource::ShopLineup { .. } => "ShopLineupParam",
            ParamSource::GameArea { .. } => "GameAreaParam",
            ParamSource::NpcParam { .. } => "NpcParam",
        }
    }

    pub fn field_name(&self) -> &str {
        match self {
            ParamSource::ItemLotMap { field, .. } => field,
            ParamSource::BonfireWarp { field, .. } => field,
            ParamSource::WorldMapPoint { field, .. } => field,
            ParamSource::ShopLineup { field, .. } => field,
            ParamSource::GameArea { field, .. } => field,
            ParamSource::NpcParam { field, .. } => field,
        }
    }

    pub fn row_id(&self) -> u32 {
        match self {
            ParamSource::ItemLotMap { row_id, .. } => *row_id,
            ParamSource::BonfireWarp { row_id, .. } => *row_id,
            ParamSource::WorldMapPoint { row_id, .. } => *row_id,
            ParamSource::ShopLineup { row_id, .. } => *row_id,
            ParamSource::GameArea { row_id, .. } => *row_id,
            ParamSource::NpcParam { row_id, .. } => *row_id,
        }
    }
}

/// A flag entry with its sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamFlag {
    pub flag_id: u32,
    pub sources: Vec<ParamSource>,
    pub category: FlagCategory,
}

/// Categorization of flag by its ID range
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlagCategory {
    Simple,      // < 60000
    Block,       // 60000-99999
    Midrange,    // 100000-999999
    Dungeon,     // 10M-44M
    Tile,        // 1B+
}

impl FlagCategory {
    pub fn from_flag_id(flag_id: u32) -> Self {
        match flag_id {
            0..=59_999 => FlagCategory::Simple,
            60_000..=99_999 => FlagCategory::Block,
            100_000..=999_999 => FlagCategory::Midrange,
            1_000_000..=999_999_999 => FlagCategory::Dungeon,
            _ => FlagCategory::Tile,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            FlagCategory::Simple => "simple",
            FlagCategory::Block => "block",
            FlagCategory::Midrange => "midrange",
            FlagCategory::Dungeon => "dungeon",
            FlagCategory::Tile => "tile",
        }
    }
}

/// Statistics about extracted flags
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionStats {
    pub total_flags: usize,
    pub by_param: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
    pub by_block: HashMap<u32, usize>,  // For midrange: block_start -> count
}

/// Metadata about the extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionMetadata {
    pub extraction_date: String,
    pub source_directory: String,
    pub params_processed: Vec<String>,
    pub stats: ExtractionStats,
}

/// Error type for param extraction
#[derive(Debug)]
pub enum ParamError {
    IoError(std::io::Error),
    XmlError(String),
    NotFound(String),
}

impl std::fmt::Display for ParamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamError::IoError(e) => write!(f, "IO error: {}", e),
            ParamError::XmlError(msg) => write!(f, "XML parse error: {}", msg),
            ParamError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for ParamError {}

impl From<std::io::Error> for ParamError {
    fn from(err: std::io::Error) -> Self {
        ParamError::IoError(err)
    }
}

/// The main param flags database
#[derive(Debug)]
pub struct ParamFlagDb {
    /// All flags indexed by flag ID
    flags: HashMap<u32, ParamFlag>,

    /// Flags grouped by param source
    by_param: HashMap<String, Vec<u32>>,

    /// Flags grouped by category
    by_category: HashMap<FlagCategory, Vec<u32>>,

    /// Flags grouped by block (for midrange)
    by_block: HashMap<u32, Vec<u32>>,

    /// Boss defeat flags with names
    boss_flags: HashMap<u32, String>,

    /// Extraction metadata
    pub metadata: ExtractionMetadata,
}

impl ParamFlagDb {
    /// Extract flags from a regulation-bin directory
    pub fn extract_from_directory<P: AsRef<Path>>(dir: P) -> Result<Self, ParamError> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Err(ParamError::NotFound(format!(
                "Directory not found: {}",
                dir.display()
            )));
        }

        let mut flags: HashMap<u32, ParamFlag> = HashMap::new();
        let mut boss_flags: HashMap<u32, String> = HashMap::new();
        let mut params_processed = Vec::new();

        // Extract from each param file
        let extractors: Vec<(&str, fn(&Path, &mut HashMap<u32, ParamFlag>, &mut HashMap<u32, String>) -> Result<usize, ParamError>)> = vec![
            ("ItemLotParam_map.param.xml", extract_item_lot_flags),
            ("BonfireWarpParam.param.xml", extract_bonfire_flags),
            ("WorldMapPointParam.param.xml", extract_world_map_flags),
            ("ShopLineupParam.param.xml", extract_shop_flags),
            ("GameAreaParam.param.xml", extract_game_area_flags),
            ("NpcParam.param.xml", extract_npc_flags),
        ];

        for (filename, extractor) in extractors {
            let path = dir.join(filename);
            if path.exists() {
                match extractor(&path, &mut flags, &mut boss_flags) {
                    Ok(count) => {
                        eprintln!("Extracted {} flags from {}", count, filename);
                        params_processed.push(filename.to_string());
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to extract from {}: {}", filename, e);
                    }
                }
            } else {
                eprintln!("Warning: {} not found", filename);
            }
        }

        // Build indexes
        let mut by_param: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_category: HashMap<FlagCategory, Vec<u32>> = HashMap::new();
        let mut by_block: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut stats = ExtractionStats::default();

        for (flag_id, flag) in &flags {
            // Index by param
            for source in &flag.sources {
                by_param
                    .entry(source.param_name().to_string())
                    .or_default()
                    .push(*flag_id);
            }

            // Index by category
            by_category
                .entry(flag.category)
                .or_default()
                .push(*flag_id);

            // Index by block (for midrange)
            if flag.category == FlagCategory::Midrange {
                let block = (*flag_id / 1000) * 1000;
                by_block.entry(block).or_default().push(*flag_id);
                *stats.by_block.entry(block).or_default() += 1;
            }
        }

        // Build stats
        stats.total_flags = flags.len();
        for (param, ids) in &by_param {
            stats.by_param.insert(param.clone(), ids.len());
        }
        for (category, ids) in &by_category {
            stats.by_category.insert(category.name().to_string(), ids.len());
        }

        let metadata = ExtractionMetadata {
            extraction_date: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            source_directory: dir.to_string_lossy().to_string(),
            params_processed,
            stats,
        };

        Ok(Self {
            flags,
            by_param,
            by_category,
            by_block,
            boss_flags,
            metadata,
        })
    }

    /// Load from a previously saved JSON file
    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Self, ParamError> {
        let content = fs::read_to_string(path)?;
        let saved: SavedParamFlagDb = serde_json::from_str(&content)
            .map_err(|e| ParamError::XmlError(format!("JSON parse error: {}", e)))?;

        // Rebuild indexes
        let mut by_param: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_category: HashMap<FlagCategory, Vec<u32>> = HashMap::new();
        let mut by_block: HashMap<u32, Vec<u32>> = HashMap::new();

        for (flag_id, flag) in &saved.flags {
            for source in &flag.sources {
                by_param
                    .entry(source.param_name().to_string())
                    .or_default()
                    .push(*flag_id);
            }
            by_category
                .entry(flag.category)
                .or_default()
                .push(*flag_id);
            if flag.category == FlagCategory::Midrange {
                let block = (*flag_id / 1000) * 1000;
                by_block.entry(block).or_default().push(*flag_id);
            }
        }

        Ok(Self {
            flags: saved.flags,
            by_param,
            by_category,
            by_block,
            boss_flags: saved.boss_flags,
            metadata: saved.metadata,
        })
    }

    /// Save to JSON file
    pub fn save_to_json<P: AsRef<Path>>(&self, path: P) -> Result<(), ParamError> {
        let saved = SavedParamFlagDb {
            metadata: self.metadata.clone(),
            flags: self.flags.clone(),
            boss_flags: self.boss_flags.clone(),
        };
        let json = serde_json::to_string_pretty(&saved)
            .map_err(|e| ParamError::XmlError(format!("JSON serialization error: {}", e)))?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Get flag by ID
    pub fn get(&self, flag_id: u32) -> Option<&ParamFlag> {
        self.flags.get(&flag_id)
    }

    /// Check if a flag exists in params
    pub fn has_flag(&self, flag_id: u32) -> bool {
        self.flags.contains_key(&flag_id)
    }

    /// Get boss name for a defeat flag
    pub fn get_boss_name(&self, flag_id: u32) -> Option<&str> {
        self.boss_flags.get(&flag_id).map(|s| s.as_str())
    }

    /// Get all flags from a specific param
    pub fn flags_from_param(&self, param_name: &str) -> Vec<&ParamFlag> {
        self.by_param
            .get(param_name)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all flags in a category
    pub fn flags_in_category(&self, category: FlagCategory) -> Vec<&ParamFlag> {
        self.by_category
            .get(&category)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all flags in a block (for midrange)
    pub fn flags_in_block(&self, block_start: u32) -> Vec<&ParamFlag> {
        self.by_block
            .get(&block_start)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all block starts that have flags
    pub fn midrange_blocks(&self) -> Vec<u32> {
        let mut blocks: Vec<u32> = self.by_block.keys().copied().collect();
        blocks.sort();
        blocks
    }

    /// Get all flag IDs
    pub fn all_flag_ids(&self) -> Vec<u32> {
        self.flags.keys().copied().collect()
    }

    /// Total number of flags
    pub fn len(&self) -> usize {
        self.flags.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }

    /// Get statistics
    pub fn stats(&self) -> &ExtractionStats {
        &self.metadata.stats
    }

    /// Print summary
    pub fn print_summary(&self) {
        println!("Param Flag Database Summary");
        println!("===========================");
        println!("Total flags: {}", self.flags.len());
        println!("Extraction: {}", self.metadata.extraction_date);
        println!();
        println!("By Param:");
        for (param, count) in &self.metadata.stats.by_param {
            println!("  {}: {}", param, count);
        }
        println!();
        println!("By Category:");
        for (category, count) in &self.metadata.stats.by_category {
            println!("  {}: {}", category, count);
        }
        println!();
        println!("Midrange Blocks (top 10):");
        let mut blocks: Vec<_> = self.metadata.stats.by_block.iter().collect();
        blocks.sort_by(|a, b| b.1.cmp(a.1));
        for (block, count) in blocks.iter().take(10) {
            println!("  {}: {} flags", block, count);
        }
        println!();
        println!("Boss flags with names: {}", self.boss_flags.len());
    }
}

/// Serializable version for JSON persistence
#[derive(Debug, Serialize, Deserialize)]
struct SavedParamFlagDb {
    metadata: ExtractionMetadata,
    flags: HashMap<u32, ParamFlag>,
    boss_flags: HashMap<u32, String>,
}

// === Extraction Functions ===

fn extract_item_lot_flags(
    path: &Path,
    flags: &mut HashMap<u32, ParamFlag>,
    _boss_flags: &mut HashMap<u32, String>,
) -> Result<usize, ParamError> {
    let content = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&content)
        .map_err(|e| ParamError::XmlError(e.to_string()))?;

    let mut count = 0;

    for row in doc.descendants().filter(|n| n.has_tag_name("row")) {
        let row_id: u32 = row
            .attribute("id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        // CRITICAL DISCOVERY (2026-01-23): For tile-based world pickups, the game uses
        // the ROW ID as the actual stored event flag, NOT getItemFlagId.
        //
        // ItemLotParam has getItemFlagId = row_id + 7000, which places the local_id
        // in the 7000+ range. But tile slots only allocate 875 bytes (7000 flags),
        // so local_id >= 7000 has NO storage.
        //
        // Save file diff analysis confirmed: when picking up item lot 1044360310,
        // flag 1044360310 (row_id, local_id 310) is SET, not 1044367310 (getItemFlagId, local_id 7310).
        //
        // Therefore: for tile-based pickups (10-digit IDs starting with 1 or 2),
        // we use row_id as the flag_id for tracking purposes.

        let is_tile_based = row_id >= 1_000_000_000 && row_id < 3_000_000_000;

        if is_tile_based {
            // For tile-based world pickups, use row_id as the actual flag
            if row_id > 0 {
                add_flag_source(
                    flags,
                    row_id,
                    ParamSource::ItemLotMap {
                        row_id,
                        field: "row_id".to_string(),
                    },
                );
                count += 1;
            }
        } else {
            // For non-tile pickups (dungeons, shops, etc.), use getItemFlagId as before
            if let Some(flag_str) = row.attribute("getItemFlagId") {
                if let Ok(flag_id) = flag_str.parse::<u32>() {
                    if flag_id > 0 {
                        add_flag_source(
                            flags,
                            flag_id,
                            ParamSource::ItemLotMap {
                                row_id,
                                field: "getItemFlagId".to_string(),
                            },
                        );
                        count += 1;
                    }
                }
            }
        }

        // Extract getItemFlagId01-08 (per-slot flags) - these are used for multi-item lots
        // and may follow different rules
        for i in 1..=8 {
            let field_name = format!("getItemFlagId{:02}", i);
            if let Some(flag_str) = row.attribute(field_name.as_str()) {
                if let Ok(flag_id) = flag_str.parse::<u32>() {
                    if flag_id > 0 {
                        add_flag_source(
                            flags,
                            flag_id,
                            ParamSource::ItemLotMap {
                                row_id,
                                field: field_name,
                            },
                        );
                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

fn extract_bonfire_flags(
    path: &Path,
    flags: &mut HashMap<u32, ParamFlag>,
    _boss_flags: &mut HashMap<u32, String>,
) -> Result<usize, ParamError> {
    let content = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&content)
        .map_err(|e| ParamError::XmlError(e.to_string()))?;

    let mut count = 0;

    for row in doc.descendants().filter(|n| n.has_tag_name("row")) {
        let row_id: u32 = row
            .attribute("id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        // Main discovery flag
        if let Some(flag_str) = row.attribute("eventflagId") {
            if let Ok(flag_id) = flag_str.parse::<u32>() {
                if flag_id > 0 {
                    add_flag_source(
                        flags,
                        flag_id,
                        ParamSource::BonfireWarp {
                            row_id,
                            field: "eventflagId".to_string(),
                        },
                    );
                    count += 1;
                }
            }
        }

        // Cleared flag
        if let Some(flag_str) = row.attribute("clearedEventFlagId") {
            if let Ok(flag_id) = flag_str.parse::<u32>() {
                if flag_id > 0 {
                    add_flag_source(
                        flags,
                        flag_id,
                        ParamSource::BonfireWarp {
                            row_id,
                            field: "clearedEventFlagId".to_string(),
                        },
                    );
                    count += 1;
                }
            }
        }

        // Text enable/disable flags
        for i in 1..=8 {
            for prefix in &["textEnableFlagId", "textDisableFlagId", "textEnableFlag2Id", "textDisableFlag2Id"] {
                let field_name = format!("{}{}", prefix, i);
                if let Some(flag_str) = row.attribute(field_name.as_str()) {
                    if let Ok(flag_id) = flag_str.parse::<u32>() {
                        if flag_id > 0 {
                            add_flag_source(
                                flags,
                                flag_id,
                                ParamSource::BonfireWarp {
                                    row_id,
                                    field: field_name,
                                },
                            );
                            count += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(count)
}

fn extract_world_map_flags(
    path: &Path,
    flags: &mut HashMap<u32, ParamFlag>,
    _boss_flags: &mut HashMap<u32, String>,
) -> Result<usize, ParamError> {
    let content = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&content)
        .map_err(|e| ParamError::XmlError(e.to_string()))?;

    let mut count = 0;

    for row in doc.descendants().filter(|n| n.has_tag_name("row")) {
        let row_id: u32 = row
            .attribute("id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if let Some(flag_str) = row.attribute("eventFlagId") {
            if let Ok(flag_id) = flag_str.parse::<u32>() {
                if flag_id > 0 {
                    add_flag_source(
                        flags,
                        flag_id,
                        ParamSource::WorldMapPoint {
                            row_id,
                            field: "eventFlagId".to_string(),
                        },
                    );
                    count += 1;
                }
            }
        }

        if let Some(flag_str) = row.attribute("distViewEventFlagId") {
            if let Ok(flag_id) = flag_str.parse::<u32>() {
                if flag_id > 0 {
                    add_flag_source(
                        flags,
                        flag_id,
                        ParamSource::WorldMapPoint {
                            row_id,
                            field: "distViewEventFlagId".to_string(),
                        },
                    );
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

fn extract_shop_flags(
    path: &Path,
    flags: &mut HashMap<u32, ParamFlag>,
    _boss_flags: &mut HashMap<u32, String>,
) -> Result<usize, ParamError> {
    let content = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&content)
        .map_err(|e| ParamError::XmlError(e.to_string()))?;

    let mut count = 0;

    for row in doc.descendants().filter(|n| n.has_tag_name("row")) {
        let row_id: u32 = row
            .attribute("id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        for field in &["eventFlag_forStock", "eventFlag_forRelease"] {
            if let Some(flag_str) = row.attribute(*field) {
                if let Ok(flag_id) = flag_str.parse::<u32>() {
                    if flag_id > 0 {
                        add_flag_source(
                            flags,
                            flag_id,
                            ParamSource::ShopLineup {
                                row_id,
                                field: field.to_string(),
                            },
                        );
                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

fn extract_game_area_flags(
    path: &Path,
    flags: &mut HashMap<u32, ParamFlag>,
    boss_flags: &mut HashMap<u32, String>,
) -> Result<usize, ParamError> {
    let content = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&content)
        .map_err(|e| ParamError::XmlError(e.to_string()))?;

    let mut count = 0;

    for row in doc.descendants().filter(|n| n.has_tag_name("row")) {
        let row_id: u32 = row
            .attribute("id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        // Get boss name if available (paramdexName attribute)
        let boss_name = row.attribute("paramdexName").map(|s| s.to_string());

        for field in &["defeatBossFlagId", "foundBossFlagId", "bossChallengeFlagId", "defeatBossFlagId_forSignAimList", "displayAimFlagId"] {
            if let Some(flag_str) = row.attribute(*field) {
                if let Ok(flag_id) = flag_str.parse::<u32>() {
                    if flag_id > 0 {
                        add_flag_source(
                            flags,
                            flag_id,
                            ParamSource::GameArea {
                                row_id,
                                field: field.to_string(),
                                boss_name: boss_name.clone(),
                            },
                        );

                        // Store boss name for defeat flags
                        if *field == "defeatBossFlagId" {
                            if let Some(ref name) = boss_name {
                                boss_flags.insert(flag_id, name.clone());
                            }
                        }

                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

fn extract_npc_flags(
    path: &Path,
    flags: &mut HashMap<u32, ParamFlag>,
    _boss_flags: &mut HashMap<u32, String>,
) -> Result<usize, ParamError> {
    let content = fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&content)
        .map_err(|e| ParamError::XmlError(e.to_string()))?;

    let mut count = 0;

    for row in doc.descendants().filter(|n| n.has_tag_name("row")) {
        let row_id: u32 = row
            .attribute("id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        for field in &["Flag_Alive", "Flag_Dead"] {
            if let Some(flag_str) = row.attribute(*field) {
                if let Ok(flag_id) = flag_str.parse::<u32>() {
                    if flag_id > 0 {
                        add_flag_source(
                            flags,
                            flag_id,
                            ParamSource::NpcParam {
                                row_id,
                                field: field.to_string(),
                            },
                        );
                        count += 1;
                    }
                }
            }
        }
    }

    Ok(count)
}

fn add_flag_source(flags: &mut HashMap<u32, ParamFlag>, flag_id: u32, source: ParamSource) {
    let entry = flags.entry(flag_id).or_insert_with(|| ParamFlag {
        flag_id,
        sources: Vec::new(),
        category: FlagCategory::from_flag_id(flag_id),
    });
    entry.sources.push(source);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_category() {
        assert_eq!(FlagCategory::from_flag_id(100), FlagCategory::Simple);
        assert_eq!(FlagCategory::from_flag_id(60000), FlagCategory::Block);
        assert_eq!(FlagCategory::from_flag_id(76100), FlagCategory::Block);
        assert_eq!(FlagCategory::from_flag_id(400000), FlagCategory::Midrange);
        assert_eq!(FlagCategory::from_flag_id(510000), FlagCategory::Midrange);
        assert_eq!(FlagCategory::from_flag_id(10000800), FlagCategory::Dungeon);
        assert_eq!(FlagCategory::from_flag_id(1060420000), FlagCategory::Tile);
    }

    #[test]
    fn test_extract_from_directory() {
        let dir = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/regulation-bin");
        if dir.exists() {
            let db = ParamFlagDb::extract_from_directory(dir);
            match db {
                Ok(db) => {
                    db.print_summary();
                    assert!(db.len() > 0, "Should extract some flags");
                }
                Err(e) => {
                    eprintln!("Extraction failed: {}", e);
                }
            }
        } else {
            eprintln!("Test skipped - regulation-bin not found");
        }
    }
}
