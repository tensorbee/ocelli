# F-048, coordinate transforms between canvas, world and index

**Status**: approved
**Epic ref**: E7.4
**Sprint**: S16
**Estimate**: 3w
**Owner**: xAI initial implementation, Codex completion, independent review deferred by operator

## Normative source, transcribed

HLD section 16:

```rust
/// CSS pixels inside a viewport element. Origin top-left, y increases down.
pub enum Canvas {}
/// DICOM patient coordinate system (LPS), millimetres.
pub enum World {}
/// Voxel indices within a volume. Origin at voxel (0,0,0).
pub enum Index {}
pub struct Transform<A, B> { m: glam::DMat4, _p: PhantomData<(A, B)> }
```

The declared operations are `apply(Pt<A>) -> Pt<B>`,
`inverse() -> Transform<B, A>` and
`then<C>(&Transform<B, C>) -> Transform<A, C>`.
HLD section 25.1 requires world coordinates within `1e-6` mm and canvas
coordinates within a quarter pixel. No tolerance change is proposed.

PS3.3 C.7.6.2.1.1 maps integer column i and row j to the center of a pixel:
`P = IPP + i * column_spacing * X + j * row_spacing * Y`.
Use the existing ImagePlane implementation of this equation.

## Problem and scope

View already maps texture-corner coordinates to a display plane, and
ImagePlane maps DICOM voxel centers to patient coordinates. There is no
public typed adapter between those domains. F-101 and F-X003 must not invent
two adapters or use device pixels as Pt<Canvas> CSS pixels.
No viewport serialization, MPR or new point-array type is in scope.

## Consumer map

| Consumer | Provider | Context and observation |
|----------|----------|-------------------------|
| Tools and measurements | View plus ImagePlane | CSS point to LPS mm and back |
| F-X003 raster | View's shared inverse mapping | Canvas sample center to stored texel |
| GPU camera | View::camera_matrix | Device fragment position converted to the same mapping |
| F-049 placement | ImagePlane inverse | Existing measured shift, retained unchanged |

## Proposed API and approach

`canvas_to_world` and `world_to_canvas` take `&ImagePlane`.
`canvas_to_index` and `index_to_canvas` take the column and row counts,
because `View` does not store the grid. The signatures are below.
Return the inverse typed transforms for the reverse operations. Use one f64
mapping owner in View rather than invert the already-rounded f32 GPU uniform.
Factor the existing basis and translation, preserving the F-045 composition.
No new generic, trait, wrapper or dynamic dispatch is required.

The public index adapter converts in index space. `image = index + 0.5` on
x and y, and image z stays the index slice. Voxel `(0,0)` has image centre
`(0.5, 0.5)`. DICOM index `(0,0,0)` stays IPP. No half-voxel is added in
patient millimetres. `Pt<Canvas>` and `View` stay in CSS pixels. A device
pixel becomes CSS once, before this mapping, as `css = device / dpr`, using
the same dpr as `device_extent`. F-101 owns that division at the boundary.
This story does not store device pixels in `View`.

```rust
pub enum CoordinateError {
    /// The f64 display map is not invertible, or a composed component is
    /// not finite. `camera_matrix` may still publish a raster fallback.
    Singular,
}

impl View {
    pub fn canvas_to_index(
        &self,
        columns: u32,
        rows: u32,
    ) -> Result<Transform<Canvas, Index>, CoordinateError>;
    pub fn index_to_canvas(
        &self,
        columns: u32,
        rows: u32,
    ) -> Result<Transform<Index, Canvas>, CoordinateError>;
    pub fn canvas_to_world(
        &self,
        plane: &ImagePlane,
    ) -> Result<Transform<Canvas, World>, CoordinateError>;
    pub fn world_to_canvas(
        &self,
        plane: &ImagePlane,
    ) -> Result<Transform<World, Canvas>, CoordinateError>;
}
```

`canvas_to_index` takes the grid because `View` does not store columns or
rows. `canvas_to_world` uses `plane.dimensions()` for that grid and
`ImagePlane::index_to_world` for patient space. A singular f64 2 by 2 map,
or a non-finite composed component, returns `CoordinateError::Singular`.
The rank-one pseudo-inverse and centre fallback stay inside `camera_matrix`
and are not a measurement. Preserve F-049's axis-aligned placement shift.

## Acceptance and discriminating tests

1. Index `(0,0,0)` maps to exactly IPP, with no physical half-voxel offset.
   Derive the matching canvas position from the settled corner/center rule.
2. Synthetic IPP `(10,20,30)`, axial X/Y, spacing row 2 and column 0.5,
   index `(2,3,0)` maps to `(11,26,30)`. This rejects transposed spacing.
3. Repeat with oblique X/Y and nonzero pan, quarter-turn, horizontal flip,
   zoom and F-049 shift. Compute reference points independently from PS3.3
   and the approved View composition. Inverse round-trips alone are not proof.
4. CSS point mapping is invariant at DPR 1, 1.25 and 2. The GPU's device
   position is converted once. Reject both missing and double DPR scaling.
5. World round-trips meet `1e-6` mm and canvas meets quarter pixel. Type
   mismatches fail to compile using the existing coordinate test conventions.
6. Degenerate spacing/zoom has an explicit unavailable/error answer. A
   singular map must not produce non-finite measurement coordinates.
7. A viewport change updates camera dirt and does not alter sample bytes or
   increment image, colour or brick upload counters.

Fixtures belong in `ocelli-viewport/tests/coordinates.rs`, with a combined
mapping GPU observation in `ocelli-render/tests/view.rs`. Include singleton
axes, anisotropic oblique planes and negative canvas positions. Record actual
selected/ignored test counts. Mutate one spacing factor and observe semantic
RED, then restore and GREEN. HLD 27.2 R2/R3 and PS3.3 C.7.6.2.1.1 apply.

