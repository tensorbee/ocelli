# F-194, stack display interpolation

**Status**: approved
**Epic ref**: E38.4
**Sprint**: S42
**Estimate**: 3w

## Normative source, transcribed

HLD section 18 says the LUT arithmetic exists once in `ocelli-pixel` and the shader reads parameters. This story does not add a window function.

The render graph loads stack texels with `textureLoad`. `crates/ocelli-render/src/graph.rs` states that both textures are read with `textureLoad` and that no sampler state decides the texel. CPU `texel_of` in `cpu.rs` returns the integer texel that contains the image coordinate. That is nearest-neighbour.

HLD section 7 and deviation D-07 require an answer for tier A, tier B and tier C. A tier that cannot run the requested behaviour reports unavailable and does not substitute another result.

`docs/hld/B-parity-surface.md` has no interpolation row. This story covers none. It does not change an oracle tolerance.

## What the specification does not cover

The HLD does not name nearest or linear as a stack display mode. The story does. Linear here is a 2 by 2 blend of the display texels around a continuous image coordinate. It is not a wgpu `FilterMode`, because the graph deliberately has no sampler, and a GPU filter would diverge from `textureLoad` on tier B where float textures are not filterable.

The blend, for image coordinates `x` and `y` in texel space with the origin at the centre of texel `(0,0)` matching the existing `INDEX_TO_IMAGE_CENTRE` shift:

```text
x0 = floor(x)
y0 = floor(y)
tx = x - x0
ty = y - y0
nearest = texel(round half away from -inf via the existing texel_of rule)
linear = (1-tx)*(1-ty)*p00 + tx*(1-ty)*p10 + (1-tx)*ty*p01 + tx*ty*p11
```

`texel_of` stays the nearest implementation. Linear reads the four texels around that coordinate. A coordinate whose 2 by 2 footprint leaves the image is unavailable for linear at that pixel, and the pixel stays the previous clear value. It is not filled with a clamped edge texel, because clamping would be a third mode.

The same function lives once in `ocelli-render` and is called from the CPU raster and from a small pure function the WGSL is checked against. The shader gains an explicit branch on a uniform `interpolation`. `0` is nearest. `1` is linear. Any other value draws nothing and reports unavailable. The CPU path uses the same integer. Tier A and tier B compile the same WGSL branch. If tier B cannot compile that branch, tier B reports linear unavailable and still draws nearest when nearest was requested. It does not draw nearest in response to linear.

Weights are `f32` in the shader and `f32` in the CPU blend of display samples. An 8-bit canvas stores that result through the existing `display_index`, which is nearest integer with ties up. The test fixture compares the `f32` blend before that step, so a rounding change cannot hide a weight error.

## Problem and reproducer

A 2 by 2 display image, values `[[0, 100], [200, 255]]`. Sample at the centre of the four texels, which is image coordinate `(0.5, 0.5)` after the centre shift the CPU path already uses. Nearest follows `texel_of`. Linear is the mean of the four values, 138.75. Current behaviour: only nearest exists, and there is no mode to ask for linear. A caller cannot observe an unavailable tier.

## Scope, API and compatibility

In scope:

- `Interpolation` in `ocelli-render` with `Nearest` and `Linear`.
- Pure function `blend_display(samples, x, y, mode) -> Result<f32, InterpolationError>`.
- CPU raster takes the mode. Default for existing callers is `Nearest`, so current CPU tests keep their pixels.
- WGSL uniform integer. Existing pipelines set `0` and keep current images.
- Tier report: `available(mode, tier) -> bool`. All three tiers are available for both modes in the plan. The unavailable path is the unknown mode, plus a test double that marks tier B linear unavailable and asserts the draw is refused rather than nearest.
- Example panel. The helper, which already links `ocelli-render` for this route only if the crate's native target allows it. `ocelli-render` is native-capable today. The route `POST /interpolate` sends the 2 by 2 fixture and returns nearest, linear, and the three tier rows. The page shows both numbers and the tier rows. A control switches the sentence for the selected mode. It does not silently show the other number.

Out of scope:

- Volume raycast sampling. Existing MPR and raycast stay on their current filters.
- Changing `textureLoad` to a sampler.
- A new tolerance in the oracle.
- In-plane magnification quality beyond this 2 by 2 blend.

```rust
pub enum Interpolation {
    Nearest = 0,
    Linear = 1,
}

pub enum InterpolationError {
    Outside,
    Unavailable,
}

pub fn blend_display(
    width: u32,
    samples: &[f32],
    x: f32,
    y: f32,
    mode: Interpolation,
) -> Result<f32, InterpolationError>;
```

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| CPU raster | `blend_display` | Existing views pass `Nearest` | Same pixels as before for nearest tests |
| WGSL stack sample | the same weights, written out in the shader | Uniform 0 or 1 | `f32` within 1 ULP of `blend_display` on the fixture |
| Tier B probe | same shader source | Compile plus the fixture when the adapter exists | Same `f32`, or a recorded skip when no adapter |
| Example panel | helper route | The 2 by 2 fixture |  nearest value, 138.75, three available rows |

