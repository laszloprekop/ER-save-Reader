# Reconstruction Fact Inventory — the union both apps need

**Last updated**: 2026-07-25 (slice 08 equipment core side)

> **Epistemic header**
> **Status: CURRENT (seed).** Written alongside the walking-skeleton extraction of
> `er-reconstruct` (ADR-0010, issue #1). Identity is reconstructed by the shared
> core today; every other concern below is still reconstructed twice (Rust in this
> reader, TypeScript in elden-map) and is listed here to **order the strangler
> slices 04–09**.
> - **Claims**: which reconstructed facts each app produces today, per concern, and
>   what the shared core's fact set (the *union*) must therefore carry.
> - **Evidence**: this reader's `src/vm/export.rs` field set and the extracted
>   `save/` structs; elden-map's `shared/types.ts` (`CharacterStats`,
>   `CharacterEquipment`, `CharacterSlotInfo`), `shared/slot-schema.ts`, and
>   `server/src/saveParser.ts` (`playerPosition`).
> - **Methodology**: field-by-field diff of the two apps' reconstructed outputs,
>   grouped by concern. Names/coordinates/labels are deliberately excluded — those
>   are Enrichment (per-app), not facts (ADR-0010).
> - **Obsolete**: none yet. Re-audit each row as its slice lands and the TypeScript
>   is deleted.

---

## How to read this

Each concern is one **strangler slice**. A concern's core fact set is the **union**
of what the two apps reconstruct — the core is widened to the union first, then the
duplicated implementation is deleted (ADR-0010), each step guarded by the
conformance corpus. **Facts only**: ID-keyed resolved state. Anything that is a
name, a coordinate label, or a UI string is Enrichment and stays in each app.

- **R** = this reader reconstructs it today (`src/vm/…`, exported by `export.rs`).
- **M** = elden-map reconstructs it today (`shared/`, `server/src/saveParser.ts`).
- **Core** = what `ReconstructedCharacter` must carry (the union), as facts.

---

## 01 — Identity  *(seed — SHARED already)*

| Fact | R | M | Core (fact) |
|------|---|---|-------------|
| character name | ✓ | ✓ | `name: String` |
| character level | ✓ | ✓ | `level: u32` |
| starting class | ✓ | ✓ (`class`) | `class_id: u8` (raw archetype id) |
| slot active | ✓ | ✓ (`isActive`) | reconstruct errors `InactiveSlot`; caller decides |
| gender | ✓ | – | append: `gender_id` |
| match-making weapon level | ✓ | – | append: `u8` |
| steam id | ✓ | – | out of scope — account metadata, not character state |

Class id → "Vagabond"/"Astrologer" is a **Canonical Name** lookup (Enrichment),
never baked into the fact. Same for gender id → "♂/♀".

## 04 — Stats

| Fact | R | M | Core (fact) |
|------|---|---|-------------|
| vigor, mind, endurance, strength, dexterity, intelligence, faith, arcane | ✓ | ✓ | 8 × `u32` |
| level (derived/stored) | ✓ | ✓ | already in identity |
| runes held / runes memory | ✓ (`souls`, `souls_memory`) | ✓ (`runes`, `runesMemory`) | `runes: u32`, `runes_memory: u32` |
| hp / fp / stamina — current, max, base-max | – | ✓ | append all three × three |
| scadutree level, spirit-ash (revered) level | ✓ | – | append: `u8` each (DLC) |

Union = superset of both. elden-map is the only source for the derived
hp/fp/stamina triples; this reader is the only source for the DLC blessing levels.

## 05 — Bosses & graces (event flags)  *(core CARRIES these now — issue #4 core side)*

| Fact | R | M | Core (fact) |
|------|---|---|-------------|
| boss defeat flags | ✓ | ✓ | ✅ `bosses: Vec<FlagFact>` — state per boss id, resolved per save |
| grace (site of grace) flags | ✓ | ✓ | ✅ `graces: Vec<FlagFact>` — state per grace id |

Both already route through `wasm-event-flags` for offset **resolution** (ADR-0008);
this slice moves the *selection of which ids mean "boss"/"grace"* into the core.
No flag base tables enter the core — positions stay resolved per save.

