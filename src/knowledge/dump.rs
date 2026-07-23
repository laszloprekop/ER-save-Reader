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

use super::evidence::slot_slice;
use crate::db::graces_data::GRACES_DATA;
use wasm_event_flags::{FlagState, ResolvedFlags};

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
        let slot = match slot_slice(&data, slot_index) {
            Some(s) => s,
            None => continue,
        };

        let det = wasm_event_flags::detect_event_flags_offset_impl(slot);
        if det.offset == 0 {
            println!("=== slot {}: no event-flag region found", slot_index);
            continue;
        }
        let ef = &slot[det.offset..];
        // Resolve the origin once for every family read in this slot's dump.
        let resolved = ResolvedFlags::from_event_flags(ef);
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
            let state: Option<bool> = resolved.as_ref().map_or(FlagState::Unknown, |r| r.world_state(flag)).into();
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
            "  graces:  {} set, {} clear, {} UNKNOWN (of {})",
            set,
            clear,
            unknown,
            GRACES_DATA.len()
        );

        // World pickups share the tile layout but split by local id into two
        // regions 500 bytes apart, so this also exercises that routing.
        let (mut pset, mut pclear, mut punknown) = (0usize, 0usize, 0usize);
        for pickup in crate::db::pickup_data::WORLD_PICKUPS.iter() {
            match Option::<bool>::from(resolved.as_ref().map_or(FlagState::Unknown, |r| crate::db::pickup_flags::pickup_state(r, pickup.event_flag))) {
                Some(true) => pset += 1,
                Some(false) => pclear += 1,
                None => punknown += 1,
            }
        }
        println!(
            "  pickups: {} collected, {} not, {} UNKNOWN (of {})",
            pset,
            pclear,
            punknown,
            crate::db::pickup_data::WORLD_PICKUPS.len()
        );
        // `db::world_pickups` is the generated complement of `db::dungeon_pickups`
        // over the primary source, and is keyed on the raw getItemFlagId rather
        // than pickup_data's row-id convention — counted separately so the two
        // conventions cannot cover for each other.
        let (mut wset, mut wclear, mut wunknown) = (0usize, 0usize, 0usize);
        for p in crate::db::world_pickups::WORLD_PICKUPS.values() {
            match Option::<bool>::from(resolved.as_ref().map_or(FlagState::Unknown, |r| crate::db::pickup_flags::pickup_state(r, p.flag_id))) {
                Some(true) => wset += 1,
                Some(false) => wclear += 1,
                None => wunknown += 1,
            }
        }
        println!(
            "  world:   {} collected, {} not, {} UNKNOWN (of {})",
            wset,
            wclear,
            wunknown,
            crate::db::world_pickups::WORLD_PICKUPS.len()
        );
        let (mut dset, mut dclear, mut dunknown) = (0usize, 0usize, 0usize);
        for p in crate::db::dungeon_pickups::DUNGEON_PICKUPS.iter() {
            match Option::<bool>::from(resolved.as_ref().map_or(FlagState::Unknown, |r| r.dungeon_pickup(p.event_flag))) {
                Some(true) => dset += 1,
                Some(false) => dclear += 1,
                None => dunknown += 1,
            }
        }
        println!(
            "  dungeon: {} collected, {} not, {} UNKNOWN (of {})",
            dset,
            dclear,
            dunknown,
            crate::db::dungeon_pickups::DUNGEON_PICKUPS.len()
        );

        // Bosses split by id width into two families: 10-digit open-world tiles
        // and 8-digit legacy maps (alloc-slot layout). Counted separately so a
        // family that silently stops resolving shows up as its own column of
        // UNKNOWN rather than being averaged away.
        let (mut btile, mut bleg) = ((0usize, 0usize, 0usize), (0usize, 0usize, 0usize));
        for boss in crate::db::bosses_data::BOSSES_DATA.values() {
            let f = boss.defeat_flag;
            let (state, c) = if f >= 1_000_000_000 {
                (Option::<bool>::from(resolved.as_ref().map_or(FlagState::Unknown, |r| r.tile_world(f))), &mut btile)
            } else {
                (Option::<bool>::from(resolved.as_ref().map_or(FlagState::Unknown, |r| r.dungeon(f))), &mut bleg)
            };
            match state {
                Some(true) => c.0 += 1,
                Some(false) => c.1 += 1,
                None => c.2 += 1,
            }
        }
        println!(
            "  bosses:  {} defeated, {} not, {} UNKNOWN  (open-world {}/{}, legacy {}/{})",
            btile.0 + bleg.0,
            btile.1 + bleg.1,
            btile.2 + bleg.2,
            btile.0,
            btile.0 + btile.1 + btile.2,
            bleg.0,
            bleg.0 + bleg.1 + bleg.2,
        );

        // The demigods by name. These are the milestones the evidence catalog
        // describes characters by ("predates the Margit/Godrick/Radahn kills"),
        // so printing them lets a reader check the read against the catalog
        // rather than against an aggregate that could hide a wrong family.
        let mut demigods: Vec<_> = crate::db::bosses_data::BOSSES_DATA
            .values()
            .filter(|b| b.boss_type == crate::db::bosses_data::BossType::Demigod)
            .collect();
        demigods.sort_by_key(|b| b.defeat_flag);
        print!("  demigods:");
        for b in demigods {
            let state = if b.defeat_flag >= 1_000_000_000 {
                Option::<bool>::from(resolved.as_ref().map_or(FlagState::Unknown, |r| r.tile_world(b.defeat_flag)))
            } else {
                Option::<bool>::from(resolved.as_ref().map_or(FlagState::Unknown, |r| r.dungeon(b.defeat_flag)))
            };
            let mark = match state {
                Some(true) => "YES",
                Some(false) => "no",
                None => "?",
            };
            print!(" {}={}", b.name.split_whitespace().next().unwrap_or("?"), mark);
        }
        println!();

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
