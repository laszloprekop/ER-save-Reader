# Changelog

All notable changes to ER-save-Editor will be documented in this file.

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: LIVING RECORD — chronological, newest first.** Each entry is true as of its date and is not retroactively corrected; later entries supersede earlier ones. For the current canonical state read `CONTEXT.md`, the ADRs, and the claims store — not old entries here.

---

## v0.33.0 - Per-save flag-offset export; elden-map cutover (BACKLOG step 4)

Completes the last migration step: elden-map moves off the removed static-offset
exports onto the per-save family readers. The only crate change is one new export;
the bulk of the work is in the elden-map repo (separate commit).

### Features
- **New WASM export `flag_offset_in_ef(event_flags, flag_id, family)`** — the honest,
  ADR-0008-compliant replacement for the removed static `get_flag_offset`. Takes the
  flag region and a family selector (`FAMILY_CODE_*`), resolves that family's base for
  THIS save via `resolve_family_base_in_ef`, and returns `valid=false` when the id is
  out of family or the origin cannot be pinned. Invents no base. Its consumer is
  elden-map's character-explorer hex view, which needs a per-save byte *position*, not
  just a bit — the one capability the ADR-0008 removals left with no replacement.

### Conformance
- Added to the `export_shape_conformance` `APPROVED_EXPORTS` manifest (it takes
  `&[u8]`, so the structural guard passes).
- Two new parity tests in `origin_conformance.rs`: `flag_offset_in_ef` lands on the
  EXACT byte/bit the five tri-state readers use (checked both directions across all
  families on a synthetic region), and refuses on the same out-of-family / unresolvable
  inputs. This keeps the offset export and the state readers from drifting apart.

### elden-map (separate repo, branch `event-flags-adr0008-cutover`)
- Rebuilt + vendored the wasm; new `shared/flag-reader.ts` routing layer
  (`readFlagState` / `resolveFlagOffset`, three-state); deleted every TS static-offset
  fallback (incl. tombstoned `337375`), four duplicate `calculateFlagLocation` copies,
  and `CalibrationService` from the read path; re-pointed all consumers (graces, bosses,
  pickups, detection, the byte-diff engine → per-save state comparison, and the live
  Character Explorer analysis + hex view). Deprecated the verification/calibration
  testing subsystem in place; stopped the capture agent baking a fabricated tile base
  into evidence (ADR-0007).
- Verified out-of-sample through elden-map's own `parseSaveFile` on the 2026-01-11
  backup (Confessor slot 0): **179 graces** (exact match to the ER-save-Editor
  validation), Margit ✓ / Godrick ✓ / Radahn ✗. Client + server typecheck clean;
  `vite build` succeeds.

### Files Modified
- crates/wasm-event-flags/src/lib.rs: `flag_offset_in_ef` + `FAMILY_CODE_*` constants
- crates/wasm-event-flags/tests/export_shape_conformance.rs: manifest entry
- crates/wasm-event-flags/tests/origin_conformance.rs: two parity tests
- docs/BACKLOG.md: step 4 marked done; migration plan (steps 1-6) complete
- docs/CHANGELOG.md, Cargo.toml: v0.33.0

## v0.32.0 - Distill and delete the pre-reset lab (BACKLOG step 5)

Removes the pre-reset Python lab and its Rust discovery sibling, and moves the CE-era
Rosetta table out of the shipping app into the knowledge base. Distill-first: every deletion
was preceded by an epistemic-headed record so the reasoning survives the code (ADR-0004,
"a recorded dead end is worth more than a deleted one").

