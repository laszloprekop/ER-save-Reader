/// Discovery Store Module
///
/// Persistent storage for discovered flag offsets with full provenance tracking.
/// Supports the discovery pipeline: Pending → Confirmed → Promoted → (or Rejected)
///
/// Discoveries are stored in `discoveries.json` and track:
/// - The discovered byte offset and bit position
/// - Confidence score based on observation count and agreement
/// - Provenance: which snapshots, saves, or probes contributed to the discovery
/// - Status transitions with timestamps

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a discovery in the pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryStatus {
    /// Awaiting additional observations for consensus
    Pending,
    /// Confirmed by multiple observations with agreement
    Confirmed,
    /// Already promoted to ground_truth_offsets.json
    Promoted,
    /// Failed validation or conflicting evidence
    Rejected,
}

impl std::fmt::Display for DiscoveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryStatus::Pending => write!(f, "pending"),
            DiscoveryStatus::Confirmed => write!(f, "confirmed"),
            DiscoveryStatus::Promoted => write!(f, "promoted"),
            DiscoveryStatus::Rejected => write!(f, "rejected"),
        }
    }
}

/// Source of an observation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ObservationSource {
    /// From comparing before/after save snapshots
    SnapshotDiff {
        before_file: String,
        after_file: String,
        action_description: String,
    },
    /// From manual verification records (user truth)
    ManualVerification {
        record_id: String,
        character_name: String,
    },
    /// From automated offset probing
    ProbeResult {
        search_radius: usize,
        confidence: f64,
        probe_method: String,
    },
    /// From cross-slot validation
    CrossSlotValidation {
        slots_validated: Vec<usize>,
        all_matched: bool,
    },
}

/// A single observation of a flag's offset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetObservation {
    /// Observed byte offset
    pub byte_offset: usize,
    /// Observed bit position (0-7)
    pub bit_position: u8,
    /// Source of this observation
    pub source: ObservationSource,
    /// When this was observed
    pub timestamp: DateTime<Utc>,
    /// Which save slot this was observed in
    pub slot_index: Option<usize>,
    /// Character name if known
    pub character_name: Option<String>,
    /// Raw confidence from the observation method
    pub raw_confidence: f64,
}

impl OffsetObservation {
    /// Create a new observation with current timestamp
    pub fn new(
        byte_offset: usize,
        bit_position: u8,
        source: ObservationSource,
        slot_index: Option<usize>,
        character_name: Option<String>,
        raw_confidence: f64,
    ) -> Self {
        Self {
            byte_offset,
            bit_position,
            source,
            timestamp: Utc::now(),
            slot_index,
            character_name,
            raw_confidence,
        }
    }
}

/// A stored discovery with all observations and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDiscovery {
    /// The flag ID
    pub flag_id: u32,
    /// Flag name (for human readability)
    pub flag_name: Option<String>,
    /// Flag category
    pub flag_category: Option<String>,
    /// Confirmed byte offset (None if contested)
    pub byte_offset: Option<usize>,
    /// Confirmed bit position (None if contested)
    pub bit_position: Option<u8>,
    /// Aggregated confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Current status in the pipeline
    pub status: DiscoveryStatus,
    /// Number of observations
    pub observation_count: usize,
    /// First observation timestamp
    pub first_observed: DateTime<Utc>,
    /// Last observation timestamp
    pub last_updated: DateTime<Utc>,
    /// All observations for this flag
    pub observations: Vec<OffsetObservation>,
    /// Rejection reason if rejected
    pub rejection_reason: Option<String>,
    /// Notes for human review
    pub notes: Option<String>,
}

impl StoredDiscovery {
    /// Create a new discovery from a first observation
    pub fn new(flag_id: u32, observation: OffsetObservation) -> Self {
        let now = Utc::now();
        Self {
            flag_id,
            flag_name: None,
            flag_category: None,
            byte_offset: Some(observation.byte_offset),
            bit_position: Some(observation.bit_position),
            confidence: observation.raw_confidence,
            status: DiscoveryStatus::Pending,
            observation_count: 1,
            first_observed: now,
            last_updated: now,
            observations: vec![observation],
            rejection_reason: None,
            notes: None,
        }
    }

    /// Add a new observation and recalculate consensus
    pub fn add_observation(&mut self, observation: OffsetObservation) {
        self.observations.push(observation);
        self.observation_count = self.observations.len();
        self.last_updated = Utc::now();
        self.recalculate_consensus();
    }

