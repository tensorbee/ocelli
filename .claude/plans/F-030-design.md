# F-030, Palette colour, planar configuration, photometric interpretation, YBR

**Status**: approved
**Epic ref**: E4.8
**Sprint**: S10
**Estimate**: 2w

## Normative source, transcribed

### HLD `docs/hld/15-lut-chain.md` section 18, the stage table

The whole of what the HLD says about this story is one table row, quoted with
the three rows above it for position:

| **Stage** | **From → To** | **Source** |
|----|----|----|
| 1\. Modality LUT | Stored → Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2\. VOI LUT | Modality → Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3\. Presentation LUT | Display → Display | Identity or INVERSE; presentation state may override |
| 4\. Palette / ICC | Display → RGB | Palette colour LUT, or the display colour pipeline |

And the section's opening instruction, quoted in full because it is the
constraint this story is most exposed to:

> This is the highest-risk arithmetic in the project. It is specified in DICOM
> PS3.3 C.11 and the stages apply strictly in order. Implement it once, in
> ocelli-pixel, and let the shader read the parameters

and, after a dash this file's voice rules cannot reproduce:

> do not let a second copy of this logic appear anywhere.

**That is the entire normative text for stage 4.** There is no formula, no
signature, no fixture table and no listing. Sections 18.1 through 18.4 cover
stages 1, 2 and 3 and the shader uniform, and none of them mentions palette,
planar configuration or YBR. So unlike F-018 and F-029, this story cannot be
transcribed from the HLD and has to be transcribed from DICOM PS3.3 instead.
Everything below with a `C.7.6.3` or `C.7.9` citation is the standard, and
everything the HLD does not settle is named in the next section rather than
presented as specified.

### DICOM PS3.3 C.7.6.3.1.2, Photometric Interpretation

The values `StoredPixelDescription` already names, and what each one means for
this story:

| Value | Samples | What stage 4 must do |
|-------|---------|----------------------|
| `MONOCHROME1`, `MONOCHROME2` | 1 | not a colour frame, stage 3 owns it |
| `PALETTE COLOR` | 1 | index the three palette LUTs, C.7.9 |
| `RGB` | 3 | already RGB, layout only |
| `YBR_FULL` | 3 | inverse full-range matrix |
| `YBR_FULL_422` | 3 | chroma upsample, then inverse full-range matrix |
| `YBR_PARTIAL_422` | 3 | chroma upsample, then inverse partial-range matrix |
| `YBR_ICT`, `YBR_RCT` | 3 | the JPEG 2000 codec owns the inverse transform |

The forward equations, transcribed from PS3.3 C.7.6.3.1.2. **These are
`RGB -> YBR`, which is the opposite of the direction this story needs**, and
the standard states only this direction:

```text
YBR_FULL, 8-bit
  Y  = + .2990 R + .5870 G + .1140 B
  Cb = - .1687 R - .3313 G + .5000 B + 128
  Cr = + .5000 R - .4187 G - .0813 B + 128

YBR_PARTIAL_422, 8-bit
  Y  = + .2568 R + .5041 G + .0979 B + 16
  Cb = - .1482 R - .2910 G + .4392 B + 128
  Cr = + .4392 R - .3678 G - .0714 B + 128
```

`YBR_FULL_422` is `YBR_FULL` with the two chroma channels subsampled two to one
horizontally. C.7.6.3.1.2's note on the storage order:

> the Cb and Cr values are sampled horizontally at half the Y rate and as a
> result there are half as many Cb and Cr values as Y values.

Each pair of horizontally adjacent pixels is stored `Y1 Y2 Cb Cr`, so a frame
is `Rows * Columns * 2` bytes and **not** `Rows * Columns * 3`.

### DICOM PS3.3 C.7.6.3.1.3, Planar Configuration

