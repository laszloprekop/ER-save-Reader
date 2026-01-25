# Discovery/Verification Methodology

A comprehensive guide to empirically discovering and verifying event flag offsets in Elden Ring save files.

---

## Prerequisites

### Required Resources

1. **Game Save Files** - Multiple character slots with varied progression
   - Slot 0: Mid-game (Confessor) - maximum verified progression
   - Slot 1: Early-game (Wretch) - negative control for differential analysis
   - Slots 2-4: Controlled variants for specific testing

2. **Decompiled Game Files** (single source of truth)
   - `ItemLotParam_map.param.xml` - World pickup flag IDs
   - `ShopLineupParam.param.xml` - Shop event flags
   - `common.emevd.js` - Event script logic
   - `openmap.eventflagalloclist` - Tile flag allocation
   - `legacymap.eventflagalloclist` - Dungeon flag allocation

3. **Ground Truth Database**
   - `ground_truth_offsets.json` - Verified offsets (single source of truth)
   - `src/generated/ground_truth.rs` - Auto-generated Rust code

### Formula Types

| Type | Flag Format | Example | Formula |
|------|-------------|---------|---------|
| Block | 5-6 digits | 71800 | `base + (flag_id - block_start) // 8` |
| Tile | 10 digits (10XXYYZZZZ) | 1043500010 | `base + tile_offset + local_id // 8` |
| Dungeon | 8 digits (AASSZZZZ) | 30020800 | `base + section * 1125 + local_id // 8` |

---

## Phase 1: Target Selection

### Priority Table

| Priority | Category | Reason |
|----------|----------|--------|
| 1 | Anchor flags | Establish base offset for entire block |
| 2 | Related flag chains | Verify consistency within block |
| 3 | Cross-block validation | Detect false positives via differential |
| 4 | Edge cases | Boundary flags, sub-blocks |

### Identifying Related Flag Chains

Before investigating a single flag, identify ALL related flags:

```
Example: Grace block 71000 (Stormveil)
- 71000: Godrick the Grafted
- 71001: Margit, the Fell Omen
- 71002: Castleward Tunnel
- 71003: Gateside Chamber
- ...
- 71008: Main Gate
```

Verification should match the EXPECTED pattern:
- Slot 0 (mid-game): Most graces SET
- Slot 1 (early-game): Few or no graces SET

---

## Phase 2: Evidence Collection

### Types of Evidence

1. **Positive Evidence** - Flag IS set when expected
   - Player has discovered the grace/item/boss
   - Bit reads as 1 (or 0 depending on convention)

2. **Negative Evidence** - Flag is NOT set when expected to be unset
   - Early-game character has NOT discovered the flag
   - Bit reads as 0 (or 1 depending on convention)

3. **Cross-Examination** - Differential between slots
   - Same offset, different slots
   - Expected SET in progressed slot, UNSET in early-game slot

### Evidence Strength Hierarchy

```
STRONGEST: Multi-slot differential with expected pattern
         ↓
STRONG:   Temporal diff (same slot, before/after action)
         ↓
MODERATE: Single slot match with inventory correlation
         ↓
WEAK:     Single flag match without corroboration
```

---

## Phase 3: Multi-Slot Differential Analysis (Gold Standard)

The **gold standard** for verification is multi-slot differential analysis:

```python
def verify_block_via_differential(block_base, flags_to_check, ef_data_s0, ef_data_s1):
    """
    For each flag:
      1. Calculate offset using candidate base
      2. Check if S0 (progressed) has flag SET
      3. Check if S1 (early-game) has flag UNSET
      4. A match requires BOTH conditions
    """
    results = []
    for flag_id, expected_name in flags_to_check:
        offset = block_base + (flag_id - block_start) // 8
        bit = 7 - (flag_id % 8)  # Block formula

        s0_set = (ef_data_s0[offset] >> bit) & 1
        s1_set = (ef_data_s1[offset] >> bit) & 1

        # Perfect differential: SET in S0, UNSET in S1
        is_valid = (s0_set == 1) and (s1_set == 0)
        results.append((flag_id, expected_name, s0_set, s1_set, is_valid))

    return results
```

### Interpreting Results

