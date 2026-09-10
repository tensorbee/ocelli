# DICOM ingest

**F-IDs that contributed:** F-016, F-017, F-019, F-021, F-025
**Last updated:** 2026-09-10

`ocelli-dicom` owns the first ingest boundary. It accepts an in-memory DICOM
Part 10 file, resolves its declared transfer syntax, and returns the complete
dicom-rs object together with evidence of the parser route that produced it.
It projects metadata without collapsing absence, emptiness, value
multiplicity, signedness, or nested data sets. It does not decode compressed
pixel frames.

`SeriesSource` is the transport-neutral response boundary above this parser.
Its first implementation, `DicomwebSource`, consumes bytes already fetched
and content-negotiated by TypeScript. QIDO JSON becomes ordered
`MetadataSet` values. WADO-URI and WADO-RS instance payloads reuse
`parse_part10`. WADO-RS frame payloads remain encoded ranges into one owned
multipart response. The source does not perform I/O or depend on browser
APIs.

## Public boundary

```rust
pub fn parse_part10(bytes: &[u8]) -> Result<ParsedDicom, ParseError>
```

`ParsedDicom::object()` borrows the complete `DefaultDicomObject`, including
File Meta Information. `into_object()` transfers ownership. The separate
`TransferSyntaxInfo` records the canonical registry UID, registry name, and
`DispatchPath`. Keeping that evidence beside the object prevents later code
from having to infer which wire encoding was selected.

`parse_part10` requires the 128-byte preamble and `DICM` prefix. A bare data
set is not accepted because it carries no Part 10 Transfer Syntax UID and would
require an encoding guess.

## Parse order

The order is fixed by DICOM PS3.10 sections 7 and 7.1.

1. Require the complete preamble and `DICM` prefix.
2. Read File Meta Information under Explicit VR Little Endian, independent of
   the syntax declared for the following data set.
3. Resolve Transfer Syntax UID `(0002,0010)` once through the static dicom-rs
   registry. An unknown UID or a syntax with no readable data-set encoding is
   refused.
4. Apply the resolved whole-data-set adapter once. This is where Deflated
   Explicit VR Little Endian is inflated.
5. Walk the adapted bytes with a strict lazy parser under the resolved syntax.
   Odd lengths, malformed nesting, incomplete values, and top-level group 0002
   elements are refused.
6. Append a fixed completion marker and collect the data set once under the
   same resolved syntax. Require and remove the marker before returning the
   object.

There is no retry with Explicit VR Little Endian and no alternate-syntax
fallback.

## Supported routes

| `DispatchPath` | Wire rule | Pixel handling |
|----------------|-----------|----------------|
| `ImplicitVrLittleEndian` | PS3.5 A.1 | Native values retained |
| `ExplicitVrLittleEndian` | PS3.5 A.2 | Native values retained |
| `ExplicitVrBigEndian` | PS3.5 A.3, retired | Native values retained |
| `DeflatedExplicitVrLittleEndian` | PS3.5 A.5 | Whole data set inflated before parsing |
| `Encapsulated` | PS3.5 A.4 | Fragment sequence retained without frame decode |

The encapsulated route covers registry entries whose data-set encoding can be
read. Codec-specific pixel capability belongs to F-023 and its dependent
stories. Deflated Explicit VR Little Endian remains a whole-data-set ingest
route rather than a frame decoder. F-025 keeps its UID `KnownUnavailable` in
`ocelli-codec` while proving this parser selects
`DispatchPath::DeflatedExplicitVrLittleEndian` and exposes the same synthetic
pixel truth as the native reference.

## Strict completion detection

dicom-rs treats EOF reached one to three bytes into a top-level tag as a clean
end. Ocelli makes that tail observable without changing dicom-rs or accepting
partial input.

The structural preflight first refuses any complete top-level group 0002
element in the main data set. Ocelli can then append `(0002,0000)` with VR `UL`
as an out-of-band completion marker, encoded according to the selected data-set
syntax. If a partial header or value consumes any marker bytes, collection
cannot return the exact marker. An unfinished encapsulated sequence also fails
the strict structural pass. The marker is removed before the object crosses
the public boundary.

This construction has no private-element allocation and no search through
untrusted value bytes.

## Error contract

| Error | Meaning |
|-------|---------|
| `TruncatedPart10` | Input ends before the complete Part 10 prefix |
| `MissingDicmPrefix` | Bytes 128 through 131 are not `DICM` |
| `InvalidFileMeta` | File Meta Information is malformed |
| `MissingTransferSyntax` | `(0002,0010)` is absent |
| `UnknownTransferSyntax` | The static registry does not know the UID |
| `UnsupportedDataSetTransferSyntax` | The registry entry cannot supply a supported data-set route |
| `TruncatedDataSet` | The data set ends inside a token, value, or sequence |
| `InvalidDataSet` | Structure is invalid under the declared syntax |

