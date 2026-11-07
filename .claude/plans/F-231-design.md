# F-231, The wasm session's own decoded frame cache, with residency and release exports

**Status**: approved
**Epic ref**: E5.9
**Sprint**: S46
**Estimate**: 4w

Proposed backlog row, to be filed by the lead under "M3, Cache and the render core" in `docs/sprints/BACKLOG.md` and in `docs/sprints/allocation.json` with `epic_key` E5, `epic` "E5 Cache & memory", milestone M3, sprint S46, `depends_eids` ["E16.6", "E5.8", "E5.7"], `depends_fids` ["F-230", "F-226", "F-224"]:

```text
| F-231 | E5.9 | S46 | Session frame cache: the wasm session keeps committed stack frames in a caller-budgeted ImageCache checked by ContentId, reports eviction, pressure and release through the ring, and exports a residency query, a show-from-cache call, and releaseSeries and releaseStudy in @ocelli/core | Rust+TS | 4w | F-230, F-226, F-224 | pending |
```

E5.9 follows F-224's E5.7 and F-226's E5.8. Allocation note to carry: "The operator's answer to F-226 Q1 on 2026-11-05, option A: a separate ~4w story giving the wasm session its own decoded frame cache, with residency and release exports, added to S46. Runs last of F-230, F-226, F-224 and F-231."

## Normative source, transcribed

### The operator's answer, relayed by the coordinator on 2026-11-05

Option A, verbatim: "File a separate ~4w story giving the wasm session its own decoded frame cache, with residency and release exports, and add it to S46 too."

### HLD section 8, `docs/hld/06-memory-and-cache.md`, first paragraph

|  |
|----|
| One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs. |

### HLD section 20, `docs/hld/17-cache-and-allocation.md`

```rust
pub trait Budgeted { fn bytes(&self) -> usize; }
impl<K, V: Budgeted> Lru<K, V> {
    /// Returns the entries evicted to make room, so the caller can emit events.
    pub fn insert(&mut self, k: K, v: V) -> Vec<(K, V)> { /* ... */ }
}
```

The first bullet: "**No allocation in the render loop.** Pre-size everything at viewport creation. A frame that allocates is a frame that can stutter." D-24 widens `insert` to `Admission`.

### HLD section 17.3, `docs/hld/14-the-boundary-in-code.md`

