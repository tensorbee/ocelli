# Changelog

The section headed by a release tag is that release's notes. `/release-notes`
renders it and the published GitHub release body is compared byte for byte
against a fresh render. See `docs/RELEASE.md`.

## Unreleased

Repository bootstrap. Nothing is published.

### Added

- The Cargo workspace and the thirteen crates of HLD section 15.1, with
  `wasm-bindgen` confined to `ocelli-wasm` and enforced by
  `ci/check-bindgen-isolation.sh`.
- The npm workspaces `@ocelli/core` and `@ocelli/react`, and the example
  viewer at `examples/viewer-react`.
- Strongly typed canvas, world and voxel-index points, composable transforms,
  and pixel-value newtypes in `ocelli-core`.
- A manifest-backed DICOM corpus with deterministic synthetic fixtures,
  transfer-syntax conformance checks, metadata auditing and digest verification.
- The authoritative Markdown specification under `docs/hld/`, sanitized during
  bootstrap so no external source-document bundle is needed by the workflow.
- `docs/sprints/`, with 190 F-IDs imported from the backlog spreadsheet, 13
  added since as `F-X` stories, and 170 of the 203 allocated across 72 sprints
  and 18 milestones.
- The gate set behind `bin/ocelli.sh gate`, and a CI floor that runs every one
  of them that needs no GPU and no corpus. `bin/ocelli.sh gate --list` is the
  list, and this line deliberately does not repeat the count, because a number
  written here is a second list that goes stale the first time a gate is
  added.
- The workflow: `.claude/WORKFLOW.md`, eighteen commands, and generated Codex
  adapters under `.agents/skills/`.
- The wasm build pipeline. `bin/ocelli.sh wasm` produces
  `crates/ocelli-wasm/pkg` through `wasm-pack` under HLD section 15.2's release
  profile, and the `wasm` gate measures that artefact against a recorded size
  budget in `ci/wasm-size-budget.json`. First measurement 14,104 bytes, and
  16,388 after F-005 added the panic hook, with the delta and its cause
  attributed in that file. Both are baselines for regression detection and
  neither is an answer to Appendix A gate A4, whose estimate is a little over
  two orders of magnitude larger, 183x at its low end and 488x at its high one.
- The cross-target build proof, `bin/ocelli.sh native` and the `native` gate.
  It links the `ocelli-desktop` and `ocelli-server` entry points, builds every
  shared crate for both wasm32 and the host, and compares resolved features
  across the two targets against a declared baseline. `ocelli-native` is now a
  compile error under wasm32 rather than a crate that merely should not be
  there.
- The GPU device-sharing contract of HLD section 31. `ocelli-render` owns
  `GpuContext`, holding one device, one queue and the resolved `Caps`, and is
  the only crate permitted to create a device. `ocelli-compute` borrows it
  through `ComputeCtx` and declares the `Kernel` trait. Enforced by the types,
  by compile-fail cases that need no GPU, and by the `device` gate.
- The npm packaging pipeline and the `packages` gate. It proves what a
  consumer receives rather than what compiles: the tarball carries every path
  its exports map advertises and both licence files, a project outside the
  workspace installs and imports it under `bundler` and `node16` resolution,
  and `npm publish --dry-run` exercises the registry path without publishing.
  `@ocelli/core` and `@ocelli/react` now ship a README and their licences.
- The differential oracle's reference half. `bin/ocelli.sh oracle` renders
  every applicable corpus row through cornerstone3D 5.8.2 in headless Chromium
  on SwiftShader and writes reference pixels plus a metadata sidecar, or a
  precise failure at one of four named boundaries. 89 of 91 rows render
  deterministically, and the two that do not are recorded with their reason in
  `tools/oracle/unsupported.json`. It compares nothing yet, which is F-011.
- Runtime tier resolution. `Caps` now has a detection procedure that resolves
  tier A, B or C from an adapter enumeration, a startup fill-rate measurement,
  the reported adapter type and the renderer string, in that order of trust,
  with an operator override through `OCELLI_TIER`. A software rasteriser
  presents a conforming WebGL2 context, so the benchmark decides and the
  strings are only a hint. `ocelli-render` now takes wgpu's `webgl` feature,
  without which tier B could not resolve in a browser at all.
- A stable error model. `ocelli-core` carries a versioned `u16` error code and
  a 32-byte record that fits the event ring's payload exactly, and
  `@ocelli/core` exports the decoder, the human-readable messages and the panic
  reader. A Rust panic is written to a fixed location in linear memory and read
  back by the shell after the trap without calling into the module, because
  `wasm32-unknown-unknown` is `panic = "abort"` in every profile and a trapped
  instance must not be reused.
- Answers to Appendix A gates A1 and A2, in `docs/spikes/`. **JPEG-LS resolves
  to a single pure-Rust decoder on every target.** **HTJ2K does not resolve**:
  `openjp2` does not link for `wasm32-unknown-unknown` and traps on every
  codestream when forced to, so HTJ2K reports unavailable until F-X013 prices a
  route. HLD section 15.2 names `openjp2` as the wasm choice, and that is
  measured not to work.
- A standing probe for every repository guard. `bin/ocelli.sh gate guards`
  drives each probed refusal into its rejected state in a disposable repository
  and requires it to fire, and a probe whose guard exits zero is a failure of
  the harness rather than a pass. The census refuses in both directions, so a
  refusal no entry claims and an entry claiming no refusal both fail. It sorts
  every discovered refusal into four buckets, and the fourth is the refusals
  watched by nothing. **That bucket is not empty**, and the number in it is a
  ratchet that may only go down, so a refusal added without a watcher fails the
  floor. `python3 scripts/guard_census.py` prints the buckets and names each
  uncovered entry, each declared limit and each open hole with its owner. Those
  are entry-level buckets summed over refusal sites rather than a count of
  refusals driven red, which the census says on the line that prints them. Holes
  in existing guards are declared rather than hidden, and a declared hole whose
  probe starts passing also fails, so the record cannot go stale in either
  direction. Two of the four declared in S03 were closed inside the same sprint
  and their declarations went with them, which is that ratchet working.
  `docs/lld/guards.md` is the design.
