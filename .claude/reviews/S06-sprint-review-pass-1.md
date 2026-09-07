# S06 sprint review, pass 1

**Reviewed**: `baaa92146ebd7844cca086c957f1d1015a1eec9f..43173e909c5b0e4b11da531b8242bd4f56cb4b15`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Top-level item-control tags bypass the structural preflight

**Where**: `crates/ocelli-dicom/src/parse.rs:285`

**What**: `validate_data_set_structure` accepts an `ElementHeader` unless it
is a top-level group 0002 tag. dicom-rs presents a top-level Item tag
`(FFFE,E000)` and Sequence Delimitation Item `(FFFE,E0DD)` through that arm,
so both pass preflight and collection as ordinary zero-length elements. A Part
10 file whose complete main data set is either control tag returns `Ok`.
A top-level Item Delimitation Item `(FFFE,E00D)` is refused, but it returns
`TruncatedDataSet` even though the complete invalid control token is present.

**Why it is wrong**: PS3.5 section 7.5 and its item encoding rules reserve
these tags for item and sequence structure. They are not standalone top-level
Data Elements. The accepted inputs have malformed nesting. This contradicts
the approved plan at lines 194 through 197, `docs/lld/dicom-ingest.md` lines
38 through 44, and the public error contract that distinguishes incomplete
input from invalid structure.

**Evidence**: A standalone synthetic harness linked against the HEAD
`ocelli-dicom` library. It constructed valid Explicit VR Little Endian Part 10
File Meta Information, then used each eight-byte little-endian control token as
the entire main data set. The observed `parse_part10(...).err()` values were:

| Main data-set bytes | Token | Observed |
|---------------------|-------|----------|
| `FE FF 00 E0 00 00 00 00` | Item, `(FFFE,E000)`, length 0 | `None` |
| `FE FF DD E0 00 00 00 00` | Sequence Delimitation Item, `(FFFE,E0DD)`, length 0 | `None` |
| `FE FF 0D E0 00 00 00 00` | Item Delimitation Item, `(FFFE,E00D)`, length 0 | `Some(TruncatedDataSet)` |

Reject item-control tags when dicom-rs exposes them as ordinary element
headers outside the required sequence context, and add public integration
tests for all three top-level forms. The complete Item Delimitation form must
be classified as `InvalidDataSet`, not truncation.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Part 10 File Meta Information is read independently under Explicit VR Little
  Endian. Transfer Syntax UID resolution selects one registry descriptor and
  there is no alternate-syntax retry.
- Implicit VR Little Endian, Explicit VR Little Endian, Explicit VR Big
  Endian, Deflated Explicit VR Little Endian, and encapsulated routes preserve
  observable dispatch evidence. Encapsulated fragments are retained without
  pixel decode.
- The fixed completion marker closes the previously reviewed partial-header,
  forged-marker, finite-search, and invalid-private-element defects. It is
  inserted only after preflight, required after collection, and removed before
  return.
- Odd lengths, incomplete values, partial top-level headers, unfinished
  encapsulated Pixel Data, and the tested malformed encapsulated delimiter are
  refused. D1 is the uncovered top-level item-control case.
- Public `ParseError` variants contain no input bytes, paths, attribute values,
  instance identifiers, or upstream error tokens. Corpus failures expose only
  row numbers and transfer-syntax identifiers.
- The diff adds no `unsafe`, no `wasm-bindgen` use, no pixel boundary, and no
  render-loop work. The one `Box<dyn>` is required by the dynamically selected
  dicom-rs data-set adapter rather than wrapping a statically known type.
- D-18, the four direct workspace dependencies, the locked feature graph, the
  no-std guard, and the LLD agree on `ocelli-dicom` using `std`. Transfer-syntax
  registry defaults, inventory registration, and pixel codec features are not
  enabled.
- `CHANGELOG.md`, the LLD index and three affected LLD files, `AS_BUILT.md`,
  `BACKLOG.md`, `CURRENT_SPRINT.md`, and `SPRINT_TRACKER.md` all record the
  implemented F-016 scope and completion state consistently.
- The S06 run state records F-016 completed after five feature reviews. Its
  sprint verification record names tree
  `c5b4b72d6e257987a5944662de0626dce31f695a`, which is exactly `HEAD^{tree}`.
  `python3 scripts/verify_ledger.py check-commit 43173e9` passed with corpus
  recorded as pass and the tree matching.
- `bin/ocelli.sh gate backlog deviations prose content provenance nostd`
  passed all six selected gates. `bin/ocelli.sh test ocelli-dicom` passed 22
  executed unit and fixture tests, with the local corpus test ignored in the
  ordinary suite.
- The recorded sprint verification covers all 30 sprint gates, including the
  corpus and oracle. The committed AS_BUILT claim of 26 floor gates plus the
  corpus gate agrees with the feature verification trailer.
- `git diff --check` passed for the complete sprint range.
