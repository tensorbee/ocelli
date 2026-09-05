# Tier resolution

**F-IDs that contributed:** F-004
**Last updated:** 2026-09-05

How a session decides whether it is tier A, tier B or tier C, and how it
records why. This describes what the code does today.

HLD section 7 gives one sentence of policy, "the tier resolves once at
startup", and section 22 gives the struct the answer lands in. Deviation D-07
says that resolving from what the platform reports is precisely the defect:

> **Tier resolution must tell a hardware adapter from a software one.** On a
> host with no GPU, a software rasteriser presents a conforming WebGL2
> context, so `Caps` as section 7 specifies it resolves tier B and runs GPU
> paths on a rasteriser that is slower than our own CPU path and burns more
> CPU. Worse, it is invisible, and presents as "the viewer is slow" rather
> than as a misdetection.

Everything below exists to make that one misdetection impossible to make
quietly.

## The two halves, and why they are two files

| File | What it holds | Needs a GPU |
|------|---------------|-------------|
| `crates/ocelli-render/src/caps.rs` | The types and the whole decision procedure. No I/O | no |
| `crates/ocelli-render/src/probe.rs` | The wgpu calls and the fill-rate workload. No decisions | yes |

**Everything that can be wrong is in the half that needs no adapter.** That is
what makes the dangerous decision exhaustively testable in the CI floor, which
deviation D-04 leaves without a GPU. `caps.rs` carries a table of signal
combinations, and `crates/ocelli-render/tests/classify_is_total.rs` carries a
proptest over generated ones. Neither touches hardware.

`probe.rs` has no test row of its own beyond the shape of its constants, and
that is deliberate rather than an omission. `docs/spikes/A7-tier-c.md` section
A7.2 is explicit that no software adapter is an oracle, and the one open
acceptance question it hands F-X002 is whether lavapipe resolves as a fallback
adapter under the pinned wgpu on `ubuntu-latest`, which is its layer 2. F-X002
builds the browser layer too, headless Chrome on SwiftShader, which A7.2 runs
nightly or by hand. What this story can prove without a GPU is the decision,
and the decision is where the defect is.

## The three signals

`docs/spikes/A7-tier-c.md` lists them in order of reliability and says none is
sufficient alone.

| Signal | Source | Says `Software` when | Says `Hardware` when |
|--------|--------|----------------------|----------------------|
| Fill-rate benchmark | `probe.rs`, one startup measurement | at or below the recorded software ceiling | at or above the recorded hardware floor |
| Adapter type | `AdapterInfo::device_type` | `DeviceType::Cpu` | `DiscreteGpu` or `IntegratedGpu` |
| Renderer string | `AdapterInfo`'s `name`, `driver` and `driver_info` | it matches A7's list | never |

Every other case is `Unknown`, which is a first-class answer and not a
failure.

**A renderer string never says `Hardware`.** The absence of a known software
string proves nothing, and browsers increasingly mask the string entirely, so
a non-match abstains.

### The renderer string arrives through wgpu and needs no browser binding

On `wasm32`, wgpu's GLES backend reads `GL_UNMASKED_RENDERER_WEBGL` when the
`WEBGL_debug_renderer_info` extension is available, and glow enables that
extension by default. So `AdapterInfo.name` is already the unmasked string A7
asks for. `ocelli-render` imports no web-sys of its own, and decision D2 and
deviation D-12 are untouched by this story.

### `Gallium` is a known false positive, and the combination rule contains it

A7's list includes `Gallium`. Mesa's Gallium framework backs the `radeonsi`
and `iris` drivers on genuine AMD and Intel GPUs, whose renderer strings have
historically read "Gallium 0.4 on AMD ...", so matching it would classify real
hardware as software and drop a working GPU session to tier C.

**The list is kept verbatim and the narrowing is in how it is used.** A string
is consulted only where the benchmark AND the adapter type both abstained, so
a real GPU that measures `Hardware`, or that reports `DiscreteGpu` or
`IntegratedGpu`, is never dropped by this entry.

**The narrowing is partial, and the residue is a real population rather than a
corner.** Where both abstain the string still decides, and both abstaining is
what a Mesa GPU on wgpu's GLES backend presents. `FillRateBands::RECORDED` has
no software ceiling and a hardware floor of 400 Mpps derived from one Apple
figure, so a genuine but slower GPU measures `Unknown`, and that backend
commonly reports `DeviceType::Other` or `VirtualGpu`, which is `Unknown` too.
`"Gallium 0.4 on AMD RADV POLARIS10"` then resolves tier C, which renders
nothing until F-X001 to F-X004.
`a_real_mesa_gpu_that_both_hints_abstain_on_is_demoted` in `caps.rs` asserts
that outcome in the direction that matters, beside the containment the
combination rule does provide. The decision to keep A7's list verbatim was
reviewed and stands, and what changed here is the claim made for it.