The variants carry no input bytes, attribute values, instance identifiers,
paths, or upstream parser tokens. A caller can report the error class without
placing DICOM content in logs.

## Lossless metadata model

`MetadataSet::from_object` projects the parsed main data set into an ordered
map keyed by `Tag`. A missing tag is the absence of a map entry. A present
zero-length element is `MetadataValue::Empty`, so callers cannot confuse Type
2 emptiness with a missing optional value. Inserting the same tag twice is a
structural error.

Each `MetadataElement` retains its declared `VR`. `MetadataValue` keeps text,
attribute tags, binary values, every signed and unsigned integer width,
floating-point widths, partial date and time values, and nested `SQ` items in
separate variants. dicom-rs collection uses its preserved-value path, so text
components keep their source spelling and legal UI NULL or text space pad.
`MetadataElement::semantic_text` uses the retained VR to compute a view without
changing that spelling. It trims both ends only for VRs where leading spaces
are insignificant. ST, LT, UT, and other trailing-pad-only VRs retain leading
spaces. Ordered multi-valued DS text therefore remains ordered text rather
than being eagerly converted to floating point.

An ordinary sequence becomes `MetadataValue::Sequence(Vec<MetadataSet>)` and
retains item order. Encapsulated Pixel Data fragments return
`UnsupportedPixelFragments` because frame assembly and decode belong to the
codec path.

F-021 constructs the same model from DICOM JSON. Person Name component
objects, `BulkDataURI`, and `InlineBinary` have explicit typed constructors.
The constructors enforce the carrier VR sets in PS3.18 F.2.2. Inline binary
must be nonempty canonical padded base64 with valid unused bits. A present
empty DICOM JSON attribute uses `MetadataValue::Empty` as required by PS3.18
F.2.5. Neither binary carrier is flattened into an ordinary string value.
`MetadataElement::sequence` fixes the VR to `SQ` and retains ordered nested
items for the DICOM JSON path without exposing a generic unchecked
constructor.
`MetadataElement::with_null_slots` adds a checked wrapper around an existing
typed value for PS3.18 F.2.5. It retains total multiplicity and ordered null
positions without weakening the existing value constructors. SQ nulls and
binary carriers cannot use the wrapper.

## Multiframe projection and frame indexing

`MultiframeMetadata` is a borrowed checked view over one `MetadataSet`.
Number of Frames `(0028,0008)` defaults to one only when absent. A present
value must be one positive IS value whose preserved spelling, including pad,
is at most 12 bytes and whose numeric value fits DICOM's signed 32-bit IS
range. Empty, zero, multiple, malformed, and overflowing declarations are
distinct refusals. A present Per-frame Functional Groups Sequence must contain
exactly the declared number of items. Shared Functional Groups may contain
zero or one item.

Attribute resolution is scoped to one checked zero-based frame. It consults
the requested group in the per-frame item first and the shared item second.
Top-level fallback is consulted only when the caller explicitly identifies it
as legal for that attribute's module. The result borrows the original
`MetadataElement`, names its winning `FunctionalGroupSource`, and records any
lower-precedence duplicate source. Duplicate evidence is not erased merely
because the per-frame value wins.

The projection leaves sequence item order unchanged. Dimension Index, Frame
Content, plane position, plane orientation, pixel measures, rescale, window,
and real-world mapping values remain lossless metadata. It does not sort
frames, derive geometry, calibrate spacing, or interpret gantry tilt. Those
operations remain F-020 scope.

`EncapsulatedFrameIndex` groups borrowed Fragment Values without concatenating
or decoding them. Basic Offset Table entries are checked against Fragment Item
Tag offsets measured from the first Fragment Item Tag. Each Fragment advances
that offset by its 8-byte Item header plus even physical Value length. Both
table paths refuse an odd physical Fragment Value. A populated table must
contain one strictly increasing offset per frame and every offset must land on
a supplied Fragment boundary. One frame may therefore borrow several adjacent
Fragments.

An empty Basic Offset Table maps a single frame to all Fragments. For multiple
frames it accepts only the proven one-Fragment-per-frame case where the counts
match. Every other empty-table mapping reports unavailable boundary evidence
instead of guessing.

Extended Offset Table input follows PS3.3 C.7.6.3.1.8. It requires one
Fragment per frame, one 64-bit offset and length per frame, the same Fragment
Item Tag offset origin, and exact ordered boundaries. A returned Fragment view
excludes the one trailing Item pad byte when the encoded frame length is odd.
Frame lookup never clamps an out-of-range index.

Native Pixel Data has no per-frame offset table. After ingest retains the
complete native Value and `MultiframeMetadata` validates Number of Frames,
`ocelli-codec::NativeFrameIndex` owns its format-specific validation and frame
extraction. That boundary is required because native frames are concatenated
without per-frame padding and a later one-bit frame may begin in the middle of
a byte or word. The index validates complete-Value OB or OW physical storage,
then uses the declared frame count and descriptor to derive checked global bit
offsets without allocation. `Decoder::decode` itself receives one logical
frame and does not guess whole-Value padding from a frame slice.

