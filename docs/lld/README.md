# Low-level design

Living current-state documents, one per area, updated by `/complete-feature`
step 9 from the design plan's `## LLD impact` list.

**These describe what the code does today, not how it got there.** A changelog
section in an LLD file is a defect: the history is in `AS_BUILT.md` and in
`git log`, and mixing the two produces a document nobody trusts as either.

Each file carries a `**F-IDs that contributed:**` line and a
`**Last updated:**` date. Both are maintained mechanically at completion.

| File | Area | F-IDs |
|------|------|-------|
| [core-types.md](core-types.md) | `crates/ocelli-core`, the coordinate and value spaces | F-001 |
| [corpus.md](corpus.md) | Golden corpus layout, generation and verification | F-009 |
| [build-targets.md](build-targets.md) | The wasm pipeline, the size budget, the cross-target proof and the isolation invariant | F-002, F-007, F-008 |
| [gpu-ownership.md](gpu-ownership.md) | One device, one queue, one owner. The section 31 contract | F-008 |
| [typescript-packaging.md](typescript-packaging.md) | What the npm packages publish, and what proves it | F-003 |
