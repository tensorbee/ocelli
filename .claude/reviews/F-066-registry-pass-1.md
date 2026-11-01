# F-066 review, pass 1

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
?? crates/ocelli-render/tests/registry.rs
```

`git diff --stat` at the start was 9 files, 117 insertions and 15 deletions. The untracked registry test is not in that stat. The same stat was printed after both temporary edits were restored. `crates/ocelli-render/src/graph.rs` is `c8dd98d1ba2f76fe95cdca91af5a710b76eca4ee1dc07a516845e02fd4e2f2c4` before the edits and after the restore. `cmp` against `/tmp/graph.rs.pre` was silent. `crates/ocelli-render/tests/registry.rs` is `65c69aaa10ca18091c10c33ae6e5c2dbd012e0d734912f7a0a1a6725cab5f602` and was not left edited. A probe that printed the adapter tier was restored before the product mutations. The end status adds this review as untracked. Product paths match the start.

**Author / reviewer**: this reviewer did not write the registry. There is no earlier F-066 review.
**Result**: 0 defects, 1 smells, 0 nitpicks

## Acceptance challenged

AC3 is the labelmap refusal after a switch. The approved plan says `show_labelmap` requires `Labelmap`, returns before the presenter, and uploads nothing. The wrong behaviour rejected here is a check that does not refuse.

`switching_drops_the_overlay_without_upload` shows a 1 by 1 labelmap, ticks once, then sets `Contour`. The next tick is `Unavailable` and `submissions` does not move. That rejects leaving the labelmap texture bound, because a bound overlay with segmentation dirt encodes and submits. The following `show_labelmap` uses a grid index of 2 against a one-colour LUT. With the check in place the error is `RepresentationMismatch`.

The check was then ignored inside `show_labelmap` only. The binding was renamed `_representation` and the comparison was deleted. The same test failed at `crates/ocelli-render/tests/registry.rs:384`. Left was `Err(Present(MissingColour))`. Right was `Err(RepresentationMismatch)`. `present_labelmap` is what returns `MissingColour` for that grid. The presenter ran. The source was restored before the contour experiment.

The claim the registry tests do not establish is that `show_contour` requires `Contour`. AC4 shows that a contour choice uploads and matches `present_contour`. AC5 shows that `show_surface` refuses a contour choice. Neither runs `show_contour` while the stored value is `Labelmap` or `Surface`. Deleting the contour comparison left all seven registry tests green. That is S1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff --stat` | `sprint/s23` working tree, start and after both restores | Executed now | HEAD `c4431e0adc602592b40178b0f3a513f86c6c355b` both times. Tracked diff stayed 9 files, 117 insertions, 15 deletions. |
| sha256 of `crates/ocelli-render/src/graph.rs` and `cmp` against `/tmp/graph.rs.pre` | pre-edit copy taken before either mutation | Executed now | `c8dd98d1ba2f76fe95cdca91af5a710b76eca4ee1dc07a516845e02fd4e2f2c4` before the edits and after the final `cp`. `cmp` matched. |
| sha256 of `crates/ocelli-render/tests/registry.rs` | same bytes as the first hash | Executed now | `65c69aaa10ca18091c10c33ae6e5c2dbd012e0d734912f7a0a1a6725cab5f602` before the adapter probe and after its restore. |
| `bin/ocelli.sh test ocelli-render --test registry -- --ignored --test-threads=1` | restored registry, before any mutation | Executed now | Exit 0. 7 passed, 0 failed, 0.69 seconds. |
| one-test adapter probe, then the test file restored | `a_new_overlay_starts_as_labelmap` only | Executed now | Exit 0. Printed `registry-adapter tier=A`. The print was not left in the tree. |
| labelmap comparison deleted, then `switching_drops_the_overlay_without_upload` | `show_labelmap` only. `diff` against the backup was that rename and the deleted `if`. | Executed now, then restored | Exit 101. Failed at `tests/registry.rs:384`. Left `Err(Present(MissingColour))`. Right `Err(RepresentationMismatch)`. |
| contour comparison deleted, then the same seven registry tests | `show_contour` only. The labelmap comparison was present. | Executed now, then restored | Exit 0. 7 passed, 0 failed, 0.76 seconds. |
| `bin/ocelli.sh cargo fmt --check -p ocelli-render -p ocelli-seg` | restored tree | Executed now | Exit 0. |
| `bin/ocelli.sh clippy ocelli-seg` and `bin/ocelli.sh clippy ocelli-render` | restored tree | Executed now | Exit 0 for each. |
| `git status --short -- crates/ocelli-render/shaders` | shader directory | Executed now | No output. No new WGSL file. |

