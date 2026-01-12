#!/usr/bin/env python3
"""
Discover correct block base offsets by comparing slots with known flag states.

This script searches the entire EventFlags section to find where specific flags
are actually stored, by comparing a slot that HAS a flag vs one that DOESN'T.
"""

import json
import sys
from pathlib import Path
from collections import defaultdict

# Add the scripts directory to path
sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser


def find_flag_in_event_flags(
    has_flag_bytes: bytes,
    not_flag_bytes: bytes,
    flag_id: int,
    search_start: int = 0,
    search_end: int = None,
    strict: bool = False
) -> list:
    """
    Find where a flag is stored by comparing two event flags sections.

    Args:
        has_flag_bytes: EventFlags from slot that HAS the flag set
        not_flag_bytes: EventFlags from slot that DOESN'T have the flag set
        flag_id: The flag ID we're looking for
        search_start: Start of search range
        search_end: End of search range
        strict: If True, only return candidates where ONLY the expected bit differs

    Returns:
        List of (byte_offset, bit_position, num_diff_bits) candidates
    """
    # Calculate expected bit position (same formula as block)
    expected_bit = 7 - (flag_id % 8)

    end = search_end or min(len(has_flag_bytes), len(not_flag_bytes))
    candidates = []

    for byte_off in range(search_start, end):
        has_byte = has_flag_bytes[byte_off]
        not_byte = not_flag_bytes[byte_off]

        # Check if the expected bit differs between the two
        has_bit_set = (has_byte >> expected_bit) & 1
        not_bit_set = (not_byte >> expected_bit) & 1

        # We want: has_flag has the bit SET, not_flag has it UNSET
        if has_bit_set == 1 and not_bit_set == 0:
            # Count how many bits differ in this byte
            diff = has_byte ^ not_byte
            num_diff_bits = bin(diff).count('1')

            if strict and num_diff_bits > 1:
                continue  # Skip if more than just our bit changed

            candidates.append((byte_off, expected_bit, num_diff_bits))

    return candidates


