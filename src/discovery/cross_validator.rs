/// Cross-Slot Validator
///
/// Validates discovered flag offsets by checking them across multiple save slots.
/// If the same offset/bit position shows consistent values across slots where
/// the flag should be set/unset, confidence increases significantly.
///
/// ## Validation Strategy:
/// - For flags that SHOULD be set: check if bit is 1 in slots where flag is expected
/// - For flags that SHOULD NOT be set: check if bit is 0 in slots where flag is not expected
/// - Agreement across 2+ slots significantly boosts confidence

use std::path::Path;

use crate::save::save::save::Save;
use crate::db::pickup_flags::EVENT_FLAGS_SIZE;

use super::discovery_store::{
    DiscoveryStore, OffsetObservation, ObservationSource,
};

/// Configuration for cross-slot validation
#[derive(Debug, Clone)]
pub struct CrossValidationConfig {
    /// Minimum slots that must agree
    pub min_slots_agree: usize,
    /// Confidence boost when all slots agree
    pub full_agreement_boost: f64,
    /// Confidence reduction when slots disagree
    pub disagreement_penalty: f64,
}

impl Default for CrossValidationConfig {
    fn default() -> Self {
        Self {
            min_slots_agree: 2,
            full_agreement_boost: 0.15,
            disagreement_penalty: 0.3,
        }
    }
}

/// Result of validating a single flag across slots
#[derive(Debug, Clone)]
pub struct CrossValidationResult {
    pub flag_id: u32,
    pub byte_offset: usize,
    pub bit_position: u8,
    pub slots_checked: Vec<SlotCheck>,
    pub all_matched: bool,
    pub match_count: usize,
    pub mismatch_count: usize,
    pub confidence_adjustment: f64,
}

/// Check result for a single slot
#[derive(Debug, Clone)]
pub struct SlotCheck {
    pub slot_index: usize,
    pub character_name: String,
    pub expected_value: Option<bool>,
    pub actual_value: bool,
    pub matched: bool,
}

/// Cross-slot validator
pub struct CrossValidator {
    config: CrossValidationConfig,
    /// Event flags from each slot
    slot_flags: Vec<Option<Vec<u8>>>,
    /// Character names for each slot
    slot_names: Vec<String>,
}

impl CrossValidator {
    /// Create a new validator by loading all slots from a save file
    pub fn from_save(save: &Save) -> Self {
        let mut slot_flags = Vec::new();
        let mut slot_names = Vec::new();

        for i in 0..10 {
            let slot = save.save_type.get_slot(i);
            if slot.event_flags.flags.len() == EVENT_FLAGS_SIZE as usize {
                slot_flags.push(Some(slot.event_flags.flags.clone()));
                slot_names.push(get_slot_name(i));
            } else {
                slot_flags.push(None);
                slot_names.push(format!("Slot {}", i));
            }
        }

        Self {
            config: CrossValidationConfig::default(),
            slot_flags,
            slot_names,
        }
    }

    /// Create from a save file path
    pub fn from_path(save_path: &Path) -> Result<Self, String> {
        let save = Save::from_path(&save_path.to_path_buf())
            .map_err(|e| format!("Failed to load save: {}", e))?;
        Ok(Self::from_save(&save))
    }

    /// Set configuration
    pub fn with_config(mut self, config: CrossValidationConfig) -> Self {
        self.config = config;
        self
    }

    /// Validate a discovered offset across all available slots
    pub fn validate_offset(
        &self,
        flag_id: u32,
        byte_offset: usize,
        bit_position: u8,
        expected_values: &[(usize, bool)], // (slot_index, expected_value)
    ) -> CrossValidationResult {
        let mut slots_checked = Vec::new();
        let mut match_count = 0;
        let mut mismatch_count = 0;

        for (slot_idx, expected) in expected_values {
            if let Some(Some(flags)) = self.slot_flags.get(*slot_idx) {
                if byte_offset < flags.len() {
                    let byte = flags[byte_offset];
                    let actual = (byte & (1 << (7 - bit_position))) != 0;
                    let matched = actual == *expected;

                    if matched {
                        match_count += 1;
                    } else {
                        mismatch_count += 1;
                    }

                    slots_checked.push(SlotCheck {
                        slot_index: *slot_idx,
                        character_name: self.slot_names.get(*slot_idx)
                            .cloned()
                            .unwrap_or_else(|| format!("Slot {}", slot_idx)),
                        expected_value: Some(*expected),
                        actual_value: actual,
                        matched,
                    });
                }
            }
        }

        let all_matched = mismatch_count == 0 && match_count >= self.config.min_slots_agree;

        // Calculate confidence adjustment
        let confidence_adjustment = if all_matched {
            self.config.full_agreement_boost
        } else if mismatch_count > 0 {
            -self.config.disagreement_penalty * (mismatch_count as f64 / slots_checked.len() as f64)
        } else {
            0.0
        };

        CrossValidationResult {
            flag_id,
            byte_offset,
            bit_position,
            slots_checked,
            all_matched,
            match_count,
            mismatch_count,
            confidence_adjustment,
        }
    }

    /// Validate a flag by just checking if the bit is set in slots where we expect it to be set
    pub fn validate_flag_simple(
        &self,
        flag_id: u32,
        byte_offset: usize,
        bit_position: u8,
        slots_where_set: &[usize],
    ) -> CrossValidationResult {
        let expected_values: Vec<(usize, bool)> = (0..self.slot_flags.len())
            .filter(|i| self.slot_flags[*i].is_some())
            .map(|i| (i, slots_where_set.contains(&i)))
            .collect();

        self.validate_offset(flag_id, byte_offset, bit_position, &expected_values)
    }

