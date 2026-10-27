# F-051, Volume construction from a stack: geometry, orientation, spacing

**Status**: approved
**Epic ref**: E8.1
**Sprint**: S18
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

The three bullets, quoted as table cells so the author's semicolon stays data:

|  |
|----|
| **Layout.** One contiguous allocation, x fastest then y then z. Slice n starts at n \* dims\[0\] \* dims\[1\] \* bytes_per_voxel. Do not introduce a per-slice Vec, the whole point is a single upload region. |
| **Progressive assembly.** Clear the 3D texture at creation and upload slices as they land. present drives the loading indicator and tells the oracle which frames are comparable yet. |
| **Bricks and a level axis, from the start.** The volume carries a multiscale level axis and 128³ brick decomposition even when a series fits comfortably. Phase 1 uploads every brick, §30 makes residency selective. Retrofitting a level axis into a volume model that never had one is the rewrite cornerstone3D cannot afford. |

`origin` is commented "IPP of the first slice". This sprint's done-means text names that slice as the lowest projected one, and the point is the centre of voxel zero.

### `docs/hld/03-architecture-and-crates.md`, section 4

```text
| ocelli-volume | Volume assembly, geometry, reslicing | yes | yes |
```

### DICOM PS3.3 C.7.6.2.1.1

With `X = IOP[0..3]` the row direction, `Y = IOP[3..6]` the column direction, `i` the column index and `j` the row index:

```text
P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
```

`PixelSpacing` is `[between rows, between columns]`. `PixelSpacing[0]` multiplies the column direction cosine. `PixelSpacing[1]` multiplies the row direction cosine. The slice normal is `N = X cross Y`.

Image Position Patient is the centre of the first voxel, not a corner.

The inter-slice step is the difference of consecutive positions projected on `N`, `dot(IPP, N)`. Spacing Between Slices, Slice Thickness, Slice Location, Instance Number and Gantry/Detector Tilt are not that step.

### What already implements that text

F-033 owns `Volume::assemble`, `group_volumes`, `VoxelStore`, `SlicePresence` (D-25) and `VolumeCache`. Assembly sorts by `position_vector().dot(slice_normal())`, sets `spacing[0]` to column millimetres and `spacing[1]` to row millimetres, copies the lowest projected Image Position Patient into `origin`, and refuses shear, duplicates and non-uniform gaps. The store is cleared. `order[n]` is the arrival index of sorted position `n`. `place` is the only slice write, and no production caller uses it. `docs/sprints/AS_BUILT.md` records F-051 as that caller.

`ImagePlane::slice_normal` and `ImagePlane::index_to_world` in `ocelli-pixel` are the one copy of the cross product and the in-plane equation. This story does not write a second one.

## What the specification does not cover

Section 19 names the struct and the layout. It does not name a function that copies stored samples into the cleared store. These are the choices this plan makes.

1. **The caller is `Volume::construct`.** It takes one `VolumeCandidate` by value, borrows it for the existing assembly checks, then calls `place` once per sorted slice with that slice's stored sample bytes. It returns the filled volume only when every place succeeds.
2. **`Volume::assemble` stays a cleared store.** Progressive fill remains a later caller's loop over `place`. Folding samples into `assemble` would make F-033's cleared-store tests a lie and would hide the write path this story exists to own.
3. **Samples stay stored values.** No modality LUT and no VOI LUT runs. The sprint forbids reopening the LUT chain. `place` copies bytes.
4. **A sheared stack is refused, not stored.** Section 19's `direction` is derived from Image Orientation Patient, whose third column is the normal. A single spacing triple cannot carry a lateral step without pretending the stack is a box. F-033 already returns `ShearedVolume`. This story propagates that error and does not drop the lateral component.
5. **`StackGeometry` in `ocelli-dicom` is not called.** It walks frames in stored order and uses a 1e-6 mm step tolerance. `assemble` sorts by projection and uses the 0.5 percent median rule. A second call would be a second geometry answer.
6. **`construct` does not insert into `VolumeCache`.** Admission stays the caller's decision. An error drops the local volume, so a failed construct publishes nothing.
7. **Orthographic reslice, pyramid levels beyond the one F-033 entry, brick upload and ray casting stay out.** Those are F-052, F-058, F-057 and F-059.

