# F-064, contour representation

**Status**: approved
**Epic ref**: E10.2
**Sprint**: S23
**Estimate**: 4w

The operator told this run to finish the sprint without a question pause.
The choices in `## What the specification does not cover` are local.
No material question remains.

## Normative source, transcribed

From `docs/hld/03-architecture-and-crates.md` section 4, the `ocelli-seg` row:

Segmentation state and the three representations. wasm yes. native yes.

From `docs/hld/B-parity-surface.md`:

Segmentation representations, 3, Labelmap, contour, surface.

From `docs/hld/19-render-graph.md` section 22:

```rust
pub enum Pass {
    Stack(StackPass),
    VolumeRaycast(VolumePass),
    SegOverlay(SegPass),
}
```

From the same section:

- Pipelines compile at init, keyed by (pass kind, blend mode, tier). Never compile a shader mid-frame.
- Dirty tracking. A viewport re-renders only when camera, VOI, slice, segmentation or data changed. Everything else is a no-op frame.
- One queue.submit() per frame across all viewports.
- Device loss is a real state. Rebuild the device and all resources, and restore viewport state from the shell's copy.

From `docs/hld/26-differentiating-capabilities.md` section 31:

Results stay resident on the GPU wherever the consumer is the renderer. A segmentation mask should never round-trip through JavaScript to be drawn.

A kernel with no fallback marks its feature unavailable. It never silently produces a different answer.

From `docs/hld/01-purpose-and-scope.md` section 1:

Phase 1 includes segmentation. Curved MPR, surface rendering, registration, the study library UI and the native desktop shell are Phase 3.

`docs/hld/` does not define a contour, a polyline, or a raster rule. F-063 already presents the labelmap. This story does not reopen that presenter.

## What the specification does not cover

The HLD names three segmentation representations and does not say what a contour stores or how it is drawn. These choices are local.

- A contour is one polyline in DICOM patient space, `Pt<World>` from `ocelli-core`. The geometric type follows the names in DICOM PS3.3 C.8.8.6 Contour Geometric Type: `Point`, `OpenPlanar`, `OpenNonplanar`, `ClosedPlanar`. F-099 parses RTSTRUCT later. This story does not parse a file.
- The draw plane is the world XY plane of a caller-supplied origin. `spacing_x_mm` and `spacing_y_mm` are the pixel steps. Column `c = round((x - origin.x) / spacing_x_mm)` and row `r = round((y - origin.y) / spacing_y_mm)`. `round` is `f64::round`, so 0.5 becomes 1 and -0.5 becomes -1.
- A point whose `z` bits differ from `origin.z` is out of plane. The presenter refuses and writes nothing. It does not drop `z`.
- `Point` holds one point and paints that pixel. `OpenPlanar` and `OpenNonplanar` hold at least two points and do not close. `ClosedPlanar` holds at least three points and the last segment returns to the first.
- The stroke is Bresenham, inclusive of both ends. A pixel outside the grid is skipped. A pixel painted twice keeps the same texel.
- The colour is the F-063 `LabelColourLut` entry for the segment. The stroke uses that alpha. It does not force alpha 255 and it does not fill the interior.
- The composite overlay already copies presenter bytes. `show_contour` reuses that upload and that shader. There is no second shader and no new blend.
- Section 1's Phase 3 phrase "surface rendering" is a volume display mode. It is not this polyline.

## Problem and reproducer

`ocelli-seg` can present a label grid. It has no contour value. An open line from `(0, 0, 0)` to `(3, 0, 0)` on a 1 mm grid must paint four pixels `(10, 20, 30, 128)` and leave the row beneath transparent. The labelmap outline of a box would also paint the other three sides and would force the boundary to alpha 255. Those bytes must not come back from the contour presenter.

## Scope, API and compatibility

In scope: the contour value, the plane frame, the CPU presenter, and `RenderGraph::show_contour` on a composite `SegOverlay`.

Out of scope: RTSTRUCT parsing, polygon fill, marching cubes, the representation registry, labelmap edge projection, any edit to `present_labelmap`, and any new WGSL file.

```rust
pub enum ContourGeometricType { Point, OpenPlanar, OpenNonplanar, ClosedPlanar }
pub struct PlaneFrame { pub origin: Pt<World>, pub spacing_x_mm: f64, pub spacing_y_mm: f64 }
pub struct Contour { pub segment: u16, pub kind: ContourGeometricType, pub points: Vec<Pt<World>> }
pub fn present_contour(contour: &Contour, frame: &PlaneFrame, lut: &LabelColourLut, width: u32, height: u32, out: &mut [u8]) -> Result<(), ContourError>
```

`RenderGraph::show_contour` presents those bytes and uploads them through the existing overlay binding. Equal bytes do not allocate. A presenter error uploads nothing.

