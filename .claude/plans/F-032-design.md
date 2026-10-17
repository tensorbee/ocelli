# F-032, Image cache with eviction events surfaced to JS

**Status**: approved
**Epic ref**: E5.2
**Sprint**: S12
**Estimate**: 2w

## Normative source, transcribed

### `docs/hld/06-memory-and-cache.md`, section 8

|  |
|----|
| One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs. |

### `docs/hld/17-cache-and-allocation.md`, section 20, the signature this story consumes

```rust
impl<K, V: Budgeted> Lru<K, V> {
    /// Returns the entries evicted to make room, so the caller can emit events.
    pub fn insert(&mut self, k: K, v: V) -> Vec<(K, V)> { /* ... */ }
}
```

**"so the caller can emit events" is this story.** F-031 built the return value
and deviation D-24 widened it to `Admission<K, V> { evicted, displaced,
refused }` precisely so the three outcomes stay distinguishable to here.

### `docs/hld/14-the-boundary-in-code.md`, section 17.3, the event ring in full

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

|  |
|----|
| *Fixed stride so the JavaScript side is arithmetic, not deserialisation. Drain once per animation frame, surface \`dropped\` in telemetry rather than swallowing it.* |

### `docs/hld/11-decision-log.md`, decision D4

| # | Decision | Rejected | Why |
|---|----------|----------|-----|
| D4 | Events polled from a ring, per frame | A JS callback per event | Removes the hot-path crossing, free coalescing |

### `docs/hld/21-worker-protocol.md`, section 24

```text
render -> main : drained event ring [copy]
```

### What the tree says, which the story title does not

**There is no event ring.** `crates/ocelli-wasm/src/` holds `lib.rs` and
`panic.rs` and nothing else. `ocelli-wasm` depends on `ocelli-core` alone, has
no `Session` type, and exposes four free functions. `scripts/unsafe_allowlist_check.py`
already names `crates/ocelli-wasm/src/ring.rs` as permitted, so the file was
anticipated and never written.

**The boundary is F-101, E16.2 in S16**, "wasm boundary: command channel, event
ring buffer, state readback", four sprints away and depending on F-100's API
design. So the words "surfaced to JS" in this story's title name a channel that
does not exist and is owned by another story. See Open question A.

## What the specification does not cover

1. **Where an eviction event is produced.** Section 20 says the caller emits
   them and does not say who the caller is. This plan makes it the image cache,
   because the cache is what knows a key's meaning and `Lru` deliberately does
   not.
2. **What an eviction event's payload contains.** Section 17.3 fixes the
   envelope, `kind`, `viewport`, `seq` and 32 payload bytes, and says nothing
   about what an eviction puts in those 32 bytes.
3. **What `viewport` means for a cache event.** An eviction is not a viewport
   event. Section 17.3's envelope has the field regardless.
4. **Which numbers `kind` takes.** `ci/error-codes.json` registers error codes
   and there is no event-kind registry. See Open question B.
5. **The decoded frame type.** Section 8 says "decoded frames sit in an LRU
   sized by the caller" and names no type. `ocelli-pixel` owns
   `StoredPixelDescription`, `SampleLayout` and the LUT chain, and no crate owns
   "a decoded frame ready to upload". This story creates it, because it is the
   value the image cache holds.
6. **Whether `seq` is per ring or per event kind.** Section 17.3 gives one
   `seq` field and one ring.

## Approach

### 1. `DecodedFrame`, the decoded tier's value type

```rust
/// One decoded frame, owned by the decoded tier of section 8.
pub struct DecodedFrame {
    description: StoredPixelDescription,
    plane: ImagePlane,
    samples: Vec<u8>,
}

impl Budgeted for DecodedFrame {
    /// The allocated size of the sample buffer, which is what this entry costs
    /// the decoded tier's budget. Section 20's rule, and F-031's own
    /// documentation of it: report what the entry costs, not what it was made
    /// from.
    fn bytes(&self) -> usize { self.samples.len() }
}
```

`bytes` is `samples.len()` and not a computed `rows * columns * bytes_per_pixel`,
because the buffer is the allocation and a computed figure can disagree with it.
A fixture asserts the two agree for a well-formed frame and that `bytes` follows
the buffer when they do not.

### 2. `ImageCache`, the decoded tier with a key that means something

```rust
/// A frame's identity, which is what makes an eviction reportable.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameKey {
    pub series: SeriesId,
    pub instance: u32,
    pub frame: u32,
}

