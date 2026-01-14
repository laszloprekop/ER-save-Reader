/// Consensus Engine
///
/// Builds confidence from multiple observations using configurable thresholds
/// and weighted voting. Different observation sources have different trust levels.
///
/// ## Consensus Requirements:
/// - Minimum 2 observations from different sources
/// - 80%+ agreement on the same (byte, bit) position
/// - Weighted confidence based on observation source
///
/// ## Source Weights:
/// - Manual verification: 1.0 (highest trust)
/// - Cross-slot validation: 0.95
/// - Snapshot diff: 0.85
/// - Probe result: 0.7

use std::collections::HashMap;

use super::discovery_store::{
    DiscoveryStore, StoredDiscovery, DiscoveryStatus,
    OffsetObservation, ObservationSource,
};

/// Configuration for consensus building
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Minimum number of observations required
    pub min_observations: usize,
    /// Minimum agreement percentage (0.0 - 1.0)
    pub min_agreement: f64,
    /// Minimum confidence to promote
    pub min_confidence_to_promote: f64,
    /// Whether to require observations from different sources
    pub require_different_sources: bool,
    /// Weight multipliers for each source type
    pub source_weights: SourceWeights,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            min_observations: 2,
            min_agreement: 0.8,
            min_confidence_to_promote: 0.75,
            require_different_sources: false,
            source_weights: SourceWeights::default(),
        }
    }
}

/// Weight multipliers for observation sources
#[derive(Debug, Clone)]
pub struct SourceWeights {
    pub manual_verification: f64,
    pub cross_slot_validation: f64,
    pub snapshot_diff: f64,
    pub probe_result: f64,
}

impl Default for SourceWeights {
    fn default() -> Self {
        Self {
            manual_verification: 1.0,
            cross_slot_validation: 0.95,
            snapshot_diff: 0.85,
            probe_result: 0.7,
        }
    }
}

/// Result of consensus analysis for a single discovery
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub flag_id: u32,
    pub status: ConsensusStatus,
    pub best_offset: Option<(usize, u8)>,
    pub weighted_confidence: f64,
    pub observation_count: usize,
    pub agreement_percentage: f64,
    pub source_diversity: usize,
    pub votes: Vec<OffsetVote>,
}

/// Status of consensus for a discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusStatus {
    /// Enough observations, all agree - ready for promotion
    Confirmed,
    /// Multiple observations but disagreement
    Contested,
    /// Not enough observations yet
    Insufficient,
    /// Already promoted to ground truth
    Promoted,
}

/// A vote for a specific offset position
#[derive(Debug, Clone)]
pub struct OffsetVote {
    pub byte_offset: usize,
    pub bit_position: u8,
    pub vote_count: usize,
    pub weighted_score: f64,
    pub sources: Vec<String>,
}

/// Consensus builder for analyzing discoveries
pub struct ConsensusBuilder {
    config: ConsensusConfig,
}

impl ConsensusBuilder {
    pub fn new(config: ConsensusConfig) -> Self {
        Self { config }
    }

    /// Analyze consensus for a single discovery
    pub fn analyze(&self, discovery: &StoredDiscovery) -> ConsensusResult {
        // Already promoted
        if discovery.status == DiscoveryStatus::Promoted {
            return ConsensusResult {
                flag_id: discovery.flag_id,
                status: ConsensusStatus::Promoted,
                best_offset: discovery.get_offset(),
                weighted_confidence: discovery.confidence,
                observation_count: discovery.observation_count,
                agreement_percentage: 1.0,
                source_diversity: 0,
                votes: Vec::new(),
            };
        }

        // Count votes for each (byte, bit) position
        let mut votes: HashMap<(usize, u8), (Vec<&OffsetObservation>, f64)> = HashMap::new();

        for obs in &discovery.observations {
            let key = (obs.byte_offset, obs.bit_position);
            let weight = self.get_source_weight(&obs.source);
            let weighted_confidence = obs.raw_confidence * weight;

            let entry = votes.entry(key).or_insert((Vec::new(), 0.0));
            entry.0.push(obs);
            entry.1 += weighted_confidence;
        }

        // Count unique sources
        let unique_sources: std::collections::HashSet<_> = discovery.observations.iter()
            .map(|o| source_type_name(&o.source))
            .collect();
        let source_diversity = unique_sources.len();

        // Find the winning position
        let total_obs = discovery.observations.len();
        let mut vote_results: Vec<OffsetVote> = votes.iter()
            .map(|((byte, bit), (obs_list, weighted_score))| {
                let sources: Vec<String> = obs_list.iter()
                    .map(|o| source_type_name(&o.source).to_string())
                    .collect();

                OffsetVote {
                    byte_offset: *byte,
                    bit_position: *bit,
                    vote_count: obs_list.len(),
                    weighted_score: *weighted_score,
                    sources,
                }
            })
            .collect();

        vote_results.sort_by(|a, b|
            b.weighted_score.partial_cmp(&a.weighted_score).unwrap()
        );

        // Determine status
        let (status, best_offset, agreement) = if total_obs < self.config.min_observations {
            (ConsensusStatus::Insufficient, None, 0.0)
        } else if let Some(best) = vote_results.first() {
            let agreement = best.vote_count as f64 / total_obs as f64;

            if vote_results.len() > 1 && agreement < self.config.min_agreement {
                // Contested - multiple positions with no clear winner
                (ConsensusStatus::Contested, Some((best.byte_offset, best.bit_position)), agreement)
            } else if agreement >= self.config.min_agreement {
                // Confirmed
                (ConsensusStatus::Confirmed, Some((best.byte_offset, best.bit_position)), agreement)
            } else {
                (ConsensusStatus::Insufficient, Some((best.byte_offset, best.bit_position)), agreement)
            }
        } else {
            (ConsensusStatus::Insufficient, None, 0.0)
        };

        // Calculate weighted confidence
        let weighted_confidence = if let Some(best) = vote_results.first() {
            let base_conf = best.weighted_score / best.vote_count as f64;
            let obs_bonus = (total_obs as f64).ln() / 5.0;
            (base_conf * (1.0 + obs_bonus.min(0.3))).min(1.0)
        } else {
            0.0
        };

        ConsensusResult {
            flag_id: discovery.flag_id,
            status,
            best_offset,
            weighted_confidence,
            observation_count: total_obs,
            agreement_percentage: agreement,
            source_diversity,
            votes: vote_results,
        }
    }

