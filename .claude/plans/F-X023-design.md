# F-X023, closure inventory and shared workflow hardening

**Status**: approved
**Epic ref**: Y1.18
**Sprint**: S16
**Estimate**: 2w
**Owner**: xAI model implementation, independent reviewer unassigned

## Normative source, transcribed

HLD section 27.2 R2: "Tests derive from the spec or the oracle, never from
reading the implementation". Section 27.3: "That a new test would actually
fail if the code were wrong. Mutate one constant, re-run, confirm it goes red."

WORKFLOW: "The delivery record is docs/sprints/" is also the authority rule
in sprint_workflow's module contract. close-sprint preflight item 1 requires
"Every story in the sprint is done, or is explicitly carried forward".

## Problem, scope and consumer map

Audit D2 removes every scratch feature and close-preflight still says ready
when receipts and Git identity match. The checker never asks allocation which
stories belong. `planned_features` reads allocation, scratch `load` reads run
state, and `backlog_check.backlog_rows` already parses authoritative statuses.
Use those concrete providers. The consumer is `/close-sprint`. No auto-close,
commit, push or release behavior is added.

## What the specification does not cover

Exact scratch membership is required at close. Resume does not silently add
new intake to stale state. An operator must reconcile new F-IDs deliberately.
Completed scratch stories must have a done backlog status. Carried stories
retain the existing named-reason rule and are not falsely marked done.
Preparation is a new `/prepare-sprint` command, not a change of meaning for
`/complete-feature --prepare` or an invocation of `/run-sprint`.

## Approach

Compare scratch keys with the allocation set in close-preflight, and compare
completed states with backlog statuses. Keep current-tree, latest-failure,
carry and clean-worktree checks. Add focused temporary-state tests and
standing disposable guard probes for missing, extra and inconsistent inventory.
Update the census site count with the existing record command after review.

Add a canonical preparation command that writes only intake, plans and
handoffs. Extend design's acceptance/consumer matrix, microscope's independent
composition challenge, verify's final-state evidence and complete-feature's
post-metadata gate ordering. Remove stale S01 exception prose from run-sprint
and document its review cap as an orchestration responsibility rather than
claiming the script enforces it. Regenerate host adapters.

## Acceptance and tests

1. Exact inventory and done backlog pass with current receipts. Empty,
   missing and extra scratch inventory fail. Tests must fail on old source.
2. A completed scratch story with pending backlog fails. A properly explained
   carried story still works. Test both the negative policy and happy path.
3. Existing stale/missing/latest-failure/dirty-tree close probes stay enforced.
4. `/prepare-sprint` explicitly stops before product implementation or Git
   publication. It lists writes, dependencies, decision readiness and handoff.
5. Designs require numbered acceptance criteria and wrong-implementation
   controls. Review requires an independent case across affected consumers.
6. Completion metadata is staged and checked before evidence is recorded.
   A later failed check invalidates its earlier success. Source and selected
   tests are named. No unavailable GPU or candidate parity is called passed.
7. Regenerated adapters match. No parallel tracker or copied normative spec
   is introduced. Relevant guard, prose and skill checks pass.

Run `python3 -m unittest discover -s scripts/tests -p test_sprint_workflow.py`,
selected `sprint-lifecycle` probes, census and skill sync checks, then the
required integrated sprint gate on the final staged union. An unmet gate or
independent reviewer remains explicit and blocks completion.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: not touched
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Dependencies and integration

F-X018 is done. F-X022 is independent product work. Canonical commands,
backlog/allocation and guard budget have one writer. Preserve the existing
S16 opening edit in CURRENT_SPRINT. Preparation of F-048, F-101 and F-X003
follows hardening and must not implement their product stories.

## Parity surface covered

None.

## Deviations

None. Workflow changes are process decisions recorded here and in WORKFLOW.

## LLD impact

Update `docs/lld/guards.md`, its index, and canonical workflow commands.
Record the workflow trigger in the next AS_BUILT completion entry. Until
completion, this plan and the scratch handoff carry that pending requirement.

## Open questions

None for the scoped guard changes. F-X003's physical-host cost baseline and
candidate comparison integration remain design questions for that story.

## Preparation already present

Codex sharpened canonical instructions and added verify_ledger invalidate with
a focused test. The close-preflight inventory/status repair and its proposed
regressions were prototyped, tested, then removed on the operator's instruction
to leave fixes to xAI model. Reproduce audit D2 before implementing. Do not describe
those changes as already present. Read the detailed repair handoff.

## Additional identity repair, 2026-10-19

Codex reproduced that `unsafe --staged` omits unchanged vendored packages and
reads working-tree bytes for changed paths. This belongs to AC6 evidence
identity and AC7 backing-policy verification. Enumerate the complete index,
read its blobs, retain the same allow-list and vendor counts, and fail closed
on inventory or source-read errors. Permanent temporary-repository controls
cover unchanged vendor inventory, staged unsafe hidden by clean working bytes,
unstaged unsafe outside a clean index and an unavailable Git inventory.
The old guard fails all four controls. The repair passes them. The existing
unsafe gate also runs these controls. Independent approval remains deferred.
