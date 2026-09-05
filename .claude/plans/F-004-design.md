# F-004, runtime capability detection and tiering (WebGPU / WebGL2 / SIMD / threads)

**Status**: approved
**Epic ref**: E1.4
**Sprint**: S03
**Estimate**: 2w

## Normative source, transcribed

_Every block below is quoted inside a fence, so nothing is reworded and no
character is normalised. `scripts/prose_check.py` exempts fenced content, which
is why the fence is used here rather than a blockquote: F-008's plan had to
convert em-dashes and semicolons inside its quotations, and a fence avoids
having to touch the author's text at all. Where the exact bytes matter,
`docs/hld/` wins._

### `docs/hld/05-rendering.md`, section 7, in full

```text
## 7. Rendering

- **Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup.

- **Volume rendering runs on both tiers**, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute — GPU segmentation, histogram passes, compute-based resampling — is tier A only and must degrade, not fail.

- **Bricking above 256 MiB.** A 512×512×600 sixteen-bit CT series is roughly 300 MB against a guaranteed maximum buffer size of 256 MiB, so chunked upload is the normal path, not an optimisation.

- **One submit per frame.** The render graph tracks dirty viewports and issues a single submission across all of them, driven by requestAnimationFrame inside the render worker.

- **Blend modes are shader variants** — composite, MIP, MinIP, average — selected by specialisation constant rather than branching per fragment.
```

### `docs/hld/19-render-graph.md`, section 22, the `Caps` shape, character for character

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

and the bullet this story has to keep true:

```text
- **Pipelines compile at init**, keyed by (pass kind, blend mode, tier). Never compile a shader mid-frame.
```

**Section 7 does not contain `Caps`.** The tier definitions and the
degrade-never-fail rule are section 7. The struct is section 22, in
`19-render-graph.md`, and both are needed.

### `docs/hld/07-concurrency-and-typescript.md`, section 9, in full

```text
## 9. Concurrency

**Start single-threaded per worker.** N decode workers, each holding its own WebAssembly instance; one render worker owning the GPUDevice and every OffscreenCanvas; the main thread doing DOM, events and tool UI. No SharedArrayBuffer, no wasm-bindgen-rayon, no nightly toolchain.

| **WHY NOT THREADS** Shared-memory threading costs a pinned nightly toolchain, -Z build-std, dual threaded and non-threaded builds with feature detection, and COOP/COEP headers — a permanent tax on the build. The dicom-rs guidance is literally to set up wasm-bindgen-rayon or disable rayon, and the multi-instance design is a legitimate answer. Escalate only on a measurement that demands it. |
```

and decision D5, from `docs/hld/11-decision-log.md`:

```text
| D5 | Single-threaded, one wasm instance per worker | Threads via SharedArrayBuffer | Stable Rust, no COOP/COEP, no nightly |
```

Section 10 fixes where the environment is read from:

```text
- DOM, pointer, touch and wheel events; canvas lifecycle; ResizeObserver
```

### `docs/hld/26-differentiating-capabilities.md`, section 31, the rule this story generalises

```text
pub trait Kernel {
    fn tier(&self) -> Tier; // A = WebGPU only, B = has fallback
    fn workgroup(&self, caps: &Caps) -> [u32; 3];
    fn dispatch(&self, ctx: &mut ComputeCtx) -> Result<(), ComputeError>;
}
```

```text
- **Shares the renderer's device.** ocelli-compute never creates a wgpu::Device; it borrows the one ocelli-render owns. Two devices cannot share textures, which would defeat the entire point.

- **Every tier-A kernel declares a fallback** — CPU, or a worker — so a feature degrades rather than fails on WebGL2. A kernel with no fallback marks its feature unavailable; it never silently produces a different answer.

- **Workgroup sizes come from `Caps`**, never hardcoded. A hardcoded 256 is a portability bug waiting for a device that reports less.
```

### `docs/hld/27-phase1-hooks.md`, section 38, in full

```text
## 38. The hooks Phase 1 must include

Everything above is post-parity except these. Each costs a few weeks now and a rewrite later, which is the only reason they appear in a parity plan at all.

| **Hook** | **Story** | **Now** | **Later** |
|----|----|----|----|
| Chunked residency in the cache | E5.6 | 4 wk | Re-architecting the cache and every consumer of it |
| Multiscale level axis on the volume | E8.8 | 3 wk | Changing the type every viewport and tool reads |
| SR as the native annotation type | E15.1 | 4 wk | Migrating stored annotations and every tool that writes them |
| ocelli-compute crate exists | E1.8 | 2 wk | A device-sharing retrofit across the renderer |
| Stable render hashes from the oracle | E2.7 | 2 wk | Little - but free here, since E2 computes them anyway |

**Phase 1 grows from 382 to 397 engineer-weeks** to carry these. That is the whole cost of keeping every option in Part III open.
```

F-004 is not itself a section 38 row. It is the story the ocelli-compute row
depends on, because `Kernel::workgroup(&self, caps: &Caps)` reads a `Caps` that
until now nothing fills.

### `docs/hld/DEVIATIONS.md`, D-07, the row

