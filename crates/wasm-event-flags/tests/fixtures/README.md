# Anchor Conformance Fixtures

Per ADR-0003, these fixtures **define** the grace-family anchor convention.
Each file is the first 131,072 bytes of one save slot's data (carved at BND4
offset `0x300 + slot*(0x280010) + 0x10`), enough to cover GaItems + the
detection window + the validation/assertion bytes.

Provenance (source files live in `Elden Ring save files/`, outside the repo):

| Fixture | Source | Slot | Source sha256[:16] | Fixture sha256[:16] |
|---|---|---|---|---|
| backup_2026-01-11_slot0 | ER0000-backup-2026-01-11.sl2 | 0 (Confessor, mid-game) | 420e4bc1fa843c9c | 227c621b10ec69c4 |
| backup_2026-01-11_slot1 | ER0000-backup-2026-01-11.sl2 | 1 (Wretch, tutorial kill only) | 420e4bc1fa843c9c | edf5befd7a7171fa |
| backup_2026-01-11_slot2 | ER0000-backup-2026-01-11.sl2 | 2 (V1, debug) | 420e4bc1fa843c9c | d5cfaf7a0af8c306 |
| backup_2026-01-11_slot3 | ER0000-backup-2026-01-11.sl2 | 3 (V2, debug) | 420e4bc1fa843c9c | 5ff8ffde195fe856 |
| backup_2026-01-11_slot4 | ER0000-backup-2026-01-11.sl2 | 4 (V3, negative control) | 420e4bc1fa843c9c | 65d0b48328daf272 |
| confessor_lvl93_slot0 | "Confessor - level 93 snapshot" | 0 | e67045fede31c503 | 3cfdd82573ab7c23 |
| b24_watchdog_before_slot0 | b24 capture (pre Watchdog kill) | 0 | d3fd0c00fea43a42 | b7940a4b29454c6a |
| b25_watchdog_after_slot0 | b25 capture (post Watchdog kill) | 0 | 69f07362014836b8 | e549db515f9e0df3 |

Key empirical facts these fixtures encode (2026-07-05 investigation):

- The pre-2026-07 "structural walk" overshot the flag region by ~146k bytes
  and reported `confident: true` unconditionally — the ~222k position it
  returned is a lookalike, not EventFlags.
- GaItems-end parsing is byte-exact (verified via the PlayerGameData name
  field at gaEnd+148 on slots 0-2).
- Grace-family base sits at gaEnd + ~35.1k..37.0k across all observed saves;
  the detection window is [gaEnd+30k, gaEnd+45k].
- Flag FAMILIES float independently per save (grace vs catacombs family:
  Δ0 on the Bee timeline save, Δ~77-141 on b24, more elsewhere) and even
  shift by different amounts within one save pair (b24→b25: GaItems +16,
  flag region +4). Byte-exact per-family bases require flip-pair analysis
  in the re-verification pipeline; the detected offset here is the
  grace-family base only.
