#!/usr/bin/env python3
"""
Debug the validation flag detection to understand why it finds certain offsets.
"""

import sys
import struct
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import (
    VALIDATION_FLAGS, EVENT_FLAGS_SEARCH_MIN, EVENT_FLAGS_SEARCH_MAX,
    EVENT_FLAGS_SIZE, SLOT_SIZE
)

current_save = Path("/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2")

with open(current_save, 'rb') as f:
    data = f.read()

# Get slot 0 offset
entry_offset = 0x40 + (0 * 0x20) + 0x10
bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
slot_start = bnd4_offset + 16

slot_data = data[slot_start:slot_start + SLOT_SIZE]

print("Validation flags used in detection:")
for flag_id, byte_off, bit_pos, name in VALIDATION_FLAGS:
    print(f"  {flag_id} ({name}): byte {byte_off}, bit {bit_pos}")
    print(f"    Check: slot_data[test_offset + {byte_off}] & (1 << {bit_pos})")

print(f"\nSearch range: 0x{EVENT_FLAGS_SEARCH_MIN:X} to 0x{EVENT_FLAGS_SEARCH_MAX:X}")

# Manually search for validation matches like the parser does
print("\n--- Searching for validation matches ---")

best_offset = 0
best_score = 0
all_scores = []

for test_offset in range(EVENT_FLAGS_SEARCH_MIN, min(EVENT_FLAGS_SEARCH_MAX, len(slot_data) - EVENT_FLAGS_SIZE), 4):
    score = 0
    flags_matched = []
    for flag_id, byte_off, bit_pos, name in VALIDATION_FLAGS:
        abs_pos = test_offset + byte_off
        if abs_pos < len(slot_data):
            byte_val = slot_data[abs_pos]
            is_set = (byte_val & (1 << bit_pos)) != 0
            if is_set:
                score += 1
                flags_matched.append(name)

    if score > 0:
        all_scores.append((test_offset, score, flags_matched))

    if score > best_score:
        best_score = score
        best_offset = test_offset

        if best_score == len(VALIDATION_FLAGS):
            break

print(f"\nBest match: offset 0x{best_offset:X} with score {best_score}/4")

# Show all offsets with matches
print(f"\nAll offsets with matches (score > 0): {len(all_scores)}")
for offset, score, flags in sorted(all_scores, key=lambda x: -x[1])[:20]:
    print(f"  0x{offset:05X}: score={score}, matched={flags}")

# Examine the best offset in detail
print(f"\n--- Examining best offset 0x{best_offset:X} ---")
for flag_id, byte_off, bit_pos, name in VALIDATION_FLAGS:
    abs_pos = best_offset + byte_off
    byte_val = slot_data[abs_pos]
    is_set = (byte_val & (1 << bit_pos)) != 0
    print(f"  {name} ({flag_id}):")
    print(f"    Position: test_offset (0x{best_offset:X}) + {byte_off} = 0x{abs_pos:X}")
    print(f"    Byte value: 0x{byte_val:02X} ({bin(byte_val)})")
    print(f"    Bit {bit_pos} check: {byte_val} & {1 << bit_pos} = {byte_val & (1 << bit_pos)} -> {is_set}")

# Let's also examine data around the best offset
print(f"\n--- Data around best offset 0x{best_offset:X} ---")
sample_start = best_offset
sample = slot_data[sample_start:sample_start + 100]
print(f"First 100 bytes at EventFlags: {' '.join(f'{b:02X}' for b in sample[:50])}")
print(f"                              {' '.join(f'{b:02X}' for b in sample[50:100])}")

# Non-zero bytes in first 10KB
ef_start = slot_data[best_offset:best_offset + 10000]
non_zero = sum(1 for b in ef_start if b != 0)
print(f"\nNon-zero bytes in first 10KB: {non_zero} ({100*non_zero/10000:.2f}%)")

# Check what's BEFORE the detected offset (to understand the structure)
print(f"\n--- Data BEFORE detected EventFlags ---")
pre_start = best_offset - 100
pre_data = slot_data[pre_start:best_offset]
print(f"Last 100 bytes before EventFlags at 0x{pre_start:X}:")
print(f"  {' '.join(f'{b:02X}' for b in pre_data[:50])}")
print(f"  {' '.join(f'{b:02X}' for b in pre_data[50:])}")
