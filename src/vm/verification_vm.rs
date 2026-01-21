//! View model for verification comparison UI
//!
//! Provides filtering, statistics, and data organization for the
//! side-by-side verification comparison view.

use std::collections::{BTreeMap, HashSet};
use crate::util::verification_records::VerificationRecord;

/// Filter status options
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum VerificationFilterStatus {
    #[default]
    All,
    Matching,
    Mismatched,
}

/// Detection category for verification analysis
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionCategory {
    /// Formula missed a manually confirmed collection (PRIMARY indicator of formula error)
    FormulaError,
    /// Auto-detected but not manually confirmed (user can verify)
    PendingVerification,
    /// Auto-detected in a region with no discovered graces (informational)
    UndiscoveredRegion,
}

impl DetectionCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            DetectionCategory::FormulaError => "Formula Error",
            DetectionCategory::PendingVerification => "Pending Verification",
            DetectionCategory::UndiscoveredRegion => "Undiscovered Region",
        }
    }

    /// Whether this category indicates a problem that needs fixing
    pub fn is_error(&self) -> bool {
        matches!(self, DetectionCategory::FormulaError)
    }

    /// Whether this is informational only
    pub fn is_informational(&self) -> bool {
        matches!(self, DetectionCategory::PendingVerification | DetectionCategory::UndiscoveredRegion)
    }
}

/// A flagged detection that needs attention or provides information
#[derive(Debug, Clone)]
pub struct FlaggedDetection {
    pub flag_id: u32,
    pub flag_name: String,
    pub flag_category: String,
    pub region: String,
    pub detection_category: DetectionCategory,
    pub auto_status: bool,
    pub manual_status: Option<bool>,
    /// Detailed description explaining the detection
    pub description: String,
}

// Type alias for backwards compatibility
pub type SuspiciousDetection = FlaggedDetection;

/// View model for verification comparison
#[derive(Debug, Clone, Default)]
pub struct VerificationViewModel {
    /// All verification records for this slot
    pub records: Vec<VerificationRecord>,
    /// Current category filter (None = all categories)
    pub filter_category: Option<String>,
    /// Current status filter
    pub filter_status: VerificationFilterStatus,
    /// Whether this view has data loaded
    pub loaded: bool,
    /// Path to the verification records file
    pub records_path: Option<String>,
    /// Regions with at least one discovered grace
    pub discovered_regions: HashSet<String>,
    /// Suspicious detections (auto-detected but possibly wrong)
    pub suspicious_detections: Vec<SuspiciousDetection>,
}

impl VerificationViewModel {
    /// Create a new view model from records
    pub fn from_records(records: Vec<VerificationRecord>) -> Self {
        Self {
            records,
            filter_category: None,
            filter_status: VerificationFilterStatus::All,
            loaded: true,
            records_path: None,
            discovered_regions: HashSet::new(),
            suspicious_detections: Vec::new(),
        }
    }

    /// Set discovered regions and compute suspicious detections
    pub fn set_discovered_regions(&mut self, regions: HashSet<String>) {
        self.discovered_regions = regions;
        self.compute_suspicious_detections();
    }

    /// Compute flagged detections based on records and discovered regions
    fn compute_suspicious_detections(&mut self) {
        self.suspicious_detections.clear();

        for record in &self.records {
            // PRIMARY: Formula Errors (manual=true, auto=false)
            // User explicitly confirmed collection, but formula doesn't detect it
            // This is the most reliable indicator of a formula problem
            if record.user_marked_complete && !record.webapp_parsed_status {
                let description = format!(
                    "FORMULA ERROR: '{}' (flag {}) was manually confirmed as collected, but the formula does NOT detect it. \
                    This strongly indicates an error in the offset calculation for {} items. \
                    Check: byte offset {}, bit position {}.",
                    record.flag_name, record.flag_id, record.flag_category,
                    record.computed_byte_offset, record.computed_bit_position
                );
                self.suspicious_detections.push(FlaggedDetection {
                    flag_id: record.flag_id,
                    flag_name: record.flag_name.clone(),
                    flag_category: record.flag_category.clone(),
                    region: record.flag_region.clone(),
                    detection_category: DetectionCategory::FormulaError,
                    auto_status: record.webapp_parsed_status,
                    manual_status: Some(record.user_marked_complete),
                    description,
                });
            }

            // INFORMATIONAL: Pending Verification (auto=true, manual=false)
            // Formula detected it, but user hasn't confirmed
            // Could be: 1) User forgot to mark it, 2) No POI exists, 3) True false positive
            if record.webapp_parsed_status && !record.user_marked_complete {
                let description = format!(
                    "Auto-detected '{}' (flag {}) as collected, but not manually confirmed. \
                    This could mean: 1) User forgot to mark it, 2) No POI exists for this flag, or 3) Actual formula error. \
                    User can verify in-game and confirm.",
                    record.flag_name, record.flag_id
                );
                self.suspicious_detections.push(FlaggedDetection {
                    flag_id: record.flag_id,
                    flag_name: record.flag_name.clone(),
                    flag_category: record.flag_category.clone(),
                    region: record.flag_region.clone(),
                    detection_category: DetectionCategory::PendingVerification,
                    auto_status: record.webapp_parsed_status,
                    manual_status: Some(record.user_marked_complete),
                    description,
                });
            }

            // INFORMATIONAL: Undiscovered Region (auto=true, in unvisited region)
            // Only flag if both auto and manual agree it's collected, but region seems unvisited
            if record.webapp_parsed_status && record.user_marked_complete && !self.is_region_discovered(&record.flag_region) {
                let description = format!(
                    "Both auto and manual confirm '{}' (flag {}) as collected, but no graces discovered in '{}'. \
                    This is likely valid (item obtained via drop/trade/different path) but worth noting.",
                    record.flag_name, record.flag_id, record.flag_region
                );
                self.suspicious_detections.push(FlaggedDetection {
                    flag_id: record.flag_id,
                    flag_name: record.flag_name.clone(),
                    flag_category: record.flag_category.clone(),
                    region: record.flag_region.clone(),
                    detection_category: DetectionCategory::UndiscoveredRegion,
                    auto_status: record.webapp_parsed_status,
                    manual_status: Some(record.user_marked_complete),
                    description,
                });
            }
        }

        // Sort by category: FormulaErrors first (most important), then others
        self.suspicious_detections.sort_by(|a, b| {
            let priority_a = if a.detection_category.is_error() { 0 } else { 1 };
            let priority_b = if b.detection_category.is_error() { 0 } else { 1 };
            priority_a.cmp(&priority_b)
        });
    }

