# F-037, ocelli-render: device init, capability tiering, device-lost recovery

**Status**: approved
**Epic ref**: E6.1
**Sprint**: S11
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/05-rendering.md`, section 7, in full

|  |
|----|
| **Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup. |
| **Volume rendering runs on both tiers**, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute — GPU segmentation, histogram passes, compute-based resampling — is tier A only and must degrade, not fail. |
| **Bricking above 256 MiB.** A 512×512×600 sixteen-bit CT series is roughly 300 MB against a guaranteed maximum buffer size of 256 MiB, so chunked upload is the normal path, not an optimisation. |
| **One submit per frame.** The render graph tracks dirty viewports and issues a single submission across all of them, driven by requestAnimationFrame inside the render worker. |
| **Blend modes are shader variants** — composite, MIP, MinIP, average — selected by specialisation constant rather than branching per fragment. |

### `docs/hld/19-render-graph.md`, section 22, the `Caps` struct and the device-loss bullet

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
| **Device loss is a real state, not an error path.** Handle device_lost, rebuild the device and all resources, and restore viewport state from the shell's copy — the same recovery path §23 needs for panics. |

### `docs/hld/26-differentiating-capabilities.md`, section 31, the device-sharing bullet

|  |
|----|
| **Shares the renderer's device.** ocelli-compute never creates a wgpu::Device; it borrows the one ocelli-render owns. Two devices cannot share textures, which would defeat the entire point. |
| **Every tier-A kernel declares a fallback** — CPU, or a worker — so a feature degrades rather than fails on WebGL2. A kernel with no fallback marks its feature unavailable; it never silently produces a different answer. |

### `docs/hld/20-errors-and-panics.md`, section 23, the recovery clause section 22 points at

|  |
|----|
| **A PANIC POISONS THE INSTANCE** Once a Rust panic aborts inside WebAssembly, that module instance's memory may be inconsistent and it must not be reused. So: panic = "abort" in release; no exported function may let a panic escape as a normal error path; and on panic the worker instance is torn down and rebuilt, with the JavaScript shell reconstructing viewport state from its own copy. That last clause is a design constraint on the shell, not an afterthought — the shell must always hold enough state to rebuild a viewport from nothing. |

|  |
|----|
| A poisoned instance surfaces to the user as a viewport-level error state, never as a silent blank canvas. |

### The pinned wgpu's own API, read from `wgpu-30.0.1` rather than from memory

`wgpu = "=30.0.1"`, HLD section 15.2, with the `webgl` feature under D-14.
Read from the vendored crate source at
`~/.cargo/registry/src/index.crates.io-*/wgpu-30.0.1/`:

- `src/api/device.rs:681`
  `pub fn set_device_lost_callback(&self, callback: impl Fn(DeviceLostReason, String) + Send + 'static)`
- `src/api/device.rs:675` `pub fn destroy(&self)`, documented as "Destroy this
  device."
- `wgpu-types-30.0.1/src/lib.rs:554` `pub enum DeviceLostReason { Unknown = 0, Destroyed = 1 }`,
  with `Unknown` documented as "The device was lost for an unspecific reason,
  including driver errors" and `Destroyed` as "The device's `destroy` method was
  called."
- `src/backend/webgpu.rs:2622` shows the browser backend mapping
  `GpuDeviceLostReason::Destroyed` and `::Unknown` onto those two variants, so
  the same two arms are what a browser produces.

**There is no third variant and no `Ok`/`Err` on the callback.** A recovery
policy in this project is therefore a function of exactly two reasons, which is
why it can be a total match rather than a catch-all.

### What section 22 does NOT contain

It says "rebuild the device and all resources, and restore viewport state from
the shell's copy" and stops. It does not say which losses are recoverable, how
many attempts to make, who observes the loss, or what happens to in-flight work.
**There are no resources and no viewports in this workspace yet.** F-038 is the
render graph, F-039 is OffscreenCanvas, F-040 is the texture upload path, and
all three are S12 or S13. This story can rebuild a device and a queue and
nothing else, because nothing else exists to rebuild.

## What the specification does not cover

1. **Which loss reasons are recovered from.** `Destroyed` is a deliberate
   teardown by the application itself. Re-creating a device the caller just
   destroyed would be a recovery nobody asked for and an infinite loop if the
   caller destroys on shutdown. `Unknown` covers driver reset, a tab backgrounded
   too long, and OOM, which are section 22's actual population. Decided in
   Approach section 3.
2. **Who observes the loss.** wgpu's callback is `Fn(DeviceLostReason, String)
   + Send + 'static`, so it cannot borrow the `GpuContext` it belongs to. The
   observable state has to live behind a shared handle the callback also holds.
3. **Whether recovery is automatic or requested.** This plan makes it requested:
   the loss is recorded, and `GpuContext::recover` is a call the owner makes.
   Automatic re-creation from inside a `Fn` callback would create a device on
   an arbitrary thread at an arbitrary moment, which is the opposite of a single
   long-lived owner.
4. **What `resolve` does with the adapter after it classifies.** Today
   `probe::resolve` opens a transient device, measures on it and drops both the
   device and the adapter, returning only `Resolution { caps, evidence }`.
   Rebuilding needs an adapter, so something has to retain one.
5. **How a GPU-requiring test is run at all.** Deviation D-04 leaves CI without
   an adapter, and `bin/ocelli.sh gate --list` shows `test` as
   `cargo test --workspace` with `needs_gpu = no`. Every device test this story
   writes is therefore invisible to every profile unless a gate is added. The
   S11 design round added one, and Approach section 7 is the change.
6. **Whether `ocelli-render` is wired into `ocelli-wasm` this sprint.** It is
   not today: `crates/ocelli-wasm/Cargo.toml` names `ocelli-core` and nothing
   else. The S11 design round decided not to wire it, and Approach section 8 is
   what that costs and what it corrects.

## Approach

### 1. Retain the adapter, which is the smallest change to `probe::resolve`

`resolve` currently builds an instance, enumerates adapters, opens a transient
probe device on the first candidate that opens, measures a fill rate, and drops
everything. The module header is explicit that the probe device "is created,
measured on and dropped before `resolve` returns", and that this is what keeps
section 31's one-device invariant true.

**That stays true.** This story adds a second, separate entry point that opens
the long-lived device, and it opens it **after** the probe device is dropped, so
there is still never a moment when two devices exist.

```rust
/// The adapter the session resolved on, retained so a lost device can be
/// rebuilt without re-running detection.
pub struct ResolvedAdapter {
    adapter: wgpu::Adapter,
    caps: Caps,
    evidence: TierEvidence,
}

/// Resolve the tier AND keep the adapter that won.
///
/// `resolve` remains as it is, because `F-004`'s callers want a verdict and no
/// device. This is the entry point that wants one.
pub async fn resolve_adapter(
    request: TierRequest,
    clock: &mut dyn FnMut() -> u64,
) -> Option<ResolvedAdapter>;
```

`None` when the resolution is tier C, because tier C has no adapter by
construction, and when no adapter would open a device at all. Both are already
`ProbeOutcome` states that `classify` turns into `Tier::Cpu`, so this adds no
new decision, it reads one `caps.tier` already carries.

### 2. The long-lived device, and `GpuContext` gains the loss state

`GpuContext::new(device, queue, caps)` already exists from F-008 and its doc
comment already names this story: "Adapter enumeration, tier resolution and
device-loss recovery are F-004 and F-037". This story adds the constructor that
opens a device from a `ResolvedAdapter`, and the loss state.

```rust
impl ResolvedAdapter {
    /// Open the session's one long-lived device.
    ///
    /// The ONLY `request_device` in this workspace other than the probe's, and
    /// both are in `ocelli-render`, which `ci/check-device-ownership.sh`
    /// asserts.
    pub async fn open(&self) -> Result<GpuContext, DeviceError>;
}

/// What the loss callback records, shared with it because the callback is
/// `Fn(..) + Send + 'static` and cannot borrow the context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceLoss {
    pub reason: wgpu::DeviceLostReason,
    /// wgpu's diagnostic text. Explicitly unstable, for humans, never matched on.
    pub message: String,
}

