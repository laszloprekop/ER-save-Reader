> **ARCHIVED** (2026-02-08): This document is a point-in-time report and has been archived for historical reference.

# Verification & Viewer Integration Summary

**Date:** 2026-02-01
**Status:** Completed

## Phase 1: Verify High-Coverage Types ✓

### Grace Verification
- **Formula bases validated**: 6 blocks (71800, 72000, 73000, 74000, 76000, 78000)
- **317 proven grace flags** tested against save data
- **Results**: 241/317 (76%) pass, 0 fail, 76 inconclusive
- **Block 78000**: 100% pass rate (18/18)
- **Block 76000**: 87% pass rate (173/199)
- **Inconclusive cases**: Due to padding bytes (0xFF) or slots with different data layouts

### Boss Chain Verification
- **10/10 boss chains validated** (100%)
- Chains validated: Godrick, Rennala, Radahn, Rykard, Morgott, Mohg, Malenia, Maliketh, Hoarah Loux, Radagon
- **4 differential confirmations** found (Rykard, Maliketh, Hoarah Loux, Radagon)
- **6 both-defeated confirmations** (player beat these bosses in both S0 and S1)

### Deliverables
- `/scripts/verification/phase1_verification.py` - Automated verification script
- `/scripts/verification/cases/verification_report_phase1.json` - Results JSON

---

## Phase 2: Audit Viewers for Ground Truth Sync ✓

### Elden Map Viewer (`eventFlagService.ts`)

**Fixes Applied:**

1. **Added DUNGEON_PICKUP_BASES** (lines 114-119)
   ```typescript
   const DUNGEON_PICKUP_BASES: Record<number, number> = {
     10: 6459,    // Stormveil Castle pickups - VERIFIED
     11: 33725,   // Leyndell Royal Capital pickups - VERIFIED
   }
   ```

2. **Expanded BLOCK_BASES** (lines 130-142)
   - Added 61000 (map area visit flags) - base 2671
   - Added 62000 (map fragments) - base 9359
   - Added 65000 (Crystal Tears) - base 37412
   - Added 72000 (DLC graces) - base 2750
   - Added 74000 (DLC dungeon graces) - base 3000
   - Added 78000 (grace guidance) - base 3500

3. **Updated calculateDungeonFlagLocation()** (lines 280-325)
   - Added pickup flag detection (localId >= 7000)
   - Uses DUNGEON_PICKUP_BASES for pickup flags
   - Returns null for unknown pickup areas (prevents false positives)

### Save Editor (`pickup_flags.rs`)
- Already correctly implements `DUNGEON_PICKUP_BASES`
- Uses `VERIFIED_BLOCK_BASES` from ground_truth via build.rs

### Deliverables
- `/scripts/verification/viewer_audit_report.md` - Full audit report with code diffs

---

## Phase 3: World Pickup Verification ✓

### Updates Made

1. **block_items.json** - Updated base offsets to match ground_truth:
   - 62000: 1500 → **9359** (map fragments)
   - 67000: 1764 → **37411** (cookbooks)
   - 68000: 1804 → **37536** (cookbooks continued)

2. **case_cli.py** - Updated `BLOCK_BASE_OFFSETS` dictionary

### Verification Status
- Tile formula verified: base_offset=485330 confirmed
- Smoldering Butterfly anchor flag confirmed at offset 852831, bit 5
- Temporal pairs available in capture catalog (38 pairs)

### Known Gaps
- World pickup verification requires matching specific captures to specific flags
- Capture catalog has flag_id metadata gaps that need enrichment

---

## Phase 4: Dungeon Pickup Verification ✓

### Verified Pickup Bases
| Area | Pickup Base | Status |
|------|-------------|--------|
| 10 (Stormveil) | 6459 | ✓ Verified |
| 11 (Leyndell) | 33725 | ✓ Verified |
| 30 (Catacombs) | ? | Needs discovery |
| 31 (Caves) | ? | Needs discovery |
| 32 (Tunnels) | ? | Needs discovery |

### Dungeon Event Bases (not pickups)
| Area | Event Base | Status |
|------|------------|--------|
| 10 | 4112 | calculated |
| 11 | 8612 | verified |
| 30 | 27411 | **verified** |
| 31 | 28634 | **verified** |
| 32 | 31577 | **verified** |

### Critical Finding
**Dungeon pickups (localId >= 7000) use DIFFERENT bases than dungeon events.**

The Save Editor correctly handles this with `DUNGEON_PICKUP_BASES`. Elden Map now also handles this after Phase 2 fixes.

---

## Files Modified

| File | Changes |
|------|---------|
| `elden-map/server/src/eventFlagService.ts` | Added pickup bases, expanded block bases, fixed dungeon calculation |
| `scripts/verification/block_items.json` | Updated base offsets from ground_truth |
| `scripts/verification/case_cli.py` | Updated BLOCK_BASE_OFFSETS |
| `scripts/verification/phase1_verification.py` | New verification script |
| `scripts/verification/viewer_audit_report.md` | New audit report |

## Files Created

| File | Purpose |
|------|---------|
| `scripts/verification/VERIFICATION_SUMMARY.md` | This summary |
| `scripts/verification/cases/verification_report_phase1.json` | Phase 1 results |

---

## Phase 5: Temporal Verification Deep Dive (2026-02-01)

### Temporal Pair Quality Analysis

From 8 capture pairs with flag IDs, only 3 were "clean" (≤20 total transitions):

| Pair | Flag | Name | Quality | Transitions |
|------|------|------|---------|-------------|
| 102/103 | 16007940 | Ghiza's Wheel | CLEAN | 18 |
| 104/105 | 16007000 | Smithing Stone [6] | CLEAN | 20 |
| 115/116 | 71603 | Prison Town Church | CLEAN | 5 |
| Others | - | - | NOISY | 400-37000 |

### Critical Discovery: Dungeon Pickup Flags

**Dungeon PICKUP flags (local_id >= 7000) use COMPLETELY DIFFERENT offsets than event flags.**

| Flag | Ground Truth Offset | Actual Offset | Status |
|------|---------------------|---------------|--------|
| 16007940 | 41509 | **1533844** | Verified at alternate |
| 16007000 | 41392 | **1534251** | Verified at alternate |

Key findings:
1. Pickup flags are stored in the **tile formula region** (~1.5M offset)
2. There is NO simple linear formula (flags not sequential by local_id)
3. The ground truth `dungeon_formula` only works for local_id < 7000

### Implications for Viewers

Both Elden Map and Save Editor need to:
- **Return NULL/unknown** for dungeon pickup flags (local_id >= 7000) unless we have proven offsets
- Only verified pickup bases: Area 10 (6459) and Area 11 (33725) for event-style pickups
- Area 16 pickups use an unknown non-linear formula in the tile region

### EF Section Structure

The Event Flags section is **1,833,375 bytes** (1.75MB):
- Bytes 0-50,000: Block flags (60000-99999 range)
- Bytes 485,330-1,885,330: Tile flags (10-digit world flags)
- Area 16 pickups appear within tile region

---

## Next Steps (Future Work)

1. **Map dungeon pickup flag locations**
   - Use more temporal pairs to discover individual offsets
   - Create a lookup table for known pickup flags

2. **Discover pickup bases for areas 30, 31, 32**
   - May also require non-linear formulas
   - Focus on event flags (local_id < 7000) first

3. **Improve temporal capture quality**
   - Minimize actions between before/after captures
   - Add flag_id verification to capture workflow

4. **Monitor for false positives**
   - Run verification against new saves
   - Ensure formulas work across different save states
