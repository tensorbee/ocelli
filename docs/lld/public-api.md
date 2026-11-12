# Public API

**F-IDs that contributed:** F-100, F-101, F-104, F-050, F-103, F-102, F-X003, F-068, F-069, F-072, F-071, F-073, F-074, F-075, F-076, F-077, F-078, F-079, F-080, F-084, F-081, F-082, F-083, F-085, F-086, F-087, F-088, F-090, F-091, F-089, F-092, F-093, F-094, F-095, F-098, F-099, F-096, F-097, F-196, F-105, F-108, F-106, F-107, F-109, F-112, F-114, F-111, F-115, F-116, F-117, F-118, F-199, F-200, F-201, F-230, F-231, F-236, F-225, F-215, F-245
**Last updated:** 2026-11-11

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
offsets, and the seven ring kinds. `snapshot.rs` is the one definition of the
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

## Conformance report reader

The index exports `readConformanceReport(text: string)`, `CONFORMANCE_RULES`,
`CONFORMANCE_SCOPE` and `SOP_CLASS_NOTES`, and the types
`ConformanceFinding`, `ConformanceMeasure`, `ConformanceModules`,
`ConformanceReport` and `ConformanceSopClassNote`. `ReadConformanceResult`
and `ConformanceRefusal` are not re-exported from the index. The returned
discriminated union is described inline. It is `{ ok: true, report }` on
success and `{ ok: false, reason }` on refusal, with `reason` one of
`json`, `scope`, `rule` and `shape`.

`@ocelli/core` validates the report JSON, not file values. There is no
TypeScript DICOM checker and no wasm memory access in this path. Invalid
JSON is `json`. A scope string that is not the catalogue sentence is
`scope`, and that check runs before findings, so a wrong sentence wins
over an unknown rule. An unknown rule id is `rule`. Any other mismatch is
`shape`. `edition` must be a string. It is not forced to `2026d`. No
promise is made that all extra unknown keys are refused. Each finding's
`section` is resolved from the catalogue, and the section on the wire is
ignored. The returned report has `scope`, `edition`, `findings`,
`modules` and `counts`. The wire `rules` array is not part of it.

