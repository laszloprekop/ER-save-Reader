# Elden Ring Save File Ground Truth

**Generated**: 2026-01-11
**Verification Method**: Empirical multi-save analysis using `scripts/run_verification.py`
**Primary Data Source**: Decompiled game files + empirical save file testing

---

## Executive Summary

This document is the **single source of truth** for Elden Ring save file parsing and event flag calculations. It supersedes all previous research documents (archived in `docs/archive/`).

### Key Findings

| Category | Status | Details |
|----------|--------|---------|
| **Graces (76xxx)** | **VERIFIED** | Block base=3250, empirically confirmed |
| **Stormveil Graces (71xxx)** | **VERIFIED** | Block base=9315, 8/9 graces matched (CORRECTED 2026-01-22) |
| **Tutorial Graces (71800)** | **VERIFIED** | Block base=2725, validation flags confirmed |
| **Progression (60xxx)** | **VERIFIED** | Block base=2548, cross-validated with 3 items |
| **Map Fragments (62xxx)** | **VERIFIED** | Block base=1500, verified from 62174 match |
| **Cookbooks (67xxx-68xxx)** | **VERIFIED** | Block base=3546 (corrected from 3987!) |
| **Dungeon Graces (73xxx)** | **VERIFIED** | Block base=2664, 13/13 dungeon graces matched |
| **Whetblades (65xxx)** | Unverified | Block base ~1875 needs testing |
| **World Pickups (col >= 30)** | **VERIFIED** | Tile formula works, base=485330 (CORRECTED 2026-01-20) |
| **World Pickups (col < 30)** | Unverified | Western tiles may use different storage |
| **Dungeon Boss Flags (30,31,32)** | **VERIFIED** | Catacombs/Caves/Tunnels bases discovered |
| **Dungeon Boss Flags (Legacy)** | Unverified | Stormveil, Academy, etc. need investigation |
| **Consumable Treasures** | **TRACKABLE** | Via Row ID formula (discovered 2026-02-02) |

---

## Save File Structure

### Constants

```
SLOT_SIZE           = 0x280000 (2,621,440 bytes per character slot)
SLOT_COUNT          = 10 (maximum character slots)
EVENT_FLAGS_SIZE    = 0x1BF99F (1,833,375 bytes per slot)
BND4_HEADER_SIZE    = 0x40 (64 bytes before file entries)
BND4_ENTRY_SIZE     = 0x20 (32 bytes per file entry)
SLOT_CHECKSUM_SIZE  = 16 bytes (MD5 checksum before slot data)
```

### BND4 Container Structure

The save file is a BND4 container with 12 files:
- Files 0-9: Character slots
- Files 10-11: Other data (user profile, etc.)

**Reading slot offsets from BND4 entries:**
```python
# Slot offset is at position 0x10 within each entry (4-byte little-endian)
entry_offset = 0x40 + (slot_index * 0x20) + 0x10
bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
slot_header_offset = bnd4_offset + 16  # Skip 16-byte checksum
```

### Slot Layout (after 16-byte checksum)

```
Offset    | Size      | Content
----------|-----------|------------------------------------------
0x0       | 4         | Version (0xFB = 251 for valid slots)
0x4       | 4         | Map ID
0x20      | Variable  | GaItems (0x1400 max × variable bytes each)
...       | ...       | Other structures (PlayerGameData, Equipment, etc.)
Variable  | 1,833,375 | EventFlags (offset around 0x12B00-0x13800)
```

**Critical**:
1. Slot offsets are **NOT at fixed intervals** - they must be read from BND4 entries
2. Each slot has a 16-byte MD5 checksum header before the actual data
3. EventFlags offset **VARIES** per slot (around 0x12B00-0x13800) due to variable-size GaItems section
4. Use validation flag pattern detection to locate EventFlags reliably

### Event Flags Section

- **Size**: 1,833,375 bytes (15,466,999 flags theoretically)
- **Format**: Bit array, 8 flags per byte
- **Bit Order**: Big-endian style (`bit_position = 7 - (flag_id % 8)`)

---

## Event Flag Formulas

### Block-Based Formula (5-6 digit flags)

For flags in specific ranges, a block-based calculation is used:

```python
block_start = (flag_id // 1000) * 1000
base_offset = BLOCK_BASES[block_start]
relative = flag_id - block_start
byte_offset = base_offset + relative // 8
bit_position = 7 - (flag_id % 8)
```

