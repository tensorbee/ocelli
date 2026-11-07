# F-226, Release by series and by study, a released event, and the folder viewer forgetting the folder it left

**Status**: approved
**Epic ref**: E5.8
**Sprint**: S46
**Estimate**: 3w

Proposed backlog row, to be filed by the lead under "M3, Cache and the render core" in `docs/sprints/BACKLOG.md` and in `docs/sprints/allocation.json` with `epic_key` E5, `epic` "E5 Cache & memory", milestone M3, sprint S46, `depends_eids` ["E38.26", "E38.18"], `depends_fids` ["F-216", "F-208"]:

```text
| F-226 | E5.8 | S46 | Release by series and study: ImageCache::release_series and BrickCache::release_volume over one Lru range removal, a cache.released ring event decoded by @ocelli/core, and the folder viewer forgetting the previous folder's held files and frames on the local process when another folder or the synthetic study is opened or the page is left | All | 3w | F-216, F-208 | pending |
```

E5.8 follows F-224's proposed E5.7. If the lead files them in the other order, the two numbers swap and nothing else changes. Allocation note to carry: "From the legacy Cornerstone issue list the operator shared on 2026-11-05, item 1, a cache never cleared on leaving a study. The folder viewer half runs after F-216 and F-208 integrate. F-122 in S48 owns the residency policy that may call the brick release this story adds."

## Normative source, transcribed

### HLD section 8, `docs/hld/06-memory-and-cache.md`, first paragraph

|  |
|----|
| One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs. |

### HLD section 20, `docs/hld/17-cache-and-allocation.md`

```rust
impl<K, V: Budgeted> Lru<K, V> {
    /// Returns the entries evicted to make room, so the caller can emit events.
    pub fn insert(&mut self, k: K, v: V) -> Vec<(K, V)> { /* ... */ }
}
```

The first bullet, verbatim: "**No allocation in the render loop.** Pre-size everything at viewport creation. A frame that allocates is a frame that can stutter." D-24 widens `insert` to `Admission`.

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

Decision D4: "Events polled from a ring, per frame".

### HLD section 30, `docs/hld/26-differentiating-capabilities.md`, the bullets F-122 owns

|  |
|----|
| **Residency** is the view frustum intersected with the current level of detail, recomputed per frame against a byte budget. GPU memory becomes a configured number rather than a function of series size. |
| **Bricks** are 128³, or 64³ for sixteen-bit data, addressed through a 3D indirection texture. The cache is ocelli-cache with a brick key — no second cache. |

### The ring kinds today, `docs/lld/events.md` and `crates/ocelli-viewport/src/boundary.rs`

| Kind | Name | Payload |
|------|------|---------|
| 1 | `cache.evicted` | series `u64`, instance `u32`, frame `u32`, bytes `u64` |
| 2 | `cache.displaced` | the same key |
| 3 | `cache.refused` | the same key |
| 4 | `cache.pressure` | tier `u8`, used `u64`, budget `u64`, entries `u32`, evicted `u32`, refused `u32` |
| 5 | `cache.budgetChanged` | tier `u8`, budget `u64`, evicted bytes `u64`, evicted `u32`, dropped `u32` |
| 6 | `frame.presented` | generation `u32`, drawn width `u32`, drawn height `u32`, flags `u32` |

`RING_EVENT_KINDS: [(&str, u32); 6]` in `boundary.rs` is the source `crates/ocelli-viewport/tests/typegen.rs` writes into `packages/core/src/generated.ts`. `docs/lld/events.md`: "Kind 0 and any kind above 6 do too" decode as null.

### The code this story changes, read from the tree at `8729656` and the F-216 worktree

`crates/ocelli-cache/src/image.rs`: `SeriesId`'s doc, "Assigning an id to a UID belongs to whoever owns a study, which is not this crate." `FrameKey` derives `Ord` over `series`, then `instance`, then `frame`, so one series is one contiguous range. `ImageCache::remove` is "Explicitly release a frame. Not an eviction, and emits no event." There is no per-series or per-study release.

`crates/ocelli-cache/src/lru.rs`: `Lru` holds a `BTreeMap<K, Entry<V>>` and exposes `insert`, `get`, `remove`, `contains_key`, `pressure`, `len`, `is_empty`, `budget`, `set_budget` and `would_admit`. No range removal.

