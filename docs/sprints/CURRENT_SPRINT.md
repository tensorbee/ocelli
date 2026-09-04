# Current sprint, S02

**Milestone**: M1, foundations and the differential oracle.
**Branch**: `sprint/s02`
**Opened**: 2026-09-04
**Goal**: Build and package every supported target, share one GPU device, and
produce headless cornerstone3D reference renders from the corpus.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-002 | E1.2 | wasm-pack build pipeline with a hard size budget gate | Build | 2w | done |
| F-003 | E1.3 | TS package scaffold, bundling, npm publish pipeline | Build | 2w | done |
| F-007 | E1.7 | Cross-target build proof: native desktop + server binary | Build | 2w | done |
| F-008 | E1.8 | ocelli-compute crate skeleton and GPU device-sharing contract | Build | 2w | done |
| F-010 | E2.2 | Headless cornerstone3D reference renderer | Test | 4w | pending |

## What this sprint is

S02 turns the S01 foundations into executable delivery paths. The same Rust
workspace must build for wasm, desktop and server without target-specific
features quietly changing the core. The TypeScript packages must consume the
wasm boundary through a reproducible bundle and publish pipeline. In parallel,
the first oracle stage must render S01's corpus through the pinned
cornerstone3D reference before any port implementation begins.

F-008 is the Phase 1 hook from HLD section 38. It establishes that compute and
rendering share one GPU device and queue. A second device hidden behind a
convenient wrapper would make later zero-copy integration impossible.

## What is carried in

Nothing is carried in. S01 closed F-001 and F-009, and every S02 dependency is
therefore done. The wasm and oracle gates were named bootstrap skips in S01.
This sprint owns the stories that remove both skips.

## The defect class this sprint is exposed to

The dangerous build defect is false portability. A workspace can compile on
one target while Cargo feature unification enables `std`, browser-only bindings
or a second GPU pathway on another. F-002 and F-007 must prove the actual
target builds, not infer portability from a host build. The size budget is
measured only from a release wasm artefact.

The dangerous oracle defect is false reference output. A headless page can
start, load a test runner and exit successfully without decoding every corpus
row, presenting a frame or reading back the rendered pixels. F-010 is complete
only when failures at each of those boundaries are visible and the output is
tied to the pinned cornerstone3D version and manifest digest.

The dangerous GPU defect is accidental device duplication. F-008 must make the
shared device and queue contract explicit before compute kernels or render
pipelines can acquire their own instances.

## What done means

- F-002 produces the release wasm package through `wasm-pack` and enforces the
  HLD size budget against that release output.
- F-003 builds the tracked TypeScript workspaces and proves the package and
  publish pipeline without publishing a release.
- F-007 compiles the declared native desktop and server entry points using the
  same core crates as wasm.
- F-008 records and tests the single-device ownership contract used by compute
  and rendering.
- F-010 renders every applicable corpus row through the pinned reference stack
  in a headless browser and emits deterministic reference pixels or a precise
  failure. It does not yet compare Ocelli output, which belongs to F-011.

## Dependency order

All declared dependencies are complete. F-002, F-003, F-007 and F-008 may be
designed from F-001. F-010 may be designed from F-009. Implementation waves
must still serialize edits to shared manifests, package configuration, sprint
ledgers and the GPU.

## Standing expectations

Read the tracked Markdown under `docs/hld/` before implementation. It is the
normative source. Record an implementation departure in
`docs/hld/DEVIATIONS.md` rather than changing a gate or tolerance to make a
check pass.

Every new guard is observed red before it is claimed. Every skipped corpus row,
missing browser frame and missing target build is a failure unless the approved
design names a narrower scope.
