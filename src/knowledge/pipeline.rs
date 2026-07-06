//! `knowledge run` — the evidence → claims pipeline (migration step 3,
//! ADR-0004: pipeline-generated claims store with status ladder).
//!
//! Method (validated empirically 2026-07-05 against the Confessor b-series):
//!
//! 1. Load hand-written hypothesis input `knowledge/inputs/attributed-transitions.json`
//!    (attributed before/after capture pairs) and the alloclist layout
//!    (`knowledge/game/eventflag-alloclists.json`).
//! 2. Verify-on-read: every evidence file is sha256-checked against the
//!    evidence-catalog manifest before use (ADR-0001).
//! 3. Per file: detect the grace-family base with the reference implementation
//!    (`crates/wasm-event-flags`, ADR-0005).
//! 4. Per pair: grace-aligned isolated-flip extraction — compare
//!    `before[grace_b + i]` vs `after[grace_a + i]` over the EF region and keep
//!    changed bytes whose ±16-byte neighborhoods are identical (shift illusions
//!    and record-list churn fail this test).
//! 5. Candidate resolution: a flip qualifies as the attributed transition if the
//!    expected bit goes 0→1 and the family base implied by the family layout is
//!    consistent with every other attributed flag of that family in the same
//!    file (earlier transitions SET, later transitions CLEAR, `known_set`
//!    anchors SET). Exactly one surviving candidate ⇒ Verified.
//! 6. Reward corroboration (ADR-0007): the inventory of each capture is parsed
//!    by ITEM IDENTITY (never GaItem handle — handles churn) and diffed across
//!    the pair window; gained/lost items are recorded as evidence on every
//!    claim, and a matching gain on a pickup/kill pair adds an independent
//!    `reward_corroboration` method.
//! 7. Tombstone checks: refutations of legacy conventions are recomputed from
//!    the bytes each run; a failing refutation aborts the run (a contradiction
//!    in the knowledge base must be investigated, not papered over).
//! 8. Deterministic emission of `knowledge/claims/event-flags.json` (sorted
//!    keys, no wall-clock timestamps): regenerating must be byte-identical.
//!
//! Flag families measured so far (bases are per-save, grace-relative — the
//! per-family float, ADR-0003 amendment):
//!   world-state-b   byte = (flag − 50000)/8, base ≈ 146.6k   (dungeon graces …)
//!   tile-open-world byte = tile_slot·875 + local/8, base ≈ 483.4k
//!   legacy-dungeon  byte = alloclist_slot·1125 + local/8, base ≈ 1,529.98k

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::catalog::sha256_file;
use crate::db::accessory_name::accessory_name::ACCESSORY_NAME;
use crate::db::aow_name::aow_name::AOW_NAME;
use crate::db::armor_name::armor_name::ARMOR_NAME;
use crate::db::item_name::item_name::ITEM_NAME;
use crate::db::weapon_name::weapon_name::WEAPON_NAME;
use crate::save::save::save::Save;

const HEADER: usize = 0x300;
const CHECKSUM: usize = 0x10;
const SLOT_SIZE: usize = 0x280000;
const EF_SIZE: usize = wasm_event_flags::EVENT_FLAGS_SIZE;
/// Neighborhood half-width for the isolated-flip test.
const ISOLATION_W: usize = 16;

const INPUT_TRANSITIONS: &str = "knowledge/inputs/attributed-transitions.json";
const INPUT_ALLOCLISTS: &str = "knowledge/game/eventflag-alloclists.json";
const CATALOG: &str = "knowledge/evidence-catalog.json";
const OUTPUT: &str = "knowledge/claims/event-flags.json";

// ---------------------------------------------------------------------------
// Family layouts (the layout formulas under test; verified by the run itself)
// ---------------------------------------------------------------------------

/// Byte offset of `flag` relative to its family's floating base, plus bit.
/// Bit convention (all families): bit = 7 - flag % 8.
fn family_rel(family: &str, flag: u64, alloc: &BTreeMap<String, u64>) -> Result<u64, String> {
    let bitcheck = |v: u64| -> Result<u64, String> { Ok(v) };
    match family {
        "world-state-b" => {
            if !(50_000..80_000).contains(&flag) {
                return Err(format!("flag {} outside world-state-b range", flag));
            }
            bitcheck((flag - 50_000) / 8)
        }
        "tile-open-world" => {
            let off = wasm_event_flags::calculate_tile_pickup_offset_with_base(flag as u32, 0);
            if !off.valid {
                return Err(format!("flag {} has no tile slot", flag));
            }
            bitcheck(off.byte_offset as u64)
        }
        "legacy-dungeon" => {
            let prefix = flag / 10_000; // AABB
            let map = format!("m{:02}_{:02}_00_00", prefix / 100, prefix % 100);
            let slot = alloc
                .get(&map)
                .ok_or_else(|| format!("map {} (flag {}) not in alloclist", map, flag))?;
            bitcheck(slot * 1125 + (flag % 10_000) / 8)
        }
        "tile-pickup-row-id" => {
            // World pickups tracked by ItemLotParam row_id (getItemFlagId =
            // row_id + 7000) in a pickup region separate from the tile
            // event-flag region (same tile layout, own floating base).
            // The input's `flag` field carries the ROW ID for this family.
            if flag % 10_000 >= 7_000 {
                return Err(format!("row_id {} has pickup-range local id", flag));
            }
            let off = wasm_event_flags::calculate_tile_pickup_offset_with_base(flag as u32, 0);
            if !off.valid {
                return Err(format!("row_id {} has no tile slot", flag));
            }
            bitcheck(off.byte_offset as u64)
        }
        "legacy-dungeon-pickup" => {
            // Dungeon pickup flags (local >= 7000): alloclist-slot layout but a
            // region distinct from the legacy event-flag region (own base).
            if flag % 10_000 < 7_000 {
                return Err(format!("flag {} is not a dungeon pickup (local < 7000)", flag));
            }
            let prefix = flag / 10_000;
            let map = format!("m{:02}_{:02}_00_00", prefix / 100, prefix % 100);
            let slot = alloc
                .get(&map)
                .ok_or_else(|| format!("map {} (flag {}) not in alloclist", map, flag))?;
            bitcheck(slot * 1125 + (flag % 10_000) / 8)
        }
        other => Err(format!("unknown family {}", other)),
    }
}

fn bit_of(flag: u64) -> u8 {
    (7 - flag % 8) as u8
}

