# F-078, ArrowAnnotate, Label, KeyImage and ETDRSGrid

**Status**: approved
**Epic ref**: E12.6
**Sprint**: S29
**Estimate**: 3w

## Normative source, transcribed

HLD decision D13 says the in-memory annotation type is a DICOM SR content tree. F-094 owns that type. `docs/hld/B-parity-surface.md` counts 26 annotation tool classes and does not define an arrow, a label, a key image or an ETDRS radius.

F-071 already draws a display list. Its plan, which is the local rule for that layer, says one `g` per mark and one `polyline` for a non-empty point list, plus one `text` element at the last point when `text` is non-null. This story does not change that layer, its refusal order, or its events.

## What the specification does not cover

The HLD does not define these marks. The decisions below are local. They are not a formula the HLD forgot to write.

- The constructors return `AnnotationMark` values. The caller passes them to `createAnnotationLayer`. This story does not listen, does not import the tool registry, and does not build an SR tree.
- A blank id, after trim, is `blank-id`. A non-finite coordinate is `frame`. Those checks match the F-071 words and they live in this module so the layer stays untouched.
- An arrow is one mark. The polyline is the shaft start, the shaft end, the positive barb, the shaft end again, and the negative barb. The existing polyline then draws a shaft and two barbs.
- The head length is `min(12, shaft_length / 4)` canvas pixels. A zero-length shaft is `degenerate`.
- Each barb is the reverse unit shaft, rotated by `PI / 6` and by its negative, times the head length. For a shaft from `(0, 0)` to `(10, 0)` the head length is 2.5. The positive rotation lands on `(7.834936490538903, -1.2499999999999998)`. The negative rotation lands on `(7.834936490538903, 1.2499999999999998)`.
- Optional arrow text is stored on the mark. Null text adds no label. Blank text, after trim, is `blank-text`.
- A label is one point and a non-blank text. It has no second geometry.
- A key image is one point and the text `KEY`. It does not change a pixel and it does not toggle. The same inputs return an equal mark.
- ETDRS radii are 0.5 mm, 1.5 mm and 3 mm. That is the usual 1 mm, 3 mm and 6 mm diameter grid. It is a local clinical convention, not an HLD number.
- `etdrsRegion` takes millimetres in a caller space where positive `y` is superior. It does not convert canvas pixels. `r < 0.5` is central. `r < 1.5` is inner. `r < 3` is outer. `r >= 3` is outside. A radius equal to a boundary belongs to the outer side of that boundary.
- A quadrant uses the larger absolute component. `|dy| >= |dx|` and `dy > 0` is superior. `|dy| >= |dx|` and `dy < 0` is inferior. `|dx| > |dy|` and `dx > 0` is temporal when `temporalIsPositiveX` is true, otherwise nasal. The opposite `dx` is the other horizontal region. `(1, 1)` is therefore inner superior, not temporal.
- A non-finite millimetre or a non-finite flag input is `frame`. The flag is boolean, so only the millimetres can be non-finite.
- `etdrsMarks` draws three closed polylines of 33 points and four cardinal spokes. A spoke runs from the 0.5 mm radius to the 3 mm radius and does not cross the centre. The spokes are the exact axis offsets, not `cos` of `PI / 2`. `mmPerPixel` must be finite and strictly positive. The circle sample count is 32 plus the repeated start.
- Ids are the prefix plus `-c1`, `-c3`, `-c6`, `-axis-0`, `-axis-1`, `-axis-2` and `-axis-3`. A blank prefix is `blank-id`.

## Problem and reproducer

The drawing layer can show a polyline and cannot build an arrow, a label, a key-image flag or an ETDRS grid. A private annotation model would violate D13.

`arrowMark` of `(0, 0)` to `(10, 0)` has five points and the two barb coordinates above. `labelMark` of `(1, 2)` with text `mm` is one point and that text. `keyImageMark` uses the text `KEY`. `etdrsRegion(0, 0)` is central. `etdrsRegion(1, 1)` is inner superior. `etdrsRegion(4, 0)` is outside. With centre `(10, 20)` and `mmPerPixel` 0.5, the outer circle starts at `(16, 20)`.

## Scope, API and compatibility

In scope:

- `packages/core/src/marks.ts`
- An export from `packages/core/src/index.ts`
- Vitest tests that also `sync` one arrow through the existing layer and read the polyline attribute

Out of scope:

- A change to `annotation.ts`, `tools.ts`, `pointer.ts`, `cursor.ts`, `events.ts` or `commands.ts`
- An SR content tree
- A Rust copy of the arrow or the ETDRS classifier
- A pixel redaction. That is F-079.

