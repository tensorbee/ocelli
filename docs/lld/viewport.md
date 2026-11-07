# Viewport

**F-IDs that contributed:** F-045, F-046, F-047, F-048, F-049, F-050, F-052, F-053, F-101, F-X003, F-X022, F-054, F-055, F-X004, F-209, F-224
**Last updated:** 2026-11-07

`View` in `crates/ocelli-viewport` is the stack camera. It does not draw.
`RenderGraph::set_view` uploads the CSS matrix `View::camera_matrix` returns
plus the device pixel ratio, twenty floats in one 80-byte uniform.
`Presentation` in the same crate holds the LUT chain and the
colormap. It does not draw either. `apply_placement` stores the
displayed spacing and the measured index shift on that view. It does
not draw.

## The types

| Type | What it is |
|------|-----------|
| `View` | Pan, zoom, rotation, the two flips, column and row millimetres, and the column and row index shift |
| `Zoom` | A scale about the fitted centre. Not a bare `f64` beside `Radians` |
| `Radians` | A clockwise angle on the y-down canvas |
| `ViewError::PanZ` | `Pt<Canvas>.z` was not zero. The stored pan is unchanged |

## The mapping

Image space origin is the top-left corner of texel `(0, 0)`. The centre is
`(columns / 2, rows / 2)`. A point moves in this order.

1. Spacing scales the offset from the centre. Column millimetres scale x.
   Row millimetres scale y. Zero is kept and is not replaced by 1.
2. Flip reflects about the centre.
3. Rotation is clockwise on the y-down plane. A quarter turn sends
   `(dx, dy)` to `(-dy, dx)`.
4. Scale is `fit_scale * zoom`.
5. Translation is the letterbox anchor, then the pan, then the index shift.
   The shift is axis-aligned in canvas pixels. It is
   `shift_col * column_mm * scale` and `shift_row * row_mm * scale`.

`fit` sets pan to zero and zoom to 1. It keeps rotation, flip, spacing and
the index shift. The scale is the largest one that places the axis-aligned
bounds of the transformed image inside the canvas. The leftover margin is
split equally on that axis. Before the first `fit`, the anchor is the image
centre and `fit_scale` is 1, so a default view at spacing 1 is the identity.

`camera_matrix` is the inverse of that map, column-major `f32`. The fragment
shader divides the device position by the device pixel ratio first, then
multiplies `(css_x, css_y, 0, 1)` by it and truncates with `i32` toward
zero. A zero spacing on one axis keeps the other axis and that axis's shift.
That axis is scaled before the squared length. A map with no extent writes the image centre. A coefficient that does not
narrow to a finite `f32` writes that same centre. A determinant that
overflows is inverted from the basis scaled by its largest entry. No path
writes a non-finite value. Narrowing `f64` to `f32` is ties-to-even.
It is not a Rust `as` cast.

## Canvas, index and world

`canvas_to_index` and `index_to_canvas` use the same f64 composition.
Image x and y are the index plus one half. DICOM PS3.3 C.7.6.2.1.1 puts
integer column `i` and row `j` on the pixel centre, and image space stores
the corner of texel `(0, 0)`. Index `(0, 0, 0)` stays Image Position Patient.
The half texel is not added in millimetres.

`canvas_to_world` reads rows and columns from the `ImagePlane` and then
applies `ImagePlane::index_to_world`. `world_to_canvas` is the inverse.
A display map that is not invertible returns `CoordinateError::Singular`.
The rank-one fallback inside `camera_matrix` is not a measurement.

`Pt<Canvas>` stays in CSS pixels. A device coordinate divides by the same
dpr as `device_extent` once, through `css_from_device`, before this mapping.
`View` does not store device pixels. The GPU camera uniform stores that CSS
map and the same ratio. The fragment shader divides a device-pixel position
by the ratio before the CSS multiply. The 96-byte state copy below keeps a
64-byte camera matrix and does not grow to match the GPU uniform.

## Commands and the ring

`StackViewport::apply_commands` parses the whole little-endian buffer before
it writes. The envelope is `[u16 kind][u16 viewport_id][u32 payload_len]`
and then the payload. Kind 0 is invalid. Ids 1 to 11 are Fit, Pan, Zoom,
Rotate, FlipHorizontal, FlipVertical, SetWindow, ApplyPreset, SetInvert,
SetColormap and Scroll. A structural or semantic failure, including
`SetWindow` on a VOI sequence, leaves the view, the presentation and the
scroll unchanged. The limit is 64 commands and 4096 bytes.

