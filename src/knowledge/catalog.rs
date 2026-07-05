//! Evidence catalog: integrity index over out-of-repo Evidence (ADR-0001).
//!
//! The catalog JSON is one of the two hand-written knowledge inputs
//! (ADR-0004): humans own `description`/`context`/`status`; this module owns
//! the machine fields (`sha256`, `size`, `files`, `total_size`, `manifest`,
//! `updated`). Directory corpora get a per-file sha256 manifest under
//! `knowledge/manifests/` so single-file corruption is detectable.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const CATALOG_PATH: &str = "knowledge/evidence-catalog.json";
const MANIFEST_DIR: &str = "knowledge/manifests";

fn sha256_file(path: &Path) -> Result<(String, u64), String> {
    let mut f = fs::File::open(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut size = 0u64;
    loop {
        let n = f.read(&mut buf).map_err(|e| format!("{}: {}", path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    Ok((format!("{:x}", hasher.finalize()), size))
}

/// Recursively list files under `dir`, sorted, as paths relative to `dir`.
fn walk_sorted(dir: &Path, excludes: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = fs::read_dir(&d).map_err(|e| format!("{}: {}", d.display(), e))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let p = entry.path();
            let rel = p.strip_prefix(dir).unwrap().to_string_lossy().to_string();
            if excludes.iter().any(|ex| rel == *ex || rel.starts_with(&format!("{}/", ex)))
                || rel.ends_with(".DS_Store")
            {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p.strip_prefix(dir).unwrap().to_path_buf());
            }
        }
    }
    out.sort();
    Ok(out)
}

fn load_catalog(repo_root: &Path) -> Result<Value, String> {
    let path = repo_root.join(CATALOG_PATH);
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
    serde_json::from_str(&text).map_err(|e| format!("catalog parse: {}", e))
}

fn corpus_dir(roots: &Value, corpus: &Value) -> Result<PathBuf, String> {
    let root_key = corpus["root"].as_str().ok_or("corpus missing root")?;
    let root = roots[root_key].as_str().ok_or_else(|| format!("unknown root {}", root_key))?;
    let rel = corpus["path"].as_str().ok_or("corpus missing path")?;
    Ok(Path::new(root).join(rel))
}

fn excludes_of(corpus: &Value) -> Vec<String> {
    corpus["excludes"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// Compute manifest lines ("<sha256>  <relpath>") for a directory corpus.
fn build_manifest(dir: &Path, excludes: &[String]) -> Result<(Vec<String>, u64), String> {
    let files = walk_sorted(dir, excludes)?;
    let mut lines = Vec::with_capacity(files.len());
    let mut total = 0u64;
    for rel in &files {
        let (hash, size) = sha256_file(&dir.join(rel))?;
        total += size;
        lines.push(format!("{}  {}", hash, rel.display()));
    }
    Ok((lines, total))
}

pub fn cmd_update(_args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let mut catalog = load_catalog(&repo_root)?;
    let roots = catalog["roots"].clone();
    fs::create_dir_all(repo_root.join(MANIFEST_DIR)).map_err(|e| e.to_string())?;

    let corpora = catalog["corpora"].as_array_mut().ok_or("catalog has no corpora array")?;
    for corpus in corpora.iter_mut() {
        let id = corpus["id"].as_str().unwrap_or("?").to_string();
        match corpus["kind"].as_str() {
            Some("file") => {
                let path = corpus_dir(&roots, corpus)?;
                let (hash, size) = sha256_file(&path)?;
                corpus["sha256"] = json!(hash);
                corpus["size"] = json!(size);
                println!("{}: file ok ({} bytes)", id, size);
            }
            Some("directory") => {
                let dir = corpus_dir(&roots, corpus)?;
                let excludes = excludes_of(corpus);
                let (lines, total) = build_manifest(&dir, &excludes)?;
                let manifest_rel = format!("{}/{}.sha256", MANIFEST_DIR, id);
                fs::write(repo_root.join(&manifest_rel), lines.join("\n") + "\n")
                    .map_err(|e| e.to_string())?;
                corpus["manifest"] = json!(manifest_rel);
                corpus["files"] = json!(lines.len());
                corpus["total_size"] = json!(total);
                println!("{}: {} files, {} bytes -> {}", id, lines.len(), total, manifest_rel);
            }
            Some("missing") => {
                let path = corpus_dir(&roots, corpus)?;
                if path.exists() {
                    println!("{}: WARNING - marked missing but path exists: {}", id, path.display());
                } else {
                    println!("{}: still missing (expected)", id);
                }
            }
            other => return Err(format!("{}: unknown kind {:?}", id, other)),
        }
    }

    catalog["updated"] = json!(chrono_date());
    let out = serde_json::to_string_pretty(&catalog).map_err(|e| e.to_string())? + "\n";
    fs::write(repo_root.join(CATALOG_PATH), out).map_err(|e| e.to_string())?;
    println!("catalog updated: {}", CATALOG_PATH);
    Ok(())
}

pub fn cmd_verify(_args: &[String]) -> Result<(), String> {
    let repo_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let catalog = load_catalog(&repo_root)?;
    let roots = &catalog["roots"];
    let mut problems = 0usize;

    for corpus in catalog["corpora"].as_array().ok_or("no corpora")? {
        let id = corpus["id"].as_str().unwrap_or("?");
        match corpus["kind"].as_str() {
            Some("file") => {
                let path = corpus_dir(roots, corpus)?;
                match sha256_file(&path) {
                    Ok((hash, _)) if Some(hash.as_str()) == corpus["sha256"].as_str() => {
                        println!("OK   {}", id);
                    }
                    Ok((hash, _)) => {
                        println!("DRIFT {}: sha256 {} != cataloged {}", id, hash,
                                 corpus["sha256"].as_str().unwrap_or("?"));
                        problems += 1;
                    }
                    Err(e) => {
                        println!("MISSING {}: {}", id, e);
                        problems += 1;
                    }
                }
            }
            Some("directory") => {
                let dir = corpus_dir(roots, corpus)?;
                let manifest_rel = corpus["manifest"].as_str().ok_or_else(
                    || format!("{}: no manifest - run catalog-update first", id))?;
                let manifest = fs::read_to_string(repo_root.join(manifest_rel))
                    .map_err(|e| format!("{}: {}", manifest_rel, e))?;
                let excludes = excludes_of(corpus);
                let (lines, _) = build_manifest(&dir, &excludes)?;
                let expected: Vec<&str> = manifest.lines().collect();
                if expected == lines.iter().map(|s| s.as_str()).collect::<Vec<_>>() {
                    println!("OK   {} ({} files)", id, lines.len());
                } else {
                    let exp: std::collections::HashSet<&str> = expected.iter().copied().collect();
                    let got: std::collections::HashSet<&str> =
                        lines.iter().map(|s| s.as_str()).collect();
                    for missing in exp.difference(&got).take(5) {
                        println!("DRIFT {}: missing/changed: {}", id, missing);
                    }
                    for extra in got.difference(&exp).take(5) {
                        println!("DRIFT {}: new/changed: {}", id, extra);
                    }
                    problems += 1;
                }
            }
            Some("missing") => {
                let path = corpus_dir(roots, corpus)?;
                if path.exists() {
                    println!("FOUND {}: marked missing but exists - update the catalog!", id);
                    problems += 1;
                } else {
                    println!("OK   {} (recorded as missing)", id);
                }
            }
            _ => {}
        }
    }

    if problems > 0 {
        Err(format!("{} corpora drifted from the evidence catalog", problems))
    } else {
        println!("evidence catalog verified: all corpora intact");
        Ok(())
    }
}

/// yyyy-mm-dd without pulling a date crate: seconds since epoch -> civil date.
fn chrono_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86400) as i64;
    // civil-from-days (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", y, m, d)
}
