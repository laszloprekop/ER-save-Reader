#!/usr/bin/env python3
"""
Timeline Narrative Builder for Elden Ring Save Diffs

Reads binary diff files (6-byte records: 4-byte LE offset + old byte + new byte)
from granular snapshot captures and reconstructs a gameplay timeline by identifying
event flag toggles, correlating with player position and inventory changes.

Event Flag Geography:
  - Block flags (5-6 digit): byte_offset = block_base + (flag_id - block_start) / 8
  - Tile flags (10 digit): tile formula with row/col/local_id
  - World pickup row ID formula: byte_offset = (row_id - 1037373320) / 8
"""

import json
import struct
import sys
from pathlib import Path
from collections import defaultdict
from datetime import datetime

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
TIMELINE_DIR = Path(
    "/Users/laszloprekop/dev/Elden Ring stuff/"
    "Elden Ring save files/Granular snapshots for debugging/timeline"
)
DIFF_DIR = TIMELINE_DIR / "slot_diffs"
JSONL_PATH = TIMELINE_DIR / "slot_changes.jsonl"
GROUND_TRUTH_PATH = Path(
    "/Users/laszloprekop/dev/Elden Ring stuff/ER-save-Editor/ground_truth_offsets.json"
)

# ---------------------------------------------------------------------------
# Block flag bases (from EVENT-FLAG-GEOGRAPHY.md and ground_truth_offsets.json)
# ---------------------------------------------------------------------------
BLOCK_BASES = {
    60000: {"base": 2548, "name": "Progression", "status": "verified"},
    61000: {"base": 2671, "name": "Map Area Visit", "status": "verified"},
    65000: {"base": 37412, "name": "Crystal Tears / Whetblades", "status": "verified"},
    68000: {"base": 37536, "name": "Cookbooks (68k)", "status": "needs_investigation"},
    71000: {"base": 9315, "name": "Stormveil Graces", "status": "unreliable"},
    71100: {"base": 2593, "name": "Graces (71100)", "status": "unreliable"},
    71600: {"base": 3198, "name": "Graces (71600)", "status": "unreliable"},
    71800: {"base": 2725, "name": "Tutorial Graces", "status": "verified"},
    72000: {"base": 2750, "name": "Graces (72k)", "status": "verified"},
    73000: {"base": 2662, "name": "Graces (73k)", "status": "unreliable"},
    74000: {"base": 3000, "name": "Graces (74k)", "status": "verified"},
    76000: {"base": 3250, "name": "Graces (76k)", "status": "verified"},
    78000: {"base": 3500, "name": "Stakes of Marika", "status": "verified"},
    520000: {"base": 1341, "name": "Spirit Ashes/Talismans", "status": "verified"},
}

# Grace flag ranges from EVENT-FLAG-GEOGRAPHY.md
GRACE_RANGES = {
    (76000, 76999): "Graces (primary, Limgrave/Liurnia/Altus)",
    (77000, 77999): "Graces (secondary)",
    (78000, 78999): "Stakes of Marika",
    (79000, 79999): "Graces (endgame)",
    (71000, 71999): "Graces (Stormveil/tutorial)",
    (72000, 72999): "Graces (72k range)",
    (73000, 73999): "Graces (73k range)",
    (74000, 74999): "Graces (74k range)",
}

LANDMARK_REGIONS = {
    (62100, 62138): "Limgrave",
    (62150, 62184): "Weeping Peninsula",
    (62200, 62284): "Liurnia of the Lakes",
    (62300, 62348): "Altus Plateau",
    (62350, 62389): "Mt. Gelmir",
    (62400, 62438): "Caelid",
    (62460, 62475): "Greyoll's Dragonbarrow",
    (62510, 62531): "Mountaintops of the Giants",
    (62550, 62574): "Consecrated Snowfield",
    (62610, 62634): "Siofra River",
    (62640, 62640): "Ainsel River",
    (62700, 62740): "Deeproot Depths",
    (62800, 62831): "Mohgwyn Palace",
    (62840, 62844): "Lake of Rot",
    (62850, 62891): "Nokron / Nokstella",
    (62900, 62943): "Leyndell",
    (62950, 62981): "Crumbling Farum Azula",
}

