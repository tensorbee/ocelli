# Volumes

**F-IDs that contributed:** F-033, F-036, F-043, F-051, F-052, F-057, F-058
**Last updated:** 2026-10-27

HLD section 19's volume, made into `crates/ocelli-volume`. Spacing comes from
`ImagePositionPatient` projected on the slice normal, DICOM PS3.3
C.7.6.2.1.1. `SpacingBetweenSlices`, `SliceThickness`, `SliceLocation`,
`InstanceNumber` and the gantry tilt tag are not read anywhere in the crate.

## The types

| Type | What it is |
|------|-----------|
| `FrameOfReferenceUid` | The `FrameOfReferenceUID` string, compared and never parsed |
| `VoxelKind` | `U8`, `I16`, `U16` and `F32`. Assembly names only the first three |
| `SliceEvidence` | One `DecodedFrame` offered as a slice |
| `VolumeCandidate` | Slices that share a frame of reference and an orientation |
| `Volume` | Section 19's struct over one `VoxelStore` |
| `VoxelStore` | One contiguous `Vec<u8>`, x fastest, then y, then z |
| `Level` | Level 0 is full resolution at offset 0. Later entries are caller-supplied coarser levels on the same store |
| `SlicePresence` | One bit per slice over `Vec<u64>`. Deviation D-25 |
| `VolumeKey` | The series id a cached volume is stored under |
| `VolumeCache` | Its own `Lru` on `CacheTier::Decoded`, with its own budget |

## How a series becomes a volume

`group_volumes` splits on `FrameOfReferenceUID` and
`ImageOrientationPatient` together, in first-seen order. A localiser mixed
with axials is two candidates. A second frame of reference is a second
candidate. Assembly never re-checks the frame of reference, because slice
evidence does not carry one.

`Volume::assemble` then refuses a candidate that is not one volume.

- Fewer than two slices has no gap to measure.
- An orientation that disagrees with the candidate is `NotOneVolume`.
- Dimensions or voxel kind that disagree are `InconsistentDimensions`.
- In-plane spacing that disagrees, or more than one sample per pixel, is
  `InconsistentSpacing`. A multi-sample frame can never be placed whole into
  a one-sample store, and the refusal does not depend on which slice arrived
  first.
- A stored description other than unsigned 8-bit or signed or unsigned
  16-bit is `UnsupportedVoxelKind`. `VoxelKind::F32` exists on the enum and
  assembly does not produce it, because a 32-bit stored container here is an
  integer, not a float.

Surviving slices sort by `position_vector().dot(slice_normal())`. The normal
is the row cosine crossed with the column cosine. The direction matrix is
those two validated cosines and that normal, so spacing index 0 steps along
the row and index 1 along the column, which is PS3.3 C.7.6.2.1.1.

The origin is the `ImagePositionPatient` of the lowest projected slice,
copied exactly. That point is the centre of voxel zero. There is no half-voxel
offset.

## Spacing, duplicates and shear

Inter-slice spacing is the median of the projected gaps. On an even count the
median averages the two middle gaps. Any gap that departs from the median by
more than 0.5 per cent is `NonUniformSpacing`. At 3 mm that bound is 15
micrometres, below a reconstruction increment and above `f64` noise in a dot
product. The mean is not used, so one outlier cannot hide inside an average.

Two projected positions within 1e-9 mm are `DuplicatePosition`. A real
duplicate is exact, and dot-product noise sits far below a nanometre.

Shear is the part of the inter-slice vector that is not parallel to the
normal. A lateral step above 1e-6 mm is `ShearedVolume`. The tilt tag is not
consulted. A sheared series is a parallelepiped and this story refuses it
rather than storing it as a box.

`order[n]` is the arrival index of the slice that landed at sorted position
`n`. `Volume::construct` copies each slice's stored samples to that position.
A later progressive caller uses the same index and does not project the
positions a second time outside this crate.

## Construction

`Volume::construct` is the caller of `place`. It borrows the candidate for
the checks above, then writes each slice's stored samples at sorted
position `n`. `order[n]` is that slice's arrival index. The returned volume
has every slice present.
`Volume::assemble` still returns the cleared store with `present_count` 0.
`construct` applies no modality or VOI LUT and does not insert into
`VolumeCache`. A failure drops the local volume.

## The store and progressive fill

