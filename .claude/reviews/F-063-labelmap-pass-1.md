# F-063 review, pass 1

**Reviewed**: working tree on `sprint/s22` at `74814ba84870ce251dcc33d4eb40a9b286d577a8` (approved design only). The implementation is uncommitted. `git rev-parse HEAD` was unchanged. The product paths in `git status --short` matched from start to end. The end pin also lists this review file. Tracked edits are `Cargo.lock`, `crates/ocelli-render/Cargo.toml`, `crates/ocelli-render/src/graph.rs`, `crates/ocelli-render/src/lib.rs` and `crates/ocelli-seg/src/lib.rs`. Untracked product files are `crates/ocelli-render/shaders/labelmap.wgsl`, `crates/ocelli-render/src/labelmap.rs`, `crates/ocelli-render/tests/labelmap.rs`, `crates/ocelli-seg/src/labelmap.rs` and `crates/ocelli-seg/tests/labelmap.rs`. Nothing was staged. Nothing was committed. The outline mutation in `crates/ocelli-seg/src/labelmap.rs` was restored before the end pin. This review file is the only path added by the pass.
**Author / reviewer**: the implementation was already in the working tree. The reviewer is an independent microscope pass and did not write that code. The author is not named by the tree. This is not a human review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. A 3 by 3 grid of segment 1 with fill and outline on. The centre, index 4, has four in-grid neighbours of segment 1, so it is fill `(10, 20, 30, 128)`. Index 0 has a neighbour outside the grid, which counts as background, so it is outline `(10, 20, 30, 255)`. `labelmap_fill_and_outline_use_the_segment_colour` passed. Replacing the outline branch with the LUT colour, alpha 128, made that row fail at `crates/ocelli-seg/tests/labelmap.rs:72` with left `[10, 20, 30, 128]` and right `[10, 20, 30, 255]`. The centre assert on the line above did not fire. The constant `OUTLINE_ALPHA` was restored to 255 and is the alpha the outline branch writes. The same test was green again.

AC2. Outline off keeps the LUT alpha on an edge. `outline_off_keeps_the_fill_alpha` passed. A border forced to 255 would fail the edge assert.

AC3. Fill off leaves the centre transparent and the edge opaque. `fill_off_draws_only_the_boundary` passed. The same outline mutation turned the edge alpha into 128 and failed this row as well.

AC4. A 2 by 1 grid of segment 1 then segment 2, with only segment 1 in the LUT, returns `MissingColour` and leaves `0xA5` sentinels. The paintable pixel is first, so a per-pixel commit would have changed the first four bytes. `a_missing_colour_writes_nothing` passed.

AC5. A short index buffer, a 35-byte destination, both flags off, and a zero width each return before any write. `a_refusal_writes_nothing` passed. On the device, both flags off and a missing colour return `LabelmapDrawError::Present` and leave `submissions` at 0.

AC6. PS3.3 C.8.20.2. Pixel 0 is background and is not a segment number. `background_is_not_a_segment` is a 2 by 1 grid `[1, 0]`. The labelled pixel is outline `(10, 20, 30, 255)` because its neighbour differs. The zero stays `(0, 0, 0, 0)`. `colour` subtracts 1 before the table lookup, and the segment 0 branch never calls it. The outline mutation also failed this row, which shows the edge assert is the outline alpha and not a copy of the neighbour into the zero.

AC7. `pipeline_compiled` is true for `SegOverlay` plus `Composite` and false for `LabelmapEdgeProjection`, for slab labelmap edge, and for `SegOverlay` plus `Mip`. `seg_overlay_compiles_composite_only` passed. The device row refuses labelmap edge with `BlendUnavailable` before `present_labelmap`.

AC8. After `show_labelmap`, the readback of the 3 by 3 target equals `present_labelmap`, including centre alpha 128 and edge alpha 255. A second show of the same bytes leaves `Dirty::default` and does not submit. A new LUT colour is read back as that presenter's bytes. `set_voi` returns `PassHasNoUniform`. `shown_labelmap_matches_the_presenter` passed on the Auto adapter. The shader is `textureLoad` of the uploaded texel. It has no neighbour test and no VOI uniform.