`crates/ocelli-cache/src/brick.rs`: `BrickKey` derives `Ord` over `volume`, `level`, `brick`. `BrickCache` exposes `insert`, `get`, `remove`, `pressure`, `missing` and `is_resident`. `crates/ocelli-volume/src/lib.rs`: `VolumeCache` is keyed by `VolumeKey { series }`, so `VolumeCache::remove` is already a per-series release.

`crates/ocelli-wasm/src/session.rs`: the session holds `last_frame: Vec<u8>` and `stored_samples: Vec<Stored>` for one viewport, and `configure_stack` calls `clear_selected_source`, which clears both. `grep -rn 'ImageCache' crates/ocelli-wasm` prints nothing. **`@ocelli/core` and its wasm session hold no decoded tier**, so `cache.evicted` to `cache.budgetChanged` are decoded by `events.ts` and produced by no shipped session. `examples/folder-viewer/src/more.html` `#worker` says the same: "Not built: decoding that envelope in the worker, its frame cache and a render of that frame."

F-216 worktree, `examples/folder-viewer/src/held.rs`: `enum HeldKey { File(FileId), Frame(FileId, u32) }` derives `Ord`, so every `Frame(id, n)` of one id is contiguous. `Viewer` holds `Mutex<CacheState>` with `lru`, `files` and `frames`. `preview_source` can take a file's bytes out of the cache for a decode, and `restore_file` and `store_frame` put bytes or a frame back after it. `serve.rs` routes `GET /held`, `POST /preview`, `/file`, `/inspect` and `/headers`. `page.html` `readFolder` calls `dropCache()` for the page caches, and no route tells the process to drop the old folder's files, which stay until LRU eviction.

## What the specification does not cover

1. **Study membership is the caller's.** The cache has no study identity, and `SeriesId`'s doc gives the UID mapping to "whoever owns a study". So Rust gets `ImageCache::release_series(series)`, and releasing a study is its owner calling that for each of its series, plus `VolumeCache::remove` and `BrickCache::release_volume` for its volumes. A `release_study(&[SeriesId])` in the cache would be a loop that only forwards, which `AGENTS.md`'s structural rules refuse, and a `StudyId` in `FrameKey` would change three wire formats for a field only release reads. Answered: lead decision under the operator's delegation of 2026-11-05.
2. **One range removal in `Lru`.** `Lru::remove_range<R: RangeBounds<K>>(&mut self, range: R) -> Vec<(K, V)>` removes every live key in the range in ascending key order, releases their bytes from `used`, and does not move the use tick. It is explicit release, not eviction, so it is `remove`'s rule applied to a range. `ImageCache::release_series`, `BrickCache::release_volume` and the folder viewer's forget all use it, so there is one release mechanism. It allocates the returned vector, which is a release call and not a render-loop call. Answered: lead decision under the operator's delegation of 2026-11-05.
3. **One `cache.released` event per series released, not one per frame.** A 1,693-frame series released per frame would put 1,693 events in a ring drained once per frame, and HLD section 17.3 treats `dropped` as a fault. `CacheEvent::Released { series, frames, bytes }`, kind 7, payload series `u64` at 0, frames `u32` at 8, a zero `u32` at 12, bytes `u64` at 16, and eight zero bytes at 24, little-endian as section 17.1 requires. `viewport` is `u32::MAX` as for every cache event. A release that finds nothing emits nothing. `frames` saturates at `u32::MAX`. Volume and brick releases return a `ReleaseSummary { entries, bytes }` and have no ring kind, because no volume or brick event exists today and `docs/lld/events.md` already says "Volume-cache removal is not frame-cache eviction". Answered: lead decision under the operator's delegation of 2026-11-05.
4. **`cache.released` has no cornerstone3D 5.8.2 counterpart.** 5.8.2 reports a removed image per image as `IMAGE_CACHE_IMAGE_REMOVED`, which `events.ts` already pairs with `cache.evicted`. A per-series release is a different observation, so it joins `UNMAPPED_EVENTS` with that reason, under `docs/lld/events.md`'s rule that "pairing one of them with a different cornerstone event would name the wrong observation". Answered: lead decision under the operator's delegation of 2026-11-05.
5. **The forget route.** `POST /forget` with a body of held ids, one `<generation>.<serial>` per line, LF separated, at most 65,536 lines and 4 MiB. Every line is parsed with `FileId::parse` before anything is removed, so a malformed line is `400` with `X-Ocelli-Error: forget` and forgets nothing. For each id the route removes `HeldKey::File(id)` and the range `HeldKey::Frame(id, 0)..=HeldKey::Frame(id, u32::MAX)`, and updates `files` and `frames`. The reply is `200`, `application/json`, `{"files":F,"frames":N,"bytes":B,"unknown":U}`, where `unknown` counts ids that held nothing, including another generation's ids and a repeated id. An `X-Ocelli-File` header on `/forget` is `400` `header`. An empty body is `200` with four zeros. Answered: lead decision under the operator's delegation of 2026-11-05.
6. **A forget wins against a decode already in flight.** `preview_source` can hand a file's bytes to a decode that later calls `restore_file` or `store_frame`, which would put a forgotten id back. `CacheState` gains `in_flight: BTreeMap<FileId, u32>`, counted up when bytes leave the cache for a decode and down when the decode returns, and `doomed: BTreeSet<FileId>`. A forget of an id with a non-zero count adds it to `doomed`. `restore_file` and `store_frame` for a doomed id return the value to their own request and insert nothing. The id leaves `doomed` when its count reaches zero, so both maps are bounded by the requests in flight, which F-216 caps at 32 connections. Answered: lead decision under the operator's delegation of 2026-11-05.
7. **When the page forgets.** On every new open, folder or synthetic study, the page sends the previous open's ids. On `pagehide` it sends the current open's ids with `navigator.sendBeacon`, in chunks of at most 1,700 ids, because Chromium caps a beacon body at 64 KiB and 1,700 ids of at most 34 bytes is 57,800 bytes. A forget that fails is not retried, because F-216's budget still bounds what the process holds. Answered: lead decision under the operator's delegation of 2026-11-05.
8. **No re-post for an old open.** F-216's page re-posts a file once on `410`. After a forget, a late reply for the previous open would re-post the old file and undo the forget. `ensureFile` re-posts only a `File` that belongs to the current open, held in a `WeakSet<File>` replaced on each open. Answered: lead decision under the operator's delegation of 2026-11-05.
9. **The TypeScript release API belongs to F-231.** The shipped `@ocelli/core` session has nothing per series to release today. The operator answered Q1 with option A on 2026-11-05, so F-231 gives the wasm session its own decoded frame cache with residency and release exports, and `releaseSeries` and `releaseStudy` in `@ocelli/core` are F-231's. F-226's TypeScript half is `cache.released`, its decoder and its catalogue row, and nothing else.

