# DICOM ingest

**F-IDs that contributed:** F-016, F-017, F-019, F-020, F-021, F-025, F-211, F-234, F-225
**Last updated:** 2026-11-09

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
   elements are refused. Sequence nesting deeper than `MAX_SEQUENCE_DEPTH` is
   refused as `SequenceTooDeep` at the push of a Sequence container, before
   dicom-rs's recursive collection runs.
6. Append a fixed completion marker and collect the data set once under the
   same resolved syntax. Require and remove the marker before returning the
   object.

There is no retry with Explicit VR Little Endian and no alternate-syntax
fallback.

## Sequence depth bound

PS3.5 section 7.5 sets no bound on nesting, so this build sets one:
`MAX_SEQUENCE_DEPTH`, re-exported beside `parse_part10`. The structural
preflight counts open Sequence containers and refuses one more than the
constant. Items are not counted, because an item cannot open without a
sequence, and encapsulated pixel data cannot nest. The refusal is typed and
carries no depth, value or tag.

The constant is a power of two at least four times the deepest nesting
measured in the corpus, in the output or fixture of every repository DICOM
writer, Rust and TypeScript, and in the PS3.16 template shapes those writers
target, each wrapped once more by de-identification's Original Attributes
Sequence, chosen so a parse at the limit succeeds on a 2 MiB debug test
thread. The deepest known shape is the PS3.16 2026d measurement chain, at 8
open Sequences and 10 wrapped, so the value is the next power of two at or
above 4 times 10, which is 64. The chain runs from the TID 1500 row 6
"Imaging Measurements" CONTAINER and its row 9 inclusion of TID 1501,
through TID 1501 row 10's TID 300 measurement and TID 300 row 1b's TID
301, to TID 301 row 11's INFERRED FROM of TID 315 "Equation or Table" with
TID 315 row 1's CODE and row 2's HAS PROPERTIES NUM, whose Measured Value
Sequence (0040,A300) of PS3.3 C.18 carries the Measurement Units Code
Sequence (0040,08EA) and the Type 3 Equivalent Code Sequence (0008,0121)
of PS3.3 Table 8.8-1, wrapped in the Original Attributes Sequence
(0400,0561) through the Modified Attributes Sequence (0400,0550) of PS3.3
C.12.1.1.9. The templates are Extensible, so this is the deepest shape
known, not a ceiling. Re-measured on 2026-11-07: the corpus is 2 deep over
100 DICOM files, the TID 1500 document `writeTid1500` writes is 5 deep, and
the deepest shape fixture is 10 wrapped. The key-object writer's 2 and the
secondary-capture and synthetic slice writers' 0 are the 2026-11-06
measurements of the same unchanged TypeScript source, not re-measured in
this round. The measured inputs and the chosen value are recorded in the
F-234 handoff. A corpus row
or fixture that nests deeper than a quarter of the constant returns the
story to the lead rather than widening the constant in place. dicom-rs 0.10
offers no depth option, and its collector recurses, which is why the bound
sits in the preflight rather than beside it.

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
| `SequenceTooDeep` | Sequences nest deeper than `MAX_SEQUENCE_DEPTH` |

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

## Conformance report

`ocelli_dicom::check_conformance(&ParsedDicom)` produces the report, and
`to_json` owns its wire form. The check is advisory and on demand. It runs
after a successful parse, performs no decode, and never gates image
display. Producing a report in the browser would need a wasm build that
links the parser, and this build does not supply one. Measurements are
recorded in [the benchmark LLD](benchmarks.md).

