# F-239, Folder viewer colour frames through the library's colour stage

**Status**: draft
**Epic ref**: E38.37
**Sprint**: S46
**Estimate**: 11w

Story row, `docs/sprints/BACKLOG.md`, retitled with the operator's answers of 2026-11-07: "Folder viewer colour frames: RGB, YBR_FULL, YBR_FULL_422, YBR_ICT, YBR_RCT and PALETTE COLOR with Planar Configuration drawn through ocelli-pixel's ColorTransform on the decoder's reported output space, multi-component JPEG 2000 and HTJ2K decoded with their component transforms, window and invert per channel as cornerstone3D 5.8.2 applies them, region statistics per channel, colour above 8 bits scaled by one library rule, an ICC profile named as not applied, and every colour shape this build cannot draw refused by name". Layer All, depends on F-030, F-209 and F-216, milestone M20. F-240 follows it and applies matrix ICC profiles.

`allocation.json` notes: "Decided by the operator on 2026-11-07: a colour series shows the fault 'this preview shows monochrome frames only' from PreviewError::NotMonochrome today, and the folder viewer previews colour frames through the library's colour stage instead. Scheduled first in the folder viewer chain, ahead of F-212. Questions 1 to 8 answered by the operator on 2026-11-07, widening the story to JPEG 2000 and HTJ2K colour decoding and cornerstone3D's colour window."

The operator answered questions 1 to 8 on 2026-11-07, and the answers are applied below and recorded under "Decided by the operator on 2026-11-07". **The status stays draft for one reason**, question 9, which the widened question 2 raised: neither vendored JPEG 2000 decoder can produce a correct colour frame without a change to its source, and choosing that change is the operator's. The exact unblock condition is under Open questions. Every path and line below was read on `sprint/s46` at 5451cb7d unless it names 636fdcc1.

**The estimate rose from 4w to 11w.** The parts, each the lead's estimate: the original preview wiring 4w, the decoder output length query 0.5w, the colour window and invert in `ocelli-pixel` and the page's tools 1w, per-channel region statistics 0.5w, the display scaling rule 0.5w, the ICC sentence 0.25w, the oracle's palette reader 0.25w, HTJ2K three-component decoding with its inverse transform 1.5w, JPEG 2000 Part 1 three-component decoding 2w, and four synthetic corpus rows 0.5w. Question 9 option C would remove the 3.5w of decoding and leave 7.5w.

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