`write_state` copies 96 bytes. Camera is at 0, centre at 64, width at 68,
slice at 72, count at 76, flags at 80, and three zero words at 84. Flag bit
3, value 8, means refining. `EventRing` keeps 256 events. Kind 6 is
Presented. Its payload is generation, drawn width, drawn height and flags.
The drawn extent is the reduced grid during interaction and the full device
on settle. A full ring drops the new event. `copy_pending` writes a 16-byte header whose
first word is the pending count, then the events in order, and advances.
Cache kinds 1 to 5 and 7 are not produced here.

## What the GPU stores

Registration writes an 80-byte `DeviceCamera` at group 0 binding 2. The
first 64 bytes are the identity CSS matrix. The next four floats are the
scale. `.x` is the device pixel ratio `1.0` and the rest is padding.
`set_view` takes the CSS matrix and a ratio, replaces those 80 bytes, and
sets `Dirty.camera`. It does not write the VOI uniform, the colour uniform,
the stored texture or a brick. `uploads`, `lut_uploads` and `brick_uploads`
stay put. The shader divides the fragment by the stored ratio before the
CSS multiply. A fragment outside `[0, columns)` by `[0, rows)` writes
`(0, 0, 0, 1)` and does not run `lut_chain`. A negative coordinate is
rejected before `i32` truncation, so it does not select texel 0.

The vertex shader stays the full-target triangle. The pipeline is compiled
at graph creation. There is no mid-frame compile. Device-loss recovery drops
the uniform and keeps the CSS matrix and the ratio. `rebind` writes them
into the new 80-byte buffer. The viewport state readback stays 96 bytes.
Its camera field stays the 64-byte CSS matrix at offset 0.

## Tiers

Tier A and tier B sample through the same fragment module. `RenderGraph::new`
returns `TierCUnavailable` and does not draw on CPU. `CpuRaster` draws tier C
with `View::canvas_to_index` and `LutChain`. Grey and Stops use the existing
preset tables. Palette and RGB use `ColorTransform` when the decoder reports
a supported space. ICC and any other unsupported space report unavailable.
Interactive and idle cost are unmeasured. No representative GPU-less host
was available.

## Presentation

`Presentation` holds a `LutChain` and a `Colormap`. A preset or `set_window`
rebuilds stage 2 through `VoiTransform::new` and keeps the modality transform,
the presentation evidence and the output range. `soft_tissue` is centre 40
and width 400, `LINEAR`. `lung` is centre -600 and width 1500. `bone` is
centre 300 and width 1500. Both of those are `LINEAR` as well. None of the
three adds a formula.

A preset on a modality LUT sequence returns `PresentationError::Sequence`.
A manual window keeps that modality sequence, because the rebuild clones it.
A preset or `set_window` on a VOI LUT sequence returns the same error before
a new window is built. The chain, photometric interpretation, evidence and
colormap stay as they were. `set_invert`
stores an explicit `PresentationLutEvidence::Shape` and rebuilds the chain.
An explicit shape is not combined with `MONOCHROME1`. `set_colormap` stores
`Grey` or `Stops`. A window change does not change it.

One match in `ocelli-render`, `colour_lut_for`, maps `Colormap` to `PresetId`
and calls `ColourLut::preset`. This crate does not depend on `ocelli-render`
and does not call `set_voi` or `upload_stored`.

Tier A and tier B consume the rebuilt chain through the existing VOI uniform.
Tier C runs the same `LutChain` inside `CpuRaster`. A colormap indexes the
same preset bytes through `map_preset_into`. The display value is not clamped
before that index. ICC stays unavailable.

## Scroll

`StackSequence` holds `FrameKey` values in the order the caller stored them.
It does not read image position, instance number or slice location.
`scroll_by` adds a signed delta and clamps to the list. It does not wrap.
An empty sequence has no index. `samples_for_upload` calls `ImageCache::get`
for the selected key only, with the `FrameSource` of the bytes and metadata
the caller now holds. A hit returns the frame's own samples. A miss is
`NotResident` and returns no neighbour bytes. A frame held under the key from
other bytes or other metadata is `Stale` and has been removed by the lookup.
An empty sequence is `Empty`.

Prefetch radius is one. `prefetch_keys` is the index minus one and the index
plus one, when those indices exist. It does not decode and it does not
upload. `ImageCache::contains` is the probe that does not refresh a
neighbour. A prefetch insert does not set `Dirty.slice`. This module does
not edit the render graph.

Tier A and tier B upload the selected samples through the existing stored
texture path. This module does not draw. Tier C selects from the host
cache. `CpuRaster` is the draw. This module still does not call it.

