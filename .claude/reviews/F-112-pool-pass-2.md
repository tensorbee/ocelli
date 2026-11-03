# F-112 review, pass 2

**Reviewed**: Staged index tree `fd10b085e6587426007f084a9c1b6d0e75a4e15a` on `sprint/s40`. HEAD is `a305453330a38f516b634d872cf2eaa9249b51ba`. The review is `git diff --cached` only. `git write-tree` matched that hash at the start, after the URL and lower-bound edits were restored, and again before this file was written. The worktree matches the index. This file is unstaged.

**Author / reviewer**: staged implementation / 01a1a0c5-50bc-7bb0-94c2-2fd5c93c9eb0. This reviewer did not write the change and is not pass 1 reviewer `01a1a0b7-06fe-7162-b5ee-659feac418bf`.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC7 were checked against `.claude/plans/F-112-design.md` and HLD section 9. The count ceiling 8 is the approved local plan. It is not an HLD formula. Section 9 requires one decode instance per worker and no shared-memory threading. Tiers A, B and C are not applicable.

Pass 1 defect D1 was replayed. The repaired scan passes a string built as `fileURLToPath`, then `dirname`, then `join`. That is the same shape as `packages/core/src/viewport.test.ts`. `packages/core/src/node-test-modules.d.ts` declares `readFileSync` with a string path only. The new AS_BUILT sentence is true: "The file scan passed a URL to `readFileSync`. This tree passes a string path."

The probe derived these expectations from the plan and from the unchanged `pool.ts`:

- Counts 0, 1.5, 9, -1, `NaN`, `Infinity`, 8.5 and 10 return null and do not call start.
- Count 8 starts indexes 0 through 7. `sharedMemory` is false and `threading` is `stable`.
- A second open of the same owner returns null and does not call start. Close returns true, then false.
- During close, the slot is already gone, so a nested open from the first terminate succeeds. A throw from the middle terminate still terminates all three handles.
- A start throw at index 2 still terminates the two handles already returned. The rethrown message is `start failed`. A later open starts.
- `packages/core/src/pool.ts` does not contain the identifier `SharedArrayBuffer`. The test file does, only as the scan needle.

Those observations reject a pool that accepts a URL at the typecheck, treats 0 as one open, keeps the slot until after terminate, stops the terminate loop on the first throw, or replaces the first pool on a second open.

Changing `count < 1` to `count < 0` made the refuse test fail. Count 0 returned a pool with `count` 0. The bound was restored to `count < 1`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` and `git rev-parse HEAD` | Index before and after the temporary edits | Executed now | Exit 0. Tree `fd10b085e6587426007f084a9c1b6d0e75a4e15a`. HEAD `a305453330a38f516b634d872cf2eaa9249b51ba`. |
| `git diff` of tree `e0198a8943f89034d713c923fdcbec16e4e2a8b0` to the staged tree | Pass 1 index against this index | Executed now | `pool.ts` is absent from the diff. `pool.test.ts` replaces `new URL("./pool.ts", import.meta.url)` with `join(dirname(fileURLToPath(import.meta.url)), "pool.ts")`. AS_BUILT adds the string-path sentence. |
| `npx tsc -p packages/core --noEmit --pretty false` | `packages/core` include `src/**/*` | Executed now | Exit 0. No diagnostics. |
| Same `tsc` after the scan argument was put back to `new URL("./pool.ts", import.meta.url)` | Mutated worktree, index not updated | Executed now | Exit 2. `packages/core/src/pool.test.ts(130,33)` TS2345. `URL` is not assignable to `string`. The file was restored from the index. |
| `npx vitest run` on `pool.test.ts` and `index.test.ts` | Staged sources | Executed now | Exit 0. 12 tests passed. 8 in `pool.test.ts` and 4 in `index.test.ts`. |
| Node probe of `pool.ts` | Same staged `pool.ts` | Executed now | Exit 0. Bounds, count 8, second open, close-before-terminate, start failure and the absent identifier matched the expectations above. |
| `npx vitest run` on the refuse test after `count < 1` became `count < 0` | Mutated worktree, index not updated | Executed now | Exit 1. The refuse test expected null and received a pool with count 0. The other seven tests were skipped by the name filter. The source was restored. |
| `git restore --worktree` on `pool.ts` and `pool.test.ts`, then `git diff` and `git write-tree` | Restored from the index | Executed now | No worktree diff. Tree still `fd10b085e6587426007f084a9c1b6d0e75a4e15a`. |
| `python3 scripts/prose_check.py --staged` | 8 staged markdown paths | Executed now | Exit 0. |
| Python compare of the public-api F-ID header with the last cell of the `public-api.md` row in `docs/lld/README.md` | Staged text | Executed now | The strings are equal and both include F-112. |

Environment: Node `v24.13.0`, TypeScript `5.9.3`, and Vitest `4.1.11`. No `cargo`, `rustc`, or `wasm-pack` was started.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass 1 D1 is closed. The staged scan at `packages/core/src/pool.test.ts` lines 130 to 133 passes a string. `fileURLToPath` returns a string, `dirname` takes that string, and `join` passes the result to `readFileSync`. The declaration at `packages/core/src/node-test-modules.d.ts` line 6 accepts only a string. Clean `tsc` exits 0. The URL argument still produces TS2345, so the declaration is the check the suite does not see.
- The AS_BUILT sentence at `docs/sprints/AS_BUILT.md` line 4653 is true. The previous index passed a URL. This index passes a string path.
- AC1. Counts outside 1 to 8, including non-integers, return null and do not call start. `Number.isInteger` refuses `NaN` and `Infinity`. The lower bound is `count < 1` at `packages/core/src/pool.ts` line 47. Replacing it with `count < 0` made the refuse test fail. The source was restored.
- AC2 and AC3. Count 1 starts index 0. Count 2 starts 0 and 1. Count 8 starts 0 through 7.
- AC4. A second open of the same owner returns null and does not call start. A different object opens its own pool.
- AC5. Close sets the closed flag, deletes the map slot, then terminates. The first close returns true. The second returns false. A throw from one `terminate` does not skip the others and does not restore the slot. A nested open during the first terminate succeeds, which is the observation that the slot is already gone.
- Start failure deletes the slot, terminates handles that `start` already returned, and rethrows the start error.
- AC6. `threadingEscalation()` returns `stable`. `sharedMemory` is false. `packages/core/src/pool.ts` does not contain the identifier `SharedArrayBuffer`. The test file contains that identifier only in the scan. The product file does not construct a worker and does not touch wasm.
- AC7. `index.ts` exports `DEFAULT_DECODE_WORKERS`, `MAX_DECODE_WORKERS`, `openDecodePool` and `threadingEscalation`, plus the types `DecodePool` and `WorkerHandle`. The hand-written runtime list matches `Object.keys` of the package module. The type names are recorded as erased.
- The plan has `## LLD impact` naming `docs/lld/public-api.md`. The header F-ID list in that file equals the last cell of its row in `docs/lld/README.md`. Both include F-112.
- `pool.ts` is unchanged from the pass 1 index. The behavior above is the same implementation, re-executed on this tree.
- The staged diff has no crate, shader, or wasm path. There is no `unsafe` and no `wasm-bindgen`. Tiers A, B and C stay not applicable.
- Staged prose passed `prose_check.py --staged`.

## Limitations and handoff

This is not the feature gate and not ledger completion. `bin/ocelli.sh gate --floor` and the corpus gate were not run. No GPU and no real `Worker` URL were used. ESLint was not re-run. A function owner was not re-probed. The unit file still does not call count 8. The node probe did.

The next owner can record the feature profile on this tree. This pass did not stage or commit.
