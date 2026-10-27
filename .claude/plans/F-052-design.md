# F-052, Orthographic viewport: axial, sagittal, coronal

**Status**: approved
**Epic ref**: E8.2
**Sprint**: S19
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/16-volume-representation.md`, section 19

```rust
pub struct Volume {
    pub dims: [u32; 3],
    pub spacing: [f64; 3], // mm
    pub origin: Pt<World>, // IPP of the first slice
    pub direction: glam::DMat3, // derived from ImageOrientationPatient
    pub voxel: VoxelKind, // U8 | I16 | U16 | F32
    pub data: VoxelStore, // one contiguous allocation, x fastest
    pub levels: Vec<Level>, // multiscale; level 0 is full resolution
    pub present: bitvec::BitVec, // per-slice, for progressive assembly
}
```

`origin` is the centre of voxel `(0, 0, 0)`. `direction` columns are the row cosine, the column cosine and the slice normal. `spacing[0]` is column millimetres, along the row cosine. `spacing[1]` is row millimetres, along the column cosine. `spacing[2]` is the median projected gap. F-051 is the construction that writes those fields. This story does not compute them again.

### `docs/hld/13-core-types.md`, section 16

```rust
pub enum Canvas {}
pub enum World {}
pub enum Index {}
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

World points from this story are `Pt<World>`. Voxel coordinates stay `[u32; 3]` or `f64` indexes inside the sampler. They are not `Pt<World>`.

### DICOM PS3.3 C.7.6.2.1.1

```text
P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
```

`X` is the row direction. `Y` is the column direction. `i` is the column index. `j` is the row index. `PixelSpacing[0]` multiplies the column direction cosine. `PixelSpacing[1]` multiplies the row direction cosine. IPP is the centre of the first voxel.

Extended by the constructed volume, with `N` the slice normal and `k` the slice index:

```text
P = origin
    + i * spacing[0] * direction.col(0)
    + j * spacing[1] * direction.col(1)
    + k * spacing[2] * direction.col(2)
```

`ImagePlane::index_to_world` remains the in-plane copy of the first two terms. This story does not re-implement that function. The third term is the volume's own spacing and normal, which a single-frame plane does not carry.

### `docs/hld/05-rendering.md`, section 7

Volume rendering runs on tier A and tier B because a 3D-texture ray cast in a fragment shader is tier-B legal. Compute resampling is tier A only and must degrade rather than fail. This story does not use a compute shader. The orthographic image is host resampling of level 0, then the existing stack viewport.

### `docs/hld/03-architecture-and-crates.md`, section 4

`ocelli-volume` is volume assembly, geometry and reslicing, on wasm and on native.

### `docs/hld/DEVIATIONS.md`, D-07

CPU MPR is required and is F-X004, which depends on this story and is S21. This story does not wire a tier C viewport. A feature that cannot run on the resolved tier reports unavailable and does not draw a different image.

### Sprint defect class

Axial, sagittal and coronal are the volume's own axes. A sagittal sample steps along the column cosine and the slice normal. A sample that steps along the row cosine instead is the swap. The planes are not a second patient frame and not a sort by Instance Number.

## What the specification does not cover

The HLD names the orthographic viewport and does not give the sample kernel or the enum. These choices keep the acquisition plane identical to stored voxels and keep one LUT.

1. **Names follow the volume axes.** `Axial` holds the slice index constant and steps along both in-plane cosines. `Sagittal` holds the column index constant and steps along the column cosine and the normal. `Coronal` holds the row index constant and steps along the row cosine and the normal. A coronal acquisition still uses these names relative to its constructed direction. Re-labeling by absolute LPS axes is not this story.
2. **On a voxel centre the sample is that voxel's container value.** Off-centre indexes use trilinear weights in `f64` over the up-to-eight corners, then narrow to `f32`. A corner whose slice is absent, and whose weight is non-zero, makes the output sample `Absent`. It is not stored zero and it is not a partial blend against zero. A weight of zero does not read that corner, so an integer axial slice does not touch the next slice.
3. **Container decode is little-endian `U8`, `I16`, `U16` or `F32`.** No second Bits Stored mask. `place` already stored those bytes.
4. **The viewport uploads only a fully present plane** through the existing stack path, `register_stored` and `upload_stored`, as `f32` stored values. `RenderGraph::show_ortho` returns `PlaneIncomplete` when any sample is `Absent` and does not write the texture. Windowing stays the stack pass and `VoiParams`. This story does not add a pass variant to section 22's enum.
5. **Tier C does not display the plane.** `reslice` is host arithmetic and runs anywhere. `show_ortho` on a tier C graph returns the existing `TierCUnavailable` and uploads nothing. F-X004 is the CPU viewport.
6. **Level 0 only.** An appended coarse level is not a plane source.
7. **Oblique planes, thick slab, MIP, MinIP, average, fusion and slice sync stay out.** F-053, F-054, F-056 and F-055.

