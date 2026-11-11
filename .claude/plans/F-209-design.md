# F-209, Folder viewer linked viewports, scroll by position in a shared frame of reference, synced window and zoom, and reference lines from the library

**Status**: approved
**Epic ref**: E38.19
**Sprint**: S46
**Estimate**: 4w

## Normative source, transcribed

### HLD

`docs/hld/07-concurrency-and-typescript.md` section 10:

```text
- Tool interaction state machines; the geometry and hit-testing behind them is Rust, in ocelli-geom
```

`docs/hld/13-core-types.md` section 16:

```rust
/// CSS pixels inside a viewport element. Origin top-left, y increases down.
pub enum Canvas {}
/// DICOM patient coordinate system (LPS), millimetres.
pub enum World {}
/// Voxel indices within a volume. Origin at voxel (0,0,0).
pub enum Index {}
```

> *The payoff: Transform\<Canvas, World\> composes with Transform\<World, Index\> and will not compose with anything else. A whole class of tool bugs stops compiling.*

`docs/hld/04-boundary-and-data-path.md` section 5.1:

> Small, infrequent, one call per user intent: set camera, set VOI, activate tool, load series. **Never one call per pointer move.**

`docs/hld/15-lut-chain.md` section 18 keeps the LUT chain in one place. A synced window is a window command sent to F-206's `/preview`. The page maps no grey value.

The allocation note for this story: "Uses the library reference line of F-199. The page computes no geometry of its own."

### DICOM PS3.3

C.7.6.2.1.1, Image Position and Image Orientation. Image Position (Patient) (0020,0032) is the x, y and z of the centre of the first voxel transmitted. Image Orientation (Patient) (0020,0037) is the direction cosines of the first row and the first column with respect to the patient. The mapping from pixel to patient millimetres is

```text
| Px |   | Xx*Δi  Yx*Δj  0  Sx |   | i |
| Py | = | Xy*Δi  Yy*Δj  0  Sy | * | j |
| Pz |   | Xz*Δi  Yz*Δj  0  Sz |   | 0 |
| 1  |   | 0      0      0  1  |   | 1 |
```

with `X` the row direction cosine, `Y` the column direction cosine, `S` Image Position (Patient), `i` the column index, `j` the row index, `Δi` the column spacing and `Δj` the row spacing, both from Pixel Spacing (0028,0030), whose first value is the row spacing.

C.7.4.1.1.1, Frame of Reference UID (0020,0052). It identifies the frame of reference of a series. Each series has one. Several series in a study may share it, and all images that share it are spatially related.

### The library this story calls

`crates/ocelli-pixel/src/image_plane.rs`, `ImagePlane::index_to_world`:

> `x` is column index `i`, so the first matrix column uses column spacing and the row direction cosine. `y` is row index `j`, so the second uses row spacing and the column direction cosine. The third column is a unit plane normal. It makes the transform invertible while leaving every in-plane point exactly on the specified equation.

`ImagePlane::slice_normal` is "the unit slice normal, DICOM PS3.3 C.7.6.2.1.1's `X cross Y`".

`crates/ocelli-dicom/src/frame_geometry.rs`, `FrameGeometry::derive`:

> Image Position Patient, Image Orientation Patient and Pixel Spacing are resolved per-frame first, then shared, then at the top level, which is DICOM PS3.3 C.7.6.16.1.1's order. ... Returns [`FrameGeometryError`] for an absent or malformed attribute, a frame outside the declared count, or a plane `ocelli-pixel` refuses. No route substitutes a default.

It refuses Imager Pixel Spacing alone as `UncalibratedSpacingOnly`.

`crates/ocelli-viewport/src/sync.rs`, `SyncScene`:

> The UID is compared as a string. A missing UID moves nobody.
> [`SyncError::Unavailable`] when `source` was not registered, when its UID is missing, or when any same-UID plane refuses the point. No slot is written.

`SyncScene` holds `HeldPlane`, which is one anchor, one normal and one spacing, so it describes a uniformly spaced volume. A folder of single-frame files is a list of planes that need not be uniform.

`packages/core/src/crosshair.ts`, the F-199 library reference line:

```ts
/** The same two segments as `crosshairLines`. */
export const referenceLines = crosshairLines;
```

> The caller supplies the canvas projection. This module does not project and does not read a frame of reference.

### The reference oracle, cornerstone3D 5.8.2

Read from `tools/oracle/node_modules/@cornerstonejs/tools` 5.8.2, which is MIT and in bounds. These are the behaviours this story reproduces. They are quoted, not paraphrased.

`synchronizers/callbacks/areViewportsCoplanar.js`:

```js
const dotProducts = vec3.dot(viewPlaneNormal1, viewPlaneNormal2);
return Math.abs(dotProducts) > 0.9;
```

`synchronizers/callbacks/imageSliceSyncCallback.js`, after the coplanar check and with an identity registration when the two Frame of Reference UIDs are equal:

```js
const closestImageIdIndex2 = _getClosestImageIdIndex(targetImagePositionPatientWithRegistrationMatrix, targetImageIds);
...
function _getClosestImageIdIndex(targetPoint, imageIds) {
    return imageIds.reduce((closestImageIdIndex, imageId, index) => {
        const { imagePositionPatient } = metaData.get('imagePlaneModule', imageId);
        const distance = vec3.distance(imagePositionPatient, targetPoint);
        if (distance < closestImageIdIndex.distance) {
            return { distance, index };
        }
        return closestImageIdIndex;
    }, { distance: Infinity, index: -1 });
}
```

The source point is the source image's Image Position (Patient). The comparison is strict, so a tie keeps the lower index. The target moves only when the index differs from its current one.

`synchronizers/callbacks/voiSyncCallback.js` copies `voiRange` to the target, and copies `invert` only when `options.syncInvertState` is set.

`synchronizers/callbacks/zoomPanSyncCallback.js` copies `getZoom()` unless `syncZoom === false`, and `getPan()` unless `syncPan === false`.

`tools/ReferenceLinesTool.js`, with defaults `enforceSameFrameOfReference: true` and `showFullDimension: false`, and `EPSILON = 1e-3` from `core/constants/epsilon.js`:

```js
if (this.isParallel(viewPlaneNormal, sourceViewPlaneNormal)) { return renderStatus; }
const targetViewportPlane = csUtils.planar.planeEquation(viewPlaneNormal, focalPoint);
const pointSet1 = [topLeft, bottomLeft, topRight, bottomRight];
const pointSet2 = [topLeft, topRight, bottomLeft, bottomRight];
let pointSetToUse = pointSet1;
let topBottomVec = vec3.subtract(vec3.create(), pointSet1[0], pointSet1[1]);
topBottomVec = vec3.normalize(vec3.create(), topBottomVec);
...
if (this.isPerpendicular(topBottomVec, viewPlaneNormal)) { pointSetToUse = pointSet2; }
const lineStartWorld = csUtils.planar.linePlaneIntersection(pointSetToUse[0], pointSetToUse[1], targetViewportPlane);
const lineEndWorld = csUtils.planar.linePlaneIntersection(pointSetToUse[2], pointSetToUse[3], targetViewportPlane);
...
isPerpendicular: Math.abs(vec3.dot(vec1, vec2)) < EPSILON
isParallel:      Math.abs(vec3.dot(vec1, vec2)) > 1 - EPSILON
```

