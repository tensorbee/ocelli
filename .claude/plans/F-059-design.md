# F-059, VOLUME_3D viewport with ray-cast direct volume rendering

**Status**: approved
**Epic ref**: E9.1
**Sprint**: S19
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/19-render-graph.md`, section 22

```rust
pub enum Pass {
    Stack(StackPass),
    VolumeRaycast(VolumePass),
    SegOverlay(SegPass),
}
pub struct Caps {
    pub compute: bool,
    pub max_tex_3d: u32,
    pub max_buffer: u64,
    pub tier: Tier, // A = WebGPU, B = WebGL2 downlevel
}
```

Pipelines compile at init, keyed by pass kind, blend mode and tier. A viewport re-renders only when camera, VOI, slice, segmentation or data changed. One `queue.submit()` per frame. Device loss is a state. The graph rebuilds the device and resources.

`VolumePass` in the tree carries only `blend`. The composite stack shader is the only body `compile` builds. A volume key has no pipeline, and the frame path reports that viewport unavailable.

### `docs/hld/05-rendering.md`, section 7

Tier A is WebGPU. Tier B is WebGL2 through wgpu's downlevel profile, fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Volume rendering runs on both tiers because a 3D-texture ray cast in a fragment shader is tier-B legal. Anything that wants compute degrades rather than fails. Blend modes are shader variants selected by specialisation constant rather than a branch per fragment. Only composite has a body today. The other four report unavailable rather than substituting composite.

### `docs/hld/15-lut-chain.md` is not re-opened

The ray reads stored samples and the existing section 18.4 uniform. The three window functions stay the WGSL already transcribed from section 18.2. This story does not add a second decision about inversion, window selection or a modality sequence. A chain with no section 18.4 uniform still reports unavailable on the host side.

### `docs/hld/DEVIATIONS.md`, D-07

Interactive volume ray casting on the CPU is not claimed. F-X005 decides between a slow path and reporting unavailable, and it must not draw a different image. This story reports unavailable on tier C.

### `docs/hld/26-differentiating-capabilities.md`, section 30.1

Opacity correction across LOD steps is F-121. This story has one step size and one level, so it does not call `correct_opacity`.

### Sprint text

The ray uses the same origin, spacing and direction as `Volume::construct`. Spacing Between Slices is not that spacing. Tier A and tier B each have an answer. Tier C reports unavailable. A CPU image that differs from the GPU image is not an answer. F-060 owns the transfer-function editor. F-061 owns lighting and shading. F-062 owns cropping planes.

## What the specification does not cover

Section 22 names `VolumeRaycast` and section 7 says the technique is a fragment-shader ray cast. They do not give the integral. These choices make a two-voxel ray hand-computable and keep absent bricks out of the sum.

1. **The view axis is one volume axis.** `RayAxis::Slice`, `Row` or `Column`. Parallel rays, one per pixel of the other two axes. The ray walks voxel centres from index 0 through `dim - 1`, or the reverse when `from_far` is set. Perspective, orbit and an arbitrary normal are not this story. `PERSPECTIVE` in Appendix B is a different viewport type.
2. **A sample is the stored container value at that integer voxel, when its brick was uploaded.** `Volume::resident` is true only when the voxel's level-0 brick is in `ready_bricks`. The shader receives a per-step residency flag. A flag of 0 skips the step. The shader does not read a texel for that step, even if a texture happens to hold bytes there.
3. **Colour is the VOI display value divided by `ymax - ymin` when that span is non-zero, else 0.** Opacity of every present sample is the constant `0.5`. Front to back:

```text
C = 0
A = 0
for each resident step, near to far:
    src = voi(stored) 
    colour = (src - ymin) / (ymax - ymin)    // 0 when ymax == ymin
    alpha = 0.5
    C = C + (1 - A) * colour * alpha
    A = A + (1 - A) * alpha
