# F-241, DICOM video in the folder viewer and @ocelli/core

**Status**: draft
**Epic ref**: E4.11
**Sprint**: S47
**Estimate**: 12w

Story row, `docs/sprints/BACKLOG.md`: "DICOM video in the folder viewer and @ocelli/core: the MPEG-2, MPEG-4 AVC/H.264 and HEVC/H.265 transfer syntaxes known to the codec registry, the stream demuxed in Rust, YBR_PARTIAL_420 converted once in ocelli-pixel, each frame shown and stepped exactly by the stream's own sample table with window, invert, readout and statistics on the decoded frame, better than cornerstone3D 5.8.2 on measured axes, and every stream this build cannot decode refused by name". Layer All, depends on F-214, F-223, F-231 and F-239, milestone M20.

`allocation.json` notes: "Decided by the operator on 2026-11-07: file a video story now. Ocelli refuses YBR_PARTIAL_420 by name after F-239, and no decoder handles the video transfer syntaxes. cornerstone3D 5.8.2 is the floor and Ocelli must be better on a stated measured axis. The decode route, the patent position and MPEG-2 are open questions for the operator."

**Why E4.11 and not E4.9.** E4 is the codec epic, where F-229 took E4.10. E4.9 is free in the allocation but is the dependency F-145 declares as `E4.9 (UNRESOLVED)`, a recorded defect that `scripts/backlog_check.py` keeps declared on purpose. Taking E4.9 would silently resolve F-145 onto this story, which is the repointing the "Recorded defects" table exists to refuse. The milestone is M20, as F-229's is, because the consumer is the folder viewer.

**Why S47.** Every dependency is in S46, and `docs/sprints/SPRINT_PLAN.md` never places a story at or before the sprint holding what it depends on. F-239 and F-214 are still pending and the folder viewer chain integrates in order, so S46 cannot hold it. S47 holds 14w today, F-120 at 8w and four small stories, and this story takes it to 26w. If F-239 carries out of S46, this story moves with it, and the lead says so at S47's opening rather than starting it on an unmerged colour route.

This plan is a draft. Every path and line below was read on `sprint/s46` at 05d9aca2. The cornerstone3D citations are the shipped ESM under `tools/oracle/node_modules/@cornerstonejs/`, version 5.8.2 in each `package.json`. dwv, Horos and Grok were not opened.

## Normative source, transcribed

### HLD section 21, codec registry, `docs/hld/18-codec-registry.md`

```text
Explicit runtime registration, not the inventory crate - inventory does not work on WebAssembly, which is precisely why dicom-rs's own plugin registry is unavailable there. Explicit registration is also what lets a native build link C codecs the browser build cannot.

pub trait Decoder: Send + Sync {
    fn transfer_syntaxes(&self) -> &'static [&'static str];
    /// Decode one frame into `out`. Must not allocate.
    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8])
    -> Result<(), CodecError>;
}

JPEG-LS has no credible pure-Rust path, the registry design deliberately allows a JS-side bridge to @cornerstonejs/codec-charls as a registered decoder, so choosing that route costs an adapter rather than a redesign.
```

The HLD's dash in the first line is an em-dash in the source and the semicolon in the last line is the source's, both rewritten here only to satisfy `scripts/prose_check.py` over this file. `docs/hld/` holds the exact text.

### HLD sections 5 and 6, `docs/hld/04-boundary-and-data-path.md`

```text
5.2 Bulk - raw bytes into linear memory, downward
The core allocates and returns a pointer and length, JavaScript builds a typed-array view immediately before writing and discards it after.

Figure 2 - One hop disappears and one constraint disappears with it. Decoded frames stop becoming JavaScript objects, and a single GPUDevice replaces the pooled WebGL contexts cornerstone works around with an entire rendering-engine variant.
```

Decision D3, `docs/hld/11-decision-log.md:24`: "Pixels never cross the boundary | Decode in wasm, render in JS | The main architectural gain".

### HLD section 18, `docs/hld/15-lut-chain.md`

Stage 4 is "Palette / ICC, Display to RGB", implemented once in `ocelli-pixel`. D-23 keeps a three-sample frame out of stages 1 to 3, and D-29 windows and inverts colour per channel after stage 4, following cornerstone3D 5.8.2.

### Appendix B, `docs/hld/B-parity-surface.md:24`

"Viewport types | 12 | STACK, ORTHOGRAPHIC, PERSPECTIVE, VOLUME_3D, PLANAR, VIDEO, WHOLE_SLIDE, ECG and _NEXT variants".

### DICOM PS3.6 2026d, Table A-1, read at dicom.nema.org on 2026-11-07

| UID | Name |
|-----|------|
| 1.2.840.10008.1.2.4.100 | MPEG2 Main Profile / Main Level |
| 1.2.840.10008.1.2.4.100.1 | Fragmentable MPEG2 Main Profile / Main Level |
| 1.2.840.10008.1.2.4.101 | MPEG2 Main Profile / High Level |
| 1.2.840.10008.1.2.4.101.1 | Fragmentable MPEG2 Main Profile / High Level |
| 1.2.840.10008.1.2.4.102 | MPEG-4 AVC/H.264 High Profile / Level 4.1 |
| 1.2.840.10008.1.2.4.102.1 | Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.1 |
| 1.2.840.10008.1.2.4.103 | MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1 |
| 1.2.840.10008.1.2.4.103.1 | Fragmentable MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1 |
| 1.2.840.10008.1.2.4.104 | MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video |
| 1.2.840.10008.1.2.4.104.1 | Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video |
| 1.2.840.10008.1.2.4.105 | MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video |
| 1.2.840.10008.1.2.4.105.1 | Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video |
| 1.2.840.10008.1.2.4.106 | MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2 |
| 1.2.840.10008.1.2.4.106.1 | Fragmentable MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2 |
| 1.2.840.10008.1.2.4.107 | HEVC/H.265 Main Profile / Level 5.1 |
| 1.2.840.10008.1.2.4.108 | HEVC/H.265 Main 10 Profile / Level 5.1 |

Sixteen UIDs. HEVC has no fragmentable form. Table A-1 also lists `1.2.840.10008.1.2.7.1` to `.7.3`, SMPTE ST 2110-20 video and 2110-30 audio, which belong to DICOM Real-Time Video, PS3.22, and are not stored in a Part 10 file. They are out of scope.

### DICOM PS3.5 2026d, sections 8.2.5 to 8.2.12, read at dicom.nema.org on 2026-11-07

