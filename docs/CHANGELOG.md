# Changelog

All notable changes to ER-save-Editor will be documented in this file.

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
- Map POIs: 100% with full coordinates (379 entries)
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
