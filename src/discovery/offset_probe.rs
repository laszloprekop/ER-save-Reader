/// Automated Offset Probing Module
///
/// Searches for the correct byte offsets of flags that fail verification.
/// Uses multiple strategies:
/// 1. Regional search - Search expected region for matching bits
/// 2. Pattern matching - Use known working flags to infer offsets
/// 3. Cross-slot validation - Compare same flag across different save slots

use std::path::Path;

use crate::db::pickup_flags::{get_flag_offset, is_flag_set, EVENT_FLAGS_SIZE};
use crate::generated::ground_truth::{VERIFIED_BLOCK_BASES, VERIFIED_DUNGEON_BASES};

use super::discovery_store::{
    DiscoveryStore, OffsetObservation, ObservationSource, StoreError,
};

/// A candidate offset found during probing
#[derive(Debug, Clone)]
pub struct OffsetCandidate {
    pub byte_offset: usize,
    pub bit_position: u8,
    pub confidence: f64,
    pub reason: String,
    /// If we can reverse-calculate a flag ID, store it
    pub reverse_flag_id: Option<u32>,
}

/// Result of probing for a single flag
#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub flag_id: u32,
    pub flag_name: String,
    pub expected_value: bool,
    pub calculated_offset: Option<(u32, u8)>,
    pub actual_value_at_calculated: bool,
    pub candidates: Vec<OffsetCandidate>,
    pub best_candidate: Option<OffsetCandidate>,
    /// Correction to apply if found
    pub correction: Option<OffsetCorrection>,
}

/// A verified offset correction
#[derive(Debug, Clone)]
pub struct OffsetCorrection {
    pub flag_id: u32,
    pub old_offset: (u32, u8),
    pub new_offset: (usize, u8),
    pub confidence: f64,
    pub validation_method: String,
}

/// Probe configuration
#[derive(Debug, Clone)]
pub struct ProbeConfig {
    /// How far to search around calculated offset (bytes)
    pub search_radius: usize,
    /// Minimum confidence to accept a candidate
    pub min_confidence: f64,
    /// Whether to use cross-slot validation
    pub cross_slot_validation: bool,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            search_radius: 500,
            min_confidence: 0.7,
            cross_slot_validation: true,
        }
    }
}

/// Main probing engine
pub struct OffsetProber {
    config: ProbeConfig,
}

impl OffsetProber {
    pub fn new(config: ProbeConfig) -> Self {
        Self { config }
    }

    /// Probe for the correct offset of a single flag
    pub fn probe_flag(
        &self,
        event_flags: &[u8],
        flag_id: u32,
        flag_name: &str,
        expected_value: bool,
    ) -> ProbeResult {
        let calculated_offset = get_flag_offset(flag_id);
        let actual_at_calculated = is_flag_set(event_flags, flag_id);

        let mut result = ProbeResult {
            flag_id,
            flag_name: flag_name.to_string(),
            expected_value,
            calculated_offset,
            actual_value_at_calculated: actual_at_calculated,
            candidates: Vec::new(),
            best_candidate: None,
            correction: None,
        };

        // If already correct, no probing needed
        if actual_at_calculated == expected_value {
            return result;
        }

        // Choose probing strategy based on flag type and expected value
        let candidates = if expected_value {
            // Flag should be TRUE but we got FALSE
            // Search for set bits that could be this flag
            self.search_for_set_bit(event_flags, flag_id, calculated_offset)
        } else {
            // Flag should be FALSE but we got TRUE
            // The calculated offset is wrong - search for the correct unset location
            self.search_for_unset_bit(event_flags, flag_id, calculated_offset)
        };

        result.candidates = candidates;

        // Select best candidate
        if let Some(best) = result.candidates.iter()
            .filter(|c| c.confidence >= self.config.min_confidence)
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
        {
            result.best_candidate = Some(best.clone());

            if let Some((calc_byte, calc_bit)) = calculated_offset {
                result.correction = Some(OffsetCorrection {
                    flag_id,
                    old_offset: (calc_byte, calc_bit),
                    new_offset: (best.byte_offset, best.bit_position),
                    confidence: best.confidence,
                    validation_method: best.reason.clone(),
                });
            }
        }

        result
    }

