# Elden Ring Save File Ground Truth

**Generated**: 2026-01-11
**Verification Method**: Empirical multi-save analysis using `scripts/run_verification.py`
**Primary Data Source**: Decompiled game files + empirical save file testing

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: ERA-MIXED — one current section, the rest pre-migration.** The load-bearing part is **"Flag Family Origin (discovered 2026-07-20)"** (CLAUDE.md points here) and the container-structure section; trust those. Treat everything dated 2026-01/02 as historical.
> - **Claims**: save-file container structure (BND4, slot layout, EF section bounds); the per-save family Origin model; block/tile/dungeon offset formulas; "empirically verified formulas as of 2026-01-11".
> - **Evidence**: empirical multi-save byte analysis; the family-Origin section is backed by attributed transitions and the resolver.
> - **Methodology**: the Origin section is current (resolve per save via the resolver). The older formula sections used the pre-reset Python `run_verification.py` and single-save measurements.
> - **Obsolete**: "Event Flag Formulas" area-specific base offsets and "Empirically Verified Formulas (2026-01-11)" are superseded — positions float per save; resolve via `crates/wasm-event-flags` + `knowledge/claims/event-flags.json`, never hardcode. `run_verification.py` is the pre-reset Python lab (migration step-5 removal target).

---

## Executive Summary

This document is the **single source of truth** for Elden Ring save file parsing and event flag calculations. It supersedes all previous research documents (archived in `docs/archive/`).

