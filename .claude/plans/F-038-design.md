# F-038, Render graph and frame scheduler with dirty tracking

**Status**: approved
**Epic ref**: E6.2
**Sprint**: S12
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/19-render-graph.md`, section 22, in full

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

|  |
|----|
| **Pipelines compile at init**, keyed by (pass kind, blend mode, tier). Never compile a shader mid-frame. |
| **Dirty tracking.** A viewport re-renders only when camera, VOI, slice, segmentation or data changed. Everything else is a no-op frame. |
| **One queue.submit() per frame** across all viewports, from the render worker's requestAnimationFrame. |
| **Device loss is a real state, not an error path.** Handle device_lost, rebuild the device and all resources, and restore viewport state from the shell's copy — the same recovery path §23 needs for panics. |

**Four normative sentences and one enum.** The enum names three pass kinds
and no payload. The `Caps` struct is already implemented by F-037 in
`crates/ocelli-render/src/caps.rs` and this story does not redefine it. The
tier comment names two tiers because deviation D-07 had not been raised when
the section was written. The resolved tier in code has three values and
section 5 below answers for all three.

### `docs/hld/05-rendering.md`, section 7, in full

|  |
|----|
| **Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup. |
| **Volume rendering runs on both tiers**, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute — GPU segmentation, histogram passes, compute-based resampling — is tier A only and must degrade, not fail. |
| **Bricking above 256 MiB.** A 512×512×600 sixteen-bit CT series is roughly 300 MB against a guaranteed maximum buffer size of 256 MiB, so chunked upload is the normal path, not an optimisation. |
| **One submit per frame.** The render graph tracks dirty viewports and issues a single submission across all of them, driven by requestAnimationFrame inside the render worker. |
| **Blend modes are shader variants** — composite, MIP, MinIP, average — selected by specialisation constant rather than branching per fragment. |

**Two statements of the one-submit rule exist and both bind this story.**
Section 22 states it as a property of the frame. Section 7 states it as a
property of the graph. A design that satisfies one and not the other has
satisfied neither.

**Section 7 and section 22 state blend selection two ways.** Section 7
selects a blend mode by specialisation constant. Section 22 keys a compiled
pipeline by blend mode. This plan reads them as one rule at two levels and
records the reading in What the specification does not cover, item 4.

### `docs/hld/23-performance-rules.md`, section 26, in full

|  |
|----|
| No allocation in the render loop. |
| One queue.submit() per frame. |
| Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload. |
| Batch pointer events into one command buffer per animation frame. Never cross the boundary per event. |
| Measure with the benchmark harness before optimising anything. The intuitions that work in JavaScript do not transfer. |

**Three of the five rules become testable for the first time in this
story.** No render loop existed before it. The fourth rule binds the shell
side of the scheduler contract in Approach section 5. The fifth rule is why
this plan adds no benchmark subject and changes no number in
`tools/bench/subjects.json`.

### `docs/hld/15-lut-chain.md`, section 18, opening and stage table

|  |
|----|
| This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere. |

| **Stage** | **From → To** | **Source** |
|----|----|----|
| 1\. Modality LUT | Stored → Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2\. VOI LUT | Modality → Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3\. Presentation LUT | Display → Display | Identity or INVERSE; presentation state may override |
| 4\. Palette / ICC | Display → RGB | Palette colour LUT, or the display colour pipeline |

### Section 18.2, the three VOI functions, character for character

```text
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
// x <= c - w/2 -> ymin
// x > c + w/2 -> ymax
// else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
// y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
```

### Section 18.3, the fixture table

Soft-tissue CT window, centre 40, width 400, output range 0 to 255.

| **Input (HU)** | **LINEAR** | **LINEAR_EXACT** | **Why this row** |
|----|----|----|----|
| −160 | 0.000 | 1.594 | LINEAR boundary is c'−w'/2 = −160 exactly; the comparison is \<=, so this clamps |
| 40 | 127.819 | 127.500 | The window centre. A 0.32 divergence no reviewer would see by eye |
| 240 | 255.000 | 255.000 | LINEAR upper bound is c'+w'/2 = 239, so 240 clamps |
| −60 | 63.910 | 63.750 | Mid-lower quarter; catches sign and slope errors |

**Row one carries declared deviation D-13.** The transcribed formula clamps
at that input and yields 0.000 for LINEAR_EXACT. F-011 recorded the
disagreement and every fixture since has computed from the formula. This
plan does the same and cites D-13 rather than copying the table value into
a test.

### Section 18.4, the shader side, in full

```wgsl
// ocelli-render/shaders/voi.wgsl
struct VoiParams {
    center : f32,
    width : f32,
    slope : f32,
    intercept : f32,
    ymin : f32,
    ymax : f32,
    fn_kind : u32, // 0 LINEAR, 1 LINEAR_EXACT, 2 SIGMOID
    invert : u32,
};
@group(0) @binding(0) var<uniform> voi : VoiParams;
```

|  |
|----|
| *A window-level drag updates thirty-two bytes per frame. No texture is re-uploaded, and that is the concrete performance claim behind Figure 2.* |

**F-041 already implemented this uniform.** `crates/ocelli-render/src/voi.rs`
holds `VoiParams` at thirty-two bytes and
`crates/ocelli-render/shaders/voi.wgsl` holds the composable shader text
with no entry point. The shader file reserves group 0 binding 0 and states
in its own header that F-038 appends the fragment entry point. This story
is that consumer and adds no second copy of the window arithmetic.

## What the specification does not cover

1. **What a viewport is.** No section defines a viewport identity, a
   registration call, a removal call or an ordering between viewports.
   `ocelli-viewport` is a scaffold with no render surface. Section 22 says
   a viewport re-renders and never says how the graph learns one exists.
   Decided in Approach section 2 and Open question 1.
2. **What the three pass payloads hold.** Section 22 names `StackPass`,
   `VolumePass` and `SegPass` and defines none of them. No field list
   exists anywhere in `docs/hld/` for any of the three. This plan defines
   the smallest payload each kind needs to be scheduled and no more.
3. **Dirty granularity and coalescing.** Section 22 lists five change
   sources and stops. It does not say whether dirt is tracked per viewport
   or per pass, whether two marks before one frame coalesce, who sets the
   mark, or what state a new viewport starts in. Decided in Approach
   section 3.
4. **How a blend mode reaches a pipeline.** Section 7 selects by
   specialisation constant and section 22 keys pipelines by blend mode.
   The pinned `wgpu` version is 30.0.1 under the workspace pin and its
   render pipeline API must be read before either reading is coded. This
   plan keys pipelines by blend mode and treats the constant wording as
   the same selection seen from the shader side. No `wgpu` call in this
   area is taken from memory, per the standing expectation in
   `docs/sprints/CURRENT_SPRINT.md`.
5. **Who calls the frame.** Section 22 places the submission inside the
   render worker at requestAnimationFrame. This crate carries no browser
   binding under deviation D-12 and cannot register that callback itself.
   The contract between the worker loop and the scheduler is unspecified.
   Decided in Approach section 5 and Open question 2.
6. **What a frame renders into.** F-039 owns OffscreenCanvas and surface
   presentation in S13. F-040 owns texture upload in S13. Neither exists.
   Section 22 assumes a target and names none, so this story must choose
   a target shape that both later stories can adopt without a redesign.
   Decided in Approach section 2 and Open question 1.
7. **Frame failure semantics.** Section 22 makes device loss a state and
   says nothing about a loss observed mid-frame, a frame with no dirty
   viewport, or a viewport whose VOI chain F-041 cannot express. Each
   needs an answer that is neither a panic nor a silent blank frame.
   Decided in Approach sections 5 and 6.
8. **Who rebuilds graph resources after a device loss.** Section 22 says
   rebuild the device and all resources. `GpuContext::recover` in
   `crates/ocelli-render/src/gpu.rs` rebuilds the device alone today and
   returns `Recovered` holding only `Caps`, because no resource type
   existed when F-037 landed. The ownership split between the context and
   the graph is unspecified. The tasking for this story fixes two points
   and leaves the rest to this plan. The `recover` signature does not
   change. `Recovered` gains the restored resources. Decided in Approach
   section 7 and Open question 3.
9. **Uniform ownership across viewports.** Section 18.4 gives one uniform
   block. Two viewports at two windows need two values at once, and no
   section says whether the block is per viewport or shared. A shared
   block cannot hold two windows in one frame. Decided in Approach
   section 6.
10. **Pass order inside one submission.** A segmentation overlay must
    composite after the stack pass it overlays. Section 22 lists three
    pass kinds and no order. This plan fixes registration order within a
    viewport and stack before overlay for the same viewport.
11. **Tier B pipeline shape.** Section 7 limits tier B to fragment
    shaders with no compute and no storage buffers. The VOI shader from
    F-041 is already tier B legal. Whether one pipeline set serves both
    GPU tiers or tier B needs variants is left to F-042 in S13. This plan
    keys the tier into the cache so a later variant needs no new key.
    Tier B has never been exercised by anything in this project, which
    `docs/sprints/CURRENT_SPRINT.md` names directly, and this plan claims
    no tier B execution as evidence.

## Approach

### 1. Scope, and what this story deliberately does not build

**In scope.** A render graph that registers viewports, tracks dirt per
viewport, compiles every pipeline it will ever use at creation, encodes
each dirty viewport once per frame into one command encoder, and submits
that encoder exactly once. A frame scheduler that turns external frame
ticks and dirty marks into submitted frames, no-op frames and reported
unavailable frames. The first production composition of the F-041 VOI
shader behind a fragment entry point. The extension of `Recovered` with
the graph resources a device rebuild restores.

**Out of scope, by sprint boundary.** Presentation to a canvas belongs to
F-039. Texture upload from decoded pixels belongs to F-040. A frame
therefore renders into caller supplied offscreen texture views and no
surface is acquired, configured or presented here. The oracle still has
no Ocelli frame to judge after this story, because no upload path feeds
the stack pass real pixels. Decision D7 in the decision log stays open
until F-040 and this plan does not claim otherwise. Brick residency
belongs to F-036 and F-043. Volume data and segmentation data arrive as
caller owned textures, and this story schedules passes over them without
owning their residency.

### 2. Types, transcribed first and extended only where the HLD stops

The pass enum is section 22 exactly. It is not redesigned.

```rust
pub enum Pass {
    Stack(StackPass),
    VolumeRaycast(VolumePass),
    SegOverlay(SegPass),
}
```

The payloads are the part the HLD does not cover, so they are kept to
scheduling data and GPU handles that already exist as concepts in this
crate. No payload carries pixel data. No payload carries a LUT decision.
The stack payload carries the `VoiParams` value F-041 defined, because a
window change is one of the five dirty sources and the uniform write is
how it reaches the GPU.

```rust
pub struct StackPass {
    pub voi: VoiParams,
    pub blend: BlendMode,
}
pub struct VolumePass {
    pub blend: BlendMode,
}
pub struct SegPass {
    pub blend: BlendMode,
}
```

`BlendMode` carries the five values the parity surface counts. Composite
is the only mode with a shader body in this story. The other four exist
as cache keys and scheduling values so F-052 and F-054 add shader bodies
without changing the key. Declaring a key is not claiming a render, and
the frame path reports a mode with no compiled pipeline as unavailable
rather than substituting composite. That is the honesty rule of deviation
D-07 applied to a blend mode instead of a tier.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    Composite,
    Mip,
    MinIp,
    Average,
    LabelmapEdgeProjection,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassKind {
    Stack,
    VolumeRaycast,
    SegOverlay,
}
```