**Verified Block Bases** (empirically confirmed via diff analysis):

| Block Start | Base Offset | Category | Status | Evidence |
|-------------|-------------|----------|--------|----------|
| 60000 | **2548** | Progression | **Verified** | Cross-validated: 60100 (Crafting Kit), 60130 (Whetstone Knife), 60220 (Furled Finger) |
| 62000 | **1500** | Map Fragments | **Verified** | 62174 (Ailing Village) matched at offset 1521 |
| 67000 | **3546** | Cookbooks | **Verified** | Missionary's Cookbook [4] diff - byte 3549 changed (NOT 3990!) |
| 68000 | 3671 | Cookbooks (continued) | Calculated | 67000 base + 125 |
| 71000 | **9315** | Stormveil Graces | **Verified** | Full search found 8/9 graces at base 9315 (CORRECTED 2026-01-22) |
| 71800 | **2725** | Tutorial Graces | **Verified** | Validation flags 71800, 71801 at byte 2725 |
| 73000 | **2664** | Dungeon Graces | **Verified** | 13/13 catacombs/caves/tunnels matched via slot comparison |
| 76000 | **3250** | World Graces | **Verified** | Validation flags 76100, 76101 |

**IMPORTANT**: Block bases are NOT contiguous! Grace blocks are stored at completely different offsets:
- Block 71000 (Stormveil) at base 9315 is FAR from block 71800 (Tutorial) at base 2725
- The 67xxx range is stored AFTER 76xxx in memory

**Unverified Block Bases** (need empirical testing):

| Block Start | Estimated Offset | Category | Notes |
|-------------|------------------|----------|-------|
| 65000 | ~1875 | Whetblades | Needs diff testing |
| 72000 | ~2750 | Legacy Dungeon Graces | Needs diff testing |
| 74000-75000 | ~3000-3125 | Extended Graces | Needs diff testing |

### Tile-Based Formula (10-digit base game flags)

For flags in format `10XXYYZZZZ` (note: prefix is 2 digits):

```python
# Extract components (prefix is "10", not "1")
row = int(flag_str[2:4])      # XX (tile row, e.g., 43)
col = int(flag_str[4:6])      # YY (tile column, e.g., 50)
local_id = int(flag_str[6:])  # ZZZZ (local flag ID, e.g., 0010)

# Calculate offset (REVERTED 2026-01-25)
base_offset = 485330          # REVERTED: 489981 was WRONG, original 485330 correct
bytes_per_slot = 875
slots_per_row = 40
row_base = 33
col_base = 30

tile_offset = ((row - row_base) * slots_per_row + (col - col_base)) * bytes_per_slot
byte_offset = base_offset + tile_offset + (local_id // 8)
bit_position = 7 - (local_id % 8)  # Uses local_id, not flag_id
```

**Verified Example**: Flag 1043500010 (Smoldering Butterfly at m60_43_50)
- row=43, col=50, local=10
- tile_offset = ((43-33)*40 + (50-30)) * 875 = 420 * 875 = 367500
- byte_offset = 485330 + 367500 + 1 = **852831**
- bit_position = 7 - (10 % 8) = 5
- Extraction: (byte >> 5) & 1
- **Re-verified 2026-01-25**: Observed 0x00→0x20 at offset 852831 (not 857482)**

**LIMITATIONS**:

1. **LocalId >= 7000 is UNTRACKABLE**: Each tile slot has 875 bytes = 7000 flags max. ItemLotParam `eventFlagId` often creates localId 7300+ for treasures - these flags have no storage space.
2. **Col < 30 may not work**: Tiles west of col_base=30 may use different storage region (needs empirical verification).

### Dungeon Formula (8-digit flags)

For flags in format `AASSZZZZ`:

```python
map_area = int(flag_str[0:2])   # AA (dungeon area code)
section = int(flag_str[2:4])     # SS (section within dungeon)
local_id = int(flag_str[4:8])    # ZZZZ (local flag ID)

# Area-specific base offset
base_offset = DUNGEON_BASES[map_area]
section_offset = section * 1125
byte_offset = base_offset + section_offset + (local_id // 8)
bit_position = 7 - (local_id % 8)
```

**Verified Dungeon Bases** (minor dungeons, 2026-01-12):

