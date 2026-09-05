# F-008 implementation review, pass 3

**Scope**: pass 2's two remediations, and a final read of the whole F-008 diff.
**Result**: 0 defects, 0 smells, 0 nitpicks. **Clean.**

## Pass 2's remediations, re-read

The `Clone` grep in `ci/check-device-ownership.sh` uses `awk` to pair the
nearest preceding `#[derive(...)]` with `pub struct GpuContext`, so it reads the
derive that actually applies to that type rather than any derive in the file.
Proved red by adding `Clone` to the struct's own derive, and green after.

The tautological test is gone. `crates/ocelli-render/src/gpu.rs` now has one
test in that area, `wgpu_device_is_a_clonable_handle`, which asserts a fact
about wgpu rather than about this code, and which is the reason the contract is
written the way it is.

## Final read

| Plan step | Landed | Evidence |
|-----------|--------|----------|
| 1, `GpuContext` in `ocelli-render`, the only device holder | yes | `gate device` green, three failure paths proved |
| 2, `ComputeCtx<'a>` borrowing, in `ocelli-compute` | yes | `E0515` compile-fail case |
| 3, `Kernel` declared with section 31's signature | yes | compared character by character against the specification |
| 4, tested without a GPU, three ways | yes | two trybuild cases plus the guard, all in the CI floor |
| 5, prove each goes red | yes | six mutations across the three passes |

The design round's two decisions both landed as written: option A, wgpu
activated with deviation D-10, and `Kernel` declared with no implementers and
the `AGENTS.md` collision recorded rather than resolved quietly.

## The six mutations, collected

| Mutation | Expected | Observed |
|----------|----------|----------|
| `into_device` added to `GpuContext` | trybuild red | `Expected test case to fail to compile, but it succeeded` |
| `request_device` called in `ocelli-compute` | `gate device` red | `crates/ocelli-compute/src/lib.rs creates a GPU device or surface` |
| `GpuContext` renamed away | `gate device` red | `ocelli-render no longer defines GpuContext` |
| An owned-device accessor added | `gate device` red | `GpuContext has an accessor that hands out an owned device` |
| `Clone` added to `GpuContext`'s derive | `gate device` red | `GpuContext derives Clone` |
| `glam` given a wasm32-only feature | `gate native` step 4 red | `host ['libm']  wasm32 ['libm', 'scalar-math']` |

## The pattern worth carrying out of this story

**Three tests that could not fail appeared in one story**, and two of the three
appeared while fixing something else. The compile-fail case that compiled, and
the tautological `Clone` assertion written during the remediation for it.

The common condition is attention being on the fix rather than on whether the
new assertion discriminates. The cheap defence is the one this project already
mandates and which caught all three: mutate, and watch it go red, **including
for a test written during a remediation**.
