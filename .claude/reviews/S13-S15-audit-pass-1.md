# S13 to S15 historical audit, pass 1

**Reviewed**: the three completed integrations below, with current source at
`d1d79e9525ccd94accce9261927a6fe1186890f8`.
**Reviewer**: Codex. Historical authorship is not independently authenticated.
**Result**: 2 defects, 0 smells charged, 0 nitpicks charged.
**Phase**: audit preserved before remediation. Later repair evidence belongs
in a separate pass and does not change these historical findings.

## Reviewed state and scope

| Sprint | Base, excluded | Integration, included | Stories |
|--------|----------------|-----------------------|---------|
| S13 | `439c44287cfe13b2a0d67acb69aa159afbe48cc1` | `f41f2fb4d9578d8dcc433efe9ad1e2ed8cf6741b` | F-035, F-039, F-040, F-042, F-044 |
| S14 | `f41f2fb4d9578d8dcc433efe9ad1e2ed8cf6741b` | `8f890663be91c7e333bd710205da41157a257250` | F-043, F-X002 |
| S15 | `8f890663be91c7e333bd710205da41157a257250` | `d1d79e9525ccd94accce9261927a6fe1186890f8` | F-045, F-046, F-047, F-049, F-100 |

First-parent main history selected these ranges. S12 was superseded as an
intake hint because S15 closed before execution. The live branch is
`sprint/s16`. At entry, the only local change was CURRENT_SPRINT.md opening
S16. It moved from unstaged to staged during inspection. That change is
another writer's work and is excluded from this audit's write ownership.
S16 run state was in design with F-048, F-101 and F-X003 pending.

The broader pass inspected all three file inventories, story membership,
final sprint reviews, delivery entries, design contracts, dependency and
boundary changes, packaging handoff, runtime tier tooling and GPU gate
selection. Deep areas were presentation/placement consumer composition,
progressive brick readiness and upload, and completion-evidence policy.
These cover arithmetic selection, resource/layout state and lifecycle state.

An archive of the exact S15 integration was extracted to
`/private/tmp/ocelli-audit-s13-s15` for Rust probes. It contains no ignored
corpus. Existing compiled dependencies were reused via CARGO_TARGET_DIR.
No product source was changed during audit. The completion probe imported
the unchanged script and mocked only load and Git responses, as stated below.

## Defects

### D1, manual window replacement bypasses the sequence refusal

**Where**: `crates/ocelli-viewport/src/presentation.rs:134`, `set_window`,
which calls `replace_window` at line 175 without the guard at line 117.
**Introduced in reviewed scope**: S15, F-046. Present at current HEAD.
**Impact**: a chain deliberately selected from a VOI LUT sequence becomes a
windowed chain through one API while the named preset API refuses it.
This changes displayed sample semantics and removes the renderer's existing
`VoiSequenceNotExpressible` refusal.

**Trigger**: identity modality rescale, VOI descriptor with three entries,
first mapped input 0, 8 bits and data `[5, 15, 25]`, with window alternatives
40/400. Call `set_window(40, 400, Linear)`.
**Expected**: `Err(PresentationError::Sequence)` and unchanged presentation,
under S15 CURRENT_SPRINT's sequence-preservation contract and F-046's
section-18.1 design discussion. **Actual**: `Ok(())`, replacing the sequence.
Control `apply_preset(SoftTissue)` returns `Sequence` and preserves state.

**Evidence executed now**: archive-only `tests/audit_sequence.rs`,
`bin/ocelli.sh test ocelli-viewport --test audit_sequence -- --nocapture`,
exit 101, 1 control passed, 1 semantic assertion failed, 0 ignored/filtered.
Assertion: left `Ok(())`, right `Err(Sequence)`.
The seven existing presentation tests passed. Sequence refusal is tested
only through `apply_preset`, manual window domains only on windowed chains.

**Normative distinction**: PS3.3 C.11.2.1.2.2 allows a VOI LUT or window as
alternative views. It does not establish universal sequence precedence.
The refusal being violated here is Ocelli's selected-chain policy, stated
in the sprint and design. Do not describe this as DICOM forbidding an
explicit alternative-view selection API. This repair does not add that API.
[PS3.3 VOI LUT module](https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.2.html).

**Smallest regression**: use the same sequence through manual and named
window entry points, assert refusal and full state preservation. Retain
ordinary window success and modality-sequence preservation as controls.
No tolerance change or arithmetic rewrite is needed.

### D2, close preflight checks only the feature inventory it is handed

**Where**: `scripts/sprint_workflow.py:363`, `cmd_close_preflight`, loops over
`data['features']` without reconciling `planned_features(sprint)`.
**Reviewed scope**: shared completion mechanism used throughout S13 to S15.
This predates those deltas. It is a current guard defect, not evidence that
one of these historical closes omitted a story.
**Impact**: lost or stale ignored run state can omit unfinished stories and
still produce a ready-to-close answer. The tracked delivery record is
explicitly authoritative over scratch state.

**Trigger**: S15 allocation contains F-045, F-046, F-047, F-049 and F-100.
Supply scratch `features={}`, a clean sprint review and passing sprint
verification for the same tree, with Git reporting a clean matching HEAD.
**Expected**: inventory refusal before readiness. **Actual**:
`OK: S15 is ready to close`, exit 0.

