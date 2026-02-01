//! Calibration Service - Dynamic formula base calibration for per-save verification.
//!
//! The tile formula base_offset (485330 in ground_truth_offsets.json) varies per save
//! due to the GaItems (inventory) section having variable size. This module provides
//! a reusable calibration service that detects the correct bases for any given save.
//!
//! # Usage
//!
//! ```rust
//! use crate::calibration::{CalibrationService, CalibrationResult};
//!
//! // Calibrate for a specific slot's event flags
//! let result = CalibrationService::calibrate(&event_flags);
//! println!("Tile base: {} (confidence: {})", result.tile_base, result.tile_base_confidence);
//! ```

use std::collections::HashMap;

use crate::generated::ground_truth::{
    VERIFIED_TILE_BASE_OFFSET,
    TILE_BYTES_PER_SLOT,
    TILE_SLOTS_PER_ROW,
    TILE_ROW_BASE,
    TILE_COL_BASE,
    VERIFIED_BLOCK_BASES,
    VERIFIED_DUNGEON_BASES,
};

// ============================================================================
// CALIBRATION ANCHORS
// ============================================================================

/// Calibration anchor for tile formula verification.
/// Smoldering Butterfly is a common early-game world pickup near Agheel Lake.
pub const TILE_ANCHOR_FLAG_ID: u32 = 1043500010;
pub const TILE_ANCHOR_ROW: u32 = 43;
pub const TILE_ANCHOR_COL: u32 = 50;
pub const TILE_ANCHOR_LOCAL_ID: u32 = 10;

/// Calibration anchor for block formula verification.
/// The First Step is the first overworld grace, always discovered.
pub const BLOCK_ANCHOR_FLAG_ID: u32 = 76100;
pub const BLOCK_ANCHOR_BLOCK_START: u32 = 76000;

// ============================================================================
// TYPES
// ============================================================================

/// Source of the calibrated tile base
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalibrationSource {
    /// Anchor flag found SET at ground truth base (highest confidence)
    AnchorVerified,
    /// Anchor flag found SET at different offset via search
    Search,
    /// Anchor flag NOT SET, using ground truth as fallback
    GroundTruth,
    /// Calibration failed
    Unknown,
}

impl CalibrationSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalibrationSource::AnchorVerified => "anchor_verified",
            CalibrationSource::Search => "search",
            CalibrationSource::GroundTruth => "ground_truth",
            CalibrationSource::Unknown => "unknown",
        }
    }
}

/// Result of calibrating formula bases for a specific save state.
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    /// Calibrated tile base offset
    pub tile_base: u32,
    /// Confidence level (0.0 - 1.0)
    pub tile_base_confidence: f32,
    /// How the tile base was determined
    pub tile_base_source: CalibrationSource,
    /// Whether block bases were verified
    pub block_bases_verified: bool,
    /// Number of verified dungeon bases
    pub dungeon_bases_count: usize,
    /// Flag IDs used for calibration
    pub calibration_flags_used: Vec<u32>,
    /// Notes about the calibration
    pub notes: String,
}

impl Default for CalibrationResult {
    fn default() -> Self {
        Self {
            tile_base: VERIFIED_TILE_BASE_OFFSET,
            tile_base_confidence: 0.0,
            tile_base_source: CalibrationSource::Unknown,
            block_bases_verified: false,
            dungeon_bases_count: 0,
            calibration_flags_used: Vec::new(),
            notes: String::new(),
        }
    }
}

// ============================================================================
// CALIBRATION SERVICE
// ============================================================================

/// Calibration service for determining correct formula bases per-save.
pub struct CalibrationService;