## Problem and reproducer

`Volume::assemble` returns geometry and a zeroed `VoxelStore`. A caller that places frames in arrival order, or that never calls `place`, builds a volume whose axial pictures can look fine while the oblique order or the voxel bytes are wrong.

Today no production function calls `place`. The crate tests do, with bytes the test invents after assembly. A stack's own samples never move.

Expected observation after this story: one candidate whose frames arrive out of projected order becomes one volume whose slice 0 holds the lowest projected frame's stored bytes, whose origin equals that frame's Image Position Patient exactly, and whose spacing triple is column millimetres, row millimetres, then the median projected gap. A sheared candidate returns `ShearedVolume` and no volume.

Current behaviour: `Volume::construct` does not exist. `assemble` leaves every slice cleared and `present_count` at 0.

Source identity: `crates/ocelli-volume/src/lib.rs` `Volume::assemble` and `Volume::place`.

## Scope, API and compatibility

In scope:

- `Volume::construct(candidate: VolumeCandidate) -> Result<Volume, VolumeError>` in `ocelli-volume`.
- A private borrow of the candidate so assembly can run without cloning sample buffers. The public `assemble` signature stays by value and still drops samples on return.
- Fixtures that judge projected order, spacing index order, origin, shear refusal and the cleared-store contract of `assemble`.

Out of scope:

- Parsing DICOM tags. The input is already a `VolumeCandidate` of `DecodedFrame` values.
- Changing the 0.5 percent gap rule, the 1e-9 mm duplicate bound, or the 1e-6 mm shear bound.
- Reslice, viewport, wasm exports, GPU upload, cache policy, extra pyramid levels.
- A new `VolumeError` variant. A broken `order` index uses the existing `SliceOutOfRange` rather than a panic. Assembly does not produce that index.

Public signature:

```rust
impl Volume {
    /// Fill one candidate.
    ///
    /// Geometry, orientation and spacing come from [`Self::assemble`].
    /// Each slice's stored samples are written by [`Self::place`] at the
    /// sorted index `order` names. The returned volume is complete.
    ///
    /// # Errors
    ///
    /// Every [`VolumeError`] assembly already returns, plus
    /// [`VolumeError::SliceSizeMismatch`] when a frame's sample buffer is
    /// not one slice. A failure returns no volume.
    pub fn construct(candidate: VolumeCandidate) -> Result<Self, VolumeError>;
}
```

`assemble` remains available. Existing tests keep calling it. `construct` is additive.

Defaults: no resampling, no rescale, no half-voxel origin shift, no tag fallback when a plane is missing. A candidate that cannot assemble does not get a substitute spacing.

Refusal: the same typed errors as `assemble`, and `SliceSizeMismatch` from `place`. Mixed orientations, inconsistent dimensions, inconsistent in-plane spacing, unsupported voxel kinds, duplicates, non-uniform gaps, shear and fewer than two slices all refuse before a volume is returned.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `Volume::construct` | `assemble`'s checks, then `Volume::place` | Load path. Stored samples. No LUT. No cache write | `Ok(Volume)` with every slice present, or `Err(VolumeError)` and no volume |
| `Volume::assemble` | unchanged geometry | Callers that still want a cleared store, including F-033 tests and a later progressive loop | Cleared store, `present_count == 0`, `order` filled |
| `Volume::place` | `VoxelStore::slice_mut` | The only byte write. Length checked before the copy | One slice replaced, presence bit set once |
| `VolumeCache::insert` | unchanged | Not called here | Cache bytes unchanged by `construct` |
| `StackGeometry::derive` | unchanged | Not called here | No second spacing |
| `ImagePlane::index_to_world` | unchanged | Not called here | In-plane equation stays in `ocelli-pixel` |
| Stack viewport placement | unchanged | Does not call `assemble` or `construct` | F-052 owns reslice |

