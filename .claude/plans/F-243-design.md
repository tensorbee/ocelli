# F-243, JPEG-LS run mode decoded per T.87

**Status**: approved
**Epic ref**: E4.13
**Sprint**: S46
**Estimate**: 3w

Story row, `docs/sprints/BACKLOG.md`: "JPEG-LS decoded per ITU-T T.87: the vendored ritk-codecs scan decoder raises the run index only for a full run segment, a scan that runs out of data or leaves unread bytes before EOI is refused, an LSE preset this decoder does not apply is refused by name, default thresholds clamp as C.2.4.1.1.1 gives them, with CharLS-encoded trigger fixtures and banded corpus rows under .80 and .81". Layer Rust, depends on F-028, milestone M20.

`allocation.json` notes: "Decided by the operator on 2026-11-07: a monochrome JPEG-LS frame with a long flat run ending a line decodes to wrong pixels with no error. The vendored run-mode loop raises RUNindex on a partial end-of-line segment, and the scan decoder never refuses a desynchronised or truncated scan. This story patches the vendored decoder, lands before F-239's codec part, and F-239's ritk-codecs patch merges onto it."

**Why E4.13.** E4 is the codec epic. F-229 took E4.10, F-241 E4.11 and F-242 E4.12. E4.9 stays the unresolved dependency F-145 declares, which `scripts/backlog_check.py` keeps declared on purpose, so it is not taken. The milestone is M20, as F-229's is, because S46's codec stories sit there.

**Why S46, and next.** The operator scheduled it on 2026-11-07 to be done next. Its only dependency, F-028, is done. F-239's codec part edits the same `PATCH-PROVENANCE.md` and the same pins gate constants, so F-239 now depends on this story and its codec branch merges onto this one.

Every path and line below was read on `sprint/s46` at dcfe9a57.

## Normative source, transcribed

### HLD section 21, `docs/hld/18-codec-registry.md:21` to `:39`

```rust
pub trait Decoder: Send + Sync {
    fn transfer_syntaxes(&self) -> &'static [&'static str];
    /// Decode one frame into `out`. Must not allocate.
    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8])
    -> Result<(), CodecError>;
}
```

The HLD names JPEG-LS an open gate, answered by Appendix A gate A2 as pure Rust and by D-20 as the vendored `ritk-codecs` `jpeg_ls` module. The HLD gives no JPEG-LS arithmetic. The arithmetic is ITU-T T.87, which is ISO/IEC 14495-1, and is transcribed below.

### D-20, `docs/hld/DEVIATIONS.md:37`, the sentence this story makes false

"The package is unchanged except that both Cargo manifests disable `jpeg-decoder` default features." And `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md:11` to `:13`: "Ocelli changes only the `jpeg-decoder` dependency in `Cargo.toml` and `Cargo.toml.orig` to disable its default Rayon feature. The source itself is unchanged."

### D-31, `docs/hld/DEVIATIONS.md:48`

"Both vendored crates carry a recorded source patch. ... Each patch is listed with its file hashes in the crate's `PATCH-PROVENANCE.md` and in `scripts/pin_and_size_check.py`, which still refuses any unrecorded byte." D-31 is F-239's, and F-239's `ritk-codecs` patch is not on `sprint/s46` yet. It lives on `work/f-239-codec` at b5060f93 and touches `src/jpeg_2000/image.rs`, `src/jpeg_2000/mod.rs`, `src/jpeg_2000/packet/reader.rs` and `src/lib.rs`. None of those is a file this story patches.

### T.87 run mode, code segments A.15 and A.16 (encoder, A.7.1.2) and F.1 step 18 (decoder)

The operator's statement of the rule, which this story implements: **the count a run hit adds is `min(2^J[RUNindex], remaining)`, and RUNindex is raised only when that count is the full `2^J[RUNindex]`.** In decoder form, T.87 F.1 step 18, p. 47, in Annex F, which "does not form an integral part of this Recommendation":

```text
while (read_bit() == 1) {
    count = MIN(1 << J[RUNindex], remaining - index)
    index = index + count
    if (count == (1 << J[RUNindex])) and (RUNindex < 31): RUNindex = RUNindex + 1
    if (index == remaining): break
}
if (index != remaining) {
    index = index + read_bits(J[RUNindex])     -- the run interruption follows
    if (RUNindex > 0): RUNindex = RUNindex - 1  -- F.1 step 18.c.ii
}
```

F.1 step 18.c.ii, p. 47, reads "If RUNindex>0, decrement RUNindex by one". A.7.2 item 8, p. 25, uses J[RUNindex] "before the decrement specified in Code segment A.16".

On the encoder side, code segment A.16, a run that reaches the end of the line with fewer than `2^J[RUNindex]` samples left is written as one `1` bit **without** raising RUNindex. Ocelli already implements exactly this rule once, for the colour scan, at `crates/ocelli-codec/src/jpegls/sample_interleaved.rs:144` to `:172`, whose comment reads "A full `J` segment advances the run index, including one that ends the line. A one-bit that only fills the remainder does not." That function also refuses a miss whose remainder carries the run past the line, `filled > remaining`, at `:169`.

`J`, T.87 A.2.1 step 3, p. 17, repeated in F.1 step 1.e, p. 46, as both the vendored crate and Ocelli carry it:

```text
J[0..32] = 0 0 0 0 1 1 1 1 2 2 2 2 3 3 3 3 4 4 5 5 6 6 7 7 8 9 10 11 12 13 14 15
```

### T.87 C.2.4.1.1, the LSE preset coding parameters (ID 1)

The segment is `LSE Ll ID MAXVAL T1 T2 T3 RESET`, ID one byte, the five parameters 16 bits each, so `Ll = 13`. A parameter value of zero selects its default. The constraints on a non-zero value are `NEAR + 1 <= T1 <= MAXVAL`, `T1 <= T2 <= MAXVAL`, `T2 <= T3 <= MAXVAL`, and RESET in `3 ..= max(255, MAXVAL)`. Table C.1, p. 33, gives MAXVAL as "0, or 1 <= MAXVAL < 2^P". The default MAXVAL is `2^P - 1` and the default RESET is 64.

### T.87 C.2.4.1.1.1, the default thresholds

```text
BASIC_T1 = 3, BASIC_T2 = 7, BASIC_T3 = 21
if MAXVAL >= 128:
    FACTOR = floor((MIN(MAXVAL, 4095) + 128) / 256)
    T1 = CLAMP(FACTOR * (BASIC_T1 - 2) + 2 + 3 * NEAR, NEAR + 1, MAXVAL)
    T2 = CLAMP(FACTOR * (BASIC_T2 - 3) + 3 + 5 * NEAR, T1, MAXVAL)
    T3 = CLAMP(FACTOR * (BASIC_T3 - 4) + 4 + 7 * NEAR, T2, MAXVAL)
else:
    FACTOR = floor(256 / (MAXVAL + 1))
    T1 = CLAMP(MAX(2, floor(BASIC_T1 / FACTOR) + 3 * NEAR), NEAR + 1, MAXVAL)
    T2 = CLAMP(MAX(3, floor(BASIC_T2 / FACTOR) + 5 * NEAR), T1, MAXVAL)
    T3 = CLAMP(MAX(4, floor(BASIC_T3 / FACTOR) + 7 * NEAR), T2, MAXVAL)

CLAMP(i, j, MAXVAL) = j if i > MAXVAL or i < j, otherwise i
```

**CLAMP is not a saturating clamp.** A value above MAXVAL becomes the lower bound `j`, not MAXVAL.

### T.87 A.2.1 and A.4.4, the coding parameters and the near-lossless bound

