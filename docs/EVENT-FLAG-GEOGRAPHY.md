# Event Flag Geography and Hierarchy

This document describes how Elden Ring event flags are organized geographically and hierarchically, how flags chain together for quests and unlocks, and which game files are the authoritative sources.

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: ERA-MIXED — durable concepts, disproven numbers.** Trust the *shapes* here (flag-id formats, the family split, quest/unlock chains, source game files); do **not** lift any numeric base, stride, or byte offset from this doc.
> - **Claims**: how flags are organized geographically and hierarchically; per-family flag-id formats; block/dungeon/tile base tables; quest, unlock, and reward chains; which game files are authoritative.
> - **Evidence**: game EMEVD/param/alloclist extracts (corpora `game-raw-1162` / `game-extracts`) back the *structure*; the numeric base tables were single-save empirical measurements from the pre-migration (pre-2026-07-05) era.
> - **Methodology**: narrative synthesis from game files + community maps — **not** the claims-store pipeline. The positions here were never re-derived per save.
> - **Obsolete**: every "Event Base / Base Offset / block base" number below is superseded. Flag positions float per save and are *resolved*, never hardcoded — see `CONTEXT.md` (*Origin*, *Family Constant*, *Resolver*), the resolver in `crates/wasm-event-flags/`, and per-family layouts in `knowledge/claims/event-flags.json`. The dungeon "Event Base" column is the disproven "+per-area stride" model **deleted in ADR-0008**; the literals **43487, 46862, 50237** are tombstoned (all-zero in every save; now banned by `export_shape_conformance.rs`). The real legacy layout is `alloc_slot(map)*1125 + localId/8`, slots from the game's own alloclists (`LEGACY_ALLOC_SLOTS` / `knowledge/game/eventflag-alloclists.json`), not this table. **Area labels 18/19/20 are mislabeled** — maps m20/m21 are the DLC areas Belurat / Enir-Ilim (DLC alloclist slots 150-156), not "Roundtable Hold / Chapel of Anticipation / Stranded Graveyard". The `event/*.emevd.js` files in "Decompiled File Location" were **not** regenerated after the 2026-07-05 reset (the pipeline parses raw `.emevd` natively); see `CLAUDE.md`.

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

### 1b. Simple Flags (flag_id < 60,000) — ML Discovery 2026-02-18

Flags below 60,000 use a direct byte calculation with no block lookup:

**Offset Formula**:
```
byte_offset = flag_id / 8
bit_position = 7 - (flag_id % 8)
```

ML clustering on 799 timeline diffs identified 132 active offsets in EF+1040-1259 (flag IDs 8320-10079) with flag-like behavior. Cross-referencing with extracted EMEVD/param data confirmed 133 known flags:

| Range | Category | Count | Examples |
|-------|----------|-------|----------|
| 9100-9190 | Remembrance | 56 | Boss remembrance obtained, Enia shop unlocks |
| 9200-9295 | Talisman Pouch | 63 | Talisman slot expansion, related progression |
| 9404-9440 | EMEVD / Shop | 9 | Ending-related flags, sorcery unlocks |
| 9500-9504 | Mending Rune | 4 | Mending rune possession for endings |
| 9800-9810 | Unknown | 2 | Good_12302, Good_12307 references |

**Status**: Extracted from game files. Formula implemented in WASM. Not yet verified via multi-slot differential.

### 2. Block-Based Flags (5-6 digit flags)

> **⚠ OBSOLETE NUMBERS (see epistemic header).** The hex base offsets below are single-save
> measurements from the pre-migration era. Block flags float per save like every other family;
> resolve them, do not hardcode these values.

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
| 71000-71099 | Dungeon Graces (Stormveil) | 0x2463 (sub-block) |
| 71100-71799 | Dungeon Graces (Leyndell, Underground, etc.) | 0xa41 (main-block) |
| 71800-71899 | Tutorial Graces | 0xaa5 (sub-block) |
| 72000-72999 | DLC Graces (Enir-Ilim) | 0xabe |
| 73000-73999 | Dungeon Graces | 0xa66 |
| 74000-74999 | DLC Dungeon Graces | 0xbb8 |
| 76000-76999 | World Graces | 0xcb2 |
| 78000-78999 | Grace Guidance | 0xdac |
| 91000-91999 | Boss Remembrance | 0x950 |
| 92000-92999 | Container Upgrades | 0x978 |

