# F-216, Folder viewer performance: upload once, held decode, measured budgets, release build

**Status**: approved
**Epic ref**: E38.26
**Sprint**: S46
**Estimate**: 4w

## Normative source, transcribed

The HLD does not specify this page or its local process. The rules it does specify, and this plan keeps, are these.

### `docs/hld/17-cache-and-allocation.md`, section 20, in full

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

|  |
|----|
| **No allocation in the render loop.** Pre-size everything at viewport creation. A frame that allocates is a frame that can stutter. |
| **Decode into caller-provided buffers** — fn decode(&self, src: &\[u8\], out: &mut \[u8\]), never a Vec returned per frame. |
| **Use bytemuck::cast_slice** for reinterpreting pixel buffers. Hand-written transmutes are unsafe code with no upside here. |
| **Reuse staging buffers by size class.** Texture uploads should draw from a small pool, not allocate. |

Deviation D-24 widens `insert` to return `Admission<K, V>` with `evicted`, `displaced` and `refused`. That is the shape `crates/ocelli-cache/src/lru.rs` ships and the shape this plan consumes.

### `docs/hld/06-memory-and-cache.md`, section 8, first paragraph

|  |
|----|
| One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs. |

### `docs/hld/23-performance-rules.md`, section 26, in full

|  |
|----|
| No allocation in the render loop. |
| One queue.submit() per frame. |
| Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload. |
| Batch pointer events into one command buffer per animation frame. Never cross the boundary per event. |
| Measure with the benchmark harness before optimising anything. The intuitions that work in JavaScript do not transfer. |

### `docs/hld/12-workspace-and-build.md`, section 15.2, the release profile

```toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

### `docs/hld/15-lut-chain.md` section 18, and decision D14

The LUT chain exists once. `LutChain` in `crates/ocelli-pixel/src/lut.rs` composes stages 1 to 3 and its doc comment says "HLD section 18 requires this arithmetic to exist exactly once". Decision D14 is "Claim MEASURED divergence, never bit-exact reproducibility", and this plan applies its spirit to cost: every budget below is a measured number on a named host, never an assertion.

### What the HLD does not contain

No section names this viewer, a loopback protocol, a file identity, an HTTP keep-alive rule, a page heap budget or any latency target. `docs/lld/benchmarks.md` records the same search result for the whole HLD: "The HLD states no performance target, and this harness invents none." Every target in this plan is therefore an operator decision, carried as Open question A.

## What the specification does not cover

1. **The protocol.** The HLD has no loopback protocol. This plan adds `POST /file` and an `X-Ocelli-File` request header, and keeps every existing route and header byte-identical.
2. **What the one LRU holds.** Section 20 gives the generic `Lru<K, V>`. The decoded tier's existing value type `ocelli_cache::DecodedFrame` requires an `ImagePlane`, which needs Image Position, Image Orientation and Pixel Spacing. A projection radiograph has none of those, so building one would be a fabricated geometry, which is the quietly wrong class. This plan reuses `Lru` and `Budgeted` with a viewer-local value enum and does not use `ImageCache`. There is still one cache.
3. **Whether a held frame is stored samples or modality values.** The lead's brief said "decoded modality-value frames". This plan holds the decoded stored bytes instead. Reasons, both measured or read: `LutChain::new` takes the modality stage as its first argument and `map_into` takes `Stored`, so feeding modality values would need a second composition path with an identity modality stage, which is the second-copy defect section 18 forbids. And the unpack from held bytes costs 6.66 ms per 8.7 Mpx frame in the release profile against 13.32 to 48.79 ms for the chain itself (Evidence E5), while halving the held size of a 16-bit frame.
4. **Encoded bytes after decode.** Section 8 says encoded bytes are "dropped as soon as a frame decodes". For a single-frame object this plan drops them. For a multi-frame object other frames still need them. Open question C.
5. **Cost accounting for a parsed object.** A `DefaultDicomObject` has no measurable footprint. This plan never holds one. It holds the posted bytes, whose allocation is exact, and re-parses on a frame miss. Parse of a 17 MB file measured about 6 ms inside `/inspect` in release (Evidence E2).
6. **Identity across a restart.** An id must never alias another file after the process restarts. This plan makes the id carry a per-process generation.
7. **Concurrency.** The HLD's single-thread rule D5 is about wasm workers, not this native example process. This plan bounds connections and decodes.
8. **The page's memory.** No source bounds the page heap. This plan states a bound and measures it.
9. **The release profile is size-tuned.** Section 15.2's `opt-level = "z"` is for the wasm size budget. Measured on this host, a full `preview_frame` on 8.7 Mpx is 188.99 ms under that profile and 134.39 ms at `opt-level = 3` (Evidence E5). This plan uses the release profile unchanged. Open question D.
10. **`panic = "abort"` in release.** A panic in any request thread ends the whole process under release, where in debug it ends one thread. The page's recovery path after a restart is the same path as an eviction, so this needs no new mechanism, and it is stated so the implementer does not assume thread isolation.

## Problem and reproducer

Source identity: HEAD `d48c468` on `sprint/s46` plus the uncommitted working-tree polish to `page.html`, `serve.rs`, `more.html` and `use.html` (diff of `examples/folder-viewer`, sha256 prefix `89b2411880b93b5c`). Open question H covers that polish.

1. `page.html` `requestGrey` sends `fetch("/preview", { method: "POST", headers, body: image.file })` for every slice change, every window, invert or preset change, every live W/L frame from `scheduleLive`, every tile, every thumbnail and every prefetch. A 17,400,516-byte radiograph is uploaded on each window step.
2. `serve.rs` spawns one thread per connection, writes `Connection: close`, reads the whole body into a `Vec`, and `preview_frame` parses, decodes, unpacks, allocates `Vec<Stored>` and `Vec<Display>` of the frame and runs the chain from scratch on every request.
3. The server never returns that memory to the system. Measured RSS: 6.2 MiB fresh, 169.3 MiB after one 17 MB preview, 661.4 MiB after four concurrent 17 MB previews, 838 MiB after the browser workload below (Evidence E3, E4).
4. `showIndex` hides the stage on a cache miss and discards any response whose `loadToken` is stale. When one preview takes longer than the key interval, the viewport stays blank until keys stop. Measured on the debug build: 0.7 presented frames per second while ArrowDown is pressed at about 60 Hz on a 512 by 512 series (Evidence E6).
5. `drawGrey`, `paintTiles` and `greyImageData` allocate a fresh `ImageData` per draw and fill it with a per-pixel byte loop. `thumbs` holds a full-resolution grey frame per series with no bound.
6. The README and the page's `GONE` sentence say `cargo run -p ocelli-folder-viewer`, which is the debug build. A window step on the 17 MB frame is 1,295 ms in debug and 125 ms in release through the page (Evidence E6).

## Scope, API and compatibility

In scope: the folder viewer's server, its page, its README section, five benchmark subjects with runners, and their baseline.

Non-goals: a display-resolution preview during a live drag (Open question F), a header-only open that avoids posting pixel data (Open question I), any change to `ocelli-cache`, `ocelli-pixel` or the root `Cargo.toml`, GPU rendering of the page, and F-208's viewport grid.

### Protocol

| Route | Request | Response | Compatibility |
|---|---|---|---|
| `POST /file` | Whole Part 10 body, optional `X-Ocelli-Frame` | 200, body byte-identical to `/inspect` for the same bytes and frame. Header `X-Ocelli-File: <id>` when held, `X-Ocelli-Held: 1` or `0` | New |
| `POST /preview` | `X-Ocelli-File: <id>`, `Content-Length: 0`, plus every existing frame and window header | Byte-identical status, `X-Ocelli-*` headers and body to the body-carrying request for the same file. Adds `X-Ocelli-Source: held` or `decoded` | New form |
| `POST /preview` | Whole body, no `X-Ocelli-File` | Unchanged, adds `X-Ocelli-Source: body` | Kept, pinned by `tests/preview.rs` |
| `POST /headers` | `X-Ocelli-File: <id>` or whole body | Unchanged JSON | Both forms |
| `POST /inspect` | Whole body | Unchanged | Kept |
| `GET /held` | none | `{"build":"release","budget":N,"used":N,"files":N,"frames":N}`, numbers only | New |
| any id route | id unknown, evicted, or from another process | 410 `Gone`, `X-Ocelli-Error: file-gone`, sentence "the file is no longer held by this process" | New |
| any id route whose body is a Part 10 file (`/preview`, `/headers`) | both an id and a non-empty body | 400, `X-Ocelli-Error: header` | New |
| `POST /value` (F-210), `POST /measure` (F-211) | `X-Ocelli-File` only. `/value` has no body. `/measure`'s body is its point list, never a file, which is why the row above names its routes | Their own JSON, with `X-Ocelli-Source` | Built on this protocol by the later stories |
| any | more than the connection cap | 503, `X-Ocelli-Error: busy`, then close | New |

The id is `<generation>.<serial>`, both lowercase hex. `generation` is a `u64` fixed at `Viewer::new` from the wall clock in nanoseconds mixed with a process-wide counter, so two `Viewer` values in one test process differ and a restarted process differs. `serial` is a per-`Viewer` monotonic `u64`. An id names no path, no UID and no content digest. A digest was rejected because a 64-bit collision would show another file's pixels.

Connections are HTTP/1.1 persistent. The server answers `Connection: keep-alive` unless the request said `Connection: close`, closes after 15 s idle between requests, and keeps any bytes read past one request's `Content-Length` as the start of the next. `handle_connection(stream)` keeps its exact public signature and its one-request `Connection: close` behaviour over a zero-budget `Viewer`, because `tests/preview.rs` calls it. It is the one forwarding function this plan adds, and it exists only for that compatibility.

### Public Rust surface

```rust
// examples/folder-viewer/src/held.rs
pub struct ViewerConfig { pub cache_bytes: usize }
impl ViewerConfig {
    /// `OCELLI_CACHE_MIB`, default from Open question B. InvalidInput when not a whole number.
    pub fn from_env() -> std::io::Result<Self>;
}
pub struct Viewer { /* Mutex<Lru<HeldKey, Held>>, generation, serial, permits, live connections */ }
impl Viewer {
    pub fn new(config: ViewerConfig) -> Self;
    pub fn pressure(&self) -> ocelli_cache::Pressure;
}