    /// Recalculate consensus byte/bit and confidence from all observations
    fn recalculate_consensus(&mut self) {
        if self.observations.is_empty() {
            self.confidence = 0.0;
            self.byte_offset = None;
            self.bit_position = None;
            return;
        }

        // Count votes for each (byte, bit) combination
        let mut votes: HashMap<(usize, u8), (usize, f64)> = HashMap::new();
        for obs in &self.observations {
            let key = (obs.byte_offset, obs.bit_position);
            let entry = votes.entry(key).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += obs.raw_confidence;
        }

        // Find the most voted position
        let total_obs = self.observations.len();
        let mut best_pos = None;
        let mut best_score = 0.0;

        for ((byte, bit), (count, conf_sum)) in &votes {
            let agreement = *count as f64 / total_obs as f64;
            let avg_conf = conf_sum / *count as f64;
            let score = agreement * avg_conf;

            if score > best_score {
                best_score = score;
                best_pos = Some((*byte, *bit));
            }
        }

        if let Some((byte, bit)) = best_pos {
            self.byte_offset = Some(byte);
            self.bit_position = Some(bit);

            // Calculate final confidence
            // Higher with more observations and higher agreement
            let agreement = votes.get(&(byte, bit)).map(|(c, _)| *c as f64 / total_obs as f64).unwrap_or(0.0);
            let obs_bonus = (total_obs as f64).ln() / 5.0; // Bonus for more observations, capped
            self.confidence = (best_score * (1.0 + obs_bonus.min(0.3))).min(1.0);

            // Check if we have enough agreement to be Confirmed
            if self.status == DiscoveryStatus::Pending {
                if total_obs >= 2 && agreement >= 0.8 && self.confidence >= 0.7 {
                    self.status = DiscoveryStatus::Confirmed;
                } else if votes.len() > 1 && agreement < 0.5 {
                    // Contested - multiple different positions with no clear winner
                    self.status = DiscoveryStatus::Pending;
                    self.notes = Some("Contested: multiple different offsets observed".to_string());
                }
            }
        }
    }

    /// Get the offset as a tuple if confirmed
    pub fn get_offset(&self) -> Option<(usize, u8)> {
        match (self.byte_offset, self.bit_position) {
            (Some(b), Some(p)) => Some((b, p)),
            _ => None,
        }
    }

    /// Check if this discovery is ready for promotion
    pub fn is_promotable(&self) -> bool {
        self.status == DiscoveryStatus::Confirmed
            && self.byte_offset.is_some()
            && self.bit_position.is_some()
            && self.confidence >= 0.75
    }

    /// Mark as promoted
    pub fn mark_promoted(&mut self) {
        self.status = DiscoveryStatus::Promoted;
        self.last_updated = Utc::now();
    }

    /// Mark as rejected with reason
    pub fn reject(&mut self, reason: &str) {
        self.status = DiscoveryStatus::Rejected;
        self.rejection_reason = Some(reason.to_string());
        self.last_updated = Utc::now();
    }
}

/// The discovery store - persists to discoveries.json
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryStore {
    /// Version for schema migrations
    pub version: u32,
    /// Metadata about the store
    pub metadata: StoreMetadata,
    /// All discoveries indexed by flag_id
    pub discoveries: HashMap<u32, StoredDiscovery>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreMetadata {
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub total_discoveries: usize,
    pub pending_count: usize,
    pub confirmed_count: usize,
    pub promoted_count: usize,
    pub rejected_count: usize,
}

impl Default for StoreMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            last_modified: now,
            total_discoveries: 0,
            pending_count: 0,
            confirmed_count: 0,
            promoted_count: 0,
            rejected_count: 0,
        }
    }
}

impl DiscoveryStore {
    /// Create a new empty store
    pub fn new() -> Self {
        Self {
            version: 1,
            metadata: StoreMetadata::default(),
            discoveries: HashMap::new(),
        }
    }

    /// Load store from JSON file
    pub fn load(path: &Path) -> Result<Self, StoreError> {
        let file = File::open(path)
            .map_err(|e| StoreError::IoError(format!("Failed to open store: {}", e)))?;

        let reader = BufReader::new(file);
        let store: Self = serde_json::from_reader(reader)
            .map_err(|e| StoreError::ParseError(format!("Failed to parse store: {}", e)))?;

        Ok(store)
    }

