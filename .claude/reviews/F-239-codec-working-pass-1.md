# F-239 review, pass 1, codec part

**Reviewed**: `work/f-239-codec`, range `05d9aca2..238ee5bb` (six commits, 37 files), in the worktree `ocelli-wt-f-239-codec`. No unstaged tracked changes at review time, only the untracked `.venv` and `tools/oracle/out`.
**Author / reviewer**: implemented by GLM in opencode with its own three-cycle self-review (`.claude/scratch/F-239-self-review-1.md` to `-3.md`, ignored files). Reviewed by Claude as an independent `/microscope --working` pass. The reviewer wrote no part of this diff. The reviewer did not read the AC17 standards at dicom.nema.org or the ISO text in this pass, and derived the Annex G arithmetic from the plan's transcription and the vendored crates' own constants.
**Result**: 7 defects, 4 smells, 5 nitpicks

## Acceptance challenged

- **AC24, inverse RCT.** Derived independently at B = 8. Pure red `255 0 0` shifts to `127 -128 -128`, forward `Y0 = floor(-257 / 4) = -65`, `Y1 = 0`, `Y2 = 255`, inverse `I1 = -65 - floor(255 / 4) = -128`, `I0 = 127`, `I2 = -128`, back to `255 0 0`. With the clamp first, `Y2` becomes 127 and the inverse gives `I1 = -96`, so `159 32 32`, as D-31 says. `200 100 50` round trips as the plan works it. At 12 bits pure red is `Y0 = floor(-4097 / 4) = -1025`, `Y2 = 4095`, and at 16 bits `Y0 = -16385`, `Y2 = 65535`. Both, and signed 8-bit `127 -128 -128`, were pushed through the vendored openjph encoder as already-transformed components and decoded exactly by the patched HTJ2K path (`review_depth.rs` in the scratch copy). The openjph component order matches G.2: `gen_rct_backward(y, cb, cr)` computes `g = y - ((cb + cr) >> 2)`, `r = cr + g`, `b = cb + g`, with component 1 `Y1 = B - G` and component 2 `Y2 = R - G`. The ritk `inverse_rct_5_3` is the same three lines with an arithmetic shift, which floors.
- **AC25, inverse ICT.** `Y0 = -28, Y1 = 22, Y2 = -38` give `-81.276`, `-8.43354`, `10.984`, plus 128 is `46.724 119.56646 138.984`, rounded `47 120 139`. openjph's constants are `GAMMA_CR2R = 1.402`, `GAMMA_CB2B = 1.772`, `GAMMA_CR2G = 0.714136`, `GAMMA_CB2G = 0.344136`, derived from the forward matrix, ritk's are Annex G's literal `0.34413` and `0.71414`. The order in both patches is inverse wavelet, inverse ICT on unrounded `f32`, one rounding, then the DC shift and clamp, which is Annex G's order.
- **AC10, report and refusal matrix.** Challenged with a discriminating case the tests do not hold: the lossless RCT fixtures under a `YBR_FULL` and an `RGB` descriptor. See D1.
- **AC23 and the codec half of AC5.** `decoded_output_len` is 4 for native `YBR_FULL_422` 2 by 1, 6 for native RGB and 6 for JPEG, and `Registry::decode` refuses a 6-byte buffer with `OutputLength { expected: 4, actual: 6 }`. Challenged with Samples per Pixel other than 3 under a 4:2:2 photometric. See D7.
- **Unchanged consumers of changed providers.** `cargo test -p ocelli-folder-viewer` and `cargo check -p ocelli-oracle` pass on the tip, so the default-method addition and HTJ2K's monochrome layout report moving from `Preserved` to `Interleaved` break no current caller. `preview.rs:1373` reads only the photometric report.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-codec -p ocelli-dicom` | worktree tip 238ee5bb | now | 0. codec 104 tests, dicom suites green, corpus 6 ignored |
| `cargo test -p ocelli-dicom --test corpus -- --ignored` | worktree, `corpus/data` symlinked to the main checkout's corpus | now | 0, 6 passed. The seven new rows' SHA-256 match the manifest |
| openjph-core `cargo test --lib`, ritk-codecs `cargo test --lib` | vendored crates at the tip | now | 151 passed, 320 passed |
| `python3 scripts/pin_and_size_check.py --with-size` | worktree | now | 0. wasm 2,179,477 against baseline 2,151,196, ceiling 2,258,755. decode 16,455 |
| `guard_probe.py --only` the three new pins probes | worktree | now | each OK, red for its declared reason |
| `python3 -m unittest scripts.tests.test_pin_and_size_check` | scratch | now | OK |
| `.venv/bin/python -m unittest scripts.tests.test_corpus_synth` | scratch, `opj_compress` on PATH | now | 60 tests OK |
| same, PATH without `opj_compress` | scratch, shim PATH holding DCMTK and `ojph_compress` | now | 0 tests ran, `RuntimeError: opj_compress is not on PATH`. `corpus_tests.py --require-prerequisites` prints `FAIL: corpus tooling tests` |
| `generate_colour.py --write` then `cmp` | scratch, main checkout's corpus | now | all seven fixtures byte-identical to the tracked ones |
| `review_probe.rs`: RCT fixtures under `YBR_FULL` and `RGB` | scratch | now | `.90` and `.201` both decode `Ok`, report `Preserved`, bytes equal the RGB reference |
| `review_measure.rs`: four colour fixtures | scratch | now | `.91` 43 samples differ from OpenJPEG, max `[1, 1, 1]`. `.203` 0. Inverse left out: `.90` 18,353, `.201` 18,353 vs reference, `.91` 18,345 vs OpenJPEG and 18,342 vs reference, `.203` 18,353 vs OpenJPEG and 18,352 vs reference |
| `review_spp.rs`: native `YBR_FULL_422` 2 by 1 at SPP 1, 2, 3 | scratch | now | decoded length 4 at every SPP, container 2, 4, 6, every decode `Ok` |
| mutation catalogue, 11 cases | scratch, own `CARGO_TARGET_DIR` | now | 10 red, 1 green (J), listed under Verified clean and S1 |
| pins guard with the openjph patched-hash loop disabled | scratch | now | a `tile.rs` drift passes, so the loop is the guard |
| `cargo tree -p ocelli-wasm` both feature sets, wasm32 | worktree | now | 0 matches for `ocelli-codec`, `ocelli-dicom`, `openjph`, `ritk` |
| `python3 scripts/unsafe_allowlist_check.py` | worktree | now | OK, openjph 104, ritk 0, unchanged |
| GLM's self-review receipts | `.claude/scratch/F-239-self-review-*.md` | prior | read, not relied on |

