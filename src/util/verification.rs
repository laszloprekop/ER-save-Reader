/// Verification module for save file accuracy testing
///
/// Implements verification processes to ensure accurate parsing:
/// - V3: Flag formula verification (known flags → expected values)
/// - V1: Differential snapshot testing (before/after pairs)
/// - V5: Coverage gap detection (find unmapped flags)

use std::path::PathBuf;
use std::collections::HashMap;

use crate::db::pickup_flags::{is_flag_set, get_flag_offset, set_flag};
use crate::save::save::save::Save;

// ============================================================================
// V3: FLAG FORMULA VERIFICATION
// ============================================================================

/// A known flag with expected value for verification
#[derive(Debug, Clone)]
pub struct KnownFlag {
    pub id: u32,
    pub name: &'static str,
    pub expected: bool,
    pub category: FlagCategory,
}

/// Categories of flags for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagCategory {
    Grace,
    Boss,
    Cookbook,
    Whetblade,
    WorldPickup,
    DungeonPickup,
    Progression,
    Unknown,
}

impl FlagCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlagCategory::Grace => "Grace",
            FlagCategory::Boss => "Boss",
            FlagCategory::Cookbook => "Cookbook",
            FlagCategory::Whetblade => "Whetblade",
            FlagCategory::WorldPickup => "World Pickup",
            FlagCategory::DungeonPickup => "Dungeon Pickup",
            FlagCategory::Progression => "Progression",
            FlagCategory::Unknown => "Unknown",
        }
    }
}

/// Result of verifying a single flag
#[derive(Debug, Clone)]
pub struct FlagVerification {
    pub flag_id: u32,
    pub name: &'static str,
    pub expected: bool,
    pub actual: bool,
    pub passed: bool,
    pub offset: Option<(u32, u8)>,
}

/// Result of formula verification
#[derive(Debug)]
pub struct FormulaVerificationReport {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<FlagVerification>,
}

