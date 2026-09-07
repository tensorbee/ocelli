# S06 sprint review, pass 3

**Reviewed**: full sprint diff from
`baaa92146ebd7844cca086c957f1d1015a1eec9f` plus staged remediation at tree
`48a2e2e987d9956f5357f73807a8cfccdac6dd76`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Undefined-length delimiters accept a nonzero Value Length

**Where**: `crates/ocelli-dicom/src/parse.rs:406`

**What**: `validate_container_end` accepts an undefined-length Item or
Sequence end whenever the parser consumed exactly eight bytes. Those eight
bytes include the delimiter's four-byte Value Length field, but the function
does not check that field. Both Item Delimitation Item `(FFFE,E00D)` and
Sequence Delimitation Item `(FFFE,E0DD)` are therefore accepted with a Value
Length other than zero. The same omission accepts a malformed Sequence
Delimitation Item at the end of encapsulated Pixel Data.

**Why it is wrong**: DICOM PS3.5 sections 7.5.1 and 7.5.2 specify that the
Value Length field of both delimitation items is `00000000`. A complete
delimiter whose Value Length is 4 or `FFFFFFFF` is invalid under the declared
transfer syntax. It must not produce `Ok`. Checking only the cursor delta
proves that a control header was consumed, not that the control header is
valid.

**Evidence**: A standalone synthetic harness linked directly against the
library built from the staged tree. It constructed no patient attributes and
reported these results:

| Malformed input | Observed result |
|-----------------|-----------------|
| Undefined Item ended by `(FFFE,E00D)` with Value Length 4 | `Ok` on all five dispatch paths |
| Undefined Sequence ended by `(FFFE,E0DD)` with Value Length 4 | `Ok` on all five dispatch paths |
| Undefined Sequence ended by `(FFFE,E0DD)` with Value Length `FFFFFFFF` | `Ok` on Explicit VR Little Endian |
| Encapsulated Pixel Data with an empty Basic Offset Table, one four-byte fragment, and `(FFFE,E0DD)` with Value Length 4 | `Ok` on the JPEG Baseline encapsulated route |

The five-path matrix covered Implicit VR Little Endian, Explicit VR Little
Endian, Explicit VR Big Endian, Deflated Explicit VR Little Endian, and the
encapsulated route. The current dicom-rs token does not retain the raw
delimiter length. Ocelli still has the adapted byte slice and both cursor
positions, so the strict preflight can require the final four bytes of every
consumed undefined-length delimiter header to be zero. Add standing tests for
both delimiter tags across all five routes, including the actual Pixel Data
fragment path.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-1 D1 is closed. Top-level Item, Item Delimitation Item, and Sequence
  Delimitation Item return `InvalidDataSet` on all five dispatch paths.
- Pass-2 D1 is closed. A defined-length Sequence accepts an empty Item and
  rejects a literal Item Delimitation Item or Sequence Delimitation Item on all
  five paths. `DataSetContainer` now distinguishes Sequence and Item context
  and retains each explicit boundary.
- A separate matrix accepted 24 valid ordinary container forms across the four
  uncompressed routes. It covered defined and undefined Sequences, defined and
  undefined Items, both mixed-length combinations, and nested Sequences with
  mixed and fully defined containers. A separately deflated nested mixed-length
  form reached `DeflatedExplicitVrLittleEndian`.
- Malformed probes classified a missing undefined Sequence delimiter and EOF
  with both undefined delimiters missing as `TruncatedDataSet`. They classified
  a scalar or nested Sequence directly inside a Sequence, an Item directly
  inside an Item, and an explicit boundary undershoot as `InvalidDataSet`. An
  explicit boundary extending past EOF remained `TruncatedDataSet`.
- `PositionedCursor` updates its shared position after every successful read
  and seek. Lazy value skipping is visible at the start of the next iteration.
  Exact explicit ends consume zero bytes and undefined ends consume one
  eight-byte control header. Container end arithmetic uses `u64` and
  `checked_add`, with no narrowing cast.
- The valid encapsulated fixture still retains its empty Basic Offset Table and
  four-byte fragment without pixel decode. Its required zero-length Sequence
  delimiter is handled correctly. Missing encapsulated termination is refused.
- Independent raw RFC 1951 inflation reproduced the exact expected 20-byte
  payload for each of the three fixed Deflate arrays. The payloads were the
  Explicit VR Little Endian defined-length SQ header followed respectively by
  an empty Item, an Item Delimitation Item, and a Sequence Delimitation Item.
- The complete sprint diff was checked again for public API, transfer-syntax
  dispatch, one-time adaptation and collection, completion-marker handling,
  patient-safe errors, corpus integration, D-18 dependency selection, no-std
  posture, cross-target behavior, LLD, and delivery records. No additional
  defect class was found.
- `bin/ocelli.sh test ocelli-dicom` passed 25 Part 10 integration tests, three
  unit tests, and left the corpus test ignored in the ordinary suite.
  `bin/ocelli.sh check ocelli-dicom` and
  `bin/ocelli.sh clippy ocelli-dicom` passed.
- `bin/ocelli.sh gate corpus`, `bin/ocelli.sh gate nostd`, and
  `bin/ocelli.sh gate deviations` passed. The corpus gate verified 92 rows
  across all 16 declared transfer syntaxes and ran the ignored Rust integration
  test.
- `bin/ocelli.sh native` passed all four stages, including wasm32, native test
  compilation, and identical direct dependency features across targets.
- The workflow ledger records a passing 30-gate sprint verification against
  the exact reviewed tree `48a2e2e987d9956f5357f73807a8cfccdac6dd76`.
- `git diff --cached --check` and the full sprint `git diff --check` passed.
