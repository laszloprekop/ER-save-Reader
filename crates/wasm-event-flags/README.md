# wasm-event-flags

WebAssembly module for Elden Ring EventFlags offset detection.

## Purpose

This crate is the **single source of truth** for detecting the EventFlags section offset within character slot data. It is used by:

- **ER-save-Editor** - Native Rust (via Cargo dependency)
- **elden-map** - WebAssembly (compiled with wasm-pack)

## Building

### Native (for ER-save-Editor)

```bash
# From workspace root
cargo build -p wasm-event-flags
```

### WASM (for elden-map)

```bash
# From this directory
wasm-pack build --target web --out-dir ../../../elden-map/wasm-event-flags
```

## API

### Rust (Native)

```rust
use wasm_event_flags::detect_event_flags_offset_impl;

let result = detect_event_flags_offset_impl(&slot_data);
println!("EventFlags at offset: {}", result.offset);
```

### JavaScript (WASM)

```javascript
import init, { detect_event_flags_offset } from './wasm_event_flags.js'

await init()
const result = detect_event_flags_offset(slotData)
console.log('EventFlags at offset:', result.offset)
result.free()
```

## Algorithm

1. Search from `0x12000` for offsets where all tier-1 grace flags are SET
2. Among candidates, prefer offsets where late-game graces are NOT SET
3. Return first perfect match, or best scoring candidate

## Documentation

See `../../docs/WASM-EVENT-FLAGS.md` for detailed documentation.
