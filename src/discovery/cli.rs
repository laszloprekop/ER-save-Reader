/// Discovery CLI Commands
///
/// Provides command-line interface for running discovery operations:
/// - batch-analyze: Process all snapshots
/// - status: Show discovery store status
/// - promotable: List discoveries ready for promotion
/// - promote: Promote confirmed discoveries to ground truth

use std::path::{Path, PathBuf};

use super::{
    batch_analyze_and_save, get_snapshot_summary,
    run_discovery_workflow,
    DiscoveryStore, ConsensusBuilder, ConsensusStatus,
    GroundTruthUpdater, UpdateConfig,
    RelationshipGraph, CorroborationEngine, CorroborationStatus,
};
use super::test_cases::{TestCaseValidator, DynamicTestCaseValidator, print_validation_report};
use crate::save::save::save::Save;

/// Default paths
const DEFAULT_SNAPSHOT_DIR: &str = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging";
const DEFAULT_SAVE_PATH: &str = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2";
const DEFAULT_STORE_PATH: &str = "discoveries.json";
const DEFAULT_GROUND_TRUTH: &str = "ground_truth_offsets.json";
const DEFAULT_RECORDS_PATH: &str = "../elden-map/server/data/verification-records.jsonl";

/// Run discovery CLI with given arguments
pub fn run_cli(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "analyze" | "a" => cmd_analyze(&args[1..]),
        "validate" | "v" => cmd_validate(&args[1..]),
        "probe" => cmd_probe(&args[1..]),
        "inventory" | "inv" => cmd_inventory(&args[1..]),
        "batch-analyze" | "batch" => cmd_batch_analyze(&args[1..]),
        "status" | "s" => cmd_status(&args[1..]),
        "promotable" | "p" => cmd_promotable(&args[1..]),
        "promote" => cmd_promote(&args[1..]),
        "corroborate" | "corr" => cmd_corroborate(&args[1..]),
        "graph" | "g" => cmd_graph(&args[1..]),
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        other => Err(format!("Unknown command: {}. Use 'discovery help' for usage.", other)),
    }
}

fn print_help() {
    println!("Event Flag Discovery System");
    println!();
    println!("USAGE:");
    println!("    er-save-editor discovery <COMMAND>");
    println!();
    println!("COMMANDS:");
    println!("    analyze          Analyze a single save file slot");
    println!("    validate         Run curated test cases against save slots");
    println!("    probe            Directly read bytes at specific offsets");
    println!("    inventory        Search inventory for items by ID or name");
    println!("    batch-analyze    Process all snapshot pairs and persist discoveries");
    println!("    status           Show discovery store statistics");
    println!("    promotable       List discoveries ready for promotion");
    println!("    promote          Promote confirmed discoveries to ground truth");
    println!("    corroborate      Multi-point validation using relationship graph");
    println!("    graph            Show relationship graph statistics");
    println!("    help             Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    er-save-editor discovery analyze 0");
    println!("    er-save-editor discovery analyze 0 /path/to/ER0000.sl2");
    println!("    er-save-editor discovery validate 2 3 4    # Validate slots 2, 3, 4");
    println!("    er-save-editor discovery validate --all    # Validate all defined slots");
    println!("    er-save-editor discovery batch-analyze");
    println!("    er-save-editor discovery status");
    println!("    er-save-editor discovery promotable");
    println!("    er-save-editor discovery promote --dry-run");
}

/// Analyze a single save file slot
fn cmd_analyze(args: &[String]) -> Result<(), String> {
    let slot_index: usize = args.get(0)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let save_path = args.get(1)
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SAVE_PATH));

    println!("Analyzing save file: {:?}", save_path);
    println!("Slot index: {}", slot_index);
    println!();

    if !save_path.exists() {
        return Err(format!("Save file not found: {:?}", save_path));
    }

    let result = run_discovery_workflow(&save_path, slot_index)
        .map_err(|e| format!("Discovery failed: {}", e))?;

    println!();
    println!("Discovery Summary:");
    println!("  Segments found: {}", result.segments_found);
    println!("  Flag bytes: {} ({:.1}%)", result.flag_bytes,
        result.flag_bytes as f64 / (result.flag_bytes + result.empty_bytes) as f64 * 100.0);
    println!("  Verification: {} passed, {} failed",
        result.verification_passed, result.verification_failed);
    println!("  Coverage: {:.1}% ({}/{} bits)",
        result.coverage.coverage_percent,
        result.coverage.covered_bits,
        result.coverage.total_set_bits);

    if !result.failed_flags.is_empty() {
        println!();
        println!("Failed flags (offset mismatch):");
        for (flag_id, name, expected, actual) in result.failed_flags.iter().take(10) {
            println!("  {} ({}): expected {}, got {}", name, flag_id, expected, actual);
        }
        if result.failed_flags.len() > 10 {
            println!("  ... and {} more", result.failed_flags.len() - 10);
        }
    }

    Ok(())
}

