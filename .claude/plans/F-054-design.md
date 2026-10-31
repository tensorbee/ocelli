# F-054, Thick slab with composite, MIP, MinIP and average blend modes

**Status**: approved
**Epic ref**: E8.4
**Sprint**: S20
**Estimate**: 4w

Depends on F-052, which is `done`. Layer is Rust. `docs/sprints/allocation.json` has an empty `notes` cell for this row.

## Normative source, transcribed

The source of the blend sentence and of the compute sentence uses an em dash. This section does not copy that character. The tier sentence joins two clauses with a semicolon. This section uses a full stop there. The words are otherwise the source words.

### `docs/hld/05-rendering.md`, section 7

**Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs. The tier resolves once at startup.

**Volume rendering runs on both tiers**, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute, GPU segmentation, histogram passes, compute-based resampling, is tier A only and must degrade, not fail.

**Blend modes are shader variants**, composite, MIP, MinIP, average, selected by specialisation constant rather than branching per fragment.

### `docs/hld/19-render-graph.md`, section 22

**Pipelines compile at init**, keyed by (pass kind, blend mode, tier). Never compile a shader mid-frame.

**One queue.submit() per frame** across all viewports, from the render worker's requestAnimationFrame.

**Dirty tracking.** A viewport re-renders only when camera, VOI, slice, segmentation or data changed. Everything else is a no-op frame.

### `docs/hld/B-parity-surface.md`, blend-mode row

| Surface | Count | Notes |
|---------|-------|-------|
| Blend modes | 5 | Composite, MIP, MinIP, average, labelmap edge projection |

Labelmap edge projection is on this row. It is not this story.

### `crates/ocelli-render/src/graph.rs`, `BlendMode`

The comment on the enum:

```text
The five blend modes the parity surface counts. Only Composite has a
shader body in this story. The rest are cache keys, and a lookup for one
reports unavailable rather than substituting composite, which is deviation
D-07's honesty rule applied to a blend mode.
```

The variants are `Composite`, `Mip`, `MinIp`, `Average`, `LabelmapEdgeProjection`.

`pipeline_compiled` is true only when the blend is `Composite` and the pass is `Stack` or `VolumeRaycast`. `a_volume_mip_is_not_the_composite_ray` locks `VolumeRaycast` plus `Mip` as uncompiled. A lookup of that key is `FrameOutcome::Unavailable`. It does not return the composite ray.

### `tools/oracle/volume-params.json`

```json
"orientations": ["AXIAL", "SAGITTAL", "CORONAL"],
"blendMode": "COMPOSITE",
"blendModeWhy": "cornerstone3D's own default for an ORTHOGRAPHIC viewport, recorded rather than assumed. A maximum-intensity or average blend over a slab would fold several slices into one pixel and hide exactly the through-plane placement this story exists to expose.",
"slabThicknessMm": null,
"slabThicknessWhy": "Not set, so the viewport keeps cornerstone3D's own default and the reformat is a single-plane cut. Recorded as null rather than omitted, because a slab thickness nobody declared is a parameter a reader would have to go and find."
```

The reference slab stays null. A slab would fold several slices into one pixel. This story does not change that file.

### What this transcription does not contain

Section 22's `Pass` enum lists `Stack`, `VolumeRaycast` and `SegOverlay`. It does not define a slab thickness, a sample step, or a blend integral. DICOM PS3.3 does not define this slab either. `SpacingBetweenSlices` and `SliceThickness` are not the step. `crates/ocelli-volume/src/lib.rs` already says spacing comes from the projected `ImagePositionPatient` gap, DICOM PS3.3 C.7.6.2.1.1, and those two tags are not read.

## What the specification does not cover

The HLD does not define the slab integral. These choices are the ones this plan makes. Composite is the centre sample only. MIP, MinIP and average reduce the stored container values and then apply the existing VOI once.

1. **A slab is a plane plus a thickness.** The plane is an orthographic plane from F-052, or the same plane frame filled with an oblique normal when F-053 has one. Thickness is millimetres along the unit patient normal. The centre sample is `sample_at` at the plane origin. Further samples are the same level-0 trilinear `sample_at`, already private on `Volume` in `crates/ocelli-volume/src/ortho.rs`. There is no second kernel.