Amending `docs/spikes/A7-tier-c.md` itself is a reviewed change to a resolved
spike and is deliberately not part of F-004.

## The combination rule

`classify(&TierSignals, TierRequest) -> Resolution`, written out because an
`if` chain is not a specification.

1. **An override of tier C short-circuits everything.** No instance, no
   adapter, no device, no benchmark. `decided_by` records `Override`, and
   `measured_tier` is `None` because nothing was measured.
2. **Rank the candidates.** An adapter on `BrowserWebGpu`, `Vulkan`, `Metal`
   or `Dx12` with `DownlevelFlags::COMPUTE_SHADERS` present is an
   **A-candidate**. An adapter on `Gl`, or one without compute shaders, is a
   **B-candidate**. `Backend::Noop` is never a candidate. The best A-candidate
   wins, then the best B-candidate, with the adapter's own device type
   breaking ties within a class and an earlier adapter winning an exact tie.
3. **No candidate resolves tier C**, `decided_by = NoAdapter`.
4. **No device resolves tier C**, `decided_by = NoDevice`, recorded apart from
   `NoAdapter` because they are different diagnoses. What it records is
   narrower than "there is no GPU path to be had", and the narrow statement is
   the true one: **the best candidate could not open a device, and no other
   adapter was tried.** `probe.rs` calls `request_device` on the single adapter
   `choose_candidate` returned and never tries the next, while native
   `enumerate_adapters(Backends::all())` commonly returns several. So a host
   with a broken Vulkan ICD beside a working GL driver picks the Vulkan
   A-candidate, fails, and resolves tier C with a tier-B path present and
   unattempted. Tier C renders nothing until F-X001 to F-X004, so the outcome
   is "renders nothing" rather than "renders on tier B". Trying the next
   adapter is a behaviour change with its own ranking and evidence questions
   and belongs to a story rather than to this paragraph.
5. **If the benchmark decided, it decides.** A7: "The micro-benchmark is the
   one to trust, and the strings are the hint. A renderer string is a claim. A
   measured fill rate is a fact." The other two verdicts are still computed
   and still recorded, and they are not consulted.
6. **Otherwise the adapter type, then the renderer string.** If both abstain
   the GPU candidate is kept, because a figure that landed between two bands
   set an order of magnitude apart is itself evidence that something real
   executed.
7. **Hardware resolves the candidate's tier, A or B. Software resolves tier
   C.** That single line is the whole defect this exists to prevent.
8. **The override applies last**, clamped to what is constructible.

## The fill-rate measurement

### What the workload is

`crates/ocelli-render/src/fill_rate.wgsl`. One oversized triangle covering the
whole viewport, into an offscreen `Rgba8Unorm` texture, with a fixed 64 ALU
steps per fragment, repeated a fixed number of passes, one `queue.submit`.

Fragment throughput is what separates a software rasteriser from a real
adapter by orders of magnitude, and it is also what a stack viewport actually
spends its frame on.

**Nothing is read back.** No `map_async`, no readback buffer, and the target
carries `RENDER_ATTACHMENT` and no `COPY_SRC`. Decision D3 says pixels never
cross the boundary, and a readback would also measure transfer rather than
shading.

### Three stages, and the warm-up is load bearing

| Stage | Size | Timed |
|-------|------|-------|
| Warm-up | 65,536 fragments | no, the figure is discarded |
| Calibration | 65,536 fragments | yes |
| Full | 16,777,216 fragments | yes, and only if the calibration did not EXCEED 2 ms |

**The warm-up is not tidiness.** The first submission on a fresh device pays
for lazy pipeline compilation, driver initialisation and command-buffer setup.
Measured on an Apple M5 Max, twenty release runs in fresh processes: 5.4 to
9.3 ms for the first 65,536 fragments, median 6.1, and 0.32 to 0.70 ms, median
0.36, for exactly the same work immediately afterwards. That is a factor of
between 8.8 and 20.7, and the spread is this machine's noise rather than a
bound.

