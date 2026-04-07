"""
Tests for calibrate_dungeon_bases.py core functions.

All tests use synthetic event_flags buffers with seeded bits — no save file
required. This keeps tests fast, deterministic, and runnable in CI.

Formula for general dungeon events (local_id < 7000):
    local_id     = flag_id % 10000
    byte_offset  = base + local_id // 8
    bit_position = 7 - (local_id % 8)
"""

import sys
from pathlib import Path

# Make the scripts directory importable
sys.path.insert(0, str(Path(__file__).parent.parent))

import pytest
from calibrate_dungeon_bases import FlagAnchor, BaseCandidate, find_base_for_flags, corroborate


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

EF_SIZE = 200_000  # synthetic buffer size — covers full scan range


def empty_ef() -> bytes:
    return bytes(EF_SIZE)


def ef_with_flags(base: int, *flag_ids: int) -> bytes:
    """Return a zeroed buffer with the specified flags set at the given base."""
    buf = bytearray(EF_SIZE)
    for flag_id in flag_ids:
        local_id = flag_id % 10000
        byte_off = base + local_id // 8
        bit_pos = 7 - (local_id % 8)
        if byte_off < EF_SIZE:
            buf[byte_off] |= (1 << bit_pos)
    return bytes(buf)


# Anchors used across tests — real m14 flags confirmed set for Bee
RED_WOLF  = FlagAnchor(flag_id=14000850, name="Red Wolf of Radagon")   # local_id=850, residue=850%8=2
MOONGRUM  = FlagAnchor(flag_id=14000499, name="Moongrum, Carian Knight")  # local_id=499, residue=499%8=3


# ---------------------------------------------------------------------------
# find_base_for_flags
# ---------------------------------------------------------------------------

class TestFindBaseForFlags:

    def test_finds_seeded_base_when_all_anchors_set(self):
        """Tracer bullet: both anchors seeded at base 29987 → candidate returned."""
        ef = ef_with_flags(29987, RED_WOLF.flag_id, MOONGRUM.flag_id)

        results = find_base_for_flags(ef, [RED_WOLF, MOONGRUM])

        bases = [c.base for c in results]
        assert 29987 in bases

    def test_returns_empty_for_all_zero_buffer(self):
        results = find_base_for_flags(empty_ef(), [RED_WOLF, MOONGRUM])
        assert results == []

    def test_does_not_return_base_when_only_one_anchor_is_set(self):
        """Partial match: only Red Wolf seeded — base must NOT appear in results."""
        ef = ef_with_flags(29987, RED_WOLF.flag_id)   # Moongrum NOT set

        results = find_base_for_flags(ef, [RED_WOLF, MOONGRUM])

        bases = [c.base for c in results]
        assert 29987 not in bases


# ---------------------------------------------------------------------------
# corroborate
# ---------------------------------------------------------------------------

class TestCorroborate:

    def test_passes_candidate_with_two_distinct_residues(self):
        """Red Wolf (residue 2) + Moongrum (residue 3) → 2 distinct → accepted."""
        candidate = BaseCandidate(base=29987, anchors=[RED_WOLF, MOONGRUM])

        result = corroborate([candidate])

        assert candidate in result

    def test_rejects_candidate_where_all_anchors_share_same_residue(self):
        """Two anchors both with local_id % 8 == 0 → 1 distinct residue → rejected."""
        anchor_a = FlagAnchor(flag_id=14000800, name="Anchor A")  # 800 % 8 == 0
        anchor_b = FlagAnchor(flag_id=14000400, name="Anchor B")  # 400 % 8 == 0
        candidate = BaseCandidate(base=29987, anchors=[anchor_a, anchor_b])

        result = corroborate([candidate])

        assert candidate not in result


# ---------------------------------------------------------------------------
# find_consistent_base
# ---------------------------------------------------------------------------

class TestFindConsistentBase:

    def test_returns_base_present_in_all_slot_candidate_lists(self):
        """Base 29987 appears in every slot's candidates → returned."""
        from calibrate_dungeon_bases import find_consistent_base

        slot_results = [
            [BaseCandidate(base=100, anchors=[RED_WOLF]),
             BaseCandidate(base=29987, anchors=[RED_WOLF, MOONGRUM])],
            [BaseCandidate(base=200, anchors=[RED_WOLF]),
             BaseCandidate(base=29987, anchors=[RED_WOLF, MOONGRUM])],
            [BaseCandidate(base=300, anchors=[MOONGRUM]),
             BaseCandidate(base=29987, anchors=[RED_WOLF, MOONGRUM])],
        ]

        result = find_consistent_base(slot_results, min_slots=2)

        assert any(c.base == 29987 for c in result)

    def test_excludes_base_appearing_in_only_one_slot(self):
        """Base 100 appears in only slot 0 → excluded when min_slots=2."""
        from calibrate_dungeon_bases import find_consistent_base

        slot_results = [
            [BaseCandidate(base=100, anchors=[RED_WOLF]),
             BaseCandidate(base=29987, anchors=[RED_WOLF, MOONGRUM])],
            [BaseCandidate(base=29987, anchors=[RED_WOLF, MOONGRUM])],
        ]

        result = find_consistent_base(slot_results, min_slots=2)

        assert not any(c.base == 100 for c in result)
        assert any(c.base == 29987 for c in result)

    def test_returns_empty_when_no_base_meets_min_slots(self):
        """All candidates differ across slots → empty result."""
        from calibrate_dungeon_bases import find_consistent_base

        slot_results = [
            [BaseCandidate(base=100, anchors=[RED_WOLF])],
            [BaseCandidate(base=200, anchors=[MOONGRUM])],
        ]

        result = find_consistent_base(slot_results, min_slots=2)

        assert result == []


# ---------------------------------------------------------------------------
# find_base_for_flags — scan_start / scan_end respect
# ---------------------------------------------------------------------------

class TestScanRange:

    def test_does_not_return_base_outside_scan_range(self):
        """Base seeded at 500 but scan starts at 1000 → not returned."""
        ef = ef_with_flags(500, RED_WOLF.flag_id, MOONGRUM.flag_id)

        results = find_base_for_flags(ef, [RED_WOLF, MOONGRUM],
                                      scan_start=1000, scan_end=5000)

        assert not any(c.base == 500 for c in results)

    def test_returns_base_within_narrowed_scan_range(self):
        """Base seeded at 29987 found when scan covers 27000-33000."""
        ef = ef_with_flags(29987, RED_WOLF.flag_id, MOONGRUM.flag_id)

        results = find_base_for_flags(ef, [RED_WOLF, MOONGRUM],
                                      scan_start=27000, scan_end=33000)

        assert any(c.base == 29987 for c in results)
