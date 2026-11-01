# F-056, Multi-volume fusion with per-volume opacity and colormap (PET-CT)

**Status**: approved
**Epic ref**: E8.6
**Sprint**: S20
**Estimate**: 4w

Depends on F-052, which is done. Layer is Rust. Epic is E8 Volume viewport and MPR. The allocation `notes` cell is empty. This plan is not approved.

## Normative source, transcribed

Quotation. Where a source sentence contains an em dash or a semicolon, that character is not copied, and the sentence is split or given a full stop in its place. The three VOI function listings in HLD section 18.2 are not recopied. This story does not implement them, and the LINEAR listing contains a semicolon.

### `.claude/skills/dicom-expert/SKILL.md`, section 1

`FrameOfReferenceUID` (0020,0052) is what makes coordinates comparable. Two series share a spatial frame if and only if they share this UID. An annotation is only transferable between series that share it. This is the mechanism HLD section 35 relies on for putting a slide layer and a volume layer in one scene.

### `.claude/skills/dicom-expert/SKILL.md`, section 9

Standardised uptake value is **not** part of the LUT chain. It is a separate path and needs the radiopharmaceutical sequence.

```text
activity = stored * RescaleSlope + RescaleIntercept          // Bq/ml
SUVbw    = activity * PatientWeight_kg * 1000 / decayed_dose_Bq
```

with

```text
decayed_dose = RadionuclideTotalDose
             * 2 ^ ( -elapsed_seconds / RadionuclideHalfLife )
```

Inputs: `RadiopharmaceuticalInformationSequence` (0054,0016), `RadionuclideTotalDose` (0018,1074), `RadionuclideHalfLife` (0018,1075), `RadiopharmaceuticalStartDateTime` (0018,1078) or `RadiopharmaceuticalStartTime` (0018,1072), `PatientWeight` (0010,1030), `Units` (0054,1001), `DecayCorrection` (0054,1102), `SeriesTime` (0008,0031).

`Units` must be `BQML`. If it says `CNTS` or `GML`, the formula does not apply and computing it anyway yields a number that looks like an SUV.

SUV is a clinical number a radiologist may act on. If any input is missing, report unavailable. Never substitute a default weight or a default half-life.

### `docs/hld/15-lut-chain.md`, section 18.1

The source sentence has an em dash between "sequence" and "do". That character is not copied. A full stop stands there. "Here" is the modality stage.

If a Modality LUT Sequence is present it wins over slope and intercept. PET SUV is a separate path and needs the radiopharmaceutical sequence. Do not fold it in here.

Section 18's opening instruction has the same character, also not copied. Implement the chain once, in `ocelli-pixel`, and let the shader read the parameters. Do not let a second copy of this logic appear anywhere.

HLD section 18.3, the window-centre row only, soft-tissue centre 40 and width 400: input 40, `LINEAR_EXACT` 127.500. This story uses that worked value as a display value. It does not re-implement the window.

### `docs/lld/volumes.md`

`group_volumes` splits on `FrameOfReferenceUID` and `ImageOrientationPatient` together, in first-seen order. A localiser mixed with axials is two candidates. A second frame of reference is a second candidate. Assembly never re-checks the frame of reference, because slice evidence does not carry one.

`FrameOfReferenceUid` is the `FrameOfReferenceUID` string, compared and never parsed.

The direction matrix is those two validated cosines and that normal, so spacing index 0 steps along the row and index 1 along the column, which is PS3.3 C.7.6.2.1.1.

The origin is the `ImagePositionPatient` of the lowest projected slice, copied exactly. That point is the centre of voxel zero. There is no half-voxel offset.

`Volume` does not store the UID. The candidate does, on `VolumeCandidate.frame_of_reference`.

### DICOM PS3.3 C.7.6.2.1.1

Recorded in the skill, section 6. `X` is the row direction. `Y` is the column direction. `i` is the column index. `j` is the row index. `PixelSpacing` is `[between rows, between columns]`. `PixelSpacing[0]` multiplies the column direction cosine. `PixelSpacing[1]` multiplies the row direction cosine. IPP is the centre of the first voxel.

```text
P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
```

