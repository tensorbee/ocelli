# S24 sprint review, pass 1

**Reviewed**: `8f2d1797f2e91f08a49e7958de4ad17a5de10255..486be24e6ece9b09315e8d0da953ebdf74364b8c` on `sprint/s24`. `git rev-parse HEAD` returned `486be24e6ece9b09315e8d0da953ebdf74364b8c`. The commit message is `F-067, segmentation intersection and labelmap edge blend`. `git rev-parse HEAD^{tree}` is `246c4be008b8c669962a4c5d91bfa367e30665ef`. `8f2d179` is an ancestor and is tag `s23`. Commits in range are `74e7f6e` (open), `f0be603` (approved design) and `486be24` (F-067). The product tree was clean at the start. Temporary edits below were restored. `git diff` was empty after each restore and at the end. This file is the only new path. `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
**Author / reviewer**: independent reviewer. This pass did not write F-067, the design, or `.claude/reviews/F-067-edge-pass-1.md` and `F-067-edge-pass-2.md`. The feature trailer says `Ocelli-Generated-By: grok`. A trailer does not identify the author.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

The cross-story claim under test was the patient cut, not a midpoint. Segment `(0, 0, 0)` to `(10, 4, 4)` at plane 1 has `t = (1 - 0) / (4 - 0) = 0.25`, so the point is `(2.5, 1, 1)` and the kind is `Point`. A constant `0.5` is `(5, 2, 1)`. Setting `let t = 0.5_f64` made `a_cut_off_the_midpoint_uses_t` fail at `crates/ocelli-seg/tests/contour.rs:234`. Left was `(5.0, 2.0, 1.0)`. Right was `(2.5, 1.0, 1.0)`. Kind stayed `Point`. The falling assertion in that test was not reached. The binding was restored to `(plane_z - start.z) / dz` in the same command. The restored suite then passed both points.

The closed cut in `a_closed_contour_cut_stays_open` was derived here. Points `(0, 0, 0)`, `(2, 0, 0)`, `(1, 1, 2)` at plane 1. The first segment stays below the plane. The second has `t = 0.5`, so the point is `(1.5, 0.5, 1)`. The wrap has `t = (1 - 2) / (0 - 2) = 0.5`, so the point is `(0.5, 0.5, 1)`. Kind is `OpenPlanar`, not `ClosedPlanar`. That test passed.

On-plane is bits, not `==`. Replacing the three on-plane `to_bits` comparisons in `intersect_contour` and `emit_segment` with `==` made `negative_zero_is_not_the_positive_plane` fail at `contour.rs:307`. Left was `Ok(Point)` with z `-0.0`. Right was `Err(Empty)`. The rising segment in that test was not reached. Those three comparisons were restored. Duplicate collapse and `present_contour` still use `to_bits`.

The edge blend was challenged by forcing the caller style. `let drawn = style` made `labelmap_edge_uploads_the_outline_and_a_foreign_pass_does_not` fail at `labelmap.rs:365`. The centre texel was `[10, 20, 30, 128]` against `[0, 0, 0, 0]`. The boundary stayed `[10, 20, 30, 255]`. The `if edge` style was restored. The same ignored test then passed in 0.52s, so an adapter ran the readback. `graph.submissions()` is asserted equal to 1 after that readback.

`labelmap_edge_is_not_drawn_as_composite` passed. The slab edge key is uncompiled, `request_outcome` is `Unavailable`, and `present_slab` returns `BlendUnavailable` before the upload closure.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git log --oneline 8f2d179..HEAD`, `git status --porcelain` | branch `sprint/s24` | Executed now | HEAD `486be24e6ece9b09315e8d0da953ebdf74364b8c`. Tree `246c4be008b8c669962a4c5d91bfa367e30665ef`. Three commits. Status empty at start and after every restore |
| `bin/ocelli.sh test ocelli-seg --test contour -- a_cut_off_the_midpoint_uses_t` | `let t = 0.5_f64` only | Executed now, then restored | Exit 101. Left `(5.0, 2.0, 1.0)`. Right `(2.5, 1.0, 1.0)`. Unused `dz` warning. 17 filtered out |
| `bin/ocelli.sh test ocelli-seg --test contour` | restored division | Executed now | Exit 0. 18 passed, 0 failed |
| `bin/ocelli.sh test ocelli-seg --test contour -- negative_zero_is_not_the_positive_plane` | three on-plane comparisons changed to `==` | Executed now, then restored | Exit 101. Left `Ok(Point)` at z `-0.0`. Right `Err(Empty)`. 17 filtered out |
| same filter on the restored file | `to_bits` back at lines 153, 206 and 207 | Executed now | Exit 0. 1 passed. 17 filtered out |
| `bin/ocelli.sh test ocelli-render --test labelmap -- seg_overlay_compiles_the_edge_copy` | restored `pipeline_compiled` | Executed now | Exit 0. 1 passed. 3 filtered. No device |
| ignored `labelmap_edge_uploads_the_outline_and_a_foreign_pass_does_not --test-threads 1` | restored `show_labelmap` | Executed now, before the fill edit and again after restore | Exit 0 both times. 1 passed in 0.32s, then 0.52s. 3 filtered |
| same ignored test after `let drawn = style` | caller fill and outline | Executed now, then restored | Exit 101. Failed at `labelmap.rs:365`. Centre `[10, 20, 30, 128]` against `[0, 0, 0, 0]` |
| `bin/ocelli.sh test ocelli-render --test slab -- labelmap_edge_is_not_drawn_as_composite` | restored graph, S23 consumer | Executed now | Exit 0. 1 passed. 6 filtered. No device |
| `git diff -- crates/ocelli-seg/src/contour.rs crates/ocelli-render/src/graph.rs` and `rg '0.5_f64\|let drawn = style'` | after each restore | Executed now | Diff empty. Neither leftover string is present |
| Feature trailer on `486be24` | profile `feature`, corpus `pass`, tree prefix `246c4be008b8` | Prior receipt, not re-run | Gate list has no `gpu`. It matches floor plus `corpus` |
| S23 sprint trailer on `8f2d179` | profile `sprint`, 31 gates, tree `97f98dab6687`, comparison verdict `pass` | Prior receipt, not re-run | Matches the audit facts in `CURRENT_SPRINT.md` except the F-067 status sentence |