2. **The step is the volume spacing along that normal.** Let `M` map an index delta to patient millimetres. Column `i` of `M` is `spacing[i]` times `direction.col(i)`. Let `n` be the unit patient normal. Let `d = M^{-1} n`. The step in millimetres is `1 / length(d)`. The index displacement of one sample is `d / length(d)`, whose length is 1. On an axis-aligned normal this is that axis's spacing. Axial uses `spacing[2]`. Sagittal uses `spacing[0]`. Coronal uses `spacing[1]`. It is never `SpacingBetweenSlices` and never `SliceThickness`.

3. **Which samples fall inside the thickness.** Integer `t` is included when the absolute value of `t * step_mm` is less than or equal to `thickness_mm / 2`. `t = 0` is the centre. Samples run from the most negative `t` to the most positive `t`. Thickness 0 includes only the centre. A non-finite or negative thickness returns `VolumeError::SlabThickness` and allocates no image. A zero or non-finite normal returns `VolumeError::SlabNormal` and allocates no image.

4. **Absence wins for every compiled mode.** If any included `sample_at` is `PlaneSample::Absent`, the output pixel is `Absent` for composite, MIP, MinIP and average. The missing sample is not dropped. It is not stored zero. It is not blended against zero. A step off the grid is `Absent`, because that is what `sample_at` already returns.

5. **`LabelmapEdgeProjection` stays unavailable.** It is not compiled for the slab pass. A lookup does not return the composite pipeline and does not draw composite pixels under that name.

6. **The slab is its own pass kind.** Section 22 keys a pipeline by pass kind, blend mode and tier. `PassKind::Slab` and `Pass::Slab` carry that key. `VolumeRaycast` plus `Mip` stays uncompiled, so the F-059 test above stays true. Stack `Mip` stays uncompiled. This is not a deviation row. The HLD never defined a slab pass, and it did not say the three listed kinds are the last kinds a later key may use.

7. **Four slab pipelines compile at init.** `Composite`, `Mip`, `MinIp` and `Average` compile beside each other for `PassKind::Slab` on tier A and tier B. The WGSL `override` named `slab_blend` is the specialisation constant. wgpu 30.0.1 sets it through `PipelineCompilationOptions.constants`. The pinned struct is in `wgpu-30.0.1/src/api/common_pipeline.rs`. The key is the identifier `slab_blend` because the declaration has no `@id`. The values are 0, 1, 2 and 3 for those four modes. The blend is not a uniform and not a branch per fragment. A lookup of an uncompiled mode, including labelmap edge, returns unavailable and does not substitute `Composite`.

8. **The fragment shader sees host samples.** The host walks `sample_at` when the slab command is accepted. Present samples are uploaded as a non-filterable 3D float texture, `textureLoad` only. An absent pixel is a mask texel of 0. The value texture is not filled with 0 for that pixel. The fragment discards on mask 0 before it loads a value. There is no storage buffer and no compute entry.

9. **Tier A and tier B use that fragment shader.** Tier C reports unavailable and does not present a CPU image. That is D-07. `slab_samples` remains host arithmetic for the fixture, in the same way `reslice` is not the tier C viewport. `show_slab` on tier C returns before any display buffer exists. F-X004 owns the CPU viewport and is not this story.

10. **One `queue.submit()` per frame.** The slab viewport is encoded into the same submission as the other dirty viewports. The tick does not allocate. The sample buffer and the 3D texture are acquired when the slab command is accepted. A depth above `caps.max_tex_3d` is unavailable, not a truncated slab and not a switch to composite. Section 7's floors are 2048 on tier A and 256 on tier B. The code reads the device limit the brick path already stores.

11. **The slab pass loads the colour target.** The stack pass still clears. The slab pass uses `wgpu::LoadOp::Load`. An absent pixel discards, so a previous non-zero colour stays. A clear to black would paint the absent pixel as 0, which this plan rejects.

12. **The reference oracle file is not the fixture.** `tools/oracle/volume-params.json` stays as transcribed. `slabThicknessMm` stays null. `blendMode` stays `COMPOSITE`. Orientations stay `AXIAL`, `SAGITTAL`, `CORONAL`. This story's numbers are hand-computed. They are not an oracle tolerance change. Decision D7 still holds. There is no Ocelli-versus-cornerstone slab diff in this story.

