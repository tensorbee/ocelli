# GPU ownership

**F-IDs that contributed:** F-004, F-005, F-008, F-037, F-041, F-X001
**Last updated:** 2026-09-15

One device, one queue, one owner. HLD section 31's first bullet, made into a
mechanism. The quote below alters that bullet in one place, splitting its
single semicolon into two sentences, because `docs/hld/` is exempt from this
repository's no-prose-semicolon rule and `docs/lld/` is not.

> **Shares the renderer's device.** ocelli-compute never creates a
> wgpu::Device. It borrows the one ocelli-render owns. Two devices cannot share
> textures, which would defeat the entire point.

This is the Phase 1 hook of HLD section 38. Its stated alternative is "a
device-sharing retrofit across the renderer", which is why a contract exists
here before anything uses it.

## The types

| Type | Crate | What it is |
|------|-------|-----------|
| `Tier` | `ocelli-render` | `A` WebGPU, `B` WebGL2 downlevel, `Cpu` under deviation D-07 |
| `Caps` | `ocelli-render` | HLD section 22's struct, field for field, plus the third tier |
| `GpuContext` | `ocelli-render` | The device, the queue, the caps and the loss slot, owned together |
| `ResolvedAdapter` | `ocelli-render` | The adapter the tier resolved on, retained so a lost device can be rebuilt. Holds no device |
| `DeviceState` | `ocelli-render` | `Live`, or `Lost` with the reason and wgpu's text. HLD section 22's "real state" |
| `DeviceError` | `ocelli-render` | `Refused`, `NotLost`, `Unrecoverable` |
| `Recovered` | `ocelli-render` | What a rebuild produced. `Caps` today, and what F-038 to F-040 extend |
| `ComputeCtx<'a>` | `ocelli-compute` | A borrow of a `GpuContext` and a command encoder, for one dispatch |
| `Kernel` | `ocelli-compute` | HLD section 31's trait, signature as given |
| `ComputeError` | `ocelli-compute` | `Unavailable` and `Workgroup` |

## The dependency direction, and why it is not a cycle

`ocelli-compute` depends on `ocelli-render`. Section 31 fixes it by saying the
renderer owns the device and compute borrows it.

**F-041 added `ocelli-render` on `ocelli-pixel`**, and it points the other way
for a different reason. HLD section 18 says to implement the LUT chain once in
`ocelli-pixel` "and let the shader read the parameters", which requires the
renderer to be able to see them. `VoiParams::from_chain` reads a resolved
`LutChain` and computes nothing. Section 4's crate table forbids no direction,
and the alternative, putting a `#[repr(C)]` uniform in a `no_std` crate that
holds no GPU type, would move a rendering concern into the pixel crate to avoid
an edge that costs nothing.

The same story added `ocelli-core` as a DEV dependency of `ocelli-render`,
because `LutChain::map_into`'s signature is in `Stored` and `Display` and the
sweep test compares against it. That one never enters the shipped surface, which
reaches `ocelli-core` through `ocelli-pixel` and does not name it.

It is not a cycle because **the renderer consumes compute results, it does not
call kernels**. The caller that drives both is `ocelli-viewport`. Section 31's
last bullet is the reason the direction matters at all: "Results stay resident
on the GPU wherever the consumer is the renderer. A segmentation mask should
never round-trip through JavaScript to be drawn."

## What enforces the contract, in order of strength

### 1. The type system

`GpuContext` holds the device and queue privately and exposes `device()`,
`queue()` and `caps()`, all shared borrows. **There is no accessor that yields
an owned `Device` or `Queue`.**

**What that does and does not defend against, because the obvious reading is
too strong.** `wgpu::Device` is itself `Clone`, measured by a test in `gpu.rs`
rather than assumed, and it is a refcounted handle, so cloning one yields the
SAME device. Section 31's concern is that "two devices cannot share textures",
and a second device only arrives from a second `request_device`. That is what
`ci/check-device-ownership.sh` refuses, and it is the load-bearing guard.

`GpuContext` is still not `Clone`, for a smaller and separate reason: the
device, the queue and the resolved `Caps` should have one owner. A second owner
is not a second device, it is a second place to look. That one is a grep in the
guard script rather than a test, because asserting the ABSENCE of a trait impl
at compile time needs specialisation.

