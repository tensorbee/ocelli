# F-057, Progressive volume rendering while the series still loads

**Status**: approved
**Epic ref**: E8.7
**Sprint**: S19
**Estimate**: 3w

## Normative source, transcribed

### `docs/hld/16-volume-representation.md`, section 19

The progressive bullet, quoted as a table cell so the author's semicolon stays data:

|  |
|----|
| **Progressive assembly.** Clear the 3D texture at creation and upload slices as they land. present drives the loading indicator and tells the oracle which frames are comparable yet. |

Layout, also quoted so its semicolon stays data:

|  |
|----|
| **Layout.** One contiguous allocation, x fastest then y then z. Slice n starts at n \* dims\[0\] \* dims\[1\] \* bytes_per_voxel. Do not introduce a per-slice Vec, the whole point is a single upload region. |

### `docs/hld/06-memory-and-cache.md`, section 8

Volume assembly is progressive by default. The viewport renders a partial volume and refines as slices land.

### `docs/hld/05-rendering.md`, section 7

Bricking above 256 MiB is the normal path. A 512 by 512 by 600 sixteen-bit series is about 300 MB against a 256 MiB buffer ceiling. One `queue.submit()` per frame.

### What already exists

`Volume::assemble` returns a cleared store and `present_count == 0`. `Volume::place` copies one slice and sets one presence bit. `Volume::construct` places every slice and returns a complete volume. `Volume::ready_bricks` lists brick coordinates whose z range is fully present, in `BrickGrid::index_of` order, and omits a brick that still has an absent slice. The comment on that function says uploading such a brick would invent zeros.

`Brick::cut` copies one brick out of a contiguous buffer. `RenderGraph::upload_brick` copies a `Brick` into a 3D texture at its true extent, or returns the brick with `BrickUploadError`. On tier C that error is `Unavailable`. `brick_uploads` counts successes only. A second upload of the same key replaces the texture and increments the counter again.

`ocelli-render` does not read `SlicePresence`. Nothing walks `ready_bricks` and calls `upload_brick`.

## What the specification does not cover

Section 19 says to clear the 3D texture and upload slices as they land. This tree's upload unit is a brick, from F-043, because a slice is not a 3D texture. These are the choices that join the two.

1. **The upload unit is a ready brick, not a slice and not a zero-filled brick.** `ready_bricks` is the list. A brick that function omits gets no texture. Absence is the lack of a texture, which is the clear. A pre-allocated zero volume texture is rejected because stored zero is a plausible pixel.
2. **A later `place` that completes a brick uploads it on the next sync.** A `place` that rewrites a slice inside an already uploaded brick uploads that brick again. A sync with no such change uploads nothing and leaves `brick_uploads` unchanged.
3. **The loading indicator is `present_count` and `dims[2]`.** This story returns those two numbers. It does not draw DOM.
4. **A present acquisition slice may be shown through the existing stack upload.** `present_slice_f32` returns that slice's container values as `f32`, or `SliceNotPresent` when the bit is clear. The caller uploads only the `Ok` buffer. A missing slice is not a buffer of zeros.
5. **Sagittal and coronal pictures of a partial volume are not drawn here.** F-052 owns plane sampling and refuses a viewport upload that would paint an absent sample. This story does not invent that picture out of zeros.
6. **Level 0 only.** Coarser levels from F-058 are not uploaded.

## Problem and reproducer

`place` can fill one slice of a cleared volume, and `ready_bricks` can name the bricks that are safe to upload. No caller does the upload. A renderer that uploaded the cleared store would show stored zero for every slice that has not arrived. On a CT that zero becomes air after rescale, which is a plausible image of the wrong patient geometry.

Expected observation: one slice placed in a two-slice volume yields `present_count == 1`, `ready_bricks` empty when that one slice does not cover a brick, and `sync` does not increment `brick_uploads`. After every slice of a brick has been placed, `sync` uploads that brick once and a readback matches the stored bytes. A second `sync` with no new `place` does not upload again. `present_slice_f32` of the missing index returns `SliceNotPresent` and no buffer.

Current behaviour: `upload_brick` works for a brick the test builds by hand. Nothing connects it to `SlicePresence`.

Source identity: `Volume::ready_bricks` in `crates/ocelli-volume/src/lib.rs` and `RenderGraph::upload_brick` in `crates/ocelli-render/src/graph.rs`.

## Scope, API and compatibility

In scope:

- `Volume::cut_ready_brick` in `ocelli-volume`.
- `Volume::present_slice_f32` in `ocelli-volume`.
- `RenderGraph::sync_volume` in `ocelli-render`, which calls `upload_brick` for ready bricks whose bytes changed.
- A dependency from `ocelli-render` to `ocelli-volume`. Volume does not depend on render.

