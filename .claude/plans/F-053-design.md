# F-053, Oblique / arbitrary-plane reslicing

**Status**: approved
**Epic ref**: E8.3
**Sprint**: S20
**Estimate**: 4w

## Normative source, transcribed

Quoted text below keeps the source words. A prose semicolon in the source is a full stop here. An em dash in the source is a comma here. `docs/hld/` itself is not edited.

### `docs/hld/03-architecture-and-crates.md`, section 4

| Crate | Responsibility | wasm | native |
|-------|----------------|------|--------|
| ocelli-volume | Volume assembly, geometry, reslicing | yes | yes |

### `docs/hld/16-volume-representation.md`, section 19

```rust
pub struct Volume {
    pub dims: [u32; 3],
    pub spacing: [f64; 3], // mm
    pub origin: Pt<World>, // IPP of the first slice
    pub direction: glam::DMat3, // derived from ImageOrientationPatient
    pub voxel: VoxelKind, // U8 | I16 | U16 | F32
    pub data: VoxelStore, // one contiguous allocation, x fastest
    pub levels: Vec<Level>, // multiscale. level 0 is full resolution
    pub present: bitvec::BitVec, // per-slice, for progressive assembly
}
```

Layout bullets from the same section:

- **Layout.** One contiguous allocation, x fastest then y then z. Slice n starts at n * dims[0] * dims[1] * bytes_per_voxel. Do not introduce a per-slice Vec. The whole point is a single upload region.
- **Progressive assembly.** Clear the 3D texture at creation and upload slices as they land. present drives the loading indicator and tells the oracle which frames are comparable yet.
- **Bricks and a level axis, from the start.** The volume carries a multiscale level axis and 128³ brick decomposition even when a series fits comfortably. Phase 1 uploads every brick. §30 makes residency selective. Retrofitting a level axis into a volume model that never had one is the rewrite cornerstone3D cannot afford.

### `docs/hld/05-rendering.md`, section 7

- **Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs. The tier resolves once at startup.
- **Volume rendering runs on both tiers**, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute, GPU segmentation, histogram passes, compute-based resampling, is tier A only and must degrade, not fail.
- **Blend modes are shader variants**, composite, MIP, MinIP, average, selected by specialisation constant rather than branching per fragment.

### `docs/hld/B-parity-surface.md`, Appendix B

| Surface | Count | Notes |
|---------|-------|-------|
| Viewport types | 12 | STACK, ORTHOGRAPHIC, PERSPECTIVE, VOLUME_3D, PLANAR, VIDEO, WHOLE_SLIDE, ECG and _NEXT variants |
| Blend modes | 5 | Composite, MIP, MinIP, average, labelmap edge projection |

### `docs/lld/volumes.md`, voxel-to-world and spacing

Spacing comes from `ImagePositionPatient` projected on the slice normal, DICOM PS3.3 C.7.6.2.1.1. `SpacingBetweenSlices`, `SliceThickness`, `SliceLocation`, `InstanceNumber` and the gantry tilt tag are not read anywhere in the crate.

The origin is the `ImagePositionPatient` of the lowest projected slice, copied exactly. That point is the centre of voxel zero. There is no half-voxel offset.

Inter-slice spacing is the median of the projected gaps.

`Volume::voxel_world` is the patient point of a continuous index `(i, j, k)`. The origin is the centre of voxel `(0, 0, 0)`. The point is the origin plus `i * spacing[0] * direction.col(0)` plus `j * spacing[1] * direction.col(1)` plus `k * spacing[2] * direction.col(2)`. `spacing[0]` is column millimetres along the row cosine. That is DICOM PS3.3 C.7.6.2.1.1 with the volume's third axis.

On a voxel centre the sample is that voxel's container value. Off centre the weights are trilinear in `f64` and then narrowed to `f32`. A corner with non-zero weight on an absent slice is `PlaneSample::Absent`. A zero weight does not read that corner. An appended coarser level is not a sample source.

### `.claude/skills/dicom-expert/SKILL.md`, section 6

`(0020,0032)` `ImagePositionPatient` is LPS mm of the centre of the first voxel. With `X` the row direction, `Y` the column direction, `i` the column index and `j` the row index:

```text
P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
```

`PixelSpacing[0]` multiplies the column direction cosine. `PixelSpacing[1]` multiplies the row direction cosine. IPP is the centre of the first voxel, not its corner.

