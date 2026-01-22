#!/usr/bin/env python3
"""
Probe for the correct Area 16 (Volcano Manor) dungeon base.

The original base 36737 was DISPROVEN because:
- Flag 16000800 (Rykard defeat) showed SET
- But the user confirmed Rykard was NOT defeated

We need to find a base where:
- 16000800 reads as UNSET (KNOWN: Rykard not defeated)
- If user has defeated Omenkiller (16000500), that should be SET

Using wide search to find candidate bases.
"""

import struct
from pathlib import Path

# Save file paths
SAVE_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files")
SAVE_FILE = SAVE_DIR / "ER0000-backup-2026-01-11.sl2"  # Most recent backup

# Slot 0 event flags location (Confessor - mid game)
SLOT_0_OFFSET = 0x310
SLOT_SIZE = 0x280010
EVENT_FLAGS_OFFSET = 0x1901D0
EVENT_FLAGS_SIZE = 0xB71

# Area 16 flags to test
# Format: (flag_id, local_offset, name, expected_slot0, expected_slot1)
# expected: True = SET, False = UNSET, None = unknown
AREA_16_FLAGS = [
    # Boss defeats - using local offset (flag_id - 16000000)
    (16000500, 500, "Omenkiller", None, False),  # Optional boss
    (16000800, 800, "God-Devouring Serpent (Rykard)", False, False),  # KNOWN: User hasn't defeated
    (16000801, 801, "God-Devouring Serpent (alt)", False, False),
    (16000850, 850, "Godskin Noble", None, False),
    (16000860, 860, "Abductor Virgin", None, False),
]

def read_save_file():
    """Read the save file and return event flags for slots 0 and 1."""
    with open(SAVE_FILE, 'rb') as f:
        # Read slot 0 event flags
        slot0_ef_offset = SLOT_0_OFFSET + EVENT_FLAGS_OFFSET
        f.seek(slot0_ef_offset)
        slot0_flags = f.read(EVENT_FLAGS_SIZE)

        # Read slot 1 event flags
        slot1_ef_offset = SLOT_0_OFFSET + SLOT_SIZE + EVENT_FLAGS_OFFSET
        f.seek(slot1_ef_offset)
        slot1_flags = f.read(EVENT_FLAGS_SIZE)

    return slot0_flags, slot1_flags

def check_flag_at_base(event_flags, base_offset, local_flag_offset):
    """Check if a flag is set at the given base + local offset."""
    # Dungeon formula: base + local_offset / 8
    byte_offset = base_offset + (local_flag_offset // 8)
    bit_position = 7 - (local_flag_offset % 8)

    if byte_offset >= len(event_flags) or byte_offset < 0:
        return None  # Out of range

    byte_val = event_flags[byte_offset]
    return bool(byte_val & (1 << bit_position))

def probe_wide_search(slot0_flags, slot1_flags, search_range=(0, 100000)):
    """Search wide range for bases that match known conditions."""

    # Key constraint: 16000800 (Rykard) should be UNSET in slot 0
    rykard_local = 800

    candidates = []

    for base in range(search_range[0], min(search_range[1], len(slot0_flags))):
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

            # Count how many flags are SET in slot 0 (interesting if some are)
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

def analyze_legacymap_slot():
    """Show what the legacymap formula gives us."""
    # From legacymap.eventflagalloclist, slot 29 = Volcano Manor
    # Formula: 4112 + slot * 1125
    slot = 29
    base = 4112 + slot * 1125
    print(f"\nLegacymap formula for Area 16 (slot {slot}):")
    print(f"  Base = 4112 + {slot} * 1125 = {base}")
    print(f"  This was DISPROVEN - reads 0xFF block at local 800")
    return base

def main():
    print("=" * 70)
    print("AREA 16 (VOLCANO MANOR) DUNGEON BASE DISCOVERY")
    print("=" * 70)

    print("\nLoading save file...")
    slot0_flags, slot1_flags = read_save_file()
    print(f"  Slot 0 flags: {len(slot0_flags)} bytes")
    print(f"  Slot 1 flags: {len(slot1_flags)} bytes")

    # Show the original formula result
    original_base = analyze_legacymap_slot()

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

    # Also show candidates where ALL flags are UNSET (possible if user hasn't done VM content)
    all_unset = [c for c in candidates if c['set_count_s0'] == 0 and c['all_match']]
    print(f"\n{len(all_unset)} bases where all Area 16 flags are UNSET")
    if all_unset and len(all_unset) < 10:
        print("These could be correct if the character hasn't done any Volcano Manor content:")
        for c in all_unset[:5]:
            print(f"  Base {c['base']} (0x{c['base']:X})")

    # Check if the user might have killed Omenkiller
    print("\n" + "=" * 70)
    print("ADDITIONAL ANALYSIS")
    print("=" * 70)
    print("\nTo narrow down the correct base, we need to know:")
    print("  1. Has the character killed Omenkiller (optional boss)?")
    print("  2. Has the character killed Godskin Noble?")
    print("  3. Has the character killed Abductor Virgin?")
    print("\nIf any of these are YES, we can use that as positive evidence.")

if __name__ == "__main__":
    main()
