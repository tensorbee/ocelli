# Pixel pipeline

**Area**: `crates/ocelli-pixel`
**Normative source**: `docs/hld/13-core-types.md` sections 16 and 16.1,
`docs/hld/15-lut-chain.md` sections 18 through 18.3, DICOM PS3.3 C.7.6.2,
C.7.6.3, C.7.9 and C.11
**F-IDs that contributed:** F-018, F-029, F-030, F-041
**Last updated:** 2026-09-15

Living current-state document. It describes what the code does today.

## Ownership

`ocelli-pixel` owns all interpretation between decoded DICOM sample containers
and display values. No decoder interprets Bits Stored or signedness.

**No shader re-makes a LUT DECISION, and one shader does evaluate the
formulas.** That sentence read "no shader reimplements modality or VOI
arithmetic" until F-041, which made it false: `crates/ocelli-render/shaders/voi.wgsl`
evaluates stage 1, all three window functions and stage 3, because HLD section
18.4's uniform hands a shader `slope`, `intercept`, `center`, `width` and
`fn_kind`, and a shader given those has to evaluate something.

What exists once is every decision. The shader receives a resolved `invert`
flag and no Photometric Interpretation, one selected window pair and no
multiplicity, rescale values and no sequence, and a width this crate already
validated. It cannot re-decide any of them, which is a property of what the
uniform does not carry. `VoiParams::from_chain` refuses outright when a LUT
Sequence makes the uniform inexpressible, rather than substituting values the
sequence overrode.

The stages are:

```text
decoded container bytes -> Stored -> Modality -> Display -> Display
```

All four of PS3.3 C.11's stages are implemented. Stage 4 is **not** a fourth arm
on the chain above, and deviation **D-23** records why: no arm of it takes a
`Display` input. Palette colour maps the **stored** value through the palette
descriptors, per C.7.6.3.1.5's "the first stored pixel value mapped", and the
`RGB` and every `YBR_*` route map decoded samples that never entered the chain
at all. ICC, which HLD section 18's table names beside palette, is not
implemented.

```text
decoded container bytes -> Stored -> Modality -> Display -> Display    stages 1 to 3
decoded container bytes -> Stored ------------------------> Rgb        stage 4
```

**Stage 3 and stage 4 partition the photometric interpretations exactly.**
`PresentationTransform::new` refuses every colour space with
`PresentationLutNotApplicable`, and `ColorTransform::resolve` refuses both
monochrome ones with `ColorTransformNotApplicable`. A frame therefore reaches
exactly one of the two, never both and never neither, and that is asserted from
both sides rather than left as a reading of the two match statements.

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

**`sample_count` is sized from `SampleLayout::stored_samples_per_pixel`, not
from Samples per Pixel**, and the two differ for 4:2:2. PS3.3 C.7.6.3.1.2
subsamples the chroma two to one horizontally and stores each pair of pixels as
`Y1 Y2 Cb Cr`, so a frame is `Rows * Columns * 2` and not `* 3`. Samples per
Pixel stays 3, because that is what the data set carries and what
`SampleLayout::new` validated. Before F-030 the count was three per pixel, so
the `synthetic/us_ybr_full_422.dcm` corpus row could not be unpacked at all:
`unpack` demanded three bytes per pixel from a source holding two, and nothing
observed it. An odd `Columns` with a 4:2:2 interpretation is refused with
`SubsampledChromaAlignment`, before any length arithmetic, because half a
chroma group cannot be stored.

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

**F-041 built that shader and the accessors it reads through.**
`ModalityTransform::rescale` returns section 18.4's `slope` and `intercept`,
`VoiTransform::window` returns its `center`, `width` and function, and
`LutChain::modality` and `LutChain::voi` expose the two stages as shared
borrows. **All four add no arithmetic**: each returns state the type already
holds, which is the reading half of section 18's "implement it once, in
ocelli-pixel, and let the shader read the parameters".