## Provider order

`ProviderRegistry` is caller-owned and contains function pointers with stable
`ProviderId` values. Registration order is the complete precedence rule. A
lookup returns the first present answer together with the identity of the
provider that supplied it. A present empty element is still an answer and
stops fallback.

Duplicate provider identities are refused. A `ProviderId` is the only
registration identity because Rust function addresses have no reliable
comparison semantics. The same function can be registered under distinct
identities when the caller deliberately gives each registration a different
place in the precedence order. The registry has no implicit provider. The two
production functions are `data_set_provider`, which reads the parsed main data
set, and `file_meta_provider`, which reads File Meta Information. They expose
the same answer type while keeping the sources distinct.

## Dependencies and targets

D-18 selects the dicom-rs 0.10 component crates directly:
`dicom-core`, `dicom-object`, `dicom-encoding`, `dicom-parser`, and
`dicom-transfer-syntax-registry`. Defaults are disabled. Only the two
components that own whole-data-set deflate enable `deflate`. No pixel codec
feature is enabled by F-016.

`flate2` is a direct dependency using its Rust backend. dicom-rs uses the same
crate for its Deflate adapter, but the erased adapter exposes only `Read` and
cannot report how many encoded bytes the stream consumed. Ocelli uses the
decoder directly for this one selected route so it can require the exact
PS3.5 A.5 end of stream and mandatory NULL padding without inflating twice.

The dicom-rs graph requires `std`, so `ocelli-dicom` is not in the repository's
self-selected no-std set. It remains a shared crate that builds for native and
`wasm32-unknown-unknown`. The `native` gate checks both targets and proves that
direct dependency features resolve identically across them.

## Verification

`crates/ocelli-dicom/tests/part10.rs` uses hand-encoded byte layouts derived
from PS3.10 section 7 and PS3.5 annex A. It exercises every route, padding and
multiplicity retention, missing and unknown syntax UIDs, malformed metadata,
odd lengths, malformed encapsulation, partial headers, truncated values, and
completion-marker collision attempts. It also binds top-level native versus
encapsulated Pixel Data to the selected transfer syntax, requires OB or OW for
Native Format, requires OB plus a Basic Offset Table and at least one nonempty
Fragment for Encapsulated Format, and rejects missing Deflate padding or
trailing bytes. Native Format Pixel Data nested in a Sequence Item remains
available under an encapsulated syntax.

The ordinary workspace suite remains self-contained. `bin/ocelli.sh gate
corpus` first checks coverage, digests, and non-sensitive metadata for the
ignored corpus. It then runs the ignored Rust integration test in
`crates/ocelli-dicom/tests/corpus.rs`, which parses every manifest row and
compares the observed UID with the manifest declaration. Its codec integration
test also compares native little-endian, native big-endian, RLE, and Deflate
syntax rows with one synthetic uncompressed reference. Native Values are read
from their wire encoding, RLE is decoded from its sole frame Fragment, and
Deflate is verified after the ingest-owned data-set adapter. Failures expose
only row numbers, fixture categories, and transfer-syntax identifiers.

`crates/ocelli-dicom/tests/metadata.rs` adds hand-encoded PS3.5 fixtures for
present empty values, legal UI and text padding, significant leading ST space,
ordered DS components, signed SS and SL values, nested sequence items, and
encapsulated-fragment refusal. PS3.18 fixtures enumerate every permitted VR for
both binary carriers and reject disallowed VRs, empty Inline Binary, and
malformed base64. Property tests exercise primitive signedness and
multiplicity. Provider tests bind first-answer precedence, present-empty
stopping, identity reporting, and duplicate `ProviderId` refusal.

`crates/ocelli-dicom/tests/dicomweb.rs` adds PS3.18 fixtures for Annex F JSON,
one-part and many-part instance responses, direct WADO-URI Part 10 input,
encoded frame ranges, boundary-like payload bytes, malformed framing, media
type refusal, and patient-safe source errors.

`crates/ocelli-dicom/tests/multiframe.rs` constructs synthetic enhanced
metadata through `MetadataSet`. It checks the absent-only frame-count default,
invalid declarations, functional-group item counts, checked frame selection,
per-frame precedence, explicit top-level permission, retained source evidence,
and frame order. `crates/ocelli-dicom/tests/frame_index.rs` uses hand-computed
Fragment Item Tag offsets to check multi-Fragment Basic Offset Table frames,
empty-table evidence, Extended Offset Table lengths and pad removal, malformed
boundaries, truncated tables, and final frame bounds. The ignored corpus test
projects the exact manifest-backed enhanced CT row through `MetadataSet` and
`MultiframeMetadata`, then checks its frame count and distinct shared and
per-frame source labels without reading attribute values into the assertion.
