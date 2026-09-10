# S07 sprint review, pass 2

**Reviewed**: full repaired sprint history from
`649af7ed05b1c5e42bfa1345f39e15eef07dc735` through
`bb05022b3027c1178a02a3b3e63a7c1c87bf9d1f`, plus exact staged remediation
tree `574d9c0cabcc05251231662ad1b971625f263911`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Four retained S07 claims still contradict the delivered state

**Where**: `.claude/plans/F-017-design.md:108`,
`.claude/plans/F-017-design.md:234`, `.claude/plans/F-022-design.md:225`,
`packages/core/src/index.ts:18`, and commit
`482ce5c7fb8845bad494bf2f4e4fe8a105f0d065`

**What**: The pass-1 documentation repair did not reach four remaining
claims.

- The F-017 plan says F-021 can construct its metadata representation without
  editing F-017 files, then says F-021 may not edit `metadata.rs`. The approved
  and delivered F-021 `NullSlots` seam edited that exact file and its metadata
  fixture.
- The F-022 plan says the backlog status is pending in the present tense. The
  authoritative backlog and active sprint both say done. The sentence is
  useful source-preflight evidence only if it identifies itself as a
  design-time snapshot.
- The current `@ocelli/core` index calls the package a scaffold whose public
  API waits for F-100 immediately before exporting F-021's public
  `DicomwebClient`, response sink, and error surface. F-100 owns the later
  session-facing viewer API, not all public API.
- The F-021 delivery commit says `Deviations, none`, while its approved plan
  and the pass-1 AS_BUILT correction both say existing D-02 and D-18 apply.
  The other S07 feature commits name their applicable existing deviations.

**Why it is wrong**: The microscope workflow classifies a false prose claim as
a defect. These files and the commit message are the durable design,
ownership, public-surface, and provenance record for later work. A reader
currently receives mutually incompatible answers about who edited the shared
metadata implementation, whether F-022 is pending, whether F-021 publishes an
API, and which deviations govern F-021.

**Evidence**: `git diff origin/sprint/s07..HEAD` shows F-021's delivered
`MetadataValue::WithNullSlots` implementation in `metadata.rs` and its
independent fixture in `tests/metadata.rs`. `CURRENT_SPRINT.md` and
`BACKLOG.md` both report F-022 done. `packages/core/src/index.ts` exports
`DicomwebClient`, `DicomwebError`, `writeDicomwebResponse`, and their public
types, and the exact runtime export test passes. `git show -s --format=%B
482ce5c7fb8845bad494bf2f4e4fe8a105f0d065` reports `Deviations, none`, while
the approved F-021 plan at lines 305 through 307 applies D-02 and D-18.

Update the two F-017 plan assertions with the already approved scoped F-021
seam exception. Mark the F-022 backlog sentence as a design-time snapshot.
Clarify the index comment so F-100 owns the session-facing viewer API while
the current F-021 public surface remains acknowledged. Recreate the F-021
commit message with D-02 and D-18 through the already approved recoverable
history-repair procedure. Preserve its exact tree and its ledger-derived
trailers. Do not write provenance trailers by hand.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-1 D1 is repaired. The retained High Bit 15 generator case, manifest
  category, corpus guide, corpus LLD, tests, and canonical tooling skill all
  label it legacy nonconforming. The exact 16 Bits Allocated, 12 Bits Stored,
  High Bit 15 signed descriptor is refused by F-023 with expected High Bit 11.
  The generated Codex skill adapter matches its canonical source.
- Pass-1 D2 is repaired. The Rust test requires the 16-UID codec catalogue to
  equal the manifest's transfer-syntax set and parses a synthetic Part 10
  object under every selected UID through F-016. The F-021 frame fixture
  retains selected JPEG-LS UID `1.2.840.10008.1.2.4.80` and observes
  `KnownUnavailable` from F-023. Replacing Rust `.203` with parser-known `.100`
  makes the direct set comparison unequal while the Python corpus catalogue
  independently continues to require `.203`.
- Pass-1 D3 is repaired. Each of the eight commits in
  `origin/sprint/s07..HEAD` passed `verify_ledger.py check-commit
  --require-corpus`. Every trailer names `codex`, reports corpus pass, and
  matches the commit's exact tree after the approved local history
  reconstruction.
- Pass-1 D4's named remediation is present. The F-021 plan now acknowledges
  its scoped metadata seam. The TypeScript packaging LLD lists F-021 and the
  delivered DICOMweb module and exports. The build-target LLD lists F-017 and
  direct `dicom-core`. Append-only AS_BUILT corrections record F-017's omitted
  metadata unit test and F-021's applicable D-02 and D-18.
- The changed manifest row still has a required empty ninth `url` field.
  `corpus_check.load()` uses literal tab splitting and requires exactly nine
  fields, so the final tab is the truthful existing encoding of that empty
  field. Removing it produces eight fields. Replacing it with text invents a
  fetch URL, and quoting is not part of this parser's contract. The
  `git diff --cached --check` trailing-whitespace warning is therefore not an
  actionable defect and no gate was weakened.
- `bin/ocelli.sh test ocelli-codec` passed 2 crate tests and 13 registry
  integration tests. The legacy descriptor refusal is among them.
- `bin/ocelli.sh test ocelli-dicom` passed 95 tests with only the declared
  corpus-present test ignored. Its 12 DICOMweb tests include the exact
  catalogue, F-016 dispatch, and F-021 capability seams.
- The canonical corpus generator suite passed 50 tests. Manifest-only shape
  validation reported 92 rows, and coverage reported all 16 selected transfer
  syntaxes with both tolerance classes represented.
- The focused TypeScript bulk, DICOMweb, and index suites passed all 23 tests.
  Type checking and lint passed.
- Skill synchronization, prose, backlog, and staged-content gates passed. No
  patient data, new unsafe block, out-of-crate `wasm-bindgen`, pixel
  arithmetic, tolerance change, or disabled gate was found in the full sprint
  diff or staged remediation.
- F-017 still preserves exact VR, source spelling, signed widths, ordered
  sequence items, carrier identity, typed null positions, and observable
  provider identity. F-021 keeps network ownership in TypeScript, response
  parsing in Rust, one bulk write, structural safe errors, and borrowed frame
  ranges over one owned response.
- F-022's checked header and payload arithmetic, qform and sform selection,
  mixed quaternion terms, selected-affine validation, and left-multiplied
  RAS-to-LPS conversion remain covered by non-symmetric fixtures. F-023 still
  uses exact UID lookup, atomic collision refusal, caller-owned decode output,
  and no concrete codec or fabricated benchmark result.
