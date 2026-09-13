# F-028, JPEG-LS: decide CharLS bridge vs pure Rust, then integrate

**Status**: approved
**Epic ref**: E4.6
**Sprint**: S09
**Estimate**: 5w
**Allocation note**: `P0 KILL CRITERION — no credible pure-Rust path`

## Normative source, transcribed

### `docs/hld/18-codec-registry.md`, section 21

Transcribed into table rows, which is where `scripts/prose_check.py` relaxes the
voice rules, so the author's text is quoted exactly.

|  |
|----|
| Explicit runtime registration, not the inventory crate — inventory does not work on WebAssembly, which is precisely why dicom-rs's own plugin registry is unavailable there. Explicit registration is also what lets a native build link C codecs the browser build cannot. |

```rust
pub trait Decoder: Send + Sync {
    fn transfer_syntaxes(&self) -> &'static [&'static str];
    /// Decode one frame into `out`. Must not allocate.
    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8])
    -> Result<(), CodecError>;
}
pub struct Registry { by_ts: HashMap<&'static str, Arc<dyn Decoder>> }
impl Registry {
    pub fn register(&mut self, d: Arc<dyn Decoder>) {
        for ts in d.transfer_syntaxes() { self.by_ts.insert(ts, d.clone()); }
    }
}
```

The gate note, quoted in full:

|  |
|----|
| **TWO OPEN GATES** HTJ2K through openjp2 is registered in dicom-rs but unverified under wasm32 — test it bit-exact against OpenJPH output in week one. JPEG-LS has no credible pure-Rust path; the registry design deliberately allows a JS-side bridge to @cornerstonejs/codec-charls as a registered decoder, so choosing that route costs an adapter rather than a redesign. |

**"JPEG-LS has no credible pure-Rust path" is the sentence this story exists to
re-check, and F-X006 already re-checked it.** `docs/spikes/A2-jpeg-ls.md` records
the answer: the sentence is still true inside dicom-rs, whose `charls` feature is
`dep:charls` over the C++ library, and it is no longer true outside it.

### `docs/hld/15-lut-chain.md`, section 18, the constraint this story must not break

|  |
|----|
| Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere. |

A JPEG-LS decoder that applies Rescale Slope and Rescale Intercept is that
second copy. See Approach.

### `docs/spikes/A2-jpeg-ls.md`, gate A2, transcribed verdict

**Status: RESOLVED. Outcome `Pure Rust`.** Recommendation, quoted:

> **Adopt `pure_jpegls` 2.0.0 for `1.2.840.10008.1.2.4.80` and
> `1.2.840.10008.1.2.4.81`, on every target, with three conditions.**

and

> **Keep `ritk-codecs` 0.6.0 as the named fallback.** It is equally correct on
> both rows, builds for both targets, and appears to handle multi-component. Its
> costs are three times the binary size and a rescale-applying API that section
> 18 does not want.

Both routes' measured digests over the canonical 12288 bytes, quoted from the
gate:

```text
R (syntax/explicit_vr_le.dcm PixelData): b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609

--- jpegls_lossless, 1.2.840.10008.1.2.4.80, lossless ---
  pure_jpegls/native       b20a1ef3...  ritk_codecs/native       b20a1ef3...
  pure_jpegls/wasm         b20a1ef3...  ritk_codecs/wasm         b20a1ef3...
  pure_jpegls/simd         b20a1ef3...  ritk_codecs/simd         b20a1ef3...
  every route EQUAL to A_dcmtk, and every route EQUAL to R

--- jpegls_near_lossless, 1.2.840.10008.1.2.4.81, near-lossless NEAR 3 ---
  every surviving route    1353c495...   max abs error 3, 0 sample(s) over 3
```

### `docs/sprints/CURRENT_SPRINT.md`, the instruction that reopens the choice

> **The allocation's note on F-028 says "no credible pure-Rust path", and that is
> now measurably out of date.** [...] F-028 must evaluate that path first and
> record the measured result. The CharLS bridge is still a legitimate outcome,
> but choosing it without first measuring the code already vendored in this
> repository would be a decision made from a stale note.

