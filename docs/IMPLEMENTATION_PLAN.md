# Implementation Plan: Database Coverage Expansion

## Overview

Expand ER-save-Editor's database coverage from ~10% to ~80%+ of trackable game state by implementing missing modules for spells, NPCs, shop items, world pickups, and expanding event flag mappings with coordinate lookup.

---

## Phase 1: Spells Database (Priority: High)

### Goal
Enable display/editing of equipped spells by name instead of raw IDs.

### Files to Create/Modify

**New: `src/db/spells.rs`**
```rust
// Pattern: HashMap<u32, SpellInfo>
pub struct SpellInfo {
    pub name: &'static str,
    pub spell_type: SpellType, // Sorcery or Incantation
    pub fp_cost: u16,
    pub slots: u8,
    pub int_req: u8,
    pub fai_req: u8,
}
pub enum SpellType { Sorcery, Incantation }
```

**Source**: `Magic.param.xml` (317 entries)
- Field mapping: `id` → key, `paramdexName` → name (strip "[Sorcery] " prefix), `ezStateBehaviorType` → spell_type (0=Sorcery, 1=Incantation), `mp` → fp_cost, `slotLength` → slots

**Modify: `src/db/mod.rs`**
- Add `pub mod spells;`

**Modify: `src/vm/slot.rs`**
- Add `equipped_spells: Vec<SpellSlot>` to ViewModel
- Implement `from_save()` to read from `equip_magic_data`

**New: `src/ui/spells.rs`**
- Display 12 spell slots with names
- Allow reordering/removing equipped spells

### Complexity: Medium (~200 LOC db, ~150 LOC ui)

---

## Phase 2: Event Flags Expansion with Coordinates (Priority: High) ✅ COMPLETE (v0.2.0)

### Goal
Expand event flag coverage from 1,350 to ~5,000+ flags with in-game world coordinates for map lookup.

### Implementation Status (v0.2.0)
**Completed**: Created `src/db/event_flags_db.rs` with ~5,000+ entries consolidated from:
- `pickup_data.rs` (~4,809 world pickups)
- `graces.rs` (~300 grace sites)
- `bosses.rs` (~200 boss defeats)
- `cookbooks.rs` (~85 cookbooks)
- `whetblades.rs` (~6 whetblades)
- Manual entries for Great Runes, Remembrances, Map Fragments, System flags

**UI**: Created `src/ui/event_flags_db_view.rs` with:
- Category filtering (20 categories)
- Region dropdown filtering
- Text search (by name or flag ID)
- JSON export (full database or filtered results)

### New Data Structure

**Modify: `src/db/event_flags.rs`**
```rust
pub struct EventFlagInfo {
    pub byte_offset: u32,
    pub bit_position: u8,
    pub name: &'static str,
    pub category: EventFlagCategory,
    pub coordinates: Option<WorldCoords>, // NEW: In-game coordinates
}

pub struct WorldCoords {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub map_id: u32, // e.g., m60_41_36_00 encoded
}

pub enum EventFlagCategory {
    Grace,
    Boss,
    WorldPickup,
    LegacyPickup,
    ShopStock,
    NpcState,
    GreatRune,
    Cookbook,
    Whetblade,
    SummoningPool,
    Colosseum,
    Mausoleum,
    Map,
}
```

### Flag Ranges to Add

| Range | Count | Category | Source |
|-------|-------|----------|--------|
| 160-187 | 28 | Great Runes | common.emevd.js |
| 9100-9199 | 100 | Remembrance possession | common.emevd.js |
| 69010-69760 | 75 | Mausoleum usage | common.emevd.js |
| 10XXYYZZZZ | ~2,000 | World pickups | ItemLotParam_map.param.xml |
| 20XXYYZZZZ | ~500 | DLC pickups | ItemLotParam_map.param.xml |
| XXYYYZZZZ | ~500 | Legacy dungeon items | legacymap.eventflagalloclist |

### Coordinate Sources

Coordinates for event flags can be extracted from:
1. **ItemLotParam_map.param.xml** - Contains `posX`, `posY`, `posZ` for world pickups
2. **MapStudioBonfire.param.xml** - Grace site positions
3. **WorldMapPointParam.param.xml** - POI/landmark coordinates
4. **NpcLocation.param** (or equivalent) - NPC spawn positions

### UI Enhancement

**Modify: `src/ui/event_flags.rs`**
- Add coordinate display column
- Add "Copy Coords" button for each flag
- Add category filter dropdown
- Consider map thumbnail with marker (future enhancement)

### Complexity: High (~1,500 LOC db expansion, ~200 LOC ui)

---

## Phase 3: NPC Tracking (Priority: High)

### Goal
Track discovered NPCs, their state, and location.

### Files to Create

**New: `src/db/npcs.rs`**
```rust
pub struct NpcInfo {
    pub id: u32,
    pub name: &'static str,
    pub discovery_flag: u32,
    pub death_flag: Option<u32>,
    pub quest_flags: Vec<u32>,
    pub location: &'static str,
    pub coordinates: Option<WorldCoords>,
}
```