| Area | Base Offset | Category | Status | Evidence |
|------|-------------|----------|--------|----------|
| 30 | **27411** | Catacombs | **Verified** | 5 boss flags matched via slot comparison |
| 31 | **28634** | Caves | **Verified** | 5 boss flags matched via slot comparison |
| 32 | **31577** | Tunnels | **Verified** | 4 boss flags matched via slot comparison |

**Unverified Dungeon Bases** (legacy dungeons):

| Area | Category | Notes |
|------|----------|-------|
| 10 | Stormveil Castle | Needs verification |
| 14 | Academy of Raya Lucaria | Needs verification |
| 16 | Volcano Manor | Needs verification |
| 35 | Mohgwyn Palace | Needs verification |

---

## Validation Flags (Anchors)

These flags are **100% reliable** for detecting the EventFlags section:

| Flag ID | Byte Offset | Bit | Name | Notes |
|---------|-------------|-----|------|-------|
| 71800 | 2725 | 7 | Cave of Knowledge | Tutorial grace |
| 71801 | 2725 | 6 | Stranded Graveyard | Tutorial grace |
| 76100 | 3262 | 3 | The First Step | First world grace |
| 76101 | 3262 | 2 | Church of Elleh | Early world grace |

Use these to validate EventFlags offset detection.

---

## Known Limitations

### Consumable Treasures (Row ID Formula)

Consumables with `getItemFlagId` creating localId >= 7000 were previously thought to be untrackable via tile formula. However, the **Row ID formula** (discovered 2026-02-02) tracks these pickups using a separate bitfield.

