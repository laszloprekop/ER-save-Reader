# EF Discovery/Verification Chain — Entry Points & Reports

**Last updated**: 2026-02-15

The chain has two distinct workflows: **Discovery** (finding unknown offsets) and **Verification** (confirming known offsets). They share a common infrastructure layer.

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: SUPERSEDED — the pre-reset Python discovery/verification pipeline.** Every entry point listed (`run_verification.py`, `verify_captures.py`, `capture_agent.py`, …) is the old Python lab. Those scripts were **removed in step 5** (distilled in `docs/archive/PYTHON-LAB.md`); the live pipeline is `er-save-reader knowledge run` feeding the claims store.
> - **Claims**: the Python script call-graph for discovery and verification.
> - **Evidence**: none — an entry-point map of code being retired.
> - **Methodology**: replaced by the Rust knowledge pipeline (ADR-0004/0007) and the single resolver (ADR-0005). See `CONTEXT.md`, `docs/adr/`.
> - **Obsolete**: treat the whole file as a historical map; do not build new work on these scripts.

---

## Architecture Stack

```
ground_truth_offsets.json         ← Single source of truth
       ↓
ground_truth_loader.py            ← Python API for reading ground truth
       ↓
constants.py                      ← Save file structure constants
       ↓
utils.py                          ← Unified API (read slot, detect EF, check flag)
       ↓
Verification/Discovery scripts    ← Import from utils.py
```

---

## Verification Entry Points

### 1. `scripts/run_verification.py` — Full Verification Pipeline

The "main" entry point. Runs the complete verification framework against a save file.

```bash
python scripts/run_verification.py [--save PATH] [--categories graces,bosses] [--verbose]
```

**What it does:**
1. Loads extracted flags (`extracted_event_flags.json`) + manual completions (`flag-correlation-candidates.jsonl`)
2. Parses all save file slots via `SaveParser`
3. Tests each flag against all active slots using formula calculations
4. Cross-references auto-detection vs manual completion

**Report:** `ground_truth_offsets.json` + printed summary with per-category verification status and match ratios.

### 2. `scripts/verification/verify_captures.py` — Temporal Pair Verification

Verifies before/after capture pairs from the capture catalog (temporal diffs).

```bash
python scripts/verification/verify_captures.py [--filter tile] [--transitions] [--json /tmp/out.json]
```

**What it does:**
1. Reads `capture_catalog.json` with paired before/after snapshots
2. For each pair: detects EF offset, calculates flag offset via ground truth formulas, checks that the expected bit transitions 0→1
3. Optionally counts total EF transitions between before/after

**Report:** Summary table with verified/failed/inconclusive counts, broken down by formula type (tile/dungeon/block/simple).

### 3. `scripts/verification/verify_pickups.py` — Comprehensive Pickup Verification

Verifies all pickup flag formulas against a live or backup save.

```bash
python scripts/verification/verify_pickups.py [--save PATH] [--slot 0] [--all-slots] [--json /tmp/out.json]
```

**What it does:**
1. Reads slot data, detects EF, calibrates tile base
2. Scans tile section for all non-zero flags
3. Checks grace flags (block-based), progression flags, and dungeon boss flags
4. Rejects 0xFF padding false positives

**Report:** Table: Category | Checked | SET | FP | Rate — for tile pickups, block flags, and dungeon bosses.

### 4. `scripts/verification/verify_timeline.py` — Timeline Temporal Verification

Uses the s5-Bee slot_diffs (701 timeline entries) for longitudinal flag transition detection.

```bash
python scripts/verification/verify_timeline.py [--limit 50] [--json /tmp/out.json]
```

**What it does:**
1. Reads `slot_changes.jsonl` (timeline entries with inventory changes, graces discovered, bosses defeated)
2. Reads corresponding sparse binary diffs (6-byte records: `[u32 offset][u8 old][u8 new]`)
3. Filters records to EF section using `eventFlagsOffset` from metadata
4. Extracts bit-level transitions correlated with known game events

**Report:** Flag transitions correlated with inventory/game-state changes per timeline entry.

### 5. `scripts/verification/batch_case_verification.py` — Case-Based Verification

Runs case-based hypothesis testing on specific flags (defense + challenge phases).

```bash
python scripts/verification/batch_case_verification.py
```

**Report:** Per-flag case status (proven/challenged/disproven).

---

## Discovery Entry Points

### 6. `scripts/discover_block_bases.py` — Block Base Discovery

Finds unknown block base offsets by comparing slots with known different progression.

**Method:** For a known flag, searches the entire EF section for byte positions where the expected bit is SET in slot 0 (progressed) and UNSET in slot 1 (early game).

### 7. `scripts/discover_bases_from_snapshots.py` — Snapshot-Based Discovery

Uses granular before/after snapshots to find exact flag byte locations and reverse-calculate block bases.

### 8. `scripts/capture_agent.py` — Capture Agent

Manages before/after snapshot capture, catalog maintenance, and serves as HTTP endpoint for elden-map webapp integration.

```bash
python scripts/capture_agent.py capture --phase before --flag-id 1044360040 --slot 0
python scripts/capture_agent.py serve --port 8765
python scripts/capture_agent.py status
```

### 9. `scripts/diff_precise_snapshots.py` — Precise Diff Tool

Diffs two save snapshots to find exact byte/bit-level changes in the EF section, with reverse flag ID calculation.

---

## Data Flow Summary

```
DISCOVERY                           VERIFICATION
─────────                           ────────────
capture_agent.py                    run_verification.py
    ↓ (snapshots)                       ↓
discover_block_bases.py             verify_captures.py (temporal pairs)
discover_bases_from_snapshots.py    verify_pickups.py  (all pickup formulas)
diff_precise_snapshots.py           verify_timeline.py (longitudinal)
    ↓                               batch_case_verification.py
    ↓                                   ↓
    └─── ground_truth_offsets.json ←────┘
              ↓
    src/generated/ground_truth.rs  (auto-generated Rust constants)
    crates/wasm-event-flags/       (shared WASM detection)
```

---

## Infrastructure Modules

| Module | Role |
|--------|------|
| `verification/__init__.py` | Package exports, architecture docstring |
| `verification/constants.py` | Save file structure (slot offsets, sizes) |
| `verification/ground_truth_loader.py` | Read ground_truth_offsets.json, calculate offsets |
| `verification/utils.py` | Unified API: read_slot_data, detect_event_flags_start, check_flag |
| `verification/save_parser.py` | Full BND4 parser with character context extraction |
| `verification/calibration.py` | Dynamic tile base calibration per-save |
| `verification/diff_analyzer.py` | Before/after save comparison |
| `verification/verification_data.py` | Data structures for verification results |