`core/utilities/planar.js`:

```js
function linePlaneIntersection(p0, p1, plane) {
    const [x0, y0, z0] = p0; const [x1, y1, z1] = p1; const [A, B, C, D] = plane;
    const a = x1 - x0; const b = y1 - y0; const c = z1 - z0;
    const t = (-1 * (A * x0 + B * y0 + C * z0 - D)) / (A * a + B * b + C * c);
    return [a * t + x0, b * t + y0, c * t + z0];
}
function planeEquation(normal, point) { const [A, B, C] = normal; const D = A * point[0] + B * point[1] + C * point[2]; return [A, B, C, D]; }
```

The four corners come from `getViewportImageCornersInWorld`. When the source shows the whole image they are `indexToWorld` of `[0,0,0]`, `[columns-1,0,0]`, `[0,rows-1,0]` and `[columns-1,rows-1,0]`, the centres of the four corner pixels.

## What the specification does not cover

**The library has no plane intersection.** F-199's `referenceLines` is `crosshairLines`. It takes a point already projected to canvas pixels and returns a horizontal and a vertical span. It cannot draw the line where one viewport's plane cuts another, and it reads no frame of reference. `sliceIntersection` clips a caller segment and also does not project. Nothing in `crates/` intersects two image planes or chooses the nearest image in a list of planes. This story therefore adds that geometry once, in Rust, and the page draws the returned segment. Where it lives is Open question 1. The default is a new module in `ocelli-viewport`, beside `placement.rs` and `sync.rs`, because that is where the next wasm consumer will look and the arithmetic must exist once.

**The page cannot load the wasm core.** It is inline script served by the local process. It gets geometry the same way it gets grey bytes, from that process.

**How the page obtains geometry.** `/inspect` is already posted once per file when a folder is read. It gains two fields, so no file is posted again for geometry, which also suits F-216's "each file posted once".

- `frameOfReferenceUid`, the trimmed text of (0020,0052), `""` when absent.
- `planes`, one entry per frame. Each entry is the plane `FrameGeometry::derive` returned, written as a token, or `null` when it refused.

A plane token is thirteen numbers joined by commas, each written with Rust's `{}` for `f64`, which is the shortest text that reads back to the same value:

```text
Sx,Sy,Sz,Xx,Xy,Xz,Yx,Yy,Yz,rowSpacing,columnSpacing,rows,columns
```

`S` is the validated position, `X` and `Y` the validated cosines from `ImageOrientationPatient::row()` and `column()`, then `PixelSpacing::row_mm()`, `column_mm()`, and `ImageDimensions::rows()`, `columns()`. Synthetic slice 3 reads `0,0,7.5,1,0,0,0,1,0,0.7,0.5,96,128`.

**The page treats a token as opaque text.** It stores it on the image record and posts it back. It never splits it, never calls `Number` on it, and never adds, subtracts or compares its parts.

**`POST /link`.** One request per change of the active viewport's image, coalesced to one in flight with a dirty flag, the same pattern as the live W/L draft. The body is plain text, one line per viewport.

```text
source <uid> <plane>
target <cell> <uid> <shown> <scroll> <plane> <plane> ...
```

`<uid>` is the UID text or `-` when empty. `<plane>` is a token or `-`. `<shown>` is the index the target cell shows now. `<scroll>` is `1` when linked scroll is on. The target's planes are in the page's image order for that series, one per listed image. The response is JSON.

```json
{"targets":[{"cell":2,"index":5,"line":[0.5,10.5,127.5,10.5],"reason":""}]}
```

`index` is the image the target should show, or `null` when it must not move. `line` is the reference line on the image the target will show, in the page's image pixel units, or `null`. `reason` is one of `""`, `frame-of-reference`, `no-plane`, `not-coplanar` and `parallel`. A malformed body, a token the library constructors refuse, or a `<shown>` outside the list is a `400` with `X-Ocelli-Error: link`, because the page only ever sends tokens the process wrote.

**Pixel units for the line.** The library's index space puts the centre of the first pixel at `0`. The page's marks put the left edge of the first pixel at `0`, so `imageToClient` reads a pixel centre at `i + 0.5`. The handler adds `0.5` to both line coordinates in one named constant, `PAGE_PIXEL_CENTRE`, and the page draws the numbers it receives. That `0.5` is a unit change between two local conventions, not DICOM geometry.

**Rules taken from the oracle.**

| Behaviour | Rule here | cornerstone3D 5.8.2 |
|---|---|---|
| Same frame | UIDs equal as strings, both non-empty | same, `getFrameOfReferenceUID()` equality |
| Different frame | Refused, `frame-of-reference` | Registers the two frames with `calculateViewportsSpatialRegistration`. Not done here, because the story is a shared frame of reference. |
| Coplanar | `abs(dot(source normal, target shown normal)) > 0.9` | same |
| Nearest image | Least Euclidean distance between source IPP and each target IPP, strict `<`, lower index on a tie | same |
| A target image with no plane | Whole target refused, `no-plane` | Would throw on a missing `imagePlaneModule`. Refusing rather than skipping keeps one gap from silently moving the choice. |
| Reference line | Source corners at corner pixel centres, edge pair chosen by `isPerpendicular`, two line-plane intersections, then into the target's index space through `index_to_world().inverse()` | same, with `showFullDimension: false`. cornerstone3D uses the visible part of a zoomed source. This story always uses the whole source image. |
| Parallel | `abs(dot) > 1 - 1e-3`, no line, `parallel` | same |
| Window sync | The source viewport's window command is copied on release of a W/L drag, on a preset and on Reset. Invert is not copied. | `voiRange` on every VOI change, invert only with `syncInvertState` |
| Zoom sync | Target scale is its own `fitScale` times the source's scale over the source's `fitScale`. Pan is not copied. | `getZoom()` and `getPan()` |

Window sync is a command and does not depend on the frame of reference, as in the oracle. A preset sent to a non-CT target is refused by F-207's preset availability, and that target keeps its own window.

**Lead amendment, colour targets, 2026-11-05.** Made by the sprint lead under the operator's delegation of the quality call, after a team using legacy Cornerstone reported wrong contrast when a greyscale window was copied onto an RGB image. The presentation stage already refuses VOI on colour (`crates/ocelli-pixel/src/lut.rs`, tested in `crates/ocelli-pixel/tests/lut_chain.rs`) and the folder viewer refuses non-monochrome images, so no wrong picture can result. What this plan left unspecified is the target's state. A window command reaching a target whose image cannot be windowed is skipped for that target alone, its own window and invert are kept, and its corner says `Window not applicable`. Zoom sync is independent and still applies. AC19 covers it.

**No echo.** Only the active viewport sends `/link`. A target moved by a link response calls `showIndex` without sending one. Making another cell active sends one request so lines refresh.

**Link controls.** A toolbar button `id="link"`, `aria-label="Link viewports"`, opens `#link-menu` with three switches, Scroll, Window and Zoom, and a display toggle `id="toggle-reference"`, `aria-label="Reference lines"`. The links cover every visible viewport as one set. Defaults are Open question 3.

**Drawing the line.** Each cell's ink layer gains `<g class="ref-lines">`. The page writes `x1`, `y1`, `x2` and `y2` from the response and uses a non-scaling stroke in the F-207 underlay-then-colour style, colour `#fff04d` from the ink palette. The ink layer already turns with the stage, so rotation and flips need no extra arithmetic. The cell clips anything outside it.

