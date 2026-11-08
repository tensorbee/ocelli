# F-239 review, pass 2, codec part

**Reviewed**: `work/f-239-codec`, range `b8ec232b..b5060f93` (three commits, 24 files), in the worktree `ocelli-wt-f-239-codec`, tip `b5060f9364d6a8a5cd91d876e293004ecaa222f8`. No unstaged tracked changes at review time, only the untracked `.venv` symlink and `tools/oracle/out`.
**Author / reviewer**: remediation by GLM in opencode with its own self-review cycles 4 to 7 (`.claude/scratch/F-239-self-review-4.md` to `-7.md`, ignored files). Reviewed by Claude as an independent `/microscope --working` pass 2. The reviewer wrote no part of this range. The reviewer read the plan's AC25 transcription of ISO/IEC 15444-1 G.3 and did not read the ISO text or PS3.5 at source in this pass.
**Result**: 4 defects, 0 smells, 6 nitpicks

## Acceptance challenged

- **Pass 1 D1, both paths.** Deleting the new MCT 1 arm in `validate_colour_shape` turns `a_transformed_codestream_refuses_a_descriptor_that_names_no_transform` red in both `tests/htj2k.rs` and `tests/jpeg2000.rs`, `Ok(())` against `FrameMismatch`. The refusal sits before any write, and both tests assert the 0xa5 sentinel survives. MCT values above 1 stay refused at `jpeg2000.rs:437`, so the `_` arm is unreachable. The report path still answers `Preserved` for RGB because it cannot see the codestream, and the decode refuses, which is the repair pass 1 asked for.
- **Pass 1 D7.** Removing the `validate_stored_samples` call, or the `== 3` guard in `stored_samples_per_pixel`, each turns `decoded_output_len_is_the_decoders` red at `registry.rs:519`. Both 4:2:2 codes at Samples per Pixel 1 and 2 are refused through the public `Registry::decode` with the header's own buffer length.
- **AC25 coefficient pins, independently derived.** At `Y0 = -28, Y1 = 22, Y2 = -38`, G.3's literal coefficients give `-81.276 -8.43354 10.984`. openjph's forward-derived `0.344136` and `0.714136` give green `-8.433824`, which differs from the plan's value by 2.84e-4. The ritk unit test goes red for `1.402` to `1.4019876` and for the green coefficients swapped. The openjph unit test goes red for `GAMMA_CR2R` to `1.4019876`, and also goes red when its green coefficients are replaced by G.3's literal `0.34413` and `0.71414`. See D1.
- **The OpenJPEG comparison as a coefficient check.** `1.402` to `1.4019876` in ritk, and `GAMMA_CR2R` to `1.4019876` in openjph, each leave every codec test and all seven ignored corpus tests green. See D2.
- **Inverse left out, re-measured now.** `.91` differs from OpenJPEG at 18,345 of 18,432 samples, max `[179, 255, 255]`, and from the reference at 18,342. `.203` differs from OpenJPEG at 18,353, max `[179, 255, 255]`, and from the reference at 18,352. Normal decode: `.91` 43 against OpenJPEG, max `[1, 1, 1]`, `.203` 0. The LLD counts and "up to 255 codes per channel" hold.
- **The `.91` corpus markers.** OpenJPEG's own decode of `.91` (the tracked fixture) starts `255 0 0, 0 255 0, 0 0 255, 199 100 50`, and the reference starts with `200 100 50` at pixel 3, so the asserted 199 is the independent decoder's value and not one read back from Ocelli.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-codec -p ocelli-dicom` | worktree tip b5060f93 | now | 0, 327 passed, no FAILED line |
| `cargo test -p ocelli-dicom --test corpus -- --ignored` | worktree, `corpus/data` symlinked to the main checkout | now | 0, 7 passed |
| openjph-core and ritk-codecs `cargo test --lib` | scratch copy of the tip, own `CARGO_TARGET_DIR` (both crates are workspace-excluded, so running them in the worktree would write a `target` into `vendor/`) | now | 154 passed, 321 passed |
| `python3 scripts/corpus_tests.py` | worktree, `.venv` interpreter | now | 0, 19 and 60 passed |
| `python3 scripts/pin_and_size_check.py --with-size` | worktree, pkg built 16:54 after the tip commit | now | 0. wasm 2,179,477, ceiling 2,258,755. decode 16,455 |
| pins gate after a one-byte append to `reader.rs`, `tile.rs`, openjph `PATCH-PROVENANCE.md` | scratch | now | each FAIL naming the file, back to OK after restore |
| `guard_probe.py --only` the ritk and openjph patched-source drift, openjph provenance, ritk provenance-extra and openjph inventory probes | scratch | now | each OK, red for its declared reason |
| `python3 -m unittest scripts.tests.test_pin_and_size_check` | scratch | now | OK |
| `corpus_tests.py --require-prerequisites` with a PATH shim lacking only `opj_compress` | scratch | now | test_corpus_synth SKIPPED naming `opj_compress (OpenJPEG, for the two JPEG 2000 colour syntaxes)`, then FAIL |
| `corpus_synth.py --case colour_syntax` without `opj_compress` | scratch | now | `RuntimeError: opj_compress is not on PATH` naming the OpenJPEG CLI and the README |
| `corpus_synth.py --case colour_syntax` with the full PATH | scratch, opj_compress 2.5.4 | now | all five written rows' SHA-256 found in `corpus/manifest.tsv` |
| `generate_colour.py` check mode, clean, one flipped byte, one missing fixture, corpus hidden | scratch | now | exit 0 "matches the corpus byte for byte", then exit 1 naming the drifted file, exit 1 naming the missing file, exit 1 naming the absent corpus path |
| mutation catalogue, 13 cases | scratch, own target dir, every restore followed by an mtime bump | now | 10 red, 3 green, the green ones named in D1 and D2 |
| `p2_measure.rs`, the four lossy measurements with and without the inverse ICT | scratch | now | figures above |
| `unsafe_allowlist_check.py`, `prose_check.py`, `deviation_check.py` | worktree | now | OK, openjph 104 and ritk 0 unchanged. Voice clean. 31 deviations |
| SHA-256 of all six patched files and both provenance records | worktree | now | each equals `PATCH-PROVENANCE.md` and `pin_and_size_check.py` |
| `git status --porcelain --ignored`, `git reflog --date=iso`, `git stash list`, `git fsck` | worktree | now | see Verified clean |
| GLM's self-review receipts | `.claude/scratch/F-239-self-review-4.md` to `-7.md` | prior | read, not relied on |

A first mutation run in this pass restored files with `shutil.copy2`, which kept the pre-mutation mtime, so cargo kept the mutated builds and later cases in that run were contaminated. Every result in this report comes from the second run, which bumps the mtime after each restore and re-baselined green (327, 154, 321, 7) before and after.

## Defects

### D1, the openjph coefficient test pins openjph's own derived green value and the record says it pins the plan's AC25 hand values
**Where**: `vendor/openjph-core-0.1.0/src/codestream/tile.rs:1118` to `:1150` (`ict_backward_is_the_annex_g_transform`), `vendor/openjph-core-0.1.0/PATCH-PROVENANCE.md:43` to `:46`.
**What**: the provenance says the third test "pins `gen_ict_backward` to the F-239 plan's AC25 hand values at a 1e-4 tolerance". The plan's AC25 green value is `-8.43354`. The test asserts `-8.433824`, the output of the crate's own `GAMMA_CB2G` and `GAMMA_CR2G`, and its failure message calls that "the Annex G value". `|-8.433824 - -8.43354| = 2.84e-4`, outside the tolerance, so the plan's hand value would fail this test. The doc comment also says "neither can an edit to any of the four constants", and an edit of 2e-6 to `GAMMA_CR2R` passes.
**Why it is wrong**: HLD 27.2 R2 and the project rule that a fixture derives from the specification, not from the implementation. The expected green value was computed from the code under test. A false claim in a provenance record that the pins gate hashes as reviewed.
**Evidence**: mutation `S1-ojph-annexg-green`, openjph's green line changed to G.3's literal `0.71414` and `0.34413`, turns the test red. Mutation `S1-ojph-small-edit`, `GAMMA_CR2R + 0.000002`, leaves it green. GLM's cycle 6 note records the first observation itself ("red at -8.433678 against -8.433824").
**Scope and regression**: present on the tip. No pixel is wrong today, because openjph's derived constants are a closer form of the same matrix and no corpus byte moves. The hazard is the record: a reader is told the plan's numbers are pinned when the crate's are. Smallest repair, stated as a rule rather than a number: either assert the plan's G.3 values with a tolerance the implementer measures to admit openjph's derived constants and reject `color.rs`'s `1.4019876`, or say in the test, its message and the provenance that it pins openjph's forward-derived coefficients and how far they sit from G.3's rounded literals. Delete the "any of the four constants" clause either way. Pass 1's N3 is the same fact and was not taken up.

### D2, six sentences still say the OpenJPEG comparison holds or pins the coefficients, which this range's own mutation shows it does not
**Where**: `docs/lld/codecs.md:117` to `:120` ("which is the comparison that holds the coefficients"), `crates/ocelli-codec/tests/htj2k.rs:331` to `:335` ("the coefficients are pinned where they can be: against OpenJPEG", then "which the F-239 review recorded") and `:351` ("so the coefficient check above is the pinned form"), `crates/ocelli-codec/tests/jpeg2000.rs:467` to `:468` ("the coefficients are pinned where they can be: against OpenJPEG"), `crates/ocelli-dicom/tests/corpus.rs:384` to `:386` (the `.91` test, "the coefficient-level check is the codec test `inverse_ict_uses_annex_g_coefficients` against OpenJPEG's own decode") and `:452` to `:454` (the `.203` test, "The coefficient-level check against OpenJPEG's own decode of this codestream is the codec test").
**What**: replacing `1.402` with `color.rs`'s `1.4019876` in ritk, or `GAMMA_CR2R` with `1.4019876` in openjph, leaves both `inverse_ict_uses_annex_g_coefficients` tests and all seven ignored corpus tests green. The comparisons hold the recorded bound and reject an omitted inverse, but they cannot distinguish the two matrices the plan's AC25 names as the wrong implementation. The htj2k comment now says both that the OpenJPEG comparison is the pinned form and that the unit test holds the exact coefficients.
**Why it is wrong**: false claims in prose. Pass 1 S1 asked for the claim to say what the tolerance does hold. GLM's cycle 6 commit message states the measurement ("makes the byte-for-byte corpus comparison a recorded bound and not a coefficient pin") and corrected the `.203` test's opening comment while leaving its closing one at `:452`, which predates the range, saying the same thing. `htj2k.rs:334` also still cites "the F-239 review", pass 1's N5.
**Evidence**: mutations `S1-ritk-cr2r-codec` and `S1-ojph-cr2r-codec`, both exit 0 across `--test jpeg2000` or `--test htj2k` and `--test corpus -- --ignored`.
**Scope and regression**: prose only, present on the tip. Prefer deleting each clause to explaining it. The unit tests, once D1 is settled, are the pin.

### D3, the codecs.md colour refusal list omits the refusal this range adds
**Where**: `docs/lld/codecs.md:73` to `:83`, "What the adapter refuses for colour, and why".
**What**: the paragraph enumerates the colour refusals and names "`MCT 0` under a `YBR_RCT` or `YBR_ICT` descriptor". It does not name the converse, MCT 1 under any descriptor other than those two, which `validate_colour_shape` now refuses on both JPEG 2000 paths with `FrameMismatch`. The sentence at `:66` to `:70` still describes `Preserved` "for everything else, so an RGB data set stays RGB through the colour stage" without saying that an RGB descriptor over a transformed codestream no longer decodes.
**Why it is wrong**: a list presented as what the adapter refuses is now incomplete, and it is the paragraph a reader of the LLD consults to learn whether pass 1's D1 case can occur. PS3.5 8.2.4 note 2, which the code comment cites.
**Evidence**: `grep -n 'MCT' docs/lld/codecs.md` returns lines 46, 58 and 79 only. Mutation `D1-arm` shows the refusal is live code.
**Scope and regression**: prose only. One clause beside the MCT 0 one.

### D4, both provenance records now count Ocelli's added tests as the package's own unmodified suite
**Where**: `vendor/openjph-core-0.1.0/PATCH-PROVENANCE.md:56` to `:57`, `vendor/ritk-codecs-0.6.0/PATCH-PROVENANCE.md:53` to `:54`, against `docs/lld/codecs.md:333`.
**What**: "The package's own test suite passes unmodified with the patch applied: 154 lib tests" and "321 lib tests". The published suites are 151 and 320. Three openjph tests and one ritk test are this repository's additions inside patched files, so the 154 and 321 are not the package's own suite, and that suite now runs with tests added to it. `codecs.md:333` still says "the published package's own 151 tests pass unmodified", so the LLD and the record disagree about the same fact.
**Why it is wrong**: a false claim in a record the pins gate hashes as reviewed.
**Evidence**: `cargo test --lib` in the scratch copy reports 154 and 321, and the range's diff adds exactly those four `#[test]` functions.
**Scope and regression**: prose only. State the published count and the added count separately, in both records.

