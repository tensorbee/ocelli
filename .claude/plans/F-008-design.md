# F-008, ocelli-compute crate skeleton and GPU device-sharing contract

**Status**: approved
**Epic ref**: E1.8
**Sprint**: S02
**Estimate**: 2w

## Normative source, transcribed

_Transcriptions below are verbatim except for one normalisation: a prose
semicolon in the source is written as a comma, and an em-dash as a hyphen,
because `scripts/prose_check.py` covers `.claude/plans/` and `docs/hld/` is
exempt. No word is changed. Where the exact bytes matter, the tracked
Markdown under `docs/hld/` wins._

### `docs/hld/26-differentiating-capabilities.md`, section 31, verbatim

> WebGPU's rendering advantage over WebGL2 is modest, and NiiVue's maintainers
> are right to say so. The compute advantage is not modest, and as of
> September 2026 essentially nobody in medical imaging is taking it.

```text
// ocelli-compute/src/lib.rs
pub trait Kernel {
    fn tier(&self) -> Tier; // A = WebGPU only, B = has fallback
    fn workgroup(&self, caps: &Caps) -> [u32; 3];
    fn dispatch(&self, ctx: &mut ComputeCtx) -> Result<(), ComputeError>;
}
```

> - **Shares the renderer's device.** ocelli-compute never creates a
>   wgpu::Device, it borrows the one ocelli-render owns. Two devices cannot
>   share textures, which would defeat the entire point.
>
> - **Every tier-A kernel declares a fallback** - CPU, or a worker - so a
>   feature degrades rather than fails on WebGL2. A kernel with no fallback
>   marks its feature unavailable, it never silently produces a different
>   answer.
>
> - **Workgroup sizes come from `Caps`**, never hardcoded. A hardcoded 256 is
>   a portability bug waiting for a device that reports less.
>
> - **Buffer pools by size class**, no per-dispatch allocation - the same
>   discipline as section 20.
>
> - **Results stay resident on the GPU** wherever the consumer is the
>   renderer. A segmentation mask should never round-trip through JavaScript
>   to be drawn.

### `docs/hld/19-render-graph.md`, section 22, verbatim

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

> - **One queue.submit() per frame** across all viewports, from the render
>   worker's requestAnimationFrame.
>
> - **Device loss is a real state, not an error path.** Handle device_lost,
>   rebuild the device and all resources, and restore viewport state from the
>   shell's copy.

### `docs/hld/27-phase1-hooks.md`, section 38, the row that is this story, verbatim

> | ocelli-compute crate exists | E1.8 | 2 wk | A device-sharing retrofit
> across the renderer |

and the framing sentence, verbatim:

> Everything above is post-parity except these. Each costs a few weeks now and
> a rewrite later, which is the only reason they appear in a parity plan at
> all.

### `docs/hld/03-architecture-and-crates.md`, section 4, the row, verbatim

> | ocelli-compute | WGSL compute kernels - segmentation, filtering,
> statistics, resampling. Shares the renderer's device. | yes | yes |

### `docs/hld/12-workspace-and-build.md`, section 15.2, the pin, verbatim

> wgpu = "=30.0.1" \# pin EXACTLY - breaking changes ~quarterly

> *The exact wgpu pin is not fussiness. Agents reliably emit wgpu 0.19-era
> pipeline code, and a caret range lets that compile against something subtly
> different from what the shader expects.*

### `docs/sprints/CURRENT_SPRINT.md`, verbatim

> F-008 is the Phase 1 hook from HLD section 38. It establishes that compute
> and rendering share one GPU device and queue. A second device hidden behind
> a convenient wrapper would make later zero-copy integration impossible.

> - F-008 records and tests the single-device ownership contract used by
>   compute and rendering.

> The dangerous GPU defect is accidental device duplication. F-008 must make
> the shared device and queue contract explicit before compute kernels or
> render pipelines can acquire their own instances.

## What the specification does not cover

Section 31 gives the `Kernel` trait character for character and states the
sharing rule in prose. It does not say:

