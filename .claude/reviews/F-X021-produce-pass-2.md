# F-X021 review, pass 2

**Reviewed**: uncommitted working tree on `sprint/s20`, base `f0aa7addfb43335aef4accb3d636bc114d3e4e3d`. Nothing staged. Tracked diff is 8 files, 286 insertions, 23 deletions. Untracked product path `tools/oracle/src/produce_current.rs`. This review file is the only path this pass adds.
**Author / reviewer**: Independent pass. This reviewer did not write the product diff or the remediation.
**Result**: 1 defects, 0 smells, 0 nitpicks

## Closed pass-1 findings

### Pass 1 D1, fitted volume pixels under the reference camera

Closed. `fitted_view` and `View::fit` are absent from `tools/oracle/src/produce_current.rs`. `render_mono_stack` at line 330 returns the `render_row` error. `stack_view` at line 1062 returns `executed_camera` or that error.

Mutation, restored afterwards. `synthetic_sidecar` replaced the cloned camera `viewUp` with `[1, 0, 0]`. `bin/ocelli.sh test ocelli-oracle --lib produce::current::tests::uniform_reformats_are_written_and_nonuniform_ones_are_absent -- --test-threads=1 --exact` exited 101. The uniform axial id was not written. `CurrentSummary` was `resources: ran`, `written: 10`, `omitted: 3`, `unsupported: 2`, reasons `camera-orientation-refused: 3` and `mono: 10`. A fit fallback would have published the three reformats. The published `parallelScale` therefore cannot stay on a fitted raster of this volume.

### Pass 1 D2, a built volume rewritten to `refused-as-declared` stayed green

Closed. `uniform_reformats_are_written_and_nonuniform_ones_are_absent` asserts `built` for the uniform and nonuniform volumes and `refused-as-declared` for `volume__real__ct_cmb_mml`. `real_mr_reformats_keep_the_spacing_refusal` asserts `built` for `volume__real__mr_eay131`.

Mutation, restored afterwards. After the early return in `rewrite_volume`, the object `outcome` was set to `refused-as-declared`. The same uniform test exited 101. The assertion was left `refused-as-declared`, right `built`.

## Acceptance challenged

AC1. The axial candidate digest must move when `parallelScale` moves, and a copy of the reference raw must fail that test. Restored `parallel_scale_moves_the_reformat_and_a_copy_does_not` exited 0. Forcing the axial writer to return `tools/oracle/out/<id>.raw` made it exit 101. Both digests stayed `28e181505dd2e5c7f0fd39248dceed40892555b80722ee8fdb5d43acdd2ea717`, which is the reference axial raw. The first equality against that raw had already passed. `parallelScale` 23 did not move a copied frame.

The first copy attempt used the relative path `tools/oracle/out/<id>.raw`. The test process cwd is the oracle package, so the read missed and that run exited 0. It did not exercise the guard. The red run used `{CARGO_MANIFEST_DIR}/out/<id>.raw`.

AC1 nearest half. `nearest_index` rounds a positive half away from zero, then `sample_at` reads that voxel. Changing `fraction >= 0.5` to `fraction > 0.5` made `nearest_half_index_reads_the_upper_voxel` exit 101. The bits were `1092616192` against `1126825984`, which is voxel 10 against voxel 170. Restored, the oblique file exited 0 and that test passed.

Patient plane against PS3.3 C.7.6.2.1.1 and `accept_pose`. `patient_plane` sets the column cosine to the negated `viewUp` and the row cosine to `viewUp` crossed with `viewPlaneNormal`. `ImageOrientationPatient::new` accepts that pair inside `DIRECTION_COSINE_TOLERANCE` and normalises it. Spacing on both axes is `2 * parallelScale / height`, and the reference sagittal `millimetresPerCanvasPixel` is that value for parallel scale `11.50000044703484` and height 512. The origin is the focal point minus those axes times the spacing times `(width - 1) / 2` and `(height - 1) / 2`, so pixel `(0, 0)` is the image-position centre and the focal point is the image centre. `patient_point` adds `row * column_index * column_mm` and `column * row_index * row_mm`. `column_mm` multiplies the row cosine and `row_mm` multiplies the column cosine. The synthetic `pixelSpacing` array is `[row_mm, column_mm]`, so `PixelSpacing[0]` is the row spacing. On this camera the two spacings are equal. `spacing_mm` still returns `(PixelSpacing[1], PixelSpacing[0])`. `accept_pose` requires the same negated column as `viewUp` and a gaze parallel to the slice normal. For the three uniform cameras the gaze is on the negative-normal side, so the raster stays unflipped. The published sidecar is the reference sidecar with `frame.sha256` replaced, so the published camera is the camera that built the plane.

Sagittal and coronal remainder. This is not a defect in the producer. The declared cut is one plane. `volume-params.json` has `slabThicknessMm` null. The reference sidecar records the resolved `reformat.slabThicknessMm` of 0.05 beside that null. Half of 0.05 mm is 0.025 mm. The uniform in-plane spacings are 0.25 mm and 0.5 mm, so that thickness does not reach the next voxel centre. No slab blend was added, and this pass does not ask for one.

