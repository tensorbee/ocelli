# F-X004 review, pass 2

**Reviewed**: working tree on `sprint/s21` at `0194c4a26d3269e83ef783e7232c3eed2c08a31b`. Nothing is staged. Unstaged tracked paths are `crates/ocelli-render/src/lib.rs`, `crates/ocelli-render/src/ortho.rs`, `docs/lld/README.md`, `docs/lld/feature-availability.md`, `docs/lld/viewport.md`, `docs/lld/volumes.md`, `docs/sprints/BACKLOG.md` and `docs/sprints/CURRENT_SPRINT.md`. Untracked `crates/ocelli-render/tests/cpu_plane.rs` is part of the reviewed tree. The unstaged diff of the tracked paths hashes to `7d279359a72c9608f31f0f39813c87d413b900a84cac6ae1fb422be70227247f`. `ortho.rs` hashes to `ce3d71903adfe56960fd442a4a13a56285b131897ae409ef9ca15eb0153dfda9`. `cpu_plane.rs` hashes to `a433df2737f87c13b01019c84071ebe9f7f81881a7855e888fb0e582fd2423ab`. `lib.rs` hashes to `15414b7fdcc0e95f5a98fbe0ba23af8da7d42cc2217a2531120c090e22bca5d9`. `raycast.wgsl` hashes to `a78aa186603bdab1f60826c9b9dab8b990413d22ac45b6b13863731250edb8e8` and is not in the diff. `graph.rs`, `cpu.rs`, `fusion.wgsl`, `voi.wgsl` and `docs/hld/DEVIATIONS.md` have empty diffs. Those `ortho.rs`, `cpu_plane.rs`, `lib.rs` and `raycast.wgsl` hashes match pass 1, so the remediation did not rewrite those bytes. This review file is not part of that product diff. Nothing was committed. During this pass `Stored(value)` was changed to `Stored(value - value)` and then restored. The restored `ortho.rs` hash is the hash above.
**Author / reviewer**: the implementation agent wrote the CPU present path, the host test and the LLD rows. A later edit rewrote the F-X004 and F-X005 sentences in `docs/sprints/CURRENT_SPRINT.md`. The reviewer is an independent microscope subagent in a separate context, is not the pass 1 reviewer, and did not write the implementation or that rewrite. This is not a human review.
**Result**: 0 defects, 0 smells, 0 nitpicks

Pass 1 D1 is closed. F-X004 stays `in-progress`.

## Acceptance challenged

D1. Pass 1 said `CURRENT_SPRINT.md` claimed F-X004 does not start while F-X003 is open, and that F-X004 and F-X005 begin blocked. Those sentences are gone. The status table says F-X004 is `in-progress` and F-X005 is `done`. `BACKLOG.md` line 190 keeps F-X003 `in-progress`. Line 212 keeps F-X004 `in-progress`. Line 213 keeps F-X005 `done`. The backlog diff changes only the F-X004 cell, from `pending` to `in-progress`. The carry paragraph says the representative host measurement stays unmet. The done-means bullets say a missing SIMD measurement is not a passed timing row, that F-X004 stays `in-progress`, and that there is no CPU volume image. The dependency paragraph says the same statuses and that the GPU-less host stays unmeasured. The audit line says F-X004 is in progress and F-X003 stays in progress. A search of the sprint file finds no remaining claim that F-X004 or F-X005 has not started or begins blocked. The word `blocked` at line 78 is the review-pass limit. Line 87 says publication is blocked by D-22. Neither sentence is about these two stories starting. The only `pending` left in the file is F-X011.

AC1. `copy_plane_stored` at `ortho.rs:79` pushes `Stored(value)`. It does not push `Stored(value - value)`. Replacing that push with `Stored(value - value)` made `cpu_plane_keeps_the_identity_samples` fail at `cpu_plane.rs:126`. The copied bits were `[0, 0, 0, 0]` against the slice 0 bit patterns `[1092616192, 1106247680, 1116471296, 1119092736]`. The command exit was 101. The refusal test still passed. Restoring `Stored(value)` made both `cpu_plane` tests pass. The command exit was 0. The restored file hash matches the hash above.

AC2. `present_plane` returns `OrthoError::TierCUnavailable` at `ortho.rs:49` before the sample loop at `ortho.rs:53`. `show_ortho` at `graph.rs:3078` calls `ortho::present_plane` and does not call `present_cpu_plane`. `graph.rs` is not in the diff.