/// Validate test cases against save file
///
/// Supports two modes:
/// 1. Static mode (default): Uses hardcoded test cases from build_test_suite()
/// 2. Dynamic mode (--records): Loads test cases from verification-records.jsonl
fn cmd_validate(args: &[String]) -> Result<(), String> {
    // Parse --save argument
    let save_path = args.iter()
        .position(|a| a == "--save" || a == "-s")
        .and_then(|i| args.get(i + 1))
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SAVE_PATH));

    // Parse --records argument for dynamic mode
    let records_path = args.iter()
        .position(|a| a == "--records" || a == "-r")
        .and_then(|i| args.get(i + 1))
        .map(|s| PathBuf::from(s));

    // Check for --dynamic flag (uses default records path)
    let use_dynamic = args.iter().any(|a| a == "--dynamic" || a == "-d") || records_path.is_some();

    if !save_path.exists() {
        return Err(format!("Save file not found: {:?}", save_path));
    }

    // Check for --all flag
    let validate_all = args.iter().any(|a| a == "--all" || a == "-a");

    // Filter out flag arguments to get slot numbers
    let slots: Vec<usize> = args.iter()
        .filter(|s| !s.starts_with('-') && !s.starts_with("--"))
        .filter(|s| {
            // Also filter out values that follow flags
            let prev_idx = args.iter().position(|x| x == *s).unwrap_or(0);
            if prev_idx > 0 {
                let prev = &args[prev_idx - 1];
                !(prev == "--save" || prev == "-s" || prev == "--records" || prev == "-r")
            } else {
                true
            }
        })
        .filter_map(|s| s.parse().ok())
        .collect();

    if use_dynamic {
        // Dynamic mode: load from verification records
        let records = records_path.unwrap_or_else(|| PathBuf::from(DEFAULT_RECORDS_PATH));

        if !records.exists() {
            return Err(format!("Verification records not found: {:?}", records));
        }

        println!("=== DYNAMIC VALIDATION MODE ===");
        println!("Records: {:?}", records);
        println!("Save: {:?}", save_path);

        let validator = DynamicTestCaseValidator::from_records(&records)?;
        println!("Loaded {} test cases across {} slots",
            validator.total_test_count(), validator.slot_count());
        println!();

        run_validation(&validator, &save_path, &slots, validate_all)?;
    } else {
        // Static mode: use hardcoded test cases
        println!("=== STATIC VALIDATION MODE ===");
        println!("(Use --dynamic or --records <path> for verification-record-based testing)");
        println!("Save: {:?}", save_path);
        println!();

        let validator = TestCaseValidator::new();

        if slots.is_empty() && !validate_all {
            println!("Usage: discovery validate [slots...] [--all] [--dynamic] [--records <path>]");
            println!();
            println!("Options:");
            println!("  --all, -a           Validate all slots");
            println!("  --dynamic, -d       Use verification records (default path)");
            println!("  --records, -r PATH  Use verification records from PATH");
            println!("  --save, -s PATH     Use save file at PATH");
            println!();
            println!("Available slots with static test cases:");
            for (slot_index, suite) in validator.suite().slots.iter() {
                let true_count = suite.known_true.len();
                let false_count = suite.known_false.len();
                println!("  Slot {}: {} ({} true, {} false tests)",
                    slot_index, suite.character_name, true_count, false_count);
            }
            return Ok(());
        }

        run_validation(&validator, &save_path, &slots, validate_all)?;
    }

    Ok(())
}

