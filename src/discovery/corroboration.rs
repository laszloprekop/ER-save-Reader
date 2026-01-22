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
use super::chain_data::{
    BOSS_DEFEAT_CHAINS, AREA_PREREQUISITES, GEOGRAPHIC_REGIONS,
    find_region_for_flag, find_boss_chain_by_defeat, find_boss_chain_by_remembrance,
    is_late_game_flag, get_geographic_correlations,
};
use super::event_graph::{EventGraph, FlagTrigger, ProgressionChain};

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

/// Result of boss defeat chain validation
#[derive(Debug, Clone)]
pub struct BossChainResult {
    pub boss_name: String,
    pub defeat_flag: u32,
    pub defeat_set: Option<bool>,
    pub remembrance_flag: u32,
    pub remembrance_set: Option<bool>,
    pub great_rune_flag: Option<u32>,
    pub great_rune_set: Option<bool>,
    pub activation_flag: Option<u32>,
    pub activation_set: Option<bool>,
    pub chain_valid: bool,
    pub contradiction: Option<String>,
}

/// Result of area prerequisite validation
#[derive(Debug, Clone)]
pub struct AreaPrerequisiteResult {
    pub area_name: String,
    pub area_flag_checked: u32,
    pub area_flag_set: Option<bool>,
    pub required_flags: Vec<(u32, Option<bool>)>,
    pub all_required_set: bool,
    pub any_required_set: bool,
    pub valid: bool,
    pub contradiction: Option<String>,
}

/// Result of geographic correlation validation
#[derive(Debug, Clone)]
pub struct GeographicCorrelationResult {
    pub region_name: String,
    pub source_flag: u32,
    pub source_set: Option<bool>,
    pub correlated_flags: Vec<(u32, &'static str, Option<bool>)>,
    pub correlation_ratio: f64,
}

/// Result of event graph validation (EMEVD evidence)
#[derive(Debug, Clone)]
pub struct EventGraphValidation {
    /// Flag has at least one SetEventFlagID trigger in EMEVD
    pub has_trigger: bool,
    /// Number of triggers found
    pub trigger_count: usize,
    /// Primary trigger context (e.g., "boss_defeat", "grace_discovery")
    pub trigger_context: Option<String>,
    /// Source files containing triggers
    pub source_files: Vec<String>,
    /// Related progression chain if any
    pub progression_chain: Option<String>,
    /// Validation confidence boost (0.0-0.2)
    pub confidence_boost: f64,
}

impl EventGraphValidation {
    /// Create validation result indicating flag exists in EMEVD
    pub fn found(
        trigger_count: usize,
        context: Option<String>,
        sources: Vec<String>,
        chain: Option<String>,
    ) -> Self {
        Self {
            has_trigger: true,
            trigger_count,
            trigger_context: context,
            source_files: sources,
            progression_chain: chain,
            confidence_boost: if trigger_count > 0 { 0.1 } else { 0.0 },
        }
    }

