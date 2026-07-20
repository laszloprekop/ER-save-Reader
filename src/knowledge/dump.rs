//! `knowledge grace-dump <save.sl2> [slot]` — first-hand evidence, layer by layer.
//!
//! Prints what each layer of the app believes about every grace in a slot, so the
//! layers can be compared against each other and against an exported JSON:
//!
//!   layer 1  the raw byte and bit in the save file
//!   layer 2  the resolver's verdict (set / clear / UNKNOWN)
//!   layer 3  the name the app's database attaches to that flag
//!
//! Layer 3 is included deliberately. A verified byte read still reaches the user
//! through a name, and a wrong name is indistinguishable from a wrong offset when
//! you are looking at a table of grace names — which is exactly how a documented
//! anchor table and the app's own database were found to disagree about which of
//! 76100/76101 is "The First Step" (docs/BACKLOG.md step 4).

use std::fs;
use std::path::Path;

use super::pipeline::{CHECKSUM, HEADER, SLOT_SIZE};
use crate::db::graces_data::GRACES_DATA;

pub fn cmd_grace_dump(args: &[String]) -> Result<(), String> {
    let path = args
        .first()
        .ok_or("usage: knowledge grace-dump <save.sl2> [slot] [--all]")?;
    let all = args.iter().any(|a| a == "--all");
    let slots: Vec<usize> = match args.get(1).and_then(|s| s.parse::<usize>().ok()) {
        Some(s) => vec![s],
        None => (0..10).collect(),
    };

    let data = fs::read(Path::new(path)).map_err(|e| format!("{}: {}", path, e))?;
    println!("file: {}", path);
    println!("size: {} bytes\n", data.len());

    for slot_index in slots {
        let start = HEADER + slot_index * (CHECKSUM + SLOT_SIZE) + CHECKSUM;
        if data.len() < start + SLOT_SIZE {
            continue;
        }
        let slot = &data[start..start + SLOT_SIZE];

        let det = wasm_event_flags::detect_event_flags_offset_impl(slot);
        if det.offset == 0 {
            println!("=== slot {}: no event-flag region found", slot_index);
            continue;
        }
        let ef = &slot[det.offset..];
        let list_end = wasm_event_flags::find_flag_list_end_in_ef(ef);
        let base = wasm_event_flags::resolve_family_base_in_ef(
            ef,
            wasm_event_flags::FAMILY_WORLD_STATE_B,
        );

        println!(
            "=== slot {}   ef_offset={}  confident={}  list_end={}  world-state-b base={}",
            slot_index,
            det.offset,
            det.confident,
            list_end
                .map(|v| v.to_string())
                .unwrap_or_else(|| "UNRESOLVED".into()),
            base.map(|v| v.to_string())
                .unwrap_or_else(|| "UNRESOLVED".into()),
        );

        let mut flags: Vec<u32> = GRACES_DATA.keys().copied().collect();
        flags.sort_unstable();

        let (mut set, mut clear, mut unknown) = (0usize, 0usize, 0usize);
        let mut found_names: Vec<(u32, &str)> = Vec::new();

        for flag in flags {
            let state = wasm_event_flags::is_world_state_flag_set(ef, flag);
            match state {
                Some(true) => set += 1,
                Some(false) => clear += 1,
                None => unknown += 1,
            }
            let name = GRACES_DATA.get(&flag).map(|g| g.name).unwrap_or("?");
            if state == Some(true) {
                found_names.push((flag, name));
            }
            if all {
                // layer 1: the actual byte, so a reader can check the arithmetic
                let cell = base.and_then(|b| {
                    let byte = b + ((flag - 50_000) / 8) as usize;
                    ef.get(byte).map(|v| (byte, *v, 7 - (flag % 8) as u8))
                });
                let raw = match cell {
                    Some((byte, val, bit)) => {
                        format!("ef[{}]=0x{:02x} bit{}", byte, val, bit)
                    }
                    None => "-".into(),
                };
                let mark = match state {
                    Some(true) => "SET    ",
                    Some(false) => "clear  ",
                    None => "UNKNOWN",
                };
                println!("  {:>7} {}  {:34} {}", flag, mark, name, raw);
            }
        }

        println!(
            "  totals: {} set, {} clear, {} UNKNOWN (of {})",
            set,
            clear,
            unknown,
            GRACES_DATA.len()
        );
        if !found_names.is_empty() && found_names.len() <= 12 {
            println!("  set flags:");
            for (flag, name) in &found_names {
                println!("    {} = {}", flag, name);
            }
        }
        println!();
    }
    Ok(())
}