## Problem and reproducer

Source identity: HEAD `8729656` for Rust and TypeScript. F-216 as integrated for the folder viewer.

1. Rust: a caller holding a study's frames in `ImageCache` and leaving it must call `remove` once per `FrameKey`, so it must keep every key it ever inserted. Nothing reports what left. `BrickCache` is the same per brick.
2. Folder viewer: open folder A of 1,693 files, then folder B. `GET /held` still reports A's files and frames, up to the 256 MiB budget, until B's inserts evict them one LRU step at a time. The process keeps a folder the user has left.
3. A late `/preview` reply for folder A after B is open answers `410` and F-216's page re-posts A's file, putting it back.

```bash
bin/ocelli.sh test ocelli-cache --test image_cache release
node examples/folder-viewer/tests/browser/forget.mjs
```

Both are new tests, so they fail today on missing items. The semantic RED is recorded in the progress note: on F-216's page and server, step 2's `GET /held` after opening B still counts A's files.

## Scope, API and compatibility

In scope: `ocelli-cache` release, the ring kind and its TypeScript decoder, the folder viewer's forget route and page calls. Non-goals: a residency policy for bricks, which is F-122. A volume or brick ring event. Releasing anything in the wasm session, and the TypeScript release functions, which are F-231. Freeing wasm linear memory, which never shrinks.

### Rust

```rust
// crates/ocelli-cache/src/lru.rs
impl<K: Ord + Clone, V: Budgeted> Lru<K, V> {
    /// Explicit release of every live key in `range`, ascending. Not an eviction.
    pub fn remove_range<R: core::ops::RangeBounds<K>>(&mut self, range: R) -> Vec<(K, V)>;
}

// crates/ocelli-cache/src/image.rs
pub const KIND_RELEASED: u32 = 7;
pub enum CacheEvent {
    Evicted { key: FrameKey, bytes: usize },
    Displaced { key: FrameKey, bytes: usize },
    Refused { key: FrameKey, bytes: usize },
    /// Every frame of one series, released by the caller.
    Released { series: SeriesId, frames: u32, bytes: usize },
}
impl ImageCache {
    /// Release every frame of `series`. One `Released` event, or none when nothing was held.
    pub fn release_series(&mut self, series: SeriesId) -> CacheEvents;
}

// crates/ocelli-cache/src/brick.rs
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReleaseSummary { pub entries: usize, pub bytes: usize }
impl BrickCache {
    /// Release every brick of every level of `volume`.
    pub fn release_volume(&mut self, volume: VolumeId) -> ReleaseSummary;
}
```