F-052 already extends that with the volume normal and `spacing[2]`. This story calls `Volume::voxel_world`. It does not write a second forward map.

```text
P = origin
    + i * spacing[0] * direction.col(0)
    + j * spacing[1] * direction.col(1)
    + k * spacing[2] * direction.col(2)
```

### `docs/hld/05-rendering.md`, section 7

The tier sentence in the source has a semicolon, and the compute sentence has em dashes. Those characters are not copied.

Two capability tiers, one codebase. Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs. The tier resolves once at startup.

Volume rendering runs on both tiers, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute is tier A only and must degrade, not fail. The source names GPU segmentation, histogram passes and compute-based resampling as that compute work.

One submit per frame. The render graph tracks dirty viewports and issues a single submission across all of them, driven by requestAnimationFrame inside the render worker.

### `docs/hld/DEVIATIONS.md`, D-07

HLD section 7 names two GPU tiers. D-07 adds tier C, CPU. A feature that cannot run on the resolved tier reports unavailable, and never silently produces a different answer. What tier C does not claim is interactive volume ray-casting on the CPU. A CPU path that quietly produces a different image is the outcome D-07 forbids.

This story reports fusion unavailable on tier C. It does not draw a CPU fusion image.

### `docs/hld/19-render-graph.md`, section 22

```rust
pub enum Pass {
    Stack(StackPass),
    VolumeRaycast(VolumePass),
    SegOverlay(SegPass),
}
```

Pipelines compile at init, keyed by pass kind, blend mode and tier. Never compile a shader mid-frame.

A viewport re-renders only when camera, VOI, slice, segmentation or data changed. Everything else is a no-op frame.

One `queue.submit()` per frame across all viewports, from the render worker's requestAnimationFrame.

The device-loss sentence in that section contains an em dash and is not copied. Device loss stays on the existing `GpuContext` recovery. This story does not add a recovery path.

### `docs/hld/03-architecture-and-crates.md`, section 4

`ocelli-pixel` is the LUT chain, photometric interpretation and the frame model. `ocelli-volume` is volume assembly, geometry and reslicing. `ocelli-render` is the wgpu device, the render graph, the WGSL shaders and the backend tiers. `ocelli-wasm` is the only crate that may import `wasm-bindgen`.

### `docs/hld/B-parity-surface.md`, Appendix B

The appendix lists viewport types, tool classes, five blend modes, three VOI functions, transfer syntaxes, segmentation representations, events and adapters. It has no fusion row and no PET SUV row. Measured there against cornerstone3D v5.8.9 is D-11's stale version. This story does not establish any of those rows.

## What the specification does not cover

The HLD does not specify a two-volume overlay, a per-volume opacity, a caller colour table, or a sample that misses one of the two volumes. The choices below are local. PET is in front: `out = pet * opacity_pet + ct * opacity_ct * (1 - opacity_pet)`. An absent volume adds no colour. The present volume is drawn. The absent sample is not stored zero.

Fusion is two volumes that are already constructed. They are displayed in one orthographic or oblique plane. The plane is a world origin, two in-plane directions, two positive millimetre steps, and an output width and height. Orthographic is that plane built from one volume's axes through the existing `OrthoPlane`. Oblique is the same struct with caller-supplied directions. F-053 is not a dependency and is not waited on. This story does not build an oblique camera.

Both inputs carry a `FrameOfReferenceUid` string, taken from the `VolumeCandidate` that built each volume. `Volume` itself still does not store the UID, and assembly is not changed to re-check it. The two strings must be equal and non-empty. Comparison is the existing string equality. Fusion does not parse the UID and does not trim it. A mismatch returns an error and draws nothing. It does not overlay the volumes anyway. An empty string is a refusal even when both strings are empty. Empty is not a shared frame. The previous presented frame stays in every refusal.

Each volume has an opacity from 0.0 to 1.0 inclusive, and an optional colormap. The opacity must be finite. A value outside that interval, or a non-finite value, is a refusal and draws nothing. The colormap is a caller-supplied table of 256 RGB triples. It is applied to the display value the volume's existing `LutChain` already produced. It is not a DICOM palette sequence and not SUV. It is not `Colormap::Grey` or `Colormap::Stops`, except that an omitted table uses the grey replication those presets already use for `Grey`: display value `d` becomes RGB `(d, d, d)`. The index into a supplied table is the existing display index, nearest integer with ties up, clamped to the table. This story does not add a second quantiser.

