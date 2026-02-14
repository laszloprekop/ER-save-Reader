#!/usr/bin/env python3
"""
Extract grace discoveries, world pickup flags, boss defeats, and other event flag
changes from the granular save file timeline.

Reads diff files (6-byte records: 4-byte LE offset + 1 byte old + 1 byte new)
and correlates changes with known event flag formulas from ground_truth_offsets.json.

Strategy: Since the timeline contains many large diffs (~200-400K bytes) from
autosave cycling, we use two approaches:
1. For small diffs (<20K bytes): These contain isolated meaningful changes.
   Scan all bit changes.
2. For all diffs: Track cumulative flag state to identify first-time SET events.
   A flag is "newly discovered" when it transitions from never-seen-SET to SET.

Uses numpy for vectorized processing of ~47M records across 140 diff files.
"""

import json
import os
import sys
import numpy as np
import xml.etree.ElementTree as ET
from collections import defaultdict
from datetime import datetime

# ── Paths ──────────────────────────────────────────────────────────────────────
BASE = "/Users/laszloprekop/dev/Elden Ring stuff"
JSONL_PATH = f"{BASE}/Elden Ring save files/Granular snapshots for debugging/timeline/slot_changes.jsonl"
DIFFS_DIR = f"{BASE}/Elden Ring save files/Granular snapshots for debugging/timeline/slot_diffs"
GROUND_TRUTH = f"{BASE}/ER-save-Editor/ground_truth_offsets.json"
WORLD_MAP_PARAM = f"{BASE}/Elden Ring decompiled game files/regulation-bin/WorldMapPointParam.param.xml"
ITEM_LOT_PARAM = f"{BASE}/Elden Ring decompiled game files/regulation-bin/ItemLotParam_map.param.xml"

# Only process diffs below this size for "isolated change" extraction
SMALL_DIFF_THRESHOLD = 20000


# ── Load functions ─────────────────────────────────────────────────────────────
def load_ground_truth():
    with open(GROUND_TRUTH) as f:
        return json.load(f)


def load_timeline():
    entries = []
    with open(JSONL_PATH) as f:
        for line in f:
            line = line.strip()
            if line:
                entries.append(json.loads(line))
    return entries


def load_world_map_points():
    lookup = {}
    try:
        tree = ET.parse(WORLD_MAP_PARAM)
        for row in tree.getroot().findall(".//row"):
            efid = row.get("eventFlagId")
            name = row.get("paramdexName", "")
            if efid:
                if name.startswith("Guidance of Grace: "):
                    name = name[len("Guidance of Grace: "):]
                lookup[int(efid)] = name
    except Exception as e:
        print(f"Warning: WorldMapPointParam: {e}", file=sys.stderr)
    return lookup


def load_item_lot_param_map(row_ids_to_find):
    lookup = {}
    if not row_ids_to_find:
        return lookup
    try:
        tree = ET.parse(ITEM_LOT_PARAM)
        ids_set = set(str(r) for r in row_ids_to_find)
        for row in tree.getroot().findall(".//row"):
            rid = row.get("id")
            if rid in ids_set:
                lookup[int(rid)] = row.get("paramdexName", f"ItemLot_{rid}")
    except Exception as e:
        print(f"Warning: ItemLotParam_map: {e}", file=sys.stderr)
    return lookup


# ── Parse diff file with numpy ────────────────────────────────────────────────
def parse_diff_numpy(filepath):
    data = np.fromfile(filepath, dtype=np.uint8)
    if len(data) < 6:
        return np.array([], dtype=np.uint32), np.array([], dtype=np.uint8), np.array([], dtype=np.uint8)
    n = len(data) // 6
    data = data[:n * 6].reshape(n, 6)
    offsets = (data[:, 0].astype(np.uint32) |
               (data[:, 1].astype(np.uint32) << 8) |
               (data[:, 2].astype(np.uint32) << 16) |
               (data[:, 3].astype(np.uint32) << 24))
    return offsets, data[:, 4], data[:, 5]