Out of scope:

- Orthographic reslice. F-052.
- A ray-cast shader. F-059 consumes the textures this story uploads.
- Changing `ready_bricks`, the brick edge, or `Brick::cut`.
- A loading widget, a wasm export, or an oracle frame. Decision D7 still has no full-viewer comparison.
- Painting a partial sagittal or coronal plane.

```rust
impl Volume {
    /// Copy one ready brick. A brick `ready_bricks` does not list is refused.
    pub fn cut_ready_brick(&self, brick: [u32; 3]) -> Result<Brick, VolumeError>;

    /// One present slice as little-endian container values in `f32`.
    pub fn present_slice_f32(&self, n: u32) -> Result<Vec<f32>, VolumeError>;
}

impl RenderGraph {
    /// Upload each ready brick whose bytes changed. Stamps live beside brick
    /// residency and clear in the same function that drops brick textures.
    /// `adapter` is the one `upload_brick` already takes.
    pub fn sync_volume(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        volume_id: VolumeId,
        volume: &Volume,
    ) -> Result<ProgressReport, ProgressError>;
}

pub struct ProgressReport {
    pub present_slices: u32,
    pub slice_count: u32,
    pub uploaded: u32,
    pub skipped: u32,
}
```

`VolumeError::BrickNotReady { brick: [u32; 3] }` is new. `VolumeError::SliceNotPresent { index: u32 }` is new. `ProgressError` carries `BrickUploadError` and `VolumeError`. An upload refusal returns the cut brick. It does not add a success path that drops the brick.

`present_slice_f32` decodes `U8`, little-endian `I16`, little-endian `U16`, or little-endian `F32`. It does not apply Bits Stored again. `place` already stored the container bytes. A signed 16-bit value uses `i16`, so `-3` stays `-3` and does not become `65533`.

Refusal: `cut_ready_brick` on a coordinate `ready_bricks` omitted returns `BrickNotReady` and copies nothing. `sync` on tier C returns the existing upload unavailable error and creates no texture. A short internal cut is not papered over. `Brick::cut` errors propagate.

Defaults: the first `sync` uploads every currently ready brick. It does not upload the others under a "temporary" zero texture.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `RenderGraph::sync_volume` | `Volume::ready_bricks`, `Volume::cut_ready_brick`, `RenderGraph::upload_brick` | Load loop, level 0, tier A or B | One texture per ready brick whose bytes are new. `ProgressReport` counts |
| `Volume::present_slice_f32` | `SlicePresence` and `VoxelStore::slice` | Stack view of an acquisition slice that has landed | `Vec<f32>` or `SliceNotPresent` |
| `RenderGraph::upload_brick` | unchanged F-043 path | Called only with a ready brick | Texture at true extent, or the brick returned with the error |
| `Volume::construct` | unchanged | Complete volume | `sync` then uploads every brick, once |
| Oracle comparator | `present_count` | Later, which frames are comparable | This story returns the count. It does not call the oracle |
| F-059 ray cast | resident brick textures | Must not sample a key this sync never uploaded | No texture means no sample |

Units: slice indexes are the sorted volume index, not arrival order and not Instance Number. Brick coordinates are `BrickGrid` coordinates. The stamp is the brick's bytes, compared by equality, not a hash that can collide and skip a real change.

Rounding: container integers that fit in 24 bits are exact in `f32`. `F32` bits are preserved with `f32::from_le_bytes`. No window is applied in this conversion.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Cleared volume, no textures | `sync` | `uploaded == 0`, `skipped` is the brick count, `present_slices == 0` | No texture created |
| One or more slices placed, no brick fully covered | `sync` | Still no texture. `present_slices` matches `present_count` | The cleared voxels are not uploaded |
| A brick becomes fully present | `sync` | That brick's texture exists and readback matches `cut_ready_brick` | Other bricks stay absent |
| Same volume, no further `place` | `sync` again | `brick_uploads` unchanged | Textures kept |
| A present slice's bytes replaced via `place` | `sync` | That brick is uploaded again. Counter increments | The previous texture is replaced, not left beside the new one |
| Absent slice index | `present_slice_f32` | `SliceNotPresent` | No zero buffer allocated for the caller |
| Tier C graph | `sync_volume` | `BrickUploadError::Unavailable` | No stamp is recorded |

Invariant: every uploaded key is a member of `ready_bricks` at the time of that upload. A brick texture's samples are the store bytes for that brick. They are not a resample and not a LUT output.

