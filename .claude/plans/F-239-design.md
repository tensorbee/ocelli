# F-239, Folder viewer colour frames through the library's colour stage

**Status**: approved
**Epic ref**: E38.37
**Sprint**: S46
**Estimate**: 12w

Story row, `docs/sprints/BACKLOG.md`, retitled with the operator's answers of 2026-11-07: "Folder viewer colour frames: RGB, YBR_FULL, YBR_FULL_422, YBR_ICT, YBR_RCT and PALETTE COLOR with Planar Configuration drawn through ocelli-pixel's ColorTransform on the decoder's reported output space, multi-component JPEG 2000 and HTJ2K decoded with their component transforms, window and invert per channel as cornerstone3D 5.8.2 applies them, region statistics per channel, colour above 8 bits scaled by one library rule, an ICC profile named as not applied, and every colour shape this build cannot draw refused by name". Layer All, depends on F-030, F-209 and F-216, milestone M20. F-240 follows it and applies matrix ICC profiles.

`allocation.json` notes: "Decided by the operator on 2026-11-07: a colour series shows the fault 'this preview shows monochrome frames only' from PreviewError::NotMonochrome today, and the folder viewer previews colour frames through the library's colour stage instead. Scheduled first in the folder viewer chain, ahead of F-212. Questions 1 to 8 answered by the operator on 2026-11-07, widening the story to JPEG 2000 and HTJ2K colour decoding and cornerstone3D's colour window."

The operator answered questions 1 to 8 on 2026-11-07, and question 9, which the widened question 2 raised, later the same day. Every answer is applied below and recorded under "Decided by the operator on 2026-11-07". The plan is approved, the three deviation rows are D-29, D-30 and D-31 in `docs/hld/DEVIATIONS.md`, written in the approving change, and the story is split into a codec part and a viewer part under "Dependencies, write ownership and resources". Every path and line below was read on `sprint/s46` at 5451cb7d unless it names 636fdcc1.

**The estimate rose from 4w to 12w.** The parts, each the lead's estimate: the original preview wiring 4w, the decoder output length query 0.5w, the colour window and invert in `ocelli-pixel` and the page's tools 1w, per-channel region statistics 0.5w, the display scaling rule 0.5w, the ICC sentence 0.25w, the oracle's palette reader 0.25w, HTJ2K three-component decoding with its inverse transform 1.5w, JPEG 2000 Part 1 three-component decoding 2w, five synthetic corpus rows 0.5w, and, from the operator's clarification that cornerstone3D is the floor, the R, G and B readout at the pointer, 0.25w, and the size, regression and comparison measurements of AC29 to AC32, 0.75w. Question 9's answer keeps all of it. The codec part is 4.5w of that, the decoder length query, both decoders and the corpus rows, and the viewer part is the other 7.5w.

## Normative source, transcribed

### HLD section 18, `docs/hld/15-lut-chain.md:18` to `:27`

```text
This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere.

| 4\. Palette / ICC | Display → RGB | Palette colour LUT, or the display colour pipeline |
```

### D-23, `docs/hld/DEVIATIONS.md:40`

"No arm of stage 4 takes a `Display` input. Palette colour maps the **stored** value through the three palette LUTs. The `RGB` and every `YBR_*` route map decoded samples that never entered the chain at all. Stage 4 is therefore a separate transform beside stages 1 to 3 rather than a fourth arm on `LutChain`, and the greyscale and colour routes are made mutually exclusive by construction: stage 3 refuses every colour space with `PresentationLutNotApplicable` and stage 4 refuses both monochrome ones with `ColorTransformNotApplicable`." And: "a three-sample frame has no scalar `Display` value to be the input of, and PS3.3 C.11's chain does not apply to it at all. ICC is not implemented by F-030 and its route reports unavailable."

### HLD section 25.1, `docs/hld/22-testing-and-tolerance.md:46`

```text
- **Colour and ultrasound:** perceptual difference below a stated threshold, because chroma subsampling and YBR conversion legitimately differ.
```

No threshold is stated anywhere in `docs/hld/`, so this story makes no oracle claim on a colour frame.

### The library's colour stage, as implemented by F-030

`crates/ocelli-pixel/src/color.rs` cites PS3.3 C.7.6.3.1.2, C.7.6.3.1.3 and C.7.9 (`color.rs:1`). The resolution rule, `ColorTransform::resolve` at `color.rs:241` to `:286`, quoted from its doc comment at `color.rs:216` to `:227`:

```text
colour space = Rgb                       when the decoder reports Rgb
               the data set's value       otherwise

layout       = Interleaved               when the decoder reports Interleaved
               Interleaved               when the encoding is Encapsulated
               the data set's value       otherwise
```

"The second layout line is PS3.3 C.7.6.3.1.3's 'required to be 0 when the Pixel Data is encapsulated', so a data set saying `1` for a JPEG frame is ignored rather than honoured." `resolve` refuses `MONOCHROME1` and `MONOCHROME2` with `ColorTransformNotApplicable` (`color.rs:251` to `:253`) and refuses `YBR_ICT` and `YBR_RCT` that a decoder did not convert with `CodecOwnedColorTransform` (`color.rs:254` to `:256`), because "PS3.3 permits those only with JPEG 2000 and the codestream's own multiple component transform is what inverts them. RCT is an integer lifting transform and not a matrix at all" (`color.rs:233` to `:237`).

The YCbCr conversion the library implements is the exact rational inverse of PS3.3 C.7.6.3.1.2's stated forward matrix, rounded once to `f32` (`color.rs:400` to `:446`). The forward equations as the library transcribes them, `color.rs:418` to `:422`:

```text
Y  = + .2990 R + .5870 G + .1140 B
Cb = - .1687 R - .3313 G + .5000 B + 128
Cr = + .5000 R - .4187 G - .0813 B + 128
```

and the inverse it applies, `color.rs:430` to `:434`, with `Cb' = Cb - 128` and `Cr' = Cr - 128`:

```text
R = 1.000000000 Y - 0.000036820 Cb' + 1.401987577 Cr'
G = 1.000000000 Y - 0.344113281 Cb' - 0.714103821 Cr'
B = 1.000000000 Y + 1.771978117 Cb' - 0.000134583 Cr'
```

`YBR_PARTIAL_422` uses `Y' = Y - 16` and the inverse at `color.rs:461` to `:465`. 4:2:2 chroma is replicated, "each pair of horizontally adjacent pixels is stored Y1 Y2 Cb Cr, and both pixels of the pair share the one chroma sample. Replication rather than interpolation, because the standard names no filter" (`color.rs:376` to `:379`). Planar Configuration, PS3.3 C.7.6.3.1.3: "Interleaved is RGBRGB, planar is RRR GGG BBB" (`color.rs:368`). Palette colour, PS3.3 C.7.6.3.1.5 and C.7.9: the lookup is `LutDescriptor`'s, which "already clamps below the first mapped input and above the last entry" (`color.rs:88` to `:98`). The stage does not clamp or round, because "rounding to an 8-bit channel is a decision HLD 27.3 makes a human review item. It belongs at the render or export boundary" (`color.rs:26` to `:30`).

### PS3.5 section 8.2, as the library carries it

`PixelDataEncoding` is "PS3.5 section 8.2, whether Pixel Data arrived native or encapsulated" (`color.rs:76` to `:86`). The folder viewer already knows which, through `DispatchPath` (`preview.rs:1229` to `:1231`).

### The render boundary's byte rule, tier C

`crates/ocelli-render/src/cpu.rs` is where the library rounds a colour channel. `RasterColor::supported` refuses a palette entry that is not an exact display byte (`cpu.rs:122` to `:131`). `raster_color_into` refuses a stored colour sample outside `0..=255` (`cpu.rs:386` to `:390`, predicate at `cpu.rs:957` to `:964`). `rgba_from_rgb` (`cpu.rs:948` to `:955`) applies `byte_from_channel` (`cpu.rs:1055` to `:1076`): non-finite or `<= 0` is 0, `>= 255` is 255, otherwise the greatest code whose preceding half is at or below the value, which rounds a tie up.

### Transcription flag

PS3.3 C.7.6.3.1.2, C.7.6.3.1.3, C.7.6.3.1.5, C.7.6.3.1.6 and C.7.9 are not in the repository. The text above is the library's own transcription, written by F-030 and generated by the same process this story is checked against. AC17 makes the reviewer check the forward matrices, the 4:2:2 wire order, the planar rule and the palette descriptor against the current edition at dicom.nema.org before the fixtures are accepted, as F-212's AC13 does for its rows.

### The JPEG 2000 component transforms, ISO/IEC 15444-1 Annex G, as cited by PS3.5 8.2.4 and A.4.4

Neither document is in the repository and `docs/hld/` does not transcribe them, so this is the lead's transcription and AC17 adds both to the reviewer's check. PS3.5 8.2.4 permits `YBR_RCT` only with the reversible transform and `YBR_ICT` only with the irreversible one, and says the Photometric Interpretation names the codestream's own component transform. Annex G applies the forward transform after the DC level shift, so a decoder applies the inverse before undoing the shift. With `I0 I1 I2` the level-shifted `R G B` and `Y0 Y1 Y2` the transformed components:

```text
RCT, G.2, reversible, integer:
  forward  Y0 = floor((I0 + 2 I1 + I2) / 4)   Y1 = I2 - I1   Y2 = I0 - I1
  inverse  I1 = Y0 - floor((Y2 + Y1) / 4)     I0 = Y2 + I1   I2 = Y1 + I1

ICT, G.3, irreversible, real:
  inverse  I0 = Y0 + 1.402 Y2
           I1 = Y0 - 0.34413 Y1 - 0.71414 Y2
           I2 = Y0 + 1.772 Y1
```

RCT's `Y1` and `Y2` span one bit more than the image depth, `-(2^B - 1) ..= 2^B - 1` at depth `B`. That is why the order of the inverse and the level shift is load-bearing below.

### cornerstone3D 5.8.2's colour rules, the reference this story follows

These are not normative and PS3.3 C.11.2 does not define a window for a colour image. The operator chose on 2026-11-07 to follow the oracle on window, invert and display scaling, so the rules are quoted here as the reference the fixtures are computed from. All under `tools/oracle/node_modules/@cornerstonejs/`, read 2026-11-07, `package-lock.json` pinning `dicom-image-loader` 5.8.2.

`core/dist/esm/RenderingEngine/helpers/cpuFallback/rendering/getVOILut.js:1` to `:7`:

```js
function generateLinearVOILUT(windowWidth, windowCenter) {
    return function (modalityLutValue) {
        const value = ((modalityLutValue - (windowCenter - 0.5)) / (windowWidth - 1) + 0.5) *
            255.0;
        return Math.min(Math.max(value, 0), 255);
    };
}
```

`generateColorLUT.js:13` to `:21`, one table for all three channels, inversion after the window:

```js
    if (invert) {
        for (let storedValue = minPixelValue; storedValue <= maxPixelValue; storedValue++) {
            lut[storedValue + -offset] = 255 - vlutfn(storedValue);
        }
    }
    else {
        for (let storedValue = minPixelValue; storedValue <= maxPixelValue; storedValue++) {
            lut[storedValue + -offset] = vlutfn(storedValue);
        }
    }
```

`storedColorPixelDataToCanvasImageData.js:23` to `:31` indexes that one `lut` with the R, G and B sample in turn and writes alpha 255. The table is a `Uint8ClampedArray` (`generateColorLUT.js:9`), so the store rounds to nearest with ties to even. No modality rescale enters the colour path, and `renderColorImage.js:17` passes no `voiLUT`, so the function is always linear. The GPU route (`StackViewport.js:1137` to `:1153`, `createLinearRGBTransferFunction.js:10` to `:11`, `invertRgbTransferFunction.js:9` to `:11`) builds the same linear ramp from `lower = center - 0.5 - (width - 1) / 2` to `upper = center - 0.5 + (width - 1) / 2` and inverts after it.

`dicom-image-loader/dist/esm/imageLoader/createImage.js:321` to `:324`, the initial window of every colour image, whatever the file says:

```js
            if (image.color) {
                image.windowWidth = 256;
                image.windowCenter = 128;
            }
```

At width 256 and centre 128 the ramp is the identity on `0 ..= 255`. Initial invert is false for colour, because it is true only for `MONOCHROME1` (`createImage.js:252`).

`colorSpaceConverters/convertPALETTECOLOR.js:2` to `:9` and `:24`, the palette entry scaling:

```js
function convertLUTto8Bit(lut, shift) {
    const numEntries = lut.length;
    const cleanedLUT = new Uint8ClampedArray(numEntries);
    for (let i = 0; i < numEntries; ++i) {
        cleanedLUT[i] = lut[i] >> shift;
    }
    return cleanedLUT;
}
    const shift = bitsStored > 8 && rData.some((num) => num > 255) ? 8 : 0;
```

Colour samples above 8 bits are not scaled anywhere in 5.8.2. Native `RGB` with Planar Configuration 0 keeps its `Uint16Array` and saturates at 255 through the clamp above. Planar, `YBR_*` and palette routes copy into a `Uint8ClampedArray` (`createImage.js:152` to `:159`), which also saturates.

`createImage.js:161` to `:213` in this install carries a `console.log` debug block that the published 5.8.2 package is not expected to contain. It changes no value. The implementer confirms the installed tree against the lockfile integrity before AC20's reference observation, and the finding is recorded rather than repaired here, because `tools/oracle/node_modules` is not tracked.

## Facts measured for this plan

Every claim here was read in the tree. Line numbers are at 636fdcc1.

**Where the folder viewer refuses colour today.**

