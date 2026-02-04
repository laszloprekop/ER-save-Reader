/// Ground Truth Probe Module
///
/// Uses manually logged completion records to probe for correct offsets.
/// This module reads the JSONL file of verified completions and probes
/// for the correct offsets of flags that don't match.

use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};

use serde::Deserialize;

use crate::save::save::save::Save;
use crate::db::pickup_flags::{is_flag_set, get_flag_offset};

use super::offset_probe::{OffsetProber, ProbeConfig, ProbeResult};

/// A verification record from the manually logged completions
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRecord {
    pub id: String,
    pub slot_index: usize,
    pub character_name: String,
    pub flag_id: u32,
    pub flag_name: String,
    pub flag_category: String,
    pub flag_region: String,
    pub flag_type: String,
    pub computed_byte_offset: i32,
    pub computed_bit_position: i8,
    #[serde(alias = "manualStatus")]
    pub user_marked_complete: bool,
    #[serde(alias = "autoStatus")]
    pub webapp_parsed_status: bool,
    #[serde(alias = "matches")]
    pub statuses_align: bool,
}

/// Load verification records from JSONL file
pub fn load_verification_records(path: &Path) -> Result<Vec<VerificationRecord>, String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open verification records: {}", e))?;

    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("Failed to read line {}: {}", line_num, e))?;
        let record: VerificationRecord = serde_json::from_str(&line)
            .map_err(|e| format!("Failed to parse line {}: {}", line_num, e))?;
        records.push(record);
    }

    Ok(records)
}

/// Probe all mismatched flags from verification records
pub fn probe_from_verification_records(
    save_path: &Path,
    records_path: &Path,
    slot_index: usize,
) -> Result<GroundTruthProbeReport, String> {
    // Load save file
    let save = Save::from_path(&save_path.to_path_buf())
        .map_err(|e| format!("Failed to load save: {}", e))?;

    let slot = save.save_type.get_slot(slot_index);
    let event_flags = &slot.event_flags.flags;

    // Load verification records
    let all_records = load_verification_records(records_path)?;

    // Filter to this slot's mismatched records where manual=true
    let mismatched: Vec<_> = all_records.iter()
        .filter(|r| r.slot_index == slot_index && r.user_marked_complete && !r.statuses_align)
        .collect();

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║        GROUND TRUTH PROBE - Slot {} ({} records)              ║",
        slot_index, mismatched.len());
    println!("╠══════════════════════════════════════════════════════════════╣");

    // Group by flag type for organized probing
    let mut by_type: std::collections::HashMap<&str, Vec<&VerificationRecord>> =
        std::collections::HashMap::new();
    for record in &mismatched {
        by_type.entry(&record.flag_type).or_default().push(record);
    }

    // Configure prober
    let config = ProbeConfig {
        search_radius: 2000,
        min_confidence: 0.5,
        cross_slot_validation: false,
    };
    let prober = OffsetProber::new(config);

    let mut results = Vec::new();
    let mut corrections_by_type: std::collections::HashMap<String, Vec<ProbeResult>> =
        std::collections::HashMap::new();

    for (flag_type, records) in &by_type {
        println!("║");
        println!("║ === {} flags ({} mismatched) ===", flag_type, records.len());

        for record in records.iter().take(5) { // Probe first 5 of each type
            println!("║ Probing: {} ({})...", record.flag_name, record.flag_id);

            let result = prober.probe_flag(
                event_flags,
                record.flag_id,
                &record.flag_name,
                record.user_marked_complete,
            );

            // Verify against actual save state
            let actual = is_flag_set(event_flags, record.flag_id);
            let _calc_offset = get_flag_offset(record.flag_id);

            println!("║   Computed: byte {}, bit {} (from formula)",
                record.computed_byte_offset, record.computed_bit_position);
            println!("║   Actual value at computed: {}", actual);

            if let Some(ref best) = result.best_candidate {
                println!("║   FOUND: byte 0x{:x} ({}), bit {} [{:.0}%]",
                    best.byte_offset, best.byte_offset, best.bit_position,
                    best.confidence * 100.0);

                // Check if the found position has the expected value
                if best.byte_offset < event_flags.len() {
                    let byte = event_flags[best.byte_offset];
                    let bit_mask = 1 << (7 - best.bit_position);
                    let found_value = (byte & bit_mask) != 0;
                    println!("║   Value at found position: {}", found_value);

                    if found_value == record.user_marked_complete {
                        println!("║   ✓ MATCH! This is likely the correct offset");
                    }
                }
            } else {
                println!("║   No candidate found");
            }

            results.push(result.clone());
            corrections_by_type.entry(flag_type.to_string())
                .or_default()
                .push(result);
        }

        if records.len() > 5 {
            println!("║   ... and {} more {} flags", records.len() - 5, flag_type);
        }
    }

    println!("║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // Summary
    let total_probed = results.len();
    let corrections_found = results.iter().filter(|r| r.best_candidate.is_some()).count();

    println!("\n=== PROBE SUMMARY ===");
    println!("Total mismatched flags: {}", mismatched.len());
    println!("Flags probed: {}", total_probed);
    println!("Potential corrections: {}", corrections_found);

    println!("\nBy type:");
    for (flag_type, type_results) in &corrections_by_type {
        let found = type_results.iter().filter(|r| r.best_candidate.is_some()).count();
        println!("  {}: {}/{} found", flag_type, found, type_results.len());
    }

    Ok(GroundTruthProbeReport {
        total_mismatched: mismatched.len(),
        total_probed: total_probed,
        corrections_found,
        results,
        corrections_by_type,
    })
}

/// Report from ground truth probing
#[derive(Debug)]
pub struct GroundTruthProbeReport {
    pub total_mismatched: usize,
    pub total_probed: usize,
    pub corrections_found: usize,
    pub results: Vec<ProbeResult>,
    pub corrections_by_type: std::collections::HashMap<String, Vec<ProbeResult>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_ground_truth_probe_slot0() {
        let save_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2");
        let records_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/verification-records.jsonl");

        if save_path.exists() && records_path.exists() {
            // Test slot 0 (Confessor - level 93)
            let result = probe_from_verification_records(save_path, records_path, 0);
            match result {
                Ok(report) => {
                    println!("\n=== FINAL REPORT (Slot 0 - Confessor) ===");
                    println!("Corrections found: {}/{}", report.corrections_found, report.total_probed);
                }
                Err(e) => {
                    panic!("Probe failed: {}", e);
                }
            }
        } else {
            println!("Files not found, skipping test");
        }
    }

    #[test]
    #[ignore]
    fn test_ground_truth_probe_slot5() {
        let save_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2");
        let records_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/verification-records.jsonl");

        if save_path.exists() && records_path.exists() {
            // Test slot 5 (Sam - level 10, more detailed verification data)
            let result = probe_from_verification_records(save_path, records_path, 5);
            match result {
                Ok(report) => {
                    println!("\n=== FINAL REPORT (Slot 5 - Sam) ===");
                    println!("Corrections found: {}/{}", report.corrections_found, report.total_probed);
                }
                Err(e) => {
                    panic!("Probe failed: {}", e);
                }
            }
        } else {
            println!("Files not found, skipping test");
        }
    }
}