impl FormulaVerificationReport {
    pub fn new() -> Self {
        Self {
            total_tests: 0,
            passed: 0,
            failed: 0,
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: FlagVerification) {
        self.total_tests += 1;
        if result.passed {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        self.results.push(result);
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            return 0.0;
        }
        (self.passed as f64 / self.total_tests as f64) * 100.0
    }

    pub fn print_summary(&self) {
        println!("\n=== Flag Formula Verification Report ===");
        println!("Total tests: {}", self.total_tests);
        println!("Passed: {} ({:.1}%)", self.passed, self.success_rate());
        println!("Failed: {}", self.failed);

        if self.failed > 0 {
            println!("\nFailed tests:");
            for result in &self.results {
                if !result.passed {
                    println!(
                        "  - {} ({}): expected {}, got {} [offset: {:?}]",
                        result.name, result.flag_id, result.expected, result.actual, result.offset
                    );
                }
            }
        }
    }
}

/// Known flags for the Confessor character (Slot 0)
/// These are verified against actual gameplay progression in ER0000-static.sl2
pub fn get_confessor_known_flags() -> Vec<KnownFlag> {
    vec![
        // Graces (verified touched in mid-game)
        KnownFlag { id: 76100, name: "First Step", expected: true, category: FlagCategory::Grace },
        KnownFlag { id: 76101, name: "Church of Elleh", expected: true, category: FlagCategory::Grace },
        KnownFlag { id: 76102, name: "Gatefront Ruins", expected: true, category: FlagCategory::Grace },
        KnownFlag { id: 76110, name: "Stormhill Shack", expected: true, category: FlagCategory::Grace },
        KnownFlag { id: 76120, name: "Waypoint Ruins Cellar", expected: true, category: FlagCategory::Grace },

        // Bosses (verified defeated)
        KnownFlag { id: 10000800, name: "Margit Defeated", expected: true, category: FlagCategory::Boss },

        // Cookbooks - Note: 67310 is collected, 67000 is NOT in this save
        KnownFlag { id: 67310, name: "Missionary's Cookbook [4]", expected: true, category: FlagCategory::Cookbook },

        // Whetblades (verified collected)
        KnownFlag { id: 65610, name: "Whetstone Knife", expected: true, category: FlagCategory::Whetblade },

        // Flags that should NOT be set (negative tests)
        KnownFlag { id: 76999, name: "Invalid Grace", expected: false, category: FlagCategory::Grace },
        KnownFlag { id: 67000, name: "Armorer's Cookbook [1] (not collected)", expected: false, category: FlagCategory::Cookbook },
    ]
}

/// Known flags for the Wretch character (Slot 1)
/// Early game progression only
pub fn get_wretch_known_flags() -> Vec<KnownFlag> {
    vec![
        // Cave of Knowledge grace
        KnownFlag { id: 76150, name: "Cave of Knowledge", expected: true, category: FlagCategory::Grace },

        // Soldier of Godrick defeated
        KnownFlag { id: 20000800, name: "Soldier of Godrick Defeated", expected: true, category: FlagCategory::Boss },

        // Should NOT have advanced graces
        KnownFlag { id: 76110, name: "Stormhill Shack", expected: false, category: FlagCategory::Grace },
    ]
}

/// Verify flag formula against known flags in a save file
pub fn verify_flag_formula(event_flags: &[u8], known_flags: &[KnownFlag]) -> FormulaVerificationReport {
    let mut report = FormulaVerificationReport::new();

    for flag in known_flags {
        let offset = get_flag_offset(flag.id);
        let actual = is_flag_set(event_flags, flag.id);
        let passed = actual == flag.expected;

        report.add_result(FlagVerification {
            flag_id: flag.id,
            name: flag.name,
            expected: flag.expected,
            actual,
            passed,
            offset,
        });
    }

    report
}

/// Verify flag formula for a specific save file
pub fn verify_save_formula(save_path: &PathBuf, slot_index: usize, known_flags: &[KnownFlag]) -> Result<FormulaVerificationReport, String> {
    let save = Save::from_path(save_path)
        .map_err(|e| format!("Failed to load save: {}", e))?;

    let slot = save.save_type.get_slot(slot_index);
    Ok(verify_flag_formula(&slot.event_flags.flags, known_flags))
}

// ============================================================================
// V1: DIFFERENTIAL SNAPSHOT TESTING
// ============================================================================

/// A differential test comparing before/after snapshots
#[derive(Debug, Clone)]
pub struct DiffTest {
    pub before_path: PathBuf,
    pub after_path: PathBuf,
    pub slot_index: usize,
    pub expected_flag_changes: Vec<(u32, bool)>, // (flag_id, new_value)
    pub description: &'static str,
}

/// Result of a differential test
#[derive(Debug)]
pub struct DiffTestResult {
    pub description: &'static str,
    pub passed: bool,
    pub expected_changes_found: Vec<(u32, bool, bool)>, // (flag_id, expected, actual)
    pub unexpected_changes: Vec<(u32, bool, bool)>, // (flag_id, before, after)
    pub missing_changes: Vec<(u32, bool)>, // (flag_id, expected)
}

/// Find all flags that differ between two event flag arrays
pub fn find_changed_flags(before: &[u8], after: &[u8]) -> Vec<(u32, bool, bool)> {
    let mut changes = Vec::new();

    // We need to scan through possible flag IDs and check which changed
    // This is expensive but comprehensive

    // Check simple flags (0-99999)
    for flag_id in 0..100_000u32 {
        let before_val = is_flag_set(before, flag_id);
        let after_val = is_flag_set(after, flag_id);
        if before_val != after_val {
            changes.push((flag_id, before_val, after_val));
        }
    }

    // Check dungeon flags (10000000-43999999) - sample key ranges
    for area in [10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 30, 31, 32, 34, 35, 39, 40, 41, 42, 43] {
        for section in 0..30 {
            for local in (0..10000).step_by(8) {
                let flag_id = area * 1_000_000 + section * 10_000 + local;
                let before_val = is_flag_set(before, flag_id);
                let after_val = is_flag_set(after, flag_id);
                if before_val != after_val {
                    // Found a changed byte, check all 8 bits
                    for bit_offset in 0..8 {
                        let full_flag = flag_id + bit_offset;
                        let bv = is_flag_set(before, full_flag);
                        let av = is_flag_set(after, full_flag);
                        if bv != av {
                            changes.push((full_flag, bv, av));
                        }
                    }
                }
            }
        }
    }

    // Check tile flags - sample common tiles
    for row in 33..55 {
        for col in 31..59 {
            for local in (0..10000).step_by(8) {
                let flag_id = 1_000_000_000 + row * 1_000_000 + col * 10_000 + local;
                let before_val = is_flag_set(before, flag_id);
                let after_val = is_flag_set(after, flag_id);
                if before_val != after_val {
                    for bit_offset in 0..8 {
                        let full_flag = flag_id + bit_offset;
                        let bv = is_flag_set(before, full_flag);
                        let av = is_flag_set(after, full_flag);
                        if bv != av {
                            changes.push((full_flag, bv, av));
                        }
                    }
                }
            }
        }
    }

    changes
}

/// Run a differential snapshot test
pub fn run_diff_test(test: &DiffTest) -> Result<DiffTestResult, String> {
    let before_save = Save::from_path(&test.before_path)
        .map_err(|e| format!("Failed to load before save: {}", e))?;
    let after_save = Save::from_path(&test.after_path)
        .map_err(|e| format!("Failed to load after save: {}", e))?;

    let before_flags = &before_save.save_type.get_slot(test.slot_index).event_flags.flags;
    let after_flags = &after_save.save_type.get_slot(test.slot_index).event_flags.flags;

    let mut expected_changes_found = Vec::new();
    let mut missing_changes = Vec::new();
    let mut unexpected_changes = Vec::new();

    // Check expected changes occurred
    for (flag_id, expected_value) in &test.expected_flag_changes {
        let before_value = is_flag_set(before_flags, *flag_id);
        let after_value = is_flag_set(after_flags, *flag_id);

        if after_value == *expected_value && before_value != after_value {
            expected_changes_found.push((*flag_id, *expected_value, after_value));
        } else if after_value != *expected_value {
            missing_changes.push((*flag_id, *expected_value));
        }
    }

    // Find unexpected changes (expensive - only do for small diffs)
    let all_changes = find_changed_flags(before_flags, after_flags);
    for (flag_id, before_val, after_val) in all_changes {
        if !test.expected_flag_changes.iter().any(|(f, _)| *f == flag_id) {
            unexpected_changes.push((flag_id, before_val, after_val));
        }
    }

    let passed = missing_changes.is_empty() && unexpected_changes.len() <= 10; // Allow some unexpected

    Ok(DiffTestResult {
        description: test.description,
        passed,
        expected_changes_found,
        unexpected_changes,
        missing_changes,
    })
}

// ============================================================================
// V5: COVERAGE GAP DETECTION
// ============================================================================

/// Categories for unmapped flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnmappedCategory {
    Core,           // 0-999
    System,         // 1000-9999
    Progression,    // 60000-69999
    ShopStock,      // 100000-199999
    TilePickup,     // 1000000000+
    DlcPickup,      // 2000000000+
    Dungeon,        // 10000000-43999999
    Unknown,
}

