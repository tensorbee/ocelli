# F-008 implementation review, pass 1

**Scope**: the working diff for F-008 (E1.8), ocelli-compute crate skeleton and
GPU device-sharing contract.
**Result**: 3 defects, 0 smells, 0 nitpicks. All three remediated.

Two of the three were found by gates rather than by reading, which is the
system working. The third was found by the test harness refusing to accept a
test that proved nothing.

## Defects

### D1. A compile-fail case that compiled

`crates/ocelli-compute/tests/ui/context_cannot_outlive_its_borrow.rs` built its
values with `unimplemented!()`. A diverging initialiser makes everything after
it unreachable, so the borrow checker never ran and the case **compiled**.
trybuild reported `Expected test case to fail to compile, but it succeeded`.

This is the worst defect shape this project has: a test that is counted as
coverage forever and can never fail. It is the same shape three F-001 review
findings had, per that story's AS_BUILT note.

**Remediation.** Both cases take their values as parameters, so no diverging
expression precedes the line under test. They now produce `E0515` and `E0599`.
Proved by mutation: adding `into_device` to `GpuContext` makes the suite report
`Expected test case to fail to compile, but it succeeded`.

### D2. Activating wgpu broke the wasm-bindgen isolation check

`gate bindgen` went red with `ocelli-compute reaches wasm-bindgen under
wasm32-unknown-unknown` and the same for `ocelli-render`.

**This is a contradiction inside the HLD, not a mistake in the story.** Section
15.2 specifies wgpu, section 4 says `ocelli-render` builds for wasm, and
section 15.3 forbids any crate but `ocelli-wasm` from reaching wasm-bindgen. On
wasm32 all three cannot hold, because wgpu talks to the browser's WebGPU
through js-sys and web-sys. The route was traced rather than assumed:

```text
wasm-bindgen v0.2.127
|-- js-sys -> wasm-bindgen-futures -> wgpu -> ocelli-render
`-- web-sys -> wgpu -> ocelli-render
```

and `cargo tree --invert` on the host reports `nothing to print`, so no such
route exists there.

**Remediation.** Deviation **D-12**. The host loop is section 15.3's, character
for character, unchanged. The wasm32 pass became a **direct declaration** check
over each crate's own manifest. D2's purpose is untouched: `ocelli-render`
carries no browser binding in its source and compiles for native unchanged,
which is the property that makes the desktop and server targets entry points
rather than rewrites. Proved still red on both a plain and a wasm32-gated
direct declaration.

### D3. F-007's feature guard was machine-specific, and only wgpu could show it

`gate native` step 4 reported 42 findings, 32 of them packages present on one
target only, and every one legitimate. The serious part is that most were
**specific to the machine**: `objc2-metal` and `raw-window-metal` are macOS
host-only, where a Linux CI runner reports `ash` and `gpu-alloc`. The baseline
would have been correct on one laptop and red in CI, and the fix for a red CI
would have been to re-declare it.

That is tolerance-tuning wearing a different hat, and section 25.1 says exactly
what it does to a suite.

**Remediation, and it edits F-007's landed work.** The check now makes one
claim: every dependency `[workspace.dependencies]` names directly resolves the
same features on both targets. The transitive closure of a cross-platform GPU
library differs per target by design, and asserting otherwise measures wgpu
rather than this project. Proved still red by giving `glam` an extra feature
under a wasm32 target gate.

**A parsing bug went with it.** `cargo tree` marks an already-printed subtree
with a trailing ` (*)`, which lands after the `--format` string and so arrives
inside the feature field, turning `default` into `default (*)`. It produced
four phantom differences between a package and itself. Stripped before the
split.

## What was checked and found clean

- Every `as` cast: none in this diff.
- Arithmetic: none. No pixel and no coordinate, so HLD 27.2 R3 does not apply.
- `unsafe`: none. `gate unsafe` green.
- `unwrap`, `expect`, `panic`: none.
- **Section 31's `Kernel` signature against the specification, character by
  character.** Matches on all three:

  ```rust
  fn tier(&self) -> Tier;
  fn workgroup(&self, caps: &Caps) -> [u32; 3];
  fn dispatch(&self, ctx: &mut ComputeCtx) -> Result<(), ComputeError>;
  ```
- **Section 22's `Caps` fields against the specification.** `compute: bool`,
  `max_tex_3d: u32`, `max_buffer: u64`, `tier: Tier`. Matches, plus `Tier::Cpu`
  from D-07.
- `GpuContext` has no `Clone` and no owned-value accessor. Both halves matter:
  a `Clone` would let a second handle exist without any accessor being added.
- The dependency direction is compute on render, which section 31 fixes, and it
  is not a cycle because the renderer consumes results rather than calling
  kernels.
