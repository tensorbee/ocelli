# F-112 review, pass 3

**Reviewed**: Staged index tree `c11344f33e517a2c4d47de087ffaedfca7e0b797` on `sprint/s40`. HEAD is `a305453330a38f516b634d872cf2eaa9249b51ba`. The review is `git diff --cached` only. `git write-tree` matched that hash before this file was written. The worktree matches the index. This file is unstaged.

**Author / reviewer**: staged implementation / 01a1a0cd-d900-7773-8960-342a99e7aa49. This reviewer did not write the change and is not pass 1 reviewer `01a1a0b7-06fe-7162-b5ee-659feac418bf` or pass 2 reviewer `01a1a0c5-50bc-7bb0-94c2-2fd5c93c9eb0`.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The delta from pass 2 tree `fd10b085e6587426007f084a9c1b6d0e75a4e15a` to this index is the addition of `.claude/reviews/F-112-pool-pass-2.md` and one sentence in `docs/sprints/AS_BUILT.md`. `packages/core/src/pool.ts` and `packages/core/src/pool.test.ts` are absent from that diff.

The new sentence is "Pass 2 reported 0 defects, 0 smells and 0 nitpicks. Reviewer `01a1a0c5-50bc-7bb0-94c2-2fd5c93c9eb0`." The staged pass 2 result line is `**Result**: 0 defects, 0 smells, 0 nitpicks`. The staged author line names that same reviewer id. The sentence is true.

Pass 2's own line "This file is unstaged" describes tree `fd10b085e6587426007f084a9c1b6d0e75a4e15a`. It is not a claim that this later index leaves that file unstaged.

AC1 was challenged again on the unchanged pool. Counts 0, -1 and 0.5 must return null and must not call start. That rejects an empty pool and a truncation to 1. Count 8 must start indexes 0 through 7. A second open of a live owner must return null. Close must delete the slot before terminate, so a nested open from the first terminate succeeds, and a throw from the middle terminate still reaches every handle. A start throw at index 2 must rethrow `start failed` even when cleanup of index 0 throws, and a later open of that owner must start.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD` and `git write-tree` | Index before this file | Executed now | Exit 0. HEAD `a305453330a38f516b634d872cf2eaa9249b51ba`. Tree `c11344f33e517a2c4d47de087ffaedfca7e0b797`. Repeated after the probes. |
| `git diff fd10b085e6587426007f084a9c1b6d0e75a4e15a c11344f33e517a2c4d47de087ffaedfca7e0b797` | Pass 2 index against this index | Executed now | Only the pass 2 review file and the AS_BUILT sentence. No product diff. |
| Python read of the pass 2 result and author lines against the AS_BUILT sentence | Staged text | Executed now | Result line is 0 defects, 0 smells, 0 nitpicks. Author line contains `01a1a0c5-50bc-7bb0-94c2-2fd5c93c9eb0`. The new sentence is present. |
| `npx tsc -p packages/core --noEmit --pretty false` | `packages/core` include `src/**/*` | Executed now | Exit 0. No diagnostics. |
| Node probe of `openDecodePool` | Staged `packages/core/src/pool.ts` | Executed now | Exit 0. Counts 0, -1, 0.5, 1.5 and 9 returned null and did not start. Count 8 started 0 through 7 with `sharedMemory` false and `threading` `stable`. Count 1 started index 0. |
| Node probe of close, a second open, and start failure | Same staged `pool.ts` | Executed now | Exit 0. Second open returned null. Close returned true then false. Three handles each terminated once while the middle throw was swallowed. A nested open during the first terminate returned count 1. Start failure at index 2 rethrew `start failed` after terminating 0 and 1, including a throw from the first cleanup. A later count of 2 started 0 and 1. `null` returned null. |
| Source read of `pool.test.ts` lines 130 to 133 and `pool.ts` line 47 | Staged sources | Executed now | `readFileSync` receives `join(dirname(fileURLToPath(import.meta.url)), "pool.ts")`. The file has no `new URL(`. The bound is `count < 1`. |
| `python3 scripts/prose_check.py --staged` | 9 staged markdown paths | Executed now | Exit 0. |
| Pass 2 vitest, URL mutation and `count < 0` mutation | Tree `fd10b085e6587426007f084a9c1b6d0e75a4e15a` | Prior receipt | 12 tests passed. A URL argument was TS2345. `count < 0` made count 0 return a pool. Those product bytes are this tree's product bytes. This pass did not edit them and did not re-run vitest. |

Environment: Node `v24.13.0` and TypeScript `5.9.3`. No `cargo`, `rustc`, or `wasm-pack` was started. No product file was edited.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The new AS_BUILT sentence matches the staged pass 2 result line and the reviewer id on the staged author line. It is not a false claim.
- The string-path repair remains closed. `fileURLToPath` returns a string, `dirname` and `join` keep that string, and `readFileSync` is declared to take a string at `packages/core/src/node-test-modules.d.ts` line 6. `tsc` exits 0 on this tree.
- AC1. `packages/core/src/pool.ts` line 47 still returns null when `count < 1`, before the map slot is stored and before `start`. The probe observed that for 0, -1 and 0.5.
- AC2 and AC3. Count 1 starts index 0. Count 8 starts 0 through 7. `DEFAULT_DECODE_WORKERS` is 1 and `MAX_DECODE_WORKERS` is 8.
- AC4. A second open of the same owner returns null and does not replace the first pool.
- AC5. Close deletes the slot, then terminates. The first close returns true. The second returns false. One terminate throw does not skip the other handles and does not restore the slot.
- Start failure deletes the slot, terminates handles already returned, and rethrows the start error rather than a cleanup error.
- AC6. `threadingEscalation()` returns `stable`. `sharedMemory` is false. `pool.ts` does not contain `SharedArrayBuffer`. The test file uses that identifier only as the scan needle.
- AC7 was not re-run. The export lines in `index.ts` and `index.test.ts` are unchanged from the pass 2 tree, whose vitest receipt covered them.
- The staged public-api header and the last cell of the `public-api.md` row in `docs/lld/README.md` both end with F-112. Those files are unchanged from the pass 2 tree.
- HLD section 9 still has no numeric N. The ceiling 8 remains the local plan. Tiers A, B and C stay not applicable. The staged diff has no crate, shader, or wasm path.
- Staged prose passed `prose_check.py --staged`.

## Limitations and handoff

This is not the feature gate and not ledger completion. `bin/ocelli.sh gate --floor` and the corpus gate were not run. No GPU and no real `Worker` URL were used. Vitest and ESLint were not re-run. The lower bound was not mutated, because this pass does not edit product files. The live calls show the current bound refuses a count below 1. Pass 2's mutation receipt is the evidence that the refuse test goes red when that bound moves, and it applies to these same product bytes.

The next owner can record the feature profile on this tree. This pass did not stage or commit.
