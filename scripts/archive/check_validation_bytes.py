#!/usr/bin/env python3
"""Check the actual byte values at validation flag locations."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser

snapshot_dir = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")
parser = SaveParser()

# Check bytes at known validation flag locations
# 71800: byte 2725, bit 7 (physical), base=2625
# 71801: byte 2725, bit 6 (physical), base=2625
# 76100: byte 3262, bit 3 (physical), base=3250
# 76101: byte 3262, bit 2 (physical), base=3250

snapshots = [
    ("ER0000.sl2 Wretch - 00 freshly created", 1),
    ("ER0000.sl2 Wretch - 03 Cave of knowledge, before Site of grace", 1),
    ("ER0000.sl2 Wretch - 04 Cave of knowledge, touched Site of grace", 1),
]

print("Checking actual byte values at validation flag locations:\n")
print(f"{'Snapshot':<55} | offset   | byte[2725] | byte[3262]")
print("-" * 95)

for filename, slot_idx in snapshots:
    filepath = snapshot_dir / filename
    if not filepath.exists():
        continue

    save = parser.parse(filepath, [slot_idx])
    if not save.slots:
        continue

    slot = save.slots[0]
    ef = slot.event_flags

    b2725 = ef[2725] if len(ef) > 2725 else 0
    b3262 = ef[3262] if len(ef) > 3262 else 0

    short_name = filename.split(" - ", 1)[1][:50] if " - " in filename else filename[:50]
    print(f"{short_name:<55} | 0x{slot.event_flags_offset:05X} | 0x{b2725:02X} ({b2725:08b}) | 0x{b3262:02X} ({b3262:08b})")

print("\n")
print("Bit analysis:")
print("  byte[2725] bit 7 (71800 Cave of Knowledge): extracted with (byte >> 7) & 1")
print("  byte[2725] bit 6 (71801 Stranded Graveyard): extracted with (byte >> 6) & 1")
print("  byte[3262] bit 3 (76100 The First Step): extracted with (byte >> 3) & 1")
print("  byte[3262] bit 2 (76101 Church of Elleh): extracted with (byte >> 2) & 1")

# Now let's also check what the validation detection code does
print("\n\nValidation flag check (from parser code):")
print("  VALIDATION_FLAGS = [")
print("    (71800, 2725, 7, 'Cave of Knowledge'),")
print("    (71801, 2725, 6, 'Stranded Graveyard'),")
print("    (76100, 3262, 3, 'The First Step'),")
print("    (76101, 3262, 2, 'Church of Elleh'),")
print("  ]")
print("  Check: (slot_data[offset + byte_off] & (1 << bit_pos)) != 0")

for filename, slot_idx in snapshots:
    filepath = snapshot_dir / filename
    if not filepath.exists():
        continue

    save = parser.parse(filepath, [slot_idx])
    if not save.slots:
        continue

    slot = save.slots[0]
    ef = slot.event_flags

    short_name = filename.split(" - ", 1)[1][:35] if " - " in filename else filename[:35]

    # Check using the parser's method
    b2725 = ef[2725] if len(ef) > 2725 else 0
    b3262 = ef[3262] if len(ef) > 3262 else 0

    # Using (1 << bit_pos) check
    c71800 = bool(b2725 & (1 << 7))
    c71801 = bool(b2725 & (1 << 6))
    c76100 = bool(b3262 & (1 << 3))
    c76101 = bool(b3262 & (1 << 2))

    print(f"\n  {short_name}")
    print(f"    71800: byte 0x{b2725:02X} & 0x{1<<7:02X} = 0x{b2725 & (1<<7):02X} -> {c71800}")
    print(f"    71801: byte 0x{b2725:02X} & 0x{1<<6:02X} = 0x{b2725 & (1<<6):02X} -> {c71801}")
    print(f"    76100: byte 0x{b3262:02X} & 0x{1<<3:02X} = 0x{b3262 & (1<<3):02X} -> {c76100}")
    print(f"    76101: byte 0x{b3262:02X} & 0x{1<<2:02X} = 0x{b3262 & (1<<2):02X} -> {c76101}")
