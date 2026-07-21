//! `knowledge gen-dungeon-pickups [<ItemLotParam_map.param.xml>] [--out PATH]`
//!
//! Regenerates `src/db/dungeon_pickups.rs` deterministically from the primary
//! source `ItemLotParam_map` (regulation 1.16.1, evidence corpus `game-extracts`).
//! This is the anti-drift replacement for the hand-maintained table: the table is
//! GENERATED, not edited. `tests/dungeon_pickups_generated.rs` re-runs `generate`
//! against the same source and asserts the committed file is byte-identical, so a
//! hand-edit or a stale table fails the test.
//!
//! Selection: every row whose `getItemFlagId` is an 8-digit legacy-dungeon flag
//! (10,000,000..44,000,000) with localId >= 7000 AND that grants an item
//! (lotItemId01 != 0). Empty lots (a flag but no item) are not pickups.
//!
//! Per-field provenance is documented in the emitted header. Everything is derived
//! from the primary source: names from `paramdexName` (an item id with no
//! paramdexName on its own row borrows the name any other row gives that id),
//! category from a documented classifier over the name + `lotItemCategory`.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// (area number, display name). Single source for both `get_dungeon_area_name`
/// and each entry's `region`, so they cannot disagree.
const AREA_NAMES: &[(u32, &str)] = &[
    (10, "Stormveil Castle"),
    (11, "Leyndell"),
    (12, "Underground"),
    (13, "Crumbling Farum Azula"),
    (14, "Academy of Raya Lucaria"),
    (15, "Haligtree"),
    (16, "Volcano Manor"),
    (18, "Roundtable Hold"),
    (20, "Stranded Graveyard"),
    (21, "Haligtree (Elphael)"),
    (22, "Castle Sol"),
    (28, "DLC Dungeon"),
    (30, "Catacombs"),
    (31, "Caves"),
    (32, "Tunnels"),
    (34, "Divine Towers"),
    (35, "Mohgwyn Palace"),
    (39, "Elden Throne"),
    (40, "Hero's Graves"),
    (41, "Minor Dungeons"),
    (42, "Crystal Caves"),
    (43, "Evergaols"),
    (59, "Unknown 59"),
    (99, "Special"),
];

fn area_name(area: u32) -> &'static str {
    AREA_NAMES
        .iter()
        .find(|(a, _)| *a == area)
        .map(|(_, n)| *n)
        .unwrap_or("Unknown")
}

/// Refined pickup category from the item name and its `lotItemCategory` (the
/// ItemLotParam item-type: 2=weapon, 3=protector, 4=accessory, 5=gem/AoW, 1=good).
/// Special goods are recognised by name (they share `goodsType` so the name is the
/// only reliable discriminator); all other goods are Consumables.
fn classify(name: &str, lot_cat: Option<&str>) -> &'static str {
    let n = name.to_lowercase();
    if n.contains("golden rune")
        || n.contains("rune arc")
        || n.contains("lord's rune")
        || n.contains("hero's rune")
        || n.contains("numen's rune")
    {
        return "GoldenRunes";
    }
    if n.contains("somber smithing stone") {
        return "SomberStones";
    }
    if n.contains("smithing stone") {
        return "SmithingStones";
    }
    if n.contains("glovewort") {
        return "Glovewort";
    }
    match lot_cat {
        Some("2") => "Weapons",
        Some("3") => "Armor",
        Some("4") => "Talismans",
        Some("5") => "AshesOfWar",
        _ => "Consumables",
    }
}

/// Strip a leading "[...]" region tag from a paramdexName:
/// "[LD - Stormveil] Wooden Greatshield" -> "Wooden Greatshield".
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

struct Entry {
    item_lot_id: u32,
    event_flag: u32,
    item_id: u32,
    name: String,
    quantity: u32,
    category: &'static str,
    region: &'static str,
    area: u32,
    section: u32,
}

