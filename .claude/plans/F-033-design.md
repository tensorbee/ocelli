# F-033, Volume cache and progressive volume assembly

**Status**: approved
**Epic ref**: E5.3
**Sprint**: S12
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/16-volume-representation.md`, section 19, in full

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

The three bullets, transcribed into table rows so the author's text is quoted
exactly and `scripts/prose_check.py` still passes:

|  |
|----|
| **Layout.** One contiguous allocation, x fastest then y then z. Slice n starts at n \* dims\[0\] \* dims\[1\] \* bytes_per_voxel. Do not introduce a per-slice Vec, the whole point is a single upload region. |
| **Progressive assembly.** Clear the 3D texture at creation and upload slices as they land. present drives the loading indicator and tells the oracle which frames are comparable yet. |
| **Bricks and a level axis, from the start.** The volume carries a multiscale level axis and 128³ brick decomposition even when a series fits comfortably. Phase 1 uploads every brick, §30 makes residency selective. Retrofitting a level axis into a volume model that never had one is the rewrite cornerstone3D cannot afford. |

### `docs/hld/06-memory-and-cache.md`, section 8, the sentence this story implements

|  |
|----|
| Volume assembly is progressive by default: the viewport renders a partial volume and refines as slices land, which matters more for perceived speed than any decode optimisation. The absence of a garbage collector is the real memory story - a 300 MB volume load has no pause behaviour to tune around, only a budget to respect. |

### `docs/hld/27-phase1-hooks.md`, section 38

| **Hook** | **Story** | **Now** | **Later** |
|----|----|----|----|
| Multiscale level axis on the volume | E8.8 | 3 wk | Changing the type every viewport and tool reads |

### DICOM PS3.3 C.7.6.2.1.1, the voxel-to-patient mapping

With `X = IOP[0..3]` the row direction, `Y = IOP[3..6]` the column direction,
`i` the **column** index and `j` the **row** index:

```text
P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
```

`PixelSpacing` is `[between rows, between columns]`. `PixelSpacing[0]` is the
row spacing, which is the vertical one, so it multiplies the **column**
direction cosine. The slice normal is `N = X x Y`.

### What the specification does NOT say about spacing, and why it is this story

Section 19 gives `spacing: [f64; 3]` and never says where the third component
comes from. PS3.3 defines `SpacingBetweenSlices` (0018,0088) and
`SliceThickness` (0018,0050) and neither is slice spacing:

- `SliceThickness` is the reconstructed slab thickness and may overlap or gap.
- `SpacingBetweenSlices` is frequently absent and, when present, frequently
  wrong.

**The ground truth is the difference between consecutive `ImagePositionPatient`
values projected onto the slice normal**, `dot(IPP, N)`. This is the defect
`docs/sprints/CURRENT_SPRINT.md` names for this sprint.

## What the specification does not cover

1. **Where the third spacing component comes from.** Above. Section 19 states
   the field and not its derivation, and getting it from a tag is the defect
   class.
2. **What `Level` is.** Section 19 names `levels: Vec<Level>` and defines no
   `Level` anywhere in the HLD. E8.8 is the multiscale story and is not this
   one. See Open question C.
3. **What `VoxelStore` is.** Named as a field type, never defined. Section 19's
   layout bullet constrains it completely: one contiguous allocation, x fastest,
   slice `n` at `n * dims[0] * dims[1] * bytes_per_voxel`. This plan makes it a
   newtype over `Vec<u8>` holding that invariant, because "do not introduce a
   per-slice Vec" is the only thing said about it.
4. **`bitvec::BitVec`.** Section 19 names the crate by path and `bitvec` is not
   a workspace dependency and is not in section 15.2's list. See Open question D.
5. **What "refuse" means for non-uniform spacing.** Section 19 says nothing.
   The `dicom-expert` reference and this sprint's own defect class say a volume
   model that assumes uniform spacing must detect and refuse rather than
   average. This plan refuses with a typed error and states the tolerance.
6. **How a series becomes a candidate volume at all.** A series can hold two
   orientations, a localiser among axials, and duplicate positions, so grouping
   precedes assembly. Section 19 starts after that grouping.
7. **The volume cache's key and budget.** Section 8 gives the decoded tier and
   the GPU tier, and a volume is neither a frame nor a texture. See Approach 5.

## Approach

### 1. Grouping, before anything is assembled

```rust
/// A set of frames that could form one volume, per PS3.3.
pub struct VolumeCandidate {
    frame_of_reference: FrameOfReferenceUid,
    orientation: ImageOrientationPatient,
    slices: Vec<SliceEvidence>,
}
```

Grouped by `FrameOfReferenceUID` **and** `ImageOrientationPatient`, because a
series carrying a localiser has two orientations and is not one volume, and two
series share a spatial frame only if they share the frame-of-reference UID.

### 2. Sorting and spacing, the story's whole risk

```rust
/// The slice normal, N = X x Y from the two direction cosines.
fn slice_normal(iop: &ImageOrientationPatient) -> DVec3;