`RANGE = floor((MAXVAL + 2 * NEAR) / (2 * NEAR + 1)) + 1`, `qbpp = ceil(log2(RANGE))`, `bpp = max(2, ceil(log2(MAXVAL + 1)))`, `LIMIT = 2 * (bpp + max(8, bpp))`. Near-lossless coding guarantees that every reconstructed sample differs from its source by at most NEAR. The vendored `CodingParams::new` at `vendor/ritk-codecs-0.6.0/src/jpeg_ls/context/mod.rs:327` to `:341` matches these four, with MAXVAL always `2^P - 1`.

### T.87 A.1 and the C.1.2 NOTE, byte stuffing and the end of the scan

Inside entropy-coded data an `0xFF` byte is followed by a byte whose most significant bit is a stuffed zero, carrying seven data bits. `0xFF` followed by a byte with the high bit set is a marker and ends the entropy-coded segment. The encoder pads the final byte to a byte boundary.

### Transcription flag

The lead could not obtain the published T.87 text for this plan: the ITU download returned a server error and the retry was refused by the session's policy. The run rule, F.1 step 18 in the decoder and code segments A.15 and A.16 in the encoder, is the operator's own statement and is the rule Ocelli's reviewed colour decoder already implements. The C.2.4.1.1 and C.2.4.1.1.1 text above is the lead's transcription. Two facts measured for this plan corroborate the threshold arm without CLAMP: CharLS 2.4.4 writes LSE `T1 18, T2 67, T3 276, RESET 64` at P 16 and NEAR 0, and `T1 27, T2 82, T3 297` at P 16 and NEAR 3, which are exactly `16 + 2`, `64 + 3`, `272 + 4` and `16 + 2 + 9`, `64 + 3 + 15`, `272 + 4 + 21`. **AC14 makes the check of every transcribed line against the published T.87 a reviewer row**, and a mismatch stops the story before code rather than being absorbed by it. The CLAMP arm is also decided by measurement, AC6, against CharLS on a stream that reaches it.

## Facts measured for this plan

From the investigation of 2026-11-07, which decoded the reported file and 512 CharLS-encoded synthetic images in a scratch copy, and from reads and probes on dcfe9a57.

| Fact | Where |
|------|-------|
| The vendored run loop adds `1 << J[idx]`, clamps to `remaining` and raises `run_index` on that partial hit | `vendor/ritk-codecs-0.6.0/src/jpeg_ls/scan.rs:177` to `:204` |
| A miss clamps `run_len` to `remaining` instead of refusing | `scan.rs:200` to `:201` |
| `read_bits` returns 0 when the stream is exhausted, and `decode_scan` never checks what it consumed | `vendor/ritk-codecs-0.6.0/src/jpeg_ls/bitstream.rs:65` to `:79`, `scan.rs:275` |
| `refill` sets `pos` to the end of the data when it meets a marker, so the marker's offset is lost | `bitstream.rs:46` to `:49` |
| The vendored encoder raises `run_index` on the overshoot hit, the mirror defect, so the upstream round trips agree with the defective decoder | `vendor/ritk-codecs-0.6.0/src/jpeg_ls/encoder.rs:180` to `:187` |
| No Ocelli code calls the vendored encoder. `grep -rn 'jpeg_ls::encoder\|encode_grayscale_jpeg_ls' crates examples tools scripts bin` finds nothing | measured |
| `parse_lse` reads T1, T2 and T3 and ignores MAXVAL and RESET | `vendor/ritk-codecs-0.6.0/src/jpeg_ls/parser.rs:86` to `:102` |
| `CodingParams::new` takes MAXVAL as `2^P - 1` always, and `RESET` is the constant 64 | `context/mod.rs:328`, `:45` |
| All three LSE thresholds are used when `t1 > 0`, and all three defaults otherwise | `scan.rs:140` to `:144` |
| `default_thresholds` saturates at MAXVAL where CLAMP gives `j` | `context/mod.rs:290` to `:306` |
| The adapter maps every vendored error to `CodecError::DecoderFailure` | `crates/ocelli-codec/src/jpegls.rs:158` |
| The adapter's header walk already stops at SOS without decoding | `jpegls.rs:233` to `:281` |
| The colour scan refuses any LSE as `UnsupportedPixelFormat` and refuses an exhausted stream | `jpegls.rs:388` to `:435`, `sample_interleaved.rs:610` to `:629` |
| CharLS 2.4.4 through imagecodecs 2026.10.10 writes **no LSE** for 8-bit input at NEAR 0 and NEAR 34, so the default threshold path runs | probe in the repository `.venv`, 2026-11-07 |
| imagecodecs takes P from the array dtype: a `uint16` array is P 16 and carries the default LSE | same probe |
| cornerstone3D 5.8.2 decodes JPEG-LS with `@cornerstonejs/codec-charls` 1.2.5, pinned at `tools/oracle/package.json:22`, whose wasm carries the version string `2.4.1` | `tools/oracle/node_modules/@cornerstonejs/codec-charls/dist` |
| Neither `ocelli-wasm` nor `ocelli-native` reaches `ocelli-codec` | D-20, D-28, re-measured in AC12 |

**The trigger is independent of LSE.** The defect is the run-mode rule, and an LSE marker neither causes it nor is needed for it. A second reviewer reproduced the wrong pixels on a 12-bit `.80` file with **no LSE marker at all**: Enhanced CT, two frames of 32 by 32, Bits Allocated 16, Bits Stored 12, the same two-band content, columns 0 to 15 at 224 and 16 to 31 at 1064. Pixels 0 to 79 were right and 915 of 1,024 were wrong from pixel 80 on, while pydicom and imagecodecs decoded it exactly. Its frame is SOF55 with P 12 and SOS with NEAR 0, nothing between. Measured for this plan: `jpeg_ls.encode_buffer(image, 32, 32, 1, 12)` from pyjpegls 1.5.1 reproduces that frame's 97 bytes exactly, and CharLS 2.4.4 decodes it to the formula.

The first reported file was Enhanced CT, two frames of 32 by 32, P 16, NEAR 0, with the default LSE. The vendored decoder returned `Ok` with 916 of 1,024 samples wrong in each frame, the first at row 2 column 16, where a run of 224 continued for 20 samples instead of 16. With only the F.1 step 18 hunk applied, the reported file, the 512 synthetic images (105 wrong before) and NEAR 1 and NEAR 3 two-band images decoded exactly, and the 19 JPEG-LS tests still passed. Keeping 90, 50 or 10 per cent of the scan of `jpegls_full_range_u16.jls` returned `Ok` every time, and CharLS refused every time.

## What the specification does not cover

