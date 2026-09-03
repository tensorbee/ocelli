# A7, is tier C worth building, and where does its value come from?

**Gate**: `docs/spikes/GATES.md`, A7.
**Gates stories**: F-X001 to F-X004. **Created by**: deviation D-07.
**Status**: **RESOLVED. Outcome `Pass`.** Every sub-question is decided.
The two remaining figures are measurements, and they are acceptance criteria
on F-X002 and F-X003 rather than open gate questions. The gate's job was to
decide whether to build tier C and how, and that is answered.

---

## A7.1, how large is the no-GPU population, really?

**Answered. Deployments are assumed to span GPU-capable clients and GPU-less
ones, both in clinical use.**

The GPU-less class is not exotic. It includes virtualised desktops without GPU
passthrough, locked-down builds where hardware acceleration is disabled by
policy, and hosts whose driver is blocklisted. In clinical estates it is
common.

### What this changes

The gate was written expecting this question to weaken tier C. It does the
opposite.

The reasoning in `GATES.md` was that WebGL2 support is broad and getting
broader, so a browser with neither WebGPU nor WebGL2 is rare. **That reasoning
is about the consumer browser population and it does not transfer to a managed
enterprise estate**, where GPU access is most often absent, virtualised, or
switched off by policy rather than by capability.

So the outcome is not the expected `Partial`. On A7.1 alone it is a **`Pass`**,
and tier C stops being a defensive fallback and becomes a **rendering path a
substantial share of clinical users may actually sit on**.

### Why a virtualised desktop is the hard case, specifically

A virtual desktop has a GPU only if someone bought and configured one for it,
which makes it the clearest example of the GPU-less class. Three arrangements,
and they are genuinely different targets:

| Arrangement | WebGPU | WebGL2 | Ocelli tier |
|-------------|--------|--------|-------------|
| With GPU passthrough | likely | yes | A, or B |
| No GPU, software rasteriser permitted | no | **yes, but software** | **B, and that is the trap** |
| No GPU, acceleration disabled by policy | no | no | **C** |

**The middle row is the one that will hurt**, and it is the finding of this
answer. See below.

Two further properties of a shared virtual desktop matter and neither is about
capability:

- **CPU is shared and it is the product.** The economics are sessions per host.
  A renderer that burns a core does not slow one user, it reduces density for
  everyone on that host, and a platform team will disable the offending
  application rather than buy more hosts. **A CPU renderer there is judged on
  CPU cost per session at least as much as on latency**, which A7.3 as
  originally written does not measure.
- **The pixels are encoded and streamed.** Every frame Ocelli changes is a
  frame the remoting stack must encode and ship. A full-screen change per
  animation frame, which is exactly what a window and level drag or a cine loop
  produces, is the worst case for that encoder.

---

## The finding this answer produced

**A software rasteriser reports itself as WebGL2, and the tier logic as
specified would believe it.**

HLD §7 resolves the tier once at startup from what the platform reports. On a
host with no GPU, Chrome commonly falls back to SwiftShader, which presents a
conforming WebGL2 context. `Caps` sees WebGL2, resolves **tier B**, and Ocelli
then runs GPU code paths on a software rasteriser.

That is worse than resolving to tier C, in three separate ways:

1. **It is probably slower than our own CPU path**, because a general-purpose
   GL software rasteriser is doing work our CPU path would skip entirely, and
   it is doing it through a driver-shaped API.
2. **It burns more CPU**, which on a shared host is the resource that actually
   matters.
3. **It is invisible.** Everything reports as working, so it presents as "the
   viewer is slow on that estate" rather than as a tier misdetection, and it is
   diagnosed months later by someone reading a renderer string.

**So tier resolution must distinguish a hardware adapter from a software one,
and must not treat "WebGL2 is present" as "tier B is appropriate".** This is a
scope addition to F-X001 and is recorded there.

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

---

## What this does to the plan

| Item | Before | After |
|------|--------|-------|
| Tier C justification | Rare browsers with no GPU at all | A primary clinical deployment class |
| A7 likely outcome | `Partial`, test mechanism only | `Pass` on A7.1 |
| F-X001 scope | Add `Tier::Cpu`, feature availability | **Plus software-adapter detection**, so tier B is not chosen on a software rasteriser |
| A7.3 measurement | Interaction latency | **Plus CPU cost per session**, which is what a shared host is sold on |
| F-X003 priority | After the stack viewport | Unchanged in order, raised in importance |

**Story order does not change.** F-X001 is already in S04 and F-X003 already
sits immediately after the stack viewport lands in S15. The dependency chain
was right, only the priority was understated.

