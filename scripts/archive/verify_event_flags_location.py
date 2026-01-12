#!/usr/bin/env python3
"""
Verify EventFlags offset detection by examining the data around the detected location.

The EventFlags section should be a 1.8MB region of mostly 0x00 bytes with some flags set.
If we're detecting the wrong offset, we might be reading structured data instead.
"""

import sys
import struct
from pathlib import Path
from collections import Counter

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser, SLOT_SIZE, EVENT_FLAGS_SIZE

snapshot_dir = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")
parser = SaveParser()

# Read a save file directly to examine its structure
save_path = snapshot_dir / "ER0000.sl2 Wretch - 00 freshly created"

with open(save_path, 'rb') as f:
    data = f.read()

# Get BND4 slot offset for slot 1 (Wretch)
entry_offset = 0x40 + (1 * 0x20) + 0x10
bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
slot_start = bnd4_offset + 16  # Skip checksum

print(f"Save file: {save_path.name}")
print(f"Slot 1 BND4 offset: 0x{bnd4_offset:X}")
print(f"Slot 1 data start (after checksum): 0x{slot_start:X}")

slot_data = data[slot_start:slot_start + SLOT_SIZE]

# Check slot header
version = struct.unpack_from('<I', slot_data, 0)[0]
map_id = struct.unpack_from('<I', slot_data, 4)[0]
gaitem_count = struct.unpack_from('<I', slot_data, 0x20)[0]

print(f"\nSlot header:")
print(f"  Version: {version} (0x{version:X})")
print(f"  Map ID: {map_id} (0x{map_id:X})")
print(f"  GaItem count: {gaitem_count}")

# Parse the save to get detected EventFlags offset
save = parser.parse(save_path, [1])
slot = save.slots[0]
detected_offset = slot.event_flags_offset

print(f"\nDetected EventFlags offset: 0x{detected_offset:X} ({detected_offset})")
print(f"EventFlags size: {EVENT_FLAGS_SIZE} bytes")

# Examine data around the detected offset
print(f"\n--- Examining data around detected offset ---")

# Check data density (non-zero bytes)
ef = slot_data[detected_offset:detected_offset + EVENT_FLAGS_SIZE]
non_zero_count = sum(1 for b in ef if b != 0)
print(f"Non-zero bytes in detected EventFlags: {non_zero_count} / {len(ef)} ({100*non_zero_count/len(ef):.2f}%)")

# Check byte value distribution
byte_counter = Counter(ef)
print(f"\nMost common byte values:")
for val, count in byte_counter.most_common(10):
    print(f"  0x{val:02X}: {count} times ({100*count/len(ef):.2f}%)")

# Sample data at specific offsets
print(f"\n--- Sample data at key offsets ---")
sample_offsets = [0, 2625, 2725, 3250, 3262, 3987, 5000, 10000]
for off in sample_offsets:
    if off + 10 < len(ef):
        sample = ef[off:off+10]
        print(f"  offset {off:5d}: {' '.join(f'{b:02X}' for b in sample)}")

# Now let's try the Rust offset (0x1a104) and see what that looks like
rust_offset = 0x1a104
print(f"\n--- Checking Rust offset 0x{rust_offset:X} ({rust_offset}) ---")
if rust_offset + 10000 < len(slot_data):
    rust_ef = slot_data[rust_offset:rust_offset + EVENT_FLAGS_SIZE]
    non_zero_rust = sum(1 for b in rust_ef if b != 0)
    print(f"Non-zero bytes at Rust offset: {non_zero_rust} / {len(rust_ef)} ({100*non_zero_rust/len(rust_ef):.2f}%)")

    print(f"Sample data at Rust offset:")
    for off in [0, 2625, 2725, 3250, 3262]:
        if off + 10 < len(rust_ef):
            sample = rust_ef[off:off+10]
            print(f"  offset {off:5d}: {' '.join(f'{b:02X}' for b in sample)}")

    # Check validation flags at Rust offset
    b2725_rust = rust_ef[2725] if len(rust_ef) > 2725 else 0
    b3262_rust = rust_ef[3262] if len(rust_ef) > 3262 else 0
    print(f"\nValidation flags at Rust offset:")
    print(f"  byte[2725] = 0x{b2725_rust:02X}")
    print(f"  byte[3262] = 0x{b3262_rust:02X}")
else:
    print(f"Rust offset is beyond slot data!")

# Try searching for a reasonable EventFlags offset by looking for sparse data
print(f"\n--- Searching for potential EventFlags regions ---")
# Look for regions where data is mostly zeros (as expected for flags)
best_regions = []
for test_offset in range(0x10000, min(0x30000, len(slot_data) - 10000), 0x100):
    test_region = slot_data[test_offset:test_offset + 10000]
    non_zero = sum(1 for b in test_region if b != 0)
    density = non_zero / 10000
    if density < 0.30:  # Less than 30% non-zero
        best_regions.append((test_offset, density, non_zero))

best_regions.sort(key=lambda x: x[1])
print("Sparsest regions (candidates for EventFlags):")
for offset, density, nz in best_regions[:10]:
    print(f"  0x{offset:05X}: {density*100:.1f}% non-zero ({nz} bytes)")
