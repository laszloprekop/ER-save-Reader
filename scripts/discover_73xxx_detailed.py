#!/usr/bin/env python3
"""
Detailed investigation of 73xxx dungeon grace storage.
Compare different character slots to find where dungeon graces are stored.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser

save_path = Path("/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2")
parser = SaveParser()

# Known dungeon grace flags from extracted_event_flags.json
# These are from BonfireWarpParam with area_no=30 (catacombs), 31 (caves), 32 (tunnels)
dungeon_graces = [
    (73000, "Tombsward Catacombs"),
    (73001, "Impaler's Catacombs"),
    (73002, "Stormfoot Catacombs"),
    (73003, "Road's End Catacombs"),
    (73004, "Murkwater Catacombs"),
    (73005, "Black Knife Catacombs"),
    (73006, "Cliffbottom Catacombs"),
    (73007, "Wyndham Catacombs"),
    (73010, "Murkwater Cave"),
    (73011, "Groveside Cave"),
    (73012, "Coastal Cave"),
    (73013, "Earthbore Cave"),
    (73014, "Stillwater Cave"),
]

# Load slots 0 (Confessor - progressed) and 4 (V3 - minimal progress)
print("Loading saves...")
save_slot0 = parser.parse(save_path, [0])
save_slot4 = parser.parse(save_path, [4])

ef_progressed = save_slot0.slots[0].event_flags
ef_fresh = save_slot4.slots[0].event_flags

print(f"Slot 0 (Confessor) EventFlags: {len(ef_progressed)} bytes")
print(f"Slot 4 (V3) EventFlags: {len(ef_fresh)} bytes")

# First, let's test the current estimated base (2875) from flag_formulas.py
print("\n" + "=" * 60)
print("Testing estimated base 2875 (from interpolation)")
print("=" * 60)

test_base = 2875
for flag_id, name in dungeon_graces[:6]:  # Test first 6
    relative = flag_id - 73000
    byte_off = test_base + (relative // 8)
    bit_pos = 7 - (flag_id % 8)

    prog_byte = ef_progressed[byte_off]
    fresh_byte = ef_fresh[byte_off]
    prog_set = (prog_byte >> bit_pos) & 1
    fresh_set = (fresh_byte >> bit_pos) & 1

    print(f"{flag_id} ({name}):")
    print(f"  Byte {byte_off}: progressed=0x{prog_byte:02X} fresh=0x{fresh_byte:02X}")
    print(f"  Bit {bit_pos}: progressed={prog_set} fresh={fresh_set}")

# Check raw bytes around 2875
print("\n" + "=" * 60)
print("Raw bytes around base 2875")
print("=" * 60)
print("Byte    Progressed   Fresh      Diff")
for i in range(2870, 2910):
    p = ef_progressed[i]
    f = ef_fresh[i]
    diff = "*" if p != f else " "
    print(f"{i:4d}    0x{p:02X}         0x{f:02X}       {diff}")

# Now search more broadly - compare byte ranges between slots
print("\n" + "=" * 60)
print("Searching for bytes that differ between slots (potential flag storage)")
print("Range: 2500-3500 (where dungeon graces might be)")
print("=" * 60)

differences = []
for i in range(2500, 3500):
    p = ef_progressed[i]
    f = ef_fresh[i]
    if p != f:
        differences.append((i, p, f))

print(f"Found {len(differences)} differing bytes")
for byte_off, prog, fresh in differences[:50]:  # Show first 50
    # For each difference, calculate what flag_id this might be for 73xxx
    # If this byte is for 73xxx flags with unknown base B:
    # byte_off = B + (flag_id - 73000) // 8
    # So flag_id could be: 73000 + (byte_off - B) * 8 + bit_position

    # We don't know B, but we can show what bits changed
    diff_bits = prog ^ fresh
    changed_bits = []
    for bit in range(8):
        if (diff_bits >> bit) & 1:
            changed_bits.append(7 - bit)  # Convert to big-endian bit order

    print(f"Byte {byte_off}: 0x{fresh:02X} -> 0x{prog:02X}  (changed bits: {changed_bits})")

# Try to find a base that makes sense
print("\n" + "=" * 60)
print("Searching for potential 73xxx base by testing all possibilities")
print("=" * 60)

# For each potential base, check if any of our known flags would be set
for test_base in range(2500, 3300):
    matches = 0
    match_details = []

    for flag_id, name in dungeon_graces:
        relative = flag_id - 73000
        byte_off = test_base + (relative // 8)
        bit_pos = 7 - (flag_id % 8)

        if byte_off >= len(ef_progressed):
            continue

        # Check if bit is set in progressed but not in fresh
        prog_set = (ef_progressed[byte_off] >> bit_pos) & 1
        fresh_set = (ef_fresh[byte_off] >> bit_pos) & 1

        if prog_set and not fresh_set:
            matches += 1
            match_details.append((flag_id, name, byte_off, bit_pos))

    if matches >= 2:
        print(f"\nBase {test_base}: {matches}/{len(dungeon_graces)} potential matches")
        for flag_id, name, byte_off, bit_pos in match_details:
            print(f"  {flag_id} ({name}): byte {byte_off}, bit {bit_pos}")

print("\n" + "=" * 60)
print("Done")