> This value shall be identically 0 if Samples per Pixel is 1.
>
> 0 = The sample values for the first pixel are followed by the sample values
> for the second pixel, etc.
> 1 = Each colour plane shall be contiguous.

And the sentence that decides the second half of this story:

> Planar Configuration is only meaningful when Samples per Pixel is greater
> than 1, and is required to be 0 when the Pixel Data is encapsulated.

### DICOM PS3.3 C.7.9 and C.7.6.3.1.5, the palette colour LUTs

Descriptors `(0028,1101)` red, `(0028,1102)` green, `(0028,1103)` blue. Each is
three values:

1. **Number of entries in the lookup table.** Transcribed: "When the number of
   table entries is equal to 2^16 then this value shall be 0."
2. **First stored pixel value mapped.** Transcribed: "the first stored pixel
   value mapped ... Stored pixel values less than this value are mapped to the
   first entry. Stored pixel values greater than [first mapped plus number of
   entries minus one] are mapped to the last entry."
3. **Number of bits for each entry**, 8 or 16.

LUT Data is `(0028,1201)` red, `(0028,1202)` green, `(0028,1203)` blue.

**The word in value two is STORED.** Not Display, not Modality. That single word
is why the HLD's `Display → RGB` row does not describe the palette path, and it
is deviation D-23 below.

### The repository's own prior decisions this story consumes

`.claude/plans/F-029-design.md` section 3, quoted because it scoped this story:

> **Stage 4, palette and ICC.** Out of scope by `CURRENT_SPRINT.md` and not
> brought in. Palette colour is a `Display -> RGB` stage over an index, and the
> index is not a `Display` value at all: PS3.3 C.7.9 maps the **stored** value
> through the palette descriptors, so a palette path that took `Display` as its
> input would be wrong in a way stage ordering alone would not reveal.

`crates/ocelli-pixel/src/stored_pixel.rs`, `StoredPixelDescription::unpack`'s
doc comment, which this story makes executable:

> The unpacker operates on decoded sample containers. A codec that expands
> subsampled colour or converts YCbCr to RGB must describe its actual output
> layout before this stage. It must not pass an on-wire subsampled byte count
> as three expanded samples per pixel.

`crates/ocelli-codec/src/registry.rs`, the two evidence enums F-024 added and
which nothing has consumed until now:

```rust
pub enum DecodePhotometricInterpretation {
    /// The output retains [`FrameDesc::photometric_interpretation`].
    Preserved,
    /// The output is packed RGB, regardless of the encapsulating DICOM value.
    Rgb,
}

pub enum DecodeSampleLayout {
    /// The decoder preserves the input sample layout described by DICOM metadata.
    Preserved,
    /// Samples for each pixel are adjacent in the output buffer.
    Interleaved,
}
```

## What the specification does not cover

The HLD gives stage 4 one table row, so this section is long and that is
honest rather than sloppy. Each item is a decision this plan makes.

1. **The inverse matrices.** PS3.3 states `RGB -> YBR` only. Decoding needs
   `YBR -> RGB`, which is the inverse of a matrix whose coefficients the
   standard rounded to four decimals. Two candidate constants exist: the exact
   inverse of the stated matrix, and the textbook BT.601 inverse. **This plan
   inverts the matrix the standard states**, because the standard's equations
   are its definition of the encoding and inverting them is self-consistent by
   construction. Measured divergence from BT.601 over the corners of the 8-bit
   YBR cube is **0.0200 of 255**, which is below one quantisation step and is
   recorded rather than claimed to be zero. The exact inverses, computed in
   rational arithmetic:

   ```text
   YBR_FULL, applied to (Y, Cb - 128, Cr - 128)
     R = 1.000000000 Y - 0.000036820 Cb' + 1.401987577 Cr'
     G = 1.000000000 Y - 0.344113281 Cb' - 0.714103821 Cr'
     B = 1.000000000 Y + 1.771978117 Cb' - 0.000134583 Cr'

   YBR_PARTIAL_422, applied to (Y - 16, Cb - 128, Cr - 128)
     R = 1.164415463 Y' - 0.000095036 Cb' + 1.596001878 Cr'
     G = 1.164415463 Y' - 0.391724564 Cb' - 0.813013368 Cr'
     B = 1.164415463 Y' + 2.017290682 Cb' - 0.000135273 Cr'
   ```

   The two near-zero coefficients are not typographical. They are the residue
   of the standard's rounding, and dropping them to zero would be a second
   rounding decision this plan is not entitled to make. HLD 27.3 makes every
   one of these a human review item, and the reviewer's job here is to check
   the six transcribed forward coefficients against PS3.3 C.7.6.3.1.2 before
   checking any of the nine inverse ones.