A viewport is an index into pre-sized storage, following the same
reasoning that made `Edge` and `Passes` distinct types in
`crates/ocelli-render/src/probe.rs`. Two viewport indices transposed at a
call site would dirty the wrong viewport and render a clean one, which is
a wrong frame with no error. One newtype removes that case.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewportId(pub u32);
```

The frame target is a `wgpu::TextureView` supplied at registration. The
graph never creates a surface and never owns the texture behind the view.
F-039 can supply a surface view later and F-040 can supply uploaded
textures later, and neither change reaches the scheduler. Open question 1
records the alternative that the graph owns its offscreen targets, with
the recommendation against it.

**No new trait. No new generic parameter. No `Box<dyn>`.** The declared
extension points in the standing rules are the decoder trait and the
series source and render target traits of section 13. A render target
trait with one caller supplied view as its only implementer today would
fail the two implementer test in `AGENTS.md`, so the concrete
`TextureView` is used until F-039 gives the abstraction a second real
shape to answer to.

### 3. Dirty tracking

Dirt is five flags per viewport, named from section 22 word for word.
The flags live in a pure decision module beside `caps.rs` in spirit, so
the frame decision is testable with no adapter in the CI floor that
deviation D-04 leaves without one. The GPU half only encodes what the
pure half decided.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dirty {
    pub camera: bool,
    pub voi: bool,
    pub slice: bool,
    pub segmentation: bool,
    pub data: bool,
}
impl Dirty {
    pub fn any(self) -> bool;
    pub fn clear(&mut self);
}
```