| Place | What it does |
|-------|--------------|
| `examples/folder-viewer/src/preview.rs:203` to `:205` | `PreviewError::NotMonochrome`, "this preview shows monochrome frames only", code `not-monochrome` at `:179` |
| `preview.rs:613` to `:616` | `frame_parts` refuses Samples per Pixel other than 1 |
| `preview.rs:648` to `:649` | `SampleLayout::new(1, photometric, None)` |
| `preview.rs:1151` to `:1157` | `photometric` accepts only `MONOCHROME1` and `MONOCHROME2` |
| `preview.rs:1373` to `:1377` | an encapsulated frame whose decoder reports anything but `Preserved` is `NotMonochrome` |
| `preview.rs:2349` to `:2353`, `:2439` | `inspect_json` writes `"monochrome"` true only for the two monochrome codes with one sample |
| `page.html:1984` to `:1993` | `showIndex` hides the stage and shows the fault sentence |
| `page.html:1664` | a file that is not monochrome is listed as one image, whatever Number of Frames says |
| `page.html:1810`, `:2022`, `:2249`, `:2265`, `:2406` | thumbnails, the main pump, prefetch and tiles skip a frame that is not monochrome |
| `page.html:2564` | the corner reads `Window not applicable` |
| `page.html:2733` to `:2740` | presets are off, "This image is not monochrome, so it has no window to set." |
| `page.html:3286` | the value request returns early, so the readout says nothing |
| `page.html:4886`, `:4892` | F-209's window link neither sends from nor sends to a frame that is not monochrome |

**How ocelli-pixel's colour API is called today.** No production crate calls it. `grep -rn 'ColorTransform' crates/*/src examples/*/src` finds `ocelli-pixel` itself, `ocelli-render/src/cpu.rs` and `ocelli-render/src/colour.rs`, and the folder viewer's `use.html` prose. The one end-to-end caller is the oracle's candidate producer, `tools/oracle/src/produce_current.rs:402` to `:423`, which reads the decoder's report through `Registry::decode_photometric_interpretation` (`:454` to `:471`), resolves with `DecodedPhotometric::Rgb` or `Preserved` and `DecodedLayout::Interleaved` for a decoded or encapsulated frame, and draws through `CpuRaster::raster_color_into` (`:2139` to `:2175`). It reads the three palette tables itself (`:578` to `:687`), and that is the only Palette Color Lookup Table reader in the tree. `ocelli_dicom::frame_open` selects a `ColourSelection` (`frame_open.rs:107` to `:116`, `:472` to `:487`) and reads no palette table.

**What each decoder reports.** The registry asks two questions per frame, `decode_photometric_interpretation` and `decode_sample_layout` (`crates/ocelli-codec/src/registry.rs:543` to `:563`). The trait defaults are `Preserved` and `Preserved` (`registry.rs:368` to `:379`).

| Decoder | Colour shapes accepted | Photometric reported | Layout reported |
|---------|------------------------|----------------------|-----------------|
| Raw, native | any, sized by Samples per Pixel | `Preserved` (default) | `Preserved` (default) |
| RLE, `rle.rs` | `PALETTE COLOR` 8 or 16 unsigned, `RGB` and `YBR_FULL` 8 or 16 unsigned (`rle.rs:70` to `:79`) | `Preserved` (default) | `Interleaved` (`rle.rs:18` to `:20`) |
| JPEG, `jpeg.rs` | three samples at 8 bits only (`jpeg.rs:179` to `:184`) | `Rgb` for three samples, except extended process with Bits Stored other than 8 (`jpeg.rs:115` to `:126`) | `Interleaved` (`jpeg.rs:128` to `:130`) |
| JPEG-LS, `jpegls.rs` | lossless 8-bit interleaved `RGB` only (`jpegls.rs:163` to `:171`) | `Rgb` for that shape (`jpegls.rs:95` to `:104`) | `Interleaved` for that shape (`jpegls.rs:106` to `:112`) |
| JPEG 2000, `jpeg2000.rs` | none, one sample and monochrome only (`jpeg2000.rs:109` to `:121`), and one component with no MCT (`:375`) | `Preserved` (default) | `Interleaved` (`jpeg2000.rs:72` to `:74`) |
| HTJ2K, `htj2k.rs` | none, one sample and monochrome only (`htj2k.rs:139` to `:150`) | `Preserved` (default) | not overridden |

So `YBR_ICT` and `YBR_RCT`, which PS3.3 permits only in a JPEG 2000 codestream, cannot be decoded by this build at all. F-229's JPEG XL decoder is in progress on `work/f-229-xai` and is not on the sprint head.

**What the two JPEG 2000 dependencies can do, measured for question 2.** Both are vendored (`Cargo.toml:4`, `:109`, `:122`).

| Dependency | Several components | Component transform | Where |
|------------|--------------------|---------------------|-------|
| `ritk-codecs` 0.6.0, JPEG 2000 Part 1 | **No.** "J2K: native decode currently supports one grayscale component" | **Refused**, "does not implement the multiple-component transform". No RCT or ICT code in `src/jpeg_2000` | `vendor/ritk-codecs-0.6.0/src/jpeg_2000/image.rs:55` to `:60`, `:79` to `:84`. Also LRCP only (`:73`) and 1 by 1 sampling (`:122` to `:132`) |
| `openjph-core` 0.1.0, HTJ2K | **Yes**, one component at a time through `Codestream::pull(comp_num)` | **Read and never applied.** `is_employing_color_transform` exists (`src/params/local.rs:864` to `:866`) and `rct_backward` and `ict_backward` exist in `src/transform/`, with no call site on the decode path | `src/codestream/mod.rs:298`, `src/codestream/local.rs:586`, `src/codestream/tile.rs:371` |

**The HTJ2K route has a defect that an adapter cannot undo.** `openjph-core` undoes the DC level shift per component and clamps to `0 ..= 2^B - 1` immediately after the inverse wavelet (`vendor/openjph-core-0.1.0/src/codestream/tile_comp.rs:346` to `:367`). Annex G's inverse transform has to run before that shift, and RCT's chroma spans one more bit than the clamp keeps. A worked case at `B = 8`: pure red `255 0 0` level-shifts to `127 -128 -128`, RCT gives `Y0 = floor(-257 / 4) = -65`, `Y1 = 0` and `Y2 = 255`, and `Y2 + 128 = 383` is clamped to 255. An adapter that subtracts the shift again and inverts gets `I1 = -65 - floor((127 + 0) / 4) = -96` and `I0 = 127 - 96 = 31`, so red 159 where the source was 255, with no error. A correct colour HTJ2K decode therefore needs a change inside the vendored crate. The vendored package is byte-identical to the published archive and the pins gate proves it (`scripts/pin_and_size_check.py:122` and onward), so the change is a patch with its own provenance, as `ritk-codecs` already carries for its manifest (`scripts/pin_and_size_check.py:100` to `:116`). Question 9's answer is that patch, D-31.

**The adapters refuse today, by test.** `crates/ocelli-codec/src/jpeg2000.rs:110` to `:119` and `:374` to `:377`, `htj2k.rs:138` to `:150` and `:287` to `:289` refuse more than one component or a non-zero MCT. `crates/ocelli-codec/tests/jpeg2000.rs:245` to `:253`, `:269` to `:294` and `:318` to `:319`, and `tests/htj2k.rs:284` to `:317` and `:343` to `:351` assert those refusals. They become positive fixtures or move to the shapes still refused. `parse_siz` (`jpeg2000.rs:344` to `:371`) reads Ssiz for component 0 only and has to return every component's depth, signedness and sampling.

**The corpus has no colour JPEG 2000 row and no ICC row.** `corpus/manifest.tsv` holds `syntax/j2k_lossless.dcm`, `j2k_lossy.dcm`, `htj2k_lossless.dcm`, `htj2k_lossless_rpcl.dcm` and `htj2k_lossy.dcm`, all `mono16`. No row's category names `ybr-rct`, `ybr-ict` or an ICC profile. `scripts/corpus_synth.py` already encodes through OpenJPEG (`:188` to `:190`, `:1033` to `:1039`) and `ojph_compress` (`:837` to `:859`), which is where the new rows come from.

**A native 4:2:2 frame is refused today, not misread.** `FrameDesc::output_len` is `Rows * Columns * Samples per Pixel * Bits Allocated / 8` (`registry.rs:304` to `:319`), and `RawDecoder::index_native_value` sizes the whole value from `frame_bits`, which multiplies by Samples per Pixel (`native.rs:96`, `native.rs:339` to `:345`). For `YBR_FULL_422` that is three samples per pixel, while PS3.3 C.7.6.3.1.2 stores two, as `StoredPixelDescription::stored_samples_per_pixel` already says (`crates/ocelli-pixel/src/stored_pixel.rs:125` to `:150`). The value is then shorter than expected and `index_native_value` returns `InvalidCodestream` (`native.rs:117` to `:119`). The folder viewer's `borrowed_pixels_fit` makes the same multiplication (`preview.rs:1255` to `:1270`). The JPEG decoder's output for the same photometric value is three samples per pixel, so the one length cannot serve both routes.

**The folder viewer's memory terms are one byte per pixel.** The reply body is "One display byte per pixel" (`preview.rs:113`). `grey_bytes` is `width * height` (`serve.rs:875` to `:879`). `GREY_CAP` is "the resident bound's grey term `P * N_max` at one grey byte per pixel" (`held.rs:370` to `:375`). The page checks `bytes.length !== width * height` (`page.html:2201` to `:2206`) and `PREPARED_PIXEL_CAP` compares a byte length with a pixel count (`page.html:853`, `:2302`). The held decoded samples are charged by buffer capacity (`preview.rs:510`), so three samples per pixel are already counted there. The workspace counts `scratch` as `Stored` per sample, `native` and `grey` (`held.rs:305` to `:311`, `:355` to `:362`).

**cornerstone3D 5.8.2, the reference, read under `tools/oracle/node_modules/@cornerstonejs/`.** `dicom-image-loader/dist/esm/imageLoader/convertColorSpace.js:18` to `:40` copies `YBR_ICT`, `YBR_RCT` and `YBR_PARTIAL_420` through as RGB, on the decoder's word. `colorSpaceConverters/convertYBRFullByPixel.js` uses `1.402`, `0.34414`, `0.71414` and `1.772`. `colorSpaceConverters/convertPALETTECOLOR.js:24` shifts every 16-bit entry right by 8 when any entry is above 255. `core/dist/esm/RenderingEngine/helpers/cpuFallback/rendering/renderColorImage.js:10` to `:22` builds a per-channel colour LUT from the viewport window and `invert`, so cornerstone applies a window and an inversion to a colour image. The oracle writes colour, palette and YBR candidates and does not score them (`produce_current.rs:3` to `:4`, `:375` to `:383`).

## What the specification does not cover

Every item is this plan's decision unless it names an open question.