// examples/folder-viewer/src/serve.rs
pub fn serve() -> std::io::Result<()>;                                    // unchanged signature
pub fn serve_on(listener: TcpListener, viewer: Arc<Viewer>) -> std::io::Result<()>;
pub fn handle_connection(stream: TcpStream) -> std::io::Result<()>;      // unchanged
pub fn handle_connection_with(stream: TcpStream, viewer: &Viewer) -> std::io::Result<()>;

// examples/folder-viewer/src/preview.rs
pub fn preview_frame(bytes: &[u8], request: FrameRequest) -> Result<Preview, PreviewError>; // unchanged
```

Internal, not exported:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum HeldKey { File(FileId), Frame(FileId, u32) }
enum Held { File(Vec<u8>), Frame(Arc<HeldFrame>) }
impl Budgeted for Held { fn bytes(&self) -> usize } // capacity of the owned buffers plus size_of::<HeldFrame>()

struct HeldFrame {
    samples: Vec<u8>,                 // decoder output for one frame, exact capacity
    description: StoredPixelDescription,
    byte_order: ByteOrder,
    recipe: Result<Recipe, PreviewError>,    // request-independent chain inputs, in today's refusal order
    facts: Result<FrameFacts, PreviewError>,
    full_range: Result<(f32, f32), PreviewError>,
    frames: u32,
    width: u32,
    height: u32,
}

fn decode_held(bytes: &[u8], frame: u32) -> Result<HeldFrame, PreviewError>;
// F-210 adds `value: Result<ValueFacts, PreviewError>` and F-211 adds
// `calibration: CalibrationInputs` to HeldFrame, both filled here.
fn render_held(held: &HeldFrame, request: FrameRequest, scratch: &mut Vec<Stored>) -> Result<Preview, PreviewError>;
```

`preview_frame` becomes `request validation`, then `decode_held`, then `render_held` with a local scratch. Every existing fixture in `tests/preview.rs` therefore runs through the same two functions the held path uses. `render_held` unpacks into `scratch`, builds the one `LutChain` exactly as `lut_chain` does today, and writes each grey byte as `display_index(chain.apply(stored).0, 255)` with the same `u8::try_from`, then inverts after the chain as today. That drops the `Vec<Display>` intermediate without a second copy of any stage, because `LutChain::map_into` is itself a loop over `LutChain::apply`.

