//! WebAssembly module for Elden Ring EventFlags detection
//!
//! This is the **SINGLE SOURCE OF TRUTH** for EventFlags offset detection.
//! Used by both ER-save-Editor (native Rust) and elden-map (via WASM).
//!
//! The algorithm searches for known grace flag patterns to locate the
//! EventFlags section within character slot data.
//!
//! ## Documentation
//!
//! - ER-save-Editor: `docs/WASM-EVENT-FLAGS.md`
//! - elden-map: `docs/WASM-EVENT-FLAGS.md`
//!
//! ## Rebuilding WASM
//!
//! ```bash
//! cd crates/wasm-event-flags
//! wasm-pack build --target web --out-dir ../../../elden-map/wasm-event-flags
//! ```

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

/// Validation flag for detecting EventFlags offset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationFlag {
    pub flag_id: u32,
    pub byte_offset: u32,
    pub bit_position: u8,
    pub name: String,
    #[serde(default)]
    pub tier: u8,
}

/// Known grace flags used to validate EventFlags offset detection (POSITIVE).
/// These graces should be discovered by any character past the tutorial.
/// Tier 1 flags MUST be set for any playable character.
pub const POSITIVE_VALIDATION_FLAGS: &[(u32, u32, u8, &str, u8)] = &[
    // Tier 1: Tutorial and first graces (MUST be set)
    (71800, 2725, 7, "Cave of Knowledge", 1),
    (71801, 2725, 6, "Stranded Graveyard", 1),
    (76100, 3262, 3, "The First Step", 1),
    (76101, 3262, 2, "Church of Elleh", 1),
    // Tier 2: Early game graces (likely set for most characters)
    (76102, 3262, 1, "Gatefront Ruins", 2),
    (76104, 3263, 7, "Agheel Lake South", 2),
    (76106, 3263, 5, "Church of Dragon Communion", 2),
];

/// Late-game grace flags used for NEGATIVE validation.
/// These graces require significant progression and should NOT be set
/// for characters that have just completed the tutorial.
/// If these ARE set at a candidate offset, it's likely a false positive.
pub const NEGATIVE_VALIDATION_FLAGS: &[(u32, u32, u8, &str)] = &[
    // Leyndell Capital - requires 2 Great Runes
    (76223, 3277, 0, "Fortified Manor, First Floor"),
    (76224, 3278, 7, "East Capital Rampart"),
    (76225, 3278, 6, "Divine Bridge"),
    // Mountaintops of the Giants - very late game
    (76300, 3287, 3, "Zamor Ruins"),
    (76301, 3287, 2, "Ancient Snow Valley Ruins"),
    // Haligtree - endgame optional area
    (76350, 3293, 5, "Haligtree Town"),
];

/// Search parameters
pub const SEARCH_START: usize = 0x12000;  // 73728
pub const MAX_SEARCH_RANGE: usize = 200_000;

/// Event flags section size (constant across all saves)
pub const EVENT_FLAGS_SIZE: usize = 0x1BF99F;  // 1,833,375 bytes

/// Result of EventFlags offset detection
#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// Detected offset from start of slot data
    pub offset: usize,
    /// Number of positive validation flags matched
    pub positive_score: usize,
    /// Number of negative flags correctly NOT set
    pub negative_score: usize,
    /// Whether detection is confident (all tier-1 positive + all negative)
    pub confident: bool,
}

/// Detect the EventFlags offset within slot data.
///
/// This is the SINGLE SOURCE OF TRUTH algorithm used by both
/// ER-save-Editor and elden-map.
///
/// Algorithm:
/// 1. Search from SEARCH_START (0x12000) for offsets where ALL tier-1 flags match
/// 2. For each candidate, calculate positive score (all flags) and negative score
/// 3. Return FIRST offset with ALL negative flags UNSET (perfect match)
/// 4. If no perfect match, pick candidate with highest negative score,
///    then highest positive score, then lowest offset
///
/// # Arguments
/// * `slot_data` - Raw bytes of the character slot (Uint8Array from JS)
///
/// # Returns
/// * `DetectionResult` with detected offset and confidence info
#[wasm_bindgen]
pub fn detect_event_flags_offset(slot_data: &[u8]) -> DetectionResult {
    detect_event_flags_offset_impl(slot_data)
}

