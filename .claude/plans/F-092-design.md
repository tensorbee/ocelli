# F-092, volume cropping and volume cropping control

**Status**: approved
**Epic ref**: E14.6
**Sprint**: S34
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` counts the classes. It does not define a
crop box. The quotation is the whole of the relevant row.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
```

F-062 already stores the planes. `crates/ocelli-render/src/clip.rs` says
the kept side is `dot(point - plane_origin, normal) >= 0`, including the
plane, and that at most six planes are stored. This story does not copy
that test and does not call `set_clip_planes`.

## What the specification does not cover

The HLD does not say how a shell drag moves a crop plane. This plan
decides that, and it is not an HLD formula.

`cropPlanes` accepts 0 to 6 planes. Each origin component and each normal
component must be finite. The normal's squared length must be finite and
greater than 0. The function returns a copy. More than six planes, a
non-finite component, or a zero normal returns nothing. An empty list is
legal and returns an empty copy. The kept side is not evaluated.

`cropDrag` adds `normal * (lastX - originX) / 256` to the origin. The
scale is 256, the same scale F-069 uses for a pixel delta. The normal is
not normalised, so the step scales with the caller normal. A zero normal,
a non-finite input, or a non-finite result returns nothing. The normal
itself is not returned and is not changed.

VolumeCropping is `cropPlanes`. VolumeCroppingControl is `cropDrag`.
Neither is a registry tool. There is no new command kind.

## Problem and reproducer

No shell function counts six planes or moves an origin along a normal.
A plane at the origin with normal (0, 0, 1) and a drag of 256 pixels to
the right must move the origin to (0, 0, 1). Today there is no entry
point.

## Scope, API and compatibility

```ts
export type CropPlane = {
  origin: readonly [number, number, number];
  normal: readonly [number, number, number];
};

export function cropPlanes(
  planes: readonly CropPlane[],
): readonly CropPlane[] | null

export function cropDrag(
  origin: readonly [number, number, number],
  normal: readonly [number, number, number],
  originX: number,
  lastX: number,
): readonly [number, number, number] | null
```

Out of scope: `set_clip_planes`, the shader uniform, a seventh plane,
and a half-space test.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `cropPlanes` | caller planes | at most six, finite, non-zero normal | a copy, or null |
| `cropDrag` | origin, normal, pixel delta | delta divided by 256 | a new origin, or null |
| F-062 `set_clip_planes` | unchanged | not called | its fixtures stay |

## State transitions and invariants

The functions are pure. A refusal returns null. The caller array is not
written.

## Ownership, commit and recovery

Not applicable. The functions do not bind a viewport.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | one plane, origin 0, normal (0, 0, 1) | a copy of that plane | the same array returned | `packages/core/src/crop.test.ts` |
| AC2 | an empty list | an empty copy | null for the empty list | same test file |
| AC3 | seven planes, a zero normal, and a NaN | null each time | a seventh plane stored | same test file |
| AC4 | origin 0, normal (0, 0, 1), drag 256 right | origin (0, 0, 1) | a normalised step of a different size | same test file |
| AC5 | the same plane, drag 256 left | origin (0, 0, -1) | a drag that ignores the sign | same test file |
| AC6 | a zero normal on the drag | null | a move with a zero normal | same test file |

Hand-computed AC4. The delta is 256 / 256, which is 1. The origin z
becomes 0 + 1 * 1.

## Approach

Add `packages/core/src/crop.ts`. Do not edit `clip.rs`. Do not compute
`dot(point - origin, normal)`.

## Dependencies, write ownership and resources

Depends on F-062, which is done. This story writes `crop.ts`, its test,
the plan and the LLD paragraph. It does not take the GPU.

## Verification and evidence reuse

Focused check: `npm test -- packages/core/src/crop.test.ts` from the
repo root. Then the feature profile. Mutate the drag sign so AC4 returns
(0, 0, -1), and confirm AC4 goes red. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a. The shell does not bind a plane.
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a. F-062 already reports the ray unavailable on CPU.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC6, hand-computed in this plan. | `packages/core/src/crop.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts the manipulation classes and does not name a crop row.
This story adds the count and the drag. It does not establish a rendered
parity view.

## Open questions

None. The HLD does not specify this drag. The rules above are the local
decision and they are approved with this plan.
