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
- `docs/sprints/`, with 190 F-IDs imported from the backlog spreadsheet and
  157 of them allocated across 72 sprints and 18 milestones.
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
  budget in `ci/wasm-size-budget.json`. The module exports `ocelli_version()`
  and nothing else until the boundary lands. First measurement 14,104 bytes,
  which is a baseline for regression detection and not an answer to Appendix A
  gate A4.
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
