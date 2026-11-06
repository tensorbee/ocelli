# F-229, JPEG XL decoding

**Status**: approved
**Epic ref**: E4.10
**Sprint**: S46
**Estimate**: 3w

Proposed story row for `docs/sprints/BACKLOG.md`, section M20: "JPEG XL decoding: the three PS3.5 A.4.12 transfer syntaxes .110, .111 and .112 registered as a sixth decoder family through jxl-oxide with default features off, exact stored values for lossless, JPEG reconstruction for .111, XYB as a codec-owned colour space, corpus rows generated from the synthetic references, and the oracle rows declared unsupported by cornerstone3D 5.8.2". Layer Rust, 3w, depends on F-217, milestone M20, phase P1.

Proposed `allocation.json` entry: `eid` E4.10, `epic` "E4 Pixel pipeline & codecs", `epic_key` E4, `depends_eids` E38.27, `depends_fids` F-217, `notes` "Asked by the operator on 2026-11-05 after reviewing the issues a team hit with legacy cornerstone, and placed in S46 the same day. E4.9 is skipped on purpose: F-145 (E35.3) declares a dependency on E4.9, which the backlog records as an import defect that `scripts/backlog_check.py` keeps declared, so a real E4.9 would silently resolve that dependency to the wrong story. Lands after F-220 integrates so F-220's generated notices are proved unchanged."

E4 is the codec epic (F-023 to F-030, E4.1 to E4.8). `docs/sprints/BACKLOG.md` "Recorded defects" says "F-145 | E35.3 | declared dependency E4.9 does not exist, E4 ends at E4.8", and `allocation.json` carries F-145's `depends_fids` as `"E4.9 (UNRESOLVED)"`. Allocating E4.9 here would make that sentence false and repoint a planning dependency without anyone deciding to, which is exactly what the recorded-defects table exists to prevent.

## Normative source, transcribed

The HLD names no JPEG XL syntax. `grep -rn -i 'jpeg xl\|jxl' docs/hld` finds nothing. Section 21's registry and section 15.2's dependency note are the HLD text this story implements against, quoted below. The transfer syntaxes are DICOM PS3.5 2026d, fetched on 2026-11-05 from dicom.nema.org into the session scratchpad and read there.

### HLD section 21, `docs/hld/18-codec-registry.md`

```rust
pub trait Decoder: Send + Sync {
    fn transfer_syntaxes(&self) -> &'static [&'static str];
    /// Decode one frame into `out`. Must not allocate.
    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8])
    -> Result<(), CodecError>;
}
```

"Explicit runtime registration, not the inventory crate - inventory does not work on WebAssembly, which is precisely why dicom-rs's own plugin registry is unavailable there. Explicit registration is also what lets a native build link C codecs the browser build cannot." (The source uses an em-dash where this quotation has a hyphen, under the voice rule.)

### HLD section 15.2, `docs/hld/12-workspace-and-build.md`

The workspace manifest lists `wgpu`, `dicom`, `glam`, `bytemuck` and `thiserror`, and the note: "Disable default features. The defaults are rayon and simd, and the gdcm feature is native-only. On wasm you want: default-features = false, then jpeg, rle, deflate and openjp2 selected explicitly."

### Deviation D-21, `docs/hld/DEVIATIONS.md`, as it stands

"Concrete JPEG, JPEG 2000, JPEG-LS and HTJ2K adapters may allocate bounded library-owned decoded storage, validate it completely, then copy it atomically into the caller-provided output. An adapter may also make a bounded encoded-input copy when its safe third-party packet API requires owned bytes. Registry dispatch and raw plus RLE decoders remain allocation-free per call."

### PS3.5 2026d section 8.2.15, JPEG XL Image Compression

"DICOM provides a mechanism for supporting the use of JPEG XL Image Compression through the Encapsulated Format. Annex A defines a number of Transfer Syntaxes that reference the JPEG XL Standard. The JPEG XL Lossless Transfer Syntax provides a compression scheme that preserves the bits of the original image, i.e., lossless. The JPEG XL JPEG Recompression Transfer Syntax preserves the bits of the (lossy) JPEG encoding. The JPEG XL Transfer Syntax is a potentially lossy compression of the original image."

"The use of the DICOM Encapsulated Format to support JPEG XL Compressed Pixel Data requires that the Data Elements that are related to the Pixel Data encoding (e.g., Photometric Interpretation, Samples per Pixel, Planar Configuration, Bits Allocated, Bits Stored, High Bit, Pixel Representation, Rows, Columns, etc.) shall contain Values that are consistent with the characteristics of the compressed data stream. The Pixel Data characteristics included in the JPEG XL bit stream shall be used to decode the compressed data stream."

"No other Standard Photometric Interpretation values shall be used."

Table 8.2.15-1, "Valid Values of Pixel Data Related Attributes for JPEG XL Transfer Syntaxes using Standard Photometric Interpretations", read from the HTML table with its row spans:

| Photometric Interpretation | Transfer Syntax | UID | Samples per Pixel | Planar Configuration | Pixel Representation | Bits Allocated | Bits Stored | High Bit |
|----------------------------|-----------------|-----|-------------------|----------------------|----------------------|----------------|-------------|----------|
| MONOCHROME1, MONOCHROME2 | JPEG XL Lossless | 1.2.840.10008.1.2.4.110 | 1 | absent | 0 or 1 | 1,8,16,24 | 1-24 | 0-23 |
| MONOCHROME1, MONOCHROME2 | JPEG XL | 1.2.840.10008.1.2.4.112 | 1 | absent | 0 or 1 | 1,8,16,24 | 1-24 | 0-23 |
| MONOCHROME2 | JPEG XL JPEG Recompression | 1.2.840.10008.1.2.4.111 | 1 | absent | 0 | 8 | 8 | 7 |
| XYB, YBR_RCT, RGB | JPEG XL Lossless | 1.2.840.10008.1.2.4.110 | 3 | 0 | 0 | 8,16,24 | 8-24 | 7-23 |
| XYB, YBR_RCT, RGB | JPEG XL | 1.2.840.10008.1.2.4.112 | 3 | 0 | 0 | 8,16,24 | 8-24 | 7-23 |
| YBR_FULL_422, XYB, RGB | JPEG XL JPEG Recompression | 1.2.840.10008.1.2.4.111 | 3 | 0 | 0 | 8 | 8 | 7 |

Notes under the table: "When decompressing, should the characteristics explicitly specified in the compressed data stream be inconsistent with those specified in the DICOM Data Elements, those explicitly specified in the compressed data stream should be used to control the decompression." "The JPEG XL bit stream is capable of encoding both signed and unsigned pixel values, hence the value of Pixel Representation (0028,0103) may be either 0 or 1 for monochrome Photometric Interpretations depending on what has been encoded." "The value of Planar Configuration (0028,0006) is irrelevant since the manner of encoding components is specified in the JPEG XL standard, hence it shall be set to 0."

### PS3.5 2026d section 10.19

"One Transfer Syntax is specified for JPEG XL Lossless Image Compression, one for JPEG XL JPEG Recompression, which allows for transcoding JPEG encoded data without additional loss, and one for a general JPEG XL Image Compression scheme for any JPEG XL encoded data." Note: "When a JPEG Baseline encoded image is transcoded to JPEG XL, if the JPEG XL JPEG Recompression Transfer Syntax is used rather than the JPEG XL Transfer Syntax, then it communicates that the exact bitwise representation of JPEG can be recovered."

