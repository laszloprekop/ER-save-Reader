# Elden Ring Save File Parsing Research

## Executive Summary

This document details comprehensive research on Elden Ring save file parsing, including verified structures, identified anomalies, and recommendations for improving parsing robustness.

**Key Findings:**
1. **Parser is fundamentally correct** - JSON exports show accurate grace/boss/event data
2. **Dual offset systems cause confusion** - EVENT_FLAGS database vs pickup_flags.rs formulas
3. **Variable-length structures** complicate offset calculations
4. **Pre-event flags gap** is hardcoded (0x1d) but may vary between saves

---

## Verified Save File Structure

### Overall File Layout (PC)

```
Offset      Size        Description
------      ----        -----------
0x000       0x300       BND4 Header (file table, metadata)
0x300       0x10        Slot 0 MD5 Checksum
0x310       0x280000    Slot 0 SaveSlot data
0x280310    0x10        Slot 1 MD5 Checksum
0x280320    0x280000    Slot 1 SaveSlot data
...                     (10 slots total)
0x1900310   0x60010     UserData10 (profile summaries, settings)
0x1960320   variable    UserData11 (regulation bin, network data)
```

### Magic Numbers Verified

| Constant | Value | Description |
|----------|-------|-------------|
| PC Header Magic | `BND4` (0x424E4434) | PC save file signature |
| Slot Size | 0x280000 | 2,621,440 bytes per character |
| Event Flags Size | 0x1BF99F | 1,833,375 bytes (14,666,999 flag bits) |
| Slot Count | 10 | Character slots |

### SaveSlot Internal Structure (Sequential Reading)

```
Field                       Size (bytes)    Notes
-----                       -----           -----
ver                         4               Version number (0xFB for current)
map_id                      4               Current map identifier
_0x18                       24              Unknown padding
ga_items[0x1400]            VARIABLE        Equipment database (8-21 bytes each)
player_game_data            ~150            Stats, name, passwords
_0xd0                       208             Unknown
equip_data                  84              Equipment slot indices
chr_asm                     104             Character assembly 1
chr_asm2                    104             Character assembly 2
equip_inventory_data        VARIABLE        Inventory (count-based)
equip_magic_data            196             Spell slots
equip_item_data             104             Item equipment
equip_gesture_data          24              Gesture bindings
equip_projectile_data       VARIABLE        Arrow/bolt data
equipped_items              136             Current equipment
equip_physics_data          8               Physics data
_0x4                        4               Unknown
_face_data                  303             Character appearance
storage_inventory_data      VARIABLE        Storage box
gesture_game_data           256             Gesture states
regions                     VARIABLE        Discovered locations
ride_game_data              40              Horse data
various gaps                ~100            Small padding sections
_menu_profile_save_load     4104            Menu settings
_trophy_equip_data          52              Trophy data
ga_item_data                0x6E28          Item tracking (0x1B58 items)
_tutorial_data              1032            Tutorial flags
_pre_event_flags_gap        29 (0x1d)       Gap before EventFlags (VARIABLE!)
event_flags                 0x1BF99F        Event flag bitvector
_0x1_1                      1               Unknown
_unk_lists[5]               VARIABLE        Unknown lists
player_coords               65              Position/map data
_game_man_unknown           15              Unknown
_0x1_2                      4               Slot active marker (0x02)
_cs_net_data_chunks         0x20000         Network/multiplayer data
world_area_weather          12              Weather state
world_area_time             12              Time of day
_0x10_1                     16              Unknown
steam_id                    8               PC account ID
_cs_ps5_activity            32              PS5 activity
_cs_dlc                     50              DLC flags
_0x80                       128             Unknown
_rest                       VARIABLE        Padding to 0x280000
```

---

## Event Flags System Analysis

### Two Offset Calculation Methods

The codebase uses **two different** methods for calculating event flag positions:

#### 1. EVENT_FLAGS Database (src/db/event_flags.rs)

Pre-calculated `(byte_offset, bit_position)` tuples for 5,751 flags:

```rust
pub static EVENT_FLAGS: Lazy<Mutex<HashMap<u32,(u32,u8)>>> = Lazy::new(|| {
    Mutex::new(HashMap::from([
        (71800, (0xaa5, 7)),  // Cave of Knowledge grace
        (71801, (0xaa5, 6)),  // Stranded Graveyard grace
        (76100, (0xcbe, 3)),  // The First Step grace
        (76101, (0xcbe, 2)),  // Church of Elleh grace
        (68030, (0x8cd, 1)),  // Cookbook flag
        // ... 5,746 more entries
    ]))
});
```

**Used by:** EventsViewModel for graces, bosses, whetblades, cookbooks, maps

#### 2. Formula-based Calculation (src/db/pickup_flags.rs)

Dynamic calculation using block bases and tile formulas:

```rust
// Block bases for flags 60000-99999
pub static BLOCK_BASES: Lazy<HashMap<u32, u32>> = Lazy::new(|| {
    HashMap::from([
        (60000, 1250),   (62000, 1500),   (65000, 1875),
        (66000, 2000),   (67000, 2125),   (68000, 2250),
        (69000, 2375),   (71000, 2625),   (73000, 2875),
        (76000, 3250),
    ])
});

// For flag 68030: byte = 2250 + (30/8) = 2253, bit = 7 - (30%8) = 1
```

**Used by:** World pickups, is_flag_set() function

### Verification: Both Methods Agree

For tested flags, both methods produce identical offsets:

| Flag ID | EVENT_FLAGS Offset | BLOCK_BASES Calculation | Match |
|---------|-------------------|------------------------|-------|
| 68000   | 0x8CA (2250)      | 2250 + 0/8 = 2250      | ✓     |
| 68030   | 0x8CD (2253)      | 2250 + 30/8 = 2253     | ✓     |
| 71800   | 0xAA5 (2725)      | 2625 + 800/8 = 2725    | ✓     |
| 76100   | 0xCBE (3262)      | 3250 + 100/8 = 3262    | ✓     |

---

## Anomalies and Issues Identified

### Issue 1: Variable-Length ga_items Structure

The `ga_items` array contains 0x1400 (5120) items with variable sizes:

| Item Type | Size (bytes) | Condition |
|-----------|--------------|-----------|
| Empty     | 8            | item_id == 0 |
| Weapon    | 21           | (item_id & 0xF0000000) == 0 |
| Armor     | 16           | (item_id & 0xF0000000) == 0x10000000 |

**Impact:** Cannot predict exact EventFlags offset without parsing

**Measured Range:**
- Minimum (all empty): 5120 × 8 = 40,960 bytes
- Maximum (all weapons): 5120 × 21 = 107,520 bytes
- Typical mid-game: ~45,000-60,000 bytes

### Issue 2: _pre_event_flags_gap Hardcoded

```rust:src/save/common/save_slot.rs
// Line 1719
save_slot._pre_event_flags_gap = br.read_bytes(0x1d)?;  // Fixed 29 bytes
```

**Comment in code says:** "Variable-length gap before EventFlags (was fixed 0x1d, but varies per character)"

**Observation:** Event flags detection code exists (`event_flags_detection.rs`) but is disabled.

### Issue 3: Massive Byte Changes Between Snapshots

Comparison of before/after pickup snapshots shows:
- **734,791 bytes differ** between captures
- Expected: Only ~1-5 bytes for a single flag change

**Likely causes:**
1. Player position/coordinates update every save
2. Timestamp fields
3. Network/multiplayer state (0x20000 bytes)
4. Possible encryption/obfuscation in some sections

### Issue 4: Grace Flag Pattern Matches at Wrong Positions

Searching for validation flag patterns (graces 71800/71801/76100/76101):
- Found 21 perfect 4/4 matches in first 0x1200 bytes of slot
- These are in the `ga_items` section, not EventFlags
- False positives due to coincidental byte patterns

---

## Recommended Improvements

### 1. Dynamic EventFlags Detection (Priority: High)

Re-enable and improve `event_flags_detection.rs`:

```rust
/// Detection algorithm:
/// 1. Search from expected position (after known structures)
/// 2. Use validation flags that are ALWAYS set for any character:
///    - Flag 71800 (Cave of Knowledge) - Tutorial grace
///    - Flag 60000 (base progression flag)
/// 3. Verify structural constraints:
///    - EF position + EF size <= slot size
///    - Remaining bytes match expected post-EF structures
```

### 2. Add Debug/Trace Logging

Add position tracking during parsing:

```rust
impl Read for SaveSlot {
    fn read(br: &mut BinaryReader) -> Result<Self, io::Error> {
        let start_pos = br.pos;
        // After each read:
        tracing::debug!("After {}: position = 0x{:X}", field_name, br.pos);
    }
}
```

### 3. Unify Flag Offset Calculation

Either:
- **Option A:** Expand EVENT_FLAGS database to include all ~15,000 flags
- **Option B:** Generate EVENT_FLAGS from pickup_flags.rs formulas at build time
- **Option C:** Use pickup_flags.rs as single source, remove hardcoded EVENT_FLAGS

### 4. Add Structural Validation

After parsing, verify:

```rust
fn validate_slot(slot: &SaveSlot) -> Result<(), ValidationError> {
    // Check known invariants
    assert!(slot.ver >= 0xFB, "Invalid version");
    assert!(slot.event_flags.flags.len() == 0x1BF99F);

    // Verify a known-always-set flag
    let cave_of_knowledge = is_flag_set(&slot.event_flags.flags, 71800);
    // (All characters have passed tutorial)
}
```

### 5. Document Variable-Length Sections

Create a structure map that tracks:
- Which sections are variable
- How to calculate their sizes
- Dependencies between sections

---

## Test Verification Results

### Grace Flags: ✓ Verified Working

- JSON export correctly shows discovered graces
- EventFlags at bytes 0xAA5 and 0xCBE contain correct values
- Both Cave of Knowledge and First Step graces detected

### Cookbook Flags: ✓ Verified Working

- Missionary's Cookbook [1-7] all show in export
- Discovered status correctly reflects in-game state

### World Pickups: Partially Verified

- `is_flag_set()` function uses correct calculation
- Some edge cases with 10-digit tile flags may need verification

---

## Data Sources Reference

### Primary (Decompiled Game Files)

| File | Content | Flags Covered |
|------|---------|---------------|
| ItemLotParam_map.param.xml | World pickup flags | 10XXYYZZZZ format |
| ShopLineupParam.param.xml | Shop stock/release flags | 60xxx, 100xxx, 150xxx |
| common.emevd.js | Event script logic | Flag relationships |

### Generated (Event Flags Database)

| Source | Flags | Coverage |
|--------|-------|----------|
| event_flags.rs | 5,751 | Graces, bosses, cookbooks, etc. |
| pickup_data.rs | ~4,500 | World pickups |
| pickup_flags.rs formulas | ~15,000+ | All calculable flags |

---

## Conclusion

The save file parser is **fundamentally sound** but has **areas for improvement**:

1. **Working well:** Basic structure parsing, event flag reading for database flags
2. **Needs attention:** Variable-length structure handling, EventFlags position detection
3. **Future work:** Comprehensive flag database, validation framework

The dual offset calculation systems (EVENT_FLAGS database vs pickup_flags.rs formulas) are mathematically consistent, but maintaining two sources creates confusion and potential for drift.