1. **Where the byte rounding lives.** The library leaves rounding to "the render or export boundary" (`color.rs:26` to `:30`). The folder viewer reply is such a boundary. It reuses tier C's rule from `ocelli-render` rather than writing one. `ocelli-render` gains one public function that runs the existing steps without resampling, and `raster_color_into` is refactored to call the same steps, so the rule exists once.
2. **The reply format.** Three bytes per pixel, `R G B` row major, with `X-Ocelli-Channels: 3`. A monochrome reply keeps one byte and gains `X-Ocelli-Channels: 1`. RGBA was rejected because the alpha byte is constant and costs a quarter of every colour reply under the cap.
3. **Which photometric values are drawn.** `RGB`, `YBR_FULL`, `YBR_FULL_422`, `PALETTE COLOR`, and `YBR_ICT` or `YBR_RCT` whenever a decoder reports `Rgb` for them. `YBR_PARTIAL_422` is drawn too, because the library route exists and a local refusal would be a rule with no source. `YBR_PARTIAL_420` is not a code `frame_open` or the folder viewer knows, so it is refused by name.
4. **Value at the pointer on a colour frame.** D-23 says PS3.3 C.11's chain does not apply to a three-sample frame, and a palette frame's stored value is an index, not a modality value. So stage 1 is not built for either and no modality value exists. The pointer readout shows the R, G and B values instead, as cornerstone3D's probe shows a colour voxel's components (`tools/dist/esm/tools/annotation/ProbeTool.js:303` and `:398` to `:404`). The values are the colour stage's output after the display scaling and before the window, the same values item 12's statistics use, printed to one decimal place, because a YBR conversion is not an integer. Region statistics are per channel under item 12.
5. **A colour shape the build cannot draw is refused, never approximated.** One new code, `colour-unavailable`, with a sentence that names the reason from a closed list. It replaces `not-monochrome`, which no longer has a meaning.
6. **Scratch is bounded by a band of rows, not by the frame.** `ColorTransform::map_into` takes `&[Stored]` and writes `&mut [Rgb]`, twelve bytes per pixel each way. At `MAX_PIXELS` (`preview.rs:96`) a whole frame would be about 805 MB per buffer. The conversion runs over bands of whole rows with an even row width for 4:2:2, and a planar frame gathers the band's three plane rows into one planar band, which is still one valid `map_into` call because the planar offsets are relative to the destination length (`color.rs:369` to `:374`). The band size is the implementer's, measured and recorded.
7. **The palette tables are read once, in the library.** A new `ocelli_dicom` reader returns `Option<PaletteColorLut>` from the three descriptors and three data elements, refusing Segmented Palette Color Lookup Table Data and a missing table by name. The oracle's private reader is replaced by it, by the lead's answer to question 7.
8. **The colour window is the existing LINEAR function, not a second copy.** The operator's answers to questions 3 and 4 follow cornerstone3D's rule, and that rule is PS3.3 C.11.2.1.2's LINEAR with output range `0 ..= 255`, character for character: `getVOILut.js:3` is `((x - (c - 0.5)) / (w - 1) + 0.5) * 255` clamped to `[0, 255]`, and `apply_window` at `crates/ocelli-pixel/src/lut.rs:776` to `:788` is the same expression with the clamps written as `x <= lower` and `x > upper`. At `w = 1`, cornerstone divides by zero and ends at 0 or 255 through the clamp, and `apply_window` reaches the same two values by comparison. So the colour window is `VoiTransform` with `VoiFunction::Linear`, `ymin = 0` and `ymax = 255`, applied to each channel, and the colour invert is `PresentationTransform` with `Inverse` over the same range, `0 + 255 - y` (`lut.rs:534` to `:543`). The only new code is the per-channel composition, `ColourWindow`, in `ocelli-pixel`. It does not go through `LutChain`, which D-23 keeps monochrome.
9. **Where the colour window sits.** After stage 4 and after the display scaling of item 11, before the byte rule, because cornerstone3D windows the 8-bit RGB its converters produce. Its initial value is width 256 and centre 128 for every colour image, ignoring the file's Window Center and Width, as `createImage.js:321` to `:324` does. Only LINEAR is offered, because the colour path never reads `voiLUTFunction`.
10. **Rounding at a tie matches the oracle**, by the operator's decision of 2026-11-07 below. cornerstone stores into a `Uint8ClampedArray`, which rounds a tie to even, and tier C's `byte_from_channel` now rounds a tie to even too. Tier C monochrome reaches that rule only with whole table bytes, so by ruling R2 monochrome keeps its rounding through `display_index` and its bytes are measured unchanged, 0 of 102 views. AC21 measures the colour tie. A second source of one-code divergence is cornerstone rounding its YBR and palette conversions to bytes before the window, while the library windows the `f32` stage 4 output. Under D14 these are claimed as measured divergences, not hidden.
11. **The display scaling rule, the operator's answer to question 8.** A colour sample or a palette entry of `n` bits is shifted right by `n - 8` for `9 <= n <= 16`, and taken as it is for `n = 8`. For a sample, `n` is Bits Stored `(0028,0101)`. For a palette, `n` is the third value of each channel's Palette Color Lookup Table Descriptor and ruling R1 gives the one rule: a palette is read as 8-bit only when at least one channel is declared above 8 bits and every entry of every such channel fits in 8 bits, otherwise each channel is shifted by its own declared depth minus 8. Colour with `n` below 8 or above 16, or signed colour samples, is refused by name. A sample is shifted before stage 4, so the 8-bit YCbCr matrix and its `+ 128` offset apply to an 8-bit value. A palette entry is shifted on the way out of the table.
12. **Statistics on a colour region.** One walk over the region, F-211's `region_statistics` in `examples/folder-viewer/src/measure.rs:468`, gathers three channel vectors and calls `roi_statistics` once per channel. The values are the colour stage's output after the display scaling and before the window and invert, so a window drag never changes a statistic, which is the rule F-211 keeps for monochrome. The unit is `RGB`. Padding is not applicable to colour and `padding_excluded` is 0.
13. **The ICC sentence.** A file with ICC Profile `(0028,2000)` at the top level is drawn and the corner and the image's help sentence say "This image's ICC profile was not applied." The presence test is the element alone, whatever its length. An ICC profile inside the Optical Path Sequence is a whole-slide concern this viewer does not open.
14. **The window link.** F-209's window link copies a window between viewports of the same kind only, colour to colour and monochrome to monochrome, because a colour window is in display sample units and a monochrome one is in modality units.
15. **The decoder's output length is the decoder's fact**, by the operator's answer to question 1. `Decoder` gains `decoded_output_len(&self, desc: &FrameDesc) -> usize`, defaulting to `desc.output_len()`, and `Registry` gains the matching query beside `decode_photometric_interpretation` and `decode_sample_layout` (`registry.rs:543` to `:563`). `Registry::decode` checks `out.len()` against the decoder's answer (`registry.rs:581`). The raw decoder answers `Rows * Columns * 2 * Bits Allocated / 8` for `YBR_FULL_422` and `YBR_PARTIAL_422`, from `StoredPixelDescription::stored_samples_per_pixel` (`stored_pixel.rs:125` to `:150`) rather than a second rule. JPEG keeps three samples per pixel. An odd Columns on a native 4:2:2 frame is refused as `FrameMismatch`, because a pixel pair has no partner and PS3.3 names no rule for it. The production callers that size a buffer from `desc.output_len()` move to the query: `examples/folder-viewer/src/preview.rs:708` and `:1282`, and `tools/oracle/src/produce.rs:723`.
16. **The component transform is inverted inside the decoder, once.** PS3.5 8.2.4 makes `YBR_ICT` and `YBR_RCT` the codestream's own transform, and `color.rs:233` to `:237` already refuses them unless a decoder converted them. So both JPEG 2000 adapters invert Annex G before the level shift and report `DecodePhotometricInterpretation::Rgb` and `DecodeSampleLayout::Interleaved` for a three-component codestream with the transform on. A three-component codestream with the transform off reports `Preserved`, so an `RGB` file stays `RGB` and the library's `ColorTransform` decides the rest. The inverse arithmetic lives inside the vendored decoders, by question 9's answer: `openjph-core`'s own `rct_backward` and `ict_backward` on its decode path, and a new three-component path in `ritk-codecs`, each before the level shift and clamp, recorded as D-31. The adapters in `jpeg2000.rs` and `htj2k.rs` own the refusals, the reports and the interleaving. Components with different depths, signedness or sampling are refused as `UnsupportedPixelFormat`, as `openjph-core`'s own `check_validity` requires for the transform (`src/params/local.rs:970` to `:985`).

## Problem and reproducer

Pick a folder holding the corpus row `synthetic/sc_rgb_planar.dcm`, or any RGB secondary capture. The series lists, its thumbnail is empty, and selecting it shows "this preview shows monochrome frames only". `POST /preview` for that file answers 400 with `X-Ocelli-Error: not-monochrome`. A multi-frame colour ultrasound lists as one image (`page.html:1664`). The library has had a colour stage since F-030, so this is absent wiring, not absent arithmetic.

## Scope, API and compatibility

In scope:

- `preview.rs` builds a colour recipe for a colour frame: the data set's `SampleLayout` with Samples per Pixel and Planar Configuration, the `PixelDataEncoding` from the dispatch path, the decoder's `DecodePhotometricInterpretation` and `DecodeSampleLayout` mapped one to one onto `DecodedPhotometric` and `DecodedLayout`, the display scaling from the header, and the palette when the space is palette. `ColorTransform::resolve` decides, once, at hold time.
- `render_held` maps a colour frame through the new `ocelli-render` function into three bytes per pixel, with the viewport's colour window and invert applied per channel by `ocelli-pixel`.
- `inspect_json` gains `"colour"`, true when the frame is one of the drawn codes with the Samples per Pixel PS3.3 gives it, Rows and Columns above zero and a frame count of at least one. `"monochrome"` keeps its meaning. It also gains `"icc"`, true when ICC Profile `(0028,2000)` is present at the top level.
- The page draws, thumbnails, prefetches, tiles and lists every frame of a colour file. Window drag and invert act on a colour image from width 256 and centre 128. Presets and the file's windows are not offered on colour. The pointer readout shows R, G and B. A region reports its area and per-channel statistics.
- `ocelli-pixel` gains `ColourWindow`, the per-channel composition of the existing LINEAR window and inverse presentation stage, and `DisplayScale`, the one shift rule of item 11, used by `ColorTransform` for samples and by `PaletteColorLut` for entries.
- `ocelli-render` exports the byte step. `ocelli-dicom` exports the palette reader and the ICC presence test. `ocelli-codec` gains the output length query, and decodes three-component JPEG 2000 and HTJ2K with their component transform inverted, through the vendored patches of D-31.
- `held.rs` and `serve.rs` charge a colour reply at three bytes per pixel and count the colour scratch.
- `scripts/corpus_synth.py` and `corpus/manifest.tsv` gain five synthetic rows, four colour JPEG 2000 rows `syntax/j2k_lossless_ybr_rct.dcm` (`.90`, OpenJPEG with the transform on), `syntax/j2k_lossy_ybr_ict.dcm` (`.91`), `syntax/htj2k_lossless_ybr_rct.dcm` (`.201`, `ojph_compress` with its colour transform on) and `syntax/htj2k_lossy_ybr_ict.dcm` (`.203`), each from the same saturated RGB reference including pure red, plus `synthetic/sc_rgb_icc.dcm`, an 8-bit RGB frame carrying a synthetic ICC Profile element written by the script. No third-party profile is copied. The data stays under ignored `corpus/data`, and nothing DICOM is staged.
- `tools/oracle/src/produce_current.rs` reads palettes through `ocelli_dicom::palette_color_lut`, and its candidate producer sizes buffers through the output length query.

Non-goals: applying an ICC profile, which is F-240, a modality value or unit on a colour frame, presets and the file's Window Center and Width on a colour image, any VOI LUT function other than LINEAR on colour, segmented palettes, Supplemental Palette Color, `YBR_PARTIAL_420`, JPEG 2000 colour with a progression other than LRCP or with sub-sampled components, signed colour, colour above 16 bits or below 8, scoring the oracle's colour rows, and any change to the synthetic study.

Public signatures added, names final at implementation:

```rust
// ocelli-pixel. Width and centre validated by VoiTransform::new with
// VoiFunction::Linear, ymin 0 and ymax 255. Identity, and skipped, when the
// width is 255 or 256, the centre is 127 or 128 and invert is off, as
// renderColorImage.js:30 to :36 skips its table.
pub struct ColourWindow { /* VoiTransform, Option<PresentationTransform> */ }
impl ColourWindow {
    pub fn new(center: f32, width: f32, invert: bool) -> Result<Self, PixelError>;
    pub fn initial() -> Self; // width 256, centre 128, invert off
    pub fn map_into(&self, source: &[Rgb], destination: &mut [Rgb]) -> Result<(), PixelError>;
}

// ocelli-pixel. Shift right by n - 8 for 9 <= n <= 16, identity at 8.
pub struct DisplayScale { /* bits */ }
impl DisplayScale {
    pub fn from_bits(bits: u16) -> Result<Self, PixelError>;
    pub fn apply(&self, value: f32) -> f32;
}

// ocelli-render, beside CpuRaster. The same accept, map and byte steps as
// raster_color_into, with no view, no resampling and no generation, and the
// colour window between the map and the byte step.
pub fn colour_bytes_into(
    color: RasterColor<'_>,
    window: &ColourWindow,
    stored: &[Stored],
    columns: u32,
    scratch: &mut Vec<Rgb>,
    out: &mut [u8],
) -> Result<(), RasterError>;

// ocelli-dicom. None for a frame that is not PALETTE COLOR.
pub fn palette_color_lut(metadata: &MetadataSet) -> Result<Option<PaletteColorLut>, FrameOpenError>;
// ocelli-dicom. Whether ICC Profile (0028,2000) is present at the top level.
pub fn has_icc_profile(metadata: &MetadataSet) -> bool;

// ocelli-codec, on Decoder with a default, and on Registry.
fn decoded_output_len(&self, desc: &FrameDesc) -> usize;
pub fn decoded_output_len(&self, transfer_syntax: &str, desc: &FrameDesc) -> Result<usize, CodecError>;
```

`DisplayScale::apply` shifts an integer-valued `f32` by `floor(value / 2^(n - 8))`, which is exact for every value up to `2^16` in `f32`.

Protocol strings that change:

| Where | Name | Value |
|-------|------|-------|
| inspect JSON | `colour` | `true` or `false` |
| inspect JSON | `icc` | `true` or `false` |
| preview reply | `X-Ocelli-Channels` | `1` or `3` |
| preview reply body | bytes | `width * height * channels` |
| preview request on colour | `X-Ocelli-Window-Center`, `X-Ocelli-Window-Width`, `X-Ocelli-Invert` | Applied per channel. A window function other than LINEAR or a preset index is `colour-not-applicable` |
| error code | `colour-unavailable` | replaces `not-monochrome` |
| error code | `colour-not-applicable` | a preset, a file window or a non-LINEAR function asked of a colour frame |
| value JSON on colour | `rgb` | `[r, g, b]` as `f32`, with `unit` `RGB` and no modality fields |
| measure JSON | `stats` on colour | `{"unit":"RGB","channels":{"r":{...},"g":{...},"b":{...}}}`, each with `count`, `mean`, `sd`, `min` and `max` |
| page sentence | preset note | "This image is colour, so the file's windows and the presets do not apply. Drag to window each channel." |
| page readout | value at the pointer on colour | `R 46.7 G 119.6 B 139.0` |
| page sentence | ICC | "This image's ICC profile was not applied." |

Refusal sentences for `colour-unavailable`, one per reason: a JPEG 2000 or HTJ2K codestream shape this build does not decode, colour samples outside 8 to 16 bits or signed, a palette descriptor outside 8 to 16 bits, a segmented or missing palette table, an odd Columns on a native 4:2:2 frame, a photometric value this build does not know.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `POST /preview`, `/file` | `decode_held`, then `render_held` with the colour recipe and the request's colour window | Decoder reports read once per held frame | Three bytes per pixel, `X-Ocelli-Channels: 3` |
| Colour space and layout | `ColorTransform::resolve` | Inputs from the data set, `DispatchPath` and the registry | One resolved transform per held frame |
| Display scaling | `ocelli_pixel::DisplayScale` | Bits Stored for samples, each palette descriptor's third value for entries unless ruling R1 reads the whole palette as 8-bit | 8-bit-scale `f32`, or a named refusal |
| Colour window and invert | `ocelli_pixel::ColourWindow` over `VoiTransform` LINEAR and `PresentationTransform` Inverse | Initial 256 and 128, skipped at cornerstone's identity condition | Windowed `Rgb` |
| Byte rounding | `ocelli_render::colour_bytes_into` | Tier C's `byte_from_channel` and refusals | Bytes, or `Unavailable` |
| Palette tables | `ocelli_dicom::palette_color_lut` | Top-level elements only | `PaletteColorLut` or a named refusal |
| ICC presence | `ocelli_dicom::has_icc_profile` | Top-level element only | `icc` in inspect JSON, the page sentence |
| Decoded frame length | `Registry::decoded_output_len` | 4:2:2 native stores two samples, JPEG three | A native `YBR_FULL_422` frame decodes |
| JPEG 2000 and HTJ2K colour | `jpeg2000.rs` and `htj2k.rs` adapters, patched under D-31 | Inverse Annex G before the level shift | Interleaved RGB, reported `Rgb` |
| `POST /value` | `value_json`, colour arm through the band routine for one pixel | Colour has no stage 1 | `rgb` triple |
| `POST /measure` | `measure_json` and F-211's `region_statistics` | One walk, three channel vectors, window never enters | Lines, area and per-channel statistics |
| Folder scan | `inspect_json` | `colour` and `icc` flags | Every frame listed |
| Thumbnails, prefetch, tiles | `requestGrey` and the page cache | Same reply at the initial colour window | RGB thumbnails and tiles |
| F-209 window link | `copyWindow` | Same kind only, item 14 | Colour to colour, monochrome to monochrome |
| F-209 scroll and zoom link, reference lines | `planes` in inspect JSON | Photometric independent | Unchanged |
| Presets, W/L drag, invert | page tool state | Drag and invert on, presets off, on colour | Requests carry a colour window and invert |
| Oracle candidate producer | `produce_current.rs` through `palette_color_lut` and `decoded_output_len` | One palette reader | Unchanged candidate bytes for both corpus palette rows, which ruling R1's fallback reads as 8-bit |
| F-213 presets and region window, pending | F-213 | Must keep presets off on colour | Recorded for F-213's design |

