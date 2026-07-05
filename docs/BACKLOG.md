# Project Backlog

**Last updated**: 2026-07-05

---

## Priority 0: Knowledge Base Reset — Migration Plan (decided 2026-07-05)

Decisions recorded in `CONTEXT.md` and `docs/adr/0001`–`0006`. Summary: Evidence = raw
bytes only; the claims store is pipeline-generated with a status ladder and tombstones;
one reference implementation in `crates/wasm-event-flags` defined by conformance
fixtures; reset the knowledge, not the code (no fresh upstream clone).

Steps, in order:

1. **Anchor conformance** — fix `compute_structural_ef_offset` (~146k overshoot, see
   next section), port the working `src/save` struct-parse logic into the wasm crate,
   commit the conformance fixture set (5 test slots + catacombs kill bytes + sd_000259
   kill transitions), add a validation gate, delete the redundant detectors (python
   SaveParser parsing, elden-map `slot-layout.ts`/`ground-truth-formulas.ts`).
2. **Evidence catalog** — committed index (path, sha256, capture context, slot
   descriptions) of game extracted files, snapshots, timeline raw diffs; pipeline
   verifies it. Timeline *metadata* is demoted to legacy claims.
3. **Pipeline** — `knowledge` subcommand family in this binary (reuses the reference
   implementation): catalog check → anchor detection → verification methods
   (multi-slot differential, kill transition, reward corroboration per ADR-0007) →
   deterministic claims-store emission. Add `ef-dump` for Python/exploratory consumers.
   Interpretation diffs PARSED DOMAIN OBJECTS, not raw bytes: inventory deltas by item
   identity (GaItem handles churn), flags per family (per-family float, ADR-0003
   amendment). Timeline re-annotation (bossesDefeated etc.) is a pipeline output.
4. **Freeze `ground_truth_offsets.json` read-only**; per-family cutover to the claims
   store (graces → boss defeats → pickups), legacy entries promoted or tombstoned.
5. **Distill and delete** the Python lab scripts (~50k lines) and shrink
   `src/discovery` to what the pipeline uses; move `src/db/event_flags.rs` (in-memory
   convention) out of the app into KB inputs as the CE-era Rosetta table.
6. **Docs audit** — epistemic header on all 14 docs (evidence / claim summary /
   methodology / obsolete), correct or retire wrong content (EVENT-FLAG-GEOGRAPHY area
   labels, stale CLAUDE.md paths); CLAUDE.md shrinks to workflow rules + pointers to
   `CONTEXT.md` and the catalog.

---

## Priority 0b: EF Anchor Detection Inconsistency (found 2026-07-05, PARTIALLY RESOLVED same day)

**Resolved** (migration step 1, anchor conformance):
- `detect_event_flags_offset_impl` reworked: primary is now a gaEnd-windowed
  grace-validation scan ([gaEnd+30k, gaEnd+45k]); the disproven structural walk is no
  longer used for detection; honest confidence gating (all-zero slots are no longer
  `confident: true`). Legacy content-fallback `SEARCH_START` corrected 0x30000→0x12000
  (it previously started PAST the real flag region). `save_slot.rs` fallback constant
  0x36500 (the ~222k lookalike) replaced with a gaEnd-derived fallback; the backwards
  "real EF is at ~222K" comment corrected.
- Proof the ~222k position is a lookalike: b24/b25 kill-transition pair — flag 30020800
  flips in the low region; the struct-walk position stays zero in both files.
- Conformance fixtures committed (ADR-0003): `crates/wasm-event-flags/tests/fixtures/`
  (8 slot prefixes with provenance) + `tests/anchor_conformance.rs` (golden detections,
  in-window property, tier-1 anchor bits, gaEnd churn tracking across the kill pair).

**NEW FINDING — per-family float (shapes the pipeline design):** flag families
(graces, catacombs, …) sit at independently floating bases per save (Δ0 on the Bee
save, Δ~77-141 on b24, ~490 on backup measurements), and regions shift by different
amounts within one save pair (b24→b25: GaItems +16, flag region +4). A single "EF
anchor" therefore cannot position all families across saves: ADR-0003's convention
must become per-family bases, and a GT offset measured against one save's anchor is
only valid for that family in that save's layout. Byte-exact family pinning =
shift-aware flip-pair analysis in the re-verification pipeline (migration step 3).

**Remaining** — original problem statement: multiple EF-offset implementations disagree on the same save slots, which silently breaks
all flag reads downstream (this — not wrong bases — was the cause of `batch-validate 0
--context boss_defeat` reporting 0/110 set on a mid-game character):

