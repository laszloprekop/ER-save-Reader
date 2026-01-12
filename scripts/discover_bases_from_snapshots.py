#!/usr/bin/env python3
"""
Discover block bases using granular snapshots with confirmed EventFlags offsets.

Strategy:
1. Parse each snapshot pair (before/after)
2. Find the exact bytes that changed
3. Calculate what block base would produce those offsets
4. Cross-validate across multiple pairs
"""

import sys
import struct
from pathlib import Path
from collections import defaultdict

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser

snapshot_dir = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")
parser = SaveParser()


def diff_event_flags(before_flags: bytes, after_flags: bytes):
    """Find all bit changes between two event flag sections."""
    changes = []
    min_len = min(len(before_flags), len(after_flags))

    for byte_off in range(min_len):
        if before_flags[byte_off] != after_flags[byte_off]:
            before_byte = before_flags[byte_off]
            after_byte = after_flags[byte_off]

            # Find bits that were SET (0->1)
            set_bits = after_byte & ~before_byte
            # Find bits that were CLEARED (1->0)
            cleared_bits = before_byte & ~after_byte

            for bit in range(8):
                if (set_bits >> bit) & 1:
                    changes.append({
                        'byte': byte_off,
                        'bit': bit,  # Physical bit position
                        'direction': 'SET',
                        'before': f'0x{before_byte:02X}',
                        'after': f'0x{after_byte:02X}',
                    })
                if (cleared_bits >> bit) & 1:
                    changes.append({
                        'byte': byte_off,
                        'bit': bit,
                        'direction': 'CLEARED',
                        'before': f'0x{before_byte:02X}',
                        'after': f'0x{after_byte:02X}',
                    })

    return changes


def calc_flag_from_offset(byte_offset: int, bit_pos: int, block_start: int, base_offset: int) -> int:
    """Calculate flag ID from byte offset and bit position given known base."""
    # flag_id = block_start + (byte_offset - base_offset) * 8 + (7 - bit_pos)
    relative_byte = byte_offset - base_offset
    flag_id = block_start + relative_byte * 8 + (7 - bit_pos)
    return flag_id


