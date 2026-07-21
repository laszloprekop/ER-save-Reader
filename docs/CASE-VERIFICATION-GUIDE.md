# Case-Based Verification System: End-to-End Guide

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: STABLE METHODOLOGY, era-mixed examples.** The case model (defend/challenge a hypothesis until it survives) is current and maps onto the Status Ladder. Tooling (`batch_case_verification.py` et al.) and worked offsets are the pre-reset Python lab.
> - **Claims**: a rigorous case-based process for accepting/rejecting a flag hypothesis.
> - **Evidence**: the method; examples used period-specific saves.
> - **Methodology**: maps onto the Status Ladder (CONTEXT.md → *Status Ladder*, *Attributed Transition*); the live implementation is the knowledge pipeline.
> - **Obsolete**: the `.py` entry points (removed in step 5, `docs/archive/PYTHON-LAB.md`) and any hardcoded offsets in examples; resolve positions per save.

## Overview

The Case-Based Verification System is a rigorous methodology for discovering and verifying event flag offsets in Elden Ring save files. It treats each flag hypothesis as a **case** that must survive multiple rounds of evidence gathering (defense) and disproof attempts (challenge) before being accepted.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         CASE-BASED VERIFICATION FLOW                        │
│                                                                             │
│   EVIDENCE          HYPOTHESIS         DEFENSE         CHALLENGE           │
│   SOURCES              ↓                  ↓                ↓               │
│      │            ┌─────────┐       ┌──────────┐     ┌───────────┐         │
│      ├─Inventory──┤  CREATE ├──────►│  GATHER  ├────►│   ATTACK  │         │
│      │            │  CASE   │       │ EVIDENCE │     │HYPOTHESIS │         │
│      ├─Manual─────┤         │       └────┬─────┘     └─────┬─────┘         │
│      │            └─────────┘            │                 │               │
│      └─Confirmed                         ▼                 ▼               │
│        Flags              ┌──────────────────────────────────┐             │
│                           │     EVALUATE & ITERATE           │             │
│                           │  ┌─────────┐    ┌──────────┐    │             │
│                           │  │VERIFIED │ or │ REJECTED │    │             │
│                           │  └─────────┘    └──────────┘    │             │
│                           └──────────────────────────────────┘             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Core Concepts

### 1. The Evidence Triangle

Every verification case draws from three evidence sources:

```
                      INVENTORY
                    (Ground Truth)
                         ▲
                        /│\
                       / │ \
                      /  │  \
                     /   │   \
                    /    │    \
        MANUAL LOGS ◄────┴────► CONFIRMED FLAGS
       (Human Verified)        (Formula Verified)
```

| Source | What It Provides | Confidence Weight |
|--------|------------------|-------------------|
| **Inventory** | If item IS in inventory, its flag MUST be set somewhere | 0.30 |
| **Manual Logs** | User-confirmed completions from gameplay | 0.20 |
| **Confirmed Flags** | Already-verified flags using known formulas | 0.10 |

### 2. Defense vs Challenge

The system uses an adversarial approach:

| Phase | Goal | Question Asked |
|-------|------|----------------|
| **DEFENSE** | Gather supporting evidence | "What evidence confirms this hypothesis?" |
| **CHALLENGE** | Attempt to disprove | "What would prove this hypothesis WRONG?" |

A case must survive at least 2 challenges to be considered verified.

### 3. Confidence Scoring

Each evidence piece contributes to overall confidence:

```python
CONFIDENCE_WEIGHTS = {
    "inventory_present": 0.30,      # Item in inventory
    "flag_detected": 0.25,          # Flag bit is set at offset
    "manual_completion": 0.20,      # User marked complete
    "cross_slot_differential": 0.15, # Different across slots
    "chain_anchor": 0.10,           # Connected to verified chain
    "temporal_consistency": 0.10,   # Before/after matches
    "formula_consistency": 0.05,    # Follows block pattern
}
```

Thresholds:
- **Verified**: confidence ≥ 0.85
- **High Confidence**: confidence ≥ 0.70
- **Partial**: confidence ≥ 0.50

---

## End-to-End Workflow

### Step 1: Create a Hypothesis

Every case starts with a hypothesis about where a flag is stored:

```python
from case_manager import CaseManager, FlagHypothesis

manager = CaseManager()

# Hypothesis: Flag 520000 is at byte offset 1341, bit 7
case = manager.create_case(
    flag_id=520000,
    item_name="Lhutel the Headless",
    category="spirit_ash",
    item_id=258000,  # Game's internal item ID
    hypothesis=FlagHypothesis(
        byte_offset=1341,
        bit_position=7,
        implied_base=1341,
        block_start=520000,
    )
)
```

**How the hypothesis is formed:**
```
For block-based flags:
  byte_offset = base_offset + (flag_id - block_start) // 8
  bit_position = 7 - (flag_id % 8)

Example for flag 520000 with base 1341:
  byte_offset = 1341 + (520000 - 520000) // 8 = 1341
  bit_position = 7 - (520000 % 8) = 7 - 0 = 7
```

### Step 2: Defense Phase - Gather Evidence

The defense phase collects evidence that SUPPORTS the hypothesis.

#### Defense Method 1: Inventory Presence Check

Checks if item presence matches flag state across all slots.

```
For each slot in save file:
  1. Check if item is in inventory
  2. Check if flag bit is set at hypothesis offset
  3. If item_present == flag_set: SUPPORTS hypothesis
  4. If item_present != flag_set: OPPOSES hypothesis
```

**Example Output:**
```
Slot 0: Item present, flag set     → +0.30 confidence
Slot 1: Item absent, flag unset    → +0.30 confidence
Slot 2: Item absent, flag unset    → +0.30 confidence
```

#### Defense Method 2: Inventory Differential

Compares slots where item IS present vs ABSENT.

```
Given:
  - Slot 0 has item (mid-game character)
  - Slot 1 lacks item (early-game character)

Check:
  - Flag should be SET in Slot 0
  - Flag should be UNSET in Slot 1

If S0_bit=1 AND S1_bit=0: Strong evidence (+0.15)
```

**Visual:**
```
           Slot 0 (has item)    Slot 1 (no item)
           ─────────────────    ────────────────
Byte 1341: 0xFF (bit 7 = 1)     0x00 (bit 7 = 0)
           ▲                    ▲
           └── FLAG SET         └── FLAG UNSET

           ✓ Differential confirmed!
```

#### Defense Method 3: Cross-Save Validation

Tests the same hypothesis against multiple save files.

```
For each save file:
  1. Load event flags section
  2. Check item presence in inventory
  3. Check flag state at hypothesis offset
  4. Record match/mismatch

If consistent across saves: +0.10 confidence per save
```

#### Defense Method 4: Chain Anchor Verification

Verifies that related flags are also set appropriately.

```
Example: Spirit Ash from catacomb

  Related anchors:
  - Catacomb boss defeat flag (30xx0800)
  - Catacomb discovery flag

  If item present AND anchors set: +0.10 confidence
  If item present BUT anchors unset: Suspicious
```

#### Defense Method 5: Temporal Snapshot

Uses before/after save captures to verify flag transitions.

```
Before snapshot: Player hasn't collected item
After snapshot:  Player collected item

Expected:
  Before: bit = 0 (flag unset)
  After:  bit = 1 (flag set)

If 0→1 transition observed: +0.10 confidence
```

### Step 3: Challenge Phase - Attempt Disproof

The challenge phase tries to DISPROVE the hypothesis.

#### Challenge 1: Padding Detection

Checks if the offset lands in a 0xFF padding region.

```
For each slot:
  Read byte at hypothesis offset

If ALL slots have 0xFF at offset:
  → This is PADDING, not real data
  → FAILS challenge (hypothesis rejected)

If ANY slot has non-0xFF value:
  → SURVIVES challenge
```

**Example of padding gap:**
```
Offset 1367 (flag 520210):
  Slot 0: 0xFF  ─┐
  Slot 1: 0xFF   │── All 0xFF = PADDING
  Slot 2: 0xFF   │
  Slot 3: 0xFF   │
  Slot 4: 0xFF  ─┘

  → Challenge FAILED: This offset is unusable
```

#### Challenge 2: False Positive Analysis

Calculates the mismatch rate between item presence and flag state.

```
For each slot:
  item_present = check_inventory(item_id)
  flag_set = check_bit(offset, bit)

  if item_present != flag_set:
    mismatches += 1

false_positive_rate = mismatches / total_slots

If rate > 20%: FAILS challenge
If rate ≤ 20%: SURVIVES challenge
```