This story does not compute SUV. The formulas above are transcribed so they are not folded into the modality stage and not added beside it. A later caller that wants SUV must supply modality values itself. Missing radiopharmaceutical inputs stay unavailable. This function does not invent `PatientWeight`, a half-life, a dose or a start time. It does not read `Units`. There is no `BQML` branch because there is no SUV branch.

Sampling uses each volume's own origin, spacing and direction, then the shared world point. The forward map is the one transcribed above. The inverse is not in the standard. With the constructed orthonormal columns it is:

```text
delta = world - origin
i = dot(delta, direction.col(0)) / spacing[0]
j = dot(delta, direction.col(1)) / spacing[1]
k = dot(delta, direction.col(2)) / spacing[2]
```

That index is then the existing `sample_at` trilinear sample. A point absent from a volume is `PlaneSample::Absent`. It does not become stored zero, and it is not passed through modality or VOI. The other volume is drawn at that pixel. Hardware linear filtering is rejected, because a border of zero is stored zero.

The composite is `out = pet * opacity_pet + ct * opacity_ct * (1 - opacity_pet)`. Both opacities 1 yield the PET colour. CT `(10, 10, 10)` and PET `(200, 0, 0)` at opacity 1 are `(200, 0, 0)`. At opacity 0.5 they are `(102.5, 2.5, 2.5)`. Sum-then-clamp `(210, 10, 10)` is not this composite. CT-in-front `(10, 10, 10)` is not this composite.

Tier A and tier B share one fragment shader. The shader calls the existing `lut_chain` in `voi.wgsl` and then looks up the colormap. It is not a second VOI implementation. Tier C reports unavailable and produces no fusion image. `wasm-bindgen` is not added. `unsafe` is not added. Pixels do not cross the boundary. The render loop does not allocate. The plane buffers, the two colour tables and the uniform storage are sized when the fusion view or the plane is bound, before the frame. One `queue.submit()` per frame remains the graph tick. LUT arithmetic stays in `ocelli-pixel`.

`Pass` stays the three variants in section 22. Fusion is not a fourth pass and not a slab blend mode. Those modes belong to F-054. The fusion shader is compiled when the fusion view is created, which is init, not mid-frame. The tick encodes that pipeline in the same command encoder as the other viewports and still submits once. A viewport with no fusion binding keeps the stack shader. `raycast.wgsl` is not edited. Its opacity constant belongs to F-060.

A chain that `VoiParams::from_chain` cannot express, including a modality or VOI LUT sequence, refuses the frame. Fusion does not compute that chain on the CPU and present the result. The previous frame stays.

Opacity, colormap, plane and volume changes set the existing data or camera dirty flag. No sixth dirty flag is added. An identical request on a settled frame is the section 22 no-op. It does not allocate and does not submit again.

Two concrete volumes are parameters. There is no fusion trait and no volume list. Registration, a shared voxel grid, elastix, transfer-function editing, lighting and cropping are out of scope. Those are F-179, F-180, F-060, F-061 and F-062. Slice-position sync stays F-055.

## Problem and reproducer

What is absent is a fused view. `group_volumes` already splits a second `FrameOfReferenceUID` into a second candidate. `Volume::reslice` and `present_plane` draw one volume. `raycast.wgsl` walks one volume. Nothing takes two constructed volumes, refuses a UID mismatch, and draws one shared plane.

Trigger: two constructed volumes, the UID string from each candidate, a plane, two opacities in range, and two existing window chains. The shared world point in the acceptance fixture is `(3, 8, 10)` mm.

Expected observation: equal non-empty UIDs produce one presented frame on tier A and tier B. Different UIDs return an error and leave the previous frame's bytes unchanged. Both opacities 1 on CT `(10, 10, 10)` and PET `(200, 0, 0)` yield `(200, 0, 0)`.

Current behavior: no such function exists. Calling the future entry point is a compile failure. That failure does not prove the UID rule or the world-point rule. The tests below are the observation.

