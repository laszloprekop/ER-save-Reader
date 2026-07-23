# Architecture Deepening Plan

**Last updated**: 2026-07-22

> **Epistemic header**
> **Status: LIVING RECORD — workstreams 0 and A are decided; B, C, D are proposals.**
> Deepenings for shallow modules found in the 2026-07-22 architecture review. **0 and A**
> were settled by grilling on 2026-07-22 and their sections record decisions, several of
> them grounded in measurements taken during it (the 249-warning dead-code census, the
> visibility-and-consumers check). **B, C and D** are not agreed: no ADR records them and
> no code exists. Each unsettled workstream carries its **open questions** — the points
> that need settling before implementation, not after.
> Vocabulary is `CONTEXT.md` for the domain and the `codebase-design` glossary (module,
> interface, implementation, depth, seam, adapter, leverage, locality) for the architecture.
> Distinct from `docs/ARCHITECTURE.md`, which is SUPERSEDED and is **not** what this
> replaces — that described the pre-reset detection model. This describes module shape only
> and asserts nothing about flag positions.

---

## Why these four and not six

The review surfaced six candidates. Four of them are two designs:

- **02 (`ResolvedFlags`) and 03 (`FlagState`)** are the same interface. `ResolvedFlags`'
  methods return `FlagState`; designing them apart means designing one interface twice and
  reconciling it later.
- **04 (`Evidence`) and 05 (`Claims`)** are the same seam. `Claims` records input hashes,
  which only `Evidence` can supply, and both are what is left of `family_distances.rs`
  once its private helpers become modules.

So: **A** (library seam), **B** (flag reading), **C** (knowledge pipeline), **D** (read
model, designed but deferred).

### Dependency order

```
A ──────────────┬──────────────┬─────────────┐
(library seam)  │              │             │
                ▼              ▼             ▼
         B (app-side)   C (knowledge)   D (read model)
                ▲                             ▲
                │                             │
      B (crate-side) ─────────────────────────┘
      needs nothing
```

- **A blocks C and D outright**, and blocks the app-side half of B. Not because of coupling
  — because `src/` has no interface to test through, so "and now it is testable" is not
  true for any of them until A lands.
- **B's crate-side half depends on nothing.** `crates/wasm-event-flags` already has a lib
  target and three conformance suites. It can start immediately, in parallel with A.
- **D consumes B** (a `Character` holds one `ResolvedFlags`) and is deferred regardless.

---

## Workstream 0 — remove `src/calibration.rs` (DECIDED 2026-07-22)

997 lines with exactly one reference in the tree: `mod calibration;` at `main.rs:13`.
Nothing calls it. It is not dormant (no feature flag) and not dead (still compiled, still
linted — last touched today, in `5f3daa4 fix(lint): unblock clippy`). That is the
unreachable-and-compiled state ADR-0009 exists to end.

**Its stated purpose has been built, differently and better.** `docs/BACKLOG.md:385` names
it as *"the right shape"* for a single-save family-base detector — but that entry is step
4b, whose own chain reduces it: *"locating ONE family locates all of them, and 4b reduces
from 'build a detector per family' to 'pin a single origin'"* (`:484`), then *"Next: pin the
single origin. That is now the whole of 4b"* (`:503`). `CONTEXT.md` records the Origin as
established 2026-07-20, and `resolve_family_base_in_ef` is that detector.

Two further reasons it cannot simply be left:

- **Its module doc still asserts the refuted premise**: *"The tile formula base_offset
  (337375 …) is constant across saves."* That is the tombstone, stated as fact, to the next
  reader who opens the file.
- **Its tests pin the disproven model.** `test_get_tile_offset_calibrated` asserts
  `get_tile_offset_calibrated(1043500010, 337375) == (704876, 5)` — 337,375 used as a base,
  a literal `export_shape_conformance.rs` bans from the wasm crate. They are not coverage
  worth keeping.

**Action**: delete the file and the `mod` declaration, and **amend the `BACKLOG.md:385`
note** to record that the Origin superseded it. Deleting silently would leave that note
pointing at a file that no longer exists; the note is the part that must not vanish.

