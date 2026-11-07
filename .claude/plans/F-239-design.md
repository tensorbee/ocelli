# F-239, Folder viewer colour frames through the library's colour stage

**Status**: draft
**Epic ref**: E38.37
**Sprint**: S46
**Estimate**: 4w

Story row, `docs/sprints/BACKLOG.md`: "Folder viewer colour frames: RGB, YBR_FULL, YBR_FULL_422, YBR_ICT, YBR_RCT and PALETTE COLOR with Planar Configuration drawn through ocelli-pixel's ColorTransform on the decoder's reported output space, window, invert, value and region statistics reported unavailable on a colour frame, and every colour shape this build cannot draw refused by name". Layer All, depends on F-030, F-209 and F-216, milestone M20.

`allocation.json` notes: "Decided by the operator on 2026-11-07: a colour series shows the fault 'this preview shows monochrome frames only' from PreviewError::NotMonochrome today, and the folder viewer previews colour frames through the library's colour stage instead. Scheduled first in the folder viewer chain, ahead of F-212."

This plan is a draft ready for the consolidated question round. Every path and line below was read on `sprint/s46` at 636fdcc1. Open questions 1 to 8 change behaviour or scope, so the status stays draft until they are answered.

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

**A native 4:2:2 frame is refused today, not misread.** `FrameDesc::output_len` is `Rows * Columns * Samples per Pixel * Bits Allocated / 8` (`registry.rs:304` to `:319`), and `RawDecoder::index_native_value` sizes the whole value from `frame_bits`, which multiplies by Samples per Pixel (`native.rs:96`, `native.rs:339` to `:345`). For `YBR_FULL_422` that is three samples per pixel, while PS3.3 C.7.6.3.1.2 stores two, as `StoredPixelDescription::stored_samples_per_pixel` already says (`crates/ocelli-pixel/src/stored_pixel.rs:125` to `:150`). The value is then shorter than expected and `index_native_value` returns `InvalidCodestream` (`native.rs:117` to `:119`). The folder viewer's `borrowed_pixels_fit` makes the same multiplication (`preview.rs:1255` to `:1270`). The JPEG decoder's output for the same photometric value is three samples per pixel, so the one length cannot serve both routes.

**The folder viewer's memory terms are one byte per pixel.** The reply body is "One display byte per pixel" (`preview.rs:113`). `grey_bytes` is `width * height` (`serve.rs:875` to `:879`). `GREY_CAP` is "the resident bound's grey term `P * N_max` at one grey byte per pixel" (`held.rs:370` to `:375`). The page checks `bytes.length !== width * height` (`page.html:2201` to `:2206`) and `PREPARED_PIXEL_CAP` compares a byte length with a pixel count (`page.html:853`, `:2302`). The held decoded samples are charged by buffer capacity (`preview.rs:510`), so three samples per pixel are already counted there. The workspace counts `scratch` as `Stored` per sample, `native` and `grey` (`held.rs:305` to `:311`, `:355` to `:362`).

**cornerstone3D 5.8.2, the reference, read under `tools/oracle/node_modules/@cornerstonejs/`.** `dicom-image-loader/dist/esm/imageLoader/convertColorSpace.js:18` to `:40` copies `YBR_ICT`, `YBR_RCT` and `YBR_PARTIAL_420` through as RGB, on the decoder's word. `colorSpaceConverters/convertYBRFullByPixel.js` uses `1.402`, `0.34414`, `0.71414` and `1.772`. `colorSpaceConverters/convertPALETTECOLOR.js:24` shifts every 16-bit entry right by 8 when any entry is above 255. `core/dist/esm/RenderingEngine/helpers/cpuFallback/rendering/renderColorImage.js:10` to `:22` builds a per-channel colour LUT from the viewport window and `invert`, so cornerstone applies a window and an inversion to a colour image. The oracle writes colour, palette and YBR candidates and does not score them (`produce_current.rs:3` to `:4`, `:375` to `:383`).

## What the specification does not cover

Every item is this plan's decision unless it names an open question.

