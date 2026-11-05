# F-119, Brick decomposition and multiscale pyramid generation

**Status**: approved
**Epic ref**: E30.1
**Sprint**: S46
**Estimate**: 10w

## Normative source, transcribed

### `docs/hld/26-differentiating-capabilities.md`, section 30, Out-of-core volume streaming

The section's bullets, quoted as table cells so the author's punctuation stays data:

|  |
|----|
| Cornerstone3D loads whole volumes into one WebGL texture and runs out of memory; the shipped mitigation is overlapping partitions swapped on scroll. Neuroglancer solved this properly at petabyte scale a decade ago and nobody brought the design to DICOM. |
| **Residency** is the view frustum intersected with the current level of detail, recomputed per frame against a byte budget. GPU memory becomes a configured number rather than a function of series size. |
| **Level selection** compares projected voxel size against screen pixel size. |
| **Bricks** are 128³, or 64³ for sixteen-bit data, addressed through a 3D indirection texture. The cache is ocelli-cache with a brick key — no second cache. |
| **Prefetch** runs along the scroll axis, which is where the next request almost always is. |
| **Time is another chunk axis.** This is what makes 4D a first-class data model instead of a special case of the memory problem. |

Section 30.1, the detail that is usually wrong:

```text
// alpha' = 1 - (1 - alpha) ^ (ds / ds_ref)
fn correct_opacity(alpha: f32, ds: f32, ds_ref: f32) -> f32 {
    1.0 - (1.0 - alpha).powf(ds / ds_ref)
}
```

That function is E30.3, F-121. This story does not implement it.

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

|  |
|----|
| **Layout.** One contiguous allocation, x fastest then y then z. Slice n starts at n \* dims\[0\] \* dims\[1\] \* bytes_per_voxel. Do not introduce a per-slice Vec; the whole point is a single upload region. |
| **Bricks and a level axis, from the start.** The volume carries a multiscale level axis and 128³ brick decomposition even when a series fits comfortably. Phase 1 uploads every brick; §30 makes residency selective. Retrofitting a level axis into a volume model that never had one is the rewrite cornerstone3D cannot afford. |

### `docs/hld/05-rendering.md`, section 7

|  |
|----|
| **Bricking above 256 MiB.** A 512×512×600 sixteen-bit CT series is roughly 300 MB against a guaranteed maximum buffer size of 256 MiB, so chunked upload is the normal path, not an optimisation. |

### `docs/hld/11-decision-log.md`, D11

| # | Decision | Rejected | Why |
|---|----------|----------|-----|
| D11 | Chunked residency is the default path | Brick only above a size threshold | Bounded GPU memory becomes a claim, not a fallback |

### `docs/hld/27-phase1-hooks.md`, section 38

| Hook | Story | Now | Later |
|------|-------|-----|-------|
| Chunked residency in the cache | E5.6 | 4 wk | Re-architecting the cache and every consumer of it |
| Multiscale level axis on the volume | E8.8 | 3 wk | Changing the type every viewport and tool reads |

### `docs/hld/17-cache-and-allocation.md`, section 20

```rust
pub trait Budgeted { fn bytes(&self) -> usize; }
pub struct Lru<K, V: Budgeted> {
    budget: usize,
    used: usize,
    /* ... */
}
```

| Rule |
|------|
| No allocation in the render loop. Pre-size everything at viewport creation. A frame that allocates is a frame that can stutter. |

`insert` returns `Admission<K, V>` rather than section 20's `Vec<(K, V)>`, which is deviation D-24. This story does not touch it.

### Backlog, allocation and sprint

| Source | Text |
|--------|------|
| `BACKLOG.md` row | `F-119, E30.1, S46, Brick decomposition and multiscale pyramid generation, Rust, 10w, depends on F-058, pending` |
| `allocation.json` notes | `Neuroglancer's design; nothing in DICOM does this` |
| `CURRENT_SPRINT.md`, carried in | One cache. A brick is a key in `ocelli-cache`. There is no second cache. Level 0 is full resolution. A series that fits in memory still has the level axis. |
| `CURRENT_SPRINT.md`, defect class | A child level that is not a downsample of level 0. Spacing on that level copied from slice thickness instead of the dimension ratio. A missing level that returns level 0. A second brick edge beside `BrickGrid::new`. |
| `CURRENT_SPRINT.md`, what done means | F-119 generates the coarser levels by downsampling level 0. `append_level` stores caller-supplied bytes and does not downsample, and this story is that downsample. Level 0 stays the full grid. Child spacing is level 0 spacing times the level 0 dimension divided by the child dimension, on each axis. The brick edge stays the one `BrickGrid::new` already chooses. Opacity correction stays F-121. |

### What already exists, read from the tree

`crates/ocelli-cache/src/brick.rs`:

```rust
pub struct BrickKey { pub volume: VolumeId, pub level: u32, pub brick: [u32; 3] }
pub struct BrickGrid { pub dims: [u32; 3], pub edge: u32, pub grid: [u32; 3], pub level: u32 }
impl BrickGrid {
    pub fn new(dims: [u32; 3], voxel: BrickWidth) -> Self; // EightBit 128, SixteenBit 64, Float32 64
    pub fn brick_of(&self, voxel: [u32; 3]) -> Result<[u32; 3], BrickError>;
    pub fn origin_of(&self, brick: [u32; 3]) -> Result<[u32; 3], BrickError>;
    pub fn extent_of(&self, brick: [u32; 3]) -> Result<[u32; 3], BrickError>;
    pub fn index_of(&self, brick: [u32; 3]) -> Result<usize, BrickError>;
    pub fn byte_offset(&self, voxel: [u32; 3], bytes_per_voxel: usize) -> Result<usize, BrickError>;
}
impl Brick {
    pub fn cut(grid: &BrickGrid, brick: [u32; 3], volume: &[u8], sample: BrickSample) -> Result<Self, BrickError>;
}
impl BrickCache {
    pub fn new(budget: usize) -> Self; // CacheTier::Gpu
    pub fn insert(&mut self, key: BrickKey, brick: Brick) -> Admission<BrickKey, Brick>;
    pub fn get(&mut self, key: &BrickKey) -> Option<&Brick>;
    pub fn missing(&self, volume: VolumeId, grid: &BrickGrid) -> Vec<BrickKey>;
    pub fn is_resident(&self, volume: VolumeId, grid: &BrickGrid) -> bool;
}
```

`crates/ocelli-volume/src/levels.rs`, from F-058:

```rust
impl Volume {
    pub fn level(&self, index: u32) -> Result<&Level, VolumeError>;
    pub fn level_bytes(&self, index: u32) -> Result<&[u8], VolumeError>;
    pub fn append_level(&mut self, dims: [u32; 3], samples: &[u8]) -> Result<u32, VolumeError>;
    pub fn brick_grid_at(&self, index: u32) -> Result<BrickGrid, VolumeError>;
}
fn coarser(full: [u32; 3], dims: [u32; 3]) -> bool;            // every axis in 1..=full, one shorter
fn scaled(full_dims: [u32; 3], full_spacing: [f64; 3], dims: [u32; 3]) -> Option<[f64; 3]>; // spacing_0 * dims_0 / dims
```

`crates/ocelli-volume/src/lib.rs`: `Level { dims, spacing, offset }`, `VolumeError::LevelAbsent { index }`, `VolumeError::LevelShape`, `VolumeError::SliceNotPresent { index }`, `Volume::brick_grid` (level 0), `Volume::ready_bricks`, `Volume::cut_ready_brick` (level 0 prefix only), `Volume::place`, `Volume::is_complete`, `VoxelStore::extend_bytes` (`pub(crate)`). `Volume.origin` is the first slice's `ImagePositionPatient`, the centre of voxel zero. The store holds stored container values. `place` copies the decoded container and nothing applies the modality stage inside this crate.

`crates/ocelli-render/src/progressive.rs`: `sync_ready_bricks` cuts every `ready_bricks` coordinate through `cut_ready_brick` and keys it `BrickKey { level: 0, .. }`. Tier C returns `BrickUploadError::Unavailable` before any cut.

`crates/ocelli-volume/src/ortho.rs`: a private `narrow_f32(value: f64) -> f32`, ties-to-even, because `as f32` is denied by the workspace lints.

## What the specification does not cover

Section 30 names bricks, a level axis and level selection. Section 19 says level 0 is full resolution. **Neither names a downsample filter, a value domain, a rounding rule, a child-dimension rule, a stop rule, or a level origin.** The allocation note says nothing in DICOM does this, and no PS3.3 section specifies a pyramid. Every item below is this plan's decision. Each is also listed under Open questions with its alternatives, because each changes a fixture.

