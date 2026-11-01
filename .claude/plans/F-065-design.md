# F-065, surface representation

**Status**: approved
**Epic ref**: E10.3
**Sprint**: S23
**Estimate**: 3w

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

`docs/hld/` does not define a mesh, a triangle, or a coverage rule. The Phase 3 words "surface rendering" name a volume display mode. Appendix B's surface is the third segmentation representation. This story implements the Appendix B row. It does not implement the Phase 3 volume mode.

## What the specification does not cover

The HLD names the surface representation and does not say what it stores or how a pixel is covered. These choices are local.

- A surface is a triangle mesh. Vertices are `Pt<World>`. Each triangle is three `u32` indexes into that list.
- The plane frame is F-064's `PlaneFrame`. Continuous pixel position is `px = (x - origin.x) / spacing_x_mm` and `py = (y - origin.y) / spacing_y_mm`. Vertices are not rounded before the coverage test.
- A vertex whose `z` bits differ from `origin.z` is out of plane. The presenter refuses and writes nothing.
- Pixel `(c, r)` is covered when the barycentric weights of its centre `(c + 0.5, r + 0.5)` are all `>= 0`. The edge is covered. A clockwise triangle uses the same formula. A triangle whose cross product is `0` is skipped.
- The covered texel is the `LabelColourLut` entry. Alpha stays the LUT alpha.
- One degenerate triangle among real triangles is skipped. A bad index refuses the whole mesh. If no pixel is covered, the presenter refuses and writes nothing.
- The caller supplies the mesh. This story does not run marching cubes and does not build a mesh from a label grid.
- `show_surface` reuses the F-063 copy shader. There is no new WGSL file.
- A contour of the same three vertices strokes the Bresenham edges only. The surface paints the interior. Pixel `(4, 0)` of the hand triangle is a contour endpoint and is outside the surface, because its centre `(4.5, 0.5)` is outside the triangle.

## Problem and reproducer

F-064 can stroke a polyline. It cannot cover a triangle. The hand mesh `(0,0,0) (4,0,0) (0,4,0)` on a 5 by 5 grid of 1 mm pixels must paint `(1,1)` with `(10, 20, 30, 128)` and must leave `(4,0)`, `(0,4)` and `(3,1)` transparent. A contour through those vertices paints `(4,0)` and `(0,4)` and leaves `(1,1)` transparent. The two presenters must disagree on those pixels.

## Scope, API and compatibility

In scope: the mesh value, the CPU coverage presenter, and `RenderGraph::show_surface` on a composite `SegOverlay`.

Out of scope: marching cubes, volume surface rendering, the representation registry, labelmap edge projection, any edit to the contour stroke rule, and any new WGSL file.

```rust
pub struct SurfaceMesh { pub segment: u16, pub vertices: Vec<Pt<World>>, pub triangles: Vec<[u32; 3]> }
pub fn present_surface(mesh: &SurfaceMesh, frame: &PlaneFrame, lut: &LabelColourLut, width: u32, height: u32, out: &mut [u8]) -> Result<(), SurfaceError>
```

`RenderGraph::show_surface` presents those bytes and uploads them through the existing overlay binding. Equal bytes do not allocate. A presenter error uploads nothing.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Host and tests | `present_surface` | Mesh, plane frame, LUT, destination | RGBA8 or a refusal with the destination unchanged |
| `RenderGraph::show_surface` | `present_surface` then the F-063 upload | Tier A or B, composite `SegOverlay` | Next tick copies those bytes |
| `present_contour` | F-064 presenter | Same vertices as a closed polyline | Edge pixels only, so the interior stays clear |
| `RenderGraph::tick` | Existing copy shader | Overlay texture already uploaded | Target matches `present_surface` |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Empty destination | Hand triangle | Covered centres hold the LUT texel | Not a refusal |
| Destination holding sentinels | Bad index, out of plane, missing colour, or nothing covered | Destination unchanged | `SurfaceError` |
| Mesh with one degenerate triangle and one real triangle | Present | The real triangle is drawn | The degenerate one adds no texel |
| Uploaded surface | Same bytes again | No new texture and no new dirt | Last image stays |
| Uploaded surface | Device recovery, then rebind, then `show_surface` | The same bytes are uploaded again | `show_surface` before rebind is `NotRebound` |

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Presenter bytes | Caller buffer | Grid size, frame, mesh, LUT | `present_surface` returns `Ok` | `Err` does not write |
| Overlay texture | `show_surface` after a successful present | Width, height and byte stamp | Slot binding replaced, then data dirt set | A present error uploads nothing |
| Pipeline | Existing F-063 compile | Composite `SegOverlay` | Graph construction | Not compiled mid-frame |

