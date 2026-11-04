# F-194 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s42` over parent `c90bdcd`, as saved in `/tmp/ocelli-f194.diff`, plus untracked `interpolation.rs`, `tests/interpolation.rs` and `interpolate.rs`. No tree hash. The reviewer had no shell.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 4 defects, 3 smells, 3 nitpicks

## Acceptance challenged

AC2 at `(0.5, 0.5)` is 138.75 for every equal-weight formula, so it cannot assign corners. The coordinate passed into linear was the corner-origin image coordinate. Mode 2 on a raster that is already nearest does not reject a fallback to nearest. The linear draw allocated inside the draw.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| Read and grep only | working tree | executed now | No test was run in this pass |
| Grep `set_interpolation` and `set_stack_interpolation` | `*.rs` | executed now | No production caller of `set_stack_interpolation`. The only `set_interpolation` call was mode 2 |

## Defects

### D1, linear was half a texel off

**Where**: `cpu.rs` centre shift into `blend_display`, and the shader `floor(mapped.x)` branch.

**What**: Image space stores the corner of texel 0. The centre of that texel is image coordinate 0.5. `blend_display` treats 0 as that centre. Both callers passed the corner-origin value. A 1:1 pixel on texel 0 therefore blended all four texels, and the last row and column fell outside the footprint.

**Why it is wrong**: `INDEX_TO_IMAGE_CENTRE` in `ocelli-viewport` is the image coordinate of index 0, not a centre-origin origin. The approved fixture `(0.5, 0.5)` is the four-texel corner in centre-origin space. It is not the canvas sample of texel 0.

**Evidence**: derived from `view.rs` and `cpu.rs`. Not executed in this pass.

**Scope and regression**: tiers A, B and C. The regression is an identity 2 by 2 linear draw whose pixel `(0, 0)` matches nearest and whose pixel `(1, 1)` matches nearest.

### D2, the centre fixture cannot see a swapped corner

**Where**: `tests/interpolation.rs` linear centre test.

**What**: At `(0.5, 0.5)` every weight is 0.25. Swapping the two middle samples stays at 138.75. Hand value at `(0.25, 0.75)` is 166.5625. The swapped formula is 116.5625.

**Why it is wrong**: a test that passes both ways is not a test.

**Evidence**: derived by hand. Not executed in this pass.

**Scope and regression**: bit-compare `blend_display` at `(0.25, 0.75)` with 166.5625.

### D3, mode 2 did not reject a fallback to nearest

**Where**: `tests/interpolation.rs` CPU mode test.

**What**: The raster starts at nearest. Asserting it is still nearest after mode 2 also passes an implementation that resets to nearest. No settled frame was compared.

**Why it is wrong**: AC4 names that fallback as the wrong behaviour.

**Evidence**: read `set_interpolation`. The implementation returned before assignment. The test did not show it.

**Scope and regression**: set linear, refuse mode 2, assert the mode is still linear, then assert the next settled draw does not write.

### D4, the linear draw allocated

**Where**: `fill_display`, `fill_rgb_planes`, and `display_linear`.

**What**: The first linear draw built a `Vec` inside the draw. `display_at` built one on every call.

**Why it is wrong**: the raster doc says shading does not allocate, and the design says the render loop does not allocate.

**Evidence**: read the call path.

**Scope and regression**: `reserve_source` owns the plane. A linear draw after that reserve leaves `allocations` unchanged.

## Smells

### S1, the tier table was not on the draw path

**Where**: `available` and `set_stack_interpolation`.

**Mechanism**: flipping `available` would change the example page and leave the graph drawing the refused mode.

### S2, a non-integer lane truncated into a real mode

**Where**: `u32(camera.scale.y)` before the refusal.

**Mechanism**: `0.5` became nearest and `1.5` became linear. The only writer stores exact 0 or 1, so the prose claim that any other value draws nothing was false for every other finite value.

### S3, two copies of the centre-shift conversion

**Where**: `display_linear` and `blended_display`.

**Mechanism**: a coordinate fix in one copy leaves `display_at` disagreeing with the drawn frame.

## Nitpicks

- `number()` special-cases the bits of 0 and 138.75.
- `Interpolation::mode` has no caller.
- `device_camera_words` always uploads nearest.

## Verified clean

The scalar shader and the CPU mono path both ran `lut_chain` before the blend. Palette linear blended table colours. Mode 2 returned before writing the slot or the stored mode. The weight text matched `blend_display`. The nearest path still contained `textureLoad(stored_tex, texel, 0).r`. The page sentence used one number. The default mode was nearest. No `unsafe`, no new `wasm-bindgen`, no sampler, and no second window function.

## Limitations and handoff

This pass ran no test. The implementer remediates D1 to D4 and S1 to S3, then pass 2 reviews that tree. This pass is not sprint readiness.
