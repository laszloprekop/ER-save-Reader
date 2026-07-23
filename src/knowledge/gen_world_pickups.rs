//! `knowledge gen-world-pickups [<ItemLotParam_map.param.xml>] [--out PATH]`
//!
//! Regenerates `src/db/world_pickups.rs` deterministically from the primary
//! source `ItemLotParam_map` (regulation 1.16.1, evidence corpus `game-extracts`).
//! Same anti-drift contract as `gen_dungeon_pickups`: the table is GENERATED, not
//! edited, and a unit test asserts the committed file equals the generator's
//! output for the committed source.
//!
//! Selection: every row that grants an item (`lotItemId01 != 0`) and carries a
//! `getItemFlagId`, EXCEPT the legacy-dungeon pickups that `gen_dungeon_pickups`
//! owns (8-digit flag 10,000,000..44,000,000 with localId >= 7000). The two
//! generators therefore partition the item-granting flagged rows of the primary
//! source exactly, with no overlap — the previous table duplicated every dungeon
//! pickup into the world browser, which has its own view.
//!
//! `flag_id` is the raw `getItemFlagId`, deliberately: `ResolvedFlags::tile_pickup`
//! accepts either the getItemFlagId or the row_id form and normalises. Do NOT
//! store a "row id" here — for 124 of the 1,691 ten-digit rows the param's own
//! row id is not `getItemFlagId - 7000`, so a row-id-keyed table addresses the
//! wrong bit (see docs/BACKLOG.md).

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// `PickupItemType` variant from the row's `lotItemCategory01` (the ItemLotParam
/// item-type). 6 is a CUSTOM WEAPON — an `EquipParamCustomWeapon` row bundling a
/// base weapon with an ash of war and an upgrade level (verified: category-6 rows
/// resolve there, e.g. 5000 = "Banished Knight's Halberd +8 - Spinning Strikes",
/// baseWepId 18030000) — so it reads as a Weapon. Category 0 is unset in the
/// source and stays `Unknown` rather than being guessed.
fn item_type(lot_cat: Option<&str>) -> &'static str {
    match lot_cat {
        Some("1") => "Good",
        Some("2") => "Weapon",
        Some("3") => "Armor",
        Some("4") => "Accessory",
        Some("5") => "AshOfWar",
        Some("6") => "Weapon",
        _ => "Unknown",
    }
}

/// Strip a leading "[...]" annotation from a paramdexName:
/// "[Stormveil - Godrick] Godrick's Great Rune" -> "Godrick's Great Rune".
fn strip_prefix(pn: &str) -> &str {
    let t = pn.trim_start();
    if let Some(rest) = t.strip_prefix('[') {
        if let Some(idx) = rest.find(']') {
            return rest[idx + 1..].trim_start();
        }
    }
    t
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Map region and open-world tile coordinates from the flag id. Ten-digit tile
/// flags are `<map><xx><yy><local>`: 10xxxxxxxx is the Lands Between grid (m60),
/// 20xxxxxxxx the Shadow of the Erdtree grid (m61). Everything else (block flags,
/// simple flags, the handful of legacy-map flags with localId < 7000) has no map
/// coordinate in the primary source and is honestly `Unknown` at (0, 0) — no real
/// tile is (0, 0), the grid starts at x = 33.
fn region_and_tile(flag: u32) -> (&'static str, u8, u8) {
    let (region, grid) = match flag {
        1_000_000_000..=1_999_999_999 => ("Lands Between", 1_000_000_000),
        2_000_000_000..=2_999_999_999 => ("Shadow of the Erdtree", 2_000_000_000),
        _ => return ("Unknown", 0, 0),
    };
    let tile_index = (flag - grid) / 10_000;
    (region, (tile_index / 100) as u8, (tile_index % 100) as u8)
}

struct Entry {
    lot_id: u32,
    flag_id: u32,
    item_id: u32,
    item_type: &'static str,
    item_name: String,
    quantity: u8,
    region: &'static str,
    tile_x: u8,
    tile_y: u8,
}

/// True for the rows `gen_dungeon_pickups` owns; this generator skips them.
fn is_dungeon_pickup(flag: u32) -> bool {
    (10_000_000..44_000_000).contains(&flag) && flag % 10_000 >= 7_000
}

/// Pure, deterministic: primary-source XML text -> the full contents of
/// `src/db/world_pickups.rs`. Re-running on the same input yields identical bytes.
pub fn generate(xml: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("XML parse: {e}"))?;
    let rows: Vec<roxmltree::Node> = doc.descendants().filter(|n| n.has_tag_name("row")).collect();

    // item_id -> name, from any row that names that id (document order; first wins).
    let mut id_name: HashMap<u32, &str> = HashMap::new();
    for r in &rows {
        if let (Some(id), Some(pn)) = (r.attribute("lotItemId01"), r.attribute("paramdexName")) {
            let name = strip_prefix(pn);
            if !name.is_empty() {
                if let Ok(iid) = id.parse::<u32>() {
                    id_name.entry(iid).or_insert(name);
                }
            }
        }
    }

    // Item-granting non-dungeon rows, keyed (and thus sorted) by lot id.
    let mut entries: BTreeMap<u32, Entry> = BTreeMap::new();
    for r in &rows {
        let flag: u32 = match r.attribute("getItemFlagId").and_then(|s| s.parse().ok()) {
            Some(f) if f != 0 => f,
            _ => continue,
        };
        if is_dungeon_pickup(flag) {
            continue; // owned by gen_dungeon_pickups
        }
        let item_id: u32 = r.attribute("lotItemId01").and_then(|s| s.parse().ok()).unwrap_or(0);
        if item_id == 0 {
            continue; // empty lot: a flag that grants no item is not a pickup
        }
        let lot_id: u32 = r.attribute("id").and_then(|s| s.parse().ok()).ok_or("row missing id")?;
        let quantity: u32 = r.attribute("lotItemNum01").and_then(|s| s.parse().ok()).unwrap_or(1);
        let quantity = u8::try_from(quantity)
            .map_err(|_| format!("lot {lot_id}: quantity {quantity} exceeds u8"))?;
        let item_name: String = match r.attribute("paramdexName").map(strip_prefix) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => id_name
                .get(&item_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Item {item_id}")),
        };
        let (region, tile_x, tile_y) = region_and_tile(flag);
        entries.insert(
            lot_id,
            Entry {
                lot_id,
                flag_id: flag,
                item_id,
                item_type: item_type(r.attribute("lotItemCategory01")),
                item_name,
                quantity,
                region,
                tile_x,
                tile_y,
            },
        );
    }

    Ok(render(&entries))
}

