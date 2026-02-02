# WASM Event Flags Detection

## Overview

The `wasm-event-flags` crate provides the **single source of truth** for EventFlags offset detection. This ensures both ER-save-Editor (native Rust) and elden-map (via WebAssembly) use the **exact same algorithm** to locate the EventFlags section within character slot data.

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
│  Exports:                                                   │
│  • detect_event_flags_offset() - WASM entry point           │
│  • detect_event_flags_offset_impl() - native Rust entry     │
│  • POSITIVE_VALIDATION_FLAGS                                │
│  • NEGATIVE_VALIDATION_FLAGS                                │
│  • SEARCH_START, EVENT_FLAGS_SIZE                           │
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

## Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `SEARCH_START` | 0x12000 (73,728) | Byte offset to start searching |
| `MAX_SEARCH_RANGE` | 200,000 | Maximum bytes to search |
| `EVENT_FLAGS_SIZE` | 0x1BF99F (1,833,375) | Size of EventFlags section |

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