1. **Filter: an exact-overlap box filter, computed from level 0.** On each axis, with parent dimension `D` and child dimension `d`, child voxel `c` covers the level 0 continuous interval `[c * D / d, (c + 1) * D / d)`. Multiplying by `d` makes every boundary an integer, `[c * D, (c + 1) * D)` against parent voxel `p` at `[p * d, (p + 1) * d)`. The axis weight is the integer overlap `w(c, p) = max(0, min((c + 1) * D, (p + 1) * d) - max(c * D, p * d))`, and the weights of one child sum to `D`. The 3D weight is the product of the three axis weights and the child value is `sum(w * v) / (Dx * Dy * Dz)`. **For an even axis halved, `D = 2d`, this is exactly the 2 by 2 by 2 box average.** For an odd axis it is the footprint the dimension-ratio spacing already claims, which the plain 2x box with a short edge cell is not. See Q1.
2. **Every level is computed from level 0, never from the level above it.** Rounding at level 1 would otherwise compound into level 2, and level 2 would not be a downsample of level 0, which is the sprint's first named defect. See Q6.
3. **Domain: the stored container values the store already holds.** Rescale Slope and Intercept, PS3.3 C.11.1, are affine, so the weighted mean of stored values maps to the weighted mean of modality values exactly before rounding. Rounding in the stored domain moves a modality value by at most `0.5 * |slope|`. A Modality LUT Sequence is not affine and the mean does not commute with it. See Q3.
4. **Integer rounding: nearest, ties away from zero, in exact `i128` arithmetic.** For a sum `S` and a weight total `W`, the result is `(2S + W) div (2W)` for `S >= 0` and `-((-2S + W) div (2W))` for `S < 0`. The mean of in-range samples is in range, so no clamp is needed, and the narrowing back to `u8`, `i16` or `u16` is `try_from`. Rust's `/` truncates toward zero and is the plausible wrong implementation. See Q2.
5. **Float: weights as `f64` from `u32`, the mean in `f64`, narrowed by the existing ties-to-even `narrow_f32`.** There is no `as` cast. The weight product is exact while `Dx * Dy * Dz < 2^53`.
6. **Child dimensions: `max(1, ceil(D0 / 2^k))` on every axis, for level `k`.** `ceil(ceil(D / 2) / 2) = ceil(D / 4)` for integers, so this is also the iterated halving. Floor would drop the last plane of an odd axis. See Q4 for the anisotropic alternative.
7. **Stop rule: stop after the first level whose `BrickGrid::new` grid is `[1, 1, 1]`.** A volume whose level 0 is already one brick gets no coarser level. The edge comes from `brick_grid_at`, which calls `BrickGrid::new`, so the stop rule and the brick edge share one match. See Q5.
8. **Spacing is never computed by the generator.** The generator hands bytes to `append_level`, and `append_level`'s `scaled` is the only spacing formula. A child level cannot get spacing from slice thickness because there is no second place spacing is written. The volume does not read Slice Thickness (0018,0050) at all, per the crate header.
9. **Level 0 must be complete.** Generation refuses with `SliceNotPresent { index }` naming the first absent slice. Averaging an absent slice would invent the zeros `ready_bricks` already refuses to upload.
10. **A later `place` drops every level above 0.** A level is a function of level 0, so a write to level 0 makes it stale, and a stale level reads as a quietly wrong pixel. After the drop, `level(1)` is `LevelAbsent`. See Q7.
11. **Generating again replaces every level above 0.** The result is a function of level 0 only, so a repeated call gives the same store bytes and the same entries. See Q8.
12. **Level geometry: one mapping from a level voxel to level 0 continuous index.** `Level` has no origin. Under the extent-preserving convention of decision 1, level voxel `c` sits at level 0 index `u = (c + 0.5) * D / d - 0.5` on each axis. For a halved axis, level 1 voxel 0 is at `0.5`, between level 0 voxels 0 and 1. A consumer that writes `origin + c * spacing_k` is half a level 0 voxel wrong at level 1 and one and a half at level 2. This plan adds `Volume::level_voxel_to_level0` so F-120 does not derive it. See Q9.
13. **A brick at level `k` is cut from that level's span.** `Volume::cut_level_brick(index, brick)` uses `brick_grid_at(index)` and `level_bytes(index)` and keys nothing itself. Level 0 delegates to `cut_ready_brick`, so readiness keeps one predicate.

## Problem and reproducer

Trigger: a complete `Volume` of `[130, 64, 64]` `U16` samples. Expected: three levels, `[130, 64, 64]`, `[65, 32, 32]` and `[33, 16, 16]`, each a box average of level 0, each with dimension-ratio spacing, and a `Brick` cut from any of them on a 64 edge. Current behaviour: `levels.len()` is 1 unless a caller hands `append_level` bytes it computed itself. Nothing in the workspace computes them. `BrickKey.level` is always 0 in production, set by `sync_ready_bricks`.

This is a story because E30.2 to E30.4 select, sample and budget levels that do not exist yet, and because the filter, the rounding and the level geometry are three places a locally reasonable choice is a quietly wrong voxel.

## Scope, API and compatibility

In scope, all in `ocelli-volume`:

```rust
impl Volume {
    /// Downsample level 0 to `dims` and append it. Returns the new index.
    ///
    /// The exact-overlap box filter of F-119 decision 1, over stored values.
    /// Spacing comes from `append_level`.
    ///
    /// # Errors
    ///
    /// [`VolumeError::SliceNotPresent`] when level 0 is incomplete.
    /// [`VolumeError::LevelShape`] when `dims` is not strictly coarser.
    /// The volume is unchanged in both cases.
    pub fn downsample_level(&mut self, dims: [u32; 3]) -> Result<u32, VolumeError>;

    /// Replace every level above 0 with the generated pyramid.
    ///
    /// Level `k` has `max(1, ceil(dims_0 / 2^k))` on each axis. Generation
    /// stops after the first level whose brick grid is one brick. Returns
    /// `levels.len()` after generation, which is 1 for a one-brick level 0.
    ///
    /// # Errors
    ///
    /// [`VolumeError::SliceNotPresent`] when level 0 is incomplete, with the
    /// volume unchanged.
    pub fn generate_pyramid(&mut self) -> Result<u32, VolumeError>;

    /// Copy one brick of stored level `index`.
    ///
    /// Level 0 is [`Volume::cut_ready_brick`].
    ///
    /// # Errors
    ///
    /// [`VolumeError::LevelAbsent`] past the axis, [`VolumeError::BrickCut`]
    /// outside that level's grid, and level 0's own refusals.
    pub fn cut_level_brick(&self, index: u32, brick: [u32; 3]) -> Result<Brick, VolumeError>;

    /// The level 0 continuous voxel index of a level `index` voxel centre.
    ///
    /// `(c + 0.5) * dims_0 / dims_k - 0.5` on each axis.
    ///
    /// # Errors
    ///
    /// [`VolumeError::LevelAbsent`] past the axis.
    pub fn level_voxel_to_level0(&self, index: u32, voxel: [f64; 3]) -> Result<[f64; 3], VolumeError>;
}
```

