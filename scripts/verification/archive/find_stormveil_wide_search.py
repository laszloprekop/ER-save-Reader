#!/usr/bin/env python3
"""
Wide search for Stormveil grace flags (block 71000) actual location.

The calculated base (2625) shows all zeros. The flags must be stored elsewhere.
We'll search the entire EventFlags region for patterns matching expected
Stormveil discovery state.

Expected state for mid-game Confessor:
- Should have discovered multiple Stormveil graces (4-7 likely)
- Flags 71000-71008 cover 9 graces

Search strategy: Look for byte patterns where bits match expected discovery pattern.
"""

import struct
from pathlib import Path

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010

SEARCH_START = 0x12000
FALLBACK_OFFSET = 0x12B00

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

NEGATIVE_VALIDATION_FLAGS = [
    (76223, 3277, 0, "Fortified Manor"),
    (76224, 3278, 7, "East Capital Rampart"),
    (76225, 3278, 6, "Divine Bridge"),
    (76300, 3287, 3, "Zamor Ruins"),
    (76301, 3287, 2, "Ancient Snow Valley"),
    (76350, 3293, 5, "Haligtree Town"),
]

def detect_event_flags_start(slot_data, search_start, fallback_offset):
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - 0x1bf99f)
    min_offset = 500
    actual_start = max(search_start, min_offset)

    candidates = []
    for test_offset in range(actual_start, search_end):
        positive_score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    positive_score += 1

        if positive_score == len(VALIDATION_FLAGS):
            negative_score = 0
            for flag_id, byte_offset, bit_pos, name in NEGATIVE_VALIDATION_FLAGS:
                abs_pos = test_offset + byte_offset
                if abs_pos < len(slot_data):
                    byte_val = slot_data[abs_pos]
                    if not (byte_val & (1 << bit_pos)):
                        negative_score += 1

            candidates.append((test_offset, negative_score))
            if negative_score == len(NEGATIVE_VALIDATION_FLAGS):
                return test_offset, positive_score, True

    if candidates:
        candidates.sort(key=lambda x: (-x[1], x[0]))
        best_offset, _ = candidates[0]
        return best_offset, len(VALIDATION_FLAGS), True

    return fallback_offset, 0, False

