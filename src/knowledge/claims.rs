//! The Status Ladder as a type, and the one emitter for `knowledge/claims/`.
//!
//! Before this module the ladder was string literals typed per site
//! (`json!("verified")` in the producer, `== "verified"` in a different
//! module's consumer) with nothing connecting the two, and serialisation was
//! three formats and three provenance shapes across five writers — three of
//! which emitted a claims file with no provenance at all. `Status` links
//! producer to consumer at compile time; `Claims` owns the format and the
//! provenance envelope, so ADR-0004's byte-for-byte regeneration is a property
//! of one module and a claims file with no `schema`/`generated_by` is no longer
//! expressible. (ADR-0004; CONTEXT.md -> Status Ladder.)

use serde::{Serialize, Serializer};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::Path;

use super::catalog::sha256_file;

/// The Status Ladder (CONTEXT.md), as a type rather than a string literal.
///
/// `Hypothesis -> Corroborated -> Verified` is the ladder proper; `Skipped` and
/// `Failed` are the operational outcomes a multi-slot differential records when
/// its anchor will not resolve or its pattern does not match. Applications
/// consume `Corroborated` and `Verified` only.
///
/// There is deliberately no `Tombstoned` variant: a refuted claim is not a
/// status a live claim carries, it is an entry in the `tombstones` array with
/// its own shape. A never-constructed variant is exactly the dead code the
/// architecture review set out to remove, so tombstones stay off this enum.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Status {
    Hypothesis,
    Corroborated,
    Verified,
    Skipped,
    Failed,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Hypothesis => "hypothesis",
            Status::Corroborated => "corroborated",
            Status::Verified => "verified",
            Status::Skipped => "skipped",
            Status::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "hypothesis" => Status::Hypothesis,
            "corroborated" => Status::Corroborated,
            "verified" => Status::Verified,
            "skipped" => Status::Skipped,
            "failed" => Status::Failed,
            _ => return None,
        })
    }

    /// Read a `status` field back off an emitted claim. `None` for a missing or
    /// unrecognised value — the same refusal the string comparison made, now
    /// against a closed set.
    pub fn from_json(v: &Value) -> Option<Self> {
        v.as_str().and_then(Status::from_str)
    }

    /// The ladder rule applications enforce: consume `Corroborated` and
    /// `Verified` only. Named once here so no consumer restates it by string.
    pub fn is_consumable(self) -> bool {
        matches!(self, Status::Corroborated | Status::Verified)
    }
}

impl Serialize for Status {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

/// Emitter for every file under `knowledge/claims/`. Owns the serialisation
/// format (pretty, trailing newline) and the provenance envelope: a `schema`
/// tag and a `generated_by` command string are present by construction.
/// `input` records the sha256 of a generator input; `note` accumulates
/// free-text caveats; `field`/`body` add the command-specific payload.
pub struct Claims {
    fields: Map<String, Value>,
}

impl Claims {
    /// A new emitter carrying its schema id and the command that produced it.
    pub fn new(schema: &str, generated_by: &str) -> Self {
        let mut fields = Map::new();
        fields.insert("schema".into(), json!(schema));
        fields.insert("generated_by".into(), json!(generated_by));
        Self { fields }
    }

    /// Record one generator input under `name`, hashing the file at repo-relative
    /// `rel`. Accumulates into an `inputs` object; a drifted or missing input is
    /// a hard error, so provenance can never point at bytes that were not read.
    pub fn input(mut self, name: &str, repo_root: &Path, rel: &str) -> Result<Self, String> {
        let (hash, _) = sha256_file(&repo_root.join(rel))?;
        self.fields
            .entry("inputs")
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .unwrap()
            .insert(name.into(), json!({ "path": rel, "sha256": hash }));
        Ok(self)
    }

    /// Merge the top-level fields of an object `Value` — the command-specific
    /// payload built as one `json!({...})`. Panics if `value` is not an object;
    /// a claims body always is.
    pub fn body(mut self, value: Value) -> Self {
        let Value::Object(map) = value else {
            panic!("Claims::body expects a JSON object");
        };
        self.fields.extend(map);
        self
    }

    /// Serialise and write to repo-relative `rel_path`. Keys are emitted sorted
    /// (serde_json without `preserve_order`), pretty-printed with a trailing
    /// newline. Returns whether the bytes on disk changed.
    pub fn write(self, repo_root: &Path, rel_path: &str) -> Result<bool, String> {
        let path = repo_root.join(rel_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(&Value::Object(self.fields))
            .map_err(|e| e.to_string())?
            + "\n";
        let changed = fs::read_to_string(&path).map(|old| old != text).unwrap_or(true);
        fs::write(&path, &text).map_err(|e| format!("{}: {}", rel_path, e))?;
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trips_through_its_string() {
        for s in [
            Status::Hypothesis,
            Status::Corroborated,
            Status::Verified,
            Status::Skipped,
            Status::Failed,
        ] {
            assert_eq!(Status::from_str(s.as_str()), Some(s));
            assert_eq!(Status::from_json(&json!(s.as_str())), Some(s));
            assert_eq!(serde_json::to_value(s).unwrap(), json!(s.as_str()));
        }
        assert_eq!(Status::from_str("tombstoned"), None);
        assert_eq!(Status::from_json(&Value::Null), None);
    }

    #[test]
    fn only_corroborated_and_verified_are_consumable() {
        assert!(Status::Verified.is_consumable());
        assert!(Status::Corroborated.is_consumable());
        assert!(!Status::Hypothesis.is_consumable());
        assert!(!Status::Skipped.is_consumable());
        assert!(!Status::Failed.is_consumable());
    }

    #[test]
    fn write_is_pretty_sorted_and_newline_terminated() {
        let dir = std::env::temp_dir().join(format!(
            "er-claims-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let rel = "knowledge/claims/probe.json";
        let changed = Claims::new("probe/1", "er-save-reader knowledge probe")
            .body(json!({ "zeta": 1, "alpha": 2 }))
            .write(&dir, rel)
            .unwrap();
        assert!(changed, "a fresh file is always a change");
        let text = std::fs::read_to_string(dir.join(rel)).unwrap();
        assert!(text.ends_with("}\n"), "must end with a single trailing newline");
        // keys sorted: alpha before generated_by before zeta
        let a = text.find("alpha").unwrap();
        let g = text.find("generated_by").unwrap();
        let z = text.find("zeta").unwrap();
        assert!(a < g && g < z, "keys must serialise sorted");
        // second identical write reports unchanged
        let again = Claims::new("probe/1", "er-save-reader knowledge probe")
            .body(json!({ "zeta": 1, "alpha": 2 }))
            .write(&dir, rel)
            .unwrap();
        assert!(!again, "an identical rewrite is not a change");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