**Out of scope.** Registration across frames of reference, pan sync, a reference cursor, crosshairs, MPR, and prior studies.

## Problem and reproducer

With F-208 in place, open a folder holding an axial CT and a coronal reformat in one frame of reference, layout 1 by 2, the axial in cell 1 and the coronal in cell 2. Scrolling the axial does not mark its level on the coronal, and a second axial series in cell 2 does not follow cell 1. `/inspect` reports no Frame of Reference UID and no plane today, so the page has nothing to link with. Research section 10 rows 17 and 18 tag synchronized scroll, zoom and window and reference lines as SHOULD.

## Scope, API and compatibility

In scope:

- `crates/ocelli-viewport/src/link.rs`, the geometry. Default under Open question 1.
- `/inspect` gains `frameOfReferenceUid` and `planes`. Existing fields and their text are unchanged, so `inspect_json_reports_the_series_fields` and `spacing_keeps_row_then_column` stay green.
- `POST /link` in `serve.rs`, handled by a new `examples/folder-viewer/src/link.rs`.
- The page's link menu, the reference toggle, the request loop and the line drawing.

Library API, default placement:

```rust
// crates/ocelli-viewport/src/link.rs
use ocelli_core::{Index, Pt};
use ocelli_pixel::ImagePlane;

/// cornerstone3D 5.8.2 `areViewportsCoplanar`.
pub const COPLANAR_DOT: f64 = 0.9;
/// cornerstone3D 5.8.2 `EPSILON`, used by `isParallel` and `isPerpendicular`.
pub const PLANE_EPSILON: f64 = 1e-3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkError {
    /// Not coplanar, parallel, an empty list, or a non-finite value.
    Unavailable,
}

/// Both UIDs present and equal as strings. An empty UID links nothing.
pub fn same_frame_of_reference(a: &str, b: &str) -> bool;

/// Index into `stack` whose Image Position Patient is nearest the source's.
/// Refused when `abs(dot(source normal, shown normal)) <= COPLANAR_DOT`.
pub fn closest_coplanar(
    source: &ImagePlane,
    shown: &ImagePlane,
    stack: &[ImagePlane],
) -> Result<usize, LinkError>;

/// Where `source` cuts `target`, as two points in `target`'s index space.
/// Refused when the planes are parallel or an intersection is not finite.
pub fn reference_segment(
    source: &ImagePlane,
    target: &ImagePlane,
) -> Result<(Pt<Index>, Pt<Index>), LinkError>;
```

The crate stays `no_std` with `alloc`. Normals come from `ImagePlane::slice_normal`. Corners come from `index_to_world()`. The target index comes from `index_to_world().inverse()`, the pattern `placement::frame_shift` already uses.

Defaults: every link off and reference lines on, pending Open question 3. Compatibility: with one cell, or with every link off and reference lines off, the page reads exactly as after F-208.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Folder read | `/inspect`, `FrameGeometry::derive` per frame | One post per file, as today | `frameOfReferenceUid`, `planes` on each image record |
| Active cell changes image | `POST /link`, `closest_coplanar` | Scroll link on, same frame, coplanar | Target cells move to the nearest image |
| Active cell changes image or another cell becomes active | `POST /link`, `reference_segment` | Reference lines on, same frame, not parallel | One line per other cell, in page pixel units |
| W/L release, preset, Reset in the active cell | The page's window command | Window link on | Each target sends its own `/preview` with that command |
| Zoom in the active cell | The page's view state | Zoom link on | Each target at the same zoom over its own fit |
| More page reference-line card | text only | none | Says the local viewer uses `ocelli_viewport::link` for the line and `referenceLines` for a crosshair |

Units: the plane and the IPP distance are millimetres in the LPS frame, owned by the library. The `0.5` pixel-centre change is owned by the example handler. Zoom relative to fit is owned by the page.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Two axial series, same frame, scroll link on | Cell 1 steps to IPP z = 7.5 | Cell 2 shows its nearest image by IPP distance | Cell 2 stays when the index is already nearest |
| Axial and coronal, same frame | Cell 1 steps | Cell 2 does not move, `not-coplanar`. Its line moves to the new level | Cell 2 index unchanged |
| Two series, different frames | Cell 1 steps | No move, no line, `frame-of-reference` | Both cells unchanged |
| A target image whose `planes` entry is `null` | Cell 1 steps | No move, no line for that cell, `no-plane` | Other targets still move |
| A `/link` in flight | Two more steps | One more request with the latest state after the first returns | A response older than the latest ticket is dropped |
| Target cell gets a new series while a request is in flight | Response arrives | Dropped for that cell by its `loadToken` | The new series is not moved by an old answer |
| Window link on | Lung in cell 1, cell 2 is MR | Cell 2 keeps its window, the preset is refused there | Cell 1 reads `WW 1500  WL -600` |
| Zoom link on | Ctrl-wheel in cell 1 to 200% of fit | Cell 2 reads `Zoom 200%` | Pan of cell 2 unchanged |
| The process stops | `/link` fetch fails | `processGone` status, lines cleared | Each cell keeps its image |
| Any state | Reference lines off | Lines cleared, no line requested | Scroll link unaffected |

Invariant: a line is always drawn on the image its cell shows. The handler computes it against the plane at the returned `index` when the cell moves, and against `<shown>` when it does not.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| A link request | `scheduleLink()` | `linkTicket`, one global counter | Response applied when its ticket is the latest | A failure clears lines and moves nobody. An identical repeat returns the same answer and moves nobody. |
| A target's index | link response | that cell's `loadToken` at send time | `showIndex(vp, index)` | Stale token, dropped |
| A target's line | link response | same | `ref-lines` children replaced | `null` removes the old line |
| A synced window | the active cell's commit | none | Each target's `showIndex` | A target's refused preset leaves its own window |

The stateless process holds nothing between requests. Each `/link` carries every plane it needs.

## Numbered acceptance inventory

