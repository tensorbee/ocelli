# S13 to S15 repair evidence and xAI model implementation handoff

**Prepared by**: Codex
**Historical audit**: `.claude/reviews/S13-S15-audit-pass-1.md`
**Implementation owner**: xAI model, the operator-selected model
**Independent review**: a subsequent Codex pass, not executed here
**Identity note**: xAI model means the operator-selected implementing model.
The source-provenance guard treats its product name as the prohibited JPEG
2000 project name, so tracked notes use the vendor identity to avoid that
ambiguity. The prohibited source project remains out of bounds.

**Status**: repair guidance. Product and closure-tool fixes remain unapplied.

## Scope and current source

The operator assigned fixes to xAI model after Codex had prototyped and checked
both repairs. Codex preserved the results below, then removed its changes to
presentation.rs, its regression tests, sprint_workflow.py, the guard catalogue,
probe budget, runbook table and viewport LLD. Those paths match the S16 base.
Neither audit defect is represented as fixed in the delivered code.

Shared instruction hardening remains: prepare-sprint, a concrete design
acceptance template, consumer maps, implementation evidence, review controls,
final metadata verification and clearer workflow stopping points. A small
`verify_ledger.py invalidate` command and its focused test remain as workflow
support. Do not reimplement that command during the closure repair.

F-X022 and F-X023 are allocated to S16 and pending. Their approved repair
plans settle the bounded fixes. F-X023 is partial preparation plus a pending
closure-tool repair, not completed delivery. The original S16 product stories
F-048, F-101 and F-X003 have draft plans and remain unimplemented.

No historical completion or review was overwritten. No new completion entry,
tracker row, clean independent review or ready-to-close state is claimed.
The next F-X023 AS_BUILT entry must record this audit as the WORKFLOW change
trigger. Eventual product and workflow commits must be separate, when an
invoked workflow authorizes them. This handoff authorizes no Git publication.

## F-X022, exact repair guidance

Read `.claude/plans/F-X022-design.md`, F-046's historical plan and audit D1.
Reproduce before changing source. Current direct manual windows bypass the
VOI-sequence guard at `Presentation::apply_preset`.

Use the existing concrete owner rather than a policy trait or wrapper:

1. In `apply_preset`, retain the modality-sequence check,
   `self.chain.modality().rescale().is_none()`, returning Sequence.
2. Move the VOI-sequence check to `replace_window`, before constructing a
   new VoiTransform or mutating anything:

   ```rust
   if self.chain.voi().window().is_none() {
       return Err(PresentationError::Sequence);
   }
   ```

   Both manual and named entry points already call this function. No new
   signature, VOI formula, texture update or error variant is necessary.
3. Document Sequence on set_window and preserve its ordinary domain errors.
   Keep the full Presentation unchanged on refusal, including evidence,
   photometric context and colormap. Update current viewport LLD and index.
4. Do not refuse manual windows just because modality is a sequence.
   `rebuild` clones that modality unchanged. The existing named-preset policy
   continues to refuse it. This control rejects an overly broad repair.

Add regressions to the existing presentation test file using its helpers:

| Case | Setup / operation | Required observation |
|------|-------------------|----------------------|
| Shared refusal | VOI LUT descriptor 3 entries, first input 0, 8 bits, `[1,2,3]`. MONOCHROME1, absent shape, Stops colormap. Clone full Presentation. | For each Linear, LinearExact and Sigmoid, set_window 40/400 returns Sequence, full clone equality holds. |
| Named control | Same selected sequence, ApplyPreset SoftTissue | Same Sequence and full equality. |
| Preserve modality | Modality LUT `[1,2,3]`, competing rescale 9/99, ordinary VOI window | Manual window 40/400 succeeds, modality equality holds, stored 1 still maps to modality 2, not rescale 108. |
| Existing behavior | Existing preset, invert, window-domain and colormap cases | Retain all existing assertions and tolerance. |

PS3.3 C.11.1 and C.11.2.1.1 establish those lookup values. Ocelli's
selected-chain policy establishes refusal. DICOM permits alternative VOI
views, so do not misstate refusal as a universal standard requirement.

Commands, each with its own status recorded:

```bash
bin/ocelli.sh test ocelli-viewport --test presentation
bin/ocelli.sh check ocelli-viewport
bin/ocelli.sh test ocelli-viewport
bin/ocelli.sh clippy ocelli-viewport
```

Record old-source semantic RED first, GREEN after the fix, then a controlled
removal of the shared guard which must turn the same regression red. Restore
it before final verification. No compiler error substitutes for semantic RED.

