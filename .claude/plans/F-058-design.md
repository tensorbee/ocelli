# F-058, Multiscale pyramid hooks in the volume model

**Status**: approved
**Epic ref**: E8.8
**Sprint**: S19
**Estimate**: 3w

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

The level-axis bullet, quoted as a table cell so the author's semicolon stays data:

|  |
|----|
| **Bricks and a level axis, from the start.** The volume carries a multiscale level axis and 128³ brick decomposition even when a series fits comfortably. Phase 1 uploads every brick, §30 makes residency selective. Retrofitting a level axis into a volume model that never had one is the rewrite cornerstone3D cannot afford. |

`Level` in the current tree already has `dims`, `spacing` and `offset`. `Volume::assemble` writes one entry at offset 0. `SlicePresence` is deviation D-25 and stays the presence set.

### `docs/hld/27-phase1-hooks.md`, section 38

| Hook | Story | Now | Later |
|------|-------|-----|-------|
| Multiscale level axis on the volume | E8.8 | 3 wk | Changing the type every viewport and tool reads |

### `docs/hld/26-differentiating-capabilities.md`, section 30

Bricks are 128³, or 64³ for sixteen-bit data, addressed through a 3D indirection texture. The cache is `ocelli-cache` with a brick key. No second cache.

Opacity correction when the sample count changes per level:

```text
// alpha' = 1 - (1 - alpha) ^ (ds / ds_ref)
fn correct_opacity(alpha: f32, ds: f32, ds_ref: f32) -> f32 {
    1.0 - (1.0 - alpha).powf(ds / ds_ref)
}
```

That function is E30.3, F-121. This story does not implement it.

### Allocation note

`docs/sprints/allocation.json` marks F-058 `ARCHITECTURE HOOK - E30 needs a level axis to exist from the start`. F-119 (E30.1, S42) is brick decomposition and multiscale pyramid generation and depends on F-058. This story does not generate a pyramid.

### What already exists

`crates/ocelli-volume/src/lib.rs` holds `Level` and a one-entry `levels` vector. `Volume::brick_grid` builds a level-0 `BrickGrid`. `BrickKey.level` exists and phase 1 stores 0. `Budgeted::bytes` is the store length. The level axis is not counted a second time.

## What the specification does not cover

Section 19 names the vector and says level 0 is full resolution. It does not name a downsample kernel, a spacing formula for a coarser entry, or a function that appends one. These choices follow from the hook and from F-119 owning generation.

1. **The hook stores a caller-supplied level. It does not downsample.** `Volume::append_level` copies bytes the caller already holds. Averaging, skipping and any other reduction stay in F-119.
2. **Coarser bytes extend the same allocation.** Level 0 stays at offset 0. The new entry's `offset` is the store length before the copy. Slice `n` of level 0 still starts at `n * dims[0] * dims[1] * bytes_per_voxel`. There is still no per-slice `Vec`.
3. **Spacing scales by the dimension ratio.** For each axis, `spacing_level[i] = spacing_0[i] * (dims_0[i] as f64 / dims_level[i] as f64)`. This is a label on the entry, not a claim that the bytes were produced by that scale.
4. **A missing level is an error.** `Volume::level` and `Volume::brick_grid_at` return `LevelAbsent` for an index the vector does not hold. They do not return level 0 in its place.
5. **Presence stays level 0.** `SlicePresence` is per slice of the full-resolution volume. An appended level is complete caller data and has no presence bits.
6. **No renderer reads level > 0.** Upload, reslice and ray cast in this sprint stay on level 0. F-120 selects a level later.

## Problem and reproducer

A reader can see `levels: Vec<Level>` and still treat the volume as one resolution. `Volume::brick_grid` always returns level 0. Nothing appends an entry, and nothing refuses a request for a level that is not there. E30 would then special-case a one-entry vector, which is the retrofit section 38 exists to avoid.

Expected observation: after `append_level` of a strictly coarser grid, `levels.len()` is 2, level 0's bytes and offset are unchanged, the new entry's offset is the old store length, its spacing follows the ratio above, and `brick_grid_at(1)` carries `level == 1`. A request for level 2 returns `LevelAbsent`. `Budgeted::bytes` grows by the appended byte length and not by a second copy of level 0.

Current behaviour: `append_level` does not exist. `levels.len()` stays 1 for every assembled volume.

Source identity: `crates/ocelli-volume/src/lib.rs` `Level`, `Volume::assemble` and `Volume::brick_grid`.

## Scope, API and compatibility

In scope:

- `Volume::append_level`, `Volume::level`, `Volume::brick_grid_at` in `ocelli-volume`.
- One new error variant, `VolumeError::LevelAbsent { index: u32 }`.
- One new error variant, `VolumeError::LevelShape`, used when the dims or the byte length are refused.
- Growth of `VoxelStore` by an exact copy, after the checks.

