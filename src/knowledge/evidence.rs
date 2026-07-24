//! The verified read side of the Evidence Catalog (ADR-0001).
//!
//! Analysis code obtains evidence bytes ONLY through `Evidence`. Every read is
//! checked against the catalog — a file corpus against its `sha256`, a file
//! inside a directory corpus against its manifest line — and a mismatch is a
//! hard error, not a warning. There is deliberately no method that returns bytes
//! without that check, and none that hands out a path for a caller to read
//! unverified afterwards.
//!
//! Before this module the "catalog entry -> verified bytes" walk was hand-rolled
//! in six places (`pipeline.rs`, `family_distances.rs` x4, `timeline.rs`), and
//! two of the copies had already drifted: one `continue`d past a corpus missing
//! from the catalog instead of erroring, and the pickup generator's primary
//! source was read with no sha256 check at all. Concentrating the walk here makes
//! each of those a single, testable behaviour — the deletion test is that
//! removing `Evidence` scatters drift detection back across those six sites, one
//! of which omitted it. (ADR-0001, ADR-0004.)

use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::catalog::sha256_file;

const CATALOG_PATH: &str = "knowledge/evidence-catalog.json";

/// Save-container geometry: a 0x300 header, then 10 x (0x10 checksum + slot).
/// Defined once here; everything that slices a raw `.sl2` image — `Evidence`,
/// the pipeline, `family_distances`, `grace-dump` — shares these.
pub const HEADER: usize = 0x300;
pub const CHECKSUM: usize = 0x10;
pub const SLOT_SIZE: usize = 0x280000;

/// One raw save slot from a full `.sl2` image, or `None` if the image is too
/// short to contain it.
pub fn slot_slice(data: &[u8], slot: usize) -> Option<&[u8]> {
    let start = HEADER + slot * (CHECKSUM + SLOT_SIZE) + CHECKSUM;
    data.get(start..start + SLOT_SIZE)
}

/// Read a file and refuse on any drift from the expected sha256. The single home
/// of the `EVIDENCE DRIFT` contract: `Evidence`'s own reads and the timeline
/// diff reader both go through here rather than restating the check. `label` is
/// what the error names (a corpus id, a rel path, a diff file) — the same string
/// each call site used before.
pub fn read_verified(path: &Path, expected_sha256: &str, label: &str) -> Result<Vec<u8>, String> {
    let (hash, _) = sha256_file(path)?;
    if hash != expected_sha256 {
        return Err(format!(
            "EVIDENCE DRIFT {}: sha256 {} != cataloged {}",
            label, hash, expected_sha256
        ));
    }
    fs::read(path).map_err(|e| format!("{}: {}", path.display(), e))
}

/// The Evidence Catalog, opened once, answering verified-byte reads.
///
/// The byte cache sits behind `RefCell` so the read methods take `&self`: a
/// caller opens one `Evidence` and reads many files without threading a mutable
/// borrow. Reads are memoized by `(corpus, rel)`, and directory manifests by
/// corpus id, so a screen or a differential that touches the same file twice
/// hashes it once.
/// A directory manifest: rel path -> sha256.
type Manifest = BTreeMap<String, String>;
/// Read-and-verified byte cache, keyed by `(corpus id, rel path)`.
type ByteCache = BTreeMap<(String, String), Arc<[u8]>>;

pub struct Evidence {
    repo_root: PathBuf,
    catalog: Value,
    manifests: RefCell<BTreeMap<String, Arc<Manifest>>>,
    bytes: RefCell<ByteCache>,
}

impl Evidence {
    /// Open the catalog under `repo_root`. Does not read any evidence yet —
    /// verification is lazy, per file, so `catalog-verify` (which wants every
    /// corpus) and `grace-dump` (which wants one file) pay only for what they
    /// touch.
    pub fn open(repo_root: &Path) -> Result<Self, String> {
        let path = repo_root.join(CATALOG_PATH);
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
        let catalog = serde_json::from_str(&text).map_err(|e| format!("catalog parse: {}", e))?;
        Ok(Self {
            repo_root: repo_root.to_path_buf(),
            catalog,
            manifests: RefCell::new(BTreeMap::new()),
            bytes: RefCell::new(BTreeMap::new()),
        })
    }

