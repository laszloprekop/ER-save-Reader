#!/usr/bin/env python3
"""Check specific bytes for known flag locations."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser

snapshot_dir = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")
parser = SaveParser()

# Missionary Cookbook [4] = flag 67030
# Current documented base = 3987
# Expected: byte = 3987 + (67030 - 67000) // 8 = 3987 + 3 = 3990
# Expected bit = 7 - (67030 % 8) = 7 - 6 = 1

print("Checking Missionary Cookbook [4] flag location")
print("Flag: 67030")
print("Expected byte offset (with base=3987): 3990")
print("Expected bit: 1")

before_path = snapshot_dir / "ER0000.sl2 Confessor - 01 before Missionary Cookbok [4] pickup"
after_path = snapshot_dir / "ER0000.sl2 Confessor - 02 after Missionary Cookbok [4] picked up"

before_save = parser.parse(before_path, [0])
after_save = parser.parse(after_path, [0])

before_ef = before_save.slots[0].event_flags
after_ef = after_save.slots[0].event_flags

# Check byte 3990
before_3990 = before_ef[3990]
after_3990 = after_ef[3990]

print(f"\nByte 3990:")
print(f"  Before: 0x{before_3990:02X} ({bin(before_3990)})")
print(f"  After:  0x{after_3990:02X} ({bin(after_3990)})")

# Check if bit 1 changed from 0 to 1
before_bit1 = (before_3990 >> 1) & 1
after_bit1 = (after_3990 >> 1) & 1
print(f"  Bit 1: {before_bit1} -> {after_bit1}")

# Also check nearby bytes in case our formula is slightly off
print("\nBytes around 3990:")
for offset in range(3985, 4000):
    b_val = before_ef[offset]
    a_val = after_ef[offset]
    if b_val != a_val:
        print(f"  byte {offset}: 0x{b_val:02X} -> 0x{a_val:02X}")

# Check the first candidate location (base=3546)
# byte = 3546 + 3 = 3549
print("\nFirst candidate (base=3546, byte=3549):")
before_3549 = before_ef[3549]
after_3549 = after_ef[3549]
print(f"  Byte 3549: 0x{before_3549:02X} -> 0x{after_3549:02X}")
print(f"  Bit 1: {(before_3549 >> 1) & 1} -> {(after_3549 >> 1) & 1}")

# Let's look for the FIRST byte that changed in the 3000-4500 range (cookbook area)
print("\nFirst changes in cookbook area (3000-4500):")
count = 0
for offset in range(3000, 4500):
    b_val = before_ef[offset]
    a_val = after_ef[offset]
    if b_val != a_val:
        print(f"  byte {offset}: 0x{b_val:02X} -> 0x{a_val:02X}")
        count += 1
        if count >= 20:
            break