F-063 viewports keep working. This story does not add a registry check.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Host and tests | `present_contour` | World points, plane frame, LUT, destination | RGBA8 or a refusal with the destination unchanged |
| `RenderGraph::show_contour` | `present_contour` then the F-063 upload | Tier A or B, composite `SegOverlay` | Next tick copies those bytes |
| `RenderGraph::tick` | Existing copy shader | Overlay texture already uploaded | Target matches `present_contour` |
| `present_labelmap` | Unchanged F-063 presenter | Label grid | Labelmap bytes, including outline alpha 255 |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Empty destination | Open line on the plane | Stroke pixels hold the LUT texel | Not a refusal |
| Destination holding sentinels | Bad length, out of plane, bad frame, or missing colour | Destination unchanged | `ContourError` |
| Uploaded contour | Same bytes again | No new texture and no new dirt | Last image stays |
| Uploaded contour | New bytes | Texture replaced after it exists, then data dirt | A present error keeps the previous binding |
| Composite overlay | Device recovery, then rebind, then `show_contour` | The same bytes are uploaded again | `show_contour` before rebind is `NotRebound` |

Index 0 is not a segment. A contour whose segment is 0 fails the colour lookup.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Presenter bytes | Caller buffer | Grid size, frame, points, LUT | `present_contour` returns `Ok` | `Err` does not write |
| Overlay texture | `show_contour` after a successful present | Width, height and byte stamp | Slot binding replaced, then data dirt set | A present error uploads nothing |
| Pipeline | Existing F-063 compile | Composite `SegOverlay` | Graph construction | Not compiled mid-frame |

Device recovery already drops the overlay texture. The next `show_contour` after `rebind` uploads again. Equal bytes after a successful upload do not allocate.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Open line `(0,0,0)` to `(3,0,0)`, spacing 1, origin 0, segment 1 colour `(10,20,30,128)`. Pixels `(0,0)` through `(3,0)` hold that colour. `(0,1)` stays transparent. | `open_line_paints_the_stroke_only` | Outline alpha 255, or a filled box | implementation |
| AC2 | Closed square `(0,0,0) (3,0,0) (3,2,0) (0,2,0)`. Border pixels hold the LUT alpha. Interior `(1,1)` stays transparent. | `closed_square_does_not_fill` | Polygon fill, or the labelmap outline of the same box | implementation |
| AC3 | Diagonal `(0,0,0)` to `(2,2,0)` paints `(0,0) (1,1) (2,2)` only. | `diagonal_bresenham_is_the_three_pixels` | A bounding box, or the four-neighbour labelmap rule | implementation |
| AC4 | Point `(1.5, 0.5, 0)` paints pixel `(2,1)` and nothing else. | `half_rounds_away_from_zero` | Truncation toward zero | implementation |
| AC5 | `z` bits differ, a short point list, a missing colour, a non-positive spacing, or a short output. Sentinels stay. | `a_contour_refusal_writes_nothing` | A partial stroke | implementation |
| AC6 | GPU readback equals `present_contour`. A second show of the same bytes does not submit. | `shown_contour_matches_the_presenter` | Shader recomputes a labelmap outline | gpu gate |
| AC7 | After `install_recovered` and `rebind`, `show_contour` uploads again and the readback matches. Before rebind the show is `NotRebound`. | `contour_recovery_reuploads` | Recovery leaves the dead texture, or equal bytes skip the reupload | gpu gate |

## Approach

Add `present_contour` in `ocelli-seg`. The crate gains a path dependency on `ocelli-core` for `Pt<World>`. `show_contour` matches `show_labelmap` and calls `labelmap::stage` with the contour bytes. The copy shader stays `labelmap.wgsl`.

Rejected: storing the contour as a label grid. Rejected: forcing outline alpha 255. Rejected: dropping `z` for an oblique point. Rejected: a new shader that strokes in WGSL.

## Dependencies, write ownership and resources

F-063 is done. This story edits `crates/ocelli-seg`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs`, and the render tests. It does not edit `voi.wgsl` or `labelmap.wgsl`.

Serial, in the canonical worktree. The GPU gate stays serial. F-065 and F-066 are separate stories and are not started here.

## Verification and evidence reuse

Focused checks: `bin/ocelli.sh test ocelli-seg` and `bin/ocelli.sh test ocelli-render --test contour` plus the labelmap test. Clippy on both crates.

Completion uses the feature floor plus corpus. The stored comparison report stays valid only while CPU candidate bytes and the oracle producer stay unchanged. This story does not edit those paths.

A mutation that fills the interior of the closed square must fail AC2. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The overlay texture is created in `show_contour`, which is not the tick.
- unsafe: none
- Tier A (WebGPU): full. The composite overlay copies the contour bytes.
- Tier B (WebGL2): full. The same fragment shader. No compute.
- Tier C (CPU): full through `present_contour`. `show_contour` returns unavailable before the points are read.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Hand RGBA for the open line, the closed square, the diagonal and the half-pixel, citing PS3.3 C.8.8.6 for the type names and this plan for the raster | `crates/ocelli-seg/tests/contour.rs` |
| unit | Refusals leave the destination unchanged | same file |
| gpu | Readback matches the presenter, and recovery uploads again | `crates/ocelli-render/tests/contour.rs`, ignored, gpu gate |

## Parity surface covered

Segmentation representations, the contour row only. Surface stays uncovered. Labelmap edge projection stays uncovered.

## Deviations

none

## LLD impact

`docs/lld/render-graph.md` and `docs/lld/feature-availability.md` gain the contour presenter. `docs/lld/README.md` if the index row needs the F-ID.

## Open questions

none
