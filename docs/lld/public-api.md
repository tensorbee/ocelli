# Public API

**F-IDs that contributed:** F-100, F-101, F-104, F-050, F-103, F-102, F-X003, F-068, F-069, F-072, F-071, F-073, F-074, F-075, F-076, F-077, F-078, F-079, F-080, F-084
**Last updated:** 2026-11-02

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

## Tool lifecycle

F-068 stores interaction state in `packages/core/src/tools.ts`.
`createToolRegistry` returns a caller-owned registry. There is no module
singleton. A group holds tools, modes and pointer bindings, and the
viewports attached to that group.

Modes are `active`, `passive` and `off`. A new tool starts `off` with no
bindings. `active` requires at least one pointer binding. `passive` and
`off` require an empty binding list. A pointer binding is one of
`primary`, `auxiliary` or `secondary`, plus a set of `alt`, `ctrl`,
`meta` and `shift`. Modifier order does not make a second binding. A
repeated modifier on one binding is one modifier.

Setting a tool `active` on a binding moves every other active owner of
that binding, in the same group, to `passive` and clears that owner's
bindings. The moved names come back in registration order. The same mode
and the same binding set return an empty demotion list. A refused call
writes nothing.

A viewport id belongs to at most one group. `viewportId(0)` is a valid
id. A negative, non-integer or unsafe number is refused. Destroying a
group releases its viewports. Removing the active tool does not promote
another tool.

The tool name is caller-chosen and is not required to equal the command.
The command is still one of the eleven. Registering a tool does not pack
that command and does not listen for events. Reads return copies.
Hit-testing and measurement stay in `ocelli-geom` and are not computed
here.

## Pointer dispatch

F-069 normalises pointer, touch and wheel input in
`packages/core/src/pointer.ts`. `createPointerDispatch` returns a
caller-owned dispatcher for one viewport in one tool group. `takeFrame`
returns at most one packed buffer. It does not call wasm.

`Pan`, `Zoom` and `Rotate` come from a caller basis captured at pointer
down. Those camera writes replace the stored value. The last sample in
the frame is the one sample. `Pan` is `basis.panX + (x - originX)` and
`basis.panY + (y - originY)`. It is not the client position. `Zoom` is
`basis.zoom * exp(-(y - originY) / 256)`. `Rotate` is
`basis.radians + (x - originX) / 256`. `Scroll`
is one step per frame. A wheel sums its samples and then clamps once to
the i32 range. A drag whose command is `Scroll` contributes the sign of
that frame's y change. The other seven commands pack nothing.

A click is a net movement strictly under 3 CSS pixels. Once a sample is
at least 3 CSS pixels from the origin, later samples of that gesture
still pack. A second down after an up is a new gesture even before
`takeFrame`. A second pointer id does not pack. Touch uses the touch
names. Mouse and pen use the pointer names.

## Cursor and tool state

F-072 emits the shell tool events from a wrapper around the F-068 registry and writes one CSS cursor. `createToolState` does not keep a second mode store. A direct `registry.setMode` does not emit. On a successful `setMode` the subject is first. `tool.modeChanged` fires when the stored mode changes. `tool.activated` fires after that event, and only when the new mode is active. Each demoted tool then emits `tool.modeChanged` to passive, in the order the registry returns. A refusal emits nothing. `registerTool` and `removeTool` emit nothing. A throw from the listener is not caught and does not roll the registry back.

`createCursorController` reads the group it was given. The active tool that owns primary supplies both the idle cursor and the dragging cursor. Primary counts in any binding slot, and a modifier does not remove it. That remains true when the cursor string is `default`. If nobody owns primary, the first active tool in registration order supplies both phases, including when that cursor is `default`. No active tool, and an unknown group, use `default`. `sync` writes `style.cursor` on every call.

The local cursors are Pan `grab` and `grabbing`, Zoom `zoom-in`, Rotate and SetWindow `crosshair`, Scroll `ns-resize`, and Fit, FlipHorizontal, FlipVertical, ApplyPreset, SetInvert and SetColormap `default`. An override replaces one command for that controller. It does not change the shared table. This module does not listen and does not import the pointer dispatcher.

## Annotation drawing

F-071 draws a caller-supplied display list into one SVG element from `packages/core/src/annotation.ts`. `createAnnotationLayer` binds that element and one listener. The list is canvas-pixel marks. It is not the SR content tree. F-094 owns that type. Points are not projected here.

`sync` validates the whole list before it writes. A refusal leaves the previous children in place and emits nothing. The refusals are `bad-mark`, `blank-id`, `non-finite` and `duplicate-id`. The first failing mark in list order wins. One mark is checked as structure, then a blank id, then a non-finite coordinate, then a duplicate of an earlier id. A list that is not an array is `bad-mark`. An element with no owner document is `bad-mark`. A blank id is `trim().length === 0`. A stored id is not trimmed.