`release_series` uses the range `FrameKey::new(series, 0, 0)..=FrameKey::new(series, u32::MAX, u32::MAX)`. `release_volume` uses `BrickKey { volume, level: 0, brick: [0, 0, 0] }..=BrickKey { volume, level: u32::MAX, brick: [u32::MAX; 3] }`. `bytes` is the sum of `Budgeted::bytes` of what left, so `pressure().used` drops by exactly that. `CacheEvent::encode` gains the `Released` arm. `lib.rs` re-exports `KIND_RELEASED` and `ReleaseSummary`. `RING_EVENT_KINDS` in `crates/ocelli-viewport/src/boundary.rs` gains `("released", KIND_RELEASED)` and its length becomes 7, and `generated.ts` is regenerated with `UPDATE_TYPES=1 cargo test -p ocelli-viewport --test typegen`.

### TypeScript, `packages/core/src/events.ts`

`OCELLI_EVENTS` gains `"cache.released"`. `decodeNamedEvent` decodes kind 7 into `{ name: "cache.released", payload: CacheReleasedPayload }` with `interface CacheReleasedPayload { readonly series: bigint; readonly frames: number; readonly bytes: bigint }`, refusing a payload shorter than 24 bytes as null like the other readers. `UNMAPPED_EVENTS` gains the row of decision 4. Kinds 0 and above 7 decode as null. No TypeScript release function is added here. F-231 adds `releaseSeries` and `releaseStudy` over the session cache it builds.

### Folder viewer

`serve.rs` routes `("POST", "/forget")`. `held.rs` gains `Viewer::forget(&self, ids: &[FileId]) -> Forgotten { files, frames, bytes, unknown }` and the `in_flight` and `doomed` bookkeeping. `page.html` gains `openIds: Set<string>` and `openFiles: WeakSet<File>` per open, `forgetIds(ids)` with a `fetch` for an open and `sendBeacon` chunks for `pagehide`, and the guard in `ensureFile`. `GET /held` is unchanged and is the observation.

Compatibility: every existing ring kind, payload and route is byte-identical. `RING_EVENT_KINDS` grows by one entry, and a consumer that treats an unknown kind as null, which `decodeNamedEvent` does, is unaffected. Under 0.x, recorded in `CHANGELOG.md` `## Unreleased`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| A Rust study owner leaving a study | `ImageCache::release_series` per series, `VolumeCache::remove`, `BrickCache::release_volume` | The owner knows the study's series and volumes | `CacheEvents` with one `Released` per series, `ReleaseSummary` per volume |
| F-122 residency policy, S48 | `BrickCache::release_volume`, `Lru::remove_range` | Decides when, per frame. Must not add a second release path | Its own ACs |
| Folder viewer `/forget` | `Lru::remove_range` over `HeldKey` | Ids from the page | JSON counts, `GET /held` drops |
| Folder viewer in-flight decode | `restore_file`, `store_frame` | Doomed ids not inserted | The reply is still served |
| `decodeNamedEvent` | `RING_EVENT_KIND.released` | Ring copy | `cache.released` payload |
| F-224, after this story | `forgetIds` | A changed file's id | The server drops the stale bytes |
| F-231, after F-224 | `ImageCache::release_series`, `KIND_RELEASED`, `decodeNamedEvent` | The wasm session's own frame cache and its `releaseSeries` and `releaseStudy` exports | `cache.released` from a shipped session |
| `examples/folder-viewer` thumbnails, prefetch, main view | F-216's controllers | Aborted on a new open, as today | No re-post for the old open |

