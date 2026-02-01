# Viewer Ground Truth Audit Report

**Date:** 2026-02-01
**Phase:** 2 - Audit Viewers for Ground Truth Sync

## Overview

This report audits both the Elden Map viewer (`eventFlagService.ts`) and the Save Editor (`pickup_flags.rs`) against the authoritative `ground_truth_offsets.json`.

## 1. Tile Formula

| Parameter | Ground Truth | Elden Map | Save Editor | Status |
|-----------|--------------|-----------|-------------|--------|
| Base Offset | 485330 | 485330 | 485330 | ✓ SYNCED |
| Row Base | 33 | 33 | 33 | ✓ SYNCED |
| Col Base | 30 | 30 | 30 | ✓ SYNCED |
| Bytes Per Slot | 875 | 875 | 875 | ✓ SYNCED |
| Slots Per Row | 40 | 40 | 40 | ✓ SYNCED |
| Max Local ID | 6999 | 6999 | 6999 | ✓ SYNCED |

**Verdict:** ✓ All tile formula parameters are synchronized.

## 2. Block Bases (60000-99999)

### Ground Truth Block Bases
| Block | Base Offset | Status | Notes |
|-------|-------------|--------|-------|
| 60000 | 2548 | verified | Progression flags |
| 61000 | 2671 | verified | Map area visit flags |
| 62000 | 9359 | verified | Map fragments |
| 65000 | 37412 | verified | Crystal Tears |
| 67000 | 37411 | verified | Cookbooks |
| 68000 | 37536 | verified | Cookbooks continued |
| 71000 | 9315 | **unreliable** | Stormveil graces - DO NOT USE |
| 71100 | 2593 | **unreliable** | Leyndell graces - DO NOT USE |
| 71600 | 3198 | **unreliable** | Volcano Manor graces - DO NOT USE |
| 71800 | 2725 | verified | Tutorial graces |
| 72000 | 2750 | verified | DLC graces |
| 73000 | 2662 | verified | Dungeon graces |
| 74000 | 3000 | verified | DLC dungeon graces |
| 75000 | 3125 | **disproven** | DO NOT USE |
| 76000 | 3250 | verified | World graces |
| 77000 | 3373 | **disproven** | DO NOT USE |
| 78000 | 3500 | verified | Grace guidance |

### Elden Map BLOCK_BASES (lines 130-139)
```typescript
const BLOCK_BASES: Record<number, number> = {
  60000: 2548,   // ✓ matches GT
  67000: 37411,  // ✓ matches GT
  68000: 37536,  // ✓ matches GT
  71800: 2725,   // ✓ matches GT
  73000: 2662,   // ✓ matches GT (NOT RELIABLE per GT notes)
  76000: 3250,   // ✓ matches GT
}
```

### Save Editor (via VERIFIED_BLOCK_BASES from build.rs)
- Uses `VERIFIED_BLOCK_BASES` generated from `ground_truth_offsets.json`
- Includes all verified blocks including 61000, 62000, 65000, 72000, 74000, 78000

### Audit Findings

| Block | Ground Truth | Elden Map | Save Editor | Issue |
|-------|--------------|-----------|-------------|-------|
| 61000 | 2671 | ❌ MISSING | ✓ | Elden Map missing map visit flags |
| 62000 | 9359 | ❌ MISSING | ✓ | Elden Map missing map fragments |
| 65000 | 37412 | ❌ MISSING | ✓ | Elden Map missing Crystal Tears |
| 72000 | 2750 | ❌ MISSING | ✓ | Elden Map missing DLC graces |
| 74000 | 3000 | ❌ MISSING | ✓ | Elden Map missing DLC dungeon graces |
| 78000 | 3500 | ❌ MISSING | ✓ | Elden Map missing grace guidance |

**Recommendation:** Add missing blocks to Elden Map's `BLOCK_BASES`:
```typescript
const BLOCK_BASES: Record<number, number> = {
  60000: 2548,   // Progression flags
  61000: 2671,   // Map area visit flags (MISSING)
  62000: 9359,   // Map fragments (MISSING)
  65000: 37412,  // Crystal Tears (MISSING)
  67000: 37411,  // Cookbooks
  68000: 37536,  // Cookbooks continued
  71800: 2725,   // Tutorial graces
  72000: 2750,   // DLC graces (MISSING)
  73000: 2662,   // Dungeon graces
  74000: 3000,   // DLC dungeon graces (MISSING)
  76000: 3250,   // World graces
  78000: 3500,   // Grace guidance (MISSING)
}
```

## 3. Dungeon Base Offsets

### Ground Truth Dungeon Bases
| Area | Base Offset | Status | Notes |
|------|-------------|--------|-------|
| 10 | 4112 | calculated | Stormveil (general events) |
| 11 | 8612 | verified | Leyndell (general events) |
| 12 | 15362 | verified | Underground |
| 13 | 26612 | calculated | Crumbling Farum Azula |
| 14 | 29987 | verified | Subterranean Shunning-Grounds |
| 15 | 33362 | calculated | Miquella's Haligtree |
| 16 | 40517 | likely_correct | Volcano Manor |
| 18 | 43487 | verified | Roundtable Hold |
| 30 | 27411 | **verified** | Catacombs |
| 31 | 28634 | **verified** | Caves |
| 32 | 31577 | **verified** | Tunnels |

### Critical Mismatch: Areas 30, 31, 32

**Problem:** The Elden Map `DUNGEON_BASE_OFFSETS` has entries that mix general event bases with something else.

| Area | Ground Truth | Elden Map | Status |
|------|--------------|-----------|--------|
| 30_00 | 27411 | 27411 | ✓ CORRECT |
| 31_00 | 28634 | 28634 | ✓ CORRECT |
| 32_00 | 31577 | 31577 | ✓ CORRECT |