1. **Which crate owns the type that carries the device.** It says
   `ocelli-render` owns the device and `ocelli-compute` borrows it, which
   fixes the dependency direction as compute depending on render, but it does
   not name the type or say where `ComputeCtx` gets its borrow from.
2. **What `ComputeCtx` is.** It appears in the `dispatch` signature and is
   never defined.
3. **What enforces "never creates a wgpu::Device".** Prose is not a mechanism,
   and section 15.3 shows this project's own answer to that problem for the
   bindgen rule: a check that runs on every pull request.
4. **Whether `Kernel` is declared before an implementer exists.** `AGENTS.md`
   forbids a new trait with fewer than two implementers today, with two named
   exceptions that do not include this one. Section 31 prescribes the trait,
   and F-125 (E31.1, S46) is the story that fills it, ten weeks of it.

## Approach

The story is a contract, not a subsystem. Everything below serves one claim:
after this story it is not possible to add a second device without the change
being visible.

**1. `ocelli-render` gains `GpuContext`, and it is the only place a device and
a queue are held together.**

```rust
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    caps: Caps,
}
```

`Caps` is section 22's struct, transcribed field for field, plus tier C from
deviation D-07. The accessors are `device()`, `queue()` and `caps()`, all
returning shared borrows. There is no accessor returning an owned device,
which is what would let a caller stash a second one.

`GpuContext::new` is the single constructor and it takes an already-created
device and queue. **This story does not create a device.** Adapter
enumeration, tier resolution and device-loss recovery are F-004 and F-039, and
doing them here would be the second copy of a decision this project only wants
once.

**2. `ocelli-compute` depends on `ocelli-render` and defines `ComputeCtx`.**

```rust
pub struct ComputeCtx<'a> {
    gpu: &'a GpuContext,
    encoder: &'a mut wgpu::CommandEncoder,
}
```

A borrow with a lifetime, not an owned handle and not an `Arc`. The lifetime
is the mechanism: a `ComputeCtx` cannot outlive the `GpuContext` it borrows,
so a kernel cannot retain a device beyond the dispatch it was given. The
`&mut CommandEncoder` is what section 22's one-submit-per-frame rule needs,
because a kernel that owned its own encoder would submit its own work.

The dependency direction is compute depending on render, which section 31
fixes and which is not a cycle: the renderer consumes compute results, it does
not call kernels. The caller that drives both is `ocelli-viewport`.

**3. `Kernel` is declared, and the plan says why that is not a structural-rule
violation.**

Section 31 prescribes the trait with its exact signature, and HLD Part II
opens by saying a prescribed signature is the intended implementation.
`AGENTS.md`'s two-implementer rule exists to stop invented abstractions, and
this one is not invented. The plan follows the specification and records the
reasoning here rather than quietly doing either thing.

What the plan does not do is invent a first implementer to satisfy the rule.
The trait is declared with no implementers and F-125 supplies them.

**4. The contract is tested without a GPU, three ways.**

- A `trybuild` compile-fail case: a consumer crate that tries to obtain an
  owned `wgpu::Device` out of a `GpuContext` fails to compile. This is the
  same mechanism F-001 used for coordinate-space mismatches, and it is the
  strongest of the three because it makes the defect a compile error rather
  than a review finding.
- A `trybuild` compile-fail case: a `ComputeCtx` outliving its `GpuContext`
  fails the borrow checker.
- `ci/check-device-ownership.sh`, modelled on `ci/check-bindgen-isolation.sh`,
  refuses any crate other than `ocelli-render` that names
  `wgpu::Instance::new`, `request_adapter` or `request_device` in its sources.

The first two need no adapter and run in the CI floor. The third is textual
and catches the case the type system cannot, which is a crate creating a
device it never puts in a `GpuContext` at all.

**5. Prove each goes red.**

Two mutations, reverted and recorded: add a `pub fn into_device(self)` to
`GpuContext` and watch the first compile-fail case stop failing, and add a
`request_device` call to `ocelli-compute` and watch the guard fail.

