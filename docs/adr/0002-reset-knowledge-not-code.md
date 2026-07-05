# Reset the knowledge base, not the application code

Facing accumulated contradictions, we considered restarting from a fresh clone of
upstream ClayAmore/ER-Save-Editor. We decided against it: upstream is a stat/inventory
editor with none of the event-flag research capability; this repo is 237 commits past a
snapshot import with no upstream remote (no clean re-merge path); crates/wasm-event-flags
is a production dependency of elden-map; and the save-format structural knowledge
(BND4/slots/GaItems/EF section parsing) is correct and hard-won. The contamination lives
in the knowledge artifacts (ground_truth_offsets.json, formula base tables) and in
redundant implementations around them. Therefore: condemn and rebuild the knowledge base
from Evidence via a repeatable pipeline (a reset we can re-run), and consolidate — not
discard — application code. Coordinated API changes to the wasm crate are acceptable;
elden-map is updated in the same effort.