1. **Where the byte rounding lives.** The library leaves rounding to "the render or export boundary" (`color.rs:26` to `:30`). The folder viewer reply is such a boundary. It reuses tier C's rule from `ocelli-render` rather than writing one. `ocelli-render` gains one public function that runs the existing steps without resampling, and `raster_color_into` is refactored to call the same steps, so the rule exists once.
2. **The reply format.** Three bytes per pixel, `R G B` row major, with `X-Ocelli-Channels: 3`. A monochrome reply keeps one byte and gains `X-Ocelli-Channels: 1`. RGBA was rejected because the alpha byte is constant and costs a quarter of every colour reply under the cap.
3. **Which photometric values are drawn.** `RGB`, `YBR_FULL`, `YBR_FULL_422`, `PALETTE COLOR`, and `YBR_ICT` or `YBR_RCT` whenever a decoder reports `Rgb` for them. `YBR_PARTIAL_422` is drawn too, because the library route exists and a local refusal would be a rule with no source. `YBR_PARTIAL_420` is not a code `frame_open` or the folder viewer knows, so it is refused by name.
4. **Value, statistics, window and invert on a colour frame.** D-23 says PS3.3 C.11's chain does not apply to a three-sample frame. A palette frame's stored value is an index, not a modality value. So stage 1 is not built for either, the value and every region statistic report unavailable, and no window or invert is applied. Open questions 3 and 4 put the window and invert choice to the operator, because cornerstone3D applies both.
5. **A colour shape the build cannot draw is refused, never approximated.** One new code, `colour-unavailable`, with a sentence that names the reason from a closed list. It replaces `not-monochrome`, which no longer has a meaning.
6. **Scratch is bounded by a band of rows, not by the frame.** `ColorTransform::map_into` takes `&[Stored]` and writes `&mut [Rgb]`, twelve bytes per pixel each way. At `MAX_PIXELS` (`preview.rs:96`) a whole frame would be about 805 MB per buffer. The conversion runs over bands of whole rows with an even row width for 4:2:2, and a planar frame gathers the band's three plane rows into one planar band, which is still one valid `map_into` call because the planar offsets are relative to the destination length (`color.rs:369` to `:374`). The band size is the implementer's, measured and recorded.
7. **The palette tables are read once, in the library.** A new `ocelli_dicom` reader returns `Option<PaletteColorLut>` from the three descriptors and three data elements, refusing Segmented Palette Color Lookup Table Data and a missing table by name. Open question 7 asks whether the oracle's private reader is replaced by it in this story.

## Problem and reproducer

Pick a folder holding the corpus row `synthetic/sc_rgb_planar.dcm`, or any RGB secondary capture. The series lists, its thumbnail is empty, and selecting it shows "this preview shows monochrome frames only". `POST /preview` for that file answers 400 with `X-Ocelli-Error: not-monochrome`. A multi-frame colour ultrasound lists as one image (`page.html:1664`). The library has had a colour stage since F-030, so this is absent wiring, not absent arithmetic.

## Scope, API and compatibility

In scope:

- `preview.rs` builds a colour recipe for a colour frame: the data set's `SampleLayout` with Samples per Pixel and Planar Configuration, the `PixelDataEncoding` from the dispatch path, the decoder's `DecodePhotometricInterpretation` and `DecodeSampleLayout` mapped one to one onto `DecodedPhotometric` and `DecodedLayout`, and the palette when the space is palette. `ColorTransform::resolve` decides, once, at hold time.
- `render_held` maps a colour frame through the new `ocelli-render` function into three bytes per pixel.
- `inspect_json` gains `"colour"`, true when the frame is one of the drawn codes with the Samples per Pixel PS3.3 gives it, Rows and Columns above zero and a frame count of at least one. `"monochrome"` keeps its meaning.
- The page draws, thumbnails, prefetches, tiles and lists every frame of a colour file. Every tool that needs a scalar value reports unavailable with a sentence.
- `ocelli-render` exports the byte step. `ocelli-dicom` exports the palette reader. `ocelli-codec` sizes a native 4:2:2 frame from the stored samples, under open question 1.
- `held.rs` and `serve.rs` charge a colour reply at three bytes per pixel and count the colour scratch.

Non-goals: ICC (open question 5), a colour value readout at the pointer, window or invert on a colour frame unless questions 3 and 4 say otherwise, segmented palettes, Supplemental Palette Color, JPEG 2000 or HTJ2K multi-component decoding unless question 2 says otherwise, and any change to the synthetic study.

