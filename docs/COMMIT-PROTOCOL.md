# Commit Snapshot Protocol

This document defines the standardized process for creating production-ready commit snapshots.

## When to Create a Snapshot

Trigger the snapshot routine when:

- ✅ **New Feature Complete**: Feature is functional and ready for production
- ✅ **Feature Refinement Done**: Improvements to existing feature are complete
- ✅ **Bug Fix Complete**: Issue is resolved and verified
- ✅ **Architecture Change**: Significant refactoring or pattern changes
- ✅ **Performance Optimization**: Measurable performance improvements
- ✅ **Documentation Update**: Major docs reorganization or additions

**Do NOT snapshot:**
- ❌ Work in progress / incomplete features
- ❌ Experimental changes not yet validated
- ❌ Temporary debugging code
- ❌ Minor typo fixes (unless part of larger change)

## Snapshot Process Checklist

### Phase 1: Validation

- [ ] **Run production build**: `pnpm build` completes without errors
- [ ] **Run type check**: `pnpm typecheck` passes
- [ ] **Run linter**: `pnpm lint` passes (or acceptable warnings documented)
- [ ] **Manual testing**: Feature works as expected in dev environment
- [ ] **Server running**: Background server operational if backend changes involved

### Phase 2: Documentation Updates

#### 2.1 Identify Documentation Type

Categorize the changes to determine which docs need updates:

**Architecture Changes** → `docs/ARCHITECTURE.md`
- New major patterns or approaches
- Migration from one technology to another
- Performance optimization strategies
- Component architecture redesigns

**Debugging Insights** → `docs/DEBUGGING-INSIGHTS.md`
- Tricky bugs that took significant time to solve
- State management gotchas
- Closure traps, race conditions
- Non-obvious solutions that should be remembered

**Deployment Issues** → `docs/DEPLOYMENT.md`
- Production environment issues
- Docker, proxy, or SSL configuration
- Environment variable handling
- OAuth or authentication setup

**Feature-Specific** → `docs/FEATURE-*.md` (create if needed)
- Complex features needing dedicated documentation
- Configuration guides
- User-facing feature documentation
- API documentation

**General Guidance** → `CLAUDE.md`
- **ONLY** if it changes "how to work with this codebase RIGHT NOW"
- New essential patterns ALL developers must know immediately
- Changes to project structure, commands, or critical workflows
- **NOT** for feature details, bug fixes, or version-specific notes

#### 2.2 Documentation Update Guidelines

**For each relevant doc file:**

1. **Add version header** (if applicable):
   ```markdown
   ## Feature Name (vX.Y.Z)
   ```

2. **Use clear structure**:
   - **Problem/Context**: What was the issue or goal?
   - **Solution**: How was it solved?
   - **Implementation Details**: Key technical decisions
   - **Code Examples**: Show patterns, not full implementations
   - **Lessons Learned**: What would you do differently?

3. **Include file references**:
   - Use format: `src/components/ComponentName.tsx:123`
   - Makes it easy to navigate to relevant code

4. **Add cross-references**:
   - Link to related docs: `See [ARCHITECTURE.md](./ARCHITECTURE.md#section)`
   - Reference external resources when helpful

#### 2.3 Always Update CHANGELOG.md

Add entry to `docs/CHANGELOG.md`:

```markdown
### vX.Y.Z - Brief Description

- Main feature/fix description
- Secondary changes if applicable
- See [ARCHITECTURE.md](./ARCHITECTURE.md#section) for implementation details
```

**Version numbering:**
- **Major (X.0.0)**: Breaking changes, major architecture shifts
- **Minor (0.X.0)**: New features, significant enhancements
- **Patch (0.0.X)**: Bug fixes, minor improvements

#### 2.4 Bump package.json Version

After determining the appropriate version number, update `package.json`:

1. **Edit package.json**: Change `"version": "0.X.Y"` to the new version
2. **Use semantic versioning**: Follow the same numbering as CHANGELOG.md entry
3. **Commit together**: package.json version bump should be included in the same commit

**Example:**
```json
{
  "name": "eldenmap",
  "version": "0.21.0",  // Changed from 0.20.0
  ...
}
```

**Consistency check:**
- CHANGELOG.md entry: `### v0.21.0 - Feature Name`
- package.json: `"version": "0.21.0"`
- These MUST match

### Phase 3: Conventional Commit

#### 3.1 Commit Type Prefixes

Use conventional commit format: `type(scope): description`

