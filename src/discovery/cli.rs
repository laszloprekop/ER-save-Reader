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
    EventGraph,
    ParamFlagDb, FlagCategory,
    UnifiedFlagDb, SourceConfidence,
};
use super::test_cases::{TestCaseValidator, DynamicTestCaseValidator, print_validation_report};
use crate::save::save::save::Save;

/// Default paths
const DEFAULT_SNAPSHOT_DIR: &str = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging";
const DEFAULT_SAVE_PATH: &str = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/ER0000.sl2";
const DEFAULT_STORE_PATH: &str = "discoveries.json";
const DEFAULT_GROUND_TRUTH: &str = "ground_truth_offsets.json";
const DEFAULT_RECORDS_PATH: &str = "../elden-map/server/data/flag-correlation-candidates.jsonl";

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
        "event-graph" | "eg" => cmd_event_graph(&args[1..]),
        "batch-validate" | "bv" => cmd_batch_validate(&args[1..]),
        "param-extract" | "pe" => cmd_param_extract(&args[1..]),
        "param-query" | "pq" => cmd_param_query(&args[1..]),
        "unified" | "u" => cmd_unified(&args[1..]),
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
    println!("    event-graph      Query EMEVD event graph for flag triggers");
    println!("    batch-validate   Validate all EMEVD-backed flags against save data");
    println!("    param-extract    Extract flags from regulation-bin XML params");
    println!("    param-query      Query the param flags database");
    println!("    unified          Query unified flag database (catalog + params + EMEVD)");
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
    println!("    er-save-editor discovery event-graph 76100     # Query flag triggers");
    println!("    er-save-editor discovery event-graph --stats   # Show event graph stats");
    println!("    er-save-editor discovery batch-validate 0      # Validate all EMEVD flags on slot 0");
    println!("    er-save-editor discovery batch-validate 0 --context boss_defeat");
    println!("    er-save-editor discovery param-extract        # Extract flags from regulation-bin");
    println!("    er-save-editor discovery param-query 400000   # Query block 400000 flags");
    println!("    er-save-editor discovery unified --build      # Build unified database");
    println!("    er-save-editor discovery unified 76100        # Query flag in unified db");
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

    // Load corroboration engine with event graph for EMEVD validation
    let engine = CorroborationEngine::load_with_event_graph()
        .or_else(|_| {
            // Fall back to without event graph if it fails to load
            println!("Note: Event graph not available, using relationship graph only");
            CorroborationEngine::load_default()
        })
        .map_err(|e| format!("Failed to load corroboration engine: {}", e))?;

    if engine.has_event_graph() {
        if let Some(summary) = engine.event_graph_summary() {
            println!("{}", summary);
        }
    }

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

        // Show event graph validation (EMEVD evidence)
        if let Some(ref eg) = result.event_graph {
            println!();
            println!("Event graph (EMEVD) validation:");
            if eg.has_trigger {
                println!("  Found in EMEVD: YES ({} triggers)", eg.trigger_count);
                if let Some(ref ctx) = eg.trigger_context {
                    println!("  Primary context: {}", ctx);
                }
                if !eg.source_files.is_empty() {
                    println!("  Sources: {}", eg.source_files.join(", "));
                }
                if let Some(ref chain) = eg.progression_chain {
                    println!("  Progression chain: {}", chain);
                }
                println!("  Confidence boost: +{:.2}", eg.confidence_boost);
            } else {
                println!("  Found in EMEVD: NO");
                println!("  (Flag may be set via other mechanisms)");
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

/// Query EMEVD event graph for flag triggers and validation
fn cmd_event_graph(args: &[String]) -> Result<(), String> {
    let show_stats = args.iter().any(|a| a == "--stats" || a == "-s");
    let show_contexts = args.iter().any(|a| a == "--contexts" || a == "-c");
    let show_chains = args.iter().any(|a| a == "--chains");

    // Check for specific flag ID
    let flag_id: Option<u32> = args.iter()
        .filter(|a| !a.starts_with('-'))
        .find_map(|s| s.parse().ok());

    // Load event graph
    let graph = EventGraph::load_default()
        .map_err(|e| format!("Failed to load event graph: {}", e))?;

    let summary = graph.summary();

    if show_stats {
        println!("=== EMEVD Event Graph Statistics ===");
        println!();
        println!("Files parsed:        {}", summary.files_parsed);
        println!("Unique flags:        {}", summary.total_flags);
        println!("Total triggers:      {}", summary.total_triggers);
        println!("Dependencies:        {}", summary.total_dependencies);
        println!("Entity mappings:     {}", summary.entity_mappings);
        println!("Progression chains:  {}", summary.progression_chains);
        println!();

        // Show trigger context distribution
        let contexts = graph.list_contexts();
        println!("Trigger contexts ({}):", contexts.len());
        for ctx in contexts.iter().take(15) {
            let count = graph.get_flags_by_context(ctx).map(|f| f.len()).unwrap_or(0);
            println!("  {:25} {:>6} flags", ctx, count);
        }
        if contexts.len() > 15 {
            println!("  ... and {} more contexts", contexts.len() - 15);
        }

        return Ok(());
    }

    if show_contexts {
        println!("=== Trigger Contexts ===");
        println!();
        let contexts = graph.list_contexts();
        for ctx in &contexts {
            let count = graph.get_flags_by_context(ctx).map(|f| f.len()).unwrap_or(0);
            println!("{:30} {:>6} flags", ctx, count);
        }
        return Ok(());
    }

    if show_chains {
        println!("=== Progression Chains ===");
        println!();

        let remembrances = graph.get_chains_by_type("remembrance");
        println!("Remembrance chains ({}):", remembrances.len());
        for chain in remembrances.iter().take(10) {
            println!("  Boss defeat {:>6} -> Possession {:>6}",
                chain.boss_defeat.unwrap_or(0),
                chain.possession_flag.unwrap_or(0));
        }
        if remembrances.len() > 10 {
            println!("  ... and {} more", remembrances.len() - 10);
        }
        println!();

        let map_frags = graph.get_chains_by_type("map_fragment");
        println!("Map fragment chains ({}):", map_frags.len());
        for chain in map_frags.iter().take(10) {
            if let Some(ref params) = chain.params.get(0..2) {
                println!("  Discovery {:>6} -> Possession {:>6}",
                    params.get(0).unwrap_or(&0_i64),
                    params.get(1).unwrap_or(&0_i64));
            }
        }
        if map_frags.len() > 10 {
            println!("  ... and {} more", map_frags.len() - 10);
        }

        return Ok(());
    }

    if let Some(flag_id) = flag_id {
        // Query specific flag
        println!("=== Event Graph Query: Flag {} ===", flag_id);
        println!();

        if graph.has_trigger(flag_id) {
            println!("Status: FOUND in EMEVD");
            println!();

            if let Some(triggers) = graph.get_triggers(flag_id) {
                println!("Triggers ({}):", triggers.len());
                for (i, trigger) in triggers.iter().enumerate() {
                    println!("  [{}] Event {}: {} ({})",
                        i + 1,
                        trigger.event_id,
                        trigger.action,
                        trigger.trigger_context);
                    println!("      Source: {}", trigger.source_file);
                    if let Some(entity) = trigger.entity_id {
                        println!("      Entity: {}", entity);
                    }
                }
            }

            // Check dependencies
            if let Some(deps) = graph.get_dependencies(flag_id) {
                if !deps.is_empty() {
                    println!();
                    println!("Dependencies ({}):", deps.len());
                    for dep in deps.iter().take(10) {
                        println!("  Requires flag {} ({})", dep.required_flag, dep.condition_type);
                    }
                }
            }

            // Check what this flag enables
            if let Some(enables) = graph.get_enables(flag_id) {
                if !enables.is_empty() {
                    println!();
                    println!("Enables ({}):", enables.len());
                    for en in enables.iter().take(10) {
                        println!("  Flag {} ({})", en.enabled_flag, en.relationship);
                    }
                }
            }

            // Check progression chains
            if let Some(chain) = graph.find_remembrance_chain(flag_id) {
                println!();
                println!("Part of remembrance chain:");
                println!("  Boss defeat: {:?}", chain.boss_defeat);
                println!("  Possession flag: {:?}", chain.possession_flag);
            }

            // Check entity mapping
            if let Some(entity_id) = graph.find_entity_for_flag(flag_id) {
                if let Some(mapping) = graph.get_entity_flags(entity_id) {
                    println!();
                    println!("Entity mapping:");
                    println!("  Entity ID: {}", entity_id);
                    println!("  Type: {}", mapping.entity_type);
                    println!("  Map tile: {}", mapping.map_tile);
                }
            }
        } else {
            println!("Status: NOT FOUND in EMEVD");
            println!();
            println!("This flag has no SetEventFlagID triggers in the parsed EMEVD files.");
            println!("This could mean:");
            println!("  - Flag is set through a different mechanism (param files, etc.)");
            println!("  - Flag ID may be incorrect");
            println!("  - The extraction may have missed this pattern");
        }

        return Ok(());
    }

    // Show usage
    println!("EMEVD Event Graph Query Tool");
    println!();
    println!("USAGE:");
    println!("    discovery event-graph <flag_id>    Query specific flag");
    println!("    discovery event-graph --stats      Show statistics");
    println!("    discovery event-graph --contexts   List all trigger contexts");
    println!("    discovery event-graph --chains     Show progression chains");
    println!();
    println!("EXAMPLES:");
    println!("    discovery event-graph 76100        # First Step grace");
    println!("    discovery event-graph 9100         # Godrick remembrance");
    println!("    discovery event-graph --stats");
    println!();
    println!("Current graph: {} flags, {} triggers from {} files",
        summary.total_flags, summary.total_triggers, summary.files_parsed);

    Ok(())
}

/// Batch validate all EMEVD-backed flags against save file
fn cmd_batch_validate(args: &[String]) -> Result<(), String> {
    use crate::save::save::save::Save;
    use crate::db::pickup_flags::{get_flag_offset, get_flag_verification_status, VerificationStatus};
    use std::collections::HashMap;

    // Parse --save argument
    let save_path = args.iter()
        .position(|a| a == "--save" || a == "-s")
        .and_then(|i| args.get(i + 1))
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SAVE_PATH));

    // Parse --context filter
    let context_filter = args.iter()
        .position(|a| a == "--context" || a == "-c")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.to_string());

    // Parse --block filter (e.g., --block 9000 for flags 9000-9999)
    let block_filter: Option<u32> = args.iter()
        .position(|a| a == "--block" || a == "-b")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok());

    // Parse slot index
    let slot_index: usize = args.iter()
        .filter(|a| !a.starts_with('-'))
        .filter(|a| {
            let idx = args.iter().position(|x| x == *a).unwrap_or(0);
            if idx > 0 {
                let prev = &args[idx - 1];
                !(prev == "--save" || prev == "-s" || prev == "--context" || prev == "-c" || prev == "--block" || prev == "-b")
            } else {
                true
            }
        })
        .find_map(|s| s.parse().ok())
        .unwrap_or(0);

    // Check for --unset flag (only show unset flags)
    let show_unset_only = args.iter().any(|a| a == "--unset" || a == "-u");
    // Check for --set flag (only show set flags)
    let show_set_only = args.iter().any(|a| a == "--set");
    // Check for --invalid flag (only show flags with no formula)
    let show_invalid_only = args.iter().any(|a| a == "--invalid" || a == "-i");

    if !save_path.exists() {
        return Err(format!("Save file not found: {:?}", save_path));
    }

    // Load event graph
    let graph = EventGraph::load_default()
        .map_err(|e| format!("Failed to load event graph: {}", e))?;

    // Load save
    let save = Save::from_path(&save_path)
        .map_err(|e| format!("Failed to load save: {}", e))?;

    let slot = save.save_type.get_slot(slot_index);
    let event_flags = &slot.event_flags.flags;

    // Convert character name
    let character_name_raw = slot.player_game_data.character_name;
    let mut character_name_trimmed: [u16; 0x10] = [0; 0x10];
    for (i, ch) in character_name_raw.iter().enumerate() {
        if *ch == 0 { break; }
        character_name_trimmed[i] = *ch;
    }
    let character_name = String::from_utf16(&character_name_trimmed).unwrap_or_else(|_| "Unknown".to_string());

    println!("=== EMEVD Flag Batch Validation ===");
    println!();
    println!("Save: {:?}", save_path);
    println!("Slot: {} ({})", slot_index, character_name);
    if let Some(ref ctx) = context_filter {
        println!("Filter: context = {}", ctx);
    }
    if let Some(block) = block_filter {
        println!("Filter: block = {}-{}", block, block + 999);
    }
    println!();

    // Collect all flags with triggers
    let all_flags = graph.get_all_flag_ids();
    let mut stats = BatchValidationStats::default();
    let mut by_context: HashMap<String, ContextStats> = HashMap::new();
    let mut by_block: HashMap<u32, BlockStats> = HashMap::new();

    for &flag_id in &all_flags {
        // Apply block filter
        if let Some(block) = block_filter {
            if flag_id < block || flag_id >= block + 1000 {
                continue;
            }
        }

        // Apply context filter
        let context = graph.get_trigger_context(flag_id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if let Some(ref filter) = context_filter {
            if !context.contains(filter) {
                continue;
            }
        }

        stats.total_flags += 1;

        // Try to calculate offset
        let offset_result = get_flag_offset(flag_id);
        let verification_status = get_flag_verification_status(flag_id);

        let (has_formula, is_set) = if let Some((byte_off, bit)) = offset_result {
            let set = if (byte_off as usize) < event_flags.len() {
                (event_flags[byte_off as usize] & (1 << bit)) != 0
            } else {
                false
            };
            (true, set)
        } else {
            (false, false)
        };

        // Update global stats
        if has_formula {
            stats.with_formula += 1;
            if is_set {
                stats.set_flags += 1;
            } else {
                stats.unset_flags += 1;
            }
            match verification_status {
                VerificationStatus::Verified => stats.verified_formula += 1,
                VerificationStatus::Calculated => stats.calculated_formula += 1,
                VerificationStatus::Unverified => stats.unverified_formula += 1,
                VerificationStatus::Unknown => stats.unknown_formula += 1,
            }
        } else {
            stats.no_formula += 1;
        }

        // Update per-context stats
        let ctx_stats = by_context.entry(context.clone()).or_default();
        ctx_stats.total += 1;
        if has_formula {
            ctx_stats.with_formula += 1;
            if is_set { ctx_stats.set_flags += 1; }
        } else {
            ctx_stats.no_formula += 1;
        }

        // Update per-block stats (1000-flag blocks)
        let block_id = (flag_id / 1000) * 1000;
        let block_stats = by_block.entry(block_id).or_default();
        block_stats.total += 1;
        if has_formula {
            block_stats.with_formula += 1;
            if is_set { block_stats.set_flags += 1; }
        } else {
            block_stats.no_formula += 1;
        }

        // Output individual flags if filtered
        let show_flag = if show_invalid_only {
            !has_formula
        } else if show_unset_only {
            has_formula && !is_set
        } else if show_set_only {
            has_formula && is_set
        } else {
            false
        };

        if show_flag {
            let status_str = if has_formula {
                if is_set { "SET" } else { "UNSET" }
            } else {
                "NO_FORMULA"
            };
            println!("  {:>10} [{:10}] {} ({})",
                flag_id, status_str, context,
                graph.get_triggers(flag_id)
                    .and_then(|t| t.first())
                    .map(|t| t.source_file.as_str())
                    .unwrap_or("unknown"));
        }
    }

    // Print summary
    println!("=== Summary ===");
    println!();
    println!("Total flags with triggers:  {:>6}", stats.total_flags);
    println!("With formula:               {:>6} ({:.1}%)",
        stats.with_formula, stats.with_formula as f64 / stats.total_flags as f64 * 100.0);
    println!("  - Verified:               {:>6}", stats.verified_formula);
    println!("  - Calculated:             {:>6}", stats.calculated_formula);
    println!("  - Unverified:             {:>6}", stats.unverified_formula);
    println!("  - Unknown:                {:>6}", stats.unknown_formula);
    println!("No formula:                 {:>6}", stats.no_formula);
    println!();
    println!("Set flags:                  {:>6} ({:.1}%)",
        stats.set_flags, stats.set_flags as f64 / stats.with_formula.max(1) as f64 * 100.0);
    println!("Unset flags:                {:>6}", stats.unset_flags);

    // Print by-context breakdown
    println!();
    println!("=== By Context ===");
    let mut contexts: Vec<_> = by_context.iter().collect();
    contexts.sort_by(|a, b| b.1.total.cmp(&a.1.total));
    for (ctx, cstats) in contexts.iter().take(15) {
        let set_pct = if cstats.with_formula > 0 {
            cstats.set_flags as f64 / cstats.with_formula as f64 * 100.0
        } else { 0.0 };
        println!("{:25} {:>5} flags, {:>5} formula, {:>5} set ({:.0}%)",
            ctx, cstats.total, cstats.with_formula, cstats.set_flags, set_pct);
    }
    if contexts.len() > 15 {
        println!("... and {} more contexts", contexts.len() - 15);
    }

    // Print by-block breakdown (only blocks with no formula or high unset rate)
    println!();
    println!("=== Blocks Needing Formula ===");
    let mut blocks: Vec<_> = by_block.iter()
        .filter(|(_, s)| s.no_formula > 0 || (s.with_formula > 0 && s.set_flags == 0))
        .collect();
    blocks.sort_by(|a, b| b.1.no_formula.cmp(&a.1.no_formula));
    for (block, bstats) in blocks.iter().take(20) {
        println!("{:>8}-{:<8}: {:>4} flags, {:>4} no formula, {:>4} formula ({} set)",
            block, *block + 999, bstats.total, bstats.no_formula, bstats.with_formula, bstats.set_flags);
    }
    if blocks.len() > 20 {
        println!("... and {} more blocks", blocks.len() - 20);
    }

    Ok(())
}