fn render(entries: &BTreeMap<u32, Entry>) -> String {
    let mut s = String::new();
    s.push_str(&header(entries.len()));
    s.push_str(&format!(
        "/// All world pickups ({} entries), indexed by item lot ID\n",
        entries.len()
    ));
    s.push_str("pub static WORLD_PICKUPS: Lazy<HashMap<u32, WorldPickup>> = Lazy::new(|| {\n");
    s.push_str("    let mut map = HashMap::new();\n");
    for e in entries.values() {
        s.push_str(&format!("    map.insert({}, WorldPickup {{\n", e.lot_id));
        s.push_str(&format!("        lot_id: {},\n", e.lot_id));
        s.push_str(&format!("        flag_id: {},\n", e.flag_id));
        s.push_str(&format!("        item_id: {},\n", e.item_id));
        s.push_str(&format!("        item_type: PickupItemType::{},\n", e.item_type));
        s.push_str(&format!("        item_name: \"{}\",\n", esc(&e.item_name)));
        s.push_str(&format!("        quantity: {},\n", e.quantity));
        s.push_str(&format!("        region: \"{}\",\n", esc(e.region)));
        s.push_str(&format!("        tile_x: {},\n", e.tile_x));
        s.push_str(&format!("        tile_y: {},\n", e.tile_y));
        s.push_str("    });\n");
    }
    s.push_str("    map\n});\n");
    s.push_str(TAIL);
    s
}

/// The query helpers, unchanged across regenerations.
const TAIL: &str = r#"
/// Index of world pickups by flag ID
pub static PICKUPS_BY_FLAG: Lazy<HashMap<u32, u32>> = Lazy::new(|| {
    WORLD_PICKUPS.iter()
        .map(|(lot_id, pickup)| (pickup.flag_id, *lot_id))
        .collect()
});

/// Get world pickup by lot ID
pub fn get_pickup(lot_id: u32) -> Option<&'static WorldPickup> {
    WORLD_PICKUPS.get(&lot_id)
}

/// Get world pickup by flag ID
pub fn get_pickup_by_flag(flag_id: u32) -> Option<&'static WorldPickup> {
    PICKUPS_BY_FLAG.get(&flag_id)
        .and_then(|lot_id| WORLD_PICKUPS.get(lot_id))
}

/// Get all pickups in a region
pub fn get_pickups_in_region(region: &str) -> Vec<&'static WorldPickup> {
    WORLD_PICKUPS.values()
        .filter(|p| p.region == region)
        .collect()
}