Section titles: 8.2.5 MPEG2 MP@ML, 8.2.6 MPEG2 MP@HL, 8.2.7 H.264 High Profile Level 4.1, 8.2.8 H.264 High Profile Level 4.2, 8.2.9 H.264 Stereo High Profile Level 4.2, 8.2.10 HEVC Main Profile Level 5.1, 8.2.11 HEVC Main 10 Profile Level 5.1, 8.2.12 constraints for audio in AVC and HEVC bit streams. As read, through the site's own summary, and to be checked against the text by AC20:

```text
8.2.5   Photometric Interpretation YBR_PARTIAL_420 for multi-component,
        MONOCHROME2 for single component. Bits Allocated 8, Bits Stored 8,
        High Bit 7, Planar Configuration 0. Non-fragmentable: one Fragment
        shall contain the whole MPEG2 stream. Fragmentable: the stream may be
        segmented into multiple Fragments. Basic Offset Table empty. Rows,
        Columns, Cine Rate (0018,0040) and Frame Time (0018,1063) or Frame
        Time Vector (0018,1065) shall be consistent. 720 by 480 or 720 by 576.
8.2.7   The container format for the video bit stream shall be MPEG-2
        Transport Stream, a.k.a. MPEG-TS, or MPEG-4, a.k.a. MP4 container.
        Photometric Interpretation shall be YBR_PARTIAL_420. Bits Allocated 8,
        Bits Stored 8. At most 1920 by 1080. Interlace at the maximum
        resolution is supported at a field rate of 50 Hz or 60 Hz. At most
        2^31 - 1 frames. Audio follows 8.2.12.
8.2.8   "When decompressing, should the characteristics explicitly specified
        in the compressed data stream be inconsistent with those specified in
        the DICOM Data Elements, those explicitly specified in the compressed
        data stream should be used to control the decompression."
8.2.11  Photometric Interpretation YBR_PARTIAL_420. Bits Allocated 16,
        Bits Stored 10. Container MPEG-TS or MP4. The same precedence sentence.
```

### DICOM PS3.3 2026d, C.7.6.3.1.2, YBR_PARTIAL_420, read at dicom.nema.org on 2026-11-07

```text
Pixel data represent a color image described by one luminance (Y) and two
chrominance planes (CB and CR). This photometric interpretation may be used
only when Samples per Pixel (0028,0002) has a Value of 3. The CB and CR values
are sampled horizontally and vertically at half the Y rate and as a result
there are four times less CB and CR values than Y values. Planar Configuration
(0028,0006) shall be 0. Shall only be used for pixel data in an Encapsulated
(compressed) format.

Y  = + .2568R + .5041G + .0979B + 16
CB = - .1482R - .2910G + .4392B + 128
CR = + .4392R - .3678G - .0714B + 128

This Photometric Interpretation is primarily used with MPEG compressed bit
streams.
```

These are the same forward equations `crates/ocelli-pixel/src/color.rs:448` inverts for `YBR_PARTIAL_422`, and the inverse there is computed once in `f64` from them.

### What the HLD does not give

No HLD section mentions video, MPEG, H.264, HEVC or `YBR_PARTIAL_420`. `grep -n -i 'video\|mpeg\|h\.264\|hevc' docs/hld/*.md` finds only Appendix B's VIDEO viewport type. Everything below about streams, containers and decode routes is the lead's.

## What the specification does not cover

1. **A stream is not a frame.** HLD 21's `Decoder` decodes one frame from its own bytes and must not allocate. A video frame depends on earlier frames of its group of pictures, so no frame of a video stream can be decoded alone. The registry gains the sixteen UIDs as known syntaxes, and a separate stream interface beside `Decoder`, `VideoStream`, owns the demuxed sample table and a decode session. `Registry::capability` reports a video syntax as available only through that interface, so a caller of `decode_frame` gets a named refusal rather than a wrong frame.
2. **Extraction.** The stream is the concatenation of every fragment of Pixel Data after the empty Basic Offset Table, one fragment for the non-fragmentable syntaxes and any number for the `.1` forms. `ocelli-dicom` returns the concatenated bytes.
3. **Containers.** By PS3.5 8.2.7 an H.264 or HEVC stream is in MP4 or MPEG-TS. 8.2.5, as read, names no container for MPEG-2, which the lead takes to be an MPEG-2 elementary stream or a transport stream, checked by AC20. Real files also carry a bare Annex B elementary stream, which the lead has seen and not measured. A demuxer in Rust, in `ocelli-codec`, reads the MP4 sample table (`moov`, `trak`, `stsd` with `avcC` or `hvcC`, `stts`, `ctts`, `stss`, `stsz`, `stsc`, `stco` or `co64`), the MPEG-TS PAT, PMT and PES timestamps, and an Annex B stream by its start codes and slice types. Its output is one table, a sample per displayed frame with its byte range, its presentation time, its decode order and whether it is a random access point. No container crate is added.
4. **Which frame is frame N.** Frame N is the N-th sample in presentation order of that table. It is never derived from a time multiplied by a frame rate. Number of Frames `(0028,0008)` that disagrees with the table is reported, and the table wins by PS3.5 8.2.8's precedence sentence.
5. **Timing.** Cine plays each sample at its own presentation time from the stream. Frame Time, Frame Time Vector and Cine Rate are read through F-192's `read_timing` and compared with the stream, and a disagreement is reported on the page, without changing which frame is shown.
6. **Colour.** PS3.3 C.7.6.3.1.2 gives one studio-range matrix, the BT.601 form of the `YBR_PARTIAL_422` equations. An HD H.264 stream usually signals BT.709 matrix coefficients in its VUI, and PS3.5 8.2.8 says the stream's characteristics should control decompression. These give different pixels for the same file, and neither choice is visible in a screenshot. This is open question 5. The lead's default, until answered, is the stream's colour description when it is present and PS3.3's equations when it is absent or unspecified, with the choice recorded per series and shown in the inspector.
7. **Chroma upsampling.** PS3.3 says only that chroma is sampled at half rate in both directions. The conversion replicates each chroma sample across its 2 by 2 block, as F-030 replicates 4:2:2 chroma across its pair, so the arithmetic is exact and hand-computable. A browser's `<video>` path filters chroma, and the difference is measured, AC16, and recorded under D14, never scored.
8. **Bit depth.** HEVC Main 10 gives 10-bit samples in 16-bit words. F-239's display scaling rule, shift by `n - 8` with `n` Bits Stored, applies after the conversion, so a 10-bit frame is scaled by 10.
9. **Stereo and 3D syntaxes.** `.105` carries frame-packed 3D in a 2D frame, which is drawn as stored with a sentence saying it is frame-packed. `.106` carries a base view any High Profile decoder decodes and a second view it does not, so the base view is drawn with a sentence naming the missing view.
10. **Audio.** PS3.5 8.2.12 lets a stream carry audio. Audio is not played, and the page says the file has an audio track. cornerstone3D 5.8.2 mutes by default and has no other audio handling.
11. **Interlace.** An interlaced H.264 stream at 1080 lines is decoded to frames as the decoder outputs them, and the page names it interlaced. No deinterlacing is applied.
12. **Memory.** A decoded 1920 by 1080 4:2:0 8-bit frame is 3,110,400 bytes, so a two-minute clip at 30 frames per second is about 11 GB decoded. Decoded frames are held in F-216's bounded cache and F-231's session cache under their budgets, keyed by F-224's content id with the sample index. A frame evicted is decoded again from the nearest random access point before it, so seek cost is bounded by the group of pictures length, which AC24 measures.
13. **Patents are outside `docs/SOURCE-POLICY.md`.** The policy governs copyright, what may be read and what may be depended on, and its "What a dependency is" section excludes host tools such as ffmpeg that are never redistributed. It says nothing about patents. H.264 and HEVC are covered by patent pools and independent holders, and MPEG-2's pool patents have, by the lead's understanding, expired in the jurisdictions the pool listed. Whether a shipped artefact may contain a software H.264 or HEVC decoder is an operator decision, open question 3, and this plan's recommendation is chosen so that it does not need one. The lead gives no legal opinion here.