**Refusal precedence is preserved exactly.** Today's order is request window validation, Part 10 parse, required attributes, size, monochrome, frame range, decode, unpack, Modality LUT Sequence, VOI resolution and full range, facts, chain map. `decode_held` stores the request-independent results as `Result` fields rather than failing early, so `render_held` reports the first refusal in today's order for every request.

`native_bytes` borrows the `U8` value instead of copying it. The `U16` path still builds bytes, because the parsed value owns words.

### Defaults

| Knob | Value | Owner |
|---|---|---|
| Server cache budget | `OCELLI_CACHE_MIB`, default 256 MiB, see Open question B | `ViewerConfig::from_env` |
| Decode permits | 2, fixed, no flag | `Viewer` |
| Connection cap | 32, fixed | `serve_on` |
| Keep-alive idle | 15 s | `handle_connection_with` |
| Page grey cache | 64 MiB, down from 256 MiB, see Open question B | `page.html` `CACHE_BYTE_BUDGET` |
| Page prefetch | next 2 and previous 1 in the scroll direction, inside the page budget | `page.html` `prefetch` |

A decode permit owns one reusable `Vec<Stored>` scratch, grown to the largest frame it has served and never shrunk. Rendering a held frame also takes a permit, because it uses that scratch.

### Release build path

README's start command becomes `cargo run --release -p ocelli-folder-viewer`, with one sentence that the debug build is about ten times slower on a window step (Evidence E6). The page's `GONE` and `processGone` sentences name the same command. When `GET /held` reports `"build":"debug"` the status line says the build is the slow debug build and names the release command. The process still prints one line, the URL. `bin/ocelli.sh` is not changed.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Page open, folder or synthetic | `POST /file` | At most 4 posts in flight, as today | Series list, `X-Ocelli-File` stored in a `WeakMap<File, string>` |
| Main view, slice change | `POST /preview` with id | One request in flight, coalesced to the latest index | Grey bytes drawn into the reused buffer |
| Live W/L in `scheduleLive` | `POST /preview` with id | One in flight, as today, not cached | Same |
| Window, invert, preset change | `POST /preview` with id | Through `fetchGrey`, file-window results cached | Same |
| Tiles in `paintTiles` | `POST /preview` with id | Own `AbortController` per paint | One reused `ImageData` per tile canvas |
| Thumbnails in `loadThumbs` | `POST /preview` with id, `plain` | Own controller, aborted on a new open | Painted once into the thumbnail canvas, no grey kept |
| Prefetch | `POST /preview` with id | Own controller, aborted when the index moves out of range | Page grey cache |
| Headers sheet | `POST /headers` with id | Re-post on 410 | JSON rows |
| F-210 `/value`, F-211 `/measure`, future | `Viewer::frame(id, frame) -> Result<Arc<HeldFrame>, Gone>` | Same id header, same 410 rule. They read modality values through the held frame's `ModalityTransform`, never page arithmetic | Their own JSON |
| `tests/preview.rs` | `handle_connection`, `preview_frame` | Unchanged | Unchanged pinned strings |
| Benchmark runners | `serve` on `OCELLI_PORT`, `GET /held` | Release build only | Run record |

Units, rounding and conversion stay where they are: modality and VOI in `ocelli-pixel`, the grey index in `ocelli_render::display_index`, invert after the chain in `render_held`. The page's grey to RGBA step is a 256-entry table `v -> 0xFF000000 | v << 16 | v << 8 | v` written through a `Uint32Array` view, which is channel replication, the same copy the current loop does, and no window arithmetic. On a big-endian host the page keeps the byte loop, detected once with `new Uint8Array(new Uint32Array([1]).buffer)[0] === 1`.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| No id for this `File` | Page needs a frame | `POST /file`, id stored, then the frame request | Not held (`X-Ocelli-Held: 0`): the page uses the body form for that file from then on |
| Id held, frame not held | `/preview` with id | `decode_held` from held bytes, frame inserted, `X-Ocelli-Source: decoded` | Decode refusal: 400 with today's code, nothing inserted |
| Frame held | Window, invert or preset change | `render_held` only, `X-Ocelli-Source: held` | Window refusal: 400 `window`, the held frame stays, the page keeps the last drawn image and the shown window |
| Id evicted or from a previous process | Any id request | 410 `file-gone` | Page deletes the mapping, re-posts once through a shared in-flight promise, retries once. A second 410 or a failed re-post shows the fault sentence |
| Process stopped | Any request | `fetch` rejects, `processGone` as today | Images already drawn stay |
| Process restarted on the same port | Next id request | 410, because the generation differs | Same re-post path, so restart recovery is transparent |
| Main view request in flight | Another slice step | Index moves, no new request until the in-flight one settles, then one request for the latest index | The previous image, its counter text, its overlays and its marks stay visible together |
| Any page request in flight | Its owner is superseded | `AbortController.abort()` | Server finishes the decode, inserts it, and its write fails on the closed socket. The held frame is useful later |
| Two identical frame misses concurrently | Same id and frame | Both decode, the second insert arrives as `displaced` | `used` stays exact under D-24, both responses identical |
| Entry larger than the whole budget | Insert | `Admission::refused` | The request is served from the value in hand, which is then dropped. `used` unchanged |

**Commit invariant for the page.** The image, the counter text, the corner overlays, the edge letters and the marks change together in `applyGrey`. The counter no longer moves ahead of the image. A stale response is discarded and never drawn under a newer counter.