/// Common validation logic for both static and dynamic validators
fn run_validation<V: Validator>(
    validator: &V,
    save_path: &Path,
    slots: &[usize],
    validate_all: bool,
) -> Result<(), String> {
    if validate_all {
        println!("Validating all slots with test cases...");

        let results = validator.validate_all_slots(save_path)
            .map_err(|e| format!("Validation failed: {}", e))?;

        for result in &results {
            print_validation_report(result);
        }

        // Summary
        let total_passed: usize = results.iter().map(|r| r.passed).sum();
        let total_failed: usize = results.iter().map(|r| r.failed).sum();
        let total_errors: usize = results.iter().map(|r| r.errors).sum();
        let total_tests: usize = results.iter().map(|r| r.total_tests).sum();

        println!();
        println!("═══════════════════════════════════════════════════════════════");
        println!("Overall Summary: {}/{} passed ({:.1}%)",
            total_passed, total_tests,
            if total_tests > 0 { total_passed as f64 / total_tests as f64 * 100.0 } else { 0.0 });
        println!("  Failed: {} | Errors: {}", total_failed, total_errors);
    } else if !slots.is_empty() {
        println!("Validating slots: {:?}", slots);

        for &slot_index in slots {
            match validator.validate_slot(save_path, slot_index) {
                Ok(result) => print_validation_report(&result),
                Err(e) => println!("Slot {} error: {}", slot_index, e),
            }
        }
    }

    Ok(())
}

/// Trait to abstract over static and dynamic validators
trait Validator {
    fn validate_slot(&self, save_path: &Path, slot_index: usize)
        -> Result<super::test_cases::SlotValidationResult, String>;
    fn validate_all_slots(&self, save_path: &Path)
        -> Result<Vec<super::test_cases::SlotValidationResult>, String>;
}

impl Validator for TestCaseValidator {
    fn validate_slot(&self, save_path: &Path, slot_index: usize)
        -> Result<super::test_cases::SlotValidationResult, String> {
        TestCaseValidator::validate_slot(self, save_path, slot_index)
    }
    fn validate_all_slots(&self, save_path: &Path)
        -> Result<Vec<super::test_cases::SlotValidationResult>, String> {
        TestCaseValidator::validate_all_slots(self, save_path)
    }
}

impl Validator for DynamicTestCaseValidator {
    fn validate_slot(&self, save_path: &Path, slot_index: usize)
        -> Result<super::test_cases::SlotValidationResult, String> {
        DynamicTestCaseValidator::validate_slot(self, save_path, slot_index)
    }
    fn validate_all_slots(&self, save_path: &Path)
        -> Result<Vec<super::test_cases::SlotValidationResult>, String> {
        DynamicTestCaseValidator::validate_all_slots(self, save_path)
    }
}

/// Directly probe bytes at specific offsets for debugging
fn cmd_probe(args: &[String]) -> Result<(), String> {
    // Parse --save argument
    let save_path = args.iter()
        .position(|a| a == "--save" || a == "-s")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());

    // Filter out --save and its value from args
    let filtered_args: Vec<&String> = args.iter()
        .enumerate()
        .filter(|(i, a)| {
            let is_save_flag = *a == "--save" || *a == "-s";
            let is_save_value = args.get(i.saturating_sub(1))
                .map(|prev| prev == "--save" || prev == "-s")
                .unwrap_or(false);
            !is_save_flag && !is_save_value
        })
        .map(|(_, a)| a)
        .collect();

    let slot_index: usize = filtered_args.get(0)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);

    // Parse byte offsets (can be decimal or hex with 0x prefix)
    let offsets: Vec<usize> = filtered_args.iter().skip(1)
        .filter_map(|s| {
            if s.starts_with("0x") || s.starts_with("0X") {
                usize::from_str_radix(&s[2..], 16).ok()
            } else {
                s.parse().ok()
            }
        })
        .collect();

    if offsets.is_empty() {
        // Default: probe the contested grace offsets
        println!("Usage: discovery probe <slot> <offset1> [offset2] ... [--save <path>]");
        println!("  Offsets can be decimal or hex (0x prefix)");
        println!();
        println!("Probing default grace region (3258-3270) on slot {}...", slot_index);
        return probe_bytes_at_offsets(slot_index, &(3258..=3270).collect::<Vec<_>>(), save_path);
    }

    probe_bytes_at_offsets(slot_index, &offsets, save_path)
}

