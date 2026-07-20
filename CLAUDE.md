## Commit Protocol

**IMPORTANT**: Never commit automatically. Always use the `/snapshot` command to ensure the commit protocol is followed properly. This ensures version bumps, changelog updates, and documentation are handled consistently.

## Remembering Command execution fault

**IMPORTANT**: When an (allowed) executed command throws an error and a corrected format of the same command is executed afterwards successfully, take note of the correct command form to prevent burning tokens repeatedly.

---

## Evidence Catalog (single source of truth for evidence inventory)

`knowledge/evidence-catalog.json` — integrity index (sha256) over all out-of-repo
evidence, with per-corpus trust context. Verify with
`er-save-editor knowledge catalog-verify` before relying on evidence files.
Glossary: `CONTEXT.md`. Decisions: `docs/adr/`.

**Claims store**: `knowledge/claims/event-flags.json` — pipeline-generated
(`er-save-editor knowledge run`), NEVER hand-edited (ADR-0004). For the families it
covers (world-state-b, tile-open-world, legacy-dungeon) it supersedes
`ground_truth_offsets.json` and the block-base tables in the docs. Check its
tombstones before re-proposing any offset convention.

**Decompiled game resource files: PARTIALLY RESTORED 2026-07-05** at
'/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files':
`regulation-bin/*.param.xml` regenerated (WitchyBND, regulation 1.16.1 — matches the
save era; catalog corpus `game-extracts`). Raw game files (EMEVD, alloclists, MSBs,
regulation.bin) live in corpus `game-raw-1162`. The old `event/*.emevd.js` decompiles
were NOT regenerated — the pipeline parses raw `.emevd` natively.

## Game save files with five character slots:

- Slot 0: Confessor, mid-game progression (NOTE: in the 2026-01-11 backup this slot
  predates the Margit/Godrick/Radahn kills — see catalog entry)
- Slot 1: Wretch, early game, few steps of progression, item collection, one boss defeat
- Slot 2: V1, very little progression, made for item pickup debugging
- Slot 3: V2, similar little amout progression as V1, different path taken, same item pickup for debugging
- Slot 4: V3, similar little amout progression as V1, different path taken, no pickup for true negative diff
- '/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files'

## Third party resource usage
Treat third party resources with caution because we don't have control over their accuracy, completeness, or reliability. Most of the time they are specific to a certain game version, thus outdated and many times partially implemented. Always verify information from third-party sources against primary sources and discard them if their correctness can not be proven.

---

## False Negative Investigation Protocol

**MANDATORY**: When investigating a false negative (auto-detection fails where manual succeeds), follow this evidence-based protocol BEFORE proposing any fixes:

### Phase 1: Evidence Collection (No Speculation)

1. **Read the actual save file** - Use hex dump or binary read to locate the flag bytes empirically
2. **Verify the byte offset** - Calculate expected offset using ground truth, then confirm it matches actual location
3. **Check both flag systems** - For world pickups, verify both tile flag AND block flag states
4. **Document observed vs expected** - Write down what the bytes actually show

### Phase 2: Multi-Slot Differential (Gold Standard)

Reference: `docs/discovery-verification-cycle.md`

- Compare flag state across character slots with known different progression
- Use the test slots (V1, V2, V3) specifically designed for this purpose
- A flag verified across multiple slots with expected differences = HIGH confidence

### Phase 3: Corroboration Check

Reference: `docs/CORROBORATION-SYSTEM.md`

- For world pickups: tile flag SET + block flag SET = corroborated
- If they disagree: the disagreement IS the clue - don't dismiss it
- Inseparable evidence (boss + grace, etc.) must be consistent

### Phase 4: Calibration Verification

- Check if calibration is returning correct base offset for THIS save file
- The ground truth tile base (337375) is constant across saves within the same game version
- The EF offset varies per character (due to variable GaItems), but tile_base within EF is fixed

### Only After Evidence Is Gathered

- Build a concrete test case with before/after hex dumps
- Document the evidence with confidence level (VERIFIED/LIKELY/UNVERIFIED)
- THEN propose a fix based on empirical findings

**NEVER skip directly to proposing fixes**. The methodology exists because event flag detection has many moving parts (calibration, per-character offsets, formula correctness) and speculation wastes effort.

---

## Technical Documentation

| Topic | Document |
|-------|----------|
| **System architecture** | `docs/ARCHITECTURE.md` |
| Event flag geography & formulas | `docs/EVENT-FLAG-GEOGRAPHY.md` |
| **WASM shared detection** | `docs/WASM-EVENT-FLAGS.md` |
| Discovery methodology | `docs/discovery-verification-cycle.md` |
| Corroboration system | `docs/CORROBORATION-SYSTEM.md` |
| Case verification guide | `docs/CASE-VERIFICATION-GUIDE.md` |
| Database coverage | `docs/DATABASE_COVERAGE_ANALYSIS.md` |
| Save file ground truth | `docs/SAVE_FILE_GROUND_TRUTH.md` |
| Data sources & characters | `docs/DATA-SOURCES.md` |
| Project backlog | `docs/BACKLOG.md` |
| Ground truth data | `ground_truth_offsets.json` |
| Save slot feature registry | `save_slot_registry.json` |

**Single Source of Truth**:
- Flag positions: **resolve them, never hardcode them.** Families sit after an
  append-only u32 list that grows with progression, so any fixed offset is valid only
  for the save it was measured on. Use
  `wasm_event_flags::resolve_family_base(slot, FAMILY_*)` —
  `family_base = ga_items_end + flag_list_end + FAMILY_CONSTANT`. See
  `docs/SAVE_FILE_GROUND_TRUTH.md` ("Flag Family Origin") and `docs/BACKLOG.md` 4b.
- `ground_truth_offsets.json`: **FROZEN read-only** (ADR-0006). Still wired in per
  family until each cuts over, but never add or edit entries — new knowledge goes to
  the claims store via `knowledge run`. `flag_formulas.py` remains deprecated.
- EventFlags detection: `crates/wasm-event-flags/` (shared with elden-map via WASM)
- Tutorial graces (71800/76100) are NOT universal anchors — they are clear on minimal
  characters. Never use them as a validity test for a detected offset.
