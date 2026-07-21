# Knowledge base — reference tables

> **Epistemic header** (added 2026-07-21 · BACKLOG step 5)
> **Status: SUPERSEDED REFERENCE.** Third-party / pre-reset tables kept for provenance and
> cross-referencing, NOT as a source of truth. Nothing here positions a flag in a live save.
> - **Claims** live in `knowledge/claims/event-flags.json` (pipeline-generated, ADR-0004).
> - **Positions** are resolved per save at runtime by `crates/wasm-event-flags` (ADR-0005);
>   family bases float per save (`CONTEXT.md` → *Origin*), so no fixed offset here is usable.
> - **Methodology**: verify against primary sources (`docs/DATA-SOURCES.md`), discard what
>   can't be proven.

## `ce-era-event-flags-rosetta.json`

The **Cheat-Engine-era Rosetta table**, extracted verbatim (BACKLOG step 5) from the retired
`src/db/event_flags.rs` before it was deleted. 5,751 entries, one per flag id:

| field | meaning |
|---|---|
| `flag_id` | Elden Ring event flag id |
| `byte_offset` / `bit_position` | **IN-MEMORY** address (CE-era), NOT a save-file byte position |
| `name` | best-effort label (4,915 populated; unverified provenance) |
| `category` | best-effort class (`Grace`, `Boss`, `WorldPickup`, `Landmark`, …) |
| `coords` | optional in-game world position (384 populated): `x/y/z` + `map_area/map_x/map_z` |

**Trust: LOW / UNVERIFIED.** This is the in-memory (Cheat Engine) coordinate convention that
pre-dates the knowledge-base reset. The offsets are memory coordinates that decode to flag ids
via the 125-byte block model (see the *Boss Reviver mod coordinates* memory) — they are **not**
save-file positions and must never be used to read a save. Names/categories are third-party
labels of unverified origin. It is preserved as a legacy cross-reference (e.g. to recover a
plausible name for a flag id, then verify it against the primary source), in the spirit of
"a recorded dead end is worth more than a deleted one."

*Not in the evidence catalog:* `knowledge/evidence-catalog.json` indexes **out-of-repo raw
evidence** by sha256 (ADR terms in `CONTEXT.md`). This file is derived, in-repo reference, so
adding it there would misrepresent derived labels as raw evidence. Provenance is recorded here
and in the file's own `_epistemic` block instead.