## Defects

### D1, a YBR_FULL or RGB descriptor over a codestream with MCT 1 decodes to RGB and reports Preserved
**Where**: `crates/ocelli-codec/src/jpeg2000.rs:432` (`validate_colour_shape`) and `:498` (`decoded_photometric`), shared by `htj2k.rs`.
**What**: `validate_colour_shape` refuses only the contradiction "MCT 0 under YBR_RCT or YBR_ICT". Its converse, MCT 1 under `YBR_FULL` or `RGB`, passes. Both vendored decoders then apply the codestream's inverse transform and hand back RGB, while `decoded_photometric` keys on the descriptor alone and reports `Preserved`. `ColorTransform::resolve` then keeps the data set's `YBR_FULL` and applies the full-range matrix to samples that are already RGB. Pure red `255 0 0` read as `Y 255, Cb 0, Cr 0` gives, by the library's inverse at `color.rs`, `R = 255 + 1.401987577 * -128 = 75.55`, `G = 390.45`, `B = 28.20`, so `76 255 28` on the tier C byte rule. A red pixel drawn green, with no error.
**Why it is wrong**: PS3.5 8.2.4, quoted in GLM's own progress note: when the Part 1 multi-component transform has been applied, Photometric Interpretation shall be `YBR_RCT` or `YBR_ICT`. The plan's item 16 and AC10 say a three-component codestream with the transform on reports `Rgb`. The report cannot see the codestream, so the decode must refuse the contradiction instead.
**Evidence**: `review_probe.rs` in the scratch copy. `.90 YBR_FULL: report=Preserved decode=Ok(()) equals_rgb_reference=true`, and the same for `RGB` and for `.201`. The `76 255 28` figure is computed, not executed through a viewer, because the folder viewer is not wired yet.
**Scope and regression**: every consumer of `decode_photometric_interpretation` for `.90`, `.91`, `.201` to `.203`, today the oracle's `produce_current.rs:470` and after the viewer part the folder viewer. `RGB` with MCT 1 gives the right pixels and is only non-conformant, so refusing it is a choice. `YBR_FULL` with MCT 1 is the quietly wrong pixel. Smallest regression: the existing COD-byte edit in `a_three_component_codestream_is_reported_rgb_and_refusals_are_kept` under a `YBR_FULL` descriptor must answer `FrameMismatch`, in both adapters.