On success the layer replaces the element's children, stores a copy of the list, and then emits. Removals come first, in the previous order. Each next mark follows. A new id emits `annotation.added`, and `annotation.completed` after that when the new mark is completed. The initial visible, locked and selected flags do not emit change events. An id that remains emits `annotation.modified` when points or text differ, then `annotation.completed` only when completed goes from false to true, then `annotation.visibilityChanged`, `annotation.locked` and `annotation.selectionChanged`, and only for the flags that differ. `annotation.rendered` is last. Its ids are the accepted list in order. An identical list still rewrites the SVG and emits only `annotation.rendered`. A throw from the listener is not caught and does not roll the write back.

Each mark is one `g`. `data-ocelli-id` is the id. `data-selected` and `data-locked` are `true` or `false`. A hidden mark stays in the SVG with `display` `none`. A visible mark does not set `display`. A non-empty point list is one `polyline`. The `points` attribute is `x,y` pairs separated by one space, using `String` on each finite number. Comparison is `===`. A non-null text is one `text` element at the last point. Null text adds no text element. This module does not listen.

## Measurement tools

F-073 adds `length`, `height`, `cobb` and `bidirectional` in `ocelli-geom`. The angle tool is the existing `angle_at`. `length` of fewer than two points is `Degenerate`. One finite point still has `polyline_length` 0. One non-finite point is `Frame` from `polyline_length` and `Degenerate` from `length`. Height is the absolute caller-space `z` difference, not the Euclidean length. Cobb is `atan2` of the cross-product length and the absolute dot, in radians, in `0..=PI/2`. Bidirectional keeps the longer distance as `length` and the shorter as `width`. Identical endpoints stay 0. The functions take `Pt` and do not convert spaces. `packages/core/src/tools.ts` does not compute them.

F-074 adds `rectangle`, `ellipse` and `circle` in `ocelli-geom`. A rectangle is the absolute `x` and `y` differences and their product. `z` is not a factor. An ellipse area is `PI` times the absolute radii. A zero radius is `Degenerate`. A circle radius is the Euclidean distance and the area is `PI * radius * radius`. Identical points are `Degenerate`. The probe remains `Volume::sample_world`. Drag is the caller repeating that point. These functions do not compute a mean, a standard deviation, an SUV or a histogram.

F-076 adds `polygon_area` and `spline_point` in `ocelli-geom`. The area is half the absolute shoelace sum on `x` and `y`, and the function closes the ring. Fewer than three points is `Degenerate`. The spline is uniform Catmull-Rom and passes through the knots. `t` outside `0..=1` is `Degenerate`. Freehand length remains `length`. These functions do not compute region statistics.

F-075 adds `roi_statistics`, `histogram` and `suv_bw` in `ocelli-geom`. The mean divides by the count. The standard deviation divides by the count. The area is the caller's number. An empty sample list is `Degenerate`. A histogram value on an interior edge belongs to the higher bin. The maximum belongs to the last bin. SUV is not the modality LUT. Units other than BQML are unavailable. There is no default weight. Rectangle, ellipse, circle and polygon area stay the earlier functions.

F-077 adds `livewire_path` and `livewire_mask` in `ocelli-geom`. The path is 4-connected. The cost is the sum of the pixels entered, and the start pixel adds nothing. The mask marks visited pixels and does not fill an interior. An empty path is `Degenerate`. `polygon_area` and `spline_point` stay the F-076 functions. There is no TypeScript copy.

F-078 adds `arrowMark`, `labelMark`, `keyImageMark`, `etdrsRegion` and `etdrsMarks` in `packages/core`. They return display-list marks for the F-071 layer. The arrow head is at most 12 pixels and at most a quarter of the shaft. A key image stores the text `KEY`. ETDRS radii are 0.5, 1.5 and 3 mm, and a boundary belongs to the outer side. The spokes are exact axis offsets. The SR content tree remains F-094.

F-079 adds `ultrasound_direction` and `redact_rect` in `ocelli-geom`. The angle is the image plane and the length includes `z`. A missed rectangle sets `applied` false and leaves the buffer unchanged. A non-finite sample in the intersection writes nothing. `distance` stays the F-070 function. There is no TypeScript copy.

F-080 adds `stamp_brush` in `ocelli-seg`. A sample is inside when its squared index distance is less than or equal to the squared radius. A circle paints only the slice whose index equals `center_z`, and both zero bit patterns select slice 0. A sphere uses every slice. Radius 0 paints only an integer centre. A negative radius or a non-finite centre writes nothing. Segment 0 erases. `present_labelmap` is unchanged. There is no TypeScript copy.

F-084 adds `segment_at`, `label_segment` and `segment_bidirectional` in `ocelli-seg`. An in-range read returns the stored number, including 0. An outside column, a zero side, or a short buffer is `Degenerate`. A label stores trimmed text and keeps an existing segment in place. Segment 0 is `Background`. Blank text is `Blank`. The longest run wins, and an equal length keeps the run found first. One sample is a run of length 1 on both axes. `bidirectional` in `ocelli-geom` is unchanged. There is no TypeScript copy.

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