**The HTJ2K route has a defect that an adapter cannot undo.** `openjph-core` undoes the DC level shift per component and clamps to `0 ..= 2^B - 1` immediately after the inverse wavelet (`vendor/openjph-core-0.1.0/src/codestream/tile_comp.rs:346` to `:367`). Annex G's inverse transform has to run before that shift, and RCT's chroma spans one more bit than the clamp keeps. A worked case at `B = 8`: pure red `255 0 0` level-shifts to `127 -128 -128`, RCT gives `Y0 = floor(-257 / 4) = -65`, `Y1 = 0` and `Y2 = 255`, and `Y2 + 128 = 383` is clamped to 255. An adapter that subtracts the shift again and inverts gets `I1 = -65 - floor((127 + 0) / 4) = -96` and `I0 = 127 - 96 = 31`, so red 159 where the source was 255, with no error. A correct colour HTJ2K decode therefore needs a change inside the vendored crate. The vendored package is byte-identical to the published archive and the pins gate proves it (`scripts/pin_and_size_check.py:122` and onward), so the change is a patch with its own provenance, as `ritk-codecs` already carries for its manifest (`scripts/pin_and_size_check.py:100` to `:116`). Question 9 asks for that decision.

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
4. **Value at the pointer on a colour frame.** D-23 says PS3.3 C.11's chain does not apply to a three-sample frame, and a palette frame's stored value is an index, not a modality value. So stage 1 is not built for either and the modality value reports unavailable. Region statistics are per channel under item 12.
5. **A colour shape the build cannot draw is refused, never approximated.** One new code, `colour-unavailable`, with a sentence that names the reason from a closed list. It replaces `not-monochrome`, which no longer has a meaning.
6. **Scratch is bounded by a band of rows, not by the frame.** `ColorTransform::map_into` takes `&[Stored]` and writes `&mut [Rgb]`, twelve bytes per pixel each way. At `MAX_PIXELS` (`preview.rs:96`) a whole frame would be about 805 MB per buffer. The conversion runs over bands of whole rows with an even row width for 4:2:2, and a planar frame gathers the band's three plane rows into one planar band, which is still one valid `map_into` call because the planar offsets are relative to the destination length (`color.rs:369` to `:374`). The band size is the implementer's, measured and recorded.
7. **The palette tables are read once, in the library.** A new `ocelli_dicom` reader returns `Option<PaletteColorLut>` from the three descriptors and three data elements, refusing Segmented Palette Color Lookup Table Data and a missing table by name. The oracle's private reader is replaced by it, by the lead's answer to question 7.
8. **The colour window is the existing LINEAR function, not a second copy.** The operator's answers to questions 3 and 4 follow cornerstone3D's rule, and that rule is PS3.3 C.11.2.1.2's LINEAR with output range `0 ..= 255`, character for character: `getVOILut.js:3` is `((x - (c - 0.5)) / (w - 1) + 0.5) * 255` clamped to `[0, 255]`, and `apply_window` at `crates/ocelli-pixel/src/lut.rs:776` to `:788` is the same expression with the clamps written as `x <= lower` and `x > upper`. At `w = 1`, cornerstone divides by zero and ends at 0 or 255 through the clamp, and `apply_window` reaches the same two values by comparison. So the colour window is `VoiTransform` with `VoiFunction::Linear`, `ymin = 0` and `ymax = 255`, applied to each channel, and the colour invert is `PresentationTransform` with `Inverse` over the same range, `0 + 255 - y` (`lut.rs:534` to `:543`). The only new code is the per-channel composition, `ColourWindow`, in `ocelli-pixel`. It does not go through `LutChain`, which D-23 keeps monochrome.
9. **Where the colour window sits.** After stage 4 and after the display scaling of item 11, before the byte rule, because cornerstone3D windows the 8-bit RGB its converters produce. Its initial value is width 256 and centre 128 for every colour image, ignoring the file's Window Center and Width, as `createImage.js:321` to `:324` does. Only LINEAR is offered, because the colour path never reads `voiLUTFunction`.
10. **Rounding at a tie is the one place this story knowingly differs from the oracle.** cornerstone stores into a `Uint8ClampedArray`, which rounds a tie to even. Tier C's `byte_from_channel` rounds a tie up (`cpu.rs:1062` to `:1063`) and serves every monochrome frame too, so changing it is out of scope. The divergence is at most one code, only where the windowed value is exactly `k + 0.5`, and AC19 measures one such case. A second source of one-code divergence is cornerstone rounding its YBR and palette conversions to bytes before the window, while the library windows the `f32` stage 4 output. Under D14 these are claimed as measured divergences, not hidden.
11. **The display scaling rule, the operator's answer to question 8.** A colour sample or a palette entry of `n` bits is shifted right by `n - 8` for `9 <= n <= 16`, and taken as it is for `n = 8`. For a sample, `n` is Bits Stored `(0028,0101)`. For a palette entry, `n` is the third value of the Red, Green and Blue Palette Color Lookup Table Descriptors, each channel by its own descriptor. The rule is decided from the header, never from the data. Colour with `n` below 8 or above 16, or signed colour samples, is refused by name. A sample is shifted before stage 4, so the 8-bit YCbCr matrix and its `+ 128` offset apply to an 8-bit value. A palette entry is shifted on the way out of the table.
12. **Statistics on a colour region.** One walk over the region, F-211's `region_statistics` in `examples/folder-viewer/src/measure.rs:468`, gathers three channel vectors and calls `roi_statistics` once per channel. The values are the colour stage's output after the display scaling and before the window and invert, so a window drag never changes a statistic, which is the rule F-211 keeps for monochrome. The unit is `RGB`. Padding is not applicable to colour and `padding_excluded` is 0.
13. **The ICC sentence.** A file with ICC Profile `(0028,2000)` at the top level is drawn and the corner and the image's help sentence say "This image's ICC profile was not applied." The presence test is the element alone, whatever its length. An ICC profile inside the Optical Path Sequence is a whole-slide concern this viewer does not open.
14. **The window link.** F-209's window link copies a window between viewports of the same kind only, colour to colour and monochrome to monochrome, because a colour window is in display sample units and a monochrome one is in modality units.
15. **The decoder's output length is the decoder's fact**, by the operator's answer to question 1. `Decoder` gains `decoded_output_len(&self, desc: &FrameDesc) -> usize`, defaulting to `desc.output_len()`, and `Registry` gains the matching query beside `decode_photometric_interpretation` and `decode_sample_layout` (`registry.rs:543` to `:563`). `Registry::decode` checks `out.len()` against the decoder's answer (`registry.rs:581`). The raw decoder answers `Rows * Columns * 2 * Bits Allocated / 8` for `YBR_FULL_422` and `YBR_PARTIAL_422`, from `StoredPixelDescription::stored_samples_per_pixel` (`stored_pixel.rs:125` to `:150`) rather than a second rule. JPEG keeps three samples per pixel. An odd Columns on a native 4:2:2 frame is refused as `FrameMismatch`, because a pixel pair has no partner and PS3.3 names no rule for it. The production callers that size a buffer from `desc.output_len()` move to the query: `examples/folder-viewer/src/preview.rs:708` and `:1282`, and `tools/oracle/src/produce.rs:723`.
16. **The component transform is inverted inside the decoder, once.** PS3.5 8.2.4 makes `YBR_ICT` and `YBR_RCT` the codestream's own transform, and `color.rs:233` to `:237` already refuses them unless a decoder converted them. So both JPEG 2000 adapters invert Annex G before the level shift and report `DecodePhotometricInterpretation::Rgb` and `DecodeSampleLayout::Interleaved` for a three-component codestream with the transform on. A three-component codestream with the transform off reports `Preserved`, so an `RGB` file stays `RGB` and the library's `ColorTransform` decides the rest. Where the inverse arithmetic lives depends on question 9. Components with different depths, signedness or sampling are refused as `UnsupportedPixelFormat`, as `openjph-core`'s own `check_validity` requires for the transform (`src/params/local.rs:970` to `:985`).

## Problem and reproducer

Pick a folder holding the corpus row `synthetic/sc_rgb_planar.dcm`, or any RGB secondary capture. The series lists, its thumbnail is empty, and selecting it shows "this preview shows monochrome frames only". `POST /preview` for that file answers 400 with `X-Ocelli-Error: not-monochrome`. A multi-frame colour ultrasound lists as one image (`page.html:1664`). The library has had a colour stage since F-030, so this is absent wiring, not absent arithmetic.

## Scope, API and compatibility

In scope:

- `preview.rs` builds a colour recipe for a colour frame: the data set's `SampleLayout` with Samples per Pixel and Planar Configuration, the `PixelDataEncoding` from the dispatch path, the decoder's `DecodePhotometricInterpretation` and `DecodeSampleLayout` mapped one to one onto `DecodedPhotometric` and `DecodedLayout`, the display scaling from the header, and the palette when the space is palette. `ColorTransform::resolve` decides, once, at hold time.
- `render_held` maps a colour frame through the new `ocelli-render` function into three bytes per pixel, with the viewport's colour window and invert applied per channel by `ocelli-pixel`.
- `inspect_json` gains `"colour"`, true when the frame is one of the drawn codes with the Samples per Pixel PS3.3 gives it, Rows and Columns above zero and a frame count of at least one. `"monochrome"` keeps its meaning. It also gains `"icc"`, true when ICC Profile `(0028,2000)` is present at the top level.
- The page draws, thumbnails, prefetches, tiles and lists every frame of a colour file. Window drag and invert act on a colour image from width 256 and centre 128. Presets and the file's windows are not offered on colour. The value readout reports unavailable with a sentence. A region reports its area and per-channel statistics.
- `ocelli-pixel` gains `ColourWindow`, the per-channel composition of the existing LINEAR window and inverse presentation stage, and `DisplayScale`, the one shift rule of item 11, used by `ColorTransform` for samples and by `PaletteColorLut` for entries.
- `ocelli-render` exports the byte step. `ocelli-dicom` exports the palette reader and the ICC presence test. `ocelli-codec` gains the output length query, and decodes three-component JPEG 2000 and HTJ2K with their component transform inverted, by the route question 9 chooses.
- `held.rs` and `serve.rs` charge a colour reply at three bytes per pixel and count the colour scratch.
- `scripts/corpus_synth.py` and `corpus/manifest.tsv` gain five synthetic rows, four colour JPEG 2000 rows `syntax/j2k_lossless_ybr_rct.dcm` (`.90`, OpenJPEG with the transform on), `syntax/j2k_lossy_ybr_ict.dcm` (`.91`), `syntax/htj2k_lossless_ybr_rct.dcm` (`.201`, `ojph_compress` with its colour transform on) and `syntax/htj2k_lossy_ybr_ict.dcm` (`.203`), each from the same saturated RGB reference including pure red, plus `synthetic/sc_rgb_icc.dcm`, an 8-bit RGB frame carrying a synthetic ICC Profile element written by the script. No third-party profile is copied. The data stays under ignored `corpus/data`, and nothing DICOM is staged.
- `tools/oracle/src/produce_current.rs` reads palettes through `ocelli_dicom::palette_color_lut`, and its candidate producer sizes buffers through the output length query.

