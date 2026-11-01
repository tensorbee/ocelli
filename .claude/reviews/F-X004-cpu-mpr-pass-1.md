# F-X004 review, pass 1

**Reviewed**: working tree on `sprint/s21` at `0194c4a26d3269e83ef783e7232c3eed2c08a31b`. Nothing is staged. Unstaged tracked paths are `crates/ocelli-render/src/lib.rs`, `crates/ocelli-render/src/ortho.rs`, `docs/lld/README.md`, `docs/lld/feature-availability.md`, `docs/lld/viewport.md`, `docs/lld/volumes.md`, `docs/sprints/BACKLOG.md` and `docs/sprints/CURRENT_SPRINT.md`. Untracked `crates/ocelli-render/tests/cpu_plane.rs` is part of the reviewed tree. The unstaged diff of the tracked paths hashes to `cf39b23e954d4566707d62999b1fdea01584b0b2798408830e13092954984ed5`. `ortho.rs` hashes to `ce3d71903adfe56960fd442a4a13a56285b131897ae409ef9ca15eb0153dfda9`. `cpu_plane.rs` hashes to `a433df2737f87c13b01019c84071ebe9f7f81881a7855e888fb0e582fd2423ab`. `lib.rs` hashes to `15414b7fdcc0e95f5a98fbe0ba23af8da7d42cc2217a2531120c090e22bca5d9`. `raycast.wgsl` hashes to `a78aa186603bdab1f60826c9b9dab8b990413d22ac45b6b13863731250edb8e8` and is not in the diff. `graph.rs`, `cpu.rs`, `fusion.wgsl`, `voi.wgsl` and `docs/hld/DEVIATIONS.md` have empty diffs. This review file is not part of that product diff. Nothing was committed. During this pass `Stored(value)` was changed to `Stored(value - value)` and then restored. A temporary probe test was added to `cpu_plane.rs` and then removed. The restored hashes are the hashes above.
**Author / reviewer**: the implementation agent wrote the CPU present path, the host test and the LLD rows. The reviewer is an independent microscope subagent in a separate context and did not write that code. This is not a human review.
**Result**: 1 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. The identity volume is spacing `(1, 1, 1)`, origin `(0, 0, 0)`, identity direction and `U8`. Slice 0 is `[10, 30, 70, 90]`. Slice 1 is `[110, 130, 150, 170]`. The sum is 760. The mix origin `(0.5, 0.5, 0.5)` with row `(1, 0, 0)` and column `(0, 0, 1)` is `Present(95)`, which is `760 / 8`. The shipped test also expects sagittal column 0 as `[10, 70, 110, 150]` and coronal row 0 as `[10, 30, 110, 130]`. Those orders match `voxel_of` in `ocelli-volume`. `copy_plane_stored` copies `Stored(value)` and does not call `Volume::reslice`. Replacing that copy with `Stored(value - value)` made `cpu_plane_keeps_the_identity_samples` fail at `cpu_plane.rs:126`. The copied bits were `[0, 0, 0, 0]` against the slice 0 bits. The command exit was 101. Restoring `Stored(value)` made that test pass. The command exit was 0.

The shipped 1 by 1 mix pixel is only the origin, so a wrong row cosine would still be 95. A temporary probe set `column_mm` to 0.5 on the same origin and axes. Pixel `(1, 0)` is then `(1.0, 0.5, 0.5)`. The x = 1 corners are 30, 90, 130 and 170, so the trilinear mean is 105. Nearest, with positive halves away from zero, is `(1, 1, 1)` = 170. Using the column axis for the column step would land on z = 1 and average to 140. Snapping y to 0 would average `(1, 0, 0)` and `(1, 0, 1)` to 80. The copied second sample was 105 and not those three. A hand-built plane of `-0.0` and `95.5` kept both `f32` bit patterns. An integer narrowing of the copy would have failed that probe. Display bytes were compared with a direct `CpuRaster` draw of the same samples. They are not the sample proof.

AC2. `caps::detection_tests::simd_support_is_not_applicable_off_wasm` passed on this host. The command exit was 0. `tools/bench/subjects.json` has no F-X004 timing row. The copy is the scalar `Stored(value)` push. There is no SIMD loop and no second volume interpolant. The LLD says a missing SIMD measurement is not a passed timing row.

