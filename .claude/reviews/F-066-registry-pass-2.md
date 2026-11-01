# F-066 review, pass 2

**Reviewed**: working tree on `sprint/s23` at `c4431e0adc602592b40178b0f3a513f86c6c355b` (`F-065, surface representation`). The F-066 implementation is uncommitted. Nothing was staged. Nothing was committed. `git rev-parse HEAD` was the same at the start and at the end. `rustc 1.97.1 (8bab26f4f 2026-07-14)`.

Start `git status --short`:

```
 M crates/ocelli-render/src/contour.rs
 M crates/ocelli-render/src/graph.rs
 M crates/ocelli-render/src/labelmap.rs
 M crates/ocelli-render/src/surface.rs
 M crates/ocelli-render/tests/contour.rs
 M crates/ocelli-render/tests/surface.rs
 M crates/ocelli-seg/src/lib.rs
 M docs/sprints/BACKLOG.md
 M docs/sprints/CURRENT_SPRINT.md
?? .claude/reviews/F-066-registry-pass-1.md
?? crates/ocelli-render/tests/registry.rs
```

`git diff --stat` at the start was 9 files, 117 insertions and 15 deletions. The untracked registry test is not in that stat. The same stat was printed after the temporary `&& false` edit was restored. `crates/ocelli-render/src/graph.rs` is `c8dd98d1ba2f76fe95cdca91af5a710b76eca4ee1dc07a516845e02fd4e2f2c4` before the edit and after the restore. That is the hash pass 1 recorded for the restored graph. `cmp` against `/tmp/f066-pass2/graph.rs.pre` was silent. `crates/ocelli-render/tests/registry.rs` is `7dc64734d5da482c8439f67951fc906147b3e8b092512cc5edb413111f0f4b0d` and was not edited. Pass 1 recorded `65c69aaa10ca18091c10c33ae6e5c2dbd012e0d734912f7a0a1a6725cab5f602` for that file, so this pass is looking at the repaired test.

The end status lists pass 1, the registry test, and this pass as untracked. Product paths match the start.

**Author / reviewer**: this reviewer did not write the registry, the repair, or pass 1.
**Result**: 0 defects, 0 smells, 0 nitpicks

Pass 1 S1 is closed. No new defect was found.

## Acceptance challenged

S1 is the contour comparison. The approved plan says `show_contour` requires `Contour` and a mismatch returns before the presenter. A new composite overlay stays `Labelmap`. The open line is spacing 1, origin 0, and points `(0, 0, 0)` and `(3, 0, 0)`, on a 4 by 2 view. `present_contour` of that line returns `Ok`, so a comparison that does not refuse makes `show_contour` return `Ok(())`.

The review tree has no `&& false`. The comparison at `graph.rs:3390` is a plain inequality. The same filter passed on those bytes: 1 passed, 7 filtered out, 0.22 seconds. Adding `&& false` made `show_contour_respects_the_default` fail at `crates/ocelli-render/tests/registry.rs:461`. Left was `Ok(())`. Right was `Err(RepresentationMismatch)`. The presenter ran and the upload returned. The assertion that the new overlay is `Labelmap` is at line 459, and the panic is at line 461, so that assert had already passed. The source was restored before this report. `diff` against the backup was that one token.

