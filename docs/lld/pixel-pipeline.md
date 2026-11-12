# Pixel pipeline

**Area**: `crates/ocelli-pixel`
**Normative source**: `docs/hld/13-core-types.md` sections 16 and 16.1,
`docs/hld/15-lut-chain.md` sections 18 through 18.3, DICOM PS3.3 C.7.6.2,
C.7.6.3, C.7.9 and C.11
**F-IDs that contributed:** F-018, F-029, F-030, F-041, F-044, F-046, F-216, F-229, F-239, F-240, F-227
**Last updated:** 2026-11-11

Living current-state document. It describes what the code does today.

## Ownership

`ocelli-pixel` owns all interpretation between decoded DICOM sample containers
and display values. No decoder interprets Bits Stored or signedness.

**No shader re-makes a LUT DECISION, and one shader does evaluate the window
formulas.** That sentence read "no shader reimplements modality or VOI
arithmetic" until F-041, which made it false: `crates/ocelli-render/shaders/voi.wgsl`
evaluated stage 1, all three window functions and stage 3, because HLD section
18.4's uniform hands a shader `slope`, `intercept`, `center`, `width` and
`fn_kind`, and a shader given those has to evaluate something. F-235 moved
stage 1's evaluation back out of the shader: `voi.wgsl` now reads stage 1's
RESULTS, from a folded modality table the graph fills with this crate's own
`modality` outputs or from bytes a CPU pre-apply staged, and the shader reads
neither of the uniform's `slope` and `intercept`. The
window functions and stage 3 are still evaluated there, from `center`,
`width` and `fn_kind`.

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
at all. ICC, which HLD section 18's table names beside palette, is implemented
for the matrix and tone-curve subset by F-240 and described under The ICC
subset below. Every other profile stays under deviation **D-30** until F-145.

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

`LutChain::stored_sample_table` builds one display value for every integer
stored sample a `StoredBits` width of at most 16 allows. Each entry is
`LutChain::apply` of that bit pattern, so the table adds no arithmetic.
`StoredSampleTable::map_containers` selects the caller's already quantised
byte for each container. `StoredBits::meaningful_bits` and
`StoredBits::stored_sample` are the PS3.3 C.7.6.3 mask and signed reading
that the table and the per-sample path already used. Bits Stored above 16
returns no table. F-216 uses the table so a window change does not walk
`display_index` once per pixel.

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
chroma group cannot be stored. `ColorTransform::map_into` applies the same
refusal to the column count it is given. An even pixel count with odd columns
still pairs the last pixel of one row with the first pixel of the next.

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

Window presets are not a stage and they do not live in this crate.
`Presentation::apply_preset` and `Presentation::set_window` in
`ocelli-viewport` call `VoiTransform::new` and keep the output range the chain
already has. They add no formula.

### The automatic window

DICOM says nothing about a window for a frame that stores none, and
`open_frame` still invents nothing: `WindowInput::Absent` means the caller
chooses. `crates/ocelli-pixel/src/auto_window.rs` is the one implementation of
that choice, F-227, from the operator's answers of 2026-11-05.

- `series_sample(m)` picks `k = min(m, 16)` images at `i * (m - 1) / (k - 1)`
  in integer division, first and last included. `m = 0` is refused.
- `AutoWindowPool::new(k)` declares the frame count, and each frame's sample
  budget is `2^20 / k`, so the pool never holds more than 2^20 values. A frame
  of `n` samples is read at every `n.div_ceil(budget)`-th sample from index 0.
  The stride depends on the size and the count only, never on content.
- `add_frame` applies that frame's own `ModalityTransform`, PS3.3 C.11.1,
  because a PT series carries a Rescale Slope per image. Stored zero and that
  frame's padding range, PS3.3 C.7.5.1.1.2, are left out as stored-space
  predicates before the rescale, so a CT with intercept -1024 leaves out air
  outside the field of view and keeps water. A refused frame leaves the pool
  as it was, so a caller skips it and continues.
- `finish` takes the nearest ranks `ceil(n / 100)` and `ceil(99 n / 100)` of
  the pooled values, with no interpolation, and returns a `LINEAR_EXACT`
  window, PS3.3 C.11.2.1.3.2. The window is accepted only when the transform
  that draws it maps the 1st percentile to the output minimum and the 99th to
  the maximum, with each moved 16 f32 units toward the other. A narrowing
  search finds that width and refuses with `UnverifiableWindow` when none
  passes.