1. **Where each refusal lives.** The run-mode rule, the exhaustion check and the end-of-scan check are inside the vendored scan decoder, because only it holds the bit position. The LSE refusal is in Ocelli's adapter, `inspect_headers` and `validate_header` in `jpegls.rs`, because `CodecError` is the typed name the operator asked for and the vendored crate returns `anyhow` errors the adapter cannot tell apart without matching strings. The adapter already owns every other header refusal. The vendored decoder then only ever receives presets it applies.
2. **The name.** `CodecError` gains one variant, `UnsupportedPreset(JpegLsPreset)`, with `JpegLsPreset` a new public enum of four values, `MaxVal`, `Reset`, `PartialDefaultThresholds` and `ThresholdOrder`. Each names a failure class and never a value, which is the rule `use.html:885` states for every crate error. A failure of the scan itself stays `DecoderFailure`, the existing mapping.
3. **What zero means.** MAXVAL 0 and RESET 0 select their defaults per C.2.4.1.1 and are accepted. All three thresholds zero selects all three defaults, which the vendored decoder already applies. One or two zero thresholds beside a non-zero one is conforming but is not what the vendored decoder applies, so it is refused as `PartialDefaultThresholds`, by the operator's decision.
4. **The end-of-scan rule.** A.1 says the final byte is padded and does not state what a decoder must refuse. This plan's rule: after the last sample, the unread entropy bits before the terminating marker must lie inside the byte that holds the last consumed bit, plus the stuffing byte that follows it when that byte is `0xFF`. A whole further byte is refused. The marker that ends the entropy-coded segment must be the codestream's final EOI, so a DNL, a second SOS or any other marker after the scan is refused. The reader must never have run out.
5. **The default-threshold CLAMP.** Not in the operator's list. It is the same defect class in the same decoder: at P 8 and NEAR 34, C.2.4.1.1.1 gives T3 = 177 and the vendored function gives 255, so any CharLS 8-bit stream at NEAR 34 or above, which carries no LSE, takes different contexts from the encoder. Fixed by transcription in `default_thresholds`, decided by the lead because the alternative, refusing such files, would leave Ocelli below the cornerstone3D floor for files CharLS decodes. AC6 measures it. The operator may veto it at review without changing any other row.
6. **The vendored encoder.** Fixed, with the code segment A.16 mirror of the decoder hunk. Ocelli does not call it, measured above, so this changes no Ocelli output. It is fixed rather than left because the published round-trip suite pairs it with the decoder, and an encoder that disagrees with the fixed decoder would turn that suite into a check against the wrong rule, or red.
7. **The miss past the line end.** A miss whose remainder carries the run beyond `remaining` is refused, as Ocelli's colour path and CharLS refuse it, rather than clamped. A miss that lands exactly on `remaining` is left as the vendored code reads it, a run with no interruption, since no conforming encoder writes it and refusing it is not needed for any wrong pixel.
8. **Which encoder writes which artefact.** The bare codestream fixtures are encoded by CharLS 2.4.4 through imagecodecs 2026.10.10, the operator's choice, with one exception. AC20's 12-bit fixture is encoded by pyjpegls 1.5.1's `encode_buffer` with `bits_stored` 12, which also wraps CharLS and is measured byte-identical to the second reviewer's frame. The corpus rows are written by `scripts/corpus_synth.py` through its existing `.80` and `.81` producer, pyjpegls 1.5.1, which also wraps CharLS. `EXTERNAL_ENCODERS` binds one producer to each transfer syntax (`scripts/corpus_synth.py:218` to `:235`) and `scripts/tests/test_corpus_synth.py:79` to `:81` asserts it, so a second producer for `.80` would break that invariant for no gain.
9. **Reference for near-lossless output.** The specification's anchor is the NEAR bound against the formula. Parity with cornerstone3D is exact equality to a CharLS decode, which is normative and deterministic. Both are asserted. The CharLS decode is stored beside the fixture as a reference, and the generator records the CharLS version that produced it.

## Problem and reproducer

**Trigger.** A monochrome JPEG-LS frame, `.80` or `.81`, in which a run reaches the end of a line with RUNindex at 4 or more and fewer than `2^J[RUNindex]` samples left. That needs content flat across several runs: masks, segmentations, banded or flat regions, screen captures. The smallest measured trigger is a two-band 8 by 4 image at 16 bits. The trigger does not depend on an LSE marker: it has been reproduced at P 16 with the default LSE and at P 12 with no LSE at all.

**Observed.** `JpegLsDecoder::lossless().decode` returns `Ok(())` and fills `out` with values unrelated to the image from the first misread run onwards. In the folder viewer the frame drew as 255 grey. A truncated scan, or a stream with an edited LSE, also returns `Ok`.

**Expected.** The frame decodes exactly as CharLS decodes it. A scan that runs out of data, leaves a whole unread byte or ends at the wrong marker is refused. A preset the decoder does not apply is refused by name.

**Why a story.** It is the quietly wrong pixel. No existing corpus row or fixture reaches the trigger, which is why every test passes today.

## Scope, API and compatibility

In scope:

- `vendor/ritk-codecs-0.6.0/src/jpeg_ls/scan.rs`: the F.1 step 18 run loop, the miss past the line refused, and the end-of-scan check after the last row.
- `vendor/ritk-codecs-0.6.0/src/jpeg_ls/bitstream.rs`: the reader records that it ran out, records the offset of the marker that ended the segment, and reports the unread entropy bits before it.
- `vendor/ritk-codecs-0.6.0/src/jpeg_ls/context/mod.rs`: `default_thresholds` follows C.2.4.1.1.1's CLAMP on both arms.
- `vendor/ritk-codecs-0.6.0/src/jpeg_ls/encoder.rs`: the code segment A.16 mirror, the overshoot hit no longer raises `run_index`.
- `crates/ocelli-codec/src/jpegls.rs`: the LSE preset check on the single-component path, before any sample is written.
- `crates/ocelli-codec/src/registry.rs` and `lib.rs`: the new variant and enum, exported.

Public signature:

```rust
/// A JPEG-LS preset coding parameter, ISO/IEC 14495-1 C.2.4.1.1, that this
/// build does not apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JpegLsPreset {
    /// MAXVAL other than 0 or 2^P - 1.
    MaxVal,
    /// RESET other than 0 or 64.
    Reset,
    /// One or two of T1, T2 and T3 zero beside a non-zero one.
    PartialDefaultThresholds,
    /// Non-zero thresholds outside NEAR + 1 <= T1 <= T2 <= T3 <= MAXVAL.
    ThresholdOrder,
}

pub enum CodecError {
    // ... existing variants unchanged ...
    /// A JPEG-LS preset parameter this decoder does not apply.
    UnsupportedPreset(JpegLsPreset),
}
```

`Display` for the new variant reads "JPEG-LS preset parameter is not applied by this decoder: " followed by the parameter's name, `MAXVAL`, `RESET`, `partial default thresholds` or `threshold order`. `CodecError` stays `Copy`, `Eq` and `Hash`. Adding a variant breaks an exhaustive match downstream. The crate is 0.1.0 and unpublished while D-22 blocks publication, and no match on `CodecError` exists outside `crates/ocelli-codec`, measured with `grep -rln CodecError crates examples tools`.

Refusal policy. Every refusal leaves `out` exactly as the caller passed it. The preset check runs after the existing descriptor and header checks, so every refusal that exists today keeps its class and its precedence. It runs on the single-component path only. The colour path keeps refusing any LSE as `UnsupportedPixelFormat`.

Non-goals:

- Applying a non-default MAXVAL, RESET or partial default thresholds. Refused until a story implements them.
- An end-of-scan check on Ocelli's colour scan. It already refuses exhaustion, and the operator's scope is the vendored scan decoder. Recorded here so a review does not count it as missed.
- LSE IDs 2 to 4, restart intervals, line interleave, point transform. Already refused, unchanged.
- Any change to the JPEG 2000 module. F-239's.
- The folder viewer's error sentence. It stays "the frame could not be decoded" for every decode refusal, `examples/folder-viewer/src/preview.rs:245`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `JpegLsDecoder::lossless().decode`, `.80` mono | `jpegls.rs` then `ritk_codecs::decode_jpeg_ls_fragment` | NEAR 0, slope 1 and intercept 0 pinned | exact stored samples, or a typed refusal |
| `JpegLsDecoder::near_lossless().decode`, `.81` mono | same | NEAR above 0 | samples within NEAR, equal to CharLS's decode |
| `.80` RGB 8-bit | `sample_interleaved.rs` | unchanged | unchanged |
| Folder viewer preview and value endpoints | `examples/folder-viewer/src/preview.rs:1491` registers the adapter | unchanged | right pixels where they were wrong, `PreviewError::Decode` where a refusal is new |
| Oracle producer | `tools/oracle/src/produce.rs:1617` registers the adapter | unchanged | the new corpus rows decode |
| Benchmark subject `decode.transfer_syntax.jpegls` | `crates/ocelli-codec/examples/decode_jpegls.rs` | release only | median within its recorded band |
| `ocelli-wasm`, `ocelli-native` | none, they do not reach `ocelli-codec` | n/a | unchanged modules |