The rounding owner is `ocelli-render`. The selection owner is `ColorTransform::resolve`. The window and scaling owner is `ocelli-pixel`. The inverse component transform owner is `ocelli-codec`. The folder viewer owns only which reason sentence a refusal shows and the window the viewport holds.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Folder scan | An RGB file of 3 frames | Three images listed, thumbnail RGB | A JPEG 2000 file of a refused shape lists and its cell shows the named refusal |
| Viewport shows a CT, monochrome | Next series is RGB | Stage shows RGB at width 256 and centre 128, presets off, corner shows the colour window | The CT's window stays on the CT viewport record |
| Viewport shows RGB with window link on | Window dragged in a CT viewport | The CT window is copied to other monochrome viewports only | The RGB viewport's reply is unchanged |
| Held colour frame | `POST /preview` with `X-Ocelli-Window-Center` and `-Width` | 200, the window applied per channel | The held frame is not decoded again |
| Held colour frame | `POST /preview` with `X-Ocelli-Invert: 1` | 200, `255 - y` per channel after the window | Same |
| Held colour frame | `POST /preview` with a window function other than LINEAR | 400 `colour-not-applicable` | The held frame and the last shown image stay |
| Held colour frame | `POST /preview` with width below 1 | 400, the existing invalid-window refusal | Same |
| Held colour frame | `POST /value` | 200 with `rgb` | `stored_at` is not called |
| Colour cell | Pointer moves | Readout shows R, G and B | One request in flight and the latest pixel wanted, as `requestValue` and `sendValue` already do for monochrome (`page.html:3284` to `:3323`), with the `info.monochrome` gate at `:3286` widened to colour |
| Colour cell | Rectangle drawn | Area and R, G and B statistics shown | A later window drag leaves the statistics unchanged |
| Colour file with ICC Profile | Opened | Drawn, ICC sentence shown | Nothing refused |

Invariants across consumers: a colour frame never reaches `LutChain`, and a monochrome frame never reaches `ColorTransform` or `ColourWindow`, as D-23 makes true by construction for the first two and the recipe arms make true for the third. `stored_at` (`value.rs:345`) reads one sample per pixel and is never called on a colour frame, because the held value is an error for every colour frame and the colour region walk reads the colour stage's output instead.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Held colour frame | `decode_held` under F-216's cache | File id and frame, as F-216 and F-224 key it | Admission to the `Lru` | A refused resolve holds the refusal in the recipe, so a retry gives the same refusal without a second decode |
| Colour reply buffer | `render_capped` under the grey gate | One reply | Reply written | Charged at `width * height * 3` from admission. A refused render charges nothing |
| Colour scratch | the decode workspace | The worker | Returned with the workspace | Counted in `workspace_bytes`. The bounded colour vectors are retained per conversion workspace and their actual capacities remain counted in workspace bytes. The existing over-twice rule applies to the grey reply |
| Page cache entry | `rememberGrey` | `cacheKey` | `cache.set` | Byte length counts against `CACHE_BYTE_BUDGET` as it does today |

Cache validity. A colour reply depends on the file, the frame, the colour window's width and centre, and invert. The page's `cacheKey` (`page.html:2065` to `:2070`) writes a `colour` token in place of the window function token for a colour image, so a monochrome and a colour entry cannot collide, and keeps the window and invert tokens. A refused request does not replace the last shown image. The held frame carries the palette, up to `3 * 65536` `f32` entries, and `charged_bytes` (`preview.rs:510`) adds its heap bytes. Statistics depend on the region and the held frame only.

## Numbered acceptance inventory

