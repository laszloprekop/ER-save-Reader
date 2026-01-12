#!/usr/bin/env python3
"""
Find the true EventFlags offset by looking for specific patterns.

The EventFlags section should:
1. Be ~1.8MB in size
2. Mostly contain zeros
3. Have specific patterns at known flag offsets

Key insight: The validation approach may be finding false positives because
byte 2725 happens to be 0xFF in multiple locations.

Let's try a different approach: search for a unique pattern that only exists
in the EventFlags region.
"""

import sys
import struct
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SLOT_SIZE, EVENT_FLAGS_SIZE

snapshot_dir = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")

# Read a progressed save that has more flags set
save_path = snapshot_dir / "ER0000.sl2 Confessor - level 93 snapshot"
# Fallback to another save if that doesn't exist
if not save_path.exists():
    save_path = snapshot_dir / "ER0000.sl2 Wretch - 35 at south of Wayward cellar sarchophagi, picked up Golden Rune [1] and [3]"

print(f"Analyzing: {save_path.name}")

with open(save_path, 'rb') as f:
    data = f.read()

# Get slot 0 (Confessor) offset
entry_offset = 0x40 + (0 * 0x20) + 0x10
bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
slot_start = bnd4_offset + 16

slot_data = data[slot_start:slot_start + SLOT_SIZE]

print(f"Slot 0 starts at: 0x{slot_start:X}")

# Instead of validation flags, let's search for the EventFlags section
# by looking for a region that is:
# 1. Mostly zeros
# 2. Has occasional set bits
# 3. Is at least 100KB in size

print("\n--- Scanning for EventFlags-like regions ---")

def analyze_region(data, start, size):
    """Analyze a region and return statistics."""
    region = data[start:start + size]
    if len(region) < size:
        return None

    non_zero = sum(1 for b in region if b != 0)
    ff_count = sum(1 for b in region if b == 0xFF)
    sparse_score = 1.0 - (non_zero / len(region))

    return {
        'start': start,
        'non_zero': non_zero,
        'ff_count': ff_count,
        'sparse_score': sparse_score,
        'density': non_zero / len(region),
    }

# Scan the slot data in chunks
chunk_size = 100000  # 100KB chunks
best_candidates = []

for offset in range(0x10000, min(0x100000, len(slot_data) - chunk_size), 0x1000):
    stats = analyze_region(slot_data, offset, chunk_size)
    if stats:
        # We want sparse but NOT empty (has some flags set)
        if 0.98 < stats['sparse_score'] < 1.0 and stats['non_zero'] > 100:
            best_candidates.append(stats)

# Sort by sparseness (most sparse first)
best_candidates.sort(key=lambda x: x['sparse_score'], reverse=True)

print("Top sparse regions (100KB chunks):")
for stats in best_candidates[:10]:
    print(f"  0x{stats['start']:05X}: {stats['density']*100:.3f}% non-zero, "
          f"{stats['non_zero']} non-zero bytes, {stats['ff_count']} 0xFF bytes")

if best_candidates:
    best_start = best_candidates[0]['start']
    print(f"\nBest candidate starts at: 0x{best_start:X}")

    # Now let's examine this region more closely
    ef_candidate = slot_data[best_start:best_start + EVENT_FLAGS_SIZE]

    # Check validation flag offsets in this region
    if len(ef_candidate) > 3300:
        b2625 = ef_candidate[2625]
        b2725 = ef_candidate[2725]
        b3250 = ef_candidate[3250]
        b3262 = ef_candidate[3262]

        print(f"\nAt candidate offset 0x{best_start:X}:")
        print(f"  byte[2625] = 0x{b2625:02X}")
        print(f"  byte[2725] = 0x{b2725:02X} ({bin(b2725)})")
        print(f"  byte[3250] = 0x{b3250:02X}")
        print(f"  byte[3262] = 0x{b3262:02X} ({bin(b3262)})")

        # Check if validation flags would match
        f71800 = bool(b2725 & 0x80)
        f71801 = bool(b2725 & 0x40)
        f76100 = bool(b3262 & 0x08)
        f76101 = bool(b3262 & 0x04)

        print(f"\n  Validation flag checks:")
        print(f"    71800 (bit 7): {f71800}")
        print(f"    71801 (bit 6): {f71801}")
        print(f"    76100 (bit 3): {f76100}")
        print(f"    76101 (bit 2): {f76101}")

# Now let's try the current save file (more progress)
print("\n" + "=" * 70)
current_save = Path("/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2")
print(f"Analyzing current save: {current_save.name}")

with open(current_save, 'rb') as f:
    current_data = f.read()

# Check slot 0
entry_offset = 0x40 + (0 * 0x20) + 0x10
bnd4_offset = struct.unpack_from('<I', current_data, entry_offset)[0]
slot_start = bnd4_offset + 16
current_slot = current_data[slot_start:slot_start + SLOT_SIZE]

print(f"Slot 0 starts at: 0x{slot_start:X}")

best_candidates = []
for offset in range(0x10000, min(0x100000, len(current_slot) - chunk_size), 0x1000):
    stats = analyze_region(current_slot, offset, chunk_size)
    if stats:
        # We want sparse but NOT empty
        if 0.98 < stats['sparse_score'] < 1.0 and stats['non_zero'] > 100:
            best_candidates.append(stats)

best_candidates.sort(key=lambda x: x['sparse_score'], reverse=True)

print("\nTop sparse regions in current save:")
for stats in best_candidates[:5]:
    print(f"  0x{stats['start']:05X}: {stats['density']*100:.3f}% non-zero, {stats['non_zero']} non-zero")

if best_candidates:
    best_start = best_candidates[0]['start']
    ef_candidate = current_slot[best_start:best_start + EVENT_FLAGS_SIZE]

    if len(ef_candidate) > 3300:
        b2725 = ef_candidate[2725]
        b3262 = ef_candidate[3262]

        print(f"\nAt offset 0x{best_start:X}:")
        print(f"  byte[2725] = 0x{b2725:02X} (flags: 71800={bool(b2725 & 0x80)}, 71801={bool(b2725 & 0x40)})")
        print(f"  byte[3262] = 0x{b3262:02X} (flags: 76100={bool(b3262 & 0x08)}, 76101={bool(b3262 & 0x04)})")