    /// Check if a region has been discovered (has at least one grace)
    fn is_region_discovered(&self, region: &str) -> bool {
        // Generic regions that don't require grace discovery
        let generic_regions = ["Various", "Unknown", "Catacombs", "Tunnel", "Cave"];
        if generic_regions.iter().any(|g| region.contains(g)) {
            return true; // Don't flag generic regions
        }

        // Check if this region or a related region is discovered
        // Use case-insensitive contains matching for flexibility
        let region_lower = region.to_lowercase();
        self.discovered_regions.iter().any(|r| {
            let r_lower = r.to_lowercase();
            r_lower.contains(&region_lower) || region_lower.contains(&r_lower)
        })
    }

    /// Get suspicious detections count
    pub fn suspicious_count(&self) -> usize {
        self.suspicious_detections.len()
    }

    /// Get flagged detections count by category
    pub fn suspicious_by_reason(&self) -> BTreeMap<String, usize> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for det in &self.suspicious_detections {
            *counts.entry(det.detection_category.as_str().to_string()).or_insert(0) += 1;
        }
        counts
    }

    /// Get count of formula errors (primary indicator of problems)
    pub fn formula_error_count(&self) -> usize {
        self.suspicious_detections.iter()
            .filter(|d| d.detection_category.is_error())
            .count()
    }

    /// Get count of informational detections (pending verification, etc.)
    pub fn informational_count(&self) -> usize {
        self.suspicious_detections.iter()
            .filter(|d| d.detection_category.is_informational())
            .count()
    }

    /// Get unique categories from records
    pub fn get_categories(&self) -> Vec<String> {
        let mut cats: Vec<_> = self.records.iter()
            .map(|r| r.flag_category.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        cats.sort();
        cats
    }

    /// Get filtered records based on current filter settings
    pub fn get_filtered_records(&self) -> Vec<&VerificationRecord> {
        self.records.iter()
            .filter(|r| {
                // Category filter
                if let Some(cat) = &self.filter_category {
                    if &r.flag_category != cat {
                        return false;
                    }
                }
                // Status filter
                match self.filter_status {
                    VerificationFilterStatus::All => true,
                    VerificationFilterStatus::Matching => r.statuses_align,
                    VerificationFilterStatus::Mismatched => !r.statuses_align,
                }
            })
            .collect()
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> VerificationSummary {
        let total = self.records.len();
        let matches = self.records.iter().filter(|r| r.statuses_align).count();
        let by_category = self.get_category_stats();

        VerificationSummary {
            total,
            matches,
            mismatches: total - matches,
            agreement_rate: if total > 0 {
                (matches as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            by_category,
        }
    }

    /// Get statistics by category
    fn get_category_stats(&self) -> BTreeMap<String, CategoryStats> {
        let mut stats: BTreeMap<String, (usize, usize)> = BTreeMap::new();

        for r in &self.records {
            let entry = stats.entry(r.flag_category.clone()).or_insert((0, 0));
            entry.0 += 1;
            if r.statuses_align {
                entry.1 += 1;
            }
        }

        stats.into_iter()
            .map(|(k, (total, matches))| {
                (k, CategoryStats {
                    total,
                    matches,
                    rate: if total > 0 {
                        (matches as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    },
                })
            })
            .collect()
    }

    /// Check if there are any records
    pub fn has_records(&self) -> bool {
        !self.records.is_empty()
    }

    /// Get count of filtered records
    pub fn filtered_count(&self) -> usize {
        self.get_filtered_records().len()
    }
}

/// Summary statistics for verification comparison
#[derive(Debug, Clone)]
pub struct VerificationSummary {
    pub total: usize,
    pub matches: usize,
    pub mismatches: usize,
    pub agreement_rate: f64,
    pub by_category: BTreeMap<String, CategoryStats>,
}

/// Per-category statistics
#[derive(Debug, Clone)]
pub struct CategoryStats {
    pub total: usize,
    pub matches: usize,
    pub rate: f64,
}
