# F-224, Frame identity: a decoded frame is never served for bytes it was not decoded from

**Status**: approved
**Epic ref**: E5.7
**Sprint**: S46
**Estimate**: 3w

Proposed backlog row, to be filed by the lead under "M3, Cache and the render core" in `docs/sprints/BACKLOG.md` and in `docs/sprints/allocation.json` with `epic_key` E5, `epic` "E5 Cache & memory", milestone M3, sprint S46, `depends_eids` ["E16.6", "E5.8"], `depends_fids` ["F-230", "F-226"]:

```text
| F-224 | E5.7 | S46 | Frame identity: the decoded tier answers a lookup only when the frame's content id, from SOP Instance UID, transfer syntax and encoded bytes, and its description and plane match, DicomwebClient takes a documented cache mode defaulting to no-cache, and the folder viewer keys page caches by the picked File, lets the same folder be chosen again, and refuses to show a file that changed on disk | All | 3w | F-230, F-226 | pending |
```

E5.7 is the next free number in E5, whose last rows are F-034 E5.4, F-035 E5.5 and F-036 E5.6, and `allocation.json` holds no E5 entry above E5.6. The row sits in M3 as F-222 sits in M14 with sprint S46. Allocation note to carry: "From the legacy Cornerstone issue list the operator shared on 2026-11-05, item 2, a stale image after a file was re-processed. Runs after F-230, which owns the DicomwebClient option list, and after F-226, which owns the folder viewer forget route and the other half of `image.rs`."

## Normative source, transcribed

### HLD section 8, `docs/hld/06-memory-and-cache.md`, first paragraph

|  |
|----|
| One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs. |

### HLD section 20, `docs/hld/17-cache-and-allocation.md`

```rust
pub trait Budgeted { fn bytes(&self) -> usize; }
pub struct Lru<K, V: Budgeted> {
    budget: usize,
    used: usize,
    /* ... */
}
impl<K, V: Budgeted> Lru<K, V> {
    /// Returns the entries evicted to make room, so the caller can emit events.
    pub fn insert(&mut self, k: K, v: V) -> Vec<(K, V)> { /* ... */ }
}
```

Deviation D-24 widens the return to `Admission<K, V>` with `evicted`, `displaced` and `refused`. That is the shape `crates/ocelli-cache/src/lru.rs` ships.

### HLD section 13, `docs/hld/10-extension-points.md`

|  |
|----|
| A SeriesSource trait abstracts where bytes come from. Phase 1 implements DICOMweb; Phase 2 adds a DIMSE-backed implementation without touching anything above it. |

The identity this story adds is computed from bytes and metadata the caller already holds, so it does not depend on which source delivered them.

### HLD section 10, `docs/hld/07-concurrency-and-typescript.md`

"Framework bindings, DICOMweb fetch and authentication" stays TypeScript. The fetch cache mode is therefore a shell option.

### The code this story changes, read from the tree at `8729656`

`crates/ocelli-cache/src/image.rs`, lines 23 to 45:

```rust
/// A series identity inside a `FrameKey`.
///
/// A `u64` rather than a `SeriesInstanceUID` string, because `FrameKey` is
/// `Ord` for the `BTreeMap` `Lru` is built on and a per-frame string comparison
/// on the decode path is a cost with no reader. Assigning an id to a UID
/// belongs to whoever owns a study, which is not this crate.
pub struct SeriesId(pub u64);

/// One frame's identity, which is what makes an eviction reportable.
pub struct FrameKey {
    pub series: SeriesId,
    pub instance: u32,
    pub frame: u32,
}
```

`ImageCache` holds `Lru<FrameKey, DecodedFrame>`. `ImageCache::get(&mut self, key: &FrameKey) -> Option<&DecodedFrame>` returns whatever frame sits under the key. `DecodedFrame` carries `description: StoredPixelDescription`, `plane: ImagePlane` and `samples: Vec<u8>`, and both carried types derive `PartialEq`. `StackSequence::samples_for_upload` in `crates/ocelli-viewport/src/scroll.rs` calls `cache.get(&key)` for the displayed key.

`FrameKey` crosses three wire formats unchanged by this story: the 16-byte stack key `configure_stack` reads in `crates/ocelli-wasm/src/session.rs`, the 24-byte `HandoffMeta` image, and the cache event payload of `CacheEvent::encode`.

`packages/core/src/dicomweb.ts` line 256 calls `this.#fetch(url, { method: "GET", headers: { Accept: accept }, ...signal })`. No `cache` member, so the request uses the Fetch `"default"` mode.

`examples/folder-viewer/src/page.html` keys its grey cache through `fileIdentity(file)`, which is `path + "\0" + file.size + "\0" + file.lastModified`, and `readFolder` calls `dropCache()` on every open. F-216's worktree (`ocelli-wt-f-216`, `examples/folder-viewer/src/held.rs`) gives each posted file a `FileId` of a per-process generation and a serial that "is never reused", and the page maps a `File` object to its id through a `WeakMap`.

