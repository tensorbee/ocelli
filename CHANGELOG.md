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
- `docs/hld/`, cut from the authored `.docx` by `scripts/split_hld.py` with a
  check that asserts no section is lost. The authored document and Appendix C,
  the commercial analysis, are held outside the repository.
- `docs/sprints/`, with 190 F-IDs imported from the backlog spreadsheet and
  157 of them allocated across 72 sprints and 18 milestones.
- The gate set behind `bin/ocelli.sh gate`, and a CI floor that runs every one
  of them that needs no GPU and no corpus. `bin/ocelli.sh gate --list` is the
  list, and this line deliberately does not repeat the count, because a number
  written here is a second list that goes stale the first time a gate is
  added.
- The workflow: `.claude/WORKFLOW.md`, eighteen commands, and generated Codex
  adapters under `.agents/skills/`.
