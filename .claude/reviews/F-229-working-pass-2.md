# F-229 review, pass 2

**Reviewed**: working tree of `work/f-229-xai` in `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-229`, HEAD and merge base `4165d1e8b4b63b3d87d271cfc2a6c16c10497744` (`git merge-base HEAD sprint/s46`). Staged plus unstaged diff against that base, 49 paths, `git diff <base> -- . ':!tools/oracle/out' | shasum -a 256` = `6309e542cd9b839c370731019f8de8f74f9bd66903820b3eeffa112b299c6823`, index tree `299fc78d752c8a4d89482b9556ad4c0267cc08c2`. Untracked: `.claude/reviews/F-229-working-pass-1.md` and `tools/oracle/out` only. Both values were the same at the start and the end of the pass. Library half only.
**Author / reviewer**: remediation by GLM-5.3 in opencode (log `glm-F-229-fix2.log`). D-21 and D-28 edited by the sprint lead. Reviewed by an independent Claude reviewer that wrote none of this material and did not write pass 1.
**Result**: 6 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **Pass 1 D1, colour encoding, rebuilt independently.** My own Python bit writer, not the test's rewriter, rewrote `jpegxl_lossless_rgb8.jxl` (bit 34) and `jpegxl_modular_xyb.jxl` (metadata `all_default` at bit 25, `xyb_encoded` kept 1) to every `ColourSpace` enumerant 0 to 3 and to the invalid values 4, 17 and 18 (long enum coding). `jxl-image`'s own `ImageHeader::parse` confirmed each rewrite parsed as intended. Controls: the `Rgb` rewrite of the RGB fixture decodes under `.110` to exactly `jpegxl_reference_rgb8.raw`, and the `Rgb` rewrite of the XYB-encoded fixture decodes under `.112` to exactly the original fixture's output, so both rewriters are sound. Results: `Xyb` and `Unknown` refuse `UnsupportedPixelFormat` under `.110` and `.112` with `out` untouched, including the dangerous case of an XYB-encoded lossy frame whose target encoding is XYB. `Grey` under 3 samples refuses `FrameMismatch`. Values 4, 17 and 18 refuse `InvalidCodestream` from `jxl-oxide`'s `InvalidEnum`. Closed.
- **The test's rewriter.** `rewrite_colour_encoding(AllDefault)` reproduces the fixture byte for byte (asserted in the test). Shortening the metadata tail copy by one bit, or copying the frame from byte 4 instead of 5, each turns `codestream_colour_space_other_than_rgb_or_grey_refuses` red. Removing the colour check, or admitting `Xyb` or `Unknown`, each turns it red.
- **The ICC pre-check, differential against the parser it protects.** I compiled a verbatim copy of `jpegxl_icc.rs` (only `pub` added to two functions) beside `jxl-image` 0.13.0 `ImageHeader::parse` followed by `jxl-bitstream` 1.1.0 `read_u64`, which is exactly what `jxl-oxide` 0.12.6 `try_init` (`lib.rs:354` to `:366`) runs before `jxl-color` sizes its buffer. Over 5 million random headers (lengths 8 to 1,024 bytes after the signature, splitmix64) the walker disagreed with the reference on 1,609 of 15,529 ICC declarations. Every disagreement traced to one of two fields (D1). With those two fields corrected in the scratch copy, a fresh 5 million headers gave 15,371 of 15,371 agreements. This rejects the claim that the walker mirrors `jxl-oxide`.
- **The bits-not-bytes bound.** As a memory bound it is sound once the walker agrees with the parser: the encoded buffer is at most eight times the remaining codestream, whatever coding the ICC stream uses, so LZ77 is not a way around the allocation bound. It is not a sound validity criterion. Three libjxl 0.12.0 encodes of the Display P3 profile with a zero-filled private tag (284, 282 and 285 bytes, ICC 4,560 to 200,560 bytes) are read and rendered by `jxl-oxide` and refused here as `InvalidCodestream` (D3).
- **Peak RSS, measured with `/usr/bin/time -l` on a release probe calling `JpegXlDecoder::general().decode`.** The 26 byte crafted stream with the pre-check: 2,310,144 bytes, and 2,293,760 in container form. Three variants that the pre-check passes: 271,073,280 bytes each (D1, D2).
- **Never panics, never out of bounds, never loops.** The walker copy, built in debug with overflow checks, ran every truncation, every single-bit flip and 200,000 random 1 to 4 byte mutations with random truncation of each of the 13 fixtures and seven crafted files, about 4 million inputs, plus 400,000 random headers up to 3,000 bytes. No panic. Every loop in the walker is bounded by input length or by a constant.
- **Pass 1 D3, S1, S2 replayed with their mutations** (scratch copy of the tree, release test profile). D3: numerator 5 fails the container, `jpegxl_lossy_stays_within_the_recorded_bound` and `lossless_only_refuses_vardct_and_xyb`. 6 fails the latter two. 12 fails the lossy bound only. 13 passes. That matches the corrected comment at `jpegxl.rs:42` to `:52`. S1: removing `if value > max` turns `samples_above_declared_bits_stored_refuse` red. S2: copying half the frame on success turns `malformed_jpegxl_never_panics` red. All closed.
- **The corruption scan collects.** Injecting a panic for every input whose length is 13 mod 97 produced exactly 41 collected failures, which is the hand count over the 13 case lengths (17, 1, 3, 3, 1, 1, 1, 3, 1, 1, 1, 1 and 7), so every case contributed and the scan does not stop at the first panic.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | reviewed tree | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | reviewed tree | now | 0 |
| `cargo test -p ocelli-codec -p ocelli-pixel -p ocelli-dicom -p ocelli-viewport` | reviewed tree, debug | now | 0, 44 result lines all ok. `jpegxl` 15 passed in 170.7 s (scan included), codec lib 12, `registry` 18, `frame_open` 25, `dicomweb` 14, `color` 16 |
| `bin/ocelli.sh gate licences`, `corpus`, `corpus-tests`, `guards`, `wasm` | reviewed tree, corpus present | now | all ALL GREEN. `gate wasm` prints "wasm size within budget" |
| `bin/ocelli.sh wasm` | reviewed tree | now | 0 |
| `guard_census.py`, `--check-runbook`, `deviation_check.py`, `prose_check.py`, `source_provenance_check.py` | reviewed tree | now | all 0. 0 watched by nothing, 28 deviations, 864 prose files, 1,710 provenance files |
| `git diff <base> -- THIRD_PARTY_LICENSES THIRD_PARTY_NOTICES` | reviewed tree | now | empty |
| CLAUDE.md counts | `corpus/manifest.tsv`, `ocelli-codec/src` | now | 19 distinct syntaxes in the manifest, 6 `pub fn register_*`, `registry.rs` asserts 17 available and `[DEFLATED, JPEGXL_RECOMPRESSION]` unavailable. The "jxl-oxide issue 512" citation was not checked (no network use) |
| Differential walker probe | verbatim copy of `jpegxl_icc.rs` against `jxl-image` 0.13.0 | now | 1,609 of 15,529 disagree on the tree, 0 of 15,371 with the two corrections |
| Crafted streams through `JpegXlDecoder::general()` | `craft.py`, `slow.py`, `bigicc.py` in scratch | now | see D1 to D5 |
| Mutation catalogue | scratch copy, `cargo test --release -p ocelli-codec --test jpegxl` and `--lib` | now | 23 mutations, control green. Survivors listed in D4 and S1 |
| GLM's 270,974,976 byte RSS for the unguarded 26 byte file | log | prior receipt | not replayed for that exact file. Three variants reaching the same `resize` were measured at 271,073,280 |

