> **ARCHIVED** (2026-02-08): This document has been archived. Superseded by [EVENT-FLAG-GEOGRAPHY.md](../EVENT-FLAG-GEOGRAPHY.md) (core concept absorbed).

# Identifying one verified flag unlocks many nearby flags.

The save file stores flags in blocks:

- Block-based flags (60000-99999): All flags in a 1000-range block share the same base offset. If we find flag 67120 at byte 3549, we calculate base = 3549 - (67120-67000)/8 = 3546, and now ALL 67xxx flags can be calculated.
- Tile-based flags (10-digit): Each map tile is 875 bytes. Finding one flag anchors all ~7000 flags in that tile.
- Dungeon flags (8-digit): Similar - section_size=1125 means one anchor reveals an entire dungeon section.

The "islands of similar patterned bytes" are exactly these flag blocks. The huge zero gaps are either:

1. Unused flag ranges (FromSoft left gaps for expansion)
2. Character data sections (stats, inventory)
3. Different save slot data

This is why the ground_truth system stores base_offset per block rather than individual flag offsets - one anchor propagates to hundreds of flags.

## Block Base Offset Overlaps

The verified block bases reveal that flag storage is NOT contiguous. Different flag categories can have overlapping byte ranges:

| Block | Base Offset | End Offset | Category |
|-------|-------------|------------|----------|
| 60000 | 2548 | 2672 | Progression |
| 71000 | 2625 | 2749 | Tutorial graces |
| 73000 | 2662 | 2786 | Dungeon graces |
| 72000 | 2750 | 2874 | DLC graces |

**Known Overlaps**:
- 60000 vs 71000: bytes 2625-2672 (48 bytes)
- 71000 vs 73000: bytes 2662-2749 (88 bytes)
- 72000 vs 73000: bytes 2750-2786 (37 bytes)

This isn't a bug - it reflects how FromSoft allocates flag storage. Each block category may only use a portion of its theoretical 1000-flag capacity. The reverse lookup handles this by:

1. Checking verified block flags FIRST (before simple flags)
2. Using the first matching block when overlaps occur
3. Trusting empirically verified base offsets over calculated ones

The overlap also explains why naive "simple flag" calculations (flag_id / 8) can produce incorrect results for bytes in the 2500-3500 range - these bytes belong to block flags, not simple flags.
