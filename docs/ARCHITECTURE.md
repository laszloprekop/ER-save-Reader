# Verification System Architecture

This document describes the structure and design principles of the event flag verification system.

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: SUPERSEDED — documents the pre-reset Python lab, not the live system.** This describes the `ground_truth_offsets.json → ground_truth_loader.py → scripts/verification/` Python architecture. That Python code was **removed in migration step 5** (2026-07-21; distilled in `docs/archive/PYTHON-LAB.md`). It is **not** how flags are resolved today.
> - **Claims**: a "single source of truth hierarchy" rooted in `ground_truth_offsets.json` and a Python verification module.
> - **Evidence**: none new — a design description of the old lab.
> - **Methodology**: the live methodology is the knowledge pipeline (`er-save-editor knowledge run` → claims store) plus the one reference resolver in `crates/wasm-event-flags` (ADR-0005), consumed by both the app and elden-map. See `docs/WASM-EVENT-FLAGS.md`, `CONTEXT.md`, `docs/adr/`.
> - **Obsolete**: `ground_truth_offsets.json` is FROZEN read-only (ADR-0006), not the source of truth; `flag_formulas.py` is deprecated; the Python verification scripts were removed in step 5 (`docs/archive/PYTHON-LAB.md`). For today's architecture read the ADRs and `CONTEXT.md`, not this file.

---

## Single Source of Truth Hierarchy

```
ground_truth_offsets.json    ← All verified offsets, formulas, flag data
    ↓
ground_truth_loader.py       ← Python API to access ground_truth
    ↓
constants.py                 ← Only save file structure (not in ground_truth)
    ↓
utils.py                     ← Unified API combining both
    ↓
verification scripts         ← Import from utils.py
```

---

## Key Principles

### 1. No Duplication of Verification Data

Verification data (validation flags, block bases, tile config) must come from a single source:

| Data Type | Source | Do NOT duplicate in |
|-----------|--------|---------------------|
| Validation flags (71800, 76100, etc.) | `ground_truth_loader.get_validation_flags()` | constants.py, scripts |
| Block/tile/dungeon bases | `ground_truth_offsets.json` via loader | constants.py, scripts |
| Tile formula config | `ground_truth_loader.get_tile_config()` | scripts |

### 2. constants.py Contains ONLY Save File Structure

These are values NOT stored in `ground_truth_offsets.json`:

```python
# Save file structure
SLOT_0_OFFSET = 0x310          # Header size before slot data
SLOT_SIZE = 0x280010           # Size of each character slot
SLOT_COUNT = 10                # Maximum character slots

# Event flags section bounds
EVENT_FLAGS_SIZE = 0x1BF99F    # 1,833,375 bytes
EVENT_FLAGS_SEARCH_MIN = 0x10000
EVENT_FLAGS_SEARCH_MAX = 0x20000

# File paths
DEFAULT_SAVE_DIR = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files"
```

### 3. Methodology Documents

| Document | Purpose |
|----------|---------|
| `discovery-verification-cycle.md` | How to discover and verify flags |
| `CORROBORATION-SYSTEM.md` | Dual-formula validation |
| `CASE-VERIFICATION-GUIDE.md` | Case-based verification system |
| `EVENT-FLAG-GEOGRAPHY.md` | Flag ranges and formats |
| `SAVE_FILE_GROUND_TRUTH.md` | Verified flag positions |
| `WASM-EVENT-FLAGS.md` | Shared detection algorithm |
| This file (`ARCHITECTURE.md`) | System structure |

---

## Module Structure

### scripts/verification/

```
scripts/verification/
├── __init__.py              # Exports shared modules
├── constants.py             # Save file structure constants only
├── utils.py                 # Shared utility functions
├── ground_truth_loader.py   # Loads from ground_truth_offsets.json
├── save_parser.py           # Full save file parsing
├── flag_schema.py           # Schema definition and allocation bitmap
├── case_manager.py          # Case-based verification system
├── case_cli.py              # CLI for case operations
├── verification_data.py     # Data structures
├── diff_analyzer.py         # Save comparison
│
├── verify_*.py              # Verification scripts
├── probe_*.py               # Probing/discovery scripts
├── check_*.py               # Quick check scripts
└── archive/                 # Superseded/historical scripts
    └── flag_formulas.py     # DEPRECATED - use ground_truth_loader
```

### Import Pattern

All verification scripts should use this pattern:

```python
#!/usr/bin/env python3
"""Script description."""

from pathlib import Path
import sys

# Add verification module to path if needed
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from scripts.verification.constants import (
    SLOT_0_OFFSET,
    SLOT_SIZE,
    EVENT_FLAGS_SIZE,
    DEFAULT_SAVE_DIR,
)
from scripts.verification.utils import (
    read_slot_data,
    detect_event_flags_start,
    extract_event_flags,
    check_flag,
)
from scripts.verification.ground_truth_loader import (
    load_block_bases,
    get_tile_config,
    calculate_block_offset,
    calculate_tile_offset,
    calculate_dungeon_offset,
)
```

---

## Function Reference

### ground_truth_loader.py

| Function | Returns | Description |
|----------|---------|-------------|
| `load_block_bases()` | `Dict[int, Dict]` | All block bases from ground_truth |
| `load_dungeon_bases()` | `Dict[int, Dict]` | All dungeon bases from ground_truth |
| `get_tile_config()` | `Dict` | Tile formula configuration |
| `get_validation_flags()` | `Dict[int, Tuple]` | Validation flags for EF detection |
| `get_block_base(flag_id)` | `Optional[int]` | Base offset for a block flag |
| `get_dungeon_base(map_area)` | `Optional[int]` | Base offset for a dungeon area |
| `calculate_block_offset(flag_id)` | `Optional[Tuple[int, int]]` | (byte_offset, bit_position) |
| `calculate_tile_offset(flag_id)` | `Optional[Tuple[int, int]]` | (byte_offset, bit_position) |
| `calculate_dungeon_offset(flag_id)` | `Optional[Tuple[int, int]]` | (byte_offset, bit_position) |
| `get_player_coords_config()` | `Dict[str, Any]` | Player coordinate extraction parameters |