    /// Load store from path, or create new if doesn't exist
    pub fn load_or_create(path: &Path) -> Result<Self, StoreError> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::new())
        }
    }

    /// Load from default location (discoveries.json in project root)
    pub fn load_default() -> Result<Self, StoreError> {
        let default_path = PathBuf::from("discoveries.json");
        Self::load_or_create(&default_path)
    }

    /// Save store to JSON file
    pub fn save(&self, path: &Path) -> Result<(), StoreError> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| StoreError::IoError(format!("Failed to create directory: {}", e)))?;
        }

        let file = File::create(path)
            .map_err(|e| StoreError::IoError(format!("Failed to create file: {}", e)))?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)
            .map_err(|e| StoreError::SerializeError(format!("Failed to serialize store: {}", e)))?;

        Ok(())
    }

    /// Save to default location
    pub fn save_default(&self) -> Result<(), StoreError> {
        self.save(&PathBuf::from("discoveries.json"))
    }

    /// Add an observation for a flag
    pub fn add_observation(&mut self, flag_id: u32, observation: OffsetObservation) {
        if let Some(discovery) = self.discoveries.get_mut(&flag_id) {
            discovery.add_observation(observation);
        } else {
            self.discoveries.insert(flag_id, StoredDiscovery::new(flag_id, observation));
        }
        self.update_metadata();
    }

    /// Add observation with flag metadata
    pub fn add_observation_with_metadata(
        &mut self,
        flag_id: u32,
        flag_name: Option<String>,
        flag_category: Option<String>,
        observation: OffsetObservation,
    ) {
        if let Some(discovery) = self.discoveries.get_mut(&flag_id) {
            discovery.add_observation(observation);
            if flag_name.is_some() {
                discovery.flag_name = flag_name;
            }
            if flag_category.is_some() {
                discovery.flag_category = flag_category;
            }
        } else {
            let mut discovery = StoredDiscovery::new(flag_id, observation);
            discovery.flag_name = flag_name;
            discovery.flag_category = flag_category;
            self.discoveries.insert(flag_id, discovery);
        }
        self.update_metadata();
    }

    /// Get a discovery by flag ID
    pub fn get(&self, flag_id: u32) -> Option<&StoredDiscovery> {
        self.discoveries.get(&flag_id)
    }

    /// Get a mutable discovery by flag ID
    pub fn get_mut(&mut self, flag_id: u32) -> Option<&mut StoredDiscovery> {
        self.discoveries.get_mut(&flag_id)
    }

    /// Get all discoveries ready for promotion
    pub fn get_promotable(&self) -> Vec<&StoredDiscovery> {
        self.discoveries
            .values()
            .filter(|d| d.is_promotable())
            .collect()
    }

    /// Get all discoveries with a specific status
    pub fn get_by_status(&self, status: DiscoveryStatus) -> Vec<&StoredDiscovery> {
        self.discoveries
            .values()
            .filter(|d| d.status == status)
            .collect()
    }

    /// Update metadata counts
    fn update_metadata(&mut self) {
        self.metadata.last_modified = Utc::now();
        self.metadata.total_discoveries = self.discoveries.len();
        self.metadata.pending_count = self.discoveries.values().filter(|d| d.status == DiscoveryStatus::Pending).count();
        self.metadata.confirmed_count = self.discoveries.values().filter(|d| d.status == DiscoveryStatus::Confirmed).count();
        self.metadata.promoted_count = self.discoveries.values().filter(|d| d.status == DiscoveryStatus::Promoted).count();
        self.metadata.rejected_count = self.discoveries.values().filter(|d| d.status == DiscoveryStatus::Rejected).count();
    }

    /// Get summary statistics
    pub fn summary(&self) -> StoreSummary {
        let mut by_category: HashMap<String, usize> = HashMap::new();
        for discovery in self.discoveries.values() {
            if let Some(ref cat) = discovery.flag_category {
                *by_category.entry(cat.clone()).or_insert(0) += 1;
            } else {
                *by_category.entry("Unknown".to_string()).or_insert(0) += 1;
            }
        }

        StoreSummary {
            total: self.discoveries.len(),
            pending: self.metadata.pending_count,
            confirmed: self.metadata.confirmed_count,
            promoted: self.metadata.promoted_count,
            rejected: self.metadata.rejected_count,
            by_category,
        }
    }

    /// Iterate over all discoveries
    pub fn iter(&self) -> impl Iterator<Item = &StoredDiscovery> {
        self.discoveries.values()
    }

    /// Number of discoveries
    pub fn len(&self) -> usize {
        self.discoveries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.discoveries.is_empty()
    }
}

impl Default for DiscoveryStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary statistics for the store
#[derive(Debug, Clone)]
pub struct StoreSummary {
    pub total: usize,
    pub pending: usize,
    pub confirmed: usize,
    pub promoted: usize,
    pub rejected: usize,
    pub by_category: HashMap<String, usize>,
}

impl std::fmt::Display for StoreSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Discovery Store Summary:")?;
        writeln!(f, "  Total: {}", self.total)?;
        writeln!(f, "  Pending: {}", self.pending)?;
        writeln!(f, "  Confirmed: {}", self.confirmed)?;
        writeln!(f, "  Promoted: {}", self.promoted)?;
        writeln!(f, "  Rejected: {}", self.rejected)?;
        writeln!(f, "  By category:")?;
        for (cat, count) in &self.by_category {
            writeln!(f, "    {}: {}", cat, count)?;
        }
        Ok(())
    }
}

/// Errors from store operations
#[derive(Debug)]
pub enum StoreError {
    IoError(String),
    ParseError(String),
    SerializeError(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::IoError(msg) => write!(f, "IO error: {}", msg),
            StoreError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            StoreError::SerializeError(msg) => write!(f, "Serialize error: {}", msg),
        }
    }
}