**Verdict:** ✓ Areas 30, 31, 32 are correctly synced.

## 4. Dungeon PICKUP Bases (local_id >= 7000)

### Ground Truth (dungeon_pickup_bases)
| Area | Pickup Base | Status |
|------|-------------|--------|
| 10 | 6459 | verified |
| 11 | 33725 | verified |
| 30 | UNKNOWN | needs discovery |
| 31 | UNKNOWN | needs discovery |
| 32 | UNKNOWN | needs discovery |

### Elden Map Status
**CRITICAL GAP:** Elden Map does NOT have separate pickup bases for dungeon item pickups.

The `calculateDungeonFlagLocation` function uses `DUNGEON_BASE_OFFSETS` for ALL dungeon flags, including pickups. This is WRONG for local_id >= 7000.

### Save Editor Status
The Save Editor correctly implements `DUNGEON_PICKUP_BASES`:
```rust
pub static DUNGEON_PICKUP_BASES: Lazy<HashMap<u32, u32>> = Lazy::new(|| {
    HashMap::from([
        (10, 6459),   // Stormveil Castle item pickups - VERIFIED
        (11, 33725),  // Leyndell Royal Capital item pickups - VERIFIED
    ])
});
```

And uses them in `calculate_dungeon_flag_offset`:
```rust
if local_id >= 7000 {
    if let Some(&pickup_base) = DUNGEON_PICKUP_BASES.get(&area) {
        // Use pickup base instead of general event base
    }
    return None; // Return None if no pickup base available
}
```

### Recommendation for Elden Map

Add `DUNGEON_PICKUP_BASES` and modify `calculateDungeonFlagLocation`:

```typescript
// Add after DUNGEON_BASE_OFFSETS
const DUNGEON_PICKUP_BASES: Record<number, number> = {
  10: 6459,   // Stormveil Castle pickups
  11: 33725,  // Leyndell pickups
};

// In calculateDungeonFlagLocation, add:
function calculateDungeonFlagLocation(flagId: number): FlagLocation | null {
  const flagIdStr = String(flagId).padStart(8, '0')
  const dungeonArea = parseInt(flagIdStr.slice(0, 2), 10)
  const section = flagIdStr.slice(2, 4)
  const localId = parseInt(flagIdStr.slice(4, 8), 10)

  const bitPosition = 7 - (localId % 8)

  // Check for pickup flags (localId >= 7000)
  if (localId >= 7000) {
    const pickupBase = DUNGEON_PICKUP_BASES[dungeonArea]
    if (pickupBase === undefined) {
      return null // No pickup base available for this area
    }
    const sectionNum = parseInt(section, 10)
    const byteOffset = pickupBase + sectionNum * DUNGEON_SECTION_SIZE + Math.floor(localId / 8)
    if (byteOffset >= 0 && byteOffset < EVENT_FLAGS_SIZE) {
      return [byteOffset, bitPosition]
    }
    return null
  }

  // ... rest of existing logic for general events
}
```

## 5. Midrange Bases (100000-999999)

### Ground Truth
| Block | Base Offset | Status |
|-------|-------------|--------|
| 510000 | 63750 | verified |
| 520000 | 1341 | partial |
| 540000 | 67500 | verified |
| 710000 | 13875 | verified |

### Comparison
| Block | Ground Truth | Elden Map | Save Editor |
|-------|--------------|-----------|-------------|
| 510000 | 63750 | 63750 | ✓ (via GT) |
| 540000 | 67500 | 67500 | ✓ (via GT) |
| 710000 | 13875 | 13875 | ✓ (via GT) |

**Verdict:** ✓ Midrange bases are synchronized.

## Summary of Issues

### Critical
1. **Elden Map missing DUNGEON_PICKUP_BASES** - Will cause false negatives for all dungeon item pickups

### Medium
2. **Elden Map missing several BLOCK_BASES** (61000, 62000, 65000, 72000, 74000, 78000)

### Low
3. None identified

## Recommended Fixes

### Fix 1: Add Dungeon Pickup Bases to Elden Map

```typescript
// Add after line 111 in eventFlagService.ts
const DUNGEON_PICKUP_BASES: Record<number, number> = {
  10: 6459,    // Stormveil Castle pickups - VERIFIED
  11: 33725,   // Leyndell Royal Capital pickups - VERIFIED
  // Areas 30, 31, 32 need discovery - Phase 4
};
```

### Fix 2: Expand BLOCK_BASES in Elden Map

```typescript
// Replace lines 130-139 in eventFlagService.ts
const BLOCK_BASES: Record<number, number> = {
  60000: 2548,   // Progression flags
  61000: 2671,   // Map area visit flags
  62000: 9359,   // Map fragments
  65000: 37412,  // Crystal Tears
  67000: 37411,  // Cookbooks
  68000: 37536,  // Cookbooks continued
  71800: 2725,   // Tutorial graces
  72000: 2750,   // DLC graces (Enir-Ilim)
  73000: 2662,   // Dungeon graces
  74000: 3000,   // DLC dungeon graces
  76000: 3250,   // World graces
  78000: 3500,   // Grace guidance flags
};
```

### Fix 3: Update calculateDungeonFlagLocation for pickups

See detailed code above in Section 4.

## Verification Checklist

- [x] Tile formula parameters match
- [x] Block bases (existing) match
- [ ] Block bases (missing) added to Elden Map
- [ ] Dungeon pickup bases added to Elden Map
- [ ] calculateDungeonFlagLocation updated for pickups
- [ ] Areas 30, 31, 32 pickup bases discovered (Phase 4)
