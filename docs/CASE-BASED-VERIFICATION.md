# Case-Based Verification System

## Overview

A **Case** is a structured hypothesis about a flag's location that must survive multiple rounds of defense and challenge before being accepted as verified. This methodology formalizes the evidence-based discovery approach used successfully for the 520000 block.

## Core Principles

### 1. Evidence Triangle
Every case must be built from at least two of these three evidence sources:

```
                    INVENTORY
                   (Ground Truth)
                        ▲
                       / \
                      /   \
                     /     \
          MANUAL LOGS ◄─────► CONFIRMED FLAGS
         (Human Verified)    (Formula Verified)
```

- **Inventory**: If item IS in inventory, its acquisition flag MUST be set somewhere
- **Manual Logs**: User-confirmed completions from gameplay sessions
- **Confirmed Flags**: Flags already verified via established formulas

### 2. Defense vs Challenge Phases

Each case goes through alternating phases:

| Phase | Goal | Actions |
|-------|------|---------|
| **DEFEND** | Find supporting evidence | Multi-slot differential, cross-save validation, formula consistency |
| **CHALLENGE** | Try to disprove | Edge cases, alternative explanations, padding detection, false positive checks |

A case must survive at least 2 challenge rounds to be considered verified.

### 3. Confidence Scoring

```python
CONFIDENCE_WEIGHTS = {
    "inventory_present": 0.30,      # Item in inventory
    "flag_detected": 0.25,          # Flag bit is set at calculated offset
    "manual_completion": 0.20,      # User marked as complete
    "cross_slot_differential": 0.15, # Different in slot where item absent
    "chain_anchor": 0.10,           # Connected to verified flag chain
    "temporal_consistency": 0.10,   # Before/after snapshot matches
    "formula_consistency": 0.05,    # Follows block formula pattern
}

# Minimum confidence for each status
THRESHOLDS = {
    "verified": 0.85,
    "high_confidence": 0.70,
    "medium_confidence": 0.50,
    "low_confidence": 0.30,
    "unverified": 0.0,
}
```

## Case Structure

```python
@dataclass
class VerificationCase:
    """A structured hypothesis about a flag's location."""

    # Identity
    case_id: str                    # Unique identifier
    flag_id: int                    # Target flag ID
    item_name: str                  # Associated item/event name
    category: str                   # "spirit_ash", "talisman", "grace", etc.

    # Hypothesis
    hypothesis: FlagHypothesis      # Proposed offset/bit location
    block_start: int                # Block this flag belongs to
    formula_type: str               # "block", "tile", "dungeon"

    # Evidence (Defense)
    evidence: List[CaseEvidence]    # All collected evidence
    supporting_sources: List[EvidenceSource]  # Save files, slots, methods

    # Challenges (Attack)
    challenges: List[CaseChallenge] # Attempted disproofs
    surviving_challenges: int       # How many challenges survived

    # Status
    status: CaseStatus              # OPEN, DEFENDING, CHALLENGING, VERIFIED, REJECTED
    confidence: float               # Aggregate confidence score
    iterations: int                 # Defense/challenge cycles completed

    # Metadata
    created_at: datetime
    last_updated: datetime
    notes: List[str]


@dataclass
class CaseEvidence:
    """A single piece of evidence for/against a case."""

    evidence_type: str              # "inventory", "flag_state", "differential", "temporal"
    source: EvidenceSource          # Where this came from
    supports_hypothesis: bool       # True = defense, False = challenge
    confidence_contribution: float  # How much this adds to confidence

    # Details
    byte_offset: int
    bit_position: int
    observed_value: int             # Actual byte value
    expected_value: Optional[int]   # What we expected

    # Context
    slot_context: Dict[str, Any]    # Character progression, inventory, etc.
    notes: str


@dataclass
class CaseChallenge:
    """An attempt to disprove a case."""

    challenge_type: str             # "padding_check", "false_positive", "alternative_base"
    description: str
    result: str                     # "survived", "failed", "inconclusive"

    # What was tested
    test_method: str
    test_data: Dict[str, Any]

    # Outcome
    disproves_hypothesis: bool
    alternative_hypothesis: Optional[FlagHypothesis]
    notes: str
```

