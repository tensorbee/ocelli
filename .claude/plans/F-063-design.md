# F-063, labelmap representation: colour LUT, fill and outline rendering

**Status**: approved
**Epic ref**: E10.1
**Sprint**: S22
**Estimate**: 4w

The operator instructed this run to finish the story. The choices in
`## What the specification does not cover` are local. No material question
remains.

## Normative source, transcribed

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

From `docs/hld/03-architecture-and-crates.md` section 4, the `ocelli-seg` row:

Segmentation state and the three representations. wasm yes. native yes.

From `docs/hld/26-differentiating-capabilities.md` section 31:

Results stay resident on the GPU wherever the consumer is the renderer. A segmentation mask should never round-trip through JavaScript to be drawn.

A kernel with no fallback marks its feature unavailable. It never silently produces a different answer.

From `docs/hld/B-parity-surface.md`:

Segmentation representations, 3, Labelmap, contour, surface.

Blend modes, 5, Composite, MIP, MinIP, average, labelmap edge projection.

DICOM PS3.3 C.8.20.2 Segmentation Image Module. Pixel values of a segmentation image are segment numbers. Pixel value 0 is background and is not a segment number. Segment numbers start at 1.

`crates/ocelli-render/src/graph.rs` already has `Pass::SegOverlay` and `pipeline_compiled` returns false for that kind. `set_voi` returns `PassHasNoUniform` for it. Labelmap edge projection stays an uncompiled blend.

## What the specification does not cover

The HLD names `SegOverlay` and does not define `SegPass`, the colour table, fill, or outline. These choices are made here.

- The grid is row-major `u16` indices. 0 is background. A non-zero value is the segment number.
- The colour LUT is RGBA8, addressed by segment number, starting at segment 1. It is not `VoiParams` and not the F-044 image colour table.
- Fill writes that RGBA8 on every labelled pixel.
- Outline is the 4-connected boundary, one pixel. A neighbour outside the grid counts as background. An outline pixel keeps the segment RGB and uses alpha 255. It replaces the fill at that pixel.
- Cornerstone3D 5.8.2 defaults outline width to 3 and fill alpha to 0.5 inside a VTK actor. That width is not in the HLD. This story does not copy it.
- Both flags off is a refusal. A missing colour for a labelled pixel is a refusal. A bad length is a refusal. A refusal writes nothing.
- The presenter in `ocelli-seg` is the only place that chooses the bytes. The shader copies those bytes. It does not recompute the outline and it does not run the VOI chain.
- Contour, surface, the representation registry and labelmap edge projection stay later stories.

## Problem and reproducer

`ocelli-seg` is the F-001 scaffold. A `SegOverlay` viewport has no pipeline, so a tick does not draw it. There is no label colour table and no fill or outline.

The observation is a 3 by 3 grid of segment 1. The centre pixel is fill RGBA `(10, 20, 30, 128)`. Each edge pixel is outline RGBA `(10, 20, 30, 255)`. A second call with no colour for segment 1 leaves the destination unchanged.

## Scope, API and compatibility

In scope: the label grid, the colour LUT, fill, the one-pixel outline, a CPU presenter, and a composite `SegOverlay` pipeline that copies the presenter bytes.

Out of scope: contour, surface, per-viewport registry, labelmap edge projection, outline width other than one, and any change to `voi.wgsl`.

```rust
pub struct Rgba8 { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }
pub struct LabelColourLut { /* entries from segment 1 */ }
pub struct LabelGrid { pub width: u32, pub height: u32, pub indices: Vec<u16> }
pub struct LabelStyle { pub fill: bool, pub outline: bool }
pub fn present_labelmap(grid: &LabelGrid, lut: &LabelColourLut, style: LabelStyle, out: &mut [u8]) -> Result<(), LabelmapError>
```

`RenderGraph::show_labelmap` uploads those bytes to an `Rgba8Unorm` texture on a `SegOverlay` viewport whose blend is composite. `pipeline_compiled` is true only for `SegOverlay` plus `Composite`. Other blends, including labelmap edge, stay false.

Default style is fill on and outline on. The caller passes the style. There is no hidden default inside the presenter.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Host and tests | `present_labelmap` | Indices, LUT, style, destination | RGBA8 or a refusal with the destination unchanged |
| `RenderGraph::show_labelmap` | `present_labelmap` then one texture upload | Tier A or B, composite `SegOverlay` | Next tick copies those bytes |
| `RenderGraph::tick` | Compiled copy shader | Overlay texture already uploaded | Target matches the presenter |
| `set_voi` on `SegOverlay` | Existing refusal | Any chain | `PassHasNoUniform` |
| Slab labelmap edge | Existing uncompiled key | `BlendMode::LabelmapEdgeProjection` | Unavailable, composite is not substituted |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Empty destination | Valid grid, LUT and style | Destination becomes the presenter bytes | Not a refusal |
| Destination holding sentinels | Missing colour, bad length, or both flags off | Destination unchanged | `LabelmapError` |
| `SegOverlay` with no upload | `tick` | Stack viewports still encode. This viewport is not ready | No overlay texture is invented |
| Uploaded overlay | Same bytes again | No new texture and no new dirt | Last image stays |
| Uploaded overlay | New bytes | Texture replaced and data dirt set | Previous binding dropped only after the new texture exists |
| `SegOverlay` | `set_voi` | No uniform write | `PassHasNoUniform` |