`Volume::place` keeps its signature and gains decision 10: after a successful write, when `levels.len() > 1`, the store is truncated to level 0's byte length and `levels` to one entry. A refused `place` changes nothing. `VoxelStore` gains a `pub(crate)` truncate beside `extend_bytes`. `narrow_f32` in `ortho.rs` becomes `pub(crate)`.

Unchanged: `append_level`, `level`, `level_bytes`, `brick_grid`, `brick_grid_at`, `ready_bricks`, `cut_ready_brick`, every `ocelli-cache` type, and `sync_ready_bricks`. No new `VolumeError` variant. No new dependency.

Non-goals:

- Opacity correction, section 30.1. F-121.
- Selecting a level, the frustum and LOD selection. F-120.
- Uploading a level above 0, the residency budget, prefetch and eviction. F-122. `sync_ready_bricks` stays level 0.
- A 3D indirection texture. Section 30 names it and no story before F-120 binds it.
- Calling `generate_pyramid` from `construct`, `ocelli-wasm`, `ocelli-native` or an example. Generation is an explicit call this sprint. See Q10.
- A label or segmentation volume. A box mean of labels invents labels. Nothing in this crate marks a volume as labels today.
- Pixel Padding Value (0028,0120) aware averaging. A padded edge averages padding into tissue at a coarse level. See Q11.
- Runtime pyramid discovery from VL Whole Slide Microscopy metadata. That is F-143, which supplies its own levels through `append_level`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `Volume::generate_pyramid` | `downsample_level` per level, `brick_grid_at` for the stop rule | Load time, after `is_complete` | `levels.len()`, or `SliceNotPresent` with the volume unchanged |
| `Volume::downsample_level` | The filter in `levels.rs`, then `append_level` | Load time. Owns filter, domain and rounding | New index, or `SliceNotPresent` or `LevelShape` with the volume unchanged |
| `Volume::append_level` | `coarser`, `byte_len`, `scaled` | Unchanged. Owns the only spacing formula | Unchanged |
| `Volume::cut_level_brick` | `brick_grid_at`, `level_bytes`, `Brick::cut`, `BrickSample::from(VoxelKind)` | Load time or F-122 residency. Not the render loop | A `Brick` at that level's true extent |
| `Volume::level_voxel_to_level0` | `Level.dims` and level 0 dims | F-120 level-to-world mapping | Level 0 continuous index. Owns the half-voxel convention |
| `Volume::place` | `VoxelStore::slice_mut`, then truncate | Progressive assembly | Level 0 written, levels above 0 dropped |
| `BrickCache` | `ocelli-cache`, unchanged | One GPU-tier cache keyed by `BrickKey.level` | Level 1 and level 0 keys are distinct entries in the same cache |
| `Budgeted for Volume` | Store length, unchanged | `VolumeCache` admission | Grows by the generated bytes. The caller re-inserts to re-admit |
| `sync_ready_bricks`, `RenderGraph::sync_volume` | `ready_bricks`, `cut_ready_brick` | Unchanged, level 0 only | Unchanged. Regression only |
| F-120, F-121, F-122 | This story's levels | Later sprints | Not consumers in S46 |

Units: dims are voxels, spacing millimetres, offsets bytes. Rounding is owned by `downsample_level` alone. The spacing ratio is owned by `append_level` alone. The level geometry convention is owned by `level_voxel_to_level0` alone.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Complete level 0, one level | `generate_pyramid` | Levels 1 to n appended in order. Level 0 bytes and entry equal | Not applicable |
| Complete level 0, one level, one brick | `generate_pyramid` | Returns 1. Store unchanged | Not applicable |
| Level 0 with slice `s` absent | `generate_pyramid` or `downsample_level` | `Err(SliceNotPresent { index: s })` for the first absent `s` | `levels` and store length unchanged |
| Any complete volume | `downsample_level` with dims not strictly coarser | `Err(LevelShape)` | Unchanged, no buffer computed |
| Pyramid present | `generate_pyramid` again | Levels above 0 replaced. Store bytes and entries equal the first call's | Not applicable |
| Pyramid present | Successful `place(n, ..)` | Level 0 written. `levels.len()` is 1. Store length is level 0's | Not applicable |
| Pyramid present | Refused `place` | Unchanged | Pyramid kept, level 0 unchanged |
| `levels.len() == n` | `cut_level_brick(n, ..)`, `level_voxel_to_level0(n, ..)` | `Err(LevelAbsent { index: n })` | Nothing substitutes level 0 |

