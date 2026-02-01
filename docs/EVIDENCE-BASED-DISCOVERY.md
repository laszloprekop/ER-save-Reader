# Evidence-Based Flag Discovery Methodology

A systematic approach for discovering unknown event flag offsets using **multiple evidence sources** and **triangulation**.

---

## Problem Statement

Some flag ranges (e.g., 520xxx) have no known formula. Items in inventory use these flags, but we can't verify them because we don't know where they're stored in the save file.

### Current Evidence Types (Single Source)
- Multi-slot differential (S0 vs S1)
- Temporal diff (before/after)
- Manual completion logs

### Goal: Multi-Evidence Triangulation
Combine multiple independent evidence sources to:
1. Narrow the search space
2. Increase confidence in discoveries
3. Enable fully automated verification

---

## The Evidence Triangle

```
                    INVENTORY
                   (Character has item)
                        /\
                       /  \
                      /    \
                     /      \
                    /   ✓    \
                   /          \
                  /____________\
    FLAG STATUS              MANUAL LOG
    (Bit set in EF)       (User marked complete)
```

**Perfect match**: All three corners agree → Very High Confidence (0.95)
**Two of three**: Inventory + Flag OR Flag + Manual → High Confidence (0.85)
**Single source**: Only one evidence type → Low Confidence (0.5)

---

## Discovery Workflow

### Phase 1: Ground Evidence Collection

Start with what we **know for certain** from the save file:

```
1. SELECT save file and slot
2. EXTRACT inventory data → List of (item_id, item_name)
3. FOR each unique item with unknown flag:
   a. Look up associated flag(s) via FLAGS_BY_ITEM
   b. Check if flag has formula (can be calculated)
   c. If no formula → candidate for discovery
4. OUTPUT: List of (item_id, flag_id, item_present=true, flag_formula=unknown)
```

### Phase 2: Related Flag Chain Expansion

Use `ItemChainResolver` to find connected flags:

```
FOR each (item_id, flag_id) candidate:
   chain = ItemChainResolver.resolve_chain(item_id)

   FOR each related_flag in chain:
      IF related_flag.has_formula:
         # This is our anchor!
         IF is_flag_set(ef_data, related_flag):
            RECORD: "Anchor flag {related_flag} is SET"
            INFER: "Unknown flag {flag_id} should also be SET"
```

**Key insight**: If boss defeat flag 171 (Godrick) is SET, then any Godrick-related item pickup flags should also be SET.

### Phase 3: Manual Log Cross-Reference

Query external flag database for corroborating evidence:

```
FOR each flag_id in candidates:
   manual_entry = query_flag_database(flag_id)

   IF manual_entry.user_marked_complete:
      RECORD: "User confirmed flag {flag_id} in web app"
      confidence += 0.2

   IF manual_entry.name matches item_name:
      RECORD: "Flag name '{manual_entry.name}' matches item '{item_name}'"
      confidence += 0.1
```

### Phase 4: Base Offset Search

With high-confidence candidates, search for the block base:

```
FUNCTION discover_block_base(flag_id, ef_data_progressed, ef_data_early):
   block_start = (flag_id // 1000) * 1000  # e.g., 520000
   expected_bit = 7 - (flag_id % 8)

   candidates = []

   FOR byte_offset in range(EVENT_FLAGS_SIZE):
      # Calculate what flag_id this would be at this offset
      # IF formula is: byte = base + (flag_id - block_start) // 8
      # THEN: base = byte_offset - (flag_id - block_start) // 8

      candidate_base = byte_offset - (flag_id - block_start) // 8

      IF candidate_base < 0:
         CONTINUE

      # Check if bit is SET in progressed save
      progressed_bit = (ef_data_progressed[byte_offset] >> expected_bit) & 1

      # Check if bit is UNSET in early save (differential)
      early_bit = (ef_data_early[byte_offset] >> expected_bit) & 1

      IF progressed_bit == 1 AND early_bit == 0:
         # Check for 0xFF padding contamination
         IF NOT is_0xff_region(ef_data_progressed, byte_offset):
            candidates.append((candidate_base, byte_offset))

   RETURN candidates
```

### Phase 5: Cross-Validation

Validate discovered base against ALL known flags in the block:

```
FUNCTION validate_block_base(block_start, candidate_base, known_flags):
   matches = 0
   total = 0

   FOR flag_id, item_info in known_flags:
      total += 1

      byte_offset = candidate_base + (flag_id - block_start) // 8
      bit = 7 - (flag_id % 8)

      flag_is_set = check_bit(ef_data, byte_offset, bit)
      item_in_inventory = check_inventory(item_info.item_id)

      IF flag_is_set == item_in_inventory:
         matches += 1

   match_rate = matches / total
   RETURN (match_rate, matches, total)
```

