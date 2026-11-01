# F-X021 review, pass 4

**Reviewed**: uncommitted working tree on `sprint/s20`, HEAD `f0aa7addfb43335aef4accb3d636bc114d3e4e3d`. Nothing staged. Tracked diff is 8 files, 286 insertions, 23 deletions. Untracked product path `tools/oracle/src/produce_current.rs`. Prior review files for passes 1 to 3 were already untracked. This review file is the only path this pass adds.
**Author / reviewer**: Independent pass. This reviewer did not write the product code or the comment fix. The human author is not named by this pass.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Closed prior findings

### Pass 2 D1, `patient_plane` named the trilinear cut

Still closed. The doc comment at `tools/oracle/src/produce_current.rs:784` names `Volume::reslice_nearest`, the rounding, and `sample_at`. `render_reformat` at line 769 calls `reslice_nearest`. A search of `produce_current.rs` finds no `reslice_oblique` and no `trilinear`. `Volume::reslice_nearest` at `crates/ocelli-volume/src/ortho.rs:174` calls `fill_oblique` with nearest set. `sample_nearest` rounds with `nearest_index` and then calls `sample_at`. `nearest_half_index_reads_the_upper_voxel` passed again in this pass.

### Pass 1 D1, fitted volume pixels under the reference camera

Still closed. `produce_current.rs` has no `View::fit`, no `fitted_view`, and no `.fit(` call. `stack_view` at line 1062 returns `executed_camera` or that error. `render_mono_stack` at line 330 returns the `render_row` error. The four stack fails checked below have an empty geometry list and an empty parameter list, so the published camera is the reference camera.

### Pass 1 D2, a built volume rewritten to `refused-as-declared`

Still closed. `rewrite_volume` at line 1231 returns immediately when `outcome` is already `refused-as-declared`. It retains written frames and replaces `sha256`. It does not assign `outcome`. The candidate `run.json` at `/tmp/ocelli-fx021-nearest/run.json` keeps `volume__synthetic__ct_series_nonuniform` and `volume__real__mr_eay131` as `built` with an empty frame list. `volume__real__ct_cmb_mml` stays `refused-as-declared` with an empty frame list.

## Acceptance challenged

The challenged claim is that the red gate is only measured divergence the design forbids papering over. A producer defect would be a wrong window, slope, or function, a new sampler, a copied reference raw, a built volume rewritten to `refused-as-declared`, or a colour view recorded as `pass`.

The report on disk is `/tmp/ocelli-fx021-gate-nearest/compare.json`. Top-level fields are `views` 94, `pass` 39, `fail` 28, `unmeasured` 27, `absent` 8, `gateVerdict` `comparison-failure`, and `green` false. The 94 records partition into those three outcomes. The eight absent ids are not among the records. `claimedVerdictViews` is 67, which is the 39 passes plus the 28 fails. Coverage is `unsupportedSourceRows` 2 and `declaredVolumeRefusals` 1. That is the comparator summary line, not a second count.

The eight absent ids are:

- `syntax__jpegls_lossless_rgb8`
- `synthetic__ct_signed_12in16_left`
- `volume__real__mr_eay131__AXIAL`
- `volume__real__mr_eay131__CORONAL`
- `volume__real__mr_eay131__SAGITTAL`
- `volume__synthetic__ct_series_nonuniform__AXIAL`
- `volume__synthetic__ct_series_nonuniform__CORONAL`
- `volume__synthetic__ct_series_nonuniform__SAGITTAL`

None of those eight has a candidate row, a candidate frame, or a candidate `.raw`. Each reference `.raw` is present. The candidate bytes that do exist for the checked fails differ from the reference bytes. The five `*Sha256` keys on `run.json` match the reference run.

