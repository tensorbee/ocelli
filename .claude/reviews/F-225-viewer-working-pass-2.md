# F-225 review, viewer half, pass 2

**Reviewed**: `e8205ef0..534c3a7492db623388670a000797050126a1a5f1` on `work/f-225-xai`, one implementation commit, `534c3a74`. The worktree had no tracked changes at the start. The untracked `.venv` and `tools/oracle/out` symlinks were preserved. Source hashes: `page.html` SHA-256 `dba14bb15a33e32683ce99c0fdb5e3acbb3eed7e965c4e741f35bf5061c86d7f`, `conformance.mjs` SHA-256 `d1672a723a1369923cb9ac62ee9c97cff103a73c18d0724374e54873438b4aed`.
**Author / reviewer**: the remediation is the prior Claude implementer's work, as recorded by the lead and commit message. This review is by Codex agent `/root/f225_review`, which authored none of the reviewed source, design or earlier findings. Scratch-only browser response substitutions are review experiments, not implementation edits.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

AC33 was challenged through the F-216 expired-id retry in the Headers caller, composed with the lazy `@ocelli/core` import and a folder replacement. The original held-reply regression tests a successful stale reply. This pass also tests a stale operation that returns `null` after `AbortError`. Both must let the next folder's due request run.

Pass 1 D1's original held-reply case passes in `conformance.mjs`. Pass 1 S1's colour and refused files both show badge 4 without opening Headers. The reset step passes. The documented immediate sheet request and body-form fallback match the amended plan.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `node examples/folder-viewer/tests/browser/conformance.mjs` | `534c3a74`, release binary rebuilt by the runner, synthetic files from the tracked test, Chromium 151.0.7922.34 | now | 0. Badge 4, four findings, first request 347 ms after first paint, resting request 303 ms after paint. Held second open, reset, colour and refused cases pass |
| `node /private/tmp/f225-codex-p2/probes.mjs` | same binary, fixture builder copied from the tracked test with imports made absolute, independent lifecycle scenarios | now, recorded second execution | 0 as an observation script. Lazy import held across a new folder: badge 4, one report cached. Two viewports: valid file badge hidden, reproducer badge 4 on return. Synthetic open: generation advanced, one report cached, badge 10. Combined old Headers repost and held import: badge hidden and request stranded |
| `node /private/tmp/f225-codex-p2/repro.mjs` | same binary, reduced combined lifecycle case | now | 1. Folder B has `due: c1.dcm`, `reports: 0`, `busy: false`, hidden badge, zero conformance requests |
| `CANDIDATE=1 node /private/tmp/f225-codex-p2/repro.mjs` | same binary with only the served HTML's null-entry branch substituted in the browser route | now | 0. Folder B has badge 4, one report cached, `due: null`. One id request answers 410, its body fallback answers 200 |
| `node /private/tmp/f225-codex-p2/repro.mjs`, after adding an explicit Close Headers click before folder B | same source and reduced case, all UI operations accessible with the sheet closed | now | 1. Same stranded request state. Log `repro-control-closed-sheet.log` |
| `node --test tools/bench/tests/viewer_work_test.mjs` | `534c3a74`, pure benchmark helpers | now, standalone command | 0. 14 tests pass |
| `python3 scripts/bench_check.py` | `534c3a74` | now | 0. 17 subjects checked |
| `git diff e8205ef0..534c3a74 --check` | reviewed range | now | 0 |

Scratch scripts and logs are under `/private/tmp/f225-codex-p2`. Every browser script closes its browser, stops its own viewer PID and removes its synthetic input directory in `finally`. The initial exploratory execution was repeated with output redirected because its session output was not retained. Only the recorded execution supplies the observations above.

## Defects

### D1, a stale aborted report still strands the next folder's report

**Where**: `examples/folder-viewer/src/page.html:5820`, `pumpChecks`, and `fetchChecks`'s `AbortError` return at line 5874.

**What**: the remediation checks `if (!entry) break` before checking whether the folder generation changed. A late operation for an old folder can return `null`, which breaks the loop with the new folder's `checks.due` still set. Its earlier timer already called `pumpChecks` while `checks.busy` was true, so no timer remains to restart the work. The new file's badge stays hidden until a later interaction requests it.

**Why it is wrong**: approved decision 14, AC33 and the state table require the current file's delayed report after a new open. The pass 1 ruling requires reading the new due entry after leaving an old open. A stale abort has the same scheduling obligation as a stale successful reply.

**Evidence**: the reduced probe uses only synthetic files and public page actions, with network delays imposed by Playwright:

1. Open folder A and hold the lazy core module import after its first image is painted and its check starts.
2. Open Headers. Its `/headers` id form answers 410 because the encoded bytes were released after decode. Hold the resulting `/file` repost after the page has removed that file's old id.
3. Close Headers and choose folder B. Let its first image paint and its checks delay elapse while the old check remains busy.
4. Release the old repost. `postFile` observes that its open was left and throws `AbortError` without restoring the old file's mapping.
5. Release the module import. The old check's `ensureFile` finds no mapping and refuses the left-open file with `AbortError`. `fetchChecks` returns `null`, and line 5820 breaks before the generation check.

Observed after a further 1.5 seconds: generation 2, shown `c1.dcm`, due `c1.dcm`, cache empty, busy false, hidden badge, no `/conformance` request. This reproduces in the exploratory run and both reduced control runs, including the closed-sheet flow.

**Scope and regression**: present on `534c3a74`. It composes unchanged F-216 retry and left-open refusal behavior with the changed F-225 scheduler. The smallest repair is to discard a stale generation before deciding whether a null entry ends the current generation's work. The scratch candidate adds that distinction inside the null branch and passes the reduced case without changing repository source. Add the closed-sheet flow to `conformance.mjs`, requiring folder B's report and badge 4. Keep the existing held successful-reply case, which guards the other path. The missed execution step is the microscope command's existing requirement to replay an adjacent refusal or recovery combination, not a missing workflow rule.

## Smells

None found in the reviewed range.

## Nitpicks

None added by this pass.

## Verified clean

- The original lost-request trigger is repaired for stale successful replies. The held-reply regression runs in the tracked browser check and passes.
- Colour and refused frame faults count as shown for Checks. The tracked probe observes badge 4 before opening the sheet in both cases.
- A held lazy import alone does not lose the new folder's request. A successful stale check is discarded, then the current file is checked.
- Activating a viewport with a valid file hides the reproducer's badge, and returning to the reproducer restores badge 4 from cache.
- Opening the synthetic study advances the generation and replaces the report cache. Its first file receives a report.
- The benchmark selector fix reads the active cell's visible `.fault`, matching the existing scroll sampler. The pure helper suite and benchmark registry check pass. This is not a first-image timing observation.
- The README changes describe the approved fault and immediate-sheet behavior. The Use and More books remain unchanged by the remediation. No parser, checker, pixel arithmetic, wasm boundary or tier behavior changed in the reviewed range.

## Limitations and handoff

- Next owner: the implementer for D1 and its permanent regression, then a fresh independent review of that repair.
- AC34 remains open for idle-host timing. No benchmark timing, GPU job, oracle run, wasm build, full workspace test, floor profile or sprint profile was run in this pass.
- The browser probe uses CPU folder-viewer rendering. It does not supply real Chrome operator acceptance or GPU evidence.
- No implementation source, plan or canonical ledger was edited. This report and the appended local progress note are the only worktree writes. No commit, push or verification receipt was made.
- This is a bounded remediation review with one blocking defect, not completed acceptance or sprint readiness.