Rules, each stated once and each testable without a device.

- Registration starts fully dirty on all five flags. A first frame that
  renders nothing because nobody marked it would present as a hang, and
  a fully dirty start makes the first tick correct by construction.
- Marking is idempotent and coalescing. Five marks before one tick are
  one encode of that viewport, not five. The scheduler holds no queue of
  marks and allocates none.
- A tick consumes dirt only after the viewport encodes successfully.
  A failed encode leaves the flags set, so the next tick retries the
  same viewport instead of losing the change that failed.
- Removal clears the slot. Re-registration of the same slot starts fully
  dirty again, for the same reason first registration does.
- Camera, slice and data marks come from the caller through one
  `mark_dirty` entry that takes a `Dirty` value. VOI marks are set by the
  uniform write path in section 6 below and by no other route, so a
  window change cannot update bytes without also scheduling the frame
  that must show them. Segmentation marks follow the same single route
  rule when F-063 gives them a writer.

Storage is one vector of viewport slots sized at graph creation from a
caller supplied capacity. Registration beyond capacity is a refused
registration with a named error, never a growth inside a frame. This is
where the pre-sized buffer of section 26 comes from, and Approach
section 8 returns to it.

### 4. Pipelines compile at init

The cache key is section 22 transcribed into a type. Tier is part of the
key even though one session resolves one tier, because a rebuild after
device loss must not hand back a pipeline compiled for a tier the session
no longer runs, and because F-042 tier B variants then need no key
change.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub pass: PassKind,
    pub blend: BlendMode,
    pub tier: Tier,
}
```

Construction compiles. `RenderGraph::new` builds the shader module from
the composed VOI text, builds the bind group layout that reserves group 0
binding 0 for the section 18.4 uniform, and compiles the composite stack
pipeline for the resolved tier before it returns. A graph that exists
has its pipelines. The frame path only looks up. A lookup miss inside a
frame is a programming error reported as a named frame outcome, and the
lookup path contains no compile call at all, so a mid-frame compile is
unreachable rather than merely avoided.

The fragment entry point is appended to `VOI_WGSL` text at module build
time, exactly as the shader header instructs. The composed source samples
a stored value and writes the result of the transcribed LUT chain. The
stored value source in this story is a caller supplied texture, and the
sampling path is the tier B legal fragment path F-041 named. No compute
entry point is added. No storage buffer is introduced. Tier B legality is
preserved by construction and still not claimed as exercised.

Pipeline compilation failure at creation is a creation error, reported
before any frame exists. It is never deferred into a frame, where it
would read as a rendering failure instead of a startup failure.

### 5. The scheduler and the one submission

The scheduler is a state machine driven from outside. The render worker
owns requestAnimationFrame and calls one tick per callback. Each pointer
or tool event batch that dirtied a viewport during that frame is already
applied before the tick, which is how section 26 batching holds without
this crate touching an event. This crate registers no callback and
crosses no boundary, which keeps deviation D-12 intact.

```rust
pub enum FrameOutcome {
    Submitted { viewports: u32, passes: u32 },
    Noop,
    DeviceLost,
    Unavailable { viewport: ViewportId },
}
pub fn tick(&mut self, ctx: &GpuContext) -> FrameOutcome;
```

One tick does the following, in order.

1. Read `ctx.state()` once. A lost device returns `DeviceLost` before any
   encoder exists. No submission is attempted on a dead device and no
   dirt is consumed, so recovery in section 7 below re-renders every
   affected viewport.
2. Collect the dirty viewports from the pure decision state. None dirty
   returns `Noop`. No encoder is created and no submission is issued,
   which makes a no-op frame a no-op in fact and not only in name.
3. Create one command encoder. Encode each dirty viewport once, in
   registration order, with stack before overlay within one viewport.
   Clear that viewport dirt on successful encode.
4. Finish the encoder and call `queue().submit()` exactly once with that
   one buffer. Return `Submitted` with the counts actually encoded.

The borrow shape is what makes the single submission structural.
`GpuContext::queue()` hands out a shared borrow and nothing else, as
`crates/ocelli-render/src/gpu.rs` already documents, so the graph holds
no second queue handle to submit on. The one submit is one call site in
one function, and the test table below asserts the observable half of
that claim through the returned outcome. No test in this crate can
intercept a `wgpu::Queue` submission, and this plan does not pretend
otherwise. The structural half is reviewed at the call site, which is
the human check section 27.3 of the agent code standards already
requires for GPU code.

A viewport whose VOI chain cannot be expressed returns `Unavailable`
for that viewport and still submits the other dirty viewports in the
same frame. One unrenderable viewport must not blank the rest of the
session. The unavailable viewport keeps its dirt, so a later chain that
can be expressed renders without a fresh mark.

### 6. The VOI uniform path

Each registered viewport owns one uniform buffer created at registration
and sized at thirty-two bytes. A window change builds `VoiParams` through
`VoiParams::from_chain`, writes it with `queue.write_buffer`, and sets
the VOI dirty flag. That is the whole update. No texture is created,
written or rebound on that path, which is the concrete form of section
26 preferring a uniform update to a texture update.

A chain driven by a Modality LUT Sequence or a VOI LUT Sequence has no
section 18.4 uniform. `from_chain` already refuses both with
`VoiParamsError`, and the graph propagates that refusal as the
`Unavailable` frame outcome for that viewport. Substituting rescale or
window values the sequence overrode would render a plausible frame from
numbers the standard did not select, which is the quietly wrong pixel
this project exists to refuse.

The arithmetic stays where section 18 put it. The graph computes no LUT
value on the CPU and adds no window formula to the shader. It moves
thirty-two resolved bytes and schedules the frame that consumes them.

### 7. Device loss, recovery, and the extended `Recovered`

`GpuContext::recover` keeps its current signature character for
character.

```rust
pub async fn recover(&mut self, adapter: &ResolvedAdapter)
    -> Result<Recovered, DeviceError>;
