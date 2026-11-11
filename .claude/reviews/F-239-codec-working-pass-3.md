# F-239 review, pass 3, codec part

**Reviewed**: `work/f-239-codec`, range `0f891849..bc50dcda` (two commits, 13 files), in the worktree `ocelli-wt-f-239-codec`, tip `bc50dcdaa6794454b508865905eaaf3eb3ae9a92`. No unstaged or staged tracked changes at review time. Untracked: the `.venv` symlink and `tools/oracle/out` only.
**Author / reviewer**: remediation by GLM in opencode under the lead ruling after pass 2 (`.claude/plans/F-239-design.md:557` to `:562`), with GLM's own self-review cycles 8 to 10 (`.claude/scratch/F-239-self-review-8.md` to `-10.md`, ignored files). Reviewed by Claude as an independent `/microscope --working` pass 3. The reviewer wrote no part of this range. ISO/IEC 15444-1 Annex G and the OpenJPEG source were not read at source in this pass, so the inverse was derived from the forward ICT the plan transcribes and OpenJPEG's coefficients are evidenced by measurement below, not by reading `mct.c`.
**Result**: 2 defects, 0 smells, 5 nitpicks

## Acceptance challenged

- **The inverse, derived independently.** From the forward ICT `Y0 = 0.299 I0 + 0.587 I1 + 0.114 I2`, `Y2 = 0.5 (I0 - Y0) / (1 - 0.299)` and `Y1 = 0.5 (I2 - Y0) / (1 - 0.114)`, the inverse is `I0 = Y0 + 1.402 Y2`, `I2 = Y0 + 1.772 Y1` and `I1 = (Y0 - 0.299 I0 - 0.114 I2) / 0.587 = Y0 - 0.3441363 Y1 - 0.7141363 Y2`. G-6's five-digit literals 0.34413 and 0.71414 are the rounded green coefficients. The RCT inverse is `I1 = Y0 - floor((Y2 + Y1) / 4)`, `I0 = Y2 + I1`, `I2 = Y1 + I1`. Both vendored decoders bind `Y1` to Cb and `Y2` to Cr, and both match these lines term for term (`colour.rs:494` to `:497` through `tile.rs:831`, `reader.rs:464` to `:469`, `reader.rs:452` to `:456`).
- **AC25's hand values, recomputed.** At `Y0 = -28, Y1 = 22, Y2 = -38` the literals give `-81.276`, `-8.43354` and `10.984`, plus 128 gives `46.724 119.56646 138.984`, rounded `47 120 139`. The plan's values hold. The published openjph constants evaluated in f32 are `0.34413627` and `0.71413624` and give green `-8.4338207`, which is 2.81e-4 from the literal value, outside the 1e-4 tolerance.
- **Order of operations.** openjph: `gen_ict_backward` on the unrounded floats (`tile.rs:731`), then `finish_deferred_level_shift` rounds, adds `2^(B-1)` and clamps (`tile_comp.rs:410` to `:433`). ritk: `inverse_ict_9_7` on the float planes, then `c.round()` (`reader.rs:431` to `:438`), then the shift and clamp after. That is inverse, rounding, DC shift, clamp, which is OpenJPEG's order. The RCT runs on the integer planes before the shift on both sides, and all six of this pass's lossless codestreams below decode byte exact at 8, 12 and 16 bits, so the extra chroma bit is not clamped.
- **Coefficient mutations, both decoders, each against the AC25 unit test.** 17 mutations in a scratch copy, own target directory, every restore followed by an mtime bump, re-baselined green after. openjph: each of the four `GAMMA_*` at plus 1.1e-4, `GAMMA_CR2R` and `GAMMA_CB2G` also at minus 1.1e-4, `GAMMA_CR2R` to `1.4019876`, the published derived expressions restored, the green pair swapped, `GAMMA_CB2G` alone to `0.344136`. ritk: each of the four literals at plus 1.1e-4, `0.71414` at minus 1.1e-4, `1.402` to `1.4019876`, the green pair to `0.344136` and `0.714136`. **All 17 red**, each failing exactly the AC25 test (`ict_backward_is_the_annex_g_transform` or `inverse_ict_9_7_is_the_annex_g_transform`), and the green swap also fails four published `ict_forward_backward_*` tests. The ruling's red-above-1e-4 rule holds on both sides.
- **The literals against OpenJPEG, discriminating case.** The same HTJ2K codestreams decoded with the tip's literals and with the published derived constants. Where the two decodes differ (4, 34, 42, 691 and 600 samples on the 8, 12 and 16-bit cases), the literal decode equals OpenJPEG at 2, 21, 28, 383 and 328 of them and the derived decode at 2, 13, 14, 222 and 199. Agreement moves toward OpenJPEG under the literals, which is what the provenance's "OpenJPEG applies the same literals" predicts. The residual is wavelet-level, shown by the no-transform controls below.