    /// Validate and create observation for discovery store
    pub fn validate_and_create_observation(
        &self,
        flag_id: u32,
        byte_offset: usize,
        bit_position: u8,
        expected_values: &[(usize, bool)],
    ) -> (CrossValidationResult, Option<OffsetObservation>) {
        let result = self.validate_offset(flag_id, byte_offset, bit_position, expected_values);

        let observation = if result.match_count >= self.config.min_slots_agree {
            let base_confidence = if result.all_matched { 0.95 } else { 0.75 };
            let adjusted_confidence = (base_confidence + result.confidence_adjustment).clamp(0.0, 1.0);

            Some(OffsetObservation::new(
                byte_offset,
                bit_position,
                ObservationSource::CrossSlotValidation {
                    slots_validated: result.slots_checked.iter().map(|s| s.slot_index).collect(),
                    all_matched: result.all_matched,
                },
                None,
                None,
                adjusted_confidence,
            ))
        } else {
            None
        };

        (result, observation)
    }

    /// Read a bit value from a specific slot
    pub fn read_bit(&self, slot_index: usize, byte_offset: usize, bit_position: u8) -> Option<bool> {
        let flags = self.slot_flags.get(slot_index)?.as_ref()?;
        if byte_offset >= flags.len() {
            return None;
        }
        let byte = flags[byte_offset];
        Some((byte & (1 << (7 - bit_position))) != 0)
    }

    /// Get the number of available slots
    pub fn available_slot_count(&self) -> usize {
        self.slot_flags.iter().filter(|s| s.is_some()).count()
    }

    /// Get slot indices that have valid event flags
    pub fn available_slots(&self) -> Vec<usize> {
        self.slot_flags.iter()
            .enumerate()
            .filter(|(_, s)| s.is_some())
            .map(|(i, _)| i)
            .collect()
    }
}

/// Get character name for a slot index
fn get_slot_name(slot_index: usize) -> String {
    match slot_index {
        0 => "Confessor".to_string(),
        1 => "Wretch".to_string(),
        2 => "V1".to_string(),
        3 => "V2".to_string(),
        4 => "V3".to_string(),
        5 => "Sam".to_string(),
        _ => format!("Slot {}", slot_index),
    }
}

/// Batch validate discoveries across slots
pub fn batch_validate(
    save_path: &Path,
    store: &mut DiscoveryStore,
    known_slot_flags: &[(u32, Vec<(usize, bool)>)], // (flag_id, [(slot, expected)])
) -> Result<BatchValidationResult, String> {
    let validator = CrossValidator::from_path(save_path)?;

    let mut validated = 0;
    let mut confirmed = 0;
    let mut rejected = 0;

    for (flag_id, expected_values) in known_slot_flags {
        if let Some(discovery) = store.get(*flag_id) {
            if let Some((byte, bit)) = discovery.get_offset() {
                let (result, observation) = validator.validate_and_create_observation(
                    *flag_id,
                    byte,
                    bit,
                    expected_values,
                );

                validated += 1;

                if let Some(obs) = observation {
                    store.add_observation(*flag_id, obs);
                    if result.all_matched {
                        confirmed += 1;
                    }
                } else if result.mismatch_count > result.match_count {
                    rejected += 1;
                }
            }
        }
    }

    Ok(BatchValidationResult {
        discoveries_validated: validated,
        discoveries_confirmed: confirmed,
        discoveries_rejected: rejected,
        available_slots: validator.available_slot_count(),
    })
}

/// Result of batch validation
#[derive(Debug)]
pub struct BatchValidationResult {
    pub discoveries_validated: usize,
    pub discoveries_confirmed: usize,
    pub discoveries_rejected: usize,
    pub available_slots: usize,
}

impl std::fmt::Display for BatchValidationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Batch Validation Result:")?;
        writeln!(f, "  Slots available: {}", self.available_slots)?;
        writeln!(f, "  Discoveries validated: {}", self.discoveries_validated)?;
        writeln!(f, "  Confirmed by cross-slot: {}", self.discoveries_confirmed)?;
        writeln!(f, "  Rejected (mismatched): {}", self.discoveries_rejected)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_name_mapping() {
        assert_eq!(get_slot_name(0), "Confessor");
        assert_eq!(get_slot_name(1), "Wretch");
        assert_eq!(get_slot_name(5), "Sam");
        assert_eq!(get_slot_name(9), "Slot 9");
    }

    #[test]
    fn test_config_defaults() {
        let config = CrossValidationConfig::default();
        assert_eq!(config.min_slots_agree, 2);
        assert!(config.full_agreement_boost > 0.0);
    }

    #[test]
    #[ignore] // Requires actual save file
    fn test_cross_validation() {
        let save_path = Path::new("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2");
        if save_path.exists() {
            let validator = CrossValidator::from_path(save_path).unwrap();
            println!("Available slots: {}", validator.available_slot_count());

            // Test a known flag (The First Step grace - 71800)
            // Should be set in slots with any progress
            let result = validator.validate_flag_simple(
                71800,
                2625 + (71800 - 71000) / 8, // Calculated offset
                7 - ((71800 % 8) as u8),
                &[0, 1], // Expected set in Confessor and Wretch
            );

            println!("Flag 71800 validation: {:?}", result);
        }
    }
}
