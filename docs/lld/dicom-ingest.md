# DICOM ingest

**F-IDs that contributed:** F-016, F-017
**Last updated:** 2026-09-09

`ocelli-dicom` owns the first ingest boundary. It accepts an in-memory DICOM
Part 10 file, resolves its declared transfer syntax, and returns the complete
dicom-rs object together with evidence of the parser route that produced it.
It projects metadata without collapsing absence, emptiness, value
multiplicity, signedness, or nested data sets. It does not decode compressed
pixel frames.

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
stories.

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

F-021 can construct the same model from DICOM JSON. Person Name component
objects, `BulkDataURI`, and `InlineBinary` have explicit typed constructors.
The constructors enforce the carrier VR sets in PS3.18 F.2.2. Inline binary
must be nonempty canonical padded base64 with valid unused bits. A present
empty DICOM JSON attribute uses `MetadataValue::Empty` as required by PS3.18
F.2.5. Neither binary carrier is flattened into an ordinary string value.

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
compares the observed UID with the manifest declaration. Failures expose only
row numbers and transfer-syntax identifiers.

`crates/ocelli-dicom/tests/metadata.rs` adds hand-encoded PS3.5 fixtures for
present empty values, legal UI and text padding, significant leading ST space,
ordered DS components, signed SS and SL values, nested sequence items, and
encapsulated-fragment refusal. PS3.18 fixtures enumerate every permitted VR for
both binary carriers and reject disallowed VRs, empty Inline Binary, and
malformed base64. Property tests exercise primitive signedness and
multiplicity. Provider tests bind first-answer precedence, present-empty
stopping, identity reporting, and duplicate `ProviderId` refusal.