## Defects

None.

## Smells

### S1, contour mismatch is never executed

**Where**: `crates/ocelli-render/src/graph.rs:3390`

**What**: `show_contour` returns `ContourDrawError::RepresentationMismatch` when the stored choice is not `Contour`. No test reaches that return. Deleting the comparison left the registry suite green.

**Why it is wrong**: The current source does refuse, so this is not a wrong result today. It will not stay refused. HLD section 22 is the overlay pass and its dirty segmentation bit. The design says `show_contour` requires `Contour` and a mismatch returns before the presenter. A branch nothing executes is not a guard. The later failure is a contour upload onto a viewport whose registry still says `Labelmap`, which replaces the labelmap the caller selected. That is the mix this story exists to stop. HLD section 31 keeps a segmentation result on the representation the renderer was given.

**Evidence**: The `if` at `graph.rs:3390` was removed and the binding was renamed `_representation`. `bin/ocelli.sh test ocelli-render --test registry -- --ignored --test-threads=1` exited 0 with 7 passed. `the_matching_show_uploads` and `recovery_keeps_the_choice_and_drops_the_texture` both call `set_representation(Contour)` before `show_contour`, so they still pass when the comparison is gone. No registry assertion expects `ContourDrawError::RepresentationMismatch`. The source was restored. `cmp` matched and the sha256 above did not change.

`crates/ocelli-render/tests/contour.rs` was not run. Its edge show expects `BlendUnavailable`, which returns at `graph.rs:3387` before this comparison. Its successful shows call `set_representation(Contour)` first. `NotRebound` and `UnknownViewport` are also outside this arm. None of those assertions name `RepresentationMismatch`.

**Scope and regression**: `show_contour` only. The labelmap arm is guarded. The mutation above went red. The surface arm is guarded by `a_surface_show_respects_the_registry`, which expects `SurfaceDrawError::RepresentationMismatch` for a mesh whose index 9 is `SurfaceError::Index` if `present_surface` runs. That surface mutation was not run.

Smallest regression: one ignored test on a new composite overlay. Do not call `set_representation`. Call `show_contour` with the open line already used by `the_matching_show_uploads`. Expect `ContourDrawError::RepresentationMismatch` and an unchanged `submissions` count. `present_contour` of that line returns `Ok`, so a deleted check makes the error `Ok` and the assertion fails. A stronger row in the same test shows a labelmap, ticks, then calls `show_contour`, expects the mismatch, and expects the next tick to be `Noop`. The current mismatch test drops the texture before the wrong show, and `submissions` does not count `write_texture`.

## Nitpicks

None.

## Verified clean

`ocelli_seg::Representation` is `Labelmap`, `Contour`, `Surface` at `crates/ocelli-seg/src/lib.rs:26`. It derives `Copy`, `PartialEq` and `Eq`. It is not a colour, a grid or a mesh. The crate stays `no_std` outside tests. `ocelli-render` stores one value on `SlotState` and does not re-export the enum. `register` writes `Labelmap` at `graph.rs:1078`. There is no `Default` impl that could drift from that write.

`representation` and `set_representation` are the public pair. Unknown ids return `GraphError::UnknownViewport` before a write. A stack, a slab and a volume return `GraphError::NotOverlay` at `graph.rs:3283` before the enum or the dirt flags change. The same value sets no dirt and does not clear `slot.labelmap`. A different value sets `dirty.segmentation` and then sets `slot.labelmap` to `None`. It does not call `stage` and does not submit. The gpu index is `id.0`, which is the same index `register`, `show_*` and `rebind` already use. The viewport id is that slot index, not a second map.