    /// Create validation result indicating flag NOT found in EMEVD
    pub fn not_found() -> Self {
        Self {
            has_trigger: false,
            trigger_count: 0,
            trigger_context: None,
            source_files: Vec::new(),
            progression_chain: None,
            confidence_boost: 0.0,
        }
    }
}

/// Result of corroboration check for a flag
#[derive(Debug, Clone)]
pub struct CorroborationResult {
    pub flag_id: u32,
    pub status: CorroborationStatus,
    pub related_checks: Vec<RelatedFlagCheck>,
    pub dual_formula: Option<DualFormulaResult>,
    pub boss_chain: Option<BossChainResult>,
    pub area_prerequisite: Option<AreaPrerequisiteResult>,
    pub geographic_correlation: Option<GeographicCorrelationResult>,
    /// Event graph validation (EMEVD evidence)
    pub event_graph: Option<EventGraphValidation>,
    pub agreement_ratio: f64,
    pub confidence_adjustment: f64,
}

/// Corroboration engine using relationship graph
pub struct CorroborationEngine {
    graph: Arc<RelationshipGraph>,
    event_graph: Option<Arc<EventGraph>>,
    config: CorroborationConfig,
}

impl CorroborationEngine {
    /// Create a new engine with the given relationship graph
    pub fn new(graph: Arc<RelationshipGraph>) -> Self {
        Self {
            graph,
            event_graph: None,
            config: CorroborationConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(graph: Arc<RelationshipGraph>, config: CorroborationConfig) -> Self {
        Self { graph, event_graph: None, config }
    }

    /// Add event graph for EMEVD validation
    pub fn with_event_graph(mut self, event_graph: Arc<EventGraph>) -> Self {
        self.event_graph = Some(event_graph);
        self
    }

    /// Load graphs from default locations
    pub fn load_default() -> Result<Self, String> {
        let graph = RelationshipGraph::load_default()
            .map_err(|e| format!("Failed to load relationship graph: {}", e))?;
        Ok(Self::new(Arc::new(graph)))
    }

    /// Load with event graph from default locations
    pub fn load_with_event_graph() -> Result<Self, String> {
        let graph = RelationshipGraph::load_default()
            .map_err(|e| format!("Failed to load relationship graph: {}", e))?;
        let event_graph = EventGraph::load_default()
            .map_err(|e| format!("Failed to load event graph: {}", e))?;
        Ok(Self::new(Arc::new(graph)).with_event_graph(Arc::new(event_graph)))
    }

    /// Check if event graph is available
    pub fn has_event_graph(&self) -> bool {
        self.event_graph.is_some()
    }

    /// Get event graph summary if available
    pub fn event_graph_summary(&self) -> Option<String> {
        self.event_graph.as_ref().map(|eg| {
            let summary = eg.summary();
            format!(
                "EventGraph: {} flags, {} triggers, {} chains",
                summary.total_flags, summary.total_triggers, summary.progression_chains
            )
        })
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
                RelationshipType::BossDefeatChain => {
                    // Boss chain: if later flag is set, earlier must be set
                    Some(expected_set)
                }
                RelationshipType::AreaPrerequisite => {
                    // Area prerequisite: if area flag set, prerequisite must be set
                    if expected_set { Some(true) } else { None }
                }
                RelationshipType::GeographicProximity => {
                    // Geographic proximity: soft correlation
                    None  // Don't count towards agreement, just informational
                }
                RelationshipType::ScrollUnlock => {
                    // If unlock (spell) is available, scroll pickup should be set
                    if expected_set { Some(true) } else { None }
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

        // Check boss defeat chain if applicable
        let boss_chain = self.check_boss_chain(flag_id, event_flags);
        if let Some(ref bc) = boss_chain {
            if bc.chain_valid {
                agrees += 1;
            } else if bc.contradiction.is_some() {
                disagrees += 2; // Boss chain contradictions are severe
            }
        }

        // Check area prerequisite if this is a late-game flag
        let area_prerequisite = self.check_area_prerequisite(flag_id, expected_set, event_flags);
        if let Some(ref ap) = area_prerequisite {
            if ap.valid {
                agrees += 1;
            } else if ap.contradiction.is_some() {
                disagrees += 2; // Area prerequisite contradictions are severe
            }
        }

        // Check geographic correlation
        let geographic_correlation = self.check_geographic_correlation(flag_id, event_flags);

        // Check event graph validation (EMEVD evidence)
        let event_graph = self.check_event_graph(flag_id);
        if let Some(ref eg) = event_graph {
            if eg.has_trigger {
                agrees += 1;  // Flag exists in EMEVD
            }
            // Note: Not having a trigger isn't necessarily a contradiction,
            // as some flags may be set through other mechanisms
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

        // Include event graph confidence boost
        let event_graph_boost = event_graph.as_ref().map(|eg| eg.confidence_boost).unwrap_or(0.0);

        CorroborationResult {
            flag_id,
            status,
            related_checks,
            dual_formula,
            boss_chain,
            area_prerequisite,
            geographic_correlation,
            event_graph,
            agreement_ratio,
            confidence_adjustment: confidence_adjustment + event_graph_boost,
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

    /// Check boss defeat chain validation
    /// Validates that boss defeat → remembrance → great rune → activation chain is consistent
    fn check_boss_chain(&self, flag_id: u32, event_flags: &[u8]) -> Option<BossChainResult> {
        // Check if this flag is part of a boss chain
        let chain = if let Some(c) = find_boss_chain_by_defeat(flag_id) {
            c
        } else if let Some(c) = find_boss_chain_by_remembrance(flag_id) {
            c
        } else if (160..=167).contains(&flag_id) {
            // Great rune possession flags
            BOSS_DEFEAT_CHAINS.iter().find(|c| c.great_rune_flag == Some(flag_id))?
        } else if (180..=187).contains(&flag_id) {
            // Great rune activation flags
            BOSS_DEFEAT_CHAINS.iter().find(|c| c.activation_flag == Some(flag_id))?
        } else if (9101..=9120).contains(&flag_id) {
            // Remembrance flags
            find_boss_chain_by_remembrance(flag_id)?
        } else {
            return None;
        };

        // Read all chain flags
        let defeat_set = self.read_flag(chain.defeat_flag, event_flags);
        let remembrance_set = self.read_flag(chain.remembrance_flag, event_flags);
        let great_rune_set = chain.great_rune_flag.and_then(|f| self.read_flag(f, event_flags));
        let activation_set = chain.activation_flag.and_then(|f| self.read_flag(f, event_flags));

        // Check for contradictions (later flags set without earlier flags)
        let mut contradiction = None;

        // Remembrance requires defeat
        if remembrance_set == Some(true) && defeat_set == Some(false) {
            contradiction = Some(format!("Remembrance set but {} not defeated", chain.name));
        }

        // Great rune requires defeat
        if great_rune_set == Some(true) && defeat_set == Some(false) {
            contradiction = Some(format!("Great Rune possessed but {} not defeated", chain.name));
        }

        // Activation requires possession
        if activation_set == Some(true) && great_rune_set == Some(false) {
            contradiction = Some(format!("Great Rune activated but not possessed for {}", chain.name));
        }

        // Chain is valid if no contradictions and at least one flag readable
        let chain_valid = contradiction.is_none() &&
            (defeat_set.is_some() || remembrance_set.is_some());

        Some(BossChainResult {
            boss_name: chain.name.to_string(),
            defeat_flag: chain.defeat_flag,
            defeat_set,
            remembrance_flag: chain.remembrance_flag,
            remembrance_set,
            great_rune_flag: chain.great_rune_flag,
            great_rune_set,
            activation_flag: chain.activation_flag,
            activation_set,
            chain_valid,
            contradiction,
        })
    }

    /// Check area prerequisite validation
    /// Validates that late-game area flags are only set if prerequisites are met
    fn check_area_prerequisite(&self, flag_id: u32, expected_set: bool, event_flags: &[u8]) -> Option<AreaPrerequisiteResult> {
        // Only check if flag is expected to be set and is in a late-game area
        if !expected_set || !is_late_game_flag(flag_id) {
            return None;
        }

        // Find which area this flag belongs to
        let area = AREA_PREREQUISITES.iter().find(|a| {
            // Check landmark range
            if let Some((start, end)) = a.landmark_range {
                if flag_id >= start && flag_id <= end {
                    return true;
                }
            }
            // Check area flags
            flag_id >= a.area_flags_start && flag_id < a.area_flags_start + 10_000_000
        })?;

        // Read area flag status
        let area_flag_set = self.read_flag(flag_id, event_flags);

        // Read all required flags
        let required_flags: Vec<(u32, Option<bool>)> = area.required_flags
            .iter()
            .map(|&f| (f, self.read_flag(f, event_flags)))
            .collect();

        // Check if all required flags are set
        let all_required_set = required_flags.iter()
            .all(|(_, set)| *set == Some(true));

        // Read any-of flags
        let any_required: Vec<(u32, Option<bool>)> = area.required_any
            .iter()
            .map(|&f| (f, self.read_flag(f, event_flags)))
            .collect();

        // Check if at least one any-required flag is set (or no any-required)
        let any_required_set = area.required_any.is_empty() ||
            any_required.iter().any(|(_, set)| *set == Some(true));

        // Determine if valid
        let prerequisites_met = (area.required_flags.is_empty() || all_required_set) &&
            (area.required_any.is_empty() || any_required_set);

        let contradiction = if area_flag_set == Some(true) && !prerequisites_met {
            Some(format!("{} flag set without prerequisites", area.area_name))
        } else {
            None
        };

        let valid = contradiction.is_none();

        // Combine required flags
        let mut all_required = required_flags;
        all_required.extend(any_required);

        Some(AreaPrerequisiteResult {
            area_name: area.area_name.to_string(),
            area_flag_checked: flag_id,
            area_flag_set,
            required_flags: all_required,
            all_required_set,
            any_required_set,
            valid,
            contradiction,
        })
    }

    /// Check geographic correlation
    /// Looks at other flags in the same region for soft correlation
    fn check_geographic_correlation(&self, flag_id: u32, event_flags: &[u8]) -> Option<GeographicCorrelationResult> {
        let region = find_region_for_flag(flag_id)?;

        let source_set = self.read_flag(flag_id, event_flags);

        // Get correlated flags
        let correlations = get_geographic_correlations(flag_id);
        if correlations.is_empty() {
            return None;
        }

        let correlated_flags: Vec<(u32, &'static str, Option<bool>)> = correlations
            .into_iter()
            .map(|(f, desc)| (f, desc, self.read_flag(f, event_flags)))
            .collect();

        // Calculate correlation ratio
        let readable: Vec<_> = correlated_flags.iter()
            .filter(|(_, _, set)| set.is_some())
            .collect();

        let correlation_ratio = if !readable.is_empty() && source_set.is_some() {
            let source_val = source_set.unwrap();
            let matching = readable.iter()
                .filter(|(_, _, set)| set.unwrap() == source_val)
                .count();
            matching as f64 / readable.len() as f64
        } else {
            0.0
        };

        Some(GeographicCorrelationResult {
            region_name: region.name.to_string(),
            source_flag: flag_id,
            source_set,
            correlated_flags,
            correlation_ratio,
        })
    }

    /// Check event graph validation (EMEVD evidence)
    /// Validates that the flag has a SetEventFlagID trigger in EMEVD files
    fn check_event_graph(&self, flag_id: u32) -> Option<EventGraphValidation> {
        let event_graph = self.event_graph.as_ref()?;

        if event_graph.has_trigger(flag_id) {
            let triggers = event_graph.get_triggers(flag_id);
            let trigger_count = triggers.map(|t| t.len()).unwrap_or(0);
            let context = event_graph.get_trigger_context(flag_id).map(|s| s.to_string());
            let sources: Vec<String> = triggers
                .map(|t| t.iter().map(|tr| tr.source_file.clone()).collect())
                .unwrap_or_default();

            // Check for progression chain
            let chain = event_graph.find_remembrance_chain(flag_id)
                .map(|c| format!("remembrance_{}", flag_id))
                .or_else(|| event_graph.find_map_fragment_chain(flag_id)
                    .map(|c| format!("map_fragment_{}", flag_id)));

            Some(EventGraphValidation::found(trigger_count, context, sources, chain))
        } else {
            Some(EventGraphValidation::not_found())
        }
    }

    /// Validate a flag using only the event graph (without save data)
    pub fn validate_via_event_graph(&self, flag_id: u32) -> Option<EventGraphValidation> {
        self.check_event_graph(flag_id)
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
