# F-202, Worker decode and owned frame render in the single showcase viewer

**Status**: approved
**Epic ref**: E38.12
**Sprint**: S47
**Estimate**: 6w

This plan replaces the round-two draft in full. It is design round 5. The
round-four review found 10 defects, 6 smells and 7 nitpicks, and confirmed
round three's D5 and D10 resolved, about 120 citations holding on `8949c801`,
both allowlists matching exactly, 95 decode names and 76 render names, and AC1,
AC2, AC18 to AC20, AC32 and AC33 holding in emulated f32. The rulings are
recorded under the round-four amendment section below. Rounds one to three
stand resolved otherwise: round one's D2 to D5, D8, D10, S4, S7, S11 and S12,
round two's eight rulings, round three's S1 to S14, the round-one and round-two
nitpicks, and the operator's three decisions of 2026-11-12 (Q-a, Q-b, Q-c),
which still stand and are recorded as decided under Open questions. The base is
`8949c801`, the head of `sprint/s47` in this worktree (`git rev-parse HEAD`,
run this session), which integrates F-267 and the approved F-204, F-205 and
F-241 plans. Scheduling and one-viewer scope were authorized by the operator on
2026-11-08. The estimate is a provisional lead engineering estimate, not
measured capacity or a calendar promise.

Every factual sentence changed this round was re-executed in this session
against `8949c801`, with the command named where the claim carries weight. The
round-four review had re-executed the unchanged citations on the same base.
AC16's window, displays and bytes were recomputed in emulated f32 this session
before they were written (`node`, `Math.fround`, over the transcribed
`auto_window.rs` and `lut.rs` arithmetic and `colour.rs`'s ramp), and the
allowlists were re-run with `cargo tree` on this base and match the recorded
lists exactly. Where a fact could not be verified from the tree or a standard
readable here, the plan says so plainly. The cornerstone 5.8.2 dist is not
installed in this worktree this session (`tools/oracle/node_modules` absent,
`ls`), so the two cornerstone reads are carried from the round-two and
round-three verification, marked as such, and are not re-verified here.

## Lead amendment after design round 2

The eight rulings are recorded here and applied in the sections named.

- **R1.** F-241 is approved, and its AC21 (`.claude/plans/F-241-design.md`
  line 261, read this session) adds an `ocelli-codec` edge to the render-worker
  module. F-202 must not add a guard that refuses `ocelli-codec` in the render
  module. F-202 instead restructures `scripts/decode_graph_check.py` into
  per-module allowlists: the render-worker and decode-worker builds each hold
  their resolved graph to an explicit named list, and any crate outside its
  list turns the guard red. F-202 lands first, and F-241 then adds its codec
  edge as one reviewed allowlist entry to the recorded render list. **The
  order is revised by the round-four amendment: F-204, F-205 and F-241 all
  land before F-202, so the render list F-202 records already carries the
  edge.** The
  functions the restructure touches are named where they live:
  `check_features`' render-worker pin at `scripts/decode_graph_check.py:72`
  to `:78` and `classify_graph` at `:220` to `:234`, the `decode-graph`
  catalogue entry at `scripts/guards/catalogue.py:10157` onward with its
  `decode-worker-dep` probe at `:10172` and `still-links` control at `:10255`,
  the guard test at `examples/viewer-react/src/csp-policy.test.ts:211` to
  `:259`, and `examples/folder-viewer/src/csp.rs`. All five are in the
  shared-file list under Dependencies with the order. Applied in Scope item 8,
  AC10, AC27 and Dependencies. The round-two sentence "which
  `decode_graph_check.py` holds both ways" is replaced by the allowlist
  mechanism, which does hold both ways and says how.
- **R2.** Stage once means parse once. The decode worker runs the bounded
  parse when the body is staged, keeps the parsed frame index and attributes
  for that instance, and each frame step decodes from that index without
  re-parsing. AC24's test sees a wasm-side parse count, a `parse_count` field
  in the facts record, and fails on a re-parse per frame. Applied in Scope
  items 1 and 5, the facts record, AC24 and D-34.
- **R3.** The separate decode budget is approved, so
  `scripts/pin_and_size_check.py` enters the write set: a decode-module record
  path writes `ci/wasm-decode-size.json` only through an explicit accept flag
  for that record, preserves the rebaselined-history shape the render record
  keeps, leaves `ci/wasm-size-budget.json` untouched, and measures gzip as
  well as raw. AC11 fails if the render budget file changes or gzip is
  unmeasured. Applied in Scope item 1, AC11 and Dependencies.
- **R4.** While the remote image is shown, nothing derived from the local
  image may be visible over it: the four overlay corners, `.scale`, the `.edge`
  orientation markers, `.ref-lines` and `.ink`. All are hidden by one named
  visibility owner for this story, the corners-from-record option is listed
  under Open questions with its cost, and AC31 fails if any local-image label
  is visible over the remote canvas. Applied in Scope item 5, AC31 and Open
  questions.
- **R5.** The native instance-reply validation is not weakened. The UID triple
  check is in the example's own server, `instance_reply`
  (`examples/folder-viewer/src/dicomweb.rs:313`), whose comparison of the
  three parsed UIDs against the expected ones is at `:366` to `:374` (read
  this session), and the provider's `decodeInstanceMetadata` identity check is
  at `packages/core/src/dicomweb-viewer.ts:418` to `:424` (read this
  session). The Rust crate's `consume_borrowed` refusal of a part whose media
  type is not `application/dicom`
  (`crates/ocelli-dicom/src/dicomweb.rs:221` to `:223`) is a different check
  and stays as cited. The UID mismatch is
  injected in the expected arguments of the page-to-decode message, through a
  test route specified in Scope item 5 with its location, its inert shipping
  and the way the test reaches it, and a second case keeps proving the native
  validation refuses a mismatched payload. Applied in AC3 and Scope item 5.
- **R6.** After a trap the body is gone by design:
  `takeStagedInstance` sets `this.#staged = null` at
  `packages/core/src/dicomweb-viewer.ts:1312`, read this session. Recovery
  refetches the instance, restages and displays, and the status sentence says
  it refetched. AC25 tests that path and requires a real wasm trap, not a
  forced JS worker error. The trap build reuses the existing `panic-probe`
  feature and its separate out-dirs, so the size gate's inputs are unchanged.
  Applied in Scope items 2 and 5, AC25, AC26 and Tests.
- **R7.** `read_profile` (`examples/folder-viewer/src/colour.rs:78` to
  `:110`) produces 15 kinds. All 15 are enumerated with `icc_kind` codes, and
  AC23 covers the two kinds the round-two draft omitted,
  `unsupported-colour-space` and `Lab PCS`. Applied in the facts record, AC23
  and the Tests table.
- **R8.** Enhanced multiframe per-frame rescale (Pixel Value Transformation
  Sequence, resolved by `rescale` at `examples/folder-viewer/src/preview.rs:2263` to `:2277`) and
  per-frame windows (Frame VOI LUT Sequence, the `(0028,9132)` tag at
  `preview.rs:74`, re-read this session) must not be quietly dropped on the
  remote route. Cornerstone 5.8.2 handles these for remote images, so refusing
  is below the floor. The generic per-frame resolver
  `MultiframeMetadata::resolve` exists at
  `crates/ocelli-dicom/src/multiframe.rs:234`, and the policy resolvers around it
  exist only in `preview.rs`. The recommended design extracts them into one
  crate module all callers share, measured in Scope item 9 at about 280
  written lines on the round-four extraction set, which also settles round
  one's S3 for this part. The facts record carries per-frame windows, because
  the record is
  written per decode. AC32 is the synthetic enhanced instance with differing
  per-frame rescale and windows, hand-computed, citing PS3.3 C.7.6.16.2.9 and
  C.7.6.16.2.10. Applied in Scope item 9, AC32, Dependencies and LLD impact.

The sixteen smells and eight nitpicks are applied in place, each at the
section its finding names. S13 is R6.

## Lead amendment after design round 3

The review found 10 defects, 9 smells and 9 nitpicks. D9 is the base move and
is applied by the re-execution itself. The rulings are recorded here and
applied in the sections named.

- **D1.** AC16 was wrong. `AutoWindowPool::add_frame` leaves the stored zero
  out (`is_stored_zero`, `crates/ocelli-pixel/src/auto_window.rs:273`, read
  this session), so a 0 to 99 fixture counts 99 values, not 100. Recomputed by
  hand from the code and PS3.3 C.11.2.1.2.1, the ranks are
  `99.div_ceil(100) = 1` and `(99 * 99).div_ceil(100) = 99`, so the percentiles
  are 1 and 99. **This ruling's centre 50, width 66 and its drawn bytes are
  withdrawn by the round-four amendment**: `margin_pair` moves the pair in f32
  steps, one `next_up` or `next_down` at a time, not integer units. The
  corrected window, displays and bytes are AC16's.
- **D2.** `parse_count` changes only at staging, never per decode. Scope item
  1's `decode_staged` sentence, which had it bumped per decode, is corrected to
  match the record's own definition and AC24, and the stray `1.` list marker in
  AC32 is fixed.
- **D3.** AC12 is rewritten. The page's outbound transfers are exactly one
  staged body per stage and one OffscreenCanvas per spawn, and no ArrayBuffer
  travels from either worker to the page. The spies assert that exact set.
- **D4.** R8 stands and is widened to exactly one copy. The extraction moves
  the whole resolver stack the remote route needs into the crate function set:
  the string-slot resolution `resolved_slots`, including `WINDOW_EXPLANATION`
  so `window_rows` keeps working, the text resolver `resolved_text`, which
  `value.rs:190` reads for Rescale Type, and the presence check `group_has`.
  `preview.rs`, `value.rs` and `remote.rs` all call the crate and nothing keeps
  a local copy. Every call site is listed by grep under Scope item 9, one
  measured line count is given, and the local route's existing suites
  (`examples/folder-viewer/tests/preview.rs` and `value.rs`) are named as the
  no-behaviour-change guard.
- **D5.** Every frame step that changes the shown frame produces a facts
  record, including a resident hit. The decode worker keeps the per-frame facts
  for the staged instance and the resident path publishes the stored record.
  New AC33 steps 0 to 1 to 0 on AC32's instance and asserts frame 0 draws at
  its own window, 87 and not 0. Applied in Scope items 2 and 7 and AC33.
- **D6.** R4 is made real with one gate. The page field is `vp.remoteDisplayed`
  (S8). Every painter that writes any of the eleven elements, the hover
  crosshair and the magnifier checks it and keeps its element hidden while the
  remote frame shows. The painters are named with their measured lines under
  Scope item 5. AC31 exercises the corners toggle, a hover with the cross on
  and the magnifier on, over the remote frame.
- **D7.** The remote canvas is visible whatever the local layout did. It lives
  in its own element in the cell outside `.stage`, shown by remote activation
  and hidden on its end, and its behaviour in tile mode, for an empty local
  cell and after a local fault is stated. New AC34 asserts a non-zero visible
  bounding box in each of those three cases. Applied in Scope items 5 and 6
  and AC34.
- **D8.** The write set is exact. `readFactsRecord` is dropped and the record
  is read with the landed `exportDecodedFrame(wasm, facts_ptr(), 76)`
  (`bulk.ts:401` to `:410`, read this session), so `bulk.ts` stays read-only.
  `examples/viewer-react/probe-decode-worker.mjs`,
  `examples/folder-viewer/tests/browser/dicomweb.mjs` and the existing
  `scripts/tests/test_pin_and_size_check.py` join the write set, and the LLD
  entry names the exact files.
- **D10.** Both allowlists are written down by name in this plan, render and
  decode, computed from `cargo tree` on `2a13dbf3` in the round-four
  session and re-verified on `8949c801` this session: 95 names
  decode, 76 names render. Each graph gets its own probe, so an unlisted crate
  in either graph turns the guard red, and the `still-links` probe keeps
  feeding the wgpu and naga fast path instead of being repurposed. Applied in
  Scope item 8, AC27 and Dependencies. The render list's F-241 delta is
  recorded by the round-four amendment under Dependencies.
- **S1.** The "holds both ways" sentence is replaced by the truth: what
  F-241's AC21 must amend is stated plainly, its render-list entry and the
  re-measurement of its decode budget claim, under Dependencies. The
  coordinator amends F-241, this plan only states the need.
- **S2.** The native UID triple check is
  `examples/folder-viewer/src/dicomweb.rs:366` to `:374` inside
  `instance_reply` (`:313`), not `crates/ocelli-dicom`. Both citations are
  fixed, in the round-two amendment paragraph and AC3.
- **S3.** Test arming is not live in production. The arming flags and the
  trap-build serving are gated on a test-only server setting, the environment
  variable `OCELLI_TEST_ARMING` read once by `serve.rs`, off by default. AC25
  traps the very instance holding the staged body, so under arming the decode
  route the worker imports is itself the panic-probe build. Applied in Scope
  items 3 and 5, AC25 and AC26.
- **S4.** AC28 is extended with one colour fixture and one ICC fixture
  compared between the two routes, the cheaper option, measured at about 100
  written lines with no production move. The extraction alternative is
  recorded as rejected with its cost. Applied in AC28 and Open questions.
- **S5.** The gzip record names the level (9), a hard ceiling rule, and what
  stale means. The two trap builds are not added as separate CI steps, with
  the reason stated. Applied in AC11 and Dependencies.
- **S6.** The eslint worker block is specified: file globs, worker globals,
  the `ocelli/no-wasm-memory-view` stance, and `probe-decode-worker.mjs`.
  Applied in the eslint write-set item, 33 after the round-four
  renumbering, and the Approach's eslint subsection.
- **S7.** The sample buffer is sized by the resolved sample kind, not 12 bytes
  per pixel for every frame, so a 2560 by 3328 monochrome frame within the
  declared bound is not refused. Applied in Scope item 1.
- **S8.** The page field is named, `vp.remoteDisplayed`, the same field D6
  gates on, and remote activation drops a held 1:1 zoom mode to fit, which
  F-204's revised plan expects. Applied in Scope item 5.
- **S9.** A gate test line with `--no-default-features --features
  decode-worker` is added so the decode-only slot tests compile and run.
  Applied in Verification and Dependencies.

The nitpicks: the arming paragraph said two flags and listed four, fixed. AC20
says `255 - 63.910 = 191.090`, and the route is `reference_ramp_index` with
`round_half_even`, fixed. AC16's "so so" and AC32's "computed as hand-computed
as" are fixed. `DECODE_EXPORTS` already exists at
`scripts/pin_and_size_check.py:93` to `:98` and is extended rather than
invented. `bin/ocelli.sh`'s clippy arm is at `:109`. The session cfg is
`crates/ocelli-wasm/src/lib.rs:47` to `:48`. `still-links` starts at
`scripts/guards/catalogue.py:10255`. The D-35 draft's "HLD says" column quotes
HLD text, not a script. Each is applied at the section it names. Round three
had 8 nitpicks, corrected here and in this plan's header from the 9 both once
wrote.

## Lead amendment after design round 4

The round-four review found 10 defects, 6 smells and 7 nitpicks. The rulings
are recorded here and applied in the sections named.

- **D1, AC16's window.** `margin_pair`
  (`crates/ocelli-pixel/src/auto_window.rs:537` to `:550`, read this session)
  moves the percentile pair by `PERCENTILE_MARGIN` 16 f32 steps, one
  `next_up` or `next_down` at a time, not 16 integer units. The round-three
  ruling of centre 50, width 66 is withdrawn. Recomputed in emulated f32 this
  session over the transcribed arithmetic: centre
  49.99993896484375, width 97.9998779296875, LINEAR_EXACT, the displays 0,
  39.031, 124.898, 210.766 and 252.398, and the bytes 0, 39, 125, 211 and 253
  through the 1024-wide ramp, whose bin lifts 252.398 to 253. The
  zero-counting mutant centres at 48.99993896484375 with bytes 0, 42, 128,
  213 and 255. Applied in the auto window bullet of the colour routes
  transcription and in AC16.
- **D2, parse_count.** The count is incremented inside the single parse
  helper on every bounded parse and is never assigned. A Rust fixture calls
  `decode_staged` three times and reads 1 every time, so a mutation moving
  the parse into `decode_staged` goes red. The stray `1.` list marker at the
  Scope item 1 sentence is removed. Applied in Scope item 1, AC24 and the
  Tests table.
- **D3, AC12's exact outbound set.** Per spawn, one MessagePort to each
  worker and one OffscreenCanvas to the render worker. Per stage, one body
  ArrayBuffer to the decode worker. Per animation-frame batch, one command
  ArrayBuffer to the render worker, HLD 24's
  `main -> render : { kind:'commands', buffer } [transfer]` line. The lead
  chooses that form. Worker to page stays zero ArrayBuffers. Applied in AC12.
- **D4, one clear and early returns.** Only the named release function
  `remoteRelease` clears `vp.remoteDisplayed`. Every local paint entry,
  `applyGrey` (`page.html:2776`), `scheduleLive` (`:4298`), `paintTiles`
  (`:3492`) and F-205's hold-release repaint, returns early for a viewport
  whose field is true. `drawGrey` no longer clears the field, and the
  round-three painter-gate mechanism (each painter checking the field and
  keeping its element hidden) is replaced by the entry early returns, which
  supersedes the round-two R4 sentence, the round-three D6 mechanism and
  F-204 decision 15's clearing sentence as far as this story's page work
  goes. Applied in Scope item 5,
  the consumer map, the state transitions, the ownership table and Open
  questions.
- **D5, the routing table.** Scrubber, wheel, arrow keys, cine, Fit and 0,
  the one-to-one control and its key, W/L drag, presets, invert, reset,
  zoom, pan, rotate and flip each either dispatch to the remote path or
  refuse with a named sentence on `#remote-state` while the field is true.
  `commitLabels` and `showIndex` are never called for the remote frame, and
  the remote route creates no series entry. Approach item 5 and the
  "no existing handler is rewritten" sentence are void. Applied in Scope
  item 5 and Approach.
- **D6, the wrapper's place.** The remote wrapper is a direct child of
  `.cell`, a sibling of `.frame` and `.tiles`, cell-sized, opaque, and
  stacked above `.frame`, `.tiles`, `.fault` and `.cell-empty`
  (`page.html:819` to `:843`, read this session). Applied in Scope item 6
  and AC34's wording.
- **D7, eslint.** The three worker files join the single enforcement block's
  files and its project-service setup, typechecked as JS under the existing
  `ocelli/no-wasm-memory-view` and `no-restricted-syntax` rules, so
  `assertTheBanIsIntact` (`eslint.config.js:150` to `:203`) still sees one
  block of each. Applied in Approach's eslint subsection and write-set
  item 33.
- **D8, the census record.** `ci/guard-probe-budget.json`, written by
  `python3 scripts/guard_census.py --record`, joins the write set. The pins
  entry gains a probe per new `pin_and_size_check.py` refusal, and the
  `refuses=` text of the `pins` and `decode-graph` entries is updated.
  AC11 and AC27 cite the record. Applied in Dependencies, AC11, AC27 and
  Verification.
- **D9, the gzip rule.** The gzip ceiling is the recorded gzip times
  `1 + TOLERANCE`, measured at level 9, the same rule as raw
  (`TOLERANCE = 0.05`, `scripts/pin_and_size_check.py:241`, read this
  session). A stale record is refused only at accept time, never by gate
  wasm. Applied in AC11 and Dependencies.
- **D10, the one-to-one seam.** Built on the coordinator's section: the
  activation function is named and calls `fitView(vp)`
  (`page.html:3865`, read this session), `blankViewport` carries
  `remoteDisplayed: false` (`:1362`), `remoteRelease` (`:1467`) hides the
  wrapper, clears the field, calls `layoutViewport` and repaints the
  painters, a "Show local" release trigger keeps the local series, refusals
  go to `#remote-state` (`:753`) with its panel `section#remote` (`:751`)
  forced visible while the field is true, and AC35 covers the seam. The
  series-structure sentence in the coordinator section is corrected. Applied
  in Scope item 5, the coordinator section and AC35.

The smells, applied in place:

- **S1, the existing per-frame resolvers.** `ocelli_dicom::open_frame`
  (`crates/ocelli-dicom/src/frame_open.rs:365`) and `series_open.rs`'s own
  resolver set are weighed against D4's extraction. The recommendation, with
  its measured cost, is recorded in Scope item 9.
- **S2, who builds the trap probes in CI.** The viewer-browser job builds
  `decode-trap-probe` and `render-trap-probe` before the suites run, on the
  gate arm pattern of `bin/ocelli.sh:161` to `:167`, so AC25 and AC26 do not
  fail by name in CI. Applied in Dependencies and both ACs.
- **S3, the armed page's CSP.** The armed policy is `configured_main_policy`
  over the armed page text, and the unarmed page bytes and policy stay
  unchanged. Applied in Scope item 3.
- **S4, AC31's visibility check.** AC31 checks computed visibility, covers
  `.ref-lines` by its `style.display` and `.ink` by its children, and the
  `.tile-label`, `.fault` and `.cell-empty` states. Applied in AC31.

Smells 5 and 6 and the seven nitpicks are cheap text corrections, each
applied at the section it names:

- The allocation claim made true or narrowed, Scope item 1 and Boundary
  and tier.
- The `bin/ocelli.sh` line numbers, the viewer-browser arm at `:219` to
  `:237` and the wasm gate arm at `:296` to `:301`, in AC9, Exclusive
  resources and Verification.
- `drawLine` at `page.html:6969` and `clearLine` at `:6983`, Scope item 5
  and AC31.
- The one-slot rule at `session.rs:158` to `:160`, Scope item 1.
- `dicomweb.rs:221` to `:223` in one form everywhere, the round-two
  amendment and the decode parse route transcription.
- The `more.html` worker section to about `:588`, AC13 and the
  documentation impact section.
- D-35 quoting the F-110 guard it departs from,
  `scripts/decode_graph_check.py:68` to `:69`, instead of HLD text.
- The cornerstone readability sentences rewritten and the `init.js`
  `maxWebWorkers` override named, in the S8 transcription section.
- Round three had 8 nitpicks, corrected in this plan's header and the
  round-three section.

## Normative source, transcribed

### HLD section 24, the worker protocol (`docs/hld/21-worker-protocol.md`)

```text
main -> decode : { kind:'decode', seriesId, frameIndex, buffer } [transfer]
decode -> render : { kind:'frame', frameId, buffer, meta } [transfer]
main -> render : { kind:'commands', buffer } [transfer]
render -> main : drained event ring [copy]
// Always transfer, never copy:
// worker.postMessage(msg, [msg.buffer]);
```

```text
Three roles: the main thread, N decode workers each with its own
WebAssembly instance, and one render worker owning the GPUDevice and every
OffscreenCanvas. Decode workers never touch the GPU; the render worker never
decodes.
```

This section decides the story's shape. The staged instance body is decoded in
a decode worker that holds its own WebAssembly instance. The frame reaches the
render worker as a transferred buffer. The OffscreenCanvas belongs to the
render worker. The main thread never holds pixels. The section's decode line
carries the body on every request. This story stages it once per displayed
instance instead, which is deviation D-34 below, recorded because the
specification says buffer and this plan says once.

### HLD section 9, concurrency (`docs/hld/07-concurrency-and-typescript.md`)

```text
**Start single-threaded per worker.** N decode workers, each holding its own
WebAssembly instance; one render worker owning the GPUDevice and every
OffscreenCanvas; the main thread doing DOM, events and tool UI. No
SharedArrayBuffer, no wasm-bindgen-rayon, no nightly toolchain.
```

Decision D5 restates this. This story uses one decode worker and one render
worker, one instance each, no shared memory. The transfer is the only route
for the frame.

### HLD section 17.2, the bulk channel and the trap (`docs/hld/14-the-boundary-in-code.md`)

```typescript
const ptr = session.alloc(bytes.byteLength);
// Build the view AFTER the allocation. Use it immediately. Let it go.
new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);
session.commit_frame(ptr, bytes.byteLength, meta);
```

```text
NEVER CACHE THE VIEW A module-level const HEAP = new Uint8Array(wasm.memory.buffer) is the
classic failure. Any wasm memory growth relocates the ArrayBuffer and
detaches every outstanding view; the next write silently targets a detached
buffer or throws far from the cause. Add an ESLint rule banning new
Uint8Array(wasm.memory.buffer) outside the two functions that are allowed to
do it.
```

The staged body reaches the decode instance through this order, in the
decode-worker build's own bulk slot. The one write is `holdSessionBytes`
(`packages/core/src/bulk.ts:343` to `:369`, read this session), which
releases a previous ticket, allocs, tickets and writes once. The new worker
scripts never build a view over `wasm.memory.buffer` themselves. The ESLint
production allowance stays `bulk.ts` and `panic.ts`
(`eslint.config.js:282`). The worker scripts import `panic.js` as well,
because section 23's record is how a trapped worker reports what happened
without calling into the poisoned instance again.

### HLD section 23, the record a trap leaves, and who reads it

`packages/core/src/panic.ts:133` (`readPanicRecord`, read this session) is the
reader the section 17.2 rule permits to touch linear memory, after the
instance has trapped, because no wasm code can run. Its own header states the
order: the shell calls `panic_record_ptr()` and `panic_record_len()` once at
instantiation, caches the two integers, and never calls into the instance
again. In this story the instance is held by a worker, so the worker is the
reader: a wasm export call that traps throws in the worker's JS, the worker
catches it, reads the record from the cached pointer and length with no
further export call, and reports the trap to the page. The worker instance is
never reused after that. This is the reading section 23's own probe takes:
`scripts/panic_probe.mjs:172` to `:178` catches the trigger's throw and reads
the record with no export call in between.

### HLD section 17.3, the event ring

```rust
#[repr(C)]
pub struct Event {
    pub kind: u32,
    pub viewport: u32,
    seq: u64,
    pub payload: [u8; 32],
}
// 48-byte stride. JS reads with a DataView at header_size + i * 48.
```

```text
Fixed stride so the JavaScript side is arithmetic, not deserialisation.
Drain once per animation frame; surface `dropped` in telemetry rather than
swallowing it.
```

The render worker drains its ring with the existing copied drain
(`drain_events`, `crates/ocelli-wasm/src/session.rs:1546`, whose comment
states there is no `ring_ptr`). A dedicated worker has no animation frame, so
the once-per-animation-frame cadence is the page's own per-frame batch
cadence relayed into the worker. The worker drains once per applied command
batch and on a bounded timer as a fallback, and the timer only drains, never
draws. `dropped` is surfaced. This is the HLD rule kept, with the cadence
named rather than a deviation raised.

### HLD section 18, the LUT chain (`docs/hld/15-lut-chain.md`)

```text
This is the highest-risk arithmetic in the project. It is specified in DICOM
PS3.3 C.11 and the stages apply strictly in order. Implement it once, in
ocelli-pixel, and let the shader read the parameters — do not let a second
copy of this logic appear anywhere.
```

| Stage | From, To | Source |
|----|----|----|
| 1. Modality LUT | Stored to Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2. VOI LUT | Modality to Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3. Presentation LUT | Display to Display | Identity or INVERSE |
| 4. Palette / ICC | Display to RGB | Palette colour LUT, or the display colour pipeline |

Section 18.2, the three functions, transcribed exactly.

```text
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
// x <= c - w/2 -> ymin
// x > c + w/2 -> ymax
// else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
// y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
```

Section 18.3, the fixture rows. Centre 40, width 400, output range 0 to 255.

| Input (HU) | LINEAR | LINEAR_EXACT |
|----|----|----|
| -160 | 0.000 | 1.594 |
| 40 | 127.819 | 127.500 |
| 240 | 255.000 | 255.000 |
| -60 | 63.910 | 63.750 |

D-13 records that the LINEAR_EXACT value at -160 is reconciled to the
formula, which yields 0.000. The displayed byte is the raster's own rule and
rounds half to even. The monochrome display byte is `round_half_even`
(`crates/ocelli-render/src/colour.rs:481`, reached through the 1024-wide
ramp at `colour.rs:446` to `:479`), and the colour channel byte is
`round_ties_even` (`crates/ocelli-render/src/cpu.rs:1169`), the same rule
under F-239's operator decision, read this session. So the LINEAR column
draws as 0, 64, 128, 255 for those four inputs through a rescale of slope 1,
intercept 0. AC1 asserts exactly those four canvas bytes.

`preset_index` (`crates/ocelli-render/src/colour.rs:424` to `:430`, read this
session) chooses the route: a grey preset with the ramp flag set reads the
1024-wide `reference_ramp_index`, and everything else, SIGMOID among it,
reads `display_index` (`colour.rs:518`). `linear_byte_ramp`
(`colour.rs:432` to `:442`) sets the ramp flag for `LINEAR` and
`LINEAR_EXACT` with a zero floor and a 255 ceiling only, so AC18 and AC1
draw through the ramp and AC19's SIGMOID draws through `display_index`. That
is the correction the round-three nitpick asked for.

Section 18.4's `fn_kind` comment reads `0 LINEAR, 1 LINEAR_EXACT,
2 SIGMOID`, and the viewport boundary constants agree:
`VOI_FUNCTION_LINEAR = 0`, `VOI_FUNCTION_LINEAR_EXACT = 1`,
`VOI_FUNCTION_SIGMOID = 2` (`crates/ocelli-viewport/src/boundary.rs:128` to
`:132`, read this session). The facts record below uses those numbers and the
tree's name SIGMOID. Cornerstone 5.8.2's own enum is a different order:
`VOILUTFunctionType` is `LINEAR`, `SAMPLED_SIGMOID`, `LINEAR_EXACT`
(`tools/oracle/node_modules/@cornerstonejs/core/dist/esm/enums/VOILUTFunctionType.js`,
read this session), with the value string `"SIGMOID"`. So the parity claim
between the two is by the value string only, never by index. The More row
under AC13 carries that wording.

Where stage 1 and stage 4 run is decided by the operator's F-235 answer,
recorded in `docs/sprints/CURRENT_SPRINT.md` line 83 and transcribed here.

> O1-prime: integer stacks read a folded 256-wide modality table built by
> `ocelli_pixel::modality`, and interpolated routes apply stage 1 on the CPU
> before upload. Stage 1 stays in `ocelli-pixel` alone.

That answer governs the shader route F-235 builds. This story adopts it by
analogy for the decode route, which is a different consumer of the same
principle. The decode worker applies stage 1 in `ocelli-pixel` before the
frame is transferred, and the render session's own stage 1 is the identity
rescale 1.0/0.0 that `StackViewport::try_identity` builds
(`crates/ocelli-viewport/src/commands.rs:135` to `:152`), so the transferred
values pass through stage 1 exactly once. The window rides as commands in
modality units, which is what the file's Window Center and Window Width
already are. The analogy is recorded as such, and the D-34 deviation below is
this story's own authority for what crosses per request.

### PS3.3 C.7.6.16.2.9 and C.7.6.16.2.10, per-frame rescale and windows

The lead's ruling R8 names the two functional-group macros. This session
could not open PS3.3 itself, so what is transcribed here is the tree's own
reading of them, each with the line that carries it:

- `crates/ocelli-pixel/tests/auto_window.rs:6` cites C.7.6.16.2.9 as
  "per-frame rescale in an enhanced object".
- `examples/folder-viewer/tests/value.rs:744` computes frame 1's modality
  from frame 1's own functional group, "PS3.3 C.7.6.16.2.9. Frame 1's
  functional group gives 1040 * 1 + (-1000)".
- `examples/folder-viewer/src/preview.rs:2263` to `:2277` resolves Rescale
  Slope and Rescale Intercept per frame through
  `resolved_decimals(multi, frame, PIXEL_VALUE_TRANSFORMATION, ...)` with
  `TopLevelFallback::Allowed` (`preview.rs:2150`, read this session), so a frame's
  own Pixel Value Transformation Sequence wins and the top level is the
  fallback.
- `preview.rs:1287` to `:1291` reads Window Center and Window Width the same
  way under `FRAME_VOI_LUT_SEQUENCE`, the `(0028,9132)` tag at
  `preview.rs:74`, so a frame's own Frame VOI LUT Sequence wins and the
  top-level window is the fallback. The default function is `LINEAR`
  (`preview.rs:2279` to `:2287`).
- A Modality LUT Sequence in the per-frame group is refused, not
  approximated: "A Modality LUT Sequence replaces rescale, so identity slope
  is wrong" (`preview.rs:1228` to `:1242`).

That is the behaviour the remote route must reproduce, and it is what the
extraction under Scope item 9 moves into one crate module both callers use.

### HLD section 8, memory and cache (`docs/hld/06-memory-and-cache.md`)

> One budget, three tiers, explicit eviction. Encoded bytes are transient and
> dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by
> the caller.

The render session's `ImageCache` is that LRU and F-231 landed it. The caller
sets the budget. The staged body is transient and is dropped at release and
at replacement.

### HLD section 7, tiers (`docs/hld/05-rendering.md`)

```text
Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048.
Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only,
no compute, no storage buffers, a conservative 3D-texture floor of 256.
Every feature declares the tier it needs; the tier resolves once at startup.
```

Deviation D-07 adds tier C. The remote path draws through the CPU raster on
every tier, which is the answer all three rows give in the Boundary and tier
section.

### HLD section 26, performance rules (`docs/hld/23-performance-rules.md`)

> No allocation in the render loop.
> One queue.submit() per frame.
> Prefer a uniform update to a texture update. Window/level is thirty-two
> bytes, not a re-upload.
> Batch pointer events into one command buffer per animation frame. Never
> cross the boundary per event.
> Measure with the benchmark harness before optimising anything. The
> intuitions that work in JavaScript do not transfer.

On the CPU raster path the same claim is served by redrawing from the
committed stored samples. A window change is a redraw, not a re-decode and
not a re-fetch. Pointer gestures are batched into one packed command buffer
per animation frame.

### The landed F-245 boundary this story plugs into

`packages/core/src/dicomweb-viewer.ts`, read this session. The provider
stages one bounded WADO-RS instance body, 32 MiB cap at line 1090
(`maxResponseBytes: 32 * 1024 * 1024`), native validation reply capped at
16 MiB at line 1194 (`readBoundedBody(response, 16 * 1024 * 1024)`), and
hands it over exactly once. The staged representation carries the multipart
boundary at line 372.

```text
export type OwnedEncodedInstance = { body: ArrayBuffer; metadata: StagedMetadata };
takeStagedInstance(expected: StagedMetadata): OwnedEncodedInstance | null
readStagedInfo(): StagedMetadata | null
releaseStaged(): void
```

`OwnedEncodedInstance` is at line 375, `takeStagedInstance` at 1233,
`readStagedInfo` at 1325 and `releaseStaged` at 1331. The boundary is
`StagedMetadata.representation.boundary`,
`representation: Readonly<{ field: "wado-rs-instances"; boundary: string }>`,
which the decode route needs to parse the multipart and which the page
carries with the body.

`takeStagedInstance` consumes: after the checks it runs
`this.#staged = null;` at line 1312 and returns a `structuredClone` transfer
of the body (lines 1312 to 1320, read this session). The page therefore
holds no copy of the body once it has taken it. R6's recovery is the
consequence and is designed under Scope item 5.

The native instance-reply validation the story must not weaken is in two
layers. The provider's is `decodeInstanceMetadata`
(`packages/core/src/dicomweb-viewer.ts:384`), which refuses a payload whose
identity triple disagrees with the requested one (lines 418 to `:421`) and
throws the `CommitRefused` native-body error. The Rust layer is
`DicomwebSource::consume_borrowed`'s WADO-RS arm, which refuses a part whose
media type is not `application/dicom`
(`crates/ocelli-dicom/src/dicomweb.rs:221` to `:223`) and parses each Part 10
part under the bounded limits (`dicomweb.rs:227` to `:236`). The browser
suite already has the refusal case for the native check
(`examples/folder-viewer/tests/browser/dicomweb.mjs:4` to `:5` and the
assertion near `:465`, read this session), and it stays green.

`docs/lld/dicomweb.md` lines 207 to 209 state the boundary this story owns.

> The fixed product status is that the instance is fetched and validated,
> remote image display is not available yet, and the local image is unchanged.
> F-202 owns remote display, and no decode or render runs on this path.

The page's landed status sentence is at
`examples/folder-viewer/src/page.html:1592`, set by `remoteFetchInstance`
after `fetchSelectedInstance` resolves. F-202 replaces that sentence with the
displayed state and keeps the sentence's shape, no story ID in product copy.
The page's release machinery is `remoteRelease` at `page.html:1467`, which
bumps `remoteEpoch`, cancels the provider and releases the staged body, and
the idle sentence is "The DICOMweb panel is idle."

### The decode parse route in the tree

`DicomwebSource::consume_borrowed` is at
`crates/ocelli-dicom/src/dicomweb.rs:204` to `:243`. It takes
`&SourceResponseKind` and refuses anything but `QidoJson` and
`WadoRsInstances { boundary }`, and for a multipart body parses the parts and
each Part 10 payload under `SourceResponseLimits`
(`encoded_body_max`, `decoded_max`, `max_parts`, `max_rows`,
`dicomweb.rs:166` to `:177`). The decode route stages the body and the
boundary and calls this with `WadoRsInstances { boundary }`.

The session's bulk cap is `MAX_BULK_BYTES`, 64 MiB
(`crates/ocelli-wasm/src/session.rs:23`). A frame whose stored samples exceed
that cannot commit through the render session's slot, so the decode route
refuses it by name before producing bytes.

The decode build has no `Session`: the session module is
`#[cfg(feature = "render-worker")]` (`crates/ocelli-wasm/src/lib.rs:47` to
`:48`, read this session), so the decode route carries its own slot struct
that mirrors the render session's ticketed `Bulk` rules. The rules it mirrors
are the ones `session.rs` states at `:157` to `:175`: one outstanding slot,
a second `alloc` returns 0 and keeps the first, `MAX_BULK_BYTES` bounds the
size, and the ticket, not the address, is the authorization
(`release_bulk` at `:681` to `:687`, `ticket_is_current` at `:689` to
`:694`).

### The colour, window and ICC routes this story reuses

- `Session::remember_frame` (`crates/ocelli-wasm/src/session.rs:702` to
  `:720`) parses the committed bytes as `f32::from_le_bytes`, four bytes per
  sample. A `Stored` is one `f32`. So interleaved RGB `u8` samples cannot
  commit through this route, and the remote route transfers three `f32` per
  pixel, 12 bytes per pixel.
- `CpuRaster::accept_rgb_source` (`crates/ocelli-render/src/cpu.rs:457` to
  `:464`) requires exactly three `Stored` per pixel,
  `RasterError::Length` otherwise, and
  `ColorTransform::accept_source` (`crates/ocelli-pixel/src/color.rs:744`
  to `:762`) requires `source.len() == pixels * stored_samples_per_pixel()`.
- The render side's native RGB transform is the identity.
  `interleaved_native_rgb()` (`cpu.rs:966` to `:983`) resolves
  `SampleLayout::new(3, PhotometricInterpretation::Rgb, Interleaved)` with
  `PixelDataEncoding::Native`, `DecodedPhotometric::Preserved`,
  `DecodedLayout::Preserved` and no palette, which resolves to
  `ColorSpace::Rgb` with `sample_scale: DisplayScale::IDENTITY` and
  `icc: None` (`color.rs:593` to `:642`), and the `Rgb` arm with no ICC
  returns the raw triple (`color.rs:836` to `:839`). So stage 4 runs once,
  on the decode side, and the render side passes the samples through.
- The session's colour draw applies `ColourWindow::initial()`
  (`cpu.rs:591` to `:603` and `:947` to `:963`, read this session), which is
  the skipped identity window of D-29, so a colour remote frame's bytes are
  its display samples. The session's colour window is not command driven.
- Inversion is resolved once in the chain. `PresentationTransform::new`
  (`crates/ocelli-pixel/src/lut.rs:503` to `:535`, the shape match at
  `:526` to `:533`, read this session), which `LutChain::new`
  (`lut.rs:596` to `:608`) calls, maps
  `PresentationLutEvidence::Absent` over `Monochrome1` to `Inverse` and an
  explicit `Shape` wins over the photometric, so an explicit `Shape(Identity)`
  is not inverted. The inverse maps one display value to
  `ymin + ymax - display` (`lut.rs:572`). `Presentation::set_invert`
  (`crates/ocelli-viewport/src/presentation.rs:148` to `:168`, read this
  session) stores an explicit shape and does not consult the photometric
  again. The file's MONOCHROME1 inversion and a user's invert command
  compose as one resolved stage, never two.
- The windowless rule is the local route's. `FileWindow::Auto`
  (`examples/folder-viewer/src/preview.rs:713` to `:721`) takes
  `ocelli_pixel`'s automatic window, LINEAR_EXACT from the 1st to the 99th
  percentile, when the file stores no Window Center and no Window Width and
  no VOI LUT Sequence. `auto_window`
  (`crates/ocelli-pixel/src/auto_window.rs:661` to `:669`) computes it from
  the stored samples, the modality transform and the Pixel Padding Value
  range, as a pool of one frame. The ranks are the nearest-rank rule
  `counted.div_ceil(100)` and `(99 * counted).div_ceil(100)`
  (`auto_window.rs:410` to `:411`), the pair is then moved up to 16 f32
  steps toward each other, one `next_up` or `next_down` at a time
  (`margin_pair`, `auto_window.rs:537` to `:550`, called at `:455`,
  `PERCENTILE_MARGIN` 16 at `:520`, the cap `min(16, (d - 1) / 2)` stated at
  `:532`), the first width tried is the moved pair's difference
  (`auto_window.rs:463`), the centre is `low_moved + width / 2` (`:465`),
  and the first width that passes the four-point check is returned
  (`search_window`, `auto_window.rs:623` to `:653`). A file window's pairs
  are paired by position per PS3.3
  C.11.2.1.2.2, and with no index the default is the first slot holding both
  a centre and a width (`preview.rs:1681` to `:1693`, read this session),
  reported as `window_index`.
- The ICC route is F-240's. `read_profile`
  (`examples/folder-viewer/src/colour.rs:78` to `:110`, read this session)
  classifies `IccStatus::Absent`, `Applied` or `NotApplied(kind)`, and an
  applied profile is attached with
  `transform.with_icc(IccTransform::from(profile))`
  (`colour.rs:162`). D-30 records the policy: a profile outside
  the subset is drawn in device RGB with the page naming the profile kind.

### The 15 kinds `read_profile` names (R7)

The `match` at `examples/folder-viewer/src/colour.rs:89` to `:109`, read
this session, produces exactly these 15 kinds, in source order. The
`icc_kind` code is the record's number for each.

| code | kind | arm |
|------|------|-----|
| 1 | `lookup-table` | `IccError::LookupTableProfile` |
| 2 | `printer` | `UnsupportedClass` with class `prtr` |
| 3 | `device-link` | `UnsupportedClass` with class `link` |
| 4 | `colour-space` | `UnsupportedClass` with class `spac` |
| 5 | `abstract` | `UnsupportedClass` with class `abst` |
| 6 | `named-colour` | `UnsupportedClass` with class `nmcl` |
| 7 | `unsupported-class` | `UnsupportedClass` fallback |
| 8 | `greyscale` | `UnsupportedColourSpace` with space `GRAY` |
| 9 | `unsupported-colour-space` | `UnsupportedColourSpace` fallback |
| 10 | `Lab PCS` | `UnsupportedPcs` with PCS `Lab ` |
| 11 | `unsupported-PCS` | `UnsupportedPcs` fallback |
| 12 | `incomplete` | `IccError::MissingTag` |
| 13 | `unsupported-curve` | `IccError::UnsupportedCurve` |
| 14 | `malformed` | `IccError::Malformed` |
| 15 | `oversized` | `IccError::TooLarge` |

The round-two draft listed 13 and omitted codes 9 and 10. `IccStatus` itself
is at `examples/folder-viewer/src/colour.rs:55` to `:64`.

### The existing section 24 handoff, already landed

`packages/core/src/bulk.ts`, read this session. `exportDecodedFrame` at line
401 copies one range out into a fresh ArrayBuffer. `acceptTransferredFrame`
at line 424 is the render-worker side. `contentId` at line 546 documents and
drives the rule that the UID, the transfer syntax UID and the encoded bytes
are hashed through `ContentId::of`, one FNV-1a in the product, and
`transferredFrameMessage` at line 592 builds section 24's frame message and
the transfer list. `crates/ocelli-cache/src/image.rs:40` to `:45` records
that assigning an id to a UID belongs to whoever owns a study, and
`image.rs:72` defines the hash. The decode worker owns the study context, so
the decode worker assigns both identities.

`packages/core/src/frame-cache.ts` exports `setFrameBudget` (line 98),
`frameResident` (107), `showResident` (127, whose `Selection` refusal throws
`CommitRefused` at `:139` to `:142`), `releaseSeries` (149) and
`requestFramePressure` (177). `packages/core/src/commands.ts` exports
`packCommands` (77), `submitCommands` (110) and `sessionCommandSink` (139).
`packages/core/src/pool.ts` exports `openDecodePool` at line 39, F-112's
pool of N identical decode workers per owner. The leaf-module allowlist the
native server keeps already names every leaf the workers import:
`bulk`, `commands`, `frame-cache`, `load` and `panic` are all in
`LIBRARY_FILES` (`examples/folder-viewer/src/serve.rs:770` to `:804`, read
this session).

### The guard that anticipates this story

`examples/viewer-react/src/csp-policy.test.ts:211` to `:259`, read this
session. The test resolves `cargo tree -p ocelli-wasm --target
wasm32-unknown-unknown -e normal --all-features --locked` and refuses any
`ocelli-codec` edge, with the message at line 256 naming F-228's design plan
item 5 and AC9 and demanding the probe be extended before the dependency
lands. F-241 lands before this story and adds the codec edge to the render
module, so its landing is what has to satisfy that demand, and the walk is
red on the tree this story merges onto unless F-241's implementation removed
or replaced it. R1 rules that F-202 neither keeps nor reinstates a
render-module codec refusal. F-202's restructure is what satisfies the
demand durably: the block is removed in the same change and its demand is
met by the D-35 allowlist custody. If F-241's landed tree already replaced
the block, that is a reported contradiction and write-set item 21 becomes
the reconciliation.

### The cornerstone 5.8.2 behaviours the More row names (S8)

Two reference facts, both read in the round-two and round-three sessions from
the pinned 5.8.2 tree the oracle installs. **This session could not re-verify
them**: `tools/oracle/node_modules` is absent in this worktree (`ls`, run this
session), so the pinned dist is not installed. The facts are carried, marked
as carried, and must be re-read at implementation before AC13's row is
written.

The first fact is that cornerstone's DICOM decode pools workers. In the
round-two session, the pinned dist's
`@cornerstonejs/dicom-image-loader/dist/esm/init.js` (`:11` to `:21`)
registered the decode worker with `maxWorkerInstances` set to
`getReasonableWorkerCount()`, and `getReasonableWorkerCount` returned
`Math.max(1, Math.floor(navigator.hardwareConcurrency / 2))`. The pool size
is also `init`'s `maxWebWorkers` override, so an embedding application can
raise or lower it, and the half-concurrency figure is the default it falls
back to. This story runs one decode worker, and AC13 names the gap.

The second fact is the VOI enum order. Cornerstone's `VOILUTFunctionType` is
`LINEAR`, `SAMPLED_SIGMOID`, `LINEAR_EXACT`, with the value string
`"SIGMOID"` in the middle slot, so only the value string matches the tree's
constants.

Cornerstone windows and inverts colour frames per channel after its colour
conversion, which D-29 records as the measured parity choice the tree
follows. The remote route's session applies `ColourWindow::initial()` and has
no colour window command, so the remote surface refuses window controls on a
colour frame instead of applying them. AC13 names that refusal in the row.

## What the specification does not cover

Every decision in this list is made by this plan and attributed here. The
list is not empty and is not allowed to look empty.

1. **The worker scripts themselves.** HLD section 24 gives three message
   lines and three roles. It does not name the script files, their module
   imports, or how the two workers are wired to one another. This plan puts
   them at `examples/folder-viewer/src/render-worker.js` and
   `examples/folder-viewer/src/decode-worker.js`, embedded by `serve.rs` the
   way the pages are (`include_str!` at `serve.rs:44` to `:52`), wired by a
   `MessageChannel` created on the page, because workers cannot address each
   other without one and the page must not relay pixel buffers. Module
   workers receive no import map, so the workers import the leaf library
   modules directly, `bulk.js`, `frame-cache.js`, `commands.js`, `load.js`
   and `panic.js` from `/lib/ocelli-core/`, and never `index.js`, whose bare
   `dcmjs` import is resolved by the page's import map alone (`serve.rs:836`
   to `:847` serves dcmjs for the SR modules of `@ocelli/core`). The
   message vocabulary the two scripts and the page speak is named in Scope
   item 2, and both scripts refuse unknown message fields with the F-230
   vocabulary.