## Problem and reproducer

A DICOM file with transfer syntax `1.2.840.10008.1.2.4.102` and Photometric Interpretation `YBR_PARTIAL_420`, opened in the folder viewer today, stops at `photometric_of` in `crates/ocelli-dicom/src/frame_open.rs:451` to `:468`, which returns `FrameOpenError::Photometric` for any code but the nine it names, and the page shows "an image attribute is missing or not usable". After F-239 the same file is refused by name, F-239 plan item 3 and its floor table row "YBR_PARTIAL_420 ... Refused by name ... The gap closes with a video decoder story". The transfer syntax itself is not in `KNOWN_TRANSFER_SYNTAXES`, `crates/ocelli-codec/src/registry.rs:21` to `:36`, so `Registry::capability` reports it unknown rather than known and unavailable. `@ocelli/core` has no video route, and the shipped wasm module does not reach `ocelli-codec` at all, `docs/lld/codecs.md:18` to `:19`.

## What cornerstone3D 5.8.2 does, read from its code

Every row was read in the shipped ESM. Lines marked inference are the lead's reading, not code observed.

- **It never decodes video.** `dicom-image-loader/dist/esm/imageLoader/decodeImageFrame.js:24` to `:61` has no MPEG, H.264 or HEVC case and rejects with "No decoder for transfer syntax ..." at `:61`, and the worker does the same at `decodeImageFrameWorker.js:277`. `shared/decoders/` holds no video decoder.
- **It hands a URL to an HTML `<video>` element.** The legacy `core/dist/esm/RenderingEngine/VideoViewport.js:172` to `:176` creates a `<video>`, never attached to the page, with `muted`, `loop`, `autoplay` and `crossOrigin = 'anonymous'`, and `renderFrame` paints it into the viewport canvas with `ctx.drawImage` at `:122`. The `VIDEO_NEXT` path, `GenericViewport/Video/HtmlVideoRenderPath.js:8` to `:25`, appends a real `<video>` element to the page instead.
- **The URL is the server's `/rendered` resource.** `core/dist/esm/utilities/VideoUtilities.js:60` to `:80` takes `IMAGE_URL.rendered`, or rewrites a `wadors:` image id containing `/frames/` to `/rendered`, and `loadVideoStreamMetadata` throws "Video Image ID ... does not have a rendered video view" when it has neither, `:43` to `:45`. The wadouri metadata provider has no `IMAGE_URL` module. Inference: a local file or a `wadouri:` video fails unless the application registers its own URL, and MPEG-2 plays only when the server transcodes it, because no browser plays MPEG-2 in a `<video>` element.
- **Which syntaxes count as video.** `metadata/dist/esm/utilities/isVideoTransferSyntax.js:2` to `:19`, the same sixteen UIDs as PS3.6. `isVideoInstance` also treats Secondary Capture with 90 or more frames as video.
- **Frame identity is time times rate.** `setFrameNumber(frame)` sets `currentTime` to `(frame - 1) / this.fps`, `VideoViewport.js:351` to `:353`, and the current frame is `1 + Math.round(currentTime * this.fps)`, `:630` to `:635`. `fps` comes from Cine Rate, Recommended Display Frame Rate or `1000 / Frame Time`, `VideoUtilities.js:54` to `:56`, never from the stream. The new path seeks only when the drift exceeds half a frame, `HtmlVideoRenderPath.js:82` to `:86`.
- **Cine assumes 24 frames per second.** `tools/dist/esm/utilities/cine/playClip.js:258` to `:285` plays a video by `setPlaybackRate(fps / 24)`.
- **Window is a CSS filter, and not a window.** `setVOI` sets `canvas.style.filter` to an SVG `feColorMatrix` built by `Viewport.js:116` to `:139`, scale `(upper - lower + 1) / 255` and offset `lower / 255`, in `linearRGB`. `setWindowLevel(ww = 256, wc = 128)` gives `lower = wc - ww / 2`, `upper = wc + ww / 2 - 1`, `VideoViewport.js:514` to `:519`. A LINEAR window scales by about `255 / width` and offsets by about `-lower / width`, so this matrix narrows the output where a window widens it. The `VIDEO_NEXT` path stores `voiRange` and `invert` and nothing in its render path reads them, inference from a search.
- **No invert, rotation or flip on the legacy path.** `VideoViewportProperties` has no `invert` field, and `getRotation` returns 0 at `:99`.
- **Pixel readback is the browser's RGBA.** `getScalarData` draws the current video frame into a temporary canvas and reads `getImageData`, `:437` to `:447`, cached by the frame number computed above. ROI statistics use it. Inference: the readback is browser-converted and colour-managed RGBA, and the CSS window is not applied to it.
- **Annotations and segmentation.** Annotations reference `/frames/N`, and a labelmap draws on video through `CanvasActor`, `tools/dist/esm/stateManagement/segmentation/helpers/getViewportLabelmapRenderMode.js:40` to `:45`.
- **Audio.** Muted by default, nothing else.
- **YBR_PARTIAL_420 in the image loader.** `convertColorSpace.js:23` to `:26` copies it as RGB on the decoder's word, which never applies, because no decoder produces it.

