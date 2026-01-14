/// Snapshot Batch Analyzer
///
/// Processes all granular before/after save snapshots to discover flag mappings.
/// Parses filenames to extract semantic actions and groups files into pairs
/// for differential analysis.
///
/// Filename patterns:
/// - "ER0000.sl2 Confessor - 01 before Missionary Cookbook [4] pickup"
/// - "ER0000.sl2 Confessor - 02 after Missionary Cookbook [4] picked up"
/// - "ER0000.sl2 Wretch - 05 Cave of knowledge, rested at Site of grace"

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::discovery_store::DiscoveryStore;
use super::flag_catalog::FlagCatalog;
use super::integration::run_differential_discovery_with_persistence;

/// Metadata extracted from a snapshot filename
#[derive(Debug, Clone)]
pub struct SnapshotMetadata {
    /// Full path to the file
    pub path: PathBuf,
    /// Character name (e.g., "Confessor", "Wretch")
    pub character: String,
    /// Slot index derived from character name
    pub slot_index: usize,
    /// Sequence number for ordering
    pub sequence: u32,
    /// Whether this is a "before" snapshot
    pub is_before: bool,
    /// Whether this is an "after" snapshot
    pub is_after: bool,
    /// Action description extracted from filename
    pub action: String,
    /// Flag ID if mentioned in filename (e.g., from "treasure_m60_43_50_00_1043500010")
    pub flag_id: Option<u32>,
}

/// A matched before/after pair for differential analysis
#[derive(Debug, Clone)]
pub struct SnapshotPair {
    pub before: SnapshotMetadata,
    pub after: SnapshotMetadata,
    pub character: String,
    pub slot_index: usize,
    pub action_description: String,
}

/// Results from batch analysis
#[derive(Debug)]
pub struct BatchAnalysisResult {
    pub files_scanned: usize,
    pub pairs_found: usize,
    pub pairs_processed: usize,
    pub discoveries_persisted: usize,
    pub errors: Vec<String>,
}

/// Parse a snapshot filename to extract metadata
pub fn parse_snapshot_filename(path: &Path) -> Option<SnapshotMetadata> {
    let filename = path.file_name()?.to_string_lossy();

    // Must start with ER0000.sl2
    if !filename.starts_with("ER0000.sl2") {
        return None;
    }

    // Find character name and sequence
    // Pattern: "ER0000.sl2 {Character} - {NN} {rest}"
    let after_prefix = filename.strip_prefix("ER0000.sl2 ")?;

    // Find the " - " separator
    let dash_pos = after_prefix.find(" - ")?;
    let character = after_prefix[..dash_pos].trim().to_string();

    let after_dash = &after_prefix[dash_pos + 3..];

    // Extract sequence number (first digits)
    let (sequence_str, rest) = extract_leading_number(after_dash)?;
    let sequence = sequence_str.parse::<u32>().ok()?;

    let rest = rest.trim();

    // Determine if before/after
    let is_before = rest.to_lowercase().contains("before");
    let is_after = rest.to_lowercase().contains("after")
        || rest.to_lowercase().contains("picked up")
        || rest.to_lowercase().contains("touched")
        || rest.to_lowercase().contains("defeated")
        || rest.to_lowercase().contains("rested");

    // Extract action description
    let action = clean_action_description(rest);

    // Try to extract flag ID from filename
    let flag_id = extract_flag_id(&filename);

    // Map character to slot index
    let slot_index = character_to_slot(&character);

    Some(SnapshotMetadata {
        path: path.to_path_buf(),
        character,
        slot_index,
        sequence,
        is_before,
        is_after,
        action,
        flag_id,
    })
}

/// Extract leading number from string
fn extract_leading_number(s: &str) -> Option<(String, &str)> {
    let num_end = s.chars().take_while(|c| c.is_ascii_digit()).count();
    if num_end == 0 {
        return None;
    }
    Some((s[..num_end].to_string(), &s[num_end..]))
}

