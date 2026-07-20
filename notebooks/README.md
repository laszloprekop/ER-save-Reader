# Notebooks

Exploratory analysis. **Nothing here is a source of truth.** Claims live in
`knowledge/claims/` and are pipeline-generated (ADR-0004); flag positions are resolved
at runtime by `crates/wasm-event-flags` (ADR-0005).

A notebook may keep results that were later refuted. That is fine — a recorded dead end
is worth more than a deleted one, provided it says so at the top. Every notebook here
must open with a status header stating which era it belongs to and what is still valid.

| notebook | status | summary |
|---|---|---|
| `ml_flag_discovery.ipynb` | **ARCHIVED — pre-reset (2026-02-18)** | ML search for unknown flag regions in the timeline diffs. Conclusions refuted: it labels from the frozen `ground_truth_offsets.json` (including the tombstoned tile base 337,375), treats the catacombs u32-record list as a bitmap, and aggregates offsets across snapshots as if positions were stable. Kept for its methods — cells 27–28 prototype the flip-clustering design that `docs/BACKLOG.md` step 3 names as the next viable timeline approach. |

## Before writing a new one

Two mistakes sank the archived notebook, and both are easy to repeat:

1. **Do not use absolute offsets across captures.** Flag families sit after an
   append-only list that grows as the character plays, so a position measured in one
   capture does not hold in the next. Resolve the origin per save first —
   `wasm_event_flags::resolve_family_base_in_ef` — and work relative to it.
2. **Do not take labels from `ground_truth_offsets.json`.** It is frozen and superseded
   per family. Use `knowledge/claims/event-flags.json`, and check its tombstones before
   trusting any offset convention.