Public signatures added, names final at implementation:

```rust
// ocelli-render, beside CpuRaster. The same accept, map and byte steps as
// raster_color_into, with no view, no resampling and no generation.
pub fn colour_bytes_into(
    color: RasterColor<'_>,
    stored: &[Stored],
    columns: u32,
    scratch: &mut Vec<Rgb>,
    out: &mut [u8],
) -> Result<(), RasterError>;

// ocelli-dicom. None for a frame that is not PALETTE COLOR.
pub fn palette_color_lut(metadata: &MetadataSet) -> Result<Option<PaletteColorLut>, FrameOpenError>;
```

Protocol strings that change:

| Where | Name | Value |
|-------|------|-------|
| inspect JSON | `colour` | `true` or `false` |
| preview reply | `X-Ocelli-Channels` | `1` or `3` |
| preview reply body | bytes | `width * height * channels` |
| error code | `colour-unavailable` | replaces `not-monochrome` |
| error code | `colour-not-applicable` | a window, invert, value or statistic asked of a colour frame |
| measure JSON | `statistics` | `"unavailable"` with a `reason` on a colour frame, under open question 6 |
| page sentence | preset note | "This image is colour, so it has no window to set." |
| page sentence | value readout | "This image is colour, so it has no modality value." |

Refusal sentences for `colour-unavailable`, one per reason: the JPEG 2000 or HTJ2K decoder in this build reads one sample per pixel, colour samples above 8 bits, a palette entry above 255 under question 8, an ICC profile under question 5, a segmented or missing palette table, a photometric value this build does not know.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `POST /preview`, `/file` | `decode_held`, then `render_held` with the colour recipe | Decoder report read once per held frame | Three bytes per pixel, `X-Ocelli-Channels: 3` |
| Colour space and layout | `ColorTransform::resolve` | Inputs from the data set, `DispatchPath` and the registry | One resolved transform per held frame |
| Byte rounding | `ocelli_render::colour_bytes_into` | Tier C's `byte_from_channel` and refusals | Bytes, or `Unavailable` |
| Palette tables | `ocelli_dicom::palette_color_lut` | Top-level elements only | `PaletteColorLut` or a named refusal |
| Native frame length | `ocelli-codec` native route | 4:2:2 stores two samples | A native `YBR_FULL_422` frame decodes |
| `POST /value` | `value_json` | Colour has no stage 1 | `colour-not-applicable` |
| `POST /measure` | `measure_json` | Lengths, angles and area use calibration only | Lines and area, statistics unavailable |
| Folder scan | `inspect_json` | `colour` flag | Every frame listed |
| Thumbnails, prefetch, tiles | `requestGrey` and the page cache | Same reply | RGB thumbnails and tiles |
| F-209 window link | `copyWindow` | Unchanged, monochrome only | A colour target receives nothing |
| F-209 scroll and zoom link, reference lines | `planes` in inspect JSON | Photometric independent | Unchanged |
| Presets, W/L drag, invert | page tool state | Off on colour with a sentence | No request carries a window or invert for colour |
| F-213 presets and region window, pending | F-213 | Must keep the colour gate | Recorded for F-213's design |

The rounding owner is `ocelli-render`. The selection owner is `ColorTransform::resolve`. The folder viewer owns only which reason sentence a refusal shows.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Folder scan | An RGB file of 3 frames | Three images listed, thumbnail RGB | A `YBR_RCT` JPEG 2000 file lists and its cell shows the named refusal |
| Viewport shows a CT, monochrome | Next series is RGB | Stage shows RGB, presets off, corner `Window not applicable` | The CT's window stays on the CT viewport record |
| Viewport shows RGB with window link on | Window dragged in a CT viewport | The CT window is copied to other monochrome viewports only | The RGB viewport's reply is unchanged |
| Held colour frame | `POST /preview` with `X-Ocelli-Window-Center` | 400 `colour-not-applicable` | The held frame and the cache entry stay |
| Held colour frame | `POST /preview` with `X-Ocelli-Invert: 1` | 400 `colour-not-applicable`, under question 4 | Same |
| Held colour frame | `POST /value` | 400 `colour-not-applicable` | No sample is read |
| Colour cell | Pointer moves | Readout shows the colour sentence | No request per move |
| Colour cell | Rectangle drawn | Area shown, statistics unavailable | Under question 6 |

