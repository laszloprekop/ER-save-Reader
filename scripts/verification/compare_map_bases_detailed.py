#!/usr/bin/env python3
"""
Compare map bases between snapshot and current save, accounting for offset differences.
"""

from typing import Optional

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
]

CURRENT_SAVE = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"
AFTER_SNAPSHOT = "/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/ER0000.sl2 Wretch - 34 Limgrave Map picked, moved to south of Wayward cellar sarchophagi"

MAP_FLAGS = [
    (62010, "Map: Limgrave, West"),
    (62011, "Map: Weeping Peninsula"),
    (62012, "Map: Limgrave, East"),
]


def detect_event_flags_offset(slot_data: bytes) -> Optional[int]:
    for test_offset in range(0x12000, 0x15000):
        score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if (byte_val & (1 << bit_pos)) != 0:
                    score += 1
        if score == len(VALIDATION_FLAGS):
            return test_offset
    return None


def check_flag(event_flags: bytes, flag_id: int, block_start: int, base: int) -> Optional[bool]:
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset >= len(event_flags) or byte_offset < 0:
        return None

    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


def load_slot(path: str, slot: int = 1):
    with open(path, 'rb') as f:
        f.seek(HEADER_SIZE + slot * SLOT_SIZE)
        return f.read(SLOT_SIZE)


def main():
    print("="*70)
    print("DETAILED BASE COMPARISON")
    print("="*70)

    # Load current save - Slot 0 (Confessor) and Slot 1 (Wretch)
    print("\n--- CURRENT SAVE ---")
    current_slot0 = load_slot(CURRENT_SAVE, slot=0)
    current_slot1 = load_slot(CURRENT_SAVE, slot=1)

    current_offset_0 = detect_event_flags_offset(current_slot0)
    current_offset_1 = detect_event_flags_offset(current_slot1)
    print(f"Slot 0 event_flags offset: 0x{current_offset_0:X} ({current_offset_0})")
    print(f"Slot 1 event_flags offset: 0x{current_offset_1:X} ({current_offset_1})")

    ef_current_0 = current_slot0[current_offset_0:]
    ef_current_1 = current_slot1[current_offset_1:]

    # Load after snapshot - Slot 1 (Wretch after map pickup)
    print("\n--- AFTER SNAPSHOT ---")
    after_slot1 = load_slot(AFTER_SNAPSHOT, slot=1)
    after_offset_1 = detect_event_flags_offset(after_slot1)
    print(f"Slot 1 event_flags offset: 0x{after_offset_1:X} ({after_offset_1})")

    ef_after_1 = after_slot1[after_offset_1:]

    offset_diff = after_offset_1 - current_offset_1
    print(f"\nOffset difference (after - current): {offset_diff} bytes")

    # Test both bases
    print("\n" + "="*70)
    print("TESTING BASES")
    print("="*70)

    for base, base_name in [(9359, "Our discovered base"), (28407, "From temporal diff")]:
        print(f"\n--- BASE {base} ({base_name}) ---")

        print("\nCurrent save Slot 0 (Confessor - should have maps):")
        for flag_id, name in MAP_FLAGS:
            result = check_flag(ef_current_0, flag_id, 62000, base)
            status = "SET" if result else "---" if result is False else "N/A"
            rel = flag_id - 62000
            byte_off = base + rel // 8
            bit_pos = 7 - (rel % 8)
            print(f"  {flag_id} {name:25} {status:3} (byte {byte_off}, bit {bit_pos})")

        print("\nCurrent save Slot 1 (Wretch - early game):")
        for flag_id, name in MAP_FLAGS:
            result = check_flag(ef_current_1, flag_id, 62000, base)
            status = "SET" if result else "---" if result is False else "N/A"
            print(f"  {flag_id} {name:25} {status:3}")

        print("\nAfter snapshot Slot 1 (Wretch - just picked up Limgrave map):")
        for flag_id, name in MAP_FLAGS:
            result = check_flag(ef_after_1, flag_id, 62000, base)
            status = "SET" if result else "---" if result is False else "N/A"
            print(f"  {flag_id} {name:25} {status:3}")

        # Also try with offset adjustment
        adjusted_base = base + offset_diff
        print(f"\nWith offset adjustment ({base} + {offset_diff} = {adjusted_base}):")
        print("After snapshot Slot 1:")
        for flag_id, name in MAP_FLAGS:
            result = check_flag(ef_after_1, flag_id, 62000, adjusted_base)
            status = "SET" if result else "---" if result is False else "N/A"
            print(f"  {flag_id} {name:25} {status:3}")


if __name__ == "__main__":
    main()
