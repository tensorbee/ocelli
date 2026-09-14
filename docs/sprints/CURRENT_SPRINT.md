# Current sprint, S10

**Milestone**: M2, DICOM ingest and the pixel pipeline.
**Branch**: `sprint/s10`
**Opened**: 2026-09-14
**Goal**: Close M2 by adding the colour half of the pixel pipeline: palette
colour, planar configuration, photometric interpretation and the YBR
transforms.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-030 | E4.8 | Palette colour, planar configuration, photometric interpretation, YBR | Rust | 2w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S10 / {print $2, $9}'
```

## What this sprint is

S10 is one story and it is the last one in M2. Nine sprints of ingest, codec
and monochrome pixel work leave exactly one gap: everything colour. F-030 adds
DICOM PS3.3 C.7.6.3's Photometric Interpretation handling beyond the monochrome
pair, Planar Configuration for uncompressed colour, the palette colour lookup
of C.7.9, and the YBR to RGB transforms. **It is the fourth stage of HLD section
18's LUT chain table**, the one F-029 deliberately left out.

**A wave of one story runs serial**, in this worktree, with no claim, no worker
branch, no worktree and no integration step. That is the simpler path rather
than a degraded one.

## What is carried in

- **F-X011** remains pending because its acceptance evidence requires a second
  physical machine and none is available. It is unfinished M1 evidence and is
  not a dependency of F-030.
- **A multi-component JPEG-LS corpus row was owed, and F-030 added it.**
  Appendix A gate A2 recorded it and F-028 did not close it. The S10 design
  round put it in this sprint rather than a separate story, and
  `syntax/jpegls_lossless_rgb8.dcm` is the row. `crates/ocelli-codec/src/jpegls.rs`
  still refuses a multi-component frame cleanly, which is the required behaviour
  rather than a gap, and the row is what makes that refusal measured against a
  real three-component codestream instead of a synthetic descriptor. **What
  remains owed is multi-component DECODING**, which is a codec story.
- **`openjph-core` 0.1.0 carries no BSD notice material.** `/release` step 5
  refuses while that is true and no development profile does. It blocks nothing
  in this sprint and it blocks publication. See D-22.
- S09 closed F-020, F-027, F-028 and F-029. F-030's only declared prerequisite
  is F-029, which is `done`, so it does not begin blocked.

## The corpus covered three of this story's four halves, and F-030 added the fourth

Measured rather than assumed, from `corpus/manifest.tsv`:

| What F-030 must interpret | Corpus row |
|---------------------------|------------|
| `RGB`, Planar Configuration 0 | `synthetic/sc_rgb_interleaved.dcm` |
| `RGB`, Planar Configuration 1 | `synthetic/sc_rgb_planar.dcm` |
| `YBR_FULL_422` | `synthetic/us_ybr_full_422.dcm` |
| A decoder-converted colour frame | `syntax/jpeg_baseline_rgb8.dcm` |
| **`PALETTE COLOR`** | **`synthetic/sc_palette_color.dcm` and `synthetic/sc_palette_color_16.dcm`, added by F-030** |

**There was no palette colour case in the corpus when this sprint opened.**
`scripts/corpus_check.py` knew the photometric value and validated one if
present, and nothing required one. F-030 added two, through a new
`corpus_synth.py --case` selector that writes one case without rebuilding the
whole layer, because a full rebuild would have re-encoded the three HTJ2K rows
with an OpenJPH that has moved since the manifest was built.

**This section originally planned the weaker route** and said so: a synthetic
fixture generated into ignored `corpus/data` from a committed generator, with no
manifest row behind it. That turned out to be unnecessary. `corpus_synth.py`
already produces manifest-backed rows for every other synthetic case, and the
only thing standing in the way was that adding one meant regenerating all of
them. The selector removed that, so the palette half is manifest-backed like the
rest rather than one rung weaker.

The planar pair was the model, two rows that are the same image in two layouts,
which is the only shape that can catch a layout read backwards. The palette pair
follows it with two rows that are the same table addressed two ways, one with a
non-zero first mapped input and one with an entry count declared zero.

**The 8-bits-per-entry LUT Data packing is deliberately not in the corpus.**
Both rows declare 16 bits per entry, where one entry per 16-bit word is
unambiguous. PS3.3 C.7.6.3.1.6's packing for 8-bit data is a PS3.5 encoding
question owned by the parser story, and a guess about it would have been frozen
into a manifest digest where it reads as evidence. The 8-bit descriptor path is
proven by `crates/ocelli-pixel/tests/palette.rs` instead, which depends on no
wire packing.

## The defect class this sprint is exposed to

**Colour is where a plausible image is most likely to be the wrong one**, and
this story concentrates four separate ways of producing it.

**The colour transform applied twice, or not at all.** A JPEG decoder usually
outputs RGB even though the DICOM header still says `YBR_FULL_422`. F-024
already made that observable: `Decoder::decode_photometric_interpretation`
returns `Rgb` or `Preserved` and `decode_sample_layout` returns `Interleaved` or
`Preserved`. **Those queries exist and nothing downstream consumes them yet.**
F-030 is the first consumer, and a second conversion applied on top of a
decoder's own darkens and shifts hue on an image that still looks like an image.
The F-024 AS_BUILT entry's closing note says exactly this and it is now due.

**`PALETTE COLOR` indexes the STORED value, not the Display value.** PS3.3 C.7.9
maps the stored value through the palette descriptors. F-029's design plan
recorded this as the reason palette is not a fourth arm on `LutChain`: a palette
path taking a `Display` input would be wrong in a way that stage ordering alone
would not reveal. Getting it wrong produces a colour image with the right shape
and the wrong colours.

**The palette descriptor's three values are a trap each.** A first value of `0`
means 65,536 entries and not zero. The second value is the first **input** value
mapped, so ignoring the offset shifts the whole colour map. A `bits per entry`
of 16 with 8-bit-looking data is real, and the descriptor is what decides, not
the data. `LutDescriptor` in `ocelli-pixel` already implements the first of
these for the modality and VOI stages and is the place to look before writing a
second one.

**`PlanarConfiguration` is meaningful only for uncompressed data.** For an
encapsulated syntax the codec defines the layout, so a header saying `1` for a
JPEG frame is to be ignored rather than honoured. `SampleLayout` in
`ocelli-pixel` already validates the Samples per Pixel, Photometric
Interpretation and Planar Configuration relationship, and
`StoredPixelDescription::unpack`'s doc comment already warns that a codec
expanding subsampled colour must describe its actual output layout. That
sentence becomes executable in this story.

**One further trap the corpus can see.** `YBR_FULL_422` stores Y1 Y2 Cb Cr for
each pair of pixels, so its frame is two bytes per pixel rather than three.
`scripts/tests/test_corpus_synth.py::test_ybr_full_422_frame_is_two_bytes_per_pixel`
asserts that about the corpus and nothing asserts it about our unpacker.

## What done means

- **F-030** interprets every Photometric Interpretation
  `crates/ocelli-pixel/src/stored_pixel.rs` already names, with each conversion
  citing its PS3.3 section and carrying a hand-computed fixture. A frame whose
  colour cannot be interpreted is refused rather than rendered in the wrong
  space.
- **The colour transform happens exactly once**, and the decoder's own
  `DecodePhotometricInterpretation` and `DecodeSampleLayout` evidence is what
  decides. A fixture proves a decoder-converted frame is not converted again,
  and it must fail if it were, which a greyscale fixture cannot show.
- **Palette colour reads the stored value**, with the descriptor's
  zero-means-65536 count, its first-mapped-input offset and its declared bits
  per entry all proven rather than assumed.
- **`PlanarConfiguration` is honoured for native syntaxes and ignored for
  encapsulated ones**, with both directions asserted.
- Palette and colour arithmetic lives in `ocelli-pixel` beside the rest of the
  LUT chain. HLD section 18 requires that arithmetic to exist exactly once and
  a second copy behind a colour check is the same defect as a second copy
  anywhere else.
- `wasm-bindgen` remains confined to `ocelli-wasm`, and no render-loop or
  network boundary is added.

## Dependency order

One story, so there is no order. F-030 depends on F-029, which is `done`.

**This sprint closes M2, and F-030 landed.** All fifteen of M2's stories are
`done` in `docs/sprints/BACKLOG.md`, counted from `allocation.json`'s
`milestone` field rather than from a number typed here, which is F-016 through
F-030 inclusive.

## Standing expectations

The HLD is authoritative. A design-plan departure is recorded in
`docs/hld/DEVIATIONS.md`, never improvised in implementation.

No patient data enters a prompt, tracked file, fixture, log, error or commit.
The ignored corpus remains behind `corpus/manifest.tsv` and its generators.

Pixel arithmetic remains observable. An unsupported colour route reports
unavailable or refuses according to its contract and is never silently treated
as the common path, which is HLD section 31's rule generalised.
