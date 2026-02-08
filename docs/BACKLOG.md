# Project Backlog

**Last updated**: 2026-02-08

This is the single location for all planned work, remaining gaps, and deferred items. Organized by priority.

---

## Priority 1: Data Coverage Gaps

### Gesture Database
- **Source**: GestureParam (~60 rows)
- **Status**: NOT STARTED
- **Impact**: Cannot display/edit unlocked gestures
- **Effort**: Low (simple enum + flag mapping)

### Full NPC Database
- **Source**: NpcParam (~500 entries)
- **Status**: Only 30 of ~500 NPCs tracked (`npcs.rs`)
- **Impact**: Cannot track most NPC encounters/questlines
- **Effort**: Medium (need to map NPC IDs to names and event flags)

---

## Priority 2: Event Flag Verification

### Boss Flag Verification Improvement
- **Current**: Great Boss 9.6%, Field Boss 4.3%, Generic Boss 13.8% verified
- **Needed**: Create test saves with specific bosses defeated for differential analysis
- **Blocked by**: Need gameplay progression in test characters

### Unreliable Block Bases
- **Blocks**: 71000, 71100, 71600, 73000
- **Issue**: Base offsets vary by save progression (not stable across saves)
- **Solution**: Dynamic calibration per save file, or discover stable alternative bases

### Unverified Dungeon Areas
- **Areas**: 20, 21 (unverified), plus 13, 15, 16, 18, 19, 34, 35, 39 (calculated but not empirically verified)
- **Method**: Multi-slot differential with appropriate test characters

### Disproven Block Bases
- **Blocks**: 75000, 77000 (0xFF padding, not real data)
- **Action**: Discover actual bases or confirm these ranges are unused

---

## Priority 3: Cross-Project Sync

### Elden Map Missing Block Bases
- **Issue**: Elden Map viewer (`eventFlagService.ts`) is missing 6 block bases that Save Editor has
- **Missing blocks**: 61000 (map visits), 62000 (map fragments), 65000 (Crystal Tears), 72000 (DLC graces), 74000 (DLC dungeon graces), 78000 (grace guidance)
- **Action**: Sync BLOCK_BASES from ground_truth_offsets.json to Elden Map

---

## Priority 4: Code Quality

### Module Consolidation (Optional)
Several data categories have parallel modules (see [DATABASE_COVERAGE_ANALYSIS.md](DATABASE_COVERAGE_ANALYSIS.md#code-redundancy-notes)):
- `world_pickups.rs` / `pickup_data.rs` (overlapping pickup data)
- `graces.rs` / `graces_data.rs` (enum + enriched split)
- `bosses.rs` / `bosses_data.rs` (enum + enriched split)
- `shop_items.rs` / `merchants_data.rs` (different grouping)

These work correctly as-is but could be consolidated to reduce maintenance burden.

---

## Priority 5: Infrastructure (Deferred)

### CI Integration for Verification
- **Concept**: Automated regression testing of flag formulas against test saves
- **Status**: NOT STARTED
- **Source**: archive/VERIFICATION_STRATEGY.md

### EvidenceDiscoveryService in Rust
- **Concept**: Rust-native version of the Python evidence discovery workflow
- **Status**: NOT STARTED
- **Source**: archive/EVIDENCE-BASED-DISCOVERY.md
- **Rationale**: Deferred - Python scripts work well enough for now

---

## Completed (for reference)

Items from previous "Next Steps" that have been done:

| Item | Completed In | Version |
|------|-------------|---------|
| Spell database | spells.rs (315 entries) | v0.8.0 |
| NPC tracking (partial) | npcs.rs (30 entries) | v0.10.0 |
| Shop stock tracking | shop_items.rs + merchants_data.rs | v0.9.0 |
| World pickup browser | world_pickups.rs + UI | v0.7.0 |
| Dungeon pickup section bases | 89 bases verified | v0.12.0 |
| Landmark database | landmarks.rs (308 entries) | v0.11.0 |
| Entity relationships | entity_relationships_data.rs (613) | v0.13.0 |
| Quest chains | quest_chains.rs (24 entries) | v0.12.0 |
| Row ID formula discovery | Consumable tracking enabled | v0.12.0 |
| Dungeon pickup per-section bases | 89 sections across 22 areas | v0.12.0 |
