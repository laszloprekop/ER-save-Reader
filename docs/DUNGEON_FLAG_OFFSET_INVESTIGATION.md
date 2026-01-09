# Dungeon Event Flag Offset Investigation

## Problem Statement

StormveilCastle (and other legacy dungeon) pickup tracking shows incorrect results:
- User reports 2/111 items collected for Confessor character
- The 2 collected items are ones with SHORT 5-digit flags (60xxx, 65xxx range)
- The 107 items with 8-digit dungeon flags (10007xxx) show as uncollected
- User confirmed they have collected items like Godskin Prayerbook (flag 10007990)

## Current Implementation

The `pickup_flags.rs` uses:
- `DUNGEON_BASE_OFFSETS` mapping dungeon area/section to byte offsets
- Formula: `byte_offset = base + local_id / 8`, `bit = 7 - (flag_id % 8)`
- Current Stormveil Castle (10_00) base: **1,383,375**

## Investigation Findings

### 1. BLOCK_BASES Formula (WORKING)

Short flags (5-digit) use BLOCK_BASES and work correctly:
- Grace 71800 (Cave of Knowledge): byte 2725, bit 7 - **VERIFIED**
- Grace 76100 (The First Step): byte 3262, bit 3 - **VERIFIED**
- Whetblade 65610 (Iron Whetblade): byte 1951 - **VERIFIED**

### 2. DUNGEON_BASE_OFFSETS (NOT WORKING)

8-digit dungeon flags use DUNGEON_BASE_OFFSETS:
- Current base for 10_00: 1,383,375
- Tested flag 10007990: byte 1,384,373 - **NOT MATCHING**

### 3. Empirical Search Results

Scanning the save file for correct offsets showed:
- Best matches (8/10 Stormveil flags) occur at base offsets 2, 10, 18, 26...
- These are much lower than the current 1,383,375
- The repeating pattern suggests either test data or a different format

### 4. Game Runtime vs Save File Format

The Grand Archives Cheat Engine table shows:
- Runtime uses a dynamic block-based system with red-black tree
- `mod` (flags per block) is read from game memory, not hardcoded
- Formula: `block = flagId / mod`, `index = flagId % mod`
- This is DIFFERENT from the save file format

### 5. Save File Analysis Issues

The `ER0000-static.sl2` file shows suspicious patterns:
- Event flags section has repeating byte patterns: `ff ff ff 00 00 00 00 ff...`
- This doesn't look like actual gameplay progression data
- May be test/debug data or corrupted

## legacymap.eventflagalloclist Verification

We verified the slot-based formula against the game's allocation file:

| Slot | Map ID | Current Offset | Expected by Formula |
|------|--------|----------------|---------------------|
| 0 | m10_00 (Stormveil) | 1,383,375 | 1,383,375 |
| 1 | m10_01 (Stormveil s1) | 1,384,500 | 1,384,500 |
| 4 | m11_00 (Leyndell) | 1,387,875 | 1,387,875 |
| 23 | m14_00 (Raya Lucaria) | 1,409,250 | 1,409,250 |

**Formula: `offset = 1,383,375 + slot * 1,125`**

The slot-based relationships are CORRECT. The question is whether 1,383,375 is the correct starting point.

## Possible Causes

1. **Wrong Starting Offset**: 1,383,375 may be incorrect for save files
2. **Save File Structure**: The save file may use a different layout than runtime
3. **Test Data**: The static save file may contain debug/test data
4. **Version Mismatch**: Offsets may vary between game versions

## RESOLVED (2026-01-09)

### Root Cause
The DUNGEON_BASE_OFFSETS values (1,383,375+) were derived from **runtime game memory**, not the save file format. The save file uses completely different offsets.

### Solution
Through empirical verification against actual save files:
- **Correct base for Stormveil (10_00): 4112** (not 1,383,375)
- Event flags are at offset **0x1A3F0** within a slot (107,504 bytes from slot start)
- The slot-based formula `offset = 4112 + slot * 1125` is correct

### Verified Flags
| Flag | Item | Byte Offset | Bit | Status |
|------|------|-------------|-----|--------|
| 10007990 | Godskin Prayerbook | 5110 | 1 | ✓ WORKING |
| 10007030 | Furlcalling Finger Remedy | 4990 | 1 | ✓ WORKING |
| 10007040 | Fire Grease | 4992 | 7 | ✓ WORKING |
| 10007110 | Golden Rune [1] | 5000 | 1 | ✓ WORKING |
| 10007200 | Throwing Dagger | 5012 | 7 | ✓ WORKING |
| 10007430 | Arrow x10 | 5040 | 1 | ✓ WORKING |
| 10007550 | Arbalest | 5055 | 1 | ✓ WORKING |

### Files Updated
- `src/db/pickup_flags.rs` - Updated all DUNGEON_BASE_OFFSETS to use correct save file offsets
- `src/save/common/save_slot.rs` - Enabled dynamic event flags detection using grace flag patterns

### Second Issue Discovered: Event Flags Section Offset

The DUNGEON_BASE_OFFSETS fix alone wasn't enough because the save file parser was reading the EventFlags section from the wrong position due to:
1. Variable section sizes not being accounted for
2. A fixed 0x1d gap being used instead of dynamic detection

**Solution**: Enabled dynamic EventFlags offset detection that:
- Searches for known grace flag patterns (Cave of Knowledge, The First Step, etc.)
- Calculates the actual gap size between tutorial_data and event_flags
- Falls back to fixed 0x1d gap if detection fails (score < 2)

## Files Modified

- `scripts/extract_pickup_data.py` - Fixed region categorization
- `src/db/pickup_data.rs` - Regenerated with 111 StormveilCastle items (was 449)
- `scripts/verify_dungeon_offsets.py` - Created for empirical verification

## References

- [The Grand Archives Elden Ring CT](https://github.com/The-Grand-Archives/Elden-Ring-CT-TGA) - Cheat Engine table with event flag code
- `legacymap.eventflagalloclist` - Game's slot-to-map allocation
- `openmap.eventflagalloclist` - Overworld tile allocation