The vendored decoder owns the run index, the bit position and the threshold values. The adapter owns the preset policy and the error class. `sample_convert` still owns the stored-domain conversion.

## State transitions and invariants

Stateless. Each `decode` call builds a fresh vendored decoder and context model, and nothing persists between calls. The invariant that spans consumers is that a returned `Ok` means every sample was decoded from bits the encoder wrote, and a refusal leaves `out` untouched.

## Ownership, commit and recovery

Not applicable beyond the existing contract. The vendored decoder decodes into its own `Vec` and the adapter copies into `out` only after the whole decode and conversion succeed (`jpegls.rs:157` to `:159`), so a refusal at any new point leaves the caller's buffer as it was. AC9 asserts it for each new refusal.

## Numbered acceptance inventory

Two-band formula, used by every banded fixture and row: `s(r, c) = A if c < floor(cols / 2) else B`. At 16 bits `A = 224` and `B = 1064`, the reported file's two values. At 8 bits the implementer chooses `A` and `B` to reach the trigger named in the row, records them, and AC rows carry the recorded values. Each fixture's raw reference is generated from the formula in the generator, never by a codec.

**RED rule.** Each fixture or row that claims to reach a defect is checked on the base tree, with the relevant hunk alone reverted, and the number of wrong samples, or the `Ok` that should have been an `Err`, is recorded in the handoff. A fixture that does not go red with its hunk reverted is replaced, not kept.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Two-band 8 by 4, P 16, NEAR 0, CharLS through imagecodecs. Decode equals the formula exactly. T.87 F.1 step 18 | `two_band_eight_by_four_decodes_exactly` against `jpegls_two_band_u16_8x4.raw` | The present loop, measured 2 wrong samples with `Ok`. Raising RUNindex on the end-of-line hit | implementation |
| AC2 | Two-band 32 by 32, P 16, NEAR 0, the reported file's shape and values. Decode equals the formula exactly | `two_band_thirty_two_reproduces_the_reported_frame` against `jpegls_two_band_u16_32x32.raw` | The present loop, measured 916 wrong. A fix that only clamps `run_len` | implementation |
| AC3 | The same image at NEAR 3. Every sample within 3 of the formula, the maximum error recorded, and the output equal byte for byte to CharLS 2.4.4's decode. T.87 A.4.4 | `two_band_near_three_is_inside_the_bound_and_equals_charls` against the formula raw and `jpegls_two_band_u16_32x32_near3_charls.raw` | The present loop, which leaves the bound. A decode right only on the bound and wrong against CharLS | implementation |
| AC4 | Truncation. `jpegls_full_range_u16.jls` with 90, 50 and 10 per cent of its scan kept and its EOI restored, each refused as `DecoderFailure`. The unmodified fixture still decodes exactly, the positive control | `a_truncated_scan_is_refused` | `read_bits` returning 0, measured `Ok` all three times | implementation |
| AC5 | End of scan, as implemented by the lead's rulings after review pass 2. After the last sample the bits left before the marker must all be zero and at most 7, and that marker must be EOI, after any 0xFF fill bytes. Measured by the generated corpus of AC21 and by the two smallest damaged streams review pass 2 found opening to wrong samples | `ocelli_agrees_with_charls_on_the_generated_corpus`, `a_damaged_scan_that_reads_as_a_shorter_valid_scan_is_refused`, `fill_bytes_before_end_of_image_are_accepted` | A check that ignores leftover bytes. A 15-bit allowance, which opens damaged scans to wrong samples. A check that ignores non-zero padding. A check that refuses fill bytes | implementation |
| AC6 | Default thresholds, C.2.4.1.1.1. An 8-bit CharLS stream at NEAR 34, which carries no LSE, 32 by 32 in 4 by 4 blocks `s(r, c) = (10, 140, 250)[(r / 4 + c / 4) % 3]`, decodes equal byte for byte to CharLS's decode and inside the NEAR bound. Hand values cited in the test: P 8, NEAR 34 gives FACTOR 1, `T1 = CLAMP(105, 35, 255) = 105`, `T2 = CLAMP(177, 105, 255) = 177`, `T3 = CLAMP(259, 177, 255) = 177`. P 8, NEAR 0 gives `3, 7, 21`. P 16, NEAR 0 gives FACTOR 16 and `18, 67, 276`. P 16, NEAR 3 gives `27, 82, 297` | `default_thresholds_clamp_to_the_lower_bound` against `jpegls_banded_u8_near34.jls` and its CharLS raw | Saturating at MAXVAL, which gives T3 255. RED required with only the CLAMP hunk reverted. Corrected at implementation by the lead's ruling under the RED rule: no two-band 8-bit frame at NEAR 34 went red with only the CLAMP reverted, measured, so the block shape replaced it | implementation |
| AC7 | A miss past the line. Hand stream, P 8, 1 row by 13 columns, all zero, scan bytes `FF 30`: eight hits carry the run to 12 at RUNindex 8, then a miss whose two remainder bits `11` give 15, past 13. Refused as `DecoderFailure`. Positive control `FF 40`, eight hits and one partial hit, decodes 13 zeros. The generator records that CharLS refuses the first and decodes the second. T.87 F.1 step 18, with J from A.2.1 step 3 | `a_run_miss_past_the_line_end_is_refused` | Clamping to `remaining`, which returns 13 zeros and `Ok` | implementation |
| AC8 | LSE refusals by name, each an edit of `jpegls_corpus_lossless.jls`'s default LSE at P 16. MAXVAL 4095 gives `UnsupportedPreset(MaxVal)`. RESET 32 gives `Reset`. `T1 30, T2 0, T3 0` and `T1 0, T2 100, T3 0` give `PartialDefaultThresholds`. `T1 67, T2 18, T3 276` gives `ThresholdOrder`. Positive controls: MAXVAL 0, RESET 0, and all three thresholds 0, each alone, decode equal to the existing reference exactly, because at P 16 and NEAR 0 those defaults are the values the stream was encoded with. C.2.4.1.1 | `lse_presets_not_applied_are_refused_by_name` | Ignoring MAXVAL and RESET, measured `Ok` today. Treating any zero threshold as all three defaults. A check that refuses the defaults as well | implementation |
| AC9 | Every new refusal in AC4, AC5, AC7 and AC8 leaves `out` byte for byte as the caller filled it, `0xa5` | added to `structural_and_descriptor_failures_preserve_caller_output` | A partial write before refusal | implementation |
| AC10 | Existing behaviour. Every existing test in `crates/ocelli-codec/tests/jpegls.rs` passes unchanged, and every existing JPEG-LS fixture and corpus row decodes to the bytes it decoded to before | `cargo test -p ocelli-codec --test jpegls` on base and tip | A patch that moves a value that was right | implementation |
| AC11 | The vendored suite. `cargo test --manifest-path vendor/ritk-codecs-0.6.0/Cargo.toml --lib` passes with no published test edited, and its count is recorded on base and tip. A published test that fails stops the story and goes to the lead rather than being edited | handoff record | Editing the published suite to fit the patch | implementation |
| AC12 | Wasm unaffected, D-20. `bin/ocelli.sh gate wasm` passes with `ci/wasm-size-budget.json` and `ci/wasm-decode-size.json` unedited and `--accept-size` never run. Both module byte sizes are measured on base and tip and both deltas recorded, expected zero. `cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none`, and again with `--no-default-features --features decode-worker`, shows neither `ocelli-codec` nor `ritk-codecs` | handoff record, `docs/lld/build-targets.md` unchanged | A new edge from the shipped module to the codec crates | implementation |
| AC13 | Decode time. Release build, the existing runner `crates/ocelli-codec/examples/decode_jpegls.rs`, 31 samples of 4 decodes after one warm-up, on the same host, on base and tip, for every corpus JPEG-LS monochrome row: `jpegls_lossless`, `jpegls_near_lossless` and the two new banded rows. The example gains an optional argument naming the fixture, the default is the current one, so the subject's invocation and output are unchanged. `bin/ocelli.sh bench --compare` keeps `decode.transfer_syntax.jpegls` inside its recorded 15 per cent band with `ci/bench-baseline.json` unedited. The four base and tip medians are recorded in `docs/lld/benchmarks.md` | handoff and LLD | A slower decode hidden by re-baselining | implementation, on the operator's machine |
| AC14 | The standard. A reviewer checks every line under "Normative source, transcribed" for T.87 run mode (A.7.1 and F.1 step 18), J (A.2.1 step 3), C.2.4.1.1 with Table C.1, C.2.4.1.1.1, and stuffing (A.1 and the C.1.2 NOTE) against the published ITU-T T.87 text, and every hand value in AC6 and AC7 against it | review record | A plan transcription that is wrong being implemented faithfully | review |
| AC15 | Provenance and pins. `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md` gains a section for this patch naming the five patched source files and the one published test edited by the lead's ruling, what each change does with its T.87 clause, their SHA-256 digests, the measured vendored suite count, and that the encoder is fixed but unused by Ocelli. `scripts/pin_and_size_check.py` records the four digests in `RITK_PATCHED_HASHES`, moves the manifest digests to `RITK_MANIFEST_PATCHED_HASHES`, and updates `RITK_PROVENANCE_SHA256`, in the shape F-239's codec branch already uses so F-239's merge is a union of entries. The refusal text for a source file reads "has undeclared changes beyond its recorded patch". `bin/ocelli.sh gate pins` passes. A planted byte in each patched file makes it fail, with the file named, and the planted byte is reverted | `scripts/tests/test_pin_and_size_check.py` gains the planted-byte case for a source file | A patched file outside the recorded list. A gate that still accepts only the manifests | implementation |
| AC16 | D-20 amended in the same change as the patch. It stops saying the package is unchanged and says the `jpeg_ls` scan decoder, bit reader, default thresholds and encoder carry a recorded patch under F-243, enforced by the pins gate. Its F-ID column gains F-243. `python3 scripts/deviation_check.py` passes | DEVIATIONS diff | A register that says the source is unchanged while it is not | implementation |
| AC17 | Corpus rows. `scripts/corpus_synth.py` writes `syntax/reference_banded16.dcm`, Explicit VR Little Endian, 32 by 32, P 16, the two-band formula, and `syntax/jpegls_banded_lossless.dcm` under `.80` and `syntax/jpegls_banded_near_lossless.dcm` under `.81` at `JPEG_LS_NEAR`, both encoded from it through the existing pyjpegls producer. `corpus/manifest.tsv` gains the three rows. No DICOM is staged, the content gate passes, and `bin/ocelli.sh gate corpus` and `gate corpus-tests` pass. `generate_jpegls.py` extracts both to `jpegls_corpus_banded_lossless.jls` and `jpegls_corpus_banded_near3.jls`. The lossless one decodes equal to the reference row's Pixel Data, the near one inside NEAR and equal to its CharLS decode. Each goes red on the base tree under the RED rule, the count recorded | `manifest_backed_banded_rows_reach_the_run_mode_trigger` | A corpus that still cannot see the defect | implementation |
| AC18 | The oracle sees the new rows. On the operator's machine `bin/ocelli.sh gate oracle` renders the three rows through cornerstone3D 5.8.2. `tools/oracle/compare-expectations.json` and any other census file the run changes record the rows with the qualifiers the run reports, in the same change | oracle run record | A row the census does not account for | implementation, on the operator's machine |
| AC20 | The trigger without LSE. Two-band 32 by 32, P 12 in a 16-bit container, NEAR 0, **no LSE marker**, encoded by pyjpegls `encode_buffer` with `bits_stored` 12, the second reviewer's shape. The generator asserts the frame carries no LSE and is byte-identical to that reviewer's 97-byte frame. Decode equals the formula exactly. RED on the base tree, expected 915 wrong from sample 80 on, recorded | `twelve_bit_two_band_without_lse_decodes_exactly` against `jpegls_two_band_u12_32x32_no_lse.jls` and its formula raw | A fix aimed at LSE handling, which leaves this stream wrong. A fix that only covers P 16 | implementation |
| AC21 | Agreement with CharLS, the lead's ruling D3, as narrowed after review pass 2. `generate_jpegls.py` writes 150 streams, 148 encoded by CharLS 2.4.4 and 2 built by hand, and 61 mutations of each, with the damage class, CharLS's verdict and sample digest per case. Every undamaged stream that CharLS opens decodes to CharLS's samples, no case decodes to samples that are neither CharLS's nor the stream's own, and the count of each class's outcome is pinned | `ocelli_agrees_with_charls_on_the_generated_corpus` | The context RESET rounding toward zero. Accepting non-zero padding. A damaged scan opened to wrong samples. A count that moves unseen | implementation |
| AC19 | Provenance policy. The implementation follows T.87 and Ocelli's own `sample_interleaved.rs`. No CharLS source is opened for it, and dwv, Horos and Grok are never opened. `scripts/source_provenance_check.py` passes | review | Translating another decoder | review |

