# Viewport

**F-IDs that contributed:** F-045, F-046, F-047, F-048, F-049, F-050, F-101, F-X003, F-X022
**Last updated:** 2026-10-27

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
A map with no extent writes the image centre. Neither path writes a
non-finite value. Narrowing `f64` to `f32` is ties-to-even. It is not a
Rust `as` cast.

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
Cache kinds 1 to 5 are not produced here.

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
for the selected key only. A miss is `NotResident` and returns no neighbour
bytes. An empty sequence is `Empty`.

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

## What this file does not own

The TypeScript names are F-100. This crate does not depend on wgpu.
Volume assembly and measured shear stay in `docs/lld/volumes.md`. This
module does not read them. Volume brick prefetch in HLD section 30 is
not the scroll module.