Units: spacing millimetres. Index 0 steps along the row cosine by `PixelSpacing[1]`. Index 1 steps along the column cosine by `PixelSpacing[0]`. Index 2 is the median projected gap in millimetres. Origin is LPS millimetres, copied, not rounded.

Rounding: none on origin. Gap comparisons stay F-033's. This story adds no new tolerance.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| No volume | `construct` on a uniform axis-aligned candidate of at least two slices | One new `Volume`. Store holds each frame's bytes at the sorted index. `present_count == dims[2]`. `is_complete` is true | Not a mutation of a previous volume |
| No volume | `construct` on shear, non-uniform gaps, a duplicate, one slice, mixed orientation, bad dimensions, bad spacing, a multi-sample layout, or an unsupported kind | `Err` of the assembly variant. No volume | Nothing inserted anywhere |
| No volume | `construct` whose geometry is valid and one sample buffer has the wrong length | `Err(SliceSizeMismatch)`. The partial store is dropped | No volume. Cache untouched |
| Cleared volume from `assemble` | unchanged call | Store still zero. `present_count == 0` | `construct` must not change this result |

Invariants that span consumers: one contiguous store, x fastest. Slice `n` starts at `n * dims[0] * dims[1] * bytes_per_voxel`. `order` still names arrival indexes. Presence counts a repeated `place` once, which `construct` does not repeat. `construct` does not allocate a per-slice `Vec` inside `Volume`.

This is a pure constructor. There is no cancel or retry token. A second `construct` on an equal candidate returns an equal volume and does not alias the first allocation.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| `VoxelStore` bytes | `assemble`'s existing `VoxelStore::new` | The `Volume` value returned by `construct` | `Ok(volume)` after every `place` | Any `Err` drops the local value. No cache key changes |
| Candidate sample buffers | The caller, already inside `DecodedFrame` | Owned by the candidate until `construct` returns | Copied by `place` into the store | Not moved out of the volume on failure, because the volume is dropped |
| `VolumeCache` | Not acquired | Unchanged | None | A caller may insert only the `Ok` value |

Side effects before the commit point are the local allocation and the copies into it. Nothing is published earlier. An identical second call allocates again and returns an equal value. It is not a no-op on the first volume.