**Accounting invariant for the server.** `pressure().used` equals the sum of the live entries' admitted bytes and never exceeds the budget, which F-031's property tests already pin for `Lru`. An `Arc<HeldFrame>` taken by a render that is still running after its entry is evicted is outside `used`. That transient is bounded by the decode permits and is inside the RSS bound below.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Posted bytes | `read_request`, allocated once at the declared length | `FileId` generation and serial | `Lru::insert` of `HeldKey::File`, then `X-Ocelli-File` in the reply | Refused: no id returned. Re-post of the same `File`: a new id, the old entry ages out |
| Held frame | `decode_held` under a permit | `HeldKey::Frame(id, frame)` | `Lru::insert`, before the reply is written | Decode refusal inserts nothing. Duplicate decode displaces with equal content |
| Encoded bytes of a single-frame object | `Held::File` | Same id | Removed with `Lru::remove` after frame 0 is inserted, if Open question C is answered yes | `/headers` by id then answers 410 and the page re-posts |
| Permit scratch | Permit pool | Permit slot | Reused, never returned | A panic aborts the process under release, recovery is the restart row |
| Page id map | `ensureFile(file)` | The `File` object, through a `WeakMap` | Set only after a 200 with `X-Ocelli-File` | 410 deletes the entry before the re-post. A re-post in flight is shared |
| Page grey cache | `rememberGrey` | Key: file identity, frame, window or preset, invert | Insert after a successful parse of the reply | `dropCache` on a new open, as today |
| Main canvas buffer | `drawGrey` | Canvas width and height | `putImageData` | A size change reallocates once. Same size reuses the `ImageData` and its `Uint32Array` view |
| Visible image and its labels | `applyGrey` | `loadToken` | One synchronous block | A failed request keeps the previous commit and shows the fault sentence only for the current index, as today |

**Cache validity inputs.** A held frame is valid for exactly `(generation, serial, frame)`. Every input that changes the decode is inside the posted bytes, so an id never needs revalidation against the bytes. Window centre, width, function, preset and invert are not decode inputs and never invalidate a held frame. The page grey cache key carries file identity, frame, window or preset or file window, and invert. A held frame keeps `width`, `height` and `frames`, and the reply headers repeat them, so the page's existing `bytes.length !== width * height` check still runs on every reply.

**Validation before a cached return.** Server: the request window rules in `preview_frame` (centre without width, preset with a numeric window, non-finite values) run before any cache lookup, so a malformed request never returns a held result. Page: `takeGrey` only returns entries stored from a validated reply.

**Valid, refused, valid.** Before settlement: request a preset on a VOI LUT Sequence file, get 400 `window`, then a numeric override on the same id returns 200 from `held`. After settlement: with the image drawn, a refused preset leaves the canvas, the counter and the shown window unchanged, and the next valid change draws.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `POST /file` returns the `/inspect` body and an id of the stated form | `a_posted_file_returns_its_inspect_json_and_an_id` | An id that is a path, a UID or a digest | implementation |
| AC2 | For every fixture and request, id form and body form give identical status, `X-Ocelli-*` headers except `X-Ocelli-Source`, and body | `a_held_frame_matches_the_posted_frame`, over `mono_frame`, `enhanced_frames` frames 0 and 1, window override, invert, preset on synthetic slices, and the refusal fixtures for Modality LUT Sequence, VOI LUT Sequence, frame out of range and not Part 10 | A held path that resolves the window or refusal order differently | implementation |
| AC3 | Hand-computed grey survives the held path: stored 50 under the file `LINEAR_EXACT` window is 64, under the override is 128, inverted 64 is 191, PS3.3 C.11.2.1.2 and C.11.2.1.3.2 as in the existing fixtures | `held_frames_keep_the_hand_computed_grey` | A held frame that stores grey or modality values and re-windows them in a second formula | implementation |
| AC4 | A window change on a held frame does not decode | `a_window_change_on_a_held_frame_does_not_decode_again`, `X-Ocelli-Source` is `decoded` then `held` | A cache keyed by window, or no reuse | implementation |
| AC5 | An evicted id and an id from another `Viewer` answer 410 `file-gone`, and another held id still answers | `an_evicted_file_answers_gone`, `an_id_from_another_process_answers_gone` | A constant generation, or a serial reused after eviction | implementation |
| AC6 | A file larger than the budget is not held and the reply says so | `a_file_larger_than_the_budget_is_not_held` | An admitted entry above budget, or an id for a refused entry | implementation |
| AC7 | `used` never exceeds the budget and equals the hand-computed sum, 524,288 bytes for a 512 by 512 16-bit frame plus the stated fixed overhead | `held_bytes_are_the_buffer_capacity`, `the_held_bytes_never_exceed_the_budget` | Accounting by `len` with a larger capacity, or by source size | implementation |
| AC8 | An id with a body is refused 400 `header` | `an_id_with_a_body_is_refused` | Silently preferring one | implementation |
| AC9 | Valid, refused, valid on one held frame | `a_window_refusal_keeps_the_held_frame` | A refusal that evicts or poisons the frame | implementation |
| AC10 | Two requests on one connection, and two pipelined in one write, get two replies | `keep_alive_serves_two_requests`, `pipelined_requests_get_two_replies` | Truncating the surplus as `read_request` does today | implementation |
| AC11 | The connection cap answers 503 `busy` | `a_connection_over_the_cap_is_busy`, cap lowered through a test-only constructor argument | Unbounded threads | implementation |
| AC12 | Existing pinned tests pass unchanged | `bin/ocelli.sh test ocelli-folder-viewer` with `tests/preview.rs` unmodified above its new tests | A changed pinned string | implementation |
| AC13 | Each file is posted once while held. Window, invert, preset, live W/L, tiles, thumbnails and prefetch send no body | Runner `viewer.window_step` counts request bodies with Playwright `request.postDataBuffer()` | A page that still posts `image.file` to `/preview` | integrated |
| AC14 | Restart recovery: kill the process, start it on the same port, the next frame re-posts and draws without a reload | Runner step inside `viewer.first_image` | A page that shows the fault forever after a 410 | integrated |
| AC15 | Scroll never blanks the viewport and the counter matches the drawn image | Runner `viewer.scroll_rate` samples `stage.hidden` and the counter at every draw | The current hide-on-miss and stale discard | integrated |
| AC16 | Canvas pixels equal the server grey at sampled points after a window change | Runner reads `getImageData` at 16 points against the reply bytes | A page-side grey transform | integrated |
| AC17 | Budgets in Open question A met on the named host, release build | Five runners and `ci/bench-baseline.json` | A number claimed without a run record | external, operator approves thresholds |
| AC18 | Server peak RSS within the stated bound, page heap within its bound | Runner `viewer.resident_memory` | Unbounded transient allocations | external |
| AC19 | README and the page name the release command, and a debug build says so | `the_page_names_the_release_build` string test plus README review | A stale `cargo run -p` line | implementation |

### Resident memory bound, stated before measurement

Server, for the page's own request pattern of at most 4 uploads in flight:

```text
RSS_peak <= B + 4 * S_max + P * (2 * S_max + 5 * N_max) + 32 MiB
```

`B` is the cache budget, `S_max` the largest posted file, `N_max` the largest frame in pixels, `P = 2` permits. Inside each permit, `2 * S_max` is the parse copy and the `U16` byte copy, and `5 * N_max` is 4 bytes of `Stored` scratch plus 1 grey byte. The constant covers the binary, thread stacks and allocator slack. RSS on this host behaves as a high-water mark (Evidence E3), so the bound is on the peak. For the measured workload, 17,400,516 bytes and 8.7 Mpx, the right-hand side is `B + 66.4 + 2 * (33.2 + 41.5) + 32` MiB, which is `B + 247.8` MiB. Other clients are not covered and the plan claims nothing for them.

Page: `JS heap <= 64 MiB grey cache + 4 * W * H per canvas + 16 MiB`, where the canvases are the main view, the four tiles and nothing for thumbnails, measured with `performance.memory.usedJSHeapSize` under `--enable-precise-memory-info`.

## Approach

1. `preview.rs`: split `preview_frame` into `decode_held` and `render_held` as above, keep it as their composition, borrow `U8` native bytes, fuse the chain and the grey index into one loop over `LutChain::apply`. Existing fixtures prove the split before anything else changes.
2. `held.rs`: `FileId`, `HeldKey`, `Held`, `Budgeted for Held`, `Viewer` with `Mutex<Lru<HeldKey, Held>>`, the permit pool (`Mutex<Vec<Vec<Stored>>>` plus `Condvar`), and `Viewer::frame`. `Viewer::frame` takes the lock, clones the `Arc`, and releases the lock before rendering.
3. `serve.rs`: a per-connection reader that keeps surplus bytes, the keep-alive loop, the connection counter, the new routes, `X-Ocelli-File` in `parse_header` with the existing `once` rule, 410 and 503 in `reason`, `X-Ocelli-Source` in `write_reply`, `GET /held`.
4. `page.html`: `ensureFile`, `WeakMap` ids, 410 re-post, controllers per requester, coalesced main view, the commit invariant in `applyGrey`, reused buffers in `drawGrey` and `paintTiles`, thumbnails painted once, `CACHE_BYTE_BUDGET` 64 MiB, `prefetch` of next 2 and previous 1, `renderSeries` no longer called per thumbnail, the release sentence.
5. `tools/bench`: five subjects and runners, a JS writer of synthetic Part 10 files into `tools/bench/out/` (ignored, never staged, the pre-commit hook refuses DICOM magic bytes anyway), the baseline.
6. README and LLD.

Rejected: `ImageCache` and `DecodedFrame` (fabricated plane), a second cache in the page holding decoded samples (pixels in JS and window arithmetic in JS), holding modality values (second composition), a content digest id (aliasing), a fixed worker pool (keep-alive connections would pin workers), a new opt-level profile (Open question D).

## Dependencies, write ownership and resources

Declared dependency F-207, done. Source-level dependency: `ocelli-cache` gains a consumer, no change to it.

Write set: `examples/folder-viewer/Cargo.toml`, `src/lib.rs`, `src/held.rs` (new), `src/preview.rs`, `src/serve.rs`, `src/page.html`, `tests/preview.rs` (new tests appended, existing tests untouched), `README.md`, `tools/bench/subjects.json`, `tools/bench/src/runners/viewer_*.mjs`, the bench test suite list where `docs/lld/benchmarks.md` says new suites must be registered, `ci/bench-baseline.json`, `docs/lld/benchmarks.md`, `docs/lld/cache.md`.

**Shared files and order.** `serve.rs`, `preview.rs`, `page.html` and `tests/preview.rs` are also written by F-208, F-209, F-210, F-211, F-212, F-213, F-214 and F-215. None of them can run concurrently with this story. **Sprint build order, decided in the S46 design round on 2026-11-05:** F-217 first (tooling, no shared viewer file, and its Vite change lands before any `package.json` edit here), then this story, then F-210 and F-211, whose `/value` and `/measure` must use `Viewer::frame` and the id header rather than posting the whole file per pointer move, then F-208, which builds viewports on this request layer and keeps one reused buffer per viewport canvas, then F-209, F-212, F-213, F-214 and F-215. F-119 is independent and may run in parallel. The operator assigned implementation in this sprint to an external coding agent, so the order is the implementer's queue.

Exclusive resources: the benchmark runs need the machine otherwise idle and the release binary built from the staged tree. Port 8742 belongs to another running copy and the runners use `OCELLI_PORT` set to a port they have just bound and released, never 8742.

## Verification and evidence reuse

Focused: `bin/ocelli.sh check ocelli-folder-viewer`, `bin/ocelli.sh test ocelli-folder-viewer`, `bin/ocelli.sh clippy ocelli-folder-viewer`, `bin/ocelli.sh gate bench`, `bin/ocelli.sh bench viewer.open viewer.first_image viewer.scroll_rate viewer.window_step viewer.resident_memory`. Final: `bin/ocelli.sh gate --sprint` in the sprint verification.

**Mutation checks, HLD 27.3.** Each is applied, the named test observed red, then reverted.

1. Make `render_held` call `decode_held` again. AC4 goes red.
2. Account `Held::File` by `len` after reading into a buffer with spare capacity. AC7 goes red.
3. Make `generation` a constant. AC5's cross-process case goes red.
4. Truncate surplus bytes as today's `read_request` does. AC10's pipelined case goes red.
5. Decode the full-range window from the request's window rather than the file. AC2 goes red on `a_missing_window_is_labelled_full_range`'s input.
6. In the page, drop the 410 re-post. AC14 goes red in the runner.

**Benchmark subjects**, each defined now so the runner cannot redefine it:

| Subject | Start | End | Workload | Sample rule |
|---|---|---|---|---|
| `viewer.open` | `setInputFiles` on `#folder` | status text matches the series sentence | 1,693 synthetic 512 by 512 16-bit files, one series | Median of 9 fresh pages |
| `viewer.first_image` | the series sentence | first `putImageData` on `#view` | Same, and one 3000 by 2900 16-bit synthetic DX | Median of 9 |
| `viewer.scroll_rate` | first ArrowDown | 3 s later plus 300 ms settle | ArrowDown every 16 ms on the 1,693 series. Reports keys delivered, frames presented per second, and blank samples | Median of 5. Invalid if fewer than 120 keys delivered |

