# F-101, command channel, event ring and state readback

**Status**: approved
**Epic ref**: E16.2
**Sprint**: S16
**Estimate**: 5w
**Owner**: xAI initial implementation, Codex completion, independent review deferred by operator

## Normative source, transcribed

HLD section 17.1, little-endian:

```text
[u16 kind][u16 viewport_id][u32 payload_len][payload ...] repeated
```

Section 17.3:

```rust
pub struct RingHeader {
    pub head: u32,
    pub tail: u32,
    pub cap: u32, // power of two
    pub dropped: u32,
}
pub struct Event {
    pub kind: u32,
    pub viewport: u32,
    pub seq: u64,
    pub payload: [u8; 32],
}
```

Section 17.4:

```rust
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

Section 5: "Three channels, and nothing else crosses." Section 9:
"Start single-threaded per worker." Section 17.2: "Build the view AFTER the
allocation. Use it immediately. Let it go." The event stride is 48 bytes,
header 16 bytes, state 96 bytes. Drain once per animation frame.

## Problem, scope and consumer map

F-100 names eleven stack commands but calls no wasm. Bulk helpers use sink
interfaces. Cache and telemetry already encode 48-byte RingEvent values,
but no session delivers them. coreAvailable still reports false.

| Entry | Existing owner | Required observation |
|-------|----------------|----------------------|
| TS packed command buffer | STACK_COMMANDS, View, Presentation, StackSequence | One batch per RAF, same semantics as direct Rust |
| Bulk commit | bulk.ts, DICOM ingest, ImageCache | Allocation before fresh view, one accepted ownership transfer |
| Cache/telemetry events | ocelli-cache RingEvent | Existing kind numbers and payload bytes preserved |
| Ring drain and readback | ocelli-wasm binding and TS bulk.ts | Copies state, no pixels or per-event callback |
| First-frame/interaction timing | tools/bench subject registry | Presented event endpoint, never pixel readback |

## Proposed API and ownership

Keep a concrete `Session` in `ocelli-wasm`. `wasm-bindgen` stays only there.
The parser, batch and ring logic that tests can run natively live in
`ocelli-viewport` and reuse `ocelli-cache::RingEvent`. The session owns every
backing allocation.

```text
alloc(len) -> pointer or 0
apply_commands(ptr, len) -> u32
commit_frame(ptr, len, viewport, columns, rows, generation)
copy_state(out_ptr) -> 96 or 0
ring_ptr() / ring_len()
```

`FrameMeta` is four `u32` values: viewport, columns, rows, generation.
`apply_commands` returns the number of commands on success. Any refusal
returns 0. An empty buffer is success and also returns 0, which matches
"nothing applied". A non-empty refusal is distinguishable because the caller
sent bytes and received 0.

Command ids follow `STACK_COMMANDS` and start at 1. Kind 0 is invalid.
Payloads are little-endian.

| Id | Name | Payload |
|----|------|---------|
| 1 | Fit | f64 css width, f64 css height, u32 columns, u32 rows |
| 2 | Pan | f64 x, f64 y |
| 3 | Zoom | f64 |
| 4 | Rotate | f64 clockwise radians |
| 5 | FlipHorizontal | u8 0 or 1 |
| 6 | FlipVertical | u8 0 or 1 |
| 7 | SetWindow | f64 centre, f64 width, u8 function |
| 8 | ApplyPreset | u8 preset |
| 9 | SetInvert | u8 0 identity or 1 inverse |
| 10 | SetColormap | u8 0 grey or 1 stops |
| 11 | Scroll | i32 step |

Function bytes are 0 linear, 1 linear_exact, 2 sigmoid. Preset bytes are
0 soft_tissue, 1 lung, 2 bone. Pan has no z on the wire. A direct Rust pan
with nonzero z still returns `ViewError::PanZ`.

Limits are 64 commands, 4096 command bytes, ring capacity 256, one
outstanding bulk allocation, and 64 MiB maximum. `commit_frame` releases
that allocation on success and on refusal. Command bytes are read during
the call and are not retained as a caller buffer.

Structural checks and semantic checks both finish before any mutation.
A failure, including `SetWindow` on a VOI sequence, leaves `View`,
presentation and scroll unchanged. Dispatch calls the existing methods.
F-X022's refusal stays in `replace_window`.

Do not copy the HLD `from_raw_parts` example. Validate the pointer and
length against the one outstanding `Vec`. No new `unsafe`.

Cache kinds 1 to 5 stay. Ring kind 6 is `Presented`. Its payload is
generation, drawn width, drawn height and flags, each `u32`. The drawn extent
is the reduced grid during interaction and the full device on settle. Viewport
flags keep bit 0 invert, bit 1 loading and bit 2 error. Bit 3, value 8,
means refining. F-X003 sets that bit. F-103 still owns the broader event
catalogue. `coreAvailable` stays false in source until a built module is
actually loaded. F-100 names stay. Tier C protocol is live. Drawing stays
unavailable until F-X003.

Fit converts nothing inside `View`. The shell passes CSS width and height.
A device fragment used as a measurement is divided by dpr once before
`canvas_to_index`. State camera bytes remain the existing f32 uniform.

## Acceptance and discriminating tests

1. Zero, one and several packed commands decode identically on Rust and TS.
   Test every command id and payload, not only a no-op batch.
2. Truncated header/payload, overflowing length, unknown kind/viewport, bad
   enum and non-finite parameters produce the approved refusal and mutation
   boundary. Distinguish structural preflight from semantic refusal.
3. SetWindow on selected VOI sequence refuses with unchanged presentation,
   matching direct Rust. Ordinary window and modality-sequence controls pass.
4. alloc triggers memory growth before constructing the fresh JS view. Commit
   receives correct synthetic bytes exactly once. Stale pointer, duplicate
   commit and mismatched length fail without leaking ownership or panicking.
5. Ring wrap, full, empty and dropped events have exact header and 48-byte
   little-endian layout, including u64 sequence. No per-event callback occurs.
6. State is copied into JS-owned storage: camera offset 0, center 64, width
   68, slice 72, count 76, flags 80 and padding 84, total 96. Later wasm growth
   cannot detach the returned copy. No pixel array is exposed.
7. Multiple pointer events become one command call per RAF and one drain.
   Commands update actual View/Presentation/scroll, not a second TS camera.
8. Register actual runners for render.first_frame, window_level_drag and
   scroll using the existing benchmark registry definitions before done.
   Timing ends at the presented ring event. Tier-unavailable observations
   stay unavailable and are not benchmark samples.

Use Rust parser/ring tests, TS pack/drain tests and a real worker browser test
with forced memory growth. Run tests on malformed input and multiple state
transitions, then controlled mutations of stride and kind/payload mapping.
No DICOM arithmetic is introduced. Existing pixel fixtures remain mandatory
when semantic dispatch touches that chain.

## Dependencies, order and checks

F-100 is done. Read F-032/F-034 event encodings and F-021 DICOMweb sink needs.
F-048 settles mapping context. F-X003 supplies CPU presented-frame behavior.
The boundary can be built after F-048, followed by CPU integration, with final
benchmark evidence after both exist. Annotated dependent work is not a claim
that imported allocation already represents every source-level dependency.

Shared writes: ocelli-wasm Session/ring, bulk.ts, viewport types, benchmark
runners and LLD. One writer for the boundary. Serialize device/browser runs.
Run wasm crate and core-package tests, lint/typecheck, boundary lint, wasm and
native proof, actual worker browser cases, benchmark instrument/refusals, then
final integrated sprint gates. Verify actual tool pins before new wgpu calls.

## Boundary and tier

- wasm-bindgen: ocelli-wasm only
- Pixels across the boundary: no
- Render-loop allocation: pre-sized command, ring and state buffers
- unsafe: confined to existing allowed ring file if necessary and reviewed
- Tier A (WebGPU): commands/events and state of the GPU stack
- Tier B (WebGL2): same protocol, actual worker observation required
- Tier C (CPU): protocol remains live, drawing unavailable until F-X003

## Parity surface covered

Boundary behavior, no external pixel-parity claim. F-103's event model,
F-104's generated types and F-102's framework bindings are non-goals.

## Deviations and LLD impact

No new deviation approved. Update public-api, viewport, cache, build-targets,
TypeScript packaging, benchmarks and an appropriate boundary LLD with index.
Preserve the published F-100 names. Keep ring safety review local.

## What the specification does not cover

Section 17 fixes the envelope, the 48-byte event, the 96-byte state and
flags bits 0 to 2. It does not assign command ids, payload tables, batch
atomicity, allocation limits, `FrameMeta` fields or a presented-frame kind.
Those are frozen above. The operator accepted the table, the atomic batch,
and flag bit 3 with ring kind 6 on 2026-10-18.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Idle session, no alloc | `alloc` within 64 MiB | One outstanding `Vec`, pointer nonzero | A second alloc is refused and the first `Vec` stays |
| Outstanding alloc | `commit_frame` with that pointer and length | Bytes accepted, alloc released, generation stored | Stale, foreign, duplicate or mismatched length releases nothing extra and does not panic. Refusal also drops this alloc |
| Any viewport | Command batch, all checks pass | View, presentation and scroll match the same direct Rust calls. Return count |  |
| Any viewport | Any structural or semantic failure, including VOI `SetWindow` | Return 0 | View, presentation and scroll byte-for-byte unchanged |
| Ring not full | `Presented` or a cache event | 48-byte slot, sequence increments | Kinds 1 to 5 unchanged |
| Ring full | One more event | `dropped` increments, oldest policy is drop-new | Header cap stays 256 |
| State copy | Caller buffer of 96 bytes | Camera at 0, centre 64, width 68, slice 72, count 76, flags 80, pad 84 | No pixel getter and no per-event callback |

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Every id and payload decodes the same on Rust and TS, including a one-command and a several-command batch | `commands.rs` and `viewport.test.ts` | A no-op batch standing in for the table | implementation |
| AC2 | Truncation, overflow, unknown kind, unknown viewport, bad enum and non-finite values return 0 with no mutation | parser tests | First-valid-command applied before the bad one | implementation |
| AC3 | `SetWindow` on a VOI sequence returns 0 and the chain is unchanged. A modality sequence accepts a manual window. A preset on that modality sequence still refuses | presentation batch tests | A second window builder that skips `replace_window` | implementation |
| AC4 | `alloc` then one commit sees the written bytes. Stale, duplicate and mismatched commits do not panic and do not leak a second owner | session tests | `from_raw_parts` on a caller address | implementation |
| AC5 | Wrap, full, empty and dropped events keep a 16-byte header and 48-byte stride | ring tests and `ring.test.ts` | A callback per event | implementation |
| AC6 | Copied state matches the offsets above and survives a later growth | state fixture | A view over linear memory returned to the shell | implementation |
| AC7 | Several pointer samples become one `apply_commands` and one drain per frame | TS batch helper | A second camera stored in TypeScript | implementation |
| AC8 | Runners exist for `render.first_frame`, `interaction.window_level_drag` and scroll. Timing ends at kind 6. An unavailable tier is not a sample | bench registry | A pixel readback used as the end event | implementation. GPU timing stays unmeasured when no adapter is opened |

## Verification and evidence reuse

Run `ocelli-viewport` and `ocelli-wasm` check, test and clippy, `npm run lint`,
`npm run typecheck`, and the package tests. A shader or wasm export change
also runs `bin/ocelli.sh gate wasm`. No new unsafe file is added, so the
unsafe gate stays green without an allow-list edit. Benchmark registration
is a source change. It does not invent a timing sample. `coreAvailable()`
remains false in the committed package. Semantic RED is a batch that applies
the first command before a later `SetWindow` sequence refusal. Mutation
removes the preflight commit barrier and that test goes red.

## Open questions

None. The operator answered on 2026-10-18.

Command contract: the table, limits and little-endian envelope above.

Batch atomicity: structural and semantic checks before any mutation.
`apply_commands` returns 0 and writes nothing.

## Repair, 2026-10-19

The approved pointer-as-token rows above are superseded for allocation
identity. `alloc` still returns the write address, and `0` still means no
slot was created. `bulk_ticket` returns a separate nonzero lifetime id.
`commit_frame` and `apply_commands` take that ticket. A ticket that is not
the outstanding one leaves the bytes in place. A matching ticket with a bad
length or viewport still drops that slot. Ticket `0` is never a live
allocation. Old buffers are not retained.

`sessionBulkSink` and `sessionCommandSink` are the public adapters. They
keep the metadata object on `writeFrame` and pass integers to the generated
export. A refused allocation does not build a view, does not write, and does
not call commit or apply.

The boundary file `packages/core/src/boundary.wasm.test.ts` is excluded from
`npm run test` because that command does not build the module. The explicit
vitest command after a dev `wasm-pack` build is the wasm evidence. It is not
a release-size measurement.

`ring_ptr` and `ring_len` in the API block above were not exported.
`copy_state` and `drain_events` return copied bytes. `ring_capacity` is 256.

Presented frames: flag bit 3 and ring kind 6. F-103 is not pulled forward.
CSS conversion happens once before the shared mapping.

## Repair, 2026-10-19, public lifetime

The address returned by `alloc` is the write pointer. It is not an
authorization. `sessionBulkSink.commit_frame` and
`sessionCommandSink.apply_commands` return 0 and do not select the newest
ticket, even when the heap reused the address.

`allocationTicket` returns the ticket captured by the latest successful
`alloc` on that adapter. `commitAllocated` and `applyAllocated` accept only
that ticket. `writeFrame`, `submitCommands` and `acceptTransferredFrame`
read the ticket immediately after `alloc` and pass that value. A delayed
caller must pass the ticket it saved. Passing the address again is a refusal.

`writeFrame` validates `FrameCommitMeta` before `alloc` on a ticket adapter.
A later exception calls `release_bulk` for the ticket captured at that
`alloc`. A foreign ticket does not drop a newer slot.

`acceptTransferredFrame` keeps `HandoffMeta` and takes an explicit
`FrameCommitMeta` target. The committed object carries `handoff` plus the
four target integers. The grid is not inferred from `bytes`.

The three registered subjects `render.first_frame`,
`interaction.window_level_drag` and `interaction.scroll` refuse until their
endpoints exist. The native example stays a diagnostic under its own name.
AC8's runners exist and do not record a sample for those subjects. This plan
does not add a budget or a baseline.

## Operator-approved completion, 2026-10-19

The operator selected "Add minimal stack initialization" when asked whether
the missing initialization path should be implemented in F-101. Preserve the
eleven command IDs and the 96-byte state layout. Add `configure_stack(ticket,
len)` for ordered bulk frame keys, each little-endian u64 series, u32 instance,
u32 frame. A successful call returns a nonzero stack epoch. Validation finishes
before replacing the sequence. Replacement resets selection and invalidates
the previous source while preserving view and presentation.

Add `commit_selected_frame(ticket, len, viewport, columns, rows, index, epoch)`.
It accepts only the current epoch and selected index. Legacy `commit_frame`
stays the standalone-frame route and refuses when a nonempty stack is active.
The existing ticket owns the allocation. A stale foreign ticket cannot free a
new slot. A current ticket with stale selection is consumed without changing
the last accepted frame. A scroll to another index invalidates source samples
and sets loading until the selected frame commits. No neighbour is drawn.

The shell adapter validates paired `stackIndex` and `stackEpoch` metadata before
allocation. `writeStack` encodes ordered keys through the permitted bulk memory
writer. No pixels are exposed and no new command wire kind is introduced.
Regressions cover malformed input, foreign/reused tickets, stack replacement,
stale epoch/index, cache-miss publication and scroll clamping. Browser benchmark
endpoints use real DOM events, one RAF batch, selected resident bytes and the
later ring drain. Host-cost collection requires explicit representative-host
configuration for acceptance. Local instrument diagnostics are labeled.
