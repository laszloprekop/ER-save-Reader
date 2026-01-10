# Changelog

All notable changes to ER-save-Editor will be documented in this file.

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
