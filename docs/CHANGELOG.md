# Changelog

All notable changes to ER-save-Editor will be documented in this file.

---

## v0.17.12 - Resolve numeric flag names and add event_action property

### Improvements
- **Numeric flag name resolution** — Reduced flags with numeric-only identifiers from ~1,448 to 764 (684 flags improved):
  - **Gesture names**: `load_gesture_names()` from GestureParam.param.xml (51 gestures). "Gesture Unlock (gesture 102)" → "Gesture Unlock (Rapture)"
  - **Entity names for templates**: `entity_names` dict passed to `extract_emevd_templates()`. "Enemy Defeat (10000280)" → "Grafted Scion - Enemy Defeat" (~216 flags)
  - **Region labels**: Door/Mechanism/Treasure flags use region as fallback. "Door Unlock (10000510)" → "Door Unlock (Stormveil Castle)" (~204 flags)
  - **Context verb entity extraction**: Character State, Spawn State, Item Award contexts now extract entity IDs from EMEVD verbs (DisableCharacter, EnableCharacter, etc.) and resolve to names (~100+ flags)
  - **MSB Region entities**: `load_msb_region_names()` parses 4,280 Region/Other XMLs with Japanese→English keyword translation for area_trigger/interaction names

- **`event_action` property** — New `raw_data["event_action"]` field on all EMEVD Literal Flags classifies the immediate EMEVD verb nearest to the SetEventFlagID call. 18 action types (boss_defeated, enemy_killed, gesture_acquired, item_acquired, cutscene_watched, door_opened, etc.). 937 of 1,360 EMEVD Literal Flags classified.

### New Functions
- `load_gesture_names()`: GestureParam.param.xml → gesture ID→name mapping
- `load_msb_region_names()`: MSB Region/Other XMLs → region entity ID→translated label mapping
- `JAPANESE_REGION_KEYWORDS`: 32-entry translation table for MSB region name keywords

### Call Site Changes
- `build_entity_name_map()` moved before `extract_emevd_templates()` in `main()` (was after)
- `extract_emevd_templates()` now accepts `entity_names` and `gesture_names` parameters
- `resolve_emevd_literal_names()` now accepts `gesture_names` and `region_entities` parameters

### Files Modified
- `scripts/extract_event_flags.py`: All changes (new loaders, enriched name resolution, event_action property)
- `scripts/extracted_event_flags.json`: Regenerated with improved names and event_action
- `scripts/extracted_event_flags.md`: Regenerated
- `docs/BACKLOG.md`: Updated Gesture Database status
- `docs/CHANGELOG.md`: v0.17.12
- `Cargo.toml`: bumped to 0.17.12

## v0.17.11 - Fix extraction categorizer priority and wrong hardcoded names

### Fixes
- **Categorizer priority bug** — Source-based checks (`ShopLineupParam.release → "Shop Unlock"`) now run before 91xx-95xx ID-range checks. Previously, ~20 Enia shop unlock flags (9101, 9104, 9107, etc.) were misclassified as "Remembrance" because the overbroad `9100-9199` range check ran first.
- **"Talisman Pouch" → "Boss Reward"** — The 9200-9299 range contains dungeon boss reward triggers (Cemetery Shade, Erdtree Burial Watchdog, etc.), not talisman pouches. Only 3 of ~60 flags in this range are actual talisman pouches. Renamed category throughout.
- **Removed wrong hardcoded entries** from `extract_common_emevd()`:
  - Remembrance (9100-9114): 6 of 15 names were wrong. These flags are now correctly sourced from ItemLotParam and ShopLineupParam.
  - Talisman Pouch (9200-9202): Now sourced from EMEVD boss trace resolution.
  - Mending Rune (9500-9502): 9500 was hardcoded as "Fell Curse" but is actually "Perfect Order" per ItemLotParam. 9504 was missing entirely.
- **Great Rune milestone flags** (160-167, 180-187) — Renamed from per-rune names ("Godrick's Great Rune - Possessed") to threshold milestone names ("Boss Drop Milestone: N+ Remembrances Collected"). These use `CountEventFlags >= threshold` where threshold=0 is always true, so flags 160/180 are set for ALL characters regardless of progression.

### Key Findings
- The 91xx range is a MIX of boss reward triggers (from EMEVD Event 1100) and Enia shop unlock flags (from ShopLineupParam). Sequential hardcoding was fundamentally wrong.
- EMEVD Events 720/730 use `CountEventFlags(range) >= threshold` — threshold=0 means the flag is always set, making flags 160 and 180 default-true for every character.

### Files Modified
- `scripts/extract_event_flags.py`: Fixed categorizer priority, renamed "Talisman Pouch" → "Boss Reward", removed hardcoded entries, fixed milestone names
- `scripts/extracted_event_flags.json`: Regenerated with corrected categories and names
- `scripts/extracted_event_flags.md`: Regenerated
- `docs/CHANGELOG.md`: v0.17.11
- `Cargo.toml`: bumped to 0.17.11

## v0.17.10 - EMEVD event context name resolution

### Extraction: EMEVD Name Resolution
- **New post-processing step** — `resolve_emevd_literal_names()` traces EMEVD event chains to resolve cryptic "Map Event Flag (N)" names to descriptive labels.
- **1,147 of 1,449 flags resolved** (79% coverage):
  - **Boss Reward (55/59)**: Traced `HandleBossDefeatAndDisplayBanner` → boss name lookup. e.g. `Map Event Flag (9206)` → `Boss Reward (Spiritcaller Snail)` _(category renamed from "Talisman Pouch" in v0.17.11)_
  - **Remembrance (17/17)**: Same boss-trace technique. e.g. `Map Event Flag (9163)` → `Remembrance (Bayle the Dread)`
  - **Progression (9/9)**: Context-dependent — boss defeats, gesture unlocks
  - **Mausoleum Duplication (4/4)**: Named by dungeon location
  - **EMEVD Literal Flags (1,066/1,360)**: Classified by surrounding code context into 9 types: Boss Defeat, Enemy Defeat, Cutscene Trigger, Gesture Unlock, Network State, Character State, Spawn State, Item Award, Door State
- **Boss/enemy name enrichment** — Boss Defeat and Enemy Defeat flags include the actual boss/enemy name when the entity ID exists in the database. e.g. `Boss Defeat Flag (30030800)` → `Boss Defeat (Spiritcaller Snail)`
- **Cutscene/gesture specifics** — Cutscene flags include cutscene ID, gesture flags include gesture ID

### Files Modified
- `scripts/extract_event_flags.py`: Added `resolve_emevd_literal_names()` (~170 lines), called in `main()` post-processing
- `scripts/extracted_event_flags.json`: Regenerated with resolved names
- `scripts/extracted_event_flags.md`: Regenerated

## v0.17.9 - Dungeon grace resolution and corroboration cleanup

### WASM: Dungeon Grace Resolution
- **Sub-block/main-block split** — Replaced single `get_block_bases()` HashMap with `get_sub_block_bases()` (100-granularity, checked first) and `get_main_block_bases()` (1000-granularity, fallback). This allows key `71000` to map to base `9315` for Stormveil graces (71000-71099) AND base `2625` for dungeon graces (71100-71799).
- **~80 dungeon graces unlocked** — Flags 71100-71799 (Leyndell, Underground, Farum Azula, Raya Lucaria, Haligtree, Volcano Manor, Fractured Marika) now resolve correctly via `calculate_simple_flag_offset()`.
- **6 new unit tests** — Stormveil sub-block routing, main-block fallback, tutorial grace, world grace regression, Leyndell grace, no-conflict validation. All 51 WASM tests pass.

### Corroboration: False-Alarm Cleanup
- **`skip_corroboration` field** — Added to `FlagRelationship` and `RawEdge` structs (`#[serde(default)]`), honored in both `check_corroboration()` loop and `corroboration_pairs` construction.
- **16 edges marked** — All are `pickup_sets_flag` edges where the tile-side flag is the row_id position (never written by the game). Includes 11 map fragments, 2 Memory Stones, Whetstone Knife, Flask of Wondrous Physick, Golden Tailoring Tools.
- **Extraction script updated** — `SKIP_CORROBORATION_PAIRS` set ensures regeneration preserves the field.

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: sub-block/main-block split + 6 tests
- `src/discovery/relationship_graph.rs`: skip_corroboration field + guard
- `src/discovery/corroboration.rs`: skip_corroboration early continue
- `scripts/flag_relationships.json`: 16 edges marked (32 total across both sections)
- `scripts/extract_flag_relationships.py`: SKIP_CORROBORATION_PAIRS for regeneration
- `docs/EVENT-FLAG-GEOGRAPHY.md`: dungeon grace block ranges + routing explanation
- `docs/WASM-EVENT-FLAGS.md`: flag offset resolution section
- `docs/BACKLOG.md`: blocks 71000/71100 resolved

---

## v0.17.8 - EF geography: simple flags, item acquisition tables, structural chain evidence

### Ground Truth Expansion
- **Simple flags (flag_id < 60,000)** — ML clustering on 799 timeline diffs identified 132 active byte offsets in EF+1040-1259 (flag IDs 8320-10079). Cross-referenced with EMEVD/param data to document 133 known flags across 5 categories: Remembrance (56), Talisman Pouch (63), EMEVD/Shop (9), Mending Rune (4), Unknown (2).
- **EF layout map** — New `ef_layout_map` section in ground_truth_offsets.json documents non-bitfield regions within the EventFlags array (item acquisition tables, structured data zones).

### Event Flag Geography Documentation
- **Simple flag formula**: `byte_offset = flag_id / 8, bit = 7 - (flag_id % 8)` for flags < 60,000
- **Item acquisition tables**: Two sorted 8-byte record tables within the EF array (EF+2208 and EF+32640) tracking items the character has ever obtained, using category prefixes (0x00=Weapon, 0x10=Armor, 0x20=Accessory, 0x40=Goods, 0x80=Custom).
- **MOEG/FOEG system**: Documented the dense state tracking region following EventFlags.

### Registry Updates
- `system.event_flags_raw`: Added evidence for structural section chain verification (GaItems→EF validated at 0x36CB5 for Bee slot 5) and browser WASM initialization fix.

## v0.17.7 - PlayerGameData unknown byte discoveries

### Save Format Discoveries
- **Flask Allocation (`_0x1a[3:5]`)** — identified with HIGH confidence (0.95)
  - `_0x1a[3]` = Crimson Flask charges, `_0x1a[4]` = Cerulean Flask charges
  - Verified via multi-save differential (5 slots × 2 saves) + Bee timeline (689 snapshots, 6 flask transitions)
  - Golden Seeds collected as inventory items; allocation only updates at grace rest (confirmed 11/11 pickups)
  - Monotonically non-decreasing across entire timeline; constant-sum invariant within periods
- **Flask Upgrade Data (`_0x1e[1]`)** — identified with MEDIUM-HIGH confidence (0.8)
  - byte[1] = Sacred Tear count (0→1 after applying 1 Sacred Tear at grace)
  - Consistent with Confessor's byte[1]=7 (7 Sacred Tears collected)
- **Defense Ratings (`_0x28`)** — 7 equipment+level dependent u32 values
  - Naked L1 Wretch: uniform 90 base; armored L1: 140-200 range; scales with level (+1-4 per level)

### Registry Updates
- Moved `flask_configuration` and `flask_charges` from `unknown` → `character_identity` group
- New features: `character_identity.flask_allocation` (0.95), `character_identity.flask_upgrade_data` (0.80)
- Coverage: 51 verified, 15 partial, 13 identified_unparsed (was 11), 10 unknown (was 12)

### Verification Method
- Multi-save differential: 5 character slots across backup (Jan 11) and latest save files
- Temporal timeline: Bee (slot 5) — 799 captures, 689 valid reconstructed snapshots
- Reverse diff reconstruction from latest save state through sparse binary diffs

### Files Modified
- `save_slot_registry.json`: flask features relocated, evidence added, confidence updated
- `docs/CHANGELOG.md`: v0.17.7 entry
- `Cargo.toml`: bumped to 0.17.7

---

## v0.17.6 - Save slot feature registry

### Documentation
- **Created `save_slot_registry.json`** — central registry of all 89 features stored in a character save slot, organized into 8 groups (character_identity, equipment, inventory, unlocks_progression, world_state, network, system, unknown)
- Coverage: 51 verified, 15 partial, 11 identified_unparsed, 12 unknown
- Each feature has stable dot-notation IDs (e.g., `character_identity.level`, `unlocks.graces_overworld`) for cross-referencing
- References `ground_truth_offsets.json` via pointers — no duplication, no code consumer changes
- Integrated registry maintenance into discovery workflow (`docs/discovery-verification-cycle.md` Phase 7 + Verification Checklist)
- Added registry to commit protocol decision table (`docs/COMMIT-PROTOCOL.md`)
- Added documentation table entry in `CLAUDE.md`

### Files Modified
- `save_slot_registry.json`: new central registry (89 features across 8 groups)
- `docs/discovery-verification-cycle.md`: registry update steps in Phase 7 + Prerequisites + Verification Checklist
- `docs/COMMIT-PROTOCOL.md`: Registry column in decision table + documentation triggers
- `CLAUDE.md`: documentation table reference
- `Cargo.toml`: bumped to 0.17.6

---

## v0.17.5 - Regenerate merged POI database with AEG pickups

### Database
- **Regenerated `merged-pois.json`** with 20,456 game POIs (up from 4,563), incorporating 15,893 AEG pickups
- **23,278 total merged locations** (was ~7,407): 2,764 merged + 17,671 game-only + 2,843 MapGenie-only
- Match breakdown: 1,944 by event flag, 596 by title, 224 by coordinate, 12 enriched with event flags
- 289 POIs now carry linked flags from causal graph