2. **Where the resolution between header evidence and decoder evidence lives.**
   `ocelli-pixel` depends on `ocelli-core` and `glam` and nothing else.
   `ocelli-codec` depends on five codec crates and not on `ocelli-core`. No
   crate depends on both in normal dependencies today. Three options were
   weighed. Making `ocelli-pixel` depend on `ocelli-codec` inverts the layering
   and drags five codec libraries into the crate whose whole point is portable
   arithmetic. Moving the two evidence enums to `ocelli-core` gives one
   definition but puts a codec concept in a crate whose HLD section 4 row reads
   "Types, coordinate spaces, geometry primitives, error model", which does not
   name it. **This plan declares the two evidence enums in `ocelli-pixel`**,
   phrased from the consumer's side, and names the mapping as owed by whichever
   later story first wires a decoder's output into the colour stage. This is
   the shape `PixelRepresentation` already has, which exists in both
   `ocelli-codec` and `ocelli-pixel` today for the same reason.

3. **The output type and where it lives.** HLD 16.1 names three value spaces
   and all three are scalar. RGB is not, and adding a fourth entry to that
   listing would edit a normative one. `Rgb` is therefore declared in
   `ocelli-pixel`, beside its only producer, with `f32` channels to match the
   three scalar spaces and to avoid introducing a rounding decision inside
   stage 4. Rounding to 8-bit belongs at the render or export boundary, and
   HLD 27.3 makes it a review item wherever it lands.

4. **`YBR_ICT` and `YBR_RCT`.** PS3.3 permits these only with JPEG 2000, where
   the codestream's own multiple component transform carries them and the
   decoder performs the inverse. `crates/ocelli-codec/src/jpeg2000.rs` line 285
   refuses `components != 1 || mct != 0`, so no frame from this repository's
   JPEG 2000 or HTJ2K path can reach stage 4 still in either space. Stage 4
   **refuses** both when the decoder reports `Preserved`, because applying the
   full-range matrix to RCT samples, which are an integer lifting transform and
   not a matrix at all, would produce a plausible image in the wrong colours.
   That is HLD section 31's rule. When a decoder reports `Rgb` the question does
   not arise, because the resolution has already replaced the space.

5. **Whether the 4:2:2 chroma upsample interpolates.** PS3.3 does not say.
   **Replication** is used, so each chroma sample serves both pixels of its
   pair. Interpolation would invent values between samples and the standard
   gives no filter, so replication is the choice that adds nothing the
   specification did not.

6. **What `sample_count` means for a 4:2:2 frame.** `StoredPixelDescription`
   today returns `rows * columns * samples_per_pixel`, which is three samples
   per pixel for `YBR_FULL_422`. The wire holds two bytes per pixel. See
   Approach section 5. This is a latent defect in F-018's code that this story
   is the first to be able to observe, and fixing it here is in scope.

7. **ICC.** The HLD's stage 4 row names it beside palette. ICC is whole-slide
   colour management, `(0028,2000)`, and is not implemented by this story.
   `CURRENT_SPRINT.md`'s "What done means" does not name it and no corpus row
   carries a profile. The route reports unavailable rather than being silently
   absent, through the same refusal any unhandled colour route takes.

