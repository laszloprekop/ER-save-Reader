# Confidence Normalization & Blindspot Analysis

This document addresses concerns about confidence score distortion and introduces new analysis capabilities.

## Question 1: Cross-Save Validation Score Inflation

**Problem**: More save files = higher confidence, regardless of actual evidence quality.

**Solution**: Diminishing returns with caps.

```python
# Each subsequent piece of same-type evidence contributes less
contribution = base_weight * (0.5 ** count)

# And there's a hard cap per type
MAX_CONTRIBUTION = {
    "cross_save": 0.20,  # Can never exceed 0.20 from cross-save alone
}
```

**Example**:
```
Cross-save evidence #1: +0.100 (total: 0.100)
Cross-save evidence #2: +0.050 (total: 0.150)  # 50% of previous
Cross-save evidence #3: +0.025 (total: 0.175)  # 50% of previous
Cross-save evidence #4: +0.012 (total: 0.187)  # 50% of previous
Cross-save evidence #5: +0.006 (total: 0.193)  # 50% of previous
Cross-save evidence #6: +0.003 (total: 0.196)  # 50% of previous
Cross-save evidence #7: +0.002 (total: 0.198)  # 50% of previous
Cross-save evidence #8: +0.001 (total: 0.199)  # 50% of previous
...cap reached at 0.20...
```

**Result**: Whether you have 3 saves or 30, cross-save can only contribute 0.20 to confidence.

---

## Question 2: Chain Anchor Score Inflation

**Problem**: More related anchors = higher confidence, even if they're redundant.

**Solution**: Same diminishing returns + cap system.

```python
CONFIDENCE_CAPS = {
    "chain_anchor": 0.15,  # Max 0.15 from chain anchors
}
```

**Additionally**: Chain anchor confidence is normalized by match rate, not count:
```python
match_rate = anchor_matches / total_anchors
supports = match_rate >= 0.7  # 70% required, regardless of count
```

**Result**: An item with 10 anchors where 7 match gets the same contribution as an item with 100 anchors where 70 match.

---

## Question 3: Blindspot/Coverage Analysis

**New Tool**: `blindspot_analysis.py`

### What It Detects

#### 1. Data Region Mapping
```
Block 520000 at base 1341:
  ├── Data region: bytes 0-6 (flags 520000-520055)
  ├── PADDING GAP: bytes 7-10 (0xFF)
  ├── Data region: bytes 11-22 (flags 520088-520183)
  ├── PADDING GAP: bytes 23-26 (0xFF)
  └── ...
```

#### 2. Coverage Percentage
```
Block      Base     Coverage   Data     Gaps
520000     1341     ████░░░░░░ 30       8
710000     13875    ██████████ 125      0
```

#### 3. Unknown Data Regions
```
--- UNKNOWN DATA REGIONS ---
Offset          Size     First Bytes
117034-117048   15       0x08 0x40 0x01 0x84
116869-116882   14       0x18 0x08 0x01 0x80
```

These are bytes with data (not 0xFF or 0x00) that don't belong to any known block.

#### 4. Inventory Correlation
```python
# How well does block structure match inventory items?
Items in data regions: 12
Items in padding regions: 3  # ← These need investigation
```

### Running the Analysis

```bash
# Full scan
python scripts/verification/blindspot_analysis.py

# Specific block
python scripts/verification/blindspot_analysis.py --block 520000 --base 1341
```

---

## Question 4: Unknown Base Tracking

**New Class**: `UnknownBaseTracker`

When cases are rejected, we track them to find patterns:

```python
base_tracker = UnknownBaseTracker()

# When a case is rejected
base_tracker.record_rejected_flag(
    flag_id=520210,
    name="Assassin's Cerulean Dagger",
    attempted_base=1341,
    rejection_reason="padding_check"
)

# Later, search for patterns
potential_bases = base_tracker.search_for_patterns(
    ef_data,
    rejected_flags,
    search_range=(0, 10000)
)

# Returns bases where multiple rejected flags WOULD work
# → Indicates these flags might use a different base
```

**Output Example**:
```
Potential base 1295:
  Supporting flags: [520210, 520330, 520450]
  Match count: 3
  Confidence: 0.30
  Notes: "Rejected flags might use this alternative base"
```

---

## Question 5: Lookup Table Discovery

**New Class**: `LookupTableDiscovery`

Some games store offset addresses directly in a table. This class searches for such patterns.

### How It Works

1. **Scan for known offset values**
   ```python
   # Search for 4-byte values matching known bases
   for offset in search_range:
       value = struct.unpack('<I', data[offset:offset+4])[0]
       if value in known_offsets:
           # Found a stored offset!
   ```

2. **Find clusters**
   ```python
   # Lookup tables have entries close together
   if entry.offset - previous.offset <= 16:
       # Part of same table
   ```

3. **Analyze confidence**
   ```python
   confidence = known_matches / total_entries
   # High confidence = likely a real lookup table
   ```

### Example Output
```
--- POTENTIAL LOOKUP TABLES ---
Table at offset 50200-50280:
  Entries: 20
  Known matches: 8
  Matched blocks: [71000, 72000, 73000, 74000, 75000, 76000, 77000, 78000]
  Confidence: 0.40
```

This would indicate a region where block offsets are stored directly.

---

## Implementation Summary

| Feature | File | Description |
|---------|------|-------------|
| Normalized confidence | `case_manager.py` | Diminishing returns + caps |
| Coverage analysis | `case_analysis.py` | Data vs padding per block |
| Unknown region scanner | `case_analysis.py` | Find unmapped data |
| Base tracker | `case_analysis.py` | Track rejected flags |
| Lookup discovery | `case_analysis.py` | Find offset tables |
| Blindspot CLI | `blindspot_analysis.py` | Command-line tool |

---

## Confidence Breakdown Example

```python
case = manager.create_case(...)

# After running verification
breakdown = case.get_confidence_breakdown()

print(breakdown)
# {
#     "total": 0.72,
#     "by_type": {
#         "inventory_presence": 0.35,  # Capped
#         "differential": 0.22,
#         "cross_save": 0.15,
#     },
#     "counts": {
#         "inventory_presence": 5,  # 5 pieces, but capped at 0.35
#         "differential": 4,
#         "cross_save": 3,
#     },
#     "caps": {
#         "inventory_presence": 0.35,
#         "differential": 0.25,
#         "cross_save": 0.20,
#     }
# }
```

This transparency helps identify when scores are hitting limits and where more diverse evidence types are needed.
