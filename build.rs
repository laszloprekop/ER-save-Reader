use std::{
    env,
    fs,
    io::{self, Write},
    path::Path,
};

fn main() -> io::Result<()> {
    // Windows resource compilation
    #[cfg(windows)]
    {
        use winres::WindowsResource;
        if env::var_os("CARGO_CFG_WINDOWS").is_some() {
            WindowsResource::new()
                .set_icon("./icon/icon.ico")
                .compile()?;
        }
    }

    // Generate Rust code from ground_truth_offsets.json
    generate_offsets_from_json()?;

    Ok(())
}

/// Generate src/generated/ground_truth.rs from ground_truth_offsets.json
fn generate_offsets_from_json() -> io::Result<()> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let json_path = Path::new(&manifest_dir).join("ground_truth_offsets.json");

    // Only regenerate if JSON exists and is newer than generated file
    if !json_path.exists() {
        println!("cargo:warning=ground_truth_offsets.json not found, skipping code generation");
        return Ok(());
    }

    // Tell Cargo to rerun if JSON changes
    println!("cargo:rerun-if-changed=ground_truth_offsets.json");

    let json_content = fs::read_to_string(&json_path)?;
    let data: serde_json::Value = serde_json::from_str(&json_content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Create generated directory
    let out_dir = Path::new(&manifest_dir).join("src").join("generated");
    fs::create_dir_all(&out_dir)?;

    // Generate the Rust file
    let mut output = String::new();

    output.push_str("//! Auto-generated from ground_truth_offsets.json\n");
    output.push_str("//! DO NOT EDIT MANUALLY - run `cargo build` to regenerate\n");
    output.push_str("//!\n");
    if let Some(metadata) = data.get("metadata") {
        if let Some(date) = metadata.get("generated_date").and_then(|v| v.as_str()) {
            output.push_str(&format!("//! Generated: {}\n", date));
        }
    }
    output.push('\n');
    output.push_str("use std::collections::HashMap;\n");
    output.push_str("use once_cell::sync::Lazy;\n\n");

    // Generate block bases
    if let Some(formulas) = data.get("formulas") {
        if let Some(block_bases) = formulas.get("block_bases").and_then(|v| v.as_object()) {
            output.push_str("// ============================================================================\n");
            output.push_str("// BLOCK BASES (verified from ground_truth_offsets.json)\n");
            output.push_str("// ============================================================================\n\n");

            output.push_str("/// Block base offsets for flags 60000-99999\n");
            output.push_str("/// Formula: byte_offset = base + (flag_id - block_start) / 8\n");
            output.push_str("pub static VERIFIED_BLOCK_BASES: Lazy<HashMap<u32, BlockBase>> = Lazy::new(|| {\n");
            output.push_str("    HashMap::from([\n");

            let mut entries: Vec<_> = block_bases.iter().collect();
            entries.sort_by_key(|(k, _)| k.parse::<u32>().unwrap_or(0));

            for (block_start, info) in entries {
                if let Some(base_offset) = info.get("base_offset").and_then(|v| v.as_u64()) {
                    let status = info.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let notes = info.get("notes").and_then(|v| v.as_str()).unwrap_or("");
                    output.push_str(&format!(
                        "        ({}, BlockBase {{ base_offset: {}, status: \"{}\", notes: \"{}\" }}),\n",
                        block_start, base_offset, status, notes.replace("\"", "\\\"")
                    ));
                }
            }

            output.push_str("    ])\n");
            output.push_str("});\n\n");

            output.push_str("#[derive(Debug, Clone)]\n");
            output.push_str("pub struct BlockBase {\n");
            output.push_str("    pub base_offset: u32,\n");
            output.push_str("    pub status: &'static str,\n");
            output.push_str("    pub notes: &'static str,\n");
            output.push_str("}\n\n");
        }

        // Generate tile formula constants
        if let Some(tile) = formulas.get("tile_formula").and_then(|v| v.as_object()) {
            output.push_str("// ============================================================================\n");
            output.push_str("// TILE FORMULA (verified from ground_truth_offsets.json)\n");
            output.push_str("// ============================================================================\n\n");

            if let Some(base) = tile.get("base_offset").and_then(|v| v.as_u64()) {
                output.push_str("/// Base offset for tile formula (verified)\n");
                output.push_str(&format!("pub const VERIFIED_TILE_BASE_OFFSET: u32 = {};\n\n", base));
            }
            if let Some(bps) = tile.get("bytes_per_slot").and_then(|v| v.as_u64()) {
                output.push_str(&format!("pub const TILE_BYTES_PER_SLOT: u32 = {};\n", bps));
            }
            if let Some(spr) = tile.get("slots_per_row").and_then(|v| v.as_u64()) {
                output.push_str(&format!("pub const TILE_SLOTS_PER_ROW: u32 = {};\n", spr));
            }
            if let Some(rb) = tile.get("row_base").and_then(|v| v.as_u64()) {
                output.push_str(&format!("pub const TILE_ROW_BASE: u32 = {};\n", rb));
            }
            if let Some(cb) = tile.get("col_base").and_then(|v| v.as_u64()) {
                output.push_str(&format!("pub const TILE_COL_BASE: u32 = {};\n", cb));
            }
            if let Some(max) = tile.get("max_local_id").and_then(|v| v.as_u64()) {
                output.push_str(&format!("pub const TILE_MAX_LOCAL_ID: u32 = {};\n", max));
            }
            output.push('\n');
        }

        // Generate midrange formula bases (100000-999999 flags like sorceries/incantations)
        if let Some(midrange) = formulas.get("midrange_formula").and_then(|v| v.as_object()) {
            output.push_str("// ============================================================================\n");
            output.push_str("// MIDRANGE FORMULA (verified from ground_truth_offsets.json)\n");
            output.push_str("// ============================================================================\n\n");

            output.push_str("/// Midrange base offsets for flags 100000-999999 (sorceries, incantations, etc.)\n");
            output.push_str("/// Formula: byte_offset = base + (flag_id - block_start) / 8\n");
            output.push_str("pub static VERIFIED_MIDRANGE_BASES: Lazy<HashMap<u32, MidrangeBase>> = Lazy::new(|| {\n");
            output.push_str("    HashMap::from([\n");

            let mut entries: Vec<_> = midrange.iter().collect();
            entries.sort_by_key(|(k, _)| k.parse::<u32>().unwrap_or(0));

            for (block_start, info) in entries {
                if let Some(base_offset) = info.get("base_offset").and_then(|v| v.as_u64()) {
                    let status = info.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let notes = info.get("notes").and_then(|v| v.as_str()).unwrap_or("");
                    output.push_str(&format!(
                        "        ({}, MidrangeBase {{ base_offset: {}, status: \"{}\", notes: \"{}\" }}),\n",
                        block_start, base_offset, status, notes.replace("\"", "\\\"")
                    ));
                }
            }

            output.push_str("    ])\n");
            output.push_str("});\n\n");

            output.push_str("#[derive(Debug, Clone)]\n");
            output.push_str("pub struct MidrangeBase {\n");
            output.push_str("    pub base_offset: u32,\n");
            output.push_str("    pub status: &'static str,\n");
            output.push_str("    pub notes: &'static str,\n");
            output.push_str("}\n\n");
        }

        // Generate dungeon formula bases
        if let Some(dungeon) = formulas.get("dungeon_formula").and_then(|v| v.as_object()) {
            output.push_str("// ============================================================================\n");
            output.push_str("// DUNGEON FORMULA (verified from ground_truth_offsets.json)\n");
            output.push_str("// ============================================================================\n\n");

            output.push_str("/// Dungeon base offsets by map area\n");
            output.push_str("/// Formula: byte_offset = base + section * section_size + local_id / 8\n");
            output.push_str("pub static VERIFIED_DUNGEON_BASES: Lazy<HashMap<u32, DungeonBase>> = Lazy::new(|| {\n");
            output.push_str("    HashMap::from([\n");

            let mut entries: Vec<_> = dungeon.iter().collect();
            entries.sort_by_key(|(k, _)| k.parse::<u32>().unwrap_or(0));

            for (area, info) in entries {
                if let Some(base_offset) = info.get("base_offset").and_then(|v| v.as_u64()) {
                    let section_size = info.get("section_size").and_then(|v| v.as_u64()).unwrap_or(1125);
                    let status = info.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let notes = info.get("notes").and_then(|v| v.as_str()).unwrap_or("");
                    output.push_str(&format!(
                        "        ({}, DungeonBase {{ base_offset: {}, section_size: {}, status: \"{}\", notes: \"{}\" }}),\n",
                        area, base_offset, section_size, status, notes.replace("\"", "\\\"")
                    ));
                }
            }

            output.push_str("    ])\n");
            output.push_str("});\n\n");

            output.push_str("#[derive(Debug, Clone)]\n");
            output.push_str("pub struct DungeonBase {\n");
            output.push_str("    pub base_offset: u32,\n");
            output.push_str("    pub section_size: u32,\n");
            output.push_str("    pub status: &'static str,\n");
            output.push_str("    pub notes: &'static str,\n");
            output.push_str("}\n\n");
        }
    }

    // Generate helper functions
    output.push_str("// ============================================================================\n");
    output.push_str("// HELPER FUNCTIONS\n");
    output.push_str("// ============================================================================\n\n");

    output.push_str("/// Calculate byte offset and bit position for a block-based flag (5-digit)\n");
    output.push_str("/// Note: Some blocks have sub-ranges with different bases (e.g., 71600 within 71000)\n");
    output.push_str("pub fn calculate_block_flag_offset(flag_id: u32) -> Option<(u32, u8)> {\n");
    output.push_str("    // First try sub-block at 100-flag granularity (e.g., 71600 for flag 71607)\n");
    output.push_str("    let sub_block_start = (flag_id / 100) * 100;\n");
    output.push_str("    if let Some(base) = VERIFIED_BLOCK_BASES.get(&sub_block_start) {\n");
    output.push_str("        let relative = flag_id - sub_block_start;\n");
    output.push_str("        let byte_offset = base.base_offset + relative / 8;\n");
    output.push_str("        let bit_position = 7 - ((flag_id % 8) as u8);\n");
    output.push_str("        return Some((byte_offset, bit_position));\n");
    output.push_str("    }\n");
    output.push_str("    \n");
    output.push_str("    // Fall back to main block at 1000-flag granularity (e.g., 71000)\n");
    output.push_str("    let block_start = (flag_id / 1000) * 1000;\n");
    output.push_str("    let base = VERIFIED_BLOCK_BASES.get(&block_start)?;\n");
    output.push_str("    let relative = flag_id - block_start;\n");
    output.push_str("    let byte_offset = base.base_offset + relative / 8;\n");
    output.push_str("    let bit_position = 7 - ((flag_id % 8) as u8);\n");
    output.push_str("    Some((byte_offset, bit_position))\n");
    output.push_str("}\n\n");

    output.push_str("/// Calculate byte offset and bit position for a tile-based flag (10-digit)\n");
    output.push_str("pub fn calculate_tile_flag_offset(flag_id: u32) -> Option<(u32, u8)> {\n");
    output.push_str("    if flag_id < 1_000_000_000 { return None; }\n");
    output.push_str("    \n");
    output.push_str("    let tile_index = (flag_id - 1_000_000_000) / 10000;\n");
    output.push_str("    let local_id = flag_id % 10000;\n");
    output.push_str("    \n");
    output.push_str("    // LocalId >= 7000 has no storage\n");
    output.push_str("    if local_id >= TILE_MAX_LOCAL_ID { return None; }\n");
    output.push_str("    \n");
    output.push_str("    let row = tile_index / 100;\n");
    output.push_str("    let col = tile_index % 100;\n");
    output.push_str("    \n");
    output.push_str("    let slot = (row as i32 - TILE_ROW_BASE as i32) * TILE_SLOTS_PER_ROW as i32\n");
    output.push_str("             + (col as i32 - TILE_COL_BASE as i32);\n");
    output.push_str("    if slot < 0 { return None; }\n");
    output.push_str("    \n");
    output.push_str("    let byte_offset = VERIFIED_TILE_BASE_OFFSET + (slot as u32) * TILE_BYTES_PER_SLOT + local_id / 8;\n");
    output.push_str("    let bit_position = 7 - ((local_id % 8) as u8);\n");
    output.push_str("    Some((byte_offset, bit_position))\n");
    output.push_str("}\n\n");

    output.push_str("/// Calculate byte offset and bit position for a midrange flag (6-digit, 100000-999999)\n");
    output.push_str("/// Used for sorceries, incantations, ashes of war unlock flags\n");
    output.push_str("pub fn calculate_midrange_flag_offset(flag_id: u32) -> Option<(u32, u8)> {\n");
    output.push_str("    if !(100_000..1_000_000).contains(&flag_id) { return None; }\n");
    output.push_str("    \n");
    output.push_str("    // Try exact block match first (1000-flag granularity)\n");
    output.push_str("    let block_start = (flag_id / 1000) * 1000;\n");
    output.push_str("    if let Some(base) = VERIFIED_MIDRANGE_BASES.get(&block_start) {\n");
    output.push_str("        let relative = flag_id - block_start;\n");
    output.push_str("        let byte_offset = base.base_offset + relative / 8;\n");
    output.push_str("        let bit_position = 7 - ((flag_id % 8) as u8);\n");
    output.push_str("        return Some((byte_offset, bit_position));\n");
    output.push_str("    }\n");
    output.push_str("    \n");
    output.push_str("    // Try 10000-flag block granularity (e.g., 540000 for all 54xxxx flags)\n");
    output.push_str("    let block_10k = (flag_id / 10000) * 10000;\n");
    output.push_str("    if let Some(base) = VERIFIED_MIDRANGE_BASES.get(&block_10k) {\n");
    output.push_str("        let relative = flag_id - block_10k;\n");
    output.push_str("        let byte_offset = base.base_offset + relative / 8;\n");
    output.push_str("        let bit_position = 7 - ((flag_id % 8) as u8);\n");
    output.push_str("        return Some((byte_offset, bit_position));\n");
    output.push_str("    }\n");
    output.push_str("    \n");
    output.push_str("    None\n");
    output.push_str("}\n\n");

    output.push_str("/// Calculate byte offset and bit position for a dungeon-based flag (8-digit)\n");
    output.push_str("pub fn calculate_dungeon_flag_offset(flag_id: u32) -> Option<(u32, u8)> {\n");
    output.push_str("    if !(10_000_000..100_000_000).contains(&flag_id) { return None; }\n");
    output.push_str("    \n");
    output.push_str("    let area = flag_id / 1_000_000;\n");
    output.push_str("    let section = (flag_id / 10_000) % 100;\n");
    output.push_str("    let local_id = flag_id % 10_000;\n");
    output.push_str("    \n");
    output.push_str("    let base = VERIFIED_DUNGEON_BASES.get(&area)?;\n");
    output.push_str("    if base.base_offset == 0 { return None; } // Unverified\n");
    output.push_str("    \n");
    output.push_str("    let byte_offset = base.base_offset + section * base.section_size + local_id / 8;\n");
    output.push_str("    let bit_position = 7 - ((local_id % 8) as u8);\n");
    output.push_str("    Some((byte_offset, bit_position))\n");
    output.push_str("}\n");

    // Write the generated file
    let output_path = out_dir.join("ground_truth.rs");
    let mut file = fs::File::create(&output_path)?;
    file.write_all(output.as_bytes())?;

    println!("cargo:warning=Generated {} from ground_truth_offsets.json", output_path.display());

    Ok(())
}