```text
| D-07 | §7, "Two capability tiers, one codebase", both of them GPU | A third tier, **C, CPU**. The resolved tier may be `Cpu`, and every tier-gated feature declares its CPU answer | §7 leaves a machine with neither WebGPU nor WebGL2 rendering nothing at all, which is a failure mode the specification does not name and does not intend. Operator decision, and spike A7.1 establishes GPU-less sessions as a primary clinical path rather than a fallback. F-X001 to F-X004. | Post-bootstrap |
```

and the two consequences from its explanatory section that are this story's
whole content:

```text
- **Tier resolution must tell a hardware adapter from a software one.** On a
  host with no GPU, a software rasteriser presents a conforming WebGL2
  context, so `Caps` as §7 specifies it resolves tier B and runs GPU paths on
  a rasteriser that is slower than our own CPU path and burns more CPU. On a
  shared host, CPU is the resource that decides how many sessions fit. Worse,
  it is invisible, and presents as "the viewer is slow" rather than as a
  misdetection.
- **The divergence bound has to cover tier A against tier C.** A mixed estate
  means two radiologists can open the same study and see pixels from different
  code paths. Decision D14 already commits to publishing a measured divergence
  bound rather than claiming bit-exactness, and that commitment now extends
  across tiers, not only across GPUs and targets.
```

```text
- **wasm SIMD128 becomes a requirement** rather than a detection detail, and
  the no-SIMD runtime is measured separately as the worst case.
- **Decision D5 holds, for a second reason.** D5 keeps the build
  single-threaded and says to escalate only on a measurement that demands it.
  A CPU renderer carrying a large share of the load looks like that
  measurement and is not: on a shared host, spending more cores per session
  reduces sessions per host, which is that deployment model's whole
  economics. Tier C is judged on CPU spent, not on wall-clock alone.
```

### `docs/spikes/A7-tier-c.md`, the finding and its detection list

```text
**A software rasteriser reports itself as WebGL2, and the tier logic as
specified would believe it.**

HLD §7 resolves the tier once at startup from what the platform reports. On a
host with no GPU, Chrome commonly falls back to SwiftShader, which presents a
conforming WebGL2 context. `Caps` sees WebGL2, resolves **tier B**, and Ocelli
then runs GPU code paths on a software rasteriser.
```