### PS3.5 2026d Annex A.4.12, the three UIDs and their names

"A Transfer Syntax with a UID of "1.2.840.10008.1.2.4.110", which specifies the use of the lossless mode of JPEG XL." "A Transfer Syntax with a UID of "1.2.840.10008.1.2.4.111", which specifies the use of reversible JPEG transcoding." "A Transfer Syntax with a UID of "1.2.840.10008.1.2.4.112", which specifies the use of any compression method in JPEG XL, including the lossy, lossless or JPEG recompression mode of JPEG XL." "For JPEG XL encoding, each Frame shall be encoded separately as a single Fragment."

The names, from section 8.2.15 and Table 8.2.15-1: JPEG XL Lossless, JPEG XL JPEG Recompression, JPEG XL. dicom-rs 0.10.0 `entries.rs` uses the same three names, with "JPEG XL Recompression" for `.111`.

### PS3.5 2026d Annex F.5

"A JPEG XL stream allows for bit depths up to 24 bits and up to 8192 components. Components do not need to all be the same type or bit depth. The color space of the image is specified in the JPEG XL encoding."

### PS3.3 2026d C.7.6.3.1.2, the XYB Photometric Interpretation

"XYB Pixel data represent a color image described by XYB, the long/medium/short wavelength (LMS) based color model inspired by the human visual system, facilitating perceptually uniform quantization. It uses a gamma of 3 for computationally efficient decoding. The exact details of the XYB encoding are defined as part of a specific image being encoded in order to optimize image fidelity. Images in XYB transcoded to other Transfer Syntaxes will use RGB or the appropriate equivalent (e.g., YBR_FULL_422 for JPEG)." Note: "This is a possible color space used in JPEG XL [ISO/IEC 18181-1]."

### The code this story extends, read from the tree

- `crates/ocelli-codec/src/registry.rs`, `KNOWN_TRANSFER_SYNTAXES`, sixteen UIDs, none of them JPEG XL. `Registry::register` refuses a UID outside the known catalogue (`RegistryError::UnknownTransferSyntax`) and a UID already owned (D-19).
- `crates/ocelli-codec/src/htj2k.rs`, the pattern this family copies: one adapter per UID with a `Mode`, a `register_*_decoders` that preflights every UID with `capability` before registering any, `transfer_syntaxes` per mode, validation of the descriptor and the codestream header before decode, and an atomic write.
- `crates/ocelli-codec/src/jpeg.rs`, `JpegDecoder` for `.50`, which accepts only marker `0xc0` at precision 8, reports `DecodePhotometricInterpretation::Rgb` for three samples and `DecodeSampleLayout::Interleaved`.
- `crates/ocelli-codec/tests/registry.rs`, `built_in_catalogue_starts_known_but_unavailable` asserts `KNOWN_TRANSFER_SYNTAXES.len() == 16`, and `every_decoder_family_registers_into_one_registry_without_collision` registers five functions and asserts the one unavailable UID is Deflated Explicit VR Little Endian.
- `crates/ocelli-dicom/src/parse.rs`, `parse_part10` resolves the UID through `TransferSyntaxRegistry` and classifies any encapsulated syntax as `DispatchPath::Encapsulated`.
- `crates/ocelli-dicom/src/frame_open.rs`, `photometric_of` maps nine strings and returns `FrameOpenError::Photometric` for anything else, including `XYB`. `ColourSelection::Colour { codec_already_rgb }` and `with_codec_rgb` refuse a second conversion.
- `crates/ocelli-pixel/src/stored_pixel.rs`, `PhotometricInterpretation` has nine variants and no `Xyb`. `crates/ocelli-pixel/src/color.rs`, `ColorTransform::resolve` refuses `YBR_ICT` and `YBR_RCT` that a decoder did not convert with `PixelError::CodecOwnedColorTransform`.
- `scripts/corpus_check.py`, `REGISTRY_TRANSFER_SYNTAXES`, the same sixteen, and the gate refuses a registry syntax with no corpus case. `scripts/corpus_synth.py`, `SYNTAX_CASES`, `EXTERNAL_ENCODERS`, `LOSSY_TRANSFER_SYNTAXES`.
- `scripts/pin_and_size_check.py`, `dependency_tree_outputs` runs `cargo tree -p ocelli-codec --edges normal,build` natively and for `wasm32-unknown-unknown` and refuses `rayon` or `rayon-core` in either.
- `tools/oracle/unsupported.json`, whose notes say "an entry has to match the row's transfer syntax, the boundary it failed at, the row's own path, and a fragment of the error the reference produced. A failure no entry describes fails the run, and a row an entry claims that then succeeds fails the run too."

## Facts measured for this plan

Measured on 2026-11-05 on the lead's macOS arm64 host, offline, from `~/.cargo/registry`, in scratch crates under the session scratchpad. Nothing was added to the repository.