### D2, the CI corpus-tooling job now fails because `opj_compress` is required and nothing installs it
**Where**: `scripts/corpus_synth.py:1302` (`require_encoders`), `scripts/corpus_tests.py:72` (`REQUIRED_TOOLS`), `.github/workflows/ci.yml:269`, `corpus/README.md:104` to `:105`.
**What**: `require_encoders` now demands `opj_compress`. `test_corpus_synth.py`'s `setUpModule` calls `corpus_synth.generate`, which calls it. The CI job installs DCMTK and builds OpenJPH only. `corpus_tests.py`'s prerequisite list does not name `opj_compress`, so a missing CLI is not reported as a prerequisite either. The error message sends the reader to `corpus/README.md`, whose prerequisites table does not mention the OpenJPEG CLI.
**Why it is wrong**: the floor CI job runs `uv run scripts/corpus_tests.py --require-prerequisites`, and a new host tool with no install step turns it red.
**Evidence**: with a PATH holding `dcmcjpeg`, `ojph_compress` and the DCMTK companions but not `opj_compress`, `unittest scripts.tests.test_corpus_synth` reports `Ran 0 tests ... FAILED (errors=1)` with the `RuntimeError`, and `corpus_tests.py --require-prerequisites` prints `test_corpus_synth.py: zero tests ran, which is not a pass.` and `FAIL: corpus tooling tests`. Not run on a CI runner.
**Scope and regression**: CI on every push once this lands. Smallest repair: an install step and a `REQUIRED_TOOLS` row for the OpenJPEG CLI, plus the README row. Also `generate_one("colour_syntax")` does not call `require_encoders`, so the selector the LLD names fails with a bare `FileNotFoundError` rather than the named refusal.

### D3, `docs/lld/pixel-pipeline.md:397` now states the opposite of the code
**Where**: `docs/lld/pixel-pipeline.md:397`.
**What**: "`crates/ocelli-codec/src/jpeg2000.rs` refuses `mct != 0`, so neither can reach this stage still in that space." After this diff both adapters accept MCT 1 and report `Rgb` for the two codes.
**Why it is wrong**: a false claim in prose. It is also the sentence a reader would use to argue D1 cannot happen.
**Evidence**: `grep -n 'mct != 0' docs/lld/pixel-pipeline.md`, against `validate_colour_shape` at `jpeg2000.rs:432`.
**Scope and regression**: `pixel-pipeline.md` is in the viewer part's write set, but the codec part makes it false and integrates first, so the tree carries the false sentence between the two integrations. Owner to agree with the lead.

### D4, `docs/lld/corpus.md:118` says five rows over a six-row table
**Where**: `docs/lld/corpus.md:118` to `:129`.
**What**: "Five rows, added through the `colour_syntax` and `sc_rgb_icc` selectors". The table beneath lists six, and the manifest gains six (`reference_rgb8_saturated.dcm` is the sixth). GLM's cycle 3 note itself counts "the manifest's six new rows".
**Evidence**: `git diff 05d9aca2..238ee5bb -- corpus/manifest.tsv | grep -c '^+[a-z]'` gives 6.
**Scope and regression**: prose only. Delete the count or make it six.