## OpenJPEG comparison on this pass's own images

Synthetic 112 by 80 RGB at 8, 12 and 16 bits, built for this pass: x and y ramps to full scale, a modular third channel, a band of uniform noise, the eight full-scale corner colours and the `200 100 50` triple scaled to depth. J2K encoded by `opj_compress` 2.5.4 (lossless default MCT 1, lossy `-I`), HTJ2K by `ojph_compress` with `-colour_trans true -prog_order LRCP`, because `opj_compress` 2.5.4 has no HTJ2K encoder. Every transformed codestream's COD says MCT 1, and every control's says MCT 0. Decoded through `Registry::decode` at the tip and through pylibjpeg-openjpeg 2.5.0 from the main checkout's `.venv`. Per-channel maximum absolute difference, Ocelli against OpenJPEG, and the count of differing samples out of 26,880:

| Case | 8-bit | 12-bit | 16-bit |
|------|-------|--------|--------|
| J2K lossless RCT `.90` | `[0, 0, 0]`, 0 | `[0, 0, 0]`, 0 | `[0, 0, 0]`, 0 |
| HTJ2K lossless RCT `.201` | `[0, 0, 0]`, 0 | `[0, 0, 0]`, 0 | `[0, 0, 0]`, 0 |
| HTJ2K lossy ICT `.203`, qstep 0.01 | `[1, 1, 1]`, 60 | `[1, 1, 1]`, 838 | `[3, 3, 3]`, 11,639 |
| HTJ2K lossy, qstep 0.01, no transform (control) | `[1, 1, 1]`, 45 | `[1, 1, 1]`, 860 | `[3, 3, 3]`, 11,656 |
| HTJ2K lossy ICT, qstep 0.001 | `[0, 0, 1]`, 3 | `[1, 1, 1]`, 873 | `[3, 3, 3]`, 11,636 |
| J2K lossy ICT `.91`, `-q 80` | `[1, 1, 1]`, 995 | `[1, 1, 1]`, 2,789 | `[11, 10, 15]`, 23,127 |
| J2K lossy, `-q 80`, MCT 0 (control) | `[1, 1, 1]`, 566 | `[2, 1, 1]`, 3,999 | `[25, 14, 12]`, 22,436 |
| J2K lossy ICT `.91`, `-r 8` | `[34, 29, 26]`, 22,308 | `[278, 276, 402]`, 25,612 | `[2012, 1676, 4596]`, 25,861 |
| J2K lossy, `-r 8`, MCT 0 (control) | `[10, 5, 15]`, 16,610 | not run | `[4917, 4559, 4627]`, 26,202 |

