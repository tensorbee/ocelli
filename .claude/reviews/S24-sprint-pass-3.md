# S24 sprint review, pass 3

**Reviewed**: HEAD `486be24e6ece9b09315e8d0da953ebdf74364b8c` on `sprint/s24`, plus the unstaged repairs in `docs/sprints/CURRENT_SPRINT.md`, `crates/ocelli-render/src/graph.rs` and `crates/ocelli-render/src/labelmap.rs`. `git rev-parse HEAD` returned that commit. `git rev-parse HEAD^{tree}` returned `246c4be008b8c669962a4c5d91bfa367e30665ef`. The index matches that tree. `git diff --cached` is empty. `git diff --name-only HEAD` names only those three paths. Untracked `.claude/reviews/S24-sprint-pass-1.md` and `.claude/reviews/S24-sprint-pass-2.md` are also present. They are prior reviews, not product. The range `8f2d179..HEAD` is `74e7f6e` (open), `f0be603` (approved design) and `486be24` (F-067). `8f2d179` is an ancestor. Annotated tag `s23` peels to that merge. The tag object is `60fb324fec3649a9d8c01d325e248a14ba549b71`. `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
**Author / reviewer**: independent reviewer. This pass did not write F-067, the design, pass 1, pass 2, or the unstaged repairs. The feature trailer says `Ocelli-Generated-By: grok`. A trailer does not identify the author.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

Pass 1 required the sprint record to say F-067 is done, to keep M6 open, and to leave the workspace at `0.1.0`. The working copy says `F-067 is done` in the subject paragraph, the carried-in bullet, and the audit intake. The same paragraphs say M6 stays open until this sprint closes. `Cargo.toml` still has `version = "0.1.0"` and is not in the diff. The backlog row for F-067 is `done`. F-063 through F-066 are `done`. None of those sentences publish `0.6.0`.

Pass 2 required the live overlay docs to stop saying the edge key stays uncompiled. `graph.rs` lines 29 to 31 now say the overlay compiles labelmap edge projection on the existing copy shader and that slab, volume and stack leave that blend uncompiled. `LabelmapDrawError::BlendUnavailable` now says the overlay blend is neither composite nor labelmap edge projection. `pipeline_compiled` is true only for `SegOverlay` plus composite or `LabelmapEdgeProjection`. Creation pushes that second key onto `LABELMAP_WGSL` and the existing labelmap layout. Slab, volume and stack are outside that push. `slab_blend_code` returns `None` for this blend. `present_slab` returns `BlendUnavailable` before the upload closure. `show_contour` and `show_surface` still require composite, so this blend is `BlendUnavailable` before the presenter.

The cut was checked with a point that is not in the fixtures. Open segment `(0, 0, 1)` to `(9, 3, 4)` at plane 2 has `t = (2 - 1) / (4 - 1) = 1/3`, so the point is `(3, 1, 2)` and the kind is `Point`. A constant `0.5` is `(4.5, 1.5, 2)`. The source uses `(plane_z - start.z) / dz` and stores `plane_z`. This pass did not execute that case. Pass 1 already made `a_cut_off_the_midpoint_uses_t` fail by setting `t` to `0.5`, with left `(5.0, 2.0, 1.0)` and right `(2.5, 1.0, 1.0)`, and then restored the division.

The 3 by 3 edge image was read against `present_labelmap`. A solid segment-1 grid has a centre whose four neighbours match, so outline-without-fill writes transparent there. A boundary neighbour that falls outside the grid is background, so those texels use outline alpha 255. The colour is `(10, 20, 30)`. Pass 1's caller-fill mutation turned the centre into `[10, 20, 30, 128]` and was restored. This pass did not repeat it.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git rev-parse HEAD^{tree}`, `git status --porcelain`, `git diff --cached --stat`, `git diff --name-only HEAD` | branch `sprint/s24` | Executed now | HEAD `486be24e6ece9b09315e8d0da953ebdf74364b8c`. Tree `246c4be008b8c669962a4c5d91bfa367e30665ef`. Index empty. Tracked dirty paths are the three repairs. Two prior review files are untracked |
| `git rev-parse s23`, `git rev-parse 's23^{commit}'`, `git rev-parse 8f2d179` | annotated tag and merge | Executed now | Tag object `60fb324fec3649a9d8c01d325e248a14ba549b71`. Peeled commit and `8f2d179` are the same `8f2d1797f2e91f08a49e7958de4ad17a5de10255`. That commit is an ancestor of HEAD |
| `git diff HEAD` on the three repairs | unstaged text against `486be24` | Executed now | Sprint prose says F-067 is done and M6 stays open. Module doc compiles the overlay edge key. Error doc names neither composite nor labelmap edge projection |
| `git show f0be603:.claude/plans/F-067-design.md` line 59, current plan line 59 | first approved wording against the current plan | Executed now | The approved commit left `pipeline_compiled` false for the overlay edge key. The current plan compiles that key. Slab, volume and stack stay false |
| Backlog awk for S24, and M6 rows F-063 to F-067 | status column | Executed now | S24 prints `F-067` and `done`. All five M6 rows are `done`. F-067 depends on F-064 only |
| `Cargo.toml` workspace `version`, `ci/wasm-size-budget.json` | line 7 and the budget object | Executed now, read | Version `0.1.0`. Baseline `2151196`, tolerance `0.05`. `int(2151196 * 1.05)` is `2258755`. The budget file is not in the sprint diff |
| Read `pipeline_compiled`, the compile push, `show_labelmap`, `show_contour`, `show_surface`, `present_slab`, `slab_blend_code`, and `encode_frame` | working `graph.rs` and `slab.rs` | Executed now, read, not re-run | Overlay edge key is compiled. Other passes are not. Edge show forces outline without fill. Contour, surface and slab refuse the blend before their presenters. One `queue().submit` in `encode_frame` |
| Read `intersect_contour` and the eleven new contour tests | `contour.rs` | Executed now, read | On-plane uses Z bits. Crossing uses the plan's `t` and stores `plane_z`. One emitted point is `Point`. Two or more are `OpenPlanar`. An on-plane contour keeps its kind |
| Pass 1 contour and GPU mutations, and pass 1 slab test | restored tree at this same HEAD | Prior receipt, not re-run | Midpoint cut and caller-fill readback went red, then restored. `labelmap_edge_is_not_drawn_as_composite` passed. This pass did not start cargo |

