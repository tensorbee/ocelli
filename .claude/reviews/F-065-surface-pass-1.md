# F-065 review, pass 1

**Reviewed**: working tree on `sprint/s23` at `130e92422bd265f8df6bbd4a18cce5728a67987c` (`F-064, contour representation`). F-065 is uncommitted. Nothing was staged. Nothing was committed. `git rev-parse HEAD` was the same at the start and at the end. `rustc 1.97.1 (8bab26f4f 2026-07-14)`.

Tracked diff at the start and after the temporary edge edit was restored: 6 files, 245 insertions, 171 deletions. Untracked product files are `crates/ocelli-render/src/surface.rs`, `crates/ocelli-seg/src/surface.rs`, and `crates/ocelli-seg/tests/surface.rs`. `git diff` does not name `crates/ocelli-seg/src/contour.rs` or any shader. `crates/ocelli-seg/src/surface.rs` is `2f517689710b1ee32271993c885d36997f6a3331487a105e00fce844b31ddc91` before the mutation and after `cmp` against `/tmp/f065-surface.rs.pre`.

**Author / reviewer**: the F-065 diff has no commit, so the author is not established from this tree. This reviewer did not write the surface code and did not resume an earlier reviewer.
**Result**: 2 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The approved plan is the contract. The HLD does not define a mesh or a coverage rule. Phase 3 surface rendering was not reviewed as this story.

AC1. Independent barycentric weights for triangle `(0,0) (4,0) (0,4)`, spacing 1, origin 0. Denominator 16. Centre `(0.5, 0.5)` is `0.75, 0.125, 0.125`. Centre `(4.5, 0.5)` is `-0.25, 1.125, 0.125`. Centre `(0.5, 3.5)` is `0.0, 0.125, 0.875`. Centre `(3.5, 1.5)` is `-0.25, 0.875, 0.375`. Covered centres on the 5 by 5 grid are exactly `(0,0) (1,0) (2,0) (3,0) (0,1) (1,1) (2,1) (0,2) (1,2) (0,3)`. That is the test list. The named clear pixels `(4,0) (0,4) (3,1) (2,2) (4,4)` are outside it. A probe through `present_surface` matched all 25 pixels, including the ten clear pixels the test does not name. A clockwise mesh through the same presenter matched those bytes. This rejects rounding the vertices onto pixels, which would paint `(4,0)`, and it rejects an edge test that uses `> 0`.

AC2. Bresenham, using the stroke in `contour.rs` and not a new implementation of it, paints `(0,0) (1,0) (2,0) (3,0) (4,0) (0,1) (3,1) (0,2) (2,2) (0,3) (1,3) (0,4)`. `present_contour` returned that set. Contour-only pixels are `(4,0) (0,4) (3,1) (2,2) (1,3)`. Surface-only pixels are `(1,1) (2,1) (1,2)`. The required disagreements hold on the real presenters.

AC3. A zero-area triangle returns `Empty` and leaves `0xA5`. A degenerate triangle before the hand triangle still paints `(1,1)`. A triangle whose cross product is negative zero, followed by the hand triangle, matches the hand image. One degenerate triangle does not refuse the mesh.

AC4. A bad index, a `z` bit mismatch, a missing colour, a short buffer, and a non-positive spacing leave the destination unchanged. Negative zero `z` against positive zero origin is `OutOfPlane` and writes nothing. A valid triangle followed by index 9 returns `Index` and writes nothing. An unused vertex off the plane also refuses the whole mesh, which is what the plan's "a vertex" sentence says.

AC5. On this host's tier A device, `show_surface` then `tick` read back bytes equal to `present_surface`. Pixel `(0,0)` is `(10, 20, 30, 128)`. Pixel `(4,0)` is transparent. The copy is not the contour stroke.

AC6. The ignored test asserts `NotRebound` before rebind, `Unavailable` until the next show, and a second readback equal to the presenter. The early returns in that test sit before the composite registration. A probe completed that registration on this host, and the ignored test passed, so those recovery asserts ran.