    /// Search for a set bit that could be the correct location for a flag
    fn search_for_set_bit(
        &self,
        event_flags: &[u8],
        flag_id: u32,
        calculated_offset: Option<(u32, u8)>,
    ) -> Vec<OffsetCandidate> {
        let mut candidates = Vec::new();

        // Determine search region based on flag type
        let search_regions = self.get_search_regions(flag_id, calculated_offset);

        for (region_start, region_end, region_name) in search_regions {
            let actual_end = region_end.min(event_flags.len());

            for byte_idx in region_start..actual_end {
                let byte = event_flags[byte_idx];
                if byte == 0 {
                    continue;
                }

                // Check each set bit in this byte
                for bit in 0..8 {
                    if (byte & (1 << bit)) != 0 {
                        let bit_pos = 7 - bit; // Convert to MSB-first

                        // Calculate what flag ID this position would represent
                        let reverse_id = self.reverse_calculate_flag_id(
                            flag_id, byte_idx, bit_pos as u8, calculated_offset
                        );

                        // Score this candidate
                        let confidence = self.score_candidate(
                            flag_id,
                            byte_idx,
                            bit_pos as u8,
                            calculated_offset,
                            reverse_id,
                            &region_name,
                        );

                        if confidence > 0.3 {
                            candidates.push(OffsetCandidate {
                                byte_offset: byte_idx,
                                bit_position: bit_pos as u8,
                                confidence,
                                reason: format!("Found in {} region", region_name),
                                reverse_flag_id: reverse_id,
                            });
                        }
                    }
                }
            }
        }

        // Sort by confidence
        candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        candidates.truncate(10); // Keep top 10
        candidates
    }

    /// Search for an unset bit (when calculated offset incorrectly shows TRUE)
    fn search_for_unset_bit(
        &self,
        event_flags: &[u8],
        flag_id: u32,
        calculated_offset: Option<(u32, u8)>,
    ) -> Vec<OffsetCandidate> {
        // This is trickier - we're looking for where the flag SHOULD be (unset)
        // For now, return candidates from expected regions that are unset
        let mut candidates = Vec::new();

        let search_regions = self.get_search_regions(flag_id, calculated_offset);

        for (region_start, region_end, region_name) in search_regions {
            let actual_end = region_end.min(event_flags.len());

            // Sample the region for unset bits
            for byte_idx in (region_start..actual_end).step_by(10) {
                let byte = event_flags[byte_idx];

                for bit in 0..8 {
                    if (byte & (1 << bit)) == 0 {
                        let bit_pos = 7 - bit;

                        let reverse_id = self.reverse_calculate_flag_id(
                            flag_id, byte_idx, bit_pos as u8, calculated_offset
                        );

                        // Only consider if reverse ID matches expected pattern
                        if let Some(rev_id) = reverse_id {
                            if self.flag_ids_similar(flag_id, rev_id) {
                                candidates.push(OffsetCandidate {
                                    byte_offset: byte_idx,
                                    bit_position: bit_pos as u8,
                                    confidence: 0.5,
                                    reason: format!("Unset in {} (reverse: {})", region_name, rev_id),
                                    reverse_flag_id: Some(rev_id),
                                });
                            }
                        }
                    }
                }
            }
        }

        candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        candidates.truncate(10);
        candidates
    }