```rust
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
| Fixed stride so the JavaScript side is arithmetic, not deserialisation. Drain once per animation frame; surface `dropped` in telemetry rather than swallowing it. |

### Decisions D2 to D5, `docs/hld/11-decision-log.md`

| D2 | wasm-bindgen in one crate only | Bindings wherever convenient | Phases 2 and 3 are entry points, not rewrites |
|----|----|----|----|
| D3 | Pixels never cross the boundary | Decode in wasm, render in JS | The main architectural gain |
| D4 | Events polled from a ring, per frame | A JS callback per event | Removes the hot-path crossing; free coalescing |
| D5 | Single-threaded, one instance per worker | wasm-bindgen-rayon from the start | Stays on stable Rust |

### What the session caches today, read from `crates/ocelli-wasm/src/session.rs` at `8729656`

- One `Session` per wasm instance, in a `thread_local!` `RefCell<Option<Session>>` inside `mod bind`, created on first use with `StackViewport::try_identity(1)`.
- `last_frame: Vec<u8>`, the bytes of the last accepted commit, and `stored_samples: Vec<Stored>`, the same bytes read as little-endian `f32` when their length is a multiple of 4. `remember_frame` clears and refills both on every accepted commit, so the displayed frame costs 8 bytes per sample, and nothing else is kept. The CPU raster draws from `stored_samples`.
- `configure_stack` installs 16-byte `FrameKey`s, bumps `stack_epoch` and calls `clear_selected_source`, which clears both buffers. `commit_selected_frame(ticket, len, meta, index, epoch)` accepts a frame only for the selected index of the current epoch.
- `ring: EventRing`, capacity 256, with `push(event)`, `push_presented` and `copy_pending`. `drain_events` copies it out. Only kind 6 is produced by the session today. `grep -rn 'ImageCache' crates/ocelli-wasm` prints nothing.

### What F-224 and F-226 provide, from their approved plans

F-224: `ContentId::of(sop_instance_uid, transfer_syntax_uid, encoded)`, FNV-1a 64 over the length-prefixed fields. `ImageCache::insert(key, content, frame)`. A lookup that presents the identity, answers `Hit`, `Miss` or `Stale { bytes }`, and removes a mismatched entry with no event. F-226: `Lru::remove_range`, `ImageCache::release_series(series) -> CacheEvents` with one `CacheEvent::Released { series, frames, bytes }`, ring kind 7, decoded by `@ocelli/core` as `cache.released`.

### F-034's event rules, `docs/lld/cache.md`

"The caller emits the returned evictions first and a `BudgetChanged` event after them." Pressure bands sit at one half and nine tenths of the tier's budget, compared as `used * 2` against `budget` and `used * 10` against `budget * 9`. "`PressureBand::crossing` returns one `TelemetryEvent::Pressure` when the band has changed and nothing when it has not. The caller keeps the last reported band."

### Wasm size, `ci/wasm-size-budget.json`

`"bytes": 2151196`, `"tolerance": 0.05`, and the note "Re-baseline deliberately with --accept-size and say why in the design plan."

## What the specification does not cover

Every item is answered: lead decision under the operator's delegation of 2026-11-05.

1. **The cached value is a committed frame, not a `DecodedFrame`.** The session receives stored samples and `columns` and `rows`, with no `StoredPixelDescription` and no `ImagePlane`. Building a `DecodedFrame` would mean fabricating a geometry, which F-216 refused for the same reason. So `ocelli-cache` gains `CommittedFrame { columns: u32, rows: u32, samples: Vec<Stored> }`, and `ImageCache` becomes `ImageCache<V: Budgeted = DecodedFrame>`. That generic has two instantiations today, `DecodedFrame` and `CommittedFrame`, which `AGENTS.md`'s rule requires, and it is what lets the session reuse F-226's `release_series`, F-032's event conversion and F-224's stale rule rather than a second cache. F-224's `get(&FrameSource)` stays on `impl ImageCache<DecodedFrame>`. `impl ImageCache<CommittedFrame>` gets `get(key, &CommittedSource)` and `is_resident(key, &CommittedSource)`, where `CommittedSource { content: ContentId, columns: u32, rows: u32 }`. `ocelli-cache` gains a path dependency on `ocelli-core` for `Stored`, which `ocelli-pixel` already depends on, so no new external crate enters the module.
2. **`Budgeted::bytes` for a committed frame is `samples.len() * 4`,** computed with `checked_mul` on `size_of::<Stored>()` and saturating at `usize::MAX`, so a 512 by 512 monochrome frame is 1,048,576 bytes. That is the frame's allocation, matching `DecodedFrame`'s rule of counting the buffer.
3. **The budget is the caller's, and the default is 0, which is off.** With a budget of 0 the session inserts nothing and emits no cache event, so a caller that never sets a budget gets today's behaviour exactly. `set_frame_budget(bytes)` moves it through `Lru::set_budget` and emits each eviction and then one `BudgetChanged`, F-034's order. The budget is a `u32` byte count, because wasm32 linear memory cannot exceed 4 GiB.
4. **What fills the cache.** Only `commit_cached_frame`, a new export that is `commit_selected_frame` plus a `content: u64`. The frame is keyed by the stack's `FrameKey` at `index` and inserted with `ContentId(content)`. A commit without a content id, through the existing `commit_frame` or `commit_selected_frame`, is displayed and not cached, because the session cannot know which bytes it came from. A commit whose length is not a multiple of 4 is displayed as today and not cached, since `stored_samples` is empty for it.
5. **The displayed frame stays outside the budget.** The CPU raster reads `stored_samples`, and pinning an LRU entry would need a policy `Lru` does not have. So the displayed frame keeps its two buffers, and a cached copy sits in the cache. A frame that is both displayed and cached costs 12 bytes per sample, 8 outside the budget and 4 inside it. `show_resident` copies a cached frame into `stored_samples` and writes its little-endian bytes into `last_frame`, so `last_frame` keeps meaning "the bytes of the displayed source".
6. **A stack change keeps the cache.** `configure_stack` still clears the displayed frame, but cached frames survive, which is the point of the story: returning to a series shows it without a commit. A frame is never served for other bytes, because every lookup presents a `ContentId` (F-224).
7. **Residency is a query that is not a use.** `frame_resident(index, epoch, content, columns, rows)` answers 1 only when the entry under the stack key at `index` matches all three. It does not move recency and does not remove a mismatch, because a query is telemetry. `show_resident` with the same arguments is the use: a match copies and displays, a mismatch is F-224's stale removal, and a miss returns 0 and changes nothing displayed.
8. **The `ContentId` producer is Rust, once.** `@ocelli/core`'s `contentId(wasm, uid, transferSyntax, encoded)` writes `uid`, `transferSyntax` and `encoded` back to back into the bulk slot and calls a new export `content_id(ticket, len, uid_len, ts_len)`, which hashes through `ContentId::of` and drops the slot. That uses the raw-bytes channel of the boundary and returns a `u64`, so no pixel crosses up and no second FNV-1a is written in TypeScript. A return of `undefined` is a refusal, because `0` is a legitimate id.
9. **Release.** `release_series(series: u64)` calls F-226's `ImageCache::release_series`, pushes its `cache.released`, and returns the frame count. `releaseStudy(wasm, series)` in TypeScript is the caller's list of series released one export call each, with duplicates skipped and the counts summed, because the cache holds no study identity, which F-226 decision 1 records. A study crosses once per series, never once per frame. A release that removes the displayed frame's cached copy leaves the displayed frame on screen, because the display is outside the cache.
10. **Events.** Each insert's `CacheEvents` are pushed in their order: `Displaced`, then evictions, then `Refused`. After every insert, release, budget change or stale removal the session compares the pressure band with the last one it reported and pushes one `TelemetryEvent::Pressure` on a crossing. `request_frame_pressure()` pushes a reading on demand. `EventRing` gains `push_with_seq(|seq| RingEvent)`, which assigns the sequence number as `push_presented` does, so every event has one sequence space. A burst larger than the free ring space increments `dropped`, which the shell already reads, and nothing is swallowed.
11. **Threads and instances.** D5 holds unchanged: the cache lives in the one `Session` of the render worker's wasm instance, behind the existing `thread_local!` `RefCell`. Decode workers have their own instances and no session cache.
12. **Size budget.** The module gains a second `ImageCache` instantiation, `BTreeMap` operations over `FrameKey`, the FNV-1a fold, F-034's encoders and nine exports. The plan expects a growth far inside the 5% tolerance, 107,559 bytes on 2,151,196, and claims no number. `bin/ocelli.sh wasm` is run on the parent commit and on the story's tree and both are recorded. A growth outside the tolerance stops the story for a plan amendment that names the cause, and `--accept-size` is not run without one.

## Problem and reproducer

Source identity: HEAD `8729656`, plus F-230, F-226 and F-224 as integrated.

1. A shell scrolls a 100-frame stack forward and back. Every return to an index needs the frame committed again, because `remember_frame` keeps only the last one. The bytes cross into linear memory again each time.
2. Switching to another series and back clears the displayed frame and keeps nothing, so the whole stack must be committed again.
3. After F-226 a shell can decode `cache.released`, and no shipped session produces it. `releaseSeries` and `releaseStudy` do not exist in `@ocelli/core`.

```bash
bin/ocelli.sh test ocelli-wasm session_cache
npx vitest run packages/core/src/frame-cache.test.ts
```

Both are new, so they fail today on missing items. The semantic RED, recorded in the progress note: on today's session, committing index 0, then index 1, then selecting index 0 again leaves `stored_samples` holding index 1's samples until a new commit arrives, and nothing can answer whether index 0 is held.

## Scope, API and compatibility

In scope: the session cache in `ocelli-wasm`, the `ImageCache<V>` generalisation and `CommittedFrame` in `ocelli-cache`, one ring helper in `ocelli-viewport`, and `@ocelli/core`'s calls over the new exports. Non-goals: a decode path in the session, GPU texture residency, prefetch policy (F-047's radius stays the shell's), encoded-byte caching, and freeing wasm linear memory, which never shrinks.

### Rust, `ocelli-cache`

```rust
pub struct CommittedFrame { columns: u32, rows: u32, samples: Vec<Stored> }
impl CommittedFrame {
    pub fn new(columns: u32, rows: u32, samples: Vec<Stored>) -> Self;
    pub fn columns(&self) -> u32;
    pub fn rows(&self) -> u32;
    pub fn samples(&self) -> &[Stored];
}
impl Budgeted for CommittedFrame { fn bytes(&self) -> usize } // samples.len() * 4

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommittedSource { pub content: ContentId, pub columns: u32, pub rows: u32 }