Slice `n` starts at `n * dims[0] * dims[1] * bytes_per_voxel` inside the one
allocation made at assembly. `place` copies one slice into that range and
sets its presence bit. Writing the same slice again replaces the bytes and
counts it once. A write of the wrong length is `SliceSizeMismatch` before
any range check. `Budgeted::bytes` is the store length. The level axis and
the presence set are not counted again.

`Volume::cut_ready_brick` copies one brick `ready_bricks` lists, from the
level 0 prefix only. A coordinate that list omits is
`VolumeError::BrickNotReady` and nothing is copied.
`Volume::present_slice_f32` returns one present slice as little-endian
container values in `f32`. A clear presence bit, including an index at or
past the slice count, is `VolumeError::SliceNotPresent`. Bits Stored is not
applied again. The bytes `FD FF` of an `I16` voxel stay `-3`.

`VolumeCache` does not share `ImageCache`'s budget. One volume admitted
against the frame budget would evict every frame a viewer is holding.
Admission, eviction and pressure are `Lru`'s, on the decoded tier.

## The level axis

Level 0 is full resolution at offset 0. `Volume::append_level` copies
caller-supplied bytes onto the end of the same store and pushes the next
`Level`. It does not downsample. Pyramid generation is F-119. Opacity
correction is F-121.

Spacing on the new entry is level 0 spacing times level 0's dimension
divided by the child dimension, on each axis. The ratio is baked at append
time from level 0.

`Volume::level` and `Volume::brick_grid_at` return
`VolumeError::LevelAbsent` when the index is missing. They do not substitute
level 0. A zero axis, an axis larger than level 0, a grid that is not
smaller on at least one axis, or a buffer whose length is not the child
voxel count times the voxel width, is `VolumeError::LevelShape`. The store
does not grow on that refusal.

`Volume::brick_grid` stays level 0. `brick_grid_at` builds `BrickGrid::new`
for that level's dimensions and then sets `level`. There is no second edge
table. `Volume::level_bytes` returns the entry's span in the store.

Assembly and `construct` still write one level. `SlicePresence` stays the
level 0 slice set. Renderers in S19 upload level 0 only.

## Orthographic planes

`Volume::reslice` samples one plane from level 0. `Axial` holds the slice
index and steps along both in-plane cosines. `Sagittal` holds the column
index and steps along the column cosine and the slice normal. `Coronal`
holds the row index and steps along the row cosine and the normal. The
names are this volume's axes.

`Volume::voxel_world` is the patient point of a continuous index
`(i, j, k)`. The origin is the centre of voxel `(0, 0, 0)`. The point is
the origin plus `i * spacing[0] * direction.col(0)` plus
`j * spacing[1] * direction.col(1)` plus
`k * spacing[2] * direction.col(2)`. `spacing[0]` is column millimetres
along the row cosine. That is DICOM PS3.3 C.7.6.2.1.1 with the volume's
third axis.

An index on the fixed axis outside `[0, dim - 1]`, including a non-finite
index, is `VolumeError::PlaneOutOfRange`. Nothing is allocated. On a voxel
centre the sample is that voxel's container value. Off centre the weights
are trilinear in `f64` and then narrowed to `f32`. A corner with non-zero
weight on an absent slice is `PlaneSample::Absent`. A zero weight does not
read that corner. Decode is little-endian `U8`, `I16`, `U16` or `F32`.
Bits Stored is not applied again. An appended coarser level is not a
sample source.

Output size is the two stepped axes. Axial spacing is `spacing[0]` by
`spacing[1]`. Sagittal spacing is `spacing[1]` by `spacing[2]`. Coronal
spacing is `spacing[0]` by `spacing[2]`.

## Tiers A, B and C

Not applicable for A and B. The volume is host memory and this crate creates
no device. The brick upload in `ocelli-render` refuses an axis longer than
the device's reported 3D limit. The pinned tier B floor for that limit is
256, which is the size a brick edge of 128 is guaranteed to fit. This crate
does not clamp the reported limit and does not apply the check. Tier C
assembles the same volume, because there is no device path to diverge from.

## What this does not do

A volume enumerates bricks through `BrickGrid` from `ocelli-cache`, via
`Volume::brick_grid`. It does not keep a second decomposition.
`Volume::ready_bricks` lists brick coordinates whose slices have all landed,
in `index_of` order, x fastest. A brick whose z range includes an absent
slice is left out, because uploading it would invent zeros. This crate
still creates no texture. `RenderGraph::sync_volume` in `ocelli-render`
uploads a ready brick. `RenderGraph::show_ortho` uploads one complete
orthographic plane. Pixels stay in the store and do not cross the worker
boundary.