2. **The decode build's export surface.** The decode-worker feature of
   `ocelli-wasm` exists since F-110 and exports only the version and panic
   functions today (`crates/ocelli-wasm/src/lib.rs:70` to `:103`, every
   other module is render-worker only). The HLD names no decode export. This
   plan defines the surface under Scope item 1. It follows section 17.2's
   order and a ticketed slot that mirrors `session.rs`'s rules, reuses
   `exportDecodedFrame` for both copy-outs, the samples and the 76-byte
   record, and makes the three exports whose names collide with
   `session.rs` (`alloc`, `bulk_ticket`, `release_bulk`, exported at
   `session.rs:1380`, `:1385` and `:1417`) conditional on the decode-worker
   build alone, so no feature combination emits two exports of one name.
3. **Where stage 1 and stage 4 run for the remote path.** The render
   session's presentation is fixed at identity modality and there is no
   command that installs a file's Modality LUT into it. The command kinds
   are the eleven of `crates/ocelli-viewport/src/commands.rs:18` to `:20`.
   This plan applies stage 1 and stage 4 decode side, per the transcribed
   F-235 answer, and routes stage 2 and stage 3 through the existing
   commands and presentation state. A file window is sent as
   `KIND_SET_WINDOW`, whose payload carries centre, width and the VOI
   function byte (`commands.rs:507` to `:516`, read this session).
   Inversion is sent as `KIND_SET_INVERT` (parsed at `commands.rs:520` to
   `:522`). A VOI LUT Sequence is refused by name rather than
   approximated, the same refusal the local preview makes, and a Modality
   LUT Sequence is refused the same way, per `preview.rs:1228` to `:1242`.