Non-goals: applying an ICC profile, which is F-240, a colour value readout at the pointer, presets and the file's Window Center and Width on a colour image, any VOI LUT function other than LINEAR on colour, segmented palettes, Supplemental Palette Color, `YBR_PARTIAL_420`, JPEG 2000 colour with a progression other than LRCP or with sub-sampled components, signed colour, colour above 16 bits or below 8, scoring the oracle's colour rows, and any change to the synthetic study.

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
| error code | `colour-not-applicable` | a modality value, a preset, a file window or a non-LINEAR function asked of a colour frame |
| measure JSON | `stats` on colour | `{"unit":"RGB","channels":{"r":{...},"g":{...},"b":{...}}}`, each with `count`, `mean`, `sd`, `min` and `max` |
| page sentence | preset note | "This image is colour, so the file's windows and the presets do not apply. Drag to window each channel." |
| page sentence | value readout | "This image is colour, so it has no modality value." |
| page sentence | ICC | "This image's ICC profile was not applied." |

Refusal sentences for `colour-unavailable`, one per reason: a JPEG 2000 or HTJ2K codestream shape this build does not decode, colour samples outside 8 to 16 bits or signed, a palette descriptor outside 8 to 16 bits, a segmented or missing palette table, an odd Columns on a native 4:2:2 frame, a photometric value this build does not know.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `POST /preview`, `/file` | `decode_held`, then `render_held` with the colour recipe and the request's colour window | Decoder reports read once per held frame | Three bytes per pixel, `X-Ocelli-Channels: 3` |
| Colour space and layout | `ColorTransform::resolve` | Inputs from the data set, `DispatchPath` and the registry | One resolved transform per held frame |
| Display scaling | `ocelli_pixel::DisplayScale` | Bits Stored for samples, each palette descriptor's third value for entries | 8-bit-scale `f32`, or a named refusal |
| Colour window and invert | `ocelli_pixel::ColourWindow` over `VoiTransform` LINEAR and `PresentationTransform` Inverse | Initial 256 and 128, skipped at cornerstone's identity condition | Windowed `Rgb` |
| Byte rounding | `ocelli_render::colour_bytes_into` | Tier C's `byte_from_channel` and refusals | Bytes, or `Unavailable` |
| Palette tables | `ocelli_dicom::palette_color_lut` | Top-level elements only | `PaletteColorLut` or a named refusal |
| ICC presence | `ocelli_dicom::has_icc_profile` | Top-level element only | `icc` in inspect JSON, the page sentence |
| Decoded frame length | `Registry::decoded_output_len` | 4:2:2 native stores two samples, JPEG three | A native `YBR_FULL_422` frame decodes |
| JPEG 2000 and HTJ2K colour | `jpeg2000.rs` and `htj2k.rs` adapters, by question 9's route | Inverse Annex G before the level shift | Interleaved RGB, reported `Rgb` |
| `POST /value` | `value_json` | Colour has no stage 1 | `colour-not-applicable` |
| `POST /measure` | `measure_json` and F-211's `region_statistics` | One walk, three channel vectors, window never enters | Lines, area and per-channel statistics |
| Folder scan | `inspect_json` | `colour` and `icc` flags | Every frame listed |
| Thumbnails, prefetch, tiles | `requestGrey` and the page cache | Same reply at the initial colour window | RGB thumbnails and tiles |
| F-209 window link | `copyWindow` | Same kind only, item 14 | Colour to colour, monochrome to monochrome |
| F-209 scroll and zoom link, reference lines | `planes` in inspect JSON | Photometric independent | Unchanged |
| Presets, W/L drag, invert | page tool state | Drag and invert on, presets off, on colour | Requests carry a colour window and invert |
| Oracle candidate producer | `produce_current.rs` through `palette_color_lut` and `decoded_output_len` | One palette reader | Unchanged candidate bytes for 8-bit palettes, recorded change for the 16-bit row |
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
| Held colour frame | `POST /value` | 400 `colour-not-applicable` | No sample is read |
| Colour cell | Pointer moves | Readout shows the colour sentence | No request per move |
| Colour cell | Rectangle drawn | Area and R, G and B statistics shown | A later window drag leaves the statistics unchanged |
| Colour file with ICC Profile | Opened | Drawn, ICC sentence shown | Nothing refused |

