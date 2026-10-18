# F-039 review, pass 4

**Reviewed**: working tree against 9426845
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None. Pass 1's N1, N2 and N3 were not promoted. `FrameOutcome::Noop` still documents the acquire-failure collapse at `crates/ocelli-render/src/graph.rs:172`. `AttachedSurface` width and height are still only written. `resize_bound` at `crates/ocelli-wasm/src/canvas.rs:108` still uses the tier C text when no context is open. None of those changed behaviour.

## Verified clean

Independent pass. The reviewer did not write the code. This pass re-checked the two comments pass 3 charged and the attach path pass 3 left uncharged. The extent overflow lines were read and the extent tests were re-run. They were not mutated, because those lines are unchanged.

Pass 3 D1 is fixed. `crates/ocelli-render/src/graph.rs:434` documents `AttachedSurface` and only that struct. The text is "A surface bound to one viewport. The colour attachment is acquired from it inside `tick` and is not stored on `GpuSlot`." `crates/ocelli-render/src/graph.rs:442` documents `RenderGraph`. The text is "The render graph: registered viewports, their dirt, and the pipelines compiled at creation." Each sentence occurs once. Each `///` block sits on the next item. A search of the crate found no second copy of either sentence.

The surface sentence matches the tick path. `tick` calls `acquire_dirty_surfaces`, which calls `attached.surface.get_current_texture`. `encode_frame` builds the colour view from that texture and uses it as the colour attachment. `GpuSlot` holds the caller-supplied `target` view, the uniform, the bind groups and the optional stored texture. It does not hold the surface or the acquired texture. Those live on `RenderGraph::surfaces` and `RenderGraph::frame_textures`. The graph sentence is the one that was true of `RenderGraph` before this story, and the struct still registers viewports, tracks dirt and compiles pipelines at creation.

Pass 3 D2 is fixed. `crates/ocelli-render/src/gpu.rs:80` says the format the surface would use is not the format the graph was compiled for. The next sentence says the error is returned before `configure`, so the previous configuration stays. The `#[error]` string is still "the surface format is not the format the graph was compiled for". The only `SurfaceError::FormatDiffers` return is `crates/ocelli-render/src/graph.rs:874`. `configure_viewport` is the next call, at line 876. `reconfigure_surface` selects the format with `first_render_attachment` and returns before that call, so `surface.configure` does not run and the previous surface configuration stays. The held frame texture is dropped earlier in the same method. The method doc already says that drop happens before `configure`. Dropping the texture is not a change to the surface configuration.

The wasm attach path is not that return. `attach_with_context` at `crates/ocelli-wasm/src/canvas.rs:225` calls `configure_viewport` and then, when a graph already exists and `color_format` differs, returns the same Display string at line 252. That string is not `Err(SurfaceError::FormatDiffers)`. The approved plan's resize rule is `reconfigure_surface`. `docs/lld/render-graph.md:80` puts the before-`configure` refusal on that method. `resize_bound` calls `reconfigure_surface`. The plan's attach rule is `attach_target` then `configure_viewport`, and the format is what `configure_viewport` returns. A second canvas whose format differs is configured as its own surface, then dropped, because the error returns before `bind_surface`. Already bound surfaces are not passed to `configure`. `docs/lld/gpu-ownership.md:354` says an empty extent is refused before `configure`. That check is inside `configure_viewport` and is not this format check. The plan and the LLD do not require `attach_with_context` to refuse before `configure`. This pass does not charge that path.

`crates/ocelli-render/src/extent.rs:37` is still `f64::round(css * dpr)`. Lines 41 to 42 still return `u32::MAX` when the rounded product is non-finite and positive, before the empty-axis floor. `bin/ocelli.sh test ocelli-render extent::tests -- --test-threads=8` exited 0. 6 passed, including `an_overflowing_product_saturates_to_u32_max` and `extent_at_dpr_one_two_and_one_and_a_quarter`. The lines did not change, so this pass did not mutate them.

The `ocelli_version` doc comment at `crates/ocelli-wasm/src/lib.rs:42` still says that function is the whole exported surface until F-101. `git diff HEAD -- crates/ocelli-wasm/src/lib.rs` does not touch that sentence. This pass does not charge it. The module comment above `mod canvas` was rewritten and does not repeat that claim.

This pass did not rebuild wasm and did not run ignored GPU tests.
