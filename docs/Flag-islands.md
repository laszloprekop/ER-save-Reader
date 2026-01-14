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