Invariants across consumers: a colour frame never reaches `LutChain`, and a monochrome frame never reaches `ColorTransform` or `ColourWindow`, as D-23 makes true by construction for the first two and the recipe arms make true for the third. `stored_at` (`value.rs:345`) reads one sample per pixel and is never called on a colour frame, because the held value is an error for every colour frame and the colour region walk reads the colour stage's output instead.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Held colour frame | `decode_held` under F-216's cache | File id and frame, as F-216 and F-224 key it | Admission to the `Lru` | A refused resolve holds the refusal in the recipe, so a retry gives the same refusal without a second decode |
| Colour reply buffer | `render_capped` under the grey gate | One reply | Reply written | Charged at `width * height * 3` from admission. A refused render charges nothing |
| Colour scratch | the decode workspace | The worker | Returned with the workspace | Counted in `workspace_bytes`. A band larger than any later frame is freed by the existing over-twice rule |
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
| AC10 | The four new corpus rows and the existing refusals. A JPEG 2000 or HTJ2K codestream with three components and the transform on decodes and reports `Rgb` and `Interleaved`. Two components, components of unequal depth, sub-sampled components, and a JPEG 2000 progression other than LRCP still answer 400 `colour-unavailable` with the codestream sentence. The existing refusal tests in `tests/jpeg2000.rs` and `tests/htj2k.rs` are kept for those shapes and moved off the three-component one | `ycc_from_jpeg_2000_decodes_once_and_other_shapes_are_refused` | Any image at all for a refused shape, or the YBR_FULL matrix applied to RCT | implementation, needs question 9 |
| AC11 | Display scaling, from the header. Native RGB, Bits Allocated 16, Bits Stored 12, stored `4095 2048 15`. Reply `255 128 0`. The same samples at Bits Stored 16 reply `15 8 0`. Native YBR_FULL at Bits Stored 12, `1600 2400 1440`, shifts to `100 150 90` and replies `47 120 139` as AC3. Bits Stored 7, Bits Stored 17 in a 32-bit container, and Pixel Representation 1 refuse `colour-unavailable` by name. PS3.3 C.7.6.3.1 Bits Stored, the rule of item 11 | `colour_above_8_bits_is_shifted_by_bits_stored` | cornerstone's saturation, which gives `255 255 15`. Shifting by Bits Allocated, which gives `15 8 0` at Bits Stored 12. A data-dependent rule | implementation |
| AC12 | A window on a colour frame is applied per channel, and a following request at the initial window draws the same bytes as before. A window function other than LINEAR answers 400 `colour-not-applicable` and the last shown image stays | `a_colour_window_is_applied_per_channel_and_the_frame_survives` | Ignoring the header and answering 200 with the unwindowed image, or windowing R only | implementation |
| AC13 | Native RGB 2 by 1, stored `10 20 30 200 100 50`, a rectangle over both pixels. `stats.unit` is `RGB`. R count 2, mean 105, SD 95, min 10, max 200. G mean 60, SD 40, min 20, max 100. B mean 40, SD 10, min 30, max 50, SD as `roi_statistics`'s population form (`crates/ocelli-geom/src/stats.rs:154` to `:160`). The same rectangle after a window of width 100 and centre 128 gives the same statistics. `POST /value` on the frame refuses `colour-not-applicable`. A length reports its millimetres | `colour_regions_report_per_channel_statistics_from_one_walk` | Reading the R sample as a modality value through `stored_at`. Statistics of the windowed bytes. Three walks, which a counting test of `region_contains` calls rejects | implementation |
| AC14 | `inspect_json` of an RGB file of 3 frames has `"colour":true` and `"monochrome":false`. The page lists 3 images | `inspect_marks_colour_and_the_page_lists_every_frame` | A colour cine listed as one image | implementation |
| AC15 | Memory. A colour reply is charged `width * height * 3` under the grey gate, the colour scratch counts in `workspace_bytes`, and F-216's resident bound is re-measured with a colour series of the largest admitted frame and recorded beside F-216's figure | `a_colour_reply_charges_three_bytes_per_pixel`, plus the recorded measurement | Charging one byte per pixel, which lets three times the cap be resident | implementation, measured by the implementer |
| AC16 | In a browser through F-208's harness, a folder of the AC1, AC5, AC7 and AC26 files: each cell's canvas pixel at the centre of source pixel 0 reads the hand value, the thumbnail is not black, a window drag changes the colour cell and AC19's values appear at the dragged window, invert gives `255 - y`, presets are off with the colour sentence, the readout shows the colour sentence, the ICC sentence shows on the AC26 file only, and a CT beside it still windows | `examples/folder-viewer/tests/browser/colour.mjs` | A canvas filled from the first channel only, or a window drag that sends nothing for colour | integrated |
| AC17 | Before the fixtures are accepted, the reviewer checks PS3.3 C.7.6.3.1, C.7.6.3.1.2, C.7.6.3.1.3, C.7.6.3.1.5, C.7.6.3.1.6 and C.7.9 and PS3.5 8.2.4 and A.4.4 at dicom.nema.org, current edition, and ISO/IEC 15444-1 Annex G, against the matrices, transforms and rules quoted above, and names the editions read | Review note | Fixtures citing text nobody checked | reviewer, before implementation |
| AC18 | Mutation. Swapping two inverse coefficients in `FULL_RANGE`, dropping the `+ 16` offset, making the planar gather interleaved, using `w` for `w - 1` in the colour window, shifting by Bits Allocated, swapping `Y1` and `Y2` in the inverse RCT, and moving the inverse after the level shift turn AC3, AC6, AC2, AC19, AC11 and AC24 red, the last through its pure red case | Recorded in the review | A test that passes on the wrong code | implementation, reviewer |
| AC19 | Colour window, cornerstone3D's rule through the existing LINEAR function, `f32`, then the tier C byte rule. Native RGB 1 by 1, stored `100 30 200`, width 100, centre 128: channels `56.666664 0 255`, reply `57 0 255`. With invert: `198.33334 255 0`, reply `198 255 0`. Stored `128 128 128` at the same window: `128.78787`, reply `129 129 129`, inverted `126 126 126`. Width 256 and centre 128, stored `10 200 50`: reply `10 200 50`. Width 255 and centre 127, stored `100 100 100`: skipped as `renderColorImage.js:30` to `:36` skips, reply `100 100 100`, where the formula would give `100.8957` and `101`. Width 2, centre 128, stored `127 128 0`: `0 255 0`. Width 10, centre 128, stored `125 128 130`: `56.666664 141.66667 198.33334`, reply `57 142 198`, inverted `198 113 57` | `colour_window_is_linear_per_channel_and_inverted_after` | `w` in place of `w - 1`, which gives `64 140 191` at width 10. LINEAR_EXACT's `c` and `w`, which gives `51 128 179`. Inverting before the window. Applying the formula at cornerstone's identity condition. The width 100 rows alone do not reject the `w` mutation, which is why the width 10 row exists | implementation |
| AC20 | Reference observation of AC19's rule. The oracle's reference half renders `synthetic/sc_rgb_interleaved.dcm` through cornerstone3D 5.8.2 under headless Chromium on SwiftShader at width 100 and centre 128, invert off and on, through one new `render-params.json` row per case, and the candidate bytes from `colour_bytes_into` at the same window are compared per pixel. Every pixel agrees except at an exact tie, and the count of tie pixels and their one-code differences are recorded. The comparison is a recorded observation, not a scored oracle row, because HLD 25.1 states no colour threshold. Missing code: the two parameter rows and the colour window in `produce_current.rs` | Recorded in `docs/lld/oracle.md` with the 5.8.2 pin and the lockfile integrity check | A rule transcribed from memory rather than the oracle | integrated, implementer, on the host the oracle gate already uses |
| AC21 | Tie divergence, measured. Native RGB 1 by 1, stored `130 130 130`, width 511, centre 128.5: `128.5` exactly, reply `129 129 129` by `byte_from_channel`. cornerstone stores 128 by ties to even. The test asserts 129, and the plan's D14 claim names this case | `a_colour_window_tie_rounds_up_as_tier_c_does` | A quiet change to the shared byte rule, which would move every monochrome tie | implementation |
| AC22 | Palette scaling. PALETTE COLOR 5 by 1, 8-bit stored `0 1 2 3 4`, descriptors `5\0\16` for all three, red `0 255 256 32768 65535`, green and blue `65535 32768 256 255 0`. Reply `0 255 255 0 128 128 1 1 1 128 0 0 255 0 0`. A 16-bit descriptor whose entries are `0 100 255` replies `0 0 0` for those entries, where cornerstone keeps `0 100 255` because no red entry is above 255. Descriptor bits 12 shift by 4. Bits 17 refuse by name | `palette_entries_are_shifted_by_their_descriptor` | cornerstone's red-only, data-dependent test. Rounding instead of truncating, which gives 128 for 32768 too but 1 for 255 | implementation |
| AC23 | Decoder output length. `decoded_output_len` for native `YBR_FULL_422` 2 by 1 at 8 bits is 4, for native RGB 2 by 1 is 6, and for JPEG baseline `YBR_FULL_422` 2 by 1 is 6. `Registry::decode` refuses a 6-byte buffer for the native 4:2:2 frame with `OutputLength { expected: 4, actual: 6 }` | `crates/ocelli-codec/tests/registry.rs`, `decoded_output_len_is_the_decoders` | A registry that still checks `desc.output_len()` | implementation |
| AC24 | Inverse RCT, ISO/IEC 15444-1 G.2. Hand values at `B = 8`: `200 100 50` level-shifts to `72 -28 -78`, forward `Y0 = floor(-62 / 4) = -16`, `Y1 = -50`, `Y2 = 100`, and the inverse gives `I1 = -16 - floor(50 / 4) = -28`, `I0 = 72`, `I2 = -78`, so `200 100 50`. Pure red `255 0 0` gives `Y0 = -65`, `Y1 = 0`, `Y2 = 255` and back. The lossless rows `syntax/j2k_lossless_ybr_rct.dcm` and `syntax/htj2k_lossless_ybr_rct.dcm` decode byte for byte to their RGB reference | `inverse_rct_is_exact_before_the_level_shift`, and the corpus rows in `crates/ocelli-dicom/tests/corpus.rs` | Inverting after `openjph-core`'s level shift and clamp, which gives `159 32 32` for pure red | implementation, needs question 9 |
| AC25 | Inverse ICT, ISO/IEC 15444-1 G.3. Hand values: `Y0 = -28`, `Y1 = 22`, `Y2 = -38` give `I0 = -28 + 1.402 * -38 = -81.276`, `I1 = -28 - 0.34413 * 22 - 0.71414 * -38 = -8.43354`, `I2 = -28 + 1.772 * 22 = 10.984`, then `+ 128` and the decoder's rounding to `47 120 139`. The lossy rows `syntax/j2k_lossy_ybr_ict.dcm` and `syntax/htj2k_lossy_ybr_ict.dcm` are compared with OpenJPEG's own decode of the same codestream through the corpus tooling, the largest per-channel difference is measured and recorded, and a decode with the inverse left out is shown to differ by more | `inverse_ict_uses_annex_g_coefficients`, plus the recorded measurement | The YBR_FULL inverse of `color.rs` in place of Annex G's, or no inverse | implementation, needs question 9 |
| AC26 | ICC sentence. `synthetic/sc_rgb_icc.dcm` draws the same bytes as the same frame without the element, inspect JSON has `"icc":true`, and the page shows "This image's ICC profile was not applied." The file without the element has `"icc":false` and no sentence | `an_icc_profile_is_named_as_not_applied` | Refusing the file, or applying nothing and saying nothing | implementation, integrated in AC16 |
| AC27 | One palette reader. `tools/oracle/src/produce_current.rs` calls `ocelli_dicom::palette_color_lut` and its private reader at `:578` to `:687` is deleted. The candidate bytes for `synthetic/sc_palette_color.dcm` are byte-identical before and after at the sprint corpus gate. `synthetic/sc_palette_color_16.dcm` changes from its current outcome to AC22's scaled bytes, and both outcomes are recorded | Corpus gate output, recorded | Two readers that agree today and drift later | implementation, sprint corpus gate |
| AC28 | Window link by kind. With link on, a window dragged on an RGB viewport reaches a second RGB viewport and not a CT viewport, and one dragged on the CT reaches the other monochrome viewport and not the RGB one | `examples/folder-viewer/tests/browser/colour.mjs`, link case | A modality-unit window sent to a colour viewport | integrated |