**What the warm-up prevents is a lost measurement, not a flipped verdict.**
There is no software band to fall into: `software_ceiling_pps` is `None`,
`ci/tier-thresholds.json` records it as `null`, and `FillRate::verdict` returns
`Unknown` rather than `Software` whenever the ceiling is absent, which the
section below states as well. Without the discard, the first submission's 5.4
to 9.3 ms for 65,536 fragments exceeds `CALIBRATION_BUDGET_NANOS` on every one
of those twenty runs, so `probe.rs` returns the calibration figure and the full
pass never runs. The recorded fill rate is then this machine's startup latency,
the benchmark abstains, and the resolver falls through to the adapter type and
the renderer string with a figure in the record that describes something else.

The two-stage split after that exists so a genuine rasteriser does not hang
startup. If the calibration exceeds its budget the full pass is never
attempted and the calibration figure is itself the answer. The comparison in
`probe.rs`'s `measure` is
`calibration.elapsed_nanos > CALIBRATION_BUDGET_NANOS`, so a calibration of
exactly 2 ms still runs the full pass.

### Completion, which is the awkward part

`Device::poll` blocks on wgpu-core backends and its own documentation says
"When running on WebGPU, this is a no-op." So the portable completion signal
is `Queue::on_submitted_work_done`, which sets an atomic flag, with
`Device::poll(PollType::Poll)` driving it natively.

The wait is bounded by the caller's clock at five seconds. In a browser the
callback arrives from the event loop and the poll does nothing, so an
unbounded loop would hang startup. On the timeout the probe reports **no
measurement**, the benchmark verdict is `Unknown`, and the combination rule
falls through to the two hints, which is exactly what it is for.

### The clock is the caller's

`resolve` takes `clock: &mut dyn FnMut() -> u64`, monotonic nanoseconds from
any fixed origin. `std::time::Instant` panics on `wasm32-unknown-unknown`, and
the alternative is a dependency reaching `performance.now()` inside
`ocelli-render`, which is the browser binding deviation D-12 says this crate
must not grow. Natively the caller uses `Instant`. In a browser it is the
shell's `performance.now()` delta coming across the boundary.

### The arithmetic is integer, and there is no cast in it

The comparison is not `pixels / seconds >= threshold`. It is

```text
pixels_shaded * 1_000_000_000  >=  threshold_pps * elapsed_nanos
```

widened to `u128` on both sides with `u128::from`. No float, so `float_cmp`
has nothing to say. No `as`, so HLD 27.3's cast review has nothing to find. No
rounding, so there is no rounding decision to get wrong. The widest inputs the
types allow are `u64::MAX` on both operands, and `(2^64 - 1)^2` is about
3.4028e38, inside `u128`. There is a test at that extreme.

A zero elapsed time is a clock with no resolution rather than an infinitely
fast adapter, and zero pixels measured nothing. Both are `Unknown`, because
the comparison alone would read both as `Hardware`.

## The recorded bands

`ci/tier-thresholds.json`, in the shape `ci/wasm-size-budget.json` set: a
recorded measurement with its provenance, never a constant invented in source.
`FillRateBands::RECORDED` in `caps.rs` carries the same two numbers, and the
test `the_recorded_bands_match_the_checked_in_file` is what stops them
drifting apart.

**The file states its provenance per figure**, because spike gate A7.3's rule
is "do not invent a number" and a recorded-but-not-consulted number must not
read as a trusted one.

| Figure | Value | Provenance |
|--------|-------|-----------|
| Hardware floor | 400,000,000 pixels per second | **Derived**, not measured. Roughly a quarter of the one measured figure |
| Software ceiling | absent | **Not measured.** No software-rasteriser figure can be taken on the machine this was recorded on |
| The measurement it was derived from | 1,671,627,015 pixels per second | **Measured**, Apple M5 Max on Metal, five runs within 17 percent |

**While the software ceiling is absent the benchmark never says `Software`.** A
low rate is `Unknown`, and the combination rule falls through to the adapter
type and then the renderer string. That is the same path a benchmark that did
not run takes, and it is stated here out loud because a benchmark that is
recorded and not consulted otherwise looks identical to one that is trusted.

Erring high on the floor is the safe direction. The defect D-07 exists to
prevent is a rasteriser being called hardware, never the reverse: a genuine
but slower GPU lands in `Unknown` and is then decided by its reported adapter
type, which is the designed fall-through.

Whether lavapipe even resolves as a fallback adapter under the pinned wgpu is
itself unproven and is F-X002's acceptance criterion, per A7.2.