A repeated `sync` with equal bytes is a no-op on the device. A repeated `sync` after `place` of different bytes is a new upload. Equal arguments are not treated as a no-op when the bytes differ.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| 3D brick texture | `RenderGraph::upload_brick` | `BrickKey` of `volume_id`, level 0, brick coordinate | Successful return of `upload_brick` | On error the brick is returned and the counter stays. The previous texture for that key stays |
| Host brick bytes | `cut_ready_brick` | The `Brick` value passed into upload | Dropped after a successful upload, as F-043 already does | `BrickNotReady` allocates no `Brick` |
| Stamp map | `RenderGraph`, beside `BrickResidency` | The graph and the `BrickKey` | Updated only after a successful upload. Cleared where brick textures are dropped | A failed upload does not stamp the brick, so the next sync tries again |
| Stack texture of one slice | The caller, through existing `upload_stored` | The stack viewport id | Not this story's commit | This story only refuses to hand over a zero slice |

Device loss still follows F-037. The rebuild path that drops brick textures also clears the stamps, so the next `sync_volume` uploads ready bricks again. A failed upload does not write a stamp.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `U8` volume dims `[4, 1, 2]`, spacing irrelevant. `place` slice 0 with `[1, 2, 3, 4]`. Slice 1 stays clear. `ready_bricks` is empty because z covers both slices in one brick of edge 128. `cut_ready_brick([0, 0, 0])` returns `BrickNotReady`. `present_slice_f32(1)` returns `SliceNotPresent`. `present_slice_f32(0)` returns `[1.0, 2.0, 3.0, 4.0]` | `an_incomplete_brick_is_not_cut` | Returning a brick whose second row is zeros. Returning a zero `Vec` for the missing slice | Implementation. `ocelli-volume` test |
| AC2 | The AC1 volume after `place` of slice 1 with `[5, 6, 7, 8]`. `cut_ready_brick([0, 0, 0])` has extent `[4, 1, 2]` and samples `[1, 2, 3, 4, 5, 6, 7, 8]` | `a_ready_brick_copies_both_slices` | x/y swap, or copying only the first slice and padding | Implementation. `ocelli-volume` test |
| AC3 | `I16` slice bytes `FD FF` for the single voxel of a one-column, one-row, two-slice volume is not used until both slices exist. After both are placed, the first voxel decodes as `-3.0`, not `65533.0` | `signed_samples_stay_signed` | Reading the container as `u16` | Implementation. `ocelli-volume` test |
| AC4 | GPU, tier A or B. Volume of AC2. First `sync` increments `brick_uploads` by 1 and readback matches the eight bytes. Second `sync` leaves the counter unchanged. `present_slices` is 2 and `slice_count` is 2 | `sync_uploads_a_ready_brick_once` in `ocelli-render` tests, ignored, run by the `gpu` gate | Uploading before the second slice. Uploading again when nothing changed. Readback of zeros | Integrated. GPU readback |
| AC5 | GPU. AC1 volume, `sync`. `brick_uploads` stays 0 and `brick_texture` for that key is absent. `skipped` is 1 | `sync_skips_a_brick_with_an_absent_slice` | Creating a texture of zeros and counting it as progress | Integrated. GPU |
| AC6 | GPU. After AC4, `place` replaces slice 0 with `[9, 9, 9, 9]` and `sync` runs. The counter increments by 1 and readback starts with `9, 9, 9, 9` | `a_replaced_slice_uploads_again` | Keeping the stamp and showing the old ramp | Integrated. GPU |
| AC7 | Tier C. `sync` returns unavailable and the stamp map stays empty | `sync_is_unavailable_on_tier_c` | A CPU buffer of zeros described as the 3D texture | Implementation. No device |

AC4, AC5 and AC6 need an adapter. They follow the ignored-test pattern in `crates/ocelli-render/tests/brick_upload.rs`. A missing adapter does not convert them to a pass. The CPU rows AC1, AC2, AC3 and AC7 are the floor.

Semantic RED: `cut_ready_brick` that ignores presence and calls `Brick::cut` on the cleared store returns zeros for the absent slice. AC1 must fail on that. GREEN is `BrickNotReady`.

## Approach

`cut_ready_brick` checks membership in `ready_bricks` and then calls `Brick::cut` on level 0's bytes only. Those bytes are the prefix of the store of length `dims product * bytes_per_voxel`, which stays true after F-058 appends a coarser level past that prefix. The cut uses `BrickSample::from(self.voxel)`.

`present_slice_f32` checks `present.test` before decoding.