### D5, recorded measurements in the LLDs and a test comment do not match the rows
**Where**: `docs/lld/codecs.md:118`, `docs/lld/corpus.md:148`, `crates/ocelli-dicom/tests/corpus.rs:371`.
**What**: both LLDs say a decode with the inverse left out differs at 18,353 of 18,432 samples "on the same rows", where the rows are the two lossy ones. Measured: `.203` 18,353 against OpenJPEG, `.91` 18,345 against OpenJPEG and 18,342 against the reference. The 18,353 figure belongs to `.90`, `.201` and `.203`, not `.91`, so GLM's cycle 1 table mislabelled the rows. The `.203` corpus test comment says "the recorded maximum is one code per channel", which is the `.91` figure. The `.203` row agrees with OpenJPEG byte for byte, as the codec test asserts with `[0, 0, 0]`.
**Evidence**: `review_measure.rs` with the ritk `cod.mct == 1` argument and the openjph transform block both disabled, in the scratch copy.
**Scope and regression**: prose only. Prefer deleting the 18,353 figure to re-explaining it. Both LLD sentences also cite "the F-239 review", which exists only in ignored scratch files.

### D6, the openjph vendor check's docstring still says there is no patch
**Where**: `scripts/pin_and_size_check.py:531` to `:533`.
**What**: "Deviation D-22. There is no patch, so unlike the ritk vendor every published file must match the archive byte for byte and the only addition is the inventory this repository writes." The function now exempts two patched files, checks their D-31 hashes and permits `PATCH-PROVENANCE.md` and `LICENSE`.
**Why it is wrong**: a false claim about what a guard does, beside the code that contradicts it.
**Scope and regression**: prose only.

### D7, the native 4:2:2 length rule is a second copy without the first copy's guard, and its comment states a false invariant
**Where**: `crates/ocelli-codec/src/native.rs:299` and `:355`.
**What**: `stored_samples_per_pixel` returns 2 for either 4:2:2 code whatever Samples per Pixel says. `ocelli_pixel`'s copy is reached only through `SampleLayout::new`, which refuses a Samples per Pixel other than the photometric's expected 3 (`stored_pixel.rs:89`). So the codec decodes a 4:2:2 header at Samples per Pixel 1 or 2 without complaint. At SPP 1 the decoded length, 4, exceeds the container length, 2. The comment at `:299` says "the stored length is below the container length", which is false at SPP 1, and the `unwrap_or(desc.output_len())` fallback is argued from it.
**Why it is wrong**: plan item 15 asks for the length "from `StoredPixelDescription::stored_samples_per_pixel` rather than a second rule". The local copy is defensible because `ocelli-codec` does not depend on `ocelli-pixel`, but the two copies diverge on exactly this input. PS3.3 C.7.6.3.1.2 defines the two codes with three samples.
**Evidence**: `review_spp.rs`: `spp 1: container 2 decoded_output_len 4 decode Ok(())`, `spp 2: container 4 decoded_output_len 4 decode Ok(())`.
**Scope and regression**: no wrong pixel was shown, because `ocelli-pixel` refuses the layout downstream. Smallest repair: answer 2 only when Samples per Pixel is 3, or refuse the contradiction as `FrameMismatch`, with an SPP 1 case in `decoded_output_len_is_the_decoders`.

## Smells

### S1, AC25's coefficient evidence cannot reject the wrong implementation the plan names
**Where**: `crates/ocelli-codec/tests/jpeg2000.rs`, `inverse_ict_uses_annex_g_coefficients`. `vendor/ritk-codecs-0.6.0/src/jpeg_2000/packet/reader.rs:464`.
**What**: the plan's AC25 rejects "the YBR_FULL inverse of `color.rs` in place of Annex G's". Mutation J replaced ritk's `1.402` with `color.rs`'s `1.4019876` and every test stayed green. That is expected, because the two inverses differ by about 0.02 at most. But the test's name and comment say the coefficients are pinned, and the ≤ 1 code tolerance against OpenJPEG cannot pin anything finer than about one percent of a coefficient. The plan's hand values `47 120 139` are asserted nowhere, and ritk's new `inverse_ict_9_7` has no unit test of its own.
**Hazard**: a later edit to a coefficient within that band passes as covered forever. Repair: a `#[cfg(test)]` case inside the ritk patch asserting the `f32` outputs `-81.276`, `-8.43354`, `10.984` to a stated tolerance, or a reworded claim that says what the tolerance does hold.

