//! Knowledge pipeline CLI family (migration step 2+, see docs/BACKLOG.md
//! Priority 0 and docs/adr/0004).
//!
//! Current commands:
//!   knowledge catalog-update   Fill/refresh machine fields (sha256, sizes,
//!                              manifests) in knowledge/evidence-catalog.json,
//!                              preserving hand-written context fields.
//!   knowledge catalog-verify   Recompute and compare; nonzero exit on drift.

pub mod catalog;

pub fn run_cli(args: &[String]) -> Result<(), String> {
    match args.first().map(|s| s.as_str()) {
        Some("catalog-update") => catalog::cmd_update(&args[1..]),
        Some("catalog-verify") => catalog::cmd_verify(&args[1..]),
        _ => {
            println!("Knowledge pipeline (evidence catalog)");
            println!();
            println!("USAGE:");
            println!("    er-save-editor knowledge <COMMAND>");
            println!();
            println!("COMMANDS:");
            println!("    catalog-update    Fill/refresh machine fields in the evidence catalog");
            println!("    catalog-verify    Verify evidence against the catalog (exit 1 on drift)");
            Ok(())
        }
    }
}
