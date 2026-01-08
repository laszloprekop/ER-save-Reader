# Database Module Coverage Analysis

**Date**: 2026-01-07
**Analyzed Save**: Slot 0 (Confessor, mid-game progression)

---

## Executive Summary

The ER-save-Editor has good coverage for core item/equipment data (~90-100%) but significant gaps in event flag coverage (<10%) and missing database modules for spells, NPCs, landmarks, and shop items.

**Key Blindspots**:
1. No spell/incantation database (317 spells in game)
2. Only 1,350 of ~15,000+ event flags mapped
3. No NPC tracking or merchant discovery
4. No landmark/POI database

---

## Current Database Modules

### Module Inventory

| Module | Purpose | Lines | Entry Count |
|--------|---------|-------|-------------|
| `weapon_name.rs` | Weapon ID → Name | 3,089 | 3,081 |
| `armor_name.rs` | Armor ID → Name | 806 | 798 |
| `item_name.rs` | Item ID → Name | 2,324 | 2,316 |
| `accessory_name.rs` | Talisman ID → Name | 164 | 156 |
| `aow_name.rs` | Ash of War ID → Name | 251 | 243 |
| `event_flags.rs` | Event ID → (byte, bit) | 1,359 | 1,350 |
| `graces.rs` | Grace enum + flags | 983 | 381 |
| `bosses.rs` | Boss enum + flags | 412 | 157 |
| `regions.rs` | Region enum + metadata | 983 | 278 |
| `summoning_pools.rs` | Pool enum + flags | 336 | 162 |
| `maps.rs` | Map enum + flags | 80 | 33 |
| `cookbooks.rs` | Cookbook enum + flags | 302 | 104 |
| `whetblades.rs` | Whetblade enum + flags | 37 | 12 |
| `colosseums.rs` | Colosseum enum + flags | 17 | 3 |
| `stats.rs` | Stat growth tables | 2,389 | N/A |
| `classes.rs` | Starting class data | 193 | 10 |

---

## Comparison with Game Params

### Primary Equipment Params

| Category | DB Module | DB Count | Game Param | Param Rows | Coverage % |
|----------|-----------|----------|------------|------------|------------|
| Weapons | weapon_name.rs | 3,081 | EquipParamWeapon | 3,554 | **87%** |
| Armor | armor_name.rs | 798 | EquipParamProtector | 820 | **97%** |
| Talismans | accessory_name.rs | 156 | EquipParamAccessory | 157 | **99%** |
| Items/Goods | item_name.rs | 2,316 | EquipParamGoods | 2,326 | **99%** |
| Ashes of War | aow_name.rs | 243 | EquipParamGem | 242 | **100%** |

### Missing from Game Params

| Category | Game Param | Rows | DB Module | Status |
|----------|------------|------|-----------|--------|
| Spells | Magic | 317 | N/A | **MISSING** |
| Special Effects | SpEffectParam | 11,325 | N/A | Not needed |
| Shop Items | ShopLineupParam | 1,277 | N/A | **MISSING** |
| World Pickups | ItemLotParam_map | 5,564 | N/A | Partial (via event_flags) |
| Gestures | GestureParam | ~60 | N/A | **MISSING** |
| NPCs | NpcParam | ~500 | N/A | **MISSING** |

---

## Save File Data Coverage

### SaveSlot Structure Analysis

From `src/save/common/save_slot.rs`:

| Field | Type | Size | DB Coverage | UI Editable |
|-------|------|------|-------------|-------------|
| `player_game_data` | PlayerGameData | 588 bytes | Partial | Yes |
| `chr_asm` | ChrAsm | 88 bytes | Yes | Yes |
| `chr_asm2` | ChrAsm2 | 84 bytes | Yes | Yes |
| `equip_inventory_data` | EquipInventoryData | 29,696 bytes | Yes | Yes |
| `storage_inventory_data` | EquipInventoryData | 29,696 bytes | Yes | Yes |
| `equip_magic_data` | EquipMagicData | 116 bytes | **No** | Partial |
| `equip_item_data` | EquipItemData | 128 bytes | Yes | Yes |
| `equip_projectile_data` | EquipProjectileData | 56 bytes | Yes | Yes |
| `equip_physics_data` | EquipPhysicsData | 40 bytes | Partial | Yes |
| `event_flags` | EventFlags | 1,835,008 bytes | **<10%** | Partial |
| `regions` | Regions | Variable | Yes | Yes |
| `ga_items` | Vec<GaItem> | 327,680 bytes | Yes | Partial |
| `ga_item_data` | GaItemData | 448,512 bytes | Partial | No |
| `gesture_game_data` | Vec<i32> | 256 bytes | **No** | No |
| `ride_game_data` | RideGameData | 36 bytes | No | No |
| `face_data` | [u8; 0x12f] | 303 bytes | No | No |
| `tutorial_data` | [u8; 0x408] | 1,032 bytes | No | No |
| `world_area_weather` | WorldAreaWeather | 12 bytes | No | No |
| `world_area_time` | WorldAreaTime | 12 bytes | No | No |

### Event Flags Analysis

**Current coverage**: 1,350 flags mapped out of ~15,000+ total

**Event Flag Byte Array**:
- Total size: 0x1bf99f bytes (1,835,039 bytes ≈ 1.75 MB)
- Bit-packed storage: Each flag = 1 bit
- Maximum flags: ~14,680,000 (theoretical)
- Actual used: ~15,000-20,000 (estimated)

**Mapped Flag Ranges**:

| Range | Count | Category |
|-------|-------|----------|
| 0-199 | ~50 | Great Runes, boss markers |
| 300-999 | ~100 | Grace flags |
| 60000-69999 | ~500 | Progression, cookbooks |
| 71000-76999 | ~200 | Bosses, maps |
| 10XXXXXXXX | ~500 | World pickups (partial) |

**Unmapped Flag Ranges**:

| Range | Estimated | Category |
|-------|-----------|----------|
| 9100-9299 | ~200 | Remembrance possession |
| 100000-160000 | ~2,000 | Shop stock flags |
| 10XXXXXXXX | ~4,000 | World pickups (most) |
| 11XXXXXXXX | ~1,000 | Legacy dungeon events |
| 20XXXXXXXX | ~2,000 | DLC pickups |

---

## World Pickup Tracking

### Categories in Save (from Export Analysis)

| Category | Total Items | Tracked in Save | Notes |
|----------|-------------|-----------------|-------|
| Golden Runes | 667 | Yes | Flag format: 10XXYYZZZZ |
| Smithing Stones | 346 | Yes | Category 7 in tile data |
| Somber Stones | 200 | Yes | Category 7 in tile data |
| Glovewort | 70 | Yes | For spirit ashes |
| Weapons | 2,367 | Yes | World-dropped weapons |
| Armor | 375 | Yes | World-dropped armor |
| Talismans | 112 | Yes | Hidden talismans |
| Ashes of War | 82 | Yes | Scarabs, chests |
| Key Items | 20 | Yes | Quest items |
| Crafting Materials | 16 | Yes | Rare spawns |
| Consumables | 18 | Yes | Boluses, etc. |

**Total World Pickups**: ~4,273 items

### POI Discovery Tracking

| POI Type | Total | Currently Tracked | DB Coverage |
|----------|-------|-------------------|-------------|
| Graces | 422 | Yes (via graces.rs) | 381/422 (90%) |
| Bosses | 104 | Yes (via bosses.rs) | 157 (includes DLC) |
| NPCs | 71 | In save, not in UI | 0% |
| Landmarks | 236 | In save, not in UI | 0% |

---

## Identified Blindspots

### Critical (High Impact)

1. **Spell Database**
   - Impact: Cannot display spell names in equipped spells
   - Source: Magic.param (317 rows)
   - Solution: Create `db/spells.rs`

2. **Event Flags (<10% coverage)**
   - Impact: Most world state not viewable/editable
   - Source: ItemLotParam_map, common.emevd.js
   - Solution: Expand event_flags.rs by ~10x

3. **NPC Discovery**
   - Impact: Cannot track which NPCs have been found
   - Source: NpcParam, TalkParam
   - Solution: Create `db/npcs.rs`