- `AutoWindowBasis` names the three outcomes. `Percentile` is the ordinary
  case. `Constant` takes percentiles fewer than three f32 units apart, draws
  them within half a grey of the middle, grey 127 or 128, and refuses beyond
  the measured magnitudes recorded on `finish`. `Background` is a pool of
  left-out values only and draws its largest at the output minimum, which
  is black on MONOCHROME2 and white on MONOCHROME1, whose presentation stage
  inverts, PS3.3 C.7.6.3.1.2.
- `auto_window` is a pool of one, the per-image window, the same code path.

The folder viewer is the one caller today. It sends the series' images in
the page's order to `POST /series-window`, pools the sampled ones that store no
window and no VOI LUT Sequence, skips the rest without replacing them, and
returns the centre and width as `f32` `Display` text that the page sends back
unchanged. Each line carries the id the page already holds, or `-` for a file
the page cannot offer, such as one F-224 marked changed, which is skipped when
sampled. Every sampled id is checked before any is decoded, and a 410 names
every one that is gone, so after a restart or F-216's eviction the page posts
exactly those files again and asks once more, for as long as each round names
an id it has not posted. Each sampled image is then read where the cache holds
it, or decoded from its held bytes with a buffer that never evicts, and
unpacked and folded into the pool in one decode job, under its permit. The
frame is unpacked into the permit's own scratch, which the permit keeps for
its next job inside F-216's permit term, as `GET /held` `workspace` reports.
The decoded samples exist only inside the job, which parks them on the free
list, inside `OCELLI_CACHE_MIB`, when cached and free bytes fit the budget,
and frees them otherwise. The decoded frame is not stored, so a sampled image
the page then shows is decoded again.

The pool moves into each job and back. It reserves at most 2^20 `f32` values,
4 MiB, when it is built, and keeps at most `2^20 / K` of them from each of
`K` sampled images, whatever the frame size. Measured on two 20-file series
of 1024 by 1024, one image kept between 209,712 and 209,716 bytes, under the
262,144 the bound gives at `K = 16`. At most `P` rounds run at once, `P`
F-216's permit count, and a round beyond them waits rather than being
refused. The pools are therefore F-216's named term `P * 4 MiB`, outside
`OCELLI_CACHE_MIB`. Four rounds asked for at once over four such series held
8,388,608 bytes of pool at once, which is `P * 4 MiB` at `P = 2`, and left
the cache's `used`, `files` and `frames` unchanged and `inflight` 0. When the
sampled files cannot all be held at once the rounds stop and each image keeps
its own window. An image with any window of its own always draws it. The
oracle injects its own declared default window through `render-params.json`
and the reference sidecar, so this policy changes no LUT chain verdict, and
cornerstone3D 5.8.2's per-image minimum and maximum is not a parity target.

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
| `YBR_ICT`, `YBR_RCT`, `XYB` | refused, `CodecOwnedColorTransform` |
| `MONOCHROME1`, `MONOCHROME2` | refused, `ColorTransformNotApplicable` |

PS3.3 permits `YBR_ICT` and `YBR_RCT` only with JPEG 2000, where the
codestream's own multiple component transform carries them. Both JPEG 2000
adapters invert that transform inside the decoder and report `Rgb` for the
two codes, and refuse a codestream whose own transform contradicts the
descriptor, so neither reaches this stage still in that space. RCT is an
integer lifting transform and not a matrix at all, so applying the full-range
matrix to it would produce a plausible image in the wrong colours. Under a
decoder reporting `Rgb` the question does not arise, because the resolution
has already replaced the space.

**`XYB` is a recognised colour space that this crate owns no transform for.**
PS3.3 C.7.6.3.1.2 defines it as the LMS-based model JPEG XL uses and gives no
matrix to or from it, so `ColorTransform::resolve` refuses it with
`CodecOwnedColorTransform` exactly as it refuses `YBR_ICT` and `YBR_RCT`.
F-229 added the variant, which expects three samples like the other colour
spaces. The one legitimate route is the codec's own conversion, reported
through a decoder that answers `Rgb`: the JPEG XL adapter does that for every
three-sample frame, because `jxl-oxide` renders XYB into the declared
encoding before the adapter sees a grid. A decoder that answered `Preserved`
for an `XYB` frame would be refused here rather than passed through as RGB.