`ComputeCtx<'a>` borrows rather than owning, so a kernel cannot retain a device
beyond the dispatch it was handed. The encoder is borrowed for the same reason
and one more: section 22 requires one `queue.submit()` per frame across all
viewports, and a kernel that owned its encoder would submit its own work.

### 2. Compile-fail cases

`crates/ocelli-compute/tests/ui/`, driven by trybuild, the same harness F-001
used for coordinate-space mismatches.

| Case | Error it must produce |
|------|----------------------|
| `no_owned_device_out_of_context.rs` | `E0599`, no method named `into_device` |
| `context_cannot_outlive_its_borrow.rs` | `E0515`, cannot return a value referencing a function parameter |

**Neither needs a GPU or an adapter**, so both run in the CI floor, where no
real device will ever exist. That is the whole reason the contract is expressed
in types: it is checkable in the environment the project actually has.

**Both cases take their values as parameters, and that is not a style choice.**
The first draft of the lifetime case built them with `unimplemented!()` and
**compiled**, because a diverging initialiser makes the rest of the function
unreachable and the borrow checker never runs. It passed as a compile-fail case
that did not fail. Any new case here must avoid a diverging expression before
the line under test.

**F-037 added a second `tests/ui/` directory**, in `ocelli-render`, with
`workload_dimensions_are_not_interchangeable.rs`. It is not about device
ownership and is listed here because this is where the harness is described:
`Edge` and `Passes` must stay distinct types, so the `edge`/`passes`
transposition `probe::measure_with` carried as an open residue for nine review
passes stays a compile error. It follows the same parameter rule for the same
reason.

### 3. `ci/check-device-ownership.sh`

The weakest of the three, and it catches what the other two cannot: **a crate
creating a device it never puts in a `GpuContext` at all.** No type is involved
in that, so no type can refuse it.

Four assertions, each proved red by mutation. `grep -c 'fail=1'
ci/check-device-ownership.sh` prints how many the script carries, and the table
below has one row each. It said three over a four-row table from S02 until the
S03 review's ninth pass, which is what a count written beside the thing it
counts does.

| Assertion | Mutation that proves it |
|-----------|------------------------|
| No crate outside `ocelli-render` names `Instance::new`, `request_adapter`, `request_device` or `create_surface` | a `request_device` call added to `ocelli-compute` |
| `ocelli-render` still defines `GpuContext` | the struct renamed |
| `GpuContext` has no `into_device`, `into_queue`, `take_device` or `clone_device` | such an accessor added |
| `GpuContext` does not derive `Clone` | `#[derive(Debug, Clone)]` on the struct |

The second assertion is not the same as the first, and it is there on purpose.
The rule is **not** satisfied by nobody holding a device. It is satisfied by
exactly one crate holding it, so deleting the contract has to fail too.

## Tiers

| Tier | What this contract does |
|------|------------------------|
| A, WebGPU | Full. Compute kernels are tier A by definition, and `Caps.compute` says so |
| B, WebGL2 | The contract holds and no kernel runs, because tier B has no compute shaders. A kernel whose `tier()` is A with no declared fallback marks its feature unavailable |
| C, CPU | Not constructible, and F-037 made that a mechanism rather than a consequence. `caps::opens_a_device` is false for tier C and `probe::resolve_adapter` returns nothing, **even when an adapter opened perfectly well and D-07's combination rule demoted it**, so a tier C session has no `GpuContext` and no `ComputeCtx`. Every kernel resolves through its section 31 fallback or reports unavailable |

**Whether a kernel may run is one predicate and it lives in `caps`.**
`ocelli_render::caps::compute_available` requires BOTH
`caps.compute`, which is what the adapter reports, and
`caps.tier.supports_compute()`, which is what this project resolved to. It is a
conjunction, and `GpuContext::supports_compute` forwards to it and does nothing
else. The conjunction is the content: an
adapter can report compute support while D-07's combination rule has already
resolved tier B or tier C because that adapter is a software rasteriser, so an
`||` in that expression reports compute available on a tier that cannot run it. It was
written out on `GpuContext`, where `new` needs a real device and the CI floor
has none, so nothing could reach it and the `||` mutation survived nine review
passes. `lib.rs` states the split it now obeys: everything that can be wrong
about a tier is in `caps`, which needs no adapter to test. The six-row truth
table is `caps::tests`.

