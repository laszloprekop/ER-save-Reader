#!/usr/bin/env python3
"""
Analyze character progression across all save slots to identify
what can be verified vs what needs new save file coverage.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
BACKUP_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
SEARCH_START = 0x12000
EVENT_FLAGS_SIZE = 0x1bf99f

# Known validated grace flags for detection
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Progression indicators to check
# Dungeon formula: base + section * 1125 + local_id / 8
# From ground_truth.rs:
DUNGEON_BASES = {
    10: 4112,   # Stormveil
    11: 8612,   # Raya Lucaria
    12: 15362,  # Underground
    13: 26612,  # Leyndell
    14: 29987,  # Sewers
    15: 33362,  # Haligtree
    16: 40517,  # Volcano Manor (candidate)
    18: 43487,  # Roundtable Hold
    30: 27411,  # Catacombs
    31: 28634,  # Caves
    32: 31577,  # Tunnels
}

def calc_dungeon_flag(flag_id):
    """Calculate byte offset and bit for dungeon flag."""
    area = flag_id // 1_000_000
    section = (flag_id // 10_000) % 100
    local_id = flag_id % 10_000
    base = DUNGEON_BASES.get(area)
    if base is None:
        return None, None
    byte_offset = base + section * 1125 + local_id // 8
    bit_pos = 7 - (local_id % 8)
    return byte_offset, bit_pos

def calc_block_flag(flag_id, block_start, base):
    """Calculate byte offset and bit for block-based flag."""
    local = flag_id - block_start
    byte_offset = base + local // 8
    bit_pos = 7 - (local % 8)
    return byte_offset, bit_pos

# Build progression checks with correct calculations
def build_progression_checks():
    checks = {
        "Bosses": [],
        "Graces": [],
        "Maps": [],
    }

    # Boss defeats using dungeon formula
    boss_flags = [
        (10000800, "Godrick (Stormveil)"),
        (11000800, "Rennala (Raya Lucaria)"),
        (16000860, "Abductor Virgins (VM teleport)"),  # The boss killed by Confessor
        (16000800, "Rykard (Volcano Manor)"),
        (14000800, "Mohg, the Omen (Sewers)"),
        (13000800, "Morgott (Leyndell)"),
        (30020800, "Cemetery Shade (Black Knife)"),
        (31010800, "Beastman (Coastal Cave)"),
    ]
    for flag_id, name in boss_flags:
        byte_off, bit_pos = calc_dungeon_flag(flag_id)
        checks["Bosses"].append((flag_id, byte_off, bit_pos, name))

    # Grace blocks with proper calculation
    grace_flags = [
        # Block 71000 Stormveil (base 9315)
        (71000, 71000, 9315, "Godrick grace"),
        (71001, 71000, 9315, "Margit grace"),
        (71008, 71000, 9315, "Stormveil Main Gate"),
        # Block 71100 Leyndell (base 2705)
        (71100, 71100, 2705, "Divine Bridge (teleport)"),
        (71107, 71100, 2705, "West Capital Rampart"),
        # Block 71600 Volcano Manor (base 2825)
        (71600, 71600, 2825, "Audience Pathway"),
        (71607, 71600, 2825, "Abductor grace"),
        # Block 71800 Tutorial (base 2725)
        (71800, 71800, 2725, "Cave of Knowledge"),
        (71801, 71800, 2725, "Stranded Graveyard"),
        # Block 76100 (base 3262 = 3250 + 100/8 = 3262.5 -> actually calculated per flag)
        (76100, 76000, 3250, "The First Step"),
        (76101, 76000, 3250, "Church of Elleh"),
        (76102, 76000, 3250, "Stormhill Shack"),
        (76103, 76000, 3250, "Warmaster's Shack"),
        (76104, 76000, 3250, "Artist's Shack"),
        (76110, 76000, 3250, "Gatefront Ruins"),
        (76117, 76000, 3250, "Agheel Lake North"),
    ]
    for flag_id, block_start, base, name in grace_flags:
        byte_off, bit_pos = calc_block_flag(flag_id, block_start, base)
        checks["Graces"].append((flag_id, byte_off, bit_pos, name))

    # Map fragments (Block 62000, base 9359)
    map_flags = [
        (62010, 62000, 9359, "Limgrave, West"),
        (62011, 62000, 9359, "Limgrave, East"),
        (62012, 62000, 9359, "Weeping Peninsula"),
        (62040, 62000, 9359, "Liurnia, East"),
        (62041, 62000, 9359, "Liurnia, North"),
        (62050, 62000, 9359, "Altus Plateau"),
        (62060, 62000, 9359, "Mt. Gelmir"),
        (62061, 62000, 9359, "Leyndell, Royal Capital"),
    ]
    for flag_id, block_start, base, name in map_flags:
        byte_off, bit_pos = calc_block_flag(flag_id, block_start, base)
        checks["Maps"].append((flag_id, byte_off, bit_pos, name))

    return checks

PROGRESSION_CHECKS = build_progression_checks()

def detect_event_flags_start(slot_data, search_start):
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - EVENT_FLAGS_SIZE)

    for test_offset in range(search_start, search_end):
        positive_score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    positive_score += 1

        if positive_score == len(VALIDATION_FLAGS):
            return test_offset

    return 0x12B00

def read_slot_data(save_file, slot_index):
    with open(save_file, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def check_flag(ef_data, byte_offset, bit_pos):
    if byte_offset is None or byte_offset >= len(ef_data):
        return None
    return bool(ef_data[byte_offset] & (1 << bit_pos))

def main():
    print("=" * 80)
    print("CHARACTER PROGRESSION ANALYSIS")
    print("=" * 80)

    slots_info = [
        (0, "Confessor (mid-game)"),
        (1, "Wretch (early game)"),
        (2, "V1 (item pickup)"),
        (3, "V2 (different path)"),
        (4, "V3 (true negative)"),
    ]

    results = {}
    for slot_idx, slot_name in slots_info:
        print(f"\n{'='*80}")
        print(f"SLOT {slot_idx}: {slot_name}")
        print("="*80)

        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = detect_event_flags_start(slot_data, SEARCH_START)
        ef_data = slot_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        print(f"EF start: 0x{ef_start:X}")

        slot_results = {}
        for category, checks in PROGRESSION_CHECKS.items():
            print(f"\n--- {category} ---")
            cat_results = {}
            for item in checks:
                flag_id, byte_off, bit_pos, name = item
                if byte_off is not None:
                    val = check_flag(ef_data, byte_off, bit_pos)
                    status = "SET" if val else "unset" if val is not None else "?"
                else:
                    status = "?"
                cat_results[flag_id] = status
                indicator = "✓" if status == "SET" else "-" if status == "unset" else "?"
                print(f"  {indicator} {flag_id}: {name}")
            slot_results[category] = cat_results
        results[slot_idx] = slot_results

    # Summary comparison
    print("\n" + "=" * 80)
    print("SUMMARY: SLOT COMPARISON")
    print("=" * 80)

    print("\n{:<30} {:>10} {:>10} {:>10} {:>10} {:>10}".format(
        "Flag", "Slot 0", "Slot 1", "Slot 2", "Slot 3", "Slot 4"
    ))
    print("-" * 80)

    for category, checks in PROGRESSION_CHECKS.items():
        print(f"\n[{category}]")
        for item in checks:
            flag_id, _, _, name = item
            row = [name[:28]]
            for slot_idx in range(5):
                if slot_idx in results and category in results[slot_idx]:
                    status = results[slot_idx][category].get(flag_id, "?")
                    row.append(status)
                else:
                    row.append("?")
            print("{:<30} {:>10} {:>10} {:>10} {:>10} {:>10}".format(*row))

    # Verification opportunities
    print("\n" + "=" * 80)
    print("VERIFICATION OPPORTUNITIES")
    print("=" * 80)
    print("""
Based on slot comparison:
- Flags SET in Slot 0 but UNSET in Slot 1+ can be verified (positive evidence)
- Flags UNSET in all slots cannot be directly verified
- Boss defeat flags need boss kills to verify

UNVERIFIED AREAS NEEDING NEW SAVES:
1. Godrick defeat (10000800) - need to defeat Godrick
2. Rennala defeat (11000800) - need to defeat Rennala
3. Rykard defeat (16000800) - need to defeat Rykard
4. Leyndell progression (13xxxxxx) - need Morgott access
5. Underground areas (12xxxxxx) - need Siofra/Ainsel exploration
""")

if __name__ == "__main__":
    main()