## Approach

One new module, `crates/ocelli-pixel/src/color.rs`, plus a bounded change to
`stored_pixel.rs` for the 4:2:2 sample count. No existing stage is edited.

### 1. The evidence types and the resolution, in one place

```rust
/// What a decoder reports about the colour space of its own output.
///
/// Mirrors `ocelli_codec::DecodePhotometricInterpretation`. See the plan's
/// "What the specification does not cover" item 2 for why it is not shared.
pub enum DecodedPhotometric { Preserved, Rgb }

/// What a decoder reports about the sample ordering of its own output.
pub enum DecodedLayout { Preserved, Interleaved }

/// PS3.5 8.2, whether Pixel Data arrived native or encapsulated.
pub enum PixelDataEncoding { Native, Encapsulated }
```

`PixelDataEncoding` is a named two-variant enum rather than a `bool` because
the call site otherwise reads as a positional flag next to two other
two-state arguments.

The resolution is two small total functions and is the only place either
question is answered:

```text
effective colour space =
    Rgb                                   when decoded photometric is Rgb
    the header's Photometric Interpretation  otherwise

effective layout =
    Interleaved                           when decoded layout is Interleaved
    Interleaved                           when encoding is Encapsulated
    the header's Planar Configuration     otherwise
```

The first line is the double-conversion guard. A JPEG frame whose header still
says `YBR_FULL_422` resolves to `Rgb` and is not converted a second time.

The second and third lines are PS3.3 C.7.6.3.1.3's "required to be 0 when the
Pixel Data is encapsulated", so a header saying `1` for a JPEG frame is
ignored, and the fourth line is the same attribute honoured for a native
syntax. Both directions are asserted.

### 2. `ColorTransform`, the resolved stage

```rust
pub struct ColorTransform { /* resolved space, resolved layout, palette */ }

impl ColorTransform {
    pub fn resolve(
        layout: SampleLayout,
        encoding: PixelDataEncoding,
        decoded_photometric: DecodedPhotometric,
        decoded_layout: DecodedLayout,
        palette: Option<PaletteColorLut>,
    ) -> Result<Self, PixelError>;

    /// Map one frame of stored samples into caller-provided RGB storage.
    pub fn map_into(&self, source: &[Stored], destination: &mut [Rgb])
        -> Result<(), PixelError>;
}
```

`map_into` follows the crate's existing contract exactly: it checks both
lengths before the first destination element is written and returns
`DestinationLength` or `SourceLength` without mutating anything. That is what
`StoredPixelDescription::unpack`, `ModalityTransform::map_into` and
`LutChain::map_into` already do, and the existing tests for that behaviour are
the model.

Refusals, each a new `PixelError` variant:

| Condition | Refusal |
|-----------|---------|
| resolved space is `Monochrome1` or `Monochrome2` | `ColorTransformNotApplicable` |
| resolved space is `YbrIct` or `YbrRct` | `CodecOwnedColorTransform` |
| resolved space is `PaletteColor` and no palette supplied | `MissingPaletteLut` |
| a palette supplied for a non-palette space | `UnexpectedPaletteLut` |
| the three palette descriptors disagree | `MismatchedPaletteDescriptors` |
| `Columns` is odd and the space is 4:2:2 | `OddColumnsForSubsampledChroma` |

`ColorTransformNotApplicable` is deliberately the mirror of
`PresentationLutNotApplicable`, which stage 3 already returns for every colour
space. The two refusals together mean a frame reaches exactly one of stage 3
and stage 4 and never both and never neither.

### 3. Palette reuses `LutDescriptor` and adds no lookup arithmetic