**Source File**: Derived from `common.emevd.js` event definitions

**Sub-block / Main-block Routing**: Block flags use a two-tier lookup. First, the flag is rounded to its 100-granularity sub-block (`flag_id / 100 * 100`). If a sub-block base exists, it's used. Otherwise, the 1000-granularity main-block (`flag_id / 1000 * 1000`) is tried. This allows block 71000 to route Stormveil graces (71000-71099) to a separate allocation (base 9315) while dungeon graces (71100-71799) use the main-block base (2625).

### 3. Dungeon & Area Flags (8-digit flags)

Dungeons and special areas use an 8-digit format:

**Format**: `AASSZZZZ`

| Component | Meaning | Range |
|-----------|---------|-------|
| `AA` | Area ID | 10-39 |
| `SS` | Section within area | 00-22 |
| `ZZZZ` | Local flag index | 0000-9999 |

#### Legacy Dungeons (Major Story Areas)

> **⚠ OBSOLETE NUMBERS (see epistemic header).** The "Event Base" column is the disproven
> "+per-area stride" model (ADR-0008). Use `alloc_slot(map)*1125 + localId/8` with slots from
> `LEGACY_ALLOC_SLOTS`, resolved per save. Area 18/19/20 labels are wrong (m20/m21 = DLC
> Belurat/Enir-Ilim). The table is kept only for its area-id → name intent, not its numbers.

| Area | Name | Event Base | Pickup Status |
|------|------|------------|---------------|
| 10 | Stormveil Castle | 4112 | Per-section bases verified (2 sections) |
| 11 | Leyndell, Royal Capital | 8612 | Per-section bases verified (3 sections) |
| 12 | Underground (Siofra, Ainsel) | 15362 | Per-section bases verified (5 sections) |
| 13 | Crumbling Farum Azula | 26612 | Per-section bases verified (1 section) |
| 14 | Academy of Raya Lucaria | 29987 | Per-section bases verified (1 section) |
| 15 | Miquella's Haligtree | 33362 | Per-section bases verified (1 section) |
| 16 | Volcano Manor | 40517 | Per-section bases verified (1 section) |

*Item pickups use per-section lookup, NOT the linear formula. See "Dungeon Pickup Bases" section below.

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

#### Dungeon Pickup Bases (CRITICAL DISCOVERY 2026-02-02)

**Item pickup flags (local_id >= 7000) use COMPLETELY DIFFERENT allocation than general dungeon events.**

The general dungeon event bases work for graces, boss defeats, etc. (local_id 0-999).
But item pickups do NOT follow the linear section formula. Each (area, section) has its own empirically-discovered base.

##### The Linear Formula is WRONG

The old formula `pickup_base + section * 1125 + local_id / 8` assumed sections were allocated contiguously in memory. **This is incorrect.**

Empirical testing showed:
- Catacombs sections use bases ranging from 1785 to 3827 (non-linear)
- Caves sections use bases ranging from 1786 to 31903 (wildly varying)
- Tunnels sections use bases ranging from 1788 to 28979 (scattered)

##### Correct Formula (Per-Section Lookup)

```
section_base = DUNGEON_PICKUP_SECTION_BASES[(area, section)]
byte_offset = section_base + local_id / 8
bit_position = 7 - (flag_id % 8)
```

##### Verified Section Bases (89 total)

| Area | Sections | Base Range | Example |
|------|----------|------------|---------|
| 10 (Stormveil) | 0-1 | 1787-31904 | (10,0)→31904 |
| 11 (Leyndell) | 0,5,10 | 1812-31903 | (11,0)→31903 |
| 12 (Underground) | 1,2,3,5,7 | 31900-31903 | (12,2)→31903 |
| 30 (Catacombs) | 0-20 | 1785-3827 | (30,6)→3827 |
| 31 (Caves) | 0-7,9-12,15,17-22 | 1786-31903 | (31,21)→31903 |
| 32 (Tunnels) | 0-2,4-5,7-8,11 | 1788-28979 | (32,8)→28979 |

Full mapping in `src/db/pickup_flags.rs::DUNGEON_PICKUP_SECTION_BASES`

##### Discovery Scripts