Both accessors return `None` for a LUT Sequence, and that is not a gap.
Section 18.4's uniform has `slope`, `intercept`, `center` and `width` and no
field for a sequence, while section 18's stage table says a sequence TAKES
PRECEDENCE over those values. So there is nothing honest to return, and
`ocelli_render::VoiParams::from_chain` turns the `None` into a refusal rather
than substituting the values the sequence overrode. That is HLD section 31's
rule generalised by deviation D-07: the GPU path reports unavailable and tier C
runs the sequence through `LutChain::map_into` as it always did.

### The measured divergence between the shader and this crate

**0.000030517578**, the maximum absolute difference between
`ocelli_render::VOI_WGSL` on a real adapter and `LutChain::map_into`, over 4096
stored values spanning both clamps, all three VOI functions, inverted and not.
That is two `f32` ULP at 255, or one at 256, against an asserted bound of
`1e-4`. Recorded on `aarch64-apple-darwin`, Metal, tier A, by
`crates/ocelli-render/tests/voi_shader.rs::the_shader_agrees_with_ocelli_pixel_over_a_sweep`,
which prints it on every run.

**It is a measured divergence and not a bit-exactness claim**, which is decision
D14. WGSL's `exp` is not required to be correctly rounded and this crate's comes
from glam's libm backend, so SIGMOID can legitimately differ in the last places
on other hardware.

**And it is weaker than an oracle verdict.** Both sides are ours, so a shared
misreading of PS3.3 would agree with itself. What stops the comparison being
circular is that the section 18.3 rows, the boundary rows, the width-one rows
and the SIGMOID row are asserted on the GPU against values hand-computed from
PS3.3 rather than against anything this repository produced.

### An open finding against this crate's own arithmetic, found by F-041

**A legal LINEAR chain can return a value outside its declared output range, at
one input per window, on the CPU and the GPU identically.**

Measured in `f32`, centre `1024.5`, width `1.0003662109375`, range `[0, 255]`.
Both are accepted by `VoiTransform::new`. Then
`c' = 1024`, `w' = 0.00036621094`, and the upper breakpoint `fl(c' + w'/2)`
rounds to `1024.0002`. The body evaluated there is **297.50003** against a
declared `ymax` of 255, an overshoot of 42.5. The comparison `x > c' + w'/2` is
false at that input, so the clamp does not fire and the body runs.

**The cause is that PS3.3 C.11.2.1.2's formula in single precision does not
reproduce its own breakpoint.** `(fl(c' + w'/2) - c') / w'` is not exactly
`0.5` when `w'` is small relative to `c'`. At the parameters above the quotient
is `0.6666667`.

**Only the UPPER breakpoint can escape**, because the comparisons are
asymmetric. The lower one is `<=`, so at `x == lower` the clamp fires and the
body is never evaluated. The upper one is `>`, so at `x == upper` it does not.
That is one input per window.

**NO PERCENTAGE IS GIVEN HERE, DELIBERATELY.** An earlier version of this
paragraph said 83 per cent of randomly drawn legal parameter pairs, which was a
figure measured for a different question, how often the lower OPERATOR is
observable, and which cannot describe this one, because the lower side never
leaves the range. The F-041 review then measured the right question at 28.3 per
cent and this author measured it at 50.0 per cent, both over 1,500,000 draws,
differing only in how `c` and `w` were distributed. A rate that moves by that
much with the sampling frame is a fact about the sampler, and quoting one
without its frame is the shape `CLAUDE.md` names as this repository's repeating
failure. The worked example above is exact and reproduces, and that is the claim
this section makes.

**It is not F-041's and F-041 did not touch it.** The shader reproduces
`LutChain::apply` to within two `f32` ULP over F-041's sweep, and **at these
particular parameters the two agree bit for bit**, `0x4394c001` on both, so both
sides overshoot together and by the same amount. The arithmetic is this
crate's, from F-018.

