# F-045 review, pass 1

**Reviewed**: working tree against 88b939a on sprint/s15
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the GPU outside tests do not drive a negative image coordinate

**Where**: `crates/ocelli-render/src/graph.rs:484` and `crates/ocelli-render/tests/view.rs:329`
**What**: `a_fragment_past_the_image_is_black_under_the_identity` draws a width-1 image into a width-2 target. `set_view_writes_sixty_four_bytes_and_only_the_camera_flag` translates by +10. Both are past the high edge. Neither puts a fragment in `(-1, 0)`, which is the range where `i32` truncation would select texel 0.
**Why it does not block**: The `< 0` clause is in the shader. For the plan's 2 by 4 fit into an 8 by 8 canvas, scale is 2 and the anchor is `(4, 4)`, so canvas x `0.5` is image x `-0.75`. That value is rejected before `i32`. The behaviour matches decision 3. The current GPU tests would stay green if only that clause were removed. The plan's test table does not require a negative GPU row.

## Plan contradiction

Not counted above. The Approach section says `ocelli-viewport` gains a dependency on `ocelli-core` only. `crates/ocelli-viewport/Cargo.toml` also depends on workspace `glam`. Decision 5 requires `no_std` and no wgpu. `f64::sin_cos` is not in `core`. The workspace `glam` entry keeps default features off and enables `libm`, which is the entry `ocelli-pixel` already uses for `f32::exp`. `ocelli-core` does not re-export `glam`, so `glam::DVec2::from_angle` needs a direct dependency. `python3 scripts/no_std_check.py` printed `OK: 7 no_std crate(s) reach no std feature`. This departs from the Approach sentence. It does not depart from decision 5, and it is not wrong behaviour.

## Verified clean

The diff is the uncommitted F-045 tree on `88b939a`. F-100 was not reviewed. `view.rs`, `tests/view.rs`, `crates/ocelli-render/tests/view.rs` and `docs/lld/viewport.md` are untracked. The tracked edits are `Cargo.lock`, `graph.rs`, `ocelli-viewport/Cargo.toml`, `lib.rs`, `docs/lld/README.md` and `docs/lld/render-graph.md`.

**Hand fixtures.** All ten tests in `crates/ocelli-viewport/tests/view.rs` passed. Identity on `(0.5, 0.5)` and `(3.5, 0.5)` is bit-exact, which is texel `(0, 0)` and `(3, 0)` after `i32` truncation. The 2 by 4 fit into an 8 by 8 canvas is scale 2 and offset `(2, 0)`, and image `(0.5, 0.5)` lands at canvas `(3, 1)`. The inverse matrix maps that canvas point back to `(0.5, 0.5)`. A quarter turn of image `(0, 0)` on a 2 by 2 image lands at `(2, 0)`. Horizontal flip sends image x `0` to canvas x `2`. The same quarter turn then fitted swaps the bounds to scale 2 and offset `(0, 2)`. Pan of `(1, 0, 0)` moves one canvas pixel, `z` of `1` and `-1` is refused without replacing a stored pan, and negative zero is accepted. Zoom `2` after that fit keeps image `(1, 2)` on canvas `(4, 4)` and doubles the scale. Spacing `(0.5, 2.0)` on a 2 by 2 image fitted into 8 by 8 is scale 2 and offset `(3, 0)`, which is the physical size F-049's fit row uses. These are the plan's composition, not a copy of a shader readback.

**Quarter-turn mutation.** `clockwise` was changed from `(dx * cos - dy * sin, dx * sin + dy * cos)` to `(dx * cos + dy * sin, -dx * sin + dy * cos)`. At `sin = 1` and `cos = 0` that is `(dy, -dx)` instead of `(-dy, dx)`. `bin/ocelli.sh test ocelli-viewport --test view -- clockwise_quarter_turn` exited 101. The failure was `x (0.0, 2.0) against (2, 0)`. The formula was restored. The same ten tests then passed, and `clippy -D warnings` on `ocelli-viewport` exited 0.

