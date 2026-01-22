#!/usr/bin/env python3
"""Debug the graces structure in ground_truth_offsets.json."""

import json
from pathlib import Path

GROUND_TRUTH_FILE = Path("/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/ground_truth_offsets.json")

def main():
    print("Loading ground_truth_offsets.json...")
    with open(GROUND_TRUTH_FILE, 'r') as f:
        data = json.load(f)

    print(f"\nTop-level keys: {list(data.keys())}")

    # Check for graces
    if 'graces' in data:
        graces = data['graces']
        print(f"\nGraces count: {len(graces)}")

        # Show first few entries
        print("\nFirst 3 graces:")
        for grace in graces[:3]:
            print(f"  {grace}")

        # Find 71000 series
        print("\nLooking for 71000-71008:")
        found = []
        for grace in graces:
            flag_id = grace.get('flag_id', 0)
            if 71000 <= flag_id <= 71008:
                found.append(grace)
                print(f"  Found: {flag_id} - {grace.get('name')} - offset={grace.get('offset')}")

        print(f"\nTotal found: {len(found)}")

    else:
        print("\nNo 'graces' key found")

        # Check what keys might contain the data
        for key in data.keys():
            if isinstance(data[key], list) and len(data[key]) > 0:
                print(f"\n{key}: list with {len(data[key])} items")
                first_item = data[key][0]
                if isinstance(first_item, dict):
                    print(f"  First item keys: {list(first_item.keys())[:10]}")

if __name__ == "__main__":
    main()