Worked values for AC3, AC5 and AC6. For `100 150 90`, `Cb' = 22` and `Cr' = -38`. `R = 100 - 0.00003682 * 22 + 1.4019876 * -38 = 46.72366`, `G = 100 - 0.3441133 * 22 - 0.7141038 * -38 = 119.56544`, `B = 100 + 1.7719781 * 22 - 0.000134583 * -38 = 138.98863`. For the 4:2:2 pair, `Cb' = -28` and `Cr' = 32`. Pixel 1 is `50 + 1.4019876 * 32 + 0.0010310 = 94.86463`, `50 + 9.6351724 - 22.851322 = 36.78385` and `50 - 49.615387 - 0.0043067 = 0.38031`, which round to `95 37 0`. Pixel 2 adds 150 to each channel, `244.86464 186.78386 150.38031`, which round to `245 187 150`. For the partial pair, `Y' = 0` gives 0 and `Y' = 219` gives `1.1644155 * 219 = 255.007`, which clamps to 255.

## Approach

1. **ocelli-pixel.** Add `DisplayScale` and `ColourWindow` in `color.rs` or a sibling module. `ColourWindow` holds a `VoiTransform` built by `VoiTransform::new(None, &[center], &[width], 0, VoiFunction::Linear, 0.0, 255.0)` and, when inverted, a `PresentationTransform` with `Inverse` over `0 ..= 255`, and maps each channel through `apply` on each. No window arithmetic is written in it. `ColorTransform::resolve` takes the sample `DisplayScale`, and `PaletteColorLut::new` takes one per channel, so the shift happens once at each place a value enters stage 4. Fixtures for both in `crates/ocelli-pixel/tests/color.rs` and `tests/palette.rs`, hand-computed as AC11, AC19, AC21 and AC22.
2. **ocelli-render.** Factor the accept, display-byte check, `map_into` and `rgba_from_rgb` steps of `raster_color_into` (`cpu.rs:363` to `:395`, `:586` to `:599`) into one private routine, call it from `raster_color_into`, and export `colour_bytes_into` writing three bytes per pixel over bands, with `ColourWindow::map_into` between the map and the byte step. `raster_color_into` passes `ColourWindow::initial()`, which is skipped, so its existing tests stay green unchanged.
3. **ocelli-dicom.** Add `palette_color_lut` in `frame_open.rs`, reading `(0028,1101)` to `(0028,1103)` and `(0028,1201)` to `(0028,1203)` into `LutDescriptor::new` and `PaletteColorLut::new`. Refuse `(0028,1221)` to `(0028,1223)` by name. Add `has_icc_profile` for `(0028,2000)`.
4. **ocelli-codec, length.** Add `Decoder::decoded_output_len` and `Registry::decoded_output_len`, override it in the raw decoder for the two 4:2:2 codes, and check the decoder's answer in `Registry::decode`. Move the three production callers.
5. **ocelli-codec, JPEG 2000 and HTJ2K colour.** Return every component from `parse_siz`. Accept three components of equal depth, signedness and 1 by 1 sampling with the transform on or off. Report `Rgb` and `Interleaved` when it is on. The inverse transform and the level shift run in Annex G's order by question 9's route. `scripts/corpus_synth.py` writes the four codestream rows and records the encoder versions it already records.
6. **preview.rs.** Accept the colour codes in `photometric` and `frame_parts`, build `SampleLayout` from the header, take the decoder's two reports in `decode_encapsulated` instead of refusing, and add a colour arm to `Recipe` carrying the resolved transform, the display scale and the ICC flag. `value` is `colour-not-applicable` for that arm. `select_window` builds a `ColourWindow` from the request's centre, width and invert on that arm, starts from `ColourWindow::initial()`, and refuses a function other than LINEAR, a preset and the file's windows. Replace `NotMonochrome` with `ColourUnavailable` and add `ColourNotApplicable`. `inspect_json` writes `colour` and `icc`.
7. **held.rs and serve.rs.** `grey_bytes` takes the channel count. `Grey` carries it and the header writer emits `X-Ocelli-Channels`. `workspace_bytes` counts the `Rgb` scratch. `charged_bytes` counts the palette.
8. **value.rs and measure.rs.** No change to `stored_at`. `region_statistics` gains a colour arm in the same walk, reading the colour stage's output for each contained pixel through the same band routine and pushing three vectors, then three `roi_statistics` calls. `push_stats` writes the per-channel shape.
9. **page.html.** One predicate for "this frame draws" beside `info.monochrome`, used at the listed lines. `fillGreyImage`, `prepareDisplay`, `paintGrey` and `paintThumbBitmap` read the channel count. W/L drag and invert send their headers on colour from width 256 and centre 128. `cacheKey` writes the colour token. The preset note, the readout, the ICC sentence, the statistics panel and the fault sentence change. `copyWindow` sends by kind.
10. **tools/oracle.** `produce_current.rs` calls `palette_color_lut` and `decoded_output_len`, gains the colour window for AC20's two rows, and `render-params.json` gains those rows.

