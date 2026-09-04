<!-- Originally converted by scripts/split_hld.py from Ocelli-HLD.docx.
     This tracked Markdown file is authoritative.

     Prose, tables and code text are the author's, unaltered. Two things in
     the code listings are NOT: blank-line spacing and indentation, both of
     which Word carried as paragraph formatting rather than as characters and
     which this script re-derives from bracket depth. Neither can change what
     the code means. Where a listing's exact bytes matter, the tracked Markdown wins. -->

# The boundary, in code

**Source**: bootstrap import from `Ocelli-HLD.docx`, section 17. This tracked Markdown is authoritative.
**Status**: normative. A deviation is raised in a design plan, not improvised.
**F-IDs that contributed**: none yet.

---

## 17. The boundary, in code

### 17.1 Control channel

Commands are a packed binary buffer, not JSON, and many commands ride in one call per frame. The header is fixed so the decoder is a loop, not a parser.

```rust
// wire format, little-endian
// [u16 kind][u16 viewport_id][u32 payload_len][payload ...] repeated
// ocelli-wasm/src/lib.rs
#[wasm_bindgen]
impl Session {
    /// Apply a packed command buffer. Returns the number of commands applied.
    #[wasm_bindgen(js_name = applyCommands)]
    pub fn apply_commands(&mut self, ptr: *const u8, len: usize) -> u32 {
        let buf = unsafe { core::slice::from_raw_parts(ptr, len) };
        self.inner.apply(buf)
    }
}
```

### 17.2 Bulk channel — and the trap

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

### 17.3 Event ring

```rust
// ocelli-wasm/src/ring.rs -- single producer (Rust), single consumer (JS)
#[repr(C)]
pub struct RingHeader {
    pub head: u32, // producer writes
    pub tail: u32, // consumer writes
    pub cap: u32, // power of two
    pub dropped: u32, // overflow count; nonzero means JS is not draining
}
#[repr(C)]
pub struct Event {
    pub kind: u32,
    pub viewport: u32,
    pub seq: u64,
    pub payload: [u8; 32],
}
// 48-byte stride. JS reads with a DataView at header_size + i * 48.
```

*Fixed stride so the JavaScript side is arithmetic, not deserialisation. Drain once per animation frame; surface \`dropped\` in telemetry rather than swallowing it.*

### 17.4 State readback

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewportState {
    pub camera: [f32; 16],
    pub voi_center: f32,
    pub voi_width: f32,
    pub slice_index: u32,
    pub num_slices: u32,
    pub flags: u32, // bit 0 invert, bit 1 loading, bit 2 error
    pub _pad: [u32; 3],
}
```

*Copied into a JS-owned buffer on request. Never returned as a JS object graph — that is one allocation and one conversion per field.*