    /// Get search regions based on flag type
    fn get_search_regions(
        &self,
        flag_id: u32,
        calculated_offset: Option<(u32, u8)>,
    ) -> Vec<(usize, usize, String)> {
        let mut regions = Vec::new();

        // Always search around calculated offset if available
        if let Some((calc_byte, _)) = calculated_offset {
            let center = calc_byte as usize;
            let start = center.saturating_sub(self.config.search_radius);
            let end = (center + self.config.search_radius).min(EVENT_FLAGS_SIZE as usize);
            regions.push((start, end, "calculated_vicinity".to_string()));
        }

        // Add type-specific regions
        if flag_id >= 1_000_000_000 {
            // Tile flag - search tile region
            let tile_base = 495830usize;
            let tile_end = (tile_base + 700_000).min(EVENT_FLAGS_SIZE as usize);
            regions.push((tile_base, tile_end, "tile_flags".to_string()));
        } else if flag_id >= 10_000_000 && flag_id < 44_000_000 {
            // Dungeon flag - search much wider for these
            let area = flag_id / 1_000_000;
            let section = (flag_id / 10_000) % 100;

            // Search verified dungeon bases
            if let Some(dungeon_info) = VERIFIED_DUNGEON_BASES.get(&area) {
                if dungeon_info.base_offset > 0 {
                    let start = dungeon_info.base_offset as usize;
                    let end = start + 50_000;
                    regions.push((start, end, format!("dungeon_area_{}", area)));
                }
            }

            // Dungeon flags can be in multiple areas - search all known dungeon regions
            // The old DUNGEON_BASE_OFFSETS shows dungeons start around byte 4112
            // Search the entire early region where dungeon flags might be
            regions.push((4000, 60000, "all_dungeon_regions".to_string()));

            // Specifically for Stormveil (area 10)
            if area == 10 {
                // Stormveil base is 4112, boss flag 10000800 should be at:
                // 4112 + section*1125 + 800/8 = 4112 + 0*1125 + 100 = 4212
                // But that's not working, so let's search wider
                regions.push((4000, 6000, "stormveil_region".to_string()));

                // Also search the section-specific area
                let section_start = 4112 + section as usize * 1125;
                let section_end = section_start + 1125;
                regions.push((section_start, section_end, format!("stormveil_section_{}", section)));
            }
        } else if flag_id >= 60000 && flag_id < 100000 {
            // Block flag
            let block_start = (flag_id / 1000) * 1000;

            if let Some(block_info) = VERIFIED_BLOCK_BASES.get(&block_start) {
                let start = block_info.base_offset as usize;
                let end = start + 200; // Block is ~125 bytes
                regions.push((start, end, format!("block_{}", block_start)));
            }

            // Search adjacent blocks too
            for adj_block in [block_start.saturating_sub(1000), block_start + 1000] {
                if let Some(adj_info) = VERIFIED_BLOCK_BASES.get(&adj_block) {
                    let start = adj_info.base_offset as usize;
                    let end = start + 200;
                    regions.push((start, end, format!("adjacent_block_{}", adj_block)));
                }
            }

            // For cookbook flags (67xxx, 68xxx), search the broader region
            if block_start >= 67000 && block_start <= 69000 {
                regions.push((3500, 4000, "cookbook_region".to_string()));
            }
        } else if flag_id < 60000 {
            // Simple flag - direct calculation should work
            let expected_byte = (flag_id / 8) as usize;
            let start = expected_byte.saturating_sub(100);
            let end = expected_byte + 100;
            regions.push((start, end, "simple_flag_region".to_string()));
        }

        regions
    }

