# Event Flag Geography and Hierarchy

This document describes how Elden Ring event flags are organized geographically and hierarchically, how flags chain together for quests and unlocks, and which game files are the authoritative sources.

---

## World Hierarchy

```
Lands Between (World)
├── Overworld Regions (Major Geographic Areas)
│   ├── Sub-regions (Named Areas within Regions)
│   │   ├── Sites of Grace (76XXX-79XXX block flags)
│   │   ├── Landmarks/POIs (62XXX block flags)
│   │   └── World Pickups (1XXYYZZZZ tile flags - 10 digits)
│   │
│   ├── Legacy Dungeons (AASSZZZZ dungeon flags - 8 digits)
│   │   ├── Stormveil Castle (Area 10)
│   │   ├── Raya Lucaria Academy (Area 11)
│   │   ├── Leyndell, Royal Capital (Area 13)
│   │   ├── Haligtree (Area 15)
│   │   └── Farum Azula (Area 16)
│   │
│   └── Minor Dungeons (AASSZZZZ dungeon flags - 8 digits)
│       ├── Catacombs (Area 30)
│       ├── Caves (Area 31)
│       ├── Tunnels (Area 32)
│       └── Hero's Graves / Divine Towers (Area 33-34)
│
├── Underground Regions (Area 12, 35, 39)
│   ├── Siofra River / Ainsel River (Area 12)
│   ├── Mohgwyn Palace (Area 35)
│   └── Deeproot Depths (Area 39)
│
├── Special Areas
│   ├── Tutorial Areas (Area 14) - Chapel of Anticipation, Cave of Knowledge, Stranded Graveyard
│   ├── Roundtable Hold (Area 18)
│   ├── Elden Throne (Area 19) - Final boss area
│   └── Area 20 - Needs investigation (may be unused)
│
└── DLC Regions (Shadow of the Erdtree)
    └── World Pickups (2XXYYZZZZ tile flags - 10 digits)
```

### Flag Format Summary

| Format | Digits | Example | Used For |
|--------|--------|---------|----------|
| Block | 5-6 | `76100` | Graces, landmarks, progression |
| Dungeon | 8 | `14000080` | Legacy dungeons, minor dungeons, special areas |
| Tile | 10 | `1042370000` | Overworld pickups (base game: 1XXX, DLC: 2XXX) |

---

## Geographic Flag Groupings

### 1. Overworld Tile System (10-digit flags)

World pickup flags use a tile-based coordinate system:

**Format**: `1XXYYZZZZ` (base game) or `2XXYYZZZZ` (DLC)

| Component | Meaning | Range |
|-----------|---------|-------|
| `1` or `2` | Base game or DLC prefix | 1=base, 2=DLC |
| `XX` | Tile X coordinate | 33-54 (base game) |
| `YY` | Tile Y coordinate | 31-58 (base game) |
| `ZZZZ` | Local flag index within tile | 0000-9999 |

**Tile to Region Mapping**:

| Tile Range | Region |
|------------|--------|
| X:42-44, Y:36-40 | Limgrave |
| X:40-43, Y:33-35 | Weeping Peninsula |
| X:37-44, Y:41-47 | Liurnia of the Lakes |
| X:37-44, Y:48-52 | Altus Plateau |
| X:33-38, Y:48-52 | Mt. Gelmir |
| X:46-54, Y:36-44 | Caelid |
| X:48-54, Y:45-50 | Greyoll's Dragonbarrow |
| X:37-44, Y:53-58 | Mountaintops of the Giants |
| X:33-38, Y:55-58 | Consecrated Snowfield |

**Source File**: `openmap.eventflagalloclist`

### 2. Block-Based Flags (5-6 digit flags)

Flags in ranges 60000-99999 are organized into 1000-flag blocks that share a base byte offset:

**Offset Formula**:
```
byte_offset = block_base_offset + (flag_id - block_start) / 8
bit_position = 7 - (flag_id % 8)
```

| Block Range | Category | Base Offset (hex) |
|-------------|----------|-------------------|
| 60000-60999 | Progression | 0x4ec |
| 62000-62999 | Map/Landmarks | 0x5dc |
| 65000-65999 | Whetblades | 0x694 |
| 66000-66999 | Pot Upgrades | 0x6bc |
| 67000-67999 | Cookbooks | 0x6e4 |
| 68000-68999 | Cookbooks | 0x70c |
| 69000-69999 | Remembrance | 0x734 |
| 76000-76999 | Graces (partial) | 0xcb2 |
| 91000-91999 | Boss Remembrance | 0x950 |
| 92000-92999 | Container Upgrades | 0x978 |