```text
Detection signals, in order of reliability. None is sufficient alone, so treat
this as evidence to combine and always allow an operator override:

- The `WEBGL_debug_renderer_info` unmasked renderer string, matched against
  known software renderers: `SwiftShader`, `llvmpipe`, `softpipe`,
  `Microsoft Basic Render Driver`, `Gallium`, `Mesa OffScreen`, `ANGLE (Software`.
- The WebGPU adapter's reported type, where a fallback adapter identifies
  itself as one.
- A **startup micro-benchmark**, which is the only signal that measures the
  thing we actually care about rather than a string a vendor chose. It is also
  the only one that survives a renderer string being masked for privacy, which
  browsers increasingly do.

**The micro-benchmark is the one to trust, and the strings are the hint.** A
renderer string is a claim. A measured fill rate is a fact.
```

```text
GPU client         ->  WebGPU              ->  TIER A
GPU-less session   ->  no adapter, or a
                       software rasteriser ->  TIER C
```

```text
- **F-006 (E1.6), the benchmark harness, must measure tier C from the start**,
  including CPU cost per session. A harness that only measures a GPU path
  cannot answer A7.3.
```

### `docs/sprints/CURRENT_SPRINT.md`, what done means and the defect class

```text
- **F-004** resolves a tier and distinguishes a hardware adapter from a
  software one, with the micro-benchmark, and an operator override exists.
```

```text
The dangerous tiering defect is deviation D-07's, stated in
`docs/spikes/A7-tier-c.md`: on a host with no GPU a software rasteriser
presents a conforming WebGL2 context, so `Caps` as HLD section 7 specifies it
resolves tier B and runs GPU paths on a rasteriser slower than our own CPU
path. It is invisible, and it presents as "the viewer is slow" rather than as a
misdetection. Detect by renderer string, adapter type **and** a startup
micro-benchmark, and trust the benchmark.
```

### `docs/sprints/allocation.json`, the F-004 note

```text
"notes": "Tier A = compute available; Tier B = fragment-only"
```

### `docs/hld/25-first-ten-files.md`, row 7 and row 8

```text
| 7 | crates/ocelli-render/src/caps.rs | Tier detection, so tier assumptions are explicit from the start |
| 8 | crates/ocelli-render/src/device.rs | Device init and loss recovery |
```

Row 7 names `caps.rs` as the home of tier detection, which is why this story
extends that file rather than inventing a new name. Row 8 is F-039 and this
story does not create it.

### What already exists in the tree, and is a given

`crates/ocelli-render/src/caps.rs`, written by F-008, already holds section
22's `Caps` field for field and a three-variant `Tier` carrying D-07's `Cpu`.
Its own docstring scopes this story:

```text
//! **This module defines the type. It does not detect it.** Adapter
//! enumeration and tier resolution are F-004 (E1.4), and device creation is
//! F-039. F-008 needs the type because HLD section 31's `Kernel::workgroup`
//! takes a `&Caps`, and a hook expressed in types needs the types.
```

`crates/ocelli-render/src/gpu.rs` holds `GpuContext` and `SharedEncoder`, and
`ci/check-device-ownership.sh` refuses any crate outside `ocelli-render` that
names `Instance::new`, `request_adapter`, `request_device` or `create_surface`.

**F-004 is the detection half only.** Neither `Tier` nor `Caps` changes shape.

## What the specification does not cover

The HLD gives one sentence of resolution policy, "the tier resolves once at
startup", and the struct the result lands in. Everything below is a decision
this plan makes, and each one is a place a reviewer should push back.

1. **What "resolves" reads.** Section 7 implies the platform's own report.
   D-07 and A7 say that is precisely the defect, so this plan resolves from
   three signals and states the combination rule explicitly rather than
   leaving it to the order of an `if` chain.
2. **What the micro-benchmark measures, and its threshold.** A7 says "a
   measured fill rate is a fact" and stops there. It gives no workload, no
   size, no iteration count and no number. A7.3 forbids inventing an absolute
   figure elsewhere, so this plan records a calibrated pair rather than
   inventing one.
3. **How the two hint signals and the measurement combine when they disagree.**
   "Trust the benchmark" is a priority, not a procedure, and the interesting
   cases are the ones where the benchmark did not run at all.
4. **What the override is, where it comes from and what it may not do.** A7
   says "always allow an operator override" and nothing else.
5. **Whether a refused override is an error.** This plan says it is a recorded
   outcome and not a `Result`, which is D-07's own honesty rule applied to the
   resolver itself.
6. **Where `max_tex_3d` and `max_buffer` come from.** Section 7 gives tier
   figures (2048, 256, 256 MiB) that read as guarantees a feature may assume.
   Section 22 gives fields that read as what the adapter reports. They are not
   the same thing.
7. **How SIMD is detected.** Section 9 covers threads and says nothing about
   SIMD. D-07 promotes SIMD128 to a requirement and points at this story
   without saying what detection means for a module that cannot start without
   it.
8. **Where threads are detected, given that D5 fixes the answer.** Detecting
   something nobody may act on needs a stated reason or it should not exist.
9. **Whether the resolver is async.** `Instance::request_adapter` and
   `Adapter::request_device` are futures in wgpu 30.0.1, so this is forced,
   but nothing in the HLD says who drives them.

## Approach

The story splits into a pure half and an I/O half, and the split is the point.
**Everything that can be wrong is in the pure half, and the pure half needs no
GPU**, so the dangerous decision is exhaustively testable in the CI floor that
deviation D-04 leaves us with.

### 1. `crates/ocelli-render/src/caps.rs` gains the decision procedure, and no I/O

`Caps` and `Tier` are untouched. The file gains:

```rust
pub enum SoftwareVerdict { Hardware, Software, Unknown }

pub struct AdapterFacts {
    pub backend: wgpu::Backend,
    pub device_type: wgpu::DeviceType,
    pub name: String,
    pub driver: String,
    pub driver_info: String,
    pub compute_shaders: bool,
    pub webgpu_compliant: bool,
    pub max_tex_3d: u32,
    pub max_buffer: u64,
}

pub struct FillRate { pub pixels_shaded: u64, pub elapsed_nanos: u64 }

pub struct TierSignals {
    pub adapters: Vec<AdapterFacts>,
    pub fill_rate: Option<FillRate>,
    pub device_created: bool,
    pub simd: SimdSupport,
}

pub struct TierEvidence { /* every signal, every verdict, and decided_by */ }
pub struct Resolution { pub caps: Caps, pub evidence: TierEvidence }

pub fn classify(signals: &TierSignals, request: Option<Tier>) -> Resolution;
```

No trait and no generic parameter, because there is one implementer of each
thing here and `AGENTS.md` forbids the shape. `TierSignals` is a plain struct
that the probe fills and `classify` reads.

### 2. The decision procedure, written out, because an `if` chain is not a specification

1. **An override of `Tier::Cpu` short-circuits everything.** No instance, no
   adapter, no device, no benchmark. This is not only tidy, it is the estate
   D-07 names getting its startup cost back, and `decided_by` records
   `Override`.
2. **Rank candidates.** An adapter on `Backend::BrowserWebGpu`, `Vulkan`,
   `Metal` or `Dx12` whose `DownlevelFlags::COMPUTE_SHADERS` is present is an
   **A-candidate**. An adapter on `Backend::Gl`, or one without
   `COMPUTE_SHADERS`, is a **B-candidate**. `Backend::Noop` is never a
   candidate. Prefer the best A-candidate, then the best B-candidate.
3. **No candidate at all resolves `Tier::Cpu`**, `decided_by = NoAdapter`.
4. **Judge the chosen candidate hardware or software**, by combining three
   verdicts:
   - **benchmark**: `Hardware` at or above the recorded hardware floor,
     `Software` at or below the recorded software ceiling, `Unknown` between
     them.
   - **adapter type**: `DeviceType::Cpu` is `Software`. `DiscreteGpu` and
     `IntegratedGpu` are `Hardware`. `VirtualGpu` and `Other` are `Unknown`.
   - **renderer string**: A7's list, case-insensitively, over `name`, `driver`
     and `driver_info` together. A match is `Software`. **A non-match is
     `Unknown` and never `Hardware`**, because the absence of a string proves
     nothing, and browsers increasingly mask the string entirely.
5. **The combination rule, and it is the load-bearing paragraph.**
   - No device could be created, so no benchmark ran: `Tier::Cpu`,
     `decided_by = NoDevice`. There is no GPU path to be had.
   - The benchmark ran and said `Hardware` or `Software`: **it decides**, and
     the two hints are recorded as evidence and not consulted. This is A7's
     "trust the benchmark" made mechanical.
   - The benchmark ran and was `Unknown`: fall to the adapter type, then to
     the string, and if both are `Unknown` keep the GPU candidate. A figure
     that landed between two bands set an order of magnitude apart is itself
     evidence that something real executed.
6. **Hardware resolves the candidate's tier**, A or B. **Software resolves
   `Tier::Cpu`.** That single line is the whole defect this story exists to
   prevent.
7. **The override applies last and is clamped to what is constructible.**
   `Cpu` always is. `B` needs some adapter. `A` needs an A-candidate. An
   override that is not constructible is **refused, recorded, and the measured
   tier stands**. Forcing `B` onto a rasteriser the benchmark called software
   is deliberately allowed, because that is how the misdetection gets
   diagnosed, and it is recorded so it is never silent.
8. **`Caps` is filled from the chosen adapter and not from section 7's tier
   figures.** `compute` is true only for `Tier::A` with `COMPUTE_SHADERS`
   present. `max_tex_3d` and `max_buffer` are the adapter's reported
   `Limits::max_texture_dimension_3d` and `Limits::max_buffer_size`, unclamped.
   Section 7's 2048, 256 and 256 MiB are floors a feature may assume for its
   tier, and clamping the reported truth down to them would make `Caps` lie in
   the direction of "we have less than we do", which is a different lie but
   still a lie. For `Tier::Cpu` all three are `false`, `0` and `0`, because
   there is no device and any other value would be invented.

### 3. `crates/ocelli-render/src/probe.rs` holds the wgpu calls and the workload

New module, because `caps.rs` should stay readable as one decision and because
this half needs a GPU to run while that half does not.

- `wgpu::util::new_instance_with_webgpu_detection` builds the instance. It
  exists in 30.0.1 at `src/util/init.rs:116` and it is the documented answer to
  `navigator.gpu` being defined on a browser that cannot actually make a
  WebGPU adapter, which is exactly the population D-07 cares about.
- `Instance::enumerate_adapters(Backends)` natively, `Instance::request_adapter`
  on wasm32 where enumeration is not meaningful. `Adapter::get_info()`,
  `Adapter::limits()` and `Adapter::get_downlevel_capabilities()` fill
  `AdapterFacts`. All four are verified present in wgpu 30.0.1.
- **The renderer string arrives through wgpu and needs no browser binding.**
  `wgpu-hal-30.0.1/src/gles/adapter.rs` reads `GL_UNMASKED_RENDERER_WEBGL`
  when `WEBGL_debug_renderer_info` is available, and glow enables that
  extension by default on `wasm32-unknown-unknown`. So `AdapterInfo.name` is
  already the unmasked string A7 asks for, and `ocelli-render` imports no
  web-sys of its own. D2 and D-12 are untouched by this story.
- **The workload.** A full-viewport triangle into an offscreen
  `Rgba8Unorm` texture, with a fragment shader doing a fixed count of ALU
  operations, repeated a fixed number of passes, one `queue.submit`.
  Fragment throughput is the thing that separates SwiftShader from a real
  adapter by orders of magnitude, and it is also what a stack viewport
  actually spends its frame on.
- **Two stages, so a slow rasteriser does not hang startup.** A small
  calibration pass first. The full pass runs only if the calibration was fast
  enough to make it affordable, and if it was not, the calibration figure is
  itself the answer and it is already in the software band.
- **The pixels are never read back.** No `map_async`, no readback buffer. The
  benchmark measures shading, not transfer, and a readback would put pixels on
  a path they have no business being on.
- **Completion, and the part that is genuinely awkward.** `Device::poll`
  blocks on wgpu-core backends and its own doc says "When running on WebGPU,
  this is a no-op." So the portable completion signal is
  `Queue::on_submitted_work_done(callback)`, which sets a flag, with
  `Device::poll(PollType::Poll)` driving it natively. `resolve` is therefore
  `async fn`, which it had to be anyway because `request_adapter` and
  `request_device` are futures.
- **The clock is the caller's.** `probe` returns `pixels_shaded` and does not
  time itself. `std::time::Instant` panics on `wasm32-unknown-unknown`, and
  the alternative is a new dependency reaching `performance.now()` inside
  `ocelli-render`, which is exactly the browser binding D-12 says this crate
  must not grow. The caller supplies `elapsed_nanos`: natively `Instant`, in
  a browser the shell's `performance.now()` delta through the boundary.
- The workload is sized so a hardware adapter takes at least a few
  milliseconds, which puts it far above `performance.now()` coarsening in a
  page that is not cross-origin isolated.
- **The probe device is transient.** It is created, measured on and dropped
  before `resolve` returns. It never becomes a `GpuContext`, so section 31's
  one-device invariant is untouched: there is never a moment when two devices
  exist. The call sits inside `ocelli-render`, so
  `ci/check-device-ownership.sh` still passes unchanged.

### 4. The threshold is recorded, never invented

`ci/tier-thresholds.json`, in the shape `ci/wasm-size-budget.json` already
established: a recorded measurement with its provenance, not a constant in
source. Two numbers with a deliberate gap, a hardware floor and a software
ceiling, so a figure between them is reported `Unknown` rather than forced to
a side. The file records which machine and which adapter produced each, and
the date.

A7.3's rule is "do not invent a number", and it applies here as much as to the
CPU budget. See open question 4: the calibration measurements have to be
taken before this file can hold anything honest.

### 5. The arithmetic is integer, and there is no `as` cast anywhere in it

The comparison is not `pixels / seconds >= threshold`. It is

```text
pixels_shaded * 1_000_000_000  >=  threshold_pps * elapsed_nanos
```

widened to `u128` on both sides. No float, so `float_cmp = "deny"` has nothing
to say, no `as`, so HLD 27.3's cast review has nothing to find, and no
rounding, so there is no rounding decision to get wrong. `f64::from` is
available for any reporting figure and is lossless from `u32`.

### 6. The operator override, and its named user

- `Tier::from_override_str` parses `"a"`, `"b"`, `"cpu"` and `"auto"`,
  case-insensitively. `auto` means no override, so a deployment can set the
  variable unconditionally. Anything else is a **refused** override, recorded,
  never a silent ignore.
- Natively the value is `OCELLI_TIER`, read by `ocelli-native`'s two entry
  points, which then print the resolved tier and its evidence alongside the
  banner they already print. That is the override's named user, and it makes
  `bin/ocelli.sh native` step 1 link something real. See open question 3.
- In a browser the value comes from the shell, because reading a query
  parameter, a config endpoint or `localStorage` is DOM work and section 10
  puts DOM work in TypeScript. `ocelli-render` receives a parsed `Option<Tier>`
  and reads no environment of its own.

### 7. SIMD, and the honest thing to say about detecting it

`SimdSupport` is `Simd128`, `None` or `NotApplicable`, derived from
`cfg!(target_arch = "wasm32")` and `cfg!(target_feature = "simd128")`. It is
**a build fact reported honestly, not a runtime probe**, and the reason is
worth writing down: a module compiled with `simd128` will not instantiate on a
runtime without it, so by the time our Rust is running the answer is always
yes. There is no version of this that a Rust function can usefully ask.

The real runtime probe belongs in the shell, before the artefact is fetched,
and this story adds it: `packages/core/src/capabilities.ts` exports
`wasmSimd128Supported()`, which runs `WebAssembly.validate` over a committed
minimal module containing one SIMD instruction. The bytes come from `wat2wasm`
over a two-line `.wat` reproduced in a comment beside them, never hand-typed.

`SimdSupport` is carried in `TierEvidence` and **not** in `Caps`. Section 22's
four fields are the four fields.

### 8. Threads, and the field that deliberately does not exist

`packages/core/src/capabilities.ts` also reports `sharedMemoryAvailable()`,
which is `typeof SharedArrayBuffer !== "undefined" && globalThis.crossOriginIsolated === true`.
It is a diagnostic for the support surface and for F-006's harness, and it is
the last time threads appear in this story.

**Nothing on the Rust side has a threads field.** Not `Caps`, not
`TierSignals`, not `TierEvidence`. D5 fixes the answer, D-07 restates it for a
second reason, and the strongest way to hold a decision is to leave nowhere to
branch on it. A field would be a place for a future story to write
`if signals.threads { ... }` and mean it. There is no such field, so there is
no such line, and that is a stronger guarantee than a comment.

### 9. Prove each new check goes red

Following the sprint's standing expectation, and in a separate command from the
one that adds them:

- Empty the renderer-string list and confirm the SwiftShader row goes red.
- Invert the benchmark comparison and confirm the hardware row and the software
  row swap and both go red.
- Set the hardware floor equal to the software ceiling and confirm the
  `Unknown` band test goes red.
- Corrupt one byte of the committed SIMD module and confirm
  `wasmSimd128Supported()` goes false, which is what stops the constant from
  being a valid non-SIMD module that validates for the wrong reason.

## Boundary and tier

- wasm-bindgen: **not touched.** `ocelli-render` gains no direct declaration
  and no `wasm_bindgen` in source. The unmasked renderer string arrives through
  wgpu's own GLES backend, so this story needs no browser binding of its own,
  and D-12's direct-declaration rule for wasm32 is satisfied by construction.
  Subject to open question 1, which is about `ocelli-wasm` rather than about
  this crate.
- Pixels across the boundary: **no.** The benchmark shades an offscreen texture
  and never maps it. Nothing is read back and nothing crosses.
- Render-loop allocation: **none.** Resolution runs once at startup, before any
  render loop exists. The workload's texture, pipeline and encoder are created
  and dropped inside `resolve`, which is the definition of not being in the
  loop.
- unsafe: **none.**
- **Tier A (WebGPU): full.** Resolved when an A-candidate adapter exists and
  the evidence says hardware. `Caps.compute` is true and section 7's compute
  path is open.
- **Tier B (WebGL2): full, and narrowed on purpose.** Resolved when the best
  candidate is fragment-only and the evidence says hardware. It is **not**
  resolved merely because a conforming WebGL2 context exists, which is the
  entire content of D-07's first consequence. Reaching a `Backend::Gl` adapter
  on wasm32 needs wgpu's `webgl` feature, which is open question 2.
- **Tier C (CPU): full.** Resolved when there is no adapter, when no device
  could be created, or when the evidence says the candidate is a software
  rasteriser. It is a first-class outcome of this resolver and not an error
  path, per D-07 and A7.1b. This story resolves the tier. Rendering on it is
  F-X001 to F-X004.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | `classify` over a table of signal combinations: each A7 renderer string, a `DeviceType::Cpu` adapter, a `Gl` adapter with a clean string, no adapter at all, a device that could not be created, and the benchmark agreeing with, disagreeing with, and absent from each hint | `crates/ocelli-render/src/caps.rs` under `#[cfg(test)]` |
| `unit` | Every override outcome: applied, refused as unconstructible, refused as unparseable, `auto`, and the `Cpu` short-circuit reaching no adapter code at all | `crates/ocelli-render/src/caps.rs` under `#[cfg(test)]` |
| `unit` | The fill-rate comparison is integer-only, correct at both band edges by hand-computed values, and does not overflow on a `u64::MAX` elapsed time | `crates/ocelli-render/src/caps.rs` under `#[cfg(test)]` |
| `unit` | `Caps` for `Tier::Cpu` is `compute: false, max_tex_3d: 0, max_buffer: 0`, and for a GPU tier carries the adapter's reported limits unclamped | `crates/ocelli-render/src/caps.rs` under `#[cfg(test)]` |
| `unit` | `wasmSimd128Supported()` is true on the committed module and false on a corrupted copy, and `sharedMemoryAvailable()` reads both conditions | `packages/core/src/capabilities.test.ts` |
| `property` | `classify` is total: over generated signal combinations it always yields exactly one tier, never panics, and never yields a tier the evidence says is unconstructible | `crates/ocelli-render/tests/` |
| `browser` | On a real GPU-less session, resolution returns `Cpu` and not `B`. **Named, not built here.** This is A7.2 layer 3 and it is F-X002 in S14 | `tools/oracle/`, later |
| `conformance` | Only `ocelli-render` names a device-creating wgpu call, still, with the probe added | `ci/check-device-ownership.sh`, gate `device` |

**No `fixture` row, and that is a statement rather than an omission.** This
story computes no pixel value and no coordinate, so HLD 27.2 R3 does not apply
and there is no DICOM section a hand-computed value could cite. The arithmetic
it does contain, the integer fill-rate comparison, gets hand-computed band-edge
values in its `unit` row instead, which is the same discipline against the only
specification it has, which is this plan.

The GPU-touching half of `probe.rs` has no row above, deliberately. A7.2 is
explicit that no software adapter is an oracle and that layer 2 is F-X002's
acceptance question rather than something to assume today. What this story can
prove without a GPU is the decision, and the decision is where the defect is.

## Parity surface covered

**None.** `docs/hld/B-parity-surface.md` is a table of surface counts and
carries no `Covered by` column in this repository, and no row in it is keyed on
E1.4. The backlog's `Epic ref` column is the mapping the `/design` step 6
refers to, and E1.4 appears there only as this story's own row. Tiering is
infrastructure under every parity row rather than one of them.

## Deviations

**None new.** Three existing rows are load-bearing here and this story stays
inside all three:

- **D-07** is the authority for `Tier::Cpu` being a resolvable outcome and for
  the software-adapter requirement. This story implements its first
  consequence and does not widen it.
- **D-12** is why `ocelli-render` may link wgpu on wasm32 without breaking D2.
  This story adds no direct wasm-bindgen declaration and no `wasm_bindgen` in
  source, so it stays on the safe side of that row.
- **D-10** is why wgpu is present in `ocelli-render` two sprints before F-039.
  Unchanged.

Two candidate rows are described in open questions 2 and 3 and are not written
here, because this plan may not edit `docs/hld/DEVIATIONS.md`.

## LLD impact

- **`docs/lld/gpu-ownership.md`**, updated. Its "What this story deliberately
  does not do" section says F-008 does not detect `Caps` and that filling it
  from an adapter is F-004. That sentence becomes describing the past, and its
  Tiers table gains the resolution rule.
- **`docs/lld/tier-resolution.md`**, new. The three signals, the combination
  rule written as the procedure above, the threshold file and its provenance,
  the override and its refusal outcomes, the probe device's transience and why
  it does not violate section 31, and the deliberate absence of a threads
  field.
- **`docs/lld/build-targets.md`**, updated only if open question 2 lands the
  `webgl` feature, in which case the exact-pins table and the cross-target
  proof both need the note.

## Open questions

1. **Does F-004 wire tier resolution into `ocelli-wasm`?** Doing so makes the
   wasm module reach wgpu and Naga for the first time and re-baselines
   `ci/wasm-size-budget.json` from 14,104 bytes to something Appendix A gate A4
   estimates at 3 to 8 MB. `docs/lld/build-targets.md` says the story that
   makes the module reach wgpu is the one that re-baselines the budget and says
   why. **Recommendation: no.** F-039 (E6.1, S11) is "device init, capability
   tiering, device-lost recovery" and depends on this story, and F-X002 (S14)
   builds the headless-Chrome layer that would exercise the browser path. This
   story delivers the resolver, proved natively.
   _Blocks_: whether the `wasm` gate re-baselines in S03, and whether any
   browser path of the resolver runs before S11.
2. **Does `ocelli-render` add wgpu's `webgl` feature?** `webgl` is not in
   wgpu 30.0.1's default feature set, so **there is no `Backend::Gl` adapter on
   wasm32 today and tier B is unreachable in a browser.** The change is
   `wgpu = { workspace = true, features = ["webgl"] }` in
   `crates/ocelli-render/Cargo.toml`, which leaves section 15.2's workspace
   line character for character as written. **Recommendation: yes**, because a
   tier the resolver can never return is not a tier.
   _If yes, a deviation row is probably wanted_, in the shape D-09 set for
   glam: "§15.2 gives `wgpu = "=30.0.1"` / `ocelli-render` adds the `webgl`
   feature at the consuming crate / §7 declares tier B as WebGL2 through wgpu's
   downlevel profile, and wgpu 30.0.1 does not enable its `webgl` backend by
   default, so without this feature tier B is unreachable on wasm32 and §7's
   second tier does not exist. The pin and the workspace entry are untouched
   and only the consuming crate's feature set changes. / F-004". Operator to
   decide whether that warrants a row or only a manifest comment.
   _Blocks_: whether tier B can ever resolve on the wasm target, and whether
   `ci/target-feature-baseline.json` needs a declared entry.
3. **Does `ocelli-native` gain a dependency on `ocelli-render`** so the two
   entry points read `OCELLI_TIER` and print the resolved tier and evidence?
   **Recommendation: yes.** "An operator override exists" is a done criterion,
   and without a caller the override is a parameter nobody passes, which
   `AGENTS.md` calls a feature flag without a named user. The cost is that
   `bin/ocelli.sh native` step 1 starts linking wgpu, which is a real build-time
   change to a gate in the floor.
   _Blocks_: whether the override has a named user in S03, and the floor gate's
   runtime.
4. **What machines are available to calibrate the two thresholds?** The
   hardware floor and the software ceiling in `ci/tier-thresholds.json` must be
   measured, not invented, and A7.3 is explicit about that. A hardware figure
   is available on this machine. A software figure needs either headless Chrome
   on SwiftShader or lavapipe on a Linux runner, and A7.2 records that whether
   lavapipe even resolves as a fallback adapter under the pinned wgpu is itself
   unproven and is F-X002's acceptance criterion.
   _Blocks_: whether the benchmark can be the deciding signal at all in S03, or
   whether it ships measured-and-recorded with the two hints deciding until the
   bands are calibrated. If it is the latter, that must be said out loud,
   because a benchmark that is recorded and not consulted looks identical to
   one that is trusted.
5. **Does the workspace gain `pollster` as a dev-dependency?** `resolve` is
   `async fn` because `request_adapter` and `request_device` are futures in
   wgpu 30.0.1, and a native `#[test]` needs something to drive it.
   **Recommendation: yes, dev-dependencies only**, which keeps it out of what
   ships and therefore out of section 15.2's list, the same argument that
   already covers `proptest` and `trybuild`.
   _Blocks_: the `property` test row and any native test of `probe.rs`.
6. **`Gallium` in A7's string list is a false positive on real hardware.**
   Mesa's Gallium framework backs the `radeonsi` and `iris` drivers on genuine
   AMD and Intel GPUs, and those renderer strings have historically read
   "Gallium 0.4 on AMD ...". Matching it classifies real hardware as software
   and drops a working GPU session to tier C. The plan's combination rule
   already contains this, because a string is only consulted when the benchmark
   did not decide, but that is the plan narrowing a signal a **resolved** spike
   lists. **Proposed: keep the list verbatim and keep the narrowing, and record
   the reason in `docs/lld/tier-resolution.md`.** Operator to confirm, since
   A7 is closed and this is a change to how one of its answers is used.
   _Blocks_: nothing in code, but it decides whether the combination rule as
   written needs A7 amended alongside it.
7. **Is a refused override an error or an outcome?** This plan makes `resolve`
   infallible and records refusals in `TierEvidence`, so it introduces no error
   type and does not collide with F-005. Confirm that is the wanted shape.
   _Blocks_: whether F-005's error model has to represent a tier refusal.

## Interaction with the rest of S03

Recorded here rather than in `docs/sprints/`, which this plan may not edit.

- **F-005, error model.** This plan deliberately introduces **no error type**.
  Every failure of detection is a recorded outcome and a resolved tier, which
  is D-07's rule applied to the resolver itself. The one thing F-005 should
  know is that `TierEvidence` will want to be serialisable into whatever
  structured-log shape F-005 defines, because "the viewer is slow on that
  estate" is diagnosed from exactly this record. Ordering is free either way.
- **F-006, benchmark harness.** The nearest thing to a collision in the sprint,
  and worth naming before both stories build one. **They are different
  instruments.** F-004's probe is a startup classifier, tens of milliseconds,
  no corpus, living inside `ocelli-render` and never exported as a harness.
  F-006 measures decode, first frame and interaction latency and records the
  numbers, and A7.1b requires it to measure tier C from the start. F-006 should
  **consume** `resolve` so its numbers carry the tier they were taken on, and
  neither story should acquire the other's job. Both want a recorded-number
  file with a tolerance, in the `ci/wasm-size-budget.json` shape. Whether those
  are one file or two is a decision worth making on purpose rather than by
  whichever lands first.
- **F-011, pixel-diff comparator.** D-07 requires the divergence bound to cover
  tier A against tier C, and A7 makes the oracle diff them. That is F-X003's
  scope, not S03's, but F-011's per-row verdict record should carry the tier
  the frame was rendered on from the start, because retrofitting a tier column
  into a recorded verdict format later is the kind of change that invalidates
  every stored result. One field, cheap now.
- **F-X006, codec spike gates.** No interaction.
- **F-X007, oracle volume and MPR reference renders.** No direct interaction.
  Both touch section 7 indirectly, since volume rendering runs on both GPU
  tiers, but F-X007 is reference-side and runs cornerstone3D.
- **F-X009, standing guard tests.** Direct and worth sequencing. If open
  question 2 lands the `webgl` feature, and if this story adds any check of its
  own, F-X009's sweep must include them or this story ships a guard with
  nothing watching it, which is precisely the defect S02 measured. F-X009
  should run after F-004 lands, or F-004 should hand it a list.

---

## Decisions taken in the design round

Answers to `## Open questions`, taken in the S03 consolidated round. The plan
above is unchanged and these govern where they differ.

**1. Wire tier resolution into `ocelli-wasm`? No.** `ocelli-wasm` has an empty
`[dependencies]` table and exports one function, the boundary is E16.2 in S16,
and this plan already records that no browser path of the resolver can run
before F-039 in S11. Wiring it now would add an entry point nothing calls and
re-baseline the wasm size budget for a feature with no user. **F-004 therefore
does not touch `ci/wasm-size-budget.json` at all**, which removes the collision
with F-005, the one story that legitimately grows that module this sprint.

**2. Add wgpu's `webgl` feature? Yes, and deviation D-14 is applied.**
Confirmed against the pinned crate's own manifest: wgpu 30.0.1's defaults are
`std, parking_lot, dx12, metal, gles, vulkan, wgsl, webgpu`, and `webgl` is a
real feature at line 117 that is not among them. Without it tier B cannot
resolve in a browser at all, which would leave an HLD section 7 tier
unreachable. Measured cost today is zero bytes, because nothing in the wasm
artefact reaches wgpu, so the decision is far cheaper now than after S11.

**3. Does `ocelli-native` depend on `ocelli-render` so the entry points read
`OCELLI_TIER`? Yes.** `AGENTS.md` forbids a feature flag with no named user, and
an operator override nothing can exercise is exactly that. The `native` gate
getting slower because it starts linking wgpu is a real cost and is accepted,
because the alternative is shipping an override with no way to observe it.

**4. Threshold calibration.** Record what this machine can actually measure into
`ci/tier-thresholds.json`, as a measurement with a tolerance in the shape
`ci/wasm-size-budget.json` uses, and **say in the file which figures are
measured and which are not**. Where no software-adapter figure can be taken,
the benchmark returns `Unknown` and the combination rule falls through to
adapter type and then renderer string, which is the behaviour this plan already
specifies. A recorded-but-not-consulted number must not read as a trusted one,
so the file states its own provenance per figure. A7.2's lavapipe question stays
F-X002's.

**5. `pollster` as a dev-dependency? Yes, dev-only.** It follows the existing
`proptest` and `trybuild` precedent in `[workspace.dependencies]`, which carries
a comment saying the entry is dev tooling outside section 15.2's scope and no
deviation row. It never enters the shipped surface.

**6. `Gallium` in A7's renderer-string list.** Keep this plan's narrowing, since
Mesa's Gallium backs radeonsi and iris on genuine hardware and treating the
string as decisive would misclassify real GPUs. **Do not amend
`docs/spikes/A7-tier-c.md` in this story.** Record it as a follow-up so the
amendment is a reviewed change to a resolved spike rather than a side effect of
an implementation.

**7. Is a refused operator override an error? No.** It is an outcome of
resolution, recorded in `TierEvidence`. Giving it a code would put a successful
resolution into the error space. F-005 agrees and `ErrorCode::Unavailable` keeps
its distinct meaning, which is a feature saying it cannot run on the resolved
tier.