## Smells

None.

## Nitpicks

- N1. `native.rs:298` to `:304`. After the `== 3` guard the stored length is always two thirds of a container length `FrameDesc::new` proved representable, so the `try_from` fallback is unreachable on every target. The new sentence "answers the container length only on a target whose `usize` is narrower than the product" describes a target that cannot exist, and the bound still multiplies by 3 where `stored` is now always 2. The sentence it replaced was true once the guard landed.
- N2. `native.rs` states the 4:2:2 Samples per Pixel predicate twice, `== 3` in `stored_samples_per_pixel` and `!= 3` in `validate_stored_samples`. A third subsampled code added to one arm and not the other would re-open pass 1's D7. Deriving the refusal from the one function would leave one copy.
- N3. `corpus_synth.py:1342`. `generate_one` calls `require_encoders` for `colour_syntax`, which then demands `dcmcjpeg` although the comment two lines above says that entry shells out to the OpenJPEG CLI and OpenJPH only.
- N4. `generate_colour.py`'s check mode works (all four outcomes above) but no gate, CI step or bin command runs it. The lossless rows are now tied to the reference row by `corpus.rs`, so the residue of pass 1's S2 is the two lossy fixtures, which only a manual run ties to their rows. This matches the sibling generators.
- N5. Carried from pass 1 and unchanged: N2 (`COLOUR_ROW_PHOTOOMETRIC`, the duplicate reference write, `opj_compress -h` run twice) and N4 (ritk's "own behaviour is unchanged" beside the ", component 0" text). `tile.rs:799` to `:801` says `gen_ict_backward` "carries the same coefficients" as G.3's `0.34413` and `0.71414`, which is pass 1's N3 and the root of D1.
- N6. `use.html:567` dropped the old trailing `desc is a FrameDesc` comment, so the snippet now uses `desc`, `transfer_syntax_uid` and `encoded` with none of them introduced.

## Verified clean

- **The cycle 4 `git checkout`.** `vendor/ritk-codecs-0.6.0/src/jpeg_2000/packet/reader.rs` hashes to `2ed1bf22d0607915048fcb807f6ccaa599c05eb29057a2b19c95ef73e611efc1`, the value in its `PATCH-PROVENANCE.md` and in `RITK_PATCHED_HASHES`, and the pins gate passes. Its committed diff in this range is the test module alone. The other five patched files and both provenance records match their recorded hashes. `git status --porcelain --ignored` shows only `.venv`, `tools/oracle/out` and the expected ignored build and scratch paths. The reflog holds the ten commits from worktree creation at 09:46 to the tip, with no reset, checkout of a branch or rewrite after 09:46. The single stash is from `sprint/s17` and the one dangling commit is a WIP on main from 2026-11-03, both older than this story. A file-level `git checkout --` leaves no reflog entry, so the reflog cannot prove a negative, and the hash and status checks are the evidence.
- **Pass 1 D1** closed on both paths, red when removed, sentinel preserved. **D2** closed: `openjpeg-tools` install step and a `command -v opj_compress` proof in the corpus tooling job, the `REQUIRED_TOOLS` row, the README row, and the selector refusal, each executed above except a real CI run. `test_ci_does_not_pin_dcmtk` still finds the DCMTK line first. **D3** closed, `pixel-pipeline.md:395` to `:403` now matches the code. **D4** closed, six rows. **D5** closed, every count re-measured equal. **D6** closed, the docstring describes the exemptions the function applies. **D7** closed, both breaks red.
- **Pass 1 S1** closed for ritk: hand values and bytes `47 120 139` asserted, the `1.4019876` swap and the green swap red. The openjph half carries D1. **S2** closed: check mode verifies bytes, `corpus.rs` decodes `.91` and compares both lossless rows with the reference row's own Pixel Data, the ritk truncating-shift mutation and the omitted-ICT mutation are red in the corpus suite. **S3** closed, the example sizes from `Registry::decoded_output_len`, whose signature matches. **S4** closed: `gen_rct_backward` and its bound are named, `min` to `max` turns the shorter-row test red by the panic at the copy, and the six-buffer bound argument holds against `rct_backward`'s allocation at `width`.
- **N1 from pass 1.** No prose semicolon, em-dash or en-dash in any added line of the range. The one surviving semicolon in a comment at `reader.rs:394` is the published crate's own comment, moved by re-indentation, and predates F-239.
- **One step from each fix.** MCT 2 and monochrome MCT 1 refusals still exist and are tested. Both 4:2:2 codes at both wrong sample counts. A provenance-record edit and an unrecorded file in `vendor/openjph-core-0.1.0` are each refused by the pins gate.
- **The ci.yml comment** that the job never compares a generated digest with the manifest: `test_corpus_synth.py`'s determinism test compares two generations with each other, and `corpus_check.py --decode-digests` runs against the real corpus, not the job's output.
- **Unchanged consumers.** No caller outside `ocelli-codec` changed signature. `cargo test` across both crates is green. The folder viewer and oracle were not re-run in this pass, since the range adds a refusal and changes no API.

## Limitations and handoff

- Not exercised: a real CI run of the corpus tooling job, where `openjpeg-tools` on ubuntu-latest is not the 2.5.4 that `BUILT_WITH` records. Whether every `test_corpus_synth.py` case is version-independent for the two OpenJPEG rows was argued from the test source, not run on that version. 16-bit colour through ritk and tile-part COD overrides remain outside both passes.
- The PS3.5 8.2.4 note GLM quotes from the 2026d edition was not re-read at source by this reviewer.
- Disk: about 1.4 GB of ignored mutation and vendor build directories under the worktree's `target/` (`mutation1`, `openjph-pristine`, `vendor-openjph`, `vendor-ritk`) date from 10:52 to 14:16, before this range, and are worth removing at integration. This pass's scratch copy and target were deleted with Python `shutil`.
- Next owner: implementation remediates D1 to D4, then a fresh independent pass 3 reviews the changed material. All four are prose or test-label repairs and need no decoder change.

## Self-review catch figure

GLM's self-review notes 4 to 7 should have caught all 4 of the 4 blocking findings. D1: cycle 6 measured G.3's literal green coefficient going red against `-8.433824` and still wrote that the test pins the plan's hand values. D2: cycle 6 measured that the OpenJPEG comparison is a bound and not a pin, and corrected one sentence while six that say otherwise remain. D3: the D1 remediation changed a refusal that a list two paragraphs above a sentence cycle 4 rewrote enumerates. D4: cycle 7 checked the 154 and 321 counts against a test run without checking the "package's own, unmodified" qualifier on tests GLM itself added. Of the nitpicks, N1 is a sentence cycle 5 executed line by line and accepted.