**It is not fixed here and no tolerance was widened to hide it.** Fixing it
means clamping the result to `[ymin, ymax]` after the body, or evaluating the
comparison in a wider type, and either is a change to the pixel arithmetic that
needs its own design plan, its own fixtures and its own oracle verdict. What
exists today is this paragraph and the numbers to reproduce it.

**The uniform's layout is `ocelli-render`'s and its values are this crate's.**
`ocelli-pixel` stays `no_std`, gains no `#[repr(C)]` struct and learns nothing
about bindings. What stops the two halves disagreeing is that `ymin` and `ymax`
reach the uniform from `VoiTransform::output_range`, which is the same range
`LutChain::new` constructed the presentation stage with, so the shader's
reflection is about the range the inversion flag was resolved against.

A declared Presentation LUT Sequence `(2050,0010)` reports
`PresentationLutSequenceUnsupported` rather than falling back to the shape. No
corpus row carries one, so an implementation would be judged only by a fixture
asserting what the code does.

`PresentationTransform::new` reports three refusals in a fixed and asserted
order: a malformed output range, then a non-greyscale photometric
interpretation, then an unsupported sequence.

## Colour stage

`ColorTransform` implements PS3.3 C.7.6.3.1.2, C.7.6.3.1.3 and C.7.9. Its
output is `Rgb`, three `f32` channels, declared in `ocelli-pixel` beside its
only producer rather than added to HLD 16.1's listing of three scalar spaces.

### The resolution, which is what stops a second conversion

Two rules, each with one place to read them:

```text
colour space = Rgb                      when the decoder reports Rgb
               the data set's value      otherwise

layout       = Interleaved              when the decoder reports Interleaved
               Interleaved              when the encoding is Encapsulated
               the data set's value      otherwise
```

The first line is the guard. A JPEG decoder usually outputs RGB even though the
data set still says `YBR_FULL_422`, and a second conversion on top of the
decoder's own darkens and shifts hue on an image that still looks like an image.
F-024 added `DecodePhotometricInterpretation` and `DecodeSampleLayout` to
`ocelli-codec` to make that observable, and until F-030 every use of them was a
decoder declaring its own behaviour. **Nothing read them to decide anything.**
F-030 is the first stage that does, through the mirrored types below.

The second layout line is C.7.6.3.1.3's "required to be 0 when the Pixel Data is
encapsulated", so a data set declaring `1` for a JPEG frame is ignored. The
third is the same attribute honoured for a native syntax. Both directions are
asserted, because a test of the first alone cannot tell "ignored correctly" from
"never read at all".

`DecodedPhotometric` and `DecodedLayout` mirror the two `ocelli-codec` enums.
The crates do not depend on each other, `ocelli-pixel` carrying portable
arithmetic and `ocelli-codec` carrying five codec libraries, so the mapping
between them is owed by whichever story first wires a decoder's output into this
stage. `PixelRepresentation` already has this shape for the same reason.

### The routes, and the two that are refused

| Resolved space | What happens |
|---|---|
| `PALETTE COLOR` | three `LutDescriptor` lookups over the **stored** value |
| `RGB` | layout only |
| `YBR_FULL` | inverse full-range matrix |
| `YBR_FULL_422` | chroma replicated across the pair, then the full-range matrix |
| `YBR_PARTIAL_422` | chroma replicated, then the partial-range matrix |
| `YBR_ICT`, `YBR_RCT` | refused, `CodecOwnedColorTransform` |
| `MONOCHROME1`, `MONOCHROME2` | refused, `ColorTransformNotApplicable` |

PS3.3 permits `YBR_ICT` and `YBR_RCT` only with JPEG 2000, where the
codestream's own multiple component transform carries them.
`crates/ocelli-codec/src/jpeg2000.rs` refuses `mct != 0`, so neither can reach
this stage still in that space. RCT is an integer lifting transform and not a
matrix at all, so applying the full-range matrix to it would produce a plausible
image in the wrong colours. Under a decoder reporting `Rgb` the question does
not arise, because the resolution has already replaced the space.