4. **Merchant Shop Stock**
   - Impact: Cannot see/reset purchased items
   - Source: ShopLineupParam (1,277 rows)
   - Solution: Create `db/shop_items.rs`

### Moderate (Quality of Life)

5. **Landmarks/POIs**
   - 236 landmarks not named in UI
   - Source: WorldMapPointParam

6. **Gestures**
   - Unlocked gestures not editable
   - Source: GestureParam (~60 rows)

7. **Great Runes**
   - Possession/activation not in event flags UI
   - Flags: 160-167, 180-187

8. **Remembrance Duplication**
   - Walking Mausoleum usage not tracked
   - Flags: 69010-69760

### Minor (Low Priority)

9. Tutorial progress
10. Weather/time state
11. Character appearance (face_data)
12. Torrent (horse) state

---

## Requirements for Full Coverage

### New Database Modules

| Module | Source Param | Rows | Priority |
|--------|--------------|------|----------|
| spells.rs | Magic | 317 | High |
| npcs.rs | NpcParam | ~500 | High |
| shop_items.rs | ShopLineupParam | 1,277 | High |
| landmarks.rs | WorldMapPointParam | ~200 | Medium |
| gestures.rs | GestureParam | ~60 | Medium |

### Event Flags Expansion Plan

**Phase 1**: Core flags (~2,000 additional)
- Great Runes (160-187)
- Remembrance possession (9100-9199)
- Shop stock subset (most-used 500)

**Phase 2**: World pickups (~4,000 additional)
- All 10-digit pickup flags from ItemLotParam_map
- Group by region for UI organization

**Phase 3**: Legacy dungeon events (~1,000 additional)
- 11xxxxxx format flags
- Quest progression markers

### UI Enhancements

| Feature | New Files | Complexity |
|---------|-----------|------------|
| Spell Management | ui/spells.rs, vm/spells.rs | Medium |
| NPC Tracker | ui/npcs.rs | Low |
| World Pickup Browser | ui/pickups.rs | High |
| Shop Stock Editor | ui/shop.rs | Medium |
| Quest Progress View | ui/quests.rs | High |

---

## Data Sources Reference

### Primary (Decompiled Game Files)

| File | Location | Purpose |
|------|----------|---------|
| EquipParamWeapon.param.xml | regulation-bin/ | Weapon definitions |
| EquipParamProtector.param.xml | regulation-bin/ | Armor definitions |
| EquipParamAccessory.param.xml | regulation-bin/ | Talisman definitions |
| EquipParamGoods.param.xml | regulation-bin/ | Item definitions |
| EquipParamGem.param.xml | regulation-bin/ | Ash of War definitions |
| Magic.param.xml | regulation-bin/ | Spell definitions |
| ShopLineupParam.param.xml | regulation-bin/ | Shop inventory |
| ItemLotParam_map.param.xml | regulation-bin/ | World pickups |
| NpcParam.param.xml | regulation-bin/ | NPC definitions |
| GestureParam.param.xml | regulation-bin/ | Gesture definitions |
| common.emevd.js | event/ | Event scripts |
| openmap.eventflagalloclist | event/eventflag/ | Overworld flag mapping |
| legacymap.eventflagalloclist | event/eventflag/ | Dungeon flag mapping |

### Save File Structure

| Section | Offset | Size | Description |
|---------|--------|------|-------------|
| Slot Header | 0x0 | 0x310 | Version, map ID, checksums |
| ga_items | 0x310 | 327,680 | Item handle metadata |
| PlayerGameData | +0x50000 | 588 | Character stats |
| Equipment | +0x5024C | ~400 | Equipped items |
| Inventory | +0x50358 | 59,392 | Held + storage items |
| Event Flags | +0x6BF58 | 1,835,008 | All world state flags |
| GaItemData | +0x21BF58 | 448,512 | Extended item data |

---

## Conclusion

The application has excellent coverage for equipment and inventory (90-100%) but significant gaps in world state tracking. Priority should be:

1. **Immediate**: Add spell database for equipped spell display
2. **Short-term**: Expand event_flags.rs by 3-5x for common flags
3. **Medium-term**: Add NPC and shop tracking modules
4. **Long-term**: Complete world pickup integration with UI browser
