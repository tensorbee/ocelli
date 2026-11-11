# F-229 review, pass 1

**Reviewed**: working tree of `work/f-229-xai` in `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-229`, HEAD and merge base `4165d1e8b4b63b3d87d271cfc2a6c16c10497744` (`git merge-base HEAD sprint/s46`). Staged plus unstaged diff against that base, 47 paths, `git diff <base> | sha256sum` = `baf357e1cf2751d58c1a50f7c5b451d97de8ac94c9c487f85b097daf755725a6`, index tree `3fa2c0b3b0f2147f25b874432459fe58d3b41394`. No untracked files outside `tools/oracle/out`. Both values were the same at the start and the end of the pass.
**Author / reviewer**: implemented by an external coding agent and then a Claude implementer. Reviewed by an independent Claude reviewer that wrote none of this material. Library half only.
**Result**: 3 defects, 2 smells, 6 nitpicks

## Acceptance challenged

- **AC4, AC5, AC6, re-derived by hand from the generator formulas, not from a decode.** `scripts/corpus_synth.py` `ramp` and `rgb_ramp` give `v[i] = i*65535 // 6143`, `v[i] = i*4095 // 6143`, and red `c*255//95`, green `r*255//63`, blue `(red+green)//2`. Hand values: 16-bit index 1000 is 10668 (6143 times 10668 is 65,533,524, remainder 1,476), index 6143 is 65535. 12-bit index 2 is 1 and index 3000 is 1999 (6143 times 1999 is 12,279,857, remainder 5,143). RGB at row 10, column 50 is (134, 40, 87). `JpegXlDecoder::lossless()` returned exactly those values. All three tracked `.raw` references also match the formulas at every sample, so the references are independent of JPEG XL. This rejects the `write_to_buffer::<u16>` rescale (16 at 12-bit index 2) and any byte-order or stride slip.
- **AC9, the quantiser, independent cases.** `floor(v*(2^BS-1)+0.5)` clamped: 0.75 at 12 bits is 3071 (3071.75), 0.001 at 8 bits is 0 (0.755), 0.25 at 16 bits is 16384 (16384.25), `f32` 0.99999994 at 8 bits is 255 (255.49998), `f32::MAX` clamps to 255, `f32::MIN` and -1e-30 clamp to 0. All matched. The `f64` product of a 24-bit mantissa and a 16-bit multiplier is exact, and the binary-search floor is correct over `(0, max+1)`. Truncation mutation is red.
- **AC4 range check, constructed.** A single-bit header patch of `jpegxl_lossless_16.jxl` (byte 52 bit 6) gives a codestream declaring 15 bits per sample whose modular data still holds the 16-bit ramp. Under Bits Stored 15 the shipped code refuses with `UnsupportedPixelFormat`. With the range check removed it returns `Ok` with stored value 65535 in a 15-bit field and the whole suite stays green. See S1.
- **Colour, AC6 and AC14, discriminating case.** A header rewrite of `jpegxl_lossless_rgb8.jxl` whose enum colour encoding declares `ColourSpace::Xyb` decodes `Ok` under `.110` and `.112`, unconverted, reported `Rgb`. See D1.
- **AC7b, `.111`.** Intact, bit-flipped and one-byte inputs return `KnownUnavailable` and leave `out` untouched (test green). Mutation that registers a `.111` adapter forwarding to the general decoder turns `jpegxl_recompression_is_unavailable` red.
- **AC16, the scan.** 13 cases, 3,718 truncations (sum of the 13 fixture lengths, measured) and 26,000 flips, green in 123.6 s in debug. Injecting a panic for every input whose length is 13 mod 97 produced 41 collected failures across all 13 cases in one run, so the scan collects and does not stop at the first panic.
- **AC17, allocation bound.** Setting `ALLOC_LIMIT_NUMERATOR` to 1 fails six tests with `DecoderFailure`, so the tracker is wired into the real decode. Thresholds measured in D3.
- **Refusals, each mutated and seen red**: VarDCT under `.110`, `xyb_encoded` under `.110`, bits-per-sample against Bits Stored, orientation, PQ, keyframe count, signed Pixel Representation, trailing data, truncating quantiser.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | reviewed tree | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | reviewed tree | now | 0 |
| `cargo test -p ocelli-codec -p ocelli-pixel -p ocelli-dicom -p ocelli-viewport` | reviewed tree, debug | now | 0 failures. `jpegxl` 12 passed, `registry` 18, `frame_open` 25, `color` 16 |
| `bin/ocelli.sh gate pins`, `licences`, `corpus`, `corpus-tests`, `guards` | reviewed tree, corpus present | now | all 0. corpus-tests 23 and 65 passed |
| `bin/ocelli.sh wasm`, `bin/ocelli.sh native` | reviewed tree | now | 0 and 0. native step 2 builds the shared crates, `ocelli-codec` with `jxl-oxide` included, for wasm32 |
| `python3 scripts/guard_census.py`, `--check-runbook`, `deviation_check.py`, `sync_agent_skills.py --check`, `prose_check.py`, `source_provenance_check.py` | reviewed tree | now | all 0. 28 deviations, 864 prose files, 1709 provenance files |
| `cargo deny check` | reviewed lock | now | 1. Only `advisories` fails, on `encoding` RUSTSEC-2021-0153 through `dicom-transfer-syntax-registry`, which predates F-229. `licenses`, `bans`, `sources` ok |
| `git diff <base> -- THIRD_PARTY_LICENSES THIRD_PARTY_NOTICES` | reviewed tree | now | empty, and gate `licences` regenerates and matches the lock |
| `cargo tree -p ocelli-wasm` and `-p ocelli-native`, `--target all` | reviewed lock | now | 0 lines matching `jxl`, `brotli` or `ocelli-codec` |
| `cargo tree -p ocelli-codec --target wasm32-unknown-unknown` | reviewed lock | now | no `rayon` |
| `.venv/bin/python crates/ocelli-codec/tests/fixtures/generate_jpegxl.py` | check mode, imagecodecs 2026.10.10 | now | 0, "jpeg xl fixtures match", lossy error 18, bound 27 |
| Grep of the tree and of `jxl-oxide` 0.12.6 for `reconstruct_jpeg`, `.reconstruct(` | tree and registry source | now | no Ocelli caller. The only `jxl-jbr` reconstruction call is `jxl-oxide/src/lib.rs:900` inside `reconstruct_jpeg`. `jxl-jbr/src/reconstruct.rs:457` is the `expect("is_last not found")` the plan names |
| Mutation catalogue on `jpegxl.rs` | scratch copy outside the worktree, release test profile | now | 13 mutations. Range check and `sample < 0` survive, all others red |
| Mutations of `corpus_check.py` (three refusals) and `corpus_synth.py` (restore check) | scratch copy | now | each red in `test_corpus_check.py` or `JpegXlEncoderRefusals`, control green |
| Implementer's scan counts and 2026-11-05 allocation measurement | progress report | prior receipt | counts reproduced, allocation comment contradicted (D3) |