Source identity: F-056, E8.6, S20. No patient data. The fixture volumes are synthetic, with the IPP, directions and spacings written in AC5.

## Scope, API and compatibility

In scope: UID refusal, per-volume opacity domain, optional 256-entry colormap after the existing VOI, world-point sampling through each volume's geometry, one shared fragment shader on tier A and tier B, tier C unavailable, previous frame preserved on every refusal.

Non-goals: SUV, palette colour, ICC, a new VOI function, a new `Pass` variant, slab MIP, MinIP, average or labelmap edge, oblique camera UI, registration, resampling both volumes onto one grid, a CPU presented fusion, N-volume fusion, editing `Volume`'s fields, editing `group_volumes`.

Public shape, in `ocelli-render`. Fields are named so a struct literal cannot grow a weight parameter without the AC3 test failing to compile.

```rust
pub struct FusionRequest<'a> {
    pub volume_a: &'a Volume,
    pub frame_a: &'a FrameOfReferenceUid,
    pub volume_b: &'a Volume,
    pub frame_b: &'a FrameOfReferenceUid,
    pub chain_a: &'a LutChain,
    pub chain_b: &'a LutChain,
    pub opacity_a: f32,
    pub opacity_b: f32,
    pub colormap_a: Option<&'a [Rgb]>,
    pub colormap_b: Option<&'a [Rgb]>,
    pub plane: &'a FusionPlane,
}
```

A present colormap slice has length 256. Any other length is a refusal and draws nothing. `FusionPlane` lives in `ocelli-volume` beside the ortho types. `sample_world` on `Volume` returns the existing `PlaneSample`. `ocelli-volume` does not take a colour table, an opacity or a radiopharmaceutical input.

```rust
pub struct FusionPlane {
    pub origin: Pt<World>,
    pub row_dir: (f64, f64, f64),
    pub column_dir: (f64, f64, f64),
    pub column_mm: f64,
    pub row_mm: f64,
    pub width: u32,
    pub height: u32,
}
```

Output pixel `(i, j)` has world point `origin + i * column_mm * row_dir + j * row_mm * column_dir`. That is the in-plane form of PS3.3 C.7.6.2.1.1. The directions are unit vectors. A non-unit, non-finite or zero step is a refusal.

Compatibility: `Volume`, `Pass`, `BlendMode`, `voi.wgsl` and `raycast.wgsl` keep their current public contracts. `present_plane` is not the fusion presenter, because its any-absent refusal would lock question 2. Existing stack and raycast viewports are unchanged when no fusion request is bound.

Refusal policy: tier C, UID mismatch, empty UID, opacity out of range, non-positive plane step, zero output extent, spacing that cannot be inverted, and a chain `VoiParams::from_chain` rejects. Each one returns before encode. None of them clear or replace the published texture.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Fusion presenter in `ocelli-render` | `Volume::sample_world` then existing `sample_at`. `LutChain::map_into` and `VoiParams::from_chain`. Existing display index. Existing graph tick for the one submit | Caller passes two already built volumes, the candidate UID strings, two chains, two opacities, two optional tables, and one plane. Units of opacity are 0.0 to 1.0. Colour index rounds ties up. Fusion does not select a window | One GPU frame on tier A or B, or a refusal that leaves the published texture |
| AC5 geometry fixture | `Volume::voxel_world` and `sample_world` | Synthetic planes. No colormap and no SUV | World point `(3, 8, 10)` and one index per volume |
| AC4 colormap fixture | `LutChain::map_into` then the existing display index | `LINEAR_EXACT`, centre 40, width 400, input 40, display 127.5, index 128 | RGB from table entry 128, not entry 40 |
| Later F-053 oblique caller | `FusionPlane` | Not built here. May supply directions later. Not required to compile F-056 | Same world-point sampler |
| Stack viewport with no fusion binding | Existing stack pipeline | Unchanged | Unchanged stack image |
| Volume raycast | Existing `raycast.wgsl` | Unchanged. Opacity constant stays the F-060 placeholder | Unchanged single-volume ray |

