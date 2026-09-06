# F-X018 close evidence review, pass 4

**Reviewed**: sprint-independent recorded-carry probe after S05 opened
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The recorded-carry accept control no longer borrows a convenient allocation
shape from the active sprint. When no allocated story already has a tracked
reason, the probe adds a reason for one allocated story inside its disposable
repository, stages that change, then records review and verification evidence
for the resulting committed tree. Committing inside the disposable repository
is necessary because production close preflight requires both a clean worktree
and evidence matching HEAD. The unrecorded-carry refusal still chooses a story
without a reason and remains independent of that accept setup.

This is test-harness construction only. Production `close-preflight` still
reads the real `CURRENT_SPRINT.md` and accepts no reason created by the probe.