Independent case, letterbox. `synthetic__ct_series_uniform__slice_000` uses the shared sidecar scale, horizontal `21.333333` and vertical `42.666667`, on a 20 by 12 grid. `CanvasExtent::rect` puts a pixel in the image only when its centre is inside the extent. That rectangle is x 43, y 0, width 426, height 512, matching the report. Column 42 has centre 42.5, which is 0.16667 px outside. Column 469 has centre 469.5, which is 0.16667 px outside. An independent read of the two RGBA8 frames, without printing pixels, finds image max absolute difference 1, 903 pixels at 1, and no pixel over 2. All 811 pixels over 2 sit on columns 42 and 469. On those pixels the candidate channel is 0 and the reference channel is not. `CpuRaster::texel` samples the pixel centre, adds `INDEX_TO_IMAGE_CENTRE`, and `texel_of` truncates. A miss is opaque black. The full-frame predicate is `monochrome_predicate` on the whole frame: at least 0.999 of pixels within 1 LSB, and no pixel differing by more than 2. The full frame is 0.9969062805175781 within 1 LSB with 811 pixels over 2, so `predicatePasses` is false. The image rectangle would pass. Bias passes. Geometry and parameters are empty. This is the GPU reference fringe against CPU point sampling. A new antialias sampler is out of scope and is not required.

Independent case, bias. `real__mr_eay131__00000001` has the same VOI on both sidecars: source `file`, function `LINEAR`, centre 329, width 743, slope 1, intercept 0. `render_mono_stack` calls `render_row` with no GPU. `raster_settled` calls `presentation_of`, which reads those five fields into one `LutChain`. The integers and the slope are exact in `f32`. The report and the raw read agree: image and full frame max absolute difference 1, fraction within 1 LSB is 1, so the predicate passes. The informative signed mean is 0.17867464484878087. `MONOCHROME_SIGNED_MEAN_BIAS` is 0.1, so bias fails. Parameter divergences are empty. Within 1 LSB with bias over the written bound is a measured result, not a wrong window.

Independent case, sigmoid. `synthetic__ct_sigmoid_width_half` has the same VOI on both sidecars: `SIGMOID`, centre 40, width 0.5, slope 0.25, intercept 0. `VoiFunction::Sigmoid` cites PS3.3 C.11.2.1.3.1. `apply_window` uses `range / (1 + exp(-4 * (x - center) / width)) + ymin`. Width above 0 is the sigmoid guard. `LINEAR` still refuses a width below 1. `sigmoid_minus_sixty_is_255_over_one_plus_e` checks that formula at centre 40 and width 400. It is not a second formula. The image max absolute difference is 57 and the fraction within 1 LSB is 0.2981220657276995. Column 469 is the same black-versus-reference fringe, 512 pixels at difference 64, and it is not the in-image miss. The producer passed the sidecar function and the sidecar width. The disagreement with cornerstone is a measured divergence. The formula stays.

`synthetic__mr_nonsquare_spacing` is the other in-image max of 64. Both sidecars are `LINEAR`, centre 1024, width 2048, slope 1, intercept 0. The rectangle is x 0, y 102, width 512, height 308. All 960 pixels over 2 are on rows 102 and 409, the first and last image rows. The interior has none. That is the same edge against point sampling, included by the centre test. It is not a wrong window and not a new sampler.

What this rejects: copying the reference raw, scoring colour as `pass`, rewriting a built volume, and binding the flag to hide the red gate. What this does not reject: the 28 measured fails and the 8 honest absences. The flag stays unbound while those remain.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Python read of `/tmp/ocelli-fx021-gate-nearest/compare.json` | Gate report the author named. Not a re-run of the gate. | Executed now | `views` 94, `pass` 39, `fail` 28, `unmeasured` 27, `absent` 8, `gateVerdict` `comparison-failure`, `green` false. Eight absent ids listed above. |
| Python aggregate of four raw pairs | Reference `tools/oracle/out` and candidate `/tmp/ocelli-fx021-nearest`. No pixel dump. | Executed now | Slice 000 image within 1 LSB. Background columns 42 and 469 hold every pixel over 2, candidate black. MR within 1 LSB. Sigmoid image max 57. Nonsquare over-2 pixels only on rows 102 and 409. |
| Sidecar field read for those four ids plus the two stack absents | Same directories | Executed now | VOI, slope, intercept, and scale match on the emitted pairs. Left high bit is bits stored 12 and high bit 15. JPEG-LS RGB is photometric `RGB` and has no candidate sidecar. |
| `run.json` volume outcomes and `*Sha256` keys | Candidate versus `tools/oracle/out/run.json` | Executed now | Nonuniform and MR volumes stay `built` with no frames. `volume__real__ct_cmb_mml` stays `refused-as-declared`. Five input digests match. Two `ok: false` rows match. Candidate raw hashes differ from the reference on the checked ids. |
| `bin/ocelli.sh test ocelli-volume --test oblique -- nearest_half_index` | `nearest_half_index_reads_the_upper_voxel` | Executed now | Exit 0. 1 passed, 0 failed, 8 filtered out. |
| `git rev-parse HEAD`, `git status --short`, `git diff --stat`, `git diff --cached --name-only` | Working tree on `sprint/s20` | Executed now | HEAD `f0aa7addfb43335aef4accb3d636bc114d3e4e3d`. Nothing staged. Tracked stat is 8 files, 286 insertions, 23 deletions. |
| Read of `patient_plane`, `render_reformat`, `stack_view`, `rewrite_volume`, `presentation_of`, `texel_of`, `CanvasExtent::rect`, `monochrome_predicate`, `JpegLsDecoder::validate_descriptor`, `StoredBits::new`, `Volume::construct` spacing test | Current sources | Executed now as a read | Closed paths still match the code. The 0.5 percent spacing test is still `worst_departure > median.abs() * 0.005`. |