## Defects

### D1, the pre-check walker desynchronises from `jxl-oxide` on two fields, and a 56 or 29 byte stream reaches the 256 MiB allocation
**Where**: `crates/ocelli-codec/src/jpegxl_icc.rs:249` to `:251` and `:286`
**What**: (a) When `default_m` is 0 the walker reads `cw_mask` and stops. `jxl-image` 0.13.0 `lib.rs` `ImageMetadata` then reads `up2_weight` (15 F16), `up4_weight` (55) and `up8_weight` (210) for each set bit, so the walker reads `enc_size` from the middle of the weights. (b) `PreviewHeader` in `jxl-image` reads `width` whenever `div8` is 0 (`cond(!div8)`), and the walker reads it only when `ratio == 0` as well. Either way the walker reports a wrong `enc_size` and the pre-check passes a stream whose real declaration is `1 << 28`.
**Why it is wrong**: the module exists to bound a buffer that `jxl-oxide` sizes (`jxl-color` 0.11.0 `icc/decode.rs:66`), so it must read the fields exactly as `jxl-oxide` does. Its own header says it walks "in the order `jxl-bitstream` reads", and `jpegxl.rs:14` to `:16` says the declared size "is bounded separately, before any parsing". Both are false on these inputs.
**Evidence**: the 26 byte crafted stream with `default_m 0, cw_mask 1` and 240 zero bits inserted (56 bytes): walker `Ok(Some((0, 46)))`, reference `(268435456, 326)`, decode `Err(InvalidCodestream)` with peak RSS 271,073,280. A preview with `div8 0, ratio 1` (29 bytes): walker `(0, 143)`, reference `(268435456, 110)`, same RSS. Container forms of both: same. The differential run above attributes all 1,609 disagreements to these two fields.
**Scope and regression**: present on the tree. Every `.110` and `.112` caller. Smallest regression: a test that compares the walker with `jxl_image::ImageHeader::parse` plus `read_u64` over seeded random headers (`jxl-image` is already in the lock), and the two crafted streams expecting the refusal. See also D3 for a remedy that removes (a) entirely.

