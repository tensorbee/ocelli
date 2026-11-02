# F-092 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s34`. HEAD is `599ddfc` (`S34, approve the utility designs`). Read in full: `.claude/plans/F-092-design.md`, `packages/core/src/crop.ts`, `packages/core/src/crop.test.ts`. Also read the kept-side comment in `crates/ocelli-render/src/clip.rs`, `GESTURE_SCALE` in `packages/core/src/pointer.ts`, and the export list in `packages/core/src/index.ts`. `docs/lld/public-api.md` does not mention F-092. The product files are untracked.
**Author / reviewer**: The author is the parent implementation agent. The reviewer is independent subagent `01a19e3d-e6e5-7aa1-825b-ee18700c15f2`. The reviewer did not write the plan or the diff.
**Result**: 1 defect, 1 smell, 0 nitpicks

## Acceptance challenged

The approved plan is the contract. The HLD does not define a crop drag. That absence is not a defect. F-062 owns the kept side. This module does not copy that test and does not call `set_clip_planes`.

AC1. One plane at the origin with normal `(0, 0, 1)` must come back as a copy. The test expects deep equality. It also rejects the same list and the same plane object. Returning the caller list fails `not.toBe`.

AC2. An empty list must be an empty copy. The test expects `[]` and rejects the same array. Null for the empty list fails `toEqual([])`.

AC3. Seven planes, a zero normal, and a `NaN` component must each be null. The count check runs before the copy. Storing a seventh plane fails the first expect. A zero normal and a `NaN` each fail the later expects.

AC4. Origin 0, normal `(0, 0, 1)`, drag from 0 to 256. The delta is `256 / 256`, which is 1. z is `0 + 1 * 1`, so the origin is `(0, 0, 1)`. Reversing the drag sign made this expect red. The received value was `(0, 0, -1)`.

AC5. The same plane with last x at `-256` must be `(0, 0, -1)`. The same sign reversal made this expect red. The received value was `(0, 0, 1)`.

AC6. A zero normal on the drag must be null. The same test refuses a `NaN` origin component and a sum of `Number.MAX_VALUE` with `Number.MAX_VALUE`. Those match the non-finite input rule and the non-finite result rule. The sum is not finite either with or without the squared-length guard, so that expect does not lock the guard.

The normal is not normalised in the body. AC4 cannot show that. Its normal has length 1, so dividing by the length still returns `(0, 0, 1)`. That gap is S1.

`cropDrag` also returns null when the squared length overflows. That is D1. `cropPlanes` must refuse the same normal, because the planes paragraph requires a finite squared length greater than 0.

## Evidence

| Check | Identity | Exit and observation |
|---|---|---|
| Pre-edit `shasum -a 256` | `packages/core/src/crop.ts` | `4dca4703e15c1da5737f4442698de1733aefe5aaf762d9210b8a6c70e206aee7` |
| Node probe of the pre-edit module | `node --experimental-strip-types` importing `crop.ts`. No suite edit. | Exit 0. Six planes copy, and normal `(0, 4, 0)` stays `(0, 4, 0)`. The list, the plane, and both triples are new objects. An empty list is a new empty array. A frozen plane does not throw and its origin stays `[9, 8, 7]`. `cropDrag([3, 4, 5], [0, 0, 2], 10, 266)` is `[3, 4, 7]` and does not write the inputs. `cropDrag([0, 0, 0], [0, 0, -2], 0, 256)` is `[0, 0, -2]`. `cropDrag([0, 0, 0], [Number.MAX_VALUE, 0, 0], 0, 256)` is null. |
| `npm test -- packages/core/src/crop.test.ts` after the sign flip | One line in `crop.ts`: `const step = (originX - lastX) / DRAG_SCALE` | Exit 1. Vitest 4.1.11. 2 failed and 4 passed. AC4 expected `[0, 0, 1]` and received `[0, 0, -1]`. AC5 expected `[0, 0, -1]` and received `[0, 0, 1]`. |
| `cp` of the pre-edit copy, then `cmp` and `shasum -a 256` | `/tmp/f092-crop.ts.orig` against `packages/core/src/crop.ts` | `cmp` exit 0. Both hashes are `4dca4703e15c1da5737f4442698de1733aefe5aaf762d9210b8a6c70e206aee7`. The minus form is absent. |
| Same npm command on the restored file | Restored step is `(lastX - originX) / DRAG_SCALE` | Exit 0. 6 passed. The hash was unchanged after the run. |