| S0 | S1 | Interpretation |
|----|----|----|
| SET | UNSET | Perfect - high confidence |
| SET | SET | Suspicious - may be wrong offset or both slots have flag |
| UNSET | UNSET | Neutral - can't confirm or deny |
| UNSET | SET | INVERTED - wrong base or 0xFF padding |

---

## Phase 4: 0xFF Padding Detection (Critical)

### The Problem

Event flag regions contain 0xFF padding bytes (all bits = 1). These will cause **false positives** because every flag in that region appears "SET".

### Detection Pattern

```python
def is_likely_false_positive(ef_data, offset, window=4):
    """
    Check if the byte and surrounding region are all 0xFF.
    If yes, this is likely padding, not real flag data.
    """
    start = max(0, offset - window)
    end = min(len(ef_data), offset + window + 1)
    region = ef_data[start:end]

    return all(b == 0xFF for b in region)
```

### When 0xFF Padding Causes Issues

1. **Inverted differential**: S1 (early-game) shows MORE flags than S0
2. **Unrealistic flag counts**: 500+ flags "set" in a region that should have few
3. **Block 75000, 77000**: Known to be 0xFF padded (disproven bases)

### Solution

Always check for 0xFF padding BEFORE trusting results. If a region is all 0xFF:
- Mark the base as "disproven" or "false_positive"
- Search for the real base elsewhere

---

## Phase 5: Confidence Levels

| Level | Requirements | Action |
|-------|--------------|--------|
| **Proven** | Multi-slot differential matches expected pattern (80%+), no 0xFF contamination | Record in ground_truth |
| **High** | Temporal diff confirmed + logical consistency | Record with notes |
| **Medium** | Single slot match with inventory correlation | Record as "candidate" |
| **Low** | Formula calculation only, no empirical validation | Mark as "calculated" |
| **Unverified** | No evidence | Mark as "unverified" |
| **Disproven** | Evidence contradicts formula | Mark as "disproven" |

---

## Phase 6: Corroboration Validation

After establishing confidence via multi-slot differential, use the corroboration system for additional validation.

### Dual-Formula Corroboration

When a flag has multiple formulas (tile + block), both should agree. If a player picks up an item:

1. **Tile flag** (10-digit): Records the world pickup location was looted
2. **Block flag** (5-digit): Records the item is now owned

Both flags should be SET or UNSET. Contradictions indicate formula errors.

See [CORROBORATION-SYSTEM.md](CORROBORATION-SYSTEM.md) for full methodology.

### Inseparable Evidence

Boss-grace pairs that cannot be set independently provide strong validation:

| Boss | Defeat Flag | Grace Flag | Grace Name |
|------|-------------|------------|------------|
| Godrick | 10000800 | 71010 | Godrick the Grafted |
| Rennala | 14000800 | 71140 | Rennala, Queen of the Full Moon |
| Morgott | 11000800 | 71110 | Morgott, the Omen King |
| Rykard | 16000800 | 71600 | Audience Pathway |
| Malenia | 15000800 | 71500 | Malenia, Goddess of Rot |
| Maliketh | 13000800 | 71300 | Beside the Great Bridge |

**Validation logic**: If boss defeat flag is SET but grace flag is UNSET (or vice versa), one of the formulas is wrong.