Cargo ran one process at a time. No sprint gate and no wasm-pack run were started.

## Defects

### D1, completion prose still says F-067 is not done

**Where**: `docs/sprints/CURRENT_SPRINT.md:140` and `docs/sprints/CURRENT_SPRINT.md:69`

**What**: The audit intake says `F-067 is pending`. The carried-in bullet says `F-067 is the remaining M6 story`. The status table on line 10 says `done`. `docs/sprints/BACKLOG.md` row F-067 says `done`. `SPRINT_TRACKER.md` has the F-067 row. The open-to-HEAD diff of this file updates the status cell and the blend paragraph. It leaves both unfinished-story sentences as they were at `74e7f6e`.

**Why it is wrong**: A false sentence in the sprint record blocks completion. The same file says the backlog is the authority. S23's audit intake was updated to `is done` for F-064, F-065 and F-066 once those rows were done. M6 is not closed by this commit, and the sentences that say M6 is not complete stay true. That does not keep F-067 pending or remaining.

**Evidence**: `grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S24 / {print $2, $9}'` printed `F-067` and `done`. The five M6 rows F-063 through F-067 are all `done`. Line 140 still says pending.

**Scope and regression**: prose only. Present on HEAD. The intersection and the edge blend are not wrong. Smallest change is to say F-067 is done in both sentences. Do not mark M6 closed and do not publish a release. The workspace version stays `0.1.0`.

## Smells

None.

## Nitpicks

None.

## Verified clean

