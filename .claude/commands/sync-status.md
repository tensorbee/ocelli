---
description: Audit consistency across the backlog, sprint plan, tracker, as-built and sprint state, and repair what is mechanically repairable.
---

# /sync-status

A consistency audit. Run it when the ledgers might have drifted, after an
interrupted sprint, or before a close.

## What it checks

```bash
python3 scripts/backlog_check.py
python3 scripts/gen_sprint_plan.py --check
python3 scripts/import_backlog_xlsx.py --check
python3 scripts/deviation_check.py
```

Then, beyond what those cover:

1. Every `in-progress` story has a design plan, a progress note and an owner.
   An `in-progress` row with no owner is an abandoned story, not a busy one.
2. No story is `in-progress` in more than one worktree.
3. Every `done` story has a tracker row, an AS_BUILT entry, and a commit whose
   subject leads with its F-ID.
4. No handoff file remains for a story that is no longer `in-progress`. **A
   handoff not consumed is proof the integration step did not run.**
5. `CURRENT_SPRINT.md` names the sprint the branch is on.
6. The sprint's story set matches `SPRINT_PLAN.md` for that sprint, or the
   difference is explained in `CURRENT_SPRINT.md` prose.

## Repair policy

**Repair only what is mechanically derivable**: a missing tracker row whose
data is in AS_BUILT, a summary block out of date, a status the commit history
settles.

**Do not invent.** A `Days actual` nobody measured is `not measured`. A missing
AS_BUILT entry for a story that shipped is written from the diff and the plan
and says that is where it came from. Report anything you could not derive
rather than filling it in plausibly.