#[derive(Default)]
struct BatchValidationStats {
    total_flags: usize,
    with_formula: usize,
    no_formula: usize,
    set_flags: usize,
    unset_flags: usize,
    verified_formula: usize,
    calculated_formula: usize,
    unverified_formula: usize,
    unknown_formula: usize,
}

#[derive(Default)]
struct ContextStats {
    total: usize,
    with_formula: usize,
    no_formula: usize,
    set_flags: usize,
}

#[derive(Default)]
struct BlockStats {
    total: usize,
    with_formula: usize,
    no_formula: usize,
    set_flags: usize,
}

// Default param directory
const DEFAULT_PARAM_DIR: &str = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/regulation-bin";
const DEFAULT_PARAM_DB_PATH: &str = "param_flags.json";

/// Extract flags from regulation-bin XML param files
fn cmd_param_extract(args: &[String]) -> Result<(), String> {
    let param_dir = args.iter()
        .position(|a| a == "--dir" || a == "-d")
        .and_then(|i| args.get(i + 1))
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PARAM_DIR));

    let output_path = args.iter()
        .position(|a| a == "--output" || a == "-o")
        .and_then(|i| args.get(i + 1))
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PARAM_DB_PATH));

    if !param_dir.exists() {
        return Err(format!("Param directory not found: {:?}", param_dir));
    }

    println!("=== Param Flag Extraction ===");
    println!();
    println!("Source: {:?}", param_dir);
    println!("Output: {:?}", output_path);
    println!();

    // Extract flags
    let db = ParamFlagDb::extract_from_directory(&param_dir)
        .map_err(|e| format!("Extraction failed: {}", e))?;

    println!();
    db.print_summary();

    // Save to JSON
    db.save_to_json(&output_path)
        .map_err(|e| format!("Failed to save: {}", e))?;

    println!();
    println!("Saved to {:?}", output_path);

    Ok(())
}

