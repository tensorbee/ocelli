# F-026, JPEG 2000 via ritk-codecs, validated against the corpus

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

PS3.5 A.4.4 carries one JPEG 2000 codestream per frame after encapsulated-frame
assembly. The SIZ marker declares image extent, component count, component
precision, and signedness. The COD marker declares the multiple-component
transform and whether the reversible 5/3 or irreversible 9/7 transform is in
use. QCD declares the default quantization style. The `.90` syntax permits
only the reversible transform with JPEG 2000 no quantization. The `.91`
syntax may carry either process. A complete codestream terminates in EOC. The
single DICOM NULL byte needed to pad an odd codestream to an even Fragment Item
Value is not part of the JPEG 2000 stream and no other trailing data is legal.

The supported pixel domain in this story is one monochrome component in an
8-bit or 16-bit DICOM container, signed or unsigned, with codestream precision
equal to `BitsStored`. `FrameDesc` has already required `HighBit` to equal
`BitsStored - 1`. The decoder returns stored samples only. It passes identity
rescale parameters to the dependency and does not apply PS3.3 C.11.1, which is
owned by `ocelli-pixel`.

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
`wasm32-unknown-unknown` in the S08 design probe.

The accepted F-026 dependency spike found a narrower resolution than the
registry package as published. The exact crates.io archive has SHA-256
`5fb65755a819c6ba38bf8aaf146f4ccc23d2fe217c8161a61bf00517c9d1cce5`.
Its embedded `.cargo_vcs_info.json` identifies repository commit
`33497ccd55b44e004c0b8314a1bcc2e0fc9cb3ed` and path
`crates/ritk-codecs`. The local vendor copy changes only the normalized
`Cargo.toml` and original `Cargo.toml.orig` dependency entries for
`jpeg-decoder` to set `default-features = false`. It retains exact
`LICENSE-MIT` and `LICENSE-APACHE` files from that commit, selects MIT as the
local redistribution basis, and carries a package inventory and concise patch
record. This removes Rayon from the native and wasm codec dependency graph.

An in-tree path dependency is automatically enrolled as a workspace member
unless excluded. Measured against the repository lint guard, leaving it
enrolled fails because upstream does not inherit Ocelli's lint table. The
workspace therefore excludes exactly `vendor/ritk-codecs-0.6.0` while using
an exact `version = "=0.6.0"` path dependency. The source stays an external
dependency and is still covered by content, provenance, licence, inventory,
and unsafe audits.

## Approach

1. Vendor the exact published package under `vendor/ritk-codecs-0.6.0`, retain
   both exact upstream licence texts, patch both packaged manifests only at the
   `jpeg-decoder` default-feature entry, and record the crates.io archive, VCS,
   package inventory, local patch, and MIT redistribution basis. Exclude this
   exact path from workspace membership and depend on its exact path and
   version.
2. Make the existing pins gate bind the exact 74-row vendor inventory to an
   immutable SHA-256 constant outside the vendor tree. Bind the complete bytes
   of both patched manifests and `PATCH-PROVENANCE.md` so their one declared
   edit and exact account are the only accepted difference. Also assert exact
   licence and provenance hashes, VCS identity,
   absence of removed package files, and absence of Rayon from the
   `ocelli-codec` native and wasm graphs. Add catalogue probes for every new
   script refusal, including coordinated source plus inventory tamper.
3. Add separate `.90` and `.91` decoder modes because the decode call does not
   receive a Transfer Syntax UID. A minimal owned inspector reads SIZ, COD,
   QCD, and terminal EOC facts before dependency decode. It accepts only one
   monochrome component, 8-bit or 16-bit containers, signed or unsigned stored
   samples, identity rescale, no MCT, exact dimensions and precision, and legal
   terminal padding. It requires one valid main-header QCD and refuses QCC
   quantization override. `.90` additionally requires the reversible transform
   and QCD no-quantization style. `.91` keeps both valid Part 1 processes and
   all three legal QCD styles. A deterministically scalar-derived style-1
   codestream is checked against an OpenJPEG 2.5.4 reference.