Invariants across consumers: a colour frame never reaches `LutChain`, and a monochrome frame never reaches `ColorTransform`, as D-23 makes true by construction. `stored_at` (`value.rs:345`) reads one sample per pixel and is never called on a colour frame, because the held value is an error for every colour frame.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Held colour frame | `decode_held` under F-216's cache | File id and frame, as F-216 and F-224 key it | Admission to the `Lru` | A refused resolve holds the refusal in the recipe, so a retry gives the same refusal without a second decode |
| Colour reply buffer | `render_capped` under the grey gate | One reply | Reply written | Charged at `width * height * 3` from admission. A refused render charges nothing |
| Colour scratch | the decode workspace | The worker | Returned with the workspace | Counted in `workspace_bytes`. A band larger than any later frame is freed by the existing over-twice rule |
| Page cache entry | `rememberGrey` | `cacheKey` | `cache.set` | Byte length counts against `CACHE_BYTE_BUDGET` as it does today |

Cache validity. A colour reply depends on the file, the frame and nothing else, because no window and no invert reach it. The page's `cacheKey` (`page.html:2065` to `:2070`) writes a fixed `colour` token for a colour image in place of the window and invert tokens, so a window change in a linked viewport cannot create a second entry for the same picture. A refused window or invert request does not replace the last shown image. The held frame carries the palette, up to `3 * 65536` `f32` entries, and `charged_bytes` (`preview.rs:510`) adds its heap bytes.

## Numbered acceptance inventory