## Routes, weighed

Three routes, and one mixed. Each is weighed against D2, D3, D5, HLD 6, the size budget, the native folder viewer and the source policy.

| Route | Library in the browser | Folder viewer | D3 and HLD 6 | Size | Source policy and patents | Availability |
|-------|------------------------|---------------|--------------|------|---------------------------|--------------|
| A. Browser decoder, WebCodecs `VideoDecoder` in a worker, frames copied into Ocelli's memory | Decodes in the core worker, planes copied into linear memory by channel 2's pattern | The page decodes in a page worker and posts the planes to the local process, which holds them like any decoded frame | No pixel reaches the main thread. A `VideoFrame` is a JavaScript object holding decoded pixels for the length of one `copyTo`, which HLD 6 says should not exist. A deviation | `web-sys` bindings and the Rust demuxer in the shipped module, measured, AC21 | No dependency. The browser vendor's decoder and the vendor's licence | H.264 in Chrome, Edge, Safari and Firefox. HEVC where the platform decodes it. **MPEG-2 nowhere**, because the WebCodecs codec registry has no MPEG-2 entry |
| B. A Rust decoder | Compiled into wasm | Compiled into the local process | Satisfied fully | Unknown and large. No H.264 decoder was found ready: `oxideav-h264` 0.1.11 says "no decode/encode functionality yet" on crates.io. `rust_h265` 0.4.1, MIT OR Apache-2.0, first published 2026-04-22, claims HEVC Main and Main 10. `openh264` 0.9.9 binds Cisco's C library, BSD-2-Clause, which the wasm build would compile from C | A software H.264 or HEVC decoder in a published npm module and binary, the case question 3 asks about. Each crate needs a SOURCE-POLICY row | Every tier and every browser, MPEG-2 only if someone writes it |
| C. The platform decoder | Not applicable | VideoToolbox on macOS, Media Foundation on Windows, VA-API or nothing on Linux | Satisfied | Native only | The OS vendor's licence. FFI is `unsafe` outside the two permitted files, and a VideoToolbox binding brings `objc2`, which F-238 removes from the shipped native build | Three decoders with three behaviours, and no Linux guarantee |
| D. cornerstone3D's route, a `<video>` element on the main thread | Main thread plays a URL | The local process serves the stream as a URL | Pixels never enter the core, so window, invert, readout and statistics are not Ocelli's arithmetic | Nothing | No dependency | As A, and frame identity is the browser's `currentTime` |

**The lead recommends A for both targets**, questions 1 and 2. It is the only route that ships no patent-encumbered decoder, adds no dependency, keeps every pixel operation in `ocelli-pixel` once, and exists on every browser Ocelli supports for H.264. Its cost is one deviation from HLD 6 and the per-frame copy, which AC22 to AC25 measure against cornerstone. B is the long-term route if the operator wants MPEG-2 or a decoder independent of the browser, and is a later story. C contradicts F-238 and the `unsafe` rule. D is the floor, and matching it alone would not be better on any axis.

## Scope, API and compatibility

In scope, on the recommended route:

- `ocelli-codec`: the sixteen UIDs join `KNOWN_TRANSFER_SYNTAXES`, a new `video` module with `VideoContainer` (`Mp4`, `MpegTs`, `AnnexB`, `Mpeg2Es`), `demux(&[u8], VideoCodec) -> Result<SampleTable, VideoError>`, and `SampleTable` with `samples()`, each `Sample { range, pts, decode_index, random_access }`, `codec_config()` giving the WebCodecs codec string and description record (`avcC` or `hvcC`), coded size, colour description and whether audio or interlace is present. `Registry::capability` reports the video syntaxes through `Capability::Stream`, and `decode_frame` refuses them with `CodecError::StreamSyntax`.
- `ocelli-dicom`: `video_stream(metadata, pixel_data) -> Result<VideoStreamBytes, FrameOpenError>`, the concatenated fragments, and `YBR_PARTIAL_420` accepted by `photometric_of` for the sixteen video syntaxes only, with every other syntax refusing it by name as F-239 does.
- `ocelli-pixel`: `ColorSpace::YbrPartial420`, the 4:2:0 replication and the PS3.3 inverse already in `color.rs`, plus the BT.709 studio-range inverse if question 5 is answered as the lead recommends, each computed once in `f64` and rounded once to `f32`. The input is three planes, `I420`, `NV12` or `I420P10`, the formats WebCodecs reports, and any other format is refused by name.
- `ocelli-wasm`: a worker-side bridge that configures `VideoDecoder` from `codec_config`, checks `VideoDecoder.isConfigSupported` first and reports unavailable when it is false, feeds `EncodedVideoChunk`s in decode order from a random access point, and copies each output `VideoFrame`'s planes into a buffer the core allocated, closing the frame at once. `wasm-bindgen` stays in this crate only.
- `@ocelli/core`: a video source on the stack viewport, `loadVideo(instance)` or a DICOMweb instance, with `play`, `pause`, `setPlaybackRate`, `setLoop`, `setFrameRange`, `setFrame(n)` and the existing frame events through the ring. A DICOMweb source may also take the server's `/rendered` resource when the bulk stream is unavailable, which is cornerstone's only route, question 4.
- Folder viewer: a page worker running the same decode, posting planes to a new `/video-frame` route of the local process, which holds the converted frame under F-216's cache and serves `/preview`, `/value` and `/measure` on it as on any frame. Cine plays the held frames at the stream's presentation times. The inspector names the container, codec, profile, colour description, audio, interlace and any disagreement between the stream and the DICOM attributes.
- Refusals by name: MPEG-2 unless question 4 says otherwise, HEVC where `isConfigSupported` is false, an unsupported container, a sample table that does not parse, a decoded format outside the three planes, and a coded size above F-239's `MAX_PIXELS`.