Stamps are `BTreeMap<BrickKey, Vec<u8>>` on the graph beside `BrickResidency`. The value is the last uploaded brick bytes. `sync_volume` lists `ready_bricks`, cuts each, and calls `upload_brick` when the key is new or the bytes differ. It does not iterate the grid and upload the complement. The extra host copy is load-time state, not a render-loop allocation, and it is dropped in the same function that calls `self.bricks.clear()`.

The render crate's `Cargo.toml` gains `ocelli-volume`. `lib.rs` gains `mod progressive`. `graph.rs` is not given a new pass. `upload_brick` stays the commit point. `sync_volume` and the stamp field live on `RenderGraph`. `sync_volume` delegates the walk to `sync_ready_bricks`, which refuses tier C before the walk and before any stamp write.

Rejected alternatives:

- Upload every brick immediately, with zeros for absent slices. The defect this sprint names.
- Upload per slice into a slice-sized 3D texture. Breaks the brick key F-043 established.
- A second presence bitset inside the renderer. `SlicePresence` is already the set.

## Dependencies, write ownership and resources

Depends on F-051, `done`. F-058 lands first in this sprint so level-0 offsets stay the documented prefix. This story does not require F-058's append API. It must tolerate a longer store.

Writes:

- `crates/ocelli-volume/src/lib.rs`
- `crates/ocelli-volume/tests/progressive.rs`
- `crates/ocelli-render/Cargo.toml`
- `crates/ocelli-render/src/lib.rs`
- `crates/ocelli-render/src/graph.rs` only for the stamp field, `sync_volume`, and the existing `bricks.clear()` site
- `crates/ocelli-render/tests/progressive.rs`
- `docs/lld/volumes.md` and `docs/lld/render-graph.md` at completion

GPU is exclusive. This story's ignored tests and F-059's ignored tests do not run at the same time. Serial wave, canonical worktree, after F-058.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-volume
bin/ocelli.sh test ocelli-volume --test progressive
bin/ocelli.sh check ocelli-render
bin/ocelli.sh test ocelli-render --test progressive
bin/ocelli.sh clippy ocelli-volume
bin/ocelli.sh clippy ocelli-render
```

The ignored GPU tests run under `bin/ocelli.sh gate gpu` at story completion and again under `bin/ocelli.sh gate --sprint` after integration. A floor run that skips `#[ignore]` does not satisfy AC4, AC5 or AC6.

Evidence is invalidated by a change to `ready_bricks` or to the stamp clear on device rebuild. No new size budget and no new timing threshold.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. Brick bytes stay in the render worker's textures
- Render-loop allocation: none. `sync` runs when a slice lands, not per fragment
- unsafe: none
- Tier A (WebGPU): full. Ready bricks upload at their integer format
- Tier B (WebGL2): full. The same path. F-043 already refuses an axis past `max_tex_3d` and does not substitute a format
- Tier C (CPU): unavailable. `BrickUploadError::Unavailable`. No zero volume is drawn instead

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 and AC2 brick bytes against a hand-filled store. PS3.3 layout is section 19's x-fastest slice formula, not a LUT section | `ocelli-volume` `tests/progressive.rs` |
| fixture | AC3 signed decode of `0xFD 0xFF` as `-3` | same |
| unit | AC7 tier C refusal | `ocelli-render` `tests/progressive.rs` |
| gpu | AC4, AC5, AC6 readback | ignored tests in that file |

## Parity surface covered

None. Appendix B does not list progressive assembly. The viewport type that will sample these bricks is F-059.

## Deviations

None.

## LLD impact

`docs/lld/volumes.md` gains `cut_ready_brick` and `present_slice_f32`. `docs/lld/render-graph.md` gains `sync_volume` and the stamp. A brick `ready_bricks` omits is not uploaded. Recovery drops textures and stamps.

## Fact correction

`RenderGraph::upload_brick` takes `&ResolvedAdapter`. `get_texture_format_features` is a method of the wgpu adapter, and `GpuContext` does not store one. `sync_volume` takes the same reference and passes it through. This is not a second adapter and not a new pass.

`RenderGraph::new` refuses tier C before a graph exists, so a floor test cannot call `sync_volume` on a device. `sync_ready_bricks` is the function `sync_volume` calls before any stamp write. The floor test drives it with `Tier::Cpu` and a stamp map. That is the same shape as `brick_plan`.

A cut failure is `VolumeError`, because `BrickError` is not `thiserror` and cannot be `?` into the upload error. `ProgressError` carries that volume error. Tier C cuts nothing, so the upload error's brick is absent.

## Open questions

None. Partial sagittal display stays with F-052's refusal, and the ray cast stays with F-059. Both are scheduled in this sprint behind this story's textures.
