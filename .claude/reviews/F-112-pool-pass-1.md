# F-112 review, pass 1

**Reviewed**: Staged index tree `e0198a8943f89034d713c923fdcbec16e4e2a8b0` on `sprint/s40`. HEAD is `a305453330a38f516b634d872cf2eaa9249b51ba`. The review is `git diff --cached` only. `git write-tree` matched that hash at the start and again after the lower-bound edit was restored. The worktree matches the index.

**Author / reviewer**: staged implementation / 01a1a0b7-06fe-7162-b5ee-659feac418bf. This reviewer did not write the change.

**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC7 were checked against `.claude/plans/F-112-design.md` and HLD section 9, not against the test titles. The count ceiling 8 is the approved local plan. It is not an HLD formula. Section 9 requires one decode instance per worker and no shared-memory threading. It does not set N. Tiers A, B and C are not applicable. No new deviation is in the staged diff.

The unit file locks refusal of 0, 1.5, 9 and -1, one start at index 0, two starts at 0 and 1, a second open of the same owner, a second owner, one terminate each with a throw on the first worker, a second close of false, start failure at index 1, and the stable decision. It does not lock count 8, `NaN`, a reopen after close, or a terminate throw during start cleanup.

The probe derived these expectations from the plan:

- Counts 0, 1.5, 9, -1, `NaN`, `Infinity`, 8.5 and 10 return null and do not call start. A later count of 1 on that owner starts index 0.
- Count 8 starts indexes 0 through 7. `sharedMemory` is false and `threading` is `stable`. A second open does not call start.
- Close returns true, each terminate runs once, and the second close returns false. The same owner can open again.
- During close, the slot is already gone, so a nested open from the first terminate succeeds. The old close stays true and does not terminate the new worker.
- A terminate throw on the middle of three workers still terminates all three. Close returns true. The slot stays deleted. A later open starts.
- A start throw at index 0 rethrows `start failed` and leaves the owner free.
- A start throw at index 2 still terminates indexes 0 and 1 when the first terminate also throws. The rethrown message is `start failed`, not the terminate message. A later open of count 2 starts 0 and 1.
- `null`, `undefined`, a number and a string do not start.

Those observations reject a pool that treats 1.5 as 1, keeps a slot after close or after a start throw, stops the terminate loop on the first throw, or replaces the first pool on a second open.