4. **The frame facts crossing back.** The render worker needs columns, rows,
   the colour route, the applied window, the inversion, the ICC state, the
   cache identity and the parse count. Section 24's frame message carries a
   `meta` field with no layout. This plan defines a fixed 76-byte
   little-endian record, set out under Scope item 1, held in the decode
   route's own buffer and copied out by the landed `exportDecodedFrame`
   (`bulk.ts:401` to `:410`, read this session) called as
   `exportDecodedFrame(wasm, facts_ptr(), 76)`, then read with a `DataView`
   over that copy and posted on as a plain object. `bulk.ts` stays read-only.
   It is data, not a command, and no ArrayBuffer carrying it reaches the page.
   The record is written byte by byte rather than through a `repr(C)`
   struct cast, because the two `u64` fields sit at offsets 52 and 60,
   which are not 8-aligned.
5. **Staging once and parsing once.** Section 24's decode line carries the
   buffer on every request. For one staged instance, re-transferring a body
   up to 32 MiB per frame step is waste, a transferred ArrayBuffer cannot be
   sent twice, and re-running the bounded parse per frame is the same waste
   in wasm. This plan stages the body into the decode instance once per
   displayed instance, runs `consume_borrowed` once at stage time, keeps the
   parsed frame index and attributes, and sends frame indexes afterwards.
   This departs from the transcribed line and is deviation D-34, with the
   saving named and counted by AC24.
6. **The remote display's place on the page.** The HLD names no control
   layout. The remote frame draws into its own canvas element inside its own
   wrapper element in the active cell, a direct child of `.cell`, a sibling
   of `.frame` and `.tiles` (D6), created once and transferred to the render
   worker. The wrapper is cell-sized, opaque and stacked above `.frame`,
   `.tiles`, `.fault` and `.cell-empty`, shown by the activation function
   `showRemoteFrame(vp)` and hidden on its end, and its visibility is owned
   by the remote state alone, never by the local layout's writes to
   `.stage`, so the remote canvas is visible in tile mode, for an empty
   local cell and after a local fault (AC34). The local canvas is never
   transferred, so the page's
   2D context keeps working. `transferControlToOffscreen` is one-time per
   element, which is the reason for the second element rather than reusing
   `.view`. The local-derived overlays the cell template carries
   (`page.html:834` to `:842`: the four `.overlay` corners, `.scale`, the
   four `.edge` markers, and the `.ref-lines` and `.ink` SVGs at
   `page.html:823` to `:824`) are not written while the remote source is
   active, because every local paint entry returns early for a viewport
   whose field is true (D4), and the opaque wrapper covers whatever they
   last drew. F-204's revised plan described the canvas as
   inside the cell's stage (its fact 19, read this session at
   `/private/tmp/ocelli-s47-f204-design/.claude/plans/F-204-design.md`), and
   its revision adopts this placement with the shared field.
7. **The sample bound.** The render session's bulk slot refuses more than
   `MAX_BULK_BYTES`, 64 MiB (`session.rs:23`). A frame whose stored samples
   exceed that cannot commit, so the decode route refuses it by name before
   producing bytes. The local native preview is not bound by that slot, so
   this is a declared limit of the worker route and not a silent difference.
   It is stated in the status, not hidden.
8. **What the remote surface refuses.** Readout, measurement, region window,
   marks, conformance, series window pooling and links stay local-route
   features. With the remote source active they answer with a named
   unavailable line rather than quietly operating on the wrong image. This
   is section 31's rule as generalised by D-07. Window controls on a colour
   remote frame are part of this refusal set, because the session's colour
   window is `ColourWindow::initial()` and is not command driven.
9. **The per-frame attribute resolvers.** `MultiframeMetadata::resolve`
   (`crates/ocelli-dicom/src/multiframe.rs:234`) is the generic lookup, and
   the policy around it, the rescale default and refusal, the position-paired
   window slots, the function default and the Modality LUT Sequence refusal,
   lives today only in `preview.rs` (`:1228` to `:1242`, `:2131` to `:2253`
   and `:2263` to `:2287`), which the decode route cannot reach. Per the
   round-four ruling D4, the whole resolver stack the remote route needs
   moves into one new `ocelli-dicom` module that `preview.rs`, `value.rs` and
   `remote.rs` all call: `group_has`, `resolved_decimals`, `resolved_slots`
   with `WINDOW_EXPLANATION` so `window_rows` keeps working, `decimal_slots`
   and `resolved_text`, plus the policy functions `rescale`, `voi_function`
   and the Modality LUT Sequence refusal. Nothing keeps a local copy. The
   plan moves that stack in Scope item 9, measured there at about 280 written
   lines, inside the story's budget. The alternative, refusing enhanced
   instances by name, is recorded as the rejected alternative with its cost.
10. **The cache budget number.** Section 8 sizes the LRU by the caller and
    names no number. This plan sets 128 MiB, the decoded cap F-245 already
    declared, decided locally this round and recorded under Open questions.
11. **The status sentences after this story.** F-245 fixed the partial
    status. This plan fixes the displayed, pending, refused, idle, auto
    window, ICC notice and trap sentences under Scope item 5. The trap
    sentence says the refetch, because the body is consumed by
    `takeStagedInstance` (`dicomweb-viewer.ts:1312`) and the recovery is a
    refetch, restage and display. No story ID in product copy.
12. **The lazy decode load and its budget.** The HLD names no load time for
    the decode module. This plan loads it on the first remote display, so
    first paint and the render path are unchanged, and records a decode
    budget measured raw and gzip through the new accept path of R3.
    Decided by the operator on 2026-11-12, recorded under Open questions.
13. **ICC on the remote route.** Section 18's stage 4 names the display
    colour pipeline and D-30 records the policy, but no HLD text names where
    the remote route applies a profile. This plan applies it decode side in
    `ocelli-pixel`'s F-240 subset, exactly as the local `ColourRecipe` does,
    with the record carrying the outcome and the page naming a profile
    outside the subset.
14. **Trap recovery for the two workers.** Section 23 says a poisoned
    instance is never reused. Nothing names who respawns a worker, what the
    page says, or where the body went. The body is gone by design
    (`dicomweb-viewer.ts:1312`), so the plan gives both workers the one
    named refetch sentence and a respawn that refetches, restages and
    displays, with an AC each and a real wasm trap behind both.
15. **The test-only armings.** The browser suite needs two things the
    product must never do in production: make the render worker's counted
    adapter request happen once, and trap a real instance. Both ride one
    one-shot query-parameter mechanism the page owns, specified in Scope
    item 5, and both are inert without the parameter and without the
    test-only server setting `OCELLI_TEST_ARMING` of Scope item 3, which is
    off by default. The trap itself reuses the existing `panic-probe`
    feature and separate out-dirs, so no new cargo feature and no
    measured-artifact change.

## Problem and reproducer

Source tree `8949c801`, the head of `sprint/s47`, which integrates F-267
(`git rev-parse HEAD`, run this session). Start the viewer with
`npm run dev` and a configured DICOMweb pair, open the DICOMweb panel, search
to an instance and pick it. The staged body is fetched, validated by the
native route and held by the provider, and the page answers with the sentence
at `page.html:1592`.

> The remote instance is fetched and validated. Remote image display is not
> available yet. The local image is unchanged.

The decode and render machinery this story needs is landed and idle: the
render session, its CPU raster and canvas publish route
(`publishCpuCanvas` at `crates/ocelli-wasm/src/canvas.rs:718`), the section
24 handoff (`bulk.ts`), the frame cache exports (`frame-cache.ts`) and the
command set. What is absent is the decode side. The wasm module decodes
nothing, the tree's own guard says so (`csp-policy.test.ts:211` to `:259`),
and no page worker exists in the showcase. The React demo under
`dev:boundary-demo` writes one stored 1x1 sample and is not acceptance for
this story, per the F-202 allocation note in `docs/sprints/allocation.json`,
read this session.

Expected observation after this story: picking Display on the fetched
instance loads the decode module once, draws the decoded frame through the
worker-owned canvas in the same viewer, window changes redraw without a
re-decode, a return to the local source restores the local image unchanged
with no local label over the remote canvas while it is shown, and a worker
trap recovers by refetching.

## Scope, API and compatibility

### 1. The decode route in `ocelli-wasm`

A new module `crates/ocelli-wasm/src/remote.rs`, compiled under the existing
`decode-worker` feature and testable natively, because the crate compiles on
the host for exactly that purpose (`docs/lld/build-targets.md:494` to
`:495`, read this session: the decode graph reaches `ocelli-pixel`,
`ocelli-cache` and `ocelli-core`). The decode-worker feature gains exactly
two new workspace dependencies, `ocelli-dicom` and `ocelli-codec`, both
optional. `ocelli-pixel` becomes an optional normal dependency enabled by
the feature. That third edge adds no module bytes to either build, because
`ocelli-pixel` is already linked in both graphs through `ocelli-cache`
(`docs/lld/build-targets.md:492` to `:495`), and the dev-dependency entry at
`crates/ocelli-wasm/Cargo.toml:102` stays so the native tests that use it
without the feature still compile. `ocelli-cache` is already a
non-optional dependency (`Cargo.toml:105`) and is not made optional. The
default render-worker build's bytes are unchanged and
`ci/wasm-size-budget.json` is untouched, proved by the wasm gate and by
AC11's byte-identical check.

The route's logic is plain `pub` functions under
`#[cfg(feature = "decode-worker")]`, so native tests and the folder viewer's
differential test reach them without the wasm exports. The exported surface,
all `#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]`, following the
pattern of `ocelli_version` at `lib.rs:70`:

```text
alloc(len) -> u32                  // the decode slot: the staged body only
bulk_ticket() -> u32
release_bulk(ticket) -> u32
stage_body(ticket, len, boundary, study, series, instance) -> Result<u32, String>
release_body() -> u32              // drop the staged body, boundary and parse
decode_staged(frame, study, series, instance) -> Result<u32, String>
sample_ptr() -> u32                // address of the decoded samples
facts_ptr() -> u32                 // address of the 76-byte record
```

`alloc`, `bulk_ticket` and `release_bulk` are conditional on
`#[cfg(all(feature = "decode-worker", not(feature = "render-worker")))]`
so no build emits two exports of one name. The remaining exports are
conditional on the decode-worker feature. `decode_staged` returns the sample
length, and the worker copies the samples out with `exportDecodedFrame`
(`bulk.ts:401`), reusing the landed copy-out rather than returning an owned
`Vec<u8>` through the glue. Refusal strings are the viewer's stable code
vocabulary: `wrong-uid`, `not-part10`, `unsupported-syntax`, `too-large`,
`decode`, `frame`, `voi-lut`, `modality-lut`, `attribute`.

The slot and the buffers are three separate things, and that separation is
the S9 answer.

- The decode slot is the body's. The worker calls `alloc` exactly once per
  displayed instance, writes the body through `holdSessionBytes`
  (`bulk.ts:343` to `:369`), and the slot holds the encoded bytes with
  their ticket until `release_body`. No other allocation touches the slot.
  A second `alloc` while the body is staged returns 0 and keeps the body,
  the rule `session.rs:158` to `:160` states and `remote.rs` mirrors.
- The samples live in `remote.rs`'s own buffer, pre-sized at stage time from
  the parsed `Columns`, `Rows` and the resolved sample kind of the staged
  instance: 4 bytes per pixel for a monochrome frame whose output is one f32
  per sample, and 12 bytes per pixel for a frame whose decoded output is
  interleaved RGB, which is a photometric RGB or YBR space or a palette
  mapped to RGB output. The bound against `MAX_BULK_BYTES` uses the same
  resolved kind, so a 2560 by 3328 monochrome frame pre-sizes 34,078,720
  bytes and is admitted, while the same frame in colour would need
  102,236,160 and is refused `too-large`. The largest admitted monochrome
  frame is 16,777,216 samples, 4096 by 4096. No wasm-side allocation happens
  per frame step after staging: the sample and record buffers are pre-sized
  at stage time and reused. The worker's copy-out is the landed
  `exportDecodedFrame`, which allocates a fresh `ArrayBuffer` per frame by
  design (`bulk.ts:401`, "copies one range out into a fresh ArrayBuffer",
  read this session), and the decode worker keeps one facts object per
  decoded frame for the staged instance's life, so the no-allocation claim
  covers the wasm route's own buffers and nothing else.
- The record lives in `remote.rs`'s own 76-byte buffer. `facts_ptr()`
  returns its address. Because the record and the samples are not slot
  allocations, the decode slot's ticket is never released or reused to read
  them: the staged body's ticket stays current from `stage_body` until
  `release_body`, and the record read calls no export. The worker reads it
  with the landed `exportDecodedFrame(wasm, facts_ptr(), 76)`
  (`bulk.ts:401` to `:410`, read this session), which copies the 76 bytes
  into a fresh buffer the worker parses with a `DataView`. No new reader is
  added to `bulk.ts`, which stays read-only in this story's write set.

`stage_body` is where the parse happens, per R2. It reads the held body,
runs the F-245 bounded parse exactly once (`DicomwebSource::consume_borrowed`
with `SourceResponseKind::WadoRsInstances { boundary }`,
`crates/ocelli-dicom/src/dicomweb.rs:204` to `:243`), refuses a dataset
whose UID triple contradicts the three expected arguments before anything
else (`wrong-uid`), refuses a body that is not Part 10 (`not-part10`) or
over the limits (`too-large`), and keeps the parsed instance: the frame
count, the per-frame attribute table, the part ranges and the transfer
syntax. It pre-sizes the sample and record buffers. A second `stage_body`
for an already staged instance is refused and changes nothing.

`decode_staged(frame, ...)` re-checks the kept UID triple against the three
expected arguments (a request for another instance is `wrong-uid`, before
the codec runs), refuses a frame index outside the kept count (`frame`),
refuses a VOI LUT Sequence (`voi-lut`) and a Modality LUT Sequence
(`modality-lut`) by name through the shared resolvers, decodes the named
frame's part through the codec registry into the pre-sized buffer
(`unsupported-syntax`, `decode`), applies stage 1 through `ocelli-pixel`,
and for colour and palette frames applies stage 4 through
`ocelli-pixel`'s `ColorTransform` with the F-240 ICC subset attached when
the profile is in it. It resolves the frame's window through the shared
resolvers: the frame's own explicit pair and function, or, when the file
names no window, `ocelli_pixel`'s automatic window computed over this
frame's decoded samples with `window_fn` LINEAR_EXACT and `window_source`
1. It then writes the 76-byte record. The record's `parse_count` field is
incremented inside the single parse helper on every bounded parse and is
never assigned: `stage_body` runs the one parse, so the field reads 1, and a
`decode_staged` that parsed again would increment it. A Rust fixture calls
`decode_staged` three times and reads 1 in every record, so the mutation that
moves the parse call into `decode_staged` turns it red, which is what AC24
says: a higher number after a frame step is a re-parse, and a correct
implementation carries 1 in every frame record.

The 76-byte facts record, little-endian, offsets in bytes. It ends at
exactly 76. It is written byte by byte, because the two `u64` fields at
offsets 52 and 60 are not 8-aligned and a `repr(C)` cast would be an
unaligned access.

```text
 0  u32 kind          0 monochrome f32 samples, 1 interleaved RGB,
                        three f32 per pixel
 4  u32 columns
 8  u32 rows
12  u32 frames        NumberOfFrames, at least 1
16  u32 sample_len    returned sample byte length
20  f32 window_center the applied window centre in modality units
24  f32 window_width  the applied window width in modality units
28  u32 window_fn     0 LINEAR, 1 LINEAR_EXACT, 2 SIGMOID
32  u32 invert        1 when the file's presentation inverts,
                        MONOCHROME1
36  u32 window_source 0 explicit file window, 1 the ocelli_pixel
                        automatic window
40  u32 window_index  the selected Window Center and Width pair,
                        0xffff_ffff when automatic
44  u32 icc           0 absent, 1 applied, 2 present and not applied
48  u32 icc_kind      1 to 15 when icc is 2, in read_profile's order,
                        0 otherwise
52  u64 content       ContentId::of over the SOP Instance UID, the
                        transfer syntax UID and the encoded frame bytes
60  u64 series        the decode worker's series identity, derived
                        from the Series Instance UID it parsed
68  u32 parse_count   bounded parses run for this staged instance,
                        1 from the stage-time parse; a higher number
                        after a frame step is a re-parse
72  u32 reserved      zero
```

The window fields carry the frame's own window: its position-paired slot
and function, or the automatic window. Because the record is written per
decode, per-frame windows ride the record and the page re-sends the frame's
window from the record on each step, so no separate re-request exists and
R8's requirement is met by the record. The `window_fn` codes are the tree's
boundary constants, and the parity claim against cornerstone is by the
value string, not the index, for the reason transcribed above.

### 2. The two worker scripts

`render-worker.js` imports the leaf library modules `/lib/ocelli-core/bulk.js`,
`frame-cache.js`, `commands.js`, `load.js` and `panic.js`, never `index.js`,
and the render glue from `/lib/ocelli-wasm/ocelli_wasm.js`. It loads the core
with `loadCore` (`packages/core/src/load.ts:19`), binds the transferred
canvas with `bindCpuCanvas` (`canvas.rs:666`), accepts section 24 frames
with `acceptTransferredFrame`, configures the remote stack with `writeStack`
from the decode record's series and the frame count, drives the F-231 cache
with `setFrameBudget`, `frameResident`, `showResident` and `releaseSeries`,
applies the page's window and inversion batches through `packCommands` and
`sessionCommandSink` in one batch per animation frame, publishes with
`publishCpuCanvas`, `publishCpuColor(1)` and `publishCpuInteraction` only
after the page's settle batch for the shown frame, counts
`navigator.gpu.requestAdapter` calls and WebGL2 context attempts over its
lifetime through one named counted wrapper each, reports both counters in
its status and on a named `self.__ocelliWorkerCounters()` probe, and drains
its ring once per applied batch and on a bounded timer fallback, surfacing
`dropped`. `decode-worker.js` imports `bulk.js` and `panic.js` and the
decode glue from `/lib/ocelli-wasm-decode/ocelli_wasm.js`, holds the staged
body and boundary through the decode slot, runs the stage-time parse, and
answers decode requests with the section 24 frame message on the
MessageChannel port plus the parsed facts object. It also keeps the facts
object of every decoded frame, keyed by frame index, for the life of the
staged instance, and answers a `facts` request for an index from that kept
table without decoding, which is how a resident hit publishes its stored
record (D5, Scope item 7). Both scripts wrap every
export call so a wasm trap is caught, read the panic record with
`readPanicRecord` and no further export call, report the trap to the page
with the record's code, and refuse unknown message fields with the F-230
vocabulary. The message set is: `stage`, `decode`, `facts`, `frame`,
`show`, `settle`, `status` and `trap`. The facts object is data and no
ArrayBuffer rides any worker-to-page message.

