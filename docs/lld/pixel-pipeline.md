# Pixel pipeline

**Area**: `crates/ocelli-pixel`
**Normative source**: `docs/hld/13-core-types.md` sections 16 and 16.1,
`docs/hld/15-lut-chain.md` sections 18 through 18.3, DICOM PS3.3 C.7.6.2,
C.7.6.3 and C.11
**F-IDs that contributed:** F-018, F-029
**Last updated:** 2026-09-13

Living current-state document. It describes what the code does today.

## Ownership

`ocelli-pixel` owns all interpretation between decoded DICOM sample containers
and display values. No decoder interprets Bits Stored or signedness. No shader
reimplements modality or VOI arithmetic.

The stages are:

```text
decoded container bytes -> Stored -> Modality -> Display -> Display
```

The three implemented stages are PS3.3 C.11's first three. Palette colour and
ICC execution, which is C.11's stage 4, are not implemented. That stage maps the
**stored** value through the palette descriptors rather than a `Display` value,
so it is not a fourth arm on this chain.

The crate is `no_std`. It uses `alloc` only while a LUT descriptor takes
ownership of setup-time data and constructs its input keys. Scalar mapping,
slice mapping, stored-value extraction and transform application allocate
nothing. Slice mapping and stored-value extraction write into storage supplied
by the caller and reject a length mismatch before the first write.

## Image-plane evidence

Four validated types keep equal-shaped DICOM attributes distinct:

| Type | Validation |
|------|------------|
| `ImagePositionPatient` | Three finite LPS-mm components |
| `ImageOrientationPatient` | Two finite, unit-length and mutually orthogonal direction vectors within a dimensionless `2e-6` input tolerance |
| `PixelSpacing` | Finite nonnegative row and column spacing with named accessors |
| `ImageDimensions` | Nonzero Rows and Columns |

`ImagePlane::index_to_world` implements DICOM PS3.3 C.7.6.2.1.1. Transform
column one is the row direction vector multiplied by column spacing. Transform
column two is the column direction vector multiplied by row spacing. The
translation is Image Position Patient, which is the centre of index
`(0, 0, 0)`.

Image Orientation Patient values are dimensionless direction cosines. If each
component is rounded to six decimal places, its error is at most `0.5e-6`.
Cauchy-Schwarz bounds the accumulated dot error below
`2 * sqrt(3) * 0.5e-6 + 3 * (0.5e-6)^2`, about `1.733e-6`. The constructor's
`2e-6` input tolerance is a conservative bound for both unit-length error and
the absolute row-column dot product. Accepted vectors are normalised, then the
column vector is orthogonalised against the row vector before storage.
Accepted rounding noise therefore cannot survive as scale or shear in the
transform.

DICOM PS3.3 section 10.7.1.3 permits a zero spacing value only for its
corresponding singleton image dimension. `PixelSpacing` admits zero
provisionally because it has no dimensions. Fallible `ImagePlane::new` permits
zero row spacing only when Rows is one and zero column spacing only when
Columns is one. Negative and non-finite spacing is always refused. Spacing and
dimensions are private fields so construction and later mutation cannot bypass
the contextual invariant.

The third matrix column is the normalised cross product of the two direction
vectors. DICOM's image-plane equation has no slice step, but the nonzero normal
makes the four-by-four transform invertible without changing any point whose
slice index is zero. A later volume builder remains responsible for replacing
that unit normal with measured inter-slice geometry.

A legal zero spacing belongs to a singleton axis whose only valid index is
zero. The transform uses a unit vector on that otherwise unused axis. This
does not move any valid voxel centre and keeps the transform invertible for
typed coordinate conversion.

The fixture uses row spacing `2.0` and column spacing `0.5`. This is deliberate.
A square-pixel fixture cannot observe a spacing-index swap.

## Stored values

`SampleLayout` validates the relationship between Samples per Pixel,
Photometric Interpretation and Planar Configuration. Monochrome and palette
layouts require one sample and no Planar Configuration. RGB and YBR evidence
require three samples and an explicit planar layout.

`StoredBits` accepts only 8, 16 and 32-bit containers. Bits Stored must be
nonzero and no wider than the container. High Bit must equal Bits Stored minus
one, which is the current PS3.3 C.7.6.3.3 rule. The legacy left-aligned corpus
case remains interoperability evidence and is not an accepted descriptor.

`StoredPixelDescription::unpack` reads little-endian or big-endian containers,
shifts to the High Bit alignment, masks Bits Stored and sign-extends from
`BitsStored - 1`. The destination is `Stored`, whose HLD-defined representation
is `f32`. Every integer through 24 significant bits is exact. Wider 32-bit
integers round to the nearest representable `f32`, which follows the chosen
value-space type rather than an extra conversion policy.

The unpacker operates on decoded sample containers. A codec that expands
subsampled colour or converts YCbCr to RGB must describe its actual output
layout before this stage. It must not pass an on-wire subsampled byte count as
three expanded samples per pixel.

## Modality stage

`ModalityTransform::new` selects a `LutDescriptor` immediately when one is
present. Rescale Slope and Rescale Intercept are ignored in that case. Without
a LUT, both finite rescale values are required and the mapping is exactly:

```text
Modality(stored * slope + intercept)
```

A LUT Descriptor validates an 8 or 16-bit entry size, the DICOM zero-means-
65,536 entry count, finite data and a first mapped input in the signed or
unsigned 16-bit range. Each LUT Data value must be an unsigned integer no
greater than 255 for an 8-bit descriptor or 65,535 for a 16-bit descriptor.
Negative, fractional and overflowing values are refused. Lookup clamps below
the first mapped input and above the last entry. Descriptor construction owns
its data. Lookup borrows it.

## VOI stage

`VoiTransform::new` gives a VOI LUT Sequence precedence over all window
evidence. Without a sequence, Window Center and Window Width must have equal,
nonzero multiplicity and the selected index must exist. `LINEAR` requires
width at least one. `LINEAR_EXACT` and `SIGMOID` require positive width.

The implementation transcribes DICOM PS3.3 C.11.2.1:

- `LINEAR` uses `c - 0.5` and `w - 1`
- `LINEAR_EXACT` uses `c` and `w` directly
- both linear functions use `x <= lower` and `x > upper`
- `SIGMOID` uses exponent `-4 * (x - c) / w`

Deviation D-13 is applied to the HLD section 18.3 fixture.
`LINEAR_EXACT(-160)` at centre 40 and width 400 is `0.000`, not `1.594`.
The formula and boundary comparison both produce zero.

`VoiTransform::output_range` reports the stage's declared output range, which
the presentation stage needs. A window reports the `[ymin, ymax]` it was
validated with. A VOI LUT Sequence reports DICOM PS3.3 C.11.2.1.1's
`0 ..= 2^bits - 1`, taken from the descriptor's third value rather than from the
largest entry present. `LutDescriptor` retains that bound from construction, so
the entry-size-to-range mapping exists once, inside the validation that refuses
every other entry size.

## Presentation stage

`PresentationTransform` implements DICOM PS3.3 C.11.6. `Identity` is a no-op
rather than an arithmetic round trip, so an identity chain is bit-identical to
the VOI stage alone. `Inverse` is the reflection `ymin + ymax - y` within the
declared output range. Writing `ymax - y` is correct only when `ymin` is zero,
which is why the fixture uses a `16` to `236` range.

**Inversion has two possible sources and is resolved exactly once**, in
`PresentationTransform::new`. `PresentationLutEvidence` names all three states
PS3.3 C.11.6 permits, so "no shape declared" stays distinguishable from "a shape
declared IDENTITY", which is what the rule turns on:

| Presentation LUT Shape `(2050,0020)` | Photometric Interpretation | Resolved |
|---|---|---|
| `INVERSE` | any monochrome | `Inverse` |
| `IDENTITY` | any monochrome | `Identity` |
| absent | `MONOCHROME1` | `Inverse` |
| absent | `MONOCHROME2` | `Identity` |
| any | palette, RGB or any `YBR_*` | refused |

**An explicit shape decides alone and is never composed with `MONOCHROME1`.** A
`MONOCHROME1` frame carrying an explicit `IDENTITY` is therefore not inverted,
which is the presentation state being honoured and reads as a bug to anyone
expecting `MONOCHROME1` to always invert. Composing the two with an exclusive-or
is the shape that can invert twice, and a double inversion is invisible at the
midpoint of the output range, which is exactly where `LINEAR_EXACT` maps the
window centre. The fixtures therefore assert an input away from the centre.

`LutChain` composes stages 1 to 3 in PS3.3 C.11's order and adds no arithmetic
of its own. Its reason to exist is the resolution above, plus `inverts()`, which
is the single flag HLD section 18.4's `invert : u32` uniform carries. A shader
reads the resolved flag and does not combine evidence itself.

A declared Presentation LUT Sequence `(2050,0010)` reports
`PresentationLutSequenceUnsupported` rather than falling back to the shape. No
corpus row carries one, so an implementation would be judged only by a fixture
asserting what the code does.

`PresentationTransform::new` reports three refusals in a fixed and asserted
order: a malformed output range, then a non-greyscale photometric
interpretation, then an unsupported sequence.

## Failure boundary

`PixelError` reports malformed plane attributes, stored-pixel descriptors,
LUT descriptors, rescale or window evidence, length overflow and caller buffer
mismatches. These are local typed refusals. Nothing crosses the wasm boundary
and no `wasm-bindgen` type appears in this crate.

## Tiers

The arithmetic is identical for all runtime tiers. Tiers A and B consume the
CPU-prepared parameters and evidence, including `LutChain::inverts`, which is
HLD section 18.4's `invert : u32`. Tier C uses the same implementation as its
authoritative pixel path, entering it through `LutChain::map_into`. There is no
tier-specific arithmetic copy.

`bin/ocelli.sh gate native` proves native linkage and shared-crate wasm
compilation. The focused `cargo check -p ocelli-pixel --all-targets --target
wasm32-unknown-unknown` isolates the pixel crate's wasm compile proof.
`bin/ocelli.sh gate wasm` builds `ocelli-wasm`, which does not currently depend
on `ocelli-pixel`, so it neither compiles nor executes this crate.
