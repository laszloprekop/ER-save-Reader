#!/usr/bin/env python3
"""
Discover dungeon formula base offsets by comparing save slots.
Format: AASSZZZZ where AA=area, SS=section, ZZZZ=localId
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser

save_path = Path("/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2")
parser = SaveParser()

# Known dungeon boss flags and stakes (8-digit format)
# Format: AASSZZZZ where AA=area, SS=section (map index), ZZZZ=local flag
dungeon_flags = [
    # Area 30 - Catacombs
    (30000800, "Cemetery Shade (Tombsward Catacombs)", 30, 0),
    (30002840, "Stake of Marika (Tombsward)", 30, 0),
    (30010800, "Miranda the Blighted Bloom (Tombsward)", 30, 1),
    (30020800, "Black Knife Assassin (Black Knife Catacombs)", 30, 2),
    (30030800, "Spirit-Caller Snail (Road's End)", 30, 3),
    (30040800, "Erdtree Burial Watchdog (Impaler's)", 30, 4),
    (30050800, "Mad Pumpkin Head Duo (Minor Erdtree)", 30, 5),
    (30100800, "Grave Warden Duelist (Murkwater)", 30, 10),
    (30110800, "Erdtree Burial Watchdog (Stormfoot)", 30, 11),

    # Area 31 - Caves
    (31000800, "Patches (Murkwater Cave)", 31, 0),
    (31010800, "Beastman of Farum Azula (Groveside)", 31, 1),
    (31020800, "Demi-Human Chiefs (Coastal)", 31, 2),
    (31030800, "Runebear (Earthbore)", 31, 3),
    (31040800, "Cleanrot Knight (Stillwater)", 31, 4),
    (31070800, "Miranda the Blighted (Perfumer's Grotto)", 31, 7),

    # Area 32 - Tunnels
    (32000800, "Stonedigger Troll (Limgrave Tunnels)", 32, 0),
    (32010800, "Crystalians (Raya Lucaria)", 32, 1),
    (32020800, "Magma Wyrm (Gael Tunnel)", 32, 2),
    (32040800, "Onyx Lord (Sealed Tunnel)", 32, 4),

    # Area 10 - Stormveil Castle
    (10000800, "Godrick the Grafted (Stormveil)", 10, 0),

    # Area 14 - Academy of Raya Lucaria
    (14000800, "Rennala, Queen of the Full Moon", 14, 0),

    # Area 16 - Volcano Manor
    (16000800, "Godskin Noble (Volcano Manor)", 16, 0),
]

# Load slots
print("Loading saves...")
save_slot0 = parser.parse(save_path, [0])  # Confessor - progressed
save_slot4 = parser.parse(save_path, [4])  # V3 - minimal

ef_prog = save_slot0.slots[0].event_flags
ef_fresh = save_slot4.slots[0].event_flags

print(f"Slot 0 (Confessor) EventFlags: {len(ef_prog)} bytes")
print(f"Slot 4 (V3) EventFlags: {len(ef_fresh)} bytes")

# Current dungeon formula from flag_formulas.py
# byte_offset = base + section * bytes_per_section + local_id // 8
# bit = 7 - (local_id % 8)

# Known from flag_formulas.py:
# Area 10 (Stormveil): base_offset = 1383375, bytes_per_section = 1125

def check_flag_at_offset(ef, byte_off, bit_pos):
    if byte_off >= len(ef):
        return None
    return (ef[byte_off] >> bit_pos) & 1

def parse_dungeon_flag(flag_id):
    """Parse 8-digit dungeon flag into components."""
    flag_str = str(flag_id)
    if len(flag_str) != 8:
        return None
    area = int(flag_str[0:2])
    section = int(flag_str[2:4])
    local_id = int(flag_str[4:8])
    return area, section, local_id

print("\n" + "=" * 70)
print("Testing dungeon flags with various base offsets")
print("=" * 70)

# Group flags by area
areas = {}
for flag_id, name, area, section in dungeon_flags:
    if area not in areas:
        areas[area] = []
    areas[area].append((flag_id, name, section))

# For each area, try to find a base that works
bytes_per_section = 1125  # Standard section size

for area, flags in sorted(areas.items()):
    print(f"\n{'=' * 70}")
    print(f"Area {area}: {len(flags)} flags")
    print("=" * 70)

    # Test a wide range of base offsets
    # The event flags section is ~1.8MB, so bases could be anywhere
    best_base = None
    best_matches = 0
    best_details = []

    # Search in chunks to find candidate ranges
    for base_start in range(0, 1800000, 10000):
        for test_base in range(base_start, min(base_start + 10000, 1800000), 100):
            matches = 0
            details = []

            for flag_id, name, section in flags:
                parsed = parse_dungeon_flag(flag_id)
                if not parsed:
                    continue
                _, sec, local_id = parsed

                byte_off = test_base + sec * bytes_per_section + local_id // 8
                bit_pos = 7 - (local_id % 8)

                prog_set = check_flag_at_offset(ef_prog, byte_off, bit_pos)
                fresh_set = check_flag_at_offset(ef_fresh, byte_off, bit_pos)

                if prog_set is not None and prog_set and not fresh_set:
                    matches += 1
                    details.append((flag_id, name, byte_off, bit_pos))

            if matches > best_matches:
                best_matches = matches
                best_base = test_base
                best_details = details

    if best_matches > 0:
        print(f"\nBest base found: {best_base} with {best_matches}/{len(flags)} matches")
        for flag_id, name, byte_off, bit_pos in best_details:
            print(f"  {flag_id} ({name}): byte {byte_off}, bit {bit_pos}")
    else:
        print(f"\nNo matches found for area {area}")

    # Also try the documented base if available
    documented_bases = {
        10: 1383375,  # Stormveil
    }

    if area in documented_bases:
        doc_base = documented_bases[area]
        print(f"\nTesting documented base {doc_base}:")
        for flag_id, name, section in flags:
            parsed = parse_dungeon_flag(flag_id)
            if not parsed:
                continue
            _, sec, local_id = parsed

            byte_off = doc_base + sec * bytes_per_section + local_id // 8
            bit_pos = 7 - (local_id % 8)

            prog_set = check_flag_at_offset(ef_prog, byte_off, bit_pos)
            fresh_set = check_flag_at_offset(ef_fresh, byte_off, bit_pos)

            status = "???"
            if prog_set is None:
                status = "OUT OF RANGE"
            elif prog_set and not fresh_set:
                status = "MATCH (prog=1, fresh=0)"
            elif prog_set and fresh_set:
                status = "both set"
            elif not prog_set and not fresh_set:
                status = "both unset"
            else:
                status = f"prog={prog_set}, fresh={fresh_set}"

            print(f"  {flag_id} ({name}): byte {byte_off}, bit {bit_pos} -> {status}")

print("\n" + "=" * 70)
print("Done")
