# F-016 review, pass 1

**Reviewed**: working tree relative to `2f0de7c`, including untracked files
**Result**: 4 defects, 0 smells, 0 nitpicks

## Defects

### D1, The implementation abandons an enforced no_std posture without a reviewed decision

**Where**: `crates/ocelli-dicom/src/lib.rs:5`

**What**: F-016 removes `#![cfg_attr(not(test), no_std)]` from
`ocelli-dicom`, while `scripts/no_std_check.py` still names that crate in
`EXPECTED_NO_STD_CRATES`. Neither the approved plan nor D-18 records this
posture change.

**Why it is wrong**: Repository workflow makes the explicit no_std set a
reviewed invariant. D-09 says every core crate carries the attribute, and the
guard says removing it is not a fix for a dependency that reaches `std`. A
necessary posture change must be designed and recorded together with the
guard update. Leaving the two in disagreement makes the completion gate red.

**Evidence**: `bin/ocelli.sh gate nostd` exited 1 with
`ocelli-dicom: stopped declaring no_std`. `cargo tree -p ocelli-dicom -e
normal,features` also showed the dicom-rs graph reaching multiple `std`
features, so restoring the attribute alone would not satisfy the existing
posture.

### D2, Structurally invalid data sets are reported as truncated

**Where**: `crates/ocelli-dicom/src/parse.rs:209`

**What**: `map_data_set_error` maps every dicom-rs `ReadError::ReadToken` to
`TruncatedDataSet`. That wrapper also carries structural parser errors such as
`InvalidElementLength`, `InvalidItemLength`, `UnexpectedItemHeader`, and
`UnexpectedItemTag`. Those inputs therefore receive the wrong public error
class. The suite has no invalid-data-set fixture that reaches
`ParseError::InvalidDataSet`.

**Why it is wrong**: The approved plan and the public API promise distinct
truncated and invalid data-set refusals. HLD section 23 requires useful error
classification, and a malformed structure is not evidence that the input
ended early.

**Evidence**: Inspection of the pinned `dicom-object` 0.10 `ReadError` and
`dicom-parser` 0.10 data-set error enums shows the structural variants are
wrapped by `ReadToken`. `bin/ocelli.sh test ocelli-dicom` ran 12 Part 10 tests,
but none supplied a complete structurally invalid data set or asserted
`InvalidDataSet`.

### D3, Some truncated data sets are silently accepted as complete

**Where**: `crates/ocelli-dicom/src/parse.rs:163`

**What**: The one-shot call to `InMemDicomObject::read_dataset_with_ts` uses
dicom-rs's permissive end handling without an Ocelli completeness check. The
pinned parser treats unexpected EOF while reading the next element tag as a
graceful top-level end. It also treats unexpected EOF inside an encapsulated
Pixel Data sequence as a graceful end. A file ending one to three bytes into
an element header, or before the required sequence delimiter of undefined
length Pixel Data, can therefore return `Ok` with a partial object.

**Why it is wrong**: PS3.5 sections 7.1 and 7.5 define complete element
headers and undefined-length sequence delimitation. The approved plan requires
a truncated data set to be refused without fallback, and the local error
documentation promises refusal when the input ends inside a token or value.
Silently dropping the tail is the dangerous failure shape for a parser.

**Evidence**: In the pinned `dicom-parser` 0.10 source, both
`UnexpectedEof` branches set `hard_break` and return `None`, which makes
`InMemDicomObject::build_object` finish successfully. The existing truncation
fixture removes one byte from a value, so it exercises `ReadValue` and does
not cover either accepted-EOF branch.

### D4, Odd data-element lengths are accepted despite PS3.5

**Where**: `crates/ocelli-dicom/src/parse.rs:163`

**What**: `read_dataset_with_ts` constructs the data-set reader with default
options. In dicom-parser 0.10, `OddLengthStrategy::default()` is `Accept`, so
a non-conformant odd value length is parsed rather than refused. The fixture
helpers assert that generated lengths are even, but there is no negative
fixture proving the public parser refuses an odd length.

**Why it is wrong**: PS3.5 sections 6.2 and 6.4 require value fields to be
even length and padded according to VR. The DICOM review reference calls out
that an odd-length element does not exist. Returning a successful object for
that structure conflicts with `ParseError::InvalidDataSet` and the plan's
claim to parse valid data under the declared syntax.

**Evidence**: The pinned parser defines `OddLengthStrategy` with `Accept` as
its default and `sanitize_length` returns the odd length unchanged on that
path. `InMemDicomObject::read_dataset_with_ts` reaches a
`DataSetReader::new_with_ts_cs` call with default options.

## Smells

None.

## Nitpicks

None.

## Verified clean

- File Meta Information is read separately from byte 128 under dicom-rs's
  fixed Explicit VR Little Endian meta parser, before the declared data-set
  transfer syntax is resolved.
- Unknown and unavailable transfer syntaxes are refused before data-set
  parsing. No Explicit VR Little Endian retry or flexible-decoding fallback is
  enabled.
- Implicit VR Little Endian, Explicit VR Little Endian, retired Explicit VR
  Big Endian, Deflated Explicit VR Little Endian, and encapsulated synthetic
  fixtures all select the intended route and retain the checked values.
- Encapsulated Pixel Data remains a fragment sequence and F-016 performs no
  pixel decode.
- Public errors retain no bytes, paths, DICOM values, or instance identifiers.
- `bin/ocelli.sh test ocelli-dicom` passed 15 executed tests with the corpus
  test ignored in the ordinary suite.
- `bin/ocelli.sh gate corpus` passed over 92 verified rows and all 16 declared
  transfer syntaxes. The corpus arm chains digest, metadata, and Rust parser
  checks with `&&`, so an earlier failure cannot be hidden by the final test.
- `bin/ocelli.sh native` passed the native build, wasm32 build, native tests,
  and cross-target feature comparison.
- The diff adds no `unsafe`, `wasm-bindgen`, decoder trait, generic parameter,
  dynamic dispatch, rendering work, or pixel movement across the boundary.
- The recorded mutation changed the two route mappings and made the implicit
  and big-endian route assertions fail, then pass after restoration.