Non-goals: audio playback, deinterlacing, the second view of `.106`, segmentation drawn on a video frame, SMPTE ST 2110 and DICOM-RTV, encoding, Secondary Capture treated as video by frame count, which cornerstone does and PS3.6 does not, a Rust decoder, and a GPU colour conversion on tiers A and B.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Folder viewer scan and inspect | `ocelli_dicom::video_stream`, `ocelli_codec::video::demux` | Once per file | Codec, container, sample count, colour description, or a named refusal |
| Folder viewer page worker | WebCodecs `VideoDecoder` | Decode order from a random access point | Planes posted to `/video-frame` |
| Local process held frame | `ColorTransform` with `YbrPartial420`, F-239's `DisplayScale` and `ColourWindow` | Once per held frame, window per draw | Display bytes, R, G and B readout and statistics |
| Library worker | `ocelli-wasm` bridge, same demux and same `ColorTransform` | One decoder per stream per worker, D5 | Frames committed through F-231's session cache |
| `@ocelli/core` video source | The bridge, through commands down and events up | Exact frame by sample index | `frame` events with the sample index |
| Registry callers of `decode_frame` | `Registry::capability` | Unchanged for the fifteen image syntaxes | `CodecError::StreamSyntax` for a video syntax |
| Oracle | `tools/oracle` | Headless Chromium has no H.264 decoder | No golden row, declared in `tools/oracle/unsupported.json` |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Folder scan | A video file | Listed as video with its sample count | A demux failure lists it with the named reason |
| Opened, nothing decoded | `setFrame(n)` | Decode from the random access point at or before `n` to `n`, sample `n` shown | Out of range is refused, the shown frame is kept |
| Playing | Frame `n + 1` not yet decoded at its presentation time | The frame stays on `n` and a late count increments | No frame is skipped silently, the count is what AC23 reports |
| Playing | `pause` then `setFrame(m)` | Sample `m` shown, decoder flushed and reset to `m`'s random access point if `m` is outside the decoded window | A stale decode output for the previous position is dropped by its generation |
| Any | Decoder error event | Video unavailable with the decoder's reason | The last shown frame stays, a retry reconfigures once |
| Any | Window drag | F-239's colour window on the held converted frame | No decode |

Invariant: frame `n` shown, read out, measured and annotated is the same sample `n` of the table.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| `VideoDecoder` | The worker bridge | One per open stream, with a generation | `configure` accepted | Closed on release, error or reconfigure. A second `setFrame` to the same `n` returns the held frame without decoding |
| `VideoFrame` | The decoder's output callback | One output | `copyTo` resolved into the core's buffer | Closed in a `finally`, so an exception never leaks a frame |
| Plane buffer | The core, channel 2 | Allocation, with a generation | The core accepts the planes for sample `n` and generation `g` | A buffer for a stale generation is released unused |
| Held converted frame | F-216's cache, F-231's cache | Content id and sample index | Inserted | Evicted by budget and decoded again on demand |

## Numbered acceptance inventory

