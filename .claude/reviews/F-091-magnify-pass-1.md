# F-091 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s33`. HEAD is `43a894b` (`S33, approve the manipulation designs`). Read in full: `.claude/plans/F-091-design.md`, `packages/core/src/magnify.ts`, `packages/core/src/magnify.test.ts`. Also read the F-091 paragraph and the contributing-id line in `docs/lld/public-api.md`, the magnify export in `packages/core/src/index.ts`, the matching `PUBLISHED` names in `packages/core/src/index.test.ts`, and the public-api row in `docs/lld/README.md`. `git diff --name-only` does not list `annotation.ts`.
**Author / reviewer**: Author is the parent implementation agent. Reviewer is independent subagent `01a19dfc-c6f1-77a1-a62a-d97f0cbb9a84` and did not write the diff.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The approved plan is the contract. The HLD does not define a magnifier. That absence is not a defect.

AC1. `magnifyCircle(5, 6, 4, null)` must return centre `(5, 6)`, radius `4` and scale `1`. The test compares the whole object. A null scale becomes `1` only after the radius is accepted. A different stored scale, or a dropped radius, fails that expect.

AC2. Scale `2` must stay `2`. The function returns the caller number when that number is finite and greater than `1`. It does not rewrite `2` as `1`. The expect is the object with scale `2`.

AC3. Scale `1` must be null, and radius `0` must be null. Those are separate expects. Storing scale `1` as an advanced scale fails the first. Returning a circle for radius `0` fails the second. The same test also refuses a non-finite advanced scale.

AC4. `10` mm at `2` pixels per millimetre must be `20`. The body multiplies the two arguments. The module has no DICOM import and no spacing parameter, so this function cannot read a spacing tag. Returning either input unchanged fails the expect of `20`. The same test refuses `0` pixels per millimetre and a non-finite length.

AC5. Letters `A B C D` at quadrant `1` must be `B C D A`. Quadrant `0` in the same test stays `A B C D`. One step moves the first letter to the end. A reversal to `D A B C` fails the quadrant `1` expect. Replacing the loop bound so the step does not run made that expect red. The received value was `A B C D`.

AC6. Quadrant `4` must be null and must not wrap. The function refuses any quadrant other than exact `0`, `1`, `2` or `3` before the loop. Four steps on four letters would have returned `A B C D` and hidden the refusal. The expect is null, so that wrap fails. The same test refuses quadrant `1.5`, so a closed range that accepts every number from `0` through `3` fails too. A five-element list and a non-string element are null in that test.

Centre finiteness was judged rather than assumed. The plan says:

> `magnifyCircle` takes a canvas centre and a radius. The radius must be finite and greater than 0. Otherwise the call is refused. Advanced magnify is the same circle plus a caller scale. The scale must be finite and greater than 1. Otherwise the call is refused.

It does not say the centre must be finite. The implementation also returns null when `x` or `y` is not finite. The AC3 test locks `magnifyCircle(Number.NaN, 6, 4, null)`.

That extra refusal is a justified local guard, not a defect. The plan's success case is a circle at the caller centre. The same paragraph already refuses a non-finite radius and a non-finite advanced scale, which are the other numbers in that circle. It never says to store a non-finite centre. AC1 still returns the finite centre unchanged. The guard does not read pixels. Refusing `NaN` follows the null refusal the plan already gives the other non-finite inputs. It does not contradict an approved acceptance row.

## Evidence

| Check | Identity | Exit and observation |
|---|---|---|
| `npm test -- packages/core/src/magnify.test.ts` from the repo root | Restored `packages/core/src/magnify.test.ts`. Loop bound is `step < quadrant`. | Exit 0. 6 passed. |
| Same command after the loop bound was changed from `step < quadrant` to `step < 0` | Only that comparison in `packages/core/src/magnify.ts`. No other test file was run. | Exit 1. 1 failed and 5 passed. AC5 received `A B C D` and expected `B C D A`. |
| Source restored, then the same command | Line 82 again uses `step < quadrant`. `step < 0` is absent. | Exit 0. 6 passed. |
| `git diff --name-only` and `git diff -- packages/core/src/annotation.ts` | Working tree on `sprint/s33` | `annotation.ts` is not in the name list. The path diff was empty. |

Vitest 4.1.11. No cargo, no wasm-pack, and no `bin/ocelli.sh`.

The restored loop is:

```ts
for (let step = 0; step < quadrant; step += 1) {
```

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`magnifyCircle` refuses a radius that is not finite or not greater than `0`. A null scale is stored as `1`. An advanced scale is stored only when it is finite and greater than `1`. Scale `1` is null. The returned object uses the caller centre, the caller radius and the accepted scale. There is no resample.

`scaleLength` returns the product of the caller millimetres and the caller pixels per millimetre. A non-finite input returns null. A pixels-per-millimetre that is not greater than `0` returns null. A negative length stays in the product. That matches the review rule that only the pixels-per-millimetre must be greater than `0`. The plan's phrase "a factor that is not greater than 0" is that pixels-per-millimetre, and the LLD says the same. When the product itself is not finite, the function also returns null. That applies the non-finite refusal to the result. It is not a second formula, and it does not read a spacing tag.

`orientationMarker` returns the four letters unchanged at quadrant `0`. Each later exact quadrant moves the first letter to the end once per step. Reading the loop gives quadrant `2` as `C D A B` and quadrant `3` as `D A B C`. Quadrant `4` is null and does not wrap. A length other than `4`, or an element that is not a string, is null. The input array is not written.

The module imports nothing. It does not read pixels, DICOM spacing, or the annotation layer. `packages/core/src/index.ts` exports `magnifyCircle`, `orientationMarker` and `scaleLength` from `./magnify.js`. `PUBLISHED` in `packages/core/src/index.test.ts` names those three under the F-091 comment. The F-091 paragraph in `docs/lld/public-api.md` matches this behaviour. The public-api row in `docs/lld/README.md` names a magnify circle, a scale overlay and an orientation marker, and it lists F-091. `annotation.ts` is unchanged.

No `unsafe`. No `wasm-bindgen`. Tiers A, B and C are not branched. The functions write no SVG.

## Limitations and handoff

`index.test.ts` was read and not executed. Other S33 stories are uncommitted in the same tree, and their modules were not reviewed. The plan also names a feature profile after the focused test. That profile was not run. The test file calls quadrants `0` and `1` only. Quadrants `2` and `3` were checked by reading the one loop. No browser and no pixel oracle were used. This story has no resample to compare.

The mutation is not left in place. The restored source uses `step < quadrant`.