```

`Recovered` stops being a one field struct. It gains the graph resources
that only exist because a device exists, rebuilt on the new device
before `recover` returns, so a caller holding a graph has everything the
rebuild restored in one value and rebuilds nothing twice.

```rust
pub struct Recovered {
    pub caps: Caps,
    pub graph: Option<GraphResources>,
}
pub struct GraphResources {
    pub pipelines_restored: u32,
    pub cache_generation: u64,
}
```

The flow is four steps. The caller observes `DeviceState::Lost`. The
caller calls `recover` on the retained adapter, and F-037 policy still
decides which reasons rebuild. The graph resources registered at graph
creation are recompiled against the new device inside the recovery path
and returned in `Recovered`. The caller installs them into the graph,
which marks every viewport fully dirty, because every GPU object the
old device owned is gone and only a full re-render restores the frame.

Two consequences are stated rather than left to discovery. First,
`Recovered` currently derives `Copy`, and carrying restored resource
state ends that. The derive drops to `Clone` and every existing caller
is updated in the same change. Second, viewport state itself is not in
`Recovered`. Section 22 restores viewport state from the shell copy, the
shell and the boundary that carries that state do not exist yet, and
inventing a viewport state type here would pre-empt E7. The graph marks
viewports dirty and the caller re-supplies their state through the same
registration and VOI calls it already makes. Open question 3 records
the alternative payload shapes.

Recovery failure leaves the context in its lost state, which is the
guarantee F-037 already documented for `DeviceError::Refused`, and the
graph is untouched in that case. A half installed pipeline set after a
failed rebuild would be worse than a lost device honestly reported.

### 8. Allocation discipline in the frame path

No Rust heap allocation happens inside `tick`. The viewport slot vector
is sized at graph creation. The dirty scan reads that vector in place.
The pipeline cache is a lookup only during a frame. The per viewport
uniform buffers exist from registration. The one object created per
submitted frame is the `wgpu` command encoder itself, which is a driver
handle rather than a Rust allocation this crate controls, and a frame
with no dirty viewport creates none. Any future need to grow capacity
belongs to registration, which is a control path call, and never to the
frame path. The benchmark harness is not used to justify any of this
before the fact, per section 26, and no performance number is claimed
in this plan.

## Boundary and tier

- wasm-bindgen: not touched. The scheduler takes ticks from its caller and registers no browser callback, which is deviation D-12 holding in the new code exactly as it holds in `probe.rs` today.
- Pixels across the boundary: no. Frames render into GPU resident texture views. Nothing is mapped, read back or returned to a caller as pixels. Host side test readback, where a test needs one, stays inside the native test and never reaches `ocelli-wasm`, which is the same line F-041 drew.
- Render-loop allocation: none in `tick`. The pre-sized buffers are the viewport slot vector sized at graph creation, the pipeline cache built at graph creation, and one thirty-two byte uniform buffer per viewport built at registration. Details in Approach section 8.
- unsafe: none. No new `unsafe` block is introduced and neither permitted file is touched.
- Tier A (WebGPU): full. The graph, the scheduler, the pipeline cache and the composed VOI fragment pipeline all run. No pass in this story needs compute, so no tier A only path is created.
- Tier B (WebGL2): full by construction, not yet exercised. The frame path uses fragment pipelines and uniform buffers only, with no compute dispatch and no storage buffer, which is the tier B legal set section 7 defines. The pipeline key carries the tier so F-042 can add variants without a key change. No test in this story runs on a downlevel adapter, because none exists in any gate, and this row claims legality rather than execution. F-042 in S13 and F-X002 in S14 are the stories that change that.
- Tier C (CPU): unavailable, and reported as such. A tier C session opens no device under deviation D-07, holds no `GpuContext`, and therefore constructs no graph. The caller learns this at graph creation as a named unavailable result rather than as a graph that silently renders on the CPU. The CPU LUT arithmetic in `ocelli-pixel` remains the tier C path for values, and F-X003 owns the CPU stack raster that will consume it. This story reimplements none of that arithmetic behind a tier check.

## Tests

| Category | What it proves | Where |
|----------|---------------|-------|
| `fixture` | The graph writes the exact thirty-two uniform bytes for the section 18.3 soft tissue chain. Centre 40, width 400, slope 1, intercept 0, range 0 to 255, no inversion gives `fn_kind` 0 and the eight field values `VoiParams::from_chain` already pins, read back from the viewport uniform path rather than from a locally built struct. DICOM PS3.3 C.11.2.1.2 for the window and section 18.4 for the layout | `crates/ocelli-render/tests/graph_voi.rs` |
| `fixture` | The four section 18.3 rows through the composed production fragment pipeline. Inputs minus 160, 40, 240 and minus 60 give LINEAR 0.000, 127.819, 255.000 and 63.910, and LINEAR_EXACT 0.000, 127.500, 255.000 and 63.750 with D-13 applied to row one. Hand computed from PS3.3 C.11.2.1.2 and C.11.2.1.3.2, not derived from `LutChain::map_into`. This is the mandatory fixture for pixel arithmetic and it runs on the graph shader rather than only on the F-041 harness | same |
| `fixture` | SIGMOID at centre 40 width 400 input minus 60 is 255 divided by one plus e, from PS3.3 C.11.2.1.3.1 with the exponent exactly plus one. Independent of any library `exp` on either side | same |
| `fixture` | Inversion through the graph is a reflection about the midpoint. Range 16 to 235 with a value mapping to 100 inverts to 151 under PS3.3 C.11.6. The `ymax` minus value form gives 135 and is separated by this row | same |
| `unit` | A new viewport starts dirty on all five flags, each of camera, VOI, slice, segmentation and data named in its own assertion. No adapter needed | `crates/ocelli-render/src/graph.rs` tests |
| `unit` | Marking is coalescing and idempotent. Five marks on one viewport before one decision pass yield one scheduled encode, and a clean viewport yields none | same |
| `unit` | Dirt survives a failed encode and clears after a successful one, per viewport, with one viewport of each kind in the same frame decision | same |
| `unit` | Registration beyond the capacity fixed at creation is refused with the named error and changes no existing slot | same |
| `unit` | `PipelineKey` distinguishes pass kind, blend mode and tier, with one differing field per row, so a tier B lookup can never return a tier A pipeline | same |
| `unit` | A blend mode with no compiled pipeline resolves to unavailable and never to composite, with MIP as the driving case | same |
| `unit` | `Recovered` carries the restored pipeline count and generation after a rebuild decision, and carries no graph resources when no graph was registered. Decision level only, no adapter | `crates/ocelli-render/src/gpu.rs` tests |
| `browser` (device) | A graph creates on a real adapter with its composite stack pipeline compiled before the first tick, and the first tick submits the fully dirty first frame. Ignored, needs a real adapter, run by the `gpu` gate | `crates/ocelli-render/tests/graph.rs` |
| `browser` (device) | Two dirty viewports in one tick produce one `Submitted` outcome covering both viewports, and a clean tick immediately after produces `Noop` with no submission. The outcome is the observable half of the one submit rule | same |
| `browser` (device) | A window change writes thirty-two bytes and creates no texture, then the next tick renders the new window. Asserted by the absence of a texture write on the recorded path and by the changed output value at the window centre, 127.819 under LINEAR | same |
| `browser` (device) | A viewport whose chain is driven by a LUT Sequence returns `Unavailable` for that viewport while a second healthy viewport in the same tick still submits | same |
| `browser` (device) | Device loss observed through the graph. A lost device makes `tick` return `DeviceLost` without consuming dirt, recovery through the unchanged `recover` signature returns the extended `Recovered`, and the following tick re-renders every viewport from fully dirty | same |
| `browser` (device) | No pipeline compiles during a frame. Every key the frame path requests was compiled at creation, asserted by exhausting the declared key set before the first tick and by the lookup only shape of the frame path | same |

On the `browser` (device) label, see the test table note in the F-037
plan. The workflow taxonomy has no name for a native test that needs a
real adapter, `browser` is the nearest row, and the label is qualified
rather than invented. Every device row above is ignored by default under
deviation D-04 and run by `bin/ocelli.sh gate gpu` with one thread, for
the adapter contention reason `docs/sprints/CURRENT_SPRINT.md` records.

**Mutation check, per the human review list in the agent code
standards.**

1. Submit inside the per viewport loop instead of once after it. The two
   viewport row reports two submissions in its counts and goes red on
   the outcome assertion, and the call site count is caught in review.
2. Compile on a cache miss inside the frame. The no compile row goes red
   because the frame path gains a compile call the test setup did not
   perform.
3. Clear dirt before encode instead of after. The failed encode row goes
   red because the change that failed is silently lost.
4. Share one uniform buffer across viewports. The two window centre row
   goes red because the second window overwrites the first in one frame.
5. Substitute composite for a missing blend pipeline. The MIP row goes
   red because unavailable became a different image.
6. Drop the tier from the pipeline key. The key row goes red because a
   tier B lookup returns the tier A pipeline.
7. Keep the old `Copy` derive on `Recovered` by dropping the restored
   resource fields. The recovery row goes red because nothing restored
   reaches the caller.

## Parity surface covered

`docs/hld/B-parity-surface.md` carries no `Covered by` column in this
repository, so coverage is stated against its surface table by epic ref
E6.2 from `docs/sprints/allocation.json` rather than by a column lookup.

| Surface row in Appendix B | What F-038 does to it |
|---------------------------|-----------------------|
| Blend modes, count 5, composite, MIP, MinIP, average, labelmap edge projection | The five values become pipeline cache keys and scheduling values. Composite gains a compiled shader body. The other four are declared and report unavailable until F-052 and F-054 give them bodies. The row is structured by this story and completed by later ones |
| VOI LUT functions, count 3 | No new function is added. The three functions F-041 compiled reach a production fragment pipeline here for the first time, scheduled by the graph rather than by a test harness |
| Segmentation representations, count 3 | The `SegOverlay` pass kind from section 22 is schedulable and ordered after the stack pass for the same viewport. No representation renders until F-063, and this story claims scheduling only |
| Viewport types, count 12 | None is implemented. The graph registers caller supplied viewports of no named type, because viewport types belong to E7. Claiming any of the twelve here would be false |

No other Appendix B row is touched. Transfer syntaxes, tool classes,
core events and adapters are outside a render graph, and this plan lists
them as untouched rather than leaving the reader to infer it.

## Deviations

None new. No row is added to `docs/hld/DEVIATIONS.md` by this plan.

Four declared deviations are consumed and none is extended. Deviation
D-04 is why every device test above is ignored by default and run by the
`gpu` gate. Deviation D-07 is why tier C constructs no graph and reports
unavailable, and why an uncompiled blend mode reports unavailable
instead of substituting another image. Deviation D-12 is why the
scheduler takes ticks from its caller and registers no browser callback.
Deviation D-13 is applied to row one of the section 18.3 fixture, as
F-011, F-029 and F-041 applied it before. Deviation D-14 is inherited
through the `webgl` feature on the existing `wgpu` dependency and gains
no new surface here.

The two readings of blend selection in section 7 and section 22 are not
a deviation. Keying a pipeline by blend mode implements both sentences
at once, and the implementation reads the pinned `wgpu` pipeline API
before choosing the mechanism that carries the selection into the
shader. If that reading proves impossible against the pinned API, the
gap comes back as a design question with a deviation row, and it is not
improvised inside the implementation.

## LLD impact

- `docs/lld/gpu-ownership.md` gains the extended `Recovered`, the graph
  resource registration that feeds it, and the statement that the one
  submission is one call site behind the existing queue borrow. Its
  current sentence that F-038 to F-040 extend `Recovered` is replaced by
  what this story actually put there.
- A new `docs/lld/render-graph.md` records the graph as built. Pass kinds
  and payloads, the five dirty flags and their consume rule, the pipeline
  key, the tick outcomes, pass order, and the two things the graph does
  not do, which are presentation and upload. Indexed in
  `docs/lld/README.md` in the same change, with its header lines kept in
  step by hand as that file instructs.
- `docs/lld/feature-availability.md` gains three rows. Graph creation on
  tier C, a blend mode with no compiled pipeline, and a VOI chain driven
  by a LUT Sequence on the GPU path. Each reports unavailable and none
  substitutes a different result.
- `docs/lld/benchmarks.md` is checked and expected to change only in its
  pointer accuracy. The `render.first_frame` subject stays unavailable,
  because a first frame with real pixels still waits on F-040 upload,
  and this plan does not let a scheduling milestone read as a rendering
  measurement.

## Open questions

None. All questions were answered in the S12 consolidated design round, as recorded below.

## Decisions from the S12 design round

**Question 1, Option A. The caller supplies a `wgpu::TextureView` per viewport** at registration and the graph never owns a target. Presentation stays with F-039 and upload with F-040.

**Question 2, Option A. The caller ticks** once per requestAnimationFrame callback. The scheduler is a pure state machine and no browser binding enters this crate, per deviation D-12.

**Question 3, Option A. `Recovered` carries the rebuilt graph resources** themselves, with counts and a generation. `Recovered` drops `Copy` to `Clone`, paid openly.

**Question 4, Option A with the key space of Option B.** The key type carries every pass, blend and tier combination from the start. Only the composite stack pipeline compiles at creation, and an uncompiled key reports unavailable rather than aliasing composite.

**Question 5, Option A. Rust-owned dirty flags per viewport,** set through `mark_dirty` and the single VOI write route, consumed by the tick.

## Recorded questions

### Question 1, what does a frame render into

- Option A. The caller supplies a `wgpu::TextureView` per viewport at
  registration and the graph never owns a target. Presentation and
  upload stay entirely with F-039 and F-040.
- Option B. The graph allocates and owns one offscreen texture per
  viewport at registration and hands views out to callers.
- Option C. The graph acquires surface textures now and F-039 adapts to
  a presentation path this story chose for it.
- **Recommendation: Option A.** It needs no surface before F-039 and no
  upload before F-040, it keeps target ownership with the stories the
  sprint plan gives those jobs to, and it is the only option under which
  the native device tests need no canvas at all. Option B duplicates the
  allocator F-040 will need for residency. Option C spends F-039 scope
  inside S12 and couples the scheduler to a presentation API nothing
  here can test.

### Question 2, who drives the scheduler

- Option A. The caller ticks once per requestAnimationFrame callback
  and the scheduler is a pure state machine over dirty flags and ticks.
- Option B. The scheduler registers its own frame callback through a
  boundary this crate does not currently have.
- Option C. The scheduler polls on a timer owned by the graph.
- **Recommendation: Option A.** It is the only option that keeps a
  browser binding out of this crate under deviation D-12, it makes every
  scheduler behaviour in the test table reachable with no browser, and
  it matches how the clock already enters `probe::resolve` from its
  caller for the same reason. Option B rebuilds the boundary inside a
  render story. Option C spends frames nobody asked for and fights the
  worker event loop it would live beside.

### Question 3, what the extended `Recovered` carries

- Option A. The rebuilt graph resources themselves, ready to install,
  with counts and a generation for assertion. `Recovered` drops `Copy`.
- Option B. Counts and a generation only, with the graph rebuilding its
  own resources after observing the recovery. `Recovered` keeps `Copy`.
- Option C. A rebuild callback registered on the context at graph
  creation, invoked inside `recover`, with `Recovered` unchanged.
- **Recommendation: Option A.** The tasking for this story says
  `Recovered` gains the resources a rebuild restores, and Option A is
  the only reading under which the caller rebuilds nothing twice and
  can assert what came back. Option B leaves the resource rebuild
  outside the value that claims to report recovery. Option C leaves
  `Recovered` exactly as F-037 wrote it and hides the rebuild inside a
  callback the return value never mentions. The cost of Option A is
  the lost `Copy` derive, and it is paid openly in Approach section 7
  rather than avoided by carrying less than the story was asked to
  carry.

### Question 4, which pipelines compile at creation

- Option A. Only the composite stack pipeline. Other keys are added by
  the stories that give them shader bodies.
- Option B. All fifteen combinations of three pass kinds, five blend
  modes and the resolved tier, with unavailable reported for keys whose
  shader body does not exist yet.
- Option C. All five blend modes for the stack pass only.
- **Recommendation: Option A, with the key space of Option B.** The key
  type carries every combination from the start, so no later story
  changes the key. Only composite compiles, because compiling a pipeline
  around a shader body nobody wrote would either fail at creation or
  silently alias composite, and the second of those is the substitution
  deviation D-07 forbids. A lookup for an uncompiled key reports
  unavailable. Options B and C both spend startup compile time on
  pipelines with nothing correct to run.

### Question 5, where dirty truth lives

- Option A. Rust owned flags per viewport, set through `mark_dirty` and
  through the single VOI write route, consumed by the tick.
- Option B. The shell diffs viewport state each frame and hands the
  graph a dirty list per tick.
- Option C. Every state write renders eagerly and no dirty state exists.
- **Recommendation: Option A.** Section 22 assigns dirty tracking to the
  graph, and flags beside the slots keep the decision testable in the
  CI floor with no shell in the loop. Option B moves the decision
  across the boundary once per frame and makes it untestable here.
  Option C re-renders on every write, which is the behaviour dirty
  tracking exists to remove, and it breaks the one submission rule
  whenever two writes land in one frame.