The label index 0 is never coloured, even when a caller might wish entry 0 were a colour. The LUT has no entry 0.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Presenter bytes | Caller buffer | Grid size and style | `present_labelmap` returns `Ok` | `Err` does not write |
| Overlay texture | `show_labelmap` after a successful present | Width, height and byte stamp | Slot binding replaced, then data dirt set | A present error uploads nothing |
| Pipeline | `RenderGraph::compile` | Pass, composite, tier | Graph construction | Not compiled mid-frame |

Device recovery already drops per-viewport GPU objects. The next `show_labelmap` uploads again. Equal bytes after a successful upload do not allocate.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | 3 by 3 segment 1, fill and outline on. Centre is `(10, 20, 30, 128)`. An edge is `(10, 20, 30, 255)`. | `labelmap_fill_and_outline_use_the_segment_colour` | Outline alpha left at 128, or the VOI grey used as the colour | implementation |
| AC2 | Outline off. The edge keeps alpha 128. | `outline_off_keeps_the_fill_alpha` | Border forced to 255 | implementation |
| AC3 | Fill off. The centre is transparent. The edge stays opaque. | `fill_off_draws_only_the_boundary` | Interior still filled | implementation |
| AC4 | Label 2 with only segment 1 in the LUT. Output sentinels stay. | `a_missing_colour_writes_nothing` | Label painted black or as segment 1 | implementation |
| AC5 | Index length or output length wrong, or both flags off. Sentinels stay. | `a_refusal_writes_nothing` | A short buffer is partly written | implementation |
| AC6 | Background 0 stays transparent beside a labelled neighbour. | `background_is_not_a_segment` | LUT entry 0 or the neighbour colour paints the zero | implementation |
| AC7 | `pipeline_compiled` is true for `SegOverlay` composite and false for labelmap edge. | `seg_overlay_compiles_composite_only` | Edge aliasing composite | implementation |
| AC8 | GPU readback of a shown labelmap equals `present_labelmap`. `set_voi` still returns `PassHasNoUniform`. | `shown_labelmap_matches_the_presenter` | Shader runs the image VOI or recomputes a different outline | gpu gate |
| AC9 | A `SegOverlay` with no upload does not add a pass, and the existing stack tick still submits one viewport. | existing `graph.rs` device row | Compiling the pipeline makes an empty overlay draw the stack shader | gpu gate |

## Approach

Add `present_labelmap` in `ocelli-seg`. `ocelli-render` depends on that crate and uploads the bytes. The new shader `labelmap.wgsl` samples the overlay and writes the texel. `compile` builds that pipeline for composite on the graph tier. `show_labelmap` presents, uploads, and marks data dirty. `tick` binds that group only for `SegOverlay`.

Rejected: running the outline test in WGSL as a second copy. Rejected: indexing the image VOI LUT by the segment number. Rejected: outline width 3 from the VTK actor.

## Dependencies, write ownership and resources

F-041 is done. This story does not edit `voi.wgsl`.

Files: `crates/ocelli-seg`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs`, `crates/ocelli-render/shaders/labelmap.wgsl`, `crates/ocelli-render/Cargo.toml`, the new render test, sprint ledgers and the LLD rows named below.

One story. Serial, in the canonical worktree. The GPU gate stays serial.

## Verification and evidence reuse

Focused checks: `bin/ocelli.sh test ocelli-seg` and `bin/ocelli.sh test ocelli-render --test labelmap` plus the existing graph test. Clippy on both crates.

Completion uses the feature floor plus corpus. The sprint profile is the later consolidated run. The stored comparison report stays valid only while CPU candidate bytes and the oracle producer stay unchanged. This story does not edit those paths. Reconfirm the report hash before the sprint record.

A mutation that writes the fill alpha on an outline pixel must fail AC1. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The overlay texture is created in `show_labelmap`, which is not the tick.
- unsafe: none
- Tier A (WebGPU): full. Composite overlay copies the presenter bytes.
- Tier B (WebGL2): full. The same fragment shader. No compute.
- Tier C (CPU): full through `present_labelmap`. `RenderGraph::new` already refuses tier C, so there is no GPU image on that tier.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Hand-computed RGBA for the 3 by 3 segment, citing PS3.3 C.8.20.2 for background 0 and this plan for fill and outline alpha | `crates/ocelli-seg/tests/labelmap.rs` |
| unit | Refusals leave the destination unchanged | same file |
| unit | Composite compiles and labelmap edge does not | `crates/ocelli-render/tests/labelmap.rs` |
| gpu | Readback matches the presenter and `set_voi` still refuses | same file, ignored, gpu gate |

## Parity surface covered

Segmentation representations, the labelmap row only. Contour and surface stay uncovered. Labelmap edge projection stays uncovered.

## Deviations

none

## LLD impact

`docs/lld/render-graph.md`, `docs/lld/feature-availability.md`, and `docs/lld/README.md` if the index row needs the F-ID.

## Open questions

none