/// Clean up action description
fn clean_action_description(s: &str) -> String {
    let s = s.trim();

    // Remove common prefixes
    let s = s.strip_prefix("before ").unwrap_or(s);
    let s = s.strip_prefix("after ").unwrap_or(s);

    // Capitalize first letter
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Extract flag ID from filename if present (e.g., "1043500010" from "treasure_m60_43_50_00_1043500010")
fn extract_flag_id(filename: &str) -> Option<u32> {
    // Look for 10-digit numbers that could be flag IDs
    let mut current_num = String::new();

    for c in filename.chars() {
        if c.is_ascii_digit() {
            current_num.push(c);
            if current_num.len() >= 10 {
                if let Ok(id) = current_num.parse::<u32>() {
                    if id >= 1_000_000_000 {
                        return Some(id);
                    }
                }
            }
        } else {
            current_num.clear();
        }
    }

    // Also look for 8-digit dungeon flags
    current_num.clear();
    for c in filename.chars() {
        if c.is_ascii_digit() {
            current_num.push(c);
            if current_num.len() == 8 {
                if let Ok(id) = current_num.parse::<u32>() {
                    if id >= 10_000_000 && id < 100_000_000 {
                        return Some(id);
                    }
                }
            }
        } else {
            if current_num.len() > 8 {
                current_num.clear();
            }
        }
    }

    None
}

/// Map character name to slot index
fn character_to_slot(character: &str) -> usize {
    match character.to_lowercase().as_str() {
        "confessor" => 0,
        "wretch" => 1,
        "v1" => 2,
        "v2" => 3,
        "v3" => 4,
        "sam" => 5,
        _ => 0, // Default to slot 0
    }
}

/// Scan a directory for snapshot files and parse their metadata
pub fn scan_snapshot_directory(dir: &Path) -> Vec<SnapshotMetadata> {
    let mut snapshots = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(meta) = parse_snapshot_filename(&path) {
                    snapshots.push(meta);
                }
            }
        }
    }

    // Sort by character then sequence
    snapshots.sort_by(|a, b| {
        a.character.cmp(&b.character)
            .then(a.sequence.cmp(&b.sequence))
    });

    snapshots
}

/// Group snapshots into before/after pairs
pub fn group_into_pairs(snapshots: &[SnapshotMetadata]) -> Vec<SnapshotPair> {
    let mut pairs = Vec::new();

    // Group by character
    let mut by_character: HashMap<String, Vec<&SnapshotMetadata>> = HashMap::new();
    for meta in snapshots {
        by_character.entry(meta.character.clone())
            .or_default()
            .push(meta);
    }

    for (character, mut group) in by_character {
        // Sort by sequence
        group.sort_by_key(|m| m.sequence);

        // Find pairs: look for consecutive before/after
        let mut i = 0;
        while i < group.len() {
            let current = group[i];

            // If this is a "before", look for the next "after"
            if current.is_before && i + 1 < group.len() {
                let next = group[i + 1];
                if next.is_after && next.sequence == current.sequence + 1 {
                    let action = if !next.action.is_empty() {
                        next.action.clone()
                    } else {
                        current.action.clone()
                    };

                    pairs.push(SnapshotPair {
                        before: current.clone(),
                        after: next.clone(),
                        character: character.clone(),
                        slot_index: current.slot_index,
                        action_description: action,
                    });

                    i += 2;
                    continue;
                }
            }

            // If no pair found, try pairing with previous if it makes sense
            // (e.g., consecutive "after" snapshots where first acts as "before")
            if current.is_after && i > 0 {
                let prev = group[i - 1];
                // Check if already paired
                let already_paired = pairs.iter().any(|p|
                    p.before.path == prev.path || p.after.path == prev.path
                );

                if !already_paired && current.sequence == prev.sequence + 1 {
                    pairs.push(SnapshotPair {
                        before: prev.clone(),
                        after: current.clone(),
                        character: character.clone(),
                        slot_index: current.slot_index,
                        action_description: current.action.clone(),
                    });
                }
            }

            i += 1;
        }
    }

    // Sort pairs by character then sequence
    pairs.sort_by(|a, b| {
        a.character.cmp(&b.character)
            .then(a.before.sequence.cmp(&b.before.sequence))
    });

    pairs
}

