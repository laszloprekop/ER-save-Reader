# Frozen legacy store with per-family cutover

During the knowledge-base rebuild, ground_truth_offsets.json is frozen read-only and keeps
feeding the applications per flag family (graces, boss defeats, tile/dungeon pickups, …).
A family flips to the pipeline-generated claims store when it reaches parity there; its
legacy entries are then either promoted (re-proven with provenance) or tombstoned. So a
reader will find the "condemned" json still wired into the app mid-migration — that is
deliberate: it keeps the UI alive and, being frozen, cannot accumulate new contamination.
Alternatives rejected: hard cutover (apps mostly "unknown" for weeks), dual-display
(double UI wiring for a transition period). The pipeline itself is a `knowledge`
subcommand family in this repo's binary, reusing the reference implementation (ADR-0005)
so pipeline and app parsing cannot drift. Re-verification order: graces first (densest
evidence and they are the anchor-validation set), then boss defeats, then pickups.