No sprint gate and no wasm-pack run were started. Cargo tests were not started.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 1 D1 is closed. The words `pending` and `remaining M6 story` are gone from `CURRENT_SPRINT.md`. `F-X011 remains pending` is a different story. The carried-in heading still says M6 is not complete and this open does not publish. A finished M6 would name `0.6.0`, and the text does not say that version is current. Tag `s23` peels to merge `8f2d179`. The merge message records 31 gates and tree `97f98dab6687`. S23 review pass 1 reports zero defects and zero smells. The size ceiling in the sprint file matches `int(baseline * 1.05)`.

Pass 2 D1 is closed. The module banner and the `BlendUnavailable` variant no longer say the edge stays uncompiled. The variant doc matches the show, which accepts composite and labelmap edge projection and refuses every other overlay blend. `show_slab` still says an uncompiled blend, including labelmap edge, returns `SlabError::BlendUnavailable`. That sentence is the slab path. `present_slab` returns that error when `slab_blend_code` is `None`, and the upload closure does not run. `docs/lld/viewport.md` says the same for the slab. `docs/lld/render-graph.md` says labelmap edge on the slab pass is uncompiled, and a later sentence says the segmentation overlay compiles the blend. `docs/lld/feature-availability.md` gives the overlay its own row and no longer ends the composite row with an uncompiled claim.

`intersect_contour` refuses a non-finite plane first, then a geometric length that does not match the kind, then a non-finite coordinate. On-plane is `to_bits` at `contour.rs:153`, `:206` and `:207`. A `Point` off the plane is `Empty`. A closed walk includes the segment back to index 0 and the result is not `ClosedPlanar`. Consecutive duplicates collapse by X, Y and Z bits. The eleven added tests are the `t = 0.25` and `t = 0.75` cut, the midpoint cut, the two-cross polyline, the on-plane closed kind, signed zero, the closed cut, non-finite input, the parallel miss, the shared vertex, the open-nonplanar cut, and the short closed contour. `crates/ocelli-seg/tests/contour.rs` has 18 `#[test]` items. `ocelli-seg` depends on `ocelli-core` only, stays `no_std` outside tests, and exports `intersect_contour`.

`show_labelmap` returns `TierCUnavailable` before `grid.width` is read. The edge arm builds `LabelStyle { fill: false, outline: true }` and does not pass the caller style. Composite still passes the caller style. `show_contour` at `graph.rs:3425` and `show_surface` at `graph.rs:3479` return `BlendUnavailable` for any non-composite blend. The device tests in `tests/contour.rs` and `tests/surface.rs` still expect that error for an edge overlay. They were read and not re-run. `encode_frame` looks up the viewport blend, so an edge overlay does not bind the composite key. The frame submit is the one call at `graph.rs:3816`. No shader file was added. The product diff has no new trait, no new generic, no `Box<dyn>`, no `unsafe`, and no `wasm-bindgen`.

The F-067 `AS_BUILT` entry says the overlay compiles the blend and that slab, volume and stack stay uncompiled. Its note that the first approved wording left the overlay key false matches `f0be603`. The current plan compiles that key. The entry's fixture points match the tests. `(0, 0, 0)` to `(10, 4, 4)` at plane 1 is `t = 0.25` and `(2.5, 1, 1)`. The 3 by 3 edge fixture is boundary `(10, 20, 30, 255)` and a transparent centre, which is `OUTLINE_ALPHA` of 255 and a non-boundary pixel with fill off. The changelog bullet above the older composite bullet records the same cut and the outline blend. The older bullet says labelmap edge was left for a later story. That is past tense for the earlier story. F-063 and F-066 `AS_BUILT` notes still contain older `stays` wording. They are prior completion records. The workflow forbids editing them, and they are not a claim that the overlay key is uncompiled now.

Changed Markdown outside tables and code fences has no em dash and no prose semicolon. The workspace version is `0.1.0`. M6 is not closed.

## Limitations and handoff

This pass did not run contour tests, GPU tests, clippy, `prose_check.py`, `bin/ocelli.sh gate`, or wasm-pack. Cargo was not started. Pass 1's mutation receipts were read and not replayed. The `t = 1/3` point above was derived from the source and was not executed. The falling `t = 0.75` assertion was not the observed failure in the prior mutation, because the rising assertion failed first. The ignored contour and surface device tests were not run. `RenderGraph::new` returns before a tier C graph exists, so the tier return was read and not called. A separate tier B device was not opened. The S23 size of 2,171,060 bytes was not measured again. The feature trailer is profile `feature` with corpus `pass` and tree prefix `246c4be008b8`. It was not re-executed. That profile does not include the gpu gate. Pass 1's adapter readback is the GPU receipt for the outline.

Nothing in the product tree was edited by this pass. Nothing was staged, committed or pushed. This file is not a sprint-profile record. Pass 1 D1 and pass 2 D1 are closed on the unstaged tree. This pass is a bounded review of that tree. It is not a recorded sprint close.
