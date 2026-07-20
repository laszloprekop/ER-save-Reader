//! Guards ADR-0008: exported flag readers refuse rather than guess.
//!
//! The risk this file exists to close: for four cutover commits, `get_dungeon_general_bases()`
//! was documented as disproven in five separate places and *still* reachable from three
//! `#[wasm_bindgen]` exports. Prose did not stop it, because a call site does not read prose.
//! Nothing failed when the wrong thing stayed wired up. These tests fail.
//!
//! The invariant, stated once: **no exported function may source a flag's position from
//! inside this crate.** A flag family's base floats per save — it sits after an append-only
//! list that grows as the character plays — so a base baked into the crate is valid only for
//! the save it was measured on. An export answering a flag question must therefore either
//! receive the save bytes (and resolve the family itself, returning Unknown when it cannot)
//! or receive the base from its caller.
//!
//! These tests read the source rather than calling the code, deliberately: the property is
//! about what *exists*, and a deleted function cannot be tested by calling it.

const SRC: &str = include_str!("../src/lib.rs");

/// Strip `//`-comments so the checks below match live code, not the removal notes
/// that necessarily mention the very names and numbers being banned.
fn code_only(src: &str) -> String {
    src.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every base table and static-offset router removed under ADR-0008.
/// If one of these names comes back as a definition, it is almost certainly the old
/// model regrowing — read the ADR before adding an exception.
const BANNED_DEFINITIONS: &[&str] = &[
    // The disproven "+3375 per area" stride table and its router.
    "get_dungeon_general_bases",
    "calculate_dungeon_flag_offset_unified",
    // Block / midrange base tables and their calculators.
    "get_sub_block_bases",
    "get_main_block_bases",
    "get_midrange_bases",
    "calculate_simple_flag_offset",
    "calculate_midrange_flag_offset",
    // Per-section dungeon pickup bases.
    "get_dungeon_pickup_section_bases",
    "calculate_dungeon_pickup_offset",
    "get_dungeon_pickup_sections",
    // The static-offset exports themselves.
    "get_flag_offset",
    "get_flag_offset_calibrated",
    "get_flag_offset_with_tile_base",
    "calculate_tile_flag_offset_unified",
    "is_flag_set",
    "is_flag_set_calibrated",
    // Row_id bitfield model, superseded 2026-02-16.
    "calculate_world_pickup_offset_by_row_id",
    "get_tile_base_offset",
    "get_world_pickup_row_id_base",
];

#[test]
fn no_removed_base_table_or_static_offset_symbol_is_defined() {
    let code = code_only(SRC);
    let mut found = Vec::new();
    for name in BANNED_DEFINITIONS {
        // Match definitions, not mentions: `fn NAME(` / `const NAME:` / `static NAME:`.
        for pat in [
            format!("fn {}(", name),
            format!("const {}:", name),
            format!("static {}:", name),
        ] {
            if code.contains(&pat) {
                found.push(format!("{} (as `{}`)", name, pat.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "ADR-0008 violation: these were removed because they hand out a flag position \
         computed from a base baked into this crate, and every family's base floats per \
         save. They are defined again:\n  {}\n\nIf a caller needs a flag's state, the \
         reader must take the flag region: is_world_state_flag_set, is_tile_world_flag_set, \
         is_tile_pickup_set, is_dungeon_flag_set, is_dungeon_pickup_set.",
        found.join("\n  ")
    );
}

#[test]
fn no_tombstoned_base_constant_appears_in_code() {
    let code = code_only(SRC);
    // (literal, what it was, why it is not a base)
    let tombstoned = [
        (
            "337375",
            "TILE_BASE_OFFSET",
            "real, but it is the DISTANCE BETWEEN two flag families, not a base \
             (tombstone `tile-base-337375-grace-anchored`)",
        ),
        (
            "1037373320",
            "WORLD_PICKUP_ROW_ID_BASE",
            "belongs to the row_id bitfield storage model disproven 2026-02-16",
        ),
        (
            "43487",
            "dungeon general base for m18 (Stranded Graveyard)",
            "reads all-zero in every save on this machine, though every character \
             necessarily sets m18 flags in the tutorial",
        ),
        (
            "46862",
            "dungeon general base for m19 (Elden Throne)",
            "same stride assumption as m18, same disproof",
        ),
    ];
    for (lit, was, why) in tombstoned {
        assert!(
            !code.contains(lit),
            "ADR-0008 violation: the literal {} ({}) is back in live code. It is {}.",
            lit,
            was,
            why
        );
    }
}

/// The approved `#[wasm_bindgen]` export surface.
///
/// This is a manifest, not a snapshot: the test asserts the real surface EQUALS it, so
/// adding or renaming an export is a deliberate edit here, where the rule is written down.
/// Each entry records how it satisfies the invariant.
const APPROVED_EXPORTS: &[(&str, &str)] = &[
    // --- Take the save/flag bytes and resolve the family for THAT save -----------------
    ("detect_event_flags_offset", "takes slot_data"),
    ("compute_structural_event_flags_offset", "takes slot_data; diagnostics only"),
    ("parse_ga_items_end", "takes slot_data"),
    ("extract_player_position", "takes slot_data; not a flag reader"),
    ("extract_equipment_data", "takes slot_data; not a flag reader"),
    ("flag_list_end", "takes slot_data; locates the list every family floats behind"),
    ("family_base", "takes slot_data; THE resolver, returns -1 when it cannot resolve"),
    // --- Tri-state readers: take the flag region, answer Unknown rather than guess -----
    ("world_state_flag_state", "takes event_flags"),
    ("tile_world_flag_state", "takes event_flags"),
    ("tile_pickup_state", "takes event_flags"),
    ("dungeon_flag_state", "takes event_flags"),
    ("dungeon_pickup_state", "takes event_flags"),
    // --- Caller supplies the base; the crate invents nothing ---------------------------
    (
        "calculate_tile_pickup_offset_calibrated",
        "tile_base is a parameter. This is tile GEOMETRY, not family location: \
         `tile_read` calls the same code with base 0 and adds a resolved base.",
    ),
    // --- Pure id arithmetic and classification: answer no positional question ----------
    ("convert_to_row_id", "id arithmetic (flag_id - 7000); no offset"),
    ("is_dungeon_pickup_flag", "classifies an id by localId; no offset"),
    ("is_tile_pickup_flag", "classifies an id by range; no offset"),
    // --- Structural constants, not family bases ----------------------------------------
    ("get_event_flags_size", "section size; fixed by the format"),
    ("get_search_start", "detection search window bound"),
    ("get_tile_max_local_id", "localId ceiling; not a position"),
    ("get_player_coords_search_start", "coordinate scan bound"),
    ("get_player_coords_search_end", "coordinate scan bound"),
];

/// Extract the names of all `#[wasm_bindgen]`-attributed `pub fn` exports.
fn actual_exports(src: &str) -> Vec<String> {
    let lines: Vec<&str> = src.lines().collect();
    let mut names = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t != "#[wasm_bindgen]" && !t.starts_with("#[wasm_bindgen(") {
            continue;
        }
        // Skip doc comments and further attributes to reach the item.
        for next in lines.iter().skip(i + 1).take(12) {
            let n = next.trim();
            if n.starts_with("///") || n.starts_with("#[") {
                continue;
            }
            if let Some(rest) = n.strip_prefix("pub fn ") {
                if let Some(paren) = rest.find('(') {
                    names.push(rest[..paren].to_string());
                }
            }
            break; // structs/impls are not exports we gate here
        }
    }
    names.sort();
    names.dedup();
    names
}

#[test]
fn exported_surface_matches_the_approved_manifest() {
    let actual = actual_exports(SRC);
    let mut approved: Vec<String> = APPROVED_EXPORTS.iter().map(|(n, _)| n.to_string()).collect();
    approved.sort();

    let added: Vec<&String> = actual.iter().filter(|n| !approved.contains(n)).collect();
    let removed: Vec<&String> = approved.iter().filter(|n| !actual.contains(n)).collect();

    assert!(
        added.is_empty(),
        "New `#[wasm_bindgen]` export(s) not in the ADR-0008 manifest: {:?}\n\n\
         Before adding them to APPROVED_EXPORTS, check the invariant: an export that \
         answers where a flag lives, or whether it is set, must take the save/flag bytes \
         (and return Unknown when the family cannot be resolved) or take the base as a \
         parameter. It must not read a base baked into this crate.",
        added
    );
    assert!(
        removed.is_empty(),
        "Manifest lists export(s) that no longer exist: {:?}. If the removal was \
         intentional, delete them from APPROVED_EXPORTS.",
        removed
    );
}

#[test]
fn every_export_answering_a_flag_question_receives_bytes_or_an_explicit_base() {
    // The structural form of the invariant, independent of the manifest's names: any
    // export whose result depends on where a flag sits must be handed that context.
    let lines: Vec<&str> = SRC.lines().collect();
    let mut violations = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t != "#[wasm_bindgen]" && !t.starts_with("#[wasm_bindgen(") {
            continue;
        }
        for next in lines.iter().skip(i + 1).take(12) {
            let n = next.trim();
            if n.starts_with("///") || n.starts_with("#[") {
                continue;
            }
            let Some(rest) = n.strip_prefix("pub fn ") else { break };
            let Some(paren) = rest.find('(') else { break };
            let name = &rest[..paren];
            let sig = &rest[paren..];

            // Does this export claim to answer a positional/state question about a flag?
            let positional = sig.contains("-> FlagOffset")
                || name.ends_with("_state")
                || name.contains("_offset");
            if !positional {
                break;
            }
            // Then it must receive the bytes, or a base from the caller.
            let receives_context = sig.contains("&[u8]") || sig.contains("base:");
            if !receives_context {
                violations.push(format!("{}{}", name, sig.trim_end_matches(" {")));
            }
            break;
        }
    }

    assert!(
        violations.is_empty(),
        "ADR-0008 violation: these exports answer a flag position/state question without \
         receiving the save bytes or an explicit base, so they can only be reading a base \
         baked into this crate — which is valid for at most one save:\n  {}",
        violations.join("\n  ")
    );
}
