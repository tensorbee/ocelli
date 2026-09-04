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
