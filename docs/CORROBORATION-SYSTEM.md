# Multi-Point Corroboration System

This document describes the corroboration system for empirical validation of event flag offset formulas.

---

## Overview

The corroboration system provides automated cross-validation of flag offset formulas using relationships extracted from decompiled game files. Instead of manually verifying each flag, we can use known relationships between flags to detect formula errors.

**Key insight**: When a player picks up an item from the world, two flags are typically set:
1. **Tile flag** (10-digit): Records the world pickup location was looted
2. **Block flag** (5-digit): Records the item is now owned

If our formulas are correct, both flags should agree (both SET or both UNSET). Contradictions indicate formula errors.

---

## Terminology

| Term | Definition |
|------|------------|
| **Corroboration** | Independent observations agreeing on the same result |
| **Dual-formula pair** | A tile flag and block flag that should have matching states |
| **Tile flag** | 10-digit flag (1XXYYZZZZ) tracking world pickup location |
| **Block flag** | 5-digit flag (60000-99999) tracking item possession |
| **Relationship** | Connection between two flags extracted from game data |
| **Agreement** | Both flags in a pair have matching SET/UNSET state |
| **Contradiction** | Flags in a pair have mismatched states |

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    DATA SOURCES                              │
├─────────────────┬─────────────────┬──────────────────────────┤
│ ItemLotParam    │ ShopLineupParam │ BonfireWarpParam         │
│ (world pickups) │ (shop items)    │ (graces)                 │
└────────┬────────┴────────┬────────┴────────┬─────────────────┘
         │                 │                 │
         └─────────────────┼─────────────────┘
                           ▼
         ┌─────────────────────────────────────┐
         │  extract_flag_relationships.py      │
         │  (scripts/)                         │
         └─────────────────┬───────────────────┘
                           │
                           ▼
         ┌─────────────────────────────────────┐
         │  flag_relationships.json            │
         │  2,796 relationships, 5,079 flags   │
         └─────────────────┬───────────────────┘
                           │
         ┌─────────────────┼─────────────────┐
         ▼                 ▼                 ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│ RelationshipGraph│ │CorroborationEngine│ │  CLI Commands   │
│ (indexing)      │ │ (validation)    │ │ (user interface)│
└─────────────────┘ └─────────────────┘ └─────────────────┘
```

### Components

**1. `scripts/extract_flag_relationships.py`**
- Parses decompiled game files (XML params, event scripts)
- Extracts flag relationships with type annotations
- Outputs `flag_relationships.json`

**2. `src/discovery/relationship_graph.rs`**
- Loads relationship JSON at runtime
- Indexes by source flag, target flag, and relationship type
- Extracts dual-formula corroboration pairs (122 pairs)

**3. `src/discovery/corroboration.rs`**
- Validates flag pairs against save file data
- Calculates agreement ratios and confidence scores
- Reports contradictions for investigation

---

## Relationship Types

| Type | Source | Target | Meaning |
|------|--------|--------|---------|
| `pickup_sets_flag` | Tile flag (10-digit) | Block flag (5-digit) | World pickup sets possession |
| `enables_purchase` | Release flag | Stock flag | Unlocking enables shop item |
| `grace_discovery` | Entity ID | Grace flag | Resting at grace sets flag |
| `boss_remembrance` | Boss defeat flag | Remembrance flag | Boss drops remembrance |
| `event_sequence` | Flag A | Flag B | Flags set together in events |
| `map_fragment` | Pickup flag | Possession flag | Map fragment discovery |

---

## CLI Commands

### Check Single Flag

```bash
cargo run -- discovery corroborate <flag_id> --slot <N> --save <path>
```

Example:
```bash
cargo run -- discovery corroborate 67650 --slot 0 --save ER0000.sl2
```

Output:
```
Corroboration check for flag 67650:

Status: StrongCorroboration
Agreement: 100.0%
Confidence adjustment: +0.20

Related flag checks:
  1046400030 (pickup_sets_flag) - Expected: SET, Actual: SET [MATCH]

Dual-formula check:
  Tile flag 1046400030: Some(true)
  Block flag 67650: Some(true)
  Agreement: YES
```

### Batch Validate All Pairs

```bash
cargo run -- discovery corroborate --all --slot <N> --save <path>
```

Example:
```bash
cargo run -- discovery corroborate --all --slot 1 --save ER0000.sl2
```

Output:
```
Validating all corroboration pairs against slot 1...

Batch Corroboration Result:
  Total pairs: 122
  Agreements: 62 (50.8%)
  Contradictions: 0
  Inconclusive: 60
```

### Show Graph Statistics

```bash
cargo run -- discovery graph
```

Output:
```
Relationship Graph Summary:
  Total relationships: 2796
  Total flags: 5079
  Corroboration pairs: 122
  By type:
    pickup_sets_flag: 1802
    enables_purchase: 325
    grace_discovery: 422
    boss_remembrance: 68
    event_sequence: 155
    map_fragment: 24