13. **No `wasm-bindgen`, no `unsafe`, no pixels across the worker boundary.** `ocelli-render` remains the only crate that creates a device.

The composite integral of a thick slab is the centre sample only. MIP, MinIP and average reduce stored container values and then apply the existing VOI once. Both decisions were taken when the design round was declined on 2026-10-31.

## Problem and reproducer

`Volume::reslice` samples one plane. `BlendMode::Mip`, `MinIp` and `Average` have no shader body. A lookup returns unavailable rather than composite, which is the correct refusal today and the wrong long-term behaviour for a slab the caller asked for. Nothing walks a thickness along the plane normal. A viewer that answered MIP with the centre composite pixel, or that stepped by `SpacingBetweenSlices`, would show a plausible image of the wrong mode or the wrong plane.

The fixture volume is built the way `crates/ocelli-volume/tests/ortho.rs` builds one. Identity direction. Origin `(10, 20, 30)`. No DICOM tag is read, so a `SpacingBetweenSlices` value cannot enter unless the walker invents one.

Axial. Spacing `[0.5, 1.0, 2.5]`. Dims `[1, 1, 3]`. Stored samples at `(0, 0, k)` are 0, 200, 0. Plane `OrthoPlane::Axial { slice: 1.0 }`. Thickness 5.0 mm. Half-thickness is 2.5 mm. Step is `spacing[2]`, which is 2.5 mm, not 5.0 mm. Included `t` is -1, 0, 1. Index `k` is 0, 1, 2. Samples are `Present(0)`, `Present(200)`, `Present(0)`. World of index `(0, 0, 1)` is `(10, 20, 32.5)`, from PS3.3 C.7.6.2.1.1 plus the volume's third axis. A step of 5.0 mm includes only `t = 0` and drops the stored 0 samples.

Sagittal. Spacing `[4.0, 2.0, 2.5]`. Dims `[3, 1, 1]`. Stored samples at `(i, 0, 0)` are 0, 200, 0. Plane `OrthoPlane::Sagittal { column: 1.0 }`. Thickness 8.0 mm. Step is `spacing[0]`, which is 4.0 mm, not `spacing[2]`. Side samples are the stored voxels at `i = 0` and `i = 2`, value 0. A step of 2.5 mm lands at index `0.375` and `1.625` and blends, so the side sample is not the stored 0.

Oblique. Spacing `[1.0, 1.0, 4.0]`. Identity direction. Unit normal `(0, 0.6, 0.8)`. `d = (0, 0.6, 0.2)`. `length(d)` squared is `0.36 + 0.04 = 0.4`. Step is `1 / sqrt(0.4)`, which is `sqrt(2.5)`, not 4.0. Index step is `(0, 0.6 / sqrt(0.4), 0.2 / sqrt(0.4))`.

Absence. The axial volume with slice 0 not present. The `t = -1` sample is `Absent`. The output pixel is `Absent` for composite, MIP, MinIP and average. It is not `Present(0)` and not the MIP of the samples that remain.

Current behaviour: `slab_samples` does not exist. A MIP pipeline key is uncompiled for every pass kind.

Source identity: `Volume::sample_at` in `crates/ocelli-volume/src/ortho.rs`, `BlendMode` and `RenderGraph::pipeline_compiled` in `crates/ocelli-render/src/graph.rs`, and `slabThicknessMm` in `tools/oracle/volume-params.json`.

MIP of stored 0 and 200 is display 200. MinIP is display 0. The stored average 100 is display 100. The window is LINEAR_EXACT, centre 127.5, width 255, output 0 to 255, PS3.3 C.11.2.1.3.2. Composite of stored 0, 200, 0 is the centre sample, display 200. The front-to-back colour `(200/255)*0.5*0.5` with alpha 0.875 is not composite. The fixture file is `crates/ocelli-volume/tests/fixtures/f054_mip_stored_0_200.txt`.

## Scope, API and compatibility

In scope:

- `slab_plane` and `slab_samples` in `ocelli-volume`, calling the existing `sample_at`.
- `PassKind::Slab`, `Pass::Slab`, four fragment pipelines compiled in `RenderGraph::compile`.
- `RenderGraph::show_slab` on tier A and tier B.
- Host fixtures for the step, the world point and absence.
- A GPU readback that an absent pixel is not painted 0, once a device is present.
- The fixture path for the MIP number, filled only after question 2 is answered.

Out of scope:

- Labelmap edge projection as a drawn mode.
- A full-volume MIP, MinIP or average. `VolumeRaycast` stays composite-only.
- Changing `tools/oracle/volume-params.json` or any oracle tolerance.
- F-053's camera or its plane fit. This story only accepts a plane frame.
- F-055 slice sync, F-056 fusion, F-060 transfer functions.
- A CPU image. F-X004.
- A second trilinear kernel in WGSL.
- `wasm-bindgen` outside `ocelli-wasm`. There is no new export.

`SlabPlane` fields: `origin_index`, `normal`, `column` and `row`, each three `f64` values. `width` and `height` are `u32`. `column_mm` and `row_mm` are `f64`.

`SlabImage` fields: `width` and `height` as `u32`, `step_mm` as `f64`, `index_step` as three `f64` values, `sample_count` as `u32`, `samples` as `Vec<PlaneSample>`.

`Volume::slab_plane` takes `OrthoPlane` and returns `Result<SlabPlane, VolumeError>`. `Volume::slab_samples` takes the plane and `thickness_mm` and returns `Result<SlabImage, VolumeError>`.

`samples` is x-fastest. Each pixel owns `sample_count` entries, from negative `t` to positive `t`. `index_step` is the index displacement of `t = 1`.

Ortho output size matches `reslice`. Axial width and height are `dims[0]` and `dims[1]`. Sagittal width and height are `dims[1]` and `dims[2]`. Coronal width and height are `dims[0]` and `dims[2]`. Column and row millimetres match that same table in the F-052 plan. An oblique `SlabPlane` keeps the width, height and in-plane spacing the caller set. This story does not invent them.

An ortho index outside `[0, dim - 1]` is the existing `VolumeError::PlaneOutOfRange` and allocates nothing.

```rust
pub struct SlabPass {
    pub blend: BlendMode,
    pub voi: VoiParams,
}

pub enum SlabError {
    TierCUnavailable,
    BlendUnavailable,
    ExtentUnavailable,
    Upload(UploadError),
}
```

`show_slab` takes the host `SlabImage`, the viewport id and the blend. `BlendUnavailable` covers `LabelmapEdgeProjection` and any key `pipeline_compiled` rejects. `ExtentUnavailable` covers a sample count above `caps.max_tex_3d`. Neither binds the composite pipeline.

A column that contains `Absent` does not become a stored 0 in the value texture. The mask texel is 0. The fragment discards. An all-present column uses the centre sample for composite and the stored reduction then one VOI for MIP, MinIP and average. No host production reducer is added beside the shader.

Units: thickness and `step_mm` are millimetres. Indexes are voxels. `Present(f32)` is the stored container number, the same meaning as `reslice`. It is not a Hounsfield unit and not a display code. Display codes exist only after the one VOI that follows the stored reduction.

Rounding: `sample_at` already weights in `f64` and narrows once. This story does not add a second rounding. The step length is the `f64` formula above, compared by exact bits against the hand expression in the test.

Compatibility: `show_ortho` stays a single plane. The volume ray stays composite. `tools/oracle/volume-params.json` stays byte-identical. A slab command does not change the stack pass's clear.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `Volume::slab_samples` | `sample_at` on level 0 | Any host. Stored values. No LUT | `SlabImage` or `SlabThickness`, `SlabNormal`, `PlaneOutOfRange` |
| `Volume::slab_plane` | `OrthoPlane` plus `spacing` and `direction` | Axial, sagittal, coronal only | `SlabPlane` whose step axis is the held axis |
| F-053 oblique caller, later | The same `SlabPlane` fields | Not built here. Caller supplies the frame | Same sampler. No second patient frame |
| `RenderGraph::show_slab` | Slab pipelines, mask, 3D sample texture | Tier A or B. Command time, not the tick | One encode in the shared submit, or `SlabError` with no composite substitute |
| `RenderGraph::pipeline_compiled` | Keys compiled in `compile` | Slab plus the four modes is true. Labelmap is false. Volume ray MIP is false | Unavailable does not alias composite |
| `show_ortho` and the volume ray | Unchanged | Single plane, composite ray | No slab thickness applied to those paths |
| `tools/oracle/volume-params.json` | Unchanged file | Reference blend `COMPOSITE`, thickness null | Not a consumer of this slab |
| F-X004 | `slab_samples` later, if it chooses | Not called by `show_slab` | No tier C image |