### External definitions, cited and not quoted

The standards text is not in the repository, so a reviewer checks these at the source.

- WHATWG Fetch Standard, a request's cache mode. `"default"` lets the browser return a stored response it considers fresh without contacting the server. `"no-cache"` uses a stored response only after a conditional request to the server, and makes an ordinary request when nothing is stored. `"no-store"` neither reads nor writes the HTTP cache. `"reload"` does not read it and does write it. `"force-cache"` and `"only-if-cached"` return a stored response whether or not it is stale.
- RFC 9111 section 4.2.2, heuristic freshness. A response with `Last-Modified` and no explicit expiry may be treated as fresh for a heuristic period. This is how a DICOMweb server that sends no `Cache-Control` still gets a re-processed object served from the browser's cache under `"default"`.
- W3C File API, a `File`'s snapshot state. Reading a `File` whose underlying storage changed after the snapshot fails with `NotReadableError`. Evidence E1 and E2 measure what Chromium treats as a change.
- FNV-1a, the 64-bit variant as specified in the IETF FNV draft (draft-eastlake-fnv): offset basis `0xcbf29ce484222325`, prime `0x00000100000001b3`, and for each byte `hash = (hash XOR byte) * prime` modulo `2^64`. Its test vectors include `""` to `0xcbf29ce484222325`, `"a"` to `0xaf63dc4c8601ec8c` and `"foobar"` to `0x85944171f73967e8`.

## What the specification does not cover

The HLD gives the decoded tier a key type and nothing about what a key must prove. Every item below is decided by the lead under the operator's delegation of 2026-11-05.

1. **The key stays the caller's slot, and the identity moves into the lookup.** `FrameKey` is the address the caller uses before any bytes exist: a stack is installed with keys before its frames are fetched. A content digest cannot be in that address, because it needs the bytes. So `FrameKey` and its three wire formats are unchanged, and the decoded tier stores, beside each frame, the identity of the bytes it was decoded from. A lookup must present that identity and gets a hit only when it matches. This is the "equivalent the plan justifies" the story names: it guarantees that a frame decoded from object A is never returned to a caller who now holds object B in the same slot, which is the stale-image defect in the form Ocelli can have it, a caller re-using a `SeriesId` for a re-processed study.
2. **`ContentId`, a 64-bit FNV-1a over the SOP Instance UID, the transfer syntax UID and the encoded bytes, each length-prefixed.** The input is `len(uid) as u64 little-endian, uid, len(ts) as u64 little-endian, ts, len(encoded) as u64 little-endian, encoded`. Length prefixes make the field boundaries unambiguous. The UID is included because WADO-RS frame retrieval returns pixel bytes with no UID in them, and the transfer syntax because the same bytes decode differently under two syntaxes. FNV-1a is chosen because it is a few lines of `no_std` code with no dependency, fixed across targets, and **a change confined to one byte position always changes the hash**: each step `h -> (h ^ b) * p` is a bijection of `u64` for odd `p`, so two inputs that differ at one position reach different states there and stay different. Two inputs that differ in more than one place collide with probability about `2^-64` for unstructured changes. A collision only matters for two versions of one slot, so the birthday bound over a whole study does not apply. SHA-256 was rejected because `sha2` is a dev and oracle dependency today, not a shipped one, and adding it to `ocelli-cache` is a new shipped dependency outside HLD section 15.2's list, which would need a `D-NN` row.
3. **Description and plane are compared exactly, not hashed.** A re-process can correct Pixel Spacing or Image Position and keep both the UID and the pixel bytes. `DecodedFrame` already holds `StoredPixelDescription` and `ImagePlane`, both `PartialEq`, so the lookup compares them by their derived equality. That covers every decode input this crate holds, with no serialisation of nested types into a hash.
4. **A mismatch removes the held frame.** The slot then holds at most one version, the one the latest caller presented, and a later lookup for the old version is a miss rather than an alternating hit. The removal is explicit, not an eviction, and emits no event, which is `ImageCache::remove`'s existing rule. `FrameLookup::Stale` reports the bytes removed so a caller can count them.
5. **What the decoded tier cannot know.** A lookup by slot without fresh bytes or a fresh validator cannot detect that the source changed. The decoded tier saves a decode, never a fetch. Freshness of the fetch is the transport's job, which is the cache mode below, and dropping a whole series or study when a caller leaves it is F-226. A re-process that changes neither the UID, the transfer syntax, the encoded bytes, the description nor the plane produces the same frame and is not stale.
6. **`DicomwebClient` cache mode `cache`, default `"no-cache"`.** Accepted values are `"no-cache"`, `"no-store"`, `"reload"` and `"default"`. `"no-cache"` is the default because it never shows a stored response without asking the server, which closes the RFC 9111 heuristic path, and a server with validators answers 304 so an unchanged object is not downloaded twice. `"default"` stays available as an explicit opt-in to browser heuristics. `"force-cache"` and `"only-if-cached"` are refused with `DicomwebError` `InvalidRequest` at construction, because both return a stored response that may be stale, which is the defect this story removes. `postWadoInstance`'s `WadoInstanceRequest` gains the same optional `cache` and passes it through. A proxy or a service worker between the browser and the server is outside what a cache mode controls, and the documentation says so.
7. **The folder viewer server needs no digest.** F-216's `FileId` is bound to exactly the bytes of one post and its serial is never reused, so a re-posted file gets a new id and cannot hit an old frame. That is stronger than a digest, which can collide. F-216 rejected a digest id for that reason, and this plan keeps the decision.
8. **The folder viewer page keys pixel caches by the `File` object.** The grey cache, the in-flight map and the thumbnail bitmaps use a per-`File` serial from a `WeakMap<File, number>`, never the path, size and time, so a correctness rule no longer depends on `dropCache` having run. `noteKey` keeps `fileIdentity`, because marks attach to a file's place in the folder, not to its bytes, and F-215 owns marks.
9. **The same folder can be chosen again.** The `change` handler copies `folderInput.files` into an array and then sets `folderInput.value = ""`. Chromium does not fire `change` when a file input's new selection equals its current one, which is the usual cause of "I opened it again and nothing changed". This plan did not measure that, because Playwright's `setInputFiles` does not go through the native picker. The reset is unconditional and harmless either way.
10. **What the page can detect without a re-pick, and what it does.** Measured in Chromium 151 (E1, E2): a rewrite that moves the file's modification time makes every read of the old `File` fail, and a rewrite that restores the modification time is read silently with the new bytes while `file.size` still reports the old length. The page probes `file.slice(0, 1).arrayBuffer()` for every file of the active series, four at a time, on window `focus`, on `visibilitychange` to visible, and on series selection. A rejected read marks the `File` as changed. A changed file is forgotten on the server through F-226's route, its page caches are dropped, and if it is on screen the viewport shows the sentence "This file changed or was removed on disk after the folder was opened. Open the folder again to see it." instead of the old image. The status line counts the changed files of the series. The page does not try to read new bytes from a changed `File`, because its snapshot refuses. The File System Access API's directory handle could re-read, and was rejected because the browser harness F-208 and F-216 use cannot drive its picker and Firefox and Safari lack it.
11. **Limits, stated in the user documents.** A rewrite that restores the modification time is not detected, and the page keeps showing the held bytes until the folder is opened again. A change is noticed at the next focus, visibility change or series selection, not instantly. Other browsers are not measured.