> **Related, and NOT part of this workstream.** Two `flag_id → byte offset` functions live
> outside the wasm crate, where `export_shape_conformance.rs` cannot see them:
> `src/generated/ground_truth.rs:164` (`byte_offset = VERIFIED_TILE_BASE_OFFSET + …`, still
> live via `db/pickup_flags.rs`) and `save/common/event_flags_detection.rs`. The first is
> ADR-0006's deliberate mid-migration state and is not a defect; both are recorded here so
> a later reader knows the ADR-0008 guard's reach stops at the crate line.

---

## Workstream A — the library seam

**Deepening**: none. A is not a deepening; it is the precondition for verifying the other
three. Listed first because it is what they are all waiting on.

### The problem, precisely

`Cargo.toml` declares no `[lib]` and `src/lib.rs` does not exist. The crate is
binary-only, so no module in `src/` has an interface anything can cross.
`tests/regression_suite.rs:3` records the consequence in a comment: *"Basic validation
tests that don't require crate imports."* Its seven tests read JSON files off disk and
check hashes. Not one of them executes a line of `src/`.

Everything testable today is testable by accident of being inline: 16 `#[cfg(test)] mod
tests` blocks, which can only reach private items in their own file.

### The change

```toml
# Cargo.toml
[lib]
name = "er_save_reader"
path = "src/lib.rs"

[[bin]]
name = "er-save-reader"
path = "src/main.rs"
```

`src/lib.rs` takes the module declarations currently at the top of `main.rs`, as `pub mod`:

```rust
pub mod db;
pub mod generated;
pub mod knowledge;
pub mod read;
pub mod save;
pub mod ui;
pub mod util;
pub mod vm;
#[cfg(feature = "save-writeback")]
pub mod write;
```

`main.rs` keeps `fn main`, the eframe boot, the `App` impl and the `knowledge` CLI
dispatch, and reaches everything else through `use er_save_reader::…`.

### Visibility: `pub(crate)` by default — DECIDED 2026-07-22

Every top-level module starts `pub(crate) mod`. Promote to `pub mod` only where `main.rs`
or a test fails to compile without it, so the interface ends up defined by what the test
surface actually needs.

The reason is not breakage risk — **there is none.** The root `Cargo.toml` declares no
`repository` and no registry metadata; elden-map consumes `crates/wasm-event-flags`, which
carries its own `[lib] crate-type = ["cdylib", "rlib"]`. The app lib's only possible
consumers are `main.rs` and `tests/`, both in-repo, so narrowing later is a compile error
fixed in the same commit. An earlier draft of this file claimed otherwise; it was wrong.

The real reason is the dead-code sweep. Module visibility alone controls it:

- `pub mod db;` → everything inside is externally reachable → **never** dead-code warned
- `pub(crate) mod db;` → analysis applies to all 1,502 `pub fn` inside, no item-level edits

Publishing a module wholesale switches the sweep off for its entire contents. That sweep is
what found `calibration.rs`.

### The suppression is six attributes, not one — MEASURED 2026-07-22

`#![allow(dead_code)]` at `main.rs:2` is nearly vestigial: removing it produces **zero** new
warnings. The real suppressors are *inner* attributes at the top of six `mod.rs` files —
`db/`, `ui/`, `vm/`, `util/`, `save/common/`, `generated/` (plus `ui/tokens/`) — each
exempting its whole subtree. That is ~195k of 205k lines with analysis switched off.

Lifting all of them yields **249 warnings**: `ui` 130, `db` 91, `vm` 15, `generated` 7,
`save` 4, `util` 2. Separately, 372 `allow(unused, non_snake_case, …)` attributes exist but
are *all* in `util/param_structs.rs`, on game-derived param structs where the naming is not
ours to fix. Those are legitimate and out of scope.

**Scope: lift `vm/`, `save/common/`, `util/` only** (21 warnings — the read path, where the
findings are ADR-relevant). `ui/` and `db/` keep their attributes with a TODO naming them
as deferred; their 221 are overwhelmingly unused accessors on generated tables, a different
and much lower-signal job.

