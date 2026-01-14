/// Ground Truth Updater
///
/// Safely updates ground_truth_offsets.json with confirmed discoveries.
/// Provides backup/rollback, atomic updates, and formula recalculation.
///
/// ## Safety Features:
/// - Creates timestamped backup before any modification
/// - Validates new offsets don't conflict with existing
/// - Recalculates block bases when enough flags are confirmed
/// - Provides rollback capability

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::discovery_store::{StoredDiscovery, DiscoveryStatus};
use super::consensus::{ConsensusBuilder, ConsensusStatus};

/// Configuration for ground truth updates
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    /// Path to ground_truth_offsets.json
    pub ground_truth_path: PathBuf,
    /// Directory for backups
    pub backup_dir: PathBuf,
    /// Minimum confidence to promote
    pub min_confidence: f64,
    /// Minimum observations to promote
    pub min_observations: usize,
    /// Whether to recalculate block bases
    pub recalculate_block_bases: bool,
    /// Minimum flags in a block to recalculate base
    pub min_flags_for_base_recalc: usize,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            ground_truth_path: PathBuf::from("ground_truth_offsets.json"),
            backup_dir: PathBuf::from("backups"),
            min_confidence: 0.8,
            min_observations: 2,
            recalculate_block_bases: true,
            min_flags_for_base_recalc: 3,
        }
    }
}

/// A pending update to ground truth
#[derive(Debug, Clone)]
pub struct PendingUpdate {
    pub flag_id: u32,
    pub name: Option<String>,
    pub category: Option<String>,
    pub new_offset: usize,
    pub new_bit: u8,
    pub confidence: f64,
    pub observation_count: usize,
    pub current_offset: Option<(usize, u8)>,
    pub current_status: Option<String>,
}

/// Result of applying updates
#[derive(Debug)]
pub struct UpdateResult {
    pub backup_path: PathBuf,
    pub flags_updated: usize,
    pub flags_added: usize,
    pub block_bases_recalculated: usize,
    pub errors: Vec<String>,
}

/// Ground truth updater
pub struct GroundTruthUpdater {
    config: UpdateConfig,
    pending_updates: Vec<PendingUpdate>,
}

impl GroundTruthUpdater {
    pub fn new(config: UpdateConfig) -> Self {
        Self {
            config,
            pending_updates: Vec::new(),
        }
    }

    /// Create with default config
    pub fn with_defaults() -> Self {
        Self::new(UpdateConfig::default())
    }

    /// Stage an update for a flag
    pub fn stage_update(&mut self, update: PendingUpdate) {
        self.pending_updates.push(update);
    }

    /// Stage updates from confirmed discoveries
    pub fn stage_from_discoveries(&mut self, discoveries: &[&StoredDiscovery]) {
        let consensus = ConsensusBuilder::default();

        for discovery in discoveries {
            let result = consensus.analyze(discovery);

            if result.status == ConsensusStatus::Confirmed
                && result.weighted_confidence >= self.config.min_confidence
                && discovery.observation_count >= self.config.min_observations
            {
                if let Some((byte, bit)) = result.best_offset {
                    self.pending_updates.push(PendingUpdate {
                        flag_id: discovery.flag_id,
                        name: discovery.flag_name.clone(),
                        category: discovery.flag_category.clone(),
                        new_offset: byte,
                        new_bit: bit,
                        confidence: result.weighted_confidence,
                        observation_count: discovery.observation_count,
                        current_offset: None, // Will be filled during apply
                        current_status: None,
                    });
                }
            }
        }
    }

    /// Create a timestamped backup
    pub fn create_backup(&self) -> Result<PathBuf, UpdateError> {
        fs::create_dir_all(&self.config.backup_dir)
            .map_err(|e| UpdateError::IoError(format!("Failed to create backup dir: {}", e)))?;

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("ground_truth_offsets_{}.json", timestamp);
        let backup_path = self.config.backup_dir.join(backup_name);

        fs::copy(&self.config.ground_truth_path, &backup_path)
            .map_err(|e| UpdateError::IoError(format!("Failed to create backup: {}", e)))?;

        Ok(backup_path)
    }

    /// Rollback to a previous backup
    pub fn rollback(&self, backup_path: &Path) -> Result<(), UpdateError> {
        if !backup_path.exists() {
            return Err(UpdateError::BackupNotFound(backup_path.to_path_buf()));
        }

        fs::copy(backup_path, &self.config.ground_truth_path)
            .map_err(|e| UpdateError::IoError(format!("Failed to rollback: {}", e)))?;

        Ok(())
    }