Hand-computed values below use the library's inverse at `color.rs:441` to `:446` in `f32` and the byte rule at `cpu.rs:1055` to `:1076`. The textbook BT.601 inverse gives the same bytes for every row here, so these rows do not distinguish the two matrices. The library's own fixture does, at a tolerance twenty times tighter than their 0.020028 divergence (`crates/ocelli-pixel/tests/color.rs:20` to `:24`).

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Native RGB 2 by 1, Planar Configuration 0, stored `10 20 30 200 100 50`. Reply `10 20 30 200 100 50`, `X-Ocelli-Channels: 3`. PS3.3 C.7.6.3.1.3 | `rgb_interleaved_is_copied_per_pixel` | A reply of one byte per pixel, or the R channel alone | implementation |
| AC2 | Native RGB 2 by 1, Planar Configuration 1, stored `10 200 20 100 30 50`. Reply `10 20 30 200 100 50`. PS3.3 C.7.6.3.1.3 | `rgb_planar_is_gathered_by_plane` | Reading planar as interleaved, which gives `10 200 20 100 30 50` | implementation |
| AC3 | Native YBR_FULL 1 by 1, `Y Cb Cr` `100 150 90`. Channels `46.72366 119.56544 138.98863`, reply `47 120 139`. Native YBR_FULL 1 by 1, `30 200 60`. Channels `-65.3378 53.7829 157.59158`, reply `0 54 158`. PS3.3 C.7.6.3.1.2 | `ybr_full_uses_the_library_inverse_and_the_tier_c_byte_rule` | No conversion, which gives `100 150 90`. A wrapping store, which keeps `-65` as 191 the way a `Uint8Array` assignment does | implementation |
| AC4 | Native YBR_FULL 2 by 1, Planar Configuration 1, stored `100 30 150 200 90 60`. Reply `47 120 139 0 54 158` | `ybr_full_planar_is_converted_after_gathering` | Converting before gathering planes | implementation |
| AC5 | Native YBR_FULL_422 2 by 1, stored `Y1 Y2 Cb Cr` `50 200 100 160`, Value Length 4. Reply `95 37 0 245 187 150`. PS3.3 C.7.6.3.1.2 replication | `ybr_full_422_native_shares_one_chroma_pair` | Sizing by Samples per Pixel, which refuses the 4-byte value today, or reading `Y Cb Cr` per pixel | implementation, needs question 1 |
| AC6 | Native YBR_PARTIAL_422 2 by 1, stored `16 235 128 128`. Reply `0 0 0 255 255 255`. A full-range matrix gives `16 16 16 235 235 235` | `ybr_partial_422_applies_the_studio_offset` | The full-range matrix on a partial-range frame | implementation |
| AC7 | PALETTE COLOR 5 by 1, 8-bit stored `9 10 12 13 99`, descriptors `4\10\16`, red `0 85 170 255`, green `255 170 85 0`, blue `10 20 30 40`. Reply `0 255 10 0 255 10 170 85 30 255 0 40 255 0 40`. PS3.3 C.7.6.3.1.5 first mapped input and clamping | `palette_maps_the_stored_value_from_the_first_mapped_input` | Ignoring the first mapped input, which gives `255 0 40` for stored 9, or windowing before the palette, which D-23 forbids | implementation |
| AC8 | RLE YBR_FULL 2 by 1 with Planar Configuration 1 declared, three literal segments `100 30`, `150 200`, `90 60`. Reply `47 120 139 0 54 158`. The decoder reports `Preserved` and `Interleaved` (`rle.rs:18` to `:20`), and encapsulated forces interleaved (`color.rs:265` to `:272`) | `rle_ybr_full_is_converted_once_and_planar_is_ignored` | Honouring the declared planar on decoder output, or skipping the conversion | implementation |
| AC9 | JPEG baseline wrapped from `crates/ocelli-codec/tests/fixtures/jpeg_baseline_rgb.jpg` with Photometric Interpretation `YBR_FULL_422`. Reply equals `Registry::decode` output byte for byte | `jpeg_rgb_output_is_not_converted_twice` | A second YBR conversion of decoder RGB, which darkens and shifts hue (`color.rs:210` to `:214`) | implementation |
| AC10 | A JPEG 2000 file with `YBR_RCT` and three samples, and one with `YBR_ICT`. Answer 400 `colour-unavailable` with the JPEG 2000 sentence. Under question 2 option B the rows become decoded fixtures instead | `ycc_from_jpeg_2000_is_refused_by_name` | Any image at all, or the full-range matrix applied to RCT | implementation |
| AC11 | Native RGB with Bits Stored 16 and one sample above 255, and the same with every sample at or below 255. Both refuse `colour-unavailable` with the bit-depth sentence, under question 8 | `colour_above_8_bits_is_refused_from_the_header` | The data-dependent rule, which draws the second file and refuses the first | implementation |
| AC12 | Window center and width, a preset, a window function, or invert sent for a colour frame. 400 `colour-not-applicable`, and a following plain request draws the same bytes as before | `a_window_or_invert_on_colour_is_refused_and_the_frame_survives` | Ignoring the header and answering 200, which is a reply that looks windowed and is not | implementation |
| AC13 | `POST /value` and `/measure` probe on a colour frame refuse `colour-not-applicable`. A rectangle on a colour frame reports its area and `statistics` unavailable. A length reports its millimetres | `colour_frames_have_no_modality_value_or_statistics` | Reading the R sample as a modality value through `stored_at` | implementation |
| AC14 | `inspect_json` of an RGB file of 3 frames has `"colour":true` and `"monochrome":false`. The page lists 3 images | `inspect_marks_colour_and_the_page_lists_every_frame` | A colour cine listed as one image | implementation |
| AC15 | Memory. A colour reply is charged `width * height * 3` under the grey gate, the colour scratch counts in `workspace_bytes`, and F-216's resident bound is re-measured with a colour series of the largest admitted frame and recorded beside F-216's figure | `a_colour_reply_charges_three_bytes_per_pixel`, plus the recorded measurement | Charging one byte per pixel, which lets three times the cap be resident | implementation, measured by the implementer |
| AC16 | In a browser through F-208's harness, a folder of the AC1, AC5 and AC7 files: each cell's canvas pixel at the centre of source pixel 0 reads the hand value, the thumbnail is not black, presets are off with the colour sentence, the readout shows the colour sentence, and a CT beside it still windows | `examples/folder-viewer/tests/browser/colour.mjs` | A canvas filled from the first channel only | integrated |
| AC17 | Before the fixtures are accepted, the reviewer checks PS3.3 C.7.6.3.1.2, C.7.6.3.1.3, C.7.6.3.1.5, C.7.6.3.1.6 and C.7.9 at dicom.nema.org, current edition, against the matrices and rules quoted above, and names the edition read | Review note | Fixtures citing text nobody checked | reviewer, before implementation |
| AC18 | Mutation. Swapping two inverse coefficients in `FULL_RANGE`, dropping the `+ 16` offset, or making the planar gather interleaved turns AC3, AC6 or AC2 red | Recorded in the review | A test that passes on the wrong code | implementation, reviewer |