### Disposition rule for the 21 — DECIDED 2026-07-22

Three ways, each already backed by an ADR, so no new judgement is invented:

- **Delete** what encodes an abandoned model. `InventoryRoute::{None, Add}`
  (`vm/inventory/mod.rs:25`) — named in ADR-0009 as evidence the editing had already
  stopped, documented but never removed. `verify_event_flags_offset`
  (`save/common/event_flags_detection.rs:102`) — validates a detected offset by testing
  tutorial graces at *hardcoded* byte offsets, which `CLAUDE.md` forbids outright
  ("71800/76100 are NOT universal anchors … never use them as a validity test").
- **Gate behind `save-writeback`** what is write-path machinery ADR-0009 calls dormant:
  `util/bit.rs::set_bit`, and the `changed` / `current_index` fields at `vm/equipment.rs:41`.
- **Keep + `#[allow(dead_code)]` with a one-line reason** what mirrors the save format or an
  external schema. `vm/stats.rs:34`'s `stamina` sits inside `hp, max_hp, fp, max_fp,
  stamina, max_stamina` under a `// HP, FP, SP (Stamina)` comment — deleting it would break
  a format-mirroring block and lose the fact that the save carries a current-stamina value.
  `util/verification_records.rs:21`'s fields belong to a `#[derive(Deserialize)]` contract
  we do not own, whose doc comment records a field-rename history.

**In a save-format reader an unread field is often documentation of the format.** That is
why "delete everything the compiler flags" is not the rule.

### Also true

- **`save-writeback` must keep compiling.** `cargo check --features save-writeback` per
  `CLAUDE.md`, and the `#[cfg]`-gated `mod write` moves with the rest.
- **Module-inception paths do not change.** `save::save::save::Save` and friends stay as
  they are; `module_inception` is already allowed in `Cargo.toml`.
- **`read/` is fine.** 5 lines, the `Read` trait, implemented by every save struct — the
  read-side counterpart to `write/`'s dormant `Write`. By the leverage measure it is the
  deepest module in the tree.

### Definition of done — DECIDED 2026-07-22

One test in `tests/` importing `er_save_reader`, asserting that `pickup_flag_state` returns
`None` — not `Some(false)` — for an unresolvable flag region and for out-of-family ids.

It needs no fixture, so it runs on a fresh clone, and it guards the tri-state invariant that
`vm/export.rs:332` is currently the only test defending. It is also the assertion B builds
on, so A and B meet at a test rather than at a merge.

**The 21 inline tests in `src/db/pickup_flags.rs` do not move.** Integration tests see only
`pub` items, so moving them would force promotions that fight `pub(crate)`-by-default. They
exercise *internal* seams, which is legitimate; the new test crosses the external seam.

An end-to-end read ("load a save, assert 179 graces") is deliberately **not** A's first
test. The committed fixtures are raw 128KB slot prefixes, not `.sl2` files, so `Save::from_path`
cannot consume them; a real save means resolving a catalog entry to verified bytes, which is
exactly what C1's `Evidence` module exists to stop being hand-rolled. Writing it here would
add a seventh copy. **It lands after C1.**

---

## Workstream B — `FlagState` and `ResolvedFlags`

**Deepening**: fourteen public entry points, each a five-line body around the same
resolve-then-index, become one type that resolves once and answers per family. Five
different collapses of Unknown become one enum.

### B1 — `FlagState`

```rust
/// The three states of a flag read. `Unknown` is not `Clear`: the position could
/// not be resolved, so nothing is known. See CONTEXT.md → Unknown.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FlagState { Set, Clear, Unknown }
```

It replaces, at five sites:

| Today | File | Unknown becomes |
|---|---|---|
| `(bool, VerificationStatus)` | `ui/events.rs:744` | `false` + a status the caller may drop |
| `(bool, bool)` | `ui/events.rs:1286` | `false` + a flag the caller may drop |
| `GraceStatus` | `vm/events.rs:302` | `Unreliable`, then `false` at `is_discovered()` |
| `.unwrap_or(false)` | `main.rs:284` | `false`, deliberately and with a comment |
| `HashSet<u32>` membership | `ui/events.rs:1544` | absence, indistinguishable from clear |
| `Option<bool>` kept intact | `ui/world_pickups_view.rs:17` | — (the one correct one) |