```

`ymin` and `ymax` come from the uniform. The constant `0.5` is the stand-in until F-060 replaces it with an edited curve. It is not a clinical preset. Lighting, gradient opacity and cropping are not applied.

4. **The production integral runs in the fragment shader.** The test's expected pixels are the same formula evaluated in the test from the constants below, not a copy of a previous readback. There is no second production ray marcher on the CPU.
5. **A brick axis longer than `max_tex_3d` reports unavailable** through the existing `BrickUploadError`. The ray does not drop that brick and draw the rest, because the rest would be a different image. F-057 already refuses the upload.
6. **Other blend modes stay unavailable.** Composite is the body. MIP, MinIP, average and labelmap edge are F-054 or later and must not fall through to this integral.

## Problem and reproducer

`Pass::VolumeRaycast` carries a blend mode and compiles no shader. A tick of that viewport is `FrameOutcome::Unavailable`. Filling the pass with the stack shader would window one slice and call it a volume.

Expected observation for one ray along `+z`, identity spacing 1 mm, `U8` voxels 0 then 200, both resident. VOI `LINEAR_EXACT`, centre `127.5`, width `255`, `ymin` 0, `ymax` 255. Section 18.2 maps stored 0 to display 0, because `0 <= 127.5 - 127.5`. It maps stored 200 to display 200, because 200 is inside `(0, 255]` and `((200 - 127.5) / 255 + 0.5) * 255 = 200`.

Both steps resident:

```text
step 0: colour 0, alpha 0.5, C = 0, A = 0.5
step 200: colour 200/255, alpha 0.5
C = 0.5 * (200/255) * 0.5 = 100/255
A = 0.5 + 0.5 * 0.5 = 0.75
```

First step marked absent, second present, and the texture still holds 200 at both texels so a mask bug is visible:

```text
C = (200/255) * 0.5 = 100/255
A = 0.5
```

A march that treats the absent step as stored 0 with alpha 0.5 instead yields `C = 50/255` and `A = 0.75`.

Current behaviour: no ray shader, no residency flag, no readback.

Source identity: `VolumePass` and `RenderGraph::compile` in `crates/ocelli-render/src/graph.rs`.

## Scope, API and compatibility

In scope:

- A fragment shader `crates/ocelli-render/shaders/raycast.wgsl`, compiled at graph creation for `PassKind::VolumeRaycast` and `BlendMode::Composite`, on tier A and tier B.
- `VolumePass` gains `voi: VoiParams` and `axis`, `from_far`.
- `Volume::resident` in `ocelli-volume`.
- A residency buffer uploaded with the brick list from F-057, one `u32` per voxel step of the view axis, 1 or 0.
- GPU readback of the two rays above.

Out of scope:

- Transfer-function editing, presets, lighting, gradient opacity, cropping. F-060, F-061, F-062.
- CPU ray casting. F-X005.
- Opacity correction across levels. F-121.
- Sampling level > 0.
- Oblique rays and perspective.
- Changing the stack shader or the three VOI formulas.
- An oracle diff against cornerstone3D. Decision D7. The oracle still has no full viewer comparison. This story's evidence is the hand-computed integral and a GPU readback.

```rust
pub struct VolumePass {
    pub blend: BlendMode,
    pub voi: VoiParams,
    pub axis: RayAxis,
    pub from_far: bool,
}

pub enum RayAxis {
    Column,
    Row,
    Slice,
}

