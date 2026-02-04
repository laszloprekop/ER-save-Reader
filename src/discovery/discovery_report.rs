/// Discovery Report and Iterative Refinement
///
/// This module implements the feedback loop between discovery and verification:
/// 1. Run verification with current formulas
/// 2. Collect failures - which flags don't match expected values
/// 3. Use byte diff to find where those flags ACTUALLY are
/// 4. Compute offset corrections
/// 5. Update formula parameters
/// 6. Repeat until convergence

use std::collections::HashMap;

use crate::db::pickup_flags::{get_flag_offset, is_flag_set};

use super::byte_diff::ByteDiffScanner;
use super::reverse_lookup::{FlagReverser, PossibleFlagType};
use super::segment_analysis::{SegmentAnalyzer, SegmentAnalysisResult};

/// A verification mismatch that needs investigation
#[derive(Debug, Clone)]
pub struct VerificationMismatch {
    pub flag_id: u32,
    pub flag_name: String,
    /// Offset calculated by current formula
    pub calculated_offset: Option<(u32, u8)>,
    /// Expected value (from ground truth or game data)
    pub expected_value: bool,
    /// Actual value found at calculated offset
    pub actual_value: bool,
    /// Possible correct offset found by search
    pub discovered_offset: Option<(usize, u8)>,
}

/// Result of a discovery iteration
#[derive(Debug, Clone)]
pub struct DiscoveryIteration {
    pub iteration: usize,
    pub mismatches_before: usize,
    pub mismatches_after: usize,
    pub corrections_found: Vec<OffsetCorrection>,
    pub flags_verified: usize,
    pub flags_passed: usize,
    pub convergence_score: f64,
}

/// A discovered offset correction
#[derive(Debug, Clone)]
pub struct OffsetCorrection {
    pub flag_id: u32,
    pub old_offset: (u32, u8),
    pub new_offset: (usize, u8),
    pub confidence: f64,
    pub source: CorrectionSource,
}

#[derive(Debug, Clone)]
pub enum CorrectionSource {
    /// Found by comparing differential saves
    Differential { before_path: String, after_path: String },
    /// Found by scanning for known bit pattern
    PatternSearch,
    /// Calculated from other verified flags in same block
    BlockInference { reference_flag: u32 },
}

/// Main discovery engine with iterative refinement
pub struct DiscoveryEngine {
    reverser: FlagReverser,
    analyzer: SegmentAnalyzer,
    /// Accumulated offset corrections
    corrections: HashMap<u32, OffsetCorrection>,
    /// Iteration history
    history: Vec<DiscoveryIteration>,
}

impl DiscoveryEngine {
    pub fn new() -> Self {
        Self {
            reverser: FlagReverser::new(),
            analyzer: SegmentAnalyzer::new(),
            corrections: HashMap::new(),
            history: Vec::new(),
        }
    }

    /// Run a full discovery iteration on a save file
    pub fn run_iteration(
        &mut self,
        event_flags: &[u8],
        known_flags: &[(u32, &str, bool)], // (flag_id, name, expected_value)
    ) -> DiscoveryIteration {
        let iteration_num = self.history.len() + 1;

        // Phase 1: Verify current formulas
        let mut mismatches = Vec::new();
        let mut passed = 0;

        for (flag_id, name, expected) in known_flags {
            let offset = get_flag_offset(*flag_id);
            let actual = is_flag_set(event_flags, *flag_id);

            if actual == *expected {
                passed += 1;
            } else {
                mismatches.push(VerificationMismatch {
                    flag_id: *flag_id,
                    flag_name: name.to_string(),
                    calculated_offset: offset,
                    expected_value: *expected,
                    actual_value: actual,
                    discovered_offset: None,
                });
            }
        }

        let mismatches_before = mismatches.len();

        // Phase 2: Try to find correct offsets for mismatches
        let mut corrections = Vec::new();
        for mismatch in &mut mismatches {
            if let Some(correction) = self.search_for_correct_offset(event_flags, mismatch) {
                mismatch.discovered_offset = Some((correction.new_offset.0, correction.new_offset.1));
                corrections.push(correction);
            }
        }

        // Phase 3: Apply corrections to internal state
        for correction in &corrections {
            self.corrections.insert(correction.flag_id, correction.clone());
        }

        let mismatches_after = mismatches.iter()
            .filter(|m| m.discovered_offset.is_none())
            .count();

        let convergence = if known_flags.len() > 0 {
            passed as f64 / known_flags.len() as f64
        } else {
            0.0
        };

        let result = DiscoveryIteration {
            iteration: iteration_num,
            mismatches_before,
            mismatches_after,
            corrections_found: corrections,
            flags_verified: known_flags.len(),
            flags_passed: passed,
            convergence_score: convergence,
        };

        self.history.push(result.clone());
        result
    }

