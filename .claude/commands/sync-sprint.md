---
description: Open a sprint. Creates the branch, rewrites CURRENT_SPRINT.md from the plan, and initialises sprint state.
---

# /sync-sprint S{NN}

## Steps

1. Confirm `main` is current and the tree is clean.
2. Create `sprint/sNN` off the latest `main`.
3. Read the sprint's stories from `docs/sprints/allocation.json`, which is the
   generated allocation, and cross-check against the `SPRINT_PLAN.md` table for
   that sprint. If they disagree, stop. `scripts/gen_sprint_plan.py --check`
   is the arbiter and the disagreement is a real defect.
4. Rewrite `docs/sprints/CURRENT_SPRINT.md`: milestone, branch, goal, the story
   table, and prose covering:
   - **What this sprint is**, in one paragraph.
   - **What is carried in** from the previous close, if anything.
   - **The defect class this sprint is exposed to.** Not generic advice.
     A sprint touching the LUT chain is exposed to boundary-comparison and
     rounding errors. A sprint touching volume construction is exposed to
     spacing derived from a tag. Name the specific one.
   - **What "done" means** for anything whose completion is not obvious.
5. Initialise state, `python3 scripts/sprint_workflow.py init --sprint SNN`.
6. Report the story set, the dependency order, and any story whose
   dependencies are not yet `done`.

## Count the rows rather than trusting a sentence

Prose in `CURRENT_SPRINT.md` that states a story count goes stale the moment a
story is added or pulled forward. Where a count matters, write the command that
produces it rather than the number:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
```