Every exit code above is the command's own status. Nothing was piped. `bin/ocelli.sh gate` was not run. `compare produce` was not run. `compare gate` was not run again. No mutation was applied. No file under `tools/oracle/out/` was written.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The red gate is a limitation the design already names. It is not a producer defect. Section 25.1, `render-params.json`, `volume-params.json`, `tolerance.rs`, and `DEVIATIONS.md` are not in the diff.

`presentation_of` is the only window builder on the CPU stack path. It does not substitute a centre, a width, or a function. Colour, palette, and YBR records in this report are the seven `colour-or-us` rows, and every one is `unmeasured`. None is `pass`. `syntax__jpegls_lossless_rgb8` is absent because `JpegLsDecoder::validate_descriptor` refuses a sample count other than 1 and a photometric other than the two monochrome names. The producer does not invent a colour decoder and does not copy the reference raw. `synthetic__ct_signed_12in16_left` is absent because `StoredBits::new` requires `high_bit == bits_stored - 1`. Bits stored 12 with high bit 15 fails that constructor. `StoredBits` is not in the diff. The sigmoid row is bits stored 16 with high bit 15, which that constructor accepts, and it is emitted.

`Volume::construct` still refuses non-uniform spacing past 0.5 percent of the median gap. The three nonuniform orientations and the three `volume__real__mr_eay131` orientations are omitted. Their volume `outcome` stays `built`. `render_volumes` skips an outcome that is already `refused-as-declared`, and `volume__real__ct_cmb_mml` remains that row.

The two reference rows with `ok` false stay `ok` false on the candidate: `synthetic__us_ybr_full_422` and `syntax__deflated_explicit_vr_le`. They are the two unsupported source rows, not absent views.

`--require-comparison` is not bound. `.githooks/pre-push` calls `python3 scripts/verify_ledger.py check-commit` with the local sha and no comparison flag. `.claude/commands/verify.md` does not name the flag. `tools/oracle/src/lib.rs` and the `compare` arm of `bin/ocelli.sh` say the flag stays unbound until every current view exists and the gate is green. This pass does not bind it.

No `unsafe` and no `wasm-bindgen` were added by this reading. No patient pixel array is in this review. No dwv, Horos, or Grok source was opened. Host is macOS and the shell is zsh.

## Limitations and handoff

This pass did not re-run `compare produce` or `compare gate`. The counts are the report already at `/tmp/ocelli-fx021-gate-nearest/compare.json`. It did not re-render the uniform sagittal or coronal raws. Those two records are `unmeasured` for `weak` and `bias`, which is the census, not a pass. It did not mutate `nearest_index`, `rewrite_volume`, or the camera. Pass 2's red mutations stay prior receipts.

Eight views stay absent for the constructor refusals above. Twenty-eight judged views fail section 25.1. The design leaves that tolerance in place and leaves `--require-comparison` unbound. This pass does not bind the flag and is not sprint verification.

The producer file is still untracked. Nothing is staged. The next owner can keep this clean pass only for the tree that was read here.