## Problem and reproducer

`Volume::construct` holds a filled grid. Nothing samples a sagittal or coronal plane from it. A viewer that sorted stored frames by Instance Number, or that walked the row cosine for a sagittal line, would show a plausible slice of the wrong axis. Square pixels hide the spacing swap. Equal test values hide the axis swap.

Expected observation, identity direction, origin `(10, 20, 30)`, spacing `[0.5, 2.0, 3.0]`, dims `[2, 2, 2]`, `U8` value `x + 10 * y + 100 * z`:

| Plane | Pixel | Voxel | Value | World point |
|-------|-------|-------|-------|-------------|
| Axial slice 0 | `(1, 1)` | `(1, 1, 0)` | 11 | `(10.5, 22, 30)` |
| Sagittal column 1 | `(0, 0)` | `(1, 0, 0)` | 1 | `(10.5, 20, 30)` |
| Sagittal column 1 | `(1, 0)` | `(1, 1, 0)` | 11 | `(10.5, 22, 30)` |
| Sagittal column 0.5 | `(0, 0)` | blend of `(0, 0, 0)` and `(1, 0, 0)` | 0.5 | `(10.25, 20, 30)` |
| Coronal row 0 | `(1, 1)` | `(1, 0, 1)` | 101 | `(10.5, 20, 33)` |

A sagittal implementation that steps along the row cosine reads voxel `(0, 1, 0)` at output `(1, 0)` and returns 10, not 11.

Current behaviour: `reslice` does not exist.

Source identity: `Volume` in `crates/ocelli-volume/src/lib.rs`.

## Scope, API and compatibility

In scope:

- `reslice` and `OrthoPlane` in `ocelli-volume`.
- `RenderGraph::show_ortho` in `ocelli-render`, using the stack upload that already exists.
- GPU readback of one axial plane and one sagittal plane through the existing VOI shader, compared with `LutChain::map_into`.

Out of scope:

- A new `Pass` variant.
- A WGSL trilinear sampler. One kernel, in Rust.
- Changing `ImagePlane::index_to_world`.
- Interactive pan and zoom policy. The uploaded image uses the existing `View`. This story does not add gestures.
- Tier C presentation. F-X004.
- Partial-plane display. Refusal, not a hole filled with zero.

```rust
pub enum OrthoPlane {
    /// Constant slice index. Steps along both in-plane cosines.
    Axial { slice: f64 },
    /// Constant column index. Steps along the column cosine and the normal.
    Sagittal { column: f64 },
    /// Constant row index. Steps along the row cosine and the normal.
    Coronal { row: f64 },
}

pub enum PlaneSample {
    Present(f32),
    Absent,
}

pub struct OrthoImage {
    pub width: u32,
    pub height: u32,
    pub column_mm: f64,
    pub row_mm: f64,
    pub samples: Vec<PlaneSample>,
}

impl Volume {
    pub fn reslice(&self, plane: OrthoPlane) -> Result<OrthoImage, VolumeError>;
    pub fn voxel_world(&self, index: [f64; 3]) -> Pt<World>;
}
```

Output size and pixel spacing:

| Plane | Width | Height | `column_mm` | `row_mm` | Voxel at pixel `(a, b)` |
|-------|-------|--------|-------------|----------|--------------------------|
| Axial | `dims[0]` | `dims[1]` | `spacing[0]` | `spacing[1]` | `(a, b, k)` |
| Sagittal | `dims[1]` | `dims[2]` | `spacing[1]` | `spacing[2]` | `(i, a, b)` |
| Coronal | `dims[0]` | `dims[2]` | `spacing[0]` | `spacing[2]` | `(a, j, b)` |

The index argument must lie in `[0, dim - 1]` on its fixed axis. Outside that closed interval, including a NaN, returns `VolumeError::PlaneOutOfRange` and no image. An in-range fractional index is valid.

`show_ortho` writes `Present` values in x-fastest order and returns `PlaneIncomplete` without a write when any sample is `Absent`. The upload counter does not move on that refusal.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `Volume::reslice` | Level-0 store, `SlicePresence`, `voxel_world` formula | Any tier. Stored values. No LUT | `OrthoImage` or `PlaneOutOfRange` |
| `Volume::voxel_world` | `origin`, `spacing`, `direction` | Fixtures and later tools | `Pt<World>` |
| `RenderGraph::show_ortho` | `upload_stored` of the present `f32` values | Tier A or B, stack pass, complete plane only | Texture write, or `PlaneIncomplete` / `TierCUnavailable` with no write |
| Stack VOI shader | existing `VoiParams` | Unchanged decisions | Windowed pixels of the uploaded stored values |
| `ImagePlane::index_to_world` | unchanged | Not used for the third axis | No second in-plane equation |
| F-X004 CPU viewport | `reslice` later | Not called by this story's display path | No tier C image from `show_ortho` |
| F-053 oblique | not this enum | Later | No arbitrary normal here |

