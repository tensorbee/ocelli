---
description: Merge one prepared worker branch into the canonical sprint worktree with a three-way merge, and consume its handoff.
---

# /integrate-feature F-XXX work/f-xxx-<agent> [--batch]

Run from the canonical sprint worktree. `--batch` creates one local F-ID commit
and does not push, mark the story done, or claim verification passed.

## Preconditions

1. The handoff `.claude/handoffs/F-XXX-ready.md` exists and validates.
2. **No implementation commit exists after the head the handoff names.** The
   handoff validator checks field shape, not ancestry, so it exits clean on a
   handoff that later commits have overtaken. This step is what catches that.
3. The last review pass for the story reports zero defects and zero smells.

## Merge with a three-way merge, never a hand-picked file list

**A worker branch based before other stories landed cannot be merged by listing
its files.** A file the worktree modified but that is not on your list is
silently dropped, and nothing notices until review, if then.

Use the worktree's base commit as the common ancestor:

```bash
git show <base>:<path> > /tmp/base
git merge-file -L mine -L base -L theirs <canonical-path> /tmp/base <worker-path>
```

## Resolving

**Keep BOTH sides where they are different facts.** An AS_BUILT entry, a
CHANGELOG bullet, a tracker row and a contributed-F-ID line are **additive**.
Taking either side whole destroys the other story's work. This is the common
case and it is reconcilable without operator direction.

Then **prove the result rather than eyeballing it**: count the rows or sections
that must now be present, and run the guard that owns the file. A file that
auto-merged with no conflict still needs checking when two stories both wrote
it.

A semantic conflict between two stories is reconciled against both approved
plans, re-reviewed, and recorded. Any other conflict stops for the operator.

## After

1. Consume and delete the handoff file. Leaving it is proof this step did not
   complete, and `/sync-status` reports it as such.
2. Record the integration commit in sprint state.
3. Remove the worktree and prune. **A worktree is a full checkout**, so every
   path-scanning guard otherwise sees a second copy of the whole repository and
   reports duplicate findings that are very hard to interpret.

```bash
git worktree remove <path> --force && git worktree prune
```
