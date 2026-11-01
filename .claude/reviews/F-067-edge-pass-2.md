# F-067 review, pass 2

**Reviewed**: unstaged worktree versus HEAD `f0be603b2d0dea29a5eea85d62df963197305b84` on `sprint/s24`. `git diff HEAD` is the review tree. The index was empty and was not used. `rustc 1.97.1 (8bab26f4f 2026-07-14)`.

Paths: `.claude/plans/F-067-design.md`, `crates/ocelli-seg/src/contour.rs`, `crates/ocelli-seg/src/lib.rs`, `crates/ocelli-seg/tests/contour.rs`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/slab.rs`, `crates/ocelli-render/tests/labelmap.rs`, `docs/lld/README.md`, `docs/lld/feature-availability.md`, `docs/lld/render-graph.md`, `docs/sprints/CURRENT_SPRINT.md`.

**Author / reviewer**: the implementation is uncommitted. This reviewer did not write it and did not write pass 1. Pass 1 is `.claude/reviews/F-067-edge-pass-1.md`.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

Pass 1 D1 is the kind sentence. `.claude/plans/F-067-design.md:56` now says one emitted point is `Point`, two or more emitted points are `OpenPlanar` when the contour was not entirely on the plane, and an entirely on-plane contour keeps its original kind and its points. `docs/lld/render-graph.md:51` says the same rule: one emitted point is `Point`, two or more emitted points are open planar when the contour was not entirely on the plane, and a contour already on the plane keeps its kind. Neither sentence says that every off-plane cut is open planar.

`intersect_contour` still matches that rule. Bits decide the on-plane return, which keeps the original kind. One emitted point is `Point` at `crates/ocelli-seg/src/contour.rs:188`. Two or more emitted points are `OpenPlanar`. The AC1 segment that leaves the plane is still one point, and `a_segment_crosses_the_plane_at_its_midpoint` passed with kind `Point`.

Pass 1 D2 is the midpoint-only fixture. The added expectations are derived here.

Rising segment `(0, 0, 0)` to `(10, 4, 4)` at plane 1. `t = (1 - 0) / (4 - 0) = 0.25`. `x = 0.25 * 10 = 2.5`. `y = 0.25 * 4 = 1`. The point is `(2.5, 1, 1)` and the kind is `Point`. A constant midpoint is `(5, 2, 1)`.

Falling segment `(0, 0, 4)` to `(8, 2, 0)` at plane 1. `t = (1 - 4) / (0 - 4) = 0.75`. `x = 0.75 * 8 = 6`. `y = 0.75 * 2 = 1.5`. The point is `(6, 1.5, 1)` and the kind is `Point`. A constant `0.25` is `(2, 0.5, 1)`. A constant `0.5` is `(4, 1, 1)`.

The wrong parameter rejected here is `t` fixed at one half. With `let t = 0.5_f64` in `emit_segment`, `a_cut_off_the_midpoint_uses_t` failed on the rising point. Left was `(5, 2, 1)`. Right was `(2.5, 1, 1)`. The falling assertion was not reached. The division `(plane_z - start.z) / dz` was restored in that same command, before the green suite and before this file. The restored test passed both points.

The edge blend, the compile key, the tier C return, and the contour and surface refusals were re-read. `graph.rs` and `contour.rs` have the same sha256 pass 1 recorded after its own restores, so those branches were not rewritten by the repair. This pass did not replay the fill mutation or the float-equality mutation.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git diff HEAD --stat`, `git status --short` | unstaged tree on `sprint/s24` | Executed now, at start and after the restore | HEAD `f0be603b2d0dea29a5eea85d62df963197305b84`. Index empty. Stat stayed 11 files, 462 insertions, 55 deletions. |
| plan line 56, LLD lines 51 to 53, `intersect_contour` lines 152 to 192 | restored kind branches | Executed now by reading | One emitted point is `Point`. Two or more are `OpenPlanar` only on the cut path. An on-plane contour returns its original kind. |
| `bin/ocelli.sh test ocelli-seg --test contour -- a_cut_off_the_midpoint_uses_t` | `let t = 0.5_f64` in `emit_segment` only | Executed now, then restored before any later command | Exit 101. Failed at `crates/ocelli-seg/tests/contour.rs:234`. Left `(5.0, 2.0, 1.0)`. Right `(2.5, 1.0, 1.0)`. Kind stayed `Point`. Unused `dz` warning. 17 filtered out. |
| sha256, `rg`, and `git diff HEAD` on `contour.rs` | same process, after the failing test | Executed now | sha256 returned to `49a346fa3d78adfdef8eb56884ba1b2fe7b96f6132cc531fc472b3e667253210`. The diff shows `let t = (plane_z - start.z) / dz`. No `0.5_f64` in that file or in `git diff HEAD`. |
| `bin/ocelli.sh test ocelli-seg --test contour` | restored division | Executed now | Exit 0. 18 passed, 0 failed. `a_cut_off_the_midpoint_uses_t` passed, so both the `0.25` point and the `0.75` point matched. |
| sha256 of `graph.rs` | `7c8222725b7384ca8d21ed86a3a27063138446d26bff79a27aa9aa930741406e` | Executed now as a hash compare. Pass 1 recorded this hash after its fill restore | The edge blend and the single `encode_frame` submit are the same bytes. The compile test and the ignored readback were not re-run. |
| `git diff HEAD` names, plus a search for `unsafe` and `wasm_bindgen` | product diff, reviews excluded | Executed now | No shader path. No `unsafe`. No `wasm_bindgen`. |

