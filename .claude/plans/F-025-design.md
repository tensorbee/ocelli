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
byte first for each sample, then by sample. Each segment uses PackBits control
bytes and must terminate exactly at its declared boundary.

Deflate applies to the encoded dataset, not to one image frame. F-016 already
implements that strict dataset-level path in `ocelli-dicom`.

## What the specification does not cover

The HLD groups `deflate` with codec features but does not reconcile the
dataset-level Deflated Explicit VR syntax with the frame-level `Decoder`
trait. This plan does not create a false frame decoder for it. F-025 verifies
and documents F-016's existing strict Deflate path, while registering raw
native pixel and RLE frame decoders in `ocelli-codec`.

Raw native transfer syntaxes share behavior but preserve exact UID dispatch.
Big-endian input is normalized to little-endian decoded container bytes, which
is the canonical output expected by later stored-value extraction. No decoder
interprets Bits Stored or applies sign extension. That remains owned by
`ocelli-pixel`.

## Approach

1. Add a raw decoder registered for implicit and explicit little endian. It
   validates exact source and output lengths before one complete copy.
2. Add a big-endian raw decoder for the retired explicit big-endian UID. For
   multi-byte samples it swaps each complete sample container into canonical
   little endian after full preflight.
3. Add a DICOM RLE decoder with a fixed 64-byte header parser. Validate segment
   count, required byte planes, monotonic offsets, source bounds, and exact
   PackBits expansion for every plane before committing output.
4. Decode RLE into setup-owned scratch, then interleave byte planes into the
   caller output only after every segment succeeds. Reject surplus decoded
   bytes, truncated literals, truncated repeats, impossible no-op placement,
   missing planes, and trailing compressed bytes within a declared segment.
5. Keep signedness as bytes. F-018 interprets signed stored samples once after
   decoding, identically for raw and compressed syntax.
6. Re-run the existing F-016 Deflate fixtures and corpus row. Add an integration
   assertion that registry capability reports the dataset syntax as handled by
   ingest rather than as a frame decoder.
7. Compare raw LE, raw BE, and RLE corpus rows byte-for-byte with the synthetic
   uncompressed reference after canonicalization.
8. Mutate one endian swap and one RLE segment order. Run named tests red and
   revert.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Decode is worker-side and scratch is pre-sized
- unsafe: none
- Tier A (WebGPU): n/a. Decode is CPU work before rendering
- Tier B (WebGL2): n/a. The same decoder is used
- Tier C (CPU): n/a. The same decoder is used

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Raw little-endian and big-endian 16-bit signed containers canonicalize to the same bytes, citing PS3.5 Annex A | `crates/ocelli-codec/tests/native.rs` |
| fixture | RLE segment offsets, MSB-first byte planes, sample ordering, literal and repeat runs reconstruct hand-computed pixels, citing PS3.5 Annex G | `crates/ocelli-codec/tests/rle.rs` |
| conformance | Manifest-backed raw LE, raw BE, RLE, and Deflate rows match the independent synthetic truth | codec and DICOM corpus tests |
| unit | Every truncated, non-monotonic, overlong, underlong, and wrong-output case fails without partial caller-buffer mutation | module tests |
| mutation | Endian and segment-order changes make named fixtures fail | feature review evidence |
| cross-target | Raw and RLE adapters plus the existing Deflate ingest compile on native and wasm | crate checks and `bin/ocelli.sh wasm` |

## Parity surface covered

Appendix B `Transfer syntaxes`: implicit LE, explicit LE, deflated explicit LE,
explicit BE, and RLE Lossless. Deflate capability is owned by ingest rather
than registered behind the frame-decoder trait.

## Deviations

- D-18 already records the direct strict `flate2` Deflate path in F-016.
- No new deviation is planned.

## LLD impact

- Update `docs/lld/codecs.md` with raw normalization, RLE ordering, atomic
  output, and the dataset-versus-frame Deflate ownership line.
- Update `docs/lld/dicom-ingest.md` and `docs/lld/README.md`.

## Write set

- `crates/ocelli-codec/src/lib.rs`
- new native and RLE source and test files under `crates/ocelli-codec/`
- `docs/lld/codecs.md`
- `docs/lld/dicom-ingest.md`
- `docs/lld/README.md`
- shared sprint ledgers during completion

## Open questions

None.