`LutDescriptor::new(entry_count, first_mapped_input, bits_per_entry, values)`
already validates every one of PS3.3 C.7.6.3.1.5's three descriptor values:
the zero-means-65,536 entry count, the first mapped input in the signed or
unsigned 16-bit range, and an 8 or 16-bit entry size with its value range.
`LutDescriptor::lookup` already clamps below the first mapped input and above
the last entry, which is exactly C.7.6.3.1.5's stated clamping.

So `PaletteColorLut` is three `LutDescriptor`s and one cross-check:

```rust
pub struct PaletteColorLut { red: LutDescriptor, green: LutDescriptor, blue: LutDescriptor }

impl PaletteColorLut {
    pub fn new(red: LutDescriptor, green: LutDescriptor, blue: LutDescriptor)
        -> Result<Self, PixelError>;
}
```

The cross-check refuses three descriptors that do not agree on entry count and
first mapped input, because a palette whose channels map different input ranges
would shift hue against luminance in a way no shape check reveals. **No new
lookup arithmetic is written**, which is HLD section 18's "do not let a second
copy of this logic appear anywhere" applied to the one place in this story
where a second copy was available for free.

`LutDescriptor` materialises an `inputs: Vec<f32>` ramp and binary-searches it.
For a 16-bit palette that is 65,536 floats per channel beside the values, so
about 1.5 MB for three channels where the index arithmetic needs none of it.
**This story does not change it.** The ramp is shared with the modality and VOI
stages and replacing the search with index arithmetic would change those two
stages' behaviour at a fractional input, which is a change to validated pixel
arithmetic and needs its own story and its own fixture. It is recorded in
`docs/lld/pixel-pipeline.md` as a measured cost rather than left for someone to
rediscover.

### 4. The three colour conversions

Each is a total function over one pixel, with the constants from "What the
specification does not cover" item 1 as named `const` values carrying their
PS3.3 citation.

```text
Rgb            -> deinterleave only, per the resolved layout
YbrFull        -> inverse full-range matrix
YbrFull422     -> replicate chroma across the pair, then the full-range matrix
YbrPartial422  -> replicate chroma across the pair, then the partial-range matrix
```

No clamping to `[0, 255]` is applied. The standard's rounded forward matrix is
not exactly normalised, so a saturated primary round-trips to `Cb = 255.5`, and
a stage that silently clamped would hide that rather than report it. Clamping
belongs with the rounding decision at the render or export boundary.

### 5. The 4:2:2 sample count, in `stored_pixel.rs`

`SampleLayout` already refuses a 4:2:2 photometric interpretation with a planar
configuration other than interleaved. It gains one accessor,
`stored_samples_per_pixel()`, returning 2 for `YbrFull422` and
`YbrPartial422` and `samples_per_pixel()` otherwise, and
`StoredPixelDescription::sample_count` uses it. `StoredPixelDescription::new`
becomes fallible for the one new condition, odd `Columns` with a 4:2:2 space,
because half a chroma pair cannot be stored.

This makes
`scripts/tests/test_corpus_synth.py::test_ybr_full_422_frame_is_two_bytes_per_pixel`,
which asserts the property about the corpus, have a counterpart asserting it
about the unpacker. Today a `us_ybr_full_422` frame cannot be unpacked at all,
because `unpack` demands three bytes per pixel from a source that holds two,
and nothing observes that.

### 6. The corpus, and one tooling change that is not optional

`CURRENT_SPRINT.md` measured the gap and it is real. There is no
`PALETTE COLOR` row. Two rows are added through the committed generator, which
is the manifest-backed route and is stronger than a Rust-only fixture:

| New row | What it traps |
|---------|---------------|
| `synthetic/sc_palette_color.dcm` | 8-bit indices, 256 entries, **first mapped input 10**, 8-bit entries |
| `synthetic/sc_palette_color_16.dcm` | 16-bit indices, **entry count declared 0**, **16-bit entries** |

The first catches the offset trap, the second catches zero-means-65,536 and the
16-bit-entry trap together. Declaring 0 on the second is not contrived: 65,536
does not fit in a `US`, so a full 16-bit palette has no other legal spelling.