**Source File**: Derived from `common.emevd.js` event definitions

### 3. Dungeon & Area Flags (8-digit flags)

Dungeons and special areas use an 8-digit format:

**Format**: `AASSZZZZ`

| Component | Meaning | Range |
|-----------|---------|-------|
| `AA` | Area ID | 10-39 |
| `SS` | Section within area | 00-22 |
| `ZZZZ` | Local flag index | 0000-9999 |

#### Legacy Dungeons (Major Story Areas)

| Area | Name | Event Base | Pickup Base* | Status |
|------|------|------------|--------------|--------|
| 10 | Stormveil Castle | 4112 | 6459 | Pickup base verified |
| 11 | Leyndell, Royal Capital | 8612 | 33725 | Pickup base verified |
| 13 | Crumbling Farum Azula | 26612 | - | Unverified |
| 14 | Academy of Raya Lucaria | 29987 | - | Event base verified |
| 15 | Miquella's Haligtree | 33362 | - | Unverified |
| 16 | Volcano Manor | 40517 | - | Event base verified |

*Pickup bases are for item pickups (local_id >= 7000). See "Dungeon Pickup Bases" section below.

#### Minor Dungeons

| Area | Name | Base Offset | Status |
|------|------|-------------|--------|
| 12 | Underground (Siofra, Ainsel, etc.) | - | Unverified |
| 30 | Catacombs | 27411 | Verified |
| 31 | Caves | 28634 | Verified |
| 32 | Tunnels | 31577 | Verified |
| 34 | Divine Towers | - | Unverified |

#### Special Areas

| Area | Name | Base Offset | Status |
|------|------|-------------|--------|
| 14 | **Tutorial Areas** (Chapel, Cave of Knowledge, Stranded Graveyard) | 29987 | Verified |
| 18 | Roundtable Hold | 43487 | Verified |
| 19 | Chapel of Anticipation (per code) | 46862 | Needs Review |
| 20 | Stranded Graveyard (per code) | 50237 | Needs Review |
| 35 | Mohgwyn Palace | - | Unverified |
| 39 | Deeproot Depths / Elden Throne | - | Unverified |

**IMPORTANT**: Empirical testing (Slot 6 Chapel, Slot 1 Cave) shows tutorial events write to Area 14 offset (29987), NOT Areas 19/20. The areas 19/20 offsets from pickup_flags.rs may be unused or for different events.

**Note**: The Grafted Scion boss uses flag 10010800 (Area 10 format).

**Section Size**: 1125 bytes per section

**Offset Formula**:
```
byte_offset = area_base_offset + section * 1125 + local_id / 8
bit_position = 7 - (flag_id % 8)
```

**Source File**: `legacymap.eventflagalloclist`

#### Dungeon Pickup Bases (IMPORTANT DISCOVERY 2026-01-23)

**Item pickup flags (local_id >= 7000) use DIFFERENT bases than general dungeon events.**

The general dungeon event bases work for graces, boss defeats, etc. (local_id 0-999).
But item pickups use separate "pickup bases" that must be empirically discovered:

| Area | General Event Base | Item Pickup Base | Verification |
|------|-------------------|------------------|--------------|
| 10 (Stormveil) | 4112 | **6459** | 11/11 flags verified |
| 11 (Leyndell) | 8612 | **33725** | 5/5 flags verified |

**Formula for item pickups (local_id >= 7000)**:
```
byte_offset = pickup_base + section * 1125 + local_id / 8
bit_position = 7 - (flag_id % 8)
```

**Note**: No consistent offset pattern found between general and pickup bases. Each area requires empirical verification using known inventory items.

---

## World Pickup Event Flags (CRITICAL DISCOVERY 2026-01-23)

### The Row ID Discovery

**Key Finding**: For tile-based world pickups, the game stores the **row_id** as the event flag, NOT the `getItemFlagId` field.

ItemLotParam_map has two related values:

| Field | Example | Local ID | Stored? |
|-------|---------|----------|---------|
| Row ID (item lot) | `1044360310` | 0310 | ✅ **YES** |
| getItemFlagId | `1044367310` | 7310 | ❌ No |

**Evidence**: Save file diff analysis showed that when picking up treasure at row_id `1044360310`:
- Flag `1044360310` (local_id 310) was SET in the save file
- Flag `1044367310` (getItemFlagId, local_id 7310) was NOT used

### The +7000 Offset Red Herring

The `getItemFlagId` field is always `row_id + 7000`, placing the local_id in the 7000+ range. This initially seemed problematic because:

- Tile slots allocate only **875 bytes** (7000 flags, local_id 0-6999)
- Flags with local_id >= 7000 would have **NO STORAGE**