## Pre-Verification: Schema-Based Filtering

**Before creating cases**, probe the block to identify trackable vs untrackable flags:

```python
from scripts.verification.flag_schema import BlockSchema

# Create schema for the target block
schema = BlockSchema(block_start=520000, base_offset=1341)
schema.load_flags_from_extracted('scripts/extracted_event_flags.json')

# Generate allocation bitmap
bitmap = schema.probe_allocation(save_path, slots=[0,1,2,3,4])

# Only create cases for trackable flags
for flag_id in candidate_flags:
    if bitmap.is_trackable(flag_id):
        # Safe to create verification case
        create_case(flag_id, ...)
    else:
        # Skip - flag is in sparse allocation gap
        log_untrackable(flag_id, reason="sparse_gap")
```

**Why this matters:**
- Some blocks have **sparse allocation** (not all flag IDs have memory allocated)
- Flags in sparse gaps show 0xFF in ALL slots (indistinguishable from padding)
- Creating cases for untrackable flags wastes effort and produces false results

**Related:** See `docs/EVENT-FLAG-GEOGRAPHY.md` section "Sparse Flag Allocation"

## Case Lifecycle

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           CASE LIFECYCLE                                 │
└─────────────────────────────────────────────────────────────────────────┘

  ┌──────────┐
  │  CREATE  │  Build initial hypothesis from evidence triangle
  └────┬─────┘
       │
       ▼
  ┌──────────┐
  │  DEFEND  │  Gather supporting evidence
  │  Phase 1 │  - Multi-slot inventory differential
  └────┬─────┘  - Cross-save validation
       │        - Formula consistency check
       ▼
  ┌──────────┐
  │ CHALLENGE│  Try to disprove
  │  Phase 1 │  - Padding detection (0xFF check)
  └────┬─────┘  - False positive analysis
       │        - Alternative base search
       │
       ├───────────── FAILED ──────────► REJECTED
       │
       ▼ SURVIVED
  ┌──────────┐
  │  DEFEND  │  Strengthen with more evidence
  │  Phase 2 │  - Temporal snapshots (before/after)
  └────┬─────┘  - Chain anchor verification
       │        - Edge case testing
       ▼
  ┌──────────┐
  │ CHALLENGE│  More rigorous disproof attempts
  │  Phase 2 │  - Different save file test
  └────┬─────┘  - Bit collision check
       │        - Block boundary verification
       │
       ├───────────── FAILED ──────────► REJECTED
       │
       ▼ SURVIVED (confidence >= 0.85)
  ┌──────────┐
  │ VERIFIED │  Record in ground_truth_offsets.json
  └──────────┘
```

## Defense Techniques

### 1. Multi-Slot Inventory Differential
```python
def defend_with_inventory_differential(case: VerificationCase) -> CaseEvidence:
    """
    Compare slots where item IS present vs ABSENT.

    If item in S0 but not S1:
      - Flag should be SET in S0 at hypothesis offset
      - Flag should be UNSET in S1 at hypothesis offset
    """
    # Load both slots
    ef_s0 = load_event_flags(save_path, slot_with_item)
    ef_s1 = load_event_flags(save_path, slot_without_item)

    # Check hypothesis
    offset = case.hypothesis.byte_offset
    bit = case.hypothesis.bit_position

    s0_set = (ef_s0[offset] >> bit) & 1
    s1_set = (ef_s1[offset] >> bit) & 1

    supports = (s0_set == 1 and s1_set == 0)

    return CaseEvidence(
        evidence_type="differential",
        supports_hypothesis=supports,
        confidence_contribution=0.15 if supports else -0.20,
        # ...
    )
