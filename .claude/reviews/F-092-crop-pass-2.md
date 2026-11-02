# F-092 review, pass 2

**Reviewed**: uncommitted working tree on `sprint/s34`. HEAD is `599ddfc` (`S34, approve the utility designs`). Read in full: `.claude/plans/F-092-design.md`, `.claude/reviews/F-092-crop-pass-1.md`, `packages/core/src/crop.ts`, and `packages/core/src/crop.test.ts`. Also read the kept-side comment in `crates/ocelli-render/src/clip.rs`, `GESTURE_SCALE` in `packages/core/src/pointer.ts`, the export list in `packages/core/src/index.ts`, and the contributed-id line in `docs/lld/public-api.md`. The product files are untracked. The pre-edit `crop.ts` sha256 matched `4f145f47e977fb2ee0effbe785e94811fb899c802d50fb75e3b56fe13d133340`, so this pass did not stop.
**Author / reviewer**: The author is the parent implementation agent. The reviewer is independent subagent `01a19e49-a57f-7591-9284-d9848a41806e`. The reviewer did not write the repair and did not resume pass 1.
**Result**: 0 defects, 1 smell, 0 nitpicks

## Acceptance challenged

The approved plan is the contract. The HLD does not define a crop drag. That absence is not a defect. F-062 owns the kept side. This module does not copy that test and does not call `set_clip_planes`.

Pass 1 D1 is closed. `cropDrag([0, 0, 0], [Number.MAX_VALUE, 0, 0], 0, 256)` returns `[Number.MAX_VALUE, 0, 0]`. The delta is `256 / 256`, which is 1. `Number.MAX_VALUE * 1` is `Number.MAX_VALUE`, and the other two components stay 0. Every input is finite, the normal is not the zero vector, and the result is finite. The drag body no longer refuses a finite squared length. Restoring that refusal made the new expect red. The received value was null. The other five tests passed. On the restored bytes, `cropPlanes` still returns null for that same normal, because the planes paragraph requires a finite squared length greater than 0. The suite does not lock that planes refusal. That gap is S1 below.

Pass 1 S1 is closed. A normal of `(0, 0, 2)` dragged 256 to the right returns `[0, 0, 2]`, which is `0 + 2 * 1`, not a unit step. Dividing each component by `Math.hypot` made that expect red. The received value was `[0, 0, 1]`. The unit normal in the same test stayed green, which is why the length-1 fixture could not lock this. Six planes return a list of length 6. Seven return null. Replacing `planes.length > 6` with `planes.length >= 6` made the length expect red, because the call returned null.

AC1. One plane at the origin with normal `(0, 0, 1)` comes back as a copy. The test expects deep equality and rejects the same list and the same plane object.

AC2. An empty list is an empty copy. The test expects `[]` and rejects the same array.

AC3. Seven planes, a zero normal, and a `NaN` component are each null. The six-plane expect now sits in that test, so a bound of `>= 6` fails before the seven-plane expect.

AC4. Origin 0, normal `(0, 0, 1)`, drag from 0 to 256. The delta is 1, so z is `0 + 1 * 1`. The same test now also expects z `2` for normal `(0, 0, 2)`.

AC5. The same unit plane with last x at `-256` is `(0, 0, -1)`. Reversing the drag sign made this expect red. The received value was `(0, 0, 1)`.

AC6. A zero normal on the drag is null. The same test refuses a `NaN` origin and a sum of `Number.MAX_VALUE` with `Number.MAX_VALUE`. It also expects the finite `Number.MAX_VALUE` normal to be kept. That is the D1 lock.

## Evidence

| Check | Identity | Exit and observation |
|---|---|---|
| Pre-edit `shasum -a 256` | `packages/core/src/crop.ts` before any edit | `4f145f47e977fb2ee0effbe785e94811fb899c802d50fb75e3b56fe13d133340` |
| `npm test -- packages/core/src/crop.test.ts` | Restored drag. No squared-length check in `cropDrag`. | Exit 0. Vitest 4.1.11. 6 passed. |
| Same command after the drag zero check was replaced by the planes squared-length refusal | `!Number.isFinite(lengthSquared) \|\| !(lengthSquared > 0)` inside `cropDrag` | Exit 1. 1 failed and 5 passed. The `Number.MAX_VALUE` expect received null. The `(0, 0, 2)` expect stayed green. |
| Same command after the step sign was reversed | `const step = (originX - lastX) / DRAG_SCALE` | Exit 1. 3 failed and 3 passed. AC4 received `[0, 0, -1]`. AC5 received `[0, 0, 1]`. The `Number.MAX_VALUE` expect received the negation. The unit expect failed before the `(0, 0, 2)` expect in that test. |
| Same command after each component was divided by `Math.hypot` | Unit length applied before the scale | Exit 1. 2 failed and 4 passed. The `(0, 0, 2)` expect received `[0, 0, 1]`. The `Number.MAX_VALUE` expect received `[1, 0, 0]`. AC5 stayed green. |
| Same command after the count check became `>= 6` | One comparison in `cropPlanes` | Exit 1. 1 failed and 5 passed. `toHaveLength(6)` received null. |
| Same command after the planes finite-square check was removed | `!(lengthSquared > 0)` kept, `Number.isFinite(lengthSquared)` deleted | Exit 0. 6 passed. A node probe then copied the `Number.MAX_VALUE` normal instead of returning null. |
| Node probe of the restored module | `node --experimental-strip-types` importing `crop.ts`. 45 checks. | Exit 0. 0 mismatches. The D1 trigger is `[Number.MAX_VALUE, 0, 0]`. The same normal is null in `cropPlanes`. `(0, 0, 2)` dragged 256 is `[0, 0, 2]`. Six planes are a new list of six new planes and six new triples. Seven are null. |
| `cmp` and `shasum -a 256` after every restore | `/tmp/f092-crop.ts.orig` against `packages/core/src/crop.ts` | `cmp` exit 0 after each mutation. Final hash `4f145f47e977fb2ee0effbe785e94811fb899c802d50fb75e3b56fe13d133340`. |

