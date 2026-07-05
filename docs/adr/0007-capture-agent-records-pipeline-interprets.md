# The capture agent records; the pipeline interprets

Timeline metadata (eventFlagsOffset, bossesDefeated, gracesDiscovered, inventoryDelta)
was computed by the capture agent at capture time with then-current detection code, and
each detection era froze its bugs into the record (~146k anchor jumps after Feb 2026,
flicker-artifact boss annotations, handle-churn-polluted inventory deltas). Decision:
capture-time interpretation is abolished. The agent records only cheap verifiable facts —
raw diffs, GaItems end, full-slot state checksums per entry, periodic full-slot keyframes
(every N entries and on GaItems resize), agent+wasm versions, timestamps, player
position. All interpretation is re-derived by the knowledge pipeline from raw bytes and
is re-runnable whenever detection improves. Existing metadata fields remain in old
entries as untrusted hints (ADR-0001).

Interpretation in the pipeline diffs parsed domain objects, not raw bytes: inventory
deltas by item identity (never by GaItem handle — handles churn across sessions), flags
per family. Boss-defeat verification combines methods: an attributed kill transition
(labeled capture pair) is Verified alone; an unattributed flag flip in a timeline window
reaches Verified only together with Reward Corroboration (boss-specific unique item
appearing in the same window); either signal alone is Corroborated. Rune jumps are a
weak supporting signal only.

Rejected alternative: "make the agent smarter" (fix detection inside the agent and keep
capture-time interpretation) — every future detection improvement would mint another era
of stale frozen interpretations. Implementation phasing: capture-flow changes ride the
coordinated elden-map change; reward corroboration lands with the pipeline (migration
step 3).
