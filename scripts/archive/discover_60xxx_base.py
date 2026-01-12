#!/usr/bin/env python3
"""Discover the 60xxx block base by searching for known flags."""

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

# Flags we're looking for:
# 60130 = Whetstone Knife (should be set for any progressed character)
# 60220 = Tarnished's Furled Finger
# 60100 = Crafting Kit

test_flags = [
    (60100, "Crafting Kit"),
    (60130, "Whetstone Knife"),
    (60220, "Tarnished's Furled Finger"),
]

print("Searching for 60xxx flags in slot 5 EventFlags")
print(f"EventFlags offset: 0x{slot.event_flags_offset:X}")
print(f"EventFlags size: {len(ef)} bytes\n")

for flag_id, name in test_flags:
    # Calculate expected bit position
    expected_bit = 7 - (flag_id % 8)
    relative_flag = flag_id - 60000

    print(f"\n{flag_id} ({name}):")
    print(f"  Expected bit position: {expected_bit}")
    print(f"  Relative to block: {relative_flag}")
    print(f"  Searching for byte where bit {expected_bit} is set...")

    # Search in a reasonable range (0-5000 bytes)
    candidates = []
    for byte_off in range(0, 5000):
        byte_val = ef[byte_off]
        if (byte_val >> expected_bit) & 1:
            # This byte has our expected bit set
            # Calculate what base this would imply
            # byte_off = base + relative_flag // 8
            # base = byte_off - relative_flag // 8
            implied_base = byte_off - (relative_flag // 8)
            if 1000 <= implied_base <= 3000:  # Reasonable range
                candidates.append((byte_off, implied_base))

    print(f"  Found {len(candidates)} candidate bytes with bit {expected_bit} set")
    if candidates:
        # Group by implied base and count
        base_counts = {}
        for byte_off, base in candidates:
            base_counts[base] = base_counts.get(base, 0) + 1

        # Show top candidates
        sorted_bases = sorted(base_counts.items(), key=lambda x: -x[1])
        print("  Top implied bases (by frequency):")
        for base, count in sorted_bases[:5]:
            print(f"    base={base}: {count} occurrences")

# Now let's cross-validate - find bases that work for ALL test flags
print("\n" + "=" * 60)
print("CROSS-VALIDATION")
print("=" * 60)

# For each potential base, check if all flags would be set
for test_base in range(2500, 2700):
    all_match = True
    for flag_id, name in test_flags:
        relative = flag_id - 60000
        byte_off = test_base + (relative // 8)
        bit_pos = 7 - (flag_id % 8)

        if byte_off >= len(ef):
            all_match = False
            break

        is_set = (ef[byte_off] >> bit_pos) & 1
        if not is_set:
            all_match = False
            break

    if all_match:
        print(f"\nBase {test_base} works for ALL test flags!")
        for flag_id, name in test_flags:
            relative = flag_id - 60000
            byte_off = test_base + (relative // 8)
            bit_pos = 7 - (flag_id % 8)
            byte_val = ef[byte_off]
            print(f"  {flag_id} ({name}): byte {byte_off}, bit {bit_pos}, value 0x{byte_val:02X}")