The rule that makes it stick: **no `bool` between a reader and a glyph.** `FlagState`
deliberately has no `fn is_set(&self) -> bool`, because that method is exactly how the
discipline gets lost — `GraceStatus::is_discovered()` at `vm/events.rs:90` is that method,
and it returns `false` for `Unreliable`. Narrowing to `bool` requires naming the choice:

```rust
impl FlagState {
    /// Treat Unknown as clear. Every call site is a place a distinction is
    /// being deliberately discarded; there should be very few.
    pub fn unknown_as_clear(self) -> bool { matches!(self, FlagState::Set) }
}
```

`main.rs:284` is a legitimate caller (region filtering). `ui/events.rs:1721` is not — it
renders Unknown as "not collected" twenty lines below a doc comment at `:742` promising it
never does. That is the live defect this workstream closes, and it is the same class of
failure as `batch-validate` reporting 0/110 boss defeats on a finished character.

### B2 — `ResolvedFlags`

```rust
/// Every flag family's base for ONE save's flag region, resolved once.
///
/// Construction is where refusal happens: no Origin, no ResolvedFlags. A value of
/// this type is a promise that the Origin was found — not that any given flag can
/// be read, which is why the methods still return FlagState.
pub struct ResolvedFlags<'a> { /* ef, origin, five family bases */ }

impl<'a> ResolvedFlags<'a> {
    pub fn from_event_flags(ef: &'a [u8]) -> Option<Self>;

    pub fn origin(&self) -> usize;
    pub fn family_base(&self, family: Family) -> Option<usize>;

    pub fn world_state(&self, flag_id: u32) -> FlagState;
    pub fn tile_world(&self, flag_id: u32) -> FlagState;
    pub fn tile_pickup(&self, id: u32) -> FlagState;
    pub fn dungeon(&self, flag_id: u32) -> FlagState;
    pub fn dungeon_pickup(&self, flag_id: u32) -> FlagState;
}
```

Two-stage refusal, and both stages are load-bearing:

- **`from_event_flags` returns `None`** when the Origin will not resolve — one decision,
  once per save, instead of the same decision re-made per flag.
- **methods still return `FlagState::Unknown`** when the *flag* has no known position in a
  resolved save: DLC tiles (`2xxxxxxxxx`), the ~935 six-digit ids in `WORLD_PICKUPS` that
  belong to no family, doubly-allocated maps, out-of-family ids.

Depth: `find_flag_list_end_in_ef` scans from EF+16,000 forward to the first 64-byte zero
run — roughly 13,400 bytes — and it runs once per flag read today. `ui/events.rs:842`
does that inside a `filter_map` over 4,809 rows, `world_pickups_view.rs:219` over 2,867,
`graces_view` over 421. `comparison_view.rs:421` is the only call site that noticed:
`.take(100) // Sample first 100 pickups to avoid performance issues`. One construction per
view per frame makes that one scan.

### The ADR-0008 constraint, and a gap it has

`ResolvedFlags` is **consistent with ADR-0008 and must stay pure Rust to remain so.**

- It never takes a bare `flag_id` and returns an offset. It takes the flag bytes and
  resolves that save's families — the ADR's own prescribed shape, stated once in a
  constructor instead of ten times in ten function bodies.
- The five `#[wasm_bindgen]` `*_state` exports **stay exactly as they are**, reimplemented
  as adapters:

  ```rust
  #[wasm_bindgen]
  pub fn world_state_flag_state(event_flags: &[u8], flag_id: u32) -> i32 {
      ResolvedFlags::from_event_flags(event_flags)
          .map_or(-1, |r| r.world_state(flag_id).into())
  }
  ```

  The `APPROVED_EXPORTS` manifest, all four `export_shape_conformance` tests, and every
  `origin_conformance` and `anchor_conformance` assertion are untouched. No ADR needs
  amending. That is the point of keeping the new type off the wasm surface.

