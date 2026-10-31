# F-056 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s20`, base `04fdf02f044762ed4774f7c02ff14b1374d63ef5` (F-055). Nothing is staged. Tracked edits are `crates/ocelli-render/src/colour.rs`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs`, `crates/ocelli-volume/src/lib.rs`, `crates/ocelli-volume/src/ortho.rs`, `docs/lld/README.md`, `docs/lld/feature-availability.md`, `docs/lld/render-graph.md`, `docs/lld/volumes.md`, `docs/sprints/BACKLOG.md` and `docs/sprints/CURRENT_SPRINT.md`. Untracked product files are `crates/ocelli-volume/src/fusion.rs`, `crates/ocelli-volume/tests/fusion.rs`, `crates/ocelli-render/src/fusion.rs`, `crates/ocelli-render/shaders/fusion.wgsl` and `crates/ocelli-render/tests/fusion.rs`. This review file is the only path this pass adds.
**Author / reviewer**: The product diff is uncommitted, so a commit does not name its author. This pass is an independent reviewer and did not write that diff. The reviewer swapped the two in-plane spacing divisors in `sample_world`, ran temporary probes, restored every product and test edit, and then re-ran the suites.
**Result**: 3 defects, 1 smell, 2 nitpicks

## Acceptance challenged

AC5 inverse. Origin `(0, 0, 0)`, spacing `[1, 2, 5]`, identity axes, voxel `(3, 4, 0)`. PS3.3 C.7.6.2.1.1 gives world `(3, 8, 0)`. The stored sample at linear index 19 is 200. Swapping the divisors so column 0 is divided by `spacing[1]` and column 1 by `spacing[0]` makes the index `(1.5, 8, 0)`. Row 8 is outside height 5, so `sample_world` is `PlaneSample::Absent`. The forward `voxel_world` checks in the same test still see `(3, 8, 0)`. The second volume in the suite uses spacing `[2, 2, 5]`, so that swap is invisible there and that test stayed green.

Exact unit length, against a direction the permanent length-2 fixture cannot produce. `ImageOrientationPatient::new` of row `(1/sqrt(2), 1/sqrt(2), 0)` and column `(-1/sqrt(2), 1/sqrt(2), 0)` succeeds. After its normalise, `length_squared() - 1.0` has `to_bits()` `4372995238176751616` on both axes. That pattern is `+2^-52`. `plane_from_ortho` of an axial slice whose columns are that row, that column, and their normalised cross is then presented at tier A. The ledger returns `FusionError::Plane`. The seeded published byte stays 7. A direction of length 2 remains a refusal in the permanent test.

A 256-entry table the CPU colour helper accepts. `colour_after_voi` of stored 40 with red `0.5` at index 128 returns that red. `pack_table` of a PET table `(255, 0, 0)` succeeds and byte 0 is 255. `pack_table` of a CT table with red `0.5` then returns `FusionError::Unavailable`. The GPU `write_texture` between those two packs was not executed.

AC1's RGB row was reviewed against the closed formula. The design table still says question 1 is open. `## Open questions` is `None`. Both opacities 1 on CT `(10, 10, 10)` and PET `(200, 0, 0)` are `(200, 0, 0)`. At opacity 0.5 the CPU composite is `(102.5, 2.5, 2.5)`. Those are `composite` bits. They are not a framebuffer readback.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume -- fusion` | Restored `sample_world`, divisors `spacing[0]` then `spacing[1]` then `spacing[2]` | Executed now | Cargo status 0. Seven fusion tests passed. Other volume binaries reported 0 matching tests |
| `bin/ocelli.sh test ocelli-volume -- fusion` | Temporary divisors `spacing[1]` then `spacing[0]` on the `i` and `j` lines only, then restored | Executed now | Cargo status 101. `world_point_uses_column_then_row_spacing` and `a_point_outside_the_grid_is_absent` panicked at `tests/fusion.rs:192` with `this point lands on a present voxel`. Five tests passed. This is `PlaneSample::Absent`, not a compiler error. `200.0_f32.to_bits()` was not reached |
| `bin/ocelli.sh test ocelli-render -- fusion` | Restored tree after the orientation probe was removed | Executed now | Cargo status 0. 14 passed. `one_submit_encodes_fusion_and_opacity_does_not_reupload` ignored. 0 failed |
| Temporary orientation probe in `crates/ocelli-render/tests/fusion.rs` | 45-degree `ImageOrientationPatient`, axial `plane_from_ortho`, `FusionLedger::present` at tier A, then deleted | Executed in this review, not re-run after deletion | `row_delta_bits=4372995238176751616` `column_delta_bits=4372995238176751616` `present=Err(Plane)` `opacity=Err(Opacity)` `held_before=Some(7)` `held_after=Some(7)` `colour=Ok(Rgb { r: 0.5, g: 0.0, b: 0.0 })` |
| Temporary `pack_table` probe | PET table `(255, 0, 0)` then a CT channel `0.5`, then deleted | Executed in this review | `pet_pack=Ok(())` `pet_byte0=255` `ct_pack=Err(Unavailable)` |
| `bin/ocelli.sh clippy ocelli-volume` and `bin/ocelli.sh clippy ocelli-render` | Restored tree | Executed now | Cargo status 0 for each |
| `git diff -U1 -- crates/ocelli-volume/src/ortho.rs` and a search for `probe_` | Restored tree | Executed now | The in-plane divisors are `spacing[0]` then `spacing[1]`. Fusion sources do not contain `probe_` |