Hand-computed values below use the library's inverse at `color.rs:441` to `:446` in `f32` and the byte rule at `cpu.rs:1055` to `:1076`. The textbook BT.601 inverse gives the same bytes for every row here, so these rows do not distinguish the two matrices. The library's own fixture does, at a tolerance twenty times tighter than their 0.020028 divergence (`crates/ocelli-pixel/tests/color.rs:20` to `:24`).

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Native RGB 2 by 1, Planar Configuration 0, stored `10 20 30 200 100 50`. Reply `10 20 30 200 100 50`, `X-Ocelli-Channels: 3`. PS3.3 C.7.6.3.1.3 | `rgb_interleaved_is_copied_per_pixel` | A reply of one byte per pixel, or the R channel alone | implementation |
| AC2 | Native RGB 2 by 1, Planar Configuration 1, stored `10 200 20 100 30 50`. Reply `10 20 30 200 100 50`. PS3.3 C.7.6.3.1.3 | `rgb_planar_is_gathered_by_plane` | Reading planar as interleaved, which gives `10 200 20 100 30 50` | implementation |
| AC3 | Native YBR_FULL 1 by 1, `Y Cb Cr` `100 150 90`. Channels `46.72366 119.56544 138.98863`, reply `47 120 139`. Native YBR_FULL 1 by 1, `30 200 60`. Channels `-65.3378 53.7829 157.59158`, reply `0 54 158`. PS3.3 C.7.6.3.1.2 | `ybr_full_uses_the_library_inverse_and_the_tier_c_byte_rule` | No conversion, which gives `100 150 90`. A wrapping store, which keeps `-65` as 191 the way a `Uint8Array` assignment does | implementation |
| AC4 | Native YBR_FULL 2 by 1, Planar Configuration 1, stored `100 30 150 200 90 60`. Reply `47 120 139 0 54 158` | `ybr_full_planar_is_converted_after_gathering` | Converting before gathering planes | implementation |
| AC5 | Native YBR_FULL_422 2 by 1, stored `Y1 Y2 Cb Cr` `50 200 100 160`, Value Length 4. `Registry::decoded_output_len` answers 4 and the reply is `95 37 0 245 187 150`. The same frame with Columns 3 is refused `colour-unavailable` with the odd-Columns sentence. PS3.3 C.7.6.3.1.2 replication | `ybr_full_422_native_shares_one_chroma_pair` | Sizing by Samples per Pixel, which refuses the 4-byte value today, or reading `Y Cb Cr` per pixel | implementation |
| AC6 | Native YBR_PARTIAL_422 2 by 1, stored `16 235 128 128`. Reply `0 0 0 255 255 255`. A full-range matrix gives `16 16 16 235 235 235` | `ybr_partial_422_applies_the_studio_offset` | The full-range matrix on a partial-range frame | implementation |
| AC7 | PALETTE COLOR 5 by 1, 8-bit stored `9 10 12 13 99`, descriptors `4\10\16`, red `0 85 170 255`, green `255 170 85 0`, blue `10 20 30 40`. Reply `0 255 10 0 255 10 170 85 30 255 0 40 255 0 40`. PS3.3 C.7.6.3.1.5 first mapped input and clamping | `palette_maps_the_stored_value_from_the_first_mapped_input` | Ignoring the first mapped input, which gives `255 0 40` for stored 9, or windowing before the palette, which D-23 forbids | implementation |
| AC8 | RLE YBR_FULL 2 by 1 with Planar Configuration 1 declared, three literal segments `100 30`, `150 200`, `90 60`. Reply `47 120 139 0 54 158`. The decoder reports `Preserved` and `Interleaved` (`rle.rs:18` to `:20`), and encapsulated forces interleaved (`color.rs:265` to `:272`) | `rle_ybr_full_is_converted_once_and_planar_is_ignored` | Honouring the declared planar on decoder output, or skipping the conversion | implementation |
| AC9 | JPEG baseline wrapped from `crates/ocelli-codec/tests/fixtures/jpeg_baseline_rgb.jpg` with Photometric Interpretation `YBR_FULL_422`. Reply equals `Registry::decode` output byte for byte | `jpeg_rgb_output_is_not_converted_twice` | A second YBR conversion of decoder RGB, which darkens and shifts hue (`color.rs:210` to `:214`) | implementation |
| AC10 | The four new corpus rows and the existing refusals. A JPEG 2000 or HTJ2K codestream with three components and the transform on decodes and reports `Rgb` and `Interleaved`. Two components, components of unequal depth, sub-sampled components, and a JPEG 2000 progression other than LRCP still answer 400 `colour-unavailable` with the codestream sentence. The existing refusal tests in `tests/jpeg2000.rs` and `tests/htj2k.rs` are kept for those shapes and moved off the three-component one | `ycc_from_jpeg_2000_decodes_once_and_other_shapes_are_refused` | Any image at all for a refused shape, or the YBR_FULL matrix applied to RCT | implementation, codec part |
| AC11 | Display scaling, from the header. Native RGB, Bits Allocated 16, Bits Stored 12, stored `4095 2048 15`. Reply `255 128 0`. The same samples at Bits Stored 16 reply `15 8 0`. Native YBR_FULL at Bits Stored 12, `1600 2400 1440`, shifts to `100 150 90` and replies `47 120 139` as AC3. Bits Stored 7, Bits Stored 17 in a 32-bit container, and Pixel Representation 1 refuse `colour-unavailable` by name. PS3.3 C.7.6.3.1 Bits Stored, the rule of item 11 | `colour_above_8_bits_is_shifted_by_bits_stored` | cornerstone's saturation, which gives `255 255 15`. Shifting by Bits Allocated, which gives `15 8 0` at Bits Stored 12. A data-dependent rule | implementation |
| AC12 | A window on a colour frame is applied per channel, and a following request at the initial window draws the same bytes as before. A window function other than LINEAR answers 400 `colour-not-applicable` and the last shown image stays | `a_colour_window_is_applied_per_channel_and_the_frame_survives` | Ignoring the header and answering 200 with the unwindowed image, or windowing R only | implementation |
| AC13 | Native RGB 2 by 1, stored `10 20 30 200 100 50`, a rectangle over both pixels. `stats.unit` is `RGB`. R count 2, mean 105, SD 95, min 10, max 200. G mean 60, SD 40, min 20, max 100. B mean 40, SD 10, min 30, max 50, SD as `roi_statistics`'s population form (`crates/ocelli-geom/src/stats.rs:154` to `:160`). The same rectangle after a window of width 100 and centre 128 gives the same statistics. `POST /value` at pixel 1 answers `rgb` `[200, 100, 50]` and no modality field. Native YBR_FULL `100 150 90` answers `[46.72366, 119.56544, 138.98863]` and the page prints `R 46.7 G 119.6 B 139.0`. A length reports its millimetres | `colour_regions_report_per_channel_statistics_from_one_walk` | Reading the R sample as a modality value through `stored_at`. Statistics of the windowed bytes. Three walks, which a counting test of `region_contains` calls rejects | implementation |
| AC14 | `inspect_json` of an RGB file of 3 frames has `"colour":true` and `"monochrome":false`. The page lists 3 images | `inspect_marks_colour_and_the_page_lists_every_frame` | A colour cine listed as one image | implementation |
| AC15 | Memory. A colour reply is charged `width * height * 3` under the grey gate, the colour scratch counts in `workspace_bytes`, and F-216's resident bound is re-measured with a colour series of the largest admitted frame and recorded beside F-216's figure | `a_colour_reply_charges_three_bytes_per_pixel`, plus the recorded measurement | Charging one byte per pixel, which lets three times the cap be resident | implementation, measured by the implementer |
| AC16 | In a browser through F-208's harness, a folder of the AC1, AC5, AC7 and AC26 files: each cell's canvas pixel at the centre of source pixel 0 reads the hand value, the thumbnail is not black, a window drag changes the colour cell and AC19's values appear at the dragged window, invert gives `255 - y`, presets are off with the colour sentence, the readout shows R, G and B at the pointer, the ICC sentence shows on the AC26 file only, and a CT beside it still windows | `examples/folder-viewer/tests/browser/colour.mjs` | A canvas filled from the first channel only, or a window drag that sends nothing for colour | integrated |
| AC17 | Before the fixtures are accepted, the reviewer checks PS3.3 C.7.6.3.1, C.7.6.3.1.2, C.7.6.3.1.3, C.7.6.3.1.5, C.7.6.3.1.6 and C.7.9 and PS3.5 8.2.4 and A.4.4 at dicom.nema.org, current edition, and ISO/IEC 15444-1 Annex G, against the matrices, transforms and rules quoted above, and names the editions read | Review note | Fixtures citing text nobody checked | reviewer, before implementation |
| AC18 | Mutation. Swapping two inverse coefficients in `FULL_RANGE`, dropping the `+ 16` offset, making the planar gather interleaved, using `w` for `w - 1` in the colour window, shifting by Bits Allocated, swapping `Y1` and `Y2` in the inverse RCT, and moving the inverse after the level shift turn AC3, AC6, AC2, AC19, AC11 and AC24 red, the last through its pure red case | Recorded in the review | A test that passes on the wrong code | implementation, reviewer |
| AC19 | Colour window, cornerstone3D's rule through the existing LINEAR function, `f32`, then the tier C byte rule. Native RGB 1 by 1, stored `100 30 200`, width 100, centre 128: channels `56.666664 0 255`, reply `57 0 255`. With invert: `198.33334 255 0`, reply `198 255 0`. Stored `128 128 128` at the same window: `128.78787`, reply `129 129 129`, inverted `126 126 126`. Width 256 and centre 128, stored `10 200 50`: reply `10 200 50`. Width 255 and centre 127, stored `100 100 100`: skipped as `renderColorImage.js:30` to `:36` skips, reply `100 100 100`, where the formula would give `100.8957` and `101`. Width 2, centre 128, stored `127 128 0`: `0 255 0`. Width 10, centre 128, stored `125 128 130`: `56.666664 141.66667 198.33334`, reply `57 142 198`, inverted `198 113 57` | `colour_window_is_linear_per_channel_and_inverted_after` | `w` in place of `w - 1`, which gives `64 140 191` at width 10. LINEAR_EXACT's `c` and `w`, which gives `51 128 179`. Inverting before the window. Applying the formula at cornerstone's identity condition. The width 100 rows alone do not reject the `w` mutation, which is why the width 10 row exists | implementation |
| AC20 | Reference observation of the floor. The oracle's reference half already renders every colour corpus row through cornerstone3D 5.8.2 under headless Chromium on SwiftShader (`produce_current.rs:375` to `:383`). For each colour row cornerstone can render, including the four new JPEG 2000 rows, the candidate bytes from `colour_bytes_into` at the initial window are compared per pixel with the reference, and the largest difference and the count of differing pixels are recorded per row. `synthetic/sc_rgb_interleaved.dcm` is also rendered at width 100 and centre 128, invert off and on, through one new `render-params.json` row per case, and compared the same way. Every difference is attributed to a row of "cornerstone3D 5.8.2 is the floor", and a difference no row explains is a defect. The comparison is a recorded observation, not a scored oracle row, because HLD 25.1 states no colour threshold. Missing code: the two parameter rows and the colour window in `produce_current.rs` | Recorded in `docs/lld/oracle.md` with the 5.8.2 pin and the lockfile integrity check | A rule transcribed from memory rather than the oracle | integrated, implementer, on the host the oracle gate already uses |
| AC21 | Tie divergence, measured. Native RGB 1 by 1, stored `130 130 130`, width 511, centre 128.5: `128.5` exactly, reply `128 128 128` by `byte_from_channel`, ties to even, as cornerstone stores. Inverted, `126.5` replies `126 126 126`. Centre 130.5 and width 100 put stored 130 on `127.5`, which replies 128, so both tie directions are asserted | `a_colour_window_tie_rounds_half_to_even` | A rule that always rounds a tie up or always down | implementation |
| AC22 | Palette scaling. PALETTE COLOR 5 by 1, 8-bit stored `0 1 2 3 4`, descriptors `5\0\16` for all three, red `0 255 256 32768 65535`, green and blue `65535 32768 256 255 0`. Reply `0 255 255 0 128 128 1 1 1 128 0 0 255 0 0`. Three 16-bit descriptors whose entries are all `0 100 255` are read as 8-bit by ruling R1 and reply `0 100 255`, as cornerstone does, with `entries_read_as_8_bit` set. Three 12-bit descriptors with the same entries do the same. The rule is decided once for the whole palette, so channels `0 100 255`, `0 100 255` and `0 100 256`, all 16-bit, are all shifted, to `0 0 0`, `0 0 0` and `0 0 1`, and the flag is clear. A 12-bit channel in a palette not read as 8-bit shifts by 4. Bits 17 refuse by name | `palette_entries_are_shifted_by_their_descriptor`, `a_palette_is_read_as_8_bit_only_when_every_wide_channel_fits_in_8_bits`, `a_twelve_bit_palette_of_8_bit_entries_is_read_as_8_bit` and `each_palette_channel_is_scaled_by_its_own_depth` | cornerstone's red-only, data-dependent test. Rounding instead of truncating, which gives 128 for 32768 too but 1 for 255 | implementation |
| AC23 | Decoder output length. `decoded_output_len` for native `YBR_FULL_422` 2 by 1 at 8 bits is 4, for native RGB 2 by 1 is 6, and for JPEG baseline `YBR_FULL_422` 2 by 1 is 6. `Registry::decode` refuses a 6-byte buffer for the native 4:2:2 frame with `OutputLength { expected: 4, actual: 6 }` | `crates/ocelli-codec/tests/registry.rs`, `decoded_output_len_is_the_decoders` | A registry that still checks `desc.output_len()` | implementation |
| AC24 | Inverse RCT, ISO/IEC 15444-1 G.2. Hand values at `B = 8`: `200 100 50` level-shifts to `72 -28 -78`, forward `Y0 = floor(-62 / 4) = -16`, `Y1 = -50`, `Y2 = 100`, and the inverse gives `I1 = -16 - floor(50 / 4) = -28`, `I0 = 72`, `I2 = -78`, so `200 100 50`. Pure red `255 0 0` gives `Y0 = -65`, `Y1 = 0`, `Y2 = 255` and back. The lossless rows `syntax/j2k_lossless_ybr_rct.dcm` and `syntax/htj2k_lossless_ybr_rct.dcm` decode byte for byte to their RGB reference | `inverse_rct_is_exact_before_the_level_shift`, and the corpus rows in `crates/ocelli-dicom/tests/corpus.rs` | Inverting after `openjph-core`'s level shift and clamp, which gives `159 32 32` for pure red | implementation, codec part |
| AC25 | Inverse ICT, ISO/IEC 15444-1 G.3. Hand values: `Y0 = -28`, `Y1 = 22`, `Y2 = -38` give `I0 = -28 + 1.402 * -38 = -81.276`, `I1 = -28 - 0.34413 * 22 - 0.71414 * -38 = -8.43354`, `I2 = -28 + 1.772 * 22 = 10.984`, then `+ 128` and the decoder's rounding to `47 120 139`. The lossy rows `syntax/j2k_lossy_ybr_ict.dcm` and `syntax/htj2k_lossy_ybr_ict.dcm` are compared with OpenJPEG's own decode of the same codestream through the corpus tooling, the largest per-channel difference is measured and recorded, and a decode with the inverse left out is shown to differ by more | `inverse_ict_uses_g6_literal_coefficients`, plus the recorded measurement | The YBR_FULL inverse of `color.rs` in place of Annex G's, or no inverse | implementation, codec part |
| AC26 | ICC sentence. `synthetic/sc_rgb_icc.dcm` draws the same bytes as the same frame without the element, inspect JSON has `"icc":true`, and the page shows "This image's ICC profile was not applied." The file without the element has `"icc":false` and no sentence | `an_icc_profile_is_named_as_not_applied` | Refusing the file, or applying nothing and saying nothing | implementation, integrated in AC16 |
| AC27 | One palette reader. `tools/oracle/src/produce_current.rs` calls `ocelli_dicom::palette_color_lut` and its private reader at `:578` to `:687` is deleted. The candidate bytes for `synthetic/sc_palette_color.dcm` are byte-identical before and after at the sprint corpus gate. `synthetic/sc_palette_color_16.dcm` is byte-identical before and after too, because both rows declare 16 bits with every entry at most 255 and ruling R1 reads them as 8-bit | Corpus gate output, recorded | Two readers that agree today and drift later | implementation, sprint corpus gate |
| AC28 | Window link by kind. With link on, a window dragged on an RGB viewport reaches a second RGB viewport and not a CT viewport, and one dragged on the CT reaches the other monochrome viewport and not the RGB one | `examples/folder-viewer/tests/browser/colour.mjs`, link case | A modality-unit window sent to a colour viewport | integrated |
| AC29 | Wasm size. `bin/ocelli.sh gate wasm` passes on the story's tip, which runs `bin/ocelli.sh wasm`, `bin/ocelli.sh wasm-decode` and `scripts/pin_and_size_check.py --with-size` against `ci/wasm-size-budget.json` and `ci/wasm-decode-size.json`, with neither budget file edited and `--accept-size` never run. The byte size of `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm` and `pkg-decode/ocelli_wasm_bg.wasm` is measured on the story's base, the sprint head its work branch starts from, and on its tip with the same commands, and both deltas are recorded. Reachability, measured for this plan on 5451cb7d with `cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none` and again with `--no-default-features --features decode-worker`: the shipped module reaches `ocelli-pixel` and `ocelli-render`, the decode module reaches `ocelli-pixel`, and neither reaches `ocelli-codec`, `ocelli-dicom`, `openjph-core` or `ritk-codecs`. So D-31's patches and the codec part cannot change either module, and the only shipped code this story adds is `ColourWindow`, `DisplayScale` and `colour_bytes_into`, which no `ocelli-wasm` export calls and fat LTO is expected to remove. The implementer reruns both `cargo tree` commands on the tip and records them. Any non-zero delta is explained by symbol, from `twiggy top` or an equivalent listing of the two builds | Recorded in `docs/lld/build-targets.md` | A new edge from `ocelli-wasm` to the codec crates, or a budget re-baselined to make room | implementation, both parts |
| AC30 | No monochrome regression. `bin/ocelli.sh bench viewer.open viewer.first_image viewer.window_step viewer.scroll_rate viewer.resident_memory` runs on the F-216 host with F-216's instrument identity, once on the story's base and once on its tip, and every subject stays within F-216's answered thresholds. Each before and after median is recorded side by side. A run on another host class is `incomparable` and leaves the row open, as F-216 records | Recorded in `docs/lld/benchmarks.md` | A monochrome request that now builds a colour recipe, or a reply header path that allocates per request | external, viewer part, sprint verification |
| AC31 | The colour path measured once. A new `tools/bench` subject `viewer.colour_frame`, registered in `tools/bench/subjects.json` with a runner beside `viewer_window_step.mjs` and given no threshold, opens a synthetic 8-bit RGB frame at the largest admitted size, 8192 by 8192 (`MAX_PIXELS`, `preview.rs:96`), and a synthetic 16-bit PALETTE COLOR frame of the same size, through the release build. It records the first `/preview` (decode, resolve, convert, bytes), the median of 9 colour window steps, and peak `ps -o rss=` sampled every 20 ms as `viewer.resident_memory` does. Run with `bin/ocelli.sh bench viewer.colour_frame` on the F-216 host. Work is done once per held frame where the memory allows: the colour resolve, the palette read, the display scale and the ICC flag are decided at hold time and never per draw. The per-pixel stage 4 map and the window run per draw in bands, because holding the converted `f32` frame would cost 12 bytes per pixel, about 805 MB at `MAX_PIXELS`, and the band scratch bound of item 6 is what peak RSS shows. At the initial window, `ColourWindow` is skipped. The numbers are recorded and are the baseline a later colour story is compared with | Recorded in `docs/lld/benchmarks.md` | A per-draw palette or recipe rebuild, which shows as window steps no faster than the first request | external, viewer part, sprint verification |
| AC32 | Not slower than cornerstone3D. After F-223 integrates, `bin/ocelli.sh bench --compare-cornerstone --pseudo <dir>` runs on the F-216 host with `<dir>` a folder `scripts/corpus_synth.py` writes of synthetic colour series: 8-bit RGB, 8-bit YBR_FULL_422 native, 16-bit PALETTE COLOR, and 512 by 512 frames of each as a series of 200. Open, first image, window step and peak memory are recorded for both viewers side by side. The target is that Ocelli is not slower on any subject and not larger in peak memory, and a subject that misses is recorded with its cause rather than passed. F-223's pixel check runs before any timing, as its plan requires | Recorded in `docs/lld/benchmarks.md` | A colour path measured against nothing | external, viewer part, sprint verification, needs F-223 |

Worked values for AC3, AC5 and AC6. For `100 150 90`, `Cb' = 22` and `Cr' = -38`. `R = 100 - 0.00003682 * 22 + 1.4019876 * -38 = 46.72366`, `G = 100 - 0.3441133 * 22 - 0.7141038 * -38 = 119.56544`, `B = 100 + 1.7719781 * 22 - 0.000134583 * -38 = 138.98863`. For the 4:2:2 pair, `Cb' = -28` and `Cr' = 32`. Pixel 1 is `50 + 1.4019876 * 32 + 0.0010310 = 94.86463`, `50 + 9.6351724 - 22.851322 = 36.78385` and `50 - 49.615387 - 0.0043067 = 0.38031`, which round to `95 37 0`. Pixel 2 adds 150 to each channel, `244.86464 186.78386 150.38031`, which round to `245 187 150`. For the partial pair, `Y' = 0` gives 0 and `Y' = 219` gives `1.1644155 * 219 = 255.007`, which clamps to 255.

## Approach

