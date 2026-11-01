# F-056 review, pass 2

**Reviewed**: uncommitted working tree on `sprint/s20`, base `04fdf02f044762ed4774f7c02ff14b1374d63ef5` (F-055). Nothing is staged. Tracked edits are `crates/ocelli-pixel/src/image_plane.rs`, `crates/ocelli-pixel/src/lib.rs`, `crates/ocelli-render/src/colour.rs`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs`, `crates/ocelli-volume/src/lib.rs`, `crates/ocelli-volume/src/ortho.rs`, `docs/lld/README.md`, `docs/lld/feature-availability.md`, `docs/lld/render-graph.md`, `docs/lld/volumes.md`, `docs/sprints/BACKLOG.md` and `docs/sprints/CURRENT_SPRINT.md`. Untracked product files are `crates/ocelli-render/shaders/fusion.wgsl`, `crates/ocelli-render/src/fusion.rs`, `crates/ocelli-render/tests/fusion.rs`, `crates/ocelli-volume/src/fusion.rs` and `crates/ocelli-volume/tests/fusion.rs`. `.claude/reviews/F-056-fusion-pass-1.md` was already untracked and was not edited. This review file is the only path this pass adds.
**Author / reviewer**: The product diff is uncommitted, so a commit does not name its author. This pass is an independent reviewer and did not write that diff or the remediation. The reviewer mutated five guards, restored every product edit, and re-ran the suites on the restored tree.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC5 inverse. Origin `(0, 0, 0)`, spacing `[1, 2, 5]`, identity axes, voxel `(3, 4, 0)`. PS3.3 C.7.6.2.1.1 gives world `(3, 8, 0)` because `PixelSpacing[0]` is row spacing and is `spacing[1]`. Swapping the in-plane divisors in `sample_world` made that point `PlaneSample::Absent`. The second volume uses spacing `[2, 2, 5]`, and that test stayed green. Restoring the divisors made all seven volume fusion tests pass.

45-degree orientation. `ImageOrientationPatient::new` of row `(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0)` and column `(-FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0)` succeeds. The restored test asserts both squared-length residuals have bits `4372995238176751616`, which is `+2^-52`, then presents the axial plane and expects the published byte to change from 7 to 9. Replacing `unit_dir` with an exact squared-length comparison made `presented.is_ok()` fail. Length `(2, 0, 0)` stays `FusionError::Plane` on the restored test and is not renormalised. `DIRECTION_COSINE_TOLERANCE` is still `2e-6`. `unit_dir` compares `(length - 1).abs()` to that constant. There is no second copy of the number in the fusion sources.

AC4. `LINEAR_EXACT`, centre 40, width 400, stored 40, output 0 to 255, is display 127.5 by PS3.3 C.11.2.1.3.2. The existing index of 127.5 is 128. The fixture puts red at 128 and blue at 40. The restored test expects red. An omitted table on that chain stays `(127.5, 127.5, 127.5)`. Output range 0 to 1 is display 0.5. The unit ratio is 0.5 for both ranges. `0.5/255` is not that ratio. This pass did not flip the index argument from the display value to the stored value.

Composite. Both opacities 1 on CT `(10, 10, 10)` and PET `(200, 0, 0)` are `(200, 0, 0)`. At opacity 0.5 the hand product is `200 * 0.5 + 10 * 0.5 * 0.5 = 102.5` and `2.5` on the other channels. Those are `composite` bits. They are not a framebuffer readback. `Rgba8Unorm` cannot store 102.5.

A fractional CT channel `0.5` is `FusionError::Unavailable` from `prepare_tables`. Skipping the CT pack made that test return `Ok`. `colour_after_voi` can still return `f32` `0.5`. `byte_from_channel` still accepts only an exact integer in 0 to 255. That split is intentional.

Format. Forcing `sample_format_for` to return `R32Float` made the tier B unbindable case `Ok(R32Float)` against `Err(Unavailable)`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume -- fusion` | Restored `sample_world`, divisors `spacing[0]` then `spacing[1]` then `spacing[2]` | Executed now, before mutations and again after restore | Exit 0 both times. Seven fusion tests passed. Other volume binaries reported 0 matching tests |
| `bin/ocelli.sh test ocelli-volume -- fusion` | Temporary divisors `spacing[1]` then `spacing[0]` on the `i` and `j` lines only, then restored | Executed now | Exit 101. `world_point_uses_column_then_row_spacing` and `a_point_outside_the_grid_is_absent` panicked at `tests/fusion.rs:192` with `this point lands on a present voxel`. That is `PlaneSample::Absent`. Five tests passed, including `second_volume_index_is_not_the_first_column` |
| `bin/ocelli.sh test ocelli-render -- fusion` | Restored tree | Executed now, before mutations and again after restore | Exit 0 both times. Lib tests 2 passed. Integration tests 16 passed. `one_submit_encodes_fusion_and_opacity_does_not_reupload` ignored. 0 failed |
| `bin/ocelli.sh test ocelli-render -- a_normalised_orientation_presents_and_length_two_does_not` | `unit_dir` temporarily required `(length_sq - 1.0).abs() == 0.0`, then restored | Executed now | Exit 101. Panic at `tests/fusion.rs:451`, `assertion failed: presented.is_ok()`. Unused-import warning for `DIRECTION_COSINE_TOLERANCE` during that compile only |
| `bin/ocelli.sh test ocelli-render -- a_fractional_ct_channel_is_refused_before_either_table_is_ok` | `prepare_tables` temporarily skipped `pack_table` of the CT bytes, then restored | Executed now | Exit 101. Panic at `src/fusion.rs:1493`. `prepare_tables` of PET `(255, 0, 0)` and CT red `0.5` was not `Err(Unavailable)` |
| `bin/ocelli.sh test ocelli-render -- the_shader_calls_the_existing_window_and_puts_pet_in_front` | `fusion.wgsl` temporarily used `display / 255.0`, then restored | Executed now | Exit 101. Panic at `tests/fusion.rs:582`. The module did not contain the stack `select` expression |
| `bin/ocelli.sh test ocelli-render -- an_unbindable_sample_format_is_unavailable` | `sample_format_for` temporarily returned `Ok(R32Float)` before any feature test, then restored | Executed now | Exit 101. `left: Ok(R32Float)` `right: Err(Unavailable)` for tier B with no sampleable stored format |
| `bin/ocelli.sh test ocelli-render -- --ignored one_submit_encodes_fusion --nocapture` | Restored tree. `TierRequest::Auto` | Executed now | Exit 0 in 0.31s. The test printed neither the tier C message nor the graph-refused message, so the counter asserts ran. One sample upload stayed 1 across an opacity change. Two submits. Pass count 2. A UID mismatch then left the submit count unchanged. This is not an attachment readback |
| `git diff --stat` and a search for `probe_` in the fusion sources | Restored tree | Executed now | 13 tracked files, 212 insertions, 48 deletions, matching the tree pinned at the start. No `probe_` in the fusion sources |