Hand-computed values. Axial planes use orientation `1\0\0\0\1\0`, Pixel Spacing `0.7\0.5`, 96 rows, 128 columns, the F-206 synthetic geometry. The coronal plane `C` uses position `0\20\30`, orientation `1\0\0\0\0\-1`, Pixel Spacing `2.5\0.5`, 13 rows, 128 columns. Its normal is `(1,0,0) × (0,0,-1) = (0,1,0)`. The sagittal plane `G` uses position `10\0\30`, orientation `0\1\0\0\0\-1`, Pixel Spacing `2.5\0.7`, 13 rows, 96 columns.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Source axial at z = 7.5, stack at z = 0, 3, 6, 9, 12. Distances 7.5, 4.5, 1.5, 1.5, 4.5. Index 2. | `link::closest_keeps_the_lower_index_on_a_tie` | `<=` in the reduce, which gives 3 | implementation |
| AC2 | Shown plane with column cosine `(0,0.8,0.6)`, normal `(0,-0.6,0.8)`, dot 0.8, refused. Column cosine `(0,0.96,0.28)`, normal `(0,-0.28,0.96)`, dot 0.96, accepted. | `link::coplanar_needs_more_than_0_9` | A threshold of 0.75, or a parallel-only test at `1 - 1e-3` | implementation |
| AC3 | Source axial and shown coronal, dot 0, refused | `link::orthogonal_series_do_not_scroll` | Nearest by distance alone | implementation |
| AC4 | Source axial at z = 5 cuts `C`. Corners `(0,0,5)`, `(63.5,0,5)`, `(0,66.5,5)`, `(63.5,66.5,5)`. Top-bottom vector `(0,-1,0)` is not perpendicular to `(0,1,0)`, so edges TL to BL and TR to BR. World `(0,20,5)` and `(63.5,20,5)`. Target index `(0,10)` and `(127,10)`, because `(0,0,-25)` dotted with `(0,0,-1)` is 25 and 25 / 2.5 = 10. | `link::axial_cuts_coronal_at_row_10` | Row and column spacing swapped, which gives row 50 | implementation |
| AC5 | Source `G` cuts `C`. Top-bottom vector `(0,0,1)` is perpendicular to `(0,1,0)`, so edges TL to TR and BL to BR. World `(10,20,30)` and `(10,20,0)`. Target index `(20,0)` and `(20,12)`. | `link::sagittal_cuts_coronal_at_column_20` | The first edge pair only, whose edges lie parallel to the target plane and give a non-finite point that must be refused, not drawn | implementation |
| AC6 | Axial cutting axial, refused as parallel | `link::parallel_planes_have_no_line` | A line at infinity | implementation |
| AC7 | `same_frame_of_reference("", "")` is false. Equal non-empty UIDs are true. | `link::an_empty_uid_links_nothing` | Treating two missing UIDs as one frame | implementation |
| AC8 | `/inspect` on synthetic slice 3 reports `"frameOfReferenceUid":"2.25.44000"` and `"planes":["0,0,7.5,1,0,0,0,1,0,0.7,0.5,96,128"]` | `inspect_reports_the_plane_token`, folder-viewer tests | A token from the raw DS text, or row and column spacing swapped | implementation |
| AC9 | A file with Imager Pixel Spacing only reports `"planes":[null]` | same test, `spacing_object(None, Some(..))` | A plane from Imager Pixel Spacing | implementation |
| AC10 | `POST /link` with source synthetic slice 2 and target `C` returns `"line":[0.5,10.5,127.5,10.5]` and `"index":null` with `"reason":"not-coplanar"` | `link_route_draws_the_axial_level` | The pixel-centre `0.5` applied twice, or not at all | implementation |
| AC11 | `POST /link`, different UIDs, returns `"index":null,"line":null,"reason":"frame-of-reference"` | `link_route_refuses_another_frame` | Linking across frames | implementation |
| AC12 | `POST /link` with one `-` plane in the target list returns `"reason":"no-plane"` and moves nothing | `link_route_refuses_a_missing_plane` | Skipping the gap and moving to a wrong image | implementation |
| AC13 | A malformed body, a non-unit cosine token, or `<shown>` out of range is `400` with `X-Ocelli-Error: link` | `link_route_refuses_a_malformed_body` | A `200` with zeros | implementation |
| AC14 | A token read back by the handler rebuilds a plane whose `index_to_world` agrees with `FrameGeometry::derive` within 1e-9 mm at the four corners | `a_plane_token_round_trips` | Fewer printed digits | implementation |
| AC15 | The page posts `/link`, has the three switches and the reference toggle, and never parses a token | `the_page_links_viewports` finds `fetch("/link"`, `id="link"`, `id="toggle-reference"`, `class="ref-lines"`, and finds no `.planes[` followed by `.split(` | Page arithmetic on plane numbers | implementation, plus review of the diff |
| AC16 | Two cells with the synthetic series, scroll link on. Scrolling cell 1 moves cell 2 to the same image. | Browser B1 | Index copied rather than position, which hides itself here, so B3 is also required | integrated / sprint close |
| AC17 | The operator's pseudo-PHI MR study, axial series 15 in cell 1 and sagittal series 16 in cell 2, which share one Frame of Reference UID, reference lines on. Cell 2 shows a yellow line, and stepping cell 1 by one image moves the line by the axial slice step projected on the sagittal plane, which the observer computes by hand from the two series' Image Position (Patient), Image Orientation (Patient) and Pixel Spacing and compares with the `line` in the `/link` reply. | Browser B2 | A crosshair from `referenceLines`, or a line that does not move with the source slice | integrated / sprint close, operator host only |
| AC18 | Window link on. Lung in cell 1 gives `WW 1500  WL -600` in both cells. Zoom link on, 200% in cell 1 gives `Zoom 200%` in cell 2. | Browser B3 | Window copied while dragging to every cell, or pan copied | integrated / sprint close |
| AC19 | Window link on. Cell 1 shows a monochrome series and cell 2 shows an image the viewer refuses to window, a colour image or a non-monochrome refusal. A W/L drag, a preset and Reset in cell 1 leave cell 2's picture, window and invert unchanged, raise no error, and cell 2's corner reads `Window not applicable`. Zoom link still reaches cell 2. Then cell 2 is made active and a preset there changes neither cell. | Browser B3 | A window copied onto a colour target, an error from the copy, or zoom sync dropped along with window sync | integrated / sprint close |

Browser procedure. Host: the operator's Mac, Chrome through the Playwright MCP tools. Start `OCELLI_PORT=0 cargo run --release -p ocelli-folder-viewer` and open the one URL it prints. Port 8765 is never used, because an operator file server holds it.

- B1. Synthetic study, layout 1 by 2, drag the series onto cell 2, Link, Scroll on. Three 40px wheel steps over cell 1. Both counters read the same image.
- B2. Folder: the operator's pseudo-PHI MR study at `~/Desktop/noeti_working/pseudo/manifest-1617826555824/Pseudo-PHI-DICOM-Data/9894340694`, opened through the Folder picker from where it lies. It stays outside git and is never copied into the repository, into `corpus/`, into a fixture or into a review file. Layout 1 by 2, series 15 (axial) in cell 1, series 16 (sagittal) in cell 2, reference lines on. Cell 2 shows the line, and stepping cell 1 by one image moves the line as AC17 states. The run note records the URL, the Chrome version, the two series numbers, the expected and observed line ends and a screenshot of cell 2 cropped to the image without the corner text. It records no name, ID, date or UID from that study.
- B3. As in AC18.

The Rust tests do not run the page. B1 to B3 are separate and are not claimed by a Rust test.

## Approach

1. `ocelli-viewport/src/link.rs` with the three functions and two constants, and `pub mod link`.
2. `preview.rs`: in `inspect_json`, read (0020,0052), build `MultiframeMetadata`, call `FrameGeometry::derive` for each frame up to the declared count, write each plane token or `null`.
3. `examples/folder-viewer/src/link.rs`: parse the body, rebuild each plane through `ImagePositionPatient::new`, `ImageOrientationPatient::new`, `PixelSpacing::new`, `ImageDimensions::new` and `ImagePlane::new`, call the library, add `PAGE_PIXEL_CENTRE`, write the JSON.
4. `serve.rs`: `("POST", "/link") => json_or_error(link_json(&request.body))`, with the `link` error code.
5. `page.html`: store `info.frameOfReferenceUid` and `info.planes` on each image, the menu and toggle, `scheduleLink`, the line layer, window and zoom sync.