## What the specification does not cover

1. **Which of the two measured-correct pure-Rust routes to adopt.** A2 chose
   `pure_jpegls` and `CURRENT_SPRINT.md` instructs this story to evaluate the
   already-vendored `ritk-codecs` first. **A2's comparison contains one factual
   error that changes the answer, found by reading the vendored source, and it
   is the reason this plan recommends the runner-up.** See Approach.
2. **What "the full stored range" means as a test.** `CURRENT_SPRINT.md`
   requires the `f32` round trip proven over the full stored range rather than
   over a sample. This plan makes that concrete.
3. **Whether `.80` and `.81` are one decoder or two.** The registry dispatches
   on exact UID. This plan uses two `Mode` arms of one adapter, which is exactly
   the shape `Jpeg2000Decoder` already has for `.90` and `.91`.

## Approach

### The decision, and the evidence that changes it

**A2's one advantage for `pure_jpegls` over `ritk-codecs` was multi-component,
and it is not real.** A2's coverage table says of `ritk-codecs`: "Its module has
`ComponentInfo` and `InterleaveMode` and parses `SOF55`, `LSE`, `DRI`, `DNL`.
**NOT MEASURED here**", and the recommendation then reads "appears to handle
multi-component". Read from the vendored source rather than inferred from module
names, `vendor/ritk-codecs-0.6.0/src/jpeg_ls/decoder.rs` lines 96 to 107:

```rust
if self.components.len() != 1 {
    bail!(
        "JPEG-LS multi-component ({}) not supported; use non-interleaved encoding",
        self.components.len()
    );
}
if self.interleave_mode != InterleaveMode::None {
    bail!(
        "JPEG-LS interleave mode {:?} not supported for single-component DICOM frames",
        self.interleave_mode
    );
}
```

**Both candidates are single-component only, and both refuse explicitly rather
than producing a wrong pixel.** A2's condition 2, "Multi-component JPEG-LS
reports unavailable until it is measured", therefore applies identically to
either route and is no longer a tiebreaker. A2's own text is careful to mark the
row `NOT MEASURED`, so this is the gate's stated uncertainty resolving rather
than the gate being wrong.

With that removed, the remaining axes:

| Axis | `pure_jpegls` 2.0.0 | `ritk-codecs` 0.6.0 |
|------|---------------------|---------------------|
| `.80` against `R`, the encoder-independent anchor | exact, measured | exact, measured |
| `.81` against the ISO 14495-1 `NEAR` bound | within, max abs 3, measured | within, max abs 3, measured |
| native and wasm, plain and `+simd128` | identical digests | identical digests |
| Multi-component | refused by the crate | **refused by the crate** |
| Already in this repository | **no** | **yes**, vendored under D-20 |
| New supply-chain surface | a second vendored package, two more licence texts, new pins-gate rows, a new archive and VCS identity to bind | **none** |
| Incremental wasm size | ~40 KB on top of the ~124 KB already paid | the `jpeg_ls` module only, **to be measured** |
| `unsafe` in the package | none | none |
| Return type | `Vec<u16>` | `Vec<f32>` through a rescale-applying layout |
| Adapter precedent in this tree | none | `crates/ocelli-codec/src/jpeg2000.rs`, reviewed and merged at F-026 |

**The recommendation is `ritk-codecs` 0.6.0's `jpeg_ls` module.** The deciding
argument is not correctness, because both are measured exact on the anchors that
matter. It is that adopting `pure_jpegls` buys a `u16` return type and pays for
it with a second vendored package: another archive digest, another VCS identity,
another pair of licence texts, another set of pins-gate rows, and a second
no-Rayon graph to enforce on native and wasm. `ritk-codecs` is already all of
those things, enforced by a gate that already runs. **A2's two stated costs for
`ritk-codecs` are both already paid in this repository**, which was not true when
A2 was written and is the fact `CURRENT_SPRINT.md` is pointing at.

