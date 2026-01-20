/// Integration module for running discovery workflows
///
/// This module provides high-level functions to run the discovery process
/// on actual save files and integrate with the verification system.

use std::path::{Path, PathBuf};

use crate::save::save::save::Save;
use crate::db::pickup_flags::{is_flag_set, get_flag_offset};
use crate::util::verification::{
    get_confessor_known_flags, get_wretch_known_flags,
    verify_flag_formula, KnownFlag,
};

use super::byte_diff::{ByteDiffScanner, find_changed_regions};
use super::segment_analysis::SegmentAnalyzer;
use super::reverse_lookup::FlagReverser;
use super::discovery_report::DiscoveryEngine;
use super::offset_probe::{probe_failing_flags, generate_ground_truth_updates};
use super::discovery_store::{
    DiscoveryStore, OffsetObservation, ObservationSource, StoreError,
};
use super::flag_catalog::FlagCatalog;

/// Run full discovery workflow on a save file slot
pub fn run_discovery_workflow(
    save_path: &Path,
    slot_index: usize,
) -> Result<DiscoveryWorkflowResult, String> {
    // Load save file
    let save = Save::from_path(&save_path.to_path_buf())
        .map_err(|e| format!("Failed to load save: {}", e))?;

    let slot = save.save_type.get_slot(slot_index);
    let event_flags = &slot.event_flags.flags;

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║              DISCOVERY WORKFLOW - Slot {}                     ║", slot_index);
    println!("╠══════════════════════════════════════════════════════════════╣");

    // Step 1: Segment Analysis
    println!("║ Step 1: Analyzing event flags structure...                   ║");
    let analyzer = SegmentAnalyzer::new().with_block_size(4096);
    let segment_result = analyzer.analyze(event_flags);
    segment_result.print_summary();

    // Step 2: Current Formula Verification
    println!("\n║ Step 2: Verifying current formulas against known flags...    ║");
    let known_flags = if slot_index == 0 {
        get_confessor_known_flags()
    } else {
        get_wretch_known_flags()
    };
    let verification_report = verify_flag_formula(event_flags, &known_flags);
    verification_report.print_summary();

    // Step 3: Identify problematic regions
    println!("\n║ Step 3: Identifying problematic regions...                   ║");
    let reverser = FlagReverser::new();

    let mut failed_flags = Vec::new();
    for result in &verification_report.results {
        if !result.passed {
            failed_flags.push((result.flag_id, result.name, result.expected, result.actual));
        }
    }

    if !failed_flags.is_empty() {
        println!("Found {} flags that don't match expected values:", failed_flags.len());
        for (flag_id, name, expected, actual) in &failed_flags {
            let offset = get_flag_offset(*flag_id);
            println!("  - {} ({}): expected {}, got {} at {:?}",
                name, flag_id, expected, actual, offset);
        }
    } else {
        println!("All known flags verified correctly!");
    }

    // Step 4: Probe for undiscovered flag regions
    println!("\n║ Step 4: Probing for undiscovered flag regions...             ║");
    let potential_regions = segment_result.find_potential_flag_regions();
    if !potential_regions.is_empty() {
        println!("Found {} potential flag regions:", potential_regions.len());
        for (start, end, description) in potential_regions.iter().take(10) {
            println!("  0x{:06x} - 0x{:06x}: {}", start, end, description);
        }
        if potential_regions.len() > 10 {
            println!("  ... and {} more", potential_regions.len() - 10);
        }
    }

    // Step 5: Analyze formula coverage
    println!("\n║ Step 5: Analyzing formula coverage...                        ║");
    let coverage = analyze_formula_coverage(event_flags, &reverser);

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(DiscoveryWorkflowResult {
        segments_found: segment_result.segments.len(),
        flag_bytes: segment_result.flag_bytes(),
        empty_bytes: segment_result.empty_bytes(),
        verification_passed: verification_report.passed,
        verification_failed: verification_report.failed,
        failed_flags,
        coverage,
    })
}

