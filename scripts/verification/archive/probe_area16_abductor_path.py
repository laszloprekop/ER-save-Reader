#!/usr/bin/env python3
"""
Probe for Area 16 base - Abductor Virgin trap path scenario.

The Confessor accessed Volcano Manor via the Abductor Virgin trap at Raya Lucaria.
This teleports directly to Subterranean Inquisition Chamber (grace 71607).

Expected boss flags for this path:
- Abductor Virgin (860): Maybe SET (if killed the boss pair to escape)
- Omenkiller (500): UNSET (not accessible from trap path)
- Godskin Noble (850): UNSET (requires going through main VM)
- Rykard (800, 801): UNSET

We need to find a base where these conditions are met.
"""

import struct
from pathlib import Path

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

# Area 16 flags with expected states for Abductor trap path
AREA_16_FLAGS = [
    (16000500, 500, "Omenkiller", False, False),  # NOT killed - wrong path
    (16000800, 800, "Rykard", False, False),  # NOT killed
    (16000801, 801, "Rykard (alt)", False, False),  # NOT killed
    (16000850, 850, "Godskin Noble", False, False),  # NOT killed - wrong path
    (16000860, 860, "Abductor Virgin", None, False),  # MAYBE killed (escape boss)
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
        # Fallback
        ef_start = 0x1901D0

    ef_end = min(ef_start + EVENT_FLAGS_SIZE, len(slot_data))
    return slot_data[ef_start:ef_end]

def check_flag_at_base(event_flags, base_offset, local_flag_offset):
    byte_offset = base_offset + (local_flag_offset // 8)
    bit_position = 7 - (local_flag_offset % 8)

    if byte_offset >= len(event_flags) or byte_offset < 0:
        return None

    byte_val = event_flags[byte_offset]
    return bool(byte_val & (1 << bit_position))

def main():
    print("=" * 70)
    print("AREA 16 PROBE - ABDUCTOR VIRGIN TRAP PATH")
    print("=" * 70)

    print("\nLoading event flags...")
    slot0_flags = read_slot_event_flags(0)
    slot1_flags = read_slot_event_flags(1)
    print(f"  Slot 0: {len(slot0_flags):,} bytes")
    print(f"  Slot 1: {len(slot1_flags):,} bytes")

    # Search for bases matching expected conditions
    print("\n" + "=" * 70)
    print("SEARCHING FOR BASES WHERE:")
    print("  - Omenkiller (500): UNSET in both slots")
    print("  - Godskin Noble (850): UNSET in both slots")
    print("  - Rykard (800): UNSET in both slots")
    print("  - Abductor Virgin (860): Any state (we'll check)")
    print("=" * 70)

    exact_matches = []  # Matches our exact expected state
    abductor_set_matches = []  # Abductor SET, others UNSET

    for base in range(0, min(100000, len(slot0_flags) - 200)):
        # Check all flags
        results = {}
        for flag_id, local, name, exp_s0, exp_s1 in AREA_16_FLAGS:
            results[local] = (
                check_flag_at_base(slot0_flags, base, local),
                check_flag_at_base(slot1_flags, base, local)
            )

        # Skip if any return None (out of range)
        if any(v[0] is None for v in results.values()):
            continue

        # Check if Omenkiller, Godskin Noble, and Rykard are all UNSET
        omenkiller_s0, omenkiller_s1 = results[500]
        godskin_s0, godskin_s1 = results[850]
        rykard_s0, rykard_s1 = results[800]
        rykard_alt_s0, rykard_alt_s1 = results[801]
        abductor_s0, abductor_s1 = results[860]

        # Condition: Omenkiller, Godskin Noble, Rykard all UNSET in slot 0
        if (not omenkiller_s0 and not godskin_s0 and not rykard_s0 and not rykard_alt_s0 and
            not omenkiller_s1 and not godskin_s1 and not rykard_s1 and not rykard_alt_s1):

            if abductor_s0 and not abductor_s1:
                # Abductor SET in slot 0, UNSET in slot 1 - STRONG candidate
                abductor_set_matches.append({
                    'base': base,
                    'abductor_s0': abductor_s0,
                    'abductor_s1': abductor_s1,
                    'results': results
                })
            elif not abductor_s0 and not abductor_s1:
                # All UNSET - possible if they escaped without killing boss
                exact_matches.append({
                    'base': base,
                    'results': results
                })

    print(f"\nFound {len(abductor_set_matches)} bases where Abductor is SET in slot 0 only")
    print(f"Found {len(exact_matches)} bases where all flags are UNSET")

    if abductor_set_matches:
        print("\n" + "-" * 70)
        print("STRONG CANDIDATES (Abductor SET in slot 0, others UNSET):")
        print("-" * 70)
        for m in abductor_set_matches[:15]:
            base = m['base']
            print(f"\n  Base {base} (0x{base:X}):")
            for local in [500, 800, 801, 850, 860]:
                s0, s1 = m['results'][local]
                name = [n for _, l, n, _, _ in AREA_16_FLAGS if l == local][0]
                print(f"    {local:3d} ({name:20s}): S0={s0}, S1={s1}")

    # Also check grace 71607 to verify it's SET (they visited Subterranean Inquisition Chamber)
    print("\n" + "=" * 70)
    print("CROSS-VALIDATION: Checking grace 71607 (Subterranean Inquisition Chamber)")
    print("=" * 70)

    # Grace 71607 should use block 71600 with base 2825
    grace_base = 2825
    grace_local = 71607 - 71600  # = 7
    grace_byte = grace_base + grace_local // 8  # 2825 + 0 = 2825
    grace_bit = 7 - (grace_local % 8)  # 7 - 7 = 0

    grace_s0 = check_flag_at_base(slot0_flags, grace_base, grace_local)
    grace_s1 = check_flag_at_base(slot1_flags, grace_base, grace_local)

    print(f"\n  Grace 71607 at base 2825, local {grace_local}:")
    print(f"    Slot 0: {grace_s0}")
    print(f"    Slot 1: {grace_s1}")

    if grace_s0:
        print("\n  ✓ Confirmed: Confessor has visited Subterranean Inquisition Chamber")
    else:
        print("\n  ✗ Warning: Grace 71607 not set - unexpected!")

    # Cross-check with nearby graces that shouldn't be set
    print("\n  Checking nearby Volcano Manor graces that should NOT be set:")
    for grace_id, name in [(71600, "Rykard grace"), (71602, "Volcano Manor"), (71603, "Prison Town Church")]:
        local = grace_id - 71600
        val_s0 = check_flag_at_base(slot0_flags, grace_base, local)
        val_s1 = check_flag_at_base(slot1_flags, grace_base, local)
        print(f"    {grace_id} ({name}): S0={val_s0}, S1={val_s1}")

if __name__ == "__main__":
    main()