# World pickup row ID base
WORLD_PICKUP_ROW_ID_BASE = 1037373320

# Tile region mapping (XX, YY ranges -> region name)
TILE_REGIONS = [
    ((42, 44), (36, 40), "Limgrave"),
    ((40, 43), (33, 35), "Weeping Peninsula"),
    ((37, 44), (41, 47), "Liurnia of the Lakes"),
    ((37, 44), (48, 52), "Altus Plateau"),
    ((33, 38), (48, 52), "Mt. Gelmir"),
    ((46, 54), (36, 44), "Caelid"),
    ((48, 54), (45, 50), "Greyoll's Dragonbarrow"),
    ((37, 44), (53, 58), "Mountaintops of the Giants"),
    ((33, 38), (55, 58), "Consecrated Snowfield"),
]

# Area name lookup for mapId
AREA_NAMES = {
    10: "Stormveil Castle",
    11: "Raya Lucaria Academy",
    12: "Underground (Siofra/Ainsel)",
    13: "Leyndell, Royal Capital",
    14: "Tutorial Areas",
    15: "Miquella's Haligtree",
    16: "Volcano Manor",
    18: "Roundtable Hold",
    19: "Elden Throne / Chapel of Anticipation",
    20: "Stranded Graveyard",
    30: "Catacombs",
    31: "Caves",
    32: "Tunnels/Mines",
    33: "Hero's Graves",
    34: "Divine Towers",
    35: "Mohgwyn Palace",
    39: "Deeproot Depths",
    60: "Overworld (m60)",
}


def decode_map_id(map_id):
    """Decode mapId [AA, BB, CC, DD] to a human-readable location."""
    if not map_id or len(map_id) < 4:
        return "Unknown"
    aa, bb, cc, dd = map_id
    if dd == 60:
        # Overworld tile
        region = "Overworld"
        for (x_lo, x_hi), (y_lo, y_hi), name in TILE_REGIONS:
            if x_lo <= bb <= x_hi and y_lo <= aa <= y_hi:
                region = name
                break
        return f"{region} (tile [{aa},{bb}], m60)"
    elif dd == 18:
        return "Roundtable Hold"
    else:
        area_name = AREA_NAMES.get(dd, f"Area {dd}")
        section = cc
        return f"{area_name} (section {section})"


def tile_region_from_coords(xx, yy):
    """Map tile coordinates to a region name."""
    for (x_lo, x_hi), (y_lo, y_hi), name in TILE_REGIONS:
        if x_lo <= xx <= x_hi and y_lo <= yy <= y_hi:
            return name
    return f"Unknown tile region ({xx},{yy})"


def flag_id_to_block_offset(flag_id, block_start, block_base):
    """Calculate byte offset and bit position from flag_id using block formula."""
    byte_offset = block_base + (flag_id - block_start) // 8
    bit_position = 7 - (flag_id % 8)
    return byte_offset, bit_position


def offset_to_flag_id(byte_offset, bit_position, block_start, block_base):
    """Reverse: given byte_offset + bit_position, compute flag_id."""
    relative_byte = byte_offset - block_base
    if relative_byte < 0 or relative_byte >= 125:  # 1000 flags / 8 = 125 bytes
        return None
    flag_id = block_start + relative_byte * 8 + (7 - bit_position)
    return flag_id


