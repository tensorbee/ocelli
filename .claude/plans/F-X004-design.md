# F-X004, CPU MPR and oblique reslicing

**Status**: approved
**Epic ref**: X1.4
**Sprint**: S21
**Estimate**: 5w

## Normative source, transcribed

From `docs/hld/05-rendering.md` section 7, a feature that cannot run on the resolved tier reports that fact rather than a different image. Deviation D-07 is tier C.

From `docs/hld/16-volume-representation.md` section 19, the plane is the volume origin, spacing and direction.

`Volume::reslice` and `Volume::reslice_oblique` already sample that frame on the host. F-052 and F-053 lock those samples. This story does not replace those formulas.

The allocation note says the inner loop uses wasm SIMD128 and the no-SIMD runtime is measured separately. `SimdSupport` in `ocelli-render` already names `simd128` and the build without it.

## What the specification does not cover

The HLD does not name a CPU presentation type for MPR. The presentation reuses `CpuRaster` from F-X003 for a monochrome plane. A photometric `CpuRaster` refuses is unavailable, not a second sampler.

## Problem and reproducer

Tier C returns `TierCUnavailable` from the orthographic and oblique display paths before an image exists. The samples themselves are already host functions. A GPU-less session has no displayed MPR.

Trigger: the F-053 identity plane. Tier C must present those same samples through `CpuRaster`, or report unavailable when that raster refuses the frame. It must not resample with a different index.

## Scope, API and compatibility

In scope: tier C presentation of the existing axial, sagittal, coronal and oblique samples. The SIMD and scalar loops return the same `f32` samples. Out of scope: a new geometry, thick-slab CPU, fusion CPU, and a CPU volume ray. F-X005 owns the volume decision.

Implementation does not start while F-X003 is `in-progress`. The design is approved. The code waits on that dependency.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Tier C ortho or oblique request | `Volume::reslice` or `Volume::reslice_oblique`, then `CpuRaster` | F-X003 available for the frame | Same samples as tier A |
| Same request | admission | `CpuRaster` refuses the frame | Unavailable, no second image |
| Tier A and tier B | existing `present_plane` | Unchanged | Existing GPU image |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Tier C, raster available | one plane | Presented samples match the host reslice | A different index |
| Tier C, raster refuses | one plane | Unavailable | No buffer published |
| Scalar and simd128 builds | the same plane | Equal samples | A SIMD-only formula |

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| CPU plane buffer | `CpuRaster` session buffer from F-X003 | That session | The raster's existing publish | A refusal does not clear the last stack frame F-X003 already published |

This story adds no allocation in a GPU render loop. The CPU buffer is the one F-X003 sized.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | F-053 identity bytes on the CPU path | The same eight samples | A reimplemented interpolant | Host fixture |
| AC2 | SIMD and scalar on that plane | Equal `f32` bits | A SIMD rounding change | Two build targets when both can run. The scalar test is mandatory here |
| AC3 | Raster refusal | Unavailable, no image | A blank plane presented as the view | Host |
| AC4 | Tier A display | Existing `present_plane` | A CPU buffer on tier A | Existing oblique test |

## Approach

Call the existing reslice functions. Present the plane with `CpuRaster` when F-X003 accepts the frame. Keep one sample formula. A `simd128` build may vectorise the copy into the raster buffer and must match the scalar samples.

## Dependencies, write ownership and resources

Depends on F-052, done, and F-X003, in progress. Do not implement until F-X003 is done. Writes the CPU present path and `docs/lld/volumes.md`. Does not edit `raycast.wgsl`. No GPU exclusivity beyond the existing tier A tests, which stay as they are.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-volume` for the unchanged reslice fixtures. A new host test that the CPU present receives those samples. Feature profile at completion. A missing SIMD measurement is recorded as not measured. It is not a passed timing row.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none on the GPU path
- unsafe: none
- Tier A (WebGPU): unchanged `present_plane`
- Tier B (WebGL2): unchanged `present_plane`
- Tier C (CPU): the same samples, presented by `CpuRaster`, or unavailable

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | F-053 identity samples arrive unchanged | `ocelli-volume` and a render host test |
| tier | Refusal publishes nothing | render host test |

## Parity surface

Appendix B names orthographic and planar viewports. This story is the tier C presentation of planes F-052 and F-053 already sample.

## Deviations and LLD impact

No new deviation. D-07 is the unavailable rule. Update `docs/lld/volumes.md` when the present path lands.

## Open questions

None. The dependency wait is a sequencing constraint, not an undecided formula.