Worked values for AC3, AC5 and AC6. For `100 150 90`, `Cb' = 22` and `Cr' = -38`. `R = 100 - 0.00003682 * 22 + 1.4019876 * -38 = 46.72366`, `G = 100 - 0.3441133 * 22 - 0.7141038 * -38 = 119.56544`, `B = 100 + 1.7719781 * 22 - 0.000134583 * -38 = 138.98863`. For the 4:2:2 pair, `Cb' = -28` and `Cr' = 32`. Pixel 1 is `50 + 1.4019876 * 32 + 0.0010310 = 94.86463`, `50 + 9.6351724 - 22.851322 = 36.78385` and `50 - 49.615387 - 0.0043067 = 0.38031`, which round to `95 37 0`. Pixel 2 adds 150 to each channel, `244.86464 186.78386 150.38031`, which round to `245 187 150`. For the partial pair, `Y' = 0` gives 0 and `Y' = 219` gives `1.1644155 * 219 = 255.007`, which clamps to 255.

## Approach

1. **ocelli-render.** Factor the accept, display-byte check, `map_into` and `rgba_from_rgb` steps of `raster_color_into` (`cpu.rs:363` to `:395`, `:586` to `:599`) into one private routine, call it from `raster_color_into`, and export `colour_bytes_into` writing three bytes per pixel over bands. The existing `raster_color_into` tests stay green unchanged.
2. **ocelli-dicom.** Add `palette_color_lut` in `frame_open.rs`, reading `(0028,1101)` to `(0028,1103)` and `(0028,1201)` to `(0028,1203)` into `LutDescriptor::new` and `PaletteColorLut::new`. Refuse `(0028,1221)` to `(0028,1223)` by name.
3. **ocelli-codec.** Under question 1, size the native route from the stored samples per pixel for the two 4:2:2 codes.
4. **preview.rs.** Accept the colour codes in `photometric` and `frame_parts`, build `SampleLayout` from the header, take the decoder's two reports in `decode_encapsulated` instead of refusing, and add a colour arm to `Recipe`. `value` and `full_range` are `colour-not-applicable` for that arm. `select_window` refuses every window input on it. Replace `NotMonochrome` with `ColourUnavailable` and add `ColourNotApplicable`. `inspect_json` writes `colour`.
5. **held.rs and serve.rs.** `grey_bytes` takes the channel count. `Grey` carries it and the header writer emits `X-Ocelli-Channels`. `workspace_bytes` counts the `Rgb` scratch. `charged_bytes` counts the palette.
6. **value.rs and measure.rs.** No change to `stored_at`. `measure_json` returns the area and an unavailable `statistics` on a colour frame under question 6.
7. **page.html.** One predicate for "this frame draws" beside `info.monochrome`, used at the listed lines. `fillGreyImage`, `prepareDisplay`, `paintGrey` and `paintThumbBitmap` read the channel count. `cacheKey` writes the colour token. The preset note, the readout and the fault sentence change. `copyWindow` stays monochrome only.

Rejected: calling `CpuRaster::raster_color_into` with an identity view, as the oracle does, because it carries a view, an index map, a device extent and a generation for a reply that is never resampled, and its scratch is a whole frame. Writing the byte rule in the folder viewer, which is a second copy at the boundary HLD 27.3 names. Converting in page JavaScript, which has no fixture runner and puts colour arithmetic outside the library.

## Dependencies, write ownership and resources

Declared dependencies F-030, F-209 and F-216, all done. F-212 now depends on this story, by the operator's order of 2026-11-07, and F-213, F-214 and F-215 follow F-212 as its plan records.