Node v24.13.0. No cargo, no wasm-pack, and no `bin/ocelli.sh`. The test file was not edited. Its hash stayed `bf3a1a565662a11bdf047664b63f25454fcf30dc6ebe923abc87f20497dcd755`.

The restored step is:

```ts
const step = (lastX - originX) / DRAG_SCALE;
```

## Defects

None.

## Smells

### S1. The suite does not lock a finite normal whose square is not finite on `cropPlanes`

File: `packages/core/src/crop.ts` lines 36 to 39. The permanent expects are in `packages/core/src/crop.test.ts`.

The planes paragraph says the squared length must be finite and greater than 0. `copyPlane` does that. The suite never builds a normal whose components are finite while `nx * nx + ny * ny + nz * nz` is not.

On the restored source, `cropPlanes([{ origin: [0, 0, 0], normal: [Number.MAX_VALUE, 0, 0] }])` is null. That is the behaviour pass 1 required to stay. Deleting `!Number.isFinite(lengthSquared) ||` and keeping `!(lengthSquared > 0)` accepts the plane, because `Infinity > 0` is true. The focused npm command then exited 0, with 6 passed. The probe of that mutated file returned the plane and left the drag result at `[Number.MAX_VALUE, 0, 0]`. The zero-normal expect still passes, because a zero square is not greater than 0. The `NaN` expect still passes, because `component` refuses the `NaN` before the square.

The drag side of this split is locked by the new `Number.MAX_VALUE` expect. The planes side is not. This was not filed in pass 1. The predicate was already present, and the repair did not add an expect for it.

## Nitpicks

None.

## Verified clean

`cropPlanes` returns a new list of 0 to 6 planes. Each returned origin and normal is a new triple. Six repeated inputs became six distinct plane objects. A missing component is null. A non-finite component is null, including `Infinity`. A zero normal is null, including a normal of all `-0`. A normal of `(0, 0, 2)` is copied unchanged. An empty list is a new empty array, not null. Seven planes are null. A later bad plane makes the whole call null, not a partial list. The caller list is not written. A frozen plane was copied and its origin stayed `[9, 8, 7]`. A finite normal of `1e200`, and a normal of `1e154` on every axis, are null because the square is not finite. A normal of `1e-200` is null because its square underflows to 0, which is the squared-length rule rather than a second zero check.

`cropDrag` uses `DRAG_SCALE`, and that constant is the literal 256. The step is `(lastX - originX) / 256`. The normal is multiplied as given. It is not normalised. A negative normal keeps its sign, so `(0, 0, -2)` dragged 256 to the right is `[0, 0, -2]`. A zero normal is null, including all `-0`. A non-finite `originX` or `lastX` is null. A non-finite component is null. A non-finite sum is null, including a drag of 512 on `Number.MAX_VALUE`, which is two times that value. A zero pixel delta keeps the origin even when the normal is `Number.MAX_VALUE`, because the step is 0. `cropDrag([3, 4, 5], [0, 0, 2], 10, 266)` is `[3, 4, 7]` and does not write the inputs. The function returns a new triple. It does not return the normal.

The module has no `unsafe` and no `wasm-bindgen`. It does not branch on tier A, B, or C. Tiers are not applicable, which matches the plan. There is no registry entry and no new command kind. The source has no dot product. The text `set_clip_planes` appears only in the comment that says this module does not call it. `GESTURE_SCALE` in `packages/core/src/pointer.ts` is 256, and `DRAG_SCALE` is a local copy of that F-069 scale rather than an import.

`packages/core/src/index.ts` does not export this module. That absence is a later slice and is not a defect.

## Limitations and handoff

This pass ran only `packages/core/src/crop.test.ts` plus node probes of `crop.ts`. It did not run `index.test.ts`, lint, typecheck, cargo, wasm-pack, `bin/ocelli.sh`, a gate, the oracle, or the corpus. The feature profile named by the plan was not run.

The plan names a paragraph in `docs/lld/public-api.md` and the public-api row in `docs/lld/README.md`. The contributed-id line does not list F-092, and neither file names this crop. This pass does not count that absence as a defect. The reviewed product is the two untracked modules.

A null list element throws `TypeError` on `.origin`. The plan's refusal list does not include a null element. This pass does not count that throw as a defect.

No patient data was read. dwv, Horos, and the medical viewer Grok were not opened.

The mutations are not left in place. `cmp` against the pre-edit copy exited 0. The restored hash is `4f145f47e977fb2ee0effbe785e94811fb899c802d50fb75e3b56fe13d133340`. This review file is the only file this pass added. It was not committed.
