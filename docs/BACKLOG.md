# Project Backlog

**Last updated**: 2026-07-22

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: LIVING RECORD — the working plan and open questions.** Holds the knowledge-base migration plan (steps 1-6) and the reasoning behind open/closed questions; entries are dated and later ones supersede earlier ones (many carry inline CORRECTED / SETTLED / tombstone notes). Canonical facts live in `CONTEXT.md` + the claims store; this is where the *reasoning and next steps* live. Newest work is dated 2026-07-22, matching the stamp above.

---

## Priority 0c: New timeline evidence has arrived uncataloged (found 2026-07-22) — DONE 2026-07-22

> **RESOLVED the same day. The hypothesis below was CONFIRMED, and it understated the
> result: the new captures do not carry a *corrected* structural anchor, they carry
> **no derived claim at all**.** Intake done, `catalog-verify` back to exit 0. The
> statement of the problem is kept because the intake method is the reusable part.
>
> **What the metadata actually says** (established by reading `slot_changes.jsonl`
> field-by-field, not by assuming a rebuild). Era 1 entries carry
> `structuralOffsets: {gaItemsEnd, eventFlagsOffset, efConfident}` — `sd_003970`, the last
> cataloged one, reads `eventFlagsOffset: 228676`, the poisoned ~223k anchor the trust note
> describes, so the poisoning ran to the very end of era 1. All 39 era-2 entries read
> `structuralOffsets: null`, `bossesDefeated: []`, `gracesDiscovered: []`, `level: null`,
> `runes: null`. Only raw observation survives: timestamp, slotIndex, characterName,
> saveType, bytesChanged, diffFile, playerPosition, inventoryDelta. That is the ADR-0007 /
> ADR-0008 posture reaching the capture agent — the fix was to **stop emitting the claim,
> not to correct it**, which is the stronger outcome and the one that needs no trust.
>
> *Scope of the new evidence:* exactly 39 files, `sd_003971`..`sd_004009`, 2026-07-21
> 15:00–19:36Z, same subject throughout (slot 5 "Bee", autosave). Verified against the
> manifest: **39 new, 0 cataloged files changed, 0 missing** — so this is pure growth, not
> tampering. (`catalog-verify` printed only 5 drift lines because it caps the listing at
> `take(5)`, `src/knowledge/catalog.rs:185`; the full account came from diffing the
> manifest.)
>
> **NEW FINDING, and it corrects this corpus's own description: the timeline is not one
> chain.** Testing `prev.new == next.old` on overlapping offsets between consecutive
> diffs: inside a contiguous run agreement is **100.00%** (checked in both eras, on
> overlaps of 250k–415k offsets); at a segment boundary it collapses to **1–25%**. At
> least **21 boundaries** exist — 20 of the 37 inter-capture gaps over 30 minutes, plus
> `sd_003274 → sd_003275` whose gap is only **288 seconds**, so a long gap predicts a
> boundary but does not determine one, and the exhaustive count needs a full scan (only
> short-gap pairs were sampled). The era-1/era-2 join is one such boundary (9.90% over
> 248,617 overlapping offsets). *Consequence:* replay state must be treated as **reset at
> each boundary**, and the corpus's "~0.7% old-value mismatch" is not spread evenly — it is
> concentrated there. This directly constrains the flip-clustering design proposed in
> step 3: cluster **within** a segment, never across a boundary.
>
> *Artifacts regenerated, not hand-edited (ADR-0004):* `evidence-catalog.json` (per-era
> `trust`, new `eras` and `segments` context) + manifest via `catalog-update`;
> `catalog-verify` now reports all 8 corpora intact. `knowledge run` re-run twice —
> **every claim body unchanged**, the only diff is the input catalog's sha256, and the
> second run reports "claims store unchanged", so determinism survives the corpus growth.
> `knowledge timeline bee` re-run over 3,869 entries: 1,208,825,803 records, mismatch
> 0.68% → **0.74%** (consistent with adding a hard boundary), confident grace detection
> 2,735/3,830 → **2,774/3,869** — i.e. all 39 new entries detect confidently, offset range
> unchanged at 72,609..82,586.
>
> *Still true, and still the reason none of this is promoted to claims:* the metadata is
> LEGACY CLAIMS, cataloged for integrity, not endorsement. Era 2 asserting nothing makes it
> honest, not authoritative. `inventoryDelta` keeps the era-1 GaItem-churn caveat — it is
> the same field from the same code path.

`knowledge catalog-verify` exits 1 on two corpora, and **this drift is not a fault — it is
new evidence**. The Bee capture agent ran again on 2026-07-21 (newest file `sd_004009.bin`,
21:36): `timeline-slot-diffs` holds 3,869 files against 3,830 cataloged, and
`timeline-metadata` (`slot_changes.jsonl`) grew with it.

**Deliberately NOT absorbed during the ADR-0009 rename commit.** Running `catalog-update`
would re-bless ~39 captures whose provenance has not been examined, which is exactly what
ADR-0007 says not to do with capture-agent output — and the corpus's hand-written
`description` and `context` both assert "3,830 sparse slot diffs, 2026-02-14 .. 2026-05-25",
so they need a human judgment, not a machine refresh.

*Why it may be worth more than a routine re-catalog:* these captures postdate the ADR-0008
cutover (2026-07-21), so they are plausibly the first ones taken by an elden-map build that
is no longer writing a poisoned ~223k anchor or a fabricated `calibrated_tile_base`. The
existing `timeline-metadata` trust note scopes trustworthiness to "the Feb-2026 era only".
If that holds for the new range, this is the first clean capture-agent evidence in the
corpus. **Unverified — that is a hypothesis about which build produced them, and it needs
checking against the agent/wasm version stamps before any of it is trusted.**

*The intake task:* establish which build wrote them, extend or split the corpus entry with
an honest per-era trust note, then `catalog-update`. Note the pipeline is unaffected either
way — `knowledge run` verify-on-read passes, because it reads the snapshot corpora, not this
one.

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
2. ~~Evidence catalog~~ DONE 2026-07-05: `knowledge/evidence-catalog.json` (7 corpora,
   hand context + machine sha256) + per-file manifests under `knowledge/manifests/`
   (3,997 files, ~12GB) + `er-save-reader knowledge catalog-update|catalog-verify`
   (start of the `knowledge` CLI family; verify runs in ~70s, exit 1 on drift).
   Timeline metadata cataloged as LEGACY CLAIMS with trust-era notes. FINDING: the
   decompiled game files corpus is MISSING from this machine (recorded as a `missing`
   corpus; only `scripts/extracted_event_flags.json` survives, provenance
   unverifiable) — restoring/re-extracting it is a prerequisite for re-grounding flag
   definitions. CLAUDE.md and DATA-SOURCES.md stale paths corrected.
   **UPDATE (2026-07-05 later):** raw sources RESTORED from the Steam install (exe
   ProductVersion 2.6.2 ≈ 1.16.x) as catalog corpus `game-raw-1162` (event/,
   regulation.bin, MSBs; 1,534 files). eventflagalloclists decompressed (ooz) and
   parsed → `knowledge/game/eventflag-alloclists.json` (143 allocation entries).
   **KEY FINDING:** the alloclist CONFIRMS the legacy-map layout including the removed
   m18=43,487 / m19=46,862 values (`base = 4112 + slot×1125`, and 4112+23×1125 = 29,987
   = the byte-verified m14 base). Reconciliation with the empirical zeros at those
   spans: the LAYOUT is correct but the legacy-region's in-save position floats per
   save (per-family float) — the pipeline must calibrate the legacy-family base per
   save and then apply the alloclist layout. Also resolved: "areas 20/21" belong to
   DLC maps m20 (Belurat) / m21 (Enir-Ilim) at DLC alloclist slots 150-156, not
   Stranded Graveyard/Haligtree as old comments claimed. Remaining extraction levels
   (EMEVD parse, param decode, MSB) documented in docs/DATA-SOURCES.md.
   **UPDATE (2026-07-05, latest):** regulation param XMLs REGENERATED on the Windows
   machine via WitchyBND (regulation version 11611000 = 1.16.1 ✓ save era); verified
   (194 params, paramdef-resolved fields, Paramdex row names; ItemLotParam_map 5,564
   rows etc.) and restored to the canonical 'Elden Ring decompiled game files' path.
   Catalog corpus `game-extracts` flipped missing→directory (390 files). Still not
   regenerated (by design): emevd.js decompiles (pipeline parses raw .emevd from
   `game-raw-1162`), MSB XMLs (optional).