#### Challenge 3: Alternative Base Search

Searches for a different base offset that explains the data better.

```
current_base = hypothesis.implied_base
current_matches = count_matches(current_base)

for test_base in range(current_base - 100, current_base + 100):
  matches = count_matches(test_base)
  if matches > best_matches:
    best_base = test_base

If better base found with 20% more matches:
  → FAILS challenge (wrong base)
  → Suggests alternative hypothesis
```

#### Challenge 4: Bit Collision Check

Verifies no two known flags map to the same location.

```
For each known_flag in database:
  if known_flag != case.flag_id:
    other_offset, other_bit = calculate_offset(known_flag)
    if (other_offset, other_bit) == (hypothesis.offset, hypothesis.bit):
      → COLLISION DETECTED
      → FAILS challenge
```

#### Challenge 5: Block Boundary Check

Ensures the offset falls within valid block range.

```
block_size_bytes = 125  # ~1000 flags / 8
expected_start = implied_base
expected_end = implied_base + block_size_bytes

If offset outside [expected_start, expected_end]:
  → Warning (but not fatal)

If offset outside event_flags section:
  → FAILS challenge
```

### Step 4: Evaluate Results

After defense and challenge phases, the case is evaluated:

```python
# Count results
failed_challenges = [c for c in case.challenges if c.disproves_hypothesis]
supporting_evidence = [e for e in case.evidence if e.supports_hypothesis]

# Determine status
if failed_challenges:
    case.status = REJECTED
elif case.confidence >= 0.85:
    case.status = VERIFIED
elif case.confidence >= 0.70:
    case.status = PARTIAL
else:
    case.status = INCONCLUSIVE
```

### Step 5: Iterate if Needed

For cases that are not definitively verified or rejected:

```
Iteration 2:
  - Add more evidence sources (different saves)
  - Run additional challenges
  - Re-evaluate confidence

Repeat until:
  - Case is VERIFIED (confidence ≥ 0.85, all challenges survived)
  - Case is REJECTED (any challenge failed)
  - Maximum iterations reached
```

---

## Practical Example: Verifying Block 520000

### Initial State

```
Known:
  - Block 520000 contains Spirit Ash and Talisman acquisition flags
  - No formula exists in ground_truth_offsets.json
  - 15 items with 520xxx flags exist in the game

Goal: Discover the base offset for block 520000
```

### Step 1: Inventory Analysis

```
Slot 0 (mid-game): 18 items with 520xxx flags present
Slot 1 (early-game): 3 items with 520xxx flags present

Differential: 15 items in S0 but not S1
  → These are our test cases
```

### Step 2: Discover Base Offset

```python
# Search for base where differential items show S0=1, S1=0
for test_base in range(0, 10000):
    matches = 0
    for flag_id, item_id, name in differential_items:
        offset = test_base + (flag_id - 520000) // 8
        bit = 7 - (flag_id % 8)

        s0_bit = (ef_s0[offset] >> bit) & 1
        s1_bit = (ef_s1[offset] >> bit) & 1

        if s0_bit == 1 and s1_bit == 0:
            matches += 1

    if matches > best_matches:
        best_base = test_base

# Result: base 1341 has 12/15 matches (80%)
```

### Step 3: Create and Verify Cases

```bash
python case_cli.py batch --block 520000 --base 1341 --save-cases
```

### Step 4: Results

```
VERIFIED (12):
  ✓ 520000: Lhutel the Headless
  ✓ 520030: Assassin's Crimson Dagger
  ✓ 520040: Banished Knight Engvall
  ✓ 520050: Twinsage Sorcerer Ashes
  ✓ 520090: Bloodhound Knight Floh
  ✓ 520110: Perfumer Tricia
  ✓ 520300: Viridian Amber Medallion
  ✓ 520310: Spelldrake Talisman
  ✓ 520350: Blue Dancer Charm
  ✓ 520370: Cerulean Amber Medallion
  ✓ 520390: Kindred of Rot's Exultation
  ✓ 520480: Godskin Swaddling Cloth

REJECTED (3):
  ✗ 520210: Assassin's Cerulean Dagger (offset 1367 = padding)
  ✗ 520330: Flamedrake Talisman (offset 1382 = padding)
  ✗ 520450: Gold Scarab (offset 1397 = padding)
```