```

### 2. Cross-Save Validation
```python
def defend_with_cross_save(case: VerificationCase) -> List[CaseEvidence]:
    """
    Test hypothesis against multiple save files.

    Same item in different saves should have flag at same offset
    (after calibration for variable EF start).
    """
    evidence = []
    for save_file in get_available_saves():
        # Calibrate for this save's EF offset
        calibration = CalibrationService.calibrate(save_file, slot=0)
        adjusted_offset = adjust_offset_for_save(case.hypothesis, calibration)

        # Check if item present and flag matches
        item_present = check_inventory(save_file, case.item_id)
        flag_set = check_flag_at_offset(save_file, adjusted_offset, case.hypothesis.bit)

        supports = (item_present == flag_set)
        evidence.append(CaseEvidence(
            evidence_type="cross_save",
            source=EvidenceSource(save_file=save_file, ...),
            supports_hypothesis=supports,
            # ...
        ))

    return evidence
```

### 3. Temporal Snapshot Validation
```python
def defend_with_temporal_snapshot(case: VerificationCase) -> CaseEvidence:
    """
    Use before/after snapshots to verify flag changes with action.

    If player collected item between snapshots:
      - Flag should be UNSET in "before"
      - Flag should be SET in "after"
    """
    before_ef = load_event_flags(before_snapshot)
    after_ef = load_event_flags(after_snapshot)

    offset = case.hypothesis.byte_offset
    bit = case.hypothesis.bit_position

    before_set = (before_ef[offset] >> bit) & 1
    after_set = (after_ef[offset] >> bit) & 1

    # Expected: 0 → 1 transition
    supports = (before_set == 0 and after_set == 1)

    return CaseEvidence(
        evidence_type="temporal",
        supports_hypothesis=supports,
        confidence_contribution=0.10 if supports else -0.15,
        # ...
    )
```

### 4. Chain Anchor Verification
```python
def defend_with_chain_anchor(case: VerificationCase) -> CaseEvidence:
    """
    Verify flag is connected to already-verified flags.

    Example: Spirit Ash from catacomb should have:
      - Catacomb boss defeat flag (30xx0800) also set
      - Catacomb discovery flag also set
    """
    anchors = get_related_flags(case.flag_id, case.category)
    anchor_matches = 0

    for anchor in anchors:
        anchor_result = calculate_offset(anchor.flag_id)
        if anchor_result:
            anchor_set = check_flag_at_offset(ef_data, *anchor_result)
            if anchor_set == case.item_present:
                anchor_matches += 1

    supports = anchor_matches >= len(anchors) * 0.8

    return CaseEvidence(
        evidence_type="chain_anchor",
        supports_hypothesis=supports,
        confidence_contribution=0.10 if supports else -0.05,
        # ...
    )
```

## Challenge Techniques

### 1. Padding Detection (0xFF Check)
```python
def challenge_padding_detection(case: VerificationCase) -> CaseChallenge:
    """
    Check if hypothesis offset lands in 0xFF padding region.

    If byte is 0xFF in ALL slots, it's padding - not real data.
    """
    offset = case.hypothesis.byte_offset

    all_ff = True
    for slot in range(5):
        ef_data = load_event_flags(save_path, slot)
        if ef_data[offset] != 0xFF:
            all_ff = False
            break

    if all_ff:
        return CaseChallenge(
            challenge_type="padding_check",
            result="failed",
            disproves_hypothesis=True,
            notes=f"Offset {offset} is 0xFF in all slots - padding region"
        )

    return CaseChallenge(
        challenge_type="padding_check",
        result="survived",
        disproves_hypothesis=False,
    )