1. **Licences.** `cargo metadata --offline --filter-platform wasm32-unknown-unknown` for a crate depending on `jxl-oxide = { version = "=0.12.6", default-features = false }` resolves, on normal and build edges: `jxl-oxide` 0.12.6, `jxl-bitstream` 1.1.0, `jxl-coding` 1.0.1, `jxl-color` 0.11.0, `jxl-frame` 0.13.3, `jxl-grid` 0.6.2, `jxl-image` 0.13.0, `jxl-jbr` 0.2.1, `jxl-modular` 0.11.3, `jxl-oxide-common` 1.0.0, `jxl-render` 0.12.4, `jxl-threadpool` 1.0.0 and `jxl-vardct` 0.11.1, all `MIT OR Apache-2.0`, `brotli-decompressor` 5.0.3 `BSD-3-Clause/MIT` (the legacy `/` separator), `alloc-no-stdlib` 2.0.4 and `alloc-stdlib` 0.2.4 `BSD-3-Clause`, `bytemuck` 1.25.2 `Zlib OR Apache-2.0 OR MIT`, `once_cell` 1.21.4 and `pin-project-lite` 0.2.17 `MIT OR Apache-2.0` or its reverse, `tracing` 0.1.44 and `tracing-core` 0.1.36 `MIT`. Every identifier is in `deny.toml`'s allow list. No `rayon`. `alloc-stdlib` ships no licence file, the others do.
2. **What enters `Cargo.lock`.** The thirteen `jxl-*` crates, `brotli-decompressor`, `alloc-no-stdlib` and `alloc-stdlib`. The other five are already locked at the same versions.
3. **The shipped closure does not change.** `ocelli-wasm` depends on `ocelli-cache`, `ocelli-core`, `ocelli-render` and `ocelli-viewport`, and `ocelli-native` on `ocelli-render`. Neither reaches `ocelli-codec`, which F-220's plan records as its fact 2 ("Neither vendored crate is in the closure today"). So F-220's `THIRD_PARTY_LICENSES`, `ci/wasm-size-budget.json` (2,151,196 bytes) and `ci/wasm-decode-size.json` (16,455 bytes) are not moved by this story. The consumers that link `ocelli-codec` are `ocelli-dicom` (dev-dependency), `examples/folder-viewer` and `tools/oracle`, all `publish = false`.
4. **Forward size cost.** A wasm32 `cdylib` that reads a JPEG XL image and renders frame 0 through `jxl-oxide` 0.12.6, built with the workspace release profile (`opt-level = "z"`, fat LTO, one codegen unit, `panic = "abort"`, `strip`) and no wasm-opt, is 940,335 bytes. An empty `cdylib` with the same profile is 75 bytes. This is what the decode worker will pay when a later story links `ocelli-codec` into a wasm module.
5. **It builds for wasm32 on the pinned toolchain.** `cargo +1.97.1 build --release --target wasm32-unknown-unknown` succeeded for the probe.
6. **The 12-bit trap.** A 64 by 48 12-bit monochrome lossless codestream written by imagecodecs 2026.10.10 (libjxl 0.12.0) reports `bit_depth: IntegerSample { bits_per_sample: 12 }`. `Render::color_channels()` holds an `I16` grid whose first values are `0, 13, 26, 39`, which are the encoded stored values. `Render::stream().write_to_buffer::<u16>` writes `0, 208, 416, 624`, because `jxl-oxide`'s `u16` sample rescales any depth other than 16 to the full 16-bit range (`fb.rs`, `copy_from_f32`, `val * 65535.0 + 0.5`). 13 times 65535 over 4095 is 208.05. dicom-rs 0.10.1's `JpegXlAdapter` decodes Bits Allocated 16 through exactly that `write_to_buffer::<u16>` call.
7. **Lossy output.** A lossy 8-bit RGB codestream (`xyb_encoded = true`, colour encoding enum sRGB) renders to `F32` grids in nominal `[0, 1]`, after `jxl-oxide`'s own XYB-to-RGB conversion into the declared encoding. Values below 0 occur (`-0.060342673`). Lossless 8-bit RGB renders to exact `I16` grids. 16-bit lossless renders to exact `I32` grids.
8. **`ImageBuffer` is not re-exported.** `jxl_oxide::Render::color_channels` returns `&[jxl_render::ImageBuffer]`, and `jxl-oxide` does not re-export the type, so naming its `I16`, `I32` and `F32` variants needs a direct dependency on `jxl-render` 0.12.4.
9. **JPEG reconstruction is exact.** A JPEG written by imagecodecs `jpeg8_encode` (2,987 bytes), transcoded by `jpegxl_encode_jpeg`, reports `JpegReconstructionStatus::Available` in `jxl-oxide`, and `reconstruct_jpeg` writes 2,987 bytes identical to the source. imagecodecs also transcodes the corpus row `syntax/jpeg_baseline_rgb8.dcm`'s 936-byte fragment (Photometric Interpretation `YBR_FULL_422`) to 674 bytes and back byte-identically.
10. **The fixture encoder is already locked.** imagecodecs 2026.10.10, `BSD-3-Clause`, pinned by F-217 in `pyproject.toml` and `uv.lock`, reports `JPEGXL.available` and `libjxl 0.12.0`. Its `licenses/` directory carries `LICENSE-libjxl`, `LICENSE-brotli`, `LICENSE-highway` and `LICENSE-lcms2`, which F-217's `licences-wheels` gate already scans. A 12-bit lossless round trip through `jpegxl_encode(lossless=True, bitspersample=12)` and `jpegxl_decode` is exact.
11. **dicom-rs already knows the UIDs.** `dicom-transfer-syntax-registry` 0.10.0, without its `jpegxl` feature, declares `JPEG_XL_LOSSLESS`, `JPEG_XL_RECOMPRESSION` and `JPEG_XL` through `create_ts_stub`, which is `Codec::EncapsulatedPixelData(None, None)`. `parse_part10` therefore classifies all three as `Encapsulated` with no change to `ocelli-dicom`'s parser.
12. **The reference cannot decode them.** `tools/oracle/node_modules/@cornerstonejs/` holds `codec-charls`, `codec-libjpeg-turbo-8bit`, `codec-openjpeg` and `codec-openjph`, and `dicom-image-loader` 5.8.2's `dist` contains none of the three UIDs.
13. **Unsafe in the dependency.** Whole-word `unsafe` lines under each crate's `src/`: `jxl-render` 114, `jxl-grid` 107, `brotli-decompressor` 98, `jxl-color` 93, `jxl-modular` 52, `alloc-stdlib` 10, `alloc-no-stdlib` 9, `jxl-vardct` 3, `jxl-oxide` 2, `jxl-coding` 2, `jxl-bitstream` 1, the other four 0. A line count, not a construct count, and not split by target.
14. **The corpus references are unsigned.** `syntax/explicit_vr_le.dcm` is 64 by 96, Bits Allocated 16, Bits Stored 16, Pixel Representation 0. `syntax/reference_mono12.dcm` is the same at Bits Stored 12. `syntax/reference_rgb8.dcm` is 64 by 96 RGB, 8 bits.

## What the specification does not cover

The HLD does not name JPEG XL, and PS3.5 does not say how a decoder represents signed samples in a JPEG XL codestream, which output a decoder writes for each bit depth, or what a lossy float sample rounds to. Every item below is a lead decision under the operator's delegation of 2026-11-05.