Every lossless case is byte exact against OpenJPEG and against the source image. For HTJ2K the ICT case and its no-transform control differ from OpenJPEG by the same amount at every depth, so the inverse ICT adds nothing measurable and the 3-code residual at 16 bits is openjph's 9/7 against OpenJPEG's. For J2K the ICT case is never worse than its MCT 0 control at matched settings, so the ritk inverse is not the source of the large rate-truncated differences, which are carried as an out-of-range concern in the handoff.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-codec -p ocelli-dicom` | worktree tip bc50dcda | now | 0, 327 passed, no failure |
| `cargo test -p ocelli-dicom --test corpus -- --ignored` | worktree, `corpus/data` symlinked to the main checkout | now | 0, 7 passed |
| openjph-core and ritk-codecs `cargo test --lib` | scratch `git archive` of the tip, own `CARGO_TARGET_DIR` | now | 0, 154 passed, 0, 321 passed |
| `python scripts/corpus_tests.py` | worktree, `.venv` interpreter | now | 0, 19 and 60 passed |
| `python scripts/pin_and_size_check.py --with-size` | worktree, pkg built 16:54 | now | 0. wasm 2,179,477, ceiling 2,258,755. decode 16,455. `cargo tree -p ocelli-wasm --target wasm32-unknown-unknown` names neither vendored crate, so the pkg predating the range does not affect the size result |
| pins gate after a one-byte append to `colour.rs`, both provenance records, and the unpatched `simd/neon_colour.rs` | scratch | now | each exit 1 naming the file, the neon case naming the three-file patch list, back to OK after restore |
| `guard_probe.py --only` `pins.openjph-patched-source-drift`, `pins.openjph-patch-provenance`, `pins.openjph-inventory`, `pins.ritk-provenance-extra`, `pins.ritk-patched-source-drift` | scratch with a git index | now | each OK, red for its declared reason |
| coefficient mutation catalogue, 17 cases | scratch, `target-v` | now | 17 red, baselines 11 and 9 `ict` tests green |
| native.rs mutations: `YBR_PARTIAL_422` dropped from the shared helper, refusal `!= 3` to `< 2` | scratch | now | both red at `decoded_output_len_is_the_decoders` |
| `corpus_synth.py --case colour_syntax` with full PATH and with a PATH holding only `ojph_compress`, `opj_compress`, `python3` | scratch, opj_compress 2.5.4 | now | all five rows' SHA-256 found once in `corpus/manifest.tsv`, and the DCMTK-free run writes all five |
| synthetic OpenJPEG comparison, 12 plus 20 plus 18 cases | scratch harness `tests/p3_measure.rs` through `Registry::decode` | now | table above |
| literal against derived constants on 9 HTJ2K ICT cases | scratch, `colour.rs` swapped and restored, hash re-checked `688d4f92...` | now | counts in Acceptance challenged |
| `unsafe_allowlist_check.py`, `prose_check.py`, `deviation_check.py` | worktree | now | OK, openjph 104 and ritk 0 unchanged, voice clean over 910 files, 31 deviations. No added line in the range contains `unsafe`, an em-dash, an en-dash or a prose semicolon |
| SHA-256 of the three openjph and four ritk patched files and both provenance records | worktree | now | each equals `PATCH-PROVENANCE.md` and `pin_and_size_check.py`. Base `colour.rs` hashes to the inventory's `a8468fef...`, so the range's colour.rs diff is the constants block alone |
| `git status --porcelain --ignored`, `git diff --quiet HEAD`, `git reflog`, `git ls-files -m -d -o` | worktree | now | see Verified clean |
| GLM's self-review receipts | `.claude/scratch/F-239-self-review-8.md` to `-10.md` | prior | read, not relied on |

## Defects

### D1, a guard note still says the D-31 openjph patch covers exactly two files
**Where**: `scripts/guards/catalogue.py:8344` to `:8347`, the `pins.openjph-inventory` probe's note.
**What**: the note ends "The recorded D-31 patch covers exactly two decode-path files." The range made it three: `OPENJPH_PATCHED_HASHES` now lists `src/transform/colour.rs`, `tile.rs` and `tile_comp.rs`, and the pins gate's own refusal text says so ("The recorded patch covers only ['src/codestream/tile.rs', 'src/codestream/tile_comp.rs', 'src/transform/colour.rs']"). The range edited the sibling note at `:8421` to drop "two" and left this one.
**Why it is wrong**: a false claim in prose describing what a guard protects, which `/microscope` section 3 makes a defect.
**Evidence**: `git grep -n "exactly two decode-path"` returns `catalogue.py:8347`. `guard_probe.py --list` prints the sentence. The pins gate refusal above names three files.
**Scope and regression**: prose only, present on the tip, read by anyone listing the guards. Smallest repair: drop the count, as `:8421` now does.

### D2, the colour.rs comment and the openjph provenance give a bound the gap exceeds
**Where**: `vendor/openjph-core-0.1.0/src/transform/colour.rs:22` to `:25`, `vendor/openjph-core-0.1.0/PATCH-PROVENANCE.md:34` to `:39`.
**What**: both say the literals differ from the forward-derived coefficients "by up to 6e-6". The exact gap on the blue-to-green coefficient is `0.3441362862 - 0.34413 = 6.29e-6`, and the published f32 constant `0.34413627` gives `6.27e-6`. Both exceed the stated bound. The provenance also calls the derived coefficients "the same values", which they are not, by that gap.
**Why it is wrong**: a false numeric claim in a source comment and in a record the pins gate hashes as reviewed. GLM's cycle 8 note computed 6.29e-6 and recorded that "up to 6e-6" holds.
**Evidence**: `numpy.float32` evaluation of the published expressions `(2 ALPHA_BF (1 - ALPHA_BF) / ALPHA_GF)` and `(2 ALPHA_RF (1 - ALPHA_RF) / ALPHA_GF)` gives `0.34413627` and `0.71413624`, gaps `6.27e-6` and `3.76e-6`.
**Scope and regression**: prose only, no pixel affected. Repairing the provenance moves `OPENJPH_PROVENANCE_SHA256`, and the colour.rs comment moves its patched hash in both records. Smallest repair: delete the number or state the measured gap, then re-pin.

## Smells

None.

## Nitpicks

- N1. `PATCH-PROVENANCE.md:15` to `:20`. "Three source files carry a recorded patch so that the codestream's own inverse multiple component transform runs on the decode path, before the DC level shift and its clamp." colour.rs's change is not for that purpose. It changes the coefficients. One clause naming the second purpose would make the opening sentence true for all three files.
- N2. `docs/hld/DEVIATIONS.md:48`, D-31 describes the openjph patch as calling the existing inverse and moving the shift, and does not say it now also changes published coefficients. The ruling asked for the provenance to record it, which it does, and the row points to that record, so this is a completeness preference.
- N3. Both codec tests named `inverse_ict_uses_annex_g_coefficients` now say in their own comments that they are a bound and not a coefficient pin. The name comes from the plan's AC25 row, so renaming is the lead's call.
- N4. Rounding mode, not in the range and not exercised. Both vendored decoders round with `f32::round`, which rounds half away from zero, and OpenJPEG's DC shift uses `opj_lrintf`, which under the default mode rounds half to even. A reconstructed float exactly at a half would differ by one code. Not observed in this pass's cases and within every recorded bound.
- N5. Carried from pass 2 N4, reported by GLM as not taken: `generate_colour.py`'s check mode is run by no gate or CI step.

## Verified clean

- **Pass 2 D1** closed. openjph's `gen_ict_backward` now carries G-6's literals, the AC25 test asserts `-81.276 -8.43354 10.984` and `47 120 139` within 1e-4, the "any of the four constants" clause is gone, and the 17 mutations above are red. The test's sentence "a change of more than 1e-4 to any one of the four turns an assert red" holds, because every input has magnitude at least 22, so the true threshold is about 4.5e-6.
- **Pass 2 D2** closed. Each of the six sentences named there now says bound and not pin, executed: both `inverse_ict_uses_annex_g_coefficients` tests and the seven corpus tests are green, the `.203` comparison is byte for byte and the `.91` one within one code. `git grep` for "holds the coefficients", "pinned where", "pinned form", "coefficient-level", "coefficient check" and "the F-239 review" outside `.claude` returns only `codecs.md:125`, which is the corrected sentence. The inverse-left-out counts 18,345 and 18,353 do not depend on the coefficients, so the change cannot have moved them.
- **Pass 2 D3** closed. `codecs.md:69` to `:70` and `:83` to `:86` name the MCT 1 refusal, and they match `validate_colour_shape`'s `1 if !matches!(...)` arm at `jpeg2000.rs:470`.
- **Pass 2 D4** closed. 154 lib tests run with 3 `#[test]` in `tile.rs`'s added module, and 321 with 1 in `reader.rs`'s, so 151 and 320 published. Both records and `codecs.md:340` agree.
- **Pass 2 nitpicks.** N1: the native.rs bound now multiplies by 2 and `bits_allocated` is at most 32 (`registry.rs:166`), so `65535 * 65535 * 2 * 32` holds. N2: one code list in `photometric_names_a_422_code`, and both mutations above are red. N3: `colour_syntax` demands only the two JPEG 2000 encoders, executed with DCMTK absent from PATH. N5: the `PHOTOOMETRIC` rename is complete, one `opj_compress -h` call, `generate_syntax_layer` still writes the saturated reference through `generate_colour_syntax_layer` at `:1223`. N6: the use.html snippet's names are introduced, and its two calls match `registry.rs:589` and `:605`.
- **Patch record and pins exact.** All seven patched files and both provenance records hash to their recorded values. The colour.rs diff from the published archive is the constants block alone. Removing the derivation leaves `ALPHA_*` and `BETA_*` untouched, so the forward path is unchanged, and the published `ict_forward_backward_*` round-trip tests pass with the literals. The NEON and x86 ICT paths load the same `colour::GAMMA_*` constants, so every architecture moves together.
- **No unsafe added.** No added line in the range contains `unsafe`, and the allow-list check reports the vendored counts unchanged.
- **Nothing outside the commits changed.** `git diff --quiet HEAD` is true. `git status --porcelain --ignored` shows only `.venv`, `tools/oracle/out` and the expected ignored build, pkg, cache and scratch paths. The reflog after pass 2's tip holds exactly the pass 2 review commit at 17:17 and GLM's two commits at 19:16 and 19:44, with no reset or checkout entry. The one stash is from `sprint/s17`. A file-level checkout leaves no reflog entry, so the hash checks above are the evidence for the vendored files, and each matches.
- **Unchanged consumers.** No signature changed. The SIMD paths and the dispatch table at `transform/mod.rs:373` consume the changed constants and the published suite covers them.