1. **ocelli-pixel.** Add `DisplayScale` and `ColourWindow` in `color.rs` or a sibling module. `ColourWindow` holds a `VoiTransform` built by `VoiTransform::new(None, &[center], &[width], 0, VoiFunction::Linear, 0.0, 255.0)` and, when inverted, a `PresentationTransform` with `Inverse` over `0 ..= 255`, and maps each channel through `apply` on each. No window arithmetic is written in it. `ColorTransform::resolve` takes the sample `DisplayScale`, and `PaletteColorLut::new` takes one per channel, so the shift happens once at each place a value enters stage 4. Fixtures for both in `crates/ocelli-pixel/tests/color.rs` and `tests/palette.rs`, hand-computed as AC11, AC19, AC21 and AC22.
2. **ocelli-render.** Factor the accept, display-byte check, `map_into` and `rgba_from_rgb` steps of `raster_color_into` (`cpu.rs:363` to `:395`, `:586` to `:599`) into one private routine, call it from `raster_color_into`, and export `colour_bytes_into` writing three bytes per pixel over bands, with `ColourWindow::map_into` between the map and the byte step. `raster_color_into` passes `ColourWindow::initial()`, which is skipped, so its existing tests stay green unchanged.
3. **ocelli-dicom.** Add `palette_color_lut` in `frame_open.rs`, reading `(0028,1101)` to `(0028,1103)` and `(0028,1201)` to `(0028,1203)` into `LutDescriptor::new` and `PaletteColorLut::new`. Refuse `(0028,1221)` to `(0028,1223)` by name. Add `has_icc_profile` for `(0028,2000)`.
4. **ocelli-codec, length.** Add `Decoder::decoded_output_len` and `Registry::decoded_output_len`, override it in the raw decoder for the two 4:2:2 codes, and check the decoder's answer in `Registry::decode`. Move the three production callers.
5. **ocelli-codec, JPEG 2000 and HTJ2K colour.** Return every component from `parse_siz`. Accept three components of equal depth, signedness and 1 by 1 sampling with the transform on or off. Report `Rgb` and `Interleaved` when it is on. The inverse transform and the level shift run in Annex G's order inside the patched crates, HTJ2K first: `openjph-core` calls its existing `rct_backward` or `ict_backward` on the decode path when `is_employing_color_transform` is true, choosing by the wavelet, and its level shift and clamp move after them. Then `ritk-codecs` learns three-component LRCP decoding for Part 1 with the same inverse order, reading its MCT byte as `1` for the reversible and the irreversible case as Part 1 defines. Each patch is recorded in the vendored crate's `PATCH-PROVENANCE.md` with the patched files' hashes, and `scripts/pin_and_size_check.py` gains `openjph-core`'s patch list beside `RITK_PATCHED_HASHES`, so the gate still refuses any unrecorded byte. `scripts/corpus_synth.py` writes the four codestream rows and records the encoder versions it already records.
6. **preview.rs.** Accept the colour codes in `photometric` and `frame_parts`, build `SampleLayout` from the header, take the decoder's two reports in `decode_encapsulated` instead of refusing, and add a colour arm to `Recipe` carrying the resolved transform, the display scale and the ICC flag. `value` is `colour-not-applicable` for that arm. `select_window` builds a `ColourWindow` from the request's centre, width and invert on that arm, starts from `ColourWindow::initial()`, and refuses a function other than LINEAR, a preset and the file's windows. Replace `NotMonochrome` with `ColourUnavailable` and add `ColourNotApplicable`. `inspect_json` writes `colour` and `icc`.
7. **held.rs and serve.rs.** `grey_bytes` takes the channel count. `Grey` carries it and the header writer emits `X-Ocelli-Channels`. `workspace_bytes` counts the `Rgb` scratch. `charged_bytes` counts the palette.
8. **value.rs and measure.rs.** No change to `stored_at`. `region_statistics` gains a colour arm in the same walk, reading the colour stage's output for each contained pixel through the same band routine and pushing three vectors, then three `roi_statistics` calls. `push_stats` writes the per-channel shape.
9. **page.html.** One predicate for "this frame draws" beside `info.monochrome`, used at the listed lines. `fillGreyImage`, `prepareDisplay`, `paintGrey` and `paintThumbBitmap` read the channel count. W/L drag and invert send their headers on colour from width 256 and centre 128. `cacheKey` writes the colour token. The preset note, the readout, the ICC sentence, the statistics panel and the fault sentence change. `copyWindow` sends by kind.
10. **tools/oracle.** `produce_current.rs` calls `palette_color_lut` and `decoded_output_len`, gains the colour window for AC20's two rows, and `render-params.json` gains those rows.

Rejected: calling `CpuRaster::raster_color_into` with an identity view, as the oracle does, because it carries a view, an index map, a device extent and a generation for a reply that is never resampled, and its scratch is a whole frame. Writing the byte rule in the folder viewer, which is a second copy at the boundary HLD 27.3 names. Converting or windowing in page JavaScript, which has no fixture runner and puts colour arithmetic outside the library. A colour window with its own formula, which would be a second copy of PS3.3 C.11.2.1.2's LINEAR. Applying the window inside `ColorTransform`, which would make stage 4 depend on view state. cornerstone's red-only, data-dependent palette test, rejected under item 11. Saturating colour above 8 bits, which turns a 12-bit image nearly white.

## Dependencies, write ownership and resources

Declared dependencies F-030, F-209 and F-216, all done. F-212 now depends on this story, by the operator's order of 2026-11-07, and F-213, F-214 and F-215 follow F-212 as its plan records. F-240, filed on 2026-11-07, depends on this story and applies matrix ICC profiles where this story only names them.

The write set is split into two parts with disjoint files, by the lead's order of 2026-11-07. Each part is one worker branch and one integration, under the one F-ID.

**Codec part, about 4.5w.** `crates/ocelli-codec/src/registry.rs`, `native.rs`, `jpeg2000.rs`, `htj2k.rs` and `crates/ocelli-codec/tests/registry.rs`, `tests/jpeg2000.rs` and `tests/htj2k.rs`, `vendor/openjph-core-0.1.0` and `vendor/ritk-codecs-0.6.0` with their `PATCH-PROVENANCE.md`, `scripts/pin_and_size_check.py` and its tests, `scripts/unsafe_allowlist_check.py`'s recorded external counts if a patch changes them, `scripts/corpus_synth.py`, `corpus/manifest.tsv`, `crates/ocelli-dicom/tests/corpus.rs`, `docs/lld/codecs.md` and `docs/lld/corpus.md`. It writes all five corpus rows, including `synthetic/sc_rgb_icc.dcm` for the viewer part's AC26, so the corpus has one writer. Its rows are AC10, AC23, AC24 and AC25, the codec half of AC5, and AC29 for its own tip. Order inside it: `decoded_output_len` first, because it is small and the viewer part calls it, then the HTJ2K patch, then `ritk-codecs`, then the corpus rows. `decoded_output_len` defaults to `desc.output_len()`, so the existing callers in `preview.rs:708` and `:1282` and `produce.rs:723` keep working unchanged, and moving them is the viewer part's.

**Viewer part, about 7.5w.** `crates/ocelli-pixel/src/color.rs`, `lib.rs`, `tests/color.rs` and `tests/palette.rs`, `crates/ocelli-render/src/cpu.rs` and `lib.rs`, `crates/ocelli-dicom/src/frame_open.rs` and `lib.rs`, `examples/folder-viewer/src/preview.rs`, `serve.rs`, `held.rs`, `measure.rs`, `page.html`, `use.html`, `examples/folder-viewer/tests/preview.rs`, `tests/value.rs`, `tests/measure.rs`, a new `tests/browser/colour.mjs`, `tools/oracle/src/produce.rs`, `produce_current.rs` and `render-params.json`, `tools/bench/subjects.json` and a new `tools/bench/src/runners/viewer_colour_frame.mjs`, `README.md`, and the other LLD files below. Its rows are AC1 to AC4, AC6 to AC9, AC11 to AC22, AC26 to AC28, AC30, AC31 and the viewer half of AC5. `docs/hld/DEVIATIONS.md` is written by the change approving this plan and by neither part.

**Which starts first.** The codec part can start now. Its files are shared with no other open story except F-229, which writes `registry.rs` and `corpus/manifest.tsv` for JPEG XL, so whichever integrates second merges those two files three ways. The viewer part's library half, `ocelli-pixel`, `ocelli-render` and `ocelli-dicom/src/frame_open.rs`, can start at the same time and run in parallel with the codec part, because no file is shared. `frame_open.rs` is shared with F-225. The viewer part's folder viewer half writes `preview.rs`, `serve.rs`, `held.rs` and `page.html`, which F-224, F-225, F-226, F-227 and F-228 also write and whose worker worktrees exist today, and which F-212 to F-215 write after this story. It cannot have concurrent writers, so it starts after those five integrate or rebases onto them with a three-way merge at integration. AC5's native 4:2:2 reply, AC10's decoded colour JPEG 2000 and the colour corpus files in AC16 need the codec part integrated first, so the viewer part integrates second. Everything else in the viewer part is testable without it, with the colour JPEG 2000 cases asserting today's refusal until the codec part lands and then the drawn image. No GPU is needed except AC20's reference half, which uses the oracle gate's existing headless Chromium on SwiftShader.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-folder-viewer`, `bin/ocelli.sh check ocelli-pixel`, `bin/ocelli.sh check ocelli-render`, `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh check ocelli-codec` while iterating, then the feature profile once at the integration tree. The sprint profile runs once for the sprint, per the operator's rule of 2026-11-06. AC16 and AC28 run under the F-208 browser harness. AC15's resident measurement names the host and the frame used. AC20 runs the oracle's reference half and records its comparison in `docs/lld/oracle.md`. AC25's lossy comparison names the OpenJPEG version `scripts/corpus_synth.py` records. AC27 is read from the sprint corpus gate. AC29 runs `bin/ocelli.sh gate wasm` on base and tip. AC30 runs `bin/ocelli.sh bench viewer.open viewer.first_image viewer.window_step viewer.scroll_rate viewer.resident_memory` on base and tip, AC31 runs `bin/ocelli.sh bench viewer.colour_frame` on the tip, and AC32 runs `bin/ocelli.sh bench --compare-cornerstone --pseudo <dir>` once F-223 has integrated, all on the F-216 host with the release build. A later edit to any file in the write set invalidates the record. The oracle's colour rows stay not scored, so no scored comparison is claimed.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The folder viewer's local HTTP reply carries display bytes, as it does today for grey.
- Render-loop allocation: n/a. The colour scratch is a workspace buffer reused across requests and counted in `workspace_bytes`.
- unsafe: none in the workspace. D-31's vendored patches touch external code whose unsafe sites are counted by the external audit, and the count is re-measured and recorded. The patches add no unsafe of their own.
- Tier A (WebGPU): n/a for the folder viewer, whose page opens no device, as F-210 and F-212 record. In the library, `ColourWindow` and `DisplayScale` have no GPU form in this story and no GPU entry point accepts a colour window or a scaled colour sample, so nothing can draw a colour window differently on tier A. The GPU colour path, `ColourLut` in `crates/ocelli-render/src/colour.rs`, is unchanged in code, and its palette bake reads the entries `PaletteColorLut` scales, so a palette bakes the same bytes tier C draws, under the one palette rule, asserted by `the_gpu_bake_holds_sixteen_bit_entries_after_their_display_scale`.
- Tier B (WebGL2): n/a, for the same reasons.
- Tier C (CPU): full for the drawn codes, through `ocelli-pixel`'s `ColorTransform`, `DisplayScale` and `ColourWindow` and `ocelli-render`'s CPU byte rule, on the host process whatever GPU the machine has. Every shape that rule refuses is refused by name, never drawn differently. The JPEG 2000 and HTJ2K decoding is tier independent and runs on every target the codec crate builds for.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Interleaved and planar RGB, PS3.3 C.7.6.3.1.3 | `examples/folder-viewer/tests/preview.rs`, AC1, AC2 |
| fixture | YBR_FULL inverse and the byte rule, PS3.3 C.7.6.3.1.2 | same, AC3, AC4 |
| fixture | 4:2:2 replication and native length, PS3.3 C.7.6.3.1.2 | same and `crates/ocelli-codec/tests/registry.rs`, AC5, AC6, AC23 |
| fixture | Palette first mapped input and clamp, PS3.3 C.7.6.3.1.5 and C.7.9 | same, AC7 |
| fixture | Decoder report honoured once, PS3.5 8.2 and C.7.6.3.1.3 | same, AC8, AC9 |
| fixture | Display scaling of samples and palette entries, PS3.3 C.7.6.3.1 Bits Stored and C.7.6.3.1.5 | `crates/ocelli-pixel/tests/color.rs`, `tests/palette.rs`, AC11, AC22 |
| fixture | Colour window and invert, PS3.3 C.11.2.1.2 LINEAR and C.11.6 INVERSE over `0 ..= 255`, following cornerstone3D 5.8.2 | `crates/ocelli-pixel/tests/color.rs`, AC19, AC21 |
| fixture | Inverse RCT and ICT, ISO/IEC 15444-1 G.2 and G.3, PS3.5 8.2.4 | `crates/ocelli-codec/tests/jpeg2000.rs`, `tests/htj2k.rs`, AC24, AC25 |
| fixture | Per-channel region statistics | `tests/measure.rs`, AC13 |
| unit | `colour_bytes_into` agrees with `raster_color_into` at an identity grid and the initial window | `crates/ocelli-render/src/cpu.rs` tests |
| unit | `palette_color_lut` refusals and `has_icc_profile` | `crates/ocelli-dicom` tests |
| integration | Refusals by name and preserved state | `tests/preview.rs`, `tests/value.rs`, `tests/measure.rs`, AC10 to AC14, AC26 |
| integration | Memory charge | `serve.rs` tests, AC15 |
| corpus | The four colour JPEG 2000 rows and the ICC row | `crates/ocelli-dicom/tests/corpus.rs`, AC24 to AC26 |
| oracle | The colour window rule against cornerstone3D 5.8.2, recorded and not scored | `bin/ocelli.sh oracle` reference half, AC20, and the sprint corpus gate, AC27 |
| browser | The page draws colour, windows and inverts it, gates the scalar value and links by kind | `tests/browser/colour.mjs`, AC16, AC28 |

The fixtures are hand-computed from the matrices, transforms and rules quoted above, not read back from the implementation.

## cornerstone3D 5.8.2 is the floor

By the operator's clarification of 2026-11-07, relayed by the lead: every behaviour cornerstone3D 5.8.2 has for these images, Ocelli has too, verified against the oracle where it can render them, or this plan names the gap and why. Where Ocelli departs, the departure must be better on a stated and measured axis, never only different, and each departure is recorded as a measured divergence under D14. Where `tools/bench` or the oracle can time cornerstone on the same frame, both numbers are recorded and the target is that Ocelli is not slower.