Rejected: calling `CpuRaster::raster_color_into` with an identity view, as the oracle does, because it carries a view, an index map, a device extent and a generation for a reply that is never resampled, and its scratch is a whole frame. Writing the byte rule in the folder viewer, which is a second copy at the boundary HLD 27.3 names. Converting or windowing in page JavaScript, which has no fixture runner and puts colour arithmetic outside the library. A colour window with its own formula, which would be a second copy of PS3.3 C.11.2.1.2's LINEAR. Applying the window inside `ColorTransform`, which would make stage 4 depend on view state. cornerstone's red-only, data-dependent palette test, rejected under item 11. Saturating colour above 8 bits, which turns a 12-bit image nearly white.

## Dependencies, write ownership and resources

Declared dependencies F-030, F-209 and F-216, all done. F-212 now depends on this story, by the operator's order of 2026-11-07, and F-213, F-214 and F-215 follow F-212 as its plan records. F-240, filed on 2026-11-07, depends on this story and applies matrix ICC profiles where this story only names them.

Writes `examples/folder-viewer/src/preview.rs`, `serve.rs`, `held.rs`, `measure.rs`, `page.html`, `use.html`, `examples/folder-viewer/tests/preview.rs`, `tests/value.rs`, `tests/measure.rs`, a new `tests/browser/colour.mjs`, `crates/ocelli-pixel/src/color.rs`, `lib.rs` and `tests/color.rs` and `tests/palette.rs`, `crates/ocelli-render/src/cpu.rs` and `lib.rs`, `crates/ocelli-dicom/src/frame_open.rs`, `lib.rs` and `tests/corpus.rs`, `crates/ocelli-codec/src/native.rs`, `registry.rs`, `jpeg2000.rs`, `htj2k.rs` and their tests, the vendored crates and `scripts/pin_and_size_check.py` under question 9 option A, `scripts/corpus_synth.py`, `corpus/manifest.tsv`, `tools/oracle/src/produce.rs`, `produce_current.rs` and `render-params.json`, `README.md`, `docs/hld/DEVIATIONS.md`, and the LLD files below.

