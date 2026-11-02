# F-087, pan, zoom, window, region window and stack scroll

**Status**: approved
**Epic ref**: E14.1
**Sprint**: S33
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` counts the classes. It does not define these
tools. The quotation is the whole of the relevant row.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
```

`docs/hld/03-architecture-and-crates.md` puts tool interaction state in the
shell. It does not give a pan, a zoom, a window or a scroll formula.

F-069 already owns the camera drag. This story does not reopen it. Pan is
`basis.panX + (lastX - originX)` and `basis.panY + (lastY - originY)`. Zoom
is `basis.zoom * exp(-(lastY - originY) / 256)`. The scale 256 is the
private `GESTURE_SCALE` in `packages/core/src/pointer.ts`. Rotate is not
this story.

## What the specification does not cover

The HLD does not say how a window drag or a stack drag moves. This plan
decides that, and it is not an HLD formula.

`window_drag` takes a caller basis `{ centre, width }` and a pointer
origin and last point. The centre becomes `basis.centre + (lastX - originX) / 256`.
The width becomes `basis.width + (originY - lastY) / 256`. A result whose
width is not greater than 0 is refused. A non-finite basis or a non-finite
result is refused. A refusal returns no command and does not change the
basis. The function does not apply a VOI LUT.

`window_region` takes caller samples. The centre is `(min + max) / 2`.
The width is `max - min`. An empty list, a non-finite sample, or a span
of 0 is refused. The function does not read pixels and it does not apply
a VOI LUT.

`stack_drag` takes the same pointer pair. The step is the truncation of
`(originY - lastY) / 256` toward zero. A step of 0 returns no command.
A non-finite point is refused.

Pan and zoom are not reimplemented. A caller who wants those drags uses
the F-069 dispatch. This story only names the five tools on the existing
registry, each bound to the existing command kind: Pan, Zoom, SetWindow,
SetWindow, Scroll.

## Problem and reproducer

No named tool produces a window or a stack step. A basis centre 40 and
width 400, dragged 256 pixels right and 256 pixels up, must become
centre 41 and width 401. Today there is no window drag entry point.

## Scope, API and compatibility

```ts
export function windowDrag(
  basis: { centre: number; width: number },
  originX: number,
  originY: number,
  lastX: number,
  lastY: number,
): { centre: number; width: number } | null

export function windowRegion(
  samples: readonly number[],
): { centre: number; width: number } | null

export function stackDrag(
  originY: number,
  lastY: number,
): number | null
```

In scope: those three functions and five registry names in
`packages/core/src/manipulation.ts`.

Out of scope: editing the pan, zoom or rotate lines, a new VOI formula,
a GPU path, and the SR content tree.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `windowDrag` | caller basis and pointer | local plan above | centre and width, or null |
| `windowRegion` | caller samples | local plan above | centre and width, or null |
| `stackDrag` | pointer Y | local plan above | integer step, or null |
| F-069 pan and zoom | unchanged | existing dispatch | its fixtures stay |

## State transitions and invariants

The functions are pure. A refusal returns null and the caller basis stays
where the caller left it. A second call with the same numbers returns the
same numbers.

## Ownership, commit and recovery

Not applicable. The functions allocate nothing and write nothing.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | centre 40, width 400, drag 256 right and 256 up | centre 41, width 401 | the width moves with Y instead of against it | `packages/core/src/manipulation.test.ts` |
| AC2 | the same drag with width 0.5 and 256 down | null, basis unchanged | a non-positive width accepted | same test file |
| AC3 | samples 10, 30, 20 | centre 20, width 20 | the mean used instead of the mid-span | same test file |
| AC4 | empty samples, and one NaN | null both times | a window of 0 | same test file |
| AC5 | origin Y 0, last Y -256 | step 1 | a fractional step | same test file |
| AC6 | last Y equals origin Y | null | a step of 0 packed | same test file |
| AC7 | the five tool names bind to Pan, Zoom, SetWindow, SetWindow, Scroll | the registry returns those command kinds | a new command kind | same test file |

Hand-computed AC1. 256 / 256 is 1. The centre gains the horizontal pixel
delta. The width gains the upward pixel delta. Up is a smaller Y.

Hand-computed AC3. The minimum is 10 and the maximum is 30. The mid-span
is 20. The difference is 20.

## Approach

Add `packages/core/src/manipulation.ts`. Do not edit `pointer.ts`. The
scale is the number 256, the same scale F-069 already uses. Do not copy
the pan or zoom expression into this file.

## Dependencies, write ownership and resources

Depends on F-068, which is done. This story writes `manipulation.ts`, its
test, the plan and the LLD paragraph. It does not take the GPU.

## Verification and evidence reuse

Focused check: `npm test -- src/manipulation.test.ts` from `packages/core`.
Then the feature profile, 27 gates, corpus pass, no comparison. The sprint
profile is a later commit.

Mutate the width sign so an upward drag decreases width, and confirm AC1
goes red. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The functions do not open a device.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC7, hand-computed in this plan. The scale 256 is the F-069 scale. The window rules are this plan. | `packages/core/src/manipulation.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts 25 manipulation classes and does not name these rows.
This story adds the five named tools. It does not establish a rendered
parity view.

## Open questions

None. The HLD does not specify the window fraction. The rules above are
the local decision and they are approved with this plan.