`a_new_overlay_starts_as_labelmap` passed on tier A and includes the unknown-id pair. `a_stack_has_no_representation` passed. The stack tick after the refusal was `Submitted { viewports: 1, passes: 1 }`. The volume and slab setters returned `NotOverlay` in that same test. `the_same_choice_keeps_the_binding` passed. After a clean tick, setting `Labelmap` again left `Dirty::default()` and the next tick was `Noop`. A same-value path that also set segmentation dirt would not be `Noop` while the texture was still bound.

`switching_drops_the_overlay_without_upload` and `the_matching_show_uploads` both saw `Unavailable` after the switch to `Contour` and before the matching show. That is the drop. The matching show then submitted once and the readback equalled `present_contour` of the same open line, frame, LUT and 4 by 2 view. That line is the F-064 fixture: spacing 1, origin 0, points `(0, 0, 0)` and `(3, 0, 0)`. `open_line_paints_the_stroke_only` paints pixels 0 through 3 as `(10, 20, 30, 128)` and leaves pixel 4 clear. A blank upload would not match that buffer. `recovery_keeps_the_choice_and_drops_the_texture` passed. After `install_recovered` the enum was still `Contour`, `show_contour` was `NotRebound`, the tick stayed unavailable after `rebind`, and the next matching show matched `present_contour` again. `install_recovered` drops every gpu slot at `graph.rs:3546` and does not assign `representation`.

`show_labelmap`, `show_contour` and `show_surface` each test `tier == Cpu` before reading a grid, a point list or a mesh. The labelmap grid is first used at `graph.rs:3345`. The contour value is first passed to `present_contour` at `graph.rs:3408`. The mesh is first passed to `present_surface` at `graph.rs:3462`. Those calls sit after the representation comparison. `RenderGraph::new` still refuses tier C, so this branch is the later `install_recovered` tier, not a second graph constructor. It was not executed. The order was read.

Mismatch is checked after the composite blend test and before `NotRebound` and the presenter. An edge overlay therefore still returns `BlendUnavailable` without a setter call. That is what `shown_contour_matches_the_presenter` and `shown_surface_matches_the_presenter` assert. Both of those tests call `set_representation` before the first successful show. Later shows in the same test keep the stored value. `crates/ocelli-render/tests/labelmap.rs` has no `set_representation` call. The default remains `Labelmap`.

`ContourDrawError::from_labelmap` and `SurfaceDrawError::from_labelmap` map `RepresentationMismatch` and still match every `LabelmapDrawError` arm. Clippy on both crates passed, so the matches are exhaustive under the workspace denials. The diff adds no `as` cast, no `unsafe`, no `wasm_bindgen`, no trait and no new shader. `encode_frame` still has one `ctx.queue().submit` at `graph.rs:3778`. `stage` writes the texture and does not submit. The registry tests saw `submissions` unchanged across the setter and the mismatched shows.

`resize_replaces_the_target_and_two_viewports_submit_once` is still at `crates/ocelli-render/tests/surface.rs:166`. It still expects one submission for two viewports and a second submission after the resize. The surface diff does not touch that function. It adds the `Representation` import and the setter in `shown_surface_matches_the_presenter`.

`docs/sprints/BACKLOG.md` and the S23 table in `docs/sprints/CURRENT_SPRINT.md` say `in-progress`. The closing note says F-066 is in progress. That matches an uncommitted story. Neither file says the story is done.

## Limitations and handoff

S1 blocks completion. The fix is a test, not a change to the refusal that is already in `show_contour`. This pass does not record a sprint review and does not start F-067.

`crates/ocelli-render/tests/labelmap.rs`, `contour.rs` and `surface.rs` were not executed. A second GPU suite was out of scope. The surface success path and the F-039 resize test were read, not run. The surface mismatch assertion was not mutated. Tier B was not separated from tier A. The run opened a tier A adapter. A machine that resolves tier C would return before the assertions, because `context()` ignores a failed open. That did not happen here.

The CPU `show_*` returns were not called. `bin/ocelli.sh wasm`, the feature floor, the oracle and the corpus were not run. `docs/lld/render-graph.md` and `docs/lld/feature-availability.md` are unchanged. The plan names them as LLD impact for completion, not as files in this diff.

The same-value test does not read the texture back. It would miss a drop that forgot to set dirt. The implementation drops only in the branch that sets dirt, and the `Unavailable` ticks cover that branch for a real change.
