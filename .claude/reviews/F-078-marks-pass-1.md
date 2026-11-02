# F-078 review, pass 1

**Reviewed**: HEAD `f3012c6cf36193be860025435825edc6d5f321a0` on `sprint/s29`. That commit is F-077 and was not amended. The F-078 product is uncommitted. Modified: `packages/core/src/index.ts`. Untracked: `packages/core/src/marks.ts` and `packages/core/src/marks.test.ts`. Before any review edit, and again after the fraction mutation and the region probe were removed, `shasum` SHA-1 values were `bd6271eb2a35433a1d546c1f044d0b597c86ee83`, `30dcc463ef1743fc65aacdf2d52ba092479f3be6`, and `87a2eb660c5048f0cc855b2439992ac28623ae56`. Those match the pins. `git diff` for `packages/core/src/annotation.ts` is empty.
**Author / reviewer**: The F-078 change has no commit, so this pass does not name an author from a commit or a trailer. The reviewer did not write the change.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-078-design.md`. It is a local decision. HLD decision D13 names an SR content tree and assigns that type to F-094. `docs/hld/B-parity-surface.md` counts annotation classes and does not define an arrow, a label, a key image, or an ETDRS radius. The constructors return F-071 `AnnotationMark` values. `marks.ts` does not build an SR tree and does not write SVG.

AC1. Shaft `(0, 0)` to `(10, 0)` with null text has head `min(12, 10 / 4) = 2.5`. An independent `node` evaluation of `Math.cos(Math.PI / 6)` and `Math.sin(Math.PI / 6)`, not an import of `marks.ts`, produced `0.8660254037844387` and `0.49999999999999994`. The positive barb is `(7.834936490538903, -1.2499999999999998)`. The negative barb is `(7.834936490538903, 1.2499999999999998)`. Both `===` the plan literals, and `String` of those components is the polyline text. `the_horizontal_arrow_barbs_are_hand_computed` passed. The same test refuses a blank id before blank text, and blank text before a non-finite end. A head along the shaft would miss those `y` values.

The fraction mutation set `HEAD_SHAFT_FRACTION` to `2` and left every other token alone. Head length for this shaft became `5`. The same test then received positive `(5.669872981077806, -2.4999999999999996)` and negative `(5.669872981077806, 2.4999999999999996)`. Those are `10 - 5 * cos(PI / 6)` and `±5 * sin(PI / 6)`. The test failed at `packages/core/src/marks.test.ts:22`. Restoring `4` made the file pass.

AC2. Equal ends `(1, 2)` and `(1, 2)` return `degenerate`. `a_zero_shaft_is_degenerate` passed. It stayed green while the fraction was `2`, so the refusal does not depend on the head length. A returned mark would fail the `ok: false` comparison.

AC3. `labelMark` of a blank id is `blank-id` even when the point is `NaN` and the text is blank. Blank text is next. A non-finite point with text `mm` is `frame`. `(1, 2)` with text `mm` is one point and that text. `a_blank_label_is_refused` passed. Drawing an empty text node would fail the refusal comparison.

AC4. Two `keyImageMark("key", { x: 4, y: 5 })` calls compare equal. The text is `KEY`. A blank id is `blank-id`. A non-finite point is `frame`. `a_key_image_is_the_text_key` passed. There is no module flag to flip. A second call cannot toggle.

AC5. `(0, 0)` is `central` for both flag values. `(1, 1)` is `inner superior` for both flag values. `(0, 0.5)` is `inner superior`. `(0, 1.5)` is `outer superior`. `(1, 0)` is `inner temporal` when `temporalIsPositiveX` is true and `inner nasal` when it is false. `NaN` is the string `frame`. `the_etdrs_regions_follow_the_radii` passed. A 1 mm central radius would make `(0, 0.5)` central. A temporal reading of the diagonal would fail `(1, 1)`.

A temporary pair was added inside that test and then deleted. `etdrsRegion(0, -1, true)` is `inner inferior`. `etdrsRegion(-1, 0, true)` is `inner nasal`. Radius `1` is inner. Negative `y` with the larger absolute component is inferior. Negative `x` with the flag true is nasal. The file passed. The test file SHA-1 matches the pin again. The pair is not in the tree.

AC6. `(4, 0)` is `outside`. `(0, 3)` is `outside`. `outside_the_outer_radius_is_outside` passed. Clamping `r = 3` onto the outer ring would fail the second assertion.

AC7. Centre `(10, 20)` and `mmPerPixel` `0.5` produce ids `grid-c1`, `grid-c3`, `grid-c6`, `grid-axis-0`, `grid-axis-1`, `grid-axis-2`, and `grid-axis-3`. The outer mark has 33 points. Its first and last points are `(16, 20)`. The positive-x spoke is `(11, 20)` to `(16, 20)`. The positive-y spoke is `(10, 21)` to `(10, 26)`. The flag-false grid deep-equals the flag-true grid. A blank prefix is `blank-id` even when `mmPerPixel` is `NaN`. A non-finite centre is `frame`. Zero `mmPerPixel` is `degenerate`. `the_outer_circle_starts_on_the_axis` passed. Using the 6 mm diameter as a radius would start the outer circle at `(22, 20)`.

AC8. `the_layer_draws_the_arrow_polyline` builds one arrow, calls the existing `createAnnotationLayer`, and reads `polyline` `points`. On the pinned tree it passed. With the fraction at `2`, `sync` still returned `ok: true`, and the attribute became `0,0 10,0 5.669872981077806,-2.4999999999999996 10,0 5.669872981077806,2.4999999999999996`. The failure is at `marks.test.ts:185`, after the `sync` assertion. The test therefore reads the layer output. A second writer that ignored the mark would not have moved with `HEAD_SHAFT_FRACTION`. `marks.ts` does not reference `document` or SVG. The test creates one `svg` element and does not create a `polyline`.

## Evidence

| Command | Exit | Observation |
|---------|------|-------------|
| `npx vitest run packages/core/src/marks.test.ts` | 0 | 9 passed and 0 failed on the pinned tree. Vitest 4.1.11 |
| same file after `HEAD_SHAFT_FRACTION` changed from `4` to `2` | 1 | 2 failed and 7 passed. `the_horizontal_arrow_barbs_are_hand_computed` and `the_layer_draws_the_arrow_polyline` failed. The other seven stayed green |
| same file after restoring `4` | 0 | 9 passed. All three SHA-1 values matched the pins |
| same file with the temporary inferior and nasal expects | 0 | 9 passed. The expects were deleted. The test file SHA-1 matched the pin again |
| `npx eslint packages/core/src/marks.ts packages/core/src/marks.test.ts packages/core/src/index.ts` | 0 | no findings on the restored tree |
| `node` barb formula, not an import of `marks.ts` | 0 | positive and negative barbs `===` the plan literals. `String` matches the polyline attribute |

Source hashes at the end of the pass, from `shasum`, after eslint:

- `packages/core/src/index.ts` `bd6271eb2a35433a1d546c1f044d0b597c86ee83`
- `packages/core/src/marks.ts` `30dcc463ef1743fc65aacdf2d52ba092479f3be6`
- `packages/core/src/marks.test.ts` `87a2eb660c5048f0cc855b2439992ac28623ae56`

Environment: macOS, zsh, Node `v24.13.0`. One vitest process ran at a time. Exit codes were read from `$?` on the vitest and eslint commands. The fraction edit and the temporary expects are gone. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`index.ts` adds one header sentence and exports `MARK_REFUSALS`, `arrowMark`, `etdrsMarks`, `etdrsRegion`, `keyImageMark`, `labelMark`, and the types `EtdrsRegion`, `MarkRefusal`, `MarkResult`, and `MarksResult` from `./marks.js`. The diff is 11 insertions. `git status` shows no other product path.

Check order in `arrowMark` is blank id, then blank text when text is not null, then a non-finite end or start, then a zero `Math.hypot`. Null text is stored as null. `labelMark` uses the same three applicable checks and then one point. `keyImageMark` checks blank id and then the point, and stores the constant `KEY`. It has no text argument, so `blank-text` does not apply. `etdrsMarks` checks a blank prefix, then a non-finite centre, a non-finite `mmPerPixel`, or a non-boolean flag, and then `mmPerPixel <= 0`. `etdrsRegion` returns the string `frame` for a non-finite millimetre. That matches the plan signature. It is not a `MarkResult`.

The arrow head is `Math.min(12, length / HEAD_SHAFT_FRACTION)` with the fraction `4` and the limit `12`. The positive barb is the reverse unit shaft rotated by `+PI / 6`. The negative barb uses the opposite sign. For a horizontal shaft the `reverseY` terms are zero, so the fixture does not exercise them. The formulas are the standard rotation. The polyline is start, end, positive barb, end, negative barb.

`etdrsRegion` uses `Math.hypot`. `r < 0.5` is central. `r >= 3` is outside. Otherwise `r < 1.5` is inner and the remainder is outer. A boundary therefore belongs to the outer side. The executed boundaries are `0.5`, `1.5`, and `3`. `quadrantOf` takes the vertical name when `abs(dy) >= abs(dx)`. Positive `y` is superior and negative `y` is inferior. The horizontal name follows `temporalIsPositiveX` on positive `x` and the opposite flag on negative `x`. The flag is not read on a vertical region. `(1, 1)` stays superior for both flag values.

`closedCircle` seeds the exact `+x` point, samples steps `1` through `31` with `cos` and `sin`, and repeats that seed. `CIRCLE_STEPS` is `32`, so the list length is `33`. The outer radius in pixels is `3 / 0.5 = 6`, so the seed is `(16, 20)`. The spokes are the four axis sign pairs from the `0.5` mm radius to the `3` mm radius. They do not call `cos(PI / 2)`. `etdrsMarks` reads the flag only to refuse a non-boolean. The drawn marks do not include it. The deep equality of the two flag values passed.

`displayMark` sets `visible` true, `locked` false, `selected` false, and `completed` true. The acceptance objects include those fields. Blank id uses `trim`, which is the same blank-id rule as `annotation.ts`. There is no new trait, no `wasm-bindgen`, and no Rust file in the uncommitted set. Tiers A, B, and C are not applicable. The functions do not open a device and do not read a tier.

The only `as` token in `marks.ts` is `as const` on `MARK_REFUSALS`. That is the same const assertion `ANNOTATION_REFUSALS` uses. It narrows the refusal tuple. It does not cast a coordinate. `marks.test.ts` has no `as`. The `index.ts` addition has no `as`.

The `marks.ts` header says the HLD does not define these marks and points at the local plan. The arrow comment states the 12 pixel cap and the quarter shaft. The region comment states the boundary rule and the flag. Neither comment says the HLD specifies the radii or the barb. The `index.ts` sentence says F-078 publishes the display-list marks.

The unreachable `axis === undefined` branch returns `degenerate` only to narrow the tuple access. The axis list has four entries and the loop index stays inside it. The branch did not run.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. The plan adds the constructors to `docs/lld/public-api.md` at completion. That file was not in this diff. Sprint status still lists F-078 as pending. This pass did not edit it.

A shaft longer than 48 pixels was not executed. On the length-10 fixture the quarter shaft is `2.5`, so the `12` cap is not the value `Math.min` selects. Changing only the cap would leave both arrow tests green. The cap is present in the source. A vertical shaft was not executed. The `reverseY` terms were read. They were not observed through `arrowMark`.

The outer circle assertion locks the length, the repeated `+x` seed, and the two positive spokes. It does not lock an interior sample, the `c1` or `c3` radius, or the negative spokes. Those paths were read. A non-blank arrow string was not given a success assertion. `displayMark` passes that string through. Null text and blank text were executed. Negative `mmPerPixel` was not executed. The same `<= 0` check that refuses zero would refuse it as `degenerate`.

Only this vitest file was run, plus the deleted probe and eslint on the three restored paths. The package typecheck was not run. No GPU and no oracle comparison. These marks have no rendered pixels.

The temporary fraction edit and the temporary expects are gone. The three SHA-1 values match the pins. Do not attach this result to a later tree. The review file is untracked.