`intersect_contour` refuses a non-finite plane before the points, then a bad geometric length, then a non-finite coordinate. On-plane is `z.to_bits()` against `plane_z.to_bits()` at `crates/ocelli-seg/src/contour.rs:153`, `:206` and `:207`. An on-plane contour returns the same segment, kind and points. A `Point` off the plane is `Empty`. One emitted point is `Point` at line 188. Two or more emitted points are `OpenPlanar`. A closed contour walks one segment per vertex, including the wrap to index 0, and the cut is not given `ClosedPlanar`. An endpoint on the plane is pushed as that point. A parallel segment off the plane adds nothing. Consecutive duplicates collapse by x, y and z bits. The crossing stores `plane_z` and uses `t = (plane_z - start.z) / dz` with `0 < t < 1`. The eleven new contour tests are the midpoint cut, the `t = 0.25` and `t = 0.75` cut, the two-cross polyline, the on-plane closed kind, signed zero, the closed cut, non-finite input, the parallel miss, the shared vertex, the open-nonplanar cut, and the short closed contour. The restored file passed all 18 contour tests.

`show_labelmap` returns `TierCUnavailable` before `grid.width` is read. The blend check is before the representation check. `LabelmapEdgeProjection` passes `LabelStyle { fill: false, outline: true }`. Composite passes the caller style. Any other blend is `BlendUnavailable`. `show_contour` and `show_surface` still require `Composite`, so this blend is `BlendUnavailable` before the presenter. The device tests in `crates/ocelli-render/tests/contour.rs` and `surface.rs` still expect that error. They were read and not re-run. The fill mutation above is the wrong style, and the ignored readback went red for it. The restored readback matched outline-only `present_labelmap`, with boundary `(10, 20, 30, 255)` and a transparent centre, and `submissions` stayed 1.

`pipeline_compiled` is true for `SegOverlay` plus `LabelmapEdgeProjection` and false for slab, volume and stack. `compile` pushes that overlay key on the existing `LABELMAP_WGSL` module and the existing labelmap layout. `slab_blend_code` returns `None` for this blend. `present_slab` returns `BlendUnavailable` before the image is read. `encode_frame` has one `ctx.queue().submit`, at `graph.rs:3814`. Tick looks up the exact key, so an uncompiled edge key does not alias composite. `readback_plane` submits its own copy. That is the test, not the frame.

`ocelli-seg` stays `no_std` outside tests, uses `alloc`, and its manifest depends on `ocelli-core` only. The product diff has no new trait, no new generic, no `Box<dyn>`, no `unsafe`, and no `wasm-bindgen`. No shader file was added. `voi.wgsl`, `fusion.wgsl`, `raycast.wgsl`, the volume sampler and the oracle producer are outside the diff. There is no vtk.js volume ray. Changed Markdown outside tables and code fences has no em dash and no prose semicolon.

`BACKLOG.md` F-067 is `done` and depends on F-064. The tracker row says `not measured` and does not invent a duration. `CHANGELOG.md`, `AS_BUILT.md`, `docs/lld/render-graph.md` and `docs/lld/feature-availability.md` match the cut and the edge blend, including one emitted point as `Point` and two or more as open planar only when the contour was not already on the plane. The workspace `version` is still `0.1.0`. `ci/wasm-size-budget.json` was not edited. M6 is not closed. The feature trailer is profile `feature` with corpus `pass` and tree prefix `246c4be008b8`.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate --sprint`, `gate --floor`, `gate gpu`, clippy, `prose_check.py`, or wasm-pack. The feature and S23 sprint trailers were read and not re-executed. The ignored contour and surface device tests were not run. `RenderGraph::new` returns before a tier C graph exists, so the `show_labelmap` tier return was read and not called. The resolved tier of the edge readback was not printed. A separate tier B device was not opened. Infinity was not a separate fixture. `is_finite` is the refusal for a non-finite plane. The falling `t = 0.75` assertion and the signed-zero rising segment were not the observed failures, because the first assertion in each test failed first. The S23 size 2,171,060 bytes was not measured again.

The restored product tree matches HEAD `486be24e6ece9b09315e8d0da953ebdf74364b8c`. Nothing was staged, committed or pushed. This file is not a sprint-profile record. D1 is unfixed. The next owner corrects the two F-067 status sentences without closing M6, then a new independent pass reviews that tree. This pass is not sprint acceptance.
