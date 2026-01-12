#!/usr/bin/env python3
"""
Refine dungeon formula base offsets with fine-grained search.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser

save_path = Path("/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2")
parser = SaveParser()

# Load slots
print("Loading saves...")
save_slot0 = parser.parse(save_path, [0])  # Confessor
save_slot4 = parser.parse(save_path, [4])  # V3

ef_prog = save_slot0.slots[0].event_flags
ef_fresh = save_slot4.slots[0].event_flags

# Dungeon flags organized by area
# Each entry: (flag_id, name, section)
area_30_flags = [  # Catacombs
    (30000800, "Cemetery Shade (Tombsward)", 0),
    (30010800, "Miranda (Tombsward Depths)", 1),
    (30020800, "Black Knife Assassin", 2),
    (30030800, "Spirit-Caller Snail (Road's End)", 3),
    (30040800, "Erdtree Burial Watchdog (Impaler's)", 4),
    (30050800, "Mad Pumpkin Head (Minor Erdtree)", 5),
    (30060800, "Erdtree Burial Watchdog (Wyndham)", 6),
    (30070800, "Ancient Hero of Zamor (Sainted Hero)", 7),
    (30080800, "Red Wolf of the Champion (Gelmir)", 8),
    (30090800, "Erdtree Burial Watchdog (Cliffbottom)", 9),
    (30100800, "Grave Warden Duelist (Murkwater)", 10),
    (30110800, "Erdtree Burial Watchdog (Stormfoot)", 11),
    (30120800, "Cemetery Shade (Caelid)", 12),
    (30130800, "Putrid Tree Spirit (War-Dead)", 13),
    (30140800, "Misbegotten Warrior (Unsightly)", 14),
    (30150800, "Ancient Hero of Zamor (Giant-Conquering)", 15),
    (30160800, "Ulcerated Tree Spirit (Giants Mountaintop)", 16),
    (30170800, "Stray Mimic Tear (Hidden Path)", 17),
    (30180800, "Ulcerated Tree Spirit (Fringefolk)", 18),
    (30190800, "Crucible Knight Ordovis (Auriza Hero)", 19),
    (30200800, "Fell Twins (Auriza Side Tomb)", 20),
]

area_31_flags = [  # Caves
    (31000800, "Patches (Murkwater)", 0),
    (31010800, "Beastman (Groveside)", 1),
    (31020800, "Demi-Human Chiefs (Coastal)", 2),
    (31030800, "Runebear (Earthbore)", 3),
    (31040800, "Cleanrot Knight (Stillwater)", 4),
    (31050800, "Kindred of Rot (Seethewater)", 5),
    (31060800, "Guardian Golem (Highroad)", 6),
    (31070800, "Miranda (Perfumer's)", 7),
    (31090800, "Necromancer Garris (Sage's)", 8),
    (31100800, "Frenzied Duelist (Gaol)", 9),
    (31170800, "Magma Wyrm (Volcano)", 17),
    (31180800, "Demi-Human Queen (Volcano)", 18),
    (31190800, "Demi-Human Queen Margot", 19),
    (31200800, "Dragonkin Soldier (Lake of Rot)", 20),
    (31210800, "Battlemage Hugues (Sellia)", 21),
    (31220800, "Cleanrot Knight (Abandoned)", 22),
]

area_32_flags = [  # Tunnels
    (32000800, "Stonedigger Troll (Limgrave)", 0),
    (32010800, "Crystalians (Raya Lucaria)", 1),
    (32020800, "Magma Wyrm (Gael)", 2),
    (32040800, "Onyx Lord (Sealed)", 4),
    (32050800, "Crystalian (Altus)", 5),
    (32070800, "Fallingstar Beast (Sellia)", 7),
]

bytes_per_section = 1125

def check_flag(ef, byte_off, bit_pos):
    if byte_off >= len(ef):
        return None
    return (ef[byte_off] >> bit_pos) & 1

def search_area(area, flags, search_ranges):
    """Search for the correct base offset for a dungeon area."""
    print(f"\n{'=' * 70}")
    print(f"Area {area}: Searching {len(flags)} boss flags")
    print("=" * 70)

    best_base = None
    best_matches = 0
    best_details = []

    for start, end in search_ranges:
        for test_base in range(start, end):
            matches = 0
            prog_only = []
            both_set = []

            for flag_id, name, section in flags:
                local_id = flag_id % 10000
                byte_off = test_base + section * bytes_per_section + local_id // 8
                bit_pos = 7 - (local_id % 8)

                prog_set = check_flag(ef_prog, byte_off, bit_pos)
                fresh_set = check_flag(ef_fresh, byte_off, bit_pos)

                if prog_set is None:
                    continue

                if prog_set and not fresh_set:
                    matches += 1
                    prog_only.append((flag_id, name, section, byte_off, bit_pos))
                elif prog_set and fresh_set:
                    both_set.append((flag_id, name, section))

            if matches > best_matches:
                best_matches = matches
                best_base = test_base
                best_details = prog_only

    if best_matches > 0:
        print(f"\nBest base: {best_base} ({best_matches}/{len(flags)} matches)")
        for flag_id, name, section, byte_off, bit_pos in best_details:
            print(f"  {flag_id} ({name}): sec={section}, byte={byte_off}, bit={bit_pos}")

        # Verify the formula
        print(f"\nFormula verification:")
        print(f"  base_offset = {best_base}")
        print(f"  bytes_per_section = {bytes_per_section}")
        print(f"  byte = base + section * {bytes_per_section} + local_id // 8")
        print(f"  bit = 7 - (local_id % 8)")
    else:
        print("No matches found")

    return best_base, best_matches

# Search ranges based on initial discovery
# Area 30: found around 27800
# Area 31: found around 28800
# Area 32: found around 31400

print("Searching with fine granularity...")

# First, let's check what bytes are different between prog and fresh in key ranges
print("\n" + "=" * 70)
print("Checking byte differences in suspected ranges")
print("=" * 70)

# For catacombs (area 30), check around bytes where boss flags would be
# If base is ~27800 and section 0 boss is at local 800, byte would be 27800 + 100 = 27900

for check_range, name in [
    ((4500, 5000), "Legacy dungeons area"),
    ((27700, 28000), "Catacombs section 0-2"),
    ((28800, 29200), "Caves section 0-2"),
    ((31300, 31700), "Tunnels section 0-2"),
]:
    start, end = check_range
    diffs = []
    for i in range(start, end):
        if ef_prog[i] != ef_fresh[i]:
            # Check which bits differ
            diff_val = ef_prog[i] ^ ef_fresh[i]
            diff_bits = [7 - b for b in range(8) if (diff_val >> b) & 1]
            diffs.append((i, ef_prog[i], ef_fresh[i], diff_bits))

    print(f"\n{name} ({start}-{end}): {len(diffs)} bytes differ")
    for byte_off, prog, fresh, bits in diffs[:10]:
        print(f"  Byte {byte_off}: 0x{fresh:02X} -> 0x{prog:02X} (bits changed: {bits})")

# Fine-grained search for each area
search_area(30, area_30_flags, [(27000, 29000)])
search_area(31, area_31_flags, [(28000, 30000)])
search_area(32, area_32_flags, [(31000, 33000)])

# Also search legacy dungeons (10, 14, 16) in a different range
legacy_flags = [
    (10000800, "Godrick (Stormveil)", 10, 0),
    (14000800, "Rennala (Academy)", 14, 0),
    (16000800, "Godskin Noble (Volcano)", 16, 0),
]

print("\n" + "=" * 70)
print("Legacy Dungeon Boss Flags (checking early byte range)")
print("=" * 70)

# Check around byte 4600 which was found earlier
for byte_range in [(4400, 4800), (4000, 5000)]:
    print(f"\nSearching range {byte_range}:")
    for flag_id, name, area, section in legacy_flags:
        local_id = flag_id % 10000
        expected_bit = 7 - (local_id % 8)  # Should be 7 for xxx800

        # Search for bytes where this bit is set in prog but not fresh
        found = []
        for byte_off in range(byte_range[0], byte_range[1]):
            prog_set = (ef_prog[byte_off] >> expected_bit) & 1
            fresh_set = (ef_fresh[byte_off] >> expected_bit) & 1
            if prog_set and not fresh_set:
                found.append(byte_off)

        if found:
            print(f"  {flag_id} ({name}): bit {expected_bit} set at bytes {found[:5]}...")
            # Calculate what base this would imply
            for byte_off in found[:3]:
                # byte_off = base + section * 1125 + local_id // 8
                # For section 0, local 800: base = byte_off - 100
                implied_base = byte_off - section * bytes_per_section - local_id // 8
                print(f"    byte {byte_off} -> implied base = {implied_base}")

print("\nDone")