### `crates/ocelli-volume/src/ortho.rs`

`OrthoPlane` is `Axial { slice }`, `Sagittal { column }` or `Coronal { row }`. `Volume::voxel_world` takes a continuous index `(i, j, k)` and documents that the origin is the centre of voxel `(0, 0, 0)`. `Volume::reslice` reslices one orthographic plane from level 0. `sample_at` is the private level-0 sampler that `reslice` calls. Off centre it is trilinear. On an integer index the neighbour weights are zero, so that call reads one voxel.

### `tools/oracle/volume-params.json`

The three orientations are the planes cornerstone3D 5.8.2 fixes in patient LPS axes: `AXIAL`, `SAGITTAL`, `CORONAL`. `blendMode` is `COMPOSITE`. `slabThicknessMm` is null. This story does not read or change that file.

### `docs/hld/DEVIATIONS.md`, D-07

A feature that cannot run on the resolved tier reports unavailable, and never silently produces a different answer. That row already exists. This plan does not add one.

## What the specification does not cover

The HLD names reslicing as `ocelli-volume` work and does not give an arbitrary-plane type. These choices are local. They are not claims that the HLD already made them.

1. A plane is `ObliquePlane { origin: Pt<World>, row: DVec3, column: DVec3, columns: u32, rows: u32, column_mm: f64, row_mm: f64 }`. `origin` is the centre of output pixel `(0, 0)`. `row` and `column` are direction cosines. Output pixel `(c, r)` is `origin + c * column_mm * row + r * row_mm * column`. That is the PS3.3 column and row naming already used by `voxel_world`. `column_mm` multiplies `row`, as `PixelSpacing[1]` multiplies `X`. `row_mm` multiplies `column`, as `PixelSpacing[0]` multiplies `Y`. The returned `OrthoImage` copies `columns` to `width`, `rows` to `height`, and the two millimetre fields. Samples are x-fastest, column then row. The vectors are not renormalised. A non-unit cosine is multiplied as given.
2. Sampling calls the existing level-0 `sample_at`. It does not copy the trilinear weights. A coarser level stays out of reach because that function does not read one. The patient point is converted to the continuous index that `voxel_world` maps back to that point. With `delta` equal to the patient point minus the volume origin, `i` is `dot(direction.col(0), delta) / spacing[0]`, and `j` and `k` use columns 1 and 2 the same way. That inverse matches `voxel_world` for the orthonormal direction this crate stores. An absent contributing slice returns `PlaneSample::Absent`, never a stored zero. A sample that falls outside the grid is also `Absent`, because `sample_at` already returns that, and the plane call still returns `Ok`. A non-finite or zero direction, a zero dimension, or a non-positive spacing returns `VolumeError::ObliquePlane` and allocates nothing. The same error covers a non-finite plane field, a spacing that is not strictly positive, a volume spacing that is not strictly positive or not finite, and a `columns` times `rows` product that does not fit in `usize`. `PlaneOutOfRange` stays the `OrthoPlane` fixed-index error. It is not reused here.
3. Axial, sagittal and coronal stay `OrthoPlane`. This story does not replace them and does not change `reslice`. An oblique plane whose normal is a volume axis is still `ObliquePlane`. It is not required to collapse to `OrthoPlane`. A half-voxel plane whose normal is an axis is still sampled with the formula in item 1. Snapping that plane onto an integer `OrthoPlane` index is the wrong image.
4. No perspective camera. No slab thickness. No blend mode. Those are F-054. No new viewport type beyond feeding the existing ortho display an `OrthoImage`. No new `Pass` variant. No WGSL resampling kernel.
5. Display reuses `show_ortho`. A host `reslice_oblique` is followed by that existing call. This story adds no GPU upload and no shader. Tier C computes the sample grid in `ocelli-volume` with no GPU. The viewport display reports unavailable on tier C, the same honesty rule as F-052. This story does not draw a different image.

`SpacingBetweenSlices` is still not read. Output `column_mm` and `row_mm` are the caller's millimetres. They are not a new measurement of the volume.

## Problem and reproducer

`Volume::reslice` samples only `OrthoPlane`. An arbitrary patient plane cannot be requested. Rotating a stored axial index grid, or snapping an oblique cut onto the nearest axial, sagittal or coronal index, produces a plausible image of the wrong plane. Square spacing hides a swapped step. Equal voxel values hide a missed neighbour.