> **CRITICAL UPDATE (2026-07-05) — per-family float:** all "block base" and offset values
> in this document are valid only **per flag family, per save layout**. Flag families
> (graces, catacombs, …) sit at independently floating bases across saves (measured:
> grace-vs-catacombs family delta 0 bytes on one save, ~77-141 on another, ~490 on a
> third), and regions shift by different amounts even within one before/after pair
> (b24→b25: GaItems +16, flag region +4). A single per-save "EF anchor" therefore cannot
> position all families. EF detection was reworked the same day: the v0.16 "structural
> detection" was DISPROVEN (~146k overshoot onto a lookalike region — the b24/b25 kill
> pair proves flags live at gaEnd+~35-37k, not ~222k) and replaced with a gaEnd-windowed
> grace-validation scan pinned by committed conformance fixtures
> (`crates/wasm-event-flags/tests/`). See `CONTEXT.md`, ADR-0003 amendment, ADR-0007,
> and BACKLOG Priority 0b.
>
> **PRIMARY SOURCE RECOVERED (2026-07-05):** the game's own `eventflagalloclist` files
> (decompressed from the raw install, parsed to `knowledge/game/eventflag-alloclists.json`)
> define the legacy-map flag layout as CSV `slot,map_id,class` with
> `base = REGION_BASE + slot × 1125`. With REGION_BASE = 4112 (grace-anchored coords of
> the verified saves) this reproduces the entire legacy table including the
> byte-verified m14 base (slot 23 → 29,987) and the previously removed m18 (slot 35 →
> 43,487) / m19 (slot 38 → 46,862): the LAYOUT is authoritative; only the region's
> in-save position floats per save. "Areas 20/21" in 8-digit flags are DLC maps m20
> (Belurat) / m21 (Enir-Ilim) at DLC alloclist slots 150-156 — old "Stranded
> Graveyard"/"Haligtree" labels were wrong. Regulation param XMLs regenerated at
> version 11611000 (= 1.16.1, save-era match); see the evidence catalog
> (`knowledge/evidence-catalog.json`, corpora `game-raw-1162` and `game-extracts`).
>
> **CLAIMS STORE LIVE — REGION MAP CORRECTED (2026-07-05, `knowledge run`):** the first
> pipeline-generated claims store (`knowledge/claims/event-flags.json`, ADR-0004) was
> produced from 24 attributed transition pairs of the Confessor captures (the numbered
> 01-10 session of 2025-12-29 plus the b-series of 2026-01-23..25; 20 flags verified,
> 4 honest hypotheses) and **supersedes this document for the families it covers**.
>
> **EXTENDED 2026-07-06** with 7 pairs from the `snapshots-root` corpus's slot-2
> (V1) and slot-7 (an uncharacterized instrument character) 2026-02-09 session —
> 27 pairs total, 27 flags verified across both corpora (see the intra-session
> drift amendment below).
>
> Measured region map (grace-relative, per-save floating bases):
>
> | family | layout | base (grace_rel) |
> |--------|--------|------------------|
> | world-state-b (dungeon graces, 60xxx/66xxx world flags …) | `(flag − 50000) / 8` | ~146.6k |
> | tile-open-world (m60 event flags, incl. overworld bosses) | `tile_slot × 875 + local/8` | ~483.47k |
> | tile-pickup-row-id (world pickups by ItemLotParam row id) | `tile_slot × 875 + (row_id % 10000)/8` | ~483.97k |
> | legacy-dungeon-pickup (dungeon pickups, local ≥ 7000) | `alloclist_slot × 1125 + local/8` | ~1,529.85k |
> | legacy-dungeon (event flags, incl. catacombs bosses) | `alloclist_slot × 1125 + local/8` | ~1,529.98k |
>
> Event flags and pickup tracking are SEPARATE regions per area type (the pickup
> regions sit ~500 bytes above the tile event region / ~129 bytes below the legacy
> event region and float independently). World pickups store `row_id =
> getItemFlagId − 7000` — beware capture annotations that miscompute this
> (b15/b16's `rowId-1042371300` was actually 1042370300).
>
> Cross-session measurements on the SAME character confirm per-save base float:
> the December session measured tile-pickup base 483,889 / world-state-b base
> 146,514 vs the b-series' 483,969 / 146,598-146,618. Within a session, bases are
> USUALLY stable — the pipeline exploits this as a **multi-file differential**:
> an ambiguous set-transition candidate whose implied base is independently
> re-measured by a later resolved pair must stay SET in that pair's files (these
> flags are set-monotonic), which disambiguated the Golden Order Seal pickup
> (candidate at grace_rel 851,264 cleared in later files; 851,389 persisted).
>
> **AMENDMENT (2026-07-06, `snapshots-root` s7 pairs):** bases can also drift
> BETWEEN individual captures within one session, not just between sessions. The
> `snapshots-root` corpus's slot-7 tile-pickup-row-id base measured 482,861 at
> 21:51 and 482,931 at 22:15-22:21 — a ~70-byte shift inside a single ~30-minute
> capture run. This does not break candidate resolution: each pair's cross-checks
> evaluate an expectation flag's bit at the CANDIDATE'S OWN implied base in that
> pair's own `after` file, never a cached base from the resolving pair, so the
> mechanism is inherently robust to intra-session drift. The corpus's four
> world-state-b pairs (progression 60220, graces 71800/76101) did not resolve at
> all (0 or many isolated-flip candidates) — left as unresolved rather than
> forced, consistent with the evidence catalog's own note that this corpus has
> cross-session churn and an unresolved flag-byte interpretation for the 71800
> pair.
>
> Copy A vs copy B (c03-c04, grace 76310): **open-world graces (76xxx) set the bit
> in BOTH world-state blocks** — copy A (the grace-anchor region detection pins) and
> copy B (~146.6k above). Dungeon graces (71xxx/73xxx) set copy B only. Both blocks
> use the same `(flag − 50000)/8` packing.
>
> Multi-slot differential (2026-07-06, V1/V2/V3 instrument files): **slots of ONE
> save file float independently** — the tile-pickup base measured 482,865 for slots
> 3/4 but 482,869 for slot 2 in the same file (the ±4 record-list float, across
> slots). Cross-slot flag checks must calibrate the family base per slot; the
> pipeline locates each slot's base by pattern-matching within ±64 of an
> anchor-pair-pinned base.
>
> Four refuted conventions are tombstoned in the store: (1) tile base 337,375 was
> expressed relative to the poisoned structural anchor (measured base − 337,375
> reproduces the ~146.1k struct-walk delta); (2) the legacy region does NOT start at
> grace_rel 4,112 — the 28-31k span (old "m14=29,987" etc.) is a u32-record LIST whose
> insertions cause the ±4 region shifts, not the legacy flag bitmap (the alloclist
> slot×1125 LAYOUT itself is confirmed within the real region); (3) no universal EF
> anchor (families float independently, measured per pair); (4) dungeon graces are NOT
> at `(flag−50000)/8` from the grace base — they live in a second world-state block
> (~146.6k) that also mirrors the tutorial anchor flags (the lookalike-region mystery).
> Block-base rows in the tables below that touch these families are era-specific legacy
> claims pending the per-family cutover (migration step 4).