## Defects

### D1, a codestream that declares XYB or Unknown colour space is reported as RGB without conversion
**Where**: `crates/ocelli-codec/src/jpegxl.rs:305` (`validate_header` colour check) and `:169` (`decode_photometric_interpretation`)
**What**: `validate_header` refuses an ICC encoding and PQ or HLG only. An enum colour encoding whose `colour_space` is `Xyb` or `Unknown` is accepted. `jxl-color` 0.11.0 `convert.rs:106` treats XYB to XYB as equivalent, so the transform is a no-op, and `jxl-render` 0.12.4 `util.rs:338` and `:339` return without converting for `Xyb` and `Unknown`. The integer grid is written untouched and the adapter reports `DecodePhotometricInterpretation::Rgb`, so `ColorTransform::resolve` and `with_codec_rgb(true)` pass XYB samples through as RGB.
**Why it is wrong**: PS3.3 C.7.6.3.1.2 gives XYB no matrix, so the only correct routes are conversion by the codec or refusal. PS3.5 8.2.15 says "The Pixel Data characteristics included in the JPEG XL bit stream shall be used to decode", and Table 8.2.15-1 lists XYB with `.110` as well as `.112`. Decision 9 and D-23's "converted exactly once" rest on the codec having produced RGB, which it has not here. CLAUDE.md: a feature that cannot run "never quietly produces a different result".
**Evidence**: `jpegxl_lossless_rgb8.jxl` rebuilt with its one-bit `all_default` colour encoding (bit 34) replaced by `all_default 0, want_icc 0, colour_space Xyb, have_gamma 0, tf sRGB, intent Relative`, frame bytes from offset 5 unchanged. An unmodified rebuild is byte-identical to the fixture, which validates the rewriter. `JxlImage::read` reports `colour_space Xyb`. `.110` and `.112` both return `Ok(())` with Photometric Interpretation `XYB` and `RGB`, reported `Rgb`, output unconverted. The same with `colour_space Unknown`. libjxl 0.12.0 refuses to encode either, so the case is a crafted or non-libjxl stream. It is not reachable from the tracked fixtures.
**Scope and regression**: present on the reviewed tree. Affects every consumer of `.110` and `.112` three-sample output (`open_frame` and the colour stage, `tools/oracle/src/produce.rs`, the folder viewer once it registers the family). Smallest regression: refuse with `UnsupportedPixelFormat` when the enum `colour_space` is not `Rgb` or `Grey` (and `Grey` only with one sample), with the 258-byte header rewrite as a fixture in `unsupported_codestreams_report_unavailable`. A later story that implements a codec-side XYB to RGB conversion can relax it.

