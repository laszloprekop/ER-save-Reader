#!/usr/bin/env python3
"""
Update Block 71000 offsets in ground_truth_offsets.json.

Changes:
- Old base: 2625 (for individual flags) / 2673 (for block)
- New base: 9315 (verified 2026-01-22)

Formula: offset = 9315 + (flag_id - 71000) // 8
         bit = 7 - (flag_id - 71000) % 8
"""

import json
from pathlib import Path

GROUND_TRUTH_FILE = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/ground_truth_offsets.json")
NEW_BASE = 9315

def calculate_offset_bit(flag_id, base=NEW_BASE):
    """Calculate offset and bit for a grace flag."""
    local = flag_id - 71000
    offset = base + local // 8
    bit = 7 - (local % 8)
    return offset, bit

def main():
    print("Loading ground_truth_offsets.json...")
    with open(GROUND_TRUTH_FILE, 'r') as f:
        data = json.load(f)

    # Update all_flags array (the correct key name)
    updates_made = 0
    if 'all_flags' in data:
        print(f"Found all_flags with {len(data['all_flags'])} entries")
        for flag_entry in data['all_flags']:
            flag_id = flag_entry.get('flag_id', 0)
            if 71000 <= flag_id <= 71008:
                new_offset, new_bit = calculate_offset_bit(flag_id)
                old_offset = flag_entry.get('offset')
                old_bit = flag_entry.get('bit')

                if old_offset != new_offset or old_bit != new_bit:
                    print(f"  Updating flag {flag_id} ({flag_entry.get('name', 'Unknown')}):")
                    print(f"    offset: {old_offset} -> {new_offset}")
                    print(f"    bit: {old_bit} -> {new_bit}")

                    flag_entry['offset'] = new_offset
                    flag_entry['bit'] = new_bit

                    # Also update formula_results if present
                    if 'formula_results' in flag_entry and 'block' in flag_entry['formula_results']:
                        flag_entry['formula_results']['block']['offset'] = new_offset
                        flag_entry['formula_results']['block']['bit'] = new_bit

                    updates_made += 1
                else:
                    print(f"  Flag {flag_id} already has correct offsets: {new_offset}, bit {new_bit}")

    print(f"\nUpdated {updates_made} all_flags entries")

    # Save updated data
    print("\nSaving updated ground_truth_offsets.json...")
    with open(GROUND_TRUTH_FILE, 'w') as f:
        json.dump(data, f, indent=2)

    print("Done!")

    # Verify changes
    print("\nVerification - checking 71000-71008 offsets:")
    for flag_id in range(71000, 71009):
        expected_offset, expected_bit = calculate_offset_bit(flag_id)
        print(f"  {flag_id}: offset={expected_offset}, bit={expected_bit}")

if __name__ == "__main__":
    main()
