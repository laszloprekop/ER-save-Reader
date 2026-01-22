# Verification Session: 2026-01-22

## Session Summary

Continued save file discovery/verification work focusing on:
1. Investigating flag 71000 (Godrick grace) mystery
2. Cross-examining inventory evidence
3. Investigating false positives in dungeon boss flags
4. Discovering 0xFF padding pattern issue

## Key Findings

### 1. Flag 71000 Mystery - RESOLVED ✓

**Issue**: Flag 71000 (Godrick the Grafted grace) showed as UNSET despite 8/9 other Stormveil graces being SET in Slot 0 (Confessor).

**Investigation**: Cross-examined multiple Godrick-related flags:
- Flag 160 (Godrick's Great Rune Possession): UNSET
- Flag 171 (Godrick World Drop marker): UNSET
- Flag 180 (Godrick's Great Rune Activated): UNSET
- Flag 9101 (Remembrance of the Grafted): UNSET
- Flag 10000800 (Godrick boss defeat - dungeon): UNSET

**Conclusion**: The Confessor character has **NOT defeated Godrick**. Flag 71000 being UNSET is **expected behavior** - the grace doesn't spawn until the boss is defeated. The Block 71000 base offset (9315) is correct.

### 2. 0xFF Padding Pattern Discovery - CRITICAL

**Issue**: False positives detected for dungeon boss flags (e.g., Flag 14000800 showing SET in early-game Slots 1-2).

**Root Cause**: Save files contain 0xFF padding regions that:
- Are at different **relative** offsets depending on the slot's EF start position
- Are at **consistent absolute** positions within the slot data
- Shift based on EF header size variations between slots

**Pattern Observed**:
| Slot | EF Start | First 0xFF (relative) | First 0xFF (absolute) |
|------|----------|----------------------|----------------------|
| 0    | 0x125A5  | 7189                 | 0x141BA             |
| 1    | 0x12B2F  | 2669                 | 0x1359C             |
| 2    | 0x12AFA  | 2722                 | 0x1359C             |
| 3    | 0x12AFA  | 2722                 | 0x1359C             |
| 4    | 0x12AF8  | 2724                 | 0x1359C             |

**Impact on Verification**:
- Offset 30087 (Area 14 boss flag) falls in 0xFF run for Slots 1-2
- Offset 29859 (Area 31 boss flag) falls in 0xFF run for Slot 3
- These cause false positives when reading dungeon boss defeat flags

**Recommendation**:
1. Flag reading code should detect 0xFF padding regions
2. Or use sanity checks (e.g., if entire surrounding region is 0xFF, flag is likely false positive)

### 3. Verified Progression Analysis

Updated slot-by-slot progression comparison:

| Flag | Description | S0 | S1 | S2 | S3 | S4 | Notes |
|------|-------------|----|----|----|----|----|----|
| **Stormveil Graces (Block 71000, base 9315)** |
| 71000 | Godrick grace | unset | unset | unset | unset | unset | Requires boss defeat |
| 71001 | Margit grace | SET | unset | unset | unset | unset | ✓ Verified |
| 71008 | Main Gate | SET | unset | unset | unset | unset | ✓ Verified |
| **Volcano Manor (Block 71600, base 2825)** |
| 71607 | Abductor grace | SET | unset | unset | unset | unset | ✓ Verified |
| **Tutorial (Block 71800, base 2725)** |
| 71800 | Cave of Knowledge | SET | SET | SET | SET | SET | All slots |
| 71801 | Stranded Graveyard | SET | SET | SET | SET | SET | All slots |
| **Maps (Block 62000, base 9359)** |
| 62010 | Limgrave, West | SET | unset | unset | unset | unset | ✓ Verified |
| 62011 | Limgrave, East | SET | unset | unset | unset | unset | ✓ Verified |
| 62040 | Liurnia, East | SET | unset | unset | unset | unset | ✓ Verified |

### 4. Known Issues Identified

| Issue | Severity | Description | Workaround |
|-------|----------|-------------|------------|
| 0xFF Padding False Positives | High | Dungeon boss flags at offsets ~29859-30087 hit padding in some slots | Detect 0xFF regions before reading |
| Area 14 Base | Medium | Base 29987 verified for many flags but hits padding for boss flag | Need positive evidence from Sewers-progressed save |
| Area 31 Base | Medium | Similar padding issue for cave boss flags | Same as above |

## Scripts Created

1. `check_godrick_defeat.py` - Check Godrick boss defeat flag and grace
2. `check_godrick_inventory.py` - Cross-examine inventory for defeat evidence
3. `analyze_character_progression.py` - Compare flags across all 5 slots
4. `investigate_area14_false_positive.py` - Area 14 Sewers investigation
5. `verify_coastal_cave_boss.py` - Area 31 Caves investigation
6. `investigate_0xff_padding.py` - 0xFF padding pattern analysis

## Next Steps

1. **Implement 0xFF detection** in flag reading code to avoid false positives
2. **Create new test saves** with:
   - Godrick defeated (to verify 71000 grace and 10000800)
   - Coastal Cave Beastman defeated (to verify Area 31)
   - Some Sewers progression (to verify Area 14)
3. **Continue verification** of other unverified dungeon areas:
   - Area 10 (Stormveil) - base 4112 calculated but bypassed
   - Area 12 (Underground) - base 15362 calculated
   - Area 13 (Leyndell/Farum) - base 26612 calculated
   - Area 15 (Haligtree) - base 33362 calculated
   - Area 16 (Volcano Manor dungeon) - base 40517 candidate