/// Each slice's position along the normal, PS3.3 C.7.6.2.1.1.
fn projected_position(ipp: &ImagePositionPatient, normal: DVec3) -> f64 {
    ipp.as_dvec3().dot(normal)
}
```

Slices sort by `projected_position` ascending. **Never by `InstanceNumber` and
never by `SliceLocation`.** Spacing is the difference between consecutive
projected positions, and `SpacingBetweenSlices` and `SliceThickness` are **not
read by this story at all**, which is stronger than reading and ignoring them.

The normal is normalised rather than assumed unit, because real files carry
cosines that are not quite normalised.

### 3. Refusing what cannot be a volume

```rust
pub enum VolumeError {
    /// Fewer than two slices, so no spacing can be derived.
    TooFewSlices,
    /// Two slices share a projected position.
    DuplicatePosition { at: f64 },
    /// The gaps are not uniform within tolerance.
    NonUniformSpacing { median: f64, worst: f64, index: usize },
    /// The inter-slice vector is not parallel to the slice normal.
    ShearedVolume { cosine: f64 },
    /// Mixed orientations or frames of reference reached assembly.
    NotOneVolume,
}
```

**Uniformity is measured against the median gap, not the mean**, because one
bad gap drags a mean and then every gap looks slightly wrong instead of one
looking very wrong. See Open question E for the tolerance.

**Shear is detected from the geometry**, by testing whether the inter-slice
vector is parallel to the slice normal, and **not** from `GantryDetectorTilt`
(0018,1120). F-020 already established that rule for the image plane and this
story does not introduce a second way of asking.

### 4. `Volume`, section 19's struct

```rust
pub struct Volume {
    pub dims: [u32; 3],
    pub spacing: [f64; 3],
    pub origin: Pt<World>,
    pub direction: DMat3,
    pub voxel: VoxelKind,
    pub data: VoxelStore,
    pub levels: Vec<Level>,
    pub present: SlicePresence,
}
```

`origin` is the first slice's `ImagePositionPatient`, which PS3.3 defines as the
centre of the first voxel and **not its corner**. A half-voxel error there
propagates into every measurement and every fused overlay and is exactly the
size that reads as acceptable registration error.

`VoxelStore` is a newtype over `Vec<u8>` whose constructor takes `dims` and
`VoxelKind` and whose only accessor is a slice-indexed write, so section 19's
layout rule has one implementation:

```rust
impl VoxelStore {
    /// Slice n starts at n * dims[0] * dims[1] * bytes_per_voxel.
    pub fn slice_mut(&mut self, n: u32) -> Result<&mut [u8], VolumeError>;
}
```

### 5. Progressive assembly and the volume cache

```rust
impl Volume {
    /// Write one slice and mark it present. Idempotent.
    pub fn place(&mut self, n: u32, samples: &[u8]) -> Result<(), VolumeError>;
    /// How many slices have landed, which drives the loading indicator.
    pub fn present_count(&self) -> u32;
    pub fn is_complete(&self) -> bool;
}

impl Budgeted for Volume {
    /// The contiguous allocation. The level axis and the presence bitset are
    /// bounded by it and are not counted separately.
    fn bytes(&self) -> usize { self.data.len() }
}

