#!/usr/bin/env python3
"""
Re-verify Volcano Manor at base 36737 (original legacymap formula).
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

# Boss flags we expect
BOSS_FLAGS = [
    (16000800, "Rykard, Lord of Blasphemy"),
    (16000850, "God-Devouring Serpent (phase 1)"),
    (16000860, "Abductor Virgins"),
]

# Grace flags
GRACE_FLAGS = [
    (16000900, "Volcano Manor"),
    (16000901, "Temple of Eiglay"),
    (16000902, "Guest Hall"),
    (16000903, "Prison Town Church"),
    (16000904, "Subterranean Inquisition Chamber"),
]

BASE_36737 = 36737  # Original legacymap formula
BASE_40517 = 40517  # Current candidate
SECTION_SIZE = 1125


def detect_event_flags_start(slot_data, search_start):
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - 0x1bf99f)

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


def check_dungeon_flag(ef_data, base, flag_id):
    """Check dungeon flag using section formula."""
    area = flag_id // 1000000
    section = (flag_id % 1000000) // 1000
    local = flag_id % 1000

    byte_offset = base + section * SECTION_SIZE + local // 8
    bit_pos = 7 - (local % 8)

    if byte_offset < len(ef_data):
        byte_val = ef_data[byte_offset]
        is_set = bool(byte_val & (1 << bit_pos))
        return is_set, byte_offset, bit_pos, byte_val, section, local
    return None, byte_offset, bit_pos, 0, section, local


def main():
    print("=" * 80)
    print("RE-VERIFY VOLCANO MANOR BASES")
    print("=" * 80)

    slot0_data = read_slot_data(BACKUP_FILE, 0)
    slot1_data = read_slot_data(BACKUP_FILE, 1)

    ef_start_s0 = detect_event_flags_start(slot0_data, SEARCH_START)
    ef_start_s1 = detect_event_flags_start(slot1_data, SEARCH_START)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_data_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_data_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]

    print(f"\nSlot 0 EF start: 0x{ef_start_s0:X}")
    print(f"Slot 1 EF start: 0x{ef_start_s1:X}")

    for base in [BASE_36737, BASE_40517]:
        print(f"\n{'='*80}")
        print(f"BASE {base}")
        print("=" * 80)

        # Show raw bytes
        print(f"\nRaw bytes at base (first 20):")
        print("Slot 0:")
        for i in range(20):
            byte_val = ef_data_s0[base + i]
            print(f"  Byte {base + i}: 0x{byte_val:02X} ({byte_val:08b})")

        print("\nSlot 1:")
        for i in range(20):
            byte_val = ef_data_s1[base + i]
            print(f"  Byte {base + i}: 0x{byte_val:02X} ({byte_val:08b})")

        # Check boss flags
        print(f"\n--- Boss Flags ---")
        for flag_id, name in BOSS_FLAGS:
            result_s0 = check_dungeon_flag(ef_data_s0, base, flag_id)
            result_s1 = check_dungeon_flag(ef_data_s1, base, flag_id)

            is_set_s0, byte_off, bit_pos, byte_val, section, local = result_s0
            is_set_s1 = result_s1[0]

            s0_str = "SET" if is_set_s0 else "unset"
            s1_str = "SET" if is_set_s1 else "unset"

            print(f"  {flag_id} ({name}): S0={s0_str}, S1={s1_str}")
            print(f"    -> byte {byte_off}, bit {bit_pos}, value 0x{byte_val:02X}")

        # Check grace flags
        print(f"\n--- Grace Flags ---")
        for flag_id, name in GRACE_FLAGS:
            result_s0 = check_dungeon_flag(ef_data_s0, base, flag_id)
            result_s1 = check_dungeon_flag(ef_data_s1, base, flag_id)

            is_set_s0 = result_s0[0]
            is_set_s1 = result_s1[0]

            s0_str = "SET" if is_set_s0 else "unset"
            s1_str = "SET" if is_set_s1 else "unset"

            print(f"  {flag_id} ({name}): S0={s0_str}, S1={s1_str}")

        # Count total set bits
        total_s0 = sum(bin(ef_data_s0[base + i]).count('1') for i in range(SECTION_SIZE) if base + i < len(ef_data_s0))
        total_s1 = sum(bin(ef_data_s1[base + i]).count('1') for i in range(SECTION_SIZE) if base + i < len(ef_data_s1))

        print(f"\n--- Sparsity Analysis ---")
        print(f"  Total SET bits in section 0: S0={total_s0}, S1={total_s1}")
        print(f"  Density: S0={total_s0/(SECTION_SIZE*8)*100:.2f}%, S1={total_s1/(SECTION_SIZE*8)*100:.2f}%")

    # Analysis
    print(f"\n{'='*80}")
    print("ANALYSIS")
    print("=" * 80)

    # Check at 36737
    abductor_36737 = check_dungeon_flag(ef_data_s0, BASE_36737, 16000860)
    rykard_36737 = check_dungeon_flag(ef_data_s0, BASE_36737, 16000800)
    serpent_36737 = check_dungeon_flag(ef_data_s0, BASE_36737, 16000850)

    print(f"\nAt base {BASE_36737}:")
    print(f"  Abductor Virgins (860): {'SET' if abductor_36737[0] else 'UNSET'}")
    print(f"  Rykard (800): {'SET' if rykard_36737[0] else 'UNSET'}")
    print(f"  God-Devouring Serpent (850): {'SET' if serpent_36737[0] else 'UNSET'}")

    # Check at 40517
    abductor_40517 = check_dungeon_flag(ef_data_s0, BASE_40517, 16000860)
    rykard_40517 = check_dungeon_flag(ef_data_s0, BASE_40517, 16000800)
    serpent_40517 = check_dungeon_flag(ef_data_s0, BASE_40517, 16000850)

    print(f"\nAt base {BASE_40517}:")
    print(f"  Abductor Virgins (860): {'SET' if abductor_40517[0] else 'UNSET'}")
    print(f"  Rykard (800): {'SET' if rykard_40517[0] else 'UNSET'}")
    print(f"  God-Devouring Serpent (850): {'SET' if serpent_40517[0] else 'UNSET'}")

    # Interpretation
    print("\n--- Interpretation ---")
    if abductor_36737[0] and not rykard_36737[0]:
        print(f"Base {BASE_36737}: Abductor SET, Rykard UNSET - matches expected pattern!")
        if serpent_36737[0]:
            print(f"  WARNING: God-Devouring Serpent also SET - unexpected if Rykard not killed")
    elif abductor_40517[0] and not rykard_40517[0]:
        print(f"Base {BASE_40517}: Abductor SET, Rykard UNSET - matches expected pattern!")
    else:
        print("Neither base shows expected pattern (Abductor SET, Rykard UNSET)")
        print("Possible explanations:")
        print("  1. The boss flag IDs are different than assumed")
        print("  2. The save file doesn't have Abductor kill recorded")
        print("  3. The base offset is completely different")


if __name__ == "__main__":
    main()
