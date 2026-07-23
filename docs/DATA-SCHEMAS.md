# Data Schemas Reference

This document defines the data schemas used by the data services layer.

## Table of Contents

1. [POI Domain](#1-poi-domain)
2. [Event Flag Domain](#2-event-flag-domain)
3. [Character Domain](#3-character-domain)
4. [Zone Domain](#4-zone-domain)
5. [Save File Parser Domain](#5-save-file-parser-domain-hex-viewer)
6. [User Progress Domain](#6-user-progress-domain)
7. [Service Cache Schemas](#7-service-cache-schemas)
8. [Schema Relationships](#schema-relationships-diagram)

---

## 1. POI Domain

```typescript
// ═══════════════════════════════════════════════════════════════════
// POI (Point of Interest) Schemas
// Source: /data/merged-pois.json, /data/comprehensive-pois.json
// ═══════════════════════════════════════════════════════════════════

interface MapLocation {
  // Identity
  id: string                              // Unique identifier
  title: string                           // Display name
  description?: string                    // Optional description

  // Coordinates (dual system)
  latitude: number                        // Geographic Y (for Mapbox)
  longitude: number                       // Geographic X (for Mapbox)
  gameWorldX?: number                     // Game coordinate X (source of truth)
  gameWorldZ?: number                     // Game coordinate Z (source of truth)

  // Classification
  categoryId: number                      // FK to Category
  markerType?: string                     // Icon type for rendering

  // Event Flag Integration
  eventFlag?: number                      // FK to GameFileEventFlag.flagId

  // Source Tracking
  poiSource: 'mapgenie' | 'game' | 'merged'
  matchMethod?: 'event_flag' | 'title' | 'coordinate' | 'manual'

  // State
  completed?: boolean                     // User completion status
}

interface Category {
  id: number                              // Primary key
  title: string                           // Display name (e.g., "Sites of Grace")
  icon: string                            // Icon identifier
  color: string                           // Hex color for markers
  visible: boolean                        // Filter visibility state
  count: number                           // Total POIs in category
}

interface ComprehensivePOI {
  // Identity
  id: string                              // Unique identifier
  type: string                            // POI type (grace, boss, item, etc.)
  name: string                            // Display name
  subtype?: string                        // Sub-classification
  region?: string                         // Game region name

  // Game Data
  eventFlag: number                       // Event flag ID
  mapId: string                           // Map tile ID (e.g., "m60_42_36")
  position: {
    x: number                             // Game X coordinate
    z: number                             // Game Z coordinate
  }
}

interface DungeonTransform {
  baseTile: { x: number; z: number }      // Tile offset
  baseOffset: { x: number; z: number }    // Pixel offset
  baseSrc: { x: number; z: number }       // Source anchor point
  scale: { x: number; z: number }         // Scale factors
  dstArea: 60 | 61                        // Destination area ID
}
```

---

## 2. Event Flag Domain

```typescript
// ═══════════════════════════════════════════════════════════════════
// Event Flag Schemas
// Source: /data/event-flags-gamefiles.json (from game param files)
// ═══════════════════════════════════════════════════════════════════

type EventFlagCategory =
  | 'grace'           // Sites of Grace discovery
  | 'boss'            // Boss defeat flags
  | 'item'            // World item pickups
  | 'shop'            // Shop/merchant flags
  | 'map'             // Map fragment acquisition
  | 'dungeon'         // Dungeon completion
  | 'poi'             // Generic points of interest
  | 'enemy_drop'      // Enemy loot flags
  | 'spirit_summon'   // Spirit ash activation
  | 'recipe'          // Cookbook acquisition
  | 'region'          // Region discovery
  | 'npc'             // NPC interaction flags
  | 'tutorial'        // Tutorial completion
  | 'system'          // System/game state flags

interface GameFileEventFlag {
  // Identity
  flagId: number                          // Primary key (e.g., 62050, 71800)
  name?: string                           // Human-readable name
  textId?: number                         // FK to FMG text lookup

  // Classification
  category: EventFlagCategory             // Flag type
  subcategory?: string                    // Further classification

  // Location
  mapId?: string                          // Map tile (e.g., "m60_48_52")
  position?: {
    x: number
    y: number                             // Elevation
    z: number
  }
  areaNo?: number                         // Area number
  gridX?: number                          // Grid X position
  gridZ?: number                          // Grid Z position
  entityId?: number                       // In-game entity reference

  // Item-specific (category: 'item')
  items?: ItemInfo[]                      // Items in this lot
  lotItemRarity?: number                  // Rarity tier
  inChest?: boolean                       // Is in a chest

  // Boss-specific (category: 'boss')
  location?: string                       // Boss arena location
  runeReward?: number                     // Runes on defeat
  bossType?: 'defeat' | 'found'           // Flag trigger type

  // Map-specific (category: 'map')
  mapFlagType?: 'acquisition' | 'open'    // How map was obtained

  // Shop-specific (category: 'shop')
  shopFlagType?: 'stock' | 'release'      // Stock vs unlock flag
  merchant?: string                       // Merchant name
  price?: number                          // Item price

  // NPC-specific (category: 'npc')
  npcId?: number                          // NPC entity ID
  npcName?: string                        // NPC name

  // Metadata
  rowId: number                           // Source param row ID
  source: string                          // Source param file name
}

interface ItemInfo {
  id: number                              // Item ID
  category: number                        // Item category code
  categoryName: string                    // 'weapon' | 'armor' | 'goods' | etc.
  name?: string                           // Item name (from FMG)
  quantity: number                        // Stack count
}

interface EventFlagDatabase {
  version: string                         // Database version
  generatedAt: string                     // ISO timestamp
  source: string                          // Generation source description
  stats: {
    totalFlags: number
    byCategory: Record<EventFlagCategory, number>
  }
  flags: GameFileEventFlag[]              // All flags
}
```

---

## 3. Character Domain

```typescript
// ═══════════════════════════════════════════════════════════════════
// Character Schemas (Live WebSocket + Static Export)
// Source: WebSocket saveParserClient, /data/character-export.json
// ═══════════════════════════════════════════════════════════════════

interface SaveFileData {
  metadata: {
    file_size: number
    format: string                        // "PC" | "PS"
    active_slot: number                   // 0-9
    total_characters: number
  }

  character: CharacterData
  all_characters: CharacterSlot[]

  inventory: GeneralInventory
  equipment: Equipment

  event_flags?: Record<number, boolean>   // flagId → isSet
  boss_defeats: Record<string, boolean>   // bossName → isDefeated

  collectibles: {
    golden_seeds: CollectibleStatus
    sacred_tears: CollectibleStatus
    crystal_tears?: CollectibleStatus
    smithing_materials?: SmithingMaterials
  }
}

interface CharacterData {
  character_name: string
  level: number
  souls: number                           // Current runes
  souls_memory: number                    // Total runes acquired

  // Stats
  hp: number
  max_hp: number
  fp: number
  max_fp: number
  stamina: number
  max_stamina: number

  // Attributes (8 total)
  vigor: number
  mind: number
  endurance: number
  strength: number
  dexterity: number
  intelligence: number
  faith: number
  arcane: number

  // Progression
  play_time?: number                      // Seconds
  deaths?: number
}

interface CharacterSlot {
  index: number                           // 0-9
  name: string
  level: number
  is_active: boolean
}

interface Equipment {
  // Weapons (with upgrade level)
  right_hand_1?: EquipmentSlot
  right_hand_2?: EquipmentSlot
  right_hand_3?: EquipmentSlot
  left_hand_1?: EquipmentSlot
  left_hand_2?: EquipmentSlot
  left_hand_3?: EquipmentSlot

  // Ammunition
  arrows_1?: EquipmentSlot
  arrows_2?: EquipmentSlot
  bolts_1?: EquipmentSlot
  bolts_2?: EquipmentSlot

  // Armor
  head?: EquipmentSlot
  chest?: EquipmentSlot
  arms?: EquipmentSlot
  legs?: EquipmentSlot

  // Talismans
  talisman_1?: EquipmentSlot
  talisman_2?: EquipmentSlot
  talisman_3?: EquipmentSlot
  talisman_4?: EquipmentSlot

  // Quick items (D-pad)
  quick_items?: QuickItem[]
}

interface EquipmentSlot {
  id: number                              // Item ID
  name: string                            // Display name
  upgrade?: number                        // +0 to +25
  affinity?: string                       // "Heavy", "Keen", etc.
  ash_of_war?: string                     // Applied Ash of War
}

interface GeneralInventory {
  weapons: InventoryItem[]
  armor: InventoryItem[]
  talismans: InventoryItem[]
  sorceries: InventoryItem[]
  incantations: InventoryItem[]
  ashes_of_war: InventoryItem[]
  spirit_ashes: InventoryItem[]
  consumables: InventoryItem[]
  materials: InventoryItem[]
  key_items: InventoryItem[]
  info_items: InventoryItem[]
}

interface InventoryItem {
  id: number
  name: string
  quantity: number
  max_stack?: number
  upgrade?: number                        // For weapons
  category?: string
}

interface CollectibleStatus {
  collected: number
  total: number
  items?: string[]                        // Names of collected items
}
```

### Save-editor JSON export — world pickups

Emitted by `src/vm/export.rs` (`ExportWorldPickupItem`), one object per
`WORLD_PICKUPS` entry, under `events.world_pickups`.

```ts
interface ExportWorldPickupItem {
  lot_id: number
  flag_id: number
  item_name: string
  item_type: string
  quantity: number
  region: string
  collected: boolean | null    // null = UNKNOWN, see below
}
```

> **`collected` became nullable in v0.30.0** (was `boolean`). `null` means the flag's
> position could not be resolved for that save — DLC tiles, ids belonging to no verified
> family, doubly-allocated maps — and is deliberately NOT reported as `false`. An export
> that prints "not collected" for a flag it could not read is asserting something it does
> not know, which is the failure this migration exists to remove. Roughly 1,517 of 4,809
> entries are `null` on a current save. Consumers that assumed a boolean must handle it.

---

## 4. Zone Domain

```typescript
// ═══════════════════════════════════════════════════════════════════
// Zone Boundary Schemas
// Source: src/data/zone-boundaries.json, src/data/zones.json
// ═══════════════════════════════════════════════════════════════════

interface ZoneBoundary {
  id: string                              // Zone identifier
  name: string                            // Display name

  // Geography
  coordinates: [number, number][]         // Polygon [lng, lat] pairs
  labelCoordinates: [number, number]      // Label anchor point

  // Progression
  levelMin: number                        // Recommended min level
  levelMax: number                        // Recommended max level
  progressionId: number                   // Order in game progression

  // Classification
  mapType: 'overworld' | 'underground'
}

interface ZoneMetadata {
  id: string
  title: string
  mapType: 'overworld' | 'underground'
  progressionId: number
  levelRange: {
    min: number
    max: number
  }
}
```

---

## 5. Save File Parser Domain (Hex Viewer)

```typescript
// ═══════════════════════════════════════════════════════════════════
// Save File Parser Schemas (Character Explorer)
// Source: Uploaded .sl2 files, parsed in-browser
// ═══════════════════════════════════════════════════════════════════

interface SlotData {
  // Raw data
  buffer: ArrayBuffer                     // Full slot data (~2.6MB)
  slotData: Uint8Array                    // View into buffer

  // Metadata
  characterName: string
  slotIndex: number                       // 0-9
  saveVersion: number                     // Game version (e.g., 251)

  // Structure
  eventFlagsOffset: number                // Computed offset to event flags
  sections: ParsedSection[]               // All identified sections
  chunks: DataChunk[]                     // Parsed data items
}

interface ParsedSection {
  id: string                              // Section identifier
  name: string                            // Display name
  offset: number                          // Absolute byte offset in slot
  size: number                            // Section size in bytes
  color: string                           // Hex color for visualization
  description: string                     // Section purpose
}

// Known sections:
// - 'header'         : Version, map ID, padding
// - 'gaItems'        : Inventory items (variable size)
// - 'playerGameData' : Character stats
// - 'equipData'      : Equipment slot indices
// - 'chrAsm'         : Equipped item IDs
// - 'eventFlags'     : Progress flags (~1.8MB)

interface DataChunk {
  id: string                              // Unique chunk ID
  name: string                            // Display name (may include value)
  offset: number                          // Absolute offset in slot
  size: number                            // Size in bytes
  sectionId: string                       // Parent section ID
  type: ChunkType                         // Chunk classification
  value?: number | string                 // Parsed value
  rawHex?: string                         // Additional hex display info
}

type ChunkType =
  | 'stat'        // Character statistics
  | 'field'       // Generic data field
  | 'equipment'   // Equipped item reference
  | 'item'        // Inventory item
  | 'flag'        // Event flag
```

---

## 6. User Progress Domain

```typescript
// ═══════════════════════════════════════════════════════════════════
// User Progress Schemas (Local Storage)
// Source: Zustand store with localStorage persistence
// ═══════════════════════════════════════════════════════════════════

interface UserProgress {
  // Manual completions (user-marked)
  completedLocations: Set<string>         // MapLocation.id

  // Auto-completions (from event flags)
  autoCompletedLocations: Set<string>     // MapLocation.id

  // Visual markers (notes/pins)
  markedLocations: Set<string>            // MapLocation.id

  // Completion metadata
  completionMetadata: Map<string, CompletionMetadata>
}

interface CompletionMetadata {
  source: 'manual' | 'event_flag'         // How it was marked complete
  completed_at: number                    // Unix timestamp
  event_flag?: number                     // Associated flag ID
}

interface FilterState {
  visibleCategories: Record<number, boolean>  // categoryId → visible
  searchQuery: string
  showCompleted: boolean
  showIncomplete: boolean
}
```

---

## 7. Service Cache Schemas

```typescript
// ═══════════════════════════════════════════════════════════════════
// Service Layer Caching Schemas
// Source: DataProvider base class, IndexedDB
// ═══════════════════════════════════════════════════════════════════

// In-memory cache entry (DataProvider)
interface CacheEntry<T> {
  data: T
  timestamp: number                       // Cache time
  ttl: number                             // Time-to-live (default: 5 min)
}

// IndexedDB cache entry (SaveFileCacheService)
interface CachedSaveFile {
  panelId: 'left' | 'right'               // Which hex viewer panel
  fileName: string                        // Original file name
  slotIndex: number                       // Selected character slot
  data: ArrayBuffer                       // Raw save data
  timestamp: number                       // Cache time
}

// Service indexes for O(1) lookups
interface ServiceIndexes {
  // POIDataService
  eventFlagIndex: Map<number, MapLocation>      // flagId → location
  idIndex: Map<string, MapLocation>             // locationId → location
  categoryIndex: Map<string, MapLocation[]>     // categoryId → locations

  // EventFlagDataService
  flagIdIndex: Map<number, GameFileEventFlag>   // flagId → flag
  categoryIndex: Map<string, GameFileEventFlag[]>
}
```

---

## Schema Relationships Diagram

```
┌─────────────────┐       ┌──────────────────┐
│   MapLocation   │       │ GameFileEventFlag│
├─────────────────┤       ├──────────────────┤
│ id              │       │ flagId (PK)      │
│ eventFlag (FK)──┼──────►│ category         │
│ categoryId (FK) │       │ items[]          │
│ latitude        │       │ position         │
│ longitude       │       │ mapId            │
└────────┬────────┘       └──────────────────┘
         │
         │ FK
         ▼
┌─────────────────┐       ┌──────────────────┐
│    Category     │       │   ZoneBoundary   │
├─────────────────┤       ├──────────────────┤
│ id (PK)         │       │ id (PK)          │
│ title           │       │ coordinates[]    │
│ icon            │       │ levelMin/Max     │
│ color           │       │ progressionId    │
└─────────────────┘       └──────────────────┘

┌─────────────────┐       ┌──────────────────┐
│  SaveFileData   │       │     SlotData     │
├─────────────────┤       ├──────────────────┤
│ character       │       │ buffer           │
│ equipment       │       │ sections[]       │
│ inventory       │       │ chunks[]         │
│ event_flags{}───┼──────►│ eventFlagsOffset │
│ boss_defeats{}  │       └──────────────────┘
└─────────────────┘
```

---

## Data Flow

```
External Sources (JSON, WebSocket, File Upload)
         │
    ┌────┴────┬─────────────┬───────────────┐
    ▼         ▼             ▼               ▼
POIData  EventFlag  CharacterData  ZoneData  SaveFileData
Service   Service     Service      Service     Service
    │         │             │           │           │
    └─────────┴─────┬───────┴───────────┴───────────┘
                    ▼
           UnifiedViewModel
        (Aggregates all services)
                    │
                    ▼
            React Components
```

---

## Service Summary

| Service | Data Source | Key Entities | Lookup Speed |
|---------|-------------|--------------|--------------|
| POIDataService | `/data/merged-pois.json` | MapLocation, Category | O(1) via indexes |
| EventFlagDataService | `/data/event-flags-gamefiles.json` | GameFileEventFlag | O(1) by flagId |
| CharacterDataService | WebSocket + fallback JSON | SaveFileData, Equipment | O(1) via Set |
| ZoneDataService | Bundled JSON | ZoneBoundary | O(n) filter |
| SaveFileDataService | File upload (.sl2) | SlotData, DataChunk | O(1) cache |
| SaveFileCacheService | IndexedDB | CachedSaveFile | O(1) |