---

## The consequence nobody asked about, and it is the serious one

**A mixed estate means two radiologists can open the same study and see pixels
produced by different code paths.** One on tier A, one on tier C.

For a clinical product that is a real question, not a theoretical one. HLD
decision D14 already refuses to claim bit-exact reproducibility and commits
instead to publishing a **measured divergence bound**. That commitment was made
about divergence across GPUs and across the browser, desktop and server
targets. **It now has to cover tier A against tier C as well.**

This is tractable, and cheaply, precisely because of §18: the LUT chain exists
once in `ocelli-pixel` and the shader reads its parameters rather than
reimplementing them. Tier C runs the same arithmetic. The remaining divergence
is sampling, interpolation and rounding in the raster stage, which is bounded
and measurable.

**So the oracle must diff tier C against tier A over the corpus, and the bound
must be published alongside the cross-GPU one.** Recorded as a scope addition
to F-X003. Without it, "the CPU path is fine" is an assumption in a product
where assumptions about pixels are the stated primary risk.

---

## Still open

## A7.2, answered. Three test layers, and tier C needs no adapter at all

**The reframing that decided this: tier C is our own Rust writing RGBA into a
buffer, so it needs no GPU adapter to test.** The bulk of tier C correctness is
a plain `cargo test`, free and on every pull request. A software adapter is
only needed for the **wgpu** paths, which is a different problem that had been
folded into the same question.

| Layer | Mechanism | Catches | Cost | Runs |
|-------|-----------|---------|------|------|
| **1** | `cargo test`, no adapter | Tier C raster and LUT arithmetic | free | every PR |
| **2** | lavapipe via `force_fallback_adapter` on `ubuntu-latest` | wgpu pipeline construction, bind-group layouts, WGSL compilation, tier-B shader variants | free | every PR |
| **3** | Headless Chrome with SwiftShader | What a GPU-less session actually runs | slow | nightly or manual |

**Layer 2 is not a simulator of the real thing and must not be described as
one.** lavapipe is a Vulkan software rasteriser reached through wgpu. A
GPU-less browser session runs SwiftShader reached through Chrome's WebGL2.
Layer 2 proves the pipeline builds and the shader compiles. Only layer 3 says
anything about real GPU-less behaviour.

**No layer is an oracle.** The reference is cornerstone3D through the harness,
per §11. A software adapter proves code runs. It does not prove a pixel is
right.

**Still to measure**, and it is F-X002's acceptance criterion rather than a
gate question: that lavapipe actually resolves as a fallback adapter under the
pinned wgpu version on `ubuntu-latest`. If it does not, layer 2 drops and
layers 1 and 3 stand.

## A7.3, answered. Beat the incumbent, then ratchet

**The budget is set relative to the viewer the estate already permits**, not
from an invented absolute.

1. Measure the incumbent viewer on a representative GPU-less session. Record
   its interactive cost and its idle cost as fractions of one vCPU.
2. **Ocelli's budget is no worse than those two numbers.**
3. Record Ocelli's observed figures and gate on regression beyond a tolerance,
   the same mechanism `ci/wasm-size-budget.json` already uses for binary size.

**Why relative and not absolute.** It needs no external answer, so it does not
block F-X003. It is measurable as soon as there is a tier C path. And it is the
argument that actually wins the conversation with a platform team: "no worse
than what you already run" is a claim they can verify and act on, where "0.4 of
a vCPU" invites a negotiation about whose vCPU.

**Report interactive and idle separately.** They answer to different people and
only one is negotiable. **Ocelli's idle cost must be indistinguishable from
zero**, because it multiplies by every open session on the host, and a viewer
that costs anything while nobody touches it is the one a platform team
removes first.

Also report frame change rate under a cine loop, because on a remoted session
every changed frame has to be encoded and shipped.

## A7.3b, answered. Progressive refinement

**Reduced resolution during interaction, full resolution on settle.**

This is already the HLD's idiom rather than a new invention: §8 assembles
volumes progressively and refines as slices land, and §30 selects a level of
detail by projected voxel size. Tier C applies the same idea to the raster
stage.

It helps twice on a GPU-less remoted session, which is why it is the right
answer here specifically:

- **Less CPU during interaction**, which is when the budget is under pressure.
- **Fewer full-frame changes for the display encoder**, which is the second
  cost on a remoted session and the one that is easy to forget because it is
  invisible from inside the browser.

**Diagnostic reading happens on the settled frame**, so quality where it counts
is unchanged. Two constraints follow and both are F-X003's job:

- **The settled frame is the one the divergence bound is measured on.** An
  interaction frame is explicitly not claimed to match tier A.
- **The user must be able to tell settled from interacting.** A radiologist
  must never measure or read from a frame that is still refining, and the
  distinction cannot be left to feel. This is the same honesty requirement as
  the feature-availability contract, applied to a frame instead of a feature.

---

## A7.1b, assume GPU passthrough or not?

**Answered. Assume not. Plan for the GPU-less class having no GPU at all.**

So deployments split cleanly, and tier C is not a tail:

```
GPU client         ->  WebGPU              ->  TIER A
GPU-less session   ->  no adapter, or a
                       software rasteriser ->  TIER C
```

**Tier C carries a substantial share of clinical use.** It is a first-class
rendering path with a first-class user, not a degraded mode, and it should be
planned, staffed and reviewed as one.

### Three of the eight Part III capabilities do not reach the GPU-less class

This is the finding worth escalating, because it is a product fact rather than
an engineering one, and it is not visible from any single story.

| HLD Part III capability | Needs | Reaches a GPU-less session |
|-------------------------|-------|-------------|
| §30 Out-of-core volume streaming | GPU residency and 3D textures | **No** |
| §31 WebGPU compute subsystem | WebGPU compute | **No** |
| §32 Prompted segmentation, SAM2 on WebGPU | WebGPU compute | **No** |
| §33 Multi-monitor | Window Management API | Yes, platform-dependent |
| §34 Standards-native annotations | Nothing GPU | Yes |
| §35 Whole-slide imaging | GPU for the pyramid at scale | Partially |
| §36 Attestable rendering | Nothing GPU | Yes, and see below |
| §37 Live multi-user sessions | Nothing GPU | Yes |

Three of the differentiators the programme is built on, including two of the
four HLD C.3 identifies as unclaimed by anyone commercial or open source, are
unavailable to half the intended users. **That does not make them wrong.** The
GPU half is real and those capabilities are why it chooses Ocelli. But the
pitch, the demo and the evidence package have to be honest about which half of
the estate each claim is addressed to, and right now they are written as though
there is one estate.

**This belongs in front of a product decision, not inside a spike record.**

### The architecture bet pays off, for a different reason than the HLD gives

The HLD argues for Rust and WebAssembly on GPU-adjacent grounds: no garbage
collector during a 300 MB volume load, pixels never becoming JavaScript
objects, one device instead of pooled WebGL contexts.

**On a GPU-less session none of those arguments apply, and the choice is more
important rather than less.** A CPU renderer in JavaScript would not be viable
at diagnostic resolution. A CPU renderer in Rust compiled to WebAssembly, with
SIMD128, plausibly is. The language choice is what makes tier C exist at all,
and that is a stronger and simpler argument than the one currently written
down.

### What this changes in the plan

- **wasm SIMD128 stops being a detection detail and becomes a requirement.**
  F-004 (E1.4) already detects it. Tier C's LUT and raster inner loops must
  use it, and the fallback path for a runtime without it needs measuring
  separately, because that combination is the genuine worst case.
- **Threading stays off, and now for a second reason.** HLD decision D5 keeps
  the build single-threaded per worker and says to escalate only on a
  measurement that demands it. A CPU renderer carrying a large share of the
  load looks like that measurement, and it is not. **On a shared host,
  spending more cores per session is the opposite of what the platform
  wants**, because density is sessions per host. D5 holds, and tier C should
  be judged on CPU spent, not on wall-clock alone.
- **MPR on CPU becomes required rather than optional.** A user on a GPU-less
  session has no other route to a reformat. Split out of the volume decision,
  see F-X004 and F-X005.
- **F-114 (E19.1) widens.** "Browser matrix validation and WebGL2 tier
  certification" must certify tier C on a representative GPU-less session, not
  only tier B in a browser.
- **F-006 (E1.6), the benchmark harness, must measure tier C from the start**,
  including CPU cost per session. A harness that only measures a GPU path
  cannot answer A7.3.

### The CPU budget, and how to set it

Do not invent a number. Ask the platform team for the density target, sessions
per physical core, and derive from it. The method:

1. Take the density target, for example 6 sessions per core.
2. That is the ceiling for **sustained** cost during interaction, shared with
   the browser, the OS and everything else in the session.
3. Ocelli's idle cost must be indistinguishable from zero. A viewer that costs
   anything while nobody is touching it multiplies by every open session on
   the host.
4. Report the interactive figure and the idle figure separately. They are
   answerable to different people, and only the second is negotiable.
