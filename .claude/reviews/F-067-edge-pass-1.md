# F-067 review, pass 1

**Reviewed**: unstaged worktree versus HEAD `f0be603b2d0dea29a5eea85d62df963197305b84` (`S24, approve the F-067 design`) on `sprint/s24`. `git diff HEAD` is the review tree. The index was not used. `rustc 1.97.1 (8bab26f4f 2026-07-14)`.

Paths: `.claude/plans/F-067-design.md`, `crates/ocelli-seg/src/contour.rs`, `crates/ocelli-seg/src/lib.rs`, `crates/ocelli-seg/tests/contour.rs`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/slab.rs`, `crates/ocelli-render/tests/labelmap.rs`, `docs/lld/README.md`, `docs/lld/feature-availability.md`, `docs/lld/render-graph.md`, `docs/sprints/CURRENT_SPRINT.md`.

**Author / reviewer**: the implementation is uncommitted. This reviewer did not write it. There is no earlier F-067 microscope pass.
**Result**: 2 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 and AC2 are the published midpoint cuts. The wrong behaviour rejected here is a parameter that is not the plan's `t`.

Hand values that the committed tests expect:

- `(0, 0, 0)` to `(0, 0, 4)` at plane 2 has `t = 2 / 4 = 0.5` and stays at x 0, y 0. The point is `(0, 0, 2)` with kind `Point`. Any other `t` in `(0, 1)` yields the same point because x and y do not change.
- `(0, 0, 0)`, `(2, 0, 2)`, `(4, 0, 0)` at plane 1 has `t = 0.5` on both segments. The points are `(1, 0, 1)` then `(3, 0, 1)`, kind `OpenPlanar`.
- Closed `(0, 0, 0)`, `(2, 0, 0)`, `(1, 1, 2)` at plane 1. The first segment is below the plane and emits nothing. The second has `t = (1 - 0) / (2 - 0) = 0.5`, so x is `2 + 0.5 * (1 - 2) = 1.5` and y is `0.5`. The closing segment has `t = (1 - 2) / (0 - 2) = 0.5`, so the point is `(0.5, 0.5, 1)`. Kind `OpenPlanar`. The committed expectation matches this derivation. It is not a wrong point.
- Open nonplanar `(2, 0, 2)` to `(4, 1, 0)` at plane 1 has `t = 0.5`, so the second point is `(3, 0.5, 1)`. That matches the test.

Those expectations are right, and they are all `t = 0.5`. Replacing the division with `0.5_f64` left all 17 contour tests green. That does not reject a midpoint-only cut.

The discriminating case, derived here and not present in the committed file, is `(0, 0, 0)` to `(10, 4, 4)` at plane 1. `t = (1 - 0) / (4 - 0) = 0.25`, so the point is `(2.5, 1, 1)` and the kind is `Point`. A temporary test against the restored function returned that point. The temporary test was deleted. A constant `0.5` would return `(5, 2, 1)`.

AC4 rejects float equality. Replacing the three on-plane bit comparisons with `==` made `negative_zero_is_not_the_positive_plane` fail at the point assertion. Left was `Ok(Point)` with z `-0.0`. Right was `Err(Empty)`. The rising-segment assertion in that test was not reached.

AC5 rejects a filled centre. The ignored readback passed on the restored tree. Forcing the caller style, with fill true, made the same test fail. The centre texel was `[10, 20, 30, 128]` against `[0, 0, 0, 0]`. The source was restored before anything else ran.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git diff HEAD --stat` | unstaged tree on `sprint/s24` | Executed now, at start and after every restore | HEAD `f0be603b2d0dea29a5eea85d62df963197305b84`. Stat stayed 11 files, 429 insertions, 53 deletions. |
| `bin/ocelli.sh test ocelli-seg --test contour` | restored `intersect_contour` | Executed now | Exit 0. 17 passed, 0 failed. |
| same command after `let t = 0.5_f64` | only that binding changed | Executed now, then restored | Exit 0. 17 passed, 0 failed. Unused `dz` warning. sha256 of `contour.rs` returned to `49a346fa3d78adfdef8eb56884ba1b2fe7b96f6132cc531fc472b3e667253210`. |
| temporary `reviewer_non_midpoint_cut`, then the test file restored | plane 1 through `(0,0,0)` and `(10,4,4)` | Executed now, then deleted | Exit 0. 1 passed. `cmp` against the pre-edit test file matched. |
| `negative_zero` after three `to_bits` comparisons became `==` | point and both segment ends | Executed now, then restored | Exit 101. Failed at `crates/ocelli-seg/tests/contour.rs:277`. `cmp` matched the same sha256. |
| `bin/ocelli.sh test ocelli-render --test labelmap -- seg_overlay_compiles_the_edge_copy` | restored graph | Executed now | Exit 0. 1 passed. |
| ignored `labelmap_edge_uploads_the_outline_and_a_foreign_pass_does_not` | restored `show_labelmap` | Executed now | Exit 0. 1 passed in 0.50s. An adapter ran the readback. |
| same ignored test after `let drawn = style` | edge blend used the caller fill | Executed now, then restored | Exit 101. Failed at `crates/ocelli-render/tests/labelmap.rs:365`. Centre bytes were `[10, 20, 30, 128]`. sha256 of `graph.rs` returned to `7c8222725b7384ca8d21ed86a3a27063138446d26bff79a27aa9aa930741406e`. |

