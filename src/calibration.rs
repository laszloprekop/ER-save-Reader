#![allow(dead_code)]
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

/// Tutorial grace anchor for delta detection.
/// Cave of Knowledge is discovered by ALL characters after tutorial.
pub const TUTORIAL_GRACE_FLAG_ID: u32 = 71800;
pub const TUTORIAL_GRACE_GROUND_TRUTH_OFFSET: u32 = 2725;
pub const TUTORIAL_GRACE_BIT: u8 = 7;

/// Unreliable 71xxx grace blocks that share a common calibration delta.
/// These blocks have per-save offset variations due to variable section sizes.
pub const UNRELIABLE_71XXX_BLOCKS: [(u32, u32); 3] = [
    (71000, 9315),  // Stormveil graces
    (71100, 2593),  // Leyndell graces
    (71600, 3198),  // Volcano Manor graces
];

/// Block 73000 (dungeon graces) needs SEPARATE calibration.
/// It has its own offset that varies independently from 71xxx blocks.
/// The delta can be very large (e.g., -2561) compared to 71xxx blocks.
pub const DUNGEON_GRACE_BLOCK_START: u32 = 73000;
pub const DUNGEON_GRACE_GROUND_TRUTH_BASE: u32 = 2662;

/// Dungeon grace anchors for 73000 block calibration.
/// These are early-game dungeons that most players visit.
/// Format: (flag_id, ground_truth_offset, bit_position, name)
pub const DUNGEON_GRACE_ANCHORS: [(u32, u32, u8, &str); 5] = [
    (73000, 2662, 7, "Tombsward Catacombs"),
    (73002, 2662, 5, "Stormfoot Catacombs"),
    (73004, 2662, 3, "Murkwater Catacombs"),
    (73100, 2674, 3, "Murkwater Cave"),      // (73100-73000)/8 + 2662 = 12+2662 = 2674
    (73103, 2674, 0, "Groveside Cave"),
];

/// Legacy dungeon grace blocks that need individual calibration.
/// Format: (block_start, ground_truth_base, name, anchors)
/// where anchors = [(flag_id, bit_position, name), ...]
pub const LEGACY_DUNGEON_BLOCKS: [(u32, u32, &str, &[(u32, u8, &str)]); 1] = [
    (71000, 9315, "Stormveil Castle", &[
        (71001, 6, "Margit, the Fell Omen"),
        (71002, 5, "Castleward Tunnel"),
        (71003, 4, "Gateside Chamber"),
        (71004, 3, "Stormveil Cliffside"),
        (71005, 2, "Rampart Tower"),
        (71007, 0, "Secluded Cell"),
    ]),
    // Volcano Manor (71600) DISABLED - optional content causes false positives
    // Leyndell (71100) - TODO: add when we have better validation strategy
];

/// Prerequisites for legacy dungeons - must be defeated before calibrating.
/// This prevents false positives when the player hasn't reached the dungeon yet.
/// Format: (block_start, prerequisite_boss_flag)
pub const DUNGEON_PREREQUISITES: [(u32, u32); 1] = [
    (71000, 10000850),  // Stormveil requires Margit defeat
];

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

/// Calibrated base for a single grace block.
#[derive(Debug, Clone, Copy)]
pub struct CalibratedGraceBlock {
    /// Block start (e.g., 71000, 71100, 71600)
    pub block_start: u32,
    /// Calibrated base offset for this save
    pub calibrated_base: u32,
    /// Original ground truth base offset
    pub ground_truth_base: u32,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
}

/// Calibration result for grace blocks.
#[derive(Debug, Clone, Default)]
pub struct GraceBlockCalibration {
    /// Whether calibration was successful
    pub success: bool,
    /// Detected offset delta from ground truth
    pub offset_delta: i32,
    /// Calibrated bases for unreliable blocks
    pub calibrated_blocks: Vec<CalibratedGraceBlock>,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
    /// Notes about the calibration
    pub notes: String,
}