### D2, a container box whose declared size outruns the input skips the pre-check, and `jxl-oxide` parses the partial box
**Where**: `crates/ocelli-codec/src/jpegxl_icc.rs:67` to `:71` and `:107` to `:108`
**What**: `split_box` returns `None` for a box longer than the input and `codestream_bytes` then returns `None`, which the caller treats as no finding ("left to `jxl-oxide`", `:53` to `:54`). `jxl-oxide`'s container parser streams a partial `jxlc` payload, reaches `read_icc` and resizes.
**Why it is wrong**: same contract as D1. A truncated container is also the most ordinary malformed input, which is exactly what the corruption scan generates.
**Evidence**: the 26 byte crafted stream in a `jxlc` box declaring 1,000 bytes more than present (46 bytes): `exceeds=false`, decode `Err(InvalidCodestream)`, peak RSS 271,073,280. The same stream split over two complete `jxlp` boxes is caught (2,359,296).
**Scope and regression**: present on the tree, container form only. Collect the available prefix of a box that outruns the input instead of abandoning the walk, with this 46 byte stream as a fixture.

### D3, valid libjxl streams with a compressible ICC profile are reported as invalid, and the "at least one bit per byte" claim is false
**Where**: `crates/ocelli-codec/src/jpegxl_icc.rs:29` to `:31` (claim), `:47` (comparison), `crates/ocelli-codec/tests/jpegxl.rs:615` ("backed by codestream bytes")
**What**: "each of its `enc_size` output bytes consumes at least one bit" does not hold for the JPEG XL entropy coder, whose ANS and prefix codes can spend well under one bit on a probable symbol. Three libjxl 0.12.0 encodes of the Display P3 profile plus a zero-filled `zzzz` private tag declare `enc_size` 4,412, 50,415 and 200,415 into roughly 1,800 to 2,200 remaining bits. `jxl-oxide` 0.12.6 reads all three, recovers the 4,560 to 200,560 byte profile and renders. Ocelli returns `InvalidCodestream`, whose text is "encoded frame is invalid or truncated" (`registry.rs:351`).
**Why it is wrong**: AC13 and the plan's decision 6 say an ICC colour encoding refuses `UnsupportedPixelFormat`, which reports unavailable. A valid file is reported as corrupt. The module doc's "changes when the refusal happens, never whether" holds, but the refusal kind is what callers report.
**Evidence**: `bigicc.py` (imports `encode_with_libjxl` from `generate_jpegxl.py`), then the probe: walker and reference agree on each `enc_size`, `exceeds=true`, decode `Err(InvalidCodestream)`. The tracked `jpegxl_icc.jxl` still gives `UnsupportedPixelFormat`.
**Scope and regression**: present on the tree. The smallest remedy also closes D1(a), D5 and the size question: every `want_icc` stream already refuses, so the pre-check can refuse `UnsupportedPixelFormat` as soon as the walker reads `want_icc 1`, with no size comparison and no walk past the colour encoding. Regression: the 284 byte libjxl stream expecting `UnsupportedPixelFormat`, and deletion of the false sentence.