    /// Reverse calculate what flag ID a (byte, bit) position would represent
    fn reverse_calculate_flag_id(
        &self,
        original_flag_id: u32,
        byte_offset: usize,
        bit_position: u8,
        _calculated_offset: Option<(u32, u8)>,
    ) -> Option<u32> {
        // Try to reverse based on flag type

        // Simple flag reverse
        if original_flag_id < 60000 {
            let flag_id = (byte_offset * 8 + (7 - bit_position as usize)) as u32;
            if flag_id < 60000 {
                return Some(flag_id);
            }
        }

        // Block flag reverse
        if original_flag_id >= 60000 && original_flag_id < 100000 {
            let target_block = (original_flag_id / 1000) * 1000;

            // Check if byte is in any known block
            for (&block_start, block_info) in VERIFIED_BLOCK_BASES.iter() {
                let block_base = block_info.base_offset as usize;
                let block_end = block_base + 125;

                if byte_offset >= block_base && byte_offset < block_end {
                    let relative_byte = byte_offset - block_base;
                    let flag_id = block_start + (relative_byte * 8 + (7 - bit_position as usize)) as u32;
                    return Some(flag_id);
                }
            }
        }

        // Dungeon flag reverse
        if original_flag_id >= 10_000_000 && original_flag_id < 44_000_000 {
            let area = original_flag_id / 1_000_000;

            if let Some(dungeon_info) = VERIFIED_DUNGEON_BASES.get(&area) {
                if dungeon_info.base_offset > 0 {
                    let base = dungeon_info.base_offset as usize;
                    let section_size = dungeon_info.section_size as usize;

                    if byte_offset >= base {
                        let relative = byte_offset - base;
                        let section = relative / section_size;
                        let local_byte = relative % section_size;
                        let local_id = local_byte * 8 + (7 - bit_position as usize);

                        let flag_id = area * 1_000_000 + (section as u32) * 10_000 + local_id as u32;
                        return Some(flag_id);
                    }
                }
            }
        }

        None
    }

    /// Score a candidate based on various heuristics
    fn score_candidate(
        &self,
        original_flag_id: u32,
        byte_offset: usize,
        bit_position: u8,
        calculated_offset: Option<(u32, u8)>,
        reverse_id: Option<u32>,
        region_name: &str,
    ) -> f64 {
        let mut score: f64 = 0.5; // Base score

        // Bonus if reverse ID matches original
        if let Some(rev_id) = reverse_id {
            if rev_id == original_flag_id {
                score += 0.4; // Perfect match!
            } else if self.flag_ids_similar(original_flag_id, rev_id) {
                score += 0.2; // Similar range
            }
        }

        // Bonus if close to calculated offset
        if let Some((calc_byte, calc_bit)) = calculated_offset {
            let byte_dist = (byte_offset as i64 - calc_byte as i64).unsigned_abs() as usize;

            if byte_dist == 0 && bit_position == calc_bit {
                score += 0.3; // Same position (shouldn't happen if we're probing)
            } else if byte_dist < 10 {
                score += 0.2; // Very close
            } else if byte_dist < 50 {
                score += 0.1; // Somewhat close
            }
        }

        // Bonus for being in expected region type
        if region_name.contains("calculated_vicinity") {
            score += 0.1;
        }

        // Special handling for dungeon boss flags (ending in 800)
        if original_flag_id >= 10_000_000 && original_flag_id < 44_000_000 {
            let local_id = original_flag_id % 10_000;
            if local_id == 800 && region_name.contains("dungeon") {
                // Boss defeat flags are important - boost if in dungeon region
                score += 0.15;
            }
        }

        // Special handling for cookbook flags
        if original_flag_id >= 67000 && original_flag_id < 69000 {
            if region_name.contains("cookbook") || region_name.contains("block_67") {
                score += 0.15;
            }
        }

        score.min(1.0)
    }

    /// Check if two flag IDs are in similar ranges
    fn flag_ids_similar(&self, id1: u32, id2: u32) -> bool {
        // Same block
        if id1 / 1000 == id2 / 1000 {
            return true;
        }

        // Same dungeon area
        if id1 >= 10_000_000 && id2 >= 10_000_000 {
            if id1 / 1_000_000 == id2 / 1_000_000 {
                return true;
            }
        }

        // Within 1000 of each other
        (id1 as i64 - id2 as i64).unsigned_abs() < 1000
    }

    /// Probe multiple flags and return all results
    pub fn probe_multiple(
        &self,
        event_flags: &[u8],
        flags: &[(u32, &str, bool)],
    ) -> Vec<ProbeResult> {
        flags.iter()
            .map(|(id, name, expected)| self.probe_flag(event_flags, *id, name, *expected))
            .collect()
    }
}

impl Default for OffsetProber {
    fn default() -> Self {
        Self::new(ProbeConfig::default())
    }
}

