# Multi-Point Corroboration System

This document describes the corroboration system for empirical validation of event flag offset formulas.

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: STABLE METHODOLOGY, era-mixed examples.** The corroboration principle (independent methods must agree; disagreement is the clue) is current and used by the pipeline. Script names and worked offsets are pre-reset.
> - **Claims**: how multiple independent signals corroborate a flag offset/state.
> - **Evidence**: the method; examples used period-specific saves.
> - **Methodology**: canonical — see CONTEXT.md → *Reward Corroboration*, *Multi-slot Differential*, *Status Ladder*.
> - **Obsolete**: Python tooling and any static-base offset in the examples; resolve per save.

---

## Overview

The corroboration system provides automated cross-validation of flag offset formulas using relationships extracted from decompiled game files. Instead of manually verifying each flag, we can use known relationships between flags to detect formula errors.

**Key insight**: When a player picks up an item from the world, two flags are typically set:

1. **Tile flag** (10-digit): Records the world pickup location was looted
2. **Block flag** (5-digit): Records the item is now owned

If our formulas are correct, both flags should agree (both SET or both UNSET). Contradictions indicate formula errors.

---

## Terminology

| Term                  | Definition                                                  |
| --------------------- | ----------------------------------------------------------- |
| **Corroboration**     | Independent observations agreeing on the same result        |
| **Dual-formula pair** | A tile flag and block flag that should have matching states |
| **Tile flag**         | 10-digit flag (1XXYYZZZZ) tracking world pickup location    |
| **Block flag**        | 5-digit flag (60000-99999) tracking item possession         |
| **Relationship**      | Connection between two flags extracted from game data       |
| **Agreement**         | Both flags in a pair have matching SET/UNSET state          |
| **Contradiction**     | Flags in a pair have mismatched states                      |

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
         ┌─────────────────┼────────────────────┐
         ▼                 ▼                    ▼
┌──────────────────┐ ┌───────────────────┐ ┌─────────────────┐
│ RelationshipGraph│ │CorroborationEngine│ │  CLI Commands   │
│ (indexing)       │ │ (validation)      │ │ (user interface)│
└──────────────────┘ └───────────────────┘ └─────────────────┘
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

| Type               | Source               | Target               | Meaning                      |
| ------------------ | -------------------- | -------------------- | ---------------------------- |
| `pickup_sets_flag` | Tile flag (10-digit) | Block flag (5-digit) | World pickup sets possession |
| `enables_purchase` | Release flag         | Stock flag           | Unlocking enables shop item  |
| `grace_discovery`  | Entity ID            | Grace flag           | Resting at grace sets flag   |
| `boss_remembrance` | Boss defeat flag     | Remembrance flag     | Boss drops remembrance       |
| `event_sequence`   | Flag A               | Flag B               | Flags set together in events |
| `map_fragment`     | Pickup flag          | Possession flag      | Map fragment discovery       |

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

| Pattern | Block Flag             | Tile Flag | Likely Cause                                |
| ------- | ---------------------- | --------- | ------------------------------------------- |
| A       | SET                    | UNSET     | Item from shop/quest, OR tile formula error |
| B       | UNSET                  | SET       | Block formula error                         |
| C       | Both SET but different | -         | Read logic error                            |

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