## Approach

Order, smallest first, each step with its RED recorded before its GREEN.

1. **Fixtures first.** `crates/ocelli-codec/tests/fixtures/generate_jpegls.py` gains an imagecodecs route beside its pyjpegls one and writes AC1, AC2, AC3, AC5, AC6 and AC20's codestreams with their formula raws, the two CharLS decode raws, AC7's two hand streams with CharLS's verdict on each recorded in the generator's check, and the extracted banded corpus rows once AC17's rows exist. It asserts each encode used the P and NEAR declared. It records `imagecodecs.__version__` and `imagecodecs.jpegls_version()` in its docstring and fails if the installed versions differ. The tests are written and seen red on the base tree.
2. **The run loop, `scan.rs`.** Replace `:179` to `:203` with the F.1 step 18 loop, the shape of `sample_interleaved.rs:146` to `:172`: `count = min(1 << J[idx], remaining - run_len)`, raise only when `count` is full, break at `remaining`, and on a miss refuse when `run_len + rem > remaining`.
3. **The encoder, `encoder.rs`.** In the overshoot branch at `:180` to `:187`, write the `1` bit and do not raise `run_index`.
4. **Exhaustion and end of scan, `bitstream.rs` and `scan.rs`.** `BitReader` gains a sticky `exhausted` flag set where `read_bits` now returns 0, the offset of the marker byte where `refill` stops, and a method giving the entropy bits left before that marker, counting a stuffed byte as seven. `decode_scan` ends with `bail!` when the reader ran out, when no marker was met or it is not at `data.len() - 2` with `FF D9`, or when the bits left exceed what the final byte and its stuffing byte can hold, item 4 above.
5. **CLAMP, `context/mod.rs`.** A local `clamp_t(i, j, maxval)` returning `j` when `i > maxval || i < j`, used in both arms of `default_thresholds`.
6. **The preset check, `jpegls.rs`.** `inspect_headers` records each LSE ID 1 payload it walks past, eleven bytes, `ID MAXVAL T1 T2 T3 RESET`. A new `validate_presets(header, presets)` runs after `validate_header` on the single-component path and returns the first failing `JpegLsPreset` in the order `MaxVal`, `Reset`, `PartialDefaultThresholds`, `ThresholdOrder`, with P from SOF55 and NEAR from SOS. An ID 1 payload whose length is not eleven is `InvalidCodestream`. Other IDs are left to the vendored parser, which refuses them.
7. **Provenance, pins and D-20.** AC15 and AC16.
8. **Corpus.** AC17, then the oracle run, AC18.
9. **Timing.** AC13 on base and tip.

