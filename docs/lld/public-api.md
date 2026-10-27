# Public API

**F-IDs that contributed:** F-100, F-101, F-104, F-050, F-103, F-102, F-X003
**Last updated:** 2026-10-27

The viewport, scene and tool surface. It is a new API, HLD section 2.1.
F-101 built the wire. F-104 moved every boundary number into one Rust table
and generates the TypeScript declarations from it. This file names what that
wire carries. The wire lives in `ocelli-viewport`, `ocelli-wasm` and
`@ocelli/core`, not in this file.

## The generated boundary module

`crates/ocelli-viewport/src/boundary.rs` is the one definition of the
command, state and ring numbers. It holds the eleven command ids and payload
lengths, the VOI function, preset, colormap and invert bytes, the flag bits,
the `ViewportState` and `RingHeader` layouts, the event stride and field
offsets, and the six ring kinds. `snapshot.rs` is the one definition of the
snapshot magic, version and error codes. `crates/ocelli-viewport/tests/typegen.rs`
writes `packages/core/src/generated.ts` from both and compares the committed
file byte for byte. A change to a Rust payload length or a struct field fails
the `test` gate until the file is regenerated:

```bash
UPDATE_TYPES=1 cargo test -p ocelli-viewport --test typegen
```

`viewport.ts`, `commands.ts` and `ring.ts` import the generated constants
rather than restating them. `VIEWPORT_STATE_BYTES` is
`size_of::<ViewportState>()`, and every `STATE_*_OFFSET` is `offset_of!` on
that struct, so the 96-byte readback and its offsets are the layout rather
than a second description of it. The ring's 48-byte stride and field offsets
come from `RingEvent` the same way.

## The boundary wire this file documents

HLD section 17.1 packs commands little-endian:

```text
[u16 kind][u16 viewport_id][u32 payload_len][payload ...] repeated
```

HLD section 17.3's event is a 48-byte stride. `kind` is `u32`, `viewport` is
`u32`, `seq` is `u64`, and `payload` is 32 bytes.

HLD section 17.4's readback is:

```text
camera: [f32; 16]
voi_center: f32
voi_width: f32
slice_index: u32
num_slices: u32
flags: u32
_pad: [u32; 3]
```

`flags` bit 0 is invert, bit 1 is loading, bit 2 is error. Bit 3, value 8,
means refining. `readViewportState` reads a 96-byte copy into plain numbers.
`drainRing` reads a copied event buffer. Neither function views wasm memory.
`coreAvailable()` stays false until a built module is loaded.

## Scene and stack viewport

A scene is an ordered list of viewport ids. It holds no pixels and no device.
Order is the order of addition. `ViewportId` is a branded number.
`viewportId` mints one. A scene stores those values. A slice index stays a
plain number.

A stack viewport records drawing availability, the sixteen camera numbers,
window centre, window width, the VOI function, the selected slice, the slice
count, a colormap, an optional window preset and an explicit invert shape.
It does not draw.

Drawing availability is `available`, `degraded` or `unavailable`, the three
states in `feature-availability.md`. The plan word `full` is published as
`available`. The stack viewport's GPU drawing is `unavailable` on tier C, and
`TIER_C_DRAWING` publishes that answer. The CPU raster path, F-X003, draws
tier C through the CPU canvas exports. It is a separate surface rather than a
fallback declared by this story, so the stack viewport's answer stays
`unavailable`.

The VOI function is `linear`, `linear_exact` or `sigmoid`. Those names map
to `VoiFunction::Linear`, `VoiFunction::LinearExact` and
`VoiFunction::Sigmoid`. Colormap names are `grey` and `stops`, the two
tables F-044 uploads. Preset names are `soft_tissue`, `lung` and `bone`.
`WindowPreset`, `Colormap` and `Invert` are declared here. `Invert` is
`identity` or `inverse`. Flag bit 0 packs `inverse`.

## Commands

The tool surface is this set and no other command:

| Command | What it asks for |
|---------|------------------|
| `Fit` | Letterbox the current image. Pan returns to zero. Zoom returns to one. Rotation and flip stay. |
| `Pan` | A canvas translation. Not a patient-space move. |
| `Zoom` | Scale about the image centre. |
| `Rotate` | Clockwise radians on a y-down canvas. |
| `FlipHorizontal` | Mirror x about the image centre. |
| `FlipVertical` | Mirror y about the image centre. |
| `SetWindow` | Centre, width and one of `linear`, `linear_exact` or `sigmoid`, through the existing chain. |
| `ApplyPreset` | `soft_tissue`, `lung` or `bone`. |
| `SetInvert` | `identity` or `inverse`. It does not XOR with photometric interpretation. |
| `SetColormap` | `grey` or `stops`. |
| `Scroll` | A step along stored order. |

A tool binding maps one caller-chosen name to one of those commands. The
pointer gesture stays in the shell. Hit-testing stays in `ocelli-geom`. The
sixty-three cornerstone tool classes are not these names. F-103 is the event
model. Annotation tools are not in the set.

`packages/core/src/viewport.ts` exports the types. It does not call wasm.

## React binding

`@ocelli/react` exports `OcelliViewport`, `useOcelliViewport` and
`createViewportPump`. The component takes a session prop. There is no context
and no module singleton. The pump enqueues commands. A frame with a non-empty queue submits one packed
buffer through `writeCommands`. An empty queue does not. Every frame drains
the ring once and dispatches F-103's named events. `checkpoint` reads
`session.snapshot()` when the caller asks, and `restore` applies that record
and then one `Scroll`. The host element is a `div` with `data-ocelli-viewport`
and `data-ocelli-core`. This story does not render a canvas. The binding
reports `TIER_C_DRAWING` because it does not resolve a GPU tier and must not
claim a drawn image.

## Snapshot

F-050 adds `snapshot`, `snapshot_error` and `restore_snapshot` on the wasm
session, and `parseSnapshot`, `sessionSnapshot`, `sessionSnapshotError` and
`restoreSessionSnapshot` in `@ocelli/core`.

`snapshot` returns the UTF-8 JSON record as a copied array. An empty array
is a refusal, and `snapshot_error` names it. Zero means the last snapshot
succeeded. `restore_snapshot` returns zero on success and the same codes
otherwise. A refused restore leaves the camera, the display area and the
presentation unchanged. Scroll, flags and generation are not in the record.

`parseSnapshot` refuses an unknown version, a bad magic, malformed JSON and
an unknown field. `SNAPSHOT_MAGIC`, `SNAPSHOT_VERSION` and `SNAPSHOT_ERROR`
come from `generated.ts`. The record stores camera inputs. It does not store
the sixteen-number matrix that `readViewportState` copies.

## Events

F-103 publishes `decodeNamedEvent`, `OCELLI_EVENTS`, `CORNERSTONE_EVENT_MAP`,
`CORNERSTONE_EVENT_COUNT`, `EVENT_DELIVERY` and `UNMAPPED_EVENTS`.

`decodeNamedEvent` reads a copied ring event. It does not take a view over
wasm memory. The layout and the 5.8.2 mapping are `docs/lld/events.md`. The
ring stride, capacity and kinds are unchanged.

## Minimal resident stack initialization

The operator approved this F-101 extension on 2026-10-19. `writeStack` writes
ordered `StackFrameKey` records through the existing ticketed bulk buffer.
Each record is little-endian u64 series, u32 instance and u32 frame. An empty
or malformed sequence is refused before replacing the viewport stack.
`configure_stack(ticket, len)` consumes only its current allocation ticket
and returns a nonzero stack epoch. Reconfiguration invalidates the selected
source and exposes loading.

`FrameCommitMeta` optionally carries both `stackIndex` and `stackEpoch`.
The public metadata adapter routes them to
`commit_selected_frame(ticket, len, viewport, columns, rows, index, epoch)`.
Only the selected index in the installed epoch can commit. After Scroll
changes the selected index, the previous sample buffer is unavailable for
rendering. The existing bitmap stays while loading. A refused stale frame
does not become the neighbour's pixels. A legacy `commit_frame` is refused
while a nonempty stack is configured. Frame pixels remain worker-owned.
This is resident selection, not F-052 series loading, decoding or prefetch.

`writeCommands` obtains its immediate memory view after allocation, because
allocation may grow wasm memory. It delegates ticket authorization to the
existing command sink. No command ID or 96-byte readback layout changed.
