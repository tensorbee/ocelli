# S06 sprint review, pass 6

**Reviewed**: full sprint diff from
`baaa92146ebd7844cca086c957f1d1015a1eec9f` plus staged remediation at tree
`90ce4baa92989aac68224cb5a49bc8161485a1dd`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Defined-length SQ Pixel Data bypasses both representation checks

**Where**: `crates/ocelli-dicom/src/parse.rs:382`

**What**: Top-level Pixel Data `(7FE0,0010)` encoded with explicit VR SQ and a
defined-length Item value produces `LazyDataToken::SequenceStart`. That branch
checks sequence context, group FFFE, and top-level group 0002, but never checks
the Pixel Data tag. It therefore bypasses both the defined primitive Pixel Data
rejection in `ElementHeader` and the OB header validation in
`PixelSequenceStart`.

**Why it is wrong**: DICOM PS3.5 sections A.1 through A.5 encode Pixel Data as
Native Format with VR OB or OW and an Explicit Value Length, or as Encapsulated
Format with VR OB and Undefined Length. Pixel Data is not a Data Set Sequence
and cannot use VR SQ under either native or encapsulated transfer syntaxes.

**Evidence**: A standalone synthetic Part 10 harness encoded `(7FE0,0010)` as
VR SQ with Explicit Length 8 and one empty defined-length Item. The complete
malformed data set returned `Ok` under both JPEG Baseline and Explicit VR
Little Endian. The staged OW and undefined-length SQ tests do not reach this
branch because they produce `PixelSequenceStart`.

Reject `tag == PIXEL_DATA` in the `SequenceStart` branch at every nesting
depth. Add a defined-length SQ fixture under both one native route and one
encapsulated route so the two Pixel Data token paths remain independently
covered.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-5 D1 is closed. Defined-length native OB Pixel Data in an Icon Image
  Sequence Item parses under JPEG Baseline, while defined-length top-level
  Pixel Data remains invalid for that encapsulated transfer syntax.
- Pass-5 D2 is closed for every `PixelSequenceStart`. The validator uses exact
  raw cursor bounds and requires the eight bytes after the tag to be VR OB,
  two zero reserved bytes, and Undefined Length. The valid OB fixture parses,
  while otherwise identical OW and SQ fixtures return `InvalidDataSet`.
- Pass-5 D3 is closed. Pixel Sequence state now distinguishes completion of
  the Basic Offset Table from completion of a Fragment. Independent fixtures
  confirmed that missing table, a two-byte table, table only, zero-length
  Fragment, undefined-length Fragment, and odd-length Fragment are invalid. A
  minimum two-byte Fragment is accepted.
- Pass-5 D4 is closed. A direct extraction comparison found the committed
  F-016 entry byte-identical to commit `43173e9`. Exactly one correction entry
  is appended at the bottom. Its change from nineteen to thirty-nine Part 10
  tests matches the executed suite, and its listed remediation areas match the
  staged diff.
- Pass-4 Deflate remediation remains correct. Even streams require no suffix,
  odd streams require exactly one NULL byte, missing padding and arbitrary
  trailing bytes are refused, and truncated streams remain refused. All six
  fixed Deflate arrays independently reach an RFC 1951 end marker with the
  required exact suffix.
- All earlier ordinary Sequence and Item findings remain closed across the
  five dispatch paths. Context, explicit boundaries, truncation, group FFFE
  controls, and zero delimiter Value Length remain enforced.
- The full sprint diff was reviewed again for File Meta Information isolation,
  one-time transfer-syntax resolution and adaptation, no fallback, raw cursor
  positions, checked arithmetic, patient-safe errors, completion detection,
  dependencies, source policy, D-18, no-std posture, corpus integration, LLD,
  provenance, and workflow records. D1 above is the only new exact finding.
- `bin/ocelli.sh test ocelli-dicom` passed 39 Part 10 integration tests and
  three unit tests. The corpus integration test remained ignored in the
  ordinary suite.
- `bin/ocelli.sh check ocelli-dicom`,
  `bin/ocelli.sh clippy ocelli-dicom`, `bin/ocelli.sh gate corpus`,
  `bin/ocelli.sh gate nostd`, and `bin/ocelli.sh gate deviations` passed. The
  corpus gate verified 92 rows across all 16 declared transfer syntaxes.
- `bin/ocelli.sh native` passed all four stages and reported identical features
  for 12 direct dependencies across native and wasm32.
- The workflow ledger records a passing 30-gate sprint verification against
  exact staged tree `90ce4baa92989aac68224cb5a49bc8161485a1dd`.
- `git diff --cached --check` and full sprint `git diff --check` passed.
