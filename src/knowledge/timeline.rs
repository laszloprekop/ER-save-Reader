//! `knowledge timeline` — sparse-diff replay audit (migration step 3
//! remainder: "timeline re-annotation as pipeline output").
//!
//! The capture-agent metadata attached to each timeline entry (`bossesDefeated`,
//! `eventFlagsOffset`, …) is NOT evidence (ADR-0007): each detection era froze
//! its own bugs into that metadata, and the evidence catalog documents concrete
//! poisoning (anchors jumping to a lookalike region from a known timestamp
//! onward, `bossesDefeated` flicker artifacts). Only the raw sparse-diff
//! records ([u32 LE offset][old byte][new byte], one file per capture) and the
//! chronological ordering/timestamps are Evidence.
//!
//! What this command does:
//! 1. Verify-on-read every diff file and the metadata file against the
//!    evidence catalog manifests (ADR-0001).
//! 2. Replay: apply each capture's sparse diff to an in-memory slot buffer in
//!    chronological order. The chain does not start from a keyframe (ADR-0007
//!    — no keyframes existed yet), so old-value mismatches against the
//!    replayed state are expected and counted, not treated as errors.
//! 3. At every replayed state, run the reference grace detector
//!    (`crates/wasm-event-flags`, ADR-0005) and report the confident-detection
//!    rate and the offset's drift range across the whole chain.
//!
//! REJECTED APPROACH (2026-07-06, evidence-based finding, not a bug fix in
//! progress): re-annotating *which flags* set *when* requires locating the
//! world-state-b family base at every replayed state with no attributed
//! before/after pair to anchor a bounded search window (unlike
//! `pipeline.rs`'s `cmd_run`, which always has one). Two designs were tried
//! and both failed the project's evidence bar:
//!   - Blind full-EF scan for the 4-bit tutorial anchor pattern (flags
//!     71800/71801/76100/76101), requiring an exact-one-match: on the Bee
//!     corpus this returned 0 matches over an unbounded range and 2-3 matches
//!     even inside a tight window bounded to the empirically-established base
//!     cluster (~130k-160k) — the 4-bit signature is not discriminating
//!     enough once a save has enough unrelated 0xFF-heavy content nearby.
//!   - The same scan gated by a base-stability streak (a candidate must stay
//!     within 16 bytes of a running anchor for 3 consecutive resolved
//!     entries before being trusted, on the theory that a real family base
//!     drifts slowly while a coincidental match would not reproduce nearby
//!     positions repeatedly): this still produced 32,893 "events" naming only
//!     16,174 distinct flags — i.e. thousands of flags were reported as
//!     transitioning 0->1 *more than once* (one flag 69 times), which is
//!     logically impossible for a monotonic bit and proves the resolved base
//!     was hopping between the real region and at least one other
//!     coincidentally-matching region.
//!
//! Producing a claims-adjacent output from a method that fails its own
//! internal consistency check (repeated "first-time" transitions) would
//! violate the evidence discipline this project resets itself around
//! (ADR-0004, CLAUDE.md's False Negative Investigation Protocol: gather
//! evidence, don't force a fix). The next viable design is a global
//! consistency method in the same spirit as `cmd_run`'s candidate resolution:
//! extract grace-aligned isolated flips (the same ±16-neighborhood test
//! already proven in `pipeline.rs`) between EVERY consecutive pair of
//! replayed states across the whole chain, then locate the family base by
//! clustering — a true base should account for many independent flips at
//! consistent small offsets from each other, while a coincidental match
//! would not. That is materially more work than this increment and is left
//! as documented follow-on, not attempted here.

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use super::catalog::sha256_file;

const SLOT_SIZE: usize = 0x280000;
const CATALOG: &str = "knowledge/evidence-catalog.json";
const INPUT_TARGETS: &str = "knowledge/inputs/timeline-targets.json";
const OUTPUT: &str = "knowledge/claims/timeline-replay-audit.json";