The owner of the step is `slab_samples`. The owner of absence is `sample_at`, and the slab does not override it. The owner of the blend arithmetic, once chosen, is the slab fragment shader. The owner of the VOI formulas remains `ocelli-pixel` and `voi.wgsl`. This story does not edit those formulas.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Complete volume | Axial, sagittal or coronal slab with a finite thickness inside the grid | `SlabImage`. Every included sample `Present`. `step_mm` is the axis spacing | Volume bytes unchanged |
| Volume with an absent slice inside the thickness | Any of the four compiled modes | That output pixel is `Absent` | No zero stored. No partial reduction |
| Any volume | Negative, NaN or infinite thickness, or a zero normal | `SlabThickness` or `SlabNormal` | No image allocated |
| Ortho index outside `[0, dim - 1]` | `slab_plane` | `PlaneOutOfRange` | No image allocated |
| Tier A or B, slab viewport | `show_slab` of a present column, blend Mip, MinIp or Average | The matching pipeline is bound. The composite pipeline is not | A missing body is not filled with composite |
| Tier A or B | `show_slab` with `LabelmapEdgeProjection` | `BlendUnavailable` | Composite pipeline not bound. Last colour unchanged |
| Tier A or B, previous non-zero colour | Slab pixel whose column contains `Absent` | Fragment discards. `LoadOp::Load` | Previous colour stays. It is not cleared to black |
| Tier C | `show_slab` | `TierCUnavailable` | No display buffer allocated |
| Dirty slab viewport plus another dirty viewport | One tick | `submissions` increases by 1 | Not one submit per viewport and not one per sample |
| Clean slab viewport | Tick with no camera, VOI, slice, segmentation or data change | No-op frame | No second upload and no new texture |

`slab_samples` does not mutate the volume. Two calls with the same arguments allocate twice. That is not a cached no-op. The GPU tick of an unchanged slab is the section 22 no-op frame.

An absent pixel must not replace a previously drawn colour. A refused blend must not replace the last usable mode with composite. A failed upload after the mask is staged must not publish a half-written slab. The previous colour remains the visible result.

Invariants that cross consumers: one `sample_at`. The step is volume spacing along the normal. `VolumeRaycast` MIP stays unavailable. Labelmap edge is never drawn as composite. Pixels do not cross the worker boundary.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| `SlabImage` buffer | `slab_samples` | The returned value | `Ok(image)` | Thickness, normal and range errors return no buffer |
| Slab 3D texture and mask | `show_slab`, at the command | Viewport id plus the sample-count generation | The tick's single `queue.submit` | Extent or blend refusal creates neither. A failed upload drops the staged textures and keeps the last colour |
| Four slab pipelines | `RenderGraph::compile` | `PipelineKey` of pass `Slab`, blend, tier | Graph creation and `install_recovered` | An uncompiled key never resolves to the composite pipeline object |
| Colour attachment | Existing viewport target | The viewport id | Store of a present fragment after `Load` | Discard of an absent pixel does not store. Clear-to-black is not used for this pass |

No browser setter. No new device creation. Recovery rebuilds the four slab pipelines inside the existing `compile` path. Textures that belonged to the dead device are dropped with the other per-viewport GPU objects. The caller supplies the slab again. Recovery does not bind composite in place of MIP.

A repeated `show_slab` with equal samples, equal thickness and equal blend does not allocate again and does not increment the upload counter. Equal arguments are not a licence to skip the refusal checks. A dirty flag still encodes on the next tick. The no-op rule applies only after that tick has cleared the dirt.