/// The device's state, which section 22 says is a real state and not an error path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceState {
    Live,
    Lost(DeviceLoss),
}

impl GpuContext {
    /// `Live`, or the loss that was recorded. Observable, not inferred.
    pub fn state(&self) -> DeviceState;

    /// Rebuild the device and the queue on the same adapter.
    ///
    /// Returns the entries the caller must rebuild. Today that list is empty
    /// because no resource type exists: F-038, F-039 and F-040 are what put
    /// things in it, and the return type exists now so they do not have to
    /// change this signature.
    pub async fn recover(&mut self, adapter: &ResolvedAdapter) -> Result<Recovered, DeviceError>;
}
```

The callback is registered inside `open`, immediately after `request_device`
returns and before the `GpuContext` is built, so there is no window in which the
device is live and unobserved:

```rust
let lost = Arc::new(Mutex::new(None::<DeviceLoss>));
let sink = Arc::clone(&lost);
device.set_device_lost_callback(move |reason, message| {
    // First loss wins. A second callback after a rebuild belongs to the OLD
    // device, and overwriting would attribute it to the new one.
    let mut slot = sink.lock().unwrap_or_else(PoisonError::into_inner);
    if slot.is_none() {
        *slot = Some(DeviceLoss { reason, message });
    }
});
```

`Arc<Mutex<..>>` and not `unsafe`. `ocelli-render` is not `no_std` under D-10
and `probe.rs` already uses `Arc` and an atomic for the same class of problem.

### 3. The recovery policy, which is the decision and is testable without a GPU

```rust
/// Whether a loss of this kind is recovered from.
///
/// `Unknown` is section 22's population: driver reset, a tab backgrounded too
/// long, an OOM. Those are recovered from.
///
/// `Destroyed` is `Device::destroy` having been called, which is the
/// application's own deliberate teardown. Re-creating a device the caller just
/// destroyed is a recovery nobody asked for, and on a shutdown path it is a
/// loop.
pub const fn recovers_from(reason: wgpu::DeviceLostReason) -> bool {
    match reason {
        wgpu::DeviceLostReason::Unknown => true,
        wgpu::DeviceLostReason::Destroyed => false,
    }
}
```

**A total match over both variants, never a wildcard.** If a future wgpu adds a
third reason this stops compiling, which is the outcome this project wants: the
alternative is a new reason silently inheriting one of the two answers.

`recovers_from` lives in `caps.rs` and not in `gpu.rs`, following the split
`lib.rs` already declares: "everything that can be WRONG about a tier is in
`caps`, which needs no adapter to test". The same argument that moved
`compute_available` there applies unchanged, and it means the one decision in
this story is reachable by `cargo test --workspace` on a machine with no GPU.

`recover` consults it and refuses a non-recoverable loss with
`DeviceError::NotRecoverable`, rather than rebuilding anyway.

### 4. What recovery does NOT do, and this is the honest scope

Section 22 says "rebuild the device and all resources, and restore viewport
state from the shell's copy". Two of those three are not this story:

- **Resources.** There is no texture, buffer, pipeline or bind group type in
  this workspace. `Recovered` is returned so F-038 and F-040 extend it rather
  than change this signature, and today it carries the new `Caps` and nothing
  else. A `Caps` **can** change across a rebuild if the adapter opens with
  different limits, and a caller that assumed otherwise would size a texture
  against a stale number.
- **Viewport state from the shell's copy.** `ocelli-viewport` is a scaffold and
  the boundary carries no viewport command. F-039 and E7 are where that lands.

Both absences are stated in `docs/lld/gpu-ownership.md` by this story, so a
later reader sees a declared boundary rather than an oversight.

### 5. `supports_compute`'s unreachable forwarder, closed

`GpuContext::supports_compute` carries a long doc comment ending "What closes it
is a `GpuContext` a test can build, which is F-037's long-lived device and
F-X002's software-adapter path, and both are named on `new` above as the reason
that constructor is public." This story builds one, so the GPU test asserts
`supports_compute` against `compute_available(caps)` on a real context and that
paragraph is rewritten to say what is now true rather than what was owed.

### 6. The `edge`/`passes` transposition `measure_with` names

`probe::measure_with`'s doc comment records a residue: `run(device, queue,
&pipeline, edge, passes, clock)` takes two adjacent `u32`s, transposing them
compiles, and no test reaches that call site. It names the fix as "a type that
makes the two arguments non-interchangeable, which is `AGENTS.md`'s 'reducing
cases is good even when it adds types' and is **F-037's to argue** when it builds
the long-lived device."

This story argues it and takes it: `run` takes `Edge(u32)` and `Passes(u32)`
newtypes, `RUN_PLAN` becomes `[(Edge, Passes); 3]`, and the transposition stops
compiling. That is one case removed rather than one place added, and it is the
shape `Pt<Canvas>` against `Pt<World>` already sets in this codebase.

### 7. A new `gpu` gate, so the device tests run in a profile rather than by hand

This is the S11 design round's answer and F-041 depends on it more than this
story does. Four files change together:

```text
bin/ocelli.sh          GATES gains  "gpu|YES|the LUT shader and the device
                       lifecycle against a real adapter (E6.1, E6.5, HLD 7)"
                       run_gate gains the arm
                       the --floor exclusion gains `gpu`