- `scripts/discover_per_section_bases.py` - Brute-force search per section
- `scripts/build_pickup_section_map.py` - Generate Rust HashMap from save files
- `scripts/verify_specific_pickups.py` - Verify pickups against actual save data

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

### Row ID Tracking for World Pickups (CRITICAL DISCOVERY 2026-02-02)

**Key Finding**: World pickups with getItemFlagId (local_id >= 7000) are tracked using a SEPARATE row_id-based bitfield, NOT the tile formula.

#### The Problem

When checking world pickup `1044360310` (Golden Rune [1] at tile 44,36):
- The `getItemFlagId` is `1044367310` (local_id = 7310)
- The tile formula marks this as "untrackable" (local_id > 6999)
- But the pickup IS tracked - the light beam disappears when collected

#### The Discovery

Through save diff analysis (Wretch captures 34→35), we found:
- Clean single-bit changes at EF+873373 bit 1 and EF+873377 bit 3
- These correspond to row_ids `1044360310` and `1044360340` (Golden Rune [1] and [3])

#### The Formula

```
WORLD_PICKUP_ROW_ID_BASE = 1037373320

byte_offset = (row_id - WORLD_PICKUP_ROW_ID_BASE) / 8
bit_position = 7 - ((row_id - WORLD_PICKUP_ROW_ID_BASE) % 8)
```

#### Verification

| Pickup | row_id | Expected Offset | Verified |
|--------|--------|-----------------|----------|
| Golden Rune [1] | 1044360310 | EF+873373 bit 1 | ✅ SET after pickup |
| Golden Rune [3] | 1044360340 | EF+873377 bit 3 | ✅ SET after pickup |

#### Two Tracking Systems for World Pickups

| Pickup Type | Flag Field | Formula | Region |
|-------------|------------|---------|--------|
| local_id < 7000 | row_id directly | Tile formula | Tile storage (875 bytes/tile) |
| local_id >= 7000 | row_id (from getItemFlagId - 7000) | Row ID formula | Row ID bitfield |

#### How to Check World Pickup Status

1. Get `getItemFlagId` from ItemLotParam_map
2. If local_id >= 7000:
   - Convert to row_id: `row_id = getItemFlagId - 7000`
   - Use row_id formula: `byte_offset = (row_id - 1037373320) / 8`
3. If local_id < 7000:
   - Use tile formula (standard)

#### Code Reference

> **OBSOLETE — removed from the code 2026-07-20 (ADR-0008). Do not implement this.**
>
> The row_id bitfield model above was superseded on 2026-02-16: world pickups with
> getItemFlagId local_id >= 7000 are stored in the TILE region at a converted local_id
> (`flagId - 7000`), not in a separate row_id bitfield. `WORLD_PICKUP_ROW_ID_BASE` and
> `calculate_world_pickup_offset_by_row_id()` no longer exist —
> `tests/export_shape_conformance.rs` fails if either returns.
>
> The base 1037373320 is doubly wrong: it named a region that is not how the game stores
> these flags, *and* it was a fixed offset, which nothing in the flag region is. Every
> family sits after an append-only list that grows as the character plays.
>
> **Current reader:** `is_tile_pickup_set(event_flags, id)` — resolves the family for the
> save it is handed and returns `None` when it cannot, rather than a plausible number.

### Implications for Save Editing

1. **World pickups ARE trackable** - even those with getItemFlagId local_id >= 7000,
   via the tile region at the converted local_id (NOT via the obsolete row_id formula)
2. **Block flags work** - cookbooks, whetblades, etc. can be read/written, but their
   position must be resolved per save; the block base tables were deleted 2026-07-20
3. **The caller picks the family** - a bare flag id is ambiguous between the open-world
   and pickup families, so no function routes on the id alone

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
~/dev/Elden Ring stuff/Elden Ring decompiled game files/
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

Event flags are stored in contiguous bit arrays within each character slot. One verified flag anchor reveals entire blocks - finding one flag's offset allows calculating all flags in the same block/tile/section.

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
| Ground truth | 485330 | Reverted 2026-01-25 (489981 was wrong) |

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

## Structured Data Tables Within the EF Array (CRITICAL DISCOVERY 2026-02-19)