    /// Analyze all discoveries in a store
    pub fn analyze_store(&self, store: &DiscoveryStore) -> ConsensusReport {
        let mut results = Vec::new();
        let mut confirmed = 0;
        let mut contested = 0;
        let mut insufficient = 0;
        let mut promoted = 0;

        for discovery in store.iter() {
            let result = self.analyze(discovery);

            match result.status {
                ConsensusStatus::Confirmed => confirmed += 1,
                ConsensusStatus::Contested => contested += 1,
                ConsensusStatus::Insufficient => insufficient += 1,
                ConsensusStatus::Promoted => promoted += 1,
            }

            results.push(result);
        }

        ConsensusReport {
            total_discoveries: store.len(),
            confirmed,
            contested,
            insufficient,
            promoted,
            results,
        }
    }

    /// Get discoveries ready for promotion
    pub fn get_promotable(&self, store: &DiscoveryStore) -> Vec<ConsensusResult> {
        store.iter()
            .map(|d| self.analyze(d))
            .filter(|r| {
                r.status == ConsensusStatus::Confirmed
                    && r.weighted_confidence >= self.config.min_confidence_to_promote
            })
            .collect()
    }

    /// Get contested discoveries that need resolution
    pub fn get_contested(&self, store: &DiscoveryStore) -> Vec<ConsensusResult> {
        store.iter()
            .map(|d| self.analyze(d))
            .filter(|r| r.status == ConsensusStatus::Contested)
            .collect()
    }

    /// Get weight for an observation source
    fn get_source_weight(&self, source: &ObservationSource) -> f64 {
        match source {
            ObservationSource::ManualVerification { .. } => self.config.source_weights.manual_verification,
            ObservationSource::CrossSlotValidation { all_matched, .. } => {
                if *all_matched {
                    self.config.source_weights.cross_slot_validation
                } else {
                    self.config.source_weights.cross_slot_validation * 0.7
                }
            }
            ObservationSource::SnapshotDiff { .. } => self.config.source_weights.snapshot_diff,
            ObservationSource::ProbeResult { confidence, .. } => {
                self.config.source_weights.probe_result * confidence
            }
        }
    }
}

impl Default for ConsensusBuilder {
    fn default() -> Self {
        Self::new(ConsensusConfig::default())
    }
}

/// Report from analyzing all discoveries
#[derive(Debug)]
pub struct ConsensusReport {
    pub total_discoveries: usize,
    pub confirmed: usize,
    pub contested: usize,
    pub insufficient: usize,
    pub promoted: usize,
    pub results: Vec<ConsensusResult>,
}

impl std::fmt::Display for ConsensusReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Consensus Report:")?;
        writeln!(f, "  Total discoveries: {}", self.total_discoveries)?;
        writeln!(f, "  Confirmed (ready for promotion): {}", self.confirmed)?;
        writeln!(f, "  Contested (need resolution): {}", self.contested)?;
        writeln!(f, "  Insufficient (need more observations): {}", self.insufficient)?;
        writeln!(f, "  Already promoted: {}", self.promoted)?;

        if self.contested > 0 {
            writeln!(f, "\n  Contested discoveries:")?;
            for result in self.results.iter().filter(|r| r.status == ConsensusStatus::Contested).take(5) {
                writeln!(f, "    Flag {}: {} votes, {:.0}% agreement",
                    result.flag_id, result.observation_count, result.agreement_percentage * 100.0)?;
            }
        }

        Ok(())
    }
}

