# F-047, Stack scroll, prefetch strategy, image sequencing

**Status**: approved
**Epic ref**: E7.3
**Sprint**: S15
**Estimate**: 3w

## Normative source, transcribed

### `docs/hld/06-memory-and-cache.md`, section 8

```text
One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs.
```

`ImageCache` is that decoded LRU. This story reads it. It does not build a second cache.

### `docs/hld/19-render-graph.md`, section 22, the slice flag

```text
Dirty tracking. A viewport re-renders only when camera, VOI, slice, segmentation or data changed. Everything else is a no-op frame.
```

`Dirty.slice` already exists. Changing the selected index is what sets it. A prefetch insert does not.

### `docs/hld/26-differentiating-capabilities.md`, section 30, the sentence this story does not implement

```text
Prefetch runs along the scroll axis, which is where the next request almost always is.
```

That sentence is about volume bricks. CURRENT_SPRINT says volume brick prefetch is not this story. The image-cache neighbour keys below are the stack strategy. They are not section 30.

### `docs/hld/14-the-boundary-in-code.md`, section 17.4, the slice fields

```rust
pub slice_index: u32,
pub num_slices: u32,
```

F-101 copies them. This story stores the index and the length those fields will carry.

### DICOM, the order this story refuses to invent

`docs/lld/volumes.md` sorts a volume by the projected image position. A stack scroll does not. The sprint text is "scrolls the stack in its stored order." Stored order is the order of the `FrameKey` list the caller supplies. Instance number, slice location and image position do not reorder it.

## What the specification does not cover

1. **The radius is one.** Prefetch keys are the selected index minus one and plus one, when those indices exist. No other radius is read from configuration. Section 30 does not supply one for this path.

2. **Prefetch returns keys. It does not decode and it does not upload.** `ImageCache::contains` is added as a forward to `Lru::contains_key`, which takes `&self` and does not update recency. `get` is called only for the selected key. A neighbour that is resident is not promoted and is not returned as the displayed frame.

3. **A miss shows nothing from this stack.** `samples_for_upload` returns `NotResident` when the selected key is absent. It does not return the previous key and it does not return a neighbour. The caller must not call `upload_stored` with any other frame's samples. This story does not edit `graph.rs`. The previous GPU texture remaining on screen would be a caller that ignored `NotResident`. The function gives that caller no bytes to upload.

4. **The sequence is not sorted.** A list whose image positions are `z = 2`, `z = 0`, `z = 1` scrolls `0`, `1`, `2`. This crate does not read positions.

5. **Clamping.** `scroll_by` adds a signed delta and clamps to `0 .. len`. It does not wrap. An empty sequence has no index and `samples_for_upload` returns `Empty`.

## Approach

`StackSequence` in `crates/ocelli-viewport/src/scroll.rs` holds `FrameKey` values in caller order and a `u32` index. `scroll_by` changes the index. `displayed_key` returns that key. `prefetch_keys` returns zero, one or two keys. `samples_for_upload(&mut ImageCache)` calls `get` only for `displayed_key` and returns the samples or `NotResident`.

`ImageCache::contains(&self, key) -> bool` forwards to `contains_key`. Prefetch planning uses `contains` so a probe cannot evict the selected frame by refreshing a neighbour.

`ocelli-viewport` depends on `ocelli-cache` for `FrameKey` and `ImageCache`. The crate stays `no_std` with `alloc`, matching `ocelli-cache`. `lib.rs` gains `pub mod scroll`.

No shader edit. No `wgpu`. The TypeScript `SliceIndex` type is F-100's.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. The returned samples stay in the cache allocation. They are not copied into a JavaScript object.
- Render-loop allocation: none. The key list is fixed when the sequence is built. Scroll does not allocate a frame.
- unsafe: none
- Tier A (WebGPU): n/a for this module. The caller uploads the selected samples through the existing stored-texture path.
- Tier B (WebGL2): n/a, same reason.
- Tier C (CPU): full for selection. The cache is host memory. Drawing the selected frame is F-X003.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Keys stored as positions `z` would sort `1, 2, 0` still scroll `0, 1, 2`. The sequence never reads a position. | `crates/ocelli-viewport/tests/scroll.rs` |
| unit | Index `1` of three prefetches keys `0` and `2` and displays key `1`. Index `0` prefetches only key `1`. Index `2` prefetches only key `1`. | same |
| unit | Cache holds `0` and `2`, index is `1`. `samples_for_upload` is `NotResident`. The bytes of `0` and of `2` are not returned. | same |
| unit | `scroll_by` across the ends clamps. It does not wrap. An empty sequence is `Empty`. | same |
| unit | `contains` on the oldest resident key does not refresh it. A later insert still evicts that oldest key. | `crates/ocelli-cache/tests/image_cache.rs` |

Mutate `samples_for_upload` so a miss returns the neighbour, and confirm the miss fixture goes red. Revert.

## Parity surface covered

none as a checklist row. Appendix B has no scroll row. The stack viewport type is F-045's.

## Deviations

none

## LLD impact

- `docs/lld/viewport.md`, a scroll section
- `docs/lld/cache.md`, one sentence that `ImageCache::contains` does not update recency

## Open questions

None.
