# Volumes

**F-IDs that contributed:** F-033, F-036, F-043
**Last updated:** 2026-10-18

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
| `Level` | One multiscale entry. This story holds exactly one, at offset 0 |
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
`n`. A caller places each arriving frame at that position and does not
project the positions a second time outside this crate.

## The store and progressive fill

Slice `n` starts at `n * dims[0] * dims[1] * bytes_per_voxel` inside the one
allocation made at assembly. `place` copies one slice into that range and
sets its presence bit. Writing the same slice again replaces the bytes and
counts it once. A write of the wrong length is `SliceSizeMismatch` before
any range check. `Budgeted::bytes` is the store length. The level axis and
the presence set are not counted again.

`VolumeCache` does not share `ImageCache`'s budget. One volume admitted
against the frame budget would evict every frame a viewer is holding.
Admission, eviction and pressure are `Lru`'s, on the decoded tier.

## Tiers A, B and C

Not applicable for A and B. The volume is host memory and this crate creates
no device. Tier B's 3D texture limit of 256 is enforced by the brick upload
in `ocelli-render`, not by this crate. Tier C assembles the same volume,
because there is no device path to diverge from.

## What this does not do

A volume enumerates bricks through `BrickGrid` from `ocelli-cache`, via
`Volume::brick_grid`. It does not keep a second decomposition.
`Volume::ready_bricks` lists brick coordinates whose slices have all landed,
in `index_of` order, x fastest. A brick whose z range includes an absent
slice is left out, because uploading it would invent zeros. The renderer
does not read `SlicePresence`. No reslice onto a viewport, which is a later
geometry story. This crate still creates no texture. Upload of a ready brick
is `ocelli-render`. Pixels stay in the store and do not cross the worker
boundary.
