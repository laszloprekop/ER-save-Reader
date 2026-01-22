#!/usr/bin/env python3
"""
Probe for the correct Area 16 (Volcano Manor) dungeon base - V2.

Uses the same EventFlags detection methodology as the Rust code.

The event_flags section is 0x1bf99f bytes (1,833,375 bytes).
Dungeon bases for Area 16 (slot 29) should be: 4112 + 29 * 1125 = 36737
But this was disproven.

We need to find where Area 16 boss flags (16000xxx) are actually stored.
"""

import struct
from pathlib import Path

# Save file paths
SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"

# Save structure constants
SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010  # Each character slot

# EventFlags constants (from Rust code)
EVENT_FLAGS_SIZE = 0x1bf99f  # 1,833,375 bytes

# Validation flags for detecting EventFlags start (from Rust code)
# Format: (flag_id, byte_offset_in_event_flags, bit_position, name)
VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge"),
    (71801, 2725, 6, "Stranded Graveyard"),
    (76100, 3262, 3, "The First Step"),
    (76101, 3262, 2, "Church of Elleh"),
]

# Area 16 flags to test - boss defeats
# Format: (flag_id, local_offset, name, expected_slot0, expected_slot1)
# expected: True = SET, False = UNSET, None = unknown
AREA_16_FLAGS = [
    # Boss defeats - local offset = flag_id - 16000000
    (16000500, 500, "Omenkiller", None, False),  # Optional boss
    (16000800, 800, "God-Devouring Serpent (Rykard)", False, False),  # KNOWN: User hasn't defeated
    (16000801, 801, "God-Devouring Serpent (alt)", False, False),
    (16000850, 850, "Godskin Noble", None, False),
    (16000860, 860, "Abductor Virgin", None, False),
]

def detect_event_flags_start(slot_data, search_start=0):
    """
    Detect the EventFlags start offset within slot data.
    Uses the same validation flags as the Rust code.
    """
    max_search = 200_000
    search_end = min(search_start + max_search, len(slot_data) - 10000)

    best_offset = None
    best_score = 0

    for test_offset in range(search_start, search_end):
        score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if byte_val & (1 << bit_pos):
                    score += 1

        if score > best_score:
            best_score = score
            best_offset = test_offset

        if score == len(VALIDATION_FLAGS):
            # All validation flags matched - likely correct
            return test_offset, score

    return best_offset, best_score

def read_slot_event_flags(slot_index):
    """Read the full event flags for a slot."""
    with open(SAVE_FILE, 'rb') as f:
        # Calculate slot start
        slot_start = SLOT_0_OFFSET + slot_index * SLOT_SIZE

        # Read the entire slot
        f.seek(slot_start)
        slot_data = f.read(SLOT_SIZE)

    # Detect event flags start within slot
    ef_start, score = detect_event_flags_start(slot_data, search_start=0x1901D0 - 1000)

    print(f"  Slot {slot_index}: EventFlags detected at offset 0x{ef_start:X} (score {score}/{len(VALIDATION_FLAGS)})")

    # Extract event flags
    ef_end = min(ef_start + EVENT_FLAGS_SIZE, len(slot_data))
    event_flags = slot_data[ef_start:ef_end]

    return event_flags