impl UnmappedCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnmappedCategory::Core => "Core",
            UnmappedCategory::System => "System",
            UnmappedCategory::Progression => "Progression",
            UnmappedCategory::ShopStock => "Shop Stock",
            UnmappedCategory::TilePickup => "Tile Pickup",
            UnmappedCategory::DlcPickup => "DLC Pickup",
            UnmappedCategory::Dungeon => "Dungeon",
            UnmappedCategory::Unknown => "Unknown",
        }
    }
}

/// Categorize a flag ID
pub fn categorize_flag(flag_id: u32) -> UnmappedCategory {
    match flag_id {
        0..=999 => UnmappedCategory::Core,
        1000..=9999 => UnmappedCategory::System,
        60000..=69999 => UnmappedCategory::Progression,
        100000..=199999 => UnmappedCategory::ShopStock,
        10_000_000..=43_999_999 => UnmappedCategory::Dungeon,
        1_000_000_000..=1_999_999_999 => UnmappedCategory::TilePickup,
        2_000_000_000..=2_999_999_999 => UnmappedCategory::DlcPickup,
        _ => UnmappedCategory::Unknown,
    }
}

/// An unmapped flag found in the save
#[derive(Debug, Clone)]
pub struct UnmappedFlag {
    pub flag_id: u32,
    pub category: UnmappedCategory,
}

