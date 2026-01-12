#!/usr/bin/env python3
"""Check EventFlags offset detection in granular snapshots."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from verification.save_parser import SaveParser

snapshot_dir = Path("/Users/laszloprekop/dev/Elden Ring stuff/Elden Ring save files/Granular snapshots for debugging")
parser = SaveParser()

# Key snapshots to check
snapshots = [
    ("ER0000.sl2 Wretch - 03 Cave of knowledge, before Site of grace", 1),
    ("ER0000.sl2 Wretch - 04 Cave of knowledge, touched Site of grace", 1),
    ("ER0000.sl2 Wretch - 11 Stranded Graveyard, before touching grace", 1),
    ("ER0000.sl2 Wretch - 12 Stranded Graveyard, after touching grace", 1),
    ("ER0000.sl2 Wretch - 14 Limgrave, before The First Step grace", 1),
    ("ER0000.sl2 Wretch - 15 Limgrave, touched The First Step grace", 1),
    ("ER0000.sl2 Confessor - 01 before Missionary Cookbok [4] pickup", 0),
    ("ER0000.sl2 Confessor - 02 after Missionary Cookbok [4] picked up", 0),
]

print("Checking EventFlags offset detection in snapshots:\n")

for filename, slot_idx in snapshots:
    filepath = snapshot_dir / filename
    if not filepath.exists():
        print(f"  {filename}: NOT FOUND")
        continue

    save = parser.parse(filepath, [slot_idx])
    if not save.slots:
        print(f"  {filename}: No slots parsed")
        continue

    slot = save.slots[0]
    print(f"  Slot {slot_idx}: offset=0x{slot.event_flags_offset:X}, validation={slot.validation_score}/4")
    print(f"    File: {filename}")

    # Check validation flags at their known offsets
    # 71800: byte 2725, bit 7
    # 76100: byte 3262, bit 3
    ef = slot.event_flags
    flag_71800 = bool((ef[2725] >> 7) & 1) if len(ef) > 2725 else False
    flag_76100 = bool((ef[3262] >> 3) & 1) if len(ef) > 3262 else False
    print(f"    71800 (Cave of Knowledge): {flag_71800}")
    print(f"    76100 (The First Step): {flag_76100}")
    print()
