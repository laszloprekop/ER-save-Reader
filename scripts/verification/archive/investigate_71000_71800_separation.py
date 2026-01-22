#!/usr/bin/env python3
"""
Investigate if blocks 71000 and 71800 are stored separately.

The calculation 71800 -> base 2625 -> 71000 gives zeros.
This suggests 71000 and 71800 might be in DIFFERENT storage regions.

Let's check the game data to understand the block structure better.
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
]

def detect_event_flags_start(slot_data, search_start, fallback_offset):
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - 0x1bf99f)
    min_offset = 500
    actual_start = max(search_start, min_offset)

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

            if negative_score == len(NEGATIVE_VALIDATION_FLAGS):
                return test_offset

    return fallback_offset

def read_slot_data(slot_index):
    with open(SAVE_FILE, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        return f.read(SLOT_SIZE)

def check_flag(ef_data, byte_offset, bit_pos):
    if byte_offset < len(ef_data):
        return bool(ef_data[byte_offset] & (1 << bit_pos))
    return None

def main():
    print("=" * 70)
    print("INVESTIGATION: 71000 vs 71800 BLOCK SEPARATION")
    print("=" * 70)

    slot0_data = read_slot_data(0)
    slot1_data = read_slot_data(1)

    ef_start_s0 = detect_event_flags_start(slot0_data, SEARCH_START, FALLBACK_OFFSET)
    ef_start_s1 = detect_event_flags_start(slot1_data, SEARCH_START, FALLBACK_OFFSET)

    EVENT_FLAGS_SIZE = 0x1bf99f
    ef_s0 = slot0_data[ef_start_s0:ef_start_s0 + EVENT_FLAGS_SIZE]
    ef_s1 = slot1_data[ef_start_s1:ef_start_s1 + EVENT_FLAGS_SIZE]

    print(f"\nDetected EF starts: S0=0x{ef_start_s0:X}, S1=0x{ef_start_s1:X}")

    # First, let's understand the verified grace blocks
    print("\n" + "=" * 70)
    print("VERIFIED GRACE BLOCKS FROM VALIDATION FLAGS")
    print("=" * 70)

    # Block analysis: flag_id -> byte_offset mapping
    # 71800 at byte 2725 → block 71800-71899 at base 2725
    # OR block 71000-71999 at base 2625 (71800 local=800, byte=2625+100=2725)

    # 76100 at byte 3262 → block 76000-76999?
    # local = 76100 - 76000 = 100
    # byte = base + 12 (100/8=12.5, floor=12)
    # 3262 = base + 12 → base = 3250

    print("\nBlock 71800 (Tutorial):")
    print("  71800: byte 2725, bit 7 (Cave of Knowledge)")
    print("  71801: byte 2725, bit 6 (Stranded Graveyard)")

    print("\nBlock 76000 (Limgrave Overworld):")
    print("  76100: byte 3262, bit 3 (The First Step)")
    print("  76101: byte 3262, bit 2 (Church of Elleh)")
    print("  Calculated base = 3262 - 12 = 3250")

    # Verify block 76000 graces
    print("\n" + "=" * 70)
    print("VERIFYING BLOCK 76000 (LIMGRAVE) PATTERN")
    print("=" * 70)

    limgrave_graces = [
        (76100, "The First Step"),
        (76101, "Church of Elleh"),
        (76102, "Gatefront Ruins"),
        (76103, "Artist's Shack"),
        (76104, "Warmaster's Shack"),
        (76105, "Stormhill Shack"),
        (76106, "Agheel Lake North"),
        (76107, "Agheel Lake South"),
        (76108, "Seaside Ruins"),
    ]

    base_76000 = 3250
    print(f"\nBase for block 76000: {base_76000}")

    for flag_id, name in limgrave_graces:
        local = flag_id - 76000
        byte_offset = base_76000 + local // 8
        bit_pos = 7 - (local % 8)

        val_s0 = check_flag(ef_s0, byte_offset, bit_pos)
        val_s1 = check_flag(ef_s1, byte_offset, bit_pos)
        status_s0 = "SET" if val_s0 else "unset"
        status_s1 = "SET" if val_s1 else "unset"
        print(f"  {flag_id} ({name:25s}): S0={status_s0}, S1={status_s1}")

    # Now let's check if 71000 uses a separate block
    print("\n" + "=" * 70)
    print("CHECKING IF 71000 IS A SEPARATE BLOCK")
    print("=" * 70)

    # If blocks are by 100s: 71000-71099, 71100-71199, ..., 71800-71899
    # Each would have its own base

    # Let's calculate expected bases if blocks are 100 flags each
    # 71800 base = 2725 (known)
    # 71700 base = 2725 - (100/8) = 2725 - 12.5 = ?
    # Actually blocks might be at irregular positions

    # Let's try to find block 71000 by searching near 71800
    # If 71000-71099 is stored before 71800-71899:
    # Distance = (71800 - 71000) / 8 = 100 bytes
    # So 71000 base would be around 2625

    # But 2625 is all zeros. So blocks might be NON-CONTIGUOUS.

    # Let's search for a base where multiple 71000-range flags are SET
    print("\nSearching for base where 71000-71008 flags are SET in S0...")

    # We know mid-game Confessor explored Stormveil, so some graces should be SET

    # Search different bases
    search_results = []

    for base in range(0, 5000):
        set_count = 0
        for i in range(9):
            byte_offset = base + i // 8
            bit_pos = 7 - (i % 8)
            if check_flag(ef_s0, byte_offset, bit_pos):
                set_count += 1

        if 3 <= set_count <= 8:  # Reasonable range for mid-game
            unset_count_s1 = 0
            for i in range(9):
                byte_offset = base + i // 8
                bit_pos = 7 - (i % 8)
                if not check_flag(ef_s1, byte_offset, bit_pos):
                    unset_count_s1 += 1

            if unset_count_s1 >= 7:  # S1 should have most unset
                search_results.append((base, set_count, unset_count_s1))

    print(f"\nFound {len(search_results)} candidate bases (3-8 flags SET in S0, most UNSET in S1)")

    if search_results:
        # Sort by number of flags set in S0
        search_results.sort(key=lambda x: -x[1])
        print("\nTop candidates:")
        for base, set_s0, unset_s1 in search_results[:15]:
            byte0 = ef_s0[base] if base < len(ef_s0) else 0
            byte1 = ef_s0[base+1] if base+1 < len(ef_s0) else 0
            print(f"  Base {base}: {set_s0} SET in S0, {unset_s1}/9 UNSET in S1 | bytes: 0x{byte0:02X} 0x{byte1:02X}")

            # Show which specific flags are set
            flags_set = []
            for i in range(9):
                byte_offset = base + i // 8
                bit_pos = 7 - (i % 8)
                if check_flag(ef_s0, byte_offset, bit_pos):
                    flags_set.append(71000 + i)
            print(f"    Flags SET: {flags_set}")

    # Also check graces using the verified offset difference
    print("\n" + "=" * 70)
    print("CHECKING 71000 USING VERIFIED OFFSET DIFFERENCES")
    print("=" * 70)

    # From openmap.eventflagalloclist, we might find the actual offsets
    # But for now, let's check if there's a consistent pattern

    # 71800 base = 2725
    # 76100 base = 3250
    # Difference: 76100 - 71800 = 4300 flags = 537.5 bytes
    # Actual byte difference: 3250 - 2725 = 525 bytes

    # So the mapping is NOT linear (537.5 vs 525)
    # This suggests blocks have gaps or different sizes

    # Let's try to find 71000 by looking at the legacymap alloclist
    print("\nThe offset calculation suggests blocks are NOT contiguous.")
    print("Block 71800 and Block 76000 have a byte gap of 525 for 4300 flag IDs")
    print("This is ~8.2 flags per byte, close to the expected 8.")

    # For 71000 (800 flags before 71800):
    # Expected byte difference = 800 / 8 = 100 bytes
    # So 71000 base ≈ 2725 - 100 = 2625
    # But that's all zeros!

    # Maybe the blocks for legacy dungeons (71000-71099) are stored elsewhere?

    print("\n" + "=" * 70)
    print("FINAL HYPOTHESIS: LEGACY DUNGEON GRACE BLOCKS")
    print("=" * 70)

    print("""
