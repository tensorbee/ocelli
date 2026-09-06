# F-X018 close evidence review, pass 1

**Reviewed**: staged F-X018 implementation
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1. The documented run never recorded verification in sprint state

**Where**: `.claude/commands/run-sprint.md`, consolidated verification

**What**: The command recorded a successful sprint gate only with
`verify_ledger.py`. The new close-preflight reads verification from
`.claude/scratch/SNN-run.json`, but the documented flow never invoked
`sprint_workflow.py record-verification`.

**Why it is wrong**: Following `/run-sprint` exactly would leave
close-preflight reporting no sprint-profile verification even after a green
gate. F-X018 requires closure ordering and commit provenance to bind to the
same staged tree.

**Evidence**: Searching the command sources for `record-verification` returned
no invocation before the repair. The consolidated verification section now
records both stores and the generated adapter matches it.

## Smells

None.

## Nitpicks

None.

## Verified clean

The pass also exercised all seven new close probes. Legacy, dirty, stale,
failed-latest and changed-tree states went red for their declared reasons. The
current clean-evidence control stayed green.
