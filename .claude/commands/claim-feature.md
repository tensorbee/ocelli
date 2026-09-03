---
description: Reserve one F-ID for one agent in one worktree, before parallel implementation begins.
---

# /claim-feature F-XXX {claude|codex} <worktree-path>

Run from the canonical sprint worktree, before the worker runs
`/start-feature --claimed`.

**This exists only to make concurrent work safe.** At one worker it protects
nothing and adds failure modes of its own: a wrong branch name, a stale
handoff, an additive conflict to reconcile. A single-story wave runs serial in
the canonical worktree with no claim at all. There is no flag for that, it is
detected.

## Steps

1. Refuse if the story is already claimed, `in-progress` elsewhere, or has an
   unfinished dependency.
2. Create the branch `work/f-xxx-<agent>`, **with the F-ID hyphenated exactly
   as written**. F-094 is `work/f-094-claude`. `sprint_workflow.py
   validate-handoff` enforces this, so a wrong name is discovered after the
   work rather than before it.
3. Create the worktree at the given path.
4. Record the claim: `python3 scripts/sprint_workflow.py mark-feature F-XXX
   --state claimed --owner <agent> --branch <branch> --worktree <path>`.
5. Report the exclusive resources this story holds, so the wave planner does
   not schedule a conflict: the source files, and any of `CURRENT_SPRINT.md`,
   `BACKLOG.md`, `SPRINT_TRACKER.md`, `AS_BUILT.md`, `CHANGELOG.md`, or a
   shared LLD section.

Never allow two agents to write the same worktree or implement the same F-ID.
