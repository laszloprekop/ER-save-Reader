/// Corroboration Engine
///
/// Multi-point validation using the flag relationship graph.
/// Validates discoveries by checking related flags and dual-formula pairs.
///
/// ## Validation Strategies:
/// 1. **Dual-formula validation**: Check both tile flag and block flag offsets
/// 2. **Related flag validation**: Check flags connected by relationships
/// 3. **Cross-slot validation**: Check same flag across multiple save slots

use std::path::Path;
use std::sync::Arc;

use crate::save::save::save::Save;
use crate::db::pickup_flags::EVENT_FLAGS_SIZE;

use super::relationship_graph::{RelationshipGraph, CorroborationPair, RelationshipType};
use super::discovery_store::{DiscoveryStore, OffsetObservation, ObservationSource};

/// Configuration for corroboration engine
#[derive(Debug, Clone)]
pub struct CorroborationConfig {
    /// Minimum agreement ratio for strong corroboration
    pub strong_threshold: f64,
    /// Minimum agreement ratio for weak corroboration
    pub weak_threshold: f64,
    /// Confidence boost for strong corroboration
    pub strong_confidence_boost: f64,
    /// Confidence boost for weak corroboration
    pub weak_confidence_boost: f64,
    /// Penalty for contradictions
    pub contradiction_penalty: f64,
}

impl Default for CorroborationConfig {
    fn default() -> Self {
        Self {
            strong_threshold: 0.8,
            weak_threshold: 0.5,
            strong_confidence_boost: 0.15,
            weak_confidence_boost: 0.05,
            contradiction_penalty: 0.3,
        }
    }
}

/// Status of corroboration check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorroborationStatus {
    /// >= 80% of related flags agree
    StrongCorroboration,
    /// 50-80% agreement
    WeakCorroboration,
    /// < 50% agreement or no related flags
    Inconclusive,
    /// Related flags actively contradict
    Contradiction,
}

impl std::fmt::Display for CorroborationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StrongCorroboration => write!(f, "Strong Corroboration"),
            Self::WeakCorroboration => write!(f, "Weak Corroboration"),
            Self::Inconclusive => write!(f, "Inconclusive"),
            Self::Contradiction => write!(f, "Contradiction"),
        }
    }
}

/// Result of checking a single related flag
#[derive(Debug, Clone)]
pub struct RelatedFlagCheck {
    pub flag_id: u32,
    pub relationship_type: RelationshipType,
    pub expected_set: bool,
    pub actual_set: Option<bool>,
    pub agrees: bool,
    pub notes: Option<String>,
}

/// Result of dual-formula validation
#[derive(Debug, Clone)]
pub struct DualFormulaResult {
    pub tile_flag: u32,
    pub block_flag: u32,
    pub tile_offset: Option<(usize, u8)>,
    pub block_offset: Option<(usize, u8)>,
    pub tile_set: Option<bool>,
    pub block_set: Option<bool>,
    pub both_agree: bool,
    pub item_name: Option<String>,
}

/// Result of corroboration check for a flag
#[derive(Debug, Clone)]
pub struct CorroborationResult {
    pub flag_id: u32,
    pub status: CorroborationStatus,
    pub related_checks: Vec<RelatedFlagCheck>,
    pub dual_formula: Option<DualFormulaResult>,
    pub agreement_ratio: f64,
    pub confidence_adjustment: f64,
}

/// Corroboration engine using relationship graph
pub struct CorroborationEngine {
    graph: Arc<RelationshipGraph>,
    config: CorroborationConfig,
}