`docs/sprints/CURRENT_SPRINT.md` states the bound. F-053 samples an arbitrary plane through the constructed volume. The plane is that volume's origin, spacing and direction. It is not a rotation of the stored slice index and not a second patient frame.

Current behaviour: `ObliquePlane` and `reslice_oblique` do not exist.

Source identity: `Volume::reslice`, `Volume::voxel_world` and `sample_at` in `crates/ocelli-volume/src/ortho.rs`.

### Hand-computed fixture, PS3.3 C.7.6.2.1.1

Identity volume. Spacing `(1, 1, 1)` mm. Origin `(0, 0, 0)`, the centre of voxel `(0, 0, 0)`. Direction identity, so column 0 is `(1, 0, 0)`, column 1 is `(0, 1, 0)` and column 2 is `(0, 0, 1)`. `U8`. No patient data. Bytes are x-fastest, then y, then z:

| Index `(x, y, z)` | Store index | Value |
|-------------------|-------------|-------|
| `(0, 0, 0)` | 0 | 10 |
| `(1, 0, 0)` | 1 | 30 |
| `(0, 1, 0)` | 2 | 70 |
| `(1, 1, 0)` | 3 | 90 |
| `(0, 0, 1)` | 4 | 110 |
| `(1, 0, 1)` | 5 | 130 |
| `(0, 1, 1)` | 6 | 150 |
| `(1, 1, 1)` | 7 | 170 |

Slice 0 is `[10, 30, 70, 90]`. Slice 1 is `[110, 130, 150, 170]`.

Oblique plane for the mix. `origin` `(0.5, 0.5, 0.5)`. `row` `(1, 0, 0)`. `column` `(0, 0, 1)`. `column_mm` 1. `row_mm` 1. `columns` 2. `rows` 1. The normal is `row` crossed with `column`, which is `(0, -1, 0)`, a volume axis. The input is still `ObliquePlane`.

Patient point, from the section 6 formula with this plane's names:

```text
P(c, r) = origin + c * column_mm * row + r * row_mm * column
```

`P(0, 0)` is `(0.5, 0.5, 0.5)`. `P(1, 0)` is `(1.5, 0.5, 0.5)`.

On this identity volume the continuous index equals the patient point. `voxel_world([0.5, 0.5, 0.5])` is `(0.5, 0.5, 0.5)`. `voxel_world([1.5, 0.5, 0.5])` is `(1.5, 0.5, 0.5)`.

Index `(0.5, 0.5, 0.5)` has fraction `0.5` on each axis. All eight corners have weight `1/8`. The corner sum is `10 + 30 + 70 + 90 + 110 + 130 + 150 + 170 = 760`. `760 / 8 = 95`. The sample is `PlaneSample::Present(95.0)`. A snap onto the y = 0 face is `(10 + 30 + 110 + 130) / 4 = 70`. A snap onto the y = 1 face is `(70 + 90 + 150 + 170) / 4 = 120`. Neither is the result.

Changing store index 1 from 30 to 38 adds `8 / 8 = 1`. Pixel `(0, 0)` becomes `Present(96.0)`. Pixel `(1, 0)` stays `Absent`.

Index `(1.5, 0.5, 0.5)` has a corner at x = 2, outside a width of 2. `sample_at` returns `Absent` for the whole sample. It does not clamp to x = 1 and it does not drop that corner and renormalise.

Slice 1 absent, same half-voxel pixel. The four z = 1 corners have weight `1/8`. The sample is `Absent`. Treating those corners as stored zero yields `200 / 8 = 25`. Renormalising over the four present corners yields `200 / 4 = 50`. Leaving the absence unread yields 95. None of those is the result.

Integer control. `origin` `(0, 0, 0)`, same directions, one column and one row. The index is `(0, 0, 0)`. The only non-zero weight is voxel 10. The sample is `Present(10.0)`. Changing store index 1 from 30 to 38 leaves `Present(10.0)`. With slice 1 absent the sample stays `Present(10.0)`, because that slice's weight is zero and `sample_at` does not read it.

Direction control, still spacing `(1, 1, 1)` and origin `(0, 0, 0)`, but the direction columns are `(0, 1, 0)`, `(0, 0, 1)` and `(1, 0, 0)`. Patient point `(0.5, 0, 0)` solves to index `(0, 0, 0.5)`, because `i` is the dot with column 0, which reads patient y, `j` reads patient z, and `k` reads patient x. The two corners are 10 and 110. The sample is `Present(60.0)`. Ignoring the direction and sampling `(0.5, 0, 0)` yields 20. That is the wrong implementation.