Host is macOS, zsh, `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Each exit above is the status printed by the shell immediately after that command. `voi.wgsl`, `raycast.wgsl` and `tools/oracle/volume-params.json` are absent from `git diff --name-only`.

## Closed pass-1 findings

### 1. A normalised DICOM orientation was refused as a non-unit plane

Closed. `unit_dir` in `crates/ocelli-render/src/fusion.rs` now accepts `(length_sq.sqrt() - 1.0).abs() <= DIRECTION_COSINE_TOLERANCE`. The constant is still `2e-6` in `crates/ocelli-pixel/src/image_plane.rs`, and the fusion check names that constant. The restored orientation test presents the axial plane and changes the published byte from 7 to 9. The exact squared-length comparison refuses that plane. A direction of length 2 remains `FusionError::Plane` and is not written back as a unit vector.

### 2. A refused CT table was written into the bound PET lut

Closed for the pack and for the source order of the writes. `prepare_tables` packs both tables into locals and returns `Unavailable` when `byte_from_channel` rejects a channel. Skipping the CT pack made the fractional-channel test fail. `FusionGpu::update` fills sample planes and packs tables before `commit_planes`, `write_lut` or `self.stamp = stamp`. `FusionGpu::create` does the same before `texture_2d` and before the slot is replaced. A refused update therefore leaves the previous stamp and the previous textures. A later request whose table hash matches that stamp has `tables_changed == false` and does not call `write_lut`, so it cannot sample a table written by the refused call. The floor test does not call `write_lut`. That limit is recorded below. It is not a remaining defect, because the `?` sits before every queue write in both functions.

### 3. Grey fusion divided the display value by 255

Closed. Omitted-table fusion uses `let range = voi.ymax - voi.ymin` and `let unit = select(0.0, (display - voi.ymin) / range, range > 0.0)`. That is the stack expression in `crates/ocelli-render/src/graph.rs`. Replacing it with `display / 255.0` made the shader string test fail. `voi.wgsl` was not edited. CPU `colour_after_voi` still returns display units.

### Smell. Tier B never reported an unbindable fusion format

Closed. `RenderGraph::show_fusion` takes `&ResolvedAdapter`. `sample_format_for` requires `RENDER_ATTACHMENT | TEXTURE_BINDING` on `Rgba16Float`, then calls `Upload::preserving_format`. `R32Float` wins when it can be sampled and copied. Tier B may use `Rg32Float` or `Rgba32Float`. Tier A does not substitute. `Rgba8Unorm` is not a candidate. Forcing `R32Float` made `an_unbindable_sample_format_is_unavailable` fail. `FusionGpu::create` calls that decision before any `texture_2d`. The pure test does not construct a texture.

### Nitpick. `sample_world` was described as the second call of `sample_at`

Closed. `docs/lld/volumes.md` says `sample_at` is the only kernel and names `Volume::reslice`, `Volume::reslice_oblique`, the slab column walk and `Volume::sample_world`. Production callers are those four. The slab call is `push_column`. A unit test in `ortho.rs` also calls `sample_at`. The sentence does not say `sample_world` is the second call.

### Nitpick. Absence was described as not stored zero

Closed. `docs/lld/render-graph.md` says an absent sample writes 0 into the stored plane and 0 into the presence plane. The shader does not window a texel whose presence is not greater than 0. The composite does not treat that stored 0 as a present sample. `fill_plane` writes `(0.0, 0.0)` for `PlaneSample::Absent`. `docs/lld/volumes.md` still says the `PlaneSample` itself is not stored zero, which is the enum, not the GPU plane.

### Nitpick. The presenter was described as requiring length exactly 1

Closed. `docs/lld/volumes.md` says the presenter accepts a direction whose length differs from 1 by at most `2e-6` and does not renormalise it. A direction of length 2 is refused. The fusion docs do not say the length is exactly 1.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`sample_world` divides `direction.col(0)` by `spacing[0]`, column 1 by `spacing[1]`, and column 2 by `spacing[2]`, then calls `sample_at`. A non-finite quotient is absent because `u32_from_nonneg_floor` rejects a non-finite index. That is the zero-spacing case in `docs/lld/volumes.md`. Axial `plane_from_ortho` steps `spacing[0]` then `spacing[1]`. Sagittal steps `spacing[1]` then `spacing[2]`. Coronal steps `spacing[0]` then `spacing[2]`. `world_on_plane` does not renormalise. The plane tests compare `(3, 8, 0)`, sagittal `(0, 2, 0)` and coronal `(1, 0, 5)` by `to_bits`.

`composite` is `pet * opacity_pet + ct * opacity_ct * (1 - opacity_pet)`. Volume B is PET. Only CT at opacity 0.5 is `5.0`, not `2.5`. Only PET is `pet * opacity_pet`. Both absent is `None`. The shader discards when both presence tests fail and uses the two single-volume products. It does not substitute black into the full formula. The fuse pass clears the private output before drawing. A discarded texel is that clear colour inside a successful frame. A refused request does not encode, so the previous successful output is left in place.

`decide` returns `TierCUnavailable` before the UID check, the opacity check, `from_chain` on chain A then chain B, the table length, the plane check and `sample_hash`. Empty UID strings are `FrameMismatch` even when both are empty. A trailing space is a different string. Opacity outside 0 to 1, including `1.5` and NaN, is `Opacity` before a sequence chain. A sequence chain is `Chain` before a short colormap. A short colormap is `ColormapLength` before a zero-width plane. Length 2 and zero spacing are `Plane`. Those ledger tests passed and the published seed stayed put on the refusals that check it.

`Pass` is still stack, volume raycast, slab and segmentation overlay. `Dirty` still has five flags. Fusion sets `data` and does not add a flag. `tick` calls `queue.submit` once. `FusionGpu::encode` records the window pass and the fuse pass and returns 2. The ignored test observed `viewports: 0`, `passes: 2`, one upload, and a second submit after an opacity change with the upload count still 1. A mismatch then produced `FrameOutcome::Noop` and did not change the submit count. `show_fusion` compiles the module when the view is created. `tick` does not compile it. `install_recovered` sets `self.fusion` to `None`. A lost device returns `FusionError::Unavailable` before `decide`, so it is not `FrameMismatch`. That loss path was not executed on a device.

`colour_after_voi` calls `LutChain::map_into` and then `display_index`. The shader calls `lut_chain` and `table_index` once each. There is no SUV field, no `PatientWeight`, no dose, no half-life, no start time and no `Units` on `FusionRequest`. The new fusion sources have no `unsafe`, no `as` cast, and no `unwrap` or `expect`. `byte_from_channel` was widened to `pub(crate)` and its body was not changed. Sample uploads write lane 0 of a zeroed texel. `textureLoad` reads `.r`. `Rgba16Float` is refused before textures when it lacks `RENDER_ATTACHMENT | TEXTURE_BINDING`.

The ledger rows still say F-056 is `in-progress`. This pass did not edit them. The design status line says approved, and a later sentence in that already-committed plan says the plan is not approved. This pass follows the workflow and treats the plan as approved. `## Open questions` is `None`.

