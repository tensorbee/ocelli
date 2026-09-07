# S06 sprint review, pass 5

**Reviewed**: full sprint diff from
`baaa92146ebd7844cca086c957f1d1015a1eec9f` plus staged remediation at tree
`bf9e4110b921b6bb2676fb2455b849541275395e`
**Result**: 4 defects, 0 smells, 0 nitpicks

## Defects

### D1, The top-level encapsulation rule rejects valid nested native Pixel Data

**Where**: `crates/ocelli-dicom/src/parse.rs:357`

**What**: Every defined-length Pixel Data element is rejected when the
selected transfer syntax is encapsulated. The check does not test whether the
element is in the top-level Data Set. It therefore rejects Native Format Pixel
Data nested in a Sequence Item, including an Icon Image Sequence.

**Why it is wrong**: DICOM PS3.5 section A.4 requires Pixel Data to use
Encapsulated Format when it is present in the top-level Data Set. The section
expressly distinguishes nested Pixel Data, which may use Native Format with an
Explicit Value Length. The transfer syntax rule is scoped to top level and
must not be applied to a nested icon image.

**Evidence**: A standalone synthetic JPEG Baseline Part 10 fixture encoded an
Icon Image Sequence `(0088,0200)` with one defined-length Item containing
defined-length OB Pixel Data `(7FE0,0010)`. The staged parser returned
`InvalidDataSet`. The same structural path is rejected by the unconditional
`encapsulated && header.tag == PIXEL_DATA` term.

Limit the native Pixel Data refusal to `containers.is_empty()`. Add a positive
encapsulated-transfer-syntax fixture with Native Format Pixel Data inside an
Icon Image Sequence while retaining the existing top-level negative test.

### D2, Encapsulated Pixel Data accepts VR OW instead of requiring OB

**Where**: `crates/ocelli-dicom/src/parse.rs:377`

**What**: The lazy reader emits `PixelSequenceStart` for tag `(7FE0,0010)` with
Undefined Length without considering its Value Representation. Ocelli accepts
that token based only on the transfer syntax and container context. A complete
encapsulated Pixel Data value encoded with VR OW therefore returns `Ok`.

**Why it is wrong**: DICOM PS3.5 section A.4 requires Encapsulated Format Pixel
Data to use VR OB and Undefined Length. OW is permitted for some Native Format
Pixel Data, not for the encapsulated representation.

**Evidence**: A synthetic JPEG Baseline fixture contained top-level Pixel Data
with VR OW, Undefined Length, an empty Basic Offset Table, one four-byte
Fragment, and a valid Sequence Delimitation Item. `parse_part10` returned
`Ok`. The otherwise identical OB fixture is valid.

Validate the raw VR bytes of a `PixelSequenceStart`, or retain the preceding
header information in a form that exposes the VR. Add OW and SQ negative
fixtures beside the valid OB case.

### D3, Encapsulated Pixel Data accepts no Fragment and an empty Fragment

**Where**: `crates/ocelli-dicom/src/parse.rs:392` and
`crates/ocelli-dicom/src/parse.rs:421`

**What**: `has_basic_offset_table` becomes true after the first Pixel Item and
then permits Sequence end immediately. The same state permits every subsequent
defined-length Item, including length zero. As a result, both a Basic Offset
Table followed by no Fragment and a zero-length Fragment are accepted.

**Why it is wrong**: DICOM PS3.5 section A.4 defines the encapsulated Pixel
Data Stream as one or more Fragment Items after the Basic Offset Table. Each
Fragment Item has an even Item Value Length greater than or equal to two. The
Basic Offset Table is not itself a Fragment.

**Evidence**: Both of these complete JPEG Baseline fixtures returned `Ok`:

- Pixel Data, an empty Basic Offset Table Item, then the Sequence Delimitation
  Item with no Fragment Item.
- Pixel Data, an empty Basic Offset Table Item, one Item of length zero, then
  the Sequence Delimitation Item.

