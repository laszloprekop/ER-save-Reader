#!/usr/bin/env python3
"""
Find the actual Volcano Manor boss flags by scanning for 8xx patterns.
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

# Candidate bases
CANDIDATE_BASES = [40517, 42687, 36737]  # Current, alternative, original formula
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


def scan_section_for_8xx(ef_data_s0, ef_data_s1, base):
    """Scan for set bits in the 800-899 range (boss flags typically)."""
    results = []

    # Local 800-899 -> byte offsets 100-112 from base
    for local in range(800, 900):
        byte_off = base + local // 8
        bit_pos = 7 - (local % 8)

        if byte_off < len(ef_data_s0):
            s0_set = bool(ef_data_s0[byte_off] & (1 << bit_pos))
            s1_set = bool(ef_data_s1[byte_off] & (1 << bit_pos))

            if s0_set or s1_set:
                results.append((local, s0_set, s1_set, byte_off, bit_pos))

    return results


def scan_section_for_9xx(ef_data_s0, ef_data_s1, base):
    """Scan for set bits in the 900-999 range (grace flags typically)."""
    results = []

    for local in range(900, 1000):
        byte_off = base + local // 8
        bit_pos = 7 - (local % 8)

        if byte_off < len(ef_data_s0):
            s0_set = bool(ef_data_s0[byte_off] & (1 << bit_pos))
            s1_set = bool(ef_data_s1[byte_off] & (1 << bit_pos))

            if s0_set or s1_set:
                results.append((local, s0_set, s1_set, byte_off, bit_pos))

    return results


def main():
    print("=" * 80)
    print("FIND VOLCANO MANOR BOSS FLAGS")
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

    for base in CANDIDATE_BASES:
        print(f"\n{'='*80}")
        print(f"BASE {base}")
        print("=" * 80)

        # Count total set bits in section 0
        total_s0 = 0
        total_s1 = 0
        for i in range(SECTION_SIZE):
            if base + i < len(ef_data_s0):
                total_s0 += bin(ef_data_s0[base + i]).count('1')
                total_s1 += bin(ef_data_s1[base + i]).count('1')

        print(f"\nTotal SET bits in section 0: S0={total_s0}, S1={total_s1}")

        # Scan 8xx range (boss flags)
        print(f"\n--- 8xx range (boss flags) ---")
        results_8xx = scan_section_for_8xx(ef_data_s0, ef_data_s1, base)

        if results_8xx:
            print(f"Found {len(results_8xx)} set flags:")
            for local, s0, s1, byte_off, bit_pos in results_8xx:
                flag_id = 16000000 + local
                s0_str = "SET" if s0 else "unset"
                s1_str = "SET" if s1 else "unset"

                # Identify known boss flags
                name = ""
                if local == 800:
                    name = "(Rykard)"
                elif local == 850:
                    name = "(God-Devouring Serpent)"
                elif local == 860:
                    name = "(Abductor Virgins)"

                print(f"  Local {local} -> {flag_id}: S0={s0_str}, S1={s1_str} {name}")
        else:
            print("No flags SET in 8xx range")

        # Scan 9xx range (grace flags)
        print(f"\n--- 9xx range (grace flags) ---")
        results_9xx = scan_section_for_9xx(ef_data_s0, ef_data_s1, base)

        if results_9xx:
            print(f"Found {len(results_9xx)} set flags:")
            for local, s0, s1, byte_off, bit_pos in results_9xx:
                flag_id = 16000000 + local
                s0_str = "SET" if s0 else "unset"
                s1_str = "SET" if s1 else "unset"

                name = ""
                if local == 900:
                    name = "(Volcano Manor)"
                elif local == 901:
                    name = "(Temple of Eiglay)"
                elif local == 902:
                    name = "(Guest Hall)"
                elif local == 903:
                    name = "(Prison Town Church)"
                elif local == 904:
                    name = "(Subterranean Inquisition Chamber)"

                print(f"  Local {local} -> {flag_id}: S0={s0_str}, S1={s1_str} {name}")
        else:
            print("No flags SET in 9xx range")

    # Search for the Abductor defeat by finding where bit pattern matches
    print(f"\n{'='*80}")
    print("SEARCH FOR ABDUCTOR VIRGIN DEFEAT PATTERN")
    print("=" * 80)

    print("\nSearching for any 8xx local flag that is SET in S0 but NOT in S1...")
    print("(This would indicate a boss defeated in Slot 0 but not Slot 1)")

    for base in range(35000, 50000, 100):  # Sample every 100 bytes
        results = scan_section_for_8xx(ef_data_s0, ef_data_s1, base)
        # Look for flags SET in S0 but not S1
        s0_only = [(l, s0, s1) for l, s0, s1, _, _ in results if s0 and not s1]
        if s0_only:
            # Check if total activity suggests this is a real section
            total_s0 = sum(bin(ef_data_s0[base + i]).count('1') for i in range(min(200, SECTION_SIZE)) if base + i < len(ef_data_s0))
            total_s1 = sum(bin(ef_data_s1[base + i]).count('1') for i in range(min(200, SECTION_SIZE)) if base + i < len(ef_data_s1))

            if total_s0 > 50 and total_s1 < 10:  # Significant differential
                print(f"\n  Base {base}: {len(s0_only)} boss flags SET in S0 only (total: S0={total_s0}, S1={total_s1})")
                for local, s0, s1 in s0_only[:5]:
                    print(f"    Local {local} -> flag 16000{local}")


if __name__ == "__main__":
    main()