impl CalibrationService {
    /// Calibrate formula bases for a slot's event flags.
    ///
    /// This determines the actual tile/dungeon bases for this save state,
    /// which may differ from other saves due to variable GaItems size.
    ///
    /// # Arguments
    ///
    /// * `event_flags` - The event flags byte slice from the save slot
    ///
    /// # Returns
    ///
    /// CalibrationResult with detected bases and confidence levels
    pub fn calibrate(event_flags: &[u8]) -> CalibrationResult {
        let mut result = CalibrationResult::default();

        // Calibrate tile base using anchor flag
        if let Some((base, confidence, source)) = Self::calibrate_tile_base(event_flags) {
            result.tile_base = base;
            result.tile_base_confidence = confidence;
            result.tile_base_source = source;
            result.calibration_flags_used.push(TILE_ANCHOR_FLAG_ID);
        }

        // Verify block bases work
        result.block_bases_verified = Self::verify_block_bases(event_flags);
        if result.block_bases_verified {
            result.calibration_flags_used.push(BLOCK_ANCHOR_FLAG_ID);
        }

        // Count verified dungeon bases
        result.dungeon_bases_count = VERIFIED_DUNGEON_BASES.len();

        result.notes = format!(
            "Tile base: {} ({}), Blocks: {}, Dungeons: {}",
            result.tile_base,
            result.tile_base_source.as_str(),
            if result.block_bases_verified { "verified" } else { "unverified" },
            result.dungeon_bases_count
        );

        result
    }

    /// Get the calibrated tile base for event flags.
    ///
    /// # Returns
    ///
    /// Tuple of (base_offset, confidence)
    pub fn get_tile_base(event_flags: &[u8]) -> (u32, f32) {
        let result = Self::calibrate(event_flags);
        (result.tile_base, result.tile_base_confidence)
    }

    /// Calculate tile flag offset using calibrated base.
    ///
    /// # Arguments
    ///
    /// * `flag_id` - The 10-digit tile flag ID (e.g., 1043500010)
    /// * `calibrated_base` - The calibrated tile base offset
    ///
    /// # Returns
    ///
    /// Option<(byte_offset, bit_position)> or None if invalid flag format
    pub fn get_tile_offset_calibrated(flag_id: u32, calibrated_base: u32) -> Option<(u32, u8)> {
        // Validate format (10-digit tile flag)
        if flag_id < 1_000_000_000 || flag_id >= 2_000_000_000 {
            return None;
        }

        // Parse components: 10XXYYZZZZ
        let flag_str = flag_id.to_string();
        if flag_str.len() != 10 {
            return None;
        }

        let row: u32 = flag_str[2..4].parse().ok()?;
        let col: u32 = flag_str[4..6].parse().ok()?;
        let local_id: u32 = flag_str[6..].parse().ok()?;

        // Check local_id limit
        if local_id > 6999 {
            return None; // Untrackable
        }

        // Calculate offset
        let tile_offset = ((row - TILE_ROW_BASE) * TILE_SLOTS_PER_ROW + (col - TILE_COL_BASE))
            * TILE_BYTES_PER_SLOT;
        let byte_offset = calibrated_base + tile_offset + local_id / 8;
        let bit_position = 7 - (local_id % 8) as u8;

        Some((byte_offset, bit_position))
    }

    /// Check if a flag is set at the given offset in event flags.
    pub fn is_flag_set(event_flags: &[u8], byte_offset: u32, bit_position: u8) -> bool {
        if (byte_offset as usize) < event_flags.len() {
            let byte_val = event_flags[byte_offset as usize];
            (byte_val >> bit_position) & 1 == 1
        } else {
            false
        }
    }

    // ------------------------------------------------------------------------
    // Private calibration methods
    // ------------------------------------------------------------------------

    /// Calibrate the tile formula base using known anchor flags.
    ///
    /// Returns (base_offset, confidence, source) or None.
    ///
    /// Confidence levels:
    /// - 0.95: Anchor flag SET at ground truth base
    /// - 0.70: Anchor flag SET at different offset (found via search)
    /// - 0.50: Anchor flag NOT SET, using ground truth as fallback
    fn calibrate_tile_base(event_flags: &[u8]) -> Option<(u32, f32, CalibrationSource)> {
        let base_offset = VERIFIED_TILE_BASE_OFFSET;

        // Calculate expected offset using ground truth base
        let tile_offset = ((TILE_ANCHOR_ROW - TILE_ROW_BASE) * TILE_SLOTS_PER_ROW
            + (TILE_ANCHOR_COL - TILE_COL_BASE))
            * TILE_BYTES_PER_SLOT;
        let byte_offset = base_offset + tile_offset + TILE_ANCHOR_LOCAL_ID / 8;
        let bit_pos = 7 - (TILE_ANCHOR_LOCAL_ID % 8) as u8;

        // Check if the flag is SET at ground truth location
        if (byte_offset as usize) < event_flags.len() {
            let byte_val = event_flags[byte_offset as usize];
            let is_set = (byte_val >> bit_pos) & 1 == 1;

            if is_set {
                // Ground truth base works for this save
                return Some((base_offset, 0.95, CalibrationSource::AnchorVerified));
            } else {
                // Try to find the correct base by searching
                if let Some((found_base, confidence, source)) =
                    Self::search_for_tile_base(event_flags)
                {
                    return Some((found_base, confidence, source));
                }
                // Anchor not found - use ground truth with low confidence
                return Some((base_offset, 0.50, CalibrationSource::GroundTruth));
            }
        }

        None
    }