### D4, the integration test for the pre-check passes with the pre-check removed
**Where**: `crates/ocelli-codec/tests/jpegxl.rs:611` (`declared_icc_larger_than_the_codestream_refuses`), guarding `crates/ocelli-codec/src/jpegxl.rs:217`
**What**: replacing the call with `if false && ...` leaves all 15 `jpegxl` tests green, because `jxl-oxide` also returns `InvalidCodestream` on the 26 byte stream, after allocating 256 MiB. The three unit tests in `jpegxl_icc.rs` test the function and not the call site.
**Why it is wrong**: microscope section 2, a test that passes both ways is a defect, and HLD 27.2 requires the mutation to be seen red. The wiring is what bounds memory.
**Evidence**: mutation "pre-check call removed": exit 0, 14 passed, 1 filtered (the scan).
**Scope and regression**: present on the tree. Under the D3 remedy, an ICC stream whose entropy data `jxl-oxide` rejects (it returns `InvalidCodestream`) while the pre-check returns `UnsupportedPixelFormat` would discriminate. Under the present design, any observable that differs, for example a stream `jxl-oxide` would accept.

### D5, a malformed ICC declaration makes header parsing quadratic: 64 KiB takes 70 seconds
**Where**: `crates/ocelli-codec/src/jpegxl.rs:263` to `:295` (`read_header`), with `jxl-oxide` `try_init` re-running `read_icc` on every call
**What**: `read_header` feeds one more byte per `try_init`, and each `try_init` re-parses from the start, resizes the encoded ICC buffer to `enc_size` and decodes until it runs out. The pre-check now caps `enc_size` at the remaining bits, which still lets it grow with the input. A stream declaring `enc_size = 8 * len - 200` followed by zero bytes, passed by the pre-check, took 0.24 s at 4 KiB, 4.03 s at 16 KiB and 69.65 s at 64 KiB in a release build, peak RSS under 5 MB. Growth is quadratic, so a 1 MiB frame would run for hours.
**Why it is wrong**: one malformed frame can hold a decode worker for minutes, or hours at a megabyte. This is the time half of the resource bound the pre-check was added for. Pass 1 N4 named it as plausible and unmeasured. It is now reproduced, so it is counted.
**Evidence**: `slow.py 4096 | 16384 | 65536`, then `/usr/bin/time -l probe decode slow_N.jxl 112 64 96 3 8 8 RGB`, each `Err(InvalidCodestream)`.
**Scope and regression**: present on the tree, and worse without the pre-check. The D3 remedy closes it for ICC. A timing bound on the 64 KiB stream, or a count of `try_init` calls, is the regression.

### D6, D-28 says "the three PS3.5 A.4.12 syntaxes `.110` and `.112`"
**Where**: `docs/hld/DEVIATIONS.md:45`
**What**: the sprint lead's correction removed `.111` from the list and kept "three", so the sentence names three syntaxes and lists two.
**Why it is wrong**: a false claim in the normative deviation register. `.111` is known and unregistered (`registry.rs` asserts it).
**Evidence**: the row text, and `register_jpegxl_decoders` registering two UIDs.
**Scope and regression**: prose only. "for the two PS3.5 A.4.12 syntaxes it registers, `.110` and `.112`".

## Smells

### S1, most of the walker's fields are executed by no test
**Where**: `crates/ocelli-codec/src/jpegxl_icc.rs` `tone_mapping`, `preview_header`, `animation_header`, the intrinsic size, `extra_channel` names, the `cw_mask` branch, `opsin_inverse_matrix`, and the `jxlp` index skip at `:76`
**What**: the only ICC-declaring inputs in any test are the one crafted header (no extra fields, no extra channels, `default_m 1`) and `jpegxl_icc.jxl`. Deleting the tone mapping read, the extra-channel name loop, the whole `cw_mask` read, or the `jxlp` index skip leaves the unit tests and the integration tests green. Changing the preview width condition also stays green.
**Mechanism**: a hand-written bit reader whose position is wrong in any untested branch silently reads `enc_size` from the wrong place, which is how D1 got through. The next edit to any of these branches has no detector.
**Regression**: the differential test proposed in D1 covers every branch at once, because random headers reach all of them (my run reached each).