Hand-computed values use PS3.3 C.7.6.3.1.2's forward equations inverted in `f64`, the 2 by 2 chroma replication of item 7, and the tier C byte rule, then F-239's colour window. The synthetic clips are written by `scripts/corpus_synth.py` with ffmpeg, a host tool under `docs/SOURCE-POLICY.md`'s "What a dependency is", run and never read, and each frame `n` carries `n` as a 16-bit code in eight 16 by 16 blocks, so a frame's identity can be read back from its pixels.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The sixteen UIDs are known, report `Capability::Stream`, and `decode_frame` refuses each with `CodecError::StreamSyntax`. The fifteen image syntaxes are unchanged | `crates/ocelli-codec/tests/registry.rs` | A video syntax decoded as one frame, or left unknown | implementation |
| AC2 | Extraction: one fragment, and a `.1` stream in three fragments, give the same bytes | `crates/ocelli-dicom/tests/video_stream.rs` | Dropping the last fragment, or keeping the empty offset table | implementation |
| AC3 | MP4 demux of a synthetic 64 by 64 H.264 clip of 60 frames with B-frames and a group of 12: 60 samples, presentation order differs from decode order exactly as the `ctts` box says, random access at 0, 12, 24, 36 and 48 | `crates/ocelli-codec/tests/video_demux.rs` | Taking decode order as display order, or `stss` as zero-based | implementation |
| AC4 | MPEG-TS and Annex B demux of the same clip give the same sample table as AC3 | same | A PES timestamp wrap or a missing start code | implementation |
| AC5 | MPEG-2 elementary stream demux gives its table, and the folder viewer refuses decode by name unless question 4 says otherwise | same, and `examples/folder-viewer/tests/video.rs` | MPEG-2 silently drawn as grey or refused as unknown | implementation |
| AC6 | YBR_PARTIAL_420, one 2 by 2 block, Y `16 235 16 235`, Cb `128`, Cr `128`. Reply `0 0 0`, `255 255 255`, `0 0 0`, `255 255 255` | `crates/ocelli-pixel/tests/color.rs`, PS3.3 C.7.6.3.1.2 | A full-range matrix, `16 235` as grey levels | implementation |
| AC7 | YBR_PARTIAL_420 colour: Y `81`, Cb `90`, Cr `240` across one block. The implementer computes R, G and B from the inverse of the quoted equations by hand, records the working, and the reply matches it in every pixel of the block | same | Chroma read at a 4:2:2 index, which gives the next block's chroma to the second row | implementation |
| AC8 | `I420`, `NV12` and `I420P10` of one 4 by 4 frame give the same converted values, the 10-bit form after F-239's shift by 2 | same | NV12's interleaved chroma read as two planes | implementation |
| AC9 | Colour description, by question 5. A BT.709 stream and a BT.601 stream of the same planes give the hand-computed values of their own matrix, and an unspecified stream gives PS3.3's | same | One matrix for every stream | implementation, needs question 5 |
| AC10 | Frame identity, folder viewer. For each synthetic clip, `setFrame(n)` for every `n` in a shuffled order shows a frame whose embedded code is `n`, all `N` of `N` | `tests/browser/video.mjs` | A frame from `currentTime * fps`, which is wrong whenever Cine Rate and the stream disagree | integrated, Chrome |
| AC11 | Frame identity, library. The same through `@ocelli/core` in its worker | `packages/core` browser test | Same | integrated, Chrome |
| AC12 | Decode independence. For each synthetic 8-bit clip, the decoded planes of every frame equal ffmpeg's decoded planes byte for byte. H.264 and HEVC define decoder output exactly, so a conforming decoder has no tolerance. A mismatch is investigated, never tolerated | Recorded comparison in `docs/lld/codecs.md` | A copy at the wrong stride or a plane swapped, which no self-consistent test sees | integrated, reviewer |
| AC13 | Cine. A 25 per second clip plays each sample at its presentation time, measured by the page's presentation callback over 10 seconds, and a stream whose Cine Rate disagrees plays at the stream's rate with the disagreement named | `tests/browser/video.mjs` | Playing at Cine Rate, or at cornerstone's `fps / 24` | integrated |
| AC14 | Window, invert, readout and region statistics on a video frame use F-239's rules on the converted values. A window of width 128 and centre 128 on the AC7 block gives the hand-computed LINEAR bytes per channel | `examples/folder-viewer/tests/video.rs` | The CSS matrix cornerstone uses, scale `(upper - lower + 1) / 255` | implementation |
| AC15 | Refusals by name: HEVC where `isConfigSupported` is false, an unknown container, a bad sample table, a format outside the three planes, a coded size above `MAX_PIXELS`, and `.106`'s second view. Each shows its sentence and keeps the last frame | `tests/video.rs`, `tests/browser/video.mjs` | A blank frame, or a different picture on another browser | implementation, integrated |
| AC16 | Measured divergence from the browser's own colour. The same frame drawn by Chrome's `<video>` element and read back through a canvas, as cornerstone reads it, against Ocelli's converted frame. Largest and mean per-channel difference recorded, under D14, never scored | Recorded in `docs/lld/pixel-pipeline.md` | A claim of agreement nobody measured | integrated |
| AC17 | Tier rows. Tier C converts on the CPU. Tiers A and B draw the converted frame as any colour frame and convert nothing on the GPU, so no tier differs | `crates/ocelli-render` tests | A GPU conversion that differs from tier C | implementation |
| AC18 | Mutation. Swapping Cb and Cr, reading chroma at a 4:2:2 index, and ordering by decode index turn AC7, AC7 and AC10 red | Recorded in the review | Tests that pass on wrong code | implementation, reviewer |
| AC19 | Unchanged consumers. Every F-239 colour fixture and every monochrome fixture produce the same bytes | existing suites | A shared colour path perturbed | implementation |
| AC20 | The reviewer checks every transcription above against the published PS3.3, PS3.5 and PS3.6 text, the MP4 boxes against ISO/IEC 14496-12, MPEG-TS against ISO/IEC 13818-1, and Annex B against ITU-T H.264 and H.265, and names the editions read, before fixtures are accepted | Review note | Fixtures and code that share one wrong transcription | reviewer |
| AC21 | Wasm size. `bin/ocelli.sh gate wasm` passes on the tip with `ci/wasm-size-budget.json` and `ci/wasm-decode-size.json` not edited and `--accept-size` never run. Both modules are measured on base and tip with `bin/ocelli.sh wasm` and `bin/ocelli.sh wasm-decode`, and both deltas are recorded with `twiggy top` or an equivalent listing by symbol. The bridge and demuxer live in the render-worker module only, the decode-worker module gains nothing, and `cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none` on the tip names the one new edge, to `ocelli-codec`'s `video` module. The 5% tolerance of the render budget is about 107,560 bytes of 2,151,196. A delta beyond it stops the story for an operator decision rather than a re-baseline | Recorded in `docs/lld/build-targets.md` | A budget raised to make room, or video in the decode module | implementation |
| AC22 | No regression. `bin/ocelli.sh bench viewer.open viewer.first_image viewer.window_step viewer.scroll_rate viewer.resident_memory viewer.colour_frame` on the F-216 host, base and tip, every subject within F-216's thresholds and F-239's recorded noise, both medians recorded side by side | Recorded in `docs/lld/benchmarks.md` | A video check on every image frame | external, sprint verification |
| AC23 | Video measured. A new `tools/bench` subject `viewer.video_playback`, registered in `tools/bench/subjects.json` with no threshold, runs on the F-216 host with the release build on a 1920 by 1080 H.264 High 4.1 clip of 300 frames at 30 per second with a group of 30, and a 720 by 576 clip at 25 per second. It records time to the first frame from the file post, sustained presented frames per second during 10 seconds of cine against the stream's rate, late frames, the median and 95th percentile of 50 random `setFrame` calls, and peak `ps -o rss=` sampled every 20 ms as `viewer.resident_memory` does | Recorded in `docs/lld/benchmarks.md` | A video path measured against nothing | external, sprint verification |
| AC24 | Seek bound. The 95th percentile of AC23's random seek is no worse than decoding one group of pictures from its random access point, measured on the same host as the median time to decode 30 frames | same | A seek that decodes from frame 0 | external, sprint verification |
| AC25 | Better than cornerstone3D, measured. F-223's harness, `bin/ocelli.sh bench --compare-cornerstone --pseudo <dir>`, gains a video mode in which its bare cornerstone3D page plays the same clips through `VideoViewport` from a `/rendered` URL the harness serves, on the same host and Chrome. Recorded for both: time to first frame, sustained frames per second, peak memory, and frame identity as AC10 counts it, cornerstone's from the embedded code of the frame its canvas shows after `setFrameNumber(n)`. The target is that Ocelli is not slower to the first frame, sustains the stream's rate where cornerstone does, and shows the right frame where cornerstone does not. A miss is recorded with its cause rather than passed | Recorded in `docs/lld/benchmarks.md` | A claim of better with no cornerstone number beside it | external, sprint verification, needs F-223 |

Evidence feasibility. The oracle's headless Chromium and Playwright's bundled Chromium are built without H.264, so AC10 to AC16 and AC23 to AC25 need Google Chrome's stable channel through Playwright's `channel: 'chrome'`, installed on the F-216 host as a host tool. HEVC rows need a browser whose `isConfigSupported` is true for `hvc1`, on the lead's understanding Safari or Chrome on hardware with an HEVC decoder, and an HEVC row with no such host stays open rather than passed. Question 8.

## Approach

1. `ocelli-codec/src/video/`: `mp4.rs`, `ts.rs`, `annexb.rs`, `mpeg2.rs` and `table.rs`, bounds-checked readers with a named constant for every size limit, no allocation per sample after the table is built.
2. `ocelli-codec/src/registry.rs`: the sixteen UIDs, `Capability::Stream`, `CodecError::StreamSyntax`.
3. `ocelli-dicom`: `video_stream` and the `YBR_PARTIAL_420` acceptance gated on the syntax.
4. `ocelli-pixel/src/color.rs`: `YbrPartial420` over three planes, reusing the partial-range inverse.
5. `ocelli-wasm`: the bridge, with `web-sys` features `VideoDecoder`, `VideoDecoderConfig`, `EncodedVideoChunk`, `VideoFrame` and `VideoFrameCopyToOptions` added at the pinned `web-sys` version, in the render-worker feature only.
6. `@ocelli/core`: the video source and its commands, and the `/rendered` fallback if question 4 says so.
7. Folder viewer: the page worker, `/video-frame`, the held frame, cine on presentation times, inspector fields and sentences.
8. `scripts/corpus_synth.py`: synthetic clips for each container and codec with the embedded frame code, rows in `corpus/manifest.tsv`, never a real patient video.
9. `tools/bench`: `viewer.video_playback` and F-223's video mode.

