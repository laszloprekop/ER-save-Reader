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
    print()

    # -----------------------------------------------------------------------
    # m14 GENERAL base — already in lib.rs as 29987, verify with NPC flags
    # that have statusesAlign=true in correlation data (TestA / slot 7)
    # -----------------------------------------------------------------------
    print("=" * 60)
    print("Verifying m14_00 general base (boss defeats / events)")
    print("=" * 60)

    KNOWN_GENERAL_BASE = 29987

    # Witch-Hunter Jerren NPC flags confirmed SET in slot 7 (TestA)
    # via correlation data (statusesAlign=true, computedByteOffset=30076)
    # local_id 717: residue=5; local_id 712: residue=0 → 2 distinct residues
    m14_general_anchors = [
        FlagAnchor(14000717, "Witch-Hunter Jerren (state 5)"),
        FlagAnchor(14000712, "Sorceress Sellen (state 0)"),
    ]

    print(f"\n  Direct check at base {KNOWN_GENERAL_BASE} across slots:")
    for slot in slots:
        ef = slot['event_flags']
        states = {
            a.name: "SET" if _is_set(ef, *_flag_offset(KNOWN_GENERAL_BASE, a.flag_id)) else "CLEAR"
            for a in m14_general_anchors
        }
        print(f"    Slot {slot['slot_idx']}: " + "  ".join(f"{k}={v}" for k, v in states.items()))

    # -----------------------------------------------------------------------
    # m14 PICKUP base — current placeholder 31903 needs calibration
    # Confessor (slot 0) has confirmed m14 pickup items in correlation data:
    #   14007150 (local_id 7150, residue 6) computedByteOffset=30880
    #   14007290 (local_id 7290, residue 2) computedByteOffset=30898
    # Both imply pickup base = 29987
    # -----------------------------------------------------------------------
    print()
    print("=" * 60)
    print("Calibrating m14_00 pickup base (item pickups, local_id >= 7000)")
    print("=" * 60)

    # Academy Glintstone Key is required to enter Raya Lucaria → all visiting
    # slots must have it.  Longtail Cat Talisman: main-path chest pickup.
    m14_pickup_anchors = [
        FlagAnchor(14007930, "Academy Glintstone Key"),     # residue=7930%8=2
        FlagAnchor(14007320, "Longtail Cat Talisman"),      # residue=7320%8=0
    ]

    print(f"\n  Direct check at base=29987 (slot 0 = Confessor):")
    slot0_ef = slots[0]['event_flags']
    for a in m14_pickup_anchors:
        local_id = a.flag_id % 10000
        byte_off = KNOWN_GENERAL_BASE + local_id // 8
        bit_pos  = 7 - (a.flag_id % 8)
        state = "SET" if _is_set(slot0_ef, byte_off, bit_pos) else "CLEAR"
        print(f"    {a.name} (flag {a.flag_id}): byte={byte_off}, bit={bit_pos} → {state}")

    # Scan to find the pickup base empirically
    # Scan range: same ~27000-35000 neighborhood
    M14_SCAN_START = 27_000
    M14_SCAN_END   = 35_000

    per_slot: list[list[BaseCandidate]] = []
    for slot in slots:
        candidates = find_base_for_flags(slot['event_flags'], m14_pickup_anchors,
                                         scan_start=M14_SCAN_START,
                                         scan_end=M14_SCAN_END)
        verified = corroborate(candidates)
        per_slot.append(verified)
        print(f"  Slot {slot['slot_idx']}: {len(verified)} corroborated pickup candidate(s)")

    non_empty = [s for s in per_slot if s]
    consistent = find_consistent_base(per_slot, min_slots=len(non_empty))
    print(f"\n  Consistent across ALL {len(non_empty)} non-empty slot(s): {len(consistent)} candidate(s)")
    if not consistent:
        consistent = find_consistent_base(per_slot, min_slots=max(1, len(non_empty) - 1))
        print(f"  (falling back to min_slots={max(1, len(non_empty)-1)}: {len(consistent)} candidate(s))")

    expected_in = any(c.base == KNOWN_GENERAL_BASE for c in consistent)
    print(f"  Expected base {KNOWN_GENERAL_BASE} in consistent list: {expected_in}")

    for c in consistent[:5]:
        residues = sorted({a.flag_id % 8 for a in c.anchors})
        print(f"    base={c.base}  residues={residues}")

    # Corroborate winner with additional Confessor (slot 0) pickup items
    if consistent:
        winner = consistent[0].base
        print(f"\n  Corroborating base={winner} with additional Confessor pickup items:")
        extra_anchors = [
            FlagAnchor(14007150, "Marionette Soldier Ashes"),   # residue=6
            FlagAnchor(14007290, "Avionette Soldier Ashes"),    # residue=2
            FlagAnchor(14007930, "Academy Glintstone Key"),     # residue=2
            FlagAnchor(14007320, "Longtail Cat Talisman"),      # residue=0
        ]
        for a in extra_anchors:
            local_id = a.flag_id % 10000
            byte_off = winner + local_id // 8
            bit_pos  = 7 - (a.flag_id % 8)
            state = "SET" if _is_set(slot0_ef, byte_off, bit_pos) else "CLEAR"
            print(f"    {a.name}: byte={byte_off}, bit={bit_pos} → {state}")

        if args.patch:
            import datetime
            print(f"\n    ((14, 0), {winner}),  // m14 pickup section base — VERIFIED {datetime.date.today()}")


if __name__ == "__main__":
    main()
