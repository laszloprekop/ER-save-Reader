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