### S2, nothing ties the tracked fixtures to the corpus rows, and the `.91` row is never decoded by Ocelli's corpus test
**Where**: `crates/ocelli-codec/tests/fixtures/generate_colour.py:20`, `crates/ocelli-dicom/tests/corpus.rs`.
**What**: the generator's docstring says running it without arguments checks the tracked bytes. It checks only that the files exist and prints their digests, which GLM's cycle 1 recorded as evidence. `corpus.rs` decodes `.90`, `.201` and `.203` but never `syntax/j2k_lossy_ybr_ict.dcm`, and it compares the two lossless rows with each other and four marker pixels rather than with `syntax/reference_rgb8_saturated.dcm`, which is in the corpus. The codec tests compare against extracted copies.
**Evidence**: regenerating the fixtures in the scratch copy reproduced all seven byte for byte, so nothing has drifted today. The guard simply does not exist.
**Hazard**: a regenerated corpus row and a stale fixture diverge silently, and AC25's "through the corpus tooling" is half met.

### S3, the `use.html` decode example sizes the buffer the next sentence says can be refused
**Where**: `examples/folder-viewer/src/use.html:567` and `:569`.
**What**: the example keeps `vec![0_u8; desc.output_len()]`, and the edited sentence below it says decode now checks `decoded_output_len`, which is shorter for a native 4:2:2 frame. A reader copying the example gets `OutputLength` on such a frame. Repair: size from `registry.decoded_output_len(uid, &desc)?` in the example.

### S4, the patch newly reaches an unsafe block on the production decode path, and the record says only that it adds none
**Where**: `vendor/openjph-core-0.1.0/src/codestream/tile.rs:786`, reaching `src/transform/colour.rs:418` to `:436`.
**What**: before this patch `gen_rct_backward`'s raw-pointer loop had no decode-path caller, only tests and a dispatch table nothing reads. Now every colour HTJ2K RCT decode runs it. Its soundness rests on the new caller's `repeat = width`, the minimum of three row lengths, never exceeding any of the six buffers. That holds today. `PATCH-PROVENANCE.md` and the unsafe audit say the patch adds no `unsafe`, which is true by count, and neither names the newly reached site.
**Hazard**: a later edit to `rct_backward`'s width computation reads out of bounds through `unsafe` that the audit record says this patch does not touch. Repair: one sentence in `PATCH-PROVENANCE.md` naming the reached block and the invariant the caller holds.

## Nitpicks

- N1. Prose semicolons in added comments and strings, outside `prose_check.py`'s scope: the comment in `corpus.rs` after the `Y2 = 72 - -28 = 100` line, `tile_comp.rs`'s `perform_idwt` doc after "the shift is undone", `pin_and_size_check.py:160`, two `catalogue.py` probe notes, two `corpus_synth.py` comments, and the new ritk "three components" message.
- N2. `COLOUR_ROW_PHOTOOMETRIC` is misspelt. `generate_syntax_layer` writes the saturated reference, binds `saturated_reference` and never uses it, then `generate_colour_syntax_layer` writes the reference again. `tool_versions` runs `opj_compress -h` twice.
- N3. openjph's ICT green coefficients are `0.344136` and `0.714136`, derived from the forward matrix, and ritk's are Annex G's rounded `0.34413` and `0.71414`. That is harmless at 8 and 16 bits. Worth one line in codecs.md, since the LLD calls both "Annex G".
- N4. ritk's provenance says `decode_tile_part`'s "own behaviour is unchanged", but its truncation error text now carries ", component 0". No consumer reads the text, because the adapter maps it to `DecoderFailure`.
- N5. Three test comments and two LLD sentences cite "the F-239 review" for a measurement that exists only in ignored `.claude/scratch` files.

## Verified clean

