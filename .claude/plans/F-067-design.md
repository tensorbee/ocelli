# F-067, segmentation intersection and labelmap edge-projection blend

**Status**: approved
**Epic ref**: E10.5
**Sprint**: S24
**Estimate**: 2w

The operator told this run to finish the sprint without a question pause.
The choices in `## What the specification does not cover` are local.
No material question remains.

## Normative source, transcribed

From `docs/hld/03-architecture-and-crates.md` section 4, the `ocelli-seg` row:

Segmentation state and the three representations. wasm yes. native yes.

From `docs/hld/B-parity-surface.md`, the blend-mode row:

Blend modes, 5, Composite, MIP, MinIP, average, labelmap edge projection.

From the same appendix, the representation row:

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
- One queue.submit() per frame across all viewports.

From `docs/hld/26-differentiating-capabilities.md` section 31:

Results stay resident on the GPU wherever the consumer is the renderer. A segmentation mask should never round-trip through JavaScript to be drawn. A feature that cannot run reports unavailable. It does not alias onto another result.

`docs/hld/` names labelmap edge projection and does not give its formula, its thickness, or an intersection equation. DICOM PS3.3 C.8.8.6 names contour geometric types. It does not define a plane cut. DICOM PS3.3 C.8.20.2 keeps segment 0 out of the colour table. F-063 already uses that rule for fill and for the one-pixel 4-connected outline.

## What the specification does not cover

These choices are local. They are not a reading of a vtk shader and they are not a DICOM formula.

- Intersection is a contour against one patient Z plane. The dependency is F-064. A volume ray is not this story.
- A point is on the plane when `z.to_bits()` equals `plane_z.to_bits()`. `-0.0` and `+0.0` are different. That is the F-064 rule.
- A segment with endpoints on opposite sides of the plane contributes one point. `t = (plane_z - z0) / (z1 - z0)` with `0 < t < 1`. The emitted Z is `plane_z`, not a recomputed Z. X and Y are `p0 + t * (p1 - p0)`.
- An endpoint on the plane is emitted as that point. A parallel segment off the plane emits nothing.
- Consecutive points with equal X, Y and Z bits are collapsed.
- Zero points returns `ContourError::Empty`. One point is `Point`. Two or more points stay the original kind when every input point was on the plane. A cut that is not entirely on the plane is `OpenPlanar`. A closed contour is not closed again by the cut.
- Non-finite `plane_z` or a non-finite coordinate returns `ContourError::Frame` and produces no contour.
- Labelmap edge projection is outline without fill. It reuses `present_labelmap` with `LabelStyle { fill: false, outline: true }`. The caller's fill flag is ignored. Outline alpha stays 255. Segment 0 stays transparent. Interior labelled pixels stay transparent.
- The overlay shader stays the F-063 copy. No new WGSL. `pipeline_compiled` for `SegOverlay` plus `LabelmapEdgeProjection` stays false.
- `show_contour` and `show_surface` still return `BlendUnavailable` for this blend. Slab and volume still have no body for it. They report unavailable. They do not draw composite in its place.
- MIP, MinIP and average on a segmentation overlay stay unavailable.

## Problem and reproducer

`BlendMode::LabelmapEdgeProjection` has no body. `show_labelmap` on that blend returns `BlendUnavailable`. A contour whose points leave the draw plane is refused by `present_contour` and nothing computes the crossing.

The expected observation is two results. A segment from `(0, 0, 0)` to `(0, 0, 4)` cut by `plane_z = 2` is the point `(0, 0, 2)`. A 3 by 3 label of segment 1, drawn with fill requested, uploads only the boundary under the edge blend. The centre pixel is transparent. The boundary is `(10, 20, 30, 255)`.

## Scope, API and compatibility

In scope:

- `intersect_contour(contour, plane_z) -> Result<Contour, ContourError>` in `ocelli-seg`.
- `show_labelmap` accepts `LabelmapEdgeProjection` and presents outline only.
- The existing edge device test stops expecting `BlendUnavailable` for that show. The foreign stack still returns `NotOverlay`. The missing colour on the composite viewport still returns `Present(MissingColour)`.

Out of scope:

- No marching cubes, no RTSTRUCT parse, no second shader, no slab body, no volume ray, no thickness other than the existing one-pixel 4-connected outline.
- No change to composite fill, contour stroke, surface coverage, or the registry enum.
- No change to fusion, VOI, the CPU image path, or the oracle producer.

`show_labelmap` keeps the caller `LabelStyle` argument. The edge blend replaces it before `present_labelmap`. Composite still passes the caller style through.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `intersect_contour` | contour point list and `plane_z` | F-064 on-plane bits | one `Contour` or `ContourError` |
| `show_labelmap` edge blend | `present_labelmap` with outline only | representation must be `Labelmap` | overlay bytes, centre clear |
| `show_labelmap` composite | existing presenter | unchanged | fill and outline as F-063 |
| `show_contour` / `show_surface` | existing blend check | edge blend | `BlendUnavailable` |
| slab and volume | existing uncompiled key | edge blend | unavailable, no composite alias |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| any contour | `intersect_contour` | a new value, no viewport change | error returns no contour |
| edge overlay, labelmap representation | `show_labelmap` with fill on | outline bytes uploaded | CPU returns before the grid is read |
| edge overlay | `show_contour` | `BlendUnavailable` | no upload |
| composite overlay | `show_labelmap` | F-063 bytes | edge rule does not apply |
| recovered edge overlay | `install_recovered` then show | texture dropped, blend unchanged | same as F-066 for the enum |