def read_slot_data(slot_index):
    with open(SAVE_FILE, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def count_bits(byte_val):
    return bin(byte_val).count('1')

def main():
    print("=" * 70)
    print("WIDE SEARCH FOR STORMVEIL GRACE FLAGS (BLOCK 71000)")
    print("=" * 70)

    slot0_data = read_slot_data(0)
    slot1_data = read_slot_data(1)

    ef_start_s0, _, _ = detect_event_flags_start(slot0_data, SEARCH_START, FALLBACK_OFFSET)
    ef_start_s1, _, _ = detect_event_flags_start(slot1_data, SEARCH_START, FALLBACK_OFFSET)

    print(f"\nDetected EF starts: Slot0=0x{ef_start_s0:X}, Slot1=0x{ef_start_s1:X}")

    # Extract EventFlags regions
    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]

    print(f"EventFlags sizes: S0={len(ef_s0):,}, S1={len(ef_s1):,}")

    # Strategy: Look for 2-byte windows where:
    # - S0 has 3-9 bits set (some Stormveil graces discovered)
    # - S1 has 0-2 bits set (early game, few graces)
    # - The pattern matches expected grace discovery

    print("\n" + "=" * 70)
    print("SEARCHING FOR PATTERNS MATCHING STORMVEIL GRACES")
    print("(Looking for 2-byte windows with differential bit patterns)")
    print("=" * 70)

    candidates = []

    # Search around the expected range (block 71000 should be near 71800's base)
    # 71800 at 2725, so 71000 should be around 2725 - (800/8) = 2625
    # But it's not there. Let's search a wider range.

    # Search the first 10000 bytes of EventFlags
    for base in range(0, min(10000, len(ef_s0) - 2)):
        # 9 graces fit in 2 bytes (flags 0-8 use bits 7,6,5,4,3,2,1,0 of byte 0 and bit 7 of byte 1)
        bits_s0 = count_bits(ef_s0[base]) + (1 if ef_s0[base+1] & 0x80 else 0)
        bits_s1 = count_bits(ef_s1[base]) + (1 if ef_s1[base+1] & 0x80 else 0)

        # Looking for: S0 has 3-9 bits, S1 has 0-2 bits, differential >= 3
        diff = bits_s0 - bits_s1
        if bits_s0 >= 3 and bits_s1 <= 2 and diff >= 3:
            candidates.append({
                'base': base,
                'bits_s0': bits_s0,
                'bits_s1': bits_s1,
                'diff': diff,
                'byte0_s0': ef_s0[base],
                'byte1_s0': ef_s0[base+1],
                'byte0_s1': ef_s1[base],
                'byte1_s1': ef_s1[base+1],
            })

    # Sort by differential (most difference = most likely)
    candidates.sort(key=lambda x: -x['diff'])

    print(f"\nFound {len(candidates)} candidate bases")
    print("\nTop 20 candidates:")
    print(f"{'Base':>6} | {'S0 bits':>7} | {'S1 bits':>7} | {'Diff':>4} | {'S0 byte0':>8} | {'S0 byte1':>8}")
    print("-" * 60)

    for c in candidates[:20]:
        print(f"{c['base']:>6} | {c['bits_s0']:>7} | {c['bits_s1']:>7} | {c['diff']:>4} | "
              f"0x{c['byte0_s0']:02X}      | 0x{c['byte1_s0']:02X}")

    # Now let's check each candidate to see if it could be block 71000
    print("\n" + "=" * 70)
    print("CHECKING TOP CANDIDATES AS BLOCK 71000 BASE")
    print("=" * 70)

    for c in candidates[:10]:
        base = c['base']
        print(f"\n  Base {base}:")

        # Decode what flags would be at this base
        # If this is block 71000 base, then:
        # - flag 71000 = byte base, bit 7
        # - flag 71001 = byte base, bit 6
        # - etc.

        flags = []
        for i in range(9):  # 9 Stormveil graces
            byte_idx = base + i // 8
            bit_pos = 7 - (i % 8)
            if byte_idx < len(ef_s0):
                val_s0 = bool(ef_s0[byte_idx] & (1 << bit_pos))
                val_s1 = bool(ef_s1[byte_idx] & (1 << bit_pos)) if byte_idx < len(ef_s1) else False
                flags.append((71000 + i, val_s0, val_s1))

        for flag_id, val_s0, val_s1 in flags:
            status_s0 = "SET" if val_s0 else "unset"
            status_s1 = "SET" if val_s1 else "unset"
            print(f"    {flag_id}: S0={status_s0}, S1={status_s1}")

    # Also check specifically around the expected area
    print("\n" + "=" * 70)
    print("CHECKING AREA AROUND EXPECTED BASE (2600-2700)")
    print("=" * 70)

    for base in range(2600, 2710, 5):
        if base + 2 >= len(ef_s0):
            continue

        byte0_s0 = ef_s0[base]
        byte1_s0 = ef_s0[base+1]
        byte0_s1 = ef_s1[base]
        byte1_s1 = ef_s1[base+1]

        if byte0_s0 != 0 or byte1_s0 != 0 or byte0_s1 != 0 or byte1_s1 != 0:
            print(f"  Base {base}: S0=[0x{byte0_s0:02X} 0x{byte1_s0:02X}] S1=[0x{byte0_s1:02X} 0x{byte1_s1:02X}]")

    # Final check: verify 71800 is working correctly
    print("\n" + "=" * 70)
    print("VERIFYING VALIDATION FLAG 71800 (Cave of Knowledge)")
    print("=" * 70)

    # 71800 at byte 2725, bit 7
    byte_2725_s0 = ef_s0[2725]
    byte_2725_s1 = ef_s1[2725]
    val_71800_s0 = bool(byte_2725_s0 & 0x80)
    val_71800_s1 = bool(byte_2725_s1 & 0x80)

    print(f"  Byte 2725: S0=0x{byte_2725_s0:02X}, S1=0x{byte_2725_s1:02X}")
    print(f"  71800 (Cave of Knowledge): S0={val_71800_s0}, S1={val_71800_s1}")

    # If 71800 works, let's trace backwards to find where 71000 should be
    # If 71800 = byte 2725 and belongs to block starting at 71000,
    # then base = 2725 - 100 = 2625

    print("\n  If 71800 belongs to block 71000:")
    print(f"    71800 local = 71800 - 71000 = 800")
    print(f"    byte = base + 800/8 = base + 100")
    print(f"    2725 = base + 100 → base = 2625")
    print(f"    So flag 71000 should be at byte 2625, bit 7")

    byte_2625_s0 = ef_s0[2625]
    byte_2625_s1 = ef_s1[2625]
    print(f"\n  Byte 2625: S0=0x{byte_2625_s0:02X}, S1=0x{byte_2625_s1:02X}")

    # Maybe blocks don't span 1000 flags. Let's check if 71000 has its own block
    print("\n" + "=" * 70)
    print("CHECKING IF 71000 HAS ITS OWN BLOCK (SEPARATE FROM 71800)")
    print("=" * 70)

    # If 71000-71099 is its own block, and 71800-71899 is another block,
    # they would have different bases.

    # Let's search for any block that could contain 71000-71008
    # Try different block structures

    block_hypotheses = [
        (71000, 2625, "Block 71000 (calculated from 71800)"),
        (71000, 2612, "Block 71000 (separate, offset -13)"),
        (71000, 2725 - 100, "Recalculated"),
        (71000, 2700, "Round number guess"),
        (71000, 2600, "Round number guess 2"),
    ]

    for block_start, base, desc in block_hypotheses:
        if base < 0 or base + 2 >= len(ef_s0):
            continue

        print(f"\n  Testing: {desc}")
        print(f"    Block start: {block_start}, Base: {base}")

        any_set = False
        for i in range(9):
            flag_id = 71000 + i
            local = flag_id - block_start
            byte_offset = base + local // 8
            bit_pos = 7 - (local % 8)

            if byte_offset < len(ef_s0):
                val_s0 = bool(ef_s0[byte_offset] & (1 << bit_pos))
                if val_s0:
                    any_set = True
                    print(f"      {flag_id}: SET in S0")

        if not any_set:
            print(f"      (all flags UNSET)")

if __name__ == "__main__":
    main()
