# S07 sprint review, pass 3

**Reviewed**: full repaired sprint history from
`649af7ed05b1c5e42bfa1345f39e15eef07dc735` through
`f0645f9325d713a352ddae4627213e1598f6250b`, plus exact pre-review staged
remediation tree `f45300cb9bc15e072a8ec453102fa3dce5cbe1df`
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, Duplicate DICOM JSON attribute names are silently replaced

**Where**: `crates/ocelli-dicom/src/dicomweb.rs:425` and the pinned
`serde_json` 1.0.151 value deserializer

**What**: `parse_qido_json` first deserializes an entire response into
`serde_json::Value`. The default `Map` is a `BTreeMap`, and its deserializer
uses `insert` for every object member. A second member with the same DICOM tag
therefore replaces the first before `parse_data_set` sees the object.
`MetadataSet::insert` has duplicate refusal, but it cannot refuse a duplicate
that has already disappeared.

**Why it is wrong**: DICOM PS3.18 2026c Annex F models one Attribute object per
Tag key and explicitly distinguishes DICOM JSON from Native DICOM XML by the
JSON model's inability to represent duplicate Tag values. A malformed QIDO
response containing two `00080060` members should reach a safe structural
error. Selecting the last value produces plausible but unauthoritative
metadata and defeats F-017's duplicate-refusing invariant at the network seam.

**Evidence**: `cargo tree -p ocelli-dicom` resolves `serde_json v1.0.151`.
That version's `src/map.rs` defines the default `MapImpl` as `BTreeMap`, while
`src/value/de.rs` inserts each key and value into the map. The current parser
then iterates only that already-collapsed map at lines 437 through 445. A
mutation that changes only the first of two equal Tag keys cannot be observed,
because the later value wins before Ocelli validation runs.

Deserialize DICOM JSON objects through a visitor that detects repeated member
names before materializing a map, including nested sequence data sets. Refuse
the response with a safe structural error. Add a fixture with two identical
Tag keys and different values. Removing the duplicate check must make the
fixture red.

### D2, Dot-segment UIDs can change the requested DICOMweb resource

**Where**: `packages/core/src/dicomweb.ts:74` and
`packages/core/src/dicomweb.ts:216`

**What**: Public QIDO and WADO methods accept UID strings without validating
the DICOM UID grammar. `#resourceUrl` applies `encodeURIComponent` and assigns
the result through `URL.pathname`. The strings `.` and `..` remain dot
segments after encoding, and the URL implementation normalizes them. For
example, `searchSeries("..")` against
`https://example.test/dicomweb` fetches
`https://example.test/dicomweb/series`, not the requested study-scoped series
resource.

**Why it is wrong**: DICOM PS3.5 section 9.1 permits decimal UID components,
not path dot segments. The F-021 plan requires the PS3.18 study, series,
instance, and frame resource shapes and exposes `InvalidRequest` for invalid
caller input. A malformed UID currently changes the endpoint and still issues
a request instead of being refused locally.

**Evidence**: A current compiled-client probe supplied `..` as `studyUid`,
captured the fetch URL, and observed exactly
`https://example.test/dicomweb/series`. The direct platform check also shows
that assigning `/dicomweb/studies/../series` to `URL.pathname` normalizes it to
`/dicomweb/series`. Existing tests cover ordinary UID values but not empty,
dot-segment, or malformed UID input.

Validate each study, series, and instance UID against the DICOM UID grammar
before URL construction, then return `DicomwebError("InvalidRequest")` without
fetching on failure. Cover empty strings, `.`, `..`, non-decimal components,
invalid leading zeroes, and the 64-character limit. Mutating out the check must
make the URL tests red.

### D3, The staged review and delivery records retain false evidence claims

**Where**: `docs/sprints/AS_BUILT.md:2103`,
`docs/sprints/AS_BUILT.md:2213`, and
`.claude/reviews/S07-sprint-review-pass-1.md:4`

**What**: Two append-only completion entries retain obsolete test counts after
the pass-1 repairs. F-021 says eleven Rust DICOMweb fixtures although the
staged suite has twelve. F-023 says twelve registry integration tests although
the staged suite has thirteen. The correction section records other pass-1
repairs but does not correct these counts. Separately, the pass-1 review names
`649af7e7599aa5e6e7762af353ae9e63d81cf7a1` as its base, but that object does
not exist. The actual sprint base is
`649af7ed05b1c5e42bfa1345f39e15eef07dc735`.