## Problem and reproducer

Source identity: HEAD `8729656` on `sprint/s46` for Rust and TypeScript. F-216 as integrated for the folder viewer.

1. Rust: `cache.insert(FrameKey::new(SeriesId(1), 0, 0), frame_a)` for object A. The caller re-opens a re-processed study, re-uses `SeriesId(1)` for it, and calls `cache.get(&FrameKey::new(SeriesId(1), 0, 0))` while holding object B. Today that returns A's samples, and nothing in the API can refuse.
2. TypeScript: a fake `fetch` passed to `DicomwebClient` records `init.cache === undefined` on all six request kinds, so a browser applies `"default"` and RFC 9111 heuristic freshness can serve the old object at the same URL.
3. Folder viewer: a file is rewritten with new pixel values after the folder was opened. With F-216's held id the page never reads the `File` again, so the old image stays on screen with nothing said. Opening the same folder again may fire no `change`.

```bash
bin/ocelli.sh test ocelli-cache --test image_cache
npx vitest run packages/core/src/dicomweb.test.ts
node examples/folder-viewer/tests/browser/changed.mjs
```

The new API does not compile against the old source, which is not the semantic RED. The semantic RED is recorded in the progress note by running each scenario against the old source: `get` returns A's samples for scenario 1, `init.cache` is `undefined` for scenario 2, and the canvas still shows the old grey after the rewrite for scenario 3.

## Scope, API and compatibility

In scope: `ocelli-cache`'s decoded tier, `ocelli-viewport`'s scroll lookup, `DicomwebClient` and `postWadoInstance`, and the folder viewer page.

Non-goals: changing `FrameKey`, the stack, handoff or event wire formats. Detecting a server-side change without a fetch. Revalidating a held folder file on every request. A digest in the folder viewer server. Purging the browser HTTP cache, which no page API can do per URL.

### Rust, `crates/ocelli-cache/src/image.rs`