No cargo, no wasm-pack, and no `bin/ocelli.sh`. The test file was not edited. Its pre-edit hash is `9b2f34740032b7c3f717a353e046e93ebeef9e95192c9cd787dc4e4faf90e7db`.

The restored step is:

```ts
const step = (lastX - originX) / DRAG_SCALE;
```

## Defects

### D1. `cropDrag` returns null when a finite normal's square overflows

File: `packages/core/src/crop.ts` lines 84 to 87.

Trigger: `cropDrag([0, 0, 0], [Number.MAX_VALUE, 0, 0], 0, 256)` on the pre-edit bytes.

Expected: the drag paragraph adds `normal * (lastX - originX) / 256` and returns nothing only for a zero normal, a non-finite input, or a non-finite result. Every input here is finite. The normal is not the zero vector. The step is `256 / 256`, which is 1. `Number.MAX_VALUE * 1` is `Number.MAX_VALUE`, and the other two components stay 0. That origin is finite, so the call returns `[Number.MAX_VALUE, 0, 0]`.

Actual: the call returns null. `Number.MAX_VALUE * Number.MAX_VALUE` is not finite, and `!Number.isFinite(lengthSquared)` returns before the addition. The planes paragraph does require a finite squared length, so `cropPlanes` is right to refuse this normal. The drag paragraph does not use that rule. The copied predicate drops a result the drag formula defines.

AC6 does not catch this. Its `Number.MAX_VALUE` expect adds the origin and the normal, and that sum is not finite anyway.

## Smells

### S1. The suite does not lock a non-unit normal or a list of six

AC4 names a normalised step of a different size as the wrong implementation it rejects. The fixture normal is `(0, 0, 1)`. Its length is 1, so a body that divides by the length still returns `(0, 0, 1)` and, for AC5, `(0, 0, -1)`. The restored body does not divide by the length. The probe moved normal `(0, 0, 2)` by 2. That substitution was not run inside the suite.

The plan accepts 0 to 6 planes. The suite calls lengths 0, 1, and 7. A comparison of `planes.length >= 6` would still pass every expect. The restored comparison is `planes.length > 6`. The probe copied six planes. That bound was not substituted inside the suite.

## Nitpicks

None.

## Verified clean

`cropPlanes` returns a new list of 0 to 6 planes. Each returned origin and normal is a new triple. A missing component is null. A non-finite component is null, including `Infinity` in the probe. A zero normal is null, including a normal of all `-0` in the probe. An empty list is a new empty array, not null. Seven planes are null, and the function does not return a partial copy. The caller list is not written. A frozen plane was copied and its origin stayed `[9, 8, 7]`.

`cropDrag` uses `DRAG_SCALE`, and that constant is the literal 256. The step is `(lastX - originX) / 256`. The normal is multiplied as given. It is not normalised, and a negative normal keeps its sign. A zero normal is null. A non-finite `originX` or `lastX` is null. A non-finite component is null. A non-finite sum is null. The function returns a new triple and does not write the caller origin or the caller normal. It does not return the normal.

The module has no `unsafe` and no `wasm-bindgen`. It does not branch on tier A, B, or C. Tiers are not applicable, which matches the plan. There is no registry entry and no new command kind. The source has no dot product. The text `set_clip_planes` appears only in the comment that says this module does not call it. `GESTURE_SCALE` in `packages/core/src/pointer.ts` is 256, and `DRAG_SCALE` is a local copy of that F-069 scale rather than an import.

`packages/core/src/index.ts` does not export this module. That absence is a later slice and is not a defect.

## Limitations and handoff

This pass ran only `packages/core/src/crop.test.ts` plus one node probe of `crop.ts`. It did not run `index.test.ts`, lint, typecheck, cargo, wasm-pack, `bin/ocelli.sh`, a gate, the oracle, or the corpus. The feature profile named by the plan was not run. The normalised body and a six-plane refusal were not substituted into the suite.

The plan names a paragraph in `docs/lld/public-api.md` and the public-api row in `docs/lld/README.md`. Neither file mentions F-092. This pass did not count that absence as a defect. The reviewed product is the two untracked modules.

No patient data was read. dwv, Horos, and the medical viewer Grok were not opened.

The sign mutation is not left in place. `cmp` against the pre-edit copy exited 0. The restored hash is `4dca4703e15c1da5737f4442698de1733aefe5aaf762d9210b8a6c70e206aee7`. This review file is the only file this pass added. It was not committed.