However, the game bypasses this by using the row_id directly for storage, which has local_id in the 0-999 range (storable).

### What getItemFlagId Is Actually For

The `getItemFlagId` field appears to be used for:
1. **Runtime checks** - In-game scripts checking if an item was picked up
2. **Shop unlock conditions** - `eventFlag_forRelease` in ShopLineupParam references these
3. **NPC dialogue triggers** - Quest progression checks

But for **persistent save file storage**, the game uses the row_id.

### Implications for Flag Database

Our flag extraction now correctly uses:
- **Row ID** for tile-based pickups (10-digit flags starting with 1 or 2)
- **getItemFlagId** for non-tile pickups (dungeons, etc.) which may follow different rules

### Code Changes

The `extract_item_lot_flags` function in `src/discovery/param_flags.rs` was updated to:

```rust
// For tile-based world pickups (1B-3B range), use row_id as the flag
let is_tile_based = row_id >= 1_000_000_000 && row_id < 3_000_000_000;

if is_tile_based {
    // Use row_id as the actual stored flag
    add_flag_source(flags, row_id, ParamSource::ItemLotMap { row_id, field: "row_id" });
} else {
    // Use getItemFlagId for non-tile pickups
    add_flag_source(flags, flag_id, ParamSource::ItemLotMap { row_id, field: "getItemFlagId" });
}
```

This ensures the flag database contains the **actually storable** flag IDs that can be verified in save files.

### Block Flags for World Pickups

These 76 special items use block flags that ARE stored:

| Range | Items | Purpose |
|-------|-------|---------|
| 60xxx | 10 | Keys, medallions (Haligtree, Rold) |
| 62xxx | 14 | Map fragments |
| 65xxx | 13 | Whetblades |
| 66xxx | 13 | Pot/Perfume upgrades |
| 67xxx-68xxx | 25 | Cookbooks |
| 69xxx | 1 | Notes |

### Implications for Save Editing

1. **Don't track most world pickups via event flags** - use inventory checks instead
2. **Block flags work** - cookbooks, whetblades, etc. can be read/written
3. **Tile flags with local_id 7000+** - these are effectively phantom flags

---

## Flag Category Ranges

### Geographic Discovery Flags

| Range | Category | Description |
|-------|----------|-------------|
| 62010-62084 | Map Fragments | Unlocks map regions |
| 62100-62999 | Landmarks | Points of interest discovered |
| 63010-63084 | Map Discovery | Internal tracking (linked to 62xxx) |
| 76000-79999 | Sites of Grace | Grace discovery and rest status |
| 78000-78999 | Stakes of Marika | Respawn point discovery |

### Landmark Flag Ranges by Region

| Range | Region |
|-------|--------|
| 62100-62138 | Limgrave |
| 62150-62184 | Weeping Peninsula |
| 62200-62284 | Liurnia of the Lakes |
| 62300-62348 | Altus Plateau |
| 62350-62389 | Mt. Gelmir |
| 62400-62438 | Caelid |
| 62460-62475 | Greyoll's Dragonbarrow |
| 62510-62531 | Mountaintops of the Giants |
| 62550-62574 | Consecrated Snowfield |
| 62610-62634 | Siofra River |
| 62640-62640 | Ainsel River |
| 62700-62740 | Deeproot Depths |
| 62800-62831 | Mohgwyn Palace |
| 62840-62844 | Lake of Rot |
| 62850-62891 | Nokron / Nokstella |
| 62900-62943 | Leyndell |
| 62950-62981 | Crumbling Farum Azula |

---

## Flag Chaining Systems

### 1. Quest Event Chains

Quests use sequential flags where each step enables the next:

**Example: Ranni's Questline**

```
[Initial Meeting]
1042360730 (Renna the Witch met at Church of Elleh)
    ↓ enables
[Quest Start]
10009616 (Ranni's Rise - spoke to Ranni)
    ↓ enables
[Quest Steps]
11109xxx (Carian Study Hall events)
    ↓ enables
[Completion]
1050389xxx (Moonlight Altar access)
```

**Source Files**:
- `common.emevd.js` - Event script logic
- Individual area `.emevd.js` files

### 2. Area Unlock Chains

Certain flags must be set before areas become accessible:

**Example: Leyndell Access**

```
[Prerequisites]
171 OR 172 (Defeat Radahn OR Defeat Rykard)
    ↓ enables
[Great Rune Activation]
180-187 (Activate 2+ Great Runes)
    ↓ enables
[Capital Access]
13000xxx (Leyndell area flags become active)
```

**Example: Haligtree Access**