| Behaviour | cornerstone3D 5.8.2 | Ocelli after F-239 | Axis and measurement |
|-----------|---------------------|--------------------|----------------------|
| RGB, interleaved and planar | Drawn | Drawn | Same. AC1, AC2, AC20 |
| YBR_FULL | Drawn, `convertYBRFullByPixel.js:16` to `:31`, textbook coefficients `1.402`, `0.34414`, `0.71414`, `1.772` | Drawn with the exact inverse of PS3.3 C.7.6.3.1.2's stated forward matrix | Better, closer to the standard. The two inverses differ by at most 0.020028 per channel (`crates/ocelli-pixel/tests/color.rs:20` to `:24`). AC20 records the per-pixel byte difference |
| YBR_FULL_422, native and JPEG | Drawn | Drawn | Same. AC5, AC9, AC20 |
| YBR_PARTIAL_422 | Throws "No color space conversion" (`convertColorSpace.js:38`) | Drawn with the studio offset | Better, a frame PS3.3 defines is drawn. AC6 |
| YBR_ICT and YBR_RCT | Copied as RGB on the decoder's word (`convertColorSpace.js:23` to `:25`) | Decoded with Annex G's inverse inside the decoder | Same outcome. AC24, AC25, AC20 on the four new rows |
| YBR_PARTIAL_420 | Copied as RGB | Refused by name | **Gap, named.** PS3.5 uses it with the MPEG and H.264 video syntaxes, and the codec registry knows none of them, so no file this build can decode carries it. The gap closes with a video decoder story |
| PALETTE COLOR, 8 and 16 bits, a channel declared above 8 bits holding an entry above 255 | Drawn, 16-bit entries shifted right by 8 when a red entry exceeds 255 | Drawn, each channel shifted by its own declared depth minus 8 | Better, closer to PS3.3 C.7.6.3.1.5. AC22 measures the case where the two differ |
| PALETTE COLOR, every channel declared 8 bits | Drawn, entries as they are | Drawn, entries as they are, not flagged | Same. `an_all_8_bit_palette_does_not_report_the_fallback` |
| PALETTE COLOR, at least one channel declared above 8 bits and every entry of every such channel at most 255 | Drawn, entries as they are | Drawn, the whole palette read as 8-bit by ruling R1, and flagged | Same. AC22's fallback rows and both corpus palette rows, 0 bytes differ from base |
| Colour above 8 bits | Saturates at 255 | Shifted by Bits Stored | Better, closer to PS3.3's Bits Stored, and a 12-bit image is not drawn nearly white. AC11 |
| Window and invert per channel | Yes, LINEAR, initial 256 and 128 | Yes, the same rule | Same. AC19, AC20 |
| Window from rounded or unrounded samples | Windows bytes its converters already rounded | Windows the `f32` stage 4 output and rounds once | Better, one rounding, HLD 27.3. AC20 counts the pixels that differ by one code |
| Rounding a tie | Ties to even, `Uint8ClampedArray` | Ties to even, tier C's `byte_from_channel` | Same, by the operator's decision of 2026-11-07. AC21 asserts both tie directions. Monochrome keeps its table address rounding by ruling R2 and its bytes are measured unchanged |
| Value at the pointer | Probe shows the components | R, G and B | Same. AC13 |
| Region statistics | Per channel, `BasicStatsCalculator.js:25` to `:54` | Per channel, one walk | Same, on unrounded values. AC13 |
| ICC profile | Ignored silently | Named on the page | Better, the omission is visible. AC26. F-240 applies it |
| Initial JPEG baseline decoder RGB | Cornerstone's decoder output | The existing native decoder's reported RGB, with no second inverse | **Gap, measured.** On the corpus JPEG baseline row, 3857 of 6144 source-centre pixels differ, maximum two codes. The initial colour transform is identity on those already decoded RGB bytes. This is a bounded observation of the existing decoders, not a new tolerance or a claim about every JPEG file |
| Explicit colour-window display quantisation | The reference's rendered window | Existing LINEAR arithmetic with one final byte rounding | Better on the measured hand fixture. Both explicit window cases match all 720 hand-derived source-centre channels in Ocelli. The reference differs on ten channels by one code, five below and five above. Default unwindowed source-centre bytes agree |
| Oracle canvas quad coverage | Fractional boundary pixels in the reference canvas | The current native CpuRaster projection used only by the observation producer | **Gap, named.** Initial synthetic RGB and palette canvases differ only at fractional edges, maximum 64 codes. Window cases have 853 edge pixels above one code. This pre-existing producer geometry is recorded separately from the exact source-centre colour arithmetic and is not claimed as canvas parity |
| Speed and memory | Measured by F-223's harness | Measured on the same frames | Target not slower. AC32 |

## Parity surface covered

None measured as a scored row. `docs/hld/B-parity-surface.md` has no colour row, and the oracle's colour candidates stay not scored because HLD 25.1 states no colour threshold. The behaviours, and every departure from cornerstone3D 5.8.2 with the axis on which it is better or the reason it is a named gap, are the table in "cornerstone3D 5.8.2 is the floor" above. Each departure is a measured divergence under D14, observed in AC11, AC20, AC21 and AC22, never a claim of bit-exact agreement.

## Deviations

Three rows, written into `docs/hld/DEVIATIONS.md` with their reasons in the change that approved this plan, per `.claude/commands/design.md` step 3. The register is authoritative and this list names them.

- **D-29**, colour window and invert. A colour frame is windowed and inverted per channel after stage 4 through `ColourWindow`, the existing LINEAR `VoiTransform` and INVERSE `PresentationTransform` over `0 ..= 255`, initial width 256 and centre 128, following cornerstone3D 5.8.2. §18 orders those stages before stage 4 and D-23 keeps colour out of them. PS3.3 C.11.2 defines no colour window, so this is a measured parity choice.
- **D-30**, a frame carrying ICC Profile `(0028,2000)` is drawn without the profile and the page says so, against §18's stage 4 and §35's "correctness requirement". F-240 narrows it to the profiles F-240 does not apply.
- **D-31**, `openjph-core` and `ritk-codecs` carry recorded source patches so the inverse component transform runs before the level shift and its clamp, and so `ritk-codecs` reads a three-component codestream, against the byte-identical vendoring D-22 records and the manifest-only patch D-21's adapter relies on.

## LLD impact

- `docs/lld/pixel-pipeline.md`: stage 4 has a production consumer, the folder viewer. `DisplayScale` and `ColourWindow`, their place after stage 4, and D-29.
- `docs/lld/render-graph.md`: `colour_bytes_into`, the shared byte step and the window between map and bytes.
- `docs/lld/codecs.md`: `decoded_output_len`, native 4:2:2 length, three-component JPEG 2000 and HTJ2K, Annex G's order and D-31's patches.
- `docs/lld/frame-open.md`: `palette_color_lut` and `has_icc_profile`.
- `docs/lld/oracle.md`: the one palette reader, AC20's recorded colour window comparison and AC27's outcomes.
- `docs/lld/corpus.md`: the five new rows.
- `docs/lld/build-targets.md`: AC29's wasm deltas and the `cargo tree` reachability.
- `docs/lld/benchmarks.md`: AC30's before and after medians and AC31's colour figures.

## User-facing documentation impact

- `README.md:309` to `:314`: the sentence "A colour frame ... stays on a fixed error sentence" is replaced. Colour frames draw. Window drag and invert act on each channel from width 256 and centre 128, as cornerstone3D does. A colour image has no presets, the pointer readout shows R, G and B, and a region on it reports R, G and B statistics. A file with an ICC profile is drawn with a sentence saying the profile was not applied. A colour image this build cannot draw shows a sentence naming why. Step numbers are measured at implementation.
- `examples/folder-viewer/src/use.html:638`: the stage 4 paragraph says the folder viewer draws colour through `ColorTransform`, `DisplayScale`, `ColourWindow` and `colour_bytes_into`, that the colour window follows cornerstone3D 5.8.2 and not PS3.3, and that ICC is not applied until F-240. The crate table at `use.html:215` names the new `ocelli-pixel`, `ocelli-render`, `ocelli-dicom` and `ocelli-codec` functions.
- `examples/folder-viewer/src/more.html`: no change. It has no colour card, and ICC remains library behaviour this page does not draw, which `use.html:638` already states.

## Decided by the operator on 2026-11-07

Each answer is the operator's unless it says lead. The original options are kept in the history of this file at 5451cb7d.

1. **Native 4:2:2 length.** Option A. Each decoder reports its own output length through a registry query, so native `YBR_FULL_422` is two samples per pixel and JPEG stays three. Applied in item 15, AC5 and AC23.
2. **YBR_ICT and YBR_RCT.** Option B, widened. F-239 decodes `YBR_ICT` and `YBR_RCT`, multi-component JPEG 2000 and HTJ2K, in `ocelli-codec`, `jpeg2000.rs` and `htj2k.rs`. Measured for the answer: neither vendored dependency can do it unchanged, which raises question 9. Applied in item 16, AC10, AC24, AC25 and the five corpus rows. The estimate rose to 12w with the floor work below.
3. **Window on a colour frame.** As cornerstone3D 5.8.2 does: one LINEAR window applied to each of R, G and B, initial width 256 and centre 128. The arithmetic is put once in `ocelli-pixel`, as the existing LINEAR function per channel. This follows the oracle and is not defined by PS3.3 C.11.2 for colour, recorded as a measured parity choice, and is recorded as D-29, because §18 and D-23 say colour does not enter stages 1 to 3. Applied in items 8 to 10, AC12, AC19 to AC21.
4. **Invert on a colour frame.** As cornerstone3D 5.8.2 does, `255 - y` per channel after the window, through the existing INVERSE presentation function over `0 ..= 255`. Same deviation as 3, D-29. Applied in AC19.
5. **ICC Profile (0028,2000).** Shown, with a page sentence that the profile was not applied, until a new story applies it. That story is F-240, filed the same day. Recorded as D-30. Applied in item 13 and AC26.
6. **Measure on a colour region.** Area plus mean, SD, minimum and maximum per channel R, G and B, through F-211's single region walk. Applied in item 12 and AC13.
7. **The oracle's palette reader** (lead). Option A, replaced by the new `ocelli-dicom` reader so one exists. Applied in AC27.
8. **Colour above 8 bits and palette entries above 255.** One library rule in `ocelli-pixel` scales both to the display's 8 bits. The rule is item 11: shift right by `n - 8` for `9 <= n <= 16`, where `n` is Bits Stored for a sample and the descriptor's third value for a palette entry, decided from the header. It matches cornerstone's `>> 8` for a 16-bit palette (`convertPALETTECOLOR.js:2` to `:9`). It departs from cornerstone where cornerstone is not defensible against PS3.3 C.7.6.3.1.5: that section makes the descriptor's third value the bit depth of every entry, so a palette whose descriptor says 16 is scaled as 16 even when its red entries happen to be small, and the green and blue tables are scaled by their own depths rather than by the red data. Ruling R1 below refines this: a palette is read as 8-bit only when at least one channel is declared above 8 bits and every entry of every such channel fits in 8 bits, otherwise each channel is shifted by its own declared depth minus 8. For colour samples, PS3.3 gives Bits Stored as the range of the sample, so a 12-bit sample is scaled by 12, where cornerstone saturates it. The truncating shift is kept because it is cornerstone's and gives every output code an equal share of the input range. AC17 makes the reviewer check C.7.6.3.1.5's text. Applied in AC11 and AC22.

9. **How the JPEG 2000 colour decode is built**, raised by the answer to question 2. Patch the vendored crates with recorded provenance and a deviation row, HTJ2K first: call `openjph-core`'s existing `rct_backward` and `ict_backward` on the decode path and move the level shift and clamp after them. Then teach `ritk-codecs` multi-component decoding for Part 1. Recorded as D-31. Applied in item 16, Approach step 5, AC10, AC24, AC25 and the codec part of the write set.

Also given on 2026-11-07, relayed by the lead and applied:

- **Size and speed.** The features must not grow the wasm build beyond budget or make the system slower. Applied in AC29, AC30 and AC31.
- **cornerstone3D 5.8.2 is the floor and Ocelli should be better.** Every cornerstone behaviour for these images is matched or named as a gap with its reason, every departure is better on a measured axis and recorded under D14, and where cornerstone can be timed on the same frame both numbers are recorded with the target that Ocelli is not slower. Applied in "cornerstone3D 5.8.2 is the floor", item 4's R, G and B readout, AC13, AC20 and AC32. The table names two gaps, `YBR_PARTIAL_420` and tie rounding, with reasons.
- **F-243 lands first.** The operator scheduled F-243, the JPEG-LS run-mode patch to the vendored `ritk-codecs` scan decoder, next on 2026-11-07, and this story now depends on it. F-243 patches only `src/jpeg_ls/*` and records them in `PATCH-PROVENANCE.md` and in `RITK_PATCHED_HASHES` in the same shape `work/f-239-codec` uses, with `RITK_MANIFEST_PATCHED_HASHES` for the manifests. The codec branch merges onto it as a union: both provenance sections, both sets of digests, one recomputed `RITK_PROVENANCE_SHA256`, and both sets of corpus rows. The source-file refusal text becomes "has undeclared changes beyond its recorded patch" rather than naming D-31.

## Open questions

None. Questions 1 to 9 are answered above. AC17's reviewer check of the cited standards and AC20's reference observation are evidence rows with an owner and a stage, not open decisions.

### Decided by the operator on 2026-11-07, tie rounding

Ruling R2 below refines this record for monochrome. The operator chose to fix the tie rounding inside F-239. The shared tier C byte rule changes from rounding a tie up to rounding half to even, as cornerstone3D 5.8.2 does, for monochrome and colour alike, so the arithmetic stays in one place.

- **Library half owns it.** The rule in `crates/ocelli-render/src/cpu.rs` and every caller of it, with hand-computed fixtures at exact ties for both parities and a control just either side of a tie.
- **Monochrome moves too, and that is measured.** Every existing test that pinned a tie byte is updated from the rule, not from the new output, and the number of changed bytes over the corpus's monochrome rows is measured and recorded. The oracle's monochrome verdict on the changed rows is a sprint verification observation.
- **Tiers A and B are checked.** The WGSL path's conversion to bytes is compared with the new rule at ties on a real adapter where one is available, and the result is recorded. A disagreement is reported, not hidden.
- **The floor table** row for tie rounding moves from a named gap to matching cornerstone.

### Rulings after the library half's review pass 1

Made on 2026-11-07 after `.claude/reviews/F-239-lib-working-pass-1.md`.