### Key Findings

| Category | Status | Details |
|----------|--------|---------|
| **Graces (76xxx)** | **VERIFIED** | Block base=3250, empirically confirmed |
| **Stormveil Graces (71xxx)** | **VERIFIED** | Block base=9315, 8/9 graces matched (CORRECTED 2026-01-22) |
| **Tutorial Graces (71800)** | **VERIFIED** | Block base=2725, validation flags confirmed |
| **Progression (60xxx)** | **VERIFIED** | Block base=2548, cross-validated with 3 items |
| **Map Fragments (62xxx)** | **VERIFIED** | Block base=1500, verified from 62174 match |
| **Cookbooks (67xxx-68xxx)** | **VERIFIED** | Block base=3546 (corrected from 3987!) |
| **Dungeon Graces (73xxx)** | **VERIFIED** | Block base=2664, 13/13 dungeon graces matched |
| **Whetblades (65xxx)** | Unverified | Block base ~1875 needs testing |
| **World Pickups (col >= 30)** | **VERIFIED** | Tile formula works, base=337375 (CORRECTED 2026-02-15) |
| **World Pickups (col < 30)** | Unverified | Western tiles may use different storage |
| **Dungeon Boss Flags (30,31,32)** | **VERIFIED** | Catacombs/Caves/Tunnels bases discovered |
| **Dungeon Boss Flags (Legacy)** | Unverified | Stormveil, Academy, etc. need investigation |
| **Consumable Treasures** | **TRACKABLE** | Via Row ID formula (discovered 2026-02-02) |

---

## Save File Structure

### Constants

```
SLOT_SIZE           = 0x280000 (2,621,440 bytes per character slot)
SLOT_COUNT          = 10 (maximum character slots)
EVENT_FLAGS_SIZE    = 0x1BF99F (1,833,375 bytes per slot)
BND4_HEADER_SIZE    = 0x40 (64 bytes before file entries)
BND4_ENTRY_SIZE     = 0x20 (32 bytes per file entry)
SLOT_CHECKSUM_SIZE  = 16 bytes (MD5 checksum before slot data)
```

### BND4 Container Structure

The save file is a BND4 container with 12 files:
- Files 0-9: Character slots
- Files 10-11: Other data (user profile, etc.)

**Reading slot offsets from BND4 entries:**
```python
# Slot offset is at position 0x10 within each entry (4-byte little-endian)
entry_offset = 0x40 + (slot_index * 0x20) + 0x10
bnd4_offset = struct.unpack_from('<I', data, entry_offset)[0]
slot_header_offset = bnd4_offset + 16  # Skip 16-byte checksum
```

### Slot Layout (after 16-byte checksum)

```
Offset    | Size      | Content
----------|-----------|------------------------------------------
0x0       | 4         | Version (0xFB = 251 for valid slots)
0x4       | 4         | Map ID
0x20      | Variable  | GaItems (0x1400 max × variable bytes each)
...       | ...       | Other structures (PlayerGameData, Equipment, etc.)
Variable  | 1,833,375 | EventFlags (grace-family base ~ gaItemsEnd + 35-37k)
```

**Critical**:
1. Slot offsets are **NOT at fixed intervals** - they must be read from BND4 entries
2. Each slot has a 16-byte MD5 checksum header before the actual data
3. EventFlags offset **VARIES** per slot due to the variable-size GaItems section (grace-family base = gaItemsEnd + ~35,100..37,100 across observed saves)
4. ~~Structural detection (v0.16.0)~~ **DISPROVEN 2026-07-05**: the sequential-section model overshoots by ~146k onto a lookalike region (the "~222K" belief came from it). Detection is the gaEnd-windowed grace-validation scan pinned by conformance fixtures (`crates/wasm-event-flags/tests/`)

### Event Flags Section

- **Size**: 1,833,375 bytes (15,466,999 flags theoretically)
- **Format**: Bit array, 8 flags per byte
- **Bit Order**: Big-endian style (`bit_position = 7 - (flag_id % 8)`)

---

## Flag Family Origin (discovered 2026-07-20)

**This section supersedes the fixed byte offsets below.** Flag families do not sit at
constant offsets. An **append-only u32 list** ahead of the flag data grows as the
character plays — one record per progression event — pushing every family 4 bytes
further along each time. Fixed offsets measured on one save are therefore only valid
for that save's layout, which is why historical offsets drifted and disagreed.