scripts/ci_floor_check.py   NOT_IN_FLOOR gains `gpu`
.github/workflows/ci.yml    unchanged, because the floor is what it runs
.claude/WORKFLOW.md    "The gate" section names the second GPU gate
```

```bash
gpu)  cargo test -p ocelli-render --test device --test voi_shader -- --ignored ;;
```

**This is an addition, not a loosening.** `AGENTS.md` refuses disabling a gate
to get a build green, and the shape being added is the one `oracle` already
sets: a `YES` in the GPU column, excluded from `--floor` under deviation D-04,
run by `gate --sprint` and `gate --all`. `scripts/ci_floor_check.py` compares
`bin/ocelli.sh`'s exclusion list against its own `NOT_IN_FLOOR` for set
equality, so the two cannot drift, and the `ci` gate asserts the floor is what
CI invokes.

`--test device --test voi_shader` is deliberately a named pair rather than
`cargo test -p ocelli-render -- --ignored`, which would also pick up
`measures_a_fill_rate_on_this_machine`. That test is a print-only instrument
that asserts nothing and always passes, and a gate containing a test that cannot
fail reads as covering more than it does.

**Per `.claude/WORKFLOW.md`'s closing section, the workflow change is committed
separately from the feature change**, with the trigger noted in the AS_BUILT
entry, and `python3 scripts/sync_agent_skills.py` is run and then `--check`.

### 8. `ocelli-render` is NOT wired into `ocelli-wasm`, and the note that says it will be is corrected

`CURRENT_SPRINT.md` says "The size budget starts moving this sprint, and that is
expected", and `ci/wasm-size-budget.json`'s `bearing_on_gate_A4` field says "The
number that will bear on A4 arrives with the render path from S11, not here."

**Neither is true as this sprint is planned.** `crates/ocelli-wasm/Cargo.toml`
names `ocelli-core` and nothing else, and none of F-031, F-037 or F-041 changes
that. The S11 design round decided not to add the edge: nothing in the shell
consumes a device or a shader yet, and a dependency added only to move a number
is a dependency with no user. It is the same answer F-004 was given in the S03
round, recorded in that same file as `only_rebaseline_in_S03`.

So this story takes **no** re-baseline, runs no `--accept-size`, and instead
corrects the stale forecast:

- `ci/wasm-size-budget.json`'s `bearing_on_gate_A4` stops naming S11 and names
  F-039, E6.3 in S13, which is the story that puts a canvas in a worker and is
  therefore the first one with a reason to reach `ocelli-render` from
  `ocelli-wasm`.
- `docs/sprints/CURRENT_SPRINT.md`'s size-budget section records that the
  expectation was not triggered and why, under `## Carried forward from S11` if
  the sprint carries anything, otherwise inline.

