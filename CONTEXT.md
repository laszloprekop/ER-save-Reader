# ER Save Reader — Event Flag Research

Save editor and event-flag research platform for Elden Ring. This context covers the
epistemic layer: how facts about the save format are established, stored, and consumed.

## Language

This is the shared vocabulary. Use these words; prefer them over inventing new ones
mid-discussion. If a term is missing, add it here rather than relying on context.

Terms deliberately **not** used, because they were tried and confused people:
*grace surface* (say "the Graces screen" or name the file), *code path* when a plain
name will do, *ground truth* (say Evidence or Claim — it hides whether something was
measured or assumed).

### Epistemics

**Evidence**:
Raw bytes we did not compute: game extracted files, raw save snapshots, and raw timeline
diff records (offset/old/new). Evidence is immutable and never edited in place.
_Avoid_: ground truth, hard truth

**Claim**:
A statement derived from Evidence (a flag's byte offset, a section base, a formula) together
with its provenance: coordinate convention, method, evidence references, and date. A claim
without evidence references is a Hypothesis.
_Avoid_: ground truth, verified offset (without provenance)

**Hypothesis**:
A claim not yet backed by evidence references. May guide investigation; must never be
consumed by application code.
_Avoid_: calculated (as a status implying trustworthiness)

**Provenance**:
The trail attached to every Claim: which Evidence, which method, which coordinate
convention, when. What separates a Claim from folklore.

**Coordinate Convention**:
The definition of byte 0 that an offset is expressed against. Per the 2026-07-05 finding,
conventions are PER FLAG FAMILY: every stored offset names its family, and offsets from
different families or conventions must never be compared or merged.
_Avoid_: anchor (when meaning the convention rather than a detected position)

**Flag Family**:
A group of event flags whose bytes move together within a save. Families float
independently across saves, so each family has its own per-save base. Named families
measured so far (see the Claims Store for layouts and bases): **world-state-b**
(dungeon graces and other world state, in a second block ~146.6k above the grace base
that also mirrors the tutorial anchor flags), **tile-open-world** (m60 tile event
flags, including overworld boss kills), **tile-pickup-row-id** (world pickups tracked
by ItemLotParam row id; same tile layout, separate region), **legacy-dungeon**
(alloclist-slot layout, including catacombs boss kills), **legacy-dungeon-pickup**
(dungeon pickups; same layout, separate region). Event flags and pickup tracking are
separate regions per area type.

**Record List**:
A u32-record structure inside the EF region (~28-31k above the grace base) that is NOT
a flag bitmap: inserting a record shifts everything after it (the ±4 float
illusions). Old "catacombs/tunnels base" offsets pointed here; its entries react to
kills, which made them look like verified flag positions.

**EF Anchor**:
The detected per-save base of one flag family (by default the grace family, which the
detection fixtures pin). A per-slot measurement, not a convention, and not valid for
positioning other families.

**Origin** (a.k.a. **List End**):
The end of the append-only u32 Record List, which is what every Flag Family is
positioned from: `family_base = list_end + FAMILY_CONSTANT`. The list grows by one
record as the character plays, pushing every family along, which is why fixed offsets
drift. Measuring from its end removes the drift entirely. Established 2026-07-20; see
docs/BACKLOG.md 4b.
_Avoid_: base offset (ambiguous — say which family), anchor (that is the detected EF
position, a different thing)

**Family Constant**:
The fixed distance from the Origin to one family's base. Measured, not derived.
Currently: world-state-b 117,192 · tile-open-world 454,067 · tile-pickup-row-id
454,567 · legacy-dungeon-pickup 1,500,442. The families are rigidly locked to each
other, so these differences reproduce the independently measured inter-family
distances exactly.

**Resolver**:
The reference-implementation code that turns a save into family positions
(`crates/wasm-event-flags`: `find_flag_list_end_in_ef`, `resolve_family_base_in_ef`,
and the per-family read functions). It validates its assumptions and returns nothing
rather than a plausible wrong answer. One implementation only (ADR-0005) — the
pipeline delegates to it so pipeline and app cannot disagree about where a family is.

**Cutover**:
Moving one Flag Family off the frozen `ground_truth_offsets.json` and onto the
Resolver, per ADR-0006. Recorded in `metadata.frozen.cutover_state`, which changes the
file's pinned digest — that is the one legitimate reason to re-pin it. Done so far:
graces, world pickups. A cutover must move EVERY read path for that family; leaving one
behind means two screens disagreeing about the same flag.

**Unknown**:
The third state of a flag read, distinct from set and clear: the position could not be
resolved, so nothing is known. Must never collapse to "not found" — that failure is
what made `batch-validate` report 0/110 boss defeats on a finished character. In code:
`FlagState::Unknown` (see below); in the UI: `-`.
_Avoid_: false, not discovered, 0 (when the truth is that we could not tell)

**FlagState**:
The type Unknown lives in: `FlagState { Set, Clear, Unknown }`, in
`crates/wasm-event-flags`. It replaced `Option<bool>`, which was a correct tri-state and
a poor one — `unwrap_or(false)`, `unwrap_or_default()` and `is_some_and()` all turn "we
could not tell" into "no" in a way that compiles and reads naturally. It deliberately has
**no `is_set()`**: that method is how the distinction gets lost, and
`GraceStatus::is_discovered()` was exactly it. The one way back to a bool is
`unknown_as_clear()`, named so that `grep -rn 'unknown_as_clear'` is the complete audit
list of the places a real distinction is being discarded.
_Avoid_: `Option<bool>` for a flag read (it is still right for genuinely optional values,
e.g. a user's manual verification mark)

**ResolvedFlags**:
One save's flag region with every Family Constant already applied — the Resolver's result
as a value rather than a computation repeated per flag. `ResolvedFlags::from_event_flags`
finds the Origin once and refuses (`None`) if it cannot; the per-family methods then
answer `FlagState`, and still answer Unknown for ids with no verified layout. Holding one
is a promise that the Origin was found, **not** that any given flag can be read. It
borrows the flag region so the resolved bases cannot be recombined with a different
save's bytes. Deliberately not `#[wasm_bindgen]`: exporting it would put the primary
reader behind `impl` methods, which the ADR-0008 guard does not scan (pinned by
`no_wasm_bindgen_impl_blocks_exist`).
_Avoid_: calling the deprecated free `is_*_set` functions in new code — each re-scans
~13,400 bytes to re-derive the same Origin.

**Row ID vs getItemFlagId**:
Two names for the same world pickup. The save addresses the pickup at the row_id, the
param tables name it by `getItemFlagId`, and `getItemFlagId = row_id + 7000` converts
between them — `is_tile_pickup_set` accepts either form and normalises.

**That identity is not universal, and treating it as one is a bug** (CORRECTED
2026-07-22). For 124 of the 1,691 ten-digit `ItemLotParam_map` rows the param's own row
id is NOT `getItemFlagId - 7000` (deltas 5,200 / 6,000 / 6,100 / 6,999 …, and some rows
whose real flag is a block flag entirely). A table keyed on the row id therefore
addresses the wrong byte for those, confidently. **Every pickup table stores the
`getItemFlagId`**; `pickup_data.rs` held row ids until v0.36.0 and read the wrong bit
for 220 entries.

Critically, a bare 10-digit id with localId < 7000 is AMBIGUOUS between an open-world
flag and a pickup row_id — the two families sit 500 bytes apart and nothing in the value
tells them apart. The caller must choose the family; a function that guesses reads a
plausible wrong bit.

**Claims Store**:
The pipeline-generated collection of Claims consumed by the applications (successor of
ground_truth_offsets.json). Never hand-edited; regenerable from Evidence at any time
(`er-save-reader knowledge run` → `knowledge/claims/event-flags.json`).
_Avoid_: ground truth file

**Attributed Transition**:
A before/after capture pair labeled with the in-game action it brackets (boss kill,
grace discovery, pickup). The pipeline's strongest instrument: an isolated bit flip
matching the expected flag, cross-checked within the after-file, is Verified on its own.

**Evidence Catalog**:
The committed index of all Evidence: paths, sha256 checksums, capture context, slot
descriptions. Binaries live outside the repo; the catalog makes silent edits detectable.

**Conformance Fixtures**:
Committed test data (reference slots plus known byte assertions) that define the canonical
Coordinate Convention. Code that disagrees with fixtures is wrong by definition.

**Status Ladder**:
Hypothesis → Corroborated (one verification method) → Verified (kill transition, or two
independent methods). Disproven claims persist as Tombstones. Applications consume
Corroborated and Verified only.

**Tombstone**:
A disproven Claim kept in the store with its refuting evidence, so the idea cannot be
re-proposed later.

**Epistemic Header**:
The standard block at the top of every `docs/` file (added in the 2026-07-20 docs audit,
BACKLOG step 6) that states, before the body, how far to trust the file. One **Status**
line — CURRENT / ERA-MIXED / SUPERSEDED / STABLE-METHODOLOGY / LIVING-RECORD — plus four
fields: **Claims** (what it asserts), **Evidence** (what backs it), **Methodology** (how it
was derived), **Obsolete** (what is superseded, and where the current source now is). Its
job is to stop an era-mixed doc from misleading a reader who trusts a stale number. When a
doc is edited, update its header rather than letting body and header drift apart.

**Reader**:
What this project is (ADR-0009). It reconstructs a character's state from a save file the
way the game loads one, and stops there — it never writes a save back. Say "the reader",
not "the editor"; the old name described a tool that no longer exists.

**Dormant**:
Code kept in the tree but excluded from the default build, so it stays resurrectable
without being live. The save write-back path is dormant behind `feature =
"save-writeback"` (ADR-0009). Dormant is not the same as dead: dead code would be
deleted, and dormant code is expected to keep compiling under its flag. It is also not
the same as unreachable — the write path was unreachable *and* compiled for months, which
is exactly the state ADR-0009 exists to end.

### Instruments

**Snapshot**:
A full save file captured at a moment in time, usually as a before/after pair around a
single in-game action. Evidence.

**Timeline**:
The sequence of sparse byte diffs captured by the elden-map agent (slot_diffs). The raw
diff records are Evidence; the per-entry metadata (eventFlagsOffset, bossesDefeated,
gracesDiscovered) is a set of Claims made by the agent's code at capture time, and has
been shown to be partly wrong.

**Multi-slot Differential**:
Verification method: compare a byte across character slots with known different
progression; the expected presence/absence pattern must match.

**Multi-file Differential**:
Verification method: track a set-monotonic flag bit across successive captures of the
same slot — once set it must stay set. Valid only when the family base is attested
unchanged (independently re-measured by another resolved pair in the later files).
Used by the pipeline to disambiguate multiple candidate flips.

**Kill Transition**:
Verification method: a byte observed changing at the recorded moment of a specific
in-game event (e.g. 00→ff on a boss kill) inside one self-consistent EF window. An
*attributed* kill transition (a labeled before/after capture pair) is Verified on its
own; an *unattributed* flip inside a timeline window needs a second independent method
(e.g. Reward Corroboration) to reach Verified. **A "window" may not cross a sparse-diff
segment boundary** (added 2026-07-22): the Bee timeline is not one chain but **28
segments separated by 27 boundaries** (exact census, `knowledge timeline-segments` →
`knowledge/claims/timeline-segments.json`; this supersedes the "≥21" lower bound, which
was a long-gap sample). Across a boundary the previous capture's new-values stop matching
the next one's old-values; within a run agreement is **exactly 100.00% on every one of
the 3,837 continuous pairs** — continuity is all-or-nothing here, not a matter of degree.
Play happened unobserved at a boundary, so a flip spanning one is not a transition.

> **CORRECTED 2026-07-22 (same day), and the correction matters more than the rule.**
> The v0.36.1 entry went further and claimed boundary-crossing was *the mechanism* behind
> the rejected re-annotation's flags "transitioning" 0→1 up to 69 times. **That causal
> claim is refuted** (`knowledge timeline-flips` →
> `knowledge/claims/timeline-flip-monotonicity.json`): excluding boundary pairs removes
> 0.954% of set-monotonicity violations while excluding 0.878% of pairs — an enrichment of
> **1.09×**, i.e. boundary pairs violate at the same rate as ordinary ones. 107,183
> violations survive segment confinement, worst offender unchanged at 57×. The boundary
> rule above stands on its own evidence (the 100%/collapse contrast is real); what does
> NOT stand is treating it as the explanation for the monotonicity failure. See
> `docs/BACKLOG.md` step 3 for what the remaining cause is believed to be.

**Flagless Pickup**:
A pickup whose `ItemLotParam_map` row has **`getItemFlagId = 0`** — the game records
nothing when it is taken, which is what lets gathering points respawn
(`AssetEnvironmentGeometryParam.isEnableRepick = 1`). A before/after capture of one can
never become an attributed pair, however clean the captures are: there is no bit to find.
**Check `getItemFlagId` before capturing a pair, not after.** Established 2026-07-22 on
the Confessor c06-c08 Golden Centipede captures (goods 20820 → lot 998200, flag 0), which
had been carried in `docs/BACKLOG.md` as a "data gap" until the null result was paired
with a positive control on c05-c06 and c09-c10 — same character, same map, same
instrument — to show the absence was measured rather than missed. Distinct from a
*Tombstone* (a disproven claim); this is a claim that can never be made.

**Reward Corroboration**:
Verification method: a boss-specific unique item (remembrance, boss soul, unique key
item/weapon) appearing in the parsed inventory in the same capture window as a flag
flip. Strong per-boss evidence; rune jumps are only a weak supporting signal. Inventory
deltas must be computed by item identity, never by GaItem handle (handles churn).

**Keyframe**:
A periodic full-slot snapshot inside the Timeline (plus per-entry state checksums),
making diff-chain replay verifiable and re-startable. Evidence.