```

### 2. False Positive Analysis
```python
def challenge_false_positive(case: VerificationCase) -> CaseChallenge:
    """
    Check if match is coincidental rather than causal.

    A false positive occurs when:
      - Item absent but flag set (other flag at same location)
      - Item present but flag unset (wrong location)
    """
    false_positives = 0
    test_count = 0

    for save_file in get_available_saves():
        for slot in range(5):
            item_present = check_inventory(save_file, slot, case.item_id)
            flag_set = check_flag(save_file, slot, case.hypothesis)

            if item_present != flag_set:
                false_positives += 1
            test_count += 1

    false_positive_rate = false_positives / test_count

    if false_positive_rate > 0.20:  # More than 20% mismatch
        return CaseChallenge(
            challenge_type="false_positive",
            result="failed",
            disproves_hypothesis=True,
            notes=f"False positive rate {false_positive_rate:.1%} exceeds threshold"
        )

    return CaseChallenge(
        challenge_type="false_positive",
        result="survived",
        notes=f"False positive rate {false_positive_rate:.1%} acceptable"
    )
```

### 3. Alternative Base Search
```python
def challenge_alternative_base(case: VerificationCase) -> CaseChallenge:
    """
    Search for a different base offset that explains the data better.

    If another base has higher match rate, hypothesis is weakened.
    """
    current_base = case.hypothesis.implied_base
    current_matches = count_matches_at_base(current_base, case.related_flags)

    # Search nearby bases
    best_alternative = None
    best_matches = current_matches

    for test_base in range(current_base - 100, current_base + 100):
        matches = count_matches_at_base(test_base, case.related_flags)
        if matches > best_matches:
            best_matches = matches
            best_alternative = test_base

    if best_alternative and best_matches > current_matches * 1.2:
        return CaseChallenge(
            challenge_type="alternative_base",
            result="failed",
            disproves_hypothesis=True,
            alternative_hypothesis=FlagHypothesis(base_offset=best_alternative, ...),
            notes=f"Base {best_alternative} has {best_matches} matches vs {current_matches}"
        )

    return CaseChallenge(
        challenge_type="alternative_base",
        result="survived",
    )
```

### 4. Bit Collision Check
```python
def challenge_bit_collision(case: VerificationCase) -> CaseChallenge:
    """
    Check if multiple flags share the same byte/bit location.

    If two different items map to same location, one is wrong.
    """
    offset = case.hypothesis.byte_offset
    bit = case.hypothesis.bit_position

    colliding_flags = []
    for other_flag in get_all_known_flags():
        if other_flag.flag_id == case.flag_id:
            continue
        other_result = calculate_offset(other_flag.flag_id)
        if other_result == (offset, bit):
            colliding_flags.append(other_flag)

    if colliding_flags:
        return CaseChallenge(
            challenge_type="bit_collision",
            result="failed" if len(colliding_flags) > 0 else "survived",
            notes=f"Collides with flags: {[f.flag_id for f in colliding_flags]}"
        )

    return CaseChallenge(
        challenge_type="bit_collision",
        result="survived",
    )
```

## Best Practices

### 1. Source Attribution
Always record WHERE evidence came from:
```python
source = EvidenceSource(
    save_file="ER0000-backup-2026-01-11.sl2",
    slot_index=0,
    evidence_type="inventory_differential",
    timestamp="2026-01-31T10:30:00",
    method="verify_520_both_set.py"
)
```

### 2. Confidence Decay
Old evidence loses confidence over time or with game updates:
```python
def adjust_confidence_for_age(evidence: CaseEvidence) -> float:
    age_days = (datetime.now() - evidence.timestamp).days
    decay_factor = 0.99 ** age_days  # 1% decay per day
    return evidence.confidence_contribution * decay_factor
```

### 3. Contradiction Documentation
When evidence contradicts, document it for later analysis:
```python
if new_evidence.supports_hypothesis != existing_evidence.supports_hypothesis:
    case.notes.append(
        f"CONTRADICTION: {new_evidence.source} says "
        f"{'supports' if new_evidence.supports_hypothesis else 'rejects'} "
        f"but {existing_evidence.source} says opposite"
    )