## The operator override

A7 says "always allow an operator override" and nothing else. This is what it
is.

`Tier::from_override_str` parses `a`, `b`, `cpu` and `auto`, case
insensitively and with surrounding whitespace trimmed. An empty value and
`auto` both mean no override, so a deployment can set the variable
unconditionally.

```rust
pub enum TierRequest { Auto, Requested(Tier), Unrecognised }
```

**Three variants and not `Option<Tier>`**, because `auto` and a typo are
different answers. Folding them together would make a mistyped deployment
variable indistinguishable from a deliberate `auto`, which is the silent
ignore the override exists to prevent.

### What it may not do

| Requested | Constructible when | Otherwise |
|-----------|--------------------|-----------|
| `cpu` | always | n/a |
| `b` | some adapter is a candidate **and a device was created** | refused, recorded, the measured tier stands |
| `a` | some adapter is an A-candidate **and a device was created** | refused, recorded, the measured tier stands |

**Both halves, and the second one is not decoration.** An adapter appearing in
the enumeration says a tier-A adapter EXISTS. `signals.device_created` says one
could actually be opened, and they are different facts. The measured path
already refuses a GPU tier without a device, at `DecidedBy::NoDevice`, and
until the S03 sprint review's first pass the override bypassed it:
`OCELLI_TIER=a` on a host where `request_device` failed returned tier A with
`Applied(A)`, against this step's own promise that an override is clamped to
what is constructible. The two `TierRequest::Requested` guard arms in
`classify` carry the corrected condition, `has_a_candidate &&
signals.device_created` and `candidate_tier.is_some() &&
signals.device_created`, and `tests/classify_is_total.rs` asserts it over every
generated combination rather than only over the override path.

**Forcing tier B onto a rasteriser the evidence called software is
deliberately allowed.** That is how the misdetection gets diagnosed on the
estate it happens on. It is recorded in `TierEvidence`, so it is never silent,
and `measured_tier` still says what the evidence found.

### A refused override is an outcome, not an error

`resolve` is infallible and this story introduces **no error type**. Giving a
refusal a code would put a successful resolution into the error space. That is
deviation D-07's own honesty rule applied to the resolver itself, and F-005's
`ErrorCode::Unavailable` keeps its distinct meaning, which is a feature saying
it cannot run on the resolved tier.

### The override's named user

`crates/ocelli-native`, both entry points. They read `OCELLI_TIER`, resolve,
and print the tier with its evidence.

```text
$ OCELLI_TIER=b cargo run -p ocelli-native --bin ocelli-desktop
Tier B, decided by Override. OCELLI_TIER override: Applied(B).
  caps      compute false, max_tex_3d 2048, max_buffer 4294967295
  adapters  1 seen, device created true, measured tier Some(A)
  candidate "Apple M5 Max" on Metal, reported IntegratedGpu
  signals   benchmark Hardware, adapter type Hardware, renderer string Unknown
  fill rate 16777216 pixels in 10594500 ns
```

`AGENTS.md` forbids a feature flag with no named user, and an override no
caller passes is exactly that. The cost is that `bin/ocelli.sh native` step 1
starts linking wgpu, which is a real build-time change to a gate in the floor,
and it is accepted because the alternative is shipping an override nobody can
observe.

In a browser the value comes from the shell instead, because reading a query
parameter, a config endpoint or `localStorage` is DOM work and HLD section 10
puts DOM work in TypeScript. `ocelli-render` receives a parsed `TierRequest`
and reads no environment of its own.

## Filling `Caps`

Section 22's four fields, from the chosen adapter and **not** from section 7's
tier figures. The fourth field is `tier` itself, which the combination rule
above produces, so the table below has three rows rather than four.

| Field | Tier A or B | Tier C |
|-------|-------------|--------|
| `compute` | tier A with `COMPUTE_SHADERS` present | `false` |
| `max_tex_3d` | `Limits::max_texture_dimension_3d`, unclamped | `0` |
| `max_buffer` | `Limits::max_buffer_size`, unclamped | `0` |

Section 7's 2048, 256 and 256 MiB are **floors a feature may assume for its
tier**, not a ceiling to clamp the reported truth down to. Clamping would make
`Caps` lie in the direction of "we have less than we do", which is a different
lie but still a lie.

Tier C's three zeros are not a default. A tier C session has no device, so any
other value would be invented.

## The device is transient