Cached output is invalid when the plane, the thickness, the blend, the VOI, the level-0 bytes or the presence bits change. Validation of the blend key and the tier happens before any cache return. A refused request does not replace the last usable slab mode.

Sequence: a valid present slab draws, an absent or labelmap request leaves that colour in place, a later valid slab draws again. The same sequence holds after the first successful submit and after `install_recovered`.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Axial fixture above. Step bits equal 2.5. `index_step` is `(0, 0, 1)`. Samples are stored 0, 200, 0. World of `(0, 0, 1)` is `(10, 20, 32.5)`. PS3.3 C.7.6.2.1.1 plus the volume spacing | `axial_slab_steps_by_volume_spacing` | A step of 5.0. `SpacingBetweenSlices` or `SliceThickness` as the gap. Only the centre sample | Implementation. `ocelli-volume` |
| AC2 | Sagittal fixture above. Step bits equal 4.0, not 2.5. Side samples are stored 0 at one column either way | `sagittal_slab_does_not_use_slice_spacing` | Walking `spacing[2]` on a sagittal normal. A blended index of 0.625 | Implementation. `ocelli-volume` |
| AC3 | Oblique fixture above. `step_mm` bits equal `1 / sqrt(0.4)`. `index_step` is `(0, 0.6 / sqrt(0.4), 0.2 / sqrt(0.4))`. The value is not 4.0 | `oblique_step_follows_the_normal` | Using `spacing[2]` for every normal | Implementation. `ocelli-volume` |
| AC4 | Axial fixture with slice 0 absent. The pixel is `Absent` for a column that would otherwise feed composite, MIP, MinIP and average | `an_absent_slab_sample_is_not_zero` | Painting 0. Dropping the absent sample and reducing the rest. Blending against zero | Implementation. `ocelli-volume` |
| AC5 | `PipelineKey` slab plus `Mip`, `MinIp` and `Average` is compiled on tier A and tier B. Each pipeline object differs from slab `Composite`. `VolumeRaycast` plus `Mip` is not compiled. Stack plus `Mip` is not compiled | `slab_mip_is_not_the_composite_pipeline` | Returning the composite pipeline for a MIP request. Compiling full-volume MIP | Implementation. `ocelli-render` |
| AC6 | `LabelmapEdgeProjection` on the slab pass. `pipeline_compiled` is false. `request_outcome` is `Unavailable`. `show_slab` returns `BlendUnavailable` and does not bind composite | `labelmap_edge_is_not_drawn_as_composite` | Drawing composite under the labelmap name | Implementation. `ocelli-render` |
| AC7 | Tier C `show_slab` returns `TierCUnavailable` before the image is read. No display buffer is allocated | `slab_is_unavailable_on_tier_c` | A CPU image from this entry point | Implementation. `ocelli-render` |
| AC8 | One dirty slab viewport and one dirty stack viewport. One tick. `submissions` increases by 1 | `one_submit_for_a_slab_and_a_stack` ignored, `gpu` gate | A submit per viewport or per sample | Integrated. GPU |
| AC9 | GPU. Draw a non-zero colour, then a slab pixel whose sample is `Absent`. Readback of that pixel stays the previous colour. The pass load op is Load | `an_absent_slab_pixel_is_not_painted_zero` ignored, `gpu` gate | `LoadOp::Clear` to black. Uploading 0 and blending it | Integrated. GPU readback |
| AC10 | MIP of stored 0 and 200 under LINEAR_EXACT centre 127.5 width 255 is display 200. MinIP is 0. Average of stored 0 and 200 is 100, then the same VOI leaves 100. PS3.3 C.11.2.1.3.2 | `crates/ocelli-volume/tests/fixtures/f054_mip_stored_0_200.txt` | A number copied from a VOI-then-reduce path that only agrees on this pair. Composite's pixel used as the MIP code | Implementation. `ocelli-volume` |
| AC11 | Thick composite of stored 0, 200, 0 in normal order is the centre sample, display 200. It is not colour `(200/255)*0.5*0.5` and it is not alpha 0.875 | `crates/ocelli-volume/tests/slab.rs` `composite_is_the_centre_sample` | Treating the ray accumulation as composite | Implementation. `ocelli-volume` |
| AC12 | Thickness 0 returns one centre sample. A NaN thickness and a zero normal allocate nothing | `slab_request_refusals` | Clamping a bad thickness to one spacing and still drawing | Implementation. `ocelli-volume` |

