# F-089, slice intersection, segmentation intersection and overlay grid

**Status**: approved
**Epic ref**: E14.3
**Sprint**: S34
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` counts the classes. It does not define a
slice intersection, a segmentation intersection or an overlay grid. The
quotation is the whole of the relevant row.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
```

`docs/hld/13-core-types.md` keeps a canvas point and a patient point as
different types. This story does not convert one into the other.

F-088 already draws two segments from one projected point. This story
does not call that module and does not change it.

## What the specification does not cover

The HLD does not say how a slice line meets a viewport. This plan decides
that, and it is not an HLD formula.

The caller supplies canvas endpoints. `sliceIntersection` clips that one
segment to the viewport with Liang-Barsky. The viewport is `x` from 0 to
the width and `y` from 0 to the height. The edges are inside. A zero
delta is not a line and returns nothing. A segment that misses the
viewport returns nothing.

`segmentationIntersection` runs the same clip on each caller segment, in
order, and drops a segment the clip refuses. A non-finite coordinate, a
non-positive side, or an empty segment list is decided before that walk.
An empty list returns an empty list. One non-finite coordinate refuses
the whole call.

`overlayGrid` returns the interior lines only. Column count and row count
are integers of at least 1. Vertical lines sit at `width * c / columns`
for `c` from 1 to `columns - 1`, from `y` 0 to the height. Horizontal
lines sit at `height * r / rows` for `r` from 1 to `rows - 1`, from `x`
0 to the width. One column and one row return an empty list. A
non-integer count returns nothing.

The functions do not project and they do not read a frame of reference.

## Problem and reproducer

No function clips a caller segment to the viewport or places an interior
grid. On a 10 by 8 viewport, the segment from (-2, 4) to (12, 4) must
become (0, 4) to (10, 4). Today there is no entry point.

## Scope, API and compatibility

```ts
export function sliceIntersection(
  x0: number,
  y0: number,
  x1: number,
  y1: number,
  width: number,
  height: number,
): Segment | null

export function segmentationIntersection(
  segments: readonly Segment[],
  width: number,
  height: number,
): readonly Segment[] | null

export function overlayGrid(
  columns: number,
  rows: number,
  width: number,
  height: number,
): readonly Segment[] | null
```

A segment is `{ x0, y0, x1, y1 }`. This module defines that type again.
It does not import `crosshair.ts`.

Out of scope: a projection, a change to F-088, the F-071 layer, contour
extraction, and the SR content tree.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `sliceIntersection` | caller endpoints and size | Liang-Barsky, edges inside | one segment, or null |
| `segmentationIntersection` | the same clip per segment | a bad number refuses the call | a list, or null |
| `overlayGrid` | integer counts and size | interior lines only | a list, or null |
| F-088 crosshair | unchanged | not called | its fixtures stay |

## State transitions and invariants

The functions are pure. A refusal returns null. A grid of one by one
returns an empty list, which is not a refusal.

## Ownership, commit and recovery

Not applicable. The functions write no SVG.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | (-2, 4) to (12, 4) on 10 by 8 | (0, 4) to (10, 4) | the unclipped ends | `packages/core/src/intersection.test.ts` |
| AC2 | (1, 1) to (2, 2) on 10 by 8 | the same segment | a clip that moves an inside end | same test file |
| AC3 | (-5, -5) to (-1, -1), and a zero delta | null both times | a miss drawn as a segment | same test file |
| AC4 | one inside segment and one miss | only the inside segment | a miss left in the list | same test file |
| AC5 | an empty segment list | an empty list | null for an empty list | same test file |
| AC6 | columns 2, rows 2, 10 by 8 | the midlines | a border included | same test file |
| AC7 | columns 1 and rows 1 | an empty list | a border drawn for one cell | same test file |
| AC8 | columns 2.5 | null | a fractional count accepted | same test file |

Hand-computed AC1. `dx` is 14 and `dy` is 0. The entering `t` is 1/7 and
the leaving `t` is 6/7. The points are (0, 4) and (10, 4).

Hand-computed AC6. Two columns place one vertical line at x 5. Two rows
place one horizontal line at y 4.

## Approach

Add `packages/core/src/intersection.ts`. Do not edit `crosshair.ts`. Do
not project.

## Dependencies, write ownership and resources

Depends on F-088, which is done. This story writes `intersection.ts`,
its test, the plan and the LLD paragraph. It does not take the GPU.

## Verification and evidence reuse

Focused check: `npm test -- packages/core/src/intersection.test.ts` from
the repo root. Then the feature profile. Mutate the leaving bound so AC1
keeps x 12, and confirm AC1 goes red. Restore it.

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
| fixture | AC1 through AC8, hand-computed in this plan. | `packages/core/src/intersection.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts the manipulation classes and does not name these three
rows. This story adds the clip and the interior grid. It does not
establish a rendered parity view.

## Open questions

None. The HLD does not specify these overlays. The rules above are the
local decision and they are approved with this plan.