**The tooling change.** `scripts/corpus_synth.py --tool-versions` reports that
the installed OpenJPH is **0.31.0** and `corpus/manifest.tsv` was built with
**0.26.3**. `generate()` removes and rebuilds the whole synthetic and syntax
layer, so adding a case the ordinary way would re-encode the three HTJ2K rows
with a different encoder and churn their digests inside a colour story. That is
a toolchain bump wearing a colour story's commit message, and it is exactly the
contamination `--tool-versions` exists to make visible.

So `corpus_synth.py` gains `--case NAME`, which runs one named case function
without the `rmtree` and without touching the syntax layer. `--write-manifest`
then re-reads every digest from files already on disk, so the ninety-two
existing rows keep their recorded digests byte for byte and two rows are added.
The OpenJPH drift is reported to the operator and left alone, because
regenerating those three rows is a separate decision with its own evidence.

Both new cases are pure `pydicom`, Explicit VR Little Endian, no external
encoder, so they are deterministic on any machine.

### 7. The multi-component JPEG-LS row, by operator decision

S10's design round placed the carried-in row in this story. See Open questions
item 1 for what it does and does not buy.

One case is added to the syntax layer through the same `--case` selector, so
nothing else in that layer is re-encoded:

| New row | Transfer syntax | Reference |
|---------|-----------------|-----------|
| `syntax/jpegls_lossless_rgb8.dcm` | `1.2.840.10008.1.2.4.80` | `reference_rgb8.dcm` |

`pyjpegls` is at 1.5.1, which is what `corpus/manifest.tsv` was built with, so
this encode introduces no toolchain drift of its own. The existing
`REFERENCE_RGB8` base already carries three samples per pixel at 8 bits, so no
new base is needed.

Three details to settle in implementation rather than assume. `SYNTAX_CASES`
becomes a map with two filenames sharing the `.80` UID, and
`scripts/tests/test_corpus_synth.py` is checked for a one-case-per-syntax
assumption before the row lands. `cases_for("pyjpegls")` filters that map by
UID and therefore picks the new file up with no edit. The manifest's modality
rule is already `"OT" if "rgb8" in name`, which this filename satisfies by
construction, and that is a coincidence worth making deliberate rather than
relying on, so the filename is chosen to match and the plan says why.

On the Rust side the codestream is extracted once into
`crates/ocelli-codec/tests/fixtures/` as a raw blob and asserted through
`include_bytes!`, which is the pattern `tests/jpeg2000.rs` already uses. That
keeps the ordinary suite self-contained and keeps a DICOM file out of git. The
assertion is that a real three-component JPEG-LS codestream is refused with the
specific `CodecError` the crate declares, and **not** that it decodes.

`docs/spikes/A2-jpeg-ls.md`'s coverage table says "REFUSED by the crate, and
NOT MEASURED here" for `Nf != 1`, and its section "The multi-component gap is
real and the corpus cannot close it" says there is no such row. Both become
false when this lands. The spike record is evidence of what was known when the
gate ran and is not rewritten. A dated note is appended naming F-030 as what
closed the corpus half, which is the same treatment D-13 gave HLD 18.3.

### 8. What is deliberately not added

- **ICC**, item 7 above.
- **Supplemental Palette Color LUTs** `(0028,1221)` and neighbours. A different
  module with a different rule, no corpus row, and no caller.
- **A second copy of the LUT lookup**, section 3.
- **Any shader, uniform or `#[repr(C)]` struct.** HLD 18.4's uniform covers
  stages 1 to 3 and says nothing about colour. A palette texture layout and a
  colour-matrix uniform are the rendering story's, and inventing them here
  would put a layout with no reader into a crate that cannot test it.