impl CorroborationEngine {
    /// Create a new engine with the given graph
    pub fn new(graph: Arc<RelationshipGraph>) -> Self {
        Self {
            graph,
            config: CorroborationConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(graph: Arc<RelationshipGraph>, config: CorroborationConfig) -> Self {
        Self { graph, config }
    }

    /// Load graph from default location
    pub fn load_default() -> Result<Self, String> {
        let graph = RelationshipGraph::load_default()
            .map_err(|e| format!("Failed to load relationship graph: {}", e))?;
        Ok(Self::new(Arc::new(graph)))
    }

    /// Check corroboration for a flag using event flags data
    pub fn check_corroboration(
        &self,
        flag_id: u32,
        expected_set: bool,
        event_flags: &[u8],
    ) -> CorroborationResult {
        let mut related_checks = Vec::new();
        let mut agrees = 0;
        let mut disagrees = 0;

        // Check related flags
        for rel in self.graph.get_related(flag_id) {
            let related_flag = if rel.source_flag == flag_id {
                rel.target_flag
            } else {
                rel.source_flag
            };

            // Try to read the related flag
            let actual = self.read_flag(related_flag, event_flags);

            // Determine expected state based on relationship type
            let expected_related: Option<bool> = match rel.relationship_type {
                RelationshipType::PickupSetsFlag => {
                    // If flag_id is the target (block flag), source (tile) should also be set
                    Some(expected_set)
                }
                RelationshipType::EnablesPurchase => {
                    // If release flag is set, stock flag might or might not be set
                    // This is one-way: release enables purchase, but purchase doesn't require release
                    if rel.source_flag == flag_id {
                        // Can't infer stock from release
                        None
                    } else {
                        // If stock is set, release should be set
                        if expected_set { Some(true) } else { None }
                    }
                }
                RelationshipType::GraceDiscovery => {
                    // Entity triggers grace flag - if grace is set, entity should have triggered
                    Some(expected_set)
                }
                RelationshipType::BossRemembrance => {
                    // Boss remembrance links - usually both set together
                    Some(expected_set)
                }
                RelationshipType::EventSequence => {
                    // Event sequence - flags set together
                    Some(expected_set)
                }
                RelationshipType::MapFragment => {
                    // Map discovery links
                    Some(expected_set)
                }
            };

            // Skip if can't determine expectation
            let expected_related = match expected_related {
                Some(exp) => exp,
                None => continue,
            };

            let agrees_with_expectation = actual.map(|a| a == expected_related).unwrap_or(false);

            if let Some(actual_val) = actual {
                if actual_val == expected_related {
                    agrees += 1;
                } else {
                    disagrees += 1;
                }
            }

            related_checks.push(RelatedFlagCheck {
                flag_id: related_flag,
                relationship_type: rel.relationship_type,
                expected_set: expected_related,
                actual_set: actual,
                agrees: agrees_with_expectation,
                notes: rel.notes.clone(),
            });
        }

        // Check dual-formula pair if this is a block flag
        let dual_formula = self.check_dual_formula(flag_id, expected_set, event_flags);

        // If dual formula check is available, include in agreement calculation
        if let Some(ref df) = dual_formula {
            if df.both_agree {
                agrees += 2; // Weight dual-formula higher
            } else if df.tile_set.is_some() && df.block_set.is_some() {
                disagrees += 1;
            }
        }

        // Calculate agreement ratio
        let total_checks = agrees + disagrees;
        let agreement_ratio = if total_checks > 0 {
            agrees as f64 / total_checks as f64
        } else {
            0.0
        };

        // Determine status
        let status = if total_checks == 0 {
            CorroborationStatus::Inconclusive
        } else if agreement_ratio >= self.config.strong_threshold {
            CorroborationStatus::StrongCorroboration
        } else if agreement_ratio >= self.config.weak_threshold {
            CorroborationStatus::WeakCorroboration
        } else if disagrees > agrees {
            CorroborationStatus::Contradiction
        } else {
            CorroborationStatus::Inconclusive
        };

        // Calculate confidence adjustment
        let confidence_adjustment = match status {
            CorroborationStatus::StrongCorroboration => self.config.strong_confidence_boost,
            CorroborationStatus::WeakCorroboration => self.config.weak_confidence_boost,
            CorroborationStatus::Inconclusive => 0.0,
            CorroborationStatus::Contradiction => -self.config.contradiction_penalty,
        };

        CorroborationResult {
            flag_id,
            status,
            related_checks,
            dual_formula,
            agreement_ratio,
            confidence_adjustment,
        }
    }

    /// Check dual-formula corroboration for a block flag
    fn check_dual_formula(
        &self,
        flag_id: u32,
        expected_set: bool,
        event_flags: &[u8],
    ) -> Option<DualFormulaResult> {
        // Only for block flags (5-digit, 60000-99999)
        if flag_id < 60000 || flag_id >= 100_000 {
            return None;
        }

        // Find corroboration pair
        let pair = self.graph.find_corroboration_for_block(flag_id)?;

        // Calculate offsets
        let block_offset = self.calculate_block_offset(pair.block_flag);
        let tile_offset = self.calculate_tile_offset(pair.tile_flag);

        // Read both flags
        let block_set = block_offset.and_then(|(byte, bit)| {
            if byte < event_flags.len() {
                Some((event_flags[byte] & (1 << bit)) != 0)
            } else {
                None
            }
        });

        let tile_set = tile_offset.and_then(|(byte, bit)| {
            if byte < event_flags.len() {
                Some((event_flags[byte] & (1 << bit)) != 0)
            } else {
                None
            }
        });

        // Check if both agree
        let both_agree = match (tile_set, block_set) {
            (Some(t), Some(b)) => t == b && t == expected_set,
            _ => false,
        };

        Some(DualFormulaResult {
            tile_flag: pair.tile_flag,
            block_flag: pair.block_flag,
            tile_offset,
            block_offset,
            tile_set,
            block_set,
            both_agree,
            item_name: pair.item_name.clone(),
        })
    }

    /// Read a flag from event flags data
    fn read_flag(&self, flag_id: u32, event_flags: &[u8]) -> Option<bool> {
        let (byte, bit) = self.calculate_flag_offset(flag_id)?;
        if byte < event_flags.len() {
            Some((event_flags[byte] & (1 << bit)) != 0)
        } else {
            None
        }
    }

    /// Calculate offset for any flag type
    fn calculate_flag_offset(&self, flag_id: u32) -> Option<(usize, u8)> {
        if flag_id >= 1_000_000_000 {
            self.calculate_tile_offset(flag_id)
        } else if flag_id >= 10_000_000 {
            self.calculate_dungeon_offset(flag_id)
        } else if flag_id >= 60000 && flag_id < 100_000 {
            self.calculate_block_offset(flag_id)
        } else {
            None
        }
    }

    /// Calculate offset for block-based flags (60000-99999)
    fn calculate_block_offset(&self, flag_id: u32) -> Option<(usize, u8)> {
        use crate::generated::ground_truth::VERIFIED_BLOCK_BASES;

        let block_start = (flag_id / 1000) * 1000;
        let base = VERIFIED_BLOCK_BASES.get(&block_start)?;

        if base.base_offset == 0 {
            return None;
        }

        let relative = flag_id - block_start;
        let byte_offset = base.base_offset as usize + (relative / 8) as usize;
        let bit_position = 7 - ((flag_id % 8) as u8);

        Some((byte_offset, bit_position))
    }

    /// Calculate offset for tile-based flags (10-digit)
    fn calculate_tile_offset(&self, flag_id: u32) -> Option<(usize, u8)> {
        use crate::generated::ground_truth::{
            VERIFIED_TILE_BASE_OFFSET, TILE_BYTES_PER_SLOT,
            TILE_SLOTS_PER_ROW, TILE_ROW_BASE, TILE_COL_BASE, TILE_MAX_LOCAL_ID,
        };

        if flag_id < 1_000_000_000 {
            return None;
        }

        let tile_index = (flag_id - 1_000_000_000) / 10000;
        let local_id = flag_id % 10000;

        if local_id >= TILE_MAX_LOCAL_ID {
            return None;
        }

        let row = tile_index / 100;
        let col = tile_index % 100;

        let slot = (row as i32 - TILE_ROW_BASE as i32) * TILE_SLOTS_PER_ROW as i32
            + (col as i32 - TILE_COL_BASE as i32);

        if slot < 0 {
            return None;
        }

        let byte_offset = VERIFIED_TILE_BASE_OFFSET as usize
            + (slot as usize) * TILE_BYTES_PER_SLOT as usize
            + (local_id / 8) as usize;
        let bit_position = 7 - ((local_id % 8) as u8);

        Some((byte_offset, bit_position))
    }

    /// Calculate offset for dungeon-based flags (8-digit)
    fn calculate_dungeon_offset(&self, flag_id: u32) -> Option<(usize, u8)> {
        use crate::generated::ground_truth::VERIFIED_DUNGEON_BASES;

        if flag_id < 10_000_000 || flag_id >= 100_000_000 {
            return None;
        }

        let area = flag_id / 1_000_000;
        let section = (flag_id / 10_000) % 100;
        let local_id = flag_id % 10_000;

        let base = VERIFIED_DUNGEON_BASES.get(&area)?;
        if base.base_offset == 0 {
            return None;
        }

        let byte_offset = base.base_offset as usize
            + section as usize * base.section_size as usize
            + (local_id / 8) as usize;
        let bit_position = 7 - ((local_id % 8) as u8);

        Some((byte_offset, bit_position))
    }

    /// Validate all corroboration pairs against a save file
    pub fn validate_all_pairs(&self, event_flags: &[u8]) -> BatchCorroborationResult {
        let pairs = self.graph.get_corroboration_pairs();
        let mut results = Vec::new();
        let mut agreements = 0;
        let mut contradictions = 0;
        let mut inconclusive = 0;

        for pair in pairs {
            let tile_set = self.calculate_tile_offset(pair.tile_flag)
                .and_then(|(byte, bit)| {
                    if byte < event_flags.len() {
                        Some((event_flags[byte] & (1 << bit)) != 0)
                    } else {
                        None
                    }
                });

            let block_set = self.calculate_block_offset(pair.block_flag)
                .and_then(|(byte, bit)| {
                    if byte < event_flags.len() {
                        Some((event_flags[byte] & (1 << bit)) != 0)
                    } else {
                        None
                    }
                });

            let status = match (tile_set, block_set) {
                (Some(t), Some(b)) if t == b => {
                    agreements += 1;
                    PairStatus::Agrees
                }
                (Some(t), Some(b)) if t != b => {
                    contradictions += 1;
                    PairStatus::Contradicts
                }
                _ => {
                    inconclusive += 1;
                    PairStatus::Inconclusive
                }
            };

            results.push(PairValidationResult {
                tile_flag: pair.tile_flag,
                block_flag: pair.block_flag,
                tile_set,
                block_set,
                status,
                item_name: pair.item_name.clone(),
            });
        }

        BatchCorroborationResult {
            total_pairs: pairs.len(),
            agreements,
            contradictions,
            inconclusive,
            results,
        }
    }

    /// Get the relationship graph
    pub fn graph(&self) -> &RelationshipGraph {
        &self.graph
    }
}

/// Status of a dual-formula pair validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairStatus {
    Agrees,
    Contradicts,
    Inconclusive,
}

/// Result of validating a single corroboration pair
#[derive(Debug, Clone)]
pub struct PairValidationResult {
    pub tile_flag: u32,
    pub block_flag: u32,
    pub tile_set: Option<bool>,
    pub block_set: Option<bool>,
    pub status: PairStatus,
    pub item_name: Option<String>,
}

/// Result of batch corroboration validation
#[derive(Debug)]
pub struct BatchCorroborationResult {
    pub total_pairs: usize,
    pub agreements: usize,
    pub contradictions: usize,
    pub inconclusive: usize,
    pub results: Vec<PairValidationResult>,
}

impl std::fmt::Display for BatchCorroborationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Batch Corroboration Result:")?;
        writeln!(f, "  Total pairs: {}", self.total_pairs)?;
        writeln!(f, "  Agreements: {} ({:.1}%)",
            self.agreements,
            (self.agreements as f64 / self.total_pairs as f64) * 100.0)?;
        writeln!(f, "  Contradictions: {}", self.contradictions)?;
        writeln!(f, "  Inconclusive: {}", self.inconclusive)?;
        Ok(())
    }
}