Rejected: decoding single-component scans with Ocelli's own `sample_interleaved.rs` logic and dropping the vendored path. Larger, and it would re-derive the regular mode the vendored decoder already gets right on every measured stream. Rejected: refusing the CLAMP case instead of fixing it, item 5. Rejected: putting the LSE check in the vendored parser, item 1.

## Dependencies, write ownership and resources

Declared dependency F-028, done. Source-level: F-239's codec part writes `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md`, `scripts/pin_and_size_check.py`, its tests, `scripts/corpus_synth.py`, `scripts/tests/test_corpus_synth.py`, `corpus/manifest.tsv`, `docs/lld/codecs.md`, `docs/lld/corpus.md` and the guard catalogue. **This story lands first and F-239's codec branch merges onto it**, by the operator's order of 2026-11-07, so F-239 now depends on F-243. The merge into F-239 is a union: two `PATCH-PROVENANCE.md` sections, the two sets of digests in `RITK_PATCHED_HASHES`, one recomputed `RITK_PROVENANCE_SHA256`, and two sets of corpus rows. No patched file is shared, because F-239 patches `src/jpeg_2000/*` and `src/lib.rs` and this story patches only `src/jpeg_ls/*`. If `src/lib.rs` would need a change here, it does not: the new public items are in `ocelli-codec`, not in the vendored crate.

Files this story writes:

- `vendor/ritk-codecs-0.6.0/src/jpeg_ls/scan.rs`, `bitstream.rs`, `context/mod.rs`, `encoder.rs`
- `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md`
- `crates/ocelli-codec/src/jpegls.rs`, `src/registry.rs`, `src/lib.rs`
- `crates/ocelli-codec/tests/jpegls.rs`, `tests/fixtures/generate_jpegls.py` and the new fixtures under `tests/fixtures/`
- `crates/ocelli-codec/examples/decode_jpegls.rs`
- `scripts/pin_and_size_check.py`, `scripts/tests/test_pin_and_size_check.py`, and `scripts/guards/catalogue.py` if a probe names the changed refusal text
- `scripts/corpus_synth.py`, `scripts/tests/test_corpus_synth.py`, `corpus/manifest.tsv`, `corpus/README.md`
- `tools/oracle/compare-expectations.json` and any oracle census file the run changes
- `docs/hld/DEVIATIONS.md` D-20
- `docs/lld/codecs.md`, `docs/lld/corpus.md`, `docs/lld/benchmarks.md`
- `README.md`, `examples/folder-viewer/src/use.html`, `examples/folder-viewer/src/more.html`

Exclusive resources: the corpus regeneration and the oracle run need the ignored corpus and the operator's GPU host. Nothing else here needs a GPU.

## Verification and evidence reuse

Focused: `bin/ocelli.sh check ocelli-codec`, `cargo test -p ocelli-codec --test jpegls`, the vendored suite of AC11, `python3 crates/ocelli-codec/tests/fixtures/generate_jpegls.py` in check mode, `bin/ocelli.sh gate pins`, `gate corpus`, `gate corpus-tests`, `gate wasm`, `bin/ocelli.sh bench --compare`, `python3 scripts/deviation_check.py`, `python3 scripts/prose_check.py`. Then one feature profile run per story and `gate --sprint` once at sprint end, per the standing rule.

Evidence identities: imagecodecs 2026.10.10 with CharLS 2.4.4, pyjpegls 1.5.1, the base commit each RED was measured on, and the tip each GREEN was measured on. A fixture regenerated with a different CharLS invalidates AC3 and AC6's CharLS raws and their records. Timing invalidates on a host class change. A missing oracle observation stays an open row and does not become a pass.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Decode is outside the render loop, and the reader's new fields are scalars
- unsafe: none. The vendored crate's recorded external count, `scripts/unsafe_allowlist_check.py:103`, stays 0
- Tier A (WebGPU): n/a. Decode runs on the host CPU before any tier is involved, and its output is the same bytes on every tier
- Tier B (WebGL2): n/a, for the same reason
- Tier C (CPU): full. The decode is CPU code and the CPU tier draws what it returns

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | F.1 step 18 run index on CharLS streams, exact against the formula, AC1, AC2, and at P 12 with no LSE, AC20 | `crates/ocelli-codec/tests/jpegls.rs` |
| fixture | A.4.4 bound and CharLS equality at NEAR 3, AC3 | same |
| fixture | C.2.4.1.1.1 CLAMP with the hand values cited, AC6 | same |
| fixture | F.1 step 18 and A.2.1 step 3, the hand-built miss past the line and its positive control, AC7 | same |
| refusal | Truncation, unread bytes, wrong end marker, AC4, AC5 | same |
| refusal | C.2.4.1.1 presets refused by name with default positive controls, AC8 | same |
| refusal | Caller output preserved, AC9 | same |
| corpus | Banded rows decode right and reach the trigger, AC17 | same, from extracted rows |
| guard | Planted byte in a patched source file, AC15 | `scripts/tests/test_pin_and_size_check.py` |
| regression | Existing JPEG-LS tests and the vendored suite unchanged, AC10, AC11 | both suites |

## cornerstone3D 5.8.2 is the floor

cornerstone3D 5.8.2 decodes JPEG-LS through `@cornerstonejs/codec-charls` 1.2.5, which is CharLS. CharLS's decode is normative, so the floor for every stream it accepts is its exact output.

| Behaviour | cornerstone3D 5.8.2 | Ocelli today | Ocelli after this story | Evidence |
|-----------|---------------------|--------------|-------------------------|----------|
| Run reaching the line end on a partial segment, lossless, with or without an LSE | exact | wrong pixels, `Ok` | exact, equal to CharLS | AC1, AC2, AC17, AC20 |
| Same, near-lossless | CharLS output | wrong pixels, `Ok` | equal to CharLS output byte for byte | AC3, AC17 |
| Default thresholds at P 8 and NEAR 34 or above | CharLS output | wrong contexts | equal to CharLS output | AC6 |
| Truncated scan | CharLS refuses, the viewport shows no image | wrong pixels, `Ok` | refused, `DecoderFailure` | AC4 |
| Scan endings and damaged scans | CharLS 2.4.4 verdict and samples recorded for 9,259 generated cases | `Ok` with wrong samples on many | every undamaged stream that CharLS opens decodes to CharLS's samples, and no case decodes to samples that are neither CharLS's nor the stream's own. Refused where CharLS draws a damaged stream to other samples on 124. **Named gap against cornerstone, measured divergence under D14, by the lead's rulings:** refusals where CharLS draws the stream's own samples, 136 with 0x00 bytes inserted before EOI, because no rule over the leftover bits tells them from the 71 damaged scans that read as a shorter valid scan and opened to wrong samples, 79 with zero bytes inserted inside the scan, 8 with final bytes dropped and 8 with an EOI inserted before the last scan byte. Errors outside the coded range and over-long limited-length codes are refused per T.87 where review pass 2 measured CharLS opening some. 0xFF fill bytes before EOI are accepted, as CharLS opens them. `CORPUS_OUTCOMES` in `tests/jpegls.rs` pins every count | AC5, AC7, AC21 |
| LSE with non-default MAXVAL, RESET or partial thresholds on a conforming stream | CharLS applies them | wrong pixels, `Ok` | **gap, named.** Refused as `UnsupportedPreset`, by the operator's decision, never drawn differently | AC8 |
| LSE thresholds out of order | CharLS refuses | wrong pixels, `Ok` | refused, `ThresholdOrder` | AC8 |
| Better than the floor | n/a | n/a | the refusal names which preset, where cornerstone shows a generic decode failure | AC8 |