/// Pure, deterministic: primary-source XML text -> the full contents of
/// `src/db/dungeon_pickups.rs`. Re-running on the same input yields identical bytes.
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

    // Item-granting dungeon-pickup rows, keyed (and thus sorted) by item_lot_id.
    let mut entries: BTreeMap<u32, Entry> = BTreeMap::new();
    for r in &rows {
        let flag: u32 = match r.attribute("getItemFlagId").and_then(|s| s.parse().ok()) {
            Some(f) => f,
            None => continue,
        };
        if !(10_000_000..44_000_000).contains(&flag) || flag % 10_000 < 7_000 {
            continue;
        }
        let item_id: u32 = r.attribute("lotItemId01").and_then(|s| s.parse().ok()).unwrap_or(0);
        if item_id == 0 {
            continue; // empty lot: a flag that grants no item is not a pickup
        }
        let lot: u32 = r.attribute("id").and_then(|s| s.parse().ok()).ok_or("row missing id")?;
        let quantity: u32 = r.attribute("lotItemNum01").and_then(|s| s.parse().ok()).unwrap_or(1);
        let lot_cat = r.attribute("lotItemCategory01");
        let name: String = match r.attribute("paramdexName").map(strip_prefix) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => id_name.get(&item_id).map(|s| s.to_string()).unwrap_or_else(|| format!("Item {item_id}")),
        };
        let area = (flag / 1_000_000) % 100;
        let section = (flag / 10_000) % 100;
        entries.insert(
            lot,
            Entry {
                item_lot_id: lot,
                event_flag: flag,
                item_id,
                category: classify(&name, lot_cat),
                name,
                quantity,
                region: area_name(area),
                area,
                section,
            },
        );
    }

    Ok(render(&entries))
}

fn render(entries: &BTreeMap<u32, Entry>) -> String {
    let mut s = String::new();
    s.push_str(&header(entries.len()));
    s.push_str("pub fn get_dungeon_area_name(area: u32) -> &'static str {\n    match area {\n");
    for (a, n) in AREA_NAMES {
        s.push_str(&format!("        {a} => \"{}\",\n", esc(n)));
    }
    s.push_str("        _ => \"Unknown\",\n    }\n}\n\n");
    s.push_str(&format!("/// All dungeon pickups ({} entries)\n", entries.len()));
    s.push_str("pub static DUNGEON_PICKUPS: &[DungeonPickup] = &[\n");
    for e in entries.values() {
        s.push_str("    DungeonPickup {\n");
        s.push_str(&format!("        item_lot_id: {},\n", e.item_lot_id));
        s.push_str(&format!("        event_flag: {},\n", e.event_flag));
        s.push_str(&format!("        item_id: {},\n", e.item_id));
        s.push_str(&format!("        name: \"{}\",\n", esc(&e.name)));
        s.push_str(&format!("        quantity: {},\n", e.quantity));
        s.push_str(&format!("        category: PickupCategory::{},\n", e.category));
        s.push_str(&format!("        region: \"{}\",\n", esc(e.region)));
        s.push_str(&format!("        dungeon_area: {},\n", e.area));
        s.push_str(&format!("        section: {},\n", e.section));
        s.push_str("    },\n");
    }
    s.push_str("];\n");
    s
}

fn header(count: usize) -> String {
    format!(
        "//! Dungeon Pickup Database\n\
//!\n\
//! GENERATED — do not hand-edit. Regenerate with\n\
//!   `er-save-editor knowledge gen-dungeon-pickups`\n\
//! (`src/knowledge/gen_dungeon_pickups.rs`). `tests/dungeon_pickups_generated.rs`\n\
//! asserts this file equals the generator's output for the committed source, so a\n\
//! hand-edit or a stale table fails the test.\n\
//!\n\
//! Source: the primary `ItemLotParam_map.param.xml` (regulation 1.16.1, evidence\n\
//! corpus `game-extracts`, verified against knowledge/manifests/game-extracts.sha256).\n\
//!\n\
//! Set: every row whose `getItemFlagId` is an 8-digit legacy-dungeon flag\n\
//! (10,000,000..44,000,000) with localId >= 7000 AND that grants an item\n\
//! (lotItemId01 != 0) -> {count} pickups. Empty lots (a flag but no item) are\n\
//! excluded. Per row: item_lot_id = row id, event_flag = getItemFlagId\n\
//! (authoritative — several lot rows can share one flag, e.g. an armor set's\n\
//! pieces), item_id = lotItemId01, quantity = lotItemNum01 (default 1), name from\n\
//! paramdexName, area/section derived from the flag, category from a documented\n\
//! classifier over the name + lotItemCategory.\n\
\n\
use crate::db::pickup_data::PickupCategory;\n\
\n\
/// A dungeon pickup entry\n\
#[derive(Debug, Clone)]\n\
pub struct DungeonPickup {{\n\
    pub item_lot_id: u32,\n\
    pub event_flag: u32,\n\
    pub item_id: u32,\n\
    pub name: &'static str,\n\
    pub quantity: u32,\n\
    pub category: PickupCategory,\n\
    pub region: &'static str,\n\
    pub dungeon_area: u32,\n\
    pub section: u32,\n\
}}\n\
\n\
/// Dungeon area names for display\n"
    )
}