| Issue             | Symptom                       | Fix                                    |
| ----------------- | ----------------------------- | -------------------------------------- |
| Wrong base offset | All flags in block off        | Adjust `base_offset` in ground_truth   |
| Wrong col_base    | Tiles in certain columns fail | Adjust `col_base` (was 42→30)          |
| Bit mask error    | Random flag misreads          | Check `(1 << bit)` vs `(1 << (7-bit))` |

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
      "base_offset": 485330,
      "bytes_per_slot": 875,
      "slots_per_row": 40,
      "row_base": 33,
      "col_base": 30,
      "max_local_id": 6999,
      "status": "verified"
    }
  }
}
```

---

## Interpreting Results

### Agreement Percentage

| Range   | Interpretation                             |
| ------- | ------------------------------------------ |
| 90-100% | Formulas highly reliable                   |
| 70-90%  | Some items obtained via non-pickup methods |
| 50-70%  | Mixed acquisition (shop purchases common)  |
| <50%    | Potential formula errors, investigate      |

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

See [EVENT-FLAG-GEOGRAPHY.md](EVENT-FLAG-GEOGRAPHY.md#1-overworld-tile-system-10-digit-flags) for the full tile formula.

Key constant: `BASE_OFFSET = 485330` (source of truth: `crates/wasm-event-flags/src/lib.rs`)

---

## Dungeon Formula Verification (2026-01-21)

### Formula

```rust
byte_offset = base + section * section_size + local_id / 8
bit = 7 - (local_id % 8)
```

Where for flag `AASSLLLL`:
- `AA` = area (10-39)
- `SS` = section (00-99)
- `LLLL` = local_id (0000-9999)

### Base Calculation

```rust
base = 4112 + slot * 1125
```

Slot assignments from `legacymap.eventflagalloclist`:
- m10 (Stormveil): slot 0,1
- m11 (Leyndell): slot 4,5,6
- m16 (Volcano Manor): slot 29
- m18 (Roundtable Hold): slot 35

### Empirically Verified Bases (2026-01-21)

| Area | Base   | Status   | Evidence                                         |
|------|--------|----------|--------------------------------------------------|
| 10   | 4112   | calculated | Test character bypassed Stormveil              |
| 11   | 8612   | verified | 50+ non-zero bytes, flags 11000001-11000950    |
| 16   | 36737  | **disproven** | Reads unrelated data - see Inseparable Evidence section |
| 18   | 43487  | verified | 60+ non-zero bytes, extensive section 0 data   |

### Verification Method

1. Detect event_flags offset using validation flags (71800, 76100, etc.)
2. Extract event_flags section from slot data
3. Check for non-zero bytes at calculated base offsets
4. Cross-reference specific boss defeat flags with game state

---

## Boss Remembrance System (Verified 2026-01-21)

### Flag Chain Structure

When a boss is defeated, multiple flags are set:

1. **Dungeon defeat flag** (8-digit): e.g., `16000800` for Rykard
2. **91xx progression flag**: Triggers Event 1100 to award progression items
3. **Remembrance pickup flag** (510xxx): Set when player collects the dropped remembrance

### Key Discovery: Event 1100 Awards Progression Items, NOT Remembrances

The common.emevd.js Event 1100 system awards **progression items** like Talisman Pouch:

```javascript
$InitializeEvent(5, 1100, 9105, 10050, 0, 60520);
// 9105 = progression flag (set on boss death)
// 10050 = ItemLot (awards Talisman Pouch, item 10040)
// 60520 = pickup completion flag
```

**Remembrances** are separate world drops with their own pickup flags (510xxx):

| Boss | Remembrance ID | Pickup Flag | Dungeon Flag |
|------|---------------|-------------|--------------|
| Godrick | 2950 | 510010 | 10000800 |
| Rennala | 2959 | 197 | 14000800 |
| Radahn | 2951 | 510300 | (field boss) |
| Morgott | 2952 | 510040 | 11000800 |
| Rykard | 2953 | 510220 | 16000800 |
| Mohg | 2955 | 510120 | 12050800 |
| Malenia | 2954 | 510200 | 15000800 |
| Maliketh | 2956 | 510160 | 13000800 |
| Hoarah Loux | 2957 | 510070 | 11000850 |
| Radagon | 2963 | 510230 | 19000800 |

### 91xx Flag Mapping (from event scripts)

The 91xx flags set on boss death are **different** from the Event 1100 params:

| Boss Death | Map | 91xx Flag |
|------------|-----|-----------|
| Godrick (10000800) | m10_00 | 9101 |
| Radahn | m10_01 | 9103 |
| Morgott (11000800) | m11_00 | 9104 |
| Hoarah Loux (11000850) | m11_00 | 9105 |
| Mohg | m11_05 | 9106 |
| Malenia | m11_05 | 9107 |
| Rykard (16000800) | m16_00 | 9122 |
| Radagon (19000800) | m19_00 | 9123 |

### Verification Script

`scripts/verification/verify_boss_chain.py` validates flag chains:

```bash
python3 scripts/verification/verify_boss_chain.py <save_path> <slot>
```

Valid states:
- **Both unset**: Boss not defeated
- **Dungeon set, pickup unset**: Boss killed, remembrance not collected
- **Both set**: Boss killed and remembrance collected
- **Dungeon unset, pickup set**: CONTRADICTION (cheating detected)

---

## Inseparable Evidence Methodology (2026-01-21)

### Overview

Inseparable evidence is a validation technique using flags that **cannot be set individually** in normal gameplay. When a player defeats a boss, multiple flags are set atomically - if our formula correctly reads one flag, all related flags must also match.

### Principle

Some game events set multiple flags that are **inseparable** in practice:
- Boss defeat flag → Post-boss grace becomes available
- Boss death → Remembrance drops → Pickup flag set when collected
- Dungeon entered → Tutorial flags set

If flag A is SET but inseparable flag B is UNSET, either:
1. The formula for A is wrong (reading unrelated data)
2. The formula for B is wrong
3. Both formulas are wrong

### Boss-Grace Inseparable Pairs

The most reliable inseparable pairs are boss defeat flags and their corresponding post-boss graces:

| Boss | Defeat Flag | Grace Flag | Grace Name |
|------|-------------|------------|------------|
| Godrick | 10000800 | 71010 | Godrick the Grafted |
| Rennala | 14000800 | 71140 | Rennala, Queen of the Full Moon |
| Morgott | 11000800 | 71110 | Morgott, the Omen King |
| Rykard | 16000800 | 71600 | Audience Pathway |
| Malenia | 15000800 | 71500 | Malenia, Goddess of Rot |
| Maliketh | 13000800 | 71300 | Beside the Great Bridge |
| Radagon | 19000800 | (none) | Final boss - no post-grace |

### Validation Logic

```python
def validate_inseparable_pair(boss_defeat, grace_flag):
    boss_set = check_dungeon_flag(boss_defeat)
    grace_set = check_block_flag(grace_flag)

    if boss_set is True and grace_set is False:
        return "IMPOSSIBLE - formula error likely"
    elif boss_set is False and grace_set is True:
        return "IMPOSSIBLE - grace before boss defeat"
    elif boss_set == grace_set:
        return "CONSISTENT"
    else:
        return "INCONCLUSIVE"
