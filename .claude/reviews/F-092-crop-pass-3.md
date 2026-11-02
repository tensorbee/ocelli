# F-092 review, pass 3

**Reviewed**: uncommitted working tree on `sprint/s34`. HEAD is `599ddfc` (`S34, approve the utility designs`). Read in full: `.claude/plans/F-092-design.md`, `.claude/reviews/F-092-crop-pass-2.md`, `packages/core/src/crop.ts`, and `packages/core/src/crop.test.ts`. The product files are untracked. The pre-edit `crop.ts` sha256 matched `4f145f47e977fb2ee0effbe785e94811fb899c802d50fb75e3b56fe13d133340`, so this pass did not stop.
**Author / reviewer**: The author is the parent implementation agent. The reviewer is independent subagent `01a19e53-86b4-74e0-8ee7-8dad23ed16c7`. The reviewer did not write the repair and did not resume pass 1 or pass 2.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

The approved plan is the contract. The HLD does not define a crop drag. That absence is not a defect. F-062 owns the kept side. This module does not copy that test and does not call `set_clip_planes`.

Pass 2 S1 is closed. The planes paragraph says the squared length must be finite and greater than 0. A normal of `[Number.MAX_VALUE, 0, 0]` has finite components and a square that is not finite. The repair adds one expect, at `packages/core/src/crop.test.ts` lines 27 to 29, that `cropPlanes` returns null for that plane. `crop.ts` was not part of the repair.

Deleting `!Number.isFinite(lengthSquared) ||` from `copyPlane` only left `!(lengthSquared > 0)`. That comparison stays false for an infinite square, so the mutated function copied the plane. The focused npm command then exited 1. One test failed and five passed. The failure is the new expect. The received value was one plane with origin `[0, 0, 0]` and normal `[Number.MAX_VALUE, 0, 0]`. The drag test stayed green on that same run, so the mutation did not change `cropDrag([0, 0, 0], [Number.MAX_VALUE, 0, 0], 0, 256)`.

On the restored bytes, that planes call is null again. A node probe of the restored module returned `[Number.MAX_VALUE, 0, 0]` for the drag. Pass 1 D1 stays closed. The drag paragraph does not refuse a finite square, and the drag body still has no squared-length check.

AC1 through AC6 remain the expects pass 2 challenged. This pass did not repeat the sign, length, or six-plane mutations. The restored suite exited 0 with six passed tests. The new expect sits in the case that already refuses seven planes, a zero normal, and a `NaN` component. The red run failed at the new expect, after those earlier expects in the same case had passed.

## Evidence

| Check | Identity | Exit and observation |
|---|---|---|
| Pre-edit `shasum -a 256` | `packages/core/src/crop.ts` before any edit | `4f145f47e977fb2ee0effbe785e94811fb899c802d50fb75e3b56fe13d133340` |
| `npm test -- packages/core/src/crop.test.ts` after the finite-square check was removed from `copyPlane` only | `if (!(lengthSquared > 0))`. One occurrence, before `cropPlanes` and `cropDrag`. | Exit 1. Vitest 4.1.11. 1 failed and 5 passed. The new expect received the plane. The stack is `crop.test.ts:29`. |
| `cmp` and `shasum -a 256` after restore | `/tmp/f092-pass3-crop.ts.orig` against `packages/core/src/crop.ts` | `cmp` exit 0. Hash `4f145f47e977fb2ee0effbe785e94811fb899c802d50fb75e3b56fe13d133340`. |
| Node probe of the restored module | `node --experimental-strip-types` importing `crop.ts` | Exit 0. `cropDrag([0, 0, 0], [Number.MAX_VALUE, 0, 0], 0, 256)` is exactly `[Number.MAX_VALUE, 0, 0]`. The same normal is null in `cropPlanes`. |
| Same npm command on the restored file | Finite-square check present again | Exit 0. 6 passed. The hash was unchanged after the run. |

Node v24.13.0. No cargo, no wasm-pack, and no `bin/ocelli.sh`. This pass did not edit the test file. Its hash stayed `843a79be10006402a728df507351eadf673a73e019d2252a00b7e5fc9b1f890c`.

The restored predicate is:

```ts
if (!Number.isFinite(lengthSquared) || !(lengthSquared > 0)) {
```

## Defects

None.

## Smells

None. Pass 2 S1 is closed.

## Nitpicks

### N1. The case title still names only a non-finite component

File: `packages/core/src/crop.test.ts` line 21.

The new expect refuses a finite normal whose square is not finite. The `it` title still says the case refuses a non-finite component. `Number.MAX_VALUE` is finite. The assertion at lines 27 to 29 is the squared-length lock, and the red run failed there. The title does not let the pass 2 deletion stay green.

## Verified clean

`crop.ts` matches the pass 2 restored bytes. `copyPlane` still returns null when `lengthSquared` is not finite or is not greater than 0. `cropPlanes` returns that null for the whole call. `cropDrag` still uses `DRAG_SCALE` of 256 and the step `(lastX - originX) / DRAG_SCALE`. It does not test the square. The probed finite overflow normal stays an origin of `[Number.MAX_VALUE, 0, 0]` when the pixel delta is 256.

The module has no `unsafe` and no `wasm-bindgen`. It does not branch on tier A, B, or C. The source has no dot product and no `Math.hypot`. The text `set_clip_planes` appears only in the comment that says this module does not call it. `packages/core/src/index.ts` does not name this module. That absence is not a defect.

## Limitations

This pass ran only `packages/core/src/crop.test.ts` plus one node probe of the restored `crop.ts`. It replayed the pass 2 S1 deletion and did not repeat the sign, unit-length, or six-plane mutations. It did not run lint, typecheck, cargo, wasm-pack, `bin/ocelli.sh`, a gate, the oracle, or the corpus. The feature profile named by the plan was not run.

The plan names a paragraph in `docs/lld/public-api.md` and the public-api row in `docs/lld/README.md`. Neither file names F-092. This pass does not count that absence as a defect. The reviewed repair is the one new expect.

No patient data was read. dwv, Horos, and the medical viewer Grok were not opened.

The mutation is not left in place. `cmp` against the pre-edit copy exited 0. The restored hash is `4f145f47e977fb2ee0effbe785e94811fb899c802d50fb75e3b56fe13d133340`. This review file is the only file this pass added. It was not committed.