    /// Search for the correct tile base by looking for the anchor flag.
    ///
    /// This handles cases where the save has a different base than ground truth
    /// due to variable inventory size.
    fn search_for_tile_base(event_flags: &[u8]) -> Option<(u32, f32, CalibrationSource)> {
        let expected_bit = 7 - (TILE_ANCHOR_LOCAL_ID % 8) as u8;

        // Calculate tile offset (constant regardless of base)
        let tile_offset = ((TILE_ANCHOR_ROW - TILE_ROW_BASE) * TILE_SLOTS_PER_ROW
            + (TILE_ANCHOR_COL - TILE_COL_BASE))
            * TILE_BYTES_PER_SLOT;
        let local_byte_offset = TILE_ANCHOR_LOCAL_ID / 8;

        // Search for the base that makes this flag SET
        // The tile region is typically around 480k-510k
        let search_start: u32 = 480000;
        let search_end = std::cmp::min(
            510000,
            event_flags.len() as u32 - tile_offset - local_byte_offset,
        );

        for base in search_start..search_end {
            let byte_offset = base + tile_offset + local_byte_offset;
            if (byte_offset as usize) < event_flags.len() {
                let byte_val = event_flags[byte_offset as usize];
                if (byte_val >> expected_bit) & 1 == 1 {
                    // Found it! But verify it's not 0xFF padding
                    if byte_val != 0xFF {
                        return Some((base, 0.70, CalibrationSource::Search));
                    }
                }
            }
        }

        None
    }

    /// Verify known block bases work for this save.
    fn verify_block_bases(event_flags: &[u8]) -> bool {
        // Check anchor flag (The First Step grace - flag 76100)
        if let Some(block_base) = VERIFIED_BLOCK_BASES.get(&BLOCK_ANCHOR_BLOCK_START) {
            let relative = BLOCK_ANCHOR_FLAG_ID - BLOCK_ANCHOR_BLOCK_START;
            let byte_offset = block_base.base_offset + relative / 8;
            let bit_pos = 7 - (BLOCK_ANCHOR_FLAG_ID % 8) as u8;

            if (byte_offset as usize) < event_flags.len() {
                let byte_val = event_flags[byte_offset as usize];
                let is_set = (byte_val >> bit_pos) & 1 == 1;
                return is_set;
            }
        }

        false
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_source_as_str() {
        assert_eq!(CalibrationSource::AnchorVerified.as_str(), "anchor_verified");
        assert_eq!(CalibrationSource::Search.as_str(), "search");
        assert_eq!(CalibrationSource::GroundTruth.as_str(), "ground_truth");
        assert_eq!(CalibrationSource::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_get_tile_offset_calibrated() {
        // Test Smoldering Butterfly (1043500010)
        let result = CalibrationService::get_tile_offset_calibrated(1043500010, 485330);
        assert!(result.is_some());
        let (offset, bit) = result.unwrap();
        assert_eq!(offset, 852831);
        assert_eq!(bit, 5);
    }

    #[test]
    fn test_get_tile_offset_calibrated_invalid() {
        // Test invalid flag (not a tile flag)
        assert!(CalibrationService::get_tile_offset_calibrated(76100, 485330).is_none());
        // Test untrackable local_id
        assert!(CalibrationService::get_tile_offset_calibrated(1043507000, 485330).is_none());
    }

    #[test]
    fn test_default_calibration_result() {
        let result = CalibrationResult::default();
        assert_eq!(result.tile_base, VERIFIED_TILE_BASE_OFFSET);
        assert_eq!(result.tile_base_confidence, 0.0);
        assert_eq!(result.tile_base_source, CalibrationSource::Unknown);
    }
}
