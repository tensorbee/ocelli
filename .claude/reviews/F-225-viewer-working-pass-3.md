# F-225 review, viewer half, pass 3

**Reviewed**: staged tree `d9efffbf5de1b4263ce3329166ac380eb256a832` on `work/f-225-xai`, HEAD `534c3a7492db623388670a000797050126a1a5f1`. Four staged paths, no unstaged tracked changes. The historical pass 2 review is context. The untracked `.venv` and `tools/oracle/out` symlinks were preserved.
**Author / reviewer**: GLM `dgx/GLM-5.3-Flash-EXL3` through OpenCode authored the product null-entry repair and its browser regression, as recorded in the existing progress note and run logs. Root Codex replaced elapsed sleeps with the test's due-state scheduling barrier and added the plan ruling. Independent review by Codex participant `/root/f244_review`, which authored none of that material or the prior F-225 review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- Pass 2 D1, the old Headers repost ending in AbortError while a new folder's report is due and the lazy module import remains held.
- The permanent regression's sensitivity to restoring the old unconditional null-entry break.
- A composition with two successive folder replacements while the old operation is pending. The latest folder has a valid image with no findings, so badge absence alone cannot count as success. Its report must actually be requested and cached.
- A stale abort after opening a folder with no DICOM series, followed by recovery into a valid folder. The abandoned report must not repopulate the empty folder, and the later valid report must still run.

## Evidence

Browser commands used the existing release viewer in the reviewed worktree, built by root for the product repair, and the pinned Playwright browser. All executed browser runs reported Chromium `151.0.7922.34`. Review-only source copies export the tracked test's fixture builder and helper with imports made absolute. They do not change product or test logic. Scripts, logs and the fixture copy are retained at `/private/tmp/ocelli-f225-review-p3`.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` | Actual index | now | 0, exact tree above |
| `permanent.mjs`, base | Actual current `observeChecks` body, unchanged served page, generated synthetic fixtures | now | 0, library served, no problems |
| `permanent.mjs mutant` | Same helper, old null-entry branch restored in eight served HTML responses | now | 1, new aborted case detects the lost request |
| First `composition.mjs` attempt | Review-only A to B to C probe | now, failed probe | 1, timeout because the probe kept the old file repost held while waiting for a due state that requires zero file posts |
| Corrected `composition.mjs` | Same independently derived cases, old repost released before the due-state barrier | now | 0, latest valid folder and empty-folder recovery pass |
| `node --check examples/folder-viewer/tests/browser/conformance.mjs` | Actual staged test source | now | 0 |
| `git diff --cached --check` | Actual index | now | 0 |
| `git diff --exit-code` | Actual unstaged worktree | now | 0 |
| Root's tightened full helper and old-branch mutation | Root execution recorded before this review's browser slot | prior receipt, not substituted for the runs above | Root reports 0 and 1 |

The initial composition timeout is a defect in the review probe's scheduling, not a product finding. Its log remains `composition.log`. The corrected run is `composition-fixed.log`. Both permanent runs and their exact results are retained in `permanent-base.log` and `permanent-mutant.log`. Each script closes its browser, stops its own viewer PID and removes its temporary DICOM fixture directory in `finally`.

## Defects

None found in this bounded pass.

## Smells

None found in this bounded pass.

## Nitpicks

None.

## Verified clean

- **Pass 2 D1 closed.** A null entry from an older generation continues the loop, which rereads the current `checks.due`. A null entry from the current generation retains the prior stopping behavior. The stale successful-reply branch is unchanged.
- **Permanent regression is discriminating.** On the base, the full helper passes the original held-success case, the new aborted-repost case, missing-library and failed-request cases, body-form fallback, report reset, and colour and refused-image badges. Restoring only `if (!entry) break` in served HTML causes the new aborted case to report a hidden badge, zero new id requests, a due `c1.dcm` and zero kept reports. The other cases do not report problems.
- **Scheduling barrier establishes the trigger.** The new case releases the old repost, then waits for zero file posts while `checks.busy` remains true and the non-null due file equals the shown file. Only then is the lazy import released. A slow timer can no longer make the case pass by rearming after the old loop has ended. The loop code and the test barrier were read together.
- **Latest folder wins.** The independent A to B to C case reaches generation 3 with shown `c2.dcm`, no due file, busy false and one cached report with zero findings. The badge is hidden as expected for that valid report. Exactly one id request, answered 410, and its body fallback, answered 200, are observed for that file, both with zero file posts and thumbnail runs.
- **No-series open and recovery.** After the stale abort completes in the folder containing only a synthetic non-DICOM text file, shown and due are null, busy is false, the report cache is empty and no conformance request was sent. Opening the valid folder afterwards produces one zero-finding report and clears due.
- The full helper observes the reproducer's four expected rows without repeating its synthetic secret values in the Checks section. Its delayed request is observed after first paint, and the report is not requested during the rapid scroll sequence. These are scheduler checks, not idle-host performance measurements.
- No parser, checker arithmetic, wasm boundary, decoder or rendering tier behavior changes in this remediation. The product change is confined to the null-entry scheduler branch.

## Limitations and handoff

- This pass establishes bounded clean coverage of the viewer repair and regression, not completed F-225 acceptance or sprint readiness.
- AC34 idle-host open, first-image and scroll timing remains open. No benchmark timing, GPU, oracle, wasm, native cross-target, full workspace, floor or sprint profile was run.
- The existing F-225 branch still needs composition with the final F-215 work and the rest of the planned viewer stack. These browser observations do not certify a future integration tree.
- The same-generation null stop was checked in source. A forced same-generation AbortError was not independently exercised. Normal request-error behavior was exercised by the full helper.
- All inputs were the tracked test's generated synthetic fixtures. No patient file or patient metadata was opened or copied.
- No product remediation, staging, commit or verification receipt was written by this reviewer. Root owns remaining acceptance, feature verification and integration.
- Browser slot was released after all owned browser and viewer processes finished.