    fn corpus(&self, id: &str) -> Result<&Value, String> {
        self.catalog["corpora"]
            .as_array()
            .and_then(|cs| cs.iter().find(|c| c["id"] == id))
            .ok_or_else(|| format!("corpus {} not in evidence catalog", id))
    }

    fn corpus_dir(&self, corpus: &Value) -> Result<PathBuf, String> {
        let root_key = corpus["root"].as_str().ok_or("corpus missing root")?;
        let root = self.catalog["roots"][root_key]
            .as_str()
            .ok_or_else(|| format!("unknown root {}", root_key))?;
        let rel = corpus["path"].as_str().ok_or("corpus missing path")?;
        Ok(Path::new(root).join(rel))
    }

    /// Directory corpus manifest (rel -> sha256), memoized by corpus id.
    fn manifest(&self, corpus_id: &str, corpus: &Value) -> Result<Arc<Manifest>, String> {
        if let Some(m) = self.manifests.borrow().get(corpus_id) {
            return Ok(m.clone());
        }
        let manifest_rel = corpus["manifest"]
            .as_str()
            .ok_or_else(|| format!("corpus {} has no manifest", corpus_id))?;
        let text = fs::read_to_string(self.repo_root.join(manifest_rel))
            .map_err(|e| format!("{}: {}", manifest_rel, e))?;
        let mut map = BTreeMap::new();
        for line in text.lines() {
            if let Some((hash, rel)) = line.split_once("  ") {
                map.insert(rel.to_string(), hash.to_string());
            }
        }
        let arc = Arc::new(map);
        self.manifests.borrow_mut().insert(corpus_id.to_string(), arc.clone());
        Ok(arc)
    }

    /// The cataloged sha256 for a file: the corpus `sha256` for a file corpus
    /// (`rel` ignored), the manifest line for a directory corpus. This is the
    /// EXPECTED hash; a successful `bytes`/`slot` read proves the bytes on disk
    /// match it.
    pub fn sha256(&self, corpus_id: &str, rel: &str) -> Result<String, String> {
        let corpus = self.corpus(corpus_id)?;
        match corpus["kind"].as_str() {
            Some("file") => corpus["sha256"]
                .as_str()
                .map(String::from)
                .ok_or_else(|| format!("corpus {} missing sha256", corpus_id)),
            Some("directory") => {
                let m = self.manifest(corpus_id, corpus)?;
                m.get(rel)
                    .cloned()
                    .ok_or_else(|| format!("{}: not in {} manifest", rel, corpus_id))
            }
            other => Err(format!("corpus {}: cannot read kind {:?}", corpus_id, other)),
        }
    }