The owner of release accounting is `Lru::remove_range`. The owner of the released event is `ImageCache::release_series`. The owner of when the viewer forgets is the page.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Series 5 holds 3 frames of 32 bytes, series 6 holds 1 | `release_series(SeriesId(5))` | 3 removed, one `Released { series 5, frames 3, bytes 96 }`, `used` 32 | series 6 untouched, its recency unchanged |
| Same | `release_series(SeriesId(5))` again | nothing removed, no event | unchanged |
| Series 7 holds nothing | `release_series(SeriesId(7))` | no event | unchanged |
| Volume 2 holds bricks on levels 0 and 1, volume 3 holds bricks | `release_volume(VolumeId(2))` | every level of volume 2 removed | volume 3 untouched |
| Folder A open, ids held | open folder B | page sends A's ids, `/forget` removes A's files and frames | B's posts proceed |
| A decode for A's id in flight | `/forget` of that id | id doomed, nothing inserted, the request answers with its frame, or 410 Gone after a wait for grey room, id leaves `doomed` | `GET /held` never counts it again |
| A late `410` for A's file after B is open | `ensureFile(fileA)` | refused, no post | A stays forgotten |
| Page open | `pagehide` | beacons in chunks of at most 1,700 ids | a failed beacon leaves entries to age out |
| `/forget` body with one malformed line | request | `400 forget` | nothing removed |

Invariants: `pressure().used` equals the sum of live entries' admitted bytes after every operation, which F-031's property test asserts and which this story extends to `remove_range`. `files` and `frames` in `GET /held` equal the live `HeldKey::File` and `HeldKey::Frame` counts after every forget.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Released frames | `ImageCache::release_series` | `SeriesId` | the returned `CacheEvents` | A repeat releases nothing and emits nothing |
| Released bricks | `BrickCache::release_volume` | `VolumeId` | the returned summary | A repeat returns zeros |
| Forgotten files and frames | `Viewer::forget` under the cache lock | `FileId` | the `200` reply | A repeat counts the id as `unknown`. A malformed body forgets nothing |
| Doomed id | `Viewer::forget` | `FileId`, while its in-flight count is non-zero | the decode's return | A crash under `panic = "abort"` ends the process, and F-216's restart path applies |
| Page open ids | `readFolder`, `openSynthetic` | the open's generation | the forget request | Replaced on the next open |

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `Lru::remove_range` over keys 1 to 9, range `3..=6`, values of 1, 2, 4, 8, 16, 32, 64, 128 and 256 bytes, budget 1,000: returns keys 3, 4, 5 and 6 in order with 4, 8, 16 and 32 bytes, `used` goes 511 to 451, and the next eviction order of the survivors is unchanged | `crates/ocelli-cache/src/lru.rs` unit test, hand-computed | Releasing a range by evicting it, which moves the tick. Forgetting to subtract `used` | implementation |
| AC2 | Property: random sequences of `insert`, `get`, `remove`, `set_budget` and `remove_range` keep `used` equal to the sum of live entries and at or below `budget` | `crates/ocelli-cache/tests/invariants.rs`, the new operation added | A range removal that double-releases a key | implementation |
| AC3 | The state table's first three rows for `release_series`, with the event's fields and `used` 128 to 32 | `crates/ocelli-cache/tests/image_cache.rs`, `release_series_releases_one_series` | A range that bleeds into series 6 because it ends at `(series + 1, 0, 0)` inclusive | implementation |
| AC4 | `Released { series: SeriesId(0x0102_0304_0506_0708), frames: 0xA0B0_C0D0, bytes: 524_288 }` encodes to kind 7, viewport `u32::MAX`, payload `08 07 06 05 04 03 02 01 D0 C0 B0 A0 00 00 00 00 00 00 08 00 00 00 00 00` then eight zeros | `image.rs` unit test, the same hand-written bytes style as `handoff_tests` | Big-endian, or bytes at offset 12 | implementation |
| AC5 | `decodeNamedEvent` of AC4's 32 payload bytes under kind 7 gives `cache.released` with series `0x0102030405060708n`, frames `2695938256`, bytes `524288n`. A 23-byte payload is null. Kind 8 is null | `packages/core/src/events.test.ts` | Reading frames as `u64`. Decoding kind 8 | implementation |
| AC6 | `RING_EVENT_KINDS` has 7 entries with 7 distinct kinds, and the regenerated `generated.ts` holds `released: 7,` | `boundary.rs` test, `tests/typegen.rs` | A hand edit of `generated.ts` | implementation |
| AC7 | `cache.released` is in `OCELLI_EVENTS` and `UNMAPPED_EVENTS`, `CORNERSTONE_EVENT_MAP` is unchanged, and `python3 scripts/event_catalogue_check.py` passes | `events.test.ts`, the script | Pairing it with `IMAGE_CACHE_IMAGE_REMOVED` | implementation |
| AC8 | `release_volume` releases levels 0 and 1 of volume 2 and leaves volume 3, with `ReleaseSummary` bytes equal to the sum of the bricks' host buffer lengths | `crates/ocelli-cache/tests/bricks.rs` | A range over level 0 only | implementation |
| AC9 | Release allocates only on the release call, never on `get` or `insert` paths it does not touch | review against `lru.rs`, and AC1's test asserting `get` after release does not change `len` beyond the removal | A release folded into `get` | review |
| AC10 | `/forget` with two held ids, one other-generation id and one repeat: reply counts files, frames and bytes hand-summed from what was held, `unknown` 2, and `GET /held` drops by exactly those | `examples/folder-viewer/tests/preview.rs`, `forget_drops_files_and_frames` | Removing the file but not its frames | implementation |
| AC11 | A malformed line forgets nothing and answers `400 forget`. An `X-Ocelli-File` header answers `400 header`. An empty body answers four zeros | `preview.rs`, `forget_refusals` | Removing the valid lines before the bad one | implementation |
| AC12 | A forget during an in-flight decode: hold the decode at a test barrier, forget the id, release the barrier. The reply is served, `GET /held` counts no file and no frame for the id, and `doomed` is empty afterwards | `held.rs` unit test with the existing `with_workspace` hook | A forget that the decode's `store_frame` undoes | implementation |
| AC13 | Browser: open synthetic folder A, draw, open folder B. `GET /held` reports only B's files and frames, and A's ids answer `410` | `examples/folder-viewer/tests/browser/forget.mjs` in F-208's `viewer-browser` gate | A page that drops its own caches only, which is today's `readFolder`. RED on F-216's page | integrated |
| AC14 | Browser: delay one of A's `/preview` replies with a Playwright route on the page side, open B, let the reply answer `410`. No `/file` post of A's file follows | same script, counting `/file` bodies | F-216's unconditional `410` re-post | integrated |
| AC15 | Browser: `pagehide` sends `/forget` beacons with at most 1,700 ids each, and a 3,400-file synthetic folder sends exactly 2 | same script, `page.on("request")` | One beacon over 64 KiB, which Chromium refuses | integrated |
| AC16 | F-216's `viewer.open` and `viewer.resident_memory` thresholds still hold, and `viewer.resident_memory` after opening two folders in sequence reports the server holding at most the second folder plus the F-216 bound | `bin/ocelli.sh bench viewer.open viewer.resident_memory` at sprint verification on the F-216 host | A forget that blocks the open | external, sprint verification |
| AC17 | Q1 as answered: `packages/core/src/index.ts` exports no `releaseSeries` or `releaseStudy` from this story, and `cache.released` is the only TypeScript addition besides its payload type | `index.test.ts` `PUBLISHED` list unchanged, because `CacheReleasedPayload` is a type and not a runtime export, and review of the TypeScript diff | A TypeScript release function with no provider, which would report success while releasing nothing | implementation, then review |
| AC18 | README, `use.html` and `more.html` describe release and the forget | review | an unrecorded behaviour | implementation, then review |

