# F-062, cropping planes and volume clipping

**Status**: approved
**Epic ref**: E9.4
**Sprint**: S21
**Estimate**: 4w

## Normative source, transcribed

From `docs/hld/16-volume-representation.md` section 19, a volume has one `origin: Pt<World>`, one `spacing` and one `direction` derived from `ImageOrientationPatient`.

From `docs/hld/05-rendering.md` section 7, the volume ray is a fragment ray on tier A and tier B, and a feature that cannot run on the resolved tier degrades rather than failing quietly. Tier C already reports the ray unavailable.

The HLD does not define a clip plane.

## What the specification does not cover

The plane count, the kept half-space, and the skip rule are repository choices. A plane is a patient point and a normal in the volume frame. The kept side is `dot(point - plane_origin, normal) >= 0`. A sample on the discarded side adds no colour and no alpha, the same as a residency flag of 0. Bricks are not dropped. The maximum is 6 planes. Zero planes leaves the ray unchanged.

## Problem and reproducer

Every resident sample on the ray is composited. A caller cannot hide one side of a plane.

Trigger: two samples at patient positions 0 mm and 10 mm on the walk axis, one plane through 5 mm with the normal pointing toward 10 mm. The sample at 0 mm is absent from the integral. The sample at 10 mm is the F-060 colour. The brick that holds the discarded sample is still bound.

## Scope, API and compatibility

In scope: up to six planes on the existing composite ray. Out of scope: canvas-space crop, brick eviction, slab clipping, fusion clipping, and a CPU ray.

```rust
pub struct ClipPlane {
    pub origin_mm: [f64; 3],
    pub normal: [f64; 3],
}
```

`set_clip_planes` refuses more than 6 planes, a non-finite component, or a normal whose length is 0. The previous plane list stays. The shader receives plane origin and normal as `f32` after the host conversion from the patient frame. The length test uses the `f64` normal before that conversion.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Volume ray | plane uniform | Empty by default | Unclipped F-060 integral |
| `set_clip_planes` | same | Patient frame | Discarded samples add nothing. Bricks stay bound |
| Tier C | admission | CPU | Unavailable |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| No planes | one legal plane | Next frame skips the discarded side | None |
| Any list | 7 planes or a zero normal | Error | Previous list unchanged |
| Sample exactly on the plane | `dot == 0` | Sample kept | A strict `>` would drop the plane itself |

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Plane uniform, 6 slots | Ray bind | Viewport slot | One submit | A refused list does not write the uniform. Unused slots are disabled, not a second draw |

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | No planes, one sample | F-060 colour | A default plane | Host fixture |
| AC2 | Plane between two samples | Only the kept sample contributes | Both samples, or the brick omitted | Host fixture of the dot |
| AC3 | Sample on the plane | Kept | Dropped by a strict comparison | same |
| AC4 | Zero-length normal | Previous planes kept | Normal treated as an axis | Host |
| AC5 | Tier C | Unavailable | A CPU clipped image | Host |

## Approach

Store at most six planes in a uniform. The fragment shader converts the voxel to a patient point with the volume origin, spacing and direction, then tests the half-space before the opacity load. Do not change brick admission.

## Dependencies, write ownership and resources

Depends on F-059, done. Edits `raycast.wgsl` after F-060 and F-061. Serial in the same wave. GPU exclusive.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-render --test raycast`. Flipping the comparison from `>=` to `>` must fail the on-plane fixture. Feature profile at completion.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Six planes are reserved at bind
- unsafe: none
- Tier A (WebGPU): full
- Tier B (WebGL2): full, fragment only
- Tier C (CPU): unavailable

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Half-space on a 10 mm step, including the plane itself | `ocelli-render` host tests |
| refusal | Count and zero normal | same |
| tier | CPU refusal | same |

Patient position uses the section 19 origin, spacing and direction. The colour of a kept sample cites the existing VOI fixture.

## Parity surface

Appendix B does not name a crop tool. This story clips the composite ray in the patient frame.

## Deviations and LLD impact

No new deviation. `docs/lld/render-graph.md` records the half-space and the six-plane cap.

## Open questions

None. The empty default keeps the measured ray. The kept side includes the plane so a sample on the plane is not a silent hole.