    /// Verified bytes of a file within a corpus. For a file corpus, `rel` is
    /// ignored — the corpus IS the file. For a directory corpus, `rel` is the
    /// path within it. Memoized by `(corpus, rel)`.
    pub fn bytes(&self, corpus_id: &str, rel: &str) -> Result<Arc<[u8]>, String> {
        let key = (corpus_id.to_string(), rel.to_string());
        if let Some(b) = self.bytes.borrow().get(&key) {
            return Ok(b.clone());
        }
        let corpus = self.corpus(corpus_id)?;
        let (path, expected, label) = match corpus["kind"].as_str() {
            Some("file") => {
                let p = self.corpus_dir(corpus)?;
                let sha = corpus["sha256"]
                    .as_str()
                    .ok_or_else(|| format!("corpus {} missing sha256", corpus_id))?;
                (p, sha.to_string(), corpus_id.to_string())
            }
            Some("directory") => {
                let dir = self.corpus_dir(corpus)?;
                let m = self.manifest(corpus_id, corpus)?;
                let sha = m
                    .get(rel)
                    .ok_or_else(|| format!("{}: not in {} manifest", rel, corpus_id))?
                    .clone();
                (dir.join(rel), sha, rel.to_string())
            }
            other => return Err(format!("corpus {}: cannot read kind {:?}", corpus_id, other)),
        };
        let data = read_verified(&path, &expected, &label)?;
        let arc: Arc<[u8]> = Arc::from(data.into_boxed_slice());
        self.bytes.borrow_mut().insert(key, arc.clone());
        Ok(arc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn sha_hex(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    /// A synthetic repo: a catalog with one file corpus and one directory
    /// corpus, their evidence under an absolute root, and a manifest. Returned as
    /// a unique temp dir the caller removes.
    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new(file_body: &[u8], dir_body: &[u8]) -> Self {
            // Unique per call, not just per wall-clock instant. Under parallel test
            // threads `as_nanos()` can collide (coarse clock resolution); two fixtures
            // then share one temp dir and one's Drop (`remove_dir_all`) deletes it out
            // from under the other's `Evidence::open`, which is the intermittent panic
            // this test used to hit. The atomic counter guarantees a distinct path.
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let uniq = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("er-evidence-test-{uniq}-{}-{seq}", std::process::id()));
            let data = root.join("data");
            let dir = data.join("dir");
            fs::create_dir_all(&dir).unwrap();
            fs::create_dir_all(root.join("knowledge/manifests")).unwrap();
            fs::write(data.join("file1.bin"), file_body).unwrap();
            fs::write(dir.join("sub.bin"), dir_body).unwrap();
            fs::write(
                root.join("knowledge/manifests/dircorp.sha256"),
                format!("{}  sub.bin\n", sha_hex(dir_body)),
            )
            .unwrap();
            let catalog = serde_json::json!({
                "roots": { "data": data.to_str().unwrap() },
                "corpora": [
                    { "id": "filecorp", "kind": "file", "root": "data",
                      "path": "file1.bin", "sha256": sha_hex(file_body) },
                    { "id": "dircorp", "kind": "directory", "root": "data",
                      "path": "dir", "manifest": "knowledge/manifests/dircorp.sha256" }
                ]
            });
            fs::write(root.join(CATALOG_PATH), serde_json::to_string_pretty(&catalog).unwrap())
                .unwrap();
            Fixture { root }
        }

        fn corrupt_file_corpus(&self, new_body: &[u8]) {
            fs::write(self.root.join("data/file1.bin"), new_body).unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn reads_verified_bytes_from_file_and_directory_corpora() {
        let fx = Fixture::new(b"the whole file", b"one entry");
        let ev = Evidence::open(&fx.root).unwrap();
        // File corpus: rel is ignored, the corpus IS the file.
        assert_eq!(&*ev.bytes("filecorp", "").unwrap(), b"the whole file");
        // Directory corpus: rel names the file within.
        assert_eq!(&*ev.bytes("dircorp", "sub.bin").unwrap(), b"one entry");
        // sha256 returns the cataloged hash, and a verified read proves it.
        assert_eq!(ev.sha256("filecorp", "").unwrap(), sha_hex(b"the whole file"));
        assert_eq!(ev.sha256("dircorp", "sub.bin").unwrap(), sha_hex(b"one entry"));
    }

    #[test]
    fn drift_is_a_hard_error_not_a_silent_read() {
        let fx = Fixture::new(b"original", b"x");
        fx.corrupt_file_corpus(b"tampered");
        let ev = Evidence::open(&fx.root).unwrap();
        let err = ev.bytes("filecorp", "").unwrap_err();
        assert!(err.contains("EVIDENCE DRIFT"), "unexpected error: {err}");
    }

    #[test]
    fn a_missing_corpus_is_named_not_treated_as_empty() {
        let fx = Fixture::new(b"a", b"b");
        let ev = Evidence::open(&fx.root).unwrap();
        let err = ev.bytes("nope", "").unwrap_err();
        assert!(err.contains("not in evidence catalog"), "unexpected error: {err}");
        // A file absent from a directory manifest is refused, not read.
        assert!(ev.bytes("dircorp", "ghost.bin").unwrap_err().contains("not in dircorp manifest"));
    }

    #[test]
    fn bytes_are_memoized_so_a_file_is_hashed_once() {
        let fx = Fixture::new(b"cache me", b"y");
        let ev = Evidence::open(&fx.root).unwrap();
        let a = ev.bytes("filecorp", "").unwrap();
        let b = ev.bytes("filecorp", "").unwrap();
        assert!(Arc::ptr_eq(&a, &b), "second read should return the memoized Arc");
    }

    #[test]
    fn slot_slice_addresses_the_container_geometry() {
        // One header + one slot: slot 0 is present, slot 1 runs off the end.
        let buf = vec![0u8; HEADER + CHECKSUM + SLOT_SIZE];
        assert_eq!(slot_slice(&buf, 0).map(|s| s.len()), Some(SLOT_SIZE));
        assert_eq!(slot_slice(&buf, 1), None);
    }
}