Units: indexes are voxels. World is LPS millimetres. Output spacing is millimetres. `Present(f32)` is the stored container number, not Hounsfield units and not a display code.

Rounding: trilinear uses `f64` weights. The result converts with `as f32` only after the weighted sum. Integer voxel values through 24 bits stay exact, so every axial pixel of a 16-bit volume equals the container integer.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Complete volume | `reslice` of an in-range axial, sagittal or coronal plane | One image. Every sample `Present`. World points match the formula | Volume bytes unchanged |
| Volume with an absent slice | `reslice` of a plane whose support touches that slice with non-zero weight | Those pixels are `Absent`. Other pixels still `Present` | No zero substituted |
| Any volume | Index outside `[0, dim - 1]` | `PlaneOutOfRange` | No image allocated |
| Stack viewport, tier A or B | `show_ortho` of an all-present image | `upload_stored` runs. Dirt data is set by that function | Previous texture replaced only by the new plane |
| Stack viewport | `show_ortho` of an image that contains `Absent` | `PlaneIncomplete` | Upload counter and texture bytes unchanged |
| Tier C | `show_ortho` | `TierCUnavailable` | No CPU framebuffer filled |

`reslice` does not mutate the volume. Two calls with the same plane return equal images and allocate twice. That is not a cached no-op.

An incomplete plane must not replace a previously uploaded complete plane. `show_ortho` checks presence before any texture write.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| `OrthoImage` buffer | `reslice` | The returned value | `Ok(image)` | `PlaneOutOfRange` returns no buffer |
| Stack stored texture | existing `upload_stored` | The viewport id | Successful `show_ortho` | `PlaneIncomplete` does not call `upload_stored` |
| Volume store | Not acquired for write | Level 0 prefix | Unchanged | Reslice only borrows it |

No browser setter. No device creation. `ocelli-render` remains the only crate that creates a device.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The fixture in the problem section. Axial `(1, 1)` is 11 at world `(10.5, 22.0, 30.0)`. Sagittal column 1 pixel `(1, 0)` is 11, not 10. Sagittal column 0.5 pixel `(0, 0)` is 0.5. Coronal row 0 pixel `(1, 1)` is 101 at world `(10.5, 20.0, 33.0)`. PS3.3 C.7.6.2.1.1 plus the volume's third axis | `ortho_planes_follow_the_volume_axes` | Row-cosine sagittal. Swapped spacing. Instance-number order. Half-voxel origin | Implementation. `ocelli-volume` |
| AC2 | Same grid, slice 1 absent. Axial slice 0 is entirely `Present` with values 0, 1, 10, 11. Axial slice 1 is entirely `Absent`. Sagittal column 1 pixel `(0, 1)` is `Absent`, not 0 and not 101. Sagittal column 0.5 pixel `(0, 0.5)` is `Absent` because the z=1 corners have weight | `an_absent_slice_is_not_stored_zero` | Blending against zero. Painting the cleared byte | Implementation. `ocelli-volume` |
| AC3 | One `I16` voxel, bytes `FD FF`, axial slice 0. The sample is `-3.0` | `sagittal_and_axial_keep_the_sign` | `u16` decode | Implementation. `ocelli-volume` |
| AC4 | `reslice` of sagittal column `-0.1` and of column `2.0` on a width-2 volume | `PlaneOutOfRange`, no image | Clamping to the edge and inventing a plane | Implementation. `ocelli-volume` |
| AC5 | GPU, tier A or B. AC1 axial plane uploaded with `show_ortho`. Readback through the stack VOI matches `LutChain::map_into` on values 0, 1, 10, 11 within 2 `f32` ULP at 255, the bound F-041 already uses for the same shader | `axial_upload_matches_the_lut_chain` ignored, `gpu` gate | A second window formula. Uploading world millimetres as if they were stored samples | Integrated. GPU readback |
| AC6 | GPU. AC2 image passed to `show_ortho`. The call returns `PlaneIncomplete`. `uploads` is unchanged and a following readback of a previously uploaded ramp still matches that ramp | `an_absent_plane_does_not_replace_the_texture` | Writing zeros over the last good plane | Integrated. GPU |
| AC7 | Tier C `show_ortho` returns `TierCUnavailable` and does not allocate a display buffer | `ortho_viewport_is_unavailable_on_tier_c` | A CPU image from this entry point | Implementation |

