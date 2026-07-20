# Exported flag readers refuse rather than guess

The wasm crate's exported entry points — `is_flag_set`, `get_flag_offset`,
`get_flag_offset_calibrated` — will REFUSE when they cannot resolve a flag's position,
even though refusing breaks consumers outside this repo (elden-map). They will not be
kept working by continuing to return a static offset.

The trigger: after the v0.30.0 cutovers no app-side reader touches the frozen legacy
store, but these exports still route through `calculate_dungeon_flag_offset_unified` into
`get_dungeon_general_bases()` — the "+3375 per area" table whose own audit comment
records entries disproven by every save on this machine. They return a plausible-looking
wrong offset rather than failing, which is the precise failure this migration exists to
remove, and elden-map has already inherited a poisoned build once.

The decisive point is not politeness to callers, it is that the export SHAPE encodes a
model the project has abandoned. `get_flag_offset(flag_id)` takes an id and promises a
static byte offset. Every flag family sits at a base that floats per save, so there is no
correct static offset to return — the signature cannot be satisfied. "Re-point it at the
resolver" is therefore not an available option: the resolver needs the flag region (or
slot bytes) to locate a family for THAT save. Any honest replacement takes the bytes,
which is a different function. Consumers break either way; the only choice is whether
they break visibly or keep reading wrong bits.

So: visibly. A caller that breaks gets fixed. A caller that reads a wrong bit does not
know it needs fixing, and neither does the person looking at the map it draws. This is
the same reasoning that made `None` a first-class state in the app (ADR-0006's cutovers,
where collapsing unknown to false reported 0/110 boss defeats on a progressed character)
— applied at the crate boundary instead of the UI.

Rejected alternatives. *Keep the static exports and document them as unreliable*: the
existing docs already said the table was disproven, and the exports were still recommended
to elden-map in this repo's own backlog until 2026-07-20 — documentation did not stop it,
because the call site does not read the docs. *Deprecate with a grace period*: a
deprecation window is a period during which known-wrong bits keep being served, and
nothing forces the window to end. *Silently return zeros*: indistinguishable from "flag
not set", which is the failure mode by another name.

Consequence: elden-map must move to the region-taking readers (`is_world_state_flag_set`,
`is_tile_pickup_set`, `is_tile_world_flag_set`, `is_dungeon_flag_set`,
`is_dungeon_pickup_set`) and handle their three-state result. That is a coordinated
change across two repos and should be sequenced deliberately — but it is not a reason to
keep serving wrong answers in the meantime.

---

**Implemented 2026-07-20. The decision applied more widely than this ADR first stated.**

The three named exports were the ones noticed, not the full set sharing the defect. Seven
were removed: the three above plus `is_flag_set_calibrated`, `calculate_dungeon_pickup_offset`,
`calculate_world_pickup_offset_by_row_id`, `calculate_tile_pickup_offset`, and the
`get_dungeon_pickup_sections` / `get_tile_base_offset` / `get_world_pickup_row_id_base`
accessors. Five base tables went with them, and the crate's `HashMap` import fell unused —
a useful signal that none survived.

Widening was forced by the reasoning already written above, not by a new judgement. If the
argument is that a `flag_id → static offset` signature cannot be satisfied, it does not
matter which table sits behind a given one; `calculate_dungeon_pickup_offset` has the same
shape and the same 88 crate-baked bases. Stopping at three would also have made the
conformance test unwritable as specified — "no exported entry point reaches a legacy base
table" is an assertion about an empty set.

The boundary that emerged is sharper than "refuse": **an export may not source a flag's
position from inside this crate.** It may receive the save bytes and resolve the family
for that save, or receive the base from its caller. This is why
`calculate_tile_pickup_offset_calibrated(flag_id, tile_base)` survives while
`calculate_tile_pickup_offset(flag_id)` does not — identical arithmetic, but one is handed
the base and the other invents it. The surviving function is also what the correct path
uses: `tile_read` calls it with base 0 and adds a resolved base, so it is tile geometry
rather than a claim about where a family sits.

Guarded by `crates/wasm-event-flags/tests/export_shape_conformance.rs`, whose three checks
were each verified by mutation rather than assumed. The load-bearing one is structural, not
a name list: any export answering a flag position/state question must receive `&[u8]` or an
explicit base. A banned-names list cannot catch this model regrowing under a new name, and
that is the form regrowth would actually take.