The 1,833,375-byte EventFlags array is NOT a pure bitfield. It contains **mixed data types**: boolean flag bitfields interspersed with sorted lookup tables and record structures. The simple formula `byte_offset = flag_id / 8` is WRONG for flag_ids whose byte offsets land in table regions.

### Item Acquisition Tables

Two sorted tables track items the character has ever obtained. Each entry is an 8-byte record:

**Record Format**:
```
[u32 quantity] [u32 category_prefix | item_id]
```

**Category Prefixes** (high byte of second u32):

| Prefix | Category | Example |
|--------|----------|---------|
| `0x00000000` | Weapons | Uchigatana (100000) |
| `0x10000000` | Protector/Armor | — |
| `0x20000000` | Accessory | — |
| `0x40000000` | Goods | Miquella's Lily (20653 → `0x400050AD`) |
| `0x80000000` | Custom/Reinforced | — |

**Table Locations** (verified in Bee slot, L18 mid-game):

| Zone | EF Offset Range | Size | Contents |
|------|----------------|------|----------|
| Small table | EF+2208 – EF+2832 | 624 bytes | ~78 records, Goods category dominant |
| Large table | EF+32640 – EF+34464 | 1824 bytes | ~228 records, all 5 categories |

**Key Evidence**:
- Miquella's Lily (item_id 20653) found at EF+2616 as `[01 00 00 00][AD 50 00 40]` (qty=1, Goods prefix)
- Tables are sorted by the combined `prefix|item_id` value
- Tables are only populated in saves with progression (Bee); absent in backup saves (Confessor/Wretch slots show zeros or 0xFF template)
- Entries appear for items the character has ever obtained, regardless of current inventory