Host is macOS, zsh, `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Each cargo status above is the status printed by the shell immediately after that command.

## Defects

### 1. A normalised DICOM orientation is refused as a non-unit plane

Severity: defect. Reviewed source: `crates/ocelli-render/src/fusion.rs` `unit_dir` at lines 391 to 399, `crates/ocelli-volume/src/fusion.rs` `plane_from_ortho` at lines 41 to 93, and `Volume` assembly at `crates/ocelli-volume/src/lib.rs` line 595.

Trigger: row `(FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0)` and column `(-FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0)` pass `ImageOrientationPatient::new`, which normalises inside a `2e-6` length tolerance (`crates/ocelli-pixel/src/image_plane.rs` lines 38 to 54). Assembly stores `DMat3::from_cols(orientation.row(), orientation.column(), normal)`. The probe wrote those same columns and called `plane_from_ortho` for axial slice 0. `FusionLedger::present` at tier A returned `FusionError::Plane`. Both length residuals were `+2^-52`. The published seed stayed 7.

Expected: directions produced by the crate's own orientation constructor are the unit directions an axial fusion plane uses. A caller vector whose length is not 1, such as `(2, 0, 0)`, stays refused and is not renormalised.

Actual: `unit_dir` accepts only a squared length whose difference from 1.0 has bits `+0` or `-0`. The 45-degree axes miss that test by one ulp, so the plane draws nothing. Cardinal axes in the permanent fixtures still present. This pass did not survey every other cosine. `(0.6, 0.8, 0)` was not measured.

Contract: the design says the plane directions are unit vectors and a non-unit direction is a refusal. `Volume` already treats the normalised `ImageOrientationPatient` columns as those directions. The presenter and `plane_from_ortho` do not compose for this orientation.

Consumer: any fusion request whose row or column is a 45-degree axis from `ImageOrientationPatient::new`.

Smallest regression: build that orientation, take the axial plane from `plane_from_ortho`, and present it. The call must succeed. The existing `(2, 0, 0)` case must still return `FusionError::Plane`.

### 2. A refused CT table is written into the bound PET lut before the error returns

Severity: defect. Reviewed source: `FusionGpu::update` at lines 792 to 813 and `upload_tables` at lines 887 to 896 of `crates/ocelli-render/src/fusion.rs`. `decide` checks table length only, at lines 329 to 331. `byte_from_channel` is exact-integer, at `crates/ocelli-render/src/colour.rs` lines 398 to 413. This story widened that function from private to `pub(crate)` and did not change its body.

Trigger: a fusion view already exists. The next request has a PET table that packs and a 256-entry CT table whose red channel is `0.5`. `decide` and `colour_after_voi` both accept the length. The CPU helper returns `Rgb { r: 0.5, g: 0.0, b: 0.0 }` for stored 40 when that channel sits at index 128. `pack_table` of PET `(255, 0, 0)` returns `Ok` and byte 0 is 255. `pack_table` of the CT table returns `Unavailable`.

Expected: the refusal returns before any bound lut or sample texture is replaced. A later request whose hashes match the previous stamp must encode that previous table.

Actual: `upload_tables` calls `write_lut` on the PET texture and then packs the CT table. `update` assigns `self.stamp` and sets `dirty.data` only after `upload_tables` returns `Ok`. On this error the stamp stays on the previous request, so a later sample-only or same-table request sees `tables_changed == false` and does not upload the PET lut again. The encode then samples the table written by the refused call. If the same refused call also changed samples, `upload_samples` and the sample-upload counter run before the table error, and a later request with the old sample hash does not put the old samples back.

The first-bind path drops the new `FusionGpu` when `create` fails, so that failure does not replace `slot`. `create` still calls `write_lut` before the CT pack is known. Those textures are dropped with the failed view. That queue interaction was not executed.

Contract: a refused request does not replace the bound values the next encode will trust. Staging writes belong to textures that a failed request drops. `colour_after_voi` and the GPU pack disagree on a channel the length check allows.

Consumer: the next `RenderGraph::tick` that encodes the surviving view.

Smallest regression: bind a valid pair, update with a packable PET table and CT red `0.5`, observe `Unavailable`, then bind the original pair and read the PET lut bytes. They must be the original table. This pass did not run that update on a device. The pack order and the `update` control flow are the source evidence.

### 3. Grey fusion divides the display value by 255 and ignores the chain output range

Severity: defect. Reviewed source: `crates/ocelli-render/shaders/fusion.wgsl` line 26. The stack formula is `crates/ocelli-render/src/graph.rs` lines 616 to 617. `VoiParams::from_chain` copies `chain.voi().output_range()` at `crates/ocelli-render/src/voi.rs` lines 164 to 172. `lut_chain` in `voi.wgsl` already applies `ymin` and `ymax`. This pass did not edit that file.

Trigger: an omitted colormap. `LINEAR_EXACT`, centre 40, width 400, stored 40, output range 0 to 1. The window returns `0.5`. The fusion grey path returns `0.5 / 255.0`. The stack path returns `(0.5 - 0) / (1 - 0) = 0.5`. The same centre on a 0 to 65535 range is a display value near 32767.5, and `display / 255.0` is greater than 1 while the stack ratio is 0.5.

Expected: an omitted table uses the display value the existing chain produced, scaled into the colour target by that chain's `ymin` and `ymax`. The story fixture, range 0 to 255, is the one range where dividing by 255 matches the stack. The permanent grey assertion locks the CPU value `(127.5, 127.5, 127.5)` for that fixture. It does not compile or run the grey shader.

Actual: `windowed_rgb` hardcodes `display / 255.0` whenever `use_table` is 0. The colormap branch still calls `table_index` and is not this defect. The shader was not executed and no attachment was read.

Contract: the design says an omitted table repeats the grey display value, and the shader calls the existing `lut_chain` rather than re-typing the window. The uniform already carries the range `lut_chain` used.

Consumer: a fusion view with no colormap whose VOI output range is not 0 to 255.

Smallest regression: a CPU or shader evaluation of stored 40 with output range 0 to 1 must produce the same grey coefficient as `(display - ymin) / (ymax - ymin)`. The 0 to 255 fixture must stay mid-grey.

## Smells

### 1. Tier B never reports an unbindable fusion format as unavailable

`FusionGpu::create` hardcodes `R32Float` for the four sample planes and `Rgba16Float` for the window target (`crates/ocelli-render/src/fusion.rs` lines 620, 633 to 689). It does not call `get_texture_format_features`. The stack upload in `Upload::preserving_format` (`crates/ocelli-render/src/graph.rs` lines 416 to 437) keeps `R32Float` when it can be sampled and, on tier B only, may select `Rg32Float` or `Rgba32Float`. If none qualify it returns unavailable.

The design's tier B row says a device that cannot bind the two planes reports unavailable and does not switch to a CPU image. Fusion has neither the feature test nor that error. WebGPU's guaranteed features include both hardcoded formats, so this is not a tier A failure mode. This pass did not open an adapter, so the wgpu consequence on a tier B device that lacks the formats was not observed. The missing branch is the smell. A later device can fail inside texture or pipeline creation instead of returning `FusionError::Unavailable`.

## Nitpicks

### 1. `sample_world` is not the second call of `sample_at`

`docs/lld/volumes.md` says `sample_world` is a second call site of `sample_at` and is not a second kernel. The kernel claim is true. `sample_at` is still the only trilinear body. The call-site count is not. Production callers are `sample_world`, `reslice`, `reslice_oblique`, and `slab`. The oblique inverse was already in `ortho.rs` and this diff does not change it.

### 2. Absence is stored as numeric zero beside a presence flag

`docs/lld/render-graph.md` says an absent sample is presence 0, is not windowed, and is not stored zero. `fill_plane` writes `0.0` into the stored plane and `0.0` into the presence plane. The shader skips windowing when presence is not greater than 0, and `composite_sample` does not substitute black into the full formula. Only CT at opacity 0.5 stays `5.0` rather than `2.5` in the CPU test. The sentence about the buffer is the loose part. The drawn result matches the closed absence rule.

## Verified clean

`sample_world` is the inverse in the design. Column index is `direction.col(0)` dotted with the origin delta, divided by `spacing[0]`. Row index uses column 1 and `spacing[1]`. Slice index uses column 2 and `spacing[2]`. It then calls `sample_at`. There is no second kernel. `spacing[0]` is column millimetres and `spacing[1]` is row millimetres, which is PS3.3 C.7.6.2.1.1 once `PixelSpacing` is read as row then column. The mutation above made `(3, 8, 0)` absent while `voxel_world` still built that point. Restoring the divisors made the seven volume fusion tests pass.

`world_on_plane` does not renormalise. Axial steps are `spacing[0]` along column 0 and `spacing[1]` along column 1. Sagittal steps are `spacing[1]` and `spacing[2]`. Coronal steps are `spacing[0]` and `spacing[2]`. The permanent plane tests compare `(3, 8, 0)`, sagittal `(0, 2, 0)`, and coronal `(1, 0, 5)` by `to_bits`. An index outside the volume returns no plane.

`composite` is `pet * opacity_pet + ct * opacity_ct * (1 - opacity_pet)`. Volume B, `chain_b`, `opacity_b`, and `colormap_b` are the front volume. The shader string contains that product with `1.0` written out, and the test rejects `textureSample`, `@compute`, and `var<storage`. `composite_sample` draws a lone CT sample as `ct * opacity_ct` and a lone PET sample as `pet * opacity_pet`. Both absent is `None`. The shader discards when both presence tests fail and uses those two single-volume products. Substituting black into the full product is what the `5.0` versus `2.5` assertion rejects.

`colour_after_voi` runs `LutChain::map_into` and then `display_index`. `LINEAR_EXACT` centre 40 width 400, input 40, display 127.5, index 128. The fixture puts red at 128 and blue at 40 and expects red. An omitted table returns `(127.5, 127.5, 127.5)` on the CPU. That CPU value is not the shader's `/255` path, which is defect 3.

`decide` returns `TierCUnavailable` before the UID check, the opacity check, `from_chain`, the table length, the plane check, and `sample_hash`. `RenderGraph::new` returns `GraphError::TierCUnavailable` when the context tier is CPU, before pipelines are compiled. `show_fusion` returns `FusionError::Unavailable` on device loss before `decide`. `install_recovered` sets `self.fusion` to `None`. Empty UID strings are `FrameMismatch` even when both are empty. Inequality is `String` equality with no trim. A trailing space is a different string in the permanent test. A mismatch after a successful present leaves the published byte and the bound UID, and a later valid present can publish again.

Opacity outside 0 to 1, including `1.5` and NaN, is `FusionError::Opacity` before a sequence chain is reported. A sequence chain is reported before a short colormap. A short colormap is reported before a zero-width plane. Those tests use the ledger and do not write GPU textures. The opacity tests check the error variant. The byte assertion after a successful frame is the UID test. Both refusals share the `Decision::Refuse` return before the encode closure.

`Pass` is still stack, volume raycast, slab, and segmentation overlay. `Dirty` still has five flags. Fusion is `Option<FusionGpu>` on the graph. `encode` records the window pass and the fuse pass on the caller encoder and returns 2. `tick` calls `queue.submit` once and adds those two passes to `FrameOutcome.passes`. The shader module is built in `FusionGpu::create`, which `show_fusion` reaches only when the view is created. Sample uploads increment when samples are packed. An opacity-only change is not a sample-hash change, so `update` does not take that path. The ignored GPU test is the counter for that claim and was not run. Its body does not read a framebuffer. A passing submit count would not be AC1 readback of `(200, 0, 0)`.

Sample bind group entries use `TextureSampleType::Float { filterable: false }`. The shader uses `textureLoad` only. There is no SUV field, no `PatientWeight`, no dose, no half-life, no start time, and no `Units` in the new fusion sources. `FusionRequest` has the fields in the design and the literal test names no weight. The new fusion sources have no `unsafe`, no `as` cast, and no `unwrap` or `expect`. `unit_dir` compares `to_bits`. Clippy on both crates exited 0. `colour.rs` changes visibility of `display_index` and `byte_from_channel` only.

`raycast.wgsl`, `voi.wgsl`, and `tools/oracle/volume-params.json` are absent from `git diff --name-only 04fdf02`. The oblique index formula in `reslice_oblique` is outside the `sample_world` hunk. The ledger rows still say F-056 is `in-progress`. This pass did not edit them.

The design status line says approved, and a later sentence says the plan is not approved. This pass follows the workflow and treats the plan as approved. `## Open questions` is `None`.

## Limitations and handoff

`one_submit_encodes_fusion_and_opacity_does_not_reupload` was not started. There is no adapter result in this pass. A missing adapter was not observed, and it would be an environment result rather than a pass. No fusion colour attachment was read. The CPU composite bits are not that readback. `Rgba8Unorm` cannot store 102.5 exactly. The output view is private to `FusionGpu`. `COPY_SRC` is set and this pass found no copy into a surface. That privacy is a limitation of what the tick exposes. It is not counted as a defect above.

`bin/ocelli.sh check`, `bin/ocelli.sh wasm`, the oracle, the corpus, and `bin/ocelli.sh gate` were not run. The 45-degree probe and the pack probe were removed before the restored suites. A negative-opacity present in the orientation probe returned `Err(Opacity)` and left the seed at 7, but that ledger had not published a successful frame first. The permanent opacity test also does not assert the published byte.

This pass does not mark F-056 done. The next owner can take the restored tree. No mutation or probe remains in the product sources.
