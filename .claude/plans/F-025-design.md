# F-025, RLE, Deflate, and raw little-endian and big-endian

**Status**: approved
**Epic ref**: E4.3
**Sprint**: S08
**Estimate**: 2w

## Normative source, transcribed

### `docs/hld/18-codec-registry.md`, section 21

```rust
pub trait Decoder: Send + Sync {
    fn transfer_syntaxes(&self) -> &'static [&'static str];
    /// Decode one frame into `out`. Must not allocate.
    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8])
    -> Result<(), CodecError>;
}
```

### `docs/hld/12-workspace-and-build.md`, section 15.2

> Disable default features. The defaults are rayon and simd, and the gdcm
> feature is native-only. On wasm you want: default-features = false, then
> jpeg, rle, deflate and openjp2 selected explicitly.

### DICOM PS3.5 Annex A and Annex G

| UID | Encoding responsibility |
|-----|-------------------------|
| `1.2.840.10008.1.2` | Implicit VR Little Endian dataset and native little-endian Pixel Data |
| `1.2.840.10008.1.2.1` | Explicit VR Little Endian dataset and native little-endian Pixel Data |
| `1.2.840.10008.1.2.1.99` | Deflated Explicit VR Little Endian dataset |
| `1.2.840.10008.1.2.2` | Explicit VR Big Endian dataset and native big-endian Pixel Data |
| `1.2.840.10008.1.2.5` | RLE Lossless encapsulated Pixel Data |

DICOM RLE starts with a 64-byte header containing a segment count and fifteen
32-bit little-endian offsets. Segments are byte planes ordered most-significant
byte first for each sample, then by sample. Each image row is encoded
separately. PackBits control byte `0x80` is a legal zero-output no-op. Each RLE
segment has even physical length, with exactly one trailing zero pad when its
encoded run stream would otherwise be odd. The pad is not another PackBits
run. PS3.5 Table 8.2.2-1 permits unsigned RGB and YBR_FULL with three samples
per pixel at either eight or sixteen Bits Allocated.

Native Pixel Data with VR `OB` is byte-order insensitive. Native Pixel Data
with VR `OW` is a sequence of 16-bit words whose bytes follow the transfer
syntax byte order, including when Bits Allocated is 8 or 32. A native Pixel
Data Value has even physical length and uses one trailing zero byte when its
logical sample bytes would otherwise be odd under OB. Under OW, the complete
Value consists of physical 16-bit words and unused bits in its final word are
insignificant rather than required to be zero. PS3.5 8.1.1 requires native
multi-frame Values to concatenate Frames without per-frame padding. With one
Bit Allocated, a later Frame may therefore start in the middle of a byte or
word. PS3.5 8.2 packs Pixel Cells least-significant bit first through each byte
or word.

Deflate applies to the encoded dataset, not to one image frame. F-016 already
implements that strict dataset-level path in `ocelli-dicom`.

## What the specification does not cover

The HLD groups `deflate` with codec features but does not reconcile the
dataset-level Deflated Explicit VR syntax with the frame-level `Decoder`
trait. This plan does not create a false frame decoder for it. F-025 verifies
and documents F-016's existing strict Deflate path, while registering raw
native pixel and RLE frame decoders in `ocelli-codec`.

Raw native transfer syntaxes preserve exact UID dispatch with one decoder value
per UID. That distinction is necessary because `Decoder::decode` does not
receive the selected UID. `FrameDesc` retains typed `OB` or `OW` Pixel Data VR
evidence. Implicit VR Little Endian requires `OW`. Explicit VR native paths may
use `OB` for eight-bit Pixel Data or `OW`. Big-endian `OW` input is normalized
one physical 16-bit word at a time to little-endian bytes, while `OB` bytes are
unchanged. This also distinguishes legal 8-bit big-endian `OB` from 8-bit
big-endian `OW`. No decoder interprets Bits Stored or applies sign extension.
That remains owned by `ocelli-pixel`.

`Decoder::decode` consumes one logical frame and cannot validate or remove
whole-Value padding. A fallible `NativeFrameIndex`, constructed through the
exact-UID `RawDecoder`, owns complete native Value validation and Number of
Frames evidence. It retains the exact global bit stride and performs
allocation-free frame extraction with value-level OW byte ordering. This
keeps mid-byte one-bit frame starts and odd-byte OW frame boundaries from being
guessed from a slice.

Decoded sample layout is reported separately from Photometric Interpretation.
Raw output preserves its input layout. RLE output is interleaved. The F-024
JPEG adapters also report interleaved output, matching the packed buffers they
already return.

## Approach

1. Add separate raw decoder values for implicit and explicit little endian so
   each retains its exact UID at decode time. One-frame decode validates exact
   logical frame and output lengths but never infers complete-Value padding.
   Implicit VR requires `OW`. Explicit VR retains its legal eight-bit `OB`
   option.
2. Add a big-endian raw decoder for the retired explicit big-endian UID. `OB`
   bytes remain ordered. Logical `OW` frames already isolated on a Value word
   boundary are normalized by physical 16-bit word after full preflight,
   regardless of whether Bits Allocated is 8, 16, or 32. Direct decode refuses
   an odd logical source length because it cannot contain complete physical OW
   words. Odd-byte or non-byte-aligned frame boundaries use the complete-Value
   index in step 3.