Writes `examples/folder-viewer/src/preview.rs`, `serve.rs`, `held.rs`, `measure.rs`, `page.html`, `use.html`, `examples/folder-viewer/tests/preview.rs`, `tests/value.rs`, `tests/measure.rs`, a new `tests/browser/colour.mjs`, `crates/ocelli-render/src/cpu.rs` and `lib.rs`, `crates/ocelli-dicom/src/frame_open.rs` and `lib.rs`, `crates/ocelli-codec/src/native.rs` and, under question 1, `registry.rs` and `jpeg.rs`, `tools/oracle/src/produce_current.rs` under question 7, `README.md`, and the LLD files below.

`preview.rs`, `serve.rs`, `held.rs` and `page.html` are also written by F-224, F-225, F-226, F-227 and F-228, whose worker worktrees exist today, and by F-212 to F-215 after this story. They cannot have concurrent writers. The safe order is after those five integrate, or a three-way merge at integration with this story rebased onto them. `frame_open.rs` is shared with F-225. No GPU is needed.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-folder-viewer`, `bin/ocelli.sh check ocelli-render`, `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh check ocelli-codec` while iterating, then the feature profile once at the integration tree. The sprint profile runs once for the sprint, per the operator's rule of 2026-11-06. AC16 runs under the F-208 browser harness. AC15's resident measurement names the host and the frame used. A later edit to any file in the write set invalidates the record. The oracle's colour rows stay not scored, so no comparison evidence is claimed.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The folder viewer's local HTTP reply carries display bytes, as it does today for grey.
- Render-loop allocation: n/a. The colour scratch is a workspace buffer reused across requests and counted in `workspace_bytes`.
- unsafe: none
- Tier A (WebGPU): n/a. The folder viewer page opens no device, as F-210 and F-212 record. The library's GPU colour path, `ColourLut` in `crates/ocelli-render/src/colour.rs`, is not used here.
- Tier B (WebGL2): n/a, for the same reason.
- Tier C (CPU): full for the drawn codes, through `ocelli-pixel`'s `ColorTransform` and `ocelli-render`'s CPU byte rule, on the host process whatever GPU the machine has. Every shape that rule refuses is refused by name, never drawn differently.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Interleaved and planar RGB, PS3.3 C.7.6.3.1.3 | `examples/folder-viewer/tests/preview.rs`, AC1, AC2 |
| fixture | YBR_FULL inverse and the byte rule, PS3.3 C.7.6.3.1.2 | same, AC3, AC4 |
| fixture | 4:2:2 replication and native length, PS3.3 C.7.6.3.1.2 | same, AC5, AC6 |
| fixture | Palette first mapped input and clamp, PS3.3 C.7.6.3.1.5 and C.7.9 | same, AC7 |
| fixture | Decoder report honoured once, PS3.5 8.2 and C.7.6.3.1.3 | same, AC8, AC9 |
| unit | `colour_bytes_into` agrees with `raster_color_into` at an identity grid | `crates/ocelli-render/src/cpu.rs` tests |
| unit | `palette_color_lut` refusals | `crates/ocelli-dicom` tests |
| integration | Refusals by name and preserved state | `tests/preview.rs`, `tests/value.rs`, `tests/measure.rs`, AC10 to AC14 |
| integration | Memory charge | `serve.rs` tests, AC15 |
| browser | The page draws colour and gates every scalar tool | `tests/browser/colour.mjs`, AC16 |

The fixtures are hand-computed from the matrices and rules quoted above, not read back from the implementation.

## Parity surface covered

None measured. `docs/hld/B-parity-surface.md` has no colour row. cornerstone3D 5.8.2 draws these images, and two of its choices differ from this plan: it applies the viewport window and invert to a colour image, and it shifts a 16-bit palette right by 8. Questions 3, 4 and 8 decide whether this story follows or reports unavailable. The oracle's colour candidates stay not scored because section 25.1 states no colour threshold.

## Deviations

None, provided the answers below keep every unsupported shape refused. Question 5 option B would need a new `D-NN` row, because it draws a frame the file says needs a profile without that profile.

## LLD impact

- `docs/lld/pixel-pipeline.md`: stage 4 has a production consumer, the folder viewer, and its inputs.
- `docs/lld/render-graph.md`: `colour_bytes_into` and the shared byte step.
- `docs/lld/codecs.md`: native 4:2:2 length, under question 1.
- `docs/lld/frame-open.md`: `palette_color_lut`.
- `docs/lld/oracle.md`: the palette reader, under question 7.

## User-facing documentation impact

- `README.md:309` to `:314`: the sentence "A colour frame ... stays on a fixed error sentence" is replaced. Colour frames draw. A colour image has no window, invert, value at the pointer or region statistics, and its corner reads `Window not applicable`. A colour image this build cannot draw shows a sentence naming why. Step numbers are measured at implementation.
- `examples/folder-viewer/src/use.html:638`: the stage 4 paragraph says the folder viewer draws colour through `ColorTransform` and `colour_bytes_into`, and the crate table at `use.html:215` names `ocelli-render`'s new function.
- `examples/folder-viewer/src/more.html`: no change. It has no colour card, and ICC remains library behaviour this page does not draw, which `use.html:638` already states.

## Open questions

1. **Native 4:2:2 length.** Affects AC5 and AC6. Options: A, the decoder reports its output length as a third registry query beside photometric and layout, the raw decoder reporting two samples per pixel for 4:2:2 and JPEG keeping three. B, the native route alone sizes from the stored samples. C, refuse native 4:2:2 in this story. Owner: operator. Recommendation: A, because the length is the decoder's fact in the same way its colour space is. Deferral keeps native `YBR_FULL_422` refused, which is safe and leaves the corpus row `synthetic/us_ybr_full_422.dcm` undrawn.
2. **YBR_ICT and YBR_RCT.** The operator listed both, and no decoder in this build reads a JPEG 2000 codestream with more than one component. Affects AC10. Options: A, this story draws them whenever a decoder reports `Rgb`, refuses them by name today, and a follow-up story gives the JPEG 2000 and HTJ2K adapters multi-component decoding that reports `Rgb`. B, widen this story to that decoding, which needs ritk-codecs and openjph-core's multi-component support measured first. Owner: operator. Recommendation: A. Deferral leaves both refused by name.
3. **Window on a colour frame.** Options: A, unavailable, as D-23 and the library's stage separation say. B, cornerstone3D's per-channel window, which would need a new library stage and a deviation. Owner: operator. Recommendation: A.
4. **Invert on a colour frame.** The library's inversion is stage 3, which refuses colour. Options: A, unavailable. B, `255 - byte` per channel after the colour stage, as cornerstone3D does. Owner: operator. Recommendation: A. Deferral keeps it unavailable.
5. **A file carrying an ICC Profile (0028,2000).** The library does not implement ICC (`ocelli-render/src/colour.rs:152` to `:155`). Options: A, refuse by name. B, draw without the profile and say so in the corner, which needs a deviation row. Owner: operator. Recommendation: A, with the consequence that many secondary capture and ultrasound files carrying an sRGB profile will be refused. Deferral is A.
6. **The measure reply on a colour region.** Options: A, area and lines measured, `statistics` unavailable with a reason. B, the whole region refused as today, losing the area. Owner: operator. Recommendation: A.
7. **The oracle's palette reader.** Options: A, `tools/oracle/src/produce_current.rs:578` to `:687` calls the new `ocelli_dicom::palette_color_lut`, so there is one reader, with the two synthetic palette rows' candidate bytes unchanged at the sprint corpus gate. B, leave the oracle's copy. Owner: sprint lead. Recommendation: A.
8. **Colour samples above 8 bits and palette entries above 255.** The tier C rule refuses a stored colour sample above 255 per draw (`cpu.rs:957` to `:964`) and a palette entry that is not a byte (`cpu.rs:122` to `:131`). The first is data-dependent, so two frames of one 16-bit series can differ in whether they draw. Options: A, the folder viewer refuses colour with Bits Stored above 8 from the header, and refuses a palette with an entry above 255, both by name. B, a library rule that scales them, for example cornerstone3D's right shift by 8 for a 16-bit palette, written once in `ocelli-render`. Owner: operator. Recommendation: A in this story and B as a follow-up, because real palettes usually carry 16-bit entries and A refuses them. Deferral is A.