```text
arrowMark(id, start, end, text) -> MarkResult
labelMark(id, point, text) -> MarkResult
keyImageMark(id, point) -> MarkResult
etdrsRegion(dxMm, dyMm, temporalIsPositiveX) -> EtdrsRegion | "frame"
etdrsMarks(prefix, centre, mmPerPixel, temporalIsPositiveX) -> MarksResult
```

`temporalIsPositiveX` is accepted by `etdrsMarks` so the call shape stays next to the classifier. The drawn spokes are the cardinal axes and do not move when the flag changes. The flag is recorded as `data-temporal-positive-x` on no element, because this story does not change the layer. The flag remains an argument so a later story can label nasal and temporal without a second classifier.

`MarkResult` is `{ ok: true, mark }` or `{ ok: false, refusal }`. `MarksResult` is the same with `marks`. Refusals are `blank-id`, `blank-text`, `frame` and `degenerate`, in that check order.

## Consumer and provider map

| Consumer | Provider | Policy | Output |
|----------|----------|--------|--------|
| The shell | `arrowMark`, `labelMark`, `keyImageMark`, `etdrsMarks` | the refusals above | display-list marks |
| F-071 `createAnnotationLayer` | unchanged | the caller syncs the marks | one polyline per mark |
| F-094 | not called | D13 | no SR tree |

## State transitions and invariants

The constructors are stateless. They do not write the SVG. A refusal returns `ok: false` and no mark.

| Start | Input | Result | Preserved on refusal |
|-------|-------|--------|----------------------|
| none | arrow `(0,0)` to `(10,0)` | five points and the two barbs | no stored state |
| none | a zero-length shaft | `degenerate` | no stored state |
| none | label text `mm` | one point and that text | no stored state |
| none | label text ` ` | `blank-text` | no stored state |
| none | key image | text `KEY` | no stored state |
| none | ETDRS `(1, 1)` | inner superior | no stored state |

## Ownership, commit and recovery

Not applicable. The functions allocate the returned marks and store nothing. `sync` remains the F-071 commit point. This story does not call it except in the test.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected | Stage |
|----|-------------------------------|------|-------------------------------|-------|
| AC1 | The horizontal arrow has the two barb literals | `the_horizontal_arrow_barbs_are_hand_computed` | a head along the shaft | implementation |
| AC2 | A zero-length shaft is `degenerate` | `a_zero_shaft_is_degenerate` | a mark with a zero head | implementation |
| AC3 | A blank label is `blank-text` | `a_blank_label_is_refused` | drawing an empty text node | implementation |
| AC4 | A key image uses `KEY` and does not toggle | `a_key_image_is_the_text_key` | flipping a stored flag | implementation |
| AC5 | `(0, 0)` is central and `(1, 1)` is inner superior | `the_etdrs_regions_follow_the_radii` | a 1 mm radius or a temporal diagonal | implementation |
| AC6 | `(4, 0)` is outside | `outside_the_outer_radius_is_outside` | clamping to the outer ring | implementation |
| AC7 | Centre `(10, 20)` and 0.5 mm per pixel starts the outer circle at `(16, 20)` | `the_outer_circle_starts_on_the_axis` | using a diameter as a radius | implementation |
| AC8 | Syncing the arrow draws one polyline of those five points | `the_layer_draws_the_arrow_polyline` | a second SVG writer | implementation |

## Approach

Add `marks.ts` beside `annotation.ts`. The arrow rotation uses `Math.cos(Math.PI / 6)` and `Math.sin(Math.PI / 6)` on the reverse unit vector. The ETDRS spokes use the four axis offsets. The test builds one arrow and calls the existing `sync`.

Rejected: a new mark kind inside `annotation.ts`, an SR tree, and a Rust duplicate.

## Dependencies, write ownership and resources

Depends on F-071, which is done. This story writes `packages/core/src/marks.ts`, `packages/core/src/index.ts` and the new test. It does not write `annotation.ts` or `ocelli-geom`. It can run beside the Rust stories only in the sense that the files differ. This sprint is serial, so it runs after F-077.

## Verification and evidence reuse

`npm test` for the focused loop, then the package lint and typecheck that `/complete-feature` already requires. The feature profile is still the 27 gates.

The barb literals and the ETDRS radii are hand-computed from this plan.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The marks are DOM. They do not read the resolved tier.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the acceptance rows above | `packages/core/src/marks.test.ts` |
| fixture | the two barb literals, the radii, and the outer-circle start | the same file |

## Parity surface covered

None. Appendix B counts the annotation classes and does not define these four.

## Deviations

none

## LLD impact

`docs/lld/public-api.md` gains the four constructors when the story is completed. It says they return display-list marks and that the SR tree remains F-094.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The arrow head, the text `KEY`, and the 0.5, 1.5 and 3 mm radii are the local decisions.
