# F-065 review, pass 2

**Reviewed**: working tree on `sprint/s23` at `130e92422bd265f8df6bbd4a18cce5728a67987c` (`F-064, contour representation`). F-065 is uncommitted. Nothing was staged. Nothing was committed. `git rev-parse HEAD` was the same at the start and at the end. `rustc 1.97.1 (8bab26f4f 2026-07-14)`.

Start `git status --short`:

```
 M crates/ocelli-render/src/graph.rs
 M crates/ocelli-render/src/lib.rs
 M crates/ocelli-render/tests/surface.rs
 M crates/ocelli-seg/src/lib.rs
 M docs/sprints/BACKLOG.md
 M docs/sprints/CURRENT_SPRINT.md
?? .claude/reviews/F-065-surface-pass-1.md
?? crates/ocelli-render/src/surface.rs
?? crates/ocelli-seg/src/surface.rs
?? crates/ocelli-seg/tests/surface.rs
```

`git diff --stat` at the start was 6 files, 382 insertions and 8 deletions. The same stat was printed after the temporary `>` edit was restored. `crates/ocelli-seg/src/surface.rs` is `2f517689710b1ee32271993c885d36997f6a3331487a105e00fce844b31ddc91` before the mutation and after `cmp` against `/tmp/f065-pass2-surface.rs.pre`. The strict comparison is not in the tree.

**Author / reviewer**: this reviewer did not write the surface code, the pass 1 repair, or pass 1.
**Result**: 0 defects, 0 smells, 0 nitpicks

Pass 1 D1 is closed. Pass 1 D2 is closed. No new defect was found.

## Acceptance challenged

AC1 is the coverage claim. The approved plan includes a pixel when every barycentric weight of its centre is `>= 0`. The HLD does not define the raster. For triangle `(0,0) (4,0) (0,4)` the independent half-plane is `px >= 0`, `py >= 0` and `px + py <= 4`, with `px = c + 0.5` and `py = r + 0.5`. That is the same set as weights with denominator 16. The four named centres are `(0.5, 0.5) = 0.75, 0.125, 0.125`, `(4.5, 0.5) = -0.25, 1.125, 0.125`, `(0.5, 3.5) = 0.0, 0.125, 0.875` and `(3.5, 1.5) = -0.25, 0.875, 0.375`. Covered centres are `(0,0) (1,0) (2,0) (3,0) (0,1) (1,1) (2,1) (0,2) (1,2) (0,3)`.

`/tmp/f065_pass2_probe` called the real `present_surface`. All 25 bytes matched that set. The clockwise mesh matched those bytes. A triangle at `(0,0) (3.9,0) (0,3.9)` left `(3,0)`, `(0,3)`, `(2,1)` and `(1,2)` clear and painted `(0,0)`, `(2,0)` and `(1,1)`. Rounding `3.9` to `4` would paint those four clear pixels, because they sit on the included edge of the integer triangle. World vertices `(10,20,5) (18,20,5) (10,28,5)` with origin `(10,20,5)` and spacing 2 matched the unit image. Treating millimetres as pixels would not cover a 5 by 5 grid.

AC2 uses the existing contour stroke. An independent Bresenham of the three closed edges matched `present_contour` on every pixel. Contour-only pixels are `(4,0) (0,4) (3,1) (2,2) (1,3)`. Surface-only pixels are `(1,1) (2,1) (1,2)`. The required disagreements hold on the real presenters.

AC3 and AC4 were the same probe. A zero-area triangle is `Empty` and leaves `0xA5`. A cross product whose bits are `8000000000000000`, which is negative zero, is skipped, and the following hand triangle matches the unit image. Negative-zero `z` against positive-zero origin is `OutOfPlane` and writes nothing, even though the two values compare equal as floats. Both vertices at negative zero are drawn. A valid triangle followed by index 9 is `Index`, writes nothing, and a later valid present on that buffer matches the hand image. An unused vertex at `z = 1` refuses the mesh. Segment 0 is `MissingColour`. Segment 2 uses `(11, 22, 33, 200)`, not the first LUT entry. Negative spacing and a spacing of `1e-320` are `Frame` and write nothing.

The `>=` comparisons were changed to `>` and `triangle_covers_centres_inside_the_edge` failed. The covered list reaches `(3,0)` first among the zero-weight centres. `(0,3)` is the same class and was not reached. The source was restored before the final suites.

