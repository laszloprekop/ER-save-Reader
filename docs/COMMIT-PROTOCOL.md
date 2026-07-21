# Commit Snapshot Protocol

This document defines the standardized process for creating production-ready commit snapshots for ER-save-Editor.

> **Epistemic header** (audited 2026-07-20 · BACKLOG step 6)
> **Status: CURRENT process doc.** The commit/snapshot procedure (invoked via `/snapshot`). Workflow, not evidence — keep in step with the `snapshot` skill and `CLAUDE.md`'s commit protocol.

## When to Create a Snapshot

Trigger the snapshot routine when:

- **New Feature Complete**: Feature is functional and ready for production
- **Database Expansion**: New event flags, items, pickups, NPCs, or other game data added
- **Bug Fix Complete**: Issue is resolved and verified
- **Save Format Change**: Changes to save file parsing or writing
- **UI Enhancement**: Significant UI improvements or new views
- **Performance Optimization**: Measurable performance improvements

**Do NOT snapshot:**
- Work in progress / incomplete features
- Experimental changes not yet validated
- Temporary debugging code
- Minor typo fixes (unless part of larger change)

## Snapshot Process Checklist

### Phase 1: Validation

- [ ] **Build succeeds**: `cargo build` completes without errors
- [ ] **Check passes**: `cargo check` reports no errors
- [ ] **Clippy clean**: `cargo clippy` passes (or acceptable warnings documented)
- [ ] **Format check**: `cargo fmt --check` (optional, run `cargo fmt` to fix)
- [ ] **Manual testing**: Load a save file, verify changes work as expected
- [ ] **No data corruption**: Test that saves can be exported/loaded correctly

### Phase 2: Documentation Updates

#### 2.1 Identify Documentation Type

Categorize the changes to determine which docs need updates:

**Save Slot Feature Registry** → `save_slot_registry.json`
- Save format discoveries, new storage locations found
- Confidence changes from new verification data
- Feature reclassification (unknown → identified → verified)
- New features identified that weren't previously tracked

**Architecture Changes** → `docs/ARCHITECTURE.md` (create if needed)
- New major patterns or approaches
- Save format discoveries
- Performance optimization strategies
- Component architecture redesigns

**Database Coverage** → `docs/DATABASE_COVERAGE_ANALYSIS.md`
- New event flags added
- Item/weapon/armor database expansions
- Coverage percentage improvements

**Implementation Plans** → `docs/BACKLOG.md`
- Planned features and their status
- Database expansion roadmaps

**Debugging Insights** → `docs/DEBUGGING-INSIGHTS.md` (create if needed)
- Tricky bugs that took significant time to solve
- Save file format gotchas
- Event flag calculation issues
- Non-obvious solutions that should be remembered

**General Guidance** → `CLAUDE.md`
- **ONLY** if it changes "how to work with this codebase RIGHT NOW"
- New essential patterns ALL developers must know immediately
- Changes to project structure, commands, or critical workflows
- **NOT** for feature details, bug fixes, or version-specific notes

**Data Schemas** → `DATA-SCHEMAS.md`
- Save file format documentation
- Event flag structures
- Item lot parameter mappings

#### 2.2 Documentation Update Guidelines

**For each relevant doc file:**

1. **Use clear structure**:
   - **Problem/Context**: What was the issue or goal?
   - **Solution**: How was it solved?
   - **Implementation Details**: Key technical decisions
   - **Code Examples**: Show patterns, not full implementations

2. **Include file references**:
   - Use format: `src/db/pickup_flags.rs:123`
   - Makes it easy to navigate to relevant code

3. **Add cross-references**:
   - Link to related docs: `See [DATA-SCHEMAS.md](./DATA-SCHEMAS.md#section)`

#### 2.3 Bump Cargo.toml Version

After determining the appropriate version number, update `Cargo.toml`:

1. **Edit Cargo.toml**: Change `version = "0.0.X"` to the new version
2. **Use semantic versioning**:
   - **Major (X.0.0)**: Breaking changes, major save format changes
   - **Minor (0.X.0)**: New features, significant enhancements
   - **Patch (0.0.X)**: Bug fixes, minor improvements, database expansions

**Example:**
```toml
[package]
name = "er-save-editor"
version = "0.0.26"  # Changed from 0.0.25
```

### Phase 3: Conventional Commit

#### 3.1 Commit Type Prefixes

Use conventional commit format: `type(scope): description`

**Common types:**
- `feat`: New feature for the user
- `fix`: Bug fix for the user
- `db`: Database expansion (event flags, items, pickups, etc.)
- `refactor`: Code change that neither fixes bug nor adds feature
- `perf`: Performance improvement
- `docs`: Documentation only changes
- `chore`: Updating dependencies, build config, etc.

**Scope examples:**
- `feat(ui)`: UI-related feature
- `feat(export)`: Export functionality
- `fix(events)`: Event flag bug fix
- `db(pickups)`: World pickup database expansion
- `db(spells)`: Spell database addition
- `perf(parsing)`: Save file parsing performance

#### 3.2 Commit Message Structure

```
type(scope): short description (50 chars max)

Detailed explanation of what changed and why (wrap at 72 chars).

Implementation details:
- Key technical decision 1
- Key technical decision 2

Files modified:
- src/db/pickup_data.rs: added 500 new pickups
- src/vm/slot.rs: updated export to use new data
- Cargo.toml: bumped to 0.0.26
```