The render worker never calls `resolvedTier`
(`canvas.rs:634` to `:662`), which resolves with `TierRequest::Auto` and can
request an adapter. AC14 observes the counters directly in the worker.

### 3. Serving

`serve.rs` gains fixed GET routes with an allowlist and the missing-file
behaviour of the existing library route (`serve.rs:836` to `:862`, whose
`X-Ocelli-Error: library` answer is stated at `:842`): the two
worker scripts, and the two wasm builds read from
`crates/ocelli-wasm/pkg` and `pkg-decode` (`ocelli_wasm.js` and
`ocelli_wasm_bg.wasm` each, `application/wasm` content type). A missing
build answers 404 with `X-Ocelli-Error: wasm`, and the page reports the
core as not built. The launcher is unchanged, matching the existing library
build prerequisite pattern.

The test-only server setting is the environment variable
`OCELLI_TEST_ARMING`, read once by `serve.rs` at startup into a lazy
static, off by default. When it is unset, nothing test-only is served or
armed: the page is served without the arming bootstrap, so the page's flag
reader never sees `window.__ocelliTestArming` and the four query flags are
inert on every code path, and the two trap-build routes answer 404 with
`X-Ocelli-Error: test-arming`. When it is set, `serve.rs` serves
`/lib/ocelli-wasm-decode/` from `crates/ocelli-wasm/target/decode-trap-probe`
and `/lib/ocelli-wasm/` from `crates/ocelli-wasm/target/render-trap-probe`
at the same URLs the workers already import, so the worker's own instance is
the panic-probe build, and injects a one-line `window.__ocelliTestArming`
bootstrap into the served page. The armed response's policy is
`configured_main_policy` computed over the armed page text, the bytes with
the injected line, so the inline hashes cover the bootstrap
(`csp.rs:35` builds the policy from whatever page text it is handed, and
`serve.rs:298` to `:299` builds the unarmed policy from the `PAGE`
constant). The unarmed page's bytes and its policy are unchanged. The trap
route needs no separate URL and no
worker change, which is what makes AC25 trap the very instance holding the
staged body. The out-dirs are gitignored (`crates/ocelli-wasm/target/`,
`.gitignore:30`, read this session) and no size gate reads them.

### 4. The worker policy

`csp.rs` gains `worker_policy()`, served on the two worker responses only.

```text
default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'
```

`'wasm-unsafe-eval'` is needed by the worker response alone, per F-228's
measured evidence in `examples/viewer-react/csp-probe.mjs` that the worker's
wasm compile obeys the worker response's policy, not the page's. The main
page policy keeps no `'wasm-unsafe-eval'` and the page keeps no `worker-src`
beyond `script-src`'s fallback. The page's own policy and its hashes are
unchanged.

### 5. The page

The remote panel gains Display on a fetched and staged instance. The first
Display creates the two workers, the MessageChannel and the remote canvas in
its own wrapper element in the active cell, transfers the canvas to the
render worker, posts the staged body and its boundary to the decode worker
once, and decodes frame 0. The decode module is fetched only then, never at
page boot.

**The wrapper's place (D6).** The wrapper is a direct child of `.cell`
(`page.html:819`), a sibling of `.frame` (`:820`) and `.tiles` (`:827`),
cell-sized, opaque, and stacked above `.frame`, `.tiles`, `.fault` (`:843`)
and `.cell-empty` (`:833`), all read this session. It is never inside
`.stage`, which the local layout hides and shows. The local canvas is never
transferred, so the page's 2D context keeps working.
`transferControlToOffscreen` is one-time per element, which is the reason
for the second element rather than reusing `.view`.

**The activation function (D10).** The page function that shows the wrapper
in the active cell and sets the field is `showRemoteFrame(vp)`, named here
because F-204's decision 15 and this plan both reference "the remote
activation site". Beside the canvas show it calls `fitView(vp)`
(`page.html:3865`, read this session), the page's own fit entry the Fit
button (`:1639`) and the `0` key (`:6059` to `:6061`) share, so a held
one-to-one mode drops to fit through the page's real fit path rather than
through record writes alone. This supersedes F-204 decision 15's
record-writes-only sentence for the drop.

**The routing table (D5).** While `vp.remoteDisplayed` is true, each control
either dispatches to the remote path or refuses with one named sentence
written to `#remote-state` (`page.html:753`) by `remoteState` (`:1493`),
with the DICOMweb panel `section#remote` (`:751`) forced visible for as long
as the field is true. The entries, with the page lines read this session:

| Control | Page entry | While the field is true |
|---------|-----------|-------------------------|
| Scrubber | `#scrub` (`:867`), input at `:1753` | Dispatches to the one routing path of Scope item 7 |
| Plain wheel | `onWheel` (`:4243`), steps at `:4283` and `:4294` | Dispatches to the one routing path |
| Ctrl plus wheel | `onWheel` zoom arm (`:4265` to `:4271`) | Dispatches `KIND_ZOOM` in the per-frame batch |
| Arrow keys | `onKey` (`:6036` to `:6041`) | Dispatch to the one routing path |
| Cine, the play button and Space | `:1750` to `togglePlay` (`:6103`), Space at `:6042` | Refuse: "Cine play is not available on the remote frame." |
| Fit button and `0` key | `:1639` and `:6059` to `:6061`, both `fitView` | Dispatch `KIND_FIT` in the batch |
| One-to-one control and its `\` key | F-204's `oneToOneView` | Refuse: "1:1 is not available on the remote frame." |
| W/L drag | `tool-wl` (`:765`), the pointer drag from `onPointerDown` (`:4361`) | Dispatches `KIND_SET_WINDOW` in the batch |
| Presets | `chooseWindow` (`:4044`), rows at `:4035` to `:4037` | Dispatch `KIND_APPLY_PRESET` in the batch |
| Invert | `#invert` (`:1641`) | Dispatches `KIND_SET_INVERT` in the batch |
| Reset | `#reset` (`:1638`) to `resetView` (`:3871`) | Dispatches the frame's own window from the record as `KIND_SET_WINDOW` |
| Zoom | `tool-zoom` (`:764`), `= + - _` at `:6050` to `:6055` | Dispatches `KIND_ZOOM` in the batch |
| Pan | `tool-pan` (`:763`) | Dispatches `KIND_PAN` in the batch |
| Rotate | `#rotate` (`:1647`), key `o` at `:6075` | Dispatches `KIND_ROTATE` in the batch |
| Flip | `#flip-h` and `#flip-v` (`:781` to `:782`), handlers beside `:1654` | Dispatch `KIND_FLIP_HORIZONTAL` or `KIND_FLIP_VERTICAL` in the batch |

Every dispatch is one packed command buffer per animation frame through
`packCommands` and `sessionCommandSink`, or the one routing path for frame
steps. `commitLabels` (`:2623`) and `showIndex` (`:2639`) are never called
for the remote frame: the remote path writes the scrubber's extent and value
from the record's frame count directly, and release restores them through
the repaint below. The remote route creates no series entry: `seriesOf(vp)`
is never handed a remote frame, and no `seriesList` element is added or
removed by a display. The Scope item 8 refusal set, readout, measurement,
region window, marks, conformance, series window pooling, links and
annotations, stands beside this table unchanged. The two refusal sentences
above join the status sentence set below and ride `#remote-state` like the
others.

**The field and the release.** The remote-state field, per D6 and S8, is one
named page field: `vp.remoteDisplayed`, a boolean on the viewport record,
the same field F-204's revised plan names and reads (its D2: "The page field
the guard reads is `vp.remoteDisplayed`", read this session).
`blankViewport` (`page.html:1362`) carries `remoteDisplayed: false`. It is
set true at the activation function, and **only `remoteRelease` clears it**
(D4). `remoteRelease` (`:1467`) is extended: when a viewport holds the
field, it hides the wrapper, clears the field, calls `layoutViewport(vp)`
(`:3589`) and repaints the painters, `commitLabels`, `paintOverlays`
(`:3647`), `paintEdges` (`:3748`) and `renderInk` (`:5129`). When no
viewport holds the field, it is today's function and its two existing
callers (`:1407`, `:1428`) are unchanged in effect. The "Show local" release
trigger, a button in the DICOMweb panel beside the Display action, calls
`remoteRelease` and keeps the local series: it re-opens nothing, reloads
nothing and re-runs no `showIndex`, because the local canvas still holds its
last frame and the release uncovers it.

Every local paint entry returns early for a viewport whose field is true
(D4): `applyGrey` (`:2776`, whose body covers `drawGrey` and the label and
painter writes), `scheduleLive` (`:4298`), `paintTiles` (`:3492`) and
F-205's hold-release repaint. The early return is at the entry, before any
write, so no local-derived label can be produced while the remote frame
shows, whatever the corners, cross and magnifier toggles say. When the local
route resumes, the release has already cleared the field, so the entries run
as today and no painter body is duplicated. The corners-from-record option
is under Open questions.

Status
sentences, replacing the F-245 partial sentence at `page.html:1592`:

`The remote instance is displayed through the worker. The local image is unchanged.` while drawn,
`Displaying the remote instance.` while pending,
`The remote frame was refused: {code}.` on a refusal,
`The automatic window was applied.` appended when `window_source` is 1,
`This image's ICC profile was not applied.` with the kind appended when `icc` is 2,
`The viewer worker stopped. Display again to refetch the instance and restart.` on a worker trap,
and after release the panel returns to F-245's idle line.

The trap sentence names the refetch because the recovery is a refetch: the
body left the page at `takeStagedInstance` (`dicomweb-viewer.ts:1312`), so
the next Display refetches the instance from the provider, restages it and
displays. On any worker trap the next Display respawns both workers and
rebinds the canvas.

The painters whose writes the early returns cover, with their measured lines
on this base: `paintOverlays`
(`page.html:3647`), which writes the four corners, including F-267's
`showCorners` gate at `:3650` to `:3655` and the toggle handler at `:1671`
to `:1672`, and ending in `paintScale(vp)` at `:3662`. `paintScale`
(`page.html:3728`), whose show condition is at `:3732`. `paintEdges`
(`page.html:3748`), whose four writes are at `:3752` to `:3755`.
`renderInk` (`page.html:5129`) and `renderInkAll` (`page.html:5142`), the
`.ink` SVG. `drawLine` (`page.html:6969`) and `clearLine` (`page.html:6983`,
the `style.display` writer), the `.ref-lines` SVG. `pointReadout`
(`page.html:4700`), which writes the cross `#cross-h`, `#cross-v` and
`#cross-read` (`page.html:855` to `:857`) at `:4721` to `:4736` and the
magnifier `#mag` (`page.html:858`) at `:4705` and `:4738`, with `hideCross`
at `:4856`.

The test-only arming, per R5 and R6 and the AC14 positive control, is one
mechanism: the page reads four one-shot flags from its own query string at
load, `ocelli-test-wrong-uid`, `ocelli-test-trap-decode`, `ocelli-test-trap-render`
and `ocelli-test-probe-adapter`. The reader runs only when the serving
process set `window.__ocelliTestArming`, which only `OCELLI_TEST_ARMING`
(Scope item 3) injects, so the flags are inert in every production load
whatever the query string carries. Each flag is consumed the first time it
is used. `ocelli-test-wrong-uid` replaces the
instance UID in the expected arguments the page builds for the decode
worker's stage message, so the mismatch is injected in the page-to-decode
message and the native validation of the payload is untouched. The payload
itself stays valid, so the provider's receipt and the native validation both
pass and the decode route's own check is what refuses. Under arming the
served decode and render modules are themselves the trap builds, so the
worker calls `panic_probe_trigger()` on its own instance, the one holding
the staged body for decode, at the arming point, after staging for decode
and after bind for render. `ocelli-test-probe-adapter` makes the render
worker run its counted adapter wrapper once at startup. Without the server
setting and a flag nothing of this exists on any path.

### 6. The frame cache

The render worker sets one budget at start, 128 MiB, commits through the
cached-frame route with the decode record's content id, answers residency
before asking the decode worker, and releases the remote series when the
remote source is left.

### 7. The one routing path

Every displayed frame, initial and stepped, goes through one sequence owned
by the page: scroll to the frame, `frameResident` answered first, a decode
request on a miss, `commit_cached_frame` at the selection, then the settle
batch and publish. On a resident hit the decode worker answers the frame's
`facts` request from its kept per-frame table without decoding, so every
frame step that changes the shown frame produces a facts record, fresh on a
miss and stored on a hit (D5). The commit requires the viewport's selection to be at
the requested index (`commit_selected_frame`, `session.rs:227` to `:244`,
the selection refusal at `:238` to `:241`, and the same rule in
`show_resident` at `session.rs:472` to `:474`, read this session), and the
selection moves by steps (`scroll_by`, applied at `commands.rs:427` to
`:429`), so the page batches the steps, at most `MAX_COMMANDS`, 64 per
batch (`commands.rs:28`), across animation frames. The render worker applies
the batch before any commit for that frame, so the selection is at the index
the commit names. On a cache miss the decoded frame is committed when it
arrives, the facts are relayed to the page, and the frame is published only
after the page's settle batch carries the frame's window from the record,
so no frame is ever published unwindowed. A refused selection keeps the
drawn frame.

### 8. The guard restructure (R1, D10, D8)

`decode_graph_check.py` is changed once, in this story, on the tree that
already carries F-241's landing:

- `check_features`' decode-worker refusal at `:68` to `:69`
  ("decode-worker must not enable a dependency") is replaced by the named
  trio: the decode-worker feature enables exactly `dep:ocelli-dicom`,
  `dep:ocelli-codec` and `dep:ocelli-pixel`. The render-worker exact-set
  pin at `:74` to `:77` is whatever F-241's landing left it as, its edge
  admitted. This restructure replaces that pin with the recorded render
  allowlist.
- The resolved decode graph (`cargo tree ... --no-default-features
  --features decode-worker`) is held to the recorded decode allowlist of
  package names under Dependencies, so any crate outside the list turns the
  guard red. The explicit `wgpu|naga|ocelli-render` fast-path refusal of
  `classify_graph` (`PACKAGE` at `:32`, `classify_graph` at `:220` to
  `:234`) stays exactly as it is, on top of the allowlist.
- The resolved render graph (default features) is held to the recorded
  render allowlist the same way, computed on the post-F-241 tree, so
  F-241's codec edge is already in the list rather than added by it.
- `scripts/guards/catalogue.py`'s `decode-graph` entry (`:10157` onward)
  gains one probe per graph and keeps the fast-path probe it has:
  `decode-worker-dep` (`:10172`) keeps feeding `check_features` an
  unlisted dependency and expecting the refusal, a new
  `decode-graph.unlisted-decode` probe feeds the decode allowlist check a
  resolved-graph line naming an unlisted crate and expects the refusal, a
  new `decode-graph.unlisted-render` probe does the same against the render
  allowlist, and `still-links` (`:10255`) keeps feeding `classify_graph`
  its literal `wgpu` line and expecting the fast-path refusal, so no
  existing probe is repurposed. The entry's `refuses=` text is rewritten to
  name the two per-module allowlists. The runbook row is regenerated with
  `python3 scripts/guard_census.py --render-runbook`, and
  `ci/guard-probe-budget.json` is re-recorded with `--record`, because the
  restructure moves the guard's constants and its refusal sites
  (D8).

Both allowlists are written down by name under Dependencies, the decode list
re-verified with `cargo tree` on `8949c801` this session and the render list
stated as the recorded pre-F-241 names plus the delta F-241's edge resolves,
and AC27 drives all four probes and the record.

### 9. The shared per-frame resolvers (R8, D4)

A new module `crates/ocelli-dicom/src/frame_presentation.rs` beside
`MultiframeMetadata` holds the per-frame resolution and policy that today
lives only in `preview.rs`, as `pub` functions over `&MultiframeMetadata`:

- `group_has(multi, frame, group, attribute) -> Result<bool, ...>`: the
  presence check, the body of `preview.rs:2131` to `:2141`.
- `resolved_slots(multi, frame, group, attribute) -> Result<Vec<String>, ...>`:
  the string-slot resolution with empty slots kept, the body of
  `preview.rs:2184` to `:2203`. `window_rows` reads
  `WINDOW_EXPLANATION` through it (`preview.rs:3854`), so it stays generic
  over the attribute and `window_rows` keeps working unchanged.
- `resolved_decimals(multi, frame, group, attribute) -> Result<Vec<f32>, ...>`:
  the body of `preview.rs:2143` to `:2176`.
- `decimal_slots(...) -> Result<Vec<Option<f32>>, ...>`: the body of
  `preview.rs:2205` to `:2227`.
- `resolved_text(...) -> Result<Option<String>, ...>`: the body of
  `preview.rs:2229` to `:2253`. `value.rs:190` reads Rescale Type through
  it, so `value.rs` calls the crate too.
- `frame_rescale(multi, frame) -> Result<(f32, f32), FramePresentationError>`:
  the policy of `rescale` (`preview.rs:2263` to `:2277`), the frame's own
  Pixel Value Transformation Sequence with top-level fallback, default 1.0
  and 0.0, non-finite refused.
- `frame_window_slots(multi, frame) -> Result<(Vec<Option<f32>>, Vec<Option<f32>>), ...>`:
  the position-paired centres and widths under the Frame VOI LUT Sequence
  with top-level fallback, the behaviour of `preview.rs:1287` to `:1291`.
- `frame_voi_function(multi, frame) -> Result<VoiFunction, ...>`: default
  `LINEAR`, unknown refused, `preview.rs:2279` to `:2287`. `VoiFunction` is
  `ocelli-pixel`'s (`crates/ocelli-pixel/src/lut.rs:305`) and
  `ocelli-dicom` already depends on `ocelli-pixel`
  (`crates/ocelli-dicom/Cargo.toml:29`).
- `frame_has_modality_lut_sequence(multi, frame) -> Result<bool, ...>`: the
  refusal `preview.rs:1228` to `:1242` states.
- `first_usable_window_index(&centers, &widths) -> usize`: the first slot
  holding both, `preview.rs:1681` to `:1693`.

Every call site, by grep on the base this session:

- `preview.rs` defines `group_has` at `:2131` and calls it at `:1191`
  (the recipe's `voi_lut` evidence) and `:1233` (inside
  `modality_blocked`, `:1228` to `:1242`).
- `preview.rs` defines `resolved_decimals` at `:2143` and calls it at
  `:2264` and `:2265`.
- `preview.rs` defines `resolved_slots` at `:2184` and calls it at
  `:2214`, `:3851`, `:3852` and `:3854`.
- `preview.rs` defines `decimal_slots` at `:2207` and calls it at `:1287`
  and `:1291`, and defines `resolved_text` at `:2229` and calls it at
  `:2280`.
- `value.rs` imports `resolved_text` at `:25` and calls it at `:190`.
- `remote.rs` calls the crate functions.

After the move, `preview.rs` deletes its private versions and maps the
crate error to `PreviewError` at its call sites, and `value.rs` imports the
crate function in place of the preview-local one. The measured cost, from
this session's reads: 163 lines leave `preview.rs` (`:1228` to `:1242`,
`:2131` to `:2253` and `:2263` to `:2287`), the new module carries them plus
its own module documentation, a `FramePresentationError` and re-exports,
about 190 lines, and about 90 lines of new crate tests beside the existing
synthetic builders. About 280 written lines, under half a week of the
6-week estimate. The no-behaviour-change guard is the local route's own
existing suites, named: `examples/folder-viewer/tests/preview.rs` and
`examples/folder-viewer/tests/value.rs`, including
`value_uses_the_per_frame_rescale` at `value.rs:742` with its frame 1
modality comment at `:744`, which must pass unchanged after the move. The
rejected alternative and its cost are under Approach.

**The crate's existing per-frame resolver, weighed (S1).** The crate already
has a per-frame resolver set: `ocelli_dicom::open_frame`
(`crates/ocelli-dicom/src/frame_open.rs:365`, read this session) resolves
`modality_of` (`:702`), `window_of` (`:740` to `:800`) and `voi_function`
(`:802` to `:818`) over its own private `resolved` (`:636`),
`optional_decimals` (`:1087`) and `has_sequence` (`:1030`), and
`series_open.rs` carries a third private set beside them, its
`resolved_text` at about `:1103`. The recommendation is that the extraction
does NOT reuse them as the one copy, and the two stacks are made to agree by
a test instead. The reason is policy, not arithmetic: `open_frame` supports
a Modality LUT Sequence (`modality_of` builds the transform from it) and a
VOI LUT Sequence (`window_of` builds `VoiTransform::new(Some(lut), ..)`),
while the remote route refuses both by name, the same refusal the local
preview makes. `window_of` returns a constructed `VoiTransform`, not the
raw centre, width, function and index the facts record and the page's
command need, and it selects index 0 rather than the first slot holding
both a centre and a width, and its `decimal_list` refuses a partly empty
multi-value where the position-paired rule of PS3.3 C.11.2.1.2.2 keeps
empty slots. Making `open_frame`'s stack the one copy would change a
shipped crate function's behaviour, which is its own reviewed change and
not this refactor. The measured cost of reuse is the `window_of` and
`modality_of` rewrite over shared helpers, their empty-slot and index
policy change, and the `frame_open.rs` test updates, about 120 to 150
touched lines. The recommended arm costs one agreement test: a fixture in
`crates/ocelli-dicom/tests/frame_presentation.rs`, about 60 to 90 written
lines beside the existing synthetic builders, asserting on a complete
per-frame group, no sequences and no empty slots, that `open_frame`'s
resolved pair, function and rescale, read through `ModalityTransform`'s and
`VoiTransform::window()`'s accessors, equal `frame_rescale`,
`frame_window_slots` and `frame_voi_function`'s, so drift between the two
readings turns red. The legitimately different cases, the sequence support
and the empty-slot and index policy, are named in the new module's
documentation.

### Explicit non-goals

- No native remote renderer. The F-245 refusal stands. The native backend
  validates staged bodies and nothing more.
- No new `@ocelli/core` public export. The worker protocol is the page's and
  the scripts'. A wrapper waits for a second caller, per the structural
  rule against one-implementer constructs.
- No GPU route for the remote frame in this story. The remote surface draws
  through `CpuRaster` on every tier. The GPU stack pass and F-235's shader
  stage 1 stay where they are and are not silently bypassed.
- No readout, measurement, region window, marks, conformance, series window
  pooling, links or annotations on the remote frame. Named unavailable while
  the remote source is active.
- No remote series prefetch, no MPR, no cine, no WADO-URI staged route. The
  provider stages `wado-rs-instances` and this story consumes exactly that.
