# F-043, Chunked brick residency as the default upload path

**Status**: approved
**Epic ref**: E6.7
**Sprint**: S14
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/05-rendering.md`, section 7

|  |
|----|
| **Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup. |
| **Volume rendering runs on both tiers**, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute — GPU segmentation, histogram passes, compute-based resampling — is tier A only and must degrade, not fail. |
| **Bricking above 256 MiB.** A 512×512×600 sixteen-bit CT series is roughly 300 MB against a guaranteed maximum buffer size of 256 MiB, so chunked upload is the normal path, not an optimisation. |
| **One submit per frame.** The render graph tracks dirty viewports and issues a single submission across all of them, driven by requestAnimationFrame inside the render worker. |

### `docs/hld/11-decision-log.md`, section 14, decision D11

| **#** | **Decision** | **Rejected alternative** | **Consequence** |
|----|----|----|----|
| D11 | Chunked residency is the default path | Brick only above a size threshold | Bounded GPU memory becomes a claim, not a fallback |

Decision D11 is not deviation D-11. The hyphen is the whole distinction.

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
| **Layout.** One contiguous allocation, x fastest then y then z. Slice n starts at n * dims[0] * dims[1] * bytes_per_voxel. Do not introduce a per-slice Vec; the whole point is a single upload region. |
| **Progressive assembly.** Clear the 3D texture at creation and upload slices as they land. present drives the loading indicator and tells the oracle which frames are comparable yet. |
| **Bricks and a level axis, from the start.** The volume carries a multiscale level axis and 128³ brick decomposition even when a series fits comfortably. Phase 1 uploads every brick; §30 makes residency selective. Retrofitting a level axis into a volume model that never had one is the rewrite cornerstone3D cannot afford. |

### `docs/hld/06-memory-and-cache.md`, section 8

|  |
|----|
| One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs. |
| Volume assembly is progressive by default: the viewport renders a partial volume and refines as slices land, which matters more for perceived speed than any decode optimisation. The absence of a garbage collector is the real memory story — a 300 MB volume load has no pause behaviour to tune around, only a budget to respect. |

### `docs/hld/17-cache-and-allocation.md`, section 20

|  |
|----|
| **No allocation in the render loop.** Pre-size everything at viewport creation. A frame that allocates is a frame that can stutter. |
| **Use bytemuck::cast_slice** for reinterpreting pixel buffers. Hand-written transmutes are unsafe code with no upside here. |
| **Reuse staging buffers by size class.** Texture uploads should draw from a small pool, not allocate. |

### `docs/hld/26-differentiating-capabilities.md`, section 30

|  |
|----|
| **Residency** is the view frustum intersected with the current level of detail, recomputed per frame against a byte budget. GPU memory becomes a configured number rather than a function of series size. |
| **Level selection** compares projected voxel size against screen pixel size. |
| **Bricks** are 128³, or 64³ for sixteen-bit data, addressed through a 3D indirection texture. The cache is ocelli-cache with a brick key — no second cache. |
| **Prefetch** runs along the scroll axis, which is where the next request almost always is. |

Section 30 is the later consumer. This story does not build it. Phase 1 uploads every brick.

### `docs/hld/04-boundary-and-data-path.md`, section 6

|  |
|----|
| The LUT chain is an explicit shader stage rather than a CPU pass. All of it reads from one uniform block, so a window-level drag uploads a few bytes per frame instead of re-uploading a texture. |

### `docs/hld/23-performance-rules.md`, section 26

|  |
|----|
| No allocation in the render loop. |
| One queue.submit() per frame. |
| Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload. |

### Pinned wgpu 30.0.1

`Queue::write_texture` copies bytes into a texture. The data must be in the same format as the texture. `wgpu_types::COPY_BYTES_PER_ROW_ALIGNMENT` is 256 and its own comment says the alignment does not apply to `Queue::write_texture`. It applies to `copy_buffer_to_texture` and `copy_texture_to_buffer`.

`TextureFormat` documents that `Unorm` scales the integer range to float 0 to 1, and that `Uint`, `Sint` and `Float` perform no conversion. `Limits::downlevel_defaults` sets `max_texture_dimension_3d` to 256.

### What the tree already decided

`BrickGrid::new` chooses the edge. Eight-bit samples take 128. Sixteen-bit samples and `F32` take 64. An edge brick keeps its true extent. `Brick::bytes` is the buffer length. `BrickCache` is one `Lru` on `CacheTier::Gpu` and holds host bytes. `Volume::brick_grid` is the only decomposition. F-040 uploads a 2D stored frame with `write_texture` and `set_voi` does not upload it again. That path stays.

## What the specification does not cover

1. **Section 7's 256 MiB sentence and decision D11 disagree.** Section 7 describes bricking as what a series larger than a buffer does. D11 rejects a size threshold. This story follows D11. A volume that fits in one 3D texture is still uploaded as bricks. There is no comparison with 256 MiB and no whole-volume texture.

2. **The GPU texture's format.** Section 30 names the brick edge and not a `TextureFormat`. The bytes in the brick are the volume's container bytes. `place` copies them and does not mask `BitsStored`. This upload does not mask them either and does not sign-extend. That unpacking already lives in `ocelli-pixel` and a second copy here would be a second stored-value path. The format is the one that stores those bits with no numeric conversion. `U8` is `R8Uint`. `U16` is `R16Uint`. `I16` is `R16Sint`. `F32` is `R32Float`. `BrickWidth` collapses the two sixteen-bit kinds, so the sample kind has to travel with the brick. `R8Unorm` and `R16Unorm` are refused because they scale to 0 to 1, which is a different number. A widening to `R32Float` is also refused. It would be a second representation and a different size, which is the substitution this sprint forbids.

3. **One texture per brick, not one texture per volume.** Section 19's single allocation is the host store. The same section says phase 1 uploads every brick. A longer axis is more brick textures. The texture's extent is the brick's true extent on each axis. It is not the nominal edge and it is not the volume's dimensions. F-036's plan said the indirection texture belongs to this story. Section 30 is where that texture lives, and this sprint's done-means leaves it there. It is not built.

4. **Row alignment is not a padded brick.** `write_texture` accepts a tight row. The copy layout's `bytes_per_row` is `extent[0] * bytes_per_voxel`. A readback through `copy_texture_to_buffer` pads its buffer to 256 bytes per row and then unpacks. That pad is not texels. The texture dimensions stay the true extent.

5. **Which bricks are ready.** Section 19's presence bit is per slice. A brick whose z range includes an absent slice is not uploaded. The helper lives on `Volume`, because the renderer does not learn `SlicePresence`. Uploading zeros for a slice that has not landed would be a voxel the study does not contain.

6. **Where the texture lives, and what happens to the host buffer.** `ocelli-cache` has no `wgpu` dependency and must not gain one. The texture map lives in `ocelli-render`, keyed by the existing `BrickKey`. There is no second cache type and no second key. Phase 1 does not evict a texture to meet a byte budget. That budget is section 30. The upload takes the `Brick` by value and drops it after the copy, so those bytes are not also inserted into `BrickCache`. `BrickCache`'s comment names F-040 as the story that removes the host buffer. F-040 did not. This story does. On refusal the brick is returned so a tier C caller can keep it in `BrickCache`.

7. **The staging pool.** Section 20 names a size-class pool. `write_texture` takes a slice and does not take a `wgpu::Buffer`. F-040 did not build the pool. This story does not either.

8. **No volume shader.** Nothing samples the 3D texture yet. This story proves the bytes and the extent. A later ray cast that cannot sample the format reports unavailable. It does not requantise.

## Approach

`BrickSample` sits beside `BrickWidth` in `ocelli-cache`. The variants are `U8`, `I16`, `U16` and `F32`. `Brick` carries one. `Brick::cut` takes it. `VoxelKind` converts to it in `ocelli-volume`, and signed and unsigned sixteen-bit stay distinct there.

`Volume::ready_bricks` returns the brick coordinates whose slices are all present, in `index_of` order. A brick that touches an absent slice is absent from that list. The caller cuts and uploads only those.

`brick_plan` in `ocelli-render` is pure. It takes the tier, `max_texture_dimension_3d`, the format features, and the brick. Tier C returns `Unavailable` and does not name a format. An extent of zero returns `EmptyExtent`. An axis longer than the device's 3D limit returns `ExtentUnavailable`. The format is the one match above, and it is usable only when `TEXTURE_BINDING` and `COPY_DST` are both set. No second format is tried. The plan's texture size is the brick extent. A volume of `[512, 512, 600]` at sixteen bits plans brick `[0, 0, 9]` as `[64, 64, 24]`, never as `[512, 512, 600]`. A volume of `[8, 8, 8]` at eight bits plans `[8, 8, 8]`, never a 128 cube and never the 2D stored-frame path.

`BrickResidency` holds one 3D texture per `BrickKey`. `upload` calls `brick_plan` first. On refusal it returns the brick. On acceptance it creates a `TextureDimension::D3` texture at the true extent, calls `queue.write_texture` with the tight row, stores the texture, drops the brick, and increments `brick_uploads` by one. A second upload of the same key replaces the texture and increments again. `RenderGraph` owns the residency. `set_voi` writes the thirty-two-byte uniform and does not call `upload`. `install_recovered` drops the textures with the other per-viewport GPU objects and does not increment `brick_uploads`. The caller supplies the bricks again.

The 2D `upload_stored` path is unchanged. A window change still does not increment `uploads` or `brick_uploads`.

Bytes do not cross into JavaScript. No `wasm_bindgen` attribute is added.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Upload runs when a ready brick is supplied. `set_voi` does not upload. The render loop binds what it already holds.
- unsafe: none
- Tier A (WebGPU): full, one 3D texture per ready brick at the brick's own sample format, when the device can sample and copy that format at the true extent
- Tier B (WebGL2): the same formats when the device's 3D limit covers the brick and the format features include `TEXTURE_BINDING` and `COPY_DST`. A brick edge of 128 fits the pinned downlevel floor of 256. A longer volume axis is more bricks. A format or an extent the device cannot sample is unavailable, with no substitute image
- Tier C (CPU): unavailable. No texture is created. The returned brick stays on the host, in `BrickCache` if the caller inserts it

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | A 3×3×3 ramp, values 0 through 26, x fastest. Section 19's layout, and PS3.3 C.7.6.3 for row-major pixel data. The planned texture is `[3, 3, 3]`. The GPU readback matches those 27 bytes. An edge brick of a `[130, 1, 1]` eight-bit volume is texture width 2, not 128. | `ocelli-render` ignored GPU test, plus the pure plan test |
| fixture | `I16` selects `R16Sint` and `U16` selects `R16Uint` for the same bytes `0x00 0x80`. PS3.3 C.7.6.3 Pixel Representation 0 and 1. The bits are not a number until the format says which. | `ocelli-render` unit test, no GPU |
| unit | `[512, 512, 600]` sixteen-bit plans 8×8×10 bricks. Brick `[0, 0, 9]` has extent `[64, 64, 24]`. An `[8, 8, 8]` volume plans the brick extent, not a whole-volume extent. No plan reads a 256 MiB threshold. | `ocelli-render` unit test, no GPU |
| unit | Tier C, a missing usage, and a 3D limit below the brick axis return before any texture and name no substitute format. | `ocelli-render` unit test, no GPU |
| unit | A brick whose z range includes an absent slice is not in `ready_bricks`. A complete brick is. | `ocelli-volume` unit test, no GPU |
| unit | `set_voi` leaves `brick_uploads` unchanged and the read-back texel unchanged. A second sample upload increments the counter by one. | `ocelli-render` ignored GPU test |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column.

## Deviations

None.

## LLD impact

`docs/lld/render-graph.md` for the residency, the counter, and the format refusal. `docs/lld/cache.md` for the host buffer leaving on a successful upload. `docs/lld/volumes.md` for `ready_bricks`.

## Open questions

None. The threshold, the true extent, the three tiers, and the exclusion of section 30 were already decided. The format rule follows the pinned `TextureFormat` conversion table and the sprint's refusal to substitute an image.