#### 3.3 Commit Message Guidelines

**DO:**
- Use imperative mood: "Add feature" not "Added feature"
- Start with lowercase after type: `feat: add spell database`
- Be specific: "Fix world pickup false negatives" not "Fix bug"
- Include context in body for non-trivial changes
- List key files modified in body

**DON'T:**
- Use vague descriptions: "Fix stuff", "Update things"
- Write essays in subject line
- Forget to stage documentation updates
- Commit broken builds

### Phase 4: Git Operations

1. **Stage changes**: `git add -A` or selectively add files
2. **Verify staging**: `git status --short` to review
3. **Preview diff**: `git diff --staged` for final check (optional)
4. **Commit with message**: Follow conventional commit format
5. **Verify commit**: `git log --oneline -1` to confirm

### Phase 5: Post-Commit

- [ ] **Push to remote** (if ready): `git push`
- [ ] **Verify build**: Ensure CI passes if configured
- [ ] **Tag release** (if appropriate): `git tag v0.0.26 && git push --tags`

## Examples

### Example 1: Database Expansion Snapshot

**Trigger**: Added accurate world pickup data with formula-based flag checking

**Validation:**
```bash
cargo build      # Success
cargo clippy     # Clean (or acceptable warnings)
```

**Documentation:**
- Update `docs/DATABASE_COVERAGE_ANALYSIS.md`: Note coverage improvement
- Update `CLAUDE.md`: Only if new essential pattern for all developers

**Commit:**
```
db(pickups): add accurate world pickup data with formula-based flags

Ported pickup_flags.rs and pickup_data.rs from elden-map project for
accurate event flag checking.

Implementation details:
- Formula-based offset calculation for tile/dungeon flags
- 4378 pickups with correct 8-digit/10-digit flag IDs
- Bit calculation: bit = 7 - (flag_id % 8)

Results:
- Before: 83/5388 collected (false negatives)
- After: 544/4378 collected (matches gameplay progression)

Files modified:
- src/db/pickup_flags.rs: formula-based flag offset calculation
- src/db/pickup_data.rs: 4378 world pickups with correct flags
- src/db/mod.rs: added new module declarations
- src/ui/events.rs: updated to use new pickup data
- src/vm/slot.rs: updated export to use new pickup data
- Cargo.toml: bumped to 0.0.26
```

### Example 2: Bug Fix Snapshot

**Trigger**: Fixed deadlock in event flags display

**Validation:**
```bash
cargo build      # Success
cargo clippy     # Clean
```

**Commit:**
```
fix(events): prevent deadlock when displaying world pickups

EVENT_FLAGS mutex was locked twice - once in the counting loop and
again in is_pickup_collected call.

Solution:
- Release lock before calling is_pickup_collected
- Wrapped counting loop in block scope to drop lock early

Files modified:
- src/ui/events.rs: fixed mutex scope
```

### Example 3: New Feature Snapshot

**Trigger**: Added filtering to World Pickups view

**Validation:**
```bash
cargo build      # Success
cargo clippy     # Clean
```

**Documentation:**
- Update `docs/BACKLOG.md`: Mark feature as complete

**Commit:**
```
feat(ui): add filtering to world pickups view

Added type, collection status, region, and search filters to the
World Pickups screen matching the database view functionality.

Implementation details:
- PickupTypeFilter enum with 12 categories
- CollectedFilter enum (All, Collected, NotCollected)
- Region dropdown with all game regions
- Text search by item name

Files modified:
- src/vm/events.rs: added filter state structs
- src/ui/events.rs: implemented filter UI and logic
- Cargo.toml: bumped to 0.0.27
```

## Project-Specific Scopes

Common scopes for this project:

| Scope | Description |
|-------|-------------|
| `ui` | User interface changes |
| `db` | Database modules (items, events, pickups, etc.) |
| `save` | Save file parsing/writing |
| `export` | JSON/character export functionality |
| `events` | Event flags, graces, bosses, etc. |
| `inventory` | Inventory management |
| `stats` | Character stats |
| `equipment` | Equipped items |
| `vm` | ViewModel layer changes |

## Quick Reference: When to Update Which Doc

| Change Type | CLAUDE.md | DATABASE_COVERAGE | IMPLEMENTATION_PLAN | DATA-SCHEMAS | Registry |
|-------------|-----------|-------------------|---------------------|--------------|----------|
| New DB module | Maybe | Yes | Yes | Maybe | Maybe |
| Event flag expansion | No | Yes | Yes | Maybe | Yes |
| Save format discovery | Maybe | No | No | Yes | Yes |
| New feature | No | No | Yes | No | Maybe |
| Bug fix | No | No | No | No | No |
| Architecture change | Yes | No | Maybe | Maybe | No |

## Common Pitfalls to Avoid

1. **Not running `cargo build` before commit** - Always validate build succeeds
2. **Forgetting to bump Cargo.toml version** - Update for any meaningful change
3. **Vague commit messages** - Be specific about what changed and why
4. **Committing too many unrelated changes** - Keep snapshots focused
5. **Not testing with actual save files** - Always verify with real data
6. **Missing documentation updates** - Document before you forget the details