A2's third cost, the rescale-applying API, is answered by precedent rather than
by argument: `crates/ocelli-codec/src/jpeg2000.rs` passes `rescale_slope: 1.0`
and `rescale_intercept: 0.0`, which makes `decode_native_pixel_bytes_checked`'s
modality step the identity, and then converts each `f32` back to an exact integer
through `exact_i64`, refusing anything non-integral. **Section 18's arithmetic
therefore still exists exactly once**, in `ocelli-pixel`, and the codec's rescale
parameters are pinned to the identity at the one call site. The same pinning is
used here, and a test asserts it by decoding a frame whose correct output would
differ if slope and intercept were ever passed through.

**The S09 design round selected `ritk-codecs`.** The alternative would have
changed this plan by one dependency line and one conversion, because
`pure_jpegls` returns `u16` directly and needs no `exact_i64`. Everything else
below was unchanged either way, which is why the choice was answerable as one
question rather than two plans.

### The adapter

New file `crates/ocelli-codec/src/jpegls.rs`, mirroring `jpeg2000.rs`:

```rust
const JPEGLS_LOSSLESS_UID: &str = "1.2.840.10008.1.2.4.80";
const JPEGLS_NEAR_LOSSLESS_UID: &str = "1.2.840.10008.1.2.4.81";

enum Mode { Lossless, NearLossless }

pub struct JpegLsDecoder { mode: Mode }

pub fn register_jpegls_decoders(registry: &mut Registry) -> Result<(), RegistryError>;
```

Registration preflights both UIDs before registering either, which is the atomic
shape `register_jpeg2000_decoders` already uses and deviation **D-19** requires.

`decode` performs, in order:

1. Refuse a caller output whose length differs from `desc.output_len()`.
2. `validate_descriptor`: `PixelDataVr::Ob`, `samples_per_pixel == 1`,
   `bits_allocated` in `{8, 16}`, photometric interpretation `MONOCHROME1` or
   `MONOCHROME2`. Anything else is `UnsupportedPixelFormat`. **This is where
   multi-component JPEG-LS reports unavailable**, and it reports it before the
   dependency is called, so the refusal is ours and typed rather than an opaque
   dependency string.
3. Parse the frame's own SOF55 marker segment for precision, dimensions and
   component count, and its SOS segment for `NEAR`, and check them against the
   descriptor. This is the `inspect_main_header` and `validate_header` pattern
   `jpeg2000.rs` already establishes, and it is what makes
   `CodecError::FrameMismatch` distinguishable from `CodecError::DecoderFailure`.
4. **Enforce the mode against `NEAR`.** `NEAR == 0` is lossless and is the only
   legal value for `.80`. `NEAR > 0` is near-lossless and is the only legal value
   for `.81`. A `.80` frame whose `NEAR` is positive is `FrameMismatch`, and so
   is a `.81` frame whose `NEAR` is zero. **This is the mode check that
   `.90`/`.91` gets from the wavelet transform and quantization style**, and
   without it the two UIDs would be interchangeable, which would make the
   lossless claim unfalsifiable.
5. Call `decode_jpeg_ls_fragment(src, layout)` with `rescale_slope: 1.0` and
   `rescale_intercept: 0.0`.
6. Convert through the same exact-integer path as `jpeg2000.rs`, refusing any
   sample outside the descriptor's stored range and any non-integral `f32`.
7. Write the caller's output in one `copy_from_slice`, so an error leaves it
   untouched.

**`convert_samples`, `append_sample` and `exact_i64` are currently private to
`jpeg2000.rs`.** They are moved to a shared private module, `sample_convert.rs`,
and used by both adapters unchanged. This is not a new abstraction: it is one
function with two call sites replacing one function with one call site, and the
alternative is a byte-identical copy of the stored-domain boundary, which is the
defect class this sprint names.

### Capability

