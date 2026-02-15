# Data sources

## Characters:

Slot 0, Confessor - midgame regions discovered, harvested, Margit, Godrick and Radahn defeated. Hundreds of world pickups completed, Limgrave, Caelid, Altus Plateau, Liurnia, Stormveil Castle explored, multiple questline progressions.

Slot 1, Wretch - early game, a few graces and pickups completed, only the tutorial enemy is defeated

Slot 2, V1 - test character, very early game, one world pickup: Flag ID 1044367310
Slot 3, V2 - test character, same as V1, just different travel path taken to the same one world pickup Flag ID 1044367310
Slot 4, V3 - test character, same as V1-V2, traveled to the same location, but did NOT picked up Flag ID 1044367310

Slot 5, Bee - early game, more progress, exploration and world pickups than Slot 1-4. Primary timeline tracking character.

Slot 6, Sam - minimal progression

## Game save files

The save files are legitimate, unaltered files saved by the game.

### Latest

(might contain fresher save slots than the Manual completion log):
'/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2'

### Archived:

'/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files'

### Progressive, before-after save file snapshots for diff

'/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging'

#### Confessor capture pairs (Slot 0)

66 full save file snapshots for before/after verification of pickups, graces, and bosses:
'/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/slot 0 Confessor'

Capture catalog (149 captures, 52 pairs):
'/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/capture_catalog.json'

#### s5-Bee timeline (Slot 5)

701 timeline entries with sparse byte-level diffs. Each .bin file is a sparse diff
(6 bytes per changed byte: `[u32_LE offset][u8 old][u8 new]`), NOT raw slot data.

Metadata:
'/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/timeline/slot_changes.jsonl'

Binary diffs:
'/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/timeline/slot_diffs/'

Key fields in slot_changes.jsonl:
- `structuralOffsets.eventFlagsOffset`: EF offset within slot (available from ~entry 50+)
- `structuralOffsets.gaItemsEnd`: GaItems section end offset
- `structuralOffsets.efConfident`: Whether detection was confident
- `inventoryDelta`: Items added/removed
- `gracesDiscovered`: Grace flag transitions
- `bossesDefeated`: Boss defeat flag transitions

## Decompiled game resource files (single source of truth)

'/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring decompiled game files'

Key files:
- `regulation-bin/ItemLotParam_map.param.xml` - World pickup definitions
- `regulation-bin/ShopLineupParam.param.xml` - Shop stock/release flags
- `regulation-bin/BonfireWarpParam.param.xml` - Grace warp points
- `regulation-bin/WorldMapPointParam.param.xml` - Grace/landmark definitions
- `regulation-bin/Magic.param.xml` - Spell definitions
- `event/common.emevd.js` - Event script logic
- `event/openmap.eventflagalloclist` - Overworld flag allocation
- `event/legacymap.eventflagalloclist` - Dungeon flag allocation

## Manually maintained completion log

A.k.a. User sets completed checkbox to true, then saved via Elden-map apps /character-game-data page. Coverage only for early stage characters, for Slot 0 - Confessor, mostly graces, unique item pickups, boss drops are checked.
'/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/flag-correlation-candidates.jsonl'

## Event flags

Extracted event flag catalog (generated from decompiled game files)
'/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/scripts/extracted_event_flags.json'