pub struct VolumeCache { volumes: Lru<VolumeKey, Volume> }
```

The cache is `Lru` from F-031 over a `CacheTier`. **Which tier is Open question
F.** Section 8 names encoded, decoded and GPU, and a host-side volume is a
decoded thing that is not a frame.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. `Volume` holds voxels and never crosses.
  Decision D3
- Render-loop allocation: **none in the loop, and the allocation is the
  point.** `Volume::new` makes one contiguous allocation at creation and
  `place` writes into it. Section 19's "do not introduce a per-slice Vec" is
  what makes that true, and `place` is a decode-path call rather than a
  render-loop one
- unsafe: none. `bytemuck::cast_slice` reinterprets the sample buffer, which is
  section 20's third bullet by name
- Tier A (WebGPU): n/a as written. The volume is host memory and F-040's upload
  is what puts it on a device
- Tier B (WebGL2): n/a, same reason. **Section 7's 3D-texture floor of 256
  against tier A's 2048 will matter to whoever uploads this**, and `dims`
  carries the number that decides it
- Tier C (CPU): **full.** A tier C session assembles and reslices the same
  volume, which is deviation D-07's whole point, and nothing here needs a device

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | **Spacing is the projected IPP difference and not a tag.** A five-slice series whose `SpacingBetweenSlices` says 5.0 and whose projected positions are 0, 3, 6, 9, 12 assembles with spacing 3.0. **The two disagree on purpose**, because a series where they agree cannot separate the two implementations. PS3.3 C.7.6.2.1.1 | `crates/ocelli-volume/tests/assembly.rs` |
| `fixture` | Sorting is by projected position. A series delivered with `InstanceNumber` ascending and projected position descending assembles in projected order, and `origin` is the slice at the lowest projection | same |
| `fixture` | `PixelSpacing[0]` multiplies the **column** direction cosine. A non-square 0.5 by 1.0 pixel with distinct row and column cosines maps voxel (1,0) and (0,1) to hand-computed patient points that differ. A square pixel cannot catch this and is why the fixture is not square | same |
| `fixture` | `origin` is the first voxel's centre. Hand-computed against an IPP, asserted to equal it exactly rather than offset by half a voxel | same |
| `fixture` | Slice `n` starts at `n * dims[0] * dims[1] * bytes_per_voxel`. A 4 by 3 by 2 sixteen-bit volume puts slice 1 at byte 24, hand-computed, asserted on the buffer | same |
| `unit` | Non-uniform spacing is refused with the median, the worst gap and its index, not averaged. Gaps 3, 3, 7, 3 refuse and name index 2 | same |
| `unit` | A duplicate projected position is refused | same |
| `unit` | Shear is detected from the inter-slice vector against the normal, with `GantryDetectorTilt` absent from the fixture entirely, so a tilt-tag reader cannot pass | same |
| `unit` | Two orientations in one candidate refuse with `NotOneVolume` | same |
| `unit` | A single slice refuses with `TooFewSlices` rather than assuming a spacing | same |
| `unit` | `place` is idempotent and `present_count` does not double-count | same |
| `unit` | A partial volume reports `is_complete` false and the unwritten slices read as the cleared value | same |
| `property` | For a randomised uniform series, the assembled spacing equals the generating spacing within 1e-9 mm, and `origin` equals the lowest-projection IPP exactly | same |

**Mutation check, HLD 27.3.** Read `SpacingBetweenSlices` instead of projecting,
and the first fixture goes red. Sort by `InstanceNumber`, and the second goes
red. Swap `PixelSpacing[0]` and `[1]`, and the non-square fixture goes red.
Offset `origin` by half a voxel, and its fixture goes red. Average the gaps
instead of refusing, and the non-uniform test goes red. Use the mean rather than
the median, and the 3, 3, 7, 3 case names the wrong index.

## Parity surface covered

None of `docs/hld/B-parity-surface.md`'s rows. Its surfaces are viewport types,
tool classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation
representations, events and adapters. A volume model underlies several and is
none of them.

## Deviations

Deviation D-25, raised by this plan in the S12 design round. The hand-rolled
`SlicePresence` instead of `bitvec::BitVec` is D-25, because section 19 names
the type. A `levels` field carrying exactly one entry takes none, because
section 19 asks for the axis and E8.8 fills it.

## LLD impact

- A new `docs/lld/volumes.md`, because no LLD file covers `ocelli-volume`.
  `docs/lld/README.md` gains its row
- `docs/lld/cache.md`, which gains the volume tier

## Open questions

None. All questions were answered in the S12 consolidated design round, as recorded below.

## Decisions from the S12 design round

**C, C1. The level axis lands now with exactly one entry,** level 0 at full resolution. `Level` carries `dims`, `spacing` and the store offset. E8.8 adds entries rather than the field.

**D, D1. A hand-rolled `SlicePresence` newtype over `Vec<u64>`.** Recorded as deviation D-25, because section 19 names `bitvec::BitVec`.

**E, E1. A gap is uniform if it is within 0.5 per cent of the median gap.** At 3 mm spacing that is 15 micrometres, far below any real reconstruction increment and far above `f64` noise in a dot product. This is a new acceptance bound recorded here with its rationale, on section 25.1 own rule for a bound written for the first time.

**F, F1. A volume sits in `CacheTier::Decoded`,** in its own `Lru` with its own budget.

## Recorded questions

**C. How much of the level axis lands now.** Section 19 says "bricks and a level
axis, from the start... even when a series fits comfortably", and section 38
makes it a Phase 1 hook whose later cost is "changing the type every viewport
and tool reads". E8.8 is the multiscale story and is not in this sprint.
- **C1, this plan's recommendation.** `levels: Vec<Level>` exists with exactly
  one entry, level 0, full resolution, and `Level` carries `dims`, `spacing` and
  the store offset. The axis is in the type from the start, which is what
  section 38 is protecting, and E8.8 adds entries rather than adding the field.
- **C2.** Omit `levels` and let E8.8 add it. Departs from section 19's struct
  and from section 38's hook, so it needs a `D-NN` row and re-opens the retrofit
  the hook exists to avoid.

**D. `bitvec::BitVec`, or a hand-rolled presence set.** Section 19 names
`bitvec::BitVec`. `bitvec` is not a workspace dependency and is not in HLD
section 15.2's list.
- **D1, this plan's recommendation.** A `SlicePresence` newtype over
  `Vec<u64>`, roughly forty lines, `no_std` with `alloc`, no new dependency.
  A 600-slice volume needs 75 bytes. Takes a `D-NN` row because section 19 names
  the type.
- **D2.** Add `bitvec` to the workspace. Matches section 19 exactly and adds a
  shipped dependency outside section 15.2's list for one bit per slice.

**E. The uniformity tolerance, and it is a tolerance so it needs a rationale.**
HLD section 25.1's tolerances are the oracle's and cover geometry as "world
coordinates within 1e-6 mm", which is a comparison bound rather than an
acceptance bound for real scanner data.
- **E1, this plan's recommendation.** A gap is uniform if it is within **0.5 per
  cent of the median gap**, which at a 3 mm spacing is 15 micrometres, far below
  any real reconstruction increment and far above `f64` noise in a dot product.
  Recorded in the plan with its rationale, which is section 25.1's own rule
  applied to a bound written for the first time.
- **E2.** An absolute bound in millimetres. Wrong shape: 10 micrometres is
  strict at 0.5 mm spacing and meaningless at 10 mm.
- **E3.** Refuse any inequality at all. Rejected: `f64` projection of real
  direction cosines does not reproduce an exactly constant gap.

**F. Which `CacheTier` a volume belongs to.** Section 8 names encoded, decoded
and GPU. A host-side assembled volume is decoded and is not a frame, and it
competes with frames for the same host memory.
- **F1, this plan's recommendation.** `CacheTier::Decoded`, in its own `Lru`
  with its own budget. One tier label, two caches, which `Pressure` already
  distinguishes by carrying the tier and which F-034 reports per cache.
- **F2.** A fourth `CacheTier::Volume` variant. Departs from section 8's three
  and needs a `D-NN` row, and F-031's `CacheTier` match arms all become
  non-exhaustive.