AC8 and AC9 need an adapter. A missing adapter does not mark them passed. AC10 and AC11 stay open criteria until the questions are answered. A compiling pipeline is not those rows.

Semantic RED for AC1: a walker whose step is 5.0 returns one sample. The fixture requires three. Semantic RED for AC5: binding the composite pipeline on a MIP key. That is the substitution D-07's honesty rule already forbids.

## Approach

`sample_at` becomes `pub(crate)` in `crates/ocelli-volume/src/ortho.rs`. New `crates/ocelli-volume/src/slab.rs` calls it. `slab_plane` builds the ortho frame with the same axis mapping as `reslice`. The oblique path does not construct a camera. It samples the `SlabPlane` it was given.

`M` is the `f64` matrix whose columns are `spacing[i] * direction.col(i)`. Invert it, multiply by the unit normal, and take the length. A singular direction is `SlabNormal` rather than a guessed axis. The implementation uses the existing `glam::DMat3` already stored on `Volume`.

New `crates/ocelli-render/shaders/slab.wgsl` includes the existing VOI module once. It declares `override slab_blend: u32`. It samples a `texture_3d<f32>` and a mask with `textureLoad`. It has no `var<storage` and no `@compute`. `RenderGraph::compile` builds four fragment pipelines from that module, one constant each, on tier A and on tier B. `pipeline_compiled` returns true for those four slab keys and stays false for labelmap and for `VolumeRaycast` MIP.

`show_slab` follows `present_plane`: tier C returns before any sample is read. The colour attachment for this pass uses `LoadOp::Load`. Present pixels are drawn. Absent pixels discard.

Composite selects the centre sample. MIP, MinIP and average reduce the stored samples along the slab and then apply the existing VOI once. Filling one mode with the other is the defect AC5 and AC11 exist to catch.

Rejected alternatives:

- Reuse `PassKind::VolumeRaycast` for the slab. That either leaves MIP uncompiled or breaks `a_volume_mip_is_not_the_composite_ray`.
- Reduce on the host and upload one plane through the stack shader. The four pipelines would share one body, and the blend would not be a specialisation constant.
- A WGSL trilinear walk over bricks. A second kernel the `sample_at` fixture can drift from. F-052 already rejected that split.
- Treat an absent corner as 0 so the MIP still returns a number.
- Step by a caller-supplied gap that stands in for `SpacingBetweenSlices`.
- Draw labelmap edge as composite until its own story exists.
- Present `slab_samples` as the tier C image.
- Edit `volume-params.json` so the reference slab is non-null. That folds slices in the oracle and is a tolerance-class change.

No new trait. No new generic. No `Box<dyn>`. The slab pass is a concrete `Pass` variant, which is the key section 22 already required.

## Dependencies, write ownership and resources

Declared dependency: F-052, done. F-053 is not a dependency. The oblique frame is an input, not a call into F-053. Source-level dependency: `sample_at`, `OrthoPlane`, `PlaneSample`, `RenderGraph::compile`, `VoiParams`, `caps.max_tex_3d`.

Safe order: volume sampler and host tests first, then the pass kind and the pipeline keys, then the shader and `show_slab`. The shader arithmetic waits until this plan is approved, which requires the two questions to be answered.

Writes:

- `crates/ocelli-volume/src/ortho.rs` (`pub(crate)` on `sample_at` only)
- `crates/ocelli-volume/src/slab.rs`
- `crates/ocelli-volume/src/lib.rs` (module, errors, exports)
- `crates/ocelli-volume/tests/slab.rs`
- `crates/ocelli-volume/tests/fixtures/f054_mip_stored_0_200.txt` only after question 2 is answered
- `crates/ocelli-render/shaders/slab.wgsl`
- `crates/ocelli-render/src/slab.rs`
- `crates/ocelli-render/src/graph.rs`
- `crates/ocelli-render/src/lib.rs`
- `crates/ocelli-render/tests/slab.rs`
- `docs/lld/volumes.md`, `docs/lld/render-graph.md` and `docs/lld/viewport.md` at completion