AC3. An absent second mix sample leaves the caller `Stored` buffer at 3.0 and the output bytes at 9. A 3-slot buffer for the 2 by 2 axial plane returns `Raster(Length)` and stays at 3.0. A 3-byte output returns `OutputLength` and stays at 9. The temporary probe used a 2 by 2 grid with three present samples and a 4-slot sentinel. That returns `Length` and writes nothing. Four present samples plus a trailing `Absent` return `PlaneIncomplete` and do not write the prefix. `Stale` leaves the output at 9. The next valid present on that same buffer writes the direct frame for 95.

AC4. `present_plane` returns `TierCUnavailable` at `ortho.rs:49` before it reads `image.samples`. The tier C upload closure stayed at 0 calls in `oblique_display_reuses_present_plane` and in the probe. Tier B called the upload once with `[95.0]`. Tier A remains that same upload path. `show_ortho` at `graph.rs:3078` still calls `present_plane` and does not call `present_cpu_plane`. `raycast.wgsl` is unchanged. There is no CPU volume ray.

`OrthoError::Raster` is a new variant. The existing `present_plane` matches still expect `TierCUnavailable`, `PlaneIncomplete` and the tier A upload. Those tests passed on the reviewed tree. The restored `ortho.rs` hash is the hash above.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff` hash | `sprint/s21` working tree, untracked `cpu_plane.rs` included | Executed now | HEAD `0194c4a26d3269e83ef783e7232c3eed2c08a31b`. Eight dirty tracked paths. Nothing staged. Diff sha256 `cf39b23e...4ed5`. `ortho.rs` sha256 `ce3d7190...fda9`. `cpu_plane.rs` sha256 `a433df27...23ab`. Shader sha256 `a78aa186...b8e8`. |
| `bin/ocelli.sh test ocelli-render --test cpu_plane --test ortho --test oblique -- --test-threads=1` | reviewed tree before the mutation | Executed now | Exit 0. `cpu_plane` 2 passed. `oblique` 1 passed. `ortho` 3 passed and 2 ignored. |
| Same identity test after `Stored(value)` became `Stored(value - value)` | temporary edit of `ortho.rs` only | Executed now | Exit 101. 0 passed, 1 failed. Panic at `cpu_plane.rs:126` with copied bits `[0, 0, 0, 0]`. File restored. Hash matches the reviewed `ortho.rs` hash. |
| `cpu_plane_keeps_the_identity_samples` after restore | restored `Stored(value)` | Executed now | Exit 0. 1 passed, 1 filtered out. |
| `bin/ocelli.sh test ocelli-render --lib simd_support_is_not_applicable_off_wasm -- --test-threads=1` | restored tree | Executed now | Exit 0. `caps::detection_tests::simd_support_is_not_applicable_off_wasm` passed. 117 filtered out. |
| `bin/ocelli.sh clippy ocelli-render` | restored tree | Executed now | Exit 0. Finished in 3.54s. No warning text. |
| Temporary `review_probe_bits_axes_refusal_and_tier_b` | probe text added to `cpu_plane.rs`, then deleted | Executed now | Exit 0. Second sample bits were 105, not 170, 140 or 80. `-0.0` and `95.5` bits matched. Mismatched sample length and a trailing absent sample left the sentinel buffer unchanged. Tier B uploaded once. Tier C did not call the closure. `Stale` and the absent plane left `out` at 9, and the following valid present wrote the direct 95 frame. The probe is not in the tree. |
| Read of `present_plane`, `copy_plane_stored`, `present_cpu_plane`, `show_ortho` | current working tree | Executed now | CPU return is before the sample loop. The copy writes only after every sample is present and the lengths match. `show_ortho` calls `present_plane`. |

## Defects

### D1, sprint text still says F-X004 has not started

**Where**: `docs/sprints/CURRENT_SPRINT.md:131` and `docs/sprints/CURRENT_SPRINT.md:146`
**What**: The status table in the same file says F-X004 is `in-progress` and F-X005 is `done`. Line 131 says F-X004 does not start while F-X003 is open. Line 146 says F-X004 and F-X005 begin blocked. The backlog row for F-X004 is `in-progress`, and `present_cpu_plane` is in the working tree.
**Why it is wrong**: The sprint file says `BACKLOG.md` is the authority for the hand-typed status column. Those two sentences contradict that authority and the code. A reader who stops at the dependency paragraph will treat a story with a present path as blocked, and will treat the done volume decision as blocked too. F-X003 staying `in-progress` is true. The host measurement staying unmet is true. The false part is the claim that F-X004 has not started.
**Evidence**: Read of `CURRENT_SPRINT.md` against `BACKLOG.md` line 190 for F-X003, line 212 for F-X004 and line 213 for F-X005. The diff updates the status table, the carry paragraph and the audit line to `in-progress`, and leaves lines 131 and 146 in place.
**Scope and regression**: Documentation only. Pixel behaviour does not depend on these sentences. Present on this working tree, not only on historical text. The smallest change is to rewrite lines 131 and 146 so they match the status table. Do not mark F-X004 `done`. Do not mark F-X003 `done`. Do not describe the representative GPU-less host as measured.

## Smells

None.

## Nitpicks

None.

## Verified clean

`present_plane` returns `OrthoError::TierCUnavailable` before it allocates and before it reads a sample. Tier A and tier B fall through to the existing upload. The new `Raster` variant is not returned by that function. `show_ortho` still passes the host image to `present_plane`. The ignored GPU rows were not needed to see that call.

`copy_plane_stored` calls `CpuRaster::accept_mono_source` and then builds a pending list. `PlaneSample::Absent` returns `PlaneIncomplete` before `copy_from_slice`. A sample count that is not the grid count returns `Raster(Length)` before that copy. `present_cpu_plane` sizes a local `Vec`, copies into it, and calls `raster_into` only after the copy returns. A refusal does not write `out`. The CPU path allocates that sample copy. The GPU frame path does not call it. The diff adds no `queue.submit`.

The new Rust has no `as` cast, no `unwrap`, no `expect`, no `unsafe` and no `wasm_bindgen`. `Stored` is `pub struct Stored(pub f32)`, so the push keeps the `f32` bits. Sagittal and coronal expectations match the existing `reslice` index order. The mix value 95 is the eight-voxel mean from `sample_at`, not a new formula. There is no CPU ray. `volume_ray_admission` is outside this diff.

`docs/lld/volumes.md` says `present_plane` still returns `TierCUnavailable` before a sample is read, that tier C MPR copies present samples through `CpuRaster`, and that SIMD timing is not measured when `SimdSupport` is `NotApplicable`. `docs/lld/viewport.md` says `show_ortho` does not draw a plane on tier C and that F-X004 does not present a slab. `docs/lld/feature-availability.md` keeps the `CpuRaster` row's representative-host acceptance unmeasured, says there is no CPU ray, and does not record a passed SIMD timing row. The README index names F-X004 on the viewport, availability and volume rows. `BACKLOG.md` leaves F-X003 `in-progress` and sets F-X004 to `in-progress`, not `done`. The audit line says the same. No deviation file was edited.

The mutation shows the shipped bit assertion fails when the copy is forced to zero. The temporary probe shows the copy also keeps a non-integer, a negative zero, and a row-dependent sample that nearest and a y = 0 snap would change.

## Limitations

The two ignored `ortho.rs` GPU rows were not run. No adapter was opened. `show_ortho` was read and not called. No wasm `simd128` build was run. This host's `SimdSupport::build_target()` is `NotApplicable`. No F-X004 benchmark subject was treated as a timing pass.

The row-step sample, the non-integer bits, the tier B upload and the stale-then-valid sequence ran only in the temporary probe. That probe was removed before the restored identity test and clippy. The shipped 1 by 1 mix pixel does not constrain the row cosine. The shipped refusal test's short buffer fails in `accept_mono_source` before the pending-length check. The probe reached that check. A second settled present without `invalidate` returns `Ok` with `wrote` false and leaves the new buffer unchanged. The shipped test uses a fresh raster, so it does not show that idle return. After `invalidate`, a different sample is drawn through the same `CpuRaster`. That is the existing publish rule, not a second interpolant.

`RenderGraph::new` on tier C and `encode_frame` were not called. The representative GPU-less host for F-X003 was not measured, and the reviewed prose does not say it was. This is a bounded feature review of the uncommitted tree. It is not a sprint-profile verification and it is not a completion record. F-X004 stays `in-progress`. No product file was left changed by this pass.