**`XYB` is a recognised colour space that this crate owns no transform for.**
PS3.3 C.7.6.3.1.2 defines it as the LMS-based model JPEG XL uses and gives no
matrix to or from it, so `ColorTransform::resolve` refuses it with
`CodecOwnedColorTransform` exactly as it refuses `YBR_ICT` and `YBR_RCT`.
F-229 added the variant, which expects three samples like the other colour
spaces. The one legitimate route is the codec's own conversion, reported
through a decoder that answers `Rgb`: the JPEG XL adapter does that for every
three-sample frame, because `jxl-oxide` renders XYB into the declared
encoding before the adapter sees a grid. A decoder that answered `Preserved`
for an `XYB` frame would be refused here rather than passed through as RGB.

**`XYB` is a recognised colour space that this crate owns no transform for.**
PS3.3 C.7.6.3.1.2 defines it as the LMS-based model JPEG XL uses and gives no
matrix to or from it, so `ColorTransform::resolve` refuses it with
`CodecOwnedColorTransform` exactly as it refuses `YBR_ICT` and `YBR_RCT`.
F-229 added the variant, which expects three samples like the other colour
spaces. The one legitimate route is the codec's own conversion, reported
through a decoder that answers `Rgb`: the JPEG XL adapter does that for every
three-sample frame, because `jxl-oxide` renders XYB into the declared
encoding before the adapter sees a grid. A decoder that answered `Preserved`
for an `XYB` frame would be refused here rather than passed through as RGB.

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

### Display scaling above 8 bits, F-239

`DisplayScale` is the one rule that brings a colour sample or a palette entry
above 8 bits to the display's 8. A value of `n` bits is shifted right by
`n - 8` for `9 <= n <= 16` and taken as it is at 8. Every other depth is
`PixelError::UnsupportedColourDepth`, by name. The shift truncates, as
cornerstone3D 5.8.2's `>> 8` does, and is `floor(value / 2^(n - 8))` in `f32`,
exact for every integer the rule accepts. The identity returns the value
untouched, so an 8-bit frame is bit-identical to one that was never scaled.

For a sample the depth comes from the header, the operator's answer to F-239's
question 8. It is Bits Stored, PS3.3 C.7.6.3.1's range of each sample.
`ColorTransform::with_sample_scale` applies it at the one place a sample
enters stage 4, before the YCbCr matrix, so the 8-bit matrix and its `+ 128`
offset meet an 8-bit value. It refuses a palette transform with
`SampleScaleNotApplicable`, because a palette's stored value is an index
C.7.6.3.1.5 keys on the raw value. `resolve` leaves the identity.

For a palette, the one rule below decides the scale. `PaletteColorLut::new`
takes each channel's declared depth from its `LutDescriptor`, and
`with_entry_bits` takes a declared depth apart from the container, so a 12-bit
descriptor arrives as a 16-bit container. It refuses a depth above the
container with `InvalidLutBits` and an entry above `2^n - 1` with
`LutValueOutOfRange`. `channel_entries` yields the scaled entries, which are
the values the stage emits. cornerstone instead shifts all three channels by 8
only when a red entry exceeds 255, which the fixtures in
`crates/ocelli-pixel/tests/palette.rs` reject.

**The one palette rule**, the operator's ruling R1 after F-239's library
review pass 1, decided once for the whole palette by the lead's ruling: a
palette is read as 8-bit only when at least one channel is declared above 8
bits and every entry of every such channel fits in 8 bits, otherwise each
channel is shifted by its own declared depth minus 8. `with_entry_bits` takes
the identity for all three channels in the first case, as cornerstone reads
such a palette, and `PaletteColorLut::entries_read_as_8_bit` reports it so the
page can state it. Strictly by C.7.6.3.1.5 such a palette would shift to
black. Decided per channel, a near-zero channel of a conforming 16-bit palette
would be read as 8-bit, about 256 times too bright beside its shifted
neighbours. A channel declared 8-bit takes no part in the decision.