## Limitations and handoff

- **Out-of-range concern, plausible, carried to the lead.** ritk-codecs' irreversible decode differs a lot from OpenJPEG on rate-truncated codestreams, with or without the colour transform. 8-bit `-r 8`: `[34, 29, 26]` with ICT and `[10, 5, 15]` with MCT 0. 8-bit `-r 20`: `[20, 10, 11]` and `[7, 4, 18]`. At 16 bits the differences run to thousands of codes. At `-n 1 -r 8`, 16 bits, MCT 0, two channels differ by exactly 2,048, and there ritk's error against the source image is 6,143 where OpenJPEG's is 4,095. The MCT 0 controls show the inverse ICT is not the cause, so this is ritk's dequantization or 9/7 reconstruction, which predates F-239. F-239 is still what exposes it to colour `.91` data. The corpus `.91` row's one-code agreement holds for that row only. Worth a backlog story. Pass 2 already listed 16-bit colour through ritk as not exercised.
- Not exercised: the openjph one-pixel case "46 119 139" sentence at `htj2k.rs:348` to `:350` (GLM's cycle 8 reports re-measuring it), ritk's "an error's text alone gained the component index" against the published bytes (no pristine ritk archive was at hand), a real CI run, and a monochrome 16-bit `.91` control, because the adapter refused this pass's `opj_compress` PGM output with `UnsupportedPixelFormat` and that was not investigated further.
- Not read at source: ISO/IEC 15444-1 G-6 and OpenJPEG's `mct.c`. The derivation and the literal-against-derived measurement are the evidence.
- Disk: about 1.4 GB of ignored mutation and vendor build directories under the worktree's `target/` (`mutation1`, `openjph-pristine`, `vendor-openjph`, `vendor-ritk`) remain from earlier cycles. This pass's scratch copy, images and target directories were deleted with Python `shutil`.
- Next owner: implementation repairs D1 and D2, both prose, D2 with a re-pin of the openjph records, then a fresh independent pass 4 reviews the changed material.

## Self-review catch figure

GLM's self-review cycles 8 to 10 should have caught **2 of the 2** blocking findings. D2: cycle 8 computed the gap as 6.29e-6 and recorded that "up to 6e-6" holds. D1: the range edited the sibling note in the same file for the same fact, and cycle 9's stale-phrase grep did not search for the old file count. Of the nitpicks, N1 is the sentence cycle 8 rewrote to say "Three" without adjusting its purpose clause.
