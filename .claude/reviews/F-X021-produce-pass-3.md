# F-X021 review, pass 3

**Reviewed**: uncommitted working tree on `sprint/s20`, HEAD `f0aa7addfb43335aef4accb3d636bc114d3e4e3d`. Nothing staged. Tracked diff is 8 files, 286 insertions, 23 deletions. Untracked product path `tools/oracle/src/produce_current.rs`. This review file is the only path this pass adds.
**Author / reviewer**: Independent pass. This reviewer did not write the product code or the comment fix. The human author is not named by this pass.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Closed prior findings

### Pass 2 D1, `patient_plane` named the trilinear cut

Closed. The doc comment at `tools/oracle/src/produce_current.rs:784` now says `` `Volume::reslice_nearest` rounds the index and `sample_at` reads that voxel. `` `render_reformat` at line 769 calls `reslice_nearest`. A search of `produce_current.rs` finds no `reslice_oblique` and no `trilinear`. The word `oblique` is only the `ObliquePlane` field.

### Pass 1 D1, fitted volume pixels under the reference camera

Still closed. `produce_current.rs` has no `View::fit`, no `fitted_view`, and no `.fit(` call. `stack_view` at line 1062 returns `executed_camera` or that error. `render_mono_stack` at line 330 returns the `render_row` error. No regression in this diff.

### Pass 1 D2, a built volume rewritten to `refused-as-declared`

Still closed. `rewrite_volume` at line 1231 returns immediately when `outcome` is already `refused-as-declared`. It retains written frames and replaces `sha256`. It does not assign `outcome`. `uniform_reformats_are_written_and_nonuniform_ones_are_absent` still asserts `built` for the uniform and nonuniform volumes and `refused-as-declared` for `volume__real__ct_cmb_mml`. `real_mr_reformats_keep_the_spacing_refusal` still asserts `built` for `volume__real__mr_eay131`.

## Acceptance challenged

The challenged claim is the repaired sentence. `Volume::reslice_nearest` rounds the index and `sample_at` reads that voxel, and that path is not the trilinear cut.

Independent case, from `crates/ocelli-volume/tests/oblique.rs`. The mix plane origin is `(0.5, 0.5, 0.5)`. Half away from zero selects voxel `(1, 1, 1)`, whose stored value is 170. The same point through `sample_at` without rounding is the trilinear mix 95. `nearest_half_index_reads_the_upper_voxel` asserts the bits of 170 and asserts they are not the bits of 95. This pass ran that test. It passed. That rejects a producer comment that still named `reslice_oblique`, and it rejects a nearest path that keeps the lower voxel or the blend.

`reslice_oblique` still calls `fill_oblique(plane, false)`, which calls `sample_at` on the unrounded index. That remains the trilinear path. `reslice_nearest` calls `fill_oblique(plane, true)`, then `sample_nearest`, then `nearest_index`, then `sample_at`.

The rustdoc claim "It is not a second reconstruction" matches that call. Rounding is not a second kernel. An integer index has a zero fraction inside `sample_at`, so one corner keeps the weight. The viewport claim matches the page. `tools/oracle/page/volume.mjs` sets `interpolationType` from `render-params.json`, and that file's base interpolation is `NEAREST`.

Sagittal and coronal remainder stays closed. Pass 2 judged it not a producer defect. This diff does not add a new cause.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume --test oblique -- nearest_half_index` | `tests/oblique.rs` `nearest_half_index_reads_the_upper_voxel`, current tree | Executed now | Exit 0. 1 passed, 0 failed, 8 filtered out. The selected test was `nearest_half_index_reads_the_upper_voxel`. |
| `git rev-parse HEAD`, `git branch --show-current`, `git status --short`, `git diff --stat`, `git diff --cached --name-only` | Working tree on `sprint/s20` | Executed now | HEAD `f0aa7addfb43335aef4accb3d636bc114d3e4e3d`. Nothing staged. Tracked stat is 8 files, 286 insertions, 23 deletions. `produce_current.rs` is untracked, so the comment fix is not in that stat. |
| Read of `patient_plane`, `render_reformat`, `reslice_nearest`, `sample_nearest`, `nearest_index`, `reslice_oblique`, `sample_at`, `rewrite_volume`, `stack_view` | Current sources listed below | Executed now as a read | The repaired sentence matches the call. No outcome rewrite. No `View::fit`. |

Every exit code above is the command's own status. Nothing was piped. `bin/ocelli.sh gate` was not run. No mutation was applied in this pass. Pass 2's red half-down mutation stays a prior receipt and was not repeated.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Read `patient_plane` at `tools/oracle/src/produce_current.rs:779` through `:837` and `render_reformat` at `:764` through `:777`. The plane spacing is `2 * parallelScale / height`. The cut is `built.volume.reslice_nearest(plane.oblique)`. The comment names that function, the rounding, and `sample_at`. It does not name `reslice_oblique`.

Read `Volume::reslice_nearest` at `crates/ocelli-volume/src/ortho.rs:169` through `:223`, `nearest_index` at `:372` through `:389`, `reslice_oblique` at `:165` through `:167`, and `sample_at` at `:234` through `:285`. Each rustdoc sentence holds. Positive halves with fraction `>= 0.5` move away from zero. A negative index at or below `-0.5` returns `None`, which `sample_nearest` turns into `PlaneSample::Absent`. The rounded index is then passed to `sample_at`. `sample_at` is still the level-0 trilinear sample, and the oblique entry still reaches it without rounding. `fill_oblique` does not grow a second reconstruction.

Read `rewrite_volume` at `:1231` through `:1255`, `stack_view` at `:1062`, and `render_mono_stack` at `:330`. The two pass 1 paths have not returned. Searched `produce_current.rs` for `reslice_oblique`, `trilinear`, `View::fit`, `fitted_view`, and `.fit(`. Only the nearest call and the `ObliquePlane` field remain for the cut.

Read `tools/oracle/render-params.json` base `interpolation` `NEAREST` and `tools/oracle/page/volume.mjs` where that value becomes `interpolationType` on the volume viewport. The rustdoc's viewport sentence matches that request.

Executed `bin/ocelli.sh test ocelli-volume --test oblique -- nearest_half_index`. Exit 0. One test passed. Host is macOS and the shell is zsh. No patient data was added. No dwv, Horos, or Grok source was opened.

## Limitations and handoff

This pass did not run `compare produce`, `compare gate`, or the rest of the oblique file. It did not re-mutate `nearest_index`, `rewrite_volume`, or the camera. The sagittal and coronal pixel counts from pass 2 were not measured again. Eight views stay absent for the reasons already declared. `--require-comparison` stays unbound. This pass is not sprint verification.

The comment fix is in the untracked producer file. Nothing is staged. The next owner can keep this clean pass only for the tree that was read here.