```

### Case Study: Volcano Manor (Area 16) Disproven

**Initial Assumption**: Base 36737 (slot 29 from legacymap)

**Test Results** (2026-01-21):
```
16000800 (Rykard defeat): SET at byte 36837
71600 (Audience Pathway grace): NOT SET
71601-71606 (VM graces): All NOT SET
```

**Analysis**:
- If Rykard was defeated, post-boss grace MUST be discoverable
- User confirmed character has NOT reached Volcano Manor
- Zero Volcano Manor graces discovered = character hasn't explored area

**Conclusion**: Base 36737 reads **unrelated data**. The 0xFF byte at offset 36837 is NOT the Rykard defeat flag - it's other data that happens to have bit 7 set.

**Status**: Area 16 marked as "disproven", base_offset = 0, awaiting correct base discovery.

### Applying to Other Areas

When validating dungeon bases:

1. **Find inseparable pairs** for that area:
   - Boss defeat flag + post-boss grace
   - Boss defeat flag + remembrance pickup (if collected)
   - Dungeon entry + tutorial flags

2. **Cross-validate ALL pairs**:
   ```
   For each (flag_a, flag_b) in inseparable_pairs:
       if (flag_a SET and flag_b UNSET) or (flag_a UNSET and flag_b SET):
           return CONTRADICTION
   return VALID
   ```

3. **Require player confirmation** when possible:
   - "Have you defeated this boss?"
   - "Have you explored this area?"

### Known Inseparable Chains

**Boss Defeat Chain**:
```
Boss Death → Sets dungeon defeat flag (e.g., 16000800)
          → Sets 91xx progression flag (e.g., 9122)
          → Remembrance drops as world item
          → Grace becomes available

Player picks up remembrance → Sets 510xxx flag
Player rests at grace → Sets 71xxx flag
```

**All flags in the chain should be consistent** with the player's actual progress.

---

## References

- `src/discovery/relationship_graph.rs`: Graph loader implementation
- `src/discovery/corroboration.rs`: Validation engine
- `scripts/extract_flag_relationships.py`: Data extraction
- `scripts/verification/verify_boss_chain.py`: Boss chain verification
- `scripts/verification/verify_rykard_chain.py`: Rykard-specific chain verification
- `ground_truth_offsets.json`: Formula definitions
- `CLAUDE.md`: Event flag range documentation