impl Volume {
    /// Level-0 brick of this voxel is fully present.
    pub fn resident(&self, voxel: [u32; 3]) -> bool;
}
```

`resident` is false outside `dims`. It does not allocate.

The shader is appended to the existing `VOI_WGSL` prefix the way the stack module already is, so the window functions are included once and not retyped. Pipelines for volume composite are built inside `compile`, which already runs at graph creation. A request for `PipelineFamily::Compute` or for a non-composite blend still returns no pipeline.

Output of one viewport is `Rgba32Float` or the graph's existing colour attachment. The test reads `C` from the red channel and `A` from the alpha channel. Green and blue equal red. The attachment format stays whatever `compile` already chose for the graph. The test records that format and reads the channels the shader writes. It does not add a second colour target.

Refusal: tier C graph creation stays `TierCUnavailable`. A volume raycast registered on A or B with a non-composite blend ticks as `Unavailable` and keeps its dirt. A brick that failed upload because of `max_tex_3d` is not marked resident.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Volume viewport tick | `raycast.wgsl` composite, tier A and tier B | Axis-aligned rays, level 0, resident steps only | Premultiplied `C` and `A` per pixel |
| `Volume::resident` | `ready_bricks` and `BrickGrid::brick_of` | Level 0 | `bool` |
| Residency buffer | `resident` along the view axis | Uploaded when data or axis changes, not per fragment | `u32` 0 or 1 per step |
| VOI uniform | existing `VoiParams` | No new LUT decision | The shader evaluates the uniform it is given |
| Brick 3D textures | F-057 `sync_volume` | Only ready bricks | A step whose brick is missing is flag 0 |
| `BlendMode::Mip` and the other three | unchanged | No body | `Unavailable`, not this integral |
| Tier C | graph creation | F-X005 later | `TierCUnavailable`, no image |
| F-060 editor | not called | The constant 0.5 stays until that story | No preset library |

Units: voxel indexes are the constructed volume's indexes. `C` and `A` are unitless fractions of the display span. One step is one voxel, so the step length is that axis's spacing and is not read from Spacing Between Slices.

Rounding: the test evaluates the formula in `f32` with the same operation order as the shader comments. Comparison is 2 `f32` ULP, the F-041 bound, except `A = 0.75` and `A = 0.5` are exact `f32` values and compare equal. A wrong alpha of 1.0 misses by far more than 2 ULP.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Graph created on A or B | Init | Volume composite pipeline is compiled. Other volume blends are not | No compile during `tick` |
| Registered volume viewport, bricks synced, dirty | `tick` | One submit. Readback matches the integral for the fixture rays | Stack viewports in the same graph still share that submit |
| Residency flag 0 on a step whose texel is non-zero | `tick` | That step adds nothing | Sampling the texel anyway fails AC2 |
| Non-composite `VolumePass` | `tick` | `Unavailable`. Dirt retained | Composite is not substituted |
| `set_voi` | new uniform | `Dirty.voi`. Next tick uses the new window. Bricks are not re-uploaded | F-043's rule stands |
| Device rebuild | F-037 path | Pipelines compile again. Brick stamps clear with the textures. The next sync uploads ready bricks | A tick before rebind stays `ViewportNotRebound` |
| Tier C | `RenderGraph::new` | `TierCUnavailable` | No CPU ray image stored anywhere |

The last good frame stays when the new pass is unavailable. A refused blend does not encode over it.

An identical second tick with no dirt submits nothing. That is the existing dirty-tracking rule. It is a no-op frame, not a second integral.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Ray pipeline | `compile` at graph creation and on rebuild | `(VolumeRaycast, Composite, tier)` | Graph `new` returns | A shader error fails graph creation. No half-compiled volume pass is cached as the stack pass |
| Residency buffer | `sync` or `set` of the axis, before the frame | The volume viewport id | Buffer write, then `Dirty.data` | A failed write leaves the previous flags and the previous dirt |
| Brick textures | F-057 | `BrickKey` | Unchanged by the ray | The ray only reads them |
| Colour attachment | The caller-supplied view, F-038 | The viewport's target | `queue.submit` of the tick | `Unavailable` does not encode that viewport |

No allocation inside the fragment loop on the CPU. The residency buffer is sized at registration from `max(dims)`, a fixed capacity for that viewport, and rewritten when the axis changes. A volume larger than that capacity is refused at registration with the existing capacity error rather than reallocated mid-frame.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | GPU, tier A or B. One brick, voxels `[0, 200]` along z, both resident. VOI as in the problem section. Readback `C` matches `100/255` within 2 ULP and `A` equals `0.75` | `two_resident_steps_match_the_integral` ignored, `gpu` gate | Stack shader of the first voxel only. Alpha 1.0. A second window formula | Integrated. GPU readback |
| AC2 | GPU. Same texture, residency `[0, 1]`. `C` matches `100/255` within 2 ULP and `A` equals `0.5`. The texel at step 0 is 200, so ignoring the flag cannot hide | `an_absent_step_is_skipped` | Treating the flag-0 step as stored 0 with alpha 0.5, which yields `A = 0.75` | Integrated. GPU readback |
| AC3 | `U8` dims `[1, 1, 129]`. Slices 0 through 127 placed. Slice 128 not placed. `resident([0, 0, 0])` and `resident([0, 0, 127])` are true. `resident([0, 0, 128])` is false. Edge is 128, so those are two bricks and only the first is ready | `resident_follows_ready_bricks` | Marking every zero byte resident. Using a 64 edge on `U8` | Implementation. `ocelli-volume` |
| AC4 | `VolumePass` with `BlendMode::Mip` ticks `Unavailable`. `pipeline_family_compiled` is false for that key. Composite stays compiled | `a_volume_mip_is_not_the_composite_ray` | Running this integral for MIP | Implementation. Render graph test, no adapter required if the compile table is host-side. The existing `pipeline_compiled` tests already run without a device |
| AC5 | Tier C `RenderGraph::new` returns `TierCUnavailable`. No function in this story returns a CPU `Vec` of ray colours | `volume_raycast_has_no_cpu_image` | A CPU march used as the viewport | Implementation |
| AC6 | `from_far` on the AC1 volume walks 200 then 0. `C = (200/255) * 0.5` and `A = 0.75` | `the_far_end_is_the_first_sample` | Ignoring the flag and keeping the near order | Integrated. GPU |

AC1, AC2 and AC6 need an adapter. They are `#[ignore]` and the `gpu` gate runs them. A missing adapter leaves them unmeasured. It does not pass the row.