Validity of a constructed volume is the candidate's planes, stored sample bytes, and F-033's uniformity and shear checks. `construct` does not cache a settled render. There is no idle return.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Axial 2 by 2 unsigned 8-bit stack. Row spacing 0.5 mm, column spacing 1.0 mm. Arrival order IPP z = 6, 0, 3 with samples `[1,1,1,1]`, `[2,2,2,2]`, `[3,3,3,3]`. Sorted slices are `[2,2,2,2]`, `[3,3,3,3]`, `[1,1,1,1]`. Origin is `(0, 0, 0)` exactly. `spacing` is `[1.0, 0.5, 3.0]`. Direction columns are row `(1,0,0)`, column `(0,1,0)`, normal `(0,0,1)`. `present_count` is 3 and `is_complete` is true. PS3.3 C.7.6.2.1.1 | `construct_places_samples_in_projected_order` | Arrival-order placement. Cleared store. Swapped spacing indexes. Half-voxel origin. A rescale of the stored bytes | Implementation. `ocelli-volume` test |
| AC2 | Oblique stack. Row cosine `(1,0,0)`, column cosine `(0,0,1)`, so the normal is `(0,-1,0)`. Arrivals: IPP `(0,0,10)` samples `[4,4,4,4]` projection 0, IPP `(0,-3,0)` samples `[5,5,5,5]` projection 3, IPP `(0,-6,5)` samples `[6,6,6,6]` projection 6. Slice 0 is `[4,4,4,4]`. Origin is `(0, 0, 10)` exactly, not the lowest z. `spacing[2]` is 3.0. Direction column 2 is `(0,-1,0)` | `construct_sorts_oblique_stack_by_projection` | Sort by the z component or by arrival index | Implementation. `ocelli-volume` test |
| AC3 | Axial positions 0, 3, 6 mm on the normal. `spacing[2]` is 3.0. The frames carry no Spacing Between Slices value a reader could prefer | `construct_spacing_is_the_projected_gap` | A hard-coded 5.0 standing in for a tag, or the mean of a later non-uniform set | Implementation. `ocelli-volume` test |
| AC4 | Two axial slices whose second IPP is `(5, 0, 3)` against a first at the origin. Lateral step is 5 mm | `construct_refuses_shear` returns `Err(ShearedVolume { .. })` | Dropping the lateral component and returning an axis-aligned box with gap 3 | Implementation. `ocelli-volume` test |
| AC5 | Projected positions 0, 3, 6, 13, 16. Gaps 3, 3, 7, 3. Median 3, worst 7, index 2 | `construct_refuses_non_uniform_spacing` | Averaging the gaps into one spacing, or using the mean so the reported index moves | Implementation. `ocelli-volume` test |
| AC6 | A repeated projected position, and a one-slice candidate | `construct_refuses_duplicate_and_singleton` returns `DuplicatePosition` and `TooFewSlices` | Inventing a spacing for one slice, or keeping both duplicates | Implementation. `ocelli-volume` test |
| AC7 | Geometry that would assemble, one frame three bytes long on a four-byte slice | `construct_refuses_a_short_slice` returns `SliceSizeMismatch` and no volume | Padding, truncating, or publishing the slices that did fit | Implementation. `ocelli-volume` test |
| AC8 | The AC1 candidate passed to `assemble` instead of `construct` | `assemble_stays_cleared_when_construct_fills`. Store slices are zeros. `present_count` is 0. The same bytes through `construct` match AC1 | Moving `place` inside `assemble` | Implementation. `ocelli-volume` test |
| AC9 | Two RGB frames, three samples per pixel, buffer length 36, same shape as F-033's multi-sample fixture | `construct_refuses_a_multi_sample_layout` returns `InconsistentSpacing` | Placing the buffer and returning `SliceSizeMismatch`, or returning `Ok` | Implementation. `ocelli-volume` test |

No external host, adapter, browser or oracle view is required. The story does not draw. A missing renderer is F-052 and later, not an open measurement on this inventory.

Semantic RED before the fix: `construct` may exist only as `assemble`'s cleared volume. AC1 then sees zeros where the fixture requires `[2,2,2,2]`. A compile error for a missing function is not that RED. After the fix the same inputs are GREEN. Mutation: place arrival index `n` at slice `n` instead of `order[n]`, and AC1's slice 0 becomes `[1,1,1,1]`. Restore the source.

## Approach

Keep F-033's geometry function as the only projection, sort, median, duplicate and shear implementation. Split its body so a private `assemble_from(&VolumeCandidate)` borrows the frames. `Volume::assemble` calls it and still returns the cleared store. `Volume::construct` calls it, clones the `order` vector of `usize` indexes, and `place`s `candidate.slices[arrival].frame().samples()` at sorted index `n`.

An arrival outside the candidate, which assembly does not produce, returns `SliceOutOfRange` with that arrival and the candidate length. A sorted index that does not fit in `u32` returns the same variant. Neither arm panics.

`place` remains the only copy into the store. `construct` does not call `slice_mut` itself.

Rejected alternatives:

- Re-implement sort and spacing inside `construct`. That is a second geometry.
- Call `StackGeometry::derive`. Different order and a different tolerance.
- Change `assemble` to fill samples. Breaks the progressive contract and AC8.
- Store shear by replacing the normal with the inter-slice vector. Section 19 says the direction comes from Image Orientation Patient, and the sprint's defect is a sheared stack stored as a box.
- Clone every `DecodedFrame` so `assemble` can consume one copy. The borrow avoids a second full sample allocation.

Files:

- `crates/ocelli-volume/src/lib.rs`
- `crates/ocelli-volume/tests/construct.rs`
- `docs/lld/volumes.md` at completion, not before the code exists

## Dependencies, write ownership and resources

Declared dependency: F-033, backlog status `done`.

Source dependency: `DecodedFrame::samples`, `ImagePlane` via the frames F-033 already reads, `Volume::place`.