/// Analyze how well current formulas cover the actual flag data
fn analyze_formula_coverage(event_flags: &[u8], reverser: &FlagReverser) -> FormulaCoverage {
    let mut covered = 0;
    let mut uncovered = 0;
    let mut uncovered_sample = Vec::new();

    // Sample set bits in the event flags
    for (byte_idx, byte) in event_flags.iter().enumerate() {
        if *byte == 0 {
            continue;
        }

        for bit in 0..8 {
            if (*byte & (1 << bit)) != 0 {
                let bit_pos = 7 - bit;
                let possibilities = reverser.reverse_lookup(byte_idx, bit_pos);

                let has_known = possibilities.iter().any(|p| p.flag_id().is_some());

                if has_known {
                    covered += 1;
                } else {
                    uncovered += 1;
                    if uncovered_sample.len() < 20 {
                        uncovered_sample.push((byte_idx, bit_pos as u8));
                    }
                }
            }
        }
    }

    let total = covered + uncovered;
    let coverage_pct = if total > 0 {
        (covered as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    println!("Formula coverage: {:.1}% ({} of {} set bits mapped)",
        coverage_pct, covered, total);

    if !uncovered_sample.is_empty() {
        println!("Sample uncovered positions:");
        for (byte, bit) in uncovered_sample.iter().take(10) {
            println!("  byte 0x{:06x}, bit {}", byte, bit);
        }
    }

    FormulaCoverage {
        total_set_bits: total,
        covered_bits: covered,
        uncovered_bits: uncovered,
        coverage_percent: coverage_pct,
    }
}

/// Compare two saves and analyze what changed
pub fn run_differential_discovery(
    before_path: &Path,
    after_path: &Path,
    slot_index: usize,
) -> Result<DifferentialResult, String> {
    let before_save = Save::from_path(&before_path.to_path_buf())
        .map_err(|e| format!("Failed to load before save: {}", e))?;
    let after_save = Save::from_path(&after_path.to_path_buf())
        .map_err(|e| format!("Failed to load after save: {}", e))?;

    let before_flags = &before_save.save_type.get_slot(slot_index).event_flags.flags;
    let after_flags = &after_save.save_type.get_slot(slot_index).event_flags.flags;

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║          DIFFERENTIAL DISCOVERY - Slot {}                     ║", slot_index);
    println!("╠══════════════════════════════════════════════════════════════╣");

    // Find changed regions
    let changed_regions = find_changed_regions(before_flags, after_flags, 256);

    println!("Found {} changed regions:", changed_regions.len());
    for (start, end, bit_changes) in changed_regions.iter().take(20) {
        println!("  0x{:06x} - 0x{:06x}: {} bit changes", start, end, bit_changes);
    }

    // Detailed analysis of changes
    let scanner = ByteDiffScanner::new();
    let diff_result = scanner.scan(before_flags, after_flags);

    println!("\nTotal bit changes: {}", diff_result.bit_changes.len());
    println!("Bits set: {}", diff_result.stats.bits_set);
    println!("Bits cleared: {}", diff_result.stats.bits_cleared);

    if let (Some(first), Some(last)) = (diff_result.stats.first_change_offset, diff_result.stats.last_change_offset) {
        println!("Change range: 0x{:06x} - 0x{:06x}", first, last);
    }

    // Reverse lookup the changes
    let reverser = FlagReverser::new();
    let mut interpreted_changes = Vec::new();

    for change in &diff_result.bit_changes {
        let possibles = reverser.reverse_lookup(change.byte_offset, change.bit_position);
        interpreted_changes.push((change.clone(), possibles));
    }

    println!("\nInterpreted changes:");
    for (change, possibles) in interpreted_changes.iter().take(20) {
        let action = if change.was_set() { "SET" } else { "CLEAR" };
        let interpretation = if let Some(p) = possibles.first() {
            format!("{:?}", p)
        } else {
            "unknown".to_string()
        };
        println!("  {} @ 0x{:06x}:{} -> {}", action, change.byte_offset, change.bit_position, interpretation);
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(DifferentialResult {
        total_changes: diff_result.bit_changes.len(),
        bits_set: diff_result.stats.bits_set,
        bits_cleared: diff_result.stats.bits_cleared,
        changed_regions: changed_regions.len(),
    })
}

/// Result of discovery workflow
#[derive(Debug)]
pub struct DiscoveryWorkflowResult {
    pub segments_found: usize,
    pub flag_bytes: usize,
    pub empty_bytes: usize,
    pub verification_passed: usize,
    pub verification_failed: usize,
    pub failed_flags: Vec<(u32, &'static str, bool, bool)>,
    pub coverage: FormulaCoverage,
}

/// Formula coverage analysis
#[derive(Debug)]
pub struct FormulaCoverage {
    pub total_set_bits: usize,
    pub covered_bits: usize,
    pub uncovered_bits: usize,
    pub coverage_percent: f64,
}

/// Result of differential discovery
#[derive(Debug)]
pub struct DifferentialResult {
    pub total_changes: usize,
    pub bits_set: usize,
    pub bits_cleared: usize,
    pub changed_regions: usize,
}

/// Run automated probing for failing flags on a save file
pub fn run_offset_probing(
    save_path: &Path,
    slot_index: usize,
) -> Result<Vec<super::offset_probe::ProbeResult>, String> {
    let save = Save::from_path(&save_path.to_path_buf())
        .map_err(|e| format!("Failed to load save: {}", e))?;

    let slot = save.save_type.get_slot(slot_index);
    let event_flags = &slot.event_flags.flags;

    // First, run verification to find failing flags
    let known_flags = if slot_index == 0 {
        get_confessor_known_flags()
    } else {
        get_wretch_known_flags()
    };

    let verification_report = verify_flag_formula(event_flags, &known_flags);

    // Collect failing flags
    let failing_flags: Vec<(u32, &str, bool, bool)> = verification_report.results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| (r.flag_id, r.name, r.expected, r.actual))
        .collect();

    if failing_flags.is_empty() {
        println!("All flags verified correctly - no probing needed!");
        return Ok(Vec::new());
    }

    println!("\nFound {} failing flags to probe:", failing_flags.len());
    for (id, name, expected, actual) in &failing_flags {
        println!("  - {} ({}): expected {}, got {}", name, id, expected, actual);
    }

    // Run probing
    let results = probe_failing_flags(event_flags, &failing_flags);

    // Generate suggested updates
    let updates = generate_ground_truth_updates(&results);
    println!("\n{}", updates);

    Ok(results)
}

// ============================================================================
// PERSISTENCE-ENABLED WORKFLOWS
// ============================================================================

/// Run differential discovery and persist observations to the store
pub fn run_differential_discovery_with_persistence(
    before_path: &Path,
    after_path: &Path,
    slot_index: usize,
    store: &mut DiscoveryStore,
    action_description: Option<&str>,
    catalog: Option<&FlagCatalog>,
) -> Result<DifferentialResult, String> {
    let before_save = Save::from_path(&before_path.to_path_buf())
        .map_err(|e| format!("Failed to load before save: {}", e))?;
    let after_save = Save::from_path(&after_path.to_path_buf())
        .map_err(|e| format!("Failed to load after save: {}", e))?;

    let before_flags = &before_save.save_type.get_slot(slot_index).event_flags.flags;
    let after_flags = &after_save.save_type.get_slot(slot_index).event_flags.flags;

    // Detailed analysis of changes
    let scanner = ByteDiffScanner::new();
    let diff_result = scanner.scan(before_flags, after_flags);

    // Reverse lookup the changes and persist observations
    let reverser = FlagReverser::new();
    let mut persisted_count = 0;

    let action_desc = action_description
        .map(|s| s.to_string())
        .unwrap_or_else(|| extract_action_from_filename(after_path));

    let before_filename = before_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let after_filename = after_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    for change in &diff_result.bit_changes {
        // Only persist bits that were SET (new flags activated)
        if !change.was_set() {
            continue;
        }

        let possibles = reverser.reverse_lookup(change.byte_offset, change.bit_position);

        // If we can identify a specific flag, persist the observation
        for possible in &possibles {
            if let Some(flag_id) = possible.flag_id() {
                let observation = OffsetObservation::new(
                    change.byte_offset,
                    change.bit_position,
                    ObservationSource::SnapshotDiff {
                        before_file: before_filename.clone(),
                        after_file: after_filename.clone(),
                        action_description: action_desc.clone(),
                    },
                    Some(slot_index),
                    get_character_name(slot_index),
                    0.8, // High confidence for snapshot diffs
                );

                // Look up flag name from catalog, or generate from ID pattern
                let flag_name = catalog
                    .map(|c| c.get_name_or_generate(flag_id));

                store.add_observation_with_metadata(
                    flag_id,
                    flag_name,
                    categorize_flag_id(flag_id),
                    observation,
                );
                persisted_count += 1;
                break; // Only persist once per change
            }
        }
    }

    println!("Persisted {} observations from differential discovery", persisted_count);

    Ok(DifferentialResult {
        total_changes: diff_result.bit_changes.len(),
        bits_set: diff_result.stats.bits_set,
        bits_cleared: diff_result.stats.bits_cleared,
        changed_regions: 0, // Not computing regions in this version
    })
}

/// Run probing with persistence
pub fn run_offset_probing_with_persistence(
    save_path: &Path,
    slot_index: usize,
    store: &mut DiscoveryStore,
) -> Result<Vec<super::offset_probe::ProbeResult>, String> {
    let save = Save::from_path(&save_path.to_path_buf())
        .map_err(|e| format!("Failed to load save: {}", e))?;

    let slot = save.save_type.get_slot(slot_index);
    let event_flags = &slot.event_flags.flags;

    // Run verification to find failing flags
    let known_flags = if slot_index == 0 {
        get_confessor_known_flags()
    } else {
        get_wretch_known_flags()
    };

    let verification_report = verify_flag_formula(event_flags, &known_flags);

    let failing_flags: Vec<(u32, &str, bool, bool)> = verification_report.results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| (r.flag_id, r.name, r.expected, r.actual))
        .collect();

    if failing_flags.is_empty() {
        println!("All flags verified correctly - no probing needed!");
        return Ok(Vec::new());
    }

    // Use probe_and_persist from offset_probe module
    let results = super::offset_probe::probe_and_persist(
        event_flags,
        &failing_flags,
        store,
        Some(slot_index),
        get_character_name(slot_index),
    );

    // Generate suggested updates (still print for manual review)
    let updates = generate_ground_truth_updates(&results);
    println!("\n{}", updates);

    Ok(results)
}

/// Extract action description from snapshot filename
fn extract_action_from_filename(path: &Path) -> String {
    let filename = path.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Parse patterns like "Confessor - 09 before picking up Smoldering Butterfly"
    if let Some(idx) = filename.find(" - ") {
        let rest = &filename[idx + 3..];
        if let Some(after_idx) = rest.find("after ") {
            return rest[after_idx + 6..].trim_end_matches(".sl2").to_string();
        }
        if let Some(before_idx) = rest.find("before ") {
            return rest[before_idx + 7..].trim_end_matches(".sl2").to_string();
        }
    }

    filename
}

/// Get character name for a slot index
fn get_character_name(slot_index: usize) -> Option<String> {
    match slot_index {
        0 => Some("Confessor".to_string()),
        1 => Some("Wretch".to_string()),
        2 => Some("V1".to_string()),
        3 => Some("V2".to_string()),
        4 => Some("V3".to_string()),
        5 => Some("Sam".to_string()),
        _ => None,
    }
}

/// Categorize a flag ID
fn categorize_flag_id(flag_id: u32) -> Option<String> {
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

/// Convenience function: Load store, run differential, save store
pub fn differential_discovery_and_save(
    before_path: &Path,
    after_path: &Path,
    slot_index: usize,
    store_path: &Path,
    action_description: Option<&str>,
) -> Result<DifferentialResult, String> {
    let mut store = DiscoveryStore::load_or_create(store_path)
        .map_err(|e| format!("Failed to load store: {}", e))?;

    // Load flag catalog for name lookups
    let catalog = FlagCatalog::load_default().ok();

    let result = run_differential_discovery_with_persistence(
        before_path,
        after_path,
        slot_index,
        &mut store,
        action_description,
        catalog.as_ref(),
    )?;

    store.save(store_path)
        .map_err(|e| format!("Failed to save store: {}", e))?;

    println!("Discovery store saved ({} total discoveries)", store.len());

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests require actual save files
    // These are marked as ignore by default
    #[test]
    #[ignore]
    fn test_discovery_workflow() {
        // Try to find a save file in the standard location
        let save_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2");
        if save_path.exists() {
            let result = run_discovery_workflow(save_path, 0);
            match result {
                Ok(r) => {
                    println!("\nDiscovery completed:");
                    println!("  Segments: {}", r.segments_found);
                    println!("  Flag bytes: {}", r.flag_bytes);
                    println!("  Verification: {}/{}", r.verification_passed, r.verification_passed + r.verification_failed);
                    println!("  Coverage: {:.1}%", r.coverage.coverage_percent);
                }
                Err(e) => {
                    panic!("Discovery failed: {}", e);
                }
            }
        } else {
            println!("Save file not found at {:?}, skipping test", save_path);
        }
    }

    #[test]
    #[ignore]
    fn test_differential_discovery() {
        let before_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000-older.sl2");
        let after_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2");

        if before_path.exists() && after_path.exists() {
            let result = run_differential_discovery(before_path, after_path, 0);
            match result {
                Ok(r) => {
                    println!("\nDifferential discovery completed:");
                    println!("  Total changes: {}", r.total_changes);
                    println!("  Bits set: {}", r.bits_set);
                    println!("  Bits cleared: {}", r.bits_cleared);
                    println!("  Changed regions: {}", r.changed_regions);
                }
                Err(e) => {
                    panic!("Differential discovery failed: {}", e);
                }
            }
        } else {
            println!("Save files not found, skipping test");
        }
    }

    #[test]
    #[ignore]
    fn test_offset_probing() {
        let save_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2");

        if save_path.exists() {
            let results = run_offset_probing(save_path, 0);
            match results {
                Ok(probe_results) => {
                    println!("\n=== PROBING SUMMARY ===");
                    let corrections = probe_results.iter()
                        .filter(|r| r.correction.is_some())
                        .count();
                    println!("Corrections found: {}/{}", corrections, probe_results.len());

                    for result in &probe_results {
                        if let Some(ref corr) = result.correction {
                            println!("\n{} ({}):", result.flag_name, result.flag_id);
                            println!("  OLD: byte {}, bit {}", corr.old_offset.0, corr.old_offset.1);
                            println!("  NEW: byte 0x{:x} ({}), bit {}",
                                corr.new_offset.0, corr.new_offset.0, corr.new_offset.1);
                            println!("  Confidence: {:.0}%", corr.confidence * 100.0);
                        }
                    }
                }
                Err(e) => {
                    panic!("Offset probing failed: {}", e);
                }
            }
        } else {
            println!("Save file not found, skipping test");
        }
    }
}