The 4:2:2 upsample **replicates** rather than interpolating. PS3.3 names no
filter, so interpolation would invent values the standard does not define.

### The inverse matrices, and a measured divergence

PS3.3 C.7.6.3.1.2 states the `RGB -> YBR` direction only. Decoding needs the
inverse of a matrix whose coefficients the standard rounded to four decimals.
**The implementation inverts the matrix the standard states**, in exact rational
arithmetic, rather than using the textbook BT.601 inverse. Inverting the stated
equations is self-consistent by construction, because those equations are the
standard's definition of the encoding.

The two differ by at most **0.020028 of 255** over the 8-bit cube, at
`(Y, Cb, Cr) = (0, 0, 0)`. That is below one quantisation step and it is
measured rather than claimed to be zero, which is decision D14. The fixtures run
at a tolerance of `0.001`, twenty times tighter, so substituting BT.601
constants makes them red. That was verified by mutation rather than reasoned
about.

Two coefficients in each matrix are near zero and are not typographical. They
are the residue of the standard's rounding, and writing them as zero would be a
second rounding decision this stage is not entitled to make.

**Nothing is clamped to `[0, 255]`.** The rounded forward matrix is not exactly
normalised, so a saturated primary encodes to `Cb = 255.5`, and three of the
eight `YBR_FULL` fixture rows are negative on at least one channel. A stage that
clamped would hide that. Clamping belongs with the rounding decision at the
render or export boundary, which HLD 27.3 makes a review item wherever it lands.

### Palette adds no lookup arithmetic

`PaletteColorLut` is three `LutDescriptor`s and one cross-check. `LutDescriptor`
already validates all three of C.7.6.3.1.5's descriptor values, including the
zero-means-65,536 entry count, and its lookup already clamps below the first
mapped input and above the last entry, which is C.7.6.3.1.5's stated clamping.
HLD section 18 requires that arithmetic to exist exactly once and this was the
one place in stage 4 where a second copy was available for free.

The cross-check refuses three descriptors that disagree on entry count or first
mapped input. A palette whose channels mapped different input ranges would shift
hue against luminance, which no shape check on the result reveals.

**A known cost, recorded rather than fixed.** `LutDescriptor` materialises an
`inputs` ramp and binary-searches it, so a 16-bit palette holds 65,536 `f32`
inputs per channel beside its values, about 1.5 MB for three channels where
index arithmetic would need none of it. F-030 did not change it: the ramp is
shared with the modality and VOI stages, and replacing the search with index
arithmetic changes those two stages' behaviour at a fractional input. That is a
change to validated pixel arithmetic and needs its own story and its own
fixture.

## Failure boundary

`PixelError` reports malformed plane attributes, stored-pixel descriptors,
LUT descriptors, rescale or window evidence, length overflow and caller buffer
mismatches. These are local typed refusals. Nothing crosses the wasm boundary
and no `wasm-bindgen` type appears in this crate.

## Tiers

The arithmetic is identical for all runtime tiers. Tiers A and B consume the
CPU-prepared parameters and evidence, including `LutChain::inverts`, which is
HLD section 18.4's `invert : u32`. Tier C uses the same implementation as its
authoritative pixel path, entering it through `LutChain::map_into` and
`ColorTransform::map_into`. There is no tier-specific arithmetic copy.

The colour stage adds no shader, uniform or `#[repr(C)]` struct. HLD 18.4's
uniform covers stages 1 to 3 and says nothing about colour, so a palette texture
layout and a colour-matrix uniform belong to the rendering story that has a
reader for them.

`bin/ocelli.sh gate native` proves native linkage and shared-crate wasm
compilation. The focused `cargo check -p ocelli-pixel --all-targets --target
wasm32-unknown-unknown` isolates the pixel crate's wasm compile proof.
`bin/ocelli.sh gate wasm` builds `ocelli-wasm`, which does not currently depend
on `ocelli-pixel`, so it neither compiles nor executes this crate.