Before F-239 a 16-bit entry left stage 4 as its raw value, so tier C and the
GPU bake refused any entry above 255 and drew one at or below it as itself.
Both now read the scaled value, so a palette with an entry above 255 in a
wide channel draws where it was refused, and a palette whose wide channels
hold only entries of at most 255 draws as before. The corpus rows
`synthetic/sc_palette_color.dcm` and `synthetic/sc_palette_color_16.dcm` both
declare 16 bits and hold entries of at most 255. Measured on 2026-11-07 with
`ocelli-compare produce`, base 699a5148 and the tip each built in its own
source tree and target directory: 102 views, 0 bytes differ, the two palette
views holding 634,212 and 649,735 non-zero RGB bytes on both sides.

### The ICC subset, F-240

`crates/ocelli-pixel/src/icc.rs` parses the matrix and tone-curve subset of
ICC.1:2022 (profile version 4.4.0.0, the color.org PDF read before any
fixture was written) and converts through it to sRGB, IEC 61966-2-1 as
amended. The subset is a profile of class `mntr` or `scnr`, data colour space
`RGB `, PCS `XYZ `, carrying all six of `rXYZ`, `gXYZ`, `bXYZ`, `rTRC`,
`gTRC` and `bTRC`, each TRC a `curv` or a `para` of function type 0 to 4. The
sRGB colourants of the fixtures are the ones the ICC's own sRGB
characterisation note publishes for matrix/TRC profiles, and the columns sum
to the D50 media white, so a neutral device code stays neutral.

`IccProfile::parse` refuses every other shape by name, one `IccError` variant
per kind: another class, a `Lab ` PCS, a `GRAY` device side, a missing tag, a
`curv` whose entries do not fit their tag, a TRC that is neither a `curv` nor
a supported `para`, bytes that do not parse at all, and a profile above the
size constant. A profile that also carries a lookup-table tag (`A2B0` to
`A2B2`, `D2B0` to `D2B3`) is refused as `LookupTableProfile` even when the six
matrix tags are present, the operator's answer to F-240's question 2. A
profile with a lookup table asks for the lookup-table answer, and applying
the matrix would be a different transform from the one the profile asks for.
The header and the tag table are read from any length, so a profile is
classified for what it is before the size constant is held against it: a
lookup-table profile above 16,384 bytes is named `LookupTableProfile`, and
`TooLarge` is named only once a payload would be read. The size constant is
`IccProfile::max_bytes`, 16,384 bytes, set from the largest matrix/TRC profile
read on the developer host, the 3,144-byte sRGB profile that ships with
macOS. The corpus retains F-239's 44-byte placeholder in
`synthetic/sc_rgb_icc.dcm`, named malformed, and adds five generated matrix,
tone-curve, colourant-swap and lookup profiles. The held colour recipe retains
both an accepted transform and a refused profile's closed kind. A held redraw
does not parse the profile again. The scan classifies the same bytes for
`icc_applied` and `icc_kind` without tabulating device codes.

`ColorTransform::with_icc` attaches the parsed transform once per held frame,
after `with_sample_scale`. The place is item 4 of the plan: in place of
F-239's display scaling, before the colour window. With a transform attached
the sample scale is bypassed and the device codes are normalised by the
encoding range `2^n - 1`, which is `n = Bits Stored` for an RGB frame, `8`
for the YCbCr routes whose matrices consume 8-bit samples and return 8-bit
device RGB, and each palette channel's declared entry depth for palette
colour, ruling R1 included. The output is the linear sRGB encoding scaled by
255 as `f32`, so the window and the tier C byte rule are unchanged. A frame
without `(0028,2000)`, or with a refused one, keeps F-239's route byte for
byte, and `resolve` leaves no transform attached.