1. **Dependency.** `jxl-oxide = { version = "=0.12.6", default-features = false }` and `jxl-render = { version = "=0.12.4", default-features = false }` in `[workspace.dependencies]`, used only by `ocelli-codec`. Default features off removes `rayon`, which D5 and the pins gate both refuse. `jxl-render` is direct because of fact 8. dicom-rs's `jpegxl` feature is not used: it goes through dicom-rs's inventory registry, which section 21 says does not work on wasm, and it pulls `zune-jpegxl` as well. This is a departure from section 15.2's dicom-rs feature route of the kind D-20 and D-22 record, so deviation D-28 records it (see Deviations).
2. **Exact stored values for lossless.** Integer grids (`I16`, `I32`) are read sample by sample from `Render::color_channels()` and written as the stored value, range-checked against Bits Stored, never through `ImageStream::write_to_buffer`, because of fact 6. A sample outside `[0, 2^BitsStored - 1]` refuses with `UnsupportedPixelFormat`, the same refusal `sample_convert.rs` makes.
3. **Lossy float samples.** An `F32` grid, which is what a VarDCT or XYB frame renders to, is quantised to the stored domain by `q = floor(v * (2^BitsStored - 1) + 0.5)`, clamped to `[0, 2^BitsStored - 1]`. A non-finite sample refuses with `DecoderFailure` before anything is written. Half rounds up, which on a non-negative operand is round half away from zero. The arithmetic is in `f64` from the `f32`, and the multiplier is exact. This is the decoder's own output contract, because neither PS3.5 nor ISO/IEC 18181-1 assigns DICOM stored values to a float render.
4. **Signed samples refuse.** Pixel Representation 1 returns `UnsupportedPixelFormat`. PS3.5 says the stream "is capable of encoding both signed and unsigned pixel values" and does not say how, the JPEG XL integer sample is unsigned, and no fixture of a signed DICOM JPEG XL frame exists. A guessed offset or two's-complement reinterpretation is the quietly wrong pixel. The refusal reports unavailable and names the reason in `docs/lld/codecs.md`. A follow-up story is filed when a reference signed file is available.
5. **Bits Allocated.** 8 with Bits Stored 1 to 8, and 16 with Bits Stored 1 to 16. Bits Allocated 1 refuses (`UnsupportedPixelFormat`, packed output is not produced). Bits Allocated 24 is refused earlier by `FrameDesc::new`, which accepts only 1, 8, 16 and 32. The codestream's `bits_per_sample` must equal Bits Stored, else `FrameMismatch`, which is the repository's existing policy for every family (the HTJ2K and JPEG headers are validated the same way) and is consistent with PS3.5's "shall contain Values that are consistent". Float samples (`BitDepth::FloatSample`) refuse.
6. **What the codestream must look like.** Width and height equal Columns and Rows, else `FrameMismatch`, checked from the header before rendering. Orientation 1 only, else `UnsupportedPixelFormat`, because `color_channels` does not apply orientation and DICOM rows and columns have no orientation field to carry it. Exactly one keyframe, else `FrameMismatch`, from A.4.12's "each Frame shall be encoded separately as a single Fragment". No extra channel (alpha, black, spot), else `UnsupportedPixelFormat`, because Samples per Pixel is 1 or 3. Colour channels must equal Samples per Pixel, else `FrameMismatch`. A colour encoding given as an ICC profile refuses (`UnsupportedPixelFormat`), because Ocelli implements no ICC (CLAUDE.md, stage 4). An enum colour encoding with a PQ or HLG transfer function refuses, because DICOM has no HDR Photometric Interpretation. Trailing bytes after the codestream follow the existing even-length rule: one trailing `0x00` pad is accepted, anything else is `TrailingData` when detectable.
7. **`.110` is lossless or it refuses.** A frame header with VarDCT encoding, or an image header with `xyb_encoded = true`, under `.110` returns `FrameMismatch`, the same shape as `.90` refusing an irreversible codestream. Lossy modular (quantised squeeze) is not detected, and `docs/lld/codecs.md` says so.
8. **`.111` reconstructs the JPEG and decodes it through the existing `.50` path.** `jpeg_reconstruction_status` must be `Available`, else `FrameMismatch`. `reconstruct_jpeg` writes the JPEG bitstream, which then goes through `JpegDecoder::baseline()`'s decode with the same `FrameDesc`. A `.111` frame therefore decodes to the same bytes as the `.50` frame it was transcoded from, which section 10.19's note makes a checkable property. A progressive source JPEG is refused by the baseline path (`0xc0` only) and that is recorded, not worked around. `.112` always renders through `jxl-oxide`, including a frame that carries reconstruction data.
9. **Output space contract.** `decode_sample_layout` is `Interleaved` for every mode. `decode_photometric_interpretation` is `Rgb` for three samples in every mode, because `jxl-oxide` renders XYB and the reconstructed JPEG decodes YCbCr into RGB. `YBR_RCT` under `.110` and `.112` also decodes to RGB, because the codestream's own transform is inverted inside `jxl-oxide`. One sample is `Preserved`. The colour stage then applies no second conversion through `with_codec_rgb`.
10. **XYB is a recognised, codec-owned colour space.** `PhotometricInterpretation::Xyb` in `ocelli-pixel` with three expected samples, `"XYB"` in `frame_open`'s map, and `ColorTransform::resolve` refusing an `Xyb` frame that its codec did not convert, with `PixelError::CodecOwnedColorTransform`, exactly as for `YBR_ICT` and `YBR_RCT`. PS3.3 C.7.6.3.1.2 gives no matrix for XYB, so no route other than the codec's exists.
11. **Bounded allocation.** Each decode builds its `JxlImage` with `AllocTracker::with_limit(n)`, where `n` is a stated multiple of `FrameDesc::output_len()`. The multiple is measured at implementation as the worst tracked peak over the fixtures and a 512 by 512 synthetic frame, then doubled, and recorded in `codecs.rs` with the measurement. `jxl-oxide` allocates its grids and the reconstruction writes a `Vec`, so D-21 gains JPEG XL by amendment.
12. **Malformed input never panics.** The release profile is `panic = "abort"`, so a dependency panic in the folder viewer's process ends the process. A deterministic mutation test (a fixed-seed linear congruential generator in the test, no new dependency) truncates every fixture at every byte offset and applies 2,000 single-bit flips to each, in the test profile, and requires every call to return `Ok` with a full write or `Err` with `out` unchanged. If `jxl-oxide` panics on any case, implementation stops and asks the operator, because the alternatives (a pre-validation pass, a different crate, or accepting the risk) change scope.
13. **Fixtures without patient data.** Every fixture is encoded from the existing synthetic corpus references by imagecodecs 2026.10.10 with libjxl 0.12.0, which F-217 already pins and licence-checks (fact 10). No new tool, no download, no patient data. The encoder is a tooling dependency, never shipped.
14. **The oracle row is declared unsupported.** cornerstone3D 5.8.2 cannot decode the three syntaxes (fact 12), so each JPEG XL corpus row gets an `unsupported.json` entry whose `errorContains` is copied from the harness report of an actual run, never written from expectation. Correctness comes from the lossless round trip against the uncompressed synthetic source, which is independent of `jxl-oxide` because a different implementation (libjxl) encoded it.

## Problem and reproducer

```bash
python3 -c "import sys; sys.path.insert(0,'scripts'); import corpus_check; print(len(corpus_check.REGISTRY_TRANSFER_SYNTAXES))"   # 16
grep -c 'register_' crates/ocelli-codec/src/lib.rs
```

A Part 10 file whose File Meta Information says `1.2.840.10008.1.2.4.110` parses today (fact 11) and then `Registry::decoder` returns `CodecError::UnknownTransferSyntax`, because the UID is not in `KNOWN_TRANSFER_SYNTAXES`. The folder viewer shows "could not be decoded". Expected after this story: the three UIDs are known and available, a lossless frame decodes to exactly the synthetic source's stored values, a `.111` frame decodes to the same bytes as its `.50` source, and a signed, ICC, HDR, oriented or multi-keyframe frame reports unavailable instead of drawing something.

## Scope, API and compatibility

In scope: the decoder family, the catalogue entries, the XYB colour space, corpus rows and fixtures, the oracle declarations, registration in the two `publish = false` consumers, the source-policy rows and unsafe audit, the D-21 amendment and deviation D-28.

Non-goals: encoding, signed samples (decision 4), Bits Allocated 1 and 24, ICC, HDR, progressive `.111` sources, a JPEG XL subject in `tools/bench`, linking `ocelli-codec` into any wasm module, and any change to the wasm size budgets (fact 3).

```rust
// crates/ocelli-codec/src/jpegxl.rs, re-exported from lib.rs
pub struct JpegXlDecoder { mode: Mode }   // Mode::Lossless (.110), Recompression (.111), General (.112)
impl JpegXlDecoder {
    pub const fn lossless() -> Self;
    pub const fn recompression() -> Self;
    pub const fn general() -> Self;
}
pub fn register_jpegxl_decoders(registry: &mut Registry) -> Result<(), RegistryError>;
```

`KNOWN_TRANSFER_SYNTAXES` gains `1.2.840.10008.1.2.4.110`, `.111` and `.112`, nineteen in all, and its doc comment says the membership source is PS3.5 Annex A. `register_jpegxl_decoders` preflights all three with `capability` before registering any, like `register_htj2k_decoders`.

`ocelli-pixel`: `PhotometricInterpretation::Xyb`. `ocelli-dicom`: `"XYB" => Ok(PhotometricInterpretation::Xyb)` and `colour_of` maps it to `ColourSelection::Colour { codec_already_rgb: false }`, which `with_codec_rgb(true)` then sets. Adding an enum variant is a compatible change for every in-repository `match`, which the compiler finds.