The probe device is created, measured on and dropped before `resolve` returns.
It never becomes a `GpuContext`, so HLD section 31's one-device invariant is
untouched: there is never a moment when two devices exist. The call sits
inside `ocelli-render`, which is the only crate permitted to make one, and
`ci/check-device-ownership.sh` passes unchanged. See
[gpu-ownership.md](gpu-ownership.md).

`request_device` is asked for `Features::empty()` and the **adapter's own**
limits, never `Limits::default()`. A downlevel GL adapter does not meet the
WebGPU defaults, which is the whole reason tier B exists.

**The pinned wgpu documents two different answers to asking for more than the
adapter has, and this code depends on the one the implementation gives.** The
doc comment on `Adapter::request_device`, in wgpu 30.0.1's own source at
`src/api/adapter.rs:49` to `:56`, lists "Limits requested exceed the values
provided by the adapter" under `# Panics`. The same function's signature
returns `Result<(Device, Queue), RequestDeviceError>`, and the wgpu-core
backend converts the core error into `Err` rather than panicking, at
`src/backend/wgpu_core.rs:943` to `:948`, over the
`RequestDeviceError::LimitsExceeded` raised in
`wgpu-core-30.0.1/src/instance.rs:941`. The browser backend maps a rejected
promise the same way. So the `let Ok((device, queue)) =
adapter.request_device(...) else` arm in `probe.rs`'s `measure_chosen` is
reachable and the
`NoDevice` path is not dead code, which it would have to be if the flat "it
panics" this paragraph used to carry were the whole story. Asking for the
adapter's own limits means neither answer is exercised here.

## SIMD, and the threads field that deliberately does not exist

### SIMD

`SimdSupport` is `Simd128`, `None` or `NotApplicable`, derived from
`cfg!(target_arch = "wasm32")` and `cfg!(target_feature = "simd128")`. It is
**a build fact reported honestly, not a runtime probe**, and the reason is
worth writing down: a module compiled with `simd128` will not instantiate on a
runtime without it, so by the time our Rust is running the answer is always
yes. There is no version of this that a Rust function can usefully ask.

The real runtime probe belongs in the shell, before the artefact is fetched.
`packages/core/src/capabilities.ts` exports `wasmSimd128Supported()`, which
runs `WebAssembly.validate` over a committed 29-byte module containing one
SIMD instruction. The bytes came from `wat2wasm` over a two-line `.wat`
reproduced in a comment beside them, never hand-typed.

`SimdSupport` is carried in `TierEvidence` and **not** in `Caps`. Section 22's
four fields are the four fields.

### Threads

`packages/core/src/capabilities.ts` also exports `sharedMemoryAvailable()`,
which is both of `typeof SharedArrayBuffer !== "undefined"` and
`globalThis.crossOriginIsolated === true`. It is a diagnostic for the support
surface and for F-006's harness, and it is the last place threads appear.

**Nothing on the Rust side has a threads field.** Not `Caps`, not
`TierSignals`, not `TierEvidence`. Decision D5 fixes the answer, deviation
D-07 restates it for a second reason, and the strongest way to hold a decision
is to leave nowhere to branch on it. A field would be a place for a future
story to write `if signals.threads` and mean it. There is no such field, so
there is no such line, and that is a stronger guarantee than a comment.

## Deviation D-14, wgpu's `webgl` feature

`crates/ocelli-render/Cargo.toml` takes wgpu with `features = ["webgl"]`.