## Approach

1. `lru.rs`: `remove_range`, collecting the keys of `self.entries.range(range)` and removing each through the existing private release so `used` stays exact.
2. `image.rs`: `KIND_RELEASED`, the `Released` variant and its encode arm, `release_series`. `brick.rs`: `ReleaseSummary`, `release_volume`. `lib.rs` re-exports.
3. `boundary.rs`, then regenerate `generated.ts`. `events.ts` and its test.
4. `held.rs`: `in_flight`, `doomed`, `Viewer::forget`. `serve.rs`: the route and its refusals. `page.html`: open ids, the forget calls, the `ensureFile` guard.
5. Tests, the browser script, CHANGELOG, LLD and the three user documents.

Rejected alternatives: per-frame `Released` events, decision 3. A `StudyId` in `FrameKey` or a study map in the cache, decision 1. A `forget all` route, which would drop another tab's files and send it through `410` re-posts. Forgetting by generation, which is per process and not per open.

## Dependencies, write ownership and resources

Declared dependencies: **F-216** and **F-208**. F-216 because the forget route, `HeldKey` and `GET /held` are its code. F-208 because the browser rows run on its `viewer-browser` harness and gate rather than a second harness. Source-level: F-210 and F-211 extend `HeldFrame` in `held.rs`, so this story lands after them in the S46 order to avoid a second merge of the same struct.