Semantic RED: residency ignored in the shader. AC2's alpha becomes `0.75` instead of `0.5`. The mutation is deleting the flag test.

The mathematical `100/255` is the review value. The test computes `(200.0_f32 / 255.0) * 0.5 * 0.5` for AC1 and `(200.0_f32 / 255.0) * 0.5` for AC2 so the expected number is the formula, not a captured frame.

## Approach

`resident` uses `brick_grid().brick_of` and a set built from `ready_bricks`. A voxel outside the grid is false.

`compile` gains the volume composite pipeline beside the stack one. The key includes the tier. Tier B compiles the same fragment text. It does not compile a compute march. The shader uses `textureLoad` on the 3D brick, which F-042 already relies on for tier B float textures. Integer brick formats stay the F-043 formats. The load converts the integer to `f32` by the sample kind in the uniform, not by normalising `Unorm`.

`VolumePass.voi` removes `PassHasNoUniform` for this pass. `set_voi` writes the same 32-byte uniform. A volume pass without a resolved uniform still fails at registration with `VoiParamsError`.

The residency buffer is a storage buffer on tier A and a uniform or texel buffer that tier B can sample. Tier B has no storage buffers, from section 7. The residency path that both tiers share is a `R32Uint` 3D or 2D texture of flags, written with `write_texture`, and `textureLoad` in the shader. That is legal on B. A storage buffer is rejected because it would make the ray tier A only, and section 7 says this technique is tier B legal.

One brick texture is bound for the fixture. A viewport whose ready set is several bricks binds them up to the device's sampled-texture limit. Above that limit, registration returns unavailable and does not drop bricks. The AC1 volume is one brick, under the limit. The limit check is a host test that a fake cap of 0 textures returns unavailable before any shader runs.