Rejected: `importExternalTexture` of a `VideoFrame` into WebGPU, which skips the copy but exists only on tier A, so tiers B and C would differ. A `<video>` element and canvas readback, route D. Converting colour in the page, which would be a second copy of stage 4.

## Dependencies, write ownership and resources

Depends on F-239, which accepts colour through `ColorTransform`, adds `DisplayScale` and `ColourWindow`, and names `YBR_PARTIAL_420` as refused. F-214, cine at Frame Time in the folder viewer, whose timing rules this story extends. F-223, the cornerstone comparison harness AC25 extends. F-231, the wasm session's frame cache. All four are in S46. Writes `crates/ocelli-codec/src/video/`, `registry.rs` and tests, `crates/ocelli-dicom/src/frame_open.rs` and tests, `crates/ocelli-pixel/src/color.rs` and tests, `crates/ocelli-wasm/src/` and `Cargo.toml`, `packages/core/src/`, every folder viewer file F-239 touches plus a page worker, `scripts/corpus_synth.py`, `corpus/manifest.tsv`, `tools/bench/` and `tools/oracle/unsupported.json`. The folder viewer files are shared with F-212 to F-215, so the viewer part integrates after them. The story splits into a library part, items 1 to 6 and 8, about 7w, and a viewer part, items 7 and 9, about 5w, as F-239 split. Chrome on the F-216 host is an exclusive resource for the timing rows.