AC5 and AC6 ran in the one ignored surface binary. `shown_surface_matches_the_presenter` compares the readback with `present_surface`, checks the equal-byte tick, and checks recovery. The resize test is the HEAD body.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff --stat` | `sprint/s23` working tree, start and after restore | Executed now | HEAD `130e92422bd265f8df6bbd4a18cce5728a67987c` both times. Tracked diff stayed 6 files, 382 insertions, 8 deletions. |
| Function extract of `resize_replaces_the_target_and_two_viewports_submit_once` and its seven helpers | HEAD `crates/ocelli-render/tests/surface.rs` against the working tree | Executed now | Bodies identical, 109 lines for the test and the same helper lengths. The only preamble diff is the plural GPU comment and the imports the upload test uses. |
| `/tmp/f065_pass2_probe` | real `present_surface` and `present_contour` | Executed now | Exit 0. `probe_failures=0`. `weight_cover=10`. `stroke_mismatch=0`. Negative-zero area bits `8000000000000000`. Negative-zero `z` equals `0.0` and the bits differ. |
| `>= 0.0` changed to `> 0.0`, then `triangle_covers_centres_inside_the_edge` | one expression in `surface.rs`. Three comparisons. | Executed now, then restored | Exit 101. Failed at `crates/ocelli-seg/tests/surface.rs:80`. Left `Some([0, 0, 0, 0])`. Right `Some([10, 20, 30, 128])`. `cmp` restore was silent. Hash matched. |
| `bin/ocelli.sh test ocelli-seg --test surface` | restored hash above | Executed now | Exit 0. 5 passed, 0 failed. |
| `bin/ocelli.sh clippy ocelli-seg` and `bin/ocelli.sh clippy ocelli-render` | restored tree, `-D warnings`, `--all-targets` | Executed now | Exit 0 and exit 0. |
| `bin/ocelli.sh test ocelli-render --test surface -- --ignored --test-threads=1` | both ignored tests in one process | Executed now | Exit 0. `resize_replaces_the_target_and_two_viewports_submit_once` ok. `shown_surface_matches_the_presenter` ok. 2 passed, 0 failed, finished in 0.39s. No second GPU suite. |

## Defects

None.

Pass 1 D1 is closed. `resize_replaces_the_target_and_two_viewports_submit_once` is back in `crates/ocelli-render/tests/surface.rs` at line 166. The function body matches HEAD, including `opened_from_this_instance`, the paint of the 4-wide target with `0x11`, the 8-wide readback, and `fresh.get(31)`. `native_clock`, `session`, `stack_pass`, `color_target`, `source_view`, `paint` and `readback` match HEAD as well. `shown_surface_matches_the_presenter` is in the same file at line 445. The serial ignored run executed both tests and both passed.

Pass 1 D2 is closed. `docs/sprints/CURRENT_SPRINT.md` line 11 says F-065 is `in-progress` and line 12 says F-066 is `pending`. Line 10 says F-064 is `done`. The audit sentence says F-065 is in progress and F-066 stays pending, and it still says F-064 is done and F-X003 stays in progress. `docs/sprints/BACKLOG.md` has the same four cells: F-063 done, F-064 done, F-065 in-progress, F-066 pending. F-X003 remains in-progress. F-065 is not marked done.

## Smells

None.

## Nitpicks

None.

## Verified clean

The repair under review is the restored F-039 test and the audit sentence. The presenter bytes are the pass 1 hash. `present_surface` still takes width and height as separate `u32` arguments. `show_surface` takes `SurfaceView` for those two fields. The method has seven parameters counting `self`. Clippy with `-D warnings` accepted that signature.

Coverage is the three weights of centre `(c + 0.5, r + 0.5)`, included at `>= 0` on `crates/ocelli-seg/src/surface.rs:187`. There is no `as` cast in that presenter or in `crates/ocelli-render/src/surface.rs`. There is no float `==`. A zero cross product is a bit test for `0` and for `-0.0`. `z` uses `to_bits`. Vertices are not rounded. The 3.9 probe is the rounding rejection. The zero-weight edge is locked by the mutation, which went red and was not a green-both-ways guard.

`show_surface` returns `TierCUnavailable` at `crates/ocelli-render/src/graph.rs:3363` before any read of the mesh. `present_surface` is the next use, at line 3391, and `store_overlay` is line 3393. The `?` on the presenter error returns before the upload. Equal bytes return inside `store_overlay` at line 3412, before `labelmap::stage`. The ignored test then sees default dirt, a `Noop` tick and an unchanged submission count.

`encode_frame` has one `queue().submit`, at `crates/ocelli-render/src/graph.rs:3707`. `show_surface` does not submit. The labelmap fragment target uses `blend: None` at line 1154, so a matching readback is the presenter texel. There is no new WGSL file. `git diff` does not name a shader. `crates/ocelli-seg/src/contour.rs` matches HEAD at `e54a9cb66bf462fa699a10c02404903e916d23138406be056a1646f6b88f8275`.

No new trait, generic or `Box<dyn>`. No `unsafe`. No `wasm-bindgen`. No marching cubes. F-066 registry code is not in this diff. The backlog and the sprint table do not say F-065 is done.

## Limitations and handoff

`docs/lld/render-graph.md`, `docs/lld/feature-availability.md` and `docs/lld/README.md` are unchanged. The plan names them under LLD impact. They are still completion work while F-065 stays in progress. The current sentences do not deny a surface presenter. They also do not describe this one.

Tier C `show_surface` was not executed. `RenderGraph::new` refuses tier C before a graph exists. The return is the first statement of `show_surface` and does not read the mesh. This host's ignored run did not print a tier. Pass 1 on this host opened tier A. The resize test's only success return before an assertion is a missing adapter, and the upload test can return before its readback if registration fails. Both tests passed in 0.39s. The CPU surface suite on this tree finished in 0.00s. That is the device evidence for this pass. Tier B was not a separate device. It uses the same fragment module and `blend: None`.

`SurfaceDrawError::from_labelmap` still maps `LabelmapDrawError::Present` to `Extent`. `store_overlay` does not return `Present`. The arm stays unreachable, same as the contour upload. That is not a smell.

`bin/ocelli.sh wasm` was not run. The oracle and the corpus were not run. This pass does not record sprint verification and does not start F-066.
