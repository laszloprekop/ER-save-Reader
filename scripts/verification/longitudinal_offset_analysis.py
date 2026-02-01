#!/usr/bin/env python3
"""
Longitudinal Offset Analysis

Track how event flag offsets change across chronological save snapshots
of the SAME character to find the dynamic offset pattern.

Strategy:
1. Start with latest snapshot - find offsets for known flags
2. Go backwards through earlier snapshots
3. Check if the same flags are at the same or different offsets
4. Look for patterns in how offsets shift
"""

import struct
from pathlib import Path
from typing import Dict, List, Tuple, Optional
import re

SNAPSHOTS_DIR = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging/slot 0 Confessor")

BND4_HEADER_SIZE = 0x40
BND4_ENTRY_SIZE = 0x20
BND4_ENTRY_OFFSET_POS = 0x10
SLOT_CHECKSUM_SIZE = 16
EVENT_FLAGS_SIZE = 0x1BF99F


def get_snapshots_chronologically() -> List[Path]:
    """Get all snapshots sorted chronologically."""
    snapshots = list(SNAPSHOTS_DIR.glob("ER0000.sl2*"))

    def sort_key(p):
        name = p.name
        # Extract sequence number from different naming patterns
        # "ER0000.sl2 S0 - b39" -> 39
        # "ER0000.sl2 Confessor - 04" -> 4 (but earlier series, so offset by -1000)
        if " - b" in name:
            match = re.search(r' - b(\d+)', name)
            if match:
                return int(match.group(1))
        elif "Confessor - " in name:
            match = re.search(r'Confessor - (\d+)', name)
            if match:
                return int(match.group(1)) - 1000  # Earlier series
        return 0

    return sorted(snapshots, key=sort_key)


def extract_slot_and_ef(filepath: Path, slot_index: int = 0) -> Tuple[bytes, int, int]:
    """Extract slot data, EF data, and EF offset."""
    with open(filepath, 'rb') as f:
        data = f.read()

    entry_offset = BND4_HEADER_SIZE + (slot_index * BND4_ENTRY_SIZE) + BND4_ENTRY_OFFSET_POS
    bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
    slot_offset = bnd4_offset + SLOT_CHECKSUM_SIZE
    slot_data = data[slot_offset:slot_offset + 0x280000]

    # Find EF offset using validation flags
    VALIDATION_FLAGS = [(71800, 2725, 7), (76100, 3262, 3)]

    ef_offset = None
    for test_offset in range(0x10000, min(0x20000, len(slot_data) - EVENT_FLAGS_SIZE), 4):
        score = sum(1 for _, byte_off, bit_pos in VALIDATION_FLAGS
                    if test_offset + byte_off < len(slot_data)
                    and (slot_data[test_offset + byte_off] & (1 << bit_pos)) != 0)
        if score == len(VALIDATION_FLAGS):
            ef_offset = test_offset
            break

    if ef_offset is None:
        ef_offset = 0x12B00  # Fallback

    ef_data = slot_data[ef_offset:ef_offset + EVENT_FLAGS_SIZE]
    return slot_data, ef_data, ef_offset


def find_flag_in_ef(ef_data: bytes, target_bit: int, search_start: int = 0,
                    search_end: int = 10000) -> List[int]:
    """Find all offsets where a specific bit pattern is set."""
    matches = []
    for offset in range(search_start, min(search_end, len(ef_data))):
        if ef_data[offset] & (1 << target_bit):
            matches.append(offset)
    return matches


def check_flag_at_offset(ef_data: bytes, offset: int, bit: int) -> bool:
    """Check if flag is set at specific offset/bit."""
    if offset < len(ef_data):
        return (ef_data[offset] & (1 << bit)) != 0
    return False


def analyze_snapshot(filepath: Path, reference_flags: Dict[str, Tuple[int, int, bool]]) -> Dict:
    """
    Analyze a snapshot and check all reference flags.

    reference_flags: {name: (offset, bit, expected_set)}
    Returns: {name: (found_at_ref, actual_offset, actual_set)}
    """
    try:
        slot_data, ef_data, ef_offset = extract_slot_and_ef(filepath)
    except Exception as e:
        return {"error": str(e), "ef_offset": None}

    results = {"ef_offset": ef_offset, "flags": {}}

    for name, (ref_offset, bit, _) in reference_flags.items():
        is_set = check_flag_at_offset(ef_data, ref_offset, bit)
        results["flags"][name] = {
            "offset": ref_offset,
            "bit": bit,
            "is_set": is_set,
        }

    return results