The PCS to linear sRGB matrix is the inverse of the D50 colourant matrix that
the ICC's sRGB characterisation note publishes for matrix/TRC profiles, the
same printed values the fixtures' colourant constants encode. The note derives
and prints the device to PCS direction, the colourants themselves, by
combining the recommended D65 to D50 chromatic adaptation with the inverse of
A.7's matrix from `XN, YN, ZN` to `RL, GL, BL`. ICC.1:2022 Annex F.3 makes
the PCS the colourant matrix times the linear device RGB, so inverting the
published colourants is the PCS to sRGB direction, and a D50 PCS white maps
to linear `1 1 1` within the printed matrix's own precision. The inverse is
computed in `f64` from the printed values and rounded once to `f32`, as
`color.rs` rounds its YCbCr inverse, and a unit test in `icc.rs` recomputes
the inverse from the printed values and asserts the constant entry by entry.
At parse time that matrix is multiplied once with the profile's own
colourant columns. The colourants are s15Fixed16, so the product is not
exactly the identity. For the note's colourants as the fixtures encode them,
each entry of the product is within `1.34e-5` of the identity's entry, and
the largest linear move the product gives a device triple in the unit cube is
`2.66e-5`, at the white vertex, green channel. Encoded, the gap between the
fixtures' hand values and the pipeline's replies reaches at most `0.0029` of
255 over the sRGB-colourant rows they assert, `0.0026` where the only
quantisation is the colourants and `0.0029` on the parametric rows, whose
curve parameters are s15Fixed16 too. The wide-gamut Adobe rows carry more,
`0.0054` of 255 at the red row's green channel: the quantisation moves that
channel by only `1.6e-6` linear against the printed decimals, but it answers
at `1.1e-4` from zero, where the `12.92` slope is `3294.6` codes per linear
unit. The unit cube's worst case, a black channel beside two saturated ones,
reaches about `0.044` of 255. The fixture tolerance is `0.05`, and the
smallest wrong implementation it must reject moves a row by about `0.31`.

The surviving, fully retained AC12 comparison is the 2026-11-09 re-observation of product tree `0fad23dfbd206135c1dfd51d1123e04df2b79bf4`. It uses the current Rust fixture builders, not the recovered historical Python builder. All six actual profile headers encode version 4.4.0.0 as `04400000`. The same profile bytes go through the public `ColorTransform` consumer and the external tool. Each profile has 22 rows, 12 at 8 bits and 10 at 12 bits, giving 132 native triples and 264 references across both modes. Raw decimal tool output and exact native `f32` bits are retained.

All fifteen recorded commands exit 0. Little CMS accepts the six minimal matrix/TRC profiles without adding tags or replacing profiles. The tool is `transicc` 5.1 over Little CMS 2.18, binary SHA-256 `8b54599cdced07536b7a8b882a2a63b508d42351b02433ac33ad244697d8c3eb`. Each invocation is `transicc -t1 -n [-s] -i <exact-profile> -o '*sRGB'`. Device inputs use the 0 to 255 tool scale, with 12-bit codes multiplied by 255/4095 and printed to ten decimal places. Tool, profiles, product inputs and the native test executable remain unchanged through the observation. The native executable SHA-256 is `8b9997a7f06573aea2dfa7088922cad2f5023dc0d2f28376f66405ea539c006c`.

All 264 reference observations remain present, including six unbounded differences above one code. They are the Adobe-like primaries at both depths. The largest is 1312.3506 codes for the red channel of device green: the unbounded reference is negative while Ocelli clips to zero. Each of these six bounded tool replies exactly equals clipping its unbounded reply to 0 through 255. The installed tool's help identifies `-s` as bounded mode that clips negatives and highlights. Ocelli's linear sRGB clipping is the existing approved pass 1 ruling, not a new tolerance or exclusion. This resolves those six flags as a difference in output policy. Across all 132 bounded references the maximum remains 0.3534772098 codes, at Adobe-like red's green channel. The 84 rows whose unbounded replies are in gamut have maximum 0.0055186096, at table2's 12-bit grey 102. The printed-matrix divergence and its non-identity-window consequence below remain open facts under D14. F-271 measures the alternative matrix separately.

The initial new helper failed compilation because it formatted a Debug-only error with Display. It produced no samples and remains a failed instrument attempt. A reviewed one-line private correction preceded this fresh observation. No product source, matrix, expected value or threshold changed. The complete run record SHA-256 is `fd44b6d5e6394a764a82488faa01c059fcc05222a0fc0b5850cb3e494798732b`, and the 264-row observation file SHA-256 is `397d7b49524328179c49a720818086f8b9c125456d6feb071fb92feb4f431997`. Root independently reparsed the native bits and all reference inputs and outputs, recomputed every error and investigated all six flags. This establishes the current comparison observation, with no timing, whole-story completion or alternative-matrix claim.

