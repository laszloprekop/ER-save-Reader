# ER Save Editor — Event Flag Research

Save editor and event-flag research platform for Elden Ring. This context covers the
epistemic layer: how facts about the save format are established, stored, and consumed.

## Language

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
A group of event flags whose bytes move together within a save (e.g. the grace family,
the catacombs family). Families float independently across saves, so each family has its
own per-save base.

**EF Anchor**:
The detected per-save base of one flag family (by default the grace family, which the
detection fixtures pin). A per-slot measurement, not a convention, and not valid for
positioning other families.

**Claims Store**:
The pipeline-generated collection of Claims consumed by the applications (successor of
ground_truth_offsets.json). Never hand-edited; regenerable from Evidence at any time.
_Avoid_: ground truth file

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

**Kill Transition**:
Verification method: a byte observed changing at the recorded moment of a specific
in-game event (e.g. 00→ff on a boss kill) inside one self-consistent EF window. An
*attributed* kill transition (a labeled before/after capture pair) is Verified on its
own; an *unattributed* flip inside a timeline window needs a second independent method
(e.g. Reward Corroboration) to reach Verified.

**Reward Corroboration**:
Verification method: a boss-specific unique item (remembrance, boss soul, unique key
item/weapon) appearing in the parsed inventory in the same capture window as a flag
flip. Strong per-boss evidence; rune jumps are only a weak supporting signal. Inventory
deltas must be computed by item identity, never by GaItem handle (handles churn).

**Keyframe**:
A periodic full-slot snapshot inside the Timeline (plus per-entry state checksums),
making diff-chain replay verifiable and re-startable. Evidence.
