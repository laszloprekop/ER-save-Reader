# Database Module Coverage Analysis

**Last updated**: 2026-02-08
**App Version**: v0.13.1

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: ERA-MIXED — partially refreshed 2026-07-20.** Sections dated 2026-07-20 ("Read Coverage", "Database vs primary source") are current; the base-status and success-rate tables predate the migration. The "Last updated 2026-02-08 / v0.13.1" header applies only to the older sections.
> - **Claims**: which DB modules cover which game data; DB-vs-primary-source gaps; EF detection success rates by formula/category; block/dungeon base status.
> - **Evidence**: DB files vs `regulation-bin` param extracts (regulation 1.16.1).
> - **Methodology**: coverage diffing; the 2026-07-20 sections were audited against the primary source.
> - **Obsolete**: "Block Base Status" / "Dungeon Base Status" and "success rate by formula type" assume the static-base model deleted in ADR-0008; positions are resolved per save now, so a "success rate" tied to fixed bases is not meaningful.

---

## Executive Summary

The ER-save-Reader has **40 database modules** cataloging **~22,184 game data entries** across equipment, world pickups, event flags, NPCs, shops, spells, and more. Coverage for core item/equipment data is excellent (~90-100%). Event flag detection is production-ready for tile and dungeon formulas, with block formula coverage at ~76%.

**Current State**:
- 21 UI routes (8 character views, 11 database views, 2 utilities)
- 8 auto-generated modules (via `scripts/generate_db.py`)
- 32 hand-maintained modules

---

## Database Module Inventory

### Equipment Name Databases

| Module | Purpose | Entry Count | Generation |
|--------|---------|-------------|------------|
| `weapon_name.rs` | Weapon ID -> Name | 3,081 | Hand-maintained |
| `armor_name.rs` | Armor ID -> Name | 798 | Hand-maintained |
| `item_name.rs` | Item ID -> Name | 2,316 | Hand-maintained |
| `accessory_name.rs` | Talisman ID -> Name | 156 | Hand-maintained |
| `aow_name.rs` | Ash of War ID -> Name | 243 | Hand-maintained |
| `unified_items.rs` | All EquipParam combined | 6,857 | Auto-generated |

### Event Flag & World Data

| Module | Purpose | Entry Count | Generation |
|--------|---------|-------------|------------|
| `event_flags.rs` | Event ID -> (byte, bit) | 5,751 | Auto-generated |
| `world_pickups.rs` | Overworld pickup tracking | 5,477 | Auto-generated |
| `pickup_data.rs` | Enriched pickup data | 4,810 | Auto-generated |
| `dungeon_pickups.rs` | Dungeon pickup tracking | 2,109 | Auto-generated |
| `pickup_flags.rs` | Formula-based flag offsets | N/A | Hand-maintained |
| `entity_relationships_data.rs` | Cross-entity relationships | 613 | Auto-generated |

### Sites of Grace & Bosses

| Module | Purpose | Entry Count | Generation |
|--------|---------|-------------|------------|
| `graces.rs` | Grace enum + flags | 382 | Hand-maintained |
| `graces_data.rs` | Enriched grace data | 421 | Auto-generated |
| `bosses.rs` | Boss enum + flags | 157 | Hand-maintained |
| `bosses_data.rs` | Enriched boss data | 205 | Auto-generated |

### Discovery & Progression

| Module | Purpose | Entry Count | Generation |
|--------|---------|-------------|------------|
| `regions.rs` | Region enum + metadata | 278 | Hand-maintained |
| `summoning_pools.rs` | Pool enum + flags | 162 | Hand-maintained |
| `maps.rs` | Map enum + flags | 33 | Hand-maintained |
| `cookbooks.rs` | Cookbook enum + flags | 104 | Hand-maintained |
| `whetblades.rs` | Whetblade enum + flags | 12 | Hand-maintained |
| `colosseums.rs` | Colosseum enum + flags | 3 | Hand-maintained |
| `landmarks.rs` | Landmark/POI tracking | 308 | Hand-maintained |

### NPCs & Quests

| Module | Purpose | Entry Count | Generation |
|--------|---------|-------------|------------|
| `npcs.rs` | NPC tracking | 30 | Hand-maintained |
| `quest_chains.rs` | Quest chain definitions | 24 | Hand-maintained |