3. **Pipeline** — CORE DONE 2026-07-05: `er-save-reader knowledge run`
   (`src/knowledge/pipeline.rs`) regenerates `knowledge/claims/event-flags.json`
   deterministically (re-run ⇒ byte-identical) from the hand-written hypothesis input
   `knowledge/inputs/attributed-transitions.json` (24 Confessor pairs: the numbered
   01-10 session of 2025-12-29, ordering confirmed by file mtimes + a byte-identical
   session-boundary file, plus the b-series of 2026-01-23..25) + the
   alloclist layout + the evidence catalog. Stages: verify-on-read (sha256 vs
   manifest) → grace-base detection (reference impl) → grace-aligned isolated-flip
   extraction (±16-byte identical neighborhoods kill shift illusions) → candidate
   resolution via within-file cross-checks (earlier transitions SET / later CLEAR /
   known-set anchors SET at the candidate family base) + multi-file differential
   disambiguation (set-monotonic candidates whose implied base is independently
   re-measured by a later resolved pair must stay SET in that pair's files) →
   tombstone refutations recomputed from bytes each run (a failing refutation
   aborts) → deterministic
   emission. **RESULTS (24 pairs):** 20 flags Verified (bosses 30020800, 30030800,
   1042370800, 1033450800; graces 73002, 71602, 76310; world flags 66700, 60260,
   67640; dungeon
   pickups 30027000, 30027030, 30037030; world pickups by row id 1044360040,
   1042320000, 1042320020, 1042370300, 1033460040, 1043500000, 1043500010) + 4
   honest hypotheses (62132
   "entering", NPC-talk 16000720/730/750 — their labeled flags provably do NOT flip
   in the EF region on those transitions); 5 family layouts Verified with measured
   per-save bases — world-state-b `(flag−50000)/8` @ ~146.6k grace_rel,
   tile-open-world `slot×875+local/8` @ ~483.47k, tile-pickup-row-id (same tile
   layout, row_id = getItemFlagId−7000, SEPARATE region) @ ~483.97k,
   legacy-dungeon `alloclist_slot×1125+local/8` @ ~1,529.98k, legacy-dungeon-pickup
   (same layout, separate region) @ ~1,529.85k; 4 tombstones (tile 337,375 was
   struct-anchor-relative; legacy-at-4,112 refuted — the 28-31k span is a u32-record
   list; universal anchor; dungeon-graces-in-copy-A). Resolution is iterative
   (expectations only from already-resolved pairs), so a wrong hypothesis cannot
   poison verified claims — this is what exposed the pickup regions as separate
   families. Also caught: b15/b16's filename `rowId-1042371300` annotation is wrong
   (getItemFlagId−7000 = 1042370300; the flip verifies the corrected id).
   NEW FINDINGS from the December session: open-world graces (76xxx) set the bit in
   BOTH world-state blocks (copy A and copy B) while dungeon graces set copy B only
   (tombstone 4 now records the contrast); family bases float per SESSION on the
   same character (Dec tile-pickup base 483,889 / world-state-b 146,514 vs b-series
   483,969 / 146,598-146,618) but are stable within a session.
   REWARD CORROBORATION DONE 2026-07-06 (ADR-0007): each capture's inventory is
   parsed by item identity (weapon/armor/AoW handles resolved via the slot's
   ga_items table; accessory/goods ids from the handle's low 28 bits; held +
   storage, common + key lists) and diffed across the pair window; deltas are
   evidence on every claim, and matching gains add an independent
   `reward_corroboration` method. Every resolvable pickup/kill label was
   corroborated by its exact item (e.g. Watchdog → Noble Sorcerer Ashes, Bols →
   Greatblade Phalanx, Crucible Knight → Aspects of the Crucible: Tail, chest →
   Arrow's Reach Talisman); grace pairs honestly show flask-refill noise instead.
   MULTI-SLOT DIFFERENTIAL DONE 2026-07-06: pairs may now override corpus/save_slot
   (cross-checks scoped to same corpus+slot — bases float per save), and a
   `multi_slot_differentials` input section verifies a flag across character slots
   with attributed different progression. The purpose-built instrument files
   ("treasure_m60_44_36_10_1044360310 picked by - V1 yes, V2 yes, V3 no/yes") verified
   rowId 1044360310 across V1/V2/V3: the V3 no→yes anchor pair pins the base
   (482,865); V2/V3 match at anchor+0 and V1 at anchor+4 (slots of ONE file float
   independently by record-list insertions — the ±4 float observed across slots).
   Each slot's 5-bit pattern (310 SET/CLEAR per attribution, 300/320/330/340 CLEAR by
   s2-before-captures + set-monotonicity) matched at exactly one base within ±64 of
   the anchor; full-EF scans showed far-away pattern matches are static constants
   refuted by the anchor transition contrast (only 482,865 shows no,rest,yes =
   0,0,1). Bonus finding: the "V3 - no" file is byte-identical to "V1 - after picked
   up rowId_1044360310" (sha match), directly attributing V1's SET state; the
   treasure content is Golden Rune [1] (reward corroboration on the anchor pair).
   S2/S7 ROOT PAIRS DONE 2026-07-06: added 7 pairs from the `snapshots-root` corpus
   (2026-02-09 session), corpus/slot overridden per pair (slot 2 = V1, slot 7 = an
   uncharacterized instrument character not among the five registered slots).
   3 world pickups resolved for s2-V1 (rowIds 1044360320/330/300, family base
   482,907) and 4 for s7 (rowIds 1042360030, 1044360310/340/330). NEW FINDING:
   s7's tile-pickup-row-id base measured 482,861 at 21:51 and 482,931 at 22:15-22:21
   — a ~70-byte shift **within the same ~30-minute session**, amending the earlier
   "stable within a session" assumption to "can drift between individual captures
   within a session, not just between sessions"; the per-pair isolated-flip +
   candidate-resolution design tolerates this because each candidate's cross-checks
   use its own file's current base, never a cached one. The four s7 world-state-b
   pairs (60220 progression, 71800/76101 graces) did NOT resolve — isolated-flip
   scans returned zero or many candidates, consistent with the evidence catalog's
   own warning that this corpus has cross-session churn and the 71800 pair's
   "flag-byte interpretation is unresolved"; left unresolved rather than forced.
   All 27 pairs total re-verified deterministic (`knowledge run` reports "claims
   store unchanged" on re-run) and the full test suite (116 main + 3 regression +
   52 wasm + 4 conformance) stays green.
   TIMELINE RE-ANNOTATION ATTEMPTED 2026-07-06 (`er-save-reader knowledge timeline`,
   `src/knowledge/timeline.rs`): replays the Bee corpus's sparse-diff chain (slot 5,
   3,830 captures, 2026-02-14..2026-05-25, verify-on-read against the evidence
   catalog) into an in-memory slot buffer and runs the reference grace detector at
   every step. Replay is self-consistent (1,194,422,113 records, 0.68% old-value
   mismatch rate against the reconstructed state — matches the 2026-07-05 audit
   exactly) and grace detection stays confident for 2,735/3,830 entries (ef_offset
   drifting 72,609-82,586 over the chain). RE-ANNOTATING WHICH FLAGS SET WHEN WAS
   ATTEMPTED AND REJECTED on evidence: unlike `cmd_run`'s attributed pairs, there is
   no known before/after transition here to anchor a bounded family-base search, so
   a blind full-EF scan for the world-state-b tutorial-anchor 4-bit pattern
   (71800/71801/76100/76101) was tried — first unbounded (0 matches), then bounded to
   the empirically-established base cluster (~130k-160k, still 2-3 candidates), then
   gated by a 3-entry base-stability streak. The streak-gated version still produced
   32,893 "events" naming only 16,174 distinct flags, with some flags "transitioning"
   0→1 up to 69 times — logically impossible for a monotonic bit, and decisive proof
   the resolved base was hopping between the real region and a coincidentally-matching
   one. This result was NOT shipped (would violate ADR-0004 / the False Negative
   Investigation Protocol); the command now emits only the replay-and-detect audit
   (`knowledge/claims/timeline-replay-audit.json`) and asserts no flags. Next viable
   design (not attempted, out of scope for this increment): cluster grace-aligned
   isolated flips (reusing the same ±16-neighborhood test from `pipeline.rs`) across
   every consecutive pair in the whole chain, and locate the family base from where
   many independent flips agree, rather than re-deriving a base from a single state.
   REMAINING in step 3: the four s7 world-state-b pairs above remain unresolved pending
   more/cleaner captures; the timeline flip-clustering design above, if someone wants to
   pursue it.

   **GOLDEN CENTIPEDE GAP CLOSED — AS A NEGATIVE, 2026-07-22.** c06-c08 was carried here
   as "lacks a flag annotation — data gap, not a mechanism gap". **That diagnosis was
   wrong: it is neither.** The action sets no event flag at all, so no quantity of further
   captures can produce a flag hypothesis. Recorded in
   `knowledge/inputs/attributed-transitions.json` → `excluded_captures` (an inert key the
   pipeline ignores) so the pairs are not re-proposed.
   - **Primary game data.** Golden Centipede is goods 20820, a *crafting material gathered
     from an environment asset*, not a treasure lot. `AssetEnvironmentGeometryParam` row
     99820 has `isEnableRepick="1"` and `pickUpItemLotParamId="998200"`; `ItemLotParam_map`
     row 998200 has **`getItemFlagId="0"`**. Nothing is written — which is exactly why the
     gathering point respawns. The one flagged Golden Centipede lot in the game (row
     35000340 → flag 35007340) is in Subterranean Shunning-Grounds; the character is in
     m60_43_50 for all of c05-c10 and 35007340 reads CLEAR throughout.
   - **The pickups are real**, so the captures are not mislabeled: goods 20820 quantity
     rises 14→15→16→17 across c06→c09 while the player moves under 2 units.
   - **Null result, with the control that makes it mean something.** The pipeline's own
     isolated-flip rule finds ZERO isolated pure 0→1 single-bit sets in either centipede
     pair (all three files detect the same `ef_offset` and resolve identical bases for all
     five families, so alignment is not in question). The *identical* test on the two
     established pairs of the same series — c05-c06 (Seal 1043500000) and c09-c10
     (Butterfly 1043500010) — does find the flip, and the readers confirm 0→1 staying SET.
     Same instrument, same character, same map: a measured absence, not a blind spot.
   - **Two decoys, named so they are not rediscovered as findings.** EF-relative byte 1221
     increments on every centipede pickup — but also across the c09-c10 control, and by 13
     over unobserved play; it is a counter. EF-relative 851264 oscillates 0x80/0x00 across
     c05-c10 and is not set-monotonic.
   - **Generalizes**: any pickup whose lot has `getItemFlagId=0` is permanently
     unattributable. Check that field *before* capturing a pair, not after.

   **SEGMENT CENSUS DONE 2026-07-22** (`knowledge timeline-segments`,
   `src/knowledge/timeline_segments.rs` → `knowledge/claims/timeline-segments.json`).
   Replaces v0.36.1's long-gap SAMPLE ("at least 21 boundaries") with a full scan of all
   3,868 consecutive pairs: **27 boundaries, 28 segments**. Findings:
   - **Continuity is all-or-nothing.** All 3,837 continuous pairs agree at *exactly*
     100.00% — the 95-100% histogram bucket is EMPTY. There is no "mostly continuous",
     so the classification threshold's exact value is irrelevant, which is a much
     stronger position than picking a defensible cut.
   - **Two pair-test failures are NOT segment cuts** (`sd_001663→sd_001664` at 6.45%
     pair / **99.98% replay**, 31 shared offsets; `sd_002371→sd_002372` at 3.23% /
     99.00%). Unobserved play makes a save stale *everywhere*; these are stale in a
     handful of bytes. Corroborating the local test against the global replay
     (`docs/CORROBORATION-SYSTEM.md`) is what separates them — the naive count was 29.
   - **The 30-minute gap heuristic is weak in both directions**: 23 hits, **14 false
     alarms** (38% of long gaps are NOT boundaries) and **4 boundaries missed** at gaps
     as short as 228s. v0.36.1 already suspected this; the census quantifies it. Do not
     use gap length as a boundary test.
   - 2 genuinely ambiguous pairs (~80%) are reported, not classified.

   **FLIP-CLUSTERING PREREQUISITE TESTED AND FAILED 2026-07-22** (`knowledge
   timeline-flips`, `src/knowledge/timeline_flips.rs` →
   `knowledge/claims/timeline-flip-monotonicity.json`). Before building the clustering,
   the cheap falsification: does confining extraction to a segment remove the
   set-monotonicity violation that sank the 2026-07-06 attempt? **No.**
   - Same isolated-flip rule as `pipeline.rs` (identical ±16 neighborhood, grace-aligned
     per state), run twice — boundaries respected vs ignored — so only the segment
     constraint differs.
   - Boundaries ignored: 108,215 repeat-violations, worst 57×. Boundaries respected:
     **107,183**, worst **still 57×**.
   - **The decisive statistic is enrichment, not the raw drop.** Excluding any pairs must
     remove some violations. Excluding 0.878% of pairs removed 0.954% of violations —
     **enrichment 1.09×**. Boundary pairs violate at the same rate as ordinary pairs.
   - **This refutes the v0.36.1 causal claim** that boundary-crossing was "the mechanism
     behind the previous attempt's flags transitioning 0→1 up to 69 times". The boundary
     finding itself stands; its proposed *consequence* does not. `CONTEXT.md` → *Kill
     Transition* carries the correction.
   - **Where the cause most likely actually lies** (hypothesis, UNVERIFIED, stated as the
     next thing to test rather than a conclusion): the alignment, not the time window.
     Both arms align two states by each state's own `detect_event_flags_offset_impl`
     result — a single global anchor. But this project's own settled finding is that
     **every family base floats independently per save, and can drift even between
     captures within one session** (the s7 ~70-byte intra-session shift, step 3 above;
     `CLAUDE.md` → Single Source of Truth). If families float relative to the detected
     anchor, then `grace_rel` i in state A and `grace_rel` i in state B are not the same
     flag, and a "bit" will appear to set repeatedly because it is not one bit. That
     would make repeat-violations a property of the *alignment model*, which no amount
     of time-window discipline can fix. Testing it means per-family base resolution at
     every replayed state — which is exactly the thing the 2026-07-06 attempt could not
     do without an attributed anchor. **The design is blocked on that, not on segments.**