## Limitations and handoff

No fusion colour attachment was read. The CPU composite bits are not AC1. The output view is private. `COPY_SRC` is set on it and this pass found no copy into a surface.

`a_fractional_ct_channel_is_refused_before_either_table_is_ok` guards `prepare_tables`. It does not call `FusionGpu::update`, `FusionGpu::create` or `write_lut`. The claim that a refused call cannot replace the bound lut is the source order above. It was not observed by reading a texture back.

`an_unbindable_sample_format_is_unavailable` calls `sample_format_for` directly. It does not construct a device texture. The ignored test's `show_fusion` returned `Ok` on this machine, so this adapter accepted a preserving sample format and `Rgba16Float` and compiled the module. The test does not name the tier. It does not read pixels. A missing adapter was not what this run observed.

The grey shader guard is a string check on `fusion_module()`. Mutating the expression made that check fail. The CPU test `omitted_grey_uses_the_chain_output_range` computes the ratio from `colour_after_voi` and would stay green if only the shader changed. The ignored test compiled the restored shader and did not check the grey coefficient.

`RenderGraph::new` on tier C returns `GraphError::TierCUnavailable` and does not call `show_fusion`. The executed `FusionError::TierCUnavailable` path is `FusionLedger::present`. The sequence-chain test puts the same sequence on both chains, so it does not separate a modality-sequence error on chain A from a VOI-sequence error on chain B. The source checks A then B.

This pass did not mutate `display_index`. It did not run clippy, `bin/ocelli.sh wasm`, `bin/ocelli.sh gate`, or the oracle. It did not cargo clean and it did not start Docker. It does not mark F-056 done.

Pass 2 counts: 0 defects, 0 smells, 0 nitpicks.