impl std::error::Error for StoreError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_store() {
        let store = DiscoveryStore::new();
        assert!(store.is_empty());
        assert_eq!(store.metadata.total_discoveries, 0);
    }

    #[test]
    fn test_add_observation() {
        let mut store = DiscoveryStore::new();

        let obs = OffsetObservation::new(
            1234,
            5,
            ObservationSource::ProbeResult {
                search_radius: 500,
                confidence: 0.85,
                probe_method: "regional_search".to_string(),
            },
            Some(0),
            Some("Confessor".to_string()),
            0.85,
        );

        store.add_observation(76100, obs);

        assert_eq!(store.len(), 1);
        let discovery = store.get(76100).unwrap();
        assert_eq!(discovery.flag_id, 76100);
        assert_eq!(discovery.byte_offset, Some(1234));
        assert_eq!(discovery.bit_position, Some(5));
        assert_eq!(discovery.status, DiscoveryStatus::Pending);
    }

    #[test]
    fn test_consensus_with_multiple_observations() {
        let mut store = DiscoveryStore::new();

        // Add first observation
        let obs1 = OffsetObservation::new(
            1234, 5,
            ObservationSource::SnapshotDiff {
                before_file: "before.sl2".to_string(),
                after_file: "after.sl2".to_string(),
                action_description: "Grace touched".to_string(),
            },
            Some(0), Some("Confessor".to_string()), 0.9,
        );
        store.add_observation(76100, obs1);

        // Add second observation with same position
        let obs2 = OffsetObservation::new(
            1234, 5,
            ObservationSource::CrossSlotValidation {
                slots_validated: vec![0, 1, 2],
                all_matched: true,
            },
            None, None, 0.95,
        );
        store.add_observation(76100, obs2);

        let discovery = store.get(76100).unwrap();
        assert_eq!(discovery.observation_count, 2);
        assert_eq!(discovery.status, DiscoveryStatus::Confirmed);
        assert!(discovery.confidence >= 0.8);
    }

    #[test]
    fn test_contested_discovery() {
        let mut store = DiscoveryStore::new();

        // Add observation for position A
        let obs1 = OffsetObservation::new(
            1234, 5,
            ObservationSource::ProbeResult {
                search_radius: 500,
                confidence: 0.7,
                probe_method: "test".to_string(),
            },
            Some(0), None, 0.7,
        );
        store.add_observation(76100, obs1);

        // Add observation for different position B
        let obs2 = OffsetObservation::new(
            5678, 3,
            ObservationSource::ProbeResult {
                search_radius: 500,
                confidence: 0.7,
                probe_method: "test".to_string(),
            },
            Some(1), None, 0.7,
        );
        store.add_observation(76100, obs2);

        let discovery = store.get(76100).unwrap();
        assert_eq!(discovery.observation_count, 2);
        // Should remain pending due to disagreement
        assert_eq!(discovery.status, DiscoveryStatus::Pending);
    }

    #[test]
    fn test_save_and_load() {
        let temp_path = std::env::temp_dir().join("test_discoveries.json");

        let mut store = DiscoveryStore::new();
        let obs = OffsetObservation::new(
            1234, 5,
            ObservationSource::ManualVerification {
                record_id: "test-123".to_string(),
                character_name: "Test".to_string(),
            },
            Some(0), Some("Test".to_string()), 0.9,
        );
        store.add_observation_with_metadata(76100, Some("Test Grace".to_string()), Some("Grace".to_string()), obs);

        // Save
        store.save(&temp_path).expect("Failed to save");

        // Load
        let loaded = DiscoveryStore::load(&temp_path).expect("Failed to load");
        assert_eq!(loaded.len(), 1);

        let discovery = loaded.get(76100).unwrap();
        assert_eq!(discovery.flag_name, Some("Test Grace".to_string()));
        assert_eq!(discovery.flag_category, Some("Grace".to_string()));

        // Cleanup
        fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_promotable_discovery() {
        let mut store = DiscoveryStore::new();

        // Need multiple agreeing observations for promotion
        for i in 0..3 {
            let obs = OffsetObservation::new(
                1234, 5,
                ObservationSource::SnapshotDiff {
                    before_file: format!("before_{}.sl2", i),
                    after_file: format!("after_{}.sl2", i),
                    action_description: "Test".to_string(),
                },
                Some(i), None, 0.9,
            );
            store.add_observation(76100, obs);
        }

        let discovery = store.get(76100).unwrap();
        assert_eq!(discovery.status, DiscoveryStatus::Confirmed);
        assert!(discovery.is_promotable());

        let promotable = store.get_promotable();
        assert_eq!(promotable.len(), 1);
    }
}