```

---

## Validation Workflow

### Step 1: Run Batch Corroboration

Start with an early-game character to establish baseline:

```bash
cargo run -- discovery corroborate --all --slot 1 --save ER0000.sl2
```

**Expected result**: 0 contradictions (character hasn't obtained items)

### Step 2: Identify Contradictions

If contradictions appear, they fall into categories:

| Pattern | Block Flag | Tile Flag | Likely Cause |
|---------|------------|-----------|--------------|
| A | SET | UNSET | Item from shop/quest, OR tile formula error |
| B | UNSET | SET | Block formula error |
| C | Both SET but different | - | Read logic error |

### Step 3: Investigate Each Contradiction

For each contradiction, check:

1. **Is the item available from shops?**
   ```bash
   grep 'eventFlag_forStock="<flag>"' ShopLineupParam.param.xml
   ```
   If found → Expected behavior (shop purchase)

2. **Is the item only world-pickup?**
   ```bash
   grep 'getItemFlagId="<flag>"' ItemLotParam_map.param.xml
   ```
   If only world pickup → Formula error

### Step 4: Fix Formula Errors

Common issues discovered:

| Issue | Symptom | Fix |
|-------|---------|-----|
| Wrong base offset | All flags in block off | Adjust `base_offset` in ground_truth |
| Wrong col_base | Tiles in certain columns fail | Adjust `col_base` (was 42→30) |
| Bit mask error | Random flag misreads | Check `(1 << bit)` vs `(1 << (7-bit))` |

### Step 5: Revalidate

After fixing, rerun corroboration to confirm:

```bash
cargo run -- discovery corroborate --all --slot 1 --save ER0000.sl2
# Should show: Contradictions: 0
```

---

## Data Files

### `scripts/flag_relationships.json`

Structure:
```json
{
  "nodes": {
    "67650": { "id": 67650, "connections": 1 },
    "1046400030": { "id": 1046400030, "connections": 1 }
  },
  "edges": [
    {
      "source": 1046400030,
      "target": 67650,
      "type": "pickup_sets_flag",
      "file": "ItemLotParam_map",
      "item": "Missionary's Cookbook [3]",
      "notes": "Picking up at 1046400030 sets flag 67650"
    }
  ],
  "by_type": { ... },
  "statistics": {
    "total_flags": 5079,
    "total_relationships": 2796,
    "relationship_types": { ... }
  }
}
```

### `ground_truth_offsets.json`

Tile formula section:
```json
{
  "formulas": {
    "tile_formula": {
      "base_offset": 495830,
      "bytes_per_slot": 875,
      "slots_per_row": 40,
      "row_base": 33,
      "col_base": 30,
      "max_local_id": 6999,
      "status": "needs_revalidation"
    }
  }
}
```

---

## Interpreting Results

### Agreement Percentage

| Range | Interpretation |
|-------|----------------|
| 90-100% | Formulas highly reliable |
| 70-90% | Some items obtained via non-pickup methods |
| 50-70% | Mixed acquisition (shop purchases common) |
| <50% | Potential formula errors, investigate |

### Inconclusive Results

Pairs are marked "inconclusive" when:
- Tile offset calculation fails (invalid coordinates)
- Block offset calculation fails (unverified block base)
- Either flag returns `None`

High inconclusive count indicates gaps in formula coverage.

### Expected Contradictions

Not all contradictions are errors. Valid reasons:
- Item purchased from merchant (shop flag, not world pickup)
- Item obtained via quest reward
- Item found in chest (different flag system)
- NPC drop (boss remembrance, etc.)

---

## Regenerating Relationship Data

If game files are updated:

```bash
cd scripts/
python3 extract_flag_relationships.py
```

Requires:
- Python 3.8+
- Decompiled game files at configured path
- Access to `regulation-bin/*.param.xml` and `event/common.emevd.js`

---

## Implementation Details

### Bit Position Convention

The system uses consistent bit positioning:
```rust
// Calculate bit position
let bit_position = 7 - ((flag_id % 8) as u8);

// Read flag (use bit directly, NOT 7-bit)
let is_set = (event_flags[byte_offset] & (1 << bit_position)) != 0;
```

### Tile Formula

```rust
let tile_index = (flag_id - 1_000_000_000) / 10000;
let local_id = flag_id % 10000;
let row = tile_index / 100;
let col = tile_index % 100;

let slot = (row - ROW_BASE) * SLOTS_PER_ROW + (col - COL_BASE);
let byte_offset = BASE_OFFSET + slot * BYTES_PER_SLOT + local_id / 8;
let bit = 7 - (local_id % 8);
```

Constants (verified):
- `BASE_OFFSET`: 495830
- `BYTES_PER_SLOT`: 875
- `SLOTS_PER_ROW`: 40
- `ROW_BASE`: 33
- `COL_BASE`: 30

---

## References

- `src/discovery/relationship_graph.rs`: Graph loader implementation
- `src/discovery/corroboration.rs`: Validation engine
- `scripts/extract_flag_relationships.py`: Data extraction
- `ground_truth_offsets.json`: Formula definitions
- `CLAUDE.md`: Event flag range documentation