fn probe_bytes_at_offsets(slot_index: usize, offsets: &[usize], save_path: Option<&str>) -> Result<(), String> {
    use crate::save::save::save::Save;

    let save_path = save_path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SAVE_PATH));
    if !save_path.exists() {
        return Err(format!("Save file not found: {:?}", save_path));
    }

    let save = Save::from_path(&save_path)
        .map_err(|e| format!("Failed to load save: {}", e))?;

    let slot = save.save_type.get_slot(slot_index);
    let event_flags = &slot.event_flags.flags;

    println!("Probing slot {} ({} bytes of event flags)", slot_index, event_flags.len());
    println!();

    for &offset in offsets {
        if offset >= event_flags.len() {
            println!("0x{:04x} ({:5}): OUT OF BOUNDS", offset, offset);
            continue;
        }

        let byte = event_flags[offset];
        let bits: Vec<&str> = (0..8)
            .rev()
            .map(|b| if (byte >> b) & 1 == 1 { "1" } else { "0" })
            .collect();

        // Show which flags would map to this byte (for 76xxx block)
        // Block 76000 base = 3250, so byte 3262 = flags 76096-76103
        let block_76000_base = 3250_usize;
        let flags_at_byte = if offset >= block_76000_base {
            let relative_byte = offset - block_76000_base;
            let first_flag = 76000 + (relative_byte * 8);
            format!("flags {}-{}", first_flag, first_flag + 7)
        } else {
            String::new()
        };

        println!("0x{:04x} ({:5}): {:02x} = {} {} {}",
            offset, offset, byte, bits.join(""),
            if byte != 0 { "<- has bits set" } else { "" },
            flags_at_byte);
    }

    Ok(())
}