## Placement

`frame_shift` places the displayed frame on the reference image position.
The reference is the first frame of the stored sequence. Image position
does not reorder that sequence. Orientation, spacing and dimensions come
from the displayed frame. The column shift is the inverse `x` of
`ImagePlane::index_to_world`. The row shift is the inverse `y`. The `z`
component is not a canvas translation.

PS3.3 C.7.6.2.1.1 is that existing function. This module does not write a
second dot product. A zero spacing on a singleton axis contributes no
shift. The `1.0` that keeps the matrix invertible is not a pixel size.

`apply_placement` stores the displayed column and row millimetres through
`set_spacing`, and the two shifts through `set_index_shift`. Imager pixel
spacing is not an argument. Gantry tilt is not an argument. A sheared
stack still returns a shift, because the positions already carry it.
`Volume::assemble` is not called.

Tier A and tier B upload the shift inside the camera matrix F-045 already
writes. This module does not draw and does not edit the render graph.
Tier C computes the same two numbers. `CpuRaster` samples with that view.

## Snapshot

F-050 stores the camera inputs, the display area and the presentation as
versioned UTF-8 JSON. The magic is `OCS1` and the only version this crate
reads is `1`. A reader refuses every other version and every field that
version does not name. The record does not store the inverse camera matrix.
`View::camera_matrix` remains the only derivation. Scroll, the selected
slice, the loading, error and refining flags, the generation and the device
pixel ratio are runtime state. A restore leaves them where it found them.

`StackViewport::snapshot` returns the bytes. `StackViewport::restore`
decodes the whole record, rebuilds the chain through `LutChain::new`, and
then assigns the view, the presentation and the CSS grid. A refusal assigns
nothing. An unfitted record uses the image centre and a fit scale of one.
A fitted record keeps the stored anchor and fit scale and does not call
`fit`.

A modality or VOI LUT sequence is stored as that sequence. It does not also
carry the rescale or the window the sequence overrode. A declared LUT entry
count of zero means 65,536 entries, DICOM PS3.3 C.11.2.1.1, and `values`
carries the resolved entries. Presentation evidence stays absent, an
explicit shape, or a sequence. Absent on `MONOCHROME1` is not stored as an
explicit inverse. DICOM PS3.3 C.11.6 names the two shapes.

The wasm exports are `snapshot`, `snapshot_error` and `restore_snapshot`.
`snapshot` copies a byte vector and returns an empty vector on refusal.
`snapshot_error` is the last encode code. Zero means the last snapshot
succeeded. `restore_snapshot` returns zero on success and the error code
otherwise. `packages/core/src/snapshot.ts` reads the magic, the version and
the codes from `generated.ts`.

The record is the same on tier A, tier B and tier C. Taking it does not
open a device and does not upload a texture.

## Orthographic planes

`RenderGraph::show_ortho` takes a host `OrthoImage` from `Volume::reslice`
or `Volume::reslice_oblique`. It does not sample the volume. `present_plane`
walks the samples. Tier C
returns `OrthoError::TierCUnavailable` before that walk and before a buffer
is allocated. An `Absent` sample returns `PlaneIncomplete`, and
`upload_stored` is not called, so `uploads` and the previous texture stay.
A complete plane uploads the `Present` `f32` values through the existing
stack pass. Windowing stays `VoiParams` and the existing VOI shader.
`show_ortho` adds no pass variant and no sampler in WGSL.

Tier A and tier B share that upload. The stored format stays the existing
choice. `show_ortho` does not draw a plane on tier C. `present_cpu_plane`
copies the same `reslice` and `reslice_oblique` samples into `CpuRaster`.
It does not upload and it does not resample. This file does not add a
second GPU upload.

## Thick slab

`RenderGraph::show_slab` takes a host `SlabImage` from
`Volume::slab_samples`. It does not sample the volume. `present_slab`
refuses tier C before it reads the image. An uncompiled blend, including
labelmap edge, returns `BlendUnavailable` before the image is read and
does not draw composite. A dimension above the device 3D limit returns
`ExtentUnavailable` and does not upload a clipped image.

A present pixel is drawn by one of the four slab pipelines. An absent
pixel discards. The pass loads the colour target, so the previous pixel
stays. `uploads` does not change. Tier A and tier B share that shader.
Tier C does not draw a slab. The sample grid remains available on the host.
F-X004 does not present a slab.

## Slice sync

`SyncScene` keeps one patient point across viewports that share a
`FrameOfReferenceUID`. The UID is the existing `FrameOfReferenceUid` string.
Comparison is `link::same_frame_of_reference`, described below, so the rule
lives in one function. A trailing space is a different UID. The scene does
not parse it and does not trim it.