/// Resolve the primary-source XML path: an explicit argument, or the
/// `game-extracts` corpus location from the evidence catalog.
pub fn resolve_source(args: &[String]) -> Result<PathBuf, String> {
    if let Some(p) = args.iter().find(|a| !a.starts_with("--")) {
        return Ok(PathBuf::from(p));
    }
    source_from_catalog(&std::env::current_dir().map_err(|e| e.to_string())?)
}

/// `<decompiled root>/regulation-bin/ItemLotParam_map.param.xml` from the catalog.
pub fn source_from_catalog(repo_root: &Path) -> Result<PathBuf, String> {
    let text = std::fs::read_to_string(repo_root.join("knowledge/evidence-catalog.json"))
        .map_err(|e| format!("read catalog: {e}"))?;
    let cat: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("catalog parse: {e}"))?;
    let root = cat["roots"]["decompiled"]
        .as_str()
        .ok_or("catalog: roots.decompiled missing")?;
    Ok(Path::new(root).join("regulation-bin/ItemLotParam_map.param.xml"))
}

pub fn cmd_gen_dungeon_pickups(args: &[String]) -> Result<(), String> {
    let src_path = resolve_source(args)?;
    let out = args
        .windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(|| PathBuf::from("src/db/dungeon_pickups.rs"));

    let xml = std::fs::read_to_string(&src_path)
        .map_err(|e| format!("read {}: {e}", src_path.display()))?;
    let generated = generate(&xml)?;
    let count = generated.matches("DungeonPickup {").count().saturating_sub(1);
    std::fs::write(&out, &generated).map_err(|e| format!("write {}: {e}", out.display()))?;
    println!(
        "wrote {} ({count} dungeon pickups) from {}",
        out.display(),
        src_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed table must equal the generator's output for the committed
    /// source — this is the anti-drift guard: a hand-edit or a stale table fails
    /// here. Skips (does not fail) when the game-extract is absent, since it is
    /// out-of-repo evidence not present in CI; runs on any machine that has it.
    #[test]
    fn committed_table_matches_generator() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = match source_from_catalog(repo_root) {
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
        let committed = std::fs::read_to_string(repo_root.join("src/db/dungeon_pickups.rs"))
            .expect("read committed table");
        assert_eq!(
            committed, generated,
            "src/db/dungeon_pickups.rs is stale or was hand-edited. \
             Regenerate: `cargo run -- knowledge gen-dungeon-pickups`."
        );
    }

    /// Self-contained (no evidence needed): selection, empty-lot exclusion,
    /// category classification, and determinism.
    #[test]
    fn generation_is_deterministic_and_correct() {
        let xml = r#"<?xml version="1.0"?><param><rows>
            <row id="30020000" lotItemId01="6000000" lotItemCategory01="2" getItemFlagId="30027000" paramdexName="[LD - Catacombs] Test Sword" />
            <row id="30020001" lotItemId01="0" lotItemCategory01="0" getItemFlagId="30027001" paramdexName="[LD - Catacombs]" />
            <row id="30020002" lotItemId01="2900" lotItemCategory01="1" getItemFlagId="30027002" paramdexName="[LD - Catacombs] Golden Rune [1]" lotItemNum01="3" />
            <row id="00000042" lotItemId01="100" lotItemCategory01="1" getItemFlagId="12345" paramdexName="not a dungeon pickup" />
        </rows></param>"#;
        let a = generate(xml).unwrap();
        assert_eq!(a, generate(xml).unwrap(), "generator must be deterministic");
        assert!(a.contains("/// All dungeon pickups (2 entries)"), "only 2 item-granting dungeon rows");
        assert!(a.contains("item_lot_id: 30020000,"));
        assert!(a.contains("category: PickupCategory::Weapons,"), "sword -> Weapons");
        assert!(a.contains("category: PickupCategory::GoldenRunes,"), "Golden Rune -> GoldenRunes");
        assert!(a.contains("quantity: 3,"));
        assert!(a.contains("region: \"Catacombs\","));
        assert!(!a.contains("30020001"), "empty lot (item 0) excluded");
        assert!(!a.contains("00000042"), "non-dungeon flag excluded");
    }
}