The estimate, 12w, is the lead's: extraction 0.5w, demux of four containers 2.5w, the stream interface 0.5w, the WebCodecs bridge 1.5w, the 4:2:0 conversion and colour description 1w, the `@ocelli/core` source 1.5w, the folder viewer 2.5w, the synthetic corpus 0.5w, and the measurements of AC21 to AC25 with F-223's video mode 1.5w.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-codec`, `check ocelli-dicom`, `check ocelli-pixel`, `check ocelli-wasm` and `check ocelli-folder-viewer` while iterating, then the feature profile once at the integration tree. The sprint profile runs once for the sprint. AC21 runs `bin/ocelli.sh gate wasm` on base and tip. AC22 to AC25 run on the F-216 host with the release build and Chrome stable, whose version is recorded with each figure. AC12's ffmpeg comparison records ffmpeg's version and exact command line. No oracle golden row is claimed, because the reference cannot decode these files locally and its harness browser has no H.264 decoder.

## Boundary and tier

- wasm-bindgen: `ocelli-wasm` only
- Pixels across the boundary: none to the main thread. Decoded planes move from a `VideoFrame` in the worker into linear memory by channel 2's pattern, which is the new deviation of question 6.
- Render-loop allocation: none. Plane buffers are allocated per held frame by the core and reused through the cache.
- unsafe: none
- Tier A (WebGPU): full. The decode is the browser's and the conversion is `ocelli-pixel`'s, and the converted frame is drawn as any colour frame.
- Tier B (WebGL2): full, for the same reason.
- Tier C (CPU): full, the conversion is CPU arithmetic on every tier. Where the browser has no decoder for the stream, every tier reports the same named refusal.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | YBR_PARTIAL_420 inverse and 4:2:0 replication, PS3.3 C.7.6.3.1.2 | `crates/ocelli-pixel/tests/color.rs`, AC6 to AC9 |
| unit | Registry capability and refusal | `crates/ocelli-codec/tests/registry.rs`, AC1 |
| unit | Sample tables from MP4, MPEG-TS, Annex B and MPEG-2, ISO/IEC 14496-12, 13818-1 | `crates/ocelli-codec/tests/video_demux.rs`, AC3 to AC5 |
| unit | Fragment concatenation, PS3.5 8.2 | `crates/ocelli-dicom/tests/video_stream.rs`, AC2 |
| integration | Folder viewer frame, window, readout and refusals | `examples/folder-viewer/tests/video.rs`, AC14, AC15 |
| browser | Frame identity, cine timing, refusals | `tests/browser/video.mjs`, `packages/core` browser test, AC10, AC11, AC13, AC15 |
| conformance | Decoded planes against ffmpeg's | AC12 |

## cornerstone3D 5.8.2 is the floor

By the operator's rule: every behaviour cornerstone3D 5.8.2 has for DICOM video, Ocelli has too or this table names the gap and why, every departure is better on a measured axis and recorded under D14, and where cornerstone can be timed on the same clip Ocelli is not slower.

| Behaviour | cornerstone3D 5.8.2 | Ocelli after F-241 | Axis and measurement |
|-----------|---------------------|--------------------|----------------------|
| A local file, `wadouri:` or `dicomfile:` | Throws "does not have a rendered video view" unless the application registers a URL | Plays | Better, a local video is shown. AC10, AC25 |
| DICOMweb with a server `/rendered` resource | Plays it | Plays the bulk stream, and `/rendered` if question 4 says so | Same at least. AC11, AC25 |
| H.264 in MP4 or MPEG-TS | Plays whatever the browser's `<video>` plays | Plays through the browser's decoder after a Rust demux, MPEG-TS included where `<video>` does not play TS | Same or better. AC3, AC4, AC12 |
| HEVC Main and Main 10 | Plays where `<video>` does | Plays where `isConfigSupported` is true, else named | Same availability, refusal visible. AC15 |
| MPEG-2 | Only through a server that transcodes it | Refused by name locally, by question 4 | **Gap, named, unless question 4 adds a route.** No browser decodes MPEG-2 |
| Which frame is frame N | `1 + round(currentTime * fps)` with `fps` from Cine Rate or Frame Time | Sample N of the stream's own table | Better, frame identity counted N of N. AC10, AC11, AC25 |
| Cine rate | `setPlaybackRate(fps / 24)` | Each sample at its presentation time | Better, measured frames per second against the stream. AC13, AC23 |
| Play, pause, loop, frame range, rate | Yes | Yes | Same. AC13 |
| Window | CSS `feColorMatrix`, scale `(upper - lower + 1) / 255`, not a window, and ignored on `VIDEO_NEXT` | D-29's LINEAR per channel on converted values | Better, hand-computed. AC14 |
| Invert | None on the legacy path, ignored on `VIDEO_NEXT` | Per channel, D-29 | Better. AC14 |
| Zoom and pan | Yes | Yes, the folder viewer's view tools | Same |
| Rotate and flip | None, `getRotation` returns 0 | The folder viewer's view tools | Better |
| Readout and region statistics | Browser RGBA from a canvas, cached by the computed frame number | R, G and B of the converted sample N, per channel statistics | Better, the statistics belong to the frame shown. AC14, AC16 |
| Colour conversion | The browser's, matrix and chroma filter unstated | PS3.3 or the stream's matrix, by question 5, exact replication | Measured divergence, AC16, not scored |
| Annotations per frame | By `/frames/N` | The folder viewer's marks per frame | Same |
| Segmentation on video | Labelmap through `CanvasActor` | Not in this story | **Gap, named.** The folder viewer draws no labelmap on any image today |
| Audio | Muted, nothing else | Not played, the track named | Same, the omission visible |
| Speed and memory | Hardware `<video>` playback | Copy and CPU conversion per frame | Target not slower to the first frame and the stream's rate sustained. AC23 to AC25. The copy is this route's measured risk |

## Parity surface covered

Appendix B's VIDEO viewport type, partly: playback, frame stepping, rate and loop of `VideoViewport` for a local file and a DICOMweb instance. Not covered: segmentation on video and `VIDEO_NEXT`'s DOM element. No scored oracle row, because the reference renders no video under the oracle's browser.

## Deviations

None written by a draft. Question 6 asks for one new row in `docs/hld/DEVIATIONS.md`, numbered when the plan is approved: HLD 6 says decoded frames stop becoming JavaScript objects, and on route A each decoded frame is a WebCodecs `VideoFrame` inside the worker until its planes are copied into linear memory. D-29 already covers the colour window on a video frame. Question 3 may also need a SOURCE-POLICY section on patents, which is a policy change rather than a deviation.

## LLD impact

- `docs/lld/codecs.md`: the stream interface, the demuxer, the sixteen UIDs and AC12's comparison.
- `docs/lld/frame-open.md`: `video_stream` and `YBR_PARTIAL_420`.
- `docs/lld/pixel-pipeline.md`: 4:2:0 in stage 4, the colour description rule and AC16.
- `docs/lld/public-api.md`: the video source.
- `docs/lld/feature-availability.md`: video per browser capability, the same on every tier.
- `docs/lld/corpus.md`: the synthetic video rows.
- `docs/lld/build-targets.md`: AC21.
- `docs/lld/benchmarks.md`: AC22 to AC25.

## User-facing documentation impact

- `README.md`: the folder viewer and library sections say which video files play, which are refused and why, and that frame N is the stream's sample N. CLAUDE.md's "fifteen of the sixteen known transfer syntaxes" count is not edited by this story, because the registry test is the authority and F-229 changes the count first.
- `examples/folder-viewer/src/use.html`: the video route, the crates and functions it calls, and the WebCodecs requirement.
- `examples/folder-viewer/src/more.html`: a video card if the page has a card list for formats at implementation, else no change.

## Open questions

Each has the lead's recommendation. None is answered, and the plan stays draft.

1. **Decode route for the library.** A, WebCodecs in the core worker with the planes copied into linear memory. B, a Rust decoder in wasm. D, cornerstone's `<video>` element. Affects every AC. **Recommendation: A.** It ships no decoder, adds no dependency and keeps the pixel arithmetic in `ocelli-pixel`. B has no ready H.264 crate and raises question 3. D is the floor and is better on no axis.
2. **Decode route for the folder viewer.** A, the page worker decodes and posts planes to the local process. B, a Rust decoder in the local process. C, the platform decoder. Affects AC10, AC13 to AC15, AC22 to AC25. **Recommendation: A**, one decode route for both targets. C needs `unsafe` FFI outside the two permitted files and `objc2`, against F-238, and gives three behaviours. B is the same question as 1.
3. **Patents.** `docs/SOURCE-POLICY.md` governs copyright and is silent on patents. Options: no shipped artefact contains a software H.264 or HEVC decoder, and those codecs are decoded only by a browser or platform decoder, recorded as a new SOURCE-POLICY section, or no rule, or the operator takes advice first. **Recommendation: the first, as a policy section written in the approving change**, so route B cannot arrive later by a dependency nobody weighed. This is not legal advice and the operator may want counsel.
4. **MPEG-2.** No browser's WebCodecs decodes it. Options: refuse by name, add DICOMweb `/rendered` to the library so a server that transcodes it plays it as cornerstone does, or a later story for a Rust MPEG-2 decoder, whose pool patents the lead understands to have expired. **Recommendation: refuse locally by name and add `/rendered` in this story**, which keeps cornerstone's only MPEG-2 route, and file the Rust decoder later if real files need it.
5. **Colour matrix.** PS3.3 gives one BT.601 studio-range matrix for YBR_PARTIAL_420. PS3.5 8.2.8 says the stream's characteristics control decompression, and HD streams usually signal BT.709. Affects AC9 and AC16. **Recommendation: the stream's colour description when present, PS3.3's equations otherwise**, the choice shown in the inspector and both matrices hand-fixtured.
6. **The deviation from HLD 6.** A decoded `VideoFrame` exists as a JavaScript object in the worker for one copy. **Recommendation: accept, as a new row**, with the reason that the alternative is shipping a decoder, question 3.
7. **Size beyond tolerance.** If AC21's delta exceeds the render budget's 5% tolerance. Options: stop for a decision, move video to a third wasm module loaded only for video, or re-baseline. **Recommendation: a third module only if measured over**, because a module loaded on first video costs nothing for a viewer that never opens one.
8. **Evidence hosts.** Chrome stable is needed for every browser row, and an HEVC-capable browser for HEVC rows. **Recommendation: Chrome stable through Playwright on the F-216 host, and HEVC rows on Safari on the same machine recorded by hand**, with an HEVC row left open where neither decodes it.
9. **One story or two.** The library part is about 7w and the viewer part about 5w. **Recommendation: one F-ID in two parts, as F-239**, because both parts share the demuxer, the conversion and the evidence clips.
