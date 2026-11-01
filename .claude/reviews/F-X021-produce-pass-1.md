# F-X021 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s20`, base `f0aa7ad` (F-056, tree `104a91c5e7db`). Nothing is staged. Tracked paths: `Cargo.lock`, `bin/ocelli.sh`, `tools/oracle/Cargo.toml`, `tools/oracle/src/bin/ocelli-compare.rs`, `tools/oracle/src/lib.rs`, `tools/oracle/src/produce.rs`. Untracked path: `tools/oracle/src/produce_current.rs`. This review file is the only path this pass adds. F-056 fusion code was not reviewed.
**Author / reviewer**: the product diff author is the implementation session (grok). This reviewer is independent and did not write the product diff. Three temporary edits were applied and restored. The window read in `presentation_of` was forced to `40.0_f64`. `VolumeError::NonUniformSpacing` was mapped to `refused-as-declared`. `rewrite_volume` was made to set `outcome` to `refused-as-declared` on every volume that was not already that outcome. A trace in `stack_view` printed `executed` or `fitted` and was removed. `git diff --stat` after restoration matches the pre-review stat: 218 insertions and 22 deletions across the six tracked paths. `produce_current.rs` remains untracked. `OCELLI_REVIEW_CAMERA` is absent.
**Result**: 2 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The challenged claim is that a current volume reformat's bytes and its published sidecar are one executed pose. The design says the producer calls `Volume::construct`, `Volume::reslice`, and the existing canvas path, and that the candidate sidecar is the F-012 layout for the view that was built. The microscope rule is stronger. A copied camera does not establish the geometry that drew the pixels.

Independent case, from `tools/oracle/out/volume__synthetic__ct_series_uniform__AXIAL.json` and the volume direction in `crates/ocelli-volume/src/lib.rs`. The reference camera focal is `[-4.8500004008412425, 10.425000405311588, -3]`, `parallelScale` is `11.50000044703484`, `viewUp` is `[0, -1, 0]`, and `viewPlaneNormal` is `[0, 0, -1]`. The series orientation is row `[0.8, 0.6, 0]` and column `[0, 0, -1]`, so the slice normal is `[-0.6, 0.8, 0]`. Spacing is `[0.25, 0.5, 2.5]` and the grid is 20 by 12 by 10. The focal's slice index is `4.500000225901607`. `round_half_up` selects 5. That acquisition plane's centre is `[-5.6, 11.425, -2.75]`, which misses the focal by about `[0.75, -1.0, -0.25]` mm. `accept_pose` allows `1e-6` mm. The patient-axial camera is not the camera of that plane.

`Volume::reslice` does run. The samples are that acquisition plane, 20 by 12, 5 mm by 6 mm. `executed_camera` refuses the synthetic sidecar. `fitted_view` letterboxes it into the 512 canvas. The fit parallel scale is 3. The published sidecar keeps `11.50000044703484` and the reference focal, view up, and view-plane normal. Those pixels can disagree with the reference frame while `world_divergences` reports no camera difference.

The same run printed `fitted` for the sagittal and coronal ids. Their reference `viewUp` is `[0, 0, 1]`. The synthetic column for those cuts is the volume normal, so the expected up is `[0.6, -0.8, 0]`. They cannot pass `accept_pose` either.

What this rejects: treating the published volume sidecar as proof that the reference reset camera was executed. What this does not reject: the call to `Volume::reslice` and `sample_at`. No second sample kernel was added. Slope and intercept still go through `presentation_of`.

Tests that stay green if the producer copied the reference `.raw`:

- `colour_is_emitted_and_not_recorded_as_pass` checks the summary reason and that the file exists. It does not hash the candidate against the reference.
- `uniform_reformats_are_written_and_nonuniform_ones_are_absent` checks that the three uniform files exist. It does not hash them against the reference.
- `a_deleted_view_is_absent_and_a_missing_raw_is_structural` checks presence and `Run::load`. It does not hash the surviving raw.