Registration is unconditional in `register_jpegls_decoders`. **There is no route
where the adapter registers and then refuses at call time while claiming
capability**, which `CURRENT_SPRINT.md` names as the specific anti-pattern for a
kill-criterion codec. The per-frame refusals above are `CodecError` results for
frames outside the measured envelope, not a capability claim. If the operator
answers Open question 2 with "do not adopt", the story instead leaves both UIDs
`KnownUnavailable`, which `Registry::capability` already distinguishes from
`Unknown`, and no adapter is written.

## Boundary and tier

- wasm-bindgen: not touched. `ritk-codecs` is pure Rust with no `wasm-bindgen`
  in its graph, which the `bindgen` gate already asserts for this crate
- Pixels across the boundary: no. Decode runs worker-side and writes into a
  caller buffer inside linear memory
- Render-loop allocation: none in the render loop. Decode is not in the render
  loop. Within decode, the dependency allocates bounded decoded storage, which
  is **deviation D-21, already declared**, and the `decode.frame` benchmark
  measures it
- unsafe: none. The published package contains no `unsafe` and this adapter adds
  none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

All three tier rows are `n/a` for the reason A2 states and this plan adopts:
decode is CPU work in a worker that completes before anything reaches a device,
the resolved tier does not select a decoder, and **a tier-gated codec path would
be section 18's rule violated with the added property that the second path would
only ever run on hardware nobody develops on.** The axis that matters here is
the target, browser against native, and that is measured on both.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `conformance` | `.80` decodes `corpus/manifest.tsv` row 45 byte-identically to `R`, the uncompressed `syntax/explicit_vr_le.dcm` Pixel Data, which is encoder-independent | `crates/ocelli-codec/tests/jpegls.rs` |
| `conformance` | `.81` decodes row 46 to the digest A2 measured, and every sample is within ISO/IEC 14495-1's `NEAR = 3` bound against `R`, with the count over the bound asserted zero | same |
| `fixture` | **The full stored range.** A 256 by 256 16-bit unsigned frame whose samples are `0 ..= 65535` exactly once each, encoded by `pyjpegls` 1.5.1 through a committed generator, decodes to the hand-constructed ramp byte for byte. This is the `f32` round trip proven over the whole domain rather than a sample | same |
| `fixture` | The same ramp at `BitsStored` 12 in a 16-bit container, so the stored range is `0 ..= 4095` and a sample above it is refused rather than truncated | same |
| `fixture` | Signed 12-bit: `-2048 ..= 2047`, hand-computed little-endian two's complement, citing PS3.3 C.7.6.3.1.4 | same |
| `fixture` | **The rescale pin.** A frame decoded through the adapter equals the same frame decoded with `rescale_slope: 1.0, rescale_intercept: 0.0`, and differs from one decoded with slope 2 intercept 5. This is the section 18 single-implementation claim made checkable | same |
| `unit` | `.80` refuses a `NEAR > 0` frame and `.81` refuses a `NEAR == 0` frame, both `FrameMismatch` | same |
| `unit` | A multi-component SOF55 is refused as `UnsupportedPixelFormat` before the dependency is called | same |
| `unit` | Dimension, precision and signedness mismatches against the descriptor are `FrameMismatch`, and the caller's output is byte-unchanged after every error | same |
| `unit` | Registration is atomic: a collision on the second UID leaves the first unregistered | same |
| `unit` | `capability` reports `Available` for both UIDs after registration and `KnownUnavailable` before | same |
| `browser` | Both corpus rows decode to the same bytes on `wasm32-unknown-unknown` as on native, plain and `+simd128`, which is A2's cross-target claim held as a standing test rather than a spike result | `bin/ocelli.sh gate native` path, `crates/ocelli-codec/tests/jpegls.rs` under the wasm target |