pub struct ImageCache {
    decoded: Lru<FrameKey, DecodedFrame>,
}

impl ImageCache {
    pub fn new(budget: usize) -> Self;
    /// Insert, and return the events the caller must emit.
    pub fn insert(&mut self, key: FrameKey, frame: DecodedFrame) -> CacheEvents;
    pub fn get(&mut self, key: &FrameKey) -> Option<&DecodedFrame>;
    pub fn remove(&mut self, key: &FrameKey) -> Option<DecodedFrame>;
    pub fn pressure(&self) -> Pressure;
}
```

`SeriesId` is a `u64` newtype rather than a `String`, because `FrameKey` is
`Ord` for the `BTreeMap` F-031 built on and a per-frame string comparison on the
decode path is a cost with no reader. The mapping from `SeriesInstanceUID` to
`SeriesId` belongs to whoever owns a study, which is not this story.

### 3. `CacheEvent`, one variant per `Admission` field

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheEvent {
    /// A live entry removed to make room for another.
    Evicted { key: FrameKey, bytes: usize },
    /// A previous value replaced under the same key.
    Displaced { key: FrameKey, bytes: usize },
    /// An entry the budget could not hold, never admitted.
    Refused { key: FrameKey, bytes: usize },
}
```

**Three variants because `Admission` has three fields**, and that is the whole
reason D-24 exists. Collapsing them here throws away what F-031 was built to
preserve, and F-034's telemetry counts evictions, which a replacement would
inflate.

### 4. The section 17.3 encoding, and where it stops

```rust
/// Section 17.3's `Event`, field for field. 48 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct RingEvent {
    pub kind: u32,
    pub viewport: u32,
    pub seq: u64,
    pub payload: [u8; 32],
}

impl CacheEvent {
    /// Encode into section 17.3's envelope.
    pub fn encode(&self, seq: u64) -> RingEvent;
}
```

The payload carries `series` as `u64`, `instance` and `frame` as `u32`, and
`bytes` as `u64`, little-endian, in that order, leaving eight of the thirty-two
reserved and zeroed. **Little-endian is not a choice**: wasm32 is
little-endian and section 17.1 says the wire format is.

`viewport` is `u32::MAX` for a cache event, because an eviction belongs to no
viewport and zero is a valid viewport id. That sentinel is asserted, not
assumed.

### 5. What this story does NOT do, and it is the title's second half

**It writes no ring and it reaches no JavaScript.** See Open question A. The
encoding above is a `#[repr(C)]` struct with a hand-checked layout fixture, and
the thing that carries it across is F-101 in S16.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: **no, and this is the story that could.** A
  `DecodedFrame` holds samples. The event carries a key and a byte count and
  never a sample. Decision D3
- Render-loop allocation: none. `insert` is a decode-path call and
  `CacheEvents` wraps a `Vec` that allocates as `Lru::insert` already does,
  and one insert can produce many events when it evicts many entries
- unsafe: none
- Tier A (WebGPU): n/a. This is the decoded tier, which holds no GPU resource
- Tier B (WebGL2): n/a, same reason
- Tier C (CPU): n/a. The cache behaves identically on every tier, because a
  decoded frame is host memory whatever renders it

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | `DecodedFrame::bytes` is the allocated buffer. A 512 by 512 sixteen-bit frame is 512 × 512 × 2 = 524,288 bytes, hand-computed, and `bytes` follows the buffer rather than the description when a malformed pair disagrees | `crates/ocelli-cache/tests/image_cache.rs` |
| `fixture` | `RingEvent` is **48 bytes** with `kind` at 0, `viewport` at 4, `seq` at 8 and `payload` at 16, asserted as raw bytes against section 17.3's declaration | same |
| `fixture` | An eviction's payload decodes back to the key and byte count it was built from, field by field, little-endian | same |
| `unit` | One insert that evicts two entries yields exactly two `Evicted` events, in the order `Admission::evicted` returned them | same |
| `unit` | A repeated key yields `Displaced` and **not** `Evicted`, and an oversized entry yields `Refused` and not `Evicted`. Three inserts, three different variants, so a collapse to one is caught | same |
| `unit` | `seq` increases by one per event and never repeats across a mixed sequence | same |
| `unit` | `viewport` is `u32::MAX` on every cache event | same |
| `property` | Over a randomised insert and get sequence, the sum of `Evicted` plus `Displaced` plus `Refused` byte counts equals the bytes the cache stopped holding | same |

