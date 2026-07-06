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
pub mod pipeline;
pub mod timeline;

pub fn run_cli(args: &[String]) -> Result<(), String> {
    match args.first().map(|s| s.as_str()) {
        Some("catalog-update") => catalog::cmd_update(&args[1..]),
        Some("catalog-verify") => catalog::cmd_verify(&args[1..]),
        Some("run") => pipeline::cmd_run(&args[1..]),
        Some("timeline") => timeline::cmd_timeline(&args[1..]),
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
            println!("    timeline <id>     Replay a sparse-diff timeline target (see");
            println!("                      knowledge/inputs/timeline-targets.json), emit");
            println!("                      knowledge/claims/timeline-events.json");
            Ok(())
        }
    }
}