```

### 4. Minimum Evidence Requirements
A case cannot be verified without meeting minimums:
```python
MIN_REQUIREMENTS = {
    "total_evidence_pieces": 3,
    "unique_sources": 2,       # Different save files or methods
    "defense_phases_passed": 2,
    "challenges_survived": 2,
    "confidence_threshold": 0.85,
}
```

### 5. Edge Case Testing
Always test boundary conditions:
- First flag in block (flag_id % 8 == 0)
- Last flag in block
- Flags near padding boundaries
- Flags with low/high bit positions

## Integration with Existing Tools

### From Evidence-Based Discovery (520000 example)
```python
# 1. Start with inventory evidence
items_in_s0 = extract_inventory(slot_0)
items_in_s1 = extract_inventory(slot_1)
differential_items = items_in_s0 - items_in_s1

# 2. Create cases for each differential item
for item in differential_items:
    case = VerificationCase(
        flag_id=item.expected_flag,
        item_name=item.name,
        hypothesis=FlagHypothesis(
            byte_offset=proposed_base + (item.expected_flag - block_start) // 8,
            bit_position=7 - (item.expected_flag % 8)
        )
    )

    # 3. Run defense/challenge cycles
    case = defend_phase_1(case)
    case = challenge_phase_1(case)

    if case.status != CaseStatus.REJECTED:
        case = defend_phase_2(case)
        case = challenge_phase_2(case)

    # 4. Record result
    if case.confidence >= 0.85:
        update_ground_truth(case)
```

### With SnapshotTestRunner
```python
# Use existing snapshot infrastructure
runner = SnapshotTestRunner()

for case in open_cases:
    # Find relevant snapshot pairs
    pairs = runner.find_pairs_for_flag(case.flag_id)

    for pair in pairs:
        result = runner.verify_flag(
            pair.before_path,
            pair.after_path,
            case.flag_id,
            case.hypothesis
        )

        case.evidence.append(CaseEvidence(
            evidence_type="temporal",
            source=EvidenceSource(save_file=pair.after_path, ...),
            supports_hypothesis=result.matches,
            confidence_contribution=0.10 if result.matches else -0.15
        ))
```

## Example: 520000 Block Discovery as Case

```
CASE: 520000_block_base
════════════════════════════════════════════════════════════════════════

HYPOTHESIS:
  Block 520000 uses base_offset = 1341
  Formula: byte = 1341 + (flag_id - 520000) // 8

DEFENSE PHASE 1:
  ✓ Inventory differential: 15 items in S0, absent in S1
  ✓ Cross-slot validation: 12/15 flags show expected differential
  ✓ Formula consistency: Follows standard block formula pattern

CHALLENGE PHASE 1:
  ✓ SURVIVED: Padding detection - data bytes found (not all 0xFF)
  ✓ SURVIVED: Alternative base search - 65000 has 0 matches
  ⚠ PARTIAL: 3 flags (520210, 520330, 520450) land in 0xFF gaps

DEFENSE PHASE 2:
  ✓ Chain anchor: Related dungeon boss flags correlate
  ✓ Block boundary: First 7 bytes show clean data

CHALLENGE PHASE 2:
  ✓ SURVIVED: False positive rate = 0% (12/12 testable flags match)
  ✓ SURVIVED: No bit collisions with other blocks

RESULT:
  Status: VERIFIED (partial)
  Confidence: 0.80
  Notes: Block has internal 0xFF gaps at offsets +7-10, +23-26, +40-41, +56-57
         12/15 key flags verified, 3 land in padding gaps
```

## Next Steps

1. **Implement CaseManager**: Orchestrates case lifecycle
2. **Build Case Database**: Track all open/verified/rejected cases
3. **Create CLI Tools**: `create-case`, `defend-case`, `challenge-case`
4. **Integration**: Hook into existing verification scripts
5. **Reporting**: Generate verification reports with case status
