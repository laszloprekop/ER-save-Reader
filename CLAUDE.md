## Commit Protocol

**IMPORTANT**: Never commit automatically. Always use the `/snapshot` command so version
bumps, changelog, and docs stay consistent.

## Remembering Command execution fault

**IMPORTANT**: When an (allowed) executed command throws an error and a corrected form of
the same command succeeds afterwards, take note of the correct form to avoid burning tokens
repeating the mistake.

---

## Orientation — read these first

- **Canonical vocabulary / glossary**: `CONTEXT.md` (Evidence, Claim, Origin, Family
  Constant, Resolver, Flag Family, Unknown, Epistemic Header, …). Use these terms; add new
  jargon there rather than inventing it per-conversation.
- **Decisions**: `docs/adr/` (ADR-0001 … ADR-0008).
- **Evidence inventory**: `knowledge/evidence-catalog.json` — sha256 index over all
  out-of-repo evidence with per-corpus trust context. Verify with
  `er-save-editor knowledge catalog-verify` before relying on any evidence file.
- **Claims store**: `knowledge/claims/event-flags.json` — pipeline-generated
  (`er-save-editor knowledge run`), NEVER hand-edited (ADR-0004). For the families it
  covers it supersedes `ground_truth_offsets.json` and the doc base tables. Check its
  tombstones before re-proposing any offset convention.
- **Docs**: every file in `docs/` opens with an Epistemic Header (status + claims /
  evidence / methodology / obsolete — see `CONTEXT.md`). Read it before trusting a number;
  several docs are era-mixed.

## Game files & saves

- **Save files** (5 character slots) and **decompiled game resources**: authoritative
  inventory is `docs/DATA-SOURCES.md` + the evidence catalog. Game extracts:
  `regulation-bin/*.param.xml` (WitchyBND, regulation 1.16.1, corpus `game-extracts`); raw
  EMEVD / alloclists / MSBs / regulation.bin in corpus `game-raw-1162`. The old
  `event/*.emevd.js` decompiles were NOT regenerated — the pipeline parses raw `.emevd`
  natively.
- **Slot 0 gotcha**: in the 2026-01-11 backup Margit and Godrick ARE defeated and Radahn is
  NOT (corrected 2026-07-20; older "predates all three" notes were measured at
  pre-migration offsets).

## Third-party resources

Treat third-party resources with caution — usually version-specific, outdated, partially
implemented. Always verify against primary sources; discard what can't be proven.

---

## False Negative Investigation Protocol

**MANDATORY**: when auto-detection fails where manual succeeds, gather evidence BEFORE
proposing any fix. NEVER skip straight to a fix — detection has many moving parts
(calibration, per-save offsets, formula correctness) and speculation wastes effort.

1. **Evidence** — read the actual save bytes; confirm observed vs expected offset; check
   both flag systems (tile AND block for world pickups).
2. **Multi-slot differential** (gold standard) — compare the byte across slots V1/V2/V3 with
   known-different progression. See `docs/discovery-verification-cycle.md`.
3. **Corroboration** — tile flag SET + block flag SET = corroborated; if they disagree, the
   disagreement IS the clue. See `docs/CORROBORATION-SYSTEM.md`.
4. **Origin** — resolve the family base for THIS save
   (`wasm_event_flags::resolve_family_base_in_ef`); confirm the resolver did not refuse (an
   unresolved origin reads Unknown, not "clear" — `CONTEXT.md`); confirm you asked for the
   right family (a bare tile id does not identify one). 337,375 is the distance BETWEEN two
   families, not a base — every family base floats per save (tombstoned; `CONTEXT.md`).

Only then build a before/after hex test case, label confidence (VERIFIED / LIKELY /
UNVERIFIED), and propose the fix.

---

## Technical Documentation

| Topic | Document |
|-------|----------|
| **Canonical vocabulary** | `CONTEXT.md` |
| **Decisions** | `docs/adr/` |
| System architecture (live) | `docs/WASM-EVENT-FLAGS.md` — note `docs/ARCHITECTURE.md` is SUPERSEDED |
| Event flag geography | `docs/EVENT-FLAG-GEOGRAPHY.md` (era-mixed — numbers obsolete) |
| Discovery / verification methodology | `docs/discovery-verification-cycle.md`, `docs/CORROBORATION-SYSTEM.md`, `docs/CASE-VERIFICATION-GUIDE.md` |
| Save file structure & family Origin | `docs/SAVE_FILE_GROUND_TRUTH.md` |
| Database coverage | `docs/DATABASE_COVERAGE_ANALYSIS.md` |
| Event template semantics | `docs/EVENT_TEMPLATE_CATALOG.md` |
| Data sources & characters | `docs/DATA-SOURCES.md` |
| Backlog / migration plan | `docs/BACKLOG.md` |
| Frozen legacy store | `ground_truth_offsets.json` (FROZEN read-only, ADR-0006) |
| Save slot feature registry | `save_slot_registry.json` |

**Single Source of Truth** (detail in `CONTEXT.md`):
- **Resolve flag positions, never hardcode them.** Families sit after an append-only u32
  list that grows with play, so any fixed offset is valid only for the save it was measured
  on. Use `wasm_event_flags::resolve_family_base(slot, FAMILY_*)`. See `CONTEXT.md` →
  *Origin*, *Family Constant*, *Resolver*.
- `ground_truth_offsets.json`: **FROZEN read-only** (ADR-0006) — never add or edit; new
  knowledge goes to the claims store via `knowledge run`. `flag_formulas.py` deprecated.
- EventFlags detection lives only in `crates/wasm-event-flags/` (shared with elden-map via
  WASM).
- A bare 10-digit tile id is AMBIGUOUS between the open-world and pickup families (regions
  500 bytes apart) — the caller must pick `is_tile_world_flag_set` vs `is_tile_pickup_set`;
  routing on the value silently reads the wrong bit. `pickup_data.rs` stores row_ids, not
  getItemFlagIds.
- Legacy maps (8-digit) split by localId: `is_dungeon_flag_set` (< 7000) vs
  `is_dungeon_pickup_set` (>= 7000), layout `alloc_slot(map) * 1125 + localId / 8` with
  slots from the game's own alloclists — NOT the deleted "+3375 per area" stride table
  (ADR-0008).
- **The wasm crate holds no flag base tables, by design** (ADR-0008). No export may source a
  flag's position from inside the crate — it takes the save/flag bytes and resolves the
  family, or takes the base as a parameter. `tests/export_shape_conformance.rs` enforces
  this, including against the model regrowing under a new name. Do not reintroduce a
  `flag_id → byte offset` function; there is no correct value for it to return.
- Tutorial graces (71800/76100) are NOT universal anchors — they are clear on minimal
  characters; never use them as a validity test for a detected offset.
