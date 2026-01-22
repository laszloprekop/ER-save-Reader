#!/usr/bin/env python3
"""
Verify Area 16 (Volcano Manor) at candidate base 40517.

Known facts:
- User killed Abductor Virgin boss (teleport trap escape) but NOT Rykard
- Abductor Virgin flag: 16000860 (local 860)
- Rykard flag: 16000800 (local 800)
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

# Volcano Manor dungeon flags (16SSSCCC format: SS=section, CCC=local)
# Section 00 is the main manor area
VOLCANO_MANOR_FLAGS = [
    # Boss flags
    (16000800, "Rykard, Lord of Blasphemy", "boss"),
    (16000850, "God-Devouring Serpent (phase 1)", "boss"),
    (16000860, "Abductor Virgins (teleport trap escape)", "boss"),

    # Grace flags (typically xxx9xx pattern)
    (16000900, "Volcano Manor (grace)", "grace"),
    (16000901, "Temple of Eiglay (grace)", "grace"),
    (16000902, "Guest Hall (grace)", "grace"),
    (16000903, "Prison Town Church (grace)", "grace"),
    (16000904, "Subterranean Inquisition Chamber", "grace"),

    # Item pickup flags
    (16000100, "Item pickup 100", "item"),
    (16000200, "Item pickup 200", "item"),
    (16000300, "Item pickup 300", "item"),
    (16000400, "Item pickup 400", "item"),
    (16000500, "Item pickup 500", "item"),
]

CANDIDATE_BASE = 40517
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
    # Flag format: AASSSCCC where AA=area, SSS=section, CCC=local
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
    print(f"VERIFY AREA 16 (VOLCANO MANOR) AT BASE {CANDIDATE_BASE}")
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

    # Show raw bytes at candidate base
    print(f"\n{'='*80}")
    print(f"RAW BYTES AT BASE {CANDIDATE_BASE} (first 20 bytes)")
    print("=" * 80)

    print("\nSlot 0:")
    for i in range(20):
        byte_val = ef_data_s0[CANDIDATE_BASE + i]
        print(f"  Byte {CANDIDATE_BASE + i}: 0x{byte_val:02X} ({byte_val:08b})")

    print("\nSlot 1:")
    for i in range(20):
        byte_val = ef_data_s1[CANDIDATE_BASE + i]
        print(f"  Byte {CANDIDATE_BASE + i}: 0x{byte_val:02X} ({byte_val:08b})")

    # Check each flag
    print(f"\n{'='*80}")
    print(f"VOLCANO MANOR FLAGS AT BASE {CANDIDATE_BASE}")
    print("=" * 80)

    print(f"\n{'Flag ID':<12} {'Name':<40} {'Type':<6} {'S0':>6} {'S1':>6} {'Byte':>8} {'Sec':>4} {'Loc':>4}")
    print("-" * 100)

    s0_set = []
    for flag_id, name, flag_type in VOLCANO_MANOR_FLAGS:
        result_s0 = check_dungeon_flag(ef_data_s0, CANDIDATE_BASE, flag_id)
        result_s1 = check_dungeon_flag(ef_data_s1, CANDIDATE_BASE, flag_id)

        is_set_s0, byte_off, bit_pos, byte_val, section, local = result_s0
        is_set_s1 = result_s1[0]

        s0_status = "SET" if is_set_s0 else "unset"
        s1_status = "SET" if is_set_s1 else "unset"

        if is_set_s0:
            s0_set.append((flag_id, name, flag_type))

        marker = ""
        if flag_id == 16000860:
            marker = " <-- Abductor (should be SET)"
        elif flag_id == 16000800:
            marker = " <-- Rykard (should be UNSET)"

        print(f"{flag_id:<12} {name:<40} {flag_type:<6} {s0_status:>6} {s1_status:>6} {byte_off:>8} {section:>4} {local:>4}{marker}")

    # Scan for any set bits in section 0
    print(f"\n{'='*80}")
    print(f"SCAN SECTION 0 FOR SET BITS (base {CANDIDATE_BASE}, {SECTION_SIZE} bytes)")
    print("=" * 80)

    set_bits_s0 = []
    set_bits_s1 = []

    for i in range(SECTION_SIZE):
        byte_s0 = ef_data_s0[CANDIDATE_BASE + i]
        byte_s1 = ef_data_s1[CANDIDATE_BASE + i]

        for bit in range(8):
            if byte_s0 & (1 << bit):
                local = i * 8 + (7 - bit)
                set_bits_s0.append((CANDIDATE_BASE + i, bit, local))
            if byte_s1 & (1 << bit):
                local = i * 8 + (7 - bit)
                set_bits_s1.append((CANDIDATE_BASE + i, bit, local))

    print(f"\nSlot 0: {len(set_bits_s0)} bits SET in section 0")
    if set_bits_s0:
        print("First 30 set bits:")
        for byte_off, bit_pos, local in set_bits_s0[:30]:
            flag_id = 16000000 + local
            print(f"  Byte {byte_off}, bit {bit_pos} -> local {local} -> flag {flag_id}")

    print(f"\nSlot 1: {len(set_bits_s1)} bits SET in section 0")
    if set_bits_s1:
        print("First 10 set bits:")
        for byte_off, bit_pos, local in set_bits_s1[:10]:
            flag_id = 16000000 + local
            print(f"  Byte {byte_off}, bit {bit_pos} -> local {local} -> flag {flag_id}")

    # Search for alternative bases
    print(f"\n{'='*80}")
    print("SEARCH FOR ABDUCTOR VIRGIN FLAG (16000860)")
    print("=" * 80)

    # Flag 16000860 -> section 0, local 860
    # At correct base: byte = base + 860//8 = base + 107, bit = 7 - (860 % 8) = 7 - 4 = 3
    target_local = 860
    target_byte_offset = target_local // 8  # 107
    target_bit = 7 - (target_local % 8)  # 3

    print(f"\nLooking for local 860 (byte offset {target_byte_offset} from base, bit {target_bit})")
    print("Searching bases 35000-50000...")

    candidates = []
    for test_base in range(35000, 50000):
        byte_addr = test_base + target_byte_offset
        if byte_addr < len(ef_data_s0):
            s0_byte = ef_data_s0[byte_addr]
            s1_byte = ef_data_s1[byte_addr]

            s0_set = bool(s0_byte & (1 << target_bit))
            s1_set = bool(s1_byte & (1 << target_bit))

            if s0_set and not s1_set:
                # Check if Rykard (local 800) is NOT set
                rykard_byte = test_base + 800 // 8
                rykard_bit = 7 - (800 % 8)
                rykard_set = bool(ef_data_s0[rykard_byte] & (1 << rykard_bit))

                if not rykard_set:
                    candidates.append((test_base, s0_byte, s1_byte))

    print(f"\nFound {len(candidates)} candidates where local 860 SET in S0, UNSET in S1, and local 800 UNSET")
    if candidates:
        print("\nFirst 20 candidates:")
        for base, s0_byte, s1_byte in candidates[:20]:
            print(f"  Base {base}: byte 0x{s0_byte:02X}")

    # Analysis
    print(f"\n{'='*80}")
    print("ANALYSIS")
    print("=" * 80)

    if CANDIDATE_BASE in [c[0] for c in candidates]:
        print(f"\n✓ Base {CANDIDATE_BASE} IS a valid candidate for Volcano Manor")
    else:
        print(f"\n✗ Base {CANDIDATE_BASE} does NOT match expected pattern")

    # Check specific expected flags
    abductor_result = check_dungeon_flag(ef_data_s0, CANDIDATE_BASE, 16000860)
    rykard_result = check_dungeon_flag(ef_data_s0, CANDIDATE_BASE, 16000800)

    print(f"\nAt base {CANDIDATE_BASE}:")
    print(f"  Abductor Virgins (16000860): {'SET' if abductor_result[0] else 'UNSET'}")
    print(f"  Rykard (16000800): {'SET' if rykard_result[0] else 'UNSET'}")

    if abductor_result[0] and not rykard_result[0]:
        print("\n✓ Pattern matches: Abductor SET, Rykard UNSET")
        print("  This is consistent with teleport trap escape without Rykard kill")
    elif not abductor_result[0]:
        print("\n✗ Abductor UNSET - base may be wrong")


if __name__ == "__main__":
    main()
