# S21 review, pass 2

**Reviewed**: `sprint/s21` at `acdd82294aa72240718ea4367804d09bbf6e8caf`. `git rev-parse HEAD^{tree}` is `aad5ae2f5748ddacb2cbd2b5a99c126fbbc13e6d`. The range is `b43bf5e538fefaada6118db72c1ddb724a869754..HEAD`, commits `0194c4a26d3269e83ef783e7232c3eed2c08a31b` and `acdd82294aa72240718ea4367804d09bbf6e8caf`. `git status --porcelain` was empty before this file. F-060, F-061 and F-062 were not reopened except at the admission test those commits leave in place. This file is the only worktree change.
**Author / reviewer**: independent. This reviewer did not write either commit and is not an earlier review context. Feature reviews were receipts. The claims below were executed on this tree. This is not a human review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

F-X005. `volume_ray_admission` returns `Result<(), GraphError>`. The CPU comparison at `graph.rs:848` is the first check. `sampled_limit` is read at `graph.rs:851`. The success type is unit, so the function cannot return a colour buffer. Replacing only that comparison with `tier == Tier::A` made `volume_raycast_has_no_cpu_image` panic at `raycast.rs:121`. The command exit was 101. The comparison was restored. The restored `graph.rs` hash is `f99809416394bf737665c8dafa99eb369c7dc27af8e573b5e23b5d4ac7c26e1e`.

The shipped row uses one ready brick and limit 16. Moving the limit check above the CPU return would still let that row pass. A temporary assertion, since removed, called `volume_ray_admission(Tier::Cpu, 1, 0)` and required `TierCUnavailable`. That command exited 0. The same test expects limit 0 on tier A to be `SampledTextureLimit`. The CPU call with limit 0 did not take that error. `raycast.wgsl` blob `2a2303005d79b2f2042697b41c37611a402b7227` is the same blob as `b43bf5e`. There is no CPU volume image in this range.

F-X004. `copy_plane_stored` pushes `Stored(value)` at `ortho.rs:79`. `place` copies the slice bytes in order, and `container_at` addresses x then y. Slice 0 is therefore `[10, 30, 70, 90]`. `voxel_of` for sagittal column 0 is `(0,0,0)`, `(0,1,0)`, `(0,0,1)`, `(0,1,1)`, which is `[10, 70, 110, 150]`. Coronal row 0 is `[10, 30, 110, 130]`. Those rows passed in `cpu_plane_keeps_the_identity_samples`. The eight bytes sum to 760, and `760 / 8` is 95. `patient_point` at column 0 and row 0 is the plane origin `(0.5, 0.5, 0.5)`. Nearest of that index is `(1, 1, 1)`, which is 170. The volume tests `half_voxel_oblique_is_the_trilinear_mix` and `nearest_half_index_reads_the_upper_voxel` passed on that same point. The render copy does not call `sample_at`.

Replacing `Stored(value)` with `Stored(value - value)` made `cpu_plane_keeps_the_identity_samples` panic at `cpu_plane.rs:126`. The copied bits were `[0, 0, 0, 0]` against `[1092616192, 1106247680, 1116471296, 1119092736]`. The command exit was 101. The push was restored. The restored `ortho.rs` hash is `ce3d71903adfe56960fd442a4a13a56285b131897ae409ef9ca15eb0153dfda9`. Both `cpu_plane` tests had passed on that hash before the edit, exit 0.

`present_plane` returns `OrthoError::TierCUnavailable` at `ortho.rs:49` before the sample loop. `oblique_display_reuses_present_plane` hands that function a plane containing `Absent` and still observes `TierCUnavailable`, with the upload closure uncalled. A walk that ran first would return `PlaneIncomplete` on that plane. `ortho_viewport_is_unavailable_on_tier_c` passed the same way. `show_ortho` at `graph.rs:3078` calls `ortho::present_plane` and does not call `present_cpu_plane`. `graph.rs` is not in the range.