This sprint has one story. The wave is serial, in the canonical worktree, on `sprint/s18`. No claim, no worker branch, no integration step.

Exclusive write set for the implementation:

- `crates/ocelli-volume/src/lib.rs`
- `crates/ocelli-volume/tests/construct.rs`

Completion later writes `docs/lld/volumes.md`, `docs/sprints/BACKLOG.md`, `CURRENT_SPRINT.md`, `SPRINT_TRACKER.md` and `AS_BUILT.md`. Those stay untouched during implementation.

No shader. No GPU. No browser. The oracle stays serial for the later sprint gate and this story's tests do not open an adapter.

## Verification and evidence reuse

Focused checks, in order:

```bash
bin/ocelli.sh test ocelli-volume --test construct
bin/ocelli.sh check ocelli-volume
bin/ocelli.sh clippy ocelli-volume
bin/ocelli.sh test ocelli-volume
```

The mutation in AC1 is part of implementation, then reverted. It is not a tolerance change.

Final sprint profile runs only after the story is integrated, which for this serial wave is the canonical tree at completion. That profile is `bin/ocelli.sh gate --sprint` over the staged tree, recorded in both ledgers. Metadata edits after that run invalidate it.

No baseline image changes. No size-budget change is expected, because no wasm export is added. If `gate --sprint` reports a size move, stop and treat it as a fact to explain, not a budget edit.

F-033's assembly tests stay GREEN without editing their expected numbers. A change to the 0.5 percent rule is out of scope.

Evidence is invalidated by any edit to `lib.rs` or `construct.rs` after the test run, and by ledger edits before the sprint-profile record.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. `construct` is a load-path call. The one store allocation is `VoxelStore::new`, already sized from `dims`. `place` copies into it. The cloned `order` vector is indexes, not voxels
- unsafe: none
- Tier A (WebGPU): n/a. Host memory. This story creates no device and uploads no texture
- Tier B (WebGL2): n/a. Same reason. It does not read the 3D texture limit
- Tier C (CPU): full. The same `construct` result, because there is no device path to diverge from. Deviation D-07

There is no tier argument and no silent substitute result.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | AC1. Projected order, exact origin, spacing index order, stored bytes. PS3.3 C.7.6.2.1.1 | `crates/ocelli-volume/tests/construct.rs` |
| `fixture` | AC2. Oblique projection, not the z component. Same section | same |
| `fixture` | AC3. Inter-slice spacing is the projected gap. Same section | same |
| `unit` | AC4 shear, AC5 non-uniform, AC6 duplicate and singleton, AC7 short buffer, AC9 multi-sample | same |
| `unit` | AC8. `assemble` stays cleared | same |

The hand-computed values live in the test comments before the implementation is allowed to pass them. They are the numbers in the acceptance table, not values read back from a first run.

Mutation check, HLD 27.3. Place by arrival index, and AC1 goes red. Sort by z, and AC2 goes red. Swap `spacing[0]` with `spacing[1]`, and AC1 goes red. Offset origin by half of `spacing[2]`, and AC1 goes red. Return `Ok` for a lateral step, and AC4 goes red.

## Parity surface covered

None of `docs/hld/B-parity-surface.md`. The surfaces are viewport types, tool classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation representations, events and adapters. `VOLUME_3D` and `ORTHOGRAPHIC` are later stories. This story establishes no rendered observation against cornerstone3D 5.8.2.

## Deviations

None new. D-25 stays as F-033 wrote it. D-07 is cited for the tier C row and is not changed.

## LLD impact

`docs/lld/volumes.md` gains a section for `Volume::construct`: it is the caller of `place`, it copies stored samples in `order`, and it leaves `assemble` cleared. The tier paragraph stays n/a, n/a, full. `docs/lld/cache.md` does not change, because the cache is not called.

## Open questions

None. Shear stays a refusal because F-033's approved plan and section 19's direction matrix already settle it. The LUT chain stays closed because this sprint's goal says so. No host measurement is undecided, because no acceptance row needs one.