See [CORROBORATION-SYSTEM.md#inseparable-evidence-methodology](CORROBORATION-SYSTEM.md#inseparable-evidence-methodology-2026-01-21) for details.

---

## Phase 7: Recording Results

### Update ground_truth_offsets.json

```json
{
  "71000": {
    "block_start": 71000,
    "base_offset": 9315,
    "block_size": 100,
    "status": "verified",
    "notes": "Stormveil graces - verified via multi-slot differential. 8/9 graces SET in S0, 0/9 in S1."
  }
}
```

### Required Fields

- `base_offset`: The verified byte offset
- `status`: "verified" | "candidate" | "calculated" | "disproven" | "unverified"
- `notes`: MUST include:
  - Date of verification
  - Method used (multi-slot, temporal, etc.)
  - Match ratio (e.g., "8/9 graces SET")
  - Any caveats or special conditions

---

## Common Pitfalls

### 1. Bit Calculation Errors

**Block flags (5-6 digit)**: Use `flag_id`
```python
bit = 7 - (flag_id % 8)
```

**Tile/Dungeon flags**: Use `local_id`, NOT full flag_id!
```python
local_id = flag_id % 10000  # Extract ZZZZ portion
bit = 7 - (local_id % 8)
```

### 2. Sub-Block Lookup Failure

Some blocks have sub-ranges with different bases (e.g., 71600 within 71000):

```python
def get_block_base(flag_id):
    # Try 100-flag granularity first (sub-block)
    sub_block = (flag_id // 100) * 100
    if sub_block in BLOCK_BASES:
        return BLOCK_BASES[sub_block]

    # Fall back to 1000-flag granularity (main block)
    main_block = (flag_id // 1000) * 1000
    return BLOCK_BASES.get(main_block)
```

### 3. EF Start Detection (Never Hardcode!)

The event flags section starts at a **variable offset** within each slot. Never hardcode:

```python
# WRONG - will break on different saves
EF_START = 0x125A5

# CORRECT - use validation flags to detect
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

def detect_ef_start(slot_data):
    """
    Search for validation flags to find EF start.
    All validation flags MUST be SET to confirm correct detection.
    """
    for search_offset in range(len(slot_data) - 10000):
        all_match = True
        for flag_id, rel_offset, bit, name in VALIDATION_FLAGS:
            byte_val = slot_data[search_offset + rel_offset]
            if not ((byte_val >> bit) & 1):
                all_match = False
                break
        if all_match:
            return search_offset
    return None
```

### 4. Confusing Absolute vs Relative Offsets

- **Relative offset**: Offset within EF section (what ground_truth stores)
- **Absolute offset**: Offset within slot data (EF_start + relative)

Always clarify which you're using in verification scripts.

### 5. Formula vs Ground Truth Desync

`flag_formulas.py` contains OUTDATED values. Always use `ground_truth_offsets.json`:

| Block | flag_formulas.py | ground_truth.json | Winner |
|-------|------------------|-------------------|--------|
| 62000 | 1500 | 9359 | ground_truth |
| 65000 | 1875 | 37412 | ground_truth |
| 67000 | 3546 | 37411 | ground_truth |
| 71000 | 2625 | 9315 | ground_truth |

---

## Verification Checklist

Before marking a flag/block as "verified":

- [ ] Identified related flag chain (not just single flag)
- [ ] Multi-slot differential analysis performed
- [ ] Expected pattern observed (progressed slot > early slot)
- [ ] Checked for 0xFF padding contamination
- [ ] Bit calculation uses correct formula for flag type
- [ ] Sub-block lookup considered (if applicable)
- [ ] Results recorded in ground_truth_offsets.json
- [ ] Notes include date, method, and match ratio

---

## Quick Reference: Correct Formulas

### Block Flags (5-6 digit)
```python
block_start = (flag_id // 1000) * 1000  # or // 100 for sub-blocks
relative_flag = flag_id - block_start
byte_offset = BLOCK_BASES[block_start] + relative_flag // 8
bit = 7 - (flag_id % 8)
```

### Tile Flags (10-digit: 10XXYYZZZZ)
```python
row = int(str(flag_id)[2:4])
col = int(str(flag_id)[4:6])
local_id = int(str(flag_id)[6:])

tile_offset = ((row - 33) * 40 + (col - 30)) * 875
byte_offset = 489981 + tile_offset + local_id // 8
bit = 7 - (local_id % 8)  # Use local_id, NOT flag_id!
```

### Dungeon Flags (8-digit: AASSZZZZ)
```python
map_area = int(str(flag_id)[:2])
section = int(str(flag_id)[2:4])
local_id = int(str(flag_id)[4:])

byte_offset = DUNGEON_BASES[map_area] + section * 1125 + local_id // 8
bit = 7 - (local_id % 8)  # Use local_id, NOT flag_id!
```

---

## Industry Best Practices for Reverse Engineering

### Differential Analysis Techniques

1. **Temporal Differencing**: Same save before/after action
   - Use case: Single flag discovery
   - Method: Capture save state, perform action, compare bytes
   - Confidence: High (0.8) when single bit changes

2. **Comparative Differencing**: Different progression states
   - Use case: Block base discovery
   - Method: Compare progressed vs early-game slots
   - Confidence: Very high (0.9) with expected patterns

3. **Statistical Analysis**: Aggregate patterns across multiple saves
   - Use case: Formula validation
   - Method: Collect data points, verify consistency
   - Confidence: High when patterns hold across samples

### Evidence Collection Standards

| Evidence Type | Confidence | Requirements |
|---------------|------------|--------------|
| Multi-slot differential | 0.9 | Both slots tested, expected pattern matches |
| Temporal diff | 0.8 | Before/after captured, single change isolated |
| Dual-formula agreement | 0.85 | Tile + block formulas agree |
| Inseparable pair | 0.95 | Boss defeat + grace both consistent |
| Single slot match | 0.5 | Only confirms flag set, not formula |

### Reproducibility Requirements

1. **Save files archived** with timestamps
2. **Script outputs logged** with input parameters
3. **Ground truth JSON versioned** in git
4. **Methodology documented** per verification session

### Tools Consideration

For complex binary format parsing, consider specialized libraries:

| Library | Language | Use Case |
|---------|----------|----------|
| `construct` | Python | Declarative binary parsing |
| `kaitai-struct` | Multiple | Cross-platform format descriptions |
| `binwalk` | Python | Firmware/binary analysis |

Current approach (manual struct unpacking) works well for our focused use case but these may help for broader save file format documentation.

---

## Automated Snapshot Capture Workflow

### Overview

Manual snapshot naming is error-prone and doesn't scale. The automated capture system provides:

1. **Proper flag ID tracking** - Uses storable flag_id (row_id for tiles), NOT getItemFlagId
2. **Before/after pairing** - Automatic linkage with support for auto-chaining
3. **Slot context extraction** - Captures EF offset and calibrated bases at capture time
4. **Structured catalog** - Machine-readable JSON catalog for test selection

### Capture Workflow

```
1. Player approaches POI in-game
2. Player quits to main menu (forces save)
3. Player opens elden-map /character-game-data
4. Player clicks on POI marker, clicks [Capture Before]
5. System copies save with indexed naming, records in catalog
6. Player performs action in-game (pickup, grace touch, boss kill)
7. Player quits to main menu
8. Player clicks [Capture After] on same POI
9. System pairs captures, triggers diff analysis
```

### Capture Catalog

The capture catalog (`capture_catalog.json`) stores:

```json
{
  "captures": [
    {
      "id": "cap_001",
      "filename": "ER0000.sl2_capture_001_before_1044360040_m60_44_36",
      "phase": "before",
      "poi": {
        "flag_id": 1044360040,
        "flag_format": "tile",
        "map_tile": "m60_44_36"
      },
      "slot_context": {
        "slot_index": 0,
        "ef_offset": 79540,
        "calibrated_tile_base": 489981
      }
    }
  ],
  "pairs": [
    {
      "pair_id": "pair_001",
      "before_capture": "cap_001",
      "after_capture": "cap_002",
      "flag_id": 1044360040,
      "verification_result": { "status": "verified" }
    }
  ]
}
```

### Using the Capture Agent

```bash
# Capture a before snapshot
python scripts/capture_agent.py capture --phase before --flag-id 1044360040 --poi-name "Somber Stone" --slot 0

# Capture an after snapshot (auto-pairs with most recent before)
python scripts/capture_agent.py capture --phase after --flag-id 1044360040 --poi-name "Somber Stone" --slot 0

# Run HTTP server for webapp integration
python scripts/capture_agent.py serve --port 8765

# Migrate existing snapshots to catalog
python scripts/capture_agent.py migrate

# Show catalog status
python scripts/capture_agent.py status
```

### Test Selection with Snapshot Test Runner

```python
from scripts.verification.snapshot_test_runner import SnapshotTestRunner

runner = SnapshotTestRunner()

# Get tests for tile formula
tests = runner.get_tests_for_formula("tile", max_count=5)

# Verify a specific flag
result = runner.verify_flag(1044360040)
print(f"Confidence: {result.aggregate_confidence:.2%}")
```

---

## References

- [ARCHITECTURE.md](ARCHITECTURE.md) - System structure and module organization
- [CORROBORATION-SYSTEM.md](CORROBORATION-SYSTEM.md) - Dual-formula validation
- [EVENT-FLAG-GEOGRAPHY.md](EVENT-FLAG-GEOGRAPHY.md) - Flag ranges and formats