// ---------------------------------------------------------------------------
// Evidence loading
// ---------------------------------------------------------------------------

struct SaveFile {
    rel_path: String,
    sha256: String,
    slot: Vec<u8>,
    grace: usize,
    ga_end: i64,
    confident: bool,
    /// item identity ("category:id") -> total quantity (held + storage box,
    /// common + key lists). Identity, never GaItem handle — handles churn
    /// across captures (ADR-0007).
    inventory: BTreeMap<String, i64>,
}

/// Parse the slot's inventory into identity -> quantity counts.
/// Weapon/armor/AoW handles resolve through the slot's ga_items table
/// (same derivation as the inventory view model); accessory and goods
/// handles carry the id in their low 28 bits.
fn inventory_identities(path: &Path, save_slot: usize) -> Result<BTreeMap<String, i64>, String> {
    let save = Save::from_path(&path.to_path_buf())
        .map_err(|e| format!("{}: typed save parse failed: {}", path.display(), e))?;
    let slot = save.save_type.get_slot(save_slot);
    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    for inv in [&slot.equip_inventory_data, &slot.storage_inventory_data] {
        for it in inv.common_items.iter().chain(inv.key_items.iter()) {
            let handle = it.ga_item_handle;
            if handle == 0 || it.quantity == 0 {
                continue;
            }
            let identity = match handle & 0xf000_0000 {
                0x8000_0000 | 0x9000_0000 | 0xc000_0000 => {
                    let ga = slot
                        .ga_items
                        .iter()
                        .find(|g| g.gaitem_handle == handle)
                        .ok_or_else(|| {
                            format!("{}: gaitem handle {:#010x} not in ga_items", path.display(), handle)
                        })?;
                    match handle & 0xf000_0000 {
                        0x8000_0000 => format!("weapon:{}", ga.item_id),
                        0x9000_0000 => format!("armor:{}", ga.item_id ^ 0x1000_0000),
                        _ => format!("aow:{}", ga.item_id ^ 0x8000_0000),
                    }
                }
                0xa000_0000 => format!("accessory:{}", handle ^ 0xa000_0000),
                0xb000_0000 => format!("goods:{}", handle ^ 0xb000_0000),
                other => format!("unknown-cat-{:x}:{}", other >> 28, handle & 0x0fff_ffff),
            };
            *counts.entry(identity).or_insert(0) += it.quantity as i64;
        }
    }
    Ok(counts)
}

/// Display name for an identity, from the in-repo name databases (labels only —
/// the claim rests on the id).
fn identity_name(identity: &str) -> Option<String> {
    let (cat, id) = identity.split_once(':')?;
    let id: u32 = id.parse().ok()?;
    let name = match cat {
        "weapon" => {
            let (base, lvl) = ((id / 100) * 100, id % 100);
            WEAPON_NAME.lock().unwrap().get(&base).map(|n| {
                if lvl > 0 {
                    format!("{} +{}", n, lvl)
                } else {
                    n.to_string()
                }
            })
        }
        "armor" => ARMOR_NAME.lock().unwrap().get(&id).map(|n| n.to_string()),
        "accessory" => ACCESSORY_NAME.lock().unwrap().get(&id).map(|n| n.to_string()),
        "goods" => ITEM_NAME.lock().unwrap().get(&id).map(|n| n.to_string()),
        "aow" => AOW_NAME.lock().unwrap().get(&id).map(|n| n.to_string()),
        _ => None,
    }?;
    if name.is_empty() || name.starts_with("+") {
        None
    } else {
        Some(name)
    }
}

/// (gained, lost) identity deltas between two inventories.
fn inventory_delta(
    before: &BTreeMap<String, i64>,
    after: &BTreeMap<String, i64>,
) -> (Vec<(String, i64)>, Vec<(String, i64)>) {
    let mut gained = Vec::new();
    let mut lost = Vec::new();
    for (id, qa) in after {
        let qb = before.get(id).copied().unwrap_or(0);
        if *qa > qb {
            gained.push((id.clone(), qa - qb));
        }
    }
    for (id, qb) in before {
        let qa = after.get(id).copied().unwrap_or(0);
        if qb > &qa {
            lost.push((id.clone(), qb - qa));
        }
    }
    (gained, lost)
}

fn delta_json(delta: &[(String, i64)]) -> Value {
    Value::Array(
        delta
            .iter()
            .map(|(id, qty)| {
                let mut m = Map::new();
                m.insert("identity".into(), json!(id));
                if let Some(n) = identity_name(id) {
                    m.insert("name".into(), json!(n));
                }
                m.insert("qty".into(), json!(qty));
                Value::Object(m)
            })
            .collect(),
    )
}

fn load_manifest(repo_root: &Path, manifest_rel: &str) -> Result<BTreeMap<String, String>, String> {
    let text = fs::read_to_string(repo_root.join(manifest_rel))
        .map_err(|e| format!("{}: {}", manifest_rel, e))?;
    let mut map = BTreeMap::new();
    for line in text.lines() {
        if let Some((hash, rel)) = line.split_once("  ") {
            map.insert(rel.to_string(), hash.to_string());
        }
    }
    Ok(map)
}