```
[Medallion Halves]
60430 (Haligtree Secret Medallion - Left)
    +
60431 (Haligtree Secret Medallion - Right)
    ↓ enables
[Lift Access]
62550+ (Consecrated Snowfield landmarks)
    ↓ enables
[Haligtree]
15000xxx (Haligtree area flags)
```

### 3. Merchant Purchase Chains

Shop items use two flag types:

| Flag Type | Field | Purpose |
|-----------|-------|---------|
| Stock Flag | `eventFlag_forStock` | Set when item purchased (depletes stock) |
| Release Flag | `eventFlag_forRelease` | Must be ON for item to appear |

**Stock Flag Ranges**:

| Range | Category |
|-------|----------|
| 60xxx | General progression items |
| 66xxx | Cracked/Ritual Pots |
| 67xxx-68xxx | Cookbooks |
| 69xxx | Notes |
| 100xxx-130xxx | NPC-specific shop items |
| 150xxx | Wandering Merchant stock |

**Release Flag Patterns**:

| Pattern | Meaning |
|---------|---------|
| 0 | Always available |
| 10XXYYZZZZ | World pickup required (give scroll/item to NPC) |
| 11xxxxxx | Legacy dungeon event required |
| 14xxxxxx | Quest progression required |
| 35xxxxxx | Specific quest state required |

**Example: Sorcery Scroll → Spell Unlock**

```
[World Pickup]
1044369244 (Royal House Scroll pickup)
    ↓ give to Sellen
[Release Flag Set]
eventFlag_forRelease = 1044369244
    ↓ enables
[Shop Item Available]
Glintstone Stars, Glintstone Arc appear in Sellen's shop
```

**Source File**: `ShopLineupParam.param.xml`

### 4. Boss Defeat → Reward Chains

Boss defeats trigger multiple flag types:

```
[Boss Defeat Flag]
171 (Godrick defeated marker)
    ↓ triggers
[Remembrance Possession]
9101 (Godrick's Remembrance obtained)
    ↓ enables
[Duplication]
69010 (Can duplicate at Walking Mausoleum)
    ↓ AND enables
[Great Rune]
160 (Godrick's Great Rune possessed)
    ↓ enables
[Activation]
180 (Godrick's Great Rune activated - after Divine Tower)
```

**Source Files**:
- `common.emevd.js` Events 720, 730, 1100, 1720
- `ItemLotParam_map.param.xml` boss drop definitions

---

## Source Game Files Reference

### Primary Event Flag Sources

| File | Contains | Path |
|------|----------|------|
| `common.emevd.js` | Core event scripts, flag relationships | `/event/` |
| `ItemLotParam_map.param.xml` | World pickup flag IDs | `/regulation-bin/` |
| `ShopLineupParam.param.xml` | Shop stock/release flags | `/regulation-bin/` |
| `WorldMapPointParam.param.xml` | Grace/landmark definitions | `/regulation-bin/` |
| `openmap.eventflagalloclist` | Overworld flag allocation | `/event/` |
| `legacymap.eventflagalloclist` | Dungeon flag allocation | `/event/` |

### Secondary Reference Files

| File | Contains |
|------|----------|
| `BonfireWarpParam.param.xml` | Grace warp points and IDs |
| `MapMimicryEstablishmentParam.param.xml` | Region boundary definitions |
| `WorldMapLegacyConvParam.param.xml` | Legacy dungeon map coordinates |

### Decompiled File Location

All source files are in:
```
/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files/
├── event/
│   ├── common.emevd.js
│   ├── openmap.eventflagalloclist
│   └── legacymap.eventflagalloclist
└── regulation-bin/
    ├── ItemLotParam_map.param.xml
    ├── ShopLineupParam.param.xml
    └── WorldMapPointParam.param.xml
```

---

## Sparse Flag Allocation (Important Discovery 2026-02-01)

### The Problem

Not all flag IDs in a block have memory allocated in the save file. The game uses **sparse allocation**, only reserving bytes for flags that are actually used.

### How to Detect Sparse Allocation

Use the **schema-based allocation probing** system:

```bash
python scripts/verification/flag_schema.py --block 520000 --base 1341 \
    --save "/path/to/save.sl2" --boundaries
```

### Terminology

| Term | Definition |
|------|------------|
| **Schema** | Predefined structure mapping known flag IDs to expected byte offsets |
| **Allocation Bitmap** | Result showing which positions have real data vs padding (0xFF) |
| **Sparse Gap** | Flag ID range where all bytes are 0xFF across all save slots |
| **Trackable Flag** | Flag ID with allocated memory (can be verified) |
| **Untrackable Flag** | Flag ID in a sparse gap (cannot be verified with block formula) |