**Common types:**
- `feat`: New feature for the user
- `fix`: Bug fix for the user
- `refactor`: Code change that neither fixes bug nor adds feature
- `perf`: Performance improvement
- `docs`: Documentation only changes
- `style`: Formatting, missing semicolons, etc (not CSS)
- `test`: Adding or correcting tests
- `chore`: Updating build tasks, dependencies, etc

**Scope examples:**
- `feat(map)`: Map-related feature
- `fix(auth)`: Authentication bug fix
- `perf(clustering)`: Clustering performance improvement
- `docs(protocol)`: Documentation protocol changes

#### 3.2 Commit Message Structure

```
type(scope): short description (50 chars max)

Detailed explanation of what changed and why (optional, wrap at 72 chars).

Implementation details:
- Key technical decision 1
- Key technical decision 2

Breaking changes (if any):
- What broke and how to migrate

Files modified:
- src/path/to/file.ts: what changed
- docs/ARCHITECTURE.md: added section on X

Closes #123 (if applicable)
```

#### 3.3 Commit Message Guidelines

**DO:**
- ✅ Use imperative mood: "Add feature" not "Added feature"
- ✅ Start with lowercase after type: `feat: add clustering`
- ✅ Be specific: "Fix marker flickering on zoom" not "Fix bug"
- ✅ Include context in body for non-trivial changes
- ✅ List key files modified in body
- ✅ Reference issue numbers if applicable

**DON'T:**
- ❌ Use vague descriptions: "Fix stuff", "Update things"
- ❌ Include AI branding: "Generated with Claude Code"
- ❌ Write essays in subject line
- ❌ Forget to stage documentation updates
- ❌ Commit broken builds

### Phase 4: Git Operations

1. **Stage changes**: `git add -A` or selectively add files
2. **Verify staging**: `git status --short` to review
3. **Preview diff**: `git diff --staged` for final check (optional)
4. **Commit with message**: Follow conventional commit format
5. **Verify commit**: `git log --oneline -1` to confirm

### Phase 5: Post-Commit

- [ ] **Push to remote** (if ready): `git push`
- [ ] **Verify CI/CD** (if applicable): Check build passes
- [ ] **Create PR** (if using PR workflow): Include commit message as PR description
- [ ] **Tag release** (if appropriate): `git tag vX.Y.Z && git push --tags`

## Examples

### Example 1: New Feature Snapshot

**Trigger**: Just completed dual-handle range slider for zone filtering

**Validation:**
```bash
pnpm build       # ✓ Success
pnpm typecheck   # ✓ No errors
pnpm lint        # ✓ Clean
```

**Documentation:**
- Update `docs/ARCHITECTURE.md`: Add "Level Range Filtering" section
- Update `docs/CHANGELOG.md`: Add v0.21.0 entry
- Skip `CLAUDE.md`: Not a new essential pattern for ALL developers

**Commit:**
```
feat(zones): add dual-handle range slider for level filtering

Implemented custom CSS-based dual-handle slider for filtering zones by
player level range.

Implementation details:
- Pure CSS/HTML solution (no external libraries)
- 12px × 24px vertical capsule handles
- Pointer events layering for smooth dragging
- Minimum 10-level gap between handles

Technical challenges solved:
- Overlapping input pointer-events causing drag issues
- Vertical centering of capsule thumbs with 1.5px track
- Range bar overflow beyond container boundaries

Files modified:
- src/components/Sidebar.tsx: Slider component and zone visibility sync
- src/components/RangeSlider.css: Custom styling for dual handles
- src/components/InteractiveMapDirect.tsx: Steel temper color palette
- docs/ARCHITECTURE.md: Added implementation details
- docs/CHANGELOG.md: Version 0.21.0 entry
- package.json: Bumped to 0.21.0
```

### Example 2: Bug Fix Snapshot

**Trigger**: Fixed category visibility reset during map interactions

**Validation:**
```bash
pnpm build       # ✓ Success
pnpm typecheck   # ✓ No errors
```

**Documentation:**
- Update `docs/DEBUGGING-INSIGHTS.md`: Add closure trap issue
- Update `docs/CHANGELOG.md`: Add v0.15.1 entry
- Maybe update `CLAUDE.md`: If this is a common pattern ALL devs must avoid