> **Gap worth recording either way.** `export_shape_conformance.rs`'s `actual_exports`
> only inspects `pub fn` following a `#[wasm_bindgen]` attribute, and its own comment says
> *"structs/impls are not exports we gate here."* So if `ResolvedFlags` were ever exported
> as a `#[wasm_bindgen]` struct, its methods would answer flag-state questions **without
> appearing in the manifest and without tripping the structural check** — the gate would
> silently stop covering the primary reader. That is an argument for the pure-Rust
> decision above; if the decision is ever reversed, the extractor must learn `impl` blocks
> *first*.

### What does NOT move into the crate

`db::pickup_flags::pickup_flag_state` routes an id to a family by numeric range. It looks
like crate material and is not: `WORLD_PICKUPS` is a five-family table (1,232 open-world,
2,010 legacy-map, 100 world-state-b, 935 unclassified, 532 DLC per `pickup_flags.rs:788`),
and the router encodes *which families that particular table mixes* — app knowledge about
a database, not save-format knowledge. Routing the whole table through the tile reader is
what shipped in v0.28.0 and left 3,577 entries Unknown.

It stays in `db/`, re-expressed against the new type:

```rust
pub fn pickup_state(flags: &ResolvedFlags, flag_id: u32) -> FlagState
```

### Open questions

1. ~~**Lifetime vs. ownership.**~~ **ANSWERED 2026-07-22 — borrowing is free.**
   `get_event_flags(&self, index) -> Option<&[u8]>` (`save/save.rs:64`) borrows
   `self.save`, not `self.vm`, so there is no conflict with `&mut ViewModel` — and the
   codebase already threads both separately: `fn world_pickups(ui, vm: &mut ViewModel,
   event_flags: Option<&[u8]>, …)`. `ResolvedFlags<'a>` costs nothing at the borrow
   checker. This was the one thing that could have made D urgent; it does not.
2. **Per-frame construction, or cached?** One scan per view per frame is a ~5,000×
   improvement and needs no invalidation. Caching on the slot needs invalidation on slot
   switch and on file open. Start uncached; revisit only if measured.
3. **Does `GraceStatus` survive?** It carries `Unreliable` plus grouping the graces screen
   uses. It may reduce to `FlagState` outright, or keep its extra states and hold a
   `FlagState` inside.
4. **Should `FlagState` live in the wasm crate or the app?** The crate is where the
   readers are; the app is where the five collapses are. Putting it in the crate means
   elden-map inherits it, which is either leverage or a coordinated release.

---

## Workstream C — `Evidence` and `Claims`

**Deepening**: "catalog entry → verified bytes", currently hand-rolled six times with two
divergences, becomes one module whose interface cannot return unverified bytes. Claims
emission, currently three formats and three provenance shapes, becomes one emitter.

### C1 — `Evidence`

```rust
/// The verified read side of the Evidence Catalog (ADR-0001). The only way to
/// obtain evidence bytes. There is deliberately no method that skips the hash.
pub struct Evidence { /* roots, corpora, manifests, memo */ }

impl Evidence {
    pub fn open(repo_root: &Path) -> Result<Self, String>;

    pub fn bytes(&self, corpus: &str, rel: &str) -> Result<Arc<[u8]>, String>;
    pub fn json(&self, corpus: &str, rel: &str) -> Result<Arc<Value>, String>;
    pub fn slot(&self, corpus: &str, rel: &str, slot: usize) -> Result<Arc<[u8]>, String>;
    pub fn sha256(&self, corpus: &str, rel: &str) -> Result<&str, String>;
    pub fn list(&self, corpus: &str) -> Result<Vec<&str>, String>;
}
```

Behind it, once each instead of N times:

| Absorbed | Copies today |
|---|---|
| find corpus in catalog, join `roots[root]` + `path` | 6 — `pipeline.rs:428`, `family_distances.rs:254/1027/1330/1466`, `timeline.rs:148`, `catalog.rs:67` |
| load `knowledge/manifests/*.sha256` | 4 |
| `"EVIDENCE DRIFT {}: sha256 {} != cataloged {}"` | 4 verbatim — `pipeline.rs:281`, `timeline.rs:102`, `timeline.rs:195`, `family_distances.rs:1129` |
| slot slice `HEADER + n*(CHECKSUM+SLOT_SIZE) + CHECKSUM` | 3 — `pipeline.rs:286`, `dump.rs:37`, `family_distances.rs:1138` |
| the `BTreeMap` memo cache | 4 re-declarations |

Two of the copies have **already drifted**, which is the argument for the seam:

- `family_distances.rs:1330` — a missing corpus `continue`s instead of erroring, so an
  absent corpus reads as an empty one.
- `gen_dungeon_pickups.rs:259` — `source_from_catalog` reads `roots.decompiled` directly,
  hard-codes `"regulation-bin/ItemLotParam_map.param.xml"`, and performs **no sha256 check
  at all**. That is the primary source both `world_pickups.rs` and `dungeon_pickups.rs` are
  generated from. The generator round-trip tests pin table-equals-generator-output; nothing
  pins generator-input-equals-cataloged-bytes.

The deletion test: delete `Evidence` and drift detection reappears in six places, one of
which already omits it. It concentrates.

### C2 — `Claims`

```rust
/// The Status Ladder (CONTEXT.md), as a type rather than a string literal.
pub enum Status { Hypothesis, Corroborated, Verified, Tombstoned, Skipped }

/// Emitter for every file under knowledge/claims/. Owns the serialisation format,
/// so ADR-0004's byte-for-byte regeneration is a property of one module.
pub struct Claims { /* schema, inputs, notes, entries */ }

impl Claims {
    pub fn new(schema: &str) -> Self;
    pub fn input(self, ev: &Evidence, path: &str) -> Result<Self, String>;  // records sha256
    pub fn note(self, text: &str) -> Self;
    pub fn claim(&mut self, status: Status, body: Value);
    pub fn write(self, repo_root: &Path, filename: &str) -> Result<WriteReport, String>;
}
```

What it makes impossible: a claims file with no provenance. Today three of the five
writers emit exactly that.

| Writer | Serialisation | Trailing `\n` | Input hashes | Status field |
|---|---|---|---|---|
| `pipeline.rs:751` | `to_string_pretty` | yes | yes | yes |
| `family_distances.rs:643` | `format!("{:#}")` | yes | **no** | **no** |
| `timeline.rs:304` | `to_string_pretty` | **no** | **no** | **no** |
| `timeline_segments.rs:413` | `to_string_pretty` | **no** | **no** | **no** |
| `timeline_flips.rs:292` | `to_string_pretty` | **no** | **no** | **no** |

The ladder is string literals typed per site — `json!("verified")` at `pipeline.rs:1148`,
`json!("hypothesis")` at `:1161`, `json!("skipped")` at `:814` — and read back by string
comparison in a *different* module at `family_distances.rs:1444`. Nothing connects producer
to consumer. `Status` does, at compile time.

> **This reformats four committed claims files.** Trailing newlines and added provenance
> change their bytes. ADR-0004 forbids hand-editing them, so the reformat is performed by
> **running the commands**, and the diff is reviewed as generated output. Do it as its own
> commit, separate from the code change, so the byte diff is readable. Confirm first that
> nothing pins those files' current digests — `tests/regression_suite.rs:39` freezes
> `ground_truth_offsets.json`, which is a different file, but check.

### C3 — what is left of `family_distances.rs`

1,599 lines exporting five unrelated `pub fn cmd_*(_args)` entry points that share only
`read_json`, `write_json`, `method_block`, and the catalog loader. Once `Evidence` and
`Claims` own those, the file has no reason to be one file. It splits along the seams it
already has:

- `origin_model.rs` — `search_base`, `predict_base`, `scan_list_end`, `aligned_at`,
  `shift_at`, `narrow`, `ORIGIN_CONSTANTS`. The actual subject matter, and the only part
  with algorithmic content worth testing directly.
