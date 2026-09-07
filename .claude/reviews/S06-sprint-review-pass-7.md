# S06 sprint review, pass 7

**Reviewed**: full sprint diff from
`baaa92146ebd7844cca086c957f1d1015a1eec9f` plus staged remediation at tree
`d43075650205898442f8217ca038524e805650b0`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Native Format Pixel Data accepts primitive VRs other than OB or OW

**Where**: `crates/ocelli-dicom/src/parse.rs:373`

**What**: An `ElementHeader` carrying Pixel Data is rejected only when it is
top-level under an encapsulated transfer syntax. Under a native transfer syntax
the same branch accepts any explicit primitive VR. It also accepts any
primitive VR for Native Format Pixel Data nested in a Sequence under an
encapsulated syntax. Pixel Data encoded as UI therefore returns `Ok` on both
valid Native Format locations.

**Why it is wrong**: DICOM PS3.5 sections 8.1 and A.1 through A.4 constrain
Native Format Pixel Data `(7FE0,0010)` to VR OB or OW, with the permitted choice
further constrained by Bits Allocated. UI is never a Pixel Data VR. This is the
same transfer-syntax representation boundary for which the preflight now
rejects SQ and requires OB on Encapsulated Format.

**Evidence**: A standalone synthetic harness used a valid padded UI value
`1.2` so the only malformed property was the Pixel Data tag and VR pairing.
Both complete inputs returned `Ok`:

| Transfer Syntax | Pixel Data location and encoding |
|-----------------|----------------------------------|
| Explicit VR Little Endian | Top-level `(7FE0,0010)`, VR UI, Explicit Length 4 |
| JPEG Baseline | Icon Image Sequence Item containing `(7FE0,0010)`, VR UI, Explicit Length 4 |

When an `ElementHeader` has the Pixel Data tag, require its resolved VR to be
OB or OW in every Native Format location. Retain the stricter top-level refusal
under an encapsulated transfer syntax. Add both fixtures above as negative
tests and keep the nested OB positive fixture.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-6 D1 is closed. Every `SequenceStart` with the Pixel Data tag now
  returns `InvalidDataSet` at every nesting depth. The new defined-length SQ
  test passes under both Explicit VR Little Endian and JPEG Baseline.
- The SQ regression test is mutation-sensitive. On the immediately prior exact
  staged tree, where the new `tag == PIXEL_DATA` term was absent, a standalone
  defined-length SQ Pixel Data fixture returned `Ok` under both routes. The
  current test asserts `InvalidDataSet` on the same token branch, so removing
  that one guard makes both table rows red.
- Pass-5 top-level and nested representation behavior remains correct for the
  covered OB cases. Top-level Native Format Pixel Data is refused under JPEG
  Baseline, nested Native Format OB Pixel Data in an Icon Image Sequence is
  accepted, and fragmented Pixel Data is refused under a native syntax.
- Every `PixelSequenceStart` still requires the exact raw Explicit VR Little
  Endian OB header, including zero reserved bytes and Undefined Length. OW and
  SQ variants are invalid.
- Pixel Sequence state still requires a valid Basic Offset Table and at least
  one completed Fragment. Missing table, a two-byte table, table only,
  zero-length Fragment, undefined-length Fragment, and odd-length Fragment are
  invalid. A two-byte Fragment is accepted.
- Deflate boundary validation remains exact. Independent probes accepted even
  streams with no suffix and odd streams with one NULL byte. Missing padding,
  stream truncation, multiple suffix bytes, and nonzero suffix bytes are
  refused. All six fixed arrays have independently verified RFC 1951 ends and
  DICOM padding.
- The committed F-016 AS_BUILT entry remains byte-identical to commit
  `43173e9`. There is exactly one append-only correction entry, and its current
  count of forty Part 10 tests matches the executed suite.
- All earlier ordinary Sequence, Item, delimiter, completion-marker, dispatch,
  and error-classification findings remain closed across the five paths.
- The full sprint diff was reviewed again for File Meta Information isolation,
  one-time adaptation, no fallback, raw cursor bounds, checked arithmetic,
  patient-safe errors, dependencies, source policy, D-18, no-std posture,
  corpus integration, LLD, provenance, and workflow state. D1 above is the
  only new exact finding.
- `bin/ocelli.sh test ocelli-dicom` passed 40 Part 10 integration tests and
  three unit tests. The corpus integration test remained ignored in the
  ordinary suite.
- `bin/ocelli.sh check ocelli-dicom`,
  `bin/ocelli.sh clippy ocelli-dicom`, `bin/ocelli.sh gate corpus`,
  `bin/ocelli.sh gate nostd`, and `bin/ocelli.sh gate deviations` passed. The
  corpus gate verified 92 rows across all 16 declared transfer syntaxes.
- `bin/ocelli.sh native` passed all four stages and reported identical features
  for 12 direct dependencies across native and wasm32.
- The workflow ledger records a passing 30-gate sprint verification against
  exact staged tree `d43075650205898442f8217ca038524e805650b0`.
- `git diff --cached --check` and full sprint `git diff --check` passed.