**Why it is wrong**: These are the durable audit records for exactly which
history and tests were reviewed and delivered. The current claims cannot be
reproduced as written. The invalid object also prevents a reader from using
the pass-1 range directly with Git.

**Evidence**: The F-021 delivery commit contains eleven `#[test]` cases in
`crates/ocelli-dicom/tests/dicomweb.rs`, while the staged file contains twelve
after the exact transfer-syntax seam repair. The F-023 delivery commit contains
twelve cases in `crates/ocelli-codec/tests/registry.rs`, while the staged file
contains thirteen after the legacy High Bit refusal repair. The pass-2 review
itself reports the resulting twelve and thirteen. `git cat-file` refuses the
pass-1 base written in the review, and `git rev-parse origin/sprint/s07`
returns the base ending in `dc735`.

Preserve AS_BUILT history by appending explicit F-021 and F-023 test-inventory
corrections. Correct the pass-1 review's mistyped base object so its reviewed
range is executable. No implementation change is required for this finding.

## Smells

None.

## Nitpicks

None.

## Verified clean

- All four pass-2 D1 repairs are present. The F-017 plan now names the scoped
  F-021 `NullSlots` seam in both ownership locations. The F-022 backlog status
  is explicitly a design-time snapshot. The `@ocelli/core` index assigns the
  delivered DICOMweb API to F-021 and the later viewer and WASM surface to
  F-100 and F-101. The recreated F-021 commit names D-02 and D-18.
- Commit `392afd527851fa962e40b0dfe17d7b4050445d21` preserves F-021's exact
  implementation tree, carries the corrected deviations text, and has
  ledger-derived `Ocelli-Verify` and `Ocelli-Generated-By: codex` trailers.
  All eight commits in `origin/sprint/s07..HEAD` passed
  `verify_ledger.py check-commit --require-corpus` with matching trees.
- Pass-1 High Bit reconciliation remains exact. The retained 16 Bits
  Allocated, 12 Bits Stored, High Bit 15 signed case is consistently called
  legacy nonconforming and F-023 refuses it with expected High Bit 11. The
  canonical tooling skill and generated adapter match.
- The codec catalogue test requires exact equality with the manifest's 16
  Transfer Syntax UIDs, and every selected UID parses through the F-016 Part
  10 dispatch seam. The F-021 multipart fixture retains selected JPEG-LS UID
  `1.2.840.10008.1.2.4.80` and observes `KnownUnavailable`. Replacing HTJ2K
  `.203` with parser-known MPEG2 `.100` makes the set proof red.
- F-017 preserves exact VR, source spelling, signed widths, sequence order,
  carrier identity, typed null positions, provider identity, and duplicate
  refusal after values reach `MetadataSet`. D1 identifies the earlier JSON
  deserialization loss that bypasses the latter invariant.
- F-021 otherwise keeps network ownership in TypeScript, byte validation in
  Rust, one bulk write, one owned multipart response, borrowed frame ranges,
  exact media-type checks, and safe body-free error messages. D2 identifies
  the uncovered caller-UID routing surface.
- F-022 retains checked header and payload arithmetic, correct qform and sform
  selection, mixed quaternion terms, selected-affine validation, and
  left-multiplied RAS-to-LPS conversion under non-symmetric fixtures.
- F-023 retains exact UID lookup, atomic collision refusal, caller-owned
  buffers, propagated decoder errors, and no concrete codec activation or
  fabricated benchmark result.
- `bin/ocelli.sh test ocelli-codec` passed 2 crate tests and 13 registry
  integration tests. `bin/ocelli.sh test ocelli-dicom` passed 95 tests, with
  only the declared corpus-present test ignored. The focused TypeScript bulk,
  DICOMweb, and package-index suites passed all 23 tests.
- The canonical corpus generator suite passed 50 tests. Manifest validation
  covered 92 rows and all 16 selected transfer syntaxes. The manifest's final
  tab remains the required empty ninth URL field, not stray whitespace.
- Skill synchronization and prose checks passed. The full sprint and staged
  review found no patient data, new unsafe block, out-of-crate `wasm-bindgen`,
  pixel arithmetic, tolerance change, disabled gate, or target portability
  regression.