**The forwarder was reachable by no test until F-037, and is now reachable by
one that needs hardware.** `GpuContext::supports_compute` needs a `GpuContext`,
`GpuContext::new` needs a real `Device` and `Queue`, and deviation D-04 leaves
the floor without an adapter to make either. Measured for the S03 review's tenth
pass: negating the forwarder's body left `bin/ocelli.sh test ocelli-render` at
exit 0 with 63 passed and 0 failed, and the method had no other caller in the
workspace. A wrong forwarder answers `true` on a tier B context, which is the
section 31 failure the predicate exists to prevent.

F-037 built the `GpuContext` a test can construct, and
`crates/ocelli-render/tests/device.rs`'s
`compute_availability_on_a_real_context_matches_the_decision` drives the
forwarder against `compute_available` on a real device. **That closes it on a
machine with an adapter and not in CI**, which is the precise statement: the
test is `#[ignore]`d and `bin/ocelli.sh gate gpu` runs it, in `--sprint` and
`--all` and never in CI. On CI the human check
`docs/hld/24-agent-code-standards.md` section 27.3 requires is still the last
line of defence. F-X002's software-adapter path is what would close it there,
and it is S14.

Which of the three a session gets is F-004's, and the rule is in
[tier-resolution.md](tier-resolution.md). The short version: three signals,
the fill-rate benchmark decides where it decided, and a candidate the evidence
calls a software rasteriser resolves tier C rather than tier B.

`ComputeError::Unavailable` names both the required and the resolved tier,
because "unavailable" without them is a message nobody can act on. Deviation
D-07's rule is unchanged by this story: a feature that cannot run on the
resolved tier reports unavailable and never silently produces a different
answer.

The general feature result, including the difference between a declared
fallback and no valid path, is defined once in
[feature-availability.md](feature-availability.md). `ComputeError` is the
in-process compute example. It is not a second availability contract and it
does not yet map to stable boundary operands.

**Both variants now have a stable number at the boundary**, added by F-005:
`ErrorCode::Unavailable` is 700 and `ErrorCode::Workgroup` is 701, registered
in `ci/error-codes.json` inside `ocelli-compute`'s declared range of 700 to
799. The Rust types are unchanged and no conversion is written yet, because
nothing crosses the boundary until F-101. What the numbers buy today is that a
tier C session reports a tier A feature unavailable in exactly the encoding a
tier A session would use to report a device loss, so the shell needs one path
for that situation and not two. See `docs/lld/errors.md`.

## What this story deliberately does not do

- **It does not create a device.** `GpuContext::new` takes one that already
  exists. **F-037 has since supplied the thing that creates one**, and it did
  not move it here: `probe::ResolvedAdapter::open` calls `request_device` and
  hands the result to this constructor, so the decision about WHICH device to
  open stays in `probe` beside the tier resolution it comes from, and this type
  still only owns a device that exists. See "The device lifecycle" below.
  F-039 is E6.3 in S13 and is OffscreenCanvas.
- **It does not detect `Caps`.** `caps.rs` defined the type because section
  31's `Kernel::workgroup` takes a `&Caps` and a hook expressed in types needs
  the types. **F-004 has since filled it**, in the same file plus
  `probe.rs`, and [tier-resolution.md](tier-resolution.md) is where that lives.
  F-004's probe device is transient: created, measured on and dropped inside
  `resolve`, so it never becomes a `GpuContext` and no second device exists on
  that path. The recovery path is the exception and is scoped below, under
  "The device lifecycle".
- **It supplies no `Kernel` implementer.** The trait is declared with none, and
  `AGENTS.md` forbids that shape. The rule exists to stop invented
  abstractions, and this one is prescribed: HLD Part II says a given signature
  is the intended implementation. The collision was raised in the design plan
  and decided in the sprint's design round rather than resolved quietly.
  F-125 (E31.1) supplies the kernels.