The 16,388-byte figure is therefore expected to be **unchanged** at the close of
S11, and a `wasm` gate that reports a change is a finding rather than a
re-baseline.

## Boundary and tier

- wasm-bindgen: not touched. `ocelli-render` must not grow a browser binding,
  which is deviation D-12, and the clock stays the caller's for the reason
  `probe::resolve` already gives
- Pixels across the boundary: no. Nothing is read back, nothing is mapped, and
  no pixel leaves the device
- Render-loop allocation: none. Device creation and recovery are startup and
  recovery paths, not frame paths. `state()` clones a small record behind a
  mutex and is not called per frame
- unsafe: none
- Tier A (WebGPU): **full.** This is the tier the long-lived device is opened
  for and the one `Caps::compute` is true on
- Tier B (WebGL2): **full.** The device is opened with the adapter's OWN limits,
  never `Limits::default()`, which is the rule `probe.rs` already states and the
  whole reason tier B can open a device at all. Device loss and recovery are
  backend-agnostic in wgpu 30.0.1: `set_device_lost_callback` is on `Device` and
  both the `wgpu_core` and `webgpu` backends implement it. **Tier B has never
  been exercised**, which `CURRENT_SPRINT.md` names as this sprint's exposure,
  so the GPU test asserts a device opens and reports its tier rather than
  asserting which tier the machine has