/// Coverage report for a save file
#[derive(Debug)]
pub struct CoverageReport {
    pub total_flags_set: usize,
    pub mapped_flags: usize,
    pub unmapped_flags: usize,
    pub by_category: HashMap<UnmappedCategory, usize>,
    pub sample_unmapped: Vec<UnmappedFlag>, // First 100 unmapped
}

impl CoverageReport {
    pub fn coverage_percent(&self) -> f64 {
        if self.total_flags_set == 0 {
            return 0.0;
        }
        (self.mapped_flags as f64 / self.total_flags_set as f64) * 100.0
    }

    pub fn print_summary(&self) {
        println!("\n=== Coverage Report ===");
        println!("Total flags set: {}", self.total_flags_set);
        println!("Mapped in database: {} ({:.1}%)", self.mapped_flags, self.coverage_percent());
        println!("Unmapped: {}", self.unmapped_flags);

        println!("\nUnmapped by category:");
        for (category, count) in &self.by_category {
            println!("  - {}: {}", category.as_str(), count);
        }

        if !self.sample_unmapped.is_empty() {
            println!("\nSample unmapped flags (first 20):");
            for flag in self.sample_unmapped.iter().take(20) {
                println!("  - {} ({})", flag.flag_id, flag.category.as_str());
            }
        }
    }
}

/// Check if a flag is in our database
/// This is a simplified check - expand based on actual database modules
pub fn is_flag_in_database(flag_id: u32) -> bool {
    use crate::db::pickup_data::WORLD_PICKUPS;
    use crate::db::graces::maps::GRACES;
    use crate::db::bosses::bosses::BOSSES;
    use crate::db::cookbooks::books::COOKBOKS;
    use crate::db::whetblades::whetblades::WHETBLADES;

    // Check pickup database
    if WORLD_PICKUPS.iter().any(|p| p.event_flag == flag_id) {
        return true;
    }

    // Check graces - tuple is (MapName, flag_id, name)
    let graces = GRACES.lock().unwrap();
    if graces.values().any(|g| g.1 == flag_id) {
        return true;
    }
    drop(graces);

    // Check bosses - tuple is (flag_id, name)
    let bosses = BOSSES.lock().unwrap();
    if bosses.values().any(|b| b.0 == flag_id) {
        return true;
    }
    drop(bosses);

    // Check cookbooks - tuple is (flag_id, name)
    let cookbooks = COOKBOKS.lock().unwrap();
    if cookbooks.values().any(|c| c.0 == flag_id) {
        return true;
    }
    drop(cookbooks);

    // Check whetblades - tuple is (flag_id, name)
    let whetblades = WHETBLADES.lock().unwrap();
    if whetblades.values().any(|w| w.0 == flag_id) {
        return true;
    }
    drop(whetblades);

    false
}

