# Low-level design

Living current-state documents, one per area, updated by `/complete-feature`
step 9 from the design plan's `## LLD impact` list.

**These describe what the code does today, not how it got there.** A changelog
section in an LLD file is a defect: the history is in `AS_BUILT.md` and in
`git log`, and mixing the two produces a document nobody trusts as either.

Each file carries a `**F-IDs that contributed:**` line and a
`**Last updated:**` date, and so does the F-IDs column of the index below.
**Nothing maintains any of the three mechanically.** This paragraph used to say
they were maintained mechanically at completion, which was a claim about a
check that does not exist: `/complete-feature` step 9 asks the agent closing the
story to write them, no gate reads them, and by the S03 review's seventh pass
the index row for `corpus.md` had drifted from the file's own line, the one
mismatch of eleven rows. If you edit an LLD file, edit its header line and its
row here in the same hand.

| File | Area | F-IDs |
|------|------|-------|
| [core-types.md](core-types.md) | `crates/ocelli-core`, the coordinate and value spaces | F-001, F-005 |
| [dicom-ingest.md](dicom-ingest.md) | Part 10 parsing, DICOMweb source responses, lossless metadata providers, multiframe projection, frame indexing, transfer-syntax dispatch and refusal boundaries | F-016, F-017, F-019, F-021 |
| [pixel-pipeline.md](pixel-pipeline.md) | Image-plane evidence, stored-value extraction, modality and VOI mapping | F-018 |
| [dicomweb.md](dicomweb.md) | TypeScript DICOMweb transport and the pure Rust response contract | F-021 |
| [nifti-ingest.md](nifti-ingest.md) | NIfTI-1.1 header, affine, coordinate conversion, and payload validation | F-022 |
| [codecs.md](codecs.md) | Explicit decoder registration, capability, exact Transfer Syntax UID dispatch, and JPEG decode | F-023, F-024 |
| [corpus.md](corpus.md) | Golden corpus layout, generation and verification | F-009, F-013, F-014, F-016, F-X006, F-X007, F-X012, F-X013 |
| [build-targets.md](build-targets.md) | The wasm pipeline, the size budget, the cross-target proof and the isolation invariant | F-002, F-004, F-005, F-007, F-008, F-016, F-017, F-021, F-022, F-023, F-X008 |
| [gpu-ownership.md](gpu-ownership.md) | One device, one queue, one owner. The section 31 contract | F-004, F-005, F-008, F-X001 |
| [tier-resolution.md](tier-resolution.md) | How a session resolves tier A, B or C, the fill-rate probe and the operator override | F-004, F-X001, F-X016 |
| [feature-availability.md](feature-availability.md) | The three-state contract every tier-gated feature declares | F-X001 |
| [errors.md](errors.md) | The error model, the code registry, the panic record and structured logging | F-005, F-X001, F-X017 |
| [typescript-packaging.md](typescript-packaging.md) | What the npm packages publish, and what proves it | F-003, F-004, F-005, F-021, F-X017 |
| [oracle.md](oracle.md) | The differential harness's reference half, cornerstone3D under headless Chromium | F-010, F-012, F-013, F-014, F-X006, F-X007, F-X008, F-X009, F-X012, F-X013 |
| [comparator.md](comparator.md) | The harness's judging half: the tolerance predicate, the verdict vocabulary, the attribution ladder and the census | F-011, F-013, F-015, F-X012 |
| [benchmarks.md](benchmarks.md) | The benchmark harness of HLD section 26, its subject registry, the host class and the `bench` gate | F-006, F-023, F-024, F-X014 |
| [guards.md](guards.md) | The guard harness: discovery, the catalogue, the sandbox, the probe runner, the census and executable skill examples | F-X008, F-X009, F-X010, F-X014, F-X015, F-X018, F-X019, F-X020 |
