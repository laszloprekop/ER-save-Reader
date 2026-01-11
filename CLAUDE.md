## Knowledge Resource files (single source of truth):

Decompiled game resource files:
'/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files'

## Game save files with five character slots:

- Slot 0: Confessor, mid-game progression
- Slot 1: Wretch, early game, few steps of progression, item collection, one boss defeat
- Slot 2: V1, very little progression, made for item pickup debugging
- Slot 3: V2, similar little amout progression as V1, different path taken, same item pickup for debugging
- Slot 4: V3, similar little amout progression as V1, different path taken, no pickup for true negative diff
- '/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files'

## Third party resource usage
Treat third party resources with caution because we don't have control over their accuracy, completeness, or reliability. Most of the time they are specific to a certain game version, thus outdated and many times partially implemented. Always verify information from third-party sources against primary sources and discard them if their correctness can not be proven.

---

## Event Flag Types Research (2026-01-07)

### Verified Event Flag Ranges (from decompiled game files)

**Primary Sources**:
- `/Elden Ring decompiled game files/regulation-bin/*.param.xml` - Game params
- `/Elden Ring decompiled game files/event/common.emevd.js` - Event scripts

### Flag Range Summary

| Range | Category | Source | Notes |
|-------|----------|--------|-------|
| 100-199 | Great Rune flags | common.emevd.js Event 720/730 | 160-167=possession, 180-187=activation |
| 171-197 | Boss defeat markers | ItemLotParam_map | Links to boss remembrances |
| 9100-9199 | Boss Remembrance possession | common.emevd.js Event 1100 | 91xx = remembrance owned |
| 9200-9281 | Talisman/Container upgrades | common.emevd.js Event 1200 | 92xx = upgrade obtained |
| 60000-60520 | General progression | ItemLotParam_map | Crafting kit, Memory Stones, etc. |
| 62010-62084 | Map fragment possession | common.emevd.js Event 1600 | Links to 63xxx (discovery) |
| 65000-65300 | Whetblade pickups | ItemLotParam_map | World pickup flags |
| 65610-65720 | Whetblade shop unlocks | common.emevd.js Event 1450 | Currently implemented |
| 65810-65879 | Shop item release flags | common.emevd.js Event 65810 | Unlocks items in shops |
| 66000-66990 | Cracked/Ritual Pot pickups | ItemLotParam_map | Container upgrades |
| 67000-67920 | Cookbook pickups | ItemLotParam_map | Cookbook flags |
| 68000-68950 | Cookbook pickups (cont.) | ItemLotParam_map | Currently implemented |
| 69010-69760 | Remembrance duplication | common.emevd.js Event 1720 | Links to boss flags |
| 510xxx | Story item pickup flags | ItemLotParam_map | Major item pickups |
| 520xxx | Upgrade item pickup flags | common.emevd.js Event 1200 | Talisman pouch, etc. |
| 10XXYYZZZZ | World pickups (base game) | ItemLotParam_map | XX=33-54, YY=31-58 |
| 20XXYYZZZZ | World pickups (DLC) | ItemLotParam_map | DLC area pickups |

---

## World Item Pickup Flags (10-digit format)

**Verified Format**: `1XXYYZZZZ` or `2XXYYZZZZ` where:
- `1` = Base game, `2` = DLC
- `XX` = Map tile X coordinate (33-54 for base game)
- `YY` = Map tile Y coordinate (31-58 for base game)
- `ZZZZ` = Local flag index within tile

**Examples from ItemLotParam_map.param.xml**:
- `1033407100` - Map tile m60_33_40
- `1042371690` - Map fragment pickup (Limgrave)
- `1052417100` - Shop release trigger

**Total unique pickup flags**: 4538 (from ItemLotParam_map)

### ItemLotParam Item Categories

| lotItemCategory | Type | EquipParam File |
|-----------------|------|-----------------|
| 1 | Goods (consumables, key items) | EquipParamGoods.param.xml |
| 2 | Weapons (including shields, staves) | EquipParamWeapon.param.xml |
| 3 | Protector (armor) | EquipParamProtector.param.xml |
| 4 | Accessory (talismans) | EquipParamAccessory.param.xml |
| 5 | Ash of War | (separate system) |

---

## Shop/Merchant System (Verified from ShopLineupParam.param.xml)

### Shop Flag Types

| Field | Purpose | Example Values |
|-------|---------|----------------|
| `eventFlag_forStock` | Set when purchased (depletes stock) | 60020, 66030, 67000, 150xxx |
| `eventFlag_forRelease` | Must be ON for item to appear | 0 (always), 10-digit (world pickup), 11xxxxxx (legacy dungeon) |