/// Check inventory for specific items
fn cmd_inventory(args: &[String]) -> Result<(), String> {
    use crate::save::save::save::Save;

    // Parse --save argument
    let save_path = args.iter()
        .position(|a| a == "--save" || a == "-s")
        .and_then(|i| args.get(i + 1))
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SAVE_PATH));

    // Filter out --save and its value from args
    let filtered_args: Vec<&String> = args.iter()
        .enumerate()
        .filter(|(i, a)| {
            let is_save_flag = *a == "--save" || *a == "-s";
            let is_save_value = args.get(i.saturating_sub(1))
                .map(|prev| prev == "--save" || prev == "-s")
                .unwrap_or(false);
            !is_save_flag && !is_save_value
        })
        .map(|(_, a)| a)
        .collect();

    let slot_index: usize = filtered_args.get(0)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Search term can be item ID or name substring
    let search_term = filtered_args.get(1).map(|s| s.as_str()).unwrap_or("");

    if !save_path.exists() {
        return Err(format!("Save file not found: {:?}", save_path));
    }

    let save = Save::from_path(&save_path)
        .map_err(|e| format!("Failed to load save: {}", e))?;

    let slot = save.save_type.get_slot(slot_index);
    let storage = &slot.equip_inventory_data;

    // Convert character name from UTF-16
    let character_name_raw = slot.player_game_data.character_name;
    let mut character_name_trimmed: [u16; 0x10] = [0; 0x10];
    for (i, ch) in character_name_raw.iter().enumerate() {
        if *ch == 0 { break; }
        character_name_trimmed[i] = *ch;
    }
    let character_name = String::from_utf16(&character_name_trimmed).unwrap_or_else(|_| "Unknown".to_string());

    println!("Checking inventory for slot {} ({})", slot_index, character_name);
    println!("Search term: '{}'", search_term);
    println!();

    // Parse search term as item ID if it's a number
    let search_id: Option<u32> = search_term.parse().ok()
        .or_else(|| {
            if search_term.starts_with("0x") {
                u32::from_str_radix(&search_term[2..], 16).ok()
            } else {
                None
            }
        });

    let search_lower = search_term.to_lowercase();
    let mut found_count = 0;

    // Check common items (consumables, materials, cookbooks, etc.)
    println!("=== Common Items ===");
    for item in &storage.common_items {
        if item.ga_item_handle == 0 || item.quantity == 0 {
            continue;
        }

        let item_id = item.ga_item_handle & 0x0FFFFFFF;
        let item_type = (item.ga_item_handle >> 28) & 0xF;

        // Get item name if available
        let item_name = crate::db::item_name::item_name::ITEM_NAME
            .lock()
            .unwrap()
            .get(&item_id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Unknown (type {})", item_type));

        let matches = search_term.is_empty()
            || search_id.map(|id| item_id == id).unwrap_or(false)
            || item_name.to_lowercase().contains(&search_lower);

        if matches {
            println!("  ID: {:8} | GA: 0x{:08X} | Qty: {:3} | {}",
                item_id, item.ga_item_handle, item.quantity, item_name);
            found_count += 1;
        }
    }

    // Check key items
    println!();
    println!("=== Key Items ===");
    for item in &storage.key_items {
        if item.ga_item_handle == 0 || item.quantity == 0 {
            continue;
        }

        let item_id = item.ga_item_handle & 0x0FFFFFFF;
        let item_type = (item.ga_item_handle >> 28) & 0xF;

        let item_name = crate::db::item_name::item_name::ITEM_NAME
            .lock()
            .unwrap()
            .get(&item_id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Unknown (type {})", item_type));

        let matches = search_term.is_empty()
            || search_id.map(|id| item_id == id).unwrap_or(false)
            || item_name.to_lowercase().contains(&search_lower);

        if matches {
            println!("  ID: {:8} | GA: 0x{:08X} | Qty: {:3} | {}",
                item_id, item.ga_item_handle, item.quantity, item_name);
            found_count += 1;
        }
    }

    println!();
    println!("Found {} matching items", found_count);

    Ok(())
}

/// Process all snapshots and persist discoveries
fn cmd_batch_analyze(args: &[String]) -> Result<(), String> {
    let snapshot_dir = args.get(0)
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SNAPSHOT_DIR));

    let store_path = args.get(1)
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STORE_PATH));

    println!("Snapshot directory: {:?}", snapshot_dir);
    println!("Store path: {:?}", store_path);
    println!();

    // Show summary first
    let summary = get_snapshot_summary(&snapshot_dir);
    println!("{}", summary);
    println!();

    // Run batch analysis
    let result = batch_analyze_and_save(&snapshot_dir, &store_path)
        .map_err(|e| format!("Batch analysis failed: {}", e))?;

    println!();
    println!("Results:");
    println!("  Files scanned: {}", result.files_scanned);
    println!("  Pairs found: {}", result.pairs_found);
    println!("  Pairs processed: {}", result.pairs_processed);
    println!("  New discoveries: {}", result.discoveries_persisted);

    if !result.errors.is_empty() {
        println!();
        println!("Errors ({}):", result.errors.len());
        for err in result.errors.iter().take(10) {
            println!("  - {}", err);
        }
    }

    Ok(())
}

/// Show discovery store status
fn cmd_status(args: &[String]) -> Result<(), String> {
    let store_path = args.get(0)
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STORE_PATH));

    let store = DiscoveryStore::load_or_create(&store_path)
        .map_err(|e| format!("Failed to load store: {}", e))?;

    let summary = store.summary();
    println!("{}", summary);

    // Also show consensus analysis
    let consensus = ConsensusBuilder::default();
    let report = consensus.analyze_store(&store);
    println!("{}", report);

    Ok(())
}

/// List discoveries ready for promotion
fn cmd_promotable(args: &[String]) -> Result<(), String> {
    let store_path = args.get(0)
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STORE_PATH));

    let store = DiscoveryStore::load_or_create(&store_path)
        .map_err(|e| format!("Failed to load store: {}", e))?;

    let consensus = ConsensusBuilder::default();
    let promotable = consensus.get_promotable(&store);

    if promotable.is_empty() {
        println!("No discoveries ready for promotion.");
        println!();
        println!("Requirements: 2+ observations, 80% agreement, 75% confidence");
        return Ok(());
    }

    println!("Discoveries ready for promotion ({}):", promotable.len());
    println!();
    println!("{:<12} {:<8} {:<8} {:<10} {:<8} {}",
        "Flag ID", "Byte", "Bit", "Confidence", "Obs", "Category");
    println!("{}", "-".repeat(70));

    for result in &promotable {
        if let Some((byte, bit)) = result.best_offset {
            let discovery = store.get(result.flag_id);
            let category = discovery
                .and_then(|d| d.flag_category.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("Unknown");

            println!("{:<12} 0x{:<6x} {:<8} {:<10.1}% {:<8} {}",
                result.flag_id,
                byte,
                bit,
                result.weighted_confidence * 100.0,
                result.observation_count,
                category);
        }
    }

    println!();
    println!("Run 'discovery promote' to update ground_truth_offsets.json");

    Ok(())
}