### Phase 6: Multi-Save Verification

Verify formula works across ALL save files:

```
FUNCTION verify_across_saves(block_start, candidate_base, save_files):
   all_results = []

   FOR save_file in save_files:
      FOR slot in [0, 1, 2, 3, 4]:
         ef_data = extract_ef_data(save_file, slot)
         inventory = extract_inventory(save_file, slot)

         result = validate_block_base(block_start, candidate_base, ...)
         all_results.append(result)

   # All slots should have consistent formula
   # (match rate may vary by progression, but formula is constant)
   consistent = check_formula_consistency(all_results)

   RETURN consistent
```

---

## Evidence Aggregation Scoring

### Confidence Calculation

```python
def calculate_discovery_confidence(evidence):
    score = 0.0

    # Inventory evidence (ground truth)
    if evidence.item_in_inventory:
        score += 0.30

    # Flag detected at candidate offset
    if evidence.flag_detected:
        score += 0.25

    # Manual log confirms
    if evidence.manual_log_complete:
        score += 0.20

    # Chain anchor is set (related flag with formula)
    if evidence.chain_anchor_set:
        score += 0.15

    # Multi-slot differential matches
    if evidence.differential_valid:
        score += 0.10

    # Cross-save verification passes
    if evidence.cross_save_verified:
        score += 0.10

    return min(score, 1.0)
```

### Confidence Thresholds

| Confidence | Status | Action |
|------------|--------|--------|
| >= 0.90 | Verified | Record in ground_truth with "verified" |
| >= 0.75 | High | Record as "candidate", schedule cross-validation |
| >= 0.50 | Medium | Record as "calculated", needs more evidence |
| < 0.50 | Low | Do not record, continue discovery |

---

## Special Case: Chain-Based Discovery

When item has NO direct flag mapping, spider through related flags:

### Example: Spirit Ash with 520xxx flag

```
Item: "Lhutel the Headless" (item_id: 258000)
Expected flag: 520000 (no formula)
Location: Tombsward Catacombs reward

Chain expansion:
1. Item pickup → flag 520000 (unknown)
2. Catacomb completion → dungeon flags (30xxx)
3. Boss defeat (Cemetery Shade) → possible boss flag
4. Map tile → tile flag (104xxx)

Search strategy:
- Find the dungeon/tile flags that ARE set
- Infer that 520000 should also be set
- Use differential analysis on 520000 range
```

### Chain Resolution Algorithm

```python
def find_verifiable_chain_anchor(item_id, ef_data):
    """
    Spider from item to find a flag we CAN verify,
    then use that to anchor our search for unknown flags.
    """
    chain = ItemChainResolver.resolve_chain(item_id)

    anchors = []
    for flag in chain.chain_flags:
        if flag.has_formula and flag.is_set:
            anchors.append(flag)

    if not anchors:
        # No direct anchor - look for geographic correlation
        location = get_item_location(item_id)
        if location:
            # Find nearby flags that are set
            nearby_flags = find_flags_in_region(location)
            for f in nearby_flags:
                if is_flag_set(ef_data, f):
                    anchors.append(f)

    return anchors
```

---

## Blocking Flag Identification

The "blocking flag" prevents re-obtaining an item:

```
Item: Remembrance of the Grafted
Chain:
  1. Boss defeat (flag 171) → BLOCKING FLAG
  2. Remembrance granted (auto)
  3. Remembrance possession (flag 510010) → Consumption tracking
  4. Remembrance duplication (Walking Mausoleum flags)

The boss defeat flag (171) is the BLOCKING FLAG because:
- It's set once and never cleared
- It gates all downstream rewards
- You cannot fight Godrick again to get another Remembrance
```

### Blocking Flag Discovery

```python
def identify_blocking_flag(item_id, chain):
    """
    Find the flag that makes item non-re-obtainable.
    """
    for flag in chain.chain_flags:
        if flag.role == FlagRole.BossDefeat:
            return flag  # Boss kills are one-time

        if flag.role == FlagRole.ItemPickup:
            # World pickups are one-time (looted)
            return flag

        if flag.role == FlagRole.AreaAccess:
            # Area access might not block (can revisit)
            continue

    # Default: first flag in chain
    return chain.chain_flags[0] if chain.chain_flags else None
```

---

## Implementation: Discovery Service

### Core Module Structure

```
src/discovery/
├── evidence_discovery.rs       # NEW: Evidence-based discovery service
├── evidence_aggregator.rs      # NEW: Multi-source evidence scoring
├── block_searcher.rs           # NEW: Unknown block offset search
├── chain_validator.rs          # NEW: Chain-based validation
├── item_chain_resolver.rs      # Existing: Spider through related flags
├── inventory_verification.rs   # Existing: Inventory-flag mapping
└── ...
```

