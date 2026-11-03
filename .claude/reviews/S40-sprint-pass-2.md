# S40 review, pass 2

**Reviewed**: staged tree `a502ffccb676040dae0ab63efdbc681d214f9d61` against HEAD `05c7691e0343b862b0be9a1aa434b25175c84ec6`. The staged paths are `.claude/reviews/S40-sprint-pass-1.md`, `docs/sprints/AS_BUILT.md` and `packages/core/src/pool.test.ts`. `packages/core/src/pool.ts` was mutated for one run and restored. It is not in the staged diff.
**Author / reviewer**: sprint/s40 / 01a1a150-34c8-76c3-888d-c58525adef38 / independent pass 2. This reviewer did not write the sprint.
**Result**: 0 defects, 0 smells, 0 nitpicks. Pass 1's ceiling gap is closed on this staged tree. A defect or a smell would block completion. Closing S40 does not complete M13 and does not publish. The version stays 0.1.0. This pass does not mark F-X003 done.

## Acceptance challenged

F-112 says `count` must be an integer from 1 to 8 and that `start` is called once per index. Pass 1 showed that replacing `count > MAX_DECODE_WORKERS` with `count >= MAX_DECODE_WORKERS` refused 8 while the eight tests stayed green. This diff adds `starts eight workers at indexes 0 through 7`. On the restored operator that test passed, the pool count was 8, and the starter log was indexes 0 through 7. The same replacement then returned null for count 8, and that new test failed. The other eight tests stayed green. Restoring `>` made all nine tests pass. That rejects a ceiling that excludes 8. The existing refusal of 0, 1.5, 9 and -1 stayed green on both runs, so the repair does not loosen the outside-range cases.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD` and `git write-tree` | assigned HEAD `05c7691` and staged tree `a502ffcc` | executed now | Both matched at the start and after the restore. |
| `git diff --cached --name-only` | index against HEAD | executed now | The three staged paths above. No other path. |
| Count of `it(` in `packages/core/src/pool.test.ts` | staged test file | executed now | 9. The tests-added line says nine tests in that file. |
| `npx vitest run packages/core/src/pool.test.ts` | restored `count > MAX_DECODE_WORKERS`, before the mutation | executed now | exit 0. 9 passed. The new test held, so count 8 opened a pool and the starter log was 0, 1, 2, 3, 4, 5, 6 and 7. |
| Same vitest after `count >= MAX_DECODE_WORKERS` | working-tree edit at `pool.ts` line 47, not staged | executed now | exit 1. The new test failed with `expected null not to be null`. The other eight tests passed. |
| Same vitest after restoring `>` | `pool.ts` line 47 back to `count > MAX_DECODE_WORKERS` | executed now | exit 0. 9 passed. |
| `git diff`, `git write-tree` and `python3 scripts/prose_check.py --staged` | restored worktree and the staged markdown | executed now | `git diff` empty. Write-tree still `a502ffccb676040dae0ab63efdbc681d214f9d61`. Prose check OK over 2 files. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The new test is `packages/core/src/pool.test.ts` lines 64 to 71. It opens a fresh owner at count 8 through the shared starter. It expects a pool, count 8, the log `[0, 1, 2, 3, 4, 5, 6, 7]`, and a first `close` of true. That is the regression pass 1 named. `.claude/plans/F-112-design.md` says `count` is an integer from 1 to 8 and that `start` is called once per index. The log is that call list. `MAX_DECODE_WORKERS` stays 8. The outside-range test still covers 0, 1.5, 9 and -1 and still expects no start.

The nine `it` calls are the refusal test, one worker, two workers, eight workers, a second open, a different owner, close, a thrown start, and the stable default. Vitest reported 9 passed on the restored file and 8 passed with 1 failed under the ceiling mutation. The failed name was `starts eight workers at indexes 0 through 7`.

The F-112 note says pass 1 reported 1 defect from reviewer `01a1a140-d410-7150-a74d-4ecbeac31101`. The staged pass 1 review says that. The note says replacing the ceiling check with `>=` left the eight tests green and refused count 8. The mutation run did that. The note says this tree opens eight workers and records indexes 0 through 7. Both green runs did that. The note still says closing S40 does not complete M13 and does not publish. `docs/sprints/CURRENT_SPRINT.md` says the same and is not in this diff.

This diff does not edit `Cargo.toml`, `packages/core/src/index.ts` or `docs/sprints/BACKLOG.md`. The version and the F-X003 row were not re-opened. Pass 1's decode-graph, admission, address-limit and SwiftShader claims were not re-run. Those files are not in this diff. The staged pass 1 review still describes tree `a0c22f39` and says that review file is not part of that tree.

The restored comparison is `count > MAX_DECODE_WORKERS`. The mutation was not left in the worktree. The staged markdown has no em-dash and no prose semicolon.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate`, `wasm-pack`, cargo or a browser. It did not rebuild wasm. It did not re-run the decode-graph check, the cache admission mutation, the address-limit mutation or the SwiftShader mutation. Feature-profile sentences in `AS_BUILT.md` remain prior receipts. No sprint profile was recorded for this HEAD or for this staged tree.

This pass did not stage this review file, did not commit and did not push. The file is outside `git write-tree`. That hash stays `a502ffccb676040dae0ab63efdbc681d214f9d61`. A later index that adds this file is a different tree. Closing S40 does not complete M13 and does not publish. Do not mark F-X003 done.
