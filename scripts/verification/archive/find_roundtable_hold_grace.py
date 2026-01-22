#!/usr/bin/env python3
"""
Find Roundtable Hold grace (71190) location.

The Roundtable Hold is in map m11_10_00 (Area 11, section 10).
Area 18 base is 43487 (verified) for Roundtable Hold events.
But 71190 is a grace flag in the 71xxx range, not a dungeon flag.
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

# Known grace block bases
GRACE_BLOCKS = {
    71000: 9315,   # Stormveil
    71100: 2593,   # Leyndell
    71600: 2825,   # Verified
    71800: 2725,   # Tutorial
    72000: 2750,   # Verified
    73000: 2662,   # Verified
    74000: 3000,   # Verified
    76000: 3250,   # Verified (First Step, etc.)
    78000: 3500,   # Verified
}

# Target: 71190 (Table of Lost Grace / Roundtable Hold)
TARGET_FLAG = 71190
TARGET_NAME = "Table of Lost Grace (Roundtable Hold)"


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


def check_flag_at_base(ef_data, base, flag_id, block_start):
    """Check flag using block formula."""
    local = flag_id - block_start
    byte_offset = base + local // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset < len(ef_data):
        byte_val = ef_data[byte_offset]
        is_set = bool(byte_val & (1 << bit_pos))
        return is_set, byte_offset, bit_pos, byte_val
    return None, byte_offset, bit_pos, 0


def main():
    print("=" * 80)
    print(f"FIND ROUNDTABLE HOLD GRACE ({TARGET_FLAG})")
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

    # Check at block 71100 base (where it would be if contiguous)
    print(f"\n{'='*80}")
    print("CHECK AT BLOCK 71100 BASE (2593)")
    print("=" * 80)

    result = check_flag_at_base(ef_data_s0, 2593, TARGET_FLAG, 71100)
    is_set, byte_off, bit_pos, byte_val = result
    result_s1 = check_flag_at_base(ef_data_s1, 2593, TARGET_FLAG, 71100)

    print(f"\n{TARGET_FLAG} at base 2593 (block 71100):")
    print(f"  Byte offset: {byte_off}, Bit: {bit_pos}")
    print(f"  Byte value: 0x{byte_val:02X} ({byte_val:08b})")
    print(f"  S0: {'SET' if is_set else 'UNSET'}, S1: {'SET' if result_s1[0] else 'UNSET'}")

    # Search for flag 71190 using bit position
    print(f"\n{'='*80}")
    print(f"SEARCH FOR FLAG {TARGET_FLAG} (bit position {7 - (TARGET_FLAG % 8)})")
    print("=" * 80)

    # 71190 % 8 = 6, so bit position = 1
    target_bit = 7 - (TARGET_FLAG % 8)
    print(f"\nTarget bit position: {target_bit}")
    print("Searching for bytes where bit 1 is SET in S0 but not S1...")

    candidates = []
    for offset in range(0, 50000):
        s0_byte = ef_data_s0[offset]
        s1_byte = ef_data_s1[offset]

        # Bit 1 is SET in S0 but not in S1
        if (s0_byte & (1 << target_bit)) and not (s1_byte & (1 << target_bit)):
            # Check if this could be a grace flag byte
            candidates.append((offset, s0_byte, s1_byte))

    print(f"\nFound {len(candidates)} candidate bytes")

    # Try to narrow down by looking for bytes near other grace data
    print(f"\n{'='*80}")
    print("NARROW DOWN CANDIDATES")
    print("=" * 80)

    # The Roundtable Hold grace should be in an area with other Roundtable events
    # Area 18 (Roundtable Hold) base is 43487

    print("\nCandidates near Area 18 base (43487 ± 1500):")
    near_area_18 = [(o, s0, s1) for o, s0, s1 in candidates if 42000 <= o <= 45000]
    for offset, s0_byte, s1_byte in near_area_18[:20]:
        print(f"  Offset {offset}: S0=0x{s0_byte:02X}, S1=0x{s1_byte:02X}")

    # Also check candidates in grace range (2500-4000)
    print("\nCandidates in grace block range (2500-4000):")
    in_grace_range = [(o, s0, s1) for o, s0, s1 in candidates if 2500 <= o <= 4000]
    for offset, s0_byte, s1_byte in in_grace_range[:30]:
        # Calculate what flag ID this would correspond to for various block starts
        for block_start in [71000, 71100, 71800]:
            if offset >= GRACE_BLOCKS.get(block_start, 0):
                local = (offset - GRACE_BLOCKS.get(block_start, 0)) * 8 + (7 - target_bit)
                flag_id = block_start + local
                if flag_id == TARGET_FLAG:
                    print(f"  *** MATCH: Offset {offset} -> flag {flag_id} (block {block_start}) ***")
        print(f"  Offset {offset}: S0=0x{s0_byte:02X}, S1=0x{s1_byte:02X}")

    # Try block 71100 with different interpretations
    print(f"\n{'='*80}")
    print("TRY DIFFERENT BLOCK INTERPRETATIONS")
    print("=" * 80)

    # Maybe 71190 is in its own mini-block
    # Search for the specific byte where 71190 would be SET

    print("\nSearching all bases 0-50000 for flag 71190...")
    found_bases = []
    for test_base in range(0, 50000):
        # For each possible block start that could contain 71190
        for block_start in [71000, 71100, 71150, 71180, 71190]:
            if block_start <= TARGET_FLAG < block_start + 1000:
                local = TARGET_FLAG - block_start
                byte_offset = test_base + local // 8
                bit_pos = 7 - (TARGET_FLAG % 8)

                if byte_offset < len(ef_data_s0):
                    s0_set = bool(ef_data_s0[byte_offset] & (1 << bit_pos))
                    s1_set = bool(ef_data_s1[byte_offset] & (1 << bit_pos))

                    if s0_set and not s1_set:
                        found_bases.append((test_base, block_start, byte_offset, bit_pos))

    print(f"\nFound {len(found_bases)} possible (base, block_start) combinations")
    if found_bases:
        print("\nFirst 30 matches:")
        for base, block_start, byte_off, bit_pos in found_bases[:30]:
            print(f"  Base {base}, block_start {block_start} -> byte {byte_off}, bit {bit_pos}")

    # Check if Roundtable grace might use dungeon formula
    print(f"\n{'='*80}")
    print("CHECK DUNGEON FORMULA (Area 18, section 10)")
    print("=" * 80)

    # Area 18 base is 43487, section_size 1125
    # If 71190 is treated as dungeon flag format: 71SSS -> section 1, local 190?
    # That doesn't quite fit...

    # Alternative: check Area 11 (Leyndell) section 10 for Roundtable
    area_11_base = 8612  # From ground truth
    section_10_offset = area_11_base + 10 * 1125  # = 8612 + 11250 = 19862

    print(f"\nArea 11 section 10 would start at: {section_10_offset}")

    # Check bytes at that location
    print(f"\nBytes at Area 11 section 10 ({section_10_offset}):")
    for i in range(20):
        s0_byte = ef_data_s0[section_10_offset + i]
        s1_byte = ef_data_s1[section_10_offset + i]
        print(f"  Byte {section_10_offset + i}: S0=0x{s0_byte:02X}, S1=0x{s1_byte:02X}")


if __name__ == "__main__":
    main()