The literals `95`, `96`, `10` and `60` are written in the test. They are not computed by calling `sample_at` from the test.

## Scope, API and compatibility

In scope:

- `ObliquePlane` and `Volume::reslice_oblique` in `ocelli-volume`.
- `VolumeError::ObliquePlane` for a plane that fails validation before any buffer exists.
- A host `OrthoImage` whose samples are level-0 `sample_at` results.
- The existing `show_ortho` and `present_plane` contract, used as the display and not rewritten.
- The tier C refusal on that existing display path.

Out of scope:

- Replacing or redefining `OrthoPlane` and `reslice`.
- A perspective camera, a slab thickness, a blend mode, MIP, MinIP or average. F-054.
- A new viewport type or a new `Pass`.
- A GPU trilinear kernel, a compute resampling pass, or a second copy of the weights.
- CPU drawing of a different image. F-X004 owns the CPU viewport.
- Renormalising direction cosines.
- Reading `SpacingBetweenSlices` or recomputing volume spacing.
- A wasm export, a pixel copy across the worker boundary, and `unsafe`.
- An oracle parameter change. Decision D7 still holds.

```rust
pub struct ObliquePlane {
    pub origin: Pt<World>,
    pub row: DVec3,
    pub column: DVec3,
    pub columns: u32,
    pub rows: u32,
    pub column_mm: f64,
    pub row_mm: f64,
}

impl Volume {
    pub fn reslice_oblique(&self, plane: ObliquePlane) -> Result<OrthoImage, VolumeError>
}
```

`lib.rs` exports `ObliquePlane` next to `OrthoImage`, `OrthoPlane` and `PlaneSample`.

Validation runs before the sample buffer is created. On failure the `Result` is `Err(VolumeError::ObliquePlane)` and that error carries no `OrthoImage`. On success every output pixel is one `sample_at` call. `Present` is the narrowed container blend. `Absent` is an absent contributing slice or a corner outside the grid. The volume bytes, presence bits and level axis are not written.

`show_ortho` still uploads only a fully present `OrthoImage` on tier A or tier B, and still returns `OrthoError::TierCUnavailable` on tier C before it reads samples. This story does not change that order.

Units are millimetres in LPS for world points, voxels for the continuous index, and container numbers for `Present(f32)`. The value is not a Hounsfield unit and not a display code. No modality LUT and no VOI LUT run inside `reslice_oblique`.

Compatibility: existing `reslice` callers keep their signatures and their images. A new error variant does not change the meaning of `PlaneOutOfRange`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `Volume::reslice_oblique` | level-0 `sample_at`, inverse of `voxel_world` | Any tier. Host only. Stored container values. No LUT | `OrthoImage`, or `ObliquePlane` with no buffer |
| `Volume::reslice` | unchanged `OrthoPlane` path | Axial, sagittal, coronal only | Same images as F-052 |
| `Volume::voxel_world` | `origin`, `spacing`, `direction` | Fixture check and the inverse | `Pt<World>` |
| `RenderGraph::show_ortho` | existing `present_plane` then `upload_stored` | Tier A or B, complete plane only. Not modified by the recommended path | Texture write, or `PlaneIncomplete` or `TierCUnavailable` with no write |
| `present_plane` | existing tier check, then `Absent` check | Tier C refuses before samples are read | `TierCUnavailable`, upload closure not called |
| F-054 slab and blend | not this struct | Later story | No thickness and no blend mode here |
| Oracle volume page | unchanged `volume-params.json` | Axial, sagittal, coronal, `COMPOSITE`, slab null | No new cornerstone observation |

Rounding stays inside `sample_at`. Weights are `f64`. The stored blend is narrowed to `f32` by the existing `narrow_f32`. This story does not add a cast. Integer container values that the existing narrowing already preserves stay exact, including 10, 60, 95 and 96.

## State transitions and invariants