Rejected alternatives:

- A CPU march uploaded as a finished image. That decides F-X005 inside this story and can disagree with a later GPU path.
- Compute ray march. Tier A only, which contradicts section 7 for volume rendering.
- Opacity equal to the windowed value. Stored 0 then becomes invisible, and the absence test cannot tell a skipped step from a zero step. The constant 0.5 is what makes AC2 fail closed.
- Sampling with trilinear inside the ray. The ray is defined on voxel centres so the integral cites stored values directly. Off-centre cameras belong with a later view story.

wgpu is pinned at 30.0.1. The implementer checks `textureLoad`, `write_texture` and the bind-group layout against that version's docs. A call that only exists in a newer wgpu is a defect even if it compiles under a different crate.

## Dependencies, write ownership and resources

Depends on F-051, `done`. In this sprint it follows F-057, because residency is `ready_bricks` and the textures are the ones `sync_volume` created. It follows F-052 only so the two stories do not edit `ocelli-render/src/lib.rs` together. It does not call `reslice`.

Writes:

- `crates/ocelli-volume/src/lib.rs` for `resident`
- `crates/ocelli-volume/tests/resident.rs`
- `crates/ocelli-render/shaders/raycast.wgsl`
- `crates/ocelli-render/src/graph.rs`
- `crates/ocelli-render/src/lib.rs`
- `crates/ocelli-render/tests/raycast.rs`
- `docs/lld/render-graph.md` at completion

GPU exclusive. Serial wave, canonical worktree, after F-052.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-volume
bin/ocelli.sh test ocelli-volume --test resident
bin/ocelli.sh check ocelli-render
bin/ocelli.sh test ocelli-render --test raycast
bin/ocelli.sh clippy ocelli-volume
bin/ocelli.sh clippy ocelli-render
```

The ignored tests run under `bin/ocelli.sh gate gpu` at completion and under `bin/ocelli.sh gate --sprint` after integration. Shader text is part of the crate. A change to the alpha constant or the VOI include invalidates the receipt.

No new performance threshold. Fill rate stays the existing probe and is not an acceptance number for this story.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Residency capacity is fixed at registration
- unsafe: none
- Tier A (WebGPU): full fragment ray cast of resident bricks
- Tier B (WebGL2): full. The same fragment shader and a residency texture. No compute and no storage buffer
- Tier C (CPU): unavailable. No CPU image

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 and AC2 integrals, section 18.2 for the two stored values, and the front-to-back sum in this plan | `tests/raycast.rs` GPU |
| fixture | AC6 far-to-near order | same |
| unit | AC3 `resident` against a 129-slice grid. The edge 128 is section 30's eight-bit brick | `tests/resident.rs` |
| unit | AC4 blend refusal and AC5 tier C | render tests |

## Parity surface covered

Appendix B viewport types, the `VOLUME_3D` behaviour: an axis-aligned ray-cast image on tier A and tier B. This story does not establish `PERSPECTIVE`, blend-mode projections, lighting, cropping, an editable transfer function, or a cornerstone diff. D7 holds.

## Deviations

None. Tier C unavailability is D-07 applied, not a new row. Section 22 names `VolumeRaycast(VolumePass)` and does not list fields. `StackPass` already carries `voi` and `blend` as payload under the same enum. `VolumePass` gains `voi`, `axis` and `from_far` the same way. The enum text is unchanged, so no new `D-NN` row is added.

## LLD impact

`docs/lld/render-graph.md` gains the volume composite shader, the residency texture, the integral, the constant opacity, and the tier C refusal. `docs/lld/volumes.md` gains `resident`.

## Open questions

None. The integral and the constant opacity are specified above so a later edit has numbers to fail. F-060 is the story that replaces the constant. F-X005 is the story that may add a CPU path after a measurement this sprint does not have.
