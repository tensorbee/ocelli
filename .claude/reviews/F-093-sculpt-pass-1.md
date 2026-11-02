# F-093 review, pass 1

**Reviewed**: untracked product files on `sprint/s34`. HEAD is `599ddfcc28a6d4c1356d8079e08d7d8aa96275d3`. Nothing was staged. `git diff --name-only` was empty. The reviewed sources are `packages/core/src/sculpt.ts` (sha256 `12bcf3bcabb54f3322f91be62d0eb4b757f237e3c47f6e0ab455dea575bf0cc8`) and `packages/core/src/sculpt.test.ts` (sha256 `0227a330480790f2e887dfebfb0217d341b2e1639b7232ed917ef5ecfa545793`). `packages/core/src/annotation.ts` was not in that name list.
**Author / reviewer**: The author is the parent implementation agent. The reviewer is independent subagent `01a19e3d-e6e5-7aa1-825b-ee2ba8de87c7` and did not write the plan or the diff.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The approved plan is the contract. A search of `docs/hld/` found MIP only as a blend mode, in `docs/hld/05-rendering.md` and `docs/hld/B-parity-surface.md`. It found no sculptor, no annotation eraser, and no click-jump rule. The blend mode is not a formula for these functions. That absence is not a defect.

AC1. Vertex 1 of `(0, 0)`, `(1, 0)`, `(0, 1)` is dragged from `(0, 0)` to `(2, 3)`. The hand sum is `(1, 0) + (2, 3) = (3, 3)`. The shipped expect is that whole list, and `points[1]` stays `(1, 0)`. An independent vertex `(40, 50)`, with origin `(1, 1)` and last `(4, -2)`, has delta `(3, -3)` and lands on `(43, 47)`. A divisor of 256 would leave `40 + 3/256` and `50 - 3/256`, not `(43, 47)`. The probe received `(43, 47)`. The caller point objects stayed the same objects, and the returned array was a different array.

AC2. A two-point list returns null. Index 3 on a three-point list returns null. Moving the short list would fail the first null expect. Accepting index 3 would fail the second. The same test also refuses a non-finite point. A probe on the restored module also returned null for index `-1` and for index `1.5`.

AC3. Marks at `(0, 0)` and `(3, 4)`, click `(0, 1)`, radius `2`. The first distance is `1`. The second is `sqrt(3 * 3 + 3 * 3)`, which is `sqrt(18)` and is greater than `2`. The squared test agrees: `1 <= 4` and `18 > 4`. The expect is the first id. Returning the farther id fails it. That fixture has only one mark inside the radius. A separate probe put an earlier mark at distance `5` and a later mark at distance `1`, both inside radius `10`. The received id was the later, nearer id. Keeping the first in-radius id would have returned the farther one.

AC4. Two marks on the click keep the earlier id. The update uses `distance < bestDistance`, so an equal distance does not replace the earlier mark. A probe of `(3, 4)` and `(0, 5)` against the origin, both at distance `5`, also received the earlier id. A later tie would fail the shipped expect of `"a"`.

AC5. Samples `1, 5, 5, 2` have maximum `5` at index `1` and index `2`. The earlier index is `1`. Replacing `sample > best` with `sample >= best` made that expect receive `2`. The unique maximum `[1, 3, 2]` is in the same test and was not the failing expect. A probe of `[5, 5]` on the restored source received `0`.

AC6. An empty list is null. `[1, NaN]` is null. Returning index `0` for an empty list would fail the first expect. A probe also received null for a leading `NaN` and for `Infinity`. Both are non-finite, so the whole call is refused.

An empty id is refused by the shipped test. The plan says an empty id refuses the whole call, not only that mark. A probe with a valid in-radius mark followed by an empty id received null, not the earlier id.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npm test -- packages/core/src/sculpt.test.ts` from the repo root | Restored predicate `sample > best`. sha256 `12bcf3bcabb54f3322f91be62d0eb4b757f237e3c47f6e0ab455dea575bf0cc8` | executed now, before the mutation | Exit 0. 6 passed. |
| Same command after `sample > best` was changed to `sample >= best` | One comparison in `packages/core/src/sculpt.ts`. Mutated sha256 `02bbacb4ad5b61372b78eed2c09f4b2499e159d3fbe8ab5bf5e0cab7e1863fa4`. No other test file was run. | executed now | Exit 1. 1 failed and 5 passed. AC5 received `2` and expected `1` at `sculpt.test.ts:95`. |
| `cmp` of the saved original against `packages/core/src/sculpt.ts`, then the same test | Bytes copied back from the pre-mutation file. `sample >= best` is absent. sha256 again `12bcf3bcabb54f3322f91be62d0eb4b757f237e3c47f6e0ab455dea575bf0cc8` | executed now | `cmp` exit 0. Test exit 0. 6 passed. |
| Node probe of the restored module | Independent delta `(43, 47)`, later nearer id, equal distance `5`, radius `5` and `4.999`, empty id after a valid mark, negative and fractional index, leading `NaN`, `Infinity` | executed now, no repo file added | Exit 0. Each of those cases matched the plan. |

Vitest 4.1.11. Node v24.13.0. No cargo, no wasm-pack, and no `bin/ocelli.sh`.

The restored comparison is:

```ts
if (sample > best) {
  best = sample;
  index = cursor;
}
```

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`sculptVertex` returns null unless the index is an integer inside a list of at least three points. The selected point moves by `(lastX - originX, lastY - originY)`. There is no divisor. The other points keep their coordinates. A non-finite origin, last point, input coordinate, or moved coordinate returns null. The function builds a new array and does not write the caller list. The AC1 expect locks `(3, 3)`. The probe locks `(43, 47)` and caller object identity.

`eraseNearest` returns null unless the click and the radius are finite and the radius is greater than `0`. It keeps the mark with the smallest squared distance that is still at most `radius * radius`. An equal distance keeps the earlier mark. No mark inside the radius returns null. An empty list returns null. A non-finite mark coordinate, a non-string id, or an empty id returns null for the whole call. The function does not remove a mark. The on-radius `3-4-5` case is included. Radius `4.999` on that same mark is null.

`mipIndex` returns the earlier index of the maximum finite sample. The walk uses a strict greater-than test, so a tie does not move the index. An empty list returns null. One non-finite sample returns null. The samples are caller numbers. The function does not read a frame.

The module imports nothing. It does not read pixels, compute a polygon area, or edit the annotation layer. `packages/core/src/index.ts` does not export it. That missing export is a later slice and is not a defect. No `unsafe`. No `wasm-bindgen`. Tiers A, B, and C are not branched. The plan marks each tier n/a. The functions write no SVG and no label grid.

## Limitations and handoff

The mutation is not left in place. `cmp` against the saved original was exit 0, and the sha256 is `12bcf3bcabb54f3322f91be62d0eb4b757f237e3c47f6e0ab455dea575bf0cc8`.

`index.ts` was not executed. The plan also names a feature profile after the focused test. That profile was not run. `docs/lld/public-api.md` has no F-093 row in this tree. The plan names that paragraph at story completion, and it was not one of the reviewed product files. Other untracked story files, `crop.ts` and `intersection.ts`, were not reviewed.

The shipped tests do not themselves place two in-radius marks at different distances, and they do not include a mark exactly on the radius. Those two cases were executed by the probe on the restored module. Negative index, a fractional index, a leading `NaN`, and `Infinity` were probed and are not separate shipped expects. Overflow of a finite square to a non-finite product was not exercised. No browser and no pixel oracle were used. This story has no rendered parity view.

No patient data was read. dwv, Horos, and the medical viewer Grok were not opened.