```rust
/// FNV-1a 64 over the length-prefixed SOP Instance UID, transfer syntax UID
/// and encoded bytes. Decision 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentId(pub u64);

impl ContentId {
    pub const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    pub const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    #[must_use]
    pub fn of(sop_instance_uid: &[u8], transfer_syntax_uid: &[u8], encoded: &[u8]) -> Self;
}

/// What a lookup presents. Decision 3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSource {
    pub content: ContentId,
    pub description: StoredPixelDescription,
    pub plane: ImagePlane,
}

/// The three answers a lookup can give.
pub enum FrameLookup<'a> {
    Hit(&'a DecodedFrame),
    Miss,
    /// A frame from other bytes or metadata was held and has been removed.
    Stale { bytes: usize },
}

impl ImageCache {
    pub fn insert(&mut self, key: FrameKey, content: ContentId, frame: DecodedFrame) -> CacheEvents; // was (key, frame)
    pub fn get(&mut self, key: &FrameKey, source: &FrameSource) -> FrameLookup<'_>;                  // was -> Option<&DecodedFrame>
    // contains, remove and pressure unchanged
}
```

The length prefix uses `u64::try_from(len)`, with no `as` cast, and a length that does not fit `u64` cannot occur on any target this crate builds. The `Lru` value becomes a private `struct HeldFrame { content: ContentId, frame: DecodedFrame }` whose `Budgeted::bytes` is `frame.bytes()`, so admitted bytes and every event payload are unchanged. `DecodedFrame` and its constructor are unchanged, so `ocelli-volume`, `examples/example-server` and `tools/oracle` need no edit. `lib.rs` re-exports `ContentId`, `FrameSource` and `FrameLookup`.

### Rust, `crates/ocelli-viewport/src/scroll.rs`

```rust
pub enum ScrollError { Empty, NotResident, Stale }
pub fn samples_for_upload<'a>(&self, cache: &'a mut ImageCache, source: &FrameSource) -> Result<&'a [u8], ScrollError>;
```

### TypeScript, `packages/core/src/dicomweb.ts` and `instance-bytes.ts`

```ts
export type DicomwebCacheMode = "no-cache" | "no-store" | "reload" | "default";
export interface DicomwebClientOptions { /* five existing fields */ readonly cache?: DicomwebCacheMode; }
export interface WadoInstanceRequest { /* existing fields */ readonly cache?: DicomwebCacheMode; }
```

Every `#fetch` call passes `cache: this.#cache`. An undefined `cache` means `"no-cache"`. Any other string throws `DicomwebError("InvalidRequest")` from the constructor. `cache` joins F-230's key lists for both types. `index.ts` exports `type DicomwebCacheMode`.

### Folder viewer page, `examples/folder-viewer/src/page.html`

`fileSerial(file)`, the change-handler reset, `probeFiles(files, generation)`, a `changedFiles` `WeakSet<File>`, the two sentences of decision 10, and a call to F-226's forget helper for a changed file's id. No route changes in `serve.rs`.

Compatibility: `ImageCache::insert`, `ImageCache::get` and `samples_for_upload` change signature. Their callers are `crates/ocelli-cache/tests/image_cache.rs`, `crates/ocelli-viewport/tests/scroll.rs` and `scroll.rs` itself, found by `grep -rln 'ImageCache\|samples_for_upload' crates examples tools`. No production caller exists, because the wasm session holds no `ImageCache`. Under 0.x this is an in-place change with a `CHANGELOG.md` entry. The DICOMweb default changes the request a browser sends, and is recorded the same way.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| A Rust caller holding decoded frames | `ImageCache::insert` with `ContentId::of`, `ImageCache::get` with `FrameSource` | Computes the id from the bytes it decoded, presents description and plane it holds | `Hit` only for the same bytes and metadata |
| `StackSequence::samples_for_upload` | `ImageCache::get` | Displayed key only, as today | `Stale` mapped to `ScrollError::Stale` |
| `ocelli-volume`, `example-server`, `tools/oracle` | `DecodedFrame::new` | Unchanged | Unchanged |
| wasm session stack, handoff, ring | `FrameKey`, `HandoffMeta`, `CacheEvent::encode` | Unchanged wire | Unchanged bytes |
| `DicomwebClient` callers | `#get` | `cache` from the constructor, default `"no-cache"` | `RequestInit.cache` on every request |
| `postWadoInstance`, `examples/viewer-react/src/core-panel.tsx` | `DicomwebClient` | Passes `cache` through, the example passes none | Default `"no-cache"` |
| Folder viewer grey cache, in-flight map, thumbnails | `fileSerial` | Keyed per `File` object | A re-pick never hits an old entry |
| Folder viewer open | change handler | Copies the files, resets `value` | The same folder can be chosen again |
| Folder viewer focus, visibility, series selection | `probeFiles` | Four reads in flight, active series only, generation token | Changed files marked, forgotten, sentence shown |