/// Get all unique regions
pub fn get_regions() -> Vec<&'static str> {
    let mut regions: Vec<_> = WORLD_PICKUPS.values()
        .map(|p| p.region)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    regions.sort();
    regions
}
"#;

/// Doc comment + type definitions. Kept as a raw string (not a `\`-continued
/// literal) so the emitted indentation survives; `{COUNT}` is the only slot.
const HEADER: &str = r##"//! World Pickup Database
//!
//! GENERATED — do not hand-edit. Regenerate with
//!   `er-save-reader knowledge gen-world-pickups`
//! (`src/knowledge/gen_world_pickups.rs`). `gen_world_pickups`'s own unit test
//! asserts this file equals the generator's output for the committed source, so a
//! hand-edit or a stale table fails the test.
//!
//! Source: the primary `ItemLotParam_map.param.xml` (regulation 1.16.1, evidence
//! corpus `game-extracts`, verified against knowledge/manifests/game-extracts.sha256).
//!
//! Set: every row that grants an item (lotItemId01 != 0) and carries a
//! getItemFlagId, MINUS the legacy-dungeon pickups owned by
//! `src/db/dungeon_pickups.rs` (8-digit flag 10,000,000..44,000,000 whose
//! localId is 7000 or above) -> {COUNT} pickups. The two tables partition the
//! primary source's item-granting flagged rows exactly; there is no overlap, and
//! each has its own database view.
//!
//! Per row: lot_id = row id, flag_id = getItemFlagId (raw — `ResolvedFlags::tile_pickup`
//! normalises the high-localId form itself; storing a row_id here would read the
//! wrong bit for the 124 ten-digit rows whose row id is not flag - 7000),
//! item_id = lotItemId01, quantity = lotItemNum01 (default 1), item_name from
//! paramdexName with its "[...]" annotation stripped (an item id whose own row
//! carries no name borrows the name any other row gives that id), item_type from
//! lotItemCategory01, region + tile from the flag id.

use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickupItemType {
    Weapon,
    Armor,
    Accessory,
    Good,
    AshOfWar,
    Unknown,
}

/// A world pickup entry
#[derive(Debug, Clone)]
pub struct WorldPickup {
    pub lot_id: u32,
    pub flag_id: u32,
    pub item_id: u32,
    pub item_type: PickupItemType,
    pub item_name: &'static str,
    pub quantity: u8,
    /// Map region the flag id places this pickup in, or "Unknown" when the flag
    /// carries no map coordinate (block/simple flags).
    pub region: &'static str,
    /// Open-world tile coordinates from the flag id; (0, 0) means "no tile" —
    /// the grid starts at x = 33.
    pub tile_x: u8,
    pub tile_y: u8,
}

"##;

fn header(count: usize) -> String {
    HEADER.replace("{COUNT}", &count.to_string())
}

/// Resolve the primary-source XML path: an explicit argument, or the
/// `game-extracts` corpus location from the evidence catalog.
pub fn resolve_source(args: &[String]) -> Result<PathBuf, String> {
    if let Some(p) = args.iter().find(|a| !a.starts_with("--")) {
        return Ok(PathBuf::from(p));
    }
    crate::knowledge::gen_dungeon_pickups::source_from_catalog(
        &std::env::current_dir().map_err(|e| e.to_string())?,
    )
}