/// Create observation from corroboration result
pub fn create_corroboration_observation(
    result: &CorroborationResult,
    byte_offset: usize,
    bit_position: u8,
) -> Option<OffsetObservation> {
    if result.status == CorroborationStatus::Inconclusive {
        return None;
    }

    let base_confidence = match result.status {
        CorroborationStatus::StrongCorroboration => 0.9,
        CorroborationStatus::WeakCorroboration => 0.7,
        CorroborationStatus::Inconclusive => 0.5,
        CorroborationStatus::Contradiction => 0.3,
    };

    Some(OffsetObservation::new(
        byte_offset,
        bit_position,
        ObservationSource::CrossSlotValidation {
            slots_validated: vec![],
            all_matched: result.status == CorroborationStatus::StrongCorroboration,
        },
        None,
        None,
        base_confidence + result.confidence_adjustment,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corroboration_config() {
        let config = CorroborationConfig::default();
        assert_eq!(config.strong_threshold, 0.8);
        assert_eq!(config.weak_threshold, 0.5);
    }

    #[test]
    fn test_corroboration_status_display() {
        assert_eq!(format!("{}", CorroborationStatus::StrongCorroboration), "Strong Corroboration");
        assert_eq!(format!("{}", CorroborationStatus::Contradiction), "Contradiction");
    }

    #[test]
    #[ignore] // Requires relationship graph
    fn test_corroboration_engine() {
        if let Ok(engine) = CorroborationEngine::load_default() {
            println!("Loaded graph with {} relationships", engine.graph().len());
            println!("Found {} corroboration pairs", engine.graph().corroboration_pair_count());
        }
    }
}