`ConformanceModules` is discriminated by `evaluated`. When true it names
the IOD, table, module list and conditions not checked. When false the
module presence rules were not evaluated, and value findings can still be
present. The SOP class selector and its note have the privacy limits
[described in DICOM ingest](dicom-ingest.md#conformance-report).
`counts.elements` counts visited elements. `counts.notCheckedUn` counts
UN elements whose values were not checked.

`crates/ocelli-dicom/tests/conformance_typegen.rs` generates
`packages/core/src/conformance.generated.ts`, distinct from the boundary
`generated.ts`. `SOP_CLASS_NOTES` is the closed
set `not a valid UID` and `not a DICOM UID`. The scope sentence, exactly
as generated, is:

> This report checks the rules it lists and nothing else. It is not a statement that the file conforms to DICOM.

The rule ids, grouped by family, are:

- Length: `VALUE-LENGTH`, `FIXED-LENGTH` and `BINARY-LENGTH`
- VR and repertoire: `VR-MISMATCH`, `REPERTOIRE` and `CHARSET-UNDECLARED`
- Multiplicity: `MULTIPLICITY`
- Text formats: `AE-SPACES`, `AS-FORMAT`, `DA-FORMAT`, `TM-FORMAT`,
  `DT-FORMAT`, `DS-FORMAT`, `DS-RANGE`, `IS-FORMAT`, `IS-RANGE`,
  `UI-FORMAT`, `PN-FORMAT` and `UR-FORMAT`
- Presence: `TYPE1-ABSENT`, `TYPE1-EMPTY`, `TYPE1C-ABSENT`,
  `TYPE1C-EMPTY`, `TYPE2-ABSENT` and `TYPE2C-ABSENT`

The generated file is the authoritative list, not this section.

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

## Options and refusals

An options object in `@ocelli/core` refuses a key it does not declare.
`packages/core/src/options.ts` holds one key list per type and
`refuseUnknownOptions`, and each entry point calls it before it reads a field.

**The in-scope rule.** Every exported interface or type alias in
`packages/core/src` whose name ends in `Options` or `Request`, plus
`FrameCommitMeta`. `OPTION_KEY_LISTS` maps these eleven names to their keys:

| Type | Checked by |
|------|------------|
| `DicomwebClientOptions` | `new DicomwebClient` |
| `DicomwebBoundedDeliveryOptions` | `new DicomwebClient` |
| `QidoSearchOptions` | `searchStudies`, `searchSeries` and `searchInstances` |
| `DicomwebProviderOptions` | `new DicomwebProvider` |
| `PointerDispatchOptions` | `createPointerDispatch` |
| `CursorControllerOptions` | `createCursorController` |
| `WadoInstanceRequest` | `postWadoInstance` |
| `KeyObjectRequest` | `writeKeyObject` |
| `SecondaryCaptureRequest` | `writeSecondaryCapture` |
| `FrameCommitMeta` | `writeFrame` with a ticketed sink, and `acceptTransferredFrame` |
| `ResidentFrameRequest` | `frameResident` |

Records such as `HandoffMeta`, `StackFrameKey` and `PackedCommand` are not
checked.

**What is refused.** An own key from `Reflect.ownKeys`, string or symbol, that
the list does not name throws `OcelliError` code `UnknownOption`. The message
is `` `${owner} has no option ${JSON.stringify(key)}` ``. The key is cut to 64
UTF-16 units first, and a symbol is written as `String(symbol)`.
`error.option` holds the same text. An inherited key is not checked. A
declared key whose value is `undefined` is accepted. A value that is not a
non-null, non-array object throws `InvalidOptions`, message
`` `${owner} is not an options object` ``. `acceptTransferredFrame`, and
`writeFrame` with a ticketed sink, throw `InvalidOptions` with `frame meta is
not an object` for a meta that is not an object, which includes a function.
An array gets the generic message. The keys of
`CursorControllerOptions.cursors` are checked against `STACK_COMMANDS`, with
that path as the owner.

The refusal comes before any fetch, allocation or write. The three QIDO
searches reject their promise rather than throw synchronously.
`writeKeyObject` and `writeSecondaryCapture` return `null` for content they
cannot write, and still throw `UnknownOption` for an unknown key.
`docs/lld/errors.md`, "Shell errors are a second namespace", lists every
shell code.

**The census.** Each key list `satisfies` the type's keys, and
`OPTION_LISTS_COMPLETE` is `true` only when every list names every field, so
a new field fails `npm run typecheck` until its list names it.
`options.test.ts`, "every options type has a key list", scans every non-test
`.ts` file in `packages/core/src` for an exported `*Options` or `*Request`
type, adds `FrameCommitMeta`, and expects exactly the names in
`OPTION_KEY_LISTS`. A new such type with no key list fails that test.

## The DICOMweb viewer provider

F-245 adds one flat core module, `packages/core/src/dicomweb-viewer.ts`,
exported from the package index. `DicomwebProvider` composes on the existing
`DicomwebClient` and adds typed QIDO envelopes and one staged instance.
`DicomwebProviderOptions` is in `OPTION_KEY_LISTS` as
`DICOMWEB_PROVIDER_OPTION_KEYS`, so `new DicomwebProvider` refuses an unknown
key like every other entry point. The legacy `DicomwebClient` surface, its
option keys, its `Promise<void>` returns and `postWadoInstance` are
unchanged. The envelope, receipt and staging contract are recorded in
`docs/lld/dicomweb.md`.

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

F-080 adds `stamp_brush` in `ocelli-seg`. A sample is inside when its squared index distance is less than or equal to the squared radius. A circle paints only the slice whose index equals `center_z`, and both zero bit patterns select slice 0. A sphere uses every slice. Radius 0 paints only an integer centre. A negative radius, a non-finite centre, or a radius whose square cannot be compared writes nothing. Segment 0 erases. `present_labelmap` is unchanged. There is no TypeScript copy.

F-084 adds `segment_at`, `label_segment` and `segment_bidirectional` in `ocelli-seg`. An in-range read returns the stored number, including 0. An outside column, a zero side, or a short buffer is `Degenerate`. A label stores trimmed text and keeps an existing segment in place. Segment 0 is `Background`. Blank text is `Blank`. The longest run wins, and an equal length keeps the run found first. One sample is a run of length 1 on both axes. `bidirectional` in `ocelli-geom` is unchanged. There is no TypeScript copy.

F-081 adds `cut_scissor` in `ocelli-seg`. It writes segment 0. A circle and a sphere call `stamp_brush`. A rectangle includes every index from its minimum through its maximum on the slice named by `z`, and both zero bit patterns select slice 0. An inverted span or a non-finite value writes nothing. `present_labelmap` is unchanged. There is no TypeScript copy.

F-082 adds `flood_fill` and `paint_fill` in `ocelli-seg`. A flood replaces the 4-connected region of the seed value. A paint replaces each sample whose stored value equals the seed. The written number is the caller segment. A different value stays. A diagonal is not connected. Segment 0 is a legal written value. An outside seed writes nothing. `present_labelmap` is unchanged. There is no TypeScript copy.

F-083 adds `threshold_roi` in `ocelli-seg`. A sample inside a rectangle or a circle is painted when its caller value is at or above `start` and at or below `end`. Both ends are included. Both zero bit patterns match. A value outside the shape stays, including a non-finite value. A non-finite value inside the shape writes nothing. An inverted span, a negative radius, or a radius whose square cannot be compared writes nothing. `present_labelmap` is unchanged. There is no TypeScript copy.

F-085 adds `edit_labelmap_with_contour` in `ocelli-seg`. A sample at an integer column and row is inside when it lies on a closed edge or when a ray toward increasing `x` crosses an odd number of edges. The ring closes from the last point to the first. A horizontal edge does not count. A non-finite vertex, a non-finite edge product, a non-finite intersection, or a product or offset of two non-zero deltas that is zero writes nothing. Fewer than three points writes nothing. `present_contour` and the representation enum are unchanged. There is no TypeScript copy.

F-086 adds `interpolate_labelmap` in `ocelli-seg`. A slice is a key when any sample equals the caller segment. Only slices strictly between consecutive keys are written. A nearer slice takes that end's mask. The exact halfway slice takes the union. Endpoint keys stay. Fewer than two keys writes nothing. An empty stack, a zero side, a bad length or mismatched slices write nothing. `present_labelmap` is unchanged. There is no TypeScript copy.

F-087 adds `windowDrag`, `windowRegion`, `stackDrag` and `MANIPULATION_TOOLS` in `packages/core`. A window drag adds the horizontal pixel delta divided by 256 to the centre, and the upward pixel delta divided by 256 to the width. A width that is not greater than 0 is refused. The caller basis is not written. A region window uses the mid-span of the caller samples, and a span of 0 is refused. A stack step truncates the upward delta divided by 256 toward zero, and a step of 0 is refused. Pan and zoom stay the F-069 expressions. The five names bind to Pan, Zoom, SetWindow, SetWindow and Scroll. These functions do not pack a command and do not apply a VOI LUT.

F-088 adds `crosshairLines`, `referenceLines` and `referenceCursor` in `packages/core`. The horizontal segment spans the viewport width at the projected y. The vertical segment spans the viewport height at the projected x. The edges are inside. A point outside, a non-finite coordinate, or a non-positive side returns nothing. `referenceLines` is `crosshairLines`. The functions do not project and do not read a frame of reference.

F-090 adds `rotationDrag`, `orientationQuadrant` and `ROTATION_TOOLS` in `packages/core`. The drag is the F-069 delta. It adds the horizontal pixel delta divided by 256 to the caller radians. TrackballRotate, VolumeRotate and PlanarRotate bind to Rotate. The quadrant truncates the angle divided by a quarter turn toward zero, then wraps into 0, 1, 2 or 3. A negative quarter turn is 3. OrientationController is that function and is not a registry tool. The module does not write a camera.

F-091 adds `magnifyCircle`, `scaleLength` and `orientationMarker` in `packages/core`. A null scale is stored as 1. An advanced scale is finite and greater than 1. A radius that is not greater than 0 is refused. A non-finite centre is refused. The scale overlay is the product of the caller millimetres and the caller pixels per millimetre. A pixels-per-millimetre that is not greater than 0 is refused. Quadrant 0 keeps the four letters. Each later quadrant moves the first letter to the end. Quadrant 4 is refused and does not wrap. The annotation layer is unchanged. These functions do not read pixels.

F-089 adds `sliceIntersection`, `segmentationIntersection` and `overlayGrid` in `packages/core`. One segment is clipped to the viewport with Liang-Barsky. The edges are inside. A zero delta and a miss return nothing. A segmentation list drops a miss and refuses one non-finite coordinate. An empty list stays empty. The grid draws interior lines for an integer column count and row count of at least 1. One cell draws nothing. The module does not project and it does not call the crosshair module.

F-092 adds `cropPlanes` and `cropDrag` in `packages/core`. At most six planes are copied. Every component must be finite. A normal whose squared length is not finite, or is not greater than 0, is refused. The drag adds the caller normal times the horizontal pixel delta divided by 256. The normal is not normalised. A finite normal whose square overflows is a legal drag and an illegal plane. The kept-side test stays in F-062. The module does not call `set_clip_planes`.

F-093 adds `sculptVertex`, `eraseNearest` and `mipIndex` in `packages/core`. A polygon needs at least three points. The selected vertex moves by the raw pixel delta and the other points are copied. The caller list is not written. The nearest mark inside a positive radius is named. An equal distance keeps the earlier mark. The function does not delete the mark. The MIP index is the earlier maximum of the finite samples. One non-finite sample refuses the call. The module does not edit an annotation and it does not read a pixel.

F-094 adds `contentTree` and `provenanceItems` in `packages/core`. The annotation type is a DICOM SR content tree. The root relationship is absent. A child uses one of five relationships. One bad node returns nothing and the caller object is not written. Provenance children are the tool, the engine version, the algorithm and the operator. A null algorithm omits that child. An empty required string returns nothing. The drawing layer is unchanged. There is no Rust twin.

F-095 adds `createAnnotationState`, `putAnnotation`, `annotationsForFrame`, `undoAnnotation` and `redoAnnotation` in `packages/core`. A record holds an id, a frame-of-reference UID and a content tree that has already passed `contentTree`. A blank id, a blank UID, or a refused tree returns nothing and does not write the previous state. Records for one UID come back in insertion order. Undo moves one successful put back. The first state has no undo. Redo restores that put. A new put drops the redo states. The module does not measure.

F-098 adds `encode_seg` and `decode_seg` in `ocelli-seg`. Frames stay in caller order. Each frame keeps its referenced SOP instance UID. Two frames may name the same UID. A sample is a whole number of at least 0, and 0 is background. One bad frame refuses the whole call. Decode does not sort. The shell does not export the round trip. `present_labelmap` is unchanged. There is no TypeScript copy.

F-099 adds `import_rtstruct_contour` in `ocelli-seg`. A closed planar contour needs at least three patient points. Each z must lie within half the slice spacing of the caller slice. The stored z is the caller z. Segment 0 is refused. A non-finite point is refused. The shell does not export the import. `present_contour` is unchanged.

F-096 adds `persistMeasurements` and `loadMeasurements` in `packages/core`. The document is JSON with format `ocelli-measurement` and version 1. Every root is copied through `contentTree`. One bad node returns nothing. A count of 4000 content items returns nothing. An empty list loads as a new empty array. The module does not define a bulk binary layout. There is no Rust twin.

F-097 adds `writeTid1500` and `readTid1500` in `packages/core`. dcmjs 0.52.2 writes a Comprehensive 3D SR document. The template identifier is 1500. Each root must contain one NUM. The number, the unit and the concept round-trip. An operator name is stored and is not returned. A count of 4000 content items returns nothing. SCOORD graphics are not mapped. There is no Rust twin.

F-196 adds `writeKeyObject`, `readKeyObject`, `writeSecondaryCapture` and `readSecondaryCapture` in `packages/core`. The key object is TID 2010 with one description and one composite reference. The secondary capture is 8-bit MONOCHROME2. The detail is `docs/lld/key-object.md`. There is no Rust twin.

F-215 adds `writeTid1500Groups`, `readTid1500Groups`, `writeKeyObjects` and `readKeyObjects` in `packages/core`, and exports the `ContentItem`, `Concept`, `GraphicType` and `Relationship` types of `sr.ts`. `GraphicType` gains `ELLIPSE`, which takes exactly four points, and an `IMAGE` item gains an optional `frame`, a Referenced Frame Number of at least 1. `writeTid1500Groups` writes one Comprehensive 3D SR whose TID 1500 row 6 container holds one TID 1410 group per root, each a `(125007, DCM)` container of any value type, with the relationship written with a space. A `ReportSource` carries the patient and study attributes, Content Date and Content Time, the evidence and the predecessors. An empty group list is written only when the source names a predecessor, as a report whose row 6 container is empty. `readTid1500Groups` returns the groups, the Predecessor Documents Sequence instances and a count of groups it could not read, and decodes every string of the content tree as the file's Specific Character Set says. The set is read from the file's own (0008,0005) bytes, because dcmjs rewrites the parsed value to ISO_IR 192. Each nested string is turned back into its bytes by inverting the windows-1252 or Latin-1 decode dcmjs used, which is exact for all 256 byte values, and decoded with the set's label. The sets read are those of PS3.3 C.12.1.1.2 without code extensions: the default repertoire, written as no value or as ISO_IR 6, ISO_IR 100, 101, 109, 110, 144, 127, 126, 138, 148, 203 and 166, ISO_IR 192, GB18030 and GBK. ISO_IR 13 is not. In a single-byte set a C1 byte, 0x80 to 0x9F, makes the string unreadable. Under ISO_IR 166 byte A0 is NO-BREAK SPACE, and a byte of DB to DE or FC to FF, the positions ISO-IR 166 says shall not be used, also makes it unreadable. The walk reads Implicit and Explicit VR Little Endian and Explicit VR Big Endian. A file it cannot walk, such as a deflated one, is read as the default repertoire when it has no Specific Character Set, and has every group counted unread when it has one. The parse is always retried past dcmjs's refusal of a set, so such a file is still read rather than skipped. A UT split on a backslash is joined back whole. A file whose set uses code extensions or is not one of those is read, and every group in it is counted unread rather than shown with guessed text. A NUM may carry `unitMeaning`, the UCUM meaning. `writeKeyObjects` writes one Key Object Selection whose references named in the evidence are IMAGE items for an image storage class, WAVEFORM items for a waveform class, and COMPOSITE items otherwise. `readKeyObjects` returns the references, Content Date, Content Time, Instance Number and Manufacturer. `writeKeyObject` follows the same rule, and `readKeyObject` reads all three. Every UID these writers make is a `2.25` value of 128 random bits, and every file carries the PS3.10 Table 7.1-1 file meta, File Meta Information Version and Implementation Class UID included. The detail is `docs/lld/key-object.md`. There is no Rust twin.

F-105 adds `enableElement`, `disableElement` and `dispatchOnElement` in `packages/core`. One element has one registration. The event type is the Ocelli name. The detail is the caller object. A second enable returns nothing. Disable removes the registration and then sends `viewport.unmounted`. The React host div uses the same functions. Tiers A, B and C are n/a.

F-108 adds `rewriteListenerNames` in `packages/core`. A listener string literal is rewritten when the event map has an Ocelli name. An unmapped name stays. `cornerstone.enable` stays. The guide is `docs/lld/migration.md`.

F-106 adds `attachSideBySide` in `packages/core`. One Ocelli element and one neighbour are stored. The same object cannot be both. Release disables only the Ocelli element. The neighbour is not enabled. Tiers A, B and C are n/a.

F-107 adds `setKindEnabled`, `kindEnabled` and `enableFlagged` in `packages/core`. Three kind flags start off. Stack turns on before MPR, and MPR turns on before volume. An element is enabled only while its kind is on. Turning that kind off disables the elements this story enabled. A mount that turns the kind off does not leave the element dispatching. Tiers A, B and C are n/a.

F-109 adds `shadowDivergence` in `packages/core`. The caller passes two byte buffers and an integer limit from 0 to 255. The count is the samples whose absolute difference is greater than that limit. Unequal lengths return nothing. The buffers are not written. The function does not render. Tiers A, B and C are n/a.

## React binding

`@ocelli/react` exports `OcelliViewport`, `useOcelliViewport` and
`createViewportPump`. The component takes a session prop. There is no context
and no module singleton. The pump enqueues commands. A frame with a non-empty queue submits one packed
buffer through `writeCommands`. An empty queue does not. Every frame drains
the ring once and dispatches F-103's named events to the caller. `OcelliViewport` also dispatches those events on the host div. `checkpoint` reads
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

## Session frame cache

F-231. `packages/core/src/frame-cache.ts` calls the session cache's exports
and `docs/lld/cache.md` says what the cache does. `setFrameBudget(wasm,
bytes)` sets the budget in bytes, a `u32`, and 0 is off, the default.
`frameResident(wasm, request)` and `showResident(wasm, request)` take a
`ResidentFrameRequest` of `index`, `epoch`, `content`, `columns` and `rows`.
`frameResident` answers whether the frame is held for that content id and
extent and is not a use. `showResident` draws a held frame and returns
true, returns false for a miss, a stale entry or a wrong epoch, and throws
`OcelliError` code `CommitRefused` when, under the current epoch, the index
is not the selected slice, as `writeFrame` does, so a caller can tell a refusal
from a miss. `releaseSeries(wasm, series)` returns the frames released.
`releaseStudy(wasm, series)` takes the caller's list of series, validates
every entry before the first call, skips duplicates and sums the counts,
one export call per series, because the cache holds no study.
`requestFramePressure(wasm)` asks for one `cache.pressure` reading.

`FrameCommitMeta` gains an optional `content`, a `bigint` `u64`. A meta with
`stackIndex`, `stackEpoch` and `content` commits through
`commit_cached_frame`, and one with the two stack keys alone through
`commit_selected_frame` as before. A `content` without both stack keys is
refused with `InvalidValue`, so a content id is never accepted and then
ignored. `contentId(wasm, sopInstanceUid, transferSyntaxUid, encoded)` in
`bulk.ts` writes the three back to back into the bulk slot and calls
`content_id`, which hashes through `ContentId::of` in Rust and drops the
slot, so FNV-1a exists once and no pixel crosses up. An `undefined` answer
is a refusal and throws `CommitRefused`, because 0 is a legitimate id.

A non-`u32` index, epoch, column, row or budget and a `bigint` outside `u64`
are `InvalidValue`, and an unknown request key is `UnknownOption`, each
before any export call. `ResidentFrameRequest` and `FrameCommitMeta`'s new
key are in F-230's key lists in `options.ts`.

## Decode worker pool

`openDecodePool` opens one to eight decode workers for one owner. The
default count is 1 and the ceiling is 8. A count outside that range, a
non-integer, a non-object owner, or a second open of the same owner
returns null and does not call the starter. `threadingEscalation` returns
`stable`. `sharedMemory` is false. The product file does not name a
shared-memory buffer. The render worker is not started here. Tiers A, B
and C are not applicable.

## Browser matrix

`certifyRow` recomputes one matrix row. A caller-supplied status is
ignored. A missing observation is `not-run`. A row is `certified` only
when the exit code is 0, the resolved tier equals the expected tier, and
the renderer string is non-empty. Tier B is certified only when that
string does not contain `SwiftShader`. Tier C is certified only when the
resolved tier is C and the string does contain `SwiftShader`. A
`SwiftShader` string with expected tier B returns null. Any other
finished run is `observed`. Tier A is observed and is not certified.
The committed matrix has no observation, so every named row is
`not-run`. The function does not launch a browser.

## Cold start

`startupArtifact` returns `pkg/ocelli_wasm_bg.wasm` for `render` and
`pkg-decode/ocelli_wasm_bg.wasm` for `decode`. Any other role returns
null. `summariseColdStart` adds `glue`, `fetch`, `compile`,
`instantiate` and `firstCall` only when the profile is `release` and
every phase is finite and at least 0. `summariseFirstFrame` returns the
caller interval on that same rule. A dev profile returns null. The
functions do not read the 2.3 ms baseline and do not close gate A4.
Tiers A, B and C are not applicable.

## Keyboard actions

`keyAction` returns a pan name for `ArrowLeft`, `ArrowRight`, `ArrowUp`
and `ArrowDown`. Shift changes `ArrowUp` to `scroll-previous` and
`ArrowDown` to `scroll-next`. A shifted horizontal arrow stays a pan.
`+` and `=` zoom in. `-` zooms out. Any other key returns null. The
match is case-sensitive. The function does not listen and does not move
a camera. Tiers A, B and C are not applicable. The key map is the
approved local plan.

## Migration order

`nextMigrationKind` returns `stack`, then `mpr`, then `volume`, and only
when the done list is a prefix of that order. An empty list returns
`stack`. The finished list returns null. A skip, a duplicate or an
unknown name returns null. The function does not rewrite the list and
does not flip a flag. Tiers A, B and C are not applicable. The page is
`docs/lld/playbook.md`.

## Stable claim

`mayClaimStable` returns true only when D-22 is closed, the matrix row
count is an integer of at least 1, and the certified count is that same
integer. Every other input returns false. `removalMinorsRemaining`
returns null for `0.x` and for `patch`, 1 for `minor`, and 0 for
`major`. Any other channel returns null. The functions do not publish
and do not change the workspace version. Tiers A, B and C are not
applicable. The page is `docs/lld/release-policy.md`.

## Provenance class

`classifyProvenance` returns `recorded` when the message has one
`Ocelli-Verify` line and one `Ocelli-Generated-By` line, and that verify
line has the fields `profile`, `gates`, `corpus` and `tree`. A field
starts after whitespace or the colon. A longer token does not count.
It returns `missing` when neither trailer line is present and there is
no agent co-author line. It returns `forged` when an agent co-author line is
present, when only one trailer line is present, when a trailer line is
duplicated, or when the verify line lacks one of those four fields. The
agent tokens are Claude, Codex, GPT and Copilot, matched without regard
to case, after `Co-Authored-By:`. A `Co-Authored-By` line that names
none of those tokens does not by itself forge the message. The function
does not write a trailer. `recorded` does not prove the named tree
matches a commit. `missing` is not success. Comparison fields are not
required. Tiers A, B and C are not applicable.

## Example layout

The example page layout section calls `overlayGrid`, `stackDrag`,
`referenceLines` and `referenceCursor`. The grid count is the returned
line count. The drag line is the stack step. The two reference lines are
labelled horizontal and vertical. The cursor is the caller point. Full
screen reads unavailable when `document.fullscreenEnabled` is not true.
Prior studies, linked scroll, synced window, MPR, slab, volume rendering
and fusion read unavailable. Those controls are F-203. The section does
not draw and it does not add a formula. Tiers A, B and C are not
applicable.

## Example manipulation

The example page manipulation section calls `windowDrag`, `windowRegion`,
`rotationDrag`, `orientationQuadrant`, `orientationMarker`,
`magnifyCircle`, `scaleLength`, `cropPlanes`, `packCommands` and
`describeError`. Preset, flip and colour map are packed for viewport 1
and the page shows the packed length. The packed bytes are the command
evidence. Window centre follows the horizontal drag and width follows
the upward drag. The page can change the rotation drag, the magnify
scale and the crop count. The default button uses rotation x 512, a
null magnify scale and one plane. The crop line is crop planes. It is
not a display shutter. One to one, histogram and four corners read unavailable. Those
controls are F-204. The section does not draw and it does not add a
window formula. Tiers A, B and C are not applicable.

## Example measurement

The example page measurement section calls `persistMeasurements`,
`loadMeasurements`, `putAnnotation`, `annotationsForFrame`,
`undoAnnotation`, `writeTid1500`, `readTid1500`, `keyImageMark`,
`keyAction` and `createPointerDispatch`. The page passes 12.5 mm for
the stored document and for the TID item. The TID line is the number
`readTid1500` returns. Each button run starts a new annotation state, and undo of
that one put is empty. A touch pointer down is `touch.start`. A mouse
down is `pointer.down`. Metadata browser and preferences read
unavailable. Those controls are F-205. Secondary capture stays on the
document panel. The section does not parse DICOM tags and it does not
add a writer. Tiers A, B and C are not applicable.