Lead amendment, 2026-11-05, under the operator's delegation of the quality call that night: the first implementation measured that an empty `setTimeout(16)` loop in the pinned headless Chromium already tops out at 51 to 52 dispatches per second, so a 16 ms timer cadence cannot demonstrate 55 frames per second whatever the page does. `viewer.scroll_rate` therefore dispatches one ArrowDown per `requestAnimationFrame` callback instead, and counts a frame as presented when the image index the page drew changed since the previous callback. The 55 frames per second budget is unchanged, blank samples must still be 0, and the run is still invalid below 120 keys delivered. The runner also reports the plain timer ceiling it measured, so a reader can see the instrument's own limit beside the result.
| `viewer.window_step` | `requestGrey` call from the page | `drawGrey` return | 9 window steps on each workload. Also counts bodies on `/preview` | Median of 9 |
| `viewer.resident_memory` | process start | end of the four subjects above in one process | Peak `ps -o rss=` sampled every 20 ms, `GET /held`, JS heap | One run, the bound above is the threshold |

Instrument identity: `tools/bench` Playwright 1.62.1, Chromium headless shell 151.0.7922.34, viewport 1600 by 1000, the workspace release profile, the binary built by the runner from the staged tree. A run on a different host class is `incomparable`. The runner must use `fetch` or Playwright and never `curl` without `-H 'Expect:'`, because this server does not answer `100-continue` and curl then waits one second per request (Evidence E1).

**Invalidates evidence:** any change to `examples/folder-viewer/src`, the release profile, the Playwright pin, the synthetic workload writer, or the host. A missing run is a missing observation and does not become a passed row.

## Evidence, measured 2026-11-05 by the plan author

Host: Apple M4 Max, 16 cores, 48 GiB, macOS 27.0.1 (26A434), rustc 1.97.1, Chromium headless shell 151.0.7922.34 through Playwright 1.62.1. Source: the identity in Problem and reproducer. Release binary built by the author from that working tree into a scratch target dir. The debug binary was a copy of `target/debug/ocelli-folder-viewer` rebuilt by another session at 03:00 the same day, so its source identity is less certain. Every server ran with `OCELLI_PORT=0` and was stopped by PID. Workloads were synthetic files written by a scratch script, not patients. These figures come from scratch instruments, not the permanent runners, and they set no threshold on their own.

| ID | Observation | Release | Debug |
|---|---|---|---|
| E1 | curl to this server without `-H 'Expect:'` | every POST over 1 MB took 1.01 s more | same |
| E2 | `POST /inspect`, 17,400,516-byte file, curl, median of 9 or 5 | 6.1 ms | 6.9 ms |
| E2 | `POST /preview`, same file, 8.7 Mpx | 181.9 ms | 1,841.2 ms |
| E2 | `POST /preview`, 524,804-byte 512 by 512 file, median of 7 | 6.2 ms | 67.1 ms |
| E2 | 1,693 posts of the 512 by 512 file to `/inspect`, 4 Python workers | 0.21 s | 0.37 s |
| E3 | Server RSS fresh, after one 512 preview, after one 17 MB preview | 6.2, 10.9, 169.3 MiB | not taken |
| E3 | Server RSS peak during 4 concurrent 17 MB previews, and 2 s later | 661.4, 661.4 MiB | not taken |
| E4 | Server RSS after the browser workload E6 | 838 MiB | 859 MiB |
| E5 | In process, 8.7 Mpx, median of 9: `preview_frame`, `LutChain::map_into` LINEAR and SIGMOID, `display_index` loop, `unpack` | `opt-level = "z"`, fat LTO: 188.99, 13.32, 48.79, 27.47 to 62.55, 6.66 ms. `opt-level = 3`: 134.39, 18.93, 49.29, 27.06 to 61.68, 1.61 ms | n/a |
| E6 | Browser, 1,693 by 512 folder: open to list, list to first image, scroll presented per s, window step median, JS heap after | 1,075 ms, 10 ms, 52.7, 6.0 ms, 69.3 MiB | 1,262 ms, 78 ms, 0.7, 41.5 ms, 54.1 MiB |
| E6 | Browser, one 17 MB DX: open to list, list to first image, window step median, JS heap after | 35 ms, 225 ms, 125.3 ms, 101.9 MiB | 33 ms, 1,965 ms, 1,295.2 ms, 143.4 MiB |
| E7 | Node 24.13.0 V8, 8.7 M grey to RGBA: fresh array per-byte loop against reused `Uint32Array` table | 9.6 ms against 5.6 ms | n/a |

