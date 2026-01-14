/// Flag Catalog Module
///
/// Loads and indexes the comprehensive flag catalog from extracted_event_flags.json
/// providing search, autocomplete, and lookup functionality for 7034+ event flags.
///
/// This catalog contains rich metadata for each flag including:
/// - Names, categories, and regions
/// - Coordinates (world and local)
/// - Source information (MSB files, params)
/// - Item associations

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::fs::File;
use std::io::BufReader;

use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{self, Visitor};

/// Custom deserializer for item_rarity which can be integer or string
fn deserialize_rarity<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct RarityVisitor;

    impl<'de> Visitor<'de> for RarityVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer, string, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
            Ok(Some(value.to_string()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
            Ok(Some(value.to_string()))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
            Ok(Some(value.to_string()))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
            Ok(Some(value))
        }
    }

    deserializer.deserialize_any(RarityVisitor)
}

/// A single flag entry from the extracted catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogFlag {
    pub flag_id: u32,
    pub name: String,
    pub category: String,
    pub region: String,

    // Source information
    #[serde(default)]
    pub source_file: Option<String>,
    #[serde(default)]
    pub source_row_id: Option<u32>,

    // Item associations
    #[serde(default)]
    pub item_id: Option<u32>,
    #[serde(default)]
    pub item_category: Option<u32>,  // 1=Goods, 2=Weapons, 3=Protector, 4=Accessory, 5=Ash of War

    // Map location
    #[serde(default)]
    pub area_no: Option<u32>,
    #[serde(default)]
    pub grid_x: Option<i32>,
    #[serde(default)]
    pub grid_z: Option<i32>,
    #[serde(default)]
    pub map_tile: Option<String>,

    // World coordinates
    #[serde(default)]
    pub pos_x: Option<f32>,
    #[serde(default)]
    pub pos_y: Option<f32>,
    #[serde(default)]
    pub pos_z: Option<f32>,
    #[serde(default)]
    pub world_x: Option<f64>,
    #[serde(default)]
    pub world_z: Option<f64>,

    // Area classification
    #[serde(default)]
    pub region_id: Option<u32>,
    #[serde(default)]
    pub is_overworld: Option<bool>,
    #[serde(default)]
    pub area_type: Option<String>,
    #[serde(default)]
    pub is_dlc: bool,
    #[serde(default)]
    pub is_underground: Option<bool>,

    // Item metadata
    #[serde(default)]
    pub treasure_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_rarity")]
    pub item_rarity: Option<String>,  // Can be integer or string in JSON
    #[serde(default)]
    pub position_confidence: Option<String>,
}

/// Metadata from the catalog file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogMetadata {
    pub extraction_date: String,
    pub total_flags: usize,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub category_counts: HashMap<String, usize>,
}

/// Raw catalog file structure
#[derive(Debug, Deserialize)]
struct CatalogFile {
    metadata: CatalogMetadata,
    flags: Vec<CatalogFlag>,
}

/// The main flag catalog providing search and lookup functionality
#[derive(Debug)]
pub struct FlagCatalog {
    /// All flags indexed by ID
    flags: HashMap<u32, CatalogFlag>,

    /// Flags indexed by lowercase name for search
    by_name: BTreeMap<String, Vec<u32>>,

    /// Flags grouped by category
    by_category: HashMap<String, Vec<u32>>,

    /// Flags grouped by region
    by_region: HashMap<String, Vec<u32>>,

    /// Flags grouped by map tile
    by_map_tile: HashMap<String, Vec<u32>>,

    /// Original metadata
    pub metadata: CatalogMetadata,
}

impl FlagCatalog {
    /// Load the catalog from JSON file
    pub fn load_from_json(path: &Path) -> Result<Self, CatalogError> {
        let file = File::open(path)
            .map_err(|e| CatalogError::IoError(format!("Failed to open catalog: {}", e)))?;

        let reader = BufReader::new(file);
        let catalog_file: CatalogFile = serde_json::from_reader(reader)
            .map_err(|e| CatalogError::ParseError(format!("Failed to parse catalog: {}", e)))?;

        Self::from_flags(catalog_file.flags, catalog_file.metadata)
    }

    /// Load from default location (scripts/extracted_event_flags.json)
    pub fn load_default() -> Result<Self, CatalogError> {
        // Try relative path first (for normal operation)
        let default_path = Path::new("scripts/extracted_event_flags.json");
        if default_path.exists() {
            return Self::load_from_json(default_path);
        }

        // Try absolute path (for tests)
        let abs_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/scripts/extracted_event_flags.json");
        if abs_path.exists() {
            return Self::load_from_json(abs_path);
        }

        Err(CatalogError::NotFound("Could not find extracted_event_flags.json".to_string()))
    }