Out of scope:

- A downsample or upsample kernel.
- Opacity correction from section 30.1.
- Uploading a coarser brick, selecting a LOD, prefetch, eviction and 4D. Those are F-119 through F-124.
- Changing level 0's edge rule. Eight-bit stays 128. Sixteen-bit and float stay 64. That match still lives in `BrickGrid::new`.
- wasm exports, a viewport field, or a change to `SlicePresence`.

```rust
impl Volume {
    /// Level `index`, or [`VolumeError::LevelAbsent`].
    pub fn level(&self, index: u32) -> Result<&Level, VolumeError>;

    /// Append one coarser level. Level 0 is unchanged.
    ///
    /// `samples` is copied to the end of the same store. The new spacing is
    /// level 0's spacing times `dims_0[i] / dims[i]` on each axis.
    pub fn append_level(
        &mut self,
        dims: [u32; 3],
        samples: &[u8],
    ) -> Result<u32, VolumeError>;

    /// The brick grid of one stored level. [`BrickGrid::level`] is that index.
    pub fn brick_grid_at(&self, index: u32) -> Result<BrickGrid, VolumeError>;
}
```

`Volume::brick_grid` stays the level-0 call. Existing callers keep their result.

Refusal leaves the volume unchanged. A short buffer, a zero axis, an axis larger than level 0, or a grid that is not strictly smaller on at least one axis returns `LevelShape` before any byte is copied.

Defaults: assembly still writes exactly one level. Nothing in `construct` or `place` appends another.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `Volume::append_level` | `VoxelStore` growth and `Level` | A later generator, or a test. No downsample here | New level index, or `LevelShape` with the store unchanged |
| `Volume::level` | `levels` vector | Any reader | The entry, or `LevelAbsent` |
| `Volume::brick_grid` | `BrickGrid::new` on level 0 | F-033 and F-043 callers | Unchanged level-0 grid |
| `Volume::brick_grid_at` | `BrickGrid::new` then `level` field set | E30 readers | Grid whose `level` equals the index, or `LevelAbsent` |
| `Volume::ready_bricks` | level-0 grid and `SlicePresence` | Progressive upload | Still level 0 only |
| `Budgeted for Volume` | store length | Cache admission | Grows only when an append commits |
| Renderer brick upload | not a consumer of level > 0 | This sprint | No new texture |

Units: `dims` are voxels. `spacing` is millimetres. `offset` is a byte index into the one store. Rounding of the spacing ratio is IEEE-754 `f64` division, with no quantisation step.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| One level, the assembled volume | `append_level` with a strictly smaller grid and a matching buffer | `levels.len()` grows by 1. Store gains that many bytes at the previous length. Level 0 bytes compare equal | Not applicable |
| One or more levels | `append_level` with a bad shape or a short buffer | `Err(LevelShape)` | `levels`, store length and level 0 bytes unchanged |
| `levels.len() == n` | `level(n)` or `brick_grid_at(n)` | `Err(LevelAbsent { index: n })` | No insert of a synthetic level 0 |
| Any volume | `place` or `construct` | Level 0 presence and bytes change as they do today | `append_level` is not called |

Invariants: one store, x fastest inside each level. Level 0's slice formula is unchanged. Level `k` starts at `levels[k].offset` and occupies `product(dims) * bytes_per_voxel` bytes. Offsets increase. A failed append does not leave a hole.

There is no cache of a rendered level. Appending twice with the same bytes creates two entries and two copies. That is a second level, not a no-op, because the caller asked for another entry.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Extra store bytes | `append_level` after the checks pass | The `Volume` value | Return of the new index | `LevelShape` allocates nothing and copies nothing |
| Level 0 bytes | Assembly, already owned | Offset 0 for the life of the value | Unchanged by append | A later `place` still writes level 0 only |
| `VolumeCache` | Not acquired | The caller's key, if they inserted earlier | None in this function | The caller re-inserts if admission must see the new size |