The owner of the identity rule is `ImageCache::get` alone. The owner of the cache mode is `DicomwebClient`'s constructor. The owner of change detection is `probeFiles`.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Slot empty | `get(key, s)` | `Miss` | nothing changes |
| Slot holds frame from `s` | `get(key, s)` | `Hit`, the frame becomes most recently used | none |
| Slot holds frame from `s` | `get(key, t)`, `t.content != s.content` | entry removed, `Stale { bytes }` | `pressure().used` drops by exactly `bytes`. No event |
| Slot holds frame from `s` | `get(key, t)`, same content, different plane or description | removed, `Stale` | same |
| After `Stale` | `get(key, s)` | `Miss` | the old frame is gone |
| After `Stale` | `insert(key, t.content, frame_t)` | admitted, no `Displaced` event | none |
| Slot holds frame from `s` | `insert(key, t.content, frame_t)` without a lookup | `Displaced`, as today | `used` exact under D-24 |
| Client constructed | `cache: "force-cache"` | throws `InvalidRequest` | no client exists |
| Page shows image of file F | F rewritten with a new modification time, window focus | F marked, F's id forgotten, sentence shown | other files of the series still draw |
| Page shows F marked | the folder is opened again | new `File` objects, new serials and ids, new grey | old marks and caches gone with the old open |
| Probe in flight | another folder is opened | probe results for the old generation are discarded | new folder unaffected |

Invariant across consumers: no path returns a decoded frame whose `ContentId`, description or plane differs from the ones the lookup presented. Invariant in the page: no pixel cache key is derived from a file name, size or time.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Held decoded frame | `ImageCache::insert` | `FrameKey` plus `ContentId` | `Lru::insert` admitted | Refused: unchanged D-24 path. A repeated identical insert displaces with equal content |
| Stale removal | `ImageCache::get` | `FrameKey` | `Lru::remove` inside the lookup | A second identical stale lookup is a `Miss` |
| `File` serial | `fileSerial` | the `File` object, `WeakMap` | first use | A re-pick gives new objects and new serials |
| Changed mark | `probeFiles` | the `File` object, `WeakSet` | a rejected read | A second probe of a marked file is skipped. A forget that fails leaves the server entry to age out under F-216's budget |
| Visible image of a changed file | `applyGrey` path | `loadToken` | the sentence replaces the image | A probe that resolves after a newer open is ignored by its generation |

Cache validity inputs for a held decoded frame: the SOP Instance UID, the transfer syntax UID, the encoded bytes, every field of `StoredPixelDescription` and every field of `ImagePlane`. Validation precedes the return: `get` compares before it marks the entry used. Valid, refused, valid: `get(key, s)` hits, `get(key, t)` is `Stale` and removes, `insert(key, t, frame_t)` then `get(key, t)` hits and `get(key, s)` is `Stale`.

## Numbered acceptance inventory