def classify_flag_id(flag_id):
    """Classify a flag_id into a category with description."""
    if 60000 <= flag_id <= 60999:
        return "Progression"
    elif 61000 <= flag_id <= 61999:
        return "Map Area Visit"
    elif 62000 <= flag_id <= 62999:
        for (lo, hi), region in LANDMARK_REGIONS.items():
            if lo <= flag_id <= hi:
                return f"Landmark ({region})"
        return "Landmark/Map"
    elif 63000 <= flag_id <= 63999:
        return "Map Discovery (internal)"
    elif 65000 <= flag_id <= 65999:
        return "Crystal Tears / Whetblades"
    elif 66000 <= flag_id <= 66999:
        return "Pot/Perfume Upgrades"
    elif 67000 <= flag_id <= 67999:
        return "Cookbooks (67k)"
    elif 68000 <= flag_id <= 68999:
        return "Cookbooks (68k)"
    elif 69000 <= flag_id <= 69999:
        return "Remembrance / Notes"
    elif 71000 <= flag_id <= 75999:
        return "Grace"
    elif 76000 <= flag_id <= 79999:
        for (lo, hi), desc in GRACE_RANGES.items():
            if lo <= flag_id <= hi:
                return desc
        return "Grace / Stake"
    elif 91000 <= flag_id <= 91999:
        return "Boss Remembrance"
    elif 92000 <= flag_id <= 92999:
        return "Container Upgrades"
    elif 520000 <= flag_id <= 520999:
        return "Spirit Ashes / Talismans"
    else:
        return f"Unknown range ({flag_id})"


def load_ground_truth_flags():
    """Load verified flags for reverse-lookup from offset -> flag name."""
    lookup = {}  # (offset, bit) -> {name, flag_id, category}
    try:
        with open(GROUND_TRUTH_PATH) as f:
            data = json.load(f)
        # From verified_flags
        vf = data.get("verified_flags", {})
        for fid_str, info in vf.items():
            off = info.get("offset")
            bit = info.get("bit")
            if off is not None and bit is not None:
                lookup[(off, bit)] = {
                    "flag_id": int(fid_str),
                    "name": info.get("name", ""),
                    "category": info.get("category", ""),
                    "status": info.get("status", ""),
                }
        # From all_flags
        for flag in data.get("all_flags", []):
            off = flag.get("offset")
            bit = flag.get("bit")
            fid = flag.get("flag_id")
            if off is not None and bit is not None and fid is not None:
                key = (off, bit)
                if key not in lookup:
                    lookup[key] = {
                        "flag_id": fid,
                        "name": flag.get("name", ""),
                        "category": flag.get("category", ""),
                        "status": flag.get("status", ""),
                    }
    except Exception as e:
        print(f"Warning: Could not load ground truth: {e}", file=sys.stderr)
    return lookup


def parse_diff_file(path):
    """Parse a binary diff file into list of change records."""
    records = []
    with open(path, "rb") as f:
        data = f.read()
    for i in range(0, len(data), 6):
        if i + 6 > len(data):
            break
        offset = struct.unpack_from("<I", data, i)[0]
        old_byte = data[i + 4]
        new_byte = data[i + 5]
        records.append((offset, old_byte, new_byte))
    return records


