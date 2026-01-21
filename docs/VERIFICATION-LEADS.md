# Verification Leads from User Records

Analysis of `verification-records.jsonl` (473 records) to identify formula errors and verification opportunities.

## Summary Statistics

| Metric | Count |
|--------|-------|
| Total Records | 473 |
| Agreements (formula correct) | 64 (13.5%) |
| Mismatches | 409 (86.5%) |
| User SET, Formula NOT SET | 407 |
| User NOT SET, Formula SET | 2 |

**Key Insight**: 99.5% of mismatches are "User SET, Formula NOT SET" - strongly suggests formula errors, not user errors.

---

## Block-Level Findings

### High Priority (Many Mismatches)

| Block | Mismatches | Current Base | Finding |
|-------|------------|--------------|---------|
| 76000 | 82 | 3250 | Best probe: 3240 (31.8% match) - likely discontinuous |
| 67000 | 33 | 2280 | 0% match in range - base elsewhere or different formula |
| 73000 | 23 | 2662 | Best: 2725/2821 (45.5%) - multiple sub-ranges |
| 62000 | 14 | 1500 | 0% match - base elsewhere |
| 78000 | 14 | 3500 | Best: 3588 (50% match) |
| 65000 | 14 | 1875 | 0% match - base elsewhere |
| 71000 | 13 | 2625 | **Complex sub-block allocation** (see below) |

### Block 71000 Deep Analysis

Block 71000 has discontinuous allocation with multiple sub-ranges:

| Sub-range | Region | Best Base | Match | Status |
|-----------|--------|-----------|-------|--------|
| 71000-71099 | Stormveil Castle | 2821 | 7/9 (77.8%) | Partial - 71000, 71008 don't match |
| 71100-71199 | Leyndell | 2725 | 1/1 | 71109 Divine Bridge matches |
| 71600-71699 | Volcano Manor | 2726 | 2/2 (100%) | **VERIFIED** - v0.4.17 |
| 71800-71899 | Tutorial | 2725 | 2/2 (100%) | **VERIFIED** |

**Stormveil Deep Dive (Base 2821):**
- Byte 2821 = 0x7F (01111111)
- Flags 71001-71007: All SET (bits 6-0) ✓
- Flag 71000 (Godrick grace): NOT SET (bit 7)
- Flag 71008 (Main Gate): NOT SET (byte 2822 bit 7)

Possible explanations:
1. User error on 71000/71008 (unlikely - notable graces)
2. Boss graces stored separately from location graces
3. Need more data points to disambiguate

---

## Dungeon Area Findings

| Area | Mismatches | Current Base | Finding |
|------|------------|--------------|---------|
| 30 (Catacombs) | 12 | 27411 | Needs investigation |
| 12 (Underground) | 7 | 15362 | Needs investigation |
| 10 (Stormveil) | 5 | 4112 | Needs investigation |
| 32 (Tunnels) | 4 | 31577 | Probe confirmed earlier |
| 14 (Raya Lucaria) | 4 | 29987 | Needs investigation |
| 18 (Tutorial) | 3 | 43487 | Previously verified |
| 34 (Divine Towers) | 2 | 60362 | Needs investigation |
| 31 (Caves) | 2 | 28634 | Previously verified |

---

## False Positives (User NOT SET, Formula SET)

Only 2 records - suggests our formulas rarely have false positives:

| Flag ID | Name | Category |
|---------|------|----------|
| 62120 | Church of Elleh | Map Fragment? |
| 62460 | Sellia Hideaway | Map Fragment? |

These need investigation - Church of Elleh is an early-game area that most players discover.

---

## Verification Strategy

### Phase 1: Fix Known Issues
1. ✅ Block 71600 (VM graces) - Fixed in v0.4.17
2. Block 71000 (Stormveil) - Base 2821 gives 77.8% match
3. Block 71100 (Leyndell) - Base 2725 looks promising

### Phase 2: Investigate 0% Match Blocks
- Block 62000, 65000, 67000 - Bases completely outside searched range
- May use different formula or storage location
- Need to expand search range or investigate alternative formulas

### Phase 3: Apply Inseparable Evidence
For each block/area with significant mismatches:
1. Find inseparable flag pairs (boss + grace, pickup + possession)
2. Cross-validate to detect false positives
3. Confirm with user knowledge where possible

---

## Character Progress Reference

| Slot | Character | Level | Records | Match Rate |
|------|-----------|-------|---------|------------|
| 0 | Confessor | 93 | 396 | 12.6% |
| 1 | Wretch | ~early | 9 | 44.4% |
| 2 | V1 | ~test | 3 | 66.7% |
| 3 | V2 | ~test | 3 | 66.7% |
| 4 | V3 | ~test | 3 | 100% |
| 5 | Sam | 10 | 59 | 5.1% |

**Confessor (Slot 0)** has the most data and is mid-game with extensive exploration:
- Has explored: Limgrave, Liurnia, Altus, Caelid, Volcano Manor (partial)
- Has NOT defeated: Rykard (confirmed)
- Best source for verification testing

---

## Next Steps

1. **Immediate**: Add sub-blocks 71000 (base 2821) and 71100 (base 2725) to ground_truth
2. **Short-term**: Expand search ranges for 62000, 65000, 67000 blocks
3. **Medium-term**: Create inseparable evidence tests for dungeon areas
4. **Ongoing**: Collect more user confirmations to improve probe accuracy

---

## Files Generated

- `scripts/verification/analyze_verification_records.py` - Record analysis
- `scripts/verification/probe_block_bases.py` - Block base probing
- `scripts/verification/probe_stormveil_graces.py` - Stormveil deep dive