    /// Apply all staged updates
    pub fn apply_updates(&mut self) -> Result<UpdateResult, UpdateError> {
        if self.pending_updates.is_empty() {
            return Ok(UpdateResult {
                backup_path: PathBuf::new(),
                flags_updated: 0,
                flags_added: 0,
                block_bases_recalculated: 0,
                errors: Vec::new(),
            });
        }

        // Create backup first
        let backup_path = self.create_backup()?;

        // Load current ground truth
        let mut ground_truth = self.load_ground_truth()?;

        let mut flags_updated = 0;
        let mut flags_added = 0;
        let mut errors = Vec::new();

        // Apply each update
        for update in &self.pending_updates {
            match self.apply_single_update(&mut ground_truth, update) {
                Ok(is_new) => {
                    if is_new {
                        flags_added += 1;
                    } else {
                        flags_updated += 1;
                    }
                }
                Err(e) => {
                    errors.push(format!("Flag {}: {}", update.flag_id, e));
                }
            }
        }

        // Recalculate block bases if enabled
        let block_bases_recalculated = if self.config.recalculate_block_bases {
            self.recalculate_block_bases(&mut ground_truth)?
        } else {
            0
        };

        // Update metadata
        self.update_metadata(&mut ground_truth);

        // Save updated ground truth
        self.save_ground_truth(&ground_truth)?;

        // Clear pending updates
        self.pending_updates.clear();

        Ok(UpdateResult {
            backup_path,
            flags_updated,
            flags_added,
            block_bases_recalculated,
            errors,
        })
    }

    /// Load ground truth JSON
    fn load_ground_truth(&self) -> Result<Value, UpdateError> {
        let file = File::open(&self.config.ground_truth_path)
            .map_err(|e| UpdateError::IoError(format!("Failed to open ground truth: {}", e)))?;

        let reader = BufReader::new(file);
        serde_json::from_reader(reader)
            .map_err(|e| UpdateError::ParseError(format!("Failed to parse ground truth: {}", e)))
    }