Sampling is stateless. The volume is borrowed and not mutated. There is no cache of the oblique image, so this section has no eviction or recovery transition of its own. The display refusal reuses the F-052 rule and does not add a second one.

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Filled identity volume | Half-voxel oblique plane in the fixture | `Ok` image. Pixel `(0, 0)` is `Present(95)`. Pixel `(1, 0)` is `Absent` | Volume bytes unchanged |
| Same volume, slice 1 clear | That same half-voxel pixel | That pixel is `Absent` | No zero written into the sample |
| Same volume | Integer origin `(0, 0, 0)` | `Present(10)` even if slice 1 is clear | Zero-weight slice is not read |
| Same volume, permuted direction | Patient point `(0.5, 0, 0)` | `Present(60)` | Not the identity-direction blend of 20 |
| Any volume | Zero direction, non-finite field, zero dimension, or spacing that is not strictly positive | `Err(ObliquePlane)` | No buffer allocated. Volume unchanged |
| Existing ortho caller | `reslice` of axial slice 0 on the fixture | Samples 10, 30, 70, 90 | `OrthoPlane` path unchanged |
| Tier C display | `present_plane` of the oblique `OrthoImage` | `TierCUnavailable` | Upload closure not called. No CPU image |
| Tier A or B display | `present_plane` of an image that contains `Absent` | `PlaneIncomplete` | Previous texture not replaced |
| Tier A or B display | `present_plane` of a fully present oblique image | Upload closure receives the `Present` values | Same path as F-052 |

Two calls with the same plane return equal images and allocate two buffers. That is not a cached no-op. An invalid plane does not clear or replace a previous `OrthoImage` the caller already holds. `reslice_oblique` never sees that previous image.

Invariants that span consumers: absence is not a stored zero. Level 0 is the only sample source. Axial, sagittal and coronal remain `OrthoPlane`. An axis-aligned normal does not change the public input type. Pixels do not cross the worker boundary.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Oblique sample buffer | `reslice_oblique`, after validation | The returned `OrthoImage` | `Ok(image)` with length `columns` times `rows` | `Err(ObliquePlane)` returns no buffer. A second `Ok` is a second buffer with equal samples |
| Volume store and presence | Not acquired for write | Level 0 prefix | Unchanged | Reslice only borrows it |
| Stack stored texture | Existing `show_ortho`, only if the caller uploads | Viewport id already owned by F-052 | Successful `show_ortho` of a fully present image | `PlaneIncomplete` and `TierCUnavailable` do not call `upload_stored` |

Validation is the only work before the commit point, and it has no side effect. There is no browser setter and no device creation. `ocelli-volume` still creates no device. `ocelli-render` remains the only crate that creates one. A refused plane cannot publish a partial grid, because the buffer is not allocated until every checked field is valid, and `sample_at` does not fail the call after that.

