# F-061, lighting, shading and gradient opacity

**Status**: approved
**Epic ref**: E9.3
**Sprint**: S21
**Estimate**: 3w

## Normative source, transcribed

From `docs/hld/05-rendering.md` section 7:

> Volume rendering runs on both tiers, because a 3D-texture ray-cast in a fragment shader is tier-B legal.

> One submit per frame.

Anything that needs a compute shader is tier A only and must degrade rather than fail. A gradient that can be read from the same `texture_3d` samples as the ray stays on the fragment path, so tier B keeps the ray.

Section 19 gives the volume one spacing and one direction. The HLD does not give a lighting equation.

## What the specification does not cover

The shade equation is a repository choice. Ambient weight defaults to 1 and diffuse weight defaults to 0, so a viewport that never sets a shade keeps the F-060 colour. The light direction is the walk axis, toward the near end of the ray. The normal is the central difference of stored samples. A missing neighbour contributes 0 on that axis. A zero gradient leaves the normal undefined and the shade equal to the ambient weight. The shaded colour is `colour * shade`. The transfer opacity is not multiplied again.

`shade = ambient + diffuse * max(0, dot(n, light))`, with `n` and `light` unit vectors when the gradient is non-zero.

## Problem and reproducer

The composite ray emits the VOI colour at the transfer opacity and does not look at neighbouring voxels. A caller cannot ask for a shaded ray.

Trigger: three stored samples on the walk axis, 0, 200, 0, spacing 1, one step at the centre. The central difference on that axis is 0 at the centre if both neighbours are 0 and 200 is the centre. A clearer fixture is stored samples 100, 200, 300 along the axis. The centre gradient is `(300-100)/2 = 100` per millimetre when spacing is 1. With ambient 0 and diffuse 1 and the light along that axis, the centre colour is the VOI colour. With the light against that axis, the diffuse term is 0 and the colour is 0.

## Scope, API and compatibility

In scope: optional shade weights on the existing composite ray, computed in the fragment shader from stored samples already being read. Out of scope: a second light, specular, a compute gradient volume, clipping, and a CPU ray.

```rust
pub struct ShadeWeights {
    pub ambient: f32,
    pub diffuse: f32,
}
```

`ShadeWeights::unlit()` is ambient 1 and diffuse 0. `from_weights` refuses a non-finite weight or a weight outside `0.0..=1.0` and leaves the previous weights in place.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Volume ray fragment | `ShadeWeights` uniform | Default unlit | F-060 colour unchanged |
| `set_shade` | same uniform | After bind | Next tick. No brick upload |
| Tier C | admission | CPU | `TierCUnavailable` |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Unlit ray | `set_shade` of a legal pair | Next frame uses the equation | None |
| Any shade | illegal weight | Error | Previous weights unchanged |
| Zero gradient | any legal shade | Shade equals ambient | Colour is `colour * ambient` |

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Shade uniform | Ray bind | Viewport slot | The tick's one submit | Refusal does not write it. No extra submit |

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Unlit default, one sample | F-060 flat colour | A diffuse term on by default | Host fixture |
| AC2 | Samples 100, 200, 300, ambient 0, diffuse 1, light along the gradient | Centre colour equals the VOI colour | A darkened centre | Host fixture of the dot product |
| AC3 | Same samples, light reversed | Centre colour 0 | The unlit colour | same |
| AC4 | Illegal weight | Previous weights kept | The bad pair stored | Host, no adapter |
| AC5 | Tier C | Unavailable before a sample | A CPU shaded image | Host |

## Approach

Extend the ray fragment after F-060's opacity load. Read the two stored neighbours on each axis when they are in the same bound brick. Do not allocate a gradient texture. Do not add a compute pass. Weights ride a small uniform beside the opacity table.

## Dependencies, write ownership and resources

Depends on F-059, done. Source dependency on F-060 because both edit `raycast.wgsl`. Serial after F-060. GPU exclusive.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-render --test raycast`. Mutating the diffuse sign must fail AC2 or AC3. Feature profile at completion.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): full
- Tier B (WebGL2): full, fragment only
- Tier C (CPU): unavailable

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Central difference and the dot product on stored 100, 200, 300 | `ocelli-render` host tests |
| refusal | Weight outside 0 to 1 | same |
| tier | CPU refusal | same |

The stored values are not a DICOM window fixture. The colour scale, when the shade is 1, still cites PS3.3 C.11.2.1.3.2 through the existing VOI.

## Parity surface

Appendix B does not name a volume light. This story adds an optional fragment shade. It does not claim a vtk light kit.

## Deviations and LLD impact

No new deviation. `docs/lld/render-graph.md` gains the shade equation and the unlit default.

## Open questions

None. The unlit default is required so F-060's measured colours stay the default image.