- **`compute_structural_ef_offset` (crates/wasm-event-flags) overshoots by ~146,000 bytes**
  (returned 227,671 on backup slot 0 where hard-fact anchoring via GT catacombs kill bytes
  puts the EF flag data at ~81,567). Because it returns `Some` with `confident: true`, the
  content-based fallback never runs and no validation gate catches it. The elden-map capture
  agent inherited this (~Mar 2026 onward): its recorded `eventFlagsOffset` jumped from ~76k
  (Feb, correct) to ~223k, and its EF-relative reads flicker whenever GaItems resizes.
- **`scripts/verification/save_parser.py` content search** lands on a lookalike region
  (~106,808 on the same slot) — grace-pattern false positive; its per-slot results are
  inconsistent with each other by small deltas (V2 vs V3 align, V1 off by 4).
- **`src/save` struct parse (probe CLI path)** is correct on the 2026-01-11 backup but reads
  all-zero on the "level 93 snapshot" — likely fails for larger GaItems.
- **`ground_truth_offsets.json` mixes anchor conventions**: catacombs/tunnels/graces-block
  families are probe-convention (consistent, trustworthy); the 71xxx/72xxx dungeon-grace
  family reads garbage at the probe anchor (verified-era drift). Offsets from different
  verification eras must not be combined in one matched filter.

Follow-ups:
1. ~~Fix detection; add a hard validation gate~~ DONE 2026-07-05 (windowed scan + gate;
   structural walk demoted to diagnostics).
2. Python `SaveParser` still runs its own (lookalike-prone) content search — re-point it
   to an `ef-dump` CLI output per ADR-0005; elden-map `slot-layout.ts` /
   `ground-truth-formulas.ts` deletion happens in the coordinated change. The same
   coordinated change upgrades the capture flow per ADR-0007: agent stops writing
   interpretations; adds full-slot keyframes (every N entries + on GaItems resize),
   per-entry state checksums, and agent+wasm version stamps. `scripts/capture_agent.py`
   catalog context fields get the same demotion (computed with the python detector).
3. Re-anchor `ground_truth_offsets.json` PER FAMILY (see per-family float finding);
   record family + source save in each claim's provenance.
4. Then locate true (18,0)/(19,0) bases (see below) — evidence points to the m18 general
   section living near the m18 pickup base 3847, not at the removed 43487.

---

This is the single location for all planned work, remaining gaps, and deferred items. Organized by priority.

---

## Priority 1: Data Coverage Gaps

### Gesture Database
- **Source**: GestureParam (~60 rows)
- **Status**: PARTIALLY DONE — `load_gesture_names()` added in extraction script (v0.17.12), 51 gestures mapped for flag name resolution. WASM/Rust enum and save editing not yet implemented.
- **Impact**: Cannot display/edit unlocked gestures in save editor
- **Effort**: Low (simple enum + flag mapping, gesture names already available)

### Full NPC Database
- **Source**: NpcParam (~500 entries)
- **Status**: Only 30 of ~500 NPCs tracked (`npcs.rs`)
- **Impact**: Cannot track most NPC encounters/questlines
- **Effort**: Medium (need to map NPC IDs to names and event flags)

---

## Priority 2: Event Flag Verification

### Boss Flag Verification Improvement
- **Current**: Great Boss 9.6%, Field Boss 4.3%, Generic Boss 13.8% verified
- **Needed**: Create test saves with specific bosses defeated for differential analysis
- **Blocked by**: Need gameplay progression in test characters

### Unreliable Block Bases
- **Blocks**: ~~71000~~, ~~71100~~, 71600, 73000
- **Issue**: Base offsets vary by save progression (not stable across saves)
- **Solution**: Dynamic calibration per save file, or discover stable alternative bases
- **Progress** (v0.15.0): Multi-tile calibration with 4 anchors from 2+ tiles mitigates drift; unified flag routing in WASM uses calibrated tile_base
- **Progress** (v0.17.9): Block 71000 resolved via sub-block/main-block split — Stormveil graces (71000-71099) route to sub-block base 9315, dungeon graces (71100-71799) route to main-block base 2625. Block 71100 now resolved as part of the 71000 main-block range