No oblique cache is introduced. Validity of a returned image is the plane fields plus the volume origin, spacing, direction, dimensions, voxel kind, level-0 bytes and presence bits. This story does not retain that image inside the volume.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Identity fixture. Pixel `(0, 0)` is `Present(95)`. `voxel_world([0.5, 0.5, 0.5])` is `(0.5, 0.5, 0.5)`. Pixel `(1, 0)` is `Absent`. Store byte 30 changed to 38 makes pixel `(0, 0)` `Present(96)` and leaves pixel `(1, 0)` `Absent`. Cites PS3.3 C.7.6.2.1.1 | `half_voxel_oblique_is_the_trilinear_mix` | Snap to y = 0 (70) or y = 1 (120). Nearest voxel. Clamp x = 1.5 to the edge. Blend that misses the mutated neighbour | Implementation. `ocelli-volume` |
| AC2 | Same half-voxel pixel with slice 1 absent | `Absent`, not 0, 25, 50 or 95. `an_absent_slice_is_not_stored_zero` | Stored zero. Partial blend. Renormalised present corners. Ignoring absence | Implementation. `ocelli-volume` |
| AC3 | Integer origin `(0, 0, 0)` is `Present(10)`. Mutating byte 30 leaves 10. Slice 1 absent still leaves `Present(10)` | `an_integer_index_reads_one_voxel` | Blending a zero-weight neighbour. Marking the pixel absent because an adjacent slice is missing | Implementation. `ocelli-volume` |
| AC4 | Permuted direction. Patient `(0.5, 0, 0)` is index `(0, 0, 0.5)` and `Present(60)`, not 20. `voxel_world([0, 0, 0.5])` is `(0.5, 0, 0)` | `direction_moves_the_patient_point` | Using the patient point as a voxel index. A second patient frame | Implementation. `ocelli-volume` |
| AC5 | `reslice(Axial { slice: 0 })` is 10, 30, 70, 90. `reslice_oblique` of origin `(0, 0, 0)`, row `(1, 0, 0)`, column `(0, 1, 0)`, 2 by 2, spacing 1 and 1, returns those same samples. `OrthoPlane` still exists | `named_planes_stay_ortho` | Deleting `reslice`. Returning a different axial row order | Implementation. `ocelli-volume` |
| AC6 | Identity volume. Origin `(0, 0, 0)`, row `(2, 0, 0)`, column `(0, 0, 1)`, `column_mm` 0.5, `row_mm` 1, two columns, one row. Pixel `(0, 0)` is 10. Pixel `(1, 0)` is 30, not 20 | `a_non_unit_row_is_not_renormalised` | Normalising the row before the multiply | Implementation. `ocelli-volume` |
| AC7 | Zero row, a non-finite column component, `columns` 0, `rows` 0, `column_mm` 0, and `row_mm` negative. Each one is `Err(VolumeError::ObliquePlane)` | `a_bad_plane_allocates_nothing` | `Ok` with an empty or clamped image. `PlaneOutOfRange`. A buffer the caller can read after the error | Implementation. `ocelli-volume` |
| AC8 | `present_plane(Tier::Cpu, image)` on the AC1 image and on a one-pixel `Present(95)` image returns `TierCUnavailable` and does not call the upload closure. `present_plane(Tier::A, complete)` receives `[95.0]`. `present_plane(Tier::A, image_with_absent)` returns `PlaneIncomplete` and does not upload. No adapter is opened | `oblique_display_reuses_present_plane` | A CPU image. A second upload path. Drawing the present pixels and skipping `Absent` on tier C | Implementation. `ocelli-render`, no device |

AC1 is the hand-computed fixture. The mutation of one contributing neighbour is what makes a frozen expected value fail when the blend is wrong. AC8 does not claim a GPU readback. Display is the existing `show_ortho` call. A missing adapter run is not recorded as a pass.

Semantic RED for AC1: sampling only the y = 0 face yields 70 where the fixture requires 95. Semantic RED for AC4: ignoring `direction` yields 20 where the fixture requires 60. Semantic RED for AC2: blending against zero yields 25 where the fixture requires `Absent`.

## Approach

Add `ObliquePlane` and `Volume::reslice_oblique` in `crates/ocelli-volume/src/ortho.rs`, beside `sample_at`. Export the struct from `lib.rs`. Add the unit variant `VolumeError::ObliquePlane`.

The function checks the plane and the volume spacing first. It then allocates one `Vec` whose length is `columns` times `rows`. For each column `c` and row `r` it builds the patient point with the formula in the fixture, solves the continuous index with the three column dots, and pushes `self.sample_at(i, j, k)`. No other sampler exists.

`show_ortho` is not edited under the recommended answer. AC8 calls the existing `present_plane`.

Rejected alternatives:

- Replace `OrthoPlane` with one general plane. The three named planes already have callers and tests.
- Require an axis-aligned oblique to collapse onto `OrthoPlane`. The user choice says that collapse is not required, and a half-voxel axis normal must still be the trilinear mix.
- Resample in a compute shader or a fragment shader. Section 7 makes compute resampling tier A only. A second kernel can drift from `sample_at`. This story stays on the host sampler, so tier B is the same grid rather than a degraded one.
- Fill an absent corner with zero. That is the defect `PlaneSample::Absent` already exists to refuse.
- Add slab thickness or a blend-mode field to `ObliquePlane`. That is F-054.
- Add a perspective viewport type. Appendix B names `PERSPECTIVE` separately, and this story does not.

No new trait. No new generic. No `Box<dyn>`. The concrete type is `ObliquePlane` and the concrete sampler is `sample_at`.

## Dependencies, write ownership and resources

Backlog dependency: F-052, status `done`. Allocation notes for F-053 are empty. There is no architecture-hook flag and no source-policy warning in that row.

Source dependencies, already present: `sample_at`, `voxel_world`, `OrthoImage`, `PlaneSample`, `show_ortho` and `present_plane`.

Safe order: volume type and fixture tests first. The render test only calls `present_plane` after `OrthoImage` can be built. No shader, no GPU device and no corpus row.

Writes:

- `crates/ocelli-volume/src/ortho.rs`
- `crates/ocelli-volume/src/lib.rs`, for the export and `VolumeError::ObliquePlane`
- `crates/ocelli-volume/tests/oblique.rs`
- `crates/ocelli-render/tests/oblique.rs`, for AC8 only
- `docs/lld/volumes.md`, at completion

`ocelli-render/src` is not a writer under the recommended answer. `graph.rs`, the VOI shader and `ocelli-wasm` are not writers. `ortho.rs` in the volume crate is exclusive to this story while it is implemented. F-054, F-055 and F-056 are the same sprint and must not edit that file concurrently. No GPU or browser exclusivity is required for AC1 to AC8.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-volume
bin/ocelli.sh test ocelli-volume --test oblique
bin/ocelli.sh clippy ocelli-volume
bin/ocelli.sh check ocelli-render
bin/ocelli.sh test ocelli-render --test oblique
bin/ocelli.sh clippy ocelli-render
```

The volume test is the geometry evidence. The render test is the tier C and `present_plane` evidence. Neither opens an adapter. A GPU readback is not a row in this inventory.

Changing the patient-point formula, the index dots, or the inherited trilinear rule invalidates AC1, AC2, AC3 and AC4. The baseline is the literals 95, 96, 10 and 60, plus `Absent`. There is no performance threshold and no size-budget change in this plan. This story adds no wasm export. If a later link pulls the new function into the wasm artifact and the size budget moves, that move is investigated outside this acceptance list rather than treated as a pass.

A missing test, or a test that builds the expected blend by calling `sample_at` itself, does not satisfy AC1.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The plane buffer is sized once from columns times rows, on the reslice call, not inside a frame
- unsafe: none
- Tier A (WebGPU): full for the host sample grid. Display of a complete image is the existing `show_ortho`. No compute shader and no second image
- Tier B (WebGL2): full for the same host samples and the same `show_ortho` path. The grid is not degraded on tier B
- Tier C (CPU): the sample grid is full in `ocelli-volume` and uses no GPU. Viewport display is unavailable, the existing `TierCUnavailable` refusal from F-052 under D-07. This story does not draw a different image

See `docs/hld/DEVIATIONS.md` D-07. The three rows are the whole tier answer.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1, AC2, AC3 and AC4, hand-computed from PS3.3 C.7.6.2.1.1 | `crates/ocelli-volume/tests/oblique.rs` |
| fixture | AC5 named planes and AC6 non-unit row, same section's multiply | `crates/ocelli-volume/tests/oblique.rs` |
| unit | AC7 refusal before a buffer exists | `crates/ocelli-volume/tests/oblique.rs` |
| unit | AC8 tier C refusal and tier A handoff of the host sample, no device | `crates/ocelli-render/tests/oblique.rs` |

The geometry rows are fixtures, not uncategorised unit tests. HLD 27.2 R3. The test comment cites PS3.3 C.7.6.2.1.1 and repeats the literals from this plan.

## Parity surface covered

Appendix B viewport types, the `ORTHOGRAPHIC` behaviour only. This story can establish that an arbitrary patient plane becomes an `OrthoImage` the existing orthographic display already accepts. It does not establish `PERSPECTIVE`, `VOLUME_3D`, `VIDEO`, `WHOLE_SLIDE`, `ECG` or the `_NEXT` variants.

Appendix B blend modes are not established. Composite, MIP, MinIP, average and labelmap edge projection are F-054. The oracle file records axial, sagittal and coronal in patient LPS, blend `COMPOSITE`, slab thickness null. Those three orientations stay `OrthoPlane`. This story does not add an oracle orientation and does not claim a cornerstone pixel diff. Decision D7 holds. There is no Ocelli renderer in the oracle for an arbitrary plane.

## Deviations

none

## LLD impact

`docs/lld/volumes.md` gains `ObliquePlane`, `reslice_oblique`, the patient-point formula, the `Absent` rule, the validation refusal, and the statement that axial, sagittal and coronal stay `OrthoPlane`.

`docs/lld/viewport.md` stays on `show_ortho`. It does not gain a second upload function.

## Open questions

None. The consolidated design round was offered on 2026-10-31 and declined. Display is a host `reslice_oblique` plus the existing `show_ortho` call. No new GPU entry point and no new shader. AC1 to AC7 stay host fixtures. AC8 stops at `present_plane` and does not open an adapter.
