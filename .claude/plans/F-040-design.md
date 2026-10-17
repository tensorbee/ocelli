# F-040, Texture upload path: decode straight to write_texture

**Status**: approved
**Epic ref**: E6.4
**Sprint**: S13
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/04-boundary-and-data-path.md`, section 6

|  |
|----|
| The LUT chain is an explicit shader stage rather than a CPU pass. All of it reads from one uniform block, so a window-level drag uploads a few bytes per frame instead of re-uploading a texture. It is also the single riskiest piece of arithmetic in the project, which is why §18 specifies it to the formula and why the oracle diffs LUT values as well as pixels. |

### `docs/hld/15-lut-chain.md`, section 18.4, the performance sentence

|  |
|----|
| A window-level drag updates thirty-two bytes per frame. No texture is re-uploaded, and that is the concrete performance claim behind Figure 2. |

### `docs/hld/23-performance-rules.md`, section 26

|  |
|----|
| No allocation in the render loop. |
| One queue.submit() per frame. |
| Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload. |

### `docs/hld/17-cache-and-allocation.md`, section 20

|  |
|----|
| **No allocation in the render loop.** Pre-size everything at viewport creation. A frame that allocates is a frame that can stutter. |
| **Use bytemuck::cast_slice** for reinterpreting pixel buffers. Hand-written transmutes are unsafe code with no upside here. |
| **Reuse staging buffers by size class.** Texture uploads should draw from a small pool, not allocate. |

### Pinned wgpu 30.0.1, `Queue::write_texture`

`queue.write_texture(texture, data, data_layout, size)` copies `data` into the texture. The existing graph tests already call it with `TextureFormat::R32Float` and `bytemuck::cast_slice`. This story makes that the production path.

### Section 18.3 fixture, with deviation D-13

The shader and the CPU chain already agree on this fixture. D-13 corrects the HLD table's `LINEAR_EXACT(-160)` from `1.594` to `0.000`, because section 18.2's comparison is `<=` and `-160` is the breakpoint. This story uses the corrected row. It does not retune `FIXTURE_TOLERANCE` (`0.001` in `tests/voi_shader.rs`).

## What the specification does not cover

1. **Section 20's staging pool and the story title name two mechanisms.** The title is `write_texture`. That call takes a slice and does not take a `wgpu::Buffer`. A size-class pool of staging buffers would be a second path the title does not ask for. This story calls `write_texture` on the caller's slice. No per-frame `Vec` is allocated. The pool is not built.

2. **The texture format.** The graph samples `texture_2d<f32>` as `R32Float`. WebGPU guarantees that format for `TEXTURE_BINDING`. A downlevel device might not. At texture creation the code asks `adapter.get_texture_format_features(R32Float)`. When `TEXTURE_BINDING` is absent the upload returns `Unavailable` and does not substitute a normalised format. Quantising Hounsfield units into `Rgba8Unorm` would be a different number. F-042 owns a tier B shader variant if a preserving format exists there. This story does not invent one.

3. **What the texels are.** They are stored samples as `f32`, which is what `lut_chain` in `voi.wgsl` consumes before slope and intercept. Uploading windowed or display values would apply the LUT twice. The function refuses a slice whose length is not `width * height`.

4. **The carried LINEAR finding.** `docs/lld/pixel-pipeline.md` records that a legal `LINEAR` evaluation can fall outside the declared output range. This story uploads stored samples and does not clamp them, and it does not change the shader's output range. The finding stays where F-041 left it.

5. **F-035's transfer is not a precondition.** The upload takes `&[f32]` already in Rust. How those bytes arrived is F-035. This story does not read JavaScript.

## Approach

`Upload::stored` creates the `R32Float` texture at registration, or returns `UploadError::FormatUnavailable`. The texture lives in the viewport's GPU slot beside the uniform buffer F-038 already creates there.

`upload_stored(&mut self, ctx, viewport, samples: &[f32])` calls `queue.write_texture` with `bytemuck::cast_slice(samples)` and marks `Dirty.data`. It increments an `uploads` counter. `set_voi` does not. A test changes the window and asserts `uploads` is unchanged while the readback changes. A test replaces the samples and asserts `uploads` increased by one.

The texture is not recreated on upload when the extent matches. A different extent replaces it at that call, which is a data change, not a frame-loop allocation. The render loop itself only binds the existing view.

The fixture uploads the four section 18.3 inputs as stored values, with slope 1 and intercept 0, centre 40, width 400, output 0 to 255. The readback matches `LutChain::map_into` within `0.001`. The corrected `LINEAR_EXACT` row at `-160` is `0.000`. A second draw after `set_voi` to a different centre matches the new chain and does not increment `uploads`.

Bytes do not cross into JavaScript. The function is Rust. No `wasm_bindgen` attribute is added.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The texture and the uniform exist before the frame. `write_texture` runs on a data change, not per frame and not on a window change.
- unsafe: none
- Tier A (WebGPU): full, `R32Float`
- Tier B (WebGL2): full when the device can sample `R32Float`. Otherwise unavailable, with no substitute format.
- Tier C (CPU): unavailable. Tier C has no device and no texture. The CPU path remains `LutChain::map_into`.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Section 18.3 rows, D-13 applied, uploaded as stored `f32` and read back through the existing shader within `0.001`. Cites PS3.3 C.11.2.1.2 and C.11.2.1.3.2. | `ocelli-render` ignored GPU test |
| unit | `set_voi` leaves `uploads` unchanged. A new sample slice increments it by one. | the same test |
| unit | A slice of the wrong length is refused and uploads nothing. | `ocelli-render` unit test, no GPU |
| unit | `FormatUnavailable` when the format features lack `TEXTURE_BINDING`, using a stub of the feature flags rather than a fake device. | `ocelli-render` unit test, no GPU |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column.

## Deviations

None.

## LLD impact

`docs/lld/render-graph.md`, for the upload, the counter, and the format refusal.

## Open questions

None. The format refusal is decision D6 and deviation D-07. A tier that cannot sample the stored values reports unavailable and does not produce a different image.