`graph.rs` and `ortho.rs` cannot have concurrent writers. Other S20 stories must not edit them in parallel with this one. GPU tests are exclusive with any other story that opens an adapter. Serial wave, canonical worktree.

Do not write `tools/oracle/volume-params.json`. Do not write `docs/hld/DEVIATIONS.md`.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-volume
bin/ocelli.sh test ocelli-volume --test slab
bin/ocelli.sh check ocelli-render
bin/ocelli.sh test ocelli-render --test slab
bin/ocelli.sh clippy ocelli-volume
bin/ocelli.sh clippy ocelli-render
```

Ignored GPU tests run under the `gpu` gate at completion. The sprint profile runs them again after integration. A missing adapter leaves AC8 and AC9 unmeasured. It does not pass them.

`git diff -- tools/oracle/volume-params.json` must be empty. A non-empty diff invalidates the run. AC1's world point and the three step expressions are the geometry baseline. Changing the step formula invalidates the receipt.

AC10's fixture file is the MIP baseline once question 2 names the code. Until that file exists, AC10 has no receipt. A shader that matches itself is not that receipt.

No new benchmark subject. The performance claim is the existing one-submit counter, not a frame-time threshold. No wasm export is added. A size move at completion is investigated with the wasm-size skill rather than accepted here.

The oracle is not an observer for this story. Reference orientations stay the single-plane cuts.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The sample buffer and the 3D texture are acquired when the slab command is accepted
- unsafe: none
- Tier A (WebGPU): full. Fragment shader, no storage buffer, no compute
- Tier B (WebGL2): full. The same fragment shader, no storage buffer, no compute. A sample count above `max_tex_3d` is unavailable, not a different image
- Tier C (CPU): unavailable. `show_slab` returns `TierCUnavailable` and does not present a CPU image. D-07. F-X004 remains the CPU viewport

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1, AC2 and AC3 step, index and world point. PS3.3 C.7.6.2.1.1 for the world point. The step is the volume spacing formula in this plan | `crates/ocelli-volume/tests/slab.rs` |
| fixture | AC4 absence. An absent sample is not stored 0 | `crates/ocelli-volume/tests/slab.rs` |
| fixture | AC10 MIP display 200, MinIP display 0, average display 100. AC11 composite display 200. PS3.3 C.11.2.1.3.2 | `crates/ocelli-volume/tests/fixtures/f054_mip_stored_0_200.txt` |
| unit | AC5 pipeline identity, AC6 labelmap refusal, AC7 tier C, AC12 bad requests | `crates/ocelli-render/tests/slab.rs` and `crates/ocelli-volume/tests/slab.rs` |
| gpu | AC8 one submit, AC9 absent pixel keeps the previous colour | ignored tests in `crates/ocelli-render/tests/slab.rs` |

AC11 is the centre sample, display 200. It is an acceptance row, not a silent pass.

## Parity surface covered

Appendix B blend modes. This story can establish composite, MIP, MinIP and average as distinct slab pipelines, and it can establish that a thick slab is not the single-plane reference. It does not establish labelmap edge projection. It does not establish a cornerstone pixel diff. The oracle row stays `COMPOSITE` with `slabThicknessMm` null, on orientations `AXIAL`, `SAGITTAL` and `CORONAL`. Decision D7 holds.

## Deviations

None.

## LLD impact

`docs/lld/volumes.md` gains the slab step, the thickness bound, the absence rule and `slab_samples`. `docs/lld/render-graph.md` gains `PassKind::Slab`, the four pipelines, the specialisation constant and the load op. `docs/lld/viewport.md` gains `show_slab`, the tier C refusal and the rule that an absent pixel does not clear the target.

## Open questions

None. The consolidated design round was offered on 2026-10-31 and declined. Composite is the centre sample, display 200 for stored 0, 200, 0 under LINEAR_EXACT centre 127.5 width 255. MIP, MinIP and average reduce stored values and then apply that VOI once, so the same pair is MIP 200, MinIP 0 and average 100. PS3.3 C.11.2.1.3.2. The ray accumulation is not composite.