The `submissions` asserts sit after line 461, so this failure did not reach them. `stage` uses `write_texture` and does not increment `submissions`. The error line is the guard that went red.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff --stat` | `sprint/s23` working tree, start and after restore | Executed now | HEAD `c4431e0adc602592b40178b0f3a513f86c6c355b` both times. Tracked diff stayed 9 files, 117 insertions, 15 deletions. |
| sha256 and `cmp` of `crates/ocelli-render/src/graph.rs` | pre-edit copy `/tmp/f066-pass2/graph.rs.pre` | Executed now | `c8dd98d1ba2f76fe95cdca91af5a710b76eca4ee1dc07a516845e02fd4e2f2c4` before the edit and after `cp`. `cmp` matched. Same hash pass 1 recorded after its restore. |
| sha256 and `cmp` of `crates/ocelli-render/tests/registry.rs` | pre-edit copy `/tmp/f066-pass2/registry.rs.pre` | Executed now | `7dc64734d5da482c8439f67951fc906147b3e8b092512cc5edb413111f0f4b0d` before the edit and after the restore. `cmp` matched. The file was not part of the mutation. |
| `bin/ocelli.sh test ocelli-render --test registry -- --ignored --test-threads=1 show_contour_respects_the_default` | restored comparison, before the mutation | Executed now | Exit 0. 1 passed, 0 failed, 7 filtered out, 0.22 seconds. |
| `&& false` added to the contour comparison, then the same filter | one token at `graph.rs:3390`. `diff` against the backup showed only that token. | Executed now, then restored | Exit 101. Failed at `tests/registry.rs:461`. Left `Ok(())`. Right `Err(RepresentationMismatch)`. 7 filtered out. 0.32 seconds. |
| `rg` for `&& false` and for `queue().submit` in `graph.rs` | restored tree | Executed now | No `&& false`. One `queue().submit`, at `graph.rs:3778`. |
| `git status --short -- crates/ocelli-render/shaders` and untracked `*.wgsl` | shader directory | Executed now | No output. No new WGSL file. |
| full ignored registry suite, labelmap, contour, and surface GPU tests | not this pass | Prior receipt, pass 1 | Pass 1 ran the seven-test registry file green, then saw the labelmap deletion go red. This pass did not repeat those suites. |

## Defects

None.

Pass 1 S1 is closed. `show_contour` at `crates/ocelli-render/src/graph.rs:3390` still returns `ContourDrawError::RepresentationMismatch` when the stored choice is not `Contour`. `show_contour_respects_the_default` reaches that return on a new composite overlay and does not call `set_representation`. Disabling the comparison made the open-line show return `Ok(())`. The test does not stay green both ways.

## Smells

None.

## Nitpicks

None.

## Verified clean

The repair under review is `show_contour_respects_the_default`. `graph.rs` is byte-identical to the tree pass 1 restored. The new test is the only product change since that hash.

The test builds a composite `SegOverlay`, reads `Labelmap`, calls `show_contour` with the open line from `the_matching_show_uploads`, expects `ContourDrawError::RepresentationMismatch`, and expects `submissions` to stay put across that call and the following tick. The green run passed. The `&& false` run failed on the error, which is the result pass 1 said a deleted check would produce. Restore was a byte copy.

`show_labelmap` still compares with `Labelmap` at `graph.rs:3336` before either `present_labelmap` call. The empty-style call is at line 3345. The filled call is at line 3355. `switching_drops_the_overlay_without_upload` still shows a 1 by 1 grid, switches to `Contour`, then calls `show_labelmap` with index 2. The LUT has one colour, so segment 2 is `MissingColour` if the presenter runs. The test still expects `RepresentationMismatch`. This pass did not delete that comparison.

`show_surface` still compares with `Surface` at `graph.rs:3444` before `present_surface` at line 3462. `a_surface_show_respects_the_registry` still sets `Contour` and shows a mesh whose triangle is `[9, 1, 2]` over three vertices. `require_index` returns `SurfaceError::Index` before a write if that presenter runs. The test still expects `SurfaceDrawError::RepresentationMismatch`. This pass did not delete that comparison.

`register` still writes `Labelmap` at `graph.rs:1078`. The failing run had already accepted that readback. There is no `Default` impl on the enum. `ocelli_seg::Representation` is still `Labelmap`, `Contour`, `Surface`, with `Copy`, `PartialEq`, and `Eq`, in a crate that stays `no_std` outside tests.

The same setter value takes the `false` branch at `graph.rs:3286`. It does not set segmentation dirt and does not clear `slot.labelmap`. The drop is guarded by `changed`. `the_same_choice_keeps_the_binding` still expects `Dirty::default()` and a `Noop` tick. That test was not re-run.

`install_recovered` still sets every GPU slot to `None` and every registry slot to `Dirty::full()`. It does not assign `representation`. `recovery_keeps_the_choice_and_drops_the_texture` still expects `Contour` after that call. That test was not re-run.

`encode_frame` still has one `ctx.queue().submit`, at `graph.rs:3778`. The zero-encode return above it does not submit. `present` after the submit is not a second submit. The diff adds no `include_str!`. The shader directory is still `fusion.wgsl`, `labelmap.wgsl`, `raycast.wgsl`, `slab.wgsl`, and `voi.wgsl`.

`resize_replaces_the_target_and_two_viewports_submit_once` is still at `crates/ocelli-render/tests/surface.rs:166`. The surface diff does not touch that function. It still expects one submission after two viewports. The added lines are the `Representation` import and the setter in `shown_surface_matches_the_presenter`. The contour test adds the setter the same way, after the edge show that expects `BlendUnavailable`. That edge return is still the blend check, which sits before the representation check. `crates/ocelli-render/tests/labelmap.rs` still has no `set_representation` call.

The diff adds no `as` cast, no `unsafe`, no `wasm_bindgen`, and no new trait. The registry test's only use of the word `as` is in a comment. The gpu gate runs every ignored `ocelli-render` test and names no file, so this ignored test is in that set. The gate was not run.

`docs/sprints/BACKLOG.md` and the S23 table say `in-progress`. The closing note says F-066 is in progress. The F-X003 backlog row is still `in-progress`. That matches an uncommitted story.

## Limitations and handoff

The other seven registry tests were filtered out. Labelmap, contour, and surface GPU tests were not run. The labelmap and surface comparisons were read and were not mutated again. Tier B was not separated from tier A. The disabled check returned `Ok(())` rather than `TierCUnavailable`, so this run was not tier C. A machine with no adapter returns from `context()` before the assertion and the ignored test passes. That did not happen here.

The same-value test still does not read the texture back. The drop stays in the branch that sets dirt. The new test's `submissions` lines were not the lines that failed. `bin/ocelli.sh wasm`, clippy, the feature floor, the oracle, and the corpus were not run. `docs/lld/render-graph.md` and `docs/lld/feature-availability.md` are unchanged.

This pass does not record a sprint review and does not start F-067. The contour mismatch guard is clean on the checks above.