The owner of the world-to-index conversion is `sample_world`. The owner of window arithmetic is `ocelli-pixel`. The owner of the 256-entry index is the existing display-index rule. The owner of publish is the graph tick's successful submit. The fusion presenter does not own a second submit.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| No published frame | Equal non-empty UIDs, opacities in range, tier A or B | Request accepted. Draw waits on questions 1 and 2 for channel values. A later successful submit publishes the first frame | A bad UID publishes nothing |
| Published frame, volumes bound | Same UID pair and a new in-range opacity or plane | Next tick encodes once and submits once. The new frame is visible at that submit | Encode failure before submit keeps the published texture |
| Published frame | Different UID, or either UID empty | Error. Bound volumes stay the previous pair. Published bytes stay | The rejected pair is not stored |
| Published frame | Opacity outside 0.0 to 1.0, or non-finite | Error. Published bytes stay. Bound volumes stay | No clamp-and-draw |
| Published frame | Same request again, nothing dirty | No-op. No allocation. Submit count unchanged | Not a second copy of the same upload |
| Any state, tier C | Any fusion request, including a valid pair | Unavailable. No sample, no upload, no submit, no CPU image | Previous GPU frame, if one existed, stays |
| Published frame | Chain that `from_chain` rejects | Unavailable. No CPU composite is presented | Published bytes stay |
| Sample outside one volume | `sample_world` | `PlaneSample::Absent`, not `Present(0)` | Draw policy for the other volume is question 2 |

Invariants that span the presenter and the fixtures: the UID test is string equality of non-empty values. A world point is converted with the volume being sampled, never with the other volume's spacing. Absence is not a stored zero. The colormap sees a display value, not a stored value. SUV inputs are not parameters and not defaults. A refusal does not replace the last published texture.

Competing route: `present_plane` and the raycast pass do not become fusion. A fusion refusal must not fall through to either of them.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Published fusion texture | Graph viewport, created with the fusion view | Viewport id plus the generation of the last successful submit | The single `queue.submit()` that encodes this view | Refusal or encode failure does not swap the generation. The previous bytes remain |
| Per-volume sample buffers | Fusion view, sized to `width * height` when the plane is bound | Plane width, height and the volume data generation | Not a commit. Upload into the unpublished staging texture | Rebound plane may allocate before the frame. The frame itself does not allocate. A failed request drops staging and keeps the published texture |
| Colormap tables and VOI uniforms | Fusion view, at bind, from the 256 triples and `VoiParams::from_chain` | Table identity and chain identity | Included in the same submit as the frame | Opacity-only change updates the uniform and does not re-upload the volume samples |
| Fusion shader module | Compiled when the fusion view is created | One module shared by tier A and tier B | Init, not the frame | Tier C never compiles it and never selects it |
| Device textures after device loss | Existing `GpuContext` recovery | The rebuilt device generation | The first successful submit after restore | Fusion does not keep a private device. A loss is not reported as a UID mismatch |

Side effects before commit are staging writes into unpublished textures. The published texture is not cleared at the start of a refused request. An identical repeated request on a settled frame does not upload and does not submit. Equal arguments are a no-op only when the dirty flags are clear. A repeated request that the caller marks dirty is a real encode, not a proof that the GPU operation was free.

Validation before an idle return: the bound UID pair, both opacities, both chain identities, both table identities, the plane, and both volume data generations. A refused request does not replace those bound values. Sequence: valid draw, mismatched UID, valid retry. The mismatch leaves the valid frame published and leaves the valid binding in place. The retry draws. The same sequence holds before the first successful submit, where the mismatch still publishes nothing and the retry is the first commit.