/// Promote confirmed discoveries to ground truth
fn cmd_promote(args: &[String]) -> Result<(), String> {
    let dry_run = args.iter().any(|a| a == "--dry-run" || a == "-n");

    let store_path = args.iter()
        .find(|a| !a.starts_with('-'))
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STORE_PATH));

    let store = DiscoveryStore::load_or_create(&store_path)
        .map_err(|e| format!("Failed to load store: {}", e))?;

    let consensus = ConsensusBuilder::default();
    let promotable = consensus.get_promotable(&store);

    if promotable.is_empty() {
        println!("No discoveries ready for promotion.");
        return Ok(());
    }

    println!("Found {} discoveries ready for promotion", promotable.len());
    println!();

    if dry_run {
        println!("DRY RUN - No changes will be made");
        println!();
        for result in &promotable {
            if let Some((byte, bit)) = result.best_offset {
                println!("  Would update flag {}: byte 0x{:x}, bit {} (conf: {:.0}%)",
                    result.flag_id, byte, bit, result.weighted_confidence * 100.0);
            }
        }
        println!();
        println!("Run without --dry-run to apply changes");
        return Ok(());
    }

    // Create updater and stage updates
    let config = UpdateConfig {
        ground_truth_path: PathBuf::from(DEFAULT_GROUND_TRUTH),
        backup_dir: PathBuf::from("backups"),
        min_confidence: 0.75,
        min_observations: 2,
        recalculate_block_bases: true,
        min_flags_for_base_recalc: 3,
    };

    let mut updater = GroundTruthUpdater::new(config);

    // Stage from discoveries
    let discoveries: Vec<_> = promotable.iter()
        .filter_map(|r| store.get(r.flag_id))
        .collect();

    updater.stage_from_discoveries(&discoveries);

    println!("Staged {} updates", updater.pending_count());

    // Apply updates
    let result = updater.apply_updates()
        .map_err(|e| format!("Failed to apply updates: {}", e))?;

    println!();
    println!("Update complete:");
    println!("  Backup created: {:?}", result.backup_path);
    println!("  Flags updated: {}", result.flags_updated);
    println!("  Flags added: {}", result.flags_added);
    println!("  Block bases recalculated: {}", result.block_bases_recalculated);

    if !result.errors.is_empty() {
        println!();
        println!("Errors:");
        for err in &result.errors {
            println!("  - {}", err);
        }
    }

    Ok(())
}

/// Show relationship graph statistics
fn cmd_graph(_args: &[String]) -> Result<(), String> {
    let graph = RelationshipGraph::load_default()
        .map_err(|e| format!("Failed to load relationship graph: {}", e))?;

    let summary = graph.summary();
    println!("{}", summary);

    // Show some example corroboration pairs
    let pairs = graph.get_corroboration_pairs();
    if !pairs.is_empty() {
        println!();
        println!("Example dual-formula corroboration pairs:");
        println!("{:<14} {:<10} {}", "Tile Flag", "Block Flag", "Item");
        println!("{}", "-".repeat(50));

        for pair in pairs.iter().take(10) {
            let item = pair.item_name.as_deref().unwrap_or("Unknown");
            println!("{:<14} {:<10} {}", pair.tile_flag, pair.block_flag, item);
        }

        if pairs.len() > 10 {
            println!("... and {} more pairs", pairs.len() - 10);
        }
    }

    // Show highly connected flags
    let high_conn = graph.flags_with_min_connections(5);
    if !high_conn.is_empty() {
        println!();
        println!("Highly connected flags (5+ connections):");
        for (flag, count) in high_conn.iter().take(10) {
            println!("  Flag {:>10}: {} connections", flag, count);
        }
    }

    Ok(())
}