def check_flag_at_base(event_flags, base_offset, local_flag_offset):
    """Check if a flag is set at the given base + local offset."""
    # Dungeon formula: base + local_offset / 8
    byte_offset = base_offset + (local_flag_offset // 8)
    bit_position = 7 - (local_flag_offset % 8)

    if byte_offset >= len(event_flags) or byte_offset < 0:
        return None  # Out of range

    byte_val = event_flags[byte_offset]
    return bool(byte_val & (1 << bit_position))

def probe_wide_search(slot0_flags, slot1_flags, search_range=None):
    """Search wide range for bases that match known conditions."""
    if search_range is None:
        search_range = (0, min(100000, len(slot0_flags) - 200))

    # Key constraint: 16000800 (Rykard) should be UNSET in slot 0
    rykard_local = 800

    candidates = []

    for base in range(search_range[0], search_range[1]):
        # Check Rykard flag
        rykard_slot0 = check_flag_at_base(slot0_flags, base, rykard_local)
        rykard_slot1 = check_flag_at_base(slot1_flags, base, rykard_local)

        if rykard_slot0 is None:
            continue

        # Key condition: Rykard should be UNSET in both slots
        if not rykard_slot0 and not rykard_slot1:
            # This base makes Rykard UNSET - potential candidate
            # Now check other flags for consistency
            all_match = True
            details = []

            for flag_id, local, name, exp_s0, exp_s1 in AREA_16_FLAGS:
                val_s0 = check_flag_at_base(slot0_flags, base, local)
                val_s1 = check_flag_at_base(slot1_flags, base, local)

                if exp_s0 is not None and val_s0 != exp_s0:
                    all_match = False
                if exp_s1 is not None and val_s1 != exp_s1:
                    all_match = False

                details.append((name, local, val_s0, val_s1, exp_s0, exp_s1))

            # Count how many flags are SET in slot 0
            set_count_s0 = sum(1 for _, _, v, _, _, _ in details if v)
            set_count_s1 = sum(1 for _, _, _, v, _, _ in details if v)

            candidates.append({
                'base': base,
                'all_match': all_match,
                'set_count_s0': set_count_s0,
                'set_count_s1': set_count_s1,
                'details': details
            })

    return candidates

def check_legacymap_formula():
    """Show what the legacymap formula gives us."""
    # From legacymap.eventflagalloclist, slot 29 = Volcano Manor
    # Formula: 4112 + slot * 1125
    slot = 29
    base = 4112 + slot * 1125
    print(f"\nLegacymap formula for Area 16 (slot {slot}):")
    print(f"  Base = 4112 + {slot} * 1125 = {base}")
    return base

def main():
    print("=" * 70)
    print("AREA 16 (VOLCANO MANOR) DUNGEON BASE DISCOVERY - V2")
    print("=" * 70)

    print("\nLoading save file and detecting EventFlags...")
    slot0_flags = read_slot_event_flags(0)
    slot1_flags = read_slot_event_flags(1)
    print(f"\n  Slot 0 flags: {len(slot0_flags):,} bytes")
    print(f"  Slot 1 flags: {len(slot1_flags):,} bytes")

    # Show the original formula result
    original_base = check_legacymap_formula()

    # Check what the original base reads
    print(f"\nOriginal base {original_base} reads:")
    for flag_id, local, name, exp_s0, exp_s1 in AREA_16_FLAGS:
        val_s0 = check_flag_at_base(slot0_flags, original_base, local)
        val_s1 = check_flag_at_base(slot1_flags, original_base, local)
        print(f"  {flag_id} ({name}): Slot0={val_s0}, Slot1={val_s1}")

    # Wide search
    print("\n" + "=" * 70)
    print("WIDE SEARCH FOR CANDIDATE BASES")
    print("=" * 70)
    print("\nSearching for bases where Rykard (16000800) is UNSET in both slots...")
    print("(This may take a moment...)")

    candidates = probe_wide_search(slot0_flags, slot1_flags)

    # Filter to interesting candidates (where at least some flags are SET in slot 0)
    interesting = [c for c in candidates if c['set_count_s0'] > 0]

    print(f"\nFound {len(candidates)} bases where Rykard is UNSET in both slots")
    print(f"Of these, {len(interesting)} have at least one flag SET in slot 0")

    if interesting:
        print("\n" + "-" * 70)
        print("INTERESTING CANDIDATES (have at least one SET flag in slot 0):")
        print("-" * 70)

        for c in sorted(interesting, key=lambda x: -x['set_count_s0'])[:20]:
            base = c['base']
            print(f"\nBase {base} (0x{base:X}):")
            print(f"  SET flags: Slot0={c['set_count_s0']}, Slot1={c['set_count_s1']}")
            for name, local, val_s0, val_s1, exp_s0, exp_s1 in c['details']:
                status = ""
                if exp_s0 is not None:
                    status = " [EXPECTED]" if val_s0 == exp_s0 else " [MISMATCH!]"
                print(f"    Local {local:3d} ({name:40s}): S0={val_s0}, S1={val_s1}{status}")

    # Also check Abductor Virgins - a boss encounter accessible early via Raya Lucaria trap
    # If the user has been teleported by the trap and killed the Abductor Virgin, local 860 would be SET
    print("\n" + "=" * 70)
    print("CHECKING FOR ABDUCTOR VIRGIN FLAG (accessible via Raya Lucaria trap)")
    print("=" * 70)

    abductor_candidates = []
    for base in range(0, min(100000, len(slot0_flags) - 200)):
        val_s0 = check_flag_at_base(slot0_flags, base, 860)  # Abductor Virgin
        val_s1 = check_flag_at_base(slot1_flags, base, 860)
        if val_s0 and not val_s1:  # SET in slot 0, UNSET in slot 1
            # Also check Rykard is UNSET
            rykard_s0 = check_flag_at_base(slot0_flags, base, 800)
            if not rykard_s0:
                abductor_candidates.append(base)

    print(f"\nFound {len(abductor_candidates)} bases where:")
    print("  - Abductor Virgin (860) is SET in slot 0, UNSET in slot 1")
    print("  - Rykard (800) is UNSET in slot 0")
    if abductor_candidates:
        print("\nTop candidates:")
        for base in abductor_candidates[:10]:
            print(f"  Base {base} (0x{base:X})")

            # Show all flags at this base
            for flag_id, local, name, exp_s0, exp_s1 in AREA_16_FLAGS:
                val_s0 = check_flag_at_base(slot0_flags, base, local)
                val_s1 = check_flag_at_base(slot1_flags, base, local)
                print(f"    {local:3d} ({name:40s}): S0={val_s0}, S1={val_s1}")

    print("\n" + "=" * 70)
    print("ADDITIONAL ANALYSIS")
    print("=" * 70)
    print("\nTo narrow down the correct base, we need to know:")
    print("  1. Has the Confessor (slot 0) killed Omenkiller?")
    print("  2. Has the Confessor killed Godskin Noble?")
    print("  3. Has the Confessor been teleported by the Abductor Virgin trap?")
    print("  4. Has the Confessor killed any Abductor Virgin bosses?")

if __name__ == "__main__":
    main()