    /// Search for the correct offset of a mismatched flag
    fn search_for_correct_offset(
        &self,
        event_flags: &[u8],
        mismatch: &VerificationMismatch,
    ) -> Option<OffsetCorrection> {
        // Strategy 1: If we expect true but got false, search for a set bit
        // in the vicinity of the calculated offset
        if mismatch.expected_value && !mismatch.actual_value {
            if let Some((calc_byte, calc_bit)) = mismatch.calculated_offset {
                // Search nearby bytes
                let search_start = calc_byte.saturating_sub(100) as usize;
                let search_end = (calc_byte as usize + 100).min(event_flags.len());

                for byte_off in search_start..search_end {
                    let byte = event_flags[byte_off];
                    if byte != 0 {
                        // Check each set bit
                        for bit in 0..8 {
                            if (byte & (1 << bit)) != 0 {
                                let bit_pos = 7 - bit;
                                // Verify this makes sense by reverse lookup
                                let possibles = self.reverser.reverse_lookup(byte_off, bit_pos);
                                for possible in &possibles {
                                    if possible.flag_id() == Some(mismatch.flag_id) {
                                        return Some(OffsetCorrection {
                                            flag_id: mismatch.flag_id,
                                            old_offset: (calc_byte, calc_bit),
                                            new_offset: (byte_off, bit_pos),
                                            confidence: 0.9,
                                            source: CorrectionSource::PatternSearch,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Run differential discovery between two saves
    pub fn discover_from_diff(
        &mut self,
        before_flags: &[u8],
        after_flags: &[u8],
        expected_changes: &[(u32, bool)], // (flag_id, new_value)
    ) -> DiffDiscoveryResult {
        let scanner = ByteDiffScanner::new();
        let diff = scanner.scan(before_flags, after_flags);

        let mut matched_changes = Vec::new();
        let mut unmatched_expected = Vec::new();
        let mut unexpected_changes = Vec::new();

        // For each bit change, figure out what flag it corresponds to
        for change in &diff.bit_changes {
            let possibles = self.reverser.reverse_lookup(change.byte_offset, change.bit_position);

            let mut found_match = false;
            for (expected_flag, expected_value) in expected_changes {
                // Check if any possible interpretation matches expected
                for possible in &possibles {
                    if possible.flag_id() == Some(*expected_flag) && change.after == *expected_value {
                        matched_changes.push(MatchedChange {
                            flag_id: *expected_flag,
                            byte_offset: change.byte_offset,
                            bit_position: change.bit_position,
                            interpretation: possible.clone(),
                        });
                        found_match = true;
                        break;
                    }
                }
                if found_match { break; }
            }

            if !found_match {
                unexpected_changes.push(UnexpectedChange {
                    byte_offset: change.byte_offset,
                    bit_position: change.bit_position,
                    before: change.before,
                    after: change.after,
                    possible_interpretations: possibles,
                });
            }
        }

        // Find expected changes that weren't matched
        for (expected_flag, expected_value) in expected_changes {
            let already_matched = matched_changes.iter()
                .any(|m| m.flag_id == *expected_flag);

            if !already_matched {
                unmatched_expected.push((*expected_flag, *expected_value));
            }
        }

        DiffDiscoveryResult {
            total_bit_changes: diff.bit_changes.len(),
            matched_changes,
            unmatched_expected,
            unexpected_changes,
        }
    }

    /// Analyze segment structure of event flags
    pub fn analyze_segments(&self, event_flags: &[u8]) -> SegmentAnalysisResult {
        self.analyzer.analyze(event_flags)
    }

    /// Get all accumulated corrections
    pub fn get_corrections(&self) -> &HashMap<u32, OffsetCorrection> {
        &self.corrections
    }

    /// Get iteration history
    pub fn get_history(&self) -> &[DiscoveryIteration] {
        &self.history
    }

    /// Check if discovery has converged (high pass rate)
    pub fn has_converged(&self, threshold: f64) -> bool {
        self.history.last()
            .map(|h| h.convergence_score >= threshold)
            .unwrap_or(false)
    }

    /// Generate a comprehensive discovery report
    pub fn generate_report(&self, event_flags: &[u8]) -> DiscoveryReport {
        let segment_analysis = self.analyze_segments(event_flags);

        DiscoveryReport {
            iterations: self.history.len(),
            final_convergence: self.history.last()
                .map(|h| h.convergence_score)
                .unwrap_or(0.0),
            total_corrections: self.corrections.len(),
            segment_summary: SegmentSummary {
                total_bytes: event_flags.len(),
                flag_bytes: segment_analysis.flag_bytes(),
                empty_bytes: segment_analysis.empty_bytes(),
                segments: segment_analysis.segments.len(),
            },
            corrections: self.corrections.values().cloned().collect(),
        }
    }
}

impl Default for DiscoveryEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of differential discovery
#[derive(Debug)]
pub struct DiffDiscoveryResult {
    pub total_bit_changes: usize,
    pub matched_changes: Vec<MatchedChange>,
    pub unmatched_expected: Vec<(u32, bool)>,
    pub unexpected_changes: Vec<UnexpectedChange>,
}

#[derive(Debug, Clone)]
pub struct MatchedChange {
    pub flag_id: u32,
    pub byte_offset: usize,
    pub bit_position: u8,
    pub interpretation: PossibleFlagType,
}

#[derive(Debug)]
pub struct UnexpectedChange {
    pub byte_offset: usize,
    pub bit_position: u8,
    pub before: bool,
    pub after: bool,
    pub possible_interpretations: Vec<PossibleFlagType>,
}

/// Summary of segment analysis
#[derive(Debug)]
pub struct SegmentSummary {
    pub total_bytes: usize,
    pub flag_bytes: usize,
    pub empty_bytes: usize,
    pub segments: usize,
}

/// Final discovery report
#[derive(Debug)]
pub struct DiscoveryReport {
    pub iterations: usize,
    pub final_convergence: f64,
    pub total_corrections: usize,
    pub segment_summary: SegmentSummary,
    pub corrections: Vec<OffsetCorrection>,
}

impl DiscoveryReport {
    pub fn print(&self) {
        println!("\n╔══════════════════════════════════════════════════════════════╗");
        println!("║                    DISCOVERY REPORT                          ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ Iterations completed: {:>38} ║", self.iterations);
        println!("║ Final convergence: {:>38.1}% ║", self.final_convergence * 100.0);
        println!("║ Offset corrections found: {:>33} ║", self.total_corrections);
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║ SEGMENT ANALYSIS                                             ║");
        println!("║   Total bytes: {:>43} ║", self.segment_summary.total_bytes);
        println!("║   Flag regions: {:>42} ║", self.segment_summary.flag_bytes);
        println!("║   Empty regions: {:>41} ║", self.segment_summary.empty_bytes);
        println!("║   Distinct segments: {:>37} ║", self.segment_summary.segments);
        println!("╚══════════════════════════════════════════════════════════════╝");

        if !self.corrections.is_empty() {
            println!("\nOffset corrections:");
            for (i, corr) in self.corrections.iter().enumerate().take(20) {
                println!("  {:3}. Flag {:>10}: ({:>6}, {:>1}) → ({:>6}, {:>1}) [{:.0}%]",
                    i + 1,
                    corr.flag_id,
                    corr.old_offset.0, corr.old_offset.1,
                    corr.new_offset.0, corr.new_offset.1,
                    corr.confidence * 100.0
                );
            }
            if self.corrections.len() > 20 {
                println!("  ... and {} more", self.corrections.len() - 20);
            }
        }
    }
}

/// Helper to run iterative discovery until convergence
pub fn run_iterative_discovery(
    event_flags: &[u8],
    known_flags: &[(u32, &str, bool)],
    max_iterations: usize,
    convergence_threshold: f64,
) -> DiscoveryReport {
    let mut engine = DiscoveryEngine::new();

    for _ in 0..max_iterations {
        let iteration = engine.run_iteration(event_flags, known_flags);

        println!("Iteration {}: {}/{} passed ({:.1}%), {} corrections found",
            iteration.iteration,
            iteration.flags_passed,
            iteration.flags_verified,
            iteration.convergence_score * 100.0,
            iteration.corrections_found.len()
        );

        if engine.has_converged(convergence_threshold) {
            println!("Converged!");
            break;
        }

        // If no new corrections found, we're stuck
        if iteration.corrections_found.is_empty() && iteration.mismatches_after > 0 {
            println!("No new corrections found, {} unresolved mismatches", iteration.mismatches_after);
            break;
        }
    }

    engine.generate_report(event_flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_engine_creation() {
        let engine = DiscoveryEngine::new();
        assert!(engine.corrections.is_empty());
        assert!(engine.history.is_empty());
    }

    #[test]
    fn test_diff_discovery() {
        let mut engine = DiscoveryEngine::new();

        let before = vec![0u8; 1000];
        let mut after = before.clone();
        after[100] = 0x01; // Set one bit

        let result = engine.discover_from_diff(&before, &after, &[]);

        assert_eq!(result.total_bit_changes, 1);
        assert_eq!(result.unexpected_changes.len(), 1);
    }
}