fn load_save(
    dir: &Path,
    rel_path: &str,
    save_slot: usize,
    manifest: &BTreeMap<String, String>,
) -> Result<SaveFile, String> {
    let path = dir.join(rel_path);
    let expected = manifest
        .get(rel_path)
        .ok_or_else(|| format!("{}: not in evidence manifest", rel_path))?;
    let (hash, _) = sha256_file(&path)?;
    if hash != *expected {
        return Err(format!(
            "EVIDENCE DRIFT {}: sha256 {} != cataloged {}",
            rel_path, hash, expected
        ));
    }
    let data = fs::read(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let start = HEADER + save_slot * (CHECKSUM + SLOT_SIZE) + CHECKSUM;
    if data.len() < start + SLOT_SIZE {
        return Err(format!("{}: too small for slot {}", rel_path, save_slot));
    }
    let slot = data[start..start + SLOT_SIZE].to_vec();
    let det = wasm_event_flags::detect_event_flags_offset_impl(&slot);
    if det.offset == 0 {
        return Err(format!("{}: grace-base detection failed", rel_path));
    }
    let ga_end = wasm_event_flags::parse_ga_items_end(&slot);
    let inventory = inventory_identities(&path, save_slot)?;
    Ok(SaveFile {
        rel_path: rel_path.to_string(),
        sha256: hash,
        slot,
        grace: det.offset,
        ga_end,
        confident: det.confident,
        inventory,
    })
}

// ---------------------------------------------------------------------------
// Flip extraction
// ---------------------------------------------------------------------------

/// Grace-aligned isolated byte flips: (grace_rel, before_byte, after_byte).
fn isolated_flips(before: &SaveFile, after: &SaveFile) -> Vec<(usize, u8, u8)> {
    let (sb, sa) = (&before.slot, &after.slot);
    let (gb, ga) = (before.grace, after.grace);
    let mut out = Vec::new();
    let max_i = EF_SIZE.min(SLOT_SIZE - ga - ISOLATION_W - 1);
    for i in 0..max_i {
        let (pb, pa) = (gb + i, ga + i);
        if sb[pb] == sa[pa] {
            continue;
        }
        if sb[pb - ISOLATION_W..pb] == sa[pa - ISOLATION_W..pa]
            && sb[pb + 1..pb + 1 + ISOLATION_W] == sa[pa + 1..pa + 1 + ISOLATION_W]
        {
            out.push((i, sb[pb], sa[pa]));
        }
    }
    out
}

fn bit_at(f: &SaveFile, grace_rel: u64, bit: u8) -> Option<bool> {
    let pos = f.grace as u64 + grace_rel;
    f.slot.get(pos as usize).map(|b| (b >> bit) & 1 == 1)
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

struct Pair {
    id: String,
    order: u64,
    corpus: String,
    save_slot: usize,
    before: String,
    after: String,
    flag: u64,
    family: String,
    kind: String,
    label: String,
    rel: u64,
    bit: u8,
}

/// Key for the loaded-files map: the same rel_path can appear under several
/// corpora/slots (multi-slot instrument files).
fn file_key(corpus: &str, save_slot: usize, rel_path: &str) -> String {
    format!("{}#{}#{}", corpus, save_slot, rel_path)
}

impl Pair {
    fn bkey(&self) -> String {
        file_key(&self.corpus, self.save_slot, &self.before)
    }
    fn akey(&self) -> String {
        file_key(&self.corpus, self.save_slot, &self.after)
    }
    /// Pairs may only cross-check each other within the same slot of the same
    /// corpus: family bases float per save, so expectations from another
    /// character's captures are meaningless.
    fn same_scope(&self, other: &Pair) -> bool {
        self.corpus == other.corpus && self.save_slot == other.save_slot
    }
}

struct Resolved {
    /// grace_rel position of the attributed flip byte in the after file
    flip_grace_rel: u64,
    byte_before: u8,
    byte_after: u8,
    /// family base (grace_rel) implied in the after file
    base_after: u64,
    /// cross-check descriptions that passed
    checks: Vec<String>,
    co_set_flags: Vec<u64>,
}

pub fn cmd_run(_args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;

    // --- inputs -----------------------------------------------------------
    let input_text = fs::read_to_string(repo_root.join(INPUT_TRANSITIONS))
        .map_err(|e| format!("{}: {}", INPUT_TRANSITIONS, e))?;
    let input: Value = serde_json::from_str(&input_text).map_err(|e| e.to_string())?;
    let alloc_text = fs::read_to_string(repo_root.join(INPUT_ALLOCLISTS))
        .map_err(|e| format!("{}: {}", INPUT_ALLOCLISTS, e))?;
    let alloc_json: Value = serde_json::from_str(&alloc_text).map_err(|e| e.to_string())?;
    let catalog_text = fs::read_to_string(repo_root.join(CATALOG))
        .map_err(|e| format!("{}: {}", CATALOG, e))?;
    let catalog: Value = serde_json::from_str(&catalog_text).map_err(|e| e.to_string())?;

    let mut alloc = BTreeMap::new();
    for list in ["legacymap", "legacymap_dlc02"] {
        for e in alloc_json["lists"][list]["entries"].as_array().unwrap_or(&vec![]) {
            alloc.insert(
                e["map"].as_str().unwrap_or_default().to_string(),
                e["slot"].as_u64().unwrap_or(0),
            );
        }
    }

    let corpus_id = input["corpus"].as_str().ok_or("input missing corpus")?;
    let save_slot = input["save_slot"].as_u64().unwrap_or(0) as usize;
    let established = input["established"].as_str().unwrap_or("").to_string();

    // resolve corpus directory + manifest from the evidence catalog (lazily,
    // per corpus id — pairs and differentials may reference several corpora)
    let mut corpora: BTreeMap<String, (PathBuf, BTreeMap<String, String>)> = BTreeMap::new();
    let mut corpus_for = |id: &str| -> Result<(PathBuf, BTreeMap<String, String>), String> {
        if let Some(v) = corpora.get(id) {
            return Ok(v.clone());
        }
        let corpus = catalog["corpora"]
            .as_array()
            .and_then(|cs| cs.iter().find(|c| c["id"] == id))
            .ok_or_else(|| format!("corpus {} not in evidence catalog", id))?;
        let root_key = corpus["root"].as_str().ok_or("corpus missing root")?;
        let root = catalog["roots"][root_key].as_str().ok_or("unknown root")?;
        let dir = Path::new(root).join(corpus["path"].as_str().ok_or("corpus missing path")?);
        let manifest_rel = corpus["manifest"].as_str().ok_or("corpus has no manifest")?;
        let manifest = load_manifest(&repo_root, manifest_rel)?;
        corpora.insert(id.to_string(), (dir.clone(), manifest.clone()));
        Ok((dir, manifest))
    };

    let mut pairs: Vec<Pair> = Vec::new();
    for p in input["pairs"].as_array().ok_or("input missing pairs")? {
        let flag = p["flag"].as_u64().ok_or("pair missing flag")?;
        let family = p["family"].as_str().ok_or("pair missing family")?.to_string();
        let rel = family_rel(&family, flag, &alloc)?;
        pairs.push(Pair {
            id: p["id"].as_str().unwrap_or_default().to_string(),
            order: p["order"].as_u64().ok_or("pair missing order")?,
            corpus: p["corpus"].as_str().unwrap_or(corpus_id).to_string(),
            save_slot: p["save_slot"].as_u64().map(|v| v as usize).unwrap_or(save_slot),
            before: p["before"].as_str().ok_or("pair missing before")?.to_string(),
            after: p["after"].as_str().ok_or("pair missing after")?.to_string(),
            flag,
            family,
            kind: p["kind"].as_str().unwrap_or_default().to_string(),
            label: p["label"].as_str().unwrap_or_default().to_string(),
            rel,
            bit: bit_of(flag),
        });
    }
    pairs.sort_by_key(|p| p.order);

    // known-set anchor flags per family (validated like everything else)
    let mut known_set: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    if let Some(ks) = input["known_set_before_all_pairs"].as_object() {
        for (fam, flags) in ks {
            let v: Vec<u64> = flags
                .as_array()
                .map(|a| a.iter().filter_map(|f| f.as_u64()).collect())
                .unwrap_or_default();
            known_set.insert(fam.clone(), v);
        }
    }

    // --- evidence (verify-on-read) ----------------------------------------
    println!("loading evidence (verify-on-read)…");
    let mut files: BTreeMap<String, SaveFile> = BTreeMap::new();
    let mut load_into = |files: &mut BTreeMap<String, SaveFile>,
                         corpus: &str,
                         slot: usize,
                         rel_path: &str|
     -> Result<(), String> {
        let key = file_key(corpus, slot, rel_path);
        if files.contains_key(&key) {
            return Ok(());
        }
        let (dir, manifest) = corpus_for(corpus)?;
        let f = load_save(&dir, rel_path, slot, &manifest)?;
        println!(
            "  [{} slot {}] {}… grace={} gaEnd={} confident={}",
            corpus,
            slot,
            &f.rel_path[..f.rel_path.len().min(52)],
            f.grace,
            f.ga_end,
            f.confident
        );
        files.insert(key, f);
        Ok(())
    };
    for p in &pairs {
        load_into(&mut files, &p.corpus, p.save_slot, &p.before)?;
        load_into(&mut files, &p.corpus, p.save_slot, &p.after)?;
    }
    let differentials: Vec<Value> = input["multi_slot_differentials"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for d in &differentials {
        let d_corpus = d["corpus"].as_str().unwrap_or(corpus_id);
        for se in d["slots"].as_array().unwrap_or(&vec![]) {
            let slot = se["save_slot"].as_u64().unwrap_or(0) as usize;
            for (rel_path, _) in se["files"].as_object().map(|o| o.iter()).into_iter().flatten() {
                load_into(&mut files, d_corpus, slot, rel_path)?;
            }
        }
    }

    // --- flip extraction + iterative candidate resolution ------------------
    // Cross-check expectations are built ONLY from already-resolved pairs, so
    // a wrong new hypothesis cannot poison verified claims. Iterate to a
    // fixpoint: each pass may resolve pairs that were ambiguous before.
    let mut flips_by_pair: BTreeMap<String, Vec<(usize, u8, u8)>> = BTreeMap::new();
    for p in &pairs {
        flips_by_pair.insert(p.id.clone(), isolated_flips(&files[&p.bkey()], &files[&p.akey()]));
    }

    let mut resolved: BTreeMap<String, Resolved> = BTreeMap::new();
    let mut diagnostics: BTreeMap<String, String> = BTreeMap::new();
    loop {
        let mut progressed = false;
        for p in &pairs {
            if resolved.contains_key(&p.id) {
                continue;
            }
            let after = &files[&p.akey()];
            let flips = &flips_by_pair[&p.id];

            // expectations from resolved same-family pairs (same corpus+slot
            // only — bases float per save) + known-set anchors (which are
            // attributed to the top-level corpus/slot context)
            let mut expectations: Vec<(u64, bool, String)> = Vec::new();
            for q in pairs.iter().filter(|q| {
                q.family == p.family
                    && q.id != p.id
                    && q.same_scope(p)
                    && resolved.contains_key(&q.id)
            }) {
                expectations.push((q.flag, q.order < p.order, format!("{} ({})", q.flag, q.id)));
            }
            if p.corpus == corpus_id && p.save_slot == save_slot {
                for f in known_set.get(&p.family).cloned().unwrap_or_default() {
                    expectations.push((f, true, format!("{} (known-set anchor)", f)));
                }
            }

            // candidates: expected bit flips 0 -> 1, family base non-negative
            let mask = 1u8 << p.bit;
            let mut survivors: Vec<Resolved> = Vec::new();
            for &(grace_rel, vb, va) in flips {
                if (va & !vb) & mask == 0 {
                    continue;
                }
                let grace_rel = grace_rel as u64;
                if grace_rel < p.rel {
                    continue;
                }
                let base = grace_rel - p.rel;
                let mut checks = Vec::new();
                let mut ok = true;
                for (qflag, expect_set, desc) in &expectations {
                    let qrel = family_rel(&p.family, *qflag, &alloc)?;
                    match bit_at(after, base + qrel, bit_of(*qflag)) {
                        Some(actual) if actual == *expect_set => {
                            checks.push(format!(
                                "{} {} at base+{}",
                                desc,
                                if *expect_set { "SET" } else { "CLEAR" },
                                qrel
                            ));
                        }
                        _ => {
                            ok = false;
                            break;
                        }
                    }
                }
                if !ok {
                    continue;
                }
                // co-set flags: other bits newly set in the same byte
                let mut co = Vec::new();
                for b in 0..8u8 {
                    if b != p.bit && (va & !vb) >> b & 1 == 1 {
                        co.push(p.flag - p.flag % 8 + (7 - b) as u64);
                    }
                }
                survivors.push(Resolved {
                    flip_grace_rel: grace_rel,
                    byte_before: vb,
                    byte_after: va,
                    base_after: base,
                    checks,
                    co_set_flags: co,
                });
            }

            // Multi-file differential disambiguation: these flags are set-monotonic
            // (pickup/kill/discovery state never clears), so a true candidate must
            // stay SET in every later capture — *provided* the family region did not
            // move, which we attest by requiring another resolved same-family pair
            // to have measured the exact base this candidate implies. Candidates
            // whose attested base fails persistence are rejected; if exactly one
            // survivor is base-corroborated and the rest have no attestation at all,
            // the corroborated one wins (two independent methods: attributed flip +
            // multi-file differential).
            if survivors.len() > 1 {
                let mut kept: Vec<(Resolved, bool)> = Vec::new();
                for mut s in survivors.drain(..) {
                    let corroborators: Vec<&Pair> = pairs
                        .iter()
                        .filter(|q| {
                            q.family == p.family
                                && q.id != p.id
                                && q.same_scope(p)
                                && resolved
                                    .get(&q.id)
                                    .is_some_and(|r| r.base_after == s.base_after)
                        })
                        .collect();
                    let mut alive = true;
                    for q in corroborators.iter().filter(|q| q.order > p.order) {
                        for fkey in [q.bkey(), q.akey()] {
                            match bit_at(&files[&fkey], s.flip_grace_rel, p.bit) {
                                Some(true) => s.checks.push(format!(
                                    "persists SET in {} file of {} (multi-file differential, base {})",
                                    if fkey == q.bkey() { "before" } else { "after" },
                                    q.id,
                                    s.base_after
                                )),
                                _ => {
                                    alive = false;
                                    break;
                                }
                            }
                        }
                        if !alive {
                            break;
                        }
                    }
                    if alive {
                        let corroborated = corroborators.iter().any(|q| q.order > p.order);
                        if corroborated {
                            s.checks.push(format!(
                                "family base {} independently measured by {}",
                                s.base_after,
                                corroborators
                                    .iter()
                                    .map(|q| q.id.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                        kept.push((s, corroborated));
                    }
                }
                if kept.len() == 1 {
                    survivors = kept.into_iter().map(|(s, _)| s).collect();
                } else if kept.iter().filter(|(_, c)| *c).count() == 1
                    && kept.iter().filter(|(_, c)| !*c).count() == kept.len() - 1
                {
                    survivors = kept
                        .into_iter()
                        .filter(|(_, c)| *c)
                        .map(|(s, _)| s)
                        .collect();
                } else {
                    survivors = kept.into_iter().map(|(s, _)| s).collect();
                }
            }

            match survivors.len() {
                1 => {
                    let r = survivors.pop().unwrap();
                    println!(
                        "{}: flag {} VERIFIED — flip {:02x}->{:02x} at grace_rel {} (family base {}), {} cross-checks",
                        p.id, p.flag, r.byte_before, r.byte_after, r.flip_grace_rel, r.base_after,
                        r.checks.len()
                    );
                    resolved.insert(p.id.clone(), r);
                    diagnostics.remove(&p.id);
                    progressed = true;
                }
                0 => {
                    diagnostics.insert(
                        p.id.clone(),
                        format!(
                            "no candidate: {} isolated flips, none matches bit {} 0->1 with consistent family base ({} expectations applied)",
                            flips.len(),
                            p.bit,
                            expectations.len()
                        ),
                    );
                }
                n => {
                    let positions: Vec<u64> =
                        survivors.iter().map(|s| s.flip_grace_rel).collect();
                    diagnostics.insert(
                        p.id.clone(),
                        format!(
                            "ambiguous: {} candidates at grace_rel {:?} ({} expectations applied)",
                            n, positions, expectations.len()
                        ),
                    );
                }
            }
        }
        if !progressed {
            break;
        }
    }
    for p in &pairs {
        if let Some(d) = diagnostics.get(&p.id) {
            println!("{}: flag {} UNRESOLVED — {}", p.id, p.flag, d);
        }
    }

    // --- tombstone refutation checks (recomputed every run) -----------------
    let tombstones = tombstone_checks(&pairs, &files, &resolved)?;

    // --- multi-slot differentials -------------------------------------------
    let msd_results = run_multi_slot_differentials(
        &differentials,
        corpus_id,
        &alloc,
        &pairs,
        &files,
        &resolved,
    )?;

    // --- claims assembly ----------------------------------------------------
    let store = build_store(
        &repo_root,
        corpus_id,
        save_slot,
        &established,
        &pairs,
        &files,
        &resolved,
        &diagnostics,
        tombstones,
        &msd_results,
    )?;

    let out_path = repo_root.join(OUTPUT);
    fs::create_dir_all(out_path.parent().unwrap()).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())? + "\n";
    let changed = fs::read_to_string(&out_path).map(|old| old != text).unwrap_or(true);
    fs::write(&out_path, &text).map_err(|e| e.to_string())?;
    println!(
        "claims store {} ({})",
        if changed { "written" } else { "unchanged" },
        OUTPUT
    );
    Ok(())
}

/// Refutations of retired conventions, recomputed from the bytes each run.
/// A refutation that stops holding is a contradiction: abort so it gets
/// investigated instead of silently emitting stale tombstones.
/// Result of one multi-slot differential instrument.
struct MsdResult {
    anchor_pair: String,
    /// method line to append to the anchor pair's flag claim (None = failed)
    method: Option<String>,
    entry: Value,
}

/// Multi-slot differential (CONTEXT.md): verify a flag across character slots
/// with attributed different progression, inside the same instrument files.
/// The anchor pair pins the family base in the anchor slot; each other slot's
/// base is located by matching its full expected bit pattern within ±64 bytes
/// of the anchor base (slots of one file float independently by record-list
/// insertions — measured Δ4 between V1 and V2/V3). Far-away pattern matches
/// exist but are static constants refuted by the anchor transition contrast
/// (full-EF scan, 2026-07-06); the bounded window plus a uniqueness
/// requirement keeps the check honest.
fn run_multi_slot_differentials(
    differentials: &[Value],
    default_corpus: &str,
    alloc: &BTreeMap<String, u64>,
    pairs: &[Pair],
    files: &BTreeMap<String, SaveFile>,
    resolved: &BTreeMap<String, Resolved>,
) -> Result<Vec<MsdResult>, String> {
    const WINDOW: i64 = 64;
    let mut out = Vec::new();
    for d in differentials {
        let id = d["id"].as_str().unwrap_or_default().to_string();
        let anchor_id = d["anchor_pair"].as_str().unwrap_or_default().to_string();
        let corpus = d["corpus"].as_str().unwrap_or(default_corpus);
        let family = d["family"].as_str().ok_or("differential missing family")?;
        let mut entry = json!({
            "id": id,
            "flag": d["flag"],
            "family": family,
            "label": d["label"],
            "established": d["established"],
            "anchor_pair": anchor_id,
        });
        let obj = entry.as_object_mut().unwrap();

        let Some(anchor) = pairs.iter().find(|p| p.id == anchor_id) else {
            return Err(format!("differential {}: anchor pair {} not in pairs", id, anchor_id));
        };
        let Some(r) = resolved.get(&anchor_id) else {
            println!("{}: SKIPPED — anchor pair {} unresolved", id, anchor_id);
            obj.insert("status".into(), json!("skipped"));
            obj.insert("diagnostic".into(), json!("anchor pair unresolved"));
            out.push(MsdResult { anchor_pair: anchor_id, method: None, entry });
            continue;
        };
        let anchor_base = r.base_after as i64;
        obj.insert("anchor_base_grace_rel".into(), json!(anchor_base));

        let mut slot_reports = Vec::new();
        let mut all_ok = true;
        for se in d["slots"].as_array().unwrap_or(&vec![]) {
            let slot = se["save_slot"].as_u64().unwrap_or(0) as usize;
            let character = se["character"].as_str().unwrap_or_default();
            let file_patterns = se["files"].as_object().ok_or("slot entry missing files")?;

            // deltas (vs the anchor base) where the slot's full pattern matches
            let mut matches: Vec<i64> = Vec::new();
            for delta in -WINDOW..=WINDOW {
                let mut ok = true;
                'outer: for (rel_path, pattern) in file_patterns {
                    let f = &files[&file_key(corpus, slot, rel_path)];
                    for (flag_s, expect) in pattern.as_object().unwrap() {
                        let flag: u64 = flag_s.parse().map_err(|_| format!("bad flag {}", flag_s))?;
                        let rel = family_rel(family, flag, alloc)? as i64;
                        let pos = anchor_base + delta + rel;
                        let actual = pos >= 0 && bit_at(f, pos as u64, bit_of(flag)) == Some(true);
                        if actual != expect.as_bool().unwrap_or(false) {
                            ok = false;
                            break 'outer;
                        }
                    }
                }
                if ok {
                    matches.push(delta);
                }
            }
            let checks: usize = file_patterns
                .values()
                .map(|p| p.as_object().map(|o| o.len()).unwrap_or(0))
                .sum();
            match matches.as_slice() {
                [delta] => {
                    println!(
                        "{}: slot {} ({}) matches at base {} (anchor{:+}), {} bit-checks across {} files",
                        id, slot, character, anchor_base + delta, delta, checks, file_patterns.len()
                    );
                    slot_reports.push(json!({
                        "save_slot": slot,
                        "character": character,
                        "base_grace_rel": anchor_base + delta,
                        "delta_vs_anchor": delta,
                        "bit_checks": checks,
                        "files": file_patterns.len(),
                        "pattern_provenance": se["pattern_provenance"],
                    }));
                }
                [] => {
                    all_ok = false;
                    slot_reports.push(json!({
                        "save_slot": slot,
                        "character": character,
                        "diagnostic": "no base within ±64 of the anchor matches the expected pattern",
                    }));
                }
                many => {
                    all_ok = false;
                    slot_reports.push(json!({
                        "save_slot": slot,
                        "character": character,
                        "diagnostic": format!("ambiguous: {} bases match within the window: {:?}", many.len(), many),
                    }));
                }
            }
        }
        obj.insert("slots".into(), Value::Array(slot_reports));
        let method = if all_ok {
            obj.insert("status".into(), json!("verified"));
            println!("{}: VERIFIED across {} slots", id, d["slots"].as_array().map(|a| a.len()).unwrap_or(0));
            Some(format!(
                "multi_slot_differential: {} — expected presence/absence pattern matches in every slot at per-slot bases within ±{} of the anchor base",
                id, WINDOW
            ))
        } else {
            obj.insert("status".into(), json!("failed"));
            println!("{}: FAILED — see slot diagnostics", id);
            None
        };
        // the anchor pair must belong to the differential's corpus
        if anchor.corpus != corpus {
            return Err(format!("differential {}: anchor pair corpus mismatch", id));
        }
        out.push(MsdResult { anchor_pair: anchor_id, method, entry });
    }
    Ok(out)
}

fn tombstone_checks(
    pairs: &[Pair],
    files: &BTreeMap<String, SaveFile>,
    resolved: &BTreeMap<String, Resolved>,
) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();

    // (1) "tile_base = 337375 grace-anchored" — the old constant was anchored
    // to the poisoned structural walk (~146k above the grace base).
    let mut tile_bases = BTreeMap::new();
    let mut tile_delta = BTreeMap::new();
    for p in pairs.iter().filter(|p| p.family == "tile-open-world") {
        let Some(r) = resolved.get(&p.id) else { continue };
        if r.base_after.abs_diff(337_375) < 100_000 {
            return Err(format!(
                "tombstone check failed: measured tile base {} is near 337375",
                r.base_after
            ));
        }
        tile_bases.insert(p.after.clone(), r.base_after);
        tile_delta.insert(p.after.clone(), r.base_after as i64 - 337_375);
    }
    out.push(json!({
        "id": "tile-base-337375-grace-anchored",
        "statement": "The tile-family base sits at grace_rel 337,375 (constant across saves)",
        "verdict": "refuted",
        "refutation": "Attributed tile-boss kill flips measure the tile base at grace_rel ~483.4k. The old constant was expressed relative to the poisoned structural anchor (~146.1k above the grace base): measured_base - 337,375 reproduces the struct-walk delta.",
        "measured_bases_grace_rel": Value::Object(tile_bases.iter().map(|(k, v)| (k.clone(), json!(v))).collect::<Map<_,_>>()),
        "delta_vs_old_constant": Value::Object(tile_delta.iter().map(|(k, v)| (k.clone(), json!(v))).collect::<Map<_,_>>()),
    }));

    // (2) "legacy-dungeon region base = grace_rel 4112 (m14=29,987 …)" —
    // verify the kill bit is NOT at the old position, and the measured base is far away.
    let mut legacy_bases = BTreeMap::new();
    for p in pairs.iter().filter(|p| p.family == "legacy-dungeon") {
        let Some(r) = resolved.get(&p.id) else { continue };
        if r.base_after.abs_diff(4112) < 100_000 {
            return Err(format!(
                "tombstone check failed: measured legacy base {} is near 4112",
                r.base_after
            ));
        }
        let after = &files[&p.akey()];
        if bit_at(after, 4112 + p.rel, p.bit) == Some(true) {
            return Err(format!(
                "tombstone check failed: {} bit also set at old 4112-based position",
                p.flag
            ));
        }
        legacy_bases.insert(p.after.clone(), r.base_after);
    }
    out.push(json!({
        "id": "legacy-region-at-grace-rel-4112",
        "statement": "Legacy-map flag blocks start at grace_rel 4,112 (m14 base 29,987, m18 43,487, m19 46,862)",
        "verdict": "refuted",
        "refutation": "Attributed catacombs kills flip bits at legacy base ~1,529.98k grace_rel; the kill bit is clear at the 4,112-based position. The grace_rel 28-31k span the old numbers pointed into is a u32-record list (entries shift the region by ±4 per insertion), not the legacy flag bitmap. The alloclist LAYOUT (slot×1125) is correct; only the region position was wrong.",
        "measured_bases_grace_rel": Value::Object(legacy_bases.iter().map(|(k, v)| (k.clone(), json!(v))).collect::<Map<_,_>>()),
    }));

    // (3) "one universal EF anchor positions all families" — family bases move
    // by different amounts than the grace base between captures. Grouped per
    // (family, corpus, slot): comparing bases across characters is meaningless.
    let mut abs_bases: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();
    for p in pairs {
        let Some(r) = resolved.get(&p.id) else { continue };
        let f = &files[&p.akey()];
        abs_bases
            .entry(format!("{} [{} slot {}]", p.family, p.corpus, p.save_slot))
            .or_default()
            .insert(p.akey(), f.grace as i64 + r.base_after as i64);
    }
    // The refutation needs at least ONE family that provably moves differently
    // from the grace base; families may coincidentally not drift between two
    // particular captures (drift is occasional, not constant).
    let mut float_evidence = Map::new();
    let mut independent_float_seen = false;
    for (family, bases) in &abs_bases {
        if bases.len() < 2 {
            continue;
        }
        let keys: Vec<&String> = bases.keys().collect();
        let (a, b) = (keys[0], keys[1]);
        let family_delta = bases[b] - bases[a];
        let grace_delta = files[b].grace as i64 - files[a].grace as i64;
        if family_delta != grace_delta {
            independent_float_seen = true;
        }
        float_evidence.insert(
            family.clone(),
            json!({
                "files": [a, b],
                "family_abs_delta": family_delta,
                "grace_abs_delta": grace_delta,
                "independent_float": family_delta != grace_delta,
            }),
        );
    }
    if !independent_float_seen {
        return Err(
            "tombstone check failed: no family shows float independent of the grace base — \
             universal-anchor refutation no longer holds on this evidence"
                .into(),
        );
    }
    out.push(json!({
        "id": "universal-ef-anchor",
        "statement": "A single per-save EF anchor positions every flag family",
        "verdict": "refuted",
        "refutation": "Between captures of the same character, family bases move by different amounts than the grace base (per-family float, ADR-0003 amendment).",
        "float_evidence": Value::Object(float_evidence),
    }));

    if tile_bases.is_empty() || legacy_bases.is_empty() {
        return Err("tombstone checks lost their measurements: core tile/legacy pairs no longer resolve".into());
    }

    // (4) "dungeon graces live at (flag-50000)/8 from the grace base (copy A)".
    // Restricted to RESOLVED grace discoveries; dungeon graces (< 76000) must stay
    // CLEAR in copy A, while open-world graces (76xxx) must be SET there — copy A is
    // the open-world grace region the detection anchors (76100/76101) live in, and
    // the c03-c04 pair (76310) showed open-world graces occupy BOTH regions.
    let mut copy_a_contrast = serde_json::Map::new();
    for p in pairs.iter().filter(|p| {
        p.family == "world-state-b" && p.kind == "grace_discovery" && resolved.contains_key(&p.id)
    }) {
        let after = &files[&p.akey()];
        let set = bit_at(after, p.rel, p.bit) == Some(true);
        let open_world = p.flag >= 76_000;
        if set != open_world {
            return Err(format!(
                "tombstone check failed: {} {} at copy-A position {} after discovery ({} grace)",
                p.flag,
                if set { "set" } else { "clear" },
                p.rel,
                if open_world { "open-world" } else { "dungeon" }
            ));
        }
        copy_a_contrast.insert(
            p.flag.to_string(),
            json!(if open_world { "set (open-world grace)" } else { "clear (dungeon grace)" }),
        );
    }
    out.push(json!({
        "id": "dungeon-graces-in-grace-anchor-block",
        "statement": "Dungeon graces (71xxx/72xxx/73xxx) live at (flag-50000)/8 relative to the detected grace base",
        "verdict": "refuted",
        "refutation": "After attributed dungeon-grace discoveries the bit at the grace-base position stays clear; the flips land in a second world-state block ~146.6k above. Open-world graces (76xxx) DO set the copy-A bit AND the copy-B bit (c03-c04, flag 76310) — copy A is the open-world grace region the detection anchors live in. The (flag-50000)/8 packing itself is correct in both blocks.",
        "copy_a_after_discovery": Value::Object(copy_a_contrast),
    }));

    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn build_store(
    repo_root: &Path,
    _corpus_id: &str,
    _save_slot: usize,
    established: &str,
    pairs: &[Pair],
    files: &BTreeMap<String, SaveFile>,
    resolved: &BTreeMap<String, Resolved>,
    diagnostics: &BTreeMap<String, String>,
    tombstones: Vec<Value>,
    msd: &[MsdResult],
) -> Result<Value, String> {
    let input_hash = |rel: &str| -> Result<String, String> {
        sha256_file(&PathBuf::from(repo_root).join(rel)).map(|(h, _)| h)
    };

    // flag claims (verified when resolved; hypothesis with diagnostics otherwise)
    let mut flags = Vec::new();
    for p in pairs {
        let before = &files[&p.bkey()];
        let after = &files[&p.akey()];
        let mut claim = json!({
            "flag": p.flag,
            "label": p.label,
            "kind": p.kind,
            "family": p.family,
            "family_rel_byte": p.rel,
            "bit": p.bit,
            "established": established,
            "evidence": {
                "corpus": p.corpus,
                "save_slot": p.save_slot,
                "pair": p.id,
                "before": { "file": before.rel_path, "sha256": before.sha256 },
                "after": { "file": after.rel_path, "sha256": after.sha256 },
            },
        });
        let obj = claim.as_object_mut().unwrap();

        // inventory delta by item identity across the pair window (ADR-0007) —
        // evidence for every pair; a method entry when it corroborates the kind
        let (gained, lost) = inventory_delta(&before.inventory, &after.inventory);
        obj["evidence"]
            .as_object_mut()
            .unwrap()
            .insert("inventory_gained".into(), delta_json(&gained));
        obj["evidence"]
            .as_object_mut()
            .unwrap()
            .insert("inventory_lost".into(), delta_json(&lost));
        let reward_method = if !gained.is_empty()
            && matches!(p.kind.as_str(), "boss_kill" | "world_pickup" | "dungeon_pickup")
        {
            let list = gained
                .iter()
                .map(|(id, qty)| {
                    let n = identity_name(id).map(|n| format!(" ({})", n)).unwrap_or_default();
                    format!("{}{} x{}", id, n, qty)
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!(
                "reward_corroboration: gained {} in the {} window",
                list, p.kind
            ))
        } else {
            None
        };

        if let Some(r) = resolved.get(&p.id) {
            let mut methods = vec![format!("attributed_transition ({})", p.kind)];
            methods.extend(reward_method.clone());
            methods.extend(
                msd.iter()
                    .filter(|m| m.anchor_pair == p.id)
                    .filter_map(|m| m.method.clone()),
            );
            methods.extend(r.checks.iter().map(|c| {
                if c.contains("multi-file differential") || c.contains("independently measured") {
                    format!("multi_file_differential: {}", c)
                } else {
                    format!("within_file_cross_check: {}", c)
                }
            }));
            obj.insert("status".into(), json!("verified"));
            obj.insert("methods".into(), json!(methods));
            obj.insert(
                "measured".into(),
                json!({
                    "flip_grace_rel": r.flip_grace_rel,
                    "byte_before": format!("{:#04x}", r.byte_before),
                    "byte_after": format!("{:#04x}", r.byte_after),
                    "family_base_grace_rel_after": r.base_after,
                }),
            );
            obj.insert("co_set_flags_hypotheses".into(), json!(r.co_set_flags));
        } else {
            obj.insert("status".into(), json!("hypothesis"));
            obj.insert(
                "unresolved".into(),
                json!(diagnostics.get(&p.id).cloned().unwrap_or_default()),
            );
        }
        flags.push(claim);
    }
    flags.sort_by_key(|f| f["flag"].as_u64());

    // family claims (bases from resolved pairs only)
    let mut families = Vec::new();
    let layouts: BTreeMap<&str, (&str, &str)> = BTreeMap::from([
        (
            "tile-pickup-row-id",
            (
                "byte = tile_slot * 875 + (row_id % 10000) / 8; bit = 7 - row_id % 8; row_id = getItemFlagId - 7000",
                "world pickups tracked by ItemLotParam row id; SEPARATE region from tile-open-world (base ~500 bytes above it); claims carry the ROW ID in the flag field",
            ),
        ),
        (
            "legacy-dungeon-pickup",
            (
                "byte = alloclist_slot(map) * 1125 + (flag % 10000) / 8; local >= 7000; bit = 7 - flag % 8",
                "dungeon pickup flags; SEPARATE region from legacy-dungeon event flags (base ~129 bytes below it), same alloclist-slot layout",
            ),
        ),
        (
            "world-state-b",
            (
                "byte = (flag - 50000) / 8; bit = 7 - flag % 8",
                "second world-state block; also mirrors the tutorial grace anchors that define the grace-base (copy A) convention",
            ),
        ),
        (
            "tile-open-world",
            (
                "byte = tile_slot * 875 + (flag % 10000) / 8; tile_slot = (row-33)*40 + (col-30) from flag 1RRCCLLLL; bit = 7 - flag % 8",
                "open-world tile flags (m60), including overworld boss kills",
            ),
        ),
        (
            "legacy-dungeon",
            (
                "byte = alloclist_slot(map) * 1125 + (flag % 10000) / 8; map = m{AABB} from flag/10000; bit = 7 - flag % 8",
                "legacy dungeon / catacombs flags; slots from the game's eventflagalloclist (primary evidence)",
            ),
        ),
    ]);
    for (family, (layout, note)) in &layouts {
        let fam_pairs: Vec<&Pair> = pairs
            .iter()
            .filter(|p| &p.family == family && resolved.contains_key(&p.id))
            .collect();
        if fam_pairs.is_empty() {
            continue;
        }
        let mut bases = Map::new();
        for p in &fam_pairs {
            bases.insert(p.after.clone(), json!(resolved[&p.id].base_after));
        }
        // The first pair of a family necessarily resolves without expectations
        // (iterative resolution); the layout is verified once ANY pair
        // cross-checks other flags at the same family base within one file.
        let cross_checked = fam_pairs.iter().any(|p| !resolved[&p.id].checks.is_empty());
        families.push(json!({
            "family": family,
            "layout": layout,
            "note": note,
            "status": if cross_checked && fam_pairs.len() >= 2 { "verified" } else { "corroborated" },
            "methods": ["attributed_transition", "within_file_cross_check"],
            "established": established,
            "base_is_per_save": true,
            "base_measurements_grace_rel": Value::Object(bases),
        }));
    }

    // per-file measurements (keyed corpus#slot#rel_path — instrument files
    // are read once per slot)
    let mut measurements = Map::new();
    for (key, f) in files {
        measurements.insert(
            key.clone(),
            json!({
                "sha256": f.sha256,
                "grace_base": f.grace,
                "ga_items_end": f.ga_end,
                "detection_confident": f.confident,
            }),
        );
    }

    Ok(json!({
        "schema": "claims-store/1",
        "generated_by": "er-save-editor knowledge run — DO NOT EDIT (ADR-0004: regenerate from evidence)",
        "convention": "grace_rel = bytes relative to the detected grace-family base (copy A, pinned by crates/wasm-event-flags/tests/fixtures). family bases float per save (ADR-0003 amendment): resolve a flag as slot[grace_base + family_base(save) + family_rel_byte], bit 7 - flag % 8.",
        "status_ladder": "hypothesis -> corroborated (one method) -> verified (attributed transition, or two independent methods); tombstones are refuted claims kept so the idea cannot return. Applications consume corroborated+verified only.",
        "inputs": {
            "attributed_transitions": { "path": INPUT_TRANSITIONS, "sha256": input_hash(INPUT_TRANSITIONS)? },
            "eventflag_alloclists": { "path": INPUT_ALLOCLISTS, "sha256": input_hash(INPUT_ALLOCLISTS)? },
            "evidence_catalog": { "path": CATALOG, "sha256": input_hash(CATALOG)? },
        },
        "families": families,
        "flags": flags,
        "multi_slot_differentials": msd.iter().map(|m| m.entry.clone()).collect::<Vec<_>>(),
        "tombstones": tombstones,
        "per_file_measurements": Value::Object(measurements),
    }))
}