# ── Build offset ranges of interest ──────────────────────────────────────────
def build_interest_ranges(gt):
    ranges = []
    formulas = gt.get("formulas", {})

    for info in formulas.get("block_bases", {}).values():
        base = info.get("base_offset")
        if base is None:
            continue
        ranges.append((base, base + info.get("block_size", 1000) // 8 + 2))

    for info in formulas.get("midrange_formula", {}).values():
        base = info.get("base_offset")
        if base is None:
            continue
        ranges.append((base, base + info.get("block_size", 1000) // 8 + 2))

    for key, info in formulas.get("dungeon_formula", {}).items():
        if key in ("description", "formula"):
            continue
        base = info.get("base_offset", 0)
        if base == 0 and info.get("status") == "unverified":
            continue
        ss = info.get("section_size", 1125)
        ranges.append((base, base + 50 * ss + 130))

    for key, info in formulas.get("dungeon_pickup_bases", {}).items():
        if key in ("description", "formula"):
            continue
        base = info.get("base_offset", 0)
        ss = info.get("section_size", 1125)
        ranges.append((base, base + 50 * ss + 130))

    tf = formulas.get("tile_formula", {})
    tile_base = tf.get("base_offset", 485330)
    ranges.append((tile_base, tile_base + 40 * 40 * tf.get("bytes_per_slot", 875)))

    ranges.append((700000, 1100000))

    ranges.sort()
    merged = []
    for lo, hi in ranges:
        if merged and lo <= merged[-1][1]:
            merged[-1] = (merged[-1][0], max(merged[-1][1], hi))
        else:
            merged.append((lo, hi))
    return merged


def filter_to_interest(offsets, old_vals, new_vals, interest_ranges):
    if len(offsets) == 0:
        return offsets, old_vals, new_vals
    mask = np.zeros(len(offsets), dtype=bool)
    for lo, hi in interest_ranges:
        mask |= (offsets >= lo) & (offsets <= hi)
    mask &= (old_vals != new_vals)
    return offsets[mask], old_vals[mask], new_vals[mask]


# ── Bit expansion ────────────────────────────────────────────────────────────
def expand_bit_changes(offsets, old_vals, new_vals):
    xor = old_vals ^ new_vals
    all_offsets, all_bits, all_dirs, all_old, all_new = [], [], [], [], []
    for bit in range(8):
        bit_mask = np.uint8(1 << bit)
        changed = (xor & bit_mask) != 0
        if not np.any(changed):
            continue
        idx = np.where(changed)[0]
        all_offsets.append(offsets[idx])
        all_bits.append(np.full(len(idx), bit, dtype=np.uint8))
        all_dirs.append(((new_vals[idx] & bit_mask) != 0).astype(np.uint8))
        all_old.append(old_vals[idx])
        all_new.append(new_vals[idx])
    if not all_offsets:
        e = np.array([], dtype=np.uint32)
        eb = np.array([], dtype=np.uint8)
        return e, eb, eb, eb, eb
    return (np.concatenate(all_offsets), np.concatenate(all_bits),
            np.concatenate(all_dirs), np.concatenate(all_old), np.concatenate(all_new))


# ── Build lookup structures ───────────────────────────────────────────────────
def build_offset_bit_lookup(gt):
    lookup = {}
    for flag in gt.get("all_flags", []):
        o, b = flag.get("offset"), flag.get("bit")
        if o is not None and b is not None:
            lookup[(o, b)] = flag
    return lookup


def build_flag_id_lookup(gt):
    return {f["flag_id"]: f for f in gt.get("all_flags", []) if f.get("flag_id") is not None}


def build_block_ranges(gt):
    ranges = []
    for info in gt.get("formulas", {}).get("block_bases", {}).values():
        base = info.get("base_offset")
        if base is None:
            continue
        ranges.append({
            "offset_min": base, "offset_max": base + info.get("block_size", 1000) // 8 + 1,
            "block_start": info["block_start"], "base_offset": base,
            "block_size": info.get("block_size", 1000), "status": info.get("status", "unknown"),
        })
    return ranges


def build_midrange_ranges(gt):
    ranges = []
    for info in gt.get("formulas", {}).get("midrange_formula", {}).values():
        base = info.get("base_offset")
        if base is None:
            continue
        ranges.append({
            "offset_min": base, "offset_max": base + info.get("block_size", 1000) // 8 + 1,
            "block_start": info["block_start"], "base_offset": base,
            "block_size": info.get("block_size", 1000), "status": info.get("status", "unknown"),
        })
    return ranges


def build_dungeon_ranges(gt):
    formulas = gt.get("formulas", {})
    ranges = []
    for key, info in formulas.get("dungeon_formula", {}).items():
        if key in ("description", "formula"):
            continue
        base = info.get("base_offset", 0)
        if base == 0 and info.get("status") == "unverified":
            continue
        ss = info.get("section_size", 1125)
        ma = info.get("map_area", int(key))
        ranges.append({"type": "event", "map_area": ma, "base_offset": base,
                        "section_size": ss, "status": info.get("status", "unknown"),
                        "offset_min": base, "offset_max": base + 50 * ss + 130})
    for key, info in formulas.get("dungeon_pickup_bases", {}).items():
        if key in ("description", "formula"):
            continue
        base = info.get("base_offset", 0)
        ss = info.get("section_size", 1125)
        ma = info.get("map_area", int(key))
        ranges.append({"type": "pickup", "map_area": ma, "base_offset": base,
                        "section_size": ss, "status": info.get("status", "unknown"),
                        "offset_min": base, "offset_max": base + 50 * ss + 130})
    return ranges


def build_tile_params(gt):
    tf = gt.get("formulas", {}).get("tile_formula", {})
    return {
        "base_offset": tf.get("base_offset", 485330),
        "bytes_per_slot": tf.get("bytes_per_slot", 875),
        "slots_per_row": tf.get("slots_per_row", 40),
        "row_base": tf.get("row_base", 33),
        "col_base": tf.get("col_base", 30),
        "max_local_id": tf.get("max_local_id", 6999),
    }


# ── Region mapping ───────────────────────────────────────────────────────────
def tile_to_region(x, y):
    if 42 <= x <= 44 and 36 <= y <= 40: return "Limgrave"
    if 40 <= x <= 43 and 33 <= y <= 35: return "Weeping Peninsula"
    if 37 <= x <= 44 and 41 <= y <= 47: return "Liurnia"
    if 44 <= x <= 48 and 44 <= y <= 48: return "Altus Plateau"
    if 46 <= x <= 50 and 48 <= y <= 55: return "Mt. Gelmir / Volcano Manor"
    if 45 <= x <= 50 and 36 <= y <= 43: return "Caelid"
    if 44 <= x <= 52 and 49 <= y <= 57: return "Mountaintops of the Giants"
    if 40 <= x <= 45 and 48 <= y <= 52: return "Leyndell / Capital Outskirts"
    if 35 <= x <= 39 and 36 <= y <= 40: return "Stormhill"
    if 48 <= x <= 54 and 36 <= y <= 42: return "Dragonbarrow"
    if 20 <= x <= 30 and 40 <= y <= 55: return "Shadow of the Erdtree DLC"
    return f"Tile({x},{y})"


MAP_AREA_NAMES = {
    10: "Stormveil Castle", 11: "Leyndell Royal Capital",
    12: "Underground", 13: "Crumbling Farum Azula",
    14: "Tutorial / Shunning-Grounds", 15: "Haligtree / Elphael",
    16: "Volcano Manor", 18: "Roundtable Hold",
    19: "Chapel of Anticipation", 20: "Stranded Graveyard / DLC",
    21: "Haligtree (alt)", 22: "Castle Sol", 28: "Area 28",
    30: "Catacombs", 31: "Caves", 32: "Tunnels", 34: "Divine Towers",
    35: "Mohgwyn Palace", 39: "Elden Throne / Deeproot",
    40: "Hero's Graves", 41: "Minor Dungeons", 42: "Crystal Caves", 43: "Evergaols",
}

BLOCK_CATEGORIES = {
    60000: "Progression Flags", 61000: "Map Area Visit Flags",
    62000: "World Map Point Flags", 65000: "Crystal Tears",
    67000: "Cookbooks/Items", 68000: "Cookbooks/Items",
    71000: "Stormveil Graces", 71100: "Leyndell Graces",
    71600: "Volcano Manor Graces", 71800: "Tutorial Graces",
    72000: "DLC Graces (Enir-Ilim)", 73000: "Dungeon Graces",
    74000: "DLC Dungeon Graces", 75000: "Extended Graces (disproven)",
    76000: "Overworld Graces", 77000: "Extended World Graces (disproven)",
    78000: "Grace Guidance / Stakes of Marika",
    520000: "Spirit Ashes/Talismans",
}

MIDRANGE_CATEGORIES = {
    510000: "Remembrance Consumption", 520000: "Spirit Ashes/Talismans",
    540000: "Sorcery/Incantation/AoW Unlock", 710000: "Roundtable NPC Progression",
}

WORLD_PICKUP_ROW_ID_BASE = 1037373320


# ── Classify a single bit change ────────────────────────────────────────────
def classify_change(offset, bit, direction_int,
                    ob_lookup, flag_lookup, block_ranges, midrange_ranges,
                    dungeon_ranges, tile_params, wmp_lookup):
    d = "SET" if direction_int == 1 else "CLEARED"

    # 1. Ground truth direct
    gt_flag = ob_lookup.get((offset, bit))
    if gt_flag:
        return {
            "flag_id": gt_flag["flag_id"], "name": gt_flag.get("name", ""),
            "category": gt_flag.get("category", "Unknown"),
            "region": gt_flag.get("region", ""), "source": "ground_truth",
            "confidence": gt_flag.get("confidence", 0.9), "direction": d,
        }

    # 2. Block formula
    for br in block_ranges:
        if br["offset_min"] <= offset <= br["offset_max"]:
            flag_id = br["block_start"] + (offset - br["base_offset"]) * 8 + (7 - bit)
            if br["block_start"] <= flag_id < br["block_start"] + br["block_size"]:
                name, region = "", ""
                category = BLOCK_CATEGORIES.get(br["block_start"], f"Block {br['block_start']}")
                fl = flag_lookup.get(flag_id)
                if fl:
                    name = fl.get("name", "")
                    region = fl.get("region", "")
                    category = fl.get("category", category)
                if not name:
                    wn = wmp_lookup.get(flag_id)
                    if wn:
                        name = wn
                if 71000 <= flag_id < 80000:
                    bk = flag_id // 1000
                    category = {71: "Dungeon Grace", 72: "DLC Grace", 73: "Dungeon Grace",
                                74: "DLC Dungeon Grace", 76: "Overworld Grace",
                                78: "Grace Guidance / Stake of Marika"}.get(bk, category)
                return {
                    "flag_id": flag_id, "name": name, "category": category,
                    "region": region, "source": f"block({br['block_start']})",
                    "confidence": 0.7 if br["status"] == "verified" else 0.4, "direction": d,
                }

    # 3. Midrange
    for mr in midrange_ranges:
        if mr["offset_min"] <= offset <= mr["offset_max"]:
            flag_id = mr["block_start"] + (offset - mr["base_offset"]) * 8 + (7 - bit)
            if mr["block_start"] <= flag_id < mr["block_start"] + mr["block_size"]:
                name = ""
                fl = flag_lookup.get(flag_id)
                if fl:
                    name = fl.get("name", "")
                cat = MIDRANGE_CATEGORIES.get(mr["block_start"], f"Midrange {mr['block_start']}")
                return {"flag_id": flag_id, "name": name, "category": cat,
                        "region": "", "source": f"midrange({mr['block_start']})",
                        "confidence": 0.7 if mr["status"] == "verified" else 0.4, "direction": d}

    # 4. Dungeon formula
    for dr in dungeon_ranges:
        if dr["offset_min"] <= offset <= dr["offset_max"]:
            rel = offset - dr["base_offset"]
            section = rel // dr["section_size"]
            local_byte = rel % dr["section_size"]
            local_id = local_byte * 8 + (7 - bit)
            valid = (dr["type"] == "event" and 0 <= local_id < 1000) or \
                    (dr["type"] == "pickup" and 7000 <= local_id < 8000)
            if valid:
                flag_id = dr["map_area"] * 1000000 + section * 10000 + local_id
                name, region = "", MAP_AREA_NAMES.get(dr["map_area"], f"Area {dr['map_area']}")
                fl = flag_lookup.get(flag_id)
                if fl:
                    name = fl.get("name", "")
                    region = fl.get("region", "") or region
                if 800 <= local_id <= 899: cat = "Boss Defeat"
                elif 900 <= local_id <= 999: cat = "Dungeon Grace/Event"
                elif local_id >= 7000: cat = "Dungeon Item Pickup"
                else: cat = "Dungeon Event"
                return {"flag_id": flag_id, "name": name, "category": cat, "region": region,
                        "source": f"dungeon(area={dr['map_area']},{dr['type']})",
                        "confidence": 0.6 if dr["status"] == "verified" else 0.3, "direction": d}

    # 5. Tile formula
    tp_base = tile_params["base_offset"]
    tp_total = tile_params["slots_per_row"] * 40 * tile_params["bytes_per_slot"]
    if tp_base <= offset < tp_base + tp_total:
        bps = tile_params["bytes_per_slot"]
        spr = tile_params["slots_per_row"]
        rel = offset - tp_base
        bpr = spr * bps
        row_idx, row_rem = divmod(rel, bpr)
        col_idx, local_byte = divmod(row_rem, bps)
        local_id = local_byte * 8 + (7 - bit)
        if local_id <= tile_params["max_local_id"]:
            ty = tile_params["row_base"] + row_idx
            tx = tile_params["col_base"] + col_idx
            if 0 <= tx <= 80 and 0 <= ty <= 80:
                row_id = int(f"1{tx:02d}{ty:02d}{local_id:04d}")
                return {"flag_id": row_id,
                        "name": f"World Pickup tile({tx},{ty}) lid={local_id}",
                        "category": "World Pickup (Tile)", "region": tile_to_region(tx, ty),
                        "source": "tile_formula", "confidence": 0.6, "direction": d,
                        "tile_x": tx, "tile_y": ty, "local_id": local_id}

    # 6. World pickup row_id bitfield
    if 700000 <= offset <= 1100000:
        row_id = WORLD_PICKUP_ROW_ID_BASE + offset * 8 + (7 - bit)
        s = str(row_id)
        if len(s) >= 9 and s[0] == '1':
            tx, ty, lid = int(s[1:3]), int(s[3:5]), int(s[5:])
            if 0 <= lid <= 9999 and 0 <= tx <= 80 and 0 <= ty <= 80:
                return {"flag_id": row_id,
                        "name": f"World Pickup tile({tx},{ty}) lid={lid}",
                        "category": "World Pickup (Row ID)", "region": tile_to_region(tx, ty),
                        "source": "row_id_bitfield", "confidence": 0.5, "direction": d,
                        "tile_x": tx, "tile_y": ty, "local_id": lid}
    return None


# ── Main extraction ──────────────────────────────────────────────────────────
def extract_all():
    print("Loading ground truth...")
    gt = load_ground_truth()
    ob_lookup = build_offset_bit_lookup(gt)
    flag_lookup = build_flag_id_lookup(gt)
    block_ranges = build_block_ranges(gt)
    midrange_ranges = build_midrange_ranges(gt)
    dungeon_ranges = build_dungeon_ranges(gt)
    tile_params = build_tile_params(gt)
    interest_ranges = build_interest_ranges(gt)

    print(f"  Interest ranges: {len(interest_ranges)} merged intervals")

    print("\nLoading WorldMapPointParam...")
    wmp_lookup = load_world_map_points()
    print(f"  {len(wmp_lookup)} entries")

    print("\nLoading timeline...")
    timeline = load_timeline()
    print(f"  {len(timeline)} snapshots")

    # ── PASS 1: Scan all diffs, track first-time SET per flag ──────────────
    print("\n--- PASS 1: Track flag state across all diffs ---")

    # flag_id -> {"first_set_snap": ..., "set_count": N, "cleared_count": N}
    flag_state = {}
    # For first-time detection: set of flag_ids that have been SET at least once so far
    ever_set = set()
    # Events for first-time SETs
    first_set_events = []
    # Events from small diffs (all SETs)
    small_diff_events = []

    total_records = 0
    total_filtered = 0

    for idx, entry in enumerate(timeline):
        snap_id = entry["id"]
        timestamp = entry["timestamp"]
        character = entry.get("characterName", "?")
        player_pos = entry.get("playerPosition")
        diff_file = entry.get("diffFile")
        bytes_changed = entry.get("bytesChanged", 0)
        is_small = bytes_changed < SMALL_DIFF_THRESHOLD

        if not diff_file:
            continue
        diff_path = os.path.join(DIFFS_DIR, diff_file)
        if not os.path.exists(diff_path):
            continue

        offsets, old_vals, new_vals = parse_diff_numpy(diff_path)
        total_records += len(offsets)
        offsets, old_vals, new_vals = filter_to_interest(offsets, old_vals, new_vals, interest_ranges)
        total_filtered += len(offsets)

        if len(offsets) == 0:
            continue

        exp_off, exp_bit, exp_dir, exp_old, exp_new = expand_bit_changes(offsets, old_vals, new_vals)

        for i in range(len(exp_off)):
            result = classify_change(
                int(exp_off[i]), int(exp_bit[i]), int(exp_dir[i]),
                ob_lookup, flag_lookup, block_ranges, midrange_ranges,
                dungeon_ranges, tile_params, wmp_lookup
            )
            if result is None:
                continue

            fid = result["flag_id"]
            result["snap_id"] = snap_id
            result["timestamp"] = timestamp
            result["character"] = character
            result["player_pos"] = player_pos
            result["offset"] = int(exp_off[i])
            result["bit"] = int(exp_bit[i])
            result["old"] = int(exp_old[i])
            result["new"] = int(exp_new[i])
            result["bytes_changed"] = bytes_changed

            if result["direction"] == "SET":
                if fid not in ever_set:
                    ever_set.add(fid)
                    result["first_time"] = True
                    first_set_events.append(result)

                if is_small:
                    result["small_diff"] = True
                    small_diff_events.append(result)

            elif result["direction"] == "CLEARED" and is_small:
                result["small_diff"] = True
                small_diff_events.append(result)

        if (idx + 1) % 20 == 0:
            print(f"  {idx + 1}/{len(timeline)}: {total_records:,} records, "
                  f"{total_filtered:,} filtered, {len(first_set_events):,} first-time SETs, "
                  f"{len(small_diff_events):,} small-diff events")

    print(f"\n  DONE: {total_records:,} records, {total_filtered:,} filtered")
    print(f"  First-time SET events: {len(first_set_events):,}")
    print(f"  Small-diff events: {len(small_diff_events):,}")
    print(f"  Unique flags ever SET: {len(ever_set):,}")

    # ── Enrich world pickups ──
    wp_ids = set()
    for ev in first_set_events + small_diff_events:
        if ev["category"].startswith("World Pickup"):
            wp_ids.add(ev["flag_id"])
    if wp_ids:
        print(f"\nLooking up {len(wp_ids)} world pickup IDs in ItemLotParam_map...")
        item_lookup = load_item_lot_param_map(wp_ids)
        enriched = 0
        for ev in first_set_events + small_diff_events:
            if ev["category"].startswith("World Pickup"):
                nm = item_lookup.get(ev["flag_id"])
                if nm:
                    ev["name"] = nm
                    enriched += 1
        print(f"  Enriched {enriched} events")

    return first_set_events, small_diff_events


# ── Output ───────────────────────────────────────────────────────────────────
def fmt_ts(ts):
    try:
        return datetime.fromisoformat(ts.replace("Z", "+00:00")).strftime("%H:%M:%S")
    except Exception:
        return ts


def fmt_pos(pos):
    if not pos:
        return "N/A"
    x, y, z = pos.get("x", 0), pos.get("y", 0), pos.get("z", 0)
    mid = pos.get("mapId", [])
    ms = ".".join(str(m) for m in mid) if mid else "?"
    return f"({x:.1f}, {y:.1f}, {z:.1f}) map={ms}"


def print_event(ev):
    ts = fmt_ts(ev["timestamp"])
    fid = str(ev["flag_id"])
    name = ev["name"] or "(unknown)"
    region = ev["region"] or ""
    pos = fmt_pos(ev["player_pos"])
    bc = ev.get("bytes_changed", "?")
    ft = " [FIRST]" if ev.get("first_time") else ""
    sd = " [small-diff]" if ev.get("small_diff") else ""

    print(f"  {ev['snap_id']} [{ts}] {ev['direction']:>7s}{ft}{sd}")
    print(f"    flag={fid:>12s}  {name}")
    print(f"    region={region}  offset=0x{ev['offset']:06x} bit={ev['bit']}  "
          f"byte:0x{ev['old']:02x}->0x{ev['new']:02x}  diffSize={bc}")
    print(f"    pos={pos}")
    if "tile_x" in ev:
        print(f"    tile=({ev['tile_x']},{ev['tile_y']}) local_id={ev['local_id']}")
    print()


def print_results(first_set_events, small_diff_events):
    # ════════════════════════════════════════════════════════════════════════
    # SECTION A: First-time flag SETs (across all diffs)
    # ════════════════════════════════════════════════════════════════════════
    print("\n" + "=" * 120)
    print("SECTION A: FIRST-TIME FLAG DISCOVERIES (across all 140 diffs)")
    print("  These are flags that were SET for the first time in the timeline.")
    print("  Large diffs may include flags from slot-switching; small-diff first-SETs are most reliable.")
    print("=" * 120)

    by_cat = defaultdict(list)
    for ev in first_set_events:
        by_cat[ev["category"]].append(ev)

    cat_order = [
        "Grace", "Tutorial Graces", "Overworld Grace", "Dungeon Grace",
        "DLC Grace", "DLC Dungeon Grace", "Grace Guidance / Stake of Marika",
        "Stormveil Graces", "Leyndell Graces", "Volcano Manor Graces",
        "Dungeon Graces", "DLC Graces (Enir-Ilim)",
        "Great Boss Defeat", "Field Boss Defeat", "Boss Defeat",
        "Dungeon Grace/Event", "Dungeon Event", "Dungeon Item Pickup",
        "Progression Flags", "Map Area Visit Flags", "World Map Point Flags",
        "Crystal Tears", "Cookbooks/Items",
        "Remembrance Consumption", "Spirit Ashes/Talismans",
        "Sorcery/Incantation/AoW Unlock", "Roundtable NPC Progression",
        "World Pickup (Tile)", "World Pickup (Row ID)",
    ]
    seen = set(cat_order)
    for c in sorted(by_cat.keys()):
        if c not in seen:
            cat_order.append(c)

    # Summary
    print(f"\nTotal first-time SET events: {len(first_set_events)}")
    print(f"Categories: {len(by_cat)}\n")
    print("CATEGORY SUMMARY:")
    print("-" * 80)
    for cat in cat_order:
        if cat in by_cat:
            evs = by_cat[cat]
            small = sum(1 for e in evs if e.get("bytes_changed", 999999) < SMALL_DIFF_THRESHOLD)
            print(f"  {cat}: {len(evs)} total ({small} from small diffs)")
    print()

    # Detailed: only grace-related, boss, and progression categories
    # (skip dungeon events and world pickups for first-set since they're noisy)
    priority_cats = {
        "Grace", "Tutorial Graces", "Overworld Grace", "Dungeon Grace",
        "DLC Grace", "DLC Dungeon Grace", "Grace Guidance / Stake of Marika",
        "Stormveil Graces", "Leyndell Graces", "Volcano Manor Graces",
        "Dungeon Graces", "DLC Graces (Enir-Ilim)",
        "Great Boss Defeat", "Field Boss Defeat", "Boss Defeat",
        "Progression Flags", "Map Area Visit Flags",
        "Crystal Tears", "Cookbooks/Items",
        "Remembrance Consumption", "Spirit Ashes/Talismans",
        "Sorcery/Incantation/AoW Unlock", "Roundtable NPC Progression",
    }

    for cat in cat_order:
        if cat not in by_cat or cat not in priority_cats:
            continue
        evs = sorted(by_cat[cat], key=lambda e: e["timestamp"])

        print("=" * 120)
        print(f"  {cat.upper()} - FIRST-TIME SETs ({len(evs)} events)")
        print("=" * 120)

        for ev in evs:
            print_event(ev)

    # World pickups: just summary
    for cat in ["World Pickup (Tile)", "World Pickup (Row ID)",
                 "Dungeon Item Pickup", "Dungeon Event", "Dungeon Grace/Event"]:
        if cat in by_cat:
            evs = sorted(by_cat[cat], key=lambda e: e["timestamp"])
            print("=" * 120)
            print(f"  {cat.upper()} - FIRST-TIME SETs ({len(evs)} events, showing first 50)")
            print("=" * 120)
            for ev in evs[:50]:
                print_event(ev)
            if len(evs) > 50:
                print(f"  ... and {len(evs) - 50} more\n")

    # ════════════════════════════════════════════════════════════════════════
    # SECTION B: Small-diff events (isolated meaningful changes)
    # ════════════════════════════════════════════════════════════════════════
    print("\n" + "=" * 120)
    print(f"SECTION B: SMALL-DIFF EVENTS (diffs < {SMALL_DIFF_THRESHOLD:,} bytes)")
    print("  These are the most reliable - isolated changes during active gameplay.")
    print("=" * 120)

    sd_by_cat = defaultdict(list)
    for ev in small_diff_events:
        sd_by_cat[ev["category"]].append(ev)

    print(f"\nTotal small-diff events: {len(small_diff_events)}")
    print(f"Categories: {len(sd_by_cat)}\n")
    print("CATEGORY SUMMARY:")
    print("-" * 80)
    for cat in cat_order:
        if cat in sd_by_cat:
            evs = sd_by_cat[cat]
            sets = sum(1 for e in evs if e["direction"] == "SET")
            clears = sum(1 for e in evs if e["direction"] == "CLEARED")
            print(f"  {cat}: {len(evs)} events ({sets} SET, {clears} CLEARED)")
    print()

    for cat in cat_order:
        if cat not in sd_by_cat:
            continue
        evs = sorted(sd_by_cat[cat], key=lambda e: e["timestamp"])

        print("=" * 120)
        print(f"  {cat.upper()} - SMALL-DIFF EVENTS ({len(evs)} events)")
        print("=" * 120)

        for ev in evs:
            print_event(ev)


def main():
    first_set, small_diff = extract_all()
    print_results(first_set, small_diff)


if __name__ == "__main__":
    main()
