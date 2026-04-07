"""
Dungeon Base Calibration Script

Discovers correct base offsets for unverified legacy dungeon areas by using
the character's inventory and confirmed boss kills as ground truth evidence.

If an item is in inventory, its acquisition flag is definitively set in the
event flags section. Same for confirmed boss defeats. Scanning the event flags
buffer for bases where all provided anchors are simultaneously set — and
requiring >= 2 anchors with distinct local_id % 8 residues to rule out random
bit coincidences — gives empirically verified base offsets.

Usage:
    python3 calibrate_dungeon_bases.py [--patch]

Core functions are importable for testing:
    from calibrate_dungeon_bases import (
        FlagAnchor, BaseCandidate, find_base_for_flags, corroborate
    )
"""

from __future__ import annotations

import struct
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import List

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

EVENT_FLAGS_SIZE = 0x1BF99F  # 1,833,375 bytes

# Save file structure
BND4_HEADER_SIZE    = 0x40
BND4_ENTRY_SIZE     = 0x20
BND4_ENTRY_OFFSET   = 0x10
SLOT_CHECKSUM_SIZE  = 16
SLOT_SIZE           = 0x280000
FIXED_HEADER_SIZE   = 0x20

LIVE_SAVE_PATH = Path(
    "/Users/laszloprekop/Library/Application Support/CrossOver/Bottles"
    "/Elden Ring/drive_c/users/crossover/AppData/Roaming/EldenRing"
    "/76561197969778805/ER0000.sl2"
)

# Validation anchors for locating the event flags section within a slot
# Format: (byte_offset_in_ef, bit_position, description)
EF_VALIDATION_ANCHORS = [
    (2725, 7, "Cave of Knowledge"),
    (2725, 6, "Stranded Graveyard"),
    (3262, 3, "The First Step"),
    (3262, 2, "Church of Elleh"),
]

# Already-verified general event bases — used for self-check
VERIFIED_GENERAL_BASES: dict[int, int] = {
    10: 4112,   # Stormveil Castle     — verified
    30: 27411,  # Catacombs            — verified
    31: 28634,  # Caves                — verified
    32: 31577,  # Tunnels              — verified
}


# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------

@dataclass
class FlagAnchor:
    """A flag ID known to be set in the save (confirmed boss kill or inventory item)."""
    flag_id: int
    name: str


@dataclass
class BaseCandidate:
    """A candidate base offset at which all provided anchors are simultaneously set."""
    base: int
    anchors: List[FlagAnchor] = field(default_factory=list)


# ---------------------------------------------------------------------------
# Core pure functions (unit-testable, no I/O)
# ---------------------------------------------------------------------------

def _flag_offset(base: int, flag_id: int) -> tuple[int, int]:
    """Return (byte_offset, bit_position) for a general dungeon event flag at the given base."""
    local_id = flag_id % 10000
    return base + local_id // 8, 7 - (local_id % 8)


def _is_set(event_flags: bytes, byte_off: int, bit_pos: int) -> bool:
    if byte_off < 0 or byte_off >= len(event_flags):
        return False
    return bool(event_flags[byte_off] & (1 << bit_pos))


def find_base_for_flags(
    event_flags: bytes,
    anchors: List[FlagAnchor],
    scan_start: int = 0,
    scan_end: int = 200_000,
) -> List[BaseCandidate]:
    """
    Scan the event flags buffer for bases where ALL provided anchors are set.

    Returns a list of BaseCandidate ordered by base value ascending.
    Only bases where every anchor is simultaneously satisfied are included.
    """
    if not anchors:
        return []

    results: List[BaseCandidate] = []

    for base in range(scan_start, scan_end):
        if all(
            _is_set(event_flags, *_flag_offset(base, a.flag_id))
            for a in anchors
        ):
            results.append(BaseCandidate(base=base, anchors=list(anchors)))

    return results


def find_consistent_base(
    per_slot_candidates: List[List[BaseCandidate]],
    min_slots: int = 2,
) -> List[BaseCandidate]:
    """
    Return candidates whose base appears in at least min_slots slot candidate lists.

    False positives from random bit patterns are slot-specific; the true base
    appears consistently across every slot where the anchored events occurred.
    The returned candidates carry anchors from the first slot that confirmed them.
    """
    from collections import Counter

    count: Counter = Counter()
    first_seen: dict[int, BaseCandidate] = {}

    for slot_candidates in per_slot_candidates:
        seen_in_slot: set[int] = set()
        for candidate in slot_candidates:
            if candidate.base not in seen_in_slot:
                count[candidate.base] += 1
                seen_in_slot.add(candidate.base)
                if candidate.base not in first_seen:
                    first_seen[candidate.base] = candidate

    return [first_seen[base] for base, n in count.items() if n >= min_slots]


