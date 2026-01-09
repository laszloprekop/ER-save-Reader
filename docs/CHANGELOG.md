# Changelog

All notable changes to ER-save-Editor will be documented in this file.

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