- No SharedArrayBuffer, no second bindgen crate, no pixel getter, no
  tolerance change.
- No overlay content over the remote canvas in this story. The corners
  stay hidden, and filling them from the record is the option under Open
  questions.
- **The React demo is unchanged where its behaviour is concerned.** The
  demo's page, its `core-worker.ts` and `bin/example-dev.sh` route are not
  touched and still decode nothing. Two files under
  `examples/viewer-react/` do change, because they are the F-228 guard and
  probe that the landed test message itself demands when a dependency
  lands: `src/csp-policy.test.ts` and `csp-probe.mjs`. What replaces the
  codec refusal is under AC10.

### Compatibility

All existing local routes, suites and status texts are untouched. The
page's inline script changes require re-hashing, which `csp.rs` derives
automatically. The browser suite that reads the remote status
(`dicomweb.mjs`, lines 514 to 515 and 573) is updated by this story, and
that update is part of AC9's changed-expectation review rather than a
silent edit. The decode budget is re-baselined with a recorded cause, raw
and gzip, once, by this story. `Cargo.lock` gains the two new decode-feature
edges and the ocelli-pixel option edge. `ci/wasm-decode-size.json` is
re-baselined here through the new accept path. F-241 lands before this
story and its AC21 already requires its own change to leave the file
unedited on its base, which the order records under Dependencies.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Remote panel Display action, `page.html` | `DicomwebProvider.takeStagedInstance`, then the decode worker's stage and decode exports | One staged body, one live display intent, generation guards, lazy first load | The staged body leaves the page exactly once, the decode module is fetched once |
| Decode worker, `decode-worker.js` | The `remote.rs` exports on the decode build | Held body and boundary, one parse at stage time, UID triple check, bounded caps, refusal codes | Section 24 frame buffer on the port plus the parsed facts object, `parse_count` in it |
| Render worker, `render-worker.js` | `loadCore`, `acceptTransferredFrame`, `writeStack`, `frame-cache.ts`, `commands.ts`, canvas publish exports | One OffscreenCanvas owned for the page lifetime, cache budget set once, counted adapter wrapper, counters reported | Canvas blits inside the worker, cache and ring status out, no adapter requested |
| Page window, invert, preset, fit, zoom, pan, rotate, flip on the remote source | `packCommands` batched per animation frame, `sessionCommandSink` | One owner for the latest remote choice, late requests held | Redraw without decode, the F-280 guard shape from `CURRENT_SPRINT.md` lines 62 to 66 |
| Page scrubber on a multiframe remote instance | The one routing path of Scope item 7 | `frameResident` answered first, scroll steps batched, settle before publish | Stepped frames, decode only on a miss, commit at the selection, the frame's own window from the record |
| Shared per-frame resolvers | `ocelli-dicom/src/frame_presentation.rs` | `preview.rs`, `value.rs` and `remote.rs`, error mapped by each caller | One resolver, three callers, AC28 holds the routes equal |
| Existing local pipeline, all S46 routes | Unchanged native handlers | Unchanged once the field is false. The local paint entries return early while `vp.remoteDisplayed` is true, and only `remoteRelease` clears the field (D4) | Unchanged pixels, suites and status |
| `examples/viewer-react/csp-policy.test.ts` and `csp-probe.mjs` | The React guard and probe this story updates | The F-228 policy | The codec refusal block removed, the worker decode case observed |
| `scripts/decode_graph_check.py` | The D-35 per-module allowlists | The `wasm` gate | Any crate outside a module's list red |

Ownership recorded once here. Units, the window is in modality units on both
routes. Rounding, the display byte is the raster's `round_ties_even`
(`cpu.rs:1169`), the monochrome ramp byte `round_half_even`
(`crates/ocelli-render/src/colour.rs:481`). Selection, the frame index
belongs to the page and the stack position to the render session.
Conversion, stage 1 and stage 4 run in `ocelli-pixel` on the decode side,
stage 2 and 3 in the session's chain, and the render side's native RGB
transform is the identity. State transitions, the page owns intents and
generations, the workers own their instances.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Remote staged, no display | Display chosen | `takeStagedInstance` once, decode module fetched, body and boundary staged in the decode worker, parsed once, frame 0 decoded through the one routing path, canvas published, status sentence switches | Missing or already taken staged state answers null, the status names it, the local image is unchanged |
| Remote displayed, multiframe | Frame step | Scroll steps batched, `frame_resident` hit answers `show_resident`, miss decodes that frame and commits at the selection, the record carries that frame's window | A wrong epoch or index refuses as `Selection`, the drawn frame is kept |
| Remote displayed | Window, preset, invert, fit, zoom, pan, rotate, flip | One packed command buffer per animation frame, redraw from committed samples | After close the setter writes nothing and publishes nothing. On a colour frame the window controls answer named unavailable |
| Remote displayed, ICC | Profile inside the F-240 subset | Stage 4 applies it decode side, the drawn bytes change accordingly, the record says applied | A profile outside the subset draws device RGB and the page names the kind, the drawn frame is kept |
| Decode refusal, any stage | Unsupported syntax, wrong UID, too large, VOI LUT Sequence, Modality LUT Sequence | Named refusal status, workers stay usable | The last usable remote frame stays drawn, a retry with a valid instance works |
| Decode worker trapped | A real wasm trap under `panic = "abort"` | The worker catches, reads the record with no export call, the page shows the refetch sentence, the trapped instance is never reused | The next Display respawns both workers, refetches the instance, restages and displays |
| Render worker trapped | A real wasm trap under `panic = "abort"` | The same refetch sentence, the local image is unaffected because the local route is native | The next Display respawns both workers and rebinds the remote canvas |
| Local open during remote pending | `readFolder`, folder change or synthetic open | The local open path calls `remoteRelease` first, which cancels the intent through the existing epoch, releases the staged body and drops the decode body, and clears the field if a viewport held it | The local pipeline runs exactly its F-215 reset, the remote never relabels it |
| Remote released | Leaving the remote source, by "Show local", a local open or the panel's own release | `release_series` on the render session, the staged body dropped, `release_body` on the decode slot, the remote wrapper hidden, `vp.remoteDisplayed` cleared by `remoteRelease` alone, `layoutViewport(vp)` run and the painters repainted (D10) | The workers stay alive for the next display, the idle sentence returns, the local series is shown as it was without a re-open |

Invariants that span consumers. The main thread never holds a decoded pixel
byte and no ArrayBuffer rides any worker-to-page message in either
direction. The staged body is transient and is dropped at release and at
replacement. The body is parsed at most once per displayed instance and the
record's `parse_count` says so. One window choice owner per source route,
the local route's owner is unchanged and the remote route's is the page's
remote setter. A refused request never replaces the last usable frame. The
decode worker's UID check is independent of the provider's receipt and of
the native validation, so one wrong layer is caught by the next. Inversion
and the colour window are each resolved in exactly one place, the chain and
the raster respectively.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Staged body ArrayBuffer and boundary | The provider, owned by the page after `takeStagedInstance`, then the decode worker's slot | Provider generation, then the decode slot ticket, never an address | `stage_body` accepted | Release and refetch recover, a second stage of the same body is refused as a duplicate and changes nothing, the body is gone after a trap |
| Parsed instance and attribute table | The decode route, at stage time | The decode slot's generation | The first `decode_staged` after `stage_body` | A re-parse would raise `parse_count` and AC24 fails it |
| Decoded samples ArrayBuffer | `exportDecodedFrame`'s copy out in the decode worker | The section 24 message's buffer, transferred once | `acceptTransferredFrame` commit on the render session | A transfer failure closes the display attempt, a retry re-decodes from the kept parse |
| Cached frame in the render session | `commit_cached_frame` with content, index, epoch | Content id plus stack position and epoch, never an address | The cached commit returning 1 | Eviction and release go through the ring, `show_resident` refuses a stale entry and keeps the drawn frame |
| Remote canvas bitmap | `bindCpuCanvas` on the transferred element | The render worker's canvas slot for the worker's lifetime | `publishCpuCanvas` returning after a successful `putImageData` | A failed blit publishes nothing and the last drawn frame stays, resize keeps a settled frame when the extent is unchanged |
| Overlay visibility and the remote wrapper in the active cell | The activation function `showRemoteFrame(vp)` and `remoteRelease`, through the one field `vp.remoteDisplayed` (D6, D4) | The active viewport record's field and the cell's DOM state; the wrapper is a direct child of `.cell`, cell-sized, opaque, stacked above `.frame`, `.tiles`, `.fault` and `.cell-empty` | `vp.remoteDisplayed` set true at activation and cleared only by `remoteRelease`, which also runs `layoutViewport` and repaints the painters | The local paint entries return early while the field is true, whatever their toggles say, and the release restores the painters' ownership |
| Remote view state on the page | The page's remote setter | One remote generation token | The status sentence and the canvas visibility | A superseded intent discards its result, the local route's state is untouched throughout |
| Both worker instances | The page, created on the first Display, respawned after a trap | A generation counter the page owns | The workers' ready replies | A trapped instance is never asked anything, the next Display rebuilds both and refetches |