## Boundary and tier

- wasm-bindgen: not touched. `ocelli-compute` and `ocelli-render` must not
  reach it, and F-007's strengthened isolation check asserts that under both
  targets.
- Pixels across the boundary: no. This story is entirely below the boundary.
- Render-loop allocation: none. `ComputeCtx` borrows an encoder rather than
  creating one, which is the pre-sizing discipline of section 31's buffer-pool
  bullet applied to the one allocation this story could have made.
- unsafe: none.
- Tier A (WebGPU): full. Compute kernels are tier A by definition, and `Caps`
  carries `compute: bool` to say so.
- Tier B (WebGL2): the contract exists and holds. No kernel can run, because
  tier B has no compute shaders. `Caps.compute` is false and a kernel whose
  `tier()` is A with no declared fallback marks its feature unavailable,
  per section 31. This story declares that shape and F-125 implements it.
- Tier C (CPU): the contract is not constructible, because there is no device.
  A tier C session has no `GpuContext`, so it has no `ComputeCtx`, and every
  kernel resolves through its section 31 fallback or reports unavailable.
  Deviation D-07's rule applies unchanged and this story does not weaken it.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | `Caps` carries section 22's four fields and D-07's third tier | `crates/ocelli-render/src/caps.rs` under `#[cfg(test)]` |
| `property` | No safe path yields an owned `wgpu::Device` from a `GpuContext`, and a `ComputeCtx` cannot outlive its borrow | `crates/ocelli-compute/tests/`, `trybuild` compile-fail |
| `conformance` | Only `ocelli-render` names a device-creating wgpu call | `ci/check-device-ownership.sh`, gate `device` |

No `fixture` row. This story computes no pixel and no coordinate. HLD 27.2 R3
does not apply, and the row is named rather than omitted.

## Parity surface covered

None directly. Appendix B does not enumerate compute. Section 31's initial
kernel set maps to segmentation and MPR rows that F-125 and later stories
cover, and this story only makes them possible.

## Deviations

**D-10**, approved in the consolidated design round and recorded in
`docs/hld/DEVIATIONS.md`:

> §15.2 lists `wgpu = "=30.0.1"` among the workspace dependencies, and this
> repository's `Cargo.toml` comment says the entry activates with F-039. wgpu
> is activated in `ocelli-render` and `ocelli-compute` at F-008, two sprints
> earlier, and both crates drop `#![cfg_attr(not(test), no_std)]`.

The `Cargo.toml` comment naming F-039 as the activating story is corrected in
this change, so the manifest and the deviation register agree.

Note what D-10 does not cover, because the boundary matters at review time:
the pin is untouched, `ocelli-wasm` does not gain a dependency on
`ocelli-render`, and the wasm size budget F-002 baselines is therefore
unaffected by this story. If a later story makes the wasm module reach wgpu,
that is the story that re-baselines the budget and says why.

## LLD impact

A new `docs/lld/gpu-ownership.md`: `GpuContext`, `ComputeCtx`, `Caps`
including tier C, the dependency direction and why it is not a cycle, the
three enforcement mechanisms and what each catches that the others do not.

## Decisions taken in the design round

1. **Option A, activate wgpu now.** The contract becomes `GpuContext` and
   `ComputeCtx<'a>`, enforced by two `trybuild` compile-fail cases and one
   textual guard, none of which needs an adapter. Section 38's argument is
   that this hook is cheap now and a rewrite later, and a hook that is only a
   comment is not a hook. Deviation D-10 is raised.
2. **`Kernel` is declared with no implementers**, following section 31's
   prescribed signature. `AGENTS.md`'s two-implementer rule exists to stop
   invented abstractions and this one is specified, not invented. The plan
   does not invent a first implementer to satisfy the rule, and F-125 supplies
   the real ones. The collision is recorded here rather than resolved
   silently, which is the point of writing it down.

## Open questions

None. Both were resolved above.