Track whether at least one Fragment has completed separately from whether the
Basic Offset Table has completed. Require each post-table Item length to be
defined, even, and at least two. Accept Sequence end only after a Fragment.
Add standing tests for both malformed forms and a positive two-byte Fragment.

### D4, The committed append-only AS_BUILT entry was rewritten

**Where**: `docs/sprints/AS_BUILT.md:2055`

**What**: The remediation edits the already committed F-016 completion entry
to add direct flate2 validation and change the recorded Part 10 test count from
nineteen to thirty-four.

**Why it is wrong**: `docs/sprints/AS_BUILT.md` lines 3 through 6 and
`.claude/commands/complete-feature.md` require this record to be append-only.
A correction must be a new follow-up entry because the log preserves what was
believed at completion. Rewriting the old entry destroys that history.

**Evidence**: `git diff 43173e9 -- docs/sprints/AS_BUILT.md` shows replacements
inside the F-016 entry committed by `43173e9`. The file itself says never to
edit a prior entry.

Restore the committed F-016 entry byte for byte. Append a correction entry at
the bottom that records the new direct dependency, sprint-review remediation,
current test count, and the verification tree that supports those new facts.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-4 D1 is closed for top-level representation mismatch. Explicit VR
  Little Endian now refuses fragmented Pixel Data, and JPEG Baseline refuses
  defined-length top-level Pixel Data. The remaining nested and VR defects are
  reported above rather than conflated with those passing cases.
- Pass-4 D2 is closed for the Basic Offset Table itself. Encapsulated Pixel
  Data now requires a first defined-length Item, and that Item length must be
  zero or a multiple of four. Missing and two-byte Basic Offset Table fixtures
  return `InvalidDataSet`.
- Pass-4 D3 is closed. Independent probes accepted an even-length Deflate
  stream with no suffix and an odd-length stream with one NULL byte. They
  rejected missing odd-stream padding, truncations through the final seven
  Deflate bytes, and bytes after the required suffix.
- Independent RFC 1951 inspection found exact valid boundaries for all six
  staged Deflate arrays. Five have odd stream lengths followed by one NULL
  byte. The remaining stream has even length and no suffix. Every stream
  reached its end marker and inflated to its intended fixture bytes.
- All four earlier container findings remain closed. Top-level item-control
  tags, literal delimiters in defined Sequences, delimiter context, explicit
  boundaries, and zero delimiter Value Length are enforced across the five
  dispatch paths.
- `PixelSequence`, `PixelItem`, checked item ends, lazy skipping, and the
  Deflate consumed-byte conversion were reviewed for truncation, overflow, and
  explicit versus undefined end behavior. No unchecked cast or seek behavior
  was introduced.
- The full sprint diff was reviewed again for File Meta Information isolation,
  transfer-syntax resolution, no fallback, patient-safe errors, completion
  detection, dependency features, source policy, D-18, no-std posture, corpus
  integration, LLD, and sprint records. The four defects above are the complete
  findings from this pass.
- The flate2 registry package declares `MIT OR Apache-2.0`, matching the new
  source-policy row. Its Rust backend builds on both repository targets.
- `bin/ocelli.sh test ocelli-dicom` passed 34 Part 10 integration tests and
  three unit tests. The corpus integration test remained ignored in the
  ordinary suite.
- `bin/ocelli.sh check ocelli-dicom`,
  `bin/ocelli.sh clippy ocelli-dicom`, `bin/ocelli.sh gate corpus`,
  `bin/ocelli.sh gate nostd`, and `bin/ocelli.sh gate deviations` passed. The
  corpus gate verified 92 rows across all 16 declared transfer syntaxes.
- `bin/ocelli.sh native` passed all four stages and reported identical features
  for 12 direct dependencies across native and wasm32.
- The workflow ledger records a passing 30-gate sprint verification against
  exact staged tree `bf9e4110b921b6bb2676fb2455b849541275395e`.
- `git diff --cached --check` and full sprint `git diff --check` passed.