The fused RGB, once question 1 is answered, is valid only for that opacity pair, those two chains, those two tables, that plane and those two geometries. A cache hit that ignores any of those is a wrong frame.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Equal non-empty UIDs. Both volumes are sampled at the shared world point and one frame is the fusion result on tier A and tier B. Channel values stay undefined until question 1 | Address fixture plus presenter acceptance. No passing RGB row while question 1 is open | Sampling both volumes with one volume's index. Drawing only the first volume. Treating the request as a single `present_plane` | Floor for the addresses. Adapter readback is not evidence until question 1 has an expected triple |
| AC2 | A published frame, then a different UID. Error. Published bytes unchanged. The new pair is not bound | Present, refuse, read the same texture bytes | Overlaying anyway. Clearing to black. Replacing the bound volumes | `ocelli-render` test. CPU texture stand-in is enough if the refuse path returns before upload |
| AC3 | The fusion request has no `PatientWeight` parameter and no dose, half-life, start time or `Units` field | Struct literal `fusion_request_has_no_patient_weight` | A required weight argument. A default of 70 kg. An SUV helper in the new modules | Compile-fail of that literal if the field is added. `ocelli-render` |
| AC4 | Colormap is applied after VOI. Stored 40, `LINEAR_EXACT` centre 40 width 400, display 127.5, index 128, table 128 is red and table 40 is blue. Result is red | Hand-computed fixture | Indexing the table by the stored sample. A second window inside the colormap. A DICOM palette descriptor | `ocelli-render` fixture. Mutating the index input from the display value to the stored value turns it red |
| AC5 | World point from PS3.3 C.7.6.2.1.1 is `(3, 8, 0)` at `k = 0` and `(3, 8, 10)` at `k = 2`. The second volume maps that point to index `(1, 4, 2)` | Hand-computed fixture | Swapping `PixelSpacing[0]` and `PixelSpacing[1]`, which yields `(6, 4, 0)`. Using one spacing for both volumes. Adding a half-voxel offset | `ocelli-volume` fixture. Swapping the spacing indices turns it red |
| AC6 | Tier C returns unavailable before sampling and before any texture write. No CPU fusion image | Tier match on `Tier::Cpu` with no adapter | Running the fragment composite on the CPU and presenting it | `ocelli-render` test. No GPU |
| AC7 | Opacity below 0, above 1, or non-finite. Error. Previous frame unchanged | Present, refuse, compare bytes | Clamping and drawing | `ocelli-render` test |
| AC8 | A world point outside a volume is `PlaneSample::Absent`, not `Present(0)` | Fixture on `sample_world` | Returning stored 0, or windowing that 0 into a display colour | `ocelli-volume` fixture. The other volume's pixels are question 2 and are not this row |
| AC9 | Valid draw, mismatched UID, valid retry | Bytes match the first frame after the mismatch, and the retry can publish a new frame | The mismatch sticking as the bound pair, or the retry being ignored | `ocelli-render` test |
| AC10 | Settled frame and the same request with clear dirty flags | No allocation and no extra submit | A second upload of the same plane | `ocelli-render` test against the submit counter |
| AC11 | A chain `VoiParams::from_chain` rejects | Unavailable, previous frame unchanged, no CPU image | Presenting `map_into` colours for that sequence on the GPU path | `ocelli-render` test |
| AC12 | Both UID strings empty | Same refusal as a mismatch. Nothing drawn | Treating empty equality as one frame of reference | `ocelli-render` test |

AC1's RGB half, and any partial-overlap pixel, stay open criteria. A missing adapter does not pass AC1's readback. A missing formula does not pass it either.

## Approach

Smallest path that stays inside the transcribed sources.

1. In `ocelli-volume`, add `FusionPlane`, `plane_from_ortho`, and `Volume::sample_world`. `sample_world` is the inverse above plus the existing private `sample_at`, made `pub(crate)` to that call. No colour. No blend. No SUV types.
2. In `ocelli-render`, add `FusionRequest` and a presenter that checks tier, then UIDs, then opacity, then `from_chain`, and returns before sampling on every failure. That order is what keeps the published texture stable.
3. Colormap-after-VOI is a pure function that calls `LutChain::map_into` and then the existing display index. The shader calls `lut_chain` and the existing `table_index` rule. It does not carry a second copy of either.
4. The fragment shader is one WGSL module, `shaders/fusion.wgsl`, appended to the existing `VOI_WGSL` text. Tier A and tier B compile that same text. It samples two already addressed planes. It does not walk a ray and it does not hardware-filter a 3D edge.
5. The graph tick encodes the fusion pipeline only when a fusion view is bound and dirty, in the existing encoder, then submits once. `Pass` is unchanged.

The composite statement inside the shader is `out = pet * opacity_pet + ct * opacity_ct * (1 - opacity_pet)`. An absent sample adds no colour and the present volume is drawn. It is not stored zero.