### Example: Block 520000 Sparse Allocation

Block 520000 (Spirit Ashes, Talismans) has multiple sparse gaps:

```
520000-520059: ALLOCATED ████████████
520060-520089: SPARSE GAP ░░░░░░░░░░
520090-520189: ALLOCATED ████████████████████████
520190-520219: SPARSE GAP ░░░░░░░░░░░░
520220-520329: ALLOCATED ████████████████████████████
520330-520349: SPARSE GAP ░░░░░░░░
520350-520449: ALLOCATED ████████████████████████████
520450-520469: SPARSE GAP ░░░░░░░░
520470-520699: ALLOCATED ████████████████████████████████████████████
520700-520749: SPARSE GAP ░░░░░░░░░░░░░░░
520750-520810: ALLOCATED ████████████████
```

### Implications

1. **Pre-filter before verification**: Use `BlockSchema.probe_allocation()` to identify trackable flags
2. **Untrackable items**: Items with flag IDs in sparse gaps may use alternative tracking mechanisms
3. **Not all ItemLotParam flags are stored**: The game may not persist all defined flag IDs

### API for Sparse Detection

```python
from scripts.verification.flag_schema import BlockSchema

schema = BlockSchema(520000, base_offset=1341)
schema.load_flags_from_extracted('scripts/extracted_event_flags.json')

bitmap = schema.probe_allocation(save_path)

if bitmap.is_trackable(520000):  # True
    # Safe to verify this flag
    pass

if not bitmap.is_trackable(520210):  # False - sparse gap
    # Cannot verify this flag with block formula
    pass
```

---

## Flag Storage in Save File

See also: `docs/Flag-islands.md`

Event flags are stored in contiguous bit arrays within each character slot:

| Section | Approximate Offset | Size |
|---------|-------------------|------|
| Block flags (60000-99999) | 0x4ec - 0x0a00 | ~1.5 KB |
| Grace flags (76000-79999) | 0x0cb2 - 0x0e00 | ~350 bytes |
| World tile flags | Variable | 875 bytes/tile |
| Dungeon flags | Variable | 1125 bytes/section |

**Key Insight**: One verified flag anchor reveals entire blocks:
- Finding flag 67120 at byte 3549 → base = 3549 - (67120-67000)/8 = 3546
- Now ALL 67xxx flags can be calculated from this base

---

## IMPORTANT: Save-Dependent Base Offsets

### The Problem

**Tile and dungeon formula bases are SAVE-DEPENDENT, not universal.**

Analysis of 55+ b-series and 10+ Confessor-series snapshots revealed:

1. **EF offset varies with GaItems count** - Inventory changes during gameplay affect GaItems section size
2. **Different save series have different calibrated bases** - We found a ~4571 byte difference between save series
3. **Hardcoded base offsets may not work** for all saves

### Evidence

| Save Series | Tile Base (Smoldering Butterfly) | Notes |
|-------------|-----------------------------------|-------|
| b-series (Slot 0) | 485951 | Early captures |
| Confessor series | 490522 | Later captures, +4571 difference |
| Ground truth | 489981 | Calibrated 2026-01-20 |

### Solution: Dynamic Calibration

Before running verification, calibrate for the specific save:

```python
from scripts.verification.snapshot_test_runner import SnapshotTestRunner

runner = SnapshotTestRunner()
cal = runner.calibrate_for_save("/path/to/save", slot=0)

print(f"EF offset: {cal.ef_offset}")
print(f"Tile base: {cal.tile_base}")
print(f"Confidence: {cal.tile_base_confidence:.2f}")
```

### Calibration Anchors

| Formula | Anchor Flag | Anchor Name | Usage |
|---------|-------------|-------------|-------|
| Tile | 1043500010 | Smoldering Butterfly | Frequently SET, used for base calibration |
| Block | 76100 | The First Step | Always SET after tutorial |
| Dungeon | 16000002 | Volcano Manor grace | Area 16 base calibration |

### Best Practices

1. **Always calibrate** before running verification on a new save file
2. **Store calibration results** in the capture catalog with each snapshot
3. **Use validation flags** to detect EF section offset
4. **Cross-validate** using multiple anchor flags when possible

---

## Related Documentation

- `CLAUDE.md` - Complete flag range reference tables
- `Flag-islands.md` - Block offset propagation
- `DATABASE_COVERAGE_ANALYSIS.md` - Current implementation coverage
- `SAVE_FILE_GROUND_TRUTH.md` - Verified flag positions
- `discovery-verification-cycle.md` - Automated capture workflow