    /// Build catalog from a list of flags
    fn from_flags(flags: Vec<CatalogFlag>, metadata: CatalogMetadata) -> Result<Self, CatalogError> {
        let mut flags_map = HashMap::with_capacity(flags.len());
        let mut by_name: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        let mut by_category: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_region: HashMap<String, Vec<u32>> = HashMap::new();
        let mut by_map_tile: HashMap<String, Vec<u32>> = HashMap::new();

        for flag in flags {
            let flag_id = flag.flag_id;

            // Index by name (lowercase for case-insensitive search)
            let name_lower = flag.name.to_lowercase();
            // Index by each word in the name for better search
            for word in name_lower.split_whitespace() {
                by_name.entry(word.to_string()).or_default().push(flag_id);
            }
            // Also index the full name
            by_name.entry(name_lower).or_default().push(flag_id);

            // Index by category
            by_category.entry(flag.category.clone()).or_default().push(flag_id);

            // Index by region
            if !flag.region.is_empty() {
                by_region.entry(flag.region.clone()).or_default().push(flag_id);
            }

            // Index by map tile
            if let Some(ref tile) = flag.map_tile {
                by_map_tile.entry(tile.clone()).or_default().push(flag_id);
            }

            flags_map.insert(flag_id, flag);
        }

        Ok(Self {
            flags: flags_map,
            by_name,
            by_category,
            by_region,
            by_map_tile,
            metadata,
        })
    }

    /// Get a flag by its ID
    pub fn get_by_id(&self, flag_id: u32) -> Option<&CatalogFlag> {
        self.flags.get(&flag_id)
    }

    /// Search flags by name (case-insensitive, partial match)
    ///
    /// Supports both single-word and multi-word queries:
    /// - Single word: finds flags where any word in name starts with query
    /// - Multi-word: finds flags that contain all query words (in any order)
    pub fn search_by_name(&self, query: &str) -> Vec<&CatalogFlag> {
        let query_lower = query.to_lowercase().trim().to_string();
        if query_lower.is_empty() {
            return Vec::new();
        }

        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        if query_words.len() == 1 {
            // Single word search - find flags where any word starts with query
            let word = query_words[0];
            for (indexed_word, flag_ids) in self.by_name.range(word.to_string()..) {
                if !indexed_word.starts_with(word) {
                    break;
                }
                for &flag_id in flag_ids {
                    if seen.insert(flag_id) {
                        if let Some(flag) = self.flags.get(&flag_id) {
                            results.push(flag);
                        }
                    }
                }
            }
        } else {
            // Multi-word search - find flags containing all words
            // Start with flags matching the first word
            let first_word = query_words[0];
            for (indexed_word, flag_ids) in self.by_name.range(first_word.to_string()..) {
                if !indexed_word.starts_with(first_word) {
                    break;
                }
                for &flag_id in flag_ids {
                    if seen.insert(flag_id) {
                        if let Some(flag) = self.flags.get(&flag_id) {
                            // Check if all other query words are present
                            let flag_name_lower = flag.name.to_lowercase();
                            let all_match = query_words[1..].iter().all(|qw| {
                                flag_name_lower.split_whitespace().any(|fw| fw.starts_with(qw))
                            });
                            if all_match {
                                results.push(flag);
                            }
                        }
                    }
                }
            }
        }

        // Sort by relevance (exact matches first, then by name length)
        results.sort_by(|a, b| {
            let a_name_lower = a.name.to_lowercase();
            let b_name_lower = b.name.to_lowercase();

            // Exact match first
            let a_exact = a_name_lower == query_lower;
            let b_exact = b_name_lower == query_lower;
            if a_exact != b_exact {
                return if a_exact { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater };
            }

            // Contains query as substring (higher priority)
            let a_contains = a_name_lower.contains(&query_lower);
            let b_contains = b_name_lower.contains(&query_lower);
            if a_contains != b_contains {
                return if a_contains { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater };
            }

            // Shorter names first (more relevant)
            a.name.len().cmp(&b.name.len())
        });

        results
    }

    /// Get autocomplete suggestions for a partial query
    pub fn autocomplete(&self, partial: &str, limit: usize) -> Vec<&CatalogFlag> {
        self.search_by_name(partial).into_iter().take(limit).collect()
    }