def find_unique_flag_locations(
    has_flag_bytes: bytes,
    not_flag_bytes: bytes,
    test_flags: list
) -> dict:
    """
    Find locations for multiple flags and cross-validate.

    If multiple flags in the same block range point to the same base offset,
    that's strong evidence of the correct base.
    """
    results = {}

    for flag_id, flag_name in test_flags:
        block_start = (flag_id // 1000) * 1000
        expected_bit = 7 - (flag_id % 8)

        # Find strict candidates (only the expected bit differs)
        candidates = find_flag_in_event_flags(
            has_flag_bytes,
            not_flag_bytes,
            flag_id,
            search_start=0,
            search_end=10000,
            strict=True
        )

        if candidates:
            results[flag_id] = {
                'name': flag_name,
                'block': block_start,
                'candidates': [(c[0], reverse_calc_block_base(c[0], flag_id)) for c in candidates]
            }
        else:
            # Fall back to non-strict search
            candidates = find_flag_in_event_flags(
                has_flag_bytes,
                not_flag_bytes,
                flag_id,
                search_start=0,
                search_end=10000,
                strict=False
            )
            # Sort by num_diff_bits (prefer less noise)
            candidates.sort(key=lambda x: x[2])
            results[flag_id] = {
                'name': flag_name,
                'block': block_start,
                'candidates': [(c[0], reverse_calc_block_base(c[0], flag_id)) for c in candidates[:5]],
                'non_strict': True
            }

    return results


def reverse_calc_block_base(byte_offset: int, flag_id: int) -> int:
    """
    Given a byte offset where a flag is stored, calculate the block base.

    Formula: byte_offset = base_offset + (flag_id - block_start) // 8
    Therefore: base_offset = byte_offset - (flag_id - block_start) // 8
    """
    block_start = (flag_id // 1000) * 1000
    relative = flag_id - block_start
    base_offset = byte_offset - (relative // 8)
    return base_offset


def main():
    # Load verification data to see what flags we know about
    jsonl_path = Path("/Users/laszloprekop/dev/Elden Ring stuff/elden-map/server/data/verification-records.jsonl")
    save_path = Path("/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2")

    print("Loading verification records...")
    records = []
    with open(jsonl_path) as f:
        for line in f:
            if line.strip():
                records.append(json.loads(line))

    # Group by slot and flag ranges
    by_slot = defaultdict(list)
    for rec in records:
        by_slot[rec["slotIndex"]].append(rec)

    print(f"Loaded {len(records)} records across slots: {sorted(by_slot.keys())}")

    # Parse save file
    print(f"\nParsing save file: {save_path}")
    parser = SaveParser()
    save_data = parser.parse(save_path)

    # Get event flags for each slot
    slot_flags = {}
    for slot in save_data.slots:
        slot_flags[slot.slot_index] = slot.event_flags
        print(f"  Slot {slot.slot_index}: {len(slot.event_flags)} bytes, validation {slot.validation_score}/4")

    # Find flags where we have different states across slots
    print("\n" + "=" * 70)
    print("SEARCHING FOR BLOCK BASES")
    print("=" * 70)

    # Focus on specific block ranges we need to discover
    target_blocks = [60000, 62000, 73000, 78000]

    for block_start in target_blocks:
        block_end = block_start + 1000

        # Find records in this block range
        block_records = []
        for rec in records:
            if block_start <= rec["flagId"] < block_end:
                block_records.append(rec)

        if not block_records:
            print(f"\n{block_start}xxx: No verification records in this range")
            continue

        print(f"\n{block_start}xxx: {len(block_records)} verification records")

        # Find flags with differing states across slots
        flag_states = defaultdict(dict)  # flag_id -> {slot_idx: status}
        for rec in block_records:
            flag_states[rec["flagId"]][rec["slotIndex"]] = rec["manualStatus"]

        # Look for flags where we have both TRUE and FALSE across different slots
        for flag_id, states in flag_states.items():
            true_slots = [s for s, v in states.items() if v]
            false_slots = [s for s, v in states.items() if not v]

            if not true_slots or not false_slots:
                continue

            # We have a good test case
            true_slot = true_slots[0]
            false_slot = false_slots[0]

            if true_slot not in slot_flags or false_slot not in slot_flags:
                continue

            flag_name = None
            for rec in block_records:
                if rec["flagId"] == flag_id:
                    flag_name = rec.get("flagName", "Unknown")
                    break

            print(f"\n  Flag {flag_id} ({flag_name}):")
            print(f"    TRUE in slot {true_slot}, FALSE in slot {false_slot}")

            # Search for where this flag is stored
            candidates = find_flag_in_event_flags(
                slot_flags[true_slot],
                slot_flags[false_slot],
                flag_id,
                search_start=0,
                search_end=10000  # Search first 10KB
            )

            if not candidates:
                print(f"    No candidates found in first 10KB!")
                # Try broader search
                candidates = find_flag_in_event_flags(
                    slot_flags[true_slot],
                    slot_flags[false_slot],
                    flag_id,
                    search_start=0,
                    search_end=50000  # Search first 50KB
                )
                if candidates:
                    print(f"    Found {len(candidates)} candidates in broader search (0-50KB)")
            else:
                print(f"    Found {len(candidates)} candidates in first 10KB")

            if candidates:
                for byte_off, bit_pos in candidates[:5]:  # Show first 5
                    base = reverse_calc_block_base(byte_off, flag_id)
                    print(f"      byte={byte_off}, bit={bit_pos} → base_offset={base}")

                # If only one candidate, that's likely correct
                if len(candidates) == 1:
                    byte_off, bit_pos = candidates[0]
                    base = reverse_calc_block_base(byte_off, flag_id)
                    print(f"    *** LIKELY CORRECT: block {block_start}xxx base = {base} ***")

            # Only test first flag that has different states
            break

    # Also do a targeted search for specific known flags
    print("\n" + "=" * 70)
    print("TARGETED FLAG SEARCH")
    print("=" * 70)

    # Use slot 4 (V3 - no pickups) as baseline for comparison
    # Compare against slot 0 (Confessor, mid-game) which has most items
    baseline_slot = 4  # V3 - no pickups
    progressed_slot = 0  # Confessor - mid-game

    if baseline_slot not in slot_flags or progressed_slot not in slot_flags:
        print(f"Error: Need both slot {baseline_slot} and slot {progressed_slot}")
        return

    baseline_flags = slot_flags[baseline_slot]
    progressed_flags = slot_flags[progressed_slot]

    print(f"\nComparing slot {progressed_slot} (progressed) vs slot {baseline_slot} (baseline)")

    # Known flags that slot 0 (Confessor) definitely has
    test_flags = [
        (60130, "Whetstone Knife"),
        (60220, "Tarnished's Furled Finger"),
        (62010, "Map: Limgrave, West"),
        (62011, "Map: Weeping Peninsula"),
        # Add more known progression flags
        (60100, "Crafting Kit"),  # Everyone gets this early
        (60110, "Memory of Grace"),  # Remembering Roundtable
    ]

    for flag_id, flag_name in test_flags:
        expected_bit = 7 - (flag_id % 8)
        block_start = (flag_id // 1000) * 1000

        print(f"\n  {flag_id} ({flag_name}):")
        print(f"    Expected bit position: {expected_bit}")

        # First try strict mode (only the expected bit differs)
        candidates = find_flag_in_event_flags(
            progressed_flags,
            baseline_flags,
            flag_id,
            search_start=0,
            search_end=10000,
            strict=True
        )

        if candidates:
            print(f"    STRICT matches ({len(candidates)}):")
            for byte_off, bit_pos, num_diff in candidates[:5]:
                base = reverse_calc_block_base(byte_off, flag_id)
                if 0 < base < 10000:
                    print(f"      byte={byte_off}, bit={bit_pos} → base={base} ✓")
        else:
            print(f"    No strict matches, trying non-strict...")
            candidates = find_flag_in_event_flags(
                progressed_flags,
                baseline_flags,
                flag_id,
                search_start=0,
                search_end=10000,
                strict=False
            )
            # Sort by num_diff_bits
            candidates.sort(key=lambda x: x[2])
            print(f"    Best non-strict ({len(candidates)} total, showing top 5):")
            for byte_off, bit_pos, num_diff in candidates[:5]:
                base = reverse_calc_block_base(byte_off, flag_id)
                if 0 < base < 10000:
                    print(f"      byte={byte_off}, bit={bit_pos}, diff_bits={num_diff} → base={base} ✓")
                else:
                    print(f"      byte={byte_off}, bit={bit_pos}, diff_bits={num_diff} → base={base}")

    # Also search for dungeon grace flags (73xxx)
    print("\n" + "=" * 70)
    print("DUNGEON GRACES (73xxx)")
    print("=" * 70)

    dungeon_graces = [
        (73000, "Murkwater Catacombs"),
        (73020, "Tombsward Catacombs"),
        (73100, "Murkwater Cave"),
        (73110, "Groveside Cave"),
    ]

    for flag_id, flag_name in dungeon_graces:
        expected_bit = 7 - (flag_id % 8)

        print(f"\n  {flag_id} ({flag_name}):")

        candidates = find_flag_in_event_flags(
            progressed_flags,
            baseline_flags,
            flag_id,
            search_start=0,
            search_end=10000,
            strict=True
        )

        if candidates:
            print(f"    STRICT matches ({len(candidates)}):")
            for byte_off, bit_pos, num_diff in candidates[:5]:
                base = reverse_calc_block_base(byte_off, flag_id)
                if 0 < base < 10000:
                    print(f"      byte={byte_off}, bit={bit_pos} → base={base} ✓")
        else:
            # Non-strict fallback
            candidates = find_flag_in_event_flags(
                progressed_flags,
                baseline_flags,
                flag_id,
                search_start=0,
                search_end=10000,
                strict=False
            )
            candidates.sort(key=lambda x: x[2])
            print(f"    Non-strict ({len(candidates)} total, best 5):")
            for byte_off, bit_pos, num_diff in candidates[:5]:
                base = reverse_calc_block_base(byte_off, flag_id)
                if 0 < base < 10000:
                    print(f"      byte={byte_off}, bit={bit_pos}, diff_bits={num_diff} → base={base} ✓")


    # Cross-validation: Compare all flags in same block to find consensus base
    print("\n" + "=" * 70)
    print("CROSS-VALIDATION ANALYSIS")
    print("=" * 70)

    all_test_flags = test_flags + dungeon_graces
    block_base_votes = defaultdict(lambda: defaultdict(int))  # block -> base -> count

    for flag_id, flag_name in all_test_flags:
        block_start = (flag_id // 1000) * 1000

        candidates = find_flag_in_event_flags(
            progressed_flags,
            baseline_flags,
            flag_id,
            search_start=0,
            search_end=10000,
            strict=True
        )

        if candidates:
            # Vote for the base offset derived from strict matches
            for byte_off, bit_pos, num_diff in candidates:
                base = reverse_calc_block_base(byte_off, flag_id)
                if 0 < base < 10000:
                    block_base_votes[block_start][base] += 1

    print("\nBase offset voting results by block:")
    for block_start in sorted(block_base_votes.keys()):
        votes = block_base_votes[block_start]
        # Sort by votes descending
        sorted_votes = sorted(votes.items(), key=lambda x: -x[1])
        print(f"\n  Block {block_start}xxx:")
        for base, count in sorted_votes[:5]:
            print(f"    base={base}: {count} votes")

        if sorted_votes:
            best_base, best_count = sorted_votes[0]
            if best_count >= 2:
                print(f"    *** LIKELY CORRECT: base = {best_base} ({best_count} consistent flags) ***")


if __name__ == "__main__":
    main()