## Nitpicks

- **N1.** `codestream_bytes` copies a container's whole codestream into a `Vec` before decode. It is bounded by the input, but D-21's encoded-input copy is worded for a third-party API that requires owned bytes, which this is not. Only the header prefix is needed.
- **N2.** An 8 by 8 XYB-encoded frame retargeted to `Grey` decodes `Err(DecoderFailure)` under `.112` with Samples per Pixel 1, which looks like the fixed overhead exceeding 26 times a 64 byte output. A tiny but valid frame refuses as a decoder failure. Not exercised beyond that one rewrite.
- **N3.** The crafted 26 byte builder exists twice, in `jpegxl_icc.rs` tests and in `tests/jpegxl.rs`, and the two can drift.

## Verified clean

- Pass 1 D1 (colour) closed: every valid enumerant through both syntaxes, both rewriters controlled by an exact decode.
- Pass 1 D2 closed for D-21: "`.111` is known and unavailable, so no reconstructed JPEG is ever built" and the tracker scope sentence are true. D-28 is D6 above.
- Pass 1 D3, S1 and S2 closed, each mutation seen red.
- The walker is correct field by field against `jxl-image` 0.13.0 and `jxl-bitstream` 1.1.0 for the signature, `SizeHeader`, `ImageMetadata` up to `cw_mask`, `BitDepth` (both branches), `num_extra` `U32(0, 1, 2 + u(4), 1 + u(12))`, `ExtraChannelInfo` (default alpha, enum, bit depth, `dim_shift`, `Name` `U32(0, u(4), 16 + u(5), 48 + u(10))`, alpha, spot and CFA tails), `ColourEncoding` (`want_icc` then the colour space enum, no further fields when `want_icc`), `ToneMapping` (gated on `extra_fields`), `Extensions` (U64 mask, up to 64 U64 lengths, then skip), `OpsinInverseMatrix` (16 F16), `AnimationHeader`, `U64` and the enum coding. The two exceptions are D1.
- It never panics or reads out of bounds on about 4.4 million inputs in a debug build. All 13 tracked fixtures return no finding, and the bits form keeps `jpegxl_icc.jxl` on `UnsupportedPixelFormat` (the bytes form turns `unsupported_codestreams_report_unavailable` red). The crafted declaration refuses in bare and container form at 2.3 MB peak RSS.
- The scan: 15 `jpegxl` tests green in 170.7 s debug, the two-sentinel check catches a partial write, and failures are collected across all 13 cases.
- Licences gate, notices byte-identical, wasm size budget, CLAUDE.md counts, census, runbook, deviation, prose and provenance checks.

## Limitations and handoff

- **Not exercised**: the oracle, any GPU or browser job, a wasm32 execution. `jxl-color` `decode_icc` also calls `Vec::with_capacity(output_size)` with `output_size` up to `1 << 28` (`icc/decode.rs:222`) for an `enc_size` of about 140 bytes, which the bits bound passes. On native that reserve is not resident, on wasm32 it would grow linear memory. Read from source and not constructed, so not counted. The D3 remedy closes it too.
- **Carried from pass 1, unchanged and outside this half**: `tools/oracle/unsupported.json` (AC24), `docs/SOURCE-POLICY.md` rows and the AC25 audit, `docs/lld/codecs.md`, `README.md`, `use.html` and `more.html`, the viewer registration, AC20.
- **Scratch**: probe crate, mutation copy, crafted files and logs under `target/f229-p2-scratch`, built with target directories inside it, deleted at the end. Diff hash and index tree unchanged. This report is the only file written in the worktree.
- **Next**: the implementer remediates D1 to D6 and S1, then a fresh independent pass 3. This is a bounded library-half review, not acceptance of F-229.