3. Add a fallible allocation-free native Value index through `RawDecoder`.
   Validate at least one Frame and the exact complete Value length. Require OB
   whole-Value NULL padding only when necessary. Require OW complete physical
   words while ignoring unused final-word bits. Extract any requested frame by
   its checked global bit offset, canonicalizing big-endian OW at the Value
   word boundary and repacking one-bit output least-significant bit first.
4. Add a DICOM RLE decoder with a fixed 64-byte header parser. Validate segment
   count, required byte planes, monotonic offsets, source bounds, and exact
   PackBits expansion for every plane before committing output. Validate the
   complete PS3.5 Table 8.2.2-1 Photometric Interpretation, Samples per Pixel,
   Bits Allocated, and Pixel Representation combination before parsing input.
5. Decode RLE in two allocation-free passes. The first pass validates every
   row, run, byte count, segment boundary, and necessary zero pad without
   writing. The second repeats the proven traversal and writes interleaved,
   little-endian sample bytes directly into caller output. Accept `0x80` as a
   zero-output no-op. Reject surplus decoded bytes, cross-row runs, truncated
   literals, truncated repeats, missing planes, bad padding, and trailing
   compressed bytes.
6. Keep signedness as bytes. F-018 interprets signed stored samples once after
   decoding, identically for raw and compressed syntax.
7. Re-run the existing F-016 Deflate fixtures and corpus row. Add an integration
   assertion that the frame registry reports the syntax as `KnownUnavailable`
   while `ocelli-dicom` reports `DispatchPath::DeflatedExplicitVrLittleEndian`.
8. Compare raw LE, raw BE, and RLE corpus rows byte-for-byte with the synthetic
   uncompressed reference after canonicalization.
9. Mutate one endian swap and one RLE segment order. Run named tests red and
   revert.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Raw and RLE decoding allocate nothing per call
- unsafe: none
- Tier A (WebGPU): n/a. Decode is CPU work before rendering
- Tier B (WebGL2): n/a. The same decoder is used
- Tier C (CPU): n/a. The same decoder is used

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Raw little-endian and big-endian 16-bit signed containers canonicalize to the same bytes, and distinct 8-bit big-endian `OB` and `OW` inputs normalize correctly, citing PS3.5 Annex A and Annex D | `crates/ocelli-codec/tests/native.rs` |
| fixture | A complete two-frame native Value has one whole-Value boundary, one-bit later Frames begin mid-byte, OB checks its necessary NULL, OW accepts nonzero insignificant final-word bits, and big-endian OW extraction uses the Value word boundary, citing PS3.5 sections 8.1.1 and 8.2 | `crates/ocelli-codec/tests/native.rs` |
| fixture | RLE segment offsets, MSB-first byte planes, sample ordering, positive six-segment RGB16 and YBR_FULL16, row boundaries, legal no-op, literal and repeat runs, and exact necessary segment padding reconstruct hand-computed pixels, citing PS3.5 Annex G and Table 8.2.2-1 | `crates/ocelli-codec/tests/rle.rs` |
| conformance | Manifest-backed raw LE, raw BE, RLE, and Deflate rows match the independent synthetic truth | `crates/ocelli-dicom/tests/corpus.rs` |
| unit | Every truncated, non-monotonic, overlong, underlong, cross-row, bad-padding, invalid Table 8.2.2-1 combination, unsupported one-bit or 32-bit RLE, and wrong-output case fails without partial caller-buffer mutation | module tests |
| mutation | Endian and segment-order changes make named fixtures fail | feature review evidence |
| cross-target | Raw and RLE adapters plus existing Deflate ingest compile on native and wasm | `bin/ocelli.sh gate native` plus direct wasm32 checks of `ocelli-codec` and `ocelli-dicom` |

## Parity surface covered

Appendix B `Transfer syntaxes`: implicit LE, explicit LE, deflated explicit LE,
explicit BE, and RLE Lossless. Deflate capability is owned by ingest rather
than registered behind the frame-decoder trait. The current RLE output contract
reports byte-interleaved sample containers and has no typed bit-packed layout,
so legal one-bit RLE reports `UnsupportedPixelFormat`. RLE with 32-bit
containers also reports
`UnsupportedPixelFormat`, because PS3.5 section 8.2.2 permits standard
photometric RLE only for one, eight, or sixteen allocated bits.

## Deviations

- D-18 already records the direct strict `flate2` Deflate path in F-016.
- No new deviation is planned.

## LLD impact

- Update `docs/lld/codecs.md` with raw normalization, RLE ordering, atomic
  output, and the dataset-versus-frame Deflate ownership line.
- Update `docs/lld/dicom-ingest.md` and `docs/lld/README.md`.

## Write set

- `crates/ocelli-codec/src/lib.rs`
- `crates/ocelli-codec/src/registry.rs`
- `crates/ocelli-codec/src/jpeg.rs` and its affected tests
- new native and RLE source and test files under `crates/ocelli-codec/`
- `crates/ocelli-dicom/tests/corpus.rs`
- `docs/lld/codecs.md`
- `docs/lld/dicom-ingest.md`
- `docs/lld/README.md`
- shared sprint ledgers during completion

## Open questions

None.