Invariants: level 0 is offset 0 and full resolution. Every level above 0 is a function of level 0 bytes and dims only. Offsets increase. One store, x fastest inside each level. `BrickGrid::new` is the only edge table.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| One level's computed buffer | `downsample_level`, a local `Vec<u8>` | The call | `append_level` returning the index | A refusal before the buffer returns with nothing allocated. The buffer is dropped after the copy |
| Store bytes above level 0 | `append_level` | The `Volume` value | The `levels` push | `generate_pyramid` truncates first, then appends. A mid-pyramid failure cannot occur after the completeness check, because every later dims value is valid by construction. The plan still tests that the volume is left with a prefix of valid levels, never a gap |
| Level 0 bytes | Assembly and `place` | Offset 0 | `place` returning `Ok` | Untouched by generation |
| `VolumeCache` entry | The caller | `VolumeKey` | The caller's re-insert | Generation never touches the cache |

Peak memory during one level is the store plus one level buffer. There is no device object and no browser setter.

## Numbered acceptance inventory

Every fixture is hand-computed from decision 1 and decision 4 of this plan. No DICOM section specifies a pyramid. The DICOM sections cited are the ones that fix the inputs: PS3.3 C.7.6.3 Pixel Representation for the signed container, C.11.1 for why the stored domain is the modality domain before rounding, C.7.6.2.1.1 for the origin being voxel 0's centre, and C.7.6.2 Pixel Spacing for the in-plane spacing.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `U8` level 0 `[4, 2, 2]`, spacing `[1.0, 2.0, 3.0]`, bytes x fastest `0,1,0,0, 2,3,0,0, 4,5,5,5, 6,7,5,5`. `downsample_level([2, 1, 1])` gives `[4, 3]`: cell A sums 28 over 8, 3.5, rounds to 4. Cell B sums 20 over 8, 2.5, rounds to 3. Spacing is `[2.0, 4.0, 6.0]` | `fixture_even_box_rounds_ties_away_from_zero` | Truncating `/` gives `[3, 2]`. Floor gives `[3, 2]`. Ties to even gives `[4, 2]`. Decimation gives `[0, 0]` | Implementation, `ocelli-volume` |
| AC2 | `I16` level 0 `[2, 2, 2]`, values `-1, -2, -3, -4, -5, -6, -7, 0` as little-endian two's complement, PS3.3 C.7.6.3. `downsample_level([1, 1, 1])` gives `-4` as `FC FF`. Sum -28 over 8 is -3.5 | `fixture_signed_tie_rounds_away_from_zero` | Truncation toward zero gives -3. Ties toward positive infinity gives -3. Decoding as `u16` gives a large positive value | Implementation |
| AC3 | `U8` level 0 `[5, 1, 1]`, `10, 20, 30, 40, 50`. `downsample_level([3, 1, 1])` gives `[14, 30, 46]`: `(3*10 + 2*20) / 5`, `(1*20 + 3*30 + 1*40) / 5`, `(2*40 + 3*50) / 5`. Spacing x is `1.0 * 5.0 / 3.0` by bits | `fixture_odd_axis_uses_exact_overlap` | A 2x box with a short edge cell gives `[15, 35, 50]`. Decimation at `2c` gives `[10, 30, 50]`. Linear sampling at the child centre gives `[13, 30, 47]`. Floor dims gives two voxels | Implementation |
| AC4 | `U8` level 0 `[4, 1, 1]`, `0, 1, 0, 0`. `downsample_level([2, 1, 1])` gives `[1, 0]`. Then `downsample_level([1, 1, 1])` gives `[0]`, because 1 over 4 is 0.25 | `fixture_every_level_is_a_downsample_of_level_zero` | Downsampling level 1 instead gives `[1]`, because 1 over 2 is 0.5 | Implementation |
| AC5 | `U16` `[2, 2, 2]` all `0xFFFF` gives `0xFFFF`. `I16` `[2, 2, 2]` all `-32768` gives `-32768` | `fixture_extremes_do_not_overflow` | A sixteen-bit accumulator wraps | Implementation |
| AC6 | `F32` `[2, 1, 1]` `1.0, 4.0` gives `2.5`. `-3.0, 0.5` gives `-1.25`. Compared by bits | `fixture_float_mean` | Averaging the `u32` bit patterns gives `2.0` for the first pair | Implementation |
| AC7 | `U8` level 0 `[6, 4, 3]`, spacing `[0.5, 0.75, 2.5]`. `downsample_level([3, 2, 2])` has spacing `[1.0, 1.5, 3.75]` by bits | `fixture_spacing_is_the_dimension_ratio` | Doubling spacing gives z `5.0`. Keeping level 0 spacing, the shape of a thickness copy, gives z `2.5` | Implementation, plus review that `levels.rs` writes spacing only in `append_level` |
| AC8 | `U16` level 0 `[130, 64, 64]` constant 1000. `generate_pyramid` returns 3. Dims `[130, 64, 64]`, `[65, 32, 32]`, `[33, 16, 16]`. Every `brick_grid_at(k).edge` is 64. Grids `[3, 1, 1]`, `[2, 1, 1]`, `[1, 1, 1]`. Every level's bytes are 1000. The same dims as `U8` constant 7 return 2, edge 128 | `fixture_pyramid_levels_follow_brick_grid` | A hard-coded edge 128 returns 2 for `U16`. Floor dims gives `[32, 16, 16]` at level 2. Running to one voxel returns 9. Zero-filled levels fail the content check | Implementation |
| AC9 | `U8` `[4, 4, 4]`, one brick. `generate_pyramid` returns 1. `level(1)`, `brick_grid_at(1)`, `cut_level_brick(1, [0, 0, 0])` and `level_voxel_to_level0(1, ..)` all return `LevelAbsent { index: 1 }` | `a_missing_generated_level_is_not_level_zero` | Returning level 0 for a missing index | Implementation |
| AC10 | AC8's `U16` volume. After generation `level_bytes(0)` equals the bytes before and `level(0)` is unchanged. `Budgeted::bytes` is the sum of the three level lengths, `1064960 + 133120 + 16896` | `generation_keeps_level_zero` | Writing a level over level 0. Counting a second copy | Implementation |
| AC11 | `[4, 4, 4]` `U8` with slice 2 never placed. `generate_pyramid` and `downsample_level([2, 2, 2])` return `SliceNotPresent { index: 2 }`. `levels.len()` 1, store length 64 | `incomplete_level_zero_is_refused` | Averaging the cleared slice as zeros | Implementation |
| AC12 | Complete `[4, 4, 4]` `U8`. `downsample_level` with `[4, 4, 4]`, `[0, 2, 2]`, `[5, 2, 2]` returns `LevelShape`. Store unchanged | `a_bad_downsample_shape_leaves_the_store` | Clamping, or dividing by a zero dimension | Implementation |
| AC13 | AC1's volume after `downsample_level([2, 1, 1])`. `cut_level_brick(1, [0, 0, 0])` has extent `[2, 1, 1]`, samples `[4, 3]`, sample `U8`. `cut_level_brick(0, [0, 0, 0])` equals `cut_ready_brick([0, 0, 0])`. One `BrickCache` holds both keys, `level` 0 and 1, as two entries, and `missing(id, &brick_grid_at(1))` is empty after the level 1 insert | `level_bricks_share_one_cache` | Cutting level 1 from the level 0 prefix. A second cache per level. Keying a level 1 brick as level 0 | Implementation |
| AC14 | AC8's `U16` volume. `generate_pyramid` twice. The second call returns 3, and the store bytes and `levels` equal the first call's | `regenerating_is_idempotent` | Appending a second pyramid behind the first | Implementation |
| AC15 | AC1's volume with level 1. A successful `place(0, ..)` leaves `levels.len()` 1 and store length 16. A `place` with a short buffer leaves level 1 in place | `placing_level_zero_drops_stale_levels` | Keeping a level that no longer downsamples level 0. Dropping it on a refused write | Implementation |
| AC16 | AC1's level 1. `level_voxel_to_level0(1, [0, 0, 0])` is `[0.5, 0.5, 0.5]`. `[1, 0, 0]` is `[2.5, 0.5, 0.5]`. AC3's level 1 voxel `[2, 0, 0]` is `2.5 * 5 / 3 - 0.5` within 1e-12. Level 0 maps a voxel to itself | `fixture_level_voxel_centres` | `c * D / d` with no half-voxel term gives `[0, 0, 0]` | Implementation |
| AC17 | Existing level-0 consumers | `tests/levels.rs`, `ocelli-render` `tests/progressive.rs` unchanged and green | A generator that changes level 0 or `sync_ready_bricks` keys | Implementation |

