//! Knowledge pipeline CLI family (migration step 2+, see docs/BACKLOG.md
//! Priority 0 and docs/adr/0004).
//!
//! Current commands:
//!   knowledge catalog-update   Fill/refresh machine fields (sha256, sizes,
//!                              manifests) in knowledge/evidence-catalog.json,
//!                              preserving hand-written context fields.
//!   knowledge catalog-verify   Recompute and compare; nonzero exit on drift.
//!   knowledge run              Evidence -> claims pipeline: verify-on-read,
//!                              attributed-transition analysis, deterministic
//!                              claims-store emission (knowledge/claims/).
//!   knowledge timeline <id>    Sparse-diff timeline replay + world-state-b
//!                              re-annotation (knowledge/claims/timeline-events.json).

pub mod catalog;
pub mod dump;
pub mod family_distances;
pub mod gen_dungeon_pickups;
pub mod pipeline;
pub mod timeline;

pub fn run_cli(args: &[String]) -> Result<(), String> {
    match args.first().map(|s| s.as_str()) {
        Some("catalog-update") => catalog::cmd_update(&args[1..]),
        Some("catalog-verify") => catalog::cmd_verify(&args[1..]),
        Some("run") => pipeline::cmd_run(&args[1..]),
        Some("timeline") => timeline::cmd_timeline(&args[1..]),
        Some("family-distances") => family_distances::cmd_family_distances(&args[1..]),
        Some("origin-probe") => family_distances::cmd_origin_probe(&args[1..]),
        Some("list-hunt") => family_distances::cmd_list_hunt(&args[1..]),
        Some("validate-origin") => family_distances::cmd_validate_origin(&args[1..]),
        Some("family-constants") => family_distances::cmd_family_constants(&args[1..]),
        Some("grace-dump") => dump::cmd_grace_dump(&args[1..]),
        Some("gen-dungeon-pickups") => gen_dungeon_pickups::cmd_gen_dungeon_pickups(&args[1..]),
        _ => {
            println!("Knowledge pipeline (evidence catalog + claims store)");
            println!();
            println!("USAGE:");
            println!("    er-save-editor knowledge <COMMAND>");
            println!();
            println!("COMMANDS:");
            println!("    catalog-update    Fill/refresh machine fields in the evidence catalog");
            println!("    catalog-verify    Verify evidence against the catalog (exit 1 on drift)");
            println!("    run               Regenerate the claims store from evidence (ADR-0004)");
            println!("    family-distances  Measure every family base per file; test whether");
            println!("                      the distance BETWEEN families is constant (step 4b)");
            println!("    origin-probe      Test whether a u32 record count explains the");
            println!("                      residual family drift (step 4b)");
            println!("    list-hunt         Locate the variable-length structures that move");
            println!("                      the flag families (step 4b)");
            println!("    validate-origin   Out-of-sample test of the origin model on");
            println!("                      characters it was not derived from (step 4b)");
            println!("    family-constants  Measure each family's distance from the origin");
            println!("                      from the attributed flips that pinned its base");
            println!("    gen-dungeon-pickups [xml] [--out PATH]");
            println!("                      Regenerate src/db/dungeon_pickups.rs from the");
            println!("                      primary ItemLotParam_map (anti-drift; the table");
            println!("                      is generated, not hand-edited)");
            println!("    grace-dump <save> [slot] [--all]");
            println!("                      Dump every grace in a slot, layer by layer:");
            println!("                      raw byte, resolver verdict, database name");
            println!("    timeline <id>     Replay a sparse-diff timeline target (see");
            println!("                      knowledge/inputs/timeline-targets.json), emit");
            println!("                      knowledge/claims/timeline-events.json");
            Ok(())
        }
    }
}