impl GraceBlockCalibration {
    /// Get calibrated base for a specific block.
    pub fn get_calibrated_base(&self, block_start: u32) -> Option<u32> {
        self.calibrated_blocks
            .iter()
            .find(|b| b.block_start == block_start)
            .map(|b| b.calibrated_base)
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
    /// Grace block calibration for unreliable blocks
    pub grace_block_calibration: GraceBlockCalibration,
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
            grace_block_calibration: GraceBlockCalibration::default(),
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

        // Calibrate grace blocks using tutorial grace anchor
        result.grace_block_calibration = Self::calibrate_grace_blocks(event_flags);
        if result.grace_block_calibration.success {
            result.calibration_flags_used.push(TUTORIAL_GRACE_FLAG_ID);
        }

        // Count verified dungeon bases
        result.dungeon_bases_count = VERIFIED_DUNGEON_BASES.len();

        result.notes = format!(
            "Tile base: {} ({}), Blocks: {}, Grace blocks: {} (delta={}), Dungeons: {}",
            result.tile_base,
            result.tile_base_source.as_str(),
            if result.block_bases_verified { "verified" } else { "unverified" },
            if result.grace_block_calibration.success { "calibrated" } else { "uncalibrated" },
            result.grace_block_calibration.offset_delta,
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
                    // Found it! Note: Don't skip 0xFF - when all flags in byte are set, 0xFF is valid
                    return Some((base, 0.70, CalibrationSource::Search));
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

    /// Calibrate grace blocks using tutorial grace as anchor.
    ///
    /// The tutorial grace (Cave of Knowledge, flag 71800) is discovered by ALL
    /// characters after completing the tutorial. We use it to detect the offset
    /// delta between ground truth and this specific save's EF layout.
    ///
    /// Note: 71xxx blocks share a common delta, but 73000 (dungeon graces)
    /// has its own independent offset and is calibrated separately.
    ///
    /// Returns GraceBlockCalibration with calibrated bases for unreliable blocks.
    fn calibrate_grace_blocks(event_flags: &[u8]) -> GraceBlockCalibration {
        let mut result = GraceBlockCalibration::default();

        // Phase 1: Detect offset delta using tutorial grace anchor
        let delta = match Self::detect_offset_delta(event_flags) {
            Some(d) => d,
            None => {
                result.notes = "Tutorial grace anchor not found".to_string();
                return result;
            }
        };

        result.offset_delta = delta;

        // Phase 2: Validate delta using block 76000 (The First Step)
        let validation_score = Self::validate_delta(event_flags, delta);
        if validation_score < 0.5 {
            result.notes = format!(
                "Delta {} failed validation (score: {:.2})",
                delta, validation_score
            );
            return result;
        }

        // Phase 3: Apply delta to unreliable 71xxx grace blocks
        for (block_start, ground_truth_base) in UNRELIABLE_71XXX_BLOCKS {
            let calibrated_base = (ground_truth_base as i32 + delta) as u32;

            // Confidence is based on validation score
            let confidence = if validation_score >= 0.9 {
                0.90
            } else if validation_score >= 0.75 {
                0.80
            } else {
                0.65
            };

            result.calibrated_blocks.push(CalibratedGraceBlock {
                block_start,
                calibrated_base,
                ground_truth_base,
                confidence,
            });
        }

        // Phase 4: Calibrate 73000 block separately (has different delta)
        if let Some(dungeon_block) = Self::calibrate_dungeon_grace_block(event_flags) {
            result.calibrated_blocks.push(dungeon_block);
        }

        // Phase 5: Calibrate legacy dungeon blocks (Stormveil, etc.)
        // Each legacy dungeon has its own offset that doesn't share delta with tutorial
        let mut legacy_notes: Vec<String> = Vec::new();
        for &(block_start, ground_truth_base, _name, anchors) in &LEGACY_DUNGEON_BLOCKS {
            if let Some(calibration) = Self::calibrate_legacy_dungeon_block(
                event_flags,
                block_start,
                ground_truth_base,
                anchors,
            ) {
                legacy_notes.push(format!("{}:{}", block_start, calibration.calibrated_base));
                result.calibrated_blocks.push(calibration);
            }
        }

        result.success = true;
        result.confidence = validation_score;

        // Include dungeon calibration info in notes
        let dungeon_delta = result.calibrated_blocks
            .iter()
            .find(|b| b.block_start == 73000)
            .map(|b| b.calibrated_base as i32 - b.ground_truth_base as i32);

        result.notes = format!(
            "Calibrated with delta {} (validation: {:.2}){}{}",
            delta,
            validation_score,
            dungeon_delta.map(|d| format!(", 73000 delta={}", d)).unwrap_or_default(),
            if legacy_notes.is_empty() { String::new() } else { format!(", legacy: {}", legacy_notes.join(", ")) }
        );

        result
    }

    /// Calibrate a legacy dungeon grace block (Stormveil, Volcano Manor, etc.).
    ///
    /// Legacy dungeon blocks do NOT share a delta with the tutorial grace block (71800).
    /// Each needs independent calibration by searching for dungeon-specific grace anchors.
    fn calibrate_legacy_dungeon_block(
        event_flags: &[u8],
        block_start: u32,
        ground_truth_base: u32,
        anchors: &[(u32, u8, &str)],
    ) -> Option<CalibratedGraceBlock> {
        // Verify prerequisite boss defeated before calibrating legacy dungeon
        // This prevents false positives when the player hasn't reached the dungeon yet
        for &(prereq_block, prereq_flag) in &DUNGEON_PREREQUISITES {
            if prereq_block == block_start {
                // Check if prerequisite boss is defeated
                // 8-digit dungeon boss flags: offset = flag_id / 8, bit = 7 - (flag_id % 8)
                let prereq_offset = prereq_flag / 8;
                let prereq_bit = 7 - (prereq_flag % 8) as u8;
                if (prereq_offset as usize) < event_flags.len() {
                    let byte_val = event_flags[prereq_offset as usize];
                    let prereq_set = (byte_val >> prereq_bit) & 1 == 1;
                    if !prereq_set {
                        return None; // Haven't reached this dungeon yet
                    }
                }
            }
        }

        // Collect all candidates with their validation scores
        let mut candidates: Vec<(u32, usize)> = Vec::new(); // (base_offset, matches)
        let mut seen_bases: std::collections::HashSet<u32> = std::collections::HashSet::new();

        // Search around the ground truth offset - not from 0 to avoid false positives
        // The actual offset should be within ±10000 of ground truth
        let search_radius: i32 = 10000;
        let search_start = (ground_truth_base as i32 - search_radius).max(0) as u32;
        let search_end = std::cmp::min(
            ground_truth_base + search_radius as u32,
            event_flags.len() as u32,
        );

        // Minimum matches required
        // Stormveil (71000): Allow 50% match (3 of 6 graces) since it's required progression
        // Others: HIGH threshold (70%) to prevent false positives from optional content
        let min_matches = if block_start == 71000 {
            std::cmp::max(3, (anchors.len() as f32 * 0.5).ceil() as usize)
        } else {
            std::cmp::max(5, (anchors.len() as f32 * 0.7).ceil() as usize)
        };

        // For legacy dungeons, graces in the same 8-flag range are in the SAME byte
        // We're looking for a byte where multiple dungeon graces are SET
        for base_offset in search_start..search_end {
            if (base_offset as usize) >= event_flags.len() {
                continue;
            }

            let byte_val = event_flags[base_offset as usize];

            // Skip empty bytes
            if byte_val == 0x00 {
                continue;
            }

            // Count how many of our anchors match at this base
            let mut matches = 0;
            for &(_flag_id, bit_position, _name) in anchors {
                if (byte_val >> bit_position) & 1 == 1 {
                    matches += 1;
                }
            }

            // Must meet minimum threshold
            if matches >= min_matches && !seen_bases.contains(&base_offset) {
                candidates.push((base_offset, matches));
                seen_bases.insert(base_offset);
            }
        }

        // Pick the candidate with the MOST matches
        if candidates.is_empty() {
            return None;
        }

        // Sort by matches descending, then by proximity to ground truth
        candidates.sort_by(|a, b| {
            if b.1 != a.1 {
                b.1.cmp(&a.1)
            } else {
                // Prefer candidates closer to ground truth
                let dist_a = (a.0 as i32 - ground_truth_base as i32).abs();
                let dist_b = (b.0 as i32 - ground_truth_base as i32).abs();
                dist_a.cmp(&dist_b)
            }
        });
        let (best_base, best_matches) = candidates[0];

        let confidence = if best_matches >= anchors.len() {
            0.95
        } else if best_matches >= anchors.len() - 1 {
            0.90
        } else if best_matches >= anchors.len() - 2 {
            0.80
        } else {
            0.70
        };

        Some(CalibratedGraceBlock {
            block_start,
            calibrated_base: best_base,
            ground_truth_base,
            confidence,
        })
    }

    /// Calibrate the 73000 block (dungeon graces) separately.
    ///
    /// The 73000 block has its own offset that varies independently from 71xxx blocks.
    /// We search for candidate deltas and pick the one with the MOST matching anchors.
    fn calibrate_dungeon_grace_block(event_flags: &[u8]) -> Option<CalibratedGraceBlock> {
        let ground_truth_base = DUNGEON_GRACE_GROUND_TRUTH_BASE;

        // Collect all candidates with their validation scores
        let mut candidates: Vec<(i32, usize)> = Vec::new(); // (delta, matches)
        let mut seen_deltas: std::collections::HashSet<i32> = std::collections::HashSet::new();

        // Search a wide range since 73000 can be far from ground truth
        // Based on empirical data: delta can be as large as -2561
        let search_range: i32 = 10000;

        for &(flag_id, _gt_offset, bit_pos, _name) in &DUNGEON_GRACE_ANCHORS {
            let expected_offset = ground_truth_base + (flag_id - 73000) / 8;

            // Search both directions
            for offset in 0..search_range {
                for &test_offset in &[offset, -offset] {
                    if test_offset == 0 && offset != 0 {
                        continue; // Skip duplicate 0
                    }

                    let actual_offset = expected_offset as i32 + test_offset;
                    if actual_offset < 0 || actual_offset as usize >= event_flags.len() {
                        continue;
                    }

                    let byte_val = event_flags[actual_offset as usize];
                    // Skip 0x00 (no flags set - can't validate), but allow 0xFF (all flags set is valid)
                    if byte_val == 0x00 {
                        continue;
                    }

                    // Check if this anchor bit is set
                    if (byte_val >> bit_pos) & 1 == 1 {
                        let delta = test_offset;

                        if !seen_deltas.contains(&delta) {
                            let matches = Self::validate_dungeon_delta(event_flags, delta);
                            if matches >= 2 {
                                candidates.push((delta, matches));
                                seen_deltas.insert(delta);
                            }
                        }
                    }
                }
            }
        }

        // Pick the candidate with the MOST matches
        if candidates.is_empty() {
            return None;
        }

        candidates.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by matches descending
        let (best_delta, best_matches) = candidates[0];

        let calibrated_base = (ground_truth_base as i32 + best_delta) as u32;

        let confidence = if best_matches >= 5 {
            0.95
        } else if best_matches >= 4 {
            0.90
        } else if best_matches >= 3 {
            0.80
        } else {
            0.70
        };

        Some(CalibratedGraceBlock {
            block_start: DUNGEON_GRACE_BLOCK_START,
            calibrated_base,
            ground_truth_base,
            confidence,
        })
    }

    /// Validate a delta for 73000 block by checking multiple dungeon grace anchors.
    fn validate_dungeon_delta(event_flags: &[u8], delta: i32) -> usize {
        let mut matches = 0;

        for &(flag_id, _gt_offset, bit_pos, _name) in &DUNGEON_GRACE_ANCHORS {
            let adjusted_offset = DUNGEON_GRACE_GROUND_TRUTH_BASE as i32
                + (flag_id - 73000) as i32 / 8
                + delta;

            if adjusted_offset >= 0 && (adjusted_offset as usize) < event_flags.len() {
                let byte_val = event_flags[adjusted_offset as usize];
                // Skip 0x00 (no flags set - can't validate), but allow 0xFF (all flags set is valid)
                if byte_val != 0x00 {
                    if (byte_val >> bit_pos) & 1 == 1 {
                        matches += 1;
                    }
                }
            }
        }

        matches
    }

    /// Detect offset delta by searching for the tutorial grace anchor.
    ///
    /// Returns the delta (actual_offset - ground_truth_offset) if found.
    fn detect_offset_delta(event_flags: &[u8]) -> Option<i32> {
        let ground_truth = TUTORIAL_GRACE_GROUND_TRUTH_OFFSET;
        let bit_pos = TUTORIAL_GRACE_BIT;

        // First check if it's at the ground truth location (delta = 0)
        // Note: Don't skip 0xFF - when all flags in byte are discovered, 0xFF is valid data
        if (ground_truth as usize) < event_flags.len() {
            let byte_val = event_flags[ground_truth as usize];
            if (byte_val >> bit_pos) & 1 == 1 {
                // Verify by checking adjacent tutorial grace (71801 at bit 6)
                let bit_71801 = 6;
                let is_71801_set = (byte_val >> bit_71801) & 1 == 1;
                if is_71801_set {
                    return Some(0);
                }
            }
        }

        // Search for where the anchor is actually SET
        // Search ±10000 bytes around ground truth
        let search_start = ground_truth.saturating_sub(10000);
        let search_end = std::cmp::min(
            ground_truth + 10000,
            event_flags.len() as u32,
        );

        for offset in search_start..search_end {
            if (offset as usize) < event_flags.len() {
                let byte_val = event_flags[offset as usize];
                // Check if bit is SET - don't skip 0xFF (all 8 flags set is valid data)
                if (byte_val >> bit_pos) & 1 == 1 {
                    // Verify this isn't a false positive by checking adjacent bits
                    // Tutorial grace 71801 should be at bit 6 of the same byte
                    let bit_71801 = 6;
                    let is_71801_set = (byte_val >> bit_71801) & 1 == 1;
                    if is_71801_set {
                        // Both tutorial graces found at this offset
                        return Some(offset as i32 - ground_truth as i32);
                    }
                }
            }
        }

        None
    }

    /// Validate the detected delta using block 76000 graces.
    ///
    /// Returns a score from 0.0 to 1.0 based on how many validation flags match.
    fn validate_delta(event_flags: &[u8], delta: i32) -> f32 {
        // Validation flags from block 76000 (Limgrave graces)
        // These are early-game graces that should be discovered
        let validation_flags: [(u32, u32, u8); 4] = [
            (76100, 3262, 3), // The First Step
            (76101, 3262, 2), // Church of Elleh
            (76102, 3262, 1), // Gatefront
            (76111, 3263, 4), // Another early grace
        ];

        let mut matches = 0;
        let mut checked = 0;

        for (_flag_id, ground_truth_offset, bit_pos) in validation_flags {
            let adjusted_offset = (ground_truth_offset as i32 + delta) as u32;

            if (adjusted_offset as usize) < event_flags.len() {
                let byte_val = event_flags[adjusted_offset as usize];
                // Note: Don't skip 0xFF - when all flags in byte are discovered, 0xFF is valid
                checked += 1;
                if (byte_val >> bit_pos) & 1 == 1 {
                    matches += 1;
                }
            }
        }

        if checked == 0 {
            return 0.0;
        }

        matches as f32 / checked as f32
    }

    /// Get calibrated offset for a grace flag from an unreliable block.
    ///
    /// # Arguments
    ///
    /// * `flag_id` - The grace flag ID (e.g., 71000, 71607, 73004, 73102)
    /// * `calibration` - The grace block calibration result
    ///
    /// # Returns
    ///
    /// Option<(byte_offset, bit_position)> or None if not calibrated
    pub fn get_grace_offset_calibrated(
        flag_id: u32,
        calibration: &GraceBlockCalibration,
    ) -> Option<(u32, u8)> {
        if !calibration.success {
            return None;
        }

        // Determine block start
        // For 73xxx: all flags use block 73000 (1000-flag granularity)
        // For 71xxx: use 100-flag sub-blocks (71000, 71100, 71600, etc.)
        let block_start = if flag_id >= 73000 && flag_id < 74000 {
            73000  // All 73xxx use the same base
        } else {
            (flag_id / 100) * 100  // 100-flag granularity for 71xxx
        };

        // Find calibrated base for this block
        let calibrated_base = calibration.get_calibrated_base(block_start)?;

        // Calculate offset using calibrated base
        let relative = flag_id - block_start;
        let byte_offset = calibrated_base + relative / 8;
        let bit_position = 7 - (flag_id % 8) as u8;

        Some((byte_offset, bit_position))
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