`show_surface` calls `present_surface` and then `labelmap::stage`. There is no new WGSL file. `contour.rs` is not in the diff.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff --stat` | `sprint/s23` working tree, start and after restore | Executed now | HEAD `130e92422bd265f8df6bbd4a18cce5728a67987c` both times. Tracked diff stayed 6 files, 245 insertions, 171 deletions. |
| Independent weights and Bresenham | Plan triangle, spacing 1, origin 0 | Executed now | Centres match the plan. Covered set matches the fixture list. Contour-only set is `(4,0) (0,4) (3,1) (2,2) (1,3)`. |
| `/tmp/f065_probe` `present_surface` and `present_contour` | Real `ocelli-seg` presenters | Executed now | Exit 0. `probe_failures=0`. All 25 hand pixels match. Clockwise matches those bytes. Origin `(10,20,5)` and spacing 2 match the unit image. Negative-zero area is skipped. Negative-zero `z`, a later bad index, and an unused off-plane vertex write nothing. |
| `bin/ocelli.sh test ocelli-seg --test surface` | Restored presenter | Executed now | Exit 0. 5 passed, 0 failed. |
| `>= 0.0` changed to `> 0.0`, then `triangle_covers_centres_inside_the_edge` | One expression in `surface.rs`. Three comparisons. | Executed now, then restored | Exit 101. Failed at `crates/ocelli-seg/tests/surface.rs:80`. Left `Some([0, 0, 0, 0])`. Right `Some([10, 20, 30, 128])`. First zero-weight centre in that loop is `(3,0)`. `(0,3)` is the same class and was not reached. `cmp` restore was silent. Hash matched. |
| `bin/ocelli.sh test ocelli-seg --test surface` | Restored hash above | Executed now | Exit 0. 5 passed, 0 failed. |
| `bin/ocelli.sh clippy ocelli-seg` and `bin/ocelli.sh clippy ocelli-render` | Restored tree, `-D warnings` | Executed now | Exit 0 and exit 0. |
| `bin/ocelli.sh test ocelli-render --test surface -- --ignored --test-threads=1` | In-tree ignored test | Executed now | Exit 0. 1 passed, 0 failed, finished in 0.29s. |
| `/tmp/f065_gpu` show and readback | Tier A device, same hand mesh | Executed now | Exit 0. `tier=A`. Edge blend is `BlendUnavailable`. Tick is `Submitted { viewports: 1, passes: 1 }`. Readback equals `present_surface`. |

## Defects

### D1, F-039 resize test replaced

**Where**: `crates/ocelli-render/tests/surface.rs:1`. Removed function at HEAD `crates/ocelli-render/tests/surface.rs:162`.
**What**: The working tree replaces `resize_replaces_the_target_and_two_viewports_submit_once` with the F-065 upload test. The old test is not moved.
**Why it is wrong**: `bin/ocelli.sh` gate `gpu` runs every ignored test in `ocelli-render`. That test was the only caller of `ResolvedAdapter::opened_from_this_instance` and the only readback where a rebind from a 4-wide colour target to an 8-wide target must draw the new far pixel and leave the old texture filled with `0x11`. `docs/sprints/AS_BUILT.md` still says the F-039 ignored device test lives in this file. Two viewports submitting once remain covered by `crates/ocelli-render/tests/graph.rs`. Stored rebind remains covered by `crates/ocelli-render/tests/upload.rs`. The resize sentinel does not.
**Evidence**: `git grep resize_replaces_the_target` is empty on the working tree. At HEAD the function is line 162. `rg opened_from_this_instance` finds only the definition in `crates/ocelli-render/src/probe.rs`. The deleted test was not re-run.
**Scope and regression**: Present only in this working tree. Consumers are the gpu gate and any later resize of a caller-supplied colour target. Smallest regression is to keep the F-039 test beside the F-065 test. Uncertainty: none about the deletion. The resize behaviour itself was not re-executed.

### D2, sprint audit still says F-065 is pending

**Where**: `docs/sprints/CURRENT_SPRINT.md:150`
**What**: The sentence says F-065 and F-066 stay pending. Line 11 of the same file, and the F-065 cell in `docs/sprints/BACKLOG.md`, say in-progress.
**Why it is wrong**: F-064 changed this sentence from "Those three are pending" to the present-tense split when that story became done. This diff updates the table and the backlog cell and leaves the sentence false.
**Evidence**: `git diff` for `docs/sprints/CURRENT_SPRINT.md` changes only the table row. The audit sentence is untouched.
**Scope and regression**: Bookkeeping only. No pixel consumer. Smallest fix is to say F-065 is in progress and F-066 stays pending.

## Smells

None.

`SurfaceDrawError::from_labelmap` still maps `LabelmapDrawError::Present` to `Extent` at `crates/ocelli-render/src/surface.rs:48`. `store_overlay` does not return `Present`. `labelmap::stage` returns `Extent` or the binding. The slot miss returns `NotRebound`. `show_surface` maps a presenter error before `store_overlay`. The arm stays unreachable, same as `crates/ocelli-render/src/contour.rs:48`. That is not a smell.

## Nitpicks

None.

## Verified clean

`SurfaceMesh` is `segment`, `vertices: Vec<Pt<World>>`, and `triangles: Vec<[u32; 3]>`. `present_surface` takes width and height separately. `show_surface` takes `SurfaceView` for those two fields. The signature stays at seven arguments. Clippy with `-D warnings` is clean on both crates.

Projection is `(x - origin.x) / spacing_x_mm` and the same for `y`. Vertices are not rounded. `z` uses `to_bits`, including a refusal of negative zero against positive zero. Coverage is the three barycentric weights of centre `(c + 0.5, r + 0.5)`, included at `>= 0`. A cross product whose bits are `0` or the bits of `-0.0` is skipped. There is no float `==`. There is no `as` cast in the new presenter or the upload module. `f64::from` of a `u32` column or row is exact.

Refusals write the caller buffer only after a full pending image is painted. A bad index is checked for every triangle before that buffer is allocated. The hand fixture's covered and clear claims match the independent grid. The mutation to `> 0.0` makes the hand test fail, so the zero-weight edge is locked. The LUT texel keeps alpha 128. Segment 0 has no colour because the existing table starts at segment 1.

`show_surface` returns `TierCUnavailable` before the mesh, frame, LUT, or view is read. A presenter error returns before `store_overlay`. Equal bytes return inside `store_overlay` before `stage`, and the ignored test sees no new dirt and no new submission. `encode_frame` still has one `queue.submit`, at `crates/ocelli-render/src/graph.rs:3707`. The labelmap pipeline blend is `None`, so the readback is the presenter texel rather than a blend over the clear. The independent tier A readback matched, including alpha 0 outside the triangle.

No new trait, generic, or `Box<dyn>`. No `unsafe`. No `wasm-bindgen`. No marching cubes. No new WGSL file. The contour stroke file is unchanged. F-066 registry code is not in this diff.

## Limitations and handoff

`docs/lld/render-graph.md`, `docs/lld/feature-availability.md`, and `docs/lld/README.md` are unchanged. The plan names them under LLD impact. F-064's clean implementation pass also reviewed the code before those edits. They are still completion work, not a behaviour defect in this pass. The current LLD sentences do not deny a surface presenter. They also do not describe this one.

Tier C `show_surface` was not executed. `RenderGraph::new` refuses tier C before a graph exists. The return sits before any mesh read. This host opened tier A, so the branch did not run. Tier B was not a separate device. It uses the same fragment module and `blend: None`.

The ignored test returns without an assertion when the adapter, the graph, or registration fails. On this host the adapter opened, registration succeeded, and both the in-tree test and an independent readback matched. The recovery readback was the in-tree test, not a second probe. A no-adapter machine would not have produced that evidence.

The blocking fixes are to restore the F-039 device test and to correct the audit sentence. This pass does not record sprint verification.
