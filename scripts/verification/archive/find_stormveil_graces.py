#!/usr/bin/env python3
"""
Wide search to find where Stormveil grace flags are actually stored.

The user confirmed they explored Stormveil and discovered graces, but
probe_block_71000.py shows all flags UNSET. This means either:
1. The flag IDs 71000-71008 are wrong
2. The base offset is wrong
3. The formula is different

Strategy: Do a wide search for bit patterns that match expected behavior:
- Slot 0 (Confessor): Should have multiple Stormveil graces SET
- Slot 1 (Wretch): Should have few/no Stormveil graces SET

We'll search for byte regions where Slot 0 has multiple bits set and Slot 1 has fewer.
"""

import struct
from pathlib import Path
from collections import defaultdict

SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
EVENT_FLAGS_SIZE = 0x1bf99f

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

def detect_event_flags_start(slot_data, search_start=0):
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - 10000)

    for test_offset in range(search_start, search_end):
        score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    score += 1

        if score == len(VALIDATION_FLAGS):
            return test_offset, score

    return None, 0

def read_slot_event_flags(slot_index):
    with open(SAVE_FILE, 'rb') as f:
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE
        f.seek(slot_start)
        slot_data = f.read(SLOT_SIZE)

    ef_start, score = detect_event_flags_start(slot_data, search_start=0x1901D0 - 1000)
    if ef_start is None:
        ef_start = 0x1901D0

    ef_end = min(ef_start + EVENT_FLAGS_SIZE, len(slot_data))
    return slot_data[ef_start:ef_end]

def count_set_bits(byte_val):
    return bin(byte_val).count('1')