4. Pass identity rescale to `decode_jpeg2000_fragment`. Validate its complete
   owned `Vec<f32>` result for exact length, finite integral samples, and the
   signed or unsigned stored-value range. Convert validated values into
   canonical little-endian caller output. Copy only after every byte is ready
   so all failures preserve caller output. D-21 records this bounded worker
   allocation.
5. Register both exact UIDs through one atomic helper. Both modes report
   preserved monochrome photometric interpretation and interleaved sample
   layout. Colour, MCT, RCT, and ICT are refused in this story rather than
   reported as converted.
6. Prove native and actual wasm execution on the two manifest-backed Part 1
   rows. `.90` must reproduce the independent uncompressed ramp exactly.
   `.91` publishes sample-domain divergence against that same truth and the
   independent pydicom/OpenJPEG decode under the unchanged HLD 25.1
   monochrome predicate. There is no production render bridge, so no oracle
   or rendered-divergence claim is made.
7. Add a permanent Node runner that calls a real `wasm32-unknown-unknown`
   executable built from the production adapter twice, once with default wasm
   target features and once with `+simd128`. Compare both with the native
   fixture result. `gate native` and this runner are the execution evidence.
   `gate wasm` still builds only `ocelli-wasm` and does not exercise the codec.
8. Add a JPEG 2000-specific benchmark subject and release runner without
   replacing the existing JPEG `decode.frame` measurement. Record its own
   baseline and measure the paired release wasm module with and without the
   vendored dependency patch. The linked size result and dependency-graph
   result are reported separately. Require exactly 31 kept timing samples and
   one warm-up call. Each timing sample covers four consecutive production
   decodes and divides elapsed time by four, preserving the one-call unit while
   excluding process startup, fixture setup, decoder construction and caller
   output allocation. Require those integer counts plus a positive finite
   ordered range that encloses the normalized median.

   This instrument correction follows a temp-only interleaved experiment on
   the exact production fixture and release profile. Extra warm-ups through 32
   did not remove the correlated slow mode. Raising kept samples from 31 to 63
   or 127 left upper deviations of 6.94 and 7.41 per cent across 30 repeats.
   Four-decode batching reduced the observed upper deviation from 8.17 to 5.71
   per cent. Batches of eight and sixteen reduced it to 5.70 and 4.67 per cent,
   so four is the smallest measured batch that materially improves stability.
   A new single series of at least 15 consecutive runs on this exact instrument
   replaces the old calibration before review. The 15 medians range from
   0.6732 to 0.7426 ms around their 0.6878 ms median. The lower and upper
   deviations are 2.12 and 7.97 per cent, so the retained 10 per cent remains
   the smallest declared symmetric band covering the series, with 7.88 and
   2.03 percentage points of lower and upper headroom. This is derived from the
   new series rather than widened to absorb an earlier outlier.
9. Correct the benchmark guard model so a real runner and baseline may be
   reviewed while its owning story is `in-progress` or `done`, while `pending`
   still forbids both and `done` still requires the runner. Fixture-first guard
   tests hold all three transitions. This keeps the gate green without
   misattributing the JPEG 2000 subject or claiming an unimplemented subject.
10. Mutate one reversible sample, one lossy divergence observation, and one
    guard transition. Run the named checks red and revert without changing a
    tolerance or disabling a gate.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Decode is worker-side. The dependency returns
  bounded owned sample storage before Ocelli validates and copies atomically