Rejected: registering every viewport in `SyncScene` per request. Its `HeldPlane` needs one uniform spacing, and a folder of single-frame files often has none. Its UID rule is kept. Rejected: computing the line in the page from tokens. That is the geometry the allocation note forbids. Rejected: a new route posting files for geometry. `/inspect` already has every file once.

## Dependencies, write ownership and resources

Depends on F-208, which must merge first because it creates the per-viewport records, and on F-199, done. Sprint build order, decided in the S46 design round on 2026-11-05: F-217, F-216, F-210, F-211, F-208, then this story, then F-212, F-213, F-214 and F-215. F-119 is independent. `/inspect` here is reached through F-216's `POST /file`, whose body is byte-identical to `/inspect`, so the new fields appear in both without a second writer. The story row says TS, and this plan also writes Rust, because the page may not compute geometry.

Writes:

- `crates/ocelli-viewport/src/link.rs`, new
- `crates/ocelli-viewport/src/lib.rs`
- `crates/ocelli-viewport/tests/link.rs`, new
- `examples/folder-viewer/src/preview.rs`
- `examples/folder-viewer/src/link.rs`, new
- `examples/folder-viewer/src/lib.rs`
- `examples/folder-viewer/src/serve.rs`
- `examples/folder-viewer/src/page.html`
- `examples/folder-viewer/src/more.html`, the reference-line card's "Lives in" sentence
- `examples/folder-viewer/tests/preview.rs`
- `docs/lld/viewport.md`

`page.html` and `preview.rs` are shared with F-210, F-211, F-212, F-213 and F-216. Serial after F-208. No GPU.

## Verification and evidence reuse

Focused: `cargo test -p ocelli-viewport --test link`, `cargo test -p ocelli-folder-viewer`, `bin/ocelli.sh check ocelli-viewport`, `bin/ocelli.sh check ocelli-folder-viewer`, and the `nostd` gate, because `ocelli-viewport` must stay `no_std`. `ocelli-wasm` does not call the new module, so no wasm size change is expected. `bin/ocelli.sh wasm` confirms it at sprint verification. Mutation controls: `COPLANAR_DOT` to 0.75 turns AC2 red, `<` to `<=` turns AC1 red, `PAGE_PIXEL_CENTRE` to 0 turns AC10 red, and always using the first edge pair turns AC5 red.

The rules are transcribed from cornerstone3D 5.8.2. These tests show internal agreement with that transcription. They are not a differential measurement against the oracle, which does not render this page.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. The page receives grey bytes and line endpoints from the local process.
- Render-loop allocation: none in the page beyond one line node per cell, reused. The handler allocates per request in the local process, outside any render loop.
- unsafe: none
- Tier A (WebGPU): n/a. This page does not open a device.
- Tier B (WebGL2): n/a
- Tier C (CPU): full. The geometry is `f64` arithmetic in the local process, and the grey images are F-206's CPU LUT.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 to AC7, hand-computed values above, citing PS3.3 C.7.6.2.1.1 and C.7.4.1.1.1 | `crates/ocelli-viewport/tests/link.rs` |
| fixture | AC8, AC9, AC14, the token from the synthetic slice and a projection file | `examples/folder-viewer/tests/preview.rs` |
| contract | AC10 to AC13, the `/link` route | `examples/folder-viewer/tests/preview.rs` |
| contract | AC15, the page controls and the absence of token parsing | `examples/folder-viewer/tests/preview.rs` |
| observation | AC16 to AC18 | B1 to B3 above |

## Parity surface covered

None measured. Appendix B counts synchronizers and the reference lines tool inside its tool total and has no row for either.

## Deviations

none. The HLD does not specify these behaviours, so the departures from cornerstone3D in the table above are local example choices, not HLD deviations.

## LLD impact

`docs/lld/viewport.md` gains a "Linked viewports" section naming `same_frame_of_reference`, `closest_coplanar`, `reference_segment`, the two constants and their oracle source.

## User-facing documentation impact

- `README.md`, "Start the local series viewer". Step 5's toolbar groups gain Link and Reference lines. A new step after the viewport step says what Link does (Scroll, Window and Zoom switches, off at load, one set over every visible cell) and that a reference line shows where another viewport's slice cuts this one when both share a Frame of Reference UID.
- `examples/folder-viewer/src/more.html`, `#reference`, the "Lives in" sentence names `ocelli_viewport::link` and says the image page draws reference lines with it. `#series`, the status line and the closing tiers paragraph say linked scroll, window and zoom run on the image page through `ocelli_viewport::link`. They keep prior studies as not built and keep the separate `npm run dev` example's "unavailable" wording, since that example does not change.
- `examples/folder-viewer/src/use.html`, chapter 10 "Geometry and spacing", gains a paragraph naming `same_frame_of_reference`, `closest_coplanar` and `reference_segment` with their refusal reasons, because they are new library exports.

The `#series` card and `use.html` join the write set above for these edits.

## Open questions

1. **Where the geometry lives.** Affects AC1 to AC7 and the write set. Options: (a) `ocelli-viewport/src/link.rs`, a library module that the wasm path can call later, with an LLD entry, (b) a private module in `examples/folder-viewer`, which keeps the core crates untouched. Recommended (a), because the arithmetic must exist once and the allocation note says "from the library". Owner: operator. Deferral blocks implementation.
2. **A non-parallel series to observe a reference line.** Affects AC17 and B2. The synthetic study is twelve axial slices, and the corpus manifest lists no study with two orientations in one frame. Options: (a) add a coronal synthetic series through the same phantom, Frame of Reference UID `2.25.44000`, served at `GET /synthetic-coronal` as `{"slices":8}` and `GET /synthetic-coronal/<n>`, so `GET /synthetic` keeps `{"slices":12}` and the twelve axial slices are untouched, and the status then reads "2 series.", (b) no synthetic change, with AC17 left open until the operator supplies a folder. Recommended (a). Owner: operator.
3. **Link defaults and membership.** Affects AC16 and AC18. Options: (a) one set over every visible cell, Scroll, Window and Zoom off at load, Reference lines on, (b) the same with Scroll on, (c) a link checkbox on each cell. Recommended (a). Owner: operator.
4. **Pan sync.** cornerstone3D copies pan with zoom unless told not to. The story names window and zoom. Options: (a) zoom only, (b) a fourth switch, Pan, off by default. Recommended (a). Owner: operator.
5. **Window sync during a drag.** Options: (a) targets update on release, the active cell stays live, (b) every target updates live, which is nine preview requests per drag step at 3 by 3. Recommended (a). Owner: operator.

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead.

1. Answered: recommended (a), `ocelli-viewport/src/link.rs`, a library module with an LLD entry.
2. Answered by the operator, neither option as written. No coronal synthetic series is added. The synthetic study stays one CT series of 12 slices and the operator protocol for it is unchanged. Browser evidence for reference lines (AC17, B2) uses the operator's pseudo-PHI MR study at `~/Desktop/noeti_working/pseudo/manifest-1617826555824/Pseudo-PHI-DICOM-Data/9894340694`, where axial series 15 and sagittal series 16 share a frame of reference. It stays outside git and is never copied into the repository or fixtures. Rust tests use synthetic in-test planes only, which AC1 to AC14 already do with the planes `C` and `G`. `synthetic.rs` leaves the write set.
3. Answered: recommended (a), one link set over every visible cell, Scroll, Window and Zoom off at load, Reference lines on.
4. Answered: recommended (a), zoom only. Pan is not synced.
5. Answered: recommended (a), linked targets update on release and the active cell stays live.