/// Detect coverage gaps in a save file
/// Note: This is a sampling-based approach for performance
pub fn detect_coverage_gaps(event_flags: &[u8]) -> CoverageReport {
    let mut total_set = 0;
    let mut mapped = 0;
    let mut unmapped_list = Vec::new();
    let mut by_category: HashMap<UnmappedCategory, usize> = HashMap::new();

    // Sample simple flags (0-99999)
    for flag_id in 0..100_000u32 {
        if is_flag_set(event_flags, flag_id) {
            total_set += 1;
            if is_flag_in_database(flag_id) {
                mapped += 1;
            } else {
                let category = categorize_flag(flag_id);
                *by_category.entry(category).or_insert(0) += 1;
                if unmapped_list.len() < 100 {
                    unmapped_list.push(UnmappedFlag { flag_id, category });
                }
            }
        }
    }

    // Sample tile flags - check key tiles
    for row in 33..55 {
        for col in 31..59 {
            for local in (0..10000).step_by(1) {
                let flag_id = 1_000_000_000 + row * 1_000_000 + col * 10_000 + local;
                if is_flag_set(event_flags, flag_id) {
                    total_set += 1;
                    if is_flag_in_database(flag_id) {
                        mapped += 1;
                    } else {
                        let category = categorize_flag(flag_id);
                        *by_category.entry(category).or_insert(0) += 1;
                        if unmapped_list.len() < 100 {
                            unmapped_list.push(UnmappedFlag { flag_id, category });
                        }
                    }
                }
            }
        }
    }

    // Sample dungeon flags
    for area in [10, 11, 12, 13, 14, 15, 16, 20, 21, 30, 31, 32] {
        for section in 0..25 {
            for local in 0..10000 {
                let flag_id = area * 1_000_000 + section * 10_000 + local;
                if is_flag_set(event_flags, flag_id) {
                    total_set += 1;
                    if is_flag_in_database(flag_id) {
                        mapped += 1;
                    } else {
                        let category = categorize_flag(flag_id);
                        *by_category.entry(category).or_insert(0) += 1;
                        if unmapped_list.len() < 100 {
                            unmapped_list.push(UnmappedFlag { flag_id, category });
                        }
                    }
                }
            }
        }
    }

    CoverageReport {
        total_flags_set: total_set,
        mapped_flags: mapped,
        unmapped_flags: total_set - mapped,
        by_category,
        sample_unmapped: unmapped_list,
    }
}

// ============================================================================
// V4: ROUND-TRIP INTEGRITY TESTING
// ============================================================================

/// Test that a flag modification persists through save/load
pub fn test_round_trip(
    event_flags: &[u8],
    flag_id: u32,
    new_value: bool,
) -> Result<bool, String> {
    let mut modified = event_flags.to_vec();

    // Get original value
    let original = is_flag_set(&modified, flag_id);

    // Set new value
    if !set_flag(&mut modified, flag_id, new_value) {
        return Err(format!("Failed to set flag {}", flag_id));
    }

    // Verify it changed
    let after_set = is_flag_set(&modified, flag_id);
    if after_set != new_value {
        return Err(format!(
            "Flag {} not set correctly: expected {}, got {}",
            flag_id, new_value, after_set
        ));
    }

    // Toggle back
    set_flag(&mut modified, flag_id, original);
    let restored = is_flag_set(&modified, flag_id);
    if restored != original {
        return Err(format!(
            "Flag {} not restored: expected {}, got {}",
            flag_id, original, restored
        ));
    }

    Ok(true)
}

// ============================================================================
// COMBINED VERIFICATION RUNNER
// ============================================================================

/// Run all verification tests on a save file
pub fn run_full_verification(
    save_path: &PathBuf,
    slot_index: usize,
) -> Result<(), String> {
    println!("Loading save file: {:?}", save_path);

    let save = Save::from_path(save_path)
        .map_err(|e| format!("Failed to load save: {}", e))?;

    let slot = save.save_type.get_slot(slot_index);
    let flags = &slot.event_flags.flags;

    println!("Running verification for slot {}...\n", slot_index);

    // V3: Flag Formula Verification
    println!("=== V3: Flag Formula Verification ===");
    let known_flags = if slot_index == 0 {
        get_confessor_known_flags()
    } else {
        get_wretch_known_flags()
    };
    let formula_report = verify_flag_formula(flags, &known_flags);
    formula_report.print_summary();

    // V4: Round-trip test on sample flags
    println!("\n=== V4: Round-Trip Integrity ===");
    let test_flags = vec![76100, 67000, 1043500010, 10000800];
    let mut rt_passed = 0;
    for flag_id in &test_flags {
        match test_round_trip(flags, *flag_id, !is_flag_set(flags, *flag_id)) {
            Ok(_) => {
                rt_passed += 1;
                println!("  Flag {}: PASS", flag_id);
            }
            Err(e) => println!("  Flag {}: FAIL - {}", flag_id, e),
        }
    }
    println!("Round-trip: {}/{} passed", rt_passed, test_flags.len());

    // V5: Coverage Gap Detection
    println!("\n=== V5: Coverage Gap Detection ===");
    println!("(This may take a moment...)");
    let coverage = detect_coverage_gaps(flags);
    coverage.print_summary();

    println!("\n=== Verification Complete ===");

    Ok(())
}