## Dependencies, order and checks

F-045 is done. Establish the shared mapping before F-X003 sampling and before
F-101 publishes coordinate-dependent readback. F-048 and F-X003 share View
and raster semantics, so implement serially unless write sets prove separate.
F-049's existing placement remains the provider. No GPU work runs concurrently.

Run viewport check/test/clippy, the existing coordinate compile-fail tests,
relevant renderer tests, browser DPR observation and triggered oracle checks,
then final integrated `gate --sprint`. A skipped adapter is not tier evidence.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: no, transforms built when state changes
- unsafe: none
- Tier A (WebGPU): shared mapping, adapter-specific draw checked separately
- Tier B (WebGL2): same mapping, actual WebGL2 observation required
- Tier C (CPU): same typed mapping, raster integration belongs to F-X003

## Parity surface covered

Coordinate mapping only. No new viewer parity claim until actually observed.

## Deviations and LLD impact

None proposed. Update `docs/lld/viewport.md`, `core-types.md` if its public
surface changes, and their index rows. Keep current f64/f32 behavior explicit.

## What the specification does not cover

HLD section 16 names the spaces and `Transform`. It does not join `View` to
`ImagePlane`, choose a public corner or centre index, place the CSS division,
or name the singular-measurement error. Those choices are recorded above.
The operator accepted them on 2026-10-18.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Any `View`, mapping call | CSS point and plane or grid | A typed transform, or `Singular` | `View` is not written |
| `set_zoom(0)` or zero spacing on both axes | Later mapping call | `CoordinateError::Singular` | Pan, zoom, rotation, flip, spacing and shift stay |
| `set_pan` with nonzero z | Pan point | `ViewError::PanZ` | Stored pan stays. This story does not change that refusal |
| `fit`, pan, zoom, rotate, flip, spacing, shift | Next mapping call | The new composition | No sample bytes and no upload counters move, because these functions do not touch the renderer |

The patient equation and the display composition stay two owners. A round
trip that shares one wrong spacing is not acceptance. The synthetic world
point is computed from PS3.3, and the canvas point is computed from
`image = index + 0.5` through the existing `image_to_canvas`.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Index `(0,0,0)` is IPP. Its canvas point is the image point `(0.5, 0.5)` through the current view | `coordinates.rs` | A half-voxel shift in millimetres, or a corner index treated as the centre | implementation |
| AC2 | IPP `(10,20,30)`, axial, row spacing 2, column 0.5, index `(2,3,0)` is world `(11,26,30)` | same fixture, tolerance `1e-6` mm | Transposed `PixelSpacing` | implementation |
| AC3 | Oblique axes, pan, quarter turn, horizontal flip, zoom and the F-049 shift match an independent composition | same fixture, canvas within a quarter pixel | Inverse-only round trip used as the expected value | implementation |
| AC4 | CSS mapping is unchanged at dpr 1, 1.25 and 2 when `css = device / dpr` is applied once | viewport fixture and `ocelli-render/tests/view.rs` | Missing division and a second division | implementation |
| AC5 | World error stays within `1e-6` mm and canvas within a quarter pixel. Cross-space calls fail to compile | existing coordinate compile-fail plus this fixture | A shared `f64` bag | implementation |
| AC6 | A singular display map returns `CoordinateError::Singular` and no non-finite point | zoom 0 and a zero basis | `camera_matrix` centre fallback reused as a measurement | implementation |
| AC7 | Mapping calls do not upload samples, colour tables or bricks | existing `set_view` counters stay the observation. These functions have no renderer call | A texture write inside the adapter | implementation |

## Verification and evidence reuse

Focused checks are `bin/ocelli.sh check ocelli-viewport`,
`bin/ocelli.sh test ocelli-viewport`, and
`bin/ocelli.sh clippy ocelli-viewport`, plus the render view test that
compares the f64 map with `camera_matrix` after one dpr division. Geometry
also runs `bin/ocelli.sh gate oracle` when that gate is available. A missing
adapter is recorded and is not a passed GPU draw. No tolerance moves.
Semantic RED is the centre fixture failing while the `+ 0.5` index shift is
absent. Mutation RED removes that shift or one spacing factor, then the
restored source is GREEN. No external host is required for these rows.

## Open questions

None. The operator answered on 2026-10-18.

Centre versus corner: convert in index space with `image = index + 0.5`.
Do not add a half-voxel in patient space.

Singular measurements: `CoordinateError::Singular`. Do not use the raster
pseudo-inverse.

CSS versus device: divide once at the boundary. `View` stays in CSS pixels.

## Repair, 2026-10-19

The GPU consumer divides in the fragment shader. The uniform is the CSS
matrix plus the same device pixel ratio `device_extent` used. The shader
divides the fragment position before the CSS multiply. `View` is not given
a device pixel ratio. A failed adapter open is not a pixel observation.

## Repair, 2026-10-19, camera uniform record

`docs/lld/render-graph.md` and `docs/lld/viewport.md` record the 80-byte
`DeviceCamera`. Registration, `set_view` and `rebind` own that uniform. The
ratio is an argument of `set_view`. The 96-byte `ViewportState` and its
64-byte camera field stay as they are. Rendered DPR 1.25, tier B and browser
GPU coverage remain unperformed observations.

## Additional native fractional-DPR observation, 2026-10-19

A permanent real-adapter test renders DPR 1.25. Device centres divided once
select source indices 0, 1, 2, 2 and 3. Missing division and double division
select different arrays. The observed frame matches the hand-derived indices.
This supplements the earlier DPR 2 frame. It does not supply a browser GPU
canvas or WebGL2 observation. Independent approval remains deferred.