AC9. A composite `SegOverlay` with no upload is not ready. Its first tick is `Unavailable` and `submissions` stays 0. That is the start of the labelmap device test. `labelmap_edge_and_a_foreign_pass_upload_nothing` then ticks a stack beside an edge overlay and a refused composite show, and submits one viewport. The existing `set_voi_is_the_single_uniform_route_and_rejects_in_order` row still submits one viewport and still reads the stack value 128. Compiling the overlay pipeline did not draw the stack shader into the overlay and did not add a second pass.

The shape the suite does not draw is a 3 by 3 cross with centre and arms at segment 1 and corners at 0. Read from `neighbour_differs`, the centre's four neighbours match, so it stays fill. Each arm touches a 0 or the outside, so it is outline. Each corner is segment 0 and stays transparent. That case was not executed. The solid 3 by 3 fixture cannot tell a 4-connected boundary from an 8-connected one, because every edge pixel already has an outside 4-neighbour and the centre has no differing diagonal. The code walks only the four orthogonal steps.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD` and `git status --short` | `sprint/s22` working tree, start and end | Executed now | HEAD `74814ba84870ce251dcc33d4eb40a9b286d577a8` both times. The same five tracked edits and five untracked product paths. The end status adds only this review file. |
| `bin/ocelli.sh test ocelli-seg --test labelmap` | restored presenter | Executed now, before and after the mutation | Exit 0 both times. 6 passed, 0 failed. |
| `bin/ocelli.sh clippy ocelli-seg` | working tree | Executed now | Exit 0. |
| `bin/ocelli.sh clippy ocelli-render` | working tree | Executed now | Exit 0. |
| `bin/ocelli.sh test ocelli-render --test labelmap` | working tree, ignored tests not run | Executed now | Exit 0. 1 passed, 2 ignored. `seg_overlay_compiles_composite_only`. |
| `cargo test -p ocelli-render --test labelmap -- --ignored --test-threads=1` | Auto adapter, serial | Executed now | Exit 0. 2 passed, 0 failed, 0.21s. Readback matched the presenter. Refusals uploaded nothing. |
| Outline branch returned the LUT colour | `crates/ocelli-seg/src/labelmap.rs` `paint` | Executed now, then restored | Exit 101. AC1, AC3 and AC6 failed. AC1 left was `[10, 20, 30, 128]` and right was `[10, 20, 30, 255]` at `tests/labelmap.rs:72`. |
| `rg OUTLINE_ALPHA crates/ocelli-seg/src/labelmap.rs` | restored file | Executed now | Line 85 is `const OUTLINE_ALPHA: u8 = 255`. Line 156 writes that constant. `git diff` on the path was empty. |
| `cargo test -p ocelli-render --test view -- --ignored set_view_dirties_only_the_camera_and_keeps_the_css_matrix --nocapture --test-threads=1` | same Auto open | Executed now | Exit 0. Printed `auto opened tier A`. Finished in 0.16s. |
| `cargo test -p ocelli-render --test graph -- --ignored set_voi_is_the_single_uniform_route_and_rejects_in_order --test-threads=1` | existing graph device row | Executed now | Exit 0. 1 passed in 0.13s. Empty composite overlay did not change the one-viewport stack submit. |
| All 256 unorm8 values through f32 divide by 255 and multiply back | outside the crate | Executed now | 0 values failed `round` or half-away. 10, 20, 30, 64, 128 and 255 are in that set. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`present_labelmap` refuses both flags off, a zero edge, an index length other than `width * height`, and a destination other than four bytes per pixel before it allocates `pending`. It then paints every pixel into that buffer. `out.copy_from_slice` runs only after the loop. `MissingColour`, `Length` and `Empty` return with `?` and drop `pending`. Segment 0 writes transparent RGBA and does not call `LabelColourLut::colour`. A non-zero segment with no entry returns `MissingColour` without copying. `colour` maps segment 1 to index 0 through `checked_sub(1)`. There is no LUT slot for 0.

Outline is one pixel. `neighbour_differs` tests `(1, 0)`, `(-1, 0)`, `(0, 1)` and `(0, -1)`. A step outside the grid returns boundary immediately. A neighbour whose segment differs also returns boundary, including another labelled segment. The outline pixel keeps the segment RGB and stores `OUTLINE_ALPHA`, which is 255. Fill off and not on the boundary writes transparent. The caller passes `LabelStyle`. The presenter has no hidden default. The image VOI types are not used.