Notes on validity. E6's scroll figure did not count keys delivered, so 52.7 is not comparable to a key rate, which is why the permanent subject counts both. E6's DX scroll figure is omitted because a one-image series redraws the same image. The lead's 4.8 s open for a 1,693-file folder is not reproduced here. This workload posts 850 MiB and lists in 1.08 s, so the lead's figure depends on files, build or thumbnails this run did not have, and `viewer.open` must report the per-phase split before any threshold is set.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The page still receives grey bytes from the local process, as F-206 and F-207 accepted. It now receives them without posting the file each time.
- Render-loop allocation: none in the page draw path after the first draw at a size. The `ImageData` and its `Uint32Array` view are sized when the canvas size changes. Server scratch is per permit and reused. The server's reply grey buffer is allocated per reply, which is a network reply and not a render loop.
- unsafe: none
- Tier A (WebGPU): n/a. Neither the page nor the process opens a device.
- Tier B (WebGL2): n/a, for the same reason.
- Tier C (CPU): full. The grey image is the CPU `LutChain` in the local process, unchanged in arithmetic, and only its inputs are now held.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Held path keeps 64, 128 and 191 for stored 50, PS3.3 C.11.2.1.2, C.11.2.1.3.2 and the invert rule | `examples/folder-viewer/tests/preview.rs`, appended |
| fixture | Held size of a 512 by 512 16-bit frame is 524,288 sample bytes plus the stated overhead, hand-computed | `examples/folder-viewer/src/held.rs` unit test |
| contract | AC1, AC2, AC4 to AC11, AC19 | `examples/folder-viewer/tests/preview.rs`, appended |
| unit | `HeldKey` order, generation differs between two `Viewer` values, permit pool returns scratch | `examples/folder-viewer/src/held.rs` |
| browser | AC13 to AC18 | `tools/bench/src/runners/viewer_*.mjs` |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` counts viewport types, tools, blend modes, VOI functions, transfer syntaxes, segmentation, events and adapters. A local example's cost is not a row.

## Deviations

none proposed. Open question C asks whether holding a multi-frame object's encoded bytes after its first frame decodes needs a `D-NN` row against section 8.

## LLD impact

`docs/lld/benchmarks.md` gains the five `viewer.*` subjects. `docs/lld/pixel-pipeline.md` gains a paragraph on `stored_sample_table`, under the lead amendment below. `docs/lld/cache.md` gains the folder viewer as an `Lru` consumer whose value is an enum over two kinds and why `ImageCache` was not used.

## User-facing documentation impact

- `README.md`, "Start the local series viewer" (already in the write set): both command blocks become `cargo run --release -p ocelli-folder-viewer`, with one sentence that the debug build is about ten times slower on a window step. The closing `cargo build -p ocelli-folder-viewer` becomes `cargo build --release -p ocelli-folder-viewer`. One sentence names `OCELLI_CACHE_MIB` and its 256 MiB default, and that each file is posted once and held by the process.
- `examples/folder-viewer/src/use.html`, chapter 4 "Build from source": the line `cargo run -p ocelli-folder-viewer   # prints one URL on 127.0.0.1` becomes the release command.
- `examples/folder-viewer/src/more.html`, `#worker` "Worker decode and the wasm path": the sentence "This folder viewer posts the file to a local native process instead" says the file is posted once and held by id in the local process, which is still not the wasm path.

`use.html` and `more.html` join the write set for these edits.

## Open questions

**A. The budgets.** Affects AC17 and AC18. Decision owner: operator. Proposed thresholds for the release build on the host above, derived from E2 to E6: open of 1,693 by 512 at most 1,100 ms (no regression, the post per file stays), first image at most 20 ms on the CT series and at most 250 ms on the 17 MB DX, scroll at least 55 presented frames per second with zero blank samples (release) and zero blank samples on debug, window step at most 5 ms on 512 by 512 and at most 70 ms on the 17 MB DX, server RSS peak within the formula above, page heap within its formula. Deferral keeps AC17 open and the story cannot complete.

**B. The defaults.** Affects AC7, AC18 and the bound. Options: server budget 256 MiB (a 1,693-file CT keeps about 500 files and re-posts the rest at about 1 ms each, smaller RSS) or 512 MiB (fewer re-posts). Page grey cache 64 MiB or keep 256 MiB. Decision owner: operator. Recommendation 256 and 64, from "minimal memory". Deferral uses the recommendation.

**C. Encoded bytes after decode.** Affects the ownership table and section 8 compliance. Options: drop a single-frame object's bytes when frame 0 is held and keep a multi-frame object's bytes while any frame is not held (recommended), or always keep bytes until eviction. The second reads against section 8's "dropped as soon as a frame decodes". Decision owner: operator, with a `D-NN` row if the multi-frame half is judged a departure. Deferral keeps bytes until eviction and records the question.

**D. Release profile.** Affects AC17. Section 15.2's `opt-level = "z"` costs about 55 ms on a full 8.7 Mpx preview against `opt-level = 3` (E5), and nothing measurable on the held window step, which is chain-bound. Options: keep release (recommended), or add a `[profile.viewer]` inheriting release with `opt-level = 3` in the root `Cargo.toml`, which leaves release untouched. Decision owner: operator. Deferral keeps release.

**E. Story order in S46.** Affects the write set. Options: F-216 first (recommended, because F-210 and F-211 then build on `Viewer::frame` and F-208 on the request layer), or F-208 first with F-216 migrating its viewports. Decision owner: sprint lead.

**F. Live W/L on very large frames.** A 17 MB DX window step is chain-bound at about 40 ms server side plus an 8.7 MB reply. A reduced preview during the drag is resampling, which is pixel arithmetic with its own fixture and its own oracle question. Recommendation: out of scope here, a new story if the measured step misses A. Decision owner: operator.

**G. Where the measurement lives.** Options: `tools/bench` subjects and runners (recommended, it is the HLD section 26 instrument and already carries Playwright), or a Rust timing test. Decision owner: sprint lead.

**H. The uncommitted polish.** `page.html`, `serve.rs`, `more.html` and `use.html` carry uncommitted changes on `sprint/s46`, including `OCELLI_PORT`. This plan assumes they land before implementation starts. Decision owner: operator. Deferral blocks the start, because the write set overlaps.

**I. Header-only open.** Open posts every byte of every file once, which is 850 MiB for the measured folder. Posting only the bytes before Pixel Data would need `parse_part10` to accept a truncated object. Recommendation: out of scope, a new story if `viewer.open` misses A. Decision owner: operator.

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead. The operator accepted the recommended answer to every question below, and set the sprint build order.

- A. Answered: recommended, the proposed thresholds on the release build and the named host: open of 1,693 by 512 at most 1,100 ms, first image at most 20 ms on the CT series and at most 250 ms on the 17 MB DX, scroll at least 55 presented frames per second with zero blank samples on release and zero blank samples on debug, window step at most 5 ms on 512 by 512 and at most 70 ms on the 17 MB DX, server RSS peak within the stated formula, page heap within its formula.
- B. Answered: recommended, server budget 256 MiB and page grey cache 64 MiB.
- C. Answered: recommended (a), drop a single-frame object's encoded bytes once frame 0 is held, and keep a multi-frame object's bytes while any of its frames is not held. No `D-NN` row is written. HLD section 8 describes the core's own cache, and this plan's normative section records that the HLD does not specify this local process. The bytes kept are the ones still needed to decode the frames that have not decoded yet. If a reviewer reads this as a departure from section 8, the row is raised then, and implementation of the multi-frame half stops until it is approved.
- D. Answered: recommended, keep the release profile. No `[profile.viewer]`.
- E. Answered by the operator with the full sprint order: F-217, then F-216, then F-210, F-211, F-208, F-209, F-212, F-213, F-214 and F-215, with F-119 in parallel. Recorded under Dependencies.
- F. Answered: recommended, a reduced preview during a live drag is out of scope, and a new story is filed if the measured step misses A.
- G. Answered: recommended, the measurement lives in `tools/bench` subjects and runners.
- H. Answered: recommended, the uncommitted polish to `page.html`, `serve.rs`, `more.html` and `use.html` lands before implementation starts. It is a start precondition owned by the sprint lead, not a design choice, and no worker starts this story on a tree without it.
- I. Answered: recommended, a header-only open is out of scope, and a new story is filed if `viewer.open` misses A.