## The device lifecycle

F-037. HLD section 22: "**Device loss is a real state, not an error path.**
Handle device_lost, rebuild the device and all resources, and restore viewport
state from the shell's copy."

| Step | Where | What it is |
|------|-------|-----------|
| Decide the tier | `caps::classify` | F-004, no GPU touched |
| Decide whether a device is opened at all | `caps::opens_a_device` | Total match. Tiers A and B yes, tier C no |
| Keep the winning adapter | `probe::resolve_adapter` | Returns `ResolvedAdapter`, or nothing for tier C |
| Open the one long-lived device | `ResolvedAdapter::open` | The second and last `request_device` in the workspace |
| Observe a loss | the callback `GpuContext::new` registers | Writes one `LossSlot`, first loss wins |
| Decide whether to rebuild | `caps::recovers_from` | Total match. `Unknown` yes, `Destroyed` no |
| Rebuild | `GpuContext::recover` | Re-opens on the same adapter, returns `Recovered` |

**The split is the same one tier resolution already has.** Both decisions are in
`caps`, are total matches over their input, and need no adapter, so
`cargo test --workspace` reaches them in the CI floor that deviation D-04 leaves
without a GPU. `probe` holds the wgpu calls. `gpu` holds the device once it
exists.

**On the resolution path there is still never a moment when two devices
exist.** `resolve_adapter` retains an `Adapter`, which is not a device. The
probe's device is created, measured on and dropped inside `detect` before
`resolve_adapter` returns, and the long-lived one is opened afterwards, by a
separate call the caller makes.

**On the RECOVERY path there is such a moment, deliberately, and this sentence
used to be unqualified.** `GpuContext::recover` opens the replacement before it
replaces the context, so between those two statements the old handle and the new
one are both alive. Releasing the old one first would need an `Option` or a
`destroy()`: the first leaves a context that can be observed with no device, and
the second converts a recoverable `Unknown` loss into a deliberate teardown and
leaves a failed rebuild with nothing at all. Opening first is what makes the
refusal path safe, which is `recover`'s documented guarantee that a
`DeviceError::Refused` leaves the caller exactly where it started.

Section 31's invariant is untouched, because it is stated as "two devices cannot
share textures, which would defeat the entire point". Nothing is shared across
that line: the old device is lost in every non-test path, this workspace has no
texture type for either to hold, and the old one is dropped at the assignment.
`ci/check-device-ownership.sh` is about which crate may CREATE a device and is
unaffected.

**`Unknown` recovers and `Destroyed` does not.** The pinned wgpu offers exactly
two reasons, so the policy is a two-row table. `Unknown` is documented by
`wgpu-types` as "lost for an unspecific reason, including driver errors", which
is section 22's population: the driver reset, the tab backgrounded too long, the
OOM. `Destroyed` is `Device::destroy` having been called, which is the
application's own teardown, and rebuilding behind a shutdown path is a loop.
Both matches are written without a wildcard arm, so a future wgpu adding a third
reason stops the build rather than letting it inherit an answer nobody chose.

**What recovery does NOT do, and it is two of section 22's three clauses.**
There are no resources and no viewports in this workspace, so `Recovered`
carries the rebuilt device's `Caps` and nothing else. It is a struct rather than
a bare `Caps` so F-038's render graph, F-039's canvas and F-040's textures
extend it instead of changing `recover`'s signature.

**`caps` cannot currently differ from the pre-loss `caps`**, and this paragraph
said it could until the F-037 review's second pass. `ResolvedAdapter::open`
copies the `Caps` the tier resolved to, which is immutable on the retained
adapter, so a rebuild reports the same four values by construction. The field is
a convenience, not a warning. Re-deriving limits from the adapter on a rebuild
would be a second place `Caps` is built, and the tier must not be re-resolved at
all, because HLD section 7 says it "resolves once at startup". If a driver reset
ever does change an adapter's limits under a live session, closing that is a
decision with its own design plan rather than a field whose documentation
promises what the code does not do.

### What the tests prove, and the two things they do not