- **Wiring a decoder's output into `ColorTransform`.** No crate depends on both
  `ocelli-pixel` and `ocelli-codec` in normal dependencies. Item 2 names this
  as owed.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. `map_into` writes caller-provided storage and
  allocates nothing. `PaletteColorLut` takes ownership of its three descriptors
  at setup, which is where `LutDescriptor` already allocates.
- unsafe: none
- Tier A (WebGPU): n/a. This story adds no rendering feature. It resolves the
  parameters a later shader story reads, which is HLD section 18's division.
- Tier B (WebGL2): n/a, for the same reason.
- Tier C (CPU): full. `ColorTransform::map_into` is the authoritative CPU
  colour path, the same way `LutChain::map_into` is the authoritative CPU
  greyscale path. There is no tier-specific arithmetic copy, which is D-07's
  standing requirement.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | Palette lookup with first mapped input 10, including both clamps, PS3.3 C.7.6.3.1.5 | `crates/ocelli-pixel/tests/palette.rs` |
| `fixture` | Entry count 0 means 65,536 entries and not zero, PS3.3 C.7.6.3.1.5 | `crates/ocelli-pixel/tests/palette.rs` |
| `fixture` | 16-bit entries are read from the descriptor and not inferred from the data, PS3.3 C.7.6.3.1.5 | `crates/ocelli-pixel/tests/palette.rs` |
| `fixture` | `YBR_FULL` inverse at eight hand-computed YBR triples, PS3.3 C.7.6.3.1.2 | `crates/ocelli-pixel/tests/color.rs` |
| `fixture` | `YBR_PARTIAL_422` inverse, including that `Y = 16` is black and not `Y = 0`, PS3.3 C.7.6.3.1.2 | `crates/ocelli-pixel/tests/color.rs` |
| `fixture` | 4:2:2 chroma replication across a pair, and the two-bytes-per-pixel frame size, PS3.3 C.7.6.3.1.2 | `crates/ocelli-pixel/tests/color.rs` |
| `fixture` | The same image interleaved and planar produces identical RGB, PS3.3 C.7.6.3.1.3 | `crates/ocelli-pixel/tests/color.rs` |
| `unit` | A decoder reporting `Rgb` suppresses a second conversion, and the same frame with `Preserved` does convert | `crates/ocelli-pixel/src/color.rs` |
| `unit` | `Encapsulated` ignores Planar Configuration 1, `Native` honours it | `crates/ocelli-pixel/src/color.rs` |
| `unit` | `YBR_ICT` and `YBR_RCT` are refused under `Preserved` and pass under `Rgb` | `crates/ocelli-pixel/src/color.rs` |
| `unit` | Every refusal in the Approach section 2 table, each by its own value | `crates/ocelli-pixel/src/color.rs` |
| `unit` | `map_into` refuses a length mismatch before the first write | `crates/ocelli-pixel/src/color.rs` |
| `unit` | Mismatched palette descriptors are refused | `crates/ocelli-pixel/tests/palette.rs` |
| `conformance` | Both palette rows carry the descriptors and data the case exists for | `scripts/tests/test_corpus_synth.py` |
| `conformance` | Both palette rows parse under their declared transfer syntax | `crates/ocelli-dicom/tests/corpus.rs`, which enumerates the manifest and needs no edit |
| `conformance` | A real three-component JPEG-LS codestream is refused with the crate's declared `CodecError`, not decoded | `crates/ocelli-codec/tests/jpegls.rs` |
| `conformance` | The multi-component JPEG-LS row carries three samples per pixel and the `.80` syntax | `scripts/tests/test_corpus_synth.py` |

**The mutation check.** Per CLAUDE.md, each fixture must go red when one
constant moves. The three that matter and are checked by hand before the story
closes: the first mapped input, where setting it to 0 must break the palette
fixture rather than shift it invisibly, the `1.401987577` red-from-Cr
coefficient, and the `+ 16` luma offset in the partial-range path, which is the
one a full-range implementation silently omits.