- **Order of operations, both patches.** openjph: `perform_idwt(defer)` skips `undo_level_shift` and, for 9/7, keeps unrounded `decoded_lines_f32`. The tile applies `gen_rct_backward` on integers or `gen_ict_backward` on floats, then `finish_deferred_level_shift` rounds and shifts with the original clamp. ritk: per-component inverse wavelet, the transform on `mallats` (5/3) or `float_planes` (9/7), one rounding, and `restore_clamped_colour_sample` in the caller. Monochrome paths are byte-for-byte the old arithmetic (the 151 and 320 crate suites and every existing codec and corpus test pass).
- **Mutations, red.** A, openjph defer off, which is AC18's "inverse after the level shift": 3 HTJ2K tests red. B, openjph RCT and ICT choice swapped: 3 red. C, openjph ICT Cb and Cr swapped: red. D, ritk ICT green coefficients swapped: red. E, ritk RCT floor replaced by truncation: red. F, ritk colour clamp removed: red. G, ritk inverse left out: 2 red. H, ritk component order reversed: 2 red. I, HTJ2K pull order reversed: 8 or more red. K, native odd-Columns refusal removed: red. GLM's cycle 1 table covers `Registry::decode` sizing, the 4:2:2 count, depth, sampling, MCT 0 and lossless-mode refusals, and they were not repeated.
- **Mutation, green.** J, see S1.
- **Casts.** `width as u32` in `tile.rs` is bounded by a SIZ `u32` dimension. `v.round() as i32` saturates, as the published code already does. `raw as f64` and `rescaled as f32` are exact for 16-bit samples at slope 1, intercept 0. The u64 product in `native.rs` cannot overflow. Test-only `u8 as i32` is exact. ritk's `(cr + cb) >> 2` could overflow only above about 30-bit precision, which the adapter's 8 and 16 bit containers exclude.
- **Patches minimal and recorded.** openjph: two files, the deferral and two transform calls, and two helpers made `pub(crate)`. ritk: four files, mostly the re-indentation of a per-component loop. All six SHA-256 match `PATCH-PROVENANCE.md` and `pin_and_size_check.py`. Drift in a patched file, an unrecorded byte in an unpatched file, a deleted patched file and an edited provenance record are each refused. Disabling the openjph patched-hash loop lets drift through, so the loop is the guard, and the catalogue probe watches it. The `unsafe` token count is unchanged in all six patched files and the allow-list check reports the audit surface unchanged.
- **Fixtures and corpus.** The reference `.raw` is the saturated reference row's own Pixel Data from `numpy` in `corpus_synth.py`. The lossy expectations are OpenJPEG's decode through pylibjpeg. The HTJ2K one-pixel fixtures push hand-computed G.2 forward values through an encoder that applies no forward transform. No expected value was read back from Ocelli's own decoder. `decode-digests.tsv`'s new rows are pylibjpeg decodes, independent of Ocelli. No `.dcm` file and no file with `DICM` at offset 128 is in the diff.
- **AC29 reachability.** `cargo tree` reaches none of the codec crates from either wasm module. The size delta against the F-050 baseline predates this diff, per GLM's base build. That base build was not repeated.
- **Prose executed and found true.** D-31's `159 32 32`. Both PATCH-PROVENANCE files' test counts, hashes and "encoder applies no forward transform" (the hand-value test depends on it and passes). codecs.md's 43 of 18,432 and `[0, 0, 0]`, the refusal list, the 4:2:2 paragraph and the sample-convert sentence. `use.html:569`'s sentence itself.

## Limitations and handoff

- Not exercised: the AC17 reading of PS3.5 8.2.4, A.4.4 and ISO/IEC 15444-1 Annex G at source. 16-bit colour through ritk, which has no three-component encoder in the tree and no 16-bit colour corpus row. COC or QCC colour codestreams, which ritk refuses by marker. Any GPU path, which none of this touches. A real CI run for D2.
- AC29's recording in `docs/lld/build-targets.md` is outside the codec write set and is still owed.
- The viewer part consumes `decode_photometric_interpretation`, so D1 should be closed before the viewer part integrates.
- Next owner: implementation remediates D1 to D7 and S1 to S4, then a fresh independent pass 2 reviews the changed material.

## Self-review catch figure

GLM's three self-review notes should have caught 10 of the 11 blocking findings: D1 to D7 and S1 to S3. D1: cycle 3 quotes the very PS3.5 note that forbids it. D2: cycle 1 added the tool to `require_encoders`. D4: cycle 3 counts six rows. D5: cycle 1 measured the numbers and mislabelled them. S2: cycle 1 cites the presence-only check as evidence. S4 is a judgement about the audit record that the notes could reasonably miss. Of the nitpicks, N1 was claimed fixed in cycle 1.
