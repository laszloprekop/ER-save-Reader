//! JSONL verification records loader
//!
//! Loads manually logged verification records from external JSONL file
//! for comparison with auto-detected flag values.

use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use crate::db::pickup_flags::get_flag_offset;

/// A verification record from the JSONL file
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRecord {
    pub id: String,
    pub slot_index: u32,
    pub character_name: String,
    #[serde(default)]
    pub character_level: u32,
    pub flag_id: u32,
    pub flag_name: String,
    pub flag_category: String,
    pub flag_region: String,
    pub flag_type: String,
    pub computed_byte_offset: i32,
    pub computed_bit_position: i32,
    pub manual_status: bool,
    pub auto_status: bool,
    pub matches: bool,
}

/// Load verification records from a JSONL file
pub fn load_verification_records(path: &Path) -> Result<Vec<VerificationRecord>, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for line in reader.lines().flatten() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str(&line) {
            records.push(record);
        }
    }
    Ok(records)
}

/// Get records for a specific slot index
pub fn get_records_for_slot(records: &[VerificationRecord], slot_index: u32) -> Vec<VerificationRecord> {
    records.iter()
        .filter(|r| r.slot_index == slot_index)
        .cloned()
        .collect()
}

/// Re-compute auto_status for records based on actual save data
///
/// Uses the CURRENT formula (get_flag_offset) instead of stored offsets,
/// ensuring the comparison reflects the latest offset calculations.
pub fn recompute_auto_status(
    records: &mut [VerificationRecord],
    event_flags: &[u8],
) {
    for record in records {
        // Use current formula instead of stored offsets
        if let Some((byte_offset, bit_position)) = get_flag_offset(record.flag_id) {
            let offset = byte_offset as usize;
            if offset < event_flags.len() {
                let byte = event_flags[offset];
                let is_set = (byte >> bit_position) & 1 == 1;
                record.auto_status = is_set;
                // Update stored offsets to reflect current formula
                record.computed_byte_offset = byte_offset as i32;
                record.computed_bit_position = bit_position as i32;
                record.matches = record.manual_status == record.auto_status;
            }
        } else {
            // No formula for this flag - mark as not computable
            record.computed_byte_offset = -1;
            record.computed_bit_position = -1;
            record.auto_status = false;
            record.matches = !record.manual_status; // Matches only if manual is also false
        }
    }
}