/// Run batch analysis on all snapshot pairs
pub fn run_batch_analysis(
    snapshot_dir: &Path,
    store: &mut DiscoveryStore,
) -> BatchAnalysisResult {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║              BATCH SNAPSHOT ANALYSIS                          ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    // Load flag catalog once for all pairs
    let catalog = match FlagCatalog::load_default() {
        Ok(c) => {
            println!("║ Loaded flag catalog ({} flags)", c.len());
            Some(c)
        }
        Err(e) => {
            println!("║ Warning: Could not load flag catalog: {}", e);
            println!("║ Flag names will not be populated");
            None
        }
    };

    let snapshots = scan_snapshot_directory(snapshot_dir);
    println!("║ Scanned {} snapshot files", snapshots.len());

    let pairs = group_into_pairs(&snapshots);
    println!("║ Found {} before/after pairs", pairs.len());

    let mut result = BatchAnalysisResult {
        files_scanned: snapshots.len(),
        pairs_found: pairs.len(),
        pairs_processed: 0,
        discoveries_persisted: 0,
        errors: Vec::new(),
    };

    let initial_discoveries = store.len();

    for (i, pair) in pairs.iter().enumerate() {
        println!("║");
        println!("║ [{}/{}] {} - {}", i + 1, pairs.len(), pair.character, pair.action_description);

        match run_differential_discovery_with_persistence(
            &pair.before.path,
            &pair.after.path,
            pair.slot_index,
            store,
            Some(&pair.action_description),
            catalog.as_ref(),
        ) {
            Ok(diff_result) => {
                result.pairs_processed += 1;
                println!("║   {} changes detected, {} bits set",
                    diff_result.total_changes, diff_result.bits_set);
            }
            Err(e) => {
                let error_msg = format!("{}: {}", pair.action_description, e);
                println!("║   ERROR: {}", e);
                result.errors.push(error_msg);
            }
        }
    }

    result.discoveries_persisted = store.len() - initial_discoveries;

    println!("║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ BATCH ANALYSIS COMPLETE                                       ║");
    println!("║   Pairs processed: {}/{}", result.pairs_processed, result.pairs_found);
    println!("║   New discoveries: {}", result.discoveries_persisted);
    println!("║   Total discoveries: {}", store.len());
    if !result.errors.is_empty() {
        println!("║   Errors: {}", result.errors.len());
    }
    println!("╚══════════════════════════════════════════════════════════════╝");

    result
}

/// Convenience function: Run batch analysis and save store
pub fn batch_analyze_and_save(
    snapshot_dir: &Path,
    store_path: &Path,
) -> Result<BatchAnalysisResult, String> {
    let mut store = DiscoveryStore::load_or_create(store_path)
        .map_err(|e| format!("Failed to load store: {}", e))?;

    let result = run_batch_analysis(snapshot_dir, &mut store);

    store.save(store_path)
        .map_err(|e| format!("Failed to save store: {}", e))?;

    println!("\nDiscovery store saved to {:?}", store_path);

    Ok(result)
}

/// List all snapshot pairs without processing them
pub fn list_snapshot_pairs(snapshot_dir: &Path) -> Vec<SnapshotPair> {
    let snapshots = scan_snapshot_directory(snapshot_dir);
    group_into_pairs(&snapshots)
}