Every `ContentId` value below is computed from the FNV-1a definition in a reference script, not from the implementation.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The FNV-1a 64 core reproduces the reference vectors: `""` `0xcbf29ce484222325`, `"a"` `0xaf63dc4c8601ec8c`, `"foobar"` `0x85944171f73967e8`, `"abc"` `0xe71fa2190541574b` | `fixture_fnv1a_reference_vectors`, a `#[cfg(test)]` unit test in `image.rs` over the private fold, because `ContentId::of` always adds the prefixes | FNV-1 order, multiply then XOR. A 32-bit prime | implementation |
| AC2 | `ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", &[0, 1, 2, 3])` is `0x49e8b650ff1cd3f1`. Last byte 4 gives `0x49e8b750ff1cd5a4`. UID `1.2.4` gives `0xaa81c9a83be590b6`. `of(b"", b"", b"")` is `0x81d23fd7003c2305` | `content_id.rs`, `fixture_content_id_values` | Omitting the length prefixes, or the UID, or the transfer syntax | implementation |
| AC3 | Length prefixes separate fields: `of(b"ab", b"c", b"")` is `0x42c5658fd73f0196` and `of(b"a", b"bc", b"")` is `0x8188530ff4c99a2a` | `content_id.rs`, `fixture_field_boundaries` | Concatenating fields without lengths, under which both are `FNV("abc")` | implementation |
| AC4 | Every single-byte change of an 8-byte encoded buffer, all 8 times 255 variants, changes the id | `content_id.rs`, `property_one_byte_change_changes_the_id` | A digest of the length only, or of a prefix | implementation |
| AC5 | Valid, stale, valid on one slot, as in the validity paragraph, with `pressure().used` hand-computed: a 4 by 4 16-bit frame is 32 bytes, so `used` goes 32, 0, 32 | `image_cache.rs`, `a_reprocessed_object_in_the_same_slot_is_not_served` | Today's `get`, which returns the old frame. RED on the old source | implementation |
| AC6 | Same content, a plane whose Image Position differs by 0.5 mm, is `Stale`. Same content and plane, Pixel Representation 0 against 1, is `Stale` | `image_cache.rs`, `metadata_changes_are_stale` | Comparing content only | implementation |
| AC7 | `Stale` emits no event and is not counted as an eviction by `TierCounters`. An insert after `Stale` produces no `Displaced`. An insert over a live slot still produces `Displaced` | `image_cache.rs`, `stale_removal_is_explicit` | Reporting a stale removal as an eviction, which inflates F-034's counters | implementation |
| AC8 | `samples_for_upload` returns `Err(Stale)` for a mismatched source and the samples for a matching one | `crates/ocelli-viewport/tests/scroll.rs`, one added case | Mapping `Stale` to a hit | implementation |
| AC9 | The 16-byte stack key, the 24-byte `HandoffMeta` image and the cache event payload are byte-identical to today | existing `handoff_tests`, `crates/ocelli-wasm` session tests and `crates/ocelli-cache` encode tests unchanged and green | Adding `ContentId` to `FrameKey` | implementation |
| AC10 | `DicomwebClient` with no `cache` sends `init.cache === "no-cache"` on `searchStudies`, `searchSeries`, `searchInstances`, `retrieveInstance`, `retrieveFrames` and `retrieveWadoUri`. Each of the four accepted values is sent unchanged | `dicomweb.test.ts`, `sends the cache mode on every request` | One request kind missed. A default left `undefined` | implementation |
| AC11 | `cache: "force-cache"`, `"only-if-cached"` and `"nocache"` throw `DicomwebError` `InvalidRequest` before any fetch. `cahce: "no-store"` throws F-230's `UnknownOption` | `dicomweb.test.ts`, `refuses a cache mode that can serve a stale response` | Accepting any string, which lets `force-cache` through | implementation |
| AC12 | `postWadoInstance({ ..., cache: "no-store" })` sends `no-store`. Without it, `no-cache` | `instance-bytes.test.ts` | A request type that drops the field | implementation |
| AC13 | Folder viewer: open a synthetic folder, draw image 0, rewrite file 0 with different stored values and a modification time 5 s later, dispatch window `focus`. The viewport shows the decision 10 sentence, `GET /held` reports file 0's id gone, and other images of the series still draw | `examples/folder-viewer/tests/browser/changed.mjs` in F-208's `viewer-browser` gate | A page that keeps the held bytes on screen. RED on F-216's page | integrated |
| AC14 | Then `setInputFiles` on the same folder again. The canvas at 16 sampled points equals the grey the server returns for the new bytes, and differs from the old grey at the hand-chosen point where the stored value changed | same script | A grey cache keyed by path, size and time that hits after a rewrite with equal size and time | integrated |
| AC15 | Limit, pinned so it stays visible: rewrite file 0 with its modification time restored and focus. The page does not mark it, and the old image stays until the folder is opened again | same script, `restored modification time is not detected` | A plan or a page that claims detection it does not have | integrated |
| AC16 | The change handler resets `folderInput.value` after copying the files, and a series selection triggers a probe | page source pins in `examples/folder-viewer/tests/preview.rs`, plus the browser script counting probe reads on selection | A reset before the copy, which empties the list | implementation, integrated |
| AC17 | A probe of 1,693 files reads one byte each, four at a time, and finishes in under 1 s in headless Chromium on the F-216 host (E3 measured 85 to 149 ms) | browser script timing, recorded in the review evidence | A probe that reads whole files | integrated |
| AC18 | F-216's `viewer.open`, `viewer.first_image`, `viewer.window_step` and `viewer.scroll_rate` thresholds still hold after this story | `bin/ocelli.sh bench viewer.open viewer.first_image viewer.window_step viewer.scroll_rate` at sprint verification on the F-216 host | A probe on every request, which would show in `window_step` | external, sprint verification |
| AC19 | README, `use.html` and `more.html` state the default cache mode, the identity rule and the three limits of decision 11 | review | Documentation that promises detection of every change | implementation, then review |

## Approach

1. `image.rs`: `ContentId::of` as a fold over the three length-prefixed fields with `wrapping_mul`, `HeldFrame`, the new `insert` and `get`. `lib.rs` re-exports.
2. `scroll.rs`: the new `samples_for_upload` and `ScrollError::Stale`.
3. `dicomweb.ts`: the `cache` option, its validation in the constructor, and `cache` in `#get`. `instance-bytes.ts` passes it through. F-230's key lists gain `cache`.
4. `page.html`: `fileSerial`, cache keys, the change-handler reset, `probeFiles` with a generation token and four workers, the focus, visibility and selection triggers, the changed-file path that calls F-226's forget helper, drops the file's grey entries and shows the sentence.
5. `examples/folder-viewer/tests/browser/changed.mjs` on F-208's harness, writing synthetic Part 10 files into a temporary directory with the bench writer F-216 added. Never a patient file.
6. Tests, CHANGELOG, LLD, the three user documents.

Rejected alternatives:

- `ContentId` inside `FrameKey`. A stack is installed before its bytes exist, and three wire formats would change.
- Replacing `SeriesId` with the Series Instance UID string. It does not detect a re-process that keeps UIDs, and `SeriesId`'s doc gives the mapping to the study owner.
- SHA-256 through `sha2`. A new shipped dependency for a property FNV-1a already gives on the change that matters, decision 2.
- A `cache` default of `"no-store"`. It re-downloads every object every time even when the server can answer 304.
- Reading every file in full on focus to digest it. 850 MiB for F-216's measured folder.

