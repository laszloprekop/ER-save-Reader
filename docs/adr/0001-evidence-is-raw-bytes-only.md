# Evidence is raw bytes only; everything else is a Claim with provenance

The 2026-07-05 audit showed that stored "ground truth" mixed offsets measured against
different EF anchor conventions from different verification eras, and that timeline
metadata (eventFlagsOffset, bossesDefeated) recorded by the capture agent was partly
wrong (~146k anchor overshoot after Feb 2026; flicker-artifact boss annotations). We
decided that only raw bytes count as Evidence — game extracted files, raw save
snapshots, raw timeline diff records — and every derived statement (offsets, bases,
formulas, anchors, boss annotations) is a Claim that must carry provenance: coordinate
convention, method, evidence references, date. Claims without evidence references are
Hypotheses and must not be consumed by application code.

## Considered Options

- Treat the timeline including its metadata as truth and patch it where wrong — rejected
  because patched and unpatched entries become indistinguishable later.
- Drop the timeline entirely as polluted — rejected because its raw diffs contain the
  only recorded kill-moment transitions we have (e.g. the sd_000259 verification of the
  m14 base).
