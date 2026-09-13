# S08 sprint review, pass 2

**Reviewed**: complete rewritten sprint history from
`7025d6fe66342bb3933ab3e17038ff93bfd7389b` through
`64457f366174c2bbe1a3335d584a1ed5996cd1b1`, exact pre-report staged tree
`f5693ca953169afcf363f3aaafb53689a52e3f37`
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Pass-one remediations proved

- **D1, manifest-backed F-019 projection.** The corrected conformance row
  names `crates/ocelli-dicom/tests/corpus.rs` and `gate corpus`. The permanent
  ignored test finds `synthetic/ct_multiframe_perframe.dcm` in the tracked
  manifest, parses the Part 10 object, constructs `MetadataSet` and
  `MultiframeMetadata`, checks the declared three frames, and distinguishes
  `Shared` Pixel Measures from `PerFrame(1)` VOI evidence without asserting or
  logging attribute values. The exact test exited 0, and `gate corpus` exited
  0 over 92 verified rows and all three ignored corpus tests.
- **D2, truthful native and wasm evidence.** The corrected plan names
  `gate native` plus the direct library command
  `cargo check -p ocelli-dicom --lib --target wasm32-unknown-unknown`.
  The direct command exited 0. `gate native` exited 0 through all eight steps,
  including native and wasm compilation, equal features for 18 direct
  dependencies, native JPEG 2000 execution, plain and SIMD wasm builds, and
  both wasm modules executing under Node.
- **D3, complete delivery state.** BACKLOG marks F-018, F-019, F-024, F-025,
  and F-026 done. AS_BUILT and SPRINT_TRACKER contain one completion entry for
  each, and CHANGELOG names the added pixel, multiframe, JPEG, native/RLE, and
  JPEG 2000 surfaces. The ignored S08 run state marks all five features
  `completed`. Each stored feature head is an ancestor of HEAD and resolves to
  its rewritten integration commit. `close-preflight S08` now reports only the
  expected stale pass-one review with five defects. It reports no incomplete
  or carried-feature failure.
- **D4, provenance rewrite.** The rewritten design commit is
  `b6569b8ca97942eb566545bbb78d4c96df397f2a`. The old `586e503` is not an
  ancestor of HEAD and the replacement is. Every one of the seven sprint
  commits from `6370a309` through `9d0d185c` passed
  `verify_ledger.py check-commit --require-corpus` with matching tree and
  corpus evidence. The old F-026 integration tree and rewritten F-026
  integration tree are both
  `fd7a6483a82427f839bfd5e096b3d8167e1699b9`, proving that the provenance
  rewrite did not change the integrated feature tree. Historical review
  reports retain the hashes they actually reviewed. The active run-state head
  mappings use the rewritten hashes.
- **D5, corrected sprint codec contract.** CURRENT_SPRINT now says that F-025
  registers native and RLE frame paths while Deflated Explicit VR Little
  Endian remains at the whole-data-set ingest boundary. CURRENT_SPRINT and
  BACKLOG name the approved `ritk-codecs` replacement for F-026. The codec
  tests exited 0, including the permanent Deflate ownership assertion. Both
  native and wasm `ocelli-codec` dependency trees exited 0 and contained no
  active Rayon dependency.

## Fresh mutation evidence

The pass changed the production `NUMBER_OF_FRAMES` tag in
`crates/ocelli-dicom/src/multiframe.rs` from `(0028,0008)` to `(0028,0009)`.
The exact manifest-backed F-019 corpus test exited 101 with
`F-019 multiframe structure is invalid`. This is distinct from pass one's
native-frame offset arithmetic mutation and proves that the new real-corpus
projection reaches the production frame-count lookup. The tag was restored
exactly, the same test exited 0, and `git diff --quiet` then exited 0.

## Verified clean

- `git write-tree` produced the requested pre-report identity
  `f5693ca953169afcf363f3aaafb53689a52e3f37`. `git diff --quiet` and
  `git diff --cached --check` exited 0 before the report. The full sprint diff
  spans 205 tracked paths, including all plans, reviews, source, fixtures,
  vendor evidence, guards, benchmarks, LLDs, and delivery records.
- `verify_ledger.py assert` exited 0 for the exact pre-report tree and found a
  sprint-profile record with corpus pass and all 30 declared gates. The HEAD
  commit carries the matching `Ocelli-Verify` tree prefix and names `codex` as
  generator.
- `test ocelli-pixel`, `test ocelli-dicom`, and `test ocelli-codec` all exited
  0. Their permanent suites exercised F-018 geometry, stored extraction and
  LUT arithmetic, F-019 functional-group precedence and both frame indexes,
  F-024 JPEG processes and padding, F-025 native and RLE layouts plus Deflate
  ownership, and F-026 JPEG 2000 marker, quantization, conversion, corpus, and
  registration contracts.
- The new AS_BUILT test counts agree with the executed suites. F-018 has ten
  module tests and sixteen integration tests. F-019 has six synthetic
  multiframe tests, nine frame-index tests, and its new ignored corpus test.
  F-024 has ten JPEG integration tests plus registry and benchmark evidence.
- `gate bench` exited 0 with 12 subjects, three baselines, 27 Python tests, and
  97 Node tests where 96 passed and the browser-only case was the declared
  skip. The benchmark records and tests still bind the production JPEG and
  JPEG 2000 subjects and their measurement provenance.
- `gate pins` and all 14 pin-and-size tests exited 0. The vendor tree remains
  78 tracked files. Its exact inventory, manifests, licences, provenance root,
  workspace exclusion, wasm size, and native plus wasm no-Rayon graphs remain
  enforced.
- The guard-catalogue integration repair preserves the existing vendor
  exclusion while adding the disposable named-member exclusion as one valid
  TOML key. Its focused unit test exited 0. The exact deep
  `lint-policy.excluded-named-member` probe drove its intended guard red while
  its control remained green and exited 0.
- `gate backlog`, `gate deviations`, `gate unsafe`, `gate provenance`,
  `gate prose`, and `gate content` exited 0. The checks found 211 backlog
  F-IDs with 40 done, 21 resolved deviations, no unsafe outside the two
  permitted files, clean provenance over 715 files, clean prose over 272
  files, and no patient data or build artefact staged.
- The corrected records introduce no geometry interpretation from F-020, no
  HTJ2K or JPEG-LS implementation from F-027 or F-028, no second LUT chain,
  no extra decoder trait, no out-of-crate `wasm-bindgen`, and no tolerance
  change. Cross-story sample-layout, colour ownership, exact UID dispatch,
  caller-buffer atomicity, and corpus boundaries remain consistent.
