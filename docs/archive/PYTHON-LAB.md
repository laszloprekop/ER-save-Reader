# The Python lab (distilled record)

> **Epistemic header** (added 2026-07-21 · BACKLOG step 5)
> **Status: DELETED — this is the distilled record of ~50k lines of pre-reset Python that
> lived under `scripts/` and was removed in migration step 5.** The code is recoverable from
> git history; this file is the "recorded dead end" so nobody re-derives what it did.
> - **Claims** it produced are NOT trusted. The knowledge-base reset (ADR-0001…0006) rebuilt
>   flag knowledge from raw bytes via the Rust pipeline; see `knowledge/claims/event-flags.json`
>   (ADR-0004) and `crates/wasm-event-flags` (ADR-0005).
> - **Evidence**: the lab's own JSON outputs (`discoveries.json`, `unified_flags.json`,
>   `param_flags.json`, `event_graph.json`, `flag_relationships.json`, the
>   `scripts/verification/` case store) were deleted with it — none were in the evidence
>   catalog, and all were consumed only by the deleted `src/discovery/` lab modules.
> - **Obsolete**: every offset/formula the lab measured. Family bases float per save
>   (`CONTEXT.md` → *Origin*); the lab's fixed offsets and stride tables are why the reset
>   happened. Do not reintroduce them.

---

## What the lab was

The pre-reset workflow: Python scripts read decompiled game files and save snapshots,
guessed flag offsets by fitting formulas to a few anchors, and recorded "discoveries" into
JSON stores that a Rust CLI (`er-save-editor discovery …`, also removed in step 5) browsed.
The whole approach assumed flag positions were stable across saves. They are not — that
finding (per-save family float) is what invalidated the lab and motivated the reset.

## Script groups (all removed)

| group | scripts (representative) | produced | replaced by |
|---|---|---|---|
| **Game-file extraction** | `extract_event_flags.py`, `extract_event_graph.py`, `extract_flag_relationships.py`, `extract_pickup_data.py`, `extract_shop_items.py`, `extract_spells.py`, `extract_world_pickups*.py`, `expand_flag_catalog.py`, `build_pickup_section_map.py`, `generate_db.py`, `generate_dungeon_pickups.py` | the committed `src/db/*_data.rs` / `pickup_*.rs` / `spells.rs` / `shop_items.rs` tables (these **survive** in-tree) | the pipeline parses raw `.emevd` / regulation params natively (`docs/DATA-SOURCES.md`) |
| **Base discovery** | `discover_*_bases.py`, `calibrate_dungeon_bases.py`, `refine_dungeon_bases.py`, `discover_bases_from_snapshots.py` | per-area base offsets & stride tables (all **obsolete** — the "+3375/area" stride was later deleted, ADR-0008) | `wasm_event_flags::resolve_family_base` (per-save origin) |
| **Verification** | `run_verification.py`, `capture_agent.py`, `verify_*.py`, `diff_precise_snapshots.py`, `scripts/verification/**` (119 py + case JSONs) | the case-based verification store (`scripts/verification/cases/`) | `er-save-editor knowledge run` + conformance fixtures (ADR-0003/0004) |
| **Timeline** | `timeline_analysis.py`, `timeline_graces_pickups.py`, `timeline_narrative.py` | narrative reconstructions of capture chains | `er-save-editor knowledge timeline` (`src/knowledge/timeline.rs`) |
| **One-off checks** | `scripts/archive/*.py` (10) | ad-hoc byte probes | — |

## What survived, and why

- **`src/db/*.rs` data tables** — kept as committed data. Their headers still say "auto-generated
  by `scripts/generate_db.py`"; that generator is gone, so the tables are now maintained in-tree
  (or regenerated from primary sources per `docs/DATA-SOURCES.md`), not by re-running the lab.
- **`scripts/windows/regenerate-game-extracts.ps1`** — kept. It is the documented way to
  regenerate the `game-extracts` corpus via WitchyBND and is cited by the evidence catalog's
  `game-extracts` provenance note. Not Python lab.
- **`notebooks/ml_flag_discovery.ipynb`** — separately archived with its own status header
  (`notebooks/README.md`); kept for the flip-clustering method in cells 27–28.
- **`scripts/extracted_event_flags.json`** — was flagged (BACKLOG step 2) as the last survivor of
  the once-missing decompiled corpus, "provenance unverifiable". The corpus was later restored
  (`game-raw-1162` / `game-extracts`), so this derived copy carried no unique evidence and went
  with the lab.

## Where the reusable knowledge already lives

The lab's durable lessons were captured before deletion and are NOT re-stated here:
methodology in `docs/discovery-verification-cycle.md` + `docs/CASE-VERIFICATION-GUIDE.md`,
the per-save-float finding and vocabulary in `CONTEXT.md`, and the migration reasoning
(including the tombstoned constants the lab mis-measured) in `docs/BACKLOG.md`.