Compatibility: every existing UID keeps its decoder and its bytes. `Registry::new()` knows three more UIDs, which a caller that never registers JPEG XL sees as `KnownUnavailable` instead of `Unknown`. That is the intended change and AC2 asserts it.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `Registry::decode`, `.110` and `.112`, integer grid | `jxl-oxide` render, `jxl-render::ImageBuffer` | Exact stored values, range-checked, atomic | `out` bytes |
| `Registry::decode`, `.112`, float grid | same, then decision 3's quantiser | Lossy, non-finite refuses | `out` bytes |
| `Registry::decode`, `.111` | `reconstruct_jpeg`, then `JpegDecoder::baseline()` | Same bytes as the `.50` source | `out` bytes |
| `decode_photometric_interpretation` | `JpegXlDecoder` | `Rgb` for three samples, `Preserved` for one | the colour stage's input space |
| `open_frame` and the colour stage | `frame_open`, `ColorTransform::resolve` | `XYB` recognised, codec-owned | one conversion, never two |
| folder viewer `codecs()` in `preview.rs` | `register_jpegxl_decoders` | Monochrome frames display, colour refuses as today with `NotMonochrome` | the image |
| `tools/oracle/src/produce.rs` | `register_jpegxl_decoders` | Candidate side registers the family | candidate frames for rows the reference cannot render |
| `scripts/corpus_check.py` | `REGISTRY_TRANSFER_SYNTAXES`, nineteen | Every registry syntax has a case | gate `corpus` |
| `scripts/tests/test_corpus_synth.py` | the declared route `imagecodecs.jpegxl_decode` | Exact for `.110`, bounded for `.112`, byte-equal for `.111` reconstruction | gate `corpus-tests` |
| `tools/oracle/unsupported.json` | measured error text | Declared, not assumed | gate `oracle` |

Owner of stored-value conversion and quantisation: `jpegxl.rs`. Owner of YCbCr and XYB to RGB: the dependency, reported through `Rgb`. Owner of the `.50` decode reused by `.111`: `jpeg.rs`, unchanged.

## State transitions and invariants

Not applicable as a stateful change. A decode is a pure function of the encoded bytes and the descriptor. The registry invariant extended: the six families partition the nineteen known UIDs, the one unavailable UID is Deflated Explicit VR Little Endian, and registration stays atomic.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Decoded grids and the reconstructed JPEG | `jxl-oxide` under the `AllocTracker` limit, per call | the call | `out.copy_from_slice` after every sample validated | Any error leaves `out` unchanged. A repeated call repeats the work and writes the same bytes |
| Registry entries | `register_jpegxl_decoders` | the `Registry` | after all three preflights pass | A second call refuses with `AlreadyRegistered` and leaves the three entries intact |

## Numbered acceptance inventory

Fixtures are encoded by imagecodecs 2026.10.10 (libjxl 0.12.0) from the synthetic references, written by `crates/ocelli-codec/tests/fixtures/generate_jpegxl.py`, and tracked as `.jxl` codestreams beside the existing `.j2c` and `.jpg` fixtures. The references are the existing raw fixtures where one exists (`jpegls_corpus_reference_u16le.raw`, SHA-256 `b20a1ef3...7609`, the 16-bit ramp of `syntax/explicit_vr_le.dcm`).