### Refactor
- **Python lab deleted — 209 files** (161 `.py` + the lab's own JSON / case-store artifacts).
  None were in the evidence catalog and all were consumed only by the deleted `src/discovery`
  modules. **Distilled first** into `docs/archive/PYTHON-LAB.md` (grouped record of what each
  script family did, what survived, what replaced it). **Kept:** `scripts/windows/
  regenerate-game-extracts.ps1` (operational; cited by the `game-extracts` catalog corpus)
  and `notebooks/` (separately archived).
- **`src/discovery` shrunk to what the app uses.** The live pipeline (`src/knowledge`) used
  none of it. The only app dependency was the `inventory_verification` leaf (UNIQUE_ITEMS
  tables) → relocated to `src/db/inventory_verification.rs`. The other 21 lab modules
  (~14.5k LOC incl. `cli.rs`), the `discovery` CLI subcommand in `main.rs`, and the orphan
  root lab JSONs (`discoveries.json`, `param_flags.json`, `unified_flags.json`) were removed.
- **CE-era Rosetta table moved out of the app.** The 46,076-line `src/db/event_flags.rs`
  (in-memory byte_offset/bit_position + coords/name/category for 5,751 flags, **unused** by
  the app) extracted verbatim to `knowledge/reference/ce-era-event-flags-rosetta.json` and
  deleted. Placed under `knowledge/reference/` (with an `_epistemic` block + README), NOT the
  evidence catalog — that indexes out-of-repo raw evidence, and this is derived in-repo
  reference. Distinct from the live `src/db/event_flags_db.rs`, which stays.

### Key Findings
- The two same-named files were a trap: `db::event_flags` (46k lines, the CE Rosetta table)
  was dead, while the 782-line `db::event_flags_db` behind the "Event Flags DB" view is live.
- The `−337,375` lesson recurs: `event_flags.rs`'s offsets are a real in-memory convention
  (125-byte block model), not save-file positions — preserved as reference, never for reads.

### Validation
- Main-crate unit tests **116 → 51** — exactly the deleted lab's tests; nothing live lost.
- Green: main 51, regression 4 (+3 ignored), wasm 22 + anchor 4 + export-shape 4 + origin 11,
  including the ADR-0008 `export_shape_conformance` guard. `cargo build`/`clippy` clean
  (pre-existing style warnings only).

### Files Modified
- `src/discovery/` (deleted, 22 modules) → `inventory_verification.rs` moved to `src/db/`
- `src/db/event_flags.rs` (deleted) → `knowledge/reference/ce-era-event-flags-rosetta.json`
- `src/db/mod.rs`, `src/main.rs`: dropped `event_flags` / `discovery` modules + CLI subcommand
- `src/ui/{events,verification_view,world_pickups_view}.rs`: import paths → `db::inventory_verification`
- `src/db/*_data.rs`, `pickup_flags.rs`: stale generator headers repointed to the archive doc
- `scripts/**` (deleted except `windows/`); root `discoveries.json` / `param_flags.json` / `unified_flags.json` deleted
- `docs/archive/PYTHON-LAB.md`, `knowledge/reference/README.md`: new distillation records
- `docs/{BACKLOG,ARCHITECTURE,CASE-VERIFICATION-GUIDE,EF-DISCOVERY-VERIFICATION-CHAIN,SAVE_FILE_GROUND_TRUTH}.md`: step 5 done; "removal target" → "removed"
- `Cargo.toml`, `docs/CHANGELOG.md`: version 0.32.0

## v0.31.1 - Docs audit: epistemic headers on every doc (BACKLOG step 6)

### Docs
- **Epistemic header on all 14 `docs/*.md`** — a block at the top of each file stating,
  before the body, how far to trust it: one **Status** line (CURRENT / ERA-MIXED /
  SUPERSEDED / STABLE-METHODOLOGY / LIVING-RECORD) plus **Claims / Evidence / Methodology /
  Obsolete**. Motivated by era-mixed docs misleading recent sessions (the "tile base 337375
  is constant" guidance, the Margit/Godrick catalog note, the retracted elden-map advice).
- **New glossary term** — `CONTEXT.md` now defines *Epistemic Header* (status values + the
  four fields), so the docs point to one definition instead of re-explaining it.
- **Wrong content corrected/retired:**
  - `EVENT-FLAG-GEOGRAPHY.md`: marked the disproven "+per-area stride" base tables obsolete
    inline; named the tombstoned literals (43487 / 46862 / 50237); corrected the area
    18/19/20 mislabels (m20/m21 are DLC Belurat / Enir-Ilim, not Roundtable/Chapel/Stranded
    Graveyard); flagged the stale `event/*.emevd.js` path.
  - `DATA-SOURCES.md`: fixed the Slot 0 claim — Radahn is **not** defeated in the 2026-01-11
    backup (only Margit and Godrick).
  - `COMMIT-PROTOCOL.md`: removed a stray leading `–` that was breaking the H1.
  - `ARCHITECTURE.md`, `EF-DISCOVERY-VERIFICATION-CHAIN.md`: marked SUPERSEDED (they
    document the pre-reset Python lab, a migration step-5 deletion target).
  - `WASM-EVENT-FLAGS.md`: flagged that its "Flag Offset Resolution" section names
    `get_sub_block_bases` / `get_main_block_bases`, both deleted in ADR-0008.
- **`CLAUDE.md` shrunk 144 → 113 lines** — duplication of `CONTEXT.md` and the 47-line False
  Negative Protocol collapsed to pointers, but every behavior-changing guardrail kept inline
  as a terse rule; added `CONTEXT.md` + `docs/adr/` as top orientation pointers.

### Files Modified
- All 14 `docs/*.md`: epistemic headers + inline corrections
- `CONTEXT.md`: *Epistemic Header* glossary entry
- `CLAUDE.md`: shrunk to workflow rules + pointers
- `docs/BACKLOG.md`: step 6 marked done
- `Cargo.toml`, `docs/CHANGELOG.md`: version 0.31.1

## v0.31.0 - Remove the static-offset wasm exports (ADR-0008, Priority 1b)

### Breaking (wasm crate public API)
The last place a caller could still get a silently wrong flag bit was the wasm crate's
exported static-offset entry points. They took a `flag_id` and returned a fixed byte
offset — a shape the project abandoned once it established that every flag family floats
per save (each sits after an append-only list that grows as the character plays). There
is no correct static offset for them to return, so they were removed, not repaired.
**This breaks elden-map by decision** (ADR-0008): its next vendored build fails to find
the exports rather than silently reading wrong bits. No in-repo caller was affected.

Removed exports (7): `get_flag_offset`, `get_flag_offset_calibrated`, `is_flag_set`,
`is_flag_set_calibrated`, `calculate_dungeon_pickup_offset`,
`calculate_world_pickup_offset_by_row_id`, `calculate_tile_pickup_offset`, plus the
`get_dungeon_pickup_sections` / `get_tile_base_offset` / `get_world_pickup_row_id_base`
accessors.

Removed base tables (5): `get_dungeon_general_bases` (the disproven "+3375 per area"
stride table), `get_sub_block_bases`, `get_main_block_bases`, `get_midrange_bases`,
`get_dungeon_pickup_section_bases`. Constants `TILE_BASE_OFFSET` (337375, tombstoned —
it is the *distance between* two families, not a base) and `WORLD_PICKUP_ROW_ID_BASE`
(a storage model disproven 2026-02-16) are gone. The crate no longer imports `HashMap`.

### The boundary
An export may not source a flag's position from inside the crate. It must take the
save/flag bytes and resolve the family for that save, or take the base as a parameter.
This is why `calculate_tile_pickup_offset_calibrated(flag_id, tile_base)` survives while
`calculate_tile_pickup_offset(flag_id)` does not — identical arithmetic, but one is
handed the base and the other invents it. The survivor is load-bearing: `tile_read`
calls it with base 0 and adds a resolved family base, so it is tile geometry, not a
claim about where a family sits. Flag reads now go through the region-taking readers:
`is_world_state_flag_set`, `is_tile_world_flag_set`, `is_tile_pickup_set`,
`is_dungeon_flag_set`, `is_dungeon_pickup_set` — each returns Unknown rather than false
when a family cannot be resolved.

### Conformance guard
New `crates/wasm-event-flags/tests/export_shape_conformance.rs` pins that no exported
entry point reaches a crate-baked base — the check whose absence let the disproven table
survive four cutover commits while documented as wrong in five places. Its load-bearing
test is structural, not a name list: any export answering a flag position/state question
must receive `&[u8]` or an explicit base, so the model cannot regrow under a new name.
Each of the three checks was verified by mutation (mutation applied, observed to fail
the intended test, reverted).

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: removed 7 exports, 5 base tables, 2 constants;
  removed the tests that asserted literal byte offsets (their assertions were the
  abandoned model restated); preserved carried evidence in comments at removal sites
- `crates/wasm-event-flags/tests/export_shape_conformance.rs`: new (4 tests)
- `docs/adr/0008-*.md`: implementation note recording the scope widening past the 3
  named exports and the emergent boundary rule
- `docs/BACKLOG.md`: Priority 1b steps 1-3 marked done; step 4 (elden-map) framed as the
  open cross-repo work; the widened scope tabulated
- `docs/EVENT-FLAG-GEOGRAPHY.md`: marked the row_id code reference OBSOLETE
- `docs/SAVE_FILE_GROUND_TRUTH.md`: past-tensed the `get_dungeon_general_bases` reference
- `CLAUDE.md`: recorded that the crate holds no flag base tables by design

---

## v0.30.0 - Route pickups by family; the app leaves the frozen store entirely

### Features
- **`pickup_flags::pickup_flag_state`** — one router for every pickup table. Resolves
  per save and dispatches by id shape: 10-digit to the open-world tile family, 8-digit
  with localId >= 7000 to legacy-map pickups, 50000-79999 to world-state-b. `None` is
  UNKNOWN and is never collapsed to "not collected".
- **Seven read paths cut over**, five of them previously unnoticed: the events-view world
  pickup table and its detail panel, the dungeon-pickup detail panel, `collect_set_flags`
  over UNIQUE_ITEMS, `comparison_view`, `world_pickups_view`, and the **JSON export**.
  No app-layer reader is left on `ground_truth_offsets.json`.
- **Export now distinguishes unknown from not-collected.**
  `ExportWorldPickupItem.collected` became `Option<bool>` and serialises as `null` when
  the flag's position could not be resolved. **This is a JSON schema change** — consumers
  that assumed a boolean will see `null` for DLC tiles and unclassified ids.
- `comparison_view` skips pickups it cannot resolve in both saves rather than reporting
  them as differences; comparing two Unknowns manufactures diffs that say nothing about
  the characters.

### Key Findings
- **`WORLD_PICKUPS` is not a single-family table**, despite the name: of 4,809 entries,
  1,232 are open-world tiles, 2,010 legacy-map, 100 world-state-b, 935 unclassified
  six-digit ids and 532 DLC. v0.28.0 routed all of it through the tile reader, leaving
  3,577 Unknown. Routing by family recovered **2,060** — Unknown fell 3,577 -> 1,517,
  Confessor collected rose 495 -> 910, Wretch 2 -> 5. The first cut was not wrong (each
  family reader rejects foreign ids, so nothing read a wrong bit) but needlessly blind,
  and only the aggregate count exposed it.
- **The dungeon-pickup detail panel carried its own copy of the legacy
  `DUNGEON_PICKUP_BASES` arithmetic**, separate from the table's, so the panel and the
  row it was opened from could disagree about the same pickup. Both now share a resolver.
- **An anomaly, recorded rather than smoothed over.** V3, the true-negative control,
  went 0 -> 2 collected. One is `60210` "Tarnished's Wizened Finger", a starting item, so
  SET is correct and the old 0 was an artifact of it reading Unknown. The other is
  `10007452` "Crimson Hood", a Stormveil pickup V3 never reached, SET on all six slots.
  Not a mislabel (primary source: `ItemLotParam_map` row 10000451, `lotItemId01=740000`)
  and not a read artifact (V3 reads exactly 1 SET of ~1,960 readable legacy pickups; a
  bad base would smear hits, and m10_00 shows 75/250 non-zero for the Confessor against
  1/250 for minimal characters). The bit is genuinely set for everyone; why is
  unestablished, and settling it needs an attributed transition the corpus lacks.
  **"V3 has zero pickups" is no longer the right expectation for the control.**
- **`dungeon_pickups.rs` diverges from `ItemLotParam_map` about 8% each way** — 189 DB
  entries absent from the primary source, 152 primary entries absent from the DB,
  clustered in m41_00/01/02, m40_02 and m13_00. Regenerating it is its own data task with
  its own verification, deliberately not folded in here; the primary source is on this
  machine, so it is unblocked.

### Verification
- 189 -> 190 tests. New unit test pins the export contract: `collected` must serialise
  as `true` / `false` / `null` and never collapse UNKNOWN to `false`.
- Live counts still match the documented character designs, with the control's
  expectation restated (see the V3 note above).
- Clippy unchanged at its 882-warning baseline; no new warnings introduced.

### Files Modified
- `src/db/pickup_flags.rs`: `pickup_flag_state`; `is_flag_set_with_status` deprecated
- `src/ui/events.rs`: world pickups, both detail panels, `collect_set_flags`
- `src/ui/world_pickups_view.rs`, `src/ui/comparison/comparison_view.rs`: routed
- `src/vm/slot.rs`, `src/vm/export.rs`: export path; `collected` is now `Option<bool>`,
  plus a test pinning the null semantics
- `src/knowledge/dump.rs`: pickup counter uses the router
- `DATA-SCHEMAS.md`: world-pickup export item documented, with the nullability change
- `docs/BACKLOG.md`, `docs/DATABASE_COVERAGE_ANALYSIS.md`: cutover, anomaly, DB audit
- `Cargo.toml`: bumped to 0.30.0

---

## v0.29.2 - Settle the legacy family address overlap

Documentation and one doc-comment. No behaviour change; all 189 tests unchanged.

### Key Findings
- **The 125-byte separation between the legacy families is REAL, and the single-base
  alternative is refuted by bytes.** File b33 carries two event flags and three pickups
  all known set — one file, so no drift confound — and each reads set at its own family's
  base and clear at the other's. The clincher is the transition: the byte that flips for
  pickup 30027000 across b20->b21 is at the pickup base (`ef[1622973]`, `0x00 -> 0x80`),
  while the single-base prediction (`ef[1623098]`) stays `0x00` on both sides.
- **The overlap is harmless because its band is empty on the event side.** Legacy event
  flags cluster in localId 0-2999 and pickups in 7000-7999; 6000-6999 is used by neither.
  Zero hits across 4,540 distinct legacy flags from three independent sources, and the
  primary source agrees — `ItemLotParam_map` (regulation 1.16.1) carries 2,143 legacy
  `getItemFlagId`s in 7000-7999 and none in 6000-6999.
- **Two `dungeon_pickups.rs` discrepancies against the primary source**, neither
  affecting a shipped read: m15_00's seven `getItemFlagId`s at localId 1200-1290 are
  absent from the DB (coverage gap), and the DB's `12022995` / `12022997` are absent from
  the primary source (suspect provenance). Both incidental finds, so a full audit of
  `dungeon_pickups.rs` against `ItemLotParam_map` is now on the backlog.

### Fixes
- **Retracted the fix proposed in v0.29.1.** It suggested pickups should index from
  `localId - 1000` at a shared base. That expands to `(base_ev - 125) + slot*1125 + L/8`
  — the shipped formula, character for character. The two were never competing
  hypotheses and no experiment could have separated them.
- **Withdrew the v0.29.1 caution** to "suspect legacy event flags with localId 6000-6999".
  It implied a live risk; there are no such flags. What remains is a trigger to watch
  for — if one is ever found it collides with a real pickup — not a standing doubt.

### Files Modified
- `docs/BACKLOG.md`: open question marked settled, with the three findings and the
  algebra mistake recorded as the reusable part
- `docs/SAVE_FILE_GROUND_TRUTH.md`: overlap section rewritten as settled; DB
  discrepancies recorded
- `crates/wasm-event-flags/src/lib.rs`: resolution recorded at `is_dungeon_pickup_set`
- `Cargo.toml`: bumped to 0.29.2

---

## v0.29.1 - Record the DLC evidence block and a legacy family address overlap

Documentation and one doc-comment. No behaviour change; all 189 tests unchanged.

### Key Findings
- **The two legacy families' address ranges OVERLAP.** Both index by the raw
  `localId / 8` inside a map's 1125-byte block — events at bytes 0-874, pickups at
  875-1124 — which would tile the block exactly if their bases were equal. They are not:
  the pickup base sits 125 bytes lower, putting pickups at bytes 750-999 of the event
  block. The consequence is exact: **event localId L and pickup localId L + 1000 resolve
  to the same bit**. Verified pickup 30027000 shares its byte with a hypothetical event
  flag 30026000.
  Each (base, formula) pair is verified against its own flips, so reads of evidenced
  flags are correct — what is unpinned is the split between base and formula, most
  likely a missing `- 1000` on the pickup index. No evidence file exercises a colliding
  pair, which is why it had not surfaced. Legacy event flags with localId in 6000-6999
  are suspect until settled.
- **m34_12 / m40_00 double allocation: INCONCLUSIVE, recorded so it is not re-run blind.**
  In backup slot 0 the slot-62 block holds 6 non-zero bytes and slot 144 none, which is
  suggestive but not decisive: one map, one save, no attributed transition, and those
  bytes sit in the overlapping range above so they cannot even be assigned to a family.
  m40_00 is undecidable outright — both blocks are zero in every slot, i.e. no character
  has been there. Both maps continue to read Unknown.

### Fixes
- **Retracted a wrong recommendation.** v0.29.0 called the DLC layout "the single
  highest-value remaining discovery for coverage" in three documents. The DLC is not
  installed on this machine and no character has progressed into it, so there is no
  transition to attribute and no way to verify a hypothesised base; inferring one from
  the alloclists would be exactly the unverifiable claim ADR-0004's status ladder exists
  to exclude. All three now record the block, the unblock condition (DLC installed plus a
  character captured either side of a DLC pickup or boss kill), and a warning that the
  size of the Unknown count is not an argument for the work.

### Files Modified
- `docs/BACKLOG.md`: DLC block; the overlap as an open question with its settling test;
  the inconclusive alloc probe; actionable next work re-pointed at
  `is_flag_set_with_status`
- `docs/SAVE_FILE_GROUND_TRUTH.md`: overlap caveat on the legacy-map layout section
- `docs/DATABASE_COVERAGE_ANALYSIS.md`: DLC gap marked blocked on evidence
- `crates/wasm-event-flags/src/lib.rs`: overlap recorded at `is_dungeon_pickup_set`
- `Cargo.toml`: bumped to 0.29.1

---

## v0.29.0 - Legacy-dungeon cutover; boss_defeats leaves the frozen store

### Features
- **`FAMILY_LEGACY_DUNGEON` = 1,500,567** — the last family without an origin constant.
  Measured from the two attributed boss-kill pairs that pinned it (30020800 b24-b25,
  30030800 b32-b33), spread 0 across a drift step.
- **`er-save-editor knowledge family-constants`** (`src/knowledge/family_distances.rs`,
  emits `knowledge/claims/family-constants.json`). Derives each family's constant from
  the attributed flips that pinned its base:
  `constant = (grace_base + family_base_grace_rel) - (ga_end + list_end)`. It reproduces
  all four already-shipped constants exactly — a second derivation chain agreeing with
  the first — and reaches the two families `list-hunt` structurally cannot see.
- **Boss defeats cut over** (ADR-0006, migration step 4). `src/ui/database/bosses_view.rs`
  no longer reads `ground_truth_offsets.json`. Both families resolve per save and are
  routed explicitly: `is_tile_world_flag_set` for 10-digit open-world bosses,
  `is_dungeon_flag_set` for 8-digit legacy-map bosses.
- **Dungeon pickups cut over.** The table in `src/ui/events.rs` used `DUNGEON_PICKUP_BASES`
  plus each pickup's `dungeon_area`/`section` fields; it now calls `is_dungeon_pickup_set`
  and takes the position from the flag id alone, so those fields are display data only and
  can no longer disagree with the flag they label.
- **`LEGACY_ALLOC_SLOTS`** (99 maps) in the reference implementation, copied from the
  game's own eventflagalloclists, with a conformance test that re-reads the source file
  and fails on drift. Replaces the disproven "+3375 per area" stride table for these
  reads. Layout: `alloc_slot(map) * 1125 + localId / 8`.

### Key Findings
- **An evidence-catalog note was wrong, and the corrected read caught it.** The catalog
  said the 2026-01-11 backup slot 0 "predates the Margit/Godrick/Radahn kills - zeros at
  their flag bytes are TRUE negatives". Read at the resolved base, Margit and Godrick ARE
  defeated; only Radahn is not. The 2026-07-05 note measured zeros at the pre-migration
  offsets. Corroborated by the adjacent found_flags (10000801, 10000851), the m10_00
  block carrying 96 non-zero bytes of 1125 where slots 1 and 4 carry 5 and 0, and
  `DATA-SOURCES.md`'s own character description.
- **The Wretch names its own boss.** A character that contributed nothing to deriving the
  constant reports exactly one defeated boss - Soldier of Godrick - out of 205
  candidates, matching the catalog's wording verbatim.
- **m18's old disproof retired.** Stranded Graveyard was removed from the legacy base
  table as DISPROVEN (its span read all zeros) despite every character killing Soldier of
  Godrick in the tutorial. Via alloc slot 35 and a resolved origin it now reads correctly.
  The layout was right; only the base was wrong - the same lesson as 337,375.
- **The two legacy families sit 125 bytes apart**, not the "~129" in the pipeline's own
  family note. That number was a cross-file subtraction of bases measured at different
  list lengths; the list-end-relative distance is the invariant one.
- **Three boss ids were unreachable in the database** - a "12" prefix where the
  open-world tile prefix is "10", so they addressed tiles outside the m60 grid and could
  never read as defeated, Starscourge Radahn among them.

### Fixes
- `src/db/bosses_data.rs`: Radahn 1252380800 -> 1052380800 and Borealis 1254560800 ->
  1054560800, with cross-references updated in `entity_relationships_data.rs`,
  `shop_items.rs` and `merchants_data.rs`. Each row contradicted itself (its own `id` and
  `area_no: 60` gave the "10" form); the game's openmap alloclist allocates m60_52_38 and
  m60_54_56; and for Borealis the CE-era dump independently lists 1054560800.
  Night's Cavalry 1248550800 is deliberately NOT corrected - its `id` agrees with the
  "12" form and the CE dump lists it far from the tile region, so it reads Unknown.

### Verification
- `knowledge validate-origin` part C: 28 verified flags (up from 23) read clear->set
  through the SHIPPED functions, with zero families skipped for want of a read function.
- `origin_conformance.rs` extended to 11 tests: the legacy alloc table must still equal
  its source file, the two legacy families must stay 125 bytes apart, and each dungeon
  read must refuse the other family's ids and every open-world tile id.
- `knowledge run` re-run: claims byte-identical apart from the recorded catalog digest.
- Live counts match the documented character designs - Confessor 51 bosses / 382 dungeon
  pickups, Wretch 1 / 0, V1-V3 0 / 0.
- Recorded, not hand-waved: 29 of 205 bosses and 36 of 2,108 dungeon pickups read
  Unknown, itemised by cause in `docs/DATABASE_COVERAGE_ANALYSIS.md`. Most of that is
  DLC, which is blocked on evidence — the DLC is not installed here and no character has
  progressed into it, so there is no transition to attribute. Those flags stay Unknown.

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: FAMILY_LEGACY_DUNGEON, dungeon read functions,
  LEGACY_ALLOC_SLOTS, WASM exports
- `crates/wasm-event-flags/tests/origin_conformance.rs`: 3 new conformance tests
- `src/knowledge/family_distances.rs`: `family-constants` command; dungeon families in
  ORIGIN_CONSTANTS and part C
- `src/knowledge/dump.rs`: per-slot boss, demigod and dungeon-pickup counts
- `src/ui/database/bosses_view.rs`, `src/ui/events.rs`: cutovers
- `src/db/bosses_data.rs` + 3 cross-reference files: boss id corrections
- `knowledge/evidence-catalog.json`, `CLAUDE.md`: slot 0 note corrected
- `ground_truth_offsets.json`: cutover_state for boss_defeats and pickups
- `tests/regression_suite.rs`: freeze digest re-pinned
- `docs/`: BACKLOG, SAVE_FILE_GROUND_TRUTH, DATABASE_COVERAGE_ANALYSIS, CHANGELOG
- `Cargo.toml`: bumped to 0.29.0

---

## v0.28.0 - World pickup cutover; explicit tile family API; shared vocabulary

### Features
- **World pickups resolve per save** (ADR-0006, migration step 4). `is_tile_pickup_set`
  positions the `tile-pickup-row-id` family from the Origin; `src/ui/world_pickups_view.rs`
  no longer calibrates a tile base. Accepts either the ItemLotParam row_id (as stored in
  `pickup_data.rs`) or the getItemFlagId (row_id + 7000).
- **`FAMILY_TILE_OPEN_WORLD` = 454,067** established from two attributed boss-kill pairs
  with exact agreement, corroborated by the claims store's bases sitting 500 bytes apart.
  Thinner evidence than the other constants (two files, not dozens) and no UI consumer yet.
- **`knowledge validate-origin` part C**: every verified flag in the claims store must
  read clear->set through the SHIPPED read functions across its own attributed
  before/after pair. 23/23 pass. Hypothesis-status flags are excluded by the status
  ladder rather than asserted on (ADR-0004).
- **`knowledge grace-dump` reports pickups** alongside graces, per slot.

### Key Findings
- **A bare 10-digit tile id is AMBIGUOUS.** Open-world flags and pickup row_ids both use
  localId < 7000 and their regions sit 500 bytes apart, so nothing in the value
  distinguishes them. The first cut auto-routed on local id and sent 1,753 pickups to
  the open-world base — reading a plausible wrong bit rather than failing. Caught by a
  sanity count (11 collected for a mid-game character), NOT by a test. The API is now
  split and the caller must choose.
- `pickup_flags::is_flag_set_with_status` was deliberately NOT cut over: it takes a bare
  id and therefore cannot determine the family.
- Live-save counts match the documented character designs, including **V3 reading exactly
  0 pickups** — the character built as a true-negative control.
- 532 tile ids in `WORLD_PICKUPS` fall outside the open-world tile grid and read Unknown.
  Unexplained; likely DLC/underground tile numbering. Recorded, not hand-waved.

### Fixes
- `CLAUDE.md` Phase 4 of the False Negative Protocol instructed future work to believe
  two tombstoned claims ("tile base 337375 is constant across saves", "tile_base within
  EF is fixed"). Rewritten to resolve the Origin instead.

### Documentation
- **`CONTEXT.md` is the project glossary** and now defines the vocabulary this work
  introduced: Origin / List End, Family Constant, Resolver, Cutover, Unknown, and
  Row ID vs getItemFlagId. Also lists terms not to use, including one ("grace surface")
  that was invented mid-discussion and meant nothing.

### Files Modified
- crates/wasm-event-flags/src/lib.rs: is_tile_pickup_set / is_tile_world_flag_set,
  FAMILY_TILE_OPEN_WORLD, WASM exports
- src/ui/world_pickups_view.rs: cutover
- src/db/pickup_flags.rs: blanket tile routing reverted, with the reason recorded
- src/knowledge/family_distances.rs: part C transition test, status-ladder filter
- src/knowledge/dump.rs: pickup counts
- CONTEXT.md, CLAUDE.md, docs/SAVE_FILE_GROUND_TRUTH.md, docs/BACKLOG.md
- Cargo.toml: bumped to 0.28.0

---

## v0.27.0 - Grace name correction; layered dump tool

### Features
- **`knowledge grace-dump <save.sl2> [slot] [--all]`** (`src/knowledge/dump.rs`):
  prints what each layer believes about every grace in a slot — the raw byte and
  bit, the resolver's verdict (set / clear / UNKNOWN), and the name the database
  attaches to that flag. Built to compare layers against each other and against an
  exported JSON, without opening the GUI.

### Fixes
- **Grace names corrected against the game files.** The app carried TWO grace
  databases that disagreed on 49 flag names, and `docs/SAVE_FILE_GROUND_TRUTH.md`
  had 76100/76101 swapped outright. All three are now aligned to
  `BonfireWarpParam.param.xml` (regulation 1.16.1, the save era), whose rows carry
  `eventflagId` directly: 76100 = Church of Elleh, 76101 = The First Step,
  71800 = Cave of Knowledge, 71801 = Stranded Graveyard. 67 names fixed in
  `src/db/graces.rs`, 22 in `src/db/graces_data.rs`; both now match the game file
  exactly on all 382 shared flags. Includes the "Eerdtree" -> "Erdtree" typos.
  Caveat: `paramdexName` is a community annotation embedded by WitchyBND, not the
  game's own display text (which lives in FMG); it is the best available reference
  and `graces_data.rs` clearly derives from it, but it is not proof.

### Key Findings
- **Grace reading verified end-to-end against a real save.** The user's GUI export
  and an independent dump of the same live save agree exactly for all six
  characters — Confessor 186, Bee 182, Wretch 6, V1/V2/V3 2 each — and agree
  grace-for-grace by name, not merely by count.
- **A wrong NAME is indistinguishable from a wrong OFFSET** when reading a table of
  grace names. The swapped 76100/76101 documentation sent an investigation down the
  wrong path: the byte reads were correct the whole time.
- Empty character slots correctly report UNKNOWN rather than "0 discovered".

### Tests
- `the_two_grace_databases_agree_on_names` — the two databases must not drift apart
  again; fix against the game file, not by copying one list onto the other.
- `limgrave_starter_graces_are_not_swapped` — guards the specific pair that was
  documented backwards.

### Files Modified
- src/knowledge/dump.rs, src/knowledge/mod.rs: grace-dump subcommand
- src/db/graces.rs, src/db/graces_data.rs: names aligned to the game file; tests
- docs/SAVE_FILE_GROUND_TRUTH.md: anchor table names corrected
- docs/BACKLOG.md, docs/CHANGELOG.md, Cargo.toml: 0.27.0

---

## v0.26.0 - Grace family cutover: first family off the frozen legacy store

### Features
- **Graces resolve per save** (ADR-0006, migration step 4). `is_world_state_flag_set`
  in the reference implementation locates the world-state-b family from the flag
  region itself; `ground_truth_offsets.json` is no longer consulted for graces. Its
  71xxx/76xxx entries are marked SUPERSEDED in `metadata.frozen.cutover_state`, and
  the freeze digest was re-pinned in the same commit.
- **EF-relative resolver API**: `find_flag_list_end_in_ef`, `resolve_family_base_in_ef`,
  `is_world_state_flag_set`, plus WASM export `world_state_flag_state`. The app holds a
  struct-parsed flag region, not raw slot bytes, and the append-only list lives inside
  that region — so the same scan works anchored on it. Verified 62/62 against the
  pipeline's measurements across probe points EF+8,000 through EF+24,000.
- **Both grace read paths cut over**: the database view and the view-model status used
  by the events view, comparison and export. The region filter in `main.rs` moved too.

### Key Findings
- **The EF-anchored resolver is self-correcting** with respect to where the flag region
  is believed to start: every value is relative to the slice, so an error of N in the
  region start moves the located list end by -N and cancels on indexing.
- **Aggregate results match the evidence catalog's own character descriptions**:
  Confessor 179/421 graces, Wretch 6, V1/V2/V3 2 each, zero unknown.
- **The 6 outstanding origin-validation FAILs are resolved**: V1/V2/V3 have exactly two
  graces, and they are 71801 (Stranded Graveyard) and 76101 (The First Step). The
  tutorial-anchor expectation was wrong; the
  model was right. Corroborated by the independent total, not just consistent with it.

### Removed
- `check_progression_gate` and `get_calibrated_grace_status` (70 lines). Both existed to
  compensate for legacy absolute offsets — one overrode the byte with an inference about
  prerequisite bosses, the other re-derived a base for "unreliable" blocks. Against a
  correctly resolved position the progression gate can only produce false NEGATIVES,
  hiding graces the player has. `PROGRESSION_GATES` retained as documentation.

### Files Modified
- crates/wasm-event-flags/src/lib.rs: EF-relative resolver + world-state read
- crates/wasm-event-flags/tests/origin_conformance.rs: 8 tests (path agreement,
  unknown-not-false)
- src/ui/database/graces_view.rs, src/vm/events.rs, src/main.rs: cutover
- src/knowledge/family_distances.rs: EF-slice anchor test, shipped-path and aggregate
  verification in validate-origin
- ground_truth_offsets.json: cutover_state.graces
- tests/regression_suite.rs: freeze digest re-pinned for the cutover
- docs/BACKLOG.md, docs/CHANGELOG.md, Cargo.toml: 0.26.0

---

## v0.25.0 - Flag family origin: single-save resolution via the append-only list

### Features
- **Origin resolver in the reference implementation**
  (`crates/wasm-event-flags/src/lib.rs`): `find_flag_list_end`,
  `resolve_family_base`, the three family constants, and WASM exports
  (`flag_list_end`, `family_base`). A save with NO history can now position every
  flag family — `family_base = ga_items_end + flag_list_end + FAMILY_CONSTANT` —
  with no before/after pair, no scoring, and no scan of the flag region. This was
  the blocker on the step-4 per-family cutover (ADR-0006).
- **`knowledge family-distances`** — measures every family base in every evidence
  file and tests whether the distance BETWEEN families is constant.
- **`knowledge origin-probe`** — tests whether a single u32 record count explains
  the residual family drift (it does not; see Key Findings).
- **`knowledge list-hunt`** — differential alignment: locates the variable-length
  structures that move the families, by finding where two captures stop aligning.
- **`knowledge validate-origin`** — out-of-sample test of the origin model against
  characters it was not derived from.
- **`ground_truth_offsets.json` frozen read-only** (ADR-0006, migration step 4a):
  `metadata.frozen` block records authority, enforcement, the convention warning,
  per-family cutover state and the known-bad entry classes; enforced by
  `tests/regression_suite.rs::test_ground_truth_is_frozen`, which pins the file's
  sha256. Audit finding: nothing in the repo writes this file, so the freeze cost
  nothing to impose.

### Key Findings
- **The families are rigidly locked to each other.** Three inter-family distances,
  ZERO spread across 37 files: tile-pickup→world-state-b = -337,375,
  legacy-pickup→world-state-b = -1,383,250, legacy-pickup→tile-pickup = -1,045,875.
  They close the triangle exactly. Locating one family locates all of them.
- **-337,375 is the tombstoned constant.** Tombstone `tile-base-337375-grace-anchored`
  correctly retired 337,375 as a tile *base*, but the NUMBER was never wrong: it is
  the invariant distance between two families. The old ground truth measured a real
  structural invariant and pinned it to the wrong origin. Some other legacy constants
  may likewise be real distances wearing the wrong anchor.
- **What moves the families is an append-only u32 list** at ga_end+~65.7k
  (grace_rel ~29.2k) — the same structure the pipeline independently identified as
  "a u32-record LIST, not the flag bitmap" (the old "catacombs" 28-31k span). It
  grows monotonically with progression, one record per event, always +4 bytes.
- **The list has no length prefix** (the bytes before it are zeros), which is why the
  single-count search failed. Record counts track progression: Confessor 291,
  Wretch 112, V1 111, V2/V3 110.
- **Measuring from the list end removes the drift entirely** — 117,192 / 454,567 /
  1,500,442, spread 0. These reproduce the independently measured inter-family
  distances to the byte: two separate measurement chains agreeing.
- **Out-of-sample validation**: 9/9 on V1/V2/V3 with exact expected bit patterns
  INCLUDING the CLEAR bits (a mislocated base fails those first), plus 4/4 on the
  Wretch slot. Five distinct characters across two backup saves and snapshots-root.
- **The tutorial "100% reliable" anchors are not universal**: 71800 and 76100 read
  CLEAR on V1/V2/V3. A +/-4096 search finds no base setting all four, so this is real
  character state. `docs/SAVE_FILE_GROUND_TRUTH.md` corrected.
- **Negative results, recorded deliberately**: the golden EF offsets are scan outputs,
  not structural truth (their tie-break comment calls the plateaus "small shifted
  echoes"), so the first structural test was inconclusive by construction rather than
  refuted; and no single u32 count field explains the drift from either anchor across
  a 190k span.
- **Method traps worth remembering**: differential alignment silently lies inside zero
  runs, where EVERY shift matches — the sync window must require real bytes or the
  sparse bitmap reads as "shift 0" and insertions inside it vanish. And a list-end
  scan starting inside a zero gap terminates instantly, producing values that look
  constant (they were exactly `delta - probe_start`).

### Files Modified
- crates/wasm-event-flags/src/lib.rs: origin resolver, family constants, WASM exports
- crates/wasm-event-flags/tests/origin_conformance.rs: 6 conformance tests (new)
- src/knowledge/family_distances.rs: four subcommands, delegating to the reference
  implementation so pipeline and app cannot drift (new)
- src/knowledge/pipeline.rs: expose loading/measurement internals to the sibling module
- src/knowledge/mod.rs: CLI dispatch for the four new subcommands
- ground_truth_offsets.json: metadata.frozen block (read-only marker)
- tests/regression_suite.rs: test_ground_truth_is_frozen (sha256 pin)
- docs/SAVE_FILE_GROUND_TRUTH.md: Flag Family Origin section; corrected the
  "100% reliable" anchor claim
- docs/BACKLOG.md: step 4a/4b record including negatives and method bugs
- knowledge/claims/{family-distances,origin-probe,list-hunt,origin-validation}.json:
  generated evidence (new)

---

## v0.24.0 - S2/S7 attributed pairs; timeline replay audit (re-annotation rejected on evidence)

### Features
- **7 new attributed pairs** from the `snapshots-root` corpus's 2026-02-09 session
  added to `knowledge/inputs/attributed-transitions.json`, with per-pair
  corpus/save_slot overrides (slot 2 = "V1", slot 7 = a previously
  uncharacterized instrument character): 3 world pickups resolve for s2-V1
  (family base 482,907) and 4 for s7 (base 482,861 then 482,931 24 minutes
  later — see Key Findings). 27 pairs total, all re-verified deterministic.
- **`knowledge timeline <target>`** (new subcommand, `src/knowledge/timeline.rs`,
  `knowledge/inputs/timeline-targets.json`): replays a sparse-diff capture-agent
  timeline (`[u32 LE offset][old byte][new byte]` records, one file per capture)
  into an in-memory slot buffer in chronological order, verify-on-read against
  the evidence catalog, and reports the reference grace detector's confidence
  and offset drift across the whole chain. Emits
  `knowledge/claims/timeline-replay-audit.json` — replay/detection statistics
  only, no flag claims (see Key Findings for why).

### Key Findings (byte-verified)
- **Family bases can drift within a single session, not just between
  sessions**: s7's tile-pickup-row-id base measured 482,861 at 21:51 and
  482,931 at 22:15-22:21 on 2026-02-09, a ~70-byte shift inside one ~30-minute
  capture run. Harmless to the pipeline's candidate resolution because every
  cross-check reads an expectation flag's bit at the CANDIDATE's own implied
  base in that candidate's own `after` file, never a cached base from the
  pair that first resolved it.
- s7's four world-state-b pairs (progression 60220, graces 71800/76101) did
  NOT resolve (isolated-flip scans returned zero or many candidates) —
  consistent with the evidence catalog's own warning that this corpus has
  cross-session churn and an unresolved flag-byte interpretation for 71800;
  left unresolved rather than forced.
- **Timeline re-annotation attempted and rejected on evidence.** Replaying the
  "Bee" corpus (slot 5, 3,830 captures, 2026-02-14..2026-05-25) is
  self-consistent: 1,194,422,113 records, 0.68% old-value mismatch rate
  (matches the earlier 2026-07-05 audit), confident grace detection on
  2,735/3,830 entries. But naming which flags set when requires locating the
  world-state-b family base per entry with no attributed before/after pair to
  anchor a search window (unlike every other pipeline stage). A blind 4-bit
  tutorial-anchor scan (71800/71801/76100/76101) gave 2-3 candidates even
  inside a tight window around the established base cluster; adding a
  3-entry base-stability streak still produced 32,893 "events" naming only
  16,174 distinct flags, with some flags "transitioning" 0→1 up to 69 times —
  logically impossible for a monotonic bit, and decisive proof the resolved
  base was hopping between the real region and a coincidentally-matching one.
  Not shipped (would violate ADR-0004 / the False Negative Investigation
  Protocol's evidence discipline). Next viable design, documented but not
  attempted: cluster grace-aligned isolated flips (reusing the same ±16-byte
  neighborhood test already proven in `pipeline.rs`) across every consecutive
  pair in the whole chain, and locate the family base from where many
  independent flips agree, instead of re-deriving a base from a single state.

### Files Modified
- knowledge/inputs/attributed-transitions.json: 7 s2/s7 pairs added (27 total)
- knowledge/claims/event-flags.json: regenerated — 27 verified flags
- src/knowledge/timeline.rs: new — sparse-diff replay audit
- src/knowledge/mod.rs: `timeline` subcommand registered
- knowledge/inputs/timeline-targets.json: new — timeline corpus/slot targets
- knowledge/claims/timeline-replay-audit.json: new — replay/detection stats
- docs/SAVE_FILE_GROUND_TRUTH.md: intra-session base-drift amendment; 27-pair note
- docs/BACKLOG.md: s2/s7 pairs marked done; timeline re-annotation attempt documented
- docs/CHANGELOG.md: version 0.24.0
- Cargo.toml: bumped to 0.24.0

---

## v0.23.0 - Multi-slot differential: cross-slot flag verification

### Features
- **Multi-slot differential** (the third CONTEXT.md verification method) in the
  knowledge pipeline: a `multi_slot_differentials` input section verifies a flag
  across character slots with attributed different progression. An anchor pair pins
  the family base in the anchor slot; each other slot's base is located by matching
  its full expected bit pattern within ±64 bytes of the anchor base, requiring
  exactly one match (`run_multi_slot_differentials`, src/knowledge/pipeline.rs).
  Passing instruments add a `multi_slot_differential` method to the anchor claim
  and a full per-slot measurement section to the store.
- **Per-pair corpus/save_slot overrides** in the attributed-transitions input; all
  cross-check machinery (expectations, multi-file differential corroborators,
  known-set anchors, the universal-anchor tombstone grouping) is now scoped to the
  same corpus+slot — family bases float per save, so another character's captures
  must never cross-check yours. Loaded files are keyed corpus#slot#rel_path.

### Key Findings (byte-verified)
- **rowId 1044360310 verified across V1/V2/V3** using the purpose-built instrument
  files: the V3 no→yes anchor pair resolves at base 482,865 — the only base whose
  (no, rested, yes) states are (clear, clear, set); all other full-EF pattern
  matches are static constants. V2/V3 match at anchor+0, V1 at anchor+4.
- **Slots of ONE save file float independently** (tile-pickup base 482,865 for
  slots 3/4 vs 482,869 for slot 2 in the same file) — the ±4 record-list float,
  now observed across slots. Cross-slot checks must calibrate per slot.
- Pattern provenance is recorded per slot: the 300/320/330/340 CLEAR expectations
  rest on the 2026-02-09 s2 before-captures plus set-monotonicity (clear at a
  later capture implies clear earlier).
- The "V3 - no" instrument file is byte-identical (sha256) to "V1 - after picked
  up rowId_1044360310" — V1's SET state is directly attributed.
- Reward corroboration on the anchor pair: the treasure's content is Golden
  Rune [1] ×1 (the only inventory change in the window).

### Files Modified
- src/knowledge/pipeline.rs: multi-slot differential stage; per-pair corpus/slot;
  scoped cross-checks; composite file keys; evidence corpus/slot per pair
- knowledge/inputs/attributed-transitions.json: v3rest-v3yes anchor pair + the
  treasure-1044360310-v1-v2-v3 differential (25 pairs total)
- knowledge/claims/event-flags.json: regenerated — 21 verified flags,
  multi_slot_differentials section
- docs/SAVE_FILE_GROUND_TRUTH.md: per-slot independent float note
- docs/BACKLOG.md: multi-slot differential marked done in step 3
- docs/CHANGELOG.md: version 0.23.0
- Cargo.toml: bumped to 0.23.0

---

## v0.22.0 - Reward corroboration: inventory diffs by item identity

### Features
- **Reward corroboration (ADR-0007)** in the knowledge pipeline: every capture's
  inventory is parsed by ITEM IDENTITY — never GaItem handle, handles churn across
  captures. Weapon/armor/AoW handles resolve through the slot's `ga_items` table
  (weapon item_id keeps its upgrade level; armor `^0x10000000`; AoW `^0x80000000`);
  accessory (`^0xa0000000`) and goods (`^0xb0000000`) ids come from the handle's low
  28 bits. Held + storage-box inventories, common + key lists, are summed per
  identity. The per-pair gained/lost delta is recorded as evidence on every claim,
  and a matching gain on a pickup/kill pair adds an independent
  `reward_corroboration` method (`src/knowledge/pipeline.rs`).
- Display names from the in-repo name databases (labels only — claims rest on ids).

### Key Findings (byte-verified)
- **Every resolvable pickup/kill label corroborated by its exact item**: bosses —
  Erdtree Burial Watchdog → Noble Sorcerer Ashes, Bols → [Sorcery] Greatblade
  Phalanx, Crucible Knight → [Incantation] Aspects of the Crucible: Tail, the
  unnamed m30_03 boss → Glintstone Sorcerer Ashes (an identifying clue for that
  dungeon); pickups — all matched, including lot annotations 2200 (Prattling Pate
  "Hello") and 20775 (Root Resin ×2), the b15/b16 chest (Arrow's Reach Talisman),
  Golden Order Seal, Jellyfish Shield, Recusant Finger, Perfume Bottle.
- Honest negatives stay honest: grace pairs gained only flask refills (resting
  refills them); NPC-dialogue pairs gained nothing — consistent with their
  hypothesis status.

### Files Modified
- src/knowledge/pipeline.rs: inventory_identities / inventory_delta / identity_name;
  deltas wired into claim evidence and methods; module doc now 8 stages
- knowledge/claims/event-flags.json: regenerated with inventory evidence
- docs/BACKLOG.md: reward corroboration marked done in step 3
- docs/CHANGELOG.md: version 0.22.0
- Cargo.toml: bumped to 0.22.0

---

## v0.21.0 - Knowledge pipeline: claims store generated from attributed transitions

### Features
- **Knowledge pipeline (migration step 3)**: `er-save-editor knowledge run`
  (`src/knowledge/pipeline.rs`) regenerates `knowledge/claims/event-flags.json`
  deterministically (re-run ⇒ byte-identical) from the hand-written hypothesis input
  `knowledge/inputs/attributed-transitions.json` + the alloclist layout + the evidence
  catalog (ADR-0004: the store is pipeline-generated, never hand-edited). Stages:
  verify-on-read (sha256 vs manifest, "EVIDENCE DRIFT" abort) → grace-base detection
  (wasm reference impl) → grace-aligned isolated-flip extraction (±16-byte identical
  neighborhoods kill shift illusions) → iterative fail-soft candidate resolution
  (cross-check expectations built only from already-resolved pairs, so a wrong
  hypothesis cannot poison verified claims) → multi-file differential disambiguation →
  tombstone refutations recomputed from bytes each run (failing refutation aborts) →
  deterministic emission (sorted keys, no wall-clock dates, input sha256s embedded).
- **Attributed-transitions input**: 24 Confessor pairs across two sessions (numbered
  01-10 of 2025-12-29, ordering confirmed by file mtimes + a byte-identical
  session-boundary file; b-series of 2026-01-23..25).
- **New verification method — multi-file differential**: an ambiguous set-monotonic
  candidate whose implied family base is independently re-measured by a later resolved
  pair must stay SET in that pair's files; disambiguated the Golden Order Seal pickup
  (candidate at grace_rel 851,264 cleared in later files; 851,389 persisted with 9
  cross-checks).

### Key Findings (byte-verified)
- **REGION MAP REDRAWN** (grace-relative, per-save floating bases): world-state-b
  `(flag−50000)/8` @ ~146.6k; tile-open-world `slot×875+local/8` @ ~483.47k;
  tile-pickup-row-id (row_id = getItemFlagId−7000, same tile layout, SEPARATE region)
  @ ~483.97k; legacy-dungeon `alloclist_slot×1125+local/8` @ ~1,529.98k (NOT 4,112 —
  the 28-31k span is a u32-record LIST whose insertions cause the ±4 shifts);
  legacy-dungeon-pickup (separate region) @ ~1,529.85k. **Event flags and pickup
  tracking are separate regions per area type.**
- **20 flags Verified** (bosses 30020800, 30030800, 1042370800, 1033450800; graces
  73002, 71602, 76310; world flags 66700, 60260, 67640; dungeon pickups 30027000,
  30027030, 30037030; world pickups by row id ×7) + 4 honest hypotheses (62132 and
  NPC-talk 16000720/730/750 provably do NOT flip at their labeled positions); 5 family
  layouts Verified.
- **Open-world graces (76xxx) set BOTH world-state blocks** (copy A and copy B);
  dungeon graces set copy B only — the copy-A tombstone records the contrast.
- **Family bases float per session** on the same character (Dec tile-pickup base
  483,889 vs b-series 483,969) but are stable within a session.
- 4 tombstones: tile-337375 (struct-anchor-relative), legacy-at-4112, universal EF
  anchor, dungeon-graces-in-copy-A.
- Capture filename annotations can be wrong: b15/b16's `rowId-1042371300` was actually
  1042370300 (getItemFlagId−7000); the flip verifies the corrected id.

### Files Modified
- src/knowledge/pipeline.rs: NEW — the pipeline (7 stages)
- src/knowledge/mod.rs: `run` subcommand wiring
- src/knowledge/catalog.rs: sha256_file made pub(crate)
- knowledge/inputs/attributed-transitions.json: NEW — 24-pair hypothesis input
- knowledge/claims/event-flags.json: NEW — generated claims store
- docs/SAVE_FILE_GROUND_TRUTH.md: claims-store header block with region map
- docs/BACKLOG.md: step 3 core done; remaining work itemized
- CONTEXT.md: Flag Family regions, Record List, Attributed Transition, Multi-file
  Differential glossary entries
- CLAUDE.md: claims-store pointer (supersedes ground_truth_offsets.json for covered
  families)
- docs/CHANGELOG.md: version 0.21.0
- Cargo.toml: bumped to 0.21.0

---

## v0.20.0 - Evidence catalog; game corpus restored; alloclist primary source

### Features
- **Evidence catalog (migration step 2)**: `knowledge/evidence-catalog.json` — integrity
  index over all out-of-repo Evidence (8 corpora incl. saves, capture pairs, timeline
  diffs, raw game files, regulation XMLs) with hand-written trust context per corpus and
  machine-owned sha256/manifests. Per-file manifests under `knowledge/manifests/`
  (5,500+ files, ~12GB covered).
- **`knowledge` CLI family** (new `src/knowledge/`): `catalog-update` (fills machine
  fields, preserves hand context) and `catalog-verify` (recompute + compare, exit 1 on
  drift; drift detection tested with an injected stray file). Wired into `main.rs`
  alongside `discovery`.
- **Windows extraction script** `scripts/windows/regenerate-game-extracts.ps1`:
  WitchyBND chain (regulation.bin → .param → .param.xml, optional MSB→XML), chunked
  invocations, silent-mode fallback.

### Evidence Restored
- **Raw game files** (corpus `game-raw-1162`, 1,534 files): event/ (590 EMEVD + 4
  eventflagalloclists), regulation.bin, MSBs — copied from the Steam install (exe
  ProductVersion 2.6.2 ≈ 1.16.x). DCX(KRAK) decompression solved locally via ooz
  (strip to DCA payload + u64 LE size prefix).
- **Regulation param XMLs regenerated** (corpus `game-extracts`, flipped
  missing→directory, 390 files): WitchyBND on the Windows machine, regulation version
  11611000 = **1.16.1 — save-era match**; 194 paramdef-resolved params with Paramdex
  row names (ItemLotParam_map 5,564 rows, NpcParam 7,039, BonfireWarpParam 422, …).
  Restored to the canonical 'Elden Ring decompiled game files' path.

### Key Findings (primary source)
- **eventflagalloclists recovered and parsed** (`knowledge/game/eventflag-alloclists.json`,
  143 entries): plain CSV `slot,map_id,class`; legacy layout
  `base = REGION_BASE + slot×1125` reproduces the full legacy table INCLUDING the
  byte-verified m14 base (slot 23 → 29,987) and the previously removed m18 (slot 35 →
  43,487) / m19 (slot 38 → 46,862) — the LAYOUT was right all along; only the region's
  in-save position floats per save (per-family float).
- "Areas 20/21" belong to DLC maps m20 (Belurat) / m21 (Enir-Ilim) at DLC alloclist
  slots 150-156 — old "Stranded Graveyard"/"Haligtree" labels were misreadings.
- The decompiled-game-files corpus had been missing from this machine entirely;
  the restoration chain is now scripted and cataloged.

### Files Modified
- `src/knowledge/{mod,catalog}.rs`: NEW knowledge CLI family
- `src/main.rs`: knowledge dispatch; `Cargo.toml`: +sha2, bumped to 0.20.0
- `knowledge/`: NEW catalog, manifests, game/eventflag-alloclists.json
- `scripts/windows/regenerate-game-extracts.ps1`: NEW
- `CLAUDE.md`, `docs/DATA-SOURCES.md`: stale corpus pointers corrected, extraction
  levels documented
- `docs/SAVE_FILE_GROUND_TRUTH.md`: alloclist primary-source note; stale "~222K"
  claims in the body corrected
- `docs/BACKLOG.md`: step 2 done + corpus restoration updates

## v0.19.0 - ef-dump consumer API; delete Python EF detectors; deploy fixed wasm to elden-map

### Features
- **`discovery ef-dump` subcommand** (ADR-0005 consumer API): per-slot JSON with
  `ga_items_end`, `ef_offset` (grace-family base; per-family caveat embedded in output),
  scores, confidence, md5 integrity hashes. `--slot N` filter, `--bytes DIR` EF-section
  export, `--raw-slot` mode for bare slot bytes.
- **`scripts/verification/ef_dump.py`**: the single sanctioned Python bridge to the Rust
  reference implementation (subprocess; clear error if the binary is not built).

### Removals (ADR-0005: one reference implementation)
- Python content-search detectors DELETED: `SaveParser._find_event_flags_offset`,
  `utils.detect_event_flags_start`, `utils.detect_event_flags_start_robust` now delegate
  to `ef_dump.detect_ef_offset_bytes()`. All ~50k lines of lab scripts keep working
  through the one choke point. Verified: Python now returns the fixture-golden offsets
  on the backup slots (81,077 / 76,758 / 76,787 / 76,787 / 76,779, all 7/7) where the
  deleted search previously found the 106,808 lookalike.

### Cross-repo (elden-map)
- Rebuilt `wasm-event-flags` (wasm-pack, web target) and deployed to
  `elden-map/wasm-event-flags/` — the vendored binary was built 2026-04-07 in the
  poisoned-detection era. Node-verified against a conformance fixture (81,077,
  confident=true, corrected search_start). Remaining elden-map work (server/bundle
  rebuild, TS detector deletion, capture-agent ADR-0007 rework) tracked in BACKLOG
  Priority 0b follow-up 3.

### Files Modified
- `src/discovery/cli.rs`: NEW cmd_ef_dump (+ help text)
- `scripts/verification/ef_dump.py`: NEW bridge module
- `scripts/verification/save_parser.py`, `scripts/verification/utils.py`: detector
  bodies replaced with delegation
- `docs/BACKLOG.md`: Priority 0b follow-ups 2 done / 3 partially done
- `docs/CHANGELOG.md`: v0.19.0; `Cargo.toml`/`Cargo.lock`: bumped to 0.19.0

## v0.18.0 - Rework EF detection (anchor conformance); per-family float discovery

### Fixes
- **EF detection reworked** (`crates/wasm-event-flags`): primary is now a gaEnd-windowed
  grace-validation scan (`[gaEnd+30k, gaEnd+45k]`); the v0.16 "structural computation" is
  no longer used for detection — its section model overshoots the real flag region by
  ~146k bytes and its `confident: true` masked the error since ~Mar 2026. Proof: the
  b24/b25 kill-transition pair (Erdtree Burial Watchdog) — flag 30020800 flips in the low
  region; the struct-walk position stays zero in both files.
- **Honest confidence gating**: all-zero slots no longer report `confident: true`;
  negative-validation violations lower confidence instead of being ignored.
- **Legacy content-fallback `SEARCH_START` corrected 0x30000 → 0x12000** — it previously
  began PAST the real flag region, guaranteeing lookalike hits.
- **`save_slot.rs`**: hardcoded fallback 0x36500 (the ~222k lookalike) replaced with a
  gaEnd-derived fallback; the backwards "real EF is at ~222K, inventory at ~76K" comment
  corrected. GaItems-end parsing verified byte-exact (PlayerGameData name at gaEnd+148).
- Verified end-to-end: the "level 93 snapshot" — previously all-zero through every
  detector — now reads real flag data via the discovery probe.

### New: Anchor Conformance Fixtures (ADR-0003)
- `crates/wasm-event-flags/tests/fixtures/`: 8 real slot-data prefixes (128 KiB each,
  provenance-hashed) from the 2026-01-11 backup (slots 0-4), the level-93 snapshot, and
  the b24/b25 kill pair.
- `tests/anchor_conformance.rs`: golden detection results, in-window property (lookalike
  regions unreachable), tier-1 anchor bits at detected offsets, gaEnd churn tracking
  across the kill pair. 52 lib + 4 conformance tests pass.

### Key Findings
- **Per-family float**: flag families (graces, catacombs, …) sit at independently
  floating bases per save (Δ0 / Δ~77-141 / Δ~490 across measured saves) and regions
  shift by different amounts within one save pair (GaItems +16, flag region +4). No
  single per-save EF anchor exists; claims must carry their family. ADR-0003 amended;
  `CONTEXT.md` gains "Flag Family"; GT offsets are family+layout-specific.
- Bee-timeline Feb-2026-era anchors were correct (deltas 35,111+8k — 8-byte-stride
  variable section); the Mar-2026+ anchors inherited the structural bug.

### Decisions (ADR-0007, grilling session)
- **The capture agent records; the pipeline interprets**: capture-time interpretation
  abolished (bossesDefeated/inventoryDelta become re-runnable pipeline outputs); agent
  gains keyframes + per-entry state checksums + version stamps (rides the coordinated
  elden-map change). Reward Corroboration verification method defined (boss-unique items
  as independent kill evidence; inventory deltas by item identity — GaItem handles churn).

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: detection rework, SEARCH_START fix, window scan
- `crates/wasm-event-flags/tests/`: NEW conformance fixtures + tests
- `src/save/common/save_slot.rs`, `src/save/common/event_flags_detection.rs`: fallback +
  comment corrections, delegation docs
- `CONTEXT.md`, `docs/adr/0003` (amended), `docs/adr/0007` (new)
- `docs/BACKLOG.md`: Priority 0b partially resolved; pipeline/capture-flow additions
- `docs/SAVE_FILE_GROUND_TRUTH.md`: per-family float critical update
- `docs/CHANGELOG.md`: v0.18.0; `Cargo.toml`/`Cargo.lock`: bumped to 0.18.0

## v0.17.13 - Remove disproven m18/m19 dungeon bases; knowledge-base reset decisions

### Fixes
- **Removed disproven dungeon general bases** `(18,0)=43487` and `(19,0)=46862` from
  `get_dungeon_general_bases()` (crates/wasm-event-flags). Multi-slot differential: all five
  test-save slots AND the byte-validated Bee day-1 timeline state show zero bytes across those
  1125-byte spans, although every character necessarily sets Stranded Graveyard (m18) tutorial
  flags (Soldier of Godrick 18000850). Both entries came from an unverified "+3375 per area"
  stride assumption. Lookups for these areas now return invalid → consumers report "unknown"
  instead of a silent false "flag not set".
- **Fixed area comments**: m18 = Stranded Graveyard (tutorial), m19 = Elden Throne
  (Radagon/Elden Beast 19000800) — previously mislabeled Roundtable Hold / Chapel of Anticipation.

### Key Findings
- **(14,0)=29987 EMPIRICALLY VERIFIED**: Red Wolf of Radagon kill (14000850) landed at exactly
  29987+106 bit5 in timeline entry sd_000259, in the same byte-validated EF window as GT-proven
  30040800@32011 and 31020800@30984 (00→ff kill transitions).
- **EF anchor detection is systemically inconsistent** (BACKLOG Priority 0b):
  `compute_structural_ef_offset` overshoots by ~146k with `confident: true` (no fallback);
  the elden-map capture agent inherited this from ~Mar 2026 (anchor flicker artifacts in
  timeline metadata); python SaveParser lands on lookalike regions; `ground_truth_offsets.json`
  mixes anchor conventions across verification eras. This — not wrong bases — caused
  `batch-validate --context boss_defeat` to report 0/110 set on a mid-game character.
- Remaining m10-m22 general-base entries stem from the same stride assumption: marked
  UNVERIFIED in an in-code verification audit comment.

### Decisions (grilling session)
- New `CONTEXT.md` glossary (Evidence / Claim / Hypothesis / Provenance / Coordinate
  Convention / Claims Store / Status Ladder / Tombstone …) and `docs/adr/0001`–`0006`:
  evidence = raw bytes only; reset the knowledge base, not the code (no upstream re-clone);
  conformance fixtures define the coordinate convention; pipeline-generated claims store with
  status ladder; single reference implementation in crates/wasm-event-flags; frozen legacy
  store with per-family cutover. Migration plan in `docs/BACKLOG.md` Priority 0.

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: removed (18,0)/(19,0) bases, verification audit comments
- `CONTEXT.md`: new domain glossary
- `docs/adr/0001`–`0006`: architecture decision records
- `docs/BACKLOG.md`: Priority 0 migration plan + Priority 0b anchor bug; dungeon-area updates
- `docs/CHANGELOG.md`: v0.17.13
- `Cargo.toml` / `Cargo.lock`: bumped to 0.17.13

## v0.17.12 - Resolve numeric flag names and add event_action property

### Improvements
- **Numeric flag name resolution** — Reduced flags with numeric-only identifiers from ~1,448 to 764 (684 flags improved):
  - **Gesture names**: `load_gesture_names()` from GestureParam.param.xml (51 gestures). "Gesture Unlock (gesture 102)" → "Gesture Unlock (Rapture)"
  - **Entity names for templates**: `entity_names` dict passed to `extract_emevd_templates()`. "Enemy Defeat (10000280)" → "Grafted Scion - Enemy Defeat" (~216 flags)
  - **Region labels**: Door/Mechanism/Treasure flags use region as fallback. "Door Unlock (10000510)" → "Door Unlock (Stormveil Castle)" (~204 flags)
  - **Context verb entity extraction**: Character State, Spawn State, Item Award contexts now extract entity IDs from EMEVD verbs (DisableCharacter, EnableCharacter, etc.) and resolve to names (~100+ flags)
  - **MSB Region entities**: `load_msb_region_names()` parses 4,280 Region/Other XMLs with Japanese→English keyword translation for area_trigger/interaction names

- **`event_action` property** — New `raw_data["event_action"]` field on all EMEVD Literal Flags classifies the immediate EMEVD verb nearest to the SetEventFlagID call. 18 action types (boss_defeated, enemy_killed, gesture_acquired, item_acquired, cutscene_watched, door_opened, etc.). 937 of 1,360 EMEVD Literal Flags classified.

### New Functions
- `load_gesture_names()`: GestureParam.param.xml → gesture ID→name mapping
- `load_msb_region_names()`: MSB Region/Other XMLs → region entity ID→translated label mapping
- `JAPANESE_REGION_KEYWORDS`: 32-entry translation table for MSB region name keywords

### Call Site Changes
- `build_entity_name_map()` moved before `extract_emevd_templates()` in `main()` (was after)
- `extract_emevd_templates()` now accepts `entity_names` and `gesture_names` parameters
- `resolve_emevd_literal_names()` now accepts `gesture_names` and `region_entities` parameters

### Files Modified
- `scripts/extract_event_flags.py`: All changes (new loaders, enriched name resolution, event_action property)
- `scripts/extracted_event_flags.json`: Regenerated with improved names and event_action
- `scripts/extracted_event_flags.md`: Regenerated
- `docs/BACKLOG.md`: Updated Gesture Database status
- `docs/CHANGELOG.md`: v0.17.12
- `Cargo.toml`: bumped to 0.17.12

## v0.17.11 - Fix extraction categorizer priority and wrong hardcoded names

### Fixes
- **Categorizer priority bug** — Source-based checks (`ShopLineupParam.release → "Shop Unlock"`) now run before 91xx-95xx ID-range checks. Previously, ~20 Enia shop unlock flags (9101, 9104, 9107, etc.) were misclassified as "Remembrance" because the overbroad `9100-9199` range check ran first.
- **"Talisman Pouch" → "Boss Reward"** — The 9200-9299 range contains dungeon boss reward triggers (Cemetery Shade, Erdtree Burial Watchdog, etc.), not talisman pouches. Only 3 of ~60 flags in this range are actual talisman pouches. Renamed category throughout.
- **Removed wrong hardcoded entries** from `extract_common_emevd()`:
  - Remembrance (9100-9114): 6 of 15 names were wrong. These flags are now correctly sourced from ItemLotParam and ShopLineupParam.
  - Talisman Pouch (9200-9202): Now sourced from EMEVD boss trace resolution.
  - Mending Rune (9500-9502): 9500 was hardcoded as "Fell Curse" but is actually "Perfect Order" per ItemLotParam. 9504 was missing entirely.
- **Great Rune milestone flags** (160-167, 180-187) — Renamed from per-rune names ("Godrick's Great Rune - Possessed") to threshold milestone names ("Boss Drop Milestone: N+ Remembrances Collected"). These use `CountEventFlags >= threshold` where threshold=0 is always true, so flags 160/180 are set for ALL characters regardless of progression.

### Key Findings
- The 91xx range is a MIX of boss reward triggers (from EMEVD Event 1100) and Enia shop unlock flags (from ShopLineupParam). Sequential hardcoding was fundamentally wrong.
- EMEVD Events 720/730 use `CountEventFlags(range) >= threshold` — threshold=0 means the flag is always set, making flags 160 and 180 default-true for every character.

### Files Modified
- `scripts/extract_event_flags.py`: Fixed categorizer priority, renamed "Talisman Pouch" → "Boss Reward", removed hardcoded entries, fixed milestone names
- `scripts/extracted_event_flags.json`: Regenerated with corrected categories and names
- `scripts/extracted_event_flags.md`: Regenerated
- `docs/CHANGELOG.md`: v0.17.11
- `Cargo.toml`: bumped to 0.17.11

## v0.17.10 - EMEVD event context name resolution

### Extraction: EMEVD Name Resolution
- **New post-processing step** — `resolve_emevd_literal_names()` traces EMEVD event chains to resolve cryptic "Map Event Flag (N)" names to descriptive labels.
- **1,147 of 1,449 flags resolved** (79% coverage):
  - **Boss Reward (55/59)**: Traced `HandleBossDefeatAndDisplayBanner` → boss name lookup. e.g. `Map Event Flag (9206)` → `Boss Reward (Spiritcaller Snail)` _(category renamed from "Talisman Pouch" in v0.17.11)_
  - **Remembrance (17/17)**: Same boss-trace technique. e.g. `Map Event Flag (9163)` → `Remembrance (Bayle the Dread)`
  - **Progression (9/9)**: Context-dependent — boss defeats, gesture unlocks
  - **Mausoleum Duplication (4/4)**: Named by dungeon location
  - **EMEVD Literal Flags (1,066/1,360)**: Classified by surrounding code context into 9 types: Boss Defeat, Enemy Defeat, Cutscene Trigger, Gesture Unlock, Network State, Character State, Spawn State, Item Award, Door State
- **Boss/enemy name enrichment** — Boss Defeat and Enemy Defeat flags include the actual boss/enemy name when the entity ID exists in the database. e.g. `Boss Defeat Flag (30030800)` → `Boss Defeat (Spiritcaller Snail)`
- **Cutscene/gesture specifics** — Cutscene flags include cutscene ID, gesture flags include gesture ID

### Files Modified
- `scripts/extract_event_flags.py`: Added `resolve_emevd_literal_names()` (~170 lines), called in `main()` post-processing
- `scripts/extracted_event_flags.json`: Regenerated with resolved names
- `scripts/extracted_event_flags.md`: Regenerated

## v0.17.9 - Dungeon grace resolution and corroboration cleanup

### WASM: Dungeon Grace Resolution
- **Sub-block/main-block split** — Replaced single `get_block_bases()` HashMap with `get_sub_block_bases()` (100-granularity, checked first) and `get_main_block_bases()` (1000-granularity, fallback). This allows key `71000` to map to base `9315` for Stormveil graces (71000-71099) AND base `2625` for dungeon graces (71100-71799).
- **~80 dungeon graces unlocked** — Flags 71100-71799 (Leyndell, Underground, Farum Azula, Raya Lucaria, Haligtree, Volcano Manor, Fractured Marika) now resolve correctly via `calculate_simple_flag_offset()`.
- **6 new unit tests** — Stormveil sub-block routing, main-block fallback, tutorial grace, world grace regression, Leyndell grace, no-conflict validation. All 51 WASM tests pass.

### Corroboration: False-Alarm Cleanup
- **`skip_corroboration` field** — Added to `FlagRelationship` and `RawEdge` structs (`#[serde(default)]`), honored in both `check_corroboration()` loop and `corroboration_pairs` construction.
- **16 edges marked** — All are `pickup_sets_flag` edges where the tile-side flag is the row_id position (never written by the game). Includes 11 map fragments, 2 Memory Stones, Whetstone Knife, Flask of Wondrous Physick, Golden Tailoring Tools.
- **Extraction script updated** — `SKIP_CORROBORATION_PAIRS` set ensures regeneration preserves the field.

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: sub-block/main-block split + 6 tests
- `src/discovery/relationship_graph.rs`: skip_corroboration field + guard
- `src/discovery/corroboration.rs`: skip_corroboration early continue
- `scripts/flag_relationships.json`: 16 edges marked (32 total across both sections)
- `scripts/extract_flag_relationships.py`: SKIP_CORROBORATION_PAIRS for regeneration
- `docs/EVENT-FLAG-GEOGRAPHY.md`: dungeon grace block ranges + routing explanation
- `docs/WASM-EVENT-FLAGS.md`: flag offset resolution section
- `docs/BACKLOG.md`: blocks 71000/71100 resolved

---

## v0.17.8 - EF geography: simple flags, item acquisition tables, structural chain evidence

### Ground Truth Expansion
- **Simple flags (flag_id < 60,000)** — ML clustering on 799 timeline diffs identified 132 active byte offsets in EF+1040-1259 (flag IDs 8320-10079). Cross-referenced with EMEVD/param data to document 133 known flags across 5 categories: Remembrance (56), Talisman Pouch (63), EMEVD/Shop (9), Mending Rune (4), Unknown (2).
- **EF layout map** — New `ef_layout_map` section in ground_truth_offsets.json documents non-bitfield regions within the EventFlags array (item acquisition tables, structured data zones).

### Event Flag Geography Documentation
- **Simple flag formula**: `byte_offset = flag_id / 8, bit = 7 - (flag_id % 8)` for flags < 60,000
- **Item acquisition tables**: Two sorted 8-byte record tables within the EF array (EF+2208 and EF+32640) tracking items the character has ever obtained, using category prefixes (0x00=Weapon, 0x10=Armor, 0x20=Accessory, 0x40=Goods, 0x80=Custom).
- **MOEG/FOEG system**: Documented the dense state tracking region following EventFlags.

### Registry Updates
- `system.event_flags_raw`: Added evidence for structural section chain verification (GaItems→EF validated at 0x36CB5 for Bee slot 5) and browser WASM initialization fix.

## v0.17.7 - PlayerGameData unknown byte discoveries

### Save Format Discoveries
- **Flask Allocation (`_0x1a[3:5]`)** — identified with HIGH confidence (0.95)
  - `_0x1a[3]` = Crimson Flask charges, `_0x1a[4]` = Cerulean Flask charges
  - Verified via multi-save differential (5 slots × 2 saves) + Bee timeline (689 snapshots, 6 flask transitions)
  - Golden Seeds collected as inventory items; allocation only updates at grace rest (confirmed 11/11 pickups)
  - Monotonically non-decreasing across entire timeline; constant-sum invariant within periods
- **Flask Upgrade Data (`_0x1e[1]`)** — identified with MEDIUM-HIGH confidence (0.8)
  - byte[1] = Sacred Tear count (0→1 after applying 1 Sacred Tear at grace)
  - Consistent with Confessor's byte[1]=7 (7 Sacred Tears collected)
- **Defense Ratings (`_0x28`)** — 7 equipment+level dependent u32 values
  - Naked L1 Wretch: uniform 90 base; armored L1: 140-200 range; scales with level (+1-4 per level)

### Registry Updates
- Moved `flask_configuration` and `flask_charges` from `unknown` → `character_identity` group
- New features: `character_identity.flask_allocation` (0.95), `character_identity.flask_upgrade_data` (0.80)
- Coverage: 51 verified, 15 partial, 13 identified_unparsed (was 11), 10 unknown (was 12)

### Verification Method
- Multi-save differential: 5 character slots across backup (Jan 11) and latest save files
- Temporal timeline: Bee (slot 5) — 799 captures, 689 valid reconstructed snapshots
- Reverse diff reconstruction from latest save state through sparse binary diffs

### Files Modified
- `save_slot_registry.json`: flask features relocated, evidence added, confidence updated
- `docs/CHANGELOG.md`: v0.17.7 entry
- `Cargo.toml`: bumped to 0.17.7

---

## v0.17.6 - Save slot feature registry

### Documentation
- **Created `save_slot_registry.json`** — central registry of all 89 features stored in a character save slot, organized into 8 groups (character_identity, equipment, inventory, unlocks_progression, world_state, network, system, unknown)
- Coverage: 51 verified, 15 partial, 11 identified_unparsed, 12 unknown
- Each feature has stable dot-notation IDs (e.g., `character_identity.level`, `unlocks.graces_overworld`) for cross-referencing
- References `ground_truth_offsets.json` via pointers — no duplication, no code consumer changes
- Integrated registry maintenance into discovery workflow (`docs/discovery-verification-cycle.md` Phase 7 + Verification Checklist)
- Added registry to commit protocol decision table (`docs/COMMIT-PROTOCOL.md`)
- Added documentation table entry in `CLAUDE.md`

### Files Modified
- `save_slot_registry.json`: new central registry (89 features across 8 groups)
- `docs/discovery-verification-cycle.md`: registry update steps in Phase 7 + Prerequisites + Verification Checklist
- `docs/COMMIT-PROTOCOL.md`: Registry column in decision table + documentation triggers
- `CLAUDE.md`: documentation table reference
- `Cargo.toml`: bumped to 0.17.6

---

## v0.17.5 - Regenerate merged POI database with AEG pickups

### Database
- **Regenerated `merged-pois.json`** with 20,456 game POIs (up from 4,563), incorporating 15,893 AEG pickups
- **23,278 total merged locations** (was ~7,407): 2,764 merged + 17,671 game-only + 2,843 MapGenie-only
- Match breakdown: 1,944 by event flag, 596 by title, 224 by coordinate, 12 enriched with event flags
- 289 POIs now carry linked flags from causal graph

### Key Findings
- Previous merge was run on Feb 17 before AEG pickups were added on Feb 18, causing all AEG pickup POIs to appear as unmatched MapGenie-only entries
- Re-running confirms merge logic correctly handles AEG pickups via title+coordinate matching (e.g., Miquella's Lily matched at distance 0.001416, well within 0.008 threshold)

### Files Modified
- `elden-map/public/data/merged-pois.json`: regenerated (23,278 locations)
- `elden-map/server/data/game-pois/merge-report.json`: regenerated
- `elden-map/server/data/flag-correlation-candidates.jsonl`: regenerated
- `docs/CHANGELOG.md`: v0.17.5
- `Cargo.toml`: bumped to 0.17.5

---

## v0.17.4 - Raw Data pane for MapGenie-only POIs

### Features
- **Raw Data JSON pane** added to MapGenie-only POI detail panel on `/character-game-data`
  - Displays the full original `MapLocation` object (latitude, longitude, region, image, poiSource, etc.) that was previously discarded during POI construction
  - Copy button with same gold/teal feedback styling as the flag detail panel
  - Scrollable `<pre>` with 10px monospace font matching existing Raw Data pane

### Implementation
- Extended `MapGeniePOI` interface with optional `_sourceLocation` field to carry the full original `MapLocation`
- `mapGenieOnlyPois` builder now preserves the source `MapLocation` via `_sourceLocation`
- MapGenie-only panel looks up the original `MapGeniePOI` by ID to resolve source data, since the map component converts `MapGeniePOI` → synthetic `POI` for click callbacks

### Files Modified
- `elden-map/src/components/character-viewer/CharacterViewerMap.tsx`: extend `MapGeniePOI` interface
- `elden-map/src/pages/CharacterGameDataPage.tsx`: pass source data + add Raw Data pane
- `docs/CHANGELOG.md`: v0.17.4
- `Cargo.toml`: bumped to 0.17.4

---

## v0.17.3 - AEG pickup extraction with renewability metadata

### Database
- **15,893 AEG (AssetEnvironmentGeometry) pickups extracted** from MSB Part/Asset files, up from 0
  - 14,525 renewable (respawning on grace rest): Rowa Fruit, Erdleaf Flower, Mushroom, etc.
  - 1,368 one-time harvest (permanently consumed): Smithing Stones, Gloveworts, Trina's/Miquella's Lily
- **Behavior classification**: each AEG pickup tagged with `aeg_behavior` (bush/breakable/one_time_harvest) and `renewable` boolean
- **Item naming fix**: same-item quantity variants no longer produce "Rowa Fruit (+3 more)" — shows just "Rowa Fruit" when all slots share the same name

### Implementation
- `load_aeg_param()`: new function parses AssetEnvironmentGeometryParam, classifying pickups by `isEnableRepick`, `isBreakOnPickUp`, `isHiddenOnRepick` flag combinations
- `extract_aeg_pickups()`: scans MSB Part/Asset dirs for AEG099_* models, resolves items via ItemLotParam_map, generates synthetic flag IDs (`3B + area*10M + gridX*100K + gridZ*1K + instance`)
- Deduplication: AEG pickups whose item lot already appears in Treasure event flags are skipped to avoid double-counting

### Key Findings
- `isEnableRepick=1` means NON-respawning (one-time harvest) — the repick mechanism tracks picked state persistently, `isHiddenOnRepick=1` hides the model permanently
- `isEnableRepick=0` means RESPAWNING — no persistent state tracking, resets on grace rest
- `isHiddenOnRepick` always equals `isEnableRepick` across all 324 param rows

### Files Modified
- `scripts/extract_event_flags.py`: add `load_aeg_param()` and `extract_aeg_pickups()` functions
- `scripts/extracted_event_flags.json`: regenerated (4,563 → 20,456 positioned flags)
- `scripts/extracted_event_flags.md`: regenerated

## v0.17.2 - MSB enemy position resolution for item drops

### Features
- **EMEVD→ItemLotParam position backfill**: 158 flags that previously lacked coordinates now inherit positions from their drop source enemy's MSB entity data
- **item_lot_positions map**: `extract_emevd_templates()` now collects a mapping of item_lot_id → position data during template processing, resolving 173 unique item lot positions
- **Relationship graph extension**: new `enemy_drops_item` edge type in `extract_flag_relationships.py` linking defeat flags to item acquisition flags (245 relationships)

### Implementation
- Restructured EMEVD template loop to resolve entity data BEFORE dedup check, ensuring positions are captured even for deduplicated flags
- Post-processing pass matches positionless ItemLotParam flags by `source_row_id` against the item_lot_positions map
- Backfilled flags receive full spatial enrichment: local coords, world coords, map tile, region, area type, DLC classification
- Provenance tracked via `position_source: "EMEVD_Enemy"`, `enemy_entity_id`, `enemy_model`, `source_emevd`

### Impact
- Spatial coverage: local coords 4,405→4,563 (49%→50%), new `enemy_drop` treasure type (158 flags)
- Categories resolved: Ash of War Drop (129), Spirit Ash Drop (59), Boss Drop (56), Crystal Tear DLC (8)
- Flag relationship graph: 2,796→3,041 total relationships (+245 enemy_drops_item)

### Files Modified
- `scripts/extract_event_flags.py`: restructure `extract_emevd_templates()` return type and flow; add backfill post-processing in `main()`
- `scripts/extract_flag_relationships.py`: add `extract_emevd_enemy_item_relationships()` function and wire into `main()`
- `scripts/extracted_event_flags.json`: regenerated with backfilled positions
- `scripts/extracted_event_flags.md`: regenerated
- `scripts/flag_relationships.json`: regenerated with enemy_drops_item edges

## v0.17.1 - Classify Unknown flags by acquisition method

### Database
- **7 new extraction categories** for 582 previously "Unknown" ItemLotParam flags:
  - Quest Reward (238): NPC quest items, bell bearings, event rewards (400K block)
  - Ash of War Drop (129): boss/quest ashes of war (540K block)
  - Spirit Ash Drop (59): spirit ash summons from events (520K block)
  - Boss Drop (56): boss weapon/item drops (530K block)
  - Boss Reward (49): remembrances and boss rewards (510K block)
  - Tutorial (30): info/tutorial popup items (550K block)
  - Painting (21): collectible paintings (580K block)
- Only 11 flags remain as "Unknown" (misc edge cases)

### elden-map
- Registered 7 new category colors and filter group assignments
- Added `inferMarkerType()` mappings for new categories

### Files Modified
- `scripts/extract_event_flags.py`: block-range rules in `categorize_flag()`
- `scripts/extracted_event_flags.json`: regenerated
- `scripts/extracted_event_flags.md`: regenerated

### elden-map Files Modified
- `src/types/eventFlag.ts`: 7 category colors, group assignments
- `src/utils/categoryMapping.ts`: inferMarkerType mappings

## v0.17.0 - Extractor enrichment & elden-map schema alignment

### Features
- **Structured items array** — `extract_item_lot_param()` now builds an `items` list from all 8
  ItemLot slots with `{id, category, category_name, name, quantity}` per entry (4,382 flags).
- **Boss enrichment** — `extract_game_area_param()` populates `boss_type`, `boss_location`, and
  `rune_reward` on Boss Arena/Discovery flags (61 flags).
- **Shop enrichment** — `extract_shop_lineup_param()` parses merchant name from `[brackets]` in
  paramdexName and populates `shop_flag_type`, `merchant`, `shop_item_name`, `equip_type`, `price`,
  `sell_quantity`.
- **Dungeon type derivation** — Post-processing pass assigns `dungeon_type` from `area_no` via
  `DUNGEON_TYPE_MAP` (11 types: catacombs, cave, tunnel, hero_grave, legacy_dungeon, etc.).
- **Spirit ash detection** — `load_item_rarities()` now detects `goodsType=8` items from
  EquipParamGoods and sets `spirit_ash_name` on matching ItemLot flags.
- **Chest indicator** — `in_chest` field derived from `treasure_type == 'chest'` (201 flags).
- **10 new EMEVD categories** — Door Unlock, Mechanism Unlock, EMEVD Treasure, Gesture Unlock,
  Quest Completion, Quest State, NPC Death Quest, NPC Defeat, Map Point Discovery, EMEVD Literal
  Flag registered with colors and category groups in elden-map.
- **Adapter wiring** — `worldX`, `worldZ`, `areaType`, `isOverworld` fields added to
  `GameFileEventFlag` and wired through `adaptExtractedFlag()`.
- **DRY coordinate transforms** — Extracted `SCALE_X/Z`, `OFFSET_X/Z` to
  `src/utils/coordConstants.ts`; replaced hardcoded constants in 6 files across scripts and
  components.

### Files Modified
- `scripts/extract_event_flags.py`: 12 new EventFlag fields, items array builder, boss/shop/dungeon
  enrichment, spirit ash detection, `get_dungeon_type()` helper
- `scripts/extracted_event_flags.json`: regenerated with enrichment data
- `scripts/extracted_event_flags.md`: regenerated

### elden-map Files Modified
- `src/types/eventFlag.ts`: 4 new GameFileEventFlag fields, 10 category colors, 2 new groups
- `src/services/data/eventFlagAdapter.ts`: world coords and area classification wiring
- `src/utils/coordConstants.ts`: new single source of truth for transform constants
- `src/utils/measurementUtils.ts`: imports from coordConstants
- `scripts/build-game-pois.ts`: uses coordConstants import
- `scripts/merge-poi-databases.ts`: uses coordConstants import
- `scripts/build-event-flag-mappings.ts`: uses coordConstants import
- `src/pages/GameMapPage.tsx`: uses coordConstants import
- `src/components/game-map/GameMap.tsx`: uses coordConstants import
- `src/components/measurement/SnappingStatsPanel.tsx`: uses coordConstants import

## v0.16.5 - Stats and Equipment views use shared components

### Refactor
- **Stats view** — Replaced monospace `display_stat_row` rendering with `ExportToolbar` + `UnifiedTable`.
  Sortable Stat/Value columns, export to JSON/CSV/Markdown, double-click row copy.
- **Equipment view** — Replaced monospace `display_equipment_row` rendering with `FilterBar` (search) +
  `ExportToolbar` + `UnifiedTable`. Five sortable columns (Category, Slot, Item Name, Item ID, GA Handle),
  empty/unarmed slots shown in dark gray, fuzzy search across all 30 equipment slots, export support.
- Added `table_state`, `export_format` to `StatsViewModel` and `table_state`, `filter_state`,
  `export_format` to `EquipmentViewModel` for UI state management.

### Files Modified
- `src/vm/stats.rs`: added TableState and ExportFormat fields
- `src/vm/equipment.rs`: added TableState, FilterBarState, ExportFormat fields
- `src/ui/stats.rs`: full rewrite using ExportToolbar + UnifiedTable
- `src/ui/equipment.rs`: full rewrite using FilterBar + ExportToolbar + UnifiedTable
- `docs/CHANGELOG.md`: version 0.16.5
- `Cargo.toml`: bumped to 0.16.5

---

## v0.16.4 - Fix world pickup getItemFlagId routing

### Bug Fixes
- **Use getItemFlagId instead of row_id for tile-based world pickups** — The extraction
  script was using ItemLotParam row IDs (e.g., 1045371000, local_id=1000) as event flag IDs.
  The game actually stores flags at the getItemFlagId position (e.g., 1045377100, local_id=7100),
  which converts to tile local_id=100 after subtracting 7000.
- **Route getItemFlagId through tile formula instead of row_id formula** — The WASM
  `calculate_tile_flag_offset_unified()` was sending converted flags to
  `calculate_world_pickup_offset_by_row_id_impl()` (byte ~999K in EF), but the game stores
  them in the tile region via the standard tile formula (byte ~763K). Same fix applied to
  the elden-map server's `getAllSetFlags()`.

### Key Finding
- Unique items (talismans, weapons, armor) from chests DO set event flags via getItemFlagId.
  Consumable/stackable items (Golden Runes, Smithing Stones) still do NOT set any event flag,
  confirming the finding in `EVENT-FLAG-TREASURE-DISCREPANCY.md`.
- Empirically verified with Axe Talisman (getItemFlagId 1045377100): SET at tile (45,37)
  local_id=100 in the save file; CLEAR at the row_id formula offset.

### Files Modified
- `scripts/extract_event_flags.py`: use getItemFlagId for tile-based pickups
- `scripts/extracted_event_flags.json`: regenerated with correct flagIds
- `scripts/extracted_event_flags.md`: regenerated
- `src/db/world_pickups.rs`: regenerated via extract_world_pickups.py
- `crates/wasm-event-flags/src/lib.rs`: route getItemFlagId to tile formula, update tests
- `../elden-map/server/src/eventFlagService.ts`: route converted flags to calibrated tile formula
- `../elden-map/wasm-event-flags/`: rebuilt WASM binary

---

## v0.16.3 - Correct tile base offset for world pickup detection

### Bug Fix
- **Corrected TILE_BASE_OFFSET from 485330 to 337375** (was 147,955 bytes too high)
- The old value was derived using an earlier incorrect EF offset formula; when EF detection was corrected, the tile base was not recalibrated
- This fixes detection of all 69 tile-type world pickup flags (10-digit flags with localId < 7000)

### Key Findings
- Tile base within EventFlags is **constant across all characters** (337375), not variable as previously assumed
- Verified via before/after snapshot diffs across 3 characters (Confessor, V1, Slot7) and 10+ capture pairs
- Old calibration search range (480000-560000) was entirely wrong; corrected to 327000-347000
- The Whetstone Knife tile flag (1042371010) is unreliable as a calibration anchor since the item is usually obtained from a chest (flag 1042371300), not the world pickup

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: TILE_BASE_OFFSET 485330→337375, updated 4 tests
- `ground_truth_offsets.json`: tile_formula.base_offset, calibration anchor, 60 tile flag offsets
- `src/calibration.rs`: updated test assertions
- `src/db/pickup_flags.rs`: updated 2 test assertions
- `src/discovery/offset_probe.rs`: updated tile_base (was 489981)
- `docs/SAVE_FILE_GROUND_TRUTH.md`: corrected tile base references
- `docs/DATABASE_COVERAGE_ANALYSIS.md`: corrected tile base reference
- `CLAUDE.md`: corrected tile base documentation
- WASM rebuilt and deployed to elden-map
- elden-map: updated calibrationService.ts, eventFlagService.ts, shared/wasm-loader.ts

## v0.16.2 - Fix emevd block base off-by-one

### Bug Fix
- **Applied +1 byte correction** to 7 emevd-derived block bases: 65000, 66000, 67000, 68000, 69000, 91000, 92000
- The raw hex values from `common.emevd.js` are off by 1 for these blocks — they point to a header/alignment byte, not the first flag data byte
- Blocks 60000 and 62000 do NOT need the correction (their emevd hex values are exact)
- At the old bases, all decoded flags ended in `...8` (non-round); at corrected bases, **100% of flags are multiples of 10**, matching Elden Ring's flag naming convention

### Verification
- Block 67000: 6/6 SET flags mod10=0 (67890, 67900, 67920, 67960, 67970, 67980)
- Block 68000: 16/16 SET flags mod10=0
- Block 69000: 20/20 SET flags mod10=0
- Block 91000: 41/41 SET flags mod10=0
- Block 92000: 16/16 SET flags mod10=0
- Blocks 65000, 66000: empty in test save, corrected by pattern extrapolation (5/5 verified blocks needed +1)

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: corrected 7 bases in `get_block_bases()`
- `ground_truth_offsets.json`: updated `base_offset` and notes for 7 blocks

---

## v0.16.1 - Correct Non-Grace Block Bases

### Bug Fix
- **Corrected block base offsets** for non-grace event flag categories (progression, maps, whetblades, cookbooks, etc.) that were calibrated against a false-positive EF offset in the GaItemData section
- Old bases (e.g. 62000→9359, 67000→37411) were checking bytes deep in intermediate save sections, not actual EventFlags
- New bases sourced from `common.emevd.js` game event scripts: 60000→1260, 62000→1500, 65000→1684, 66000→1724, 67000→1764, 68000→1804, 69000→1844, 91000→2384, 92000→2424
- Added 4 new block entries (66000, 69000, 91000, 92000); removed incorrect 61000 entry

### Verification
- Map fragment base 1500 verified via 6 timeline diffs with exact bit-level matches
- Cross-validated across 3 character slots (Confessor mid-game, Wretch early, Bee extensive) with progression-appropriate results
- Grace bases (2725, 3250) confirmed unaffected — they were already correct

### Key Finding
- The old "verified 12/12 match" was a false positive: the old bases mapped to byte positions within the GaItemData section (~37K into the slot), which contains non-zero structured data that coincidentally passed bit checks. The correct bases are all within the first ~4K bytes of the EF section, consistent with the system flag allocation layout in `common.emevd.js`.

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: corrected `get_block_bases()`, fixed cookbook test
- `ground_truth_offsets.json`: updated block_bases with correct values
- `src/db/pickup_flags.rs`: updated crystal tears test assertions
- `scripts/verification/block_items.json`: updated bases for blocks 62000, 67000, 68000
- `scripts/verification/test_formulas.py`: updated expected byte offsets

---

## v0.16.0 - Structural EventFlags Detection

### Features
- **Structural offset computation** replaces content-based search as the primary EventFlags detection method
- Sequential section parsing from GaItems through TutorialData deterministically computes the EventFlags offset without searching for grace flag patterns
- Handles two variable-size sections: EquipProjectileData (4 + count×8) and Regions (4 + count×4)
- Pre-EventFlags gap empirically verified as constant 29 bytes (0x1D) across 898 slot measurements
- Content-based search retained as fallback only for corrupted/unknown formats
- Works for brand-new characters with zero graces (content-based cannot)

### Implementation
- Added 30+ section size constants to WASM module mirroring `save_slot.rs` parsing chain
- `compute_structural_ef_offset()`: deterministic offset from sequential section sizes
- `validate_at_offset()`: extracted grace flag validation as reusable helper
- `detect_event_flags_content_based()`: legacy search isolated as fallback
- Native wrapper trusts `confident: true` from structural detection
- New WASM export: `compute_structural_event_flags_offset()`
- 7 new tests for structural detection

### Verification
- `scripts/verification/measure_pre_ef_gap.py`: empirical gap measurement across all save data
- `scripts/verification/verify_captures.py`: capture pair verification framework
- `scripts/verification/verify_pickups.py`: pickup verification framework
- `scripts/verification/verify_timeline.py`: timeline verification framework
- Improved Python EF detection: 0xFF padding rejection, better candidate ranking

### Key Findings
- The pre-EventFlags gap is constant at 29 bytes regardless of character progression
- Content-based detection produced false positives for mid-game and test characters
- Structural detection eliminates all false positives by computing the exact offset

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: structural computation, section constants, 7 new tests
- `src/save/common/event_flags_detection.rs`: updated fallback logic, docstrings
- `docs/DATA-SOURCES.md`: documented capture pairs and timeline data sources
- `scripts/verification/save_parser.py`: 0xFF rejection, improved candidate ranking
- `scripts/verification/utils.py`: 0xFF rejection, robust detection function
- `scripts/verification/ground_truth_loader.py`: None guard in block offset calculation
- `scripts/verification/measure_pre_ef_gap.py`: new empirical measurement script
- `scripts/verification/verify_captures.py`: new capture verification framework
- `scripts/verification/verify_pickups.py`: new pickup verification framework
- `scripts/verification/verify_timeline.py`: new timeline verification framework

---

## v0.15.1 - Fix EventFlags Detection False Positives

### Bug Fixes
- **SEARCH_START raised from 0x12000 to 0x30000**: inventory data at ~76K contained coincidental bit patterns that matched positive validation flags, causing the detector to return a false-positive offset ~146K below the real EventFlags section
- **Removed early return on first perfect match**: the algorithm now collects ALL candidates and selects the best, preventing premature lock-on to false positives
- **Tiebreaker changed to prefer highest offset**: when candidates have equal scores, the last (highest) valid match is selected — empirically validated across 701 captures showing the real EF copy is always the last one (2622 bytes after false copies)
- **Fixed mislabeled validation flag**: flag 76102 was labeled "Gatefront Ruins" but is actually "Stormhill Shack" (real Gatefront is flag 76111)
- **Updated fallback offset in save_slot.rs**: from 0x12B00 to 0x36500 to match the real EF region

### Key Findings
- Real EventFlags offset is ~222K-225K (0x36000-0x37000), NOT ~76K-78K
- The gap between gaItemsEnd and EventFlags grows monotonically during gameplay (+4/+8 byte increments), making any fixed formula unreliable
- Dynamic detection via validation flag scanning is the only correct approach
- Verified stable across 701 captures spanning 14.5 hours of gameplay, 9 area codes, 83 map tiles

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: SEARCH_START, early return removal, tiebreaker, flag label fix
- `src/save/common/event_flags_detection.rs`: test assertion update
- `src/save/common/save_slot.rs`: local SEARCH_START and FALLBACK_OFFSET constants
- `docs/SAVE_FILE_GROUND_TRUTH.md`: corrected EF offset range from 0x12B00 to 0x36000

---

## v0.15.0 - Unified Flag Resolution & Multi-Tile Calibration

### Features
- **Unified flag offset routing** (`get_flag_offset`, `get_flag_offset_calibrated`): single dispatcher handling all flag ranges — tile (1B+), dungeon (8-digit), midrange (6-digit), block/simple (< 100k)
- **Block/midrange/dungeon base maps in WASM**: 12 block bases (60k–78k), 3 midrange bases (510k–710k), ~40 dungeon area+section tuples — all synced from `ground_truth_offsets.json`
- **Multi-tile calibration**: replaced single-anchor calibration with 4-anchor constraint satisfaction from 2+ distinct tiles (Python + Rust), reducing false positives to near-zero
- **Position validation**: reject candidates with denormalized float coordinates or extreme facing angles (|angle| > 2π)
- **Equipment extraction in WASM**: `parse_quick_items_data()`, `parse_equipped_items_data()` for equipped slots, talismans, quick items, pouch
- **`verify-anchors` CLI command**: matrix display of tile pickup anchors across multiple slots for calibration anchor discovery
- **Timeline analysis scripts**: binary diff parsing, grace/pickup extraction, and gameplay narrative reconstruction from granular snapshots

### Implementation
- `get_flag_offset_with_tile_base()`: routes 1B+ → tile formula (local_id < 7000) or row_id formula (≥ 7000); 8-digit → dungeon; 6-digit → midrange lookup; < 100k → block/simple
- Multi-tile calibration searches 430k–510k for candidates matching ≥ 3 anchors from ≥ 2 distinct tiles simultaneously
- `is_denormalized(v: f32)`: checks exponent bits == 0 with non-zero value; `FACING_ANGLE_MAX = TAU`
- `CorroborationEngine` now accepts calibrated tile_base via `with_calibrated_tile_base()`
- Test cases updated with empirical world pickup data from timeline capture files 119–127

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: unified flag resolution, block/midrange/dungeon bases, position validation, equipment extraction, 13 new tests
- `scripts/verification/calibration.py`: multi-tile calibration anchors, multi-constraint search
- `src/calibration.rs`: Rust multi-tile calibration (parallel to Python)
- `src/discovery/cli.rs`: calibrated corroboration, `verify-anchors` command
- `src/discovery/corroboration.rs`: `calibrated_tile_base` field and setter
- `src/discovery/test_cases.rs`: empirical tile-formula test cases, calibrated validation
- `scripts/timeline_analysis.py`: new — binary diff analysis
- `scripts/timeline_graces_pickups.py`: new — grace/pickup extraction from timeline
- `scripts/timeline_narrative.py`: new — gameplay event reconstruction

---

## v0.13.3 - Player Coordinate Extraction Verification

### Changes
- Add `scripts/verification/verify_player_coords.py`: signature-based PlayerCoords extraction from save file snapshots
- Validates extracted coordinates against known grace/boss world positions across 15 test cases (2 character slots, 7 locations)
- Extraction method: searches for slot header `map_id` pattern in the 0x1D0000–0x280000 range, validates surrounding padding bytes (17+16 byte blocks), reads 3×f32 coordinates
- Validation guards: coordinate range (±10,000), magnitude threshold (>10), NaN/Inf rejection, false-positive filtering via padding zero-count scoring

### Key Findings
- Structural parsing (EventFlags→UknownLists→PlayerCoords) fails due to EF offset false positives; signature-based search is reliable
- PlayerCoords `padding2` (16 bytes after coords2) being mostly zeros is the strongest discriminator
- Typical extraction accuracy: 1–14 game units from reference positions for graces, 5–35 units for boss arenas

### Files Modified
- `scripts/verification/verify_player_coords.py`: new file

---

## v0.13.2 - Documentation Audit, Cleanup & Restructuring

### Changes
- Rewrote `docs/DATABASE_COVERAGE_ANALYSIS.md` to reflect current state (40 modules, ~22,184 entries)
- Fixed contradictions across docs: tile base_offset 489981→485330, consumable tracking now TRACKABLE, deprecated flag_formulas.py references removed
- Deduplicated inline formulas in discovery-verification-cycle.md and CORROBORATION-SYSTEM.md with cross-references to EVENT-FLAG-GEOGRAPHY.md
- Archived 9 stale documents to `docs/archive/` with archive headers
- Created `docs/BACKLOG.md` consolidating all scattered "Next Steps" into one prioritized list
- Updated CLAUDE.md doc table (added CASE-VERIFICATION-GUIDE, SAVE_FILE_GROUND_TRUTH, DATA-SOURCES, BACKLOG)
- Updated ARCHITECTURE.md methodology table and COMMIT-PROTOCOL.md references
- Updated `/snapshot` command to keep BACKLOG.md up to date
- Merged confidence normalization concepts into CASE-VERIFICATION-GUIDE.md

### Files Modified
- `docs/DATABASE_COVERAGE_ANALYSIS.md`: full rewrite with current audit data
- `docs/SAVE_FILE_GROUND_TRUTH.md`: fixed contradictions, updated timestamp
- `docs/CORROBORATION-SYSTEM.md`: fixed tile base, deduplicated formulas
- `docs/discovery-verification-cycle.md`: fixed tile base, deduplicated formulas
- `docs/EVENT-FLAG-GEOGRAPHY.md`: absorbed Flag-islands concept, fixed stale value
- `docs/CASE-VERIFICATION-GUIDE.md`: merged confidence normalization
- `docs/ARCHITECTURE.md`: updated methodology table
- `docs/COMMIT-PROTOCOL.md`: fixed IMPLEMENTATION_PLAN→BACKLOG reference
- `docs/DATA-SOURCES.md`: added decompiled game files section
- `docs/BACKLOG.md`: new file consolidating all planned work
- `CLAUDE.md`: updated doc table
- `.claude/commands/snapshot.md`: added BACKLOG.md tracking
- 9 files moved to `docs/archive/` with archive headers

---

## v0.13.1 - Utilities Section & Icon Font Reference

### Features
- Added Utilities top-level section with navigation breadcrumbs
- Added Icomoon icon font reference grid (96 Elden Map marker glyphs)
- Click-to-copy glyph properties, search/filter, hover tooltips with 48px preview
- Registered Icomoon font family for use in UI components

### Files Modified
- src/main.rs: font registration, Utilities route handling, IconsViewState
- src/ui/menu.rs: UtilitiesSelect/UtilitiesIcons routes, breadcrumbs, navigation
- src/ui/mod.rs: added utilities module
- src/ui/utilities/mod.rs: new module
- src/ui/utilities/icons_view.rs: new 96-glyph reference grid
- assets/fonts/icomoon.ttf: new icon font asset
- docs/CHANGELOG.md: v0.13.1
- Cargo.toml: bumped to 0.13.1

---

## v0.13.0 - Entity Relationships Pipeline & DRY Refactor

### Entity Relationships Upstream Migration
- Expanded boss database from 17 hardcoded entries to 205 extracted from GameAreaParam
- Added grace↔boss proximity computation with 200m threshold (188 bosses with nearby graces, 313 graces with nearby bosses)
- Migrated boss drops to structured JSON (`scripts/boss_drops.json`) with 53 boss drop groups
- Generated `entity_relationships_data.rs` with BOSS_DROPS, ITEM_DROPPED_BY, BOSS_DROP_INDEX, BOSS_NEARBY_GRACES, GRACE_NEARBY_BOSSES
- Boss detail panel now shows "Drops" and "Nearby Graces" sections
- Grace detail panel now shows "Nearby Bosses" section with distances
- Accurate boss type classification using SHARDBEARER_FLAGS set + rune tiers

### DRY Refactoring
- Added `mapgenie_section()` shared helper (was duplicated in bosses + graces views)
- Added `section_from_relationships()` generic filter→map→section builder
- Extracted `build_item_sections()` in items_view.rs (removed ~70 lines of copy-paste)
- Extracted `build_merchant_sections()` in merchants_view.rs
- Merged identical CSV/Markdown export arms in all 4 database views
- Removed 6 unused helper functions from relationship_list.rs

### Files Modified
- scripts/generate_db.py: GameAreaParam extraction, proximity computation, boss drops loading, relationship generation
- scripts/boss_drops.json: new structured boss drops data
- src/db/entity_relationships_data.rs: new generated relationships module
- src/db/entity_relationships.rs: refactored to use generated data
- src/db/bosses_data.rs: regenerated with 205 bosses and full metadata
- src/db/boss_drops.rs: deleted (replaced by generated data)
- src/db/mod.rs: updated module declarations
- src/ui/components/detail_panel/relationship_list.rs: new shared helpers, removed dead code
- src/ui/components/detail_panel/mod.rs: updated re-exports
- src/ui/database/bosses_view.rs: uses shared helpers, merged export arms
- src/ui/database/graces_view.rs: uses shared helpers, merged export arms
- src/ui/database/items_view.rs: extracted build_item_sections, merged export arms
- src/ui/database/merchants_view.rs: extracted build_merchant_sections, merged export arms
- Cargo.toml: bumped to 0.13.0

---

## v0.12.1 - Detail Panel Navigation & UI Polish

### Detail Panel Navigation
- External links (MapGenie) now open in default browser via `open` crate
- Merchant relationship links navigate to merchant detail view
- Item detail panel shows merchant cross-navigation with price info
- Added `NavigateToMerchant` and `OpenExternalUrl` detail panel actions

### UI Polish
- Replaced emoji lock icon with Phosphor `LOCK` icon on locked talisman slots
- Improved icon label layout: wider name area (100px), better text wrapping
- Removed monospace from stat values on Character General page
- Code formatting cleanup across general.rs and icons/mod.rs

### Files Modified
- src/ui/general.rs: formatting, lock icon, style tweaks
- src/ui/icons/mod.rs: icon label sizing, formatting
- src/ui/components/detail_panel/panel.rs: new action variants
- src/main.rs: handlers for NavigateToMerchant and OpenExternalUrl
- src/ui/database/bosses_view.rs: OpenExternalUrl for MapGenie links
- src/ui/database/graces_view.rs: OpenExternalUrl for MapGenie links
- src/ui/database/items_view.rs: NavigateToMerchant for merchant links
- src/ui/database/merchants_view.rs: item relationship navigation
- Cargo.toml: added `open` dependency

---

## v0.12.0 - Database Browser & Game Icons

### Game Icon System
- New icon loading module (`src/ui/icons/`) for displaying game item icons
- Icons loaded from extracted game files (160x160 PNG, displayed at 64x64)
- Equipment slots on Character General now show icons with names below
- Lazy-loaded texture caching with egui TextureHandle
- Graceful fallback to dark placeholder when icons unavailable

### Database Browser Enhancements
- Single-click now opens detail panel (was double-click)
- Table columns auto-width based on content
- Navigation breadcrumbs show entity names (e.g., "Graces > Table of Lost Grace")
- Quest chains view is now character-agnostic (pure reference data, no completion tracking)

### New Database Modules
- `src/db/bosses_data.rs`: Boss definitions with defeat flags
- `src/db/graces_data.rs`: Site of Grace database
- `src/db/merchants_data.rs`: Merchant locations and inventory
- `src/db/quest_chains.rs`: Quest progression steps with flag IDs
- `src/db/entity_relationships.rs`: Cross-entity relationship mapping
- `src/db/unified_items.rs`: Consolidated item database

### UI Components
- Detail panel system (`src/ui/components/detail_panel/`)
- Navigation breadcrumb component (`src/ui/components/navigation/`)
- Database views (`src/ui/database/`) for browsing game data
- Comparison view scaffolding (`src/ui/comparison/`)
- Validation view scaffolding (`src/ui/validation/`)

### Equipment ViewModel
- Added `icon_id: u16` field to `EquipmentItemViewModel`
- Icon IDs extracted from param data (EquipWeaponParam, EquipProtectorParam, etc.)

### Files Modified
- `src/ui/general.rs`: Equipment display with game icons
- `src/ui/icons/mod.rs`: New icon loading and caching system
- `src/vm/equipment.rs`: Added icon_id to equipment view model
- `src/ui/database/event_chains_view.rs`: Character-agnostic quest reference
- `src/main.rs`: Updated routing and view calls
- Multiple database and UI component files

---

## v0.11.1 - Warning Cleanup

### Compiler Warnings Fixed
- Fixed unreachable pattern in `vm/general.rs` - DLC region detection now correctly ordered
- Fixed overlapping range patterns in `pickup_flags.rs` and `event_flags_db.rs` for region detection
- Removed 97 unused imports via `cargo fix`
- Added `#![allow(dead_code)]` to research/development modules (discovery, verification, tokens)
- Moved workspace profile settings from subcrate to root `Cargo.toml`

### Files Modified
- `src/vm/general.rs`: Reordered DLC pattern before base game pattern
- `src/db/pickup_flags.rs`: Fixed overlapping region ranges
- `src/db/event_flags_db.rs`: Fixed overlapping region ranges
- `Cargo.toml`: Added workspace-level release profile
- `crates/wasm-event-flags/Cargo.toml`: Removed profile (moved to workspace root)
- Multiple modules: Added `#![allow(dead_code)]` for research tooling

---

## v0.11.0 - Character General Page Redesign

### Build Planner-Style Layout
- Redesigned Character > General page with 3-column layout inspired by build planners
- Column 1: Character Status (matching game's status screen)
  - Level, Runes Held
  - All 8 attributes (Vigor, Mind, Endurance, Strength, Dexterity, Intelligence, Faith, Arcane)
  - HP (current/max), FP (current/max), Stamina
  - Weapon Level, Total Runes
  - DLC Blessings (Scadutree, Spirit Ash) - shown only if > 0
  - Current Location (region name + map ID)
- Column 2: Equipment grid layout
  - Equipped Gear: 3-column grid (Right Hand | Armor | Left Hand)
  - Armaments: 4-column grid for arrows/bolts
  - Talismans: 4-column grid with lock icons for unavailable slots
- Column 3: Quick Items (10 slots) and Pouch (6 slots)

### New Data Fields
- `StatsViewModel`: Added hp, max_hp, fp, max_fp, stamina, max_stamina
- `GeneralViewModel`: Added map_id with MapID struct
- `MapID`: Parses 4-byte location, provides display_name() for region names

### Visual Design
- Dark card backgrounds (Color32::from_rgb(30, 30, 35))
- Grid cells expand to fill available container width
- Double-click to copy item names
- Right-click context menu on equipment slots

### Files Modified
- `src/vm/stats.rs`: Added HP, FP, Stamina fields
- `src/vm/general.rs`: Added MapID struct with region name mapping
- `src/ui/general.rs`: Complete rewrite with 3-column build planner layout
- `docs/CHANGELOG.md`: Added v0.11.0 entry
- `Cargo.toml`: Bumped to 0.11.0

---

## v0.10.0 - Unified Table Design for Event Flags and Inventory

### Event Flags UI Redesign
- Applied World Pickups design pattern (FilterBar + UnifiedTable + ExportToolbar) to all Event Flag subpages
- Created generic `simple_event_flag_view()` helper function to reduce code duplication
- Refactored 7 simple pages to use the generic helper:
  - Whetblades, Cookbooks, Maps, Bosses, Summoning Pools, Colosseums, Landmarks
- Sites of Grace: Flat table with region column, region dropdown filter, status chips
- Dungeon Pickups: Flat table with dungeon dropdown, type/status chips

### Inventory Browse Redesign
- Complete rewrite of Browse view using FilterBar + UnifiedTable + ExportToolbar
- Storage location filter dropdown (All/Equipped/Storage Box)
- Type filter chips for 6 item categories
- Default route changed from None to Browse
- Row colors: green for Equipped, gray for Storage Box

### New State Structs
- `SimpleEventFlagViewState` for generic event flag pages
- `GracesViewState` for Sites of Grace (has region filter)
- `BrowseViewState` for Inventory Browse
- `StorageLocation` enum (All, Equipped, StorageBox)

### Export Structs
- `SimpleEventFlagExportItem`, `GraceExportItem`, `DungeonPickupExportItem`
- `InventoryExportItem` for inventory browse export

### Files Modified
- `src/vm/events.rs`: Added view state structs, updated EventsViewModel
- `src/ui/events.rs`: Refactored 9 view functions, added generic helper
- `src/vm/inventory/mod.rs`: Added BrowseViewState, StorageLocation, default Browse route
- `src/ui/inventory/browse.rs`: Complete rewrite with new design pattern
- `docs/CHANGELOG.md`: Added v0.10.0 entry
- `Cargo.toml`: Bumped to 0.10.0

---

## v0.9.0 - Hierarchical Navigation Restructure

### Navigation Architecture
- **Two-path navigation system**
  - Path A (File): Home → PC|SteamId → CharName → Area → Subroute
  - Path B (Database): Home → Database → DatabaseName

- **New intermediate routes**
  - `CharacterSelect`: File loaded, shows character slots in submenu
  - `DatabaseSelect`: Database mode, shows database list in submenu

- **Clickable breadcrumb levels**
  - Each segment navigates to that hierarchy level
  - Platform/SteamID shows full save path on hover

### Landing Page
- **New home view with recent files**
  - Shows list of recently opened save files
  - Displays character names for each save
  - Persists to `~/.er-save-editor/config.json`
  - Supports drag-and-drop file opening

### Top Menu Bar
- **Simplified toolbar layout**
  - Left: Open button (with recent files dropdown), Database button
  - Right: Save (disabled/strikethrough), Export button

### Compact Footer
- **Icon-only status bar legend**
  - Shows Flag and Inv section labels
  - Icons with hover tooltips for detailed explanations
  - Reduced height from 28px to 24px

### Route Enum Restructure
- Renamed routes for clarity:
  - `General` → `CharacterGeneral`, etc.
  - `Spells` → `DatabaseSpells`, etc.
- Added `CharacterSelect` and `DatabaseSelect` routes
- Added `DatabaseDungeonPickups` route

### Files Modified
- `src/ui/menu.rs`: Route enum, breadcrumb_bar, navigation_buttons
- `src/main.rs`: Top menu, content routing, App struct with recent_files
- `src/ui/landing.rs`: New landing page module
- `src/ui/state/recent_files.rs`: Recent files persistence
- `src/ui/state/mod.rs`: Export recent_files module
- `src/ui/mod.rs`: Export landing module
- `src/ui/components/status_bar.rs`: Compact icon legend with hover tooltips
- `docs/CHANGELOG.md`: Added v0.9.0 entry
- `Cargo.toml`: Bumped to 0.9.0

---

## v0.8.4 - IBM Plex Fonts and UI Polish

### Typography
- **Added IBM Plex font family**
  - IBM Plex Sans: Default UI font (proportional)
  - IBM Plex Sans Condensed: Table/list headers (`font_condensed()`)
  - IBM Plex Mono: Monospace text (`.monospace()`)
  - IBM Plex Serif: Paragraph/description text (`font_serif()`)

### UI Polish
- **Replaced separator lines with spacer component**
  - Added `spacer(ui)` function in `style.rs` with `SECTION_SPACING = 8.0`
  - Removed gray horizontal lines from all views
  - Cleaner visual appearance

- **Breadcrumb caret icon**
  - Replaced ">" text with Phosphor CARET_RIGHT icon

### Files Modified
- `src/main.rs`: Font configuration with IBM Plex family
- `src/ui/style.rs`: Added `spacer()`, `font_condensed()`, `font_serif()`
- `src/ui/*.rs`: Replaced `ui.separator()` with `spacer(ui)` (14 files)
- `src/ui/menu.rs`: Breadcrumb uses CARET_RIGHT icon
- `assets/fonts/`: IBM Plex font files (Sans, Condensed, Mono, Serif)
- `docs/CHANGELOG.md`: Added v0.8.4 entry
- `Cargo.toml`: Bumped to 0.8.4

---

## v0.8.3 - Horizontal Navigation Layout

### UI Restructure
- **Replaced vertical sidebars with horizontal 3-row navigation**
  - Row 1: Toolbar with Open/Save buttons, platform info, Steam ID, Export button
  - Row 2: Clickable breadcrumb trail (Characters > CharacterName > Area > Subroute)
  - Row 3: Dynamic navigation buttons that change based on current level

- **Navigation hierarchy**
  - Level 1 (Root): Character buttons + Database view buttons
  - Level 2 (Character selected): Area buttons (General, Stats, Equipment, etc.)
  - Level 3 (Event Flags): Subroute buttons (Sites of Grace, Bosses, World Pickups, etc.)

- **Added display_name() methods** to Route and EventsRoute enums for breadcrumb display

### Removed
- Left sidebar for character list
- Left sidebar for slot sections menu
- Left sidebar for EventFlags subroute navigation

### Files Modified
- `src/ui/menu.rs`: Added breadcrumb_bar(), navigation_buttons(), helper functions, Route::display_name()
- `src/vm/events.rs`: Added EventsRoute::display_name()
- `src/main.rs`: Removed sidebars, added breadcrumb/navigation panels, updated toolbar layout
- `src/ui/events.rs`: Removed left sidebar, content renders directly into provided ui
- `src/ui/none.rs`: Updated empty state message
- `docs/CHANGELOG.md`: Added v0.8.3 entry
- `Cargo.toml`: Bumped to 0.8.3

---

## v0.8.2 - Special Override Detection in Event Flag Extraction

### Enhancements
- **Special override detection** for tile-based items with block-based getItemFlagId
  - Items like Whetstone Knife (tile row_id) use block flag 60130 instead of tile formula
  - Extraction scripts now detect when `getItemFlagId` returns a different flag type
  - Prevents incorrect flag ID assignment in generated database

- **Improved region parsing** in world pickups extraction
  - Better 10-digit tile ID vs 8-digit dungeon ID differentiation
  - Cleaner region classification logic

### Database Regeneration
- Regenerated `extracted_event_flags.json` (7086 flags)
- Regenerated `extracted_event_flags.md` with location data
- Regenerated `src/db/world_pickups.rs` database

### Files Modified
- `scripts/extract_event_flags.py`: Special override detection logic
- `scripts/extract_world_pickups_v2.py`: Improved region ID parsing
- `scripts/extracted_event_flags.json`: Regenerated database
- `scripts/extracted_event_flags.md`: Regenerated documentation
- `src/db/world_pickups.rs`: Regenerated Rust database
- `docs/CHANGELOG.md`: Added v0.8.2 entry
- `Cargo.toml`: Bumped to 0.8.2

---

## v0.8.1 - Context Metadata in Flag Details Export

### Enhancement
- **Added context metadata to Copy Details export**
  - `timestamp`: When the export was created
  - `save_file`: Full path to the loaded .sl2 file
  - `slot_index`: Character slot number (0-9)
  - `character_name`: Character's in-game name
  - `event_flags_size`: Size of event flags array (validates data loaded)

### Files Modified
- `src/main.rs`: Pass save_path to events view
- `src/ui/events.rs`: Include context metadata in Copy Details output
- `docs/CHANGELOG.md`: Added v0.8.1 entry
- `Cargo.toml`: Bumped to 0.8.1

---

## v0.8.0 - Flag Details Sidebar with Inventory Evidence

### Features
- **Flag Details sidebar panel** for World/Dungeon Pickups
  - Click any pickup row to select it and open details panel
  - Shows flag ID (decimal/hex), item name, collected status
  - Displays byte offset and bit position for debugging
  - "Copy Details" button exports comprehensive debug data

- **Inventory evidence matching** with fuzzy search
  - Searches both equipped inventory AND storage box (4 locations total)
  - Shows whether inventory evidence SUPPORTS or CHALLENGES flag status
  - Collapsible "Raw Data" section with ga_item_handle, inventory_index, storage location
  - Match scoring (exact=100%, contains=90%, word overlap=60%+)

- **World pickup row_id formula** for local_id >= 7000
  - Discovery: World pickups with getItemFlagId (local_id 7000+) use separate bitfield
  - Formula: `byte_offset = (row_id - 1037373320) / 8`
  - Verified via before/after save captures of Golden Rune pickups

### Bug Fixes
- **Reverse lookup returns all overlapping blocks** - Fixed to return ALL matching blocks when byte ranges overlap (blocks 71600 and 76000 overlap at [3250, 3323))
- **Widget ID collisions** - Fixed egui ID errors in inventory matches loop using `push_id`

### Technical Changes
- Added `get_storage_inventory()` method to SaveType
- Added `WORLD_PICKUP_ROW_ID_BASE` constant (1037373320)
- Added `calculate_world_pickup_offset_by_row_id()` function
- Updated `calculate_tile_flag_offset` to use row_id formula for local_id >= 7000
- Extended WASM crate with pickup flag calculations and tests
- Updated tests to reflect new formula expectations

### Documentation
- Added "False Negative Investigation Protocol" to CLAUDE.md
- Documented row_id tracking discovery in EVENT-FLAG-GEOGRAPHY.md

### Files Modified
- `src/ui/events.rs`: Flag details sidebar, inventory matching, Copy Details
- `src/vm/events.rs`: Added selected_flag_id to filter structs
- `src/db/pickup_flags.rs`: Row_id formula, updated tile/dungeon offset calculations
- `src/save/save.rs`: Added get_storage_inventory() method
- `src/main.rs`: Pass storage inventory to events view
- `src/discovery/reverse_lookup.rs`: Return all overlapping blocks
- `crates/wasm-event-flags/src/lib.rs`: Pickup flag calculations
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Row_id tracking documentation
- `CLAUDE.md`: False Negative Investigation Protocol
- `tests/regression_suite.rs`: Updated block base test
- `Cargo.toml`: bumped to 0.8.0

---

## v0.7.2 - Documentation: Per-Section Discovery

### Documentation
- Updated `docs/EVENT-FLAG-GEOGRAPHY.md` with per-section discovery findings
  - Added "Dungeon Pickup Bases (CRITICAL DISCOVERY)" section
  - Documented why linear formula was wrong
  - Added table of verified section bases (89 total)
  - Listed discovery scripts for future reference
  - Updated Legacy Dungeons table with verification status

### Files Modified
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Added per-section discovery documentation
- `docs/CHANGELOG.md`: Added v0.7.2 entry
- `Cargo.toml`: bumped to 0.7.2

---

## v0.7.1 - Per-Section Pickup Base Discovery

### Bug Fix
- **Fixed dungeon pickup detection for Catacombs, Caves, Tunnels**
  - Discovery: The linear formula `base + section * 1125` was WRONG
  - Each (area, section) combination has its own empirically-discovered base offset
  - Detection improved from ~25% to **100%** for all verified sections

### Key Finding
The linear section formula assumed contiguous memory allocation, but in reality:
- Catacombs sections use bases ranging from 1785 to 3827 (non-linear)
- Caves sections use bases ranging from 1786 to 31903 (wildly varying)
- Tunnels sections use bases ranging from 1788 to 28979 (scattered)

### Technical Changes
- Added `DUNGEON_PICKUP_SECTION_BASES` HashMap with 89 verified entries
- Each entry maps `(area, section)` → `base_offset`
- Formula: `offset = section_base + local_id/8` (no section multiplication)
- All 89 entries verified with 100% match rates across save files

### Verification Results
| Area | Before | After |
|------|--------|-------|
| Catacombs (30) | 27/106 (25%) | 106/106 (100%) |
| Caves (31) | 34/140 (24%) | 140/140 (100%) |
| Tunnels (32) | 23/56 (41%) | 56/56 (100%) |

### Scripts Added
- `scripts/verify_specific_pickups.py`: Check pickups against save data
- `scripts/discover_per_section_bases.py`: Brute-force base discovery
- `scripts/build_pickup_section_map.py`: Generate Rust HashMap code

### Files Modified
- `src/db/pickup_flags.rs`: Added DUNGEON_PICKUP_SECTION_BASES, updated calculation
- `Cargo.toml`: bumped to 0.7.1

---

## v0.7.0 - Complete Dungeon Pickup Database

### Features
- **Complete dungeon pickup database** (2,108 entries)
  - Covers all 23 dungeon area types: Stormveil Castle, Leyndell, Catacombs, Caves, Tunnels, Hero's Graves, etc.
  - Up from 1,950 entries to 2,108 (including 150 items without MSB position data)
  - Item names resolved from EquipParam files (weapons, armor, goods, talismans, ashes of war)
  - Categories: Golden Runes (359), Smithing Stones (293), Consumables (1,115), Weapons (145), Armor (112), Talismans (57)

- **New Dungeon Pickups UI view**
  - Filter by dungeon area, item category, and collection status
  - Shows collection progress per area (e.g., "Catacombs: 45/123 collected")
  - Search functionality for item names
  - Highlights items from unverified pickup bases

- **Dungeon pickup generation script** (`scripts/generate_dungeon_pickups.py`)
  - Combines extracted_event_flags.json with ItemLotParam_map for complete coverage
  - Cross-references EquipParam files for accurate item names
  - Category-aware name resolution (handles overlapping item IDs)
  - Outputs analysis report showing coverage by area

- **Grace reliability improvements**
  - GraceStatus enum with Discovered/NotDiscovered/Unreliable variants
  - Unreliable block detection shown in UI with warning
  - Count summaries exclude unreliable graces

### Technical Details
- Flag formula: `event_flag = row_id + 7000` for dungeon pickups
- 1,958 pickups have MSB position data, 150 are ItemLotParam-only
- Section size: 1,125 bytes per dungeon section
- All 17 pickup base offsets verified via temporal differential analysis

### Files Modified
- `src/db/dungeon_pickups.rs`: Regenerated with 2,108 entries
- `src/db/pickup_flags.rs`: Added DUNGEON_PICKUP_BASES map
- `src/ui/events.rs`: New dungeon_pickups() view, grace reliability display
- `src/vm/events.rs`: Added GraceStatus enum, DungeonPickups route
- `scripts/generate_dungeon_pickups.py`: New generation script
- `scripts/discover_all_dungeon_pickup_bases.py`: Discovery tool
- `scripts/verify_dungeon_pickup_bases.py`: Verification tool

---

## v0.6.0 - WASM Shared EventFlags Detection

### Features
- **Single source of truth for EventFlags detection**
  - New `wasm-event-flags` crate provides shared detection algorithm
  - Used by both ER-save-Editor (native Rust) and elden-map (via WASM)
  - Eliminates implementation drift between projects
  - Guarantees identical detection results

- **Improved detection algorithm**
  - Added negative validation flags (late-game graces that should NOT be set)
  - Prevents false positives from random data matching bit patterns
  - Fixed search start offset to 0x12000 (73,728 bytes)

- **Detection parameters in ground_truth_offsets.json**
  - Added `event_flags_detection` section with all validation flags
  - Documents positive validation (7 flags) and negative validation (6 flags)
  - Single source of truth for detection configuration

### Architecture
- `crates/wasm-event-flags/` - New Rust crate with detection algorithm
- `src/save/common/event_flags_detection.rs` - Delegates to shared crate
- Builds to WebAssembly for elden-map via `wasm-pack`

### Documentation
- Added `docs/WASM-EVENT-FLAGS.md` with full documentation
- Updated `CLAUDE.md` with WASM docs reference

### Files Modified
- `Cargo.toml`: Added workspace, wasm-event-flags dependency
- `crates/wasm-event-flags/`: New shared detection crate
- `src/save/common/event_flags_detection.rs`: Delegates to shared crate
- `ground_truth_offsets.json`: Added event_flags_detection section
- `docs/WASM-EVENT-FLAGS.md`: New documentation

---

## v0.5.4 - Item Pickup Auto-Completion & Late-Game Grace Fixes

### Features
- **Progression-gated validation for late-game graces (76400+)**
  - Level 10 characters no longer show Forbidden Lands (76500) as discovered
  - Graces require prerequisite boss defeats: Morgott for 76500-76700, Fire Giant for 76700+
  - Prevents false positives from uninitialized memory in late-game grace regions

- **Dungeon prerequisite validation for Stormveil Castle (71000)**
  - Calibration now checks if Margit (10000850) is defeated before calibrating Stormveil
  - Prevents false positives when player hasn't reached the castle
  - Lowered match threshold to 50% (3 of 6 graces) for Stormveil since it's required progression

- **Row ID conversion for world tile pickups**
  - Added `convert_to_row_id()` to convert getItemFlagId (localId 7000+) to row_id (localId 0-999)
  - The game stores row_id, not getItemFlagId - this enables 993 world pickups to be tracked
  - Added `is_tile_pickup_flag_set()` for calibrated tile pickup checking

### Technical Changes
- Added `PROGRESSION_GATES` constant with boss flag requirements per grace range
- Added `check_progression_gate()` to verify boss defeats before showing late-game graces
- Added `DUNGEON_PREREQUISITES` constant mapping dungeon blocks to required boss flags
- Added `LEGACY_DUNGEON_BLOCKS` with Stormveil grace anchors for calibration
- Added `calibrate_legacy_dungeon_block()` for independent legacy dungeon calibration

### Key Findings
- **Row ID Discovery (2026-01-23)**: For tile-based world pickups, ItemLotParam has `getItemFlagId = row_id + 7000`. The game stores `row_id` (storable), NOT `getItemFlagId` (unstorable). Example: flag 1044367310 (localId 7310) → stored as 1044360310 (localId 310).
- **Progression gates**: Late-game grace flags (76500+) can show false positives on early-game saves because the memory region may contain uninitialized/garbage data. Gating by boss defeats ensures the player has actually reached those areas.

### Files Modified
- `src/calibration.rs`: Added DUNGEON_PREREQUISITES, LEGACY_DUNGEON_BLOCKS, calibrate_legacy_dungeon_block()
- `src/db/pickup_flags.rs`: Added convert_to_row_id(), is_tile_pickup_flag_set(), test
- `src/vm/events.rs`: Added PROGRESSION_GATES, check_progression_gate()

---

## v0.5.3 - Dynamic Grace Block Calibration

### Features
- **Dynamic calibration for unreliable grace blocks**: Graces from blocks 71000, 71100, 71600 now use per-save calibration
  - Uses tutorial grace (Cave of Knowledge, flag 71800) as calibration anchor
  - Detects offset delta between ground truth and actual save layout
  - Validates calibration using multiple early-game graces (The First Step, Church of Elleh, etc.)
  - Confidence scoring: 0.90+ for high-quality matches, lower for uncertain calibration

- **Reliability filtering fallback**: When calibration fails, graces are marked `[?]` and excluded from counts
  - Prevents false positives where calibration cannot be determined
  - UI shows warning for unreliable graces with uncertain status

### Technical Changes
- Added `GraceBlockCalibration` struct with calibrated bases per block
- Added `CalibrationService::calibrate_grace_blocks()` for dynamic offset detection
- Added `CalibrationService::detect_offset_delta()` using tutorial grace anchor
- Added `CalibrationService::validate_delta()` for cross-validation
- Added `CalibrationService::get_grace_offset_calibrated()` for calibrated lookups
- Added `GraceStatus` enum with `Discovered`, `NotDiscovered`, `Unreliable` variants
- Added `is_block_reliable(flag_id)` function for static reliability checks
- Unreliable graces (failed calibration) are skipped when writing to save file

### Coverage Impact
- **Before**: 329/421 graces (78%) reliably detectable, 92 (22%) marked unreliable
- **After**: Up to 421/421 graces (100%) detectable when calibration succeeds
- Calibration success depends on save having tutorial graces discovered

### Files Modified
- `src/calibration.rs`: Added grace block calibration infrastructure
- `src/db/pickup_flags.rs`: Added `is_block_reliable()` function
- `src/vm/events.rs`: Use calibration for grace status detection
- `src/ui/events.rs`: Updated graces view to show reliability status
- `src/vm/vm.rs`: Skip unreliable graces when updating save
- `src/vm/slot.rs`: Handle GraceStatus in export

---

## v0.5.2 - Block 520000 Expansion & 67000 Investigation

### Database Expansion
- **Block 520000**: Added 6 new verified flags (5/5 inventory-differential match)
  - 520600: Rusted Anchor
  - 520610: Roar Medallion
  - 520620: Smithing-Stone Miner's Bell Bearing [1]
  - 520650: Somberstone Miner's Bell Bearing [2]
  - 520660: Dragon Heart
  - 520670: Somber Smithing Stone [6]
- Block 520000 now has **18 verified flags** (was 12)

### Data Corrections
- **Block 67000**: Marked `blocked` status (was `needs_investigation`)
  - BLOCK_ITEMS mappings are completely incorrect (e.g., says 67120="Missionary's Cookbook [1]" but game data says 67120="Nomadic Warrior's Cookbook [21]")
  - Actual world pickup flags: 67030, 67120, 67130, 67300, 67420, 67430, 67630, 67860, 67880, 67890, 67910
  - Need to rebuild flag-item mappings from game params before verification can proceed

### Files Modified
- `ground_truth_offsets.json`: Added 6 flags, updated block 67000 status

---

## v0.5.1 - Schema Pre-filtering & Block Investigation

### Features
- **Schema-based pre-filtering in batch verification** (`scripts/verification/case_cli.py`)
  - Added `--schema-filter` flag to automatically skip untrackable flags
  - Probes save file before verification loop to identify sparse allocation gaps
  - Reports skipped flags in "EVIDENCE GAPS" section
  - Prevents wasted effort investigating flags known to be in padding regions

### Bug Fixes / Data Corrections
- **Flagged incorrect block bases in ground_truth_offsets.json**
  - Block 62000: Marked `needs_investigation` - flag IDs in BLOCK_ITEMS (62010-62080) don't exist in game data; offset 9359 contains 8-byte record structure, not bit-packed flags
  - Block 67000: Marked `needs_investigation` - base offset 37411 incorrect; flags show unset even when items present in inventory
  - Block 68000: Marked `needs_investigation` - derived from incorrect 67000 base

### Key Findings
- **Block 62000**: BLOCK_ITEMS used assumed flag IDs that don't exist. Actual map fragment pickup flags are 10-digit tile-based (e.g., 1042370200). Block 62000 contains WorldMapPointParam flags for location discovery.
- **Block 67000/68000**: Flag IDs are valid but base offsets need re-discovery. Original verification likely used different save file.

### Files Modified
- `ground_truth_offsets.json`: Updated status for blocks 62000, 67000, 68000
- `scripts/verification/case_cli.py`: Added schema-filter integration, documented block issues

---

## v0.5.0 - Schema-Based Allocation Detection & Case Verification System

### Features
- **Schema-based flag allocation detection** (`scripts/verification/flag_schema.py`)
  - `BlockSchema`: Define known flag IDs and their expected byte offsets
  - `AllocationBitmap`: Probe save data to identify trackable vs untrackable flags
  - Detects **sparse allocation gaps** where the game doesn't allocate memory
  - CLI: `python flag_schema.py --block 520000 --base 1341 --save /path/to/save.sl2 --boundaries`

- **Case-based verification system** (`scripts/verification/case_manager.py`, `case_cli.py`)
  - Defense/Challenge methodology for rigorous flag verification
  - Evidence aggregation from inventory, differential, temporal sources
  - Formula update proposals when verification fails
  - Gap reporting for untrackable flags

- **Verified block 520000** (Spirit Ashes, Talismans)
  - Base offset: 1341
  - 46 flags trackable, 13 in sparse gaps
  - 12 flags exported to ground_truth with confidence 1.0

### Bug Fixes
- **Fixed anchor database access** in `case_manager.py`
  - `boss_defeat_chains`: Now correctly accesses nested `.get('chains', {})` structure
  - `geographic_regions`: Now correctly accesses nested `.get('regions', {})` structure

### Refactoring (DRY)
- **Centralized all formula constants** in `ground_truth_loader.py`
  - Removed hardcoded BLOCK_BASES from `extract_test_cases.py`, `case_cli.py`, `verify_boss_chain.py`
  - All verification scripts now use `get_block_base()`, `get_tile_config()`, etc.
  - Archived deprecated `flag_formulas.py` to `archive/` directory

### Documentation
- **docs/ARCHITECTURE.md**: Added flag_schema.py API reference
- **docs/EVENT-FLAG-GEOGRAPHY.md**: Added "Sparse Flag Allocation" section with terminology
- **docs/EVIDENCE-BASED-DISCOVERY.md**: Updated block 520000 findings with verified results
- **docs/CASE-BASED-VERIFICATION.md**: Added schema pre-filtering section

### Key Discovery: Sparse Flag Allocation
Block 520000 uses sparse memory allocation - not all flag IDs have storage:
```
520000-520059: ALLOCATED
520060-520089: SPARSE GAP (0xFF in all slots)
520090-520189: ALLOCATED
520190-520219: SPARSE GAP
...
```
Flags in sparse gaps (e.g., 520210, 520330, 520450) cannot be verified with the block formula.

### Files Modified
- `scripts/verification/flag_schema.py`: New schema/allocation bitmap system
- `scripts/verification/case_manager.py`: Bug fixes for anchor database
- `scripts/verification/case_cli.py`: DRY refactoring, gap reporting
- `scripts/verification/extract_test_cases.py`: DRY refactoring
- `scripts/verification/verify_boss_chain.py`: DRY refactoring
- `ground_truth_offsets.json`: Added block 520000, 12 verified flags, untrackable_flags
- `docs/*.md`: Documentation updates

---

## v0.4.31 - Tile Formula Base Offset Reversion

### Bug Fixes
- **Reverted tile formula base_offset from 489981 back to 485330**: The v0.4.28 "correction" was wrong
  - Re-verification showed offset 857482 had NO change during Smoldering Butterfly pickup
  - Actual observed change: offset **852831** bit 5 SET (0x00 → 0x20)
  - Calculation confirms: 485330 + 420*875 + 1 = 852831

### Enhancements
- **Added calibration_anchors section** to `ground_truth_offsets.json`
  - Tile anchor: Smoldering Butterfly (1043500010) at offset 852831, bit 5
  - Block anchors: The First Step (76100), Church of Elleh (76101), Cave of Knowledge (71800)
  - Enables runtime validation of formula correctness

### Files Modified
- `ground_truth_offsets.json`: Reverted tile base, added calibration_anchors
- `elden-map/server/src/eventFlagService.ts`: TILE_BASE_OFFSET=485330, TILE_COL_BASE=30
- `scripts/verification/flag_formulas.py`: TILE_CONFIG.base_offset=485330
- `src/db/pickup_flags.rs`: Updated comments and test assertions
- `scripts/capture_agent.py`: Updated TILE_BASE_OFFSET constant
- `scripts/verification/*.py`: Updated default fallback values
- `docs/SAVE_FILE_GROUND_TRUTH.md`: Corrected tile formula documentation

---

## v0.4.30 - Snapshot Capture Automation

### Features
- **Automated snapshot capture workflow**: New system for capturing save file snapshots with POI metadata
  - `scripts/capture_agent.py`: Standalone HTTP server (port 8765) for save file capture
  - Supports before/after pairing with auto-chaining for sequential captures
  - Generates indexed filenames with flag_id, map_tile, and phase
  - Updates `capture_catalog.json` with full metadata
  - CLI commands: `serve`, `capture`, `migrate`, `status`

- **Dynamic verification test runner**: New calibration-aware test selection
  - `scripts/verification/snapshot_test_runner.py`: Selects appropriate snapshot pairs for testing
  - Calibrates formula bases per-save (addresses save-dependent offset issue)
  - Filters tests by flag format, verification status, and confidence level

### Documentation
- **EVENT-FLAG-GEOGRAPHY.md**: Added "Save-Dependent Base Offsets" warning section
  - Documents that tile/dungeon formula bases vary per save file
  - Explains GaItems section size variability affecting EF section offset
  - Provides calibration anchors for each formula type

- **discovery-verification-cycle.md**: Added "Automated Snapshot Capture Workflow"
  - Documents complete user workflow from in-game to capture
  - Explains auto-chaining logic for sequential snapshots
  - Describes capture_catalog.json schema and usage

### Files Added
- `scripts/capture_agent.py`: HTTP capture agent with catalog management
- `scripts/verification/snapshot_test_runner.py`: Dynamic test selection and calibration

### Files Modified
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Save-dependent base offset documentation
- `docs/discovery-verification-cycle.md`: Automated capture workflow documentation

---

## v0.4.29 - World Pickup False Positive Fix

### Bug Fixes
- **Fixed false positives in World Pickups view**: Items no longer incorrectly show as collected
  - Root cause: Using `getItemFlagId` instead of lot_id (ROW ID) for tile-based world pickups
  - Game stores lot_id directly (local_id 0-999) not getItemFlagId (lot_id + 7000)
  - Updated `extract_pickup_data.py` to use correct flag IDs
  - Regenerated `pickup_data.rs` with corrected event flags

- **Formula-based offset calculation**: Migrated from stale EVENT_FLAGS lookup table to dynamic formulas
  - `events.rs`, `vm.rs`, `world_pickups_view.rs` now use `get_flag_offset()` from `pickup_flags.rs`
  - Added 100-flag granularity for block flags (e.g., 71600, 71800)
  - Returns None for dungeon pickups without verified base offsets (prevents false positives)

### Features
- **Unverified filter**: Added "Unverified" option to Status filters in World Pickups view
  - Shows only items where verification status is uncertain
  - Helps identify potentially inaccurate flag mappings

### Key Findings
- Tile-based pickups (10-digit flags ≥1B) use ROW ID as flag, not getItemFlagId
- getItemFlagId formula adds 7000 to lot_id, but local_id ≥7000 has no storage allocation

### Files Modified
- `src/db/pickup_flags.rs`: Added 100-flag block granularity, None for unverified dungeons
- `src/db/pickup_data.rs`: Regenerated with correct flag IDs
- `src/ui/events.rs`: Added Unverified filter, use `get_flag_offset()`
- `src/ui/world_pickups_view.rs`: Added Unverified filter
- `src/vm/events.rs`: Use `get_flag_offset()` for all flag lookups
- `src/vm/vm.rs`: Use `get_flag_offset()` for writing event flags
- `scripts/extract_pickup_data.py`: Use lot_id for tile-based pickups
- `docs/CHANGELOG.md`: Version 0.4.29
- `Cargo.toml`: Bumped to 0.4.29

---

## v0.4.28 - Flag Formula Discovery

### New Formulas
- **Block 61000**: Base 2671 - Map area visit tracking flags (108 flags)
  - Correlates 611xx to mXX dungeon map codes (e.g., 61100→m10 Stormveil, 61128→m18 Roundtable Hold)
  - Verified via multi-flag correlation on Slot 0 mid-game save

- **Midrange 510000**: Base 63750 - Remembrance consumption flags (64 flags)
  - Set when remembrance is USED at Enia, not when obtained
  - Derived from event_flags.rs hardcoded data

- **Midrange 710000**: Base 13875 - Roundtable Hold NPC progression flags (41 flags)
  - Tracks NPC state changes during game progression
  - Derived from event_flags.rs hardcoded data

### Coverage Improvement
- Formula coverage: 57.4% → 60.9% (+3.5%, +213 flags)
- Remembrance context now 100% covered (68/68 flags)

### Files Modified
- `ground_truth_offsets.json`: Added 61000, 510000, 710000 formulas
- `docs/CHANGELOG.md`: Version 0.4.28
- `Cargo.toml`: Bumped to 0.4.28

---

## v0.4.27 - Unified Flag Database

### Features
- **Unified Flag Database**: Merges three data sources into single queryable database
  - Flag Catalog: names, positions, regions, item associations
  - Param Database: source traceability (param file, row ID, field)
  - Event Graph: EMEVD triggers, dependencies, progression chains

- **New `param-extract` CLI command**: Extracts flags from regulation-bin XML params
  - Supports: ItemLotParam_map, BonfireWarpParam, WorldMapPointParam, ShopLineupParam, GameAreaParam, NpcParam
  - Outputs to param_flags.json for reuse

- **New `param-query` CLI command**: Query param flag database
  - `--stats`: Show summary statistics
  - `--blocks`: List midrange blocks with flags
  - `--bosses`: List boss defeat flags with names
  - `--param <name>`: Filter by param source

- **New `unified` CLI command**: Query unified flag database
  - `--build`: Build/rebuild from all sources
  - `--search <name>`: Search flags by name
  - `--needs-formula`: Flags in params but not EMEVD
  - `--high`: High-confidence flags (all 3 sources)
  - `--category`, `--context`: Filter queries

### Technical Details
- `SourceConfidence` enum: High/Medium/Low/Inferred based on source count
- Indexed lookups by category, param, trigger context, region
- JSON persistence for fast subsequent loads

### Files Modified
- `src/discovery/unified_db.rs`: New unified database implementation
- `src/discovery/param_flags.rs`: New param extraction module
- `src/discovery/mod.rs`: Module exports for new components
- `src/discovery/cli.rs`: CLI commands for unified and param modules
- `docs/CHANGELOG.md`: Version 0.4.27
- `Cargo.toml`: Bumped to 0.4.27

---

## v0.4.26 - Batch Validation Tool for EMEVD-Backed Flags

### Features
- **New `batch-validate` CLI command**: Validates all EMEVD-backed flags against save data
  - Reports formula coverage, set/unset status, and verification levels
  - Breaks down by trigger context and flag block
  - Identifies blocks needing formula coverage

### Command Options
- `--block <id>`: Filter to specific 1000-flag block (e.g., `--block 9000`)
- `--context <name>`: Filter by trigger context (e.g., `--context boss_defeat`)
- `--set` / `--unset`: Show only set or unset flags
- `--invalid`: Show only flags without offset formulas

### Key Findings
- Block 9000 (remembrance flags 91xx) confirmed using simple formula
- 6,161 flags with EMEVD triggers, 3,537 (57.4%) have formulas
- Identified coverage gaps: blocks 510000, 710000, 61000

### Files Modified
- `src/discovery/cli.rs`: Added cmd_batch_validate function and stats structs
- `src/discovery/event_graph.rs`: Added get_all_flag_ids() method
- `docs/CHANGELOG.md`: Version 0.4.26
- `Cargo.toml`: Bumped to 0.4.26

---

## v0.4.25 - Midrange Flag Formula Support (Sorceries/Incantations)

### Features
- **New midrange formula**: Support for 6-digit flags (100000-999999)
  - Covers sorcery, incantation, and ash of war unlock flags
  - Formula: `byte_offset = base + (flag_id - block_start) / 8`
  - Block 540000 verified with 129/129 flags matching

### Technical Details
- Added `VERIFIED_MIDRANGE_BASES` to ground_truth_offsets.json
- Added `calculate_midrange_flag_offset()` to pickup_flags.rs
- Build system generates midrange bases from JSON at compile time
- Supports both 1000-flag and 10000-flag block granularity

### Verification
- All 129 sorcery/incantation flags (540100-540652) verified against event_flags.rs hardcoded data

### Files Modified
- `build.rs`: Generate VERIFIED_MIDRANGE_BASES and MidrangeBase struct
- `ground_truth_offsets.json`: Added midrange_formula section
- `src/db/pickup_flags.rs`: Added midrange flag calculation
- `docs/CHANGELOG.md`: Version 0.4.25
- `Cargo.toml`: Bumped to 0.4.25

---

## v0.4.24 - EventGraph Integration into Verification Chain

### Features
- **Corroboration engine integration**: EventGraph now provides EMEVD evidence during flag validation
  - Adds +1 to agreement count when flag has SetEventFlagID trigger
  - Adds +0.1 confidence boost for flags found in EMEVD
  - Reports trigger context, source files, and progression chains

- **New CLI command** `discovery event-graph`:
  - `<flag_id>` - Query specific flag for triggers, dependencies, entity mappings
  - `--stats` - Show event graph statistics (6,161 flags, 13,612 triggers)
  - `--contexts` - List all trigger contexts with counts
  - `--chains` - Show remembrance and map fragment progression chains

- **Enhanced corroborate command**:
  - Automatically loads event graph when available
  - Shows EMEVD validation in output (trigger count, context, sources)
  - Falls back gracefully if event graph unavailable

### Integration Points
```rust
// Load corroboration engine with EMEVD validation
let engine = CorroborationEngine::load_with_event_graph()?;

// Result now includes event graph evidence
result.event_graph.has_trigger      // Flag exists in EMEVD
result.event_graph.trigger_context  // "boss_defeat", "grace_discovery", etc.
result.event_graph.confidence_boost // +0.1 when found
```

### Files Modified
- `src/discovery/corroboration.rs`: Added EventGraphValidation, integration methods
- `src/discovery/cli.rs`: Added event-graph command, enhanced corroborate output
- `src/discovery/mod.rs`: Added EventGraphValidation export
- `docs/CHANGELOG.md`: Version 0.4.24
- `Cargo.toml`: Bumped to 0.4.24

---

## v0.4.23 - EMEVD Event Graph Extraction System

### Features
- **New extraction system**: Parses all 587 EMEVD files to build queryable event graph
- **Python extraction script** (`scripts/extract_event_graph.py`):
  - Parses `common_func.emevd.js` for event templates (183 templates)
  - Parses `common.emevd.js` for known progression chains
  - Processes all map EMEVD files for flag triggers and dependencies
  - Outputs structured JSON for Rust consumption

- **Rust loader module** (`src/discovery/event_graph.rs`):
  - O(1) flag trigger lookup via HashMap indexes
  - Dependency graph traversal methods
  - Entity-to-flag mapping queries
  - Progression chain lookup (remembrances, map fragments)
  - Validation evidence API for formula verification

### Extraction Results
- **6,161 unique flags** extracted with trigger information
- **13,612 total triggers** (SetEventFlagID calls)
- **1,932 dependency relationships** (EventFlag conditions)
- **378 entity mappings** (boss/grace entities to flags)
- **92 progression chains** (remembrances, map fragments)

### Key Methods
```rust
// Validate flag existence via SetEventFlagID evidence
graph.has_trigger(flag_id) -> bool

// Get trigger context (boss_defeat, grace_discovery, etc.)
graph.get_trigger_context(flag_id) -> Option<&str>

// Find remembrance chain by boss defeat flag
graph.find_remembrance_chain(9100) -> Option<&ProgressionChain>
```

### Files Created
- `scripts/extract_event_graph.py`: Python extraction (~400 lines)
- `scripts/event_graph.json`: Generated graph data (6.1 MB)
- `src/discovery/event_graph.rs`: Rust loader module (~460 lines)

### Files Modified
- `src/discovery/mod.rs`: Added event_graph module export
- `Cargo.toml`: Bumped to 0.4.23

---

## v0.4.22 - Documentation Restructuring & Verification Framework DRY Refactor

### Documentation Restructuring
- **CLAUDE.md reduced 86%**: From 299 to 41 lines by removing duplicated content
  - Kept: Commit protocol, knowledge resources, third-party warnings, slot descriptions
  - Added: Technical documentation reference table pointing to dedicated docs
  - Removed: All technical details already documented in docs/*.md

- **New `docs/ARCHITECTURE.md`**: Persistent architecture reference (237 lines)
  - Single source of truth hierarchy diagram
  - Module structure and import patterns
  - Script migration checklist and examples
  - Key principles for avoiding duplication

- **Updated `docs/discovery-verification-cycle.md`**:
  - Added Phase 6: Corroboration Validation (dual-formula + inseparable evidence)
  - Added Industry Best Practices section
  - Added cross-references to related documentation

### Verification Framework DRY Refactor
- **New `scripts/verification/constants.py`**: Save file structure constants only
  - SLOT_0_OFFSET, SLOT_SIZE, EVENT_FLAGS_SIZE, etc.
  - Clear docstring: validation flags and block bases come from ground_truth_loader

- **New `scripts/verification/utils.py`**: Shared utility functions (449 lines)
  - `read_slot_data()`, `detect_event_flags_start()`, `extract_event_flags()`
  - `check_flag()` with automatic formula selection
  - `is_0xff_padding()`, `multi_slot_differential()` for verification
  - Uses ground_truth_loader for all offset calculations

- **Updated `scripts/verification/__init__.py`**: Version 2.0.0
  - Exports all new modules
  - Documents architecture in module docstring
  - Maintains backward compatibility with legacy modules

- **New `scripts/verification/archive/`**: Directory for superseded scripts
  - README explaining archival criteria

- **Migrated `verify_tile_formula.py`**: Example migration to shared modules

### Architecture Principles Established
- `ground_truth_offsets.json` is the single source of truth for all offsets
- `ground_truth_loader.py` provides Python API to access ground_truth
- `constants.py` contains ONLY save file structure (not verification data)
- `utils.py` combines both into unified API for verification scripts

### Files Modified
- `CLAUDE.md`: Reduced to 41 lines with docs reference table
- `docs/ARCHITECTURE.md`: New - system architecture documentation
- `docs/discovery-verification-cycle.md`: Added Phase 6 and best practices
- `scripts/verification/constants.py`: New - save file structure constants
- `scripts/verification/utils.py`: New - shared utility functions
- `scripts/verification/__init__.py`: Version 2.0.0 with new exports
- `scripts/verification/archive/README.md`: New - archive directory docs
- `scripts/verification/verify_tile_formula.py`: Migrated to shared modules

---

## v0.4.21 - Fix Block 71000 Stormveil Grace Offsets

### Database Fix
- **Block 71000 (Stormveil Graces)**: Corrected base offset from 2673 to 9315
  - Previous base showed only 3/9 graces, new base shows 8/9 graces
  - Flag 71008 (Stormveil Main Gate) now correctly detected as SET
  - Verified via full search across bases 0-15000 with differential slot analysis

### Key Finding
- Grace blocks are NOT contiguous in memory:
  - Block 71000 (Stormveil) at base 9315
  - Block 71800 (Tutorial) at base 2725
  - These are stored ~6590 bytes apart despite sequential flag IDs

### Files Modified
- `ground_truth_offsets.json`: Updated block 71000 base_offset and all 71000-71008 flag entries
- `docs/SAVE_FILE_GROUND_TRUTH.md`: Updated block table and key findings
- `docs/CHANGELOG.md`: Added version entry

---

## v0.4.20 - UI Improvements and Verification Updates

### UI Improvements
- **Category filter overflow**: Fixed verification page category filters to wrap instead of overflow (changed to `horizontal_wrapped`)
- **Smaller monospace fonts**: Reduced table monospace font size from 12px to 9px (75% reduction) for better density
- **Consolidated styling**: Created `src/ui/style.rs` with shared `TABLE_MONO_SIZE` constant used across 10 view files
- **File dialog memory**: Open/save dialogs now remember the last used directory

### Verification Framework Updates
- Updated Rust code to use renamed correlation file (`flag-correlation-candidates.jsonl`)
- Updated field names in `VerificationRecord`:
  - `manual_status` → `user_marked_complete` (with serde alias for compatibility)
  - `auto_status` → `webapp_parsed_status`
  - `matches` → `statuses_align`

### Files Modified
- `src/ui/style.rs`: New shared style constants module
- `src/ui/verification_view.rs`: Category filter wrapping, style imports
- `src/ui/events.rs`, `src/ui/event_flags_db_view.rs`, `src/ui/world_pickups_view.rs`: Monospace size
- `src/ui/equipment.rs`, `src/ui/general.rs`, `src/ui/stats.rs`: Monospace size
- `src/ui/npcs_view.rs`, `src/ui/spells_view.rs`, `src/ui/shop_items_view.rs`: Monospace size
- `src/main.rs`: File dialog directory memory
- `src/util/verification_records.rs`: Field name updates
- `src/vm/verification_vm.rs`, `src/vm/slot.rs`: Field references
- `src/discovery/ground_truth_probe.rs`, `src/discovery/cli.rs`, `src/discovery/test_cases.rs`: Field names

---

## v0.4.19 - Major Block Base Corrections

### Critical Fixes
Three block bases were found to be completely incorrect (0% match against actual save data):

| Block | Category | Old Base | New Base | Evidence |
|-------|----------|----------|----------|----------|
| 62000 | Map Fragments | 1500 | **9359** | 12/12 match + negative validation |
| 65000 | Crystal Tears | 1875 | **37412** | 15/15 match + negative validation |
| 67000 | Cookbooks | 2280 | **37411** | 34/34 match + negative validation |

### Methodology: Multi-Slot Validation
- **Positive evidence**: Slot 0 (mid-game Confessor) - all confirmed items show as SET
- **Negative evidence**: Slot 1 (early-game Wretch) - all items show as UNSET
- Both conditions required for verification

### Key Finding
The old bases (1500-2280) were in the typical block range but gave 0% match.
The correct bases (9359-37412) are in higher ranges, suggesting these item categories
use a different storage region than grace/progression flags.

### New Verification Scripts
- `probe_wide_search.py`: Search entire event_flags section for bases
- `probe_maps_with_negatives.py`: Validate with positive AND negative evidence
- `probe_items_with_negatives.py`: Multi-slot validation for items
- `compare_bases.py`: Compare old vs new bases side-by-side
- `validate_map_fragments.py`: Inseparable evidence validation for maps
- `verify_map_base_multi_slot.py`: Cross-character validation

### Files Modified
- `ground_truth_offsets.json`: Corrected bases for blocks 62000, 65000, 67000, 68000
- `Cargo.lock`: Updated from build
- Added 7 new verification scripts

---

## v0.4.18 - Correlation Schema Updates

### Schema Alignment
- Updated all verification scripts to use renamed file `flag-correlation-candidates.jsonl`
  - Previously named `verification-records.jsonl`
  - Better reflects the file's purpose as correlation candidates, not verified records

### Field Name Updates
All scripts updated to use new field names from elden-map webapp:
- `manualStatus` → `userMarkedComplete` (user manually marked flag as complete)
- `autoStatus` → `webappParsedStatus` (webapp's formula detection result)
- `matches` → `statusesAlign` (whether user and webapp agree)

### Documentation Fixes
- Fixed VM grace base in VERIFICATION-LEADS.md (2726 → 2825)
- Fixed inconsistent Area 16 status in CORROBORATION-SYSTEM.md (was "verified", now "disproven")

### Files Modified
- `scripts/run_verification.py`: Updated paths and help text
- `scripts/verify_from_jsonl.py`: Updated paths and field references
- `scripts/discover_block_bases.py`: Updated path and field references
- `scripts/verification/*.py`: All scripts updated with new schema
- `docs/VERIFICATION-LEADS.md`: Fixed filename and base offset references
- `docs/CORROBORATION-SYSTEM.md`: Fixed inconsistent Area 16 status

---

## v0.4.17 - Volcano Manor Grace Sub-Block Discovery

### Critical Fix
- **Block 71600 discovered**: Volcano Manor graces use different base than tutorial graces
  - Flag 71607 (Subterranean Inquisition Chamber) empirically at byte 2825, bit 0
  - Sub-block 71600-71699 uses base 2825 (corrected from initial 2750 discovery)
  - User confirmed grace SET, but formula returned NOT SET - probing found correct location
  - Block 71000 has **discontinuous allocation** - different sub-ranges use different bases

### Technical Improvement
- **Sub-block support added** to `calculate_block_flag_offset()`
  - Now checks 100-flag granularity first (e.g., 71600)
  - Falls back to 1000-flag granularity if no sub-block found (e.g., 71000)
  - Enables future sub-block discoveries without code changes

### New Verification Scripts
- `scripts/verification/verify_grace_blocks.py`: Cross-validate grace blocks
- `scripts/verification/probe_vm_graces_extended.py`: Probe VM grace locations
- `scripts/verification/probe_grace_71607.py`: Find correct 71607 offset

### Files Modified
- `ground_truth_offsets.json`: Added 71600 sub-block, marked 71000 as partial
- `build.rs`: Added sub-block handling to code generator
- `docs/CHANGELOG.md`: v0.4.17

---

## v0.4.16 - Inseparable Evidence Methodology & Area 16 Disproven

### Critical Fix
- **Area 16 (Volcano Manor) base disproven**: Base 36737 (slot 29) reads unrelated data
  - Inseparable evidence test: 16000800 (Rykard defeat) showed SET, but grace 71600 showed NOT SET
  - User confirmed character has not defeated Rykard
  - Byte at 36837 (0xFF) is unrelated data, not Rykard defeat flag
  - Area 16 marked as "disproven" with base_offset = 0

### New Methodology: Inseparable Evidence
- **Inseparable flags**: Flags that cannot be set individually in normal gameplay
- **Boss-grace pairs**: Boss defeat flag + post-boss grace must be consistent
- Cross-validation catches false positives from formulas reading wrong data
- Documented in `docs/CORROBORATION-SYSTEM.md`

### Documentation
- **Boss Remembrance System**: Complete mapping of boss defeat → remembrance → pickup chains
  - Event 1100 awards progression items (Talisman Pouch), NOT remembrances
  - 91xx flags trigger Event 1100 on boss death
  - 510xxx flags track remembrance pickups
- **Inseparable Evidence Methodology**: Validation technique for dungeon base verification

### New Verification Scripts
- `scripts/verification/verify_boss_chain.py`: Validates boss defeat → remembrance pickup chains
- `scripts/verification/verify_rykard_chain.py`: Rykard-specific chain verification

### Files Modified
- `ground_truth_offsets.json`: Area 16 marked as disproven
- `docs/CORROBORATION-SYSTEM.md`: Added inseparable evidence methodology
- `Cargo.toml`: Bumped to 0.4.16

---

## v0.4.15 - Tile Formula Correction & Legacy Dungeon Base Discovery

### Critical Fix
- **Tile formula base_offset corrected**: Changed from 485330 to **489981** (+4651 bytes)
  - Verified empirically via Smoldering Butterfly pickup temporal diff
  - Flag 1043500010 confirmed at byte 857482 in event_flags section
  - This fixes all tile flag calculations for base game world pickups

### Database Expansion
- **Legacy dungeon bases discovered** using `legacymap.eventflagalloclist` slot formula:
  - Formula verified: `base = 4112 + slot × 1125` matches Areas 14 (29987) and 18 (43487) exactly
  - Area 11 (Leyndell): 8612 (slot 4)
  - Area 12 (Underground): 15362 (slot 10)
  - Area 13 (Leyndell Royal Capital): 26612 (slot 20)
  - Area 15 (Miquella's Haligtree): 33362 (slot 26)
  - Area 16 (Volcano Manor): 36737 (slot 29)
  - Area 19 (Chapel of Anticipation): 46862 (slot 38)
  - Area 34 (Divine Towers): 60362 (derived from section 10 at slot 60)
  - Area 35 (Mohgwyn Palace): 50237 (slot 41)
  - Area 39 (Elden Throne): 31112 (derived from section 20 at slot 44)

### Test Case Expansion
- Added 38 confirmed test cases from verification-records.jsonl (Slot 0, Confessor)
  - 34 block flags (graces, cookbooks, progression)
  - 4 dungeon flags (Stormveil bosses and pickups)

### New Verification Scripts
- `scripts/verification/verify_tile_formula.py`: Proper tile formula verification with slot/event_flags extraction
- `scripts/verification/extract_test_cases.py`: Extracts confirmed test cases from JSONL verification data

### Key Finding
- Web app (elden-map) uses different formula constants than our Rust project
- `computedByteOffset` values in verification-records.jsonl cannot be used directly
- `matches` field is still valuable for confirming flag states

### Files Modified
- `ground_truth_offsets.json`: Updated tile formula and all dungeon bases
- `src/db/pickup_flags.rs`: Updated test assertions for corrected base
- `src/discovery/offset_probe.rs`: Updated hardcoded tile base
- `src/discovery/test_cases.rs`: Added 38 confirmed test cases
- `scripts/verification/flag_formulas.py`: Synced tile base constant
- `docs/SAVE_FILE_GROUND_TRUTH.md`: Updated tile formula documentation
- `Cargo.toml`: Bumped to 0.4.15

---

## v0.4.14 - Area 14 = Tutorial Areas Discovery

### Key Discovery
- **Area 14 is Tutorial Areas, NOT Shunning-Grounds**
  - Chapel of Anticipation, Cave of Knowledge, and Stranded Graveyard all write to Area 14 (offset 29987)
  - Verified from 6,722 unique flags across Slot 6 Chapel and Slot 1 Cave empirical data
  - Areas 19/20 offsets from code appear unused for tutorial events

### Bug Fixes
- **Fixed reverse lookup priority**: Block flags now checked BEFORE simple flags
  - Prevents misidentification of flags in 2500-3500 byte range
  - Example: byte 2625 correctly identified as block 71000, not simple flag 21000

### Features
- **Dynamic slot mapping**: snapshot_batch.rs now handles "Slot X" pattern dynamically
  - Added "wr1" => 6, "sam" => 5 character mappings

### Documentation
- **Block overlaps documented**: Flag-islands.md now explains non-contiguous storage
  - Blocks 60000, 71000, 72000, 73000 have overlapping byte ranges
  - Not a bug - reflects FromSoft's flag allocation strategy
- **EVENT-FLAG-GEOGRAPHY.md**: Corrected Area 14 from "Shunning-Grounds" to "Tutorial Areas"

### Files Modified
- `ground_truth_offsets.json`: Updated Area 14, 19, 20 with corrected notes
- `src/discovery/reverse_lookup.rs`: Fixed block flag priority
- `src/discovery/flag_catalog.rs`: Changed Area 14 label to "Tutorial Event"
- `src/discovery/snapshot_batch.rs`: Added dynamic slot mapping
- `docs/Flag-islands.md`: Added block overlap documentation
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Corrected Area 14 documentation
- `Cargo.toml`: Bumped to 0.4.14

---

## v0.4.13 - Area 19/20 Formula Investigation

### Features
- **Area 19 (Elden Throne)**: Added formula base_offset=1426125 (derived from event_flags.rs)
  - Corrected: Area 19 is NOT Chapel of Anticipation - it's the final boss area
  - Contains Radagon/Elden Beast defeat flag (19000810)
  - Status: needs_review (no empirical verification yet)
- **Area 20 (Stranded Graveyard)**: Added formula base_offset=2500000 (derived from event_flags.rs)
  - Tutorial dungeon events (20007xxx flags)
  - Status: needs_review

### Key Findings
- **Chapel of Anticipation** shares Area 10 (Stormveil Castle) flags, NOT Area 19
  - Grafted Scion boss uses flag 10010800
- Tutorial grace flags (71800, 71801) use Block 71000, not dungeon areas

### Documentation Updates
- **EVENT-FLAG-GEOGRAPHY.md**: Updated Special Areas table with correct names
- **flag_catalog.rs**: Changed "Chapel Event" → "Elden Throne Event" with clarifying comment

### Data Collection Issues Identified
- Stranded Graveyard save snapshots (Wretch 11-12) are identical - snapshot wasn't captured correctly
- Grace flag 71800 not captured due to snapshot pairing limitations

### Files Modified
- `ground_truth_offsets.json`: Added Areas 19, 20 with needs_review status
- `src/discovery/flag_catalog.rs`: Fixed Area 19 UI label
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Updated Special Areas table
- `Cargo.toml`: Bumped to 0.4.13

---

## v0.4.12 - Dungeon Area Formula Verification

### Features
- **Area 14 (Shunning-Grounds)**: Verified base_offset=29987 with 1968/1968 flags matching (100%)
- **Area 18 (Roundtable Hold)**: Verified base_offset=43487 with 176/176 flags matching (100%)
- **Area 11 (Raya Lucaria)**: Identified base_offset=4112 (same as Stormveil), 172/187 match (92%), marked needs_review

### Documentation Updates
- **EVENT-FLAG-GEOGRAPHY.md**: Major restructure
  - Fixed terminology: Legacy Dungeons vs Minor Dungeons vs Special Areas
  - Corrected flag format from `XXYYYZZZZ` to `AASSZZZZ`
  - Fixed Area 18 = Roundtable Hold (was incorrectly documented as Area 19)
  - Added verification status for all dungeon areas
  - Added Flag Format Summary table
  - Reorganized World Hierarchy diagram

### Dungeon Area Name Corrections
| Area | Old Name | Correct Name |
|------|----------|--------------|
| 11 | Leyndell | Academy of Raya Lucaria |
| 13 | Farum Azula | Leyndell, Royal Capital |
| 14 | Raya Lucaria | Shunning-Grounds (Sewers) |
| 15 | Caria Manor | Miquella's Haligtree |
| 16 | Volcano Manor | Crumbling Farum Azula |

### Tests Added
- `test_verified_dungeon_shunning_grounds()`: Area 14 formula validation
- `test_verified_dungeon_roundtable()`: Area 18 formula validation

### Files Modified
- `ground_truth_offsets.json`: Updated Areas 11, 13, 14, 15, 16, 18 with correct names and offsets
- `src/db/pickup_flags.rs`: Added 2 new dungeon verification tests
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Restructured hierarchy and terminology
- `Cargo.toml`: Bumped to 0.4.12

---

## v0.4.11 - Tile Formula Base Offset Correction

### Fixes
- **Tile formula base offset**: Corrected from 337359 to 485330
  - Analysis of 69 empirical flags from discoveries.json showed consistent +147971 byte offset difference
  - Confirmed via flag 1041740610 (byte_offset=803906 matches formula exactly)
  - All 69 tile flags now calculate correct offsets

### Technical Details
- **Root cause**: Previous base offset (337359) was incorrectly derived
- **Verification method**: Cross-referenced all tile flags in discoveries.json against calculated offsets
- **Result**: 100% match rate after correction

### Tests Added
- `test_tile_confirmed_empirical()`: Validates empirically confirmed flag 1041740610
- Updated `test_tile_flag_formula_verified()` with corrected expected values

### Files Modified
- `ground_truth_offsets.json`: Updated tile_formula.base_offset to 485330, added proven tile flag
- `src/db/pickup_flags.rs`: Updated comment, fixed test values, added new test
- `src/discovery/offset_probe.rs`: Updated tile_base constant from 495830 to 485330
- `src/generated/ground_truth.rs`: Auto-regenerated with correct value
- `Cargo.toml`: Bumped to 0.4.11

---

## v0.4.10 - UI Unverified Indicator Fix

### Fixes
- **Unverified indicator position**: Moved "!" indicator from end of row to directly after status brackets
  - Before: `[X] | Grace Name | Region | 76100!`
  - After: `[X]! | Grace Name | Region | 76100`
- **Import cleanup**: Removed unused `ScrollArea` and `VerificationStatus` imports from events.rs

### Files Modified
- `src/ui/events.rs`: Updated `display_event_row()` to insert "!" after brackets
- `Cargo.toml`: Bumped to 0.4.10

---

## v0.4.9 - Block and Dungeon Formula Verification

### Features
- **Block Formula Verification**: Verified 5 previously unverified block bases
  - Block 65000 (Whetblades): Verified via hardcoded offsets (65610=0x79f, 65700=0x7aa, 65720=0x7ad)
  - Block 72000 (DLC Enir-Ilim graces): 10+ consistent proven flags
  - Block 74000 (DLC dungeon graces): 8+ consistent proven flags
  - Block 75000: Marked as "calculated" (no known flags in range)
  - Block 78000 (Grace guidance): 8+ proven flags (78210=3526, 78304=3538, etc.)

- **Dungeon Formula Verification**: Verified Area 30 (Catacombs)
  - Corrected from "needs_review" to "verified" status
  - 7 boss defeat flags matched formula (30020800=29761, 30030800=30886, etc.)
  - Confirmed base_offset=27411, section_size=1125

- **Verification Tests**: Added 6 new tests
  - `test_block_65000_whetblades_verified`
  - `test_block_72000_dlc_graces_verified`
  - `test_block_74000_dlc_dungeon_graces_verified`
  - `test_block_78000_grace_guidance_verified`
  - Updated `test_verified_dungeon_catacombs` with proven boss flags

### Verification Status Summary
| Formula Type | Verified | Calculated | Unverified |
|--------------|----------|------------|------------|
| Block bases  | 10       | 3          | 0          |
| Dungeon areas| 4        | 0          | 11         |
| Tile formula | 1        | 0          | 0          |

### Files Modified
- `ground_truth_offsets.json`: Updated status for 6 blocks/areas
- `src/db/pickup_flags.rs`: Added 6 verification tests
- `Cargo.toml`: Bumped to 0.4.9

---

## v0.4.8 - Enhanced Corroboration with Chain Validation

### Features
- **Chain Data Module** (`src/discovery/chain_data.rs`):
  - Boss defeat chains: 10 major bosses with defeat→remembrance→great rune→activation flag sequences
  - Area prerequisites: 6 late-game areas (Consecrated Snowfield, Haligtree, Leyndell, Farum Azula, etc.)
  - Geographic regions: 17 regions with landmark ranges, tile coordinates, grace ranges, map fragments
  - Scroll unlocks: 10 scroll/prayerbook→spell unlock chains
  - Verified block bases: 10 block base offsets for cross-validation

- **New RelationshipTypes**:
  - `BossDefeatChain`: Validates boss defeat → remembrance → great rune → activation consistency
  - `AreaPrerequisite`: Validates late-game flags have required prerequisites
  - `GeographicProximity`: Soft correlation for flags in same region
  - `ScrollUnlock`: Scroll pickup enables spell availability

- **Enhanced Corroboration Engine**:
  - `check_boss_chain()`: Detects contradictions like "Remembrance set but boss not defeated"
  - `check_area_prerequisite()`: Detects "Haligtree flag set without medallion halves"
  - `check_geographic_correlation()`: Regional flag correlation analysis
  - New result types: `BossChainResult`, `AreaPrerequisiteResult`, `GeographicCorrelationResult`

### Chain Validation Examples
| Chain Type | Validation | Contradiction Detection |
|------------|------------|------------------------|
| Boss | Godrick defeat (171) → Remembrance (9101) → Great Rune (160) → Activation (180) | Activation without possession |
| Area | Medallion halves (60430, 60431) → Consecrated Snowfield (62550+) | Late-game flags without prereqs |
| Geographic | Limgrave landmarks (62100-62138) correlate with Limgrave graces (76100-76199) | Soft validation |

### Files Added
- `src/discovery/chain_data.rs`: Static chain data and helper functions

### Files Modified
- `src/discovery/relationship_graph.rs`: 4 new RelationshipTypes
- `src/discovery/corroboration.rs`: 3 new validation methods, 3 new result types
- `src/discovery/mod.rs`: Exports for chain_data module

---

## v0.4.7 - Landmark Integration & Event Flag Geography

### Features
- **Landmark Category in Event Flags DB**: Added Landmark (62xxx) as a filterable category
  - 308 landmarks from LANDMARKS lookup table imported into database
  - Region resolution based on flag ID ranges (Limgrave, Liurnia, Caelid, etc.)
  - Light blue color coding in UI (RGB 180,220,255)
  - New filter button in category row

- **Event Flag Geography Documentation** (`docs/EVENT-FLAG-GEOGRAPHY.md`):
  - Complete world hierarchy (Regions → Sub-regions → Landmarks/Graces/Dungeons)
  - Geographic flag groupings (tile system, block-based, legacy dungeons)
  - Flag chaining systems (quests, area unlocks, merchant purchases, boss rewards)
  - Source game file reference with paths

### Bug Fixes
- **Fixed ~200 landmark byte offsets**: Flags 62100-62981 had incorrect offsets
  - Was using wrong formula `flag_id / 8` instead of `base_offset + (flag_id - block_start) / 8`
  - Old offsets: 0x1e52-0x1e73 (~7762-7795 bytes)
  - New offsets: 0x5e8-0x656 (~1512-1622 bytes)
  - Block 62000 base offset confirmed as 0x5dc (1500)

### Files Added
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Comprehensive event flag system documentation
- `src/db/landmarks.rs`: Landmarks lookup table module

### Files Modified
- `src/db/event_flags.rs`: Corrected 62100-62981 byte offsets
- `src/db/event_flags_db.rs`: Added Landmark category, get_landmark_region(), LANDMARKS import
- `src/db/mod.rs`: Export landmarks module
- `src/ui/event_flags_db_view.rs`: Added Landmark filter and color

---

## v0.4.6 - Multi-Point Corroboration System

### Features
- **Relationship Graph Module** (`src/discovery/relationship_graph.rs`):
  - Loads 2,796 flag relationships across 5,079 flags from `scripts/flag_relationships.json`
  - Indexes relationships by source, target, and type for O(1) lookups
  - Extracts 122 dual-formula corroboration pairs (tile↔block)
  - Supports 6 relationship types: pickup_sets_flag, enables_purchase, grace_discovery, boss_remembrance, event_sequence, map_fragment

- **Corroboration Engine** (`src/discovery/corroboration.rs`):
  - Multi-point validation using relationship graph
  - Dual-formula validation: cross-checks tile flag (10-digit) with block flag (5-digit) for same item
  - Confidence scoring with agreement ratios
  - Batch validation of all corroboration pairs

- **New CLI Commands**:
  - `discovery corroborate <flag_id>` - Single flag validation with related flag checks
  - `discovery corroborate --all` - Batch validate all 122 corroboration pairs
  - `discovery graph` - Show relationship graph statistics

- **Flag Extraction Script** (`scripts/extract_flag_relationships.py`):
  - Extracts flag relationships from decompiled game files
  - Parses ItemLotParam_map, ShopLineupParam, BonfireWarpParam, common.emevd.js
  - Generates `flag_relationships.json` for runtime use

### Bug Fixes
- **Tile formula col_base corrected**: Changed from 42 to **30**
  - Actual column range is 30-58, formula was excluding columns 30-41
  - Discovered through corroboration analysis showing contradictions
  - Fixed in `ground_truth_offsets.json`

- **Bit mask bug in corroboration**: Changed `(1 << (7 - bit))` to `(1 << bit)`
  - Bit was already calculated as `7 - (flag % 8)`, double-negation caused wrong bit reads
  - Affected check_dual_formula, read_flag, and validate_all_pairs methods

### Validation Results
| Slot | Character | Agreements | Contradictions | Status |
|------|-----------|------------|----------------|--------|
| 0 | Confessor (mid-game) | 57 | 5 | Expected (4 world pickups + 1 shop) |
| 1 | Wretch (early-game) | 62 | 0 | Formula validated |

### Files Added
- `src/discovery/relationship_graph.rs`: Relationship graph loader and indexer
- `src/discovery/corroboration.rs`: Multi-point validation engine
- `scripts/extract_flag_relationships.py`: Game data extraction script
- `scripts/flag_relationships.json`: 2,796 relationships, 5,079 flags
- `tests/regression_suite.rs`: Ground truth schema validation tests

### Files Modified
- `src/discovery/mod.rs`: Export new modules
- `src/discovery/cli.rs`: Added corroborate and graph commands
- `ground_truth_offsets.json`: Fixed tile formula col_base (42→30)

---

## v0.4.5 - Dynamic Test Validation & UI Improvements

### Features
- **Dynamic Test Case Loading**: Test cases now load from verification records instead of hardcoded values
  - `DynamicTestCaseValidator` loads expectations from JSONL file
  - `--dynamic` or `--records <path>` flags for CLI validation
  - Adapts automatically when verification records are updated
  - `build_test_suite_from_records()` function for programmatic use

### UI Improvements
- **Catppuccin Frappé color palette** for verification view
  - Consistent colors: Red (#e78284), Green (#a6d189), Yellow (#e5c890), Peach (#ef9f76), Teal (#81c8be)
- **Monospace font size reduced to 85%** (12px) for better table density
- **Removed text truncation** - full flag names now visible with horizontal scrolling

### Bug Fixes
- Fixed verification records path: now correctly points to `verification-records.jsonl`

### Files Modified
- `src/discovery/test_cases.rs`: Added DynamicTestCaseValidator, build_test_suite_from_records()
- `src/discovery/cli.rs`: Added --dynamic, --records flags, Validator trait
- `src/ui/verification_view.rs`: Catppuccin Frappé palette, font sizing, no truncation
- `src/main.rs`: Fixed verification records path

---

## v0.4.4 - Block Offset Corrections

### Bug Fixes
- **Fixed block 76000 base offset**: Changed from 3248 to **3250** (was off by 2 bytes)
  - Root cause: Previous fix in v0.4.3 used wrong base offset
  - Validation showed 76101 (The First Step) returning FALSE for Wretch when it should be TRUE
  - Cross-referenced with elden-map verification tool to confirm correct offset

### CLI Improvements
- Added `--save <path>` parameter to `discovery validate` and `discovery probe` commands
- Commands now support custom save file paths instead of hardcoded default

### Test Case Updates
- Simplified test cases to only include reliably verifiable flags
- Removed unstable Confessor entries where save data has changed since verification
- All 6 slots now pass 100% validation (15/15 tests)

### Cross-Project Sync
- Synced block 73000 base offset fix to elden-map (2875 → 2662)
  - Updated `elden-map/server/src/verificationService.ts`
  - Updated `elden-map/server/src/eventFlagService.ts`

### Files Modified
- `ground_truth_offsets.json`: Block 76000 base_offset 3248 → 3250
- `src/discovery/cli.rs`: Added --save parameter parsing
- `src/discovery/test_cases.rs`: Simplified to verified flags only

---

## v0.4.3 - Test Case Validation System

### Features
- **Test Case Validator**: Curated test cases for verifying flag offset formulas
  - `FlagTestCase` struct with category, verification method, expected state
  - `SlotTestSuite` for per-character test suites
  - `TestCaseValidator` for running validation against save files
  - Helper functions: `grace()`, `world_pickup()`, `boss_defeat()`, `cookbook()`

- **CLI Commands**:
  - `discovery validate <slot> [slot...]` - Run curated test cases
  - `discovery validate --all` - Validate all defined slots
  - `discovery probe <slot> <offset>...` - Direct byte inspection for debugging

### Bug Fixes
- **Fixed 29 incorrect flag offsets** in `ground_truth_offsets.json` for 76xxx grace flags
  - All 76xxx flags were consistently 2 bytes off from correct formula
  - Root cause: Individual entries were added independently without verifying against block base
  - Fixed by recalculating offsets from verified block base (76000 → 3248)

### Verification Results
- The First Step (76101) validates correctly @ 0xcbc:2 = TRUE across slots 2, 3, 4
- Test case system distinguishes true positives from false negatives

### Files Created
- `src/discovery/test_cases.rs`: Test case infrastructure

### Files Modified
- `src/discovery/cli.rs`: Added validate and probe commands
- `src/discovery/mod.rs`: Export test_cases module
- `ground_truth_offsets.json`: Corrected 76xxx flag offsets

---

## v0.4.2 - Expanded Flag Catalog

### Features
- **Expanded Flag Catalog**: Increased from 7,034 to 22,376 documented flags
  - Extracted 5,047 flags from ItemLotParam_map.param.xml
  - Extracted 1,291 flags from ShopLineupParam.param.xml
  - Extracted 15,921 flags from event scripts (*.emevd.js)

- **Automatic Name Generation**: All discovered flags now get descriptive names
  - Pattern-based naming for undocumented flags (e.g., "Sewers Event 8642")
  - Dungeon/region prefixes: Stormveil, Raya Lucaria, Sewers, Cave, etc.
  - World pickup names include map tile coordinates
  - Catalog lookup takes precedence when flag is documented

### Technical Details
- `FlagCatalog::get_name_or_generate()` provides fallback naming
- `FlagCatalog::generate_flag_name()` maps ID patterns to descriptive names
- Batch analysis now loads catalog once and passes to all operations

### Files Created
- `scripts/expand_flag_catalog.py`: Extraction tool for expanding catalog

### Files Modified
- `scripts/extracted_event_flags.json`: Expanded from 7,034 to 22,376 flags
- `src/discovery/flag_catalog.rs`: Added name generation methods
- `src/discovery/integration.rs`: Use `get_name_or_generate()` for lookups
- `src/discovery/snapshot_batch.rs`: Load catalog for batch processing

---

## v0.4.1 - Discovery CLI Commands

### Features
- **CLI Interface**: Run discovery operations from command line
  - `discovery batch-analyze`: Process all snapshot pairs and persist discoveries
  - `discovery status`: Show discovery store statistics and consensus report
  - `discovery promotable`: List discoveries ready for promotion
  - `discovery promote [--dry-run]`: Promote confirmed discoveries to ground truth

### Usage
```bash
# Process snapshots
cargo run -- discovery batch-analyze

# Check status
cargo run -- discovery status

# Preview promotions
cargo run -- discovery promote --dry-run
```

### Files Created
- `src/discovery/cli.rs`: CLI command handlers

### Files Modified
- `src/main.rs`: CLI argument detection before GUI launch
- `src/discovery/mod.rs`: Added cli module export

---

## v0.4.0 - Event Flag Discovery System

### Features
- **Flag Catalog Integration**: Load and index 7,034 flags from `extracted_event_flags.json`
  - Search by name with multi-word query support
  - Autocomplete functionality for flag lookup
  - Category and region-based lookups (39 categories, 158 regions)

- **Discovery Store**: Persistent storage with full provenance tracking
  - Observations tracked with source type: SnapshotDiff, ProbeResult, CrossSlotValidation, ManualVerification
  - Status pipeline: Pending → Confirmed → Promoted (or Rejected)
  - Automatic consensus recalculation when observations are added
  - Persists to `discoveries.json`

- **Batch Snapshot Analyzer**: Process all granular before/after save snapshots
  - Parses filenames to extract character, sequence number, action description
  - Groups files into before/after pairs automatically
  - Runs differential discovery on each pair

- **Consensus Engine**: Multi-observation consensus with weighted voting
  - Source weights: Manual verification (1.0), Cross-slot (0.95), Snapshot diff (0.85), Probe (0.7)
  - Configurable thresholds: min 2 observations, 80% agreement to confirm
  - Reports contested vs confirmed discoveries

- **Cross-Slot Validator**: Validate discoveries across multiple save slots
  - Checks same offset/bit across different character slots
  - Confidence adjustments based on agreement/disagreement
  - Supports batch validation

- **Ground Truth Updater**: Safe automated updates to `ground_truth_offsets.json`
  - Timestamped backups before any modification
  - Block base recalculation when enough flags confirmed
  - Rollback capability

### Technical Details
- Consensus requires: 2+ observations, 80%+ agreement, 75%+ confidence for promotion
- Finding one verified flag in a block unlocks ~125 adjacent flags (block formula)
- 41 unit tests added (7 integration tests require save files)

### Files Created
- `src/discovery/flag_catalog.rs`: Flag catalog loader and search
- `src/discovery/discovery_store.rs`: Persistent discovery storage
- `src/discovery/snapshot_batch.rs`: Batch snapshot processor
- `src/discovery/consensus.rs`: Consensus building engine
- `src/discovery/cross_validator.rs`: Cross-slot validation
- `src/discovery/ground_truth_updater.rs`: Safe ground truth updates

### Files Modified
- `src/discovery/mod.rs`: Added new module exports
- `src/discovery/offset_probe.rs`: Added persistence hooks
- `src/discovery/integration.rs`: Added persistence-enabled workflows
- `Cargo.toml`: Added chrono dependency for timestamps

---

## v0.3.4 - Verification Integration & Detection Categories

### Features
- **Verification moved to Event Flags**: Verification view now integrated as a per-character tab within Event Flags section instead of a standalone database view
  - Loads verification records specific to selected character slot
  - Per-slot loading state tracked with `verification_loaded_slots: [bool; 10]`

- **Detection category refactor**: Renamed misleading "False Positive" labels to proper detection categories
  - `FormulaError` (RED): manual=true, auto=false - User confirmed collection but formula missed it. **Primary indicator of formula problems**
  - `PendingVerification` (ORANGE): auto=true, manual=false - Formula detected but not manually confirmed. Could be: forgotten, no POI exists, or actual error
  - `UndiscoveredRegion` (YELLOW): Both agree but no graces discovered in region. Informational only

- **Enhanced flagged detection UI**:
  - Color-coded rows by detection category severity
  - Auto-opens section when Formula Errors exist (immediate attention needed)
  - Hover tooltips with detailed descriptions
  - Context menu with copy options and full details
  - Formula error count prominently displayed at top

- **Updated export format**: New fields in verification export
  - `flagged_count`, `formula_error_count`, `informational_count`
  - `flagged_by_category` breakdown
  - `FlaggedDetectionExport` with `detection_category`, `is_error`, `description` fields

### Technical Details
- Verification methodology: Only flags EXPLICITLY marked as complete are in verification file
  - `manual=false` is ambiguous (true negative OR forgotten)
  - `manual=true, auto=false` is the reliable signal for formula errors
- Formula Errors sorted first in flagged list for priority attention
- 45 Formula Errors identified for investigation

### Files Modified
- `src/vm/verification_vm.rs`: Refactored detection categories and methods
- `src/vm/events.rs`: Added `Verification` route and `verification_vm` field
- `src/ui/events.rs`: Added Verification tab to Event Flags
- `src/ui/verification_view.rs`: Updated UI with color coding and auto-open
- `src/ui/menu.rs`: Removed standalone Verification route
- `src/vm/export.rs`: Updated export structures
- `src/vm/slot.rs`: Updated export building
- `src/main.rs`: Per-slot verification loading

---

## v0.3.3 - Improve Event Flags Offset Detection Accuracy

### Critical Bug Fix
- **Negative validation for offset detection**: Fixed false positives in event flags detection
  - Previous algorithm found 396 offsets matching the 4 tutorial grace pattern
  - Was picking first match (0x12CF0) instead of correct offset (0x12D76)
  - Added 6 late-game grace flags as NEGATIVE validation (must NOT be set)
  - Algorithm now: (1) finds all 4/4 positive matches, (2) picks best by negative score
  - Fixes late-game items incorrectly showing as "collected" for early-game characters

### Technical Details
- Detection now uses 3-phase approach:
  1. Find all offsets where ALL positive flags match (early-game graces)
  2. Among those, pick offset with highest negative score (late-game graces NOT set)
  3. Fall back to partial match if no perfect match found

### Files Modified
- `src/save/common/event_flags_detection.rs`: Added `NEGATIVE_VALIDATION_FLAGS`, rewrote detection algorithm

---

## v0.3.2 - Fix Event Flags Offset Detection

### Critical Bug Fix
- **Dynamic event flags offset detection**: Fixed save parser using wrong hardcoded offset
  - Was using `0x1a104` (~107KB) but actual offset is `~0x12B00` (~76KB)
  - Caused massive false positives (55 items showing "collected" when they weren't)
  - Now uses validation-based detection to find correct offset per slot
  - Detection uses anchor flags (tutorial graces) that all characters should have

### Files Modified
- `src/save/common/save_slot.rs`: Replace hardcoded offset with dynamic detection

---

## v0.3.1 - Wire Generated Ground Truth into App

### Bug Fix
- **pickup_flags.rs now uses verified offsets**: The generated ground truth module is now wired into the app's event flag calculations
  - Tile base offset: 347,000 → **495,830** (verified)
  - Block bases: Now uses `VERIFIED_BLOCK_BASES` from JSON
  - Dungeon bases: Uses verified bases for areas 30, 31, 32 (catacombs, caves, tunnels)

### Features
- **Untrackable flag filtering**: LocalId > 6999 now returns `None` (consumables with localId 7000+ cannot be tracked)
- **Status-aware dungeon lookup**: Only uses verified dungeon bases when status is "verified"

### Files Modified
- `src/main.rs`: Added `mod generated;`
- `src/db/pickup_flags.rs`: Imports from generated module, uses verified constants

---

## v0.3.0 - Ground Truth Code Generation & Cross-Project Integration

### Features
- **Code Generation from JSON** (`build.rs`): Generates Rust code from `ground_truth_offsets.json` at compile time
  - `src/generated/ground_truth.rs`: Auto-generated with verified block bases, tile formula, dungeon bases
  - Provides `calculate_block_flag_offset()`, `calculate_tile_flag_offset()`, `calculate_dungeon_flag_offset()`
  - Single source of truth shared between Rust and TypeScript projects

- **TypeScript Integration** (elden-map): Symlink and TypeScript module for web app
  - `ground-truth-formulas.ts`: Type-safe offset calculation functions
  - Imports directly from shared `ground_truth_offsets.json`

- **Character Slot Identification**: Test output now shows character names and per-slot flag status
  - Extracts UTF-16LE names from save slots at variable offsets
  - Display format: `Slot 0 (Confessor): [✓ ✓ ✓ ✓ ✓ ✓]`

- **Formula Test Suite** (`scripts/verification/test_formulas.py`): Comprehensive formula validation
  - Tests block, tile, and dungeon formulas against actual save data
  - Reports per-slot verification status

### Verification Results
- **392 flags proven** (from 656 tested)
- **Block formula**: Verified for 60000, 62000, 67000, 71000, 73000, 76000 ranges
- **Tile formula**: Verified with base offset 495830
- **Dungeon formula**: Verified for areas 30 (catacombs), 31 (caves), 32 (tunnels)

### Files Modified
- `build.rs`: Extended with JSON code generation
- `Cargo.toml`: Added serde_json build dependency, bumped to 0.3.0
- `src/generated/mod.rs`: Module wrapper for generated code
- `.gitignore`: Exclude generated ground_truth.rs
- `scripts/verification/test_formulas.py`: Added character slot display
- `scripts/verification/save_parser.py`: Added character name extraction

---

## v0.2.9 - Event Flag Verification Framework

### Features
- **Verification Framework** (`scripts/verification/`): Complete Python tool suite to systematically test and verify event flag formulas against actual save files
  - `save_parser.py`: Structural save file parsing with dynamic offset detection
  - `flag_formulas.py`: All known formulas (block, tile, dungeon) with documented limitations
  - `diff_analyzer.py`: Before/after comparison for empirical offset discovery
  - `data_loader.py`: Loads extracted flags and manual completions
  - `verification_data.py`: Data structures for tracking verification status

- **Ground Truth Documentation** (`docs/SAVE_FILE_GROUND_TRUTH.md`): Single source of truth consolidating all save file parsing research
  - Verified constants and formulas
  - Known limitations documented (consumable treasures untrackable)
  - Formula accuracy statistics

- **Verification Runner** (`scripts/run_verification.py`): Main script to run verification pipeline
  - Tests all flag formulas against save data
  - Generates `ground_truth_offsets.json` with verified offsets
  - Reports formula accuracy by category

### Verification Results
- **81 grace flags verified** (block formula working)
- **Block formula**: 26.6% accuracy with evidence
- **Tile formula**: Needs dungeon base offset discovery
- **Dungeon formula**: 101/104 base offsets unknown

### Key Findings
- Block-based formulas (65xxx-76xxx) work reliably for graces/cookbooks
- LocalId >= 7000 flags are **structurally untrackable** (875 bytes/slot = 7000 flags max)
- Consumable treasures (Golden Runes, Smithing Stones) cannot be tracked via event flags

### Usage
```bash
python scripts/run_verification.py --verbose
```

---

## v0.2.8 - Treasure Metadata Fields

### Features
- **Treasure Type Classification**: Added `treasure_type` field to event flags
  - Detects: chest, corpse, cart, ground_pickup based on MSB InChest field and asset patterns
  - Cart treasures (AEG100_101) correctly identified with known position error

- **Item Rarity Lookup**: Added `item_rarity` field from EquipParam files
  - 0 = consumable (white glow)
  - 1 = standard (white glow)
  - 2 = rare/unique (purple glow)
  - 3 = legendary (orange glow)

- **Position Confidence**: Added `position_confidence` field
  - `high`: chest/corpse positions (~40 unit accuracy)
  - `low`: cart positions (~70-100 meter error due to model origin vs interact point)
  - `none`: no position data available

- **Underground Detection**: Added `is_underground` field
  - Uses filename keywords (地下, 洞窟, 地底, 地下室, 坑道)
  - Falls back to area_type (underworld/subterranean = underground)
  - Returns null when uncertain to avoid false positives

### Coverage
- Treasure types: corpse (1,937), ground_pickup (278), chest (201), cart (13)
- Item rarities: common (1,391), standard (1,339), rare (1,225), legendary (154)
- Position confidence: high (2,416), low (13), none (4,605)
- Underground detection: confident (2,162), uncertain (4,872)

---

## v0.2.7 - POI Region Derivation & Generic NPC Filtering

### Features
- **POI Region Extraction**: Added `get_region_from_poi_name()` function
  - Parses POI paramdexName to extract accurate region names
  - Handles Legacy Dungeon, Guidance of Grace, Minor Erdtree, Divine Tower patterns
  - Fixes POIs like "Crumbling Farum Azula" showing region "Various"

- **Generic NPC Filtering**: Added `filter_generic` parameter to NPC extraction
  - Excludes NPCs with generic names like "NPC (c1000)", "NPC (c0000)"
  - Reduces noise in exported data (541 generic NPCs filtered)
  - Keeps 305 named NPCs for cleaner output

### Improvements
- **Multi-method region assignment** for WorldMapPointParam:
  1. Extract from POI name (paramdexName)
  2. Derive from 10-digit flag ID
  3. Use grid coordinates for overworld areas
  4. Fallback to "Various"

### Coverage
- Total unique flags: 7,575 → 7,034 (filtered generic NPCs)
- POI region accuracy improved for legacy dungeons

---

## v0.2.6 - NPC Name Resolution Lookup Table

### Features
- **NPC Name Lookup Table**: Added coordinate-matched lookup for quest NPCs
  - 40 key NPCs now resolved instead of showing generic "NPC (c1000)"
  - Auto-generated by matching MSB entity positions against elden-map POI database
  - High-confidence matches only (distance < 40 units)

### NPCs Now Resolved
- Quest NPCs: Roderika, Ranni, Millicent, Boc, Patches, Hyetta, Melina
- Merchants: Kalé, Hermit Merchant, Nomadic Merchants, Isolated Merchants
- Key Characters: Iron Fist Alexander, Knight Bernahl, Edgar, Jerren
- Special: Miriel Pastor of Vows, Primeval Sorcerer Azur, Great-Jar

### Coverage Improvement
- Generic NPC (c1000): 558 → 518 (-40 resolved)
- Named NPCs: ~228 → 262 (+34)
- Lookup table can be expanded as new mappings are discovered

### Data Sources
- Coordinate matching against merged-pois.json from elden-map project
- Entity IDs from MSB Part/Enemy files

---

## v0.2.5 - Map Feature Extraction (Boss Arenas, Stakes, Spirit Springs)

### Features
- **Boss Arena Extraction**: Parse GameAreaParam for boss arena locations
  - 150+ boss arenas with defeat flags and coordinates
  - Boss discovery flags for tracking boss encounters
  - Soul reward data (single player and multiplayer)
  - Region names extracted from boss name prefixes (e.g., "[Stormveil Castle]")

- **Dungeon Info Extraction**: Parse MapDefaultInfoParam for dungeon data
  - Fast travel unlock flags (EnableFastTravelEventFlagId)
  - Links dungeon completion to boss defeats
  - 80+ dungeon entries with named locations

- **Stake of Marika Extraction**: Parse MSB SpawnPoint regions
  - 85+ Stakes of Marika with positions
  - Entity IDs for respawn point tracking
  - Distributed across dungeons and legacy areas

- **Spirit Spring Extraction**: Parse MSB MountJump regions
  - 90+ Spirit Springs with positions
  - Jump height data for each spring
  - Overworld locations with world coordinates

- **Region Name Lookup**: Load region names from MapGdRegionInfoParam
  - 135+ named regions and dungeons
  - Used for proper region classification

### New Event Flag Categories
- Boss Arena: 150+ flags with coordinates
- Boss Discovery: Flags for boss encounters
- Dungeon Cleared: Fast travel unlock flags
- Stake of Marika: 85+ respawn points
- Spirit Spring: 90+ jump pads

### Coverage Improvement
- Total unique flags: 8,052 → **7,575** (deduplicated, removed overlapping flags)
- Spatial data coverage: 60% with local coords, 77% with map tiles
- MSB-sourced coordinates: 3,444 entries

### Data Sources Added
- GameAreaParam.param.xml (boss arenas with coordinates)
- MapDefaultInfoParam.param.xml (dungeon fast travel flags)
- MapGdRegionInfoParam.param.xml (region name lookup)
- MSB Region/SpawnPoint/*.xml (Stakes of Marika)
- MSB Region/MountJump/*.xml (Spirit Springs)

---

## v0.2.4 - Enemy Defeat Flag Extraction & NPC Locations

### Features
- **MSB Enemy Extraction**: Parse MSB Part/Enemy/*.xml for boss/enemy positions
  - Cross-validates EntityIDs against event scripts for accuracy
  - Includes enemies from multiple tracking sources:
    - `SetNetworkconnectedEventFlagID` for general tracking
    - `HandleBossDefeatAndDisplayBanner` for boss defeats
    - `InitializeCommonEvent(90005860)` for field boss defeats
    - `InitializeCommonEvent(90005870)` for boss name tracking
  - 174 verified enemy defeat flags with coordinates

- **Enemy Name Resolution** (priority order):
  1. NpcName.fmg via constructed nameId (9 + model + variation) - gives full in-game names like "Margit, the Fell Omen", "Tree Sentinel"
  2. BgmBossChrIdConv for major boss display names (Godrick, Rennala, Malenia, etc.)
  3. ChrModelParam.paramdexName for general enemy names (266 model mappings)
  4. NPCParamID → nameId → NpcName.fmg fallback

- **Enemy Type Classification** (based on entity ID patterns and model):
  - `Great Boss`: Main demigods with c2xxx/c4xxx models (Godrick, Rennala, Malenia, Mohg, etc.)
  - `Boss`: Entity IDs ending in 0800/0801 (Tree Sentinel, Night's Cavalry, dungeon bosses)
  - `Field Boss`: Entity IDs ending in 0850/0851 (Margit pre-Stormveil, various)
  - `Invasion`: Player model (c0000) NPC invaders
  - `Enemy`: Other trackable one-time enemies

- **NPC Location Extraction**: Extract characters with dialog (TalkID > 0)
  - 846 unique NPCs with positions from MSB files
  - NPC type classification: Merchant, Smith, Quest NPC, Trainer, etc.
  - Includes key NPCs like War Counselor Iji, Nomadic Merchants, questgivers
  - Uses EntityID as flag ID for tracking (most NPCs lack explicit event flags)

### New Event Flag Categories
- Great Boss Defeat: 88 flags
- Boss Defeat: 58 flags
- Field Boss Defeat: 23 flags
- Invasion Defeat: 2 flags
- Enemy Defeat: 2 flags
- Elite Enemy Defeat: 1 flag
- NPC (with dialog): 846 entries

### Coverage Improvement
- Total unique flags: 6,213 → **8,052** (+1,839 flags)
- Enemy defeat flags: 174 with verified positions
- NPCs with dialog: 846 with positions from MSB files

### Data Sources Added
- NpcParam.param.xml (7,038 NPC definitions)
- ChrModelParam.param.xml (266 model → name mappings)
- WwiseValueToStrParam_BgmBossChrIdConv.param.xml (15 boss names)
- NpcName.fmg.xml (479 NPC names with constructed nameId lookup)
- MSB Part/Enemy/*.xml (positions for 174 verified enemies + 846 NPCs)
- Event scripts (*.emevd.js) for defeat flag validation

---

## v0.2.3 - Multi-Item Chest Position Linking

### Features
- **Multi-item chest linking**: Secondary items in a chest now inherit position from the base item
  - Example: Ash of War: Storm Stomp (row 1042371011) now gets position from Whetstone Knife (row 1042371010) since they're in the same chest
  - Checks consecutive row IDs (row_id-1 through row_id-10) for MSB treasure entries
  - New field `msb_base_row_id` tracks when position came from a different row

### Coverage Improvement
- MSB positions used: 2,368 → **2,504** (+136 items)
- Flags with local coords: 51% → **52%**

---

## v0.2.2 - MSB Area/Grid Extraction Fix

### Bug Fixes
- **Parse area/grid from MSB directory names**: Flags like Whetstone Knife (60130) that don't encode location in their ID now get area/grid info from the MSB directory name (e.g., `m60_42_37_00-msb-dcx` → area=60, grid=(42,37))
- This enables correct world coordinate calculation for ~76 additional flags

### Coverage Improvement
- Flags with world coords: 24% → **25%** (+76 flags)
- Flags with map tile: 70% → **72%** (+121 flags)

---

## v0.2.1 - MSB Position Data & Area Type Classification

### Features
- **MSB Treasure Position Extraction**: Parse Map Studio Binary files for accurate item positions
  - Loads treasure positions from 935 MSB directories
  - Links ItemLotID → TreasurePartName → Asset Position
  - 2,379 treasure positions extracted, 2,368 matched to event flags
  - Position source tracked in `raw_data.position_source` ("MSB" or "WorldMapPointParam")

- **Area Type Classification**: Distinguish location types for proper coordinate handling
  - `overworld_surface`: Open world (area 60 base, 61 DLC) - world coords valid
  - `underworld`: Underground open areas (area 12) - Siofra, Ainsel, Nokron
  - `subterranean`: Deep underground (area 35) - Shunning-Grounds, Mohgwyn
  - `legacy_dungeon`: Major story dungeons (areas 10-16, 19-28)
  - `minor_dungeon`: Caves, catacombs, tunnels (areas 30-32, 39-43)
  - `divine_tower`: Divine Tower locations (area 34)
  - `tutorial`: Tutorial area (area 18)

- **Base Game vs DLC Distinction**: New `is_dlc` field for filtering

### Bug Fixes
- **Fixed world coordinate calculation for dungeons**: Previously applied `grid * 256 + pos` formula to all locations, which is only valid for overworld tiles. Dungeon coordinates are now correctly left as local positions with `world_x`/`world_z` set to null.

### New Fields
- `is_overworld`: Boolean - true only for area 60/61
- `world_x`, `world_z`: Computed world coordinates (null for non-overworld)
- `area_type`: Location classification string
- `is_dlc`: Boolean - true for Shadow of the Erdtree content

### Spatial Data Coverage
- Flags with local coords: 51% (3,174/6,213)
- Flags with world coords: 24% (1,510/6,213) - overworld only
- Flags with map tile: 70% (4,362/6,213)
- Coordinates from MSB files: 2,368

---

## v0.2.0 - Event Flags Database

### Features
- **Event Flags DB View**: New comprehensive database view with ~5,000+ event flags
  - Category filtering (22 categories including Great Runes, Graces, Cookbooks, etc.)
  - Region dropdown filtering
  - Text search by name or flag ID
  - JSON export (full database or filtered results)

- **Enhanced Extraction Script** (`scripts/extract_event_flags.py`):
  - DLC01 name file support for proper DLC item names
  - Fixed Crystal Tear vs Whetblade categorization
  - Added `common.emevd.js` parsing for Great Runes, Remembrances, Talisman Pouches
  - Markdown and JSON output formats with full data preservation
  - **Spatial data extraction**: map tiles, XYZ coordinates, region IDs
  - 6,213 unique event flags extracted across 23 categories

### Spatial Data Coverage
- Graces: 100% with full coordinates (422 entries)
- Landmarks: 100% with full coordinates (379 entries)
- World pickups: 81% with map tiles derived from flag ID
- New fields: `area_no`, `grid_x`, `grid_z`, `pos_x`, `pos_y`, `pos_z`, `map_tile`, `region_id`

### Data Sources
- `ItemLotParam_map.param.xml` - World pickups
- `BonfireWarpParam.param.xml` - Grace sites (with coordinates)
- `ShopLineupParam.param.xml` - Shop items
- `WorldMapPointParam.param.xml` - POI locations (with coordinates)
- `WorldMapPieceParam.param.xml` - Region definitions
- `common.emevd.js` - Event scripts (Great Runes, Remembrances)

### Bug Fixes
- Fixed Map Fragment category showing only 1 entry (was being overwritten by WorldPickup category)
- Fixed Crystal Tears (65000-65399) being miscategorized as Whetblades (65610-65720)

---

## v0.1.0 - Initial Release

- Core save file parsing and editing
- Character stats editing
- Inventory management
- Equipment editing
- Grace/Boss tracking
- Regions database