Based on the analysis:
1. 71800-71899 (Tutorial) is at base 2725 ✓
2. 76000-76999 (Limgrave overworld) is at base 3250 ✓
3. 71000-71099 (Stormveil Castle) is NOT at calculated base 2625

Possible explanations:
A) Legacy dungeon graces (71xxx where xxx < 800) use a different storage region
B) The grace discovery flags might use a different format for legacy dungeons
C) The flag IDs in BonfireWarpParam might not directly map to EventFlags

Let's check if there's an eventflagalloclist pattern we're missing.
""")

    # Check if maybe the 71000 range is AFTER 76000 instead of before
    print("Checking if 71000 block is AFTER 76000 (non-sequential flag storage):")

    # If 71000 is stored after 76000:
    # 76999 is the end of block 76000
    # 76999 - 76000 = 999 flags = 124.875 bytes
    # 76999 byte = 3250 + 124 = 3374
    # If 71000 starts at 3375:

    test_bases = [3375, 3380, 3400, 3500, 3600]

    for test_base in test_bases:
        set_count = 0
        for i in range(9):
            byte_offset = test_base + i // 8
            bit_pos = 7 - (i % 8)
            if byte_offset < len(ef_s0):
                if check_flag(ef_s0, byte_offset, bit_pos):
                    set_count += 1

        if set_count > 0:
            print(f"  Base {test_base}: {set_count}/9 SET in S0")

if __name__ == "__main__":
    main()