Author receipts were not used as evidence for these rows. No sprint gate and no wasm-pack run were started. Cargo ran one process at a time.

## Defects

### D1, a one-point cut is called open planar

**Where**: `docs/lld/render-graph.md:51` and `.claude/plans/F-067-design.md:56`

**What**: The LLD says a cut that leaves the plane is open planar. The plan bullet says one point is `Point`, then says a cut that is not entirely on the plane is `OpenPlanar`. `intersect_contour` returns `Point` when one point was emitted. AC1 is that case. The segment `(0, 0, 0)` to `(0, 0, 4)` at plane 2 is not entirely on the plane, and the kind is `Point`, not `OpenPlanar`.

**Why it is wrong**: The HLD does not give an intersection kind. The plan's local rule and AC1 are the contract. A one-point result is `Point`. `OpenPlanar` is the kind for two or more emitted points. The LLD sentence has no one-point exception, so it contradicts the function and AC1. The plan states the exception and then states a universal that AC1 falsifies.

**Evidence**: `a_segment_crosses_the_plane_at_its_midpoint` expects `ContourGeometricType::Point` and passed in the 17-test run. The kind selection is `emitted.len() == 1` at `crates/ocelli-seg/src/contour.rs:188`.

**Scope and regression**: prose only. The function already returns `Point`. Present on this worktree, not on the design commit's LLD text, which said labelmap edge was uncompiled. Smallest change is to say one emitted point is `Point` and two or more emitted points are `OpenPlanar`, in the LLD sentence and in the plan bullet. The entirely-on-plane rule stays the original kind.

### D2, every committed crossing uses the midpoint

**Where**: `crates/ocelli-seg/src/contour.rs:227` and the new cases in `crates/ocelli-seg/tests/contour.rs`

**What**: The function computes `t = (plane_z - start.z) / dz`. Every committed crossing that reaches that division has true `t = 0.5`. The vertical AC1 segment does not move x or y, so it accepts every `t` in `(0, 1)`. Setting `t` to `0.5_f64` left the contour suite green.

**Why it is wrong**: The plan's local rule is that division, with `0 < t < 1`, and x, y from `p0 + t * (p1 - p0)`. The current function does that. The suite does not. A later edit that keeps only the midpoint still passes every fixture that this story adds. The crossing then is the wrong patient point whenever the plane is not halfway along the segment. That is the intersection defect this sprint names. A test that passes both ways is not coverage of the formula.