Intersection does not touch the render graph. The edge show uses the existing `store_overlay` commit. Equal outline bytes do not allocate. One `queue.submit` remains the frame commit.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| intersection contour | `intersect_contour` | the returned value | the `Ok` | error allocates no contour the caller can draw |
| edge overlay texture | `store_overlay` | viewport id | the later tick | presenter error uploads nothing |
| caller `LabelStyle` | the caller | not stored | ignored when the blend is edge | a second show with the same grid uploads nothing new when the bytes match |

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Segment `(0,0,0)` to `(0,0,4)` at plane 2 is point `(0,0,2)` | CPU fixture | emitting an endpoint or `t` outside `(0, 1)` | implementation |
| AC2 | Open polyline `(0,0,0)`, `(2,0,2)`, `(4,0,0)` at plane 1 is `(1,0,1)` then `(3,0,1)`, kind `OpenPlanar` | CPU fixture | one point, or a closed kind | implementation |
| AC3 | A contour already on the plane is returned with the same kind and the same points | CPU fixture | dropping the closed kind | implementation |
| AC4 | `-0.0` against plane `+0.0` contributes no point | CPU fixture | treating the signs as one plane | implementation |
| AC5 | A 3 by 3 segment-1 grid under the edge blend matches outline-only `present_labelmap`. Centre is transparent. Boundary is `(10, 20, 30, 255)` even when the caller asked for fill | ignored GPU readback | uploading the composite fill | gpu gate |
| AC6 | `show_contour` on the edge overlay returns `BlendUnavailable` and does not upload | existing contour device test | stroking the contour through the edge blend | gpu gate |
| AC7 | Slab and `pipeline_compiled` still refuse a labelmap-edge pipeline | existing slab and compile tests | compiling composite under the edge key | implementation |
| AC8 | CPU `show_labelmap` returns unavailable before the grid is read | existing tier check, read in review | presenting on the CPU device path | implementation |

## Approach

Add `intersect_contour` beside `present_contour` in `ocelli-seg`. Export it from the crate root. Do not give `ocelli-seg` a wgpu dependency.

In `show_labelmap`, treat `LabelmapEdgeProjection` as a second accepted blend. Build `LabelStyle { fill: false, outline: true }` and pass that to `present_labelmap`. Leave the composite arm on the caller style. Leave every other blend on `BlendUnavailable`.

Do not add a shader. Do not add a `queue.submit`.

Update `labelmap_edge_and_a_foreign_pass_upload_nothing` so the edge show's readback is the outline fixture. Keep the foreign-pass and missing-colour assertions.

## Dependencies, write ownership and resources

Depends on F-064, which is done. F-063's presenter is reused and not rewritten except by the style the edge blend passes in.

Writers: `crates/ocelli-seg/src/contour.rs`, `crates/ocelli-seg/src/lib.rs`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/tests/labelmap.rs`, and the seg contour tests. GPU tests stay serial. No shader file.

## Verification and evidence reuse

Focused checks are `bin/ocelli.sh test ocelli-seg --test contour` and `bin/ocelli.sh test ocelli-render --test labelmap`. The edge readback is an ignored test and runs under `bin/ocelli.sh gate gpu`, which the sprint profile includes. Feature completion is `bin/ocelli.sh gate --floor` plus the corpus gate. This story does not change CPU candidate bytes. The stored comparison report stays the sprint evidence and is reconfirmed before that record.

A mutation that lets the edge blend call `present_labelmap` with the caller's fill flag must turn the centre pixel into `(10, 20, 30, 128)`.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: the presenter buffer is sized from the grid before the upload, which is the F-063 path
- unsafe: none
- Tier A (WebGPU): full for the overlay outline and for the CPU intersection
- Tier B (WebGL2): the same overlay gate and the same copy shader. No separate device run is claimed on this host
- Tier C (CPU): `intersect_contour` and `present_labelmap` run. `show_labelmap` returns unavailable before the grid is read

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 to AC4, citing the local plane cut and PS3.3 C.8.20.2 for segment colour | `crates/ocelli-seg/tests/contour.rs` |
| gpu | AC5 outline readback, fill ignored | `crates/ocelli-render/tests/labelmap.rs`, ignored |
| unit | AC7 compile refusal stays red for the edge key | existing `seg_overlay_compiles_composite_only` |

## Parity surface covered

The labelmap edge-projection blend, as the overlay outline. The vtk volume ray of the same name is not this row. Slab MIP, MinIP and average stay where they are.

## Deviations

none

## LLD impact

`docs/lld/render-graph.md` and `docs/lld/feature-availability.md` name the intersection and the edge blend. `docs/lld/README.md` if the index row needs the F-ID.

## Open questions

none
