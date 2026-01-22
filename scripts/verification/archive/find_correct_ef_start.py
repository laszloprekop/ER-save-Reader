#!/usr/bin/env python3
"""
Find the CORRECT EventFlags start by using negative validation
and cross-checking Stormveil grace patterns.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
BACKUP_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
SEARCH_START = 0x12000

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Negative flags - late-game graces that mid-game character shouldn't have
NEGATIVE_FLAGS = [
    (76300, 3287, 3, "Zamor Ruins"),  # Mountaintops
    (76301, 3287, 2, "Ancient Snow Valley"),  # Mountaintops
    (76350, 3293, 5, "Haligtree Town"),  # Endgame
]

STORMVEIL_GRACES = [
    (71000, "Godrick the Grafted"),
    (71001, "Margit, the Fell Omen"),
    (71002, "Castleward Tunnel"),
    (71003, "Gateside Chamber"),
    (71004, "Stormveil Cliffside"),
    (71005, "Rampart Tower"),
    (71006, "Liftside Chamber"),
    (71007, "Secluded Cell"),
    (71008, "Stormveil Main Gate"),
]

def read_slot_data(save_file, slot_index):
    with open(save_file, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def check_flag(data, offset, byte_off, bit_pos):
    abs_pos = offset + byte_off
    if abs_pos < len(data):
        return bool(data[abs_pos] & (1 << bit_pos))
    return False

def main():
    print("=" * 70)
    print("FIND CORRECT EF START WITH NEGATIVE VALIDATION")
    print("=" * 70)

    slot0_data = read_slot_data(BACKUP_FILE, 0)

    # Find all candidates that pass positive validation
    candidates = []
    max_search = 200_000
    search_end = min(SEARCH_START + max_search, len(slot0_data) - 0x1bf99f)

    for test_offset in range(SEARCH_START, search_end):
        # Check positive flags
        pos_score = 0
        for flag_id, byte_off, bit_pos, name in VALIDATION_FLAGS:
            if check_flag(slot0_data, test_offset, byte_off, bit_pos):
                pos_score += 1

        if pos_score == 4:  # All positive flags match
            # Check negative flags
            neg_score = 0
            for flag_id, byte_off, bit_pos, name in NEGATIVE_FLAGS:
                if not check_flag(slot0_data, test_offset, byte_off, bit_pos):
                    neg_score += 1

            candidates.append((test_offset, neg_score))

    print(f"\nFound {len(candidates)} candidates with all 4 positive flags SET")

    # Sort by negative score (higher = more late-game graces NOT set = more likely correct)
    candidates.sort(key=lambda x: -x[1])

    print("\nTop 10 candidates (by negative validation score):")
    for offset, neg_score in candidates[:10]:
        print(f"  0x{offset:X}: neg_score={neg_score}/3")

    # For top candidates, check Stormveil graces
    print("\n" + "=" * 70)
    print("STORMVEIL GRACES CHECK FOR TOP CANDIDATES")
    print("=" * 70)

    for offset, neg_score in candidates[:5]:
        print(f"\n--- EF start 0x{offset:X} (neg_score={neg_score}) ---")

        # Check Stormveil at base 2673
        base = 2673
        set_count = 0
        flags_status = []

        for flag_id, name in STORMVEIL_GRACES:
            local = flag_id - 71000
            byte_off = base + local // 8
            bit_pos = 7 - (local % 8)

            is_set = check_flag(slot0_data, offset, byte_off, bit_pos)
            if is_set:
                set_count += 1
                flags_status.append(f"{flag_id}:SET")
            else:
                flags_status.append(f"{flag_id}:unset")

        print(f"  Base 2673: {set_count}/9 SET")
        print(f"  Details: {', '.join(flags_status)}")

        # Check 71008 specifically
        local_71008 = 8
        byte_71008 = base + local_71008 // 8
        bit_71008 = 7 - (local_71008 % 8)
        abs_byte = offset + byte_71008
        byte_val = slot0_data[abs_byte]
        is_71008_set = bool(byte_val & (1 << bit_71008))
        print(f"  71008 (Main Gate): byte {byte_71008} = 0x{byte_val:02X}, bit {bit_71008} = {is_71008_set}")

    # Also try searching different bases for the best candidate
    print("\n" + "=" * 70)
    print("SEARCHING FOR BEST BASE AT TOP CANDIDATE")
    print("=" * 70)

    best_offset = candidates[0][0]
    print(f"\nUsing EF start 0x{best_offset:X}")
    print("Searching bases 2600-2750 for where 71008 IS SET...")

    for test_base in range(2600, 2750):
        local_71008 = 8
        byte_off = test_base + local_71008 // 8
        bit_pos = 7 - (local_71008 % 8)

        if check_flag(slot0_data, best_offset, byte_off, bit_pos):
            # 71008 is SET here
            # Count other graces
            set_count = 0
            flags_set = []
            for flag_id, name in STORMVEIL_GRACES:
                local = flag_id - 71000
                bo = test_base + local // 8
                bp = 7 - (local % 8)
                if check_flag(slot0_data, best_offset, bo, bp):
                    set_count += 1
                    flags_set.append(flag_id)

            if set_count >= 3:
                print(f"  Base {test_base}: 71008 SET, total {set_count}/9")
                print(f"    Flags SET: {flags_set}")

if __name__ == "__main__":
    main()