| ID | Trigger and expected behaviour | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The three UIDs are in `KNOWN_TRANSFER_SYNTAXES`, nineteen in all, and `KNOWN_TRANSFER_SYNTAXES` and `REGISTRY_TRANSFER_SYNTAXES` list the same UIDs | `built_in_catalogue_starts_known_but_unavailable` (16 becomes 19), `test_corpus_check.py` | A catalogue that knows the UIDs in one place only | implementation |
| AC2 | Six families register into one registry without collision, eighteen available, Deflated the one unavailable. A fresh `Registry::new()` reports the three as `KnownUnavailable`, not `Unknown` | `every_decoder_family_registers_into_one_registry_without_collision`, extended | A JPEG XL family claiming a UID another family owns | implementation |
| AC3 | `register_jpegxl_decoders` twice refuses with `AlreadyRegistered` and leaves all three available. A registry built with `with_known` lacking `.111` refuses before registering `.110` | `registering_jpegxl_twice_refuses`, `preflight_is_atomic` | Registering `.110` before discovering `.111` is unknown | implementation |
| AC4 | Lossless 16-bit, PS3.5 8.2.15. `.110` of the 64 by 96 16-bit ramp decodes byte-identical to `jpegls_corpus_reference_u16le.raw` | `jpegxl_lossless_16_matches_the_reference` | Any rescaling or byte-order slip. Mutation: add 1 to one sample in the conversion, the test goes red | implementation |
| AC5 | Lossless 12-bit, the trap of fact 6. `.110` of `reference_mono12`'s ramp decodes to the reference's stored values exactly. The reference's first six stored values are `0, 0, 1, 1, 2, 3` and its maximum is 4095, so sample index 2 must decode to 1, not 16 (1 times 65535 over 4095 is 16.0037), and the maximum to 4095, not 65535 (PS3.5 8.2.15, Bits Stored) | `jpegxl_lossless_12_keeps_stored_values` | `write_to_buffer::<u16>`, which writes 16 and 65535 | implementation |
| AC6 | Lossless RGB 8-bit. `.110` of `reference_rgb8` decodes to the reference's interleaved RGB exactly, and `decode_photometric_interpretation` is `Rgb`, `decode_sample_layout` is `Interleaved` | `jpegxl_lossless_rgb8_matches_the_reference` | Planar output. `Preserved` reported for a colour frame | implementation |
| AC7 | Recompression, PS3.5 10.19. `.111` of the `.50` fixture `jpeg_baseline_rgb.jpg` decodes byte-identical to `JpegDecoder::baseline()` on the original JPEG with the same `FrameDesc` | `jpegxl_recompression_matches_the_jpeg_it_came_from` | Rendering `.111` through `jxl-oxide`'s own IDCT, which differs from jpeg-decoder's | implementation |
| AC8 | `.111` without reconstruction data (a `.112` lossy fixture relabelled) refuses with `FrameMismatch` | `recompression_without_jbrd_refuses` | Falling back to rendering | implementation |
| AC9 | Lossy, decision 3, hand-computed. The quantiser at Bits Stored 8: 0.0 gives 0, 1.0 gives 255, 0.5 gives 128 (127.5 rounds up), 0.498 gives 127 (126.99), -0.25 gives 0, 1.25 gives 255. At Bits Stored 12: 0.5 gives 2048 (2047.5), 1/4095 as `f32` gives 1. NaN and infinity refuse | `fixture_lossy_quantiser` | Truncation, which gives 127 for 0.5. Rounding without the clamp, which wraps -0.25 | implementation |
| AC10 | Lossy round trip. `.112` lossy 12-bit monochrome of `reference_mono12` decodes within a maximum absolute error of B stored units of the reference, where B is 1.5 times the error measured once at implementation through imagecodecs `jpegxl_decode`, recorded with the encoder settings. If `jxl-oxide`'s error exceeds B, implementation stops and asks. The bound is never raised to pass | `jpegxl_lossy_stays_within_the_recorded_bound` | A wrong scale, which gives a 16-fold error, or an off-by-one row stride | implementation |
| AC11 | `.110` refuses a lossy codestream. The AC10 fixture labelled `.110` returns `FrameMismatch` | `lossless_only_refuses_vardct_and_xyb` | Accepting lossy data under a lossless UID | implementation |
| AC12 | Descriptor refusals, each with `out` unchanged: Pixel Representation 1 `UnsupportedPixelFormat`. Bits Allocated 1 `UnsupportedPixelFormat`. Bits Stored 12 against a 16-bit codestream `FrameMismatch`. Rows or Columns differing `FrameMismatch`. Samples per Pixel 3 against a grey codestream `FrameMismatch` | `descriptor_mismatches_refuse` | Trusting the DICOM attributes over the codestream | implementation |
| AC13 | Codestream refusals, each with `out` unchanged: orientation 6, two keyframes, an alpha channel, an ICC colour encoding, a PQ transfer function, `BitDepth::FloatSample`. Each fixture is generated by imagecodecs with the named option | `unsupported_codestreams_report_unavailable` | Applying orientation silently. Dropping alpha. Ignoring ICC | implementation |
| AC14 | XYB, PS3.3 C.7.6.3.1.2. `frame_open` maps `XYB` to `Xyb`, and `ColorTransform::resolve` on an `Xyb` frame with a decoder that reported `Preserved` refuses with `CodecOwnedColorTransform`. With `Rgb` reported it passes through unconverted | `xyb_is_codec_owned` in `ocelli-pixel/tests/color.rs`, `frame_open` test | A matrix applied to XYB. `FrameOpenError::Photometric` for a valid XYB file | implementation |
| AC15 | No second colour conversion. A `.111` `YBR_FULL_422` frame opened through `open_frame` and the colour stage equals the AC7 RGB bytes | `recompressed_ybr_is_converted_once` | Treating `YBR_FULL_422` in the header as still YCbCr | implementation |
| AC16 | Malformed input never panics (decision 12). Every truncation and 2,000 bit flips per fixture return `Ok` with a full write or `Err` with `out` unchanged | `malformed_jpegxl_never_panics` | A panic that, under `panic = "abort"`, ends the viewer's process | implementation. A panic stops the story and goes to the operator |
| AC17 | Bounded allocation. A codestream declaring 65,535 by 65,535 under a 64 by 96 descriptor refuses with `FrameMismatch` before any grid is allocated. The `AllocTracker` limit and its measurement are recorded | `oversized_header_refuses_before_render` | Rendering first and comparing after | implementation |
| AC18 | No Rayon and the pinned versions. `scripts/pin_and_size_check.py` stays green on both `cargo tree` graphs, `Cargo.toml` pins `=0.12.6` and `=0.12.4` with defaults off | `bin/ocelli.sh gate pins` | Default features left on | implementation |
| AC19 | Licences. `bin/ocelli.sh gate licences` is green with the new lock and adds no exception to `ci/licences.toml` | `gate licences` | A licence nobody read | implementation |
| AC20 | Shipped closure unchanged (fact 3). After F-220 integrates, F-220's regenerated `THIRD_PARTY_LICENSES` is byte-identical before and after this story, and `bin/ocelli.sh wasm` and `bin/ocelli.sh wasm-decode` measure within their budgets with no budget file edited | F-220's gate, `gate wasm`, release builds | A codec edge added to `ocelli-wasm` by accident | integrated |
| AC21 | wasm32 build. `bin/ocelli.sh native` builds `ocelli-codec` for `wasm32-unknown-unknown` with the family compiled in. No wasm32 execution of a JPEG XL decode exists, so byte identity on wasm32 is argued from the integer path, not observed | `bin/ocelli.sh native` | A dependency that compiles natively only | implementation |
| AC22 | Corpus rows. Five synthetic rows, generated by `scripts/corpus_synth.py` through imagecodecs and added with `scripts/corpus_check.py --add`: `syntax/jpegxl_lossless.dcm` (`.110`, from `explicit_vr_le.dcm`, `synthetic, mono16, transfer-syntax`), `syntax/jpegxl_lossless_rgb8.dcm` (`.110`, RGB, from `reference_rgb8.dcm`, `synthetic, colour, transfer-syntax`), `syntax/jpegxl_recompression_rgb8.dcm` (`.111`, `YBR_FULL_422`, the `.50` row's fragment transcoded), `syntax/jpegxl_lossy.dcm` (`.112`, from `reference_mono12.dcm`), `syntax/jpegxl_xyb_rgb8.dcm` (`.112`, `XYB`, from `reference_rgb8.dcm`). Licence column `MIT OR Apache-2.0`, as for every generated row. `EXTERNAL_ENCODERS` gains imagecodecs with libjxl, `LOSSY_TRANSFER_SYNTAXES` gains `.112`, `SYNTAX_REFERENCE` names each source | `gate corpus`, `gate corpus-tests` | A row with no case for a registry syntax. A row whose source is not the synthetic reference | implementation, corpus present |
| AC23 | Python read-back. `test_lossless_syntaxes_round_trip_exactly` covers both `.110` rows through `imagecodecs.jpegxl_decode`, the `.111` row is checked by `jpegxl_decode_jpeg` returning the `.50` fragment byte-identically, and the two `.112` rows sit under the lossy bound test. The route table maps each new syntax to one declared decoder | `scripts/tests/test_corpus_synth.py` | Relying on pydicom's plugin order, which has no JPEG XL route | implementation |
| AC24 | Oracle declaration. `node tools/oracle/run.mjs --report-unsupported` lists the five rows, and `tools/oracle/unsupported.json` gains entries whose `errorContains` and `boundary` are copied from that report, with a `why` citing fact 12. `bin/ocelli.sh gate oracle` passes, which also proves no entry claims a row that succeeds | `gate oracle` | An entry written from expectation that matches no real error | integrated, needs the oracle host |
| AC25 | Unsafe audit. `docs/SOURCE-POLICY.md` gains a "jxl-oxide 0.12.6, the unsafe audit" section in the shape of the openjph-core one: whole-word `unsafe` constructs counted per file over every `src/` of the closure, split by whether the path is reachable on wasm32, and a malformed-input table from AC16. Extension rows for `jxl-oxide` 0.12.6 with its subcrates and `brotli-decompressor` 5.0.3 are added | review, `gate provenance prose` | A dependency added with no read of what it does unsafely | implementation |
| AC26 | Semantic RED, recorded in the progress note: AC1, AC2, AC4 and AC5 fail on today's source with `UnknownTransferSyntax` or the count 16, and AC5 fails against a `write_to_buffer::<u16>` implementation with 16 at index 2. Mutations after GREEN, each seen red: dropping the range check (AC4), truncating instead of rounding (AC9), rendering `.111` instead of reconstructing (AC7), removing the `.110` VarDCT refusal (AC11) | progress note | Tests that pass for the wrong reason | implementation |

## Approach

1. Workspace pins and `ocelli-codec`'s `Cargo.toml`. `cargo tree` for both targets to confirm no Rayon.
2. `jpegxl.rs`: header inspection through `JxlImage::builder().alloc_tracker(..).read(src)`, decisions 5 to 8 as validation before `render_frame(0)`, then the integer or float path, or the `.111` reconstruction into `JpegDecoder::baseline()`.
3. Registry: three UIDs, `lib.rs` exports, `tests/registry.rs` and `tests/jpegxl.rs`.
4. `ocelli-pixel` `Xyb`, its colour-stage refusal and fixture, and `frame_open`'s map.
5. Fixtures: `generate_jpegxl.py` with check and `--write` modes like `generate_htj2k.py`.
6. Corpus: `corpus_synth.py` cases, `corpus_check.py` and its test, `test_corpus_synth.py`, manifest rows through `--add`.
7. Consumers: `preview.rs` `codecs()`, `tools/oracle/src/produce.rs`.
8. Oracle run, `unsupported.json`.
9. Source policy, LLD, user documents, `CLAUDE.md` counts.

Rejected alternatives:

- dicom-rs's `jpegxl` feature. Inventory registry, unavailable on wasm (section 21), and it adds `zune-jpegxl`.
- `ImageStream::write_to_buffer` for integer output (fact 6).
- Rendering `.111` through `jxl-oxide`. It would make a transcoded image differ from its JPEG source by IDCT differences, which section 10.19's note says need not happen.
- Treating XYB as RGB in `frame_open` without a variant. It would let a decoder that did not convert pass XYB samples as RGB, the quietly wrong colour.
- A C libjxl binding in the product. A second toolchain for the browser build and C code the wasm build cannot link.
- Guessing a signed-sample convention (decision 4).

## Dependencies, write ownership and resources

Declared dependency: F-217 (done), whose licence gates check the new lock. Ordering, not dependency: after F-220 integrates, so AC20 is observed through F-220's generator. F-220 runs after F-221 in this sprint. The order is safe either way, because the shipped closure does not change (fact 3), and if a later story links `ocelli-codec` into a wasm module, F-220's generator picks up the `jxl-*` crates and `brotli-decompressor` from the lock, and `alloc-stdlib`, which ships no licence file, then needs a pinned text in F-220's `ci/notices/sources.toml`. That is recorded in `docs/lld/codecs.md` for the decode-worker story.

Write set:

```text
Cargo.toml, Cargo.lock
crates/ocelli-codec/Cargo.toml, src/lib.rs, src/registry.rs, src/jpegxl.rs (new)
crates/ocelli-codec/tests/jpegxl.rs (new), tests/registry.rs
crates/ocelli-codec/tests/fixtures/generate_jpegxl.py (new), *.jxl (new)
crates/ocelli-pixel/src/stored_pixel.rs, src/color.rs, tests/color.rs
crates/ocelli-dicom/src/frame_open.rs, tests/frame_open.rs
examples/folder-viewer/src/preview.rs
tools/oracle/src/produce.rs, tools/oracle/unsupported.json
scripts/corpus_synth.py, scripts/corpus_check.py
scripts/tests/test_corpus_synth.py, scripts/tests/test_corpus_check.py
corpus/manifest.tsv (five rows)
ci/target-feature-baseline.json (only if target_feature_check reports the two new direct dependencies)
docs/SOURCE-POLICY.md, docs/hld/DEVIATIONS.md (D-21 amendment and D-28, written with this approval on 2026-11-05)
docs/lld/codecs.md, docs/lld/corpus.md, docs/lld/oracle.md, docs/lld/pixel-pipeline.md, docs/lld/frame-open.md
.claude/skills/dicom-expert/SKILL.md and its generated adapters
CLAUDE.md (the transfer-syntax and family counts)
README.md, examples/folder-viewer/src/use.html, examples/folder-viewer/src/more.html
```

Exclusive or shared: `Cargo.lock` and `Cargo.toml` (no other open S46 story writes them, F-220 reads the lock), `corpus/manifest.tsv` and `tools/oracle/unsupported.json` (no other S46 story is known to write them), `examples/folder-viewer/src/preview.rs`, `use.html`, `more.html` and `README.md`, which every viewer story writes. The library half (steps 1 to 6 and the oracle files) can run in a worktree in parallel with the viewer stories. The `preview.rs`, `use.html`, `more.html` and `README.md` edits serialise after F-215, the last viewer story in F-217's recorded order, and after F-225 if F-225 has already taken those files. AC24 needs the oracle host, Chromium under SwiftShader, and runs alone on it. Network: none, the fixtures use the locked environment.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-codec
bin/ocelli.sh test ocelli-codec --test jpegxl
bin/ocelli.sh test ocelli-codec --test registry
bin/ocelli.sh test ocelli-pixel --test color
bin/ocelli.sh test ocelli-dicom --test frame_open
bin/ocelli.sh clippy
python3 crates/ocelli-codec/tests/fixtures/generate_jpegxl.py
bin/ocelli.sh gate pins licences provenance corpus corpus-tests ci prose deviations
bin/ocelli.sh native
bin/ocelli.sh wasm && bin/ocelli.sh wasm-decode
bin/ocelli.sh gate oracle
```

Final profile: `bin/ocelli.sh gate --sprint`. Identities: `jxl-oxide` 0.12.6, `jxl-render` 0.12.4, imagecodecs 2026.10.10 with libjxl 0.12.0, cornerstone3D 5.8.2, DICOM 2026d. Invalidated by any lock change, an imagecodecs or libjxl version change (fixture bytes and the AC10 bound), or any change to `jpegxl.rs`, `jpeg.rs` or `sample_convert.rs`. AC24 is open until the oracle run exists. AC20 is open until F-220 integrates.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. Decode is in the process or worker that owns the bytes
- Render-loop allocation: none. Decode is load-time work, and its allocation is bounded by decision 11 under the D-21 amendment
- unsafe: none in this repository. The dependency's unsafe is audited under AC25
- Tier A (WebGPU): full. Decode is CPU work and hands the same stored bytes to every tier
- Tier B (WebGL2): full, the same bytes
- Tier C (CPU): full, the same bytes. No wasm module runs the decoder in this story (fact 3), so wasm32 execution is argued, not observed (AC21)

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC9 quantiser, hand-computed, citing PS3.5 8.2.15 for Bits Stored and the codestream's precedence | `crates/ocelli-codec/tests/jpegxl.rs` |
| conformance | AC4 to AC8 and AC10 to AC13, each syntax against the uncompressed synthetic reference or its JPEG source | `crates/ocelli-codec/tests/jpegxl.rs` |
| fixture | AC14 and AC15, XYB codec ownership and one conversion, citing PS3.3 C.7.6.3.1.2 | `crates/ocelli-pixel/tests/color.rs`, `crates/ocelli-dicom/tests/frame_open.rs` |
| unit | AC1 to AC3, catalogue and registration | `crates/ocelli-codec/tests/registry.rs` |
| property | AC16 malformed input, deterministic mutation | `crates/ocelli-codec/tests/jpegxl.rs` |
| conformance | AC22 and AC23, corpus rows and Python read-back | `scripts/tests/test_corpus_synth.py`, `gate corpus` |
| golden | AC24, declared unsupported, no reference frame exists | `tools/oracle` |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` row "Transfer syntaxes | ~13" is the cornerstone3D surface, and cornerstone3D 5.8.2 decodes no JPEG XL syntax (fact 12). This is a capability beyond parity, and the oracle rows are declared unsupported.

## Deviations

- **D-21, amended.** Its list of adapters that may allocate bounded library-owned storage gains JPEG XL, with `jxl-oxide`'s grids, the reconstructed JPEG `Vec` and decision 11's `AllocTracker` bound as the reason. Written into the register with this approval on 2026-11-05, and F-229 added to its Raised column.
- **D-28, the JPEG XL dependency.** Section 15.2 routes codecs through dicom-rs features. This story adds `jxl-oxide =0.12.6` and `jxl-render =0.12.4` directly, with default features off, because dicom-rs's route is its inventory registry, which section 21 says is unavailable on wasm, the same reason D-20 and D-22 record for their packages. Written into the register with this approval on 2026-11-05.

## LLD impact

- `docs/lld/codecs.md`: the sixth family, the three modes, the integer and float paths, the quantiser, the `.111` reconstruction route, the refusals and their reasons, the `AllocTracker` bound, lossy modular not detected, signed samples refused, and the forward wasm cost of fact 4 with the `alloc-stdlib` note for F-220.
- `docs/lld/corpus.md`: the five rows, the imagecodecs and libjxl producer, the read-back routes.
- `docs/lld/oracle.md`: the five declared-unsupported rows.
- `docs/lld/pixel-pipeline.md` and `docs/lld/frame-open.md`: `XYB` as a codec-owned colour space.

## User-facing documentation impact

- `README.md`: wherever it lists what the viewer opens, JPEG XL lossless, JPEG recompression and lossy, with signed samples, ICC and HDR reported as unavailable.
- `examples/folder-viewer/src/use.html`: the registry section's "sixteen UIDs" and "Five functions register the five families" become nineteen and six, the code block gains `register_jpegxl_decoders`, the table gains the three rows with family "JPEG XL", and "Fifteen are available" becomes eighteen.
- `examples/folder-viewer/src/more.html`: one paragraph on JPEG XL, that cornerstone3D 5.8.2 cannot decode it so the oracle has no reference for it, and that correctness here comes from exact lossless round trips against synthetic sources.
- `CLAUDE.md`, "Current state": "covering all sixteen transfer syntaxes", "five decoder families" with "Six names, five families", and "Fifteen of the sixteen known transfer syntaxes are available" become nineteen, six families and seven names, and eighteen of nineteen, with `crates/ocelli-codec/tests/registry.rs` still the source that asserts them.

## Open questions

None open. Each is a lead decision under the operator's delegation of 2026-11-05.

- Q1, the E-ID. Answered: lead decision under the operator's delegation of 2026-11-05. E4.10, skipping E4.9 for the recorded F-145 defect.
- Q2, signed samples. Answered: lead decision under the operator's delegation of 2026-11-05. Refused as unavailable, decision 4, with a follow-up when a reference signed file exists.
- Q3, `.111`. Answered: lead decision under the operator's delegation of 2026-11-05. Reconstruct and decode through the `.50` path, decision 8.
- Q4, ICC, HDR, orientation, extra channels. Answered: lead decision under the operator's delegation of 2026-11-05. All refuse as unavailable, decision 6.
- Q5, the lossy quantiser. Answered: lead decision under the operator's delegation of 2026-11-05. Decision 3, round half up on the non-negative scaled value, clamped.
- Q6, the fixture encoder and its licence. Answered: lead decision under the operator's delegation of 2026-11-05. imagecodecs 2026.10.10 with libjxl 0.12.0, BSD-3-Clause, already pinned and licence-checked by F-217, tooling only.
- Q7, order against F-220. Answered: lead decision under the operator's delegation of 2026-11-05. After F-220 integrates, which is safe in either order because the shipped closure does not change (fact 3).
- Q8, XYB. Answered: lead decision under the operator's delegation of 2026-11-05. A codec-owned `ocelli-pixel` variant, decision 10.
- Q9, a panic in the dependency. Answered: lead decision under the operator's delegation of 2026-11-05. AC16 is the test, and a panic found there stops the story and goes to the operator with options, decision 12. This is a conditional escalation, not an open question today.

### Operator answer after the robustness stop

Given by the operator on 2026-11-05, after `malformed_jpegxl_never_panics` found that a bit-flipped `.111` codestream reaches an unconditional `expect("is_last not found")` at `jxl-jbr` 0.2.1 `src/reconstruct.rs:457`, in the DHT branch of JPEG reconstruction, which under `panic = "abort"` ends the process.

- **`.110` and `.112` ship. `.111` is reported unavailable.** JPEG XL JPEG Recompression is registered as a known syntax that is unavailable, with a reason naming the upstream defect, and no Ocelli path calls `reconstruct_jpeg` or any other `jxl-jbr` entry point. A `.111` frame is refused before any byte reaches `jxl-oxide`, the same way Deflated is unavailable today. ACs that required a `.111` decode are replaced by AC7b: a `.111` frame, intact or malformed, returns the unavailable refusal and never panics.
- **The corruption scan is complete before the story ships.** `malformed_jpegxl_never_panics` runs every truncation and every fixed-seed bit flip over every `.110` and `.112` fixture, not stopping at the first panic, and must pass. A panic on any of them stops the story again.
- **The upstream defect is filed** with jxl-oxide by the sprint lead as https://github.com/tirr-c/jxl-oxide/issues/512, and a later story turns `.111` on once a released version fixes it.

### Lead amendments after review pass 2

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-229-working-pass-2.md`, which found that the hand-written ICC pre-check read two fields differently from `jxl-oxide` and so still let a 29-byte input reach a 256 MiB allocation.

- **The pre-check refuses on the ICC flag, not on a size.** Every codestream that declares an embedded ICC profile is refused anyway (AC13, `UnsupportedPixelFormat`), so the pre-check refuses as soon as it reads `want_icc`, with no size comparison.
- **Prefer the library's own header parser.** If `jxl-image`'s `ImageHeader::parse` reads the header up to and including `want_icc` without allocating the profile, the pre-check calls it and the hand-written walker is deleted. `jxl-image` is then pinned exactly beside `jxl-oxide` and `jxl-render`, and D-28 names it. If it cannot be used without the allocation, the walker stays and a seeded differential test against `ImageHeader::parse` must show zero disagreements on whether `want_icc` is set.
- **The pre-check fails closed.** Any header it cannot read to `want_icc`, a container box whose size runs past the input included, is refused before `jxl-oxide` sees a byte.

### Lead amendments after review pass 3

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-229-working-pass-3.md`, which found that a 1,129 byte container with a compressed `brob` box makes `jxl-oxide` allocate 1 GiB outside the `AllocTracker` and then decode successfully.

- **`jxl-oxide` receives the codestream alone.** For a container, the pre-check extracts the codestream from the `jxlc` box or the `jxlp` boxes in order, and passes only those bytes. No other box (`brob`, `jbrd`, `Exif`, `xml `, `jumb` or any other) ever reaches `jxl-oxide`. A container with no codestream, two `jxlc` boxes, a `jxlc` beside `jxlp` boxes, or `jxlp` boxes out of order or missing the last flag is refused before decode.
- **A header that declares extra channels is refused in the pre-check.** Extra channels are refused later anyway, so the pre-check refuses a non-empty `ec_info` with `UnsupportedPixelFormat` before any parse that could grow with the header. `read_header` also grows its window geometrically rather than one byte at a time, so its cost is linear in the header length.
- **`jxl-image` is not a direct dependency.** `jxl-oxide` re-exports the crate, so only `jxl-bitstream` and `jxl-oxide-common` stay direct, and D-28 says so.

### Lead rulings after review pass 7

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-229-working-pass-7.md`.

- **The scanner's string rule is fixed in this story.** `scripts/unsafe_allowlist_check.py`'s string pattern must not treat a Rust string continuation as the end of a string, so code after it is scanned. A test pins it, and the fix must leave every tracked file's count unchanged.
- **The audit's scope is a named commit.** The section names base commit `4165d1e8` rather than a merge-base command, and says that neither `ocelli-wasm` nor `ocelli-native` reaches `ocelli-codec`, so the shipped modules do not carry this closure today.

### Lead rulings after review pass 8

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-229-working-pass-8.md`. The sentence naming which builds compile the closure for wasm32 is deleted, because no build passes `+simd128` to the closure's crates and step 2 of `bin/ocelli.sh native` also compiles it. The scanner's remaining blind spots, after a `'"'` character literal, a raw string holding a quote and a line comment with an unpaired quote, predate this story and change no audit count, so they leave F-229 and go to their own story with a real lexer, recorded for the operator.