**Status (2026-07-24, core side landed in `er-reconstruct`):** `reconstruct()` now
returns `graces` and `bosses` as `Vec<FlagFact { id, state }>`, ascending by id, the
tri-state (`Set`/`Clear`/`Unknown`) resolved per save via `wasm-event-flags`
(`ResolvedFlags`, one shared origin scan). The grace/boss id **selection** moved
into the core (`src/facts/flag_ids.rs`, extracted from the reader's `GRACES`/`BOSSES`
id columns; names stayed behind as Enrichment). A boss id→family router
(`boss_family_state`) mirrors the reader's `world_flag_state`. Conformance corpus
gained grace/boss expectations, cross-checked against the reader's independent
`discovered` tally (slot 0: 179 graces / 49 bosses; Godrick + Margit Set; the
uncalibrated Consecrated-Snowfield tile `1248550800` honestly Unknown).
**Still open for #4:** reader renders graces/bosses *from these facts* (its own
computation retired), and elden-map resolves them via WASM + deletes its grace/boss
TS — both gated behind #3 (browser-calls-core), so deferred.

## 06 — Pickups (world + dungeon)  *(core CARRIES world+dungeon now — issue #5 core side)*

| Fact | R | M | Core (fact) |
|------|---|---|-------------|
| world pickup flags | ✓ (`world_pickups`) | ✓ | ✅ `world_pickups: Vec<FlagFact>` — state per `getItemFlagId` |
| dungeon pickup flags | ✓ | ✓ | ✅ `dungeon_pickups: Vec<FlagFact>` — state, legacy-dungeon family (ADR-0008) |
| summoning pools | ✓ | – | append: flag state per pool id — **deferred** (family mis-identified) |

Tables store `getItemFlagId`, never a row id (CLAUDE.md); the core takes the
same rule. A bare 10-digit tile id stays ambiguous between families — the caller
picks world vs pickup, never the value.

