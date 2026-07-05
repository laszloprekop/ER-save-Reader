# One reference implementation in crates/wasm-event-flags; everything else consumes

Six implementations of save parsing / EF anchoring / flag formulas exist across two repos
(wasm crate, src/save struct-parse, python SaveParser, deprecated flag_formulas.py, and
elden-map's slot-layout.ts + ground-truth-formulas.ts), and they disagree. Decision: the
reference implementation lives in crates/wasm-event-flags, gated by the conformance
fixtures (ADR-0003). The working struct-parse logic from src/save is ported into it;
src/save calls the crate; elden-map keeps only wasm-loader.ts and deletes its TypeScript
reimplementations; Python tooling consumes parsed output via an `ef-dump` CLI subcommand
and never parses saves itself. Consequences: src/db/event_flags.rs (46k lines,
in-memory/CheatEngine coordinates) moves out of the app into knowledge-base inputs as
in-memory-convention claims (useful as a Rosetta stone for decoding community CE data);
the ~50k lines of Python lab scripts are distilled into claims/tombstones/docs and
deleted; Evidence stays outside the repo but is pinned by a committed catalog of sha256
checksums and capture context that the pipeline verifies.