**Source**: Cross-reference multiple params:
- `NpcParam.param.xml` - NPC definitions (~500 rows)
- `TalkParam.param.xml` - Dialogue triggers
- `common.emevd.js` - NPC state flag patterns (100-300 range)

**Key NPCs** (71 trackable in save):
- Merchants (Kalé, Isolated Merchant, etc.)
- Quest NPCs (Ranni, Millicent, Alexander, etc.)
- Invaders (state tracking)

### Complexity: Medium (~300 LOC db, ~150 LOC ui)

---

## Phase 4: Shop Stock Tracking (Priority: Medium)

### Goal
View/reset purchased shop items per merchant.

### Files to Create

**New: `src/db/shop_items.rs`**
```rust
pub struct ShopItem {
    pub id: u32,
    pub item_id: u32,
    pub item_type: ItemType, // Weapon, Armor, Good, etc.
    pub merchant_id: u32,
    pub stock_flag: u32,      // eventFlag_forStock
    pub release_flag: u32,    // eventFlag_forRelease (Bell Bearing)
    pub price: u32,
    pub quantity: i32,        // -1 = unlimited
}

pub struct Merchant {
    pub id: u32,
    pub name: &'static str,
    pub items: Vec<u32>,      // ShopItem IDs
    pub bell_bearing_flag: Option<u32>,
}
```

**Source**: `ShopLineupParam.param.xml` (1,277 entries)
- Group by `shopType` to associate with merchants
- `eventFlag_forStock` = purchase tracking flag
- `eventFlag_forRelease` = availability flag (Bell Bearing donation)

### Complexity: Medium (~400 LOC db, ~200 LOC ui)

---

## Phase 5: World Pickup Browser (Priority: Medium)

### Goal
Browse all world pickups with collection status and coordinates.

### Files to Create

**New: `src/db/world_pickups.rs`**
```rust
pub struct WorldPickup {
    pub flag_id: u32,           // 10-digit format
    pub item_id: u32,
    pub item_type: ItemType,
    pub item_name: &'static str,
    pub quantity: u32,
    pub region: Region,
    pub sub_area: &'static str,
    pub coordinates: WorldCoords,
    pub pickup_type: PickupType, // Corpse, Chest, Scarab, Boss, etc.
}

pub enum PickupType {
    Corpse,
    Chest,
    Scarab,
    BossDrop,
    NpcDrop,
    Hidden,
}
```

**Source**: `ItemLotParam_map.param.xml` (5,564 entries)
- `getItemFlagId` → flag_id
- `lotItemId01-08` → item IDs (resolve via existing db modules)
- Position fields for coordinates

### UI Features

**New: `src/ui/world_pickups.rs`**
- Filter by: Region, Item Type, Collection Status
- Sort by: Name, Region, Flag ID
- Show coordinates with copy button
- Bulk operations: Mark all collected in region

### Complexity: High (~800 LOC db, ~300 LOC ui)

---

## Phase 6: UI Integration

### Menu Updates

**Modify: `src/ui/menu.rs`**
```rust
pub enum Route {
    // Existing...
    Spells,          // NEW
    Npcs,            // NEW
    ShopStock,       // NEW
    WorldPickups,    // NEW
}
```

### ViewModel Updates

**Modify: `src/vm/slot.rs`**
- Add fields for each new feature
- Implement `from_save()` / `update_save()` for all

---

## Implementation Order

1. **Spells** - Quick win, enables spell name display
2. **Event Flags with Coords** - Foundation for other features
3. **NPCs** - Builds on event flag expansion
4. **Shop Stock** - Uses event flag infrastructure
5. **World Pickups** - Largest scope, depends on all above

---

## Data Extraction Scripts

Create helper scripts to extract data from game params:

**`scripts/extract_spells.py`**
- Parse Magic.param.xml → generate spells.rs

**`scripts/extract_event_flags.py`**
- Parse ItemLotParam_map.param.xml for pickup flags + coordinates
- Parse common.emevd.js for quest/state flags
- Output expanded event_flags.rs

**`scripts/extract_shop_items.py`**
- Parse ShopLineupParam.param.xml → generate shop_items.rs

---

## Estimated Total Scope

| Component | New Lines | Modified Lines |
|-----------|-----------|----------------|
| spells.rs | 350 | - |
| event_flags.rs | 1,500 | 200 |
| npcs.rs | 450 | - |
| shop_items.rs | 600 | - |
| world_pickups.rs | 1,100 | - |
| UI files | 800 | 150 |
| ViewModel | - | 300 |
| Extraction scripts | 400 | - |
| **Total** | **~5,200** | **~650** |

---

## Success Criteria

- [ ] Equipped spells show names in UI
- [ ] Event flags expanded to 5,000+ with coordinate display
- [ ] 71 NPCs trackable with discovery status
- [ ] Shop stock viewable/resettable per merchant
- [ ] World pickups browsable with region filter and coordinates
- [ ] All new data properly saves back to .sl2 file