An independent sampler, pydicom plus the same plane, half-up index, modality slope 1 intercept -1024, and `LINEAR` 40/400, was compared with `tools/oracle/out` for `volume__synthetic__ct_series_uniform`. Axial differed in 0 pixels. Sagittal differed in 600 pixels, max 255, mae 0.1449. Coronal differed in 607 pixels, max 255, mae 0.1716. Those are the reported producer figures. Shifting the index by half a voxel made the counts worse: sagittal 600 became 1454, 2297, 2568, or 3700. Coronal 607 became 2090 or more. Of the sagittal diffs, 227 equal a face-neighbour voxel and 88 are within 2 display codes. Of the coronal diffs, 321 equal a face neighbour. The unshifted plane is the nearest single-plane cut. Section 25.1 would still fail those two views because the max is 255. The design leaves that tolerance in place and leaves `--require-comparison` unbound. Copying the reference raw is the behaviour the scale test rejects.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume --test oblique nearest_half_index_reads_the_upper_voxel -- --exact` | `fraction > 0.5` in `ortho.rs` `nearest_index`, then restored to `>=` | Executed now | 101. Voxel 10 against voxel 170. |
| `bin/ocelli.sh test ocelli-oracle --lib produce::current::tests::uniform_reformats_are_written_and_nonuniform_ones_are_absent -- --test-threads=1 --exact` | `rewrite_volume` forced `refused-as-declared`, then restored | Executed now | 101. Uniform outcome was `refused-as-declared` against `built`. |
| Same uniform test | Synthetic `viewUp` set to `[1, 0, 0]`, then restored | Executed now | 101. Three reformats omitted with `camera-orientation-refused`. |
| `bin/ocelli.sh test ocelli-oracle --lib produce::current::tests::parallel_scale_moves_the_reformat_and_a_copy_does_not -- --test-threads=1 --exact` | Axial `render_reformat` returned the reference raw via `CARGO_MANIFEST_DIR`, then restored | Executed now | 101. Both digests `28e181505dd2e5c7f0fd39248dceed40892555b80722ee8fdb5d43acdd2ea717`. |
| Same scale test with relative `tools/oracle/out/<id>.raw` | Copy path missed from the package cwd | Executed now | 0. Guard not reached. Not the proof. |
| `bin/ocelli.sh test ocelli-volume --test oblique` | Restored tree | Executed now | 0. 9 passed, including `nearest_half_index_reads_the_upper_voxel`. |
| `bin/ocelli.sh test ocelli-oracle --lib produce::current::tests -- --test-threads=1` | Restored tree, corpus and `tools/oracle/out` present | Executed now | 0. 10 passed in 34.76s. 102 filtered out. |
| Independent nearest plane against the three uniform reference raws | Synthetic series, reference sidecars, half-up then `LINEAR` | Executed now | Axial 0. Sagittal 600, mae 0.1449, max 255. Coronal 607, mae 0.1716, max 255. Half-index shifts scored worse. |

Every exit code above is the test command's own status. Nothing was piped. `bin/ocelli.sh gate` was not run.

## Defects

### D1, `patient_plane` names the trilinear cut

**Where**: `tools/oracle/src/produce_current.rs:784`
**What**: The doc comment says `` `Volume::reslice_oblique` samples that plane through `sample_at` ``. `render_reformat` at line 769 calls `Volume::reslice_nearest`. That path rounds the index in `sample_nearest` and then calls `sample_at`. `reslice_oblique` remains the trilinear fill.
**Why it is wrong**: A false claim about which function places the pixels. `tools/oracle/render-params.json` asks for `NEAREST`. The spacing sentence on the line above is the live formula. The function name is not.
**Evidence**: Read of lines 764 to 785 and of `reslice_nearest` at `crates/ocelli-volume/src/ortho.rs:174`. The half-down mutation above shows the rounding is on the live nearest path.
**Scope and regression**: The pixels follow `reslice_nearest`. The comment does not. Smallest change is to name `reslice_nearest` in that sentence. No tolerance edit, no reference-byte copy, and no binding of `--require-comparison`.

## Smells

None.

## Nitpicks

None.

## Verified clean

`reslice_nearest` rounds with `nearest_index`, then calls `sample_at`. An integer index has a zero fraction, so one corner keeps the weight. `floor_nonneg` masks bits. The source does not call `f64::floor`. That rounding is not a second reconstruction kernel.

No new `as` cast in the tracked Rust diff. The only ` as ` hit is a comment in `produce.rs` about YBR stored samples. `produce_current.rs` has no `as` cast, no `unsafe`, and no `wasm-bindgen`. Absent samples are stored as a placeholder and painted `[0, 0, 0, 255]` in `paint_absent` after the LUT. `draw_mono` uses `presentation_of`, the existing `LutChain`.

Pass 1's clean list for colour, palette, YBR, 8-bit `OW`, member omission, and the spacing refusal was not re-litigated. On this tree the restored suite still passed `colour_is_emitted_and_not_recorded_as_pass`, `a_left_aligned_high_bit_is_absent`, `a_missing_member_omits_the_volume_frame`, and `real_mr_reformats_keep_the_spacing_refusal`.

The product diff was restored after every mutation. `git diff --stat` is again 8 files, 286 insertions, 23 deletions. `produce_current.rs` no longer contains the outcome rewrite, the `viewUp` overwrite, or the reference-raw return.

## Limitations and handoff

This pass did not run `compare produce` and did not run `compare gate`. The sagittal and coronal counts are the independent sampler against `tools/oracle/out`, matched to the measurement supplied with this pass. The Rust producer binary was not launched again for those two raws. The axial equality is the restored scale test, which hashes the producer output.

Eight views stay absent for the reasons already declared: left-aligned high bit, JPEG-LS RGB, nonuniform spacing orientations, and `volume__real__mr_eay131` spacing orientations. `--require-comparison` stays unbound. This pass is not sprint verification.

Implementation owns D1. A later independent pass reviews that comment change.
