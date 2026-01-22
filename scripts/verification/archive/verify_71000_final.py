#!/usr/bin/env python3
"""
Final verification of Block 71000 with corrected base 9315.
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

NEW_BASE = 9315  # Corrected base

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

def check_flag(ef_data, byte_offset, bit_pos):
    if byte_offset < len(ef_data):
        return bool(ef_data[byte_offset] & (1 << bit_pos))
    return None

def main():
    print("=" * 70)
    print("FINAL VERIFICATION: BLOCK 71000 WITH BASE 9315")
    print("=" * 70)

    slot0_data = read_slot_data(BACKUP_FILE, 0)
    slot1_data = read_slot_data(BACKUP_FILE, 1)

    ef_start_s0 = detect_event_flags_start(slot0_data, SEARCH_START)
    ef_start_s1 = detect_event_flags_start(slot1_data, SEARCH_START)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_data_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_data_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]

    print(f"\nSlot 0 EF start: 0x{ef_start_s0:X}")
    print(f"Slot 1 EF start: 0x{ef_start_s1:X}")

    print(f"\n{'='*70}")
    print("STORMVEIL GRACES AT BASE 9315")
    print("='*70")

    print("\n{:<8} {:<30} {:>8} {:>8}".format("Flag ID", "Name", "Slot 0", "Slot 1"))
    print("-" * 60)

    s0_count = 0
    s1_count = 0

    for flag_id, name in STORMVEIL_GRACES:
        local = flag_id - 71000
        byte_offset = NEW_BASE + local // 8
        bit_pos = 7 - (local % 8)

        val_s0 = check_flag(ef_data_s0, byte_offset, bit_pos)
        val_s1 = check_flag(ef_data_s1, byte_offset, bit_pos)

        s0_status = "SET" if val_s0 else "unset"
        s1_status = "SET" if val_s1 else "unset"

        if val_s0:
            s0_count += 1
        if val_s1:
            s1_count += 1

        highlight = " <-- MAIN GATE" if flag_id == 71008 else ""
        print(f"{flag_id:<8} {name:<30} {s0_status:>8} {s1_status:>8}{highlight}")

    print("-" * 60)
    print(f"{'TOTAL':<8} {'':<30} {s0_count}/9{'':<5} {s1_count}/9")

    print(f"\n{'='*70}")
    print("VERIFICATION RESULT")
    print("='*70")

    if s0_count >= 7 and s1_count == 0:
        print("\n✓ SUCCESS: Differential pattern matches expectations!")
        print("  - Slot 0 (mid-game): Most graces SET")
        print("  - Slot 1 (early-game): No graces SET")
        print("\n✓ Block 71000 base 9315 VERIFIED")
    else:
        print(f"\n⚠ Unexpected pattern: S0={s0_count}/9, S1={s1_count}/9")

    # Show raw bytes
    print(f"\n{'='*70}")
    print("RAW BYTES AT BASE 9315")
    print("='*70")

    print("\nSlot 0:")
    for i in range(3):
        byte_val = ef_data_s0[NEW_BASE + i]
        print(f"  Byte {NEW_BASE + i}: 0x{byte_val:02X} ({byte_val:08b})")

    print("\nSlot 1:")
    for i in range(3):
        byte_val = ef_data_s1[NEW_BASE + i]
        print(f"  Byte {NEW_BASE + i}: 0x{byte_val:02X} ({byte_val:08b})")

if __name__ == "__main__":
    main()