Validation preceding an idle or cached return. The decode facts record is
checked for length and kind before any publish. The render session's own
refusals, `Stale`, `Length`, `Viewport`, `Selection`, are surfaced and leave
the drawn frame. The sequence valid draw, invalid request, valid retry is
AC8 and is exercised before and after settlement.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|---------------------|-------------------------------|------------------------|
| AC1 | Display a synthetic staged 16-bit signed MONOCHROME2 instance, 4 by 1, stored values -160, -60, 40, 240, rescale 1/0, window 40/400 LINEAR. At the forced device scale of the harness the four source columns map to four device bands, and the drawn canvas reads 0, 64, 128, 255 at one device column inside each band, the four LINEAR rows of HLD 18.3 through the 1024-wide ramp and `round_half_even` (`colour.rs:446` to `:479`) | New Rust fixture on the decode route, hand-computed and shown working in a comment before the code, citing section 18.2 and PS3.3 C.11, then browser `worker.mjs` reading the real transferred canvas by drawing it into a scratch canvas and calling `getImageData` there, the pattern of `examples/viewer-react/csp-probe.mjs:723` to `:729`. `getContext("2d")` on the transferred canvas itself is unusable and is a failure | A decode route that also applies the file window, with the page still sending it, draws 102, 143, 184, 255 at the same positions, and a JavaScript LUT beside the Rust one leaves the decode route's samples unwindowed, which the Rust fixture's sample assertion rejects | Implementation, then integrated browser |
| AC2 | A 2 by 1 staged instance with rescale slope 2, intercept 0, stored 20 and 40, the same window. Stage 1 decode side maps them to modality 40 and 80, and the canvas reads 128 and 153. Hand-computed: `((80 - 39.5) / 399 + 0.5) * 255 = 153.383`, `((40 - 39.5) / 399 + 0.5) * 255 = 127.819` | Rust fixture plus the browser pixel read through the scratch canvas, citing PS3.3 C.11.1 and C.11.2.1.2 | Skipping the rescale draws 115 and 128, a slope of 0.5 draws 109 and 115, and an inverted slope draws 77 and 51, each distinct from the expected pair at the same positions | Implementation, then integrated browser |
| AC3 | A page-to-decode stage message whose expected instance UID is wrong is refused `wrong-uid` before decode, the status names it, and the previously drawn frame is retained. The mismatch is injected in the expected arguments only, through the page's `ocelli-test-wrong-uid` one-shot flag of Scope item 5 under `OCELLI_TEST_ARMING`, so the served payload stays valid and the provider's receipt and the native validation both pass. The native UID triple check is `instance_reply`'s comparison at `examples/folder-viewer/src/dicomweb.rs:366` to `:374` (S2), and the provider's is `dicomweb-viewer.ts:418` to `:424`. The body also carries an invalid codestream, so an implementation that decoded first would answer `decode` and not `wrong-uid` | Rust refusal test on `stage_body`'s triple check, then a browser case loading the page with the flag. The native validation's own refusal case stays green: `dicomweb.mjs:4` to `:5` and the assertion near `:465` | Weakening the native validation, trusting the provider receipt alone, refusing after decode, or drawing the wrong instance | Implementation, then integrated browser |
| AC4 | On a synthetic 3-frame instance, show frame 0, step to 1, back to 0. The return answers from the cache, the decode spy records no second decode request for frame 0, and the pixels are identical to the first show | Browser `worker.mjs` with a decode counter in the decode worker's messages, plus the Rust residency tests | Re-decoding on every show, or a hit keyed on address or on a stale content id | Implementation, then integrated browser |
| AC5 | A window change on the drawn remote frame redraws to the expected bytes with no decode request and no staged body re-read, and a command batch prepared before a close is not posted after it, observed by the worker's publish count | Browser control that holds the page's per-animation-frame batch across close and flushes it, per the F-280 guard rule in `CURRENT_SPRINT.md` lines 62 to 66 | Routing window changes through re-decode, a JavaScript window pass, or a setter with no owner | Integrated browser |
| AC6 | Local A open, display remote B, open local A again. The local canvas is byte-identical to before the remote display, marks and checks survive, `cache.released` is observed, and the remote surface and every local-derived label are gone from view | Browser `worker.mjs` with a local pixel snapshot taken before and after | A display path that resets local state, or a remote surface that leaks into the local route | Integrated browser |
| AC7 | Switch sources while B's decode is pending. The pending reply is discarded, nothing publishes after the switch, and a subsequent valid display works | Browser case with the decode completion held past the switch | Publishing a late frame, or a cancel that breaks the next intent | Integrated browser |
| AC8 | A staged instance whose transfer syntax is JPEG XL JPEG Recompression, `1.2.840.10008.1.2.4.110`, known to the registry and unregistered (`crates/ocelli-codec/src/registry.rs:37`), refuses `unsupported-syntax` by name, the workers stay usable, and a following valid instance displays. Exercised before and after a settled draw | Rust refusal tests plus the browser refusal and retry case | A refusal that poisons a worker, or a silent fallback draw | Implementation, then integrated browser |
| AC9 | Every existing viewer-browser suite passes with its local expectations unchanged, and the new suite is registered in `bin/ocelli.sh` under `viewer-browser` before `launcher.mjs` (`bin/ocelli.sh:219` to `:237`). The remote-state expectations in `dicomweb.mjs`, which assert the F-245 partial sentence at lines 515 and 573, move to the delivered states as a named edit | `bin/ocelli.sh gate viewer-browser`, reading exit codes from the command | A changed local expectation, or a skipped suite counted as coverage | Integrated browser |
| AC10 | The worker responses carry `script-src 'self' 'wasm-unsafe-eval'` and `connect-src 'self'`, the page policy carries neither `'wasm-unsafe-eval'` nor any weaker source. The React test's codec-refusal tree walk at `csp-policy.test.ts:211` to `:259` is gone by this story's landing: F-241 lands first and its own demand sentence requires the probe to be extended before its dependency, and F-202's write-set item removes or reconciles the block in the same change, with the D-35 allowlists holding the decode feature's exact set. The React probe then decodes one frame in its worker under the probe's own policy with `'wasm-unsafe-eval'` on the worker response: the case's page, worker script, module and serving are specified under Approach, in its own subsection, and `core-worker.ts` is unchanged | `csp-policy.test.ts` run by `npm run test`, which vitest's config includes for `examples/*/src`, and the two probe scripts run explicitly per their own headers with the built modules and recorded in the story evidence | Keeping a render-module codec refusal that F-241's landed edge turns red, widening the page policy, adding `unsafe-eval`, a blob or data worker, or claiming the decode case without running it | Integrated, both probes |
| AC11 | The render budget in `ci/wasm-size-budget.json` is byte-identical to base through the wasm gate, and `ci/wasm-decode-size.json` is re-measured and re-baselined once through `pin_and_size_check.py`'s explicit decode accept path, raw and gzip, with the recorded cause and the rebaselined-history shape the render record keeps, and no tolerance change. Per D9: the gzip figure is measured at compression level 9 and its ceiling is the recorded gzip times `1 + TOLERANCE` (`TOLERANCE = 0.05`, `scripts/pin_and_size_check.py:241`, read this session), the same rule as the raw bytes, so a rebuild whose gzip exceeds that ceiling fails and a smaller one passes. A stale record, the recorded raw `bytes` no longer equal to the built module's size, is refused only at accept time, never by gate wasm: the gate applies the two ceilings and nothing else, and the accept path refuses an accept whose built module still matches the recorded bytes, because there is nothing moved to re-baseline. `--accept-size` is never run | `bin/ocelli.sh wasm`, `bin/ocelli.sh wasm-decode`, `python3 scripts/pin_and_size_check.py --with-size` and the new `--accept-decode-size` path, the unit tests in `scripts/tests/test_pin_and_size_check.py`, and a diff check that the render record is unchanged | A silent budget edit, a widened tolerance, a gzip figure not measured, measured below level 9, a hard no-tolerance gzip ceiling, a stale record that turns gate wasm red or is silently accepted by a no-op accept, a render budget file that changed, or a decode module built with the render feature set | Build |
| AC12 | The page's outbound transfers are exactly, per spawn, one `MessagePort` to the decode worker and one to the render worker, the two ends of the page-created `MessageChannel`, and one `OffscreenCanvas` to the render worker; per stage, one `ArrayBuffer`, the staged body, to the decode worker; per animation-frame batch, one `ArrayBuffer`, the packed command buffer, to the render worker, HLD 24's `main -> render : { kind:'commands', buffer } [transfer]` line (D3). No `ArrayBuffer` travels from either worker to the page, ever: the decode-to-render frame transfer rides the `MessageChannel` port and the facts record travels as a plain object. The suite's init script wraps `Worker.prototype.postMessage` and `MessagePort.prototype.postMessage` (S7) and records every transfer list in both directions | Source check over the two worker scripts and the page, plus the browser suite's wrapped observation over the real workers | A pixel export, a page-relayed frame, a facts record posted as an `ArrayBuffer`, a command batch copied instead of transferred or posted per event instead of one buffer per batch, a second body transfer on a re-stage, a second canvas or port transfer on a rebind, or a `toDataURL` shortcut | Implementation, then integrated browser |
| AC13 | The More matrix row for F-202 at `more.html:183` states Partial and names the gaps against cornerstone3D 5.8.2's DICOM worker decode: cornerstone pools `hardwareConcurrency / 2` decode workers (`dicom-image-loader/dist/esm/init.js:11` to `:21`, carried from the round-two read, not re-verified this session) against one here, one staged instance at a time against streamed instances, no worker-side prefetch, the CPU raster with no GPU path, the resolved-kind sample buffer bound of 64 MiB, no readout, measurement, links or annotations on the remote frame, and the remote route refusing window controls on a colour frame per D-29. The VOI parity claim is by the value string, not the enum index, for the reason transcribed above. The worker decode section at `more.html:560` to `:588` and the Use book and README match, and the fixed status sentences carry no story ID | The More snapshot check and a read of the three documents against the implemented behavior | An overstated matrix row, a row claiming parity with the reference's pooled streaming decode, an index-based VOI claim, or the F-245 partial sentence left standing | Integration and docs |
| AC14 | The remote display opens no device. The render worker counts `navigator.gpu.requestAdapter` calls and WebGL2 context attempts through one named counted wrapper each and no other call site, observed directly in the worker with Playwright's `Worker.evaluate` over `self.__ocelliWorkerCounters()`, not only through the status text: zero during the remote display while the drawn bytes are the expected ones. The positive control arms the worker's counted wrapper once with the `ocelli-test-probe-adapter` flag under the arming setting and the counter reads 1, proving the counting code is live. The control does not need an adapter to resolve | The browser suite over the real workers with `page.waitForEvent("worker")` and `worker.evaluate`, the armed and unarmed cases, and a source check that the worker script names `requestAdapter` only inside the counted wrapper | Self-reported status counts alone, an uncounted `requestAdapter` call site, a page-level spy that cannot see a dedicated worker's navigator, a `resolvedTier` call that quietly requests an adapter (`canvas.rs:634` to `:662`), or a counter whose wiring is never proven | Integrated browser |
| AC15 | Display a synthetic staged 2 by 1 RGB photometric instance with samples 255,0,0 and 0,128,255. The colour route publishes interleaved RGB as three f32 per pixel, 12 bytes per pixel, and the canvas bands read 255,0,0 and 0,128,255 | The Rust colour fixture plus the browser pixel read through `publishCpuColor(1)` and the scratch canvas, citing D-23's recorded reading that a colour frame's samples are its display input | A channel-swapped publish reads 0,0,255 and 255,128,0, and a monochrome publish of interleaved bytes refuses or reads single channels, each distinct at the same positions | Implementation, then integrated browser |
| AC16 | A synthetic staged monochrome instance with no Window Center, no Window Width and no VOI LUT Sequence takes `ocelli_pixel`'s automatic window. The fixture stores 0 to 99, 100 values, rescale 1 and 0. `AutoWindowPool::add_frame` leaves the stored zero out (`is_stored_zero`, `auto_window.rs:273`, read this session), so 99 values are counted. The nearest-rank ranks are `99.div_ceil(100) = 1` and `(99 * 99).div_ceil(100) = 99` (`auto_window.rs:410` to `:411`), so the percentiles are 1 and 99. `margin_pair` (`auto_window.rs:537` to `:550`) moves them 16 f32 steps each, one `next_up` or `next_down` at a time, to 1.0000019073486328 and 98.9998779296875, so the first width tried, `high_moved - low_moved`, rounds to 97.9998779296875 and the centre, `low_moved + width / 2`, rounds half to even to 49.99993896484375; the four-point check passes at that first width and `search_window` returns it. The record carries it with `window_source` 1 and `window_fn` 1, the page's first settle batch applies it before the first `publishCpuCanvas`, and the drawn bytes at stored 0, 16, 49, 82 and 98 are 0, 39, 125, 211 and 253: the displays are 0, 39.031, 124.898, 210.766 and 252.398 through C.11.2.1.3.2, and the 1024-wide ramp's bin lifts 252.398 to 253, with no unwindowed frame drawn. All values recomputed in emulated f32 this session over the transcribed `apply_window` and ramp arithmetic | Rust fixture computing the auto window over the fixture samples, plus the browser read of the first publish | A windowless frame drawn unwindowed first and re-published after the window command, which would read 0, 16, 49, 82, 98 at the same positions, a pool that counts the stored zero and so centres at 48.99993896484375, width 97.9998779296875, drawing 0, 42, 128, 213, 255, a JavaScript percentile beside `ocelli_pixel`'s, or an auto window computed in display space | Implementation, then integrated browser |
| AC17 | A staged instance whose Window Center and Width are multi-valued with an empty first slot applies the first slot holding both a centre and a width, the local route's default (`preview.rs:1681` to `:1693`), and reports it as `window_index` | Rust fixture over the paired values plus the browser status read | Taking the first slot regardless of an empty value, pairing by position after an empty slot, or reporting a pair the draw did not use | Implementation, then integrated browser |
| AC18 | A staged instance whose file window names VOI LUT Function LINEAR_EXACT draws through the session's stage 2 with that function. The fixture inputs are the 18.3 rows plus 140, and the drawn bytes are 0, 64, 128, 255, 191, where the input 140 separates the two linear functions through the 1024-wide ramp: 18.2's LINEAR displays 191.729 and draws 192, LINEAR_EXACT displays 191.250 and draws 191, hand-computed | Rust fixture asserting the display value through the raster's `display_at` route, 127.500 against 127.819 at the centre, plus the browser pixel read, citing C.11.2.1.2 and C.11.2.1.3.2 | Sending the window without the function byte draws the LINEAR bytes, 192 at input 140 instead of 191, and a second application of the function decode side leaves the session's display value at the LINEAR number, which the fixture rejects | Implementation, then integrated browser |
| AC19 | A staged instance whose file window names VOI LUT Function SIGMOID draws through 18.2's SIGMOID formula at the 18.3 inputs. SIGMOID is not the ramp: `preset_index` (`colour.rs:424` to `:430`) sends it to `display_index` (`colour.rs:518`), and the bytes are 30, 69, 128, 225, hand-computed through `display_index`'s largest-code-not-past-half rule | Rust fixture plus the browser pixel read, citing C.11.2.1.3.1 | A sampled table beside the formula, the LINEAR bytes 0, 64, 128, 255, the LINEAR_EXACT bytes, or a ramp-read SIGMOID, each distinct at the same inputs | Implementation, then integrated browser |
| AC20 | A staged MONOCHROME1 instance with no Presentation LUT Sequence inverts. The record's `invert` is 1, the page's first settle batch carries `KIND_SET_INVERT(true)` before the first publish, and the drawn bytes are 255, 191, 127 and 0 at AC1's inputs, hand-computed as the inverse of the LINEAR displays through `ymin + ymax - display` (`lut.rs:572`) drawn by the 1024-wide `reference_ramp_index` with `round_half_even` (`colour.rs:448`, `:481`): 255 - 63.910 = 191.090 to 191, 255 - 127.819 = 127.181 to 127. The user's invert toggle replaces the explicit shape, exactly as `set_invert` resolves it (`presentation.rs:148` to `:168`), so toggle-off draws 0, 64, 128, 255 and toggle-on draws 255, 191, 127, 0, one inverse in the chain each time (`lut.rs:526` to `:533` and `:596` to `:608`) | Rust fixture over the inversion plus the browser reads for the initial draw and both toggle states, citing C.11.6 and C.7.6.3.1.2 | A decode-side manual inversion beside the chain's stage, which double-inverts on toggle, an initial draw that is not inverted, or an inversion composed by XOR with the file's photometric | Implementation, then integrated browser |
| AC21 | A staged palette-colour instance with 8-bit indices and a supplied palette maps the STORED index through the palette decode side, per D-23. The fixture indices are 2 and 5 with 8-bit entries, entry 2 is (200, 100, 0) and entry 5 is (0, 50, 250) per the descriptor rule of PS3.3 C.7.6.3.1.5, and the canvas reads those entries | Rust fixture with the entries hand-set and shown working plus the browser pixel read | Indexing the palette with a modality value: the fixture carries rescale slope 2, so a modality index would read entry 4 or 10, which are (0, 0, 0), distinct at the same positions, or a second palette application in the render session | Implementation, then integrated browser |
| AC22 | A staged colour frame carrying an ICC Profile inside the F-240 subset has it applied decode side, and the drawn bytes differ from the same frame without the profile by the profile's transform. The fixture is F-240's gamma-1.0 sRGB profile: device samples 5, 64, 128, 192, 255 per channel draw 38, 137, 188, 225, 255, hand-computed in `crates/ocelli-pixel/tests/icc.rs:322` to `:344` from ICC.1:2022 Table 7 and IEC 61966-2-1 | Rust fixture applying `with_icc` over the fixture samples, the same rows `icc.rs` proves, plus the browser read | A remote route that drops the profile quietly, or applies it render side a second time | Implementation, then integrated browser |
| AC23 | A staged colour frame carrying a lookup-table ICC Profile draws in device RGB and the page names the kind, the D-30 sentence, from the record's `icc` 2 and `icc_kind` 1. Two further profiles cover the kinds the round-two draft omitted: one with an unsupported data colour space names `unsupported-colour-space`, `icc_kind` 9, and one with a Lab PCS names `Lab PCS`, `icc_kind` 10, both built with the minimal-profile shape of `crates/ocelli-pixel/tests/icc.rs:134` | Rust fixture asserting the device RGB bytes and the record fields for the three kinds, plus the browser status read | A silent device-RGB draw with no notice, a refusal that leaves the frame undrawn, or a kind list that is not the 15 of `colour.rs:89` to `:109` | Implementation, then integrated browser |
| AC24 | Display a staged 3-frame instance and step to the third frame. The staged body is posted to the decode worker exactly once, `stage_body` runs once and the bounded parse runs once, every frame record's `parse_count` stays 1, and the per-frame messages carry only frame indexes. A re-parsing implementation's records would read 2 and 3 and fail | Browser `worker.mjs` reading the parse count from the facts objects the render worker relays, the wasm-side number, plus the Rust record test | Re-transferring the body per frame request, the section 24 line read literally, which D-34 refuses, or staging once but re-parsing per frame, which R2 refuses | Implementation, then integrated browser |
| AC25 | The decode worker takes a real wasm trap on the very instance holding the staged body: with `OCELLI_TEST_ARMING` set, `serve.rs` serves the decode route from `crates/ocelli-wasm/target/decode-trap-probe` at the URL the worker already imports (Scope item 3), so the worker's own instance is the panic-probe build, and the `ocelli-test-trap-decode` flag makes it call `panic_probe_trigger()` after `stage_body` succeeds, so the body is already staged in that instance and gone from the page (`dicomweb-viewer.ts:1312`). The worker catches the trap, reads the record with no export call and reports it, the page shows the refetch sentence, and the next Display respawns both workers, records exactly one new instance fetch in the network log, restages and displays | Browser case over the real trap module, whose build command `wasm-pack build crates/ocelli-wasm --target web --out-dir target/decode-trap-probe -- --no-default-features --features decode-worker,panic-probe` is documented in the suite header, built before the suites run by the viewer-browser CI job on the panic arm's pattern (`bin/ocelli.sh:161` to `:167`, S2) and on demand otherwise, the way `scripts/panic_probe.mjs:130` to `:134` reports a missing probe module and names its build command, with the suite failing by name when the module cannot be built. The case runs under the arming setting and asserts the unarmed server answers the trap routes 404. The trap is observed as a `WebAssembly.RuntimeError`-shaped error, not a synthetic `Error` | A forced JS worker error standing in for a trap, a trap fired in a second instance beside the one holding the body, a respawn that reuses the poisoned instance, a sentence that hides the trap, or a recovery that does not refetch | Integrated browser |
| AC26 | The render worker takes a real wasm trap the same way, on its own instance: with arming on, the served render module is `crates/ocelli-wasm/target/render-trap-probe`, built with `wasm-pack build crates/ocelli-wasm --target web --out-dir target/render-trap-probe -- --features panic-probe` into the gitignored target tree, and the `ocelli-test-trap-render` flag calls `panic_probe_trigger()` after bind. The page shows the refetch sentence, the local image is unaffected because the local route is native, and the next Display respawns both workers and the remote canvas is rebound. Without the arming setting the trap routes answer 404 and nothing arms | Browser case over the real trap module, same mechanism and same CI build of `render-trap-probe` before the suites (S2) | A trap that takes the local image with it, a canvas rebind against the dead instance, or a synthetic error | Integrated browser |
| AC27 | The per-module allowlists hold. `python3 scripts/decode_graph_check.py` passes with the recorded lists and refuses a manifest whose decode-worker feature enables an unlisted dependency, a resolved decode graph containing any crate outside the 95-name decode list, and a resolved render graph containing any crate outside the render list recorded on the post-F-241 tree, the 76 names plus the delta F-241's edge resolves. The four catalogue probes drive all three refusals red: `decode-worker-dep` for the manifest, the new `unlisted-decode` and `unlisted-render` probes for the two allowlists, and `still-links` for the wgpu and naga fast path it always fed. The `pins` entry's new refusals each carry their probe, the `refuses=` text of both entries matches the refusals, and `ci/guard-probe-budget.json` is re-recorded with `--record` (D8). The runbook row is regenerated, and `bin/ocelli.sh gate wasm` runs the guard | The guard itself, the four probes (`decode-worker-dep`, `unlisted-decode`, `unlisted-render`, `still-links`), the re-recorded census and its unit checks, and the gate, exit codes read from the commands | A guard that names the trio but lets any other crate through in either graph, a probe that no longer matches the refusal, a repurposed `still-links` that no longer proves the fast path, a stale `refuses=` text, or a runbook row that no longer matches the probe | Implementation, then the gate |
| AC28 | The two routes agree on the same quantity. The same synthetic instance is decoded through the decode route and drawn through the local preview route, and the comparison reads the u8 display bytes of both: the local route's `Preview.grey` (`preview.rs:155`) and the decode route's samples committed through a host-side render session and drawn by the same `CpuRaster`, read as u8 at the same positions. The window choice, pair, function and inversion agree through `Preview`'s public fields (`preview.rs:145` struct, `:165` to `:172`) against the record's fields. Per S4, two further fixtures compare the colour routes: one RGB photometric instance and one ICC-profile instance drawn through both routes, so colour and stage 4 drift between `ColourRecipe` and the decode route's transform is red | `examples/folder-viewer/tests/remote_route.rs`, a new differential fixture over both routes, with the folder viewer gaining a dev-dependency on `ocelli-wasm` with `decode-worker`. Measured cost of the two added fixtures, from the existing fixture shape: about 100 written lines, no production move | A recipe drift, where one route applies a stage the other does not, a different auto window, a different pair selection, a colour or ICC transform applied on one route only, or a comparison of f32 samples against u8 bytes, which are not the same quantity | Implementation |
| AC29 | The one routing path holds. Initial display and scrub steps both record scroll, residency answer with the stored record published on a hit (AC33), a decode request on a miss, commit at the selection, settle, publish. A decode commit issued while the selection is elsewhere is refused `Selection` and the drawn frame is kept | Browser `worker.mjs` sequencing assertions plus the Rust selection refusal tests (`session.rs:472` to `:474`) | A second initial-decode route that skips the scroll, a publish before the settle batch, a commit before the batch is applied, or a silent commit at the wrong index | Implementation, then integrated browser |
| AC30 | The decode module is lazy. A fresh page load with a network log records no `/lib/ocelli-wasm-decode` request before the first Display, exactly one at the first Display, and no re-fetch on later displays | Browser `worker.mjs` network assertions | Eager module loading that slows first paint, or a re-fetch per display | Integrated browser |
| AC31 | While the remote frame is drawn, nothing derived from the local image is visible over it, whatever the painters' own toggles do. The case turns the corners toggle on and off, enables the cross with `showCross`, and enables the magnifier, all over the drawn remote frame, then asserts by COMPUTED visibility, `getComputedStyle`, not by the `hidden` attribute alone: the eleven elements of `page.html:823` to `:842`, the four `.overlay` corners, `.scale`, the four `.edge` markers, `.ref-lines` and `.ink`, plus `#cross-h`, `#cross-v`, `#cross-read` (`page.html:855` to `:857`) and `#mag` (`page.html:858`) are `display: none` or not rendered. `.ref-lines` is checked through its `style.display`, the mechanism `clearLine` (`page.html:6983`) writes, and `.ink` through its children, `childElementCount` zero, because an empty SVG has no box of its own. The three cell states are covered too: in tile mode each visible `.tile-label` sits beneath the opaque wrapper, and in the empty-cell and fault cases `.cell-empty` (`:833`) and `.fault` (`:843`) sit beneath it the same way, so none renders over the remote canvas. The paint entries early-return per D4: `applyGrey` (`:2776`), `scheduleLive` (`:4298`), `paintTiles` (`:3492`) and F-205's hold-release repaint, including F-267's corners painter inside `paintOverlays` (`page.html:3650` to `:3655`) with the toggle at `:1671` to `:1672`, `paintScale` (`:3728`), `paintEdges` (`:3748`), `renderInk` (`:5129`), `drawLine` (`:6969`) and `pointReadout`'s cross and magnifier writes (`:4700`, `:4721` to `:4738`), while the remote canvas shows the expected bytes | Browser `worker.mjs` toggling each control, hovering the remote frame with the magnifier armed, walking the active cell's elements and reading each computed display and visibility, the `.ink` child count, the wrapper's opaque box over `.tile-label`, `.fault` and `.cell-empty` in the three states, plus the canvas byte read | Hiding only the corners and leaving `.scale`, an `.edge` marker, the cross or the magnifier visible, checking the `hidden` attribute where the painter writes `style.display`, an `.ink` child drawn after activation, a painter that honours `showCorners` or `showCross` but runs while the field is true, a hover handler that resolves no viewport over the remote canvas and so skips its gate, or labels drawn over the remote canvas | Integrated browser |
| AC32 | A synthetic staged enhanced multiframe instance has per-frame rescale and per-frame windows that differ, with a top-level rescale present that is wrong for both frames: frame 0 stores 1000 with per-frame rescale 1 and -1024 and Frame VOI LUT 40/400 LINEAR, frame 1 stores 1000 with per-frame rescale 2 and 0 and Frame VOI LUT 2000/400 LINEAR. Frame 0 draws 87, hand-computed as modality `1000 * 1 - 1024 = -24` then `((-24 - 39.5) / 399 + 0.5) * 255 = 86.917`. Frame 1 draws 128, hand-computed as modality `1000 * 2 + 0 = 2000` then `((2000 - 1999.5) / 399 + 0.5) * 255 = 127.820`, citing PS3.3 C.7.6.16.2.9, C.7.6.16.2.10 and C.11.2.1.2. The top-level rescale is 3 and 0, so a resolver that reads it draws 255 and 255, and one that drops per-frame windows draws 255 on frame 1 | Rust fixture over the shared resolvers and the decode route, plus the browser read of both frames | A remote route that quietly applies top-level or absent rescale, drops per-frame windows, or refuses the instance by name | Implementation, then integrated browser |
| AC33 | Every frame step that changes the shown frame produces a facts record, including a resident hit (D5). On AC32's instance the case shows frame 0, steps to frame 1, and steps back to 0. The return answers from the cache with no second decode request for frame 0, the decode worker's kept per-frame facts are published as the stored record, the page's settle batch carries frame 0's own window 40 and 400 before the publish, and the drawn byte at AC32's position is 87. An implementation that published the resident hit without a record would leave frame 1's window 2000 and 400 applied, under which frame 0's modality -24 is at or below the lower edge and draws 0 | Browser `worker.mjs` stepping 0, 1, 0 over the AC32 instance, reading the decode counter, the published record and the canvas byte, plus a Rust fixture asserting `decode_staged` writes each frame's own window into its record, which is what the kept table stores | A resident path that publishes with no record and so keeps the previous frame's window, drawing 0 instead of 87, a kept table keyed on anything but the frame index of the staged instance, or a second decode of frame 0 | Implementation, then integrated browser |
| AC34 | The remote canvas is visible whatever the local layout did (D7, D6). The remote canvas lives in its own wrapper element, a direct child of `.cell`, a sibling of `.frame` and `.tiles`, opaque and stacked above `.frame`, `.tiles`, `.fault` and `.cell-empty`, so each of these three cases leaves it with a non-zero visible bounding box: tile mode, an empty local cell with no local frame loaded, and the local route after a fault. The case arms each in turn over a drawn remote frame and asserts `getBoundingClientRect()` has width above 0 and height above 0 and the element visible in all three | Browser `worker.mjs` measuring the wrapper's box in the three cases, plus the canvas byte read each time | A canvas placed inside `.stage`, which the local layout hides in tile mode, for an empty cell or on a fault, and so hides the remote frame with it, a wrapper the local route's writes can remove or paint under, or a wrapper beneath `.fault` or `.cell-empty` | Integrated browser |
| AC35 | The one-to-one seam holds (D10). On a local image with F-204's one-to-one mode held, displaying the remote frame in that cell drops the mode: the activation function `showRemoteFrame(vp)` calls `fitView(vp)` (`page.html:3865`), so the record reads fit with the fit percent label and no `Zoom 1:1` is displayed over the remote canvas. The one-to-one control and its `\` key refuse with the sentence "1:1 is not available on the remote frame." written to `#remote-state` (`:753`), whose panel `section#remote` (`:751`) is forced visible while `vp.remoteDisplayed` is true, and never to the corner line, which the early returns leave unwritten. "Show local" releases: the wrapper is hidden, the field cleared, `layoutViewport(vp)` runs and the painters repaint, and the local series is shown as it was, its selection and images untouched and no re-open issued. After release the `\` key enters the mode again | Browser `worker.mjs`: hold the mode on the local image, display the remote frame, assert the corner carries no `Zoom 1:1` and the label is the fit percent, press `\` and assert the refusal on `#remote-state` with the panel visible and nothing written to the corners, release with "Show local", assert the local image at fit with the fit label and the layout applied, and press `\` again and assert the mode enters. The series structures are compared before and after the release | A held `Zoom 1:1` after display, a refusal reason written to the hidden corner, a release without the `layoutViewport` call, the key refused after release, or a release that re-opens the local series | Integrated browser |

Rows AC1, AC2, AC15, AC18, AC19, AC20, AC21, AC22 and AC32 name their
fixtures as written first. The fixture values are derived from section 18.2's
transcribed formulas and checked against section 18.3's table, the rescale
row follows PS3.3 C.11.1's `Stored * slope + intercept`, the colour rows'
samples are their own expected bytes because a colour frame's decoded
samples are the display input and never enter stages 1 to 3, which is D-23's
recorded reading, and the palette row indexes by the stored value, also
D-23. AC16's window, displays and bytes were recomputed in emulated f32 this
session before they were written, a `node` script with `Math.fround` over
`margin_pair`'s steps, `apply_window`'s LINEAR_EXACT branch and the
`reference_ramp_index` bin, and AC33's 0 is the round-four recompute that
the review confirmed.

## Approach

The smallest implementation in the transcribed source, in build order.

1. **The shared resolvers and the decode route, fixtures first.** Write the
   Rust fixtures for AC1, AC2 and AC15 with the expected bytes derived from
   section 18.2 by hand, watch them fail, then write
   `crates/ocelli-dicom/src/frame_presentation.rs` and
   `crates/ocelli-wasm/src/remote.rs` against `ocelli-dicom`'s
   `consume_borrowed` and `ParsedDicom`
   (`crates/ocelli-dicom/src/parse.rs:154`), the codec registry's `decode`
   into a caller buffer, and `ocelli-pixel`'s modality, colour, auto window
   and ICC transforms. Mutate one constant, confirm red, revert.
2. **The decode build and its guard.** Feature-gated dependencies in
   `crates/ocelli-wasm/Cargo.toml`, exports in `lib.rs`, the D-35
   restructure of `decode_graph_check.py`, the catalogue entry, the
   regenerated runbook, `wasm-decode` rebuilt, `ci/wasm-decode-size.json`
   re-baselined once through the new accept path, raw and gzip.
3. **The render worker and the page.** `render-worker.js` reuses `loadCore`,
   `acceptTransferredFrame`, `writeStack`, the frame-cache exports and the
   canvas publish route exactly as landed. `page.html` adds the Display
   action, the remote wrapper and canvas outside `.stage`, the remote view
   state, `vp.remoteDisplayed` and the painter gates,
   the one routing path and the status sentences. Rejected alternative,
   reusing the React demo's `core-worker.ts`. It is the sample-only proof,
   holds no cache route and no facts record, and its URL constant is a Vite
   define the native path does not have.
4. **Policy, routes, probes, CI, docs.** `csp.rs`, `serve.rs`, the two
   probes, the CI viewer-browser build steps, the three user documents, the
   LLD files, the eslint worker block.
5. **The page's handlers route, not sit beside (D5).** The round-four
   sentence that no existing handler is rewritten is void. The controls of
   Scope item 5's routing table gain the field check at their entry points:
   the ones the table marks dispatch call the remote path, the ones it
   marks refuse write their named sentence to `#remote-state`, and the
   local paint entries early-return. The local route keeps its owner once
   the field is false.

### The eslint worker block (D7, S6)

The three worker files join the single enforcement block, not a new block.
The block at `eslint.config.js:245` to `:264` is the one place
`no-restricted-syntax` and `ocelli/no-wasm-memory-view` are enforced, and
`assertTheBanIsIntact` (`eslint.config.js:150` to `:203`, read this session)
refuses a config with any other count, so a second block is not an option.
Its `files` list gains the three paths beside `**/*.{ts,tsx}`:

```text
files: [
  "**/*.{ts,tsx}",
  "examples/folder-viewer/src/render-worker.js",
  "examples/folder-viewer/src/decode-worker.js",
  "examples/viewer-react/probe-decode-worker.mjs",
],
```

and its `projectService.allowDefaultProject` gains the same three, so the
files parse and typecheck as JavaScript under the block's typed parser
instead of failing project lookup. The worker global scope the three files
run in is named readonly in the same block's `languageOptions.globals`:
`self`, `postMessage`, `onmessage`, `OffscreenCanvas`, `MessageChannel`,
`MessagePort`, `MessageEvent`, `Event`, `fetch`, `WebAssembly`,
`navigator`, `console`, `performance`, `setTimeout`, `clearTimeout`,
`Error`, `Promise`, `URL` and `Blob`. Naming them in the shared block is
harmless to the TypeScript files, whose `no-undef` the typescript-eslint
recommended overrides already disable, and it keeps `no-undef` live for the
three workers, so a typo in any of them is red. The
`ocelli/no-wasm-memory-view` stance is enforced, `error`, for all three
files: the workers import `bulk.js` and `panic.js` for every linear-memory
touch and build no view themselves, `probe-decode-worker.mjs` uses
`holdSessionBytes` and `exportDecodedFrame` the same way, and no third
allowance is added to the two-file list at `eslint.config.js:282`. The
browser-suite block at `:533` gains the page globals the new and updated
suite files read, named the way the F-208 block names them.

### The React probe's decode case (S12)

The decode case lives in `examples/viewer-react/csp-probe.mjs`, which runs
explicitly per its own header with built modules and binds no port, serving
every case through `page.route` on the invented origins. The case is:

- The probe fulfils five routes from disk: the case's own minimal page, a
  probe-owned worker script at `examples/viewer-react/probe-decode-worker.mjs`,
  the leaf `/lib/ocelli-core/bulk.js` from `packages/core/dist/bulk.js`, and
  the decode glue and module
  `/lib/ocelli-wasm-decode/ocelli_wasm.js` and `ocelli_wasm_bg.wasm` from
  `crates/ocelli-wasm/pkg-decode`. No Vite config, public directory or
  build step changes, and `core-worker.ts` is untouched: the demo's own
  worker still decodes nothing.
- The probe worker imports `bulk.js` and the decode glue, instantiates the
  module, stages one wholly synthetic 1x1 multipart body through
  `holdSessionBytes` and decodes one frame, then reports the drawn sample
  count and the record's `parse_count` back to the probe.
