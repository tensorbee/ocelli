# F-091, magnify, scale overlay and orientation marker

**Status**: approved
**Epic ref**: E14.5
**Sprint**: S33
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` counts the classes. It does not define a
magnifier. The quotation is the whole of the relevant row.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
```

F-071 draws one polyline and optional text for a caller mark. This story
does not reopen that layer and it does not edit `annotation.ts`.

## What the specification does not cover

The HLD does not say what a magnify tool returns. This plan decides that,
and it is not an HLD formula.

`magnifyCircle` takes a canvas centre and a radius. The radius must be
finite and greater than 0. Otherwise the call is refused. Advanced
magnify is the same circle plus a caller scale. The scale must be finite
and greater than 1. Otherwise the call is refused. The function does not
read pixels and it does not scale them.

`scaleLength` takes a caller length in millimetres and a caller
pixels-per-millimetre. The pixel length is the product. A non-finite
factor or a factor that is not greater than 0 is refused. The function
does not read DICOM spacing.

`orientationMarker` takes four caller letters and a quadrant 0, 1, 2 or 3.
Quadrant 0 returns the letters unchanged. Each step moves the first letter
to the end. A list that is not four strings, or a quadrant outside 0 to 3,
is refused.

## Problem and reproducer

No function accepts a magnify radius. A centre (5, 6) and a radius 4 must
come back as that circle. A radius of 0 must be refused. Today there is
no entry point.

## Scope, API and compatibility

```ts
export function magnifyCircle(
  x: number,
  y: number,
  radius: number,
  scale: number | null,
): { x: number; y: number; radius: number; scale: number } | null

export function scaleLength(
  millimetres: number,
  pixelsPerMillimetre: number,
): number | null

export function orientationMarker(
  letters: readonly [string, string, string, string],
  quadrant: number,
): readonly [string, string, string, string] | null
```

A null scale is the plain magnify and is stored as 1. An advanced scale
is the caller number when it is greater than 1.

Out of scope: editing `annotation.ts`, a pixel resample, and a DICOM
spacing read.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `magnifyCircle` | caller centre, radius, scale | local plan above | the circle, or null |
| `scaleLength` | caller factors | product | pixels, or null |
| `orientationMarker` | four letters and a quadrant | rotate the list | four letters, or null |
| F-071 layer | unchanged | not called | its fixtures stay |

## State transitions and invariants

The functions are pure. A refusal returns null.

## Ownership, commit and recovery

Not applicable. The functions write no SVG.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | centre (5, 6), radius 4, no scale | the circle, scale 1 | a zero radius accepted | `packages/core/src/magnify.test.ts` |
| AC2 | scale 2 | scale 2 | a scale of 1 treated as advanced | same test file |
| AC3 | scale 1, and radius 0 | null both times | a non-positive radius drawn | same test file |
| AC4 | 10 mm at 2 pixels per mm | 20 | a spacing tag read | same test file |
| AC5 | letters A B C D at quadrant 1 | B C D A | the letters reversed | same test file |
| AC6 | quadrant 4 | null | a wrap that hides the refusal | same test file |

Hand-computed AC4. The product of 10 and 2 is 20.

Hand-computed AC5. One step moves A to the end. The order is B, C, D, A.

## Approach

Add `packages/core/src/magnify.ts`. Do not edit `annotation.ts`. Do not
read pixels.

## Dependencies, write ownership and resources

Depends on F-071, which is done. This story writes `magnify.ts`, its
test, the plan and the LLD paragraph. It does not take the GPU.

## Verification and evidence reuse

Focused check: `npm test -- src/magnify.test.ts` from `packages/core`.
Then the feature profile. Mutate the quadrant step so index 1 stays
A B C D, and confirm AC5 goes red. Restore it.

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
| fixture | AC1 through AC6, hand-computed in this plan. | `packages/core/src/magnify.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts the manipulation classes and does not name a magnify
row. This story adds the circle, the scale product and the marker.
It does not establish a rendered parity view.

## Open questions

None. The HLD does not specify these overlays. The rules above are the
local decision and they are approved with this plan.