AC3. `volume_ray_admission` returns `GraphError::TierCUnavailable` at `graph.rs:849` when the tier is CPU, before any volume image is produced. `raycast.wgsl` is unchanged. `tools/bench/subjects.json` contains no F-X004 id and no SIMD subject. The copy is the scalar `Stored(value)` push. There is no passed F-X004 timing row in that file.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff` hash, `shasum -a 256` | `sprint/s21` working tree, untracked `cpu_plane.rs` included in its own hash | Executed now | HEAD `0194c4a26d3269e83ef783e7232c3eed2c08a31b`. Eight dirty tracked paths. Nothing staged. Diff sha256 `7d279359...247f`. `ortho.rs` sha256 `ce3d7190...fda9`. `cpu_plane.rs` sha256 `a433df27...23ab`. Both match pass 1. Shader sha256 `a78aa186...b8e8`. |
| Search of `docs/sprints/CURRENT_SPRINT.md` for a claim that F-X004 or F-X005 has not started or begins blocked | current sprint file, 177 lines | Executed now | No such claim. `blocked` occurs once, at line 78, for the review-pass limit. Line 87 blocks publication under D-22. |
| `bin/ocelli.sh test ocelli-render --test cpu_plane -- --test-threads=1` then `code=$?` | reviewed tree before the mutation | Executed now | `code` 0. 2 passed, 0 failed. |
| Same command after `Stored(value)` became `Stored(value - value)` | temporary edit of `ortho.rs` only | Executed now | `code` 101. Identity test panicked at `cpu_plane.rs:126` with copied bits `[0, 0, 0, 0]`. Refusal test passed. |
| Same command after `Stored(value)` was restored | restored `ortho.rs`, hash `ce3d7190...fda9` | Executed now | `code` 0. 2 passed, 0 failed. |
| Read of `present_plane`, `copy_plane_stored`, `show_ortho`, `volume_ray_admission` | current working tree | Executed now | CPU return is before the sample loop. The copy writes `Stored(value)` only after every sample is present and the lengths match. `show_ortho` calls `present_plane`. Volume admission returns before an image. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

D1 is closed. The sprint file now agrees with `BACKLOG.md` and with the code on the points pass 1 named. F-X004 is `in-progress`. F-X005 is `done`. F-X003 stays `in-progress`. The representative GPU-less host stays unmeasured in the sprint text, in `docs/lld/viewport.md` and in the `CpuRaster` row of `docs/lld/feature-availability.md`. The file does not say that host was measured. It does not mark F-X004 `done`.

`present_plane` returns `OrthoError::TierCUnavailable` before it allocates and before it reads a sample. Tier A and tier B fall through to the existing upload. `show_ortho` still passes the host image to `present_plane`. `graph.rs` has an empty diff.

`copy_plane_stored` calls `CpuRaster::accept_mono_source` and then builds a pending list. `PlaneSample::Absent` returns `PlaneIncomplete` before `copy_from_slice`. A sample count that is not the grid count returns `Raster(Length)` before that copy. The push is `Stored(value)`. The mutation shows the shipped bit assertion fails when that copy is forced to zero. `present_cpu_plane` does not call `Volume::reslice`. `ortho.rs` has no `as` cast, no `unwrap`, no `expect`, no `unsafe`, no `wasm_bindgen`, no `queue.submit` and no SIMD loop. `cpu.rs` is not in the diff, so the raster this path uses was already in the tree.

`volume_ray_admission` still refuses tier C before a volume image. `raycast.wgsl` is not in the diff. There is no CPU volume image in this change. `tools/bench/subjects.json` has no F-X004 timing row, so a missing SIMD measurement is not recorded there as a pass.

## Limitations

The ignored GPU rows in `tests/ortho.rs` were not run. No adapter was opened. `show_ortho` was read and not called. No wasm `simd128` build was run. No benchmark subject was executed, and none was treated as a timing pass. The representative GPU-less host for F-X003 was not measured.

This pass did not repeat pass 1's removed probe for a row-step sample, a non-integer bit pattern, a tier B upload, or a stale-then-valid buffer. The shipped 1 by 1 mix pixel is only the origin. The axial, sagittal and coronal expectations are what constrain those sample orders, and the mutation constrains the copy. `RenderGraph::new` on tier C and `encode_frame` were not called.

This is a bounded repair review of the uncommitted tree. It is not a sprint-profile verification and it is not a completion record. F-X004 stays `in-progress`. The temporary edit was restored. No product file was left changed by this pass.