Device recovery already drops the overlay texture. The next `show_surface` after `rebind` uploads again.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Triangle `(0,0,0) (4,0,0) (0,4,0)`, spacing 1, origin 0, 5 by 5. `(0,0)`, `(1,0)`, `(0,3)` and `(1,1)` are `(10,20,30,128)`. `(4,0)`, `(0,4)` and `(3,1)` are transparent. | `triangle_covers_centres_inside_the_edge` | Painting the rounded vertex, or leaving the interior clear | implementation |
| AC2 | The same three vertices as an F-064 closed contour paint `(4,0)` and leave `(1,1)` clear. | `contour_and_surface_disagree_on_the_hand_triangle` | One presenter calling the other | implementation |
| AC3 | A zero-area triangle writes nothing. A mesh whose second triangle is the hand triangle still paints `(1,1)`. | `a_degenerate_triangle_is_skipped` | Refusing the whole mesh because one triangle has no area | implementation |
| AC4 | A bad index, a `z` mismatch, a missing colour, or a short output leaves sentinels. | `a_surface_refusal_writes_nothing` | A partial triangle | implementation |
| AC5 | GPU readback equals `present_surface`. | `shown_surface_matches_the_presenter` | The copy shader strokes the edges | gpu gate |
| AC6 | After `install_recovered` and `rebind`, `show_surface` uploads again and the readback matches. | `surface_recovery_reuploads` | Equal bytes skip the reupload after the binding was dropped | gpu gate |

## Approach

Add `present_surface` in `ocelli-seg` beside the contour presenter. Coverage is the barycentric test of the pixel centre in the plane frame's continuous pixel space. `show_surface` uploads through `labelmap::stage`.

The hand weights for centre `(0.5, 0.5)` are `0.75`, `0.125`, `0.125`. Centre `(4.5, 0.5)` has a negative weight. Centre `(0.5, 3.5)` has a zero weight and is covered. Centre `(3.5, 1.5)` has a negative weight.

Rejected: marching cubes. Rejected: drawing only the three edges. Rejected: rounding vertices to pixels and then filling the rounded triangle, which would paint `(4,0)`.

## Dependencies, write ownership and resources

F-064 lands first because AC2 calls `present_contour`. This story edits `crates/ocelli-seg` and `crates/ocelli-render`. It does not edit the contour stroke.

Serial, in the canonical worktree. The GPU gate stays serial.

## Verification and evidence reuse

Focused checks: `bin/ocelli.sh test ocelli-seg` and `bin/ocelli.sh test ocelli-render --test surface`. Clippy on both crates.

Completion uses the feature floor plus corpus. This story does not edit CPU candidate bytes or the oracle producer.

A mutation that uses the contour stroke for the mesh must fail AC1 at `(1,1)` or at `(4,0)`. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The overlay texture is created in `show_surface`, which is not the tick.
- unsafe: none
- Tier A (WebGPU): full. The composite overlay copies the surface bytes.
- Tier B (WebGL2): full. The same fragment shader. No compute.
- Tier C (CPU): full through `present_surface`. `show_surface` returns unavailable before the mesh is read.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Hand coverage for the 5 by 5 triangle, citing this plan for the centre test and PS3.3 C.8.20.2 for segment numbers starting at 1 | `crates/ocelli-seg/tests/surface.rs` |
| unit | Refusals and the skipped degenerate triangle | same file |
| gpu | Readback matches the presenter, and recovery uploads again | `crates/ocelli-render/tests/surface.rs`, ignored, gpu gate |

## Parity surface covered

Segmentation representations, the surface row only. Labelmap edge projection stays uncovered.

## Deviations

none

## LLD impact

`docs/lld/render-graph.md` and `docs/lld/feature-availability.md` gain the surface presenter. `docs/lld/README.md` if the index row needs the F-ID.

## Open questions

none