Order, recommended to the lead: F-216, F-210, F-211, F-208, then this story, then F-224, then F-209 and the rest. F-231 follows F-224 and builds the session cache and the TypeScript release functions on this story's `release_series` and event. F-224 depends on this story because both write `crates/ocelli-cache/src/image.rs`, `lib.rs` and `page.html`'s open path, and F-224's changed-file path calls `forgetIds`. F-230 shares no file with this story.

F-122 overlap: F-122 is pending in S48 and writes the brick residency policy of HLD section 30. This story gives it the primitive, `release_volume` over `remove_range`, and no policy. F-122's plan must call these and not add a second release path, which this plan records so its design round reads it.

Write set:

- `crates/ocelli-cache/src/lru.rs`, `image.rs`, `brick.rs`, `lib.rs`, `tests/invariants.rs`, `tests/image_cache.rs`, `tests/bricks.rs`
- `crates/ocelli-viewport/src/boundary.rs`, `tests/typegen.rs`, `packages/core/src/generated.ts` (regenerated)
- `packages/core/src/events.ts`, `events.test.ts`, `index.ts` (the payload type)
- `examples/folder-viewer/src/held.rs`, `serve.rs`, `page.html`, `tests/preview.rs` (appended), `tests/browser/forget.mjs` (new)
- `CHANGELOG.md`, `README.md`, `examples/folder-viewer/src/use.html`, `examples/folder-viewer/src/more.html`
- `docs/lld/cache.md`, `docs/lld/events.md`, at `/complete-feature`

Files without concurrent writers: `image.rs` and `lib.rs` with F-224, every folder viewer file with every viewer story. The browser script binds `OCELLI_PORT=0` and stops the process by PID. Ports 8742 and 8765 are never used.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-cache
bin/ocelli.sh test ocelli-cache
bin/ocelli.sh clippy ocelli-cache
UPDATE_TYPES=1 cargo test -p ocelli-viewport --test typegen
bin/ocelli.sh test ocelli-viewport
npx vitest run packages/core/src/events.test.ts
python3 scripts/event_catalogue_check.py
bin/ocelli.sh test ocelli-folder-viewer
bin/ocelli.sh gate viewer-browser
bin/ocelli.sh native
```

Final: `bin/ocelli.sh gate --sprint`, including the bench rerun of AC16. Mutations, each applied, observed red, reverted: end the series range at `(series + 1, 0, 0)` inclusive (AC3), skip the `used` release in `remove_range` (AC1, AC2), let `store_frame` ignore `doomed` (AC12), remove the `ensureFile` guard (AC14), send one beacon for every id (AC15). Evidence is invalidated by any change in the write set or to F-216's `HeldKey` order.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. `cache.released` carries a series, a count and a byte total
- Render-loop allocation: none. `remove_range` allocates its return on a release call
- unsafe: none
- Tier A (WebGPU): full for `release_volume`, which releases host bricks. No texture is touched, and destroying uploaded textures stays F-043's and F-122's
- Tier B (WebGL2): full, the same host code
- Tier C (CPU): full. The decoded tier and the folder viewer process are host memory on every tier

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | AC1 range removal, hand-computed bytes | `crates/ocelli-cache/src/lru.rs` |
| property | AC2 accounting invariant with `remove_range` | `crates/ocelli-cache/tests/invariants.rs` |
| unit | AC3, AC8 series and volume release | `crates/ocelli-cache/tests/image_cache.rs`, `tests/bricks.rs` |
| contract | AC4 to AC7 event wire and catalogue | `image.rs`, `boundary.rs`, `tests/typegen.rs`, `packages/core/src/events.test.ts` |
| contract | AC10 to AC12 forget route and race | `examples/folder-viewer/tests/preview.rs`, `src/held.rs` |
| browser | AC13 to AC15 | `examples/folder-viewer/tests/browser/forget.mjs` |

No pixel or geometry arithmetic, so no DICOM-section fixture is owed.

## Parity surface covered

The "Core events" row of `docs/hld/B-parity-surface.md`, 50 events, "re-shaped rather than copied". This story adds one Ocelli event with no 5.8.2 counterpart and changes no mapped row, so the count of mapped 5.8.2 events is unchanged.

## Deviations

None. Section 17.3 lists no kinds, so kind 7 fills an unspecified slot. Section 20 and D-24 are unchanged.

## LLD impact

- `docs/lld/cache.md`: a section "Release" with `remove_range`, `release_series`, `release_volume`, why study membership is the caller's, and the F-122 boundary. The folder viewer paragraph gains `/forget`, the in-flight rule and when the page forgets.
- `docs/lld/events.md`: kind 7 in the ring kinds table, its payload, its `UNMAPPED_EVENTS` reason, and "above 7" for null.

## User-facing documentation impact

- `README.md`, the folder viewer list: one item that opening another folder or the synthetic study releases the previous folder's files from the local process, and that leaving the page does the same.
- `examples/folder-viewer/src/use.html`, 5.3 "The event ring": "six kinds" becomes seven and names `cache.released`. 12 "Cache and volume": a short sample of `release_series` and `release_volume`, and one sentence that a study is released by its owner's series.
- `examples/folder-viewer/src/more.html`, `#worker`: one sentence that `cache.released` reports a Rust release, and that the worker's own frame cache and its release calls arrive with F-231.