**Controlled mutation, required by `CURRENT_SPRINT.md`'s "every concrete decoder
added this sprint has a controlled mutation observed red for the claimed
reason".** Three mutations, each named in the implementation note with its
observed exit: swap the `NEAR` mode check so `.80` accepts near-lossless, change
the rescale pin from `1.0`/`0.0` to `2.0`/`5.0`, and drop the stored-range
refusal in `append_sample`. Each must go red in a named test and the test name is
recorded beside the mutation.

**The codec benchmark covers it**, per the same sprint requirement:
`tools/bench/subjects.json` gains `.80` and `.81` entries on the existing
`decode.frame` subject.

## Parity surface covered

`docs/hld/B-parity-surface.md` has a row **Transfer syntaxes, count ~13, "Two of
them, JPEG-LS and HTJ2K, are the open gates in Appendix A"**. This story closes
the JPEG-LS half of that row in implementation, A2 having closed it as a
decision. There is no `Covered by` column in this repository's copy to update.

## Deviations

- **D-19**, atomic registration. Already declared, reused unchanged
- **D-20**, the vendored `ritk-codecs` 0.6.0 path. Already declared. **Its text
  said "for JPEG 2000 Part 1 on native and wasm", so adopting the same package's
  `jpeg_ls` module widens what the row covers.** The row is **amended in the
  design-approval change** to name JPEG-LS as well, to record that the module is
  the same no-unsafe published package under the same pins-gate bindings, to
  record the slope 1.0 and intercept 0.0 pin that keeps section 18's arithmetic
  single, and its `Raised` cell now reads `F-026, F-028`. That is an edit to an
  existing row rather than a new `D-NN`, because the departure from section 15.2
  is the same departure and splitting it would make the register count a thing it
  is not
- **D-21**, bounded decoder-owned allocation. Already declared and already names
  `ritk-codecs`. `Raised` now reads `F-024, F-026, F-027, F-028`

**No new `D-NN` row.** That was true only for the selected route. `pure_jpegls`
would have required one, for a second vendored codec package outside section
15.2's list.

## LLD impact

- `docs/lld/codecs.md`, a JPEG-LS section covering the two UIDs, the `NEAR` mode
  rule, the single-component limit reported as a refusal, the rescale pin, and
  the shared `sample_convert` module now used by two adapters
- `docs/spikes/A2-jpeg-ls.md`, a correction note recording that `ritk-codecs`
  0.6.0 refuses multi-component in source, so the "appears to handle
  multi-component" line is withdrawn. **The measurements in that file are
  untouched.** Only the inference drawn from an unmeasured row changes
- `docs/hld/DEVIATIONS.md`, D-20 and D-21 `Raised` cells

## Open questions

None. All three were answered in the S09 consolidated design round.

## Decisions from the S09 design round

**1. `ritk-codecs` 0.6.0's `jpeg_ls` module, not `pure_jpegls` 2.0.0.** The
multi-component advantage A2 gave `pure_jpegls` is not real, read from the
vendored source, and every remaining cost of the runner-up is already paid in
this repository. **A2's written recommendation is therefore overridden on
evidence A2 itself marked `NOT MEASURED`**, which is why this was an operator
decision rather than an implementation detail. A2's measurements are untouched
and only the inference from an unmeasured row changes.

**2. Single-component with an explicit refusal is this sprint's answer**, and it
closes the P0 kill criterion. A DICOM JPEG-LS frame can be RGB and neither
candidate decodes one, so `validate_descriptor` refuses `samples_per_pixel != 1`
before the dependency is reached. That is a clean refusal rather than a wrong
pixel. **The whole syntax is not reported unavailable**, because both corpus
rows are single-component and both decode exactly. The multi-component corpus
row A2 owes remains a separate story.

**3. `tools/spikes/a2-jpeg-ls/` is deleted by this story.** A2's own lifecycle
sentence says the harnesses go when the gates close, and F-028 is the closing
story for A2. `docs/spikes/A2-jpeg-ls.md` stays, because it is the evidence.
Its **Reproducing this** section gains a note that the harness it names has been
removed and at which commit, so the section describes a rig that existed rather
than one a reader will fail to find.