**Commit:**
```
fix(map): prevent category visibility reset during map interactions

Event handlers captured stale createClusterIndexes in closure, causing
hidden categories to reappear after zoom/pan/drag.

Root cause:
- MapBox event handlers registered in useEffect captured initial state
- updateClusters() used outdated filter state from closure
- Lightbeam effects worked correctly (React-managed state)

Solution:
- Introduced clusterIndexesRef to store latest indexes
- Updated all event handlers to use ref instead of closure value
- Ensures map interactions use current filter state

Debugging key:
- Lightbeam effects working proved issue was MapBox-specific
- React closure traps require refs for dynamic state in event handlers

Files modified:
- src/components/InteractiveMapDirect.tsx: Added clusterIndexesRef
- docs/DEBUGGING-INSIGHTS.md: Documented closure trap pattern
- docs/CHANGELOG.md: Version 0.15.1 entry
```

### Example 3: Documentation-Only Snapshot

**Trigger**: Reorganized documentation structure

**Validation:** (Skip build steps for doc-only changes)

**Documentation:**
- Create `docs/ARCHITECTURE.md`, `docs/CHANGELOG.md`
- Update `CLAUDE.md`: Remove version history, add current patterns
- Update multiple existing docs with appended sections

**Commit:**
```
docs: reorganize CLAUDE.md and extract version history

Refactored documentation to separate essential guidance from historical
implementation details.

Changes:
- CLAUDE.md: 896 → 531 lines (40% reduction)
- Removed 12 version-specific sections (v0.11.0 - v0.22.0)
- Added "Current Architecture Patterns" with code examples
- Retained essential guidance and external resources

New documentation:
- docs/ARCHITECTURE.md: Detailed implementation stories
- docs/CHANGELOG.md: Clean version history

Enhanced docs:
- docs/ZONE-BOUNDARIES.md: Appended v0.20.0 history
- docs/DEBUGGING-INSIGHTS.md: Appended v0.15.0 insights
- docs/DEPLOYMENT.md: Appended OAuth troubleshooting

Documentation philosophy:
- CLAUDE.md: "How to work with this codebase RIGHT NOW"
- docs/ARCHITECTURE.md: "WHY and HOW decisions were made"
- docs/CHANGELOG.md: "WHAT changed and WHEN"
```

## Documentation File Naming Conventions

When creating new documentation files:

- `ARCHITECTURE-*.md` - Architecture decisions, patterns, migrations
- `DEBUGGING-*.md` - Debugging stories, gotchas, tricky issues
- `FEATURE-*.md` - Feature-specific documentation
- `GUIDE-*.md` - How-to guides, tutorials
- `REFERENCE-*.md` - API references, data structures

**Use UPPERCASE for category prefix, kebab-case for description:**
- ✅ `DEBUGGING-state-management.md`
- ✅ `FEATURE-preset-system.md`
- ✅ `GUIDE-zone-boundaries.md`
- ❌ `debugging-state.md` (not clear it's debugging category)
- ❌ `StateManagement.md` (not clear category)

## Common Pitfalls to Avoid

1. **Forgetting to update CHANGELOG.md** - Always update, even for small features
2. **Updating CLAUDE.md for everything** - Only for truly essential patterns
3. **Vague commit messages** - Be specific about what changed and why
4. **Not running build before commit** - Always validate production build
5. **Committing too many unrelated changes** - Keep snapshots focused
6. **Missing documentation updates** - Document before you forget the details
7. **Not cross-referencing docs** - Help future readers find related info

## Quick Reference: When to Update Which Doc

| Change Type | CLAUDE.md | ARCHITECTURE.md | DEBUGGING-INSIGHTS.md | CHANGELOG.md | Feature Doc |
|-------------|-----------|-----------------|----------------------|--------------|-------------|
| New pattern ALL devs need | ✅ | ✅ | ❌ | ✅ | ❌ |
| Architecture decision | ❌ | ✅ | ❌ | ✅ | Maybe |
| Tricky bug fix | Maybe | ❌ | ✅ | ✅ | ❌ |
| New feature | ❌ | Maybe | ❌ | ✅ | ✅ |
| Bug fix | ❌ | ❌ | Maybe | ✅ | ❌ |
| Performance optimization | ❌ | ✅ | ❌ | ✅ | ❌ |
| Deployment issue | ❌ | ❌ | ❌ | ✅ | Update DEPLOYMENT.md |
| Refactoring | ❌ | Maybe | ❌ | ✅ | ❌ |

## Automation Opportunities

Future improvements to this protocol:

- [ ] Pre-commit hook to verify build passes
- [ ] Commit message template generator
- [ ] Automated CHANGELOG.md entry from commit message
- [ ] Documentation completeness checker
- [ ] Version number suggester based on changes