### Shops & Commerce

| Module | Purpose | Entry Count | Generation |
|--------|---------|-------------|------------|
| `shop_items.rs` | Shop stock tracking | 1,372 | Hand-maintained |
| `merchants_data.rs` | Merchant-grouped data | 1,277 | Hand-maintained |

### Spells & Character

| Module | Purpose | Entry Count | Generation |
|--------|---------|-------------|------------|
| `spells.rs` | Spell database | 315 | Auto-generated |
| `stats.rs` | Stat growth tables | N/A | Hand-maintained |
| `classes.rs` | Starting class data | 10 | Hand-maintained |

---

## Comparison with Game Params

### Equipment Coverage

| Category | DB Count | Game Param | Param Rows | Coverage |
|----------|----------|------------|------------|----------|
| Weapons | 3,081 | EquipParamWeapon | 3,554 | **87%** |
| Armor | 798 | EquipParamProtector | 820 | **97%** |
| Talismans | 156 | EquipParamAccessory | 157 | **99%** |
| Items/Goods | 2,316 | EquipParamGoods | 2,326 | **99%** |
| Ashes of War | 243 | EquipParamGem | 242 | **100%** |
| Spells | 315 | Magic | 317 | **99%** |

### World Data Coverage

| Category | DB Count | Estimated Total | Coverage |
|----------|----------|-----------------|----------|
| World Pickups | 5,477 + 4,810 | ~5,564 | **98%+** |
| Dungeon Pickups | 2,109 | ~2,500 | **84%** |
| Graces | 421 | 422 | **99%** |
| Bosses | 205 | ~210 | **98%** |
| Landmarks | 308 | ~320 | **96%** |
| NPCs | 30 | ~500 | **6%** |
| Shop Items | 1,372 + 1,277 | ~1,400 | **98%** |

### Read Coverage (added 2026-07-20)

Having a row in the database is not the same as being able to READ its flag. Since the
family cutovers (ADR-0006) an unreadable entry renders as Unknown rather than as "not
found", so these gaps are now visible instead of silently counted as negatives:

| Category | Resolvable | Unknown | Why |
|----------|-----------|---------|-----|
| Graces | 421 / 421 | 0 | — |
| Bosses | 176 / 205 | 29 | 26 DLC tiles outside the m60 grid; 2 doubly-allocated maps (m34_12, m40_00); 1 disputed id (Night's Cavalry 1248550800) |
| Dungeon Pickups | 2,072 / 2,108 | 36 | 32 in the two doubly-allocated maps; 2 under a bogus prefix 9901; 2 whose localId < 7000 (so not pickups at all) |
| World Pickups | 3,292 / 4,809 | 1,517 | 532 DLC tiles; ~935 six-digit ids belonging to no verified family; the remainder doubly-allocated or out-of-grid |

> **`WORLD_PICKUPS` is not a single-family table**, despite the name. Of its 4,809
> entries only 1,232 are open-world tiles; there are also 2,010 legacy-map pickups, 100
> world-state-b flags, 935 unclassified six-digit ids and 532 DLC tiles. v0.28.0 routed
> the whole table through the tile reader, which left 3,577 reading Unknown. Routing by
> family (`pickup_flags::pickup_flag_state`) recovered 2,060 of them. The first cut was
> not wrong — no entry read a wrong bit, because each family reader rejects foreign ids —
> it was needlessly blind, and the aggregate count was what exposed it.

### Database vs primary source (audited 2026-07-20)

`dungeon_pickups.rs` diverges from `ItemLotParam_map` (regulation 1.16.1) in both
directions, roughly 8% each way:

| | count |
|---|---|
| legacy `getItemFlagId`s in the primary source | 2,069 |
| legacy entries in `dungeon_pickups.rs` | 2,106 |
| in the DB, absent from the primary source | 189 |
| in the primary source, absent from the DB | 152 |

The missing-from-DB entries cluster in m41_00/01/02, m40_02 and m13_00 (29+25+19+18+17
for the top five maps). Provenance of the 189 DB-only entries is unknown; they are
third-party in origin and unverified. Regenerating `dungeon_pickups.rs` from
`ItemLotParam_map` is its own task with its own verification — it is NOT a cleanup to
fold into a flag-layout change — but the primary source is now on this machine, so it is
unblocked whenever someone wants it.

The DLC gap is one issue, not four: DLC maps (m61 tiles, `2xxxxxxxxx` ids) have no
verified layout.

> **BLOCKED ON EVIDENCE, not on effort (2026-07-20).** The DLC is not installed on this
> machine and no character has progressed into DLC content. Verification here depends on
> attributed before/after transitions, so with no DLC-progressed save there is nothing to
> attribute and no way to test a hypothesised base. A layout inferred from the alloclists
> alone would be an unverifiable claim of exactly the kind ADR-0004's status ladder
> exists to keep out of the app. These flags read Unknown, which is the correct answer
> until the evidence exists. **Do not treat the size of this number as an argument for
> working on it.** Unblocking needs the DLC installed and a character captured either
> side of a DLC pickup or boss kill.

### Still Missing

| Category | Game Param | Estimated Rows | Status |
|----------|------------|----------------|--------|
| Gestures | GestureParam | ~60 | NOT STARTED |
| Full NPC DB | NpcParam | ~500 | Only 30 of ~500 |

---

## Event Flag Detection Success Rates

**Overall: ~60.7% of tested flags proven correct**

### By Formula Type

| Formula Type | Success Rate | Count | Status |
|-------------|-------------|-------|--------|
| **Tile Formula** (10-digit) | **100%** | 69/69 | PRODUCTION READY |
| **Dungeon Formula** (8-digit) | **99.4%** | 2,316/2,331 | PRODUCTION READY |
| **Block Formula** (5-digit) | **76.4%** | 375/491 | MOSTLY READY |

### By Flag Category

| Flag Category | Proven Rate | Notes |
|--------------|------------|-------|
| Grace Flags | 76.2% (375/492) | 6 block bases validated |
| Great Boss Defeat | 9.6% (8/83) | Low coverage, needs work |
| Field Boss Defeat | 4.3% (1/23) | Very low coverage |
| Generic Boss Defeat | 13.8% (8/58) | Low coverage |

### Block Base Status

| Status | Count | Blocks |
|--------|-------|--------|
| **Verified** | 9 | 60000, 61000, 62000, 65000, 67000, 68000, 71800, 76000, 78000 |
| **Unreliable** | 4 | 71000, 71100, 71600, 73000 (varies by save progression) |
| **Disproven** | 2 | 75000, 77000 |

### Dungeon Base Status

| Status | Count | Details |
|--------|-------|---------|
| **Verified** | 7 areas | 10, 11, 12, 14, 30, 31, 32 |
| **Calculated** | 8 areas | 13, 15, 16, 18, 19, 34, 35, 39 |
| **Unverified** | 2 areas | 20, 21 |
| **Pickup section bases** | 89 sections | Across 22 areas |

---

## Code Redundancy Notes

Several data categories have parallel modules that evolved independently:

| Data | Module A | Module B | Issue |
|------|----------|----------|-------|
| Pickups | `world_pickups.rs` (5,477) | `pickup_data.rs` (4,810) | Different struct, overlapping data |
| Graces | `graces.rs` (382 enum) | `graces_data.rs` (421 enriched) | Enum + enriched data split |
| Bosses | `bosses.rs` (157 enum) | `bosses_data.rs` (205 enriched) | Enum + enriched data split |
| Shops | `shop_items.rs` (1,372) | `merchants_data.rs` (1,277) | Different grouping of same source |

These pairs serve different purposes (enum-based vs data-enriched) but represent potential consolidation opportunities.

---

## Key Technical Constants

See [EVENT-FLAG-GEOGRAPHY.md](EVENT-FLAG-GEOGRAPHY.md) for complete formula documentation.

Single source of truth for constants: `crates/wasm-event-flags/src/lib.rs`

| Constant | Value |
|----------|-------|
| Tile base_offset | 337375 |
| Row base | 33 |
| Col base | 30 |
| Bytes per slot | 875 |
| Slots per row | 40 |
| Max local ID | 6999 |
| World pickup row ID base | 1037373320 |
| Event flags size | 0x1BF99F (1,833,375) |

---

## Remaining Gaps

See [BACKLOG.md](BACKLOG.md) for the full prioritized list of outstanding work.