/// Run automated probing on failing verification flags
pub fn probe_failing_flags(
    event_flags: &[u8],
    failing_flags: &[(u32, &str, bool, bool)], // (id, name, expected, actual)
) -> Vec<ProbeResult> {
    // Use a lower confidence threshold for probing
    let config = ProbeConfig {
        search_radius: 1000, // Wider search
        min_confidence: 0.6, // Lower threshold
        cross_slot_validation: false,
    };
    let prober = OffsetProber::new(config);

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║              AUTOMATED OFFSET PROBING                        ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    let mut results = Vec::new();

    for (flag_id, name, expected, _actual) in failing_flags {
        println!("║ Probing: {} ({})...", name, flag_id);

        let result = prober.probe_flag(event_flags, *flag_id, name, *expected);

        println!("║   Calculated offset: {:?}", result.calculated_offset);
        println!("║   Candidates found: {}", result.candidates.len());

        // Show top 3 candidates regardless of confidence
        if !result.candidates.is_empty() {
            println!("║   Top candidates:");
            for (i, cand) in result.candidates.iter().take(3).enumerate() {
                let rev_str = cand.reverse_flag_id
                    .map(|id| format!(" (→{})", id))
                    .unwrap_or_default();
                println!("║     {}. byte 0x{:06x}, bit {} [{:.0}%]{} - {}",
                    i + 1,
                    cand.byte_offset,
                    cand.bit_position,
                    cand.confidence * 100.0,
                    rev_str,
                    cand.reason);
            }
        }

        if let Some(ref best) = result.best_candidate {
            println!("║   SELECTED: byte 0x{:06x}, bit {} (confidence: {:.0}%)",
                best.byte_offset, best.bit_position, best.confidence * 100.0);
        } else {
            println!("║   No candidate met confidence threshold");
        }

        if let Some(ref correction) = result.correction {
            println!("║   CORRECTION: ({}, {}) → (0x{:06x}, {})",
                correction.old_offset.0, correction.old_offset.1,
                correction.new_offset.0, correction.new_offset.1);
        }

        println!("║");
        results.push(result);
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    // Summary
    let corrections_found = results.iter().filter(|r| r.correction.is_some()).count();
    println!("\nProbing complete: {}/{} corrections found", corrections_found, results.len());

    results
}

/// Generate ground truth updates from probe results
pub fn generate_ground_truth_updates(results: &[ProbeResult]) -> String {
    let mut updates = String::new();
    updates.push_str("// Suggested ground_truth_offsets.json updates:\n\n");

    for result in results {
        if let Some(ref correction) = result.correction {
            updates.push_str(&format!(
                "Flag {}: byte {} bit {} → byte 0x{:x} bit {} (confidence: {:.0}%)\n",
                result.flag_id,
                correction.old_offset.0,
                correction.old_offset.1,
                correction.new_offset.0,
                correction.new_offset.1,
                correction.confidence * 100.0
            ));

            // Suggest block base correction if applicable
            if result.flag_id >= 60000 && result.flag_id < 100000 {
                let block_start = (result.flag_id / 1000) * 1000;
                let relative = result.flag_id - block_start;
                let suggested_base = correction.new_offset.0 - (relative / 8) as usize;

                updates.push_str(&format!(
                    "  → Block {} suggested base: {}\n",
                    block_start, suggested_base
                ));
            }
        }
    }

    updates
}

// ============================================================================
// PERSISTENCE HOOKS
// ============================================================================

/// Convert a probe result into an observation for the discovery store
fn probe_result_to_observation(
    result: &ProbeResult,
    slot_index: Option<usize>,
    character_name: Option<String>,
    config: &ProbeConfig,
) -> Option<OffsetObservation> {
    let best = result.best_candidate.as_ref()?;

    Some(OffsetObservation::new(
        best.byte_offset,
        best.bit_position,
        ObservationSource::ProbeResult {
            search_radius: config.search_radius,
            confidence: best.confidence,
            probe_method: best.reason.clone(),
        },
        slot_index,
        character_name,
        best.confidence,
    ))
}

/// Run probing and persist results to the discovery store
pub fn probe_and_persist(
    event_flags: &[u8],
    failing_flags: &[(u32, &str, bool, bool)],
    store: &mut DiscoveryStore,
    slot_index: Option<usize>,
    character_name: Option<String>,
) -> Vec<ProbeResult> {
    let config = ProbeConfig {
        search_radius: 1000,
        min_confidence: 0.6,
        cross_slot_validation: false,
    };
    let prober = OffsetProber::new(config.clone());

    let mut results = Vec::new();
    let mut persisted_count = 0;

    for (flag_id, name, expected, _actual) in failing_flags {
        let result = prober.probe_flag(event_flags, *flag_id, name, *expected);

        // Persist if we found a candidate
        if let Some(observation) = probe_result_to_observation(
            &result,
            slot_index,
            character_name.clone(),
            &config,
        ) {
            store.add_observation_with_metadata(
                *flag_id,
                Some(name.to_string()),
                categorize_flag(*flag_id),
                observation,
            );
            persisted_count += 1;
        }

        results.push(result);
    }

    println!("Persisted {} probe observations to discovery store", persisted_count);
    results
}

/// Persist results from an already-completed probe run
pub fn persist_probe_results(
    results: &[ProbeResult],
    store: &mut DiscoveryStore,
    slot_index: Option<usize>,
    character_name: Option<String>,
    config: &ProbeConfig,
) -> usize {
    let mut persisted = 0;

    for result in results {
        if let Some(observation) = probe_result_to_observation(
            result,
            slot_index,
            character_name.clone(),
            config,
        ) {
            store.add_observation_with_metadata(
                result.flag_id,
                Some(result.flag_name.clone()),
                categorize_flag(result.flag_id),
                observation,
            );
            persisted += 1;
        }
    }

    persisted
}

/// Categorize a flag ID into a human-readable category
fn categorize_flag(flag_id: u32) -> Option<String> {
    Some(match flag_id {
        0..=59_999 => "Simple Flag".to_string(),
        60_000..=61_999 => "Progression".to_string(),
        62_000..=62_999 => "Map Fragment".to_string(),
        65_000..=65_999 => "Whetblade".to_string(),
        66_000..=66_999 => "Container Upgrade".to_string(),
        67_000..=68_999 => "Cookbook".to_string(),
        71_000..=77_999 => "Grace".to_string(),
        78_000..=99_999 => "Landmark".to_string(),
        10_000_000..=19_999_999 => "Stormveil/Leyndell".to_string(),
        30_000_000..=30_999_999 => "Catacomb".to_string(),
        31_000_000..=31_999_999 => "Cave".to_string(),
        32_000_000..=32_999_999 => "Tunnel".to_string(),
        1_000_000_000..=u32::MAX => "World Pickup".to_string(),
        _ => "Unknown".to_string(),
    })
}

/// Load, update, and save the discovery store in one operation
pub fn probe_and_save(
    event_flags: &[u8],
    failing_flags: &[(u32, &str, bool, bool)],
    store_path: &Path,
    slot_index: Option<usize>,
    character_name: Option<String>,
) -> Result<(Vec<ProbeResult>, usize), StoreError> {
    // Load or create the store
    let mut store = DiscoveryStore::load_or_create(store_path)?;

    // Run probing and persist
    let results = probe_and_persist(
        event_flags,
        failing_flags,
        &mut store,
        slot_index,
        character_name,
    );

    let persisted = results.iter().filter(|r| r.best_candidate.is_some()).count();

    // Save the store
    store.save(store_path)?;

    println!("Discovery store saved to {:?} ({} total discoveries)",
        store_path, store.len());

    Ok((results, persisted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prober_creation() {
        let prober = OffsetProber::default();
        assert_eq!(prober.config.search_radius, 500);
    }

    #[test]
    fn test_flag_similarity() {
        let prober = OffsetProber::default();

        // Same block
        assert!(prober.flag_ids_similar(76100, 76150));

        // Same dungeon area
        assert!(prober.flag_ids_similar(10000800, 10001000));

        // Different
        assert!(!prober.flag_ids_similar(76100, 67000));
    }
}