No question is left open.

### Lead amendments after review pass 1

Made by the sprint lead on 2026-11-05 under the operator's delegation of the quality call, after `.claude/reviews/F-216-working-pass-1.md`.

- **`ocelli-pixel` gains an additive API.** The non-goal above excluded any change to `ocelli-pixel`. The window path needs a table of display values for every stored sample, and building that table in the viewer would be a second copy of the LUT chain, which HLD section 18 forbids. So `LutChain::stored_sample_table`, `StoredSampleTable`, `StoredBits::meaningful_bits` and `StoredBits::stored_sample` are allowed in `ocelli-pixel`, each evaluating the existing chain and adding no arithmetic of its own. Every other change to `ocelli-pixel` is still out of scope. `docs/lld/pixel-pipeline.md` joins the LLD impact list.
- **The page heap bound counts only canvases the workload sizes.** The four tile canvases are counted only when the run sizes them. The `viewer.resident_memory` workload sizes the main view alone, so its bound is `64 MiB + 4 * W * H + 16 MiB` for the main view.
- **Debug blank samples are measured.** The scroll budget's debug half is taken by running `viewer.scroll_rate` once on the debug build, with the stage sampled on every animation frame, not only after a draw.

### Lead amendments after review pass 2

Made by the sprint lead on 2026-11-05 under the operator's delegation of the quality call, after `.claude/reviews/F-216-working-pass-2.md`.

- **The page heap bound is a bound on retained heap.** It is read after forced collection until two readings agree, because a heap read before collection counts greys the page has already discarded. The reading before any collection is reported beside it as information, and has no threshold.
- **The resident memory subject measures one process.** `viewer.resident_memory` runs the open, first image, scroll and window workloads against one server process with no restart, and the server's peak is that process's peak. The restart recovery check in `viewer.first_image` stays, outside this measurement.
- **Every byte the process holds for pixels counts.** Bytes waiting for a decode permit and any recycle pool are inside the 256 MiB budget or inside the formula's named terms. The bound and its formula do not change.

### Operator answer after review pass 2

Given by the operator on 2026-11-05, after the third remediation measured the folder viewer's live heap at about 330 MB while macOS libmalloc kept freed small and large magazines resident, so one process read 703 to 736 MB against the 528,194,192 byte bound.

- **The folder viewer binary uses mimalloc.** `examples/folder-viewer` declares `mimalloc` as its global allocator, pinned exactly in the workspace manifest, default features only, and no other crate uses it. It is MIT licensed, within `deny.toml` and `ci/licences.toml`. Declaring a global allocator needs no `unsafe` block of ours. The resident bound and its formula do not change, and the one-process run is measured again against them.

### Lead amendments after review pass 3

Made by the sprint lead on 2026-11-05 under the operator's delegation of the quality call, after `.claude/reviews/F-216-working-pass-3.md`.

- **`GET /held` also reports `inflight`, `workspace` and `free`.** The `/held` row in the scope table above is extended by this line, and corrected after review pass 4, because the upload pool this section removes no longer exists and the free list added later reports `free`. Each is a byte count the runner reads, and each doc comment says only what the counter counts.
- **The recycle pool and the sample-class reservation are removed.** Under mimalloc a build without them measured 492.2 and 500.2 MB, the same spread as with them, so they no longer pay for their code. An upload is held only while it is read and decoded, and its bytes are the formula's upload term.
- **Prose names the allocator, not its version.** The pin in the manifest is the one place the version is written.

### Operator answer after review pass 3

Given by the operator on 2026-11-05, after two long-lived decode threads bounded the thread count but a quiet one-process run still peaked at 566,018,048 bytes, with about 268 MB live and the rest held by the allocator for frames the cache had evicted.

- **An evicted frame's buffer is reused, not freed.** A decoded frame's sample buffer that leaves the cache goes to a free list keyed by its exact byte length, and the next decode of that length takes it instead of allocating. The free list is counted inside the 256 MiB cache budget, so cached frames plus free buffers never exceed it, and the cache evicts to make room for either. The bound and its formula do not change.
- **Fallback.** If ten one-process runs, five quiet and five beside a CPU load above 10, do not all stay under the bound with the free list in place, the story records allocator retention as a term of the formula and sets the bound to the measured worst case plus a stated margin.
- **User-facing documentation for the free list.** `README.md`, `examples/folder-viewer/src/use.html` and `docs/lld/cache.md` say that a decoded frame's sample buffer waits on a free list inside the 256 MiB budget. A request body is still freed on the thread that allocated it. The folder viewer binary is the one that uses mimalloc.

### Lead amendments after review pass 4

Made by the sprint lead on 2026-11-05 under the operator's delegation of the quality call, after `.claude/reviews/F-216-working-pass-4.md`.

- **Every decode runs on the two decode threads.** A preview that carries the file's bytes is decoded by the same scheduler as one that names a held id, under a decode permit and charged as in flight. No connection thread decodes.
- **A decode permit ends with the decode.** The permit is released when the grey bytes are ready, before the reply is written, so a client that stops reading holds no decode permit. Either decode thread takes the next job.
- **A buffer leaving the cache waits on the free list only when it fits.** When cached bytes plus free bytes would exceed the budget, the buffer is freed instead. The steal takes the oldest cached frame of that length only when no reply holds it, and otherwise allocates.

### Lead amendments after review pass 5

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-216-working-pass-5.md`.

- **A body-form preview never evicts or steals.** Its decode takes a free buffer of the right length when one is parked, and otherwise allocates. It never evicts a cached entry or steals a cached frame, because its frame is never stored.
- **Grey reply bytes are bounded by a cap, not by the permits.** Grey bytes held by replies that are not yet written count against one fixed cap equal to the formula's existing grey term. A preview whose grey would exceed it waits for capacity, up to the write timeout, then answers `503 busy`. The bound and its formula do not change.