The GPU comparison is 1 ULP at the `f32` blend, not a new image tolerance. Tests that already ignore the GPU keep that ignore and the ignore message stays.

## State transitions and invariants

The mode is part of the draw inputs. Changing it invalidates the CPU frame. An unavailable mode does not replace the last successful frame. The page shows the error and keeps the last numbers.

Invariant. Requesting linear never returns the nearest texel as a successful linear result. The centre sample 138.75 is not equal to 0, 100, 200 or 255, so a nearest implementation fails AC2.

## Ownership, commit and recovery

No new GPU allocation in the render loop. The uniform is one integer in the existing uniform buffer if there is a spare field, or one new 4-byte field padded to the struct alignment the shader already uses. The plan prefers a spare field. If none exists, the uniform grows and every writer is updated in this story. That growth is a pipeline rebuild at mode-change time, not per pixel.

A repeated request with the same mode is a new blend with the same result. It does not allocate a texture.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | 2 by 2 values and the coordinate `texel_of` already maps to a single texel. Nearest returns that texel. | CPU unit citing the existing `texel_of` rule | Linear weights used for nearest | implementation |
| AC2 | Same image at the four-texel centre. Linear returns 138.75. | Fixture. Weights are one quarter each. | Returning any corner | implementation |
| AC3 | Coordinate outside the image. `InterpolationError::Outside`. | Unit | Clamped edge sample | implementation |
| AC4 | Mode value 2 on the CPU path. `Unavailable`. The previous frame buffer is unchanged. | Unit | Falling back to nearest | implementation |
| AC5 | Existing CPU nearest tests still pass with the default mode. | Existing `cpu` tests | A default of linear | implementation |
| AC6 | Shader source contains the linear weights and the nearest `textureLoad` path. | Source test, same style as the VOI shader tests | A sampler filter as the only switch | implementation |
| AC7 | Page shows 138.75 for linear and the nearest corner for nearest, plus three tier rows that say available. | curl | Numbers only in Rust | integrated |

AC6 is a source test, not a GPU run. A machine with an adapter may add a later assertion that the GPU blend matches 138.75 within 1 ULP. That assertion stays ignored with the existing GPU ignore message when no adapter is present. A skip is not a pass of the ULP row. The row's required evidence is AC2 and AC6. The ULP run is additional when a GPU exists.

## Approach

Add `interpolation.rs` in `ocelli-render`. CPU `sample_at` calls it for the display buffer it already indexes. Nearest keeps `texel_of` so the binary search and the centre shift stay one implementation. Linear does not reimplement `texel_of`.

The shader change is the smallest branch at the existing `textureLoad` site. Weights match the formula above. No new render pipeline type.

The example route calls `blend_display` twice and reports tier availability from a const table. The table is the declaration the story requires. A tier marked false in a test returns unavailable from `available`.

Rejected. `wgpu::FilterMode::Linear`. A second blend in the example page. Changing nearest's integer rule.

## Dependencies, write ownership and resources

Depends on F-042 and F-045, both done. Serial after F-193 because `App.tsx` and the example server are shared. Render files are otherwise free of F-191 to F-193.

Writes:

- `crates/ocelli-render/src/interpolation.rs`
- the CPU sample site and the stack shader
- `crates/ocelli-render/tests/interpolation.rs`
- example server route and `App.tsx` panel
- `docs/lld/render-graph.md` gains one paragraph at completion, not a rewrite

GPU tests stay ignored without an adapter. One compiler at a time. This story does not run beside another `cargo` or `wasm-pack`.

## Verification and evidence reuse

```bash
bin/ocelli.sh test ocelli-render interpolation
bin/ocelli.sh clippy ocelli-render
bin/ocelli.sh test ocelli-render cpu
```

`cpu` is the nearest regression. Do not run the full workspace test in a loop. The sprint gate is later.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The blend writes into the existing destination.
- unsafe: none
- Tier A (WebGPU): full for nearest and linear, same weights
- Tier B (WebGL2): full for nearest and linear when the shared shader compiles. Unavailable for linear if that compile fails, and the failure is reported. No silent nearest.
- Tier C (CPU): full for nearest and linear, same function

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC2, the value 138.75 from four equal weights | `interpolation.rs` tests |
| unit | AC1, AC3, AC4, AC5 | same, plus existing CPU tests |
| source | AC6 shader branch | shader source test |
| integrated | AC7 curl | example server |

## Parity surface covered

None.

## Deviations

None. D-07 is applied, not amended.

## LLD impact

One paragraph in `docs/lld/render-graph.md` naming the uniform and the shared blend. No new tolerance section.

## Open questions

None.