pub struct ImageCache<V: Budgeted = DecodedFrame> { /* Lru<FrameKey, held value with ContentId> */ }
impl<V: Budgeted> ImageCache<V> {
    pub fn new(budget: usize) -> Self;
    pub fn insert(&mut self, key: FrameKey, content: ContentId, value: V) -> CacheEvents;
    pub fn set_budget(&mut self, budget: usize) -> CacheEvents;  // evictions only, F-034 order
    pub fn release_series(&mut self, series: SeriesId) -> CacheEvents; // F-226
    pub fn contains(&self, key: &FrameKey) -> bool;
    pub fn remove(&mut self, key: &FrameKey) -> Option<V>;
    pub fn pressure(&self) -> Pressure;
    pub fn len(&self) -> usize;
}
impl ImageCache<DecodedFrame> { pub fn get(&mut self, key: &FrameKey, source: &FrameSource) -> FrameLookup<'_, DecodedFrame>; } // F-224
impl ImageCache<CommittedFrame> {
    pub fn get(&mut self, key: &FrameKey, source: &CommittedSource) -> FrameLookup<'_, CommittedFrame>;
    pub fn is_resident(&self, key: &FrameKey, source: &CommittedSource) -> bool;
}
```

`FrameLookup` gains the value type parameter. Its three variants are unchanged.

### Rust, `ocelli-viewport/src/ring.rs`

```rust
impl EventRing { pub fn push_with_seq(&mut self, event: impl FnOnce(u64) -> RingEvent); }
```

### Rust, `ocelli-wasm/src/session.rs`, `Session` methods and their exports in `mod bind`

| `Session` method | Export, `js_name` | Returns |
|---|---|---|
| `set_frame_budget(bytes: u32)` | `set_frame_budget` | `1`, or `0` with no session |
| `commit_cached_frame(ticket, len, meta, index, epoch, content: u64)` | `commit_cached_frame` | `1` committed, `0` refused as `commit_selected_frame` refuses |
| `frame_resident(index, epoch, content, columns, rows)` | `frame_resident` | `1` or `0` |
| `show_resident(index, epoch, content, columns, rows)` | `show_resident` | `1` shown, `0` miss, stale or wrong epoch |
| `release_series(series: u64)` | `release_series` | frames released |
| `request_frame_pressure()` | `request_frame_pressure` | `1` |
| `content_id(ticket, len, uid_len, ts_len)` | `content_id` | `Option<u64>`, `undefined` on refusal |
| `frame_cache_pressure() -> Pressure` | none, native tests only | |

Every export is a `#[wasm_bindgen]` free function in `crates/ocelli-wasm/src/session.rs`'s `mod bind`, which is D2. `show_resident` adopts the frame exactly as `commit_current_frame` does after its checks: `remember` from the cached samples, `last_meta` with generation 0, `set_loading(false)`, and `adopt_cpu_source(0)`.

### TypeScript, `packages/core/src/frame-cache.ts` (new) and `bulk.ts`

```ts
export interface FrameCacheExports {
  set_frame_budget(bytes: number): number;
  frame_resident(index: number, epoch: number, content: bigint, columns: number, rows: number): number;
  show_resident(index: number, epoch: number, content: bigint, columns: number, rows: number): number;
  release_series(series: bigint): number;
  request_frame_pressure(): number;
}
export interface ResidentFrameRequest {
  readonly index: number; readonly epoch: number; readonly content: bigint;
  readonly columns: number; readonly rows: number;
}
export function setFrameBudget(wasm: FrameCacheExports, bytes: number): void;
export function frameResident(wasm: FrameCacheExports, request: ResidentFrameRequest): boolean;
export function showResident(wasm: FrameCacheExports, request: ResidentFrameRequest): boolean;
export function releaseSeries(wasm: FrameCacheExports, series: bigint): number;
export function releaseStudy(wasm: FrameCacheExports, series: readonly bigint[]): number;
export function requestFramePressure(wasm: FrameCacheExports): void;

// bulk.ts, the file allowed to view linear memory
export function contentId(wasm: SessionBulkExports & { content_id(ticket: number, len: number, uidLen: number, tsLen: number): bigint | undefined },
  sopInstanceUid: string, transferSyntaxUid: string, encoded: Uint8Array): bigint;
```

`FrameCommitMeta` gains `content?: bigint`. `sessionBulkSink.commitAllocated` routes a meta with `stackIndex`, `stackEpoch` and `content` to `commit_cached_frame`. Range errors are F-230's `OcelliError` `InvalidValue`, and `ResidentFrameRequest` joins F-230's key lists and census, as does `FrameCommitMeta`'s new key.

Compatibility: every existing export, its arguments and its return are unchanged. A caller who never calls `set_frame_budget` sees today's behaviour, AC3. `ImageCache`'s default type parameter keeps `ImageCache::new` callers compiling. Under 0.x, recorded in `CHANGELOG.md`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Shell commit with a content id | `commit_cached_frame` | Selected index of the current epoch only | Displayed as today, cached when the budget admits it |
| Shell scroll back, series return | `frame_resident`, then `show_resident` | The shell keeps the content id per slot | Drawn with no bytes crossing |
| Shell leaving a series or study | `releaseSeries`, `releaseStudy` | Study is the caller's list of series | `cache.released` per series |
| Shell budget control | `setFrameBudget` | Default 0, off | Evictions then `cache.budgetChanged` |
| Ring drain | `drain_events` | Once per frame, D4 | kinds 1 to 5 and 7 from a shipped session for the first time, and 6 as today |
| `@ocelli/react` `OcelliSession` | unchanged exports | Does not call the new ones | Unchanged |
| CPU raster | `stored_samples` | Unchanged | The shown frame |
| `contentId` | `content_id`, `ContentId::of` | Bytes down the bulk slot | A `bigint` |

Units: bytes everywhere, as `Budgeted::bytes` counts them. The owner of admission and eviction is `Lru`. The owner of band reporting is the session's `last_band`.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Budget 0 | `commit_cached_frame` | displayed, nothing cached, no event | as today |
| Budget B, stack epoch E | `commit_cached_frame(index i, content c)` | displayed and inserted under `keys[i]` | a wrong epoch or index refuses and caches nothing |
| `keys[i]` cached with `c` | `frame_resident(i, E, c, cols, rows)` | `1`, recency unchanged | none |
| Same | `frame_resident(i, E, d, ...)` | `0`, entry kept | none |
| Same | `show_resident(i, E, c, ...)` | shown, entry most recently used | displayed frame replaced only on success |
| Same | `show_resident(i, E, d, ...)` | stale removal, `0` | displayed frame unchanged |
| Same | `configure_stack` with the same keys | new epoch E2, display cleared | cache kept, `frame_resident(i, E2, c, ...)` is `1` |
| Series s cached | `release_series(s)` | removed, `cache.released` | displayed frame unchanged |
| Budget B with entries | `set_frame_budget(smaller)` | LRU evictions, then `BudgetChanged`, then a band event if crossed | none |
| Ring nearly full | a burst | events pushed until full, the rest counted in `dropped` | none swallowed |

Invariants: `frame_cache_pressure().used` equals the sum of cached frames' `bytes` and never exceeds the budget. No lookup returns a frame whose `ContentId`, `columns` or `rows` differ from the request. The displayed frame changes only on a commit or a successful `show_resident`.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Bulk slot for a cached commit | `alloc`, ticket | ticket | `commit_cached_frame` returns 1 | Refusal releases the slot as `commit_selected_frame` does, nothing is inserted |
| Cached frame | `ImageCache::insert` | `FrameKey` plus `ContentId` | `Lru::insert` admitted | Refused by size: `cache.refused`, display unaffected. A repeat commit of the same content displaces with equal samples |
| Shown frame | `show_resident` | `FrameKey`, epoch | `stored_samples` replaced | A miss or stale leaves the previous display |
| Content id slot | `contentId` | ticket | `content_id` returns | A refused export drops the slot, and `contentId` throws `OcelliError` `CommitRefused` |
| Instance poisoned by a panic | HLD section 23 | instance | none | The cache dies with the instance, and the shell rebuilds from its own state as section 23 requires |

The `alloc` of a cached commit, the `remember` copy and the insert all precede the reply, and none is in a frame callback. A repeated `show_resident` of the frame already shown copies again and bumps the generation, which is what a repeated commit does today.

## Numbered acceptance inventory

Byte figures are `columns * rows * samples_per_pixel * 4`, from decision 2. Band edges are F-034's integer comparisons.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `CommittedFrame` of 512 by 512 monochrome is 1,048,576 bytes. 4 by 4 interleaved RGB, 48 samples, is 192 bytes | `ocelli-cache` unit test | Counting `last_frame` and `stored_samples` both, or bytes per sample 1 | implementation |
| AC2 | Budget 3,145,728 and four 512 by 512 cached commits at indices 0 to 3. `used` reads 1,048,576, 2,097,152, 3,145,728, 3,145,728. Events in order: none, `cache.pressure` Elevated (`2 * 2,097,152 >= 3,145,728`, `10 * 2,097,152 < 9 * 3,145,728`), `cache.pressure` Critical (`31,457,280 >= 28,311,552`), then `cache.evicted` for index 0's key with 1,048,576 bytes and no pressure event | `session_cache` native test over `Session`, reading `copy_ring` | Pressure re-sent with no crossing. Evicting index 3. An event before the insert completes | implementation |
| AC3 | With no `set_frame_budget`, every existing session test passes unchanged and `copy_ring` holds only kind 6 after cached commits | existing `session.rs` tests plus one new case | A default budget that caches and emits | implementation |
| AC4 | `frame_resident` is 1 for the matching triple and 0 for another content, other columns or other rows. It changes neither recency nor `used`: after it, the next eviction is still the same key | `session_cache`, `residency_is_not_a_use` | A query that marks use or removes a mismatch | implementation |
| AC5 | `show_resident` of index 1 after index 2 is displayed puts index 1's samples in `stored_samples` and their little-endian bytes in `last_frame`, clears loading, and `draw_committed_mono` draws them. A mismatch removes the entry, returns 0 and leaves index 2 displayed | `session_cache`, `show_resident_displays_and_stale_refuses` | Showing a stale frame. Replacing the display before the check | implementation |
| AC6 | `configure_stack` with the same keys keeps the cache, and residency under the new epoch is 1. Under the old epoch it is 0 | `session_cache` | A stack change that clears the cache, or one that accepts an old epoch | implementation |
| AC7 | Budget 4,194,304 holding three 1,048,576-byte frames of series 5 and one of series 6, `used` 4,194,304, band Critical. `release_series(5)` returns 3, `used` drops to 1,048,576, and the ring holds one `cache.released` with frames 3 and bytes 3,145,728, then one `cache.pressure` Low (`2 * 1,048,576 < 4,194,304`). The displayed frame is unchanged | `session_cache` | One event per frame. Clearing the display | implementation |
| AC8 | `set_frame_budget(1,048,576)` with three frames cached evicts the two least recently used, emits two `cache.evicted`, then one `cache.budgetChanged` with budget 1,048,576, evicted 2 and evicted bytes 2,097,152, then a band event only if it crossed | `session_cache` | `BudgetChanged` before the evictions, against F-034's order | implementation |
| AC9 | A commit whose length is not a multiple of 4, and a commit through `commit_selected_frame` with no content, are displayed as today and not cached | `session_cache` | Caching an empty `stored_samples` | implementation |
| AC10 | A frame larger than the whole budget is displayed, refused by the cache with one `cache.refused`, and `used` unchanged | `session_cache` | Admitting above budget, against D-24 | implementation |
| AC11 | 300 events in one burst into an empty ring push 256 and count 44 in `dropped`, which `drain_events` reports | `ocelli-viewport` ring test plus a session case | Dropping silently or overwriting old events | implementation |
| AC12 | `content_id` over `"1.2.3"`, `"1.2.840.10008.1.2.1"` and `[0, 1, 2, 3]` returns `0x49e8b650ff1cd3f1n`, F-224's AC2 value, through `contentId` on the built module | `packages/core/src/boundary.wasm.test.ts`, the suite that loads the built wasm | A TypeScript re-implementation, or a slot that is not dropped | implementation, built target |
| AC13 | The same native scenarios as AC2, AC4, AC5 and AC7 through the built wasm exports and `@ocelli/core`'s functions, with bigint series and content | `boundary.wasm.test.ts`, appended | A `u64` argument truncated to `u32` at the export | implementation, built target |
| AC14 | `releaseStudy(wasm, [5n, 6n, 5n])` calls `release_series` twice and returns the summed frames | `frame-cache.test.ts` with a stub | Calling a series twice, or a per-frame crossing | implementation |
| AC15 | Bad inputs: a non-`u32` index, a negative budget, a `bigint` series outside `u64`, an unknown key in `ResidentFrameRequest` throw F-230's `OcelliError` with `InvalidValue` or `UnknownOption` before any export call | `frame-cache.test.ts` | Passing the bad value to wasm | implementation |
| AC16 | `decodeNamedEvent` names every kind the session now emits, and `cache.evicted`'s `CORNERSTONE_EVENT_MAP` row gains a live producer note | `events.test.ts`, `scripts/event_catalogue_check.py` | A producer field that still says no shipped session | implementation |
| AC17 | `bin/ocelli.sh wasm` on the parent commit and on this story's tree, both recorded. The growth is within the 5% tolerance of `ci/wasm-size-budget.json`, or the story stops for a plan amendment | the two runs in the review evidence | A silent `--accept-size` | implementation, release build |
| AC18 | `ci/check-bindgen-isolation.sh` passes, so `wasm-bindgen` is still only in `ocelli-wasm` | the script | A binding outside `ocelli-wasm` | implementation |
| AC19 | No export returns pixel data. The only new return types are `u32` and `Option<u64>` | review of `mod bind` and the generated `.d.ts` | A `Vec<u8>` of samples | review |
| AC20 | README, `use.html` and `more.html` describe the session cache, its default off, and the release calls | review | Undocumented behaviour | implementation, then review |

## Approach

1. `ocelli-cache`: `CommittedFrame`, `CommittedSource`, `ImageCache<V>` with the generic block and the two value-specific blocks, `set_budget` returning `CacheEvents`, and `FrameLookup<'a, V>`. `Cargo.toml` adds `ocelli-core` by path.
2. `ocelli-viewport/src/ring.rs`: `push_with_seq`.
3. `ocelli-wasm/src/session.rs`: a `frames: ImageCache<CommittedFrame>`, `last_band`, the methods of the table, and a private `emit(&CacheEvents)` and `emit_band()`. `remember_frame` is split so `show_resident` can call the same copy. The exports in `mod bind`.
4. `@ocelli/core`: `frame-cache.ts`, `contentId` in `bulk.ts`, `FrameCommitMeta.content`, `sessionBulkSink` routing, `index.ts` exports and `PUBLISHED`, key lists.
5. Tests, the two `bin/ocelli.sh wasm` runs, CHANGELOG, LLD and the three user documents.

Rejected: a second cache type in `ocelli-wasm`, which duplicates release, events and the stale rule. A `DecodedFrame` with a fabricated plane. Pinning the displayed entry inside the `Lru`. A TypeScript FNV-1a. One `cache.released` per frame. A default budget above zero, which would change every current caller's memory without asking.

## Dependencies, write ownership and resources

Declared dependencies: **F-230**, **F-226** and **F-224**.

Order among the four, recommended to the lead: **F-230, then F-226, then F-224, then F-231.**

- F-230 first, because it owns the coded error class and the key lists this story's TypeScript refusals and `ResidentFrameRequest` use, and because F-224 adds a key to its lists.
- F-226 before F-224 and F-231, because it adds `Lru::remove_range`, `release_series` and kind 7, which this story's release export pushes, and its folder viewer forget route is what F-224 calls.
- F-224 before F-231, because this story takes F-224's `ContentId` on every lookup. With F-231 first, the session cache would ship a lookup with no content check, which is the stale-image defect F-224 removes, and F-224 would then have to change a shipped export's arguments.
- F-231 last, because it generalises `ImageCache` after both earlier stories have changed it, so `image.rs` is rewritten once more rather than merged three ways.

F-230 may start now. F-226 waits for F-216 and F-208. This story shares no folder viewer file, so it does not wait for any viewer story beyond its three dependencies.

Write set:

- `crates/ocelli-cache/Cargo.toml`, `src/image.rs`, `src/lib.rs`, `tests/image_cache.rs`
- `crates/ocelli-viewport/src/ring.rs`, `tests/ring.rs`
- `crates/ocelli-wasm/src/session.rs`
- `packages/core/src/frame-cache.ts` (new), `frame-cache.test.ts` (new), `bulk.ts`, `bulk.test.ts`, `options.ts`, `events.ts`, `index.ts`, `index.test.ts`, `boundary.wasm.test.ts`
- `ci/wasm-size-budget.json` only if a plan amendment approves a rebaseline
- `CHANGELOG.md`, `README.md`, `examples/folder-viewer/src/use.html`, `examples/folder-viewer/src/more.html`
- `docs/lld/cache.md`, `docs/lld/events.md`, `docs/lld/public-api.md`, `docs/lld/build-targets.md`, at `/complete-feature`

Files without concurrent writers: `image.rs` and `lib.rs` after F-224, `session.rs` with any story touching the session, `bulk.ts` and `options.ts` after F-230. Exclusive resource: the release `wasm-pack` build for AC17, run from the staged tree in this story's own target directory.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-cache
bin/ocelli.sh test ocelli-cache
bin/ocelli.sh test ocelli-viewport --test ring
bin/ocelli.sh test ocelli-wasm
bin/ocelli.sh cargo check -p ocelli-wasm --target wasm32-unknown-unknown
bin/ocelli.sh wasm
npm run build
npx vitest run packages/core
bash ci/check-bindgen-isolation.sh
python3 scripts/event_catalogue_check.py
```

Final: `bin/ocelli.sh gate --sprint`. Mutations, each applied, observed red, reverted: let `frame_resident` call `get` (AC4), skip the content check in `get` (AC5), clear the cache in `configure_stack` (AC6), emit `BudgetChanged` first (AC8), truncate the `u64` series to `u32` at the export (AC13), default the budget to 64 MiB (AC3). Evidence is invalidated by any change in the write set, by F-224's `ContentId` definition, or by F-226's event layout. AC17's figures are valid for the tree they were built from only.

## Boundary and tier

- wasm-bindgen: `ocelli-wasm` only, in `session.rs`'s `mod bind` (D2)
- Pixels across the boundary: no (D3). Encoded bytes cross down the bulk slot for `content_id`, which is the raw-bytes channel. Cached frames move inside linear memory. Every new export returns `u32` or `Option<u64>`
- Events: polled from the existing ring once per frame (D4), with no callback
- Threads: one session in one instance per worker, behind the existing `thread_local!` (D5)
- Render-loop allocation: none. Inserts, copies and releases happen on commit, show and release calls. `show_resident` reuses the capacity of `stored_samples` and `last_frame`
- unsafe: none
- Tier A (WebGPU): full for host residency. The cache holds host samples, and the session's draw path is the CPU raster on every tier today, so no texture is cached and no GPU residency changes
- Tier B (WebGL2): full, for the same reason
- Tier C (CPU): full. `show_resident` feeds `stored_samples`, which the CPU raster already draws, with arithmetic unchanged

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | AC1 byte accounting | `crates/ocelli-cache/src/image.rs`, `tests/image_cache.rs` |
| unit | AC2 to AC10, hand-computed `used`, band edges and event order | `crates/ocelli-wasm/src/session.rs` tests, `session_cache` |
| unit | AC11 ring overflow | `crates/ocelli-viewport/tests/ring.rs` |
| browser | AC12, AC13 on the built module | `packages/core/src/boundary.wasm.test.ts` |
| unit | AC14, AC15 TypeScript calls and refusals | `packages/core/src/frame-cache.test.ts` |
| contract | AC16 catalogue | `packages/core/src/events.test.ts`, the script |
| contract | AC17, AC18 size and binding isolation | `bin/ocelli.sh wasm`, `ci/check-bindgen-isolation.sh` |

No pixel or geometry arithmetic changes. The shown samples are copied, not computed, so no DICOM-section fixture is owed.

## Parity surface covered

The "Core events" row of `docs/hld/B-parity-surface.md`. `IMAGE_CACHE_IMAGE_REMOVED` and `CACHE_SIZE_EXCEEDED`, already mapped to `cache.evicted` and `cache.pressure`, get a shipped producer for the first time. No row's count changes.

## Deviations

None. Section 8's caller-sized LRU, section 20's `Lru` and D-24 are used as they are. A size rebaseline, if AC17 needs one, is a plan amendment and not a deviation.

## LLD impact

- `docs/lld/cache.md`: `ImageCache<V>`, `CommittedFrame`, and a section "The session's decoded tier" with the default-off budget, what fills it, residency against use, and the displayed frame outside the budget.
- `docs/lld/events.md`: kinds 1 to 5 and 7 now have a shipped producer, and the event order of an insert, a release and a budget change.
- `docs/lld/public-api.md`: the `frame-cache.ts` calls, `contentId`, and `FrameCommitMeta.content`.
- `docs/lld/build-targets.md`: the new exports and the AC17 size figures.

## User-facing documentation impact

- `README.md`, "Use the library": one paragraph that the core can keep committed frames in a cache whose budget the caller sets, off by default, and that `releaseSeries` and `releaseStudy` empty it.
- `examples/folder-viewer/src/use.html`, 5.5 "Bytes into the core": a sample of `contentId`, a cached commit, `frameResident` and `showResident`. 5.3 "The event ring": the session now produces the cache kinds. 12 "Cache and volume": `CommittedFrame` beside `DecodedFrame`.
- `examples/folder-viewer/src/more.html`, `#worker`: "Not built: ... its frame cache" becomes the built session cache, with decode in the worker still not built.

## Open questions

None open.

- Q1, the cached value. Answered: lead decision under the operator's delegation of 2026-11-05. `CommittedFrame` in a generic `ImageCache<V>`, decision 1.
- Q2, the default budget. Answered: lead decision under the operator's delegation of 2026-11-05. 0, off, decision 3.
- Q3, the displayed frame. Answered: lead decision under the operator's delegation of 2026-11-05. Outside the budget, decision 5.
- Q4, residency and use. Answered: lead decision under the operator's delegation of 2026-11-05. A query that is not a use, and `show_resident` that is, decision 7.
- Q5, where `ContentId` is computed. Answered: lead decision under the operator's delegation of 2026-11-05. In Rust through `content_id`, decision 8.
- Q6, study release. Answered: lead decision under the operator's delegation of 2026-11-05. The caller's list of series, decision 9.
- Q7, order. Answered: lead decision under the operator's delegation of 2026-11-05. F-230, F-226, F-224, F-231, recorded under Dependencies.

### Lead rulings after review pass 1

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-231-working-pass-1.md`.

- **`show_resident` shows only the selected slice.** An index that is not the viewport's selected index is refused with the same `Selection` refusal `commit_selected_frame` gives, and nothing is drawn. AC5 and AC13 are corrected to that.
- **A content id needs its stack position.** A commit that carries `content` without both `stackIndex` and `stackEpoch` is refused with the coded error, so a content id is never accepted and then ignored, and `content` is always validated as a `u64`.
- **Pressure readings carry real counts.** `evicted`, `evicted_bytes` and `refused` are the counts since the previous reading, kept by F-034's `TierCounters`, never a constant zero.

Lead ruling after review pass 2: in `@ocelli/core`, `showResident` throws the coded `CommitRefused` error for a `Selection` refusal, as `writeFrame` does, and returns false only for a miss, so a caller can tell the two apart.