- Tier C (CPU): **unavailable, deliberately and reportably.** A tier C session
  has no adapter, so `resolve_adapter` returns `None` and there is no device to
  lose. That is HLD section 31's rule under D-07: the feature reports
  unavailable rather than producing a different answer. `TierRequest` of
  `Tier::Cpu` already short-circuits before any instance is created, and this
  story does not weaken that

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | `recovers_from(Unknown)` is true and `recovers_from(Destroyed)` is false, both arms named. **Needs no GPU** | `crates/ocelli-render/src/caps.rs` |
| `unit` | `recovers_from` is a total match: adding a variant is a compile error, asserted by the absence of a wildcard arm and checked by the human review item rather than by a test | same |
| `unit` | `Edge` and `Passes` are not interchangeable, asserted as a compile error | `crates/ocelli-render/tests/ui/` via trybuild |
| `unit` | `RUN_PLAN` is unchanged in value after the newtype change: still the calibration twice and the full pass, still 65,536, 65,536 and 16,777,216 fragments | `crates/ocelli-render/src/probe.rs` |
| `unit` | A tier C `TierRequest` yields no `ResolvedAdapter`, driven through the existing `classify` path with `TierSignals::unprobed`, so no adapter is needed | `crates/ocelli-render/src/probe.rs` |
| `browser` (device) | A device opens on this machine, `state()` is `Live`, and `caps()` matches what `resolve` classified. **Ignored, needs a real adapter** | `crates/ocelli-render/tests/device.rs` |
| `browser` (device) | `supports_compute()` on a real context equals `compute_available(caps)`, which closes the forwarder nothing reached | same |
| `browser` (device) | **Device loss is observed.** `device().destroy()`, then poll, then `state()` is `Lost(DeviceLoss { reason: Destroyed, .. })`. This is the only deterministic way to make the callback fire, per the pinned wgpu's own documentation | same |
| `browser` (device) | **A non-recoverable loss is refused rather than rebuilt.** `recover` after a `Destroyed` loss returns `DeviceError::NotRecoverable` and the context is still `Lost` | same |
| `browser` (device) | **Recovery rebuilds.** A loss record injected as `Unknown` through the same shared slot the callback writes, then `recover`, yields a `Live` context whose queue accepts a submission. The injection is what makes the recoverable arm reachable at all, because nothing in the pinned wgpu can produce an `Unknown` loss on demand | same |
| `browser` (device) | A second callback firing after a rebuild does not overwrite the new device's state, asserted by the first-loss-wins rule | same |

**On the `browser` (device) label.** `.claude/WORKFLOW.md`'s test taxonomy has
six categories and none of them is "native test needing a real adapter". The
nearest is `browser`, "the boundary, workers and tiers under a real engine", and
these tests are about tiers and a real engine while being native rather than
under a browser. The label is qualified rather than invented, and the gap is
recorded here so a later story can decide whether the taxonomy gains a row
rather than each plan inventing its own word.

**On the injected `Unknown`.** It is stated here rather than hidden, because a
test that fabricates its own input is weaker than one that does not. The two
halves it separates are both real: the callback path is proved end to end by the
`Destroyed` case on a real device, and the rebuild path is proved by the
injection. What is NOT proved by either is that a real driver reset produces
`Unknown` and reaches the same slot, and no test on one machine can prove that.
It is recorded in `docs/lld/gpu-ownership.md` as a known limit rather than
claimed as covered.