See [EVENT-FLAG-GEOGRAPHY.md](EVENT-FLAG-GEOGRAPHY.md#row-id-tracking-for-world-pickups-critical-discovery-2026-02-02) for the Row ID formula details.

**Summary**: World pickups with localId >= 7000 ARE tracked via `row_id = getItemFlagId - 7000` using a dedicated row ID bitfield (base: 1037373320).

---

## Verification Data

### Empirically Verified Formulas (as of 2026-01-11)

| Formula | Category | Status | Evidence |
|---------|----------|--------|----------|
| Block 60000 | Progression | **VERIFIED** | Cross-validated with 60100, 60130, 60220 (base=2548) |
| Block 62000 | Map Fragments | **VERIFIED** | 62174 (Ailing Village) matched at offset 1521 (base=1500) |
| Block 67000 | Cookbooks | **VERIFIED** | Missionary's Cookbook [4] pickup diff (base=3546, NOT 3987!) |
| Block 71000 | Stormveil Graces | **VERIFIED** | 8/9 graces matched (base=9315, CORRECTED from 2625 on 2026-01-22) |
| Block 71800 | Tutorial Graces | **VERIFIED** | Validation flags 71800, 71801 (base=2725) |
| Block 76000 | World Graces | **VERIFIED** | Validation flags 76100, 76101 (base=3250), 65% match rate |
| Block 73000 | Dungeon Graces | **VERIFIED** | 13/13 dungeon graces matched via slot comparison (base=2664) |
| Block 78000 | POI Flags | UNVERIFIED | 0% match rate - base offset needs discovery |
| Tile (col >= 30) | World Pickups | **VERIFIED** | Smoldering Butterfly (1043500010) temporal diff, base=485330 |
| Tile (col < 30) | World Pickups | UNVERIFIED | Western tiles may use different storage |
| Dungeon Area 30 | Catacombs | **VERIFIED** | 5 boss flags matched (base=27411) |
| Dungeon Area 31 | Caves | **VERIFIED** | 5 boss flags matched (base=28634) |
| Dungeon Area 32 | Tunnels | **VERIFIED** | 4 boss flags matched (base=31577) |
| Dungeon (Legacy) | Stormveil, Academy, etc. | UNVERIFIED | Base offsets need discovery |

**Note on EventFlags offset**: The Rust ER-Save-Editor uses a fixed offset of 0x1a104, but this is incorrect for our test saves. The actual offset varies per slot (0x12B00-0x13800) depending on GaItems count. Use validation flag detection to find the correct offset.

### Verification Methodology

1. **Diff Analysis**: Compare before/after save snapshots to find exact byte changes
2. **Reverse Calculation**: From changed byte/bit, derive flag ID
3. **Forward Verification**: Use derived base to predict new flag offsets
4. **Cross-Validation**: Test against multiple save files and characters

---

## Files Reference

### Primary Sources (Decompiled Game Data)

| File | Location | Content |
|------|----------|---------|
| ItemLotParam_map.param.xml | regulation-bin/ | World pickup definitions |
| BonfireWarpParam.param.xml | regulation-bin/ | Grace site data |
| ShopLineupParam.param.xml | regulation-bin/ | Shop item flags |
| common.emevd.js | event/ | Event scripts with flag logic |

### Verification Tools

| Script | Purpose |
|--------|---------|
| `scripts/run_verification.py` | Main verification runner |
| `scripts/verification/save_parser.py` | Save file parsing |
| `scripts/verification/ground_truth_loader.py` | Loads formulas from ground_truth_offsets.json |
| `scripts/verification/diff_analyzer.py` | Before/after comparison |

### Output Files

| File | Content |
|------|---------|
| `ground_truth_offsets.json` | Verified flag offsets and formulas |
| `scripts/extracted_event_flags.json` | All known flags from game files |

---

## Archived Documents

Previous research documents have been moved to `docs/archive/`:

- `SAVE_FILE_PARSING_RESEARCH.md`
- `EVENT_FLAG_OFFSET_INVESTIGATION.md`
- `SAVE_FILE_SCHEMA based on ER-Save-Editor.md`
- `SAVE-FILE-ANATOMY.md`
- `SAVE-FILE-STRUCTURE-RESEARCH.md`
- `EVENT-FLAG-SYSTEM-ANALYSIS.md`

These contain historical research that informed this ground truth but may have outdated or incorrect information.

---

## Usage Example

```python
from verification import SaveParser, FlagFormulas

# Parse save file
parser = SaveParser()
save = parser.parse("ER0000.sl2")

# Check a specific flag
formulas = FlagFormulas()
flag_id = 76100  # The First Step grace

results = formulas.calculate_offset(flag_id)
if "block" in results and results["block"].is_valid:
    offset = results["block"].byte_offset
    bit = results["block"].bit_position

    for slot in save.slots:
        is_set = parser.check_flag_at_offset(slot.event_flags, offset, bit)
        print(f"Slot {slot.slot_index}: {'SET' if is_set else 'NOT SET'}")
```

---

## Contributing

To improve the ground truth:

1. Run `python scripts/run_verification.py --verbose`
2. Review mismatches and unverified flags
3. Use `diff_analyzer.py` with before/after saves to discover correct offsets
4. Update `ground_truth_offsets.json` with corrected base offsets (NOT `flag_formulas.py`, which is deprecated)
5. Re-run verification to confirm improvements

---

## Changelog

### 2026-01-25
- **REVERT** tile formula base_offset: 489981 → **485330** (reverted to original)
- The 2026-01-20 "correction" to 489981 was WRONG - offset 857482 showed no change during pickup
- Re-verified: Smoldering Butterfly (1043500010) at byte **852831** bit 5 (0x00→0x20)
- Added calibration_anchors section to ground_truth_offsets.json for runtime validation
- **Final value: 485330** (confirmed in `crates/wasm-event-flags/src/lib.rs`)

### 2026-01-12
- **VERIFIED** Dungeon formula bases for minor dungeons:
  - Area 30 (Catacombs): base=**27411** (5 boss flags matched)
  - Area 31 (Caves): base=**28634** (5 boss flags matched)
  - Area 32 (Tunnels): base=**31577** (4 boss flags matched)
- Formula: `byte = base + section * 1125 + local_id // 8`

### 2026-01-11 (Late Evening Update)
- **VERIFIED** 73xxx dungeon graces base: **2664** (13/13 catacombs/caves/tunnels matched via slot comparison)
- Previous investigation was looking at wrong byte range (2875 vs 2664)

### 2026-01-11 (Evening Update)
- **CORRECTED** cookbook base: 3987 → **3546** (verified via Missionary's Cookbook [4] diff)
- **VERIFIED** 60xxx progression base: **2548** (cross-validated with Crafting Kit, Whetstone Knife, Furled Finger)
- **VERIFIED** 62xxx map fragment base: **1500** (verified from 62174 Ailing Village match)

### 2026-01-11 (Initial)
- Created ground truth document
- Verified 71xxx (base=2625), 76xxx (base=3250) from validation flags
- Verified tile formula for col >= 30 (base=485330, col_base=30) - LATER CORRECTED

---

*Last updated: 2026-02-08*