def analyze_change(offset, old_byte, new_byte, gt_lookup):
    """
    Analyze a single byte change.
    Returns a dict with classification information.
    """
    xor = old_byte ^ new_byte
    bits_changed = bin(xor).count("1")

    result = {
        "offset": offset,
        "old": old_byte,
        "new": new_byte,
        "xor": xor,
        "bits_changed": bits_changed,
        "is_single_bit": bits_changed == 1,
        "flag_matches": [],
    }

    if bits_changed == 1:
        # Find which bit
        for b in range(8):
            if xor & (1 << b):
                bit_pos_from_lsb = b
                bit_pos_from_msb = 7 - b  # Standard bit numbering (MSB=7)
                break

        is_set = bool(new_byte & xor)  # bit went 0->1
        result["bit_set"] = is_set
        result["bit_position"] = bit_pos_from_msb  # As used in ER flag convention

        # Try to match against known block bases
        for block_start, block_info in BLOCK_BASES.items():
            base = block_info["base"]
            flag_id = offset_to_flag_id(offset, bit_pos_from_msb, block_start, base)
            if flag_id is not None and block_start <= flag_id < block_start + 1000:
                category = classify_flag_id(flag_id)
                # Check ground truth
                gt = gt_lookup.get((offset, bit_pos_from_msb))
                gt_name = gt["name"] if gt else None
                result["flag_matches"].append({
                    "flag_id": flag_id,
                    "block": block_start,
                    "block_name": block_info["name"],
                    "block_status": block_info["status"],
                    "category": category,
                    "gt_name": gt_name,
                })

        # Also check ground truth directly (for any block)
        gt = gt_lookup.get((offset, bit_pos_from_msb))
        if gt and not any(m.get("gt_name") for m in result["flag_matches"]):
            result["flag_matches"].append({
                "flag_id": gt["flag_id"],
                "block": None,
                "block_name": "ground_truth_direct",
                "block_status": gt["status"],
                "category": gt["category"],
                "gt_name": gt["name"],
            })

        # Try world pickup row_id formula for high offsets
        if offset > 100000:
            # row_id = WORLD_PICKUP_ROW_ID_BASE + (offset * 8) + (7 - bit_pos_from_msb)
            row_id = WORLD_PICKUP_ROW_ID_BASE + offset * 8 + (7 - bit_pos_from_msb)
            # Check if it looks like a valid tile flag (1XXYYZZZZ)
            if 1000000000 <= row_id <= 2999999999:
                row_str = str(row_id)
                prefix = int(row_str[0])
                xx = int(row_str[1:3])
                yy = int(row_str[3:5])
                local_id = int(row_str[5:])
                if 30 <= xx <= 60 and 30 <= yy <= 60 and local_id < 10000:
                    region = tile_region_from_coords(xx, yy)
                    result["flag_matches"].append({
                        "flag_id": row_id,
                        "block": None,
                        "block_name": "World Pickup (row_id formula)",
                        "block_status": "formula",
                        "category": f"Tile pickup ({region}, [{xx},{yy}], local={local_id})",
                        "gt_name": None,
                    })

    return result


def categorize_offset_region(offset):
    """Categorize an offset into a save-file structural region."""
    if offset < 32:
        return "Header/Checksum"
    elif offset < 100:
        return "Character Stats"
    elif offset < 1000:
        return "Character Data"
    elif offset < 5000:
        return "Block Flags (1k-5k)"
    elif offset < 10000:
        return "Extended Block Flags (5k-10k)"
    elif offset < 50000:
        return "Equipment/Misc Flags (10k-50k)"
    elif offset < 110000:
        return "Inventory/GaItems (~50k-110k)"
    elif offset < 500000:
        return "Mid-range Data (110k-500k)"
    elif offset < 1000000:
        return "World Pickup Flags (500k-1M)"
    elif offset < 2000000:
        return "Extended World Flags (1M-2M)"
    else:
        return "Tail/Player Data (>2M)"


def format_inventory_delta(delta):
    """Format inventory changes for display."""
    if not delta:
        return ""
    lines = []
    added = delta.get("added", [])
    removed = delta.get("removed", [])
    if added:
        items = [f"{i['itemId']}({i['category']})" for i in added]
        lines.append(f"  +ADDED: {', '.join(items)}")
    if removed:
        items = [f"{i['itemId']}({i['category']})" for i in removed]
        lines.append(f"  -REMOVED: {', '.join(items)}")
    return "\n".join(lines)