Semantic RED: each fixture is run against the named mutation and must go red, recorded in the progress note. AC1 against truncating `/`, AC2 against ties toward positive infinity, AC3 against the short edge cell, AC4 against downsampling the previous level, AC7 against a doubled spacing, AC8 against edge 128, AC9 against returning level 0, AC16 against the missing half-voxel term. A compile failure before the methods exist is not this RED.

No GPU, browser, corpus or oracle row. The oracle has no pyramid to compare, because cornerstone3D 5.8.2 does not build one.

## Approach

All new code lives in `crates/ocelli-volume/src/levels.rs`. `lib.rs` changes only `place`, `VoxelStore` and the doc comment on `Level` and `levels`.

1. `downsample_level` checks completeness, then `coarser`, then computes the child buffer by iterating child voxels z, y, x and, per axis, the parent range `p` from `c * D div d` to `((c + 1) * D - 1) div d`. Weights are `u64` per axis. Integer kinds sum `i128`, round by decision 4 and narrow with `try_from`. `F32` sums `f64` and narrows with `narrow_f32`. It then calls `append_level(dims, &buffer)`.
2. `generate_pyramid` checks completeness, truncates levels above 0, and loops `k = 1, 2, ..` while the previous level's `brick_grid_at` grid is not `[1, 1, 1]`, calling `downsample_level` with decision 6's dims.
3. `cut_level_brick` delegates level 0 to `cut_ready_brick`. Above 0 it calls `Brick::cut(&brick_grid_at(index)?, brick, level_bytes(index)?, BrickSample::from(self.voxel))` and maps the error through the existing `brick_cut_error`.
4. `level_voxel_to_level0` is three lines per axis over `f64::from(u32)`.

