# Flag Catalog vs Param Database

| Aspect | Flag Catalog | Param Database |
|--------|--------------|----------------|
| Source file | extracted_event_flags.json | Regulation-bin XML params |
| Flag count | 7,034 | 6,706 |
| Primary source | MSB files (map data) | Game param tables |
| Extraction | Pre-extracted, static | Dynamic from XML |

## What Each Contains

### Flag Catalog - Spatial/Display focused:

- flag_id, name, category, region
- World coordinates (pos_x, pos_y, pos_z, world_x, world_z)
- Map tile, area_no, grid_x, grid_z
- Item associations (item_id, item_category, treasure_type)
- Area classification (is_overworld, is_dlc, is_underground)

### Param Database - Source/Semantic focused:

- flag_id, category (simple/block/midrange/dungeon/tile)
- Source: which param file, row ID, field name
- Boss names (from GameAreaParam)
- Semantic meaning inferred from param:
  - ItemLotParam_map → world pickup
  - BonfireWarpParam → grace discovery
  - ShopLineupParam → shop unlock
  - GameAreaParam → boss defeat

### Coverage Comparison

Flag Catalog: Primarily world pickups from MSB files
Strong on: positions, item names, regions

Param Database: Game mechanic flags from params
Strong on: graces, shops, bosses, unlocks
Includes flags NOT tied to map positions

## Overlap Analysis

Param Database: 6,706 flags
Flag Catalog: 22,376 flags

In BOTH: 6,652 flags (99.2% of param)
Param only: 54 flags
Catalog only: 15,724 flags

## Key Insight

The Flag Catalog is a superset - it contains almost all param flags plus 15,724 more from other sources (MSB
files, event scripts, etc.).

## So Why Have Both?

| Flag Catalog | Param Database |
|--------------|----------------|
| What the flag represents | Where in game data it comes from |
| Display name, region, position | Param file, row ID, field name |
| Good for: UI, search, display | Good for: validation, tracing, debugging |

### Example - Flag 76100 (First Step grace):

Catalog knows: - Name: "First Step" - Category: Grace - Region: Limgrave - Position: (x, y, z)

Param Database knows: - Source: BonfireWarpParam, row 76100, field eventflagId - Also has: clearedEventFlagId, textEnableFlagId entries

## Recommendation

Merge them into a unified flag database:

- Catalog provides the display layer (names, positions)
- Param Database provides the source layer (traceability)
- Event Graph provides the trigger layer (EMEVD evidence)

This would give us a complete picture: what a flag is, where it's defined, and how it gets set.
