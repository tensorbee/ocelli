# F-090, trackball, volume and planar rotation

**Status**: approved
**Epic ref**: E14.4
**Sprint**: S33
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` counts the classes. It does not define a
rotation. The quotation is the whole of the relevant row.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
```

F-069 already owns the rotate drag. This story does not reopen it. Rotate
is `basis.radians + (lastX - originX) / 256`. The scale 256 is the private
`GESTURE_SCALE` in `packages/core/src/pointer.ts`.

## What the specification does not cover

The HLD does not say how a trackball differs from a planar rotate. This
plan decides that, and it is not an HLD formula.

All three rotation tools use the F-069 delta. `rotationDrag` returns
`basis + (lastX - originX) / 256`. A non-finite basis or a non-finite
point is refused. Trackball, volume and planar are three registry names
for the existing Rotate command. They do not multiply a new matrix.

`orientationQuadrant` reads an angle in radians and returns 0, 1, 2 or 3.
The step is one quarter turn. The index is the truncation of
`angle / (PI / 2)` toward zero, then the remainder in `{0, 1, 2, 3}`
after a negative wrap. A non-finite angle is refused. The function does
not write the camera.

## Problem and reproducer

No named rotation tool sits on the registry beside the F-069 drag. A
basis of 0 dragged 256 pixels to the right must become 1 radian. Today
the delta exists only inside the pointer dispatch.

## Scope, API and compatibility

```ts
export function rotationDrag(
  radians: number,
  originX: number,
  lastX: number,
): number | null

export function orientationQuadrant(radians: number): 0 | 1 | 2 | 3 | null
```

The three tool names are TrackballRotate, VolumeRotate and PlanarRotate.
Each binds to Rotate. OrientationController is the quadrant function.
It is not a registry tool, because a registry tool requires a command.

Out of scope: a new rotation matrix, editing the F-069 rotate line, and
the volume ray.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `rotationDrag` | caller radians and pointer X | F-069 delta | radians, or null |
| `orientationQuadrant` | caller radians | quarter turns | 0 to 3, or null |
| F-069 rotate line | unchanged | existing dispatch | its fixtures stay |

## State transitions and invariants

The functions are pure. A refusal returns null.

## Ownership, commit and recovery

Not applicable. The functions write no camera.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | radians 0, drag 256 right | 1 | the Y delta used | `packages/core/src/orientation.test.ts` |
| AC2 | a non-finite basis | null | a NaN angle stored | same test file |
| AC3 | PI / 2 | quadrant 1 | a degree conversion | same test file |
| AC4 | a negative PI / 2 | quadrant 3 | a negative index | same test file |
| AC5 | the three names bind to Rotate | the registry command is Rotate | a second rotate command | same test file |

Hand-computed AC1. 256 / 256 is 1. The basis gains that radian.

Hand-computed AC3. One quarter turn is `Math.PI / 2`, which is index 1.

## Approach

Add `packages/core/src/orientation.ts`. Do not edit `pointer.ts`. Do not
add a matrix.

## Dependencies, write ownership and resources

Depends on F-059, which is done. This story writes `orientation.ts`, its
test, the plan and the LLD paragraph. It does not take the GPU.

## Verification and evidence reuse

Focused check: `npm test -- src/orientation.test.ts` from `packages/core`.
Then the feature profile. Mutate the sign of the drag and confirm AC1
goes red. Restore it.

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
| fixture | AC1 through AC5, hand-computed in this plan. The delta is the F-069 scale. The quadrant is this plan. | `packages/core/src/orientation.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts the manipulation classes and does not name a trackball
row. This story adds the three names and the quadrant. It does not
establish a rendered parity view.

## Open questions

None. The HLD does not specify a second rotation. The rules above are
the local decision and they are approved with this plan.