def main():
    print("=" * 70)
    print("WIDE SEARCH FOR STORMVEIL GRACE FLAGS")
    print("=" * 70)

    print("\nLoading event flags...")
    slot0_flags = read_slot_event_flags(0)
    slot1_flags = read_slot_event_flags(1)
    print(f"  Slot 0: {len(slot0_flags):,} bytes")
    print(f"  Slot 1: {len(slot1_flags):,} bytes")

    # First, let's verify the known working graces
    print("\n" + "=" * 70)
    print("VALIDATING KNOWN GRACES (from validation flags)")
    print("=" * 70)

    for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
        byte_s0 = slot0_flags[byte_offset] if byte_offset < len(slot0_flags) else 0
        byte_s1 = slot1_flags[byte_offset] if byte_offset < len(slot1_flags) else 0
        val_s0 = bool(byte_s0 & (1 << bit_pos))
        val_s1 = bool(byte_s1 & (1 << bit_pos))
        print(f"  {flag_id} ({name}): S0={val_s0}, S1={val_s1}")

    # Check the formula for 71800
    print("\n" + "=" * 70)
    print("UNDERSTANDING THE FORMULA FOR 71800")
    print("=" * 70)

    # 71800 is at byte 2725, bit 7
    # If block start is 71800 with base 2725:
    #   local = 71800 - 71800 = 0
    #   byte_offset = 2725 + 0/8 = 2725
    #   bit = 7 - 0%8 = 7 ✓

    # For 71000, if it follows the same pattern with block 71000:
    #   We need to find the base for block 71000

    # Let's check if there's a pattern in verified offsets
    # 71800 block at base 2725
    # 76100 block at byte 3262 → base = 3262 - (100/8) = 3262 - 12 = 3250

    print("  71800 block: base 2725 (verified)")
    print("  76100 block: checking formula...")

    # Verify 76100
    # If block start = 76100, base = ?
    # byte 3262 = base + (76100 - 76100)/8 = base
    # So base for 76100 block = 3262? No wait...

    # Let me recalculate. 76100 at byte 3262, bit 3
    # local = 76100 - block_start
    # byte = base + local/8
    # bit = 7 - local%8

    # If block_start = 76100:
    #   local = 0
    #   byte = base + 0 = base
    #   bit = 7 - 0 = 7 (but we expect bit 3!)

    # So block_start is NOT 76100. Let's reverse engineer:
    # bit = 7 - local%8 = 3 → local%8 = 4
    # So local = 4, 12, 20, ... or local = 76100 - block_start = something where mod 8 = 4

    # 76100 % 8 = 4, so if block_start % 8 = 0, then local % 8 = 4 ✓
    # byte = base + local/8 = 3262
    # If local = 4 (block_start = 76096): base = 3262 - 0 = 3262
    # But 76096 is a weird start. Let's try block_start = 76000:
    #   local = 76100 - 76000 = 100
    #   byte = base + 100/8 = base + 12
    #   bit = 7 - 100%8 = 7 - 4 = 3 ✓
    #   3262 = base + 12 → base = 3250

    print("  76100: block_start=76000, base=3250, local=100, byte=3262, bit=3 ✓")

    # Now for 71800:
    #   local = 71800 - block_start
    #   byte = 2725 = base + local/8
    #   bit = 7 = 7 - local%8 → local%8 = 0

    # If block_start = 71800:
    #   local = 0, byte = base = 2725, bit = 7 ✓
    # If block_start = 71000:
    #   local = 800, byte = base + 100, bit = 7 - 0 = 7
    #   2725 = base + 100 → base = 2625

    print("  71800: Could be block_start=71800 base=2725, OR block_start=71000 base=2625")

    # Let's check block 71000 with base 2625
    print("\n" + "=" * 70)
    print("CHECKING BLOCK 71000 WITH BASE 2625")
    print("=" * 70)

    # With block_start=71000, base=2625:
    # 71800: local=800, byte=2625+100=2725, bit=7-0=7 ✓
    # 71801: local=801, byte=2625+100=2725, bit=7-1=6 ✓

    # But what about 71000 (Godrick grace)?
    # local=0, byte=2625+0=2625, bit=7

    base_71000 = 2625
    block_start = 71000

    for flag_id in [71000, 71001, 71002, 71003, 71004, 71005, 71006, 71007, 71008]:
        local = flag_id - block_start
        byte_offset = base_71000 + local // 8
        bit_pos = 7 - (local % 8)

        if byte_offset < len(slot0_flags):
            byte_s0 = slot0_flags[byte_offset]
            byte_s1 = slot1_flags[byte_offset]
            val_s0 = bool(byte_s0 & (1 << bit_pos))
            val_s1 = bool(byte_s1 & (1 << bit_pos))
            print(f"  {flag_id}: local={local}, byte={byte_offset}, bit={bit_pos} → S0={val_s0}, S1={val_s1}")

    # Let's also show the raw bytes around 2625
    print("\n" + "=" * 70)
    print("RAW BYTES AROUND OFFSET 2625")
    print("=" * 70)

    for offset in range(2620, 2730):
        byte_s0 = slot0_flags[offset] if offset < len(slot0_flags) else 0
        byte_s1 = slot1_flags[offset] if offset < len(slot1_flags) else 0
        if byte_s0 != 0 or byte_s1 != 0:  # Only show non-zero
            print(f"  Byte {offset}: S0=0x{byte_s0:02X} ({byte_s0:08b}), S1=0x{byte_s1:02X} ({byte_s1:08b})")

    # Now let's try to find where bits are actually set that could be Stormveil graces
    # We're looking for a region where:
    # - Multiple bits are SET in slot 0
    # - Fewer/no bits are SET in slot 1
    # - The pattern is consistent with 9 possible graces

    print("\n" + "=" * 70)
    print("SEARCHING FOR REGIONS WITH STORMVEIL-LIKE PATTERN")
    print("(Many bits in S0, few in S1, in a ~2 byte window)")
    print("=" * 70)

    candidates = []
    # Graces 71000-71008 would span 2 bytes (9 flags / 8 = 1.125 bytes)

    for start_byte in range(2000, 4000):  # Search in likely region
        if start_byte + 2 >= len(slot0_flags):
            continue

        # Count bits in 2-byte window
        bits_s0 = 0
        bits_s1 = 0
        for i in range(2):
            bits_s0 += count_set_bits(slot0_flags[start_byte + i])
            bits_s1 += count_set_bits(slot1_flags[start_byte + i])

        # We want: S0 has 4-9 bits set, S1 has 0-2 bits set (differential)
        if bits_s0 >= 4 and bits_s1 <= 2 and bits_s0 - bits_s1 >= 3:
            candidates.append({
                'byte': start_byte,
                'bits_s0': bits_s0,
                'bits_s1': bits_s1,
                'diff': bits_s0 - bits_s1
            })

    # Sort by differential
    candidates.sort(key=lambda x: -x['diff'])

    print(f"\nFound {len(candidates)} candidate regions")
    print("\nTop 15 candidates:")
    for c in candidates[:15]:
        byte = c['byte']
        b0_s0 = slot0_flags[byte]
        b1_s0 = slot0_flags[byte + 1]
        b0_s1 = slot1_flags[byte]
        b1_s1 = slot1_flags[byte + 1]
        print(f"  Byte {byte}: S0 bits={c['bits_s0']}, S1 bits={c['bits_s1']}, diff={c['diff']}")
        print(f"    S0: 0x{b0_s0:02X} 0x{b1_s0:02X} ({b0_s0:08b} {b1_s0:08b})")
        print(f"    S1: 0x{b0_s1:02X} 0x{b1_s1:02X} ({b0_s1:08b} {b1_s1:08b})")

    # Check if 2625 or nearby bytes show anything interesting
    print("\n" + "=" * 70)
    print("CHECKING EXPECTED LOCATION (BYTE 2625)")
    print("=" * 70)

    byte_2625_s0 = slot0_flags[2625]
    byte_2626_s0 = slot0_flags[2626]
    byte_2625_s1 = slot1_flags[2625]
    byte_2626_s1 = slot1_flags[2626]

    print(f"  Byte 2625: S0=0x{byte_2625_s0:02X} ({byte_2625_s0:08b}), S1=0x{byte_2625_s1:02X} ({byte_2625_s1:08b})")
    print(f"  Byte 2626: S0=0x{byte_2626_s0:02X} ({byte_2626_s0:08b}), S1=0x{byte_2626_s1:02X} ({byte_2626_s1:08b})")
    print(f"  Total bits at 2625-2626: S0={count_set_bits(byte_2625_s0) + count_set_bits(byte_2626_s0)}, S1={count_set_bits(byte_2625_s1) + count_set_bits(byte_2626_s1)}")

    # Maybe the grace flag IDs are different. Let's check what we know about graces:
    # 71800 = Cave of Knowledge (works)
    # 76100 = The First Step (works)
    #
    # The 71xxx range seems to be for legacy dungeon graces
    # Maybe Stormveil uses 71000-71099 or a different sub-block?

    print("\n" + "=" * 70)
    print("CHECKING VERIFIED GRACES FOR PATTERN")
    print("=" * 70)

    # Let's see what other grace blocks look like
    # Block 71800 (Cave of Knowledge area): base 2725
    # Block 76000 (Limgrave overworld): base 3250

    # If Stormveil is block 71000, it should be at a different base
    # Let's calculate potential bases based on the block number difference

    # 71800 - 71000 = 800 flags = 100 bytes
    # If 71800 block starts at byte 2725, and 71000 is 100 bytes earlier:
    #   base_71000 = 2725 - 100 = 2625
    # But we already checked this and it's all zeros!

    # Maybe the flag IDs for Stormveil graces are NOT 71000-71008?
    # Let's look at what flag IDs might actually be used

    # From game data, Stormveil is map 10 (m10_00_00_00)
    # Legacy dungeon flags often use format 10XXYYYY where XX is map and YYYY is local

    # But graces seem to use 7xxxx format

    # Let me check if maybe Stormveil graces use 71100 range
    print("\nTrying block 71100 (Stormveil alternative):")
    base_71100 = 2737  # Calculated: 2725 + (71100-71800)/8 = 2725 - 87.5... hmm

    # Actually, let's be more systematic. If block boundaries are every 1000:
    # Block 71000: flags 71000-71999
    # Block 76000: flags 76000-76999

    # For 71800 at byte 2725:
    # If block_start = 71000: local = 800, byte = base + 100
    #   2725 = base + 100 → base = 2625
    # For 71000: local = 0, byte = 2625, bit = 7

    # But byte 2625 is all zeros in both slots!

    # Let me try a different hypothesis: maybe grace flag IDs follow the map ID
    # Stormveil is map m10_00_00_00, maybe grace flags are 71 + map = 7110xx?

    print("\nTrying different Stormveil flag ID hypotheses:")

    hypotheses = [
        (71000, 2625, "Block 71000 at base 2625"),
        (71100, 2637, "Block 71100 at base 2637"),
        (71010, 2626, "Block 71000 at offset 10"),
    ]

    for flag_start, base, desc in hypotheses:
        print(f"\n  {desc}:")
        any_set = False
        for i in range(9):
            flag_id = flag_start + i
            local = flag_id - (flag_start - (flag_start % 1000))  # Align to block
            byte_offset = base + local // 8
            bit_pos = 7 - (local % 8)

            if byte_offset < len(slot0_flags):
                val_s0 = bool(slot0_flags[byte_offset] & (1 << bit_pos))
                val_s1 = bool(slot1_flags[byte_offset] & (1 << bit_pos))
                if val_s0 or val_s1:
                    any_set = True
                    print(f"    {flag_id}: S0={val_s0}, S1={val_s1}")
        if not any_set:
            print("    (all UNSET)")

if __name__ == "__main__":
    main()