def main():
    print("="*80)
    print("LONGITUDINAL OFFSET ANALYSIS")
    print("="*80)

    snapshots = get_snapshots_chronologically()
    print(f"\nFound {len(snapshots)} snapshots")

    # Show snapshot order
    print("\nSnapshot order (earliest to latest):")
    for i, s in enumerate(snapshots[:10]):
        print(f"  {i+1}. {s.name}")
    if len(snapshots) > 10:
        print(f"  ... and {len(snapshots) - 10} more")

    # Reference flags to track (using validation-verified offsets)
    # These are flags that should be SET early in the game
    reference_flags = {
        "71800 Cave of Knowledge": (2725, 7, True),
        "71801 Stranded Graveyard": (2725, 6, True),
        "76100 The First Step": (3262, 3, True),
        "76101 Church of Elleh": (3262, 2, True),
        "76102 Gatefront Ruins": (3262, 1, True),  # Early grace
    }

    print("\n" + "="*80)
    print("TRACKING VALIDATION FLAGS ACROSS SNAPSHOTS")
    print("="*80)

    # Analyze each snapshot
    snapshot_results = []
    for snapshot in snapshots:
        result = analyze_snapshot(snapshot, reference_flags)
        result["name"] = snapshot.name
        snapshot_results.append(result)

    # Print comparison table
    print("\nEF Offset changes:")
    prev_ef_offset = None
    for result in snapshot_results:
        ef_offset = result.get("ef_offset")
        if ef_offset:
            delta = f" (Δ{ef_offset - prev_ef_offset:+d})" if prev_ef_offset else ""
            print(f"  {result['name'][:60]:60s} EF=0x{ef_offset:05X}{delta}")
            prev_ef_offset = ef_offset

    # Check if validation flags remain at same offsets
    print("\n" + "="*80)
    print("CHECKING IF FLAGS MOVE BETWEEN SNAPSHOTS")
    print("="*80)

    for flag_name in reference_flags:
        print(f"\n{flag_name}:")
        offset, bit, _ = reference_flags[flag_name]

        prev_state = None
        state_changes = []

        for result in snapshot_results:
            if "error" in result:
                continue

            flag_data = result.get("flags", {}).get(flag_name, {})
            is_set = flag_data.get("is_set", False)

            if prev_state is not None and is_set != prev_state:
                state_changes.append((result["name"], prev_state, is_set))

            prev_state = is_set

        if state_changes:
            print(f"  State changes detected:")
            for name, old, new in state_changes:
                print(f"    {name[:50]}: {old} -> {new}")
        else:
            final_state = prev_state
            print(f"  Stable at offset {offset}, bit {bit}: always {'SET' if final_state else 'NOT SET'}")

    # Now look for the 71602 (Volcano Manor) flag transition
    print("\n" + "="*80)
    print("SEARCHING FOR 71602 (Volcano Manor) ACROSS SNAPSHOTS")
    print("="*80)

    # In b38/b39 we know 71602 was set at offset 3198, bit 5
    # Let's track this across all snapshots

    vm_offset = 3198
    vm_bit = 5

    print(f"\nChecking offset {vm_offset}, bit {vm_bit} (where 71602 was found):")

    prev_byte = None
    for result in snapshot_results:
        if "error" in result:
            continue

        try:
            _, ef_data, _ = extract_slot_and_ef(Path(SNAPSHOTS_DIR / result["name"]))
            byte_val = ef_data[vm_offset] if vm_offset < len(ef_data) else 0
            is_set = (byte_val & (1 << vm_bit)) != 0

            if prev_byte is None or byte_val != prev_byte:
                print(f"  {result['name'][:55]:55s} byte=0x{byte_val:02x} bit5={'SET' if is_set else 'NOT'}")
                prev_byte = byte_val
        except:
            pass

    # Look for pointer table - search for offsets stored as 4-byte values
    print("\n" + "="*80)
    print("SEARCHING FOR POINTER TABLE IN SLOT DATA")
    print("="*80)

    # Get latest snapshot
    latest = snapshots[-1]
    print(f"\nAnalyzing: {latest.name}")

    slot_data, ef_data, ef_offset = extract_slot_and_ef(latest)

    # Search for known offsets as stored values
    known_offsets = [2725, 3250, 3262, 3198, 2662]

    print(f"\nSearching for known offsets stored as 4-byte LE values in slot data:")
    for target in known_offsets:
        target_bytes = struct.pack('<I', target)
        positions = []
        pos = 0
        while True:
            pos = slot_data.find(target_bytes, pos)
            if pos == -1:
                break
            # Only report if before EF section
            if pos < ef_offset:
                positions.append(pos)
            pos += 1

        if positions:
            print(f"  Offset {target}: found at slot positions {positions[:5]}{'...' if len(positions) > 5 else ''}")

    # Search in the EF header area (first 1000 bytes of EF)
    print(f"\nSearching in EF header (first 1000 bytes):")
    for target in known_offsets:
        target_bytes = struct.pack('<I', target)
        positions = []
        pos = 0
        while True:
            pos = ef_data.find(target_bytes, pos)
            if pos == -1 or pos > 1000:
                break
            positions.append(pos)
            pos += 1

        if positions:
            print(f"  Offset {target}: found at EF positions {positions}")


if __name__ == "__main__":
    main()