`crates/ocelli-render/tests/device.rs` and one `#[ignore]`d case in `gpu.rs`,
both run by `bin/ocelli.sh gate gpu`. **They are not that gate's whole set**,
which is every `#[ignore]`d test in the crate: the device lifecycle, the tier C
short-circuit test, the fill-rate instrument, and F-041's LUT-shader comparison.
What follows is the device lifecycle alone. A device opens and agrees with the resolved tier. A destroy is
observed as a loss carrying `Destroyed`. That destroy is
refused rather than rebuilt, and the refusal leaves the state it refused to act
on unchanged. Recovering a live device is a different refusal, `NotLost`. An
injected `Unknown` is rebuilt and the rebuilt context starts `Live` rather than
inheriting the loss it was rebuilt from.

**First, the injected `Unknown` is fabricated input and is weaker than a real
one.** The pinned wgpu can produce exactly one loss on demand and it is the one
this project refuses, so the callback path is proved end to end on `Destroyed`
and the rebuild path is proved on a record the test writes through
`GpuContext::inject_loss`, a `pub(crate)` and `#[cfg(test)]` seam that applies
the same `record_loss` the callback applies. **That a real driver reset produces
`Unknown` and reaches the same slot is not proved and cannot be proved on one
machine.**

**Second, and this is the sharper gap: nothing here catches a regression in the
device's requested limits.** `ResolvedAdapter::open` asks for the adapter's OWN
limits and never `Limits::default()`, which is the whole reason a tier B adapter
can open a device at all. F-037 measured the mutation: replacing that line with
`wgpu::Limits::default()` left `cargo test -p ocelli-render --test device --
--ignored` at exit 0, 6 passed and 0 failed, on the aarch64-apple-darwin Metal
adapter in `ci/tier-thresholds.json`. A tier A adapter exceeds the WebGPU
defaults, so asking for them succeeds and the mutation is invisible. **Only a
downlevel adapter separates the two, and HLD section 7's tier B has never been
exercised by anything in this project.** F-042 is the WebGL2 story and F-X002 is
the story that gets a software adapter into a test.

## Deviations D-10 and D-12

### D-12, wgpu reaches wasm-bindgen and nothing can stop it

Activating wgpu broke `ci/check-bindgen-isolation.sh`, and the break is a
contradiction inside the HLD rather than a mistake in this story. Section 15.2
specifies wgpu, section 4 says `ocelli-render` builds for wasm, and section
15.3 forbids any crate but `ocelli-wasm` from reaching wasm-bindgen. On wasm32
all three cannot hold, because wgpu talks to the browser's WebGPU through
js-sys and web-sys.

The only route, measured:

```text
wasm-bindgen v0.2.127
|-- js-sys -> wasm-bindgen-futures -> wgpu -> ocelli-render
`-- web-sys -> wgpu -> ocelli-render
```

On the host that route does not exist, so section 15.3's loop still means
exactly what it says and runs unchanged. For wasm32 the rule became **direct
declaration in a crate's own manifest** rather than transitive reachability.
D2's purpose survives: `ocelli-render` carries no browser binding in its source
and compiles for native unchanged, which is the property that makes the desktop
and server targets entry points rather than rewrites.

### D-10, wgpu two sprints early

`ocelli-render` and `ocelli-compute` link wgpu from F-008 rather than F-037,
and both drop `#![cfg_attr(not(test), no_std)]` because wgpu needs `std`. The
pin is untouched. **F-037 and not F-039**, for the reason the section above
gives twice: F-037 is E6.1 in S11, device init and capability tiering, and
F-039 is E6.3 in S13 and is OffscreenCanvas. The root `Cargo.toml` comment
D-10 is written against said F-039 until the S03 review's fifth pass, and
D-10's row in `docs/hld/DEVIATIONS.md` quotes that spelling rather than
endorsing it. `scripts/no_std_check.py` reads the attribute from each crate's
source rather than carrying an exemption list, so these two left the check by
construction.

`ocelli-wasm` does not depend on `ocelli-render`, so **the wasm size budget is
unaffected by this story**. The first story that makes the wasm module reach
wgpu is the one that re-baselines it and says why.
