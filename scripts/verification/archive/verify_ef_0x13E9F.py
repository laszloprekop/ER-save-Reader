#!/usr/bin/env python3
"""
Verify EF offset 0x13E9F with base 2731 for block 71000.
"""

from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
BACKUP_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

NEGATIVE_FLAGS = [
    (76300, 3287, 3, "Zamor Ruins"),
    (76301, 3287, 2, "Ancient Snow Valley"),
    (76350, 3293, 5, "Haligtree Town"),
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

def main():
    print("=" * 70)
    print("VERIFY EF OFFSET 0x13E9F")
    print("=" * 70)

    slot0_data = read_slot_data(BACKUP_FILE, 0)
    ef_offset = 0x13E9F

    print(f"\nEF offset: 0x{ef_offset:X}")

    # Verify validation flags
    print("\n" + "=" * 70)
    print("VALIDATION FLAGS")
    print("=" * 70)

    for flag_id, byte_off, bit_pos, name in VALIDATION_FLAGS:
        abs_pos = ef_offset + byte_off
        byte_val = slot0_data[abs_pos]
        is_set = bool(byte_val & (1 << bit_pos))
        print(f"  {flag_id} ({name:20s}): byte 0x{byte_val:02X}, bit {bit_pos} = {is_set}")

    # Check negative flags
    print("\n" + "=" * 70)
    print("NEGATIVE FLAGS (should be UNSET for mid-game)")
    print("=" * 70)

    for flag_id, byte_off, bit_pos, name in NEGATIVE_FLAGS:
        abs_pos = ef_offset + byte_off
        byte_val = slot0_data[abs_pos]
        is_set = bool(byte_val & (1 << bit_pos))
        print(f"  {flag_id} ({name:20s}): byte 0x{byte_val:02X}, bit {bit_pos} = {is_set}")

    # Check Stormveil graces at base 2731
    print("\n" + "=" * 70)
    print("STORMVEIL GRACES AT BASE 2731")
    print("=" * 70)

    base = 2731

    # Show raw bytes first
    print(f"\nRaw bytes at base {base}:")
    for i in range(4):
        abs_pos = ef_offset + base + i
        byte_val = slot0_data[abs_pos]
        print(f"  Byte {base + i}: 0x{byte_val:02X} ({byte_val:08b})")

    print(f"\nGrace flags:")
    for flag_id, name in STORMVEIL_GRACES:
        local = flag_id - 71000
        byte_off = base + local // 8
        bit_pos = 7 - (local % 8)

        abs_pos = ef_offset + byte_off
        byte_val = slot0_data[abs_pos]
        is_set = bool(byte_val & (1 << bit_pos))

        status = "SET" if is_set else "unset"
        print(f"  {flag_id} ({name:25s}): byte {byte_off}, bit {bit_pos} = {status}")
        print(f"    (byte value: 0x{byte_val:02X})")

    # Now compare with the ORIGINAL base 2673
    print("\n" + "=" * 70)
    print("COMPARISON: BASE 2731 vs BASE 2673")
    print("=" * 70)

    # The relationship: 2731 - 2673 = 58 bytes offset
    print(f"\nBase difference: 2731 - 2673 = 58 bytes")
    print(f"This means: If base 2731 is correct, our original calculation was off by 58 bytes")

    # Check what this means for block structure
    # Original: 71800 at byte 2725, so base 71000 = 2725 - 100 = 2625
    # But 2625 was wrong, we found 2673 (48 bytes more)
    # Now we find 2731 (106 bytes more than 2625)

    print("\nBlock calculation review:")
    print("  Original calculation: 71800 at 2725, so 71000 base = 2625")
    print("  We found base 2673 earlier (48 bytes more)")
    print("  Now finding base 2731 works (106 bytes more than 2625)")

    # Check if 71800 still works with adjusted understanding
    print("\n" + "=" * 70)
    print("VERIFYING 71800 BLOCK CONSISTENCY")
    print("=" * 70)

    # If 71000 base is 2731, then 71800 would be at:
    # 71800 - 71000 = 800 flags = 100 bytes
    # So 71800 base = 2731 + 100 = 2831? But we know it's at 2725!

    print(f"\nIf block 71000 base = 2731:")
    print(f"  71800 local = 800, byte offset = 100")
    print(f"  71800 would be at byte 2731 + 100 = 2831")
    print(f"  But validation says 71800 is at byte 2725!")

    # Check what's at byte 2831
    abs_pos_2831 = ef_offset + 2831
    byte_2831 = slot0_data[abs_pos_2831]
    print(f"\n  Byte 2831 value: 0x{byte_2831:02X} ({byte_2831:08b})")
    print(f"  Bit 7 (where 71800 would be): {bool(byte_2831 & 0x80)}")

    # And check byte 2725 directly
    abs_pos_2725 = ef_offset + 2725
    byte_2725 = slot0_data[abs_pos_2725]
    print(f"\n  Byte 2725 value (where validation says 71800 is): 0x{byte_2725:02X}")
    print(f"  Bit 7: {bool(byte_2725 & 0x80)}")
    print(f"  Bit 6: {bool(byte_2725 & 0x40)}")

    # This suggests blocks 71000 and 71800 are NOT contiguous!
    print("\n" + "=" * 70)
    print("CONCLUSION")
    print("=" * 70)
    print("""
If both are true:
- 71000-71008 (Stormveil) at base 2731
- 71800-71801 (Tutorial) at byte 2725

Then these are SEPARATE blocks stored at DIFFERENT locations!
Block 71000 is NOT contiguous with block 71800.
""")

if __name__ == "__main__":
    main()