wgpu 30.0.1's default set is `std, parking_lot, dx12, metal, gles, vulkan,
wgsl, webgpu`, read from the pinned crate's own manifest, and `webgl` is not
among them. `gles` is the native GL backend rather than the browser one. So
without this line the crate reaches WebGPU on wasm32 and cannot reach WebGL2
at all, and HLD section 7's tier B could never resolve in a browser. A tier the
resolver can never return is not a tier.

The pin and the workspace entry are untouched and only the consuming crate's
feature set changes, which is the shape deviation D-09 set for glam. Measured
cost today is zero bytes, because `ocelli-wasm` does not depend on
`ocelli-render` and so never reaches wgpu, so the size budget moves only when
the render path is wired in from S11. That crate had an empty
`[dependencies]` table when this was written and F-005 has since added
`ocelli-core` to it, which changes the premise and not the conclusion.

## What this story deliberately does not do

- **It does not wire tier resolution into `ocelli-wasm`.** That module has no
  dependency on `ocelli-render` and so never reaches wgpu. The boundary is
  E16.2 in S16, and no browser path of the resolver can run before F-037 in S11.
  Wiring it now would add an entry point nothing calls and re-baseline the
  wasm size budget for a feature with no user. F-004 touches
  `ci/wasm-size-budget.json` not at all.
- **It does not create a long-lived device.** The probe device is transient
  and `GpuContext` is still built by whoever owns one. Device creation and
  loss recovery are F-037, which is E6.1 in S11, "ocelli-render: device init,
  capability tiering, device-lost recovery". F-039 is E6.3 in S13 and is
  OffscreenCanvas.
- **It does not render on tier C.** This story resolves the tier. Rendering on
  it is F-X001 to F-X004.
- **It does not amend `docs/spikes/A7-tier-c.md`.** The `Gallium` narrowing is
  recorded above and the amendment to a resolved spike is a separate reviewed
  change.

## The guards this adds

For F-X009, which gives every guard a standing test.

| Guard | Where | What it refuses |
|-------|-------|-----------------|
| `the_recorded_bands_match_the_checked_in_file` | `caps.rs` | `FillRateBands::RECORDED` drifting away from `ci/tier-thresholds.json` |
| `classify_is_total_and_never_invents_a_tier` | `crates/ocelli-render/tests/classify_is_total.rs` | Tier A without an A-candidate, tier B without any candidate, tier C with a non-zero `Caps`, a panic on any signal combination |
| `each_a7_software_renderer_string_resolves_cpu` | `caps.rs` | **Any single** A7 renderer string being dropped from the list, and an entry added to it with no case of its own. Each row is a whole renderer string paired with the one entry it stands for, the rows are asserted equal to the constant in order, and each row is asserted to match its own entry and no other. Until the fifth review pass the `gallium` row read `"Gallium 0.4 on llvmpipe"`, which matches `llvmpipe` on its own, so the entry-by-entry claim this row makes was not true of `gallium` |
| `the_a7_list_is_seven_lowercase_entries` | `caps.rs` | An entry added in mixed case, which the lowercase match would never find |
| `a_real_mesa_gpu_that_both_hints_abstain_on_is_demoted` | `caps.rs` | The `gallium` narrowing being restated as complete. It asserts the demotion that survives it as well as the containment it provides |
| `a_gl_adapter_reporting_compute_shaders_is_still_a_b_candidate` | `caps.rs` | The boundary between the two GPU tiers being read off the downlevel flags rather than off the backend. HLD section 7 names tier B by its API, so a native GLES adapter reporting `COMPUTE_SHADERS` is still tier B. The proptest cannot cover this and must not be asked to: its `a_candidates` count calls `candidate_tier`, so on that branch the property is a tautology |
| `the_adapter_type_signal_speaks_only_where_a7_licenses_it` | `caps.rs` | `VirtualGpu` or `Other` being read as a claim in either direction. A7 licenses the signal "where a fallback adapter identifies itself as one", and `SOFTWARE_RENDERER_STRINGS`'s stated residue leans on `VirtualGpu` abstaining |
| `a_virtual_gpu_does_not_out_rank_a_software_renderer_string` | `caps.rs` | A virtualised adapter keeping a GPU tier on a device-type claim, ahead of a renderer string that named a rasteriser |
| `the_device_type_preference_is_strict_at_every_step` | `caps.rs` | Any step of the device-type order being reordered, in either list position. The rank decides which adapter's limits and renderer string reach `Caps` and `TierEvidence`, so a wrong order is a wrong record and not only a wrong preference. `the_ranked_adapters_own_limits_are_the_ones_recorded` is the record half |
| `an_exact_tie_is_won_by_the_earlier_adapter` | `caps.rs` | Two adapters of equal class and equal device type resolving by enumeration order, which moves with a driver update or a hotplugged display |
| `the_override_parser_accepts_exactly_its_four_words` | `caps.rs` | An alias being ADDED to `from_override_str`. Every string of at most three characters over `[a-z0-9-_]` is refused except `a`, `b`, `cpu` and the empty string, so `OCELLI_TIER=c` cannot come to mean tier C on one deployment and be refused on the next |
| `rejects the same module with one byte of its SIMD opcode broken` | `packages/core/src/capabilities.test.ts` | The committed probe module being replaced by a valid non-SIMD one that validates for the wrong reason |
| `the_override_variable_is_named_once` | `crates/ocelli-native/src/lib.rs` | `OCELLI_TIER` being spelled two ways |