**The fixture direction.** Every colour fixture states YBR and asserts RGB,
never the reverse. Building expected values by running the standard's forward
equations would reach saturated inputs the rounded matrix does not normalise,
where pure red encodes to `Cb = 255.5`, and a fixture built through an
unrepresentable intermediate asserts the wrong thing.

## Parity surface covered

None. `docs/hld/B-parity-surface.md`'s eight surface rows are viewport types,
tool classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation
representations, core events and adapters. Colour interpretation is not one of
them, and the `VOI LUT functions` row is stage 2's and was covered by F-018.

## Deviations

**D-23, new.** HLD section 18's stage table gives stage 4 as `Display → RGB`.
The palette path maps the **stored** value, per PS3.3 C.7.6.3.1.5's "the first
stored pixel value mapped", and the RGB and YBR paths map decoded samples that
never entered the chain at all. So no arm of stage 4 takes a `Display` input.
The row is added to `docs/hld/DEVIATIONS.md` in this change.

Existing deviations this story operates under and does not alter: **D-07**, the
tier C row above. **D-13** is untouched, because this story adds no VOI
arithmetic.

## LLD impact

- `docs/lld/pixel-pipeline.md`. A `Colour stage` section, the resolution table,
  the two inverse matrices with their measured BT.601 divergence, the 4:2:2
  sample-count correction, and the `LutDescriptor` ramp cost recorded as a
  known allocation rather than a surprise.
- `docs/lld/corpus.md`. The two palette rows and the multi-component JPEG-LS
  row, what each traps, and the `--case` selector with the OpenJPH drift that
  motivated it.
- `docs/lld/codecs.md`. The measured multi-component JPEG-LS refusal, which
  until now was a declared behaviour with no codestream behind it.
- `docs/spikes/A2-jpeg-ls.md`. A dated appended note, not a rewrite. See
  Approach section 7.
- `docs/lld/errors.md`, **checked during implementation and not touched.** It
  covers `ocelli-core`'s `ErrorCode`, the numbered boundary registry, and does
  not enumerate `PixelError` at all. The six new variants therefore need no
  entry there and no `ci/error-codes.json` row, because
  `scripts/error_code_check.py` reads `ErrorCode` and nothing else.

## Open questions

Both were put to the operator in S10's consolidated design round and both are
answered. They are kept here with their answers rather than deleted, because a
plan that shows only the chosen branch reads as though no choice was made.

1. **The multi-component JPEG-LS corpus row.** `CURRENT_SPRINT.md` carried it
   in and left the placement to this plan. The plan recommended filing it
   separately. **The operator's answer is to add it in F-030**, and Approach
   section 7 is the scope that answer creates. The recommendation's reasoning
   still stands as the cost of the decision and not as a dissent: the row is
   evidence of a clean refusal rather than of a decoded frame, because
   `crates/ocelli-codec/src/jpegls.rs` line 129 refuses `samples_per_pixel != 1`
   before the dependency is reached and the adopted crate has no
   multi-component support. What the answer buys is that the refusal is
   measured against a real multi-component codestream instead of a synthetic
   `FrameDesc`, and that `docs/spikes/A2-jpeg-ls.md`'s "NOT MEASURED here" row
   stops being true. The `--case` selector of Approach section 6 is what keeps
   the added `pyjpegls` encode from touching any other row.

2. **`YBR_PARTIAL_422`.** Implement the conversion, or report unavailable?
   **The operator's answer is to implement**, which is the plan's own
   recommendation. It is fully specified by PS3.3 C.7.6.3.1.2, `SampleLayout`
   already accepts it, and it is real in ultrasound. The cost carried into
   implementation is that there is no corpus row, so the only evidence is the
   hand-computed fixture, and the `+ 16` luma offset is exactly the detail a
   fixture derived from the same misreading as the code would not catch. The
   mutation check in the Tests section names that offset as one of the three
   constants moved by hand before the story closes.
