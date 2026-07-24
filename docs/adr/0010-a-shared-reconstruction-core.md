# A shared reconstruction core, compiled two ways

Character reconstruction — turning a slot of a save into a character's state — becomes a
single Rust library, the **Character Reconstructor**, extracted into its own repository
(`er-reconstruct`) and compiled two ways: **native**, linked in-process by this reader's
CLI and egui, and **WASM**, called by elden-map's browser *and* its Node server. elden-map
deletes its parallel TypeScript reconstruction and calls the core instead. The core returns
**facts** (`ReconstructedCharacter`) plus a separate **canonical-name** lookup; naming for
display, map placement, and rendering stay in each app's Enrichment stage. This widens the
pattern `wasm-event-flags` already proves for flag resolution to cover the whole
reconstruction, and it layers *on top of* that crate rather than absorbing it.

The trigger is duplication that no mechanism keeps honest. Reconstruction exists twice: in
Rust (`src/save/`, `src/vm/`, `src/db/`) serving this reader's CLI and GUI, and again in
elden-map's TypeScript (`server/src/saveParser.ts`, `inventoryParser.ts`, the 1,555-line
`character-explorer-parser.ts`, `shared/slot-schema.ts`, `field-parsers.ts`) serving its
browser upload flow and its live save-watching server. The only shared code is
`wasm-event-flags`, and it covers only flag *offset resolution* — everything above it is
two implementations of the same logic. A bug in reconstruction is therefore fixed twice or
not at all, and a fix to one silently diverges from the other. The stated goal is that a
fix to a faulty reconstruction reflect in both UIs; two implementations cannot deliver that,
by construction.

The decisive point is that only shared *code* satisfies the goal — a shared *contract*
does not. A language-neutral spec plus a golden test corpus, with both implementations kept
alive, would *catch* divergence in CI but never *heal* it: the bug is still fixed twice.
"Fix once, reflected everywhere" is a property of a single implementation, and nothing
weaker. That is what forces the expensive half of this decision — elden-map's working
TypeScript reconstruction is deleted, not merely conformance-tested — and it is why the
cost is worth paying.

Three boundaries keep the core small and free of app coupling. First, the core is a
**library, not a running service**: neither frontend needs *remote* computation, they need
the *same* computation, and a service would saddle the server-less reader with a process
dependency it does not have today (ADR-0009). Second, the core returns **facts only** —
ID-keyed resolved state — because the bug class in view ("faulty reconstruction": a wrong
stat, a missed boss, a mis-decoded item) lives entirely at the facts layer, while names and
coordinates are static lookups that do not reconstruct wrong. Third, the split between
**shared game-knowledge and app-specific projection**: canonical names are game files,
identical for every save, so they are centralized (as a separate `nameOf(id)` lookup, not
baked into the facts); map coordinates and community POI labels are elden-map's projection
and never enter a reader crate. elden-map layers its community POI database on top of the
shared canonical name in its own Enrichment stage — single-sourced game truth, app-owned
presentation.

The core's fact set is the **union** of both apps' needs, not what the reader shows today.
elden-map surfaces facts the reader has no reason to (player world position, for its map);
those are ported *into* the core, not dropped. Migration is therefore *widen the core to
the union, then delete the TypeScript* — a **strangler**, one concern at a time (bosses,
pickups, inventory, stats, position), each guarded by a **conformance corpus** (real saves →
expected facts, via the Multi-slot Differential method) that must show identical output
before the corresponding TypeScript is removed. The corpus is permanent: it is the oracle
during migration, the `native == WASM` parity gate, and the CI regression guard afterward.

Consumption is **drift-proof, not zero-human-action**. elden-map builds the WASM from
**pinned source in its CI**, never a hand-committed blob, so the artifact can never disagree
with the source it claims to be built from. Adopting a fix is a deliberate pin bump — which
is *preferable* to silent propagation: a reconstruction change should not alter a live map's
behaviour mid-session without someone choosing to adopt it.

Rejected alternatives. *Shared contract, two implementations kept alive*: catches drift,
never heals it — the bug is still fixed twice, which is the whole problem. *A running
backend service both frontends call*: forces the server-less reader to depend on a live
process and buys an ops surface (deploy, versioning, latency, offline) neither app needs,
since neither needs *remote* computation. *A monorepo fusing both apps*: the closest to
silent propagation, but silent propagation is a non-goal, and fusing two mature codebases
with different toolchains (cargo + pnpm) and governance is a large move unjustified by
sharing one crate. *Rename-and-widen `wasm-event-flags`*: disturbs the tightly-guarded flag
resolver and its `export_shape_conformance` invariants (ADR-0008); layering a new crate on
top leaves those exactly as they are. *Keep the checked-in `.wasm` with a CI freshness
gate*: a fine stopgap, but it leaves the core embedded in the reader's workspace and keeps
a binary blob in elden-map's tree. *Big-bang cut-over*: every concern unverified at once
against a live app; the strangler keeps elden-map green throughout.

What this does *not* change. ADR-0008 still binds the flag layer: the core holds
reconstruction *logic*, never hardcoded flag base tables — positions stay resolved per
save. `wasm-event-flags` keeps its exports and invariants unchanged; the core sits above
it. This reader stays a reader (ADR-0009) and stays server-less. elden-map's live-session
concerns (file-watching, slot diffing, timeline, WebSocket) are *not* reconstruction and
stay in elden-map — the core is handed bytes and returns facts; watching for new bytes is
the caller's job.
