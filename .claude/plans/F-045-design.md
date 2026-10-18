# F-045, Stack viewport: display, fit, pan, zoom, rotate, flip

**Status**: approved
**Epic ref**: E7.1
**Sprint**: S15
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/03-architecture-and-crates.md`, section 4, the crate row

| **Crate** | **Responsibility** | **wasm** | **native** |
|----|----|----|----|
| ocelli-viewport | Viewport and scene model, camera, presentation state | yes | yes |

The crate exists and holds the scaffold only. This story fills the camera half. Presentation state is F-046. Scene documentation is F-100.

### `docs/hld/13-core-types.md`, section 16, the signatures

```rust
// ocelli-core/src/space.rs
use core::marker::PhantomData;
/// CSS pixels inside a viewport element. Origin top-left, y increases down.
pub enum Canvas {}
/// DICOM patient coordinate system (LPS), millimetres.
pub enum World {}
/// Voxel indices within a volume. Origin at voxel (0,0,0).
pub enum Index {}
#[derive(Debug, PartialEq)]
pub struct Pt<S> { pub x: f64, pub y: f64, pub z: f64, _s: PhantomData<S> }
// NOTE: derive(Clone, Copy) would add an S: Clone bound that the marker
// types do not satisfy. Implement by hand.
impl<S> Clone for Pt<S> { fn clone(&self) -> Self { *self } }
impl<S> Copy for Pt<S> {}
impl<S> Pt<S> {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z, _s: PhantomData }
    }
}
pub struct Transform<A, B> { m: glam::DMat4, _p: PhantomData<(A, B)> }
impl<A, B> Transform<A, B> {
    pub fn apply(&self, p: Pt<A>) -> Pt<B> { /* ... */ }
    pub fn inverse(&self) -> Transform<B, A> { /* ... */ }
    pub fn then<C>(&self, next: &Transform<B, C>) -> Transform<A, C> { /* ... */ }
}
```

Canvas origin is top-left and y increases down. A pan is a `Pt<Canvas>`. It is not a `Pt<World>` and it is not a bare list of numbers. F-048 owns `Transform<Canvas, World>`. This story does not construct one.

### `docs/hld/19-render-graph.md`, section 22

```rust
pub enum Pass {
    Stack(StackPass),
    VolumeRaycast(VolumePass),
    SegOverlay(SegPass),
}
```

```text
Pipelines compile at init, keyed by (pass kind, blend mode, tier). Never compile a shader mid-frame.
Dirty tracking. A viewport re-renders only when camera, VOI, slice, segmentation or data changed. Everything else is a no-op frame.
One queue.submit() per frame across all viewports, from the render worker's requestAnimationFrame.
```

`Dirty.camera` already exists. `Pass::Stack` already exists. This story adds the camera uniform those words name. It does not add a pass kind.

### `docs/hld/23-performance-rules.md`, section 26

```text
Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload.
No allocation in the render loop.
One queue.submit() per frame.
```

The thirty-two bytes are `VoiParams`. The camera is a different uniform. A pan does not write `VoiParams` and does not upload the stored texture.

### `docs/hld/14-the-boundary-in-code.md`, section 17.4, the camera field this story's bytes must match

```rust
pub struct ViewportState {
    pub camera: [f32; 16],
    pub voi_center: f32,
    pub voi_width: f32,
    pub slice_index: u32,
    pub num_slices: u32,
    pub flags: u32, // bit 0 invert, bit 1 loading, bit 2 error
    pub _pad: [u32; 3],
}
```

F-101 copies this struct out. This story does not. The sixteen floats it writes are that `camera` field, column-major, so the later copy is the same bytes.

### Deviation D-07, the tier C sentence

```text
What tier C claims. A stack viewport renders, windows, scrolls and measures on the CPU.
```

CURRENT_SPRINT assigns that raster to F-X003. This story reports drawing unavailable on tier C. It does not draw a different image on the CPU.

## What the specification does not cover

The HLD names a stack pass, a camera dirty flag and a sixteen-float camera. It does not give fit, pan, zoom, rotate or flip. The decisions below are the ones this plan makes. F-046, F-047, F-049 and F-100 use the same names.

1. **Display is the mapping that already ships.** `fs_main` loads `textureLoad(stored_tex, vec2<i32>(i32(at.x), i32(at.y)), 0)`. WebGPU fragment position is the pixel centre, so `i32` of that coordinate is the texel. The default camera is the identity on that coordinate. `the_section_18_3_rows_render_through_the_graph` reads red `[0, 128, 255, 64]` from a target whose width equals the input count. That expected row stays. Fit is a separate call. Registration does not fit.

2. **One matrix, applied to the fragment coordinate.** The vertex shader stays the full-target triangle. The fragment multiplies `(at.x, at.y, 0, 1)` by the camera matrix and truncates with `i32` toward zero to a texel. Identity reproduces today's load. The matrix is sixteen `f32`, column-major, matching section 17.4. It is group 0 binding 2. Bindings 0 and 1 stay the VOI uniform and the colour uniform.

3. **Outside the image is black, not a windowed zero.** A fragment whose image coordinate is outside `[0, columns)` by `[0, rows)` writes `(0, 0, 0, 1)` and does not run `lut_chain`. Equal extents never hit this path, which is why the section 18.3 row is unchanged. `textureLoad` of an out-of-range coordinate returns zero today and would window it. That is not the behaviour of a letterboxed margin.

4. **`set_view` is the writer.** It writes the sixty-four bytes and sets `Dirty.camera`. It does not write the thirty-two-byte VOI uniform, the colour uniform, the stored texture or a brick. `uploads`, `lut_uploads` and `brick_uploads` stay put. `mark_dirty` still accepts `camera`, because `surface.rs` marks it on a resize that keeps the identity. A fit, pan, zoom, rotate or flip goes through `set_view` and not through `mark_dirty` alone.

5. **The math lives in `ocelli-viewport` and is `no_std`.** The render crate stores the matrix and samples with it. It does not decide fit. The viewport crate does not depend on `wgpu`.

6. **Composition order, y down, about the image centre.** Image space has its origin at the top-left corner of texel `(0, 0)`, x along columns, y along rows. The image centre is `(columns / 2, rows / 2)`. Operations on an image-space point, in this order:

   - flip in x about the centre, then flip in y about the centre
   - rotate about the centre by the stored radians, clockwise on the screen
   - scale by `fit_scale * zoom`
   - translate by the fit offset, then by `Pt<Canvas>`, then by the index shift scaled into canvas pixels

   Clockwise on a y-down plane sends `(dx, dy)` at a quarter turn to `(-dy, dx)`. `Pt<Canvas>.z` must be `0`. A non-zero z is refused, so a slice index cannot hide in a pan. Zoom is a newtype. Radians are a newtype. They are not bare `f64` arguments beside each other.

7. **Fit letterboxes the axis-aligned bounds of the transformed image.** `fit` sets pan to zero and zoom to `1`, and it keeps rotation and flip. The scale is the largest one that places those bounds inside the canvas. The leftover margin is split equally on that axis. Spacing defaults to `1` mm and `1` mm. The index shift defaults to `0` and `0`. F-049 is what stores a real spacing and a real shift. Both fields are already inputs of this matrix, so F-049 does not edit the shader.

8. **Zoom keeps the image centre fixed.** It multiplies the zoom newtype and recomputes the translation. It does not pan.

9. **Coordinate transforms are not this story.** No `Transform<Canvas, World>`. No world point. No index point except the shift F-049 will store as two `f64` image-pixel offsets, which are not a `Pt<Index>`.

## Approach

`View` in `crates/ocelli-viewport/src/view.rs` holds the pan, the zoom, the rotation, the two flips, the column and row millimetres, and the column and row index shift. `set_spacing` and `set_index_shift` are the writers F-049 calls. Defaults are `1` mm, `1` mm, shift `0`, shift `0`. `image_to_canvas` is the pure function the fixtures call. `camera_matrix` writes the sixteen floats that map a canvas pixel coordinate to an image coordinate, which is the inverse of that function, so the fragment shader can start from `@builtin(position)`.

`RenderGraph::set_view` writes those bytes into a uniform created at registration. The identity matrix is what registration writes, so a viewport that never calls `set_view` samples as it does today. The pipeline is still compiled at creation. The new binding is part of that layout. There is no mid-frame compile.

`ocelli-viewport` gains a dependency on `ocelli-core` only. `lib.rs` declares `pub mod view` and nothing else. F-046, F-047 and F-049 each add their own module line when they land.

The TypeScript command names are F-100's `packages/core/src/viewport.ts`. This story does not add a second declaration and does not reimplement the matrix in TypeScript.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The uniform buffer is created at registration. `set_view` writes into it.
- unsafe: none
- Tier A (WebGPU): full. The stack pass samples through the camera matrix.
- Tier B (WebGL2): full. The same fragment module, the same matrix. No compute and no storage buffer.
- Tier C (CPU): unavailable for drawing. `RenderGraph::new` already returns `TierCUnavailable`. The `View` math runs with no device and is the same math the GPU uniform carries. F-X003 is the CPU raster. This story does not rasterise one.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Identity on canvas `(0.5, 0.5)` is image `(0.5, 0.5)`, so `i32` truncation is texel `(0, 0)`. Canvas `(3.5, 0.5)` on a width-4 target is texel `(3, 0)`. | `crates/ocelli-viewport/tests/view.rs` |
| fixture | Fit of a 2 by 4 image into an 8 by 8 canvas, spacing `1` and `1`, no rotation. Scale `2`, offset `(2, 0)`. Canvas centre `(3, 1)` maps to image `(0.5, 0.5)`. | same |
| fixture | Quarter turn clockwise of image point `(0, 0)` on a 2 by 2 image lands at `(2, 0)`. The centre is `(1, 1)`. The vector `(-1, -1)` goes to `(1, -1)`. | same |
| fixture | Horizontal flip of a 2 by 2 image sends image x `0` to x `2` about the centre. | same |
| fixture | Fit after that quarter turn letterboxes the swapped bounds. Image 2 by 4 becomes bounds 4 by 2. Scale `2`, offset `(0, 2)` in an 8 by 8 canvas. | same |
| fixture | Pan of `Pt<Canvas>::new(1, 0, 0)` moves the image one canvas pixel to the right. `z` of `1` is refused. | same |
| fixture | Zoom `2` about the fitted centre keeps that centre and doubles the scale. The stored samples are not an input of `camera_matrix`. | same |
| unit | `set_view` writes 64 bytes, sets `Dirty.camera`, and leaves `uploads`, `lut_uploads` and `brick_uploads` unchanged. The VOI uniform bytes are unchanged. | `crates/ocelli-render/tests/view.rs`, GPU, ignored for the gpu gate |
| regression | `the_section_18_3_rows_render_through_the_graph` still reads `[0, 128, 255, 64]` with no edit to that expected row. | existing `graph.rs` test |

The fit, rotation and flip rows are hand-computed from the composition in this plan. They cite HLD section 16 for the canvas axis, not a DICOM pixel formula. F-049's rows cite PS3.3.

Mutate the clockwise quarter-turn once, from `(-dy, dx)` to `(dy, -dx)`, and confirm the quarter-turn fixture goes red. Revert.

## Parity surface covered

`docs/hld/B-parity-surface.md` has no per-epic rows and no `Covered by` column. The aggregate row this story starts is `Viewport types`, whose note lists `STACK`. The other eleven types are later stories. Tool classes are not this story.

## Deviations

none

## LLD impact

- `docs/lld/viewport.md`, created, for `View` and the camera matrix
- `docs/lld/README.md`, one index row for that file
- `docs/lld/render-graph.md`, the camera uniform and `set_view`

## Open questions

None. The choices the HLD does not make are the numbered decisions above, and the same decisions are the ones F-046, F-047, F-049 and F-100 are written against.
