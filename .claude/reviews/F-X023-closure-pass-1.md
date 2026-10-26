# F-X023 closure review, pass 1

**Reviewed**: staged tree `74de1e694d6c55bce512b3913bcfa7a74765c9fc`
**Base**: preparation tree `d013d266b4cb45e5e614ef46b3059d16b6b47971`
**Author / reviewer**: xAI model per operator handoff / Codex
**Result**: 0 defects, 0 smells, 0 nitpicks in the closure implementation delta
**Scope**: partial independent review of F-X023, not approval of all shared preparation
**Date**: 2026-10-18

## Acceptance challenged

AC1 to AC3 cover exact allocation membership, completed backlog status and
preservation of existing receipt, carry and cleanliness checks. Reviewed the
command, parser contract, new tests, disposable probe builders, guard catalogue,
budget changes, corresponding LLD and changed close-command prose.

The independent composition case records a carry reason for an allocated
story absent from scratch. With otherwise valid evidence, close must still
refuse the missing inventory. Restoring that story as carried must pass beside
a completed neighbor. Another case uses backlog `in-progress` with scratch
`completed`, proving enforcement is broader than the literal `pending` case.

## Evidence

Native Python tests use isolated synthetic S99 records and mocked Git identity.
CLI probes use the harness's disposable repositories and actual Git tree
identities. Command, tests and catalogue were byte-compared with the supplied
tree. Unstaged instruction refinements are excluded.

| Command / selection | Executed now / prior | Exit and observation |
|---------------------|----------------------|----------------------|
| `python3 -m unittest discover -s scripts/tests -p test_sprint_workflow.py` | now, supplied tests | 0, 16 passed |
| Same command in isolated export with two independent composition tests | now | 0, 18 passed |
| Same isolated suite with inventory block removed | now, mutation | 1, 5 semantic failures including the omitted carried story |
| Same isolated suite with backlog status block removed | now, mutation | 1, 3 semantic failures including in-progress backlog |
| Same isolated suite after restoration | now | 0, 18 passed |
| `python3 scripts/guard_probe.py` with repeated exact selectors for all 13 `sprint-lifecycle.*` probes | now | 0, 11 declared refusals, 2 accepts, 2 harness controls |
| `python3 scripts/guard_census.py` | now | 0, all 864 scanned refusal sites claimed |
| Integrated sprint gate and old-source RED | implementer receipt only | reported in ignored audit-verification JSON, not rerun here |

Isolated source and added cases are under
`/private/tmp/ocelli-independent-6kkmmfma`. Logs are
`/private/tmp/ocelli-independent-closure-green.log`,
`/private/tmp/ocelli-independent-closure-inventory-mutation.log`,
`/private/tmp/ocelli-independent-closure-status-mutation.log` and
`/private/tmp/ocelli-independent-closure-restored.log`.
The real CLI probe output is
`/private/tmp/ocelli-codex-independent-lifecycle.log`. Mutations were restored
and never applied to the canonical checkout.

## Defects

None found in the scoped closure implementation delta.

## Smells

None found in the scoped closure implementation delta.

## Nitpicks

None recorded.

## Verified clean

Allocation is consulted independently of scratch. Empty, missing and extra
inventory cannot close. Completed scratch stories require a parsed done row,
including refusal of absent rows. Carried work retains its reason-based path.
The new probe builder makes backlog done before constructing current receipts.
The pending-backlog probe refreshes both receipt identities, so a stale tree
cannot falsely qualify that negative control. Existing current/carry accepts
and latest-failure, stale, dirty and unrecorded-carry refusals still execute.
New refusal counts and the census agree.

## Limitations and handoff

Codex authored the earlier shared instruction preparation, ledger invalidation
addition and later unstaged refinements. This reviewer cannot provide their
independent approval. F-X023 AC4 to AC7 therefore still need another reviewer
for that material. This report approves the other participant's closure delta
only. It is not a clean whole-sprint pass and does not mark F-X023 done.

The supplied 31-gate receipt applies to the supplied staged tree. Once the
instruction refinements and reports are integrated, verify the actual final
union again with the required sprint profile and obtain its independent
whole-sprint review. F-X003's representative-host measurement and the three
draft plans remain open. No implementation fix is requested by this pass.
