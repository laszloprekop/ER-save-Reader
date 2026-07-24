//! Headless capture of the reader's enriched output for the **Output baseline**
//! (ADR-0010; `CONTEXT.md` → *Output baseline*; ticket #11).
//!
//! The GUI's Export button serialises one slot's `ExportData` to JSON. This
//! module does the same headlessly for every active slot of a save, so the
//! reader half of the Output baseline can be regenerated and diffed in CI as the
//! reconstruction moves into the shared core. It is a **change-detector, not an
//! oracle**: it captures what the reader renders *today*, and every later diff is
//! triaged regression-vs-improvement (see `baselines/README.md`).
//!
//! Two fields of `ExportMetadata` are normalised so the committed baseline is
//! byte-stable and carries no account identifier — neither is part of the
//! reconstructed character:
//!   * `export_date` is a wall-clock timestamp that changes every run, so it is
//!     pinned to a constant.
//!   * `steam_id` is a real 64-bit Steam account id, so it is zeroed rather than
//!     written into a file under version control.

use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};

use crate::save::save::save::Save;
use crate::vm::vm::vm::ViewModel;

/// CLI entry: `er-save-reader baseline <save.sl2> <out_dir>`.
pub fn run_cli(args: &[String]) -> io::Result<()> {
    let (save, out) = match args {
        [save, out] => (PathBuf::from(save), PathBuf::from(out)),
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "usage: er-save-reader baseline <save.sl2> <out_dir>",
            ));
        }
    };
    let written = capture(&save, &out)?;
    println!(
        "wrote {} slot baseline(s) to {}",
        written,
        out.display()
    );
    Ok(())
}

/// Serialise every active slot's `ExportData` to
/// `<out_dir>/slotNN_<name>.json`. Returns the number of slots written.
///
/// Deterministic for a fixed save: the only non-stable fields are normalised
/// here, and every collection in `ExportData` is an ordered `Vec`/`BTreeMap`
/// built in save order.
fn capture(save_path: &Path, out_dir: &Path) -> io::Result<usize> {
    let save = Save::from_path(&save_path.to_path_buf())?;
    let vm = ViewModel::from_save(&save);
    if vm.active != Some(true) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "save failed validation; nothing to capture",
        ));
    }

    fs::create_dir_all(out_dir)?;
    let mut written = 0;
    for (index, active) in save.save_type.active_slots().iter().enumerate() {
        if !*active {
            continue;
        }
        let event_flags = save.save_type.get_event_flags(index);
        // steam_id zeroed: not part of the character, and a real account id must
        // not land in a committed baseline.
        let mut export = vm.slots[index].to_export_data(index, 0, event_flags);
        // Pin the wall-clock field so the baseline is byte-stable across runs.
        export.metadata.export_date = "normalized".to_string();

        let name = sanitize(&vm.slots[index].general_vm.character_name);
        let json = serde_json::to_string_pretty(&export).map_err(Error::other)?;
        let file = out_dir.join(format!("slot{index:02}_{name}.json"));
        fs::write(&file, json)?;
        written += 1;
    }

    if written == 0 {
        return Err(Error::new(
            ErrorKind::NotFound,
            "no active slots found in save",
        ));
    }
    Ok(written)
}

/// Filesystem-safe, deterministic filename fragment from a character name.
fn sanitize(raw: &str) -> String {
    let cleaned: String = raw
        .trim_matches('\0')
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if cleaned.is_empty() {
        "unnamed".to_string()
    } else {
        cleaned
    }
}
