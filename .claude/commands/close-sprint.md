---
description: Close a sprint. Validates readiness, merges to main, creates the sNN tag, and opens the next sprint. Never publishes.
---

# /close-sprint S{NN} --next S{MM}

The operator's command. `/run-sprint` deliberately stops short of it.

**This creates an `sNN` history tag. It does not create a release tag and it
never publishes.** Publication is `/release`, and it is gated on a milestone,
not a sprint. See `docs/RELEASE.md`.

## Preflight

Refuse if any fails:

1. Every story in the sprint is `done`, or is explicitly carried forward with a
   recorded reason in `CURRENT_SPRINT.md`.
2. `python3 scripts/sprint_workflow.py close-preflight SNN` passes.
3. `bin/ocelli.sh gate --sprint` is green at the current HEAD. The only
   difference from `--all` is S01's named pre-oracle skip while F-010 remains
   pending in S02. Release verification stays strict.
4. `python3 scripts/verify_ledger.py check-commit HEAD` passes.
5. `/sync-status` reports no drift.
6. No handoff file remains in `.claude/handoffs/`.
7. No worktree remains for this sprint's workers.
8. The `## Unreleased` CHANGELOG section covers every user-visible change.

## Close

1. Merge the sprint branch into `main` with `--no-ff`.
2. Create the annotated tag `sNN` with a message naming the milestone, the
   stories closed and the gate result.
3. Push `main` and the tag.
4. Run `/sync-sprint SMM`.

## Milestone boundary

If this sprint completes a milestone, say so explicitly and report:

- the milestone, its stories, and its total estimated engineer-weeks
- the **measured** total from `SPRINT_TRACKER.md`, and the ratio
- whether a release is due per the table in `docs/RELEASE.md`

**Recalculate capacity after S03 and S06** and write the result into
`SPRINT_PLAN.md` under "Capacity calibration". Do not recalculate off one
sprint, and do not recalculate off S01 to S03 alone when forecasting the port
work: foundations and oracle work is not representative of it.