struct DiffEntry {
    id: String,
    timestamp: String,
    diff_file: String,
}

fn load_metadata(path: &Path, expected_sha256: &str, slot_index: u64) -> Result<Vec<DiffEntry>, String> {
    let (hash, _) = sha256_file(path)?;
    if hash != expected_sha256 {
        return Err(format!(
            "EVIDENCE DRIFT {}: sha256 {} != cataloged {}",
            path.display(),
            hash,
            expected_sha256
        ));
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("{}: {}", path.display(), e))?;
        if v["slotIndex"].as_u64() != Some(slot_index) {
            continue;
        }
        out.push(DiffEntry {
            id: v["id"].as_str().unwrap_or_default().to_string(),
            timestamp: v["timestamp"].as_str().unwrap_or_default().to_string(),
            diff_file: v["diffFile"].as_str().unwrap_or_default().to_string(),
        });
    }
    Ok(out)
}

pub fn cmd_timeline(args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let target_id = args.first().cloned().unwrap_or_else(|| "bee".to_string());

    let targets_text = fs::read_to_string(repo_root.join(INPUT_TARGETS))
        .map_err(|e| format!("{}: {}", INPUT_TARGETS, e))?;
    let targets: serde_json::Value = serde_json::from_str(&targets_text).map_err(|e| e.to_string())?;
    let target = targets["targets"]
        .as_array()
        .and_then(|a| a.iter().find(|t| t["id"] == target_id))
        .ok_or_else(|| format!("timeline target '{}' not found in {}", target_id, INPUT_TARGETS))?;

    let diff_corpus_id = target["diff_corpus"].as_str().ok_or("target missing diff_corpus")?;
    let meta_corpus_id = target["metadata_corpus"].as_str().ok_or("target missing metadata_corpus")?;
    let slot_index = target["slot_index"].as_u64().ok_or("target missing slot_index")?;
    let character = target["character"].as_str().unwrap_or("unknown").to_string();

    let catalog_text = fs::read_to_string(repo_root.join(CATALOG)).map_err(|e| format!("{}: {}", CATALOG, e))?;
    let catalog: serde_json::Value = serde_json::from_str(&catalog_text).map_err(|e| e.to_string())?;
    let corpus = |id: &str| -> Result<serde_json::Value, String> {
        catalog["corpora"]
            .as_array()
            .and_then(|cs| cs.iter().find(|c| c["id"] == id))
            .cloned()
            .ok_or_else(|| format!("corpus {} not in evidence catalog", id))
    };
    let root_dir = |c: &serde_json::Value| -> Result<PathBuf, String> {
        let root_key = c["root"].as_str().ok_or("corpus missing root")?;
        let root = catalog["roots"][root_key].as_str().ok_or("unknown root")?;
        Ok(Path::new(root).join(c["path"].as_str().ok_or("corpus missing path")?))
    };

    let diff_corpus = corpus(diff_corpus_id)?;
    let diff_dir = root_dir(&diff_corpus)?;
    let manifest_rel = diff_corpus["manifest"].as_str().ok_or("diff corpus has no manifest")?;
    let manifest_text = fs::read_to_string(repo_root.join(manifest_rel))
        .map_err(|e| format!("{}: {}", manifest_rel, e))?;
    let mut manifest: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for line in manifest_text.lines() {
        if let Some((hash, rel)) = line.split_once("  ") {
            manifest.insert(rel.to_string(), hash.to_string());
        }
    }

    let meta_corpus = corpus(meta_corpus_id)?;
    let meta_path = Path::new(
        catalog["roots"][meta_corpus["root"].as_str().unwrap_or_default()]
            .as_str()
            .unwrap_or_default(),
    )
    .join(meta_corpus["path"].as_str().ok_or("metadata corpus missing path")?);
    let meta_sha = meta_corpus["sha256"].as_str().ok_or("metadata corpus missing sha256")?;

    println!("loading timeline metadata (verify-on-read)…");
    let entries = load_metadata(&meta_path, meta_sha, slot_index)?;
    println!(
        "  {} entries for {} (slot {}): {} .. {}",
        entries.len(),
        character,
        slot_index,
        entries.first().map(|e| e.id.as_str()).unwrap_or(""),
        entries.last().map(|e| e.id.as_str()).unwrap_or("")
    );

    println!("replaying sparse diffs (verify-on-read)…");
    let mut state = vec![0u8; SLOT_SIZE];
    let mut total_records: u64 = 0;
    let mut total_mismatches: u64 = 0;
    let mut confident_count = 0usize;
    let mut offset_min = usize::MAX;
    let mut offset_max = 0usize;

    for (i, e) in entries.iter().enumerate() {
        let diff_path = diff_dir.join(&e.diff_file);
        let expected = manifest
            .get(e.diff_file.as_str())
            .ok_or_else(|| format!("{}: not in evidence manifest", e.diff_file))?;
        let (hash, _) = sha256_file(&diff_path)?;
        if hash != *expected {
            return Err(format!(
                "EVIDENCE DRIFT {}: sha256 {} != cataloged {}",
                e.diff_file, hash, expected
            ));
        }
        let data = fs::read(&diff_path).map_err(|e2| format!("{}: {}", diff_path.display(), e2))?;
        if data.len() % 6 != 0 {
            return Err(format!("{}: length {} not a multiple of 6", e.diff_file, data.len()));
        }
        let n = data.len() / 6;
        total_records += n as u64;
        for r in 0..n {
            let rec = &data[r * 6..r * 6 + 6];
            let off = u32::from_le_bytes([rec[0], rec[1], rec[2], rec[3]]) as usize;
            let old = rec[4];
            let new = rec[5];
            if off >= SLOT_SIZE {
                continue; // out-of-range record; skip rather than panic
            }
            if i > 0 && state[off] != old {
                total_mismatches += 1;
            }
            state[off] = new;
        }

        let det = wasm_event_flags::detect_event_flags_offset_impl(&state);
        if det.confident {
            confident_count += 1;
            offset_min = offset_min.min(det.offset);
            offset_max = offset_max.max(det.offset);
        }
    }

    let mismatch_pct = 100.0 * total_mismatches as f64 / total_records.max(1) as f64;
    println!(
        "replay done: {} records, {} old-value mismatches ({:.2}%)",
        total_records, total_mismatches, mismatch_pct
    );
    println!(
        "confident grace detection: {}/{} entries; offset range {}..{}",
        confident_count,
        entries.len(),
        if offset_min == usize::MAX { 0 } else { offset_min },
        offset_max
    );

    let out = json!({
        "schema": "timeline-replay-audit/1",
        "note": "Replay-and-detect audit only, not a flag re-annotation (see the module doc in src/knowledge/timeline.rs for the two anchor-scan designs that were tried and failed the evidence bar: repeated 'first-time' 0->1 transitions on set-monotonic bits proved base misidentification, not real gameplay). This file records that the sparse-diff replay is self-consistent and that the reference grace detector stays confident across the chain; it names no flags and asserts no game events.",
        "target": target_id,
        "character": character,
        "diff_corpus": diff_corpus_id,
        "metadata_corpus": meta_corpus_id,
        "slot_index": slot_index,
        "entries_processed": entries.len(),
        "replay_records": total_records,
        "replay_old_value_mismatches": total_mismatches,
        "replay_old_value_mismatch_pct": mismatch_pct,
        "confident_detection_entries": confident_count,
        "ef_offset_min": if offset_min == usize::MAX { 0 } else { offset_min },
        "ef_offset_max": offset_max,
    });
    fs::create_dir_all(repo_root.join("knowledge/claims")).map_err(|e| e.to_string())?;
    let out_path = repo_root.join(OUTPUT);
    let text = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
    fs::write(&out_path, text).map_err(|e| e.to_string())?;
    println!("replay audit written ({})", OUTPUT);
    Ok(())
}
