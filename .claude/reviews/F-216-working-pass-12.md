# F-216 review, pass 12

**Reviewed**: `git diff 051a5c4 952e9ad` on `work/f-216-xai`, HEAD `952e9ad38a88713ebb62120d5372eb2eeba4c5bd`. One file, `tools/bench/tests/run_test.mjs`, 8 insertions and 4 deletions. No unstaged tracked changes. Untracked at start and end: the eleven earlier review reports and `tools/oracle/out`.
**Author / reviewer**: the sprint lead (Claude) wrote 952e9ad. This pass is a separate Claude reviewer subagent that did not write it. Both are the same model, which limits how independent this review is.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Pass 11 S1, the test must not run the real bench when main skips the refusal.** I worked out the expected result from `run.mjs:316-351`. `resolveRun` is called once, at line 334, before the `options.list` return at line 340, and `parseArgs` has no rule that `--list` conflicts with `--accept`, `--subject`, `--tolerance` or positional ids. So adding `--list` cannot change which refusal fires or its text. A main that skips the refusal must then print the listing and return 0, with no call to `runSubjects`. Both mutations below behaved that way.
- **Pass 11 N2, cleanup inside `finally`.** Expected: a throw from `main` still propagates, `process.stderr.write` is restored, and the temp dir is removed. The test's assertions run after `mainRefusal` returns, so the `finally` cannot mask them. `code` is set inside `try` and read after the `finally`, so it is kept.
- **Does the `outCreated` assertion still hold anything now that `--list` is added?** Yes. It is red for a main that creates `--out` before refusing, and the third mutation shows it.

## Evidence

All mutations ran in a scratch copy, `target/p12-review/tree`, extracted with `git archive 952e9ad`. It had no `node_modules`, which matches CI, and `runSubjects` was left real. I deleted it afterwards.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `node --test --test-name-pattern="main refuses" tools/bench/tests/run_test.mjs`, unmutated | scratch at 952e9ad | now | pass, 5.7 ms |
| S1 mutation from pass 10: `resolveRun(loaded, { ...options, tolerances: {} })` in `main` | scratch | now | 1 fail out of 27. `viewer.first_image ... must exit 2`, actual 0. Test 5.7 ms, whole process 0.08 s. No file newer than the start marker except `run.mjs`. No new viewer, cargo, vite or chromium process. |
| `--subject` check replaced with `if (false)` | scratch | now | 2 fail (the `resolveRun` unit test and the main test, actual 0). Test 7.0 ms. No file written, no new process. |
| `main` throws `probe-throw-p12` when `--list` is in argv | scratch | now | red with `Error: probe-throw-p12`, so the error is not masked. No new `ocelli-bench-main-*` dir in `os.tmpdir()`. |
| `await mkdir(options.out, ...)` moved before `loadRegistry()` | scratch | now | red, so `outCreated` still guards that ordering |
| scratch `ci/*.json` checksums | scratch | now | unchanged by every run |
| `node --test tools/bench/tests/run_test.mjs tools/bench/tests/record_test.mjs` | worktree at HEAD | now | 56 tests, 55 pass, 0 fail, 1 skip (`release worker instrument ...`, opt-in) |
| `bin/ocelli.sh gate bench` | worktree | now | ALL GREEN, 1 gate, 2 skipped |
| `python3 scripts/guard_census.py` | worktree | now | exit 0 |
| `python3 scripts/guard_census.py --check-runbook` | worktree | now | exit 0 |
| `python3 scripts/prose_check.py` | worktree | now | exit 0, clean over 840 files |
| `python3 scripts/source_provenance_check.py` | worktree | now | exit 0, clean over 1677 files |
| `git status --porcelain`, and `shasum` of `ci/*.json` and `tools/bench/subjects.json` | worktree | start and end of pass | identical |

The bench itself was not run. No oracle or GPU job ran. Ports 8742 and 8765 were not used, and no process was started outside the node test runs. PID 98891 (`./target/debug/ocelli-folder-viewer`) was there before this pass and was left alone.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, main's refusal is now held only on the `--list` route

The refusal sits at one call site above the branch, so today the two routes cannot differ. A future change that resolves the run differently in the non-list branch would not be caught by this test. I did not find a way to hold the non-list route without the risk pass 11 S1 described, and that tradeoff is acceptable.

### N2, carried from pass 11 N1: nothing tests the registry-fault exit code

Unchanged by this diff.

### N3, carried from pass 11 N3 (pass 10 N4): a named subject the run cannot measure is dropped silently

Unchanged and still unreachable with the tracked registry.

## Verified clean

- **Pass 11 S1 closed.** Both required mutations go red in under 10 ms with nothing built, no viewer started and no file written outside a temp dir. The tracked `ci/*.json` files are unchanged.
- **Pass 11 N2 closed.** The `rm` is inside the `finally`, and a throw from `main` leaves no temp dir. The two `ocelli-bench-main-*` dirs already in `os.tmpdir()` (`wxeaPQ` and `xswxK0`) predate this pass and the count stayed at two.
- **`--list` changes neither the refusal nor its text.** The three anchored `^...\n$` stderr expectations pass unchanged, including `unknown subject viewer.absent`.
- **The new comment sentences are true.** "resolveRun runs before main's --list return" matches `run.mjs:334` against `:340`. "a main that skipped the refusal lists the subjects and returns rather than running the bench" matches what both mutations showed: exit 0, no build, no file written. The earlier sentence, "main must refuse before runSubjects, so the --out directory is never created", is still true and is held by the `outCreated` assertion (fourth mutation).
- **The `finally` cannot mask or drop anything it should not.** Assertions are outside the helper. `code` survives. `stderr.write` is restored before the `rm`. A failing `rm` could only replace an error from `main` if the directory `mkdtemp` created had gone missing, which nothing in `main` does.
- **The commit message** has no em-dash, en-dash or semicolon.
- **Docs.** The diff touches only a test, so README.md, `use.html`, `more.html` and `docs/lld/benchmarks.md` are not affected.

## Limitations and handoff

- The bench was not run, and no GPU or oracle job ran.
- This is a bounded review of one commit, 952e9ad. It is not AC17 acceptance and not sprint readiness.
- N1 to N3 do not block.