**Evidence executed now**: imported `sprint_workflow` and called
`cmd_close_preflight(Namespace(sprint='S15'))` with `unittest.mock.patch` on
`load`, `staged_tree`, and subprocess Git responses. The allocation and
CURRENT_SPRINT text were real repository inputs. Printed allocation had
five F-IDs and scratch had none. This is an isolated policy probe, not an
actual close or a claim to have run its other external prerequisites.

**Normative references**: WORKFLOW's tracked delivery authority and
close-sprint preflight item 1, every story done or explicitly carried.
`/sync-status` separately asks a human to reconcile the story set. That
manual step limits the full workflow's exposure but does not repair this
mechanical preflight's false answer.

**Smallest regression**: exact inventory succeeds, missing, empty and extra
inventories fail. Exercise the CLI via the existing disposable guard harness,
plus a focused policy test. Reconcile completion state against actual backlog
status so an inventory match alone cannot conceal pending work.

## Verified clean

Executed on the S15 archive with synthetic inputs, each command exit 0:

- `bin/ocelli.sh test ocelli-viewport --test presentation --test placement
  --test scroll --test view`: 30 passed, 0 failed, 0 ignored/filtered.
- `bin/ocelli.sh test ocelli-volume --test ready_bricks`: 4 passed,
  0 failed, 0 ignored/filtered. Missing final slices and x-fastest ordering
  have discriminating controls.
- `bin/ocelli.sh test ocelli-render --test brick_upload`: 4 passed,
  0 failed, 1 ignored GPU readback, 0 filtered. Signed/unsigned formats,
  tight rows, true edge extents and unavailable paths have distinct checks.

Read the readiness-to-cut-to-upload flow: Volume checks every slice in a
brick, Brick::cut uses checked row offsets, brick_plan checks true extent,
exact sample format and required usages, upload checks byte length before
texture creation, and recovery clears texture residency. No defect charged
in that sampled flow. GPU texture eviction to a byte budget is explicitly
not implemented by F-043 and is not represented as present here.

S13 bulk handoff builds fresh memory views after allocation and transfers a
fresh ArrayBuffer, rather than returning a view across wasm memory growth.
The browser canvas proof explicitly checks bitmap dimensions on tier C.
Its recorded errors are not an overlooked tier B success claim. The final
S13 records qualify that evidence. S14 lavapipe resolves a fallback adapter
only. SwiftShader checks its renderer and tier C. Neither is a pixel oracle.

S15 placement delegates to ImagePlane's inverse and passes column and row
spacing in the order View expects. Synthetic anisotropic and singleton-axis
fixtures passed. Axis-aligned shift after rotation is an explicit F-045
composition choice, not a new geometric deviation inferred by this audit.
Future coordinate conversion must distinguish texel corners from DICOM
voxel centers and CSS from device pixels.

Reviewed final historical reports: S13 pass 3, S14 pass 2, S15 pass 1.
Their green counts do not cover the independent D1 case. No claim is made
that their full historical verification was rerun here.

## Patterns and targeted changes

Strengths: concrete ownership, typed coordinate spaces, specification-based
arithmetic, refusal before GPU writes, and mutations targeted at wrong
constants. Delivery records often distinguish host tests, browser tiers and
unperformed parity honestly. Keep these requirements.

Weaknesses: tests partition consumers by entry point and do not challenge
shared state through every mutation route. D1 shares constructors but not
refusal policy. Require a consumer map and risk-based combinations in design
and review. D2 shows exact-tree receipts still need an exact acceptance
inventory. Require final-state metadata reconciliation and negative guard
cases, not just successful receipt matching.

Process drift inspected: run-sprint still describes the removed S01 exception,
while WORKFLOW and the runner say it is gone. `max_review_passes` is written
and not consumed by the script, an already-carried S11 finding. Do not recount
it as a newly discovered product defect. Clarify the command-level review
limit versus mechanical enforcement while preserving the per-feature
microscope loop. complete-feature verifies before completion metadata edits
and needs an explicit final-stage gate-and-record step.

## Limits, contribution intake and disposition

No new GPU, browser or external cornerstone candidate render was performed
in this historical pass. No performance optimization was attempted. No
patient data was read. Existing reference/comparator mutation evidence is
not viewer parity. The carried LINEAR output-range finding, ICC absence,
second-machine evidence and release-notice blocker remain separate work.
This audit changes neither tolerance nor release eligibility.

`gh issue list --repo tensorbee/ocelli --state open --limit 100 --json
number,title,body,url,updatedAt` and the equivalent `gh pr list` including
`headRefOid` both exited 0 and returned `[]` on 2026-10-18. There is no open
contribution acceptance inventory to pin at this intake. Recheck at handoff.
No external write was performed.

Route D1 to F-X022 and D2 plus workflow hardening to F-X023 in S16. Retain
both as in-progress until independent review and required final gates exist.
Prepare F-048, F-101 and F-X003 without implementing them. Workflow edits,
repair checks and remaining decisions will be recorded separately.
