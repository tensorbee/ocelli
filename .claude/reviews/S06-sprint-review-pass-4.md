# S06 sprint review, pass 4

**Reviewed**: full sprint diff from
`baaa92146ebd7844cca086c957f1d1015a1eec9f` plus staged remediation at tree
`eb65d102157ac315132eef7ae86ea1d13f326b07`
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, Pixel Data representation is not bound to the transfer syntax

**Where**: `crates/ocelli-dicom/src/parse.rs:325` and
`crates/ocelli-dicom/src/parse.rs:344`

**What**: The structural preflight accepts `PixelSequenceStart` without
checking that the selected transfer syntax uses encapsulated Pixel Data. It
also accepts a defined-length top-level Pixel Data element as an ordinary
value when the selected syntax is encapsulated. A Part 10 file can therefore
declare a native syntax while carrying fragments, or declare JPEG Baseline
while carrying native Pixel Data, and still return `Ok` with dispatch evidence
for the declared but incompatible route.

**Why it is wrong**: DICOM PS3.5 annex A binds Pixel Data representation to the
Transfer Syntax. Section A.4 requires top-level Pixel Data under an
encapsulated syntax to use VR OB, Undefined Length, and encapsulated Items.
Sections A.1 through A.3 use Native Format for the native transfer syntaxes.
Section A.5 explicitly requires Deflated Explicit VR Little Endian Pixel Data
to be sequential uncompressed frames without encapsulation before the whole
data set is deflated. Observable dispatch is not trustworthy if the returned
Pixel Data contradicts that dispatch.

**Evidence**: A standalone synthetic Part 10 harness linked against the staged
library. Both complete inputs returned `Ok`:

| Declared Transfer Syntax | Top-level Pixel Data encoding |
|--------------------------|-------------------------------|
| Explicit VR Little Endian | OB, Undefined Length, empty Basic Offset Table, one four-byte Fragment, Sequence Delimitation Item |
| JPEG Baseline | OB, Explicit Length 4, four native bytes |

Pass the selected Pixel Data representation into structural validation. Reject
top-level `PixelSequenceStart` for native and whole-data-set Deflate routes,
and reject defined-length top-level Pixel Data for encapsulated routes. Add a
negative test for each direction.

### D2, Encapsulated Pixel Data does not require a valid Basic Offset Table Item

**Where**: `crates/ocelli-dicom/src/parse.rs:344`

**What**: `PixelSequenceStart` is represented as an ordinary undefined-length
Sequence. No state records that its first Item is the Basic Offset Table. An
immediate Sequence Delimitation Item is accepted, so the required Item is
absent. A first Item with a two-byte Value is also accepted and becomes an
empty offset table even though it cannot contain a sequence of 32-bit offsets.

**Why it is wrong**: DICOM PS3.5 section A.4 requires the first Item before the
encoded Pixel Data Stream to be the Basic Offset Table Item. Its Item Length is
zero when no value is present. Otherwise its value is a concatenation of
32-bit unsigned offsets, so the Item Length must be a multiple of four. The
Item itself is mandatory even when its value is empty.

**Evidence**: Two independent JPEG Baseline Part 10 fixtures returned `Ok`:

- Pixel Data header with Undefined Length followed immediately by the
  zero-length Sequence Delimitation Item, with no Basic Offset Table Item.
- Pixel Data header with Undefined Length followed by one Item of length 2,
  two zero bytes, and the Sequence Delimitation Item.

Track Pixel Data separately from an ordinary Sequence. Require its first token
to be a defined-length Item whose length is zero or a multiple of four before
accepting fragment Items or the final delimiter. Add standing tests for the
missing and malformed Basic Offset Table cases.

### D3, Deflate stream termination and mandatory padding are not validated

**Where**: `crates/ocelli-dicom/src/parse.rs:272` and
`crates/ocelli-dicom/tests/part10.rs:20`

