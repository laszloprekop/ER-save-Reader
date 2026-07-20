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
2. ~~Evidence catalog~~ DONE 2026-07-05: `knowledge/evidence-catalog.json` (7 corpora,
   hand context + machine sha256) + per-file manifests under `knowledge/manifests/`
   (3,997 files, ~12GB) + `er-save-editor knowledge catalog-update|catalog-verify`
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
3. **Pipeline** — CORE DONE 2026-07-05: `er-save-editor knowledge run`
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
   TIMELINE RE-ANNOTATION ATTEMPTED 2026-07-06 (`er-save-editor knowledge timeline`,
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
   REMAINING in step 3: c06-c08 Golden Centipede pairs still lack flag annotations
   (cannot become pairs without a flag hypothesis) — data gap, not a mechanism gap;
   the four s7 world-state-b pairs above also remain unresolved pending more/cleaner
   captures; the timeline flip-clustering design above, if someone wants to pursue it.
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

`pickup_data.rs` stores `event_flag` = `item_lot_id` = the ROW ID, not the
getItemFlagId. `is_tile_pickup_set` accepts either form and normalises.

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

   Nearest existing mechanism is `src/calibration.rs`, which is the right shape
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
   `er-save-editor knowledge family-distances` (`src/knowledge/family_distances.rs`,
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
   `er-save-editor knowledge origin-probe` (same module, emits
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
   `er-save-editor knowledge validate-origin` (emits
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
   verified and the origin resolves.

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
