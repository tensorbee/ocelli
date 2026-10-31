# S19 whole-sprint review, pass 1

**Reviewed**: `sprint/s19` at `2cb087f253182561eee424dce3dba3847985126e`, tree `4c2658ac21337c8fd5ffe6d91152137c9604fef8`, confirmed with `git rev-parse HEAD^{tree}` and `git write-tree` before this file. Range `ad2cdd0d20db7e361430afe3c6db1a4be5d73d19..HEAD`. The worktree was clean. This file is not in that tree.
**Author / reviewer**: Sprint commits are by Atul Sharma. Per-story reviews under `.claude/reviews/` were separate passes. This reviewer is an independent microscope subagent and did not write F-052, F-057, F-058 or F-059.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The cross-story claim is that F-058 may append a coarser level and F-057, F-052 and F-059 still stay on level 0. A `[2, 2, 2]` `U8` volume filled with `0x11` received `append_level([1, 1, 1], &[0xAB])`. The expected level-0 cut is eight bytes of `0x11`. The expected axial and sagittal samples are `Present(17.0)`. The coarse byte, if sampled, is `Present(171.0)`. The ready brick, the plane and `sync_ready_bricks` matched level 0. `key.level` was 0. The stamp did not contain `0xAB`.

That claim is not locked by the story tests alone. `ortho_planes_follow_the_volume_axes` stayed green when `sample_at` returned the first byte of level 1, because those fixtures never append. The temporary probe went red on the same mutation, `Present(171.0)` against `Present(17.0)`. Replacing the level-0 prefix with the bytes after that prefix made the probe's cut fail. The one-byte tail cannot fill the eight-byte brick. Setting the sync key to level 1 made `equal_brick_bytes_are_not_uploaded_twice` fail with left `1` and right `0`.

The shared graph claim is that orthographic upload, progressive sync and the volume ray do not share a pipeline, do not submit twice, and do not draw on tier C. `show_ortho` writes the stack stored texture. `sync_volume` writes level-0 brick textures. `bind_volume_ray` builds a separate bind group and is refused unless the pass is `VolumeRaycast`. `tick` encodes both kinds into one encoder and calls `queue.submit` once. Tier C returns before a plane walk, a brick cut, a stamp write, or a ray image.