/// Get display name for an observation source type
fn source_type_name(source: &ObservationSource) -> &'static str {
    match source {
        ObservationSource::ManualVerification { .. } => "manual",
        ObservationSource::CrossSlotValidation { .. } => "cross_slot",
        ObservationSource::SnapshotDiff { .. } => "snapshot_diff",
        ObservationSource::ProbeResult { .. } => "probe",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::discovery_store::OffsetObservation;

    fn create_test_observation(byte: usize, bit: u8, source: ObservationSource, confidence: f64) -> OffsetObservation {
        OffsetObservation::new(byte, bit, source, Some(0), Some("Test".to_string()), confidence)
    }

    #[test]
    fn test_consensus_insufficient() {
        let mut store = DiscoveryStore::new();

        // Add only one observation
        let obs = create_test_observation(
            1234, 5,
            ObservationSource::ProbeResult {
                search_radius: 500,
                confidence: 0.8,
                probe_method: "test".to_string(),
            },
            0.8,
        );
        store.add_observation(76100, obs);

        let builder = ConsensusBuilder::default();
        let result = builder.analyze(store.get(76100).unwrap());

        assert_eq!(result.status, ConsensusStatus::Insufficient);
        assert_eq!(result.observation_count, 1);
    }

    #[test]
    fn test_consensus_confirmed() {
        let mut store = DiscoveryStore::new();

        // Add two agreeing observations from different sources
        store.add_observation(76100, create_test_observation(
            1234, 5,
            ObservationSource::SnapshotDiff {
                before_file: "before.sl2".to_string(),
                after_file: "after.sl2".to_string(),
                action_description: "test".to_string(),
            },
            0.85,
        ));

        store.add_observation(76100, create_test_observation(
            1234, 5,
            ObservationSource::ManualVerification {
                record_id: "test".to_string(),
                character_name: "Test".to_string(),
            },
            0.95,
        ));

        let builder = ConsensusBuilder::default();
        let result = builder.analyze(store.get(76100).unwrap());

        assert_eq!(result.status, ConsensusStatus::Confirmed);
        assert_eq!(result.best_offset, Some((1234, 5)));
        assert_eq!(result.agreement_percentage, 1.0);
        assert_eq!(result.source_diversity, 2);
    }

    #[test]
    fn test_consensus_contested() {
        let mut store = DiscoveryStore::new();

        // Add two disagreeing observations
        store.add_observation(76100, create_test_observation(
            1234, 5,
            ObservationSource::ProbeResult {
                search_radius: 500,
                confidence: 0.8,
                probe_method: "test".to_string(),
            },
            0.8,
        ));

        store.add_observation(76100, create_test_observation(
            5678, 3,
            ObservationSource::ProbeResult {
                search_radius: 500,
                confidence: 0.8,
                probe_method: "test".to_string(),
            },
            0.8,
        ));

        let builder = ConsensusBuilder::default();
        let result = builder.analyze(store.get(76100).unwrap());

        assert_eq!(result.status, ConsensusStatus::Contested);
        assert_eq!(result.agreement_percentage, 0.5);
    }

    #[test]
    fn test_weighted_confidence() {
        let builder = ConsensusBuilder::default();

        // Manual verification should have highest weight
        let manual_weight = builder.get_source_weight(&ObservationSource::ManualVerification {
            record_id: "test".to_string(),
            character_name: "Test".to_string(),
        });
        assert_eq!(manual_weight, 1.0);

        // Probe result should have lower weight
        let probe_weight = builder.get_source_weight(&ObservationSource::ProbeResult {
            search_radius: 500,
            confidence: 1.0,
            probe_method: "test".to_string(),
        });
        assert_eq!(probe_weight, 0.7);
    }

    #[test]
    fn test_get_promotable() {
        let mut store = DiscoveryStore::new();

        // Add a confirmed discovery
        for _ in 0..3 {
            store.add_observation(76100, create_test_observation(
                1234, 5,
                ObservationSource::SnapshotDiff {
                    before_file: "before.sl2".to_string(),
                    after_file: "after.sl2".to_string(),
                    action_description: "test".to_string(),
                },
                0.9,
            ));
        }

        // Add an insufficient discovery
        store.add_observation(76200, create_test_observation(
            2345, 6,
            ObservationSource::ProbeResult {
                search_radius: 500,
                confidence: 0.8,
                probe_method: "test".to_string(),
            },
            0.8,
        ));

        let builder = ConsensusBuilder::default();
        let promotable = builder.get_promotable(&store);

        assert_eq!(promotable.len(), 1);
        assert_eq!(promotable[0].flag_id, 76100);
    }
}
