# Conformance fixtures define the Coordinate Convention, not code

Four EF-anchor implementations disagreed on the same save slot (wasm structural ~+146k
overshoot, python content search on a lookalike region, src/save struct parse correct on
one save but failing on another, capture-agent anchors drifting between eras). We decided
the canonical Coordinate Convention is defined by a committed conformance fixture set —
the five test slots plus known byte assertions (catacombs kill bytes, grace validation
bytes, the sd_000259 kill transitions) — and a single reference implementation in
crates/wasm-event-flags that must pass them. The other detectors are deleted. When code
and fixtures disagree, the code is wrong. This prevents a future implementation bug from
silently redefining every stored offset, which is exactly how the timeline metadata was
poisoned after Feb 2026.