`window_change_moves_the_candidate_and_a_copy_does_not` would go red for `synthetic__ct_unsigned_16`, because a copy does not move when the window changes. The omission tests would go red if a copy wrote the refused ids.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git status --short`, `git diff --stat`, `git rev-parse HEAD` | Working tree on `sprint/s20` | Executed now | HEAD `f0aa7ad`. Six tracked paths modified. `produce_current.rs` untracked. Nothing staged. |
| `bin/ocelli.sh test ocelli-oracle --lib produce::current::tests -- --test-threads=1` | Unmodified producer, `tools/oracle/out` and `corpus/data` | Executed now | Exit 0. 9 passed, 0 failed, 26.62s. |
| `bin/ocelli.sh test ocelli-oracle --bin ocelli-compare -- --test-threads=1` | Unmodified comparator binary | Executed now | Exit 0. 11 passed, 0 failed, 0.01s. |
| Window read forced to `40.0_f64`, then the window test | `presentation_of` in `tools/oracle/src/produce.rs` | Executed now, then restored | Exit 101. `assert_ne` failed. Both digests were `14ce7906c75ad131cadbe355af3452c607721f0832c22bf77192ae94c7c4ad41`. |
| `NonUniformSpacing` mapped to `refused-as-declared`, then the MR test | `volume_reason` | Executed now, then restored | Exit 101. Left `Some("refused-as-declared")`. Right `Some("volume-spacing-refused")`. |
| `rewrite_volume` set `outcome` to `refused-as-declared` for every other volume, filter `reformats` | Both reformat tests | Executed now, then restored | Exit 0. Both tests passed, 17.52s. |
| Temporary `fitted` / `executed` trace, uniform reformat test, `--nocapture` | `stack_view` | Executed now, then removed | Exit 0. Printed `fitted` for `volume__synthetic__ct_series_uniform__AXIAL`, `__SAGITTAL`, and `__CORONAL`. |
| Read `.github/workflows/ci.yml` evidence job | Workflow text, not a run | Executed now as a read | The step is `python3 scripts/verify_ledger.py check-commit HEAD`. No GPU, no oracle render, no `compare produce`. The file is not in the diff. |
| `git diff --stat` after restoration | Same six paths | Executed now | 218 insertions, 22 deletions. Matches the pre-review stat. |

Host is macOS, zsh. `rustc 1.97.1`. Cargo went through `bin/ocelli.sh`. No `bin/ocelli.sh gate`. No full corpus produce. No Docker.

## Defects

### D1, fitted volume pixels are published under the reference camera

**Severity**: defect

**Where**: `tools/oracle/src/produce_current.rs:297`, `:787`, `:1092`, and `:326`

**Trigger**: `produce_current` on the uniform synthetic series, orientations `AXIAL`, `SAGITTAL`, and `CORONAL`.

**Expected**: the published sidecar names the camera and plane that drew the pixels, or the producer writes no image for that view.

**Actual**: `render_reformat` reslices the acquisition-axis plane, builds a synthetic stack sidecar, and draws it with `raster_mono`. `stack_view` tries `executed_camera`. On failure it calls `fitted_view`, which sets both flips to false and calls `View::fit`. It does not apply the reference `parallelScale`. `render_volumes` then passes the original volume sidecar to `push_prepared`. `published_sidecar` updates `frame.sha256` and drops statistics. The camera stays the reference camera.

For the axial view the published `parallelScale` stays `11.50000044703484` and `viewPlaneNormal` stays `[0, 0, -1]`. The raster's plane normal is `[-0.6, 0.8, 0]` and its fit scale is 3. The trace printed `fitted` for all three orientations. The uniform test still passed.

`render_mono_stack` at line 326 has the same publication rule. If `render_row` fails and `render_mono_fitted` succeeds, line 246 still stores the reference sidecar. This pass did not see a current stack id take that branch. All 93 stack sidecars have a parallel scale and neither flip flag, which is not proof that `executed_camera` accepts them.

**Why it is wrong**: `world_divergences` in `tools/oracle/src/geometry.rs:80` compares the two cameras at `WORLD_TOLERANCE_MM` (`tools/oracle/src/tolerance.rs:106`, `1e-6`). A copied camera makes that list empty. `compare_view` then treats a pixel miss as the candidate's pixels (`tools/oracle/src/attribution.rs:904`). The microscope rule is that identical copied sidecars do not establish the executed pose. PS3.3 C.7.6.2.1.1 is the patient plane. The published axial camera looks along patient Z. The bytes are a fitted acquisition slice. `accept_pose` (`tools/oracle/src/produce.rs:1414`) is what rejects the synthetic plane, because the focal is not the image centre.

**Evidence**: the trace command in the table, exit 0, three `fitted` lines. The index, centre, and scale were computed from the axial sidecar and from `Volume` column order at `crates/ocelli-volume/src/lib.rs:594`. The trace was removed. The product diff stat is unchanged.

**Contract**: F-X021 design, candidate bytes come from decode, `LutChain::map_into`, `Volume::reslice`, and the existing raster, and the published sidecar is that view. HLD section 11, metadata is diffed with the pixels because a wrong pose can still look like an image.

**Scope and regression**: the three uniform reformats this producer writes. Nonuniform spacing and `volume__real__mr_eay131` stop before the raster. `volume__real__ct_cmb_mml` stays the copied declared refusal. The consumer is `ocelli-compare gate`. Smallest regression: produce the axial id and fail if `View::fit` ran while the published `camera.parallelScale` is still the reference value. Do not copy the reference raw and do not loosen a tolerance.

### D2, rewriting a built volume to refused-as-declared stays green

**Severity**: defect

**Where**: `tools/oracle/src/produce_current.rs:1707` and `:1810`. `rewrite_volume` at `:1199` does not change `outcome` today.

**Trigger**: after the early return for an outcome that is already `refused-as-declared`, set every other volume's `outcome` to `refused-as-declared`. Run the two reformat tests.

**Expected**: a test fails. A built volume that drops its frames stays `built`. It is not rewritten to `refused-as-declared`.

**Actual**: exit 0. `real_mr_reformats_keep_the_spacing_refusal` and `uniform_reformats_are_written_and_nonuniform_ones_are_absent` both passed. The mutation was restored.

**Why it is wrong**: `Run::declared_volume_refusals` (`tools/oracle/src/sidecar.rs:506`) counts `outcome` `refused-as-declared`. That count is a gate coverage field. The MR test locks the summary reason `volume-spacing-refused`. A separate mutation of that reason went red, exit 101. Neither reformat test reads `outcome`. A test that passes both ways will be counted as coverage of the contract it does not check.

**Evidence**: `bin/ocelli.sh test ocelli-oracle --lib reformats -- --test-threads=1`, exit 0, 2 passed, 17.52s. The reason-string mutation failed with left `Some("refused-as-declared")` and right `Some("volume-spacing-refused")`.

**Contract**: a built volume that cannot be cut removes frame objects and keeps outcome `built`. The declared refusal `volume__real__ct_cmb_mml` is the only `refused-as-declared` row.

**Scope and regression**: the current `rewrite_volume` matches that contract. The hole is the test. Smallest regression: after produce, assert `outcome` is still `built` for `volume__real__mr_eay131` and `volume__synthetic__ct_series_nonuniform`.

## Smells

None.

## Nitpicks

None.

## Verified clean

Entry points read: `produce_current`, `render_stacks`, `render_volumes`, `render_reformat`, `render_mono_stack`, `render_mono_fitted`, `render_colour`, `rewrite_run`, `replace_directory`, and `produce_command`. `produce_applicable` still classifies colour, palette, YBR, and reformats out of the bounded monochrome set. It was not widened.

No `as` cast in `produce_current.rs`. The only ` as ` hit is the comment that a plane sample is already `f32`. New conversions use `try_from` and `from`. No `unsafe`. No `wasm-bindgen`. No new trait, generic, or `Box<dyn>`. `Cargo.lock` adds `ocelli-cache` and `ocelli-volume` to `ocelli-oracle` and does not move the wgpu pin.

`Volume::reslice` is the cut. `sample_at` stays the only sample kernel. `PlaneSample::Present` is wrapped as `Stored` with no second mask. Slope and intercept are applied later by `presentation_of`, which builds one `LutChain`. Colour does not call `presentation_of`. `voi.source` `none` still returns `voi-absent` there. `render_row` is the first monochrome stack path, so a stack that it accepts still uses the sidecar window, including `synthetic__ct_multiframe_perframe`. That sidecar names frame 0's window 40, width 400, slope 1, and intercept -1024, with origin `file-functional-groups`. Those are the first frame's values in `scripts/corpus_synth.py`. The producer does not ignore a window the sidecar names, and it does not invent a second VOI formula. `LINEAR` and `LINEAR_EXACT` stay in the existing LUT.

`PixelSpacing` on the synthetic plane is `[row_mm, column_mm]` from `OrthoImage`. That type documents `column_mm` on the row cosine and `row_mm` on the column cosine, which is PS3.3 C.7.6.2.1.1. Slice spacing used to build the volume is the existing projected-gap median inside `Volume::construct`, not a tag.

Palette descriptors are `(0028,1101)`, `(0028,1102)`, and `(0028,1103)`, and data are `(0028,1201)`, `(0028,1202)`, and `(0028,1203)`. Entry count 0 becomes 65536. The second descriptor value is `first_mapped_input` on `LutDescriptor::new`. A negative `I16` word fails `u32::try_from` and returns `palette-refused`. It is not stored as 0. The colour test wrote both synthetic palettes. The 8-bit synthetic palette's first mapped input is 10 in `scripts/corpus_synth.py`.

`stored_sample_count` returns 2 for `YBR_FULL_422` and `YBR_PARTIAL_422` while Samples per Pixel stays 3. That matches PS3.3 C.7.6.3.1.2 and `SampleLayout::stored_samples_per_pixel`. A decoder that reports RGB takes `DecodedPhotometric::Rgb`, so JPEG baseline is not converted again. `syntax__jpegls_lossless_rgb8` was not written. `synthetic__ct_signed_12in16_left` stayed `pixel-layout-refused`. `StoredBits::new` still requires `high_bit == bits_stored - 1`.

The new 8-bit `OW` arm calls the existing `pack_u16` little-endian pack. The comment states little endian. The one current big-endian row in the manifest is `syntax/explicit_vr_be.dcm`, 16-bit monochrome. This pass did not execute an 8-bit big-endian `OW` file and does not treat that case as proved.

`rewrite_run` clones the reference object, so top-level `*Sha256` keys stay equal. `ok: false` rows stay. An `ok: true` row that was not written is dropped. A built volume that fails loses frame objects and does not change `outcome` in the current code. Written frame digests are the sha256 of the bytes just produced, stored on the sidecar and on the volume frame. `volume__real__ct_cmb_mml` is left untouched. The MR test locks the summary reason `volume-spacing-refused`. It does not lock `outcome`, which is D2.

`members_are_written` requires every `stackSidecar` stem to be in the prepared set. A missing member omits the reformat and does not omit the member row. That test passed.

`bin/ocelli.sh compare` with no arguments still requires `tools/oracle/out`. With arguments the binary decides. `produce` requires explicit `--reference`, `--candidate`, and `--corpus`. Other commands still default a missing candidate to the reference inside `parse_arguments`. `gate` still requires two explicit distinct directories. A missing reference returns `reference-run-absent` and the missing-reference test saw no `run.json`. `replace_directory` moves the previous candidate aside and puts it back if the staging rename fails. `--require-comparison` is not bound in this diff. The shell comment and `lib.rs` say it stays unbound. CI, the hooks, and `verify.md` are not in the diff.

Left-aligned high bit, nonuniform spacing, the MR spacing refusal, colour reasons other than `pass`, and the candidate-equals-reference refusal are real guards. The first two mutations went red. The colour test asserts `colour-not-scored`, `palette-not-scored`, and `ybr-not-scored`. Class two in `compare_view` returns `unmeasured`, not `pass`.

New comments and the `lib.rs` paragraph have no em dash and no prose semicolon. The factual sentences checked above match the code that was executed or read. No patient data was added. No dwv, Horos, or Grok source was opened.

## Limitations and handoff

This pass did not run `compare produce` over the full corpus and did not run `compare gate`. A red gate from an earlier receipt was not re-run and is not, by itself, a finding. The pixel disagreement predicted for the fitted axial plane was not measured as a raw diff. The branch trace and the camera arithmetic are the evidence. The stack `render_mono_fitted` fallback was not observed on a current stack id. An 8-bit big-endian `OW` file was not executed.

The product tree matches the pre-review diff. Implementation owns D1 and D2. A later independent pass reviews the repair. Do not bind `--require-comparison` while any current view is absent or the gate is red. This pass is not sprint verification.