- **R1, decided by the operator: the standard, with a fallback.** Decided once for the whole palette by the lead's later ruling, a palette is read as 8-bit only when at least one channel is declared above 8 bits and every entry of every such channel fits in 8 bits, otherwise each channel is shifted by its own declared depth minus 8. The 8-bit reading is cornerstone3D 5.8.2's, and the frame carries a flag the page states. An all-8-bit palette is not flagged, and a conforming palette is unaffected. AC22 gains rows for that palette and AC27's two corpus palette rows are expected to draw as at base and as the reference does.
- **R2, lead: monochrome keeps its rounding.** Tier C monochrome hands whole table bytes to the byte rule, and the table address rounds a tie up through `display_index`, which tiers A and B share and which matches the reference's `Math.floor(x * 255 + 0.5)` table quantisation. It does not change. The tie section above is corrected: the half-to-even rule governs colour bytes, and monochrome bytes are measured unchanged, 0 of 102 views.
- **R3, lead: the unorm8 store is recorded, not changed.** No colour route reaches the `present_display` mode 0 path. The 128 of 255 ties it rounds up are recorded as measured in the LLD.
- **R4, lead: stale text is corrected in this change.** D-29, item 10, AC21's expected bytes (128 and 126), the floor table's tie row and the Boundary section's statement that `ColourLut` is unchanged are brought to what the code and the rulings say.

### Lead ruling after the codec part's review pass 2

Made by the sprint lead on 2026-11-07 after `.claude/reviews/F-239-codec-working-pass-2.md`, which found openjph-core's inverse ICT uses its own derived constants, so green at the AC25 point is `-8.433824` where ISO/IEC 15444-1 Annex G equation G-6's literal coefficients give `-8.43354`.

- **The inverse ICT uses G-6's literal coefficients,** 1.402, 0.34413, 0.71414 and 1.772, in both vendored decoders. OpenJPEG, which cornerstone3D 5.8.2 decodes with, uses the same literals, so the floor and the standard agree. The openjph patch changes those constants and D-31's provenance records it, and the unit tests assert AC25's hand values within 1e-4 and go red under any one coefficient changed by more than that.
- **The OpenJPEG comparison is a bound, not a pin,** and every sentence that says otherwise is corrected.

### Lead ruling after the codec part's review pass 3

Pass 3 found two defects in prose and no wrong pixel.

- **D1.** The `pins.openjph-inventory` note in `scripts/guards/catalogue.py` drops its file count. The gate's refusal already names the files, so a count beside it only goes stale.
- **D2.** `colour.rs` and the openjph `PATCH-PROVENANCE.md` either state the measured gap between the G-6 literals and the derived coefficients, measured again by the implementer, or give no number. "The same values" goes. Both openjph hashes are pinned again after the edit.
- **Nitpicks taken.** The provenance's purpose clause covers `colour.rs`. The D-31 row names the coefficient change in one clause. The test is renamed so its name says the G-6 literals, and this plan's Tests row follows the new name.
- **Not taken.** The f32 half-rounding difference from OpenJPEG was not observed and is left as a note.
- **Outside this story.** Pass 3 measured ritk's J2K lossy 9/7 path far from OpenJPEG on colour `.91` data. That code predates F-239, and it becomes its own backlog story at sprint close.

### Lead ruling after the codec part's review pass 4

- **S1, the vendored suites run in no gate.** The floor gains one step that runs the library tests of both vendored crates, `cargo test --offline --locked --manifest-path vendor/<crate>/Cargo.toml --lib`, with `CARGO_TARGET_DIR` outside `vendor/`, and `.github/workflows/ci.yml` runs it. The step leaves each vendored `Cargo.lock` byte-identical and the pins gate green. A guard probe applies a 1e-2 change to one inverse ICT coefficient in a scratch copy and requires the step red. If CI cannot fetch a vendored crate's dependencies offline, the implementer reports that and how CI already supplies the workspace's, rather than dropping `--offline`.
- **N1 taken.** The catalogue note says the files its refusal names.
- **Outside this story.** F-243's ritk patch gains the same protection through this step once both land, and the integration order already merges the codec branch onto F-243.

### Lead ruling after the codec part's review pass 5

- **S1.** Both lint-policy probe blocks are copied from `sprint/s46` byte for byte by the lead, so this branch carries no wording of its own for them.
- **N1 taken,** by the lead: the comment credits the 300 seconds to `Sandbox.run`'s default.
- **Outside this story.** In a clean clone `cargo test --workspace` fails two `ocelli-oracle` tests that read the untracked `tools/oracle/out/run.json`, present on `sprint/s46` too since F-X021. It is recorded for the lead to fix before the sprint push, because CI's rust job runs that command.


### Viewer integration wiring, 2026-11-08

The colour browser probe is invoked by the existing `viewer-browser` gate in
`bin/ocelli.sh`, and its ordinary page globals are declared in
`eslint.config.js`. No lint rule or gate is disabled. The page's window-state
selection is also used for mixed-kind tiles and prefetches, so a CT window is
not sent to a colour image in the same series. A four-cell browser case checks
both directions of AC28 with window linking enabled.


### Viewer review ownership ruling, 2026-11-08

Pass 1 measured 1,966,050 bytes of colour scratch after a large planar band
and the same retained capacity after a small RGB frame. The ownership table's
claim that the over-twice rule frees colour bands was false. That rule applies
to the grey reply only. Colour Stored, RGB and packed vectors remain reusable
conversion workspace storage, bounded by the whole-row 65,536-pixel band, at most 1.875 MiB
per workspace for a 16-bit planar frame. A 32-bit container with supported
stored bits uses up to 2.25 MiB per workspace. Their actual capacities are
charged in workspace bytes. No trimming optimization or memory budget change is made.


### Protocol fixture ruling after GLM test review, 2026-11-08

The AC13 YBR RGB array is checked numerically against the hand values
46.72366, 119.56544 and 138.98863, using the existing 0.001 bound of
ocelli-pixel/tests/color.rs. This is the already approved colour-stage fixture
rule described above, not a new renderer or scored oracle threshold. Each of
the three channels must be finite. The one-decimal readout text remains exact.
Observed shortest-f32 JSON spellings are not expected-value evidence and are
removed from the test. The numeric-channel mutation with unchanged text must
fail. This repair is reviewed independently before test acceptance.

### AC20 observation instrument amendment, 2026-11-08

The resumed implementation found the two declared colour-window observations
and candidate window binding still missing. The lead supplies the two cases as
`colourObservations` rows in the existing render-params declaration, selected
only by `--colour-observation <id>` into a separate partial output directory.
The default reference parameters and scored census are unchanged. Each row
names the existing synthetic RGB file, centre 128, width 100 and invert off
or on. The reference page records and applies the explicit LINEAR window and
invert together. The candidate uses `colour_bytes_into` for stage 4, sample
scale and the colour window, then the existing native RGB raster for geometric
projection of those bytes. No new pixel formula or public renderer API is
added. All colour observations remain unscored. This measured source-byte
projection uses the declared NEAREST interpolation. Differences are recorded
per row and investigated against the floor table, without a new threshold.

The full AC20 reference run initially refused metadata coverage because no
hand-written expectation asserted YBR_RCT or YBR_ICT. The four declared
64 by 96, three-component, unsigned 8-bit colour codestream fixtures now
carry those literal assertions in metadata-truth.json. They follow the
approved PS3.5 8.2.4 names and input specification, not observed output.
The installed dicom-image-loader 5.8.2 package was compared with its official
registry tarball after SHA-512 verification against the lockfile. All 248
regular files match, including createImage.js. Its debug block is therefore
present in the published pinned artifact, correcting the earlier expectation
that it was a local modification. No package source was changed.

### Lead clarification, the shared regression instrument, 2026-11-08

AC30 uses the same corrected permanent instrument on both products. The
original `5451cb7d` source inherits a first-image runner that waits for
the element renamed by F-208.
The F-225 one-line selector repair must therefore also instrument the base.
The current shared helper includes the already approved F-227 informational
PET workload and pool accounting. Both sides use its exact same bytes.
The five existing monochrome subjects keep their answered F-216 thresholds.
No baseline file, tolerance or product source is changed on the original.

The prepared base is original source tree `dd269895` plus only
`tools/bench/src/viewer_work.mjs`, giving staged tree `68702951`.
Independent byte comparison regenerated the registered first CT and DX
files with their real subject UID and instance arguments and proved their
bytes unchanged: CT 524822 bytes, SHA256
`01cb3563cdd9a0a41e1e9ec6a1eb859643c4ac8926d7d7c276e6a235874ff582`,
and DX 17400538 bytes, SHA256
`a94585cdd1cbaf1aaff6603b93a748790e13c8a60531710e01e32a790e2925e7`.
The run records the full helper hash, binary hash, staged source, actual
browser and host on each side. Any later helper edit is copied and checked
on both sides before measurement. This is an instrument repair, not an
isolated causal claim for every story in the combined source stack.
The factual size attribution belongs in `docs/lld/build-targets.md`.

### Lead ruling, reuse palette acceptance across a draw, 2026-11-08

The actual release AC31 observation at source `0c327dd7` records a
19023.172166 ms median over the nine 8192-square palette window steps.
GLM's subsequent source investigation found that the folder draw calls
RasterColor::supported inside each conversion band. At that width the band
is eight rows, so the same immutable palette is checked 1024 times per draw.
The library's AcceptedColor already carries the validated borrow, but this
caller discards it after each band. This is measured investigation followed
by a source-level repeated-work finding, not a new numerical rule.

The lead approves validating once per draw and reusing that same descriptor
for every band. RasterColor becomes Copy and Clone because it contains only
an immutable borrowed acceptance descriptor or the unsupported discriminant.
No transform mutation, cross-frame cache, new allocation or pixel arithmetic
is introduced. Unsupported transforms still refuse before conversion.
The exact existing colour, palette and ICC fixtures remain authoritative.
Run affected checks and the same release benchmark after the repair, preserve
the old observations and report actual changes without a new tolerance or
baseline acceptance. Controlled cost comparisons remain separate evidence.

### Lead implementation amendment, exact final byte store, 2026-11-08

The largest-frame benchmark records a 4699.183 ms RGB8 window median after
the palette validation hoist. A separate GLM source investigation identified
the final channel store's binary search as a candidate for measurement, not
an established dominant cost. Its illustrative IEEE examples contained two
errors, retained in the original report and corrected during review. They
do not supply fixture expectations.

This bounded implementation keeps ruling R2 unchanged. The pinned Rust
1.97.1 standard library's `round_ties_even` returns the nearest integer with
halfway values rounded to even. The original non-finite and endpoint guards
remain in their original order. For the interior positive value below 255,
the rounded value is an exact integer in 0 through 255. The existing shared
`exact_f32_integer` reader and checked `u8::try_from` recover that integer.
There is no new float-to-integer cast, unsafe code, allocation or dependency.
The exact-byte predicate, monochrome table addressing and table construction
in `colour.rs` are unchanged.

Existing independent fixtures cover all 255 exact halves and their adjacent
floats through the public colour entry. Additional literal IEEE controls
retain the non-finite policy, especially positive infinity to zero, both
signed zeros, subnormals and finite extremes. A dropped or reordered
non-finite guard must fail the positive-infinity control. The implementation
requires independent source review, native and wasm checks, and an unchanged
registered largest-frame instrument before any cost conclusion. No threshold
or tolerance changes.

### Lead implementation ruling, AC27, 2026-11-08

The oracle candidate projects only the shared palette provider's eleven input
attributes into `MetadataSet`, then calls `palette_color_lut` directly. This
projection copies metadata values and performs no LUT interpretation. Projecting
the complete DICOM object would copy native Pixel Data and refuse encapsulated
fragments, neither of which is required to read a palette. The existing
decoder-reported RGB bypass, sidecar-driven non-palette eligibility and coarse
`palette-refused` output remain the candidate's policy.

The private LUT reader and its now-unused parsing helpers are removed. Two
native producer controls exercise the approved R1 fallback, true 16-bit words,
first-mapped clamping, missing channels, illegal depth, segmented element
presence and non-palette bypass. Removing only the segmented presence tags from
the candidate projection makes the policy control fail. The two existing
synthetic corpus rows still require the actual before and after candidate byte
identity record that AC27 names. Unit tests do not replace that observation.

### Lead correction, nearest inverse coefficients, 2026-11-08

AC17 at-source review found three legacy inverse literals rounded through
short decimal summaries, despite the stated single nearest-f32 policy.
Independent exact rational inversions by root and a second reviewer agree.
Change only FULL red-Cb to nearest bits b81a6f2a, FULL blue-Cr to b90d1ef6,
and PARTIAL red-Cb to b8c74e0f. The matrices, offsets, sample scaling, order
of operations, final byte rule and existing tolerance stay unchanged.
Clarify that the printed decimal equations are approximations, with the
three exact rational fractions stated beside the literals.

Three public basis inputs isolate these coefficients and assert their
exact output bits. A fourth public colour/window control uses YBR
[0,128,0], LINEAR centre bits3f0468f7 and width1. The independent binary32
calculation predicts corrected blue255, versus oldblue0. These controls
must fail on the old literals and pass on the corrected source. The
existing full/partial fixture suite also runs unchanged.

The observed identity-window NumPy sweep does not establish absence of
effects under other windows. Actual native and wasm checks, candidate
observations and the source-independent review remain required before
completion. This repairs the declared arithmetic rather than changing
a tolerance or substituting the textbook inverse.


### Lead ruling, explicit colour census declarations, 2026-11-09

The first post-citation oracle gate reached the reference identity comparison and failed its coverage census. Eleven newly added synthetic colour views were correctly unmeasured under existing D-16 but absent from the declared inventory. The four compressed YBR views and saturated RGB reference belong to F-239. The six RGB ICC views belong to F-239 and F-240. Every one is absent from the canonical pre-integration manifest and has three components with eight stored bits in the actual reference sidecar. No previously scored view is demoted.

The lead adopts an explicit addition of those eleven identifiers with only `unstated-threshold` to `tools/oracle/compare-expectations.json`, subject to independent review like a tolerance change. All 48 prior entries and qualifiers remain unchanged. This applies the approved unscored-colour scope and existing HLD 25.1/D-16, with no new metric, numerical tolerance, class rule, pixel gold, gate bypass or passed-colour claim. It does not absorb an unexpected scored failure. The exact identifiers, provenance and review are recorded in the existing feature review series. The failed gate remains failed. The corrected tree must pass the actual identity and mutation consumers and a fresh named oracle gate before a passing receipt is recorded. Native candidate parity remains a separate production comparison.