### Pickup family cutover — PARTIAL, 2026-07-20

**World pickups CUT OVER** (`tile-pickup-row-id`). `src/ui/world_pickups_view.rs` and
`knowledge grace-dump` resolve per save via `wasm_event_flags::is_tile_pickup_set`.
`FAMILY_TILE_OPEN_WORLD` = 454,067 also established (two attributed boss-kill pairs,
exact agreement, corroborated by the claims store's bases sitting 500 bytes apart) —
thinner evidence than the other constants, and no UI consumer yet.

**Design finding that cost a bug: a bare 10-digit tile id is AMBIGUOUS.** Open-world
flags and pickup row_ids both have localId < 7000 and live in regions 500 bytes apart,
so nothing in the value distinguishes them. The first cut auto-routed on local id and
sent 1,753 pickups to the open-world base — reading a plausible wrong bit, not failing.
Caught by a sanity count (11 collected for a mid-game character), not by a test.
The API is now split — `is_tile_pickup_set` / `is_tile_world_flag_set` — and the caller
must choose. `pickup_flags::is_flag_set_with_status` was deliberately NOT cut over for
this reason: it takes a bare id and cannot know the family.

~~`pickup_data.rs` stores `event_flag` = `item_lot_id` = the ROW ID, not the
getItemFlagId. `is_tile_pickup_set` accepts either form and normalises.~~
**CORRECTED 2026-07-22 — that convention was a bug, not a design.** It reads the right
bit only while `row_id + 7000 == getItemFlagId`, which is false for 124 of the 1,691
ten-digit rows; 220 entries resolved to the wrong bit or the wrong family. `event_flag`
now holds the `getItemFlagId` for every entry, pinned to the primary source by
`test_event_flags_match_primary_source`. See the world-pickup regeneration entry below.

*Verification.* All 23 verified flags in the claims store read clear->set through the
shipped functions (`knowledge validate-origin` part C), with the 4 hypotheses correctly
excluded by the status ladder rather than asserted on. Live-save counts match the
documented character designs, including V3 reading exactly 0 pickups — the character
built as a true-negative control.

*Remaining.* Dungeon pickups (`legacy-dungeon-pickup`, constant 1,500,442 known) need
the eventflagalloclist map->slot table embedded in the reference implementation; the
layout is `alloclist_slot * 1125 + (flag % 10000) / 8`, local >= 7000. 3,577 of 4,809
entries in `WORLD_PICKUPS` still read UNKNOWN: 1,996 dungeon flags, 1,049 non-tile
families, and 532 tile ids outside the open-world tile grid (likely DLC/underground
maps — unexplained, worth a look before claiming pickup coverage).

---

4. **Freeze `ground_truth_offsets.json` read-only**; per-family cutover to the claims
   store (graces → boss defeats → pickups), legacy entries promoted or tombstoned.

   **4a. FREEZE — DONE 2026-07-19.** `metadata.frozen` block added to the store itself
   (authority, enforcement, convention warning, per-family cutover state, and the two
   known-bad entry classes: tombstoned tile base 337375, and the catacombs/tunnels
   families that describe a u32-record list rather than the flag bitmap). Enforced by
   `tests/regression_suite.rs::test_ground_truth_is_frozen`, which pins the file's
   sha256 (`5b2256d1…`) and asserts the marker survives; verified to actually bite by
   perturbing one offset and observing the failure. Audit finding: **nothing in the
   repo writes this file** — `build.rs` codegens `src/generated/ground_truth.rs` from
   it and `tests/regression_suite.rs` reads it, so the freeze cost nothing to impose.

   **4b. CUTOVER — BLOCKED, and it is not a data migration.** The two stores use
   incompatible models: the legacy store holds ABSOLUTE per-flag offsets (71000 →
   offset 9315), while every claims-store family is `base_is_per_save: true`. So legacy
   entries cannot be mapped onto claims entries one-for-one — for a family to flip, the
   app must detect that family's base *in the save in front of it* and then apply the
   layout formula. The claims store's contribution is the verified LAYOUT (e.g.
   world-state-b `(flag−50000)/8`), which covers every flag in the family; what is
   missing is the per-save base.

   The missing capability is a **single-save family-base detector**. The pipeline
   measures bases from attributed before/after transition PAIRS (isolated-flip
   analysis), which the app cannot do — it has one save, no pair.

   > **SUPERSEDED 2026-07-22, and `src/calibration.rs` is deleted.** The paragraph below
   > held that file up as the right *shape* for the missing single-save detector. That
   > detector was built instead by pinning the Origin — see `:503` "Next: pin the single
   > origin. That is now the whole of 4b", established 2026-07-20 — and it lives in
   > `crates/wasm-event-flags` as `find_flag_list_end_in_ef` /
   > `resolve_family_base_in_ef`. The bounded-anchor-scan approach was never taken up.
   > The file was removed because it was unreachable (one `mod` declaration, zero
   > callers) while asserting the tombstoned premise below as fact in its module doc, and
   > its tests pinned 337,375 as a base. Kept here because the *reasoning* — why a
   > pre-reset calibrator could not simply be re-pointed — is what stops it being
   > re-proposed.

   Nearest existing mechanism ~~is~~ *was* `src/calibration.rs`, which is the right shape
   (bounded anchor scan, ≥3 anchor matches across ≥2 distinct tiles, window
   430k–510k) but is entirely pre-reset:
   - its header comment asserts the tile base "is constant across saves" — the exact
     claim the per-family float finding refuted, citing the now-tombstoned 337375;
   - its anchors/windows are EF-relative from the GT era (tile_base 446321 / 453473)
     while the claims store measures grace_rel (~483.4k) — different conventions,
     not comparable as written;
   - it covers tiles only. There is **no world-state-b calibrator at all**, and
     ADR-0006 puts graces first.

   Note the hazard is already charted: blind full-EF pattern scanning for the
   world-state-b tutorial anchors was tried during the timeline work and produced
   32,893 bogus "events" with flags flipping 0→1 up to 69 times. A *bounded*
   multi-anchor scan is a different animal and is NOT refuted by that result — but it
   must be re-grounded on claims-store conventions and re-verified against the
   attributed pairs (whose bases are known) before any family flips.

   So 4b is a discovery task, not a refactor: build and prove a single-save
   world-state-b base detector, validating it against the pairs where the pipeline
   already knows the right answer. Only then do grace entries get promoted/tombstoned.

   **4b INVESTIGATION 2026-07-19/20 — the float is quantized, not arbitrary.**
   Decision taken: pursue the structural theory (the location must be predictable
   from the file's own structure, since the game itself reconstructs everything from
   one file) with a pre-registered pass criterion — back the constant out of every
   fixture independently, and *all* fixtures must agree on ONE number. Zero residual
   was NOT required (the fixed table is known bad); zero spread was.

   *Confirmed:* an independent Python reimplementation of `find_ga_items_end`
   reproduced all 8 conformance goldens exactly. GaItems parsing is genuinely
   understood, not fitted.

   *Test 1 (EF region start vs ga_end) — INCONCLUSIVE, measurement contaminated.*
   No single count field explains the gap. But the goldens it was tested against are
   outputs of the byte-by-byte scan in `detect_in_window`, whose own tie-break comment
   calls the plateaus it chooses between "small shifted echoes". The near-identical
   low-progression slots 1-4 share one ga_end yet their gaps differ by 29/21/8 bytes —
   the same order as the scan's ambiguity. **Do not treat the golden EF offsets as
   structural truth; they are scan results.** Gross structure is real and far above the
   noise floor (low-progression gap ~35.3k vs mid-game ~36.5k, a ~1,200-byte
   progression-linked difference); fine structure is unmeasurable this way.

   *Test 2 (family base vs ga_end) — byte-exact, and the key result.* `grace_base +
   family_base_grace_rel` recovers the exact absolute base (the scan jitter cancels,
   since the rel value was computed against that same grace_base). Deltas from ga_end:

   | family | n files | delta range | spread |
   |---|---|---|---|
   | world-state-b | 6 | 183,101–183,157 | 56 |
   | tile-open-world | 2 | 520,008–520,016 | 8 |
   | tile-pickup-row-id (snapshots-root) | 8 | 518,200–518,248 | 48 |
   | tile-pickup-row-id (snapshots-confessor) | 7 | 520,476–520,520 | 44 |
   | legacy-dungeon | 2 | 1,566,516–1,566,520 | 4 |
   | legacy-dungeon-pickup | 3 | 1,566,387–1,566,391 | 4 |

   Findings: (1) the base is NOT a fixed distance from ga_end, so ga_end alone cannot
   position a family — decisively shown by the 8 `snapshots-root` files, which share
   an identical ga_end of 41,448 yet spread 48 bytes; (2) but the variation is ~0.03%
   of the magnitude, so the gross structure is essentially pinned; (3) **every step in
   every family is a multiple of 4** (steps observed: 4, 8, 12, 16, 20, 32, 36);
   (4) tile-pickup-row-id splits cleanly by character (~518.2k root vs ~520.5k
   confessor) with only ~48 bytes of spread *within* each — the "2,320-byte float"
   is a between-character offset, not per-save chaos.

   *Interpretation (inference, not yet proven):* a variable-length u32 record list
   sits between ga_end and the flag families, so
   `family_base = ga_end + FIXED(family) + 4 × record_count`. This is consistent with
   the pipeline's independent finding that the old "catacombs" 28-31k span is a
   u32-record LIST rather than a bitmap. It also reconciles the two facts that looked
   contradictory: a fixed-size bitmap cannot reflow, yet the bases move.

   *Decisive next experiment (blocked on a pipeline run, not on knowledge):* measure
   ALL family bases in EACH file and test whether the distance BETWEEN families is
   constant. This could not be run from the claims store — each pair records only the
   family of its own flag, so no file currently carries two bases. If inter-family
   distances are constant, then locating ONE family locates every family, and 4b
   reduces to pinning a single origin plus reading one record count.

   **INTER-FAMILY TEST DONE 2026-07-20 — CONSTANT, and it collapses 4b.**
   `er-save-reader knowledge family-distances` (`src/knowledge/family_distances.rs`,
   emits `knowledge/claims/family-distances.json`, byte-identical on re-run). It
   re-measures each family in files where no flag of that family flipped, by finding
   the UNIQUE position in a bounded window at which every expected flag state holds
   (expectations from set-monotonicity over pipeline-verified flips, plus the
   `known_set_before_all_pairs` anchors; `MIN_ANCHORS` = 3, window = the family's
   measured delta range ±512). 48 files measured, 37 carrying ≥2 family bases.

   | distance | n files | value | spread |
   |---|---|---|---|
   | tile-pickup-row-id → world-state-b | 37 | −337,375 | **0** |
   | legacy-dungeon-pickup → world-state-b | 16 | −1,383,250 | **0** |
   | legacy-dungeon-pickup → tile-pickup-row-id | 16 | −1,045,875 | **0** |

   Zero spread on all three, and they are mutually consistent by an arithmetic check
   that was not imposed by construction: −1,383,250 − (−1,045,875) = −337,375 exactly.

   **The families are rigidly locked to each other.** Each family's distance from
   ga_end wanders (the 4-byte-quantized record-list growth above), but they all wander
   *together*. So locating ONE family locates all of them, and 4b reduces from "build a
   detector per family" to "pin a single origin, then add a known constant."

   **−337,375 is the tombstoned constant.** Tombstone `tile-base-337375-grace-anchored`
   retired 337,375 as a tile base, and that refutation stands — it was being used as an
   absolute offset from the disproven structural anchor. But the NUMBER was never
   wrong: it is the exact, invariant distance between the tile-pickup and world-state-b
   families. The old ground truth had measured a real structural invariant and
   misattributed it to the wrong origin. Worth remembering before dismissing other
   legacy constants as noise — some may be real distances wearing the wrong anchor.

   *Honest limits of this run.* The search windows are centred on prior measurements,
   so this command re-measures known families; it does NOT locate a family from nothing
   and must not be cited as doing so (the emitted JSON says so in its `method` block).
   Unresolved: `tile-open-world` never resolved (29 files, too few anchors — only 2
   verified flags exist for it); `world-state-b` found no candidate in 15 files and
   `legacy-dungeon` in 6, which needs a look — likely captures predating the tutorial
   anchors, or the churny s7 files the evidence catalog already warns about.

   *Next:* pin the single origin. That is now the whole of 4b, and the ±512 windows
   plus three mutually-consistent constants give it a far tighter target than the blind
   scan that failed in step 3.

   **ORIGIN PROBE 2026-07-20 — the drift is monotonic; no count field explains it.**
   `er-save-reader knowledge origin-probe` (same module, emits
   `knowledge/claims/origin-probe.json`, byte-identical on re-run). Origin proxy is
   world-state-b (resolved in 47 files, all `snapshots-confessor`). Its delta from
   ga_end takes 7 distinct values over 183,101–183,157, and they are ordered by
   capture sequence:

   | capture | delta | step |
   |---|---|---|
   | Confessor 01 (earliest) | 183,101 | — |
   | b1 | 183,133 | +32 |
   | b19 | 183,137 | +4 |
   | b25 | 183,141 | +4 |
   | b33 | 183,145 | +4 |
   | b38 | 183,153 | +8 |
   | b43 (latest) | 183,157 | +4 |

   **Monotonically growing, every step a multiple of 4.** The drift is a structure
   being appended to as the character plays — it never shrinks. This is the clearest
   confirmation yet of the record-list model, and it also means the drift is a
   *function of progression*, not of save/load noise.

   *Negative result:* no single u32 count field explains it. The probe searched
   [0, 190,000) from BOTH `ga_end` and `grace_base`, multipliers 1/2/4/8/12, requiring
   `delta − mult × count` to be identical across all 47 files. Zero candidates.
   (The two anchors matter: the ga_end→EF variable section has ~1,277 bytes of spread,
   so a count stored after it is not at a stable offset from ga_end. Neither anchor
   worked.) The first run of this probe used a 70,000 span, which never even reached
   the origin proxy at ga_end+183k — the full-span re-run is the one that counts.

   *What this does and does not mean.* The record-list model is NOT refuted; the
   single-count *form* of it is. Live hypotheses, in rough order of promise:
   (1) the list is sentinel/terminator-delimited rather than length-prefixed, so its
   size is found by scanning for the terminator and no count exists to find — this
   would explain every observation and is still fully parseable;
   (2) the count lives outside the searched span (before ga_end, or in a section not
   covered); (3) several lists sum to the observed drift; (4) the width is not u32.

   *Next experiment:* stop looking for a count and look for the LIST. Scan the region
   between grace_base and world-state-b for a run of fixed-width records ending in a
   terminator, and test whether the record count tracks the drift across the 47 files
   in capture order. The monotonic ordering above is a strong constraint: any correct
   model must reproduce that exact sequence of steps (32, 4, 4, 4, 8, 4).

   **LIST FOUND 2026-07-20 — the origin is pinned. `knowledge list-hunt`**
   (`src/knowledge/family_distances.rs`, emits `knowledge/claims/list-hunt.json`,
   byte-identical on re-run). Method: differential alignment. For two captures whose
   measured family delta differs, find every position where the byte alignment between
   them shifts; each shift change is a variable-length structure.

   *The list.* Every diffed pair puts its FIRST shift at ga_end+65,7xx, and the shift
   there already equals the pair's TOTAL family drift — everything later (72k–77k)
   churns but nets back out. The bytes there are 4-byte little-endian records
   (`0x00125764, 0x00125752, … 0x002f7859, 0x002f785a`), and the later capture has
   exactly one more appended (`0x002f7858`). An **append-only u32 list**. The detected
   boundary itself creeps by exactly the previous pair's growth (65,727 → 65,731 →
   65,735 → 65,739 → 65,747), confirming it is the list's END.
   At grace_rel ≈ 29,200 this is the same structure the pipeline independently called
   "a u32-record LIST, not the flag bitmap" (the old "catacombs" 28-31k span) — two
   separate lines of evidence landing on one structure.

   *The payoff.* Measuring from the list's end removes the drift completely:

   | family | n | base − list_end | spread |
   |---|---|---|---|
   | world-state-b | 47 | **117,192** | 0 |
   | tile-pickup-row-id | 38 (37 confessor + 1 root) | **454,567** | 0 |
   | legacy-dungeon-pickup | 16 | **1,500,442** | 0 |

   These reproduce the independently measured inter-family distances exactly:
   454,567 − 117,192 = 337,375 ✓ · 1,500,442 − 117,192 = 1,383,250 ✓ ·
   1,500,442 − 454,567 = 1,045,875 ✓. Two separately derived measurement chains agree
   to the byte.

   **So a single save with no history can position every family:** parse ga_end
   (already exact), find the list end, add the family constant. No before/after pair,
   no scoring, no scan of the flag region.

   *Honest limits — this is not finished.* (1) `find_list_end` is a heuristic, not a
   format parse: skip to the first non-zero byte at/after ga_end+60,000, then take the
   first 64-byte zero run. Both constants are empirical, and a character whose list is
   far longer or shorter could walk out of that window. A real parse of the enclosing
   section should replace it. (2) Cross-character evidence is thin: 47 of the files are
   one Confessor, and the only non-confessor confirmation is a SINGLE `snapshots-root`
   file (which does hit 454,567 exactly). Validating against the conformance fixtures
   and the V1/V2/V3 slots is the necessary next step before any of this is wired into
   the app. (3) The constants are measured, not derived from the format.

   **CROSS-CHARACTER VALIDATION 2026-07-20 — the model holds out-of-sample.**
   `er-save-reader knowledge validate-origin` (emits
   `knowledge/claims/origin-validation.json`, byte-identical on re-run). Predicts each
   family base from the slot's own bytes only — `ga_end + find_list_end(slot) +
   constant` — then checks against states established independently of the model.

   *A. Multi-slot differentials — 9/9 PASS.* V1 (slot 2), V2 (slot 3) and V3 (slot 4)
   across three instrument files each, 5 exact expected bits per file (rowIds
   1044360300/310/320/330/340), **including the CLEAR ones**, which are the real
   discriminator — a mislocated base fails those first. Predicted bases differ per
   slot exactly as the per-slot float predicts (559,656 / 559,652 / 559,644), and V3's
   third file correctly shifts to 559,652 after its anchor transition. These three
   characters contributed nothing to deriving the constants.

   *B. Backup saves, five characters, tutorial grace anchors.* Slot 0 (Confessor) and
   slot 1 (Wretch) PASS 4/4 on both backups; Wretch is fully out-of-sample. Slots 2-4
   read 71801/76101 SET but 71800/76100 CLEAR, identically across all three characters
   and both backups.

   *That is a bad expectation, not a bad base.* `known_set_before_all_pairs` is an
   assumption about the CONFESSOR's state; applying it to V1/V2/V3 was out of scope —
   my test-design error. The command now discriminates the two explanations directly:
   it searches ±4096 around the prediction for any base at which all four anchors read
   SET. **There is none, for any of the three slots.** A mislocated base would have a
   rescue position; a genuinely untouched grace has none. Combined with part A
   validating these very characters, the model is corroborated rather than refuted.
   Not proven, though: settling what V1/V2/V3 actually touched needs independent
   evidence about those characters, which `save_slot_registry.json` does not carry
   (it is a feature registry, not per-slot state). The 6 are recorded as FAIL, not
   quietly reclassified.

   *Standing.* The origin model now validates on five distinct characters (Confessor,
   Wretch, V1, V2, V3) across two backup saves and the snapshots-root corpus. The
   remaining weakness is no longer cross-character coverage but the heuristic in
   `find_list_end` (empirical probe start and zero-run length rather than a parse of
   the enclosing section), which is what should be hardened before graces are cut over.

   **HARDENED 2026-07-20 — resolver moved into the reference implementation.**
   `crates/wasm-event-flags/src/lib.rs` now owns the origin: `find_flag_list_end`,
   `resolve_family_base`, the three family constants, and WASM exports
   (`flag_list_end`, `family_base`). `src/knowledge/family_distances.rs` DELEGATES to
   it — the local copy is deleted, so the pipeline and the app cannot disagree about
   where a family is (ADR-0005). Re-running `list-hunt` and `validate-origin` after
   the cutover reproduced every number byte-identically, including all 19 validation
   verdicts and the three list-end constants, so the delegation is verified rather
   than assumed.

   *Anatomy checked first (5 backup characters).* The list has NO length prefix —
   the 32 bytes before its start are zeros in every slot — which is why the earlier
   single-count search found nothing. Record counts track progression (Confessor 291,
   Wretch 112, V1 111, V2/V3 110). So the end genuinely has to be scanned for; the
   hardening makes that scan honest rather than replacing it.

   *What hardening actually means here.* The resolver checks its own assumptions and
   returns `None` rather than a plausible-looking wrong answer, because a wrong base
   reads garbage flags silently:
   - the probe point must be followed by `ORIGIN_MIN_LEAD_ZEROS` (256) actual zeros,
     proving we started in the gap and not inside the list, where the "start" would
     be wherever we happened to enter;
   - `(list_end - ga_end)` must land in [55,000, 80,000] (observed 63,629-65,949);
   - the resulting base must lie inside the data.
   Probe start moved 60,000 → 50,000 to sit further from the earliest observed list
   start (63,187), with the lead-zero check as the guard rather than luck.

   *Locked by `crates/wasm-event-flags/tests/origin_conformance.rs`* (6 tests):
   golden ga_end + list_end for all 8 fixtures; the declared sanity range must contain
   real saves with ≥2,000 bytes of margin at both ends (bounds that only just fit are
   bounds about to reject a valid save); the family constants must reproduce the
   independently measured inter-family distances; and refusal on empty, all-zero,
   too-short, and truncated input. Writing these caught a real one: `resolve_family_base`
   correctly refuses on the 128k fixtures because the bases lie at ~228k, i.e. the
   bounds check fired on the test author first.

   **GRACE FAMILY CUT OVER 2026-07-20 — first family off the legacy store.**
   Graces no longer read `ground_truth_offsets.json`. Positions resolve per save via
   `wasm_event_flags::is_world_state_flag_set` (world-state-b, located from the
   append-only list's end). `metadata.frozen.cutover_state.graces` records this and the
   freeze digest was re-pinned in the same commit — the workflow the freeze test was
   built for.

   *Both grace read paths were cut over*, not just the visible one: the database view
   (`src/ui/database/graces_view.rs`) and the view-model status used by the events view,
   comparison and export (`src/vm/events.rs`). Leaving one on legacy offsets would have
   had two screens disagree about the same grace. `src/main.rs`'s region filter also
   moved, where unresolved deliberately means "offer no region" rather than "offer all".

   *Unknown is now a first-class state.* `GraceStatus::Unreliable` / `Option<bool>::None`
   render as "-", never as "not discovered". Collapsing unknown to false is how
   `batch-validate` reported 0/110 boss defeats on a progressed character.

   *Legacy compensation deleted, not just bypassed* (70 lines): `check_progression_gate`
   overrode the byte with an inference ("prerequisite boss not defeated → report not
   discovered") and `get_calibrated_grace_status` re-derived a base for "unreliable"
   blocks. Both existed to suppress false positives from wrong offsets; against a
   correctly resolved position the gate can only manufacture false NEGATIVES, hiding a
   grace the player has. `PROGRESSION_GATES` is kept as prerequisite documentation.

   *Verification.* Shipped EF-relative path agrees with the validated slot-absolute path
   on 10/10 slot-checks across both backups. Conformance extended to 8 tests, including
   that the two paths resolve the same absolute byte, and that an unresolvable read is
   `None` rather than `Some(false)`. Aggregate smoke test matches the catalog's own
   character descriptions:

   | slot | character | graces discovered | catalog says |
   |---|---|---|---|
   | 0 | Confessor | 179 / 421 | mid-game progression |
   | 1 | Wretch | 6 | "a few graces/pickups" |
   | 2-4 | V1/V2/V3 | 2 each | "very little progression" |

   Zero unknown on all five. **This also closes the 6 outstanding validation FAILs**:
   V1/V2/V3 have exactly TWO graces, and they are 71801 (Stranded Graveyard) and
   76101 (The First Step) — so 71800/76100
   reading clear is corroborated by the independent total, not merely consistent with it.
   The tutorial-anchor expectation was wrong, as suspected; the model was not.

   *Remaining families.* boss_defeats still needs a verified layout for non-dungeon
   bosses. Pickups are now wiring rather than discovery: both pickup family layouts are
   verified and the origin resolves. (Both settled 2026-07-20 — see the legacy-dungeon
   cutover below.)

   *Still not derived.* The constants are measured, and the scan is bounded-structural
   rather than a parse of the enclosing section. A full parse would need the section
   layout around grace_rel 29k, which nothing in the corpus documents yet.

   *Two method bugs worth remembering.* The first list-end scan returned garbage
   because ga_end+60,000 is itself inside a zero gap, so it terminated instantly and
   produced values that were exactly `delta − 60,000` — a "constant-looking" failure
   mode that only showed up as suspiciously round arithmetic. And differential
   alignment silently lies inside zero runs, where EVERY shift matches: the search must
   require the sync window to contain real bytes (`MIN_INFORMATIVE`) or the whole
   sparse bitmap reads as "shift 0" and insertions inside it become invisible.

   **LEGACY-DUNGEON FAMILY CUT OVER 2026-07-20 — boss_defeats leaves the legacy
   store; every family now has a constant.** `FAMILY_LEGACY_DUNGEON = 1,500,567`.

   *Why list-hunt never produced it.* `list-hunt` derives constants from
   `family-distances`, which re-measures a base by finding the unique window position
   where ≥3 known flag states hold. `legacy-dungeon` and `tile-open-world` have only
   two verified flags each, so no file ever carries enough anchors and both were absent
   from its table — not because their bases were unknown, but because that particular
   instrument could not see them.

   *What produced it.* `er-save-reader knowledge family-constants`
   (`src/knowledge/family_distances.rs`, emits `knowledge/claims/family-constants.json`).
   The pipeline already pins a family base in the pair that established each flag, by
   isolated-flip analysis, and records it — a *stronger* positioning than a windowed
   pattern match, just present in fewer files. The command re-expresses those
   measurements against the resolver's own origin:

       constant = (grace_base + family_base_grace_rel) − (ga_end + list_end)

   It reproduces all four already-shipped constants exactly (world-state-b 117,192 over
   6 files, tile-open-world 454,067 over 2, tile-pickup-row-id 454,567 over 15,
   legacy-dungeon-pickup 1,500,442 over 3) — a second derivation chain agreeing with
   the first — and yields legacy-dungeon at 1,500,567 from its two boss-kill pairs
   (30020800 b24-b25, 30030800 b32-b33), spread 0 across a drift step.

   *Thin, and labelled thin.* Two files, both catacombs (alloc slots 82 and 83). The
   only cross-check the constant has is its 125-byte separation from the pickup family,
   which `origin_conformance.rs` now pins. Note 125 ≠ the "~129 bytes" in the pipeline's
   own family note: that number was a cross-file subtraction of bases measured at
   different list lengths. The list-end-relative distance is the invariant one.

   *Layout comes from the game, not from the stride assumption.* The wasm crate now
   carries `LEGACY_ALLOC_SLOTS` (99 maps) copied from the game's own
   eventflagalloclists, with a conformance test that re-reads the source file and fails
   on drift. The two maps the game allocates TWICE (m34_12 → 62 and 144, m40_00 → 70
   and 170) resolve to `None`, because nothing establishes which allocation holds the
   bits and a guess reads a wrong bit ~92KB away. This table replaces nothing less than
   `get_dungeon_general_bases()`, whose own audit comment records entries disproven by
   every save on this machine.

   *Cut over.* `bosses_view` (both id widths, routed explicitly by family) and the
   dungeon-pickups table in `events.rs`, which had been positioning flags with
   `DUNGEON_PICKUP_BASES` plus the pickup's `dungeon_area`/`section` fields — those are
   now display data only and can no longer disagree with the flag they label.

   *Out-of-sample verification.* `validate-origin` part C rose from 23 to 28 verified
   flags reading clear→set through the SHIPPED functions, with zero families skipped
   for want of a read function. Live counts match the documented character designs:

   | slot | character | bosses | dungeon pickups | catalog says |
   |---|---|---|---|---|
   | 0 | Confessor | 51 | 382 | mid-game progression |
   | 1 | Wretch | **1 — Soldier of Godrick** | 0 | "only the tutorial enemy (Soldier of Godrick) defeated" |
   | 2-4 | V1/V2/V3 | 0 | 0 | "very little progression" |

   The Wretch line is the strongest single result here: a character that contributed
   nothing to deriving the constant, whose one defeated boss is named correctly by the
   read, out of 205 candidates.

   **It also corrected the knowledge base.** The evidence catalog said the 2026-01-11
   backup slot 0 "predates the Margit/Godrick/Radahn kills — zeros at their flag bytes
   are TRUE negatives". Read at the resolved base, **Margit and Godrick are defeated**;
   only Radahn is not. That 2026-07-05 note measured zeros at the pre-migration offsets,
   i.e. at the wrong bytes — the failure this migration exists to remove, caught
   annotating the evidence itself. Corroboration: both bits sit beside their found_flags
   (10000801, 10000851), the m10_00 block carries 96 non-zero bytes of 1125 where slots
   1 and 4 carry 5 and 0, and `DATA-SOURCES.md`'s own character description says the
   Confessor felled all three. Catalog and CLAUDE.md corrected.

   *And it retires an old disproof.* m18 (Stranded Graveyard) was "DISPROVEN and
   removed" from `get_dungeon_general_bases()` because its span read all zeros, with the
   reasoning that "the true m18 general section lies near 2.5k-4k". Via alloc slot 35
   and a resolved origin, m18's tutorial boss now reads correctly on both the Wretch and
   the Confessor. The layout was always right; only the base was wrong — the same lesson
   as 337,375. Before dismissing a legacy constant, check whether it is a real
   structure wearing the wrong anchor.

   *Three boss ids were wrong in the database.* `bosses_data.rs` carried a "12" prefix
   where the open-world tile prefix is "10", so those bosses addressed tiles outside the
   m60 grid and could never read as defeated — Starscourge Radahn among them. Radahn and
   Borealis are corrected (each row contradicted itself via its own `id`/`area_no`; the
   game's openmap alloclist allocates m60_52_38 and m60_54_56; and for Borealis the
   CE-era dump independently lists 1054560800). Night's Cavalry 1248550800 is
   deliberately NOT corrected: its `id` agrees with the "12" form and the CE dump lists
   it at an in-memory address far from the tile region. Two sources each way, so it
   reads Unknown.

   *Known gaps, recorded not hand-waved.* 29 of 205 bosses read Unknown: 26 DLC tiles
   outside the m60 grid (the same gap as the 532 unknown WORLD_PICKUPS, now with a
   legible cause — they are named DLC bosses), 2 doubly-allocated maps, 1 disputed id.
   36 of 2,108 dungeon pickups likewise: 32 in the two doubly-allocated maps, 2 under a
   bogus prefix 9901, 2 whose local id is below 7000 and so are not pickups at all.

   *The DLC gap is BLOCKED ON EVIDENCE and must not be picked up.* The DLC is not
   installed here and no character has progressed into it (confirmed 2026-07-20), so
   there is no attributed transition to work from and no way to verify a hypothesised
   base — the alloclists alone would give an unverifiable claim. It is the largest
   Unknown count in the app and therefore the most tempting target; that is a trap, and
   the correct state for those flags is Unknown. Unblocking requires the DLC installed
   plus a character captured either side of a DLC pickup or boss kill.

   **PICKUP READERS CUT OVER 2026-07-20 — the app no longer reads the frozen store.**
   `is_flag_set_with_status` was described as blocked because a bare id cannot identify
   its family. That was the wrong framing: the missing information was never in the id,
   it was in the CALLER. An entry in `WORLD_PICKUPS` or `DUNGEON_PICKUPS` is known to be
   a pickup, and that alone resolves the ambiguity that blocked the cutover (a 10-digit
   id being either an open-world event flag or a pickup row_id). Given "this is a
   pickup", family follows from the id's shape, which is what
   `pickup_flags::pickup_flag_state` does.

   *Six read paths cut over*, four of them previously unnoticed: the events-view world
   pickup table and its detail panel, the dungeon-pickup detail panel (which carried its
   OWN copy of the `DUNGEON_PICKUP_BASES` arithmetic, so it could disagree with the very
   row it was opened from), `collect_set_flags` over UNIQUE_ITEMS, `comparison_view`, and
   `world_pickups_view` (already cut over but single-family). The comparison view now
   SKIPS pickups it cannot resolve in both saves rather than reporting them as
   differences — comparing two Unknowns manufactures diffs that say nothing.

   *Effect, measured:* `WORLD_PICKUPS` Unknown fell 3,577 -> 1,517 across every slot,
   exactly the 2,060 predicted from the family census. Confessor collected rose 495 ->
   910; Wretch 2 -> 5; V1/V2 1 -> 3.

   *One anomaly, recorded rather than smoothed over.* V3 — the true-negative control —
   went from 0 collected to 2. One is `60210` "Tarnished's Wizened Finger", a starting
   item every character is given, so reading SET is correct and the old 0 was an artifact
   of it being Unknown. The other is `10007452` "Crimson Hood", a Stormveil pickup that
   V3 never reached, and it reads SET on ALL SIX slots. It is not a mislabel — the
   primary source has it (`ItemLotParam_map` row 10000451, `lotItemId01=740000` =
   Crimson Hood) — and it is not a read artifact: V3 reads exactly 1 SET out of ~1,960
   readable legacy pickups, whereas a misplaced base or wrong stride would smear hits
   across many blocks, and the m10_00 block shows a clean differential (75/250 non-zero
   for the Confessor who cleared Stormveil, 1/250 for every minimal character). So the
   bit is genuinely set for everyone; WHY is unestablished. Settling it needs an
   attributed transition on that flag, which the corpus does not have. **"V3 has zero
   pickups" is no longer the right expectation for the control** — it has zero *chosen*
   pickups plus whatever the game sets for all characters.

   **`dungeon_pickups.rs` diverges from the primary source — its own task, now unblocked.**
   Audited against `ItemLotParam_map` (regulation 1.16.1): 189 DB entries absent from the
   primary source, 152 primary entries absent from the DB, about 8% each way, with the
   missing ones clustered in m41_00/01/02, m40_02 and m13_00. The DB-only entries are
   third-party in origin with unverified provenance. Regenerating the table from the
   primary source is a data task with its own verification and must NOT be folded into a
   flag-layout change — but the primary source is on this machine, so nothing blocks it.

   **DONE 2026-07-21 — regenerated from the primary source, with a committed generator.**
   `dungeon_pickups.rs` is now GENERATED, not hand-maintained: `er-save-reader knowledge
   gen-dungeon-pickups` (`src/knowledge/gen_dungeon_pickups.rs`) parses `ItemLotParam_map`
   (sha256-verified), selects every row whose `getItemFlagId` is an 8-digit dungeon flag
   with localId >= 7000 AND that grants an item (`lotItemId01 != 0`), and emits the table
   deterministically. 2108 -> **2031 entries**. A unit test asserts the committed file equals
   the generator's output (mutation-verified: a hand-edit fails it), so the drift that made
   this a task cannot recur.

   The reconciliation was subtler than the audit's headline. Keyed by the natural key
   (`item_lot_id`): **77 removed, 0 item-granting added, 114 event_flags corrected.** The
   audit's "189/152" counted `getItemFlagId`s, which conflated three things:
   - **77 removed** are items whose REAL `getItemFlagId` is not a dungeon-pickup flag at
     all — the old table fabricated `item_lot_id + 7000`. Verified every one: 61 are block
     flags (whetblades/cookbooks/maps — Iron Whetblade is flag 65610, not 10007420), 7 are
     dungeon-EVENT flags localId<7000 (the m15 Golden Seeds — the "seven getItemFlagIds at
     localId 1200-1290" noted below), 7 are Great Runes (simple flags 114/191-196), 2 are
     area-99 junk. They read the wrong bit here and are tracked via their real families
     elsewhere. This includes the two suspect entries `12022995/12022997`.
   - **114 event_flags corrected**: several lot rows legitimately share ONE flag — armor
     sets (the Raging Wolf set's four pieces 11001985-988 all record on 11007985, not
     per-piece 1100898N). The old per-piece flags read unset/wrong bits.
   - **112 empty lots** (a flag but no item) are not pickups and are excluded.

   *Verified.* Structural fields (flag/item/qty) identical to a hand-checked reconciliation
   (0 diffs); categories improved (79 Somber stones the old table mislabelled as
   SmithingStones now read SomberStones; gloveworts/runes it missed now caught). Runtime on
   the 2026-01-11 backup (Confessor slot 0): dungeon **395 collected / 30 UNKNOWN of 2031**
   (up from 382, UNKNOWN down from 36) — the 30 UNKNOWN are exactly the two doubly-allocated
   maps (15x m34_12 + 15x m40_00), `None` by design. All tests green (53 + 4).

   **DONE 2026-07-22 — `world_pickups.rs` regenerated the same way, and it exposed a
   silent misread in `pickup_data.rs`.**

   *The generator.* `er-save-reader knowledge gen-world-pickups`
   (`src/knowledge/gen_world_pickups.rs`) selects every item-granting flagged row MINUS
   the dungeon pickups `gen_dungeon_pickups` owns, so the two tables now **partition** the
   primary source exactly (2,867 + 2,031 = 4,898, no overlap; a unit test asserts the
   disjointness on a fixture). The old table carried all 4,898 — it duplicated every
   dungeon pickup into the world browser, which has its own view.

   *What was wrong in the 2,867 survivors.* `flag_id`, `item_id`, `quantity` and `region`
   were already correct (0 diffs). Everything else was not: **every single item_name was
   wrong** — 2,603 read `"Unknown Item <id>"` and 264 named a real but different item (lot
   20450 said "Immunizing Cured Meat"; it is the Gold Scarab). `item_type` was wrong for
   all 2,867 (the old table typed 3,797 of 4,898 rows Armor and recognised 6 weapons in
   the whole game). `tile_x`/`tile_y` were off by one digit on all 1,663 tile rows — the
   old code sliced the flag `[2:3],[3:5]` instead of `[2:4],[4:6]`, so the V1/V2 control
   pickup read tile (4, 43) instead of (44, 36) = m60_44_36.

   *New primary-source finding:* `lotItemCategory` **6 = custom weapon**
   (`EquipParamCustomWeapon` — a base weapon + ash of war + upgrade level; row 5000 =
   "Banished Knight's Halberd +8 - Spinning Strikes", `baseWepId` 18030000). Typed Weapon.
   Category 0 (4 rows) is unset in the source and stays `Unknown` rather than guessed.

   *The misread it exposed.* `pickup_data.rs` stored `event_flag = item_lot_id` for the
   open-world family. Measured at the RESOLVED READ ADDRESS (not the raw value, which
   overcounts), **220 entries read the wrong thing**: 97 the wrong address within the tile
   family, 77 a nonexistent tile bit for items actually recorded on block flags (every map
   fragment / cookbook / Crystal Tear dropped by an open-world lot), 46 Unknown where the
   real flag is a readable block flag. **0 regressions**; 4,580 entries resolve
   identically. Fixed by setting `event_flag = getItemFlagId` for all 1,754 entries that
   differed, making the convention uniform.

   *Anti-drift where a generator isn't possible.* `pickup_data.rs` cannot be regenerated —
   its `region` taxonomy and 1,326 `mapgenie_id`s are enrichment absent from the primary
   source. So `test_event_flags_match_primary_source` re-derives the one structural field
   from `ItemLotParam_map` on every run instead. Both new tests mutation-verified. The
   file header claimed "Auto-generated - do not edit manually", false on both counts and
   part of why the row-id convention went unexamined; rewritten.

   *Verified — multi-slot differential on the 2026-01-11 backup.* UNKNOWN fell
   1,517 → 1,471 **on every slot**, exactly the 46 predicted. Confessor 910 → 976
   collected; **V3 unchanged at 2 and V1/V2 at 3**, so the true-negative controls gained
   no false positives. Block-flag probe, Confessor vs V3: Confessor now reads exactly the
   maps for the regions `DATA-SOURCES.md` says it explored (Limgrave W/E, Weeping
   Peninsula, Liurnia E/N/W, Altus, Leyndell, Mt. Gelmir, Caelid, Dragonbarrow, Ainsel,
   Siofra) plus 6 Memory Stones / 2 Talisman Pouches / 9 Crystal Tears, and **none** of
   the Mountaintops / Consecrated Snowfield / Farum Azula maps — consistent with Radahn
   not defeated. V3 reads exactly one: Tarnished's Wizened Finger, the universal starting
   item. On `world_pickups.rs` itself, V1/V2 read flag 1044367310 SET as "Golden Rune [1]"
   @ tile (44, 36) and V3 does not, matching the claims store's reward corroboration on
   that anchor pair.

   *Loose end closed.* The 9 `pickup_data` lots absent from the primary source are the
   **Troll Carriage** rows, whose param row id is 9-prefixed (`934490010`) while
   `pickup_data` keys them by the storage address (`1034490010`). Their flags were already
   correct; the test covers them by asserting the flag is a real `getItemFlagId`.

   *Remaining (not done).* The 488 DLC open-world pickups still read Unknown —
   `is_tile_pickup_set` is scoped to the `1_000_000_000..2_000_000_000` grid and the DLC
   grid's family base has never been established. And `pickup_data.rs` could become fully
   generated if its enrichment were lifted into a committed overlay keyed by
   `item_lot_id`; see `docs/DATABASE_COVERAGE_ANALYSIS.md` → Code Redundancy Notes.

   **SETTLED 2026-07-20 — the overlap is real, and harmless.** Jump to the resolution
   below; the statement of the question is kept because the reasoning that closed it is
   the reusable part.

   **THE QUESTION — the two legacy families' address ranges OVERLAP (found 2026-07-20).**
   Both use `alloc_slot * 1125 + localId / 8` with no subtraction for the pickup family,
   so within one map's 1125-byte block the event flags occupy bytes 0-874 (localId
   < 7000) and the pickups 875-1124 (localId >= 7000). If the two bases were equal that
   would tile one block exactly. They are not equal: the pickup base sits 125 bytes
   LOWER, which puts the pickup range at bytes 750-999 of the event block. So for any map,

       event localId L  and  pickup localId L + 1000  resolve to the same bit.

   Concretely, verified pickup 30027000 shares its byte with a hypothetical event flag
   30026000 — both inside their own family's declared range. Each family was verified
   against its own flips, so each (base, formula) pair is individually correct on the
   evidence; the split of that pair into "base" and "formula" is what is not pinned. Most
   likely one family's formula needs a term the other lacks (e.g. pickups indexed from
   `localId - 1000` rather than `localId`, which would make the bases equal and the block
   tile cleanly).

   **THE RESOLUTION (2026-07-20).** Three findings, in the order they landed.

   *1. My proposed fix was not a fix.* "Pickups index from `localId - 1000` at a shared
   base" expands to `(base_ev - 125) + slot*1125 + L/8` — the shipped formula, character
   for character. The two were never competing hypotheses and no experiment could have
   separated them. Check the algebra before designing the experiment.

   *2. The single-base model is REFUTED, by bytes.* File b33 carries two event flags and
   three pickups all known set — the same file, so no drift confound. Every one reads set
   at its own family's base and clear at the other's. The clincher is the transition: the
   byte that flips for pickup 30027000 across b20→b21 is at the pickup base, while the
   single-base prediction (`ef[1623098]`) stays `0x00` on both sides. The 125-byte
   separation is a real property of the save layout.

   *3. The overlap band is empty, so nothing is at risk.* Legacy event flags cluster in
   localId 0-2999 and pickups in 7000-7999; 6000-6999 is used by neither. Zero hits
   across 4,540 distinct legacy flags from three independent sources (the CE-era memory
   dump, `dungeon_pickups.rs`, `bosses_data.rs`), and the primary source agrees —
   `ItemLotParam_map` (regulation 1.16.1) carries 2,143 legacy `getItemFlagId`s in
   7000-7999 and none in 6000-6999. The earlier caution to "suspect legacy event flags
   with localId 6000-6999" is withdrawn: there are none. If one is ever found it collides
   with a real pickup and the layout needs revisiting — that is the trigger to watch for,
   not a standing doubt.

   *New, small, from the same primary source — two DB discrepancies, neither affecting a
   shipped read.* `ItemLotParam_map` gives m15_00 seven `getItemFlagId`s at localId
   1200-1290, the only legacy pickups outside 7000-7999, and `dungeon_pickups.rs` does
   not carry them at all — a coverage gap, not a misread (added, they would be rejected
   on the 7000 rule and read Unknown). Conversely `dungeon_pickups.rs` carries two
   entries the primary source does not list as legacy pickups, `12022995` and `12022997`
   (m12_02, localId 2995/2997), which read Unknown today; unverified provenance, treat as
   suspect. Worth an audit of `dungeon_pickups.rs` against `ItemLotParam_map` as a whole —
   these two turned up incidentally, so there are probably more.

   *Also inconclusive, recorded so it is not re-run blind:* whether m34_12 belongs to
   alloc slot 62 or 144. In backup slot 0 the slot-62 block holds 6 non-zero bytes and
   slot 144 none, which is suggestive but not decisive — one map, one save, no attributed
   transition, and the bytes sit in the overlapping range above, so they cannot even be
   assigned to a family with confidence. m40_00 is undecidable outright: both its blocks
   are zero in every slot, i.e. no character has been there. Both maps stay Unknown.
   *Agreed order for the remaining migration work (2026-07-20):* **Priority 1b first**
   (the exported wasm readers — the only remaining correctness risk, and it needs a
   decision on breaking elden-map loudly), then step 6 (docs audit), then step 5
   (distill and delete). Step 6 before step 5 because era-mixed docs actively misled
   this session more than once — the Margit/Godrick catalog note, the tombstoned
   "tile base 337375 is constant" guidance, and the retracted elden-map advice above
   were each believed before being checked.

5. ~~**Distill and delete** the Python lab scripts (~50k lines) and shrink
   `src/discovery` to what the pipeline uses; move `src/db/event_flags.rs` (in-memory
   convention) out of the app into KB inputs as the CE-era Rosetta table.~~
   **DONE 2026-07-21.**

   *Python lab deleted* — 209 files (161 `.py` + the lab's own JSON/case-store artifacts,
   none in the evidence catalog, all consumed only by the deleted `src/discovery` modules).
   Distilled first into `docs/archive/PYTHON-LAB.md` (grouped record of what each script
   family did, what survived, what replaced it) — "a recorded dead end is worth more than a
   deleted one." **Kept:** `scripts/windows/regenerate-game-extracts.ps1` (operational, cited
   by the `game-extracts` catalog corpus) and `notebooks/` (separately archived).

   *`src/discovery` shrunk* — the pipeline (`src/knowledge`) used NONE of it. The app used
   only the `inventory_verification` leaf (UNIQUE_ITEMS tables), relocated to
   `src/db/inventory_verification.rs`; the other 21 lab modules (~14.5k LOC incl. `cli.rs`)
   and the `discovery` CLI subcommand (`main.rs`) were deleted, along with the orphaned root
   lab JSONs (`discoveries.json`, `param_flags.json`, `unified_flags.json`). Main-crate unit
   tests dropped 116→51 — exactly the deleted lab's tests.

   *`event_flags.rs` moved out* — the 46,076-line CE-era Rosetta table (in-memory
   byte_offset/bit_position + coords/name/category for 5,751 flags, unused by the app)
   extracted verbatim to `knowledge/reference/ce-era-event-flags-rosetta.json` (+ epistemic
   header, `knowledge/reference/README.md`) and the module deleted. NOT added to the evidence
   catalog — that indexes out-of-repo raw evidence, and this is derived in-repo reference
   (recorded in the README). Distinct from the live `src/db/event_flags_db.rs`, which stays.

   *Green:* main 51, regression 4 (+3 ignored), wasm 22 + anchor 4 + export-shape 4 + origin
   11 — including the ADR-0008 `export_shape_conformance` guard. Stale "auto-generated by
   scripts/…" headers on the surviving `src/db/*_data.rs` tables repointed to the distillation
   doc (the generators are gone). **Step 4 (elden-map) DONE 2026-07-21 — see Priority 1b.**
6. ~~**Docs audit** — epistemic header on all 14 docs (evidence / claim summary /
   methodology / obsolete), correct or retire wrong content (EVENT-FLAG-GEOGRAPHY area
   labels, stale CLAUDE.md paths); CLAUDE.md shrinks to workflow rules + pointers to
   `CONTEXT.md` and the catalog.~~ **DONE 2026-07-20 (v0.31.1).** All 14 docs headered;
   *Epistemic Header* defined in `CONTEXT.md`; EVENT-FLAG-GEOGRAPHY area labels + tombstoned
   literals + stale `.emevd.js` path corrected; DATA-SOURCES Radahn fact fixed;
   COMMIT-PROTOCOL H1 fixed; CLAUDE.md 144→113 (guardrails kept inline, duplication →
   pointers). **Step 5 done 2026-07-21; step 4 (elden-map) done 2026-07-21 — the migration
   plan (steps 1-6) is complete.**

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
  **UPDATE (2026-07-05, `knowledge run`):** largely explained. The dungeon-grace family
  lives in a second world-state block ~146.6k above the grace base (claims store,
  family `world-state-b`); tile offsets were struct-anchor-relative (~146.1k above
  grace, tombstone `tile-base-337375-grace-anchored`); and the old "catacombs region"
  around grace_rel 28-31k is a u32-record LIST, not the flag bitmap — its entries react
  to kills (hence the old c=0 validations correlating) but the real boss-kill bits are
  in the legacy-dungeon family at ~1,529.98k grace_rel. Old catacombs/tunnels GT
  offsets should be treated as record-list observations, not flag positions, during the
  step-4 cutover.

Follow-ups:
1. ~~Fix detection; add a hard validation gate~~ DONE 2026-07-05 (windowed scan + gate;
   structural walk demoted to diagnostics).
2. ~~Python re-point~~ DONE 2026-07-05: `discovery ef-dump` subcommand added (JSON,
   `--raw-slot` mode); `scripts/verification/ef_dump.py` is the single sanctioned
   bridge; `SaveParser._find_event_flags_offset`, `utils.detect_event_flags_start`
   and `_robust` now DELEGATE to it (python content search deleted). Verified: python
   now returns the fixture-golden offsets (81,077 etc.; previously 106,808).
3. elden-map coordinated change, PARTIALLY DONE 2026-07-05: rebuilt
   `wasm-event-flags` deployed to `elden-map/wasm-event-flags/` (vendored build was
   from the poisoned Apr-07 era) and verified under node against a fixture (81,077,
   confident). REMAINING: rebuild/restart the elden-map server+bundle to pick it up;
   delete `slot-layout.ts` / `ground-truth-formulas.ts` and re-point consumers;
   capture-agent rework per ADR-0007 (stop writing interpretations; add full-slot
   keyframes every N entries + on GaItems resize, per-entry state checksums,
   agent+wasm version stamps). `scripts/capture_agent.py` catalog context fields get
   the same demotion (were computed with the removed python detector).
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

## Priority 1b: The WASM exports still reach a disproven table (found 2026-07-20)

**This is the last place a caller can still get a silently wrong bit, and the callers
are outside this repo where we cannot see them.** After the v0.30.0 cutover no app-side
reader touches the legacy store — but the wasm crate's exported entry points do:

```
#[wasm_bindgen] is_flag_set / get_flag_offset / get_flag_offset_calibrated
  └─ get_flag_offset_with_tile_base        (lib.rs:816-837)
       └─ calculate_dungeon_flag_offset_unified
            └─ get_dungeon_general_bases()  ← the disproven "+3375 per area" table
```

`get_dungeon_general_bases()` is the table whose own audit comment records entries
disproven by every save on this machine (m18=43,487 and m19=46,862 removed; most of the
rest UNVERIFIED and reading all-zero in every available save). These exports return a
plausible-looking wrong offset rather than refusing — the exact failure mode this
migration exists to remove — and elden-map has already inherited a poisoned build once
(step 3 note above, the Apr-07 vendored wasm).

**DECIDED 2026-07-20 — refuse loudly (ADR-0008).** The exports will fail rather than
return a static offset, accepting that this breaks elden-map. Reasoning in the ADR; the
short form is that the export SHAPE encodes an abandoned model — `get_flag_offset(flag_id)`
promises a static byte offset, every family floats per save, so there is no correct value
to return. "Re-point it at the resolver" is not available: the resolver needs the flag
region to locate a family for that save, so any honest replacement takes different
arguments. Consumers break either way; the only choice is visibly or silently.

*The work, in order:*
1. ~~Delete the disproven path — `get_dungeon_general_bases()` and
   `calculate_dungeon_flag_offset_unified` — and every route into it.~~ **DONE 2026-07-20.**
2. ~~Make `is_flag_set` / `get_flag_offset` / `get_flag_offset_calibrated` refuse.~~
   **DONE 2026-07-20 — removed outright**, per the ADR's own note that an export which
   always fails is still an export people call.
3. ~~Conformance test pinning that no exported entry point reaches a legacy base table.~~
   **DONE 2026-07-20** — `crates/wasm-event-flags/tests/export_shape_conformance.rs`.
4. ~~Coordinate elden-map onto the region-taking readers.~~ **DONE 2026-07-21** — see
   the step 4 write-up below. (This line read "STILL OPEN" until 2026-07-22; the
   summary was never updated when the work landed.)

**SCOPE WIDENED DURING THE WORK (2026-07-20).** Steps 1-2 named three exports; seven were
removed, plus five base tables. Removing only the three named would have left step 3
unwritable: the conformance test asserts an empty set, and these still reached crate-baked
bases by exactly the same shape.

| Also removed | Reached | Why it could not stay |
|---|---|---|
| `calculate_dungeon_pickup_offset{,_impl}` | `get_dungeon_pickup_section_bases()` (88 per-section bases) | same defect, different table |
| `get_dungeon_pickup_sections` | same table | served its keys as JSON for callers to trust |
| `calculate_tile_pickup_offset` | static `TILE_BASE_OFFSET` = 337375 | 337375 is tombstoned — it is the distance *between* two families, not a base |
| `calculate_world_pickup_offset_by_row_id{,_impl}` | `WORLD_PICKUP_ROW_ID_BASE` | its own doc comment recorded the row_id model as superseded 2026-02-16 |
| `get_tile_base_offset`, `get_world_pickup_row_id_base` | those two constants | handed the tombstoned bases out directly |

Base tables deleted: `get_dungeon_general_bases`, `get_sub_block_bases`,
`get_main_block_bases`, `get_midrange_bases`, `get_dungeon_pickup_section_bases`. The
crate no longer imports `HashMap` — that import going unused is the check that none
survived. Constants `TILE_BASE_OFFSET` and `WORLD_PICKUP_ROW_ID_BASE` are gone.

*Kept, deliberately:* `calculate_tile_pickup_offset_calibrated(flag_id, tile_base)` — the
base is a parameter, so it invents nothing. This is tile *geometry*, not family location,
and it is load-bearing for the correct path: `tile_read` calls it with base 0 and adds a
resolved family base. Deleting it would push callers to reimplement the slot arithmetic,
adding an error source rather than removing one.

*What the conformance test guards*, verified by mutation (each mutation was applied,
observed to fail the intended test, then reverted):
- a banned symbol reappearing as a definition;
- a tombstoned literal (337375, 1037373320, 43487, 46862) returning to live code;
- **a new static-offset export under an unbanned name** — caught by two independent
  tests (the manifest-equality check and the structural check that any export answering
  a flag position/state question must receive `&[u8]` or an explicit base). This is the
  case a banned-names list alone cannot catch, and the one most likely to actually happen.

*Fallout:* none in this repo. No app-side Rust caller referenced any removed export
(`cargo build` and the full 127-test app suite pass untouched); the removals were felt
only by tests that asserted literal byte offsets, which were deleted rather than
re-pointed — their assertions were the abandoned model restated. Evidence those tests
carried (tile captures 119-127, the getItemFlagId routing correction) is preserved in
comments at the removal sites and still covered by `tests/origin_conformance.rs`.

**Step 4 (cross-repo): coordinate elden-map onto the region-taking readers.**
**DONE 2026-07-21.** elden-map now reads every flag through the five three-state
readers (`is_world_state_flag_set`, `is_tile_pickup_set`, `is_tile_world_flag_set`,
`is_dungeon_flag_set`, `is_dungeon_pickup_set`), routed per family, with a genuine
`unresolved` state distinct from `clear`. The surface it moved to:

| elden-map called | now calls | note |
|---|---|---|
| `is_flag_set(ef, id)` | one of the five readers via `shared/flag-reader.ts` | a bare id does NOT pick the family; the caller chooses (bosses → `world`, else `pickup`) |
| `get_flag_offset(id)` | `flag_offset_in_ef(ef, id, family)` (NEW export) | per-save byte position, resolved from the flag region; invalid when unresolvable |
| `get_tile_base_offset()` | — | 337375 was never a base |

*The one crate change this needed.* elden-map's live Character Explorer overlays flag
names on save bytes, so it needs a per-save byte *position*, which no export provided
after the ADR-0008 removals. Added **`flag_offset_in_ef(event_flags, flag_id, family)`**
(`crates/wasm-event-flags/src/lib.rs`): takes the flag region, resolves the chosen
family's base for THAT save, returns `valid=false` when it cannot — the honest,
ADR-0008-compliant replacement for the removed static `get_flag_offset`. Added to the
`export_shape_conformance` manifest and locked to the tri-state readers by two parity
tests in `origin_conformance.rs` (same byte/bit, and matching refusals).

*What changed in elden-map* (branch `event-flags-adr0008-cutover`, separate repo/commit):
rebuilt + vendored the wasm; new `shared/flag-reader.ts` routing layer (`readFlagState`
/ `resolveFlagOffset`, three-state); deleted every TS static-offset fallback (incl.
tombstoned `337375`), the four duplicate `calculateFlagLocation` copies, and
`CalibrationService` from the read path; re-pointed all consumers (eventFlagService,
event-flag-detection, data/event-flags, eventFlagDiffService byte-diff → per-save state
comparison, and the Character Explorer analysis + hex view); deprecated the
verification/calibration testing subsystem in place; stopped the capture agent baking a
fabricated `calibrated_tile_base` into evidence (ADR-0007); hardened `parseSaveFile` to
await wasm (no static fallback exists now, so a pre-init read would show everything
undiscovered).

*Verified out-of-sample.* The real 2026-01-11 backup, Confessor slot 0, through
elden-map's own `parseSaveFile`: **179 graces** (exact match to the ER-save-Reader
validation), Margit ✓, Godrick ✓, Radahn ✗ — the corrected fact. Client + server
typecheck clean; `vite build` succeeds.

The three-state result was the substance of the change, not a detail: `false` and
"could not resolve" now reach the map as different things.

### Elden Map Missing Block Bases
- **Issue**: Elden Map viewer (`eventFlagService.ts`) is missing block bases that Save Editor has
- **Missing blocks**: 62000 (map fragments), 65000 (Crystal Tears), 72000 (DLC graces), 74000 (DLC dungeon graces), 78000 (grace guidance)
- **Action**: Sync BLOCK_BASES from ground_truth_offsets.json to Elden Map
- ~~**Progress** (v0.15.0): elden-map can use WASM `get_flag_offset()` instead of
  maintaining separate lookup tables~~ **RETRACTED 2026-07-20.** That advice points at
  the disproven path above. `get_flag_offset()` returns a static offset, and the project
  has since established that every family's position floats per save. Do not follow it.
- **Progress** (v0.16.1): Block bases corrected — old bases were false positives calibrated against GaItemData section. 61000 removed (disproven). New blocks added: 66000, 69000, 91000, 92000

---

## Priority 4: Code Quality

### Module Consolidation (Optional)
Several data categories have parallel modules (see [DATABASE_COVERAGE_ANALYSIS.md](DATABASE_COVERAGE_ANALYSIS.md#code-redundancy-notes)):
- ~~`world_pickups.rs` / `pickup_data.rs` (overlapping pickup data)~~ **RESOLVED
  2026-07-22** — the three pickup tables now have distinct stated jobs (two generated and
  disjoint, one enriched); the residual consolidation step is spelled out in
  `docs/DATABASE_COVERAGE_ANALYSIS.md` → Code Redundancy Notes
- `graces.rs` / `graces_data.rs` (enum + enriched split)
- `bosses.rs` / `bosses_data.rs` (enum + enriched split)
- `shop_items.rs` / `merchants_data.rs` (different grouping)

These work correctly as-is but could be consolidated to reduce maintenance burden.

### `cargo clippy` does not pass (noticed 2026-07-22)
- **Concept**: `cargo clippy --workspace` fails with **6 errors**, all
  `enum_clike_unportable_variant` ("C-like enum variant discriminant is not portable to
  32-bit targets") in `src/vm/inventory/mod.rs:136` (`InventoryItemType::AOW = 0x80000000`)
  and `:167-171` (`InventoryGaitemType::{WEAPON..AOW} = 0x80000000..0xc0000000`). Both
  enums also carry a `-1` default variant, so they infer `isize` and the high tags do not
  fit a 32-bit one.
- **Status**: NOT STARTED. **Pre-existing and unrelated to any recent commit** — verified
  by stashing and re-running against `HEAD` (v0.36.1), which produces the identical 6.
  Recorded here so the next `/snapshot` does not re-discover it and mistake it for new
  breakage.
- **Not cosmetic, and not urgent**: these discriminants are the game's own GaItem handle
  tag bits, so the values are load-bearing and must not be "fixed" by changing them. The
  correct fix is a representation change (`#[repr(u32)]` with a non-negative sentinel, or
  widening to `i64`), which touches every `From`/`match` on both enums — a real refactor,
  not a lint suppression. The app targets 64-bit desktop only, so nothing is actually
  broken today.

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

### Dependency vulnerability audit (surfaced 2026-07-22) — DONE 2026-07-22
- **Concept**: GitHub reported **12 Dependabot advisories on the default branch — 7 high,
  1 moderate, 4 low** when v0.36.1 was pushed
  (`github.com/laszloprekop/ER-Save-Editor/security/dependabot`). Unrelated to any commit
  in that push; no dependency changed. They have been accumulating unreviewed.
- **Status**: RESOLVED — all 12 cleared, and **not one of them by a version bump of an
  affected crate**. Eleven were unreachable code that should never have been compiled in;
  one was a genuine transitive patch.
- **The actual question is reachability, not the count** — and reachability turned out to
  be *zero* for almost all of it. The audit found two root causes, both "why is this
  linked at all", not "which version":
  1. **`reqwest` was a dead direct dependency.** It appeared **exactly once in the entire
     repository** — the `Cargo.toml` line declaring it — with no `use reqwest`, no call
     site, in `src/`, `crates/`, or `build.rs`. `git log -S"use reqwest" --all` returns
     **nothing across the whole history**: it was declared and never called, inherited
     from the upstream ClayAmore fork. It alone dragged in **10 of the 12 advisories**
     (`rustls-webpki` ×4, `aws-lc-sys` ×5, `quinn-proto` ×1) via
     `reqwest → rustls/hyper-rustls/tokio-rustls/rustls-platform-verifier`. A save reader
     was linking a TLS stack and a QUIC implementation to do nothing with them.
  2. **`image`'s default features pulled in the AVIF *encoder*.** `image = "0.25"` with
     defaults enables `default-formats → avif → ravif → rav1e → rand 0.9.2` (alert 18).
     The reader has exactly two `image` call sites and both are **PNG-only**: an embedded
     `icon.png` (`src/main.rs:57`) and `MENU_ItemIcon_{:05}.png` with the extension
     hardcoded in the format string (`src/ui/icons/mod.rs:53`). Nothing is ever *encoded*.
     Pinned to `default-features = false, features = ["png"]`.
- **The one real patch**: `rand` had **two** alerts for the same GHSA (cq8v-f236-94qc),
  because two different `rand` versions were in the tree. Its ranges are `>= 0.9.0, < 0.9.3`
  **and** `>= 0.7.0, < 0.8.6` — so killing AVIF removed the 0.9.2 instance (alert 18) but
  **not** `rand 0.8.5`, which arrives via `eframe → egui-winit → accesskit_winit →
  accesskit_unix → zbus` (alert 19, patched version 0.8.6). Bumped `0.8.5 → 0.8.6`. Worth
  remembering: a duplicated GHSA in the alert list means two copies in the lock, not a
  GitHub glitch — deduplicating the alert would have hidden a live one.
- **Result**: **1,289 lines / ~120 packages removed from `Cargo.lock`**, including the whole
  `rustls`/`tokio`/`hyper`/`quinn`/`aws-lc` stack and the `rav1e`/`exr`/`gif`/`tiff`/`webp`
  codec set. `cargo check --workspace` and `cargo check --features save-writeback` clean;
  **104 tests pass, 0 fail — identical to the pre-change baseline** (measured by stashing
  the manifest and re-running, so the count is a comparison, not an assertion).
- **The egui/eframe churn this entry feared did not happen**: no direct dependency was
  upgraded and `eframe`/`egui` are untouched. The fix was *subtractive*.
- **Do not conflate with**: the third-party-resource caution in `CLAUDE.md`, which is about
  *game-data* provenance, not Rust crates.
- **If a network feature is ever genuinely wanted** (update check, etc.), re-adding
  `reqwest` re-adds all 10 advisories' crates. That is a real cost to weigh at that point,
  not a reason to keep a dead dependency now.

### Repo name still says "Editor" (noted 2026-07-22) — DONE 2026-07-22
- **Concept**: The project renamed itself `er-save-editor` -> `er-save-reader` in
  v0.35.0 (ADR-0009), but the git remote was still
  `git@github.com:laszloprekop/ER-Save-Editor.git`.
- **Status**: DONE — GitHub repo renamed to **`laszloprekop/ER-save-Reader`** (the casing
  the project already uses for itself in `docs/CHANGELOG.md:3` and the working directory,
  rather than a newly invented one). Local `origin` re-pointed; `git fetch` verified
  against the new URL. The repo `description` was stale in the same way — "Elden Ring Save
  **Editor**. Compatible with PC and Playstation saves." — and was updated at the same
  time; it is the first thing a visitor reads and it contradicted ADR-0009.
- **CORRECTION to this entry's own warning.** It said all four in-repo files mentioning
  `ER-Save-Editor` refer to the upstream ClayAmore project and that "the only thing to
  change is the remote URL". That was wrong about one of them: **`README.md:28-29` is a
  *self*-reference** — the clone instructions for this repo
  (`git clone .../your-username/ER-Save-Editor.git` / `cd ER-Save-Editor`) — and was
  updated. The rest of the mentions do correctly name upstream and were left alone:
  `README.md:4,16,101,106` (ClayAmore links/assets), `docs/SAVE_FILE_GROUND_TRUTH.md:533`
  (a derived filename), `docs/adr/0002`, and `docs/CHANGELOG.md:298` (a historical entry —
  the changelog is never retroactively corrected, per its epistemic header).
- **Lesson worth keeping**: "all N occurrences are upstream" was asserted from a grep
  *count*, not from reading each hit. One in six was the opposite of what the note claimed.
- **Not renamed, deliberately**: old `github.com/laszloprekop/ER-Save-Editor` links keep
  working via GitHub's redirect, and `docs/BACKLOG.md:1318`'s dependabot URL is left as the
  historical record of where those alerts were reported.

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