/// Get summary of snapshot directory
pub fn get_snapshot_summary(snapshot_dir: &Path) -> SnapshotSummary {
    let snapshots = scan_snapshot_directory(snapshot_dir);
    let pairs = group_into_pairs(&snapshots);

    let mut by_character: HashMap<String, usize> = HashMap::new();
    for meta in &snapshots {
        *by_character.entry(meta.character.clone()).or_insert(0) += 1;
    }

    let mut flags_mentioned: Vec<u32> = snapshots.iter()
        .filter_map(|m| m.flag_id)
        .collect();
    flags_mentioned.sort();
    flags_mentioned.dedup();

    SnapshotSummary {
        total_files: snapshots.len(),
        pairs_found: pairs.len(),
        by_character,
        flags_mentioned,
    }
}

/// Summary of snapshot directory
#[derive(Debug)]
pub struct SnapshotSummary {
    pub total_files: usize,
    pub pairs_found: usize,
    pub by_character: HashMap<String, usize>,
    pub flags_mentioned: Vec<u32>,
}

impl std::fmt::Display for SnapshotSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Snapshot Summary:")?;
        writeln!(f, "  Total files: {}", self.total_files)?;
        writeln!(f, "  Pairs found: {}", self.pairs_found)?;
        writeln!(f, "  By character:")?;
        for (char, count) in &self.by_character {
            writeln!(f, "    {}: {} files", char, count)?;
        }
        if !self.flags_mentioned.is_empty() {
            writeln!(f, "  Flag IDs mentioned: {}", self.flags_mentioned.len())?;
            for flag in self.flags_mentioned.iter().take(10) {
                writeln!(f, "    {}", flag)?;
            }
            if self.flags_mentioned.len() > 10 {
                writeln!(f, "    ... and {} more", self.flags_mentioned.len() - 10)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_confessor_filename() {
        let path = Path::new("ER0000.sl2 Confessor - 01 before Missionary Cookbook [4] pickup");
        let meta = parse_snapshot_filename(path).unwrap();

        assert_eq!(meta.character, "Confessor");
        assert_eq!(meta.slot_index, 0);
        assert_eq!(meta.sequence, 1);
        assert!(meta.is_before);
        assert!(!meta.is_after);
    }

    #[test]
    fn test_parse_wretch_filename() {
        let path = Path::new("ER0000.sl2 Wretch - 05 Cave of knowledge, rested at Site of grace");
        let meta = parse_snapshot_filename(path).unwrap();

        assert_eq!(meta.character, "Wretch");
        assert_eq!(meta.slot_index, 1);
        assert_eq!(meta.sequence, 5);
        assert!(meta.is_after); // "rested" implies after
    }

    #[test]
    fn test_parse_with_flag_id() {
        let path = Path::new("ER0000.sl2 Confessor - 09 before picking up Smoldering Butterfly treasure_m60_43_50_00_1043500010");
        let meta = parse_snapshot_filename(path).unwrap();

        assert_eq!(meta.character, "Confessor");
        assert_eq!(meta.sequence, 9);
        assert_eq!(meta.flag_id, Some(1043500010));
    }

    #[test]
    fn test_extract_flag_id() {
        assert_eq!(extract_flag_id("treasure_m60_43_50_00_1043500010"), Some(1043500010));
        assert_eq!(extract_flag_id("no flag here"), None);
        assert_eq!(extract_flag_id("dungeon flag 10000800 test"), Some(10000800));
    }

    #[test]
    fn test_character_to_slot() {
        assert_eq!(character_to_slot("Confessor"), 0);
        assert_eq!(character_to_slot("Wretch"), 1);
        assert_eq!(character_to_slot("V1"), 2);
        assert_eq!(character_to_slot("CONFESSOR"), 0);
    }

    #[test]
    #[ignore] // Requires actual snapshot directory
    fn test_scan_directory() {
        let dir = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging");
        if dir.exists() {
            let summary = get_snapshot_summary(dir);
            println!("{}", summary);
            assert!(summary.total_files > 0);
        }
    }
}