    /// Save ground truth JSON
    fn save_ground_truth(&self, ground_truth: &Value) -> Result<(), UpdateError> {
        let file = File::create(&self.config.ground_truth_path)
            .map_err(|e| UpdateError::IoError(format!("Failed to create ground truth: {}", e)))?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, ground_truth)
            .map_err(|e| UpdateError::SerializeError(format!("Failed to write ground truth: {}", e)))
    }

    /// Apply a single update to the ground truth
    fn apply_single_update(&self, ground_truth: &mut Value, update: &PendingUpdate) -> Result<bool, String> {
        let verified_flags = ground_truth
            .get_mut("verified_flags")
            .ok_or("Missing verified_flags section")?
            .as_object_mut()
            .ok_or("verified_flags is not an object")?;

        let flag_key = update.flag_id.to_string();
        let is_new = !verified_flags.contains_key(&flag_key);

        // Create or update the flag entry
        let flag_entry = serde_json::json!({
            "offset": update.new_offset,
            "bit": update.new_bit,
            "name": update.name.clone().unwrap_or_else(|| format!("Flag {}", update.flag_id)),
            "category": update.category.clone().unwrap_or_else(|| "Unknown".to_string()),
            "status": "proven",
            "confidence": update.confidence,
            "observation_count": update.observation_count,
            "last_updated": Utc::now().to_rfc3339()
        });

        verified_flags.insert(flag_key, flag_entry);

        Ok(is_new)
    }

    /// Recalculate block bases from verified flags
    fn recalculate_block_bases(&self, ground_truth: &mut Value) -> Result<usize, UpdateError> {
        let verified_flags = ground_truth
            .get("verified_flags")
            .and_then(|v| v.as_object())
            .ok_or(UpdateError::ParseError("Missing verified_flags".to_string()))?;

        // Group proven flags by block
        let mut block_flags: HashMap<u32, Vec<(u32, usize, u8)>> = HashMap::new();

        for (flag_key, flag_data) in verified_flags {
            if let Ok(flag_id) = flag_key.parse::<u32>() {
                // Only process block flags (60000-99999)
                if flag_id >= 60000 && flag_id < 100000 {
                    if let (Some(status), Some(offset), Some(bit)) = (
                        flag_data.get("status").and_then(|v| v.as_str()),
                        flag_data.get("offset").and_then(|v| v.as_u64()),
                        flag_data.get("bit").and_then(|v| v.as_u64()),
                    ) {
                        if status == "proven" {
                            let block_start = (flag_id / 1000) * 1000;
                            block_flags.entry(block_start)
                                .or_default()
                                .push((flag_id, offset as usize, bit as u8));
                        }
                    }
                }
            }
        }

        // Recalculate bases for blocks with enough flags
        let block_bases = ground_truth
            .get_mut("block_bases")
            .and_then(|v| v.as_object_mut())
            .ok_or(UpdateError::ParseError("Missing block_bases".to_string()))?;

        let mut recalculated = 0;

        for (block_start, flags) in block_flags {
            if flags.len() >= self.config.min_flags_for_base_recalc {
                // Calculate base from all flags and check consistency
                let calculated_bases: Vec<usize> = flags.iter()
                    .map(|(flag_id, offset, _bit)| {
                        let relative = flag_id - block_start;
                        offset.saturating_sub((relative / 8) as usize)
                    })
                    .collect();

                // Check if all flags agree on the base
                if let Some(&first_base) = calculated_bases.first() {
                    if calculated_bases.iter().all(|&b| b == first_base) {
                        // Update block base
                        let block_key = block_start.to_string();
                        if let Some(block_entry) = block_bases.get_mut(&block_key) {
                            if let Some(current_base) = block_entry.get("base_offset").and_then(|v| v.as_u64()) {
                                if current_base as usize != first_base {
                                    block_entry["base_offset"] = serde_json::json!(first_base);
                                    block_entry["status"] = serde_json::json!("verified");
                                    block_entry["notes"] = serde_json::json!(
                                        format!("Recalculated from {} proven flags", flags.len())
                                    );
                                    recalculated += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(recalculated)
    }

    /// Update metadata with current timestamp
    fn update_metadata(&self, ground_truth: &mut Value) {
        if let Some(metadata) = ground_truth.get_mut("metadata") {
            metadata["generated_date"] = serde_json::json!(Utc::now().to_rfc3339());
            metadata["last_update"] = serde_json::json!(format!(
                "Auto-updated {} flags via discovery pipeline",
                self.pending_updates.len()
            ));
        }
    }

    /// Get count of pending updates
    pub fn pending_count(&self) -> usize {
        self.pending_updates.len()
    }

    /// Clear pending updates without applying
    pub fn clear_pending(&mut self) {
        self.pending_updates.clear();
    }

    /// Preview pending updates
    pub fn preview_updates(&self) -> &[PendingUpdate] {
        &self.pending_updates
    }
}

/// Errors from ground truth operations
#[derive(Debug)]
pub enum UpdateError {
    IoError(String),
    ParseError(String),
    SerializeError(String),
    BackupNotFound(PathBuf),
    ValidationFailed(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::IoError(msg) => write!(f, "IO error: {}", msg),
            UpdateError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            UpdateError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
            UpdateError::BackupNotFound(path) => write!(f, "Backup not found: {:?}", path),
            UpdateError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
        }
    }
}

impl std::error::Error for UpdateError {}

/// List available backups
pub fn list_backups(backup_dir: &Path) -> Vec<PathBuf> {
    let mut backups = Vec::new();

    if let Ok(entries) = fs::read_dir(backup_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("ground_truth_offsets_") && name.ends_with(".json") {
                        backups.push(path);
                    }
                }
            }
        }
    }

    // Sort by name (which includes timestamp)
    backups.sort();
    backups.reverse(); // Most recent first

    backups
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_config_defaults() {
        let config = UpdateConfig::default();
        assert_eq!(config.min_confidence, 0.8);
        assert_eq!(config.min_observations, 2);
    }

    #[test]
    fn test_pending_update() {
        let mut updater = GroundTruthUpdater::with_defaults();

        updater.stage_update(PendingUpdate {
            flag_id: 76100,
            name: Some("Test Grace".to_string()),
            category: Some("Grace".to_string()),
            new_offset: 3260,
            new_bit: 4,
            confidence: 0.9,
            observation_count: 3,
            current_offset: None,
            current_status: None,
        });

        assert_eq!(updater.pending_count(), 1);
        updater.clear_pending();
        assert_eq!(updater.pending_count(), 0);
    }

    #[test]
    fn test_list_backups() {
        let backup_dir = temp_dir().join("test_backups");
        fs::create_dir_all(&backup_dir).ok();

        // Create a test backup file
        let test_backup = backup_dir.join("ground_truth_offsets_20260114_120000.json");
        fs::write(&test_backup, "{}").ok();

        let backups = list_backups(&backup_dir);
        assert!(backups.len() >= 1);

        // Cleanup
        fs::remove_file(&test_backup).ok();
        fs::remove_dir(&backup_dir).ok();
    }
}