**Mutation check, HLD 27.3.** Collapse `Displaced` into `Evicted` and the
three-variant test goes red. Report `bytes` from the description rather than the
buffer and the malformed-pair fixture goes red. Reorder two payload fields and
the round-trip fixture goes red. Use `0` as the viewport sentinel and its test
goes red.

## Parity surface covered

`docs/hld/B-parity-surface.md` has a row **Core events, count 50**, "plus 53
tool events, re-shaped rather than copied". This story defines three of the
re-shaped ones and does not deliver an event model. F-103, E16.4 in S17, is
"Event model: the ~103 cornerstone events, re-shaped and documented". The
appendix has no `Covered by` column in this repository to update.

## Deviations

None expected. `bytemuck` is already a dependency of `ocelli-render` from F-041,
and this is its second consumer. `ocelli-cache` gains it, and gains
`ocelli-pixel` for `StoredPixelDescription` and `ImagePlane`.

**If Open question A is answered by building the ring**, that is not a deviation
either: section 17.3 specifies it and `scripts/unsafe_allowlist_check.py`
already permits `crates/ocelli-wasm/src/ring.rs`. It is a scope change and a
dependency edge, and the wasm size budget moves.

## LLD impact

- `docs/lld/cache.md`, which gains the decoded tier's value type, the key and
  the three events
- A new `docs/lld/events.md` if Open question B lands an event-kind registry,
  otherwise the kinds live in `cache.md`

## Open questions

None. All questions were answered in the S12 consolidated design round, as recorded below.

## Decisions from the S12 design round

**A, A1. Stop at the encoding.** F-032 produces `CacheEvent` and its 48-byte `RingEvent`, both tested, and F-101 carries them. The title second half is recorded as not delivered. Shared with F-034 question A, answered once.

**B, B1. No event-kind registry this sprint.** Three constants in `ocelli-cache` with a range comment, shared with F-034 question D. F-103 event model is where a registry belongs if one is wanted.

## Recorded questions

**A. Does this story build section 17.3's event ring, or stop at the encoding?
Shared with F-034.** The title says "surfaced to JS" and there is no channel.
`ocelli-wasm` holds no `Session`, the ring does not exist, and F-101 in S16 owns
"command channel, event ring buffer, state readback".
- **A1, this plan's recommendation.** Stop at the encoding. F-032 produces
  `CacheEvent` and its 48-byte `RingEvent`, both tested, and F-101 carries them.
  A ring with no consumer is a layout nobody can check, which is the argument
  F-029 used to refuse writing section 18.4's uniform before the shader existed.
  The title's "to JS" is recorded as not delivered.
- **A2.** Build `crates/ocelli-wasm/src/ring.rs` now, to section 17.3 exactly.
  It is fully specified, the unsafe allowlist already names the path, and it is
  testable natively with no JavaScript. Costs: `ocelli-wasm` gains a dependency
  on `ocelli-cache`, the wasm size budget moves for the first time, and F-101's
  title then means the binding and the TypeScript reader rather than the ring.
- **A3.** Build the ring in `ocelli-core` instead, where it needs no
  `wasm-bindgen` and the size budget does not move, and let F-101 expose the
  pointer. Departs from section 17.3's stated path and needs a `D-NN` row.

**B. Do event kinds get a registry, like error codes?** `ci/error-codes.json`
exists because HLD section 23 requires stable numeric codes agreeing across
Rust, TypeScript and a registry, and the `errors` gate asserts it. Section 17.3
gives `kind: u32` and no such requirement.
- **B1, this plan's recommendation.** No registry this story. Three constants in
  `ocelli-cache` with a range comment, and F-103's event model is where a
  registry belongs if one is wanted. A registry with three entries and no second
  language to agree with is a gate that asserts a file against itself.
- **B2.** Add `ci/event-kinds.json` and a gate now, matching the error-code
  shape, so the numbers are stable before anything reads them.
