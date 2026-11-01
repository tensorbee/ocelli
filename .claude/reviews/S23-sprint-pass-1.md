# S23 sprint review, pass 1

**Reviewed**: `33ccb34020d4d4a56ee1603ba74e0e1f354a8874..7a588d9cef739d1398d9c61a9808e8ad1874c07b` on `sprint/s23`. `git rev-parse HEAD` and `git rev-parse sprint/s23` both returned `7a588d9cef739d1398d9c61a9808e8ad1874c07b`. `33ccb34` is an ancestor. Commits are the S23 open, `47dfcad` (F-063 labelmap test evidence only), `bc27515` (approved designs), `130e924` (F-064), `c4431e0` (F-065) and `7a588d9` (F-066). The product tree was clean at the start. The temporary edit below was restored before this report. This file is the only new path.
**Author / reviewer**: independent reviewer. This pass did not write F-064, F-065, F-066, the F-063 evidence commit, or the per-story reviews. The feature trailers say `Ocelli-Generated-By: grok`. A trailer does not identify the author.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The cross-story claim under test was F-066 AC5. `show_surface` must return `RepresentationMismatch` before `present_surface` when the viewport choice is `Contour`. The registry mesh uses triangle index 9, which is not a vertex. A skipped comparison therefore cannot return `Ok`. It returns `Present(Index)`.

Disabling `representation != Representation::Surface` with `&& false` made `a_surface_show_respects_the_registry` fail. Left was `Err(Present(Index))`. Right was `Err(RepresentationMismatch)`. The presenter ran. Restoring the same bytes made that test pass. That rejects a registry check that is compiled but not taken, and it rejects a surface show that paints despite a contour choice.

F-064 AC1 to AC5 and F-065 AC1 to AC4 were run as the CPU fixtures. The hand triangle was also checked outside the crate. Centres use weights of pixel `(c + 0.5, r + 0.5)` on vertices `(0, 0)`, `(4, 0)` and `(0, 4)`. `(0.5, 0.5)` is `0.75`, `0.125`, `0.125`. Weight `0` at `(3, 0)` and `(0, 3)` is covered. `(4, 0)`, `(0, 4)`, `(3, 1)`, `(2, 2)` and `(4, 4)` are clear. `contour_and_surface_disagree_on_the_hand_triangle` passed, so the stroke and the fill do not share a raster.

`round_half_away` adds `0.5 - 0.25 * f64::EPSILON`, which is the next `f64` below `0.5`, then truncates toward zero. Exact `0.5` reaches `1`. The float just below `0.5` stays below `1`. `a_float_just_below_one_half_stays_on_zero` passed. A full `+0.5` is not what the source adds.

Z uses `to_bits` in both presenters. `-0.0` and `+0.0` differ in those bits. The CPU refusal test drives `z = 1`, not `-0.0`. Bresenham uses `e2 > -ady` and `e2 < adx`, and the diagonal fixture passed. A bad surface index is rejected before the coverage loop. A zero cross product is skipped with `to_bits`, including the negative-zero bits. One degenerate triangle does not erase a later real triangle. Nothing covered returns `Empty` and does not copy.

F-066 AC1 is the register path. A new slot stores `Representation::Labelmap`. `crates/ocelli-render/tests/labelmap.rs` does not call `set_representation`. `show_labelmap`, `show_contour` and `show_surface` use one order. Tier C returns first. Then the pass is an overlay or `NotOverlay` or `UnknownViewport`. Then composite blend or `BlendUnavailable`. Then the representation comparison. Then a missing GPU slot is `NotRebound`. Then the presenter. The labelmap both-flags call sits after the mismatch return, so a contour choice does not reach `present_labelmap` for an empty style either. Blend stays before representation, which is what the edge-blend asserts in the contour and surface device tests require.

`set_representation` on a non-overlay returns `NotOverlay` before it writes. The same value does not drop `slot.labelmap` and does not set dirt. A different value sets segmentation dirt and drops the texture only when the GPU slot exists. It does not upload. `install_recovered` sets every GPU slot to `None` and does not assign `representation`. `encode_frame` has one `queue().submit`, at `graph.rs:3778`. `gpu_binding_ready` for `SegOverlay` requires the labelmap binding, so a dropped overlay is not encoded and does not submit when it is the only dirty viewport.