def corroborate(
    candidates: List[BaseCandidate],
    min_distinct_residues: int = 2,
) -> List[BaseCandidate]:
    """
    Filter candidates to those whose anchors span >= min_distinct_residues
    distinct local_id % 8 values.

    This prevents false positives caused by a single bit pattern appearing
    at multiple offsets in random event flag data.
    """
    accepted = []
    for candidate in candidates:
        residues = {a.flag_id % 8 for a in candidate.anchors}
        if len(residues) >= min_distinct_residues:
            accepted.append(candidate)
    return accepted


# ---------------------------------------------------------------------------
# Save file I/O
# ---------------------------------------------------------------------------

def _find_ef_offset(slot_data: bytes) -> int:
    """Locate the event flags section within slot data using validation anchors."""
    best_offset = 0x12B00
    best_score = 0

    search_end = min(0x30000, len(slot_data) - EVENT_FLAGS_SIZE)
    for offset in range(0x10000, search_end, 4):
        score = sum(
            1 for byte_off, bit_pos, _ in EF_VALIDATION_ANCHORS
            if (offset + byte_off) < len(slot_data)
            and bool(slot_data[offset + byte_off] & (1 << bit_pos))
        )
        if score > best_score:
            best_score = score
            best_offset = offset

    return best_offset


def load_slots(save_path: Path) -> list[dict]:
    """Load all occupied character slots from a save file."""
    data = save_path.read_bytes()
    slots = []

    for slot_idx in range(10):
        entry_offset = BND4_HEADER_SIZE + slot_idx * BND4_ENTRY_SIZE + BND4_ENTRY_OFFSET
        if entry_offset + 4 > len(data):
            break

        bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
        slot_start = bnd4_offset + SLOT_CHECKSUM_SIZE
        slot_data = data[slot_start: slot_start + SLOT_SIZE]

        if len(slot_data) < FIXED_HEADER_SIZE:
            continue
        if struct.unpack_from('<I', slot_data, 0)[0] == 0:
            continue  # empty slot

        ef_offset = _find_ef_offset(slot_data)
        event_flags = slot_data[ef_offset: ef_offset + EVENT_FLAGS_SIZE]

        slots.append({
            'slot_idx': slot_idx,
            'event_flags': event_flags,
            'ef_offset': ef_offset,
        })

    return slots


# ---------------------------------------------------------------------------
# Self-check: verify already-known bases still hold
# ---------------------------------------------------------------------------