Ledger claim: backlog rows F-052, F-057, F-058 and F-059 are `done`, the current-sprint status column matches, and each story has an AS_BUILT entry and a tracker row.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `HEAD^{tree}`, `git write-tree`, `git status --porcelain` | `sprint/s19` before this file | executed now | HEAD `2cb087f253182561eee424dce3dba3847985126e`, tree and index `4c2658ac21337c8fd5ffe6d91152137c9604fef8`, status empty |
| `git diff --stat ad2cdd0..HEAD` | four story commits plus the S19 open | executed now | 40 files, 5883 insertions, 185 deletions |
| Backlog awk for sprint `S19`, current-sprint status table, tracker rows, AS_BUILT headings | tracked sprint files | executed now | F-052, F-057, F-058 and F-059 are `done` in the backlog and the status table. Tracker rows exist. AS_BUILT headings are at lines 3728, 3744, 3760 and 3776 |
| `bin/ocelli.sh test ocelli-volume --test levels --test progressive --test ortho --test resident` | restored tree | executed now | exit 0. 5, 3, 4 and 1 passed |
| `bin/ocelli.sh test ocelli-render --test progressive --test ortho --test raycast` | restored tree, ignored GPU rows not selected | executed now | exit 0. 6, 3 and 2 passed. 3, 2 and 4 ignored |
| Temporary volume probe, append then `cut_ready_brick` and `reslice` | untracked test, deleted | executed now | exit 0 before mutation. Cut was eight `0x11` bytes. Planes were `Present(17.0)`. Level 1 stayed `[0xAB]` |
| Same probe after `sample_at` returned level 1's first byte | `ortho.rs`, then restored | executed now | exit 101. Axial sample was `Present(171.0)` against `Present(17.0)`. The four ortho fixtures stayed green |
| Same probe after `level0_prefix` returned `get(len..)` | `lib.rs`, then restored | executed now | exit 101. The cut failed. `a_ready_brick_copies_both_slices` and `signed_samples_stay_signed` also failed |
| Temporary render probe, `sync_ready_bricks` after append | untracked test, deleted | executed now | exit 0. `key.level` was 0. Samples were eight `0x11` bytes. No stamp contained `0xAB` |
| `equal_brick_bytes_are_not_uploaded_twice` after `level: 1` | `progressive.rs`, then restored | executed now | exit 101. Assertion left `1`, right `0`, at `progressive.rs:65` |
| Same test and `a_ready_brick_copies_both_slices` after restore | restored sources | executed now | exit 0. `git diff --exit-code` was empty. `git write-tree` was still `4c2658ac21337c8fd5ffe6d91152137c9604fef8` |
| `bin/ocelli.sh gate --sprint` | tree `4c2658ac21337c8fd5ffe6d91152137c9604fef8` | prior receipt, not re-run | exit 0, 31 gates, corpus pass. Cited only because `HEAD^{tree}` still matches |
| `rustc --version` | host | executed now | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Level 0 stays the prefix. `append_level` copies after `data.len()` and does not change `dims`. `VoxelStore::slice` stops at `dims[2]`. The last level-0 slice ends where the coarse bytes begin. `level0_prefix` passes `get(..len)` into `Brick::cut`, and `ready_bricks` uses `brick_grid`, whose level is 0. `sync_ready_bricks` stamps `BrickKey.level` as 0. `reslice` reads `container_at` through those slices. `bind_volume_ray` does not call `level_bytes` or `brick_grid_at`. The ray samples textures the caller names, and the sprint uploader does not create a level above 0.
- Pipelines stay apart. `compile` builds the stack module and `volume_ray_module` as two pipelines. The keys are `(Stack, Composite, tier)` and `(VolumeRaycast, Composite, tier)`. `pipeline_index` compares the full key and the family. Encode uses the ray bind groups only when the pass is `VolumeRaycast` and `slot.ray` is present. Otherwise a volume viewport is skipped. It is not drawn with the stack groups. `set_view` remains stack-only. `set_voi` writes the same thirty-two byte uniform for both passes and does not upload a texture.
- One submit remains. `graph.rs` has one `queue.submit`, at line 2750, after the viewport loop. `show_ortho`, `sync_volume` and `bind_volume_ray` use `write_texture` or `write_buffer`. They do not submit. `brick_uploads` counts brick copies. `uploads` counts stored-sample copies. A window change increments neither.
- Tier C does not draw these views. `RenderGraph::new` returns `TierCUnavailable`. `present_plane` returns before the sample walk and before its buffer. `sync_ready_bricks` returns `BrickUploadError::Unavailable` before a cut and before a stamp write. `volume_ray_admission` returns `TierCUnavailable` before an image. `CpuRaster` is not called from these paths. MIP stays uncompiled. `request_outcome` for that key is `Unavailable`.
- The live ledger matches the code. The status table and the backlog both say `done` for the four stories. Each has an AS_BUILT entry and a tracker row. The audit-intake line that the four stories are pending is under the dated open note. It is history, not the status table. The README rows name the level axis, the ready-brick cut, orthographic reslice, progressive sync and the composite ray. Those functions exist. The changelog sentences match the landed refusal and the level-0 upload. `ci/wasm-size-budget.json` still records 2151196 bytes, which is what the carried-in paragraph says.
- No new `as` cast, no new `unsafe`, and no `wasm-bindgen` outside `ocelli-wasm`. The new arithmetic uses `try_from` and the existing bit narrowing. Pixels are not exported across the worker boundary. `ocelli-wasm` and `packages/` do not name these entry points.

## Limitations and handoff

This pass did not re-run `bin/ocelli.sh gate --sprint`, the oracle, wasm, clippy, or any ignored GPU test. The prior sprint-gate receipt belongs to this tree and was not executed again. Row and Column rays, an all-`R16Uint` bind, and a native GL tier B device were not drawn. F-059 pass 3 already recorded those limits. A combined ortho viewport and bound volume ray were not ticked on a device. The single submit is the one call in `encode_frame`, not a measured two-viewport frame.

The temporary probes were deleted. They are not coverage. The permanent guard for the sync key is `equal_brick_bytes_are_not_uploaded_twice`. The permanent ortho fixtures do not append a level, so they do not lock the coarse-byte refusal. The production prefix and `sample_at` do.

The F-057 display string for `BrickNotReady`, and the F-052 early return in the ignored GPU tests, stay carried nitpicks from those story passes. They were not reopened. F-059 pass 3's closed pixel findings were not reopened. The ray still steps one voxel, which is the reviewed F-059 contract, not a millimetre step.

This file was not staged. `scripts/sprint_workflow.py record-sprint-review` was not run. Recording it changes the tree and needs its own verification. This pass does not merge and does not close the sprint.