AC5 and AC6 need an adapter. A missing adapter does not mark them passed. The VOI parameters in AC5 are the section 18.3 soft-tissue pair only as a window on these stored values: centre 40, width 400, `LINEAR`, output 0 to 255. The expected codes are `LutChain::map_into` of the four stored numbers, not a hand-copied screenshot. The geometry expectations in AC1 are hand-computed and do not go through the LUT.

Semantic RED: sagittal pixel `(1, 0)` implemented as voxel `(a, i, b)` yields 10. AC1 requires 11. That mutation is the axis swap.

## Approach

New file `crates/ocelli-volume/src/ortho.rs`. `reslice` decodes level-0 voxels through one private function used by both the integer fast path and the trilinear path. Weights are `1 - frac` and `frac` on each axis. A corner with weight `0` is not read. Presence is tested on the slice index of each corner that is read.

`voxel_world` is the formula in the normative section, in `f64`, returned as `Pt<World>`.

`show_ortho` lives in `ocelli-render` and takes `&OrthoImage` plus the viewport id from `register_stored` of a stack pass. It depends on `ocelli-volume`, which F-057 already added. It does not parse DICOM and does not build a `Volume`.

Rejected alternatives:

- Sample bricks in a fragment shader. A second kernel the CPU fixture can drift from.
- Add `Pass::Ortho` to section 22's enum. The output is a 2D stored image and the stack pass already windows those.
- Display `Absent` as alpha 0 inside the VOI shader. That edits the LUT shader to carry a policy decision. Refusal keeps the last complete plane instead, which is the recovery rule for a settled viewport.
- Anatomical axial detection from the normal's largest component. The sprint binds the names to the constructed axes.

## Dependencies, write ownership and resources

Depends on F-051, `done`. In this sprint it follows F-058 and F-057 so the volume store prefix and the render dependency exist. It does not need F-059.

Writes:

- `crates/ocelli-volume/src/lib.rs` (`mod ortho`, `PlaneOutOfRange`)
- `crates/ocelli-volume/src/ortho.rs`
- `crates/ocelli-volume/tests/ortho.rs`
- `crates/ocelli-render/src/lib.rs`
- `crates/ocelli-render/src/ortho.rs`
- `crates/ocelli-render/tests/ortho.rs`
- `docs/lld/volumes.md` and `docs/lld/viewport.md` at completion

`graph.rs` is not edited unless `show_ortho` cannot call `upload_stored` from another module. Prefer calling the public method. GPU tests are exclusive with F-057 and F-059. Serial wave, canonical worktree.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-volume
bin/ocelli.sh test ocelli-volume --test ortho
bin/ocelli.sh check ocelli-render
bin/ocelli.sh test ocelli-render --test ortho
bin/ocelli.sh clippy ocelli-volume
bin/ocelli.sh clippy ocelli-render
```

Ignored GPU tests run under the `gpu` gate at completion. The sprint profile runs them again after integration. AC1's world points are the geometry baseline. Changing the formula invalidates the receipt.

No wasm-size change is expected beyond the new functions if they are linked into the wasm crate. If `ocelli-wasm` does not name `reslice`, the size budget is untouched. This story does not add a wasm export. A size move at completion is investigated with the wasm-size skill rather than accepted in this plan.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. `reslice` allocates the plane image when the plane changes, which is a command, not a fragment
- unsafe: none
- Tier A (WebGPU): full. Host reslice, existing stack upload and VOI shader
- Tier B (WebGL2): full. The same samples. Stored-texture format follows F-042 when `R32Float` cannot be sampled
- Tier C (CPU): unavailable for `show_ortho`. `reslice` remains callable host arithmetic for F-X004 and is not presented as the viewport

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 world points and voxel values, PS3.3 C.7.6.2.1.1 | `tests/ortho.rs` |
| fixture | AC2 absence and AC3 sign | `tests/ortho.rs` |
| unit | AC4 range refusal and AC7 tier C | volume and render tests |
| gpu | AC5 LUT agreement and AC6 refusal to replace a plane | ignored render tests |

## Parity surface covered

Appendix B viewport types, the `ORTHOGRAPHIC` behaviour: three orthogonal planes from the constructed volume. This story does not establish `PERSPECTIVE`, `VOLUME_3D`, oblique reslice, thick-slab blend modes, or a cornerstone pixel diff. The oracle still has no full Ocelli viewer. Decision D7 holds. The GPU observation is a readback against `LutChain`, not against cornerstone3D.

## Deviations

None.

## LLD impact

`docs/lld/volumes.md` gains `reslice`, the three planes, the trilinear and absence rules, and `voxel_world`. `docs/lld/viewport.md` gains `show_ortho`, the incomplete-plane refusal, and the tier C refusal.

## Open questions

None. The axis names are the sprint text. The kernel is the one that keeps an on-grid axial pixel equal to the stored voxel and refuses to blend in an absent neighbour. F-X004 remains the tier C viewport.
