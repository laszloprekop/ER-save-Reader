#!/usr/bin/env python3
"""
Verify map fragment base 9359 across multiple character slots.
"""

from typing import Optional

SAVE_PATH = "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing/76561197969778805/ER0000.sl2"

SLOT_SIZE = 0x280020
HEADER_SIZE = 0x310

VALIDATION_FLAGS = [
    (71800, 2725, 7, "Cave of Knowledge grace"),
    (71801, 2725, 6, "Stranded Graveyard"),
]

# Character expectations based on CLAUDE.md:
# Slot 0: Confessor, mid-game - has many maps
# Slot 1: Wretch, early game - probably only Limgrave maps
# Slot 2-4: V1/V2/V3 - very little progression
# Slot 5: Sam, level 10 - early game

SLOT_EXPECTATIONS = {
    0: {
        "name": "Confessor",
        "maps": ["62010", "62011", "62012", "62020", "62021", "62022", "62030", "62032", "62040", "62041"],
        "no_maps": ["62050", "62051", "62080"],  # Late game
    },
    1: {
        "name": "Wretch",
        "maps": ["62010"],  # Probably just Limgrave West
        "no_maps": ["62050", "62051", "62030", "62080"],  # Definitely no late game
    },
    5: {
        "name": "Sam",
        "maps": [],  # Uncertain
        "no_maps": ["62050", "62051", "62080"],  # Definitely no late game
    },
}

MAP_NAMES = {
    62010: "Limgrave, West",
    62011: "Weeping Peninsula",
    62012: "Limgrave, East",
    62020: "Liurnia, East",
    62021: "Liurnia, North",
    62022: "Liurnia, West",
    62030: "Altus Plateau",
    62031: "Capital Outskirts",
    62032: "Mt. Gelmir",
    62040: "Caelid",
    62041: "Dragonbarrow",
    62050: "Mountaintops, West",
    62051: "Mountaintops, East",
    62060: "Ainsel River",
    62061: "Lake of Rot",
    62062: "Deeproot Depths",
    62063: "Siofra River",
    62070: "Lake of Rot (alt)",
    62080: "Mohgwyn Palace",
}


def detect_event_flags_offset(slot_data: bytes) -> Optional[int]:
    for test_offset in range(0x12000, 0x15000):
        score = 0
        for flag_id, byte_offset, bit_pos, name in VALIDATION_FLAGS:
            abs_pos = test_offset + byte_offset
            if abs_pos < len(slot_data):
                byte_val = slot_data[abs_pos]
                if (byte_val & (1 << bit_pos)) != 0:
                    score += 1
        if score == len(VALIDATION_FLAGS):
            return test_offset
    return None


def check_flag(event_flags: bytes, flag_id: int, block_start: int, base: int) -> Optional[bool]:
    relative = flag_id - block_start
    byte_offset = base + relative // 8
    bit_pos = 7 - (flag_id % 8)

    if byte_offset >= len(event_flags) or byte_offset < 0:
        return None

    byte_val = event_flags[byte_offset]
    return (byte_val >> bit_pos) & 1 == 1


def main():
    BASE = 9359

    print(f"Testing map fragment base {BASE} across multiple slots")
    print("="*80)

    with open(SAVE_PATH, 'rb') as f:
        for slot in [0, 1, 5]:
            f.seek(HEADER_SIZE + slot * SLOT_SIZE)
            slot_data = f.read(SLOT_SIZE)

            event_flags_offset = detect_event_flags_offset(slot_data)
            if event_flags_offset is None:
                print(f"\nSlot {slot}: Could not detect event_flags offset")
                continue

            event_flags = slot_data[event_flags_offset:]
            expectations = SLOT_EXPECTATIONS.get(slot, {})

            print(f"\n{'='*60}")
            print(f"Slot {slot}: {expectations.get('name', 'Unknown')}")
            print(f"Event flags offset: 0x{event_flags_offset:X}")
            print("-"*60)

            # Show all map flags
            print("\nAll map fragments at base 9359:")
            for flag_id in sorted(MAP_NAMES.keys()):
                name = MAP_NAMES[flag_id]
                actual = check_flag(event_flags, flag_id, 62000, BASE)
                status = "SET" if actual else "---"

                # Check expectations
                if str(flag_id) in expectations.get('maps', []):
                    expected = "expected SET"
                elif str(flag_id) in expectations.get('no_maps', []):
                    expected = "expected ---"
                else:
                    expected = ""

                match = ""
                if expected:
                    if ("SET" in expected and actual) or ("---" in expected and not actual):
                        match = "✓"
                    else:
                        match = "✗"

                print(f"  {flag_id} {name:25} {status:3} {match:2} {expected}")


if __name__ == "__main__":
    main()