/// Multi-point corroboration validation
fn cmd_corroborate(args: &[String]) -> Result<(), String> {
    // Parse arguments
    let save_path = args.iter()
        .position(|a| a == "--save" || a == "-s")
        .and_then(|i| args.get(i + 1))
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SAVE_PATH));

    let slot_index: usize = args.iter()
        .position(|a| a == "--slot")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Check for --all flag first
    let check_all = args.iter().any(|a| a == "--all" || a == "-a");

    // Check for specific flag ID (exclude values after --slot and --save)
    let flag_id: Option<u32> = if check_all {
        None
    } else {
        // Find positions of --slot and --save to exclude their values
        let slot_pos = args.iter().position(|a| a == "--slot");
        let save_pos = args.iter().position(|a| a == "--save" || a == "-s");

        args.iter()
            .enumerate()
            .find(|(i, a)| {
                // Skip values that follow --slot or --save
                if slot_pos.map(|p| *i == p + 1).unwrap_or(false) { return false; }
                if save_pos.map(|p| *i == p + 1).unwrap_or(false) { return false; }
                // Must be a positive integer that doesn't start with -
                !a.starts_with('-') && a.parse::<u32>().is_ok()
            })
            .and_then(|(_, s)| s.parse().ok())
    };

    if !save_path.exists() {
        return Err(format!("Save file not found: {:?}", save_path));
    }

    // Load save file
    let save = Save::from_path(&save_path)
        .map_err(|e| format!("Failed to load save: {}", e))?;

    let slot = save.save_type.get_slot(slot_index);
    let event_flags = &slot.event_flags.flags;

    // Load corroboration engine
    let engine = CorroborationEngine::load_default()
        .map_err(|e| format!("Failed to load corroboration engine: {}", e))?;

    if let Some(flag_id) = flag_id {
        // Check single flag
        println!("Corroboration check for flag {}:", flag_id);
        println!();

        let result = engine.check_corroboration(flag_id, true, event_flags);

        println!("Status: {}", result.status);
        println!("Agreement: {:.1}%", result.agreement_ratio * 100.0);
        println!("Confidence adjustment: {:+.2}", result.confidence_adjustment);

        if !result.related_checks.is_empty() {
            println!();
            println!("Related flag checks:");
            for check in &result.related_checks {
                let status = if check.agrees { "OK" } else { "MISMATCH" };
                let actual = check.actual_set.map(|b| if b { "SET" } else { "UNSET" })
                    .unwrap_or("N/A");
                println!("  {} ({}) - Expected: {}, Actual: {} [{}]",
                    check.flag_id,
                    check.relationship_type,
                    if check.expected_set { "SET" } else { "UNSET" },
                    actual,
                    status);
            }
        }

        if let Some(ref df) = result.dual_formula {
            println!();
            println!("Dual-formula check:");
            println!("  Tile flag {}: {:?}", df.tile_flag, df.tile_set);
            println!("  Block flag {}: {:?}", df.block_flag, df.block_set);
            println!("  Agreement: {}", if df.both_agree { "YES" } else { "NO" });
            if let Some(ref name) = df.item_name {
                println!("  Item: {}", name);
            }
        }
    } else if check_all {
        // Validate all corroboration pairs
        println!("Validating all corroboration pairs against slot {}...", slot_index);
        println!();

        let result = engine.validate_all_pairs(event_flags);
        println!("{}", result);

        // Show contradictions
        let contradictions: Vec<_> = result.results.iter()
            .filter(|r| r.status == super::PairStatus::Contradicts)
            .collect();

        if !contradictions.is_empty() {
            println!();
            println!("Contradictions found ({}):", contradictions.len());
            for r in contradictions.iter().take(10) {
                println!("  Tile {} ({:?}) vs Block {} ({:?}) - {}",
                    r.tile_flag,
                    r.tile_set,
                    r.block_flag,
                    r.block_set,
                    r.item_name.as_deref().unwrap_or("Unknown"));
            }
        }
    } else {
        println!("Usage:");
        println!("  discovery corroborate <flag_id>     Check single flag corroboration");
        println!("  discovery corroborate --all         Validate all corroboration pairs");
        println!();
        println!("Options:");
        println!("  --save, -s <path>    Save file path (default: {})", DEFAULT_SAVE_PATH);
        println!("  --slot <index>       Slot index (default: 0)");
        println!();
        println!("Examples:");
        println!("  discovery corroborate 67650");
        println!("  discovery corroborate 67650 --slot 5");
        println!("  discovery corroborate --all --slot 0");
    }

    Ok(())
}