## F-X023, closure-tool repair guidance

Read `.claude/plans/F-X023-design.md` and audit D2. Exact tree receipts are
necessary but the current code accepts whichever feature inventory scratch
contains, including none. The tracked allocation and backlog remain authority.

Inside `cmd_close_preflight`, before the closure-state loops:

1. Build expected F-IDs through existing `planned_features(sprint)` and actual
   F-IDs from `data['features']`. Refuse an empty expected set or inequality.
   Name sorted missing and extra F-IDs in the message. Do not silently populate
   scratch or discard its review history during a readiness check.
2. Read statuses through existing `backlog_check.backlog_rows`. Its `cells`
   include the trailing empty field, so status is `cells[-2]`. A completed
   scratch feature needs status done. A missing row is a refusal too.
3. Leave carried states on the existing nonempty carry-reason path. Do not
   mark carried work done or demand an implementation review for it.
4. Preserve current staged-tree/HEAD/clean-worktree checks, latest failing
   verification precedence, latest sprint-review cleanliness and no-handoff
   checks. Do not broaden this repair into an invented new tracker.

Suggested stable refusal fragments, observed in the prototype:

```text
feature inventory differs from allocation: missing=[...], extra=[...]
F-XXX is completed in run state but backlog status is pending
```

Tests must keep scratch, allocation and backlog as independent inputs:

- Exact inventory and done backlog with current receipts pass.
- Empty inventory, one missing feature and one extra feature fail, even when
  review/verification/Git tree identities are otherwise green.
- A completed scratch feature with a pending backlog fails.
- Recorded carried work and existing stale/failed/dirty conditions retain
  their current results. Do not fix the tests by weakening those controls.

The initial focused policy test mocked only external Git identity and used
separate temporary CURRENT_SPRINT/BACKLOG inputs. It supplied a two-feature
allocation and the scratch variants above. For a robust permanent test,
isolate HANDOFFS as well as SPRINTS so unrelated local handoffs cannot change
its result. Keep this suite in `scripts/tests/test_sprint_workflow.py`, already
run by the guards gate. Preserve its existing grammar and receipt tests.

Add real CLI probes under the existing sprint-lifecycle entry:

```text
sprint-lifecycle.close-missing-inventory
sprint-lifecycle.close-extra-inventory
sprint-lifecycle.close-pending-backlog
```

The existing `_write_close_state` fixture currently marks scratch completed
without marking the sandbox backlog done. Once status is checked, update the
fixture to create an actually completed tracked backlog in the disposable
repository, stage/commit that fixture there, then record its tree. Never run
those fixture mutations in the real checkout. Existing probe helpers that
provide stale evidence deliberately must retain their intended failure.

For the missing/extra probes mutate ignored state only. For pending backlog,
change the sandbox backlog, stage/commit there, and refresh both receipt trees
to that new tree before calling close-preflight. Otherwise the probe may fail
on a stale tree and never prove the pending-status guard works.

Prototype checks to repeat:

```bash
python3 -m unittest discover -s scripts/tests -p test_sprint_workflow.py
python3 scripts/guard_probe.py --only sprint-lifecycle.close-missing-inventory
python3 scripts/guard_probe.py --only sprint-lifecycle.close-extra-inventory
python3 scripts/guard_probe.py --only sprint-lifecycle.close-pending-backlog
```

`--only` takes exact probe ids and may be repeated. A bare guard id is not a
valid selector. Run all existing sprint-lifecycle probes too. Pause edits
while the harness runs because its tripwire checks repository stability.
After adding and reviewing the refusals and probes, run:

```bash
python3 scripts/guard_census.py --record
python3 scripts/guard_census.py --render-runbook
python3 scripts/guard_census.py
```

The prototype grew sprint-lifecycle from 32 to 34 refusal sites. Re-measure,
do not copy the number into the budget by hand. Update guards LLD/index to
state what the tool actually enforces. Remove temporary workflow caveats about
pending mechanical enforcement only after the repaired command proves them.

## Prior prototype evidence, not current implementation evidence

| Check | Old-source result | Proposed-repair result |
|-------|-------------------|------------------------|
| Presentation file with two new regressions | exit 101, 8 pass, 1 semantic fail | exit 0, 9 pass |
| Closure inventory policy cases | exit 1, four failing subcases for empty/missing/extra/pending | exit 0 |
| Entire viewport crate with regressions | not claimed | exit 0, 33 tests, 0 ignored |
| Viewport check and clippy | not claimed | both exit 0 |
| All sprint-lifecycle CLI probes with proposed repair | not claimed | exit 0, 11 refusals, 2 accepts, 2 harness controls |

