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

| Area | Name | Base Offset | Status |
|------|------|-------------|--------|
| 10 | Stormveil Castle | 4112 | Verified |
| 11 | Academy of Raya Lucaria | 4112 | Needs Review (92% match) |
| 13 | Leyndell, Royal Capital | - | Unverified |
| 15 | Miquella's Haligtree | - | Unverified |
| 16 | Crumbling Farum Azula | - | Unverified |

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

## Related Documentation

- `CLAUDE.md` - Complete flag range reference tables
- `Flag-islands.md` - Block offset propagation
- `DATABASE_COVERAGE_ANALYSIS.md` - Current implementation coverage
- `SAVE_FILE_GROUND_TRUTH.md` - Verified flag positions