### Step 5: Record Discovery

```json
// Added to ground_truth_offsets.json
"520000": {
  "block_start": 520000,
  "base_offset": 1341,
  "block_size": 500,
  "status": "partial",
  "verified_flags": [520000, 520030, 520040, ...],
  "padding_gaps": [520210, 520330, 520450],
  "notes": "Block has 0xFF gaps at offsets +26, +41, +56"
}
```

---

## CLI Reference

### Create a Case
```bash
python case_cli.py create \
  --flag 520000 \
  --name "Lhutel the Headless" \
  --category spirit_ash \
  --item-id 258000 \
  --offset 1341 \
  --bit 7 \
  --base 1341
```

### Verify a Case
```bash
python case_cli.py verify \
  --case-id 520000 \
  --save /path/to/save.sl2 \
  --slots-with 0 \
  --slots-without 1,2,3,4 \
  --iterations 2
```

### Batch Verify a Block
```bash
python case_cli.py batch \
  --block 520000 \
  --base 1341 \
  --save-cases
```

### List All Cases
```bash
python case_cli.py list
```

### Show Case Report
```bash
python case_cli.py report --case-id 520000
```

### Discover Unknown Base
```bash
python case_cli.py discover \
  --block 530000 \
  --items "530000:123456:ItemA,530010:234567:ItemB" \
  --search-start 0 \
  --search-end 100000
```

---

## File Structure

```
scripts/verification/
├── case_manager.py              # Core case system
├── case_cli.py                  # Command-line interface
├── case_analysis.py             # Coverage, blindspot, and base tracking
├── blindspot_analysis.py        # CLI for blindspot detection
├── batch_case_verification.py   # Batch runner
├── cases/                       # Saved case files
│   ├── 520000_20260201.json
│   ├── 520030_20260201.json
│   └── ...
└── ...

docs/
├── CASE-VERIFICATION-GUIDE.md   # This guide (authoritative)
└── archive/
    ├── CASE-BASED-VERIFICATION.md   # Superseded methodology doc
    └── CONFIDENCE-NORMALIZATION.md  # Merged into this guide
```

---

## Confidence Normalization

To prevent score inflation from repetitive evidence:

- **Diminishing returns**: Each subsequent piece of same-type evidence contributes 50% less: `contribution = base_weight * (0.5 ** count)`
- **Per-type caps**: `cross_save: 0.20`, `chain_anchor: 0.15`, `inventory_presence: 0.35`, `differential: 0.25`
- **Match rate normalization**: Chain anchor confidence uses match rate (not count): `supports = match_rate >= 0.7`

This ensures confidence reflects evidence diversity, not just volume.

### Blindspot Analysis

Use `scripts/verification/blindspot_analysis.py` to detect:
- Data vs padding regions within blocks
- Coverage percentages per block
- Unknown data regions (non-0xFF/0x00 bytes not belonging to any known block)

---

## Best Practices

### 1. Always Use Multiple Evidence Sources
```
Don't rely on a single check. Combine:
  - Inventory presence (ground truth)
  - Multi-slot differential
  - Cross-save validation
```

### 2. Challenge Before Accepting
```
A case without challenges is untested.
Always run:
  - Padding detection
  - False positive check
  - Alternative base search
```

### 3. Document Rejections
```
Rejected cases are valuable data:
  - They identify padding gaps
  - They reveal block structure
  - They prevent future false positives
```

### 4. Track Source Attribution
```
Every piece of evidence should record:
  - Save file path
  - Slot index
  - Method used
  - Timestamp
```

### 5. Iterate When Uncertain
```
If confidence is between 0.50 and 0.85:
  - Add more evidence sources
  - Run additional challenges
  - Test with different saves
```

---

## Summary

The Case-Based Verification System provides a rigorous, repeatable methodology for discovering event flag offsets:

1. **Create** a hypothesis about flag location
2. **Defend** with multiple evidence sources
3. **Challenge** with disproof attempts
4. **Evaluate** based on confidence and challenge survival
5. **Iterate** until verified or rejected
6. **Record** results to ground truth database

This approach ensures that verified flags are reliable and that rejected hypotheses are documented for future reference.