/// Query the param flags database
fn cmd_param_query(args: &[String]) -> Result<(), String> {
    let db_path = args.iter()
        .position(|a| a == "--db")
        .and_then(|i| args.get(i + 1))
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PARAM_DB_PATH));

    // Check for --blocks to list midrange blocks
    let show_blocks = args.iter().any(|a| a == "--blocks" || a == "-b");
    // Check for --stats
    let show_stats = args.iter().any(|a| a == "--stats" || a == "-s");
    // Check for --bosses
    let show_bosses = args.iter().any(|a| a == "--bosses");
    // Check for --param filter
    let param_filter = args.iter()
        .position(|a| a == "--param" || a == "-p")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.to_string());

    // Check for block number or flag ID
    let query_id: Option<u32> = args.iter()
        .filter(|a| !a.starts_with('-'))
        .filter(|a| {
            let idx = args.iter().position(|x| x == *a).unwrap_or(0);
            if idx > 0 {
                let prev = &args[idx - 1];
                !(prev == "--db" || prev == "--param" || prev == "-p")
            } else {
                true
            }
        })
        .find_map(|s| s.parse().ok());

    // Try to load existing database or extract fresh
    let db = if db_path.exists() {
        ParamFlagDb::load_from_json(&db_path)
            .map_err(|e| format!("Failed to load param database: {}", e))?
    } else {
        println!("Param database not found at {:?}", db_path);
        println!("Extracting from default location...");
        println!();

        let db = ParamFlagDb::extract_from_directory(DEFAULT_PARAM_DIR)
            .map_err(|e| format!("Extraction failed: {}", e))?;

        db.save_to_json(&db_path)
            .map_err(|e| format!("Failed to save: {}", e))?;

        db
    };

    if show_stats {
        db.print_summary();
        return Ok(());
    }

    if show_blocks {
        println!("=== Midrange Blocks with Param Flags ===");
        println!();
        let blocks = db.midrange_blocks();
        println!("{:<10} {:>6}", "Block", "Count");
        println!("{}", "-".repeat(20));
        for block in &blocks {
            let count = db.flags_in_block(*block).len();
            println!("{:<10} {:>6}", block, count);
        }
        println!();
        println!("Total: {} blocks, {} midrange flags",
            blocks.len(),
            db.flags_in_category(FlagCategory::Midrange).len());
        return Ok(());
    }

    if show_bosses {
        println!("=== Boss Defeat Flags ===");
        println!();
        let game_area_flags = db.flags_from_param("GameAreaParam");
        let mut boss_entries: Vec<_> = game_area_flags.iter()
            .filter_map(|f| {
                db.get_boss_name(f.flag_id).map(|name| (f.flag_id, name))
            })
            .collect();
        boss_entries.sort_by_key(|(id, _)| *id);

        println!("{:<12} {}", "Flag ID", "Boss Name");
        println!("{}", "-".repeat(60));
        for (flag_id, name) in &boss_entries {
            println!("{:<12} {}", flag_id, name);
        }
        println!();
        println!("Total: {} named boss flags", boss_entries.len());
        return Ok(());
    }

    if let Some(ref param) = param_filter {
        println!("=== Flags from {} ===", param);
        println!();
        let flags = db.flags_from_param(param);
        if flags.is_empty() {
            println!("No flags found from param: {}", param);
            println!();
            println!("Available params:");
            for (name, _) in &db.stats().by_param {
                println!("  {}", name);
            }
            return Ok(());
        }

        println!("{:<12} {:>10} {}", "Flag ID", "Category", "Sources");
        println!("{}", "-".repeat(50));
        for flag in flags.iter().take(50) {
            let sources: Vec<_> = flag.sources.iter()
                .map(|s| s.field_name().to_string())
                .collect();
            println!("{:<12} {:>10} {}",
                flag.flag_id,
                flag.category.name(),
                sources.join(", "));
        }
        if flags.len() > 50 {
            println!("... and {} more flags", flags.len() - 50);
        }
        println!();
        println!("Total: {} flags from {}", flags.len(), param);
        return Ok(());
    }

    if let Some(id) = query_id {
        // Check if this is a block query (exact 1000 multiple) or flag query
        if id % 1000 == 0 && id >= 100000 && id < 1000000 {
            // Block query
            println!("=== Block {} Flags ===", id);
            println!();
            let flags = db.flags_in_block(id);
            if flags.is_empty() {
                println!("No flags found in block {}", id);
                return Ok(());
            }

            println!("{:<12} {}", "Flag ID", "Sources");
            println!("{}", "-".repeat(60));
            for flag in &flags {
                let sources: Vec<_> = flag.sources.iter()
                    .map(|s| format!("{}:{}", s.param_name(), s.field_name()))
                    .collect();
                println!("{:<12} {}", flag.flag_id, sources.join(", "));
            }
            println!();
            println!("Total: {} flags in block {}", flags.len(), id);
        } else {
            // Single flag query
            println!("=== Flag {} ===", id);
            println!();
            if let Some(flag) = db.get(id) {
                println!("Category: {}", flag.category.name());
                println!();
                println!("Sources:");
                for source in &flag.sources {
                    println!("  {} (row {}, field: {})",
                        source.param_name(),
                        source.row_id(),
                        source.field_name());
                    if let super::param_flags::ParamSource::GameArea { boss_name: Some(ref name), .. } = source {
                        println!("    Boss: {}", name);
                    }
                }

                // Also check if in event graph
                if let Ok(graph) = EventGraph::load_default() {
                    if graph.has_trigger(id) {
                        println!();
                        println!("EMEVD: Found in event graph");
                        if let Some(ctx) = graph.get_trigger_context(id) {
                            println!("  Context: {}", ctx);
                        }
                    }
                }
            } else {
                println!("Flag {} not found in param database", id);
                println!();
                println!("This flag may exist in:");
                println!("  - Event graph (EMEVD files)");
                println!("  - Other game mechanisms");
            }
        }
        return Ok(());
    }

    // Show usage
    println!("Param Flags Database Query");
    println!();
    println!("USAGE:");
    println!("    discovery param-query <block>     Query block (e.g., 400000)");
    println!("    discovery param-query <flag_id>   Query specific flag");
    println!("    discovery param-query --stats     Show database statistics");
    println!("    discovery param-query --blocks    List all midrange blocks");
    println!("    discovery param-query --bosses    List boss defeat flags with names");
    println!("    discovery param-query --param <name>  List flags from specific param");
    println!();
    println!("OPTIONS:");
    println!("    --db <path>       Path to param_flags.json");
    println!("    --stats, -s       Show summary statistics");
    println!("    --blocks, -b      List midrange blocks");
    println!("    --bosses          List boss defeat flags");
    println!("    --param, -p NAME  Filter by param name");
    println!();
    println!("EXAMPLES:");
    println!("    discovery param-query --stats");
    println!("    discovery param-query 400000      # Block 400000 flags");
    println!("    discovery param-query 510120      # Specific flag");
    println!("    discovery param-query --param GameAreaParam");
    println!("    discovery param-query --bosses");
    println!();
    println!("Current database: {} flags", db.len());

    Ok(())
}

