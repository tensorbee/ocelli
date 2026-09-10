# F-024, JPEG baseline, extended, and lossless

**Status**: approved
**Epic ref**: E4.2
**Sprint**: S08
**Estimate**: 3w

## Normative source, transcribed

### `docs/hld/18-codec-registry.md`, section 21

```rust
pub trait Decoder: Send + Sync {
    fn transfer_syntaxes(&self) -> &'static [&'static str];
    /// Decode one frame into `out`. Must not allocate.
    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8])
    -> Result<(), CodecError>;
}
pub struct Registry { by_ts: HashMap<&'static str, Arc<dyn Decoder>> }
```

Deviation D-19 makes registration result-returning, atomic, and collision
refusing while preserving this decoder contract.

### `docs/hld/12-workspace-and-build.md`, section 15.2

> Disable default features. The defaults are rayon and simd, and the gdcm
> feature is native-only. On wasm you want: default-features = false, then
> jpeg, rle, deflate and openjp2 selected explicitly.

### DICOM PS3.5 Annex F and transfer syntax UIDs

| UID | Process |
|-----|---------|
| `1.2.840.10008.1.2.4.50` | JPEG Baseline, process 1, 8-bit |
| `1.2.840.10008.1.2.4.51` | JPEG Extended, processes 2 and 4, commonly 12-bit |
| `1.2.840.10008.1.2.4.57` | JPEG Lossless, process 14 |
| `1.2.840.10008.1.2.4.70` | JPEG Lossless, process 14, selection value 1 |

Encapsulated frame fragments are reassembled before this decoder boundary.
The decoder writes one frame with the exact rows, columns, sample count, and
container width declared by `FrameDesc`.

### `docs/hld/22-testing-and-tolerance.md`, section 25

| **Layer** | **What it proves** | **Where it comes from** |
|----|----|----|
| Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |

## What the specification does not cover

The HLD does not define JPEG library output layout, colour-transform ownership,
allocation adaptation, codestream termination, or behavior when decoder
capability is narrower than a DICOM UID.

Measured dependency fact: `jpeg-decoder` 0.3.2 supports process 14 lossless up
to 16-bit and ordinary 8-bit DCT JPEG. Its decoder rejects non-lossless sample
precision other than 8-bit. The S08 corpus row for `.51` is 12-bit. Therefore
the story's prescribed crate cannot decode its own extended corpus evidence.
The operator approved `oxideav-mjpeg` 0.1.8 with default features disabled for
the `.51` path. Its standalone decoder explicitly supports 12-bit SOF1,
contains no C dependency, carries an MIT licence file, and compiles for native
and `wasm32-unknown-unknown` in the S08 design probe. It is used only where
`jpeg-decoder` cannot meet the DICOM precision contract.

## Approach

1. Pin `jpeg-decoder` with default features disabled so rayon is absent on
   native and wasm.
2. Pin `oxideav-mjpeg` 0.1.8 with default features disabled for 12-bit `.51`.
   Add one `JpegDecoder` adapter for every UID its complete dependency set can
   honestly decode. Register no UID until its corpus row passes.
3. Decode into library-owned temporary output because both selected safe APIs
   return owned frame storage. Validate metadata and exact length first, then
   copy into the caller buffer only after complete success. D-21 records this
   bounded decode-worker allocation rather than claiming the HLD section 21
   no-allocation sentence is met.
4. Validate JPEG width, height, component count, precision, pixel format, and
   complete codestream termination against `FrameDesc`. Refuse mismatch before
   modifying the caller output.
5. Record whether colour conversion already occurred. JPEG YCbCr decoded to
   RGB is labelled RGB output so no later stage converts it again.
6. Compare `.50`, `.51`, `.57`, and `.70` decoded sample buffers against the
   uncompressed synthetic ramp or independently decoded truth. Lossless rows
   require exact equality. The baseline colour row uses its declared class-two
   measurement rather than a fabricated exact original.
7. Add the first real `decode.frame` benchmark runner and baseline only after
   at least one production adapter exists.
8. Mutate one decoded sample and one colour-output label. Run the conformance
   and double-conversion checks red and revert.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Decode is worker-side. Decoder scratch is
  pre-sized at construction
- unsafe: none
- Tier A (WebGPU): n/a. Decode is CPU work before rendering
- Tier B (WebGL2): n/a. The same decoder is used
- Tier C (CPU): n/a. The same decoder is used

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| conformance | Each registered JPEG UID decodes its manifest-backed corpus frame to independently established truth | `crates/ocelli-codec/tests/jpeg_corpus.rs` |
| fixture | Process 14 and SV1 preserve hand-computed 12-bit sample extrema and exact output byte order, citing PS3.5 Annex F | `crates/ocelli-codec/tests/jpeg.rs` |
| fixture | Baseline colour output is labelled once as RGB after decoder conversion | `crates/ocelli-codec/tests/jpeg.rs` |
| unit | Truncated, trailing, dimension-mismatched, precision-mismatched, and wrong-length output inputs fail without partial caller-buffer mutation | module tests |
| mutation | A sample change and colour-label change make named tests fail | feature review evidence |
| benchmark | `decode.frame` measures one registered decoder over one corpus frame with setup excluded | `tools/bench/src/runners/decode_frame.rs` |
| cross-target | Every registered UID builds and runs through the same Rust adapter on native and wasm | `bin/ocelli.sh check ocelli-codec` and `bin/ocelli.sh wasm` |

## Parity surface covered

Appendix B `Transfer syntaxes`: `.50`, `.51`, `.57`, and `.70`, but only after
each exact UID has conformance evidence. Unsupported precision cannot be
reported as available.

## Deviations

- D-21 records bounded library-owned decode output before the atomic copy into
  the caller buffer.

## LLD impact

- Update `docs/lld/codecs.md` with JPEG capability, colour ownership,
  termination, scratch allocation, and exact conformance evidence.
- Update `docs/lld/benchmarks.md` and `docs/lld/README.md`.

## Write set

- `Cargo.toml` and `Cargo.lock`
- `crates/ocelli-codec/Cargo.toml`
- `crates/ocelli-codec/src/lib.rs`
- new JPEG source and test files under `crates/ocelli-codec/`
- `tools/bench/` decoder runner and baseline files
- `docs/lld/codecs.md`
- `docs/lld/benchmarks.md`
- `docs/lld/README.md`
- shared sprint ledgers during completion

## Open questions

None. The operator approved a second permissive pure-Rust decoder for `.51`.
`oxideav-mjpeg` 0.1.8 is the selected candidate after provenance and native
plus wasm compile checks.
