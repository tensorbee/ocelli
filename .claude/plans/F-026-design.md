# F-026, JPEG 2000 via openjp2, validated against the corpus

**Status**: approved
**Epic ref**: E4.4
**Sprint**: S08
**Estimate**: 4w

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

> On wasm you want: default-features = false, then jpeg, rle, deflate and
> openjp2 selected explicitly.

### DICOM PS3.5 Annex A

| UID | Process |
|-----|---------|
| `1.2.840.10008.1.2.4.90` | JPEG 2000 Image Compression, lossless only |
| `1.2.840.10008.1.2.4.91` | JPEG 2000 Image Compression |

Lossless output requires exact stored samples. Lossy output is measured under
the fixed tolerance policy and is not described as bit-exact.

### Resolved Appendix A evidence

`docs/spikes/A1-htj2k-openjp2.md` records that `openjp2` 0.6.1 does not link as
published for `wasm32-unknown-unknown`. With an allocator shim it traps on both
HTJ2K and JPEG 2000 Part 1 control codestreams. It also records a null pointer
reaching deallocation on native. `docs/sprints/AS_BUILT.md` states that the
crate is not recommended natively either.

## What the specification does not cover

The normative HLD and this story prescribe `openjp2`, while already completed
repository evidence proves that exact crate cannot satisfy the required wasm
target and has an unresolved native undefined-behavior defect. Completing the
story as written would contradict both the cross-target acceptance and the
resolved spike.

The operator approved a pure-Rust replacement. `ritk-codecs` 0.6.0 is selected
because it implements reversible 5/3 and irreversible 9/7 JPEG 2000, carries
MIT or Apache-2.0 registry metadata and a repository URL, contains no unsafe
Rust in the published package, and compiles for native and
`wasm32-unknown-unknown` in the S08 design probe. D-20 records the replacement
and its unused transitive Rayon reachability.

## Approach

1. Pin `ritk-codecs` 0.6.0 exactly and use only its JPEG 2000 decode surface.
   D-20 records the deviation from the HLD's `openjp2` prescription.
2. Prove native and wasm decoding on the two manifest-backed Part 1 rows before
   registering either UID. Lossless `.90` must reproduce the independent
   uncompressed ramp exactly. Lossy `.91` must publish its measured sample and
   rendered divergence under the unchanged tolerance policy.
3. Wrap the dependency behind the existing concrete `Decoder` implementation,
   not a second trait. Its safe API returns owned sample storage. D-21 records
   that bounded decode-worker allocation. Commit to caller output only after
   complete success.
4. Validate codestream dimensions, components, precision, signedness, colour
   transform, exact output length, and termination against `FrameDesc`.
5. Record whether JPEG 2000 RCT or ICT has already been converted so later
   pixel stages never apply a colour transform twice.
6. Refuse every unregistered, unsupported, malformed, truncated, or
   inconsistent route without partial output.
7. Mutate one reversible sample and one lossy divergence bound observation.
   Run named tests red and revert without changing tolerance.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Decode is worker-side and scratch is pre-sized
- unsafe: none in Ocelli. Dependency unsafe requires an explicit audit record
- Tier A (WebGPU): n/a. Decode is CPU work before rendering
- Tier B (WebGL2): n/a. The same decoder must be used
- Tier C (CPU): n/a. The same decoder must be used

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| conformance | `.90` is exact against the synthetic uncompressed reference on native and wasm | `crates/ocelli-codec/tests/jpeg2000_corpus.rs` |
| conformance | `.91` publishes measured sample divergence and passes the unchanged rendering tolerance on native and wasm | codec corpus test and oracle |
| unit | Truncated, trailing, dimension, component, precision, colour, and output-length mismatches fail without partial output | module tests |
| mutation | Reversible sample corruption and loss of the measured lossy divergence make named checks fail | feature review evidence |
| cross-target | One implementation decodes both rows on native and wasm with no wasm-bindgen or JS codec bridge | wasm and native gates |

## Parity surface covered

Appendix B `Transfer syntaxes`: JPEG 2000 `.90` and `.91` only. HTJ2K remains
F-027 and is not inferred from a library's broader claims.

## Deviations

- D-20 replaces `openjp2` with `ritk-codecs` 0.6.0 and records the dependency's
  transitive but unused Rayon reachability.
- D-21 records bounded library-owned decode output before the atomic copy into
  the caller buffer.

## LLD impact

- Update `docs/lld/codecs.md` with the selected dependency, provenance,
  allocation, colour ownership, exact lossless result, and measured lossy
  divergence.
- Update `docs/lld/corpus.md`, `docs/lld/oracle.md`, and `docs/lld/README.md`.

## Write set

- `Cargo.toml` and `Cargo.lock`
- `crates/ocelli-codec/Cargo.toml`
- `crates/ocelli-codec/src/lib.rs`
- new JPEG 2000 source and test files under `crates/ocelli-codec/`
- `docs/hld/DEVIATIONS.md`
- codec, corpus, oracle, and README LLD files
- shared sprint ledgers during completion

## Open questions

None. The operator approved a reviewed pure-Rust replacement and the required
HLD deviation. `ritk-codecs` 0.6.0 is the selected candidate after provenance,
unsafe-surface, native, and wasm compile checks.