Rejected alternatives:

- A 2x box with a short edge cell. Its footprint disagrees with the mandated dimension-ratio spacing on an odd axis by up to half a level 0 voxel, which is AC3's mutation.
- A cascade from the previous level. Faster and not a downsample of level 0, which is AC4's mutation.
- Decimation, or nearest or linear sampling at the child centre. Aliasing in the first case and not a box average in either.
- A per-level `Vec` or a second store. Section 19 asks for one allocation.
- A `PyramidCache` or a per-level `BrickCache`. Section 30 says no second cache.
- A filter trait. One implementer today, so the structural rule refuses it.
- A spacing computation in the generator. It would be the second spacing formula the sprint's defect class names.

## Dependencies, write ownership and resources

Declared dependency F-058 is done. Source dependencies: F-036's `BrickGrid` and `Brick::cut`, F-043's `sync_ready_bricks`, both read only.

Write set:

- `crates/ocelli-volume/src/levels.rs`
- `crates/ocelli-volume/src/lib.rs`, `place`, `VoxelStore` truncate, doc comments
- `crates/ocelli-volume/src/ortho.rs`, `narrow_f32` visibility only
- `crates/ocelli-volume/tests/pyramid.rs`, new
- `docs/lld/volumes.md`, at `/complete-feature`

Not written: `ocelli-cache`, `ocelli-render`, shaders, `ocelli-wasm`, the examples. F-208 to F-215 write `examples/folder-viewer` and `ocelli-wasm` or the example server, so they share no file with this story and can run in parallel.

