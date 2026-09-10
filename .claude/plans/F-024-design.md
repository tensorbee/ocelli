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

PS3.5 A.4 requires even-length Fragment Item Values. For JPEG it permits FF
fill bytes inside the compressed stream before a marker so EOI ends on an even
boundary, or the single trailing NULL required to pad an odd-length stream
after EOI.

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
the `.51` path. The published crate does decode 12-bit SOF1 internally, but
the documented standalone entry point is `pub(crate)` in version 0.1.8. The
smallest public route is its `registry` feature and
`oxideav_mjpeg::registry::make_decoder`. The operator approved that explicit
feature after native and `wasm32-unknown-unknown` probes passed. It resolves
`oxideav-core` 0.1.35 through the lockfile. The dependency contains no C code,
carries an MIT licence file, and is used only where `jpeg-decoder` cannot meet
the DICOM precision contract.

The approved colour-ownership intent also contradicted the immutable HLD
signature above. `decode` cannot mutate `FrameDesc` or return a changed colour
description. The compatible adaptation is a compact typed
`DecodePhotometricInterpretation` query on `Decoder`, defaulting to
`Preserved`, with Registry forwarding. JPEG returns `Rgb` when its dependency
has converted a three-sample colour frame. Raw, RLE, and JPEG 2000 adapters
inherit the preservation default without changing their decode signature.

## Approach

1. Pin `jpeg-decoder` with default features disabled so rayon is absent on
   native and wasm. Use it for `.50`, `.57`, `.70`, and `.51` process 2 at
   eight-bit precision.
2. Pin `oxideav-mjpeg` 0.1.8 with default features disabled and only its
   `registry` feature enabled for 12-bit `.51`. Resolve `oxideav-core` 0.1.35
   exactly in the lockfile. Add one configured `JpegDecoder` value per exact
   UID. Register no UID until its fixture evidence passes.
3. Decode into library-owned temporary output because both selected safe APIs
   return owned frame storage. The public `oxideav-core` packet API also
   requires one bounded encoded-input copy for twelve-bit `.51`. Validate
   metadata and exact length first, then copy into the caller buffer only after
   complete success. D-21 records both decode-worker allocations rather than
   claiming the HLD section 21 no-allocation sentence is met.
4. Validate JPEG width, height, component count, precision, pixel format, and
   complete codestream termination against `FrameDesc`. Accept PS3.5 A.4 FF
   marker fill and the single necessary NULL after EOI, then remove that
   external pad before dependency decode. Refuse excess or non-padding trailing
   data before modifying the caller output.
5. Record whether colour conversion already occurred. JPEG YCbCr decoded to
   RGB is labelled RGB so a downstream consumer can avoid converting it again.
   The query reports this fact but does not require consumers to use it.
6. Compare `.50`, `.51`, `.57`, and `.70` decoded sample buffers against the
   uncompressed synthetic ramp or independently decoded truth. Lossless rows
   require exact equality. The baseline colour row uses its declared class-two
   measurement rather than a fabricated exact original.
7. Add the first real `decode.frame` benchmark runner and baseline only after
   at least one production adapter exists.
8. Mutate one decoded sample and one colour-output label. Run the conformance
   and output-description checks red and revert.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Decode is worker-side. Both dependency decoder
  values and their bounded owned buffers are created inside each decode call
- unsafe: none
- Tier A (WebGPU): n/a. Decode is CPU work before rendering
- Tier B (WebGL2): n/a. The same decoder is used
- Tier C (CPU): n/a. The same decoder is used

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| conformance | Each registered JPEG UID decodes synthetic or corpus-derived bytes to hand-computed or independently established truth | `crates/ocelli-codec/tests/jpeg.rs` |
| fixture | Process 14 and SV1 preserve hand-computed 12-bit sample extrema and exact output byte order, citing PS3.5 Annex F | `crates/ocelli-codec/tests/jpeg.rs` |
| fixture | Baseline colour output is reported as RGB and computes fixed class-two statistics against the synthetic corpus reference | `crates/ocelli-codec/tests/jpeg.rs` |
| fixture | JPEG Extended process 2 at 8-bit decodes against independent DCMTK truth | `crates/ocelli-codec/tests/jpeg.rs` |
| unit | PS3.5 A.4 FF fill and one necessary NULL after EOI decode correctly, while truncation, excess or non-padding trailing data, dimension mismatch, precision mismatch, and wrong output length fail without partial caller-buffer mutation | `crates/ocelli-codec/tests/jpeg.rs` |
| mutation | A sample change and colour-label change make named tests fail | feature review evidence |
| benchmark | `decode.frame` measures one registered decoder over one synthetic corpus frame with setup excluded | `crates/ocelli-codec/examples/decode_frame.rs` and `tools/bench/src/runners/decode_frame.mjs` |
| cross-target | Every registered JPEG adapter configuration compiles from the same Rust source for native and `wasm32-unknown-unknown` | `bin/ocelli.sh gate native` and `bin/ocelli.sh cargo check -p ocelli-codec --all-targets --target wasm32-unknown-unknown` |

## Parity surface covered

Appendix B `Transfer syntaxes`: `.50`, `.51`, `.57`, and `.70`, but only after
each exact UID has conformance evidence. Unsupported precision cannot be
reported as available.

## Deviations

- D-21 records bounded library-owned decode output before the atomic copy into
  the caller buffer.
- `oxideav-mjpeg` needs its explicit `registry` feature because the published
  standalone decode function in 0.1.8 is not public. This expands the graph to
  `oxideav-core` 0.1.35, whose published source contains eleven audited unsafe
  tokens in arena support, four unsafe implementations and seven unsafe
  blocks. The audit matches whole-word `unsafe` over every `*.rs` file below
  the exact package's `src/` directory. The dependency's unsafe is not
  repository unsafe.

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

None. The operator approved a second permissive pure-Rust decoder for `.51`
and the public `registry` route its published version requires.
`oxideav-mjpeg` 0.1.8 with `oxideav-core` 0.1.35 is the selected candidate
after provenance and native plus wasm compile checks.