Changing `count < 1` to `count < 0` made count 0 return a pool with `count` 0. The start log stayed empty because the loop does not run. The refuse test failed. The bound was restored to `count < 1`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` and `git rev-parse HEAD` | Index before review | Executed now | Exit 0. Tree `e0198a8943f89034d713c923fdcbec16e4e2a8b0`. HEAD `a305453330a38f516b634d872cf2eaa9249b51ba`. |
| `npx vitest run` on `pool.test.ts`, `index.test.ts` and `events.test.ts` | Staged sources | Executed now | Exit 0. 20 tests passed. 8 in `pool.test.ts`, 4 in `index.test.ts`, 8 in `events.test.ts`. |
| Node probe of `pool.ts` | Same staged `pool.ts` | Executed now | Exit 0. Bounds, slot deletion, terminate throw and start failure matched the expectations above. |
| Same vitest files for pool and index after `count < 1` became `count < 0` | Mutated worktree, index not updated | Executed now | Exit 1. `refuses a count outside 1 to 8 and does not start` expected null and received a pool with count 0. The other 11 tests passed. |
| Node probe of count 0 and -1 on that mutation | Same mutated worktree | Executed now | Count 0 returned a pool and did not call start. Count -1 stayed null. |
| `git restore --worktree -- packages/core/src/pool.ts`, then `git diff` and `git write-tree` | Restored from the index | Executed now | No worktree diff. Tree still `e0198a8943f89034d713c923fdcbec16e4e2a8b0`. |
| `npx tsc -p packages/core --noEmit --pretty false` | `packages/core` include `src/**/*` | Executed now | Exit 2. `pool.test.ts(128,33)` TS2345. `URL` is not assignable to `string`. |
| `npx eslint` on `pool.ts`, `pool.test.ts`, `index.ts` and `index.test.ts` | Those four files | Executed now | Exit 0. |
| `npx vitest run` on `pool.test.ts` and `index.test.ts` after restore | Restored staged sources | Executed now | Exit 0. 12 tests passed. |
| `python3 scripts/prose_check.py --staged` | 7 staged markdown paths | Executed now | Exit 0. |
| Python compare of the public-api F-ID header with the last cell of the `public-api.md` row in `docs/lld/README.md` | Staged text | Executed now | The strings are equal and both include F-112. |

Environment: Node `v24.13.0`, TypeScript `5.9.3`, Vitest `4.1.11`. No `cargo`, `rustc`, or `wasm-pack` was started.

## Defects

### D1, the source scan does not typecheck

**Where**: `packages/core/src/pool.test.ts:128`

**What**: `readFileSync` is called with `new URL("./pool.ts", import.meta.url)`. The package declaration of `readFileSync` accepts only a `string`. `tsc -p packages/core --noEmit` exits 2 with TS2345.

**Why it is wrong**: `packages/core/src/node-test-modules.d.ts` line 6 is the `node:fs` surface this package typechecks against. `packages/core/tsconfig.json` includes `src/**/*`, so the test is part of `tsc --build`. That command is the `types` gate in `bin/ocelli.sh` and the rebuild the design names before the floor gate. Vitest strips types, so the green suite does not see this. Sibling scans pass a string from `fileURLToPath`.

**Evidence**: `npx tsc -p packages/core --noEmit --pretty false` printed `packages/core/src/pool.test.ts(128,33): error TS2345: Argument of type 'URL' is not assignable to parameter of type 'string'.` Exit 2. The same file's vitest run exited 0.

**Scope and regression**: The pool functions are not the failure. The types gate cannot pass on this tree. The smallest change is to pass the string path the declaration already accepts. Present on the staged tree. No uncertainty about the diagnostic.

## Smells

None.

## Nitpicks

None.

## Verified clean

- AC1. Counts 0, 1.5, 9 and -1 return null and do not call start. `Number.isInteger` also refuses `NaN` and `Infinity`. The lower bound is `count < 1`. Replacing it with `count < 0` made the refuse test fail and made count 0 return a pool. The source was restored.
- AC2 and AC3. Count 1 starts index 0. Count 2 starts 0 and 1. Count 8 starts 0 through 7.
- AC4. A second open of the same owner returns null and does not call start. A different object, including a distinct array, opens its own pool.
- AC5. Close deletes the map slot before `terminate`. The first close returns true. The second returns false and does not terminate again. A throw from one `terminate` does not skip the others and does not restore the slot.
- Start failure deletes the slot, terminates handles that `start` already returned, and rethrows the start error. A throw from `terminate` during that cleanup does not replace the start error.
- AC6. `threadingEscalation()` returns `stable`. `sharedMemory` is false. `packages/core/src/pool.ts` does not contain the identifier `SharedArrayBuffer`. The test file contains that identifier only in the scan. The product file does not construct a worker, does not name a GPU device, and does not touch wasm.
- AC7. `index.ts` exports `DEFAULT_DECODE_WORKERS`, `MAX_DECODE_WORKERS`, `openDecodePool` and `threadingEscalation`, plus the types `DecodePool` and `WorkerHandle`. The hand-written runtime list matches `Object.keys` of the package module. The type names are recorded as erased.
- The plan has `## LLD impact` naming `docs/lld/public-api.md`. The header F-ID list in that file equals the last cell of its row in `docs/lld/README.md`. Both include F-112.
- `docs/sprints/AS_BUILT.md` uses the house verification paragraph. It does not claim a feature-gate receipt. `docs/sprints/SPRINT_TRACKER.md` records a count-bound mutation and does not claim a staged feature verification.
- The staged diff has no crate, shader, or wasm path. There is no `unsafe` and no `wasm-bindgen`. Tiers A, B and C stay not applicable. `events.test.ts` still sees no new listener and no second event catalogue.
- `npx eslint` on the four TypeScript files exited 0. Staged prose passed `prose_check.py --staged`.

## Limitations and handoff

This is not the feature gate and not ledger completion. `bin/ocelli.sh gate --floor` and the corpus gate were not run. No GPU and no real `Worker` URL were used. A function owner returns null because `typeof` is `function`. That case is outside the plan's listed refusals and was only probed.

The next owner should make the source scan typecheck, then run the types gate on the repaired tree. A fresh independent pass has to review that repair. This pass did not stage or commit.