- five thin `cmd_*` modules, each an adapter from argv to `origin_model` + `Claims`.

Also folded in: `timeline_flips.rs:49`'s `isolated_flips`, copied verbatim from
`pipeline.rs:313` and labelled as such at `timeline_flips.rs:268`.

### Open questions

1. **`&self` + memo needs interior mutability.** `Arc<[u8]>` returns with a
   `RefCell<HashMap<…>>` inside, or `&mut self` and thread the mutable borrow through?
   The former is nicer to call and slightly more machinery.
   — **RESOLVED v0.37.10: `&self` + `RefCell`.** The five `cmd_*` readers hold one `Evidence`
   and read many files without threading a mutable borrow; the byte cache and manifest cache
   both sit behind `RefCell`.
2. **Does `Evidence` verify eagerly or lazily?** `catalog-verify` wants every corpus;
   `grace-dump` wants one file. Lazy-per-file with a memo serves both; eager makes
   `catalog-verify` fall out for free.
   — **RESOLVED v0.37.10: lazy-per-file with a memo.** `open` reads no evidence; each `bytes`
   call verifies and caches. `catalog-verify` stays in `catalog.rs` (the integrity authority
   `Evidence` trusts), so it did not need eager verification folded in.
3. **`grace-dump` reads a user-supplied save, not evidence.** It should stay outside
   `Evidence` — but it should still not carry its own slot-slice arithmetic. Where does the
   slot slicer live: `Evidence`, the wasm crate, or a third place both use?
   — **RESOLVED v0.37.10: a free `slot_slice` fn in `evidence.rs`.** Not the wasm crate (flag
   geometry, not container structure) and not `save/` (the typed parser). `grace-dump` calls
   it on its user bytes; `Evidence` and the pipeline call it on verified bytes.
4. **Does `Claims` need to read as well as write?** `family_distances.rs:1444` reads
   `event-flags.json` back. A read side makes the schema symmetric and the consumer typed;
   it also doubles the module.
   — **DECIDED (for C2): write-only.** The one tombstone read-back stays as-is; add a read
   side only if a second consumer appears. (C2 not yet implemented.)

---

## Workstream D — `Character` and `ScreenState` (designed, deferred)

**Deepening**: one mutable struct holding both the reconstructed character and every
screen's widget state becomes two modules with a seam between them.

Designed here so the cost is visible. **Going ahead is a separate decision, to be taken
after A lands** — the whole benefit is "the reconstruction becomes assertable", and until
A there is nothing to assert it from.

### The problem

`ViewModel` (`vm/vm.rs:42`) holds `slots: [SlotViewModel; 10]` inline. Below it,
`EventsViewModel` (`vm/events.rs:219`) has 21 fields: nine data maps and twelve of pure
egui widget state (`world_pickups_filter`, `dungeon_pickups_filter`, eight
`*_view_state` structs, `verification_vm`). `InventoryViewModel` (`vm/inventory/mod.rs:327`)
has 24, including `log: Vec<String>` and `changed`. Fourteen view functions take
`&mut ViewModel` and write filter text into it mid-render.

Two consequences:

- **Nothing can assert on the reconstruction without a frame.** The read model has no
  interface separate from the widget state that surrounds it. `vm/export.rs:332` is the
  single vm test in the tree and it works by going through the export path.
- **Two sources of flag bytes.** The vm reads `slot.event_flags.flags`
  (`vm/events.rs:303`); every view reads `save.save_type.get_event_flags(vm.index)`, at
  seven call sites in `main.rs`. They agree today. Nothing makes them.

### The change

```rust
/// A character reconstructed from one slot, the way the game loads one. Immutable,
/// no egui, no filters. Holds the slot's flag region and one ResolvedFlags over it —
/// so "which bytes" stops being a question each view answers for itself.
pub struct Character<'a> { /* stats, inventory, equipment, flags: ResolvedFlags<'a>, … */ }

impl<'a> Character<'a> {
    pub fn from_slot(slot: &'a SaveSlot) -> Self;
    pub fn flags(&self) -> &ResolvedFlags<'a>;
}

/// Per-screen mutable widget state for the active slot. Filters, sorts, selection,
/// navigation. Nothing reconstructed lives here.
pub struct ScreenState { /* the twelve fields that are currently in EventsViewModel */ }
```