## Parity surface covered

`docs/hld/B-parity-surface.md:28`, "Transfer syntaxes", for `.80` and `.81` monochrome: decoded exactly as cornerstone3D's CharLS decodes them, on streams that reach the run-mode trigger. Non-default LSE presets remain unavailable and named.

## Deviations

- **D-20**, amended by this story when the patch lands, AC16. Its sentence that the package source is unchanged becomes false, and the amendment records the `jpeg_ls` patch and its enforcement.
- **D-31** is F-239's and is not changed. Its statement that each patch is listed in `PATCH-PROVENANCE.md` and the pins gate stays true for both patches after F-239 merges.

No new deviation. The row is amended in the implementing change rather than now, because writing that the source is patched before it is patched would make the register false in the other direction.

## LLD impact

- `docs/lld/codecs.md`, the JPEG-LS section: the run-mode rule and that the vendored decoder now follows it, the end-of-scan rule, the CLAMP fix, the refusal table gaining four `UnsupportedPreset` rows and three `DecoderFailure` rows, and the anchors gaining the CharLS decode for near-lossless parity.
- `docs/lld/corpus.md`: the three banded rows and why they exist.
- `docs/lld/benchmarks.md`: the four base and tip medians of AC13.

## User-facing documentation impact

- `README.md:182` to `:183`, the folder viewer's syntax sentence: a JPEG-LS frame with a non-default preset is refused rather than drawn.
- `examples/folder-viewer/src/use.html`, the paragraph after the codec table: JPEG-LS frames decode to CharLS 2.4.4's samples on every undamaged stream of the generated corpus that CharLS opens, a truncated or over-long scan is refused, and an LSE preset other than the defaults is refused as `CodecError::UnsupportedPreset` naming the parameter.
- `examples/folder-viewer/src/more.html:394`, the decoder registry paragraph: one sentence that JPEG-LS follows T.87's run mode and refuses a preset it does not apply.

## Decided by the operator on 2026-11-07

1. A new S46 story, done next, patching the vendored `ritk-codecs` JPEG-LS scan decoder.
2. Run mode per T.87 F.1 step 18 in the decoder and code segments A.15 and A.16 in the encoder, count `min(2^J[RUNindex], remaining)`, RUNindex raised only for a full segment. The encoder's mirror defect fixed or the encoder shown unused. **Lead's answer: fixed, and also unused by Ocelli**, item 6.
3. A scan that runs out of data or leaves more than the final partial byte before EOI is refused.
4. An LSE preset the decoder does not apply is refused by name: MAXVAL other than `2^P - 1`, RESET other than 64, a zero T mixed with non-zero ones, thresholds out of order. **Lead's reading:** zero MAXVAL and zero RESET select the defaults per C.2.4.1.1 and are accepted, item 3.
5. The patch recorded in `PATCH-PROVENANCE.md` and the pins gate, D-20 amended, and this story landing before F-239's codec part, which merges onto it.
6. CharLS through imagecodecs as a host tool, references from the formula, the named fixtures, and a banded corpus row under `.80` and `.81` from `scripts/corpus_synth.py`, never staging DICOM. **Lead's answer:** the corpus rows use the script's existing CharLS producer, pyjpegls, item 8.
7. Size and speed: wasm unaffected per D-20, decode time measured before and after. cornerstone3D 5.8.2 is the floor and is matched exactly.

Lead additions, each open to the operator's veto at review without changing another row: the CLAMP fix, item 5, and the refusal of a run miss past the line end, item 7.

## Open questions

None. The operator's decision settles scope, and the lead's choices above are stated with their reasons. AC14 is a reviewer evidence row, and AC13 and AC18 are measurements on the operator's machine with their procedures named, not open decisions.

### Lead rulings after review pass 1

Made by the sprint lead on 2026-11-07 after `.claude/reviews/F-243-working-pass-1.md`, which found a second wrong-pixel defect and showed the padding ruling's premise false.

- **D1, the RESET halving is fixed in this story.** `context/mod.rs` halves B with a floor, as CharLS 2.4.4 does, in the decoder and the encoder that share `update_context`. Ten frames encoded from real corpus CT slices returned Ok with up to 114,504 of 115,600 samples wrong on the base, so this is the defect class the story exists for. The fifth patched file's hunk grows, and PATCH-PROVENANCE and the pins record it.
- **D2, the padding ruling is withdrawn.** Bits after the last sample are not proven unread, because a flipped bit in the last three scan bytes changed samples while Ocelli returned Ok. Ocelli refuses what CharLS refuses at the end of a scan, padding values included. The published 2x2 test's scan byte `F8`, which CharLS refuses, is changed to `F0` in the vendored test, recorded in PATCH-PROVENANCE as an edit to a published test with its reason, and the vendored suite stays 320 of 320.
- **D3, the rule is agreement with CharLS, measured.** A deterministic generated corpus of CharLS streams and their mutations, including the reviewer's dropped-final-byte and bit-flip shapes, records CharLS 2.4.4's verdict and decoded samples for each. Ocelli must give the same verdict, and identical samples on every accepted stream. A stream CharLS opens and Ocelli refuses is a defect unless CharLS's samples differ from the encoded image, and any such exception is listed by name. Every sentence that claims parity states exactly what this test measures.
- **D4 and D5** are fixed with tests that kill the surviving mutants and a re-measured count.
- **AC14** stays owed to a reviewer with the published T.87 text. CharLS agreement on the generated corpus is the measured cross-check meanwhile.

### Lead rulings after review pass 2

Made by the sprint lead on 2026-11-07 after `.claude/reviews/F-243-working-pass-2.md`, which found every undamaged stream it made, 6,724 of them including streams from real corpus CT and MR slices, decoded exactly as CharLS decodes them. The story's defect, wrong samples on a valid stream, is closed. What remains is how damaged streams are treated.

- **D1, safety over parity on damaged streams.** The end-of-scan allowance is narrowed to 7 bits, which refuses all 71 damaged streams that opened to wrong samples and still decodes every undamaged stream that CharLS opens. The 136 damaged streams with one inserted 0x00 that CharLS opens and Ocelli now refuses are a named gap against cornerstone under D14, with the reason that no rule can tell them from the 71.
- **D2, fill bytes are accepted, other damage is named.** One or more 0xFF fill bytes before EOI are accepted, per T.87 D.2, which lets optional 0xFF fill bytes precede a marker, because they sit outside the coded data. Zero bytes inserted inside the scan, codes outside the coded range, and a dropped final byte or early EOI stay refused and are named classes under D14 with their measured counts.
- **No claim is made that damaged scans are always refused.** An entropy-coded stream can be damaged into another valid stream, and no decoder can detect that. The documents say what is measured: every undamaged stream of the generated corpus that CharLS opens decodes as CharLS does, and the named damaged classes are refused or accepted as listed.
- **D3 to D5.** Every sentence about CharLS states what CharLS was measured to do, every count is re-measured on the tree it names or deleted, and README and D-20 make no unqualified claim.