`ocelli-seg` is `no_std` outside tests, uses `alloc`, and has no dependencies. The integration test links that library, so the green seg run is a `no_std` build. `ocelli-render` depends on `ocelli-seg`. The reverse edge is absent. The new Rust has no `unsafe`, no `wasm_bindgen`, no `as` cast, and no `unwrap`, `expect` or slice index. Clippy on both crates was clean with `-D warnings`.

`show_labelmap` returns `TierCUnavailable` before it reads the grid. `RenderGraph::new` already returns `TierCUnavailable` on tier C. A non-overlay viewport is `NotOverlay`. A non-composite blend is `BlendUnavailable`. Both of those returns happen before `present_labelmap`. An absent GPU slot is `NotRebound`. Present errors return before `stage`. `stage` creates the `Rgba8Unorm` texture, the padded upload and the bind group, and the slot stores that binding only after `stage` returns. Equal bytes return before `stage`, so the texture is not replaced and dirt is not set. The GPU row observed `Dirty::default` and an unchanged `submissions` count. `show_labelmap` does allocate a host `Vec` for the presenter bytes, and `present_labelmap` allocates `pending`, including on the equal-bytes path. That is the show path. `tick` does not create the overlay texture.

`encode_frame` still has one `ctx.queue().submit`. `stage` uses `write_texture`, not `submit`. The labelmap pipeline is built in `compile`, which `new` and `install_recovered` call. It is not built inside `tick`. The bind group is created in `stage` and only bound from `tick`. `gpu_binding_ready` for `SegOverlay` requires the labelmap binding. An unbound overlay is skipped and does not fall through onto `bind_uniform` or `bind_source`. `LoadOp` is `Clear` for every pass other than `Slab`, so the overlay is not on the slab load path. Its pipeline key and bind group are its own. `set_voi` matches only stack, volume ray and slab, and returns `PassHasNoUniform` for `SegOverlay` before any uniform write. The GPU row observed that error.

`labelmap.wgsl` samples `texture_2d<f32>` with `textureLoad` and returns that vector. A fragment outside the overlay dimensions is `discard`, which avoids an out-of-bounds load. The vertex triangle matches the existing fullscreen triangle. Fragment position uses framebuffer coordinates, and the upload origin is the first presenter row, so row 0 is the top row. The 3 by 3 fixture is symmetric, so that read is what covers orientation. The pipeline blend is `None`, so the stored bytes are the shader output. The unorm8 round trip holds for every byte, which is why alpha 128 and 255 can match after `Rgba8Unorm`.

`install_recovered` stores the new `labelmap_layout` and then drops every GPU slot. `rebind` builds a slot whose `labelmap` is `None`. The next `show_labelmap` therefore fails the equal-bytes check and uploads on the new layout. That sequence was read and not run.

The inspected gaps are not findings. Tier B was not opened. The labelmap module string does not branch on tier, and this machine's Auto open is tier A. AC6's transparent background is the CPU presenter. The GPU grid is all segment 1, and the shader never sees segment numbers. A target larger than the label discards the extra fragments, which stay `wgpu::Color::BLACK` (alpha 1) because the load is `Clear`. The accepted GPU fixture is 3 by 3 for both the grid and the target, so those fragments are overwritten. A labelled pixel that fails mid-grid does not touch `out`. Segment 0 does not look up the LUT. An unbound `SegOverlay` does not encode. The frame still submits once. The overlay is not aliased onto the slab path.

Contour, surface, the segmentation registry and labelmap edge projection are not implemented. `voi.wgsl` is not in the diff.

## Limitations and handoff

Tier B was not opened. The ignored labelmap tests call `resolve_adapter` with `TierRequest::Auto`. A separate view test on this machine printed tier A. The labelmap binary itself does not print the tier. Its readback asserts ran, because a skipped open returns before them and they passed.

`show_labelmap` after `install_recovered` and `rebind` was not executed. A render target larger than the label was not drawn. The GPU fixture has no background texel. A 4-connected case whose only differing neighbour is diagonal was not drawn. wasm-pack was not run. The full sprint gate was not run. No browser and no JS boundary were exercised. The presenter is Rust and does not export pixels.

This is a bounded review of the uncommitted labelmap tree. It is not a sprint-profile verification and it is not a completion record.
