#!/usr/bin/env python3
"""Check the freshly created save to understand baseline flag states."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser

snapshot_dir = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")
parser = SaveParser()

# Check a series of early snapshots to find when flags get set
early_snapshots = [
    "ER0000.sl2 Wretch - 00 freshly created",
    "ER0000.sl2 Wretch - 01 Tarnished Wizened Finger taken",
    "ER0000.sl2 Wretch - 02 Opened door of starter building",
    "ER0000.sl2 Wretch - 03 Cave of knowledge, before Site of grace",
    "ER0000.sl2 Wretch - 04 Cave of knowledge, touched Site of grace",
]

print("Tracking flag states through early game progression:\n")
print(f"{'Snapshot':<60} | 71800 | 71801 | 76100 | 76101")
print("-" * 90)

for filename in early_snapshots:
    filepath = snapshot_dir / filename
    if not filepath.exists():
        continue

    save = parser.parse(filepath, [1])  # Slot 1 = Wretch
    if not save.slots:
        continue

    slot = save.slots[0]
    ef = slot.event_flags

    # Check known flags
    f71800 = bool((ef[2725] >> 7) & 1) if len(ef) > 2725 else False
    f71801 = bool((ef[2725] >> 6) & 1) if len(ef) > 2725 else False
    f76100 = bool((ef[3262] >> 3) & 1) if len(ef) > 3262 else False
    f76101 = bool((ef[3262] >> 2) & 1) if len(ef) > 3262 else False

    short_name = filename.split(" - ", 1)[1] if " - " in filename else filename
    print(f"{short_name:<60} | {str(f71800):<5} | {str(f71801):<5} | {str(f76100):<5} | {str(f76101):<5}")

print("\n")

# Now let's diff between 00 and 03 to see what flags changed
print("Diffing Wretch 00 vs 03 to find what flags were set during character intro:\n")

f00 = snapshot_dir / "ER0000.sl2 Wretch - 00 freshly created"
f03 = snapshot_dir / "ER0000.sl2 Wretch - 03 Cave of knowledge, before Site of grace"

if f00.exists() and f03.exists():
    save00 = parser.parse(f00, [1])
    save03 = parser.parse(f03, [1])

    if save00.slots and save03.slots:
        ef00 = save00.slots[0].event_flags
        ef03 = save03.slots[0].event_flags

        # Find all SET changes (flags that were clear in 00 and set in 03)
        set_changes = []
        min_len = min(len(ef00), len(ef03))

        for byte_off in range(min_len):
            if ef00[byte_off] != ef03[byte_off]:
                diff = ef03[byte_off] & ~ef00[byte_off]  # Bits set in 03 but not in 00
                for bit in range(8):
                    if (diff >> bit) & 1:
                        logical_bit = 7 - bit
                        set_changes.append((byte_off, logical_bit))

        print(f"Total flags SET between 00 and 03: {len(set_changes)}")

        # Check if byte 2725 is in the changes
        b2725_changes = [(b, bit) for b, bit in set_changes if b == 2725]
        print(f"\nByte 2725 changes (71800/71801 location): {b2725_changes}")

        # Check if byte 3262 is in the changes
        b3262_changes = [(b, bit) for b, bit in set_changes if b == 3262]
        print(f"Byte 3262 changes (76100/76101 location): {b3262_changes}")

        # Show changes in known ranges
        print("\nChanges in 2700-2750 range (tutorial graces):")
        for byte_off, logical_bit in set_changes:
            if 2700 <= byte_off <= 2750:
                print(f"  byte={byte_off}, bit={logical_bit}")

        print("\nChanges in 3250-3300 range (world graces):")
        for byte_off, logical_bit in set_changes:
            if 3250 <= byte_off <= 3300:
                print(f"  byte={byte_off}, bit={logical_bit}")
