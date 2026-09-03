---
description: Mark an F-ID in progress, create its branch context and test stubs, and open its progress note.
---

# /start-feature F-XXX [--claimed]

Begin work on a story whose design plan is `approved`. `--claimed` is the
parallel-worker form, used after `/claim-feature`.

## Preconditions

Refuse if any fails:

1. `.claude/plans/F-XXX-design.md` exists and its `**Status**` is `approved`.
   A `draft` plan means the design round did not finish.
2. Every dependency in the story's `Depends on` column is `done`. If one is
   not, say which, and stop.
3. The story's sprint matches `docs/sprints/CURRENT_SPRINT.md`.
4. The tree is clean, or its changes belong to this story.
5. Under `--claimed`, a claim record exists naming this agent and worktree.

## Steps

1. Confirm the branch. Serial work is on `sprint/sNN`. Claimed work is on
   `work/f-xxx-<agent>`, **with the F-ID hyphenated exactly as written**, so
   F-094 is `work/f-094-claude` and never `work/f094-claude`.
2. Set the backlog row to `in-progress`, and record it in sprint state with
   `python3 scripts/sprint_workflow.py mark-feature F-XXX --state in-progress`.
3. **Create the test stubs before the implementation files.** For any story
   doing pixel or geometry arithmetic, write the fixture with its expected
   values transcribed from the plan's normative-source section, and watch it
   fail. A stub that passes on an empty implementation is not a stub.
4. Open `.claude/scratch/F-XXX-progress.md` with the plan's summary, the
   branch, and the first action.
5. Report the plan's `## Boundary and tier` block back, so the constraints are
   in front of you rather than three files away.