Under `-s` the largest difference over the whole row set is `0.3535` of 255,
at the Adobe-like profile's device red row's green channel at 8 and 12 bits
alike. That row is a
measured divergence from Little CMS under decision D14, stated with its
cause. Adobe RGB 1998 and sRGB share the red chromaticity `0.64, 0.33`, so
the colorimetric answer for Adobe device red in sRGB is green and blue
exactly 0. The two engines answer on opposite sides of that zero because
their PCS to sRGB matrices differ: Ocelli's is the note's printed matrix and
Little CMS's is the one its built-in sRGB profile carries. On that row Ocelli's green is `+1.073e-4` linear,
`0.3535` of 255 encoded, and Little CMS's is `-2.3e-6` linear, `-0.0077` of
255 unbounded and `0.0000` under `-s`, both within `1.1e-4` linear of zero
and Ocelli's the further side, the same near-zero class pass 1 recorded on
the primary rows. The same row's blue differs by `0.102` of 255, `+3.8e-5`
linear against `+7.1e-6`. No stored byte differs on any row at the identity
window, which is below AC12's one-code investigation threshold there. A
non-identity D-29 colour window does see the gap, because tier C's
`map_colour` windows the unrounded `map_into` output before `rgb_bytes`:
driven through `ocelli-render`'s `colour_bytes_into`, the window at centre
10 width 20 stores Ocelli's red row as `[255, 5, 2]` and Little CMS's `-s`
values, put through the same window and byte step, as `[255, 0, 0]`.

Per draw, each pixel costs three table reads, one 3 by 3 product and three
sRGB encodings. `IccTransform::tabulated` builds a table of each TRC over
every `2^n` device code, at most three tables of 65,536 `f32` entries or 768
KiB, measured by `IccTransform::heap_bytes` and aggregated by
`ColorTransform::heap_bytes`. The folder viewer includes that storage in the
colour recipe's exact allocation claim before cache admission. A channel
reads its table when the sample is a whole code the channel's own table
covers, and evaluates the curve continuously otherwise, which is the YBR
routes' case because their device values leave the matrix fractional. The
sRGB encoding is never tabulated: its input is continuous and a table would
change the values. The linear value clips to 0 and 1 before the encoding, A.7
of the ICC's sRGB note, so an out-of-gamut device colour, a wide-gamut
profile such as ADOBERGB being an expected input of PS3.3 C.11.15.1.2, leaves
display codes at `0 ..= 255` and the colour window and the readout act on
values sRGB has.

Tier A and tier B keep `ColourLut::icc` unavailable in `ocelli-render`, and a
GPU route given a profiled frame reports unavailable rather than drawing
device RGB: the palette bake takes the frame's profile presence,
`IccProfilePresence`, and refuses an accepted profile before any texel is
built. Tier C is full for the subset through `ColorTransform::map_into`.
`docs/lld/feature-availability.md` carries the declaration.

The fixtures are `crates/ocelli-pixel/tests/icc.rs`, hand computed from the
two published texts with the editions named in their module comment. AC11's
mutations were each run against a scratch copy of the sources rather than the
tracked tree. Dropping the sRGB encoding turns thirteen of the sixteen
fixtures red and transposing the colourant matrix turns fourteen red, the two
refusals-only rows staying green in both, the colourant swap joining them
under the dropped encoding because its strong channel is exactly `1.0`
linear, which the dropped encoding maps to itself. Treating `para` type 3's
`c` as type 4's `e` turns only the parametric fixture red. Three neighbouring
mutations of the same kinds needed rows of their own before they died: an
encode knee moved to `0.04045` needed the device `5` row, an encoding range
of `2^depth` needed the unit test that compares the tables with the
continuous curve over real colourants, and a lookup-table list missing
`D2B0` needed the loop over all seven tags.

### The colour window, deviation D-29

`ColourWindow` windows and inverts a colour frame per channel after stage 4,
as cornerstone3D 5.8.2 does, and adds no window arithmetic. The window is
`VoiTransform` with `VoiFunction::Linear` over `0 ..= 255`, which is
cornerstone's `getVOILut.js:3` character for character, and the invert is
`PresentationTransform`'s INVERSE over the same range, `0 + 255 - y`, after
the window. `PresentationTransform::new` still refuses every colour space, so
the invert is reached through the crate-private `inverse_over`, which builds
the same reflection without naming a photometric interpretation the frame
does not have. `LutChain` stays monochrome.