### utils.py

| Function | Returns | Description |
|----------|---------|-------------|
| `read_slot_data(save_path, slot_index)` | `bytes` | Raw slot data from save file |
| `detect_event_flags_start(slot_data)` | `Optional[int]` | Event flags offset in slot |
| `extract_event_flags(slot_data)` | `bytes` | Event flags section |
| `check_flag(event_flags, flag_id)` | `Tuple[bool, int, int]` | (is_set, offset, bit) |
| `is_0xff_padding(event_flags, offset)` | `bool` | True if region is 0xFF padding |
| `multi_slot_differential(...)` | `List[Dict]` | Compare flags between slots |

### flag_schema.py

Schema-based allocation detection for handling sparse flag allocation.

| Class/Function | Description |
|----------------|-------------|
| `BlockSchema` | Defines known flag positions for a block |
| `AllocationBitmap` | Result showing which flags are trackable |
| `FlagDefinition` | A flag's position as defined in the schema |
| `AllocationEntry` | Probe result for a single flag |
| `probe_block()` | Convenience function to probe a block |

**Key Concepts:**

- **Schema**: Predefined structure mapping flag IDs to expected byte offsets
- **Allocation Bitmap**: Result showing which positions have real data vs padding
- **Sparse Allocation**: When the game only allocates memory for flags actually used

**Usage:**

```python
from scripts.verification.flag_schema import BlockSchema, AllocationBitmap

# Create schema for block 520000
schema = BlockSchema(block_start=520000, base_offset=1341)
schema.load_flags_from_extracted('scripts/extracted_event_flags.json')

# Probe save to generate allocation bitmap
bitmap: AllocationBitmap = schema.probe_allocation(save_path, slots=[0,1,2,3,4])

# Query the bitmap
trackable = bitmap.get_trackable_flags()      # Flags that CAN be verified
untrackable = bitmap.get_untrackable_flags()  # Flags in sparse gaps
is_ok = bitmap.is_trackable(520000)           # True
is_ok = bitmap.is_trackable(520210)           # False (sparse gap)

# Get allocation boundaries
boundaries = schema.get_allocation_boundaries(save_path)
# Returns: [(520000, 520059, 'ALLOCATED'), (520060, 520089, 'UNALLOCATED'), ...]
```

**CLI:**

```bash
python scripts/verification/flag_schema.py --block 520000 --base 1341 \
    --save "/path/to/save.sl2" --boundaries --json
```

---

## Script Migration Checklist

When updating a verification script to use shared modules:

- [ ] Remove local `VALIDATION_FLAGS` → use `get_validation_flags()`
- [ ] Remove local `SLOT_0_OFFSET`, `SLOT_SIZE` → import from constants
- [ ] Remove local `detect_event_flags_start()` → import from utils
- [ ] Remove local `read_slot_data()` → import from utils
- [ ] Remove local `check_flag()` → import from utils
- [ ] Remove hardcoded block bases → use `ground_truth_loader`
- [ ] Test that script still produces correct output

---

## Verification Workflow

### Quick Verification (Single Flag)

```python
from scripts.verification.utils import read_slot_data, detect_event_flags_start, check_flag

slot_data = read_slot_data("/path/to/save", slot_index=0)
ef_start = detect_event_flags_start(slot_data)
event_flags = slot_data[ef_start:]

is_set, offset, bit = check_flag(event_flags, 71800)
print(f"Flag 71800: {'SET' if is_set else 'UNSET'} at offset {offset}, bit {bit}")
```

### Multi-Slot Differential (Gold Standard)

```python
from scripts.verification.utils import read_slot_data, detect_event_flags_start, multi_slot_differential

# Load progressed slot (S0) and early-game slot (S1)
s0_data = read_slot_data(save_path, 0)
s1_data = read_slot_data(save_path, 1)

ef0 = s0_data[detect_event_flags_start(s0_data):]
ef1 = s1_data[detect_event_flags_start(s1_data):]

# Check flags - progressed should be SET, early-game should be UNSET
flags_to_check = [(71800, "Cave of Knowledge"), (76100, "The First Step")]
results = multi_slot_differential(ef0, ef1, flags_to_check)

for result in results:
    print(f"{result['name']}: S0={result['s0_set']}, S1={result['s1_set']} -> {result['status']}")
```

---

## Adding New Block Bases

1. **Discover candidate offset** using probe scripts
2. **Verify via multi-slot differential** (Phase 3 in methodology)
3. **Check for 0xFF padding** (Phase 4)
4. **Add to `ground_truth_offsets.json`**:

```json
{
  "formulas": {
    "block_bases": {
      "71000": {
        "base_offset": 9315,
        "block_size": 100,
        "status": "verified",
        "notes": "Stormveil graces - verified 2026-01-22 via multi-slot diff"
      }
    }
  }
}
```

5. **Test with verification script** to confirm

---

## Archive Policy

Scripts should be moved to `scripts/verification/archive/` when:

- Superseded by a more comprehensive script
- One-time investigation completed
- Uses deprecated methods (hardcoded offsets)

Archive scripts are preserved for historical reference but should not be used for active verification.