Measuring from the list's END removes the drift completely:

```
family_base = ga_items_end + flag_list_end + FAMILY_CONSTANT
```

| family | constant | evidence |
|---|---|---|
| world-state-b (graces, world state) | 117,192 | 47 captures, spread 0 |
| tile-open-world (overworld boss kills) | 454,067 | 2 attributed pairs, exact agreement |
| tile-pickup-row-id (world pickups) | 454,567 | 38 captures, spread 0 |
| legacy-dungeon (legacy-map boss kills, NPC/world state) | 1,500,567 | 2 attributed pairs, exact agreement |
| legacy-dungeon-pickup | 1,500,442 | 16 captures, spread 0 |

Re-derive any of these with `er-save-editor knowledge family-constants`, which measures
each family from the attributed flips that pinned it (a chain independent of the
`list-hunt` route that produced the first four) and emits
`knowledge/claims/family-constants.json`.

> **A bare 10-digit tile id does not tell you its family.** Open-world flags and pickup
> row_ids both use localId < 7000 and their regions sit 500 bytes apart, so a function
> that routes on the id alone will read a plausible wrong bit rather than fail. The
> caller must choose: `is_tile_world_flag_set` or `is_tile_pickup_set`. Note also that
> `pickup_data.rs` stores row_ids (its `event_flag` = `item_lot_id`), while the game's
> param tables use `getItemFlagId` = row_id + 7000.

### Legacy-map families (anything not an open-world tile)

Legacy maps address flags by **allocation slot**, not by map id:

```
byte = alloc_slot(map) * 1125 + localId / 8      bit = 7 - flagId % 8
```

Slots come from the game's own `eventflagalloclists` (corpus `game-raw-1162`,
decompressed to `knowledge/game/eventflag-alloclists.json`), mirrored into the
reference implementation as `LEGACY_ALLOC_SLOTS` with a conformance test that re-reads
the source file. They did **not** come from `get_dungeon_general_bases()` — the disproven
"+3375 per area" stride table, whose own audit comment recorded entries contradicted by
every save on this machine. That table was **deleted 2026-07-20 (ADR-0008)** along with
every export reaching it; `tests/export_shape_conformance.rs` fails if it reappears.

The same localId split applies as for tiles — `is_dungeon_flag_set` for localId < 7000,
`is_dungeon_pickup_set` for >= 7000 — and the two regions sit 125 bytes apart.

> **Those two ranges overlap — real, and harmless (settled 2026-07-20).** Both families
> index by the raw `localId / 8`, and the pickup base sits 125 bytes lower, which places
> the pickup range at bytes 750-999 of the event block. The consequence is exact:
>
> ```
> event localId L   and   pickup localId L + 1000   resolve to the same bit
> ```
>
> **The single-base alternative is refuted, not merely unproven.** In file b33, where two
> event flags and three pickups are all known set, each reads set at its own base and
> clear at the other's, and the byte that actually flipped for pickup 30027000 across
> b20→b21 is at the pickup base while the single-base prediction stayed `0x00` through
> the transition.
>
> Note also that "pickups index from `localId - 1000` at a shared base" is *the same
> model*, not a competing one: `base_ev + slot*1125 + (L-1000)/8` expands to
> `(base_ev - 125) + slot*1125 + L/8`, which is the shipped formula exactly.
>
> **The overlap never fires because its band is empty on the event side.** Legacy event
> flags cluster in localId 0-2999, pickups in 7000-7999; 6000-6999 is used by neither.
> Verified across 4,540 distinct legacy flags from three independent sources, and against
> the primary source — `ItemLotParam_map` (regulation 1.16.1) carries 2,143 legacy
> `getItemFlagId`s in 7000-7999 and none in 6000-6999. If a legacy event flag in that
> band is ever found, it collides with a real pickup and the layout needs revisiting.

**Two database discrepancies found against the primary source**, both small and neither
affecting a shipped read:

- `ItemLotParam_map` gives m15_00 (Miquella's Haligtree) seven `getItemFlagId`s at
  localId 1200-1290, below the 7000 split. They are the only legacy pickups in the
  primary source outside 7000-7999, and `dungeon_pickups.rs` does not carry them at all —
  a DB coverage gap, not a misread. Were they added, `is_dungeon_pickup_set` would
  reject them on the 7000 rule and they would read Unknown.
- Conversely `dungeon_pickups.rs` carries two entries the primary source does not list
  as legacy pickups: `12022995` and `12022997` (m12_02, localId 2995/2997). They read
  Unknown today. Unverified provenance; treat as suspect.

**Two maps are allocated twice** (m34_12 → slots 62 and 144; m40_00 → 70 and 170).
Nothing in the evidence establishes which allocation holds the bits, so both resolve to
`None`. A guess there reads a wrong bit ~92KB away.

> **An old disproof retired here.** m18 (Stranded Graveyard) was removed from the legacy
> base table as DISPROVEN because its span read all zeros, even though every character
> necessarily kills Soldier of Godrick in the tutorial. Via alloc slot 35 and a resolved
> origin, 18000850 now reads correctly on both the Wretch and the Confessor. The layout
> was right; only the base was wrong — the same lesson as the 337,375 constant. Check
> whether a legacy constant is a real structure wearing the wrong anchor before
> dismissing it.

The families are **rigidly locked to each other**: the distances between them
(337,375 / 1,383,250 / 1,045,875) were measured through an independent route and agree
with these constants to the byte. So locating one family locates all of them.

The list has **no length prefix** — the bytes before it are zeros — so its end must be
scanned for. Reference implementation: `crates/wasm-event-flags/src/lib.rs`
(`find_flag_list_end`, `resolve_family_base`), locked by
`crates/wasm-event-flags/tests/origin_conformance.rs`. It validates its assumptions and
returns nothing rather than a plausible-looking wrong answer, because a wrong base
reads garbage flags silently. Full derivation and negative results: `docs/BACKLOG.md`
step 4b; generated evidence: `knowledge/claims/{family-distances,list-hunt,
origin-validation}.json`.

Validated out-of-sample on five characters (Confessor, Wretch, V1, V2, V3) across two
backup saves and the snapshots-root corpus. The constants are **measured, not derived**;
the scan is bounded-structural, not a parse of the enclosing section.

---

## Event Flag Formulas

### Block-Based Formula (5-6 digit flags)

For flags in specific ranges, a block-based calculation is used:

```python
block_start = (flag_id // 1000) * 1000
base_offset = BLOCK_BASES[block_start]
relative = flag_id - block_start
byte_offset = base_offset + relative // 8
bit_position = 7 - (flag_id % 8)
```

**Verified Block Bases** (empirically confirmed via diff analysis):

| Block Start | Base Offset | Category | Status | Evidence |
|-------------|-------------|----------|--------|----------|
| 60000 | **2548** | Progression | **Verified** | Cross-validated: 60100 (Crafting Kit), 60130 (Whetstone Knife), 60220 (Furled Finger) |
| 62000 | **1500** | Map Fragments | **Verified** | 62174 (Ailing Village) matched at offset 1521 |
| 67000 | **3546** | Cookbooks | **Verified** | Missionary's Cookbook [4] diff - byte 3549 changed (NOT 3990!) |
| 68000 | 3671 | Cookbooks (continued) | Calculated | 67000 base + 125 |
| 71000 | **9315** | Stormveil Graces | **Verified** | Full search found 8/9 graces at base 9315 (CORRECTED 2026-01-22) |
| 71800 | **2725** | Tutorial Graces | **Verified** | Validation flags 71800, 71801 at byte 2725 |
| 73000 | **2664** | Dungeon Graces | **Verified** | 13/13 catacombs/caves/tunnels matched via slot comparison |
| 76000 | **3250** | World Graces | **Verified** | Validation flags 76100, 76101 |

**IMPORTANT**: Block bases are NOT contiguous! Grace blocks are stored at completely different offsets:
- Block 71000 (Stormveil) at base 9315 is FAR from block 71800 (Tutorial) at base 2725
- The 67xxx range is stored AFTER 76xxx in memory

**Unverified Block Bases** (need empirical testing):

| Block Start | Estimated Offset | Category | Notes |
|-------------|------------------|----------|-------|
| 65000 | ~1875 | Whetblades | Needs diff testing |
| 72000 | ~2750 | Legacy Dungeon Graces | Needs diff testing |
| 74000-75000 | ~3000-3125 | Extended Graces | Needs diff testing |

### Tile-Based Formula (10-digit base game flags)

For flags in format `10XXYYZZZZ` (note: prefix is 2 digits):

```python
# Extract components (prefix is "10", not "1")
row = int(flag_str[2:4])      # XX (tile row, e.g., 43)
col = int(flag_str[4:6])      # YY (tile column, e.g., 50)
local_id = int(flag_str[6:])  # ZZZZ (local flag ID, e.g., 0010)

# Calculate offset (CORRECTED 2026-02-15)
base_offset = 337375          # CORRECTED: 485330 was 147955 bytes too high
bytes_per_slot = 875
slots_per_row = 40
row_base = 33
col_base = 30

tile_offset = ((row - row_base) * slots_per_row + (col - col_base)) * bytes_per_slot
byte_offset = base_offset + tile_offset + (local_id // 8)
bit_position = 7 - (local_id % 8)  # Uses local_id, not flag_id
```

**Verified Example**: Flag 1043500010 (Smoldering Butterfly at m60_43_50)
- row=43, col=50, local=10
- tile_offset = ((43-33)*40 + (50-30)) * 875 = 420 * 875 = 367500
- byte_offset = 337375 + 367500 + 1 = **704876**
- bit_position = 7 - (10 % 8) = 5
- Extraction: (byte >> 5) & 1
- **Corrected 2026-02-15**: Verified via before/after snapshot diffs across 3 characters, 10+ pairs

**LIMITATIONS**:

1. **LocalId >= 7000 is UNTRACKABLE**: Each tile slot has 875 bytes = 7000 flags max. ItemLotParam `eventFlagId` often creates localId 7300+ for treasures - these flags have no storage space.
2. **Col < 30 may not work**: Tiles west of col_base=30 may use different storage region (needs empirical verification).

### Dungeon Formula (8-digit flags)

For flags in format `AASSZZZZ`:

```python
map_area = int(flag_str[0:2])   # AA (dungeon area code)
section = int(flag_str[2:4])     # SS (section within dungeon)
local_id = int(flag_str[4:8])    # ZZZZ (local flag ID)

# Area-specific base offset
base_offset = DUNGEON_BASES[map_area]
section_offset = section * 1125
byte_offset = base_offset + section_offset + (local_id // 8)
bit_position = 7 - (local_id % 8)
```

**Verified Dungeon Bases** (minor dungeons, 2026-01-12):

| Area | Base Offset | Category | Status | Evidence |
|------|-------------|----------|--------|----------|
| 30 | **27411** | Catacombs | **Verified** | 5 boss flags matched via slot comparison |
| 31 | **28634** | Caves | **Verified** | 5 boss flags matched via slot comparison |
| 32 | **31577** | Tunnels | **Verified** | 4 boss flags matched via slot comparison |

**Unverified Dungeon Bases** (legacy dungeons):

| Area | Category | Notes |
|------|----------|-------|
| 10 | Stormveil Castle | Needs verification |
| 14 | Academy of Raya Lucaria | Needs verification |
| 16 | Volcano Manor | Needs verification |
| 35 | Mohgwyn Palace | Needs verification |

---

## Validation Flags (Anchors)

> **CORRECTED 2026-07-20 — these are NOT 100% reliable.** The claim below was
> falsified by out-of-sample validation (`knowledge/claims/origin-validation.json`):
> on the V1/V2/V3 pickup-debugging characters, **71800 and 76100 read CLEAR** in both
> backup saves, while 71801 and 76101 read SET. A ±4096 search around the verified base
> found no position at which all four are SET, so this is genuine character state, not
> a detection error. Anchor sets that assume a "normal" progression will reject valid
> minimal characters — the false-negative mode this project keeps rediscovering.
> The byte offsets in the table are also save-specific; see *Flag Family Origin* above.

> **NAMES CORRECTED 2026-07-20.** This table had 76100 and 76101 swapped. Verified
> against the primary source — `BonfireWarpParam.param.xml` (regulation 1.16.1, the
> save era) in the `game-extracts` corpus, whose rows carry `eventflagId` directly:
> 76100 = `[Limgrave] Church of Elleh`, 76101 = `[Limgrave] The First Step`. Both of
> the app's grace databases (`src/db/graces_data.rs`, `src/db/graces/maps.rs`) already
> had this right; only this table was wrong. A wrong NAME is indistinguishable from a
> wrong OFFSET when you are looking at a table of grace names, so this cost real time.

| Flag ID | Byte Offset | Bit | Name | Notes |
|---------|-------------|-----|------|-------|
| 71800 | 2725 | 7 | Cave of Knowledge | Tutorial grace — CLEAR on V1/V2/V3 |
| 71801 | 2725 | 6 | Stranded Graveyard | Tutorial grace |
| 76100 | 3262 | 3 | Church of Elleh | CLEAR on V1/V2/V3 |
| 76101 | 3262 | 2 | The First Step | First world grace |

Usable as corroboration for characters known to have progressed past the tutorial.
Never as a universal validity test.

---

## Known Limitations

### Consumable Treasures (Row ID Formula)

Consumables with `getItemFlagId` creating localId >= 7000 were previously thought to be untrackable via tile formula. However, the **Row ID formula** (discovered 2026-02-02) tracks these pickups using a separate bitfield.

See [EVENT-FLAG-GEOGRAPHY.md](EVENT-FLAG-GEOGRAPHY.md#row-id-tracking-for-world-pickups-critical-discovery-2026-02-02) for the Row ID formula details.

**Summary**: World pickups with localId >= 7000 ARE tracked via `row_id = getItemFlagId - 7000` using a dedicated row ID bitfield (base: 1037373320).

---

## Verification Data

### Empirically Verified Formulas (as of 2026-01-11)

| Formula | Category | Status | Evidence |
|---------|----------|--------|----------|
| Block 60000 | Progression | **VERIFIED** | Cross-validated with 60100, 60130, 60220 (base=2548) |
| Block 62000 | Map Fragments | **VERIFIED** | 62174 (Ailing Village) matched at offset 1521 (base=1500) |
| Block 67000 | Cookbooks | **VERIFIED** | Missionary's Cookbook [4] pickup diff (base=3546, NOT 3987!) |
| Block 71000 | Stormveil Graces | **VERIFIED** | 8/9 graces matched (base=9315, CORRECTED from 2625 on 2026-01-22) |
| Block 71800 | Tutorial Graces | **VERIFIED** | Validation flags 71800, 71801 (base=2725) |
| Block 76000 | World Graces | **VERIFIED** | Validation flags 76100, 76101 (base=3250), 65% match rate |
| Block 73000 | Dungeon Graces | **VERIFIED** | 13/13 dungeon graces matched via slot comparison (base=2664) |
| Block 78000 | POI Flags | UNVERIFIED | 0% match rate - base offset needs discovery |
| Tile (col >= 30) | World Pickups | **VERIFIED** | Smoldering Butterfly (1043500010) temporal diff, base=337375 (corrected 2026-02-15) |
| Tile (col < 30) | World Pickups | UNVERIFIED | Western tiles may use different storage |
| Dungeon Area 30 | Catacombs | **VERIFIED** | 5 boss flags matched (base=27411) |
| Dungeon Area 31 | Caves | **VERIFIED** | 5 boss flags matched (base=28634) |
| Dungeon Area 32 | Tunnels | **VERIFIED** | 4 boss flags matched (base=31577) |
| Dungeon (Legacy) | Stormveil, Academy, etc. | UNVERIFIED | Base offsets need discovery |

**Note on EventFlags offset**: The EventFlags offset varies per slot (~0x36000-0x37000) depending on GaItems count and two variable-size sections (EquipProjectileData, Regions). Since v0.16.0, **structural detection** computes the exact offset by sequential section parsing. The pre-EventFlags gap is a constant 29 bytes (0x1D), verified across 898 slot measurements.

### Verification Methodology

1. **Diff Analysis**: Compare before/after save snapshots to find exact byte changes
2. **Reverse Calculation**: From changed byte/bit, derive flag ID
3. **Forward Verification**: Use derived base to predict new flag offsets
4. **Cross-Validation**: Test against multiple save files and characters

---

## Files Reference

### Primary Sources (Decompiled Game Data)

| File | Location | Content |
|------|----------|---------|
| ItemLotParam_map.param.xml | regulation-bin/ | World pickup definitions |
| BonfireWarpParam.param.xml | regulation-bin/ | Grace site data |
| ShopLineupParam.param.xml | regulation-bin/ | Shop item flags |
| common.emevd.js | event/ | Event scripts with flag logic |

### Verification Tools

| Script | Purpose |
|--------|---------|
| `scripts/run_verification.py` | Main verification runner |
| `scripts/verification/save_parser.py` | Save file parsing |
| `scripts/verification/ground_truth_loader.py` | Loads formulas from ground_truth_offsets.json |
| `scripts/verification/diff_analyzer.py` | Before/after comparison |

### Output Files

| File | Content |
|------|---------|
| `ground_truth_offsets.json` | Verified flag offsets and formulas |
| `scripts/extracted_event_flags.json` | All known flags from game files |

---

## Archived Documents

Previous research documents have been moved to `docs/archive/`:

- `SAVE_FILE_PARSING_RESEARCH.md`
- `EVENT_FLAG_OFFSET_INVESTIGATION.md`
- `SAVE_FILE_SCHEMA based on ER-Save-Editor.md`
- `SAVE-FILE-ANATOMY.md`
- `SAVE-FILE-STRUCTURE-RESEARCH.md`
- `EVENT-FLAG-SYSTEM-ANALYSIS.md`

These contain historical research that informed this ground truth but may have outdated or incorrect information.

---

## Usage Example

```python
from verification import SaveParser, FlagFormulas

# Parse save file
parser = SaveParser()
save = parser.parse("ER0000.sl2")

# Check a specific flag
formulas = FlagFormulas()
flag_id = 76100  # The First Step grace

results = formulas.calculate_offset(flag_id)
if "block" in results and results["block"].is_valid:
    offset = results["block"].byte_offset
    bit = results["block"].bit_position

    for slot in save.slots:
        is_set = parser.check_flag_at_offset(slot.event_flags, offset, bit)
        print(f"Slot {slot.slot_index}: {'SET' if is_set else 'NOT SET'}")
```

---

## Contributing

To improve the ground truth:

1. Run `python scripts/run_verification.py --verbose`
2. Review mismatches and unverified flags
3. Use `diff_analyzer.py` with before/after saves to discover correct offsets
4. Update `ground_truth_offsets.json` with corrected base offsets (NOT `flag_formulas.py`, which is deprecated)
5. Re-run verification to confirm improvements

---

## Changelog

### 2026-02-15
- **Structural EventFlags detection** (v0.16.0): Sequential section parsing replaces content-based search
- Pre-EventFlags gap verified as constant 29 bytes (0x1D) across 898 slot measurements
- Section chain: GaItems → PlayerGameData → ... → TutorialData → 29-byte gap → EventFlags
- Two variable-size sections parsed: EquipProjectileData (4 + count×8), Regions (4 + count×4)
- Content-based search retained as fallback only

### 2026-01-25
- **REVERT** tile formula base_offset: 489981 → **485330** (reverted to original)
- The 2026-01-20 "correction" to 489981 was WRONG - offset 857482 showed no change during pickup
- Re-verified: Smoldering Butterfly (1043500010) at byte **852831** bit 5 (0x00→0x20)
- Added calibration_anchors section to ground_truth_offsets.json for runtime validation
- **Final value: 485330** (confirmed in `crates/wasm-event-flags/src/lib.rs`)

### 2026-01-12
- **VERIFIED** Dungeon formula bases for minor dungeons:
  - Area 30 (Catacombs): base=**27411** (5 boss flags matched)
  - Area 31 (Caves): base=**28634** (5 boss flags matched)
  - Area 32 (Tunnels): base=**31577** (4 boss flags matched)
- Formula: `byte = base + section * 1125 + local_id // 8`

### 2026-01-11 (Late Evening Update)
- **VERIFIED** 73xxx dungeon graces base: **2664** (13/13 catacombs/caves/tunnels matched via slot comparison)
- Previous investigation was looking at wrong byte range (2875 vs 2664)

### 2026-01-11 (Evening Update)
- **CORRECTED** cookbook base: 3987 → **3546** (verified via Missionary's Cookbook [4] diff)
- **VERIFIED** 60xxx progression base: **2548** (cross-validated with Crafting Kit, Whetstone Knife, Furled Finger)
- **VERIFIED** 62xxx map fragment base: **1500** (verified from 62174 Ailing Village match)

### 2026-01-11 (Initial)
- Created ground truth document
- Verified 71xxx (base=2625), 76xxx (base=3250) from validation flags
- Verified tile formula for col >= 30 (base=485330, col_base=30) - LATER CORRECTED

---

*Last updated: 2026-02-15*