The copy is the only side effect, and it happens after validation. There is no device object and no browser setter.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Level 0 is `[4, 4, 4]`, spacing `[1.0, 2.0, 3.0]`, `U8`, filled with `0x11`. Append `[2, 2, 2]` of eight bytes `1, 2, 3, 4, 5, 6, 7, 8`. New index is 1. Spacing is `[2.0, 4.0, 6.0]`. Offset equals the old length `64`. Level 0's first byte is still `0x11`. `Budgeted::bytes` is 72 | `append_level_keeps_level_zero_and_scales_spacing` | Writing the coarse bytes over level 0. Leaving spacing equal to level 0. Counting the level vector as a second budget | Implementation. `ocelli-volume` test |
| AC2 | The AC1 volume. `brick_grid_at(0)` matches `brick_grid` and has `level == 0`. `brick_grid_at(1)` has dims `[2, 2, 2]` and `level == 1`. `level(2)` and `brick_grid_at(2)` return `LevelAbsent { index: 2 }` | `a_missing_level_is_not_level_zero` | Returning level 0 for every index | Implementation. `ocelli-volume` test |
| AC3 | Append of dims `[4, 4, 4]`, `[0, 2, 2]`, `[5, 2, 2]`, and a `[2, 2, 2]` buffer that is one byte short | Each call returns `LevelShape`. `levels.len()` stays 1. Store length stays 64 | Truncating, padding with zeros, or clamping a larger axis | Implementation. `ocelli-volume` test |
| AC4 | `construct` of a two-slice stack, then no append | `levels.len()` is 1, offset 0, dims equal `Volume.dims` | Assembly starting at two levels, or `construct` dropping the vector | Implementation. Existing construct tests plus one assertion |
| AC5 | Sixteen-bit level 0 `[4, 4, 4]`. Append `[2, 1, 2]` with `2 * 1 * 2 * 2` bytes. `brick_grid_at(1).edge` is 64, from `BrickWidth::SixteenBit`, and `level` is 1 | `appended_sixteen_bit_level_keeps_the_brick_edge` | Hard-coding edge 128 on the coarse level | Implementation. `ocelli-volume` test |

No GPU, browser, corpus or oracle row. A missing pyramid consumer is F-119, not an open measurement here.

Semantic RED before the method exists is a compile failure, which is not the behaviour claim. The behaviour RED is a stand-in that returns level 0 for index 1. AC2 must go red under that stand-in and green when `LevelAbsent` is returned. Mutation: scale spacing by `dims_level / dims_0` instead of the inverse, and AC1 expects `[2, 4, 6]` but observes `[0.5, 1.0, 1.5]`.

## Approach

Add `crates/ocelli-volume/src/levels.rs` and call it from `impl Volume` in that module. `lib.rs` gains `mod levels` and the two error variants.

`append_level` checks shape and length, records the offset, extends the store, then pushes `Level`. The extend method on `VoxelStore` is `pub(crate)` and refuses to run with a length that was not checked. Failure returns before that call.

`brick_grid_at` builds `BrickGrid::new(level.dims, BrickWidth::from(self.voxel))` and assigns the public `level` field. It does not add a second edge table.

Rejected alternatives:

- Generate the coarse bytes by box filtering. That is F-119's kernel, and a silent average here becomes the pyramid E30 cannot replace.
- A parallel `Vec<u8>` per level. Section 19 asks for one allocation and an offset.
- Leave the vector at one entry and document the hook in a comment. The type every later reader uses would still have no way to name level 1.

## Dependencies, write ownership and resources

Depends on F-051, which is `done`. No other S19 story is a backlog dependency.

This story writes:

- `crates/ocelli-volume/src/lib.rs`
- `crates/ocelli-volume/src/levels.rs`
- `crates/ocelli-volume/tests/levels.rs`
- `docs/lld/volumes.md` at completion

It does not write `ocelli-render`, shaders, `CURRENT_SPRINT.md` or the other ledgers until `/complete-feature`. It does not use the GPU.

Serial in the canonical worktree. F-057, F-052 and F-059 also take `lib.rs` or the render crate that will depend on this volume, so they follow this story. They read level 0 through the existing slice layout and must not assume `levels.len() == 1` as a panic condition.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-volume
bin/ocelli.sh test ocelli-volume --test levels
bin/ocelli.sh clippy ocelli-volume
```

Completion of the story later runs the sprint profile over the integrated tree. This plan does not record a verification.

Evidence is invalidated by an edit to the spacing ratio, the offset rule, or the store layout. AC1's numbers are the baseline. There is no performance threshold.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Append is a load-time copy
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The bytes are host data. No tier renders them in this story, and no tier substitutes a different grid.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 spacing ratio and unchanged level-0 byte, hand-computed from the dimension ratio | `tests/levels.rs` |
| fixture | AC5 brick edge stays the sixteen-bit edge from `BrickGrid::new` | `tests/levels.rs` |
| unit | AC2, AC3, AC4 refusals and the one-entry assembly default | `tests/levels.rs` |

No pixel LUT arithmetic. The spacing ratio is geometry metadata, not a resampled intensity.

## Parity surface covered

None. Appendix B has no pyramid row. Viewport types and blend modes are other stories.

## Deviations

None. D-25 stays as it is. The presence field is not changed.

## LLD impact

`docs/lld/volumes.md`. Replace the sentence that this story holds exactly one level. Describe append, the spacing ratio, `LevelAbsent`, and that renderers in S19 still upload level 0 only.

## Open questions

None. Downsample policy stays with F-119. Opacity correction stays with F-121. Both are already scheduled, so they are non-goals rather than unanswered questions.