## Dependencies, write ownership and resources

Declared dependencies: **F-230** and **F-226**. Source-level dependencies: F-216 integrated (page ids and `/held`), F-208 integrated (the `viewer-browser` harness and gate), F-226 integrated (forget helper, and the other half of `image.rs`).

Order, recommended to the lead: F-230 now in parallel with F-216, then F-216, F-210, F-211 and F-208 in the order the S46 design round set, then F-226, then this story, then F-209 and the rest. F-230 before this story because F-230 owns `DicomwebClientOptions`'s key list and the coded error class this story's refusal uses, so this story adds one key under a compile-time check rather than inventing a refusal F-230 would rework. F-226 before this story because both write `crates/ocelli-cache/src/image.rs`, `lib.rs` and `page.html`'s `readFolder`, and this story's changed-file path calls F-226's forget route.

Write set:

- `crates/ocelli-cache/src/image.rs`, `src/lib.rs`, `tests/image_cache.rs`, `tests/content_id.rs` (new)
- `crates/ocelli-viewport/src/scroll.rs`, `tests/scroll.rs`
- `packages/core/src/dicomweb.ts`, `dicomweb.test.ts`, `instance-bytes.ts`, `instance-bytes.test.ts`, `options.ts` (two key lists), `index.ts`
- `examples/folder-viewer/src/page.html`, `tests/preview.rs` (pins appended), `tests/browser/changed.mjs` (new), and the script list of the `viewer-browser` arm in `bin/ocelli.sh` if F-208 registers scripts by name
- `CHANGELOG.md`, `README.md`, `examples/folder-viewer/src/use.html`, `examples/folder-viewer/src/more.html`
- `docs/lld/cache.md`, `docs/lld/dicomweb.md`, `docs/lld/viewport.md`, at `/complete-feature`

Files that cannot have concurrent writers: `image.rs` with F-226, `page.html` with every viewer story, `dicomweb.ts` with F-230. Exclusive resources: the browser script binds `OCELLI_PORT=0`, reads the printed URL and stops the process by PID. Ports 8742 and 8765 are never used.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-cache
bin/ocelli.sh test ocelli-cache
bin/ocelli.sh test ocelli-viewport --test scroll
bin/ocelli.sh clippy ocelli-cache
bin/ocelli.sh native
npx vitest run packages/core/src/dicomweb.test.ts packages/core/src/instance-bytes.test.ts
bin/ocelli.sh test ocelli-folder-viewer
bin/ocelli.sh gate viewer-browser
```

Final: `bin/ocelli.sh gate --sprint` at sprint verification, including the bench rerun of AC18. Mutations, each applied, observed red, reverted: drop the transfer syntax from `ContentId::of` (AC2), drop the length prefixes (AC3), compare content only in `get` (AC6), restore today's `get` (AC5), leave `cache` out of `retrieveFrames` (AC10), key the grey cache by `fileIdentity` again (AC14), remove the focus probe (AC13). Evidence is invalidated by any change in the write set, by F-216's held id rule changing, or by a Chromium version other than 151.0.7922.34 for AC13 to AC17.

## Evidence, measured 2026-11-05 by the plan author

Host: Apple M4 Max, macOS 27.0.1, Chromium headless shell 151.0.7922.34 through `tools/bench`'s Playwright, a page holding `<input type="file" webkitdirectory multiple>` filled by `setInputFiles` from a scratch directory of non-DICOM bytes. No server was started.

| ID | Observation |
|---|---|
| E1 | Rewrite to the same size with a modification time 5 s later: `file.slice(0, 8).text()` rejects `NotReadableError`, `new Response(file).arrayBuffer()` rejects `TypeError` |
| E2 | Rewrite with the modification time restored: same size reads the new bytes, a 10-byte rewrite of an 8-byte file reads 10 new bytes, a 6-byte rewrite reads 6, and `file.size` stays 8 in all three. `setInputFiles` again gives a `File` of size 6 with the new bytes |
| E3 | 1,693 files of 524,804 bytes, one-byte slice read each, four in flight: 148.5 ms first run, then 85.0, 85.4, 90.4 and 90.3 ms |

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. `ContentId` is computed where the bytes already are, and the folder viewer still receives grey bytes from its local process as F-206 and F-216 accepted
- Render-loop allocation: none. `ContentId::of` runs on the decode path, `get` allocates nothing, and the page probe runs on focus and selection
- unsafe: none
- Tier A (WebGPU): n/a. The decoded tier is host memory, and no texture or shader changes
- Tier B (WebGL2): n/a, for the same reason
- Tier C (CPU): full. The decoded tier and the folder viewer's CPU `LutChain` path are unchanged in arithmetic. Only which frame a lookup may return changes

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1, FNV-1a reference vectors from the IETF draft | `crates/ocelli-cache/src/image.rs`, `#[cfg(test)]` |
| fixture | AC2 and AC3, composite ids computed from the FNV definition | `crates/ocelli-cache/tests/content_id.rs` |
| property | AC4, every one-byte change changes the id | `crates/ocelli-cache/tests/content_id.rs` |
| unit | AC5 to AC7 lookup, stale removal, events and hand-computed `used` | `crates/ocelli-cache/tests/image_cache.rs` |
| unit | AC8 scroll lookup | `crates/ocelli-viewport/tests/scroll.rs` |
| contract | AC9 unchanged wire images | existing tests in `ocelli-cache` and `ocelli-wasm` |
| unit | AC10 to AC12 cache mode | `packages/core/src/dicomweb.test.ts`, `instance-bytes.test.ts` |
| browser | AC13 to AC17 | `examples/folder-viewer/tests/browser/changed.mjs` |