def main():
    # Load metadata
    print("=" * 100)
    print("ELDEN RING SAVE FILE TIMELINE NARRATIVE")
    print("=" * 100)
    print()

    with open(JSONL_PATH) as f:
        entries = {
            e["id"]: e for e in (json.loads(line) for line in f)
        }

    gt_lookup = load_ground_truth_flags()
    print(f"Loaded {len(gt_lookup)} ground truth flag mappings")
    print(f"Total snapshots: {len(entries)}")
    print()

    # Determine which snapshots to analyze
    # Small: < 10000 bytes
    # Also include the specific high-value ones from the task
    high_value_ids = {
        "sd_000011", "sd_000018", "sd_000068", "sd_000082", "sd_000083",
        "sd_000084", "sd_000103", "sd_000121", "sd_000135", "sd_000136",
    }
    # Medium: 10000-100000
    medium_ids = set()
    small_ids = set()
    for eid, e in entries.items():
        bc = e["bytesChanged"]
        if bc < 10000:
            small_ids.add(eid)
        elif bc < 100000:
            medium_ids.add(eid)

    # Add a few medium ones
    analyze_ids = small_ids | high_value_ids | medium_ids
    # Sort by snapshot number
    analyze_list = sorted(
        analyze_ids,
        key=lambda x: int(x.split("_")[1])
    )

    print(f"Analyzing {len(analyze_list)} snapshots "
          f"({len(small_ids)} small, {len(medium_ids)} medium, "
          f"{len(high_value_ids)} high-value)")
    print()

    # -----------------------------------------------------------------------
    # Phase 1: Full timeline narrative
    # -----------------------------------------------------------------------
    print("=" * 100)
    print("PHASE 1: GAMEPLAY TIMELINE NARRATIVE")
    print("=" * 100)
    print()

    all_flag_changes = []  # Collect for summary

    for snap_id in analyze_list:
        entry = entries.get(snap_id)
        if not entry:
            continue

        diff_path = DIFF_DIR / entry["diffFile"]
        if not diff_path.exists():
            print(f"[{snap_id}] MISSING diff file: {diff_path}")
            continue

        records = parse_diff_file(diff_path)
        is_high_value = snap_id in high_value_ids

        # Analyze all changes
        single_bit_sets = []
        single_bit_clears = []
        multi_bit = []
        region_counts = defaultdict(int)

        for offset, old_b, new_b in records:
            analysis = analyze_change(offset, old_b, new_b, gt_lookup)
            region = categorize_offset_region(offset)
            region_counts[region] += 1

            if analysis["is_single_bit"]:
                if analysis.get("bit_set"):
                    single_bit_sets.append(analysis)
                else:
                    single_bit_clears.append(analysis)
            else:
                multi_bit.append(analysis)

        # Format timestamp
        ts = entry.get("timestamp", "")
        try:
            dt = datetime.fromisoformat(ts.replace("Z", "+00:00"))
            ts_display = dt.strftime("%H:%M:%S")
        except:
            ts_display = ts

        # Location
        pos = entry.get("playerPosition")
        if pos:
            map_id = pos.get("mapId")
            location = decode_map_id(map_id)
            coords = f"({pos['x']:.0f}, {pos['y']:.0f}, {pos['z']:.0f})"
        else:
            location = "Unknown (no position data)"
            coords = ""

        # Inventory
        inv_delta = entry.get("inventoryDelta")
        inv_text = format_inventory_delta(inv_delta) if inv_delta else ""

        # Print narrative
        marker = " *** HIGH VALUE ***" if is_high_value else ""
        print(f"{'=' * 80}")
        print(f"[{snap_id}] {ts_display}  |  {entry['bytesChanged']} bytes  "
              f"|  {entry.get('characterName', '?')}{marker}")
        print(f"  Location: {location} {coords}")
        if inv_text:
            print(f"  Inventory:")
            print(inv_text)
        print()

        # Region summary
        print(f"  Change distribution ({len(records)} total records):")
        for region, count in sorted(region_counts.items(),
                                    key=lambda x: x[1], reverse=True):
            print(f"    {region}: {count}")
        print()

        # Flag SET changes (most interesting)
        if single_bit_sets:
            print(f"  FLAG SETs ({len(single_bit_sets)} single-bit 0->1):")
            for analysis in sorted(single_bit_sets, key=lambda a: a["offset"]):
                off = analysis["offset"]
                bp = analysis.get("bit_position", "?")
                region = categorize_offset_region(off)

                if analysis["flag_matches"]:
                    for match in analysis["flag_matches"]:
                        fid = match["flag_id"]
                        bn = match["block_name"]
                        cat = match["category"]
                        gtn = match.get("gt_name") or ""
                        status = match.get("block_status", "")
                        name_str = f' "{gtn}"' if gtn else ""
                        print(f"    offset={off:>8d} bit={bp}  ->  "
                              f"flag={fid} [{cat}]{name_str} "
                              f"(via {bn}, {status})")
                        all_flag_changes.append({
                            "snap_id": snap_id,
                            "timestamp": ts_display,
                            "location": location,
                            "flag_id": fid,
                            "category": cat,
                            "name": gtn,
                            "action": "SET",
                            "offset": off,
                            "bit": bp,
                        })
                else:
                    print(f"    offset={off:>8d} bit={bp}  ->  "
                          f"[{region}] (no block match)")
                    all_flag_changes.append({
                        "snap_id": snap_id,
                        "timestamp": ts_display,
                        "location": location,
                        "flag_id": None,
                        "category": region,
                        "name": "",
                        "action": "SET",
                        "offset": off,
                        "bit": bp,
                    })
            print()

        # Flag CLEAR changes
        if single_bit_clears:
            print(f"  FLAG CLEARs ({len(single_bit_clears)} single-bit 1->0):")
            for analysis in sorted(single_bit_clears,
                                   key=lambda a: a["offset"])[:20]:
                off = analysis["offset"]
                bp = analysis.get("bit_position", "?")
                if analysis["flag_matches"]:
                    for match in analysis["flag_matches"]:
                        fid = match["flag_id"]
                        cat = match["category"]
                        gtn = match.get("gt_name") or ""
                        name_str = f' "{gtn}"' if gtn else ""
                        print(f"    offset={off:>8d} bit={bp}  ->  "
                              f"flag={fid} [{cat}]{name_str}")
                else:
                    region = categorize_offset_region(off)
                    print(f"    offset={off:>8d} bit={bp}  ->  "
                          f"[{region}]")
            if len(single_bit_clears) > 20:
                print(f"    ... and {len(single_bit_clears) - 20} more CLEARs")
            print()

        # Multi-bit changes summary
        if multi_bit:
            # Group by region for summary
            mb_regions = defaultdict(int)
            for a in multi_bit:
                mb_regions[categorize_offset_region(a["offset"])] += 1
            print(f"  Multi-bit changes ({len(multi_bit)} records):")
            for region, count in sorted(mb_regions.items(),
                                        key=lambda x: x[1], reverse=True):
                print(f"    {region}: {count}")
            print()

        # Narrative interpretation
        print(f"  INTERPRETATION:")
        if not single_bit_sets and not single_bit_clears:
            print(f"    Pure data update (counters, positions, etc.)")
        else:
            # Categorize the flag SETs
            flag_categories = defaultdict(list)
            for a in single_bit_sets:
                for m in a.get("flag_matches", []):
                    flag_categories[m["category"]].append(m)
                if not a.get("flag_matches"):
                    region = categorize_offset_region(a["offset"])
                    flag_categories[region].append({"offset": a["offset"]})

            if flag_categories:
                for cat, matches in sorted(flag_categories.items()):
                    if len(matches) == 1 and "flag_id" in matches[0]:
                        m = matches[0]
                        name = m.get("gt_name") or f"flag {m['flag_id']}"
                        print(f"    - {cat}: {name}")
                    else:
                        flag_ids = [str(m.get("flag_id", f"@{m.get('offset', '?')}"))
                                    for m in matches]
                        print(f"    - {cat}: {len(matches)} flags "
                              f"({', '.join(flag_ids[:8])}{'...' if len(flag_ids) > 8 else ''})")
            else:
                print(f"    {len(single_bit_sets)} flags SET, "
                      f"{len(single_bit_clears)} flags CLEAR")
        print()

    # -----------------------------------------------------------------------
    # Phase 2: Summary of all identified flags
    # -----------------------------------------------------------------------
    print()
    print("=" * 100)
    print("PHASE 2: ALL IDENTIFIED EVENT FLAGS ACROSS TIMELINE")
    print("=" * 100)
    print()

    # Group by category
    by_category = defaultdict(list)
    for fc in all_flag_changes:
        by_category[fc["category"]].append(fc)

    for cat in sorted(by_category.keys()):
        changes = by_category[cat]
        print(f"\n--- {cat} ({len(changes)} changes) ---")
        for c in sorted(changes, key=lambda x: (x["snap_id"], x.get("flag_id") or 0)):
            fid = c["flag_id"] or f"@offset={c['offset']}"
            name = c["name"]
            name_str = f' "{name}"' if name else ""
            print(f"  [{c['snap_id']}] {c['timestamp']}  "
                  f"{c['action']} flag={fid}{name_str}  "
                  f"at {c['location']}")

    # -----------------------------------------------------------------------
    # Phase 3: Detailed analysis of high-value snapshots
    # -----------------------------------------------------------------------
    print()
    print("=" * 100)
    print("PHASE 3: DETAILED HIGH-VALUE SNAPSHOT ANALYSIS")
    print("=" * 100)
    print()

    for snap_id in sorted(high_value_ids, key=lambda x: int(x.split("_")[1])):
        entry = entries.get(snap_id)
        if not entry:
            continue

        diff_path = DIFF_DIR / entry["diffFile"]
        if not diff_path.exists():
            continue

        records = parse_diff_file(diff_path)

        print(f"\n{'#' * 80}")
        print(f"# DETAILED: {snap_id} ({entry['bytesChanged']} bytes changed)")
        pos = entry.get("playerPosition")
        if pos:
            print(f"# Location: {decode_map_id(pos.get('mapId'))}")
        print(f"{'#' * 80}")
        print()

        # List ALL single-bit changes with full detail
        single_bit_changes = []
        for offset, old_b, new_b in records:
            xor = old_b ^ new_b
            if bin(xor).count("1") == 1:
                for b in range(8):
                    if xor & (1 << b):
                        bit_pos = 7 - b
                        break
                is_set = bool(new_b & xor)
                single_bit_changes.append((offset, old_b, new_b, bit_pos, is_set))

        print(f"Total records: {len(records)}")
        print(f"Single-bit changes: {len(single_bit_changes)}")
        print()

        if single_bit_changes:
            print(f"{'Offset':>10s}  {'Bit':>3s}  {'Dir':>5s}  "
                  f"{'Old':>4s}  {'New':>4s}  {'Region':30s}  {'Flag Match'}")
            print("-" * 120)

            for offset, old_b, new_b, bit_pos, is_set in sorted(
                    single_bit_changes, key=lambda x: x[0]):
                direction = "SET" if is_set else "CLR"
                region = categorize_offset_region(offset)

                # Try all block base matches
                matches = []
                for block_start, block_info in BLOCK_BASES.items():
                    base = block_info["base"]
                    fid = offset_to_flag_id(offset, bit_pos, block_start, base)
                    if fid is not None and block_start <= fid < block_start + 1000:
                        gt = gt_lookup.get((offset, bit_pos))
                        gtn = f' "{gt["name"]}"' if gt else ""
                        status_tag = f"[{block_info['status'][:3]}]"
                        matches.append(
                            f"flag={fid} ({block_info['name']}) "
                            f"{status_tag}{gtn}"
                        )

                # Try world pickup
                if offset > 100000:
                    row_id = WORLD_PICKUP_ROW_ID_BASE + offset * 8 + (7 - bit_pos)
                    if 1000000000 <= row_id <= 2999999999:
                        row_str = str(row_id)
                        xx = int(row_str[1:3])
                        yy = int(row_str[3:5])
                        local_id = int(row_str[5:])
                        if 30 <= xx <= 60 and 30 <= yy <= 60 and local_id < 10000:
                            rgn = tile_region_from_coords(xx, yy)
                            matches.append(
                                f"pickup row_id={row_id} "
                                f"tile=[{xx},{yy}] local={local_id} ({rgn})"
                            )

                # Ground truth direct
                gt = gt_lookup.get((offset, bit_pos))
                if gt and not matches:
                    matches.append(
                        f'flag={gt["flag_id"]} "{gt["name"]}" '
                        f'({gt["category"]}) [{gt["status"]}]'
                    )

                match_str = " | ".join(matches) if matches else "-"
                print(f"{offset:>10d}  {bit_pos:>3d}  {direction:>5s}  "
                      f"0x{old_b:02x}  0x{new_b:02x}  {region:30s}  {match_str}")

        # Also show offset range analysis
        print()
        offsets = [r[0] for r in records]
        if offsets:
            print(f"Offset range: {min(offsets)} - {max(offsets)}")
            # Cluster analysis
            clusters = []
            sorted_offsets = sorted(offsets)
            cluster_start = sorted_offsets[0]
            cluster_end = sorted_offsets[0]
            for o in sorted_offsets[1:]:
                if o - cluster_end > 100:
                    clusters.append((cluster_start, cluster_end,
                                     cluster_end - cluster_start + 1))
                    cluster_start = o
                cluster_end = o
            clusters.append((cluster_start, cluster_end,
                             cluster_end - cluster_start + 1))
            print(f"Offset clusters (gap > 100 bytes):")
            for start, end, span in clusters:
                count = sum(1 for o in sorted_offsets if start <= o <= end)
                region = categorize_offset_region(start)
                print(f"  {start:>10d} - {end:>10d}  "
                      f"(span={span:>6d}, {count:>4d} records)  [{region}]")
        print()

    # -----------------------------------------------------------------------
    # Phase 4: Cross-snapshot flag correlation
    # -----------------------------------------------------------------------
    print()
    print("=" * 100)
    print("PHASE 4: CROSS-SNAPSHOT FLAG CORRELATION")
    print("=" * 100)
    print()

    # Find offsets that appear in multiple snapshots with single-bit changes
    offset_snaps = defaultdict(list)  # (offset, bit) -> list of snap_ids
    for snap_id in analyze_list:
        entry = entries.get(snap_id)
        if not entry:
            continue
        diff_path = DIFF_DIR / entry["diffFile"]
        if not diff_path.exists():
            continue
        records = parse_diff_file(diff_path)
        for offset, old_b, new_b in records:
            xor = old_b ^ new_b
            if bin(xor).count("1") == 1:
                for b in range(8):
                    if xor & (1 << b):
                        bit_pos = 7 - b
                        break
                is_set = bool(new_b & xor)
                offset_snaps[(offset, bit_pos)].append(
                    (snap_id, "SET" if is_set else "CLR")
                )

    # Find flags that toggle in multiple snapshots
    multi_toggle = {k: v for k, v in offset_snaps.items() if len(v) > 1}
    if multi_toggle:
        print(f"Flags toggling in multiple snapshots: {len(multi_toggle)}")
        print()
        for (offset, bit), snaps in sorted(multi_toggle.items()):
            # Check if it's a known flag
            gt = gt_lookup.get((offset, bit))
            matches = []
            for block_start, block_info in BLOCK_BASES.items():
                base = block_info["base"]
                fid = offset_to_flag_id(offset, bit, block_start, base)
                if fid is not None and block_start <= fid < block_start + 1000:
                    matches.append(f"flag={fid} ({block_info['name']})")

            match_str = ", ".join(matches) if matches else ""
            gt_str = f' "{gt["name"]}"' if gt else ""
            snap_str = ", ".join(f"{s[0]}({s[1]})" for s in snaps)
            print(f"  offset={offset:>8d} bit={bit}: "
                  f"{match_str}{gt_str}  appears in: {snap_str}")
    else:
        print("No flags toggle in multiple snapshots in this selection.")

    print()
    print("=" * 100)
    print("ANALYSIS COMPLETE")
    print("=" * 100)


if __name__ == "__main__":
    main()