`preview.rs`, `serve.rs`, `held.rs` and `page.html` are also written by F-224, F-225, F-226, F-227 and F-228, whose worker worktrees exist today, and by F-212 to F-215 after this story. They cannot have concurrent writers. The safe order is after those five integrate, or a three-way merge at integration with this story rebased onto them. `frame_open.rs` is shared with F-225. `registry.rs` and `corpus/manifest.tsv` are shared with F-229, whose JPEG XL decoder registers through the same registry and adds corpus rows, so the second to integrate merges three ways. No GPU is needed except AC20's reference half, which uses the oracle gate's existing headless Chromium on SwiftShader.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-folder-viewer`, `bin/ocelli.sh check ocelli-pixel`, `bin/ocelli.sh check ocelli-render`, `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh check ocelli-codec` while iterating, then the feature profile once at the integration tree. The sprint profile runs once for the sprint, per the operator's rule of 2026-11-06. AC16 and AC28 run under the F-208 browser harness. AC15's resident measurement names the host and the frame used. AC20 runs the oracle's reference half and records its comparison in `docs/lld/oracle.md`. AC25's lossy comparison names the OpenJPEG version `scripts/corpus_synth.py` records. AC27 is read from the sprint corpus gate. A later edit to any file in the write set invalidates the record. The oracle's colour rows stay not scored, so no scored comparison is claimed.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The folder viewer's local HTTP reply carries display bytes, as it does today for grey.
- Render-loop allocation: n/a. The colour scratch is a workspace buffer reused across requests and counted in `workspace_bytes`.
- unsafe: none in the workspace. A vendored patch under question 9 option A touches external code whose unsafe sites are counted by `scripts/unsafe_allowlist_check.py`'s external audit, and the count is re-measured.
- Tier A (WebGPU): n/a for the folder viewer, whose page opens no device, as F-210 and F-212 record. In the library, `ColourWindow` and `DisplayScale` have no GPU form in this story and no GPU entry point accepts a colour window or a scaled colour sample, so nothing can draw a colour window differently on tier A. The GPU colour path, `ColourLut` in `crates/ocelli-render/src/colour.rs`, is unchanged.
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

## Parity surface covered

None measured as a scored row. `docs/hld/B-parity-surface.md` has no colour row. This story follows cornerstone3D 5.8.2 on the colour window, its initial value of width 256 and centre 128, invert after the window, and the 16-bit palette shift, and AC20 records the agreement of the window rule on the reference renderer. It knowingly differs from the oracle in four measured places, each claimed under D14 rather than hidden: a window tie rounds up (AC21), YBR and palette conversions are windowed from `f32` rather than from cornerstone's rounded bytes, a 16-bit palette whose entries are all below 256 is shifted (AC22), and colour samples above 8 bits are shifted rather than saturated (AC11). The oracle's colour candidates stay not scored because HLD 25.1 states no colour threshold.

## Deviations

Three rows are needed, and under `.claude/commands/design.md` step 3 they are written into `docs/hld/DEVIATIONS.md` with their reason in the same change that sets this plan to approved, at the next free numbers. Their text is settled by the operator's answers and is recorded here so the approving change copies it rather than rewrites it.

1. **Colour window and invert.** HLD says: §18's table orders stage 2, VOI, and stage 3, Presentation, before stage 4, and D-23 records that a three-sample frame never enters stages 1 to 3 because "PS3.3 C.11's chain does not apply to it at all". We do: a colour frame is windowed and inverted per channel after stage 4 through `ColourWindow`, which applies the existing `VoiTransform` LINEAR window over `0 ..= 255` and the existing `PresentationTransform` INVERSE to each of R, G and B, initial width 256 and centre 128, ignoring the file's windows. Why: the operator's answer of 2026-11-07 to follow cornerstone3D 5.8.2, which does exactly this (`getVOILut.js:1` to `:7`, `generateColorLUT.js:13` to `:21`, `createImage.js:321` to `:324`). PS3.3 C.11.2 does not define a window for colour, so this is a measured parity choice and not a reading of the standard. The arithmetic still exists once, because it is the stage 2 and stage 3 functions applied per channel, and `LutChain` stays monochrome as D-23 requires. Raised: F-239.
2. **ICC profile drawn without being applied.** HLD says: §18's stage 4 is "Palette colour LUT, or the display colour pipeline", and §35 calls ICC colour management "a correctness requirement" (`docs/hld/26-differentiating-capabilities.md:136`). We do: a frame carrying ICC Profile `(0028,2000)` is drawn in device RGB with the sentence "This image's ICC profile was not applied." Why: the operator's answer of 2026-11-07. Refusing would leave most secondary capture and ultrasound files undrawn, and the sentence makes the omission visible rather than quiet. F-240 retires this row for matrix and tone-curve profiles, and F-145 for the rest. Raised: F-239.
3. **Vendored JPEG 2000 decoder patched**, only under question 9 option A. HLD says: D-22 and the pins gate keep `openjph-core` byte-identical to its published archive, and D-21 and the gate keep `ritk-codecs` to its two-file manifest patch. We do: a recorded source patch so the inverse component transform runs before the level shift and its clamp, and, for `ritk-codecs`, so a three-component LRCP codestream is read. Why: measured above, an unpatched decode of pure red RCT is `159 32 32`. Raised: F-239.

## LLD impact

- `docs/lld/pixel-pipeline.md`: stage 4 has a production consumer, the folder viewer. `DisplayScale` and `ColourWindow`, their place after stage 4, and the first deviation above.
- `docs/lld/render-graph.md`: `colour_bytes_into`, the shared byte step and the window between map and bytes.
- `docs/lld/codecs.md`: `decoded_output_len`, native 4:2:2 length, three-component JPEG 2000 and HTJ2K, Annex G's order and question 9's route.
- `docs/lld/frame-open.md`: `palette_color_lut` and `has_icc_profile`.
- `docs/lld/oracle.md`: the one palette reader, AC20's recorded colour window comparison and AC27's outcomes.
- `docs/lld/corpus.md`: the five new rows.

## User-facing documentation impact

- `README.md:309` to `:314`: the sentence "A colour frame ... stays on a fixed error sentence" is replaced. Colour frames draw. Window drag and invert act on each channel from width 256 and centre 128, as cornerstone3D does. A colour image has no presets and no value at the pointer, and a region on it reports R, G and B statistics. A file with an ICC profile is drawn with a sentence saying the profile was not applied. A colour image this build cannot draw shows a sentence naming why. Step numbers are measured at implementation.
- `examples/folder-viewer/src/use.html:638`: the stage 4 paragraph says the folder viewer draws colour through `ColorTransform`, `DisplayScale`, `ColourWindow` and `colour_bytes_into`, that the colour window follows cornerstone3D 5.8.2 and not PS3.3, and that ICC is not applied until F-240. The crate table at `use.html:215` names the new `ocelli-pixel`, `ocelli-render`, `ocelli-dicom` and `ocelli-codec` functions.
- `examples/folder-viewer/src/more.html`: no change. It has no colour card, and ICC remains library behaviour this page does not draw, which `use.html:638` already states.

## Decided by the operator on 2026-11-07

Each answer is the operator's unless it says lead. The original options are kept in the history of this file at 5451cb7d.

1. **Native 4:2:2 length.** Option A. Each decoder reports its own output length through a registry query, so native `YBR_FULL_422` is two samples per pixel and JPEG stays three. Applied in item 15, AC5 and AC23.
2. **YBR_ICT and YBR_RCT.** Option B, widened. F-239 decodes `YBR_ICT` and `YBR_RCT`, multi-component JPEG 2000 and HTJ2K, in `ocelli-codec`, `jpeg2000.rs` and `htj2k.rs`. Measured for the answer: neither vendored dependency can do it unchanged, which raises question 9. Applied in item 16, AC10, AC24, AC25 and the five corpus rows. The estimate rose to 11w.
3. **Window on a colour frame.** As cornerstone3D 5.8.2 does: one LINEAR window applied to each of R, G and B, initial width 256 and centre 128. The arithmetic is put once in `ocelli-pixel`, as the existing LINEAR function per channel. This follows the oracle and is not defined by PS3.3 C.11.2 for colour, recorded as a measured parity choice, and needs the first deviation above, because §18 and D-23 say colour does not enter stages 1 to 3. Applied in items 8 to 10, AC12, AC19 to AC21.
4. **Invert on a colour frame.** As cornerstone3D 5.8.2 does, `255 - y` per channel after the window, through the existing INVERSE presentation function over `0 ..= 255`. Same deviation as 3. Applied in AC19.
5. **ICC Profile (0028,2000).** Shown, with a page sentence that the profile was not applied, until a new story applies it. That story is F-240, filed the same day. Needs the second deviation above. Applied in item 13 and AC26.
6. **Measure on a colour region.** Area plus mean, SD, minimum and maximum per channel R, G and B, through F-211's single region walk. Applied in item 12 and AC13.
7. **The oracle's palette reader** (lead). Option A, replaced by the new `ocelli-dicom` reader so one exists. Applied in AC27.
8. **Colour above 8 bits and palette entries above 255.** One library rule in `ocelli-pixel` scales both to the display's 8 bits. The rule is item 11: shift right by `n - 8` for `9 <= n <= 16`, where `n` is Bits Stored for a sample and the descriptor's third value for a palette entry, decided from the header. It matches cornerstone's `>> 8` for a 16-bit palette (`convertPALETTECOLOR.js:2` to `:9`). It departs from cornerstone where cornerstone is not defensible against PS3.3 C.7.6.3.1.5: that section makes the descriptor's third value the bit depth of every entry, so a palette whose descriptor says 16 is scaled as 16 even when its red entries happen to be small, and the green and blue tables are scaled by their own descriptors rather than by the red data. For colour samples, PS3.3 gives Bits Stored as the range of the sample, so a 12-bit sample is scaled by 12, where cornerstone saturates it. The truncating shift is kept because it is cornerstone's and gives every output code an equal share of the input range. AC17 makes the reviewer check C.7.6.3.1.5's text. Applied in AC11 and AC22.

## Open questions

9. **How the JPEG 2000 colour decode is built.** Raised by the answer to question 2 and blocking approval. Affects AC10, AC24, AC25, item 16, the write set and the estimate. Measured facts: `ritk-codecs` reads one component and refuses any MCT (`vendor/ritk-codecs-0.6.0/src/jpeg_2000/image.rs:55` to `:60`, `:79` to `:84`). `openjph-core` reads several components but never applies its own inverse transform and clamps each component at the level shift first (`src/codestream/tile_comp.rs:346` to `:367`), which corrupts saturated RCT colour. Options: A, patch both vendored crates with recorded provenance, extending the pins gate's patch list as `ritk-codecs` already has one, with the third deviation above: for `openjph-core`, call its existing `rct_backward` or `ict_backward` before the level shift, and for `ritk-codecs`, a three-component LRCP packet reader with the inverse transform. B, replace one or both with another dependency, assessed under `docs/SOURCE-POLICY.md`'s three questions with a dated extension row, an exact pin, `deny.toml`'s licence list, the no-Rayon graph, the notices gate and both wasm size budgets, and a deviation row as D-28 did for JPEG XL. C, split the decoding into its own story and have F-239 refuse `YBR_ICT` and `YBR_RCT` by name, which reverses part of answer 2 and brings the estimate to 7.5w. Owner: operator. Lead recommendation: A for HTJ2K, because the fix calls code the crate already has and AC24's pure red case proves it, and the operator's choice between A and B for JPEG 2000 Part 1, because a multi-component packet reader is new decoder code in a vendored crate. **Unblock condition: an answer naming the route for HTJ2K and for JPEG 2000 Part 1. The approving change then writes the deviation rows above with their numbers and sets Status: approved.** Deferral keeps this plan draft and F-239 unstartable.