`a_refusal_publishes_nothing` passed on the clean tree. An absent sample leaves `[Stored(3.0), Stored(3.0)]`. A three-slot buffer for the four-sample axial plane returns `RasterError::Length` and leaves the three slots at `Stored(3.0)`. A three-byte output stays `[9, 9, 9]`.

Prose. `BACKLOG.md` marks F-X004 and F-X005 `done` and F-X003 `in-progress`. `CURRENT_SPRINT.md` says the same. The representative GPU-less host stays unmeasured. SIMD timing is not measured. `tools/bench/subjects.json` has no F-X004 id and no F-X005 id. `SimdSupport::build_target` returns `NotApplicable` unless `target_arch` is `wasm32`, and these tests ran natively. There is no CPU ray. No current sentence says F-X004 has not started. `docs/hld/DEVIATIONS.md` has an empty diff in this range.

The added Rust lines have no `as` cast, no `.unwrap(`, no `.expect(`, no `unsafe` and no `wasm_bindgen`. The library diff adds no second interpolant. `encode_frame` still submits once, at `graph.rs:3467`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD` and `HEAD^{tree}`, `git status --porcelain` | `sprint/s21` before this file | Executed now | Commit `acdd82294aa72240718ea4367804d09bbf6e8caf`. Tree `aad5ae2f5748ddacb2cbd2b5a99c126fbbc13e6d`. Status empty. |
| `git diff --name-only` and blob ids for `raycast.wgsl`, `voi.wgsl`, `fusion.wgsl`, `graph.rs`, `cpu.rs`, `DEVIATIONS.md`, `subjects.json` | `b43bf5e..HEAD` | Executed now | Those paths are absent. `raycast.wgsl` blob `2a2303005d79b2f2042697b41c37611a402b7227` matches both ends. |
| `bin/ocelli.sh test ocelli-render --test raycast -- --test-threads=1 volume_raycast_has_no_cpu_image` then `code=$?` | restored admission, `Tier::Cpu` at `graph.rs:848` | Executed now, before the tier edit | `code` 0. 1 passed, 0 failed, 25 filtered out. |
| Same command after `tier == Tier::Cpu` became `tier == Tier::A` inside `volume_ray_admission` only | temporary edit of `graph.rs` | Executed now | `code` 101. Panic at `raycast.rs:121`: `assertion failed: matches!(cpu_ray, Err(GraphError::TierCUnavailable))`. |
| Same command after restore | `graph.rs` hash `f99809416394bf737665c8dafa99eb369c7dc27af8e573b5e23b5d4ac7c26e1e` | Hash executed now. The green run is the pre-edit row above | Hash matches the reviewed file. |
| Same test with a temporary `volume_ray_admission(Tier::Cpu, 1, 0)` assertion, then removed | one brick and limit 0 on the restored CPU check | Executed now | `code` 0. 1 passed. `raycast.rs` hash restored to `8478a26e1eb84fc46333da7b75b1370dfe809b189f59fe5fba5fd146a08cd948`. |
| `bin/ocelli.sh test ocelli-render --test cpu_plane -- --test-threads=1` then `code=$?` | restored `Stored(value)` | Executed now, before the copy edit | `code` 0. 2 passed, 0 failed. |
| `bin/ocelli.sh test ocelli-render --test cpu_plane -- --test-threads=1 cpu_plane_keeps_the_identity_samples` after `Stored(value - value)` | temporary edit of `ortho.rs` only | Executed now | `code` 101. Panic at `cpu_plane.rs:126` with copied bits `[0, 0, 0, 0]` against the slice 0 bits. |
| Restore of that push | `ortho.rs` hash `ce3d71903adfe56960fd442a4a13a56285b131897ae409ef9ca15eb0153dfda9` | Executed now | Hash matches the file that passed both `cpu_plane` tests. |
| `bin/ocelli.sh test ocelli-volume --test oblique -- --test-threads=1 half_voxel_oblique_is_the_trilinear_mix nearest_half_index_reads_the_upper_voxel` | existing F-053 fixture, not in this diff | Executed now | `code` 0. 2 passed, 7 filtered out. |
| `bin/ocelli.sh test ocelli-render --test oblique --test ortho -- --test-threads=1 oblique_display_reuses_present_plane ortho_viewport_is_unavailable_on_tier_c an_absent_sample_is_not_uploaded` | `present_plane` on the restored tree | Executed now | `code` 0. Oblique 1 passed. Ortho 2 passed, 3 filtered out. |
| Added-line scan for `as` casts, `.unwrap(`, `.expect(`, `unsafe`, `wasm_bindgen`, and a second library interpolant | `git diff -U0 b43bf5e..HEAD -- crates` | Executed now | No cast, unwrap call, expect call, unsafe, or wasm-bindgen. Reslice calls are in the host test. The library push is `Stored(value)`. |
| `git rev-parse 0194c4a^{tree}` and `acdd822^{tree}` against the verify trailers | commit messages | Executed now. The gates were not re-run | `5d645b68d4f934a81b911483a78bfdcd28df116d` matches trailer `5d645b68d4f9`. `aad5ae2f5748ddacb2cbd2b5a99c126fbbc13e6d` matches trailer `aad5ae2f5748`. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`volume_ray_admission` still refuses tier C before the sampled-texture check. Tier A with one brick and with two bricks, and tier B with one brick and limit 2, stay `Ok(())` in the same host test, and that test passed before the mutation. `RenderGraph::new` still returns `TierCUnavailable` at `graph.rs:1395` before `compile`. The F-X005 commit does not edit a library body or a shader. The success type of admission is unit.

`present_cpu_plane` copies through `copy_plane_stored` and then calls the existing `CpuRaster::raster_into`. It does not call `Volume::reslice` or `Volume::reslice_oblique`. The host test does. An absent sample returns before `copy_from_slice`. A length that is not the grid count returns before that copy. `present_plane` is unchanged for tier A and tier B. `show_ortho` still calls it. The zero-copy mutation shows the shipped slice 0 assertion fails when the push is forced to zero.

`BACKLOG.md` and `CURRENT_SPRINT.md` both say F-X004 is done, F-X005 is done, and F-X003 is `in-progress`. The sprint text says the GPU-less host measurement stays unmet. It says a missing SIMD measurement is not a passed timing row. It says there is no CPU volume image. The availability row and `docs/lld/volumes.md` say there is no CPU ray and that SIMD timing is not measured. The scalar copy is the sample lock. No simd128 loop was added. The fixture bytes are synthetic. No patient data was added.

`encode_frame` has one `ctx.queue().submit`. `voi.wgsl` and `fusion.wgsl` are unchanged. The diff adds no `as` cast, no `unsafe` and no `wasm-bindgen`.

## Limitations

The ignored GPU rows in `tests/raycast.rs` and `tests/ortho.rs` were not run. No adapter was opened. `show_ortho` was read and not called. No wasm `simd128` build was run. No benchmark subject was executed, and none was treated as a timing pass. The representative GPU-less host for F-X003 was not measured. `SimdSupport::build_target` was read. Its native unit test was not run.

A second settled `present_cpu_plane` on one raster was not called. The shipped refusal test was not re-run under the zeroed copy. The clean run of that test was before the edit. The identity filter is the command that went red.

`docs/lld/render-graph.md` is outside the diff. It still says F-X005 owns the path that does not return a CPU colour vector. That sentence does not say a CPU image exists. The availability row in this range says there is no CPU ray.

The F-X005 note in `docs/sprints/AS_BUILT.md` still says F-X004 stays pending. That sentence is the note from `0194c4a`. The F-X004 section, the backlog, and the sprint table now say done. This pass did not treat the earlier note as the current status.

The feature-profile gates named in the commit trailers were not re-run. The trailer tree prefixes match `git rev-parse` of those commits. That match is not a new gate run. The temporary edits were restored. HEAD is still `acdd82294aa72240718ea4367804d09bbf6e8caf`.