Author receipts were not used. Cargo ran one process at a time. No sprint gate and no wasm-pack run were started.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 1 D1 is closed. The plan bullet and the LLD sentence name `Point` for one emitted point, `OpenPlanar` for two or more emitted points when the contour was not entirely on the plane, and the original kind when the contour is already on the plane. `intersect_contour` does that. A sentence that calls every off-plane cut open planar is not in those two places.

Pass 1 D2 is closed. `a_cut_off_the_midpoint_uses_t` locks `t = 0.25` at `(2.5, 1, 1)` and `t = 0.75` at `(6, 1.5, 1)`. A constant `0.5` fails the rising assertion. The restored division is the source that the 18-test run passed. The mutation is not left in the tree.

`show_labelmap` still returns `TierCUnavailable` before `grid.width` is read. The blend check is before the representation check. `LabelmapEdgeProjection` passes `LabelStyle { fill: false, outline: true }`. Composite passes the caller style. Any other blend is `BlendUnavailable`. `show_contour` and `show_surface` still require `Composite`, so this blend is `BlendUnavailable` there, before the presenter runs.

`pipeline_compiled` is true for `SegOverlay` plus `LabelmapEdgeProjection` on tier A and tier B, and false for slab, volume, and stack. `compile` pushes that overlay key on the existing `LABELMAP_WGSL` module and the existing labelmap layout. `slab_blend_code` still returns `None` for this blend. `encode_frame` still has one `ctx.queue().submit`, at `crates/ocelli-render/src/graph.rs:3814`. `readback_plane` submits its own copy encoder. That is the test, not the frame. Tick looks up the exact key, so an uncompiled edge key does not alias composite.

On-plane is `z.to_bits()` against `plane_z.to_bits()` at `crates/ocelli-seg/src/contour.rs:153`, `:206`, and `:207`. Duplicate collapse uses bits for x, y, and z. The new source has no float `==` on a coordinate. The side test is `<`. `negative_zero_is_not_the_positive_plane` passed on the restored suite. The `==` mutation from pass 1 was not replayed.

`ocelli-seg` stays `no_std` outside tests, uses `alloc`, and its manifest depends on `ocelli-core` only. The diff has no new trait, no new generic, no `Box<dyn>`, no `unsafe`, and no `wasm_bindgen`. No shader file was added. `present_contour`, `present_labelmap`, `present_surface`, fusion, `voi.wgsl`, `fusion.wgsl`, `raycast.wgsl`, the volume sampler, and the oracle producer are outside `git diff HEAD`. There is no vtk.js volume ray in this diff.

`CURRENT_SPRINT.md` and `docs/lld/feature-availability.md` say the overlay edge key compiles on the existing copy shader and that slab, volume, and stack do not draw it as composite. That matches `pipeline_compiled`, `present_slab`, and the exact key lookup in `tick`. The feature-availability edge row also says `intersect_contour` cuts one contour by a patient Z plane on every tier, and that tier C `show_labelmap` returns before the grid is read. Both match the function, which has no tier gate, and the early return in `show_labelmap`.

## Limitations and handoff

This pass did not run clippy, `prose_check.py`, `bin/ocelli.sh gate`, wasm-pack, the slab suite, or the ignored contour, surface, and labelmap device tests. The signed-zero equality mutation and the caller-fill mutation were not replayed. Their product files match the hashes pass 1 recorded after restore, and the branches were re-read. The falling assertion was not the observed failure while `t` was `0.5`, because the rising assertion failed first. No tier B device was opened.

The temporary binding was restored before the green suite. Product `git diff HEAD --stat` is the same 11-file stat as the start. This file is the only review artefact added by this pass. Pass 1 was not edited. Both prior defects are closed, and this pass found no new defect or smell. That is a bounded review of this worktree. It is not sprint acceptance and it is not a verify receipt.
