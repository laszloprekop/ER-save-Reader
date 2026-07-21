# WASM Shared Detection Module

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: MOSTLY CURRENT — the resolver is the live path; one section is stale.** This is the doc for today's detection/resolution crate. **Exception:** the "Flag Offset Resolution" section describes `get_sub_block_bases()` / `get_main_block_bases()`, which were **deleted in ADR-0008** (2026-07-20) along with every static base table — the crate now holds no flag base tables. Ignore that section's function names.
> - **Claims**: the crate is the single reference implementation (ADR-0005) for EF detection and player-coord extraction, shared with elden-map via WASM.
> - **Evidence**: conformance fixtures (`crates/wasm-event-flags/tests/`) define the coordinate convention (ADR-0003).
> - **Methodology**: detection + per-save family resolution; positions are resolved, not looked up in a static table.
> - **Obsolete**: "Flag Offset Resolution" (sub/main-block base lookup) — those functions and the block base tables are gone (ADR-0008); the static-offset exports were removed and are now banned by `tests/export_shape_conformance.rs`. "Constants … sourced from `ground_truth_offsets.json`" reflects the frozen store (ADR-0006), not new truth.

## Overview

The `wasm-event-flags` crate provides the **single source of truth** for both EventFlags offset detection and player coordinate extraction. This ensures both ER-save-Editor (native Rust) and elden-map (via WebAssembly) use the **exact same algorithms**.

## Why This Matters

The EventFlags section in Elden Ring save files contains ~1.8MB of bit flags tracking game progress (graces discovered, bosses defeated, items collected, etc.). However, its position within slot data is **not fixed** - it varies per character based on inventory size and other factors.

Previously, ER-save-Editor and elden-map had separate implementations of the detection algorithm. This led to:
- **Inconsistent results** - different offsets found for the same save file
- **Maintenance burden** - bug fixes needed in two codebases
- **Drift risk** - implementations could diverge over time

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│           wasm-event-flags (Rust crate)                     │
│                                                             │
│  Location: crates/wasm-event-flags/                         │
│                                                             │
│  Event Flags:                                               │
│  • detect_event_flags_offset() - WASM entry point           │
│  • detect_event_flags_offset_impl() - native Rust entry     │
│  • POSITIVE/NEGATIVE_VALIDATION_FLAGS                       │
│  • SEARCH_START, EVENT_FLAGS_SIZE                           │
│                                                             │
│  Player Position:                                           │
│  • extract_player_position() - WASM entry point             │
│  • extract_player_position_impl() - native Rust entry       │
│  • PlayerPositionResult (x,y,z,x2,y2,z2,facing,map_id)     │
│  • PLAYER_COORDS_SEARCH_START/END                           │
└────────────────────┬────────────────────┬───────────────────┘
                     │                    │
          ┌──────────▼──────────┐  ┌──────▼──────────────────┐
          │   ER-save-Editor    │  │      elden-map          │
          │      (Native)       │  │        (WASM)           │
          │                     │  │                         │
          │ Uses Rust crate     │  │ Loads wasm-event-flags  │
          │ directly via Cargo  │  │ package at startup      │
          └─────────────────────┘  └─────────────────────────┘
```

## Detection Algorithm

The algorithm uses a two-phase validation approach:

### Phase 1: Positive Validation
Search for offsets where **ALL** tier-1 grace flags are SET:

| Flag ID | Byte Offset | Bit | Name | Tier |
|---------|-------------|-----|------|------|
| 71800 | 2725 | 7 | Cave of Knowledge | 1 |
| 71801 | 2725 | 6 | Stranded Graveyard | 1 |
| 76100 | 3262 | 3 | The First Step | 1 |
| 76101 | 3262 | 2 | Church of Elleh | 1 |
| 76102 | 3262 | 1 | Gatefront Ruins | 2 |
| 76104 | 3263 | 7 | Agheel Lake South | 2 |
| 76106 | 3263 | 5 | Church of Dragon Communion | 2 |

### Phase 2: Negative Validation
Among candidates, prefer offsets where late-game graces are **NOT SET**:

| Flag ID | Byte Offset | Bit | Name |
|---------|-------------|-----|------|
| 76223 | 3277 | 0 | Fortified Manor, First Floor |
| 76224 | 3278 | 7 | East Capital Rampart |
| 76225 | 3278 | 6 | Divine Bridge |
| 76300 | 3287 | 3 | Zamor Ruins |
| 76301 | 3287 | 2 | Ancient Snow Valley Ruins |
| 76350 | 3293 | 5 | Haligtree Town |

### Selection Criteria
1. Return **FIRST** offset with ALL tier-1 positive flags SET and ALL negative flags UNSET
2. If no perfect match, prefer: highest negative score → highest positive score → lowest offset

## Usage in ER-save-Editor

The main application uses the shared crate through `src/save/common/event_flags_detection.rs`:

```rust
// The module delegates to the shared crate
use wasm_event_flags::{
    POSITIVE_VALIDATION_FLAGS,
    NEGATIVE_VALIDATION_FLAGS,
    SEARCH_START,
    EVENT_FLAGS_SIZE,
};