Constraint from the same round: browser checks use `OCELLI_PORT=0` and the printed URL, never port 8765. No question is left open.

### Lead amendments before implementation, against F-208's tree

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, before any F-209 code is written, against F-208's tree in `ocelli-wt-f-208` at `5c8b28f7` (review pass 7 remediation committed, no pass 8 report). F-208 changes no file under `crates/` or `packages/`, so library names hold for `sprint/s46`. Page and handler line numbers will move when F-208 merges, so every rule below names a function, not a line. Where this section and the text above differ, this section holds. Each DICOM claim carries its PS3.3 section, and the implementer checks it against the current edition, not against the comment beside the code.

#### Geometry

- **`ocelli_viewport::link` is new in this story, and `ocelli-viewport` owns it.** `crates/ocelli-viewport/src/link.rs` does not exist, `lib.rs` declares no `link` module, and nothing in `crates/` or `packages/core/src` intersects two planes or chooses a nearest image. The operator's answer 1 put it in `ocelli-viewport`. The crate stays `no_std` with `alloc` and needs no new dependency, since it already depends on `glam`, `ocelli-core`, `ocelli-pixel` and `ocelli-volume`. Nothing in the folder viewer, the page or `packages/core` computes a normal, a dot product, a distance or an intersection.
- **One frame-of-reference rule, shared with `SyncScene`.** `same_frame_of_reference` takes `Option<&FrameOfReferenceUid>` twice and is true only when both are `Some` and equal. `FrameOfReferenceUid` is the existing `ocelli_volume` type. `SyncScene::set_point` is changed to call it instead of its own `!=` and `None` handling, so the rule lives in one function. The `sync.rs` tests stay unchanged and green. `crates/ocelli-viewport/src/sync.rs` joins the write set. PS3.3 C.7.4.1.1.1, Frame of Reference UID: the implementer checks whether the standard scopes "spatially related" to images in a series or across series before the LLD repeats the plan's sentence at line 52.
- **`LinkError` names its refusals.** It becomes `NotCoplanar`, `Parallel` and `Degenerate`. `Degenerate` covers an empty stack, a non-finite intersection or index, and a target with a zero spacing on a singleton axis, because `index_to_world` puts a unit extension on that axis and an index read through it is in millimetres, not pixels. The handler writes `not-coplanar`, `parallel` and the new reason `degenerate`, and none of the three moves a cell or draws a line.
- **Fixtures are hand-computed and cite their section.** AC1 to AC7 keep the plan's values, which the groundwork re-derived from PS3.3 C.7.6.2.1.1 and the lead re-derived again for AC4 and AC5 (axial at z = 5 cuts `C` at target index `(0,10)` and `(127,10)`, `G` cuts `C` at `(20,0)` and `(20,12)`). Each test comment cites PS3.3 C.7.6.2.1.1 for the matrix and C.7.6.2.1.1's normal as `X cross Y` only after the implementer has confirmed that section defines or implies the normal. The Pixel Spacing value order (row spacing first) cites PS3.3 10.7.1.3, which F-208 pass 1 used, and the implementer checks it and records that one citation in `link.rs`, the handler and the new tests. `measure.rs` and `tests/preview.rs` cite C.7.6.2 and C.7.6.2.1.1 for the same fact today and are left as they are.
- **AC23, new.** A target with 1 row, row spacing 0 and 128 columns is refused by `reference_segment` as `Degenerate`. Test `link::a_singleton_axis_target_has_no_line`. Wrong implementation rejected: a line read through the unit extension.

#### The plane token and the `/link` body

- **Geometry reaches the page through `POST /file`, not `/inspect`.** The page never calls `/inspect`. A folder read posts each file once to `POST /file`, whose JSON is `inspect_json`'s, and the synthetic study goes through the same `readFolder`. The two new fields are added in `inspect_json`, so both routes carry them. Lines 157 and 273 and step 2 of the Approach read accordingly. The `/inspect` route itself is unchanged.
- **Nothing new is stored on an image record.** A record is `{ file, frame, info, reported }` and `info` is that file's one reply. The image's token is `image.info.planes[image.frame]` and its UID is `image.info.frameOfReferenceUid`. Step 5's "store `info.frameOfReferenceUid` and `info.planes`" is struck.
- **`planes` is capped at the page's frame list.** `inspect_json` writes one entry per frame up to the declared count, and no more than a Rust `FRAME_LIST_CAP` with the page's value. The page test pins that the two constants are equal.
- **The token is written with `{:e}`, not `{}`.** Rust's `{}` never uses an exponent, and the lead measured 327 characters for `-2.2250738585072014e-308` against at most 24 for `{:e}` over the extreme `f64` values tried, both reading back to the same bits. A finite position or spacing of any size is accepted by the `ocelli-pixel` constructors, so `{}` would leave the body without a usable bound. The two `u32` fields keep `{}`. The implementer measures and records the longest field text in a test.
- **AC8, restated.** `POST /file` (and `/inspect`) on synthetic slice 3 reports `"frameOfReferenceUid":"2.25.44000"` and `"planes":["0e0,0e0,7.5e0,1e0,0e0,0e0,0e0,1e0,0e0,7e-1,5e-1,96,128"]`. The lead computed that text from the plan's values with `{:e}`, and the test confirms the cosines `ImageOrientationPatient::row()` and `column()` return are exactly those. Wrong implementation rejected: a token from the raw DS text, row and column spacing swapped, or `{}` text.
- **A UID the process cannot send is reported empty.** `frameOfReferenceUid` is the trimmed text of (0020,0052) when every character is a digit or `.`, and `""` otherwise, so a malformed UID links nothing rather than breaking the space-separated body. The UID character set is PS3.5 9.1, which the implementer checks.
- **Each listed image carries its own UID.** A series can mix frames of reference, and a target line with one UID would compare every image against the shown one. The body is restated:

```text
source <uid> <plane>
target <cell> <shown> <scroll> <uid> <plane> <uid> <plane> ...
```

  `<cell>` is `vp.cell`, zero-based, 0 to 8, each at most once. `<uid>` is the text or `-`, `<plane>` the token or `-`. The handler links a target only when the source UID and every listed image's UID are the same frame by `same_frame_of_reference`, and otherwise answers `frame-of-reference` for that target. Response `"cell"` values are the same zero-based numbers.
