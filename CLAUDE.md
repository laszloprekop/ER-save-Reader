## Commit Protocol

**IMPORTANT**: Never commit automatically. Always use the `/snapshot` command to ensure the commit protocol is followed properly. This ensures version bumps, changelog updates, and documentation are handled consistently.

## Remembering Command execution fault

**IMPORTANT**: When an (allowed) executed command throws an error and a corrected format of the same command is executed afterwards successfully, take note of the correct command form to prevent burning tokens repeatedly.

---

## Knowledge Resource files (single source of truth):

Decompiled game resource files:
'/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files'

## Game save files with five character slots:

- Slot 0: Confessor, mid-game progression
- Slot 1: Wretch, early game, few steps of progression, item collection, one boss defeat
- Slot 2: V1, very little progression, made for item pickup debugging
- Slot 3: V2, similar little amout progression as V1, different path taken, same item pickup for debugging
- Slot 4: V3, similar little amout progression as V1, different path taken, no pickup for true negative diff
- '/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files'

## Third party resource usage
Treat third party resources with caution because we don't have control over their accuracy, completeness, or reliability. Most of the time they are specific to a certain game version, thus outdated and many times partially implemented. Always verify information from third-party sources against primary sources and discard them if their correctness can not be proven.

---

## Technical Documentation

| Topic | Document |
|-------|----------|
| **System architecture** | `docs/ARCHITECTURE.md` |
| Event flag geography & formulas | `docs/EVENT-FLAG-GEOGRAPHY.md` |
| Discovery methodology | `docs/discovery-verification-cycle.md` |
| Corroboration system | `docs/CORROBORATION-SYSTEM.md` |
| Database coverage | `docs/DATABASE_COVERAGE_ANALYSIS.md` |
| Ground truth data | `ground_truth_offsets.json` |

**Single Source of Truth**: Always use `ground_truth_offsets.json` for offset values. Never use `flag_formulas.py` which contains outdated values.
