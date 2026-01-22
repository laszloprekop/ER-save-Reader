#!/usr/bin/env python3
"""
Investigate the false positive for flag 14000800 (Mohg, the Omen) in Slots 1/2.
The current dungeon base for area 14 is 29987. This appears to be incorrect.

Area 14 = Subterranean Shunning-Grounds (Sewers under Leyndell)
Flag 14000800 = Mohg, the Omen boss defeat

Early game characters (Slots 1-4) cannot have defeated Mohg in the Sewers.
If the flag shows as SET, the base offset is wrong.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
BACKUP_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
SEARCH_START = 0x12000
EVENT_FLAGS_SIZE = 0x1bf99f

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Current dungeon base for area 14 (from ground_truth.rs)
CURRENT_AREA14_BASE = 29987

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

def calc_dungeon_flag_offset(flag_id, base_offset, section_size=1125):
    """Calculate offset for dungeon flag given a base."""
    section = (flag_id // 10_000) % 100
    local_id = flag_id % 10_000
    byte_offset = base_offset + section * section_size + local_id // 8
    bit_pos = 7 - (local_id % 8)
    return byte_offset, bit_pos

def main():
    print("=" * 80)
    print("INVESTIGATE AREA 14 FALSE POSITIVE")
    print("=" * 80)
    print(f"\nCurrent Area 14 base: {CURRENT_AREA14_BASE}")
    print("Flag 14000800 = Mohg, the Omen boss defeat (Sewers)")

    # Check flag 14000800 at current base
    byte_off, bit_pos = calc_dungeon_flag_offset(14000800, CURRENT_AREA14_BASE)
    print(f"\nFlag 14000800 at current base:")
    print(f"  Byte offset: {byte_off}, bit: {bit_pos}")

    print("\n" + "=" * 80)
    print("CHECK FLAG 14000800 ACROSS ALL SLOTS")
    print("=" * 80)

    for slot_idx in range(5):
        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = detect_event_flags_start(slot_data, SEARCH_START)
        ef_data = slot_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        byte_val = ef_data[byte_off] if byte_off < len(ef_data) else 0
        is_set = bool(byte_val & (1 << bit_pos))

        slot_names = ["Confessor (mid)", "Wretch (early)", "V1", "V2", "V3"]
        print(f"Slot {slot_idx} ({slot_names[slot_idx]}): Byte 0x{byte_val:02X}, Flag {'SET' if is_set else 'unset'}")

    # Analyze what byte value we're reading
    print("\n" + "=" * 80)
    print(f"ANALYZE RAW BYTES AT OFFSET {byte_off}")
    print("=" * 80)

    for slot_idx in range(5):
        slot_data = read_slot_data(BACKUP_FILE, slot_idx)
        ef_start = detect_event_flags_start(slot_data, SEARCH_START)
        ef_data = slot_data[ef_start:ef_start + EVENT_FLAGS_SIZE]

        slot_names = ["Confessor (mid)", "Wretch (early)", "V1", "V2", "V3"]
        print(f"\nSlot {slot_idx} ({slot_names[slot_idx]}), EF start 0x{ef_start:X}:")
        print(f"  Bytes around offset {byte_off}:")
        start = max(0, byte_off - 5)
        end = min(len(ef_data), byte_off + 10)
        for i in range(start, end):
            marker = " <--" if i == byte_off else ""
            print(f"    Byte {i}: 0x{ef_data[i]:02X} ({ef_data[i]:08b}){marker}")

    # Search for a better base for area 14
    print("\n" + "=" * 80)
    print("SEARCH FOR CORRECT AREA 14 BASE")
    print("=" * 80)
    print("\nCriteria: Flag 14000800 should be UNSET in Slots 1-4 (no Sewers access)")

    # Test bases from 25000 to 35000
    slot0_data = read_slot_data(BACKUP_FILE, 0)
    slot1_data = read_slot_data(BACKUP_FILE, 1)
    slot4_data = read_slot_data(BACKUP_FILE, 4)

    ef_start_s0 = detect_event_flags_start(slot0_data, SEARCH_START)
    ef_start_s1 = detect_event_flags_start(slot1_data, SEARCH_START)
    ef_start_s4 = detect_event_flags_start(slot4_data, SEARCH_START)

    ef_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]
    ef_s4 = slot4_data[ef_start_s4:ef_start_s4 + EVENT_FLAGS_SIZE]

    candidates = []
    for test_base in range(20000, 40000):
        byte_off, bit_pos = calc_dungeon_flag_offset(14000800, test_base)
        if byte_off >= len(ef_s0):
            continue

        # Flag should be UNSET in all slots (no one has defeated Mohg)
        s0_val = ef_s0[byte_off] & (1 << bit_pos)
        s1_val = ef_s1[byte_off] & (1 << bit_pos)
        s4_val = ef_s4[byte_off] & (1 << bit_pos)

        if s0_val == 0 and s1_val == 0 and s4_val == 0:
            # All unset - potential candidate
            # Check byte sparsity (Sewers area should be mostly empty in early saves)
            sparsity = sum(1 for i in range(test_base, min(test_base + 100, len(ef_s1))) if ef_s1[i] != 0)
            if sparsity < 5:  # Very sparse region
                candidates.append((test_base, sparsity))

    if candidates:
        print(f"\nFound {len(candidates)} candidate bases where 14000800 is UNSET in all slots:")
        for base, sparsity in sorted(candidates, key=lambda x: x[1])[:10]:
            print(f"  Base {base}: sparsity = {sparsity} bytes non-zero in first 100 bytes")
    else:
        print("\nNo candidates found where 14000800 is UNSET in all slots")

    # The issue might be that the formula is wrong
    print("\n" + "=" * 80)
    print("FORMULA CHECK")
    print("=" * 80)
    print("""
Flag 14000800 breakdown:
  Area: 14 (Subterranean Shunning-Grounds)
  Section: 00
  Local ID: 0800 (2048 decimal)

Formula: byte_offset = base + section * 1125 + local_id / 8
         = base + 0 * 1125 + 800 / 8
         = base + 100

So if base = 29987, byte_offset = 29987 + 100 = 30087
Bit position = 7 - (800 % 8) = 7 - 0 = 7 (MSB)
""")

if __name__ == "__main__":
    main()