## Open questions

- **Q1, the TypeScript release API.** Answered by the operator on 2026-11-05, relayed by the coordinator: option A, "File a separate ~4w story giving the wasm session its own decoded frame cache, with residency and release exports, and add it to S46 too." That story is F-231, `.claude/plans/F-231-design.md`. F-226's TypeScript half is `cache.released`, its decoder and its catalogue row. `releaseSeries` and `releaseStudy` in `@ocelli/core` are F-231's, built on this story's `ImageCache::release_series`. F-231 depends on this story.
- Q2, study release in Rust. Answered: lead decision under the operator's delegation of 2026-11-05. Per series by the owner, decision 1.
- Q3, one event per series. Answered: lead decision under the operator's delegation of 2026-11-05. Decision 3.
- Q4, the forget route, its race and its timing. Answered: lead decision under the operator's delegation of 2026-11-05. Decisions 5 to 8.
- Q5, order. Answered: lead decision under the operator's delegation of 2026-11-05. After F-208, before F-224, recorded under Dependencies.

### Lead amendment after review pass 2

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-226-working-pass-2.md`. AC3's fixture holds the state table's first three rows plus a frame at each end of the series range, so the range ends are tested. Its expected frames and bytes follow from that fixture and are hand-computed in the test, and they are not the state table's figures.

### Lead amendments for the folder viewer half, against F-209's tree

Made by the sprint lead on 2026-11-07 from read-only groundwork against the sprint head after F-209. Where these differ from the text above, these win. Line numbers are re-measured at implementation.

- **There is no `restore_file`.** `preview_source` hands out shared posted bytes. The in-flight count is taken where `preview_source` hands out the shared bytes, and only `store_frame_with` checks the doomed set. Decision 6, Approach step 4 and the state table row for an in-flight decode are restated on that basis.
- **The forget goes through the existing removal path.** It removes through `forget_frame` and `reclaim_held`, so `frame_order`, `files`, `frames` and the free buffer lists stay consistent. A test forgets a file with frames held and then shows a later store and eviction behave as if the file had never been held.
- **One forget path.** The production forget replaces the test-only `forget_file`, and its tests move to the production route.
- **New refusals are claimed.** Any refusal the `/forget` route adds is registered with `scripts/guards/catalogue.py` and the census stays at zero unwatched.
- **The new browser check is wired** into `bin/ocelli.sh viewer-browser` and its `--list` description, beside `aspect.mjs`, `readout.mjs` and `link.mjs`.
- **Pass 4's carried note** on `RingEvent::kind`'s doc is fixed if it is still untrue.

### Lead ruling after the viewer half's AC15 measurement

Made by the sprint lead on 2026-11-07. The implementer measured in Chromium that the 64 KiB keepalive limit is on all keepalive bytes in flight from the page, not on each beacon, so decision 7's two beacons of 1,700 ids can never both arrive, and a 3,400-file folder left 1,700 files held.

- **Leaving the page forgets by page, in one short beacon.** Each page load makes a random page token and sends it with every `/file` post. The server records the token beside each held file. On `pagehide` the page sends one beacon naming its token, and the server forgets every file posted under it, through the same removal path as `/forget`. Another page's files are untouched. Opening another folder still forgets by id, through an ordinary request with no keepalive limit.
- **AC15 is restated.** A 3,400-file synthetic folder sends exactly one beacon on `pagehide`, under 1 KiB, after which `GET /held` counts none of that page's files, while a second page's held files are unchanged. Sending one beacon per id goes red, and so does forgetting by token without matching the token.
- **The token is not trusted for more than its own files.** A malformed token is refused with `400 forget`. An unknown token forgets nothing and answers four zeros.
- **Content type.** `/forget` replies with the same JSON content type as the other JSON routes.