**Relationship to AEG Pickups**:
- AEG pickups (e.g., Miquella's Lily from `AssetEnvironmentGeometryParam`) have `getItemFlagId=0` — no event flag assigned
- The item acquisition table tracks item TYPES (has the player ever obtained this item?), not specific instances
- Per-instance one-time tracking for AEG pickups may use a separate mechanism (MOEG/FOEG dense state records — see below)

### MOEG/FOEG System (Post-EF Region)

Beyond the EventFlags array, each character slot contains object state tracking structures near the end of the 2.6MB slot:

**Structure Hierarchy** (starting at ~0x1F6661 in slot):
```
CHR header
  └── CSBC: Visited tiles list
  └── MOEG: Map Object Enable Group (currently loaded tiles)
  └── FOEG: Far Object Enable Group (all visited tiles, superset of MOEG)
  └── Dense State Records: 20-byte per-object state
  └── Havok Data
```

**Dense State Records** (20 bytes each):
```
[u32 marker=0x0C] [f32 timer] [u32 status] [4-byte flags]
```
- `status=0`: Untouched
- `status=10`: Interacted
- Count correlates with progression (Bee: 239 records, Confessor: 146, Wretch: 0)

**MOEG Object Filtering**: MOEG tracks MSB Part/Asset entries where `behaviorType != 1` in `AssetEnvironmentGeometryParam`. For tile m60_49_36_00: 93 of 325 total assets (88 with behaviorType=0 + 5 with behaviorType=2).

**Note**: Timeline diff analysis showed 99.96% overlap between MOEG changes and non-AEG entity loading diffs, indicating MOEG records are primarily entity loading state, not pickup-specific persistence. The actual per-instance pickup tracking mechanism remains under investigation.

---

### Complete EF Layout Map

Based on comprehensive hex scanning across 3 characters (Bee L18, Confessor L95, Wretch L1):

```
EF+0 ─────────── Simple flags bitfield (flag_id < 60,000)
  │                 Active range: EF+1040-1259 (flags 8320-10079)
  │
EF+~1260 ──────── Block flags start (60000-69999, 91000-92999)
  │
EF+~2048 ──────── ┌─ STRUCTURED ZONE 1: Item acquisition tables ─┐
  │                │  EF+2208-2832: Small item table (Goods)       │
  │                └───────────────────────────────────────────────┘
  │
EF+~3072 ──────── Block flags continue (graces 76000-78999)
  │
EF+~3625 ──────── ┌─ EXCLUSION: Waypoint/Position table ──────────┐
  │                │  16-byte records with float32 coordinates      │
  │                └───────────────────────────────────────────────┘
EF+4112 ───────── Dungeon flags bitfield (areas 10-43)
  │
EF+~27648 ─────── ┌─ STRUCTURED ZONE 2 ───────────────────────────┐
  │                │  EF+27648-32640: Waypoint/sentinel records     │
  │                │  EF+32640-34464: Large item manifest (5 cats)  │
  │                └───────────────────────────────────────────────┘
EF+~34560 ─────── Tile/world pickup bitfield
  │                 tile_base = 337375
  │
EF+~214500 ────── ┌─ EXCLUSION: Map position cursor ──────────────┐
  │                │  Single 0x08 byte tracks current map area      │
  │                └───────────────────────────────────────────────┘
  │
EF+~1700000 ───── All zeros (unused tail)
  │
EF+1833375 ────── End of EF array
```

**Implications**:
1. **Flag formula validation**: Any flag whose calculated `byte_offset` falls within a structured zone should be flagged as potentially invalid
2. **AEG flag routing**: Synthetic AEG flags (3B+ range) route to tile formula → compute offsets that are either in the tile bitfield or out of bounds → return invalid. These flags are NOT stored in the EF array.
3. **Mixed data detection**: The item acquisition tables can be distinguished from bitfield regions by the repeating `[u32][u32 with 0xX0000000 prefix]` pattern

---

## Non-Flag Regions Within the EF Array

The 1,833,375-byte event flag array contains regions that are NOT event flags but still show activity in binary diffs. These must be excluded from flag detection to avoid false positives.

### Map Position Cursor (EF+214,500 – EF+226,000)

**Discovery**: ML clustering on timeline diffs (2026-02-18) identified a cluster of ~11,500 active offsets near EF+222,000. Investigation revealed this is a **map area cursor**, not event flags.

**Behavior**:
- Exactly **one byte** in this range is set to `0x08` at any time; all others are `0x00`
- When the player moves between map areas, the `0x08` byte **shifts position** (old byte clears, new byte sets)
- This produces alternating SET/CLEAR patterns — unlike permanent event flags which are SET-only

**Why it's not flags**:
- Flags are SET permanently; this region CLEARS bits when the player moves
- The single-byte `0x08` pattern doesn't match any flag formula (block, tile, dungeon, or simple)
- Co-occurrence with inventory pickups is spurious — both pickups and area changes happen during normal gameplay movement

**Implications for detection**:
- Offsets in range EF+214,500 to EF+226,000 should be **excluded** from flag candidate analysis
- Any ML or heuristic pipeline should filter this region to avoid false positives

### Waypoint/Position Table (EF+3,625 – EF+4,112)

**Discovery**: ML clustering (2026-02-18) found 132 active offsets in this range. Multi-slot hex dump revealed it's a structured record table, not event flags.

**Structure**: 16-byte records starting at EF+3,910:

| Bytes | Field | Example |
|-------|-------|---------|
| 0-3 | Entry ID (LE u32) | `07 00 00 00` = 7 |
| 4-7 | Float32 X coordinate | `ea a4 b7 44` = 1469.15 |
| 8-11 | Float32 Y coordinate | `1e 34 8f 45` = 4582.51 |
| 12-15 | Status / type | `00 02 00 00` |

Empty slots use sentinel value `0xFFFFFFFF` for the ID with zero coordinates.

**Multi-slot evidence**:
- Slot 0 (Confessor, mid-game): 5 populated records with world coordinates, 35 empty
- Slot 1 (Wretch, early game): All zeros — region not yet initialized
- Slots 2-4 (V1/V2/V3, minimal progression): Uniform `0xFF/0x00` template pattern
- The region sits between block 78000 (ends at EF+3,625) and dungeon area 10 (starts at EF+4,112)

**Why it's not flags**: Contains float32 coordinate values, not bit-level boolean states. Changes are from record updates, not flag SETs.

---

## Related Documentation

- [SAVE_FILE_GROUND_TRUTH.md](SAVE_FILE_GROUND_TRUTH.md) - Verified flag positions
- [DATABASE_COVERAGE_ANALYSIS.md](DATABASE_COVERAGE_ANALYSIS.md) - Current implementation coverage
- [discovery-verification-cycle.md](discovery-verification-cycle.md) - Discovery and verification methodology
- [CORROBORATION-SYSTEM.md](CORROBORATION-SYSTEM.md) - Dual-formula validation