pub fn cmd_gen_world_pickups(args: &[String]) -> Result<(), String> {
    let src_path = resolve_source(args)?;
    let out = args
        .windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(|| PathBuf::from("src/db/world_pickups.rs"));

    let xml = std::fs::read_to_string(&src_path)
        .map_err(|e| format!("read {}: {e}", src_path.display()))?;
    let generated = generate(&xml)?;
    let count = generated.matches("map.insert(").count();
    std::fs::write(&out, &generated).map_err(|e| format!("write {}: {e}", out.display()))?;
    println!(
        "wrote {} ({count} world pickups) from {}",
        out.display(),
        src_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::gen_dungeon_pickups;
    use std::path::Path;

    /// The committed table must equal the generator's output for the committed
    /// source — the anti-drift guard: a hand-edit or a stale table fails here.
    /// Skips (does not fail) when the game-extract is absent, since it is
    /// out-of-repo evidence not present in CI.
    #[test]
    fn committed_table_matches_generator() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = match gen_dungeon_pickups::source_from_catalog(repo_root) {
            Ok(p) if p.exists() => p,
            _ => {
                eprintln!(
                    "skip committed_table_matches_generator: ItemLotParam_map extract absent \
                     (game-extracts corpus). Run where the evidence is present to check drift."
                );
                return;
            }
        };
        let xml = std::fs::read_to_string(&src).expect("read source xml");
        let generated = generate(&xml).expect("generate");
        let committed = std::fs::read_to_string(repo_root.join("src/db/world_pickups.rs"))
            .expect("read committed table");
        assert_eq!(
            committed, generated,
            "src/db/world_pickups.rs is stale or was hand-edited. \
             Regenerate: `cargo run -- knowledge gen-world-pickups`."
        );
    }

    /// The two pickup tables must partition the primary source, not overlap: a
    /// lot id selected here must never be selected by `gen_dungeon_pickups`.
    /// This is what the previous table got wrong — it carried every dungeon
    /// pickup as well, duplicating the dungeon browser.
    #[test]
    fn world_and_dungeon_selections_are_disjoint() {
        let xml = r#"<?xml version="1.0"?><param><rows>
            <row id="1042320000" lotItemId01="15110" lotItemCategory01="1" getItemFlagId="1042327000" lotItemNum01="3" paramdexName="Great Dragonfly Head" />
            <row id="30020000" lotItemId01="6000000" lotItemCategory01="2" getItemFlagId="30027000" paramdexName="[LD - Catacombs] Test Sword" />
        </rows></param>"#;
        let world = generate(xml).unwrap();
        let dungeon = gen_dungeon_pickups::generate(xml).unwrap();
        assert!(world.contains("lot_id: 1042320000,"), "tile pickup is a world pickup");
        assert!(!world.contains("30020000"), "dungeon pickup must not appear in the world table");
        assert!(dungeon.contains("item_lot_id: 30020000,"));
        assert!(!dungeon.contains("1042320000"));
    }

    /// Self-contained (no evidence needed): selection, empty-lot exclusion, name
    /// resolution, item typing, tile derivation, and determinism.
    #[test]
    fn generation_is_deterministic_and_correct() {
        let xml = r#"<?xml version="1.0"?><param><rows>
            <row id="1044360040" lotItemId01="10160" lotItemCategory01="1" getItemFlagId="1044367040" paramdexName="Somber Smithing Stone [1]" />
            <row id="2045440010" lotItemId01="4411061" lotItemCategory01="6" getItemFlagId="2045447010" paramdexName="Great Katana - Overhead Stance" />
            <row id="510010" lotItemId01="8148" lotItemCategory01="1" getItemFlagId="510010" paramdexName="[Stormveil - Godrick] Remembrance of the Grafted" />
            <row id="15001210" lotItemId01="2900" lotItemCategory01="0" getItemFlagId="15001210" paramdexName="[LD - Elphael / Miquella's Haligtree]" />
            <row id="30020002" lotItemId01="2900" lotItemCategory01="1" getItemFlagId="30027002" paramdexName="[LD - Catacombs] Golden Rune [1]" />
            <row id="99999998" lotItemId01="0" lotItemCategory01="1" getItemFlagId="60001" paramdexName="empty lot" />
            <row id="99999999" lotItemId01="123" lotItemCategory01="1" getItemFlagId="0" paramdexName="no flag" />
        </rows></param>"#;
        let a = generate(xml).unwrap();
        assert_eq!(a, generate(xml).unwrap(), "generator must be deterministic");
        assert!(
            a.contains("/// All world pickups (4 entries)"),
            "4 selected; the dungeon pickup, the empty lot and the flagless row are rejected"
        );

        // Tile flag 1044367040 -> tile index 4436 -> (44, 36), Lands Between.
        assert!(a.contains("tile_x: 44,\n        tile_y: 36,"));
        assert!(a.contains("region: \"Lands Between\","));
        // DLC grid 20xxxxxxxx -> Shadow of the Erdtree, and category 6 is a weapon.
        assert!(a.contains("region: \"Shadow of the Erdtree\","));
        assert!(a.contains("item_type: PickupItemType::Weapon,"));
        // The "[...]" annotation is stripped from the name.
        assert!(a.contains("item_name: \"Remembrance of the Grafted\","));
        // A row whose paramdexName is only an annotation borrows the item's name.
        assert!(a.contains("item_name: \"Golden Rune [1]\","), "name borrowed by item id");
        // Unset lotItemCategory stays Unknown rather than being guessed.
        assert!(a.contains("item_type: PickupItemType::Unknown,"));
        // Non-tile flags carry no coordinate.
        assert!(a.contains("tile_x: 0,\n        tile_y: 0,"));
        assert!(!a.contains("99999998"), "empty lot excluded");
        assert!(!a.contains("99999999"), "row without a getItemFlagId excluded");
    }
}