**Sprint build order, decided in the S46 design round on 2026-11-05.** F-217, then F-216, then F-210, F-211, F-208, F-209, F-212, F-213, F-214 and F-215. F-119 is outside that chain. It writes only `ocelli-volume` and its LLD, and it may run in parallel with any of them. F-210 reads the modality stage and does not touch `ocelli-volume`. No GPU or browser resource.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-volume
bin/ocelli.sh test ocelli-volume --test pyramid
bin/ocelli.sh test ocelli-volume --test levels
bin/ocelli.sh test ocelli-render --test progressive
bin/ocelli.sh clippy ocelli-volume
bin/ocelli.sh native
```

`native` is the cross-target proof that `ocelli-volume` still builds for both targets. The sprint profile runs at sprint verification. Evidence is invalidated by any change to the filter, the rounding rule, the dims rule, the stop rule or `place`. There is no performance claim. Generation on a 512 by 512 by 600 `I16` series reads level 0 once per level, four levels at edge 64, and `tools/bench/subjects.json` has no subject for it. A load-time budget is Q12.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. Levels are host bytes in the worker's store
- Render-loop allocation: none. Generation and `cut_level_brick` are load-time calls. The level buffer is a local `Vec` per level
- unsafe: none
- Tier A (WebGPU): full for generation, which is host CPU work and tier-independent. Uploading a level above 0 is F-120 and F-122
- Tier B (WebGL2): full for generation, the same bytes. Bricks stay at most 128 on an edge, under the tier B 3D floor of 256
- Tier C (CPU): full for generation, the same bytes. `sync_ready_bricks` still reports `Unavailable` on tier C, and no tier C consumer reads a coarse level in this story

Generation produces the same bytes on every tier because no tier participates. Later GPU use: F-120 selects a level per frame from `Level.spacing` and `level_voxel_to_level0`, F-121 corrects opacity per level step, F-122 budgets and evicts `BrickKey { level: k }` entries in the one `BrickCache`. Each of those declares its own three rows.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 to AC6, filter, domain and rounding, hand-computed, citing PS3.3 C.7.6.3 and C.11.1 | `crates/ocelli-volume/tests/pyramid.rs` |
| fixture | AC7 spacing ratio, AC16 level centres, citing C.7.6.2.1.1 and C.7.6.2 | `crates/ocelli-volume/tests/pyramid.rs` |
| fixture | AC8 dims, stop rule and edge from `BrickGrid::new` | `crates/ocelli-volume/tests/pyramid.rs` |
| unit | AC9 to AC15 refusals, idempotence, staleness, one cache | `crates/ocelli-volume/tests/pyramid.rs` |
| unit | AC17 regression | existing `tests/levels.rs`, `ocelli-render/tests/progressive.rs` |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no pyramid or streaming row, and cornerstone3D 5.8.2 does not build a pyramid. Section 30 is a Part III capability, not parity.

## Deviations

None. The filter, rounding, stop rule and level geometry are gaps the HLD does not cover, not departures from it. D-24 and D-25 are unchanged.

## LLD impact

`docs/lld/volumes.md`, section "The level axis": generation, the filter, the stored domain, the rounding, the dims and stop rules, the level centre mapping, `place` dropping levels above 0, and `cut_level_brick`. `docs/lld/cache.md` gains one line that a level above 0 is a `BrickKey.level` in the same `BrickCache`.

## User-facing documentation impact

No change to `README.md`, `examples/folder-viewer/src/use.html` or `examples/folder-viewer/src/more.html`. Under Q10 nothing in production calls `generate_pyramid` this sprint, so no page, control or command can reach a coarse level. Chapter 12 of `use.html`, Cache and volume, and the Volume rendering section of `more.html` describe behaviour a caller can reach, and both stay true. F-120, the first consumer, updates them when a level becomes reachable.

## Open questions

Each has a recommended answer, used by every AC above. The operator's answer is recorded here when given.

| Q | Decision | Options | Recommended | ACs | Consequence of deferral |
|---|----------|---------|-------------|-----|-------------------------|
| Q1 | Downsample filter | (a) exact-overlap box from level 0. (b) 2x box, short edge cell averages fewer voxels. (c) decimation | (a). Equals the 2x2x2 box on even axes and is the only one consistent with the mandated dimension-ratio spacing on an odd axis | AC1, AC3, AC16 | AC3's numbers change |
| Q2 | Integer rounding | (a) nearest, ties away from zero. (b) nearest, ties to even. (c) floor | (a). Symmetric in sign, so a signed CT box is not biased toward air | AC1, AC2 | AC1 and AC2 numbers change |
| Q3 | Value domain | (a) stored container values. (b) modality values, which the volume does not hold | (a). Exact for rescale, PS3.3 C.11.1, before rounding. For a Modality LUT Sequence the mean does not commute, and the plan records that rather than refusing, because the volume carries no modality stage to check | AC1 to AC6 | A modality-domain answer needs the volume to carry the modality stage, which is a new field |
| Q4 | Which axes halve | (a) every axis with a dimension above 1. (b) anisotropy-aware, halving only axes whose spacing is within 2x of the finest, as Neuroglancer's downsampler does | (a) for this story. (b) changes level count and dims, and F-120's level selection would then need per-axis level choice | AC8 | AC8's dims change |
| Q5 | Stop rule | (a) after the first one-brick level. (b) down to one voxel | (a). A level below one brick saves no residency | AC8, AC9 | AC8's count changes |
| Q6 | Direct or cascade | (a) every level from level 0. (b) from the level above | (a). (b) is the sprint's first defect class. Cost is one extra read of level 0 per level, unmeasured | AC4 | AC4 is removed |
| Q7 | `place` after a pyramid | (a) drop levels above 0. (b) refuse the write. (c) keep stale levels | (a). (c) is a quietly wrong voxel. (b) breaks progressive replacement, which `place` documents as idempotent | AC15 | A stale level is reachable |
| Q8 | `generate_pyramid` with levels above 0 present | (a) replace them. (b) refuse | (a). The result is a function of level 0, so replacing is idempotent. It does discard caller-appended levels, which only F-143 writes and it does not generate | AC14 | AC14 becomes a refusal |
| Q9 | Level geometry helper here or in F-120 | (a) `level_voxel_to_level0` in this story. (b) F-120 derives it | (a). The convention belongs to whoever chose the filter, and leaving it to F-120 is how a half-voxel shift appears | AC16 | F-120 owns a convention it did not choose |
| Q10 | Who calls `generate_pyramid` | (a) nobody in production this sprint. (b) `construct`. (c) the wasm and native open path | (a). F-120 is the first consumer and decides when. (b) would grow every volume by about a seventh before anything reads a coarse level | none | The pyramid is reachable only from tests until F-120 |
| Q11 | Pixel Padding Value | (a) out of scope, padding averages into tissue at coarse levels. (b) exclude padding from the mean | (a), recorded as a non-goal. (b) needs the padding value on the volume, which it does not carry | none | Coarse edges of padded CT read darker than tissue |
| Q12 | Load-time budget | (a) none, unmeasured. (b) add a bench subject with a release-build threshold | (a) for the design round. The operator may want (b) before F-120 makes generation part of opening a series | none | No performance claim is made |

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead. The operator accepted the recommended answer to every question in this plan.

- Q1. Answered: recommended (a), the exact-overlap box filter from level 0.
- Q2. Answered: recommended (a), nearest with ties away from zero.
- Q3. Answered: recommended (a), stored container values, with the Modality LUT Sequence non-commutation recorded and not refused.
- Q4. Answered: recommended (a), every axis with a dimension above 1 halves. Anisotropy-aware halving is left to a later story with F-120's level selection.
- Q5. Answered: recommended (a), stop after the first one-brick level.
- Q6. Answered: recommended (a), every level generated directly from level 0.
- Q7. Answered: recommended (a), `place` drops every level above 0.
- Q8. Answered: recommended (a), `generate_pyramid` replaces levels above 0.
- Q9. Answered: recommended (a), `level_voxel_to_level0` is written in this story.
- Q10. Answered: recommended (a), nobody calls `generate_pyramid` in production this sprint. F-120 decides when.
- Q11. Answered: recommended (a), Pixel Padding Value is out of scope and recorded as a non-goal.
- Q12. Answered: recommended (a), no load-time budget for this story. No performance claim is made.

No question is left open.
