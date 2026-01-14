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
    DiscoveryStore, ConsensusBuilder, ConsensusStatus,
    GroundTruthUpdater, UpdateConfig,
};

/// Default paths
const DEFAULT_SNAPSHOT_DIR: &str = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging";
const DEFAULT_STORE_PATH: &str = "discoveries.json";
const DEFAULT_GROUND_TRUTH: &str = "ground_truth_offsets.json";

/// Run discovery CLI with given arguments
pub fn run_cli(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "batch-analyze" | "batch" => cmd_batch_analyze(&args[1..]),
        "status" | "s" => cmd_status(&args[1..]),
        "promotable" | "p" => cmd_promotable(&args[1..]),
        "promote" => cmd_promote(&args[1..]),
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
    println!("    batch-analyze    Process all snapshot pairs and persist discoveries");
    println!("    status           Show discovery store statistics");
    println!("    promotable       List discoveries ready for promotion");
    println!("    promote          Promote confirmed discoveries to ground truth");
    println!("    help             Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    er-save-editor discovery batch-analyze");
    println!("    er-save-editor discovery status");
    println!("    er-save-editor discovery promotable");
    println!("    er-save-editor discovery promote --dry-run");
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
