---
description: Close one F-ID. Updates every ledger, the LLD, and commits with the provenance trailer written from the verify ledger.
---

# /complete-feature F-XXX [--prepare]

`--prepare` is the parallel-worker form: it writes a handoff and commits only
feature-local changes to the worker branch, and does not touch sprint totals.

## Preconditions

**Refuse if any fails. Do not proceed and repair as you go.**

1. The latest `/microscope F-XXX` pass reports **zero defects and zero smells**.
2. `/verify` passed at the **current tree**. Check with
   `python3 scripts/verify_ledger.py assert`. A record for an ancestor commit
   does not count, it is a claim about different content.
3. The design plan's `## Tests` table is satisfied, and any `fixture` row cites
   a DICOM section.
4. Any deviation the work relied on has a `D-NN` row in
   `docs/hld/DEVIATIONS.md`.
5. The working tree contains no `.dcm` and no build artefact.

## Steps

1. **AS_BUILT entry**, appended at the bottom of `docs/sprints/AS_BUILT.md` in
   the documented format. Never edit a prior entry. Fill `Fixture provenance`
   and `Tier coverage` honestly, including "no pixel arithmetic" and "n/a"
   where that is the truth.
2. **SPRINT_TRACKER row.** `Days actual` is measured or it is `not measured`.
   Never an estimate dressed as a measurement.
3. **BACKLOG row** to `done`.
4. **CHANGELOG** under `## Unreleased`, if the change is user-visible. A
   refactor with no external effect does not get a line.
5. **Sprint state**, `python3 scripts/sprint_workflow.py mark-feature F-XXX
   --state completed`.
6. **LLD update**, step 9 below.
7. **Commit.** The pre-commit hook appends the provenance trailers from the
   verify ledger. Do not write them by hand.

```text
F-XXX, {short title}

{one paragraph: what was built, why, and any non-obvious choices}

Tests, {summary}
HLD, {sections implemented}
Deviations, {D-NN, or none}
```

No agent co-author trailer.

## Step 9, the LLD update

Read the plan's `## LLD impact` list and update exactly those files:

1. Add the F-ID to the file's `**F-IDs that contributed:**` line.
2. Bump `**Last updated:**`.
3. **Replace stale prose with current reality.** These are
   living-current-state documents, not changelogs. Do not append a history
   section describing what changed.
4. Confirm the file is referenced from its area README.

If `## LLD impact` is missing or wrong, that is a defect the review should have
caught. Fix the plan too.

## Under `--prepare`

Write `.claude/handoffs/F-XXX-ready.md` with the branch, base commit, head
commit, the files touched, the review pass that came back clean, and the verify
tree. Commit only feature-local changes.

**If the story is remediated after the handoff is written, the handoff is
stale and must be regenerated at the reviewed head.** A handoff naming a head
that later commits have overtaken is a record of work that is not what landed.