**What**: The data-set adapter is given a fresh cursor and consumed only
through `read_to_end` on the decoded side. Ocelli never observes where the RFC
1951 stream ended or what encoded bytes remained. Arbitrary trailing bytes are
silently ignored. Conversely, five staged fixtures use an odd number of
Deflate bytes without appending the mandatory DICOM padding byte.

**Why it is wrong**: DICOM PS3.5 section A.5 requires the entire Explicit VR
Little Endian Data Set to be one Deflate stream. If that stream has odd byte
length, exactly one trailing NULL byte shall be appended after the stream.
Other trailing bytes are not part of this Transfer Syntax. Accepting ignored
bytes defeats complete-input validation, while unpadded odd fixtures are not
conformant evidence for the route they claim to test.

**Evidence**:

- A valid 49-byte raw Deflate stream followed by bytes `01 02 03 04` returned
  `Ok`. The same unpadded 49-byte stream also returned `Ok`.
- Independent counting of the staged constants found these odd encoded
  lengths with no appended padding byte:

| Fixture | Encoded bytes |
|---------|--------------:|
| `DEFLATED_DATA_SET` | 67 |
| `DEFLATED_DEFINED_SEQUENCE_WITH_ITEM` | 21 |
| `DEFLATED_DEFINED_SEQUENCE_WITH_ITEM_DELIMITER` | 21 |
| `DEFLATED_DEFINED_SEQUENCE_WITH_SEQUENCE_DELIMITER` | 21 |
| `DEFLATED_UNDEFINED_SEQUENCE_WITH_NONZERO_DELIMITER` | 21 |

Preserve access to the encoded cursor after inflation. Require the end marker
to be followed by no bytes when the compressed stream length is even, or by
one NULL byte when it is odd. Reject every other suffix. Append the required
NULL byte to each odd fixture and add negative cases for missing padding and
extra nonzero bytes.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-3 D1 is closed. `validate_container_end` now checks that the four bytes
  of an undefined-length Item or Sequence delimiter Value Length are all zero.
  The new route matrix refuses both delimiter tags on all five dispatch paths,
  and the actual encapsulated fragment fixture refuses a nonzero Sequence
  Delimitation Item length.
- The two earlier sprint findings remain closed. Top-level item-control tags
  are invalid, and defined-length Sequences distinguish Items from delimiters
  with exact explicit boundaries.
- The valid defined and undefined ordinary Sequence and Item combinations from
  pass 3 remain accepted, including nested mixed-length structures and the
  Deflated route. Truncation and invalid-structure classifications remain
  distinct for the tested boundary undershoot, overshoot, and missing
  delimiter cases.
- `PositionedCursor` position updates, checked container arithmetic, explicit
  zero-byte ends, undefined eight-byte ends, and the added raw delimiter slice
  bounds were reviewed. The pass-3 fix has no unchecked integer conversion and
  does not reject the valid big-endian zero length encoding.
- The complete sprint diff was reviewed again for public API, File Meta
  Information isolation, transfer-syntax resolution, no fallback, error
  safety, completion-marker handling, dependency features, D-18, no-std
  posture, corpus integration, LLD, and delivery records. Apart from the three
  defects above, these remain consistent with the approved scope.
- `bin/ocelli.sh test ocelli-dicom` passed 28 Part 10 integration tests and
  three unit tests. The corpus integration test remained ignored in the
  ordinary suite.
- `bin/ocelli.sh check ocelli-dicom`,
  `bin/ocelli.sh clippy ocelli-dicom`, `bin/ocelli.sh gate corpus`,
  `bin/ocelli.sh gate nostd`, and `bin/ocelli.sh gate deviations` passed. The
  corpus gate verified 92 rows over all 16 declared transfer syntaxes.
- `bin/ocelli.sh native` passed all four stages, including wasm32, native test
  compilation, and identical direct dependency features across targets.
- The workflow ledger records a passing 30-gate sprint verification against
  exact staged tree `eb65d102157ac315132eef7ae86ea1d13f326b07`.
- `git diff --cached --check` and full sprint `git diff --check` passed.
