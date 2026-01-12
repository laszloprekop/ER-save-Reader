#!/usr/bin/env python3
"""Discover the 73xxx block base by searching for known dungeon grace flags."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser

save_path = Path("/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2")
parser = SaveParser()

# Parse slot 5 (Sam - has more progression)
save = parser.parse(save_path, [5])
slot = save.slots[0]
ef = slot.event_flags

# Dungeon grace flags we might have discovered
# 73xxx = dungeon graces (catacombs, caves, tunnels, etc.)
test_flags = [
    (73000, "Murkwater Catacombs"),
    (73010, "Impaler's Catacombs"),
    (73011, "Deathtouched Catacombs"),  # This was in verification records
    (73020, "Tombsward Catacombs"),
    (73100, "Murkwater Cave"),
    (73110, "Groveside Cave"),
]

print("Searching for 73xxx flags in slot 5 EventFlags")
print(f"EventFlags offset: 0x{slot.event_flags_offset:X}")
print(f"EventFlags size: {len(ef)} bytes\n")

for flag_id, name in test_flags:
    expected_bit = 7 - (flag_id % 8)
    relative_flag = flag_id - 73000

    print(f"\n{flag_id} ({name}):")
    print(f"  Expected bit: {expected_bit}, relative: {relative_flag}")

# Cross-validate across a range of possible bases
print("\n" + "=" * 60)
print("CROSS-VALIDATION (searching bases 2500-3200)")
print("=" * 60)

# For dungeon graces, try a wider range
for test_base in range(2500, 3200):
    matches = 0
    match_details = []

    for flag_id, name in test_flags:
        relative = flag_id - 73000
        byte_off = test_base + (relative // 8)
        bit_pos = 7 - (flag_id % 8)

        if byte_off >= len(ef):
            continue

        is_set = (ef[byte_off] >> bit_pos) & 1
        if is_set:
            matches += 1
            match_details.append((flag_id, name, byte_off, bit_pos, ef[byte_off]))

    if matches >= 2:  # At least 2 flags match
        print(f"\nBase {test_base}: {matches}/{len(test_flags)} flags match")
        for flag_id, name, byte_off, bit_pos, byte_val in match_details:
            print(f"  {flag_id} ({name}): byte {byte_off}, bit {bit_pos}, value 0x{byte_val:02X}")

# Also check slot 0 which might have different dungeon progress
print("\n" + "=" * 60)
print("Checking slot 0 (Confessor - more progress)")
print("=" * 60)

save = parser.parse(save_path, [0])
slot = save.slots[0]
ef = slot.event_flags

for test_base in range(2500, 3200):
    matches = 0
    match_details = []

    for flag_id, name in test_flags:
        relative = flag_id - 73000
        byte_off = test_base + (relative // 8)
        bit_pos = 7 - (flag_id % 8)

        if byte_off >= len(ef):
            continue

        is_set = (ef[byte_off] >> bit_pos) & 1
        if is_set:
            matches += 1
            match_details.append((flag_id, name, byte_off, bit_pos, ef[byte_off]))

    if matches >= 2:
        print(f"\nBase {test_base}: {matches}/{len(test_flags)} flags match")
        for flag_id, name, byte_off, bit_pos, byte_val in match_details:
            print(f"  {flag_id} ({name}): byte {byte_off}, bit {bit_pos}, value 0x{byte_val:02X}")
