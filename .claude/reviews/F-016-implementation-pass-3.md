# F-016 review, pass 3

**Reviewed**: twice-remediated working tree relative to `2f0de7c`, including
untracked files
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, Valid value bytes can exhaust the sentinel search

**Where**: `crates/ocelli-dicom/src/parse.rs:265`

**What**: `choose_completion_sentinel` marks an element number unavailable
whenever its four encoded tag bytes occur in any four-byte window of the data
set, including value bytes. It has only 65,536 candidates. A valid binary value
can contain each sequence formed by the encoded group `7777` and every `u16`
element number. The search then returns `InvalidDataSet` even though none of
those byte sequences is an encoded data element and the DICOM input is valid.

**Why it is wrong**: F-016 is a parser for arbitrary valid Part 10 input. Pixel
Data and other binary VRs may contain any byte sequence. Internal completion
bookkeeping must not turn approximately 256 KiB of legal value bytes into an
invalid-data-set refusal. Scanning value bytes avoids parsed-tag collision for
ordinary inputs, but a finite single-tag namespace cannot make the marker
unconditionally available.

**Evidence**: The loop marks one slot for every four-byte window decoding to
group `7777`, and lines 288 through 293 refuse when all slots are marked. The
concatenation of `[0x77, 0x77, element_lo, element_hi]` for all little-endian
element numbers is an even-length 262,144-byte value and marks the complete
table. The corresponding big-endian construction does the same with the
element bytes reversed.

### D2, The selected marker is normally not a valid private data element

**Where**: `crates/ocelli-dicom/src/parse.rs:288`

**What**: The search starts at element `0000`, so an ordinary data set selects
`(7777,0000)`. Element `0000` is the retired Group Length element, not a
Private Creator or a Private Data Element. The explicit-VR marker nevertheless
encodes it as `UN`. Even if the search advances into `0010-00FF`, those are
Private Creator elements and require VR `LO` with a non-empty identification
code. Elements `1000-FFFF` are private data blocks whose corresponding
Private Creator is required, but the implementation appends no creator.

**Why it is wrong**: PS3.5 section 7.8.1 assigns `0010-00FF` to Private Creator
elements and maps the reserved private blocks into `1000-FFFF`. Calling the
chosen tag a complete private element is false for the candidates this search
produces, and encoding `(7777,0000)` as `UN` is not the tag's defined VR. The
strictness mechanism currently depends on dicom-rs accepting a non-conformant
element that it then removes.

**Evidence**: `present.iter().position(...)` begins at zero, while
`append_completion_sentinel` uses `UN` for every explicit-VR candidate. The
ordinary fixtures do not contain bytes `77 77 00 00`, so their selected
element is zero. The comments at lines 261 and 296 and the approved plan call
this a private tag or private element, but no Private Creator is encoded.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-2 D1 is closed for candidate tags that exist. The scan covers every
  four-byte window of the adapted input in the route's byte order, so a chosen
  tag cannot already be a parsed input tag or appear inside an input value.
  A consumed marker then leaves no chosen tag in the collected object. D1
  above is the remaining valid-input exhaustion case.
- Main-data-set group 0002 elements are now refused before marker removal.
  The old fixed-tag forgery returns `InvalidDataSet`, and a twelve-byte missing
  value that consumes the appended explicit marker returns
  `TruncatedDataSet`.
- Pass-2 D2 is closed. The plan now describes explicit data-set adaptation
  followed by strict dicom-rs collection, and no longer claims
  `InMemDicomObject::read_dataset_with_ts` runs.
- The scan is bounded to one linear pass over the already materialized data
  set and one fixed 65,536-entry boolean table. It uses little-endian windows
  for implicit, explicit little-endian, deflated, and encapsulated routes, and
  big-endian windows for Explicit VR Big Endian.
- The encoded marker header has the correct implicit-VR shape and the correct
  short or long explicit-VR length field shape for its selected VR and byte
  order. D2 concerns the semantic tag and VR assignment, not byte order.
- Deflated Explicit VR Little Endian is adapted once before scanning and
  collection. The collector uses the resolved transfer syntax without a
  fallback or flexible decoding.
- Odd lengths are refused. EOF-backed collector failures map to
  `TruncatedDataSet`, while structural failures map to `InvalidDataSet`.
- The returned object retains File Meta Information, string multiplicity,
  checked native values, and encapsulated fragments. No completion element is
  returned on the successful fixtures.
- `bin/ocelli.sh test ocelli-dicom` passed 21 executed unit and fixture tests,
  with the corpus integration test ignored in the ordinary suite.
- `bin/ocelli.sh gate corpus` passed over 92 verified rows and all 16 declared
  transfer syntaxes. Digest and metadata checks remain chained before the
  ignored Rust integration test.
- Public errors contain no input bytes, paths, DICOM values, or instance
  identifiers.
