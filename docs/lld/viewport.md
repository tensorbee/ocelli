# Viewport

**F-IDs that contributed:** F-045, F-046, F-047, F-049
**Last updated:** 2026-10-18

`View` in `crates/ocelli-viewport` is the stack camera. It does not draw.
`RenderGraph::set_view` uploads the sixteen floats `View::camera_matrix`
returns. `Presentation` in the same crate holds the LUT chain and the
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
shader multiplies `(at.x, at.y, 0, 1)` by it and truncates with `i32` toward
zero. A zero spacing on one axis keeps the other axis and that axis's shift.
A map with no extent writes the image centre. Neither path writes a
non-finite value. Narrowing `f64` to `f32` is ties-to-even. It is not a
Rust `as` cast.

## What the GPU stores

Registration writes the identity matrix into a 64-byte uniform at group 0
binding 2. `set_view` replaces those bytes and sets `Dirty.camera`. It does
not write the VOI uniform, the colour uniform, the stored texture or a brick.
`uploads`, `lut_uploads` and `brick_uploads` stay put. A fragment outside
`[0, columns)` by `[0, rows)` writes `(0, 0, 0, 1)` and does not run
`lut_chain`. A negative coordinate is rejected before `i32` truncation, so
it does not select texel 0.

The vertex shader stays the full-target triangle. The pipeline is compiled
at graph creation. There is no mid-frame compile. Device-loss recovery drops
the uniform and keeps the floats. `rebind` writes them into the new buffer.

## Tiers

Tier A and tier B sample through the same fragment module. Tier C drawing
stays unavailable. `RenderGraph::new` returns `TierCUnavailable`. The `View`
math runs with no device. CPU raster is F-X003.

## Presentation

`Presentation` holds a `LutChain` and a `Colormap`. A preset or `set_window`
rebuilds stage 2 through `VoiTransform::new` and keeps the modality transform,
the presentation evidence and the output range. `soft_tissue` is centre 40
and width 400, `LINEAR`. `lung` is centre -600 and width 1500. `bone` is
centre 300 and width 1500. Both of those are `LINEAR` as well. None of the
three adds a formula.

A preset on a modality LUT sequence or a VOI LUT sequence returns
`PresentationError::Sequence` and leaves the chain in place. `set_invert`
stores an explicit `PresentationLutEvidence::Shape` and rebuilds the chain.
An explicit shape is not combined with `MONOCHROME1`. `set_colormap` stores
`Grey` or `Stops`. A window change does not change it.

One match in `ocelli-render`, `colour_lut_for`, maps `Colormap` to `PresetId`
and calls `ColourLut::preset`. This crate does not depend on `ocelli-render`
and does not call `set_voi` or `upload_stored`.

Tier A and tier B consume the rebuilt chain through the existing VOI uniform.
Tier C runs the same `LutChain`. Drawing on tier C stays unavailable. A
colormap on tier C indexes the same preset bytes, after `map_into`.

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
cache. Drawing stays unavailable.

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
Tier C computes the same two numbers. Drawing stays unavailable.

## What this file does not own

The TypeScript names are F-100. This crate does not depend on wgpu.
Volume assembly and measured shear stay in `docs/lld/volumes.md`. This
module does not read them. Volume brick prefetch in HLD section 30 is
not the scroll module.
