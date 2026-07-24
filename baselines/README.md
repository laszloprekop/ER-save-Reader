# Output baselines

Snapshots of each app's **enriched output** for a fixed corpus of real saves,
captured *before* character reconstruction moved into the shared core
(ADR-0010, ticket #11). See `CONTEXT.md` → **Output baseline**.

> **These are a change-detector, not a correctness oracle.** They record what each
> app renders *today*. During the migration (tickets #4–#9) every diff a slice
> produces against these files is **triaged**, never blindly failed — because
> elden-map's current output is known to be partly wrong (`CONTEXT.md` → Timeline)
> and re-blessing it as "the target" would enshrine its bugs. The baseline's job is
> to surface *every* behavioural delta so a human classifies it.

## Layout

Each app's baseline lives in its own repo, captured from the **same corpus**:

```
ER-save-Reader/baselines/reader/<save-tag>/slotNN_<name>.json   # reader ExportData
elden-map/baselines/<save-tag>/slotNN_<name>.json               # elden-map parseSaveFile output
```

The elden-map half is captured by `elden-map/scripts/capture-output-baseline.ts`
(`npx tsx scripts/capture-output-baseline.ts`), which snapshots `parseSaveFile →
SaveFileData` — its reconstruction output, the analog of the reader's `ExportData`.
Its POI/marker enrichment is downstream projection and deliberately out of scope.

The corpus is the two real backups under `Elden Ring save files/`
(`ER0000-backup-2026-01-11`, `ER0000-backup-2026-01-01`). The 2026-01-11 backup is
CLAUDE.md's reference save; its slots include the V1/V2/V3 known-differential test
characters. Ticket #1's conformance corpus reuses this same corpus.

## Regenerate the reader baseline

```
er-save-reader baseline <path-to.sl2> baselines/reader/<save-tag>
```

One JSON file per active slot. The capture is **deterministic** for a fixed save:
`export_date` is pinned and `steam_id` is zeroed (neither is part of the
reconstructed character, and a real account id must not be committed), and every
collection in `ExportData` is an ordered `Vec`/`BTreeMap`. Re-running must produce
byte-identical files — if it doesn't, that is a bug in the capture, not a diff to
triage.

## Triage workflow (per diff, during tickets #4–#9)

When a concern slice changes an app's output relative to its baseline:

1. **Regression** — the new output is *wrong* where the old was right → **fix the
   slice**. The baseline stands.
2. **Improvement** — the new output is *right* where the old was wrong (the whole
   point of the rework) → **re-bless**: update the baseline file in the same
   commit and note *why* it changed in the commit message / PR. Prefer citing the
   ground-truth method (multi-slot differential, attributed capture) that proves
   the new value correct.
3. **Neutral churn** (field reorder, cosmetic) — avoid it; if unavoidable,
   re-bless with a one-line note.

Never re-bless to make CI green without classifying the diff. A re-blessed baseline
must always be traceable to a reason.