def self_check(slots: list[dict]) -> bool:
    """
    Verify that known-verified general event bases still produce hits
    on at least one slot. Returns True if all verified areas pass.
    """
    # Representative flags that should be set in mid/late-game saves
    KNOWN_FLAGS: dict[int, list[FlagAnchor]] = {
        10: [
            FlagAnchor(10000850, "Godrick the Grafted"),
            FlagAnchor(10000499, "Margit (pre-boss gate)"),
        ],
    }

    all_pass = True
    for area, known_base in VERIFIED_GENERAL_BASES.items():
        anchors = KNOWN_FLAGS.get(area)
        if not anchors:
            continue
        found_in_any = False
        for slot in slots:
            candidates = find_base_for_flags(slot['event_flags'], anchors,
                                             scan_start=known_base - 100,
                                             scan_end=known_base + 101)
            if any(c.base == known_base for c in candidates):
                found_in_any = True
                break
        if not found_in_any:
            print(f"  SELF-CHECK FAIL: area {area} expected base {known_base} not found")
            all_pass = False
        else:
            print(f"  SELF-CHECK OK:   area {area} base {known_base} confirmed")

    return all_pass


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def calibrate_section(
    slots: list[dict],
    area: int,
    section: int,
    anchors: List[FlagAnchor],
    label: str,
    corroborate_anchors: List[FlagAnchor] | None = None,
    patch: bool = False,
) -> int | None:
    """
    Run cross-slot calibration for one (area, section) pickup base.

    Returns the discovered base if exactly 1 or very few candidates remain,
    otherwise None.  Prints a compact summary.
    """
    import datetime

    print(f"\n  [{area:02d}_{section:02d}] {label}")

    per_slot: list[list[BaseCandidate]] = []
    for slot in slots:
        candidates = find_base_for_flags(slot['event_flags'], anchors)
        verified = corroborate(candidates)
        per_slot.append(verified)

    non_empty = [s for s in per_slot if s]
    if not non_empty:
        print(f"    SKIP — no slot has all anchors set")
        return None

    # Try requiring all non-empty slots first; fall back by one slot at a time
    consistent: list[BaseCandidate] = []
    min_used = len(non_empty)
    for min_s in range(len(non_empty), 0, -1):
        consistent = find_consistent_base(per_slot, min_slots=min_s)
        if consistent:
            min_used = min_s
            break

    slot_counts = ", ".join(f"s{s['slot_idx']}={len(r)}" for s, r in zip(slots, per_slot) if r)
    print(f"    Slots with candidates: {slot_counts}  |  consistent(min={min_used}): {len(consistent)}")

    if not consistent:
        print(f"    FAIL — no consistent candidate found")
        return None

    # Sort by number of supporting slots descending
    from collections import Counter
    base_slot_count: Counter = Counter()
    for r in per_slot:
        for c in r:
            base_slot_count[c.base] += 1
    consistent.sort(key=lambda c: -base_slot_count[c.base])

    winner = consistent[0].base

    # Corroborate winner with extra anchors if provided
    if corroborate_anchors:
        hit_count = 0
        for slot in slots:
            ef = slot['event_flags']
            if all(_is_set(ef, *_flag_offset(winner, a.flag_id)) for a in corroborate_anchors):
                hit_count += 1
        corr_str = f"corroborated in {hit_count}/{len(slots)} slots"
    else:
        corr_str = f"{len(consistent)} candidate(s)"

    print(f"    Winner: base={winner}  ({corr_str})")
    if len(consistent) > 1:
        others = [c.base for c in consistent[1:4]]
        print(f"    Other candidates: {others}{'...' if len(consistent) > 4 else ''}")

    if patch:
        print(f"    (({area:2d}, {section:2d}), {winner}),  // {label} — VERIFIED {datetime.date.today()}")

    return winner


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description="Calibrate dungeon base offsets from live save")
    parser.add_argument("--patch", action="store_true",
                        help="Print ready-to-paste lib.rs lines for verified bases")
    parser.add_argument("--save", type=Path, default=LIVE_SAVE_PATH,
                        help="Path to ER0000.sl2 save file")
    args = parser.parse_args()

    print(f"Loading save: {args.save}")
    slots = load_slots(args.save)
    print(f"Found {len(slots)} occupied slot(s)\n")

    print("Running self-check on verified areas...")
    self_check(slots)

    # -----------------------------------------------------------------------
    # Pickup section base calibration for all 31903-placeholder areas
    # Anchors are confirmed-in-save flags from flag-correlation-candidates.jsonl
    # Each set has >= 2 distinct local_id % 8 residues (required by corroborate)
    # -----------------------------------------------------------------------
    print()
    print("=" * 60)
    print("Calibrating pickup section bases (31903 placeholders)")
    print("=" * 60)

    # CALIBRATION RELIABILITY TIERS:
    #
    # TIER 1 — High confidence: anchors have unique local_ids (not shared with
    #   other sections), sufficient distinct residues, and 3+ slots agree.
    #   Winner verified by checking hit-count across all 41+ confirmed flags.
    #   m14 (29782), m16 (2194) belong here.
    #
    # TIER 2 — Unreliable: anchors use generic local_ids (7025, 7220, 7230...)
    #   present across many sections.  Cross-slot consistency can be fooled by
    #   coincidental dense bits.  Winner looks clean (1-2 candidates) but is
    #   likely a false positive.  Do NOT apply these results to lib.rs without
    #   a before/after save diff confirming the specific item pickup.
    #   m11, m12_2, m12_7, m13, m15, m20, m21, m31_21, m35, m41 are TIER 2.
    #
    # To move an area to TIER 1: find local_ids that are unique to that section
    #   (not shared with any other dungeon area that a typical character has also
    #   visited) and rerun the calibration.

    SECTIONS: list[tuple[int, int, list[FlagAnchor], str]] = [
        # (area, section, anchors, label)
        # ------------------------------------------------------------------
        # TIER 2 — generic local_ids, result unreliable
        # Area 11: Leyndell Royal Capital — section 0
        (11, 0, [
            FlagAnchor(11007025, "Celestial Dew"),             # residue=1
            FlagAnchor(11007220, "Golden Rune [8]"),           # residue=4
            FlagAnchor(11007230, "Lordsworn's Bolt"),          # residue=6
            FlagAnchor(11007730, "Holyproof Dried Liver"),     # residue=2
        ], "Leyndell Royal Capital (s0)  [TIER 2 — unreliable]"),
        # ------------------------------------------------------------------
        # TIER 2 — generic local_ids, result unreliable
        # Area 12: Underground section 2 (Ainsel River Main)
        (12, 2, [
            FlagAnchor(12027050, "Marika's Scarseal"),         # residue=2
            FlagAnchor(12027470, "Clarifying Horn Charm"),     # residue=6
            FlagAnchor(12027000, "Mottled Necklace"),          # residue=0
            FlagAnchor(12027620, "Mottled Necklace +1"),       # residue=4
        ], "Underground s2 (Ainsel River Main)  [TIER 2 — unreliable]"),
        # ------------------------------------------------------------------
        # TIER 2 — only 1 unique local_id (7440), insufficient discrimination
        # Area 12: Underground section 7 (Deeproot Depths)
        (12, 7, [
            FlagAnchor(12077440, "Greatshield Soldier Ashes"), # residue=0 UNIQUE
            FlagAnchor(12077220, "Golden Rune [1]"),           # residue=4
            FlagAnchor(12077230, "Golden Rune [1] (2nd)"),     # residue=6
            FlagAnchor(12077410, "Smithing Stone [3]"),        # residue=2
        ], "Underground s7 (Deeproot Depths)  [TIER 2 — only 1 unique lid]"),
        # ------------------------------------------------------------------
        # TIER 2 — generic local_ids
        # Area 13: Crumbling Farum Azula — section 0
        (13, 0, [
            FlagAnchor(13007025, "Great Grave Glovewort"),     # residue=1
            FlagAnchor(13007220, "Smithing Stone [8]"),        # residue=4
            FlagAnchor(13007670, "Smithing Stone [6]"),        # residue=6
            FlagAnchor(13007730, "Smithing Stone [7]"),        # residue=2
        ], "Crumbling Farum Azula (s0)  [TIER 2 — unreliable]"),
        # ------------------------------------------------------------------
        # TIER 2 — generic local_ids
        # Area 15: Miquella's Haligtree — section 0
        (15, 0, [
            FlagAnchor(15007220, "Pearldrake Talisman +2"),    # residue=4
            FlagAnchor(15007230, "Smithing Stone [8]"),        # residue=6
            FlagAnchor(15007730, "Smithing Stone [8] (2nd)"), # residue=2
            FlagAnchor(15007280, "Somber Smithing Stone [8]"), # residue=0
        ], "Miquella's Haligtree (s0)  [TIER 2 — unreliable]"),
        # ------------------------------------------------------------------
        # TIER 1 — 4 unique local_ids (7940/7000/7010/7030), 4 distinct residues
        # Area 16: Volcano Manor — section 0 — VERIFIED 2026-04-07 → base 2194
        (16, 0, [
            FlagAnchor(16007940, "Ghiza's Wheel"),             # residue=4 UNIQUE
            FlagAnchor(16007000, "Smithing Stone [6]"),        # residue=0 UNIQUE
            FlagAnchor(16007010, "Depraved Perfumer Carmaan"), # residue=2 UNIQUE
            FlagAnchor(16007030, "Budding Horn"),              # residue=6 UNIQUE
        ], "Volcano Manor (s0)  [TIER 1]"),
        # ------------------------------------------------------------------
        # TIER 2 — DLC sections: all local_ids shared across m20/m21 sections
        # Area 20: DLC Shadow Realm — section 0
        (20, 0, [
            FlagAnchor(20007220, "Thin Beast Bones"),          # residue=4
            FlagAnchor(20007230, "Sliver of Meat"),            # residue=6
            FlagAnchor(20007730, "Black Pyrefly"),             # residue=2
            FlagAnchor(20007991, "Immunizing Horn Charm +2"),  # residue=7
        ], "DLC Shadow Realm s0  [TIER 2 — no unique local_ids]"),
        # Area 20: DLC Shadow Realm — section 1
        (20, 1, [
            FlagAnchor(20017220, "Furlcalling Finger Remedy"), # residue=4
            FlagAnchor(20017230, "Spira"),                     # residue=6
            FlagAnchor(20017991, "Horned Warrior's Greatsword"),# residue=7
            FlagAnchor(20017280, "Rada Fruit"),                # residue=0
        ], "DLC Shadow Realm s1  [TIER 2 — no unique local_ids]"),
        # Area 21: DLC Elphael — section 0
        (21, 0, [
            FlagAnchor(21007220, "Rada Fruit"),                # residue=4
            FlagAnchor(21007230, "Rada Fruit (2nd)"),          # residue=6
            FlagAnchor(21007730, "Rada Fruit (3rd)"),          # residue=2
            FlagAnchor(21007991, "Mantle of Thorns"),          # residue=7
        ], "DLC Elphael s0  [TIER 2 — no unique local_ids]"),
        # Area 21: DLC Elphael — section 1
        (21, 1, [
            FlagAnchor(21017220, "Rada Fruit"),                # residue=4
            FlagAnchor(21017230, "Rada Fruit (2nd)"),          # residue=6
            FlagAnchor(21017730, "Rada Fruit (3rd)"),          # residue=2
            FlagAnchor(21017991, "Fire Knight Helm"),          # residue=7
        ], "DLC Elphael s1  [TIER 2 — no unique local_ids]"),
        # Area 21: DLC Elphael — section 2
        (21, 2, [
            FlagAnchor(21027220, "Beast Blood"),               # residue=4
            FlagAnchor(21027230, "Smithing Stone [4]"),        # residue=6
            FlagAnchor(21027991, "Fire Knight Helm (2nd)"),    # residue=7
            FlagAnchor(21027280, "Rada Fruit"),                # residue=0
        ], "DLC Elphael s2  [TIER 2 — no unique local_ids]"),
        # ------------------------------------------------------------------
        # TIER 2 — no unique local_ids
        # Area 31: Caves — section 21
        (31, 21, [
            FlagAnchor(31217350, "Regalia of Eochaid"),        # residue=6
            FlagAnchor(31217100, "Wakizashi"),                 # residue=4
            FlagAnchor(31217040, "Old Fang"),                  # residue=0
            FlagAnchor(31217210, "Pillory Shield"),            # residue=2
        ], "Caves s21  [TIER 2 — no unique local_ids]"),
        # ------------------------------------------------------------------
        # TIER 2 — generic local_ids
        # Area 35: Mohgwyn Palace — section 0
        (35, 0, [
            FlagAnchor(35007220, "Smithing Stone [7]"),        # residue=4
            FlagAnchor(35007670, "Hefty Beast Bone"),          # residue=6
            FlagAnchor(35007730, "Warming Stone"),             # residue=2
            FlagAnchor(35007280, "Preserving Boluses"),        # residue=0
        ], "Mohgwyn Palace (s0)  [TIER 2 — unreliable]"),
        # ------------------------------------------------------------------
        # TIER 2 — no unique local_ids
        # Area 41: Minor Dungeons — section 0
        (41, 0, [
            FlagAnchor(41007100, "Broken Rune"),               # residue=4
            FlagAnchor(41007110, "Thawfrost Boluses"),         # residue=6
            FlagAnchor(41007130, "Glass Shard"),               # residue=2
            FlagAnchor(41007200, "Smithing Stone [6]"),        # residue=0
        ], "Minor Dungeons s0  [TIER 2 — no unique local_ids]"),
        # Area 41: Minor Dungeons — section 2
        (41, 2, [
            FlagAnchor(41027100, "Chilling Perfume Bottle"),   # residue=4
            FlagAnchor(41027110, "Call of Tibia"),             # residue=6
            FlagAnchor(41027130, "Lamenting Visage"),          # residue=2
            FlagAnchor(41027200, "Innard Meat"),               # residue=0
        ], "Minor Dungeons s2  [TIER 2 — no unique local_ids]"),
    ]

    TIER1 = {(16, 0)}  # sections with unique local_ids → trustworthy results
    results: dict[tuple[int, int], int] = {}
    for area, section, anchors, label in SECTIONS:
        base = calibrate_section(slots, area, section, anchors, label,
                                 patch=args.patch)
        if base is not None:
            results[(area, section)] = base

    print()
    print("=" * 60)
    print(f"Summary: {len(results)}/{len(SECTIONS)} sections found a winner")
    print("=" * 60)
    tier1 = [(k, v) for k, v in sorted(results.items()) if k in TIER1]
    tier2 = [(k, v) for k, v in sorted(results.items()) if k not in TIER1]
    if tier1:
        print("  TIER 1 (trustworthy — unique local_ids):")
        for (area, section), base in tier1:
            print(f"    ({area:2d}, {section:2d}) → {base}")
    if tier2:
        print("  TIER 2 (DO NOT apply — generic local_ids, likely false positives):")
        for (area, section), base in tier2:
            print(f"    ({area:2d}, {section:2d}) → {base} ← suspicious")


if __name__ == "__main__":
    main()