**Status (2026-07-24, core side landed in `er-reconstruct`):** `reconstruct()` now
returns `world_pickups` (2438 unique ids) and `dungeon_pickups` (1950) as
`Vec<FlagFact { id, state }>`, ascending, resolved per save via `wasm-event-flags`
(the same shared origin scan as #4). A `pickup_family_state` router mirrors the
reader's `pickup_state` (`tile_pickup` / `dungeon_pickup` / `world_state`, else
`Unknown`). The id **selection** moved into the core (`src/facts/pickup_ids.rs`),
extracted sorted/deduped from the reader's machine-checked `world_pickups.rs`
(`flag_id`) and `dungeon_pickups.rs` (`event_flag`); the reader's
`gen-world-pickups` test stays the source-of-truth machine-check, and the core
arrays are a regenerated derivative. Corpus guards pickups three ways: table
totals, known-truth differential bounds (L93 slot 593 world / 386 dungeon vs L9
slot 3 / 1), and monotonic collected anchors. **Summoning pools stay deferred** —
the reader itself reads them `false` because the family is mis-identified
(`events.rs`; ADR-0008 refuse-don't-guess), so no honest fact exists to move yet.
**Still open for #5:** reader renders pickups from these facts; elden-map WASM + TS
delete — both gated behind #3.

## 07 — Inventory  *(core CARRIES held inventory now — issue #6 core side)*

| Fact | R | M | Core (fact) |
|------|---|---|-------------|
| held inventory | ✓ | ✓ | ✅ `held_inventory` + `held_key_items`: `Vec<InventoryFact { category, item_id, quantity }>` |
| storage box inventory | ✓ | ✓ (GaItems map) | append: same shape — **deferred** (`storage_inventory_data`, a later slice) |

Keyed by **item identity**, never GaItem handle (handles churn — CONTEXT.md).
elden-map's handle→itemId resolution collapses into item identity in the core.

**Status (2026-07-25, core side landed in `er-reconstruct`):** `reconstruct()` now
returns `held_inventory` (the common list) and `held_key_items` (the key-item list) as
`Vec<InventoryFact { category, item_id, quantity }>`, in save order. This is the first
**non-flag** fact and the reusable **GaItem-decode foundation** (`src/facts/inventory.rs`,
mirroring the reader's `InventoryItemViewModel::from_save`): a weapon/armor/ash handle
indirects through the slot's gaitem map (new `get_ga_items` accessor) to its param id;
accessory/consumable ids XOR-decode from the handle. `category` (Weapon/Armor/Accessory/
Item/Aow) is carried because id → name resolves against a different DB per category and
weapon/item ids overlap numerically — fact shape chosen deliberately over the bare
`{item_id, count}` (append-only contract). The handle and per-save `inventory_index` are
dropped (churny, not identity). Corpus guards it two ways: exact `held_common_count` /
`held_key_count` against the reader export's distinct counts (694/103 Confessor slot 0,
18/0 V1 slot 2), and targeted `items` per-id known-truth (Academy Glintstone Staff, key
item Crafting Kit, Longsword, Memory of Grace). **Storage box deferred** — same decode,
different list, a later slice. **Still open for #6:** reader renders held inventory from
these facts; elden-map WASM + TS delete — both gated behind #3.

## 08 — Equipment  *(core CARRIES equipment now — issue #9 core side)*

| Fact | R | M | Core (fact) |
|------|---|---|-------------|
| right hand ×3, left hand ×2/3 | ✓ | ✓ | ✅ `{ slot, item_id, upgrade }` per occupied slot |
| arrows ×2, bolts ×2 | ✓ | ✓ | ✅ same shape, `upgrade` 0 |
| head / chest / arms / legs | ✓ | ✓ | ✅ same shape, `upgrade` 0 |
| talismans ×4 | ✓ | ✓ | ✅ same shape, `upgrade` 0 |

Item id + upgrade level only; the name is Enrichment.

**Status (2026-07-25, core side landed in `er-reconstruct`):** `reconstruct()` now
returns `equipment` as a flat `Vec<EquipmentFact { slot, item_id, upgrade }>` — the
**positional** counterpart to held inventory, built on the #6 GaItem-decode foundation
(`src/facts/equipment.rs`, mirroring the reader's `EquipmentViewModel::from_save`). The
flat-Vec-with-`EquipSlot`-enum shape was chosen by the USER over a fixed struct
(append-only contract), consistent with the prior Vec facts; **only occupied slots**
appear. Weapons and projectiles indirect through the gaitem map (weapon `item_id` is the
full reinforced value, `upgrade = item_id % 100`; projectiles carry no upgrade); armor
clears the armor tag off the indirected id; talismans XOR-decode straight from the
handle — reusing `inventory`'s now-`pub(crate)` `map_item_id` + key constants (one
decode, two callers). A new `get_chr_asm2` accessor exposes the loadout handles. Empty
slots carry one of **two sentinels** — `0` or `u32::MAX` (`0xFFFFFFFF`, a *cleared* slot
whose gaitem-map entry indirects to `u32::MAX` too) — both dropped; missing the second is
how a cleared quiver leaks a bogus `item_id 4294967295` (caught by the V1-backup corpus
case). Quick-slots and pouch are **not** equipment facts (a later slice may append them).
Corpus guards it two ways: exact `equipment_count` vs the reader export's occupied
fact-relevant slots (Confessor slot 0 = 17 incl. 3 Unarmed hands; V1 slot 2 = 10), and
targeted per-slot `equipment` known-truth (Curved Club +14, Magic Brass Shield +13,
Confessor Hood, Gold Scarab; V1's +0 starting gear). **Still open for #9:** reader renders
equipment from these facts; elden-map WASM + TS delete — both gated behind #3, deferred.

## 09 — World position  *(elden-map only today — ported INTO the core)*

| Fact | R | M | Core (fact) |
|------|---|---|-------------|
| player world position | – | ✓ (`playerPosition`) | append: coordinates + map/block id |

This is the archetypal ADR-0010 union-add: the reader has no reason to surface it,
elden-map needs it for its map, so it is **ported into the core, not dropped**. The
raw coordinates are facts; turning them into a map pin (POI labels, grid overlay)
is elden-map's Enrichment and never enters a reader crate.

---

## Not facts — stays per-app (Enrichment)

- id → **Canonical Name** (class, item, boss, grace, map). Game reference data;
  exposed later as a separate `nameOf(id)` lookup, never baked into facts (ADR-0010).
- map coordinates → screen pins, community POI labels, grid overlays (elden-map).
- UI layout, sorting, filtering, export formatting (both).
- live-session concerns — file watching, slot diffing, timeline, WebSocket — are
  **not reconstruction** and stay in elden-map (ADR-0010).

## Slice ordering rationale

Identity first (smallest fact, proves every layer — done). Then **stats** and
**flags** (bosses/graces), because they are the highest-traffic bug surface and the
flag layer is already shared. **Pickups** and **inventory** next (largest tables,
most divergence risk). **Equipment** and **world position** last — equipment is
mechanical, and world position is single-sourced from elden-map so it has no
reader oracle until its slice. Each slice widens the core to the union, then deletes
the corresponding TypeScript, gated by the conformance corpus.