Rejected alternatives:

- Folding SUV into modality, or computing it beside fusion. Forbidden by section 18.1 and by the skill, section 9.
- Overlaying different UID strings. Forbidden by the skill, section 1, and by the second-candidate rule.
- A CPU fusion image on tier C. Forbidden by D-07.
- Resampling both volumes onto one index grid. That discards the second origin, spacing and direction.
- A new `Pass` variant. Section 22's enum is the signature. A new variant would be a deviation, and this plan has none.
- Reusing `VolumeRaycast` or a slab blend mode. Those are other stories and other images.
- Hardware linear filtering. A zero border is stored zero.
- A fusion trait, or a `Vec` of volumes behind a trait object. Two implementers do not exist. The parameters are two volumes.

## Dependencies, write ownership and resources

Declared dependency: F-052, done. Source dependencies already in tree: `FrameOfReferenceUid`, `VolumeCandidate.frame_of_reference`, `Volume::voxel_world`, `sample_at`, `PlaneSample`, `LutChain::map_into`, `VoiParams::from_chain`, `voi.wgsl` `lut_chain`, the display index, `Tier`, the graph tick and its submit counter. F-053, F-054 and F-055 are the same sprint and are not dependencies. No backlog cell here names them.

Order: volume sampler and AC5, then refusal presenter and AC2, AC3, AC6, AC7, AC12, then AC4, then the shader shell with no composite constant. Questions 1 and 2 block the composite and the absent-pixel branch.

Exclusive new files: `crates/ocelli-volume/src/fusion.rs`, `crates/ocelli-volume/tests/fusion.rs`, `crates/ocelli-render/src/fusion.rs`, `crates/ocelli-render/shaders/fusion.wgsl`, `crates/ocelli-render/tests/fusion.rs`.

Shared and limited:

- `crates/ocelli-volume/src/ortho.rs` gains `sample_world` and `pub(crate)` on `sample_at` only. F-053 must not rewrite `sample_at`. It may call `sample_world`.
- `crates/ocelli-volume/src/lib.rs` gains one module declaration.
- `crates/ocelli-render/src/lib.rs` gains one module declaration.
- `crates/ocelli-render/src/graph.rs` may gain one encode call and may share the existing `table_index` text so it is not forked. It must not add a `Pass` variant, a `BlendMode`, or a dirty flag. F-054 owns the slab blend bodies in this file.
- `crates/ocelli-render/src/colour.rs` is not given a second index. Widen visibility only if AC4 cannot call the existing function. Do not change the rounding.
- Do not edit `voi.wgsl`, `raycast.wgsl`, `Volume`'s fields, or `group_volumes`.

GPU exclusivity stays `ocelli-render`. One device. Fusion textures are graph resources. No corpus row is read. No patient fixture is added.

## Verification and evidence reuse

Focused checks, from the command itself and not from a pipe status:

```text
bin/ocelli.sh check ocelli-volume
bin/ocelli.sh test ocelli-volume -- fusion
bin/ocelli.sh check ocelli-render
bin/ocelli.sh test ocelli-render -- fusion
```

Floor profile has no GPU and no corpus. AC2, AC3, AC4, AC5, AC6, AC7, AC8, AC11 and AC12 are floor evidence. AC1's address half is floor evidence. AC1's presented pixel is not floor evidence and is not passable until question 1 names the triple. AC9 and AC10 run with the presenter and the submit counter and do not need an adapter if the refuse path never encodes.

No tolerance change. No gate disabled. No baseline image is updated, because no oracle fusion view exists. Appendix B is not an expected image. wgpu calls in the new module are checked against the pinned wgpu in `ocelli-render`, not from memory. A compile of the shader is not a pixel proof.

Invalidating evidence: a change to `sample_at`, to `lut_chain`, to the display index, to the UID type's equality, or to the submit counter's meaning. Completion metadata does not turn AC1's open RGB row into a pass.