- **The page lists shown cells only, and refuses a series it cannot list.** The body names `shownViewports()` other than the active cell, with a series and an image. A target whose series has more images than `FRAME_LIST_CAP` is not listed, its line is cleared, and it does not scroll. README says so in the limits sentence.
- **`/link` has its own declared-length cap.** `LINK_BODY_CAP` sits beside `MEASURE_BODY_CAP` and is refused from the declared length before the body is read, with `X-Ocelli-Error: link`. Its value is a `const` expression from named terms: 8 targets (`MAX_SIDE` 3 squared, less the source), `FRAME_LIST_CAP` images each, the longest UID (64 characters for VR UI in PS3.5 Table 6.2-1, which the implementer checks), the longest token from the measured field length, 13 fields and 12 commas, separators and the fixed words. The lead states no total. A test builds the largest admissible body and shows it is accepted, and one byte more is refused.
- **Refusals do not go through `json_or_error`.** `json_or_error` maps a public `PreviewError` through `error_reply`, and `PreviewError` has no `link` code. The handler `link_json` returns `Result<String, LinkRefused>` with a crate-private `LinkRefused`, and `serve.rs`'s new `link_route` writes `text_reply(400, Some("link"), ...)` for it, as `value_route` and `measure_route` do. `PreviewError` gains no variant. Approach step 4 reads accordingly.
- **AC13, restated.** A malformed body, a non-unit cosine token, `<shown>` out of range, a `<cell>` above 8 or repeated, and a declared length over `LINK_BODY_CAP` are each `400` with `X-Ocelli-Error: link`. Test `link_route_refuses_a_malformed_body`. Wrong implementation rejected: a `200` with zeros, or a cap read after the body.
- **AC10, restated for the body.** `POST /link` with source synthetic slice 2 and target `C` in cell 1, both with UID `2.25.44000`, returns `{"targets":[{"cell":1,"index":null,"line":[0.5,10.5,127.5,10.5],"reason":"not-coplanar"}]}`.

#### Pixel units

- **The half-pixel shift is the one F-211 already owns, not a second constant.** `measure.rs` says `index = page - 0.5` "here and nowhere else", as an inline literal. The shift moves onto the page point type in `value.rs`: one private `PAGE_PIXEL_CENTRE`, `ImagePoint::to_index(self) -> Pt<Index>` and `ImagePoint::from_index(Pt<Index>) -> ImagePoint`. `measure_frame` calls `to_index` instead of its literal, the handler calls `from_index` on both line ends, and the "here and nowhere else" comment moves with the constant. `measure.rs` joins the write set. The plan's `PAGE_PIXEL_CENTRE` in the handler is struck.
- **AC21, new.** `PAGE_PIXEL_CENTRE` set to 0 turns AC10 red and turns F-211's measure fixtures red, in one edit. Wrong implementation rejected: a second `0.5` anywhere in the folder viewer's Rust.

#### Where the line is drawn

- **A sibling layer, not a group inside ink.** `renderInk`, `showIndex`'s colour path and `resetViewports` each call `ink.replaceChildren()`, so a `<g>` inside `svg.ink` is erased by every mark change and every repaint with marks. `#cell-template` gains `<svg class="ref-lines" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="none">` inside `.stage`, between `canvas.view` and `svg.ink`, so marks draw over the line. Its CSS is that of `.ink` plus `pointer-events: none`. Its `viewBox` and its width and height are set in the same statements that set them on ink, in `drawGrey` and `layoutViewport`, so it turns and flips with the stage and no arithmetic is added. One `<line>` under and one over per cell, made once and reused, styled with `stroke(node, color, width)`, the underlay `#000000` width 4 at opacity 0.8, the colour `INKS[2]`.
- **Only line code writes the layer.** `renderInk` and every `ink.replaceChildren()` leave it alone. The line is cleared when the cell's image key changes in `showIndex`, when its series changes in `loadSeries`, when the cell becomes active, when it is hidden by `setLayout`, in `resetViewports`, in `processGone`, on a failed `/link`, and when Reference lines goes off. A cell in tile mode or showing a fault has its stage hidden, so its line is hidden with it and is valid again when the stage shows the same image.
- **AC15, restated.** `the_page_links_viewports` finds `fetch("/link"`, `id="link"`, `id="toggle-reference"` and `class="ref-lines"`, finds `class="ref-lines"` before `class="ink"` inside `#cell-template`, finds no `.ref-lines` inside `renderInk`'s body, finds no `.planes[` followed by `.split(`, and checks the page's `FRAME_LIST_CAP` against the Rust one. Mutation controls: the layer moved inside ink, and a `split` on a token, each turn it red.
- **AC20, new.** Browser B2, extended. A Length mark on cell 1's image, Window link on, a W/L release in cell 0. Cell 1 repaints with the copied window and its mark, and its reference line is still drawn at the same ends. Wrong implementation rejected: the line inside ink, erased by the repaint.

#### Cross-cell state

- **F-208's invariant is amended for linked state only.** F-208 says "No function reads another cell's state, except `positions`". This story adds the link functions and nothing else: the body builder reads every shown cell's `cell`, `seriesKey`, `viewIndex`, `loadToken` and its images' `info.planes` and `info.frameOfReferenceUid`, the reply applier writes a target's index through `showIndex` and its `ref-lines` layer, and the window and zoom copies write a target's window command and scale fields, each only while its switch is on. Every other function still reads and writes only its own cell, and reviewers apply F-208's rule to everything else.
- **The request loop is page-global, on F-210's pattern.** `linkState = { busy, want, ticket }` sits with the other page globals, not in `blankViewport`, as `valueState` does for `/value`. Line 172's reference to the live W/L draft is struck, since that draft is now per viewport.
- **A reply applies to a target only if nothing moved under it.** It applies when its ticket is the latest and the cell's `loadToken` and `viewIndex` equal those sent. Otherwise that entry is dropped and the next request redraws it.
- **The send hook compares the image key.** `showIndex` is called by every redraw path, so it schedules `/link` only when `noteKey(image)` differs from `previousKey` and `vp.cell === activeCell`. A reply moves only non-active cells, so it never echoes. `setActive` schedules one after `activeCell` changes. `setLayout` schedules one inside its own body, so the page-wide `setLayout(` count AC13 of F-208 pins stays 3. `loadSeries` schedules one for any shown cell, so a layout grow and `fillCells` give the new cells their line through the one coalesced request. Nothing is sent with one shown cell, or when Scroll is off and Reference lines is off.
- **A fetch failure.** `processGone()` runs, every line is cleared and no cell moves.

#### Window sync

- **A copy sets the target record and redraws it.** `/preview` takes the window as headers from `view.overrideWindow`, `view.activePreset` and `view.invert`, so a copy sets the target's `overrideWindow` (a copy of the object) or `activePreset`, clears its `draftWindow`, and calls `showIndex(target, target.viewIndex)`. It never calls `applyPreset` or `resetView` on a target, because both clear more than a window. Reset copies the file window, `overrideWindow` and `activePreset` both null. Invert, rotation, flips and pan are never copied.
- **A target that cannot be windowed is sent nothing and keeps its window.** A target whose current image is not monochrome is skipped for every window copy. A preset is skipped for a target whose current image's modality is not CT, read from `image.info.modality` before anything is touched, because the presets are in Hounsfield units and `showIndex` would drop the preset and leave the target on its file window. A numeric W/L release still reaches an MR target, as `voiRange` does in the oracle.
- **A source that cannot be windowed sends nothing.** The copy function reads the source's current image and returns when it is not monochrome. The preset buttons are disabled when the active image is not monochrome as well as when it is not CT, the availability cache keys on both, and the note reads "This image is not monochrome, so it has no window to set." for that case.
- **`Window not applicable` is a corner state, on every page.** `paintOverlays` writes `Window not applicable` in place of the `WW` and `WL` line for a non-monochrome image, at every layout and with any link setting, because the file window printed there today describes a picture that is not shown. The `Zoom` line is kept. Line 267's compatibility sentence reads: with one cell, or with every link off and reference lines off, the page behaves as after F-208, except for the Link and Reference lines buttons and this corner text on a non-monochrome image. F-208's AC12 is read the same way.
- **A row click is a load, and copies nothing.** A click on the open series row resets that cell's window, preset and fit through `loadSeries`, as F-208's D2 ruling keeps. It sends `/link` and copies no window and no zoom to any target. A drop does the same.
- **Row 292, restated.** Window link on. Lung in cell 0, which shows CT, while cell 1 shows MR. Cell 0 reads `WW 1500  WL -600`. Cell 1 is sent nothing, makes no `/preview` request, and keeps its window and its corner text.
- **AC18, restated.** Window link on. Lung in cell 0 gives `WW 1500  WL -600` in cells 0 and 1, both CT. A row click on cell 0's series leaves cell 1 at `WW 1500  WL -600`. Zoom link on, 200% in cell 0 by Ctrl-wheel gives `Zoom 200%` in cell 1, with cell 1's pan unchanged. Wrong implementation rejected: a window copied during a drag, pan copied, or a row click copied.
- **AC19, restated.** Layout 3 by 1. Window and Zoom link on. Cell 0 and cell 2 show a monochrome series, cell 1 a non-monochrome image. A W/L drag, a preset and Reset in cell 0 reach cell 2 and leave cell 1's picture, window and invert unchanged, raise no error, and cell 1's corner reads `Window not applicable`. A Ctrl-wheel in cell 0 to 200% gives `Zoom 200%` in cell 2. Cell 1 is then made active, its preset buttons are disabled, and no window reaches cell 0 or cell 2. Wrong implementation rejected: a window copied onto a colour target, an error from the copy, zoom sync dropped with window sync, or a preset applied from a colour source.

