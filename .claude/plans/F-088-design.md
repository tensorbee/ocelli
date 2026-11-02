# F-088, crosshairs, reference lines and reference cursors

**Status**: approved
**Epic ref**: E14.2
**Sprint**: S33
**Estimate**: 5w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` counts the classes. It does not define a
crosshair. The quotation is the whole of the relevant row.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
```

`docs/hld/13-core-types.md` keeps a canvas point and a patient point as
different types. This story does not convert one into the other.

F-055 already shares one patient point across viewports with one
`FrameOfReferenceUID`. This story does not reopen that index.

## What the specification does not cover

The HLD does not say which lines a crosshair draws. This plan decides
that, and it is not an HLD formula.

The caller supplies the canvas projection of the shared patient point,
and the viewport width and height. `crosshairLines` returns two segments.
The horizontal segment runs from x 0 to the width at the projected y.
The vertical segment runs from y 0 to the height at the projected x.
A point outside the viewport, a non-finite coordinate, or a non-positive
side returns no segments.

`referenceLines` is the same two segments. The world crosshair is not a
second point. `referenceCursor` returns that one projected point when it
lies inside the viewport, and returns none otherwise. Inside includes
the edges. The functions do not read the frame of reference and they do
not project.

## Problem and reproducer

No function turns a projected point into the two crosshair segments. On
a 10 by 8 viewport, the projected point (4, 3) must produce the
horizontal segment (0, 3) to (10, 3) and the vertical segment (4, 0) to
(4, 8). Today there is no entry point.

## Scope, API and compatibility

```ts
export function crosshairLines(
  x: number,
  y: number,
  width: number,
  height: number,
): readonly [Segment, Segment] | null

export function referenceCursor(
  x: number,
  y: number,
  width: number,
  height: number,
): { x: number; y: number } | null
```

`referenceLines` is `crosshairLines`. A segment is `{ x0, y0, x1, y1 }`.

Out of scope: a projection, a change to F-055, the F-071 layer, and the
SR content tree.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `crosshairLines` | caller projection and size | local plan above | two segments, or null |
| `referenceCursor` | the same point | edges included | the point, or null |
| F-071 layer | unchanged | not called | its fixtures stay |

## State transitions and invariants

The functions are pure. A refusal returns null.

## Ownership, commit and recovery

Not applicable. The functions allocate the returned segments and write
no viewport state.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | point (4, 3) on a 10 by 8 view | horizontal (0, 3)-(10, 3) and vertical (4, 0)-(4, 8) | the axes swapped | `packages/core/src/crosshair.test.ts` |
| AC2 | point on the edge (0, 8) of that view | both segments, and the cursor is the point | an edge rejected | same test file |
| AC3 | point (-1, 3), and a zero width | null both times | a segment drawn outside | same test file |
| AC4 | `referenceLines` on the AC1 point | the same two segments | a second geometry | same test file |

Hand-computed AC1. The horizontal line keeps y at 3 and spans the width.
The vertical line keeps x at 4 and spans the height.

## Approach

Add `packages/core/src/crosshair.ts`. Do not edit `annotation.ts`. Do not
add a projection.

## Dependencies, write ownership and resources

Depends on F-055, which is done. This story writes `crosshair.ts`, its
test, the plan and the LLD paragraph. It does not take the GPU.

## Verification and evidence reuse

Focused check: `npm test -- src/crosshair.test.ts` from `packages/core`.
Then the feature profile. Mutate the horizontal end so it stops at
`width - 1`, and confirm AC1 goes red. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC4, hand-computed in this plan. | `packages/core/src/crosshair.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts the manipulation classes and does not name a crosshair
row. This story adds the lines and the cursor. It does not establish a
rendered parity view.

## Open questions

None. The HLD does not specify the segments. The rules above are the
local decision and they are approved with this plan.