No pixel or geometry arithmetic changes, so no DICOM-section fixture is owed. The comparison of `ImagePlane` uses its derived equality and computes nothing.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no cache row. cornerstone3D 5.8.2 keys its image cache on the caller's image id, and this story's identity rule is not a parity item.

## Deviations

None. `FrameKey` keeps section 20's `Lru` key role, D-24's `Admission` is unchanged, and no shipped dependency is added.

## LLD impact

- `docs/lld/cache.md`: a section "Frame identity" with `ContentId`'s definition, the exact comparison of description and plane, stale removal as an explicit release, and what the decoded tier cannot know. The folder viewer paragraph F-216 added gains the per-`File` keys and the change probe.
- `docs/lld/dicomweb.md`, "Transport validation and errors": the cache mode, its default and the refused modes.
- `docs/lld/viewport.md`, the `samples_for_upload` paragraph: the source argument and `ScrollError::Stale`.

## User-facing documentation impact

- `README.md`, the folder viewer list: one item that opening the same folder again reads the files again, that a file changed on disk is reported when the window regains focus or a series is selected, and that a rewrite which keeps the file's modification time is not detected. The `DicomwebClient` paragraph: the `cache` option, its `no-cache` default, and that a proxy or service worker is outside it.
- `examples/folder-viewer/src/use.html`, 5.6 "DICOMweb": a sixth row in the options table for `cache`, with the four values and the default, and one sentence on why `force-cache` is refused. 12 "Cache and volume": one paragraph and a three-line sample for `ContentId::of`, `FrameSource` and `FrameLookup`.
- `examples/folder-viewer/src/more.html`, `#worker` "Worker decode and the wasm path": one sentence that the library's decoded tier answers only for the bytes a frame was decoded from, and that the worker path still holds no frame cache.

## Open questions

None open. Every judgement call is a lead decision.

- Q1, what the identity is. Answered: lead decision under the operator's delegation of 2026-11-05. The slot key is unchanged and the lookup presents `ContentId`, description and plane, decisions 1 to 3.
- Q2, the digest. Answered: lead decision under the operator's delegation of 2026-11-05. FNV-1a 64 over length-prefixed UID, transfer syntax and bytes, no new dependency, decision 2.
- Q3, the cache mode default. Answered: lead decision under the operator's delegation of 2026-11-05. `no-cache`, with `force-cache` and `only-if-cached` refused, decision 6.
- Q4, what the folder viewer can detect. Answered: lead decision under the operator's delegation of 2026-11-05. A re-pick always, a rewrite that moves the modification time at the next focus or selection, a rewrite that restores it never, decisions 9 to 11, from evidence E1 to E3.
- Q5, order. Answered: lead decision under the operator's delegation of 2026-11-05. After F-230 and F-226, recorded under Dependencies.

### Lead ruling after review pass 1

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-224-working-pass-1.md`. `ImageCache` never hands an admission to `TierCounters`, so AC7's counter half is a structural fact checked by review, and the test block that could not fail is removed. AC7's event half, no event on `Stale` and no `Displaced` after it, stays a test.

### Lead amendments for the folder viewer half, against the sprint head after F-226

Made by the sprint lead on 2026-11-07 from read-only groundwork. Where these differ from the text above, these win. Line numbers are re-measured at implementation.

- **The forget is F-226's.** A file detected as changed is forgotten through the page's existing forget helper and `POST /forget` from F-226, not a second route.
- **Thumbnails are keyed by series, not by file.** Decision 8's thumbnail re-keying clause has nothing to re-key and is withdrawn. A changed file's thumbnail is refreshed only if its series' thumbnail was drawn from that file.
- **`cacheKey` is shared with F-213,** which will add the file window index. This story replaces only the file identity prefix, so the later merge is one line.
- **The new browser check is wired** into `bin/ocelli.sh viewer-browser` and its `--list` description, beside the four checks already there.
- **Owed with this half:** README, `use.html` section 12 and `more.html` `#worker` per AC19, the `postWadoInstance` `cache` option in its option table (pass 3's note). CHANGELOG is written at integration.