### D2, D-21 and D-28 still describe a `.111` decode the tree no longer performs
**Where**: `docs/hld/DEVIATIONS.md:38` (D-21) and `:45` (D-28)
**What**: D-21 says `jxl-oxide` "for `.111`, writes the reconstructed JPEG bitstream into an owned `Vec` before the existing `.50` adapter decodes it". D-28 says the dependency is used "for the three PS3.5 A.4.12 syntaxes `.110`, `.111` and `.112`". After the operator answer, `.111` is `KnownUnavailable` and no path reconstructs, so both sentences are false.
**Why it is wrong**: the deviation register is normative and is the one place a reader goes to learn what departs from the HLD. The plan's final section is authoritative and contradicts both rows. This file is not among the implementer's listed open items.
**Evidence**: `register_jpegxl_decoders` registers `.110` and `.112` only (`jpegxl.rs:104`), `every_decoder_family_registers_into_one_registry_without_collision` asserts `.111` unavailable, and no `reconstruct_jpeg` call exists. `deviation_check.py` passes, because it checks citations and not content.
**Scope and regression**: present on the tree since the design commit `761b434`. Fix the two sentences. While editing D-21, its "library-owned storage is bounded by the frame it describes" is broader than the code: `jxl-color` 0.11.0 `icc/decode.rs` resizes the encoded ICC buffer to `enc_size`, up to `1 << 28`, outside `AllocTracker`, before `validate_header` can refuse the ICC. That part is read from source and not reproduced, so it is a wording correction rather than a counted finding.

### D3, the allocation-limit comment misstates the fixture measurement
**Where**: `crates/ocelli-codec/src/jpegxl.rs:46` to `:48`
**What**: "The tracked fixtures peaked at a ratio of 6 on the 8 by 16 container." Measured: with the numerator at 6, 7 or 8, `lossless_only_refuses_vardct_and_xyb` and `jpegxl_lossy_stays_within_the_recorded_bound` fail with `DecoderFailure`. At 10 and 12 the lossy one still fails. At 13 all pass. The container case passes from 6. So the tracked fixtures peak between 12 and 13, on the 64 by 96 `jpegxl_lossy_12.jxl`, not at 6 on the container.
**Why it is wrong**: a factual claim in a doc comment that is false. It is the stated basis of the margin for D-21's bound. The tracked lossy fixture has the same margin as the 512 by 512 case and not four times that.
**Evidence**: the numerator mutated to 4, 5, 6, 7, 8, 10, 12, 13, 14 in a scratch copy, `cargo test --release -p ocelli-codec --test jpegxl -- --skip malformed`.
**Scope and regression**: comment only, present on the tree. The constant 26 is still twice the worst measured ratio, so the bound itself holds. Correct the sentence. The 512 by 512 measurement could not be re-run, because no such fixture is tracked.

## Smells