View functions become `fn events(ui: &mut Ui, ch: &Character, ss: &mut ScreenState)` — one
`&mut`, and it is the one that should be mutable.

Wins: the read model gets a test surface; one source of flag bytes; slot switch stops
carrying ten copies of widget state for slots nobody is looking at.

Cost: fourteen view signatures, and every `vm.slots[vm.index].events_vm.…` path in `ui/`.
This is the large one. It should not start until A has landed and B has settled, because
`Character` holding a `ResolvedFlags` is B's lifetime question (open question B.1) in its
most demanding form.

### Open questions

1. **Does `Character` borrow or own?** Borrowing the slot makes it cheap and makes the
   lifetime question acute. Owning means copying parsed state and re-answering "when is it
   rebuilt".
2. **Where does `ScreenState` live — per slot, or one for the active slot?** Ten copies is
   today's behaviour; one is the honest amount. Switching slots would then reset filters,
   which is a UX change, not just a refactor.
3. **Is `verification_vm` read model or screen state?** It is lazily loaded per slot at
   `main.rs:248` behind a `verification_loaded_slots: [bool; 10]` guard, so it is neither
   cleanly.

---

## Suggested sequence

| # | Work | Blocked by | Size |
|---|---|---|---|
| 0 | ✅ **DONE** v0.37.4 — deleted `calibration.rs`, amended the BACKLOG 4b note | — | small |
| 1 | ✅ **DONE** v0.37.5 — `[lib]` + `pub(crate)` + the Unknown-preservation test | — | small, mechanical |
| 1b | ✅ **DONE** v0.37.6 — lifted `vm`/`save/common`/`util` allows, triaged the 21 | 1 | small |
| 2 | ✅ **DONE** v0.37.7 — B2: `ResolvedFlags` + `FlagState` in the crate; five exports become adapters | — (parallel with 1) | medium; conformance suites unchanged |
| 3 | ✅ **DONE** v0.37.8 — B1: migrated every reader to `FlagState`; deleted `GraceStatus`; fixed `ui/events.rs` detail panel + two `comparison_view` defects | 1, 2 | medium; touched every view that reads a flag |
| 3b | ✅ **DONE** v0.37.9 — B3: deleted the five deprecated free readers; re-expressed `origin_conformance` against the `*_state` exports and rewrote `resolved_flags_conformance` (exact-bit round-trip replaces old-vs-new comparison); relocated the overlap-band note to `ResolvedFlags::dungeon_pickup` | 3 | small |
| 4 | ✅ **DONE** v0.37.10 — C1: `Evidence` seam (`bytes`/`sha256`/`slot_slice`/`read_verified`); migrated all six hand-rolled loaders + the file-corpus loop + `dump`; fixed both drifted sites (missing-corpus silent `continue` → hard error; `gen_*` primary source now verified against the manifest). Behaviour-preserving — every knowledge command byte-identical | 1 | medium |
| 5 | C2 — `Claims` + `Status`; regenerate the four under-provenanced files | 4 | medium; one generated-output commit |
| 6 | C3 — split `family_distances.rs` | 4, 5 | small once 4 and 5 land |
| 7 | D — decide, then possibly do | 1, 3 | large |

Steps 3 and 5 each close a live defect (`events.rs:1721` renders Unknown as clear; three
claims writers emit no provenance). Steps 1, 2, 4, 6 change no behaviour.

## Next step

Nothing here is decided. Per workstream, the next move is to grill the open questions —
they are the points where an implementation would otherwise pick a default silently. Terms
that survive grilling (`FlagState`, `ResolvedFlags`, `Evidence` as a module rather than a
concept, `Character`, `ScreenState`) go into `CONTEXT.md` at that point, not before;
workstreams rejected with a load-bearing reason get an ADR so the next review does not
re-suggest them.
