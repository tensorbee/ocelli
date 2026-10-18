# Public API

**F-IDs that contributed:** F-100
**Last updated:** 2026-10-18

The viewport, scene and tool surface. It is a new API, HLD section 2.1.
F-101 implements the wire. This file names what that wire will carry.

## The boundary this file does not build

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

`flags` bit 0 is invert, bit 1 is loading, bit 2 is error. This layout is
the readback contract. Nothing in this sprint copies the struct into
JavaScript.

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
`available`. Tier C drawing is `unavailable`. The CPU raster is F-X003. This
story declares no drawing fallback, so the tier C answer is `unavailable`.

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
`packages/react` stays a scaffold. F-102 owns framework bindings.
