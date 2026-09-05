# F-002 implementation review, pass 2

**Scope**: the F-002 working diff including pass 1's two remediations, which
were themselves unreviewed work until this pass.
**Result**: 1 defect, 0 smells, 1 nitpick. The defect is remediated.

## Defects

### D3. The wasm-opt flag list asserted something about rustc that had not been checked

`crates/ocelli-wasm/Cargo.toml` carried six `--enable-*` flags with the
comment "rustc for wasm32-unknown-unknown enables these WebAssembly proposals
by default". Only one of them, bulk-memory, appeared in an observed error. The
other five were carried over from familiarity, which is exactly the shape HLD
27.2 R4 warns about for wgpu: an agent emitting a plausible list from memory.

The claim turns out to be **true**, and that is not the point. It was true by
luck until it was checked. `rustc --print cfg --target wasm32-unknown-unknown`
reports precisely six default target features:

```
bulk-memory  multivalue  mutable-globals  nontrapping-fptoint
reference-types  sign-ext
```

which map one to one onto the six flags, with `nontrapping-fptoint` written in
wasm-opt's longer spelling.

**Remediation.** The comment now names the command it came from, so the next
person can re-derive the list instead of trusting it, and says plainly that
only `--enable-bulk-memory` is required to build today. Measured: with the
list trimmed to `["-O", "--enable-bulk-memory"]` the build still succeeds,
because this module is one function. The other five are kept for the module
F-096 and the render stories will produce, and the comment says that rather
than implying they are all load-bearing now.

The artefact is unchanged by the correction: 14,104 bytes before and after,
so the recorded baseline still describes the build that produced it.

## Nitpicks

### N1. A missing `wasm-pack` now fails the floor rather than skipping it

Removing the gate's skip means a developer without `wasm-pack` gets a red
`gate --floor` instead of a named skip. This is deliberate and is recorded
here rather than left to be rediscovered. `wasm-pack` is a documented
prerequisite in `docs/DEVELOPER_SETUP.md`, and the project's own rule is that
a skipped gate is not a pass. Treating an absent documented prerequisite as a
failure is the same choice `corpus_tests.py --require-prerequisites` already
makes in CI. Does not block.

## What was checked and found clean

- Pass 1's two remediations re-read in full. The isolation script's typo guard
  and the corrected doc comment both do what they now claim.
- `bin/ocelli.sh`: the comment block sits between `case` arms, which is valid,
  and the arm is still chained on `&&` so the build's status cannot be masked
  by the size check's.
- `.github/workflows/ci.yml`: the stale skip comment is gone and the job still
  goes through the gate runner rather than calling wasm-pack directly, so CI
  and a developer get one definition.
- `Cargo.toml`: the comment naming F-008 rather than F-039 as wgpu's
  activating story matches deviation D-10, which is already committed to
  `docs/hld/DEVIATIONS.md`.
- `ci/wasm-size-budget.json` is committed, which it must be for CI to read it.