    /// Get all flags in a category
    pub fn flags_for_category(&self, category: &str) -> Vec<&CatalogFlag> {
        self.by_category
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all flags in a region
    pub fn flags_for_region(&self, region: &str) -> Vec<&CatalogFlag> {
        self.by_region
            .get(region)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all flags for a map tile
    pub fn flags_for_map_tile(&self, map_tile: &str) -> Vec<&CatalogFlag> {
        self.by_map_tile
            .get(map_tile)
            .map(|ids| ids.iter().filter_map(|id| self.flags.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all available categories
    pub fn categories(&self) -> Vec<&str> {
        self.by_category.keys().map(|s| s.as_str()).collect()
    }

    /// Get all available regions
    pub fn regions(&self) -> Vec<&str> {
        self.by_region.keys().map(|s| s.as_str()).collect()
    }

    /// Get total number of flags
    pub fn len(&self) -> usize {
        self.flags.len()
    }

    /// Check if catalog is empty
    pub fn is_empty(&self) -> bool {
        self.flags.is_empty()
    }

    /// Iterate over all flags
    pub fn iter(&self) -> impl Iterator<Item = &CatalogFlag> {
        self.flags.values()
    }

    /// Get flags by a list of IDs
    pub fn get_many(&self, flag_ids: &[u32]) -> Vec<&CatalogFlag> {
        flag_ids.iter().filter_map(|id| self.flags.get(id)).collect()
    }

    /// Search flags near a world coordinate
    pub fn flags_near_world_pos(&self, world_x: f64, world_z: f64, radius: f64) -> Vec<&CatalogFlag> {
        self.flags
            .values()
            .filter(|f| {
                if let (Some(fx), Some(fz)) = (f.world_x, f.world_z) {
                    let dx = fx - world_x;
                    let dz = fz - world_z;
                    (dx * dx + dz * dz).sqrt() <= radius
                } else {
                    false
                }
            })
            .collect()
    }
}

/// Errors that can occur when loading or using the catalog
#[derive(Debug)]
pub enum CatalogError {
    IoError(String),
    ParseError(String),
    NotFound(String),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::IoError(msg) => write!(f, "IO error: {}", msg),
            CatalogError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            CatalogError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for CatalogError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_catalog() {
        let catalog = FlagCatalog::load_default();
        match catalog {
            Ok(cat) => {
                println!("Loaded {} flags", cat.len());
                println!("Categories: {:?}", cat.categories().len());
                println!("Regions: {:?}", cat.regions().len());
                assert!(cat.len() > 7000, "Expected 7000+ flags");
            }
            Err(e) => {
                println!("Could not load catalog (expected in CI): {}", e);
            }
        }
    }

    #[test]
    fn test_search_by_name() {
        if let Ok(catalog) = FlagCatalog::load_default() {
            // Search for graces
            let results = catalog.search_by_name("first step");
            println!("Found {} results for 'first step'", results.len());
            for flag in results.iter().take(5) {
                println!("  {} - {} ({})", flag.flag_id, flag.name, flag.category);
            }

            // Search for cookbooks
            let results = catalog.search_by_name("cookbook");
            println!("Found {} results for 'cookbook'", results.len());
            assert!(!results.is_empty(), "Should find cookbooks");
        }
    }

    #[test]
    fn test_autocomplete() {
        if let Ok(catalog) = FlagCatalog::load_default() {
            let suggestions = catalog.autocomplete("margit", 5);
            println!("Autocomplete 'margit':");
            for flag in &suggestions {
                println!("  {} - {}", flag.flag_id, flag.name);
            }
        }
    }

    #[test]
    fn test_category_lookup() {
        if let Ok(catalog) = FlagCatalog::load_default() {
            let graces = catalog.flags_for_category("Grace");
            println!("Found {} grace flags", graces.len());
            assert!(graces.len() > 400, "Expected 400+ graces");

            let bosses = catalog.flags_for_category("Great Boss Defeat");
            println!("Found {} great boss defeat flags", bosses.len());
        }
    }

    #[test]
    fn test_region_lookup() {
        if let Ok(catalog) = FlagCatalog::load_default() {
            let limgrave = catalog.flags_for_region("Limgrave");
            println!("Found {} flags in Limgrave", limgrave.len());

            let caelid = catalog.flags_for_region("Caelid");
            println!("Found {} flags in Caelid", caelid.len());
        }
    }

    #[test]
    fn test_get_by_id() {
        if let Ok(catalog) = FlagCatalog::load_default() {
            // Test a known flag ID (First Step grace is typically 76100)
            if let Some(flag) = catalog.get_by_id(76100) {
                println!("Flag 76100: {} ({})", flag.name, flag.category);
            }
        }
    }
}