### Evidence Discovery Service

```rust
pub struct EvidenceDiscoveryService {
    chain_resolver: ItemChainResolver,
    block_searcher: BlockSearcher,
    evidence_aggregator: EvidenceAggregator,
}

impl EvidenceDiscoveryService {
    /// Discover unknown flag offsets starting from inventory evidence
    pub fn discover_from_inventory(
        &self,
        save_path: &Path,
        slot_index: usize,
    ) -> Vec<DiscoveryResult> {
        let save = load_save(save_path);
        let ef_data = extract_event_flags(&save, slot_index);
        let inventory = extract_inventory(&save, slot_index);

        let mut results = Vec::new();

        // Find items with unknown flag formulas
        for item in get_items_with_unknown_flags(&inventory) {
            let chain = self.chain_resolver.resolve_chain(
                item.item_id,
                &item.name,
                Some(&ef_data),
                None,
            );

            // Aggregate evidence
            let evidence = self.evidence_aggregator.collect_evidence(
                &item,
                &chain,
                &ef_data,
                &inventory,
            );

            if evidence.confidence >= 0.5 {
                // Try to discover the block base
                let candidates = self.block_searcher.search(
                    item.flag_id,
                    &ef_data,
                );

                results.push(DiscoveryResult {
                    item,
                    chain,
                    evidence,
                    base_candidates: candidates,
                });
            }
        }

        results
    }
}
```

---

## Recording Progress

### Discovery State Machine

```
                    ┌─────────────┐
                    │  UNKNOWN    │
                    └──────┬──────┘
                           │ evidence collected
                           ▼
                    ┌─────────────┐
                    │  CANDIDATE  │
                    └──────┬──────┘
                           │ multi-slot validated
                           ▼
                    ┌─────────────┐
                    │  VERIFIED   │◄──────────┐
                    └──────┬──────┘           │
                           │ cross-save       │ re-verified
                           │ fails            │
                           ▼                  │
                    ┌─────────────┐           │
                    │ REGRESSION  ├───────────┘
                    └─────────────┘
```

### Progress Record Format

```json
{
  "520000": {
    "block_start": 520000,
    "discovery_status": "candidate",
    "candidate_bases": [
      {
        "base_offset": 65000,
        "match_rate": 0.73,
        "evidence": {
          "inventory_matches": 11,
          "manual_log_matches": 8,
          "chain_anchors": 3
        }
      },
      {
        "base_offset": 65125,
        "match_rate": 0.67,
        "evidence": {}
      }
    ],
    "blockers": [
      "Need differential capture for Spirit Ashes",
      "520090 shows inverted in S1"
    ],
    "discovery_date": "2026-01-31",
    "last_verified": null
  }
}
```

---

## Success Criteria

A block is marked **verified** when:

1. **Match Rate >= 80%** across all known flags in block
2. **Multi-slot differential** shows expected pattern
3. **No 0xFF contamination** in the region
4. **Cross-save verification** passes (at least 2 saves)
5. **Chain anchors agree** (related flags with formulas match)
6. **No inversions** (S1 never shows MORE than S0)

---

## Practical Example: Discovering Block 520000

### Step 1: Collect Ground Evidence

```
Save: ER0000.sl2, Slot 0 (Confessor, mid-game)

Inventory items with 520xxx flags:
- Lhutel the Headless (258000) → flag 520000
- Assassin's Crimson Dagger (5050) → flag 520030
- Twinsage Sorcerer Ashes (219000) → flag 520050
... (15 more items)
```

### Step 2: Find Chain Anchors

```
Item: Lhutel the Headless
Location: Tombsward Catacombs

Chain expansion:
- Dungeon completion: 30020800 (Cemetery Shade defeat)
- Grace discovery: 71800+ (Church of Pilgrimage nearby)
- Tile flags: 1042460xxx (Weeping Peninsula tiles)

Checking anchors:
- 30020800: has_formula=true, is_set=true ✓
- 71801: has_formula=true, is_set=true ✓

Inference: Item obtained, flag 520000 SHOULD be set
```

### Step 3: Search for Base Offset

```
Block 520000, expected bit = 7 - (520000 % 8) = 7

Searching event flags region (0 - 500,000 bytes):
- At byte 65000, bit 7: S0=1, S1=0 ✓ (differential match)
- Calculated base: 65000 - (520000 - 520000) // 8 = 65000

Candidate base: 65000
```

### Step 4: Validate Against All Known Flags

```
Testing base 65000 against all 520xxx items in inventory:

520000 (Lhutel): byte=65000, bit=7, S0=1, S1=0 ✓
520030 (Assassin's Crimson): byte=65003, bit=5, S0=1, S1=0 ✓
520050 (Twinsage): byte=65006, bit=5, S0=1, S1=0 ✓
520080 (Kristoff): byte=65010, bit=7, S0=1, S1=0 ✓
...

Match rate: 15/17 = 88.2%
```