### Lead ruling after review pass 3

- **D1.** Every sentence claiming that every undamaged stream of the generated corpus decodes to CharLS's samples says instead "every undamaged stream that CharLS opens". That covers `PATCH-PROVENANCE.md`, AC21 and the parity row of this plan, `docs/lld/codecs.md` and the test comment in `crates/ocelli-codec/tests/jpegls.rs`. A grep for the old wording across the tree comes back empty. `RITK_PROVENANCE_SHA256` is pinned again, and `gate pins` is green.
- **Nitpicks taken.** N2, `codecs.md` says one `0x00` byte. N3, the fill test's comment says what was measured, not that the corpus records it. N4, the `bitstream.rs` comment says CharLS opens the scan rather than describing its internals. N5, the 6,724 figure is replaced by a sentence naming what the tree reproduces, or deleted. N8, plan item 8 drops the P 12 sentence if it is false.
- **Not taken.** N1, the refusals of fill before SOF or SOS and of `0xFF` after EOI are on the safe side and stay. N6 and N7 stay carried.

### Lead ruling after review pass 5

Pass 5 is clean. AC14 is checked against ITU-T T.87 and T.81 from itu.int by an independent reviewer, and AC18 runs the oracle on the operator's machine, both before completion. AC13's timing waits for the idle-host run at sprint close, with F-227's AC17, because this host's load makes a 15 per cent band unmeasurable today. The story is not completed until that run is recorded.

### AC18 record

`bin/ocelli.sh gate oracle` at `15d4817e` on the operator's machine: 105 views, 74 pass, 0 fail, 31 unmeasured, 0 absent, 29 mutations all detected, ALL GREEN in 185 s. cornerstone3D 5.8.2 renders both banded rows, deterministically across both passes, and the comparator measures each as a pass with no qualifier. So neither belongs in `tools/oracle/compare-expectations.json`, and the run changed no census file. The two banded rows render identical frames because the two-band image is flat in each band, and both the `.80` and `.81` streams decode to the values 224 and 1064 only, sample for sample equal. The `.81` row exercises the near-lossless path on the reported shape, not a visible near-lossless error.

### Lead ruling after the AC14 standard check

`.claude/reviews/F-243-ac14-standard-check.md` checked the transcription against ITU-T T.87 (06/1998) from itu.int. Every formula, constant, table value and rule agrees. AC6's and AC7's hand values agree. Three clause citations are wrong, and a wrong citation is a false sentence, so each is corrected wherever it appears. No arithmetic or control flow changes.

- **Run decoding.** The decoder's run procedure is T.87 Annex F, F.1 step 18, which is informative, and A.16 is the encoder's run-segment coding. Every place citing A.16, or A.15, for the decoder, or A.15 for the end-of-line hit that is A.16's branch, cites the right clause.
- **J.** The run-length order table J is T.87 A.2.1 step 3. Every place citing Table C.1 or A.7.1 for J, AC7 included, cites A.2.1.
- **Stuffing and padding.** These are T.87 A.1 and the C.1.2 NOTE, not C.2.1. Every place citing C.2.1 for them cites A.1.
- **Fill bytes.** T.81 B.1.1.2 could not be checked, because itu.int sells T.81. T.87 D.2 was checked and gives the same permission. Every place citing T.81 B.1.1.2 cites T.87 D.2 instead, and may keep T.81 B.1.1.2 only as the clause D.2 points to.
- **Scope.** This covers the plan, `docs/lld/codecs.md`, `PATCH-PROVENANCE.md`, the tests, the generator, the vendored files F-243 already patches, and `crates/ocelli-codec/src/jpegls/sample_interleaved.rs`. Comments that predate F-243 in a patched vendored file are corrected too, because the file is already in the recorded patch. A vendored file F-243 does not patch is left alone. Every changed hash is pinned again.
- **Omissions.** The decoder block gains F.1 step 18.c.ii, the RUNindex decrement after a miss, and Table C.1's MAXVAL range, both transcribed from the record.

AC14 is satisfied once these land and a fresh pass reviews them.

Erratum to the AC14 record: A.1 is on printed page 16 of T.87 (06/1998), not page 15. Outside that record, no committed text carries page 15.

### Lead ruling after review pass 6

- **D1 to D3.** Corrected as pass 6 gives them: `reconstruct` cites A.4.4 with the decoder's modulo wrap from F.1 step 14, the predictor block in `scan.rs` cites A.4.1 and loses the C.2.3 cite, the "All 8 modes" sentence and the MAXVAL/2 clause, and `pin_and_size_check.py` says code segment A.16. Every changed file is pinned again.
- **D4.** Corrected in this plan by the lead.
- **N1 taken.** The two false context-count comments in `context/mod.rs` say what the code holds, or go. N2 taken.
- **The run miss at the line end.** The implementer builds a stream whose run-interruption remainder lands exactly on the line end, decodes it with CharLS run as a tool and with Ocelli, and reports both. No code changes on this until the lead rules.

### Lead ruling after review pass 7

- **D1 to D4.** Corrected as pass 7 gives them. `read_golomb`'s comment names both callers' limits, LIMIT from A.2.1 for a regular sample and glimit from A.7.2 item 8 for a run-interruption sample, and loses the upstream em-dash and semicolon. `compute_k` states code segment A.10's rule, the smallest k with `N * 2^k >= A`, and loses the floor invariant. RANGE's comment gives the A.2.1 step 1 rule for NEAR > 0 as well. `ContextState::default` is not called the standard's initial state. Pass 7's N1, N2 and N3 are taken too, and the `Predictor::None` comment says what the code returns.
- **The cap on k.** `compute_k` caps k at qbpp, and A.10 has no cap. The implementer establishes whether the cap can bind on any stream, by an argument from A.2.1 and A.6's bounds on A and N, checked by counting in a scratch build across the generated corpus and the damaged sweep. If it can bind, it is reported, and no code changes until the lead rules.
- **Where the sweep stops.** Pass 7 confirms every clause citation in the five patched files agrees with T.87, which is what `PATCH-PROVENANCE.md` claims. After this round, a further false upstream comment that is not a citation and not on a line F-243 changed is recorded in a backlog story at sprint close, an audit of the vendored JPEG-LS comments against T.87, and does not block F-243. A wrong line F-243 wrote or rewrote still blocks.

### Operator disposition for AC13, 2026-11-09

This later disposition changes only AC13's completion dependency. It supersedes the pass 5 statement that F-243 cannot complete until that timing run is recorded. The historical text and measurements remain. All other acceptance criteria, verification requirements, baselines, bands, host labels and gates stay unchanged.

The operator has approved F-243 formal completion with a named outstanding AC13 external host and instrument gap. The available Darwin 27 host and actual instrument differ from the recorded Darwin 25.5 host and instrument. The registered comparison verdict remains incomparable, not a pass. A matching host and instrument run, with the idle-host qualification, remains owed after completion.

This disposition permits completion with that gap. It does not itself record the story as completed, does not claim the owed run happened, and does not satisfy F-239's dependency on F-243 by this text. F-239's own acceptance checks stand unchanged.

The 2026-11-09 read-only source binding stays a dated inspection, not a current equivalence claim. Nothing here revises the earlier ramp observations or the historical benchmark record. Evidence remains under the existing F-243 acceptance handoff.