**Evidence**: With `let t = 0.5_f64`, `bin/ocelli.sh test ocelli-seg --test contour` exited 0, 17 passed. The binding was restored and `cmp` matched. Separately, the restored function returned `(2.5, 1, 1)` for `(0, 0, 0)` to `(10, 4, 4)` at plane 1. A constant midpoint on that segment is `(5, 2, 1)`.

**Scope and regression**: `intersect_contour` only. The wrong parameter is not in the tree now. Smallest regression is one open segment whose true `t` is not one half, with the hand point above and kind `Point`. The constant binding must fail that assertion. The on-plane, signed-zero, parallel, duplicate, length, and non-finite rows do not lock `t`.

## Smells

None.

## Nitpicks

None.

## Verified clean

`intersect_contour` rejects a non-finite plane before the points, then a bad geometric length, then a non-finite coordinate. On-plane is `z.to_bits()` against `plane_z.to_bits()`. The restored source has no float `==` on a coordinate. Entirely on-plane returns the same segment, kind, and points. A `Point` off the plane is `Empty`. A closed contour walks `count` segments, including the wrap to index 0, and a cut of two or more points is `OpenPlanar` rather than `ClosedPlanar`. An endpoint on the plane is pushed as that point. A parallel segment off the plane returns without a point. Consecutive duplicates collapse by x, y, and z bits. The closed-cut expectation derived above matches the test. AC4's point assertion goes red if those bit comparisons become `==`.

`show_labelmap` returns `TierCUnavailable` before `grid.width` is read. The blend check is before the representation check. `LabelmapEdgeProjection` passes `LabelStyle { fill: false, outline: true }`. Composite passes the caller style. Any other blend is `BlendUnavailable`. `show_contour` and `show_surface` still require `Composite`, so this blend is `BlendUnavailable` there. The fill mutation above is the wrong style, and the ignored readback went red for it.

`pipeline_compiled` is true for `SegOverlay` plus `LabelmapEdgeProjection` on tier A and tier B in the predicate, and false for slab, volume, and stack. `compile` pushes that overlay key on the existing `LABELMAP_WGSL` module and the existing labelmap layout. No shader file was added. `slab_blend_code` still returns `None` for this blend. `encode_frame` still has one `queue.submit`, at `crates/ocelli-render/src/graph.rs:3814`. `readback_plane` submits its own copy encoder. That is the test, not the frame.

`ocelli-seg` stays `no_std` outside tests, uses `alloc`, and does not depend on wgpu. The diff has no new trait, no new generic, no `Box<dyn>`, no `unsafe`, and no `wasm-bindgen`. `present_contour`, `present_labelmap`, `present_surface`, fusion, `voi.wgsl`, `fusion.wgsl`, `raycast.wgsl`, the volume sampler, and the oracle producer are outside `git diff HEAD`. Surface coverage was not edited.

`CURRENT_SPRINT.md` and `docs/lld/feature-availability.md` say the overlay edge key compiles on the existing copy shader and that slab, volume, and stack do not draw it as composite. That matches `pipeline_compiled`, `present_slab`, and the exact key lookup in `tick`. The feature-availability edge row is about that blend. The composite row still describes the composite upload.

## Limitations and handoff

This pass did not run clippy, `prose_check.py`, `bin/ocelli.sh gate`, wasm-pack, the slab suite, or the ignored contour and surface device tests. `show_contour` and `show_surface` were read. Their edge returns are `BlendUnavailable` before the presenter. The author receipts for those two tests were not replayed.

The signed-zero mutation stopped at the first assertion. The rising segment was not the observed failure. No tier B device was opened. The plan does not claim one. The edge readback did open an adapter on this host.

Temporary edits were restored. Product `git diff HEAD --stat` is the same 11-file stat as the start. This file is the only review artefact added. The two defects are unfixed. The next owner corrects the kind sentences and adds a non-midpoint fixture, then a new independent pass reviews that tree. This pass is not sprint acceptance.