def calc_base_from_flag(byte_offset: int, bit_pos: int, flag_id: int) -> int:
    """Calculate base offset from known flag and its byte location."""
    # flag_id = block_start + (byte_offset - base_offset) * 8 + (7 - bit_pos)
    # base_offset = byte_offset - (flag_id - block_start - (7 - bit_pos)) / 8
    block_start = (flag_id // 1000) * 1000
    remainder = 7 - bit_pos
    relative = flag_id - block_start
    # relative = (byte_offset - base_offset) * 8 + remainder
    # byte_offset - base_offset = (relative - remainder) / 8
    base_offset = byte_offset - (relative - remainder) // 8
    return base_offset


# Define snapshot pairs with expected changes
# (before_file, after_file, slot, expected_flag_id, expected_name)
known_pairs = [
    # Verified graces (we know these work)
    ("ER0000.sl2 Wretch - 21 Limgrave, before Gatefront grace",
     "ER0000.sl2 Wretch - 22 Limgrave, touched Gatefront grace",
     1, 76102, "Gatefront Grace"),

    # Agheel Lake North
    ("ER0000.sl2 Wretch - 27 Limgrave, approaching Agheel Lake North grace, on mount",
     "ER0000.sl2 Wretch - 28 Limgrave, touched Agheel Lake North grace, dismounted",
     1, 76103, "Agheel Lake North"),

    # Cookbook - we know base 3987 (67xxx)
    ("ER0000.sl2 Confessor - 01 before Missionary Cookbok [4] pickup",
     "ER0000.sl2 Confessor - 02 after Missionary Cookbok [4] picked up",
     0, 67030, "Missionary Cookbook [4]"),

    # Minor Erdtree Church grace
    ("ER0000.sl2 Confessor - 03 before touching  Minor Eldtree Church grace",
     "ER0000.sl2 Confessor - 04 after touched Minor Eldtree Church grace",
     0, None, "Minor Erdtree Church Grace"),  # Unknown flag ID
]

print("=" * 70)
print("DISCOVERING BLOCK BASES FROM GRANULAR SNAPSHOTS")
print("=" * 70)

discovered_bases = defaultdict(list)  # block_start -> [(flag_id, base_offset, source)]

for before_file, after_file, slot_idx, expected_flag, expected_name in known_pairs:
    before_path = snapshot_dir / before_file
    after_path = snapshot_dir / after_file

    if not before_path.exists() or not after_path.exists():
        print(f"\nSkipping {expected_name}: Files not found")
        continue

    print(f"\n{'=' * 70}")
    print(f"{expected_name}")
    print(f"{'=' * 70}")
    print(f"Before: {before_file}")
    print(f"After:  {after_file}")

    # Parse both saves
    before_save = parser.parse(before_path, [slot_idx])
    after_save = parser.parse(after_path, [slot_idx])

    if not before_save.slots or not after_save.slots:
        print("  Error: Could not parse slots")
        continue

    before_slot = before_save.slots[0]
    after_slot = after_save.slots[0]

    print(f"Before EventFlags offset: 0x{before_slot.event_flags_offset:X}")
    print(f"After EventFlags offset: 0x{after_slot.event_flags_offset:X}")

    # Ensure offsets match (otherwise comparison is invalid)
    if before_slot.event_flags_offset != after_slot.event_flags_offset:
        print("  WARNING: EventFlags offsets differ!")
        # Continue anyway - the diff should still work within each section

    before_ef = before_slot.event_flags
    after_ef = after_slot.event_flags

    # Find differences
    changes = diff_event_flags(before_ef, after_ef)
    set_changes = [c for c in changes if c['direction'] == 'SET']

    print(f"\nTotal bit changes: {len(changes)}")
    print(f"SET changes: {len(set_changes)}")

    # If we have an expected flag ID, verify it
    if expected_flag:
        block_start = (expected_flag // 1000) * 1000
        expected_bit = 7 - (expected_flag % 8)

        print(f"\nExpected flag {expected_flag}:")
        print(f"  Block: {block_start}xxx")
        print(f"  Expected bit position: {expected_bit}")

        # Look for SET changes that could be this flag
        for c in set_changes:
            if c['bit'] == expected_bit:
                # This change could be our flag
                base = calc_base_from_flag(c['byte'], c['bit'], expected_flag)
                print(f"\n  Candidate match at byte {c['byte']}:")
                print(f"    Byte changed: {c['before']} -> {c['after']}")
                print(f"    Calculated base offset: {base}")

                # Verify by calculating flag ID from this base
                calc_flag = calc_flag_from_offset(c['byte'], c['bit'], block_start, base)
                print(f"    Verification: base={base}, byte={c['byte']}, bit={c['bit']} -> flag {calc_flag}")

                if calc_flag == expected_flag:
                    print(f"    *** VERIFIED: base={base} produces flag {expected_flag} ***")
                    discovered_bases[block_start].append((expected_flag, base, expected_name))
    else:
        # Unknown flag - show top SET changes
        print(f"\nTop SET changes:")
        for c in set_changes[:10]:
            print(f"  byte={c['byte']}, bit={c['bit']}: {c['before']} -> {c['after']}")

# Summary
print("\n" + "=" * 70)
print("DISCOVERY SUMMARY")
print("=" * 70)

for block_start in sorted(discovered_bases.keys()):
    bases = discovered_bases[block_start]
    print(f"\nBlock {block_start}xxx:")
    for flag_id, base, source in bases:
        print(f"  flag {flag_id} ({source}): base = {base}")

    # Check if all bases agree
    unique_bases = set(b for _, b, _ in bases)
    if len(unique_bases) == 1:
        print(f"  *** CONFIRMED: base = {unique_bases.pop()} ***")
    elif len(unique_bases) > 1:
        print(f"  WARNING: Conflicting bases: {unique_bases}")
