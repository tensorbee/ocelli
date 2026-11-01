# F-X005, the CPU volume-rendering decision

**Status**: approved
**Epic ref**: X1.5
**Sprint**: S21
**Estimate**: 2w

## Normative source, transcribed

From `docs/hld/05-rendering.md` section 7, volume rendering runs on tier A and tier B because a 3D-texture ray-cast in a fragment shader is tier-B legal.

`docs/lld/feature-availability.md` assigns this story the tier A, B and C declaration for volume rendering, and says F-X005 may report volume rendering unavailable on tier C. That is an application of the existing contract. It is not a new availability state. The same file says this contract does not build a CPU volume renderer, and it does not choose shell wording.

Deviation D-07: a feature that cannot run on the resolved tier reports unavailable and does not produce a different image.

## What the specification does not cover

The HLD does not require a CPU ray, and it does not forbid one. The allocation note says the expected outcome is that `VOLUME_3D` reports unavailable on tier C, and that a slow CPU image which differs from the GPU ray is forbidden. This plan adopts that expected outcome. No CPU ray is added.

## Problem and reproducer

`volume_ray_admission` already returns `TierCUnavailable` on tier C and draws nothing. The availability note still lists the volume tier declaration as owned here. A reader can think the refusal is temporary.

Trigger: tier C, one ready brick. The call returns unavailable. No colour buffer is written. Tier A still runs the composite ray.

## Scope, API and compatibility

In scope: record the decision, keep the existing refusal, and name `VOLUME_3D` in the availability note as unavailable on tier C and full on tier A and tier B. Out of scope: a CPU integrator, a shell string, a new error enum, and any change to the GPU integral.

Implementation waits until F-X003 is done only because the backlog names that dependency. The decision itself does not call `CpuRaster`. If F-X003 is still open when this story is reached, the story stays blocked and is not given a fake CPU image.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `volume_ray_admission` | existing `TierCUnavailable` | Tier C | Error, no image |
| Tier A and tier B ray | existing fragment module | Unchanged | Composite ray |
| Availability note | `docs/lld/feature-availability.md` | This story's row | Volume ray unavailable on C |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Tier C | bind or draw a volume ray | Unavailable | No partial CPU frame |
| Tier A | the same bricks | Existing ray | Not routed through the CPU refusal |

## Ownership, commit and recovery

Not a new resource. The GPU ray binding is unchanged. A tier C call allocates nothing and publishes nothing.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Tier C admission | `TierCUnavailable`, no buffer | A CPU colour vector | Existing and extended host test |
| AC2 | Tier A one-step ray | F-060 flat colour still holds | The tier C refusal applied on tier A | Existing GPU test |
| AC3 | Availability note | States unavailable on C, full on A and B | A CPU path described as the same image | Doc review |

## Approach

Do not add a shader or a CPU loop. Extend the host test that already locks `volume_ray_admission`. Update the availability LLD row. Leave shell copy to the contract's existing exclusion.

## Dependencies, write ownership and resources

Depends on F-059, done, and F-X003, in progress. Writes `docs/lld/feature-availability.md` and the admission test. Does not edit `raycast.wgsl` except by leaving it alone. No GPU exclusivity beyond the regression that tier A still draws.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-render --test raycast` for the admission test. Feature profile at completion. No timing claim.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): full, the existing ray
- Tier B (WebGL2): full, the same fragment module
- Tier C (CPU): unavailable. No image

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| tier | CPU admission writes nothing | `ocelli-render` host test |
| regression | Tier A flat ray unchanged | existing ignored GPU test |

## Parity surface

Appendix B lists `VOLUME_3D`. On tier C that viewport is unavailable. This story does not add a CPU volume mode to the parity table.

## Deviations and LLD impact

No new deviation. D-07 is the rule applied. Update `docs/lld/feature-availability.md`.

## Open questions

None. The allocation note names unavailable as the expected outcome, and the code already refuses the draw.