### Unverified Dungeon Areas
- **Update 2026-07-05 (multi-slot differential + Bee timeline audit)**:
  - **(14,0)=29987 VERIFIED** — Red Wolf kill (14000850) at 29987+106 bit5 in timeline
    entry sd_000259, same byte-validated window as GT-proven 30040800/31020800.
  - **(18,0)=43487 and (19,0)=46862 DISPROVEN and removed** from
    `get_dungeon_general_bases` — all five test slots and Bee's day-1 state have zero
    bytes in those spans despite mandatory tutorial (m18) flags. m18 = Stranded
    Graveyard (Soldier of Godrick 18000850), m19 = Elden Throne (Radagon 19000800);
    old comments had these areas mislabeled. True m18 section likely near the
    calibrated m18 pickup base 3847. Consumers now report "unknown" for these areas.
  - Remaining m10/m11/m12/m13/m15/m20/m21/m22 entries stem from the same "+3375 per
    area" stride assumption and read all-zero in every available save — treat as
    UNVERIFIED ((35,0) duplicates (20,0), (39,20) duplicates (21,0)).
- **Areas**: 20, 21 (unverified), plus 10, 11, 12, 13, 15, 16, 34, 35, 39 (calculated but not empirically verified)
- **Method**: Multi-slot differential with appropriate test characters — blocked on the
  Priority 0 anchor fix for byte-precise localization

### Disproven Block Bases
- **Blocks**: 75000, 77000 (0xFF padding, not real data)
- **Action**: Discover actual bases or confirm these ranges are unused

---

## Priority 3: Cross-Project Sync

### Elden Map Missing Block Bases
- **Issue**: Elden Map viewer (`eventFlagService.ts`) is missing block bases that Save Editor has
- **Missing blocks**: 62000 (map fragments), 65000 (Crystal Tears), 72000 (DLC graces), 74000 (DLC dungeon graces), 78000 (grace guidance)
- **Action**: Sync BLOCK_BASES from ground_truth_offsets.json to Elden Map
- **Progress** (v0.15.0): WASM unified flag routing now includes all block bases — elden-map can use WASM `get_flag_offset()` instead of maintaining separate lookup tables
- **Progress** (v0.16.1): Block bases corrected — old bases were false positives calibrated against GaItemData section. 61000 removed (disproven). New blocks added: 66000, 69000, 91000, 92000

---

## Priority 4: Code Quality

### Module Consolidation (Optional)
Several data categories have parallel modules (see [DATABASE_COVERAGE_ANALYSIS.md](DATABASE_COVERAGE_ANALYSIS.md#code-redundancy-notes)):
- `world_pickups.rs` / `pickup_data.rs` (overlapping pickup data)
- `graces.rs` / `graces_data.rs` (enum + enriched split)
- `bosses.rs` / `bosses_data.rs` (enum + enriched split)
- `shop_items.rs` / `merchants_data.rs` (different grouping)

These work correctly as-is but could be consolidated to reduce maintenance burden.

---

## Priority 5: Infrastructure (Deferred)

### CI Integration for Verification
- **Concept**: Automated regression testing of flag formulas against test saves
- **Status**: NOT STARTED
- **Source**: archive/VERIFICATION_STRATEGY.md

### EvidenceDiscoveryService in Rust
- **Concept**: Rust-native version of the Python evidence discovery workflow
- **Status**: NOT STARTED
- **Source**: archive/EVIDENCE-BASED-DISCOVERY.md
- **Rationale**: Deferred - Python scripts work well enough for now

---

## Completed (for reference)

Items from previous "Next Steps" that have been done:

| Item | Completed In | Version |
|------|-------------|---------|
| Spell database | spells.rs (315 entries) | v0.8.0 |
| NPC tracking (partial) | npcs.rs (30 entries) | v0.10.0 |
| Shop stock tracking | shop_items.rs + merchants_data.rs | v0.9.0 |
| World pickup browser | world_pickups.rs + UI | v0.7.0 |
| Dungeon pickup section bases | 89 bases verified | v0.12.0 |
| Landmark database | landmarks.rs (308 entries) | v0.11.0 |
| Entity relationships | entity_relationships_data.rs (613) | v0.13.0 |
| Quest chains | quest_chains.rs (24 entries) | v0.12.0 |
| Row ID formula discovery | Consumable tracking enabled | v0.12.0 |
| Dungeon pickup per-section bases | 89 sections across 22 areas | v0.12.0 |
| Player coord extraction (DRY) | Consolidated into shared WASM module | v0.14.0 |
| Unified flag resolution | Single WASM dispatcher for all flag ranges | v0.15.0 |
| Multi-tile calibration | 4-anchor constraint satisfaction from 2+ tiles | v0.15.0 |
| Position validation hardening | Denormalized float + angle range rejection | v0.15.0 |
| Equipment extraction (WASM) | Equipped items, quick items, pouch parsing | v0.15.0 |
| Structural EF detection | Sequential section parsing replaces content-based search | v0.16.0 |