Performance: no new budget. The acceptance threshold is the existing rule, one submit and no render-loop allocation, observed with the submit counter and a buffer-capacity check. This story does not add a benchmark subject.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Plane buffers and colour tables are sized when the view or the plane is bound
- unsafe: none
- Tier A (WebGPU): full, once questions 1 and 2 are answered. Same fragment shader as tier B. Until then the presenter may refuse rather than encode an unchosen composite
- Tier B (WebGL2): full on that same shader, with no compute and no storage buffer. A device that cannot bind the two planes reports unavailable. It does not switch to a CPU image
- Tier C (CPU): unavailable. No fusion image. D-07

The three rows are deliberate. Tier C is not an omitted row and not a silent reslice.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC5. World point `(3, 8, 10)` cites PS3.3 C.7.6.2.1.1. Second volume index `(1, 4, 2)`. Swapped spacing indices go red | `crates/ocelli-volume/tests/fusion.rs` |
| fixture | AC8. Outside point is `Absent`, not `Present(0)` | `crates/ocelli-volume/tests/fusion.rs` |
| fixture | AC4. Display 127.5 from HLD 18.3's centre row indexes table 128, not stored 40. Cites PS3.3 C.11.2.1.3.2 for the window that produces 127.5, and the existing index rule for 128 | `crates/ocelli-render/tests/fusion.rs` |
| unit | AC2, AC7, AC9, AC12. Refusal leaves the previous frame | `crates/ocelli-render/tests/fusion.rs` |
| unit | AC3. Struct literal has no `PatientWeight` | `crates/ocelli-render/tests/fusion.rs` |
| unit | AC6 and AC11. Tier C and a sequence chain produce no image | `crates/ocelli-render/tests/fusion.rs` |
| unit | AC10. Settled duplicate does not submit again | `crates/ocelli-render/tests/fusion.rs` |

AC5 numbers, hand-computed. IPP and origin `(0, 0, 0)`. Row direction `X = (1, 0, 0)`. Column direction `Y = (0, 1, 0)`. Normal `(0, 0, 1)`. `PixelSpacing = [2.0, 1.0]`, so `spacing[0] = 1`, `spacing[1] = 2`, `spacing[2] = 5`. Column index `i = 3`, row index `j = 4`, slice index `k = 2`.

```text
P = (0, 0, 0) + 3 * 1 * (1, 0, 0) + 4 * 2 * (0, 1, 0) + 2 * 5 * (0, 0, 1)
  = (3, 8, 10)
```

At `k = 0` the point is `(3, 8, 0)`. The swapped-spacing mistake gives `(6, 4, 0)` and must fail.

Second volume, same directions, origin `(1, 0, 0)`, spacing `[2, 2, 5]`. Delta from its origin is `(2, 8, 10)`. Index is `(1, 4, 2)`. A sampler that reuses `(3, 4, 2)` is wrong.

AC4 numbers. `LINEAR_EXACT`, centre 40, width 400, output 0 to 255, input 40. HLD section 18.3 gives 127.500. The existing index of 127.5 is 128. Table entry 128 is `(255, 0, 0)`. Table entry 40 is `(0, 0, 255)`. The fixture expects red.

## Parity surface covered

None. Appendix B has no fusion row and no SUV row. This story does not claim an `ORTHOGRAPHIC` parity observation and does not claim a slab blend-mode observation. No cornerstone fusion pixels are produced or compared.

## Deviations

none

## LLD impact

`/complete-feature` step 9 updates `docs/lld/volumes.md` with `FusionPlane` and `sample_world`, and the rule that a second frame of reference is refused at fuse time rather than overlaid. It updates `docs/lld/render-graph.md` only to say a fusion view encodes inside the existing tick and does not add a `Pass`. It updates `docs/lld/feature-availability.md` with fusion unavailable on tier C. It does not add an SUV section. Those edits are not part of this draft.

## Open questions

None. The consolidated design round was offered on 2026-10-31 and declined. PET is in front. `out = pet * opacity_pet + ct * opacity_ct * (1 - opacity_pet)`. Both opacities 1 on CT `(10, 10, 10)` and PET `(200, 0, 0)` yield `(200, 0, 0)`. At opacity 0.5 the pixel is `(102.5, 2.5, 2.5)`. An absent volume adds no colour and the present volume is drawn. The absent sample is not stored zero. A hardware border of zero is rejected.