### Key Findings
- Previous merge was run on Feb 17 before AEG pickups were added on Feb 18, causing all AEG pickup POIs to appear as unmatched MapGenie-only entries
- Re-running confirms merge logic correctly handles AEG pickups via title+coordinate matching (e.g., Miquella's Lily matched at distance 0.001416, well within 0.008 threshold)

### Files Modified
- `elden-map/public/data/merged-pois.json`: regenerated (23,278 locations)
- `elden-map/server/data/game-pois/merge-report.json`: regenerated
- `elden-map/server/data/flag-correlation-candidates.jsonl`: regenerated
- `docs/CHANGELOG.md`: v0.17.5
- `Cargo.toml`: bumped to 0.17.5

---

## v0.17.4 - Raw Data pane for MapGenie-only POIs

### Features
- **Raw Data JSON pane** added to MapGenie-only POI detail panel on `/character-game-data`
  - Displays the full original `MapLocation` object (latitude, longitude, region, image, poiSource, etc.) that was previously discarded during POI construction
  - Copy button with same gold/teal feedback styling as the flag detail panel
  - Scrollable `<pre>` with 10px monospace font matching existing Raw Data pane

### Implementation
- Extended `MapGeniePOI` interface with optional `_sourceLocation` field to carry the full original `MapLocation`
- `mapGenieOnlyPois` builder now preserves the source `MapLocation` via `_sourceLocation`
- MapGenie-only panel looks up the original `MapGeniePOI` by ID to resolve source data, since the map component converts `MapGeniePOI` → synthetic `POI` for click callbacks

### Files Modified
- `elden-map/src/components/character-viewer/CharacterViewerMap.tsx`: extend `MapGeniePOI` interface
- `elden-map/src/pages/CharacterGameDataPage.tsx`: pass source data + add Raw Data pane
- `docs/CHANGELOG.md`: v0.17.4
- `Cargo.toml`: bumped to 0.17.4

---

## v0.17.3 - AEG pickup extraction with renewability metadata

### Database
- **15,893 AEG (AssetEnvironmentGeometry) pickups extracted** from MSB Part/Asset files, up from 0
  - 14,525 renewable (respawning on grace rest): Rowa Fruit, Erdleaf Flower, Mushroom, etc.
  - 1,368 one-time harvest (permanently consumed): Smithing Stones, Gloveworts, Trina's/Miquella's Lily
- **Behavior classification**: each AEG pickup tagged with `aeg_behavior` (bush/breakable/one_time_harvest) and `renewable` boolean
- **Item naming fix**: same-item quantity variants no longer produce "Rowa Fruit (+3 more)" — shows just "Rowa Fruit" when all slots share the same name

### Implementation
- `load_aeg_param()`: new function parses AssetEnvironmentGeometryParam, classifying pickups by `isEnableRepick`, `isBreakOnPickUp`, `isHiddenOnRepick` flag combinations
- `extract_aeg_pickups()`: scans MSB Part/Asset dirs for AEG099_* models, resolves items via ItemLotParam_map, generates synthetic flag IDs (`3B + area*10M + gridX*100K + gridZ*1K + instance`)
- Deduplication: AEG pickups whose item lot already appears in Treasure event flags are skipped to avoid double-counting

### Key Findings
- `isEnableRepick=1` means NON-respawning (one-time harvest) — the repick mechanism tracks picked state persistently, `isHiddenOnRepick=1` hides the model permanently
- `isEnableRepick=0` means RESPAWNING — no persistent state tracking, resets on grace rest
- `isHiddenOnRepick` always equals `isEnableRepick` across all 324 param rows

### Files Modified
- `scripts/extract_event_flags.py`: add `load_aeg_param()` and `extract_aeg_pickups()` functions
- `scripts/extracted_event_flags.json`: regenerated (4,563 → 20,456 positioned flags)
- `scripts/extracted_event_flags.md`: regenerated

## v0.17.2 - MSB enemy position resolution for item drops

### Features
- **EMEVD→ItemLotParam position backfill**: 158 flags that previously lacked coordinates now inherit positions from their drop source enemy's MSB entity data
- **item_lot_positions map**: `extract_emevd_templates()` now collects a mapping of item_lot_id → position data during template processing, resolving 173 unique item lot positions
- **Relationship graph extension**: new `enemy_drops_item` edge type in `extract_flag_relationships.py` linking defeat flags to item acquisition flags (245 relationships)

### Implementation
- Restructured EMEVD template loop to resolve entity data BEFORE dedup check, ensuring positions are captured even for deduplicated flags
- Post-processing pass matches positionless ItemLotParam flags by `source_row_id` against the item_lot_positions map
- Backfilled flags receive full spatial enrichment: local coords, world coords, map tile, region, area type, DLC classification
- Provenance tracked via `position_source: "EMEVD_Enemy"`, `enemy_entity_id`, `enemy_model`, `source_emevd`

### Impact
- Spatial coverage: local coords 4,405→4,563 (49%→50%), new `enemy_drop` treasure type (158 flags)
- Categories resolved: Ash of War Drop (129), Spirit Ash Drop (59), Boss Drop (56), Crystal Tear DLC (8)
- Flag relationship graph: 2,796→3,041 total relationships (+245 enemy_drops_item)

### Files Modified
- `scripts/extract_event_flags.py`: restructure `extract_emevd_templates()` return type and flow; add backfill post-processing in `main()`
- `scripts/extract_flag_relationships.py`: add `extract_emevd_enemy_item_relationships()` function and wire into `main()`
- `scripts/extracted_event_flags.json`: regenerated with backfilled positions
- `scripts/extracted_event_flags.md`: regenerated
- `scripts/flag_relationships.json`: regenerated with enemy_drops_item edges

## v0.17.1 - Classify Unknown flags by acquisition method

### Database
- **7 new extraction categories** for 582 previously "Unknown" ItemLotParam flags:
  - Quest Reward (238): NPC quest items, bell bearings, event rewards (400K block)
  - Ash of War Drop (129): boss/quest ashes of war (540K block)
  - Spirit Ash Drop (59): spirit ash summons from events (520K block)
  - Boss Drop (56): boss weapon/item drops (530K block)
  - Boss Reward (49): remembrances and boss rewards (510K block)
  - Tutorial (30): info/tutorial popup items (550K block)
  - Painting (21): collectible paintings (580K block)
- Only 11 flags remain as "Unknown" (misc edge cases)

### elden-map
- Registered 7 new category colors and filter group assignments
- Added `inferMarkerType()` mappings for new categories

### Files Modified
- `scripts/extract_event_flags.py`: block-range rules in `categorize_flag()`
- `scripts/extracted_event_flags.json`: regenerated
- `scripts/extracted_event_flags.md`: regenerated

### elden-map Files Modified
- `src/types/eventFlag.ts`: 7 category colors, group assignments
- `src/utils/categoryMapping.ts`: inferMarkerType mappings

## v0.17.0 - Extractor enrichment & elden-map schema alignment

### Features
- **Structured items array** — `extract_item_lot_param()` now builds an `items` list from all 8
  ItemLot slots with `{id, category, category_name, name, quantity}` per entry (4,382 flags).
- **Boss enrichment** — `extract_game_area_param()` populates `boss_type`, `boss_location`, and
  `rune_reward` on Boss Arena/Discovery flags (61 flags).
- **Shop enrichment** — `extract_shop_lineup_param()` parses merchant name from `[brackets]` in
  paramdexName and populates `shop_flag_type`, `merchant`, `shop_item_name`, `equip_type`, `price`,
  `sell_quantity`.
- **Dungeon type derivation** — Post-processing pass assigns `dungeon_type` from `area_no` via
  `DUNGEON_TYPE_MAP` (11 types: catacombs, cave, tunnel, hero_grave, legacy_dungeon, etc.).
- **Spirit ash detection** — `load_item_rarities()` now detects `goodsType=8` items from
  EquipParamGoods and sets `spirit_ash_name` on matching ItemLot flags.
- **Chest indicator** — `in_chest` field derived from `treasure_type == 'chest'` (201 flags).
- **10 new EMEVD categories** — Door Unlock, Mechanism Unlock, EMEVD Treasure, Gesture Unlock,
  Quest Completion, Quest State, NPC Death Quest, NPC Defeat, Map Point Discovery, EMEVD Literal
  Flag registered with colors and category groups in elden-map.
- **Adapter wiring** — `worldX`, `worldZ`, `areaType`, `isOverworld` fields added to
  `GameFileEventFlag` and wired through `adaptExtractedFlag()`.
- **DRY coordinate transforms** — Extracted `SCALE_X/Z`, `OFFSET_X/Z` to
  `src/utils/coordConstants.ts`; replaced hardcoded constants in 6 files across scripts and
  components.

### Files Modified
- `scripts/extract_event_flags.py`: 12 new EventFlag fields, items array builder, boss/shop/dungeon
  enrichment, spirit ash detection, `get_dungeon_type()` helper
- `scripts/extracted_event_flags.json`: regenerated with enrichment data
- `scripts/extracted_event_flags.md`: regenerated

### elden-map Files Modified
- `src/types/eventFlag.ts`: 4 new GameFileEventFlag fields, 10 category colors, 2 new groups
- `src/services/data/eventFlagAdapter.ts`: world coords and area classification wiring
- `src/utils/coordConstants.ts`: new single source of truth for transform constants
- `src/utils/measurementUtils.ts`: imports from coordConstants
- `scripts/build-game-pois.ts`: uses coordConstants import
- `scripts/merge-poi-databases.ts`: uses coordConstants import
- `scripts/build-event-flag-mappings.ts`: uses coordConstants import
- `src/pages/GameMapPage.tsx`: uses coordConstants import
- `src/components/game-map/GameMap.tsx`: uses coordConstants import
- `src/components/measurement/SnappingStatsPanel.tsx`: uses coordConstants import

## v0.16.5 - Stats and Equipment views use shared components

### Refactor
- **Stats view** — Replaced monospace `display_stat_row` rendering with `ExportToolbar` + `UnifiedTable`.
  Sortable Stat/Value columns, export to JSON/CSV/Markdown, double-click row copy.
- **Equipment view** — Replaced monospace `display_equipment_row` rendering with `FilterBar` (search) +
  `ExportToolbar` + `UnifiedTable`. Five sortable columns (Category, Slot, Item Name, Item ID, GA Handle),
  empty/unarmed slots shown in dark gray, fuzzy search across all 30 equipment slots, export support.
- Added `table_state`, `export_format` to `StatsViewModel` and `table_state`, `filter_state`,
  `export_format` to `EquipmentViewModel` for UI state management.

### Files Modified
- `src/vm/stats.rs`: added TableState and ExportFormat fields
- `src/vm/equipment.rs`: added TableState, FilterBarState, ExportFormat fields
- `src/ui/stats.rs`: full rewrite using ExportToolbar + UnifiedTable
- `src/ui/equipment.rs`: full rewrite using FilterBar + ExportToolbar + UnifiedTable
- `docs/CHANGELOG.md`: version 0.16.5
- `Cargo.toml`: bumped to 0.16.5

---

## v0.16.4 - Fix world pickup getItemFlagId routing

### Bug Fixes
- **Use getItemFlagId instead of row_id for tile-based world pickups** — The extraction
  script was using ItemLotParam row IDs (e.g., 1045371000, local_id=1000) as event flag IDs.
  The game actually stores flags at the getItemFlagId position (e.g., 1045377100, local_id=7100),
  which converts to tile local_id=100 after subtracting 7000.
- **Route getItemFlagId through tile formula instead of row_id formula** — The WASM
  `calculate_tile_flag_offset_unified()` was sending converted flags to
  `calculate_world_pickup_offset_by_row_id_impl()` (byte ~999K in EF), but the game stores
  them in the tile region via the standard tile formula (byte ~763K). Same fix applied to
  the elden-map server's `getAllSetFlags()`.

### Key Finding
- Unique items (talismans, weapons, armor) from chests DO set event flags via getItemFlagId.
  Consumable/stackable items (Golden Runes, Smithing Stones) still do NOT set any event flag,
  confirming the finding in `EVENT-FLAG-TREASURE-DISCREPANCY.md`.
- Empirically verified with Axe Talisman (getItemFlagId 1045377100): SET at tile (45,37)
  local_id=100 in the save file; CLEAR at the row_id formula offset.

### Files Modified
- `scripts/extract_event_flags.py`: use getItemFlagId for tile-based pickups
- `scripts/extracted_event_flags.json`: regenerated with correct flagIds
- `scripts/extracted_event_flags.md`: regenerated
- `src/db/world_pickups.rs`: regenerated via extract_world_pickups.py
- `crates/wasm-event-flags/src/lib.rs`: route getItemFlagId to tile formula, update tests
- `../elden-map/server/src/eventFlagService.ts`: route converted flags to calibrated tile formula
- `../elden-map/wasm-event-flags/`: rebuilt WASM binary

---

## v0.16.3 - Correct tile base offset for world pickup detection

### Bug Fix
- **Corrected TILE_BASE_OFFSET from 485330 to 337375** (was 147,955 bytes too high)
- The old value was derived using an earlier incorrect EF offset formula; when EF detection was corrected, the tile base was not recalibrated
- This fixes detection of all 69 tile-type world pickup flags (10-digit flags with localId < 7000)

### Key Findings
- Tile base within EventFlags is **constant across all characters** (337375), not variable as previously assumed
- Verified via before/after snapshot diffs across 3 characters (Confessor, V1, Slot7) and 10+ capture pairs
- Old calibration search range (480000-560000) was entirely wrong; corrected to 327000-347000
- The Whetstone Knife tile flag (1042371010) is unreliable as a calibration anchor since the item is usually obtained from a chest (flag 1042371300), not the world pickup

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: TILE_BASE_OFFSET 485330→337375, updated 4 tests
- `ground_truth_offsets.json`: tile_formula.base_offset, calibration anchor, 60 tile flag offsets
- `src/calibration.rs`: updated test assertions
- `src/db/pickup_flags.rs`: updated 2 test assertions
- `src/discovery/offset_probe.rs`: updated tile_base (was 489981)
- `docs/SAVE_FILE_GROUND_TRUTH.md`: corrected tile base references
- `docs/DATABASE_COVERAGE_ANALYSIS.md`: corrected tile base reference
- `CLAUDE.md`: corrected tile base documentation
- WASM rebuilt and deployed to elden-map
- elden-map: updated calibrationService.ts, eventFlagService.ts, shared/wasm-loader.ts

## v0.16.2 - Fix emevd block base off-by-one

### Bug Fix
- **Applied +1 byte correction** to 7 emevd-derived block bases: 65000, 66000, 67000, 68000, 69000, 91000, 92000
- The raw hex values from `common.emevd.js` are off by 1 for these blocks — they point to a header/alignment byte, not the first flag data byte
- Blocks 60000 and 62000 do NOT need the correction (their emevd hex values are exact)
- At the old bases, all decoded flags ended in `...8` (non-round); at corrected bases, **100% of flags are multiples of 10**, matching Elden Ring's flag naming convention

### Verification
- Block 67000: 6/6 SET flags mod10=0 (67890, 67900, 67920, 67960, 67970, 67980)
- Block 68000: 16/16 SET flags mod10=0
- Block 69000: 20/20 SET flags mod10=0
- Block 91000: 41/41 SET flags mod10=0
- Block 92000: 16/16 SET flags mod10=0
- Blocks 65000, 66000: empty in test save, corrected by pattern extrapolation (5/5 verified blocks needed +1)

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: corrected 7 bases in `get_block_bases()`
- `ground_truth_offsets.json`: updated `base_offset` and notes for 7 blocks

---

## v0.16.1 - Correct Non-Grace Block Bases

### Bug Fix
- **Corrected block base offsets** for non-grace event flag categories (progression, maps, whetblades, cookbooks, etc.) that were calibrated against a false-positive EF offset in the GaItemData section
- Old bases (e.g. 62000→9359, 67000→37411) were checking bytes deep in intermediate save sections, not actual EventFlags
- New bases sourced from `common.emevd.js` game event scripts: 60000→1260, 62000→1500, 65000→1684, 66000→1724, 67000→1764, 68000→1804, 69000→1844, 91000→2384, 92000→2424
- Added 4 new block entries (66000, 69000, 91000, 92000); removed incorrect 61000 entry

### Verification
- Map fragment base 1500 verified via 6 timeline diffs with exact bit-level matches
- Cross-validated across 3 character slots (Confessor mid-game, Wretch early, Bee extensive) with progression-appropriate results
- Grace bases (2725, 3250) confirmed unaffected — they were already correct

### Key Finding
- The old "verified 12/12 match" was a false positive: the old bases mapped to byte positions within the GaItemData section (~37K into the slot), which contains non-zero structured data that coincidentally passed bit checks. The correct bases are all within the first ~4K bytes of the EF section, consistent with the system flag allocation layout in `common.emevd.js`.

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: corrected `get_block_bases()`, fixed cookbook test
- `ground_truth_offsets.json`: updated block_bases with correct values
- `src/db/pickup_flags.rs`: updated crystal tears test assertions
- `scripts/verification/block_items.json`: updated bases for blocks 62000, 67000, 68000
- `scripts/verification/test_formulas.py`: updated expected byte offsets

---

## v0.16.0 - Structural EventFlags Detection

### Features
- **Structural offset computation** replaces content-based search as the primary EventFlags detection method
- Sequential section parsing from GaItems through TutorialData deterministically computes the EventFlags offset without searching for grace flag patterns
- Handles two variable-size sections: EquipProjectileData (4 + count×8) and Regions (4 + count×4)
- Pre-EventFlags gap empirically verified as constant 29 bytes (0x1D) across 898 slot measurements
- Content-based search retained as fallback only for corrupted/unknown formats
- Works for brand-new characters with zero graces (content-based cannot)

### Implementation
- Added 30+ section size constants to WASM module mirroring `save_slot.rs` parsing chain
- `compute_structural_ef_offset()`: deterministic offset from sequential section sizes
- `validate_at_offset()`: extracted grace flag validation as reusable helper
- `detect_event_flags_content_based()`: legacy search isolated as fallback
- Native wrapper trusts `confident: true` from structural detection
- New WASM export: `compute_structural_event_flags_offset()`
- 7 new tests for structural detection

### Verification
- `scripts/verification/measure_pre_ef_gap.py`: empirical gap measurement across all save data
- `scripts/verification/verify_captures.py`: capture pair verification framework
- `scripts/verification/verify_pickups.py`: pickup verification framework
- `scripts/verification/verify_timeline.py`: timeline verification framework
- Improved Python EF detection: 0xFF padding rejection, better candidate ranking

### Key Findings
- The pre-EventFlags gap is constant at 29 bytes regardless of character progression
- Content-based detection produced false positives for mid-game and test characters
- Structural detection eliminates all false positives by computing the exact offset

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: structural computation, section constants, 7 new tests
- `src/save/common/event_flags_detection.rs`: updated fallback logic, docstrings
- `docs/DATA-SOURCES.md`: documented capture pairs and timeline data sources
- `scripts/verification/save_parser.py`: 0xFF rejection, improved candidate ranking
- `scripts/verification/utils.py`: 0xFF rejection, robust detection function
- `scripts/verification/ground_truth_loader.py`: None guard in block offset calculation
- `scripts/verification/measure_pre_ef_gap.py`: new empirical measurement script
- `scripts/verification/verify_captures.py`: new capture verification framework
- `scripts/verification/verify_pickups.py`: new pickup verification framework
- `scripts/verification/verify_timeline.py`: new timeline verification framework

---

## v0.15.1 - Fix EventFlags Detection False Positives

### Bug Fixes
- **SEARCH_START raised from 0x12000 to 0x30000**: inventory data at ~76K contained coincidental bit patterns that matched positive validation flags, causing the detector to return a false-positive offset ~146K below the real EventFlags section
- **Removed early return on first perfect match**: the algorithm now collects ALL candidates and selects the best, preventing premature lock-on to false positives
- **Tiebreaker changed to prefer highest offset**: when candidates have equal scores, the last (highest) valid match is selected — empirically validated across 701 captures showing the real EF copy is always the last one (2622 bytes after false copies)
- **Fixed mislabeled validation flag**: flag 76102 was labeled "Gatefront Ruins" but is actually "Stormhill Shack" (real Gatefront is flag 76111)
- **Updated fallback offset in save_slot.rs**: from 0x12B00 to 0x36500 to match the real EF region

### Key Findings
- Real EventFlags offset is ~222K-225K (0x36000-0x37000), NOT ~76K-78K
- The gap between gaItemsEnd and EventFlags grows monotonically during gameplay (+4/+8 byte increments), making any fixed formula unreliable
- Dynamic detection via validation flag scanning is the only correct approach
- Verified stable across 701 captures spanning 14.5 hours of gameplay, 9 area codes, 83 map tiles

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: SEARCH_START, early return removal, tiebreaker, flag label fix
- `src/save/common/event_flags_detection.rs`: test assertion update
- `src/save/common/save_slot.rs`: local SEARCH_START and FALLBACK_OFFSET constants
- `docs/SAVE_FILE_GROUND_TRUTH.md`: corrected EF offset range from 0x12B00 to 0x36000

---

## v0.15.0 - Unified Flag Resolution & Multi-Tile Calibration

### Features
- **Unified flag offset routing** (`get_flag_offset`, `get_flag_offset_calibrated`): single dispatcher handling all flag ranges — tile (1B+), dungeon (8-digit), midrange (6-digit), block/simple (< 100k)
- **Block/midrange/dungeon base maps in WASM**: 12 block bases (60k–78k), 3 midrange bases (510k–710k), ~40 dungeon area+section tuples — all synced from `ground_truth_offsets.json`
- **Multi-tile calibration**: replaced single-anchor calibration with 4-anchor constraint satisfaction from 2+ distinct tiles (Python + Rust), reducing false positives to near-zero
- **Position validation**: reject candidates with denormalized float coordinates or extreme facing angles (|angle| > 2π)
- **Equipment extraction in WASM**: `parse_quick_items_data()`, `parse_equipped_items_data()` for equipped slots, talismans, quick items, pouch
- **`verify-anchors` CLI command**: matrix display of tile pickup anchors across multiple slots for calibration anchor discovery
- **Timeline analysis scripts**: binary diff parsing, grace/pickup extraction, and gameplay narrative reconstruction from granular snapshots

### Implementation
- `get_flag_offset_with_tile_base()`: routes 1B+ → tile formula (local_id < 7000) or row_id formula (≥ 7000); 8-digit → dungeon; 6-digit → midrange lookup; < 100k → block/simple
- Multi-tile calibration searches 430k–510k for candidates matching ≥ 3 anchors from ≥ 2 distinct tiles simultaneously
- `is_denormalized(v: f32)`: checks exponent bits == 0 with non-zero value; `FACING_ANGLE_MAX = TAU`
- `CorroborationEngine` now accepts calibrated tile_base via `with_calibrated_tile_base()`
- Test cases updated with empirical world pickup data from timeline capture files 119–127

### Files Modified
- `crates/wasm-event-flags/src/lib.rs`: unified flag resolution, block/midrange/dungeon bases, position validation, equipment extraction, 13 new tests
- `scripts/verification/calibration.py`: multi-tile calibration anchors, multi-constraint search
- `src/calibration.rs`: Rust multi-tile calibration (parallel to Python)
- `src/discovery/cli.rs`: calibrated corroboration, `verify-anchors` command
- `src/discovery/corroboration.rs`: `calibrated_tile_base` field and setter
- `src/discovery/test_cases.rs`: empirical tile-formula test cases, calibrated validation
- `scripts/timeline_analysis.py`: new — binary diff analysis
- `scripts/timeline_graces_pickups.py`: new — grace/pickup extraction from timeline
- `scripts/timeline_narrative.py`: new — gameplay event reconstruction

---

## v0.13.3 - Player Coordinate Extraction Verification

### Changes
- Add `scripts/verification/verify_player_coords.py`: signature-based PlayerCoords extraction from save file snapshots
- Validates extracted coordinates against known grace/boss world positions across 15 test cases (2 character slots, 7 locations)
- Extraction method: searches for slot header `map_id` pattern in the 0x1D0000–0x280000 range, validates surrounding padding bytes (17+16 byte blocks), reads 3×f32 coordinates
- Validation guards: coordinate range (±10,000), magnitude threshold (>10), NaN/Inf rejection, false-positive filtering via padding zero-count scoring

### Key Findings
- Structural parsing (EventFlags→UknownLists→PlayerCoords) fails due to EF offset false positives; signature-based search is reliable
- PlayerCoords `padding2` (16 bytes after coords2) being mostly zeros is the strongest discriminator
- Typical extraction accuracy: 1–14 game units from reference positions for graces, 5–35 units for boss arenas

### Files Modified
- `scripts/verification/verify_player_coords.py`: new file

---

## v0.13.2 - Documentation Audit, Cleanup & Restructuring

### Changes
- Rewrote `docs/DATABASE_COVERAGE_ANALYSIS.md` to reflect current state (40 modules, ~22,184 entries)
- Fixed contradictions across docs: tile base_offset 489981→485330, consumable tracking now TRACKABLE, deprecated flag_formulas.py references removed
- Deduplicated inline formulas in discovery-verification-cycle.md and CORROBORATION-SYSTEM.md with cross-references to EVENT-FLAG-GEOGRAPHY.md
- Archived 9 stale documents to `docs/archive/` with archive headers
- Created `docs/BACKLOG.md` consolidating all scattered "Next Steps" into one prioritized list
- Updated CLAUDE.md doc table (added CASE-VERIFICATION-GUIDE, SAVE_FILE_GROUND_TRUTH, DATA-SOURCES, BACKLOG)
- Updated ARCHITECTURE.md methodology table and COMMIT-PROTOCOL.md references
- Updated `/snapshot` command to keep BACKLOG.md up to date
- Merged confidence normalization concepts into CASE-VERIFICATION-GUIDE.md

### Files Modified
- `docs/DATABASE_COVERAGE_ANALYSIS.md`: full rewrite with current audit data
- `docs/SAVE_FILE_GROUND_TRUTH.md`: fixed contradictions, updated timestamp
- `docs/CORROBORATION-SYSTEM.md`: fixed tile base, deduplicated formulas
- `docs/discovery-verification-cycle.md`: fixed tile base, deduplicated formulas
- `docs/EVENT-FLAG-GEOGRAPHY.md`: absorbed Flag-islands concept, fixed stale value
- `docs/CASE-VERIFICATION-GUIDE.md`: merged confidence normalization
- `docs/ARCHITECTURE.md`: updated methodology table
- `docs/COMMIT-PROTOCOL.md`: fixed IMPLEMENTATION_PLAN→BACKLOG reference
- `docs/DATA-SOURCES.md`: added decompiled game files section
- `docs/BACKLOG.md`: new file consolidating all planned work
- `CLAUDE.md`: updated doc table
- `.claude/commands/snapshot.md`: added BACKLOG.md tracking
- 9 files moved to `docs/archive/` with archive headers

---

## v0.13.1 - Utilities Section & Icon Font Reference

### Features
- Added Utilities top-level section with navigation breadcrumbs
- Added Icomoon icon font reference grid (96 Elden Map marker glyphs)
- Click-to-copy glyph properties, search/filter, hover tooltips with 48px preview
- Registered Icomoon font family for use in UI components

### Files Modified
- src/main.rs: font registration, Utilities route handling, IconsViewState
- src/ui/menu.rs: UtilitiesSelect/UtilitiesIcons routes, breadcrumbs, navigation
- src/ui/mod.rs: added utilities module
- src/ui/utilities/mod.rs: new module
- src/ui/utilities/icons_view.rs: new 96-glyph reference grid
- assets/fonts/icomoon.ttf: new icon font asset
- docs/CHANGELOG.md: v0.13.1
- Cargo.toml: bumped to 0.13.1

---

## v0.13.0 - Entity Relationships Pipeline & DRY Refactor

### Entity Relationships Upstream Migration
- Expanded boss database from 17 hardcoded entries to 205 extracted from GameAreaParam
- Added grace↔boss proximity computation with 200m threshold (188 bosses with nearby graces, 313 graces with nearby bosses)
- Migrated boss drops to structured JSON (`scripts/boss_drops.json`) with 53 boss drop groups
- Generated `entity_relationships_data.rs` with BOSS_DROPS, ITEM_DROPPED_BY, BOSS_DROP_INDEX, BOSS_NEARBY_GRACES, GRACE_NEARBY_BOSSES
- Boss detail panel now shows "Drops" and "Nearby Graces" sections
- Grace detail panel now shows "Nearby Bosses" section with distances
- Accurate boss type classification using SHARDBEARER_FLAGS set + rune tiers

### DRY Refactoring
- Added `mapgenie_section()` shared helper (was duplicated in bosses + graces views)
- Added `section_from_relationships()` generic filter→map→section builder
- Extracted `build_item_sections()` in items_view.rs (removed ~70 lines of copy-paste)
- Extracted `build_merchant_sections()` in merchants_view.rs
- Merged identical CSV/Markdown export arms in all 4 database views
- Removed 6 unused helper functions from relationship_list.rs

### Files Modified
- scripts/generate_db.py: GameAreaParam extraction, proximity computation, boss drops loading, relationship generation
- scripts/boss_drops.json: new structured boss drops data
- src/db/entity_relationships_data.rs: new generated relationships module
- src/db/entity_relationships.rs: refactored to use generated data
- src/db/bosses_data.rs: regenerated with 205 bosses and full metadata
- src/db/boss_drops.rs: deleted (replaced by generated data)
- src/db/mod.rs: updated module declarations
- src/ui/components/detail_panel/relationship_list.rs: new shared helpers, removed dead code
- src/ui/components/detail_panel/mod.rs: updated re-exports
- src/ui/database/bosses_view.rs: uses shared helpers, merged export arms
- src/ui/database/graces_view.rs: uses shared helpers, merged export arms
- src/ui/database/items_view.rs: extracted build_item_sections, merged export arms
- src/ui/database/merchants_view.rs: extracted build_merchant_sections, merged export arms
- Cargo.toml: bumped to 0.13.0

---

## v0.12.1 - Detail Panel Navigation & UI Polish

### Detail Panel Navigation
- External links (MapGenie) now open in default browser via `open` crate
- Merchant relationship links navigate to merchant detail view
- Item detail panel shows merchant cross-navigation with price info
- Added `NavigateToMerchant` and `OpenExternalUrl` detail panel actions

### UI Polish
- Replaced emoji lock icon with Phosphor `LOCK` icon on locked talisman slots
- Improved icon label layout: wider name area (100px), better text wrapping
- Removed monospace from stat values on Character General page
- Code formatting cleanup across general.rs and icons/mod.rs

### Files Modified
- src/ui/general.rs: formatting, lock icon, style tweaks
- src/ui/icons/mod.rs: icon label sizing, formatting
- src/ui/components/detail_panel/panel.rs: new action variants
- src/main.rs: handlers for NavigateToMerchant and OpenExternalUrl
- src/ui/database/bosses_view.rs: OpenExternalUrl for MapGenie links
- src/ui/database/graces_view.rs: OpenExternalUrl for MapGenie links
- src/ui/database/items_view.rs: NavigateToMerchant for merchant links
- src/ui/database/merchants_view.rs: item relationship navigation
- Cargo.toml: added `open` dependency

---

## v0.12.0 - Database Browser & Game Icons

### Game Icon System
- New icon loading module (`src/ui/icons/`) for displaying game item icons
- Icons loaded from extracted game files (160x160 PNG, displayed at 64x64)
- Equipment slots on Character General now show icons with names below
- Lazy-loaded texture caching with egui TextureHandle
- Graceful fallback to dark placeholder when icons unavailable

### Database Browser Enhancements
- Single-click now opens detail panel (was double-click)
- Table columns auto-width based on content
- Navigation breadcrumbs show entity names (e.g., "Graces > Table of Lost Grace")
- Quest chains view is now character-agnostic (pure reference data, no completion tracking)

### New Database Modules
- `src/db/bosses_data.rs`: Boss definitions with defeat flags
- `src/db/graces_data.rs`: Site of Grace database
- `src/db/merchants_data.rs`: Merchant locations and inventory
- `src/db/quest_chains.rs`: Quest progression steps with flag IDs
- `src/db/entity_relationships.rs`: Cross-entity relationship mapping
- `src/db/unified_items.rs`: Consolidated item database

### UI Components
- Detail panel system (`src/ui/components/detail_panel/`)
- Navigation breadcrumb component (`src/ui/components/navigation/`)
- Database views (`src/ui/database/`) for browsing game data
- Comparison view scaffolding (`src/ui/comparison/`)
- Validation view scaffolding (`src/ui/validation/`)

### Equipment ViewModel
- Added `icon_id: u16` field to `EquipmentItemViewModel`
- Icon IDs extracted from param data (EquipWeaponParam, EquipProtectorParam, etc.)

### Files Modified
- `src/ui/general.rs`: Equipment display with game icons
- `src/ui/icons/mod.rs`: New icon loading and caching system
- `src/vm/equipment.rs`: Added icon_id to equipment view model
- `src/ui/database/event_chains_view.rs`: Character-agnostic quest reference
- `src/main.rs`: Updated routing and view calls
- Multiple database and UI component files

---

## v0.11.1 - Warning Cleanup

### Compiler Warnings Fixed
- Fixed unreachable pattern in `vm/general.rs` - DLC region detection now correctly ordered
- Fixed overlapping range patterns in `pickup_flags.rs` and `event_flags_db.rs` for region detection
- Removed 97 unused imports via `cargo fix`
- Added `#![allow(dead_code)]` to research/development modules (discovery, verification, tokens)
- Moved workspace profile settings from subcrate to root `Cargo.toml`

### Files Modified
- `src/vm/general.rs`: Reordered DLC pattern before base game pattern
- `src/db/pickup_flags.rs`: Fixed overlapping region ranges
- `src/db/event_flags_db.rs`: Fixed overlapping region ranges
- `Cargo.toml`: Added workspace-level release profile
- `crates/wasm-event-flags/Cargo.toml`: Removed profile (moved to workspace root)
- Multiple modules: Added `#![allow(dead_code)]` for research tooling

---

## v0.11.0 - Character General Page Redesign

### Build Planner-Style Layout
- Redesigned Character > General page with 3-column layout inspired by build planners
- Column 1: Character Status (matching game's status screen)
  - Level, Runes Held
  - All 8 attributes (Vigor, Mind, Endurance, Strength, Dexterity, Intelligence, Faith, Arcane)
  - HP (current/max), FP (current/max), Stamina
  - Weapon Level, Total Runes
  - DLC Blessings (Scadutree, Spirit Ash) - shown only if > 0
  - Current Location (region name + map ID)
- Column 2: Equipment grid layout
  - Equipped Gear: 3-column grid (Right Hand | Armor | Left Hand)
  - Armaments: 4-column grid for arrows/bolts
  - Talismans: 4-column grid with lock icons for unavailable slots
- Column 3: Quick Items (10 slots) and Pouch (6 slots)

### New Data Fields
- `StatsViewModel`: Added hp, max_hp, fp, max_fp, stamina, max_stamina
- `GeneralViewModel`: Added map_id with MapID struct
- `MapID`: Parses 4-byte location, provides display_name() for region names

### Visual Design
- Dark card backgrounds (Color32::from_rgb(30, 30, 35))
- Grid cells expand to fill available container width
- Double-click to copy item names
- Right-click context menu on equipment slots

### Files Modified
- `src/vm/stats.rs`: Added HP, FP, Stamina fields
- `src/vm/general.rs`: Added MapID struct with region name mapping
- `src/ui/general.rs`: Complete rewrite with 3-column build planner layout
- `docs/CHANGELOG.md`: Added v0.11.0 entry
- `Cargo.toml`: Bumped to 0.11.0

---

## v0.10.0 - Unified Table Design for Event Flags and Inventory

### Event Flags UI Redesign
- Applied World Pickups design pattern (FilterBar + UnifiedTable + ExportToolbar) to all Event Flag subpages
- Created generic `simple_event_flag_view()` helper function to reduce code duplication
- Refactored 7 simple pages to use the generic helper:
  - Whetblades, Cookbooks, Maps, Bosses, Summoning Pools, Colosseums, Landmarks
- Sites of Grace: Flat table with region column, region dropdown filter, status chips
- Dungeon Pickups: Flat table with dungeon dropdown, type/status chips

### Inventory Browse Redesign
- Complete rewrite of Browse view using FilterBar + UnifiedTable + ExportToolbar
- Storage location filter dropdown (All/Equipped/Storage Box)
- Type filter chips for 6 item categories
- Default route changed from None to Browse
- Row colors: green for Equipped, gray for Storage Box

### New State Structs
- `SimpleEventFlagViewState` for generic event flag pages
- `GracesViewState` for Sites of Grace (has region filter)
- `BrowseViewState` for Inventory Browse
- `StorageLocation` enum (All, Equipped, StorageBox)

### Export Structs
- `SimpleEventFlagExportItem`, `GraceExportItem`, `DungeonPickupExportItem`
- `InventoryExportItem` for inventory browse export

### Files Modified
- `src/vm/events.rs`: Added view state structs, updated EventsViewModel
- `src/ui/events.rs`: Refactored 9 view functions, added generic helper
- `src/vm/inventory/mod.rs`: Added BrowseViewState, StorageLocation, default Browse route
- `src/ui/inventory/browse.rs`: Complete rewrite with new design pattern
- `docs/CHANGELOG.md`: Added v0.10.0 entry
- `Cargo.toml`: Bumped to 0.10.0

---

## v0.9.0 - Hierarchical Navigation Restructure

### Navigation Architecture
- **Two-path navigation system**
  - Path A (File): Home → PC|SteamId → CharName → Area → Subroute
  - Path B (Database): Home → Database → DatabaseName

- **New intermediate routes**
  - `CharacterSelect`: File loaded, shows character slots in submenu
  - `DatabaseSelect`: Database mode, shows database list in submenu

- **Clickable breadcrumb levels**
  - Each segment navigates to that hierarchy level
  - Platform/SteamID shows full save path on hover

### Landing Page
- **New home view with recent files**
  - Shows list of recently opened save files
  - Displays character names for each save
  - Persists to `~/.er-save-editor/config.json`
  - Supports drag-and-drop file opening

### Top Menu Bar
- **Simplified toolbar layout**
  - Left: Open button (with recent files dropdown), Database button
  - Right: Save (disabled/strikethrough), Export button

### Compact Footer
- **Icon-only status bar legend**
  - Shows Flag and Inv section labels
  - Icons with hover tooltips for detailed explanations
  - Reduced height from 28px to 24px

### Route Enum Restructure
- Renamed routes for clarity:
  - `General` → `CharacterGeneral`, etc.
  - `Spells` → `DatabaseSpells`, etc.
- Added `CharacterSelect` and `DatabaseSelect` routes
- Added `DatabaseDungeonPickups` route

### Files Modified
- `src/ui/menu.rs`: Route enum, breadcrumb_bar, navigation_buttons
- `src/main.rs`: Top menu, content routing, App struct with recent_files
- `src/ui/landing.rs`: New landing page module
- `src/ui/state/recent_files.rs`: Recent files persistence
- `src/ui/state/mod.rs`: Export recent_files module
- `src/ui/mod.rs`: Export landing module
- `src/ui/components/status_bar.rs`: Compact icon legend with hover tooltips
- `docs/CHANGELOG.md`: Added v0.9.0 entry
- `Cargo.toml`: Bumped to 0.9.0

---

## v0.8.4 - IBM Plex Fonts and UI Polish

### Typography
- **Added IBM Plex font family**
  - IBM Plex Sans: Default UI font (proportional)
  - IBM Plex Sans Condensed: Table/list headers (`font_condensed()`)
  - IBM Plex Mono: Monospace text (`.monospace()`)
  - IBM Plex Serif: Paragraph/description text (`font_serif()`)

### UI Polish
- **Replaced separator lines with spacer component**
  - Added `spacer(ui)` function in `style.rs` with `SECTION_SPACING = 8.0`
  - Removed gray horizontal lines from all views
  - Cleaner visual appearance

- **Breadcrumb caret icon**
  - Replaced ">" text with Phosphor CARET_RIGHT icon

### Files Modified
- `src/main.rs`: Font configuration with IBM Plex family
- `src/ui/style.rs`: Added `spacer()`, `font_condensed()`, `font_serif()`
- `src/ui/*.rs`: Replaced `ui.separator()` with `spacer(ui)` (14 files)
- `src/ui/menu.rs`: Breadcrumb uses CARET_RIGHT icon
- `assets/fonts/`: IBM Plex font files (Sans, Condensed, Mono, Serif)
- `docs/CHANGELOG.md`: Added v0.8.4 entry
- `Cargo.toml`: Bumped to 0.8.4

---

## v0.8.3 - Horizontal Navigation Layout

### UI Restructure
- **Replaced vertical sidebars with horizontal 3-row navigation**
  - Row 1: Toolbar with Open/Save buttons, platform info, Steam ID, Export button
  - Row 2: Clickable breadcrumb trail (Characters > CharacterName > Area > Subroute)
  - Row 3: Dynamic navigation buttons that change based on current level

- **Navigation hierarchy**
  - Level 1 (Root): Character buttons + Database view buttons
  - Level 2 (Character selected): Area buttons (General, Stats, Equipment, etc.)
  - Level 3 (Event Flags): Subroute buttons (Sites of Grace, Bosses, World Pickups, etc.)

- **Added display_name() methods** to Route and EventsRoute enums for breadcrumb display

### Removed
- Left sidebar for character list
- Left sidebar for slot sections menu
- Left sidebar for EventFlags subroute navigation

### Files Modified
- `src/ui/menu.rs`: Added breadcrumb_bar(), navigation_buttons(), helper functions, Route::display_name()
- `src/vm/events.rs`: Added EventsRoute::display_name()
- `src/main.rs`: Removed sidebars, added breadcrumb/navigation panels, updated toolbar layout
- `src/ui/events.rs`: Removed left sidebar, content renders directly into provided ui
- `src/ui/none.rs`: Updated empty state message
- `docs/CHANGELOG.md`: Added v0.8.3 entry
- `Cargo.toml`: Bumped to 0.8.3

---

## v0.8.2 - Special Override Detection in Event Flag Extraction

### Enhancements
- **Special override detection** for tile-based items with block-based getItemFlagId
  - Items like Whetstone Knife (tile row_id) use block flag 60130 instead of tile formula
  - Extraction scripts now detect when `getItemFlagId` returns a different flag type
  - Prevents incorrect flag ID assignment in generated database

- **Improved region parsing** in world pickups extraction
  - Better 10-digit tile ID vs 8-digit dungeon ID differentiation
  - Cleaner region classification logic

### Database Regeneration
- Regenerated `extracted_event_flags.json` (7086 flags)
- Regenerated `extracted_event_flags.md` with location data
- Regenerated `src/db/world_pickups.rs` database

### Files Modified
- `scripts/extract_event_flags.py`: Special override detection logic
- `scripts/extract_world_pickups_v2.py`: Improved region ID parsing
- `scripts/extracted_event_flags.json`: Regenerated database
- `scripts/extracted_event_flags.md`: Regenerated documentation
- `src/db/world_pickups.rs`: Regenerated Rust database
- `docs/CHANGELOG.md`: Added v0.8.2 entry
- `Cargo.toml`: Bumped to 0.8.2

---

## v0.8.1 - Context Metadata in Flag Details Export

### Enhancement
- **Added context metadata to Copy Details export**
  - `timestamp`: When the export was created
  - `save_file`: Full path to the loaded .sl2 file
  - `slot_index`: Character slot number (0-9)
  - `character_name`: Character's in-game name
  - `event_flags_size`: Size of event flags array (validates data loaded)

### Files Modified
- `src/main.rs`: Pass save_path to events view
- `src/ui/events.rs`: Include context metadata in Copy Details output
- `docs/CHANGELOG.md`: Added v0.8.1 entry
- `Cargo.toml`: Bumped to 0.8.1

---

## v0.8.0 - Flag Details Sidebar with Inventory Evidence

### Features
- **Flag Details sidebar panel** for World/Dungeon Pickups
  - Click any pickup row to select it and open details panel
  - Shows flag ID (decimal/hex), item name, collected status
  - Displays byte offset and bit position for debugging
  - "Copy Details" button exports comprehensive debug data

- **Inventory evidence matching** with fuzzy search
  - Searches both equipped inventory AND storage box (4 locations total)
  - Shows whether inventory evidence SUPPORTS or CHALLENGES flag status
  - Collapsible "Raw Data" section with ga_item_handle, inventory_index, storage location
  - Match scoring (exact=100%, contains=90%, word overlap=60%+)

- **World pickup row_id formula** for local_id >= 7000
  - Discovery: World pickups with getItemFlagId (local_id 7000+) use separate bitfield
  - Formula: `byte_offset = (row_id - 1037373320) / 8`
  - Verified via before/after save captures of Golden Rune pickups

### Bug Fixes
- **Reverse lookup returns all overlapping blocks** - Fixed to return ALL matching blocks when byte ranges overlap (blocks 71600 and 76000 overlap at [3250, 3323))
- **Widget ID collisions** - Fixed egui ID errors in inventory matches loop using `push_id`

### Technical Changes
- Added `get_storage_inventory()` method to SaveType
- Added `WORLD_PICKUP_ROW_ID_BASE` constant (1037373320)
- Added `calculate_world_pickup_offset_by_row_id()` function
- Updated `calculate_tile_flag_offset` to use row_id formula for local_id >= 7000
- Extended WASM crate with pickup flag calculations and tests
- Updated tests to reflect new formula expectations

### Documentation
- Added "False Negative Investigation Protocol" to CLAUDE.md
- Documented row_id tracking discovery in EVENT-FLAG-GEOGRAPHY.md

### Files Modified
- `src/ui/events.rs`: Flag details sidebar, inventory matching, Copy Details
- `src/vm/events.rs`: Added selected_flag_id to filter structs
- `src/db/pickup_flags.rs`: Row_id formula, updated tile/dungeon offset calculations
- `src/save/save.rs`: Added get_storage_inventory() method
- `src/main.rs`: Pass storage inventory to events view
- `src/discovery/reverse_lookup.rs`: Return all overlapping blocks
- `crates/wasm-event-flags/src/lib.rs`: Pickup flag calculations
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Row_id tracking documentation
- `CLAUDE.md`: False Negative Investigation Protocol
- `tests/regression_suite.rs`: Updated block base test
- `Cargo.toml`: bumped to 0.8.0

---

## v0.7.2 - Documentation: Per-Section Discovery

### Documentation
- Updated `docs/EVENT-FLAG-GEOGRAPHY.md` with per-section discovery findings
  - Added "Dungeon Pickup Bases (CRITICAL DISCOVERY)" section
  - Documented why linear formula was wrong
  - Added table of verified section bases (89 total)
  - Listed discovery scripts for future reference
  - Updated Legacy Dungeons table with verification status

### Files Modified
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Added per-section discovery documentation
- `docs/CHANGELOG.md`: Added v0.7.2 entry
- `Cargo.toml`: bumped to 0.7.2

---

## v0.7.1 - Per-Section Pickup Base Discovery

### Bug Fix
- **Fixed dungeon pickup detection for Catacombs, Caves, Tunnels**
  - Discovery: The linear formula `base + section * 1125` was WRONG
  - Each (area, section) combination has its own empirically-discovered base offset
  - Detection improved from ~25% to **100%** for all verified sections

### Key Finding
The linear section formula assumed contiguous memory allocation, but in reality:
- Catacombs sections use bases ranging from 1785 to 3827 (non-linear)
- Caves sections use bases ranging from 1786 to 31903 (wildly varying)
- Tunnels sections use bases ranging from 1788 to 28979 (scattered)

### Technical Changes
- Added `DUNGEON_PICKUP_SECTION_BASES` HashMap with 89 verified entries
- Each entry maps `(area, section)` → `base_offset`
- Formula: `offset = section_base + local_id/8` (no section multiplication)
- All 89 entries verified with 100% match rates across save files

### Verification Results
| Area | Before | After |
|------|--------|-------|
| Catacombs (30) | 27/106 (25%) | 106/106 (100%) |
| Caves (31) | 34/140 (24%) | 140/140 (100%) |
| Tunnels (32) | 23/56 (41%) | 56/56 (100%) |

### Scripts Added
- `scripts/verify_specific_pickups.py`: Check pickups against save data
- `scripts/discover_per_section_bases.py`: Brute-force base discovery
- `scripts/build_pickup_section_map.py`: Generate Rust HashMap code

### Files Modified
- `src/db/pickup_flags.rs`: Added DUNGEON_PICKUP_SECTION_BASES, updated calculation
- `Cargo.toml`: bumped to 0.7.1

---

## v0.7.0 - Complete Dungeon Pickup Database

### Features
- **Complete dungeon pickup database** (2,108 entries)
  - Covers all 23 dungeon area types: Stormveil Castle, Leyndell, Catacombs, Caves, Tunnels, Hero's Graves, etc.
  - Up from 1,950 entries to 2,108 (including 150 items without MSB position data)
  - Item names resolved from EquipParam files (weapons, armor, goods, talismans, ashes of war)
  - Categories: Golden Runes (359), Smithing Stones (293), Consumables (1,115), Weapons (145), Armor (112), Talismans (57)

- **New Dungeon Pickups UI view**
  - Filter by dungeon area, item category, and collection status
  - Shows collection progress per area (e.g., "Catacombs: 45/123 collected")
  - Search functionality for item names
  - Highlights items from unverified pickup bases

- **Dungeon pickup generation script** (`scripts/generate_dungeon_pickups.py`)
  - Combines extracted_event_flags.json with ItemLotParam_map for complete coverage
  - Cross-references EquipParam files for accurate item names
  - Category-aware name resolution (handles overlapping item IDs)
  - Outputs analysis report showing coverage by area

- **Grace reliability improvements**
  - GraceStatus enum with Discovered/NotDiscovered/Unreliable variants
  - Unreliable block detection shown in UI with warning
  - Count summaries exclude unreliable graces

### Technical Details
- Flag formula: `event_flag = row_id + 7000` for dungeon pickups
- 1,958 pickups have MSB position data, 150 are ItemLotParam-only
- Section size: 1,125 bytes per dungeon section
- All 17 pickup base offsets verified via temporal differential analysis

### Files Modified
- `src/db/dungeon_pickups.rs`: Regenerated with 2,108 entries
- `src/db/pickup_flags.rs`: Added DUNGEON_PICKUP_BASES map
- `src/ui/events.rs`: New dungeon_pickups() view, grace reliability display
- `src/vm/events.rs`: Added GraceStatus enum, DungeonPickups route
- `scripts/generate_dungeon_pickups.py`: New generation script
- `scripts/discover_all_dungeon_pickup_bases.py`: Discovery tool
- `scripts/verify_dungeon_pickup_bases.py`: Verification tool

---

## v0.6.0 - WASM Shared EventFlags Detection

### Features
- **Single source of truth for EventFlags detection**
  - New `wasm-event-flags` crate provides shared detection algorithm
  - Used by both ER-save-Editor (native Rust) and elden-map (via WASM)
  - Eliminates implementation drift between projects
  - Guarantees identical detection results

- **Improved detection algorithm**
  - Added negative validation flags (late-game graces that should NOT be set)
  - Prevents false positives from random data matching bit patterns
  - Fixed search start offset to 0x12000 (73,728 bytes)

- **Detection parameters in ground_truth_offsets.json**
  - Added `event_flags_detection` section with all validation flags
  - Documents positive validation (7 flags) and negative validation (6 flags)
  - Single source of truth for detection configuration

### Architecture
- `crates/wasm-event-flags/` - New Rust crate with detection algorithm
- `src/save/common/event_flags_detection.rs` - Delegates to shared crate
- Builds to WebAssembly for elden-map via `wasm-pack`

### Documentation
- Added `docs/WASM-EVENT-FLAGS.md` with full documentation
- Updated `CLAUDE.md` with WASM docs reference

### Files Modified
- `Cargo.toml`: Added workspace, wasm-event-flags dependency
- `crates/wasm-event-flags/`: New shared detection crate
- `src/save/common/event_flags_detection.rs`: Delegates to shared crate
- `ground_truth_offsets.json`: Added event_flags_detection section
- `docs/WASM-EVENT-FLAGS.md`: New documentation

---

## v0.5.4 - Item Pickup Auto-Completion & Late-Game Grace Fixes

### Features
- **Progression-gated validation for late-game graces (76400+)**
  - Level 10 characters no longer show Forbidden Lands (76500) as discovered
  - Graces require prerequisite boss defeats: Morgott for 76500-76700, Fire Giant for 76700+
  - Prevents false positives from uninitialized memory in late-game grace regions

- **Dungeon prerequisite validation for Stormveil Castle (71000)**
  - Calibration now checks if Margit (10000850) is defeated before calibrating Stormveil
  - Prevents false positives when player hasn't reached the castle
  - Lowered match threshold to 50% (3 of 6 graces) for Stormveil since it's required progression

- **Row ID conversion for world tile pickups**
  - Added `convert_to_row_id()` to convert getItemFlagId (localId 7000+) to row_id (localId 0-999)
  - The game stores row_id, not getItemFlagId - this enables 993 world pickups to be tracked
  - Added `is_tile_pickup_flag_set()` for calibrated tile pickup checking

### Technical Changes
- Added `PROGRESSION_GATES` constant with boss flag requirements per grace range
- Added `check_progression_gate()` to verify boss defeats before showing late-game graces
- Added `DUNGEON_PREREQUISITES` constant mapping dungeon blocks to required boss flags
- Added `LEGACY_DUNGEON_BLOCKS` with Stormveil grace anchors for calibration
- Added `calibrate_legacy_dungeon_block()` for independent legacy dungeon calibration

### Key Findings
- **Row ID Discovery (2026-01-23)**: For tile-based world pickups, ItemLotParam has `getItemFlagId = row_id + 7000`. The game stores `row_id` (storable), NOT `getItemFlagId` (unstorable). Example: flag 1044367310 (localId 7310) → stored as 1044360310 (localId 310).
- **Progression gates**: Late-game grace flags (76500+) can show false positives on early-game saves because the memory region may contain uninitialized/garbage data. Gating by boss defeats ensures the player has actually reached those areas.

### Files Modified
- `src/calibration.rs`: Added DUNGEON_PREREQUISITES, LEGACY_DUNGEON_BLOCKS, calibrate_legacy_dungeon_block()
- `src/db/pickup_flags.rs`: Added convert_to_row_id(), is_tile_pickup_flag_set(), test
- `src/vm/events.rs`: Added PROGRESSION_GATES, check_progression_gate()

---

## v0.5.3 - Dynamic Grace Block Calibration

### Features
- **Dynamic calibration for unreliable grace blocks**: Graces from blocks 71000, 71100, 71600 now use per-save calibration
  - Uses tutorial grace (Cave of Knowledge, flag 71800) as calibration anchor
  - Detects offset delta between ground truth and actual save layout
  - Validates calibration using multiple early-game graces (The First Step, Church of Elleh, etc.)
  - Confidence scoring: 0.90+ for high-quality matches, lower for uncertain calibration

- **Reliability filtering fallback**: When calibration fails, graces are marked `[?]` and excluded from counts
  - Prevents false positives where calibration cannot be determined
  - UI shows warning for unreliable graces with uncertain status

### Technical Changes
- Added `GraceBlockCalibration` struct with calibrated bases per block
- Added `CalibrationService::calibrate_grace_blocks()` for dynamic offset detection
- Added `CalibrationService::detect_offset_delta()` using tutorial grace anchor
- Added `CalibrationService::validate_delta()` for cross-validation
- Added `CalibrationService::get_grace_offset_calibrated()` for calibrated lookups
- Added `GraceStatus` enum with `Discovered`, `NotDiscovered`, `Unreliable` variants
- Added `is_block_reliable(flag_id)` function for static reliability checks
- Unreliable graces (failed calibration) are skipped when writing to save file

### Coverage Impact
- **Before**: 329/421 graces (78%) reliably detectable, 92 (22%) marked unreliable
- **After**: Up to 421/421 graces (100%) detectable when calibration succeeds
- Calibration success depends on save having tutorial graces discovered

### Files Modified
- `src/calibration.rs`: Added grace block calibration infrastructure
- `src/db/pickup_flags.rs`: Added `is_block_reliable()` function
- `src/vm/events.rs`: Use calibration for grace status detection
- `src/ui/events.rs`: Updated graces view to show reliability status
- `src/vm/vm.rs`: Skip unreliable graces when updating save
- `src/vm/slot.rs`: Handle GraceStatus in export

---

## v0.5.2 - Block 520000 Expansion & 67000 Investigation

### Database Expansion
- **Block 520000**: Added 6 new verified flags (5/5 inventory-differential match)
  - 520600: Rusted Anchor
  - 520610: Roar Medallion
  - 520620: Smithing-Stone Miner's Bell Bearing [1]
  - 520650: Somberstone Miner's Bell Bearing [2]
  - 520660: Dragon Heart
  - 520670: Somber Smithing Stone [6]
- Block 520000 now has **18 verified flags** (was 12)

### Data Corrections
- **Block 67000**: Marked `blocked` status (was `needs_investigation`)
  - BLOCK_ITEMS mappings are completely incorrect (e.g., says 67120="Missionary's Cookbook [1]" but game data says 67120="Nomadic Warrior's Cookbook [21]")
  - Actual world pickup flags: 67030, 67120, 67130, 67300, 67420, 67430, 67630, 67860, 67880, 67890, 67910
  - Need to rebuild flag-item mappings from game params before verification can proceed

### Files Modified
- `ground_truth_offsets.json`: Added 6 flags, updated block 67000 status

---

## v0.5.1 - Schema Pre-filtering & Block Investigation

### Features
- **Schema-based pre-filtering in batch verification** (`scripts/verification/case_cli.py`)
  - Added `--schema-filter` flag to automatically skip untrackable flags
  - Probes save file before verification loop to identify sparse allocation gaps
  - Reports skipped flags in "EVIDENCE GAPS" section
  - Prevents wasted effort investigating flags known to be in padding regions

### Bug Fixes / Data Corrections
- **Flagged incorrect block bases in ground_truth_offsets.json**
  - Block 62000: Marked `needs_investigation` - flag IDs in BLOCK_ITEMS (62010-62080) don't exist in game data; offset 9359 contains 8-byte record structure, not bit-packed flags
  - Block 67000: Marked `needs_investigation` - base offset 37411 incorrect; flags show unset even when items present in inventory
  - Block 68000: Marked `needs_investigation` - derived from incorrect 67000 base

### Key Findings
- **Block 62000**: BLOCK_ITEMS used assumed flag IDs that don't exist. Actual map fragment pickup flags are 10-digit tile-based (e.g., 1042370200). Block 62000 contains WorldMapPointParam flags for location discovery.
- **Block 67000/68000**: Flag IDs are valid but base offsets need re-discovery. Original verification likely used different save file.

### Files Modified
- `ground_truth_offsets.json`: Updated status for blocks 62000, 67000, 68000
- `scripts/verification/case_cli.py`: Added schema-filter integration, documented block issues

---

## v0.5.0 - Schema-Based Allocation Detection & Case Verification System

### Features
- **Schema-based flag allocation detection** (`scripts/verification/flag_schema.py`)
  - `BlockSchema`: Define known flag IDs and their expected byte offsets
  - `AllocationBitmap`: Probe save data to identify trackable vs untrackable flags
  - Detects **sparse allocation gaps** where the game doesn't allocate memory
  - CLI: `python flag_schema.py --block 520000 --base 1341 --save /path/to/save.sl2 --boundaries`

- **Case-based verification system** (`scripts/verification/case_manager.py`, `case_cli.py`)
  - Defense/Challenge methodology for rigorous flag verification
  - Evidence aggregation from inventory, differential, temporal sources
  - Formula update proposals when verification fails
  - Gap reporting for untrackable flags

- **Verified block 520000** (Spirit Ashes, Talismans)
  - Base offset: 1341
  - 46 flags trackable, 13 in sparse gaps
  - 12 flags exported to ground_truth with confidence 1.0

### Bug Fixes
- **Fixed anchor database access** in `case_manager.py`
  - `boss_defeat_chains`: Now correctly accesses nested `.get('chains', {})` structure
  - `geographic_regions`: Now correctly accesses nested `.get('regions', {})` structure

### Refactoring (DRY)
- **Centralized all formula constants** in `ground_truth_loader.py`
  - Removed hardcoded BLOCK_BASES from `extract_test_cases.py`, `case_cli.py`, `verify_boss_chain.py`
  - All verification scripts now use `get_block_base()`, `get_tile_config()`, etc.
  - Archived deprecated `flag_formulas.py` to `archive/` directory

### Documentation
- **docs/ARCHITECTURE.md**: Added flag_schema.py API reference
- **docs/EVENT-FLAG-GEOGRAPHY.md**: Added "Sparse Flag Allocation" section with terminology
- **docs/EVIDENCE-BASED-DISCOVERY.md**: Updated block 520000 findings with verified results
- **docs/CASE-BASED-VERIFICATION.md**: Added schema pre-filtering section

### Key Discovery: Sparse Flag Allocation
Block 520000 uses sparse memory allocation - not all flag IDs have storage:
```
520000-520059: ALLOCATED
520060-520089: SPARSE GAP (0xFF in all slots)
520090-520189: ALLOCATED
520190-520219: SPARSE GAP
...
```
Flags in sparse gaps (e.g., 520210, 520330, 520450) cannot be verified with the block formula.

### Files Modified
- `scripts/verification/flag_schema.py`: New schema/allocation bitmap system
- `scripts/verification/case_manager.py`: Bug fixes for anchor database
- `scripts/verification/case_cli.py`: DRY refactoring, gap reporting
- `scripts/verification/extract_test_cases.py`: DRY refactoring
- `scripts/verification/verify_boss_chain.py`: DRY refactoring
- `ground_truth_offsets.json`: Added block 520000, 12 verified flags, untrackable_flags
- `docs/*.md`: Documentation updates

---

## v0.4.31 - Tile Formula Base Offset Reversion

### Bug Fixes
- **Reverted tile formula base_offset from 489981 back to 485330**: The v0.4.28 "correction" was wrong
  - Re-verification showed offset 857482 had NO change during Smoldering Butterfly pickup
  - Actual observed change: offset **852831** bit 5 SET (0x00 → 0x20)
  - Calculation confirms: 485330 + 420*875 + 1 = 852831

### Enhancements
- **Added calibration_anchors section** to `ground_truth_offsets.json`
  - Tile anchor: Smoldering Butterfly (1043500010) at offset 852831, bit 5
  - Block anchors: The First Step (76100), Church of Elleh (76101), Cave of Knowledge (71800)
  - Enables runtime validation of formula correctness

### Files Modified
- `ground_truth_offsets.json`: Reverted tile base, added calibration_anchors
- `elden-map/server/src/eventFlagService.ts`: TILE_BASE_OFFSET=485330, TILE_COL_BASE=30
- `scripts/verification/flag_formulas.py`: TILE_CONFIG.base_offset=485330
- `src/db/pickup_flags.rs`: Updated comments and test assertions
- `scripts/capture_agent.py`: Updated TILE_BASE_OFFSET constant
- `scripts/verification/*.py`: Updated default fallback values
- `docs/SAVE_FILE_GROUND_TRUTH.md`: Corrected tile formula documentation

---

## v0.4.30 - Snapshot Capture Automation

### Features
- **Automated snapshot capture workflow**: New system for capturing save file snapshots with POI metadata
  - `scripts/capture_agent.py`: Standalone HTTP server (port 8765) for save file capture
  - Supports before/after pairing with auto-chaining for sequential captures
  - Generates indexed filenames with flag_id, map_tile, and phase
  - Updates `capture_catalog.json` with full metadata
  - CLI commands: `serve`, `capture`, `migrate`, `status`

- **Dynamic verification test runner**: New calibration-aware test selection
  - `scripts/verification/snapshot_test_runner.py`: Selects appropriate snapshot pairs for testing
  - Calibrates formula bases per-save (addresses save-dependent offset issue)
  - Filters tests by flag format, verification status, and confidence level

### Documentation
- **EVENT-FLAG-GEOGRAPHY.md**: Added "Save-Dependent Base Offsets" warning section
  - Documents that tile/dungeon formula bases vary per save file
  - Explains GaItems section size variability affecting EF section offset
  - Provides calibration anchors for each formula type

- **discovery-verification-cycle.md**: Added "Automated Snapshot Capture Workflow"
  - Documents complete user workflow from in-game to capture
  - Explains auto-chaining logic for sequential snapshots
  - Describes capture_catalog.json schema and usage

### Files Added
- `scripts/capture_agent.py`: HTTP capture agent with catalog management
- `scripts/verification/snapshot_test_runner.py`: Dynamic test selection and calibration

### Files Modified
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Save-dependent base offset documentation
- `docs/discovery-verification-cycle.md`: Automated capture workflow documentation

---

## v0.4.29 - World Pickup False Positive Fix

### Bug Fixes
- **Fixed false positives in World Pickups view**: Items no longer incorrectly show as collected
  - Root cause: Using `getItemFlagId` instead of lot_id (ROW ID) for tile-based world pickups
  - Game stores lot_id directly (local_id 0-999) not getItemFlagId (lot_id + 7000)
  - Updated `extract_pickup_data.py` to use correct flag IDs
  - Regenerated `pickup_data.rs` with corrected event flags

- **Formula-based offset calculation**: Migrated from stale EVENT_FLAGS lookup table to dynamic formulas
  - `events.rs`, `vm.rs`, `world_pickups_view.rs` now use `get_flag_offset()` from `pickup_flags.rs`
  - Added 100-flag granularity for block flags (e.g., 71600, 71800)
  - Returns None for dungeon pickups without verified base offsets (prevents false positives)

### Features
- **Unverified filter**: Added "Unverified" option to Status filters in World Pickups view
  - Shows only items where verification status is uncertain
  - Helps identify potentially inaccurate flag mappings

### Key Findings
- Tile-based pickups (10-digit flags ≥1B) use ROW ID as flag, not getItemFlagId
- getItemFlagId formula adds 7000 to lot_id, but local_id ≥7000 has no storage allocation

### Files Modified
- `src/db/pickup_flags.rs`: Added 100-flag block granularity, None for unverified dungeons
- `src/db/pickup_data.rs`: Regenerated with correct flag IDs
- `src/ui/events.rs`: Added Unverified filter, use `get_flag_offset()`
- `src/ui/world_pickups_view.rs`: Added Unverified filter
- `src/vm/events.rs`: Use `get_flag_offset()` for all flag lookups
- `src/vm/vm.rs`: Use `get_flag_offset()` for writing event flags
- `scripts/extract_pickup_data.py`: Use lot_id for tile-based pickups
- `docs/CHANGELOG.md`: Version 0.4.29
- `Cargo.toml`: Bumped to 0.4.29

---

## v0.4.28 - Flag Formula Discovery

### New Formulas
- **Block 61000**: Base 2671 - Map area visit tracking flags (108 flags)
  - Correlates 611xx to mXX dungeon map codes (e.g., 61100→m10 Stormveil, 61128→m18 Roundtable Hold)
  - Verified via multi-flag correlation on Slot 0 mid-game save

- **Midrange 510000**: Base 63750 - Remembrance consumption flags (64 flags)
  - Set when remembrance is USED at Enia, not when obtained
  - Derived from event_flags.rs hardcoded data

- **Midrange 710000**: Base 13875 - Roundtable Hold NPC progression flags (41 flags)
  - Tracks NPC state changes during game progression
  - Derived from event_flags.rs hardcoded data

### Coverage Improvement
- Formula coverage: 57.4% → 60.9% (+3.5%, +213 flags)
- Remembrance context now 100% covered (68/68 flags)

### Files Modified
- `ground_truth_offsets.json`: Added 61000, 510000, 710000 formulas
- `docs/CHANGELOG.md`: Version 0.4.28
- `Cargo.toml`: Bumped to 0.4.28

---

## v0.4.27 - Unified Flag Database

### Features
- **Unified Flag Database**: Merges three data sources into single queryable database
  - Flag Catalog: names, positions, regions, item associations
  - Param Database: source traceability (param file, row ID, field)
  - Event Graph: EMEVD triggers, dependencies, progression chains

- **New `param-extract` CLI command**: Extracts flags from regulation-bin XML params
  - Supports: ItemLotParam_map, BonfireWarpParam, WorldMapPointParam, ShopLineupParam, GameAreaParam, NpcParam
  - Outputs to param_flags.json for reuse

- **New `param-query` CLI command**: Query param flag database
  - `--stats`: Show summary statistics
  - `--blocks`: List midrange blocks with flags
  - `--bosses`: List boss defeat flags with names
  - `--param <name>`: Filter by param source

- **New `unified` CLI command**: Query unified flag database
  - `--build`: Build/rebuild from all sources
  - `--search <name>`: Search flags by name
  - `--needs-formula`: Flags in params but not EMEVD
  - `--high`: High-confidence flags (all 3 sources)
  - `--category`, `--context`: Filter queries

### Technical Details
- `SourceConfidence` enum: High/Medium/Low/Inferred based on source count
- Indexed lookups by category, param, trigger context, region
- JSON persistence for fast subsequent loads

### Files Modified
- `src/discovery/unified_db.rs`: New unified database implementation
- `src/discovery/param_flags.rs`: New param extraction module
- `src/discovery/mod.rs`: Module exports for new components
- `src/discovery/cli.rs`: CLI commands for unified and param modules
- `docs/CHANGELOG.md`: Version 0.4.27
- `Cargo.toml`: Bumped to 0.4.27

---

## v0.4.26 - Batch Validation Tool for EMEVD-Backed Flags

### Features
- **New `batch-validate` CLI command**: Validates all EMEVD-backed flags against save data
  - Reports formula coverage, set/unset status, and verification levels
  - Breaks down by trigger context and flag block
  - Identifies blocks needing formula coverage

### Command Options
- `--block <id>`: Filter to specific 1000-flag block (e.g., `--block 9000`)
- `--context <name>`: Filter by trigger context (e.g., `--context boss_defeat`)
- `--set` / `--unset`: Show only set or unset flags
- `--invalid`: Show only flags without offset formulas

### Key Findings
- Block 9000 (remembrance flags 91xx) confirmed using simple formula
- 6,161 flags with EMEVD triggers, 3,537 (57.4%) have formulas
- Identified coverage gaps: blocks 510000, 710000, 61000

### Files Modified
- `src/discovery/cli.rs`: Added cmd_batch_validate function and stats structs
- `src/discovery/event_graph.rs`: Added get_all_flag_ids() method
- `docs/CHANGELOG.md`: Version 0.4.26
- `Cargo.toml`: Bumped to 0.4.26

---

## v0.4.25 - Midrange Flag Formula Support (Sorceries/Incantations)

### Features
- **New midrange formula**: Support for 6-digit flags (100000-999999)
  - Covers sorcery, incantation, and ash of war unlock flags
  - Formula: `byte_offset = base + (flag_id - block_start) / 8`
  - Block 540000 verified with 129/129 flags matching

### Technical Details
- Added `VERIFIED_MIDRANGE_BASES` to ground_truth_offsets.json
- Added `calculate_midrange_flag_offset()` to pickup_flags.rs
- Build system generates midrange bases from JSON at compile time
- Supports both 1000-flag and 10000-flag block granularity

### Verification
- All 129 sorcery/incantation flags (540100-540652) verified against event_flags.rs hardcoded data

### Files Modified
- `build.rs`: Generate VERIFIED_MIDRANGE_BASES and MidrangeBase struct
- `ground_truth_offsets.json`: Added midrange_formula section
- `src/db/pickup_flags.rs`: Added midrange flag calculation
- `docs/CHANGELOG.md`: Version 0.4.25
- `Cargo.toml`: Bumped to 0.4.25

---

## v0.4.24 - EventGraph Integration into Verification Chain

### Features
- **Corroboration engine integration**: EventGraph now provides EMEVD evidence during flag validation
  - Adds +1 to agreement count when flag has SetEventFlagID trigger
  - Adds +0.1 confidence boost for flags found in EMEVD
  - Reports trigger context, source files, and progression chains

- **New CLI command** `discovery event-graph`:
  - `<flag_id>` - Query specific flag for triggers, dependencies, entity mappings
  - `--stats` - Show event graph statistics (6,161 flags, 13,612 triggers)
  - `--contexts` - List all trigger contexts with counts
  - `--chains` - Show remembrance and map fragment progression chains

- **Enhanced corroborate command**:
  - Automatically loads event graph when available
  - Shows EMEVD validation in output (trigger count, context, sources)
  - Falls back gracefully if event graph unavailable

### Integration Points
```rust
// Load corroboration engine with EMEVD validation
let engine = CorroborationEngine::load_with_event_graph()?;

// Result now includes event graph evidence
result.event_graph.has_trigger      // Flag exists in EMEVD
result.event_graph.trigger_context  // "boss_defeat", "grace_discovery", etc.
result.event_graph.confidence_boost // +0.1 when found
```

### Files Modified
- `src/discovery/corroboration.rs`: Added EventGraphValidation, integration methods
- `src/discovery/cli.rs`: Added event-graph command, enhanced corroborate output
- `src/discovery/mod.rs`: Added EventGraphValidation export
- `docs/CHANGELOG.md`: Version 0.4.24
- `Cargo.toml`: Bumped to 0.4.24

---

## v0.4.23 - EMEVD Event Graph Extraction System

### Features
- **New extraction system**: Parses all 587 EMEVD files to build queryable event graph
- **Python extraction script** (`scripts/extract_event_graph.py`):
  - Parses `common_func.emevd.js` for event templates (183 templates)
  - Parses `common.emevd.js` for known progression chains
  - Processes all map EMEVD files for flag triggers and dependencies
  - Outputs structured JSON for Rust consumption

- **Rust loader module** (`src/discovery/event_graph.rs`):
  - O(1) flag trigger lookup via HashMap indexes
  - Dependency graph traversal methods
  - Entity-to-flag mapping queries
  - Progression chain lookup (remembrances, map fragments)
  - Validation evidence API for formula verification

### Extraction Results
- **6,161 unique flags** extracted with trigger information
- **13,612 total triggers** (SetEventFlagID calls)
- **1,932 dependency relationships** (EventFlag conditions)
- **378 entity mappings** (boss/grace entities to flags)
- **92 progression chains** (remembrances, map fragments)

### Key Methods
```rust
// Validate flag existence via SetEventFlagID evidence
graph.has_trigger(flag_id) -> bool

// Get trigger context (boss_defeat, grace_discovery, etc.)
graph.get_trigger_context(flag_id) -> Option<&str>

// Find remembrance chain by boss defeat flag
graph.find_remembrance_chain(9100) -> Option<&ProgressionChain>
```

### Files Created
- `scripts/extract_event_graph.py`: Python extraction (~400 lines)
- `scripts/event_graph.json`: Generated graph data (6.1 MB)
- `src/discovery/event_graph.rs`: Rust loader module (~460 lines)

### Files Modified
- `src/discovery/mod.rs`: Added event_graph module export
- `Cargo.toml`: Bumped to 0.4.23

---

## v0.4.22 - Documentation Restructuring & Verification Framework DRY Refactor

### Documentation Restructuring
- **CLAUDE.md reduced 86%**: From 299 to 41 lines by removing duplicated content
  - Kept: Commit protocol, knowledge resources, third-party warnings, slot descriptions
  - Added: Technical documentation reference table pointing to dedicated docs
  - Removed: All technical details already documented in docs/*.md

- **New `docs/ARCHITECTURE.md`**: Persistent architecture reference (237 lines)
  - Single source of truth hierarchy diagram
  - Module structure and import patterns
  - Script migration checklist and examples
  - Key principles for avoiding duplication

- **Updated `docs/discovery-verification-cycle.md`**:
  - Added Phase 6: Corroboration Validation (dual-formula + inseparable evidence)
  - Added Industry Best Practices section
  - Added cross-references to related documentation

### Verification Framework DRY Refactor
- **New `scripts/verification/constants.py`**: Save file structure constants only
  - SLOT_0_OFFSET, SLOT_SIZE, EVENT_FLAGS_SIZE, etc.
  - Clear docstring: validation flags and block bases come from ground_truth_loader

- **New `scripts/verification/utils.py`**: Shared utility functions (449 lines)
  - `read_slot_data()`, `detect_event_flags_start()`, `extract_event_flags()`
  - `check_flag()` with automatic formula selection
  - `is_0xff_padding()`, `multi_slot_differential()` for verification
  - Uses ground_truth_loader for all offset calculations

- **Updated `scripts/verification/__init__.py`**: Version 2.0.0
  - Exports all new modules
  - Documents architecture in module docstring
  - Maintains backward compatibility with legacy modules

- **New `scripts/verification/archive/`**: Directory for superseded scripts
  - README explaining archival criteria

- **Migrated `verify_tile_formula.py`**: Example migration to shared modules

### Architecture Principles Established
- `ground_truth_offsets.json` is the single source of truth for all offsets
- `ground_truth_loader.py` provides Python API to access ground_truth
- `constants.py` contains ONLY save file structure (not verification data)
- `utils.py` combines both into unified API for verification scripts

### Files Modified
- `CLAUDE.md`: Reduced to 41 lines with docs reference table
- `docs/ARCHITECTURE.md`: New - system architecture documentation
- `docs/discovery-verification-cycle.md`: Added Phase 6 and best practices
- `scripts/verification/constants.py`: New - save file structure constants
- `scripts/verification/utils.py`: New - shared utility functions
- `scripts/verification/__init__.py`: Version 2.0.0 with new exports
- `scripts/verification/archive/README.md`: New - archive directory docs
- `scripts/verification/verify_tile_formula.py`: Migrated to shared modules

---

## v0.4.21 - Fix Block 71000 Stormveil Grace Offsets

### Database Fix
- **Block 71000 (Stormveil Graces)**: Corrected base offset from 2673 to 9315
  - Previous base showed only 3/9 graces, new base shows 8/9 graces
  - Flag 71008 (Stormveil Main Gate) now correctly detected as SET
  - Verified via full search across bases 0-15000 with differential slot analysis

### Key Finding
- Grace blocks are NOT contiguous in memory:
  - Block 71000 (Stormveil) at base 9315
  - Block 71800 (Tutorial) at base 2725
  - These are stored ~6590 bytes apart despite sequential flag IDs

### Files Modified
- `ground_truth_offsets.json`: Updated block 71000 base_offset and all 71000-71008 flag entries
- `docs/SAVE_FILE_GROUND_TRUTH.md`: Updated block table and key findings
- `docs/CHANGELOG.md`: Added version entry

---

## v0.4.20 - UI Improvements and Verification Updates

### UI Improvements
- **Category filter overflow**: Fixed verification page category filters to wrap instead of overflow (changed to `horizontal_wrapped`)
- **Smaller monospace fonts**: Reduced table monospace font size from 12px to 9px (75% reduction) for better density
- **Consolidated styling**: Created `src/ui/style.rs` with shared `TABLE_MONO_SIZE` constant used across 10 view files
- **File dialog memory**: Open/save dialogs now remember the last used directory

### Verification Framework Updates
- Updated Rust code to use renamed correlation file (`flag-correlation-candidates.jsonl`)
- Updated field names in `VerificationRecord`:
  - `manual_status` → `user_marked_complete` (with serde alias for compatibility)
  - `auto_status` → `webapp_parsed_status`
  - `matches` → `statuses_align`

### Files Modified
- `src/ui/style.rs`: New shared style constants module
- `src/ui/verification_view.rs`: Category filter wrapping, style imports
- `src/ui/events.rs`, `src/ui/event_flags_db_view.rs`, `src/ui/world_pickups_view.rs`: Monospace size
- `src/ui/equipment.rs`, `src/ui/general.rs`, `src/ui/stats.rs`: Monospace size
- `src/ui/npcs_view.rs`, `src/ui/spells_view.rs`, `src/ui/shop_items_view.rs`: Monospace size
- `src/main.rs`: File dialog directory memory
- `src/util/verification_records.rs`: Field name updates
- `src/vm/verification_vm.rs`, `src/vm/slot.rs`: Field references
- `src/discovery/ground_truth_probe.rs`, `src/discovery/cli.rs`, `src/discovery/test_cases.rs`: Field names

---

## v0.4.19 - Major Block Base Corrections

### Critical Fixes
Three block bases were found to be completely incorrect (0% match against actual save data):

| Block | Category | Old Base | New Base | Evidence |
|-------|----------|----------|----------|----------|
| 62000 | Map Fragments | 1500 | **9359** | 12/12 match + negative validation |
| 65000 | Crystal Tears | 1875 | **37412** | 15/15 match + negative validation |
| 67000 | Cookbooks | 2280 | **37411** | 34/34 match + negative validation |

### Methodology: Multi-Slot Validation
- **Positive evidence**: Slot 0 (mid-game Confessor) - all confirmed items show as SET
- **Negative evidence**: Slot 1 (early-game Wretch) - all items show as UNSET
- Both conditions required for verification

### Key Finding
The old bases (1500-2280) were in the typical block range but gave 0% match.
The correct bases (9359-37412) are in higher ranges, suggesting these item categories
use a different storage region than grace/progression flags.

### New Verification Scripts
- `probe_wide_search.py`: Search entire event_flags section for bases
- `probe_maps_with_negatives.py`: Validate with positive AND negative evidence
- `probe_items_with_negatives.py`: Multi-slot validation for items
- `compare_bases.py`: Compare old vs new bases side-by-side
- `validate_map_fragments.py`: Inseparable evidence validation for maps
- `verify_map_base_multi_slot.py`: Cross-character validation

### Files Modified
- `ground_truth_offsets.json`: Corrected bases for blocks 62000, 65000, 67000, 68000
- `Cargo.lock`: Updated from build
- Added 7 new verification scripts

---

## v0.4.18 - Correlation Schema Updates

### Schema Alignment
- Updated all verification scripts to use renamed file `flag-correlation-candidates.jsonl`
  - Previously named `verification-records.jsonl`
  - Better reflects the file's purpose as correlation candidates, not verified records

### Field Name Updates
All scripts updated to use new field names from elden-map webapp:
- `manualStatus` → `userMarkedComplete` (user manually marked flag as complete)
- `autoStatus` → `webappParsedStatus` (webapp's formula detection result)
- `matches` → `statusesAlign` (whether user and webapp agree)

### Documentation Fixes
- Fixed VM grace base in VERIFICATION-LEADS.md (2726 → 2825)
- Fixed inconsistent Area 16 status in CORROBORATION-SYSTEM.md (was "verified", now "disproven")

### Files Modified
- `scripts/run_verification.py`: Updated paths and help text
- `scripts/verify_from_jsonl.py`: Updated paths and field references
- `scripts/discover_block_bases.py`: Updated path and field references
- `scripts/verification/*.py`: All scripts updated with new schema
- `docs/VERIFICATION-LEADS.md`: Fixed filename and base offset references
- `docs/CORROBORATION-SYSTEM.md`: Fixed inconsistent Area 16 status

---

## v0.4.17 - Volcano Manor Grace Sub-Block Discovery

### Critical Fix
- **Block 71600 discovered**: Volcano Manor graces use different base than tutorial graces
  - Flag 71607 (Subterranean Inquisition Chamber) empirically at byte 2825, bit 0
  - Sub-block 71600-71699 uses base 2825 (corrected from initial 2750 discovery)
  - User confirmed grace SET, but formula returned NOT SET - probing found correct location
  - Block 71000 has **discontinuous allocation** - different sub-ranges use different bases

### Technical Improvement
- **Sub-block support added** to `calculate_block_flag_offset()`
  - Now checks 100-flag granularity first (e.g., 71600)
  - Falls back to 1000-flag granularity if no sub-block found (e.g., 71000)
  - Enables future sub-block discoveries without code changes

### New Verification Scripts
- `scripts/verification/verify_grace_blocks.py`: Cross-validate grace blocks
- `scripts/verification/probe_vm_graces_extended.py`: Probe VM grace locations
- `scripts/verification/probe_grace_71607.py`: Find correct 71607 offset

### Files Modified
- `ground_truth_offsets.json`: Added 71600 sub-block, marked 71000 as partial
- `build.rs`: Added sub-block handling to code generator
- `docs/CHANGELOG.md`: v0.4.17

---

## v0.4.16 - Inseparable Evidence Methodology & Area 16 Disproven

### Critical Fix
- **Area 16 (Volcano Manor) base disproven**: Base 36737 (slot 29) reads unrelated data
  - Inseparable evidence test: 16000800 (Rykard defeat) showed SET, but grace 71600 showed NOT SET
  - User confirmed character has not defeated Rykard
  - Byte at 36837 (0xFF) is unrelated data, not Rykard defeat flag
  - Area 16 marked as "disproven" with base_offset = 0

### New Methodology: Inseparable Evidence
- **Inseparable flags**: Flags that cannot be set individually in normal gameplay
- **Boss-grace pairs**: Boss defeat flag + post-boss grace must be consistent
- Cross-validation catches false positives from formulas reading wrong data
- Documented in `docs/CORROBORATION-SYSTEM.md`

### Documentation
- **Boss Remembrance System**: Complete mapping of boss defeat → remembrance → pickup chains
  - Event 1100 awards progression items (Talisman Pouch), NOT remembrances
  - 91xx flags trigger Event 1100 on boss death
  - 510xxx flags track remembrance pickups
- **Inseparable Evidence Methodology**: Validation technique for dungeon base verification

### New Verification Scripts
- `scripts/verification/verify_boss_chain.py`: Validates boss defeat → remembrance pickup chains
- `scripts/verification/verify_rykard_chain.py`: Rykard-specific chain verification

### Files Modified
- `ground_truth_offsets.json`: Area 16 marked as disproven
- `docs/CORROBORATION-SYSTEM.md`: Added inseparable evidence methodology
- `Cargo.toml`: Bumped to 0.4.16

---

## v0.4.15 - Tile Formula Correction & Legacy Dungeon Base Discovery

### Critical Fix
- **Tile formula base_offset corrected**: Changed from 485330 to **489981** (+4651 bytes)
  - Verified empirically via Smoldering Butterfly pickup temporal diff
  - Flag 1043500010 confirmed at byte 857482 in event_flags section
  - This fixes all tile flag calculations for base game world pickups

### Database Expansion
- **Legacy dungeon bases discovered** using `legacymap.eventflagalloclist` slot formula:
  - Formula verified: `base = 4112 + slot × 1125` matches Areas 14 (29987) and 18 (43487) exactly
  - Area 11 (Leyndell): 8612 (slot 4)
  - Area 12 (Underground): 15362 (slot 10)
  - Area 13 (Leyndell Royal Capital): 26612 (slot 20)
  - Area 15 (Miquella's Haligtree): 33362 (slot 26)
  - Area 16 (Volcano Manor): 36737 (slot 29)
  - Area 19 (Chapel of Anticipation): 46862 (slot 38)
  - Area 34 (Divine Towers): 60362 (derived from section 10 at slot 60)
  - Area 35 (Mohgwyn Palace): 50237 (slot 41)
  - Area 39 (Elden Throne): 31112 (derived from section 20 at slot 44)

### Test Case Expansion
- Added 38 confirmed test cases from verification-records.jsonl (Slot 0, Confessor)
  - 34 block flags (graces, cookbooks, progression)
  - 4 dungeon flags (Stormveil bosses and pickups)

### New Verification Scripts
- `scripts/verification/verify_tile_formula.py`: Proper tile formula verification with slot/event_flags extraction
- `scripts/verification/extract_test_cases.py`: Extracts confirmed test cases from JSONL verification data

### Key Finding
- Web app (elden-map) uses different formula constants than our Rust project
- `computedByteOffset` values in verification-records.jsonl cannot be used directly
- `matches` field is still valuable for confirming flag states

### Files Modified
- `ground_truth_offsets.json`: Updated tile formula and all dungeon bases
- `src/db/pickup_flags.rs`: Updated test assertions for corrected base
- `src/discovery/offset_probe.rs`: Updated hardcoded tile base
- `src/discovery/test_cases.rs`: Added 38 confirmed test cases
- `scripts/verification/flag_formulas.py`: Synced tile base constant
- `docs/SAVE_FILE_GROUND_TRUTH.md`: Updated tile formula documentation
- `Cargo.toml`: Bumped to 0.4.15

---

## v0.4.14 - Area 14 = Tutorial Areas Discovery

### Key Discovery
- **Area 14 is Tutorial Areas, NOT Shunning-Grounds**
  - Chapel of Anticipation, Cave of Knowledge, and Stranded Graveyard all write to Area 14 (offset 29987)
  - Verified from 6,722 unique flags across Slot 6 Chapel and Slot 1 Cave empirical data
  - Areas 19/20 offsets from code appear unused for tutorial events

### Bug Fixes
- **Fixed reverse lookup priority**: Block flags now checked BEFORE simple flags
  - Prevents misidentification of flags in 2500-3500 byte range
  - Example: byte 2625 correctly identified as block 71000, not simple flag 21000

### Features
- **Dynamic slot mapping**: snapshot_batch.rs now handles "Slot X" pattern dynamically
  - Added "wr1" => 6, "sam" => 5 character mappings

### Documentation
- **Block overlaps documented**: Flag-islands.md now explains non-contiguous storage
  - Blocks 60000, 71000, 72000, 73000 have overlapping byte ranges
  - Not a bug - reflects FromSoft's flag allocation strategy
- **EVENT-FLAG-GEOGRAPHY.md**: Corrected Area 14 from "Shunning-Grounds" to "Tutorial Areas"

### Files Modified
- `ground_truth_offsets.json`: Updated Area 14, 19, 20 with corrected notes
- `src/discovery/reverse_lookup.rs`: Fixed block flag priority
- `src/discovery/flag_catalog.rs`: Changed Area 14 label to "Tutorial Event"
- `src/discovery/snapshot_batch.rs`: Added dynamic slot mapping
- `docs/Flag-islands.md`: Added block overlap documentation
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Corrected Area 14 documentation
- `Cargo.toml`: Bumped to 0.4.14

---

## v0.4.13 - Area 19/20 Formula Investigation

### Features
- **Area 19 (Elden Throne)**: Added formula base_offset=1426125 (derived from event_flags.rs)
  - Corrected: Area 19 is NOT Chapel of Anticipation - it's the final boss area
  - Contains Radagon/Elden Beast defeat flag (19000810)
  - Status: needs_review (no empirical verification yet)
- **Area 20 (Stranded Graveyard)**: Added formula base_offset=2500000 (derived from event_flags.rs)
  - Tutorial dungeon events (20007xxx flags)
  - Status: needs_review

### Key Findings
- **Chapel of Anticipation** shares Area 10 (Stormveil Castle) flags, NOT Area 19
  - Grafted Scion boss uses flag 10010800
- Tutorial grace flags (71800, 71801) use Block 71000, not dungeon areas

### Documentation Updates
- **EVENT-FLAG-GEOGRAPHY.md**: Updated Special Areas table with correct names
- **flag_catalog.rs**: Changed "Chapel Event" → "Elden Throne Event" with clarifying comment

### Data Collection Issues Identified
- Stranded Graveyard save snapshots (Wretch 11-12) are identical - snapshot wasn't captured correctly
- Grace flag 71800 not captured due to snapshot pairing limitations

### Files Modified
- `ground_truth_offsets.json`: Added Areas 19, 20 with needs_review status
- `src/discovery/flag_catalog.rs`: Fixed Area 19 UI label
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Updated Special Areas table
- `Cargo.toml`: Bumped to 0.4.13

---

## v0.4.12 - Dungeon Area Formula Verification

### Features
- **Area 14 (Shunning-Grounds)**: Verified base_offset=29987 with 1968/1968 flags matching (100%)
- **Area 18 (Roundtable Hold)**: Verified base_offset=43487 with 176/176 flags matching (100%)
- **Area 11 (Raya Lucaria)**: Identified base_offset=4112 (same as Stormveil), 172/187 match (92%), marked needs_review

### Documentation Updates
- **EVENT-FLAG-GEOGRAPHY.md**: Major restructure
  - Fixed terminology: Legacy Dungeons vs Minor Dungeons vs Special Areas
  - Corrected flag format from `XXYYYZZZZ` to `AASSZZZZ`
  - Fixed Area 18 = Roundtable Hold (was incorrectly documented as Area 19)
  - Added verification status for all dungeon areas
  - Added Flag Format Summary table
  - Reorganized World Hierarchy diagram

### Dungeon Area Name Corrections
| Area | Old Name | Correct Name |
|------|----------|--------------|
| 11 | Leyndell | Academy of Raya Lucaria |
| 13 | Farum Azula | Leyndell, Royal Capital |
| 14 | Raya Lucaria | Shunning-Grounds (Sewers) |
| 15 | Caria Manor | Miquella's Haligtree |
| 16 | Volcano Manor | Crumbling Farum Azula |

### Tests Added
- `test_verified_dungeon_shunning_grounds()`: Area 14 formula validation
- `test_verified_dungeon_roundtable()`: Area 18 formula validation

### Files Modified
- `ground_truth_offsets.json`: Updated Areas 11, 13, 14, 15, 16, 18 with correct names and offsets
- `src/db/pickup_flags.rs`: Added 2 new dungeon verification tests
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Restructured hierarchy and terminology
- `Cargo.toml`: Bumped to 0.4.12

---

## v0.4.11 - Tile Formula Base Offset Correction

### Fixes
- **Tile formula base offset**: Corrected from 337359 to 485330
  - Analysis of 69 empirical flags from discoveries.json showed consistent +147971 byte offset difference
  - Confirmed via flag 1041740610 (byte_offset=803906 matches formula exactly)
  - All 69 tile flags now calculate correct offsets

### Technical Details
- **Root cause**: Previous base offset (337359) was incorrectly derived
- **Verification method**: Cross-referenced all tile flags in discoveries.json against calculated offsets
- **Result**: 100% match rate after correction

### Tests Added
- `test_tile_confirmed_empirical()`: Validates empirically confirmed flag 1041740610
- Updated `test_tile_flag_formula_verified()` with corrected expected values

### Files Modified
- `ground_truth_offsets.json`: Updated tile_formula.base_offset to 485330, added proven tile flag
- `src/db/pickup_flags.rs`: Updated comment, fixed test values, added new test
- `src/discovery/offset_probe.rs`: Updated tile_base constant from 495830 to 485330
- `src/generated/ground_truth.rs`: Auto-regenerated with correct value
- `Cargo.toml`: Bumped to 0.4.11

---

## v0.4.10 - UI Unverified Indicator Fix

### Fixes
- **Unverified indicator position**: Moved "!" indicator from end of row to directly after status brackets
  - Before: `[X] | Grace Name | Region | 76100!`
  - After: `[X]! | Grace Name | Region | 76100`
- **Import cleanup**: Removed unused `ScrollArea` and `VerificationStatus` imports from events.rs

### Files Modified
- `src/ui/events.rs`: Updated `display_event_row()` to insert "!" after brackets
- `Cargo.toml`: Bumped to 0.4.10

---

## v0.4.9 - Block and Dungeon Formula Verification

### Features
- **Block Formula Verification**: Verified 5 previously unverified block bases
  - Block 65000 (Whetblades): Verified via hardcoded offsets (65610=0x79f, 65700=0x7aa, 65720=0x7ad)
  - Block 72000 (DLC Enir-Ilim graces): 10+ consistent proven flags
  - Block 74000 (DLC dungeon graces): 8+ consistent proven flags
  - Block 75000: Marked as "calculated" (no known flags in range)
  - Block 78000 (Grace guidance): 8+ proven flags (78210=3526, 78304=3538, etc.)

- **Dungeon Formula Verification**: Verified Area 30 (Catacombs)
  - Corrected from "needs_review" to "verified" status
  - 7 boss defeat flags matched formula (30020800=29761, 30030800=30886, etc.)
  - Confirmed base_offset=27411, section_size=1125

- **Verification Tests**: Added 6 new tests
  - `test_block_65000_whetblades_verified`
  - `test_block_72000_dlc_graces_verified`
  - `test_block_74000_dlc_dungeon_graces_verified`
  - `test_block_78000_grace_guidance_verified`
  - Updated `test_verified_dungeon_catacombs` with proven boss flags

### Verification Status Summary
| Formula Type | Verified | Calculated | Unverified |
|--------------|----------|------------|------------|
| Block bases  | 10       | 3          | 0          |
| Dungeon areas| 4        | 0          | 11         |
| Tile formula | 1        | 0          | 0          |

### Files Modified
- `ground_truth_offsets.json`: Updated status for 6 blocks/areas
- `src/db/pickup_flags.rs`: Added 6 verification tests
- `Cargo.toml`: Bumped to 0.4.9

---

## v0.4.8 - Enhanced Corroboration with Chain Validation

### Features
- **Chain Data Module** (`src/discovery/chain_data.rs`):
  - Boss defeat chains: 10 major bosses with defeat→remembrance→great rune→activation flag sequences
  - Area prerequisites: 6 late-game areas (Consecrated Snowfield, Haligtree, Leyndell, Farum Azula, etc.)
  - Geographic regions: 17 regions with landmark ranges, tile coordinates, grace ranges, map fragments
  - Scroll unlocks: 10 scroll/prayerbook→spell unlock chains
  - Verified block bases: 10 block base offsets for cross-validation

- **New RelationshipTypes**:
  - `BossDefeatChain`: Validates boss defeat → remembrance → great rune → activation consistency
  - `AreaPrerequisite`: Validates late-game flags have required prerequisites
  - `GeographicProximity`: Soft correlation for flags in same region
  - `ScrollUnlock`: Scroll pickup enables spell availability

- **Enhanced Corroboration Engine**:
  - `check_boss_chain()`: Detects contradictions like "Remembrance set but boss not defeated"
  - `check_area_prerequisite()`: Detects "Haligtree flag set without medallion halves"
  - `check_geographic_correlation()`: Regional flag correlation analysis
  - New result types: `BossChainResult`, `AreaPrerequisiteResult`, `GeographicCorrelationResult`

### Chain Validation Examples
| Chain Type | Validation | Contradiction Detection |
|------------|------------|------------------------|
| Boss | Godrick defeat (171) → Remembrance (9101) → Great Rune (160) → Activation (180) | Activation without possession |
| Area | Medallion halves (60430, 60431) → Consecrated Snowfield (62550+) | Late-game flags without prereqs |
| Geographic | Limgrave landmarks (62100-62138) correlate with Limgrave graces (76100-76199) | Soft validation |

### Files Added
- `src/discovery/chain_data.rs`: Static chain data and helper functions

### Files Modified
- `src/discovery/relationship_graph.rs`: 4 new RelationshipTypes
- `src/discovery/corroboration.rs`: 3 new validation methods, 3 new result types
- `src/discovery/mod.rs`: Exports for chain_data module

---

## v0.4.7 - Landmark Integration & Event Flag Geography

### Features
- **Landmark Category in Event Flags DB**: Added Landmark (62xxx) as a filterable category
  - 308 landmarks from LANDMARKS lookup table imported into database
  - Region resolution based on flag ID ranges (Limgrave, Liurnia, Caelid, etc.)
  - Light blue color coding in UI (RGB 180,220,255)
  - New filter button in category row

- **Event Flag Geography Documentation** (`docs/EVENT-FLAG-GEOGRAPHY.md`):
  - Complete world hierarchy (Regions → Sub-regions → Landmarks/Graces/Dungeons)
  - Geographic flag groupings (tile system, block-based, legacy dungeons)
  - Flag chaining systems (quests, area unlocks, merchant purchases, boss rewards)
  - Source game file reference with paths

### Bug Fixes
- **Fixed ~200 landmark byte offsets**: Flags 62100-62981 had incorrect offsets
  - Was using wrong formula `flag_id / 8` instead of `base_offset + (flag_id - block_start) / 8`
  - Old offsets: 0x1e52-0x1e73 (~7762-7795 bytes)
  - New offsets: 0x5e8-0x656 (~1512-1622 bytes)
  - Block 62000 base offset confirmed as 0x5dc (1500)

### Files Added
- `docs/EVENT-FLAG-GEOGRAPHY.md`: Comprehensive event flag system documentation
- `src/db/landmarks.rs`: Landmarks lookup table module

### Files Modified
- `src/db/event_flags.rs`: Corrected 62100-62981 byte offsets
- `src/db/event_flags_db.rs`: Added Landmark category, get_landmark_region(), LANDMARKS import
- `src/db/mod.rs`: Export landmarks module
- `src/ui/event_flags_db_view.rs`: Added Landmark filter and color

---

## v0.4.6 - Multi-Point Corroboration System

### Features
- **Relationship Graph Module** (`src/discovery/relationship_graph.rs`):
  - Loads 2,796 flag relationships across 5,079 flags from `scripts/flag_relationships.json`
  - Indexes relationships by source, target, and type for O(1) lookups
  - Extracts 122 dual-formula corroboration pairs (tile↔block)
  - Supports 6 relationship types: pickup_sets_flag, enables_purchase, grace_discovery, boss_remembrance, event_sequence, map_fragment

- **Corroboration Engine** (`src/discovery/corroboration.rs`):
  - Multi-point validation using relationship graph
  - Dual-formula validation: cross-checks tile flag (10-digit) with block flag (5-digit) for same item
  - Confidence scoring with agreement ratios
  - Batch validation of all corroboration pairs

- **New CLI Commands**:
  - `discovery corroborate <flag_id>` - Single flag validation with related flag checks
  - `discovery corroborate --all` - Batch validate all 122 corroboration pairs
  - `discovery graph` - Show relationship graph statistics

- **Flag Extraction Script** (`scripts/extract_flag_relationships.py`):
  - Extracts flag relationships from decompiled game files
  - Parses ItemLotParam_map, ShopLineupParam, BonfireWarpParam, common.emevd.js
  - Generates `flag_relationships.json` for runtime use

### Bug Fixes
- **Tile formula col_base corrected**: Changed from 42 to **30**
  - Actual column range is 30-58, formula was excluding columns 30-41
  - Discovered through corroboration analysis showing contradictions
  - Fixed in `ground_truth_offsets.json`

- **Bit mask bug in corroboration**: Changed `(1 << (7 - bit))` to `(1 << bit)`
  - Bit was already calculated as `7 - (flag % 8)`, double-negation caused wrong bit reads
  - Affected check_dual_formula, read_flag, and validate_all_pairs methods

### Validation Results
| Slot | Character | Agreements | Contradictions | Status |
|------|-----------|------------|----------------|--------|
| 0 | Confessor (mid-game) | 57 | 5 | Expected (4 world pickups + 1 shop) |
| 1 | Wretch (early-game) | 62 | 0 | Formula validated |

### Files Added
- `src/discovery/relationship_graph.rs`: Relationship graph loader and indexer
- `src/discovery/corroboration.rs`: Multi-point validation engine
- `scripts/extract_flag_relationships.py`: Game data extraction script
- `scripts/flag_relationships.json`: 2,796 relationships, 5,079 flags
- `tests/regression_suite.rs`: Ground truth schema validation tests

### Files Modified
- `src/discovery/mod.rs`: Export new modules
- `src/discovery/cli.rs`: Added corroborate and graph commands
- `ground_truth_offsets.json`: Fixed tile formula col_base (42→30)

---

## v0.4.5 - Dynamic Test Validation & UI Improvements

### Features
- **Dynamic Test Case Loading**: Test cases now load from verification records instead of hardcoded values
  - `DynamicTestCaseValidator` loads expectations from JSONL file
  - `--dynamic` or `--records <path>` flags for CLI validation
  - Adapts automatically when verification records are updated
  - `build_test_suite_from_records()` function for programmatic use

### UI Improvements
- **Catppuccin Frappé color palette** for verification view
  - Consistent colors: Red (#e78284), Green (#a6d189), Yellow (#e5c890), Peach (#ef9f76), Teal (#81c8be)
- **Monospace font size reduced to 85%** (12px) for better table density
- **Removed text truncation** - full flag names now visible with horizontal scrolling

### Bug Fixes
- Fixed verification records path: now correctly points to `verification-records.jsonl`

### Files Modified
- `src/discovery/test_cases.rs`: Added DynamicTestCaseValidator, build_test_suite_from_records()
- `src/discovery/cli.rs`: Added --dynamic, --records flags, Validator trait
- `src/ui/verification_view.rs`: Catppuccin Frappé palette, font sizing, no truncation
- `src/main.rs`: Fixed verification records path

---

## v0.4.4 - Block Offset Corrections

### Bug Fixes
- **Fixed block 76000 base offset**: Changed from 3248 to **3250** (was off by 2 bytes)
  - Root cause: Previous fix in v0.4.3 used wrong base offset
  - Validation showed 76101 (The First Step) returning FALSE for Wretch when it should be TRUE
  - Cross-referenced with elden-map verification tool to confirm correct offset

### CLI Improvements
- Added `--save <path>` parameter to `discovery validate` and `discovery probe` commands
- Commands now support custom save file paths instead of hardcoded default

### Test Case Updates
- Simplified test cases to only include reliably verifiable flags
- Removed unstable Confessor entries where save data has changed since verification
- All 6 slots now pass 100% validation (15/15 tests)

### Cross-Project Sync
- Synced block 73000 base offset fix to elden-map (2875 → 2662)
  - Updated `elden-map/server/src/verificationService.ts`
  - Updated `elden-map/server/src/eventFlagService.ts`

### Files Modified
- `ground_truth_offsets.json`: Block 76000 base_offset 3248 → 3250
- `src/discovery/cli.rs`: Added --save parameter parsing
- `src/discovery/test_cases.rs`: Simplified to verified flags only

---

## v0.4.3 - Test Case Validation System

### Features
- **Test Case Validator**: Curated test cases for verifying flag offset formulas
  - `FlagTestCase` struct with category, verification method, expected state
  - `SlotTestSuite` for per-character test suites
  - `TestCaseValidator` for running validation against save files
  - Helper functions: `grace()`, `world_pickup()`, `boss_defeat()`, `cookbook()`

- **CLI Commands**:
  - `discovery validate <slot> [slot...]` - Run curated test cases
  - `discovery validate --all` - Validate all defined slots
  - `discovery probe <slot> <offset>...` - Direct byte inspection for debugging

### Bug Fixes
- **Fixed 29 incorrect flag offsets** in `ground_truth_offsets.json` for 76xxx grace flags
  - All 76xxx flags were consistently 2 bytes off from correct formula
  - Root cause: Individual entries were added independently without verifying against block base
  - Fixed by recalculating offsets from verified block base (76000 → 3248)

### Verification Results
- The First Step (76101) validates correctly @ 0xcbc:2 = TRUE across slots 2, 3, 4
- Test case system distinguishes true positives from false negatives

### Files Created
- `src/discovery/test_cases.rs`: Test case infrastructure

### Files Modified
- `src/discovery/cli.rs`: Added validate and probe commands
- `src/discovery/mod.rs`: Export test_cases module
- `ground_truth_offsets.json`: Corrected 76xxx flag offsets

---

## v0.4.2 - Expanded Flag Catalog

### Features
- **Expanded Flag Catalog**: Increased from 7,034 to 22,376 documented flags
  - Extracted 5,047 flags from ItemLotParam_map.param.xml
  - Extracted 1,291 flags from ShopLineupParam.param.xml
  - Extracted 15,921 flags from event scripts (*.emevd.js)

- **Automatic Name Generation**: All discovered flags now get descriptive names
  - Pattern-based naming for undocumented flags (e.g., "Sewers Event 8642")
  - Dungeon/region prefixes: Stormveil, Raya Lucaria, Sewers, Cave, etc.
  - World pickup names include map tile coordinates
  - Catalog lookup takes precedence when flag is documented

### Technical Details
- `FlagCatalog::get_name_or_generate()` provides fallback naming
- `FlagCatalog::generate_flag_name()` maps ID patterns to descriptive names
- Batch analysis now loads catalog once and passes to all operations

### Files Created
- `scripts/expand_flag_catalog.py`: Extraction tool for expanding catalog

### Files Modified
- `scripts/extracted_event_flags.json`: Expanded from 7,034 to 22,376 flags
- `src/discovery/flag_catalog.rs`: Added name generation methods
- `src/discovery/integration.rs`: Use `get_name_or_generate()` for lookups
- `src/discovery/snapshot_batch.rs`: Load catalog for batch processing

---

## v0.4.1 - Discovery CLI Commands

### Features
- **CLI Interface**: Run discovery operations from command line
  - `discovery batch-analyze`: Process all snapshot pairs and persist discoveries
  - `discovery status`: Show discovery store statistics and consensus report
  - `discovery promotable`: List discoveries ready for promotion
  - `discovery promote [--dry-run]`: Promote confirmed discoveries to ground truth

### Usage
```bash
# Process snapshots
cargo run -- discovery batch-analyze

# Check status
cargo run -- discovery status

# Preview promotions
cargo run -- discovery promote --dry-run
```

### Files Created
- `src/discovery/cli.rs`: CLI command handlers

### Files Modified
- `src/main.rs`: CLI argument detection before GUI launch
- `src/discovery/mod.rs`: Added cli module export

---

## v0.4.0 - Event Flag Discovery System

### Features
- **Flag Catalog Integration**: Load and index 7,034 flags from `extracted_event_flags.json`
  - Search by name with multi-word query support
  - Autocomplete functionality for flag lookup
  - Category and region-based lookups (39 categories, 158 regions)

- **Discovery Store**: Persistent storage with full provenance tracking
  - Observations tracked with source type: SnapshotDiff, ProbeResult, CrossSlotValidation, ManualVerification
  - Status pipeline: Pending → Confirmed → Promoted (or Rejected)
  - Automatic consensus recalculation when observations are added
  - Persists to `discoveries.json`

- **Batch Snapshot Analyzer**: Process all granular before/after save snapshots
  - Parses filenames to extract character, sequence number, action description
  - Groups files into before/after pairs automatically
  - Runs differential discovery on each pair

- **Consensus Engine**: Multi-observation consensus with weighted voting
  - Source weights: Manual verification (1.0), Cross-slot (0.95), Snapshot diff (0.85), Probe (0.7)
  - Configurable thresholds: min 2 observations, 80% agreement to confirm
  - Reports contested vs confirmed discoveries

- **Cross-Slot Validator**: Validate discoveries across multiple save slots
  - Checks same offset/bit across different character slots
  - Confidence adjustments based on agreement/disagreement
  - Supports batch validation

- **Ground Truth Updater**: Safe automated updates to `ground_truth_offsets.json`
  - Timestamped backups before any modification
  - Block base recalculation when enough flags confirmed
  - Rollback capability

### Technical Details
- Consensus requires: 2+ observations, 80%+ agreement, 75%+ confidence for promotion
- Finding one verified flag in a block unlocks ~125 adjacent flags (block formula)
- 41 unit tests added (7 integration tests require save files)

### Files Created
- `src/discovery/flag_catalog.rs`: Flag catalog loader and search
- `src/discovery/discovery_store.rs`: Persistent discovery storage
- `src/discovery/snapshot_batch.rs`: Batch snapshot processor
- `src/discovery/consensus.rs`: Consensus building engine
- `src/discovery/cross_validator.rs`: Cross-slot validation
- `src/discovery/ground_truth_updater.rs`: Safe ground truth updates

### Files Modified
- `src/discovery/mod.rs`: Added new module exports
- `src/discovery/offset_probe.rs`: Added persistence hooks
- `src/discovery/integration.rs`: Added persistence-enabled workflows
- `Cargo.toml`: Added chrono dependency for timestamps

---

## v0.3.4 - Verification Integration & Detection Categories

### Features
- **Verification moved to Event Flags**: Verification view now integrated as a per-character tab within Event Flags section instead of a standalone database view
  - Loads verification records specific to selected character slot
  - Per-slot loading state tracked with `verification_loaded_slots: [bool; 10]`

- **Detection category refactor**: Renamed misleading "False Positive" labels to proper detection categories
  - `FormulaError` (RED): manual=true, auto=false - User confirmed collection but formula missed it. **Primary indicator of formula problems**
  - `PendingVerification` (ORANGE): auto=true, manual=false - Formula detected but not manually confirmed. Could be: forgotten, no POI exists, or actual error
  - `UndiscoveredRegion` (YELLOW): Both agree but no graces discovered in region. Informational only

- **Enhanced flagged detection UI**:
  - Color-coded rows by detection category severity
  - Auto-opens section when Formula Errors exist (immediate attention needed)
  - Hover tooltips with detailed descriptions
  - Context menu with copy options and full details
  - Formula error count prominently displayed at top

- **Updated export format**: New fields in verification export
  - `flagged_count`, `formula_error_count`, `informational_count`
  - `flagged_by_category` breakdown
  - `FlaggedDetectionExport` with `detection_category`, `is_error`, `description` fields

### Technical Details
- Verification methodology: Only flags EXPLICITLY marked as complete are in verification file
  - `manual=false` is ambiguous (true negative OR forgotten)
  - `manual=true, auto=false` is the reliable signal for formula errors
- Formula Errors sorted first in flagged list for priority attention
- 45 Formula Errors identified for investigation

### Files Modified
- `src/vm/verification_vm.rs`: Refactored detection categories and methods
- `src/vm/events.rs`: Added `Verification` route and `verification_vm` field
- `src/ui/events.rs`: Added Verification tab to Event Flags
- `src/ui/verification_view.rs`: Updated UI with color coding and auto-open
- `src/ui/menu.rs`: Removed standalone Verification route
- `src/vm/export.rs`: Updated export structures
- `src/vm/slot.rs`: Updated export building
- `src/main.rs`: Per-slot verification loading

---

## v0.3.3 - Improve Event Flags Offset Detection Accuracy

### Critical Bug Fix
- **Negative validation for offset detection**: Fixed false positives in event flags detection
  - Previous algorithm found 396 offsets matching the 4 tutorial grace pattern
  - Was picking first match (0x12CF0) instead of correct offset (0x12D76)
  - Added 6 late-game grace flags as NEGATIVE validation (must NOT be set)
  - Algorithm now: (1) finds all 4/4 positive matches, (2) picks best by negative score
  - Fixes late-game items incorrectly showing as "collected" for early-game characters

### Technical Details
- Detection now uses 3-phase approach:
  1. Find all offsets where ALL positive flags match (early-game graces)
  2. Among those, pick offset with highest negative score (late-game graces NOT set)
  3. Fall back to partial match if no perfect match found

### Files Modified
- `src/save/common/event_flags_detection.rs`: Added `NEGATIVE_VALIDATION_FLAGS`, rewrote detection algorithm

---

## v0.3.2 - Fix Event Flags Offset Detection

### Critical Bug Fix
- **Dynamic event flags offset detection**: Fixed save parser using wrong hardcoded offset
  - Was using `0x1a104` (~107KB) but actual offset is `~0x12B00` (~76KB)
  - Caused massive false positives (55 items showing "collected" when they weren't)
  - Now uses validation-based detection to find correct offset per slot
  - Detection uses anchor flags (tutorial graces) that all characters should have

### Files Modified
- `src/save/common/save_slot.rs`: Replace hardcoded offset with dynamic detection

---

## v0.3.1 - Wire Generated Ground Truth into App

### Bug Fix
- **pickup_flags.rs now uses verified offsets**: The generated ground truth module is now wired into the app's event flag calculations
  - Tile base offset: 347,000 → **495,830** (verified)
  - Block bases: Now uses `VERIFIED_BLOCK_BASES` from JSON
  - Dungeon bases: Uses verified bases for areas 30, 31, 32 (catacombs, caves, tunnels)

### Features
- **Untrackable flag filtering**: LocalId > 6999 now returns `None` (consumables with localId 7000+ cannot be tracked)
- **Status-aware dungeon lookup**: Only uses verified dungeon bases when status is "verified"

### Files Modified
- `src/main.rs`: Added `mod generated;`
- `src/db/pickup_flags.rs`: Imports from generated module, uses verified constants

---

## v0.3.0 - Ground Truth Code Generation & Cross-Project Integration

### Features
- **Code Generation from JSON** (`build.rs`): Generates Rust code from `ground_truth_offsets.json` at compile time
  - `src/generated/ground_truth.rs`: Auto-generated with verified block bases, tile formula, dungeon bases
  - Provides `calculate_block_flag_offset()`, `calculate_tile_flag_offset()`, `calculate_dungeon_flag_offset()`
  - Single source of truth shared between Rust and TypeScript projects

- **TypeScript Integration** (elden-map): Symlink and TypeScript module for web app
  - `ground-truth-formulas.ts`: Type-safe offset calculation functions
  - Imports directly from shared `ground_truth_offsets.json`

- **Character Slot Identification**: Test output now shows character names and per-slot flag status
  - Extracts UTF-16LE names from save slots at variable offsets
  - Display format: `Slot 0 (Confessor): [✓ ✓ ✓ ✓ ✓ ✓]`

- **Formula Test Suite** (`scripts/verification/test_formulas.py`): Comprehensive formula validation
  - Tests block, tile, and dungeon formulas against actual save data
  - Reports per-slot verification status

### Verification Results
- **392 flags proven** (from 656 tested)
- **Block formula**: Verified for 60000, 62000, 67000, 71000, 73000, 76000 ranges
- **Tile formula**: Verified with base offset 495830
- **Dungeon formula**: Verified for areas 30 (catacombs), 31 (caves), 32 (tunnels)

### Files Modified
- `build.rs`: Extended with JSON code generation
- `Cargo.toml`: Added serde_json build dependency, bumped to 0.3.0
- `src/generated/mod.rs`: Module wrapper for generated code
- `.gitignore`: Exclude generated ground_truth.rs
- `scripts/verification/test_formulas.py`: Added character slot display
- `scripts/verification/save_parser.py`: Added character name extraction

---

## v0.2.9 - Event Flag Verification Framework

### Features
- **Verification Framework** (`scripts/verification/`): Complete Python tool suite to systematically test and verify event flag formulas against actual save files
  - `save_parser.py`: Structural save file parsing with dynamic offset detection
  - `flag_formulas.py`: All known formulas (block, tile, dungeon) with documented limitations
  - `diff_analyzer.py`: Before/after comparison for empirical offset discovery
  - `data_loader.py`: Loads extracted flags and manual completions
  - `verification_data.py`: Data structures for tracking verification status

- **Ground Truth Documentation** (`docs/SAVE_FILE_GROUND_TRUTH.md`): Single source of truth consolidating all save file parsing research
  - Verified constants and formulas
  - Known limitations documented (consumable treasures untrackable)
  - Formula accuracy statistics

- **Verification Runner** (`scripts/run_verification.py`): Main script to run verification pipeline
  - Tests all flag formulas against save data
  - Generates `ground_truth_offsets.json` with verified offsets
  - Reports formula accuracy by category

### Verification Results
- **81 grace flags verified** (block formula working)
- **Block formula**: 26.6% accuracy with evidence
- **Tile formula**: Needs dungeon base offset discovery
- **Dungeon formula**: 101/104 base offsets unknown

### Key Findings
- Block-based formulas (65xxx-76xxx) work reliably for graces/cookbooks
- LocalId >= 7000 flags are **structurally untrackable** (875 bytes/slot = 7000 flags max)
- Consumable treasures (Golden Runes, Smithing Stones) cannot be tracked via event flags

### Usage
```bash
python scripts/run_verification.py --verbose
```

---

## v0.2.8 - Treasure Metadata Fields

### Features
- **Treasure Type Classification**: Added `treasure_type` field to event flags
  - Detects: chest, corpse, cart, ground_pickup based on MSB InChest field and asset patterns
  - Cart treasures (AEG100_101) correctly identified with known position error

- **Item Rarity Lookup**: Added `item_rarity` field from EquipParam files
  - 0 = consumable (white glow)
  - 1 = standard (white glow)
  - 2 = rare/unique (purple glow)
  - 3 = legendary (orange glow)

- **Position Confidence**: Added `position_confidence` field
  - `high`: chest/corpse positions (~40 unit accuracy)
  - `low`: cart positions (~70-100 meter error due to model origin vs interact point)
  - `none`: no position data available

- **Underground Detection**: Added `is_underground` field
  - Uses filename keywords (地下, 洞窟, 地底, 地下室, 坑道)
  - Falls back to area_type (underworld/subterranean = underground)
  - Returns null when uncertain to avoid false positives

### Coverage
- Treasure types: corpse (1,937), ground_pickup (278), chest (201), cart (13)
- Item rarities: common (1,391), standard (1,339), rare (1,225), legendary (154)
- Position confidence: high (2,416), low (13), none (4,605)
- Underground detection: confident (2,162), uncertain (4,872)

---

## v0.2.7 - POI Region Derivation & Generic NPC Filtering

### Features
- **POI Region Extraction**: Added `get_region_from_poi_name()` function
  - Parses POI paramdexName to extract accurate region names
  - Handles Legacy Dungeon, Guidance of Grace, Minor Erdtree, Divine Tower patterns
  - Fixes POIs like "Crumbling Farum Azula" showing region "Various"

- **Generic NPC Filtering**: Added `filter_generic` parameter to NPC extraction
  - Excludes NPCs with generic names like "NPC (c1000)", "NPC (c0000)"
  - Reduces noise in exported data (541 generic NPCs filtered)
  - Keeps 305 named NPCs for cleaner output

### Improvements
- **Multi-method region assignment** for WorldMapPointParam:
  1. Extract from POI name (paramdexName)
  2. Derive from 10-digit flag ID
  3. Use grid coordinates for overworld areas
  4. Fallback to "Various"

### Coverage
- Total unique flags: 7,575 → 7,034 (filtered generic NPCs)
- POI region accuracy improved for legacy dungeons

---

## v0.2.6 - NPC Name Resolution Lookup Table

### Features
- **NPC Name Lookup Table**: Added coordinate-matched lookup for quest NPCs
  - 40 key NPCs now resolved instead of showing generic "NPC (c1000)"
  - Auto-generated by matching MSB entity positions against elden-map POI database
  - High-confidence matches only (distance < 40 units)

### NPCs Now Resolved
- Quest NPCs: Roderika, Ranni, Millicent, Boc, Patches, Hyetta, Melina
- Merchants: Kalé, Hermit Merchant, Nomadic Merchants, Isolated Merchants
- Key Characters: Iron Fist Alexander, Knight Bernahl, Edgar, Jerren
- Special: Miriel Pastor of Vows, Primeval Sorcerer Azur, Great-Jar

### Coverage Improvement
- Generic NPC (c1000): 558 → 518 (-40 resolved)
- Named NPCs: ~228 → 262 (+34)
- Lookup table can be expanded as new mappings are discovered

### Data Sources
- Coordinate matching against merged-pois.json from elden-map project
- Entity IDs from MSB Part/Enemy files

---

## v0.2.5 - Map Feature Extraction (Boss Arenas, Stakes, Spirit Springs)

### Features
- **Boss Arena Extraction**: Parse GameAreaParam for boss arena locations
  - 150+ boss arenas with defeat flags and coordinates
  - Boss discovery flags for tracking boss encounters
  - Soul reward data (single player and multiplayer)
  - Region names extracted from boss name prefixes (e.g., "[Stormveil Castle]")

- **Dungeon Info Extraction**: Parse MapDefaultInfoParam for dungeon data
  - Fast travel unlock flags (EnableFastTravelEventFlagId)
  - Links dungeon completion to boss defeats
  - 80+ dungeon entries with named locations

- **Stake of Marika Extraction**: Parse MSB SpawnPoint regions
  - 85+ Stakes of Marika with positions
  - Entity IDs for respawn point tracking
  - Distributed across dungeons and legacy areas

- **Spirit Spring Extraction**: Parse MSB MountJump regions
  - 90+ Spirit Springs with positions
  - Jump height data for each spring
  - Overworld locations with world coordinates

- **Region Name Lookup**: Load region names from MapGdRegionInfoParam
  - 135+ named regions and dungeons
  - Used for proper region classification

### New Event Flag Categories
- Boss Arena: 150+ flags with coordinates
- Boss Discovery: Flags for boss encounters
- Dungeon Cleared: Fast travel unlock flags
- Stake of Marika: 85+ respawn points
- Spirit Spring: 90+ jump pads

### Coverage Improvement
- Total unique flags: 8,052 → **7,575** (deduplicated, removed overlapping flags)
- Spatial data coverage: 60% with local coords, 77% with map tiles
- MSB-sourced coordinates: 3,444 entries

### Data Sources Added
- GameAreaParam.param.xml (boss arenas with coordinates)
- MapDefaultInfoParam.param.xml (dungeon fast travel flags)
- MapGdRegionInfoParam.param.xml (region name lookup)
- MSB Region/SpawnPoint/*.xml (Stakes of Marika)
- MSB Region/MountJump/*.xml (Spirit Springs)

---

## v0.2.4 - Enemy Defeat Flag Extraction & NPC Locations

### Features
- **MSB Enemy Extraction**: Parse MSB Part/Enemy/*.xml for boss/enemy positions
  - Cross-validates EntityIDs against event scripts for accuracy
  - Includes enemies from multiple tracking sources:
    - `SetNetworkconnectedEventFlagID` for general tracking
    - `HandleBossDefeatAndDisplayBanner` for boss defeats
    - `InitializeCommonEvent(90005860)` for field boss defeats
    - `InitializeCommonEvent(90005870)` for boss name tracking
  - 174 verified enemy defeat flags with coordinates

- **Enemy Name Resolution** (priority order):
  1. NpcName.fmg via constructed nameId (9 + model + variation) - gives full in-game names like "Margit, the Fell Omen", "Tree Sentinel"
  2. BgmBossChrIdConv for major boss display names (Godrick, Rennala, Malenia, etc.)
  3. ChrModelParam.paramdexName for general enemy names (266 model mappings)
  4. NPCParamID → nameId → NpcName.fmg fallback

- **Enemy Type Classification** (based on entity ID patterns and model):
  - `Great Boss`: Main demigods with c2xxx/c4xxx models (Godrick, Rennala, Malenia, Mohg, etc.)
  - `Boss`: Entity IDs ending in 0800/0801 (Tree Sentinel, Night's Cavalry, dungeon bosses)
  - `Field Boss`: Entity IDs ending in 0850/0851 (Margit pre-Stormveil, various)
  - `Invasion`: Player model (c0000) NPC invaders
  - `Enemy`: Other trackable one-time enemies

- **NPC Location Extraction**: Extract characters with dialog (TalkID > 0)
  - 846 unique NPCs with positions from MSB files
  - NPC type classification: Merchant, Smith, Quest NPC, Trainer, etc.
  - Includes key NPCs like War Counselor Iji, Nomadic Merchants, questgivers
  - Uses EntityID as flag ID for tracking (most NPCs lack explicit event flags)

### New Event Flag Categories
- Great Boss Defeat: 88 flags
- Boss Defeat: 58 flags
- Field Boss Defeat: 23 flags
- Invasion Defeat: 2 flags
- Enemy Defeat: 2 flags
- Elite Enemy Defeat: 1 flag
- NPC (with dialog): 846 entries

### Coverage Improvement
- Total unique flags: 6,213 → **8,052** (+1,839 flags)
- Enemy defeat flags: 174 with verified positions
- NPCs with dialog: 846 with positions from MSB files

### Data Sources Added
- NpcParam.param.xml (7,038 NPC definitions)
- ChrModelParam.param.xml (266 model → name mappings)
- WwiseValueToStrParam_BgmBossChrIdConv.param.xml (15 boss names)
- NpcName.fmg.xml (479 NPC names with constructed nameId lookup)
- MSB Part/Enemy/*.xml (positions for 174 verified enemies + 846 NPCs)
- Event scripts (*.emevd.js) for defeat flag validation

---

## v0.2.3 - Multi-Item Chest Position Linking

### Features
- **Multi-item chest linking**: Secondary items in a chest now inherit position from the base item
  - Example: Ash of War: Storm Stomp (row 1042371011) now gets position from Whetstone Knife (row 1042371010) since they're in the same chest
  - Checks consecutive row IDs (row_id-1 through row_id-10) for MSB treasure entries
  - New field `msb_base_row_id` tracks when position came from a different row

### Coverage Improvement
- MSB positions used: 2,368 → **2,504** (+136 items)
- Flags with local coords: 51% → **52%**

---

## v0.2.2 - MSB Area/Grid Extraction Fix

### Bug Fixes
- **Parse area/grid from MSB directory names**: Flags like Whetstone Knife (60130) that don't encode location in their ID now get area/grid info from the MSB directory name (e.g., `m60_42_37_00-msb-dcx` → area=60, grid=(42,37))
- This enables correct world coordinate calculation for ~76 additional flags

### Coverage Improvement
- Flags with world coords: 24% → **25%** (+76 flags)
- Flags with map tile: 70% → **72%** (+121 flags)

---

## v0.2.1 - MSB Position Data & Area Type Classification

### Features
- **MSB Treasure Position Extraction**: Parse Map Studio Binary files for accurate item positions
  - Loads treasure positions from 935 MSB directories
  - Links ItemLotID → TreasurePartName → Asset Position
  - 2,379 treasure positions extracted, 2,368 matched to event flags
  - Position source tracked in `raw_data.position_source` ("MSB" or "WorldMapPointParam")

- **Area Type Classification**: Distinguish location types for proper coordinate handling
  - `overworld_surface`: Open world (area 60 base, 61 DLC) - world coords valid
  - `underworld`: Underground open areas (area 12) - Siofra, Ainsel, Nokron
  - `subterranean`: Deep underground (area 35) - Shunning-Grounds, Mohgwyn
  - `legacy_dungeon`: Major story dungeons (areas 10-16, 19-28)
  - `minor_dungeon`: Caves, catacombs, tunnels (areas 30-32, 39-43)
  - `divine_tower`: Divine Tower locations (area 34)
  - `tutorial`: Tutorial area (area 18)

- **Base Game vs DLC Distinction**: New `is_dlc` field for filtering

### Bug Fixes
- **Fixed world coordinate calculation for dungeons**: Previously applied `grid * 256 + pos` formula to all locations, which is only valid for overworld tiles. Dungeon coordinates are now correctly left as local positions with `world_x`/`world_z` set to null.

### New Fields
- `is_overworld`: Boolean - true only for area 60/61
- `world_x`, `world_z`: Computed world coordinates (null for non-overworld)
- `area_type`: Location classification string
- `is_dlc`: Boolean - true for Shadow of the Erdtree content

### Spatial Data Coverage
- Flags with local coords: 51% (3,174/6,213)
- Flags with world coords: 24% (1,510/6,213) - overworld only
- Flags with map tile: 70% (4,362/6,213)
- Coordinates from MSB files: 2,368

---

## v0.2.0 - Event Flags Database

### Features
- **Event Flags DB View**: New comprehensive database view with ~5,000+ event flags
  - Category filtering (22 categories including Great Runes, Graces, Cookbooks, etc.)
  - Region dropdown filtering
  - Text search by name or flag ID
  - JSON export (full database or filtered results)

- **Enhanced Extraction Script** (`scripts/extract_event_flags.py`):
  - DLC01 name file support for proper DLC item names
  - Fixed Crystal Tear vs Whetblade categorization
  - Added `common.emevd.js` parsing for Great Runes, Remembrances, Talisman Pouches
  - Markdown and JSON output formats with full data preservation
  - **Spatial data extraction**: map tiles, XYZ coordinates, region IDs
  - 6,213 unique event flags extracted across 23 categories

### Spatial Data Coverage
- Graces: 100% with full coordinates (422 entries)
- Landmarks: 100% with full coordinates (379 entries)
- World pickups: 81% with map tiles derived from flag ID
- New fields: `area_no`, `grid_x`, `grid_z`, `pos_x`, `pos_y`, `pos_z`, `map_tile`, `region_id`

### Data Sources
- `ItemLotParam_map.param.xml` - World pickups
- `BonfireWarpParam.param.xml` - Grace sites (with coordinates)
- `ShopLineupParam.param.xml` - Shop items
- `WorldMapPointParam.param.xml` - POI locations (with coordinates)
- `WorldMapPieceParam.param.xml` - Region definitions
- `common.emevd.js` - Event scripts (Great Runes, Remembrances)

### Bug Fixes
- Fixed Map Fragment category showing only 1 entry (was being overwritten by WorldPickup category)
- Fixed Crystal Tears (65000-65399) being miscategorized as Whetblades (65610-65720)

---

## v0.1.0 - Initial Release

- Core save file parsing and editing
- Character stats editing
- Inventory management
- Equipment editing
- Grace/Boss tracking
- Regions database
