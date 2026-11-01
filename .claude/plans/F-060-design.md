# F-060, transfer-function editing surface and preset library

**Status**: approved
**Epic ref**: E9.2
**Sprint**: S21
**Estimate**: 3w

## Normative source, transcribed

From `docs/hld/05-rendering.md` section 7:

> Two capability tiers, one codebase. Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs. The tier resolves once at startup.

> Volume rendering runs on both tiers, because a 3D-texture ray-cast in a fragment shader is tier-B legal.

> One submit per frame. The render graph tracks dirty viewports and issues a single submission across all of them, driven by requestAnimationFrame inside the render worker.

From `docs/hld/16-volume-representation.md` section 19, the volume has one `origin`, one `spacing` and one `direction`.

`docs/lld/render-graph.md` records the integral this story replaces. Opacity is the constant 0.5. Colour and alpha accumulate `(1 - A) * colour * 0.5` and `(1 - A) * 0.5`. The pipeline blend is off. `display_index` in `crates/ocelli-render/src/colour.rs` addresses a table: a non-finite display or a display at or below 0 uses entry 0, a display at or above `last` uses `last`, and a finite value in between uses the nearest integer with ties up.

The HLD does not name a transfer-function formula or a preset list.

## What the specification does not cover

The opacity table, its length, the preset names, and the refusal of a bad table are repository choices. They are not HLD quotations. The default table is every entry `0.5`, because that is the integral F-059 already locked. A second preset exists so a test can see that the table is read. A caller table is 256 finite entries, each in `0..=1`.

## Problem and reproducer

`shaders/raycast.wgsl` sets `let alpha = 0.5` on every step. `RenderGraph::set_voi` changes the window and does not change that opacity. There is no preset and no caller table.

Trigger: one resident unsigned voxel whose LINEAR_EXACT display is 200 on output 0 to 255. With the flat preset the colour and alpha stay the F-059 one-step values. With the ramp preset the opacity is `200/255` and the colour is `(200/255) * (200/255)`.

## Scope, API and compatibility

In scope: a 256-entry opacity table on the existing composite ray, two presets, and a caller table. The table is a uniform of 256 `f32` values. A change writes that uniform and does not upload bricks.

Out of scope: lighting, gradient opacity, clipping, a CPU ray, MIP and the other volume blends, fusion, and any change to `lut_chain` or `voi.wgsl`.

```rust
pub enum OpacityPreset {
    Flat,
    Ramp,
}

pub struct OpacityTable {
    entries: [f32; 256],
}

impl OpacityTable {
    pub fn preset(id: OpacityPreset) -> Self;
    pub fn from_entries(entries: &[f32]) -> Result<Self, OpacityError>;
    pub fn entries(&self) -> &[f32; 256];
}
```

`Flat` is `0.5` at every index. `Ramp` entry `i` is `f32::from(i) / 255.0`. `from_entries` refuses a length other than 256, a non-finite value, or a value outside `0.0..=1.0`, and the previous table stays in place.

The shader index is `display_index(display, 255)` after `lut_chain`. The address is not the normalised `colour` already used as the grey level, and it is not the 1024-wide grey ramp.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `RenderGraph` volume viewport | `OpacityTable` uniform | Default `Flat` at bind | F-059 one-step colour and alpha |
| `set_opacity` | same uniform | Preset or caller table | Next tick uses the new alpha. Bricks stay |
| Ignored GPU ray tests | readback | One resident sample | Flat matches F-059. Ramp matches the hand product |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Bound ray, flat table | `set_opacity(Ramp)` | Uniform replaced. Next frame uses ramp alpha | None |
| Bound ray, any table | `from_entries` of a bad slice | Call returns the error | Previous uniform unchanged |
| Tier C | any opacity call | `TierCUnavailable` | No image and no table write |

A repeated `set_opacity` of the same bytes does not upload bricks and does not allocate a second uniform.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Opacity uniform | Bind of the volume ray | The viewport slot | `queue.submit` of the tick that encodes the ray | A refused table does not write the uniform. Device loss keeps the existing F-037 recovery and does not invent a CPU frame |

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Flat preset, display 200, one step | Colour `(200/255)*0.5`, alpha `0.5` | A ramp or a constant other than 0.5 | Host fixture, then ignored GPU readback |
| AC2 | Ramp preset, display 200, one step | Colour `(200/255)*(200/255)`, alpha `200/255` | The flat integral left in place | Same |
| AC3 | 255 entries, or a value `1.1`, or a NaN | Error, previous table unchanged | The bad table becomes the uniform | Host test, no adapter |
| AC4 | `set_opacity` after a successful bind | Brick upload count unchanged | A window-style texture re-upload | GPU test |
| AC5 | Tier C | `TierCUnavailable` before a sample read | A CPU image | Host test |

## Approach

Add `OpacityTable` beside the ray uniform in `ocelli-render`. `volume_ray_module` keeps one fragment module for tier A and tier B. Replace `let alpha = 0.5` with the table load. Do not add a texture. Do not change `RayHeader`'s 48-byte prefix. The table is a second uniform binding.

## Dependencies, write ownership and resources

Depends on F-059, which is done. Writes `shaders/raycast.wgsl`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/tests/raycast.rs`, `docs/lld/render-graph.md` and `docs/lld/README.md`. The GPU is exclusive. F-061 and F-062 later edit the same shader, so they follow this story in one serial wave.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-render` while iterating. `bin/ocelli.sh test ocelli-render --test raycast`. One constant mutation of the flat value must fail AC1. Feature profile at completion. The F-059 GPU readbacks stay the flat expected colours.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The uniform is created at bind
- unsafe: none
- Tier A (WebGPU): full, the same fragment module
- Tier B (WebGL2): full, the same fragment module. No compute and no storage buffer
- Tier C (CPU): unavailable. No CPU image. F-X005 owns that decision

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Flat and ramp products from PS3.3 C.11.2.1.3.2 LINEAR_EXACT, centre 127.5, width 255, output 0 to 255, stored 200 | `ocelli-render` host tests |
| refusal | Bad length, non-finite, out of range | same |
| tier | CPU returns before a sample | same |

## Parity surface

Appendix B names `VOLUME_3D` as a viewport type and does not name opacity presets. This story makes the composite ray's opacity a table. It does not claim a vtk preset name.

## Deviations and LLD impact

No new deviation. Update the volume-ray paragraphs in `docs/lld/render-graph.md`.

## Open questions

None. The default of 0.5 is the locked F-059 integral. The table length and address are the existing 256-entry `display_index`.