Those results show a concrete candidate repair, not accepted xAI model delivery.
The proposed source was staged as `1f525c04258fd64ae832c4cf215f65eafe46e135`
for a sprint-profile attempt before the operator reassigned implementation.
Its run is superseded as final-tree evidence by the scope change, regardless
of its exit status. The final delivered tree has different source and is
verified separately. Do not reuse the prototype receipt for xAI model's changes.
The local proposed patch is retained at
`/private/tmp/ocelli-audit-proposed-repairs.patch` as optional reference. The
instructions above are self-contained and do not depend on that temporary file.

An earlier probe run's enclosing harness exited 1 because Codex regenerated
budget/runbook files concurrently. The sequential rerun with edits paused
exited 0. Keep the failed attempt visible. Do not weaken the tripwire.

## Shared instruction changes already present

The design template now requires concrete reproducers, scope/API choices,
consumer/provider context, numbered acceptance and wrong-implementation
controls, dependencies/write ownership, evidence stages and invalidation.
Implementation and microscope commands require semantic RED/GREEN and actual
consumer observations, with bounded finding criteria and actual participant
identities. prepare-sprint explicitly stops before product implementation.

verify invalidates an older tree receipt before a new attempt. Completion
metadata must be staged and checked before recording evidence. The original
stale S01 exception was removed from run-sprint prose. Its review cap is
accurately described as host orchestration, not automatic script enforcement.
WORKFLOW now names commit-msg as the provenance writer, pre-commit as warning
on missing receipts, and pre-push as enforcing head evidence, matching source.
Canonical domain skills distinguish DICOM semantics from project selection
policy and require independent fixture derivation through consumers.
Generated adapters must match those canonical changes.

## Remaining S16 work and decisions

Execution order is shared F-048 mapping, common F-101 boundary, then F-X003
raster and final state/benchmark integration. Read their draft plans before
implementation. Shared files and GPU access require coordinated serial steps.
The absence of imported intra-sprint dependencies does not prove independence.

The operator confirmed no GPU-less host or incumbent CPU-cost baseline is
available yet. F-X003's measurement acceptance remains unmet. Do not invent
budgets or replace the representative-host requirement with a SwiftShader run.
Settle its colour acceptance and bounded stack candidate producer explicitly.
The later F-X021 full-viewer producer does not waive F-X003's tier A/C bound.

Other material design choices remain in draft Open questions: CSS/device and
center/corner conventions, noninvertible measurement errors, command numeric
ids and payloads, semantic batch atomicity, allocation/ring limits and concrete
binding signatures. Consolidate these once, reuse existing decisions and do
not implement unresolved behavior by assumption.

F-X022 adds 1 and F-X023 adds 2 estimated engineer-weeks to the original 14,
for 17 estimated engineer-weeks. No measured duration or capacity is inferred.
The read-only remote refresh returned no open issues or PRs. Recheck at intake,
pin relevant new issue bodies/PR heads, and keep partial resolution partial.

## Verify, review and finish

1. Reproduce the two audit findings on the current base. Apply the repair
   plans with discriminating regressions and capture independent old/new results.
2. Ask an independent Codex reviewer to follow `/microscope F-X022 --working`
   and `/microscope F-X023 --working`. Preserve findings, remediate, and request
   a fresh pass on changed material. This author's prototype checks are not
   that independent approval.
3. Update final metadata only after acceptance is met. Stage only owned
   content with real git add. Set OCELLI_AGENT to the actual implementing model.
4. Follow `/verify --profile sprint` on the final integrated union. Invalidate
   its prior receipt first and record actual gate/corpus results. A later failed
   attempt cannot reuse the old success. Run whole-sprint independent review
   on that same staged tree, and repeat the required cycle after remediation.
5. `python3 scripts/sprint_workflow.py status --sprint S16` is the state view.
   `/prepare-sprint` revises remaining draft designs without implementation.
   `/run-sprint --max-workers 1` is a later execution entry point only when
   separately invoked with its authorization contract. Do not use it to plan.
6. Completing, committing, pushing or `/close-sprint S16 --next S17` is not
   authorized merely by reading this handoff. No publication or external
   messages are included. Record the next AS_BUILT workflow-change trigger
   when actual completion is authorized and verified.

Final delivered-tree gate results, actual source identity and logs belong in
ignored `.claude/scratch/S13-S15-audit-verification.json` and the final user
report. No outcome is predeclared here. The known defects remain open until
xAI model implements and independent review accepts them.