### S1, the AC4 range check has no test, and a fixture that exercises it is one bit away
**Where**: `crates/ocelli-codec/src/jpegxl.rs:446` (`if value > max`), `crates/ocelli-codec/tests/jpegxl.rs`
**What**: removing the range check leaves all 12 `jpegxl` tests green. AC26 requires this mutation to be seen red. A one-bit header patch of `jpegxl_lossless_16.jxl` at byte 52 bit 6 (bits per sample 15) or bit 7 (14), or byte 53 bit 0 (12), keeps 96 by 64 grey. Under the matching Bits Stored, the shipped code refuses with `UnsupportedPixelFormat` and the mutant writes 65535 into a 15-bit stored field.
**Mechanism**: the check is the only thing that stops a codestream whose modular samples exceed its declared depth from becoming out-of-range stored values. The first refactor that drops it, for example a move to `write_to_buffer`, will pass the suite.
**Regression**: add the patched bytes (an in-test single-bit XOR of the tracked fixture is enough, as `jpegxl_recompression_is_unavailable` already does) with descriptor Bits Allocated 16, Bits Stored 15 and expected `UnsupportedPixelFormat` with `out` unchanged. PS3.3 C.7.6.1.1 (Bits Stored) and PS3.5 8.2.15 are the citations.

### S2, the scan's "full write" check cannot fail
**Where**: `crates/ocelli-codec/tests/jpegxl.rs:571` to `:576`
**What**: on `Ok(Ok(()))` the scan checks `out.len() != desc.output_len()`. `out` is a `Vec` allocated at that length and lent as `&mut [u8]`, so its length cannot change and the branch never fires. The test comment (`:395`) claims every `Ok` is "with a full write".
**Evidence**: a mutation in `write_rendered` that copies only the first half of the frame on success leaves `malformed_jpegxl_never_panics` green. The intact-fixture tests catch it, and the scan does not.
**Mechanism**: a guard that looks authoritative and is never reached. A later partial-write regression that only a malformed input triggers would pass the scan while its comment says it is covered.
**Regression**: decode each `Ok` input twice into buffers pre-filled with two different sentinels and require identical output, or delete the branch and the "full write" wording.

## Nitpicks

- **N1.** `packages/core/src/snapshot.ts` `PhotometricSnapshot` and `PHOTOMETRIC` omit `"xyb"`, which `crates/ocelli-viewport/src/snapshot.rs` now publishes. It is unreachable today, because `LutChain::new` refuses every colour space, so a snapshot never carries it. The two lists have no test tying them together.
- **N2.** `jpegxl.rs:441` `if sample < 0` duplicates the refusal `u32::try_from` makes on the next line. Its removal is an equivalent mutant.
- **N3.** `corpus_synth.generate()` encodes the five JPEG XL rows twice, once from `generate_syntax_layer` and once from the `jpegxl` entry in `case_callables`. The output is deterministic, so the cost is time only. The docstring implies one call.
- **N4.** `read_header` grows its window one byte per `try_init`, and `jxl-oxide` 0.12.6 `try_init` re-parses the image header and the whole embedded ICC from the start each time (`lib.rs:353`). Header parsing is therefore quadratic in header length. Harmless for the fixtures (largest header under 300 bytes) and for typical ICC sizes. It is a plausible slow-input concern for a large embedded ICC, not measured, because libjxl refused the synthetic large profile this pass tried.
- **N5.** The `.110` VarDCT refusal reads `frame_header(0)`, which is keyframe 0 only. A reference-only, LF or zero-duration layer frame is not checked. Plausible, not constructed. It belongs beside "lossy modular is not detected" in `docs/lld/codecs.md`.
- **N6.** `JPEGXL_XYB_MAX_ABS = 21` in `scripts/tests/test_corpus_synth.py` is the measured maximum itself, where AC10's own rule for the Rust bound is 1.5 times the measurement. It is deterministic under the pinned encoder.

## Verified clean

