# GPU ownership

**F-IDs that contributed:** F-008
**Last updated:** 2026-09-05

One device, one queue, one owner. HLD section 31's first bullet, made into a
mechanism.

> **Shares the renderer's device.** ocelli-compute never creates a
> wgpu::Device, it borrows the one ocelli-render owns. Two devices cannot share
> textures, which would defeat the entire point.

This is the Phase 1 hook of HLD section 38. Its stated alternative is "a
device-sharing retrofit across the renderer", which is why a contract exists
here before anything uses it.

## The types

| Type | Crate | What it is |
|------|-------|-----------|
| `Tier` | `ocelli-render` | `A` WebGPU, `B` WebGL2 downlevel, `Cpu` under deviation D-07 |
| `Caps` | `ocelli-render` | HLD section 22's struct, field for field, plus the third tier |
| `GpuContext` | `ocelli-render` | The device, the queue and the caps, owned together |
| `ComputeCtx<'a>` | `ocelli-compute` | A borrow of a `GpuContext` and a command encoder, for one dispatch |
| `Kernel` | `ocelli-compute` | HLD section 31's trait, signature as given |
| `ComputeError` | `ocelli-compute` | `Unavailable` and `Workgroup` |

## The dependency direction, and why it is not a cycle

`ocelli-compute` depends on `ocelli-render`. Section 31 fixes it by saying the
renderer owns the device and compute borrows it.

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

### 3. `ci/check-device-ownership.sh`

The weakest of the three, and it catches what the other two cannot: **a crate
creating a device it never puts in a `GpuContext` at all.** No type is involved
in that, so no type can refuse it.

Three assertions, each proved red by mutation:

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
| C, CPU | Not constructible. A tier C session has no device, so it has no `GpuContext` and no `ComputeCtx`. Every kernel resolves through its section 31 fallback or reports unavailable |

`ComputeError::Unavailable` names both the required and the resolved tier,
because "unavailable" without them is a message nobody can act on. Deviation
D-07's rule is unchanged by this story: a feature that cannot run on the
resolved tier reports unavailable and never silently produces a different
answer.

## What this story deliberately does not do

- **It does not create a device.** `GpuContext::new` takes one that already
  exists. Adapter enumeration and tier resolution are F-004, device creation
  and loss recovery are F-039. Doing them here would be a second copy of a
  decision the project wants exactly once.
- **It does not detect `Caps`.** `caps.rs` defines the type because section
  31's `Kernel::workgroup` takes a `&Caps` and a hook expressed in types needs
  the types. Filling it from an adapter is F-004.
- **It supplies no `Kernel` implementer.** The trait is declared with none, and
  `AGENTS.md` forbids that shape. The rule exists to stop invented
  abstractions, and this one is prescribed: HLD Part II says a given signature
  is the intended implementation. The collision was raised in the design plan
  and decided in the sprint's design round rather than resolved quietly.
  F-125 (E31.1) supplies the kernels.

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

`ocelli-render` and `ocelli-compute` link wgpu from F-008 rather than F-039,
and both drop `#![cfg_attr(not(test), no_std)]` because wgpu needs `std`. The
pin is untouched. `scripts/no_std_check.py` reads the attribute from each
crate's source rather than carrying an exemption list, so these two left the
check by construction.

`ocelli-wasm` does not depend on `ocelli-render`, so **the wasm size budget is
unaffected by this story**. The first story that makes the wasm module reach
wgpu is the one that re-baselines it and says why.
