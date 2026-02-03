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

## False Negative Investigation Protocol

**MANDATORY**: When investigating a false negative (auto-detection fails where manual succeeds), follow this evidence-based protocol BEFORE proposing any fixes:

### Phase 1: Evidence Collection (No Speculation)

1. **Read the actual save file** - Use hex dump or binary read to locate the flag bytes empirically
2. **Verify the byte offset** - Calculate expected offset using ground truth, then confirm it matches actual location
3. **Check both flag systems** - For world pickups, verify both tile flag AND block flag states
4. **Document observed vs expected** - Write down what the bytes actually show

### Phase 2: Multi-Slot Differential (Gold Standard)

Reference: `docs/discovery-verification-cycle.md`

- Compare flag state across character slots with known different progression
- Use the test slots (V1, V2, V3) specifically designed for this purpose
- A flag verified across multiple slots with expected differences = HIGH confidence

### Phase 3: Corroboration Check

Reference: `docs/CORROBORATION-SYSTEM.md`

- For world pickups: tile flag SET + block flag SET = corroborated
- If they disagree: the disagreement IS the clue - don't dismiss it
- Inseparable evidence (boss + grace, etc.) must be consistent

### Phase 4: Calibration Verification

- Check if calibration is returning correct base offset for THIS save file
- The ground truth base (485330) only applies to specific save structures
- Level/progression can shift the actual base by tens of thousands of bytes

### Only After Evidence Is Gathered

- Build a concrete test case with before/after hex dumps
- Document the evidence with confidence level (VERIFIED/LIKELY/UNVERIFIED)
- THEN propose a fix based on empirical findings

**NEVER skip directly to proposing fixes**. The methodology exists because event flag detection has many moving parts (calibration, per-character offsets, formula correctness) and speculation wastes effort.

---

## Technical Documentation

| Topic | Document |
|-------|----------|
| **System architecture** | `docs/ARCHITECTURE.md` |
| Event flag geography & formulas | `docs/EVENT-FLAG-GEOGRAPHY.md` |
| **WASM shared detection** | `docs/WASM-EVENT-FLAGS.md` |
| Discovery methodology | `docs/discovery-verification-cycle.md` |
| Corroboration system | `docs/CORROBORATION-SYSTEM.md` |
| Database coverage | `docs/DATABASE_COVERAGE_ANALYSIS.md` |
| Ground truth data | `ground_truth_offsets.json` |

**Single Source of Truth**:
- Offset values: `ground_truth_offsets.json` (never use `flag_formulas.py`)
- EventFlags detection: `crates/wasm-event-flags/` (shared with elden-map via WASM)