### Step 5: Record Result

```json
{
  "520000": {
    "block_start": 520000,
    "base_offset": 65000,
    "block_size": 1000,
    "status": "verified",
    "notes": "Spirit Ash/Talisman catacomb rewards. Discovered 2026-01-31 via evidence-based methodology. Match rate 88.2% (15/17 items). Validated across S0/S1 differential."
  }
}
```

---

## Discovery Results: Block 520000 (Updated 2026-02-01)

### Verified Findings

Block 520000 (Spirit Ashes, Talismans) **DOES** follow the standard block formula but with **sparse allocation**:

| Property | Value |
|----------|-------|
| Block start | 520000 |
| Base offset | **1341** |
| Total flags in schema | 59 |
| Allocated (trackable) | **46** |
| Unallocated (sparse gaps) | **13** |

### Schema-Based Verification

Using the `flag_schema.py` tool to generate an **allocation bitmap**:

```bash
python scripts/verification/flag_schema.py --block 520000 --base 1341 \
    --save "/path/to/save.sl2" --boundaries
```

**Allocation Boundaries Discovered:**
```
520000-520059: ALLOCATED
520060-520089: UNALLOCATED (sparse gap)
520090-520189: ALLOCATED
520190-520219: UNALLOCATED (sparse gap)
520220-520329: ALLOCATED
520330-520349: UNALLOCATED (sparse gap)
520350-520449: ALLOCATED
520450-520469: UNALLOCATED (sparse gap)
520470-520699: ALLOCATED
520700-520749: UNALLOCATED (sparse gap)
520750-520810: ALLOCATED
```

### Key Discovery: Sparse Allocation

The game uses **sparse flag allocation** for block 520000:

- Not all flag IDs have memory allocated
- Unallocated positions show 0xFF in **all** save slots (padding)
- Flag IDs in gaps (e.g., 520210, 520330, 520450) cannot be tracked via this formula

**Implications:**
1. Use the **allocation bitmap** to pre-filter trackable flags before verification
2. Items with flag IDs in sparse gaps may use different tracking mechanisms
3. The `BlockSchema` class provides `is_trackable(flag_id)` to check before verification

### Verified Flags (12 items exported to ground_truth)

| Flag ID | Item Name | Offset | Status |
|---------|-----------|--------|--------|
| 520000 | Lhutel the Headless | 1341 | Verified |
| 520030 | Assassin's Crimson Dagger | 1344 | Verified |
| 520040 | Banished Knight Engvall | 1346 | Verified |
| 520050 | Twinsage Sorcerer Ashes | 1347 | Verified |
| 520090 | Bloodhound Knight Floh | 1352 | Verified |
| 520110 | Perfumer Tricia | 1354 | Verified |
| 520300 | Viridian Amber Medallion | 1378 | Verified |
| 520310 | Spelldrake Talisman | 1379 | Verified |
| 520350 | Blue Dancer Charm | 1384 | Verified |
| 520370 | Cerulean Amber Medallion | 1387 | Verified |
| 520390 | Kindred of Rot's Exultation | 1389 | Verified |
| 520480 | Godskin Swaddling Cloth | 1401 | Verified |

### Untrackable Flags (in sparse gaps)

| Flag ID | Item Name | Reason |
|---------|-----------|--------|
| 520060 | Glintstone Sorcerer Ashes | Sparse gap 520060-520089 |
| 520070 | Kaiden Sellsword Ashes | Sparse gap 520060-520089 |
| 520080 | Ancient Dragon Knight Kristoff | Sparse gap 520060-520089 |
| 520210 | Assassin's Cerulean Dagger | Sparse gap 520190-520219 |
| 520330 | Flamedrake Talisman | Sparse gap 520330-520349 |
| 520450 | Gold Scarab | Sparse gap 520450-520469 |

### Methodology Used

1. **Schema Definition**: Load known flags from `extracted_event_flags.json`
2. **Allocation Probing**: Check each schema position across 5 save slots
3. **Bitmap Generation**: Mark positions as ALLOCATED or UNALLOCATED
4. **Case Verification**: Run case-based verification only on allocated flags
5. **Export**: Add verified flags to `ground_truth_offsets.json`

---

## Next Steps

1. Implement `EvidenceDiscoveryService` in Rust
2. Add `BlockSearcher` for systematic offset search
3. Create Python CLI for interactive discovery
4. Integrate with capture workflow for temporal diffs
5. Add regression testing to detect formula drift
6. **Investigate 520xxx as dungeon-linked flags rather than block formula**