**Mutation check, HLD 27.3.**

1. Flip `recovers_from(Destroyed)` to `true`. The refusal test goes red.
2. Register the loss callback after building the `GpuContext` rather than before.
   Add a `destroy` immediately after `open` and the observation test goes red.
3. Let the callback overwrite an existing loss. The first-loss-wins test goes
   red.
4. Open the device with `Limits::default()` instead of the adapter's own. A tier
   B adapter fails to open, which is the defect `probe.rs` already names.
5. Transpose `edge` and `passes` at `run`'s call site. This now fails to
   compile, which is the point of the newtypes and is the mutation that was
   green before this story.

## Parity surface covered

None directly. `docs/hld/B-parity-surface.md` counts viewport types, tool
classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation
representations, events and adapters. A device is underneath all of them and is
none of them. The appendix has no `Covered by` column in this repository.

## Deviations

**No new deviation.** D-07 (tier C), D-10 (wgpu activated early) and D-14 (the
`webgl` feature) are all already declared and this story is a consumer of each
rather than an extension of any.

D-04 is the reason the device tests are ignored by default, and it is declared.
The new `gpu` gate of Approach section 7 is **not** a deviation: it is an
addition to `bin/ocelli.sh`, to `scripts/ci_floor_check.py`'s `NOT_IN_FLOOR` and
to `.claude/WORKFLOW.md`, committed separately from the feature change, and it
extends D-04's existing arrangement rather than departing from the HLD. D-04's
own row already records that CI runs no GPU gate, and adding a second gate to
the set CI does not run leaves that row true.

## LLD impact

- `docs/lld/gpu-ownership.md`, which says "Nor does this type create anything.
  `new` takes a device and a queue that already exist" and names this story as
  the one that changes it. Gains the device state, the recovery policy and the
  two things recovery does not do
- `docs/lld/tier-resolution.md`, which gains `resolve_adapter` beside `resolve`
  and the `Edge`/`Passes` newtypes in the recorded workload
- `docs/lld/feature-availability.md`, which gains the tier C row for a device
- `docs/lld/guards.md`, which gains the `gpu` gate beside `oracle` in whatever
  census it keeps of what runs where
- `.claude/WORKFLOW.md`'s "The gate" section, which today describes `--floor` as
  "everything needing no GPU and no corpus" and lists the profiles. It gains the
  second GPU gate. **Committed separately from the feature change**, per that
  file's own closing section

## Open questions

None. All three were answered in the S11 consolidated design round, and the
fourth was decided in this plan with its reason recorded below.

## Decisions from the S11 design round

**1. Recover from `Unknown`, refuse `Destroyed`.** Decided in this plan rather
than asked, because the alternative has a failure mode with no upside: a
shutdown path that calls `Device::destroy` would get a new device built behind
it, and on a repeated teardown that is a loop. `Unknown` is what the pinned
wgpu's own documentation attaches to "driver errors", and with the browser
backend mapping `GpuDeviceLostReason::Unknown` onto it, that is section 22's
population. **The consequence to hold in review:** the only loss reason this
project can produce deterministically is the one it refuses to recover from, so
the recoverable arm is reached by an injected record and the plan says so in the
test table rather than claiming the path is covered end to end.

**2. A new `gpu|YES` gate, not a fold into `oracle` and not hand-running.**
Approach section 7 is the change. Folding into `oracle` would conflate "the
corpus diverges from cornerstone3D" with "this machine has no working device",
which are different failures with different responses. Leaving the tests
`#[ignore]`d with no gate would make F-041's stated evidence evidence that no
profile checks, and the sprint would close green on a shader nothing verified.

**3. `ocelli-render` is not wired into `ocelli-wasm`, and the forecast that said
it would be is corrected.** Approach section 8. The size budget is expected to
stay at 16,388 bytes through S11, and a change in it is a finding.

**4. The `Edge` and `Passes` newtypes are taken.** `probe::measure_with`'s doc
comment names this story as the one to argue it, and the argument is
`AGENTS.md`'s: two adjacent `u32` arguments that can be transposed at a call
site no test reaches is a case a type can remove. It is the only change this
story makes to code it is not otherwise touching, and it is made because the
file it lives in asked for it by name.