// Detection function wraps the shared implementation
pub fn detect_event_flags_offset(slot_data: &[u8], _search_start: usize) -> EventFlagsDetectionResult {
    let result = wasm_event_flags::detect_event_flags_offset_impl(slot_data);
    // ... convert to local result type
}
```

## Building the WASM Package

After modifying the detection algorithm in `crates/wasm-event-flags/src/lib.rs`:

```bash
# From ER-save-Editor root
cd crates/wasm-event-flags

# Build WASM package for elden-map
wasm-pack build --target web --out-dir ../../../elden-map/wasm-event-flags

# The output includes:
# - wasm_event_flags_bg.wasm (26KB optimized)
# - wasm_event_flags.js (JS wrapper)
# - wasm_event_flags.d.ts (TypeScript definitions)
```

## Player Position Extraction

The crate also extracts player coordinates from slot data using a signature-based search. This consolidates three previously independent implementations (Rust, Python, TypeScript) into a single shared algorithm.

### Algorithm

1. Read header map_id from slot bytes 4-7
2. Search range `PLAYER_COORDS_SEARCH_START` to `PLAYER_COORDS_SEARCH_END` for map_id match
3. Validate mid-section pattern: 4B zeros + 4B facing_angle + 8B zeros + 1B (0x01)
4. Validate padding2: 16 bytes with >=8 zeros
5. Read f32 coordinates and facing angle
6. Select best candidate by padding quality

### Struct Layout (61 bytes)

```
12B coords (x,y,z as f32 LE) + 4B map_id + 17B mid_section + 12B coords2 + 16B pad2
```

### Facing Angle

The mid-section contains a Y-axis rotation (yaw/heading) in radians [-pi, pi] at bytes [4:8] as f32 little-endian. Verified across 7 test saves.

### Result: `PlayerPositionResult`

| Field | Type | Description |
|-------|------|-------------|
| `x, y, z` | f32 | Primary coordinates |
| `x2, y2, z2` | f32 | Secondary coordinates (usually same as primary) |
| `facing_angle` | f32 | Y-axis rotation in radians [-pi, pi] |
| `map_id_0..3` | u8 | Map ID bytes (individual fields; wasm_bindgen limitation) |
| `valid` | bool | Whether extraction succeeded |
| `offset` | usize | Byte offset where coords were found |

## Constants

### Event Flags Detection

| Constant | Value | Description |
|----------|-------|-------------|
| `SEARCH_START` | 0x12000 (73,728) | Byte offset to start searching |
| `MAX_SEARCH_RANGE` | 200,000 | Maximum bytes to search |
| `EVENT_FLAGS_SIZE` | 0x1BF99F (1,833,375) | Size of EventFlags section |

### Player Position Extraction

| Constant | Value | Description |
|----------|-------|-------------|
| `PLAYER_COORDS_SEARCH_START` | 0x1D0000 (1,900,544) | Start of search range |
| `PLAYER_COORDS_SEARCH_END` | 0x280000 (2,621,440) | End of search range |
| `PLAYER_COORDS_STRUCT_SIZE` | 61 | Total struct size in bytes |
| `MID_SECTION_SIZE` | 17 | Mid-section between map_id and coords2 |
| `COORD_RANGE_MAX` | 10,000.0 | Maximum valid coordinate value |

All constants are sourced from `ground_truth_offsets.json` (`player_coords_extraction` section).

### Flag Offset Resolution

Block flags (60,000-99,999) use a two-tier lookup in `calculate_simple_flag_offset()`:

1. **Sub-block bases** (`get_sub_block_bases()`, 100-granularity) — checked first for overrides
2. **Main-block bases** (`get_main_block_bases()`, 1000-granularity) — fallback

This split allows key `71000` to map to base `9315` for Stormveil graces (71000-71099) at the sub-block level, while flags 71100-71799 fall through to the main-block base of `2625` for other dungeon graces. See [Event Flag Geography](EVENT-FLAG-GEOGRAPHY.md) for the full block table.

## Testing

```bash
# Run Rust tests
cargo test -p wasm-event-flags

# Build main application (uses shared crate)
cargo build
```

## File Structure

```
crates/wasm-event-flags/
├── Cargo.toml          # Crate manifest (cdylib + rlib)
└── src/
    └── lib.rs          # Detection algorithm implementation
```

## Related Documentation

- [Event Flag Geography](EVENT-FLAG-GEOGRAPHY.md) - Flag ID structure and formulas
- [Discovery Verification Cycle](discovery-verification-cycle.md) - Offset verification methodology