#### Zoom sync

- **Through the record, outside `pointerPlaces`.** One function copies the active cell's zoom to each shown target: when the source is in fit mode the target's `fitMode` is set, and otherwise the target gets `fitMode = false` and `scale = fitScale(target) * source.scale / fitScale(source)` with its pan kept, then `layoutViewport(target)`. It is called at each zoom entry point after the source has laid out: Ctrl-wheel, `zoomCentre` for `=` and `-`, `fitView`, `resetView`, and in a middle or Zoom-tool drag after the source's `pointReadout(event)`, outside the `try` that sets `pointerPlaces`, so a target's own `leavePointerFor` runs. A pan drag changes no scale and copies nothing.
- **A target with no single view keeps its record.** A target in tile mode or showing a fault has its stage hidden and is skipped, so no hidden scale surprises it later. This supersedes "Zoom sync is independent and still applies" in the colour amendment for a target whose stage is hidden. AC19 above observes zoom on cell 2.
- **AC22, new.** Browser B3, extended. The pointer rests over cell 1 with the readout showing, cell 0 is active, Zoom link on, `=`. Cell 1 zooms and its readout, cross and magnifier hide. A pointer move shows them again. Wrong implementation rejected: the target laid out inside `pointerPlaces`, which leaves a readout over a moved picture. `tests/browser/readout.mjs` stays unchanged and green under `gate viewer-browser`.

#### Tile mode, layout and controls

- **Tile mode.** A source in tile mode sends the plane of its `viewIndex` image. A target in tile mode is scrolled and window-copied as any target, its tiles repaint through `showIndex`, it is skipped by zoom, and its line is hidden with its stage.
- **The menu does not resize the grid.** `#link-menu` is `position: fixed`, styled and closed like `#layout-picker` (Escape and an outside click), so opening it does not resize `#wrap` and `relayout` does not clear the readout. Hover sentences for `link`, the three switches and `toggle-reference` go in `SENTENCE`.
- **No key is bound.** Bare `1` to `9` stay free for F-213 under F-208's operator answer 1, and nothing in `onKey` uses a token F-208's AC13 pins.
- **Cell numbers and layouts.** Prose and tests use `vp.cell`, zero-based. The picker label is columns by rows, so two cells side by side is 2 by 1. B1 and B2 read "layout 2 by 1, cell 0 and cell 1". AC16 and AC17 read cell 0 for the plan's cell 1 and cell 1 for its cell 2. Rows 286 to 293 read the same way.
- **Row 295, restated.** Reference lines off: every line is cleared, lines in replies are not drawn, and `/link` is sent only while Scroll is on.

#### Stale names in the text above

- Line 75 and line 37: `packages/core/src/crosshair.ts` is F-088's library code, per its header and its one commit `c12948d3`. F-199 is the example story that consumed it. Read "the F-088 library reference line".
- Line 187: `imageToClient(vp, point)` and `clientToImage(vp, x, y)` are per viewport. The pixel-centre rule is unchanged.
- Line 201 and line 277: `fitScale(vp)` and `layoutViewport(vp)` are the names, and the zoom label is computed in `layoutViewport`.
- Lines 203 and 276: "refused by F-207's preset availability" and "its own `/preview` with that command" are replaced by the window sync rules above. `syncPresetAvailability` acts on the active cell only.
- Line 205: the string `Window not applicable` does not exist yet and is added by this story in `paintOverlays`.
- Lines 290, 291 and 303: `loadToken` moves on `loadSeries`, `resetViewports` and a hide in `setLayout`, not on `showIndex`, which is why the reply check above also compares `viewIndex`.
- Line 415: `more.html` `#reference`'s "Lives in" sentence, and in `#series` both the "Lives in" sentence and the closing tiers paragraph, whose "Its viewports do not follow each other" goes. The `npm run dev` example's "unavailable" wording stays.

#### README

- **No step is renumbered.** F-208 inserted the series-drop step, so the toolbar groups are step 6 today, not step 5, and F-212 to F-215 already cite step numbers that have moved. Link and Reference lines join step 6's layout group. What Link does (Scroll, Window and Zoom, off at load, one set over every visible viewport, a window copied on release, a preset not sent to a non-CT viewport, nothing sent to a non-monochrome one, a row click not copied) and what a reference line shows go at the end of step 4, the viewport step, as sentences, not as a new numbered step. Later plans name steps by their opening words.

#### Write set and verification, as amended

- Added: `crates/ocelli-viewport/src/sync.rs`, `examples/folder-viewer/src/value.rs`, `examples/folder-viewer/src/measure.rs`, `examples/folder-viewer/src/use.html` and `README.md` (both already named in the documentation section).
- Mutation controls, in addition to the plan's: `same_frame_of_reference` returning true for two `None` turns AC7 and a `sync.rs` test red, `ref-lines` moved inside ink turns AC15 red, `{:e}` changed to `{}` turns AC8 red, and `LINK_BODY_CAP` read after the body turns AC13 red.
- Focused checks add `cargo test -p ocelli-viewport --test sync` and `bin/ocelli.sh gate viewer-browser`.

## Lead cross-reference, F-239 colour support, 2026-11-08

F-239 item14 and AC28 supersede AC19's historical colour-refusal example.
Supported RGB is drawable and has its own window and invert. A window links
only between images of the same kind, monochrome to monochrome or colour to
colour. Therefore a monochrome drag, preset or Reset leaves a colour target's
picture, window and invert unchanged, while linked zoom still reaches that
visible target. A colour drag can reach another colour target and leaves
monochrome targets unchanged. Modality presets remain unavailable for colour.
The `Window not applicable` corner is now for an image that cannot be drawn,
whose hidden stage is also skipped by zoom linking.

The integrated operator check uses these same-kind and mixed-kind cases.
This records the already approved F-239 contract and does not claim the
operator has performed that check.
