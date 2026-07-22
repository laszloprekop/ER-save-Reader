# A reader, not an editor

This project reconstructs a character's state from a save file the way the game itself
loads one, and stops there. It does not write saves back. The write-back path is not
deleted — it stays in the tree behind `feature = "save-writeback"`, off by default — but
it is dormant, and the project is renamed from ER-save-Editor to ER-save-Reader to stop
the old name promising something the app no longer does.

The trigger is that the editing had already stopped without anyone saying so. The Save
button was disabled with a strikethrough; `InventoryRoute::Add` carried the comment "Can't
reach this state anymore"; the region checkboxes were wrapped in `add_enabled(false)`;
`App::save` had no callers at all. What remained was machinery: 40 `impl Write` blocks, 44
`SaveType` mutators, `ViewModel::update_save` and its six fan-out methods, and a character
importer reachable from nothing. All of it compiled, none of it ran. That gap is the
problem — the code says the app can write saves, the app cannot, and the name on the
window agrees with the code rather than the behaviour.

The decisive point is what the leftover write path does to reasoning about the read path.
This repo's whole discipline is that a flag's position is resolved per save and never
hardcoded (ADR-0006, ADR-0008), because a wrong offset read produces a plausible-looking
wrong answer. A wrong offset *written* corrupts someone's character. Keeping a write path
that nobody exercises means keeping a set of byte-layout assumptions that nothing tests
and nobody has reason to re-derive when the read side learns something new. The read side
has been re-derived repeatedly — the family Origin work, the ADR-0008 cutover — and the
write side has not followed. It is not merely unused; it is unused *and* drifting away
from what the project now believes about the file format. Deleting it outright would be
defensible. Leaving it live would not be.

So: dormant, gated, and explicitly labelled. The gate is a Cargo feature rather than a
directory of quarantined files because the write impls interleave with the `Read` impls
and struct definitions they mirror — splitting them out would scatter a coherent
serialisation model across the tree and make resurrection a merge rather than a flag.
`cargo check --features save-writeback` compiles it, which is the cheap guard against it
rotting into something that cannot be brought back at all.

Rejected alternatives. *Delete it and rely on git history*: "resurrectable" then means
knowing which commit to look in and hand-reapplying ~1,500 lines across 12 files; the
feature flag costs one line in Cargo.toml and keeps the code adjacent to the read code it
mirrors, which is where anyone reviving it would want it. *Leave it compiled but
unreachable*: that is the state we are leaving, and the state that let the drift go
unnoticed — an unreachable path that still compiles reads as maintained. *Keep the name
and only wither the code*: the name is how the drift got normalised; a tool called an
editor accumulates editor code by gravity.

Consequence. The binary is now `er-save-reader`, so the `generated_by` provenance strings
in `knowledge/claims/*.json` will change on the next `knowledge run` — expected churn, not
a claim change (ADR-0004: the store is regenerated, never hand-edited). `ground_truth_offsets.json`
is untouched, being frozen (ADR-0006). The user config directory moved from
`~/.er-save-editor` to `~/.er-save-reader`. elden-map refers to this repo by name in
comments only — no build path or symlink crosses the boundary — so its references were
updated as prose.

What this does *not* change: the read path, the resolver, the claims pipeline, the wasm
crate's exports, or the UI. The reader is the product now; it was already the only part
that worked.