/// Internal implementation (also usable from native Rust without WASM)
pub fn detect_event_flags_offset_impl(slot_data: &[u8]) -> DetectionResult {
    let search_end = (SEARCH_START + MAX_SEARCH_RANGE).min(slot_data.len().saturating_sub(10000));

    let tier1_flags: Vec<_> = POSITIVE_VALIDATION_FLAGS.iter()
        .filter(|(_, _, _, _, tier)| *tier == 1)
        .collect();
    let tier1_count = tier1_flags.len();

    struct Candidate {
        offset: usize,
        positive_score: usize,
        negative_score: usize,
    }

    let mut candidates: Vec<Candidate> = Vec::new();

    // Phase 1: Find ALL offsets where all Tier 1 flags match
    for test_offset in SEARCH_START..search_end {
        let mut tier1_score = 0;
        let mut positive_score = 0;

        // Check positive flags
        for &(_, byte_offset, bit_pos, _, tier) in POSITIVE_VALIDATION_FLAGS {
            let abs_pos = test_offset + byte_offset as usize;
            if abs_pos < slot_data.len() {
                let byte = slot_data[abs_pos];
                if (byte & (1 << bit_pos)) != 0 {
                    positive_score += 1;
                    if tier == 1 {
                        tier1_score += 1;
                    }
                }
            }
        }

        // Only consider offsets where ALL Tier 1 flags match
        if tier1_score >= tier1_count {
            // Count negative flags that are NOT set
            let mut negative_score = 0;
            for &(_, byte_offset, bit_pos, _) in NEGATIVE_VALIDATION_FLAGS {
                let abs_pos = test_offset + byte_offset as usize;
                if abs_pos < slot_data.len() {
                    let byte = slot_data[abs_pos];
                    if (byte & (1 << bit_pos)) == 0 {
                        negative_score += 1;
                    }
                }
            }

            // If all negative flags are NOT set, this is a perfect match
            if negative_score == NEGATIVE_VALIDATION_FLAGS.len() {
                return DetectionResult {
                    offset: test_offset,
                    positive_score,
                    negative_score,
                    confident: true,
                };
            }

            candidates.push(Candidate {
                offset: test_offset,
                positive_score,
                negative_score,
            });
        }
    }

    // Phase 2: No perfect match - pick best candidate
    if !candidates.is_empty() {
        // Sort by: highest negative score, then highest positive score, then lowest offset
        candidates.sort_by(|a, b| {
            b.negative_score.cmp(&a.negative_score)
                .then_with(|| b.positive_score.cmp(&a.positive_score))
                .then_with(|| a.offset.cmp(&b.offset))
        });

        let best = &candidates[0];
        return DetectionResult {
            offset: best.offset,
            positive_score: best.positive_score,
            negative_score: best.negative_score,
            confident: best.negative_score >= NEGATIVE_VALIDATION_FLAGS.len() / 2,
        };
    }

    // Phase 3: Fallback - no candidates with all Tier 1 flags
    let mut best_offset = SEARCH_START;
    let mut best_tier1_score = 0;

    for test_offset in SEARCH_START..search_end {
        let mut tier1_score = 0;

        for &(_, byte_offset, bit_pos, _, tier) in POSITIVE_VALIDATION_FLAGS {
            if tier != 1 {
                continue;
            }
            let abs_pos = test_offset + byte_offset as usize;
            if abs_pos < slot_data.len() {
                let byte = slot_data[abs_pos];
                if (byte & (1 << bit_pos)) != 0 {
                    tier1_score += 1;
                }
            }
        }

        if tier1_score > best_tier1_score {
            best_tier1_score = tier1_score;
            best_offset = test_offset;
        }
    }

    DetectionResult {
        offset: best_offset,
        positive_score: best_tier1_score,
        negative_score: 0,
        confident: false,
    }
}

/// Get the event flags section size constant
#[wasm_bindgen]
pub fn get_event_flags_size() -> usize {
    EVENT_FLAGS_SIZE
}

/// Get the search start offset
#[wasm_bindgen]
pub fn get_search_start() -> usize {
    SEARCH_START
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(SEARCH_START, 0x12000);
        assert_eq!(EVENT_FLAGS_SIZE, 0x1BF99F);
        assert_eq!(POSITIVE_VALIDATION_FLAGS.len(), 7);
        assert_eq!(NEGATIVE_VALIDATION_FLAGS.len(), 6);
    }

    #[test]
    fn test_tier1_count() {
        let tier1_count = POSITIVE_VALIDATION_FLAGS.iter()
            .filter(|(_, _, _, _, tier)| *tier == 1)
            .count();
        assert_eq!(tier1_count, 4);
    }
}