- The policy under which the worker runs is the probe's own worker response
  policy with `'wasm-unsafe-eval'`, the same variation the policy cases
  serve, so the case measures the policy's compile-and-run claim rather
  than the demo's page.

### Rejected alternatives, recorded so they stay rejected

Decoding in the render worker contradicts section 24 and inflates the
shipped module by the codec registry. Decoding in the native backend is
F-245's refused native remote renderer and leaves no worker decode at all.
Parsing the dataset in JavaScript with dcmjs moves the core's work into the
shell, which section 10 assigns to Rust, and would be a second parser beside
`ocelli-dicom`. Relaying the frame through the main thread would put pixels
on the page, which section 24's transfer exists to avoid. Installing the
file's presentation into the render session through `restore_snapshot`
would need a viewport in the decode worker to produce the record, or a
hand-written second producer of the snapshot JSON beside `StackViewport`, so
the decode side applies stage 1 instead and the session keeps the
presentation it constructs. Returning an owned `Vec<u8>` from
`decode_staged` would invent a second copy-out beside `exportDecodedFrame`,
which already exists for exactly this shape. Reusing `openDecodePool`
(`pool.ts:39`) would manage a pool of N identical decode workers per owner
and offers none of what the remote route needs, the MessageChannel wiring,
the role split, the trap recovery and the per-generation refusals, so it
would be a wrapper that only forwards.

The whole-recipe extraction beside `preview.rs` stays refused, for the
reasons the round-two draft recorded: the shared arithmetic already exists
exactly once in `ocelli-pixel`, the recipe that differs is policy the
example owns, and AC28 makes any drift a red differential test. What R8
extracts is narrower and is justified by a second caller existing today:
the per-frame attribute resolution only, about 280 lines measured in Scope
item 9. The alternative R8 names, refusing enhanced instances by name with
a follow-up story, costs one more refusal code, one More-row gap and one
carried story, and leaves the remote route below the cornerstone floor for
enhanced objects, which is why it is rejected rather than deferred.

## Dependencies, write ownership and resources

Declared dependencies. F-245 is integrated in the base at `8949c801`, its AC8
measurement remains owed and is unaffected here. F-198, F-231 and F-236 are
done, checked in `docs/sprints/BACKLOG.md` this session. F-235 is pending
and is not a dependency, because the remote path opens no device and runs
no shader.

### The exact write set

One list, in the order the work touches them. Files marked new are new.
`packages/core/src/bulk.ts` is not in the list and is not written: the
record is read with the landed `exportDecodedFrame`
(`bulk.ts:401` to `:410`, read this session), called as
`exportDecodedFrame(wasm, facts_ptr(), 76)`, so the section 17.2 allowance
stays two files.

1. `crates/ocelli-dicom/src/frame_presentation.rs` (new), the shared
   per-frame resolvers
2. `crates/ocelli-dicom/src/lib.rs`, the module declaration and exports
3. `crates/ocelli-dicom/tests/frame_presentation.rs` (new)
4. `crates/ocelli-wasm/src/remote.rs` (new), the decode route
5. `crates/ocelli-wasm/src/lib.rs`, the decode exports and the module
6. `crates/ocelli-wasm/Cargo.toml`, the decode-worker feature's three
   edges
7. `crates/ocelli-wasm/tests/remote_decode.rs` (new), the native fixtures
8. `examples/folder-viewer/src/preview.rs`, the call sites rewire to the
   shared resolvers
9. `examples/folder-viewer/src/value.rs`, the `resolved_text` import moved
   to the crate function, no behaviour change
10. `examples/folder-viewer/src/lib.rs`, the `pub use` of the synthetic
    instance builders the differential fixture reuses, so one builder has
    two readers instead of a second copy
11. `examples/folder-viewer/Cargo.toml`, the dev-dependency on
    `ocelli-wasm` with `decode-worker`
12. `examples/folder-viewer/src/render-worker.js` (new)
13. `examples/folder-viewer/src/decode-worker.js` (new)
14. `examples/folder-viewer/src/page.html`, the Display action, the remote
    wrapper and canvas outside `.stage`, `vp.remoteDisplayed`, the painter
    gates, the one-shot test flags and the sentences
15. `examples/folder-viewer/src/serve.rs`, the worker, wasm and trap
    routes and the `OCELLI_TEST_ARMING` read
16. `examples/folder-viewer/src/csp.rs`, `worker_policy()`
17. `examples/folder-viewer/tests/remote_route.rs` (new), the AC28
    fixtures, colour and ICC included per S4
18. `examples/folder-viewer/tests/browser/worker.mjs` (new), the browser
    suite