- unsafe: none in Ocelli. Dependency unsafe requires an explicit audit record
- Tier A (WebGPU): n/a. Decode is CPU work before rendering
- Tier B (WebGL2): n/a. The same decoder must be used
- Tier C (CPU): n/a. The same decoder must be used

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | DICOM PS3.5 A.4.4 SIZ, COD, all three legal QCD styles, and EOC fixtures plus PS3.3 C.7.6.3 signed and unsigned stored samples validate exact little-endian output and lossless no-quantization refusal. A checked generator derives style 1 and its independent OpenJPEG 2.5.4 reference | `crates/ocelli-codec/tests/jpeg2000.rs` and `crates/ocelli-codec/tests/fixtures/generate_jpeg2000_style1.py` |
| conformance | DICOM PS3.5 A.4.4 `.90` is exact against the synthetic uncompressed reference on native and actual Node-hosted wasm | `crates/ocelli-codec/tests/jpeg2000.rs` and codec wasm runner |
| conformance | `.91` publishes measured sample divergence and passes unchanged HLD 25.1 sample bounds on native and actual Node-hosted wasm | codec corpus test and codec wasm runner |
| unit | Truncated, trailing, dimension, component, precision, signedness, transform, MCT, non-integral, range, and output-length mismatches fail without partial output | module and integration tests |
| guard | Pending forbids a benchmark runner and baseline, in-progress permits review, and done requires the runner | benchmark guard fixtures and catalogue probes |
| dependency | Exact vendor inventory, patch, provenance, licences, workspace exclusion, and no-Rayon graphs stay mechanically asserted | pins guard and catalogue probes |
| mutation | Reversible sample corruption and loss of the measured lossy divergence make named checks fail | feature review evidence |
| cross-target | One production implementation executes both rows under native, wasm plain, and wasm `+simd128` with no wasm-bindgen or JS codec bridge | native gate and Node codec runner |
| benchmark | Existing JPEG and new JPEG 2000 release decode subjects remain separately measurable with provenance-bearing baselines. The JPEG 2000 record retains all 15 calibration medians, stores their median, and proves the documented range lies inside its symmetric band | benchmark gate and runner test |

## Parity surface covered

Appendix B `Transfer syntaxes`: JPEG 2000 `.90` and `.91` only. HTJ2K remains
F-027 and is not inferred from a library's broader claims.

## Deviations

- D-20 replaces `openjp2` with the exact locally patched `ritk-codecs` 0.6.0
  package and records that its codec graph no longer reaches Rayon.
- D-21 records bounded library-owned decode output before the atomic copy into
  the caller buffer.

## LLD impact

- Update `docs/lld/codecs.md` with the selected dependency, vendor inventory,
  allocation, refused colour domain, exact lossless result, measured lossy
  divergence, and actual cross-target execution.
- Update `docs/lld/benchmarks.md` with the separate JPEG 2000 subject and
  paired dependency and wasm-size evidence.
- Update `docs/lld/build-targets.md` with the permanent native, plain wasm and
  SIMD wasm execution steps and the explicit `gate wasm` boundary.
- Update `docs/lld/corpus.md` and `docs/lld/README.md`. `docs/lld/oracle.md`
  records only that no production render bridge exists and no oracle claim was
  made.

## Write set

- `Cargo.toml` and `Cargo.lock`
- `crates/ocelli-codec/Cargo.toml`
- `crates/ocelli-codec/src/lib.rs`
- new JPEG 2000 source and test files under `crates/ocelli-codec/`
- deterministic JPEG 2000 style-1 fixture generator and OpenJPEG reference
  under `crates/ocelli-codec/tests/fixtures/`
- exact published package contents, licences, inventory, and provenance under
  `vendor/ritk-codecs-0.6.0/`
- codec wasm execution example and Node runner under `crates/ocelli-codec/`
- `bin/ocelli.sh`
- `scripts/bench_check.py`, its tests, and required guard catalogue entries
- `scripts/pin_and_size_check.py`, its tests, and required guard catalogue
  entries
- `tools/bench/subjects.json`, JPEG 2000 runner, runner tests, and benchmark
  baseline
- `docs/hld/DEVIATIONS.md`
- `docs/SOURCE-POLICY.md`
- codec, benchmark, build-target, corpus, oracle, and README LLD files
- shared sprint ledgers during completion

## Open questions

None. The operator approved the exact local-vendor patch after archive, VCS,
licence, inventory, unsafe-surface, dependency-graph, native, and actual wasm
execution checks. The benchmark guard transition is part of this feature so a
real in-progress subject can be reviewed before its completion ledger changes.