### Stock Flag Ranges (eventFlag_forStock)

| Range | Category | Example |
|-------|----------|---------|
| 60xxx | General progression | 60020, 60110-60500 |
| 66xxx | Cracked/Ritual Pots | 66030, 66060, 66450 |
| 67xxx | Cookbooks | 67000-67920 |
| 68xxx | Cookbooks (cont.) | 68010-68230 |
| 69xxx | Notes | 69600-69760 |
| 100xxx-130xxx | NPC shop stock | 100000 (Gostoc), 110000 (Patches) |
| 150xxx | Merchant stock | 150050-150900 |

### Release Flag Patterns (eventFlag_forRelease)

| Pattern | Meaning | Example |
|---------|---------|---------|
| 0 | Always available | Most base items |
| 10XXYYZZZZ | World pickup required | 1044369244 (scroll pickup) |
| 11xxxxxx | Legacy dungeon event | 11109874 (Fire Monks' Prayerbook) |
| 14xxxxxx | Legacy dungeon event | 14009267 (Sellen quest) |
| 35xxxxxx | Legacy dungeon event | 35009326 (Dung Eater quest) |

### Merchant Entries (ShopLineupParam row IDs)

| Row ID Range | Merchant |
|--------------|----------|
| 100000-100024 | Gatekeeper Gostoc |
| 100050-100086 | Sorceress Sellen, Knight Bernahl |
| 100100-100124 | Patches, D Hunter |
| 100150-100185 | Blackguard Big Boggart, Gowry |
| 100200-100339 | Rogier, Iji, Seluvis, Pidia |
| 100350-100399 | Brother Corhyn |
| 100500-100520 | Merchant Kale |
| 100525-100547 | Merchant - North Limgrave |
| 100550-100567 | Merchant - East Limgrave |
| 100575-100590 | Merchant - Coastal Cave |

---

## Legacy Dungeon Event Flags (8-digit format)

**Format**: `XXYYYZZZZ` where XX = legacy dungeon map ID

| Map ID | Dungeon | Example Flag |
|--------|---------|--------------|
| 10 | Stormveil Castle | 10000800 (boss) |
| 11 | Raya Lucaria | 11000800 (boss), 11109xxx (events) |
| 12 | Various dungeons | 12010800, 12050800 |
| 13 | Leyndell | 13000800 (boss) |
| 14 | Shunning Grounds | 14000800 (boss) |
| 15 | Haligtree | 15000800 (boss) |
| 16 | Farum Azula | 16000800 (boss) |
| 35 | Mohgwyn Palace | 35000800 (boss) |
| 39 | Deeproot Depths | 39200800 |

---

## Event Script Reference (common.emevd.js)

### Key Events

| Event ID | Purpose | Parameters |
|----------|---------|------------|
| 720 | Great Rune possession | flag, index |
| 730 | Great Rune activation | flag, index |
| 930 | Map fragment events | Various flag mappings |
| 1100 | Boss Remembrance | 91xx flag, ItemLot, pickup flag |
| 1200 | Container upgrades | 92xx flag, ItemLot, pickup flag |
| 1450 | Whetblade unlocks | 656xx flags |
| 1600 | Map fragment collection | 62xxx to 63xxx mapping |
| 1720 | Remembrance duplication | 69xxx, ItemLot, boss flag |
| 65810 | Shop item release | 658xx, shop row, conditions |

---

## Primary Data Sources for Implementation

**Use these decompiled game files (not third-party extractions)**:

1. **ItemLotParam_map.param.xml** - All world pickup flags with `getItemFlagId`
2. **ShopLineupParam.param.xml** - Shop item flags with `eventFlag_forStock/forRelease`
3. **common.emevd.js** - Event logic showing flag relationships
4. **openmap.eventflagalloclist** - Overworld flag to map tile mapping
5. **legacymap.eventflagalloclist** - Legacy dungeon flag allocation

---

## Database Coverage Analysis

**Full analysis**: See `docs/DATABASE_COVERAGE_ANALYSIS.md`

### Key Blindspots Summary

| Gap | Severity | Description |
|-----|----------|-------------|
| Spells | Critical | No spell database (317 in game) |
| Event Flags | Critical | Only ~10% coverage (1,350 of 15,000+) |
| NPCs | High | No NPC tracking module |
| Shop Stock | High | No shop purchase tracking |
| Landmarks | Medium | 236 POIs not named |
| Gestures | Medium | 60 gestures not tracked |

### Priority Additions

1. `db/spells.rs` - From Magic.param (317 entries)
2. Expand `event_flags.rs` - Add 3,000+ flags
3. `db/npcs.rs` - NPC discovery tracking
4. `db/shop_items.rs` - Shop stock flags
