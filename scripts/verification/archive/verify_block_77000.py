#!/usr/bin/env python3
"""
Verify Block 77000 (Extended world graces) at calculated base 3373.
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

CANDIDATE_BASE = 3373
BLOCK_START = 77000
BLOCK_SIZE = 1000


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


def check_flag(ef_data, base, flag_id, block_start):
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
    print(f"VERIFY BLOCK {BLOCK_START} AT BASE {CANDIDATE_BASE}")
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

    # Show raw bytes
    print(f"\n{'='*80}")
    print(f"RAW BYTES AT BASE {CANDIDATE_BASE}")
    print("=" * 80)

    print("\nSlot 0:")
    for i in range(20):
        byte_val = ef_data_s0[CANDIDATE_BASE + i]
        print(f"  Byte {CANDIDATE_BASE + i}: 0x{byte_val:02X} ({byte_val:08b})")

    print("\nSlot 1:")
    for i in range(20):
        byte_val = ef_data_s1[CANDIDATE_BASE + i]
        print(f"  Byte {CANDIDATE_BASE + i}: 0x{byte_val:02X} ({byte_val:08b})")

    # Scan for set flags
    print(f"\n{'='*80}")
    print(f"SCAN FOR SET FLAGS IN BLOCK {BLOCK_START}")
    print("=" * 80)

    block_bytes = BLOCK_SIZE // 8 + 1

    s0_set_flags = []
    s1_set_flags = []

    for local in range(BLOCK_SIZE):
        flag_id = BLOCK_START + local
        is_set_s0, byte_off, bit_pos, _ = check_flag(ef_data_s0, CANDIDATE_BASE, flag_id, BLOCK_START)
        is_set_s1, _, _, _ = check_flag(ef_data_s1, CANDIDATE_BASE, flag_id, BLOCK_START)

        if is_set_s0:
            s0_set_flags.append((flag_id, local, byte_off, bit_pos))
        if is_set_s1:
            s1_set_flags.append((flag_id, local, byte_off, bit_pos))

    print(f"\nSlot 0: {len(s0_set_flags)} flags SET")
    if s0_set_flags:
        print("First 20 SET flags:")
        for flag_id, local, byte_off, bit_pos in s0_set_flags[:20]:
            print(f"  {flag_id} (local {local}): byte {byte_off}, bit {bit_pos}")

    print(f"\nSlot 1: {len(s1_set_flags)} flags SET")
    if s1_set_flags:
        print("First 10 SET flags:")
        for flag_id, local, byte_off, bit_pos in s1_set_flags[:10]:
            print(f"  {flag_id} (local {local}): byte {byte_off}, bit {bit_pos}")

    # Sparsity analysis
    print(f"\n{'='*80}")
    print("SPARSITY ANALYSIS")
    print("=" * 80)

    total_s0 = sum(bin(ef_data_s0[CANDIDATE_BASE + i]).count('1') for i in range(block_bytes) if CANDIDATE_BASE + i < len(ef_data_s0))
    total_s1 = sum(bin(ef_data_s1[CANDIDATE_BASE + i]).count('1') for i in range(block_bytes) if CANDIDATE_BASE + i < len(ef_data_s1))

    print(f"\nTotal SET bits: S0={total_s0}, S1={total_s1}")
    print(f"Density: S0={total_s0/(block_bytes*8)*100:.2f}%, S1={total_s1/(block_bytes*8)*100:.2f}%")

    # Analysis
    print(f"\n{'='*80}")
    print("ANALYSIS")
    print("=" * 80)

    if len(s0_set_flags) > 0 and len(s1_set_flags) == 0:
        print(f"\n✓ Good differential: {len(s0_set_flags)} flags in S0, 0 in S1")
    elif len(s0_set_flags) > len(s1_set_flags) * 2:
        print(f"\n~ Partial differential: S0={len(s0_set_flags)}, S1={len(s1_set_flags)}")
    else:
        print(f"\n? No clear differential or inverted: S0={len(s0_set_flags)}, S1={len(s1_set_flags)}")


if __name__ == "__main__":
    main()
