#!/usr/bin/env python3
"""
Validate Area 16 candidate bases by checking broader flag patterns.

We have 32 candidates from inseparable evidence. Let's test them by:
1. Checking local offsets 0-1000 for coherent patterns
2. Looking for "clean" regions (all zeros) vs "noisy" regions (random bits)
3. The correct base should show sparse boss flags, not random noise
"""

import struct
from pathlib import Path
from collections import Counter

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

# Candidates from inseparable evidence probe
CANDIDATES = [4807, 11452, 11477, 11516, 11552, 12202, 21605, 22327,
              40337, 40517, 41457, 41537, 42687, 43702, 62137]

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

def analyze_byte_range(event_flags, base, num_bytes=125):
    """Analyze a byte range for patterns."""
    results = {
        'zero_bytes': 0,
        'nonzero_bytes': 0,
        'total_set_bits': 0,
        'pattern': ''
    }

    for i in range(num_bytes):
        byte_offset = base + i
        if byte_offset < len(event_flags):
            byte_val = event_flags[byte_offset]
            if byte_val == 0:
                results['zero_bytes'] += 1
                results['pattern'] += '.'
            else:
                results['nonzero_bytes'] += 1
                results['total_set_bits'] += bin(byte_val).count('1')
                results['pattern'] += 'X'
        else:
            results['pattern'] += '?'

    return results

def main():
    print("=" * 70)
    print("AREA 16 CANDIDATE VALIDATION")
    print("=" * 70)

    print("\nLoading event flags...")
    slot0_flags = read_slot_event_flags(0)
    slot1_flags = read_slot_event_flags(1)

    print(f"\nAnalyzing {len(CANDIDATES)} candidate bases...")
    print("Looking for bases with sparse, coherent patterns (few set bits)\n")

    # Dungeon sections are typically 1125 bytes
    # Area 16 boss flags are at local offsets 500, 800, 850, 860
    # So we need to check at least bytes 0-125 (which covers local 0-999)

    results = []
    for base in CANDIDATES:
        analysis_s0 = analyze_byte_range(slot0_flags, base, num_bytes=125)
        analysis_s1 = analyze_byte_range(slot1_flags, base, num_bytes=125)

        results.append({
            'base': base,
            'zero_s0': analysis_s0['zero_bytes'],
            'nonzero_s0': analysis_s0['nonzero_bytes'],
            'bits_s0': analysis_s0['total_set_bits'],
            'zero_s1': analysis_s1['zero_bytes'],
            'nonzero_s1': analysis_s1['nonzero_bytes'],
            'bits_s1': analysis_s1['total_set_bits'],
            'pattern_s0': analysis_s0['pattern'],
        })

    # Sort by sparsity (fewer set bits = better)
    results.sort(key=lambda x: (x['bits_s0'], x['bits_s1']))

    print("=" * 70)
    print("CANDIDATES SORTED BY SPARSITY (fewer set bits = better)")
    print("=" * 70)
    print(f"{'Base':>8} | {'Zero S0':>8} | {'NonZ S0':>8} | {'Bits S0':>8} | {'Bits S1':>8}")
    print("-" * 70)

    for r in results:
        print(f"{r['base']:>8} | {r['zero_s0']:>8} | {r['nonzero_s0']:>8} | {r['bits_s0']:>8} | {r['bits_s1']:>8}")

    # Show detailed pattern for top candidates
    print("\n" + "=" * 70)
    print("DETAILED PATTERNS FOR TOP 5 CANDIDATES")
    print("(. = zero byte, X = nonzero byte)")
    print("=" * 70)

    for r in results[:5]:
        print(f"\nBase {r['base']} (0x{r['base']:X}):")
        print(f"  Slot 0: {r['bits_s0']} set bits in 125 bytes")
        print(f"  Pattern: {r['pattern_s0']}")

        # Show specific bytes at boss flag positions
        base = r['base']
        print(f"  Boss flag byte values:")
        for local, name in [(62, "~500"), (100, "~800"), (106, "~850"), (107, "~860")]:
            byte_s0 = slot0_flags[base + local] if base + local < len(slot0_flags) else 0
            byte_s1 = slot1_flags[base + local] if base + local < len(slot1_flags) else 0
            print(f"    Byte {local} ({name}): S0=0x{byte_s0:02X}, S1=0x{byte_s1:02X}")

    # Also check if candidate 43702 (close to Area 18) shows special pattern
    print("\n" + "=" * 70)
    print("SPECIAL CHECK: Base 43702 (close to Area 18's 43487)")
    print("=" * 70)
    base = 43702
    if base in [r['base'] for r in results]:
        r = [r for r in results if r['base'] == base][0]
        print(f"  Set bits: {r['bits_s0']} (slot 0), {r['bits_s1']} (slot 1)")
        print(f"  Pattern: {r['pattern_s0']}")

if __name__ == "__main__":
    main()