`register` stores one slot, or replaces the slot with the same `u16`. It
does not move a peer. `set_point` looks up the source. A missing source or
a missing source UID is `SyncError::Unavailable`, and no slot is written.
Every slot whose UID equals the source UID receives
`HeldPlane::index_through_point`. A different UID stays. A missing peer UID
stays, and that case is not an error. Any same-UID plane that refuses the
point refuses the call before a slot is written.

The change list is registration order. An index that compares equal under
`==`, including the two zeros, is omitted and is not rewritten. `Ok` of an
empty list is a successful no-op. The anchor stays the world point of index
0. The normal and the spacing stay. The geometry is the volume function, so
tier A, tier B and tier C store the same index. This crate creates no
device.

## Linked viewports

`link` holds the geometry of linked scroll and reference lines over a list of
planes that need not be uniformly spaced, which `SyncScene`'s `HeldPlane`
cannot describe. F-209 added it. It is `no_std` with `alloc`, holds no state
and creates no device.

- `same_frame_of_reference(Option<&FrameOfReferenceUid>,
  Option<&FrameOfReferenceUid>)` is true only when both are `Some` and equal
  as strings. PS3.3 C.7.4.1.1.1 says the UID uniquely identifies a Frame of
  Reference for a Series, that several series in a study may share one, and
  that all images in a series that share the same UID are spatially related.
  The sentence on spatial relation is scoped to a series. Linking two series
  rests on the UID identifying one Frame of Reference, as cornerstone3D
  5.8.2's `getFrameOfReferenceUID()` equality does. A missing UID links
  nothing, not even to another missing one. `FrameOfReferenceUid` is
  re-exported here so a caller needs no dependency on `ocelli-volume`.
- `closest_coplanar(source, shown, stack)` is the index into `stack` whose
  Image Position (Patient) is nearest the source's, by Euclidean distance in
  millimetres, with a strict `<` so a tie keeps the lower index. It is
  `LinkError::NotCoplanar` when `abs(dot)` of the source normal and the shown
  target normal is not above `COPLANAR_DOT`, 0.9, and `LinkError::Degenerate`
  when no distance is finite, which includes an empty stack.
- `reference_segment(source, target)` is where the source plane cuts the
  target, as two `Pt<Index>` in the target's index space, column `x` and row
  `y`, with the first pixel centre at 0. The source corners are the centres
  of its four corner pixels through `index_to_world`. When the source's
  top-bottom edge is perpendicular to the target normal within
  `PLANE_EPSILON`, 1e-3, the top and bottom edges are cut, otherwise the left
  and right. It is `LinkError::Parallel` when `abs(dot)` of the normals is
  above `1 - PLANE_EPSILON`, and `LinkError::Degenerate` for a non-finite
  point or for a target with a zero spacing, which PS3.3 10.7.1.3 allows only
  on a single row or column and where `index_to_world` puts a unit extension.

`COPLANAR_DOT` is cornerstone3D 5.8.2's `areViewportsCoplanar` threshold and
`PLANE_EPSILON` its `EPSILON`, used by `isParallel` and `isPerpendicular` in
`ReferenceLinesTool`. The intersection is that tool's `linePlaneIntersection`
with `showFullDimension: false` over the whole source image. Registration
across frames of reference is not done.

The normal is `ImagePlane::slice_normal`, `X cross Y`. PS3.3 C.7.6.2.1.1
spans the image plane with the row cosine `X` and the column cosine `Y` and
requires them orthogonal and of unit length, so that cross product is a unit
normal of the plane. The section does not name a normal, and no rule here
reads its sign, because every comparison takes the absolute value of a dot
product. The index is `index_to_world().inverse()`, the pattern
`placement::frame_shift` uses. The arithmetic is `f64` CPU arithmetic, so
tiers A, B and C give the same answer.

The folder viewer's `POST /link` is the one caller of the geometry functions
today, and `SyncScene` also calls `same_frame_of_reference`. It sends each plane
as a token of thirteen numbers written by the local process and rebuilds it
through the `ocelli-pixel` constructors, and the page draws the returned line
in image pixel units, one half pixel from index space.

## What this file does not own

The TypeScript names are F-100. This crate does not depend on wgpu.
Volume assembly and measured shear stay in `docs/lld/volumes.md`. Scroll
does not read them. `SyncScene` calls `HeldPlane::index_through_point` and
does not assemble a volume. Volume brick prefetch in HLD section 30 is
not the scroll module.