`initial` is width 256 and centre 128 with invert off, every colour image's
first window in cornerstone whatever the file says. `new` returns that same
skipped window when the width is 255 or 256, the centre is 127 or 128 and
invert is off, which is `renderColorImage.js:30` to `:36`. At width 255 and
centre 127 the formula is not the identity, so skipping matters there by a
byte. A width below 1 is the existing `InvalidWindow`.

The fixtures in `crates/ocelli-pixel/tests/color.rs` include the width 10 row,
`125 128 130` to `56.666664 141.66667 198.33334`, which is what rejects `w` in
place of `w - 1`, and an exact tie, `130` at width 511 and centre 128.5 to
`128.5`, whose byte is decided by the render boundary's tie rule alone.

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

The colour stage in this crate adds no shader and no uniform. HLD 18.4's
uniform covers stages 1 to 3 and stays thirty-two bytes. F-044 keeps that
uniform as the LUT decision, including `invert`, and puts the colour table in
`ocelli-render`.

A preset is indexed by the display value `LutChain::map_into` already
returned. The lookup does not apply greyscale normalisation on top of the
texel, and it does not run modality, VOI, photometric interpretation or a
second YBR conversion of its own. A palette texel is one call to
`ColorTransform::map_into` on the stored sample, baked once into the bytes
the renderer uploads. The shader does not walk the descriptor and does not
run `lut_chain` for a palette. When no colour LUT is bound, the output stays
the greyscale normalisation.

The index clamp is a table address. Values at or below 0 use the first texel.
Values at or above the last texel use that texel. The clamp is not applied to
the value `LutChain` returns. The greyscale path still emits the unclamped
LINEAR value recorded above. ICC stays unavailable on this GPU path and is
not passed through as device RGB, F-240's declaration below. A palette on a
monochrome photometric interpretation is unavailable, matching
`ColorTransform`'s refusal. A preset on a palette image is unavailable.

Tier C uses the same table bytes. A preset indexes them after
`LutChain::map_into`. A palette goes through `ColorTransform::map_into`.
There is no second palette formula. A palette entry that is not an integer
in 0 to 255 after its display scale is unavailable when the tier C colour
transform is accepted, which is the same refusal as the GPU bake. Since F-239
no scaled entry can fail that test, so the refusal is a guard rather than a
route. A later draw does not walk the table again. A colour sample whose value
after the transform's sample scale is outside 0 to 255 is unavailable on tier
C. An in-range sample still rounds to a byte, and a YBR result outside 0 to
255 still becomes 0 or 255. The byte rule rounds to the nearest code and a tie
to the even code since F-239, as `docs/lld/render-graph.md` records. A tier B
colour comparison was not run.

ICC on tier C is the F-240 subset above, through `ColorTransform::with_icc`
and `map_into`. On tiers A and B `ColourLut::icc` still returns `Unavailable`,
and a GPU route given a profiled frame reports unavailable rather than drawing
device RGB: the palette bake takes the frame's profile presence,
`IccProfilePresence`, and refuses an accepted profile before any texel is
built. A profile the subset refuses keeps the CPU route's own bytes, which is
D-30, and is not a pass-through of device RGB through a colour table.

`bin/ocelli.sh gate native` proves native linkage and shared-crate wasm
compilation. The focused `cargo check -p ocelli-pixel --all-targets --target
wasm32-unknown-unknown` isolates the pixel crate's wasm compile proof.
`bin/ocelli.sh gate wasm` builds `ocelli-wasm`. That crate depends on
`ocelli-render`, and `ocelli-render` depends on this crate, so the build
compiles `ocelli-pixel`. The gate does not execute this crate's tests.


## Folder viewer colour consumer

The folder viewer resolves its colour recipe once before sample admission,
including the decoder's actual output layout and length, palette scaling and
ICC disposition. It draws through `colour_bytes_into` in whole-row bands of
at most 65,536 pixels. Planar bands gather each plane's corresponding rows.
The three packed display bytes are windowed RGB, with the same pre-window
stage-four output supplying pointer values and each channel's region statistics.
Monochrome retains its single-byte reply and its modality chain.

The page uses the channel header for images, thumbnails, prefetch and tiles.
Colour starts at centre 128 and width 256, offers no file windows or presets,
and links a window only to the same kind. Mixed-kind tiles and prefetches use
their own defaults. An applied matrix/TRC profile has no omission sentence.
Every profile outside that subset retains device RGB and displays its closed
kind. Palette, ICC and retained conversion capacities are charged explicitly.