- No `as` cast, no `#[allow]` and no `unsafe` in `jpegxl.rs`. Every conversion is `u32::from`, `usize::from`, `try_from` or checked arithmetic.
- The 12-bit path reads `I16` grids sample by sample, never `write_to_buffer`. Stored values are exact for 16-bit, 12-bit and RGB, with interleaved `R, G, B` order.
- Signed Pixel Representation, Bits Allocated 1 and 32, OW, float samples, alpha, ICC, PQ, orientation 6, two keyframes, size mismatch, sample-count mismatch and Bits Stored mismatch all refuse before `out` is written. Atomic copy after full validation.
- `.111`: `register_jpegxl_decoders` preflights all three UIDs, registers two, leaves `.111` `KnownUnavailable`, and `Registry::decode` refuses before the adapter is reached. No Ocelli path or dependency path outside `reconstruct_jpeg` reaches `jxl-jbr` reconstruction. A `.112` container with a `jbrd` box reaches only the parse half (`aux_box/jbrd.rs`), and that is in the scan.
- Registry counts executed: 19 known, 17 available, 2 unavailable (Deflated and `.111`), 6 `register_*` functions, 7 names, 19 distinct syntaxes in `corpus/manifest.tsv`, 100 manifest rows, `REGISTRY_TRANSFER_SYNTAXES` 19. CLAUDE.md's changed sentences match. `crates/ocelli-dicom/tests/dicomweb.rs:453` asserts 19 and equality with the corpus set, and it passes.
- Licences: gate green, `licenses ok` under cargo-deny, `THIRD_PARTY_LICENSES` and `THIRD_PARTY_NOTICES` unchanged and regenerated equal. D-28's "Neither `ocelli-wasm` nor `ocelli-native` reaches `ocelli-codec`" holds. No Rayon on wasm32. Pins `=0.12.6` and `=0.12.4` with defaults off.
- Corpus: five rows added, generated from the uncompressed bases, decode digests for `.110` equal to the existing reference digests (`9cb18079...` for mono16), `.111` has no digest and the gate refuses one. Census counts 43 to 46 and 6 to 9 are each covered by a test that a mutation turns red. `--check-runbook` green.
- `Xyb` variant: present in every exhaustive match (`ocelli-pixel`, `ocelli-dicom`, `ocelli-render`, `ocelli-viewport`, folder viewer), refused by stage 3, refused by stage 4 under `Preserved`, passed through under `Rgb`.

## Limitations and handoff

- **Not exercised**: the oracle, any GPU or browser job, a wasm32 execution of a JPEG XL decode (AC21 is argued from the integer path, as the plan says), the 512 by 512 allocation measurement, and a large-ICC stream (N4).
- **Carried open items, judged against the plan's split** (plan "Dependencies, write ownership and resources", "the library half (steps 1 to 6 and the oracle files)"):
  - `tools/oracle/unsupported.json` (AC24) has evidence stage "integrated, needs the oracle host" and needs a measured error text, so it cannot be written correctly without the run. It still blocks completion, and `produce.rs` plus five reference-undecodable rows will make `gate oracle` red until it lands. That consequence is inferred from `unsupported.json`'s stated rule and was not executed.
  - `docs/SOURCE-POLICY.md` rows and the AC25 unsafe audit are plan step 9. The policy says to record the answer "before taking anything from a source not in the table", and the dependency is already in the lock. Not counted as a defect of this half, but it is the first thing completion must do.
  - `docs/lld/codecs.md:304` still says "sixteen-UID", and the other LLDs, the `dicom-expert` skill, `README.md`, `use.html` (lines 546 and 583 say sixteen, five and fifteen) and `more.html` are step 9 or serialised behind the viewer stories. They are stale on this tree and block completion. `examples/folder-viewer/src/preview.rs` does not register the family yet, which is the viewer half's.
  - AC20 is the integrated stage. The tree already shows `THIRD_PARTY_LICENSES` unchanged with F-220 in the base.
- **External**: `cargo deny check` advisories fail on `encoding` through dicom-rs, unrelated to F-229.
- **Scratch**: probes and mutations ran in a copy under the session scratchpad with `CARGO_TARGET_DIR=<wt>/target/f229-scratch`. Both were deleted. The reviewed tree hash and index tree were unchanged at the end. This report is the only file written in the worktree.
- **Next**: the implementer remediates D1 to D3, S1 and S2, then a fresh independent pass 2 reviews the changed material. This pass is a bounded library-half review, not acceptance of F-229.
