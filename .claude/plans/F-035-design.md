# F-035, Zero-copy handoff strategy from decode worker to renderer

**Status**: approved
**Epic ref**: E5.5
**Sprint**: S13
**Estimate**: 2w

## Normative source, transcribed

### `docs/hld/04-boundary-and-data-path.md`, section 5.2

|  |
|----|
| The core allocates and returns a pointer and length; JavaScript builds a typed-array view immediately before writing and discards it after. Views are never cached across a call that might allocate, because any WebAssembly memory growth relocates the backing buffer and detaches every outstanding view. This is the sharpest edge in the whole design and §17.2 gives the exact pattern. |

### `docs/hld/04-boundary-and-data-path.md`, section 6, figure caption

|  |
|----|
| One hop disappears and one constraint disappears with it. Decoded frames stop becoming JavaScript objects, and a single GPUDevice replaces the pooled WebGL contexts cornerstone works around with an entire rendering-engine variant. |

### `docs/hld/14-the-boundary-in-code.md`, section 17.2

```typescript
#[wasm_bindgen]
impl Session {
    /// Reserve `len` bytes and return a pointer into linear memory.
    pub fn alloc(&mut self, len: usize) -> *mut u8 { /* ... */ }
    /// Hand ownership back; `ptr` must be the value returned by `alloc`.
    pub fn commit_frame(&mut self, ptr: *mut u8, len: usize, meta: &FrameMeta) { /* ... */ }
}
// packages/core/src/bulk.ts -- the ONLY correct order
const ptr = session.alloc(bytes.byteLength);
// Build the view AFTER the allocation. Use it immediately. Let it go.
new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);
session.commit_frame(ptr, bytes.byteLength, meta);
```

|  |
|----|
| **NEVER CACHE THE VIEW** A module-level const HEAP = new Uint8Array(wasm.memory.buffer) is the classic failure. Any wasm memory growth relocates the ArrayBuffer and detaches every outstanding view; the next write silently targets a detached buffer or throws far from the cause. Add an ESLint rule banning new Uint8Array(wasm.memory.buffer) outside the two functions that are allowed to do it. |

### `docs/hld/21-worker-protocol.md`, section 24

```text
main -> decode : { kind:'decode', seriesId, frameIndex, buffer } [transfer]
decode -> render : { kind:'frame', frameId, buffer, meta } [transfer]
main -> render : { kind:'commands', buffer } [transfer]
render -> main : drained event ring [copy]
// Always transfer, never copy:
// worker.postMessage(msg, [msg.buffer]);
```

|  |
|----|
| Three roles: the main thread, N decode workers each with its own WebAssembly instance, and one render worker owning the GPUDevice and every OffscreenCanvas. Decode workers never touch the GPU; the render worker never decodes. |

### `docs/hld/02-standing-decisions.md`, decision D5, as restated in `CLAUDE.md`

|  |
|----|
| Single-threaded, one wasm instance per worker. Stable Rust, no SharedArrayBuffer. |

## What the specification does not cover

1. **F-101 owns the live `Session`.** Section 17.2's `alloc` and `commit_frame` are the boundary story in S16. This story does not add those exports. `packages/core/src/bulk.ts` already speaks to a `BulkSink` stand-in, and the new functions use that same stand-in.

2. **Two wasm instances cannot share linear memory.** Decision D5 forbids `SharedArrayBuffer`. A decoded frame therefore cannot move by pointer. Section 24's transferred `ArrayBuffer` is the specified hop. Producing that buffer requires one copy out of the decode instance. The transfer itself copies nothing. The render instance then copies once into its own linear memory, through the section 17.2 order, and drops the JavaScript buffer. A design that skipped the render-side copy and called `write_texture` from JavaScript would keep the pixels as a JavaScript view, which section 6 forbids. `write_texture` itself is F-040.

3. **The story title says zero-copy, and section 17.2 is a copy.** The zero-copy edge is the worker transfer in section 24. The copies are the two bulk-channel crossings the separate instances force. Collapsing the hop into a `Uint8Array` that outlives the call fails decision D3, which is pixels never crossing the boundary as JavaScript objects. The returned value is an `ArrayBuffer` destined for `postMessage` transfer, not a retained view.

4. **No third file may build a view over `wasm.memory.buffer`.** `eslint.config.js` allows that in `bulk.ts` and `panic.ts` only. The header of `bulk.ts` says a third production entry is a design-plan decision. This plan does not take it. Both new functions live in `bulk.ts`.

5. **`meta` has no specified layout beyond the name.** This story carries `FrameKey` plus the byte length, as `HandoffMeta` in `ocelli-cache`, which already owns `FrameKey` from F-032. It carries no samples. Samples stay in the transferred buffer.

## Approach

`exportDecodedFrame(wasm, ptr, len)` copies `[ptr, ptr + len)` out of linear memory into a fresh `ArrayBuffer`. The view is constructed inside the function, used for that copy, and then unreachable. The function returns the `ArrayBuffer` and nothing else. It does not accept a pre-built view, and it does not store one.

`acceptTransferredFrame(wasm, session, buffer, meta)` is the render-worker side. It copies `buffer` into the core with the section 17.2 order, which is `alloc`, then a view, then `set`, then `commit_frame`, and it does not retain `buffer` or the view. The caller transfers `buffer` with `postMessage`. A helper `transferredFrameMessage(frameId, buffer, meta)` returns `{ kind: 'frame', frameId, buffer, meta }` and the transfer list `[buffer]`, matching section 24's line. It does not call `postMessage` itself, because this package does not own the worker.

`HandoffMeta` is a `#[repr(C)]` value of `series`, `instance`, `frame` and `bytes`, little-endian, with no heap. `FrameKey` is the identity. `bytes` is the sample length the transfer must match. A mismatch is a typed refusal on the Rust side and a thrown error on the TypeScript side, before any copy.

The growth fixture is the one `bulk.test.ts` already uses. `WebAssembly.Memory.prototype.grow` detaches the previous buffer. The first call succeeds. The memory then grows. The second call must still deliver the new bytes. A module-level view taken on the first call fails the second. That is the mutation the test is written to catch, and the file records the expected red count the way the existing ordering tests do.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: the transferred `ArrayBuffer` is section 24's hop, consumed immediately on receipt. No retained `Uint8Array` over wasm memory, and no pixel object graph.
- Render-loop allocation: none. This story is not on the render loop.
- unsafe: none
- Tier A (WebGPU): n/a. No GPU object is created.
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | A growth between two exports still delivers the second call's bytes. A hoisted view fails it. | `packages/core/src/bulk.test.ts` |
| unit | `acceptTransferredFrame` finishes the copy before `commit_frame`, in the section 17.2 order. | `packages/core/src/bulk.test.ts` |
| unit | The message helper lists the buffer in the transfer list and returns `kind: 'frame'`. | `packages/core/src/bulk.test.ts` |
| unit | `HandoffMeta` rejects a length that disagrees with the buffer, and its fields round-trip little-endian. | `crates/ocelli-cache` test beside `image.rs` |

No pixel arithmetic is added. The bytes are copied unchanged.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column. `docs/sprints/AS_BUILT.md` already records that.

## Deviations

None.

## LLD impact

`docs/lld/typescript-packaging.md`, for the two new functions and the reason they stay in `bulk.ts`. `docs/lld/cache.md`, for `HandoffMeta`.

## Open questions

None. The constraints above are fixed by section 24, decision D5, the eslint allow-list, and F-101's ownership of the live `Session`.
