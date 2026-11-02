# F-093, sculptor, annotation eraser and MIP jump to click

**Status**: approved
**Epic ref**: E14.7
**Sprint**: S34
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` counts the classes. It does not define a
sculptor, an eraser or a MIP jump. The quotation is the whole of the
relevant row.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
```

F-076 already measures polygon area and one spline point. This story
does not copy that area and does not call it. F-071 already draws marks.
This story does not edit that layer and does not delete a mark.

## What the specification does not cover

The HLD does not say how a vertex moves, which mark an eraser names, or
which sample a MIP click selects. This plan decides that, and it is not
an HLD formula.

`sculptVertex` requires at least three points. The index is an integer
inside that list. The selected point moves by `(lastX - originX, lastY -
originY)` with no divisor. The other points stay. The caller list is not
written. Fewer than three points, an index outside the list, or a
non-finite number returns nothing.

`eraseNearest` names the nearest mark whose Euclidean distance from the
click is at most the radius. The radius must be finite and greater than
0. An equal distance keeps the earlier mark. No mark inside the radius
returns nothing. An empty list returns nothing. A non-finite coordinate,
a non-string id, or an empty id refuses the whole call. The function
does not remove the mark.

`mipIndex` returns the earlier index of the maximum finite sample. An
empty list returns nothing. One non-finite sample refuses the whole
call. The samples are caller numbers. The function does not read a frame.

## Problem and reproducer

No function moves one polygon vertex, names a mark inside a radius, or
returns the earlier maximum index. A triangle (0, 0), (1, 0), (0, 1)
with vertex 1 dragged by (2, 3) must become (0, 0), (3, 3), (0, 1).
Today there is no entry point.

## Scope, API and compatibility

```ts
export function sculptVertex(
  points: readonly { x: number; y: number }[],
  index: number,
  originX: number,
  originY: number,
  lastX: number,
  lastY: number,
): readonly { x: number; y: number }[] | null

export function eraseNearest(
  marks: readonly { id: string; x: number; y: number }[],
  x: number,
  y: number,
  radius: number,
): string | null

export function mipIndex(samples: readonly number[]): number | null
```

Out of scope: polygon area, the spline, the annotation layer, a pixel
read, and the SR content tree.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `sculptVertex` | caller points and a pixel delta | at least three points | a new list, or null |
| `eraseNearest` | caller marks, click, radius | earlier tie wins | an id, or null |
| `mipIndex` | caller samples | earlier maximum wins | an index, or null |
| F-076 area | unchanged | not called | its fixtures stay |
| F-071 layer | unchanged | not called | its fixtures stay |

## State transitions and invariants

The functions are pure. A refusal returns null. The caller arrays are
not written.

## Ownership, commit and recovery

Not applicable. The functions write no SVG and no label grid.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | vertex 1 dragged by (2, 3) | (0, 0), (3, 3), (0, 1) | a delta divided by 256 | `packages/core/src/sculpt.test.ts` |
| AC2 | two points, and index 3 | null both times | a short list moved | same test file |
| AC3 | marks at (0, 0) and (3, 4), click (0, 1), radius 2 | the first id | the farther id | same test file |
| AC4 | two marks on the click | the earlier id | the later tie | same test file |
| AC5 | samples 1, 5, 5, 2 | index 1 | the later tie at index 2 | same test file |
| AC6 | an empty sample list, and one NaN | null both times | index 0 for an empty list | same test file |

Hand-computed AC1. Vertex 1 starts at (1, 0). Adding (2, 3) gives (3, 3).

Hand-computed AC3. The first distance is 1. The second distance is
`sqrt(3 * 3 + 3 * 3)`, which is greater than 2. The first id is inside
and nearer.

Hand-computed AC5. The maximum is 5. Index 1 and index 2 both hold it.
The earlier index is 1.

## Approach

Add `packages/core/src/sculpt.ts`. Do not edit `annotation.ts`. Do not
read pixels. Do not compute a polygon area.

## Dependencies, write ownership and resources

Depends on F-076, which is done. This story writes `sculpt.ts`, its
test, the plan and the LLD paragraph. It does not take the GPU.

## Verification and evidence reuse

Focused check: `npm test -- packages/core/src/sculpt.test.ts` from the
repo root. Then the feature profile. Mutate the maximum walk so AC5
returns 2, and confirm AC5 goes red. Restore it.

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
| fixture | AC1 through AC6, hand-computed in this plan. | `packages/core/src/sculpt.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts the manipulation classes and does not name these three
rows. This story adds the vertex move, the nearest id and the earlier
maximum. It does not establish a rendered parity view.

## Open questions

None. The HLD does not specify these tools. The rules above are the
local decision and they are approved with this plan.