**Inverse and narrowing.** A temporary probe, removed before this file was written, round-tripped `image_to_canvas` through `camera_matrix` for horizontal flip, vertical flip, both flips with a quarter turn, `0.4` rad with spacing `(0.5, 2.0)` and a shift, a negative quarter turn with spacing `(2.0, 0.5)` and a shift, and `PI` with a vertical flip. Each case was fitted, zoomed by `1.5` and panned. Five image points came back within `1e-4`. With a horizontal flip and a zero centre, component 12 matched Rust `as f32` for exponents `-1100` through `1099` and factors `1`, `1.25`, `1.5`, `1.75`, `-1` and `-1.5`, covering f32 subnormals and overflow to signed infinity. The tie `1 + 2^-24` narrowed to `1.0`. `camera_matrix` is column-major `[a, d, 0, 0, b, e, 0, 0, 0, 0, 1, 0, c, f, 0, 1]`, which is the inverse of the forward basis. There is no `as` cast in `view.rs`.

**Singular spacing.** F-049 decision 2 says a zero spacing contributes no shift. `set_spacing(0, 1)` with shifts `(4, 3)` keeps every matrix component finite, keeps x on the centre `0.5`, and keeps the row shift. Both spacings zero write the image centre `(0.5, 2)`. That is the rank-one fallback and the empty-matrix centre, not a second formula.

**Shift and zoom.** Decision 6 places the index shift in the translation, after scale, on canvas axes. `translation` adds `shift_col * column_mm * scale` and does not rotate it. The LLD states that formula. A non-zero shift therefore scales with zoom, so the fixed point is the anchor rather than the geometric centre. Decision 8's fixture has no shift, and that fixture holds. F-049 decision 5 is satisfied. `set_spacing` and `set_index_shift` are the writers, and neither `view.rs` nor the shader has to change for F-049 to call them.

**GPU, tier A.** `bin/ocelli.sh test ocelli-render --test view -- --ignored --nocapture --test-threads=1` exited 0 in 0.60s. Both camera tests printed `auto opened tier A` and passed. The width-2 identity target read back `[128, 128, 128, 255, 0, 0, 0, 255]`. Stored `40` at centre `40` and width `400` is red `128`, so the second pixel is black rather than a windowed zero. `set_view` of a `+10` translation then read back `[0, 0, 0, 255]`. `uploads`, `lut_uploads` and `brick_uploads` stayed put. `dirty` was camera only. `voi_params` bytes were unchanged. That comparison reads the CPU `Pass`, not the GPU VOI buffer. The function writes only `slot.camera` and then sets `state.camera` and `dirty.camera`. After `install_recovered`, `set_view` returned `ViewportNotRebound`. `rebind` kept the same sixteen floats, and the new draw was black rather than red `128`, so the restored uniform is the one the fragment used. `bin/ocelli.sh test ocelli-render --test graph -- --ignored the_section_18_3_rows_render_through_the_graph --test-threads=1` exited 0. The expected row in that test is still `[0, 128, 255, 64]`. Registration passes `IDENTITY_CAMERA` into the new binding. `vs_main` is unchanged.

**Tiers.** Tier A ran, as above. Tier B uses the same fragment module. `tools/tier-b/run.mjs` calls `read_section_18_3`, which registers through `RenderGraph::register_stored` and does not call `set_view`, so a browser draw of section 18.3 is the identity camera on this shader. This pass did not launch that browser run. The native GL test printed `native Backends::GL found no adapter. This is not a tier B run.` That is not a tier B pass. Tier C is unchanged. `RenderGraph::new` still returns `TierCUnavailable` when the tier is CPU. `View` has no device and does not rasterise. There is no second LUT chain. `the_colour_shader_keeps_one_lut_chain_and_the_two_keys` passed, and so did the two VOI shader text tests. `fs_main` returns black before `lut_chain` when the mapped coordinate is outside the texture. The camera buffer is created in `create_gpu_slot`, not in `tick`. `tick` still submits once. `mark_dirty` still accepts `camera`. `bind_surface` and `reconfigure_surface` set that flag and do not replace the stored matrix. `rebind` and `rebind_stored` rebuild the uniform from `state.camera`. `select_colour_lut` and `replace_stored` replace the source bind group and leave the camera buffer in place.

**Boundary.** No `unsafe` and no `wasm-bindgen` in the viewport crate. No new `as` cast in the viewport math or in the `set_view` path. The wgpu calls are the ones `set_voi` already uses on wgpu 30.0.1: `create_buffer_init`, `write_buffer`, `BindGroupEntry` and `as_entire_binding`. `clippy -D warnings` on `ocelli-render` exited 0. The README row for `viewport.md` lists F-045, matching that file's header. The `render-graph.md` header and its index row both list F-045. The new camera paragraph matches the code above. Pan is `Pt<Canvas>`, and a non-zero `z` cannot carry a slice index.
