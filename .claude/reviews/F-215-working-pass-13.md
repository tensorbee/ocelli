# F-215 review, pass 13

**Reviewed**: staged tree `d01534d92675c225d1cf2e7548a0ddb9143e8cbd` on `work/f-215-xai`, HEAD `b599c273446c39a30f421bbcf59773a46e685479`. This bounded pass reviews the pending-save test repair since pass 12. Product source is unchanged from the tree reviewed in pass 12. The added historical pass 12 review is context, not new independent approval by its own author.
**Author / reviewer**: test repair by root Codex. Independent review by Codex participant `/root/f244_review`, which wrote none of the repaired test. Browser instrumentation and the served-page mutation live outside the worktree.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- Pass 12 D1, replayed with the actual exported permanent `observePendingSaves` helper.
- The Save button's real registered listener, its captured Promise, and the point at which the test observes B before issuing another Save.
- The generation-guard negative control from pass 12, with only that production guard removed from the served HTML.
- The adjacent valid same-folder case, which still includes both marks in one saved SR.

## Evidence

The independent driver is `/private/tmp/ocelli-f215-review-p13/independent-discrimination.mjs`. It imports the actual test helper and starts the existing release viewer, then uses the pinned Playwright browser. That is the same browser installation which reported Chromium `151.0.7922.34` in pass 12. This driver's logs do not independently print the browser version. Node reported `v26.6.0` in the mutant failure. Browser and viewer are stopped in `finally` in both runs.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` | Actual index | now | 0, exact tree above |
| Independent permanent-helper driver, base | Actual staged test helper and unmodified served page | now | 0, expected pending state and saved group counts |
| Same driver, generation-guard mutation | Actual staged helper, two page responses with only the Save generation guard removed | now | 1, semantic stale-state assertion |
| `git diff --exit-code` | Actual worktree after the probe | now | 0, no unstaged tracked changes |
| Root's repaired-helper base and mutant, ESLint | Root implementation evidence | prior receipt, not rerun by reviewer | Root reports 0, 1 and 0 respectively |

Base and mutant logs are `base.log` and `no-generation.log` in the same temporary directory. The mutation target is checked for presence before substitution. No source worktree file is mutated.

## Defects

None found in this bounded pass.

## Smells

None found in this bounded pass.

## Nitpicks

None.

## Verified clean

- **Actual listener observed.** The test saves the original `window.saveMarks` function, removes that exact function from the button, and registers one wrapper calling it. The wrapper stores the returned Promise, and the test requires a Promise before continuing. This closes the observation defect from pass 12.
- **Old operation observed before the valid new operation.** After releasing the import and awaiting that captured Promise, the folder-switch base case observes zero remembered documents, one changed study and zero pending files. Only then does it issue a valid B Save. That Save writes one group whose first column is 60.
- **Wrong implementation rejected.** Removing the production Save generation guard now observes one remembered document, zero changed studies and one pending file before the valid B Save. The independent driver exits 1 at its stale-state assertion. The missing guard no longer hides behind a later correct download.
- **Adjacent valid operation preserved.** The same-folder case writes one document containing both marks, with the first column approximately 10 as before.
- **Permanent verdict consumes the observation.** `marksProblems` now checks the returned pending document, changed-study and file observations for the folder-switch case in addition to its final group assertions. This is source inspection of the verdict branch, separate from the executed helper and independent assertions.
- The listener instrumentation changes how the test captures completion, not the product save state or module-loading implementation.

## Limitations and handoff

- This is a clean bounded test-repair review. The product lifecycle, duplicate-load and saved-report ownership observations remain the evidence in pass 12 and were not repeated here.
- No complete frontend suite, full marks suite, viewer-browser gate, GPU, oracle, cross-target, wasm or sprint profile was run by the reviewer in this pass.
- Real Chrome AC18 and external-viewer AC19 remain open. This report does not complete story acceptance or sprint verification.
- No remediation, staging or commit was performed by this reviewer. Root owns feature verification, remaining acceptance and integration.
- Browser slot was released after the two probes and owned processes finished.