const DEFAULT_UNIFIED_DB_PATH: &str = "unified_flags.json";

/// Unified database commands
fn cmd_unified(args: &[String]) -> Result<(), String> {
    let db_path = args.iter()
        .position(|a| a == "--db")
        .and_then(|i| args.get(i + 1))
        .map(|s| PathBuf::from(s))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_UNIFIED_DB_PATH));

    // Check for --build flag
    let do_build = args.iter().any(|a| a == "--build" || a == "-b");
    // Check for --stats flag
    let show_stats = args.iter().any(|a| a == "--stats" || a == "-s");
    // Check for --search
    let search_query = args.iter()
        .position(|a| a == "--search")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.to_string());
    // Check for --needs-formula
    let show_needs_formula = args.iter().any(|a| a == "--needs-formula" || a == "-f");
    // Check for --category filter
    let category_filter = args.iter()
        .position(|a| a == "--category" || a == "-c")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.to_string());
    // Check for --context filter
    let context_filter = args.iter()
        .position(|a| a == "--context")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.to_string());
    // Check for --high-confidence
    let show_high_conf = args.iter().any(|a| a == "--high" || a == "--high-confidence");

    // Query flag ID
    let query_id: Option<u32> = args.iter()
        .filter(|a| !a.starts_with('-'))
        .filter(|a| {
            let idx = args.iter().position(|x| x == *a).unwrap_or(0);
            if idx > 0 {
                let prev = &args[idx - 1];
                !(prev == "--db" || prev == "--search" || prev == "--category" || prev == "-c" || prev == "--context")
            } else {
                true
            }
        })
        .find_map(|s| s.parse().ok());

    if do_build {
        println!("=== Building Unified Flag Database ===");
        println!();

        // Ensure param_flags.json exists
        if !Path::new("param_flags.json").exists() {
            println!("Extracting param flags first...");
            let param_db = ParamFlagDb::extract_from_directory(DEFAULT_PARAM_DIR)
                .map_err(|e| format!("Failed to extract params: {}", e))?;
            param_db.save_to_json("param_flags.json")
                .map_err(|e| format!("Failed to save params: {}", e))?;
            println!("Saved param_flags.json");
            println!();
        }

        let db = UnifiedFlagDb::build_default()
            .map_err(|e| format!("Failed to build unified database: {}", e))?;

        db.print_summary();

        db.save_to_json(&db_path)
            .map_err(|e| format!("Failed to save: {}", e))?;

        println!();
        println!("Saved to {:?}", db_path);
        return Ok(());
    }

    // Load database
    let db = if db_path.exists() {
        UnifiedFlagDb::load_from_json(&db_path)
            .map_err(|e| format!("Failed to load: {}", e))?
    } else {
        println!("Unified database not found at {:?}", db_path);
        println!("Building from sources...");
        println!();

        let db = UnifiedFlagDb::build_default()
            .map_err(|e| format!("Failed to build: {}", e))?;

        db.save_to_json(&db_path)
            .map_err(|e| format!("Failed to save: {}", e))?;

        db
    };

    if show_stats {
        db.print_summary();
        return Ok(());
    }

    if show_needs_formula {
        println!("=== Flags Needing Formula Discovery ===");
        println!("(In params but NOT in EMEVD)");
        println!();

        let flags = db.flags_needing_formulas();
        println!("{:<12} {:>12} {}", "Flag ID", "Category", "Param Source");
        println!("{}", "-".repeat(60));

        for flag in flags.iter().take(50) {
            let param = flag.param_sources.first()
                .map(|s| s.param_name.as_str())
                .unwrap_or("?");
            println!("{:<12} {:>12} {}",
                flag.flag_id,
                flag.flag_category.name(),
                param);
        }

        if flags.len() > 50 {
            println!("... and {} more", flags.len() - 50);
        }
        println!();
        println!("Total: {} flags need formula discovery", flags.len());
        return Ok(());
    }

    if show_high_conf {
        println!("=== High Confidence Flags (All 3 Sources) ===");
        println!();

        let flags = db.flags_by_confidence(SourceConfidence::High);
        println!("{:<12} {:>20} {:>15} {}", "Flag ID", "Name", "Category", "Context");
        println!("{}", "-".repeat(80));

        for flag in flags.iter().take(30) {
            let name = flag.name.as_deref().unwrap_or("-");
            let name_short = if name.len() > 18 { &name[..18] } else { name };
            let cat = flag.category.as_deref().unwrap_or("-");
            let ctx = flag.trigger_context.as_deref().unwrap_or("-");
            println!("{:<12} {:>20} {:>15} {}",
                flag.flag_id, name_short, cat, ctx);
        }

        println!();
        println!("Total: {} high-confidence flags", flags.len());
        return Ok(());
    }

    if let Some(ref query) = search_query {
        println!("=== Search: '{}' ===", query);
        println!();

        let results = db.search_by_name(query);
        for flag in results.iter().take(20) {
            let name = flag.display_name();
            let sources = format!("{}{}{}",
                if flag.has_catalog_data() { "C" } else { "-" },
                if flag.has_param_data() { "P" } else { "-" },
                if flag.has_emevd_data() { "E" } else { "-" });
            println!("{:<12} [{}] {}",
                flag.flag_id, sources, name);
        }

        if results.len() > 20 {
            println!("... and {} more", results.len() - 20);
        }
        println!();
        println!("Found {} matching flags", results.len());
        return Ok(());
    }

    if let Some(ref cat) = category_filter {
        println!("=== Category: {} ===", cat);
        println!();

        let flags = db.flags_by_category(cat);
        if flags.is_empty() {
            println!("No flags in category: {}", cat);
            println!();
            println!("Available categories:");
            for c in db.categories().iter().take(20) {
                println!("  {}", c);
            }
            return Ok(());
        }

        for flag in flags.iter().take(30) {
            let name = flag.name.as_deref().unwrap_or("-");
            println!("{:<12} {}", flag.flag_id, name);
        }

        if flags.len() > 30 {
            println!("... and {} more", flags.len() - 30);
        }
        println!();
        println!("Total: {} flags in category", flags.len());
        return Ok(());
    }

    if let Some(ref ctx) = context_filter {
        println!("=== Trigger Context: {} ===", ctx);
        println!();

        let flags = db.flags_by_trigger_context(ctx);
        if flags.is_empty() {
            println!("No flags with context: {}", ctx);
            println!();
            println!("Available contexts:");
            for c in db.trigger_contexts().iter().take(20) {
                println!("  {}", c);
            }
            return Ok(());
        }

        for flag in flags.iter().take(30) {
            let name = flag.display_name();
            println!("{:<12} {}", flag.flag_id, name);
        }

        if flags.len() > 30 {
            println!("... and {} more", flags.len() - 30);
        }
        println!();
        println!("Total: {} flags with context", flags.len());
        return Ok(());
    }

    if let Some(id) = query_id {
        println!("=== Flag {} ===", id);
        println!();

        if let Some(flag) = db.get(id) {
            // Display name
            println!("Name: {}", flag.display_name());
            if let Some(ref boss) = flag.boss_name {
                println!("Boss: {}", boss);
            }
            println!();

            // Source coverage
            let sources = format!("{}{}{}",
                if flag.has_catalog_data() { "Catalog " } else { "" },
                if flag.has_param_data() { "Params " } else { "" },
                if flag.has_emevd_data() { "EMEVD" } else { "" });
            println!("Sources: {} ({:?})", sources.trim(), flag.confidence);
            println!("Category: {} / {:?}",
                flag.category.as_deref().unwrap_or("-"),
                flag.flag_category);
            if let Some(ref region) = flag.region {
                println!("Region: {}", region);
            }
            if let Some(ref tile) = flag.map_tile {
                println!("Map tile: {}", tile);
            }

            // Position
            if let Some(ref pos) = flag.position {
                println!();
                println!("Position: ({:.1}, {:.1}, {:.1})", pos.pos_x, pos.pos_y, pos.pos_z);
                if let (Some(wx), Some(wz)) = (pos.world_x, pos.world_z) {
                    println!("World: ({:.1}, {:.1})", wx, wz);
                }
            }

            // Item info
            if let Some(ref item) = flag.item_info {
                println!();
                println!("Item ID: {}", item.item_id);
                if let Some(cat) = item.item_category {
                    println!("Item category: {}", cat);
                }
                if let Some(ref tt) = item.treasure_type {
                    println!("Treasure type: {}", tt);
                }
            }

            // Param sources
            if !flag.param_sources.is_empty() {
                println!();
                println!("Param Sources:");
                for source in &flag.param_sources {
                    println!("  {} row {} ({})",
                        source.param_name, source.row_id, source.field_name);
                }
            }

            // EMEVD triggers
            if !flag.emevd_triggers.is_empty() {
                println!();
                println!("EMEVD Triggers:");
                for trigger in flag.emevd_triggers.iter().take(5) {
                    println!("  Event {} [{}] in {}",
                        trigger.event_id, trigger.context, trigger.source_file);
                }
                if flag.emevd_triggers.len() > 5 {
                    println!("  ... and {} more", flag.emevd_triggers.len() - 5);
                }
            }

            // Progression chain
            if let Some(ref chain) = flag.in_progression_chain {
                println!();
                println!("Progression chain: {}", chain);
            }
        } else {
            println!("Flag {} not found in unified database", id);
        }
        return Ok(());
    }

    // Show usage
    println!("Unified Flag Database");
    println!();
    println!("Combines: Flag Catalog + Param Database + Event Graph");
    println!();
    println!("USAGE:");
    println!("    discovery unified --build         Build/rebuild the database");
    println!("    discovery unified <flag_id>       Query specific flag");
    println!("    discovery unified --stats         Show statistics");
    println!("    discovery unified --search NAME   Search by name");
    println!("    discovery unified --needs-formula Flags needing formula discovery");
    println!("    discovery unified --high          High-confidence flags (all 3 sources)");
    println!("    discovery unified --category CAT  Filter by catalog category");
    println!("    discovery unified --context CTX   Filter by EMEVD trigger context");
    println!();
    println!("OPTIONS:");
    println!("    --db <path>       Path to unified_flags.json");
    println!();
    println!("Current database: {} flags", db.len());

    Ok(())
}