19. `examples/folder-viewer/tests/browser/dicomweb.mjs`, the remote-state
    expectations moved to the delivered states (AC9's named edit)
20. `examples/folder-viewer/csp-probe.mjs`, the worker-policy case
21. `examples/viewer-react/src/csp-policy.test.ts`, the codec-refusal block
    removed or reconciled after F-241's landing, the decode case referenced
22. `examples/viewer-react/csp-probe.mjs`, the decode case under
    Approach
23. `examples/viewer-react/probe-decode-worker.mjs` (new), the decode
    case's probe-owned worker script
24. `scripts/decode_graph_check.py`, the D-35 restructure
25. `scripts/guards/catalogue.py`, the `decode-graph` probes and the
    rewritten `refuses=` text of the `decode-graph` and `pins` entries,
    with a probe per new `pin_and_size_check.py` refusal (D8)
26. `scripts/pin_and_size_check.py`, the decode accept path, the gzip
    measurement, and `DECODE_EXPORTS` (`:93` to `:98`, read this session)
    extended with the new decode exports
27. `scripts/tests/test_pin_and_size_check.py`, the accept-path and gzip
    unit tests
28. `ci/guard-probe-budget.json`, re-recorded with
    `python3 scripts/guard_census.py --record`, which moves the moved
    constants and refusal-site counts (D8)
29. `docs/runbooks/guard-verification.md`, regenerated with
    `python3 scripts/guard_census.py --render-runbook`
30. `docs/hld/DEVIATIONS.md`, the D-34 and D-35 rows, written in the
    change that sets this plan to approved
31. `ci/wasm-decode-size.json`, re-baselined once
32. `.github/workflows/ci.yml`, the viewer-browser job's build steps, the
    two shipped wasm builds and the two trap-probe builds (S2)
33. `eslint.config.js`, the single enforcement block's files,
    project-service setup and worker globals, and the browser-suite
    globals
34. `bin/ocelli.sh`, the suite registration, the wasm gate's added
    commands and the viewer-browser arm's description
35. `Cargo.lock`, the decode-feature edges only
36. `README.md`, `examples/folder-viewer/src/use.html` and `more.html`
37. the LLD files, exactly: `docs/lld/feature-availability.md`,
    `docs/lld/dicomweb.md`, `docs/lld/build-targets.md`,
    `docs/lld/frame-open.md`, `docs/lld/typescript-packaging.md`,
    `docs/lld/public-api.md`, `docs/lld/codecs.md` and
    `docs/lld/README.md`

The D-34 and D-35 rows are written in the change that sets this plan to
approved, per the workflow, and the implementation does not re-edit the
register.

### Read dependencies, not written

`packages/core/src/bulk.ts`, `frame-cache.ts`, `commands.ts`, `load.ts`,
`panic.ts`, `dicomweb-viewer.ts`, `generated.ts`. `crates/ocelli-wasm/src/
session.rs`, `canvas.rs`, `panic.rs`. `crates/ocelli-dicom/src/dicomweb.rs`,
`parse.rs`, `multiframe.rs`. `crates/ocelli-codec/src/registry.rs`.
`crates/ocelli-pixel/src/lut.rs`, `color.rs`, `auto_window.rs`, and its
`tests/icc.rs`. `crates/ocelli-viewport/src/commands.rs`,
`presentation.rs`, `boundary.rs`. `crates/ocelli-render/src/cpu.rs`,
`colour.rs`. `crates/ocelli-cache/src/image.rs`. `tools/bench/src/
viewer_work.mjs` and the pinned Playwright. The cornerstone dist under
`tools/oracle/node_modules/@cornerstonejs/`, read-only.

### The recorded allowlists (S14, D10, revised for the order)

Both lists are written down by name here, re-verified with `cargo tree` on
`8949c801` this session. The decode list is the resolved graph of the future
decode build,
computed as the union of `cargo tree -p <crate> --target
wasm32-unknown-unknown -e normal --prefix none` over `ocelli-dicom`,
`ocelli-codec`, `ocelli-pixel`, `ocelli-cache`, `ocelli-core` and
`cargo tree -p ocelli-wasm --target wasm32-unknown-unknown -e normal
--no-default-features --features decode-worker --prefix none`, deduplicated
to package names. It names these 95 packages, byte-identical to the
round-three list and to the round-four recompute, and is regenerated with
the real command on landing:

adler2, alloc-no-stdlib, alloc-stdlib, anyhow, brotli-decompressor,
bumpalo, bytemuck, bytemuck_derive, byteorder, byteordered, cfg-if, chrono,
crc32fast, dicom-core, dicom-dictionary-std, dicom-encoding, dicom-object,
dicom-parser, dicom-transfer-syntax-registry, either, encoding,
encoding_index_tests, encoding-index-japanese, encoding-index-korean,
encoding-index-simpchinese, encoding-index-singlebyte,
encoding-index-tradchinese, flate2, fnv, futures-core, futures-task,
futures-util, glam, heck, itertools, itoa, jpeg-decoder, js-sys,
jxl-bitstream, jxl-coding, jxl-color, jxl-frame, jxl-grid, jxl-image,
jxl-jbr, jxl-modular, jxl-oxide, jxl-oxide-common, jxl-render,
jxl-threadpool, jxl-vardct, lazy_static, libm, memchr, miniz_oxide,
num-traits, ocelli-cache, ocelli-codec, ocelli-core, ocelli-dicom,
ocelli-pixel, ocelli-wasm, once_cell, openjph-core, oxideav-core,
oxideav-mjpeg, pin-project-lite, proc-macro2, quote, ritk-codecs,
safe-transmute, serde, serde_core, serde_derive, serde_json, simd-adler32,
slab, smallvec, snafu, snafu-derive, syn, thiserror, thiserror-impl,
tracing, tracing-attributes, tracing-core, unicode-ident, uriparse,
wasm-bindgen, wasm-bindgen-futures, wasm-bindgen-macro,
wasm-bindgen-macro-support, wasm-bindgen-shared, web-sys, zmij.

The render list is the resolved graph of the default build, `cargo tree -p
ocelli-wasm --target wasm32-unknown-unknown -e normal --prefix none`,
deduplicated the same way. It names these 76 packages on the pre-F-241
graph, re-verified this session:

arrayvec, bit-set, bit-vec, bitflags, bumpalo, bytemuck, bytemuck_derive,
cfg-if, codespan-reporting, document-features, equivalent, foldhash,
futures-core, futures-task, futures-util, glam, glow, half, hashbrown,
indexmap, itoa, js-sys, libm, litrs, lock_api, log, memchr, naga,
naga-types, num-traits, ocelli-cache, ocelli-core, ocelli-pixel,
ocelli-render, ocelli-seg, ocelli-viewport, ocelli-volume, ocelli-wasm,
once_cell, parking_lot, parking_lot_core, pin-project-lite, proc-macro2,
profiling, quote, raw-window-handle, rustc-hash, scopeguard, serde,
serde_core, serde_derive, serde_json, slab, slotmap, smallvec,
static_assertions, syn, thiserror, thiserror-impl, unicode-ident,
unicode-width, wasm-bindgen, wasm-bindgen-futures, wasm-bindgen-macro,
wasm-bindgen-macro-support, wasm-bindgen-shared, web-sys, wgpu, wgpu-core,
wgpu-core-deps-wasm, wgpu-hal, wgpu-naga-bridge, wgpu-types, zerocopy,
zerocopy-derive, zmij.

Because F-241 lands first and adds the render-worker edge to `ocelli-codec`
for its video module, the render list F-202 records at landing is these 76
names plus the closure that edge resolves. Computed this session as
`cargo tree -p ocelli-codec --target wasm32-unknown-unknown -e normal
--prefix none`, deduplicated, minus the 76, the delta is these 25 packages:
`ocelli-codec` itself, `alloc-no-stdlib`, `alloc-stdlib`, `anyhow`,
`brotli-decompressor`, `jpeg-decoder`, the fourteen `jxl-*` packages,
`openjph-core`, `oxideav-core`, `oxideav-mjpeg`, `ritk-codecs`,
`tracing` and `tracing-core`. The landing-time recomputation on F-241's
actual feature declaration is authoritative. This delta assumes a
default-features edge, which is what F-241's approach names. A narrower
declaration shrinks the list.

No `wgpu`, no `naga` and no `ocelli-render` is in the decode list. With
F-241's edge in it, the render list names `ocelli-codec` and that closure
and nothing else from the decode graph, and this story adds no decoder-side
crate to it. Each graph has its own probe (Scope item 8), so an unlisted
crate in either graph turns the guard red, and the `still-links` probe
keeps the wgpu and naga fast path. Of the direct edges, the range-pinned
ones in
`[workspace.dependencies]` are the dicom-rs family at `0.10`
(`dicom-core`, `dicom-encoding`, `dicom-parser`,
`dicom-transfer-syntax-registry`), `flate2` at `1.0.28`, `uriparse` at
`0.6.4`, `glam` at `0.30`, `bytemuck` at `1`, `thiserror` at `2` and
`serde` and `serde_json` at `1`. The exact ones are `wasm-bindgen`
`=0.2.127`, `web-sys` and `js-sys` `=0.3.104`,
`wasm-bindgen-futures` `=0.4.77`, the vendored `dicom-object` `=0.10.0`,
`jpeg-decoder` `=0.3.2`, `oxideav-core` `=0.1.35`, `oxideav-mjpeg`
`=0.1.8` and the four `jxl-*` exact pins, all read from `Cargo.toml` this
session. A range-pinned direct edge moving is a reviewed allowlist edit,
not a silent one.

### Overlaps and order

Three stories share the folder viewer and the boundary, and the lead's
rulings name the files.

With F-241, which lands BEFORE this story under the lead's revised order
of 2026-11-12: the shared files are
`crates/ocelli-wasm/src/lib.rs`, `crates/ocelli-wasm/Cargo.toml`,
`scripts/decode_graph_check.py` with its `check_features` render exact-set
pin at `:74` to `:77` and `classify_graph` at `:220` to `:234`,
`scripts/guards/catalogue.py`'s `decode-graph` entry,
`examples/viewer-react/src/csp-policy.test.ts` the guard test,
`examples/folder-viewer/src/csp.rs`, `serve.rs`, `page.html`,
`packages/core/src/` and the built `pkg` and `pkg-decode`. The order the
lead revised is **F-204, F-205, F-241, then F-202**. F-241's landing adds
the render-worker edge to `ocelli-codec` under its approved AC21
(`.claude/plans/F-241-design.md:261`, read this session), which requires
editing `decode_graph_check.py`'s render pin to admit the edge and
satisfying its own guard-test demand, and leaves
`ci/wasm-decode-size.json` unedited. F-202 then restructures the script
into the per-module allowlists, with F-241's edge already in the render
list, re-baselines the decode record once, and extends `DECODE_EXPORTS`
(`:93` to `:98`). The reason the order works is the decode budget:
F-241 measures both modules against a base whose decode feature still
enables nothing new, so its receipt stays valid, and F-202's re-baseline
is the one change that moves the decode module afterwards, with the cause
recording both its own three edges and the registry content F-241 left in
`ocelli-codec`. Each AC is about its own delta, measured against its own
base, and neither raises a budget.

**What F-202 inherits from F-241 landing first, stated plainly.** The
round-four text asked the coordinator to amend F-241 in two places. Under
the revised order both are withdrawn, because F-241 lands before the
restructure and its AC21 needs no amendment:

1. Its render-list entry. F-241's landing extends the render exact-set pin
   in `check_features` itself, and F-202's restructure replaces that pin
   with the recorded render allowlist, which already names
   `ocelli-codec` and the 25-package delta above. No guard refuses
   `ocelli-codec` in the render module at any point in the sequence.
2. Its decode-budget claim. When F-241 lands, the decode-worker feature
   enables no codec, so its registry changes move no decode-module byte
   and its "the decode-worker module gains nothing" claim holds on its own
   base. F-202's landing is what first links the registry into the decode
   module, and F-202's own re-baseline records that delta.

What F-202 owes F-241 in return: the restructure keeps the four probes of
Scope item 8 so a decode edge in the render graph and a GPU edge in the
decode graph each still turn the guard red, and the re-recorded census
pins the guard's new constants.

With F-267: it is already integrated in the base this plan merges onto,
`8949c801`, the head of `sprint/s47` (`git rev-parse HEAD`, run this
session). Its approved write set touched `src/preview.rs`, `src/corners.rs`,
`src/lib.rs`, `src/serve.rs`, `src/page.html`, `tests/preview.rs`, the
`viewer-browser` arm and `README.md`, so every F-202 citation in those
files was re-executed on the post-F-267 tree. No
serialisation remains. F-202's changes to the corners painters are the
early returns of Scope item 5, which do not touch F-267's corner content.

F-204 and F-205 land before this story and write `page.html` and
`use.html`. No two of F-202, F-204,
F-205, F-241 and F-267 edit `page.html` concurrently, and F-280 in S48
will move the remote setter into the one owner it builds.

### Files that cannot have concurrent writers

The page, serve, use, preview and lib files above, and the two guard
scripts. `decode_graph_check.py` and `catalogue.py` cannot have F-202 and
F-241 writers at once, which the revised order settles: F-241 writes its
pin and leaves, F-202 restructures after.

### Exclusive resources

The wasm builds write `crates/ocelli-wasm/pkg` and `pkg-decode`, which the
size gates read. The trap builds write under
`crates/ocelli-wasm/target/`, which `.gitignore:30` ignores and the size
gates never read. Browser suites run against one started viewer at a time
on one port, and the `viewer-browser` gate runs its suites serially today
(`bin/ocelli.sh:219` to `:237`). No GPU device is opened by this story, so
no adapter slot is taken. The CI viewer-browser job needs the pinned
toolchain with the wasm32 target, wasm-pack, the root `npm ci`, a built
`@ocelli/core` so `packages/core/dist` exists, and the two wasm builds
added, because it builds none of those today
(`.github/workflows/ci.yml:409` to `:427`, read this session), and the job
gains all of them before the gate. `ci/wasm-decode-size.json` is
written once by the new accept path and read by the existing guards.

The two trap probe builds ARE CI steps (S2). The viewer-browser job builds
`decode-trap-probe` and `render-trap-probe` into their gitignored out-dirs
before the suites run, on the gate arm pattern of `bin/ocelli.sh:161` to
`:167`: check wasm-pack, build into the out-dir, run. Without them, AC25
and AC26 fail by name in CI on every run, because the cases refuse to skip
when their module is missing, which is the named-failure behaviour the
suite keeps for a local run without the build. The documented on-demand
command stays in the suite header for local runs, the way
`scripts/panic_probe.mjs:130` to `:134` reports a missing probe module and
names its build command. The job already gains wasm-pack and the wasm32
target for the two shipped builds, which is all the trap builds need.

Ledgers. No sprint ledger is written by this plan. The implementation
writes no ledger either, `/complete-feature` does.

### The feature-unification consequence (S11)

The folder viewer's dev-dependency on `ocelli-wasm` with `decode-worker`
unifies features in the workspace test graph. `cargo test --workspace` and
`cargo clippy --workspace --all-targets` then compile `ocelli-wasm` with
both `render-worker` and `decode-worker` enabled, so `remote.rs` and its
fixtures compile and run in every workspace test run, which corrects the
round-two claim that the fixture would run in no profile. What the native
build then contains is a host-target `ocelli-wasm` with both features: the
render session and the decode route together, plus all five workspace
edges of the two features. That is acceptable, because the shipped
artefacts are built by `wasm-pack` with explicit feature sets and are
untouched, the decode exports that are exclusive to the decode-only build
are `cfg`-gated out of the both-features build, and the decode-only
clippy and test lines below are what lint those exclusive exports and
compile the decode-only slot tests (S9). The cross-target
proof `bin/ocelli.sh native` builds the twelve workspace crates and not
the example, so it is unaffected.

## Verification and evidence reuse

Focused checks while implementing, exit codes read from the commands
themselves.

```bash
bin/ocelli.sh check ocelli-dicom
bin/ocelli.sh check ocelli-wasm
bin/ocelli.sh test  ocelli-wasm --features decode-worker
cargo test -p ocelli-wasm --no-default-features --features decode-worker
cargo clippy -p ocelli-wasm --all-targets --no-default-features \
  --features decode-worker -- -D warnings
bin/ocelli.sh clippy ocelli-wasm
npm run lint && npm run typecheck && npm run test
bin/ocelli.sh wasm && bin/ocelli.sh wasm-decode
python3 scripts/pin_and_size_check.py --with-size
bin/ocelli.sh gate wasm
bin/ocelli.sh gate viewer-browser
python3 scripts/decode_graph_check.py
python3 scripts/guard_census.py --record
python3 scripts/guard_census.py --render-runbook
```

The clippy line with `decode-worker` and without the default features is
new, because the workspace clippy arm at `bin/ocelli.sh:109` runs default
features only and would never compile `remote.rs`'s cfg-exclusive exports.
The decode-only test line beside it is new per S9, because the decode-only
slot tests are cfg'd on
`all(feature = "decode-worker", not(feature = "render-worker"))` and do not
compile under the unified default features. The `wasm` gate arm at
`bin/ocelli.sh:296` to `:301` gains three commands, the decode-feature test
run, the decode-only test run and the decode-feature clippy, so every
profile that runs wasm checks the fixtures, compiles the decode-only slots
and lints the route, the floor included. The workspace `test` arm at
`bin/ocelli.sh:110` now also
compiles the fixture through the unified features of S11, and the gate
addition keeps that true on the floor, where the full workspace test does
not run.

The final required profile is the feature profile on the staged tree at
completion, per the workflow, plus `viewer-browser`, `wasm`, `packages`,
`prose` and `docs`. The sprint profile runs once at sprint close.

Input and tool identities. The fixtures are synthetic Part 10 and multipart
bodies written by the tests, with no patient attributes and no corpus
writes. The browser suites use the pinned Playwright, 1.62.1 declared in
`tools/bench/package.json:22` and `tools/oracle/package.json:39`, and the
SwiftShader Chromium arguments the standing harnesses use. The React
probe's decode-in-worker case uses the same harness and a real built
decode module from `pkg-decode`.

What invalidates evidence. A change to `remote.rs`, `frame_presentation.rs`,
the worker scripts, the facts record, the served routes or the page's
remote section invalidates the affected observations. The decode budget
record is valid only for the module bytes it names, so a rebuild that
moves the size requires a new measurement, and `--accept-decode-size` is
the only writer of that record. The render budget's evidence is
invalidated by any render-feature dependency change, which the wasm gate
proves against the recorded number, and by any edit to
`ci/wasm-size-budget.json`, which AC11's diff check refuses. The D-35
guard's evidence is invalidated by any change to `decode_graph_check.py`
or the recorded lists, which the two catalogue probes re-drive red, and by
any moved constant or refusal site the recorded census pins, which
`--record` re-writes.

Performance. No new numeric performance claim is made by this story. The
window change on the remote path is a redraw from committed samples, and no
timing is attached to it. The transfer and parse saving of D-34 is a
counted observation, AC24, not a timing. If a later story claims a remote
timing axis, the subject row is registered first and measured on the
F-216 host, per `tools/bench/subjects.json`'s own rule. The registered
viewer subjects keep their bands, and because the local path is untouched,
no before and after run is owed, which is the F-245 AC8 precedent.

Semantic RED and GREEN. Each fixture is written before its code and observed
failing for the named reason, then passing after the repair, then killed by
one constant mutation. The records name the source identities at each step.

## Boundary and tier

- wasm-bindgen: ocelli-wasm only. The decode exports land in
  `crates/ocelli-wasm/src/remote.rs` under the decode-worker feature. No
  other crate gains an edge to wasm-bindgen, checked by the existing
  isolation script.
- Pixels across the boundary: no, in the D3 sense the tree already holds.
  The main thread never sees a pixel. Inside the decode worker the samples
  are copied out of the decode instance into a JS ArrayBuffer exactly as
  the landed `exportDecodedFrame` contract does, and travel to the render
  worker as section 24's own frame transfer. That is the protocol section
  24 specifies, quoted above, not a new crossing. The render instance
  receives them through `acceptTransferredFrame` and blits inside the
  worker.
- Render-loop allocation: none in the wasm render loop. The remote canvas's
  buffers are pre-sized at `bind_cpu` (`session.rs:806` to `:824`). The
  decode worker's body slot is staged once, and its wasm-side sample and
  record buffers are pre-sized at stage time from the parse and reused per
  frame. Two allocations are per batch or per frame by design and are not
  render-loop allocations: the one packed command buffer per animation
  frame is a fresh `Uint8Array` from `packCommands`
  (`packages/core/src/commands.ts:77` to `:90`, read this session), HLD
  26's own batch shape, and the worker's frame copy-out is the landed
  `exportDecodedFrame`, a fresh `ArrayBuffer` per frame
  (`bulk.ts:401`).
- unsafe: none. No new unsafe site. The two permitted files are untouched.
- Tier A (WebGPU): CPU raster, identical answer, no device opened. The GPU
  stack pass is not used and nothing is degraded.
- Tier B (WebGL2): CPU raster, identical answer, no device opened.
- Tier C (CPU): CPU raster, identical answer. This is the same CPU raster
  the other tiers get, so there is no CPU-specific second arithmetic and no
  counterfeit fallback. Decode is CPU on every tier through the same
  registry, which matches the More page's standing sentence that tiers A,
  B and C decode on the CPU.

All three rows required and answered, and the answer is the same route on
all three, reported by the status. The refusal path is tier-independent as
well, because it never resolves an adapter, which AC14 counts.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1, AC2, AC15, AC16, AC18, AC19, AC20, AC21, AC22 and AC32's hand-computed window, rescale, colour, palette, inversion, ICC and per-frame rows through the whole decode route, citing HLD 18.1, 18.2, 18.3, PS3.3 C.11, C.7.6.16.2.9 and C.7.6.16.2.10, written before the code and mutation-killed | `crates/ocelli-wasm/tests/remote_decode.rs` and `crates/ocelli-dicom/tests/frame_presentation.rs`, native, run with `--features decode-worker`, run again with `--no-default-features --features decode-worker` for the decode-only slots, and by the `wasm` gate |
| fixture | AC28, the two routes agree on u8 display bytes and window choice for the same synthetic instance, with the colour and ICC fixtures of S4 | `examples/folder-viewer/tests/remote_route.rs` |
| unit | The decode refusals, wrong UID before decode, unsupported `1.2.840.10008.1.2.4.110`, VOI LUT Sequence, Modality LUT Sequence, too large, the 76-byte record layout, the slot staging and release rules, the ICC kind codes, the parse count incremented in the single parse helper and read 1 across three `decode_staged` calls, and the decode worker's kept per-frame facts | `crates/ocelli-wasm/src/remote.rs` `#[cfg(test)]` and the fixture file |
| behavioural browser contract | The worker scripts refuse unknown message fields, hold one owner for the remote window choice, follow the one routing path, keep the page's outbound transfers to the exact set of AC12, publish a stored record on a resident hit, run every local paint entry's early return while `vp.remoteDisplayed` is true, so no local-derived overlay, cross or magnifier is written, dispatch and refuse the controls exactly as Scope item 5's routing table says, keep the remote wrapper visible as a direct child of `.cell` in tile mode, for an empty cell and after a local fault, and record the lazy fetch. Executed against the real workers, not string scans over embedded literals | `examples/folder-viewer/tests/browser/worker.mjs` |
| browser | AC1 to AC9, AC12 to AC35 through the real page, the real workers and the real built modules, with the synthetic upstream of `dicomweb.mjs` extended to serve the staged bodies the provider accepts | `examples/folder-viewer/tests/browser/worker.mjs`, registered in `bin/ocelli.sh` under `viewer-browser` before `launcher.mjs` |
| guard and probe | AC10 and AC27, the worker policy shape, the page policy's unchanged minimality, the removed React codec refusal, the decode-in-worker case, the four allowlist and fast-path probes, the banned needles still banned | `examples/viewer-react/src/csp-policy.test.ts`, `examples/viewer-react/csp-probe.mjs`, `examples/folder-viewer/csp-probe.mjs`, `scripts/decode_graph_check.py` |
| budget | AC11, the decode module size re-measured raw and gzip and recorded with its cause through the accept path, the render module proven unchanged | `ci/wasm-decode-size.json`, `bin/ocelli.sh wasm-decode`, `scripts/pin_and_size_check.py`, `bin/ocelli.sh gate wasm` |

`fixture` is mandatory for the decode arithmetic and is present. The
fixtures run under the decode-worker feature, carried into every profile
by the unified features of S11 and by the `wasm` gate addition.

## Parity surface covered

None. `docs/hld/B-parity-surface.md`'s rows are viewport types, tool
classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation
representations, core events and adapters, read this session. This story
establishes no new row. The transfer syntaxes row counts what the registry
can decode, and the registry's capability is unchanged by being reached
from a worker. The VOI LUT functions row counts the three names and is
unchanged, and the index-order caveat is recorded in the transcription and
AC13. The More matrix, which is the viewer's own statement, gains the
Partial row under AC13. F-262 owns the inventory.

## Deviations

Two rows are needed and are written in the change that sets this plan to
approved, per the workflow. The register ends at D-33, read this session.
Their drafts, in the register's columns, are under Open questions.

- **D-34.** HLD section 24's decode line carries the body on every request.
  The remote route stages the body once per displayed instance, runs the
  bounded parse once at stage time, keeps the parsed attributes and sends
  frame indexes afterwards, releasing the body at replacement, at leaving
  the remote source and at the trap respawn. The saving is one body
  transfer and one parse per display instead of one per frame step, each
  transfer up to 32 MiB, and AC24 counts both. The reason is the section 8
  rule that encoded bytes are transient, the browser rule that a
  transferred ArrayBuffer cannot be sent twice, and the same waste in a
  re-parse per frame. `scripts/deviation_check.py` refuses this draft
  until the approving change writes the two rows, which is the mechanism
  design.md step 3 names, and is the expected state of a draft that needs
  a deviation.
- **D-35.** The decode-worker feature enables exactly `ocelli-dicom`,
  `ocelli-codec` and `ocelli-pixel`, and `decode_graph_check.py` holds
  each module's resolved graph to a recorded allowlist of package names, so
  an outside crate turns the guard red and a reviewed entry is the only way
  in. The old blanket refusal of any decode-worker dependency, and the
  React test's render-module codec refusal, are both replaced. `ocelli-cache`
  and `ocelli-core` are already non-optional dependencies of the crate and
  `ocelli-pixel`'s dev-dependency entry stays. The reason is that the
  decode build exists to parse and decode, which its F-110 feature set
  could not do, and that dependency additions must stay deliberate after
  F-241's approved render-module edge.

The existing register is otherwise unchanged. D-07 is applied, not
extended, because the remote path declares the CPU answer on every tier and
reports the route. D-11 keeps the 5.8.2 pin and no parity claim is made.
D-30's named-notice rule is followed by the remote route for profiles
outside the F-240 subset, AC23. D-29's colour window and D-23's palette
input rule are followed, AC21 and AC13. D-21's decoder allocation note is
respected by the decode route, which bounds its library-owned output before
allocation and copies once. `scripts/deviation_check.py` accepts the two
new rows in the approving change.

## LLD impact

`docs/lld/feature-availability.md`, the paragraph at lines 191 to 200 that
declares remote display unavailable in all tiers until F-202, rewritten to
the delivered answer, with the three tier rows and the no-device statement.

`docs/lld/dicomweb.md`, the sentence at lines 207 to 209 that F-202 owns
remote display, replaced by the delivered route, the staged body's lifetime
through take and release, the parse-once rule and the display status
sentences.

`docs/lld/build-targets.md`, the served wasm routes and worker scripts, the
decode feature's new dependency edges and the D-35 per-module allowlists
with both recorded allowlists, the 95-name decode list and the render list
with F-241's recorded delta, the decode-feature clippy and the two test additions
to the `wasm` gate, the re-baselined decode budget with its cause, raw and
gzip, the re-recorded census and the updated `refuses=` rows, and the lazy
load.

`docs/lld/frame-open.md`, the new `frame_presentation` resolvers and the
call sites that now share them.

`docs/lld/typescript-packaging.md`, the folder viewer's page policy section
gains the worker response policy and the probe extension, and the section
24 handoff paragraph gains the decode side and the `exportDecodedFrame`
copy-out reuse for both the samples and the record.

`docs/lld/public-api.md`, an explicit note that `@ocelli/core` gains no
export in this story and the worker protocol is the example's.

`docs/lld/codecs.md`, the decode route reaching the registry from a worker.

`docs/lld/README.md`, the index entries for the changed files.

## User-facing documentation impact

`README.md`, the example viewer section gains the worker decode sentence
beside the existing native decode paragraph, and the build prerequisites
gain the two wasm builds for the remote path.

`examples/folder-viewer/src/use.html`, section 4's build-from-source gains
`bin/ocelli.sh wasm` and `wasm-decode` as remote-path prerequisites,
section 5.1's worker example is extended with the two-worker shape under
section 24 and the leaf-module imports, and section 5.6's DICOMweb text
replaces the partial display sentence with the Display action, its
refusals, the ICC notice and the refetch trap sentence.

`examples/folder-viewer/src/more.html`, the F-202 matrix row at line 183 is
rewritten to Partial with the gaps against cornerstone3D 5.8.2's DICOM
worker decode named, including the pooled-decode and colour-window rows of
AC13, and the worker decode section at lines 560 to 588 moves from Not yet
to the delivered description, including what the remote surface refuses,
the declared sample bound, the ICC notice and the trap recovery.

The inline script and style hashes of all three pages are re-derived
automatically by `csp.rs` and are never written by hand.

## Open questions

The two deviation rows are drafted here in the register's columns, per the
review's nitpick, and are written to `docs/hld/DEVIATIONS.md` only in the
change that sets this plan to approved. The operator's three decisions of
2026-11-12 stand and are recorded as decided after the drafts. One
material option remains open, with its cost stated.

- **D-34, draft.** HLD says, section 24's decode line is
  `main -> decode : { kind:'decode', seriesId, frameIndex, buffer }
  [transfer]`, the buffer on every request. We do, stage the body once per
  displayed instance, run the
  bounded parse once at stage time, keep the parsed attributes and send
  frame indexes afterwards, releasing the body at replacement, at leaving
  the remote source and at the trap respawn. Why, encoded bytes are
  transient per section 8, a transferred ArrayBuffer cannot be sent twice,
  and a re-parse per frame is the same waste in wasm, counted by AC24.
  Raised, this plan, design round 2, reaffirmed round 3 by ruling R2 and
  round 4 by the base move.
- **D-35, draft.** HLD says, section 24 names three roles and no dependency
  set for the decode module. The guard it departs from is F-110's,
  `scripts/decode_graph_check.py:68` to `:69`, whose refusal reads
  "decode-worker must not enable a dependency", with the render-module
  codec refusal carried by the `csp-policy.test.ts:211` to `:259` tree
  walk. We do, enable
  exactly `ocelli-dicom`,
  `ocelli-codec` and `ocelli-pixel` on the decode-worker feature and hold
  each module's resolved graph to a recorded allowlist, so an outside crate
  turns the guard red and a reviewed entry is the only way in. Why, the
  decode build exists to parse and decode, and dependency additions must
  stay deliberate after F-241's approved render-module edge has landed.
  Raised,
  this plan, design round 2, restructured round 3 by ruling R1 and round 4
  by ruling D10, ordered after F-241 by the round-four lead amendment.

**Open, owner the coordinator at approval.**

- **Filling the overlay corners from the remote record.** While the remote
  source is active this story hides all eleven local-derived elements plus
  the crosshair and the magnifier (AC31). The option is to populate the
  four corner texts from the facts
  record instead, columns, rows, modality and the applied window. The cost
  is a second corner producer beside `corners.rs` and `preview.rs`'s
  writer, which F-267 has now landed in the base, so the work sits behind
  the painters' new `vp.remoteDisplayed` gate, plus new sentences and ACs
  for which fields a remote record
  can honestly supply, because the record carries no patient, orientation
  or exposure data. Deferral cost, the remote canvas shows no metadata
  while displayed, which AC13's row states as a gap. Recommendation,
  defer with the option recorded here for the follow-up that takes it.
  Affects AC31 and AC13.

**Decided by the operator, 2026-11-12, design round 2, still standing.**

- **Q-a, the decode dependency allowlist.** The decode-worker feature may
  enable exactly `ocelli-dicom`, `ocelli-codec`, `ocelli-pixel` and their
  pinned dependencies. Applied in Scope item 8 as restructured by R1,
  AC27 and D-35. Affects AC11 and AC27.
- **Q-b, a separate lazy decode budget.** The render module and its budget
  stay unchanged. The decode module gets its own budget, measured at
  implementation and recorded raw and gzip, and is loaded lazily on the
  first remote decode. Applied in Scope item 1, AC11 and AC30 as
  implemented by R3. Affects AC11, AC30 and AC13.
- **Q-c, stage once.** The body is staged once per displayed instance, the
  D-34 deviation records the departure from section 24's per-request
  buffer, and AC24 measures it, now with the parse-once half R2 added.
  Applied in Scope items 1 and 5, Deviations and AC24. Affects AC4, AC24
  and AC29.

**Decided locally, owner the plan, revisable by the coordinator at
approval.**

- The frame cache budget is 128 MiB, matching F-245's decoded cap. Affects
  AC4, AC5 and AC6.
- The multiframe stepper reuses the page's scrubber with the remote frame
  count. Affects AC4, AC24, AC29 and AC33.
- The browser colour rows cover RGB and YBR_FULL, and palette and the
  remaining spaces are covered at the Rust fixture. Affects AC15 and AC21.
- The remote window setter stays separate until F-280 lands in S48, with
  the late-request guard this plan specifies, and folds into the one owner
  then. Affects AC5.
- The status sentence set is the seven sentences under Scope item 5,
  including the refetch wording of R6. Affects AC13, AC25 and AC9.
- The per-frame resolvers are extracted rather than refused, measured in
  Scope item 9. Affects AC32, AC33 and Dependencies.
- **S4's choice is the cheaper option.** AC28 gains one colour fixture and
  one ICC fixture compared between the routes, measured at about 100
  written lines in the existing differential file. The alternative, moving
  the colour recipe and the 15-kind classification into the shared crate
  module beside the resolvers, is rejected this round: the 15-kind match
  and `ColourRecipe` are about 250 lines interwoven with the folder
  viewer's own metadata types and F-240's landed tests, so the move costs
  several times the fixture cost and buys nothing AC28's comparison does
  not. Affects AC28 and AC22.
- The remote-state field is `vp.remoteDisplayed`, the name F-204's revised
  plan reads, set true at the activation function `showRemoteFrame(vp)` and
  cleared only by `remoteRelease` (D4). Affects AC31, AC34, AC35 and
  Dependencies.
- The test-only server setting is the environment variable
  `OCELLI_TEST_ARMING`, read once by `serve.rs`, off by default. Affects
  AC3, AC14, AC25 and AC26.


## Lead amendment on landing order and the one-to-one seam, 2026-11-12

This section is authoritative over every earlier sentence it touches. It is
revised by the round-four amendment for the order and the seam.

The base is `sprint/s47` at `8949c801`, which holds F-267 and the approved
F-204, F-205 and F-241 plans. F-204, F-205 and F-241 all land before F-202,
and F-202 merges onto all three. F-241 adds its own render-worker edge to
`ocelli-codec` under its approved AC21, editing `decode_graph_check.py` as
its plan needs, and F-202 then restructures the script into the
per-module allowlists with F-241's edge already in the render list.
F-204's approved plan moved the whole remote seam of the
one-to-one mode into F-202 (F-204 plan, "Lead amendment after design
review round 4"). F-202 therefore builds three things there. When a remote
frame is shown in a cell, the activation function `showRemoteFrame(vp)`
calls `fitView(vp)` (`page.html:3865`, read this session), so the
viewport's held one-to-one mode drops to fit through F-204's fit path at
the same activation point that sets the remote state field this plan
names. While the remote source is active, the
one-to-one control and its `\` key are refused with a named reason on this
plan's remote status line `#remote-state`, not the corner line, which no
paint entry writes while the field is true. On release, `remoteRelease`
hides the wrapper, clears the field, calls `layoutViewport` and repaints
the painters, so the viewport returns to its local layout and the zoom
label is restored. AC35 covers all of it: a held mode on a
local image, a remote display in the same cell reading no `Zoom 1:1`, the
control refused with its reason on the status line, and a release that
shows the local image at fit with the fit label, after which `\` enters
the mode again.

F-205's approved plan applies a stored default window preset per viewport
per series load, behind a first-paint hold tied to `loadToken`. A remote
display is not a series load and applies no stored preset. F-205's open
question 2 asks where F-202's remote frames enter the series structures,
and notes that if the remote route bypasses `loadSeries` the default would
not apply until a one-line hook is added. The remote route creates no
series entry and does not route through `loadSeries`, so the default does
not apply to a remote display, and no hook is added: that is the outcome
open question 2's own deferral sentence describes, not a gap. Every
write-set and order entry above that places F-202 before F-204, F-205 or
F-241 is void. The implementer re-reads every cited `page.html` line on
the new base before editing, because F-204, F-205 and F-241 will have
moved them.

## Lead amendment after design round 5, 2026-11-12

This section is authoritative over every earlier sentence it touches. The
independent round 5 review is tracked at
`.claude/reviews/F-202-design-pass-5.md`, and each fix below is applied
exactly as that file states it, at the lines it cites on `8949c801`.

**D1, the routing table is completed.** Every input that file names joins
the table, dispatching or refusing by name: the series list click and
double click, cell drop, `fillCells` and `startViewports`, the stack-scroll
drag tool, the window list rows of kind `file` and `auto` and the number
keys, the frame-window refresh re-show, the draw, measure, region window and
key image tools, Shift+R, tile mode through Shift+G and the layout picker,
the changed-file re-show and the `setActive` draft cancel. The field check
sits in `loadSeries`, `showIndex`, `chooseRow` and `chooseWindow`,
`resetView`, and the drag modes of `onPointerDown` and `onPointerMove`,
ahead of the `!currentImage(vp) || vp.tileMode` bail. `copyWindow`,
`copyZoom` and the link scroll skip a viewport whose field is true.
`remoteRelease` redraws the local frame with `showIndex(vp, vp.viewIndex)`
from the cache, which is not a re-open.

**D2.** `pointReadout` and `updateMag` return early, after `hideCross()` and
`mag.hidden = true`, for a viewport whose field is true. The wrapper's
z-index is above 3.

**D3.** AC31 and AC34 assert occlusion for the in-cell elements: a page
screenshot of each element's box equals the expected remote canvas bytes,
and the wrapper's computed z-index is above each sibling's. Computed
`display: none` is kept for `#cross-*` and `#mag`.

**D4.** Every render-worker spawn replaces the wrapper's canvas element with
a fresh one and transfers it. The canvas is never described as created once
or rebound.

**D5.** AC12 adds a page-side `message` listener on each worker that walks
`event.data` for ArrayBuffer, typed arrays, MessagePort and ImageBitmap and
asserts none, and a driver that sends five wheel or pointer events inside one
animation frame and asserts exactly one command buffer in the transfer list.

**D6.** F-241 lands first and owns the render-worker pin edit in
`scripts/decode_graph_check.py`, with its probe and census re-record, and the
narrowing of the `ocelli-codec` walk in `csp-policy.test.ts`. The coordinator
records that in F-241's plan. This plan's statements that F-241's AC21 needs
no amendment and that no guard refuses `ocelli-codec` in the render module
are withdrawn.

**Smells.** The field check runs at the handlers, not inside `fitView`, and
activation calls `fitView` before it sets the field. Reset dispatches
`KIND_FIT`, `KIND_SET_INVERT` with the record's own invert, and the inverse
rotation and flips. A remote W/L drag takes its origin from the remote
setter's latest choice and dispatches in an empty local cell, and remote
presets come from the remote record, not `windowRows`. AC24 adds a positive
control that drives the helper twice and reads 2. AC14 patches
`navigator.gpu.requestAdapter` and `OffscreenCanvas.prototype.getContext`
before the glue is imported. AC11 reads: `gate wasm` applies the raw and
gzip ceilings only, and the accept path refuses a no-op where built bytes
equal recorded bytes. It cites the census record and four new pins
refusals, gzip over its ceiling, a missing gzip field, a no-op accept and a
missing decode export. The `viewer-browser` arm builds pkg, pkg-decode,
`packages/core/dist` and both trap probes on the panic arm's pattern, and CI
calls the arm. more.html about 177, README about 325 and `dicomweb.mjs`
about 769 are updated and named. Scope items 3 and 4 extend F-241's page
worker serving and policy rather than add a copy. AC35 asserts `loadToken`
unchanged and no new `/preview` request. The `source-dicomweb` toggle is
refused while the field is true. A mutation caching a `wasm.memory.buffer`
view in a worker must turn `npm run lint` red.

**Nitpicks** are applied as the review file lists them, the cornerstone
facts are read at the main checkout and verified, and the status sentence
set is counted and written out in full.

**The open option is deferred.** Corners filled from the remote record need
a record extension and a second corner producer. They are not built here,
AC13 and the More row name the gap, and a follow-up story is filed when
F-202 is integrated.
