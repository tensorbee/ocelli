# S20 whole-sprint review, pass 1

**Reviewed**: `sprint/s20` at `d198eed9ddd84425ba4781854b476a5ee1a480ca`, tree `80852061458a85dde9a17619a4422fe760626b73`. `git rev-parse HEAD^{tree}` and `git write-tree` both returned that tree before this file existed. This review file is outside that tree.
**Author / reviewer**: independent. This reviewer did not write the sprint.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

Ledger. Backlog rows F-053, F-054, F-055, F-056 and F-X021 are `done`. The S20 status column in `docs/sprints/CURRENT_SPRINT.md` is `done` for the same five rows. Each story has an `AS_BUILT` heading and a `SPRINT_TRACKER.md` row. The F-X021 markdown added in `d198eed` has no em dash and no semicolon.

Sample paths. Oblique `reslice_oblique` trilinear-samples the patient plane. `reslice_nearest` uses that same plane walk, then rounds, then reads one voxel. Slab walks index steps and reduces stored samples. Fusion samples with `sample_world` on a plane of the same patient equation, and only after admission. The producer calls `Volume::reslice_nearest` for a volume reformat. These are not one kernel presented four times. Replacing the nearest half test `fraction >= 0.5` with `fraction > 0.5` made `nearest_half_index_reads_the_upper_voxel` fail. The pixel was 10, bits `1092616192`, against 170, bits `1126825984`. The trilinear mix of that same point is 95, bits `1119748096`, and that was not the value returned. The edit was restored.

Tier C. `RenderGraph::new` returns `TierCUnavailable` before pipeline compile. `present_plane` and `present_slab` return before the image is read. `fusion::decide` returns `FusionError::TierCUnavailable` before a frame-of-reference check, and `show_fusion` calls `decide` before `stage`. `stage` is what calls `sample_world`. Tier C does not build a second image and name it the GPU view.

`volume__real__ct_cmb_mml`. `render_volumes` skips a volume whose outcome is `refused-as-declared` before `Volume::construct`. `endpoint_grid` runs only for `NonUniformSpacing`. A repeated position stays `volume-construct-refused` and is not placed on an invented grid. The prior compare gate has `declaredVolumeRefusals` 1 and no volume-reformat id for that series.

SIGMOID. `crates/ocelli-pixel/src/lut.rs` and `crates/ocelli-render/shaders/voi.wgsl` are unchanged against `6808930`. The product formula is still `(ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin` on the width it is given. `sigmoid_width_below_one_keeps_the_file_width` locks file width 0.5 at centre 40 and modality 39.5 to about 4.586. `presented_sigmoid_window` changes the width only when the function is SIGMOID and the file width is inside `(0, 1)`. Decision 5 in `.claude/plans/F-X021-design.md` is not treated as a defect.

Copy. `write_pair` copies the reference raw into the reference subset and writes `frame.rgba` as the candidate raw. `window_change_moves_the_candidate_and_a_copy_does_not` requires two windows to produce different candidate digests, so a candidate that copied one reference raw would fail. `parallel_scale_moves_the_reformat_and_a_copy_does_not` requires a scale change to move the reformat off the first digest.

Submit. `encode_frame` calls `queue().submit` once after viewport passes and, when a fusion view is due, after `FusionGpu::encode`. That encode records the window pipeline and the composite pipeline and does not submit. The fusion GPU test expects `passes: 2` and `submissions() == 1` after one tick. The only new `submit` in the sprint diff is the slab readback helper in `crates/ocelli-render/tests/slab.rs`. It is not a second frame submit.

Bounds. The sprint diff adds no `unsafe` and no `wasm_bindgen`.

## Evidence

| Check | Identity | Executed now or prior receipt | Observation |
|---|---|---|---|
| `git rev-parse HEAD` and `git write-tree` | `sprint/s20` | executed now, before this file | `d198eed9ddd84425ba4781854b476a5ee1a480ca`, tree `80852061458a85dde9a17619a4422fe760626b73` |
| Backlog, current sprint, tracker, `AS_BUILT` | rows named above | read now | all five are `done`, with a heading and a tracker row |
| F-X021 markdown in `d198eed` | added lines of `*.md` | read now | no em dash and no semicolon |
| `nearest_index` half constant | `ortho.rs`, then restored | executed now | one test, exit 101, bits `1092616192` against `1126825984`, which is 10 against 170 |
| `git diff` and `git write-tree` after restore | product tree | executed now, before this file | diff empty, tree still `80852061458a85dde9a17619a4422fe760626b73` |
| `voi.wgsl` and `lut.rs` | against `6808930` | `git diff --quiet` now | unchanged, exit 0 |
| Sprint `*.rs` and `*.wgsl` added lines | `6808930..HEAD` | scanned now | no added `unsafe`, no added `wasm_bindgen`, one added `submit` and it is the slab test readback |
| Compare gate file | `/tmp/ocelli-fx021-fix-gate/compare.json` | prior receipt, aggregates only | operation `gate`, `gateVerdict` `pass`, `green` true, 102 views, 71 pass, 0 fail, 31 unmeasured, 0 absent, `declaredVolumeRefusals` 1 |

## Defects

None.

## Smells

None.

## Nitpicks

1. The F-X021 cell in `docs/sprints/SPRINT_TRACKER.md` says six produce passes and ends that list at 0/0. This tree also has `.claude/reviews/F-X021-produce-pass-7.md`, result 0 defects, 0 smells and 0 nitpicks. `AS_BUILT` names passes 1 to 6, records pass 6 as 0 defects, 0 smells and 1 nitpick, and does not name pass 7. Backlog status, the sprint table and the `AS_BUILT` heading still say `done`.

## Limitation

This pass did not re-run `bin/ocelli.sh gate --sprint` and did not re-run `compare produce`. The compare counts above are the prior gate file, not a new run. No patient pixels were copied into this review.