`Present(_)` mapped to `Extent` in `ContourDrawError::from_labelmap` and `SurfaceDrawError::from_labelmap` is not reached. `store_overlay` returns `NotRebound` or `Extent`. Presenter errors return before that call. No caller hits the arm.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git log --oneline 33ccb34..HEAD`, `git status --porcelain` | branch `sprint/s23` | Executed now | HEAD `7a588d9cef739d1398d9c61a9808e8ad1874c07b`. Six commits. Status empty at start |
| `bin/ocelli.sh test ocelli-seg --test contour --test surface` | CPU presenters, no device | Executed now | Exit 0. Contour 7 passed. Surface 5 passed, including the hand disagreement |
| `bin/ocelli.sh test ocelli-render --test labelmap seg_overlay_compiles_composite_only -- --test-threads 1` | `PipelineKey` only | Executed now | Exit 0. 1 passed. 3 filtered. Composite segmentation compiles. Labelmap edge does not. No device |
| Mutated `graph.rs:3444` to `!= Surface && false`, then `bin/ocelli.sh test ocelli-render --test registry -- --ignored a_surface_show_respects_the_registry --test-threads 1` | SHA-256 `20ed7ed77f3a081cfbc46c10408d6e368a93c3463846a97fbf05e6773f943782`. Mesh index 9. Choice `Contour` | Executed now | Exit 101. Panic at `registry.rs:490`. Left `Err(Present(Index))`. Right `Err(RepresentationMismatch)`. 7 filtered. `rustc 1.97.1` |
| Restored `graph.rs` from `/tmp/s23-review-graph.rs.before`, then the same ignored filter | SHA-256 `c8dd98d1ba2f76fe95cdca91af5a710b76eca4ee1dc07a516845e02fd4e2f2c4` before the edit and after `cmp` | Executed now | `cmp` silent. Exit 0. 1 passed in 0.30s. 7 filtered. Adapter was present for both registry runs |
| `rg '&& false' crates/ocelli-render/src/graph.rs` | restored graph | Executed now | No match. `git diff -- crates/ocelli-render/src/graph.rs` empty |
| Feature trailers on `130e924`, `c4431e0`, `7a588d9` | `Ocelli-Verify` profile `feature`, corpus `pass` | Prior receipt, not re-run | Gate list has no `gpu`. Tree prefixes `437bcc080038`, `ea1466b45f42`, `bb7b8abcc157` |
| S22 sprint trailer on `6f750a1` | profile `sprint`, 31 gate names, tree `eb4825e5c3c3`, comparison verdict `pass` | Prior receipt, not re-run | Matches the audit sentence in `CURRENT_SPRINT.md`. This pass did not run `gate --sprint` |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`present_contour` stores `Pt<World>` and uses the PS3.3 C.8.8.6 names `Point`, `OpenPlanar`, `OpenNonplanar` and `ClosedPlanar`. It does not parse RTSTRUCT. `Point` paints one pixel. Open kinds do not close. `ClosedPlanar` needs three points and strokes back to the first without a fill. Pixels outside the grid are skipped. A stroke that hits nothing returns `Empty`. Segment 0 and any other missing segment return `MissingColour` through `LabelColourLut::colour_bytes` before a write. The stroke copies the LUT alpha. The open line, hollow square, diagonal, exact half and refusal rows passed.

`present_surface` takes a caller mesh. It does not run marching cubes. Vertices stay continuous. Coverage is all barycentric weights `>= 0`, so the edge is included. A clockwise triple of the same points passed. A bad index returns `Index` before any write. A single degenerate triangle is skipped. An empty cover leaves the destination unchanged. The hand colours are `(10, 20, 30, 128)`.

`Representation` lives in `ocelli-seg` and is not a colour, a grid or a mesh. `RenderGraph` stores one value per viewport. `show_contour` and `show_surface` call `store_overlay`, which keeps the existing texture when the bytes and size match. There is no second WGSL file. `labelmap.wgsl` loads the presenter texel and does not stroke, fill or run VOI. `seg_overlay_compiles_composite_only` passed, so labelmap edge stays uncompiled. F-067 is not implemented here.

`47dfcad` changes only `crates/ocelli-render/tests/labelmap.rs`. That recovery row still calls `show_labelmap` with no setter. The default remains `Labelmap`, so the new comparison does not refuse it. The contour device test sets `Contour` before the successful show at `contour.rs:225`. The surface device test sets `Surface` before the successful show at `surface.rs:491`. Those two device files were not executed this pass. The calls are in the source.

The new Rust has no `unsafe` and no `wasm-bindgen`. `ocelli-seg` stays `no_std` outside tests. No patient bytes were added. The changed Markdown in the plans, reviews, changelog, LLD and sprint files has no em dash and no prose semicolon outside tables and code fences. `docs/hld/` was not edited. `CHANGELOG.md` matches the three behaviours. The LLD overlay section matches the copy shader, the three presenters, the registry and the single submit. `AS_BUILT` test counts match the runs above. Seven contour fixtures, five surface fixtures, one ignored contour device test, one ignored surface device test beside the F-039 resize test, and eight ignored registry tests. The resize test `resize_replaces_the_target_and_two_viewports_submit_once` is still in `surface.rs`. Backlog rows F-064, F-065 and F-066 are `done` and each depends on F-063 only. F-067 stays `pending` on F-064.

`ci/wasm-size-budget.json` is still `bytes` 2151196 and `tolerance` 0.05. The integer ceiling named in `CURRENT_SPRINT.md` is 2258755, which is that product. The budget file was not edited by this sprint.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate --sprint`, `gate --all`, `gate gpu` or `wasm-pack`. The other seven registry tests stayed filtered. The labelmap, contour and surface device rows were not run. Tier B was not opened. `RenderGraph::new` returns `TierCUnavailable` before a graph exists, so the tier check inside `show_*` was read and not called. `-0.0` was not passed through either presenter. The bit patterns differ, and the source compares `to_bits`, but that input is not a fixture. The S22 size 2170992 is the receipt in `.claude/scratch/F-063-progress.md`. It was not measured again.

The restored product tree matches HEAD `7a588d9cef739d1398d9c61a9808e8ad1874c07b`. `graph.rs` SHA-256 is `c8dd98d1ba2f76fe95cdca91af5a710b76eca4ee1dc07a516845e02fd4e2f2c4`. Nothing was staged, committed or pushed. This file is not a sprint-profile record. A later host can stage and call `scripts/sprint_workflow.py record-sprint-review` if that receipt is required. No remediation is waiting on this pass.