The rule catalogue is produced by
`crates/ocelli-dicom/tests/conformance_typegen.rs`, which writes
`conformance.generated.ts` into `packages/core/src`. That file is distinct
from the boundary `generated.ts`. The VM source is `conformance/vm.rs`, 47
tags transcribed from PS3.6 2026d Table 6-1. It is not the full PS3.6
dictionary, and it is cross-checked against the dictionary VRs. Encoded UN
content is counted and is not value checked. The
[public report reader](public-api.md#conformance-report-reader) lists the
rule identifiers by family. `DS-RANGE` names the binary64 range limit, not
a violation of the decimal-string grammar.

The report carries this exact `CONFORMANCE_SCOPE` sentence:

> This report checks the rules it lists and nothing else. It is not a statement that the file conforms to DICOM.

Module presence covers Patient, General Study, General Series, General
Image, Image Plane, Image Pixel and SOP Common for the four IODs below.
Classic CR Image Storage has six, all but Image Plane. Classic CT, MR and PET Image Storage
have all seven. Other SOP classes, including the Enhanced variants, do not
evaluate module presence. Their value, VR and limited VM rules still
apply. This is not full DICOM conformance.

The decidable 1C and 2C conditions are the ones the approved plan names.
`PlanarConfiguration` when `SamplesPerPixel` is greater than 1. The six
palette elements for PALETTE COLOR. `PixelData` when `PixelDataProviderURL`
is absent. The EOT lengths when EOT is present. `SpecificCharacterSet`
when relevant text uses the expanded or replacement repertoire.
`PatientPosition` for CT and MR when (0054,0410) is absent.
`PatientOrientation` for CR. Not checked are `Laterality`, `ContentDate`
and `ContentTime`, `AnatomicalOrientationType`, `PixelAspectRatio`,
`PixelPaddingRangeLimit` and the `PixelDataProviderURL` condition itself,
plus the nonhuman, alternative calendar, deidentification, encryption,
HL7, `QueryRetrieveView` and conversion rows. The distinction matters. An
absent URL decides `PixelData`, while the condition that requires a URL is
not decided.

A finding carries tag, keyword, path, rule and section, with an optional
value ordinal and an optional measure. It never carries the original
attribute value, a substring of it or a character code from it. `path` is a
list of sequence steps. Each step's `item` and a finding's `value` are
1-based item and value ordinals, not contents. A character measure carries
a 0-based `index` and a character `class`. Retained numeric lengths and
counts are actionable. The module scope of an unevaluated SOP class may
repeat a well-formed UID under the DICOM root. Invalid UIDs and UIDs
outside that root are not repeated. The field is null with a closed note, or just null
when absent or empty. This selector is separate from a finding's value
ordinal and does not permit arbitrary attribute contents in findings.

`POST /conformance` accepts the original raw Part 10 body, or an
`X-Ocelli-File` id with an empty body. Both together are a 400. An unknown
id, or one whose encoded bytes have retired, answers 410. Bytes that are
not Part 10 get the
refusal `/headers` gives. The route performs no decode and touches no held
frame. Encoded bytes can retire after decode, for CT and DX alike, while
the decoded held frames remain. The viewer asks with the id first. On 410
it retries once with the original File as the `/conformance` body. It does
not repost to `/file` and does not create a new held id.

The page asks for the report of the file the active viewport shows once
that file has stayed stable for 300 ms. Delayed background requests wait
during file posts and thumbnails. One request is in flight at a time. Reports are cached
against the File object, not against a retired native id, at most 512 per
folder, oldest dropped first. A new folder resets the cache, and replies
from a stale generation are dropped. A sheet open can request immediately.
Findings never gate display. The badge stays hidden until the shown file
has findings above zero, and an absent badge is not a conformance
certificate. A missing library build shows:

> The checks need the library build. Run npm run build in the repository, then start the viewer again.

If the library entry is served but the library cannot load, it shows:

> The checks need dcmjs, which the library imports. Run npm ci in the repository, then start the viewer again.

## Derived frame geometry

F-020 is the first module in the programme that computes a coordinate from a tag
rather than retaining one, so it cannot be judged on losslessness. Every
derivation cites the PS3.3 section it implements and a value that cannot be
derived is refused rather than defaulted.

**The arithmetic is not here.** `ocelli-pixel` owns `ImagePlane` and PS3.3
C.7.6.2.1.1's index-to-world transform, and this module assembles validated
attributes into those existing types. `FrameGeometry::index_to_world` forwards.
`ImagePlane::slice_normal` and `position_vector` exist so a cross-frame consumer
can project an inter-frame vector without re-deriving the cross product.

Image Position Patient `(0020,0032)`, Image Orientation Patient `(0020,0037)`
and Pixel Spacing `(0028,0030)` resolve per-frame, then shared, then top level,
which is PS3.3 C.7.6.16.1.1's order. The top-level fallback is permitted because
a legacy single-frame instance carries all three there and has no functional
groups at all. A duplicate declaration is malformed under C.7.6.16.1.1 and is
**retained observably** in `FrameGeometrySources::duplicates` rather than
refused, which is the choice F-019 already made for the same condition.

Imager Pixel Spacing `(0018,1164)`, Gantry/Detector Tilt `(0018,1120)`, Rows and
Columns belong to no functional group macro and are read from the main data set
only.

### Spacing precedence, PS3.3 10.7.1.1

| Present | `SpacingRelationship` | Spacing used |
|---------|----------------------|--------------|
| Pixel Spacing only | `ImageOnly` | Pixel Spacing |
| both, equal within `1e-9 mm` (`ocelli_core::spacing_pair_agrees`) | `ImagerAgrees` | Pixel Spacing |
| both, different | `Calibrated` | Pixel Spacing, Imager retained |
| Imager Pixel Spacing only | refused, `UncalibratedSpacingOnly` | none |
| neither | refused, `MissingPixelSpacing` | none |

**The fourth row is the load-bearing one.** PS3.3 C.8.11.4: Imager Pixel
Spacing is measured at the front plane of the detector housing and is never
adjusted for geometric magnification. PS3.3 C.8.7.5's magnification factor
relates it to the patient plane. Substituting it would give a geometry that
renders correctly and measures in the detector plane. The caller decides.
PS3.3 10.7.1.1 says that an uncorrected Pixel Spacing has the same values
as Imager Pixel Spacing and a corrected one differs, which the second and
third rows record.

Slice Thickness `(0018,0050)` and Spacing Between Slices `(0018,0088)` are
retained as evidence and are never used as inter-frame spacing. Thickness is the
reconstructed slab thickness and may overlap or gap, and Spacing Between Slices
is frequently absent and frequently wrong.

### Stack shear, measured rather than read

`StackGeometry` projects each inter-frame step onto the slice normal
`N = X cross Y`. A stack is `AxisAligned` when every off-axis component is within
tolerance and the projected steps agree. The reported `step_mm` is the
**signed** projection, so a descending stack is negative. An off-axis
stack whose projected steps agree is `Sheared` with the largest
off-axis component, which means the volume is a sheared parallelepiped rather
than a box.

**Gantry/Detector Tilt decides nothing.** PS3.3 C.8.7.3.1.1 makes it the nominal
angle and Type 3. It is retained as evidence, and fixtures assert both
directions: a tag of `0` with sheared geometry reports `Sheared`, and a tag of
`30` with axis-aligned geometry reports `AxisAligned`.

A stack whose projected steps are not equal within tolerance is
**refused** as `NonUniformFrameSpacing` rather than averaged. The refusal
includes a sheared stack, and it is decided before that verdict. Non-uniform
spacing is real in dose-modulated and multi-slab acquisitions, and rendering it
as uniform distorts geometry in a way no measurement tool flags.

A later frame whose Pixel Spacing differs from the first is refused as
`NonUniformPixelSpacing`. The positions can still lie on one normal. Later
slices would be measured with the first frame's in-plane spacing.

A stack whose every projected step is zero within tolerance is refused as
`CoincidentFrames`. The frames occupy one plane. A shift inside that plane is
the same refusal, and it is decided before a sheared verdict. A volume built
from that step is singular. A zero step beside a real step stays
`NonUniformFrameSpacing`.

Cross-instance slice spacing over a series is not here. Within one multiframe
instance every frame's position comes from the same instance, which is what
makes the step derivable. Sorting and spacing a series of separate instances
belongs to the volume builder.

### The two tolerances, both declared once

`GEOMETRY_TOLERANCE_MM` is `1e-6`. It bounds both the off-axis component before
a stack is called sheared and the agreement between projected steps before it is
called uniform. Image Position Patient arrives as a Decimal String, which PS3.5
section 6.2 caps at sixteen characters, and a patient coordinate routinely
carries three or four digits before the point, so the attribute can express
about eleven decimal places at the very most and real files carry four to six.
The figure is therefore below the precision the source attribute can state,
which is the property wanted: it absorbs `f64` arithmetic noise and can never
mask a difference a file could actually express.

`SPACING_EQUALITY_MM` is `1e-9` and is not a physical tolerance. It lives in
`ocelli_core::spacing`, with `spacing_pair_agrees`, since F-211. `ImagerAgrees`,
in-plane agreement between frames and series, and `ocelli_geom::select_calibration`'s
PS3.3 10.7.1.1 test all call that one function. Text that differs only in
trailing zeros or exponent form already parses to the same `f64`. What the
tolerance absorbs is a difference a 16-byte Decimal String can still state, such
as `0.1430000001` against `0.143`, about 1e-10 mm.

### Decimal String parsing

`parse_decimal_string` implements PS3.5 section 6.2: an optionally signed
decimal with an optional fraction and an optional exponent, at most sixteen
bytes including the pad. `f64::from_str` performs the conversion and accepts
three spellings the grammar does not. `inf` and `nan` are refused by the
finiteness check. Rust's `1_0` digit separator is refused by a character filter
that runs **before** the parse rather than relying on the parser. A leading `+`
is legal DS and both accept it. The sixteen-byte cap is checked before trimming,
because the pad is part of the value.

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
Content, rescale, window and real-world mapping values remain lossless
metadata that nothing here interprets. It does not sort frames by Dimension
Index. **Derived frame geometry is below**, and it is the one thing F-020 moved
out of this paragraph: plane position, plane orientation and pixel measures are
now interpreted, and gantry tilt is measured rather than read.

`EncapsulatedFrameIndex` groups borrowed Fragment Values without concatenating
or decoding them. Basic Offset Table entries are checked against Fragment Item
Tag offsets measured from the first Fragment Item Tag. Each Fragment advances
that offset by its 8-byte Item header plus even physical Value length. Both
table paths refuse an odd physical Fragment Value. A populated table must
contain one strictly increasing offset per frame and every offset must land on
a supplied Fragment boundary. One frame may therefore borrow several adjacent
Fragments.

An empty Basic Offset Table maps a single frame to all Fragments, and several
frames to one Fragment each when the counts match, with no marker read. F-244
adds the case PS3.5 8.2 requires and A.4 leaves without an algorithm, more
Fragments than frames. `FrameDelimiter::for_transfer_syntax` owns the
delimiter table: `JpegMarkers` for `.50`, `.51`, `.57`, `.70`, `.80` and `.81`
(A.4.1 and A.4.3), `Jpeg2000Markers` for `.90`, `.91`, `.201`, `.202` and
`.203` (A.4.4, HTJ2K read leniently because it shares the Part 1 markers), and
`Unmarked` for RLE, JPEG XL and any other UID. Frame 0 starts at Fragment 0.
Each frame ends at the first Fragment whose logical suffix through its Value
ends `FF D9`, or `FF D9 00` with A.4's one pad byte. The next frame starts at
the following Fragment, whose logical prefix across Fragment Values must
begin `FF D8 FF` or `FF 4F FF 51`. Either marker may cross an even Fragment
boundary. Only a bounded prefix and three suffix bytes are inspected, without
joining a frame. An empty-table frame count greater than the Fragment count
is refused before reservation. Marker ranges use fallible reservation, with
`AllocationFailed` on allocation failure. Exactly the declared number of
frames must be found, the last ending at the last Fragment.
Anything else, and every `Unmarked` case, is `BoundaryUnavailable`, so an
`FF D9` inside a header segment refuses the file rather than handing a decoder
the wrong bytes. The populated-table and equal-count paths read no markers.

`EncapsulatedFrameIndex::from_parsed` is the one constructor both production
callers use, the folder viewer preview and the oracle producer. It reads the
top-level Pixel Data, refusing native or absent Pixel Data as
`NotEncapsulated`. When Extended Offset Table `(7FE0,0001)` is present it
requires an empty Basic Offset Table, refusing `ExtendedWithBasicOffsets`
otherwise as A.4 forbids both, requires Extended Offset Table Lengths
`(7FE0,0002)` and both as `OV` 64-bit values, refusing
`ExtendedOffsetTableUnreadable` otherwise, and builds through `from_extended`.
Without it, it builds through `from_basic` with the transfer syntax's
delimiter. It looks elements up with `get`, because dicom-object's `element`
error captures a backtrace, about 4 microseconds per absent tag, measured.

Extended Offset Table input follows PS3.3 C.7.6.3.1.8. It requires one
Fragment per frame, one 64-bit offset and length per frame, the same Fragment
Item Tag offset origin, and exact ordered boundaries. A returned Fragment view
excludes the one trailing Item pad byte when the encoded frame length is odd.
Frame lookup never clamps an out-of-range index.

### cornerstone3D 5.8.2 and decision D14

The floor is `@cornerstonejs/dicom-image-loader` 5.8.2 with `dicom-parser`
1.8.21, read from `tools/oracle/node_modules`. It reads an empty table as
empty, takes Fragment `i` for frame `i` when the counts match, and otherwise
ends each frame at the first Fragment whose last bytes hold `FF D9`. It never
reads the Extended Offset Table. Ocelli opens every case it opens, except two
named departures, both on input PS3.5 or PS3.3 call non-conformant:

- An RLE or JPEG XL frame split across Fragments, which A.4.2 and A.4.12
  forbid. cornerstone3D gives frame 0 every Fragment and throws for later
  frames. Ocelli refuses the file as `BoundaryUnavailable`, because neither
  codestream has an end marker to prove a boundary.
- An Extended Offset Table beside a populated Basic table, or one that
  disagrees with the Fragments. cornerstone3D ignores it. Ocelli refuses by
  name, because a table that disagrees with the Fragments means one of the two
  is wrong.

One case is better than the floor: an `FF D9` at the end of a Fragment that
is not the frame's end, followed by a Fragment with no start marker, is
refused rather than handed to a decoder. These are behaviours read from the
two sources, not measured divergence. The oracle measurement over the new
corpus rows is F-244's AC22, run on a GPU host.

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

F-244 vendors `dicom-object` 0.10.0 at `vendor/dicom-object-0.10.0` with one
recorded source patch, `first = false;` at the end of the `ItemEnd` arm of
`build_encapsulated_data` in `src/collector.rs`. The published collector
cleared its first-item flag only when an Item value was read, so the first
Fragment after a zero-length Basic Offset Table was read as the table, at the
top level and inside an Icon Image Sequence item. 0.10.1 and upstream `master`
carry the same code. The pins gate's `check_dicom_object_vendor` holds the
inventory, the patched file's digest and `PATCH-PROVENANCE.md`. D-18 records
that a published `ocelli-dicom` would lose the patch, so it is not published
until a dicom-object release carries the fix, when a follow-up story removes
the vendor directory and returns the dependency to the registry.

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
available under an encapsulated syntax. The depth fixtures parse at
`MAX_SEQUENCE_DEPTH`, refuse one past it on every little endian form, refuse
100,000 nested Sequences on a 256 KiB stack, refuse a deflated data set one
past the limit, and run a proptest over per-level length forms. F-244 adds PS3.5 Table A.4-1 and A.4-2
shapes by hand: an empty table then Fragments of six, eight, and four, six and
two bytes, each asserted as an empty table and exactly those Fragments, a
populated two-frame table returned unchanged, and an Icon Image Sequence item
whose encapsulated Pixel Data has an empty table and a six-byte Fragment.

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
boundaries, truncated tables, and final frame bounds. F-244 adds the marker
rule for JPEG and JPEG 2000 frames spanning Fragments, its five refusals
including a false end marker, equal counts reading no marker, the delimiter
table against A.4.1 to A.4.12, and `from_parsed` over a hand-built Extended
Offset Table with its wrong offset, wrong length, populated Basic table and
missing Lengths refusals. The ignored corpus test also decodes the nineteen
empty-table rows through `from_parsed`, each single-frame row equal to its
source row's decode and each multi-frame row frame by frame equal to the
uncompressed reference, reversed by columns and reversed by rows. The ignored corpus test
projects the exact manifest-backed enhanced CT row through `MetadataSet` and
`MultiframeMetadata`, then checks its frame count and distinct shared and
per-frame source labels without reading attribute values into the assertion.
